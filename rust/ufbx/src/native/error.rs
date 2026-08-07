//! Port of the `// -- Errors` banner section (ufbx.c:3364-3614), plus the
//! check-macro family this unit owns per the porting plan:
//! - the error-target macros (ufbx.c:3550-3557),
//! - the `uc`-context macros (ufbx.c:6652-6671; C defines them next to
//!   `ufbxi_context` — the Rust forms take the context as an explicit first
//!   argument because `macro_rules!` hygiene cannot capture a call-site `uc`),
//! - the options-validation guards (ufbx.c:30302-30328).
//! Also hosts the panic entry points (`ufbxi_panicf_imp` ufbx.c:3384-3403 and
//! the default panic handler ufbx.c:387-397) and the small libc string shims
//! (`strlen`/`strcmp` — C uses libc via the string.h shim, ufbx.c:721-731).
//!
//! Error-model notes (PORTING.md "Error threading"):
//! - Internal fallible fns return `Result<T, Fail>`; C's `return 0` failure
//!   becomes `return Err(Fail)`. The actual `ufbx_error` lives in the context,
//!   as in C.
//! - Conditions are evaluated EXACTLY ONCE (C: `ufbxi_trace(cond)`); every
//!   macro binds the condition to a local before testing and `stringify!`s the
//!   token tree separately.
//! - First error wins: `fail_imp_err` sets `description` only if unset. It
//!   does NOT resolve the error type; `fix_error_type` does the strcmp ladder
//!   at entry points, with per-entry-point default descriptions supplied by
//!   the callers.
//! - `ufbxi_report_err_msg!` records the error and KEEPS GOING — it is the one
//!   non-returning member of the family.
//! - `UFBXI_FEATURE_ERROR_STACK` (ufbx.c:104-108: on under `UFBX_DEV`) is the
//!   `error-stack` cargo feature (implied by `dev`).
//! - C records `__FUNCTION__` into `ufbx_error_frame.function`; Rust has no
//!   stable function-name macro, so `ufbxi_function!` records `module_path!()`
//!   (NUL-terminated) — `line!()` disambiguates the site. Frame values differ
//!   from C by design (PORTING.md: the fuzz table is regenerated per-build;
//!   the mechanism must exist).
//!
//! Dormant references: the `uc`-context macros expand to
//! `$crate::native::parse::fail_imp` / `fail_imp_no_stack`
//! (C: ufbx.c:6652-6662) which the parse unit must define when `ufbxi_context`
//! is ported — macro bodies are not resolved until first expansion.
//!
//! Phase 1: most items have no consumers yet.
#![allow(dead_code, unused_macros, unused_imports)]

use crate::generated::{Error, ErrorFrame, ErrorType, Panic};
use crate::native::platform::{min_sz, ufbx_assert, ufbxi_ignore};
use crate::native::printf::{vprint, PrintArg, PrintBuffer};

// Zero-sized failure token: the C `return 0` failure channel. The actual
// `ufbx_error` lives in the context, as in C (PORTING.md "Error threading").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fail;

// ufbx.h:163-165 (array extents of the public error/panic types)
pub(crate) const ERROR_STACK_MAX_DEPTH: usize = 8;
pub(crate) const PANIC_MESSAGE_LENGTH: usize = 128;
pub(crate) const ERROR_INFO_LENGTH: usize = 256;

// Layout pins tying the hand-duplicated extents above to the generated field
// types (`Error.stack`, `Error.info_buf`, `Panic.message_buf`) — the consts
// are used as raw-pointer buffer bounds below, so a regen/upstream change to
// the array sizes must fail to compile, not silently desync (PORTING.md
// ground rule 0 / "layout-pinned by const assert").
const _: () = {
    fn _pin_stack(e: &Error) -> &[ErrorFrame; ERROR_STACK_MAX_DEPTH] { &e.stack }
    fn _pin_info(e: &Error) -> &crate::prelude::InlineBuf<[u8; ERROR_INFO_LENGTH]> { &e.info_buf }
    fn _pin_message(p: &Panic) -> &crate::prelude::InlineBuf<[u8; PANIC_MESSAGE_LENGTH]> { &p.message_buf }
};

// -- libc string shims (C: strlen/strcmp via the string.h shim, ufbx.c:721-731)

pub(crate) unsafe fn strlen(str_: *const u8) -> usize {
    let mut n: usize = 0;
    while *str_.add(n) != 0 { n += 1; }
    n
}

// C `strcmp` semantics: compare as unsigned chars, return the difference at
// the first mismatch (only the ==0 / !=0 result is consumed in ufbx.c).
pub(crate) unsafe fn strcmp(a: *const u8, b: *const u8) -> i32 {
    let mut i: usize = 0;
    loop {
        let ca = *a.add(i);
        let cb = *b.add(i);
        if ca != cb || ca == 0 { return ca as i32 - cb as i32; }
        i += 1;
    }
}

// ufbx.c:387-397 `ufbxi_panic_handler` (the default `ufbx_panic_handler`).
// C allows a compile-time user override via `#define ufbx_panic_handler`; the
// cargo-world analogue is runtime registration via `crate::set_panic_handler`
// (an atomic fn-pointer read paid only on the panic path). Matching C: a user
// handler that RETURNS makes the ufbxi_panicf caller take its graceful
// bail-out path (ufbx.c:3405-3406), so the registered handler is not required
// to abort.
static USER_PANIC_HANDLER: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

pub(crate) fn set_user_panic_handler(f: fn(&str)) {
    USER_PANIC_HANDLER.store(f as usize, core::sync::atomic::Ordering::Release);
}

