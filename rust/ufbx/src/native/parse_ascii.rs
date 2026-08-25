//! Port of the `// -- ASCII parsing` banner section of `ufbx.c`
//! (starting at ufbx.c:9404). This module covers the section in full: the
//! refill/yield/peek/next source machinery, the magic-comment
//! version scanner, the space character class, the token string buffers, the
//! `ufbxi_ascii_span` array capture, the tokenizer plus its
//! `try_ignore_string`/`accept` front end (ufbx.c:9404-9908), and the
//! value/array parsing layer — the inline int/float array readers, the
//! `ufbxi_ascii_array_task` span scanner, and the base64 decoder
//! (ufbx.c:9910-10282), and finally the node-level parser
//! `ufbxi_ascii_parse_node` (ufbx.c:10285-10694).
//!
//! The `ufbxi_ascii` / `ufbxi_ascii_token` context structs are owned by
//! `native::parse` (they are declared with the rest of `ufbxi_context` at
//! ufbx.c:6252-6291) — used from here, not redefined. Number parsing routes
//! through `native::float_parse` (`ufbxi_parse_int64`/`ufbxi_parse_double`),
//! including the `UFBXI_PARSE_DOUBLE_AS_BINARY32` flag override for
//! `ua->parse_as_f32`.
//!
//! C `char` note: this section compares source bytes against ASCII ranges
//! (`c >= 'A' && c <= 'Z'`, ...). Every such comparison is bounded on BOTH
//! ends, so a byte >= 0x80 fails the range identically whether `char` is
//! signed (the oracle targets) or the `u8` used here; the PORTING.md
//! "`char` (value, where signedness is observable)" exception therefore does
//! NOT apply and plain `u8` storage is used throughout.
//!
//! Threading: `ua->retain_buf` / `ua->src_is_retained` / `uc->tmp_ascii_spans`
//! are used by the single-threaded path as well, so they carry no fork. The
//! `ufbxi_ascii_array_task` family (ufbx.c:9966-10158) IS the threaded array
//! worker; the only place it forks off the single-threaded path is its
//! thread-pool submission site in `ufbxi_ascii_parse_node`
//! (ufbx.c:10653-10659), which dispatches `ascii_array_task_fn` through
//! `ufbxi_thread_pool_create_task` / `ufbxi_thread_pool_run_task`
//! (`native::thread`) and falls back to the inline
//! `ufbxi_ascii_array_task_imp` when no task slot is available.
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
use core::ffi::c_void;

use crate::generated::{Exporter, WarningType};
use crate::native::allocator::ZERO_SIZE_BUFFER;
use crate::native::buf::{pop_size, push, push_pop_size, push_size, push_size_zero, Buf, BufView};
use crate::native::error::{
    memchr, memcmp, ufbxi_check, ufbxi_check_msg, ufbxi_check_return, ufbxi_check_return_msg,
    ufbxi_fail, ufbxi_fail_msg, Fail, EMPTY_CHAR,
};
use crate::native::float_parse::{
    parse_double, parse_double_init_flags, parse_inf_nan, parse_int64, PARSE_DOUBLE_AS_BINARY32,
};
use crate::native::hash::hash_string_check_ascii;
use crate::native::parse::{
    array_type_size, is_array_node, is_raw_string, normalize_array_type, report_progress,
    update_parse_state, ArrayInfo, Ascii, AsciiToken, AsciiTokenView, AsciiView, Context, Node,
    ParseState, Value, ValueArray, ValueType, ARRAY_FLAG_ACCURATE_F32, ARRAY_FLAG_PAD_BEGIN,
    ARRAY_FLAG_RESULT, ARRAY_FLAG_TMP_BUF, MAX_NODE_DEPTH, MAX_NON_ARRAY_VALUES,
};
use crate::native::platform::{
    add_ptr, f64_to_i32, f64_to_i64, max_sz, min32, sub_ptr, to_size, ufbx_assert,
    ufbxi_dev_assert, MIN_THREADED_ASCII_VALUES,
};
use crate::native::string_pool::{
    push_sanitized_string, push_string, push_string_place_str, SanitizedStringView,
};
use crate::native::thread::{thread_pool_create_task, thread_pool_run_task, Task};
use crate::native::view::{view_read, view_write, Mut, View};
use crate::native::warnings::ufbxi_warnf;
use crate::prelude::{slice_from_ptr, Real, String, StringView};

// ufbx.c:9406 `#define UFBXI_ASCII_END '\0'`
pub(crate) const ASCII_END: u8 = b'\0';
// ufbx.c:9407 `#define UFBXI_ASCII_NAME 'N'`
pub(crate) const ASCII_NAME: u8 = b'N';
// ufbx.c:9408 `#define UFBXI_ASCII_BARE_WORD 'B'`
pub(crate) const ASCII_BARE_WORD: u8 = b'B';
// ufbx.c:9409 `#define UFBXI_ASCII_INT 'I'`
pub(crate) const ASCII_INT: u8 = b'I';
// ufbx.c:9410 `#define UFBXI_ASCII_FLOAT 'F'`
pub(crate) const ASCII_FLOAT: u8 = b'F';
// ufbx.c:9411 `#define UFBXI_ASCII_STRING 'S'`
pub(crate) const ASCII_STRING: u8 = b'S';

// C: `uc->data = uc->data_begin = ua->src = "";` (ufbx.c:9450) — the empty
// string literal is one readable NUL byte, and `ua->src_end = ua->src + 1`
// depends on that byte existing.
static ASCII_EMPTY_STRING: [u8; 1] = [0];

// ufbx.c:9413-9456 `ufbxi_ascii_refill`
// `allow(unused_assignments)`: C's `char *dst_buffer = NULL; size_t dst_size = 0;`
// initializers are kept verbatim even though both branches overwrite them.
#[allow(unused_assignments)]
#[inline(never)]
pub(crate) fn ascii_refill(uc: &Context) -> u8 {
    let ua: &AsciiView = uc.ascii_view();
    uc.set_data_offset(
        uc.data_offset()
            .wrapping_add(to_size(ua.src() as isize - uc.data_begin() as isize) as u64),
    );
    if uc.read_fn().is_some() {
        let mut dst_buffer: *mut u8 = core::ptr::null_mut();
        let mut dst_size: usize = 0;

        if !ua.retain_buf().is_null() {
            dst_size = uc.opts_view().read_buffer_size();
            // SAFETY: `retain_buf` is a non-null buf owned by `uc` (the array
            // reader parks its retention target there before refilling).
            dst_buffer = unsafe { push::<u8>(ua.retain_buf(), dst_size) };
            ufbxi_check_return!(uc, !dst_buffer.is_null(), b'\0', "dst_buffer");
            ua.set_src_is_retained(true);
            ua.set_src_buf(ua.retain_buf());
        } else {
            // Grow the read buffer if necessary
            if uc.read_buffer_size() < uc.opts_view().read_buffer_size() {
                let new_size: usize = uc.opts_view().read_buffer_size();
                ufbxi_check_return!(
                    uc,
                    // SAFETY: growing `uc`'s own paired `read_buffer`/
                    // `read_buffer_size` growth state through its temp
                    // allocator (uc construction invariant).
                    unsafe {
                        crate::native::allocator::grow_array::<u8>(
                            uc.ator_tmp_mut_ptr(),
                            uc.read_buffer_mut_ptr(),
                            uc.read_buffer_size_mut_ptr(),
                            new_size
                        )
                    },
                    b'\0',
                    "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->read_buffer)), (&uc->read_buffer), (&uc->read_buffer_size), (new_size))"
                );
            }
            dst_buffer = uc.read_buffer();
            dst_size = uc.read_buffer_size();
            ua.set_src_is_retained(false);
            ua.set_src_buf(core::ptr::null_mut());
        }

        // Read user data, return '\0' on EOF
        // TODO: Very unoptimal for non-full-size reads in some cases
        // SAFETY: `read_fn` is `Some` (enclosing branch) and is called with its
        // paired `read_user`; `dst_buffer` is the freshly pushed retain-buf run
        // or `uc`'s read buffer, in both cases `dst_size` writable bytes.
        let num_read: usize = unsafe {
            (uc.read_fn().unwrap_unchecked())(
                uc.read_user(),
                dst_buffer as *mut core::ffi::c_void,
                dst_size,
            )
        };
        ufbxi_check_return_msg!(
            uc,
            num_read != usize::MAX,
            b'\0',
            "IO error",
            "num_read != SIZE_MAX"
        );
        ufbxi_check_return!(uc, num_read <= dst_size, b'\0', "num_read <= dst_size");
        if num_read == 0 {
            return b'\0';
        }

        ua.set_src(dst_buffer);
        uc.set_data_begin(dst_buffer);
        uc.set_data(dst_buffer);
        // SAFETY: `num_read <= dst_size` (checked above), so the end cursor is
        // at most one past the end of `dst_buffer`, and `num_read > 0` means
        // the first byte was written by the read.
        unsafe {
            ua.set_src_end(dst_buffer.add(num_read));
            *ua.src()
        }
    } else {
        // If the user didn't specify a `read_fn()` treat anything
        // past the initial data buffer as EOF.
        ua.set_src(ASCII_EMPTY_STRING.as_ptr());
        uc.set_data_begin(ASCII_EMPTY_STRING.as_ptr());
        uc.set_data(ASCII_EMPTY_STRING.as_ptr());
        // SAFETY: `src` was just set to the one-byte `ASCII_EMPTY_STRING`
        // static, so `+ 1` is its one-past-the-end.
        ua.set_src_end(unsafe { ua.src().add(1) });
        b'\0'
    }
}

// ufbx.c:9458-9479 `ufbxi_ascii_yield`
#[inline(never)]
pub(crate) fn ascii_yield(uc: &Context) -> u8 {
    let ua: &AsciiView = uc.ascii_view();

    let ret: u8;
    if ua.src() == ua.src_end() {
        ret = ascii_refill(uc);
    } else {
        // SAFETY: `src != src_end`, so the cursor is inside the `src..src_end`
        // source window and one byte is readable there.
        ret = unsafe { *ua.src() };
    }

    if to_size(ua.src_end() as isize - ua.src() as isize) < uc.progress_interval() {
        ua.set_src_yield(ua.src_end());
    } else {
        // SAFETY: this branch has `src_end - src >= progress_interval`, so the
        // yield cursor lands at or before `src_end`, inside the source window.
        ua.set_src_yield(unsafe { ua.src().add(uc.progress_interval()) });
    }

    // TODO: Unify these properly
    uc.set_data(ua.src());
    ufbxi_check_return!(
        uc,
        report_progress(uc).is_ok(),
        b'\0',
        "ufbxi_report_progress(uc)"
    );
    ret
}

// ufbx.c:9481-9486 `ufbxi_ascii_peek`
#[inline(always)]
pub(crate) fn ascii_peek(uc: &Context) -> u8 {
    let ua: &AsciiView = uc.ascii_view();
    if ua.src() == ua.src_yield() {
        return ascii_yield(uc);
    }
    // SAFETY: `src != src_yield` and `src <= src_yield <= src_end` (the source
    // window invariant), so one byte is readable at the cursor.
    unsafe { *ua.src() }
}

// ufbx.c:9488-9495 `ufbxi_ascii_next`
#[inline(always)]
pub(crate) fn ascii_next(uc: &Context) -> u8 {
    let ua: &AsciiView = uc.ascii_view();
    if ua.src() == ua.src_yield() {
        return ascii_yield(uc);
    }
    // SAFETY: `src != src_yield` and `src <= src_yield <= src_end` (the source
    // window invariant), so the one-step advance stays within that window.
    ua.set_src(unsafe { ua.src().add(1) });
    if ua.src() == ua.src_yield() {
        return ascii_yield(uc);
    }
    // SAFETY: `src != src_yield` after the advance, so the cursor is strictly
    // before the window end and one byte is readable there.
    unsafe { *ua.src() }
}

// ufbx.c:9497-9534 `ufbxi_ascii_parse_version`
#[inline(never)]
pub(crate) fn ascii_parse_version(uc: &Context) -> u32 {
    // C: `uint8_t digits[3];` — written before read (`num_digits` gates every
    // read), zero-filled here.
    let mut digits: [u8; 3] = [0; 3];
    let mut num_digits: u32 = 0;

    let mut c: u8 = ascii_next(uc);

    let fmt: [u8; 11] = *b" FBX ?.?.?\0";
    let mut ix: u32 = 0;
    while num_digits < 3 {
        let r#ref: u8 = fmt[ix as usize];
        ix += 1;
        match r#ref {
            // Digit
            b'?' => {
                if c < b'0' || c > b'9' {
                    return 0;
                }
                digits[num_digits as usize] = c - b'0';
                num_digits += 1;
                c = ascii_next(uc);
            }

            // Whitespace
            b' ' => {
                while c == b' ' || c == b'\t' {
                    c = ascii_next(uc);
                }
            }

            // Literal character
            _ => {
                if c != r#ref {
                    return 0;
                }
                c = ascii_next(uc);
            }
        }
    }

    if num_digits != 3 {
        return 0;
    }
    1000u32 * digits[0] as u32 + 100u32 * digits[1] as u32 + 10u32 * digits[2] as u32
}

