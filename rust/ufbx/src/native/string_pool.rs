//! Port of the `// -- String pool` banner section (ufbx.c:4895-5286) plus the
//! `// -- String constants` section (ufbx.c:5288-5979): interning into the
//! string arena (`pool.buf`) via the hashmap (`pool.map` of `ufbx_string`),
//! UTF-8 sanitization, the canonical static string table, and the small vec2/
//! vec3 helpers that close the constants section.
//!
//! Interning order and hashmap interaction are allocation-observable
//! (PORTING.md checklist #12): `ufbxi_map_grow` is called BEFORE hashing /
//! sanitizing, sanitization grows `temp_str` through the map's allocator, and
//! the arena copy happens only on insert-miss — keep the sequence exactly.
//!
//! Non-UTF8 handling (PORTING.md checklist #11): all strings are raw byte
//! slices / pointers internally; no `str::from_utf8` in this module.
//!
//! The string constants are statics (their ADDRESSES are the canonical
//! interned pointers — C compares `str.data == ufbxi_Foo`); identifiers keep
//! the exact C spelling minus the `ufbxi_` prefix for grep-parity across the
//! ~1500 later use sites (`AllSame`, `Cone_angle`, `d_X`, ...), hence the
//! `non_upper_case_globals` allow.
//!
//! Phase 1: no consumers yet (`ufbxi_context` arrives with the parse units).
#![allow(dead_code, non_upper_case_globals)]
use core::ffi::c_void;
use core::ptr;

use crate::generated::{Error, UnicodeErrorHandling, Vec2, Vec3, WarningType};
use crate::native::allocator::{free, grow_array};
use crate::native::buf::Buf;
use crate::native::error::{
    memcmp, strlen, ufbxi_check_err, ufbxi_check_err_msg, ufbxi_check_return_err,
    utf8_valid_length, Fail, EMPTY_CHAR,
};
use crate::native::hash::{hash_string, hash_string_check_ascii, map_free, Map};
use crate::native::platform::{
    math, min_real, min_sz, to_size, ufbx_assert, ufbxi_regression_assert,
};
use crate::native::warnings::{ufbxi_warnf_imp, Warnings};
use crate::prelude::as_f64;
use crate::prelude::{Blob, Real, String};

// -- String pool (ufbx.c:4895)
//
// C comment (ufbx.c:4897-4899):
// All strings found in FBX files are interned for deduplication and fast
// comparison. Our fixed internal strings (`ufbxi_String`) are considered the
// canonical pointers for said strings so we can compare them by address.

// ufbx.c:4901-4910 `ufbxi_string_pool`
// NOT `Copy`/`Clone`: embeds an owning `Buf` and `Map` — see PORTING.md
// "Copy vs non-Copy structs".
#[repr(C)]
pub(crate) struct StringPool {
    pub error: *mut Error,
    pub buf: Buf,            // < Buffer for the actual string data
    pub map: Map,            // < Map of `ufbxi_string`
    pub initial_size: usize, // < Number of initial entries
    pub temp_str: *mut u8,   // < Temporary string buffer of `temp_cap`
    pub temp_cap: usize,     // < Capacity of the temporary buffer
    pub error_handling: UnicodeErrorHandling,
    pub warnings: *mut Warnings,
}

// Typed interior-mutable VIEW over an owned `StringPool` field, reinterpreted in
// place. `.buf` recurses into `BufView`; other leaves are getters/setters or raw-ptr
// getters for addr-of sites. The whole-struct copy (`cc.string_pool = uc.string_pool`)
// uses the context-level value getter/setter, not this view.
pub(crate) type StringPoolView = crate::native::view::View<StringPool>;

impl StringPoolView {
    #[inline(always)]
    /// Moves the field out by bitwise read (`ptr::read`). C does this as plain
    /// struct assignment; the source field still holds the stale bits (no
    /// `Drop`), so the caller must overwrite it or treat it as moved-from.
    pub(crate) fn take_buf(&self) -> Buf {
        unsafe { core::ptr::read(&raw const (*self.get()).buf) }
    }
    #[inline(always)]
    pub(crate) fn buf_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).buf as *mut crate::native::buf::BufView) }
    }
    #[inline(always)]
    pub(crate) fn buf_mut_ptr(&self) -> *mut Buf {
        unsafe { &raw mut (*self.get()).buf }
    }
    #[inline(always)]
    pub(crate) fn set_error(&self, error: *mut Error) {
        unsafe {
            (*self.get()).error = error;
        }
    }
    #[inline(always)]
    pub(crate) fn map_mut_ptr(&self) -> *mut Map {
        unsafe { &raw mut (*self.get()).map }
    }
    // `map` (Map) — typed VIEW handle (reinterpret-in-place); accessors on MapView.
    #[inline(always)]
    pub(crate) fn map_view(&self) -> &crate::native::hash::MapView {
        unsafe { &*(&raw mut (*self.get()).map as *mut crate::native::hash::MapView) }
    }
    #[inline(always)]
    pub(crate) fn set_initial_size(&self, initial_size: usize) {
        unsafe {
            (*self.get()).initial_size = initial_size;
        }
    }
    #[inline(always)]
    pub(crate) fn set_error_handling(&self, error_handling: UnicodeErrorHandling) {
        unsafe {
            (*self.get()).error_handling = error_handling;
        }
    }
    #[inline(always)]
    pub(crate) fn set_warnings(&self, warnings: *mut Warnings) {
        unsafe {
            (*self.get()).warnings = warnings;
        }
    }
}

// ufbx.c:4912-4916 `ufbxi_sanitized_string`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct SanitizedString {
    pub raw_data: *const u8, // < UTF-8 data follows at `raw_length+1` if `utf8_length > 0`
    pub raw_length: u32,     // < Length of the non-sanitized original string
    pub utf8_length: u32,    // < Length of sanitized UTF-8 string (or zero)
}

// ufbx.c:4918-4921 `ufbxi_str_equal`
// Slice equality is exactly C's shape: length compare first, then a byte
// memcmp over the (equal) lengths.
#[inline(always)]
pub(crate) fn str_equal(a: &[u8], b: &[u8]) -> bool {
    a == b
}

// ufbx.c:4923-4929 `ufbxi_str_less`
#[inline(always)]
pub(crate) fn str_less(a: &[u8], b: &[u8]) -> bool {
    let len = min_sz(a.len(), b.len());
    // C: `memcmp` over the shorter length — byte slices compare as unsigned
    // chars, matching C's `(const unsigned char *)` walk.
    let cmp = a[..len].cmp(&b[..len]);
    if cmp != core::cmp::Ordering::Equal {
        return cmp == core::cmp::Ordering::Less;
    }
    a.len() < b.len()
}

// ufbx.c:4931-4938 `ufbxi_str_cmp`
// C returns raw `memcmp` output whose magnitude is unspecified; every caller
// uses only the sign, so the `Ordering` mapped to -1/0/1 is equivalent.
#[inline(always)]
pub(crate) fn str_cmp(a: &[u8], b: &[u8]) -> i32 {
    let len = min_sz(a.len(), b.len());
    let cmp = a[..len].cmp(&b[..len]);
    if cmp != core::cmp::Ordering::Equal {
        return if cmp == core::cmp::Ordering::Less {
            -1
        } else {
            1
        };
    }
    if a.len() != b.len() {
        return if a.len() < b.len() { -1 } else { 1 };
    }
    0
}

// Unanchored raw-`String` projections over the three anchored slice bodies
// above, for call sites whose operands are still by-value `String`s with no
// view to borrow from (locals, temporaries, raw-probe fields).
#[inline(always)]
pub(crate) unsafe fn str_equal_raw(a: String, b: String) -> bool {
    // SAFETY: the caller vouches `a`/`b` are valid `String` runs — each `data`
    // readable for its own `length` (the `as_bytes` contract).
    unsafe { str_equal(a.as_bytes(), b.as_bytes()) }
}

#[inline(always)]
pub(crate) unsafe fn str_less_raw(a: String, b: String) -> bool {
    // SAFETY: the caller vouches `a`/`b` are valid `String` runs — each `data`
    // readable for its own `length` (the `as_bytes` contract).
    unsafe { str_less(a.as_bytes(), b.as_bytes()) }
}

#[inline(always)]
pub(crate) unsafe fn str_cmp_raw(a: String, b: String) -> i32 {
    // SAFETY: the caller vouches `a`/`b` are valid `String` runs — each `data`
    // readable for its own `length` (the `as_bytes` contract).
    unsafe { str_cmp(a.as_bytes(), b.as_bytes()) }
}

// ufbx.c:4940-4944 `ufbxi_str_c`
#[inline(always)]
pub(crate) unsafe fn str_c(str_: *const u8) -> String {
    // SAFETY: the caller vouches `str_` points at a NUL-terminated C string, the
    // precondition `strlen` requires.
    String::new_c(str_, unsafe { strlen(str_) })
}

// ufbx.c:4946-4958 `ufbxi_get_concat_key`
#[inline(never)]
pub(crate) unsafe fn get_concat_key(parts: *const String, num_parts: usize) -> u32 {
    let mut key: u32 = 0;
    let mut shift: u32 = 32;
    // C: `ufbxi_for(const ufbx_string, part, parts, num_parts)`
    let mut part = parts;
    // SAFETY: the caller vouches `parts` addresses `num_parts` `String`s, so the
    // one-past-the-end pointer is in bounds of that same allocation.
    let part_end = unsafe { parts.add(num_parts) };
    while part != part_end {
        // SAFETY: `part` walks `[parts, part_end)`, so it addresses a live
        // `String`; when its length is the C-string sentinel, `strlen` reads the
        // NUL-terminated run `part->data` points at.
        let length = unsafe {
            if (*part).length != usize::MAX {
                (*part).length
            } else {
                strlen((*part).data)
            }
        };
        let mut i: usize = 0;
        while i < length {
            shift -= 8;
            // C: `key |= (uint32_t)(uint8_t)part->data[i] << shift;`
            // SAFETY: `part` addresses a live `String` and `i < length`, its
            // number of readable bytes at `part->data`.
            key |= (unsafe { *(*part).data.add(i) } as u32) << shift;
            if shift == 0 {
                return key;
            }
            i += 1;
        }
        // SAFETY: `part` is in `[parts, part_end)`, so advancing by one lands at
        // or before the one-past-the-end `part_end`.
        part = unsafe { part.add(1) };
    }
    key
}

// ufbx.c:4960-4972 `ufbxi_concat_str_cmp`
#[inline(never)]
pub(crate) unsafe fn concat_str_cmp(
    ref_: *const String,
    parts: *const String,
    num_parts: usize,
) -> i32 {
    // SAFETY: the caller vouches `ref_` addresses a valid `String`, whose
    // `data` is readable for `length` bytes, so `end` is that run's
    // one-past-the-end pointer.
    let (mut ptr_, end) = unsafe { ((*ref_).data, (*ref_).data.add((*ref_).length)) };
    // C: `ufbxi_for(const ufbx_string, part, parts, num_parts)`
    let mut part = parts;
    // SAFETY: the caller vouches `parts` addresses `num_parts` `String`s, so the
    // one-past-the-end pointer is in bounds of that same allocation.
    let part_end = unsafe { parts.add(num_parts) };
    while part != part_end {
        // SAFETY: `part` walks `[parts, part_end)`, so it addresses a live
        // `String`; when its length is the C-string sentinel, `strlen` reads the
        // NUL-terminated run `part->data` points at.
        let length = unsafe {
            if (*part).length != usize::MAX {
                (*part).length
            } else {
                strlen((*part).data)
            }
        };
        // SAFETY: `ptr_` and `end` bracket the same `ref_->data` run — `ptr_`
        // only advances toward `end` below — so they are two pointers into one
        // object, which is what `offset_from` requires.
        let to_cmp = min_sz(to_size(unsafe { end.offset_from(ptr_) }), length);
        let cmp = if to_cmp > 0 {
            // SAFETY: `to_cmp <= end - ptr_` readable bytes at `ptr_` and
            // `to_cmp <= length`, the bytes readable at `part->data`.
            unsafe { memcmp(ptr_, (*part).data, to_cmp) }
        } else {
            0
        };
        if cmp != 0 {
            return cmp;
        }
        if to_cmp != length {
            return -1;
        }
        // SAFETY: this point is reached only when `to_cmp == length`, so
        // `length <= end - ptr_` and the advance stays at or before `end`.
        ptr_ = unsafe { ptr_.add(length) };
        // SAFETY: `part` is in `[parts, part_end)`, so advancing by one lands at
        // or before the one-past-the-end `part_end`.
        part = unsafe { part.add(1) };
    }
    if ptr_ == end {
        0
    } else {
        1
    }
}

// ufbx.c:4974-4977 `ufbxi_starts_with`
#[inline(always)]
pub(crate) unsafe fn starts_with(str_: String, prefix: String) -> bool {
    // SAFETY: the caller vouches `str_`/`prefix` are valid `String` runs; the
    // compare is reached only when `str_.length >= prefix.length`, so
    // `prefix.length` bytes are readable from both `str_.data` and `prefix.data`.
    str_.length >= prefix.length && unsafe { memcmp(str_.data, prefix.data, prefix.length) } == 0
}

// ufbx.c:4979-4982 `ufbxi_ends_with`
#[inline(always)]
pub(crate) unsafe fn ends_with(str_: String, suffix: String) -> bool {
    // SAFETY: the caller vouches `str_`/`suffix` are valid `String` runs; the
    // compare is reached only when `str_.length >= suffix.length`, so
    // `str_.length - suffix.length` is in bounds of `str_.data` and the trailing
    // `suffix.length` bytes are readable from both there and `suffix.data`.
    str_.length >= suffix.length
        && unsafe {
            memcmp(
                str_.data.add(str_.length - suffix.length),
                suffix.data,
                suffix.length,
            )
        } == 0
}

// ufbx.c:4984-4993 `ufbxi_remove_prefix_len`
#[inline(never)]
pub(crate) unsafe fn remove_prefix_len(
    str_: *mut String,
    prefix: *const u8,
    prefix_len: usize,
) -> bool {
    let prefix_str = String::new_c(prefix, prefix_len);
    // SAFETY: the caller vouches `str_` addresses a valid `String` and `prefix`
    // is readable for `prefix_len` bytes; `*str_` and `prefix_str` are the two
    // valid `String` runs `starts_with` requires.
    if unsafe { starts_with(*str_, prefix_str) } {
        // SAFETY: `starts_with` just confirmed `str_->length >= prefix_len`, so
        // `str_->data + prefix_len` stays within the run and the shortened
        // length is non-negative.
        unsafe {
            (*str_).data = (*str_).data.add(prefix_len);
            (*str_).length -= prefix_len;
        }
        return true;
    }
    false
}

// ufbx.c:4995-5003 `ufbxi_remove_suffix_len`
#[inline(never)]
pub(crate) unsafe fn remove_suffix_len(
    str_: *mut String,
    suffix: *const u8,
    suffix_len: usize,
) -> bool {
    let suffix_str = String::new_c(suffix, suffix_len);
    // SAFETY: the caller vouches `str_` addresses a valid `String` and `suffix`
    // is readable for `suffix_len` bytes; `*str_` and `suffix_str` are the two
    // valid `String` runs `ends_with` requires.
    if unsafe { ends_with(*str_, suffix_str) } {
        // SAFETY: `ends_with` just confirmed `str_->length >= suffix_len`, so
        // the shortened length is non-negative.
        unsafe { (*str_).length -= suffix_len };
        return true;
    }
    false
}

