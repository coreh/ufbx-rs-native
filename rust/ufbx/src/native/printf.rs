//! Port of the `// -- Printf` banner section (ufbx.c:3280-3362).
//!
//! ufbx.c ships its own printf with a restricted format subset (`%s`, `%u`,
//! `%zu`, plus `*` / `.*` width/precision and `%%`); `ufbxi_vprint` calls
//! `ufbxi_unreachable("Bad printf format")` on anything else. Ported
//! byte-for-byte per PORTING.md "Printf and variadics": NEVER substitute
//! `format!`/`write!` — truncation semantics and the returned length feed
//! `ufbx_error.info_length` and geometry-cache frame filenames.
//!
//! C variadics (`va_list`/`va_arg`) have no Rust analogue: variadic entry
//! points take `&[PrintArg]` slices built by `macro_rules!` wrappers at call
//! sites (the wrappers live with their C entry points, e.g.
//! `ufbxi_snprintf!` / `ufbxi_fmt_err_info!` / `ufbxi_panicf!` in the error
//! module). `PrintArg` mirrors the exact `va_arg` pulls the C performs:
//! `int` (for `*` / `.*` widths), `uint32_t` (`%u`), `size_t` (`%zu`) and
//! `const char *` (`%s`).
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
use crate::native::platform::{ufbxi_dev_assert, ufbxi_unreachable};

// C `va_list` replacement (PORTING.md "Printf and variadics"): one variant per
// `va_arg` type the restricted format subset can pull. Slices of these are the
// ONE sanctioned slice use in ported paths.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PrintArg<'a> {
    /// `va_arg(args, int)` — `*` / `.*` width and precision arguments.
    Int(i32),
    /// `va_arg(args, uint32_t)` — `%u`.
    Uint(u32),
    /// `va_arg(args, size_t)` — `%zu`.
    Size(usize),
    /// `va_arg(args, const char*)` — `%s`. C's pointer is carried as the byte
    /// run it addresses: `print_append` stops at the first NUL, at the `%.*s`
    /// precision, or at the end of the slice (an exhausted slice reads as the
    /// terminator, the `c_strcmp` convention). For a NUL-terminated C string
    /// borrowed up to (or through) its terminator that is byte-for-byte the C
    /// scan, and the slice borrow carries the readability the scan needs.
    Str(&'a [u8]),
}

impl From<i32> for PrintArg<'_> {
    fn from(v: i32) -> Self {
        PrintArg::Int(v)
    }
}
impl From<u32> for PrintArg<'_> {
    fn from(v: u32) -> Self {
        PrintArg::Uint(v)
    }
}
impl From<usize> for PrintArg<'_> {
    fn from(v: usize) -> Self {
        PrintArg::Size(v)
    }
}
impl<'a> From<&'a [u8]> for PrintArg<'a> {
    fn from(v: &'a [u8]) -> PrintArg<'a> {
        PrintArg::Str(v)
    }
}
impl<'a, const N: usize> From<&'a [u8; N]> for PrintArg<'a> {
    fn from(v: &'a [u8; N]) -> PrintArg<'a> {
        PrintArg::Str(v)
    }
}
impl From<&'static str> for PrintArg<'static> {
    // Convenience for static literals at call sites (C passes string literals
    // directly as `const char *`).
    fn from(v: &'static str) -> PrintArg<'static> {
        PrintArg::Str(v.as_bytes())
    }
}

impl<'a> PrintArg<'a> {
    /// `%s` argument from a raw C string: the run from `str_` up to its NUL.
    ///
    /// # Safety
    /// `str_` must point at a NUL-terminated byte run that stays live and
    /// unwritten for `'a`.
    pub(crate) unsafe fn c_str(str_: *const u8) -> PrintArg<'a> {
        // SAFETY: fn contract above — every byte up to the terminator is
        // readable, which is exactly the run `strlen` measures.
        PrintArg::Str(unsafe {
            crate::prelude::slice_from_ptr(str_, crate::native::error::strlen(str_))
        })
    }