// ufbx.c:9536-9540 `ufbxi_space_mask`
pub(crate) const SPACE_MASK: u32 = (1u32 << (b' ' as u32 - 1))
    | (1u32 << (b'\t' as u32 - 1))
    | (1u32 << (b'\r' as u32 - 1))
    | (1u32 << (b'\n' as u32 - 1));

// ufbx.c:9542-9544 `ufbx_static_assert(space_codepoint, ...)`
const _: () = assert!(
    b' ' as u32 <= 32u32 && b'\t' as u32 <= 32u32 && b'\r' as u32 <= 32u32 && b'\n' as u32 <= 32u32
);

// ufbx.c:9546-9550 `ufbxi_is_space`
#[inline(always)]
pub(crate) fn is_space(c: u8) -> bool {
    let v: u32 = (c as u32).wrapping_sub(1);
    v < 32 && ((SPACE_MASK >> v) & 0x1) != 0
}

// ufbx.c:9552-9610 `ufbxi_ascii_skip_whitespace`
#[inline(never)]
pub(crate) fn ascii_skip_whitespace(uc: &Context) -> u8 {
    let ua: &AsciiView = uc.ascii_view();

    // Ignore whitespace
    let mut c: u8 = ascii_peek(uc);
    loop {
        while is_space(c) {
            c = ascii_next(uc);
        }

        // Line comment
        if c == b';' {
            let mut read_magic = false;
            // FBX ASCII files begin with a magic comment of form "; FBX 7.7.0 project file"
            // Try to extract the version number from the magic comment
            if !ua.read_first_comment() {
                ua.set_read_first_comment(true);
                let version: u32 = ascii_parse_version(uc);
                if version != 0 {
                    uc.set_version(version);
                    ua.set_found_version(true);
                    read_magic = true;
                }
            }

            c = ascii_next(uc);
            while c != b'\n' && c != b'\0' {
                c = ascii_next(uc);
            }
            c = ascii_next(uc);

            // Try to determine if this is a Blender 6100 ASCII file
            if read_magic {
                if c == b';' {
                    // C: `char line[32];` — only `line[0..line_len]` is ever
                    // read back, zero-filled here.
                    let mut line: [u8; 32] = [0; 32];
                    let mut line_len: usize = 0;

                    c = ascii_next(uc);
                    while c != b'\n' && c != b'\0' {
                        if line_len < core::mem::size_of_val(&line) {
                            line[line_len] = c;
                            line_len += 1;
                        }
                        c = ascii_next(uc);
                    }

                    // SAFETY: `line_len >= 19` bytes of the 32-byte local
                    // `line` are filled, and the literal is 19 bytes long.
                    if line_len >= 19
                        && unsafe { memcmp(line.as_ptr(), b" Created by Blender".as_ptr(), 19) }
                            == 0
                    {
                        uc.set_exporter(Exporter::BlenderAscii);
                    }
                }
            }
        } else {
            break;
        }
    }
    c
}

// ufbx.c:9612-9623 `ufbxi_ascii_push_token_char`
#[inline(always)]
pub(crate) fn ascii_push_token_char(
    uc: &Context,
    token: &AsciiTokenView,
    c: u8,
) -> Result<(), Fail> {
    // Grow the string data buffer if necessary
    if token.str_len() == token.str_cap() {
        let len: usize = max_sz(token.str_len() + 1, 256);
        ufbxi_check!(
            uc,
            // SAFETY: `grow_array` is an `unsafe fn` over a paired pointer/cap
            // slot pair; the two projections address `token`'s own
            // `str_data`/`str_cap` fields through its view (write-capable by
            // its mint), and the allocator is `uc`'s own temp allocator (uc
            // construction invariant).
            unsafe {
                crate::native::allocator::grow_array::<u8>(
                    uc.ator_tmp_mut_ptr(),
                    token.str_data_mut_ptr(),
                    token.str_cap_mut_ptr(),
                    len
                )
            },
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&token->str_data)), (&token->str_data), (&token->str_cap), (len))"
        );
    }

    // SAFETY: the guard/grow above leaves `str_len < str_cap` (the grow runs
    // only when the two are equal, and requests at least `str_len + 1`), so
    // `str_data + str_len` is a writable slot inside `token`'s string buffer.
    unsafe {
        *token.str_data().add(token.str_len()) = c;
    }
    // The increment stays within capacity by the same argument.
    token.set_str_len(token.str_len() + 1);

    Ok(())
}

// ufbx.c:9625-9637 `ufbxi_ascii_push_token_string`
///
/// # Safety
/// `data` must address a readable run of `length` bytes — an untypeable run
/// contract the caller vouches for.
#[inline(always)]
pub(crate) unsafe fn ascii_push_token_string(
    uc: &Context,
    token: &AsciiTokenView,
    data: *const u8,
    length: usize,
) -> Result<(), Fail> {
    // Grow the string data buffer if necessary
    if token.str_len() + length >= token.str_cap() {
        let len: usize = max_sz(token.str_len() + length, 256);
        ufbxi_check!(
            uc,
            // SAFETY: `grow_array` is an `unsafe fn` over a paired pointer/cap
            // slot pair; the two projections address `token`'s own
            // `str_data`/`str_cap` fields through its view (write-capable by
            // its mint), and the allocator is `uc`'s own temp allocator (uc
            // construction invariant).
            unsafe {
                crate::native::allocator::grow_array::<u8>(
                    uc.ator_tmp_mut_ptr(),
                    token.str_data_mut_ptr(),
                    token.str_cap_mut_ptr(),
                    len
                )
            },
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&token->str_data)), (&token->str_data), (&token->str_cap), (len))"
        );
    }

    // SAFETY: the guard/grow above guarantees `str_len + length <= str_cap`
    // (the grow requests exactly that many bytes, so the capacity can land on
    // it), so the `length`-byte copy lands inside `token`'s string buffer
    // starting at `str_data + str_len`; `data` is the caller's readable source
    // run of `length` bytes (this fn's own contract).
    unsafe {
        core::ptr::copy_nonoverlapping(data, token.str_data().add(token.str_len()), length);
    }
    // The advance stays within capacity by the same argument.
    token.set_str_len(token.str_len() + length);

    Ok(())
}

// ufbx.c:9639-9659 `ufbxi_ascii_skip_until`
#[inline(never)]
pub(crate) fn ascii_skip_until(uc: &Context, dst: u8) -> Result<(), Fail> {
    let ua: &AsciiView = uc.ascii_view();

    loop {
        let buffered: usize = to_size(ua.src_yield() as isize - ua.src() as isize);
        // SAFETY: `buffered` is exactly the `src..src_yield` run, which is
        // readable (source window invariant `src <= src_yield <= src_end`);
        // `memchr` returns a pointer inside it, and the `else` advance by
        // `buffered` lands on `src_yield`.
        unsafe {
            let match_: *const u8 = memchr(ua.src(), dst, buffered);
            if !match_.is_null() {
                ua.set_src(match_);
                break;
            } else {
                ua.set_src(ua.src().add(buffered));
            }
        }
        if buffered == 0 {
            let c: u8 = ascii_yield(uc);
            ufbxi_check!(uc, c != b'\0', "c != '\\0'");
        }
    }

    Ok(())
}

// ufbx.c:9660-9664 `ufbxi_ascii_span`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct AsciiSpan {
    pub source: *const u8,
    pub length: usize,
}

// ufbx.c:9666-9708 `ufbxi_ascii_store_array`
#[inline(never)]
pub(crate) unsafe fn ascii_store_array(uc: &Context, tmp_buf: &BufView) -> Result<(), Fail> {
    let ua: *mut Ascii = uc.ascii_mut_ptr();

    // The `retain_buf` field is a raw back-pointer the ascii reader stores for
    // its own pushes; recover the pointer from the view.
    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context (via `ascii_mut_ptr`);
    // this writes its `retain_buf` field in place.
    unsafe {
        (*ua).retain_buf = tmp_buf.get();
    }

    loop {
        // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; `src`/`src_yield`
        // bracket the readable source window (`src <= src_yield`).
        let buffered: usize = to_size(unsafe { (*ua).src_yield as isize - (*ua).src as isize });
        if buffered == 0 {
            let c: u8 = ascii_yield(uc);
            ufbxi_check!(uc, c != b'\0', "c != '\\0'");
            continue;
        }

        // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its `src`
        // cursor, which points inside the source window.
        let begin: *const u8 = unsafe { (*ua).src };
        let end: *const u8;
        // SAFETY: `begin` is the live `src` cursor and `buffered` is the readable
        // `src..src_yield` run, so `memchr` scans in-bounds bytes.
        let match_: *const u8 = unsafe { memchr(begin, b'}', buffered) };
        if !match_.is_null() {
            end = match_;
        } else {
            // SAFETY: `buffered` is the length of the readable `begin` run, so
            // `begin + buffered` is its one-past-the-end (== `src_yield`).
            end = unsafe { begin.add(buffered) };
        }
        // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; `end` lands in
        // `[begin, src_yield]`, still within the source window.
        unsafe {
            (*ua).src = end;
        }

        let mut length: usize = to_size(end as isize - begin as isize);
        let span: *mut AsciiSpan = uc.tmp_ascii_spans_view().push::<AsciiSpan>(1);
        ufbxi_check!(uc, !span.is_null(), "span");
        // Store the trailing '}' for parsing
        if !match_.is_null() {
            length += 1;
        }
        // SAFETY: `span` is the freshly pushed `AsciiSpan` slot checked non-null
        // above; writes its own `length` field.
        unsafe {
            (*span).length = length;
        }
        // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its
        // `src_is_retained` flag.
        if unsafe { (*ua).src_is_retained } || uc.read_fn().is_none() {
            // SAFETY: `span` is the live pushed slot; `begin` points into the
            // retained/inline source window, kept alive for the span's lifetime.
            unsafe {
                (*span).source = begin;
            }
        } else {
            // SAFETY: `span` is the live pushed slot; `begin` is readable for
            // `length` bytes (the `src` run just scanned) and `tmp_buf` is a live
            // buf, so `push_copy` copies that run into a fresh slot.
            unsafe {
                (*span).source = tmp_buf.push_copy_raw::<u8>(length, begin);
            }
            // SAFETY: `span` is the live pushed slot; reads back its `source`.
            ufbxi_check!(uc, !unsafe { (*span).source }.is_null(), "span->source");
        }

        if !match_.is_null() {
            break;
        }
    }

    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; clears its
    // `retain_buf` back-pointer.
    unsafe {
        (*ua).retain_buf = core::ptr::null_mut();
    }

    Ok(())
}

// ufbx.c:9710-9736 `ufbxi_ascii_try_ignore_string`
// C-parity: the C function returns `int` and conflates "not a string" (0) with
// the `ufbxi_check()` failure path (also 0) — the caller (ufbx.c:10374) cannot
// tell them apart and proceeds either way with `uc->error` set. Ported as
// `-> bool` with `ufbxi_check_return!(..., false)` so the error frame is still
// pushed; do NOT "fix" this into a `Result`.
// C `ufbxi_nodiscard` -> `#[must_use]`.
#[must_use]
#[inline(never)]
pub(crate) fn ascii_try_ignore_string(uc: &Context, token: &AsciiTokenView) -> bool {
    let ua: *mut Ascii = uc.ascii_mut_ptr();

    let c: u8 = ascii_skip_whitespace(uc);
    token.set_str_len(0);

    if c == b'"' {
        // Replace `prev_token` with `token` but swap the buffers so `token` uses
        // the now-unused string buffer of the old `prev_token`.
        // SAFETY: `ua` is `uc`'s own live `ascii` sub-context (via
        // `ascii_mut_ptr`); these read and rewrite its `token`/`prev_token`
        // fields in place.
        unsafe {
            let swap_data: *mut u8 = (*ua).prev_token.str_data;
            let swap_cap: usize = (*ua).prev_token.str_cap;
            (*ua).prev_token = (*ua).token;
            (*ua).token.str_data = swap_data;
            (*ua).token.str_cap = swap_cap;
        }

        token.set_type_(ASCII_STRING);
        // Skip opening quote
        ascii_next(uc);
        ufbxi_check_return!(
            uc,
            ascii_skip_until(uc, b'"').is_ok(),
            false,
            "ufbxi_ascii_skip_until(uc, '\"')"
        );
        // Skip closing quote
        ascii_next(uc);
        return true;
    }

    false
}