pub(crate) unsafe fn panic_handler(message: *const u8) {
    let user = USER_PANIC_HANDLER.load(core::sync::atomic::Ordering::Acquire);
    if user != 0 {
        let f: fn(&str) = core::mem::transmute(user);
        let bytes = core::slice::from_raw_parts(message, strlen(message));
        f(&std::string::String::from_utf8_lossy(bytes));
        return;
    }
    default_panic_handler(message)
}

unsafe fn default_panic_handler(message: *const u8) {
    // C: fprintf(stderr, "ufbx panic: %s\n", message);
    // (slice use is confined to the stderr IO boundary)
    use std::io::Write;
    let bytes = core::slice::from_raw_parts(message, strlen(message));
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(b"ufbx panic: ");
    let _ = stderr.write_all(bytes);
    let _ = stderr.write_all(b"\n");
    ufbx_assert!(false, "ufbx panic: See stderr for more information");
}

// ufbx.c:3366 `static const char ufbxi_empty_char[1] = { '\0' };`
pub(crate) static EMPTY_CHAR: [u8; 1] = [b'\0'];

// ufbx.c:3368-3373 `ufbxi_vsnprintf`
#[inline(never)]
pub(crate) unsafe fn vsnprintf(buf: *mut u8, buf_size: usize, fmt: *const u8, args: &[PrintArg]) -> i32 {
    let mut buffer = PrintBuffer { dst: buf, length: buf_size, pos: 0 };
    vprint(&mut buffer, fmt, args);
    // C-parity: `buf_size - 1` — callers never pass buf_size == 0 (wrapping
    // matches the C unsigned underflow if one ever did).
    min_sz(buffer.pos, buf_size.wrapping_sub(1)) as i32
}

// ufbx.c:3375-3382 `ufbxi_snprintf` (the variadic entry point; the `...` /
// `va_list` pair collapses into the `args` slice — see `ufbxi_snprintf!`).
// C: `va_list args; // ufbxi_uninit` (ufbx.c:3377) — collapsed into `args`.
#[inline(never)]
pub(crate) unsafe fn snprintf(buf: *mut u8, buf_size: usize, fmt: *const u8, args: &[PrintArg]) -> i32 {
    let result = vsnprintf(buf, buf_size, fmt, args);
    result
}

// Call-site wrapper building the `&[PrintArg]` argument pack
// (PORTING.md "Printf and variadics").
macro_rules! ufbxi_snprintf {
    ($buf:expr, $buf_size:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::native::error::snprintf($buf, $buf_size, concat!($fmt, "\0").as_ptr(),
            &[$($crate::native::printf::PrintArg::from($arg)),*])
    };
}
pub(crate) use ufbxi_snprintf;

// ufbx.c:3384-3403 `ufbxi_panicf_imp`
// C: `va_list args; // ufbxi_uninit` (ufbx.c:3388) — collapsed into `args`.
#[inline(never)]
pub(crate) unsafe fn panicf_imp(panic: *mut Panic, fmt: *const u8, args: &[PrintArg]) {
    if !panic.is_null() && (*panic).did_panic { return; }

    if !panic.is_null() {
        (*panic).did_panic = true;
        let message = (*panic).message_buf.data.as_mut_ptr() as *mut u8;
        (*panic).message_length = vsnprintf(message, PANIC_MESSAGE_LENGTH, fmt, args) as usize;
    } else {
        let mut message = [0u8; PANIC_MESSAGE_LENGTH];
        vsnprintf(message.as_mut_ptr(), PANIC_MESSAGE_LENGTH, fmt, args);

        panic_handler(message.as_ptr());
    }
}

