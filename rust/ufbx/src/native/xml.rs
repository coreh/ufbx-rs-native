//! Port of the `// -- XML` banner section (ufbx.c:7245-7682).
//!
//! C gates the whole section on `UFBXI_FEATURE_XML`, which is DERIVED from
//! `UFBXI_FEATURE_GEOMETRY_CACHE` (ufbx.c:177-185) and is not independently
//! selectable — the Rust mapping is `#[cfg(feature = "geometry-cache")]` on
//! the whole module (PORTING.md "Macros & feature gates"). The parser has no
//! public entry point of its own: it exists only to feed
//! `ufbxi_cache_load_xml` in `native::cache`, so there is nothing to keep
//! callable when the feature is off.
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
#![cfg(feature = "geometry-cache")]

use core::ffi::c_void;
use core::mem::size_of_val;

use crate::generated::Error;
use crate::native::allocator::{free, grow_array, Allocator};
use crate::native::buf::{buf_free, push, push_copy, push_pop, push_zero, Buf};
use crate::native::error::{
    strcmp, ufbxi_check_err, ufbxi_check_err_msg, ufbxi_fail_err, Fail, EMPTY_CHAR,
};
use crate::native::platform::{add_ptr, ufbx_assert, IS_REGRESSION};
use crate::prelude::String;

// ufbx.c:53 `#define UFBXI_MAX_XML_DEPTH 32` — owned here; the XML tag parser
// is its only user.
pub(crate) const MAX_XML_DEPTH: usize = 32;

// ufbx.c:7253-7256 `ufbxi_xml_attrib`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct XmlAttrib {
    pub name: String,
    pub value: String,
}

// ufbx.c:7258-7267 `ufbxi_xml_tag`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct XmlTag {
    pub name: String,
    pub text: String,

    pub attribs: *mut XmlAttrib,
    pub num_attribs: usize,

    pub children: *mut XmlTag,
    pub num_children: usize,
}

// ufbx.c:7269-7272 `ufbxi_xml_document`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct XmlDocument {
    pub root: *mut XmlTag,
    pub buf: Buf,
}

// ufbx.c:7274-7295 `ufbxi_xml_context`
#[repr(C)]
pub(crate) struct InnerXmlContext {
    pub error: Error,

    pub ator: *mut Allocator,

    pub tmp_stack: Buf,
    pub result: Buf,

    pub doc: *mut XmlDocument,

    pub read_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>,
    pub read_user: *mut c_void,

    pub tok: *mut u8,
    pub tok_cap: usize,
    pub tok_len: usize,

    pub pos: *const u8,
    pub pos_end: *const u8,
    pub data: [u8; 4096],

    pub io_error: bool,
}

// Safe `&XmlContext` handle over the fields-struct `InnerXmlContext`, mirroring the
// `Context`/`InnerContext` seam in `parse.rs`. `MaybeUninit` keeps it uniform with
// the other context wrappers (the embedded `Error` carries an `ErrorType` enum);
// `UnsafeCell` provides the interior mutability every `&XmlContext` site needs.
#[repr(transparent)]
pub(crate) struct XmlContext(core::cell::UnsafeCell<core::mem::MaybeUninit<InnerXmlContext>>);

