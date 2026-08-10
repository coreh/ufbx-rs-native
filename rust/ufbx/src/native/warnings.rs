//! Port of the `// -- Warnings` banner section (ufbx.c:4822-4894), plus the
//! `uc`-context warning macros (ufbx.c:6673-6674; C defines them next to
//! `ufbxi_context` — the Rust forms take the context as an explicit first
//! argument because `macro_rules!` hygiene cannot capture a call-site `uc`,
//! same convention as the check macros in `native/error.rs`).
//!
//! Variadic entry: `ufbxi_warnf_imp` is `(..., const char *fmt, ...)` in C;
//! the `...` / `va_list` pair collapses into a `&[PrintArg]` slice built by
//! the `ufbxi_warnf_imp!` call-site wrapper (PORTING.md "Printf and
//! variadics").
//!
//! Warning dedup order and counts are hash-oracle-observable — the
//! `prev_warnings` bookkeeping and the early-out `count++` path must match C
//! exactly.
//!
//! Phase 1: no consumers yet (`ufbxi_context` / string pool arrive later).
#![allow(dead_code, unused_macros, unused_imports)]

use crate::generated::{Error, Warning, WarningType};
use crate::native::buf::{self, Buf};
use crate::native::error::{clean_string_utf8, ufbxi_check_err, vsnprintf, Fail};
use crate::native::printf::PrintArg;
use crate::prelude::List;

// ufbx.h:3592 `UFBX_ENUM_TYPE(ufbx_warning_type, UFBX_WARNING_TYPE,
// UFBX_WARNING_UNKNOWN_OBJ_DIRECTIVE)` — `UFBX_WARNING_TYPE_COUNT` = last + 1.
pub(crate) const WARNING_TYPE_COUNT: usize = WarningType::UnknownObjDirective as usize + 1;

// ufbx.h:3587 `UFBX_WARNING_TYPE_FIRST_DEDUPLICATED = UFBX_WARNING_INDEX_CLAMPED`
// C comment (ufbx.h): Warnings after this one are deduplicated.
pub(crate) const WARNING_TYPE_FIRST_DEDUPLICATED: u32 = WarningType::IndexClamped as u32;

// Layout pin tying the hand-duplicated count to the generated
// `Metadata.has_warning: [bool; UFBX_WARNING_TYPE_COUNT]` array — an upstream
// enum change must fail to compile, not silently desync (`pop_warnings` writes
// through `p_has_warning` with raw indexing below).
const _: () = {
    fn _pin_has_warning(m: &crate::generated::Metadata) -> &[bool; WARNING_TYPE_COUNT] {
        &m.has_warning
    }
};

// ufbx.c:4824-4831 `ufbxi_warnings`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Warnings {
    pub error: *mut Error,
    pub result: *mut Buf,
    pub tmp_stack: Buf,
    pub deferred_element_id_plus_one: u32,
    // Separate lists for specific and non-specific warnings
    pub prev_warnings: [[*mut Warning; 2]; WARNING_TYPE_COUNT],
}

// Typed interior-mutable VIEW over an owned `Warnings` field, reinterpreted in place.
// `.tmp_stack` recurses into `BufView`; other leaves are setters / raw-ptr getters.
pub(crate) type WarningsView = crate::native::view::View<Warnings>;

impl WarningsView {
    #[inline(always)]
    pub(crate) fn tmp_stack_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_stack as *mut crate::native::buf::BufView) }
    }
    #[inline(always)]
    pub(crate) fn tmp_stack_mut_ptr(&self) -> *mut Buf {
        unsafe { &raw mut (*self.get()).tmp_stack }
    }
    #[inline(always)]
    pub(crate) fn set_error(&self, error: *mut Error) {
        unsafe {
            (*self.get()).error = error;
        }
    }
    #[inline(always)]
    pub(crate) fn set_result(&self, result: *mut Buf) {
        unsafe {
            (*self.get()).result = result;
        }
    }
    #[inline(always)]
    pub(crate) fn set_deferred_element_id_plus_one(&self, deferred_element_id_plus_one: u32) {
        unsafe {
            (*self.get()).deferred_element_id_plus_one = deferred_element_id_plus_one;
        }
    }
}

