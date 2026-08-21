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
use core::ffi::{c_void, CStr};

use crate::generated::Error;
use crate::native::allocator::{free, grow_array, Allocator};
use crate::native::buf::{buf_free, push_copy, Buf};
use crate::native::error::{
    strcmp, ufbxi_check_err, ufbxi_check_err_msg, ufbxi_fail_err, Fail, EMPTY_CHAR,
};
use crate::native::platform::{ufbx_assert, IS_REGRESSION};
use crate::native::view::{SliceViewIter, View};
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

// Reinterpret-in-place views over arena-allocated `XmlTag`/`XmlAttrib` runs
// (Rust-port infrastructure; see `native::view`). ufbx materializes children and
// attribs as contiguous `push_pop` runs walked by `ufbxi_for`, so
// `SliceViewIter` over `(children/attribs, num_children/num_attribs)` is the safe
// iteration form. `View<T>` supplies `get()` / `from_ptr()`; the accessors below
// are the per-struct residue.
pub(crate) type XmlTagView = View<XmlTag>;

impl View<XmlTag> {
    #[inline(always)]
    pub(crate) fn name_data(&self) -> *const u8 {
        // SAFETY: reading the `name.data` pointer field of a valid arena `XmlTag`.
        unsafe { (*self.get()).name.data }
    }
    #[inline(always)]
    pub(crate) fn text(&self) -> String {
        // SAFETY: `String` is a POD `{ptr,len}`; reading it from a valid `XmlTag`.
        unsafe { (*self.get()).text }
    }
    #[inline(always)]
    pub(crate) fn attribs(&self) -> *mut XmlAttrib {
        // SAFETY: reading the `attribs` run pointer of a valid arena `XmlTag`.
        unsafe { (*self.get()).attribs }
    }
    #[inline(always)]
    pub(crate) fn num_attribs(&self) -> usize {
        // SAFETY: reading a `usize` count field.
        unsafe { (*self.get()).num_attribs }
    }
    #[inline(always)]
    pub(crate) fn children(&self) -> *mut XmlTag {
        // SAFETY: reading the `children` run pointer of a valid arena `XmlTag`.
        unsafe { (*self.get()).children }
    }
    #[inline(always)]
    pub(crate) fn num_children(&self) -> usize {
        // SAFETY: reading a `usize` count field.
        unsafe { (*self.get()).num_children }
    }
}

pub(crate) type XmlAttribView = View<XmlAttrib>;

impl View<XmlAttrib> {
    #[inline(always)]
    pub(crate) fn name_data(&self) -> *const u8 {
        // SAFETY: reading the `name.data` pointer field of a valid arena `XmlAttrib`.
        unsafe { (*self.get()).name.data }
    }
    #[inline(always)]
    pub(crate) fn value(&self) -> String {
        // SAFETY: `String` is a POD `{ptr,len}`; reading it from a valid `XmlAttrib`.
        unsafe { (*self.get()).value }
    }
}