// ufbx.c:3405-3406 `ufbxi_panicf(panic, cond, ...)`
// C: `((cond) ? false : (ufbxi_panicf_imp((panic), __VA_ARGS__), true))` —
// returns true when the panic fired; the condition is evaluated once.
macro_rules! ufbxi_panicf {
    ($panic:expr, $cond:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {{
        let cond = $cond;
        if cond {
            false
        } else {
            $crate::native::error::panicf_imp($panic, concat!($fmt, "\0").as_ptr(),
                &[$($crate::native::printf::PrintArg::from($arg)),*]);
            true
        }
    }};
}
pub(crate) use ufbxi_panicf;

// ufbx.c:3408-3409
// C comment: Prefix the error condition with $Description\0 for a human readable description
// `#define ufbxi_error_msg(cond, msg) "$" msg "\0" cond`
// Rust form appends the string-literal NUL terminator C gets implicitly.
macro_rules! ufbxi_error_msg {
    ($cond:literal, $msg:literal) => { concat!("$", $msg, "\0", $cond, "\0") };
}
pub(crate) use ufbxi_error_msg;

// `ufbxi_error_msg(ufbxi_cond_str(cond), msg)` as one step: the stringified
// condition exists only under the error stack (ufbx.c:3536/3540) — user
// macros cannot nest inside `concat!`, hence the fused form.
// The optional trailing literal is the VERBATIM C condition text (`#cond` as
// the C preprocessor stringifies it) for call sites where Rust `stringify!`
// would diverge from the C bytes (checklist #13: stringified conditions
// byte-identical) — e.g. `ator->num_allocs` vs `(*ator).num_allocs`.
#[cfg(feature = "error-stack")]
macro_rules! ufbxi_error_msg_cond {
    ($cond:expr, $msg:literal) => { concat!("$", $msg, "\0", stringify!($cond), "\0") };
    ($cond:expr, $msg:literal, $c_cond_str:literal) => { concat!("$", $msg, "\0", $c_cond_str, "\0") };
}
#[cfg(not(feature = "error-stack"))]
macro_rules! ufbxi_error_msg_cond {
    ($cond:expr, $msg:literal) => { concat!("$", $msg, "\0\0") };
    ($cond:expr, $msg:literal, $c_cond_str:literal) => { concat!("$", $msg, "\0\0") };
}
pub(crate) use ufbxi_error_msg_cond;

// ufbx.c:3411-3444 `ufbxi_fail_imp_err`
#[allow(unused_mut, unused_variables)]
#[inline(never)]
pub(crate) unsafe fn fail_imp_err(err: *mut Error, mut cond: *const u8, func: *const u8, line: u32) -> i32 {
    if !cond.is_null() && *cond == b'$' {
        if (*err).description.data.is_null() {
            (*err).description.data = cond.add(1);
            (*err).description.length = strlen((*err).description.data);
        }

        #[cfg(feature = "error-stack")]
        {
            // Skip the description part if adding to a stack
            cond = cond.add(strlen(cond) + 1);
        }
    }

    // NOTE: This is the base function all fails boil down to, place a breakpoint here to
    // break at the first error
    #[cfg(feature = "error-stack")]
    {
        ufbx_assert!(!cond.is_null());
        ufbx_assert!(!func.is_null());
        if ((*err).stack_size as usize) < ERROR_STACK_MAX_DEPTH {
            // C: `&err->stack[err->stack_size++]` — decomposed.
            let frame: *mut ErrorFrame = &mut (*err).stack[(*err).stack_size as usize];
            (*err).stack_size += 1;
            (*frame).description.data = cond;
            (*frame).description.length = strlen(cond);
            (*frame).function.data = func;
            (*frame).function.length = strlen(func);
            (*frame).source_line = line;
        }
    }
    #[cfg(not(feature = "error-stack"))]
    {
        ufbxi_ignore!(func);
        ufbxi_ignore!(line);
    }

    0
}

// ufbx.c:3446-3486 `ufbxi_utf8_valid_length`
#[must_use]
#[inline(never)]
pub(crate) unsafe fn utf8_valid_length(str_: *const u8, length: usize) -> usize {
    let mut index: usize = 0;
    while index < length {
        let c: u8 = *str_.add(index);
        let left = length - index;

        if (c & 0x80) == 0 {
            if c != 0 {
                index += 1;
                continue;
            }
        } else if (c & 0xe0) == 0xc0 && left >= 2 {
            let t0: u8 = *str_.add(index + 1);
            let code: u32 = (c as u32) << 8 | t0 as u32;
            if (code & 0xc0) == 0x80 && code >= 0xc280 {
                index += 2;
                continue;
            }
        } else if (c & 0xf0) == 0xe0 && left >= 3 {
            let t0: u8 = *str_.add(index + 1);
            let t1: u8 = *str_.add(index + 2);
            let code: u32 = (c as u32) << 16 | (t0 as u32) << 8 | t1 as u32;
            if (code & 0xc0c0) == 0x8080 && code >= 0xe0a080 && (code < 0xeda080 || code >= 0xee8080) {
                index += 3;
                continue;
            }
        } else if (c & 0xf8) == 0xf0 && left >= 4 {
            let t0: u8 = *str_.add(index + 1);
            let t1: u8 = *str_.add(index + 2);
            let t2: u8 = *str_.add(index + 3);
            let code: u32 = (c as u32) << 24 | (t0 as u32) << 16 | (t1 as u32) << 8 | t2 as u32;
            if (code & 0xc0c0c0) == 0x808080 && code >= 0xf0908080u32 && code <= 0xf48fbfbfu32 {
                index += 4;
                continue;
            }
        }

        break;
    }

    ufbx_assert!(index <= length);
    index
}

// ufbx.c:3488-3496 `ufbxi_clean_string_utf8`
#[inline(never)]
pub(crate) unsafe fn clean_string_utf8(str_: *mut u8, length: usize) {
    let mut pos: usize = 0;
    loop {
        // C-parity: C passes the FULL `length` (not `length - pos`) as the
        // scan bound here; a terminating NUL at `str_[length]` (guaranteed by
        // both callers) is what keeps the scan in bounds.
        pos += utf8_valid_length(str_.add(pos) as *const u8, length);
        if pos == length { break; }
        *str_.add(pos) = b'?';
        pos += 1;
    }
}

// ufbx.c:3498-3508 `ufbxi_set_err_info`
#[inline(never)]
pub(crate) unsafe fn set_err_info(err: *mut Error, data: *const u8, mut length: usize) {
    if err.is_null() { return; }

    if length == usize::MAX { length = strlen(data); }
    let info = (*err).info_buf.data.as_mut_ptr() as *mut u8;
    let to_copy = min_sz(ERROR_INFO_LENGTH - 1, length);
    core::ptr::copy_nonoverlapping(data, info, to_copy);
    *info.add(to_copy) = b'\0';
    (*err).info_length = to_copy;
    clean_string_utf8(info, (*err).info_length);
}

// ufbx.c:3510-3519 `ufbxi_fmt_err_info` (variadic entry point — see
// `ufbxi_fmt_err_info!`).
// C: `va_list args; // ufbxi_uninit` (ufbx.c:3514) — collapsed into `args`.
#[inline(never)]
pub(crate) unsafe fn fmt_err_info(err: *mut Error, fmt: *const u8, args: &[PrintArg]) {
    if err.is_null() { return; }

    let info = (*err).info_buf.data.as_mut_ptr() as *mut u8;
    (*err).info_length = vsnprintf(info, ERROR_INFO_LENGTH, fmt, args) as usize;
    clean_string_utf8(info, (*err).info_length);
}

// Call-site wrapper building the `&[PrintArg]` argument pack.
macro_rules! ufbxi_fmt_err_info {
    ($err:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::native::error::fmt_err_info($err, concat!($fmt, "\0").as_ptr(),
            &[$($crate::native::printf::PrintArg::from($arg)),*])
    };
}
pub(crate) use ufbxi_fmt_err_info;

// ufbx.c:3521-3530 `ufbxi_clear_error`
#[inline(never)]
pub(crate) unsafe fn clear_error(err: *mut Error) {
    if err.is_null() { return; }

    (*err).type_ = ErrorType::None;
    (*err).description.data = EMPTY_CHAR.as_ptr();
    (*err).description.length = 0;
    (*err).stack_size = 0;
    *((*err).info_buf.data.as_mut_ptr() as *mut u8) = b'\0';
    (*err).info_length = 0;
}

// ufbx.c:3532-3541
//   #if UFBXI_FEATURE_ERROR_STACK: ufbxi_function = __FUNCTION__, ufbxi_line =
//   __LINE__, ufbxi_cond_str(cond) = #cond; else NULL / 0 / "".
// Rust: `module_path!()` stands in for `__FUNCTION__` (see module docs).
#[cfg(feature = "error-stack")]
macro_rules! ufbxi_function {
    () => { concat!(module_path!(), "\0").as_ptr() };
}
#[cfg(not(feature = "error-stack"))]
macro_rules! ufbxi_function {
    () => { core::ptr::null::<u8>() };
}
pub(crate) use ufbxi_function;

#[cfg(feature = "error-stack")]
macro_rules! ufbxi_line {
    () => { line!() };
}
#[cfg(not(feature = "error-stack"))]
macro_rules! ufbxi_line {
    () => { 0u32 };
}
pub(crate) use ufbxi_line;

// `ufbxi_cond_str(cond)` as a NUL-terminated literal (C string literals carry
// the NUL implicitly). Stringifies WITHOUT evaluating.
// The optional second literal is the VERBATIM C condition text, used where
// Rust `stringify!` diverges from C's `#cond` bytes (checklist #13).
#[cfg(feature = "error-stack")]
macro_rules! ufbxi_cond_str {
    ($cond:expr) => { concat!(stringify!($cond), "\0") };
    ($cond:expr, $c_cond_str:literal) => { concat!($c_cond_str, "\0") };
}
#[cfg(not(feature = "error-stack"))]
macro_rules! ufbxi_cond_str {
    ($cond:expr) => { "\0" };
    ($cond:expr, $c_cond_str:literal) => { "\0" };
}
pub(crate) use ufbxi_cond_str;

// ufbx.c:3543-3548 `ufbxi_fail_err_no_msg(err, cond, func, line)`
// Takes the already-stringified condition / description pointer expression;
// the no-stack form drops it unevaluated (it is always a literal `.as_ptr()`).
#[cfg(feature = "error-stack")]
macro_rules! ufbxi_fail_err_no_msg {
    ($err:expr, $cond_str:expr) => {
        $crate::native::error::fail_imp_err($err, $cond_str,
            $crate::native::error::ufbxi_function!(), $crate::native::error::ufbxi_line!())
    };
}
#[cfg(not(feature = "error-stack"))]
macro_rules! ufbxi_fail_err_no_msg {
    ($err:expr, $cond_str:expr) => {
        $crate::native::error::fail_imp_err_no_stack($err)
    };
}
pub(crate) use ufbxi_fail_err_no_msg;

// ufbx.c:3546 (no-stack branch helper)
#[cfg(not(feature = "error-stack"))]
#[inline(never)]
pub(crate) unsafe fn fail_imp_err_no_stack(err: *mut Error) -> i32 {
    fail_imp_err(err, core::ptr::null(), core::ptr::null(), 0)
}

// -- The check-macro family, error-target forms (ufbx.c:3550-3557)
//
// C failure returns (`return 0` in int-returning fns) become
// `return Err(Fail)`; the `_return` variants return their `ret` expression
// verbatim. Conditions are bound to a local first — evaluated exactly once.

// ufbx.c:3550 `ufbxi_check_err(err, cond)`
// Optional trailing literal: the verbatim C condition text (see `ufbxi_cond_str`).
macro_rules! ufbxi_check_err {
    ($err:expr, $cond:expr) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::ufbxi_fail_err_no_msg!($err,
                $crate::native::error::ufbxi_cond_str!($cond).as_ptr());
            return Err($crate::native::error::Fail);
        }
    }};
    ($err:expr, $cond:expr, $c_cond_str:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::ufbxi_fail_err_no_msg!($err,
                $crate::native::error::ufbxi_cond_str!($cond, $c_cond_str).as_ptr());
            return Err($crate::native::error::Fail);
        }
    }};
}
pub(crate) use ufbxi_check_err;