// ufbx.c:4833-4872 `ufbxi_vwarnf_imp`
// C: `ufbxi_nodiscard static ufbxi_noinline int` — `return 1` becomes
// `Ok(())`, the `ufbxi_check_err` failure path returns `Err(Fail)`.
#[inline(never)]
pub(crate) unsafe fn vwarnf_imp(
    ws: *mut Warnings,
    type_: WarningType,
    mut element_id: u32,
    fmt: *const u8,
    args: &[PrintArg],
) -> Result<(), Fail> {
    if ws.is_null() {
        return Ok(());
    }

    // HACK(warning-element): Encode potential deferred element ID into `ufbx_warning.element_id`,
    // `ws->element_id_index_plus_one` contains index to `uc->tmp_element_id`.
    // Tag deferred indices with the high bit.
    if element_id == !0u32 && (*ws).deferred_element_id_plus_one > 0 {
        element_id = ((*ws).deferred_element_id_plus_one - 1) | 0x80000000u32;
    }

    let has_element_id: u32 = (element_id != !0u32) as u32;
    if type_ as u32 >= WARNING_TYPE_FIRST_DEDUPLICATED {
        let prev: *mut Warning = (*ws).prev_warnings[type_ as usize][has_element_id as usize];
        if !prev.is_null() && (*prev).element_id == element_id {
            // C: `prev->count++;` (size_t)
            (*prev).count = (*prev).count.wrapping_add(1);
            return Ok(());
        }
    }

    // C: `char desc[256];` (uninitialized; `vsnprintf` writes `desc_len`
    // bytes + NUL before any read — the zero-fill is unobservable).
    let mut desc = [0u8; 256];
    let desc_len: usize =
        vsnprintf(desc.as_mut_ptr(), core::mem::size_of_val(&desc), fmt, args) as usize;

    clean_string_utf8(desc.as_mut_ptr(), desc_len);

    let desc_copy: *mut u8 = buf::push_copy::<u8>((*ws).result, desc_len + 1, desc.as_ptr());
    ufbxi_check_err!((*ws).error, !desc_copy.is_null(), "desc_copy");

    let warning: *mut Warning = buf::push::<Warning>(&mut (*ws).tmp_stack, 1);
    ufbxi_check_err!((*ws).error, !warning.is_null(), "warning");

    (*warning).type_ = type_;
    (*warning).description.data = desc_copy;
    (*warning).description.length = desc_len;
    (*warning).element_id = element_id;
    (*warning).count = 1;
    (*ws).prev_warnings[type_ as usize][has_element_id as usize] = warning;

    Ok(())
}

// ufbx.c:4874-4882 `ufbxi_warnf_imp` (variadic entry point — see
// `ufbxi_warnf_imp!` / `ufbxi_warnf!` / `ufbxi_warnf_tag!`).
#[inline(never)]
pub(crate) unsafe fn warnf_imp(
    ws: *mut Warnings,
    type_: WarningType,
    element_id: u32,
    fmt: *const u8,
    args: &[PrintArg],
) -> Result<(), Fail> {
    // NOTE: `ws` may be `NULL` here, handled by `ufbxi_vwarnf()`
    // C: `va_list args; // ufbxi_uninit` (ufbx.c:4876) — collapsed into `args`.

    vwarnf_imp(ws, type_, element_id, fmt, args)
}

// Call-site wrapper building the `&[PrintArg]` argument pack
// (PORTING.md "Printf and variadics").
macro_rules! ufbxi_warnf_imp {
    ($ws:expr, $type:expr, $element_id:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::native::warnings::warnf_imp($ws, $type, $element_id,
            concat!($fmt, "\0").as_ptr(),
            &[$($crate::native::printf::PrintArg::from($arg)),*])
    };
}
pub(crate) use ufbxi_warnf_imp;

// ufbx.c:6673 `ufbxi_warnf(type, ...)` — C hardcodes `&uc->warnings`; the
// Rust form takes the context as an explicit first argument (see module docs).
macro_rules! ufbxi_warnf {
    ($uc:expr, $type:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::native::warnings::ufbxi_warnf_imp!(
            &mut (*$uc.get()).warnings as *mut $crate::native::warnings::Warnings,
            $type, !0u32, $fmt $(, $arg)*)
    };
}
pub(crate) use ufbxi_warnf;

// ufbx.c:6674 `ufbxi_warnf_tag(type, element_id, ...)`
macro_rules! ufbxi_warnf_tag {
    ($uc:expr, $type:expr, $element_id:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::native::warnings::ufbxi_warnf_imp!(
            &mut (*$uc.get()).warnings as *mut $crate::native::warnings::Warnings,
            $type, $element_id, $fmt $(, $arg)*)
    };
}
pub(crate) use ufbxi_warnf_tag;