// ufbx.c:5005-5008 `ufbxi_remove_prefix_str`
#[inline(always)]
pub(crate) unsafe fn remove_prefix_str(str_: *mut String, prefix: String) -> bool {
    // SAFETY: the caller's `str_` contract is forwarded unchanged, and `prefix`
    // is a valid `String` whose `data`/`length` describe one readable run.
    unsafe { remove_prefix_len(str_, prefix.data, prefix.length) }
}

// ufbx.c:5010-5013 `ufbxi_remove_suffix_c`
#[inline(always)]
pub(crate) unsafe fn remove_suffix_c(str_: *mut String, suffix: *const u8) -> bool {
    // SAFETY: the caller's `str_` contract is forwarded unchanged, and `suffix`
    // points at a NUL-terminated C string, so `strlen` reads its run and the
    // same run is readable for the length it returns.
    unsafe { remove_suffix_len(str_, suffix, strlen(suffix)) }
}

// ufbx.c:5015-5020 `ufbxi_map_cmp_string`
pub(crate) unsafe extern "C" fn map_cmp_string(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> i32 {
    let _ = user; // (void)user
    let a = va as *const String;
    let b = vb as *const String;
    // SAFETY: the map comparator contract gives `va`/`vb` as pointers to the
    // live `String` keys being compared, so `*a`/`*b` read valid `String` runs.
    unsafe { str_cmp_raw(*a, *b) }
}

// ufbx.c:5022-5026 `ufbxi_safe_string`
// Sound as a safe fn: `data` is only stored (via `String::new_c`), never
// dereferenced here — the caller still carries the obligation to dereference
// the resulting `String` soundly.
#[inline(always)]
pub(crate) fn safe_string(data: *const u8, length: usize) -> String {
    String::new_c(
        if length > 0 {
            data
        } else {
            EMPTY_CHAR.as_ptr()
        },
        length,
    )
}

// ufbx.c:5028-5032 `ufbxi_string_pool_temp_free`
pub(crate) unsafe fn string_pool_temp_free(pool: *mut StringPool) {
    // SAFETY: the caller vouches `pool` addresses a live `StringPool`;
    // `temp_str`/`temp_cap` are either the null/0 never-allocated pair (`free`
    // no-ops on count 0) or the buffer/capacity pair grown through the pool's own
    // `map.ator`, the pairing `free` requires.
    unsafe { free::<u8>((*pool).map.ator, (*pool).temp_str, (*pool).temp_cap) };
    // SAFETY: `&mut (*pool).map` addresses the pool's own live `Map`, uniquely
    // borrowed here for its last use before the pool is discarded.
    unsafe { map_free(&mut (*pool).map) };
}

// ufbx.c:5034-5064 `ufbxi_add_replacement_char`
// C: `ufbxi_nodiscard static size_t` — infallible, plain return value.
pub(crate) unsafe fn add_replacement_char(pool: *mut StringPool, dst: *mut u8, c: u8) -> usize {
    // SAFETY: the caller vouches `pool` addresses a live `StringPool`.
    match unsafe { (*pool).error_handling } {
        UnicodeErrorHandling::ReplacementCharacter => {
            // SAFETY: the caller vouches `dst` has room for the up-to-3-byte
            // replacement this arm writes (`sanitize_string` keeps >= 16 free
            // bytes at `dst`).
            unsafe {
                *dst.add(0) = 0xefu8;
                *dst.add(1) = 0xbfu8;
                *dst.add(2) = 0xbdu8;
            }
            3
        }

        UnicodeErrorHandling::Underscore => {
            // SAFETY: the caller vouches `dst` has room for the single byte this
            // arm writes.
            unsafe { *dst.add(0) = b'_' };
            1
        }

        UnicodeErrorHandling::QuestionMark => {
            // SAFETY: the caller vouches `dst` has room for the single byte this
            // arm writes.
            unsafe { *dst.add(0) = b'?' };
            1
        }

        UnicodeErrorHandling::Remove => 0,

        UnicodeErrorHandling::UnsafeIgnore => {
            // SAFETY: the caller vouches `dst` has room for the single byte this
            // arm writes.
            unsafe { *dst.add(0) = c };
            1
        }

        // C `default:` arm (out-of-range enum values); `AbortLoading` is
        // rejected before this function is reached — Rust's closed enum makes
        // the arm reachable only for `AbortLoading`.
        UnicodeErrorHandling::AbortLoading => 0,
    }
}

// ufbx.c:5065-5166 `ufbxi_sanitize_string`
#[inline(never)]
pub(crate) unsafe fn sanitize_string(
    pool: *mut StringPool,
    sanitized: *mut SanitizedString,
    str_: *const u8,
    length: usize,
    valid_length: usize,
    push_both: bool,
) -> Result<(), Fail> {
    // Handle only invalid cases here
    ufbx_assert!(valid_length < length);
    ufbxi_check_err_msg!(
        unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
        // SAFETY: the caller vouches `pool` addresses a live `StringPool`.
        unsafe { (*pool).error_handling } != UnicodeErrorHandling::AbortLoading,
        "Invalid UTF-8",
        "pool->error_handling != UFBX_UNICODE_ERROR_HANDLING_ABORT_LOADING"
    );
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
        ufbxi_warnf_imp!(
            // SAFETY: `pool->warnings`, when non-null, is the context-owned
            // warnings sink the pool was initialized with (`set_warnings`),
            // live with write provenance for the whole load.
            unsafe { crate::native::warnings::opt_warnings_view((*pool).warnings) },
            WarningType::BadUnicode,
            !0u32,
            "Bad UTF-8 string"
        )
        .is_ok(),
        "ufbxi_warnf_imp(pool->warnings, UFBX_WARNING_BAD_UNICODE, ~0u, \"Bad UTF-8 string\")"
    );

    let mut index = valid_length;
    let mut dst_len = index;
    if push_both {
        // Copy both the full raw string and the initial valid part
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
            length <= usize::MAX / 2 - 64,
            "length <= SIZE_MAX / 2 - 64"
        );
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
            // SAFETY: the growth targets are the pool's own `temp_str`/`temp_cap`
            // buffer pair, grown through the pool's own `map.ator` — the pairing
            // `grow_array` requires. The verbatim C condition text follows, so
            // wrapping does not perturb the recorded error string.
            unsafe {
                grow_array::<u8>(
                    (*pool).map.ator,
                    &mut (*pool).temp_str,
                    &mut (*pool).temp_cap,
                    length * 2 + 64
                )
            },
            "ufbxi_grow_array_size((pool->map.ator), sizeof(**(&pool->temp_str)), (&pool->temp_str), (&pool->temp_cap), (length * 2 + 64))"
        );
        // SAFETY: `grow_array` just ensured `temp_cap >= length * 2 + 64`, so
        // `temp_str` is writable for the `length` copied bytes, the trailing NUL
        // at `[length]`, and the `index <= length` bytes copied at `[length + 1]`;
        // `str_` is the caller's source, readable for `length` bytes and, per the
        // caller contract, never points into the pool's temp buffer, so the copies
        // do not overlap.
        unsafe {
            ptr::copy_nonoverlapping(str_, (*pool).temp_str, length);
            *(*pool).temp_str.add(length) = b'\0';
            ptr::copy_nonoverlapping(str_, (*pool).temp_str.add(length + 1), index);
        }
        dst_len += length + 1;
    } else {
        // Copy the initial valid part
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
            length <= usize::MAX - 64,
            "length <= SIZE_MAX - 64"
        );
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
            // SAFETY: the growth targets are the pool's own `temp_str`/`temp_cap`
            // buffer pair, grown through the pool's own `map.ator` — the pairing
            // `grow_array` requires. The verbatim C condition text follows, so
            // wrapping does not perturb the recorded error string.
            unsafe {
                grow_array::<u8>(
                    (*pool).map.ator,
                    &mut (*pool).temp_str,
                    &mut (*pool).temp_cap,
                    length + 64
                )
            },
            "ufbxi_grow_array_size((pool->map.ator), sizeof(**(&pool->temp_str)), (&pool->temp_str), (&pool->temp_cap), (length + 64))"
        );
        // SAFETY: `grow_array` just ensured `temp_cap >= length + 64`, so
        // `temp_str` is writable for the `index <= length` copied bytes; `str_` is
        // the caller's source, readable for `index` bytes and, per the caller
        // contract, never points into the pool's temp buffer.
        unsafe { ptr::copy_nonoverlapping(str_, (*pool).temp_str, index) };
    }

    // SAFETY: `temp_str` was grown by the branch above, so it addresses the pool's
    // live temp buffer.
    let mut dst = unsafe { (*pool).temp_str };
    while index < length {
        // SAFETY: `index < length`, the caller's count of readable bytes at `str_`.
        let c = unsafe { *str_.add(index) };
        let left = length - index;

        // Not optimal but not the worst thing ever
        // SAFETY: the caller vouches `pool` addresses a live `StringPool`.
        if unsafe { (*pool).temp_cap } - dst_len < 16 {
            ufbxi_check_err!(
                unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
                // SAFETY: the growth targets are the pool's own
                // `temp_str`/`temp_cap` buffer pair, grown through the pool's own
                // `map.ator` — the pairing `grow_array` requires. The verbatim C
                // condition text follows, so wrapping does not perturb the
                // recorded error string.
                unsafe {
                    grow_array::<u8>(
                        (*pool).map.ator,
                        &mut (*pool).temp_str,
                        &mut (*pool).temp_cap,
                        dst_len + 16
                    )
                },
                "ufbxi_grow_array_size((pool->map.ator), sizeof(**(&pool->temp_str)), (&pool->temp_str), (&pool->temp_cap), (dst_len + 16))"
            );
            // SAFETY: `temp_str` was just re-grown, so it addresses the pool's
            // live temp buffer.
            dst = unsafe { (*pool).temp_str };
        }

        if (c & 0x80) == 0 {
            if c != 0 {
                // SAFETY: the block above guarantees >= 16 free bytes at
                // `dst[dst_len]`, room for this 1-byte write.
                unsafe { *dst.add(dst_len) = c };
                dst_len += 1;
                index += 1;
                continue;
            }
        } else if (c & 0xe0) == 0xc0 && left >= 2 {
            // SAFETY: `left >= 2` means `index + 1 < length`, in bounds of `str_`.
            let t0 = unsafe { *str_.add(index + 1) };
            let code = (c as u32) << 8 | (t0 as u32) << 0;
            if (code & 0xc0) == 0x80 && code >= 0xc280 {
                // SAFETY: >= 16 free bytes at `dst[dst_len]`, room for 2 bytes.
                unsafe {
                    *dst.add(dst_len + 0) = c;
                    *dst.add(dst_len + 1) = t0;
                }
                dst_len += 2;
                index += 2;
                continue;
            }
        } else if (c & 0xf0) == 0xe0 && left >= 3 {
            // SAFETY: `left >= 3` means `index + 2 < length`, in bounds of `str_`.
            let t0 = unsafe { *str_.add(index + 1) };
            let t1 = unsafe { *str_.add(index + 2) };
            let code = (c as u32) << 16 | (t0 as u32) << 8 | (t1 as u32);
            if (code & 0xc0c0) == 0x8080
                && code >= 0xe0a080
                && (code < 0xeda080 || code >= 0xee8080)
            {
                // SAFETY: >= 16 free bytes at `dst[dst_len]`, room for 3 bytes.
                unsafe {
                    *dst.add(dst_len + 0) = c;
                    *dst.add(dst_len + 1) = t0;
                    *dst.add(dst_len + 2) = t1;
                }
                dst_len += 3;
                index += 3;
                continue;
            }
        } else if (c & 0xf8) == 0xf0 && left >= 4 {
            // SAFETY: `left >= 4` means `index + 3 < length`, in bounds of `str_`.
            let t0 = unsafe { *str_.add(index + 1) };
            let t1 = unsafe { *str_.add(index + 2) };
            let t2 = unsafe { *str_.add(index + 3) };
            let code = (c as u32) << 24 | (t0 as u32) << 16 | (t1 as u32) << 8 | (t2 as u32);
            if (code & 0xc0c0c0) == 0x808080 && code >= 0xf0908080u32 && code <= 0xf48fbfbfu32 {
                // SAFETY: >= 16 free bytes at `dst[dst_len]`, room for 4 bytes.
                unsafe {
                    *dst.add(dst_len + 0) = c;
                    *dst.add(dst_len + 1) = t0;
                    *dst.add(dst_len + 2) = t1;
                    *dst.add(dst_len + 3) = t2;
                }
                dst_len += 4;
                index += 4;
                continue;
            }
        }

        // SAFETY: `pool` is live and `dst[dst_len]` has >= 16 free bytes, room for
        // the up-to-3-byte replacement `add_replacement_char` writes.
        dst_len += unsafe { add_replacement_char(pool, dst.add(dst_len), c) };
        index += 1;
    }

    // Sanitized strings are packed to 32-bit integers, in practice this should be fine
    // as strings are limited to 32-bit length in FBX itself.
    // The only problem case is a massive string that is full of unicode errors, ie.
    // >1GB binary blob, but these should never be sanitized.
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
        length <= u32::MAX as usize,
        "length <= UINT32_MAX"
    );
    // SAFETY: the caller vouches `sanitized` addresses a live `SanitizedString`
    // and `pool` a live `StringPool`.
    unsafe { (*sanitized).raw_data = (*pool).temp_str };
    if push_both {
        // Reserve `UINT32_MAX` for invalid UTF-8 without sanitization
        let utf8_length = dst_len - (length + 1);
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
            utf8_length < u32::MAX as usize,
            "utf8_length < UINT32_MAX"
        );
        // SAFETY: `sanitized` addresses a live `SanitizedString`.
        unsafe {
            (*sanitized).raw_length = length as u32;
            (*sanitized).utf8_length = utf8_length as u32;
        }
    } else {
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
            dst_len <= u32::MAX as usize,
            "dst_len <= UINT32_MAX"
        );
        // SAFETY: `sanitized` addresses a live `SanitizedString`.
        unsafe {
            (*sanitized).raw_length = dst_len as u32;
            (*sanitized).utf8_length = 0;
        }
    }

    Ok(())
}

