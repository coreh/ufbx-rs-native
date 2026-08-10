//! 1:1 port of `extra/ufbx_math.c` — the external math implementation the C
//! oracle builds link (`UFBX_EXTERNAL_MATH`, test/hash_scene.c:285;
//! misc/run_tests.py:996 and :1542 add `extra/ufbx_math.c` to the sources).
//!
//! ufbx.c:257-276 declares the math shim surface (`ufbx_sqrt` .. `ufbx_isnan`);
//! `native::platform::math` delegates every one of those to this module, so the
//! port never touches platform libm (PORTING.md "Floats"). Platform libm is
//! *not* bit-identical to this code: measured over 2M random samples the Apple
//! libm disagrees with `ufbx_math.c` on sin 3.9%, cos 4.7%, tan 39.4%,
//! asin 8.7%, acos 17.2%, atan 7.1%, atan2 23.3%, pow 9.3% of inputs (1-3 ULP),
//! which lands directly in the scene-hash oracle.
//!
//! Anchors in this file are `// ufbx_math.c:NNNN-MMMM` (not ufbx.c) since that
//! is the C file being translated.
//!
//! Based on LIBM <https://www.netlib.org/libm/>
//!
//! ```text
//!  ====================================================
//!  Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//!
//!  Developed at SunSoft, a Sun Microsystems, Inc. business.
//!  Permission to use, copy, modify, and distribute this
//!  software is freely granted, provided that this notice
//!  is preserved.
//!  ====================================================
//! ```

// `x - x`, `(x - x) / (x - x)` etc. are verbatim fdlibm idioms for generating a
// signed zero or a NaN (with the correct exception behaviour); the equal operands
// are intentional, so clippy::eq_op is allowed for this ported-math module only.
#![allow(clippy::eq_op)]

// C-parity: `ufbxm_int` = `int` = i32, `ufbxm_uint` = `unsigned` = u32
// (ufbx_math.c:69-70). All the bit twiddling below is written against those
// exact widths; signed wraparound (UB in C, two's-complement in practice on the
// oracle targets) is spelled `wrapping_*` per PORTING.md "Integer semantics".

/// ufbx_math.c:71-83 `ufbxm_bits` — the little-endian member order (`lo` then
/// `hi`); the big-endian arm of the `#if` is a layout-only difference that
/// collapses away here because the split is done arithmetically on `f64::to_bits`
/// rather than through a union.
#[derive(Clone, Copy)]
struct Bits {
    lo: u32,
    hi: i32,
}

// -- double/bits conversion

// ufbx_math.c:95-100 `ufbxm_to_bits`
#[inline(always)]
fn to_bits(x: f64) -> Bits {
    let b = x.to_bits();
    Bits {
        lo: b as u32,
        hi: (b >> 32) as u32 as i32,
    }
}

// ufbx_math.c:102-108 `ufbxm_from_bits`
#[inline(always)]
fn from_bits(hi: i32, lo: u32) -> f64 {
    f64::from_bits(((hi as u32 as u64) << 32) | lo as u64)
}

// ufbx_math.c:110-115 `ufbxm_hi`
#[inline(always)]
fn hi(x: f64) -> i32 {
    (x.to_bits() >> 32) as u32 as i32
}

// ufbx_math.c:119-125 `ufbxm_zero_lo`
#[inline(always)]
fn zero_lo(x: f64) -> f64 {
    f64::from_bits(x.to_bits() & 0xffff_ffff_0000_0000)
}

// ufbx_math.c:127-133 `ufbxm_set_hi`
#[inline(always)]
fn set_hi(x: f64, hi: i32) -> f64 {
    f64::from_bits(((hi as u32 as u64) << 32) | (x.to_bits() & 0xffff_ffff))
}

// ufbx_math.c:148-153 `ufbx_copysign`
pub(crate) fn copysign(x: f64, y: f64) -> f64 {
    let xb = to_bits(x);
    let yb = to_bits(y);
    from_bits((xb.hi & 0x7fffffff) | (yb.hi & 0x80000000u32 as i32), xb.lo)
}

// ufbx_math.c:155-163 `ufbx_fabs`
// C-parity: the `#if defined(ufbxm_fabs)` arm (SSE `andnot` / NEON `vabs_f64`,
// ufbx_math.c:38-46) is what both oracle targets compile; `f64::abs` lowers to
// the same instruction and is bit-identical to the portable `hi & 0x7fffffff`
// fallback for every input including NaN payloads.
pub(crate) fn fabs(x: f64) -> f64 {
    x.abs()
}

// ufbx_math.c:165-208 `ufbx_scalbn`
pub(crate) fn scalbn(x: f64, n: i32) -> f64 {
    const TWO54: f64 = 1.80143985094819840000e+16; /* 0x43500000, 0x00000000 */
    const TWOM54: f64 = 5.55111512312578270212e-17; /* 0x3C900000, 0x00000000 */
    const HUGE: f64 = 1.0e+300;
    const TINY: f64 = 1.0e-300;

    let mut x = x;
    let mut bx = to_bits(x);
    let mut hx: i32 = bx.hi;
    let lx: i32 = bx.lo as i32;
    let mut k: i32 = (hx & 0x7ff00000) >> 20; /* extract exponent */
    if k == 0 {
        /* 0 or subnormal x */
        if (lx | (hx & 0x7fffffff)) == 0 {
            return x; /* +-0 */
        }
        x *= TWO54;
        bx = to_bits(x);
        hx = bx.hi;
        k = ((hx & 0x7ff00000) >> 20) - 54;
        if n < -50000 {
            return TINY * x; /*underflow*/
        }
    }
    if k == 0x7ff {
        return x + x; /* NaN or Inf */
    }
    k = k.wrapping_add(n);
    if k > 0x7fe {
        return HUGE * copysign(HUGE, x); /* overflow  */
    }
    if k > 0
    /* normal result */
    {
        hx = (hx & 0x800fffffu32 as i32) | (k << 20);
        return from_bits(hx, bx.lo);
    }
    if k <= -54 {
        if n > 50000 {
            /* in case integer overflow in n+k */
            return HUGE * copysign(HUGE, x); /*overflow*/
        } else {
            return TINY * copysign(TINY, x); /*underflow*/
        }
    }
    k += 54; /* subnormal result */
    hx = (hx & 0x800fffffu32 as i32) | (k << 20);
    from_bits(hx, bx.lo) * TWOM54
}

// ufbx_math.c:210-269 `ufbx_floor`
pub(crate) fn floor(x: f64) -> f64 {
    const HUGE: f64 = 1.0e300;

    let xb = to_bits(x);
    let mut i0: i32 = xb.hi;
    let mut i1: i32 = xb.lo as i32;
    let j0: i32 = ((i0 >> 20) & 0x7ff) - 0x3ff;
    if j0 < 20 {
        if j0 < 0 {
            /* raise inexact if x != 0 */
            if HUGE + x > 0.0 {
                /* return 0*sign(x) if |x|<1 */
                if i0 >= 0 {
                    i0 = 0;
                    i1 = 0;
                } else if ((i0 & 0x7fffffff) | i1) != 0 {
                    i0 = 0xbff00000u32 as i32;
                    i1 = 0;
                }
            }
        } else {
            let i: u32 = 0x000fffffu32 >> j0;
            if ((i0 & i as i32) | i1) == 0 {
                return x; /* x is integral */
            }
            if HUGE + x > 0.0 {
                /* raise inexact flag */
                if i0 < 0 {
                    i0 = i0.wrapping_add(0x00100000 >> j0);
                }
                i0 &= !(i as i32);
                i1 = 0;
            }
        }
    } else if j0 > 51 {
        if j0 == 0x400 {
            return x + x; /* inf or NaN */
        } else {
            return x; /* x is integral */
        }
    } else {
        let i: u32 = 0xffffffffu32 >> (j0 - 20);
        if (i1 & i as i32) == 0 {
            return x; /* x is integral */
        }
        if HUGE + x > 0.0 {
            /* raise inexact flag */
            if i0 < 0 {
                if j0 == 20 {
                    i0 = i0.wrapping_add(1);
                } else {
                    let j: u32 = (i1 as u32).wrapping_add(1u32 << (52 - j0));
                    if j < i1 as u32 {
                        i0 = i0.wrapping_add(1); /* got a carry */
                    }
                    i1 = j as i32;
                }
            }
            i1 &= !(i as i32);
        }
    }
    from_bits(i0, i1 as u32)
}

// ufbx_math.c:271-296 `ufbx_frexp`
// C-parity: declared `ufbx_math_abi` (ufbx_math.c:142) but not part of the
// ufbx.c:257-276 shim surface and unused inside ufbx_math.c itself; ported for
// completeness of the unit so upstream syncs keep applying.
#[allow(dead_code)]
pub(crate) fn frexp(x: f64, eptr: &mut i32) -> f64 {
    const TWO54: f64 = 1.80143985094819840000e+16; /* 0x43500000, 0x00000000 */

    let mut x = x;
    let mut bx = to_bits(x);
    let mut hx: i32 = bx.hi;
    let lx: i32 = bx.lo as i32;
    let mut ix: i32 = 0x7fffffff & hx;
    *eptr = 0;
    if ix >= 0x7ff00000 || ((ix | lx) == 0) {
        return x; /* 0,inf,nan */
    }
    if ix < 0x00100000 {
        /* subnormal */
        x *= TWO54;
        bx = to_bits(x);
        hx = bx.hi;
        ix = hx & 0x7fffffff;
        *eptr = -54;
    }
    *eptr += (ix >> 20) - 1022;
    hx = (hx & 0x800fffffu32 as i32) | 0x3fe00000;
    from_bits(hx, bx.lo)
}

