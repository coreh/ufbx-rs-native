//! Port of the `// -- Float parsing` banner section:
//! the `ufbxi_bigint` arbitrary-precision machinery (ufbx.c:1349-1529), and
//! the strtod machinery on top of it (ufbx.c:1531-1846:
//! `ufbxi_parse_double_flag`, `ufbxi_scan_ignorecase`, `ufbxi_parse_inf_nan`,
//! `ufbxi_parse_double`, `ufbxi_parse_double_init_flags` — the volatile strtod
//! probe —, `ufbxi_parse_int64`, `ufbxi_parse_uint32_radix`).
//!
//! Limb arithmetic is wrapping-by-design (PORTING.md integer table): all
//! add/mul on limbs/accums use `wrapping_*`; every shift amount in this
//! section is statically bounded (< width) in C, so shifts port as plain
//! `<<`/`>>` (PORTING.md: "every `<<` in ufbx.c has statically bounded
//! amounts — port as plain `<<`").
//!
//! Collapsed-away C apparatus:
//! - ufbx.c:1369 `ufbxi_bigint_array(arr)`: `sizeof`-deriving wrapper around
//!   `ufbxi_bigint_make` — ported as the `bigint_array!` macro below.
//! - `ufbxi_arraycount` at the dev-assert sites (second half) collapses to
//!   `.len()`.
#![allow(dead_code, unused_macros, unused_imports)]

use crate::native::platform::{lzcnt32, lzcnt64, math, ufbxi_dev_assert, ufbxi_maybe_uninit};

// ufbx.c:1351-1355
pub(crate) const BIGINT_LIMB_BITS: u32 = 32;
pub(crate) const BIGINT_ACCUM_BITS: u32 = BIGINT_LIMB_BITS * 2;
pub(crate) const BIGINT_LIMB_MAX: BigintLimb =
    (((1 as BigintAccum) << BIGINT_LIMB_BITS) - 1) as BigintLimb;
// ufbx.c:1354 `typedef uint32_t ufbxi_bigint_limb;`
pub(crate) type BigintLimb = u32;
// ufbx.c:1355 `typedef uint64_t ufbxi_bigint_accum;`
pub(crate) type BigintAccum = u64;

// ufbx.c:1357-1361 `ufbxi_bigint`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Bigint {
    pub(crate) limbs: *mut BigintLimb,
    pub(crate) capacity: u32,
    pub(crate) length: u32,
}

// ufbx.c:1363-1367 `ufbxi_bigint_make`
// C aggregate init `{ limbs, (uint32_t)capacity }` zero-fills `length`.
pub(crate) fn bigint_make(limbs: *mut BigintLimb, capacity: usize) -> Bigint {
    Bigint { limbs, capacity: capacity as u32, length: 0 }
}

// ufbx.c:1369 `#define ufbxi_bigint_array(arr) ufbxi_bigint_make((arr), sizeof(arr) / sizeof(*(arr)))`
// Evaluate-once parity: C evaluates `arr` once at runtime (the second use is
// inside `sizeof`, unevaluated), so bind the argument before the double use.
macro_rules! bigint_array {
    ($arr:expr) => {{
        let arr = &mut $arr;
        $crate::native::float_parse::bigint_make(arr.as_mut_ptr(), arr.len())
    }};
}
pub(crate) use bigint_array;

// ufbx.c:1371-1376 `ufbxi_pow5_tab`
pub(crate) static POW5_TAB: [u64; 28] = [
    0x1, 0x5, 0x19, 0x7d, 0x271, 0xc35, 0x3d09, 0x1312d, 0x5f5e1,
    0x1dcd65, 0x9502f9, 0x2e90edd, 0xe8d4a51, 0x48c27395, 0x16bcc41e9, 0x71afd498d,
    0x2386f26fc1, 0xb1a2bc2ec5, 0x3782dace9d9, 0x1158e460913d, 0x56bc75e2d631, 0x1b1ae4d6e2ef5,
    0x878678326eac9, 0x2a5a058fc295ed, 0xd3c21bcecceda1, 0x422ca8b0a00a425, 0x14adf4b7320334b9, 0x6765c793fa10079d,
];

// ufbx.c:1378-1380 `ufbxi_pow10_tab_f64`
pub(crate) static POW10_TAB_F64: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];

// ufbx.c:1382-1402 `ufbxi_bigint_mad`
#[inline(never)]
pub(crate) unsafe fn bigint_mad(bigint: *mut Bigint, multiplicand: BigintAccum, addend: BigintAccum) {
    ufbxi_dev_assert!((multiplicand | addend) >> (BIGINT_ACCUM_BITS - 1) == 0);
    let mut b = *bigint;
    let m_lo = multiplicand as BigintLimb;
    let m_hi = (multiplicand >> BIGINT_LIMB_BITS) as BigintLimb;
    let mut carry: BigintAccum = addend;
    for i in 0..b.length {
        let limb = *b.limbs.add(i as usize) as BigintAccum;
        let lo = (limb.wrapping_mul(m_lo as BigintAccum))
            .wrapping_add(carry & BIGINT_LIMB_MAX as BigintAccum);
        let hi = limb.wrapping_mul(m_hi as BigintAccum);
        *b.limbs.add(i as usize) = lo as BigintLimb;
        carry = (carry >> 32u32).wrapping_add(lo >> 32u32).wrapping_add(hi);
    }
    while carry != 0 {
        // C: `b.limbs[b.length++] = (ufbxi_bigint_limb)carry;` — post-increment decomposed.
        *b.limbs.add(b.length as usize) = carry as BigintLimb;
        b.length += 1;
        ufbxi_dev_assert!(b.length < b.capacity);
        carry >>= 32u32;
    }
    (*bigint).length = b.length;
}

// ufbx.c:1404-1450 `ufbxi_bigint_div`
// Returns true if a (nonzero) remainder was left in the low limbs of `u`.
#[inline(never)]
pub(crate) unsafe fn bigint_div(q: *mut Bigint, u: *mut Bigint, v: *mut Bigint) -> bool {
    let n = (*v).length as i32;
    let m = (*u).length as i32 - n;
    let v_hi: BigintLimb = *(*v).limbs.add(((*v).length - 1) as usize);
    let un = (*u).limbs;
    let vn = (*v).limbs;
    ufbxi_dev_assert!(
        n >= 2 && m >= 1 && v_hi >> (BIGINT_LIMB_BITS - 1) != 0
            && *un.add((n + m - 1) as usize) >> (BIGINT_LIMB_BITS - 1) == 0
    );
    *un.add((n + m) as usize) = 0;
    (*q).length = 0;
    // C: `for (int32_t j = m - 1; j >= 0; j--)`
    let mut j = m - 1;
    while j >= 0 {
        let u_hi: BigintAccum = ((*un.add((n + j) as usize) as BigintAccum) << BIGINT_LIMB_BITS)
            | *un.add((n + j - 1) as usize) as BigintAccum;
        let mut t: BigintAccum;
        let mut qhat: BigintAccum = u_hi / v_hi as BigintAccum;
        let mut rhat: BigintAccum = u_hi % v_hi as BigintAccum;
        while qhat >> BIGINT_LIMB_BITS != 0
            || qhat.wrapping_mul(*vn.add((n - 2) as usize) as BigintAccum)
                > ((rhat << BIGINT_LIMB_BITS) | *un.add((j + n - 2) as usize) as BigintAccum)
        {
            qhat = qhat.wrapping_sub(1);
            rhat = rhat.wrapping_add(v_hi as BigintAccum);
            if rhat >> BIGINT_LIMB_BITS != 0 {
                break;
            }
        }
        let mut carry: BigintLimb = 0;
        for i in 0..n {
            let p: BigintAccum = qhat.wrapping_mul(*vn.add(i as usize) as BigintAccum);
            // C: `t = (ufbxi_bigint_accum)un[i+j] - carry - (ufbxi_bigint_limb)p;` — borrow trick, wraps.
            t = (*un.add((i + j) as usize) as BigintAccum)
                .wrapping_sub(carry as BigintAccum)
                .wrapping_sub((p as BigintLimb) as BigintAccum);
            *un.add((i + j) as usize) = t as BigintLimb;
            carry = ((p >> BIGINT_LIMB_BITS).wrapping_sub(t >> BIGINT_LIMB_BITS)) as BigintLimb;
        }
        t = (*un.add((j + n) as usize) as BigintAccum).wrapping_sub(carry as BigintAccum);
        *un.add((j + n) as usize) = t as BigintLimb;
        if t >> BIGINT_LIMB_BITS != 0 {
            qhat = qhat.wrapping_sub(1);
            carry = 0;
            for i in 0..n {
                t = (*un.add((i + j) as usize) as BigintAccum)
                    .wrapping_add(*vn.add(i as usize) as BigintAccum)
                    .wrapping_add(carry as BigintAccum);
                *un.add((i + j) as usize) = t as BigintLimb;
                carry = (t >> BIGINT_LIMB_BITS) as BigintLimb;
            }
            // C: `un[j+n] += carry;`
            *un.add((j + n) as usize) = (*un.add((j + n) as usize)).wrapping_add(carry);
        }
        *(*q).limbs.add(j as usize) = qhat as BigintLimb;
        if qhat != 0 && (*q).length == 0 {
            ufbxi_dev_assert!(j + 1 < (*q).capacity as i32);
            (*q).length = (j + 1) as u32;
        }
        j -= 1;
    }
    for i in 0..n {
        if *un.add(i as usize) != 0 {
            return true;
        }
    }
    false
}