// ufbx.c:5168-5209 `ufbxi_push_sanitized_string`
#[inline(never)]
pub(crate) unsafe fn push_sanitized_string(
    pool: *mut StringPool,
    sanitized: *mut SanitizedString,
    str_: *const u8,
    length: usize,
    mut hash: u32,
    raw: bool,
) -> Result<(), Fail> {
    // SAFETY: the caller vouches `str_` is readable for `length` bytes, the run
    // `hash_string` hashes.
    ufbxi_regression_assert!(hash == unsafe { hash_string(str_, length) });

    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
        length <= u32::MAX as usize,
        "length <= UINT32_MAX"
    );
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
        // SAFETY: the caller vouches `pool` addresses a live `StringPool`, so
        // `from_ptr` reinterprets a live pool and `(*pool).initial_size` reads it.
        unsafe { StringPoolView::from_ptr(pool) }
            .map_view()
            .grow::<String>(unsafe { (*pool).initial_size }),
        "ufbxi_map_grow_size((&pool->map), sizeof(ufbx_string), (pool->initial_size))"
    );

    let mut total_data: *const u8 = str_;
    let mut total_length: usize = length;

    // SAFETY: the caller vouches `sanitized` addresses a live `SanitizedString`.
    unsafe {
        (*sanitized).raw_length = length as u32;
        (*sanitized).utf8_length = 0;
    }

    if !raw {
        // SAFETY: `str_` is readable for `length` bytes.
        let valid_length = unsafe { utf8_valid_length(str_, length) };
        if valid_length != length {
            // C: `ufbxi_check_err(pool->error, ufbxi_sanitize_string(...))` — `?`
            // per PORTING.md error threading.
            // SAFETY: `pool`/`sanitized` are live, `str_` is readable for
            // `length` bytes, and `valid_length < length` (just checked), the
            // precondition `sanitize_string` asserts.
            unsafe { sanitize_string(pool, sanitized, str_, length, valid_length, true) }?;
            // SAFETY: `sanitized` is live; `sanitize_string` wrote its fields.
            total_data = unsafe { (*sanitized).raw_data };
            // C-parity: `sanitized->raw_length + sanitized->utf8_length + 1` is
            // computed in uint32_t (wraps) before widening to size_t.
            // SAFETY: `sanitized` is live with the fields just written.
            total_length = unsafe {
                (*sanitized)
                    .raw_length
                    .wrapping_add((*sanitized).utf8_length)
                    .wrapping_add(1)
            } as usize;
            // SAFETY: `str_` is readable for `length` bytes.
            hash = unsafe { hash_string(str_, length) };
        }
    }

    let ref_ = String::new_c(total_data, total_length);

    // SAFETY: the caller vouches `pool` addresses a live `StringPool`, which
    // `from_ptr` reinterprets in place.
    let entry: *mut String = unsafe { StringPoolView::from_ptr(pool) }
        .map_view()
        .find::<String, _>(hash, &ref_);
    if !entry.is_null() {
        // SAFETY: `entry` is a non-null map slot addressing a live `String`, and
        // `sanitized` is live.
        unsafe { (*sanitized).raw_data = (*entry).data };
    } else {
        // SAFETY: `pool` addresses a live `StringPool`, reinterpreted in place.
        let entry = unsafe { StringPoolView::from_ptr(pool) }
            .map_view()
            .insert::<String, _>(hash, &ref_);
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
            !entry.is_null(),
            "entry"
        );
        // SAFETY: `entry` is the just-inserted non-null map slot.
        unsafe { (*entry).length = total_length };
        // SAFETY: `pool` addresses a live `StringPool`, reinterpreted in place.
        let dst: *mut u8 = unsafe { StringPoolView::from_ptr(pool) }
            .buf_view()
            .push::<u8>(total_length + 1);
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
            !dst.is_null(),
            "dst"
        );
        // SAFETY: `dst` is a non-null run of `total_length + 1` bytes just pushed
        // onto the pool's arena, so it is writable for the `total_length` copied
        // bytes plus the trailing NUL; `total_data` is readable for
        // `total_length` bytes and is a distinct object from the fresh arena run.
        unsafe {
            ptr::copy_nonoverlapping(total_data, dst, total_length);
            *dst.add(total_length) = b'\0';
        }
        // SAFETY: `entry` is the live inserted slot and `sanitized` is live.
        unsafe {
            (*entry).data = dst;
            (*sanitized).raw_data = dst;
        }
    }

    Ok(())
}

// ufbx.c:5211-5253 `ufbxi_push_string_imp`
// C: `ufbxi_nodiscard static ufbxi_noinline const char *` — NULL on failure.
#[inline(never)]
pub(crate) unsafe fn push_string_imp(
    pool: *mut StringPool,
    mut str_: *const u8,
    mut length: usize,
    p_out_length: *mut usize,
    copy: bool,
    raw: bool,
) -> *const u8 {
    if length == 0 {
        return EMPTY_CHAR.as_ptr();
    }

    ufbxi_check_return_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
        // SAFETY: the caller vouches `pool` addresses a live `StringPool`, so
        // `from_ptr` reinterprets a live pool and `(*pool).initial_size` reads it.
        unsafe { StringPoolView::from_ptr(pool) }
            .map_view()
            .grow::<String>(unsafe { (*pool).initial_size }),
        ptr::null(),
        "ufbxi_map_grow_size((&pool->map), sizeof(ufbx_string), (pool->initial_size))"
    );

    let mut hash: u32;
    if raw {
        // SAFETY: the caller vouches `str_` is readable for `length` bytes.
        hash = unsafe { hash_string(str_, length) };
    } else {
        let mut non_ascii = false;
        // SAFETY: `str_` is readable for `length` bytes and `non_ascii` is an
        // unaliased local out-param.
        hash = unsafe { hash_string_check_ascii(str_, length, &mut non_ascii) };
        if non_ascii {
            // SAFETY: `str_` is readable for `length` bytes.
            let valid_length = unsafe { utf8_valid_length(str_, length) };
            if valid_length < length {
                // C: `ufbxi_sanitized_string sanitized;` (written in full by
                // `ufbxi_sanitize_string` before any read).
                let mut sanitized = SanitizedString {
                    raw_data: ptr::null(),
                    raw_length: 0,
                    utf8_length: 0,
                };
                // C: `ufbxi_check_return_err(pool->error, ufbxi_sanitize_string(...), NULL);`
                // SAFETY: `pool` is live, `&mut sanitized` is a live local,
                // `str_` is readable for `length` bytes, and `valid_length <
                // length` (just checked), the precondition `sanitize_string`
                // asserts.
                if unsafe {
                    sanitize_string(pool, &mut sanitized, str_, length, valid_length, false)
                }
                .is_err()
                {
                    return ptr::null();
                }
                str_ = sanitized.raw_data;
                length = sanitized.raw_length as usize;
                // SAFETY: `str_`/`length` are the sanitized run just produced,
                // readable for `length` bytes.
                hash = unsafe { hash_string(str_, length) };
                // SAFETY: the caller vouches `p_out_length` addresses a live
                // `usize` out-param.
                unsafe { *p_out_length = length };
            }
        }
    }

    let ref_ = String::new_c(str_, length);

    // SAFETY: the caller vouches `pool` addresses a live `StringPool`, which
    // `from_ptr` reinterprets in place.
    let entry: *mut String = unsafe { StringPoolView::from_ptr(pool) }
        .map_view()
        .find::<String, _>(hash, &ref_);
    if !entry.is_null() {
        // SAFETY: `entry` is a non-null map slot addressing a live `String`.
        return unsafe { (*entry).data };
    }
    // SAFETY: `pool` addresses a live `StringPool`, reinterpreted in place.
    let entry = unsafe { StringPoolView::from_ptr(pool) }
        .map_view()
        .insert::<String, _>(hash, &ref_);
    ufbxi_check_return_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
        !entry.is_null(),
        ptr::null(),
        "entry"
    );
    // SAFETY: `entry` is the just-inserted non-null map slot.
    unsafe { (*entry).length = length };
    if copy {
        // SAFETY: `pool` addresses a live `StringPool`, reinterpreted in place.
        let dst: *mut u8 = unsafe { StringPoolView::from_ptr(pool) }
            .buf_view()
            .push::<u8>(length + 1);
        ufbxi_check_return_err!(
            unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
            !dst.is_null(),
            ptr::null(),
            "dst"
        );
        // SAFETY: `dst` is a non-null run of `length + 1` bytes just pushed onto
        // the pool's arena, so it is writable for the `length` copied bytes plus
        // the trailing NUL; `str_` is readable for `length` bytes and distinct
        // from the fresh arena run.
        unsafe {
            ptr::copy_nonoverlapping(str_, dst, length);
            *dst.add(length) = b'\0';
        }
        // SAFETY: `entry` is the live inserted slot.
        unsafe { (*entry).data = dst };
    } else {
        // SAFETY: `entry` is the live inserted slot.
        unsafe { (*entry).data = str_ };
    }
    // SAFETY: `entry` is the live inserted slot.
    unsafe { (*entry).data }
}

// ufbx.c:5255-5258 `ufbxi_push_string`
#[inline(always)]
pub(crate) unsafe fn push_string(
    pool: *mut StringPool,
    str_: *const u8,
    length: usize,
    p_out_length: *mut usize,
    raw: bool,
) -> *const u8 {
    // SAFETY: the caller's `pool`/`str_`/`length`/`p_out_length` contract is
    // forwarded unchanged to `push_string_imp`.
    unsafe { push_string_imp(pool, str_, length, p_out_length, true, raw) }
}

// ufbx.c:5260-5269 `ufbxi_push_string_place`
#[inline(always)]
pub(crate) unsafe fn push_string_place(
    pool: *mut StringPool,
    p_str: *mut *const u8,
    p_length: *mut usize,
    raw: bool,
) -> Result<(), Fail> {
    // SAFETY: the caller vouches `p_str`/`p_length` address live, initialized
    // in-out slots holding the string to intern.
    let mut str_ = unsafe { *p_str };
    let length = unsafe { *p_length };
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
        !str_.is_null() || length == 0,
        "str || length == 0"
    );
    // SAFETY: `pool` is live; the caller vouches `*p_str`/`*p_length` describe a
    // run readable for `length` bytes (the check above additionally rejects a null
    // `str_` with nonzero length), and `p_length` is the caller's live in-out
    // length slot.
    str_ = unsafe { push_string(pool, str_, length, p_length, raw) };
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
        !str_.is_null(),
        "str"
    );
    // SAFETY: `p_str` is the caller's live out-param.
    unsafe { *p_str = str_ };
    Ok(())
}

// ufbx.c:5271-5275 `ufbxi_push_string_place_str`
#[inline(never)]
pub(crate) unsafe fn push_string_place_str(
    pool: *mut StringPool,
    p_str: *mut String,
    raw: bool,
) -> Result<(), Fail> {
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
        !p_str.is_null(),
        "p_str"
    );
    // SAFETY: `pool` is live; `p_str` was just checked non-null and addresses a
    // live `String`, so `&mut (*p_str).data`/`.length` are its own field
    // out-params.
    unsafe { push_string_place(pool, &mut (*p_str).data, &mut (*p_str).length, raw) }
}

// ufbx.c:5277-5286 `ufbxi_push_string_place_blob`
#[inline(never)]
pub(crate) unsafe fn push_string_place_blob(
    pool: *mut StringPool,
    p_blob: *mut Blob,
    raw: bool,
) -> Result<(), Fail> {
    // SAFETY: the caller vouches `p_blob` addresses a live `Blob`.
    if unsafe { (*p_blob).size } == 0 {
        // SAFETY: `p_blob` addresses a live `Blob`.
        unsafe { (*p_blob).data = ptr::null() };
        return Ok(());
    }
    // SAFETY: `pool` is live; `p_blob` is a live `Blob` whose `data`/`size`
    // describe its run, and `&mut (*p_blob).size` is its own field out-param.
    unsafe {
        (*p_blob).data = push_string(
            pool,
            (*p_blob).data,
            (*p_blob).size,
            &mut (*p_blob).size,
            raw,
        );
    }
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*pool).error) },
        // SAFETY: `p_blob` addresses a live `Blob`.
        !unsafe { (*p_blob).data }.is_null(),
        "p_blob->data"
    );
    Ok(())
}

// -- String constants (ufbx.c:5288)
//
// C comment (ufbx.c:5289-5292):
// All strings in FBX files are pooled so by having canonical string constant
// addresses we can compare strings to these constants by comparing pointers.
// Keep the list alphabetically sorted!
//
// Byte arrays include the C string literal's implicit NUL; the padded
// constants (`"OO\0"`, `"R\0\0"`, ...) keep their extra embedded NULs so the
// arrays are byte-identical to C (NUL conventions per PORTING.md).

// ufbx.c:5294-5595 (`ufbxi_AllSame` ... `ufbxi_d_Z`)
pub(crate) static AllSame: [u8; b"AllSame\0".len()] = *b"AllSame\0";
pub(crate) static Alphas: [u8; b"Alphas\0".len()] = *b"Alphas\0";
pub(crate) static AmbientColor: [u8; b"AmbientColor\0".len()] = *b"AmbientColor\0";
pub(crate) static AnimationCurveNode: [u8; b"AnimationCurveNode\0".len()] =
    *b"AnimationCurveNode\0";
pub(crate) static AnimationCurve: [u8; b"AnimationCurve\0".len()] = *b"AnimationCurve\0";
pub(crate) static AnimationLayer: [u8; b"AnimationLayer\0".len()] = *b"AnimationLayer\0";
pub(crate) static AnimationStack: [u8; b"AnimationStack\0".len()] = *b"AnimationStack\0";
pub(crate) static ApertureFormat: [u8; b"ApertureFormat\0".len()] = *b"ApertureFormat\0";
pub(crate) static ApertureMode: [u8; b"ApertureMode\0".len()] = *b"ApertureMode\0";
pub(crate) static AreaLightShape: [u8; b"AreaLightShape\0".len()] = *b"AreaLightShape\0";
pub(crate) static AspectH: [u8; b"AspectH\0".len()] = *b"AspectH\0";
pub(crate) static AspectHeight: [u8; b"AspectHeight\0".len()] = *b"AspectHeight\0";
pub(crate) static AspectRatioMode: [u8; b"AspectRatioMode\0".len()] = *b"AspectRatioMode\0";
pub(crate) static AspectW: [u8; b"AspectW\0".len()] = *b"AspectW\0";
pub(crate) static AspectWidth: [u8; b"AspectWidth\0".len()] = *b"AspectWidth\0";
pub(crate) static Audio: [u8; b"Audio\0".len()] = *b"Audio\0";
pub(crate) static AudioLayer: [u8; b"AudioLayer\0".len()] = *b"AudioLayer\0";
pub(crate) static BaseLayer: [u8; b"BaseLayer\0".len()] = *b"BaseLayer\0";
pub(crate) static BinaryData: [u8; b"BinaryData\0".len()] = *b"BinaryData\0";
pub(crate) static BindPose: [u8; b"BindPose\0".len()] = *b"BindPose\0";
pub(crate) static BindingTable: [u8; b"BindingTable\0".len()] = *b"BindingTable\0";
pub(crate) static Binormals: [u8; b"Binormals\0".len()] = *b"Binormals\0";
pub(crate) static BinormalsIndex: [u8; b"BinormalsIndex\0".len()] = *b"BinormalsIndex\0";
pub(crate) static BinormalsW: [u8; b"BinormalsW\0".len()] = *b"BinormalsW\0";
pub(crate) static BlendMode: [u8; b"BlendMode\0".len()] = *b"BlendMode\0";
pub(crate) static BlendModes: [u8; b"BlendModes\0".len()] = *b"BlendModes\0";
pub(crate) static BlendShapeChannel: [u8; b"BlendShapeChannel\0".len()] = *b"BlendShapeChannel\0";
pub(crate) static BlendShape: [u8; b"BlendShape\0".len()] = *b"BlendShape\0";
pub(crate) static BlendWeights: [u8; b"BlendWeights\0".len()] = *b"BlendWeights\0";
pub(crate) static BoundaryRule: [u8; b"BoundaryRule\0".len()] = *b"BoundaryRule\0";
pub(crate) static Boundary: [u8; b"Boundary\0".len()] = *b"Boundary\0";
pub(crate) static ByEdge: [u8; b"ByEdge\0".len()] = *b"ByEdge\0";
pub(crate) static ByPolygonVertex: [u8; b"ByPolygonVertex\0".len()] = *b"ByPolygonVertex\0";
pub(crate) static ByPolygon: [u8; b"ByPolygon\0".len()] = *b"ByPolygon\0";
pub(crate) static ByVertex: [u8; b"ByVertex\0".len()] = *b"ByVertex\0";
pub(crate) static ByVertice: [u8; b"ByVertice\0".len()] = *b"ByVertice\0";
pub(crate) static Cache: [u8; b"Cache\0".len()] = *b"Cache\0";
pub(crate) static CameraProjectionType: [u8; b"CameraProjectionType\0".len()] =
    *b"CameraProjectionType\0";