// ufbx.c:4884-4894 `ufbxi_pop_warnings`
#[inline(never)]
pub(crate) unsafe fn pop_warnings(
    ws: *mut Warnings,
    warnings: *mut List<Warning>,
    p_has_warning: *mut bool,
) -> Result<(), Fail> {
    (*warnings).count = (*ws).tmp_stack.num_items;
    (*warnings).data =
        buf::push_pop::<Warning>((*ws).result, &mut (*ws).tmp_stack, (*warnings).count);
    ufbxi_check_err!((*ws).error, !(*warnings).data.is_null(), "warnings->data");
    // C: `ufbxi_for_list(ufbx_warning, warning, *warnings)` (ufbx.c:1098)
    let mut warning = (*warnings).data as *mut Warning;
    let warning_end = crate::native::platform::add_ptr(warning, (*warnings).count);
    while warning != warning_end {
        *p_has_warning.add((*warning).type_ as usize) = true;
        warning = warning.add(1);
    }
    Ok(())
}

// CONTINUATION POINT: `// -- Warnings` section complete (ufbx.c:4822-4894).
// Next banner: ufbx.c:4896 `// -- String pool` (owned by native/string_pool.rs;
// its `ufbxi_warnf_imp` call at ufbx.c:5070 goes through `ufbxi_warnf_imp!`).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::allocator::{init_ator, Allocator};
    use core::mem::MaybeUninit;

    struct Fixture {
        err: Error,
        ator: Allocator,
        result: Buf,
    }

    unsafe fn make_buf(ator: *mut Allocator) -> Buf {
        let mut buf = MaybeUninit::<Buf>::zeroed().assume_init();
        buf.ator = ator;
        buf
    }

    unsafe fn make_fixture() -> Box<Fixture> {
        let mut fx: Box<Fixture> = Box::new(Fixture {
            err: Error::default(),
            ator: MaybeUninit::zeroed().assume_init(),
            result: MaybeUninit::zeroed().assume_init(),
        });
        init_ator(
            &mut fx.err,
            &mut fx.ator,
            core::ptr::null(),
            b"test\0".as_ptr(),
        );
        let ator = &mut fx.ator as *mut Allocator;
        fx.result = make_buf(ator);
        fx
    }

    unsafe fn make_warnings(fx: &mut Fixture) -> Warnings {
        Warnings {
            error: &mut fx.err,
            result: &mut fx.result,
            tmp_stack: make_buf(&mut fx.ator),
            deferred_element_id_plus_one: 0,
            prev_warnings: [[core::ptr::null_mut(); 2]; WARNING_TYPE_COUNT],
        }
    }

    unsafe fn free_all_chunks(b: &mut Buf) {
        for list_ix in 0..2 {
            let chunk = b.chunks[list_ix];
            if chunk.is_null() {
                continue;
            }
            let mut c = (*chunk).root;
            while !c.is_null() {
                let next = (*c).next;
                crate::native::allocator::free_size(
                    b.ator,
                    1,
                    c as *mut core::ffi::c_void,
                    core::mem::size_of::<crate::native::buf::BufChunk>() + (*c).size,
                );
                c = next;
            }
            b.chunks[list_ix] = core::ptr::null_mut();
        }
    }

    unsafe fn warning_slice<'a>(list: &List<Warning>) -> &'a [Warning] {
        core::slice::from_raw_parts(list.data, list.count)
    }

    fn desc(w: &Warning) -> &str {
        unsafe {
            core::str::from_utf8(core::slice::from_raw_parts(
                w.description.data,
                w.description.length,
            ))
            .unwrap()
        }
    }

    #[test]
    fn test_null_ws_is_ok() {
        unsafe {
            // NOTE: `ws` may be `NULL` — must succeed without touching anything.
            let r = ufbxi_warnf_imp!(
                core::ptr::null_mut(),
                WarningType::IndexClamped,
                !0u32,
                "Clamped index"
            );
            assert_eq!(r, Ok(()));
        }
    }

    #[test]
    fn test_format_dedup_and_pop() {
        unsafe {
            let mut fx = make_fixture();
            let mut ws = make_warnings(&mut fx);
            let ws_ptr = &mut ws as *mut Warnings;

            // Non-deduplicated warning: repeated pushes stay separate.
            assert_eq!(
                ufbxi_warnf_imp!(
                    ws_ptr,
                    WarningType::UnsupportedVersion,
                    !0u32,
                    "Unsupported FBX version (%u)",
                    6000u32
                ),
                Ok(())
            );
            assert_eq!(
                ufbxi_warnf_imp!(
                    ws_ptr,
                    WarningType::UnsupportedVersion,
                    !0u32,
                    "Unsupported FBX version (%u)",
                    6000u32
                ),
                Ok(())
            );

            // Deduplicated warning (>= FIRST_DEDUPLICATED): same element_id
            // increments count instead of pushing.
            assert_eq!(
                ufbxi_warnf_imp!(ws_ptr, WarningType::IndexClamped, !0u32, "Clamped index"),
                Ok(())
            );
            assert_eq!(
                ufbxi_warnf_imp!(ws_ptr, WarningType::IndexClamped, !0u32, "Clamped index"),
                Ok(())
            );
            assert_eq!(
                ufbxi_warnf_imp!(ws_ptr, WarningType::IndexClamped, !0u32, "Clamped index"),
                Ok(())
            );

            // Specific (element-tagged) and non-specific lists are separate:
            // a tagged IndexClamped does not merge with the untagged one.
            assert_eq!(
                ufbxi_warnf_imp!(ws_ptr, WarningType::IndexClamped, 7u32, "Clamped index"),
                Ok(())
            );
            assert_eq!(
                ufbxi_warnf_imp!(ws_ptr, WarningType::IndexClamped, 7u32, "Clamped index"),
                Ok(())
            );
            // Different element_id on the same type: new warning.
            assert_eq!(
                ufbxi_warnf_imp!(ws_ptr, WarningType::IndexClamped, 8u32, "Clamped index"),
                Ok(())
            );

            let mut list = core::mem::MaybeUninit::<List<Warning>>::zeroed().assume_init();
            let mut has_warning = [false; WARNING_TYPE_COUNT];
            assert_eq!(
                pop_warnings(ws_ptr, &mut list, has_warning.as_mut_ptr()),
                Ok(())
            );

            let warnings = warning_slice(&list);
            assert_eq!(warnings.len(), 5);

            assert_eq!(warnings[0].type_, WarningType::UnsupportedVersion);
            assert_eq!(desc(&warnings[0]), "Unsupported FBX version (6000)");
            assert_eq!(warnings[0].element_id, !0u32);
            assert_eq!(warnings[0].count, 1);
            assert_eq!(warnings[1].type_, WarningType::UnsupportedVersion);
            assert_eq!(warnings[1].count, 1);

            assert_eq!(warnings[2].type_, WarningType::IndexClamped);
            assert_eq!(desc(&warnings[2]), "Clamped index");
            assert_eq!(warnings[2].element_id, !0u32);
            assert_eq!(warnings[2].count, 3);

            assert_eq!(warnings[3].type_, WarningType::IndexClamped);
            assert_eq!(warnings[3].element_id, 7);
            assert_eq!(warnings[3].count, 2);

            assert_eq!(warnings[4].type_, WarningType::IndexClamped);
            assert_eq!(warnings[4].element_id, 8);
            assert_eq!(warnings[4].count, 1);

            for (ix, has) in has_warning.iter().enumerate() {
                let expect = ix == WarningType::UnsupportedVersion as usize
                    || ix == WarningType::IndexClamped as usize;
                assert_eq!(*has, expect, "has_warning[{}]", ix);
            }

            free_all_chunks(&mut ws.tmp_stack);
            free_all_chunks(&mut fx.result);
            assert_eq!(fx.ator.current_size, 0);
        }
    }

    #[test]
    fn test_dedup_interleave_resets_prev() {
        unsafe {
            // Dedup only merges with the IMMEDIATELY previous warning of the
            // same type/list: A A B A yields counts 2,1,1 (order preserved).
            let mut fx = make_fixture();
            let mut ws = make_warnings(&mut fx);
            let ws_ptr = &mut ws as *mut Warnings;

            assert_eq!(
                ufbxi_warnf_imp!(ws_ptr, WarningType::IndexClamped, 1u32, "Clamped index"),
                Ok(())
            );
            assert_eq!(
                ufbxi_warnf_imp!(ws_ptr, WarningType::IndexClamped, 1u32, "Clamped index"),
                Ok(())
            );
            assert_eq!(
                ufbxi_warnf_imp!(ws_ptr, WarningType::IndexClamped, 2u32, "Clamped index"),
                Ok(())
            );
            assert_eq!(
                ufbxi_warnf_imp!(ws_ptr, WarningType::IndexClamped, 1u32, "Clamped index"),
                Ok(())
            );

            let mut list = core::mem::MaybeUninit::<List<Warning>>::zeroed().assume_init();
            let mut has_warning = [false; WARNING_TYPE_COUNT];
            assert_eq!(
                pop_warnings(ws_ptr, &mut list, has_warning.as_mut_ptr()),
                Ok(())
            );

            let warnings = warning_slice(&list);
            assert_eq!(warnings.len(), 3);
            assert_eq!(warnings[0].element_id, 1);
            assert_eq!(warnings[0].count, 2);
            assert_eq!(warnings[1].element_id, 2);
            assert_eq!(warnings[1].count, 1);
            assert_eq!(warnings[2].element_id, 1);
            assert_eq!(warnings[2].count, 1);

            free_all_chunks(&mut fx.result);
        }
    }

    #[test]
    fn test_deferred_element_id() {
        unsafe {
            // HACK(warning-element): `~0u` element_id with a pending deferred
            // index gets encoded as `(index) | 0x80000000`.
            let mut fx = make_fixture();
            let mut ws = make_warnings(&mut fx);
            ws.deferred_element_id_plus_one = 5;
            let ws_ptr = &mut ws as *mut Warnings;

            assert_eq!(
                ufbxi_warnf_imp!(ws_ptr, WarningType::IndexClamped, !0u32, "Clamped index"),
                Ok(())
            );
            // An explicit element_id is NOT rewritten.
            assert_eq!(
                ufbxi_warnf_imp!(ws_ptr, WarningType::IndexClamped, 3u32, "Clamped index"),
                Ok(())
            );

            let mut list = core::mem::MaybeUninit::<List<Warning>>::zeroed().assume_init();
            let mut has_warning = [false; WARNING_TYPE_COUNT];
            assert_eq!(
                pop_warnings(ws_ptr, &mut list, has_warning.as_mut_ptr()),
                Ok(())
            );

            let warnings = warning_slice(&list);
            assert_eq!(warnings.len(), 2);
            assert_eq!(warnings[0].element_id, 4 | 0x80000000u32);
            assert_eq!(warnings[1].element_id, 3);

            free_all_chunks(&mut fx.result);
        }
    }

    #[test]
    fn test_description_truncation_and_utf8_clean() {
        unsafe {
            // desc[256]: vsnprintf truncates to 255 chars + NUL; invalid UTF-8
            // bytes are replaced with '?'.
            let mut fx = make_fixture();
            let mut ws = make_warnings(&mut fx);
            let ws_ptr = &mut ws as *mut Warnings;

            let long = [b'x'; 300];
            let mut long_z = [0u8; 301];
            long_z[..300].copy_from_slice(&long);
            assert_eq!(
                ufbxi_warnf_imp!(
                    ws_ptr,
                    WarningType::MissingExternalFile,
                    !0u32,
                    "Could not open .mtl file: %s",
                    long_z.as_ptr()
                ),
                Ok(())
            );
            assert_eq!(
                ufbxi_warnf_imp!(
                    ws_ptr,
                    WarningType::MissingExternalFile,
                    !0u32,
                    "Bad byte: %s",
                    b"a\xffb\0".as_ptr()
                ),
                Ok(())
            );

            let mut list = core::mem::MaybeUninit::<List<Warning>>::zeroed().assume_init();
            let mut has_warning = [false; WARNING_TYPE_COUNT];
            assert_eq!(
                pop_warnings(ws_ptr, &mut list, has_warning.as_mut_ptr()),
                Ok(())
            );

            let warnings = warning_slice(&list);
            assert_eq!(warnings.len(), 2);
            assert_eq!(warnings[0].description.length, 255);
            assert!(desc(&warnings[0]).starts_with("Could not open .mtl file: xxx"));
            // NUL terminator copied into the result buffer.
            assert_eq!(*warnings[0].description.data.add(255), 0);
            assert_eq!(desc(&warnings[1]), "Bad byte: a?b");

            free_all_chunks(&mut fx.result);
        }
    }

    #[test]
    fn test_allocation_failure_sets_error() {
        unsafe {
            // Zero-sized allocator limit: `ufbxi_push_copy` fails and
            // `ufbxi_check_err` records the error.
            let mut fx = make_fixture();
            fx.ator.max_size = 1;
            let mut ws = make_warnings(&mut fx);
            let ws_ptr = &mut ws as *mut Warnings;

            let r = ufbxi_warnf_imp!(
                ws_ptr,
                WarningType::UnsupportedVersion,
                !0u32,
                "Unsupported FBX version (%u)",
                6000u32
            );
            assert_eq!(r, Err(Fail));
            assert_eq!(
                core::slice::from_raw_parts(fx.err.description.data, fx.err.description.length),
                b"Memory limit exceeded"
            );
        }
    }
}