// ufbx.c:1452-1458 `ufbxi_bigint_mul_pow5`
pub(crate) unsafe fn bigint_mul_pow5(b: *mut Bigint, power: u32) {
    let mut power = power;
    while power > 27 {
        bigint_mad(b, POW5_TAB[27], 0);
        power -= 27;
    }
    bigint_mad(b, POW5_TAB[power as usize], 0);
}

// ufbx.c:1460-1488 `ufbxi_bigint_shift_left`
#[inline(never)]
pub(crate) unsafe fn bigint_shift_left(bigint: *mut Bigint, amount: u32) {
    let words = amount / BIGINT_LIMB_BITS;
    let bits = amount % BIGINT_LIMB_BITS;
    let b = *bigint;
    ufbxi_dev_assert!(b.length + words + 1 < b.capacity && b.capacity >= 4);
    let bits_down = BIGINT_LIMB_BITS - bits - 1;
    // C: `bigint->length += words + (b.limbs[b.length - 1] >> 1 >> bits_down != 0 ? 1 : 0);`
    // (the local copy `b` keeps the OLD length for the rest of the function).
    // C-parity: with `b.length == 0` the C reads `limbs[(uint32_t)-1]` — UB;
    // here `b.length - 1` underflows (debug panic / wild index in release).
    // Unreachable from all callers (they always shift a nonzero bigint), so
    // divergence-in-the-unreachable is accepted per PORTING.md ground rule 4.
    (*bigint).length +=
        words + if *b.limbs.add((b.length - 1) as usize) >> 1 >> bits_down != 0 { 1 } else { 0 };
    *b.limbs.add(b.length as usize) = 0;
    if b.length <= 3 && words <= 3 {
        let l0: BigintLimb = *b.limbs.add(0);
        let l1: BigintLimb = ufbxi_maybe_uninit!(b.length >= 1, *b.limbs.add(1), !0u32);
        let l2: BigintLimb = ufbxi_maybe_uninit!(b.length >= 2, *b.limbs.add(2), !0u32);
        *b.limbs.add(0) = 0;
        *b.limbs.add(1) = 0;
        *b.limbs.add(2) = 0;
        *b.limbs.add((words + 0) as usize) = l0 << bits;
        *b.limbs.add((words + 1) as usize) = (l1 << bits) | (l0 >> 1 >> bits_down);
        *b.limbs.add((words + 2) as usize) = (l2 << bits) | (l1 >> 1 >> bits_down);
        *b.limbs.add((words + 3) as usize) = l2 >> 1 >> bits_down;
    } else {
        // C: `for (uint32_t i = b.length + 1; i-- > 1; )` — body sees i from b.length down to 1.
        let mut i = b.length + 1;
        while i > 1 {
            i -= 1;
            *b.limbs.add((i + words) as usize) =
                (*b.limbs.add(i as usize) << bits) | (*b.limbs.add((i - 1) as usize) >> 1 >> bits_down);
        }
        *b.limbs.add(words as usize) = *b.limbs.add(0) << bits;
        for i in 0..words {
            *b.limbs.add(i as usize) = 0;
        }
    }
}

// ufbx.c:1490-1492 `ufbxi_bigint_top_limb` (takes the bigint by value, as C does)
pub(crate) unsafe fn bigint_top_limb(b: Bigint, index: u32) -> BigintLimb {
    if index < b.length { *b.limbs.add((b.length - 1 - index) as usize) } else { 0 }
}

// ufbx.c:1494-1514 `ufbxi_bigint_extract_high`
// C-parity: callers pass a bigint whose top limb is nonzero (mad counts length
// to the top nonzero carry; div sets q->length at the highest nonzero qhat), so
// `result` has its top 32 bits nonzero and `shift < 32` — the shifts below are
// in-range exactly when C's are (out-of-range would be UB in C too).
#[inline(never)]
pub(crate) unsafe fn bigint_extract_high(b: Bigint, p_exponent: *mut i32, p_tail: *mut bool) -> u64 {
    ufbxi_dev_assert!(b.length != 0);
    let mut result: u64 = 0;
    let limb_count: u32 = 64 / BIGINT_LIMB_BITS;
    for i in 0..limb_count {
        result = (result << BIGINT_LIMB_BITS) | bigint_top_limb(b, i) as u64;
    }
    let shift = lzcnt64(result);
    result <<= shift;
    let lo: BigintLimb = bigint_top_limb(b, limb_count);
    if shift > 0 {
        result |= (lo >> (BIGINT_LIMB_BITS - shift)) as u64;
    }
    *p_tail |= (lo << shift) as BigintLimb != 0;
    for i in (limb_count + 1)..b.length {
        *p_tail |= bigint_top_limb(b, i) != 0;
    }
    *p_exponent += (b.length * BIGINT_LIMB_BITS - shift - 1) as i32;
    result
}

// ufbx.c:1516-1529 `ufbxi_shift_right_round`
pub(crate) fn shift_right_round(value: u64, shift: u32, tail: bool) -> u64 {
    if shift == 0 {
        return value;
    }
    if shift > 64 {
        return 0;
    }
    let result = value >> (shift - 1);
    let tail_mask: u64 = (1u64 << (shift - 1)) - 1;

    let r_odd = (result & 0x2) != 0;
    let r_round = (result & 0x1) != 0;
    let r_tail = tail || (value & tail_mask) != 0;
    let round_bit: u64 = if r_round && (r_odd || r_tail) { 1 } else { 0 };

    (result >> 1u32).wrapping_add(round_bit)
}

// ufbx.c:1531-1534 `ufbxi_parse_double_flag`
// C: `typedef enum { ... } ufbxi_parse_double_flag;` — plain bit-flag constants.
pub(crate) const PARSE_DOUBLE_ALLOW_FAST_PATH: u32 = 0x1;
pub(crate) const PARSE_DOUBLE_AS_BINARY32: u32 = 0x2;

// ufbx.c:1536-1543 `ufbxi_scan_ignorecase`
// C `fmt` is a NUL-terminated literal scanned with `for (f = fmt; *f; f++, p++)`;
// the Rust byte-slice iteration visits exactly the same characters.
pub(crate) unsafe fn scan_ignorecase(p: *const u8, end: *const u8, fmt: &[u8]) -> bool {
    let mut p = p;
    for &f in fmt {
        if p >= end {
            return false;
        }
        if (*p | 0x20) != f {
            return false;
        }
        p = p.add(1);
    }
    true
}