// ufbx_math.c:298-414 `ufbx_atan`
pub(crate) fn atan(x: f64) -> f64 {
    const ATANHI: [f64; 4] = [
        4.63647609000806093515e-01, /* atan(0.5)hi 0x3FDDAC67, 0x0561BB4F */
        7.85398163397448278999e-01, /* atan(1.0)hi 0x3FE921FB, 0x54442D18 */
        9.82793723247329054082e-01, /* atan(1.5)hi 0x3FEF730B, 0xD281F69B */
        1.57079632679489655800e+00, /* atan(inf)hi 0x3FF921FB, 0x54442D18 */
    ];

    const ATANLO: [f64; 4] = [
        2.26987774529616870924e-17, /* atan(0.5)lo 0x3C7A2B7F, 0x222F65E2 */
        3.06161699786838301793e-17, /* atan(1.0)lo 0x3C81A626, 0x33145C07 */
        1.39033110312309984516e-17, /* atan(1.5)lo 0x3C700788, 0x7AF0CBBD */
        6.12323399573676603587e-17, /* atan(inf)lo 0x3C91A626, 0x33145C07 */
    ];

    const AT: [f64; 11] = [
        3.33333333333329318027e-01,  /* 0x3FD55555, 0x5555550D */
        -1.99999999998764832476e-01, /* 0xBFC99999, 0x9998EBC4 */
        1.42857142725034663711e-01,  /* 0x3FC24924, 0x920083FF */
        -1.11111104054623557880e-01, /* 0xBFBC71C6, 0xFE231671 */
        9.09088713343650656196e-02,  /* 0x3FB745CD, 0xC54C206E */
        -7.69187620504482999495e-02, /* 0xBFB3B0F2, 0xAF749A6D */
        6.66107313738753120669e-02,  /* 0x3FB10D66, 0xA0D03D51 */
        -5.83357013379057348645e-02, /* 0xBFADDE2D, 0x52DEFD9A */
        4.97687799461593236017e-02,  /* 0x3FA97B4B, 0x24760DEB */
        -3.65315727442169155270e-02, /* 0xBFA2B444, 0x2C6A6C2F */
        1.62858201153657823623e-02,  /* 0x3F90AD3A, 0xE322DA11 */
    ];

    const ONE: f64 = 1.0;
    const HUGE: f64 = 1.0e300;

    let mut x = x;
    let (w, s1, s2, mut z): (f64, f64, f64, f64);
    let id: i32;

    let bx = to_bits(x);
    let hx: i32 = bx.hi;
    let ix: i32 = hx & 0x7fffffff;
    if ix >= 0x44100000 {
        /* if |x| >= 2^66 */
        if ix > 0x7ff00000 || (ix == 0x7ff00000 && (bx.lo != 0)) {
            return x + x; /* NaN */
        }
        if hx > 0 {
            return ATANHI[3] + ATANLO[3];
        } else {
            return -ATANHI[3] - ATANLO[3];
        }
    }
    if ix < 0x3fdc0000 {
        /* |x| < 0.4375 */
        if ix < 0x3e200000 {
            /* |x| < 2^-29 */
            if HUGE + x > ONE {
                return x; /* raise inexact */
            }
        }
        id = -1;
    } else {
        x = fabs(x);
        if ix < 0x3ff30000 {
            /* |x| < 1.1875 */
            if ix < 0x3fe60000 {
                /* 7/16 <=|x|<11/16 */
                id = 0;
                x = (2.0 * x - ONE) / (2.0 + x);
            } else {
                /* 11/16<=|x|< 19/16 */
                id = 1;
                x = (x - ONE) / (x + ONE);
            }
        } else if ix < 0x40038000 {
            /* |x| < 2.4375 */
            id = 2;
            x = (x - 1.5) / (ONE + 1.5 * x);
        } else {
            /* 2.4375 <= |x| < 2^66 */
            id = 3;
            x = -1.0 / x;
        }
    }
    /* end of argument reduction */
    z = x * x;
    w = z * z;
    /* break sum from i=0 to 10 aT[i]z**(i+1) into odd and even poly */
    s1 = z * (AT[0] + w * (AT[2] + w * (AT[4] + w * (AT[6] + w * (AT[8] + w * AT[10])))));
    s2 = w * (AT[1] + w * (AT[3] + w * (AT[5] + w * (AT[7] + w * AT[9]))));
    if id < 0 {
        x - x * (s1 + s2)
    } else {
        z = ATANHI[id as usize] - ((x * (s1 + s2) - ATANLO[id as usize]) - x);
        if hx < 0 {
            -z
        } else {
            z
        }
    }
}

// ufbx_math.c:416-541 `ufbx_sqrt`
// C-parity: the `#if defined(ufbxm_sqrt)` arm (SSE `_mm_sqrt_sd` / NEON
// `vsqrt_f64`, ufbx_math.c:38-46) is what both oracle targets compile.
// `f64::sqrt` lowers to the same instruction; IEEE-754 square root is correctly
// rounded, so it is also bit-identical to the ported software fallback.
pub(crate) fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

/*
 * Table of constants for 2/pi, 396 Hex digits (476 decimal) of 2/pi
 */
// ufbx_math.c:543-612 `ufbxm_two_over_pi`
static TWO_OVER_PI: [i32; 66] = [
    0xA2F983, 0x6E4E44, 0x1529FC, 0x2757D1, 0xF534DD, 0xC0DB62, 0x95993C, 0x439041, 0xFE5163,
    0xABDEBB, 0xC561B7, 0x246E3A, 0x424DD2, 0xE00649, 0x2EEA09, 0xD1921C, 0xFE1DEB, 0x1CB129,
    0xA73EE8, 0x8235F5, 0x2EBB44, 0x84E99C, 0x7026B4, 0x5F7E41, 0x3991D6, 0x398353, 0x39F49C,
    0x845F8B, 0xBDF928, 0x3B1FF8, 0x97FFDE, 0x05980F, 0xEF2F11, 0x8B5A0A, 0x6D1F6D, 0x367ECF,
    0x27CB09, 0xB74F46, 0x3F669E, 0x5FEA2D, 0x7527BA, 0xC7EBE5, 0xF17B3D, 0x0739F7, 0x8A5292,
    0xEA6BFB, 0x5FB11F, 0x8D5D08, 0x560330, 0x46FC7B, 0x6BABF0, 0xCFBC20, 0x9AF436, 0x1DA9E3,
    0x91615E, 0xE61B08, 0x659985, 0x5F14A0, 0x68408D, 0xFFD880, 0x4D7327, 0x310606, 0x1556CA,
    0x73A8C9, 0x60E27B, 0xC08C6B,
];

// ufbx_math.c:614-647 `ufbxm_npio2_hw`
static NPIO2_HW: [i32; 32] = [
    0x3FF921FB, 0x400921FB, 0x4012D97C, 0x401921FB, 0x401F6A7A, 0x4022D97C, 0x4025FDBB, 0x402921FB,
    0x402C463A, 0x402F6A7A, 0x4031475C, 0x4032D97C, 0x40346B9C, 0x4035FDBB, 0x40378FDB, 0x403921FB,
    0x403AB41B, 0x403C463A, 0x403DD85A, 0x403F6A7A, 0x40407E4C, 0x4041475C, 0x4042106C, 0x4042D97C,
    0x4043A28C, 0x40446B9C, 0x404534AC, 0x4045FDBB, 0x4046C6CB, 0x40478FDB, 0x404858EB, 0x404921FB,
];