impl XmlContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerXmlContext {
        self.0.get().cast()
    }

    #[inline(always)]
    pub(crate) fn set_io_error(&self, io_error: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).io_error = io_error;
        }
    }

    // `pos_end` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn pos_end(&self) -> *const u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*const u8` are valid.
        unsafe { (*self.get()).pos_end }
    }

    #[inline(always)]
    pub(crate) fn set_pos_end(&self, pos_end: *const u8) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).pos_end = pos_end;
        }
    }

    // `pos` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn pos(&self) -> *const u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*const u8` are valid.
        unsafe { (*self.get()).pos }
    }

    #[inline(always)]
    pub(crate) fn set_pos(&self, pos: *const u8) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).pos = pos;
        }
    }

    // `tok_len` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tok_len(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).tok_len }
    }

    #[inline(always)]
    pub(crate) fn set_tok_len(&self, tok_len: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).tok_len = tok_len;
        }
    }

    // `tok_cap` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tok_cap(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).tok_cap }
    }

    // `tok` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tok(&self) -> *mut u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u8` are valid.
        unsafe { (*self.get()).tok }
    }

    // `read_user` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn read_user(&self) -> *mut c_void {
        // SAFETY: reading a scalar field; all bit patterns of `*mut c_void` are valid.
        unsafe { (*self.get()).read_user }
    }

    #[inline(always)]
    pub(crate) fn set_read_user(&self, read_user: *mut c_void) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).read_user = read_user;
        }
    }

    // `read_fn` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn read_fn(
        &self,
    ) -> Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize> {
        // SAFETY: reading a scalar field; all bit patterns of `Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>` are valid.
        unsafe { (*self.get()).read_fn }
    }

    #[inline(always)]
    pub(crate) fn set_read_fn(
        &self,
        read_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>,
    ) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).read_fn = read_fn;
        }
    }

    // `doc` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn doc(&self) -> *mut XmlDocument {
        // SAFETY: reading a scalar field; all bit patterns of `*mut XmlDocument` are valid.
        unsafe { (*self.get()).doc }
    }

    #[inline(always)]
    pub(crate) fn set_doc(&self, doc: *mut XmlDocument) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).doc = doc;
        }
    }

    // `ator` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn ator(&self) -> *mut Allocator {
        // SAFETY: reading a scalar field; all bit patterns of `*mut Allocator` are valid.
        unsafe { (*self.get()).ator }
    }

    #[inline(always)]
    pub(crate) fn set_ator(&self, ator: *mut Allocator) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).ator = ator;
        }
    }
}

// ufbx.c:7297-7304 `enum { UFBXI_XML_CTYPE_* }`
pub(crate) const XML_CTYPE_WHITESPACE: u32 = 0x1;
pub(crate) const XML_CTYPE_SINGLE_QUOTE: u32 = 0x2;
pub(crate) const XML_CTYPE_DOUBLE_QUOTE: u32 = 0x4;
pub(crate) const XML_CTYPE_NAME_END: u32 = 0x8;
pub(crate) const XML_CTYPE_TAG_START: u32 = 0x10;
pub(crate) const XML_CTYPE_END_OF_FILE: u32 = 0x20;

// ufbx.c:7307-7310 `static const uint8_t ufbxi_xml_ctype[256]`
// Generated by `misc/gen_xml_ctype.py`
// The C initializer lists only the first 64 entries; the remaining 192 are
// implicitly zero (C aggregate initialization).
static XML_CTYPE: [u8; 256] = {
    let init: [u8; 64] = [
        32, 0, 0, 0, 0, 0, 0, 0, 0, 9, 9, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 9, 0, 12, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        16, 8, 8, 8,
    ];
    let mut table = [0u8; 256];
    let mut i = 0usize;
    while i < 64 {
        table[i] = init[i];
        i += 1;
    }
    table
};

// ufbx.c:7312-7321 `ufbxi_xml_refill`
#[inline(never)]
pub(crate) unsafe fn xml_refill(xc: &XmlContext) {
    let mut num: usize = (xc.read_fn().unwrap_unchecked())(
        xc.read_user(),
        (*xc.get()).data.as_mut_ptr() as *mut c_void,
        size_of_val(&(*xc.get()).data),
    );
    if num == usize::MAX || num < size_of_val(&(*xc.get()).data) {
        xc.set_io_error(true);
    }
    if num < size_of_val(&(*xc.get()).data) {
        // C: `xc->data[num++] = '\0';`
        (*xc.get()).data[num] = b'\0';
        num += 1;
    }
    xc.set_pos((*xc.get()).data.as_ptr());
    // C-parity: `xc->pos_end = xc->data + num;` — a misbehaving `read_fn` can
    // return `SIZE_MAX` here (the `io_error` flag is set but the pointer is
    // still formed), so the offset is applied with wrapping semantics rather
    // than `add`, which would trip the pointer-overflow precondition.
    xc.set_pos_end((*xc.get()).data.as_ptr().wrapping_add(num));
}

