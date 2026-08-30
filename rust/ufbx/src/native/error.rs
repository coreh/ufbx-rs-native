//! Port of the `// -- Errors` banner section (ufbx.c:3364-3614), plus the
//! check-macro family this unit owns per the porting plan:
//! - the error-target macros (ufbx.c:3550-3557),
//! - the `uc`-context macros (ufbx.c:6652-6671; C defines them next to
//!   `ufbxi_context` — the Rust forms take the context as an explicit first
//!   argument because `macro_rules!` hygiene cannot capture a call-site `uc`),
//! - the options-validation guards (ufbx.c:30302-30328).
//!
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
//!   with the `::` separators collapsed to `.` (NUL-terminated) — `line!()`
//!   disambiguates the site. The collapse is load-bearing: `ufbx_format_error`
//!   output is parsed back by upstream tests via `sscanf("%u:%63[^:]: ...")`
//!   (test/test_parse.h:211), so the function string must be colon-free, as C
//!   `__FUNCTION__` always is. Frame values otherwise differ from C by design
//!   (PORTING.md: the fuzz table is regenerated per-build; the mechanism must
//!   exist).
#![allow(dead_code, unused_macros, unused_imports)]
use crate::generated::{Error, ErrorFrame, ErrorType, Panic};
use crate::native::platform::{min_sz, ufbx_assert, ufbxi_ignore};
use crate::native::printf::{vprint, PrintArg, PrintBuffer};
use crate::native::view::{view_project, view_raw_mut, view_read, view_write};

// Zero-sized failure token: the C `return 0` failure channel. The actual
// `ufbx_error` lives in the context, as in C (PORTING.md "Error threading").
// The field is private: a `Fail` is minted either by the recording fail/report
// family (`fail_err`/`fail_imp`/... — the WITNESS that an error was written to
// the target) or by the explicit [`Fail::unrecorded`] constructor, so
// `Err(Fail)` can no longer be conjured at arbitrary sites.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fail(());

impl Fail {
    /// C-parity `return 0` WITHOUT a fail call at the return site: the error
    /// was recorded earlier (deferred/copied), will be recorded by the caller,
    /// or is deliberately silent — match the C body. Deliberately greppable:
    /// every use declares that this site is NOT the recording point.
    #[inline(always)]
    pub(crate) fn unrecorded() -> Fail {
        Fail(())
    }
}

// Carrier for the `'static`, NUL-terminated byte strings the fail path forwards
// as messages and function names (C: `const char *` literals). The invariant —
// `'static` and NUL-terminated (messages are the packed `$desc\0cond\0` form,
// interior NULs and all) — is what lets `fail_imp` / `fail_imp_no_stack` be safe
// fns: the pointer they forward into `fail_imp_err` is valid by construction, not
// by a caller obligation. Only the check-macro family constructs these, always
// from `concat!(..., "\0")` literals or the NUL-padded `ufbxi_function!` buffer.
#[derive(Clone, Copy)]
pub(crate) struct FailStr(&'static [u8]);

impl FailStr {
    #[inline(always)]
    pub(crate) const fn new(bytes: &'static [u8]) -> Self {
        // `fail_imp_err` reads these via `strlen` from `.as_ptr()`; a trailing
        // NUL is what keeps that read in bounds. Every constructor passes a
        // NUL-terminated literal; assert it so a future non-terminated caller
        // fails loudly rather than reading past the end.
        debug_assert!(!bytes.is_empty() && bytes[bytes.len() - 1] == 0);
        Self(bytes)
    }
    #[inline(always)]
    pub(crate) fn as_ptr(self) -> *const u8 {
        self.0.as_ptr()
    }
}

// `Option<FailStr>` -> raw pointer, null for `None` (the no-stack / no-message
// paths where C passes a null `const char *`).
#[inline(always)]
fn fail_str_ptr(s: Option<FailStr>) -> *const u8 {
    s.map_or(core::ptr::null(), FailStr::as_ptr)
}

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
    fn _pin_stack(e: &Error) -> &[ErrorFrame; ERROR_STACK_MAX_DEPTH] {
        &e.stack
    }
    fn _pin_info(e: &Error) -> &crate::prelude::InlineBuf<[u8; ERROR_INFO_LENGTH]> {
        &e.info_buf
    }
    fn _pin_message(p: &Panic) -> &crate::prelude::InlineBuf<[u8; PANIC_MESSAGE_LENGTH]> {
        &p.message_buf
    }
};

// -- libc string shims (C: strlen/strcmp via the string.h shim, ufbx.c:721-731)

pub(crate) unsafe fn strlen(str_: *const u8) -> usize {
    let mut n: usize = 0;
    // SAFETY: the caller's contract is that `str_` points at a NUL-terminated
    // run, so every byte up to and including the terminator is readable; the
    // loop stops at that terminator and never advances past it.
    while unsafe { *str_.add(n) } != 0 {
        n += 1;
    }
    n
}

// C `strcmp` semantics: compare as unsigned chars, return the difference at
// the first mismatch (only the ==0 / !=0 result is consumed in ufbx.c).
pub(crate) unsafe fn strcmp(a: *const u8, b: *const u8) -> i32 {
    let mut i: usize = 0;
    loop {
        // SAFETY: the caller's contract is that both `a` and `b` are
        // NUL-terminated; the loop exits at the first NUL, so index `i` stays
        // within both runs.
        let ca = unsafe { *a.add(i) };
        // SAFETY: same NUL-terminated contract, on `b`.
        let cb = unsafe { *b.add(i) };
        if ca != cb || ca == 0 {
            return ca as i32 - cb as i32;
        }
        i += 1;
    }
}

// C `strcmp` semantics over byte slices: compare as unsigned chars, stopping
// at the first NUL byte or slice end (an exhausted slice reads as NUL). For a
// `String` whose `data` is NUL-terminated at `length` — every interned or
// `FailStr`-derived description — this is byte-for-byte `strcmp`: both walks
// stop at the same first NUL, whether it sits inside the slice or right past
// its end. Returns the difference at the first mismatch, like `strcmp` above.
pub(crate) fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i: usize = 0;
    loop {
        let ca = if i < a.len() { a[i] } else { 0 };
        let cb = if i < b.len() { b[i] } else { 0 };
        if ca != cb || ca == 0 {
            return ca as i32 - cb as i32;
        }
        i += 1;
    }
}

// C `memcmp` semantics: lexicographic compare of `n` bytes as unsigned chars;
// returns the difference at the first mismatch.
pub(crate) unsafe fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i: usize = 0;
    while i < n {
        // SAFETY: the caller's contract is that `a` and `b` each address at
        // least `n` readable bytes, and the loop condition holds `i < n`.
        let ca = unsafe { *a.add(i) };
        // SAFETY: same `n`-byte contract, on `b`.
        let cb = unsafe { *b.add(i) };
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        i += 1;
    }
    0
}

// C `memchr` semantics: return a pointer to the first occurrence of `c` in the
// first `n` bytes of `s`, or NULL if there is none.
pub(crate) unsafe fn memchr(s: *const u8, c: u8, n: usize) -> *const u8 {
    let mut i: usize = 0;
    while i < n {
        // SAFETY: the caller's contract is that `s` addresses at least `n`
        // readable bytes, and the loop condition holds `i < n`.
        if unsafe { *s.add(i) } == c {
            // SAFETY: same `n`-byte contract; `i < n` keeps the offset in range.
            return unsafe { s.add(i) };
        }
        i += 1;
    }
    core::ptr::null()
}