// ufbx.c:9738-9896 `ufbxi_ascii_next_token`
#[inline(never)]
pub(crate) fn ascii_next_token(uc: &Context, token: &AsciiTokenView) -> Result<(), Fail> {
    let ua: *mut Ascii = uc.ascii_mut_ptr();

    // Replace `prev_token` with `token` but swap the buffers so `token` uses
    // the now-unused string buffer of the old `prev_token`.
    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context (via `ascii_mut_ptr`);
    // these read and rewrite its `token`/`prev_token` fields in place.
    unsafe {
        let swap_data: *mut u8 = (*ua).prev_token.str_data;
        let swap_cap: usize = (*ua).prev_token.str_cap;
        (*ua).prev_token = (*ua).token;
        (*ua).token.str_data = swap_data;
        (*ua).token.str_cap = swap_cap;
    }

    let mut c: u8 = ascii_skip_whitespace(uc);
    token.set_str_len(0);

    if (c >= b'A' && c <= b'Z') || (c >= b'a' && c <= b'z') || c == b'_' {
        token.set_type_(ASCII_BARE_WORD);
        while (c >= b'A' && c <= b'Z')
            || (c >= b'a' && c <= b'z')
            || (c >= b'0' && c <= b'9')
            || c == b'_'
            || c == b'-'
            || c == b'('
            || c == b')'
        {
            ascii_push_token_char(uc, token, c)?;
            c = ascii_next(uc);
        }

        // Skip whitespace to find if there's a following ':'
        c = ascii_skip_whitespace(uc);
        if c == b':' {
            token.set_value_name_len(token.str_len());
            token.set_type_(ASCII_NAME);
            ascii_next(uc);
        }
    } else if (c >= b'0' && c <= b'9') || c == b'-' || c == b'+' || c == b'.' {
        token.set_type_(ASCII_INT);

        token.set_negative(c == b'-');
        while (c >= b'0' && c <= b'9')
            || c == b'-'
            || c == b'+'
            || c == b'.'
            || c == b'e'
            || c == b'E'
        {
            if c == b'.' || c == b'e' || c == b'E' {
                token.set_type_(ASCII_FLOAT);
            }
            ascii_push_token_char(uc, token, c)?;
            c = ascii_next(uc);
        }

        let mut nan_like = false;
        while (c >= b'A' && c <= b'Z')
            || (c >= b'a' && c <= b'z')
            || (c >= b'0' && c <= b'9')
            || c == b'#'
            || c == b'('
            || c == b')'
        {
            nan_like = true;
            ascii_push_token_char(uc, token, c)?;
            c = ascii_next(uc);
        }
        ascii_push_token_char(uc, token, b'\0')?;
        if nan_like {
            token.set_type_(ASCII_FLOAT);
        }

        let mut end: *const u8 = core::ptr::null();
        if token.type_() == ASCII_INT {
            // SAFETY: `token->str_data` is its NUL-terminated number buffer just
            // filled above; `parse_int64` scans it and stores its end in `end`.
            token.set_value_i64(unsafe { parse_int64(token.str_data(), &raw mut end) });
            ufbxi_check!(
                uc,
                // SAFETY: `token`'s buffer holds `str_len` bytes (the digits plus
                // the trailing NUL), so `str_data + str_len - 1` is the NUL slot
                // `parse_int64` stops at.
                end == unsafe { token.str_data().add(token.str_len() - 1) },
                "end == token->str_data + token->str_len - 1"
            );
        } else if token.type_() == ASCII_FLOAT {
            let mut flags: u32 = uc.double_parse_flags();
            // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its
            // `parse_as_f32` flag.
            if unsafe { (*ua).parse_as_f32 } {
                flags = PARSE_DOUBLE_AS_BINARY32;
            }
            // SAFETY: `token->str_data`/`str_len` describe its NUL-terminated
            // number buffer; `parse_double` scans it and stores its end in `end`.
            token.set_value_f64(unsafe {
                parse_double(token.str_data(), token.str_len(), &raw mut end, flags)
            });
            ufbxi_check!(
                uc,
                // SAFETY: as in the int branch — `str_data + str_len - 1` is the
                // trailing-NUL slot within `token`'s buffer.
                end == unsafe { token.str_data().add(token.str_len() - 1) },
                "end == token->str_data + token->str_len - 1"
            );
        }
    } else if c == b'"' {
        token.set_type_(ASCII_STRING);
        c = ascii_next(uc);
        while c != b'"' {
            // Optimized string parsing for non-special characters
            // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; the
            // `src`/`src_yield` reads are of its live cursors. When the peek/next
            // that produced `c` left a readable byte, `src < src_yield <= src_end`
            // holds, so `src + 1` is at most the source window's one-past-the-end;
            // on the exhausted-source path the cursors are equal and the
            // comparison rejects the fast run before any byte is read.
            if unsafe { (*ua).src.add(1) < (*ua).src_yield } {
                // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its
                // `src`/`src_yield` cursors delimiting the readable window.
                let begin: *const u8 = unsafe { (*ua).src };
                let mut end: *const u8 = unsafe { (*ua).src_yield };
                // SAFETY: `[begin, end)` is a sub-run of the readable source
                // window, so `memchr` scans in-bounds bytes.
                let quot: *const u8 =
                    unsafe { memchr(begin, b'"', to_size(end as isize - begin as isize)) };
                if !quot.is_null() {
                    end = quot;
                }
                // SAFETY: `end` only shrank toward `begin`, so `[begin, end)`
                // remains a readable sub-run for `memchr` to scan.
                let esc: *const u8 =
                    unsafe { memchr(begin, b'&', to_size(end as isize - begin as isize)) };
                if !esc.is_null() {
                    end = esc;
                }

                if begin < end {
                    // SAFETY: `[begin, end)` is a readable run of the source
                    // window, so `begin` addresses that many readable bytes.
                    unsafe {
                        ascii_push_token_string(
                            uc,
                            token,
                            begin,
                            to_size(end as isize - begin as isize),
                        )
                    }?;
                    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; `end`
                    // lands inside the readable window, a valid `src` cursor.
                    unsafe {
                        (*ua).src = end;
                    }
                    c = ascii_peek(uc);
                    continue;
                }
            }

            // Escape XML-like elements, funny enough there is no way to escape '&' itself, there is no `&amp`.
            // '&quot;' -> '"'
            // '&cr;' -> '\r'
            // '&lf;' -> '\n'
            if c == b'&' {
                let entity: *const u8;
                let replacement: u8;

                c = ascii_next(uc);
                match c {
                    b'q' => {
                        entity = b"&quot;\0".as_ptr();
                        replacement = b'"';
                    }
                    b'c' => {
                        entity = b"&cr;\0".as_ptr();
                        replacement = b'\r';
                    }
                    b'l' => {
                        entity = b"&lf;\0".as_ptr();
                        replacement = b'\n';
                    }
                    _ => {
                        // As '&' is not escaped in any way just map '&' -> '&'
                        entity = b"&\0".as_ptr();
                        replacement = b'&';
                    }
                }

                let mut step: usize = 1;

                // SAFETY: `entity` addresses a static NUL-terminated entity
                // literal (assigned in every match arm), so its first byte is
                // readable.
                ufbxi_dev_assert!(!entity.is_null() && unsafe { *entity } != 0);
                // `entity` is a NULL terminated string longer than a single character
                // cppcheck-suppress arrayIndexOutOfBounds
                // SAFETY: `entity` addresses a static NUL-terminated entity
                // literal; `step` never passes its terminating NUL (the loop stops
                // at the first `0`), so `entity + step` is a readable byte.
                while unsafe { *entity.add(step) } != 0 {
                    // SAFETY: as above — `step` still indexes a byte before the NUL.
                    if c != unsafe { *entity.add(step) } {
                        break;
                    }
                    c = ascii_next(uc);
                    step += 1;
                }

                // SAFETY: `entity` addresses a static NUL-terminated entity
                // literal; the loop left `step` at or before its terminating NUL,
                // so `entity + step` is readable.
                if unsafe { *entity.add(step) } == b'\0' {
                    // Full match: Push the replacement character
                    ascii_push_token_char(uc, token, replacement)?;
                } else {
                    // Partial match: Push the prefix we have skipped already
                    let mut i: usize = 0;
                    while i < step {
                        // SAFETY: `i < step` indexes a matched prefix byte of
                        // the entity literal, before its terminating NUL.
                        ascii_push_token_char(uc, token, unsafe { *entity.add(i) })?;
                        i += 1;
                    }
                }
                continue;
            }

            ufbxi_check!(uc, c != b'\0', "c != '\\0'");
            ascii_push_token_char(uc, token, c)?;
            c = ascii_next(uc);
        }
        // Skip closing quote
        let next: u8 = ascii_next(uc);

        // Check if the next character is ':', in some legacy FBX files we have names with
        // spaces, like `"Transport Tool Settings": { ... }`
        if next == b':' {
            token.set_value_name_len(token.str_len());
            token.set_type_(ASCII_NAME);
            ascii_next(uc);
        }
    } else {
        // Single character token
        token.set_type_(c);
        ascii_next(uc);
    }

    Ok(())
}

// ufbx.c:9898-9908 `ufbxi_ascii_accept`
// C-parity: same `int`-conflation as `ufbxi_ascii_try_ignore_string` — the
// `ufbxi_check()` failure path and "token type did not match" both return 0,
// and callers use the result both as a boolean (ufbx.c:10402) and via
// `ufbxi_check()` (ufbx.c:10308). Ported as `-> bool`.
// C `ufbxi_nodiscard` -> `#[must_use]`.
#[must_use]
pub(crate) fn ascii_accept(uc: &Context, type_: u8) -> bool {
    let ua: &AsciiView = uc.ascii_view();

    if ua.token_view().type_() == type_ {
        ufbxi_check_return!(
            uc,
            ascii_next_token(uc, ua.token_view()).is_ok(),
            false,
            "ufbxi_ascii_next_token(uc, &ua->token)"
        );
        true
    } else {
        false
    }
}

// ufbx.c:9910-9964 `ufbxi_ascii_read_int_array`
#[inline(never)]
pub(crate) fn ascii_read_int_array(
    uc: &Context,
    type_: u8,
    p_num_read: &mut usize,
) -> Result<(), Fail> {
    let ua: *mut Ascii = uc.ascii_mut_ptr();
    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context (via `ascii_mut_ptr`);
    // reads its `parse_as_f32` flag.
    if unsafe { (*ua).parse_as_f32 } {
        return Ok(());
    }
    let initial_items: usize = uc.tmp_stack_view().num_items();

    let mut val: i64;
    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its current
    // token's `type_`, and its `value.i64_` union member holds the parsed int
    // whenever the type is `ASCII_INT`.
    if unsafe { (*ua).token.type_ } == ASCII_INT {
        val = unsafe { (*ua).token.value.i64_ };
    } else {
        return Ok(());
    }

    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its `src` cursor
    // and `src_yield` window end.
    let mut src: *const u8 = unsafe { (*ua).src };
    let end: *const u8 = unsafe { (*ua).src_yield };
    let mut src_scan: *const u8 = src;

    loop {
        // Skip '\s*,\s*' between array elements. If we don't find a comma after an element
        // don't push it as we can't be 100% certain whether it's a part of the array.
        // SAFETY: the `src_scan != end` guard short-circuits before the deref, so
        // `src_scan` addresses a byte inside `[src, src_yield)`; the advance then
        // lands at or before `end`.
        while src_scan != end && unsafe { is_space(*src_scan) } {
            src_scan = unsafe { src_scan.add(1) };
        }
        // SAFETY: as above — `src_scan != end` guards the deref.
        if src_scan == end || unsafe { *src_scan } != b',' {
            break;
        }
        // SAFETY: reached only when `src_scan != end`, so `+1` lands at or before
        // `end` within the source window.
        src_scan = unsafe { src_scan.add(1) };
        // SAFETY: as the first scan — guarded deref, advance stays within window.
        while src_scan != end && unsafe { is_space(*src_scan) } {
            src_scan = unsafe { src_scan.add(1) };
        }

        // Found comma, commit to the position and push the previous value to the array
        src = src_scan;
        if type_ == b'i' {
            let v: *mut i32 = uc.tmp_stack_view().push_fast::<i32>(1);
            ufbxi_check!(uc, !v.is_null(), "v");
            // SAFETY: `v` is the freshly pushed, non-null `i32` slot.
            unsafe {
                *v = val as i32;
            }
        } else if type_ == b'l' {
            let v: *mut i64 = uc.tmp_stack_view().push_fast::<i64>(1);
            ufbxi_check!(uc, !v.is_null(), "v");
            // SAFETY: `v` is the freshly pushed, non-null `i64` slot.
            unsafe {
                *v = val;
            }
        }

        // Try to parse the next value, we don't commit this until we find a comma after it above.
        let left: usize = to_size(end as isize - src_scan as isize);
        if left < 32 {
            break;
        }

        // SAFETY: `left >= 32` bytes are readable at `src_scan` inside the source
        // window; `parse_int64` scans them and stores its end back into `src_scan`.
        val = unsafe { parse_int64(src_scan, &raw mut src_scan) };
        if src_scan.is_null() {
            break;
        }
    }

    // Resume conventional parsing if we moved `src`.
    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its `src` cursor.
    if src != unsafe { (*ua).src } {
        // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; `src` was derived
        // from its window and stays inside it, a valid cursor to store back.
        unsafe {
            (*ua).src = src;
        }
        // `uc`'s own `ascii.token` is retokenized in place.
        ascii_next_token(uc, uc.ascii_view().token_view())?;
    }

    *p_num_read = uc.tmp_stack_view().num_items() - initial_items;
    Ok(())
}