// ufbx.c:3551 `ufbxi_check_return_err(err, cond, ret)`
// Optional trailing literal: the verbatim C condition text (see `ufbxi_cond_str`).
macro_rules! ufbxi_check_return_err {
    ($err:expr, $cond:expr, $ret:expr) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::ufbxi_fail_err_no_msg!($err,
                $crate::native::error::ufbxi_cond_str!($cond).as_ptr());
            return $ret;
        }
    }};
    ($err:expr, $cond:expr, $ret:expr, $c_cond_str:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::ufbxi_fail_err_no_msg!($err,
                $crate::native::error::ufbxi_cond_str!($cond, $c_cond_str).as_ptr());
            return $ret;
        }
    }};
}
pub(crate) use ufbxi_check_return_err;

// ufbx.c:3552 `ufbxi_fail_err(err, desc)` — unconditional fail-and-return.
// `desc` is a verbatim string literal (all ufbx.c call sites are literals).
macro_rules! ufbxi_fail_err {
    ($err:expr, $desc:literal) => {{
        $crate::native::error::ufbxi_fail_err_no_msg!($err, concat!($desc, "\0").as_ptr());
        return Err($crate::native::error::Fail);
    }};
}
pub(crate) use ufbxi_fail_err;

// ufbx.c:3554 `ufbxi_check_err_msg(err, cond, msg)` — description is
// `"$msg\0<stringified cond>"`; calls `fail_imp_err` in BOTH stack modes (this
// is how release builds get error descriptions).
macro_rules! ufbxi_check_err_msg {
    ($err:expr, $cond:expr, $msg:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::fail_imp_err($err,
                $crate::native::error::ufbxi_error_msg_cond!($cond, $msg).as_ptr(),
                $crate::native::error::ufbxi_function!(), $crate::native::error::ufbxi_line!());
            return Err($crate::native::error::Fail);
        }
    }};
}
pub(crate) use ufbxi_check_err_msg;

