//! Port of the `// -- Binary parsing` banner section (ufbx.c:8607-…).
//!
//! PORTED: ufbx.c:8607-9402 — the whole `// -- Binary parsing` section: the
//! endian-swap layer (`ufbxi_swap_endian`, `ufbxi_swap_endian_array`,
//! `ufbxi_swap_endian_value`), the post-7000 array converter
//! (`ufbxi_binary_convert_array`), the pre-7000 multivalue array reader
//! (`ufbxi_binary_parse_multivalue_array`), the array destination allocator
//! (`ufbxi_push_array_data`), the bool post-pass
//! (`ufbxi_postprocess_bool_array`), the DEFLATE work item
//! (`ufbxi_deflate_task` / `ufbxi_deflate_task_fn`), the node record parser
//! (`ufbxi_binary_parse_node`) and the binary magic constants.
//!
//! The thread-pool branch inside `ufbxi_binary_parse_node` dispatches
//! `ufbxi_deflate_task_fn` through `ufbxi_thread_pool_create_task` /
//! `ufbxi_thread_pool_run_task` (`native::thread`).
//!
//! The big-endian path is live here (PORTING.md "Byte order"): `ufbxi_swap*`
//! materialize a byte-swapped copy in `uc->swap_arr` and hand back a pointer
//! into it, so every reader downstream stays little-endian.
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]

use core::ffi::c_void;
use core::mem::size_of;

use crate::generated::{InflateInput, InflateRetain};
use crate::native::allocator::{does_overflow, grow_array, ZERO_SIZE_BUFFER};
use crate::native::buf::{push, push_copy, push_pop, push_size, push_zero, Buf};
use crate::native::deflate::{inflate, inflate_init_retain};
use crate::native::error::{
    ufbxi_check, ufbxi_check_err, ufbxi_check_msg, ufbxi_check_return, ufbxi_fail, ufbxi_fail_err,
    Fail, EMPTY_CHAR,
};
use crate::native::hash::hash_string_check_ascii;
use crate::native::io::{
    consume_bytes, pause_progress, peek_bytes, read_bytes, read_to, resume_progress, skip_bytes,
};
use crate::native::parse::{
    array_type_size, get_read_offset, is_array_node, is_raw_string, normalize_array_type,
    update_parse_state, ArrayInfo, Context, InnerContext, Node, ParseState, Value, ValueArray,
    ValueType, ARRAY_FLAG_PAD_BEGIN, ARRAY_FLAG_RESULT, ARRAY_FLAG_TMP_BUF, MAX_NODE_DEPTH,
    MAX_NON_ARRAY_VALUES,
};
use crate::native::platform::{
    f64_to_i32, f64_to_i64, min32, read_f32, read_f64, read_i16, read_i32, read_i64, read_u32,
    read_u64, read_u8, ufbx_assert, ufbxi_dev_assert, ufbxi_unreachable,
    MIN_THREADED_DEFLATE_BYTES,
};
use crate::native::string_pool::{push_sanitized_string, push_string, push_string_place_str};
use crate::native::thread::{thread_pool_create_task, thread_pool_run_task, Task};
use crate::prelude::String;

// -- Binary parsing

// The C conversion macros paste a cast token (`(uint8_t)`, `(int32_t)`, …) or a
// function name (`ufbxi_f64_to_i32`) into `m_cast(m_expr)`. Rust has no cast
// tokens, and a closure cannot be generic over the source type, so each C cast
// operand becomes one of these `macro_rules!` appliers, passed by ident.
//
// C-parity: `(uint8_t)` applied to a float operand is undefined behavior in C
// when the value is out of range; clang -O2 on x86-64 emits a 32-bit
// `cvttsd2si`/`cvttss2si` + low-byte narrow, so the oracle build yields
// trunc(val) mod 256 for |val| < 2^31 and the integer-indefinite 0x80000000
// (low byte 0x00) for NaN / |val| >= 2^31. Per the PORTING.md bare-float-cast
// row the port uses plain `as` (saturating, NaN -> 0) — a known, accepted
// divergence in this C-UB class; do not hand-roll a truncating helper. For
// integer operands `as u8` is modulo narrowing, matching C exactly.
macro_rules! ufbxi_cast_u8 {
    ($e:expr) => {
        $e as u8
    };
}
macro_rules! ufbxi_cast_i32 {
    ($e:expr) => {
        $e as i32
    };
}
macro_rules! ufbxi_cast_i64 {
    ($e:expr) => {
        $e as i64
    };
}
macro_rules! ufbxi_cast_f32 {
    ($e:expr) => {
        $e as f32
    };
}
macro_rules! ufbxi_cast_f64 {
    ($e:expr) => {
        $e as f64
    };
}
// C: `ufbxi_f64_to_i32` / `ufbxi_f64_to_i64` as `m_cast_float`. The operand is a
// `float` in the 4-byte cases and C promotes it to `double` at the call — the
// `as f64` reproduces that promotion (a no-op for the 8-byte cases).
macro_rules! ufbxi_cast_f64_to_i32 {
    ($e:expr) => {
        f64_to_i32($e as f64)
    };
}
macro_rules! ufbxi_cast_f64_to_i64 {
    ($e:expr) => {
        f64_to_i64($e as f64)
    };
}