// ufbx.c:7323-7326 `ufbxi_xml_advance`
#[inline(always)]
pub(crate) unsafe fn xml_advance(xc: &XmlContext) {
    // C: `if (++xc->pos == xc->pos_end)` — pre-increment decomposed.
    xc.set_pos(xc.pos().add(1));
    if xc.pos() == xc.pos_end() {
        xml_refill(xc);
    }
}

// ufbx.c:7328-7335 `ufbxi_xml_push_token_char`
#[inline(never)]
pub(crate) unsafe fn xml_push_token_char(xc: &XmlContext, c: u8) -> Result<(), Fail> {
    if xc.tok_len() == xc.tok_cap() || IS_REGRESSION {
        ufbxi_check_err!(
            &mut (*xc.get()).error,
            grow_array::<u8>(
                xc.ator(),
                &mut (*xc.get()).tok,
                &mut (*xc.get()).tok_cap,
                xc.tok_len() + 1
            ),
            "ufbxi_grow_array_size((xc->ator), sizeof(**(&xc->tok)), (&xc->tok), (&xc->tok_cap), (xc->tok_len + 1))"
        );
    }
    // C: `xc->tok[xc->tok_len++] = c;`
    *xc.tok().add(xc.tok_len()) = c;
    xc.set_tok_len(xc.tok_len() + 1);
    Ok(())
}

// ufbx.c:7337-7345 `ufbxi_xml_accept`
// C returns `int` 1/0; every call site consumes it as a boolean.
#[inline(never)]
pub(crate) unsafe fn xml_accept(xc: &XmlContext, ch: u8) -> bool {
    if *xc.pos() == ch {
        xml_advance(xc);
        true
    } else {
        false
    }
}

// ufbx.c:7347-7352 `ufbxi_xml_skip_while`
#[inline(never)]
pub(crate) unsafe fn xml_skip_while(xc: &XmlContext, ctypes: u32) {
    while XML_CTYPE[*xc.pos() as usize] as u32 & ctypes != 0 {
        xml_advance(xc);
    }
}

// ufbx.c:7354-7386 `ufbxi_xml_skip_until_string`
#[allow(unused_assignments)]
#[inline(never)]
pub(crate) unsafe fn xml_skip_until_string(
    xc: &XmlContext,
    dst: *mut String,
    suffix: *const u8,
) -> Result<(), Fail> {
    xc.set_tok_len(0);
    let mut match_len: usize = 0;
    let mut ix: usize = 0;
    let suffix_len: usize = crate::native::error::strlen(suffix);
    let mut buf: [u8; 16] = [0; 16];
    let wrap_mask: usize = buf.len() - 1;
    ufbx_assert!(suffix_len < buf.len());
    loop {
        let c: u8 = *xc.pos();
        ufbxi_check_err_msg!(&mut (*xc.get()).error, c != 0, "Truncated file");
        xml_advance(xc);
        if ix >= suffix_len {
            xml_push_token_char(xc, buf[(ix - suffix_len) & wrap_mask])?;
        }

        // C: `buf[ix++ & wrap_mask] = c;`
        buf[ix & wrap_mask] = c;
        ix += 1;
        match_len = 0;
        while match_len < suffix_len {
            // C-parity: `ix - suffix_len + match_len` wraps while the ring
            // buffer is still filling; the mask makes the wrapped value valid.
            if buf[ix.wrapping_sub(suffix_len).wrapping_add(match_len) & wrap_mask]
                != *suffix.add(match_len)
            {
                break;
            }
            match_len += 1;
        }
        if match_len == suffix_len {
            break;
        }
    }

    xml_push_token_char(xc, b'\0')?;
    if !dst.is_null() {
        (*dst).length = xc.tok_len() - 1;
        (*dst).data = push_copy::<u8>(&mut (*xc.get()).result, xc.tok_len(), xc.tok());
        ufbxi_check_err!(&mut (*xc.get()).error, !(*dst).data.is_null(), "dst->data");
    }

    Ok(())
}