// ufbx_math.c:649-877 `ufbxm_kernel_rem_pio2`
fn kernel_rem_pio2(x: &[f64], y: &mut [f64], e0: i32, nx: i32, prec: i32, ipio2: &[i32]) -> i32 {
    const INIT_JK: [i32; 4] = [2, 3, 4, 6]; /* initial value for jk */

    const PIO2: [f64; 8] = [
        1.57079625129699707031e+00, /* 0x3FF921FB, 0x40000000 */
        7.54978941586159635335e-08, /* 0x3E74442D, 0x00000000 */
        5.39030252995776476554e-15, /* 0x3CF84698, 0x80000000 */
        3.28200341580791294123e-22, /* 0x3B78CC51, 0x60000000 */
        1.27065575308067607349e-29, /* 0x39F01B83, 0x80000000 */
        1.22933308981111328932e-36, /* 0x387A2520, 0x40000000 */
        2.73370053816464559624e-44, /* 0x36E38222, 0x80000000 */
        2.16741683877804819444e-51, /* 0x3569F31D, 0x00000000 */
    ];

    const ZERO: f64 = 0.0;
    const ONE: f64 = 1.0;
    const TWO24: f64 = 1.67772160000000000000e+07; /* 0x41700000, 0x00000000 */
    const TWON24: f64 = 5.96046447753906250000e-08; /* 0x3E700000, 0x00000000 */

    // C: `ufbxm_int jz, jx, jv, jp, jk, carry, n, iq[20], i, j, k, m, q0, ih;`
    // `double z, fw, f[20], fq[20], q[20];` — declared uninitialized; every
    // element is written before it is read (see the loops below), the zero-fill
    // here is only because Rust has no uninitialized locals.
    let (mut jz, jx, mut jv, jp, jk): (i32, i32, i32, i32, i32);
    let (mut carry, mut n, mut i, mut j, mut k, m, mut q0, mut ih): (
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
    );
    let mut iq: [i32; 20] = [0; 20];
    let (mut z, mut fw): (f64, f64);
    let mut f: [f64; 20] = [0.0; 20];
    let mut fq: [f64; 20] = [0.0; 20];
    let mut q: [f64; 20] = [0.0; 20];

    /* initialize jk*/
    jk = INIT_JK[prec as usize];
    jp = jk;

    /* determine jx,jv,q0, note that 3>q0 */
    jx = nx - 1;
    jv = (e0 - 3) / 24;
    if jv < 0 {
        jv = 0;
    }
    q0 = e0 - 24 * (jv + 1);

    /* set up f[0] to f[jx+jk] where f[jx+jk] = ipio2[jv+jk] */
    j = jv - jx;
    m = jx + jk;
    i = 0;
    while i <= m {
        f[i as usize] = if j < 0 {
            ZERO
        } else {
            ipio2[j as usize] as f64
        };
        i += 1;
        j += 1;
    }

    /* compute q[0],q[1],...q[jk] */
    i = 0;
    while i <= jk {
        j = 0;
        fw = 0.0;
        while j <= jx {
            fw += x[j as usize] * f[(jx + i - j) as usize];
            j += 1;
        }
        q[i as usize] = fw;
        i += 1;
    }

    jz = jk;

    loop {
        /* distill q[] into iq[] reversingly */
        i = 0;
        j = jz;
        z = q[jz as usize];
        while j > 0 {
            fw = (TWON24 * z) as i32 as f64;
            iq[i as usize] = (z - TWO24 * fw) as i32;
            z = q[(j - 1) as usize] + fw;
            i += 1;
            j -= 1;
        }

        /* compute n */
        z = scalbn(z, q0); /* actual value of z */
        z -= 8.0 * floor(z * 0.125); /* trim off integer >= 8 */
        n = z as i32;
        z -= n as f64;
        ih = 0;
        if q0 > 0 {
            /* need iq[jz-1] to determine n */
            i = iq[(jz - 1) as usize] >> (24 - q0);
            n += i;
            iq[(jz - 1) as usize] -= i << (24 - q0);
            ih = iq[(jz - 1) as usize] >> (23 - q0);
        } else if q0 == 0 {
            ih = iq[(jz - 1) as usize] >> 23;
        } else if z >= 0.5 {
            ih = 2;
        }

        if ih > 0 {
            /* q > 0.5 */
            n += 1;
            carry = 0;
            i = 0;
            while i < jz {
                /* compute 1-q */
                j = iq[i as usize];
                if carry == 0 {
                    if j != 0 {
                        carry = 1;
                        iq[i as usize] = 0x1000000 - j;
                    }
                } else {
                    iq[i as usize] = 0xffffff - j;
                }
                i += 1;
            }
            if q0 > 0 {
                /* rare case: chance is 1 in 12 */
                match q0 {
                    1 => {
                        iq[(jz - 1) as usize] &= 0x7fffff;
                    }
                    2 => {
                        iq[(jz - 1) as usize] &= 0x3fffff;
                    }
                    _ => {}
                }
            }
            if ih == 2 {
                z = ONE - z;
                if carry != 0 {
                    z -= scalbn(ONE, q0);
                }
            }
        }

        /* check if recomputation is needed */
        if z == ZERO {
            j = 0;
            i = jz - 1;
            while i >= jk {
                j |= iq[i as usize];
                i -= 1;
            }
            if j == 0 {
                /* need recomputation */
                k = 1;
                while iq[(jk - k) as usize] == 0 {
                    k += 1;
                } /* k = no. of terms needed */

                i = jz + 1;
                while i <= jz + k {
                    /* add q[jz+1] to q[jz+k] */
                    f[(jx + i) as usize] = ipio2[(jv + i) as usize] as f64;
                    j = 0;
                    fw = 0.0;
                    while j <= jx {
                        fw += x[j as usize] * f[(jx + i - j) as usize];
                        j += 1;
                    }
                    q[i as usize] = fw;
                    i += 1;
                }
                jz += k;
                // goto recompute;
                continue;
            }
        }

        break;
    }

    /* chop off zero terms */
    if z == 0.0 {
        jz -= 1;
        q0 -= 24;
        while iq[jz as usize] == 0 {
            jz -= 1;
            q0 -= 24;
        }
    } else {
        /* break z into 24-bit if necessary */
        z = scalbn(z, -q0);
        if z >= TWO24 {
            fw = (TWON24 * z) as i32 as f64;
            iq[jz as usize] = (z - TWO24 * fw) as i32;
            jz += 1;
            q0 += 24;
            iq[jz as usize] = fw as i32;
        } else {
            iq[jz as usize] = z as i32;
        }
    }

    /* convert integer "bit" chunk to floating-point value */
    fw = scalbn(ONE, q0);
    i = jz;
    while i >= 0 {
        q[i as usize] = fw * iq[i as usize] as f64;
        fw *= TWON24;
        i -= 1;
    }

    /* compute PIo2[0,...,jp]*q[jz,...,0] */
    fq[0] = 0.0; /* dumb warning fix */
    i = jz;
    while i >= 0 {
        fw = 0.0;
        k = 0;
        while k <= jp && k <= jz - i {
            fw += PIO2[k as usize] * q[(i + k) as usize];
            k += 1;
        }
        fq[(jz - i) as usize] = fw;
        i -= 1;
    }

    /* compress fq[] into y[] */
    match prec {
        0 => {
            fw = 0.0;
            i = jz;
            while i >= 0 {
                fw += fq[i as usize];
                i -= 1;
            }
            y[0] = if ih == 0 { fw } else { -fw };
        }
        1 | 2 => {
            fw = 0.0;
            i = jz;
            while i >= 0 {
                fw += fq[i as usize];
                i -= 1;
            }
            y[0] = if ih == 0 { fw } else { -fw };
            fw = fq[0] - fw;
            i = 1;
            while i <= jz {
                fw += fq[i as usize];
                i += 1;
            }
            y[1] = if ih == 0 { fw } else { -fw };
        }
        3 => {
            /* painful */
            i = jz;
            while i > 0 {
                fw = fq[(i - 1) as usize] + fq[i as usize];
                fq[i as usize] += fq[(i - 1) as usize] - fw;
                fq[(i - 1) as usize] = fw;
                i -= 1;
            }
            i = jz;
            while i > 1 {
                fw = fq[(i - 1) as usize] + fq[i as usize];
                fq[i as usize] += fq[(i - 1) as usize] - fw;
                fq[(i - 1) as usize] = fw;
                i -= 1;
            }
            fw = 0.0;
            i = jz;
            while i >= 2 {
                fw += fq[i as usize];
                i -= 1;
            }
            if ih == 0 {
                y[0] = fq[0];
                y[1] = fq[1];
                y[2] = fw;
            } else {
                y[0] = -fq[0];
                y[1] = -fq[1];
                y[2] = -fw;
            }
        }
        _ => {}
    }
    n & 7
}

/*
 * invpio2:  53 bits of 2/pi
 * pio2_1:   first  33 bit of pi/2
 * pio2_1t:  pi/2 - pio2_1
 * pio2_2:   second 33 bit of pi/2
 * pio2_2t:  pi/2 - (pio2_1+pio2_2)
 * pio2_3:   third  33 bit of pi/2
 * pio2_3t:  pi/2 - (pio2_1+pio2_2+pio2_3)
 */

// ufbx_math.c:889-1026 `ufbxm_rem_pio2`
fn rem_pio2(x: f64, y: &mut [f64; 2]) -> i32 {
    const ZERO: f64 = 0.00000000000000000000e+00; /* 0x00000000, 0x00000000 */
    const HALF: f64 = 5.00000000000000000000e-01; /* 0x3FE00000, 0x00000000 */
    const TWO24: f64 = 1.67772160000000000000e+07; /* 0x41700000, 0x00000000 */
    const INVPIO2: f64 = 6.36619772367581382433e-01; /* 0x3FE45F30, 0x6DC9C883 */
    const PIO2_1: f64 = 1.57079632673412561417e+00; /* 0x3FF921FB, 0x54400000 */
    const PIO2_1T: f64 = 6.07710050650619224932e-11; /* 0x3DD0B461, 0x1A626331 */
    const PIO2_2: f64 = 6.07710050630396597660e-11; /* 0x3DD0B461, 0x1A600000 */
    const PIO2_2T: f64 = 2.02226624879595063154e-21; /* 0x3BA3198A, 0x2E037073 */
    const PIO2_3: f64 = 2.02226624871116645580e-21; /* 0x3BA3198A, 0x2E000000 */
    const PIO2_3T: f64 = 8.47842766036889956997e-32; /* 0x397B839A, 0x252049C1 */

    let mut z: f64;
    let mut w: f64;
    let mut t: f64;
    let mut r: f64;
    let fn_: f64;
    let mut tx: [f64; 3] = [0.0; 3];

    let mut i: i32;
    let j: i32;
    let mut nx: i32;
    let n: i32;

    let bx = to_bits(x);
    let hx: i32 = bx.hi; /* high word of x */
    let ix: i32 = hx & 0x7fffffff;
    if ix <= 0x3fe921fb
    /* |x| ~<= pi/4 , no need for reduction */
    {
        y[0] = x;
        y[1] = 0.0;
        return 0;
    }
    if ix < 0x4002d97c {
        /* |x| < 3pi/4, special case with n=+-1 */
        if hx > 0 {
            z = x - PIO2_1;
            if ix != 0x3ff921fb {
                /* 33+53 bit pi is good enough */
                y[0] = z - PIO2_1T;
                y[1] = (z - y[0]) - PIO2_1T;
            } else {
                /* near pi/2, use 33+33+53 bit pi */
                z -= PIO2_2;
                y[0] = z - PIO2_2T;
                y[1] = (z - y[0]) - PIO2_2T;
            }
            return 1;
        } else {
            /* negative x */
            z = x + PIO2_1;
            if ix != 0x3ff921fb {
                /* 33+53 bit pi is good enough */
                y[0] = z + PIO2_1T;
                y[1] = (z - y[0]) + PIO2_1T;
            } else {
                /* near pi/2, use 33+33+53 bit pi */
                z += PIO2_2;
                y[0] = z + PIO2_2T;
                y[1] = (z - y[0]) + PIO2_2T;
            }
            return -1;
        }
    }
    if ix <= 0x413921fb {
        /* |x| ~<= 2^19*(pi/2), medium size */
        t = fabs(x);
        n = (t * INVPIO2 + HALF) as i32;
        fn_ = n as f64;
        r = t - fn_ * PIO2_1;
        w = fn_ * PIO2_1T; /* 1st round good to 85 bit */
        if n < 32 && ix != NPIO2_HW[(n - 1) as usize] {
            y[0] = r - w; /* quick check no cancellation */
        } else {
            j = ix >> 20;
            y[0] = r - w;
            i = j - ((hi(y[0]) >> 20) & 0x7ff);
            if i > 16 {
                /* 2nd iteration needed, good to 118 */
                t = r;
                w = fn_ * PIO2_2;
                r = t - w;
                w = fn_ * PIO2_2T - ((t - r) - w);
                y[0] = r - w;
                i = j - ((hi(y[0]) >> 20) & 0x7ff);
                if i > 49 {
                    /* 3rd iteration need, 151 bits acc */
                    t = r; /* will cover all possible cases */
                    w = fn_ * PIO2_3;
                    r = t - w;
                    w = fn_ * PIO2_3T - ((t - r) - w);
                    y[0] = r - w;
                }
            }
        }
        y[1] = (r - y[0]) - w;
        if hx < 0 {
            y[0] = -y[0];
            y[1] = -y[1];
            return -n;
        } else {
            return n;
        }
    }
    /*
     * all other (large) arguments
     */
    if ix >= 0x7ff00000 {
        /* x is inf or NaN */
        // C: `y[0] = y[1] = x - x;`
        y[1] = x - x;
        y[0] = y[1];
        return 0;
    }
    /* set z = scalbn(|x|,ilogb(x)-23) */
    let e0: i32 = (ix >> 20) - 1046; /* e0 = ilogb(z)-23; */
    z = from_bits(ix - (e0 << 20), bx.lo);
    i = 0;
    while i < 2 {
        tx[i as usize] = z as i32 as f64;
        z = (z - tx[i as usize]) * TWO24;
        i += 1;
    }
    tx[2] = z;
    nx = 3;
    while tx[(nx - 1) as usize] == ZERO {
        nx -= 1; /* skip zero term */
    }
    n = kernel_rem_pio2(&tx, y, e0, nx, 2, &TWO_OVER_PI);
    if hx < 0 {
        y[0] = -y[0];
        y[1] = -y[1];
        return -n;
    }
    n
}