// ufbx.c:3555 `ufbxi_check_return_err_msg(err, cond, ret, msg)`
// Optional trailing literal: the verbatim C condition text (see `ufbxi_error_msg_cond`).
macro_rules! ufbxi_check_return_err_msg {
    ($err:expr, $cond:expr, $ret:expr, $msg:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::fail_imp_err($err,
                $crate::native::error::ufbxi_error_msg_cond!($cond, $msg).as_ptr(),
                $crate::native::error::ufbxi_function!(), $crate::native::error::ufbxi_line!());
            return $ret;
        }
    }};
    ($err:expr, $cond:expr, $ret:expr, $msg:literal, $c_cond_str:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::fail_imp_err($err,
                $crate::native::error::ufbxi_error_msg_cond!($cond, $msg, $c_cond_str).as_ptr(),
                $crate::native::error::ufbxi_function!(), $crate::native::error::ufbxi_line!());
            return $ret;
        }
    }};
}
pub(crate) use ufbxi_check_return_err_msg;

// ufbx.c:3556 `ufbxi_fail_err_msg(err, desc, msg)` — desc is a VERBATIM
// literal, NOT a stringified condition (PORTING.md: do not mix these up).
macro_rules! ufbxi_fail_err_msg {
    ($err:expr, $desc:literal, $msg:literal) => {{
        $crate::native::error::fail_imp_err($err,
            $crate::native::error::ufbxi_error_msg!($desc, $msg).as_ptr(),
            $crate::native::error::ufbxi_function!(), $crate::native::error::ufbxi_line!());
        return Err($crate::native::error::Fail);
    }};
}
pub(crate) use ufbxi_fail_err_msg;

// ufbx.c:3557 `ufbxi_report_err_msg(err, desc, msg)` — records the error and
// KEEPS GOING (C: `(void)` result). NOT a return (PORTING.md trap #16).
macro_rules! ufbxi_report_err_msg {
    ($err:expr, $desc:literal, $msg:literal) => {{
        let _ = $crate::native::error::fail_imp_err($err,
            $crate::native::error::ufbxi_error_msg!($desc, $msg).as_ptr(),
            $crate::native::error::ufbxi_function!(), $crate::native::error::ufbxi_line!());
    }};
}
pub(crate) use ufbxi_report_err_msg;

// -- The check-macro family, uc-context forms (ufbx.c:6652-6671)
//
// C hardcodes `uc` in the macro bodies; `macro_rules!` hygiene cannot capture
// a call-site local, so the Rust forms take the context expression as an
// explicit first argument. They expand to `$crate::native::parse::fail_imp` /
// `fail_imp_no_stack` (ufbx.c:6652-6662), which the parse unit defines when
// `ufbxi_context` is ported.

// ufbx.c:6656-6662 `ufbxi_fail_no_msg(uc, cond, func, line)`
#[cfg(feature = "error-stack")]
macro_rules! ufbxi_fail_no_msg {
    ($uc:expr, $cond_str:expr) => {
        $crate::native::parse::fail_imp($uc, $cond_str,
            $crate::native::error::ufbxi_function!(), $crate::native::error::ufbxi_line!())
    };
}
#[cfg(not(feature = "error-stack"))]
macro_rules! ufbxi_fail_no_msg {
    ($uc:expr, $cond_str:expr) => {
        $crate::native::parse::fail_imp_no_stack($uc)
    };
}
pub(crate) use ufbxi_fail_no_msg;

// ufbx.c:6664 `ufbxi_check(cond)`
macro_rules! ufbxi_check {
    ($uc:expr, $cond:expr) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::ufbxi_fail_no_msg!($uc,
                $crate::native::error::ufbxi_cond_str!($cond).as_ptr());
            return Err($crate::native::error::Fail);
        }
    }};
}
pub(crate) use ufbxi_check;

// ufbx.c:6665 `ufbxi_check_return(cond, ret)`
macro_rules! ufbxi_check_return {
    ($uc:expr, $cond:expr, $ret:expr) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::ufbxi_fail_no_msg!($uc,
                $crate::native::error::ufbxi_cond_str!($cond).as_ptr());
            return $ret;
        }
    }};
}
pub(crate) use ufbxi_check_return;

// ufbx.c:6666 `ufbxi_fail(desc)`
macro_rules! ufbxi_fail {
    ($uc:expr, $desc:literal) => {{
        $crate::native::error::ufbxi_fail_no_msg!($uc, concat!($desc, "\0").as_ptr());
        return Err($crate::native::error::Fail);
    }};
}
pub(crate) use ufbxi_fail;

// ufbx.c:6667 `ufbxi_fail_return(desc, ret)`
macro_rules! ufbxi_fail_return {
    ($uc:expr, $desc:literal, $ret:expr) => {{
        $crate::native::error::ufbxi_fail_no_msg!($uc, concat!($desc, "\0").as_ptr());
        return $ret;
    }};
}
pub(crate) use ufbxi_fail_return;