// ufbx.c:7388-7463 `ufbxi_xml_read_until`
#[allow(unused_assignments)]
#[inline(never)]
pub(crate) unsafe fn xml_read_until(
    xc: &XmlContext,
    dst: *mut String,
    ctypes: u32,
) -> Result<(), Fail> {
    xc.set_tok_len(0);
    loop {
        let mut c: u8 = *xc.pos();

        if c == b'&' {
            let entity_begin: usize = xc.tok_len();
            loop {
                xml_advance(xc);
                c = *xc.pos();
                ufbxi_check_err!(&mut (*xc.get()).error, c != b'\0', "c != '\\0'");
                if c == b';' {
                    break;
                }
                xml_push_token_char(xc, c)?;
            }
            xml_advance(xc);
            xml_push_token_char(xc, b'\0')?;

            let entity: *mut u8 = xc.tok().add(entity_begin);
            xc.set_tok_len(entity_begin);

            if *entity.add(0) == b'#' {
                // C: `unsigned long code` — 64-bit on the oracle targets; the
                // value always comes from `ufbxi_parse_uint32_radix`.
                let mut code: u64 = 0;
                if *entity.add(1) == b'x' {
                    code = crate::native::float_parse::parse_uint32_radix(entity.add(2), 16) as u64;
                } else {
                    code = crate::native::float_parse::parse_uint32_radix(entity.add(1), 10) as u64;
                }

                let mut bytes: [u8; 5] = [0; 5];
                if code < 0x80 {
                    bytes[0] = code as u8;
                } else if code < 0x800 {
                    bytes[0] = (0xc0 | (code >> 6)) as u8;
                    bytes[1] = (0x80 | (code & 0x3f)) as u8;
                } else if code < 0x10000 {
                    bytes[0] = (0xe0 | (code >> 12)) as u8;
                    bytes[1] = (0x80 | ((code >> 6) & 0x3f)) as u8;
                    bytes[2] = (0x80 | (code & 0x3f)) as u8;
                } else {
                    bytes[0] = (0xf0 | (code >> 18)) as u8;
                    bytes[1] = (0x80 | ((code >> 12) & 0x3f)) as u8;
                    bytes[2] = (0x80 | ((code >> 6) & 0x3f)) as u8;
                    bytes[3] = (0x80 | (code & 0x3f)) as u8;
                }
                // C: `for (char *b = bytes; *b; b++)`
                let mut b: *mut u8 = bytes.as_mut_ptr();
                while *b != 0 {
                    xml_push_token_char(xc, *b)?;
                    b = b.add(1);
                }
            } else {
                let mut ch: u8 = b'\0';
                if strcmp(entity, b"lt\0".as_ptr()) == 0 {
                    ch = b'<';
                } else if strcmp(entity, b"quot\0".as_ptr()) == 0 {
                    ch = b'"';
                } else if strcmp(entity, b"amp\0".as_ptr()) == 0 {
                    ch = b'&';
                } else if strcmp(entity, b"apos\0".as_ptr()) == 0 {
                    ch = b'\'';
                } else if strcmp(entity, b"gt\0".as_ptr()) == 0 {
                    ch = b'>';
                }
                if ch != 0 {
                    xml_push_token_char(xc, ch)?;
                }
            }
        } else {
            if (XML_CTYPE[c as usize] as u32 & ctypes) != 0 {
                break;
            }
            ufbxi_check_err_msg!(&mut (*xc.get()).error, c != 0, "Truncated file");
            xml_push_token_char(xc, c)?;
            xml_advance(xc);
        }
    }

    xml_push_token_char(xc, b'\0')?;
    if !dst.is_null() {
        (*dst).length = xc.tok_len() - 1;
        (*dst).data = push_copy::<u8>(&mut (*xc.get()).result, xc.tok_len(), xc.tok());
        ufbxi_check_err!(&mut (*xc.get()).error, !(*dst).data.is_null(), "dst->data");
    }

    Ok(())
}

