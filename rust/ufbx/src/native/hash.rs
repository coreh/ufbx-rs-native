//! Port of the `// -- Hash functions` banner section, ufbx.c:4702-4821.
//! The `// -- Hash map` machinery (ufbxi_map struct, grow/find/insert, the
//! `ufbxi_map_cmp_*` comparators at ufbx.c:4661-4700) is a later unit; only
//! the `ufbxi_ptr_id` key type is declared here minimally because
//! `ufbxi_hash_ptr_id` takes it by value.
//!
//! Phase 1: not all items have consumers yet.
#![allow(dead_code, unused_macros, unused_imports)]

use crate::native::platform::{read_u32, ufbx_assert};

// ufbx.c:4688-4691 `ufbxi_ptr_id` — key type owned by the hash-map unit
// (comparator `ufbxi_map_cmp_ptr_id`, ufbx.c:4693-4700, is ported with it);
// declared here because `ufbxi_hash_ptr_id` takes it by value.
// TODO(hashmap unit): move this definition into the hash-map module when it is
// ported — do NOT define a second copy there (ground rule 3: one #[repr(C)]
// definition per C struct, in the owning module).
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PtrId {
    pub ptr: usize,
    pub id: u64,
}

// ufbx.c:4704-4730 `ufbxi_hash_string`
#[inline(never)]
pub(crate) unsafe fn hash_string(mut str_: *const u8, mut length: usize) -> u32 {
    let mut hash = length as u32;
    let seed = 0x9e3779b9u32;
    if length >= 4 {
        loop {
            let word = read_u32(str_);
            hash = ((hash << 5 | hash >> 27) ^ word).wrapping_mul(seed);
            str_ = str_.add(4);
            length -= 4;
            if !(length >= 4) {
                break;
            }
        }

        let word = read_u32(str_.add(length).sub(4));
        hash = ((hash << 5 | hash >> 27) ^ word).wrapping_mul(seed);
    } else {
        let mut word = 0u32;
        if length >= 1 {
            word |= (*str_.add(0) as u32) << 0;
        }
        if length >= 2 {
            word |= (*str_.add(1) as u32) << 8;
        }
        if length >= 3 {
            word |= (*str_.add(2) as u32) << 16;
        }
        hash = ((hash << 5 | hash >> 27) ^ word).wrapping_mul(seed);
    }
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb352d);
    hash ^= hash >> 15;
    hash
}

// ufbx.c:4732-4779 `ufbxi_hash_string_check_ascii`
// NOTE: _Must_ match `ufbxi_hash_string()`
#[inline(never)]
pub(crate) unsafe fn hash_string_check_ascii(
    mut str_: *const u8,
    mut length: usize,
    p_non_ascii: *mut bool,
) -> u32 {
    let mut ascii_mask = 0u32;
    let mut zero_mask = 0u32;

    ufbx_assert!(length > 0);

    let mut hash = length as u32;
    let seed = 0x9e3779b9u32;
    if length >= 4 {
        loop {
            let word = read_u32(str_);
            ascii_mask |= word;
            zero_mask |= 0x80808080u32.wrapping_sub(word);

            hash = ((hash << 5 | hash >> 27) ^ word).wrapping_mul(seed);
            str_ = str_.add(4);
            length -= 4;
            if !(length >= 4) {
                break;
            }
        }

        let word = read_u32(str_.add(length).sub(4));
        ascii_mask |= word;
        zero_mask |= 0x80808080u32.wrapping_sub(word);

        hash = ((hash << 5 | hash >> 27) ^ word).wrapping_mul(seed);
    } else {
        let mut word = 0u32;
        if length >= 1 {
            word |= (*str_.add(0) as u32) << 0;
        }
        if length >= 2 {
            word |= (*str_.add(1) as u32) << 8;
        }
        if length >= 3 {
            word |= (*str_.add(2) as u32) << 16;
        }

        ascii_mask |= word;
        // C-parity: at length == 0 the C shift amount is 32 (UB in C,
        // ufbx.c:4757; masks to `>> 0` on x86/ARM) and would be a debug-build
        // overflow panic here. Unreachable today via the unconditional
        // `ufbx_assert!(length > 0)` above; if that assert is ever feature-
        // gated off (no-assert), this shift must gain a `& 31` mask.
        zero_mask |= (0x80808080u32 >> ((4 - length) * 8)).wrapping_sub(word);

        hash = ((hash << 5 | hash >> 27) ^ word).wrapping_mul(seed);
    }

    // If any character has high bit set or is zero we're not ASCII
    if ((ascii_mask | zero_mask) & 0x80808080u32) != 0 {
        *p_non_ascii = true;
    }

    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb352d);
    hash ^= hash >> 15;

    hash
}

// ufbx.c:4781-4789 `ufbxi_hash32`
#[inline(always)]
pub(crate) fn hash32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846ca68b);
    x ^= x >> 16;
    x
}

// ufbx.c:4791-4799 `ufbxi_hash64`
#[inline(always)]
pub(crate) fn hash64(mut x: u64) -> u32 {
    x ^= x >> 32;
    x = x.wrapping_mul(0xd6e8feb86659fd93);
    x ^= x >> 32;
    x = x.wrapping_mul(0xd6e8feb86659fd93);
    x ^= x >> 32;
    x as u32
}

// ufbx.c:4801-4812 `ufbxi_hash_uptr`
// The C three-way `UFBXI_UINTPTR_SIZE` fork (8 / 4 / unknown-at-preprocess)
// maps to `target_pointer_width` cfgs; the runtime-sizeof fallback branch
// (CHERI targets, ufbx.c:862) has no rustc analogue — the byte-hash arm below
// preserves its behavior for any other pointer width.
#[inline(always)]
pub(crate) fn hash_uptr(ptr: usize) -> u32 {
    #[cfg(target_pointer_width = "64")]
    {
        hash64(ptr as u64)
    }
    #[cfg(target_pointer_width = "32")]
    {
        hash32(ptr as u32)
    }
    #[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
    {
        // C fallback: hash the pointer's bytes.
        unsafe { hash_string(&ptr as *const usize as *const u8, core::mem::size_of::<usize>()) }
    }
}