// ufbx.c:6669 `ufbxi_check_msg(cond, msg)` — calls `fail_imp` in BOTH stack
// modes (release-build descriptions).
macro_rules! ufbxi_check_msg {
    ($uc:expr, $cond:expr, $msg:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::parse::fail_imp($uc,
                $crate::native::error::ufbxi_error_msg_cond!($cond, $msg).as_ptr(),
                $crate::native::error::ufbxi_function!(), $crate::native::error::ufbxi_line!());
            return Err($crate::native::error::Fail);
        }
    }};
}
pub(crate) use ufbxi_check_msg;

// ufbx.c:6670 `ufbxi_check_return_msg(cond, ret, msg)`
macro_rules! ufbxi_check_return_msg {
    ($uc:expr, $cond:expr, $ret:expr, $msg:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::parse::fail_imp($uc,
                $crate::native::error::ufbxi_error_msg_cond!($cond, $msg).as_ptr(),
                $crate::native::error::ufbxi_function!(), $crate::native::error::ufbxi_line!());
            return $ret;
        }
    }};
}
pub(crate) use ufbxi_check_return_msg;

// ufbx.c:6671 `ufbxi_fail_msg(desc, msg)` — desc verbatim (e.g.
// `ufbxi_fail_msg("UFBXI_FEATURE_FORMAT_OBJ", "Feature disabled")`,
// ufbx.c:18052).
macro_rules! ufbxi_fail_msg {
    ($uc:expr, $desc:literal, $msg:literal) => {{
        $crate::native::parse::fail_imp($uc,
            $crate::native::error::ufbxi_error_msg!($desc, $msg).as_ptr(),
            $crate::native::error::ufbxi_function!(), $crate::native::error::ufbxi_line!());
        return Err($crate::native::error::Fail);
    }};
}
pub(crate) use ufbxi_fail_msg;

// ufbx.c:3559-3614 `ufbxi_fix_error_type`
// The strcmp ladder, called from the top-level entry points; `default_desc` is
// the per-entry-point default description ("Failed to load", "Failed to
// evaluate", ...) substituted when none was set. All literals are part of
// byte-exact error parity (PORTING.md trap #13).
#[inline(never)]
pub(crate) unsafe fn fix_error_type(error: *mut Error, default_desc: *const u8, p_error: *mut Error) {
    let mut desc = (*error).description.data;
    if desc.is_null() { desc = default_desc; }
    (*error).type_ = ErrorType::Unknown;
    if strcmp(desc, b"Out of memory\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::OutOfMemory;
    } else if strcmp(desc, b"Memory limit exceeded\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::MemoryLimit;
    } else if strcmp(desc, b"Allocation limit exceeded\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::AllocationLimit;
    } else if strcmp(desc, b"Truncated file\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::TruncatedFile;
    } else if strcmp(desc, b"IO error\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::Io;
    } else if strcmp(desc, b"Cancelled\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::Cancelled;
    } else if strcmp(desc, b"Unrecognized file format\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::UnrecognizedFileFormat;
    } else if strcmp(desc, b"File not found\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::FileNotFound;
    } else if strcmp(desc, b"Empty file\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::EmptyFile;
    } else if strcmp(desc, b"External file not found\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::ExternalFileNotFound;
    } else if strcmp(desc, b"Uninitialized options\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::UninitializedOptions;
    } else if strcmp(desc, b"Zero vertex size\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::ZeroVertexSize;
    } else if strcmp(desc, b"Truncated vertex stream\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::TruncatedVertexStream;
    } else if strcmp(desc, b"Invalid UTF-8\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::InvalidUtf8;
    } else if strcmp(desc, b"Feature disabled\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::FeatureDisabled;
    } else if strcmp(desc, b"Bad NURBS geometry\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::BadNurbs;
    } else if strcmp(desc, b"Bad index\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::BadIndex;
    } else if strcmp(desc, b"Node depth limit exceeded\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::NodeDepthLimit;
    } else if strcmp(desc, b"Threaded ASCII parse error\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::ThreadedAsciiParse;
    } else if strcmp(desc, b"Unsafe options\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::UnsafeOptions;
    } else if strcmp(desc, b"Duplicate override\0".as_ptr()) == 0 {
        (*error).type_ = ErrorType::DuplicateOverride;
    }
    (*error).description.data = desc;
    (*error).description.length = strlen(desc);
    if !p_error.is_null() {
        // memcpy(p_error, error, sizeof(ufbx_error));
        core::ptr::copy_nonoverlapping(error as *const Error, p_error, 1);
    }
}

// -- Options-validation guards (ufbx.c:30302-30328; C defines these in the
// `-- Setup` / API prelude — hosted here with the rest of the macro family)

// ufbx.c:30302-30312 `ufbxi_uninitialized_options`
#[inline(never)]
pub(crate) unsafe fn uninitialized_options(p_error: *mut Error) -> *mut core::ffi::c_void {
    if !p_error.is_null() {
        core::ptr::write_bytes(p_error as *mut u8, 0, core::mem::size_of::<Error>());
        (*p_error).type_ = ErrorType::UninitializedOptions;
        (*p_error).description.data = b"Uninitialized options\0".as_ptr();
        (*p_error).description.length = strlen(b"Uninitialized options\0".as_ptr());
    }
    core::ptr::null_mut()
}

