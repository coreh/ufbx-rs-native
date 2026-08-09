//! Port of the `// -- ASCII parsing` banner section of `ufbx.c`
//! (starting at ufbx.c:9404). This module covers the section in full: the
//! refill/yield/peek/next source machinery, the magic-comment
//! version scanner, the space character class, the token string buffers, the
//! `ufbxi_ascii_span` array capture, the tokenizer plus its
//! `try_ignore_string`/`accept` front end (ufbx.c:9404-9908), and the
//! value/array parsing layer — the inline int/float array readers, the
//! `ufbxi_ascii_array_task` span scanner, and the base64 decoder
//! (ufbx.c:9910-10282), and finally the node-level parser
//! `ufbxi_ascii_parse_node` (ufbx.c:10284-10695).
//!
//! CONTINUATION POINT: `// -- DOM retention` (ufbx.c:10696) — the next banner
//! section, owned by `native::parse`.
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
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]

use core::ffi::c_void;

use crate::generated::{Exporter, WarningType};
use crate::native::allocator::ZERO_SIZE_BUFFER;
use crate::native::buf::{
    pop_size, push, push_copy, push_fast, push_pop, push_pop_size, push_size, push_size_zero,
    push_zero, Buf,
};
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
    update_parse_state, ArrayInfo, Ascii, AsciiToken, Context, Node, ParseState, Value, ValueArray,
    ValueType, ARRAY_FLAG_ACCURATE_F32, ARRAY_FLAG_PAD_BEGIN, ARRAY_FLAG_RESULT,
    ARRAY_FLAG_TMP_BUF, MAX_NODE_DEPTH, MAX_NON_ARRAY_VALUES,
};
use crate::native::platform::{
    add_ptr, f64_to_i32, f64_to_i64, max_sz, min32, sub_ptr, to_size, ufbx_assert,
    ufbxi_dev_assert, MIN_THREADED_ASCII_VALUES,
};
use crate::native::string_pool::{push_sanitized_string, push_string, push_string_place_str};
use crate::native::thread::{thread_pool_create_task, thread_pool_run_task, Task};
use crate::native::warnings::ufbxi_warnf;
use crate::prelude::{Real, String};

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

// ufbx.c:9413-9455 `ufbxi_ascii_refill`
// `allow(unused_assignments)`: C's `char *dst_buffer = NULL; size_t dst_size = 0;`
// initializers are kept verbatim even though both branches overwrite them.
#[allow(unused_assignments)]
#[inline(never)]
pub(crate) unsafe fn ascii_refill(uc: &Context) -> u8 {
    let ua: *mut Ascii = uc.ascii_mut_ptr();
    uc.set_data_offset(
        (*uc.get())
            .data_offset
            .wrapping_add(to_size((*ua).src as isize - uc.data_begin() as isize) as u64),
    );
    if uc.read_fn().is_some() {
        let mut dst_buffer: *mut u8 = core::ptr::null_mut();
        let mut dst_size: usize = 0;

        if !(*ua).retain_buf.is_null() {
            dst_size = (*uc.get()).opts.read_buffer_size;
            dst_buffer = push::<u8>((*ua).retain_buf, dst_size);
            ufbxi_check_return!(uc, !dst_buffer.is_null(), b'\0', "dst_buffer");
            (*ua).src_is_retained = true;
            (*ua).src_buf = (*ua).retain_buf;
        } else {
            // Grow the read buffer if necessary
            if uc.read_buffer_size() < (*uc.get()).opts.read_buffer_size {
                let new_size: usize = (*uc.get()).opts.read_buffer_size;
                ufbxi_check_return!(
                    uc,
                    crate::native::allocator::grow_array::<u8>(
                        uc.ator_tmp_mut_ptr(),
                        uc.read_buffer_mut_ptr(),
                        uc.read_buffer_size_mut_ptr(),
                        new_size
                    ),
                    b'\0',
                    "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->read_buffer)), (&uc->read_buffer), (&uc->read_buffer_size), (new_size))"
                );
            }
            dst_buffer = uc.read_buffer();
            dst_size = uc.read_buffer_size();
            (*ua).src_is_retained = false;
            (*ua).src_buf = core::ptr::null_mut();
        }

        // Read user data, return '\0' on EOF
        // TODO: Very unoptimal for non-full-size reads in some cases
        let num_read: usize = (uc.read_fn().unwrap_unchecked())(
            uc.read_user(),
            dst_buffer as *mut core::ffi::c_void,
            dst_size,
        );
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

        (*ua).src = dst_buffer;
        uc.set_data_begin(dst_buffer);
        uc.set_data(dst_buffer);
        (*ua).src_end = dst_buffer.add(num_read);
        *(*ua).src
    } else {
        // If the user didn't specify a `read_fn()` treat anything
        // past the initial data buffer as EOF.
        (*ua).src = ASCII_EMPTY_STRING.as_ptr();
        uc.set_data_begin(ASCII_EMPTY_STRING.as_ptr());
        uc.set_data(ASCII_EMPTY_STRING.as_ptr());
        (*ua).src_end = (*ua).src.add(1);
        b'\0'
    }
}

// ufbx.c:9457-9478 `ufbxi_ascii_yield`
#[inline(never)]
pub(crate) unsafe fn ascii_yield(uc: &Context) -> u8 {
    let ua: *mut Ascii = uc.ascii_mut_ptr();

    let ret: u8;
    if (*ua).src == (*ua).src_end {
        ret = ascii_refill(uc);
    } else {
        ret = *(*ua).src;
    }

    if to_size((*ua).src_end as isize - (*ua).src as isize) < uc.progress_interval() {
        (*ua).src_yield = (*ua).src_end;
    } else {
        (*ua).src_yield = (*ua).src.add(uc.progress_interval());
    }

    // TODO: Unify these properly
    uc.set_data((*ua).src);
    ufbxi_check_return!(
        uc,
        report_progress(uc).is_ok(),
        b'\0',
        "ufbxi_report_progress(uc)"
    );
    ret
}

// ufbx.c:9480-9485 `ufbxi_ascii_peek`
#[inline(always)]
pub(crate) unsafe fn ascii_peek(uc: &Context) -> u8 {
    let ua: *mut Ascii = uc.ascii_mut_ptr();
    if (*ua).src == (*ua).src_yield {
        return ascii_yield(uc);
    }
    *(*ua).src
}