// ufbx.c:8609-8646 `ufbxi_swap_endian`
// C: `ufbxi_nodiscard static ufbxi_noinline char *` — returns NULL on failure.
#[inline(never)]
#[must_use]
pub(crate) unsafe fn swap_endian(
    uc: &Context,
    src: *const c_void,
    count: usize,
    elem_size: usize,
) -> *mut u8 {
    ufbxi_dev_assert!(elem_size > 1);
    let total_size = count.wrapping_mul(elem_size);
    ufbxi_check_return!(
        uc,
        !does_overflow(total_size, count, elem_size),
        core::ptr::null_mut(),
        "!ufbxi_does_overflow(total_size, count, elem_size)"
    );
    if (*uc.get()).swap_arr_size < total_size {
        ufbxi_check_return!(
            uc,
            grow_array(
                uc.ator_tmp(),
                &mut (*uc.get()).swap_arr,
                &mut (*uc.get()).swap_arr_size,
                total_size
            ),
            core::ptr::null_mut(),
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->swap_arr)), (&uc->swap_arr), (&uc->swap_arr_size), (total_size))"
        );
    }
    let dst: *mut u8 = (*uc.get()).swap_arr;
    let mut d: *mut u8 = dst;

    let mut s: *const u8 = src as *const u8;
    match elem_size {
        2 => {
            // C: `ufbxi_nounroll` — optimizer pragma, no Rust analogue.
            for _i in 0..count {
                *d.add(0) = *s.add(1);
                *d.add(1) = *s.add(0);
                d = d.add(2);
                s = s.add(2);
            }
        }
        4 => {
            // C: `ufbxi_nounroll` — optimizer pragma, no Rust analogue.
            for _i in 0..count {
                *d.add(0) = *s.add(3);
                *d.add(1) = *s.add(2);
                *d.add(2) = *s.add(1);
                *d.add(3) = *s.add(0);
                d = d.add(4);
                s = s.add(4);
            }
        }
        8 => {
            // C: `ufbxi_nounroll` — optimizer pragma, no Rust analogue.
            for _i in 0..count {
                *d.add(0) = *s.add(7);
                *d.add(1) = *s.add(6);
                *d.add(2) = *s.add(5);
                *d.add(3) = *s.add(4);
                *d.add(4) = *s.add(3);
                *d.add(5) = *s.add(2);
                *d.add(6) = *s.add(1);
                *d.add(7) = *s.add(0);
                d = d.add(8);
                s = s.add(8);
            }
        }
        _ => {
            ufbxi_unreachable!("Bad endian swap size");
        }
    }

    dst
}

// Swap the endianness of an array typed with a lowercase letter
// ufbx.c:8648-8656 `ufbxi_swap_endian_array`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn swap_endian_array(
    uc: &Context,
    src: *const c_void,
    count: usize,
    type_: u8,
) -> *const u8 {
    match type_ {
        b'i' | b'f' => swap_endian(uc, src, count, 4),
        b'l' | b'd' => swap_endian(uc, src, count, 8),
        _ => src as *const u8,
    }
}

// Swap the endianness of a single value (shallow, swaps string/array header words)
// ufbx.c:8658-8670 `ufbxi_swap_endian_value`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn swap_endian_value(uc: &Context, src: *const c_void, type_: u8) -> *const u8 {
    match type_ {
        b'Y' => swap_endian(uc, src, 1, 2),
        b'I' | b'F' => swap_endian(uc, src, 1, 4),
        b'L' | b'D' => swap_endian(uc, src, 1, 8),
        b'S' | b'R' => swap_endian(uc, src, 1, 4),
        b'i' | b'l' | b'f' | b'd' | b'b' => swap_endian(uc, src, 3, 4),
        _ => src as *const u8,
    }
}

