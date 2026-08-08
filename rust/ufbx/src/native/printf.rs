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
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]

use crate::native::platform::{ufbxi_dev_assert, ufbxi_unreachable};

// C `va_list` replacement (PORTING.md "Printf and variadics"): one variant per
// `va_arg` type the restricted format subset can pull. Slices of these are the
// ONE sanctioned slice use in ported paths.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PrintArg {
    /// `va_arg(args, int)` — `*` / `.*` width and precision arguments.
    Int(i32),
    /// `va_arg(args, uint32_t)` — `%u`.
    Uint(u32),
    /// `va_arg(args, size_t)` — `%zu`.
    Size(usize),
    /// `va_arg(args, const char*)` — `%s` (NUL-terminated unless bounded by `%.*s`).
    Str(*const u8),
}

impl From<i32> for PrintArg {
    fn from(v: i32) -> PrintArg {
        PrintArg::Int(v)
    }
}
impl From<u32> for PrintArg {
    fn from(v: u32) -> PrintArg {
        PrintArg::Uint(v)
    }
}
impl From<usize> for PrintArg {
    fn from(v: usize) -> PrintArg {
        PrintArg::Size(v)
    }
}
impl From<*const u8> for PrintArg {
    fn from(v: *const u8) -> PrintArg {
        PrintArg::Str(v)
    }
}
impl From<*mut u8> for PrintArg {
    fn from(v: *mut u8) -> PrintArg {
        PrintArg::Str(v)
    }
}
impl From<&'static str> for PrintArg {
    // Convenience for NUL-terminated static literals at call sites
    // (C passes string literals directly as `const char *`). The literal MUST
    // carry an explicit trailing NUL (`"...\0"`) — C literals get it
    // implicitly, Rust ones do not, and `print_append` scans to NUL when the
    // width is unbounded (`%s` without `.*`).
    fn from(v: &'static str) -> PrintArg {
        debug_assert!(
            v.ends_with('\0'),
            "PrintArg %s literals must be \\0-terminated"
        );
        PrintArg::Str(v.as_ptr())
    }
}

impl PrintArg {
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
    fn as_str(self) -> *const u8 {
        match self {
            PrintArg::Str(v) => v,
            _ => {
                ufbxi_unreachable!("Bad printf format");
                b"\0".as_ptr()
            }
        }
    }
}

// `va_arg`: pull the next argument. Exhausting the argument list is UB in C;
// defined-but-asserting here.
fn va_arg(args: &[PrintArg], ix: &mut usize) -> PrintArg {
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
    str: *const u8,
) {
    let mut width: usize = 0;
    while width < max_width {
        if *str.add(width) == 0 {
            break;
        }
        width += 1;
    }
    let pad = if min_width > width {
        min_width - width
    } else {
        0
    };
    for _i in 0..pad {
        if (*buf).pos < (*buf).length {
            *(*buf).dst.add((*buf).pos) = b' ';
            (*buf).pos += 1;
        }
    }
    for i in 0..width {
        if (*buf).pos < (*buf).length {
            *(*buf).dst.add((*buf).pos) = *str.add(i);
            (*buf).pos += 1;
        }
    }
}

// ufbx.c:3307-3316 `ufbxi_print_format_int`
// C formats backwards from one-past-the-end: `*--buffer = ...`.
pub(crate) unsafe fn print_format_int(mut buffer: *mut u8, mut value: u64) -> *mut u8 {
    buffer = buffer.sub(1);
    *buffer = b'\0';
    loop {
        let digit = (value % 10) as u32;
        value = value / 10;
        buffer = buffer.sub(1);
        *buffer = b'0' + digit as u8;
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
    while *p != 0 {
        // C: `if (*p == '%' && *++p != '%')` — the increment happens only when
        // `*p == '%'`; on `%%` the else-branch then emits the second '%'.
        if *p == b'%' && {
            p = p.add(1);
            *p != b'%'
        } {
            let mut min_width: usize = 0;
            let mut max_width: usize = usize::MAX;
            if *p == b'*' {
                p = p.add(1);
                min_width = va_arg(args, &mut arg_ix).as_int() as usize;
            }
            if *p == b'.' {
                ufbxi_dev_assert!(*p.add(1) == b'*');
                p = p.add(2);
                max_width = va_arg(args, &mut arg_ix).as_int() as usize;
            }
            let mut flags: u32 = 0;
            match *p {
                b'z' => {
                    p = p.add(1);
                    flags |= PRINT_SIZE_T;
                }
                _ => {}
            }
            let spec = *p;
            p = p.add(1);
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
                print_append(buf, min_width, max_width, str);
            } else if (flags & PRINT_UNSIGNED) != 0 {
                let value: u64 = if (flags & PRINT_SIZE_T) != 0 {
                    va_arg(args, &mut arg_ix).as_size() as u64
                } else {
                    va_arg(args, &mut arg_ix).as_u32() as u64
                };
                let str = print_format_int(buffer.as_mut_ptr().add(buffer.len()), value);
                print_append(buf, min_width, max_width, str);
            } else {
                ufbxi_unreachable!("Bad printf format");
            }
        } else {
            if (*buf).pos < (*buf).length {
                *(*buf).dst.add((*buf).pos) = *p;
                (*buf).pos += 1;
            }
            p = p.add(1);
        }
    }
    if (*buf).length != 0 && !(*buf).dst.is_null() {
        let end = if (*buf).pos <= (*buf).length - 1 {
            (*buf).pos
        } else {
            (*buf).length - 1
        };
        *(*buf).dst.add(end) = b'\0';
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
        vprint(&mut pb, fmt_nul.as_ptr(), args);
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
                &[PrintArg::Uint(1234), PrintArg::Str(b"str\0".as_ptr())],
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
                &[PrintArg::Int(3), PrintArg::Str(b"abcdef\0".as_ptr())],
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