// ufbx.c:9966-9973 `ufbxi_ascii_array_task`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct AsciiArrayTask {
    pub arr_data: *mut core::ffi::c_void,
    pub arr_type: u8,
    pub arr_size: usize,
    pub spans: *const AsciiSpan,
    pub num_spans: usize,
    pub offset: usize,
}

impl View<AsciiArrayTask> {
    pub(crate) fn arr_data(&self) -> *mut c_void {
        view_read!(self, arr_data)
    }

    pub(crate) fn arr_type(&self) -> u8 {
        view_read!(self, arr_type)
    }

    pub(crate) fn arr_size(&self) -> usize {
        view_read!(self, arr_size)
    }

    pub(crate) fn spans(&self) -> *const AsciiSpan {
        view_read!(self, spans)
    }

    pub(crate) fn num_spans(&self) -> usize {
        view_read!(self, num_spans)
    }

    pub(crate) fn offset(&self) -> usize {
        view_read!(self, offset)
    }

    pub(crate) fn set_offset(&self, value: usize) {
        view_write!(self, offset, value);
    }
}

// ufbx.c:9975-10008 `ufbxi_ascii_array_task_parse_floats`
///
/// Parses comma-separated floats out of `src` into the task's destination
/// array, returning the offset within `src` of the byte after the last
/// committed comma, or `None` for C's `NULL` (destination array full).
///
/// # Safety
/// `t`'s `arr_data` / `arr_size` / `arr_type` fields must describe a live
/// destination run: `arr_data` is the base of an array of at least `arr_size`
/// elements of the type `arr_type` names (`'f'` -> `f32`, `'d'` -> `f64`), of
/// which the first `offset` are already written. That `(pointer, count)` run is
/// a caller vouch the view type cannot carry.
#[inline(never)]
pub(crate) unsafe fn ascii_array_task_parse_floats(
    t: &View<AsciiArrayTask, Mut>,
    src: &[u8],
    parse_flags: u32,
) -> Option<usize> {
    let mut src_ix: usize = 0;
    let mut offset: usize = t.offset();
    // When the task type is `'f'`/`'d'`, `arr_data` is the base of the matching
    // destination array and `offset` is the count already written, so `add_ptr`
    // addresses its next slot.
    let (mut dst_float, mut dst_double): (*mut f32, *mut f64) = (
        if t.arr_type() == b'f' {
            add_ptr(t.arr_data() as *mut f32, offset)
        } else {
            core::ptr::null_mut()
        },
        if t.arr_type() == b'd' {
            add_ptr(t.arr_data() as *mut f64, offset)
        } else {
            core::ptr::null_mut()
        },
    );
    ufbx_assert!(!dst_float.is_null() || !dst_double.is_null());
    let mut src_begin: usize = src_ix;

    while src_ix != src.len() {
        // Both callers guarantee the run's final byte `src[src.len() - 1]` is a
        // `','` (imp's buffer appends one when it leaves the value state; its
        // span path cuts `parse_end` just past one), a non-space that halts this
        // scan strictly before the run end.
        while is_space(src[src_ix]) {
            src_ix += 1;
        }

        // Try to parse the next value, we don't commit this until we find a comma after it above.
        let mut num_end: *const u8 = core::ptr::null();
        // SAFETY: `src[src_ix..]` is a readable run of `src.len() - src_ix`
        // bytes; `parse_double` scans at most that many and stores its end into
        // `num_end`, which therefore lands inside `src`.
        let val: f64 = unsafe {
            parse_double(
                src[src_ix..].as_ptr(),
                src.len() - src_ix,
                &raw mut num_end,
                parse_flags,
            )
        };
        if num_end.is_null() {
            return Some(src_begin);
        }
        src_ix = to_size(num_end as isize - src.as_ptr() as isize);

        // As the first scan — the run's trailing `','` halts it strictly before
        // the run end.
        while is_space(src[src_ix]) {
            src_ix += 1;
        }
        if src[src_ix] != b',' {
            break;
        }
        src_ix += 1;
        src_begin = src_ix;

        if offset >= t.arr_size() {
            return None;
        }
        if !dst_double.is_null() {
            // SAFETY: `offset < arr_size` (checked above), so `dst_double` (base +
            // offset) is a live slot of the `f64` destination array; advance stays
            // within it.
            unsafe {
                *dst_double = val;
                dst_double = dst_double.add(1);
            }
        } else {
            // SAFETY: `offset < arr_size`, so `dst_float` is a live slot of the
            // `f32` destination array; advance stays within it.
            unsafe {
                *dst_float = val as f32;
                dst_float = dst_float.add(1);
            }
        }
        offset += 1;
    }

    t.set_offset(offset);
    Some(src_begin)
}

// ufbx.c:10010-10040 `ufbxi_ascii_array_task_parse_ints`
///
/// Parses comma-separated integers out of `src` into the task's destination
/// array, returning the offset within `src` of the byte after the last
/// committed comma, or `None` for C's `NULL` (destination array full).
///
/// # Safety
/// `t`'s `arr_data` / `arr_size` / `arr_type` fields must describe a live
/// destination run: `arr_data` is the base of an array of at least `arr_size`
/// elements of the type `arr_type` names (`'i'` -> `i32`, `'l'` -> `i64`), of
/// which the first `offset` are already written. That `(pointer, count)` run is
/// a caller vouch the view type cannot carry.
#[inline(never)]
pub(crate) unsafe fn ascii_array_task_parse_ints(
    t: &View<AsciiArrayTask, Mut>,
    src: &[u8],
) -> Option<usize> {
    let mut src_ix: usize = 0;
    let mut offset: usize = t.offset();
    // When the task type is `'i'`/`'l'`, `arr_data` is the base of the matching
    // destination array and `offset` is the count already written, so `add_ptr`
    // addresses its next slot.
    let (mut dst32, mut dst64): (*mut i32, *mut i64) = (
        if t.arr_type() == b'i' {
            add_ptr(t.arr_data() as *mut i32, offset)
        } else {
            core::ptr::null_mut()
        },
        if t.arr_type() == b'l' {
            add_ptr(t.arr_data() as *mut i64, offset)
        } else {
            core::ptr::null_mut()
        },
    );
    ufbx_assert!(!dst32.is_null() || !dst64.is_null());
    let mut src_begin: usize = src_ix;

    while src_ix != src.len() {
        // Both callers guarantee the run's final byte `src[src.len() - 1]` is a
        // `','` (imp's buffer appends one when it leaves the value state; its
        // span path cuts `parse_end` just past one), a non-space non-digit that
        // halts this scan and `parse_int64` strictly before the run end.
        while is_space(src[src_ix]) {
            src_ix += 1;
        }

        let mut num_end: *const u8 = core::ptr::null();
        // SAFETY: `src[src_ix..]` is a readable run halted by the trailing
        // `','`; `parse_int64` scans it and stores its end into `num_end`, which
        // therefore lands inside `src`.
        let val: i64 = unsafe { parse_int64(src[src_ix..].as_ptr(), &raw mut num_end) };
        if num_end.is_null() {
            return None;
        }
        src_ix = to_size(num_end as isize - src.as_ptr() as isize);

        // As the first scan — the run's trailing `','` halts it strictly before
        // the run end.
        while is_space(src[src_ix]) {
            src_ix += 1;
        }
        if src[src_ix] != b',' {
            break;
        }
        src_ix += 1;
        src_begin = src_ix;

        if offset >= t.arr_size() {
            return None;
        }
        if !dst32.is_null() {
            // SAFETY: `offset < arr_size` (checked above), so `dst32` (base +
            // offset) is a live slot of the `i32` destination array; advance stays
            // within it.
            unsafe {
                *dst32 = val as i32;
                dst32 = dst32.add(1);
            }
        } else {
            // SAFETY: `offset < arr_size`, so `dst64` is a live slot of the `i64`
            // destination array; advance stays within it.
            unsafe {
                *dst64 = val;
                dst64 = dst64.add(1);
            }
        }
        offset += 1;
    }

    t.set_offset(offset);
    Some(src_begin)
}

// ufbx.c:10042-10050 `ufbxi_ascii_array_task_parse`
///
/// Dispatches to the float or int worker by the task's `arr_type`, returning
/// the offset within `src` of the byte after the last committed comma, or
/// `None` for C's `NULL`.
///
/// # Safety
/// `t`'s `arr_data` / `arr_size` / `arr_type` fields must describe a live
/// destination run, exactly as the workers this dispatches to document: that
/// `(pointer, count)` run is a caller vouch the view type cannot carry.
#[inline(never)]
pub(crate) unsafe fn ascii_array_task_parse(
    t: &View<AsciiArrayTask, Mut>,
    src: &[u8],
) -> Option<usize> {
    if t.arr_type() == b'f' || t.arr_type() == b'd' {
        let flags: u32 = parse_double_init_flags();
        // SAFETY: `t` carries the destination-run vouch the float worker
        // documents, forwarded from this function's own contract.
        unsafe { ascii_array_task_parse_floats(t, src, flags) }
    } else {
        // SAFETY: `t` carries the destination-run vouch the int worker
        // documents, forwarded from this function's own contract.
        unsafe { ascii_array_task_parse_ints(t, src) }
    }
}

// ufbx.c:10052-10057 `ufbxi_ascii_scan_state`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AsciiScanState {
    Value = 0,
    Whitespace = 1,
    Comment = 2,
    Comma = 3,
}