// Read and convert a post-7000 FBX data array into a different format. `src_type` may be equal to `dst_type`
// if the platform is not binary compatible with the FBX data representation.
// ufbx.c:8672-8765 `ufbxi_binary_convert_array`
// C returns `int`: 1 on success, 0 on failure. The `default:` arms return 0
// WITHOUT recording an error (and with `maybe_uc == NULL` so can they not) —
// `Err(Fail)` carries no payload, so that maps directly.
#[inline(never)]
pub(crate) unsafe fn binary_convert_array(
    maybe_uc: *mut InnerContext,
    src_type: u8,
    dst_type: u8,
    mut src: *const c_void,
    dst: *mut c_void,
    size: usize,
) -> Result<(), Fail> {
    // TODO: We might want to use the slow path if the machine float/double doesn't match IEEE 754!
    // Convert commented out lines under some `#if UFBX_NON_IEE754` define or something.
    if src_type == dst_type {
        ufbx_assert!(
            !maybe_uc.is_null() && (*maybe_uc).file_big_endian != (*maybe_uc).local_big_endian
        );
        src = swap_endian_array(Context::from_ptr(maybe_uc), src, size, src_type) as *const c_void;
        ufbxi_check_err!(&mut (*maybe_uc).error, !src.is_null(), "src");
        core::ptr::copy_nonoverlapping(
            src as *const u8,
            dst as *mut u8,
            size * array_type_size(dst_type),
        );
        return Ok(());
    }

    if !maybe_uc.is_null() && (*maybe_uc).file_big_endian {
        src = swap_endian_array(Context::from_ptr(maybe_uc), src, size, src_type) as *const c_void;
        ufbxi_check_err!(&mut (*maybe_uc).error, !src.is_null(), "src");
    }

    // C: the two `#define`s below live inside the `switch`; in Rust they must
    // be declared before use. Defined here (inside the fn) so that hygiene lets
    // their bodies see the locals `src`, `dst` and `size`, exactly as the C
    // macros see the enclosing scope. `m_expr` reads the loop cursor, so the
    // cursor's identifier is threaded through as `$val`.
    macro_rules! ufbxi_convert_loop_fast {
        ($m_dst:ty, $m_cast:ident, $m_size:expr, $val:ident, $m_expr:expr) => {{
            let mut $val: *const u8 = src as *const u8;
            let val_end: *const u8 = $val.add(size * $m_size);
            let mut d: *mut $m_dst = dst as *mut $m_dst;
            while $val != val_end {
                *d = $m_cast!($m_expr);
                d = d.add(1);
                $val = $val.add($m_size);
            }
        }};
    }

    // C: identical to `ufbxi_convert_loop_fast` apart from `ufbxi_nounroll` on
    // the loop — an optimizer pragma with no Rust analogue. Kept as a separate
    // macro for call-site diff parity.
    macro_rules! ufbxi_convert_loop_slow {
        ($m_dst:ty, $m_cast:ident, $m_size:expr, $val:ident, $m_expr:expr) => {{
            let mut $val: *const u8 = src as *const u8;
            let val_end: *const u8 = $val.add(size * $m_size);
            let mut d: *mut $m_dst = dst as *mut $m_dst;
            while $val != val_end {
                *d = $m_cast!($m_expr);
                d = d.add(1);
                $val = $val.add($m_size);
            }
        }};
    }

    match dst_type {
        b'c' => match src_type {
            // case 'c': ufbxi_convert_loop_fast(char, (char), 1, *val != 0); break;
            b'i' => ufbxi_convert_loop_slow!(u8, ufbxi_cast_u8, 4, val, read_i32(val)),
            b'l' => ufbxi_convert_loop_slow!(u8, ufbxi_cast_u8, 8, val, read_i64(val)),
            b'f' => ufbxi_convert_loop_slow!(u8, ufbxi_cast_u8, 4, val, read_f32(val)),
            b'd' => ufbxi_convert_loop_slow!(u8, ufbxi_cast_u8, 8, val, read_f64(val)),
            _ => {
                if !maybe_uc.is_null() {
                    ufbxi_fail_err!(&mut (*maybe_uc).error, "Bad array source type");
                }
                return Err(Fail);
            }
        },

        b'i' => match src_type {
            // C-parity: `*val` is `char`, which is SIGNED on this port's targets
            // (x86-64 SysV and Apple AArch64) — the read sign-extends. This is the
            // documented exception to the "`char` → `u8` everywhere" storage rule
            // (PORTING.md "Integer semantics", the `char` (value) row): do NOT
            // "fix" these five `*const i8` derefs back to `u8` on an upstream sync,
            // it changes every source byte >= 0x80.
            b'c' => ufbxi_convert_loop_slow!(i32, ufbxi_cast_i32, 1, val, *(val as *const i8)),
            // case 'i': ufbxi_convert_loop_slow(int32_t, (int32_t), 4, ufbxi_read_i32(val)); break;
            b'l' => ufbxi_convert_loop_slow!(i32, ufbxi_cast_i32, 8, val, read_i64(val)),
            b'f' => ufbxi_convert_loop_slow!(i32, ufbxi_cast_f64_to_i32, 4, val, read_f32(val)),
            b'd' => ufbxi_convert_loop_slow!(i32, ufbxi_cast_f64_to_i32, 8, val, read_f64(val)),
            _ => {
                if !maybe_uc.is_null() {
                    ufbxi_fail_err!(&mut (*maybe_uc).error, "Bad array source type");
                }
                return Err(Fail);
            }
        },

        b'l' => match src_type {
            // C-parity: signed `char` deref, see the `dst_type == 'i'` arm above.
            b'c' => ufbxi_convert_loop_slow!(i64, ufbxi_cast_i64, 1, val, *(val as *const i8)),
            b'i' => ufbxi_convert_loop_slow!(i64, ufbxi_cast_i64, 4, val, read_i32(val)),
            // case 'l': ufbxi_convert_loop_slow(int64_t, (int64_t), 8, ufbxi_read_i64(val)); break;
            b'f' => ufbxi_convert_loop_slow!(i64, ufbxi_cast_f64_to_i64, 4, val, read_f32(val)),
            b'd' => ufbxi_convert_loop_slow!(i64, ufbxi_cast_f64_to_i64, 8, val, read_f64(val)),
            _ => {
                if !maybe_uc.is_null() {
                    ufbxi_fail_err!(&mut (*maybe_uc).error, "Bad array source type");
                }
                return Err(Fail);
            }
        },

        b'f' => match src_type {
            // C-parity: signed `char` deref, see the `dst_type == 'i'` arm above.
            b'c' => ufbxi_convert_loop_slow!(f32, ufbxi_cast_f32, 1, val, *(val as *const i8)),
            b'i' => ufbxi_convert_loop_slow!(f32, ufbxi_cast_f32, 4, val, read_i32(val)),
            b'l' => ufbxi_convert_loop_slow!(f32, ufbxi_cast_f32, 8, val, read_i64(val)),
            // case 'f': ufbxi_convert_loop_slow(float, (float), 4, ufbxi_read_f32(val)); break;
            b'd' => ufbxi_convert_loop_fast!(f32, ufbxi_cast_f32, 8, val, read_f64(val)),
            _ => {
                if !maybe_uc.is_null() {
                    ufbxi_fail_err!(&mut (*maybe_uc).error, "Bad array source type");
                }
                return Err(Fail);
            }
        },

        b'd' => match src_type {
            // C-parity: signed `char` deref, see the `dst_type == 'i'` arm above.
            b'c' => ufbxi_convert_loop_slow!(f64, ufbxi_cast_f64, 1, val, *(val as *const i8)),
            b'i' => ufbxi_convert_loop_slow!(f64, ufbxi_cast_f64, 4, val, read_i32(val)),
            b'l' => ufbxi_convert_loop_slow!(f64, ufbxi_cast_f64, 8, val, read_i64(val)),
            b'f' => ufbxi_convert_loop_fast!(f64, ufbxi_cast_f64, 4, val, read_f32(val)),
            // case 'd': ufbxi_convert_loop_slow(double, (double), 8, ufbxi_read_f64(val)); break;
            _ => {
                if !maybe_uc.is_null() {
                    ufbxi_fail_err!(&mut (*maybe_uc).error, "Bad array source type");
                }
                return Err(Fail);
            }
        },

        _ => return Err(Fail),
    }

    Ok(())
}