// ufbx_math.c:1028-1055 `ufbxm_kernel_sin`
fn kernel_sin(x: f64, y: f64, iy: i32) -> f64 {
    const HALF: f64 = 5.00000000000000000000e-01; /* 0x3FE00000, 0x00000000 */
    const S1: f64 = -1.66666666666666324348e-01; /* 0xBFC55555, 0x55555549 */
    const S2: f64 = 8.33333333332248946124e-03; /* 0x3F811111, 0x1110F8A6 */
    const S3: f64 = -1.98412698298579493134e-04; /* 0xBF2A01A0, 0x19C161D5 */
    const S4: f64 = 2.75573137070700676789e-06; /* 0x3EC71DE3, 0x57B1FE7D */
    const S5: f64 = -2.50507602534068634195e-08; /* 0xBE5AE5E6, 0x8A2B9CEB */
    const S6: f64 = 1.58969099521155010221e-10; /* 0x3DE5D93A, 0x5ACFD57C */

    let (z, r, v): (f64, f64, f64);
    let ix: i32 = hi(x) & 0x7fffffff; /* high word of x */
    if ix < 0x3e400000
    /* |x| < 2**-27 */
    {
        if x as i32 == 0 {
            return x;
        }
    } /* generate inexact */
    z = x * x;
    v = z * x;
    r = S2 + z * (S3 + z * (S4 + z * (S5 + z * S6)));
    if iy == 0 {
        x + v * (S1 + z * r)
    } else {
        x - ((z * (HALF * y - v * r) - y) - v * S1)
    }
}

// ufbx_math.c:1057-1091 `ufbxm_kernel_cos`
fn kernel_cos(x: f64, y: f64) -> f64 {
    const ONE: f64 = 1.00000000000000000000e+00; /* 0x3FF00000, 0x00000000 */
    const C1: f64 = 4.16666666666666019037e-02; /* 0x3FA55555, 0x5555554C */
    const C2: f64 = -1.38888888888741095749e-03; /* 0xBF56C16C, 0x16C15177 */
    const C3: f64 = 2.48015872894767294178e-05; /* 0x3EFA01A0, 0x19CB1590 */
    const C4: f64 = -2.75573143513906633035e-07; /* 0xBE927E4F, 0x809C52AD */
    const C5: f64 = 2.08757232129817482790e-09; /* 0x3E21EE9E, 0xBDB4B1C4 */
    const C6: f64 = -1.13596475577881948265e-11; /* 0xBDA8FAE9, 0xBE8838D4 */

    let (a, hz, z, r, qx): (f64, f64, f64, f64, f64);
    let ix: i32 = hi(x) & 0x7fffffff; /* ix = |x|'s high word*/
    if ix < 0x3e400000 {
        /* if x < 2**27 */
        if x as i32 == 0 {
            return ONE; /* generate inexact */
        }
    }
    z = x * x;
    r = z * (C1 + z * (C2 + z * (C3 + z * (C4 + z * (C5 + z * C6)))));
    if ix < 0x3FD33333
    /* if |x| < 0.3 */
    {
        ONE - (0.5 * z - (z * r - x * y))
    } else {
        if ix > 0x3fe90000 {
            /* x > 0.78125 */
            qx = 0.28125;
        } else {
            qx = from_bits(ix - 0x00200000, 0); /* x/4 */
        }
        hz = 0.5 * z - qx;
        a = ONE - qx;
        a - (hz - (z * r - x * y))
    }
}

// ufbx_math.c:1093-1211 `ufbxm_kernel_tan`
fn kernel_tan(x: f64, y: f64, iy: i32) -> f64 {
    let (mut x, mut y) = (x, y);
    let (mut z, mut r, mut v, mut w, mut s): (f64, f64, f64, f64, f64);

    const XXX: [f64; 16] = [
        3.33333333333334091986e-01,  /* 3FD55555, 55555563 */
        1.33333333333201242699e-01,  /* 3FC11111, 1110FE7A */
        5.39682539762260521377e-02,  /* 3FABA1BA, 1BB341FE */
        2.18694882948595424599e-02,  /* 3F9664F4, 8406D637 */
        8.86323982359930005737e-03,  /* 3F8226E3, E96E8493 */
        3.59207910759131235356e-03,  /* 3F6D6D22, C9560328 */
        1.45620945432529025516e-03,  /* 3F57DBC8, FEE08315 */
        5.88041240820264096874e-04,  /* 3F4344D8, F2F26501 */
        2.46463134818469906812e-04,  /* 3F3026F7, 1A8D1068 */
        7.81794442939557092300e-05,  /* 3F147E88, A03792A6 */
        7.14072491382608190305e-05,  /* 3F12B80F, 32F0A7E9 */
        -1.85586374855275456654e-05, /* BEF375CB, DB605373 */
        2.59073051863633712884e-05,  /* 3EFB2A70, 74BF7AD4 */
        /* one */ 1.00000000000000000000e+00, /* 3FF00000, 00000000 */
        /* pio4 */ 7.85398163397448278999e-01, /* 3FE921FB, 54442D18 */
        /* pio4lo */ 3.06161699786838301793e-17, /* 3C81A626, 33145C07 */
    ];
    let one: f64 = XXX[13];
    let pio4: f64 = XXX[14];
    let pio4lo: f64 = XXX[15];
    let t_: &[f64; 16] = &XXX;

    let bx = to_bits(x);
    let hx: i32 = bx.hi; /* high word of x */
    let ix: i32 = hx & 0x7fffffff; /* high word of |x| */
    if ix < 0x3e300000 {
        /* x < 2**-28 */
        if x as i32 == 0 {
            /* generate inexact */
            if ((ix | bx.lo as i32) | (iy + 1)) == 0 {
                return one / fabs(x);
            } else if iy == 1 {
                return x;
            } else {
                /* compute -1 / (x+y) carefully */
                let (a, mut t): (f64, f64);

                w = x + y;
                z = w;
                z = zero_lo(z);
                v = y - (z - x);
                a = -one / w;
                t = a;
                t = zero_lo(t);
                s = one + t * z;
                return t + a * (s + t * v);
            }
        }
    }
    if ix >= 0x3FE59428 {
        /* |x| >= 0.6744 */
        if hx < 0 {
            x = -x;
            y = -y;
        }
        z = pio4 - x;
        w = pio4lo - y;
        x = z + w;
        y = 0.0;
    }
    z = x * x;
    w = z * z;
    /*
     * Break x^5*(T[1]+x^2*T[2]+...) into
     * x^5(T[1]+x^4*T[3]+...+x^20*T[11]) +
     * x^5(x^2*(T[2]+x^4*T[4]+...+x^22*[T12]))
     */
    r = t_[1] + w * (t_[3] + w * (t_[5] + w * (t_[7] + w * (t_[9] + w * t_[11]))));
    v = z * (t_[2] + w * (t_[4] + w * (t_[6] + w * (t_[8] + w * (t_[10] + w * t_[12])))));
    s = z * x;
    r = y + z * (s * (r + v) + y);
    r += t_[0] * s;
    w = x + r;
    if ix >= 0x3FE59428 {
        v = iy as f64;
        return (1 - ((hx >> 30) & 2)) as f64 * (v - 2.0 * (x - (w * w / (w + v) - r)));
    }
    if iy == 1 {
        w
    } else {
        /*
         * if allow error up to 2 ulp, simply return
         * -1.0 / (x+r) here
         */
        /* compute -1.0 / (x+r) accurately */
        let (a, mut t): (f64, f64);
        z = w;
        z = zero_lo(z);
        v = r - (z - x); /* z+v = r+x */
        a = -1.0 / w; /* a = -1.0/w */
        t = a;
        t = zero_lo(t);
        s = 1.0 + t * z;
        t + a * (s + t * v)
    }
}

// ufbx_math.c:1213-1246 `ufbx_sin`
pub(crate) fn sin(x: f64) -> f64 {
    let mut y: [f64; 2] = [0.0; 2];
    let z: f64 = 0.0;
    let mut ix: i32;

    /* High word of x. */
    ix = hi(x);

    /* |x| ~< pi/4 */
    ix &= 0x7fffffff;
    if ix <= 0x3fe921fb {
        kernel_sin(x, z, 0)
    }
    /* sin(Inf or NaN) is NaN */
    else if ix >= 0x7ff00000 {
        x - x
    }
    /* argument reduction needed */
    else {
        let n = rem_pio2(x, &mut y);
        match n & 3 {
            0 => kernel_sin(y[0], y[1], 1),
            1 => kernel_cos(y[0], y[1]),
            2 => -kernel_sin(y[0], y[1], 1),
            _ => -kernel_cos(y[0], y[1]),
        }
    }
}

// ufbx_math.c:1248-1281 `ufbx_cos`
pub(crate) fn cos(x: f64) -> f64 {
    let mut y: [f64; 2] = [0.0; 2];
    let z: f64 = 0.0;
    let mut ix: i32;

    /* High word of x. */
    ix = hi(x);

    /* |x| ~< pi/4 */
    ix &= 0x7fffffff;
    if ix <= 0x3fe921fb {
        kernel_cos(x, z)
    }
    /* cos(Inf or NaN) is NaN */
    else if ix >= 0x7ff00000 {
        x - x
    }
    /* argument reduction needed */
    else {
        let n = rem_pio2(x, &mut y);
        match n & 3 {
            0 => kernel_cos(y[0], y[1]),
            1 => -kernel_sin(y[0], y[1], 1),
            2 => -kernel_cos(y[0], y[1]),
            _ => kernel_sin(y[0], y[1], 1),
        }
    }
}