// Recursion limited by check at the start
// ufbx.c:7466-7584 `ufbxi_xml_parse_tag`
// `ufbxi_recursive_function(int, ufbxi_xml_parse_tag, (xc, depth, p_closing,
// opening), UFBXI_MAX_XML_DEPTH + 1, ...)` (ufbx.c:7467-7468): under
// regression a thread-local depth guard wraps the recursive body; otherwise
// the macro is empty and the wrapper is a plain call.
#[inline(never)]
pub(crate) unsafe fn xml_parse_tag(
    xc: &XmlContext,
    depth: usize,
    p_closing: *mut bool,
    opening: *const u8,
) -> Result<(), Fail> {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|d| {
            ufbx_assert!((d.get() as usize) < MAX_XML_DEPTH + 1);
            d.set(d.get() + 1);
        });
        let ret = xml_parse_tag_rec(xc, depth, p_closing, opening);
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        return ret;
    }
    #[cfg(not(feature = "regression"))]
    xml_parse_tag_rec(xc, depth, p_closing, opening)
}

// ufbx.c:7469-7584 `ufbxi_xml_parse_tag` body (the `_rec` half of the
// `ufbxi_recursive_function` body; see the wrapper above)
#[allow(unused_assignments)]
#[inline(never)]
unsafe fn xml_parse_tag_rec(
    xc: &XmlContext,
    depth: usize,
    p_closing: *mut bool,
    opening: *const u8,
) -> Result<(), Fail> {
    ufbxi_check_err!(
        &mut (*xc.get()).error,
        depth < MAX_XML_DEPTH,
        "depth < UFBXI_MAX_XML_DEPTH"
    );

    if !xml_accept(xc, b'<') {
        if *xc.pos() == b'\0' {
            *p_closing = true;
        } else {
            xml_read_until(
                xc,
                core::ptr::null_mut(),
                XML_CTYPE_TAG_START | XML_CTYPE_END_OF_FILE,
            )?;
            let mut has_text: bool = false;
            let mut i: usize = 0;
            while i < xc.tok_len() {
                if (XML_CTYPE[*xc.tok().add(i) as usize] as u32 & XML_CTYPE_WHITESPACE) == 0 {
                    has_text = true;
                    break;
                }
                i += 1;
            }

            if has_text {
                let tag: *mut XmlTag = push_zero(&mut (*xc.get()).tmp_stack, 1);
                ufbxi_check_err!(&mut (*xc.get()).error, !tag.is_null(), "tag");
                (*tag).name.data = EMPTY_CHAR.as_ptr();

                (*tag).text.length = xc.tok_len() - 1;
                (*tag).text.data = push_copy::<u8>(&mut (*xc.get()).result, xc.tok_len(), xc.tok());
                ufbxi_check_err!(
                    &mut (*xc.get()).error,
                    !(*tag).text.data.is_null(),
                    "tag->text.data"
                );
            }
        }
        return Ok(());
    }

    if xml_accept(xc, b'/') {
        xml_read_until(xc, core::ptr::null_mut(), XML_CTYPE_NAME_END)?;
        ufbxi_check_err!(
            &mut (*xc.get()).error,
            !opening.is_null() && strcmp(xc.tok(), opening) == 0,
            "opening && !strcmp(xc->tok, opening)"
        );
        xml_skip_while(xc, XML_CTYPE_WHITESPACE);
        if !xml_accept(xc, b'>') {
            return Err(Fail);
        }
        *p_closing = true;
        return Ok(());
    } else if xml_accept(xc, b'!') {
        if xml_accept(xc, b'[') {
            // C: `for (const char *ch = "CDATA["; *ch; ch++)`
            let mut ch: *const u8 = b"CDATA[\0".as_ptr();
            while *ch != 0 {
                if !xml_accept(xc, *ch) {
                    return Err(Fail);
                }
                ch = ch.add(1);
            }

            let tag: *mut XmlTag = push_zero(&mut (*xc.get()).tmp_stack, 1);
            ufbxi_check_err!(&mut (*xc.get()).error, !tag.is_null(), "tag");
            xml_skip_until_string(xc, &mut (*tag).text, b"]]>\0".as_ptr())?;
            (*tag).name.data = EMPTY_CHAR.as_ptr();
        } else if xml_accept(xc, b'-') {
            if !xml_accept(xc, b'-') {
                return Err(Fail);
            }
            xml_skip_until_string(xc, core::ptr::null_mut(), b"-->\0".as_ptr())?;
        } else {
            // TODO: !DOCTYPE
            xml_skip_until_string(xc, core::ptr::null_mut(), b">\0".as_ptr())?;
        }
        return Ok(());
    } else if xml_accept(xc, b'?') {
        xml_skip_until_string(xc, core::ptr::null_mut(), b"?>\0".as_ptr())?;
        return Ok(());
    }

    let tag: *mut XmlTag = push_zero(&mut (*xc.get()).tmp_stack, 1);
    ufbxi_check_err!(&mut (*xc.get()).error, !tag.is_null(), "tag");
    xml_read_until(xc, &mut (*tag).name, XML_CTYPE_NAME_END)?;
    (*tag).text.data = EMPTY_CHAR.as_ptr();

    let mut has_children: bool = false;

    let mut num_attribs: usize = 0;
    loop {
        xml_skip_while(xc, XML_CTYPE_WHITESPACE);
        if xml_accept(xc, b'/') {
            if !xml_accept(xc, b'>') {
                return Err(Fail);
            }
            break;
        } else if xml_accept(xc, b'>') {
            has_children = true;
            break;
        } else {
            let attrib: *mut XmlAttrib = push_zero(&mut (*xc.get()).tmp_stack, 1);
            ufbxi_check_err!(&mut (*xc.get()).error, !attrib.is_null(), "attrib");
            xml_read_until(xc, &mut (*attrib).name, XML_CTYPE_NAME_END)?;
            xml_skip_while(xc, XML_CTYPE_WHITESPACE);
            if !xml_accept(xc, b'=') {
                return Err(Fail);
            }
            xml_skip_while(xc, XML_CTYPE_WHITESPACE);
            let mut quote_ctype: u32 = 0;
            if xml_accept(xc, b'"') {
                quote_ctype = XML_CTYPE_DOUBLE_QUOTE;
            } else if xml_accept(xc, b'\'') {
                quote_ctype = XML_CTYPE_SINGLE_QUOTE;
            } else {
                ufbxi_fail_err!(&mut (*xc.get()).error, "Bad attrib value");
            }
            xml_read_until(xc, &mut (*attrib).value, quote_ctype)?;
            xml_advance(xc);
            num_attribs += 1;
        }
    }

    (*tag).num_attribs = num_attribs;
    (*tag).attribs = push_pop(
        &mut (*xc.get()).result,
        &mut (*xc.get()).tmp_stack,
        num_attribs,
    );
    ufbxi_check_err!(
        &mut (*xc.get()).error,
        !(*tag).attribs.is_null(),
        "tag->attribs"
    );

    if has_children {
        let children_begin: usize = (*xc.get()).tmp_stack.num_items;
        loop {
            let mut closing: bool = false;
            xml_parse_tag(xc, depth + 1, &mut closing, (*tag).name.data)?;
            if closing {
                break;
            }
        }

        (*tag).num_children = (*xc.get()).tmp_stack.num_items - children_begin;
        (*tag).children = push_pop(
            &mut (*xc.get()).result,
            &mut (*xc.get()).tmp_stack,
            (*tag).num_children,
        );
        ufbxi_check_err!(
            &mut (*xc.get()).error,
            !(*tag).children.is_null(),
            "tag->children"
        );
    }

    Ok(())
}