pub(crate) static CameraStereo: [u8; b"CameraStereo\0".len()] = *b"CameraStereo\0";
pub(crate) static CameraSwitcher: [u8; b"CameraSwitcher\0".len()] = *b"CameraSwitcher\0";
pub(crate) static Camera: [u8; b"Camera\0".len()] = *b"Camera\0";
pub(crate) static CastLight: [u8; b"CastLight\0".len()] = *b"CastLight\0";
pub(crate) static CastShadows: [u8; b"CastShadows\0".len()] = *b"CastShadows\0";
pub(crate) static Channel: [u8; b"Channel\0".len()] = *b"Channel\0";
pub(crate) static Character: [u8; b"Character\0".len()] = *b"Character\0";
pub(crate) static Children: [u8; b"Children\0".len()] = *b"Children\0";
pub(crate) static Cluster: [u8; b"Cluster\0".len()] = *b"Cluster\0";
pub(crate) static CollectionExclusive: [u8; b"CollectionExclusive\0".len()] =
    *b"CollectionExclusive\0";
pub(crate) static Collection: [u8; b"Collection\0".len()] = *b"Collection\0";
pub(crate) static ColorIndex: [u8; b"ColorIndex\0".len()] = *b"ColorIndex\0";
pub(crate) static Color: [u8; b"Color\0".len()] = *b"Color\0";
pub(crate) static Colors: [u8; b"Colors\0".len()] = *b"Colors\0";
pub(crate) static Cone_angle: [u8; b"Cone angle\0".len()] = *b"Cone angle\0";
pub(crate) static ConeAngle: [u8; b"ConeAngle\0".len()] = *b"ConeAngle\0";
pub(crate) static Connections: [u8; b"Connections\0".len()] = *b"Connections\0";
pub(crate) static Constraint: [u8; b"Constraint\0".len()] = *b"Constraint\0";
pub(crate) static Content: [u8; b"Content\0".len()] = *b"Content\0";
pub(crate) static CoordAxisSign: [u8; b"CoordAxisSign\0".len()] = *b"CoordAxisSign\0";
pub(crate) static CoordAxis: [u8; b"CoordAxis\0".len()] = *b"CoordAxis\0";
pub(crate) static Count: [u8; b"Count\0".len()] = *b"Count\0";
pub(crate) static Creator: [u8; b"Creator\0".len()] = *b"Creator\0";
pub(crate) static CurrentTextureBlendMode: [u8; b"CurrentTextureBlendMode\0".len()] =
    *b"CurrentTextureBlendMode\0";
pub(crate) static CurrentTimeMarker: [u8; b"CurrentTimeMarker\0".len()] = *b"CurrentTimeMarker\0";
pub(crate) static CustomFrameRate: [u8; b"CustomFrameRate\0".len()] = *b"CustomFrameRate\0";
pub(crate) static DecayType: [u8; b"DecayType\0".len()] = *b"DecayType\0";
pub(crate) static DefaultCamera: [u8; b"DefaultCamera\0".len()] = *b"DefaultCamera\0";
pub(crate) static Default: [u8; b"Default\0".len()] = *b"Default\0";
pub(crate) static Definitions: [u8; b"Definitions\0".len()] = *b"Definitions\0";
pub(crate) static DeformPercent: [u8; b"DeformPercent\0".len()] = *b"DeformPercent\0";
pub(crate) static Deformer: [u8; b"Deformer\0".len()] = *b"Deformer\0";
pub(crate) static DiffuseColor: [u8; b"DiffuseColor\0".len()] = *b"DiffuseColor\0";
pub(crate) static Dimension: [u8; b"Dimension\0".len()] = *b"Dimension\0";
pub(crate) static Dimensions: [u8; b"Dimensions\0".len()] = *b"Dimensions\0";
pub(crate) static DisplayLayer: [u8; b"DisplayLayer\0".len()] = *b"DisplayLayer\0";
pub(crate) static Document: [u8; b"Document\0".len()] = *b"Document\0";
pub(crate) static Documents: [u8; b"Documents\0".len()] = *b"Documents\0";
pub(crate) static EdgeCrease: [u8; b"EdgeCrease\0".len()] = *b"EdgeCrease\0";
pub(crate) static EdgeIndexArray: [u8; b"EdgeIndexArray\0".len()] = *b"EdgeIndexArray\0";
pub(crate) static Edges: [u8; b"Edges\0".len()] = *b"Edges\0";
pub(crate) static EmissiveColor: [u8; b"EmissiveColor\0".len()] = *b"EmissiveColor\0";
pub(crate) static Entry: [u8; b"Entry\0".len()] = *b"Entry\0";
pub(crate) static FBXHeaderExtension: [u8; b"FBXHeaderExtension\0".len()] =
    *b"FBXHeaderExtension\0";
pub(crate) static FBXHeaderVersion: [u8; b"FBXHeaderVersion\0".len()] = *b"FBXHeaderVersion\0";
pub(crate) static FBXVersion: [u8; b"FBXVersion\0".len()] = *b"FBXVersion\0";
pub(crate) static FKEffector: [u8; b"FKEffector\0".len()] = *b"FKEffector\0";
pub(crate) static FarPlane: [u8; b"FarPlane\0".len()] = *b"FarPlane\0";
pub(crate) static FbxPropertyEntry: [u8; b"FbxPropertyEntry\0".len()] = *b"FbxPropertyEntry\0";
pub(crate) static FbxSemanticEntry: [u8; b"FbxSemanticEntry\0".len()] = *b"FbxSemanticEntry\0";
pub(crate) static FieldOfViewX: [u8; b"FieldOfViewX\0".len()] = *b"FieldOfViewX\0";
pub(crate) static FieldOfViewY: [u8; b"FieldOfViewY\0".len()] = *b"FieldOfViewY\0";
pub(crate) static FieldOfView: [u8; b"FieldOfView\0".len()] = *b"FieldOfView\0";
pub(crate) static FileName: [u8; b"FileName\0".len()] = *b"FileName\0";
pub(crate) static Filename: [u8; b"Filename\0".len()] = *b"Filename\0";
pub(crate) static FilmHeight: [u8; b"FilmHeight\0".len()] = *b"FilmHeight\0";
pub(crate) static FilmSqueezeRatio: [u8; b"FilmSqueezeRatio\0".len()] = *b"FilmSqueezeRatio\0";
pub(crate) static FilmWidth: [u8; b"FilmWidth\0".len()] = *b"FilmWidth\0";
pub(crate) static FlipNormals: [u8; b"FlipNormals\0".len()] = *b"FlipNormals\0";
pub(crate) static FocalLength: [u8; b"FocalLength\0".len()] = *b"FocalLength\0";
pub(crate) static Form: [u8; b"Form\0".len()] = *b"Form\0";
pub(crate) static Freeze: [u8; b"Freeze\0".len()] = *b"Freeze\0";
pub(crate) static FrontAxisSign: [u8; b"FrontAxisSign\0".len()] = *b"FrontAxisSign\0";
pub(crate) static FrontAxis: [u8; b"FrontAxis\0".len()] = *b"FrontAxis\0";
pub(crate) static FullWeights: [u8; b"FullWeights\0".len()] = *b"FullWeights\0";
pub(crate) static GateFit: [u8; b"GateFit\0".len()] = *b"GateFit\0";
pub(crate) static GeometricRotation: [u8; b"GeometricRotation\0".len()] = *b"GeometricRotation\0";
pub(crate) static GeometricScaling: [u8; b"GeometricScaling\0".len()] = *b"GeometricScaling\0";
pub(crate) static GeometricTranslation: [u8; b"GeometricTranslation\0".len()] =
    *b"GeometricTranslation\0";
pub(crate) static GeometryUVInfo: [u8; b"GeometryUVInfo\0".len()] = *b"GeometryUVInfo\0";
pub(crate) static Geometry: [u8; b"Geometry\0".len()] = *b"Geometry\0";
pub(crate) static GlobalSettings: [u8; b"GlobalSettings\0".len()] = *b"GlobalSettings\0";
pub(crate) static Hole: [u8; b"Hole\0".len()] = *b"Hole\0";
pub(crate) static HotSpot: [u8; b"HotSpot\0".len()] = *b"HotSpot\0";
pub(crate) static IKEffector: [u8; b"IKEffector\0".len()] = *b"IKEffector\0";
pub(crate) static ImageData: [u8; b"ImageData\0".len()] = *b"ImageData\0";
pub(crate) static Implementation: [u8; b"Implementation\0".len()] = *b"Implementation\0";
pub(crate) static Indexes: [u8; b"Indexes\0".len()] = *b"Indexes\0";
pub(crate) static InheritType: [u8; b"InheritType\0".len()] = *b"InheritType\0";
pub(crate) static InnerAngle: [u8; b"InnerAngle\0".len()] = *b"InnerAngle\0";
pub(crate) static Intensity: [u8; b"Intensity\0".len()] = *b"Intensity\0";
pub(crate) static IsTheNodeInSet: [u8; b"IsTheNodeInSet\0".len()] = *b"IsTheNodeInSet\0";
pub(crate) static KeyAttrDataFloat: [u8; b"KeyAttrDataFloat\0".len()] = *b"KeyAttrDataFloat\0";
pub(crate) static KeyAttrFlags: [u8; b"KeyAttrFlags\0".len()] = *b"KeyAttrFlags\0";
pub(crate) static KeyAttrRefCount: [u8; b"KeyAttrRefCount\0".len()] = *b"KeyAttrRefCount\0";
pub(crate) static KeyCount: [u8; b"KeyCount\0".len()] = *b"KeyCount\0";
pub(crate) static KeyTime: [u8; b"KeyTime\0".len()] = *b"KeyTime\0";
pub(crate) static KeyValueFloat: [u8; b"KeyValueFloat\0".len()] = *b"KeyValueFloat\0";
pub(crate) static KeyVer: [u8; b"KeyVer\0".len()] = *b"KeyVer\0";
pub(crate) static Key: [u8; b"Key\0".len()] = *b"Key\0";
pub(crate) static KnotVectorU: [u8; b"KnotVectorU\0".len()] = *b"KnotVectorU\0";
pub(crate) static KnotVectorV: [u8; b"KnotVectorV\0".len()] = *b"KnotVectorV\0";
pub(crate) static KnotVector: [u8; b"KnotVector\0".len()] = *b"KnotVector\0";
pub(crate) static LayerElementBinormal: [u8; b"LayerElementBinormal\0".len()] =
    *b"LayerElementBinormal\0";
pub(crate) static LayerElementColor: [u8; b"LayerElementColor\0".len()] = *b"LayerElementColor\0";
pub(crate) static LayerElementEdgeCrease: [u8; b"LayerElementEdgeCrease\0".len()] =
    *b"LayerElementEdgeCrease\0";
pub(crate) static LayerElementHole: [u8; b"LayerElementHole\0".len()] = *b"LayerElementHole\0";
pub(crate) static LayerElementMaterial: [u8; b"LayerElementMaterial\0".len()] =
    *b"LayerElementMaterial\0";
pub(crate) static LayerElementNormal: [u8; b"LayerElementNormal\0".len()] =
    *b"LayerElementNormal\0";
pub(crate) static LayerElementPolygonGroup: [u8; b"LayerElementPolygonGroup\0".len()] =
    *b"LayerElementPolygonGroup\0";
pub(crate) static LayerElementSmoothing: [u8; b"LayerElementSmoothing\0".len()] =
    *b"LayerElementSmoothing\0";
pub(crate) static LayerElementTangent: [u8; b"LayerElementTangent\0".len()] =
    *b"LayerElementTangent\0";
pub(crate) static LayerElementUV: [u8; b"LayerElementUV\0".len()] = *b"LayerElementUV\0";
pub(crate) static LayerElementVertexCrease: [u8; b"LayerElementVertexCrease\0".len()] =
    *b"LayerElementVertexCrease\0";
pub(crate) static LayerElementVisibility: [u8; b"LayerElementVisibility\0".len()] =
    *b"LayerElementVisibility\0";
pub(crate) static LayerElement: [u8; b"LayerElement\0".len()] = *b"LayerElement\0";
pub(crate) static Layer: [u8; b"Layer\0".len()] = *b"Layer\0";
pub(crate) static LayeredTexture: [u8; b"LayeredTexture\0".len()] = *b"LayeredTexture\0";
pub(crate) static Lcl_Rotation: [u8; b"Lcl Rotation\0".len()] = *b"Lcl Rotation\0";
pub(crate) static Lcl_Scaling: [u8; b"Lcl Scaling\0".len()] = *b"Lcl Scaling\0";
pub(crate) static Lcl_Translation: [u8; b"Lcl Translation\0".len()] = *b"Lcl Translation\0";
pub(crate) static LeftCamera: [u8; b"LeftCamera\0".len()] = *b"LeftCamera\0";
pub(crate) static LightType: [u8; b"LightType\0".len()] = *b"LightType\0";
pub(crate) static Light: [u8; b"Light\0".len()] = *b"Light\0";
pub(crate) static LimbLength: [u8; b"LimbLength\0".len()] = *b"LimbLength\0";
pub(crate) static LimbNode: [u8; b"LimbNode\0".len()] = *b"LimbNode\0";
pub(crate) static Limb: [u8; b"Limb\0".len()] = *b"Limb\0";
pub(crate) static Line: [u8; b"Line\0".len()] = *b"Line\0";
pub(crate) static Link: [u8; b"Link\0".len()] = *b"Link\0";
pub(crate) static LocalStart: [u8; b"LocalStart\0".len()] = *b"LocalStart\0";
pub(crate) static LocalStop: [u8; b"LocalStop\0".len()] = *b"LocalStop\0";
pub(crate) static LocalTime: [u8; b"LocalTime\0".len()] = *b"LocalTime\0";
pub(crate) static LodGroup: [u8; b"LodGroup\0".len()] = *b"LodGroup\0";
pub(crate) static MappingInformationType: [u8; b"MappingInformationType\0".len()] =
    *b"MappingInformationType\0";
pub(crate) static Marker: [u8; b"Marker\0".len()] = *b"Marker\0";
pub(crate) static MaterialAssignation: [u8; b"MaterialAssignation\0".len()] =
    *b"MaterialAssignation\0";