// Read pre-7000 separate properties as an array.
// ufbx.c:8767-8873 `ufbxi_binary_parse_multivalue_array`
#[inline(never)]
pub(crate) unsafe fn binary_parse_multivalue_array(
    uc: &Context,
    dst_type: u8,
    dst: *mut c_void,
    size: usize,
    tmp_buf: *mut Buf,
) -> Result<(), Fail> {
    if size == 0 {
        return Ok(());
    }
    let mut val: *const u8;
    let mut val_size: usize;

    let file_big_endian: bool = (*uc.get()).file_big_endian;

    // String array special case
    if dst_type == b's' || dst_type == b'S' || dst_type == b'C' {
        let raw: bool = dst_type == b's';
        let mut d: *mut String = dst as *mut String;
        for _i in 0..size {
            val = peek_bytes(uc, 13);
            ufbxi_check!(uc, !val.is_null(), "val");
            let type_: u8 = *val;
            val = val.add(1);
            ufbxi_check!(
                uc,
                type_ == b'S' || type_ == b'R',
                "type == 'S' || type == 'R'"
            );
            if file_big_endian {
                val = swap_endian_value(uc, val as *const c_void, type_);
                ufbxi_check!(uc, !val.is_null(), "val");
            }
            let len: usize = read_u32(val) as usize;
            consume_bytes(uc, 5);
            (*d).data = read_bytes(uc, len);
            (*d).length = len;
            ufbxi_check!(uc, !(*d).data.is_null(), "d->data");
            if dst_type == b'C' {
                let buf: *mut Buf = if size == 1 || (*uc.get()).opts.retain_dom {
                    &mut (*uc.get()).result
                } else {
                    tmp_buf
                };
                (*d).data = push_copy::<u8>(buf, len, (*d).data);
                ufbxi_check!(uc, !(*d).data.is_null(), "d->data");
            } else {
                push_string_place_str(&mut (*uc.get()).string_pool, d, raw)?;
            }
            d = d.add(1);
        }
        return Ok(());
    }

    // Optimize a couple of common cases
    let mut base: usize = 0;

    // C: `#define ufbxi_convert_parse_fast` sits above the string special case;
    // in Rust the macro body has to see the local `base` (and `val`), so the
    // definition moves down to just before its first use. Same expansion.
    macro_rules! ufbxi_convert_parse_fast {
        ($m_dst:ty, $m_type:expr, $m_expr:expr) => {{
            let mut d: *mut $m_dst = dst as *mut $m_dst;
            while base < size {
                val = peek_bytes(uc, 13);
                ufbxi_check!(uc, !val.is_null(), "val");
                if *val != $m_type {
                    break;
                }
                val = val.add(1);
                *d = ($m_expr) as $m_dst;
                d = d.add(1);
                consume_bytes(uc, 1 + size_of::<$m_dst>());
                base += 1;
            }
        }};
    }

    if !file_big_endian {
        match dst_type {
            b'i' => ufbxi_convert_parse_fast!(i32, b'I', read_i32(val)),
            b'l' => ufbxi_convert_parse_fast!(i64, b'L', read_i64(val)),
            b'f' => ufbxi_convert_parse_fast!(f32, b'F', read_f32(val)),
            b'd' => ufbxi_convert_parse_fast!(f64, b'D', read_f64(val)),
            _ => {} // Fallthrough to rest
        }

        // Early return if we handled everything
        if base == size {
            return Ok(());
        }
    }

    // C: `#define ufbxi_convert_parse(m_cast, m_size, m_expr)` is expanded
    // inline in each arm below (`*d++ = m_cast(m_expr); val_size = m_size + 1;`)
    // — a nested `macro_rules!` cannot be defined from within another one.
    macro_rules! ufbxi_convert_parse_switch {
        ($m_dst:ty, $m_cast_int:ident, $m_cast_float:ident) => {{
            let mut d: *mut $m_dst = (dst as *mut $m_dst).add(base);
            let mut i: usize = base;
            while i < size {
                val = peek_bytes(uc, 13);
                ufbxi_check!(uc, !val.is_null(), "val");
                let type_: u8 = *val;
                val = val.add(1);
                if file_big_endian {
                    val = swap_endian_value(uc, val as *const c_void, type_);
                    ufbxi_check!(uc, !val.is_null(), "val");
                }
                match type_ {
                    // C-parity: `*val` is `char` — SIGNED on this port's targets.
                    // Documented exception to the `u8` storage rule, see
                    // PORTING.md "Integer semantics" (the `char` (value) row).
                    b'C' | b'B' => {
                        *d = $m_cast_int!(*(val as *const i8));
                        d = d.add(1);
                        val_size = 1 + 1;
                    }
                    b'Y' => {
                        *d = $m_cast_int!(read_i16(val));
                        d = d.add(1);
                        val_size = 2 + 1;
                    }
                    b'I' => {
                        *d = $m_cast_int!(read_i32(val));
                        d = d.add(1);
                        val_size = 4 + 1;
                    }
                    b'L' => {
                        *d = $m_cast_int!(read_i64(val));
                        d = d.add(1);
                        val_size = 8 + 1;
                    }
                    b'F' => {
                        *d = $m_cast_float!(read_f32(val));
                        d = d.add(1);
                        val_size = 4 + 1;
                    }
                    b'D' => {
                        *d = $m_cast_float!(read_f64(val));
                        d = d.add(1);
                        val_size = 8 + 1;
                    }
                    _ => ufbxi_fail!(uc, "Bad multivalue array type"),
                }
                consume_bytes(uc, val_size);
                i += 1;
            }
        }};
    }

    match dst_type {
        b'c' => ufbxi_convert_parse_switch!(u8, ufbxi_cast_u8, ufbxi_cast_u8),
        b'i' => ufbxi_convert_parse_switch!(i32, ufbxi_cast_i32, ufbxi_cast_f64_to_i32),
        b'l' => ufbxi_convert_parse_switch!(i64, ufbxi_cast_i64, ufbxi_cast_f64_to_i64),
        b'f' => ufbxi_convert_parse_switch!(f32, ufbxi_cast_f32, ufbxi_cast_f32),
        b'd' => ufbxi_convert_parse_switch!(f64, ufbxi_cast_f64, ufbxi_cast_f64),

        _ => return Err(Fail),
    }

    Ok(())
}

// ufbx.c:8875-8895 `ufbxi_push_array_data`
// C: `ufbxi_nodiscard ufbxi_noinline static void *` — returns NULL on failure.
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_array_data(
    uc: &Context,
    info: *const ArrayInfo,
    mut size: usize,
    tmp_buf: *mut Buf,
) -> *mut c_void {
    let elem_size: usize = array_type_size((*info).type_);
    // C: `uint32_t flags = info->flags;` (widened from the `uint8_t` field).
    let flags: u32 = (*info).flags as u32;
    if flags & ARRAY_FLAG_PAD_BEGIN as u32 != 0 {
        size += 4;
    }

    // The array may be pushed either to the result or temporary buffer depending
    // if it's already in the right format
    let mut arr_buf: *mut Buf = tmp_buf;
    if flags & ARRAY_FLAG_RESULT as u32 != 0 {
        arr_buf = &mut (*uc.get()).result;
    } else if flags & ARRAY_FLAG_TMP_BUF as u32 != 0 {
        arr_buf = &mut (*uc.get()).tmp;
    }
    let mut data: *mut u8 = push_size(arr_buf, elem_size, size) as *mut u8;
    ufbxi_check_return!(uc, !data.is_null(), core::ptr::null_mut(), "data");

    if flags & ARRAY_FLAG_PAD_BEGIN as u32 != 0 {
        core::ptr::write_bytes(data, 0, elem_size * 4);
        data = data.add(elem_size * 4);
    }

    data as *mut c_void
}

// ufbx.c:8897-8902 `ufbxi_postprocess_bool_array`
#[inline(never)]
pub(crate) unsafe fn postprocess_bool_array(data: *mut u8, size: usize) {
    // C: `ufbxi_for(char, b, (char*)data, size)`
    let mut b: *mut u8 = data;
    let b_end: *mut u8 = data.add(size);
    while b != b_end {
        *b = (*b != 0) as u8;
        b = b.add(1);
    }
}

// ufbx.c:8904-8915 `ufbxi_deflate_task`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct DeflateTask {
    pub encoded_size: usize,
    pub src_elem_size: usize,
    pub array_size: usize,
    pub src_type: u8,
    pub dst_type: u8,
    pub arr_type: u8,
    pub encoded_data: *const c_void,
    pub decoded_data: *mut c_void,
    pub dst_data: *mut c_void,
    pub inflate_retain: *mut InflateRetain,
}