// ufbx.c:30314-30318 `ufbxi_check_opts_ptr(m_type, m_opts, m_error)`
macro_rules! ufbxi_check_opts_ptr {
    ($m_type:ty, $m_opts:expr, $m_error:expr) => {{
        let m_opts = $m_opts;
        if !m_opts.is_null() {
            let opts_cleared_to_zero = (*m_opts)._begin_zero | (*m_opts)._end_zero;
            $crate::native::platform::ufbx_assert!(opts_cleared_to_zero == 0);
            if opts_cleared_to_zero != 0 {
                return $crate::native::error::uninitialized_options($m_error) as *mut $m_type;
            }
        }
    }};
}
pub(crate) use ufbxi_check_opts_ptr;

// ufbx.c:30320-30327 `ufbxi_check_opts_return(m_value, m_opts, m_error)`
macro_rules! ufbxi_check_opts_return {
    ($m_value:expr, $m_opts:expr, $m_error:expr) => {{
        let m_opts = $m_opts;
        if !m_opts.is_null() {
            let opts_cleared_to_zero = (*m_opts)._begin_zero | (*m_opts)._end_zero;
            $crate::native::platform::ufbx_assert!(opts_cleared_to_zero == 0);
            if opts_cleared_to_zero != 0 {
                $crate::native::error::uninitialized_options($m_error);
                return $m_value;
            }
        }
    }};
}
pub(crate) use ufbxi_check_opts_return;