// ufbx_math.c:1283-1307 `ufbx_tan`
pub(crate) fn tan(x: f64) -> f64 {
    let mut y: [f64; 2] = [0.0; 2];
    let z: f64 = 0.0;
    let mut ix: i32;

    /* High word of x. */
    ix = hi(x);

    /* |x| ~< pi/4 */
    ix &= 0x7fffffff;
    if ix <= 0x3fe921fb {
        kernel_tan(x, z, 1)
    }
    /* tan(Inf or NaN) is NaN */
    else if ix >= 0x7ff00000 {
        x - x /* NaN */
    }
    /* argument reduction needed */
    else {
        let n = rem_pio2(x, &mut y);
        kernel_tan(y[0], y[1], 1 - ((n & 1) << 1)) /*   1 -- n even
                                                   -1 -- n odd */
    }
}

// ufbx_math.c:1309-1380 `ufbx_asin`
pub(crate) fn asin(x: f64) -> f64 {
    const ONE: f64 = 1.00000000000000000000e+00; /* 0x3FF00000, 0x00000000 */
    const HUGE: f64 = 1.000e+300;
    const PIO2_HI: f64 = 1.57079632679489655800e+00; /* 0x3FF921FB, 0x54442D18 */
    const PIO2_LO: f64 = 6.12323399573676603587e-17; /* 0x3C91A626, 0x33145C07 */
    const PIO4_HI: f64 = 7.85398163397448278999e-01; /* 0x3FE921FB, 0x54442D18 */
    /* coefficient for R(x^2) */
    const PS0: f64 = 1.66666666666666657415e-01; /* 0x3FC55555, 0x55555555 */
    const PS1: f64 = -3.25565818622400915405e-01; /* 0xBFD4D612, 0x03EB6F7D */
    const PS2: f64 = 2.01212532134862925881e-01; /* 0x3FC9C155, 0x0E884455 */
    const PS3: f64 = -4.00555345006794114027e-02; /* 0xBFA48228, 0xB5688F3B */
    const PS4: f64 = 7.91534994289814532176e-04; /* 0x3F49EFE0, 0x7501B288 */
    const PS5: f64 = 3.47933107596021167570e-05; /* 0x3F023DE1, 0x0DFDF709 */
    const QS1: f64 = -2.40339491173441421878e+00; /* 0xC0033A27, 0x1C8A2D4B */
    const QS2: f64 = 2.02094576023350569471e+00; /* 0x40002AE5, 0x9C598AC8 */
    const QS3: f64 = -6.88283971605453293030e-01; /* 0xBFE6066C, 0x1B8D0159 */
    const QS4: f64 = 7.70381505559019352791e-02; /* 0x3FB3B8C5, 0xB12E9282 */

    let (mut t, mut w, mut p, mut q, c, r, s): (f64, f64, f64, f64, f64, f64, f64);
    t = 0.0;
    let bx = to_bits(x);
    let hx: i32 = bx.hi;
    let ix: i32 = hx & 0x7fffffff;
    if ix >= 0x3ff00000 {
        /* |x|>= 1 */
        if ((ix - 0x3ff00000) | bx.lo as i32) == 0 {
            /* asin(1)=+-pi/2 with inexact */
            return x * PIO2_HI + x * PIO2_LO;
        }
        return (x - x) / (x - x); /* asin(|x|>1) is NaN */
    } else if ix < 0x3fe00000 {
        /* |x|<0.5 */
        if ix < 0x3e400000 {
            /* if |x| < 2**-27 */
            if HUGE + x > ONE {
                return x; /* return x with inexact if x!=0*/
            }
        } else {
            t = x * x;
        }
        p = t * (PS0 + t * (PS1 + t * (PS2 + t * (PS3 + t * (PS4 + t * PS5)))));
        q = ONE + t * (QS1 + t * (QS2 + t * (QS3 + t * QS4)));
        w = p / q;
        return x + x * w;
    }
    /* 1> |x|>= 0.5 */
    w = ONE - fabs(x);
    t = w * 0.5;
    p = t * (PS0 + t * (PS1 + t * (PS2 + t * (PS3 + t * (PS4 + t * PS5)))));
    q = ONE + t * (QS1 + t * (QS2 + t * (QS3 + t * QS4)));
    s = sqrt(t);
    if ix >= 0x3FEF3333 {
        /* if |x| > 0.975 */
        w = p / q;
        t = PIO2_HI - (2.0 * (s + s * w) - PIO2_LO);
    } else {
        w = s;
        w = zero_lo(w);
        c = (t - w * w) / (s + w);
        r = p / q;
        p = 2.0 * s * r - (PIO2_LO - 2.0 * c);
        q = PIO4_HI - 2.0 * w;
        t = PIO4_HI - (p - q);
    }
    if hx > 0 {
        t
    } else {
        -t
    }
}

// ufbx_math.c:1382-1449 `ufbx_acos`
pub(crate) fn acos(x: f64) -> f64 {
    const ONE: f64 = 1.00000000000000000000e+00; /* 0x3FF00000, 0x00000000 */
    const PI: f64 = 3.14159265358979311600e+00; /* 0x400921FB, 0x54442D18 */
    const PIO2_HI: f64 = 1.57079632679489655800e+00; /* 0x3FF921FB, 0x54442D18 */
    const PIO2_LO: f64 = 6.12323399573676603587e-17; /* 0x3C91A626, 0x33145C07 */
    const PS0: f64 = 1.66666666666666657415e-01; /* 0x3FC55555, 0x55555555 */
    const PS1: f64 = -3.25565818622400915405e-01; /* 0xBFD4D612, 0x03EB6F7D */
    const PS2: f64 = 2.01212532134862925881e-01; /* 0x3FC9C155, 0x0E884455 */
    const PS3: f64 = -4.00555345006794114027e-02; /* 0xBFA48228, 0xB5688F3B */
    const PS4: f64 = 7.91534994289814532176e-04; /* 0x3F49EFE0, 0x7501B288 */
    const PS5: f64 = 3.47933107596021167570e-05; /* 0x3F023DE1, 0x0DFDF709 */
    const QS1: f64 = -2.40339491173441421878e+00; /* 0xC0033A27, 0x1C8A2D4B */
    const QS2: f64 = 2.02094576023350569471e+00; /* 0x40002AE5, 0x9C598AC8 */
    const QS3: f64 = -6.88283971605453293030e-01; /* 0xBFE6066C, 0x1B8D0159 */
    const QS4: f64 = 7.70381505559019352791e-02; /* 0x3FB3B8C5, 0xB12E9282 */

    let (z, p, q, r, w, s, c, mut df): (f64, f64, f64, f64, f64, f64, f64, f64);
    let bx = to_bits(x);
    let hx: i32 = bx.hi;
    let ix: i32 = hx & 0x7fffffff;
    if ix >= 0x3ff00000 {
        /* |x| >= 1 */
        if ((ix - 0x3ff00000) | bx.lo as i32) == 0 {
            /* |x|==1 */
            if hx > 0 {
                return 0.0; /* acos(1) = 0  */
            } else {
                return PI + 2.0 * PIO2_LO; /* acos(-1)= pi */
            }
        }
        return (x - x) / (x - x); /* acos(|x|>1) is NaN */
    }
    if ix < 0x3fe00000 {
        /* |x| < 0.5 */
        if ix <= 0x3c600000 {
            return PIO2_HI + PIO2_LO; /*if|x|<2**-57*/
        }
        z = x * x;
        p = z * (PS0 + z * (PS1 + z * (PS2 + z * (PS3 + z * (PS4 + z * PS5)))));
        q = ONE + z * (QS1 + z * (QS2 + z * (QS3 + z * QS4)));
        r = p / q;
        PIO2_HI - (x - (PIO2_LO - x * r))
    } else if hx < 0 {
        /* x < -0.5 */
        z = (ONE + x) * 0.5;
        p = z * (PS0 + z * (PS1 + z * (PS2 + z * (PS3 + z * (PS4 + z * PS5)))));
        q = ONE + z * (QS1 + z * (QS2 + z * (QS3 + z * QS4)));
        s = sqrt(z);
        r = p / q;
        w = r * s - PIO2_LO;
        PI - 2.0 * (s + w)
    } else {
        /* x > 0.5 */
        z = (ONE - x) * 0.5;
        s = sqrt(z);
        df = s;
        df = zero_lo(df);
        c = (z - df * df) / (s + df);
        p = z * (PS0 + z * (PS1 + z * (PS2 + z * (PS3 + z * (PS4 + z * PS5)))));
        q = ONE + z * (QS1 + z * (QS2 + z * (QS3 + z * QS4)));
        r = p / q;
        w = r * s + c;
        2.0 * (df + w)
    }
}