// ufbx.c:1545-1599 `ufbxi_parse_inf_nan`
#[inline(never)]
pub(crate) unsafe fn parse_inf_nan(
    p_result: *mut f64,
    str_: *const u8,
    max_length: usize,
    p_end: *mut *const u8,
) -> bool {
    let mut negative = false;
    let mut p = str_;
    let end = p.add(max_length);
    if p != end && (*p == b'+' || *p == b'-') {
        // C: `negative = *p++ == '-';` — post-increment decomposed.
        negative = *p == b'-';
        p = p.add(1);
    }

    let mut top_bits: u32 = 0;
    // C: `end - p >= 3` (ptrdiff comparison).
    if end as usize - p as usize >= 3
        && (*p >= b'0' && *p <= b'9')
        && *p.add(1) == b'.'
        && *p.add(2) == b'#'
    {
        // Legacy MSVC 1.#NAN
        p = p.add(3);
        if scan_ignorecase(p, end, b"inf") {
            p = p.add(3);
            top_bits = 0x7ff0;
        } else if scan_ignorecase(p, end, b"nan") || scan_ignorecase(p, end, b"ind") {
            p = p.add(3);
            top_bits = 0x7ff8;
        } else {
            return false;
        }
        while p != end && *p >= b'0' && *p <= b'9' {
            p = p.add(1);
        }
    } else {
        // Standard
        if scan_ignorecase(p, end, b"nan") {
            p = p.add(3);
            top_bits = 0x7ff8;
            if p != end && *p == b'(' {
                p = p.add(1);
                while p != end && *p != b')' {
                    let c = *p;
                    if !((c >= b'0' && c <= b'9')
                        || (c >= b'a' && c <= b'z')
                        || (c >= b'A' && c <= b'Z'))
                    {
                        return false;
                    }
                    p = p.add(1);
                }
                if p == end {
                    return false;
                }
                p = p.add(1);
            }
        } else if scan_ignorecase(p, end, b"inf") {
            p = p.add(if scan_ignorecase(p.add(3), end, b"inity") { 8 } else { 3 });
            top_bits = 0x7ff0;
        }
    }

    *p_end = p;
    top_bits |= if negative { 0x8000 } else { 0 };
    let bits = (top_bits as u64) << 48;
    // C: `ufbxi_bit_cast(double, result, uint64_t, bits);`
    let result = f64::from_bits(bits);
    *p_result = result;
    true
}

// ufbx.c:1601-1793 `ufbxi_parse_double`
#[inline(never)]
pub(crate) unsafe fn parse_double(
    str_: *const u8,
    max_length: usize,
    p_end: *mut *const u8,
    flags: u32,
) -> f64 {
    let max_limbs: u32 = 14;

    // C: `ufbxi_bigint_limb mantissa_limbs[42], divisor_limbs[42],
    // quotient_limbs[42];` — uninitialized in C; zero-filled here (the bigint
    // ops never read a limb they did not write, so zero-fill is inert).
    let mut mantissa_limbs = [0 as BigintLimb; 42];
    let mut divisor_limbs = [0 as BigintLimb; 42];
    let mut quotient_limbs = [0 as BigintLimb; 42];
    let mut big_mantissa = bigint_array!(mantissa_limbs);
    let mut big_quotient = bigint_array!(quotient_limbs);
    // C: `int32_t dec_exponent = 0, has_dot = 0;` — has_dot stays an i32
    // (`has_dot = true` promotes to 1; it feeds integer arithmetic below).
    let mut dec_exponent: i32 = 0;
    let mut has_dot: i32 = 0;
    let mut negative = false;
    let mut tail = false;
    let mut digits_valid = true;
    let mut digits: u64 = 0;
    let mut num_digits: u32 = 0;

    let mut p = str_;
    let end = p.add(max_length);
    if p != end && (*p == b'+' || *p == b'-') {
        // C: `negative = *p++ == '-';` — post-increment decomposed.
        negative = *p == b'-';
        p = p.add(1);
    }
    while p != end {
        let c = *p;
        if c >= b'0' && c <= b'9' {
            if big_mantissa.length < max_limbs {
                digits = digits * 10 + (c - b'0') as u64;
                num_digits += 1;
                if num_digits >= 18 {
                    ufbxi_dev_assert!((num_digits as usize) < POW5_TAB.len());
                    bigint_mad(
                        &mut big_mantissa,
                        POW5_TAB[num_digits as usize] << num_digits,
                        digits,
                    );
                    digits = 0;
                    num_digits = 0;
                    digits_valid = false;
                }
                // C: `dec_exponent -= has_dot;` — wrapping: only overflows for
                // inputs with >2^31 digits (UB in C as well).
                dec_exponent = dec_exponent.wrapping_sub(has_dot);
            } else {
                // C: `dec_exponent += 1 - has_dot;` — same wrapping note.
                dec_exponent = dec_exponent.wrapping_add(1 - has_dot);
            }
            p = p.add(1);
        } else if c == b'.' && has_dot == 0 {
            has_dot = 1; // C: `has_dot = true;`
            p = p.add(1);
        } else {
            break;
        }
    }
    if p != end && (*p == b'e' || *p == b'E') {
        p = p.add(1);
        let mut exp_negative = false;
        if p != end && (*p == b'+' || *p == b'-') {
            exp_negative = *p == b'-';
            p = p.add(1);
        }
        let mut exp: i32 = 0;
        while p != end {
            let c = *p;
            if c >= b'0' && c <= b'9' {
                p = p.add(1);
                exp = exp * 10 + (c - b'0') as i32;
                if exp >= 10000 {
                    break;
                }
            } else {
                break;
            }
        }
        // C: `dec_exponent += exp_negative ? -exp : exp;` — wrapping as above.
        dec_exponent = dec_exponent.wrapping_add(if exp_negative { -exp } else { exp });
    }

    if p != end {
        let c = *p;
        if c == b'#' || c == b'i' || c == b'I' || c == b'n' || c == b'N' {
            // C: `double result;` (uninitialized; written before the read).
            let mut result: f64 = 0.0;
            if parse_inf_nan(&mut result, str_, max_length, p_end) {
                return result;
            }
        }
    }

    *p_end = p;

    // Both power of 10 and integer are exactly representable as doubles
    // Powers of 10 are factored as 2*5, and 2^N can be always exactly represented.
    if (flags & PARSE_DOUBLE_ALLOW_FAST_PATH) != 0
        && big_mantissa.length == 0
        && dec_exponent >= -22
        && dec_exponent <= 22
        && (digits >> 53) == 0
    {
        let value: f64;
        if dec_exponent < 0 {
            value = digits as f64 / POW10_TAB_F64[(-dec_exponent) as usize];
        } else {
            value = digits as f64 * POW10_TAB_F64[dec_exponent as usize];
        }
        return if negative { -value } else { value };
    }

    if big_mantissa.length == 0 {
        *big_mantissa.limbs.add(0) = digits as BigintLimb;
        *big_mantissa.limbs.add(1) = (digits >> 32u32) as BigintLimb;
        // C: `big_mantissa.length = (digits >> 32u) ? 2 : digits ? 1 : 0;`
        big_mantissa.length = if (digits >> 32u32) != 0 {
            2
        } else if digits != 0 {
            1
        } else {
            0
        };
        if big_mantissa.length == 0 {
            return if negative { -0.0 } else { 0.0 };
        }
    } else {
        ufbxi_dev_assert!((num_digits as usize) < POW5_TAB.len());
        bigint_mad(&mut big_mantissa, POW5_TAB[num_digits as usize] << num_digits, digits);
    }

    let mut enc_sign_shift: u32 = 63;
    let mut enc_mantissa_bits: u32 = 53;
    let mut enc_max_exponent: i32 = 1023;
    if (flags & PARSE_DOUBLE_AS_BINARY32) != 0 {
        enc_sign_shift = 31;
        enc_mantissa_bits = 24;
        enc_max_exponent = 127;
    }

    let mut exponent: i32 = 0;
    if dec_exponent < 0 {
        // C: `dec_exponent + (int32_t)big_mantissa.length * 10 <= -325`
        if dec_exponent.wrapping_add(big_mantissa.length as i32 * 10) <= -325 {
            return if negative { -0.0 } else { 0.0 };
        }

        let mut big_divisor = bigint_array!(divisor_limbs);
        // C: `uint32_t pow5 = (uint32_t)-dec_exponent;` — negation in `int`
        // (UB only at INT32_MIN, unreachable); wrapping negation matches.
        let mut pow5: u32 = (dec_exponent as u32).wrapping_neg();
        let initial_pow5: u32 = if pow5 <= 27 { pow5 } else { 27 };
        let pow5_value: u64 = POW5_TAB[initial_pow5 as usize];
        pow5 -= initial_pow5;
        exponent += dec_exponent;

        if pow5 == 0 && digits_valid && digits >> 63 == 0 {
            let divisor_zeros: u32 = lzcnt64(pow5_value);
            // C: `uint64_t mantissa_zeros = ufbxi_lzcnt64(digits) - 1;`
            // (`digits >> 63 == 0` guarantees lzcnt >= 1 — no underflow).
            let mantissa_zeros: u64 = (lzcnt64(digits) - 1) as u64;
            let divisor_bits: u64 = pow5_value << divisor_zeros;
            let mantissa_bits: u64 = digits << mantissa_zeros;
            *big_divisor.limbs.add(0) = divisor_bits as BigintLimb;
            *big_divisor.limbs.add(1) = (divisor_bits >> 32u32) as BigintLimb;
            big_divisor.length = 2;
            *big_mantissa.limbs.add(0) = 0;
            *big_mantissa.limbs.add(1) = 0;
            *big_mantissa.limbs.add(2) = mantissa_bits as BigintLimb;
            *big_mantissa.limbs.add(3) = (mantissa_bits >> 32u32) as BigintLimb;
            big_mantissa.length = 4;
            exponent += divisor_zeros as i32 - mantissa_zeros as i32 - 64;
        } else {
            *big_divisor.limbs.add(0) = pow5_value as BigintLimb;
            *big_divisor.limbs.add(1) = (pow5_value >> 32u32) as BigintLimb;
            big_divisor.length = if (pow5_value >> 32u32) != 0 { 2 } else { 1 };
            if pow5 > 0 {
                bigint_mul_pow5(&mut big_divisor, pow5);
            }

            let mut divisor_zeros: u32 =
                lzcnt32(*big_divisor.limbs.add((big_divisor.length - 1) as usize));
            if big_divisor.length == 1 {
                divisor_zeros += BIGINT_LIMB_BITS;
            }
            bigint_shift_left(&mut big_divisor, divisor_zeros);
            let divisor_bits: u32 = big_divisor.length * BIGINT_LIMB_BITS;

            let mantissa_zeros: u32 =
                lzcnt32(*big_mantissa.limbs.add((big_mantissa.length - 1) as usize));
            let mantissa_bits: u32 = big_mantissa.length * BIGINT_LIMB_BITS - mantissa_zeros;
            let mantissa_min_bits: u32 = divisor_bits + enc_mantissa_bits + 2;
            let mut mantissa_shift: u32 = if mantissa_bits < mantissa_min_bits {
                mantissa_min_bits - mantissa_bits
            } else {
                0
            };
            // Align mantissa to never have a high bit, this means we can skip the first digit during division.
            // C: `(mantissa_shift - mantissa_zeros)` — unsigned subtraction, wraps.
            mantissa_shift +=
                if (mantissa_shift.wrapping_sub(mantissa_zeros)) & (BIGINT_LIMB_BITS - 1) == 0 {
                    1
                } else {
                    0
                };
            if mantissa_shift > 0 {
                bigint_shift_left(&mut big_mantissa, mantissa_shift);
            }
            exponent += divisor_zeros as i32 - mantissa_shift as i32;
        }

        tail = bigint_div(&mut big_quotient, &mut big_mantissa, &mut big_divisor);
        big_mantissa = big_quotient;
    } else if dec_exponent > 0 {
        // C: `dec_exponent + (int32_t)(big_mantissa.length - 1) * 9 >= 310`
        if dec_exponent.wrapping_add((big_mantissa.length.wrapping_sub(1)) as i32 * 9) >= 310 {
            return if negative { -math::INFINITY } else { math::INFINITY };
        }

        exponent += dec_exponent;
        bigint_mul_pow5(&mut big_mantissa, dec_exponent as u32);
    }

    let mut mantissa: u64 = bigint_extract_high(big_mantissa, &mut exponent, &mut tail);
    let sign_bit: u64 = (if negative { 1u64 } else { 0u64 }) << enc_sign_shift;

    let mut mantissa_shift: u32 = 64 - enc_mantissa_bits;
    if exponent > enc_max_exponent {
        return if negative { -math::INFINITY } else { math::INFINITY };
    } else if exponent <= -enc_max_exponent {
        mantissa_shift += (-enc_max_exponent + 1 - exponent) as u32;
        exponent = -enc_max_exponent + 1;
    }

    mantissa = shift_right_round(mantissa, mantissa_shift, tail);
    if mantissa == 0 {
        return if negative { -0.0 } else { 0.0 };
    }

    let mut bits: u64 = mantissa;
    // C: `bits += (uint64_t)(exponent + enc_max_exponent - 1) << (enc_mantissa_bits - 1);`
    // (i32 → u64 sign-extends in both languages; the sum wraps in u64).
    bits = bits
        .wrapping_add(((exponent + enc_max_exponent - 1) as u64) << (enc_mantissa_bits - 1));
    bits |= sign_bit;

    if (flags & PARSE_DOUBLE_AS_BINARY32) != 0 {
        let bits_lo: u32 = bits as u32;
        // C: `ufbxi_bit_cast(float, result, uint32_t, bits_lo); return result;`
        // (the float return value converts to double at the call boundary).
        let result = f32::from_bits(bits_lo);
        result as f64
    } else {
        // C: `ufbxi_bit_cast(double, result, uint64_t, bits);`
        f64::from_bits(bits)
    }
}