// ufbx.c:10059-10148 `ufbxi_ascii_array_task_imp`
///
/// Scans the task's spans, parsing their comma-separated values into the task's
/// destination array; returns C's `false` on a parse error or a destination the
/// spans do not fill exactly.
///
/// # Safety
/// `t`'s `spans` / `num_spans` fields must describe a live span array, each of
/// whose `source` / `length` pairs is a readable byte run, and its `arr_data` /
/// `arr_size` / `arr_type` fields must describe a live destination run exactly
/// as the parse helper this forwards to documents. Both are `(pointer, count)`
/// runs the view type cannot carry.
#[inline(never)]
pub(crate) unsafe fn ascii_array_task_imp(t: &View<AsciiArrayTask, Mut>) -> bool {
    // Temporary buffer for parsing between spans
    // C: uninitialized; only `buffer[0..buffer_len]` is ever read back (and the
    // parse helpers stop at the trailing ',' the state machine writes), so the
    // zero-fill here is inert.
    let mut buffer: [u8; 128] = [0; 128]; // ufbxi_uninit
    let mut buffer_len: usize = 0;
    let mut buffer_value: bool = false;

    let mut state: AsciiScanState = AsciiScanState::Whitespace;
    // C: `ufbxi_for(const ufbxi_ascii_span, span, t->spans, t->num_spans)`
    // `spans`/`num_spans` describe the task's span array (fn contract), so the
    // base and the one-past-the-end pointer are in bounds of that array.
    let mut span: *const AsciiSpan = t.spans();
    let span_end: *const AsciiSpan = add_ptr(t.spans() as *mut AsciiSpan, t.num_spans());
    while span != span_end {
        // SAFETY: `span` walks `[spans, span_end)`, so it addresses a live
        // `AsciiSpan`; `source`/`length` describe its readable byte run, so
        // `source + length` is that run's one-past-the-end.
        let mut src: *const u8 = unsafe { (*span).source };
        let end: *const u8 = unsafe { src.add((*span).length) };

        while src != end {
            // State machine for skipping whitespace and comments, potentially
            // between multiple spans.
            while src != end {
                // SAFETY: the enclosing `src != end` guard means `src` addresses a
                // byte inside the span's `[source, source+length)` run.
                let c: u8 = unsafe { *src };
                if state == AsciiScanState::Value {
                    if buffer_len >= core::mem::size_of_val(&buffer) - 1 {
                        return false;
                    }
                    if c == b'"' {
                        return false;
                    } else if c == b';' || is_space(c) {
                        state = AsciiScanState::Whitespace;
                        buffer[buffer_len] = b' ';
                        buffer_len += 1;
                    } else if c == b',' || c == b'}' {
                        state = AsciiScanState::Comma;
                        buffer[buffer_len] = b',';
                        buffer_len += 1;
                        // SAFETY: reached with `src != end`, so `+1` lands at or
                        // before the span run end.
                        src = unsafe { src.add(1) };
                        break;
                    } else {
                        buffer_value = true;
                        buffer[buffer_len] = c;
                        buffer_len += 1;
                        // SAFETY: reached with `src != end`, so `+1` stays in run.
                        src = unsafe { src.add(1) };
                    }
                } else if state == AsciiScanState::Whitespace {
                    if c == b';' {
                        state = AsciiScanState::Comment;
                    } else if is_space(c) {
                        // SAFETY: reached with `src != end`, so `+1` stays in run.
                        src = unsafe { src.add(1) };
                    } else {
                        state = AsciiScanState::Value;
                    }
                } else if state == AsciiScanState::Comment {
                    if c == b'\n' {
                        state = AsciiScanState::Whitespace;
                    } else {
                        // SAFETY: reached with `src != end`, so `+1` stays in run.
                        src = unsafe { src.add(1) };
                    }
                } else if state == AsciiScanState::Comma {
                    state = AsciiScanState::Whitespace;
                }
            }

            if state == AsciiScanState::Comma {
                // Parse a value from the buffer
                if buffer_value {
                    // SAFETY: `t` carries the destination-run vouch the parse
                    // helper documents, forwarded from this function's own
                    // contract; `buffer[..buffer_len]` is the local `buffer`'s
                    // written prefix (`buffer_len <= 127`), a readable parse
                    // run.
                    let buffer_end: Option<usize> =
                        unsafe { ascii_array_task_parse(t, &buffer[..buffer_len]) };
                    if buffer_end.is_none() || buffer_end == Some(0) {
                        return false;
                    }
                }

                // If not at end, we are past the last comma, so try to find a
                // safe range to parse.
                if src != end {
                    let mut parse_end: *const u8 = end;
                    while parse_end > src {
                        // SAFETY: `parse_end > src >= source`, so `parse_end - 1`
                        // addresses a byte inside the span run.
                        if unsafe { *parse_end.sub(1) } == b',' {
                            break;
                        }
                        // SAFETY: as above — `parse_end - 1` stays at or after
                        // `src` within the run.
                        parse_end = unsafe { parse_end.sub(1) };
                    }
                    if src < parse_end {
                        // SAFETY: `t` carries the destination-run vouch the
                        // parse helper documents, forwarded from this
                        // function's own contract; `[src, parse_end)` is a
                        // sub-run of the span's readable bytes, minted once as
                        // the helper's slice.
                        let src_begin: Option<usize> = unsafe {
                            ascii_array_task_parse(
                                t,
                                core::slice::from_raw_parts(
                                    src,
                                    to_size(parse_end as isize - src as isize),
                                ),
                            )
                        };
                        match src_begin {
                            // SAFETY: `src_begin` is an offset at or before
                            // `parse_end - src`, so it lands inside the span's
                            // run, derived from `src` to keep its provenance.
                            Some(offset) => src = unsafe { src.add(offset) },
                            None => return false,
                        }
                    }
                }

                buffer_len = 0;
                buffer_value = false;
            }
        }

        // SAFETY: `span` is in `[spans, span_end)`, so `+1` lands at or before
        // the one-past-the-end `span_end`.
        span = unsafe { span.add(1) };
    }

    if t.offset() != t.arr_size() {
        return false;
    }

    true
}

// ufbx.c:10150-10158 `ufbxi_ascii_array_task_fn`
// `ufbxi_task_fn` entry point handed to the thread pool by
// `ufbxi_ascii_parse_node` (ufbx.c:10653-10659).
#[inline(never)]
pub(crate) unsafe extern "C" fn ascii_array_task_fn(task: *mut Task) -> bool {
    // SAFETY: `task` points to a valid, live `Task` (thread-pool contract); its
    // `data` field is the `AsciiArrayTask` this entry point was created with.
    let t: *mut AsciiArrayTask = unsafe { (*task).data } as *mut AsciiArrayTask;
    // SAFETY: `t` is the task's own live, write-capable `AsciiArrayTask`
    // payload, so it mints the task view, and its span/destination fields carry
    // the runs `ascii_array_task_imp` documents.
    if !unsafe { ascii_array_task_imp(View::<AsciiArrayTask, Mut>::from_ptr(t)) } {
        // SAFETY: `task` points to a valid, live `Task`; writes its `error` field
        // to a static string literal.
        unsafe {
            (*task).error = b"Threaded ASCII parse error\0".as_ptr();
        }
        return false;
    }
    true
}

// ufbx.c:10160-10222 `ufbxi_ascii_read_float_array`
#[inline(never)]
pub(crate) fn ascii_read_float_array(
    uc: &Context,
    type_: u8,
    p_num_read: &mut usize,
) -> Result<(), Fail> {
    let ua: *mut Ascii = uc.ascii_mut_ptr();
    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context (via `ascii_mut_ptr`);
    // reads its `parse_as_f32` flag.
    if unsafe { (*ua).parse_as_f32 } {
        return Ok(());
    }

    let mut val: f64;
    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its current
    // token. For an `ASCII_FLOAT` token the `value.f64_` union member is live;
    // for `ASCII_INT` the `value.i64_` member and `negative` flag are.
    if unsafe { (*ua).token.type_ } == ASCII_FLOAT {
        val = unsafe { (*ua).token.value.f64_ };
    } else if unsafe { (*ua).token.type_ } == ASCII_INT {
        let fsign: f64 = if unsafe { (*ua).token.value.i64_ == 0 && (*ua).token.negative } {
            -1.0
        } else {
            1.0
        };
        val = unsafe { (*ua).token.value.i64_ } as f64 * fsign;
    } else {
        return Ok(());
    }

    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its `src` cursor
    // and `src_yield` window end.
    let mut src: *const u8 = unsafe { (*ua).src };
    let end: *const u8 = unsafe { (*ua).src_yield };

    let parse_flags: u32 = uc.double_parse_flags();

    let initial_items: usize = uc.tmp_stack_view().num_items();
    let mut src_scan: *const u8 = src;
    loop {
        // Skip '\s*,\s*' between array elements. If we don't find a comma after an element
        // don't push it as we can't be 100% certain whether it's a part of the array.
        // SAFETY: the `src_scan != end` guard short-circuits before the deref, so
        // `src_scan` addresses a byte inside `[src, src_yield)`; the advance then
        // lands at or before `end`.
        while src_scan != end && unsafe { is_space(*src_scan) } {
            src_scan = unsafe { src_scan.add(1) };
        }
        // SAFETY: as above — `src_scan != end` guards the deref.
        if src_scan == end || unsafe { *src_scan } != b',' {
            break;
        }
        // SAFETY: reached only when `src_scan != end`, so `+1` lands at or before
        // `end` within the source window.
        src_scan = unsafe { src_scan.add(1) };
        // SAFETY: as the first scan — guarded deref, advance stays within window.
        while src_scan != end && unsafe { is_space(*src_scan) } {
            src_scan = unsafe { src_scan.add(1) };
        }

        // Found comma, commit to the position and push the previous value to the array
        src = src_scan;
        if type_ == b'd' {
            let v: *mut f64 = uc.tmp_stack_view().push_fast::<f64>(1);
            ufbxi_check!(uc, !v.is_null(), "v");
            // SAFETY: `v` is the freshly pushed, non-null `f64` slot.
            unsafe {
                *v = val;
            }
        } else if type_ == b'f' {
            let v: *mut f32 = uc.tmp_stack_view().push_fast::<f32>(1);
            ufbxi_check!(uc, !v.is_null(), "v");
            // SAFETY: `v` is the freshly pushed, non-null `f32` slot.
            unsafe {
                *v = val as f32;
            }
        }

        // Try to parse the next value, we don't commit this until we find a comma after it above.
        let mut num_end: *const u8 = core::ptr::null();
        let left: usize = to_size(end as isize - src_scan as isize);
        // SAFETY: `left` is the readable `src_scan..src_yield` run; `parse_double`
        // scans at most that many bytes and stores its end into `num_end`.
        val = unsafe { parse_double(src_scan, left, &raw mut num_end, parse_flags) };
        if num_end.is_null() || num_end == src_scan || num_end >= end {
            break;
        }

        src_scan = num_end;
    }

    // Resume conventional parsing if we moved `src`.
    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its `src` cursor.
    if src != unsafe { (*ua).src } {
        // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; `src` stays inside
        // its window, a valid cursor to store back.
        unsafe {
            (*ua).src = src;
        }
        // `uc`'s own `ascii.token` is retokenized in place.
        ascii_next_token(uc, uc.ascii_view().token_view())?;
    }

    *p_num_read = uc.tmp_stack_view().num_items() - initial_items;
    Ok(())
}

// ufbx.c:10224-10239 `ufbxi_setup_base64`
// C `ufbxi_nounroll` is an optimizer pragma with no Rust analogue; the loops
// port as plain `while`s.
#[inline(never)]
pub(crate) fn setup_base64(uc: &Context) -> Result<(), Fail> {
    let table: *mut u8 = uc.tmp_view().push::<u8>(256);
    ufbxi_check!(uc, !table.is_null(), "table");
    uc.set_base64_table(table);

    // SAFETY: `table` is the freshly pushed 256-byte run checked non-null
    // above; every index written below is a `u8` value, hence `< 256`.
    unsafe {
        core::ptr::write_bytes(table, 0x80, 256);
        let mut c: u8 = b'A';
        while c <= b'Z' {
            *table.add(c as usize) = (c as i32 - b'A' as i32) as u8;
            c += 1;
        }
        let mut c: u8 = b'a';
        while c <= b'z' {
            *table.add(c as usize) = (26 + (c as i32 - b'a' as i32)) as u8;
            c += 1;
        }
        let mut c: u8 = b'0';
        while c <= b'9' {
            *table.add(c as usize) = (52 + (c as i32 - b'0' as i32)) as u8;
            c += 1;
        }
        *table.add(b'+' as usize) = 62;
        *table.add(b'/' as usize) = 63;
        *table.add(b'=' as usize) = 0x40;
    }

    Ok(())
}