// ufbx.c:8917-8962 `ufbxi_deflate_task_fn`
// `ufbxi_task_fn` run from the thread pool; dispatched by the threading branch
// in `ufbxi_binary_parse_node` below.
pub(crate) unsafe extern "C" fn deflate_task_fn(task: *mut Task) -> bool {
    let t: *mut DeflateTask = (*task).data as *mut DeflateTask;

    let mut input = core::mem::MaybeUninit::<InflateInput>::uninit(); // ufbxi_uninit
    let input: *mut InflateInput = input.as_mut_ptr();
    (*input).total_size = (*t).encoded_size;
    (*input).data = (*t).encoded_data;
    (*input).data_size = (*t).encoded_size;
    (*input).no_header = false;
    (*input).no_checksum = false;
    (*input).internal_fast_bits = 0;
    (*input).progress_cb.fn_ = None;
    (*input).progress_cb.user = core::ptr::null_mut();
    (*input).progress_size_before = 0;
    (*input).progress_size_after = 0;
    (*input).progress_interval_hint = 0;
    (*input).buffer = core::ptr::null_mut();
    (*input).buffer_size = 0;
    (*input).read_fn = None;
    (*input).read_user = core::ptr::null_mut();

    let decoded_data_size: usize = (*t).src_elem_size * (*t).array_size;
    let res: isize = inflate(
        (*t).decoded_data,
        decoded_data_size,
        input,
        (*t).inflate_retain,
    );
    if res == -28 {
        (*task).error = "Cancelled\0".as_ptr();
        return false;
    } else if res != decoded_data_size as isize {
        (*task).error = "Bad DEFLATE data\0".as_ptr();
        return false;
    }

    if (*t).decoded_data != (*t).dst_data {
        let ok = binary_convert_array(
            core::ptr::null_mut(),
            (*t).src_type,
            (*t).dst_type,
            (*t).decoded_data,
            (*t).dst_data,
            (*t).array_size,
        );
        if ok.is_err() {
            (*task).error = "Failed to convert array\0".as_ptr();
            return false;
        }
    }

    if (*t).arr_type == b'b' {
        postprocess_bool_array((*t).dst_data as *mut u8, (*t).array_size);
    }

    true
}

// Recursion limited by check at the start
// ufbx.c:8964-9398 `ufbxi_binary_parse_node`
// `ufbxi_recursive_function(int, ufbxi_binary_parse_node, ..., UFBXI_MAX_NODE_DEPTH + 1, ...)`
// (ufbx.c:8965-8966): under regression, a thread-local depth guard wraps the
// recursive body (which C splits into `ufbxi_binary_parse_node_rec`);
// otherwise the macro is empty and the wrapper is a plain call.
#[inline(never)]
pub(crate) unsafe fn binary_parse_node(
    uc: &Context,
    depth: u32,
    parent_state: ParseState,
    p_end: *mut bool,
    tmp_buf: *mut Buf,
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
        let ret = binary_parse_node_rec(uc, depth, parent_state, p_end, tmp_buf, recursive);
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        ret
    }
    #[cfg(not(feature = "regression"))]
    {
        binary_parse_node_rec(uc, depth, parent_state, p_end, tmp_buf, recursive)
    }
}