// ufbx.c:1795-1805 `ufbxi_parse_double_init_flags`
#[inline(never)]
pub(crate) fn parse_double_init_flags() -> u32 {
    // We require evaluation in double precision, either for doubles (0) or always (1)
    // and rounding to nearest, which we can check for with `1 + eps == 1 - eps`.
    // C gate: `#if UFBX_FLT_EVAL_METHOD == 0 || UFBX_FLT_EVAL_METHOD == 1` —
    // FLT_EVAL_METHOD is a const in the Rust port (platform::math), so the
    // preprocessor conditional becomes a const-foldable `if`.
    if math::FLT_EVAL_METHOD == 0 || math::FLT_EVAL_METHOD == 1 {
        // C: `static volatile double ufbxi_volatile_eps = 2.2250738585072014e-308;`
        // The volatile read is load-bearing (PORTING.md "Floats"): a plain
        // static read const-folds and silently pins one strtod path. C reads
        // the volatile once per comparison operand — keep both reads.
        static VOLATILE_EPS: f64 = 2.2250738585072014e-308;
        let lhs = 1.0 + unsafe { core::ptr::read_volatile(&VOLATILE_EPS) };
        let rhs = 1.0 - unsafe { core::ptr::read_volatile(&VOLATILE_EPS) };
        if lhs == rhs {
            return PARSE_DOUBLE_ALLOW_FAST_PATH;
        }
    }

    0
}

// ufbx.c:1807-1828 `ufbxi_parse_int64`
// C reads `str[len]` without an end bound (max 30 chars) — callers guarantee
// readable memory past the number (NUL-terminated token buffers).
#[inline(always)]
pub(crate) unsafe fn parse_int64(str_: *const u8, end: *mut *const u8) -> i64 {
    let mut abs_val: u64 = 0;
    let negative = *str_ == b'-';
    let positive = *str_ == b'+';

    // C: `size_t init_len = (negative | positive) ? 1 : 0;` — non-short-circuit.
    let init_len: usize = if negative | positive { 1 } else { 0 };
    let mut len = init_len;
    while len < 30 {
        let c = *str_.add(len);
        if !(c >= b'0' && c <= b'9') {
            break;
        }
        // C: `abs_val = 10 * abs_val + (uint64_t)(c - '0');` — unsigned, wraps
        // for >20-digit inputs (the loop admits up to 29 digits).
        abs_val = abs_val.wrapping_mul(10).wrapping_add((c - b'0') as u64);
        len += 1;
    }
    if len == 30 || len == init_len {
        *end = core::ptr::null();
        return 0;
    }

    // TODO: Wrap/clamp?
    *end = str_.add(len);
    // C: `negative ? (int64_t)(0 - abs_val) : (int64_t)abs_val;` — the
    // canonical `0u64.wrapping_sub` site (PORTING.md integer table, ufbx.c:1827).
    if negative { (0u64.wrapping_sub(abs_val)) as i64 } else { abs_val as i64 }
}