// ufbx.c:4814-4818 `ufbxi_hash_ptr_id`
#[inline(always)]
pub(crate) fn hash_ptr_id(id: PtrId) -> u32 {
    // Trivial reduction is fine: Only `ptr` or `id` is defined.
    hash_uptr(id.ptr) ^ hash64(id.id)
}

// ufbx.c:4820 `#define ufbxi_hash_ptr(ptr) ufbxi_hash_uptr((uintptr_t)(ptr))`
macro_rules! hash_ptr {
    ($ptr:expr) => {
        $crate::native::hash::hash_uptr(($ptr) as usize)
    };
}
pub(crate) use hash_ptr;

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn hs(b: &[u8]) -> u32 {
        hash_string(b.as_ptr(), b.len())
    }

    unsafe fn hsca(b: &[u8]) -> (u32, bool) {
        let mut non_ascii = false;
        let h = hash_string_check_ascii(b.as_ptr(), b.len(), &mut non_ascii);
        (h, non_ascii)
    }

    // Reference values from the C algorithm (little-endian reads).
    #[test]
    fn hash_string_known_values() {
        unsafe {
            assert_eq!(hs(b"a"), 0x88b1a51d);
            assert_eq!(hs(b"ab"), 0x198f65f0);
            assert_eq!(hs(b"abc"), 0x021f1b84);
            assert_eq!(hs(b"abcd"), 0xec8b77cd);
            assert_eq!(hs(b"abcde"), 0x2fe5a905);
            assert_eq!(hs(b"Geometry"), 0xf63e5026);
            assert_eq!(hs(b"ObjectType"), 0x11aa0eb0);
            assert_eq!(hs(b"\xffx"), 0x732948c2);
            assert_eq!(hs(b"a\x00b"), 0x257d0e0a);
        }
    }

    // C comment: "NOTE: _Must_ match `ufbxi_hash_string()`" — verify
    // bit-for-bit equality across all length classes (1-3, exact multiple of
    // 4, and the overlapping-tail path).
    #[test]
    fn check_ascii_matches_hash_string() {
        let data: &[u8] = b"The quick brown fox jumps over the lazy dog \xff\x00\x80\x01";
        unsafe {
            for start in 0..8 {
                for len in 1..=(data.len() - start) {
                    let slice = &data[start..start + len];
                    let (h, _) = hsca(slice);
                    assert_eq!(h, hs(slice), "mismatch for {:?}", slice);
                }
            }
        }
    }

    #[test]
    fn check_ascii_flag() {
        unsafe {
            // Pure ASCII (0x01..=0x7f) must NOT set the flag.
            assert_eq!(hsca(b"a").1, false);
            assert_eq!(hsca(b"ab").1, false);
            assert_eq!(hsca(b"abc").1, false);
            assert_eq!(hsca(b"abcd").1, false);
            assert_eq!(hsca(b"abcdefg").1, false);
            assert_eq!(hsca(b"\x01\x7f").1, false);
            // High bit set → non-ASCII, in every byte position of both paths.
            assert_eq!(hsca(b"\x80").1, true);
            assert_eq!(hsca(b"a\xff").1, true);
            assert_eq!(hsca(b"ab\x80").1, true);
            assert_eq!(hsca(b"abc\x80").1, true);
            assert_eq!(hsca(b"abcd\x80").1, true);
            assert_eq!(hsca(b"abcdefg\x80").1, true);
            // Embedded zero → non-ASCII.
            assert_eq!(hsca(b"a\x00").1, true);
            assert_eq!(hsca(b"a\x00b").1, true);
            assert_eq!(hsca(b"abc\x00").1, true);
            assert_eq!(hsca(b"abcdef\x00h").1, true);
        }
    }

    #[test]
    fn check_ascii_only_sets_flag() {
        // The C only ever writes `true`; a pre-set flag must survive an
        // all-ASCII string.
        unsafe {
            let mut non_ascii = true;
            hash_string_check_ascii(b"abc".as_ptr(), 3, &mut non_ascii);
            assert_eq!(non_ascii, true);
        }
    }

    #[test]
    fn hash32_known_values() {
        assert_eq!(hash32(0), 0);
        assert_eq!(hash32(1), 0x688990c0);
        assert_eq!(hash32(0xdeadbeef), 0xe628c683);
    }

    #[test]
    fn hash64_known_values() {
        assert_eq!(hash64(0), 0);
        assert_eq!(hash64(1), 0xe0c0e0d0);
        assert_eq!(hash64(0x123456789abcdef0), 0x079081a9);
    }

    #[test]
    fn hash_uptr_and_ptr_id() {
        #[cfg(target_pointer_width = "64")]
        assert_eq!(hash_uptr(1), hash64(1));
        #[cfg(target_pointer_width = "32")]
        assert_eq!(hash_uptr(1), hash32(1));

        // "Only `ptr` or `id` is defined" — trivial xor reduction.
        let a = PtrId { ptr: 0x1234, id: 0 };
        assert_eq!(hash_ptr_id(a), hash_uptr(0x1234) ^ hash64(0));
        let b = PtrId { ptr: 0, id: 77 };
        assert_eq!(hash_ptr_id(b), hash_uptr(0) ^ hash64(77));

        let x = 5u32;
        let p = &x as *const u32;
        assert_eq!(hash_ptr!(p), hash_uptr(p as usize));
    }
}