// C `strncmp` semantics: compare at most `n` bytes as unsigned chars, stopping
// at the first NUL; returns the difference at the first mismatch (only the
// ==0 / !=0 result is consumed in ufbx.c).
pub(crate) unsafe fn strncmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i: usize = 0;
    while i < n {
        // SAFETY: the caller's contract is that `a` and `b` are readable for
        // `n` bytes or up to a NUL, whichever comes first; `i < n` holds here
        // and the loop exits at the first NUL.
        let ca = unsafe { *a.add(i) };
        // SAFETY: same bounded/NUL-terminated contract, on `b`.
        let cb = unsafe { *b.add(i) };
        if ca != cb || ca == 0 {
            return ca as i32 - cb as i32;
        }
        i += 1;
    }
    0
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
        // SAFETY: the stored value is nonzero, and `set_user_panic_handler` is
        // the only writer — it stores a `fn(&str)` cast to `usize`.
        let f: fn(&str) = unsafe { core::mem::transmute(user) };
        // SAFETY: the caller's contract is that `message` is NUL-terminated, so
        // `strlen` reads in bounds and the run it measures is readable.
        let bytes = unsafe { core::slice::from_raw_parts(message, strlen(message)) };
        f(&std::string::String::from_utf8_lossy(bytes));
        return;
    }
    // SAFETY: `message` carries this fn's own NUL-terminated contract, which is
    // exactly what `default_panic_handler` requires of its parameter.
    unsafe { default_panic_handler(message) }
}

unsafe fn default_panic_handler(message: *const u8) {
    // C: fprintf(stderr, "ufbx panic: %s\n", message);
    // (slice use is confined to the stderr IO boundary)
    use std::io::Write;
    // SAFETY: the caller's contract is that `message` is NUL-terminated, so
    // `strlen` reads in bounds and the run it measures is readable.
    let bytes = unsafe { core::slice::from_raw_parts(message, strlen(message)) };
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(b"ufbx panic: ");
    let _ = stderr.write_all(bytes);
    let _ = stderr.write_all(b"\n");
    ufbx_assert!(false, "ufbx panic: See stderr for more information");
}

// ufbx.c:3366 `static const char ufbxi_empty_char[1] = { '\0' };`
pub(crate) static EMPTY_CHAR: [u8; 1] = *b"\0";

// ufbx.c:3368-3373 `ufbxi_vsnprintf`
#[inline(never)]
pub(crate) unsafe fn vsnprintf(
    buf: *mut u8,
    buf_size: usize,
    fmt: *const u8,
    args: &[PrintArg],
) -> i32 {
    let mut buffer = PrintBuffer {
        dst: buf,
        length: buf_size,
        pos: 0,
    };
    // SAFETY: `buffer` describes the caller's `buf`/`buf_size` pair verbatim,
    // and `fmt` carries this fn's NUL-terminated format-string contract — the
    // two obligations `vprint` states.
    unsafe { vprint(&raw mut buffer, fmt, args) };
    // C-parity: `buf_size - 1` — callers never pass buf_size == 0 (wrapping
    // matches the C unsigned underflow if one ever did).
    min_sz(buffer.pos, buf_size.wrapping_sub(1)) as i32
}