// ufbx.c:9487-9495 `ufbxi_ascii_next`
#[inline(always)]
pub(crate) unsafe fn ascii_next(uc: &Context) -> u8 {
    let ua: *mut Ascii = uc.ascii_mut_ptr();
    if (*ua).src == (*ua).src_yield {
        return ascii_yield(uc);
    }
    (*ua).src = (*ua).src.add(1);
    if (*ua).src == (*ua).src_yield {
        return ascii_yield(uc);
    }
    *(*ua).src
}

// ufbx.c:9497-9531 `ufbxi_ascii_parse_version`
#[inline(never)]
pub(crate) unsafe fn ascii_parse_version(uc: &Context) -> u32 {
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
                digits[num_digits as usize] = (c - b'0') as u8;
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

// ufbx.c:9533-9537 `ufbxi_space_mask`
pub(crate) const SPACE_MASK: u32 = (1u32 << (b' ' as u32 - 1))
    | (1u32 << (b'\t' as u32 - 1))
    | (1u32 << (b'\r' as u32 - 1))
    | (1u32 << (b'\n' as u32 - 1));

// ufbx.c:9539-9541 `ufbx_static_assert(space_codepoint, ...)`
const _: () = assert!(
    b' ' as u32 <= 32u32 && b'\t' as u32 <= 32u32 && b'\r' as u32 <= 32u32 && b'\n' as u32 <= 32u32
);

// ufbx.c:9543-9547 `ufbxi_is_space`
#[inline(always)]
pub(crate) fn is_space(c: u8) -> bool {
    let v: u32 = (c as u32).wrapping_sub(1);
    v < 32 && ((SPACE_MASK >> v) & 0x1) != 0
}

// ufbx.c:9549-9610 `ufbxi_ascii_skip_whitespace`
#[inline(never)]
pub(crate) unsafe fn ascii_skip_whitespace(uc: &Context) -> u8 {
    let ua: *mut Ascii = uc.ascii_mut_ptr();

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
            if !(*ua).read_first_comment {
                (*ua).read_first_comment = true;
                let version: u32 = ascii_parse_version(uc);
                if version != 0 {
                    uc.set_version(version);
                    (*ua).found_version = true;
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

                    if line_len >= 19
                        && memcmp(line.as_ptr(), b" Created by Blender".as_ptr(), 19) == 0
                    {
                        (*uc.get()).exporter = Exporter::BlenderAscii;
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
pub(crate) unsafe fn ascii_push_token_char(
    uc: &Context,
    token: *mut AsciiToken,
    c: u8,
) -> Result<(), Fail> {
    // Grow the string data buffer if necessary
    if (*token).str_len == (*token).str_cap {
        let len: usize = max_sz((*token).str_len + 1, 256);
        ufbxi_check!(
            uc,
            crate::native::allocator::grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                &raw mut (*token).str_data,
                &raw mut (*token).str_cap,
                len
            ),
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&token->str_data)), (&token->str_data), (&token->str_cap), (len))"
        );
    }

    *(*token).str_data.add((*token).str_len) = c;
    (*token).str_len += 1;

    Ok(())
}

// ufbx.c:9625-9637 `ufbxi_ascii_push_token_string`
#[inline(always)]
pub(crate) unsafe fn ascii_push_token_string(
    uc: &Context,
    token: *mut AsciiToken,
    data: *const u8,
    length: usize,
) -> Result<(), Fail> {
    // Grow the string data buffer if necessary
    if (*token).str_len + length >= (*token).str_cap {
        let len: usize = max_sz((*token).str_len + length, 256);
        ufbxi_check!(
            uc,
            crate::native::allocator::grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                &raw mut (*token).str_data,
                &raw mut (*token).str_cap,
                len
            ),
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&token->str_data)), (&token->str_data), (&token->str_cap), (len))"
        );
    }

    core::ptr::copy_nonoverlapping(data, (*token).str_data.add((*token).str_len), length);
    (*token).str_len += length;

    Ok(())
}