// ufbx_math.c:1451-1556 `ufbx_atan2`
pub(crate) fn atan2(y: f64, x: f64) -> f64 {
    const TINY: f64 = 1.0e-300;
    const ZERO: f64 = 0.0;
    const PI_O_4: f64 = 7.8539816339744827900E-01; /* 0x3FE921FB, 0x54442D18 */
    const PI_O_2: f64 = 1.5707963267948965580E+00; /* 0x3FF921FB, 0x54442D18 */
    const PI: f64 = 3.1415926535897931160E+00; /* 0x400921FB, 0x54442D18 */
    const PI_LO: f64 = 1.2246467991473531772E-16; /* 0x3CA1A626, 0x33145C07 */

    let z: f64;
    let bx = to_bits(x);
    let by = to_bits(y);

    let hx: i32 = bx.hi;
    let lx: u32 = bx.lo;
    let hy: i32 = by.hi;
    let ly: u32 = by.lo;
    let ix: i32 = hx & 0x7fffffff;
    let iy: i32 = hy & 0x7fffffff;
    if ((ix as u32 | ((lx | lx.wrapping_neg()) >> 31)) > 0x7ff00000)
        || ((iy as u32 | ((ly | ly.wrapping_neg()) >> 31)) > 0x7ff00000)
    /* x or y is NaN */
    {
        return x + y;
    }
    if ((hx.wrapping_sub(0x3ff00000)) | lx as i32) == 0 {
        return atan(y); /* x=1.0 */
    }
    let m: i32 = ((hy >> 31) & 1) | ((hx >> 30) & 2); /* 2*sign(x)+sign(y) */

    /* when y = 0 */
    if (iy | ly as i32) == 0 {
        match m {
            0 | 1 => return y,      /* atan(+-0,+anything)=+-0 */
            2 => return PI + TINY,  /* atan(+0,-anything) = pi */
            3 => return -PI - TINY, /* atan(-0,-anything) =-pi */
            _ => {}
        }
    }
    /* when x = 0 */
    if (ix | lx as i32) == 0 {
        return if hy < 0 {
            -PI_O_2 - TINY
        } else {
            PI_O_2 + TINY
        };
    }

    /* when x is INF */
    if ix == 0x7ff00000 {
        if iy == 0x7ff00000 {
            match m {
                0 => return PI_O_4 + TINY,        /* atan(+INF,+INF) */
                1 => return -PI_O_4 - TINY,       /* atan(-INF,+INF) */
                2 => return 3.0 * PI_O_4 + TINY,  /*atan(+INF,-INF)*/
                3 => return -3.0 * PI_O_4 - TINY, /*atan(-INF,-INF)*/
                _ => {}
            }
        } else {
            match m {
                0 => return ZERO,       /* atan(+...,+INF) */
                1 => return -ZERO,      /* atan(-...,+INF) */
                2 => return PI + TINY,  /* atan(+...,-INF) */
                3 => return -PI - TINY, /* atan(-...,-INF) */
                _ => {}
            }
        }
    }
    /* when y is INF */
    if iy == 0x7ff00000 {
        return if hy < 0 {
            -PI_O_2 - TINY
        } else {
            PI_O_2 + TINY
        };
    }

    /* compute y/x */
    let k: i32 = (iy - ix) >> 20;
    if k > 60 {
        z = PI_O_2 + 0.5 * PI_LO; /* |y/x| >  2**60 */
    } else if hx < 0 && k < -60 {
        z = 0.0; /* |y|/x < -2**60 */
    } else {
        z = atan(fabs(y / x)); /* safe to do y/x */
    }
    match m {
        0 => z,                /* atan(+,+) */
        1 => -z,               /* atan(-,+) */
        2 => PI - (z - PI_LO), /* atan(+,-) */
        _ => (z - PI_LO) - PI, /* case 3: atan(-,-) */
    }
}

// ufbx_math.c:1558-1864 `ufbx_pow`
pub(crate) fn pow(x: f64, y: f64) -> f64 {
    const BP: [f64; 2] = [1.0, 1.5];
    const DP_H: [f64; 2] = [0.0, 5.84962487220764160156e-01]; /* 0x3FE2B803, 0x40000000 */
    const DP_L: [f64; 2] = [0.0, 1.35003920212974897128e-08]; /* 0x3E4CFDEB, 0x43CFD006 */
    const ZERO: f64 = 0.0;
    const ONE: f64 = 1.0;
    const TWO: f64 = 2.0;
    const TWO53: f64 = 9007199254740992.0; /* 0x43400000, 0x00000000 */
    const HUGE: f64 = 1.0e300;
    const TINY: f64 = 1.0e-300;
    /* poly coefs for (3/2)*(log(x)-2s-2/3*s**3 */
    const L1: f64 = 5.99999999999994648725e-01; /* 0x3FE33333, 0x33333303 */
    const L2: f64 = 4.28571428578550184252e-01; /* 0x3FDB6DB6, 0xDB6FABFF */
    const L3: f64 = 3.33333329818377432918e-01; /* 0x3FD55555, 0x518F264D */
    const L4: f64 = 2.72728123808534006489e-01; /* 0x3FD17460, 0xA91D4101 */
    const L5: f64 = 2.30660745775561754067e-01; /* 0x3FCD864A, 0x93C9DB65 */
    const L6: f64 = 2.06975017800338417784e-01; /* 0x3FCA7E28, 0x4A454EEF */
    const P1: f64 = 1.66666666666666019037e-01; /* 0x3FC55555, 0x5555553E */
    const P2: f64 = -2.77777777770155933842e-03; /* 0xBF66C16C, 0x16BEBD93 */
    const P3: f64 = 6.61375632143793436117e-05; /* 0x3F11566A, 0xAF25DE2C */
    const P4: f64 = -1.65339022054652515390e-06; /* 0xBEBBBD41, 0xC5D26BF1 */
    const P5: f64 = 4.13813679705723846039e-08; /* 0x3E663769, 0x72BEA4D0 */
    const LG2: f64 = 6.93147180559945286227e-01; /* 0x3FE62E42, 0xFEFA39EF */
    const LG2_H: f64 = 6.93147182464599609375e-01; /* 0x3FE62E43, 0x00000000 */
    const LG2_L: f64 = -1.90465429995776804525e-09; /* 0xBE205C61, 0x0CA86C39 */
    const OVT: f64 = 8.0085662595372944372e-0017; /* -(1024-log2(ovfl+.5ulp)) */
    const CP: f64 = 9.61796693925975554329e-01; /* 0x3FEEC709, 0xDC3A03FD =2/(3ln2) */
    const CP_H: f64 = 9.61796700954437255859e-01; /* 0x3FEEC709, 0xE0000000 =(float)cp */
    const CP_L: f64 = -7.02846165095275826516e-09; /* 0xBE3E2FE0, 0x145B01F5 =tail of cp_h*/
    const IVLN2: f64 = 1.44269504088896338700e+00; /* 0x3FF71547, 0x652B82FE =1/ln2 */
    const IVLN2_H: f64 = 1.44269502162933349609e+00; /* 0x3FF71547, 0x60000000 =24b 1/ln2*/
    const IVLN2_L: f64 = 1.92596299112661746887e-08; /* 0x3E54AE0B, 0xF85DDF44 =1/ln2 tail*/

    let (mut z, mut ax, z_h, z_l, mut p_h, mut p_l): (f64, f64, f64, f64, f64, f64);
    let (mut y1, mut t1, t2, mut r, mut s, mut t, mut u, mut v, mut w): (
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
    );
    let (mut i, mut j, mut k, mut yisint, mut n): (i32, i32, i32, i32, i32);
    let (hx, hy, mut ix, iy): (i32, i32, i32, i32);
    let (lx, ly): (u32, u32);
    let bx = to_bits(x);
    let by = to_bits(y);
    let mut bz: Bits;

    hx = bx.hi;
    lx = bx.lo;
    hy = by.hi;
    ly = by.lo;
    ix = hx & 0x7fffffff;
    iy = hy & 0x7fffffff;

    /* y==zero: x**0 = 1 */
    if (iy | ly as i32) == 0 {
        return ONE;
    }

    /* +-NaN return x+y */
    if ix > 0x7ff00000
        || ((ix == 0x7ff00000) && (lx != 0))
        || iy > 0x7ff00000
        || ((iy == 0x7ff00000) && (ly != 0))
    {
        return x + y;
    }

    /* determine if y is an odd ufbxm_int when x < 0
     * yisint = 0	... y is not an integer
     * yisint = 1	... y is an odd ufbxm_int
     * yisint = 2	... y is an even ufbxm_int
     */
    yisint = 0;
    if hx < 0 {
        if iy >= 0x43400000 {
            yisint = 2; /* even integer y */
        } else if iy >= 0x3ff00000 {
            k = (iy >> 20) - 0x3ff; /* exponent */
            if k > 20 {
                j = (ly >> (52 - k)) as i32;
                if (j << (52 - k)) == ly as i32 {
                    yisint = 2 - (j & 1);
                }
            } else if ly == 0 {
                j = iy >> (20 - k);
                if (j << (20 - k)) == iy {
                    yisint = 2 - (j & 1);
                }
            }
        }
    }

    /* special value of y */
    if ly == 0 {
        if iy == 0x7ff00000 {
            /* y is +-inf */
            if ((ix - 0x3ff00000) | lx as i32) == 0 {
                return y - y; /* inf**+-1 is NaN */
            } else if ix >= 0x3ff00000 {
                /* (|x|>1)**+-inf = inf,0 */
                return if hy >= 0 { y } else { ZERO };
            } else {
                /* (|x|<1)**-,+inf = inf,0 */
                return if hy < 0 { -y } else { ZERO };
            }
        }
        if iy == 0x3ff00000 {
            /* y is  +-1 */
            if hy < 0 {
                return ONE / x;
            } else {
                return x;
            }
        }
        if hy == 0x40000000 {
            return x * x; /* y is  2 */
        }
        if hy == 0x3fe00000 {
            /* y is  0.5 */
            if hx >= 0 {
                /* x >= +0 */
                return sqrt(x);
            }
        }
    }

    ax = fabs(x);
    /* special value of x */
    if lx == 0 {
        if ix == 0x7ff00000 || ix == 0 || ix == 0x3ff00000 {
            z = ax; /*x is +-0,+-inf,+-1*/
            if hy < 0 {
                z = ONE / z; /* z = (1/|x|) */
            }
            if hx < 0 {
                if ((ix - 0x3ff00000) | yisint) == 0 {
                    z = (z - z) / (z - z); /* (-1)**non-ufbxm_int is NaN */
                } else if yisint == 1 {
                    z = -z; /* (x<0)**odd = -(|x|**odd) */
                }
            }
            return z;
        }
    }

    n = (hx >> 31) + 1;

    /* (x<0)**(non-ufbxm_int) is NaN */
    if (n | yisint) == 0 {
        return (x - x) / (x - x);
    }

    s = ONE; /* s (sign of result -ve**odd) = -1 else = 1 */
    if (n | (yisint - 1)) == 0 {
        s = -ONE; /* (-ve)**(odd ufbxm_int) */
    }

    /* |y| is huge */
    if iy > 0x41e00000 {
        /* if |y| > 2**31 */
        if iy > 0x43f00000 {
            /* if |y| > 2**64, must o/uflow */
            if ix <= 0x3fefffff {
                return if hy < 0 { HUGE * HUGE } else { TINY * TINY };
            }
            if ix >= 0x3ff00000 {
                return if hy > 0 { HUGE * HUGE } else { TINY * TINY };
            }
        }
        /* over/underflow if x is not close to one */
        if ix < 0x3fefffff {
            return if hy < 0 {
                s * HUGE * HUGE
            } else {
                s * TINY * TINY
            };
        }
        if ix > 0x3ff00000 {
            return if hy > 0 {
                s * HUGE * HUGE
            } else {
                s * TINY * TINY
            };
        }
        /* now |1-x| is tiny <= 2**-20, suffice to compute
        log(x) by x-x^2/2+x^3/3-x^4/4 */
        t = ax - ONE; /* t has 20 trailing zeros */
        w = (t * t) * (0.5 - t * (0.3333333333333333333333 - t * 0.25));
        u = IVLN2_H * t; /* ivln2_h has 21 sig. bits */
        v = t * IVLN2_L - w * IVLN2;
        t1 = u + v;
        t1 = zero_lo(t1);
        t2 = v - (t1 - u);
    } else {
        let (ss, mut s2, mut s_h, s_l, mut t_h, mut t_l): (f64, f64, f64, f64, f64, f64);
        n = 0;
        /* take care subnormal number */
        if ix < 0x00100000 {
            ax *= TWO53;
            n -= 53;
            ix = hi(ax);
        }
        n += (ix >> 20) - 0x3ff;
        j = ix & 0x000fffff;
        /* determine interval */
        ix = j | 0x3ff00000; /* normalize ix */
        if j <= 0x3988E {
            k = 0; /* |x|<sqrt(3/2) */
        } else if j < 0xBB67A {
            k = 1; /* |x|<sqrt(3)   */
        } else {
            k = 0;
            n += 1;
            ix -= 0x00100000;
        }
        ax = set_hi(ax, ix);

        /* compute ss = s_h+s_l = (x-1)/(x+1) or (x-1.5)/(x+1.5) */
        u = ax - BP[k as usize]; /* bp[0]=1.0, bp[1]=1.5 */
        v = ONE / (ax + BP[k as usize]);
        ss = u * v;
        s_h = ss;
        s_h = zero_lo(s_h);
        /* t_h=ax+bp[k] High */
        t_h = ZERO;
        t_h = set_hi(
            t_h,
            (((ix >> 1) | 0x20000000) + 0x00080000).wrapping_add(k << 18),
        );
        t_l = ax - (t_h - BP[k as usize]);
        s_l = v * ((u - s_h * t_h) - s_h * t_l);
        /* compute log(ax) */
        s2 = ss * ss;
        r = s2 * s2 * (L1 + s2 * (L2 + s2 * (L3 + s2 * (L4 + s2 * (L5 + s2 * L6)))));
        r += s_l * (s_h + ss);
        s2 = s_h * s_h;
        t_h = 3.0 + s2 + r;
        t_h = zero_lo(t_h);
        t_l = r - ((t_h - 3.0) - s2);
        /* u+v = ss*(1+...) */
        u = s_h * t_h;
        v = s_l * t_h + t_l * ss;
        /* 2/(3log2)*(ss+...) */
        p_h = u + v;
        p_h = zero_lo(p_h);
        p_l = v - (p_h - u);
        z_h = CP_H * p_h; /* cp_h+cp_l = 2/(3*log2) */
        z_l = CP_L * p_h + p_l * CP + DP_L[k as usize];
        /* log2(ax) = (ss+..)*2/(3*log2) = n + dp_h + z_h + z_l */
        t = n as f64;
        t1 = ((z_h + z_l) + DP_H[k as usize]) + t;
        t1 = zero_lo(t1);
        t2 = z_l - (((t1 - t) - DP_H[k as usize]) - z_h);
    }

    /* split up y into y1+y2 and compute (y1+y2)*(t1+t2) */
    y1 = y;
    y1 = zero_lo(y1);
    p_l = (y - y1) * t1 + y * t2;
    p_h = y1 * t1;
    z = p_l + p_h;
    bz = to_bits(z);
    j = bz.hi;
    i = bz.lo as i32;
    if j >= 0x40900000 {
        /* z >= 1024 */
        if ((j - 0x40900000) | i) != 0 {
            /* if z > 1024 */
            return s * HUGE * HUGE; /* overflow */
        } else if p_l + OVT > z - p_h {
            return s * HUGE * HUGE; /* overflow */
        }
    } else if (j & 0x7fffffff) >= 0x4090cc00 {
        /* z <= -1075 */
        if (j.wrapping_sub(0xc090cc00u32 as i32) | i) != 0 {
            /* z < -1075 */
            return s * TINY * TINY; /* underflow */
        } else if p_l <= z - p_h {
            return s * TINY * TINY; /* underflow */
        }
    }
    /*
     * compute 2**(p_h+p_l)
     */
    i = j & 0x7fffffff;
    k = (i >> 20) - 0x3ff;
    n = 0;
    if i > 0x3fe00000 {
        /* if |z| > 0.5, set n = [z+0.5] */
        n = j.wrapping_add(0x00100000 >> (k + 1));
        k = ((n & 0x7fffffff) >> 20) - 0x3ff; /* new k for n */
        t = ZERO;
        t = set_hi(t, n & !(0x000fffff >> k));
        n = ((n & 0x000fffff) | 0x00100000) >> (20 - k);
        if j < 0 {
            n = -n;
        }
        p_h -= t;
    }
    t = p_l + p_h;
    t = zero_lo(t);
    u = t * LG2_H;
    v = (p_l - (t - p_h)) * LG2 + t * LG2_L;
    z = u + v;
    w = v - (z - u);
    t = z * z;
    t1 = z - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
    r = (z * t1) / (t1 - TWO) - (w + z * w);
    z = ONE - (r - z);
    j = hi(z);
    j = j.wrapping_add(n << 20);
    if (j >> 20) <= 0 {
        z = scalbn(z, n); /* subnormal output */
    } else {
        bz = to_bits(z);
        bz.hi = bz.hi.wrapping_add(n << 20);
        z = from_bits(bz.hi, bz.lo);
    }
    s * z
}