// ufbx.c:3375-3382 `ufbxi_snprintf` (the variadic entry point; the `...` /
// `va_list` pair collapses into the `args` slice — see `ufbxi_snprintf!`).
// C: `va_list args; // ufbxi_uninit` (ufbx.c:3377) — collapsed into `args`.
#[inline(never)]
pub(crate) unsafe fn snprintf(
    buf: *mut u8,
    buf_size: usize,
    fmt: *const u8,
    args: &[PrintArg],
) -> i32 {
    // SAFETY: the parameters are forwarded unchanged, so this fn's own
    // contract (writable `buf` of `buf_size`, NUL-terminated `fmt`) is exactly
    // what `vsnprintf` requires.
    unsafe { vsnprintf(buf, buf_size, fmt, args) }
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
// Safe fn: `panic` is C's nullable catch-me out-param, modernized to
// `Option<&mut Panic>` — unlike `Error` (aliased via stored back-pointers, so
// it takes the interior-mutable `ErrorView`), a `Panic` is a caller stack
// local with a single writer, which is exactly what `&mut` asserts. The
// format string rides in a [`FailStr`] (NUL-terminated 'static literal from
// the `ufbxi_panicf!` macro), carrying the invariant `vsnprintf` needs.
pub(crate) fn panicf_imp(panic: Option<&mut Panic>, fmt: FailStr, args: &[PrintArg]) {
    match panic {
        Some(panic) => {
            if panic.did_panic {
                return;
            }
            panic.did_panic = true;
            let message = panic.message_buf.data.as_mut_ptr() as *mut u8;
            // SAFETY: `message` is the panic's own PANIC_MESSAGE_LENGTH buffer;
            // `fmt` is NUL-terminated 'static (FailStr invariant).
            panic.message_length =
                unsafe { vsnprintf(message, PANIC_MESSAGE_LENGTH, fmt.as_ptr(), args) } as usize;
        }
        None => {
            let mut message = [0u8; PANIC_MESSAGE_LENGTH];
            // SAFETY: local buffer of PANIC_MESSAGE_LENGTH; NUL-terminated fmt.
            unsafe {
                vsnprintf(
                    message.as_mut_ptr(),
                    PANIC_MESSAGE_LENGTH,
                    fmt.as_ptr(),
                    args,
                );
                panic_handler(message.as_ptr());
            }
        }
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
            $crate::native::error::panicf_imp($panic.as_deref_mut(),
                $crate::native::error::FailStr::new(concat!($fmt, "\0").as_bytes()),
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
    ($cond:literal, $msg:literal) => {
        concat!("$", $msg, "\0", $cond, "\0")
    };
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
    ($cond:expr, $msg:literal) => {
        concat!("$", $msg, "\0", stringify!($cond), "\0")
    };
    ($cond:expr, $msg:literal, $c_cond_str:literal) => {
        concat!("$", $msg, "\0", $c_cond_str, "\0")
    };
}
#[cfg(not(feature = "error-stack"))]
macro_rules! ufbxi_error_msg_cond {
    ($cond:expr, $msg:literal) => {
        concat!("$", $msg, "\0\0")
    };
    ($cond:expr, $msg:literal, $c_cond_str:literal) => {
        concat!("$", $msg, "\0\0")
    };
}
pub(crate) use ufbxi_error_msg_cond;

// ufbx.c:3411-3444 `ufbxi_fail_imp_err`
#[allow(unused_mut, unused_variables)]
#[inline(never)]
pub(crate) fn fail_imp_err(
    err: &ErrorView,
    cond: Option<FailStr>,
    func: Option<FailStr>,
    line: u32,
) -> i32 {
    let mut cond: *const u8 = fail_str_ptr(cond);
    let func: *const u8 = fail_str_ptr(func);
    // SAFETY: `cond` is non-null on the right of the `&&`, and it comes from a
    // `FailStr` — a NUL-terminated 'static run — so the first byte is readable.
    if !cond.is_null() && unsafe { *cond } == b'$' {
        let description = err.description_view();
        if description.data().is_null() {
            // SAFETY: the leading byte read above is `'$'`, so the FailStr run
            // holds at least one more byte (its NUL), and `strlen` then walks
            // that same NUL-terminated run.
            description.set_data(unsafe { cond.add(1) });
            description.set_length(unsafe { strlen(description.data()) });
        }

        #[cfg(feature = "error-stack")]
        {
            // Skip the description part if adding to a stack
            // SAFETY: a `'$'`-prefixed FailStr is the packed `$desc\0cond\0`
            // form, so the byte after the description's NUL — the offset
            // `strlen(cond) + 1` lands on — is still inside the same run.
            cond = unsafe { cond.add(strlen(cond) + 1) };
        }
    }

    // NOTE: This is the base function all fails boil down to, place a breakpoint here to
    // break at the first error
    #[cfg(feature = "error-stack")]
    {
        ufbx_assert!(!cond.is_null());
        ufbx_assert!(!func.is_null());
        let stack_size = err.stack_size();
        if (stack_size as usize) < ERROR_STACK_MAX_DEPTH {
            // C: `&err->stack[err->stack_size++]` — decomposed.
            // SAFETY: `stack_frame_view` requires an in-bounds index, which the
            // `< ERROR_STACK_MAX_DEPTH` check above establishes.
            let frame = unsafe { err.stack_frame_view(stack_size as usize) };
            err.set_stack_size(stack_size + 1);
            let description = frame.description_view();
            let function = frame.function_view();
            // SAFETY (both `strlen` calls): `cond` and `func` are non-null
            // (asserted above) and come from `FailStr` carriers, whose
            // invariant is a NUL-terminated 'static run.
            description.set_data(cond);
            description.set_length(unsafe { strlen(cond) });
            function.set_data(func);
            function.set_length(unsafe { strlen(func) });
            frame.set_source_line(line);
        }
    }
    #[cfg(not(feature = "error-stack"))]
    {
        ufbxi_ignore!(func);
        ufbxi_ignore!(line);
    }

    0
}

// `Fail`-minting wrapper over `fail_imp_err`: the C body returns `0` (the
// int-shaped failure), the Rust callers need the `Fail` witness that an error
// was written to the target. `cond`/`func` are already-safe `FailStr` carriers
// and the target is an anchored `&ErrorView`, so the call carries no unsafe
// (mirrors the uc-form `fail_imp` in parse.rs).
#[inline]
pub(crate) fn fail_err(
    err: &ErrorView,
    cond: Option<FailStr>,
    func: Option<FailStr>,
    line: u32,
) -> Fail {
    fail_imp_err(err, cond, func, line);
    Fail(())
}

// ufbx.c:3446-3486 `ufbxi_utf8_valid_length`
#[must_use]
#[inline(never)]
pub(crate) fn utf8_valid_length(str_: &[u8]) -> usize {
    let length = str_.len();
    let mut index: usize = 0;
    while index < length {
        let c: u8 = str_[index];
        let left = length - index;

        if (c & 0x80) == 0 {
            if c != 0 {
                index += 1;
                continue;
            }
        } else if (c & 0xe0) == 0xc0 && left >= 2 {
            let t0: u8 = str_[index + 1];
            let code: u32 = (c as u32) << 8 | t0 as u32;
            if (code & 0xc0) == 0x80 && code >= 0xc280 {
                index += 2;
                continue;
            }
        } else if (c & 0xf0) == 0xe0 && left >= 3 {
            let t0: u8 = str_[index + 1];
            let t1: u8 = str_[index + 2];
            let code: u32 = (c as u32) << 16 | (t0 as u32) << 8 | t1 as u32;
            if (code & 0xc0c0) == 0x8080
                && code >= 0xe0a080
                && (code < 0xeda080 || code >= 0xee8080)
            {
                index += 3;
                continue;
            }
        } else if (c & 0xf8) == 0xf0 && left >= 4 {
            let t0: u8 = str_[index + 1];
            let t1: u8 = str_[index + 2];
            let t2: u8 = str_[index + 3];
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
pub(crate) fn clean_string_utf8(str_: &mut [u8]) {
    let length = str_.len();
    let mut pos: usize = 0;
    loop {
        // PORT DIVERGENCE (ufbx.c:3492): C passes the full `length` from the
        // offset base `str + pos`, so `utf8_valid_length`'s multi-byte lookahead
        // reads up to two bytes past the terminating NUL. This passes the
        // remaining `length - pos` instead; reconcile once upstream lands the fix.
        pos += utf8_valid_length(&str_[pos..]);
        if pos == length {
            break;
        }
        str_[pos] = b'?';
        pos += 1;
    }
}

// ufbx.c:3498-3508 `ufbxi_set_err_info`
// C's nullable `ufbx_error *err` is an `Option<&ErrorView>`; the `!err`
// early-out is the `None` arm.
//
// # Safety
// `data` must be readable for `length` bytes — or, when `length` is
// `usize::MAX` (C's `SIZE_MAX` sentinel), NUL-terminated. That extent is a
// caller promise no parameter type carries.
#[inline(never)]
pub(crate) unsafe fn set_err_info(err: Option<&ErrorView>, data: *const u8, mut length: usize) {
    let Some(err) = err else {
        return;
    };

    if length == usize::MAX {
        // SAFETY: `usize::MAX` is C's `SIZE_MAX` sentinel meaning "measure it",
        // which callers only pass for a NUL-terminated `data`.
        length = unsafe { strlen(data) };
    }
    let info = err.info_mut_ptr();
    let to_copy = min_sz(ERROR_INFO_LENGTH - 1, length);
    // SAFETY: `to_copy <= length`, so the source run is readable per this fn's
    // `data`/`length` contract; `info` is the error's own ERROR_INFO_LENGTH
    // buffer, inside the live `Error` the view was minted over (its mint
    // invariant), and `to_copy <= ERROR_INFO_LENGTH - 1`. The two buffers are
    // distinct objects, so the copy is non-overlapping.
    unsafe { core::ptr::copy_nonoverlapping(data, info, to_copy) };
    // SAFETY: `to_copy <= ERROR_INFO_LENGTH - 1`, so this NUL lands on the last
    // byte of the info buffer at worst.
    unsafe { *info.add(to_copy) = b'\0' };
    err.set_info_length(to_copy);
    // SAFETY: `info` is writable for `info_length` bytes with the terminating
    // NUL written just above at `info[info_length]`.
    let info = unsafe { core::slice::from_raw_parts_mut(info, err.info_length()) };
    clean_string_utf8(info);
}

// ufbx.c:3510-3519 `ufbxi_fmt_err_info` (variadic entry point — see
// `ufbxi_fmt_err_info!`).
// C: `va_list args; // ufbxi_uninit` (ufbx.c:3514) — collapsed into `args`.
// C's nullable `ufbx_error *err` is an `Option<&ErrorView>`; the `!err`
// early-out is the `None` arm.
//
// # Safety
// `fmt` must be a NUL-terminated format string whose conversions match `args`
// one-for-one — including every `%s` argument being a NUL-terminated run. That
// pairing is prose in C and no parameter type carries it.
#[inline(never)]
pub(crate) unsafe fn fmt_err_info(err: Option<&ErrorView>, fmt: *const u8, args: &[PrintArg]) {
    let Some(err) = err else {
        return;
    };

    let info = err.info_mut_ptr();
    // SAFETY: `info` is the error's own buffer, exactly ERROR_INFO_LENGTH
    // bytes, inside the live `Error` the view was minted over (its mint
    // invariant); `fmt`/`args` carry this fn's format-string contract.
    let info_length = unsafe { vsnprintf(info, ERROR_INFO_LENGTH, fmt, args) } as usize;
    err.set_info_length(info_length);
    // SAFETY: `vsnprintf` returns at most `ERROR_INFO_LENGTH - 1` and
    // NUL-terminates at that length, so `info` is writable for `info_length`
    // bytes.
    let info = unsafe { core::slice::from_raw_parts_mut(info, err.info_length()) };
    clean_string_utf8(info);
}

// Call-site wrapper building the `&[PrintArg]` argument pack.
macro_rules! ufbxi_fmt_err_info {
    ($err:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::native::error::fmt_err_info($err, concat!($fmt, "\0").as_ptr(),
            &[$($crate::native::printf::PrintArg::from($arg)),*])
    };
}
pub(crate) use ufbxi_fmt_err_info;

// ufbx.c:3521-3531 `ufbxi_clear_error`
// C's nullable `ufbx_error *err` is an `Option<&ErrorView>`; the `!err`
// early-out is the `None` arm.
#[inline(never)]
pub(crate) fn clear_error(err: Option<&ErrorView>) {
    let Some(err) = err else {
        return;
    };

    err.set_type_(ErrorType::None);
    let description = err.description_view();
    description.set_data(EMPTY_CHAR.as_ptr());
    description.set_length(0);
    err.set_stack_size(0);
    // SAFETY: writing the first byte of the error's own info buffer, which is
    // ERROR_INFO_LENGTH (>= 1) bytes long and sits inside the live `Error` this
    // view was minted from (the view's write-capable mint invariant).
    unsafe { *err.info_mut_ptr() = b'\0' };
    err.set_info_length(0);
}

// ufbx.c:3532-3541
//   #if UFBXI_FEATURE_ERROR_STACK: ufbxi_function = __FUNCTION__, ufbxi_line =
//   __LINE__, ufbxi_cond_str(cond) = #cond; else NULL / 0 / "".
// Rust: `module_path!()` stands in for `__FUNCTION__`, with `::` collapsed to
// `.` — the string must be colon-free (see module docs).
#[cfg(feature = "error-stack")]
pub(crate) const fn function_name<const N: usize>(src: &str) -> [u8; N] {
    let bytes = src.as_bytes();
    let mut out = [0u8; N];
    let mut i = 0;
    let mut o = 0;
    while i < bytes.len() {
        if bytes[i] == b':' {
            // Collapse `::` path separators to a single `.`.
            out[o] = b'.';
            o += 1;
            while i < bytes.len() && bytes[i] == b':' {
                i += 1;
            }
        } else {
            out[o] = bytes[i];
            o += 1;
            i += 1;
        }
    }
    // Remaining bytes stay 0 — the string is consumed via `strlen`.
    out
}

#[cfg(feature = "error-stack")]
macro_rules! ufbxi_function {
    () => {{
        const LEN: usize = module_path!().len() + 1;
        static NAME: [u8; LEN] = $crate::native::error::function_name::<LEN>(module_path!());
        Some($crate::native::error::FailStr::new(&NAME))
    }};
}
#[cfg(not(feature = "error-stack"))]
macro_rules! ufbxi_function {
    () => {
        Option::<$crate::native::error::FailStr>::None
    };
}
pub(crate) use ufbxi_function;

#[cfg(feature = "error-stack")]
macro_rules! ufbxi_line {
    () => {
        line!()
    };
}
#[cfg(not(feature = "error-stack"))]
macro_rules! ufbxi_line {
    () => {
        0u32
    };
}
pub(crate) use ufbxi_line;

// `ufbxi_cond_str(cond)` as a NUL-terminated literal (C string literals carry
// the NUL implicitly). Stringifies WITHOUT evaluating.
// The optional second literal is the VERBATIM C condition text, used where
// Rust `stringify!` diverges from C's `#cond` bytes (checklist #13).
#[cfg(feature = "error-stack")]
macro_rules! ufbxi_cond_str {
    ($cond:expr) => {
        concat!(stringify!($cond), "\0")
    };
    ($cond:expr, $c_cond_str:literal) => {
        concat!($c_cond_str, "\0")
    };
}
#[cfg(not(feature = "error-stack"))]
macro_rules! ufbxi_cond_str {
    ($cond:expr) => {
        "\0"
    };
    ($cond:expr, $c_cond_str:literal) => {
        "\0"
    };
}
pub(crate) use ufbxi_cond_str;

// ufbx.c:3543-3548 `ufbxi_fail_err_no_msg(err, cond, func, line)`
// Takes the already-stringified condition / description pointer expression;
// the no-stack form drops it unevaluated (it is always a literal `.as_ptr()`).
#[cfg(feature = "error-stack")]
macro_rules! ufbxi_fail_err_no_msg {
    ($err:expr, $cond_str:expr) => {
        $crate::native::error::fail_err(
            $err,
            $cond_str,
            $crate::native::error::ufbxi_function!(),
            $crate::native::error::ufbxi_line!(),
        )
    };
}
#[cfg(not(feature = "error-stack"))]
macro_rules! ufbxi_fail_err_no_msg {
    ($err:expr, $cond_str:expr) => {
        $crate::native::error::fail_err_no_stack($err)
    };
}
pub(crate) use ufbxi_fail_err_no_msg;

// ufbx.c:3546 (no-stack branch helper)
#[cfg(not(feature = "error-stack"))]
#[inline(never)]
pub(crate) fn fail_imp_err_no_stack(err: &ErrorView) -> i32 {
    fail_imp_err(err, None, None, 0)
}

// `Fail`-minting wrapper over `fail_imp_err_no_stack`: the C body returns `0`
// (the int-shaped failure), the Rust callers need the `Fail` witness that an
// error was written to the target.
#[cfg(not(feature = "error-stack"))]
#[inline]
pub(crate) fn fail_err_no_stack(err: &ErrorView) -> Fail {
    fail_imp_err_no_stack(err);
    Fail(())
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
            return Err($crate::native::error::ufbxi_fail_err_no_msg!(
                $err,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_cond_str!($cond).as_bytes(),
                ))
            ));
        }
    }};
    ($err:expr, $cond:expr, $c_cond_str:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            return Err($crate::native::error::ufbxi_fail_err_no_msg!(
                $err,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_cond_str!($cond, $c_cond_str).as_bytes(),
                ))
            ));
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
            $crate::native::error::ufbxi_fail_err_no_msg!(
                $err,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_cond_str!($cond).as_bytes(),
                ))
            );
            return $ret;
        }
    }};
    ($err:expr, $cond:expr, $ret:expr, $c_cond_str:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::ufbxi_fail_err_no_msg!(
                $err,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_cond_str!($cond, $c_cond_str).as_bytes(),
                ))
            );
            return $ret;
        }
    }};
}
pub(crate) use ufbxi_check_return_err;