// ufbx.c:1830-1846 `ufbxi_parse_uint32_radix`
// C scans until the first non-digit with no end bound — callers pass
// NUL-terminated buffers (XML entity parsing, ufbx.c:7412-7414).
#[inline(never)]
pub(crate) unsafe fn parse_uint32_radix(str_: *const u8, radix: u32) -> u32 {
    let mut value: u32 = 0;
    let mut p = str_;
    loop {
        let c = *p;
        if c >= b'0' && c <= b'9' {
            // C: `value = value * radix + (uint32_t)(c - '0');` — unsigned, wraps.
            value = value.wrapping_mul(radix).wrapping_add((c - b'0') as u32);
        } else if radix == 16 && (c >= b'a' && c <= b'f') {
            // C: `(uint32_t)(c + (10 - 'a'))` — computed in `int` (integer
            // promotion, PORTING.md trap #8): widen to i32 before the add,
            // NOT u8 math.
            value = value
                .wrapping_mul(radix)
                .wrapping_add((c as i32 + (10 - b'a' as i32)) as u32);
        } else if radix == 16 && (c >= b'A' && c <= b'F') {
            value = value
                .wrapping_mul(radix)
                .wrapping_add((c as i32 + (10 - b'A' as i32)) as u32);
        } else {
            break;
        }
        p = p.add(1);
    }
    value
}

// Ported white-box tests from test/unit_tests.c — the correctness oracle for
// this unit (helpers test/unit_tests.c:363-455, tests 457-647; strtod helpers
// 649-718, tests 720-881; bigdecimal helpers 286-361).
#[cfg(test)]
mod tests {
    use super::*;

    // test/unit_tests.c:363-384 `ufbxt_bigint_div_word`
    unsafe fn bigint_div_word(b: *mut Bigint, divisor: BigintLimb) -> BigintLimb {
        let mut new_length: u32 = 0;
        let mut accum: BigintAccum = 0;
        // C: `for (uint32_t i = b->length; i-- > 0; )`
        let mut i = (*b).length;
        while i > 0 {
            i -= 1;
            accum = (accum << BIGINT_LIMB_BITS) | *(*b).limbs.add(i as usize) as BigintAccum;
            if accum >= divisor as BigintAccum {
                let quot: BigintAccum = accum / divisor as BigintAccum;
                let rem: BigintAccum = accum % divisor as BigintAccum;

                *(*b).limbs.add(i as usize) = quot as BigintLimb;
                if quot > 0 && new_length == 0 {
                    new_length = i + 1;
                }
                accum = rem;
            } else {
                *(*b).limbs.add(i as usize) = 0;
            }
        }
        (*b).length = new_length;
        accum as BigintLimb
    }

    // test/unit_tests.c:386-406 `ufbxt_bigint_parse`
    unsafe fn bigint_parse(b: *mut Bigint, str_: &str) {
        let mut radix: BigintLimb = 10;
        let mut s = str_.as_bytes();
        if s.len() >= 2 && s[0] == b'0' && s[1] == b'x' {
            radix = 16;
            s = &s[2..];
        }

        (*b).length = 0;
        for &c in s {
            let mut digit: BigintLimb = BIGINT_LIMB_MAX;
            if c >= b'0' && c <= b'9' {
                digit = (c - b'0') as BigintLimb;
            } else if c >= b'a' && c <= b'z' {
                digit = (c - b'a') as BigintLimb + 10;
            }

            assert!(digit < radix);
            bigint_mad(b, radix as BigintAccum, digit as BigintAccum);
        }
    }

    // test/unit_tests.c:408-426 `ufbxt_bigint_format`
    unsafe fn bigint_format(bi: Bigint, radix: BigintLimb) -> String {
        let mut limbs = [0 as BigintLimb; 64];
        core::ptr::copy_nonoverlapping(bi.limbs as *const BigintLimb, limbs.as_mut_ptr(), bi.length as usize);
        let mut b = Bigint { limbs: limbs.as_mut_ptr(), capacity: 64, length: bi.length };

        let digits = b"0123456789abcdef";

        let mut buffer = [0u8; 256];
        let mut pos = buffer.len();
        pos -= 1;
        buffer[pos] = 0; // '\0'
        loop {
            let digit = bigint_div_word(&mut b, radix);
            assert!(digit < radix);
            pos -= 1;
            buffer[pos] = digits[digit as usize];
            if b.length == 0 {
                break;
            }
        }

        String::from_utf8(buffer[pos..buffer.len() - 1].to_vec()).unwrap()
    }

    // test/unit_tests.c:428-448 `ufbxt_check_bigint`
    unsafe fn check_bigint(bi: Bigint, expected_: &str) {
        let mut radix: BigintLimb = 10;
        let mut expected = expected_;
        if expected.as_bytes().len() >= 2 && expected.as_bytes()[0] == b'0' && expected.as_bytes()[1] == b'x' {
            radix = 16;
            expected = &expected[2..];
        }

        let parsed = bigint_format(bi, radix);
        assert_eq!(parsed, expected, "ufbxt_check_bigint() fail: got {}, expected {}", parsed, expected);

        // Leading zero is not allowed
        if bi.length > 0 {
            assert!(*bi.limbs.add((bi.length - 1) as usize) != 0);
        }
    }

    // test/unit_tests.c:450-455 `ufbxt_bigint_copy`
    unsafe fn bigint_copy(dst: *mut Bigint, src: *const Bigint) {
        core::ptr::copy_nonoverlapping((*src).limbs as *const BigintLimb, (*dst).limbs, (*src).length as usize);
        (*dst).length = (*src).length;
        ufbxi_dev_assert!((*dst).capacity > (*src).length);
    }

    // test/unit_tests.c:457-473 `test_bigint_basics`
    #[test]
    fn test_bigint_basics() {
        unsafe {
            let mut a_limbs = [0 as BigintLimb; 64];
            let mut a = bigint_array!(a_limbs);

            bigint_parse(&mut a, "123");
            check_bigint(a, "123");

            bigint_parse(&mut a, "1230000000000000000000000000000000000000000456");
            check_bigint(a, "1230000000000000000000000000000000000000000456");

            bigint_parse(&mut a, "0xdead00beef00ab00cd00ef0012003400560079009a");
            check_bigint(a, "0xdead00beef00ab00cd00ef0012003400560079009a");

            bigint_parse(&mut a, "0x0000000000000000000000dead00beef00ab00cd00ef0012003400560079009a");
            check_bigint(a, "0xdead00beef00ab00cd00ef0012003400560079009a");
        }
    }

    // test/unit_tests.c:475-496 `test_bigint_mad`
    #[test]
    fn test_bigint_mad() {
        unsafe {
            let mut a_limbs = [0 as BigintLimb; 64];
            let mut a = bigint_array!(a_limbs);

            bigint_parse(&mut a, "1000");
            bigint_mad(&mut a, 4, 321);
            check_bigint(a, "4321");

            bigint_parse(&mut a, "4000000000");
            bigint_mad(&mut a, 2, 0);
            check_bigint(a, "8000000000");

            bigint_parse(&mut a, "0xffffffffffffffffffffffffffffffff");
            bigint_mad(&mut a, 9223372036854775807u64, 9223372036854775807u64);
            check_bigint(a, "0x7fffffffffffffff00000000000000000000000000000000");

            bigint_parse(&mut a, "0");
            bigint_mad(&mut a, 1000000000000000000u64, 111222333344455566u64);
            bigint_mad(&mut a, 10000000000u64, 6777888999u64);
            check_bigint(a, "1112223333444555666777888999");
        }
    }