// ufbx.c:7586-7610 `ufbxi_xml_parse_root`
#[inline(never)]
pub(crate) unsafe fn xml_parse_root(xc: &XmlContext) -> Result<(), Fail> {
    let tag: *mut XmlTag = push_zero(&mut (*xc.get()).result, 1);
    ufbxi_check_err!(&mut (*xc.get()).error, !tag.is_null(), "tag");
    (*tag).name.data = EMPTY_CHAR.as_ptr();
    (*tag).text.data = EMPTY_CHAR.as_ptr();

    loop {
        let mut closing: bool = false;
        xml_parse_tag(xc, 0, &mut closing, core::ptr::null())?;
        if closing {
            break;
        }
    }

    (*tag).num_children = (*xc.get()).tmp_stack.num_items;
    (*tag).children = push_pop(
        &mut (*xc.get()).result,
        &mut (*xc.get()).tmp_stack,
        (*tag).num_children,
    );
    ufbxi_check_err!(
        &mut (*xc.get()).error,
        !(*tag).children.is_null(),
        "tag->children"
    );

    xc.set_doc(push(&mut (*xc.get()).result, 1));
    ufbxi_check_err!(&mut (*xc.get()).error, !xc.doc().is_null(), "xc->doc");

    (*xc.doc()).root = tag;
    (*xc.doc()).buf = (*xc.get()).result;

    Ok(())
}