// ufbx.c:3552 `ufbxi_fail_err(err, desc)` — unconditional fail-and-return.
// `desc` is a verbatim string literal (all ufbx.c call sites are literals).
macro_rules! ufbxi_fail_err {
    ($err:expr, $desc:literal) => {{
        return Err($crate::native::error::ufbxi_fail_err_no_msg!(
            $err,
            Some($crate::native::error::FailStr::new(
                concat!($desc, "\0").as_bytes()
            ))
        ));
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
            return Err($crate::native::error::fail_err(
                $err,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_error_msg_cond!($cond, $msg).as_bytes(),
                )),
                $crate::native::error::ufbxi_function!(),
                $crate::native::error::ufbxi_line!(),
            ));
        }
    }};
    ($err:expr, $cond:expr, $msg:literal, $c_cond_str:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            return Err($crate::native::error::fail_err(
                $err,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_error_msg_cond!($cond, $msg, $c_cond_str)
                        .as_bytes(),
                )),
                $crate::native::error::ufbxi_function!(),
                $crate::native::error::ufbxi_line!(),
            ));
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
            $crate::native::error::fail_err(
                $err,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_error_msg_cond!($cond, $msg).as_bytes(),
                )),
                $crate::native::error::ufbxi_function!(),
                $crate::native::error::ufbxi_line!(),
            );
            return $ret;
        }
    }};
    ($err:expr, $cond:expr, $ret:expr, $msg:literal, $c_cond_str:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::fail_err(
                $err,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_error_msg_cond!($cond, $msg, $c_cond_str)
                        .as_bytes(),
                )),
                $crate::native::error::ufbxi_function!(),
                $crate::native::error::ufbxi_line!(),
            );
            return $ret;
        }
    }};
}
pub(crate) use ufbxi_check_return_err_msg;