    // The `as_*` accessors are the `va_arg` reinterpretations. A wrong-type
    // pull is UB in C (garbage read); here it trips "Bad printf format" and
    // yields a defined dummy so execution continues if asserts are off.
    fn as_int(self) -> i32 {
        match self {
            PrintArg::Int(v) => v,
            _ => {
                ufbxi_unreachable!("Bad printf format");
                0
            }
        }
    }
    fn as_u32(self) -> u32 {
        match self {
            PrintArg::Uint(v) => v,
            _ => {
                ufbxi_unreachable!("Bad printf format");
                0
            }
        }
    }
    fn as_size(self) -> usize {
        match self {
            PrintArg::Size(v) => v,
            _ => {
                ufbxi_unreachable!("Bad printf format");
                0
            }
        }
    }
    fn as_str(self) -> &'a [u8] {
        match self {
            PrintArg::Str(v) => v,
            _ => {
                ufbxi_unreachable!("Bad printf format");
                b""
            }
        }
    }
}

// `va_arg`: pull the next argument. Exhausting the argument list is UB in C;
// defined-but-asserting here.
fn va_arg<'a>(args: &[PrintArg<'a>], ix: &mut usize) -> PrintArg<'a> {
    if *ix < args.len() {
        let arg = args[*ix];
        *ix += 1;
        arg
    } else {
        ufbxi_unreachable!("Bad printf format");
        PrintArg::Int(0)
    }
}

// ufbx.c:3282-3286 `ufbxi_print_buffer`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PrintBuffer {
    pub(crate) dst: *mut u8,
    pub(crate) length: usize,
    pub(crate) pos: usize,
}

// ufbx.c:3288-3290
pub(crate) const PRINT_UNSIGNED: u32 = 0x1;
pub(crate) const PRINT_STRING: u32 = 0x2;
pub(crate) const PRINT_SIZE_T: u32 = 0x10;

// ufbx.c:3292-3305 `ufbxi_print_append`
pub(crate) unsafe fn print_append(
    buf: *mut PrintBuffer,
    min_width: usize,
    max_width: usize,
    str: &[u8],
) {
    // SAFETY: `buf` is a live, writable `PrintBuffer` (fn raw-param contract);
    // this borrow is the only access path to it for the rest of the fn.
    let buf = unsafe { &mut *buf };
    let mut width: usize = 0;
    // C scans to the NUL or `max_width`; the end of the slice reads as the
    // terminator (`PrintArg::Str`).
    while width < max_width {
        if width >= str.len() || str[width] == 0 {
            break;
        }
        width += 1;
    }
    let pad = min_width.saturating_sub(width);
    for _i in 0..pad {
        if buf.pos < buf.length {
            // SAFETY: `buf.dst` addresses `buf.length` writable bytes and the
            // guard above establishes `buf.pos < buf.length`.
            unsafe { *buf.dst.add(buf.pos) = b' ' };
            buf.pos += 1;
        }
    }
    for i in 0..width {
        if buf.pos < buf.length {
            // SAFETY: `buf.dst` addresses `buf.length` writable bytes with
            // `buf.pos < buf.length` established by the guard.
            unsafe { *buf.dst.add(buf.pos) = str[i] };
            buf.pos += 1;
        }
    }
}

// ufbx.c:3307-3316 `ufbxi_print_format_int`
// C formats backwards from one-past-the-end: `*--buffer = ...`.
pub(crate) unsafe fn print_format_int(mut buffer: *mut u8, mut value: u64) -> *mut u8 {
    // SAFETY: `buffer` is one past the end of a scratch buffer with room for the
    // terminator plus every decimal digit of `value` (callers pass the end of a
    // 96-byte array; a `u64` needs at most 20 digits), so stepping back one byte
    // and writing the NUL stays inside that buffer.
    buffer = unsafe { buffer.sub(1) };
    unsafe { *buffer = b'\0' };
    loop {
        let digit = (value % 10) as u32;
        value /= 10;
        // SAFETY: one step back per emitted digit, bounded by the digit count of
        // `value`, stays inside the same scratch buffer.
        buffer = unsafe { buffer.sub(1) };
        unsafe { *buffer = b'0' + digit as u8 };
        if !(value > 0) {
            break;
        }
    }
    buffer
}