// ufbx.c:7612-7618 `ufbxi_xml_load_opts`
#[repr(C)]
pub(crate) struct XmlLoadOpts {
    pub ator: *mut Allocator,
    pub read_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>,
    pub read_user: *mut c_void,
    pub prefix: *const u8,
    pub prefix_length: usize,
}

// ufbx.c:7620-7654 `ufbxi_load_xml`
#[inline(never)]
pub(crate) unsafe fn load_xml(opts: *mut XmlLoadOpts, error: *mut Error) -> *mut XmlDocument {
    // C: `ufbxi_xml_context xc = { UFBX_ERROR_NONE };` — the aggregate
    // initializer zeroes the whole (4 KiB) context.
    let xc: XmlContext = core::mem::zeroed();
    xc.set_ator((*opts).ator);
    xc.set_read_fn((*opts).read_fn);
    xc.set_read_user((*opts).read_user);

    (*xc.get()).tmp_stack.ator = xc.ator();
    (*xc.get()).result.ator = xc.ator();

    (*xc.get()).result.unordered = true;

    if (*opts).prefix_length > 0 {
        xc.set_pos((*opts).prefix);
        xc.set_pos_end((*opts).prefix.add((*opts).prefix_length));
    } else {
        xml_refill(&xc);
    }

    let ok = xml_parse_root(&xc).is_ok();

    buf_free(&mut (*xc.get()).tmp_stack);
    free::<u8>(xc.ator(), xc.tok(), xc.tok_cap());

    if ok {
        xc.doc()
    } else {
        buf_free(&mut (*xc.get()).result);
        if !error.is_null() {
            core::ptr::write(error, core::ptr::read(&(*xc.get()).error));
        }

        core::ptr::null_mut()
    }
}

// ufbx.c:7656-7660 `ufbxi_free_xml`
#[inline(never)]
pub(crate) unsafe fn free_xml(doc: *mut XmlDocument) {
    let mut buf: Buf = (*doc).buf;
    buf_free(&mut buf);
}

// ufbx.c:7662-7670 `ufbxi_xml_find_child`
#[inline(never)]
pub(crate) unsafe fn xml_find_child(tag: *mut XmlTag, name: *const u8) -> *mut XmlTag {
    // C: `ufbxi_for(ufbxi_xml_tag, child, tag->children, tag->num_children)`
    let mut child: *mut XmlTag = (*tag).children;
    let child_end: *mut XmlTag = add_ptr(child, (*tag).num_children);
    while child != child_end {
        if strcmp((*child).name.data, name) == 0 {
            return child;
        }
        child = child.add(1);
    }
    core::ptr::null_mut()
}

// ufbx.c:7672-7680 `ufbxi_xml_find_attrib`
#[inline(never)]
pub(crate) unsafe fn xml_find_attrib(tag: *mut XmlTag, name: *const u8) -> *mut XmlAttrib {
    // C: `ufbxi_for(ufbxi_xml_attrib, attrib, tag->attribs, tag->num_attribs)`
    let mut attrib: *mut XmlAttrib = (*tag).attribs;
    let attrib_end: *mut XmlAttrib = add_ptr(attrib, (*tag).num_attribs);
    while attrib != attrib_end {
        if strcmp((*attrib).name.data, name) == 0 {
            return attrib;
        }
        attrib = attrib.add(1);
    }
    core::ptr::null_mut()
}