pub(crate) static Material: [u8; b"Material\0".len()] = *b"Material\0";
pub(crate) static Materials: [u8; b"Materials\0".len()] = *b"Materials\0";
pub(crate) static Matrix: [u8; b"Matrix\0".len()] = *b"Matrix\0";
pub(crate) static Media: [u8; b"Media\0".len()] = *b"Media\0";
pub(crate) static Mesh: [u8; b"Mesh\0".len()] = *b"Mesh\0";
pub(crate) static Model: [u8; b"Model\0".len()] = *b"Model\0";
pub(crate) static Name: [u8; b"Name\0".len()] = *b"Name\0";
pub(crate) static NearPlane: [u8; b"NearPlane\0".len()] = *b"NearPlane\0";
pub(crate) static NodeAttributeName: [u8; b"NodeAttributeName\0".len()] = *b"NodeAttributeName\0";
pub(crate) static NodeAttribute: [u8; b"NodeAttribute\0".len()] = *b"NodeAttribute\0";
pub(crate) static Node: [u8; b"Node\0".len()] = *b"Node\0";
pub(crate) static Normals: [u8; b"Normals\0".len()] = *b"Normals\0";
pub(crate) static NormalsIndex: [u8; b"NormalsIndex\0".len()] = *b"NormalsIndex\0";
pub(crate) static NormalsW: [u8; b"NormalsW\0".len()] = *b"NormalsW\0";
pub(crate) static Null: [u8; b"Null\0".len()] = *b"Null\0";
pub(crate) static NurbsCurve: [u8; b"NurbsCurve\0".len()] = *b"NurbsCurve\0";
pub(crate) static NurbsSurfaceOrder: [u8; b"NurbsSurfaceOrder\0".len()] = *b"NurbsSurfaceOrder\0";
pub(crate) static NurbsSurface: [u8; b"NurbsSurface\0".len()] = *b"NurbsSurface\0";
pub(crate) static Nurbs: [u8; b"Nurbs\0".len()] = *b"Nurbs\0";
pub(crate) static OO: [u8; b"OO\0\0".len()] = *b"OO\0\0";
pub(crate) static OP: [u8; b"OP\0\0".len()] = *b"OP\0\0";
pub(crate) static ObjectMetaData: [u8; b"ObjectMetaData\0".len()] = *b"ObjectMetaData\0";
pub(crate) static ObjectType: [u8; b"ObjectType\0".len()] = *b"ObjectType\0";
pub(crate) static Objects: [u8; b"Objects\0".len()] = *b"Objects\0";
pub(crate) static Order: [u8; b"Order\0".len()] = *b"Order\0";
pub(crate) static OriginalUnitScaleFactor: [u8; b"OriginalUnitScaleFactor\0".len()] =
    *b"OriginalUnitScaleFactor\0";
pub(crate) static OriginalUpAxis: [u8; b"OriginalUpAxis\0".len()] = *b"OriginalUpAxis\0";
pub(crate) static OriginalUpAxisSign: [u8; b"OriginalUpAxisSign\0".len()] =
    *b"OriginalUpAxisSign\0";
pub(crate) static OrthoZoom: [u8; b"OrthoZoom\0".len()] = *b"OrthoZoom\0";
pub(crate) static OtherFlags: [u8; b"OtherFlags\0".len()] = *b"OtherFlags\0";
pub(crate) static OuterAngle: [u8; b"OuterAngle\0".len()] = *b"OuterAngle\0";
pub(crate) static PO: [u8; b"PO\0\0".len()] = *b"PO\0\0";
pub(crate) static PP: [u8; b"PP\0\0".len()] = *b"PP\0\0";
pub(crate) static PointsIndex: [u8; b"PointsIndex\0".len()] = *b"PointsIndex\0";
pub(crate) static Points: [u8; b"Points\0".len()] = *b"Points\0";
pub(crate) static PolygonGroup: [u8; b"PolygonGroup\0".len()] = *b"PolygonGroup\0";
pub(crate) static PolygonIndexArray: [u8; b"PolygonIndexArray\0".len()] = *b"PolygonIndexArray\0";
pub(crate) static PolygonVertexIndex: [u8; b"PolygonVertexIndex\0".len()] =
    *b"PolygonVertexIndex\0";
pub(crate) static PoseNode: [u8; b"PoseNode\0".len()] = *b"PoseNode\0";
pub(crate) static Pose: [u8; b"Pose\0".len()] = *b"Pose\0";
pub(crate) static Post_Extrapolation: [u8; b"Post-Extrapolation\0".len()] =
    *b"Post-Extrapolation\0";
pub(crate) static PostRotation: [u8; b"PostRotation\0".len()] = *b"PostRotation\0";
pub(crate) static Pre_Extrapolation: [u8; b"Pre-Extrapolation\0".len()] = *b"Pre-Extrapolation\0";
pub(crate) static PreRotation: [u8; b"PreRotation\0".len()] = *b"PreRotation\0";
pub(crate) static PreviewDivisionLevels: [u8; b"PreviewDivisionLevels\0".len()] =
    *b"PreviewDivisionLevels\0";
pub(crate) static Properties60: [u8; b"Properties60\0".len()] = *b"Properties60\0";
pub(crate) static Properties70: [u8; b"Properties70\0".len()] = *b"Properties70\0";
pub(crate) static PropertyTemplate: [u8; b"PropertyTemplate\0".len()] = *b"PropertyTemplate\0";
pub(crate) static R: [u8; b"R\0\0\0".len()] = *b"R\0\0\0";
pub(crate) static ReferenceStart: [u8; b"ReferenceStart\0".len()] = *b"ReferenceStart\0";
pub(crate) static ReferenceStop: [u8; b"ReferenceStop\0".len()] = *b"ReferenceStop\0";
pub(crate) static ReferenceTime: [u8; b"ReferenceTime\0".len()] = *b"ReferenceTime\0";
pub(crate) static RelativeFileName: [u8; b"RelativeFileName\0".len()] = *b"RelativeFileName\0";
pub(crate) static RelativeFilename: [u8; b"RelativeFilename\0".len()] = *b"RelativeFilename\0";
pub(crate) static RenderDivisionLevels: [u8; b"RenderDivisionLevels\0".len()] =
    *b"RenderDivisionLevels\0";
pub(crate) static Repetition: [u8; b"Repetition\0".len()] = *b"Repetition\0";
pub(crate) static RightCamera: [u8; b"RightCamera\0".len()] = *b"RightCamera\0";
pub(crate) static RootNode: [u8; b"RootNode\0".len()] = *b"RootNode\0";
pub(crate) static Root: [u8; b"Root\0".len()] = *b"Root\0";
pub(crate) static RotationAccumulationMode: [u8; b"RotationAccumulationMode\0".len()] =
    *b"RotationAccumulationMode\0";
pub(crate) static RotationActive: [u8; b"RotationActive\0".len()] = *b"RotationActive\0";
pub(crate) static RotationOffset: [u8; b"RotationOffset\0".len()] = *b"RotationOffset\0";
pub(crate) static RotationOrder: [u8; b"RotationOrder\0".len()] = *b"RotationOrder\0";
pub(crate) static RotationPivot: [u8; b"RotationPivot\0".len()] = *b"RotationPivot\0";
pub(crate) static RotationSpaceForLimitOnly: [u8; b"RotationSpaceForLimitOnly\0".len()] =
    *b"RotationSpaceForLimitOnly\0";
pub(crate) static Rotation: [u8; b"Rotation\0".len()] = *b"Rotation\0";
pub(crate) static S: [u8; b"S\0\0\0".len()] = *b"S\0\0\0";
pub(crate) static ScaleAccumulationMode: [u8; b"ScaleAccumulationMode\0".len()] =
    *b"ScaleAccumulationMode\0";
pub(crate) static ScalingOffset: [u8; b"ScalingOffset\0".len()] = *b"ScalingOffset\0";
pub(crate) static ScalingPivot: [u8; b"ScalingPivot\0".len()] = *b"ScalingPivot\0";
pub(crate) static Scaling: [u8; b"Scaling\0".len()] = *b"Scaling\0";
pub(crate) static SceneInfo: [u8; b"SceneInfo\0".len()] = *b"SceneInfo\0";
pub(crate) static SelectionNode: [u8; b"SelectionNode\0".len()] = *b"SelectionNode\0";
pub(crate) static SelectionSet: [u8; b"SelectionSet\0".len()] = *b"SelectionSet\0";
pub(crate) static ShadingModel: [u8; b"ShadingModel\0".len()] = *b"ShadingModel\0";
pub(crate) static Shape: [u8; b"Shape\0".len()] = *b"Shape\0";
pub(crate) static Shininess: [u8; b"Shininess\0".len()] = *b"Shininess\0";
pub(crate) static Show: [u8; b"Show\0".len()] = *b"Show\0";
pub(crate) static Size: [u8; b"Size\0".len()] = *b"Size\0";
pub(crate) static Skin: [u8; b"Skin\0".len()] = *b"Skin\0";
pub(crate) static SkinningType: [u8; b"SkinningType\0".len()] = *b"SkinningType\0";
pub(crate) static Smoothing: [u8; b"Smoothing\0".len()] = *b"Smoothing\0";
pub(crate) static Smoothness: [u8; b"Smoothness\0".len()] = *b"Smoothness\0";
pub(crate) static SnapOnFrameMode: [u8; b"SnapOnFrameMode\0".len()] = *b"SnapOnFrameMode\0";
pub(crate) static SpecularColor: [u8; b"SpecularColor\0".len()] = *b"SpecularColor\0";
pub(crate) static Step: [u8; b"Step\0".len()] = *b"Step\0";
pub(crate) static SubDeformer: [u8; b"SubDeformer\0".len()] = *b"SubDeformer\0";
pub(crate) static T: [u8; b"T\0\0\0".len()] = *b"T\0\0\0";
pub(crate) static TCDefinition: [u8; b"TCDefinition\0".len()] = *b"TCDefinition\0";
pub(crate) static Take: [u8; b"Take\0".len()] = *b"Take\0";
pub(crate) static Takes: [u8; b"Takes\0".len()] = *b"Takes\0";
pub(crate) static Tangents: [u8; b"Tangents\0".len()] = *b"Tangents\0";
pub(crate) static TangentsIndex: [u8; b"TangentsIndex\0".len()] = *b"TangentsIndex\0";
pub(crate) static TangentsW: [u8; b"TangentsW\0".len()] = *b"TangentsW\0";
pub(crate) static Texture: [u8; b"Texture\0".len()] = *b"Texture\0";
pub(crate) static Texture_alpha: [u8; b"Texture alpha\0".len()] = *b"Texture alpha\0";
pub(crate) static TextureId: [u8; b"TextureId\0".len()] = *b"TextureId\0";
pub(crate) static TextureRotationPivot: [u8; b"TextureRotationPivot\0".len()] =
    *b"TextureRotationPivot\0";
pub(crate) static TextureScalingPivot: [u8; b"TextureScalingPivot\0".len()] =
    *b"TextureScalingPivot\0";
pub(crate) static TextureUV: [u8; b"TextureUV\0".len()] = *b"TextureUV\0";
pub(crate) static TextureUVVerticeIndex: [u8; b"TextureUVVerticeIndex\0".len()] =
    *b"TextureUVVerticeIndex\0";
pub(crate) static Thumbnail: [u8; b"Thumbnail\0".len()] = *b"Thumbnail\0";
pub(crate) static TimeMarker: [u8; b"TimeMarker\0".len()] = *b"TimeMarker\0";
pub(crate) static TimeMode: [u8; b"TimeMode\0".len()] = *b"TimeMode\0";
pub(crate) static TimeProtocol: [u8; b"TimeProtocol\0".len()] = *b"TimeProtocol\0";
pub(crate) static TimeSpanStart: [u8; b"TimeSpanStart\0".len()] = *b"TimeSpanStart\0";
pub(crate) static TimeSpanStop: [u8; b"TimeSpanStop\0".len()] = *b"TimeSpanStop\0";
pub(crate) static TransformLink: [u8; b"TransformLink\0".len()] = *b"TransformLink\0";
pub(crate) static Transform: [u8; b"Transform\0".len()] = *b"Transform\0";
pub(crate) static Translation: [u8; b"Translation\0".len()] = *b"Translation\0";
pub(crate) static TrimNurbsSurface: [u8; b"TrimNurbsSurface\0".len()] = *b"TrimNurbsSurface\0";
pub(crate) static Type: [u8; b"Type\0".len()] = *b"Type\0";
pub(crate) static TypedIndex: [u8; b"TypedIndex\0".len()] = *b"TypedIndex\0";
pub(crate) static UVIndex: [u8; b"UVIndex\0".len()] = *b"UVIndex\0";
pub(crate) static UVSet: [u8; b"UVSet\0".len()] = *b"UVSet\0";
pub(crate) static UVSwap: [u8; b"UVSwap\0".len()] = *b"UVSwap\0";
pub(crate) static UV: [u8; b"UV\0\0".len()] = *b"UV\0\0";
pub(crate) static UnitScaleFactor: [u8; b"UnitScaleFactor\0".len()] = *b"UnitScaleFactor\0";
pub(crate) static UpAxisSign: [u8; b"UpAxisSign\0".len()] = *b"UpAxisSign\0";
pub(crate) static UpAxis: [u8; b"UpAxis\0".len()] = *b"UpAxis\0";
pub(crate) static Version5: [u8; b"Version5\0".len()] = *b"Version5\0";
pub(crate) static VertexCacheDeformer: [u8; b"VertexCacheDeformer\0".len()] =
    *b"VertexCacheDeformer\0";
pub(crate) static VertexCrease: [u8; b"VertexCrease\0".len()] = *b"VertexCrease\0";
pub(crate) static VertexCreaseIndex: [u8; b"VertexCreaseIndex\0".len()] = *b"VertexCreaseIndex\0";
pub(crate) static VertexIndexArray: [u8; b"VertexIndexArray\0".len()] = *b"VertexIndexArray\0";
pub(crate) static Vertices: [u8; b"Vertices\0".len()] = *b"Vertices\0";
pub(crate) static Video: [u8; b"Video\0".len()] = *b"Video\0";
pub(crate) static Visibility: [u8; b"Visibility\0".len()] = *b"Visibility\0";
pub(crate) static Weight: [u8; b"Weight\0".len()] = *b"Weight\0";
pub(crate) static Weights: [u8; b"Weights\0".len()] = *b"Weights\0";
pub(crate) static WrapModeU: [u8; b"WrapModeU\0".len()] = *b"WrapModeU\0";
pub(crate) static WrapModeV: [u8; b"WrapModeV\0".len()] = *b"WrapModeV\0";
pub(crate) static X: [u8; b"X\0\0\0".len()] = *b"X\0\0\0";
pub(crate) static Y: [u8; b"Y\0\0\0".len()] = *b"Y\0\0\0";
pub(crate) static Z: [u8; b"Z\0\0\0".len()] = *b"Z\0\0\0";
pub(crate) static d_X: [u8; b"d|X\0".len()] = *b"d|X\0";
pub(crate) static d_Y: [u8; b"d|Y\0".len()] = *b"d|Y\0";
pub(crate) static d_Z: [u8; b"d|Z\0".len()] = *b"d|Z\0";