// ufbx.c:3556 `ufbxi_fail_err_msg(err, desc, msg)` — desc is a VERBATIM
// literal, NOT a stringified condition (PORTING.md: do not mix these up).
macro_rules! ufbxi_fail_err_msg {
    ($err:expr, $desc:literal, $msg:literal) => {{
        return Err($crate::native::error::fail_err(
            $err,
            Some($crate::native::error::FailStr::new(
                $crate::native::error::ufbxi_error_msg!($desc, $msg).as_bytes(),
            )),
            $crate::native::error::ufbxi_function!(),
            $crate::native::error::ufbxi_line!(),
        ));
    }};
}
pub(crate) use ufbxi_fail_err_msg;

// ufbx.c:3557 `ufbxi_report_err_msg(err, desc, msg)` — records the error and
// KEEPS GOING (C: `(void)` result). NOT a return (PORTING.md trap #16).
macro_rules! ufbxi_report_err_msg {
    ($err:expr, $desc:literal, $msg:literal) => {{
        $crate::native::error::fail_err(
            $err,
            Some($crate::native::error::FailStr::new(
                $crate::native::error::ufbxi_error_msg!($desc, $msg).as_bytes(),
            )),
            $crate::native::error::ufbxi_function!(),
            $crate::native::error::ufbxi_line!(),
        )
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
        $crate::native::parse::fail_imp(
            $uc,
            $cond_str,
            $crate::native::error::ufbxi_function!(),
            $crate::native::error::ufbxi_line!(),
        )
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
// Optional trailing literal: the verbatim C condition text (see `ufbxi_cond_str`).
macro_rules! ufbxi_check {
    ($uc:expr, $cond:expr) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            return Err($crate::native::error::ufbxi_fail_no_msg!(
                $uc,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_cond_str!($cond).as_bytes(),
                ))
            ));
        }
    }};
    ($uc:expr, $cond:expr, $c_cond_str:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            return Err($crate::native::error::ufbxi_fail_no_msg!(
                $uc,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_cond_str!($cond, $c_cond_str).as_bytes(),
                ))
            ));
        }
    }};
}
pub(crate) use ufbxi_check;

// Port-local: `ufbxi_check(cond)` over an `Option`. The typed value fetches
// (`get_val*` / `find_val*`) return the value instead of writing an
// out-pointer, so the check yields it on `Some` and fails exactly as
// `ufbxi_check` does on `None`. The trailing literal is the verbatim C
// condition text (see `ufbxi_cond_str`).
macro_rules! ufbxi_check_some {
    ($uc:expr, $opt:expr, $c_cond_str:literal) => {{
        match $opt {
            Some(v) => v,
            None => {
                return Err($crate::native::error::ufbxi_fail_no_msg!(
                    $uc,
                    Some($crate::native::error::FailStr::new(
                        $crate::native::error::ufbxi_cond_str!($opt, $c_cond_str).as_bytes(),
                    ))
                ));
            }
        }
    }};
}
pub(crate) use ufbxi_check_some;

// ufbx.c:6665 `ufbxi_check_return(cond, ret)`
// Optional trailing literal: the verbatim C condition text (see `ufbxi_cond_str`).
macro_rules! ufbxi_check_return {
    ($uc:expr, $cond:expr, $ret:expr) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::ufbxi_fail_no_msg!(
                $uc,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_cond_str!($cond).as_bytes(),
                ))
            );
            return $ret;
        }
    }};
    ($uc:expr, $cond:expr, $ret:expr, $c_cond_str:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::error::ufbxi_fail_no_msg!(
                $uc,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_cond_str!($cond, $c_cond_str).as_bytes(),
                ))
            );
            return $ret;
        }
    }};
}
pub(crate) use ufbxi_check_return;

// ufbx.c:6666 `ufbxi_fail(desc)`
macro_rules! ufbxi_fail {
    ($uc:expr, $desc:literal) => {{
        return Err($crate::native::error::ufbxi_fail_no_msg!(
            $uc,
            Some($crate::native::error::FailStr::new(
                concat!($desc, "\0").as_bytes()
            ))
        ));
    }};
}
pub(crate) use ufbxi_fail;

// ufbx.c:6667 `ufbxi_fail_return(desc, ret)`
macro_rules! ufbxi_fail_return {
    ($uc:expr, $desc:literal, $ret:expr) => {{
        $crate::native::error::ufbxi_fail_no_msg!(
            $uc,
            Some($crate::native::error::FailStr::new(
                concat!($desc, "\0").as_bytes()
            ))
        );
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
            return Err($crate::native::parse::fail_imp(
                $uc,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_error_msg_cond!($cond, $msg).as_bytes(),
                )),
                $crate::native::error::ufbxi_function!(),
                $crate::native::error::ufbxi_line!(),
            ));
        }
    }};
    ($uc:expr, $cond:expr, $msg:literal, $c_cond_str:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            return Err($crate::native::parse::fail_imp(
                $uc,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_error_msg_cond!($cond, $msg, $c_cond_str)
                        .as_bytes(),
                )),
                $crate::native::error::ufbxi_function!(),
                $crate::native::error::ufbxi_line!(),
            ));
        }
    }};
}
pub(crate) use ufbxi_check_msg;