    // test/unit_tests.c:498-529 `test_bigint_pow5`
    #[test]
    fn test_bigint_pow5() {
        unsafe {
            let mut a_limbs = [0 as BigintLimb; 64];
            let mut a = bigint_array!(a_limbs);

            bigint_parse(&mut a, "1000");
            bigint_mul_pow5(&mut a, 1);
            check_bigint(a, "5000");

            bigint_parse(&mut a, "1");
            bigint_mul_pow5(&mut a, 20);
            check_bigint(a, "95367431640625");

            bigint_parse(&mut a, "1");
            bigint_mul_pow5(&mut a, 40);
            check_bigint(a, "9094947017729282379150390625");

            bigint_parse(&mut a, "1");
            bigint_mul_pow5(&mut a, 300);
            check_bigint(a, concat!(
                "49090934652977265530957719549862756429752155124994495651115491171871052547217158",
                "56460097884037331952277183571565131878513167918610424718902807514824108963452253",
                "10546445986192853894181098439730703830718994140625"));

            bigint_parse(&mut a, "123");
            bigint_mul_pow5(&mut a, 0);
            check_bigint(a, "123");

            bigint_parse(&mut a, "10000000000010000000000010000000000");
            bigint_mul_pow5(&mut a, 1);
            check_bigint(a, "50000000000050000000000050000000000");
        }
    }

    // test/unit_tests.c:531-563 `test_bigint_shift_left`
    #[test]
    fn test_bigint_shift_left() {
        unsafe {
            let mut a_limbs = [0 as BigintLimb; 64];
            let mut a = bigint_array!(a_limbs);

            bigint_parse(&mut a, "1");
            bigint_shift_left(&mut a, 3);
            check_bigint(a, "8");

            bigint_parse(&mut a, "0x80000000");
            bigint_shift_left(&mut a, 1);
            check_bigint(a, "0x100000000");

            bigint_parse(&mut a, "0x123456789abcdef0123456789abcdef0123456789abcdef");
            bigint_shift_left(&mut a, 20);
            check_bigint(a, "0x123456789abcdef0123456789abcdef0123456789abcdef00000");

            bigint_parse(&mut a, "123456789");
            bigint_shift_left(&mut a, 0);
            check_bigint(a, "123456789");

            bigint_parse(&mut a, "12345678900000000000000000000");
            bigint_shift_left(&mut a, 0);
            check_bigint(a, "12345678900000000000000000000");

            bigint_parse(&mut a, "1");
            bigint_shift_left(&mut a, 32);
            check_bigint(a, "0x100000000");

            bigint_parse(&mut a, "0xa");
            bigint_shift_left(&mut a, 48);
            check_bigint(a, "0xa000000000000");
        }
    }

    // test/unit_tests.c:565-578 `test_bigint_div_manual`
    #[test]
    fn test_bigint_div_manual() {
        unsafe {
            let mut a_limbs = [0 as BigintLimb; 64];
            let mut b_limbs = [0 as BigintLimb; 64];
            let mut c_limbs = [0 as BigintLimb; 64];
            let mut a = bigint_array!(a_limbs);
            let mut b = bigint_array!(b_limbs);
            let mut c = bigint_array!(c_limbs);
            let rem: bool;

            bigint_parse(&mut a, "0x10000000000000000");
            bigint_parse(&mut b, "0x8000000000000000");
            rem = bigint_div(&mut c, &mut a, &mut b);
            check_bigint(c, "2");
            assert!(!rem);
        }
    }

    // test/unit_tests.c:580-588 `ufbxt_bigint_shift_right1`
    unsafe fn bigint_shift_right1(b: *mut Bigint) {
        let length = (*b).length;
        *(*b).limbs.add(length as usize) = 0;
        for i in 0..length {
            *(*b).limbs.add(i as usize) = (*(*b).limbs.add(i as usize) >> 1)
                | (*(*b).limbs.add((i + 1) as usize) << (BIGINT_LIMB_BITS - 1));
            if *(*b).limbs.add(i as usize) != 0 {
                (*b).length = i + 1;
            }
        }
    }

    // test/unit_tests.c:590-621 `ufbxt_check_bigint_div`
    unsafe fn check_bigint_div(dividend: &str, divisor: &str, quotient: &str, remainder: bool) {
        let mut a_limbs = [0 as BigintLimb; 64];
        let mut b_limbs = [0 as BigintLimb; 64];
        let mut c_limbs = [0 as BigintLimb; 64];
        let mut a = bigint_array!(a_limbs);
        let mut b = bigint_array!(b_limbs);
        let mut c = bigint_array!(c_limbs);

        bigint_parse(&mut a, dividend);
        bigint_parse(&mut b, divisor);

        assert!(b.length >= 1);
        let mut shift = lzcnt64(*b.limbs.add((b.length - 1) as usize) as u64) - 32;
        if b.length == 1 {
            shift += BIGINT_LIMB_BITS;
        }

        bigint_shift_left(&mut a, shift);
        bigint_shift_left(&mut b, shift);

        let mut a_shifted = false;
        if a.length != 0 && *a.limbs.add((a.length - 1) as usize) >> (BIGINT_LIMB_BITS - 1) != 0 {
            bigint_shift_left(&mut a, 1);
            a_shifted = true;
        }

        let rem = bigint_div(&mut c, &mut a, &mut b);

        if a_shifted {
            bigint_shift_right1(&mut c);
        }

        check_bigint(c, quotient);
        assert!(rem == remainder);
    }

    // test/unit_tests.c:623-647 `test_bigint_div`
    #[test]
    fn test_bigint_div() {
        unsafe {
            check_bigint_div("123", "10", "12", true);
            check_bigint_div("120", "10", "12", false);
            check_bigint_div("0xdeadbeef", "0x10000", "0xdead", true);
            check_bigint_div("82718061255302767487140869206996285356581211090087890625", "9094947017729282379150390625", "9094947017729282379150390625", false);
            check_bigint_div("82718061255302767487140869206996285356581211090087890625", "9094947017729282379150390624", "9094947017729282379150390626", true);
            check_bigint_div("82718061255302767487140869206996285356581211090087890625", "9094947017729282379150390626", "9094947017729282379150390624", true);
            check_bigint_div(
                "9173994463960286046443283581092555673948943761249553509941667449694421519688219440078102364094996072628225",
                "6277101735386680763495507056286727952620534092958556749825",
                "1461501637330902918282912995212100613184356876287", true);
            check_bigint_div(
                "0xffffffffffffffffffffffff00000000",
                "0x8000000000000000ffffffff",
                "0x1ffffffff", true);
            check_bigint_div(
                "0x7fffffff000000010000000000000000",
                "0x80000000fffffffefffffffe",
                "0xfffffffc", true);
            check_bigint_div(
                "0x7fffffff000000010000000000000000",
                "0x8000000000000001fffffffe",
                "0xfffffffd", true);
        }
    }