// ufbx.c:5597-5900 `ufbxi_strings` (canonical interned-string table; keep sorted)
// `ufbx_string` holds a raw pointer (not auto-`Sync`); the table is
// immutable data pointing at immutable statics, so sharing is sound.
#[repr(transparent)]
pub(crate) struct StringTable(pub [String; 302]);
unsafe impl Sync for StringTable {}

pub(crate) static STRINGS: StringTable = StringTable([
    String::new_c(AllSame.as_ptr(), 7),
    String::new_c(Alphas.as_ptr(), 6),
    String::new_c(AmbientColor.as_ptr(), 12),
    String::new_c(AnimationCurve.as_ptr(), 14),
    String::new_c(AnimationCurveNode.as_ptr(), 18),
    String::new_c(AnimationLayer.as_ptr(), 14),
    String::new_c(AnimationStack.as_ptr(), 14),
    String::new_c(ApertureFormat.as_ptr(), 14),
    String::new_c(ApertureMode.as_ptr(), 12),
    String::new_c(AreaLightShape.as_ptr(), 14),
    String::new_c(AspectH.as_ptr(), 7),
    String::new_c(AspectHeight.as_ptr(), 12),
    String::new_c(AspectRatioMode.as_ptr(), 15),
    String::new_c(AspectW.as_ptr(), 7),
    String::new_c(AspectWidth.as_ptr(), 11),
    String::new_c(Audio.as_ptr(), 5),
    String::new_c(AudioLayer.as_ptr(), 10),
    String::new_c(BaseLayer.as_ptr(), 9),
    String::new_c(BinaryData.as_ptr(), 10),
    String::new_c(BindPose.as_ptr(), 8),
    String::new_c(BindingTable.as_ptr(), 12),
    String::new_c(Binormals.as_ptr(), 9),
    String::new_c(BinormalsIndex.as_ptr(), 14),
    String::new_c(BinormalsW.as_ptr(), 10),
    String::new_c(BlendMode.as_ptr(), 9),
    String::new_c(BlendModes.as_ptr(), 10),
    String::new_c(BlendShape.as_ptr(), 10),
    String::new_c(BlendShapeChannel.as_ptr(), 17),
    String::new_c(BlendWeights.as_ptr(), 12),
    String::new_c(Boundary.as_ptr(), 8),
    String::new_c(BoundaryRule.as_ptr(), 12),
    String::new_c(ByEdge.as_ptr(), 6),
    String::new_c(ByPolygon.as_ptr(), 9),
    String::new_c(ByPolygonVertex.as_ptr(), 15),
    String::new_c(ByVertex.as_ptr(), 8),
    String::new_c(ByVertice.as_ptr(), 9),
    String::new_c(Cache.as_ptr(), 5),
    String::new_c(Camera.as_ptr(), 6),
    String::new_c(CameraProjectionType.as_ptr(), 20),
    String::new_c(CameraStereo.as_ptr(), 12),
    String::new_c(CameraSwitcher.as_ptr(), 14),
    String::new_c(CastLight.as_ptr(), 9),
    String::new_c(CastShadows.as_ptr(), 11),
    String::new_c(Channel.as_ptr(), 7),
    String::new_c(Character.as_ptr(), Character.len() - 1), // sizeof(ufbxi_Character) - 1
    String::new_c(Children.as_ptr(), 8),
    String::new_c(Cluster.as_ptr(), 7),
    String::new_c(Collection.as_ptr(), 10),
    String::new_c(CollectionExclusive.as_ptr(), 19),
    String::new_c(Color.as_ptr(), 5),
    String::new_c(ColorIndex.as_ptr(), 10),
    String::new_c(Colors.as_ptr(), 6),
    String::new_c(Cone_angle.as_ptr(), 10),
    String::new_c(ConeAngle.as_ptr(), 9),
    String::new_c(Connections.as_ptr(), 11),
    String::new_c(Constraint.as_ptr(), Constraint.len() - 1), // sizeof(ufbxi_Constraint) - 1
    String::new_c(Content.as_ptr(), 7),
    String::new_c(CoordAxis.as_ptr(), 9),
    String::new_c(CoordAxisSign.as_ptr(), 13),
    String::new_c(Count.as_ptr(), 5),
    String::new_c(Creator.as_ptr(), 7),
    String::new_c(CurrentTextureBlendMode.as_ptr(), 23),
    String::new_c(CurrentTimeMarker.as_ptr(), 17),
    String::new_c(CustomFrameRate.as_ptr(), 15),
    String::new_c(DecayType.as_ptr(), 9),
    String::new_c(Default.as_ptr(), 7),
    String::new_c(DefaultCamera.as_ptr(), 13),
    String::new_c(Definitions.as_ptr(), 11),
    String::new_c(DeformPercent.as_ptr(), 13),
    String::new_c(Deformer.as_ptr(), 8),
    String::new_c(DiffuseColor.as_ptr(), 12),
    String::new_c(Dimension.as_ptr(), 9),
    String::new_c(Dimensions.as_ptr(), 10),
    String::new_c(DisplayLayer.as_ptr(), 12),
    String::new_c(Document.as_ptr(), 8),
    String::new_c(Documents.as_ptr(), 9),
    String::new_c(EdgeCrease.as_ptr(), 10),
    String::new_c(EdgeIndexArray.as_ptr(), 14),
    String::new_c(Edges.as_ptr(), 5),
    String::new_c(EmissiveColor.as_ptr(), 13),
    String::new_c(Entry.as_ptr(), 5),
    String::new_c(FBXHeaderExtension.as_ptr(), 18),
    String::new_c(FBXHeaderVersion.as_ptr(), 16),
    String::new_c(FBXVersion.as_ptr(), 10),
    String::new_c(FKEffector.as_ptr(), 10),
    String::new_c(FarPlane.as_ptr(), 8),
    String::new_c(FbxPropertyEntry.as_ptr(), 16),
    String::new_c(FbxSemanticEntry.as_ptr(), 16),
    String::new_c(FieldOfView.as_ptr(), 11),
    String::new_c(FieldOfViewX.as_ptr(), 12),
    String::new_c(FieldOfViewY.as_ptr(), 12),
    String::new_c(FileName.as_ptr(), 8),
    String::new_c(Filename.as_ptr(), 8),
    String::new_c(FilmHeight.as_ptr(), 10),
    String::new_c(FilmSqueezeRatio.as_ptr(), 16),
    String::new_c(FilmWidth.as_ptr(), 9),
    String::new_c(FlipNormals.as_ptr(), 11),
    String::new_c(FocalLength.as_ptr(), 11),
    String::new_c(Form.as_ptr(), 4),
    String::new_c(Freeze.as_ptr(), 6),
    String::new_c(FrontAxis.as_ptr(), 9),
    String::new_c(FrontAxisSign.as_ptr(), 13),
    String::new_c(FullWeights.as_ptr(), 11),
    String::new_c(GateFit.as_ptr(), 7),
    String::new_c(GeometricRotation.as_ptr(), 17),
    String::new_c(GeometricScaling.as_ptr(), 16),
    String::new_c(GeometricTranslation.as_ptr(), 20),
    String::new_c(Geometry.as_ptr(), 8),
    String::new_c(GeometryUVInfo.as_ptr(), 14),
    String::new_c(GlobalSettings.as_ptr(), 14),
    String::new_c(Hole.as_ptr(), 4),
    String::new_c(HotSpot.as_ptr(), 7),
    String::new_c(IKEffector.as_ptr(), 10),
    String::new_c(ImageData.as_ptr(), 9),
    String::new_c(Implementation.as_ptr(), 14),
    String::new_c(Indexes.as_ptr(), 7),
    String::new_c(InheritType.as_ptr(), 11),
    String::new_c(InnerAngle.as_ptr(), 10),
    String::new_c(Intensity.as_ptr(), 9),
    String::new_c(IsTheNodeInSet.as_ptr(), 14),
    String::new_c(Key.as_ptr(), 3),
    String::new_c(KeyAttrDataFloat.as_ptr(), 16),
    String::new_c(KeyAttrFlags.as_ptr(), 12),
    String::new_c(KeyAttrRefCount.as_ptr(), 15),
    String::new_c(KeyCount.as_ptr(), 8),
    String::new_c(KeyTime.as_ptr(), 7),
    String::new_c(KeyValueFloat.as_ptr(), 13),
    String::new_c(KeyVer.as_ptr(), 6),
    String::new_c(KnotVector.as_ptr(), 10),
    String::new_c(KnotVectorU.as_ptr(), 11),
    String::new_c(KnotVectorV.as_ptr(), 11),
    String::new_c(Layer.as_ptr(), 5),
    String::new_c(LayerElement.as_ptr(), 12),
    String::new_c(LayerElementBinormal.as_ptr(), 20),
    String::new_c(LayerElementColor.as_ptr(), 17),
    String::new_c(LayerElementEdgeCrease.as_ptr(), 22),
    String::new_c(LayerElementHole.as_ptr(), 16),
    String::new_c(LayerElementMaterial.as_ptr(), 20),
    String::new_c(LayerElementNormal.as_ptr(), 18),
    String::new_c(LayerElementPolygonGroup.as_ptr(), 24),
    String::new_c(LayerElementSmoothing.as_ptr(), 21),
    String::new_c(LayerElementTangent.as_ptr(), 19),
    String::new_c(LayerElementUV.as_ptr(), 14),
    String::new_c(LayerElementVertexCrease.as_ptr(), 24),
    String::new_c(LayerElementVisibility.as_ptr(), 22),
    String::new_c(LayeredTexture.as_ptr(), 14),
    String::new_c(Lcl_Rotation.as_ptr(), 12),
    String::new_c(Lcl_Scaling.as_ptr(), 11),
    String::new_c(Lcl_Translation.as_ptr(), 15),
    String::new_c(LeftCamera.as_ptr(), 10),
    String::new_c(Light.as_ptr(), 5),
    String::new_c(LightType.as_ptr(), 9),
    String::new_c(Limb.as_ptr(), 4),
    String::new_c(LimbLength.as_ptr(), 10),
    String::new_c(LimbNode.as_ptr(), 8),
    String::new_c(Line.as_ptr(), 4),
    String::new_c(Link.as_ptr(), 4),
    String::new_c(LocalStart.as_ptr(), 10),
    String::new_c(LocalStop.as_ptr(), 9),
    String::new_c(LocalTime.as_ptr(), 9),
    String::new_c(LodGroup.as_ptr(), 8),
    String::new_c(MappingInformationType.as_ptr(), 22),
    String::new_c(Marker.as_ptr(), 6),
    String::new_c(Material.as_ptr(), 8),
    String::new_c(MaterialAssignation.as_ptr(), 19),
    String::new_c(Materials.as_ptr(), 9),
    String::new_c(Matrix.as_ptr(), 6),
    String::new_c(Media.as_ptr(), 5),
    String::new_c(Mesh.as_ptr(), 4),
    String::new_c(Model.as_ptr(), 5),
    String::new_c(Name.as_ptr(), 4),
    String::new_c(NearPlane.as_ptr(), 9),
    String::new_c(Node.as_ptr(), 4),
    String::new_c(NodeAttribute.as_ptr(), 13),
    String::new_c(NodeAttributeName.as_ptr(), 17),
    String::new_c(Normals.as_ptr(), 7),
    String::new_c(NormalsIndex.as_ptr(), 12),
    String::new_c(NormalsW.as_ptr(), 8),
    String::new_c(Null.as_ptr(), 4),
    String::new_c(Nurbs.as_ptr(), 5),
    String::new_c(NurbsCurve.as_ptr(), 10),
    String::new_c(NurbsSurface.as_ptr(), 12),
    String::new_c(NurbsSurfaceOrder.as_ptr(), 17),
    String::new_c(OO.as_ptr(), 2),
    String::new_c(OP.as_ptr(), 2),
    String::new_c(ObjectMetaData.as_ptr(), 14),
    String::new_c(ObjectType.as_ptr(), 10),
    String::new_c(Objects.as_ptr(), 7),
    String::new_c(Order.as_ptr(), 5),
    String::new_c(OriginalUnitScaleFactor.as_ptr(), 23),
    String::new_c(OriginalUpAxis.as_ptr(), 14),
    String::new_c(OriginalUpAxisSign.as_ptr(), 18),
    String::new_c(OrthoZoom.as_ptr(), 9),
    String::new_c(OtherFlags.as_ptr(), 10),
    String::new_c(OuterAngle.as_ptr(), 10),
    String::new_c(PO.as_ptr(), 2),
    String::new_c(PP.as_ptr(), 2),
    String::new_c(Points.as_ptr(), 6),
    String::new_c(PointsIndex.as_ptr(), 11),
    String::new_c(PolygonGroup.as_ptr(), 12),
    String::new_c(PolygonIndexArray.as_ptr(), 17),
    String::new_c(PolygonVertexIndex.as_ptr(), 18),
    String::new_c(Pose.as_ptr(), 4),
    String::new_c(PoseNode.as_ptr(), 8),
    String::new_c(Post_Extrapolation.as_ptr(), 18),
    String::new_c(PostRotation.as_ptr(), 12),
    String::new_c(Pre_Extrapolation.as_ptr(), 17),
    String::new_c(PreRotation.as_ptr(), 11),
    String::new_c(PreviewDivisionLevels.as_ptr(), 21),
    String::new_c(Properties60.as_ptr(), 12),
    String::new_c(Properties70.as_ptr(), 12),
    String::new_c(PropertyTemplate.as_ptr(), 16),
    String::new_c(R.as_ptr(), 1),
    String::new_c(ReferenceStart.as_ptr(), 14),
    String::new_c(ReferenceStop.as_ptr(), 13),
    String::new_c(ReferenceTime.as_ptr(), 13),
    String::new_c(RelativeFileName.as_ptr(), 16),
    String::new_c(RelativeFilename.as_ptr(), 16),
    String::new_c(RenderDivisionLevels.as_ptr(), 20),
    String::new_c(Repetition.as_ptr(), 10),
    String::new_c(RightCamera.as_ptr(), 11),
    String::new_c(Root.as_ptr(), 4),
    String::new_c(RootNode.as_ptr(), 8),
    String::new_c(Rotation.as_ptr(), 8),
    String::new_c(RotationAccumulationMode.as_ptr(), 24),
    String::new_c(RotationActive.as_ptr(), 14),
    String::new_c(RotationOffset.as_ptr(), 14),
    String::new_c(RotationOrder.as_ptr(), 13),
    String::new_c(RotationPivot.as_ptr(), 13),
    String::new_c(RotationSpaceForLimitOnly.as_ptr(), 25),
    String::new_c(S.as_ptr(), 1),
    String::new_c(ScaleAccumulationMode.as_ptr(), 21),
    String::new_c(Scaling.as_ptr(), 7),
    String::new_c(ScalingOffset.as_ptr(), 13),
    String::new_c(ScalingPivot.as_ptr(), 12),
    String::new_c(SceneInfo.as_ptr(), 9),
    String::new_c(SelectionNode.as_ptr(), 13),
    String::new_c(SelectionSet.as_ptr(), 12),
    String::new_c(ShadingModel.as_ptr(), 12),
    String::new_c(Shape.as_ptr(), 5),
    String::new_c(Shininess.as_ptr(), 9),
    String::new_c(Show.as_ptr(), 4),
    String::new_c(Size.as_ptr(), 4),
    String::new_c(Skin.as_ptr(), 4),
    String::new_c(SkinningType.as_ptr(), 12),
    String::new_c(Smoothing.as_ptr(), 9),
    String::new_c(Smoothness.as_ptr(), 10),
    String::new_c(SnapOnFrameMode.as_ptr(), 15),
    String::new_c(SpecularColor.as_ptr(), 13),
    String::new_c(Step.as_ptr(), 4),
    String::new_c(SubDeformer.as_ptr(), 11),
    String::new_c(T.as_ptr(), 1),
    String::new_c(TCDefinition.as_ptr(), 12),
    String::new_c(Take.as_ptr(), 4),
    String::new_c(Takes.as_ptr(), 5),
    String::new_c(Tangents.as_ptr(), 8),
    String::new_c(TangentsIndex.as_ptr(), 13),
    String::new_c(TangentsW.as_ptr(), 9),
    String::new_c(Texture.as_ptr(), 7),
    String::new_c(Texture_alpha.as_ptr(), 13),
    String::new_c(TextureId.as_ptr(), 9),
    String::new_c(TextureRotationPivot.as_ptr(), 20),
    String::new_c(TextureScalingPivot.as_ptr(), 19),
    String::new_c(TextureUV.as_ptr(), 9),
    String::new_c(TextureUVVerticeIndex.as_ptr(), 21),
    String::new_c(Thumbnail.as_ptr(), 9),
    String::new_c(TimeMarker.as_ptr(), 10),
    String::new_c(TimeMode.as_ptr(), 8),
    String::new_c(TimeProtocol.as_ptr(), 12),
    String::new_c(TimeSpanStart.as_ptr(), 13),
    String::new_c(TimeSpanStop.as_ptr(), 12),
    String::new_c(Transform.as_ptr(), 9),
    String::new_c(TransformLink.as_ptr(), 13),
    String::new_c(Translation.as_ptr(), 11),
    String::new_c(TrimNurbsSurface.as_ptr(), 16),
    String::new_c(Type.as_ptr(), 4),
    String::new_c(TypedIndex.as_ptr(), 10),
    String::new_c(UV.as_ptr(), 2),
    String::new_c(UVIndex.as_ptr(), 7),
    String::new_c(UVSet.as_ptr(), 5),
    String::new_c(UVSwap.as_ptr(), 6),
    String::new_c(UnitScaleFactor.as_ptr(), 15),
    String::new_c(UpAxis.as_ptr(), 6),
    String::new_c(UpAxisSign.as_ptr(), 10),
    String::new_c(Version5.as_ptr(), 8),
    String::new_c(VertexCacheDeformer.as_ptr(), 19),
    String::new_c(VertexCrease.as_ptr(), 12),
    String::new_c(VertexCreaseIndex.as_ptr(), 17),
    String::new_c(VertexIndexArray.as_ptr(), 16),
    String::new_c(Vertices.as_ptr(), 8),
    String::new_c(Video.as_ptr(), 5),
    String::new_c(Visibility.as_ptr(), 10),
    String::new_c(Weight.as_ptr(), 6),
    String::new_c(Weights.as_ptr(), 7),
    String::new_c(WrapModeU.as_ptr(), 9),
    String::new_c(WrapModeV.as_ptr(), 9),
    String::new_c(X.as_ptr(), 1),
    String::new_c(Y.as_ptr(), 1),
    String::new_c(Z.as_ptr(), 1),
    String::new_c(d_X.as_ptr(), 3),
    String::new_c(d_Y.as_ptr(), 3),
    String::new_c(d_Z.as_ptr(), 3),
]);