// ufbx.c:6670 `ufbxi_check_return_msg(cond, ret, msg)`
macro_rules! ufbxi_check_return_msg {
    ($uc:expr, $cond:expr, $ret:expr, $msg:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::parse::fail_imp(
                $uc,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_error_msg_cond!($cond, $msg).as_bytes(),
                )),
                $crate::native::error::ufbxi_function!(),
                $crate::native::error::ufbxi_line!(),
            );
            return $ret;
        }
    }};
    ($uc:expr, $cond:expr, $ret:expr, $msg:literal, $c_cond_str:literal) => {{
        let cond = $cond;
        if $crate::native::platform::unlikely(!cond) {
            $crate::native::parse::fail_imp(
                $uc,
                Some($crate::native::error::FailStr::new(
                    $crate::native::error::ufbxi_error_msg_cond!($cond, $msg, $c_cond_str)
                        .as_bytes(),
                )),
                $crate::native::error::ufbxi_function!(),
                $crate::native::error::ufbxi_line!(),
            );
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
        return Err($crate::native::parse::fail_imp(
            $uc,
            Some($crate::native::error::FailStr::new(
                $crate::native::error::ufbxi_error_msg!($desc, $msg).as_bytes(),
            )),
            $crate::native::error::ufbxi_function!(),
            $crate::native::error::ufbxi_line!(),
        ));
    }};
}
pub(crate) use ufbxi_fail_msg;

// ufbx.c:3559-3612 `ufbxi_fix_error_type`
// The strcmp ladder, called from the top-level entry points; `default_desc` is
// the per-entry-point default description ("Failed to load", "Failed to
// evaluate", ...) substituted when none was set. All literals are part of
// byte-exact error parity (PORTING.md trap #13).
#[inline(never)]
pub(crate) fn fix_error_type(
    error: &ErrorView,
    default_desc: &'static [u8],
    p_error: Option<&ErrorView>,
) {
    let desc_ptr = error.description_view().data();
    let desc: &[u8] = if desc_ptr.is_null() {
        default_desc
    } else {
        // SAFETY: the recorded description is always a NUL-terminated 'static
        // run from a `FailStr`, so `strlen` finds its terminator and the run
        // stays live for this whole fn.
        unsafe { crate::prelude::slice_from_ptr(desc_ptr, strlen(desc_ptr)) }
    };
    error.set_type_(ErrorType::Unknown);
    if c_strcmp(desc, b"Out of memory\0") == 0 {
        error.set_type_(ErrorType::OutOfMemory);
    } else if c_strcmp(desc, b"Memory limit exceeded\0") == 0 {
        error.set_type_(ErrorType::MemoryLimit);
    } else if c_strcmp(desc, b"Allocation limit exceeded\0") == 0 {
        error.set_type_(ErrorType::AllocationLimit);
    } else if c_strcmp(desc, b"Truncated file\0") == 0 {
        error.set_type_(ErrorType::TruncatedFile);
    } else if c_strcmp(desc, b"IO error\0") == 0 {
        error.set_type_(ErrorType::Io);
    } else if c_strcmp(desc, b"Cancelled\0") == 0 {
        error.set_type_(ErrorType::Cancelled);
    } else if c_strcmp(desc, b"Unrecognized file format\0") == 0 {
        error.set_type_(ErrorType::UnrecognizedFileFormat);
    } else if c_strcmp(desc, b"File not found\0") == 0 {
        error.set_type_(ErrorType::FileNotFound);
    } else if c_strcmp(desc, b"Empty file\0") == 0 {
        error.set_type_(ErrorType::EmptyFile);
    } else if c_strcmp(desc, b"External file not found\0") == 0 {
        error.set_type_(ErrorType::ExternalFileNotFound);
    } else if c_strcmp(desc, b"Uninitialized options\0") == 0 {
        error.set_type_(ErrorType::UninitializedOptions);
    } else if c_strcmp(desc, b"Zero vertex size\0") == 0 {
        error.set_type_(ErrorType::ZeroVertexSize);
    } else if c_strcmp(desc, b"Truncated vertex stream\0") == 0 {
        error.set_type_(ErrorType::TruncatedVertexStream);
    } else if c_strcmp(desc, b"Invalid UTF-8\0") == 0 {
        error.set_type_(ErrorType::InvalidUtf8);
    } else if c_strcmp(desc, b"Feature disabled\0") == 0 {
        error.set_type_(ErrorType::FeatureDisabled);
    } else if c_strcmp(desc, b"Bad NURBS geometry\0") == 0 {
        error.set_type_(ErrorType::BadNurbs);
    } else if c_strcmp(desc, b"Bad index\0") == 0 {
        error.set_type_(ErrorType::BadIndex);
    } else if c_strcmp(desc, b"Node depth limit exceeded\0") == 0 {
        error.set_type_(ErrorType::NodeDepthLimit);
    } else if c_strcmp(desc, b"Threaded ASCII parse error\0") == 0 {
        error.set_type_(ErrorType::ThreadedAsciiParse);
    } else if c_strcmp(desc, b"Unsafe options\0") == 0 {
        error.set_type_(ErrorType::UnsafeOptions);
    } else if c_strcmp(desc, b"Duplicate override\0") == 0 {
        error.set_type_(ErrorType::DuplicateOverride);
    }
    error.description_view().set_data(desc.as_ptr());
    // C: `error->description.length = strlen(desc);` — the default literal
    // carries its trailing NUL inside the slice, so cut at the first NUL.
    error
        .description_view()
        .set_length(desc.iter().position(|&b| b == 0).unwrap_or(desc.len()));
    if let Some(p_error) = p_error {
        // memcpy(p_error, error, sizeof(ufbx_error));
        // Every call site passes two distinct objects (the context error vs.
        // the caller's out-slot), which `debug_assert_ne!` pins down; the copy
        // itself is a `copy` (memmove), not `copy_nonoverlapping`, so that the
        // SAFE signature carries no unwritable non-overlap obligation — a
        // shared `&ErrorView` cannot express distinctness, and the two
        // primitives agree byte-for-byte whenever the slots are disjoint.
        debug_assert_ne!(error.get(), p_error.get());
        // SAFETY: both views were minted over live, write-capable `Error`
        // objects (the `View::from_ptr` contract), so each addresses
        // `size_of::<Error>()` accessible bytes; `copy` is defined for any
        // overlap, so no aliasing precondition is left to the caller.
        unsafe { core::ptr::copy(error.get() as *const Error, p_error.get(), 1) };
    }
}

// -- Options-validation guards (ufbx.c:30302-30328; C defines these in the
// `-- Setup` / API prelude — hosted here with the rest of the macro family)

// ufbx.c:30302-30311 `ufbxi_uninitialized_options`
#[inline(never)]
pub(crate) fn uninitialized_options(p_error: Option<&ErrorView>) -> *mut core::ffi::c_void {
    if let Some(p_error) = p_error {
        // SAFETY: the view was minted over a live, write-capable `Error` slot
        // (the `View::from_ptr` mint invariant), so exactly
        // `size_of::<Error>()` bytes are writable at `get()`.
        unsafe {
            core::ptr::write_bytes(p_error.get() as *mut u8, 0, core::mem::size_of::<Error>())
        };
        p_error.set_type_(ErrorType::UninitializedOptions);
        p_error
            .description_view()
            .set_data(b"Uninitialized options\0".as_ptr());
        // SAFETY: the argument is a NUL-terminated byte literal.
        p_error
            .description_view()
            .set_length(unsafe { strlen(b"Uninitialized options\0".as_ptr()) });
    }
    core::ptr::null_mut()
}