// ufbx_math.c:1866-1869 `ufbx_fmin`
pub(crate) fn fmin(a: f64, b: f64) -> f64 {
    if a < b {
        a
    } else {
        b
    }
}

// ufbx_math.c:1871-1874 `ufbx_fmax`
pub(crate) fn fmax(a: f64, b: f64) -> f64 {
    if a < b {
        b
    } else {
        a
    }
}

// ufbx_math.c:1876-1946 `ufbx_nextafter`
pub(crate) fn nextafter(x: f64, y: f64) -> f64 {
    let mut x = x;
    let mut y = y;
    let bx = to_bits(x);
    let by = to_bits(y);

    let mut hx: i32 = bx.hi; /* high word of x */
    let mut lx: u32 = bx.lo; /* low  word of x */
    let hy: i32 = by.hi; /* high word of y */
    let ly: u32 = by.lo; /* low  word of y */
    let ix: i32 = hx & 0x7fffffff; /* |x| */
    let iy: i32 = hy & 0x7fffffff; /* |y| */

    if ((ix >= 0x7ff00000) && ((ix - 0x7ff00000) | lx as i32) != 0)/* x is nan */
        || ((iy >= 0x7ff00000) && ((iy - 0x7ff00000) | ly as i32) != 0)
    /* y is nan */
    {
        return x + y;
    }
    if x == y {
        return x; /* x=y, return x */
    }
    if (ix | lx as i32) == 0 {
        /* x == 0 */
        x = from_bits(hy & 0x80000000u32 as i32, 1);
        y = x * x;
        if y == x {
            return y;
        } else {
            return x; /* raise underflow flag */
        }
    }
    if hx >= 0 {
        /* x > 0 */
        if hx > hy || ((hx == hy) && (lx > ly)) {
            /* x > y, x -= ulp */
            if lx == 0 {
                hx = hx.wrapping_sub(1);
            }
            lx = lx.wrapping_sub(1);
        } else {
            /* x < y, x += ulp */
            lx = lx.wrapping_add(1);
            if lx == 0 {
                hx = hx.wrapping_add(1);
            }
        }
    } else {
        /* x < 0 */
        if hy >= 0 || hx > hy || ((hx == hy) && (lx > ly)) {
            /* x < y, x -= ulp */
            if lx == 0 {
                hx = hx.wrapping_sub(1);
            }
            lx = lx.wrapping_sub(1);
        } else {
            /* x > y, x += ulp */
            lx = lx.wrapping_add(1);
            if lx == 0 {
                hx = hx.wrapping_add(1);
            }
        }
    }
    let hy2: i32 = hx & 0x7ff00000;
    if hy2 >= 0x7ff00000 {
        return x + x; /* overflow  */
    }
    if hy2 < 0x00100000 {
        /* underflow */
        y = x * x;
        if y != x {
            /* raise underflow flag */
            return from_bits(hx, lx);
        }
    }
    from_bits(hx, lx)
}

// ufbx_math.c:1948-2020 `ufbx_ceil`
pub(crate) fn ceil(x: f64) -> f64 {
    const HUGE: f64 = 1.0e300;

    let bx = to_bits(x);
    let mut i0: i32 = bx.hi;
    let mut i1: i32 = bx.lo as i32;
    let j0: i32 = ((i0 >> 20) & 0x7ff) - 0x3ff;
    if j0 < 20 {
        if j0 < 0 {
            /* raise inexact if x != 0 */
            if HUGE + x > 0.0 {
                /* return 0*sign(x) if |x|<1 */
                if i0 < 0 {
                    i0 = 0x80000000u32 as i32;
                    i1 = 0;
                } else if (i0 | i1) != 0 {
                    i0 = 0x3ff00000u32 as i32;
                    i1 = 0;
                }
            }
        } else {
            let i: u32 = 0x000fffffu32 >> j0;
            if ((i0 & i as i32) | i1) == 0 {
                return x; /* x is integral */
            }
            if HUGE + x > 0.0 {
                /* raise inexact flag */
                if i0 > 0 {
                    i0 = i0.wrapping_add(0x00100000 >> j0);
                }
                i0 &= !(i as i32);
                i1 = 0;
            }
        }
    } else if j0 > 51 {
        if j0 == 0x400 {
            return x + x; /* inf or NaN */
        } else {
            return x; /* x is integral */
        }
    } else {
        let i: u32 = 0xffffffffu32 >> (j0 - 20);
        if (i1 & i as i32) == 0 {
            return x; /* x is integral */
        }
        if HUGE + x > 0.0 {
            /* raise inexact flag */
            if i0 > 0 {
                if j0 == 20 {
                    i0 = i0.wrapping_add(1);
                } else {
                    let j: u32 = (i1 as u32).wrapping_add(1u32 << (52 - j0));
                    if j < i1 as u32 {
                        i0 = i0.wrapping_add(1); /* got a carry */
                    }
                    i1 = j as i32;
                }
            }
            i1 &= !(i as i32);
        }
    }
    from_bits(i0, i1 as u32)
}