// ufbx.c:5902 `ufbxi_one_vec3`
// C: `{ 1.0f, 1.0f, 1.0f }` — float literals widen exactly to `ufbx_real`.
pub(crate) static ONE_VEC3: Vec3 = Vec3 {
    x: 1.0,
    y: 1.0,
    z: 1.0,
};

// ufbx.c:5904 `UFBXI_PI`
// C: `((ufbx_real)3.14159265358979323846)` — an unsuffixed (`double`) constant
// narrowed to `ufbx_real`, so spell the literal `f64` and narrow explicitly.
pub(crate) const PI: Real = 3.14159265358979323846_f64 as Real;
// ufbx.c:5905 `UFBXI_DPI`
pub(crate) const DPI: f64 = 3.14159265358979323846;
// ufbx.c:5906 `UFBXI_DEG_TO_RAD`
// C: `((ufbx_real)(UFBXI_PI / 180.0))` — `UFBXI_PI` is `ufbx_real` but `180.0`
// is `double`, so the division happens in `double` and narrows on the cast.
pub(crate) const DEG_TO_RAD: Real = (as_f64!(PI) / 180.0) as Real;
// ufbx.c:5907 `UFBXI_RAD_TO_DEG`
// C: `((ufbx_real)(180.0 / UFBXI_PI))` — same `double` division, same narrowing.
pub(crate) const RAD_TO_DEG: Real = (180.0 / as_f64!(PI)) as Real;
// ufbx.c:5908 `UFBXI_DEG_TO_RAD_DOUBLE`
pub(crate) const DEG_TO_RAD_DOUBLE: f64 = DPI / 180.0;
// ufbx.c:5909 `UFBXI_RAD_TO_DEG_DOUBLE`
pub(crate) const RAD_TO_DEG_DOUBLE: f64 = 180.0 / DPI;
// ufbx.c:5910 `UFBXI_MM_TO_INCH`
// C: `((ufbx_real)0.0393700787)` — a `double` constant narrowed to `ufbx_real`.
pub(crate) const MM_TO_INCH: Real = 0.0393700787_f64 as Real;

// ufbx.c:5912-5915 `ufbxi_add3`
#[inline(always)]
pub(crate) fn add3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3 {
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    }
}

// ufbx.c:5917-5920 `ufbxi_sub3`
#[inline(always)]
pub(crate) fn sub3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3 {
        x: a.x - b.x,
        y: a.y - b.y,
        z: a.z - b.z,
    }
}

// ufbx.c:5922-5925 `ufbxi_mul3`
#[inline(always)]
pub(crate) fn mul3(a: Vec3, b: Real) -> Vec3 {
    Vec3 {
        x: a.x * b,
        y: a.y * b,
        z: a.z * b,
    }
}

// ufbx.c:5927-5931 `ufbxi_lerp3`
#[inline(always)]
pub(crate) fn lerp3(a: Vec3, b: Vec3, t: Real) -> Vec3 {
    // C: `ufbx_real u = 1.0f - t;`
    let u: Real = 1.0 - t;

    Vec3 {
        x: a.x * u + b.x * t,
        y: a.y * u + b.y * t,
        z: a.z * u + b.z * t,
    }
}

// ufbx.c:5933-5935 `ufbxi_dot3`
#[inline(always)]
pub(crate) fn dot3(a: Vec3, b: Vec3) -> Real {
    a.x * b.x + a.y * b.y + a.z * b.z
}

// ufbx.c:5937-5940 `ufbxi_length3`
#[inline(always)]
pub(crate) fn length3(v: Vec3) -> Real {
    // C: `(ufbx_real)ufbx_sqrt(v.x*v.x + v.y*v.y + v.z*v.z)` — the `ufbx_real`
    // sum promotes to `double` at the `ufbx_sqrt` call and the result narrows
    // back on the explicit cast.
    math::sqrt(as_f64!(v.x * v.x + v.y * v.y + v.z * v.z)) as Real
}

// ufbx.c:5942-5945 `ufbxi_min3`
#[inline(always)]
pub(crate) fn min3(v: Vec3) -> Real {
    min_real(min_real(v.x, v.y), v.z)
}

// ufbx.c:5947-5950 `ufbxi_cross3`
#[inline(always)]
pub(crate) fn cross3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