/// The formatted "Uninitialized options" error as a VALUE, for the surface
/// entries that return `Result<T, Error>` (PORTING.md "Trailing
/// `ufbx_error *error` out-params"): same bytes `uninitialized_options`
/// writes into the caller slot, carried in the `Err` instead.
pub(crate) fn uninitialized_options_value() -> Error {
    Error {
        type_: ErrorType::UninitializedOptions,
        description: crate::prelude::String::new_c(
            b"Uninitialized options\0".as_ptr(),
            b"Uninitialized options".len(),
        ),
        ..Error::default()
    }
}

// `ufbxi_check_opts_ptr` flavor for `Result<T, Error>`-shaped surface entries:
// yields `Err` by value instead of writing the caller slot (the boundary shim
// owns the slot writes).
macro_rules! ufbxi_check_opts_res {
    ($m_opts:expr) => {{
        let m_opts = $m_opts;
        if !m_opts.is_null() {
            let opts_cleared_to_zero = (*m_opts)._begin_zero | (*m_opts)._end_zero;
            $crate::native::platform::ufbx_assert!(opts_cleared_to_zero == 0);
            if opts_cleared_to_zero != 0 {
                return Err($crate::native::error::uninitialized_options_value());
            }
        }
    }};
}
pub(crate) use ufbxi_check_opts_res;

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
            if opts_cleared_to_zero != 0 {
                return $m_value;
            }
        }
    }};
}
pub(crate) use ufbxi_check_opts_return_no_error;

// Typed interior-mutable VIEW over an `Error` field, reinterpreted in place.
pub(crate) type ErrorView = crate::native::view::View<Error>;

impl ErrorView {
    #[inline(always)]
    pub(crate) fn type_(&self) -> crate::generated::ErrorType {
        view_read!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn set_type_(&self, type_: crate::generated::ErrorType) {
        view_write!(self, type_, type_)
    }
    #[inline(always)]
    pub(crate) fn stack_size(&self) -> u32 {
        view_read!(self, stack_size)
    }
    #[inline(always)]
    pub(crate) fn set_stack_size(&self, stack_size: u32) {
        view_write!(self, stack_size, stack_size)
    }
    /// One frame of the error's own `stack` array, as a view (C:
    /// `&err->stack[index]`).
    ///
    /// # Safety
    /// `index` must be `< ERROR_STACK_MAX_DEPTH` — the array extent, which the
    /// leaf macros cannot bound because the place is a field *and* an index.
    #[inline(always)]
    pub(crate) unsafe fn stack_frame_view(&self, index: usize) -> &ErrorFrameView {
        // SAFETY: `stack` is an `[ErrorFrame; ERROR_STACK_MAX_DEPTH]` inside the
        // live, write-capable `Error` this view was minted over (the mint
        // invariant), so the array projection plus an in-bounds `index` (fn
        // contract above) addresses one live `ErrorFrame` slot in the same
        // allocation.
        unsafe {
            ErrorFrameView::from_ptr(
                (&raw mut (*self.get()).stack)
                    .cast::<ErrorFrame>()
                    .add(index),
            )
        }
    }
    #[inline(always)]
    pub(crate) fn info_length(&self) -> usize {
        view_read!(self, info_length)
    }
    #[inline(always)]
    pub(crate) fn set_info_length(&self, info_length: usize) {
        view_write!(self, info_length, info_length)
    }
    /// The error's own inline info buffer, as a writable byte pointer
    /// (ERROR_INFO_LENGTH bytes).
    #[inline(always)]
    pub(crate) fn info_mut_ptr(&self) -> *mut u8 {
        view_raw_mut!(self, info_buf) as *mut u8
    }
    #[inline(always)]
    pub(crate) fn description_view(&self) -> &crate::prelude::StringView {
        // SAFETY: `View` is `#[repr(transparent)]` over its storage, so the
        // field pointer reinterprets in place as a `StringView`; the field sits
        // inside the live `Error` this view was minted from, keeping the derived
        // `&StringView` valid for the lifetime of `&self`.
        unsafe { &*(&raw mut (*self.get()).description as *mut crate::prelude::StringView) }
    }
}

// Typed interior-mutable VIEW over one `ErrorFrame` of an `Error`'s stack.
pub(crate) type ErrorFrameView = crate::native::view::View<ErrorFrame>;

impl ErrorFrameView {
    #[inline(always)]
    pub(crate) fn set_source_line(&self, source_line: u32) {
        view_write!(self, source_line, source_line)
    }
    #[inline(always)]
    pub(crate) fn function_view(&self) -> &crate::prelude::StringView {
        view_project!(self, function)
    }
    #[inline(always)]
    pub(crate) fn description_view(&self) -> &crate::prelude::StringView {
        view_project!(self, description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::printf::PrintArg;

    fn desc_bytes(err: &Error) -> &[u8] {
        // SAFETY: every `err` inspected here was filled by an error setter in
        // this module, whose descriptions are `'static` literals — so the
        // `data`/`length` pair is a live run and outlives the borrow of `err`.
        unsafe { core::slice::from_raw_parts(err.description.data, err.description.length) }
    }

    #[test]
    fn test_snprintf_macro() {
        let mut buf = [0u8; 32];
        // SAFETY: `buf.as_mut_ptr()`/`buf.len()` describe the local array
        // exactly, and the `%s` argument is a NUL-terminated byte literal.
        let len = unsafe {
            ufbxi_snprintf!(
                buf.as_mut_ptr(),
                buf.len(),
                "Frame%uTick%u.%s",
                12u32,
                7u32,
                b"pc2\0".as_ptr()
            )
        };
        assert_eq!(len, 16);
        assert_eq!(&buf[..len as usize], b"Frame12Tick7.pc2");
        assert_eq!(buf[len as usize], 0);
    }

    #[test]
    fn test_snprintf_truncation_length() {
        let mut buf = [0xAAu8; 8];
        // SAFETY: `buf.as_mut_ptr()`/`buf.len()` describe the local array
        // exactly, and the `%s` argument is a NUL-terminated byte literal.
        let len =
            unsafe { ufbxi_snprintf!(buf.as_mut_ptr(), buf.len(), "%s", b"0123456789\0".as_ptr()) };
        // pos saturates at length (8); returned min(pos, size - 1) = 7.
        assert_eq!(len, 7);
        assert_eq!(&buf[..8], b"0123456\0");
    }

    fn checked_fn(err: *mut Error, ok: bool, hits: &mut u32) -> Result<u32, Fail> {
        // Evaluation-once: the condition increments `hits` exactly once.
        ufbxi_check_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(err) },
            {
                *hits += 1;
                ok
            },
            "Out of memory"
        );
        Ok(42)
    }