// ufbx.c:3318-3362 `ufbxi_vprint`
// `fmt` is a NUL-terminated byte string (macro wrappers append the NUL);
// `args` replaces the C `va_list` (see module docs).
pub(crate) unsafe fn vprint(buf: *mut PrintBuffer, fmt: *const u8, args: &[PrintArg]) {
    let mut buffer = [0u8; 96]; // ufbxi_uninit
    let mut arg_ix: usize = 0;
    let mut p = fmt;
    // SAFETY (every `*p` read and `p.add` step below): `fmt` is a NUL-terminated
    // byte string (fn raw-param contract, macro wrappers append the NUL). Each
    // advance happens after reading a non-NUL byte at `p` — a format directive
    // always has at least its spec byte before the terminator — so `p` stays
    // inside the string, at worst on its NUL. The two-byte `.`/`*` step of
    // `%.*s` reads the byte after `.`, which the supported format subset
    // guarantees is `*` (dev-asserted).
    while unsafe { *p } != 0 {
        // C: `if (*p == '%' && *++p != '%')` — the increment happens only when
        // `*p == '%'`; on `%%` the else-branch then emits the second '%'.
        if unsafe { *p } == b'%' && {
            p = unsafe { p.add(1) };
            (unsafe { *p }) != b'%'
        } {
            let mut min_width: usize = 0;
            let mut max_width: usize = usize::MAX;
            if unsafe { *p } == b'*' {
                p = unsafe { p.add(1) };
                min_width = va_arg(args, &mut arg_ix).as_int() as usize;
            }
            if unsafe { *p } == b'.' {
                ufbxi_dev_assert!(unsafe { *p.add(1) } == b'*');
                p = unsafe { p.add(2) };
                max_width = va_arg(args, &mut arg_ix).as_int() as usize;
            }
            let mut flags: u32 = 0;
            if unsafe { *p } == b'z' {
                p = unsafe { p.add(1) };
                flags |= PRINT_SIZE_T;
            }
            let spec = unsafe { *p };
            p = unsafe { p.add(1) };
            match spec {
                b'u' => {
                    flags |= PRINT_UNSIGNED;
                }
                b's' => {
                    flags |= PRINT_STRING;
                }
                _ => {}
            }
            if (flags & PRINT_STRING) != 0 {
                let str = va_arg(args, &mut arg_ix).as_str();
                // SAFETY: `buf` is the live `PrintBuffer` this fn received.
                unsafe { print_append(buf, min_width, max_width, str) };
            } else if (flags & PRINT_UNSIGNED) != 0 {
                let value: u64 = if (flags & PRINT_SIZE_T) != 0 {
                    va_arg(args, &mut arg_ix).as_size() as u64
                } else {
                    va_arg(args, &mut arg_ix).as_u32() as u64
                };
                // SAFETY: `buffer` is a 96-byte local array, so the pointer is
                // one past its end and it holds room for the NUL plus every
                // digit of a `u64`.
                let str = unsafe { print_format_int(buffer.as_mut_ptr().add(buffer.len()), value) };
                // SAFETY: `print_format_int` returned the start of its
                // NUL-terminated digit run inside `buffer`.
                let start = unsafe { str.offset_from(buffer.as_ptr()) } as usize;
                // SAFETY: `buf` is the live `PrintBuffer` this fn received.
                unsafe { print_append(buf, min_width, max_width, &buffer[start..]) };
            } else {
                ufbxi_unreachable!("Bad printf format");
            }
        } else {
            // SAFETY: `buf` is a live, writable `PrintBuffer` (fn raw-param
            // contract); the borrow ends with this else-branch.
            let buf = unsafe { &mut *buf };
            if buf.pos < buf.length {
                // SAFETY: `p` addresses a byte inside the NUL-terminated `fmt`
                // (loop condition, or the second '%' just read on the `%%`
                // path), and `buf.dst` addresses `buf.length` writable bytes
                // with `buf.pos < buf.length` established by the guard.
                unsafe { *buf.dst.add(buf.pos) = *p };
                buf.pos += 1;
            }
            p = unsafe { p.add(1) };
        }
    }
    // SAFETY: `buf` is a live, writable `PrintBuffer` (fn raw-param contract).
    let buf = unsafe { &mut *buf };
    if buf.length != 0 && !buf.dst.is_null() {
        let end = if buf.pos < buf.length {
            buf.pos
        } else {
            buf.length - 1
        };
        // SAFETY: `buf.dst` addresses `buf.length` writable bytes (non-null and
        // non-empty per the guard), and `end` is either `buf.pos` when that is
        // below `buf.length` or `buf.length - 1`.
        unsafe { *buf.dst.add(end) = b'\0' };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn fmt(buf: &mut [u8], fmt: &str, args: &[PrintArg]) -> (usize, Vec<u8>) {
        let mut pb = PrintBuffer {
            dst: buf.as_mut_ptr(),
            length: buf.len(),
            pos: 0,
        };
        let fmt_nul: Vec<u8> = fmt.as_bytes().iter().copied().chain([0u8]).collect();
        // SAFETY: `pb` describes the caller's `buf` slice, `fmt_nul` is the
        // format string with a NUL appended.
        unsafe { vprint(&mut pb, fmt_nul.as_ptr(), args) };
        let nul = buf.iter().position(|&b| b == 0).unwrap();
        (pb.pos, buf[..nul].to_vec())
    }

    #[test]
    fn test_vprint_basic() {
        unsafe {
            let mut buf = [0xAAu8; 64];
            let (pos, out) = fmt(
                &mut buf,
                "hello %u world %s",
                &[PrintArg::Uint(1234), PrintArg::Str(b"str\0")],
            );
            assert_eq!(out, b"hello 1234 world str");
            assert_eq!(pos, 20);
        }
    }

    #[test]
    fn test_vprint_zu_and_percent() {
        unsafe {
            let mut buf = [0u8; 64];
            let (_, out) = fmt(
                &mut buf,
                "%zu%%%u",
                &[PrintArg::Size(usize::MAX), PrintArg::Uint(0)],
            );
            let expect = format!("{}%0", usize::MAX);
            assert_eq!(out, expect.as_bytes());
        }
    }

    #[test]
    fn test_vprint_widths() {
        unsafe {
            // "%*u" — right-pad to min width with spaces.
            let mut buf = [0u8; 64];
            let (_, out) = fmt(&mut buf, "%*u:", &[PrintArg::Int(5), PrintArg::Uint(42)]);
            assert_eq!(out, b"   42:");
            // "%.*s" — bound a non-NUL-terminated string.
            let mut buf = [0u8; 64];
            let (_, out) = fmt(
                &mut buf,
                "(%.*s)",
                &[PrintArg::Int(3), PrintArg::Str(b"abcdef\0")],
            );
            assert_eq!(out, b"(abc)");
        }
    }

    #[test]
    fn test_vprint_truncation() {
        unsafe {
            // C semantics: pos stops advancing at length; final NUL lands at
            // min(pos, length - 1).
            let mut buf = [0xAAu8; 8];
            let (pos, out) = fmt(&mut buf, "0123456789", &[]);
            assert_eq!(out, b"0123456"); // 7 chars + NUL
            assert_eq!(pos, 8); // pos saturates at length
        }
    }

    #[test]
    fn test_print_format_int() {
        unsafe {
            let mut buffer = [0u8; 96];
            let p = print_format_int(buffer.as_mut_ptr().add(96), 0);
            assert_eq!(*p, b'0');
            assert_eq!(*p.add(1), 0);
            let p = print_format_int(buffer.as_mut_ptr().add(96), u64::MAX);
            let s = core::slice::from_raw_parts(p, 20);
            assert_eq!(s, b"18446744073709551615");
        }
    }
}