// ufbx.c:5952-5960 `ufbxi_normalize3`
#[inline(always)]
pub(crate) fn normalize3(a: Vec3) -> Vec3 {
    // C: `ufbx_real len = (ufbx_real)ufbx_sqrt(ufbxi_dot3(a, a));` — the
    // `ufbx_real` dot product promotes to `double` at the `ufbx_sqrt` call and
    // the result narrows back on the explicit cast.
    let len: Real = math::sqrt(dot3(a, a) as f64) as Real;
    if len > math::EPSILON {
        mul3(a, 1.0 / len)
    } else {
        // C: `ufbx_vec3 zero = { (ufbx_real)0 };`

        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

// ufbx.c:5962-5965 `ufbxi_neg3`
#[inline(always)]
pub(crate) fn neg3(a: Vec3) -> Vec3 {
    Vec3 {
        x: -a.x,
        y: -a.y,
        z: -a.z,
    }
}

// ufbx.c:5967-5970 `ufbxi_distsq2`
#[inline(always)]
pub(crate) fn distsq2(a: Vec2, b: Vec2) -> Real {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

// ufbx.c:5972-5974 `ufbxi_slow_normalize3`
#[inline(never)]
pub(crate) unsafe fn slow_normalize3(a: *const Vec3) -> Vec3 {
    // SAFETY: the caller vouches `a` addresses a live `Vec3`.
    normalize3(unsafe { *a })
}

// ufbx.c:5976-5978 `ufbxi_slow_normalized_cross3`
#[inline(never)]
pub(crate) unsafe fn slow_normalized_cross3(a: *const Vec3, b: *const Vec3) -> Vec3 {
    // SAFETY: the caller vouches `a`/`b` address live `Vec3`s.
    normalize3(cross3(unsafe { *a }, unsafe { *b }))
}

// CONTINUATION POINT: `// -- String pool` (ufbx.c:4895-5286) and
// `// -- String constants` (ufbx.c:5288-5979) complete. Next banner:
// ufbx.c:6175 `// -- Type definitions` (`ufbxi_value` / `ufbxi_node`, owned by
// the parse units).

#[cfg(test)]
mod tests {
    // Tests assert invariants on math `const`s (e.g. `assert_eq!(PI, DPI as Real)`);
    // asserting on a constant is the intent, so `assertions_on_constants` is allowed here.
    #![allow(clippy::assertions_on_constants)]
    use super::*;
    use crate::generated::UnicodeErrorHandling;
    use crate::native::allocator::{init_ator, Allocator};
    use crate::native::buf::buf_free;
    use crate::native::hash::map_init;
    use core::mem::MaybeUninit;

    struct Fixture {
        err: Error,
        ator: Allocator,
        pool: StringPool,
    }

    fn make_fixture(handling: UnicodeErrorHandling) -> Box<Fixture> {
        // SAFETY: `Allocator` and `StringPool` are plain pointer/integer/enum
        // aggregates whose all-zero bit pattern is a valid value for every
        // field.
        let mut fx: Box<Fixture> = unsafe {
            Box::new(Fixture {
                err: Error::default(),
                ator: MaybeUninit::zeroed().assume_init(),
                pool: MaybeUninit::zeroed().assume_init(),
            })
        };
        // SAFETY: the allocator is initialized from the fixture's own `err`
        // and a `'static` NUL-terminated name literal.
        unsafe {
            init_ator(&mut fx.err, &mut fx.ator, core::ptr::null(), c"test");
        }
        let ator = &mut fx.ator as *mut Allocator;
        fx.pool.error = &mut fx.err;
        fx.pool.buf.ator = ator;
        // C: `ufbxi_map_init(&uc->string_pool.map, &uc->ator_tmp, &ufbxi_map_cmp_string, NULL)`
        // SAFETY: initializing the fixture's own zeroed map against the
        // fixture's own allocator; `map_cmp_string` takes no user data, so the
        // null `user` is what it expects.
        unsafe {
            map_init(
                &mut fx.pool.map,
                ator,
                map_cmp_string,
                core::ptr::null_mut(),
            );
        }
        fx.pool.initial_size = 64; // ufbx.c:7192 `string_pool.initial_size = 1024` (smaller for tests)
        fx.pool.error_handling = handling;
        fx.pool.warnings = core::ptr::null_mut();
        fx
    }

    fn free_fixture(fx: &mut Fixture) {
        // SAFETY: `fx` is a fixture the caller owns exclusively; its pool and
        // buf were built by `make_fixture` over that same fixture's allocator,
        // and this teardown is their last use.
        unsafe {
            buf_free(&mut fx.pool.buf);
            string_pool_temp_free(&mut fx.pool);
        }
        assert_eq!(fx.ator.current_size, 0);
    }

    fn push(fx: &mut Fixture, s: &[u8]) -> (*const u8, usize) {
        let mut out_len = s.len();
        // SAFETY: `s.as_ptr()`/`s.len()` describe exactly one live run;
        // `out_len` is an unaliased local out-param; `fx.pool` is the
        // fixture's own initialized pool.
        let ptr_ = unsafe { push_string(&mut fx.pool, s.as_ptr(), s.len(), &mut out_len, false) };
        (ptr_, out_len)
    }

    fn s(b: &'static [u8]) -> String {
        String::new_c(b.as_ptr(), b.len())
    }

    unsafe fn bytes<'a>(ptr_: *const u8, len: usize) -> &'a [u8] {
        // SAFETY: the caller vouches `ptr_` is readable for `len` bytes and that
        // the borrowed run outlives the returned slice.
        unsafe { core::slice::from_raw_parts(ptr_, len) }
    }

    #[test]
    fn test_push_string_interning() {
        unsafe {
            let mut fx = make_fixture(UnicodeErrorHandling::ReplacementCharacter);

            let (a, a_len) = push(&mut fx, b"Geometry");
            let (b, b_len) = push(&mut fx, b"Geometry");
            let (c, _) = push(&mut fx, b"Material");
            assert!(!a.is_null());
            // Same bytes intern to the SAME canonical pointer.
            assert_eq!(a, b);
            assert_ne!(a, c);
            assert_eq!(a_len, 8);
            assert_eq!(b_len, 8);
            assert_eq!(bytes(a, 8), b"Geometry");
            // Interned copies are NUL-terminated in the arena.
            assert_eq!(*a.add(8), 0);

            // Empty string: canonical `ufbxi_empty_char`, no allocation.
            let (e, e_len) = push(&mut fx, b"");
            assert_eq!(e, EMPTY_CHAR.as_ptr());
            assert_eq!(e_len, 0);

            // raw=true: bytes are interned untouched even when invalid UTF-8.
            let mut raw_len = 2usize;
            let r = push_string(&mut fx.pool, b"\xff\xfe".as_ptr(), 2, &mut raw_len, true);
            assert!(!r.is_null());
            assert_eq!(raw_len, 2);
            assert_eq!(bytes(r, 2), b"\xff\xfe");

            free_fixture(&mut fx);
        }
    }

    #[test]
    fn test_push_string_sanitization_modes() {
        unsafe {
            // (input, handling, expected sanitized bytes)
            let cases: &[(&[u8], UnicodeErrorHandling, &[u8])] = &[
                (
                    b"a\xffb",
                    UnicodeErrorHandling::ReplacementCharacter,
                    b"a\xef\xbf\xbdb",
                ),
                (b"a\xffb", UnicodeErrorHandling::Underscore, b"a_b"),
                (b"a\xffb", UnicodeErrorHandling::QuestionMark, b"a?b"),
                (b"a\xffb", UnicodeErrorHandling::Remove, b"ab"),
                (b"a\xffb", UnicodeErrorHandling::UnsafeIgnore, b"a\xffb"),
                // Embedded NUL is a unicode error too (C: `if (c != 0)`).
                (b"a\x00b", UnicodeErrorHandling::QuestionMark, b"a?b"),
                // Overlong encoding rejected, valid 2-byte kept.
                (
                    b"\xc0\xaf\xc2\x80",
                    UnicodeErrorHandling::QuestionMark,
                    b"??\xc2\x80",
                ),
                // Surrogate range D800-DFFF rejected (ED A0 80 -> one '?' per
                // byte, resyncing after each), U+FFFF kept.
                (
                    b"\xed\xa0\x80\xef\xbf\xbf",
                    UnicodeErrorHandling::QuestionMark,
                    b"???\xef\xbf\xbf",
                ),
                // 4-byte: U+10FFFF kept, F4 90 80 80 (> U+10FFFF) rejected.
                (
                    b"\xf4\x8f\xbf\xbf\xf4\x90\x80\x80",
                    UnicodeErrorHandling::QuestionMark,
                    b"\xf4\x8f\xbf\xbf????",
                ),
                // Truncated sequence at end of string.
                (b"ab\xe2\x82", UnicodeErrorHandling::QuestionMark, b"ab??"),
            ];
            for &(input, handling, expect) in cases {
                let mut fx = make_fixture(handling);
                let (p, len) = push(&mut fx, input);
                assert!(!p.is_null());
                assert_eq!(
                    bytes(p, len),
                    expect,
                    "input {:?} handling {:?}",
                    input,
                    handling
                );
                free_fixture(&mut fx);
            }
        }
    }

    #[test]
    fn test_push_string_abort_loading() {
        unsafe {
            let mut fx = make_fixture(UnicodeErrorHandling::AbortLoading);
            let mut out_len = 3usize;
            let p = push_string(&mut fx.pool, b"a\xffb".as_ptr(), 3, &mut out_len, false);
            assert!(p.is_null());
            assert_eq!(
                bytes(fx.err.description.data, fx.err.description.length),
                b"Invalid UTF-8"
            );
            free_fixture(&mut fx);
        }
    }

    #[test]
    fn test_push_sanitized_string() {
        unsafe {
            let mut fx = make_fixture(UnicodeErrorHandling::ReplacementCharacter);

            // Valid UTF-8: raw_length = length, utf8_length = 0, interned copy.
            let mut san = SanitizedString {
                raw_data: core::ptr::null(),
                raw_length: 0,
                utf8_length: 0,
            };
            let hash = hash_string(b"Model".as_ptr(), 5);
            assert_eq!(
                push_sanitized_string(&mut fx.pool, &mut san, b"Model".as_ptr(), 5, hash, false),
                Ok(())
            );
            assert_eq!(san.raw_length, 5);
            assert_eq!(san.utf8_length, 0);
            assert_eq!(bytes(san.raw_data, 5), b"Model");
            assert_eq!(*san.raw_data.add(5), 0);

            // Same string again: dedup to the same pointer.
            let mut san2 = SanitizedString {
                raw_data: core::ptr::null(),
                raw_length: 0,
                utf8_length: 0,
            };
            assert_eq!(
                push_sanitized_string(&mut fx.pool, &mut san2, b"Model".as_ptr(), 5, hash, false),
                Ok(())
            );
            assert_eq!(san2.raw_data, san.raw_data);

            // Invalid UTF-8 with push_both: `raw\0utf8` packing.
            let inp = b"a\xffb";
            let h = hash_string(inp.as_ptr(), 3);
            let mut san3 = SanitizedString {
                raw_data: core::ptr::null(),
                raw_length: 0,
                utf8_length: 0,
            };
            assert_eq!(
                push_sanitized_string(&mut fx.pool, &mut san3, inp.as_ptr(), 3, h, false),
                Ok(())
            );
            assert_eq!(san3.raw_length, 3);
            assert_eq!(san3.utf8_length, 5); // "a" + EF BF BD + "b"
            assert_eq!(bytes(san3.raw_data, 3), b"a\xffb");
            assert_eq!(*san3.raw_data.add(3), 0);
            // UTF-8 data follows at `raw_length+1`.
            assert_eq!(bytes(san3.raw_data.add(4), 5), b"a\xef\xbf\xbdb");

            // raw=true: no sanitization even for invalid bytes.
            let mut san4 = SanitizedString {
                raw_data: core::ptr::null(),
                raw_length: 0,
                utf8_length: 0,
            };
            assert_eq!(
                push_sanitized_string(&mut fx.pool, &mut san4, inp.as_ptr(), 3, h, true),
                Ok(())
            );
            assert_eq!(san4.raw_length, 3);
            assert_eq!(san4.utf8_length, 0);
            assert_eq!(bytes(san4.raw_data, 3), b"a\xffb");

            free_fixture(&mut fx);
        }
    }

    #[test]
    fn test_push_string_place_str_and_blob() {
        unsafe {
            let mut fx = make_fixture(UnicodeErrorHandling::ReplacementCharacter);

            let mut str_ = s(b"Vertices");
            assert_eq!(
                push_string_place_str(&mut fx.pool, &mut str_, false),
                Ok(())
            );
            assert_eq!(str_.length, 8);
            assert_eq!(bytes(str_.data, 8), b"Vertices");
            let interned = str_.data;
            // NULL data with zero length is allowed.
            let mut str2 = String::new_c(core::ptr::null(), 0);
            assert_eq!(
                push_string_place_str(&mut fx.pool, &mut str2, false),
                Ok(())
            );
            assert_eq!(str2.data, EMPTY_CHAR.as_ptr());

            // Blob shares the pool with strings (same canonical pointer).
            let mut blob = MaybeUninit::<Blob>::zeroed().assume_init();
            blob.data = b"Vertices".as_ptr();
            blob.size = 8;
            assert_eq!(
                push_string_place_blob(&mut fx.pool, &mut blob, true),
                Ok(())
            );
            assert_eq!(blob.data, interned);
            // Zero-size blob: data forced to NULL.
            let mut blob2 = MaybeUninit::<Blob>::zeroed().assume_init();
            blob2.data = b"x".as_ptr();
            blob2.size = 0;
            assert_eq!(
                push_string_place_blob(&mut fx.pool, &mut blob2, true),
                Ok(())
            );
            assert!(blob2.data.is_null());

            free_fixture(&mut fx);
        }
    }

    #[test]
    fn test_string_helpers() {
        unsafe {
            assert!(str_equal_raw(s(b"abc"), s(b"abc")));
            assert!(!str_equal_raw(s(b"abc"), s(b"abd")));
            assert!(!str_equal_raw(s(b"abc"), s(b"ab")));

            assert!(str_less_raw(s(b"ab"), s(b"abc")));
            assert!(str_less_raw(s(b"abc"), s(b"abd")));
            assert!(!str_less_raw(s(b"abc"), s(b"abc")));
            // memcmp compares as UNSIGNED chars.
            assert!(str_less_raw(s(b"a"), s(b"\xff")));

            assert_eq!(str_cmp_raw(s(b"abc"), s(b"abc")), 0);
            assert!(str_cmp_raw(s(b"ab"), s(b"abc")) < 0);
            assert!(str_cmp_raw(s(b"abd"), s(b"abc")) > 0);

            let c = str_c(b"Model\0".as_ptr());
            assert_eq!(c.length, 5);

            assert!(starts_with(s(b"Lcl Rotation"), s(b"Lcl ")));
            assert!(!starts_with(s(b"Lcl"), s(b"Lcl ")));
            assert!(ends_with(s(b"NormalsW"), s(b"W")));
            assert!(!ends_with(s(b"W"), s(b"sW")));

            let mut t = s(b"d|X");
            assert!(remove_prefix_len(&mut t, b"d|".as_ptr(), 2));
            assert_eq!(bytes(t.data, t.length), b"X");
            assert!(!remove_prefix_str(&mut t, s(b"Y")));
            let mut u = s(b"FileName");
            assert!(remove_suffix_c(&mut u, b"Name\0".as_ptr()));
            assert_eq!(bytes(u.data, u.length), b"File");
            assert!(!remove_suffix_len(&mut u, b"x".as_ptr(), 1));

            let sf = safe_string(core::ptr::null(), 0);
            assert_eq!(sf.data, EMPTY_CHAR.as_ptr());
            assert_eq!(sf.length, 0);
        }
    }

    #[test]
    fn test_concat_key_and_cmp() {
        unsafe {
            // Key packs the first 4 bytes big-endian across parts.
            let parts = [s(b"ab"), s(b"cd")];
            assert_eq!(get_concat_key(parts.as_ptr(), 2), 0x61626364);
            // SIZE_MAX length -> strlen(data) (C-string part).
            let parts2 = [String::new_c(b"abcd\0".as_ptr(), usize::MAX)];
            assert_eq!(get_concat_key(parts2.as_ptr(), 1), 0x61626364);
            let short = [s(b"a")];
            assert_eq!(get_concat_key(short.as_ptr(), 1), 0x61000000);

            let ref_ = s(b"abcd");
            assert_eq!(concat_str_cmp(&ref_, parts.as_ptr(), 2), 0);
            let parts3 = [s(b"ab"), s(b"ce")];
            assert!(concat_str_cmp(&ref_, parts3.as_ptr(), 2) < 0);
            // Ref shorter than concat -> -1; longer -> +1.
            let ref_short = s(b"abc");
            assert_eq!(concat_str_cmp(&ref_short, parts.as_ptr(), 2), -1);
            let ref_long = s(b"abcde");
            assert_eq!(concat_str_cmp(&ref_long, parts.as_ptr(), 2), 1);
        }
    }

    #[test]
    fn test_map_cmp_string() {
        unsafe {
            let a = s(b"abc");
            let b = s(b"abd");
            let pa = &a as *const String as *const c_void;
            let pb = &b as *const String as *const c_void;
            assert!(map_cmp_string(core::ptr::null_mut(), pa, pb) < 0);
            assert!(map_cmp_string(core::ptr::null_mut(), pb, pa) > 0);
            assert_eq!(map_cmp_string(core::ptr::null_mut(), pa, pa), 0);
        }
    }

    #[test]
    fn test_string_constants_table() {
        unsafe {
            // Mirror of C `ufbxi_check_string_ordering` (ufbx.c:11414-11424):
            // the table must be strictly sorted by `ufbxi_str_less`.
            let mut prev = String::new_c(EMPTY_CHAR.as_ptr(), 0);
            for str_ in STRINGS.0.iter() {
                assert!(
                    str_less_raw(prev, *str_) || (prev.length == 0 && str_.length > 0),
                    "ufbxi_strings out of order at {:?}",
                    core::str::from_utf8(bytes(str_.data, str_.length))
                );
                prev = *str_;
            }
            // NUL conventions: every entry's length equals strlen of its data
            // (padded constants like "OO\0" have length 2), and every backing
            // array is NUL-terminated at `length`.
            for str_ in STRINGS.0.iter() {
                assert_eq!(strlen(str_.data), str_.length);
                assert_eq!(*str_.data.add(str_.length), 0);
            }
            // Spot-check the canonical pointers and padded sizes.
            assert_eq!(STRINGS.0[0].data, AllSame.as_ptr());
            assert_eq!(core::mem::size_of_val(&OO), 4); // "OO\0" + implicit NUL
            assert_eq!(core::mem::size_of_val(&R), 4); // "R\0\0" + implicit NUL
            assert_eq!(core::mem::size_of_val(&UV), 4); // "UV\0" + implicit NUL
            assert_eq!(core::mem::size_of_val(&d_X), 4); // "d|X" + implicit NUL
            assert_eq!(STRINGS.0.len(), 302);
        }
    }

    #[test]
    fn test_constants_intern_to_their_lengths() {
        unsafe {
            // Interning a constant's bytes yields a pooled copy with identical
            // bytes (the canonicalization to constant ADDRESSES happens in the
            // reader via `ufbxi_strings`, ufbx.c:11419 / 26609-26622).
            let mut fx = make_fixture(UnicodeErrorHandling::ReplacementCharacter);
            for str_ in STRINGS.0.iter() {
                let mut out_len = str_.length;
                let p = push_string(&mut fx.pool, str_.data, str_.length, &mut out_len, false);
                assert!(!p.is_null());
                assert_eq!(out_len, str_.length);
                assert_eq!(bytes(p, out_len), bytes(str_.data, str_.length));
            }
            free_fixture(&mut fx);
        }
    }

    #[test]
    fn test_vec_math() {
        let a = Vec3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let b = Vec3 {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let v = add3(a, b);
        assert_eq!((v.x, v.y, v.z), (5.0, 7.0, 9.0));
        let v = sub3(b, a);
        assert_eq!((v.x, v.y, v.z), (3.0, 3.0, 3.0));
        let v = mul3(a, 2.0);
        assert_eq!((v.x, v.y, v.z), (2.0, 4.0, 6.0));
        let v = lerp3(a, b, 0.5);
        assert_eq!((v.x, v.y, v.z), (2.5, 3.5, 4.5));
        assert_eq!(dot3(a, b), 32.0);
        assert_eq!(
            length3(Vec3 {
                x: 3.0,
                y: 4.0,
                z: 0.0
            }),
            5.0
        );
        assert_eq!(
            min3(Vec3 {
                x: 2.0,
                y: 1.0,
                z: 3.0
            }),
            1.0
        );
        let v = cross3(
            Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        );
        assert_eq!((v.x, v.y, v.z), (0.0, 0.0, 1.0));
        let v = normalize3(Vec3 {
            x: 0.0,
            y: 0.0,
            z: 2.0,
        });
        assert_eq!((v.x, v.y, v.z), (0.0, 0.0, 1.0));
        // Degenerate: below-epsilon length normalizes to zero.
        let v = normalize3(Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
        assert_eq!((v.x, v.y, v.z), (0.0, 0.0, 0.0));
        let v = neg3(a);
        assert_eq!((v.x, v.y, v.z), (-1.0, -2.0, -3.0));
        assert_eq!(
            distsq2(Vec2 { x: 1.0, y: 2.0 }, Vec2 { x: 4.0, y: 6.0 }),
            25.0
        );
        unsafe {
            let v = slow_normalize3(&Vec3 {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            });
            assert_eq!((v.x, v.y, v.z), (1.0, 0.0, 0.0));
            let v = slow_normalized_cross3(
                &Vec3 {
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                },
                &Vec3 {
                    x: 0.0,
                    y: 2.0,
                    z: 0.0,
                },
            );
            assert_eq!((v.x, v.y, v.z), (0.0, 0.0, 1.0));
        }
        assert_eq!(ONE_VEC3.x, 1.0);
        // `UFBXI_PI` is `UFBXI_DPI` narrowed to `ufbx_real`.
        assert_eq!(PI, DPI as Real);
        assert!((DEG_TO_RAD * RAD_TO_DEG - 1.0).abs() <= 4.0 * Real::EPSILON);
        assert!(DEG_TO_RAD_DOUBLE * RAD_TO_DEG_DOUBLE - 1.0 < 1e-15);
        assert!(MM_TO_INCH > 0.0393 && MM_TO_INCH < 0.0394);
    }
}