    #[test]
    fn test_check_err_msg_and_first_error_wins() {
        let mut err = Error::default();
        let mut hits = 0u32;
        assert_eq!(checked_fn(&mut err, true, &mut hits), Ok(42));
        assert_eq!(hits, 1);
        assert!(err.description.data.is_null());

        assert!(checked_fn(&mut err, false, &mut hits).is_err());
        assert_eq!(hits, 2);
        assert_eq!(desc_bytes(&err), b"Out of memory");

        // First error wins: a second failure does not overwrite.
        fn fail2(err: *mut Error) -> Result<(), Fail> {
            ufbxi_check_err_msg!(
                unsafe { crate::native::error::ErrorView::from_ptr(err) },
                false,
                "Truncated file"
            );
            Ok(())
        }
        assert!(fail2(&mut err).is_err());
        assert_eq!(desc_bytes(&err), b"Out of memory");

        let mut p_error = Error::default();
        fix_error_type(
            ErrorView::from_mut(&mut err),
            b"Failed to load\0",
            Some(ErrorView::from_mut(&mut p_error)),
        );
        assert_eq!(err.type_, ErrorType::OutOfMemory);
        assert_eq!(desc_bytes(&err), b"Out of memory");
        assert_eq!(p_error.type_, ErrorType::OutOfMemory);
    }

    #[test]
    fn test_fix_error_type_default_desc() {
        // No description set -> per-entry-point default, type Unknown.
        let mut err = Error::default();
        fix_error_type(ErrorView::from_mut(&mut err), b"Failed to evaluate\0", None);
        assert_eq!(err.type_, ErrorType::Unknown);
        assert_eq!(desc_bytes(&err), b"Failed to evaluate");

        // Ladder entries map byte-exactly.
        let mut err = Error::default();
        err.description.data = b"Threaded ASCII parse error\0".as_ptr();
        err.description.length = 26;
        fix_error_type(ErrorView::from_mut(&mut err), b"Failed to load\0", None);
        assert_eq!(err.type_, ErrorType::ThreadedAsciiParse);
    }

    #[test]
    fn test_report_err_msg_keeps_going() {
        let mut err = Error::default();
        // Sentinel: stays `false` iff the macro early-returns (what this test forbids),
        // so the initial value is load-bearing on the failure path.
        #[allow(unused_assignments)]
        let mut reached = false;
        {
            ufbxi_report_err_msg!(
                crate::native::error::ErrorView::from_mut(&mut err),
                "ptr",
                "Out of memory"
            );
            reached = true;
        };
        assert!(reached, "ufbxi_report_err_msg must not return early");
        assert_eq!(desc_bytes(&err), b"Out of memory");
    }

    #[test]
    fn test_fail_err_and_check_return_err() {
        {
            // ufbxi_fail_err with a desc that is NOT '$'-prefixed sets no
            // description (only the stack frame, when enabled).
            let mut err = Error::default();
            fn f(err: *mut Error) -> Result<(), Fail> {
                ufbxi_fail_err!(
                    unsafe { crate::native::error::ErrorView::from_ptr(err) },
                    "Task failed"
                );
            }
            assert!(f(&mut err).is_err());
            assert!(err.description.data.is_null());
            #[cfg(feature = "error-stack")]
            unsafe {
                assert_eq!(err.stack_size, 1);
                assert_eq!(
                    core::slice::from_raw_parts(
                        err.stack[0].description.data,
                        err.stack[0].description.length
                    ),
                    b"Task failed"
                );
            }

            // check_return_err returns the given value verbatim.
            let mut err = Error::default();
            fn g(err: *mut Error) -> u32 {
                ufbxi_check_return_err!(
                    unsafe { crate::native::error::ErrorView::from_ptr(err) },
                    false,
                    7
                );
                1
            }
            assert_eq!(g(&mut err), 7);
        }
    }

    #[test]
    fn test_clear_error_and_fmt_err_info() {
        unsafe {
            let mut err = Error::default();
            clear_error(Some(ErrorView::from_mut(&mut err)));
            assert_eq!(err.type_, ErrorType::None);
            assert_eq!(err.description.data, EMPTY_CHAR.as_ptr());
            assert_eq!(err.description.length, 0);
            assert_eq!(err.info(), "");

            let view = ErrorView::from_mut(&mut err);
            ufbxi_fmt_err_info!(Some(view), "%u (max %u)", 5u32, 3u32);
            assert_eq!(err.info(), "5 (max 3)");

            let view = ErrorView::from_mut(&mut err);
            set_err_info(Some(view), b"UFBX_ENABLE_FORMAT_OBJ".as_ptr(), 22);
            assert_eq!(err.info(), "UFBX_ENABLE_FORMAT_OBJ");
            // SIZE_MAX length -> strlen
            let view = ErrorView::from_mut(&mut err);
            set_err_info(Some(view), b"abc\0".as_ptr(), usize::MAX);
            assert_eq!(err.info(), "abc");
        }
    }

    #[test]
    fn test_utf8_valid_length_and_clean() {
        // ASCII
        assert_eq!(utf8_valid_length(b"hello"), 5);
        // NUL stops the scan
        assert_eq!(utf8_valid_length(b"he\0lo"), 2);
        // 2-byte: U+00E4, overlong C1 80 rejected
        assert_eq!(utf8_valid_length(b"\xc3\xa4"), 2);
        assert_eq!(utf8_valid_length(b"\xc1\x80"), 0);
        // 3-byte: U+20AC valid; UTF-16 surrogate ED A0 80 rejected
        assert_eq!(utf8_valid_length(b"\xe2\x82\xac"), 3);
        assert_eq!(utf8_valid_length(b"\xed\xa0\x80"), 0);
        // 4-byte: U+1F600 valid; > U+10FFFF rejected
        assert_eq!(utf8_valid_length(b"\xf0\x9f\x98\x80"), 4);
        assert_eq!(utf8_valid_length(b"\xf4\x90\x80\x80"), 0);

        // clean_string_utf8 replaces each invalid byte with '?'
        let mut s = *b"a\xffb\xc3\xa4\xed\xa0\x80";
        clean_string_utf8(&mut s);
        assert_eq!(&s, b"a?b\xc3\xa4???");
    }

    #[test]
    fn test_panicf() {
        let mut storage = Panic::default();
        let mut panic = Some(&mut storage);
        let fired = ufbxi_panicf!(
            panic,
            1 < 2,
            "vertex (%zu) out of bounds (%zu)",
            5usize,
            3usize
        );
        assert!(!fired);

        let fired = ufbxi_panicf!(
            panic,
            false,
            "vertex (%zu) out of bounds (%zu)",
            5usize,
            3usize
        );
        assert!(fired);

        // Already-panicked: message preserved
        let fired = ufbxi_panicf!(panic, false, "other");
        assert!(fired);
        assert!(storage.did_panic);
        assert_eq!(storage.message(), "vertex (5) out of bounds (3)");
    }

    #[test]
    fn test_uninitialized_options() {
        let mut err = Error {
            type_: ErrorType::Io,
            ..Default::default()
        };
        let err_view = ErrorView::from_mut(&mut err);
        let ret = uninitialized_options(Some(err_view));
        assert!(ret.is_null());
        assert_eq!(err.type_, ErrorType::UninitializedOptions);
        assert_eq!(desc_bytes(&err), b"Uninitialized options");
        assert!(uninitialized_options(None).is_null());
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