// ufbx_math.c:2022-2074 `ufbx_rint`
// NOTE: rint is round-half-to-EVEN — never `f64::round` (PORTING.md "Floats");
// it sits on the keyframe-time quantization path (ufbx.c:26003-26004).
pub(crate) fn rint(x: f64) -> f64 {
    const TWO52: [f64; 2] = [
        4.50359962737049600000e+15,  /* 0x43300000, 0x00000000 */
        -4.50359962737049600000e+15, /* 0xC3300000, 0x00000000 */
    ];

    let mut x = x;
    let (w, t): (f64, f64);
    let bx = to_bits(x);
    let mut i0: i32 = bx.hi;
    let mut i1: u32 = bx.lo;
    let sx: i32 = (i0 >> 31) & 1;
    let j0: i32 = ((i0 >> 20) & 0x7ff) - 0x3ff;
    if j0 < 20 {
        if j0 < 0 {
            if ((i0 & 0x7fffffff) | i1 as i32) == 0 {
                return x;
            }
            i1 |= (i0 & 0x0fffff) as u32;
            i0 &= 0xfffe0000u32 as i32;
            i0 |= (((i1 | i1.wrapping_neg()) >> 12) as i32) & 0x80000;
            x = set_hi(x, i0);
            w = TWO52[sx as usize] + x;
            t = w - TWO52[sx as usize];
            let bt = to_bits(t);
            i0 = bt.hi;
            return from_bits((i0 & 0x7fffffff) | (sx << 31), bt.lo);
        } else {
            let mut i: u32 = 0x000fffffu32 >> j0;
            if ((i0 & i as i32) | i1 as i32) == 0 {
                return x; /* x is integral */
            }
            i >>= 1;
            if ((i0 & i as i32) | i1 as i32) != 0 {
                if j0 == 19 {
                    i1 = 0x40000000;
                } else {
                    i0 = (i0 & !(i as i32)) | (0x20000 >> j0);
                }
            }
        }
    } else if j0 > 51 {
        if j0 == 0x400 {
            return x + x; /* inf or NaN */
        } else {
            return x; /* x is integral */
        }
    } else {
        let mut i: u32 = 0xffffffffu32 >> (j0 - 20);
        if (i1 & i) == 0 {
            return x; /* x is integral */
        }
        i >>= 1;
        if (i1 & i) != 0 {
            i1 = (i1 & !i) | (0x40000000u32 >> (j0 - 20));
        }
    }
    x = from_bits(i0, i1);
    let w = TWO52[sx as usize] + x;
    w - TWO52[sx as usize]
}

// ufbx_math.c:2076-2085 `ufbx_isnan` (C returns `int`)
pub(crate) fn isnan(x: f64) -> i32 {
    let bx = to_bits(x);
    let mut hx: i32 = bx.hi & 0x7fffffff;
    let lx: i32 = bx.lo as i32;
    hx |= (((lx | lx.wrapping_neg()) as u32) >> 31) as i32;
    hx = 0x7ff00000i32.wrapping_sub(hx);
    ((hx as u32) >> 31) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    // Deterministic xorshift so the sweeps are reproducible.
    struct Rng(u64);
    impl Rng {
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
        fn f64_range(&mut self, lo: f64, hi: f64) -> f64 {
            let u = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
            lo + (hi - lo) * u
        }
        fn any_f64(&mut self) -> f64 {
            f64::from_bits(self.next_u64())
        }
    }

    fn edge_cases() -> Vec<f64> {
        let mut v = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            1.5,
            -1.5,
            2.0,
            -2.0,
            0.25,
            2.5,
            -2.5,
            3.5,
            -3.5,
            4503599627370496.0, // 2^52
            -4503599627370496.0,
            9007199254740992.0, // 2^53
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            f64::MIN_POSITIVE / 3.0, // subnormal
            f64::MAX,
            f64::MIN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            core::f64::consts::PI,
            core::f64::consts::FRAC_PI_2,
            core::f64::consts::FRAC_PI_4,
            1e300,
            1e-300,
        ];
        let mut rng = Rng(0x1234_5678_9abc_def1);
        for _ in 0..2000 {
            v.push(rng.any_f64());
            v.push(rng.f64_range(-1e6, 1e6));
            v.push(rng.f64_range(-1.0, 1.0));
        }
        v
    }

    fn same(a: f64, b: f64) -> bool {
        (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits()
    }

    // The bit-exact-by-construction members of the shim are also exactly
    // reproducible by std, so they get a full differential sweep. Transcription
    // errors in the ported bit twiddling show up here.
    #[test]
    fn exact_ops_match_std() {
        for &x in edge_cases().iter() {
            assert!(same(floor(x), x.floor()), "floor({x:?})");
            assert!(same(ceil(x), x.ceil()), "ceil({x:?})");
            assert!(same(rint(x), x.round_ties_even()), "rint({x:?})");
            assert!(same(fabs(x), x.abs()), "fabs({x:?})");
            assert!(same(sqrt(x), x.sqrt()), "sqrt({x:?})");
            assert_eq!(isnan(x) != 0, x.is_nan(), "isnan({x:?})");
            for &y in [1.0f64, -1.0, 0.0, -0.0, f64::NAN, -f64::NAN].iter() {
                assert!(same(copysign(x, y), x.copysign(y)), "copysign({x:?},{y:?})");
            }
        }
    }

    #[test]
    fn nextafter_matches_std() {
        for &x in edge_cases().iter() {
            for &y in [f64::NEG_INFINITY, f64::INFINITY, 0.0, -0.0, 1.0, -1.0].iter() {
                let got = nextafter(x, y);
                let want = if x.is_nan() || y.is_nan() {
                    x + y
                } else if x == y {
                    x
                } else if y > x {
                    x.next_up()
                } else {
                    x.next_down()
                };
                assert!(
                    same(got, want),
                    "nextafter({x:?},{y:?}) = {got:?} != {want:?}"
                );
            }
        }
    }

    #[test]
    fn scalbn_matches_std() {
        let mut rng = Rng(0xdead_beef_0bad_f00d);
        for _ in 0..20000 {
            let x = rng.any_f64();
            let n = (rng.next_u64() % 2200) as i32 - 1100;
            let got = scalbn(x, n);
            // Reference: exact scaling by 2^n via repeated halving/doubling in
            // steps small enough to avoid spurious over/underflow.
            let mut want = x;
            let mut k = n;
            while k > 0 {
                let step = k.min(500);
                want *= f64::from_bits(((1023i64 + step as i64) as u64) << 52);
                k -= step;
            }
            while k < 0 {
                let step = (-k).min(500);
                want *= f64::from_bits(((1023i64 - step as i64) as u64) << 52);
                k += step;
            }
            assert!(same(got, want), "scalbn({x:?},{n}) = {got:?} != {want:?}");
        }
    }

    // The transcendentals have no bit-exact std reference (that is the entire
    // point of this module), so these only bound the error — bit-exactness vs
    // the C is proven by the scene-hash oracle.
    #[test]
    fn transcendentals_are_close_to_std() {
        fn close(a: f64, b: f64, ulps: u64) -> bool {
            if a.is_nan() && b.is_nan() {
                return true;
            }
            if a == b {
                return true;
            }
            if !a.is_finite() || !b.is_finite() {
                return false;
            }
            let (ia, ib) = (a.to_bits() as i64, b.to_bits() as i64);
            if (ia < 0) != (ib < 0) {
                return a.abs() < 1e-300 && b.abs() < 1e-300;
            }
            (ia - ib).unsigned_abs() <= ulps
        }
        let mut rng = Rng(0x0123_4567_89ab_cdef);
        for _ in 0..20000 {
            let x = rng.f64_range(-1e5, 1e5);
            assert!(close(sin(x), x.sin(), 2), "sin({x:?})");
            assert!(close(cos(x), x.cos(), 2), "cos({x:?})");
            assert!(close(tan(x), x.tan(), 4), "tan({x:?})");
            assert!(close(atan(x), x.atan(), 2), "atan({x:?})");
            let u = rng.f64_range(-1.0, 1.0);
            assert!(close(asin(u), u.asin(), 2), "asin({u:?})");
            assert!(close(acos(u), u.acos(), 2), "acos({u:?})");
            let y = rng.f64_range(-1e5, 1e5);
            assert!(close(atan2(y, x), y.atan2(x), 2), "atan2({y:?},{x:?})");
            let b = rng.f64_range(0.0, 100.0);
            let e = rng.f64_range(-20.0, 20.0);
            assert!(close(pow(b, e), b.powf(e), 4), "pow({b:?},{e:?})");
        }
        // Huge-argument reduction (the ufbxm_kernel_rem_pio2 path).
        for &x in [1e18f64, -1e18, 1e22, 123456789.0e10, 6.283185307179586e15].iter() {
            assert!(close(sin(x), x.sin(), 4), "sin({x:?})");
            assert!(close(cos(x), x.cos(), 4), "cos({x:?})");
        }
        // Special values.
        assert!(sin(f64::NAN).is_nan());
        assert!(cos(f64::INFINITY).is_nan());
        assert!(tan(f64::NEG_INFINITY).is_nan());
        assert!(asin(2.0).is_nan());
        assert!(acos(-2.0).is_nan());
        assert_eq!(pow(2.0, 10.0), 1024.0);
        assert_eq!(pow(0.0, 0.0), 1.0);
        assert_eq!(pow(-2.0, 3.0), -8.0);
        assert!(pow(-2.0, 0.5).is_nan());
        assert_eq!(atan2(0.0, -1.0), core::f64::consts::PI);
    }
}