// ufbx.c:7269-7272 `ufbxi_xml_document`
// NOT `Copy`/`Clone`: embeds the owning `Buf` the whole document is allocated
// from — see PORTING.md "Copy vs non-Copy structs".
#[repr(C)]
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
    pub(crate) fn data_size(&self) -> usize {
        // SAFETY: `size_of_val` only needs the place's type; the `data` array is
        // a field of the context this handle owns.
        unsafe { core::mem::size_of_val(&(*self.get()).data) }
    }

    #[inline(always)]
    /// Moves the field out by bitwise read (`ptr::read`). C does this as plain
    /// struct assignment; the source field still holds the stale bits (no
    /// `Drop`), so the caller must overwrite it or treat it as moved-from.
    pub(crate) fn take_result(&self) -> crate::native::buf::Buf {
        // SAFETY: the `result` field is live, initialized context storage; the
        // bitwise read moves the `Buf` out, leaving stale bits behind (no
        // `Drop`), which the doc comment above makes the caller's obligation.
        unsafe { core::ptr::read(&raw const (*self.get()).result) }
    }

    // `data` (`[u8; 4096]`) — whole-array raw-ptr getters (read/write buffer base).
    #[inline(always)]
    pub(crate) fn data_ptr(&self) -> *const u8 {
        // SAFETY: `&raw mut` computes the array's base address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { (&raw mut (*self.get()).data) as *const u8 }
    }
    #[inline(always)]
    pub(crate) fn data_mut_ptr(&self) -> *mut u8 {
        // SAFETY: `&raw mut` computes the array's base address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { (&raw mut (*self.get()).data) as *mut u8 }
    }
    #[inline(always)]
    pub(crate) fn data_at(&self, i: usize) -> &crate::prelude::ScalarView<u8> {
        // SAFETY: the indexing panics unless `i` is inside the `data` array, so
        // the reinterpreted element cell lies in context-owned interior-mutable
        // storage; the borrow of `self` anchors its lifetime.
        unsafe { &*(&raw mut (*self.get()).data[i] as *mut crate::prelude::ScalarView<u8>) }
    }

    // `result` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn result_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).result as *mut crate::native::buf::BufView) }
    }

    // `tmp_stack` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_stack_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_stack as *mut crate::native::buf::BufView) }
    }

    // `tok_cap` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tok_cap_mut_ptr(&self) -> *mut usize {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tok_cap }
    }

    // `tok` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tok_mut_ptr(&self) -> *mut *mut u8 {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tok }
    }

    // `tmp_stack` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_stack_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_stack }
    }

    // `result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn result_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).result }
    }

    // `error` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn error_mut_ptr(&self) -> *mut Error {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).error }
    }

    // `error` — anchored VIEW handle; accessors on `ErrorView`. Routes the
    // error-form check macros through the SAFE `fail_err`/`fail_err_no_stack`.
    #[inline(always)]
    pub(crate) fn error_view(&self) -> &crate::native::error::ErrorView {
        // SAFETY: the context-owned `error` field is interior-mutable arena memory;
        // `&raw mut` keeps write provenance (never `&T`); borrow of `self` anchors `'a <= self`.
        unsafe { crate::native::error::ErrorView::from_ptr(&raw mut (*self.get()).error) }
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
pub(crate) fn xml_refill(xc: &XmlContext) {
    // SAFETY: the sole entry point (`cache_load_xml` -> `load_xml`) fills
    // `read_fn` from an opened stream, so it is Some wherever refill runs; the
    // callback reads into `data`, which addresses `data_size()` writable bytes.
    // (`load_xml`'s signature admits a null `read_fn` with a non-empty prefix;
    // no caller does that today, so `unwrap_unchecked` holds.)
    let mut num: usize = unsafe {
        (xc.read_fn().unwrap_unchecked())(
            xc.read_user(),
            xc.data_mut_ptr() as *mut c_void,
            xc.data_size(),
        )
    };
    // PORT DIVERGENCE (ufbx.c:7315): C keeps `num == SIZE_MAX` (the read_fn
    // IO-error return), skipping the sentinel append and wrapping `pos_end` to
    // `data - 1` so later scans run past the buffer. Clamp it to a zero-length
    // read so the sentinel is appended and `pos_end` stays in bounds (the
    // `io_error` flag is then set by the short-read branch); reconcile once
    // upstream lands the fix.
    if num == usize::MAX {
        num = 0;
    }
    if num < xc.data_size() {
        xc.set_io_error(true);
        // C: `xc->data[num++] = '\0';`
        xc.data_at(num).set(b'\0');
        num += 1;
    }
    xc.set_pos(xc.data_ptr());
    // C: `xc->pos_end = xc->data + num;`. `num <= data_size()` after the clamp
    // above, so this stays within `data`'s one-past-the-end bound.
    xc.set_pos_end(xc.data_ptr().wrapping_add(num));
}

// ufbx.c:7323-7326 `ufbxi_xml_advance`
#[inline(always)]
pub(crate) fn xml_advance(xc: &XmlContext) {
    // C: `if (++xc->pos == xc->pos_end)` — pre-increment decomposed.
    // SAFETY: `pos < pos_end` on entry (the caller reads `*pos` first), and
    // `pos_end` bounds the current window (the prefix, or the refill buffer
    // one-past its last byte), so advancing by one reaches at most `pos_end`.
    xc.set_pos(unsafe { xc.pos().add(1) });
    if xc.pos() == xc.pos_end() {
        xml_refill(xc);
    }
}

// ufbx.c:7328-7335 `ufbxi_xml_push_token_char`
#[inline(never)]
pub(crate) fn xml_push_token_char(xc: &XmlContext, c: u8) -> Result<(), Fail> {
    if xc.tok_len() == xc.tok_cap() || IS_REGRESSION {
        ufbxi_check_err!(
            xc.error_view(),
            // SAFETY: growing xc's own paired `tok`/`tok_cap` growth state
            // through its allocator (xc construction invariant).
            unsafe {
                grow_array::<u8>(
                    xc.ator(),
                    xc.tok_mut_ptr(),
                    xc.tok_cap_mut_ptr(),
                    xc.tok_len() + 1
                )
            },
            "ufbxi_grow_array_size((xc->ator), sizeof(**(&xc->tok)), (&xc->tok), (&xc->tok_cap), (xc->tok_len + 1))"
        );
    }
    // C: `xc->tok[xc->tok_len++] = c;`
    // SAFETY: `tok_len < tok_cap` holds here — either it already did, or the
    // grow above succeeded to at least `tok_len + 1` — so this indexes within
    // the token allocation.
    unsafe { *xc.tok().add(xc.tok_len()) = c };
    xc.set_tok_len(xc.tok_len() + 1);
    Ok(())
}

// ufbx.c:7337-7345 `ufbxi_xml_accept`
// C returns `int` 1/0; every call site consumes it as a boolean.
#[inline(never)]
pub(crate) fn xml_accept(xc: &XmlContext, ch: u8) -> bool {
    // SAFETY: `pos < pos_end`, so `*pos` is a readable byte. `xml_advance`
    // refills exactly when `pos` reaches `pos_end`, and `xml_refill` always
    // leaves `pos < pos_end` (a data byte, or the terminating NUL on a short or
    // errored read), so every read here lands on a live byte of the current
    // window (the prefix or the refill buffer).
    if unsafe { *xc.pos() } == ch {
        xml_advance(xc);
        true
    } else {
        false
    }
}

// ufbx.c:7347-7352 `ufbxi_xml_skip_while`
#[inline(never)]
pub(crate) fn xml_skip_while(xc: &XmlContext, ctypes: u32) {
    // SAFETY: `pos < pos_end`, so `*pos` is a readable byte — `xml_advance`
    // refills at `pos == pos_end` and `xml_refill` always leaves `pos < pos_end`
    // (a data byte, or the terminating NUL); the NUL is not in `ctypes`, so the
    // loop stops there rather than advancing past it.
    while XML_CTYPE[unsafe { *xc.pos() } as usize] as u32 & ctypes != 0 {
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
    // SAFETY: `suffix` is a NUL-terminated string (every call site passes a
    // byte-string literal), which is `strlen`'s raw-param contract.
    let suffix_len: usize = unsafe { crate::native::error::strlen(suffix) };
    let mut buf: [u8; 16] = [0; 16];
    let wrap_mask: usize = buf.len() - 1;
    ufbx_assert!(suffix_len < buf.len());
    loop {
        // SAFETY: `pos < pos_end`, so `*pos` is a readable byte of the current
        // window (prefix or refill buffer) — `xml_advance` refills at
        // `pos == pos_end` and `xml_refill` always leaves `pos < pos_end`; the
        // `c != 0` check below stops the loop at the NUL a short/errored refill
        // appends, before another read can run past the data.
        let c: u8 = unsafe { *xc.pos() };
        ufbxi_check_err_msg!(xc.error_view(), c != 0, "Truncated file");
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
            // SAFETY: `match_len < suffix_len`, so the offset stays inside the
            // NUL-terminated `suffix` string.
            if buf[ix.wrapping_sub(suffix_len).wrapping_add(match_len) & wrap_mask]
                != unsafe { *suffix.add(match_len) }
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
        // SAFETY: `dst` is non-null per the check and points at a caller-owned
        // `String` slot (fn raw-param contract).
        unsafe { (*dst).length = xc.tok_len() - 1 };
        // SAFETY: `dst` as above; `push_copy` copies `tok_len` bytes out of xc's
        // own token buffer, which holds exactly that many, into xc's result buf.
        unsafe { (*dst).data = push_copy::<u8>(xc.result_mut_ptr(), xc.tok_len(), xc.tok()) };
        ufbxi_check_err!(
            xc.error_view(),
            // SAFETY: reading the pointer field just stored through `dst`.
            !unsafe { (*dst).data }.is_null(),
            "dst->data"
        );
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
        // SAFETY: `pos < pos_end`, so `*pos` is a readable byte of the current
        // window (prefix or refill buffer) — `xml_advance` refills at
        // `pos == pos_end` and `xml_refill` always leaves `pos < pos_end`; the
        // NUL a short/errored refill appends is not in `ctypes` (and the `&`
        // path's `c != 0` check catches it), so the walk stops before running past.
        let mut c: u8 = unsafe { *xc.pos() };

        if c == b'&' {
            let entity_begin: usize = xc.tok_len();
            loop {
                xml_advance(xc);
                // SAFETY: `pos < pos_end` as above (the preceding `xml_advance`
                // refilled if needed), so `*pos` is a readable byte; the
                // `c != '\0'` check stops at the terminating NUL.
                c = unsafe { *xc.pos() };
                ufbxi_check_err!(xc.error_view(), c != b'\0', "c != '\\0'");
                if c == b';' {
                    break;
                }
                xml_push_token_char(xc, c)?;
            }
            xml_advance(xc);
            xml_push_token_char(xc, b'\0')?;

            // SAFETY: `entity_begin` is a token length captured before the
            // entity was pushed, so it is at most the current `tok_len` and the
            // offset stays inside the token allocation.
            let entity: *mut u8 = unsafe { xc.tok().add(entity_begin) };
            xc.set_tok_len(entity_begin);

            // SAFETY: `entity` points at the entity text just pushed, which the
            // `'\0'` push above terminates, so byte 0 is readable.
            if unsafe { *entity.add(0) } == b'#' {
                // C: `unsigned long code` — 64-bit on the oracle targets; the
                // value always comes from `ufbxi_parse_uint32_radix`.
                let mut code: u64 = 0;
                // SAFETY: byte 0 is `'#'`, so byte 1 is still inside the
                // NUL-terminated entity text (worst case it is the terminator).
                if unsafe { *entity.add(1) } == b'x' {
                    // SAFETY: `"#x"` precedes it, so the offset addresses the
                    // remaining NUL-terminated digits `parse_uint32_radix` scans.
                    code = unsafe {
                        crate::native::float_parse::parse_uint32_radix(entity.add(2), 16)
                    } as u64;
                } else {
                    // SAFETY: `'#'` precedes it, so the offset addresses the
                    // remaining NUL-terminated digits `parse_uint32_radix` scans.
                    code = unsafe {
                        crate::native::float_parse::parse_uint32_radix(entity.add(1), 10)
                    } as u64;
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
                // SAFETY (walk and reads): `bytes` is a 5-byte local whose last
                // element stays zero — the encoder above fills at most four —
                // so the NUL walk stops inside the array.
                while unsafe { *b } != 0 {
                    xml_push_token_char(xc, unsafe { *b })?;
                    b = unsafe { b.add(1) };
                }
            } else {
                let mut ch: u8 = b'\0';
                // SAFETY (all five compares): `entity` is the NUL-terminated
                // entity text pushed above and each literal is a NUL-terminated
                // `'static` run, which is `strcmp`'s raw-param contract.
                if unsafe { strcmp(entity, b"lt\0".as_ptr()) } == 0 {
                    ch = b'<';
                } else if unsafe { strcmp(entity, b"quot\0".as_ptr()) } == 0 {
                    ch = b'"';
                } else if unsafe { strcmp(entity, b"amp\0".as_ptr()) } == 0 {
                    ch = b'&';
                } else if unsafe { strcmp(entity, b"apos\0".as_ptr()) } == 0 {
                    ch = b'\'';
                } else if unsafe { strcmp(entity, b"gt\0".as_ptr()) } == 0 {
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
            ufbxi_check_err_msg!(xc.error_view(), c != 0, "Truncated file");
            xml_push_token_char(xc, c)?;
            xml_advance(xc);
        }
    }

    xml_push_token_char(xc, b'\0')?;
    if !dst.is_null() {
        // SAFETY: `dst` is non-null per the check and points at a caller-owned
        // `String` slot (fn raw-param contract).
        unsafe { (*dst).length = xc.tok_len() - 1 };
        // SAFETY: `dst` as above; `push_copy` copies `tok_len` bytes out of xc's
        // own token buffer, which holds exactly that many, into xc's result buf.
        unsafe { (*dst).data = push_copy::<u8>(xc.result_mut_ptr(), xc.tok_len(), xc.tok()) };
        ufbxi_check_err!(
            xc.error_view(),
            // SAFETY: reading the pointer field just stored through `dst`.
            !unsafe { (*dst).data }.is_null(),
            "dst->data"
        );
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
        // SAFETY: `p_closing` and `opening` are forwarded unchanged, so this
        // fn's raw-param contract is exactly the callee's.
        let ret = unsafe { xml_parse_tag_rec(xc, depth, p_closing, opening) };
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        return ret;
    }
    // SAFETY: `p_closing` and `opening` are forwarded unchanged, so this fn's
    // raw-param contract is exactly the callee's.
    #[cfg(not(feature = "regression"))]
    unsafe {
        xml_parse_tag_rec(xc, depth, p_closing, opening)
    }
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
        xc.error_view(),
        depth < MAX_XML_DEPTH,
        "depth < UFBXI_MAX_XML_DEPTH"
    );

    if !xml_accept(xc, b'<') {
        // SAFETY: `pos < pos_end`, so `*pos` is a readable byte of the current
        // window (prefix or refill buffer) — `xml_advance` refills at
        // `pos == pos_end` and `xml_refill` always leaves `pos < pos_end`; this
        // read tests for the terminating NUL itself.
        if unsafe { *xc.pos() } == b'\0' {
            // SAFETY: `p_closing` is the caller's live `bool` out-param (fn
            // raw-param contract).
            unsafe { *p_closing = true };
        } else {
            // SAFETY: a null `dst` is the "discard the token" sentinel the
            // callee checks for.
            unsafe {
                xml_read_until(
                    xc,
                    core::ptr::null_mut(),
                    XML_CTYPE_TAG_START | XML_CTYPE_END_OF_FILE,
                )?
            };
            let mut has_text: bool = false;
            let mut i: usize = 0;
            while i < xc.tok_len() {
                // SAFETY: `i < tok_len <= tok_cap`, so the offset addresses a
                // byte the token buffer holds.
                if (XML_CTYPE[unsafe { *xc.tok().add(i) } as usize] as u32 & XML_CTYPE_WHITESPACE)
                    == 0
                {
                    has_text = true;
                    break;
                }
                i += 1;
            }

            if has_text {
                let tag: *mut XmlTag = xc.tmp_stack_view().push_zero(1);
                ufbxi_check_err!(xc.error_view(), !tag.is_null(), "tag");
                // SAFETY (this store and the two below): `tag` is the fresh,
                // checked-non-null single-element push above, so writing its
                // fields writes xc's own tmp-stack allocation; `EMPTY_CHAR` is a
                // NUL-terminated `'static` run, and `push_copy` copies the
                // `tok_len` bytes the token buffer holds into xc's result buf.
                unsafe { (*tag).name.data = EMPTY_CHAR.as_ptr() };

                unsafe { (*tag).text.length = xc.tok_len() - 1 };
                unsafe {
                    (*tag).text.data = push_copy::<u8>(xc.result_mut_ptr(), xc.tok_len(), xc.tok())
                };
                ufbxi_check_err!(
                    xc.error_view(),
                    // SAFETY: reading the pointer field just stored in `tag`.
                    !unsafe { (*tag).text.data }.is_null(),
                    "tag->text.data"
                );
            }
        }
        return Ok(());
    }

    if xml_accept(xc, b'/') {
        // SAFETY: a null `dst` is the "discard the token" sentinel the callee
        // checks for.
        unsafe { xml_read_until(xc, core::ptr::null_mut(), XML_CTYPE_NAME_END)? };
        ufbxi_check_err!(
            xc.error_view(),
            // SAFETY: `strcmp` runs only once `opening` is known non-null, and
            // both it and xc's token buffer are NUL-terminated (the token by the
            // `'\0'` `xml_read_until` pushes), which is `strcmp`'s contract.
            !opening.is_null() && unsafe { strcmp(xc.tok(), opening) } == 0,
            "opening && !strcmp(xc->tok, opening)"
        );
        xml_skip_while(xc, XML_CTYPE_WHITESPACE);
        if !xml_accept(xc, b'>') {
            return Err(Fail);
        }
        // SAFETY: `p_closing` is the caller's live `bool` out-param (fn
        // raw-param contract).
        unsafe { *p_closing = true };
        return Ok(());
    } else if xml_accept(xc, b'!') {
        if xml_accept(xc, b'[') {
            // C: `for (const char *ch = "CDATA["; *ch; ch++)`
            let mut ch: *const u8 = b"CDATA[\0".as_ptr();
            // SAFETY (walk and reads): `ch` walks a NUL-terminated `'static`
            // literal and the loop stops at its terminator, so every read and
            // the bump stay inside it.
            while unsafe { *ch } != 0 {
                if !xml_accept(xc, unsafe { *ch }) {
                    return Err(Fail);
                }
                ch = unsafe { ch.add(1) };
            }

            let tag: *mut XmlTag = xc.tmp_stack_view().push_zero(1);
            ufbxi_check_err!(xc.error_view(), !tag.is_null(), "tag");
            // SAFETY: `tag` is the fresh, checked-non-null push above, so
            // `&raw mut` on its `text` field addresses xc's own tmp-stack
            // allocation; the suffix is a NUL-terminated `'static` literal.
            unsafe { xml_skip_until_string(xc, &raw mut (*tag).text, b"]]>\0".as_ptr())? };
            // SAFETY: writing the same fresh push; `EMPTY_CHAR` is a
            // NUL-terminated `'static` run.
            unsafe { (*tag).name.data = EMPTY_CHAR.as_ptr() };
        } else if xml_accept(xc, b'-') {
            if !xml_accept(xc, b'-') {
                return Err(Fail);
            }
            // SAFETY: a null `dst` is the "discard the token" sentinel the
            // callee checks for; the suffix is a NUL-terminated `'static`
            // literal.
            unsafe { xml_skip_until_string(xc, core::ptr::null_mut(), b"-->\0".as_ptr())? };
        } else {
            // TODO: !DOCTYPE
            // SAFETY: a null `dst` is the "discard the token" sentinel the
            // callee checks for; the suffix is a NUL-terminated `'static`
            // literal.
            unsafe { xml_skip_until_string(xc, core::ptr::null_mut(), b">\0".as_ptr())? };
        }
        return Ok(());
    } else if xml_accept(xc, b'?') {
        // SAFETY: a null `dst` is the "discard the token" sentinel the callee
        // checks for; the suffix is a NUL-terminated `'static` literal.
        unsafe { xml_skip_until_string(xc, core::ptr::null_mut(), b"?>\0".as_ptr())? };
        return Ok(());
    }

    let tag: *mut XmlTag = xc.tmp_stack_view().push_zero(1);
    ufbxi_check_err!(xc.error_view(), !tag.is_null(), "tag");
    // SAFETY: `tag` is the fresh, checked-non-null push above, so `&raw mut` on
    // its `name` field addresses xc's own tmp-stack allocation.
    unsafe { xml_read_until(xc, &raw mut (*tag).name, XML_CTYPE_NAME_END)? };
    // SAFETY: writing the same fresh push; `EMPTY_CHAR` is a NUL-terminated
    // `'static` run.
    unsafe { (*tag).text.data = EMPTY_CHAR.as_ptr() };

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
            let attrib: *mut XmlAttrib = xc.tmp_stack_view().push_zero(1);
            ufbxi_check_err!(xc.error_view(), !attrib.is_null(), "attrib");
            // SAFETY: `attrib` is the fresh, checked-non-null push above, so
            // `&raw mut` on its `name` field addresses xc's own tmp-stack
            // allocation.
            unsafe { xml_read_until(xc, &raw mut (*attrib).name, XML_CTYPE_NAME_END)? };
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
                ufbxi_fail_err!(xc.error_view(), "Bad attrib value");
            }
            // SAFETY: `attrib` is still that fresh push, so `&raw mut` on its
            // `value` field addresses xc's own tmp-stack allocation.
            unsafe { xml_read_until(xc, &raw mut (*attrib).value, quote_ctype)? };
            xml_advance(xc);
            num_attribs += 1;
        }
    }

    // SAFETY (both stores): `tag` is the fresh, checked-non-null push above;
    // `push_pop` moves exactly the `num_attribs` attribs this loop stacked on
    // xc's tmp stack into xc's result buf.
    unsafe { (*tag).num_attribs = num_attribs };
    unsafe { (*tag).attribs = xc.result_view().push_pop(xc.tmp_stack_view(), num_attribs) };
    ufbxi_check_err!(
        xc.error_view(),
        // SAFETY: reading the pointer field just stored in `tag`.
        !unsafe { (*tag).attribs }.is_null(),
        "tag->attribs"
    );

    if has_children {
        let children_begin: usize = xc.tmp_stack_view().num_items();
        loop {
            let mut closing: bool = false;
            // SAFETY: `closing` is an unaliased local out-param; `tag`'s `name`
            // is the NUL-terminated arena string the callee compares the
            // closing tag against.
            unsafe { xml_parse_tag(xc, depth + 1, &mut closing, (*tag).name.data)? };
            if closing {
                break;
            }
        }

        // SAFETY (both stores): `tag` is that same fresh push; `push_pop` moves
        // exactly the `num_children` tags the loop stacked on xc's tmp stack
        // into xc's result buf.
        unsafe { (*tag).num_children = xc.tmp_stack_view().num_items() - children_begin };
        unsafe {
            (*tag).children = xc
                .result_view()
                .push_pop(xc.tmp_stack_view(), (*tag).num_children)
        };
        ufbxi_check_err!(
            xc.error_view(),
            // SAFETY: reading the pointer field just stored in `tag`.
            !unsafe { (*tag).children }.is_null(),
            "tag->children"
        );
    }

    Ok(())
}

// ufbx.c:7586-7610 `ufbxi_xml_parse_root`
#[inline(never)]
pub(crate) fn xml_parse_root(xc: &XmlContext) -> Result<(), Fail> {
    let tag: *mut XmlTag = xc.result_view().push_zero(1);
    ufbxi_check_err!(xc.error_view(), !tag.is_null(), "tag");
    // SAFETY: `tag` is a fresh non-null single-element push, so writing its
    // fields is writing our own allocation; `EMPTY_CHAR` is a `'static` run.
    unsafe {
        (*tag).name.data = EMPTY_CHAR.as_ptr();
        (*tag).text.data = EMPTY_CHAR.as_ptr();
    }

    loop {
        let mut closing: bool = false;
        // SAFETY: `closing` is an unaliased local out-param; a null `name`
        // is the "root has no expected closing tag" sentinel the callee reads
        // as such.
        unsafe { xml_parse_tag(xc, 0, &mut closing, core::ptr::null())? };
        if closing {
            break;
        }
    }

    // SAFETY: `tag` is still our fresh push, so writing its fields is writing
    // our own allocation. `push_pop` moves exactly the `num_items` tags this
    // parse stacked on xc's tmp stack into xc's result buf.
    unsafe {
        (*tag).num_children = xc.tmp_stack_view().num_items();
        (*tag).children = xc
            .result_view()
            .push_pop(xc.tmp_stack_view(), (*tag).num_children);
    }
    ufbxi_check_err!(
        xc.error_view(),
        !unsafe { (*tag).children }.is_null(),
        "tag->children"
    );

    xc.set_doc(xc.result_view().push(1));
    ufbxi_check_err!(xc.error_view(), !xc.doc().is_null(), "xc->doc");

    // SAFETY: `xc.doc()` was just checked non-null and is our fresh push, whose
    // fields we fill before any reader sees it.
    unsafe {
        (*xc.doc()).root = tag;
        (*xc.doc()).buf = xc.take_result();
    }

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
    // SAFETY: `XmlContext` wraps `MaybeUninit`, and all-zero is the state the C
    // aggregate initializer leaves the context in (every field is a scalar,
    // pointer, POD array or zeroable `Buf`).
    let xc: XmlContext = unsafe { core::mem::zeroed() };
    // SAFETY (all three reads): `opts` points at a live `XmlLoadOpts` the
    // caller owns for the duration of the call (fn raw-param contract).
    xc.set_ator(unsafe { (*opts).ator });
    xc.set_read_fn(unsafe { (*opts).read_fn });
    xc.set_read_user(unsafe { (*opts).read_user });

    xc.tmp_stack_view().set_ator(xc.ator());
    xc.result_view().set_ator(xc.ator());

    xc.result_view().set_unordered(true);

    // SAFETY: reading a scalar field of the live `opts`.
    if unsafe { (*opts).prefix_length } > 0 {
        // SAFETY: `opts` is live; `prefix`/`prefix_length` describe one caller-
        // owned run, so the offset forms an in-range one-past-the-end pointer.
        xc.set_pos(unsafe { (*opts).prefix });
        xc.set_pos_end(unsafe { (*opts).prefix.add((*opts).prefix_length) });
    } else {
        xml_refill(&xc);
    }

    let ok = xml_parse_root(&xc).is_ok();

    // SAFETY: both are xc's own state — the tmp stack buf it owns, and the
    // token run either grown from `xc.ator()` to exactly `tok_cap` bytes or
    // still `(null, 0)`, which `free` ignores.
    unsafe { buf_free(xc.tmp_stack_mut_ptr()) };
    unsafe { free::<u8>(xc.ator(), xc.tok(), xc.tok_cap()) };

    if ok {
        xc.doc()
    } else {
        // SAFETY: the result buf is xc's own owned state; on the failure path
        // nothing was handed out of it.
        unsafe { buf_free(xc.result_mut_ptr()) };
        if !error.is_null() {
            // SAFETY: `error` is the caller's live, writable `Error` slot
            // (checked non-null); the source is xc's own error field, copied
            // bitwise as C does with struct assignment.
            unsafe { core::ptr::write(error, core::ptr::read(xc.error_mut_ptr())) };
        }

        core::ptr::null_mut()
    }
}

// ufbx.c:7656-7660 `ufbxi_free_xml`
#[inline(never)]
pub(crate) unsafe fn free_xml(doc: *mut XmlDocument) {
    // Move the buf to the stack before freeing: `doc` itself is allocated from
    // this very buffer (C copies by struct assignment; `Buf` is not `Copy`).
    // SAFETY: `doc` is a live document `load_xml` returned (fn raw-param
    // contract); the bitwise read moves the owning `Buf` to the stack, and the
    // stale field dies with the storage this call frees.
    let mut buf: Buf = unsafe { core::ptr::read(&raw const (*doc).buf) };
    // SAFETY: `buf` is that live stack copy, the sole owner of the chunk list.
    unsafe { buf_free(&mut buf) };
}

// ufbx.c:7662-7670 `ufbxi_xml_find_child`
// Explicit loop (not `Iterator::find`) mirrors the C `ufbxi_for` control-flow for
// upstream line-correspondence and hosts the per-element SAFETY note.
#[allow(clippy::manual_find)]
#[inline(never)]
pub(crate) fn xml_find_child<'a>(tag: &'a XmlTagView, name: &CStr) -> Option<&'a XmlTagView> {
    // C: `ufbxi_for(ufbxi_xml_tag, child, tag->children, tag->num_children)`
    // SAFETY: `children`/`num_children` describe a contiguous arena run (built by
    // `xml_parse_tag` via `push_pop`), valid and stable for `tag`'s lifetime `'a`.
    let children: SliceViewIter<'a, XmlTag> =
        unsafe { SliceViewIter::from_raw_parts(tag.children(), tag.num_children()) };
    for child in children {
        // SAFETY: `child.name_data()` is a valid NUL-terminated arena string;
        // `name` is a valid NUL-terminated C string.
        if unsafe { strcmp(child.name_data(), name.as_ptr().cast()) } == 0 {
            return Some(child);
        }
    }
    None
}

// ufbx.c:7672-7680 `ufbxi_xml_find_attrib`
// Explicit loop (not `Iterator::find`) mirrors the C `ufbxi_for` control-flow for
// upstream line-correspondence and hosts the per-element SAFETY note.
#[allow(clippy::manual_find)]
#[inline(never)]
pub(crate) fn xml_find_attrib<'a>(tag: &'a XmlTagView, name: &CStr) -> Option<&'a XmlAttribView> {
    // C: `ufbxi_for(ufbxi_xml_attrib, attrib, tag->attribs, tag->num_attribs)`
    // SAFETY: `attribs`/`num_attribs` describe a contiguous arena run (built by
    // `xml_parse_tag` via `push_pop`), valid and stable for `tag`'s lifetime `'a`.
    let attribs: SliceViewIter<'a, XmlAttrib> =
        unsafe { SliceViewIter::from_raw_parts(tag.attribs(), tag.num_attribs()) };
    for attrib in attribs {
        // SAFETY: `attrib.name_data()` is a valid NUL-terminated arena string;
        // `name` is a valid NUL-terminated C string.
        if unsafe { strcmp(attrib.name_data(), name.as_ptr().cast()) } == 0 {
            return Some(attrib);
        }
    }
    None
}