// ufbx.c:10241-10282 `ufbxi_decode_base64`
///
/// # Safety
/// `src` must be readable for `src_length` bytes, unmoved and unwritten for the
/// call (a raw run: the pointer and its length are separate parameters, so no
/// type carries the relation). `p_result`'s `data` must address a writable
/// output buffer of at least `src_length / 4 * 3 + 3` bytes — the capacity the
/// caller sizes and pushes (ufbx.c:10412-10414); the decode writes 3 bytes per
/// 4 input bytes.
#[inline(never)]
pub(crate) unsafe fn decode_base64(
    uc: &Context,
    p_result: &StringView,
    src: *const u8,
    src_length: usize,
    p_failed: &mut bool,
) -> Result<(), Fail> {
    if uc.base64_table().is_null() {
        setup_base64(uc)?;
    }

    let table: *mut u8 = uc.base64_table();
    let mut error_mask: u32 = 0;
    let mut pad_error: u32 = 0;

    // `p_result`'s `data` field is the freshly pushed output buffer sized for
    // the decode (fn contract).
    let mut p: *mut u8 = p_result.data() as *mut u8;
    let mut i: usize = 0;
    while i + 4 <= src_length {
        // SAFETY: `i + 4 <= src_length`, so `src + i + 0..3` are readable source
        // bytes; each byte value is `< 256`, in bounds for the 256-entry `table`.
        let (a, b, c, d) = unsafe {
            (
                *table.add(*src.add(i + 0) as usize) as u32,
                *table.add(*src.add(i + 1) as usize) as u32,
                *table.add(*src.add(i + 2) as usize) as u32,
                *table.add(*src.add(i + 3) as usize) as u32,
            )
        };
        pad_error = error_mask;
        error_mask |= a | b | c | d;

        // SAFETY: the output buffer holds 3 bytes per 4 input bytes; this
        // iteration writes 3 and advances `p` by 3, staying within its capacity.
        unsafe {
            *p.add(0) = (a << 2 | b >> 4) as u8;
            *p.add(1) = (b << 4 | c >> 2) as u8;
            *p.add(2) = (c << 6 | d) as u8;
            p = p.add(3);
        }

        i += 4;
    }

    if src_length >= 4 {
        // SAFETY: `src_length >= 4`, so `src + src_length - 4` addresses the last
        // 4-byte input group, all readable.
        let end: *const u8 = unsafe { src.add(src_length - 4) };
        let mut padding: u32 = 0;
        // SAFETY: `end` is the last input group's base, so `end + 0..3` are the
        // four readable trailing bytes.
        unsafe {
            padding |= if *end.add(0) == b'=' { 0x8 } else { 0x0 };
            padding |= if *end.add(1) == b'=' { 0x4 } else { 0x0 };
            padding |= if *end.add(2) == b'=' { 0x2 } else { 0x0 };
            padding |= if *end.add(3) == b'=' { 0x1 } else { 0x0 };
        }
        if padding <= 0x1 {
            p = sub_ptr(p, padding as usize); // "xxx=" or "xxxx"
        } else if padding == 0x3 {
            p = sub_ptr(p, 2); // "xx=="
        } else {
            pad_error |= 0x40; // anything else
        }
    }

    if ((error_mask & 0x80) != 0 || (pad_error & 0x40) != 0 || src_length % 4 != 0) && !*p_failed {
        ufbxi_check!(
            uc,
            ufbxi_warnf!(
                uc,
                WarningType::BadBase64Content,
                "Ignored bad base64 embedded content"
            )
            .is_ok(),
            "ufbxi_warnf(UFBX_WARNING_BAD_BASE64_CONTENT, \"Ignored bad base64 embedded content\")"
        );
        *p_failed = true;
    }

    // `p` advanced from `p_result->data` within the output buffer, so their
    // difference is the decoded byte count, stored back into its `length`.
    p_result.set_length(to_size(p as isize - p_result.data() as isize));
    Ok(())
}

// Recursion limited by check at the start
// ufbx.c:10285-10694 `ufbxi_ascii_parse_node`
// `ufbxi_recursive_function(int, ufbxi_ascii_parse_node, ..., UFBXI_MAX_NODE_DEPTH + 1, ...)`
// (ufbx.c:10286-10287): under regression, a thread-local depth guard wraps the
// recursive body (which C splits into `ufbxi_ascii_parse_node_rec`); otherwise
// the macro is empty and the wrapper is a plain call.
#[inline(never)]
pub(crate) fn ascii_parse_node(
    uc: &Context,
    depth: u32,
    parent_state: ParseState,
    p_end: &mut bool,
    tmp_buf: &BufView,
    recursive: bool,
) -> Result<(), Fail> {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|d| {
            ufbx_assert!(d.get() < MAX_NODE_DEPTH + 1);
            d.set(d.get() + 1);
        });
        let ret = ascii_parse_node_rec(uc, depth, parent_state, p_end, tmp_buf, recursive);
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        ret
    }
    #[cfg(not(feature = "regression"))]
    {
        ascii_parse_node_rec(uc, depth, parent_state, p_end, tmp_buf, recursive)
    }
}