// ufbx.c:30329-30332 `ufbxi_check_opts_return_no_error(m_value, m_opts)`
// (no assert, no error write)
macro_rules! ufbxi_check_opts_return_no_error {
    ($m_value:expr, $m_opts:expr) => {{
        let m_opts = $m_opts;
        if !m_opts.is_null() {
            let opts_cleared_to_zero = (*m_opts)._begin_zero | (*m_opts)._end_zero;
            if opts_cleared_to_zero != 0 { return $m_value; }
        }
    }};
}
pub(crate) use ufbxi_check_opts_return_no_error;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::printf::PrintArg;

    unsafe fn desc_bytes(err: &Error) -> &[u8] {
        core::slice::from_raw_parts(err.description.data, err.description.length)
    }

    #[test]
    fn test_snprintf_macro() {
        unsafe {
            let mut buf = [0u8; 32];
            let len = ufbxi_snprintf!(buf.as_mut_ptr(), buf.len(), "Frame%uTick%u.%s", 12u32, 7u32, b"pc2\0".as_ptr() as *const u8);
            assert_eq!(len, 16);
            assert_eq!(&buf[..len as usize], b"Frame12Tick7.pc2");
            assert_eq!(buf[len as usize], 0);
        }
    }

    #[test]
    fn test_snprintf_truncation_length() {
        unsafe {
            let mut buf = [0xAAu8; 8];
            let len = ufbxi_snprintf!(buf.as_mut_ptr(), buf.len(), "%s", b"0123456789\0".as_ptr() as *const u8);
            // pos saturates at length (8); returned min(pos, size - 1) = 7.
            assert_eq!(len, 7);
            assert_eq!(&buf[..8], b"0123456\0");
        }
    }

    fn checked_fn(err: *mut Error, ok: bool, hits: &mut u32) -> Result<u32, Fail> {
        unsafe {
            // Evaluation-once: the condition increments `hits` exactly once.
            ufbxi_check_err_msg!(err, { *hits += 1; ok }, "Out of memory");
            Ok(42)
        }
    }

    #[test]
    fn test_check_err_msg_and_first_error_wins() {
        unsafe {
            let mut err = Error::default();
            let mut hits = 0u32;
            assert_eq!(checked_fn(&mut err, true, &mut hits), Ok(42));
            assert_eq!(hits, 1);
            assert!(err.description.data.is_null());

            assert_eq!(checked_fn(&mut err, false, &mut hits), Err(Fail));
            assert_eq!(hits, 2);
            assert_eq!(desc_bytes(&err), b"Out of memory");

            // First error wins: a second failure does not overwrite.
            fn fail2(err: *mut Error) -> Result<(), Fail> {
                unsafe {
                    ufbxi_check_err_msg!(err, false, "Truncated file");
                    Ok(())
                }
            }
            assert_eq!(fail2(&mut err), Err(Fail));
            assert_eq!(desc_bytes(&err), b"Out of memory");

            let mut p_error = Error::default();
            fix_error_type(&mut err, b"Failed to load\0".as_ptr(), &mut p_error);
            assert_eq!(err.type_, ErrorType::OutOfMemory);
            assert_eq!(desc_bytes(&err), b"Out of memory");
            assert_eq!(p_error.type_, ErrorType::OutOfMemory);
        }
    }

    #[test]
    fn test_fix_error_type_default_desc() {
        unsafe {
            // No description set -> per-entry-point default, type Unknown.
            let mut err = Error::default();
            fix_error_type(&mut err, b"Failed to evaluate\0".as_ptr(), core::ptr::null_mut());
            assert_eq!(err.type_, ErrorType::Unknown);
            assert_eq!(desc_bytes(&err), b"Failed to evaluate");

            // Ladder entries map byte-exactly.
            let mut err = Error::default();
            err.description.data = b"Threaded ASCII parse error\0".as_ptr();
            err.description.length = 26;
            fix_error_type(&mut err, b"Failed to load\0".as_ptr(), core::ptr::null_mut());
            assert_eq!(err.type_, ErrorType::ThreadedAsciiParse);
        }
    }

    #[test]
    fn test_report_err_msg_keeps_going() {
        unsafe {
            let mut err = Error::default();
            let mut reached = false;
            (|| {
                ufbxi_report_err_msg!(&mut err as *mut Error, "ptr", "Out of memory");
                reached = true;
            })();
            assert!(reached, "ufbxi_report_err_msg must not return early");
            assert_eq!(desc_bytes(&err), b"Out of memory");
        }
    }

    #[test]
    fn test_fail_err_and_check_return_err() {
        {
            // ufbxi_fail_err with a desc that is NOT '$'-prefixed sets no
            // description (only the stack frame, when enabled).
            let mut err = Error::default();
            fn f(err: *mut Error) -> Result<(), Fail> {
                unsafe { ufbxi_fail_err!(err, "Task failed"); }
            }
            assert_eq!(f(&mut err), Err(Fail));
            assert!(err.description.data.is_null());
            #[cfg(feature = "error-stack")]
            unsafe {
                assert_eq!(err.stack_size, 1);
                assert_eq!(
                    core::slice::from_raw_parts(err.stack[0].description.data, err.stack[0].description.length),
                    b"Task failed");
            }

            // check_return_err returns the given value verbatim.
            let mut err = Error::default();
            fn g(err: *mut Error) -> u32 {
                unsafe {
                    ufbxi_check_return_err!(err, false, 7);
                    1
                }
            }
            assert_eq!(g(&mut err), 7);
        }
    }

    #[test]
    fn test_clear_error_and_fmt_err_info() {
        unsafe {
            let mut err = Error::default();
            clear_error(&mut err);
            assert_eq!(err.type_, ErrorType::None);
            assert_eq!(err.description.data, EMPTY_CHAR.as_ptr());
            assert_eq!(err.description.length, 0);
            assert_eq!(err.info(), "");

            ufbxi_fmt_err_info!(&mut err as *mut Error, "%u (max %u)", 5u32, 3u32);
            assert_eq!(err.info(), "5 (max 3)");

            set_err_info(&mut err, b"UFBX_ENABLE_FORMAT_OBJ".as_ptr(), 22);
            assert_eq!(err.info(), "UFBX_ENABLE_FORMAT_OBJ");
            // SIZE_MAX length -> strlen
            set_err_info(&mut err, b"abc\0".as_ptr(), usize::MAX);
            assert_eq!(err.info(), "abc");
        }
    }

    #[test]
    fn test_utf8_valid_length_and_clean() {
        unsafe {
            // ASCII
            assert_eq!(utf8_valid_length(b"hello\0".as_ptr(), 5), 5);
            // NUL stops the scan
            assert_eq!(utf8_valid_length(b"he\0lo\0".as_ptr(), 5), 2);
            // 2-byte: U+00E4, overlong C1 80 rejected
            assert_eq!(utf8_valid_length(b"\xc3\xa4\0".as_ptr(), 2), 2);
            assert_eq!(utf8_valid_length(b"\xc1\x80\0".as_ptr(), 2), 0);
            // 3-byte: U+20AC valid; UTF-16 surrogate ED A0 80 rejected
            assert_eq!(utf8_valid_length(b"\xe2\x82\xac\0".as_ptr(), 3), 3);
            assert_eq!(utf8_valid_length(b"\xed\xa0\x80\0".as_ptr(), 3), 0);
            // 4-byte: U+1F600 valid; > U+10FFFF rejected
            assert_eq!(utf8_valid_length(b"\xf0\x9f\x98\x80\0".as_ptr(), 4), 4);
            assert_eq!(utf8_valid_length(b"\xf4\x90\x80\x80\0".as_ptr(), 4), 0);

            // clean_string_utf8 replaces each invalid byte with '?'
            let mut s = *b"a\xffb\xc3\xa4\xed\xa0\x80\0";
            let len = 8;
            clean_string_utf8(s.as_mut_ptr(), len);
            assert_eq!(&s[..len], b"a?b\xc3\xa4???");
        }
    }

    #[test]
    fn test_panicf() {
        unsafe {
            let mut panic = Panic::default();
            let fired = ufbxi_panicf!(&mut panic as *mut Panic, 1 < 2, "vertex (%zu) out of bounds (%zu)", 5usize, 3usize);
            assert!(!fired);
            assert!(!panic.did_panic);

            let fired = ufbxi_panicf!(&mut panic as *mut Panic, false, "vertex (%zu) out of bounds (%zu)", 5usize, 3usize);
            assert!(fired);
            assert!(panic.did_panic);
            assert_eq!(panic.message(), "vertex (5) out of bounds (3)");

            // Already-panicked: message preserved
            let fired = ufbxi_panicf!(&mut panic as *mut Panic, false, "other");
            assert!(fired);
            assert_eq!(panic.message(), "vertex (5) out of bounds (3)");
        }
    }

    #[test]
    fn test_uninitialized_options() {
        unsafe {
            let mut err = Error::default();
            err.type_ = ErrorType::Io;
            let ret = uninitialized_options(&mut err);
            assert!(ret.is_null());
            assert_eq!(err.type_, ErrorType::UninitializedOptions);
            assert_eq!(desc_bytes(&err), b"Uninitialized options");
            assert!(uninitialized_options(core::ptr::null_mut()).is_null());
        }
    }

    #[test]
    fn test_strlen_strcmp() {
        unsafe {
            assert_eq!(strlen(b"\0".as_ptr()), 0);
            assert_eq!(strlen(b"abc\0".as_ptr()), 3);
            assert_eq!(strcmp(b"abc\0".as_ptr(), b"abc\0".as_ptr()), 0);
            assert!(strcmp(b"abc\0".as_ptr(), b"abd\0".as_ptr()) < 0);
            assert!(strcmp(b"abd\0".as_ptr(), b"abc\0".as_ptr()) > 0);
            assert!(strcmp(b"ab\0".as_ptr(), b"abc\0".as_ptr()) < 0);
            assert!(strcmp(b"abc\0".as_ptr(), b"ab\0".as_ptr()) > 0);
        }
    }
}