// ufbx.c:9639-9658 `ufbxi_ascii_skip_until`
#[inline(never)]
pub(crate) unsafe fn ascii_skip_until(uc: &Context, dst: u8) -> Result<(), Fail> {
    let ua: *mut Ascii = uc.ascii_mut_ptr();

    loop {
        let buffered: usize = to_size((*ua).src_yield as isize - (*ua).src as isize);
        let match_: *const u8 = memchr((*ua).src, dst, buffered);
        if !match_.is_null() {
            (*ua).src = match_;
            break;
        } else {
            (*ua).src = (*ua).src.add(buffered);
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
pub(crate) unsafe fn ascii_store_array(uc: &Context, tmp_buf: *mut Buf) -> Result<(), Fail> {
    let ua: *mut Ascii = uc.ascii_mut_ptr();

    (*ua).retain_buf = tmp_buf;

    loop {
        let buffered: usize = to_size((*ua).src_yield as isize - (*ua).src as isize);
        if buffered == 0 {
            let c: u8 = ascii_yield(uc);
            ufbxi_check!(uc, c != b'\0', "c != '\\0'");
            continue;
        }

        let begin: *const u8 = (*ua).src;
        let end: *const u8;
        let match_: *const u8 = memchr(begin, b'}', buffered);
        if !match_.is_null() {
            end = match_;
        } else {
            end = begin.add(buffered);
        }
        (*ua).src = end;

        let mut length: usize = to_size(end as isize - begin as isize);
        let span: *mut AsciiSpan = push::<AsciiSpan>(uc.tmp_ascii_spans_mut_ptr(), 1);
        ufbxi_check!(uc, !span.is_null(), "span");
        // Store the trailing '}' for parsing
        if !match_.is_null() {
            length += 1;
        }
        (*span).length = length;
        if (*ua).src_is_retained || uc.read_fn().is_none() {
            (*span).source = begin;
        } else {
            (*span).source = push_copy::<u8>(tmp_buf, length, begin);
            ufbxi_check!(uc, !(*span).source.is_null(), "span->source");
        }

        if !match_.is_null() {
            break;
        }
    }

    (*ua).retain_buf = core::ptr::null_mut();

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
pub(crate) unsafe fn ascii_try_ignore_string(uc: &Context, token: *mut AsciiToken) -> bool {
    let ua: *mut Ascii = uc.ascii_mut_ptr();

    let c: u8 = ascii_skip_whitespace(uc);
    (*token).str_len = 0;

    if c == b'"' {
        // Replace `prev_token` with `token` but swap the buffers so `token` uses
        // the now-unused string buffer of the old `prev_token`.
        let swap_data: *mut u8 = (*ua).prev_token.str_data;
        let swap_cap: usize = (*ua).prev_token.str_cap;
        (*ua).prev_token = (*ua).token;
        (*ua).token.str_data = swap_data;
        (*ua).token.str_cap = swap_cap;

        (*token).type_ = ASCII_STRING;
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
pub(crate) unsafe fn ascii_next_token(uc: &Context, token: *mut AsciiToken) -> Result<(), Fail> {
    let ua: *mut Ascii = uc.ascii_mut_ptr();

    // Replace `prev_token` with `token` but swap the buffers so `token` uses
    // the now-unused string buffer of the old `prev_token`.
    let swap_data: *mut u8 = (*ua).prev_token.str_data;
    let swap_cap: usize = (*ua).prev_token.str_cap;
    (*ua).prev_token = (*ua).token;
    (*ua).token.str_data = swap_data;
    (*ua).token.str_cap = swap_cap;

    let mut c: u8 = ascii_skip_whitespace(uc);
    (*token).str_len = 0;

    if (c >= b'A' && c <= b'Z') || (c >= b'a' && c <= b'z') || c == b'_' {
        (*token).type_ = ASCII_BARE_WORD;
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
            (*token).value.name_len = (*token).str_len;
            (*token).type_ = ASCII_NAME;
            ascii_next(uc);
        }
    } else if (c >= b'0' && c <= b'9') || c == b'-' || c == b'+' || c == b'.' {
        (*token).type_ = ASCII_INT;

        (*token).negative = c == b'-';
        while (c >= b'0' && c <= b'9')
            || c == b'-'
            || c == b'+'
            || c == b'.'
            || c == b'e'
            || c == b'E'
        {
            if c == b'.' || c == b'e' || c == b'E' {
                (*token).type_ = ASCII_FLOAT;
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
            (*token).type_ = ASCII_FLOAT;
        }

        let mut end: *const u8 = core::ptr::null();
        if (*token).type_ == ASCII_INT {
            (*token).value.i64_ = parse_int64((*token).str_data, &raw mut end);
            ufbxi_check!(
                uc,
                end == (*token).str_data.add((*token).str_len - 1),
                "end == token->str_data + token->str_len - 1"
            );
        } else if (*token).type_ == ASCII_FLOAT {
            let mut flags: u32 = uc.double_parse_flags();
            if (*ua).parse_as_f32 {
                flags = PARSE_DOUBLE_AS_BINARY32;
            }
            (*token).value.f64_ =
                parse_double((*token).str_data, (*token).str_len, &raw mut end, flags);
            ufbxi_check!(
                uc,
                end == (*token).str_data.add((*token).str_len - 1),
                "end == token->str_data + token->str_len - 1"
            );
        }
    } else if c == b'"' {
        (*token).type_ = ASCII_STRING;
        c = ascii_next(uc);
        while c != b'"' {
            // Optimized string parsing for non-special characters
            if (*ua).src.add(1) < (*ua).src_yield {
                let begin: *const u8 = (*ua).src;
                let mut end: *const u8 = (*ua).src_yield;
                let quot: *const u8 = memchr(begin, b'"', to_size(end as isize - begin as isize));
                if !quot.is_null() {
                    end = quot;
                }
                let esc: *const u8 = memchr(begin, b'&', to_size(end as isize - begin as isize));
                if !esc.is_null() {
                    end = esc;
                }

                if begin < end {
                    ascii_push_token_string(
                        uc,
                        token,
                        begin,
                        to_size(end as isize - begin as isize),
                    )?;
                    (*ua).src = end;
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

                ufbxi_dev_assert!(!entity.is_null() && *entity != 0);
                // `entity` is a NULL terminated string longer than a single character
                // cppcheck-suppress arrayIndexOutOfBounds
                while *entity.add(step) != 0 {
                    if c != *entity.add(step) {
                        break;
                    }
                    c = ascii_next(uc);
                    step += 1;
                }

                if *entity.add(step) == b'\0' {
                    // Full match: Push the replacement character
                    ascii_push_token_char(uc, token, replacement)?;
                } else {
                    // Partial match: Push the prefix we have skipped already
                    let mut i: usize = 0;
                    while i < step {
                        ascii_push_token_char(uc, token, *entity.add(i))?;
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
            (*token).value.name_len = (*token).str_len;
            (*token).type_ = ASCII_NAME;
            ascii_next(uc);
        }
    } else {
        // Single character token
        (*token).type_ = c;
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
pub(crate) unsafe fn ascii_accept(uc: &Context, type_: u8) -> bool {
    let ua: *mut Ascii = uc.ascii_mut_ptr();

    if (*ua).token.type_ == type_ {
        ufbxi_check_return!(
            uc,
            ascii_next_token(uc, &raw mut (*ua).token).is_ok(),
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
pub(crate) unsafe fn ascii_read_int_array(
    uc: &Context,
    type_: u8,
    p_num_read: *mut usize,
) -> Result<(), Fail> {
    let ua: *mut Ascii = uc.ascii_mut_ptr();
    if (*ua).parse_as_f32 {
        return Ok(());
    }
    let initial_items: usize = (*uc.get()).tmp_stack.num_items;

    let mut val: i64;
    if (*ua).token.type_ == ASCII_INT {
        val = (*ua).token.value.i64_;
    } else {
        return Ok(());
    }

    let mut src: *const u8 = (*ua).src;
    let end: *const u8 = (*ua).src_yield;
    let mut src_scan: *const u8 = src;

    loop {
        // Skip '\s*,\s*' between array elements. If we don't find a comma after an element
        // don't push it as we can't be 100% certain whether it's a part of the array.
        while src_scan != end && is_space(*src_scan) {
            src_scan = src_scan.add(1);
        }
        if src_scan == end || *src_scan != b',' {
            break;
        }
        src_scan = src_scan.add(1);
        while src_scan != end && is_space(*src_scan) {
            src_scan = src_scan.add(1);
        }

        // Found comma, commit to the position and push the previous value to the array
        src = src_scan;
        if type_ == b'i' {
            let v: *mut i32 = push_fast::<i32>(uc.tmp_stack_mut_ptr(), 1);
            ufbxi_check!(uc, !v.is_null(), "v");
            *v = val as i32;
        } else if type_ == b'l' {
            let v: *mut i64 = push_fast::<i64>(uc.tmp_stack_mut_ptr(), 1);
            ufbxi_check!(uc, !v.is_null(), "v");
            *v = val as i64;
        }

        // Try to parse the next value, we don't commit this until we find a comma after it above.
        let left: usize = to_size(end as isize - src_scan as isize);
        if left < 32 {
            break;
        }

        val = parse_int64(src_scan, &raw mut src_scan);
        if src_scan.is_null() {
            break;
        }
    }

    // Resume conventional parsing if we moved `src`.
    if src != (*ua).src {
        (*ua).src = src;
        ascii_next_token(uc, &raw mut (*ua).token)?;
    }

    *p_num_read = (*uc.get()).tmp_stack.num_items - initial_items;
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

// ufbx.c:9975-10008 `ufbxi_ascii_array_task_parse_floats`
#[inline(never)]
pub(crate) unsafe fn ascii_array_task_parse_floats(
    t: *mut AsciiArrayTask,
    src: *const u8,
    src_end: *const u8,
    parse_flags: u32,
) -> *const u8 {
    let mut src: *const u8 = src;
    let mut offset: usize = (*t).offset;
    let mut dst_float: *mut f32 = if (*t).arr_type == b'f' {
        add_ptr((*t).arr_data as *mut f32, offset)
    } else {
        core::ptr::null_mut()
    };
    let mut dst_double: *mut f64 = if (*t).arr_type == b'd' {
        add_ptr((*t).arr_data as *mut f64, offset)
    } else {
        core::ptr::null_mut()
    };
    ufbx_assert!(!dst_float.is_null() || !dst_double.is_null());
    let mut src_begin: *const u8 = src;

    while src != src_end {
        while is_space(*src) {
            src = src.add(1);
        }

        // Try to parse the next value, we don't commit this until we find a comma after it above.
        let mut num_end: *const u8 = core::ptr::null();
        let val: f64 = parse_double(
            src,
            to_size(src_end as isize - src as isize),
            &raw mut num_end,
            parse_flags,
        );
        if num_end.is_null() {
            return src_begin;
        }
        src = num_end;

        while is_space(*src) {
            src = src.add(1);
        }
        if *src != b',' {
            break;
        }
        src = src.add(1);
        src_begin = src;

        if offset >= (*t).arr_size {
            return core::ptr::null();
        }
        if !dst_double.is_null() {
            *dst_double = val;
            dst_double = dst_double.add(1);
        } else {
            *dst_float = val as f32;
            dst_float = dst_float.add(1);
        }
        offset += 1;
    }

    (*t).offset = offset;
    src_begin
}

// ufbx.c:10010-10040 `ufbxi_ascii_array_task_parse_ints`
#[inline(never)]
pub(crate) unsafe fn ascii_array_task_parse_ints(
    t: *mut AsciiArrayTask,
    src: *const u8,
    src_end: *const u8,
) -> *const u8 {
    let mut src: *const u8 = src;
    let mut offset: usize = (*t).offset;
    let mut dst32: *mut i32 = if (*t).arr_type == b'i' {
        add_ptr((*t).arr_data as *mut i32, offset)
    } else {
        core::ptr::null_mut()
    };
    let mut dst64: *mut i64 = if (*t).arr_type == b'l' {
        add_ptr((*t).arr_data as *mut i64, offset)
    } else {
        core::ptr::null_mut()
    };
    ufbx_assert!(!dst32.is_null() || !dst64.is_null());
    let mut src_begin: *const u8 = src;

    while src != src_end {
        while is_space(*src) {
            src = src.add(1);
        }

        let val: i64 = parse_int64(src, &raw mut src);
        if src.is_null() {
            return core::ptr::null();
        }

        while is_space(*src) {
            src = src.add(1);
        }
        if *src != b',' {
            break;
        }
        src = src.add(1);
        src_begin = src;

        if offset >= (*t).arr_size {
            return core::ptr::null();
        }
        if !dst32.is_null() {
            *dst32 = val as i32;
            dst32 = dst32.add(1);
        } else {
            *dst64 = val;
            dst64 = dst64.add(1);
        }
        offset += 1;
    }

    (*t).offset = offset;
    src_begin
}

// ufbx.c:10042-10050 `ufbxi_ascii_array_task_parse`
#[inline(never)]
pub(crate) unsafe fn ascii_array_task_parse(
    t: *mut AsciiArrayTask,
    src: *const u8,
    src_end: *const u8,
) -> *const u8 {
    if (*t).arr_type == b'f' || (*t).arr_type == b'd' {
        let flags: u32 = parse_double_init_flags();
        ascii_array_task_parse_floats(t, src, src_end, flags)
    } else {
        ascii_array_task_parse_ints(t, src, src_end)
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
#[inline(never)]
pub(crate) unsafe fn ascii_array_task_imp(t: *mut AsciiArrayTask) -> bool {
    // Temporary buffer for parsing between spans
    // C: uninitialized; only `buffer[0..buffer_len]` is ever read back (and the
    // parse helpers stop at the trailing ',' the state machine writes), so the
    // zero-fill here is inert.
    let mut buffer: [u8; 128] = [0; 128]; // ufbxi_uninit
    let mut buffer_len: usize = 0;
    let mut buffer_value: bool = false;

    let mut state: AsciiScanState = AsciiScanState::Whitespace;
    // C: `ufbxi_for(const ufbxi_ascii_span, span, t->spans, t->num_spans)`
    let mut span: *const AsciiSpan = (*t).spans;
    let span_end: *const AsciiSpan = add_ptr((*t).spans as *mut AsciiSpan, (*t).num_spans);
    while span != span_end {
        let mut src: *const u8 = (*span).source;
        let end: *const u8 = src.add((*span).length);

        while src != end {
            // State machine for skipping whitespace and comments, potentially
            // between multiple spans.
            while src != end {
                let c: u8 = *src;
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
                        src = src.add(1);
                        break;
                    } else {
                        buffer_value = true;
                        buffer[buffer_len] = c;
                        buffer_len += 1;
                        src = src.add(1);
                    }
                } else if state == AsciiScanState::Whitespace {
                    if c == b';' {
                        state = AsciiScanState::Comment;
                    } else if is_space(c) {
                        src = src.add(1);
                    } else {
                        state = AsciiScanState::Value;
                    }
                } else if state == AsciiScanState::Comment {
                    if c == b'\n' {
                        state = AsciiScanState::Whitespace;
                    } else {
                        src = src.add(1);
                    }
                } else if state == AsciiScanState::Comma {
                    state = AsciiScanState::Whitespace;
                }
            }

            if state == AsciiScanState::Comma {
                // Parse a value from the buffer
                if buffer_value {
                    let buffer_end: *const u8 =
                        ascii_array_task_parse(t, buffer.as_ptr(), buffer.as_ptr().add(buffer_len));
                    if buffer_end.is_null() || buffer_end == buffer.as_ptr() {
                        return false;
                    }
                }

                // If not at end, we are past the last comma, so try to find a
                // safe range to parse.
                if src != end {
                    let mut parse_end: *const u8 = end;
                    while parse_end > src {
                        if *parse_end.sub(1) == b',' {
                            break;
                        }
                        parse_end = parse_end.sub(1);
                    }
                    if src < parse_end {
                        src = ascii_array_task_parse(t, src, parse_end);
                        if src.is_null() {
                            return false;
                        }
                    }
                }

                buffer_len = 0;
                buffer_value = false;
            }
        }

        span = span.add(1);
    }

    if (*t).offset != (*t).arr_size {
        return false;
    }

    true
}

// ufbx.c:10150-10158 `ufbxi_ascii_array_task_fn`
// `ufbxi_task_fn` entry point handed to the thread pool by
// `ufbxi_ascii_parse_node` (ufbx.c:10653-10659).
#[inline(never)]
pub(crate) unsafe extern "C" fn ascii_array_task_fn(task: *mut Task) -> bool {
    let t: *mut AsciiArrayTask = (*task).data as *mut AsciiArrayTask;
    if !ascii_array_task_imp(t) {
        (*task).error = b"Threaded ASCII parse error\0".as_ptr();
        return false;
    }
    true
}

// ufbx.c:10160-10222 `ufbxi_ascii_read_float_array`
#[inline(never)]
pub(crate) unsafe fn ascii_read_float_array(
    uc: &Context,
    type_: u8,
    p_num_read: *mut usize,
) -> Result<(), Fail> {
    let ua: *mut Ascii = uc.ascii_mut_ptr();
    if (*ua).parse_as_f32 {
        return Ok(());
    }

    let mut val: f64;
    if (*ua).token.type_ == ASCII_FLOAT {
        val = (*ua).token.value.f64_;
    } else if (*ua).token.type_ == ASCII_INT {
        let fsign: f64 = if (*ua).token.value.i64_ == 0 && (*ua).token.negative {
            -1.0
        } else {
            1.0
        };
        val = (*ua).token.value.i64_ as f64 * fsign;
    } else {
        return Ok(());
    }

    let mut src: *const u8 = (*ua).src;
    let end: *const u8 = (*ua).src_yield;

    let parse_flags: u32 = uc.double_parse_flags();

    let initial_items: usize = (*uc.get()).tmp_stack.num_items;
    let mut src_scan: *const u8 = src;
    loop {
        // Skip '\s*,\s*' between array elements. If we don't find a comma after an element
        // don't push it as we can't be 100% certain whether it's a part of the array.
        while src_scan != end && is_space(*src_scan) {
            src_scan = src_scan.add(1);
        }
        if src_scan == end || *src_scan != b',' {
            break;
        }
        src_scan = src_scan.add(1);
        while src_scan != end && is_space(*src_scan) {
            src_scan = src_scan.add(1);
        }

        // Found comma, commit to the position and push the previous value to the array
        src = src_scan;
        if type_ == b'd' {
            let v: *mut f64 = push_fast::<f64>(uc.tmp_stack_mut_ptr(), 1);
            ufbxi_check!(uc, !v.is_null(), "v");
            *v = val as f64;
        } else if type_ == b'f' {
            let v: *mut f32 = push_fast::<f32>(uc.tmp_stack_mut_ptr(), 1);
            ufbxi_check!(uc, !v.is_null(), "v");
            *v = val as f32;
        }

        // Try to parse the next value, we don't commit this until we find a comma after it above.
        let mut num_end: *const u8 = core::ptr::null();
        let left: usize = to_size(end as isize - src_scan as isize);
        val = parse_double(src_scan, left, &raw mut num_end, parse_flags);
        if num_end.is_null() || num_end == src_scan || num_end >= end {
            break;
        }

        src_scan = num_end;
    }

    // Resume conventional parsing if we moved `src`.
    if src != (*ua).src {
        (*ua).src = src;
        ascii_next_token(uc, &raw mut (*ua).token)?;
    }

    *p_num_read = (*uc.get()).tmp_stack.num_items - initial_items;
    Ok(())
}

// ufbx.c:10224-10239 `ufbxi_setup_base64`
// C `ufbxi_nounroll` is an optimizer pragma with no Rust analogue; the loops
// port as plain `while`s.
#[inline(never)]
pub(crate) unsafe fn setup_base64(uc: &Context) -> Result<(), Fail> {
    let table: *mut u8 = push::<u8>(uc.tmp_mut_ptr(), 256);
    ufbxi_check!(uc, !table.is_null(), "table");
    uc.set_base64_table(table);

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

    Ok(())
}

// ufbx.c:10241-10282 `ufbxi_decode_base64`
#[inline(never)]
pub(crate) unsafe fn decode_base64(
    uc: &Context,
    p_result: *mut String,
    src: *const u8,
    src_length: usize,
    p_failed: *mut bool,
) -> Result<(), Fail> {
    if uc.base64_table().is_null() {
        setup_base64(uc)?;
    }

    let table: *mut u8 = uc.base64_table();
    let mut error_mask: u32 = 0;
    let mut pad_error: u32 = 0;

    let mut p: *mut u8 = (*p_result).data as *mut u8;
    let mut i: usize = 0;
    while i + 4 <= src_length {
        let a: u32 = *table.add(*src.add(i + 0) as usize) as u32;
        let b: u32 = *table.add(*src.add(i + 1) as usize) as u32;
        let c: u32 = *table.add(*src.add(i + 2) as usize) as u32;
        let d: u32 = *table.add(*src.add(i + 3) as usize) as u32;
        pad_error = error_mask;
        error_mask |= a | b | c | d;

        *p.add(0) = (a << 2 | b >> 4) as u8;
        *p.add(1) = (b << 4 | c >> 2) as u8;
        *p.add(2) = (c << 6 | d) as u8;
        p = p.add(3);

        i += 4;
    }

    if src_length >= 4 {
        let end: *const u8 = src.add(src_length - 4);
        let mut padding: u32 = 0;
        padding |= if *end.add(0) == b'=' { 0x8 } else { 0x0 };
        padding |= if *end.add(1) == b'=' { 0x4 } else { 0x0 };
        padding |= if *end.add(2) == b'=' { 0x2 } else { 0x0 };
        padding |= if *end.add(3) == b'=' { 0x1 } else { 0x0 };
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

    (*p_result).length = to_size(p as isize - (*p_result).data as isize);
    Ok(())
}

// Recursion limited by check at the start
// ufbx.c:10284-10695 `ufbxi_ascii_parse_node`
// `ufbxi_recursive_function(int, ufbxi_ascii_parse_node, ..., UFBXI_MAX_NODE_DEPTH + 1, ...)`
// (ufbx.c:10286-10287): under regression, a thread-local depth guard wraps the
// recursive body (which C splits into `ufbxi_ascii_parse_node_rec`); otherwise
// the macro is empty and the wrapper is a plain call.
#[inline(never)]
pub(crate) unsafe fn ascii_parse_node(
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
unsafe fn ascii_parse_node_rec(
    uc: &Context,
    depth: u32,
    parent_state: ParseState,
    p_end: *mut bool,
    tmp_buf: *mut Buf,
    recursive: bool,
) -> Result<(), Fail> {
    let ua: *mut Ascii = uc.ascii_mut_ptr();

    if (*ua).token.type_ == b'}' {
        ascii_next_token(uc, &raw mut (*ua).token)?;
        *p_end = true;
        return Ok(());
    }

    if (*ua).token.type_ == ASCII_END {
        ufbxi_check_msg!(uc, depth == 0, "Truncated file");
        *p_end = true;
        return Ok(());
    }

    // Parse the name eg. "Node:" token and intern the name
    ufbxi_check!(uc, depth < MAX_NODE_DEPTH, "depth < UFBXI_MAX_NODE_DEPTH");
    if !uc.sure_fbx() && depth == 0 && (*ua).token.type_ != ASCII_NAME {
        ufbxi_fail_msg!(uc, "Expected a 'Name:' token", "Not an FBX file");
    }
    ufbxi_check!(
        uc,
        ascii_accept(uc, ASCII_NAME),
        "ufbxi_ascii_accept(uc, UFBXI_ASCII_NAME)"
    );
    let name_len: usize = (*ua).prev_token.value.name_len;
    ufbxi_check!(uc, name_len <= 0xff, "name_len <= 0xff");
    let name: *const u8 = push_string(
        uc.string_pool_mut_ptr(),
        (*ua).prev_token.str_data,
        (*ua).prev_token.str_len,
        core::ptr::null_mut(),
        true,
    );
    ufbxi_check!(uc, !name.is_null(), "name");

    // Push the parsed node into the `tmp_stack` buffer, the nodes will be popped by
    // calling code after its done parsing all of it's children.
    let node: *mut Node = push_zero::<Node>(uc.tmp_stack_mut_ptr(), 1);
    ufbxi_check!(uc, !node.is_null(), "node");
    (*node).name = name;
    (*node).name_len = name_len as u8;

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
    let arr_info: *mut ArrayInfo = arr_info.as_mut_ptr();
    if is_array_node(uc, parent_state, name, arr_info) {
        let flags: u8 = (*arr_info).flags;
        arr_type = normalize_array_type((*arr_info).type_, b'b');
        arr_buf = tmp_buf;
        if (flags & ARRAY_FLAG_RESULT) != 0 {
            arr_buf = uc.result_mut_ptr();
        } else if (flags & ARRAY_FLAG_TMP_BUF) != 0 {
            arr_buf = uc.tmp_mut_ptr();
        }

        let arr: *mut ValueArray = push::<ValueArray>(tmp_buf, 1);
        ufbxi_check!(uc, !arr.is_null(), "arr");
        (*node).value_type_mask = ValueType::Array as u16;
        (*node).content.array = arr;
        (*arr).type_ = arr_type;

        // Parse array values using strtof() if the array destination is 32-bit float
        // since KeyAttrDataFloat packs integer data (!) into floating point values so we
        // should try to be as exact as possible.
        if ((*arr_info).flags & ARRAY_FLAG_ACCURATE_F32) != 0 {
            (*ua).parse_as_f32 = true;
        }

        arr_elem_size = array_type_size(arr_type);

        if arr_type != b'-' {
            // Force alignment for array contents: This allows us to use `ufbxi_push_fast()`
            // in fast parsing functions.
            ufbxi_check!(
                uc,
                !push_size_zero(uc.tmp_stack_mut_ptr(), 8, 1).is_null(),
                "ufbxi_push_size_zero(&uc->tmp_stack, 8, 1)"
            );

            // Pad with 4 zero elements to make indexing with `-1` safe.
            if (flags & ARRAY_FLAG_PAD_BEGIN) != 0 {
                ufbxi_check!(
                    uc,
                    !push_size_zero(uc.tmp_stack_mut_ptr(), arr_elem_size, 4).is_null(),
                    "ufbxi_push_size_zero(&uc->tmp_stack, arr_elem_size, 4)"
                );
                num_values += 4;
            }
        }
    }

    // Some fields in ASCII may have leading commas eg. `Content: , "base64-string"`
    if (*ua).token.type_ == b',' {
        // HACK: If we are parsing an "array" that should be ignored, ie. `Content` when
        // `opts.ignore_embedded == true` try to skip the next token string if possible.
        if arr_type == b'-' {
            if !ascii_try_ignore_string(uc, &raw mut (*ua).token) {
                ascii_next_token(uc, &raw mut (*ua).token)?;
            }
        } else {
            ascii_next_token(uc, &raw mut (*ua).token)?;
        }
    }

    let parse_state: ParseState = update_parse_state(parent_state, (*node).name);
    // C: `ufbxi_value vals[UFBXI_MAX_NON_ARRAY_VALUES];` — left uninitialized;
    // only the `[0, num_values)` prefix written in the loop below is ever read
    // (by the `ufbxi_push_copy` at the end).
    let mut vals = core::mem::MaybeUninit::<[Value; MAX_NON_ARRAY_VALUES]>::uninit();
    let vals: *mut Value = vals.as_mut_ptr() as *mut Value;

    let mut deferred_size: u32 = 0;

    // NOTE: Infinite loop to allow skipping the comma parsing via `continue`.
    loop {
        let tok: *mut AsciiToken = &raw mut (*ua).prev_token;

        if arr_type != 0 {
            let mut num_read: usize = 0;
            if arr_type == b'f' || arr_type == b'd' {
                ascii_read_float_array(uc, arr_type, &raw mut num_read)?;
            } else if arr_type == b'i' || arr_type == b'l' {
                ascii_read_int_array(uc, arr_type, &raw mut num_read)?;
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
                    let v: *mut String = push::<String>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    if arr_type == b'C' {
                        let buf: *mut Buf = if (*uc.get()).opts.retain_dom {
                            uc.result_mut_ptr()
                        } else {
                            tmp_buf
                        };
                        let capacity: usize = (*tok).str_len / 4 * 3 + 3;
                        (*v).data = push::<u8>(buf, capacity);
                        ufbxi_check!(uc, !(*v).data.is_null(), "v->data");
                        decode_base64(uc, v, (*tok).str_data, (*tok).str_len, &raw mut arr_error)?;
                        ufbx_assert!((*v).length <= capacity);
                    } else {
                        (*v).data = (*tok).str_data;
                        (*v).length = (*tok).str_len;
                        push_string_place_str(uc.string_pool_mut_ptr(), v, raw)?;
                    }
                } else {
                    // Ignore strings in non-string arrays, decrement `num_values` as it will be
                    // incremented after the loop iteration is done to ignore it.
                    num_values = num_values.wrapping_sub(1);
                }
            } else if (num_values as usize) < MAX_NON_ARRAY_VALUES {
                type_mask |= (ValueType::String as u32) << (num_values * 2);
                let v: *mut Value = vals.add(num_values as usize);

                let str_: *const u8 = (*tok).str_data;
                let length: usize = (*tok).str_len;
                ufbxi_check!(uc, !str_.is_null(), "str");

                if length == 0 {
                    (*v).s.raw_data = EMPTY_CHAR.as_ptr();
                    (*v).s.raw_length = 0;
                    (*v).s.utf8_length = 0;
                } else {
                    let mut non_ascii: bool = false;
                    let hash: u32 = hash_string_check_ascii(str_, length, &raw mut non_ascii);
                    let raw: bool =
                        !non_ascii || is_raw_string(uc, parent_state, name, num_values as usize);
                    push_sanitized_string(
                        uc.string_pool_mut_ptr(),
                        &raw mut (*v).s,
                        str_,
                        length,
                        hash,
                        raw,
                    )?;
                    if non_ascii && raw {
                        (*v).s.utf8_length = u32::MAX;
                    }
                }
            }
        } else if ascii_accept(uc, ASCII_INT) {
            let val: i64 = (*tok).value.i64_;
            let fsign: Real = if val == 0 && (*tok).negative {
                -1.0f32 as Real
            } else {
                1.0f32 as Real
            };

            match arr_type {
                0 => {
                    // Parse version from comment if there was no magic comment
                    if !(*ua).found_version
                        && parse_state == ParseState::FbxVersion
                        && num_values == 0
                    {
                        if val >= 6000 && val <= 10000 {
                            (*ua).found_version = true;
                            uc.set_version(val as u32);
                        }
                    }

                    if (num_values as usize) < MAX_NON_ARRAY_VALUES {
                        type_mask |= (ValueType::Number as u32) << (num_values * 2);
                        let v: *mut Value = vals.add(num_values as usize);
                        // False positive: `v->f` and `v->i` do not overlap in the union.
                        // cppcheck-suppress overlappingWriteUnion
                        // C: `v->f = (double)(v->i = val) * (double)fsign;`
                        (*v).num.i = val;
                        (*v).num.f = (*v).num.i as f64 * fsign as f64;
                    }
                }

                b'b' => {
                    let v: *mut bool = push::<bool>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val != 0;
                }
                b'c' => {
                    let v: *mut u8 = push::<u8>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val as u8;
                }
                b'i' => {
                    let v: *mut i32 = push::<i32>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val as i32;
                }
                b'l' => {
                    let v: *mut i64 = push::<i64>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val;
                }
                b'f' => {
                    let v: *mut f32 = push::<f32>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val as f32 * fsign as f32;
                }
                b'd' => {
                    let v: *mut f64 = push::<f64>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val as f64 * fsign as f64;
                }
                b'-' => {
                    num_values = num_values.wrapping_sub(1);
                }

                _ => ufbxi_fail!(uc, "Bad array dst type"),
            }
        } else if ascii_accept(uc, ASCII_FLOAT) {
            let val: f64 = (*tok).value.f64_;

            match arr_type {
                0 => {
                    if (num_values as usize) < MAX_NON_ARRAY_VALUES {
                        type_mask |= (ValueType::Number as u32) << (num_values * 2);
                        let v: *mut Value = vals.add(num_values as usize);
                        // False positive: `v->f` and `v->i` do not overlap in the union.
                        // cppcheck-suppress overlappingWriteUnion
                        // C: `v->i = ufbxi_f64_to_i64(v->f = val);`
                        (*v).num.f = val;
                        (*v).num.i = f64_to_i64((*v).num.f);
                    }
                }

                b'b' => {
                    let v: *mut bool = push::<bool>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val != 0.0;
                }
                b'c' => {
                    let v: *mut u8 = push::<u8>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    // C-parity: C's `(uint8_t)val` on a `double` is UB out of range; the
                    // x86-64 oracle emits a 32-bit `cvttsd2si` + low-byte narrow. Plain
                    // `as` (saturating) per the PORTING.md bare-float-cast row — known,
                    // accepted divergence, same choice as the `ufbxi_cast_u8` appliers in
                    // parse_binary.rs.
                    *v = val as u8;
                }
                b'i' => {
                    let v: *mut i32 = push::<i32>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = f64_to_i32(val);
                }
                b'l' => {
                    let v: *mut i64 = push::<i64>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = f64_to_i64(val);
                }
                b'f' => {
                    let v: *mut f32 = push::<f32>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val as f32;
                }
                b'd' => {
                    let v: *mut f64 = push::<f64>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val;
                }
                b'-' => {
                    num_values = num_values.wrapping_sub(1);
                }

                _ => ufbxi_fail!(uc, "Bad array dst type"),
            }
        } else if ascii_accept(uc, ASCII_BARE_WORD) {
            let mut val: i64 = 0;
            let mut val_f: f64 = 0.0;
            if (*tok).str_len >= 1 {
                // C-parity: `tok->str_data` is `char *` and the oracle targets make
                // C `char` signed, so a byte >= 0x80 converts to a NEGATIVE `int64_t`
                // here (PORTING.md "`char` (value, where signedness is observable)").
                val = *((*tok).str_data as *const i8) as i64;
                val_f = val as f64;
                if (*tok).str_len > 1 && (*tok).str_len < 64 {
                    // Try to parse the bare word as NAN/INF
                    let mut str_data = core::mem::MaybeUninit::<[u8; 64]>::uninit(); // ufbxi_uninit
                    let str_data: *mut u8 = str_data.as_mut_ptr() as *mut u8;
                    let str_len: usize = (*tok).str_len;
                    core::ptr::copy_nonoverlapping((*tok).str_data, str_data, str_len);
                    *str_data.add(str_len) = b'\0';
                    let mut inf_nan: f64 = 0.0;
                    let mut end: *const u8 = core::ptr::null();
                    if parse_inf_nan(&raw mut inf_nan, str_data, str_len, &raw mut end)
                        && end == str_data.add(str_len)
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
                        let v: *mut Value = vals.add(num_values as usize);
                        // False positive: `v->f` and `v->i` do not overlap in the union.
                        // cppcheck-suppress overlappingWriteUnion
                        (*v).num.i = val;
                        (*v).num.f = val_f;
                    }
                }

                b'b' => {
                    let v: *mut bool = push::<bool>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val != 0;
                }
                b'c' => {
                    let v: *mut u8 = push::<u8>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val as u8;
                }
                b'i' => {
                    let v: *mut i32 = push::<i32>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val as i32;
                }
                b'l' => {
                    let v: *mut i64 = push::<i64>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val;
                }
                b'f' => {
                    let v: *mut f32 = push::<f32>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val_f as f32;
                }
                b'd' => {
                    let v: *mut f64 = push::<f64>(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !v.is_null(), "v");
                    *v = val_f;
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
            let count: i64 = (*ua).prev_token.value.i64_;

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
                    && !(*uc.get()).opts.force_single_thread_ascii_parsing
                    && !(*ua).parse_as_f32
                    && (arr_type == b'i'
                        || arr_type == b'l'
                        || arr_type == b'f'
                        || arr_type == b'd')
                {
                    // Don't bother with small arrays due to fixed overhead
                    if count >= MIN_THREADED_ASCII_VALUES as i64 && count <= u32::MAX as i64 {
                        deferred_size = (count as u32).wrapping_sub(1);
                        ascii_store_array(uc, tmp_buf)?;
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

    (*ua).parse_as_f32 = false;

    if arr_type != 0 {
        if arr_type == b'-' {
            (*(*node).content.array).data = core::ptr::null_mut();
            (*(*node).content.array).size = 0;
        } else {
            let mut arr_data: *mut c_void = core::ptr::null_mut();

            if deferred_size > 0 {
                arr_data = push_size(
                    arr_buf,
                    arr_elem_size,
                    num_values.wrapping_add(deferred_size) as usize,
                );
                // Pop any previously pushed values
                if num_values > 0 {
                    pop_size(
                        uc.tmp_stack_mut_ptr(),
                        arr_elem_size,
                        num_values as usize,
                        arr_data,
                        false,
                    );
                }
            } else if arr_error {
                pop_size(
                    uc.tmp_stack_mut_ptr(),
                    arr_elem_size,
                    num_values as usize,
                    core::ptr::null_mut(),
                    false,
                );
                num_values = 0;
                arr_data = ZERO_SIZE_BUFFER.as_ptr() as *mut c_void;
            } else {
                arr_data = push_pop_size(
                    arr_buf,
                    uc.tmp_stack_mut_ptr(),
                    arr_elem_size,
                    num_values as usize,
                );
            }
            ufbxi_check!(uc, !arr_data.is_null(), "arr_data");
            if ((*arr_info).flags & ARRAY_FLAG_PAD_BEGIN) != 0 {
                (*(*node).content.array).data =
                    (arr_data as *mut u8).add(4usize.wrapping_mul(arr_elem_size)) as *mut c_void;
                (*(*node).content.array).size =
                    num_values.wrapping_add(deferred_size).wrapping_sub(4) as usize;
            } else {
                (*(*node).content.array).data = arr_data;
                (*(*node).content.array).size = num_values.wrapping_add(deferred_size) as usize;
            }

            // Pop alignment helper
            pop_size(uc.tmp_stack_mut_ptr(), 8, 1, core::ptr::null_mut(), false);

            // Deferred parsing
            if deferred_size > 0 {
                let num_spans: usize = (*uc.get()).tmp_ascii_spans.num_items;
                let spans: *mut AsciiSpan =
                    push_pop::<AsciiSpan>(tmp_buf, uc.tmp_ascii_spans_mut_ptr(), num_spans);
                ufbxi_check!(uc, !spans.is_null(), "spans");

                let mut t = core::mem::MaybeUninit::<AsciiArrayTask>::uninit(); // ufbxi_uninit
                let t: *mut AsciiArrayTask = t.as_mut_ptr();
                (*t).arr_data = (arr_data as *mut u8)
                    .add((num_values as usize).wrapping_mul(arr_elem_size))
                    as *mut c_void;
                (*t).arr_type = arr_type;
                (*t).arr_size = deferred_size as usize;
                (*t).num_spans = num_spans;
                (*t).spans = spans;
                (*t).offset = 0;

                // TODO: Split these further
                let task: *mut Task =
                    thread_pool_create_task(uc.thread_pool_mut_ptr(), ascii_array_task_fn);
                if !task.is_null() {
                    (*task).data = push_copy::<AsciiArrayTask>(tmp_buf, 1, t) as *mut c_void;
                    ufbxi_check!(uc, !(*task).data.is_null(), "task->data");
                    thread_pool_run_task(uc.thread_pool_mut_ptr(), task);
                } else {
                    ufbxi_check_msg!(
                        uc,
                        ascii_array_task_imp(t),
                        "Threaded ASCII parse error",
                        "ufbxi_ascii_array_task_imp(&t)"
                    );
                }
            }
        }
    } else {
        num_values = min32(num_values, MAX_NON_ARRAY_VALUES as u32);
        (*node).value_type_mask = type_mask as u16;
        (*node).content.vals = push_copy::<Value>(tmp_buf, num_values as usize, vals);
        ufbxi_check!(uc, !(*node).content.vals.is_null(), "node->vals");
    }

    // Recursively parse the children of this node. Update the parse state
    // to provide context for child node parsing.
    if ascii_accept(uc, b'{') {
        if recursive {
            let mut num_children: usize = 0;
            loop {
                let mut end: bool = false;
                ascii_parse_node(uc, depth + 1, parse_state, &raw mut end, tmp_buf, recursive)?;
                if end {
                    break;
                }
                num_children += 1;
            }

            // Pop children from `tmp_stack` to a contiguous array
            (*node).children = push_pop::<Node>(tmp_buf, uc.tmp_stack_mut_ptr(), num_children);
            ufbxi_check!(uc, !(*node).children.is_null(), "node->children");
            (*node).num_children = num_children as u32;
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

    // `ufbxi_is_space` (ufbx.c:9543-9547) is context-free: exactly
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