#[inline(never)]
unsafe fn binary_parse_node_rec(
    uc: &Context,
    depth: u32,
    parent_state: ParseState,
    p_end: *mut bool,
    tmp_buf: *mut Buf,
    recursive: bool,
) -> Result<(), Fail> {
    // https://code.blender.org/2013/08/fbx-binary-file-format-specification
    // Parse an FBX document node in the binary format
    ufbxi_check!(uc, depth < MAX_NODE_DEPTH, "depth < UFBXI_MAX_NODE_DEPTH");

    // Parse the node header, post-7500 versions use 64-bit values for most
    // header fields.
    let end_offset: u64;
    let num_values64: u64;
    let values_len: u64;
    let name_len: u8;
    let header_size: usize = if uc.version() >= 7500 { 25 } else { 13 };
    let header: *const u8 = read_bytes(uc, header_size);
    let mut header_words: *const u8 = header;
    ufbxi_check!(uc, !header.is_null(), "header");
    if uc.version() >= 7500 {
        if (*uc.get()).file_big_endian {
            header_words = swap_endian(uc, header_words as *const c_void, 3, 8);
            ufbxi_check!(uc, !header_words.is_null(), "header_words");
        }
        end_offset = read_u64(header_words.add(0));
        num_values64 = read_u64(header_words.add(8));
        values_len = read_u64(header_words.add(16));
        name_len = read_u8(header.add(24));
    } else {
        if (*uc.get()).file_big_endian {
            header_words = swap_endian(uc, header_words as *const c_void, 3, 4);
            ufbxi_check!(uc, !header_words.is_null(), "header_words");
        }
        end_offset = read_u32(header_words.add(0)) as u64;
        num_values64 = read_u32(header_words.add(4)) as u64;
        values_len = read_u32(header_words.add(8)) as u64;
        name_len = read_u8(header.add(12));
    }

    ufbxi_check!(
        uc,
        num_values64 <= u32::MAX as u64,
        "num_values64 <= UINT32_MAX"
    );
    let mut num_values: u32 = num_values64 as u32;

    // If `end_offset` and `name_len` is zero we treat as the node as a NULL-sentinel
    // that terminates a node list.
    if end_offset == 0 && name_len == 0 {
        *p_end = true;
        return Ok(());
    }

    // Update estimated end offset if possible
    if end_offset > (*uc.get()).progress_bytes_total {
        (*uc.get()).progress_bytes_total = end_offset;
    }

    // Push the parsed node into the `tmp_stack` buffer, the nodes will be popped by
    // calling code after its done parsing all of it's children.
    let node: *mut Node = push_zero::<Node>(&mut (*uc.get()).tmp_stack, 1);
    ufbxi_check!(uc, !node.is_null(), "node");

    // Parse and intern the name to the string pool.
    let mut name: *const u8 = read_bytes(uc, name_len as usize);
    ufbxi_check!(uc, !name.is_null(), "name");
    name = push_string(
        &mut (*uc.get()).string_pool,
        name,
        name_len as usize,
        core::ptr::null_mut(),
        true,
    );
    ufbxi_check!(uc, !name.is_null(), "name");
    (*node).name_len = name_len;
    (*node).name = name;

    let values_end_offset: u64 = get_read_offset(uc).wrapping_add(values_len);

    // Check if the values of the node we're parsing currently should be
    // treated as an array.
    let mut arr_info = core::mem::MaybeUninit::<ArrayInfo>::uninit();
    let arr_info: *mut ArrayInfo = arr_info.as_mut_ptr();
    if is_array_node(uc, parent_state, name, arr_info) {
        // Normalize the array type (eg. 'r' to 'f'/'d' depending on the build)
        // and get the per-element size of the array.
        // Boolean arrays 'b' are normalized to 'c' as they are postprocessed
        // below based on `arr_info.type`.
        let dst_type: u8 = normalize_array_type((*arr_info).type_, b'c');

        let arr: *mut ValueArray = push::<ValueArray>(tmp_buf, 1);
        ufbxi_check!(uc, !arr.is_null(), "arr");

        (*node).value_type_mask = ValueType::Array as u16;
        (*node).content.array = arr;
        (*arr).type_ = normalize_array_type((*arr_info).type_, b'b');

        // Peek the first bytes of the array. We can always look at least 13 bytes
        // ahead safely as valid FBX files must end in a 13/25 byte NULL record.
        let data: *const u8 = peek_bytes(uc, 13);
        ufbxi_check!(uc, !data.is_null(), "data");

        // Check if the data type is one of the explicit array types (post-7000).
        // Otherwise we form the array by concatenating all the normal values of the
        // node (pre-7000)
        let mut c: u8 = *data.add(0);

        // HACK: Override the "type" if either the array is empty or we want to
        // specifically ignore the contents.
        if num_values == 0 {
            c = b'0';
        }
        if dst_type == b'-' {
            c = b'-';
        }

        let mut deferred: bool = false;

        if c == b'c' || c == b'b' || c == b'i' || c == b'l' || c == b'f' || c == b'd' {
            let mut arr_words: *const u8 = data.add(1);
            if (*uc.get()).file_big_endian {
                arr_words = swap_endian(uc, arr_words as *const c_void, 3, 4);
                ufbxi_check!(uc, !arr_words.is_null(), "arr_words");
            }

            // Parse the array header from the prefix we already peeked above.
            let mut src_type: u8 = *data.add(0);
            let size: u32 = read_u32(arr_words.add(0));
            let encoding: u32 = read_u32(arr_words.add(4));
            let encoded_size: u32 = read_u32(arr_words.add(8));
            consume_bytes(uc, 13);

            // Normalize the source type as well, but don't convert UFBX-specific
            // 'r' to 'f'/'d', but fail later instead.
            if src_type != b'r' {
                src_type = normalize_array_type(src_type, b'c');
            }
            let src_elem_size: usize = array_type_size(src_type);
            let decoded_data_size: usize = src_elem_size.wrapping_mul(size as usize);

            // Allocate `size` elements for the array.
            let arr_data: *mut u8 =
                push_array_data(uc, arr_info, size as usize, tmp_buf) as *mut u8;
            ufbxi_check!(uc, !arr_data.is_null(), "arr_data");

            let arr_begin: u64 = get_read_offset(uc);
            ufbxi_check!(
                uc,
                u64::MAX - encoded_size as u64 > arr_begin,
                "UINT64_MAX - encoded_size > arr_begin"
            );
            let arr_end: u64 = arr_begin.wrapping_add(encoded_size as u64);
            if arr_end > (*uc.get()).progress_bytes_total {
                (*uc.get()).progress_bytes_total = arr_end;
            }

            // Threading
            if (*uc.get()).parse_threaded
                && encoding == 1
                && encoded_size as usize >= MIN_THREADED_DEFLATE_BYTES
                && !(*uc.get()).file_big_endian
                && !(*uc.get()).local_big_endian
            {
                let task: *mut Task =
                    thread_pool_create_task(&raw mut (*uc.get()).thread_pool, deflate_task_fn);
                if !task.is_null() {
                    let t: *mut DeflateTask = push_zero::<DeflateTask>(tmp_buf, 1);
                    ufbxi_check!(uc, !t.is_null(), "t");

                    inflate_init_retain((*uc.get()).inflate_retain);

                    (*t).src_elem_size = src_elem_size;
                    (*t).encoded_size = encoded_size as usize;
                    (*t).array_size = size as usize;
                    (*t).src_type = src_type;
                    (*t).dst_type = dst_type;
                    (*t).arr_type = (*arr).type_;
                    (*t).dst_data = arr_data as *mut c_void;
                    (*t).inflate_retain = (*uc.get()).inflate_retain;

                    if uc.read_fn().is_none() {
                        // From memory, no need to copy
                        (*t).encoded_data = uc.data() as *const c_void;
                    } else {
                        let encoded_data: *mut c_void =
                            push::<u8>(tmp_buf, encoded_size as usize) as *mut c_void;
                        ufbxi_check!(uc, !encoded_data.is_null(), "encoded_data");
                        read_to(uc, encoded_data, encoded_size as usize)?;
                        (*t).encoded_data = encoded_data;
                    }

                    if src_type != dst_type {
                        (*t).decoded_data = push_size(tmp_buf, src_elem_size, size as usize);
                        ufbxi_check!(uc, !(*t).decoded_data.is_null(), "t->decoded_data");
                    } else {
                        (*t).decoded_data = arr_data as *mut c_void;
                    }

                    (*task).data = t as *mut c_void;
                    thread_pool_run_task(&raw mut (*uc.get()).thread_pool, task);
                    deferred = true;
                }
            }

            // If the source and destination types are equal and our build is binary-compatible
            // with the FBX format we can read the decoded data directly into the array buffer.
            // Otherwise we need a temporary buffer to decode the array into before conversion.
            let mut decoded_data: *mut c_void = arr_data as *mut c_void;
            if !deferred
                && (src_type != dst_type
                    || (*uc.get()).local_big_endian != (*uc.get()).file_big_endian)
            {
                ufbxi_check!(
                    uc,
                    grow_array(
                        uc.ator_tmp(),
                        &mut (*uc.get()).tmp_arr,
                        &mut (*uc.get()).tmp_arr_size,
                        decoded_data_size
                    ),
                    "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (decoded_data_size))"
                );
                decoded_data = (*uc.get()).tmp_arr as *mut c_void;
            }

            if deferred {
                // Nop
            } else if encoding == 0 {
                // Encoding 0: Plain binary data.
                ufbxi_check!(
                    uc,
                    encoded_size as usize == decoded_data_size,
                    "encoded_size == decoded_data_size"
                );

                // If the array is contained in the current read buffer and we need to convert
                // the data anyway we can use the read buffer as the decoded array source, otherwise
                // do a plain byte copy to the array/conversion buffer.
                if uc.yield_size().wrapping_add(uc.data_size()) >= encoded_size as usize
                    && decoded_data != arr_data as *mut c_void
                {
                    // Yield right after this if we crossed the yield threshold
                    if encoded_size as usize > uc.yield_size() {
                        uc.set_data_size(uc.data_size().wrapping_add(uc.yield_size()));
                        uc.set_yield_size(encoded_size as usize);
                        uc.set_data_size(uc.data_size().wrapping_sub(uc.yield_size()));
                    }

                    decoded_data = uc.data() as *mut c_void;
                    consume_bytes(uc, encoded_size as usize);
                } else {
                    read_to(uc, decoded_data, encoded_size as usize)?;
                }
            } else if encoding == 1 {
                // Encoding 1: DEFLATE

                pause_progress(uc);

                // Inflate the data from the user-provided IO buffer / read callbacks
                let mut input = core::mem::MaybeUninit::<InflateInput>::uninit();
                let input: *mut InflateInput = input.as_mut_ptr();
                (*input).total_size = encoded_size as usize;
                (*input).data = uc.data() as *const c_void;
                (*input).data_size = uc.data_size();
                (*input).no_header = false;
                (*input).no_checksum = false;
                (*input).internal_fast_bits = 0;

                if (*uc.get()).opts.progress_cb.fn_.is_some() {
                    (*input).progress_cb = (*uc.get()).opts.progress_cb;
                    (*input).progress_size_before = arr_begin;
                    (*input).progress_size_after =
                        (*uc.get()).progress_bytes_total.wrapping_sub(arr_end);
                    (*input).progress_interval_hint = uc.progress_interval() as u64;
                } else {
                    (*input).progress_cb.fn_ = None;
                    (*input).progress_cb.user = core::ptr::null_mut();
                    (*input).progress_size_before = 0;
                    (*input).progress_size_after = 0;
                    (*input).progress_interval_hint = 0;
                }

                // If the encoded array is larger than the data we have currently buffered
                // we need to allow `ufbx_inflate()` to read from the IO callback. We can
                // let `ufbx_inflate()` freely clobber our `read_buffer` as all the data
                // in the buffer will be consumed. `ufbx_inflate()` always reads exactly
                // the amount of bytes needed so we can continue reading from `read_fn` as
                // usual (given that we clear the `uc->data/_size` buffer below).
                // NOTE: We _cannot_ share `read_buffer` if we plan to read later from it
                // as `ufbx_inflate()` overwrites parts of it with zeroes.
                if encoded_size as usize > (*input).data_size {
                    (*input).buffer = uc.read_buffer() as *mut c_void;
                    (*input).buffer_size = (*uc.get()).read_buffer_size;
                    (*input).read_fn = uc.read_fn();
                    (*input).read_user = uc.read_user();
                    (*uc.get()).data_offset = (*uc.get()).data_offset.wrapping_add(
                        (encoded_size as usize).wrapping_sub((*input).data_size) as u64,
                    );
                    uc.set_data(uc.data().add((*input).data_size));
                    uc.set_data_size(0);
                } else {
                    (*input).buffer = core::ptr::null_mut();
                    (*input).buffer_size = 0;
                    (*input).read_fn = None;
                    (*input).read_user = core::ptr::null_mut();
                    uc.set_data(uc.data().add(encoded_size as usize));
                    uc.set_data_size(uc.data_size().wrapping_sub(encoded_size as usize));
                    resume_progress(uc)?;
                }

                let res: isize = inflate(
                    decoded_data,
                    decoded_data_size,
                    input,
                    (*uc.get()).inflate_retain,
                );
                ufbxi_check_msg!(uc, res != -28, "Cancelled");
                ufbxi_check_msg!(
                    uc,
                    res == decoded_data_size as isize,
                    "Bad DEFLATE data",
                    "res == (ptrdiff_t)decoded_data_size"
                );
            } else {
                ufbxi_fail!(uc, "Bad array encoding");
            }

            // Convert the decoded array if necessary.
            if !deferred && decoded_data != arr_data as *mut c_void {
                binary_convert_array(
                    uc.get(),
                    src_type,
                    dst_type,
                    decoded_data,
                    arr_data as *mut c_void,
                    size as usize,
                )?;
            }

            (*arr).data = arr_data as *mut c_void;
            (*arr).size = size as usize;
        } else if c == b'0' || c == b'-' {
            // Ignore the array
            (*arr).type_ = if c == b'-' { b'-' } else { dst_type };
            (*arr).data = ZERO_SIZE_BUFFER.as_ptr().add(32) as *mut c_void;
            (*arr).size = 0;
        } else {
            // Allocate `num_values` elements for the array and parse single values into it.
            let arr_data: *mut u8 =
                push_array_data(uc, arr_info, num_values as usize, tmp_buf) as *mut u8;
            ufbxi_check!(uc, !arr_data.is_null(), "arr_data");
            binary_parse_multivalue_array(
                uc,
                dst_type,
                arr_data as *mut c_void,
                num_values as usize,
                tmp_buf,
            )?;
            (*arr).data = arr_data as *mut c_void;
            (*arr).size = num_values as usize;
        }

        // Post-process boolean arrays
        if !deferred && (*arr_info).type_ == b'b' {
            postprocess_bool_array((*arr).data as *mut u8, (*arr).size);
        }
    } else {
        // Parse up to UFBXI_MAX_NON_ARRAY_VALUES as plain values
        num_values = min32(num_values, MAX_NON_ARRAY_VALUES as u32);
        let vals: *mut Value = push::<Value>(tmp_buf, num_values as usize);
        ufbxi_check!(uc, !vals.is_null(), "vals");
        (*node).content.vals = vals;

        let mut type_mask: u32 = 0;
        let mut i: usize = 0;
        while i < num_values as usize {
            // The file must end in a 13/25 byte NULL record, so we can peek
            // up to 13 bytes safely here.
            let data: *const u8 = peek_bytes(uc, 13);
            ufbxi_check!(uc, !data.is_null(), "data");

            let mut value: *const u8 = data.add(1);

            let type_: u8 = *data.add(0);
            if (*uc.get()).file_big_endian {
                value = swap_endian_value(uc, value as *const c_void, type_);
                ufbxi_check!(uc, !value.is_null(), "value");
            }

            match type_ {
                b'C' | b'B' | b'Z' => {
                    type_mask |= (ValueType::Number as u32) << (i * 2);
                    // C: `vals[i].f = (double)(vals[i].i = (int64_t)(uint8_t)value[0]);`
                    // — the inner assignment happens first, then its value is
                    // converted; decomposed per PORTING.md "Evaluation order".
                    (*vals.add(i)).num.i = *value.add(0) as i64;
                    (*vals.add(i)).num.f = (*vals.add(i)).num.i as f64;
                    consume_bytes(uc, 2);
                }

                b'Y' => {
                    type_mask |= (ValueType::Number as u32) << (i * 2);
                    (*vals.add(i)).num.i = read_i16(value) as i64;
                    (*vals.add(i)).num.f = (*vals.add(i)).num.i as f64;
                    consume_bytes(uc, 3);
                }

                b'I' => {
                    type_mask |= (ValueType::Number as u32) << (i * 2);
                    (*vals.add(i)).num.i = read_i32(value) as i64;
                    (*vals.add(i)).num.f = (*vals.add(i)).num.i as f64;
                    consume_bytes(uc, 5);
                }

                b'L' => {
                    type_mask |= (ValueType::Number as u32) << (i * 2);
                    (*vals.add(i)).num.i = read_i64(value);
                    (*vals.add(i)).num.f = (*vals.add(i)).num.i as f64;
                    consume_bytes(uc, 9);
                }

                b'F' => {
                    type_mask |= (ValueType::Number as u32) << (i * 2);
                    // C: `vals[i].i = ufbxi_f64_to_i64(vals[i].f = ufbxi_read_f32(value));`
                    // — the `float` is promoted to `double` by the assignment to
                    // the `double` member, and that value feeds `ufbxi_f64_to_i64`.
                    (*vals.add(i)).num.f = read_f32(value) as f64;
                    (*vals.add(i)).num.i = f64_to_i64((*vals.add(i)).num.f);
                    consume_bytes(uc, 5);
                }

                b'D' => {
                    type_mask |= (ValueType::Number as u32) << (i * 2);
                    (*vals.add(i)).num.f = read_f64(value);
                    (*vals.add(i)).num.i = f64_to_i64((*vals.add(i)).num.f);
                    consume_bytes(uc, 9);
                }

                b'S' | b'R' => {
                    let length: u32 = read_u32(value);
                    consume_bytes(uc, 5);
                    let str_: *const u8 = read_bytes(uc, length as usize);
                    ufbxi_check!(uc, !str_.is_null(), "str");

                    if length == 0 {
                        (*vals.add(i)).s.raw_data = EMPTY_CHAR.as_ptr();
                        (*vals.add(i)).s.raw_length = 0;
                        (*vals.add(i)).s.utf8_length = 0;
                    } else {
                        let mut non_ascii: bool = false;
                        let hash: u32 =
                            hash_string_check_ascii(str_, length as usize, &mut non_ascii);
                        let raw: bool = !non_ascii || is_raw_string(uc, parent_state, name, i);
                        push_sanitized_string(
                            &mut (*uc.get()).string_pool,
                            &mut (*vals.add(i)).s,
                            str_,
                            length as usize,
                            hash,
                            raw,
                        )?;

                        // Mark the data as invalid UTF-8
                        if non_ascii && raw {
                            (*vals.add(i)).s.utf8_length = u32::MAX;
                        }
                    }

                    type_mask |= (ValueType::String as u32) << (i * 2);
                }

                // Treat arrays as non-values and skip them
                b'c' | b'b' | b'i' | b'l' | b'f' | b'd' => {
                    let encoded_size: u32 = read_u32(value.add(8));
                    consume_bytes(uc, 13);
                    skip_bytes(uc, encoded_size as u64)?;
                }

                _ => ufbxi_fail!(uc, "Bad value type"),
            }
            i += 1;
        }

        (*node).value_type_mask = type_mask as u16;
    }

    // Skip over remaining values if necessary if we for example truncated
    // the list of values or if there are values after an array
    let offset: u64 = get_read_offset(uc);
    ufbxi_check!(
        uc,
        offset <= values_end_offset,
        "offset <= values_end_offset"
    );
    if offset < values_end_offset {
        skip_bytes(uc, values_end_offset.wrapping_sub(offset))?;
    }

    if recursive {
        // Recursively parse the children of this node. Update the parse state
        // to provide context for child node parsing.
        let parse_state: ParseState = update_parse_state(parent_state, (*node).name);
        let mut num_children: u32 = 0;
        loop {
            // Stop at end offset
            let current_offset: u64 = get_read_offset(uc);
            if current_offset >= end_offset {
                ufbxi_check!(
                    uc,
                    current_offset == end_offset || end_offset == 0,
                    "current_offset == end_offset || end_offset == 0"
                );
                break;
            }

            let mut end: bool = false;
            binary_parse_node(uc, depth + 1, parse_state, &mut end, tmp_buf, true)?;
            if end {
                break;
            }
            num_children += 1;
        }

        // Pop children from `tmp_stack` to a contiguous array
        (*node).num_children = num_children;
        if num_children > 0 {
            (*node).children =
                push_pop::<Node>(tmp_buf, &mut (*uc.get()).tmp_stack, num_children as usize);
            ufbxi_check!(uc, !(*node).children.is_null(), "node->children");
        }
    } else {
        let current_offset: u64 = get_read_offset(uc);
        (*uc.get()).has_next_child = current_offset < end_offset;
    }

    Ok(())
}

// ufbx.c:9400 `#define UFBXI_BINARY_MAGIC_SIZE 22`
pub(crate) const BINARY_MAGIC_SIZE: usize = 22;
// ufbx.c:9401 `#define UFBXI_BINARY_HEADER_SIZE 27`
pub(crate) const BINARY_HEADER_SIZE: usize = 27;
// ufbx.c:9402 `static const char ufbxi_binary_magic[] = "Kaydara FBX Binary  \x00\x1a";`
// C-parity: the C array is 23 bytes (22 magic bytes + the implicit NUL); only
// the first `UFBXI_BINARY_MAGIC_SIZE` bytes are ever compared.
pub(crate) static BINARY_MAGIC: [u8; 23] = *b"Kaydara FBX Binary  \x00\x1a\x00";

// CONTINUATION POINT: ufbx.c:9404 `// -- ASCII parsing` — the end of the
// `// -- Binary parsing` section; `native::parse_ascii` owns what follows.