// `allow(unused_assignments)`: C's `void *arr_data = NULL;` initializer is kept
// verbatim even though every branch overwrites it.
#[allow(unused_assignments)]
#[inline(never)]
fn ascii_parse_node_rec(
    uc: &Context,
    depth: u32,
    parent_state: ParseState,
    p_end: &mut bool,
    tmp_buf: &BufView,
    recursive: bool,
) -> Result<(), Fail> {
    let ua: *mut Ascii = uc.ascii_mut_ptr();

    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context (via `ascii_mut_ptr`);
    // reads its current token's `type_`.
    if unsafe { (*ua).token.type_ } == b'}' {
        // `uc`'s own `ascii.token` is retokenized in place.
        ascii_next_token(uc, uc.ascii_view().token_view())?;
        *p_end = true;
        return Ok(());
    }

    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its token type.
    if unsafe { (*ua).token.type_ } == ASCII_END {
        ufbxi_check_msg!(uc, depth == 0, "Truncated file");
        *p_end = true;
        return Ok(());
    }

    // Parse the name eg. "Node:" token and intern the name
    ufbxi_check!(uc, depth < MAX_NODE_DEPTH, "depth < UFBXI_MAX_NODE_DEPTH");
    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its token type.
    if !uc.sure_fbx() && depth == 0 && unsafe { (*ua).token.type_ } != ASCII_NAME {
        ufbxi_fail_msg!(uc, "Expected a 'Name:' token", "Not an FBX file");
    }
    ufbxi_check!(
        uc,
        ascii_accept(uc, ASCII_NAME),
        "ufbxi_ascii_accept(uc, UFBXI_ASCII_NAME)"
    );
    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; the accepted name token
    // moved to `prev_token`, whose `value.name_len` union member is live.
    let name_len: usize = unsafe { (*ua).prev_token.value.name_len };
    ufbxi_check!(uc, name_len <= 0xff, "name_len <= 0xff");
    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; `prev_token.str_data`
    // holds `str_len` readable name bytes, interned via `uc`'s own string pool.
    let name: *const u8 = unsafe {
        push_string(
            uc.string_pool_mut_ptr(),
            (*ua).prev_token.str_data,
            (*ua).prev_token.str_len,
            core::ptr::null_mut(),
            true,
        )
    };
    ufbxi_check!(uc, !name.is_null(), "name");

    // Push the parsed node into the `tmp_stack` buffer, the nodes will be popped by
    // calling code after its done parsing all of it's children.
    let node: *mut Node = uc.tmp_stack_view().push_zero::<Node>(1);
    ufbxi_check!(uc, !node.is_null(), "node");
    // SAFETY: `node` is the freshly pushed, non-null `Node` slot; writes its own
    // `name`/`name_len` fields.
    unsafe {
        (*node).name = name;
        (*node).name_len = name_len as u8;
    }

    let mut in_ascii_array: bool = false;

    let mut num_values: u32 = 0;
    let mut type_mask: u32 = 0;

    let mut arr_type: u8 = 0;
    let mut arr_buf: *mut Buf = core::ptr::null_mut();
    let mut arr_elem_size: usize = 0;
    let mut arr_error: bool = false;

    // Check if the values of the node we're parsing currently should be
    // treated as an array.
    let mut arr_info = core::mem::MaybeUninit::<ArrayInfo>::uninit();
    // SAFETY: this frame's own `ArrayInfo` storage, borrowed exclusively for the
    // rest of the body — nothing else names it; `is_array_node` writes `flags`
    // before reading it and writes `type_` on every path returning `true`, so
    // the reads below never touch uninitialized bytes.
    let arr_info: &mut ArrayInfo = unsafe { &mut *arr_info.as_mut_ptr() };
    // SAFETY: `name` is the non-null pooled node name checked above, addressing
    // its `name_len` interned bytes; `is_array_node` compares the run by
    // interned pointer identity.
    let name_run: &[u8] = unsafe { slice_from_ptr(name, name_len) };
    if is_array_node(uc, parent_state, name_run, arr_info) {
        let flags: u8 = arr_info.flags;
        arr_type = normalize_array_type(arr_info.type_, b'b');
        arr_buf = tmp_buf.get();
        if (flags & ARRAY_FLAG_RESULT) != 0 {
            arr_buf = uc.result_mut_ptr();
        } else if (flags & ARRAY_FLAG_TMP_BUF) != 0 {
            arr_buf = uc.tmp_mut_ptr();
        }

        let arr: *mut ValueArray = tmp_buf.push::<ValueArray>(1);
        ufbxi_check!(uc, !arr.is_null(), "arr");
        // SAFETY: `node` is the live pushed `Node` slot and `arr` the live pushed
        // `ValueArray`; writes `node`'s value-type/array fields and `arr`'s type.
        unsafe {
            (*node).value_type_mask = ValueType::Array as u16;
            (*node).content.array = arr;
            (*arr).type_ = arr_type;
        }

        // Parse array values using strtof() if the array destination is 32-bit float
        // since KeyAttrDataFloat packs integer data (!) into floating point values so we
        // should try to be as exact as possible.
        if (arr_info.flags & ARRAY_FLAG_ACCURATE_F32) != 0 {
            // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; sets its flag.
            unsafe {
                (*ua).parse_as_f32 = true;
            }
        }

        arr_elem_size = array_type_size(arr_type);

        if arr_type != b'-' {
            // Force alignment for array contents: This allows us to use `ufbxi_push_fast()`
            // in fast parsing functions.
            ufbxi_check!(
                uc,
                // SAFETY: `tmp_stack_mut_ptr` is `uc`'s own live temp-stack buf.
                !unsafe { push_size_zero(uc.tmp_stack_mut_ptr(), 8, 1) }.is_null(),
                "ufbxi_push_size_zero(&uc->tmp_stack, 8, 1)"
            );

            // Pad with 4 zero elements to make indexing with `-1` safe.
            if (flags & ARRAY_FLAG_PAD_BEGIN) != 0 {
                ufbxi_check!(
                    uc,
                    // SAFETY: `tmp_stack_mut_ptr` is `uc`'s own live temp-stack buf.
                    !unsafe { push_size_zero(uc.tmp_stack_mut_ptr(), arr_elem_size, 4) }.is_null(),
                    "ufbxi_push_size_zero(&uc->tmp_stack, arr_elem_size, 4)"
                );
                num_values += 4;
            }
        }
    }

    // Some fields in ASCII may have leading commas eg. `Content: , "base64-string"`
    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its token type.
    if unsafe { (*ua).token.type_ } == b',' {
        // HACK: If we are parsing an "array" that should be ignored, ie. `Content` when
        // `opts.ignore_embedded == true` try to skip the next token string if possible.
        if arr_type == b'-' {
            if !ascii_try_ignore_string(uc, uc.ascii_view().token_view()) {
                ascii_next_token(uc, uc.ascii_view().token_view())?;
            }
        } else {
            ascii_next_token(uc, uc.ascii_view().token_view())?;
        }
    }

    // SAFETY: `node` is the live pushed `Node` slot; reads its interned `name`
    // pointer, which `update_parse_state` matches against known node names.
    let parse_state: ParseState = unsafe { update_parse_state(parent_state, (*node).name) };
    // C: `ufbxi_value vals[UFBXI_MAX_NON_ARRAY_VALUES];` — left uninitialized;
    // only the `[0, num_values)` prefix written in the loop below is ever read
    // (by the `ufbxi_push_copy` at the end).
    let mut vals = core::mem::MaybeUninit::<[Value; MAX_NON_ARRAY_VALUES]>::uninit();
    let vals: *mut Value = vals.as_mut_ptr() as *mut Value;

    let mut deferred_size: u32 = 0;

    // NOTE: Infinite loop to allow skipping the comma parsing via `continue`.
    loop {
        // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; the `&raw mut`
        // projects the address of its own `prev_token` field without a reference.
        let tok: *mut AsciiToken = unsafe { &raw mut (*ua).prev_token };

        if arr_type != 0 {
            let mut num_read: usize = 0;
            if arr_type == b'f' || arr_type == b'd' {
                ascii_read_float_array(uc, arr_type, &mut num_read)?;
            } else if arr_type == b'i' || arr_type == b'l' {
                ascii_read_int_array(uc, arr_type, &mut num_read)?;
            }
            ufbxi_check!(
                uc,
                (u32::MAX - num_values) as usize > num_read,
                "UINT32_MAX - num_values > num_read"
            );
            num_values = num_values.wrapping_add(num_read as u32);
        }

        if ascii_accept(uc, ASCII_STRING) {
            if arr_type != 0 {
                if arr_type == b's' || arr_type == b'S' || arr_type == b'C' {
                    let raw: bool = arr_type == b's';
                    let v: *mut String = uc.tmp_stack_view().push::<String>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    if arr_type == b'C' {
                        let buf: &BufView = if uc.opts_view().retain_dom() {
                            uc.result_view()
                        } else {
                            tmp_buf
                        };
                        // SAFETY: `tok` addresses `ua`'s live `prev_token`; reads
                        // its `str_len`.
                        let capacity: usize = unsafe { (*tok).str_len } / 4 * 3 + 3;
                        // SAFETY: `v` is the freshly pushed, non-null `String` slot;
                        // writes its `data` to the freshly pushed output buffer.
                        unsafe {
                            (*v).data = buf.push::<u8>(capacity);
                        }
                        // SAFETY: `v` is the live pushed slot; reads back its `data`.
                        ufbxi_check!(uc, !unsafe { (*v).data }.is_null(), "v->data");
                        // SAFETY: `v` is the live pushed `String` slot (the base64
                        // out-param), write-capable arena provenance from the tmp
                        // stack and unmoved for the call, and its `data` addresses
                        // the freshly pushed `capacity`-byte output buffer; `tok`'s
                        // `str_data`/`str_len` are its readable base64 bytes.
                        unsafe {
                            decode_base64(
                                uc,
                                StringView::from_ptr(v),
                                (*tok).str_data,
                                (*tok).str_len,
                                &mut arr_error,
                            )?;
                        }
                        // SAFETY: `v` is the live pushed slot; reads back its length.
                        ufbx_assert!(unsafe { (*v).length } <= capacity);
                    } else {
                        // SAFETY: `v` is the freshly pushed, non-null `String` slot;
                        // writes its `data`/`length` from `tok`'s live `prev_token`
                        // string, then interns it through `uc`'s own string pool.
                        unsafe {
                            (*v).data = (*tok).str_data;
                            (*v).length = (*tok).str_len;
                            push_string_place_str(uc.string_pool_mut_ptr(), v, raw)?;
                        }
                    }
                } else {
                    // Ignore strings in non-string arrays, decrement `num_values` as it will be
                    // incremented after the loop iteration is done to ignore it.
                    num_values = num_values.wrapping_sub(1);
                }
            } else if (num_values as usize) < MAX_NON_ARRAY_VALUES {
                type_mask |= (ValueType::String as u32) << (num_values * 2);
                // SAFETY: `num_values < MAX_NON_ARRAY_VALUES` (branch guard), so
                // `vals + num_values` is a live slot of the local `vals` array.
                let v: *mut Value = unsafe { vals.add(num_values as usize) };

                // SAFETY: `tok` addresses `ua`'s live `prev_token`; reads its
                // `str_data`/`str_len`.
                let str_: *const u8 = unsafe { (*tok).str_data };
                let length: usize = unsafe { (*tok).str_len };
                ufbxi_check!(uc, !str_.is_null(), "str");

                if length == 0 {
                    // SAFETY: `v` is a live `vals` slot; writes its `s` string
                    // union member to the shared empty-string sentinel.
                    unsafe {
                        (*v).s.raw_data = EMPTY_CHAR.as_ptr();
                        (*v).s.raw_length = 0;
                        (*v).s.utf8_length = 0;
                    }
                } else {
                    let mut non_ascii: bool = false;
                    // SAFETY: `str_` is readable for `length` bytes (the token
                    // string); `non_ascii` is a local out-param.
                    let hash: u32 =
                        unsafe { hash_string_check_ascii(str_, length, &raw mut non_ascii) };
                    // SAFETY: `name` is the interned node name; `is_raw_string`
                    // matches it against known raw-string node fields.
                    let raw: bool = !non_ascii
                        || unsafe { is_raw_string(uc, parent_state, name, num_values as usize) };
                    push_sanitized_string(
                        uc.string_pool_view(),
                        // SAFETY: `v` is a live `vals` slot; `&raw mut (*v).s`
                        // projects its own `s` field — write-capable arena memory
                        // outliving the call — as the sanitize out-param.
                        unsafe { SanitizedStringView::from_ptr(&raw mut (*v).s) },
                        // SAFETY: `str_` is readable for `length` bytes (the token
                        // string) and unwritten for the borrow — in particular it
                        // is not the pool's own temp buffer, which the callee
                        // writes.
                        unsafe { slice_from_ptr(str_, length) },
                        hash,
                        raw,
                    )?;
                    if non_ascii && raw {
                        // SAFETY: `v` is a live `vals` slot; writes its `s.utf8_length`.
                        unsafe {
                            (*v).s.utf8_length = u32::MAX;
                        }
                    }
                }
            }
        } else if ascii_accept(uc, ASCII_INT) {
            // SAFETY: `tok` addresses `ua`'s live `prev_token`; the accepted int
            // token's `value.i64_` union member and `negative` flag are live.
            let val: i64 = unsafe { (*tok).value.i64_ };
            let fsign: Real = if val == 0 && unsafe { (*tok).negative } {
                -1.0f32 as Real
            } else {
                1.0f32 as Real
            };

            match arr_type {
                0 => {
                    // Parse version from comment if there was no magic comment
                    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads
                    // its `found_version` flag.
                    if !unsafe { (*ua).found_version }
                        && parse_state == ParseState::FbxVersion
                        && num_values == 0
                    {
                        if val >= 6000 && val <= 10000 {
                            // SAFETY: `ua` is `uc`'s own live `ascii` sub-context;
                            // sets its `found_version` flag.
                            unsafe {
                                (*ua).found_version = true;
                            }
                            uc.set_version(val as u32);
                        }
                    }

                    if (num_values as usize) < MAX_NON_ARRAY_VALUES {
                        type_mask |= (ValueType::Number as u32) << (num_values * 2);
                        // SAFETY: `num_values < MAX_NON_ARRAY_VALUES`, so
                        // `vals + num_values` is a live slot of the local array.
                        let v: *mut Value = unsafe { vals.add(num_values as usize) };
                        // False positive: `v->f` and `v->i` do not overlap in the union.
                        // cppcheck-suppress overlappingWriteUnion
                        // C: `v->f = (double)(v->i = val) * (double)fsign;`
                        // SAFETY: `v` is a live `vals` slot; writes its `num` union.
                        unsafe {
                            (*v).num.i = val;
                            (*v).num.f = (*v).num.i as f64 * fsign as f64;
                        }
                    }
                }

                b'b' => {
                    let v: *mut bool = uc.tmp_stack_view().push::<bool>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `bool` slot.
                    unsafe {
                        *v = val != 0;
                    }
                }
                b'c' => {
                    let v: *mut u8 = uc.tmp_stack_view().push::<u8>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `u8` slot.
                    unsafe {
                        *v = val as u8;
                    }
                }
                b'i' => {
                    let v: *mut i32 = uc.tmp_stack_view().push::<i32>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `i32` slot.
                    unsafe {
                        *v = val as i32;
                    }
                }
                b'l' => {
                    let v: *mut i64 = uc.tmp_stack_view().push::<i64>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `i64` slot.
                    unsafe {
                        *v = val;
                    }
                }
                b'f' => {
                    let v: *mut f32 = uc.tmp_stack_view().push::<f32>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `f32` slot.
                    unsafe {
                        *v = val as f32 * fsign as f32;
                    }
                }
                b'd' => {
                    let v: *mut f64 = uc.tmp_stack_view().push::<f64>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `f64` slot.
                    unsafe {
                        *v = val as f64 * fsign as f64;
                    }
                }
                b'-' => {
                    num_values = num_values.wrapping_sub(1);
                }

                _ => ufbxi_fail!(uc, "Bad array dst type"),
            }
        } else if ascii_accept(uc, ASCII_FLOAT) {
            // SAFETY: `tok` addresses `ua`'s live `prev_token`; the accepted float
            // token's `value.f64_` union member is live.
            let val: f64 = unsafe { (*tok).value.f64_ };

            match arr_type {
                0 => {
                    if (num_values as usize) < MAX_NON_ARRAY_VALUES {
                        type_mask |= (ValueType::Number as u32) << (num_values * 2);
                        // SAFETY: `num_values < MAX_NON_ARRAY_VALUES`, so
                        // `vals + num_values` is a live slot of the local array.
                        let v: *mut Value = unsafe { vals.add(num_values as usize) };
                        // False positive: `v->f` and `v->i` do not overlap in the union.
                        // cppcheck-suppress overlappingWriteUnion
                        // C: `v->i = ufbxi_f64_to_i64(v->f = val);`
                        // SAFETY: `v` is a live `vals` slot; writes its `num` union.
                        unsafe {
                            (*v).num.f = val;
                            (*v).num.i = f64_to_i64((*v).num.f);
                        }
                    }
                }

                b'b' => {
                    let v: *mut bool = uc.tmp_stack_view().push::<bool>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `bool` slot.
                    unsafe {
                        *v = val != 0.0;
                    }
                }
                b'c' => {
                    let v: *mut u8 = uc.tmp_stack_view().push::<u8>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // C-parity: C's `(uint8_t)val` on a `double` is UB out of range; the
                    // x86-64 oracle emits a 32-bit `cvttsd2si` + low-byte narrow. Plain
                    // `as` (saturating) per the PORTING.md bare-float-cast row — known,
                    // accepted divergence, same choice as the `ufbxi_cast_u8` appliers in
                    // parse_binary.rs.
                    // SAFETY: `v` is the freshly pushed, non-null `u8` slot.
                    unsafe {
                        *v = val as u8;
                    }
                }
                b'i' => {
                    let v: *mut i32 = uc.tmp_stack_view().push::<i32>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `i32` slot.
                    unsafe {
                        *v = f64_to_i32(val);
                    }
                }
                b'l' => {
                    let v: *mut i64 = uc.tmp_stack_view().push::<i64>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `i64` slot.
                    unsafe {
                        *v = f64_to_i64(val);
                    }
                }
                b'f' => {
                    let v: *mut f32 = uc.tmp_stack_view().push::<f32>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `f32` slot.
                    unsafe {
                        *v = val as f32;
                    }
                }
                b'd' => {
                    let v: *mut f64 = uc.tmp_stack_view().push::<f64>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `f64` slot.
                    unsafe {
                        *v = val;
                    }
                }
                b'-' => {
                    num_values = num_values.wrapping_sub(1);
                }

                _ => ufbxi_fail!(uc, "Bad array dst type"),
            }
        } else if ascii_accept(uc, ASCII_BARE_WORD) {
            let mut val: i64 = 0;
            let mut val_f: f64 = 0.0;
            // SAFETY: `tok` addresses `ua`'s live `prev_token`; reads its `str_len`.
            if unsafe { (*tok).str_len } >= 1 {
                // C-parity: `tok->str_data` is `char *` and the oracle targets make
                // C `char` signed, so a byte >= 0x80 converts to a NEGATIVE `int64_t`
                // here (PORTING.md "`char` (value, where signedness is observable)").
                // SAFETY: `str_len >= 1`, so `tok->str_data`'s first byte is readable
                // and reinterpreted as a signed `i8`.
                val = unsafe { *((*tok).str_data as *const i8) } as i64;
                val_f = val as f64;
                // SAFETY: `tok` addresses `ua`'s live `prev_token`; reads its `str_len`.
                if unsafe { (*tok).str_len } > 1 && unsafe { (*tok).str_len } < 64 {
                    // Try to parse the bare word as NAN/INF
                    let mut str_data = core::mem::MaybeUninit::<[u8; 64]>::uninit(); // ufbxi_uninit
                    let str_data: *mut u8 = str_data.as_mut_ptr() as *mut u8;
                    // SAFETY: `tok` addresses `ua`'s live `prev_token`; reads its `str_len`.
                    let str_len: usize = unsafe { (*tok).str_len };
                    // SAFETY: `str_len < 64`, so copying `str_len` bytes from
                    // `tok->str_data` into the 64-byte local `str_data` fits, and
                    // writing the NUL at index `str_len` (< 64) stays in bounds.
                    unsafe {
                        core::ptr::copy_nonoverlapping((*tok).str_data, str_data, str_len);
                        *str_data.add(str_len) = b'\0';
                    }
                    let mut inf_nan: f64 = 0.0;
                    let mut end: *const u8 = core::ptr::null();
                    // SAFETY: `str_data` holds `str_len` bytes plus a trailing NUL;
                    // `parse_inf_nan` scans it and stores its end into `end`, and
                    // `str_data + str_len` is the NUL slot within the local buffer.
                    if unsafe { parse_inf_nan(&raw mut inf_nan, str_data, str_len, &raw mut end) }
                        && end == unsafe { str_data.add(str_len) }
                    {
                        val = 0;
                        val_f = inf_nan;
                    }
                }
            }

            match arr_type {
                0 => {
                    if (num_values as usize) < MAX_NON_ARRAY_VALUES {
                        type_mask |= (ValueType::Number as u32) << (num_values * 2);
                        // SAFETY: `num_values < MAX_NON_ARRAY_VALUES`, so
                        // `vals + num_values` is a live slot of the local array.
                        let v: *mut Value = unsafe { vals.add(num_values as usize) };
                        // False positive: `v->f` and `v->i` do not overlap in the union.
                        // cppcheck-suppress overlappingWriteUnion
                        // SAFETY: `v` is a live `vals` slot; writes its `num` union.
                        unsafe {
                            (*v).num.i = val;
                            (*v).num.f = val_f;
                        }
                    }
                }

                b'b' => {
                    let v: *mut bool = uc.tmp_stack_view().push::<bool>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `bool` slot.
                    unsafe {
                        *v = val != 0;
                    }
                }
                b'c' => {
                    let v: *mut u8 = uc.tmp_stack_view().push::<u8>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `u8` slot.
                    unsafe {
                        *v = val as u8;
                    }
                }
                b'i' => {
                    let v: *mut i32 = uc.tmp_stack_view().push::<i32>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `i32` slot.
                    unsafe {
                        *v = val as i32;
                    }
                }
                b'l' => {
                    let v: *mut i64 = uc.tmp_stack_view().push::<i64>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `i64` slot.
                    unsafe {
                        *v = val;
                    }
                }
                b'f' => {
                    let v: *mut f32 = uc.tmp_stack_view().push::<f32>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `f32` slot.
                    unsafe {
                        *v = val_f as f32;
                    }
                }
                b'd' => {
                    let v: *mut f64 = uc.tmp_stack_view().push::<f64>(1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // SAFETY: `v` is the freshly pushed, non-null `f64` slot.
                    unsafe {
                        *v = val_f;
                    }
                }
                b'-' => {
                    num_values = num_values.wrapping_sub(1);
                }

                _ => ufbxi_fail!(uc, "Bad array dst type"),
            }
        } else if ascii_accept(uc, b'*') {
            // Parse a post-7000 ASCII array eg. "*3 { 1,2,3 }"
            ufbxi_check!(uc, !in_ascii_array, "!in_ascii_array");
            ufbxi_check!(
                uc,
                ascii_accept(uc, ASCII_INT),
                "ufbxi_ascii_accept(uc, UFBXI_ASCII_INT)"
            );
            // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; the accepted int
            // count moved to `prev_token`, whose `value.i64_` union member is live.
            let count: i64 = unsafe { (*ua).prev_token.value.i64_ };

            if ascii_accept(uc, b'{') {
                ufbxi_check!(
                    uc,
                    ascii_accept(uc, ASCII_NAME),
                    "ufbxi_ascii_accept(uc, UFBXI_ASCII_NAME)"
                );
                in_ascii_array = true;

                // Optimized array skipping and threaded parsing
                if arr_type == b'-' {
                    ascii_skip_until(uc, b'}')?;
                } else if uc.parse_threaded()
                    && !uc.opts_view().force_single_thread_ascii_parsing()
                    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; reads its
                    // `parse_as_f32` flag.
                    && !unsafe { (*ua).parse_as_f32 }
                    && (arr_type == b'i'
                        || arr_type == b'l'
                        || arr_type == b'f'
                        || arr_type == b'd')
                {
                    // Don't bother with small arrays due to fixed overhead
                    if count >= MIN_THREADED_ASCII_VALUES as i64 && count <= u32::MAX as i64 {
                        deferred_size = (count as u32).wrapping_sub(1);
                        // SAFETY: `tmp_buf` is the caller's live scratch buf where
                        // the array's source spans are parked.
                        unsafe {
                            ascii_store_array(uc, tmp_buf)?;
                        }
                    }
                }
            }

            // NOTE: This `continue` skips incrementing `num_values` and parsing
            // a comma, continuing to parse the values in the array.
            continue;
        } else {
            break;
        }

        // Add value and keep parsing if there's a comma. This part may be
        // skipped if we enter an array block.
        num_values = num_values.wrapping_add(1);
        ufbxi_check!(uc, num_values < u32::MAX, "num_values < UINT32_MAX");
        if !ascii_accept(uc, b',') {
            break;
        }
    }

    // Close the ASCII array if we are in one
    if in_ascii_array {
        ufbxi_check!(uc, ascii_accept(uc, b'}'), "ufbxi_ascii_accept(uc, '}')");
    }

    // SAFETY: `ua` is `uc`'s own live `ascii` sub-context; clears its flag.
    unsafe {
        (*ua).parse_as_f32 = false;
    }

    if arr_type != 0 {
        if arr_type == b'-' {
            // SAFETY: this node was tagged an array above, so `node`'s live
            // `content.array` union member points at the pushed `ValueArray`;
            // writes its `data`/`size` fields to the empty state.
            unsafe {
                (*(*node).content.array).data = core::ptr::null_mut();
                (*(*node).content.array).size = 0;
            }
        } else {
            let mut arr_data: *mut c_void = core::ptr::null_mut();

            if deferred_size > 0 {
                // SAFETY: `arr_buf` is a live buf (tmp/result/tmp_stack) chosen
                // above; reserves `num_values + deferred_size` elements of
                // `arr_elem_size` bytes.
                arr_data = unsafe {
                    push_size(
                        arr_buf,
                        arr_elem_size,
                        num_values.wrapping_add(deferred_size) as usize,
                    )
                };
                // Pop any previously pushed values
                if num_values > 0 {
                    // SAFETY: `tmp_stack_mut_ptr` is `uc`'s own live temp stack; it
                    // holds `num_values` pushed elements, popped into the front of
                    // the just-reserved `arr_data` run.
                    unsafe {
                        pop_size(
                            uc.tmp_stack_mut_ptr(),
                            arr_elem_size,
                            num_values as usize,
                            arr_data,
                            false,
                        );
                    }
                }
            } else if arr_error {
                // SAFETY: `tmp_stack_mut_ptr` is `uc`'s own live temp stack holding
                // `num_values` pushed elements, discarded here (null destination).
                unsafe {
                    pop_size(
                        uc.tmp_stack_mut_ptr(),
                        arr_elem_size,
                        num_values as usize,
                        core::ptr::null_mut(),
                        false,
                    );
                }
                num_values = 0;
                arr_data = ZERO_SIZE_BUFFER.as_ptr() as *mut c_void;
            } else {
                // SAFETY: `arr_buf` and `uc`'s temp stack are live bufs; moves the
                // `num_values` pushed elements from the stack into `arr_buf`.
                arr_data = unsafe {
                    push_pop_size(
                        arr_buf,
                        uc.tmp_stack_mut_ptr(),
                        arr_elem_size,
                        num_values as usize,
                    )
                };
            }
            ufbxi_check!(uc, !arr_data.is_null(), "arr_data");
            if (arr_info.flags & ARRAY_FLAG_PAD_BEGIN) != 0 {
                // SAFETY: `node`'s live `content.array` points at the pushed
                // `ValueArray`; `arr_data` has the 4-element pad prefix, so skipping
                // `4 * arr_elem_size` bytes yields the real data start, and the size
                // drops those 4 pad elements.
                unsafe {
                    (*(*node).content.array).data = (arr_data as *mut u8)
                        .add(4usize.wrapping_mul(arr_elem_size))
                        as *mut c_void;
                    (*(*node).content.array).size =
                        num_values.wrapping_add(deferred_size).wrapping_sub(4) as usize;
                }
            } else {
                // SAFETY: `node`'s live `content.array` points at the pushed
                // `ValueArray`; writes its `data`/`size` for the un-padded case.
                unsafe {
                    (*(*node).content.array).data = arr_data;
                    (*(*node).content.array).size = num_values.wrapping_add(deferred_size) as usize;
                }
            }

            // Pop alignment helper
            // SAFETY: `tmp_stack_mut_ptr` is `uc`'s own live temp stack; pops the
            // single 8-byte alignment element pushed at array setup.
            unsafe {
                pop_size(uc.tmp_stack_mut_ptr(), 8, 1, core::ptr::null_mut(), false);
            }

            // Deferred parsing
            if deferred_size > 0 {
                let num_spans: usize = uc.tmp_ascii_spans_view().num_items();
                let spans: *mut AsciiSpan = tmp_buf.push_pop(uc.tmp_ascii_spans_view(), num_spans);
                ufbxi_check!(uc, !spans.is_null(), "spans");

                let mut t = core::mem::MaybeUninit::<AsciiArrayTask>::uninit(); // ufbxi_uninit
                let t: *mut AsciiArrayTask = t.as_mut_ptr();
                // SAFETY: `t` is the local `MaybeUninit<AsciiArrayTask>`'s address;
                // all its fields are written here. `arr_data` (base of the reserved
                // run) advanced by `num_values * arr_elem_size` bytes points at the
                // deferred region the worker fills.
                unsafe {
                    (*t).arr_data = (arr_data as *mut u8)
                        .add((num_values as usize).wrapping_mul(arr_elem_size))
                        as *mut c_void;
                    (*t).arr_type = arr_type;
                    (*t).arr_size = deferred_size as usize;
                    (*t).num_spans = num_spans;
                    (*t).spans = spans;
                    (*t).offset = 0;
                }

                // TODO: Split these further
                // SAFETY: `thread_pool_mut_ptr` is `uc`'s own live thread pool.
                let task: *mut Task = unsafe {
                    thread_pool_create_task(uc.thread_pool_mut_ptr(), ascii_array_task_fn)
                };
                if !task.is_null() {
                    // SAFETY: `task` is the live created task; `t` is the fully
                    // initialized local task, copied into `tmp_buf` (a live buf) and
                    // stored as the task's `data`.
                    unsafe {
                        (*task).data = tmp_buf.push_copy_raw::<AsciiArrayTask>(1, t) as *mut c_void;
                    }
                    // SAFETY: `task` is the live created task; reads back its `data`.
                    ufbxi_check!(uc, !unsafe { (*task).data }.is_null(), "task->data");
                    // SAFETY: `thread_pool_mut_ptr` is `uc`'s own live thread pool;
                    // `task` is the populated task to enqueue.
                    unsafe {
                        thread_pool_run_task(uc.thread_pool_mut_ptr(), task);
                    }
                } else {
                    ufbxi_check_msg!(
                        uc,
                        // SAFETY: `t` is the fully initialized local task, so
                        // it mints the task view, and its span/destination
                        // fields carry the runs `ascii_array_task_imp`
                        // documents; run inline.
                        unsafe { ascii_array_task_imp(View::<AsciiArrayTask, Mut>::from_ptr(t)) },
                        "Threaded ASCII parse error",
                        "ufbxi_ascii_array_task_imp(&t)"
                    );
                }
            }
        }
    } else {
        num_values = min32(num_values, MAX_NON_ARRAY_VALUES as u32);
        // SAFETY: `node` is the live pushed `Node` slot; writes its value-type mask
        // and copies the `num_values` written `vals` entries into `tmp_buf` (a live
        // buf) as its `content.vals`.
        unsafe {
            (*node).value_type_mask = type_mask as u16;
            (*node).content.vals = tmp_buf.push_copy_raw::<Value>(num_values as usize, vals);
        }
        // SAFETY: `node` is the live pushed slot; reads back its `content.vals`.
        ufbxi_check!(uc, !unsafe { (*node).content.vals }.is_null(), "node->vals");
    }

    // Recursively parse the children of this node. Update the parse state
    // to provide context for child node parsing.
    if ascii_accept(uc, b'{') {
        if recursive {
            let mut num_children: usize = 0;
            loop {
                let mut end: bool = false;
                ascii_parse_node(uc, depth + 1, parse_state, &mut end, tmp_buf, recursive)?;
                if end {
                    break;
                }
                num_children += 1;
            }

            // Pop children from `tmp_stack` to a contiguous array
            // SAFETY: `node` is the live pushed `Node` slot; moves its `num_children`
            // parsed children from the temp stack into `tmp_buf` as its `children`.
            unsafe {
                (*node).children = tmp_buf.push_pop::<Node>(uc.tmp_stack_view(), num_children);
            }
            // SAFETY: `node` is the live pushed slot; reads back its `children`.
            ufbxi_check!(uc, !unsafe { (*node).children }.is_null(), "node->children");
            // SAFETY: `node` is the live pushed slot; writes its `num_children`.
            unsafe {
                (*node).num_children = num_children as u32;
            }
        }

        uc.set_has_next_child(true);
    } else {
        uc.set_has_next_child(false);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // `ufbxi_is_space` (ufbx.c:9546-9550) is context-free: exactly
    // ' ', '\t', '\r', '\n' and nothing else, including the c == 0 wraparound.
    #[test]
    fn test_is_space() {
        for i in 0..=255u32 {
            let c = i as u8;
            let expect = c == b' ' || c == b'\t' || c == b'\r' || c == b'\n';
            assert_eq!(is_space(c), expect, "byte {c:#04x}");
        }
        assert!(!is_space(0));
    }
}