    // Independent re-implementation of C `strtod`'s longest-valid-prefix rule
    // for the decimal grammar (the check_float corpus is decimal-only; inf/nan
    // go through check_inf/check_nan which use no reference). Returns the byte
    // length of the subject sequence: optional whitespace and sign, digits with
    // at most one '.', and an exponent part only if `e`/`E` [sign] is followed
    // by at least one digit (otherwise strtod backs up to before the 'e').
    fn strtod_prefix_len(s: &str) -> usize {
        let b = s.as_bytes();
        let mut i = 0;
        while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
            i += 1;
        }
        let ws = i;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let mut digits = 0usize;
        let mut seen_dot = false;
        while i < b.len() {
            match b[i] {
                b'0'..=b'9' => digits += 1,
                b'.' if !seen_dot => seen_dot = true,
                _ => break,
            }
            i += 1;
        }
        if digits == 0 {
            // No conversion performed: strtod sets end == str.
            return 0;
        }
        // Whitespace-only skip with no conversion cannot happen past this point.
        let _ = ws;
        if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
            let mut j = i + 1;
            if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                j += 1;
            }
            let exp_digits_start = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > exp_digits_start {
                i = j;
            }
        }
        i
    }

    // test/unit_tests.c:649-679 `ufbxt_check_float`
    // C reference oracle: `strtod(str, NULL)` / `strtof(str, NULL)` — computed
    // independently of ufbxi_parse_double's end pointer. The Rust reference is
    // `str::parse` (also correctly rounded) applied to the longest valid prefix
    // determined by the independent `strtod_prefix_len` scanner above, so an
    // end-pointer bug in `parse_double` cannot shift the reference with it.
    unsafe fn check_float(s: &str) {
        let b = s.as_bytes();
        let prefix = s[..strtod_prefix_len(s)].trim_start();
        // strtod/strtof return 0.0 when no conversion could be performed.
        let ref_d: f64 = if prefix.is_empty() { 0.0 } else { prefix.parse().unwrap() };
        let ref_f: f32 = if prefix.is_empty() { 0.0 } else { prefix.parse().unwrap() };
        let mut end: *const u8 = core::ptr::null();
        let slow_d = parse_double(b.as_ptr(), b.len(), &mut end, 0);
        let fast_d = parse_double(b.as_ptr(), b.len(), &mut end, PARSE_DOUBLE_ALLOW_FAST_PATH);
        let slow_f = parse_double(b.as_ptr(), b.len(), &mut end, PARSE_DOUBLE_AS_BINARY32) as f32;

        if ref_d.is_finite() {
            assert!(
                slow_d == ref_d,
                "strtod() mismatch (slow): '{}': reference {:e}, ufbxc {:e}",
                s, ref_d, slow_d
            );
            assert!(
                fast_d == ref_d,
                "strtod() mismatch (fast): '{}': reference {:e}, ufbxc {:e}",
                s, ref_d, fast_d
            );
        } else {
            assert!(!fast_d.is_finite());
            assert!(!slow_f.is_finite());
        }
        if ref_f.is_finite() {
            assert!(
                slow_f == ref_f,
                "strtof() mismatch (slow): '{}': reference {:e}, ufbxc {:e}",
                s, ref_f, slow_f
            );
        } else {
            assert!(!slow_f.is_finite());
        }
    }

    // test/unit_tests.c:681-691 `ufbxt_check_nan`
    unsafe fn check_nan(s: &str) {
        let b = s.as_bytes();
        let mut end_d: *const u8 = core::ptr::null();
        let mut end_f: *const u8 = core::ptr::null();
        let slow_d = parse_double(b.as_ptr(), b.len(), &mut end_d, 0);
        let slow_f = parse_double(b.as_ptr(), b.len(), &mut end_f, PARSE_DOUBLE_AS_BINARY32) as f32;
        assert!(slow_d.is_nan());
        assert!(slow_f.is_nan());
        assert!(end_d == b.as_ptr().add(b.len()));
        assert!(end_f == b.as_ptr().add(b.len()));
    }

    // test/unit_tests.c:692-705 `ufbxt_check_inf`
    unsafe fn check_inf(s: &str, sign: i32) {
        let b = s.as_bytes();
        let mut end_d: *const u8 = core::ptr::null();
        let mut end_f: *const u8 = core::ptr::null();
        let slow_d = parse_double(b.as_ptr(), b.len(), &mut end_d, 0);
        let slow_f = parse_double(b.as_ptr(), b.len(), &mut end_f, PARSE_DOUBLE_AS_BINARY32) as f32;
        assert!(slow_d.is_infinite());
        assert!(slow_f.is_infinite());
        // C: `slow_d < 0 == sign < 0`
        assert!((slow_d < 0.0) == (sign < 0));
        assert!((slow_f < 0.0) == (sign < 0));
        assert!(slow_f.is_infinite());
        assert!(end_d == b.as_ptr().add(b.len()));
        assert!(end_f == b.as_ptr().add(b.len()));
    }

    // test/unit_tests.c:706-712 `TEST_NINES` (5 x 40 nines) / 713-718 `TEST_ZEROS`
    fn test_nines() -> String { "9".repeat(200) }
    fn test_zeros() -> String { "0".repeat(200) }

    // test/unit_tests.c:720-762 `test_double_parse`
    #[test]
    fn test_double_parse() {
        let nines = test_nines();
        let zeros = test_zeros();
        unsafe {
            check_float("1");
            check_float("123.0");
            check_float("123.456");
            check_float("123e-6");
            check_float("-1.5");
            check_float("1112223333444.555666777888999");
            check_float("0");
            check_float("-0");
            check_float(".5");
            check_float("-.5");
            check_float("1e100");
            check_float("1e1000");
            check_float("1e-1000");
            check_float("1e10000");
            check_float("1e-10000");
            check_float("7.67844768714563e-239");
            check_float(&nines);
            check_float(&format!("{nines}.999999999999999999999999999999999999999"));
            check_float(&format!("{nines}e+108"));
            check_float(&format!("{nines}e+109"));
            check_float(&format!("{nines}e+118"));
            check_float(&format!("{nines}e+119"));
            check_float(&format!("{nines}e+120"));
            check_float(&format!("{nines}e+300"));
            check_float(&format!("{nines}e-200"));
            check_float(&format!("{nines}e-400"));
            check_float(&format!("{nines}e-520"));
            check_float(&format!("{nines}e-523"));
            check_float(&format!("{nines}e-524"));
            check_float(&format!("{zeros}123"));
            check_float(&format!("{zeros}.123"));
            check_float(&format!("{zeros}.{zeros}123"));
            check_float(&format!("{zeros}.{zeros}123{zeros}"));
            check_float(&format!("{zeros}.{zeros}{zeros}123"));
            check_float("241309881603643e20");
            check_float(".5.57999999993498");
            check_float("-71862.4328795732984723456847839347829321867347892347893274982374982349872136217381623872E-273");
            // C: `#if !defined(_MSC_VER)` — excluded there only because MSVC's
            // reference strtod() rounds it incorrectly; the Rust reference is
            // correctly rounded, so always included.
            check_float("4656612873077392578125e-8");
        }
    }

    // test/unit_tests.c:764-787 `test_double_parse_nan`
    #[test]
    fn test_double_parse_nan() {
        unsafe {
            check_nan("nan");
            check_nan("NAN");
            check_nan("NaN");
            check_nan("-nan");
            check_nan("+nan");
            check_nan("nan(1234)");
            check_nan("nan(0x123456789abcdef)");
            check_nan("nan(nan)");
            check_nan("nan(ind)");
            check_nan("nan(nans)");
            check_inf("inf", 1);
            check_inf("-inf", -1);
            check_inf("INF", 1);
            check_inf("INFINITY", 1);

            check_nan("1.#NAN");
            check_nan("0.#NAN12345678");
            check_nan("1.#IND");
            check_nan("-7.#NAN00");
            check_inf("1.#INF", 1);
            check_inf("-1.#INF", -1);
        }
    }

    // ufbx.c:1795-1805 — every Rust target evaluates f64 at native precision
    // (FLT_EVAL_METHOD == 0) with IEEE round-to-nearest, so the probe must
    // select the fast path.
    #[test]
    fn test_parse_double_init_flags() {
        assert_eq!(parse_double_init_flags(), PARSE_DOUBLE_ALLOW_FAST_PATH);
    }

    // test/unit_tests.c:789-815 `test_double_parse_fmt`
    // C formats with `snprintf(fmt = "%.*f" / "%.*e")`; Rust `{:.*}` / `{:.*e}`
    // produce the same correctly-rounded decimal digits (the `%e` exponent
    // spelling differs — "e8" vs "e+08" — which the parser accepts equally).
    // C formats into `char buffer[128]`, so `%.*f` output for large-magnitude
    // doubles (up to ~320 chars near 1e308) is snprintf-truncated to 127 chars
    // before being checked — mirror that so the sweep feeds identical inputs
    // (`%.*e` output at width <= 12 never reaches the limit).
    fn snprintf_truncate(mut s: String) -> String {
        s.truncate(127);
        s
    }

    unsafe fn test_double_parse_fmt(exp_fmt: bool, width: usize, bits: u32) {
        let max_hi: u32 = 1 << bits;
        for hi in 0..max_hi {
            for delta in -2i32..=2 {
                // C: `(hi << (32u - bits)) + (uint32_t)delta` — unsigned wrap.
                let bits_f: u32 = (hi << (32 - bits)).wrapping_add(delta as u32);
                // C: `((uint64_t)hi << (64u - bits)) + (uint64_t)(int64_t)delta`
                let bits_d: u64 = ((hi as u64) << (64 - bits)).wrapping_add(delta as i64 as u64);

                let val_f = f32::from_bits(bits_f);
                let val_d = f64::from_bits(bits_d);

                if val_f.is_finite() {
                    // C passes the float vararg promoted to double.
                    let s = snprintf_truncate(if exp_fmt {
                        format!("{:.*e}", width, val_f as f64)
                    } else {
                        format!("{:.*}", width, val_f as f64)
                    });
                    check_float(&s);
                }
                if val_d.is_finite() {
                    let s = snprintf_truncate(if exp_fmt {
                        format!("{:.*e}", width, val_d)
                    } else {
                        format!("{:.*}", width, val_d)
                    });
                    check_float(&s);
                }
            }
        }
    }

    // test/unit_tests.c:817-828 `test_double_parse_bits`
    // Expensive sweep (~3.7M parses) — ignored in debug builds only; run via
    // `cargo test --release` (or `cargo test -- --ignored`).
    #[test]
    #[cfg_attr(debug_assertions, ignore = "expensive sweep; run with `cargo test --release`")]
    fn test_double_parse_bits() {
        let bits: u32 = 12;
        let max_width: usize = 12;

        unsafe {
            for width in 4..=max_width {
                test_double_parse_fmt(false, width, bits); // "%.*f"
            }
            for width in 4..=max_width {
                test_double_parse_fmt(true, width, bits); // "%.*e"
            }
        }
    }

    // test/unit_tests.c:286-294 `bigdecimal`
    const BIGDECIMAL_DIGITS: usize = 1024;
    const BIGDECIMAL_SUFFIX: usize = 32;

    #[derive(Clone, Copy)]
    struct Bigdecimal {
        // Little-endian digit chars ending at index BIGDECIMAL_DIGITS; the
        // exponent suffix (and its NUL) lives at BIGDECIMAL_DIGITS + 1 onward.
        digits: [u8; BIGDECIMAL_DIGITS + 2 + BIGDECIMAL_SUFFIX],
        length: usize,
    }

    // test/unit_tests.c:296-302 `bigdecimal_init` (out-param in C; returned here)
    fn bigdecimal_init(initial: i32) -> Bigdecimal {
        assert!(initial >= 0 && initial <= 9);
        let mut d = Bigdecimal { digits: [0u8; BIGDECIMAL_DIGITS + 2 + BIGDECIMAL_SUFFIX], length: 1 };
        d.digits[BIGDECIMAL_DIGITS + 1] = 0; // '\0'
        d.digits[BIGDECIMAL_DIGITS] = b'0' + initial as u8;
        d
    }

    // test/unit_tests.c:304-310 `bigdecimal_suffixf` (vsnprintf in C; the only
    // format used is "e%+d", pre-formatted by the caller here)
    fn bigdecimal_suffix(d: &mut Bigdecimal, s: &str) {
        assert!(s.len() < BIGDECIMAL_SUFFIX);
        let start = BIGDECIMAL_DIGITS + 1;
        d.digits[start..start + s.len()].copy_from_slice(s.as_bytes());
        d.digits[start + s.len()] = 0;
    }

    // test/unit_tests.c:312-320 `bigdecimal_string` (returns an interior
    // pointer in C; an owned copy of the same bytes here)
    fn bigdecimal_string(d: &Bigdecimal) -> String {
        let nul = BIGDECIMAL_DIGITS + 1
            + d.digits[BIGDECIMAL_DIGITS + 1..].iter().position(|&c| c == 0).unwrap();
        for i in (1..=d.length).rev() {
            let idx = BIGDECIMAL_DIGITS - i + 1;
            if d.digits[idx] != b'0' {
                return String::from_utf8(d.digits[idx..nul].to_vec()).unwrap();
            }
        }
        String::new()
    }

    // test/unit_tests.c:322-335 `bigdecimal_mul`
    fn bigdecimal_mul(d: &mut Bigdecimal, multiplicand: i32) {
        let mut carry: i32 = 0;
        for i in 0..d.length {
            let digit = (d.digits[BIGDECIMAL_DIGITS - i] - b'0') as i32;
            let product = digit * multiplicand + carry;
            d.digits[BIGDECIMAL_DIGITS - i] = ((product % 10) + b'0' as i32) as u8;
            carry = product / 10;
        }
        if carry != 0 {
            assert!(d.length < BIGDECIMAL_DIGITS);
            // C: `d->digits[BIGDECIMAL_DIGITS - d->length++] = ...` — decomposed.
            d.digits[BIGDECIMAL_DIGITS - d.length] = (carry + b'0' as i32) as u8;
            d.length += 1;
        }
    }

    // test/unit_tests.c:337-361 `bigdecimal_add`
    fn bigdecimal_add(d: &mut Bigdecimal, addend: i32) {
        let mut carry: i32 = addend;
        for i in 0..d.length {
            let digit = (d.digits[BIGDECIMAL_DIGITS - i] - b'0') as i32;
            let sum = digit + carry;
            if sum >= 0 && sum < 10 {
                d.digits[BIGDECIMAL_DIGITS - i] = (sum + b'0' as i32) as u8;
                carry = 0;
                break;
            } else if sum < 0 {
                d.digits[BIGDECIMAL_DIGITS - i] = ((sum + 10) + b'0' as i32) as u8;
                carry = -1;
            } else {
                // C: `else if (sum >= 10)` — the only remaining case.
                d.digits[BIGDECIMAL_DIGITS - i] = ((sum % 10) + b'0' as i32) as u8;
                carry = 1;
            }
        }
        if carry > 0 {
            assert!(d.length < BIGDECIMAL_DIGITS);
            d.digits[BIGDECIMAL_DIGITS - d.length] = (carry + b'0' as i32) as u8;
            d.length += 1;
        } else if carry < 0 {
            panic!("Negative bigdecimal");
        }
    }

    // test/unit_tests.c:830-881 `test_double_parse_decimal`
    // Expensive sweep (~18M parse checks, like the C original; ~8s release,
    // minutes in debug) — ignored in debug builds only; run via
    // `cargo test --release` (or `cargo test -- --ignored`).
    #[test]
    #[cfg_attr(debug_assertions, ignore = "expensive sweep; run with `cargo test --release`")]
    fn test_double_parse_decimal() {
        let max_pow2 = 128;
        let max_pow5 = 128;
        let min_exp = -32;
        let max_exp = 32;
        let max_delta = 8;

        // C: `max_decimals` is SIZE_MAX except under MSVC (buggy reference
        // strtod for >19 decimals); the Rust reference is correctly rounded,
        // so no limit applies.

        let mut pow2 = bigdecimal_init(1);
        for _p2 in 0..max_pow2 {
            // C: `memcpy(&pow5, &pow2, sizeof(bigdecimal));`
            let mut pow5 = pow2;

            for _p5 in 0..max_pow5 {
                if pow5.length >= 2 {
                    bigdecimal_add(&mut pow5, -max_delta);
                    for _d in -max_delta..=max_delta {
                        for exp in min_exp..=max_exp {
                            bigdecimal_suffix(&mut pow5, &format!("e{:+}", exp));
                            let s = bigdecimal_string(&pow5);
                            unsafe { check_float(&s); }
                        }
                        bigdecimal_add(&mut pow5, 1);
                    }
                    bigdecimal_add(&mut pow5, -max_delta - 1);
                } else {
                    for exp in min_exp..=max_exp {
                        bigdecimal_suffix(&mut pow5, &format!("e{:+}", exp));
                        let s = bigdecimal_string(&pow5);
                        unsafe { check_float(&s); }
                    }
                }

                bigdecimal_mul(&mut pow5, 5);
            }

            bigdecimal_mul(&mut pow2, 2);
        }
    }
}
