//! Port of the `// -- Geometry caches` (ufbx.c:23946-24785) and
//! `// -- External files` (ufbx.c:24787-25010) banner sections.
//!
//! The geometry-cache half is gated on `UFBXI_FEATURE_GEOMETRY_CACHE`; C keeps
//! `ufbxi_geometry_cache_imp`, `ufbxi_load_geometry_cache` and
//! `ufbxi_free_geometry_cache_imp` defined in BOTH arms (the `#else` at
//! ufbx.c:24763), so both arms are ported here. The external-files half sits
//! outside the `#if` entirely and is always compiled; only the body of
//! `ufbxi_load_external_cache` forks.
//!
//! The XML parser this section drives lives in `native::xml`
//! (`UFBXI_FEATURE_XML`, derived from the geometry-cache feature).
#![allow(dead_code, unused_imports)]
use core::ffi::c_void;
use core::mem::{size_of, size_of_val, MaybeUninit};

use crate::generated::{
    CacheChannel, CacheDataEncoding, CacheDataFormat, CacheDeformer, CacheFile, CacheFileFormat,
    CacheFrame, CacheInterpretation, CoordinateAxes, Error, ErrorType, GeometryCache, Matrix,
    MirrorAxis, Node, OpenFileType, RawGeometryCacheOpts, RawOpenFileCb, RawStream,
    SpaceConversion, WarningType,
};
use crate::native::allocator::{
    free, free_ator, grow_array, init_ator, Allocator, CACHE_IMP_MAGIC,
};
use crate::native::api::{
    coordinate_axes_valid, matrix_determinant, matrix_mul, matrix_to_transform,
    transform_to_matrix, EMPTY_STRING,
};
use crate::native::buf::{buf_free, Buf};
use crate::native::error::{
    clear_error, fix_error_type, set_err_info, strlen, ufbxi_check, ufbxi_check_err,
    ufbxi_check_err_msg, ufbxi_fail_err, ufbxi_fail_err_msg, ufbxi_fail_msg, ufbxi_fmt_err_info,
    ufbxi_report_err_msg, ufbxi_snprintf, Fail, EMPTY_CHAR,
};
use crate::native::hash::map_init;
use crate::native::parse::{
    finish_imp, is_transform_identity, r#match, Context, InnerContext, Refcount,
};
use crate::native::platform::{
    add_ptr, macro_lower_bound_eq, min32, min64, min_sz, read_f32, read_u32, stable_sort, to_size,
    ufbx_assert, ufbxi_dev_assert, ufbxi_regression_assert, unstable_sort, MAX_SKIP_SIZE,
};
use crate::native::read::{open_file, opt_ptr, ref_ptr};
use crate::native::scene_process::{
    axis_matrix, mirror_matrix, mirror_matrix_dst, round_if_near, POW10_TARGETS,
};
use crate::native::string_pool::{
    map_cmp_string, push_string_place_str, str_cmp_raw, str_equal_raw, str_less_raw,
    string_pool_temp_free, StringPool,
};
#[cfg(feature = "geometry-cache")]
use crate::native::view::SliceViewIter;
use crate::native::warnings::ufbxi_warnf;
#[cfg(feature = "geometry-cache")]
use crate::native::xml::{
    free_xml, load_xml, xml_find_attrib, xml_find_child, XmlDocument, XmlLoadOpts, XmlTagView,
};
use crate::prelude::{Real, Ref, String};

// ufbx.c:23950-23957 `ufbxi_geometry_cache_imp` (UFBXI_FEATURE_GEOMETRY_CACHE)
#[cfg(feature = "geometry-cache")]
#[repr(C)]
pub(crate) struct GeometryCacheImp {
    pub refcount: Refcount,
    pub cache: crate::generated::GeometryCache,
    pub magic: u32,
    pub owned_by_scene: bool,

    pub string_buf: Buf,
}

// ufbx.c:23959 `ufbx_static_assert(geometry_cache_imp_offset, offsetof(ufbxi_geometry_cache_imp, cache) == sizeof(ufbxi_refcount));`
#[cfg(feature = "geometry-cache")]
const _: () =
    assert!(core::mem::offset_of!(GeometryCacheImp, cache) == core::mem::size_of::<Refcount>());

// SAFETY: `#[repr(C)]` with `refcount` leading, `CACHE_IMP_MAGIC` is the magic
// `ufbxi_get_imp(ufbxi_geometry_cache_imp, ...)` users check, `Payload` is the
// public struct at the pinned offset, and `header_parts` projects the two
// named fields of the passed `imp`. The extra `owned_by_scene` / `string_buf`
// fields trail the shared header and are stamped by the call site.
#[cfg(feature = "geometry-cache")]
unsafe impl crate::native::parse::ImpRecover for GeometryCacheImp {
    type Payload = crate::generated::GeometryCache;
    const MAGIC: u32 = CACHE_IMP_MAGIC;

    #[inline(always)]
    unsafe fn header_parts(imp: *mut Self) -> (*mut Refcount, *mut u32) {
        // SAFETY: the caller vouches `imp` addresses a live `GeometryCacheImp`,
        // so these field projections stay inside that allocation.
        unsafe { (&raw mut (*imp).refcount, &raw mut (*imp).magic) }
    }
}

// SAFETY: `parts` projects the three named fields of the passed `imp` (layout
// pinned by the `offset_of` assert above).
#[cfg(feature = "geometry-cache")]
unsafe impl crate::native::parse::ImpHeader for GeometryCacheImp {
    #[inline(always)]
    unsafe fn parts(imp: *mut Self) -> (*mut Refcount, *mut Self::Payload, *mut u32) {
        // SAFETY: the caller vouches `imp` addresses a live `GeometryCacheImp`,
        // so these field projections stay inside that allocation.
        unsafe {
            (
                &raw mut (*imp).refcount,
                &raw mut (*imp).cache,
                &raw mut (*imp).magic,
            )
        }
    }
}

// ufbx.c:23961-23970 `ufbxi_cache_tmp_channel`
#[cfg(feature = "geometry-cache")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct CacheTmpChannel {
    pub name: String,
    pub interpretation: String,
    pub sample_rate: u32,
    pub start_time: u32,
    pub end_time: u32,
    pub current_time: u32,
    pub consecutive_fails: u32,
    pub try_load: bool,
}

// Reinterpret-in-place VIEW over one `CacheTmpChannel` inside the `cc.channels`
// contiguous `push_zero` run; `SliceViewIter` walks that run in
// `cache_load_frame_files` yielding these, replacing the raw `chan.add(1)`
// pointer walk. Interior-mutable, so the loop's per-channel mutations go through
// setters.
#[cfg(feature = "geometry-cache")]
pub(crate) type CacheTmpChannelView = crate::native::view::View<CacheTmpChannel>;

#[cfg(feature = "geometry-cache")]
impl CacheTmpChannelView {
    #[inline(always)]
    pub(crate) fn try_load(&self) -> bool {
        // SAFETY: reading a `bool` field of a valid arena `CacheTmpChannel`.
        unsafe { (*self.get()).try_load }
    }
    #[inline(always)]
    pub(crate) fn set_try_load(&self, try_load: bool) {
        // SAFETY: interior-mutable write of a POD field.
        unsafe {
            (*self.get()).try_load = try_load;
        }
    }
    #[inline(always)]
    pub(crate) fn consecutive_fails(&self) -> u32 {
        // SAFETY: reading a `u32` field.
        unsafe { (*self.get()).consecutive_fails }
    }
    #[inline(always)]
    pub(crate) fn set_consecutive_fails(&self, consecutive_fails: u32) {
        // SAFETY: interior-mutable write of a POD field.
        unsafe {
            (*self.get()).consecutive_fails = consecutive_fails;
        }
    }
    #[inline(always)]
    pub(crate) fn sample_rate(&self) -> u32 {
        // SAFETY: reading a `u32` field.
        unsafe { (*self.get()).sample_rate }
    }
    #[inline(always)]
    pub(crate) fn current_time(&self) -> u32 {
        // SAFETY: reading a `u32` field.
        unsafe { (*self.get()).current_time }
    }
    #[inline(always)]
    pub(crate) fn set_current_time(&self, current_time: u32) {
        // SAFETY: interior-mutable write of a POD field.
        unsafe {
            (*self.get()).current_time = current_time;
        }
    }
    #[inline(always)]
    pub(crate) fn end_time(&self) -> u32 {
        // SAFETY: reading a `u32` field.
        unsafe { (*self.get()).end_time }
    }
}

// Reinterpret-in-place VIEW over the loaded `XmlDocument`; `cache_load_xml`
// bridges the raw `load_xml` result once and threads the anchored view into
// `cache_load_xml_imp`, whose `root()` supersedes the raw `(*doc).root` deref.
#[cfg(feature = "geometry-cache")]
pub(crate) type XmlDocumentView = crate::native::view::View<XmlDocument>;

#[cfg(feature = "geometry-cache")]
impl XmlDocumentView {
    #[inline(always)]
    pub(crate) fn root(&self) -> *mut crate::native::xml::XmlTag {
        // SAFETY: reading the `root` run pointer of a valid arena `XmlDocument`.
        unsafe { (*self.get()).root }
    }
}

// Reinterpret-in-place VIEW over one `CacheFrame` inside a channel's contiguous
// `frames` run; `SliceViewIter` walks it in `cache_setup_channels` to stamp the
// per-frame mirror axis / scale factor, replacing the raw `f.add(1)` walk.
#[cfg(feature = "geometry-cache")]
pub(crate) type CacheFrameView = crate::native::view::View<CacheFrame>;

#[cfg(feature = "geometry-cache")]
impl CacheFrameView {
    #[inline(always)]
    pub(crate) fn set_mirror_axis(&self, mirror_axis: MirrorAxis) {
        // SAFETY: interior-mutable write of a POD enum field.
        unsafe {
            (*self.get()).mirror_axis = mirror_axis;
        }
    }
    #[inline(always)]
    pub(crate) fn set_scale_factor(&self, scale_factor: Real) {
        // SAFETY: interior-mutable write of a POD field.
        unsafe {
            (*self.get()).scale_factor = scale_factor;
        }
    }
}

// ufbx.c:23972-23976 `ufbxi_cache_xml_type`
#[cfg(feature = "geometry-cache")]
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CacheXmlType {
    None = 0,
    FilePerFrame = 1,
    SingleFile = 2,
}

// ufbx.c:23978-23982 `ufbxi_cache_xml_format`
#[cfg(feature = "geometry-cache")]
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CacheXmlFormat {
    None = 0,
    Mcc = 1,
    Mcx = 2,
}

// ufbx.c:23984-24034 `ufbxi_cache_context`
#[cfg(feature = "geometry-cache")]
#[repr(C)]
pub(crate) struct InnerCacheContext {
    pub error: Error,
    pub filename: String,
    pub owned_by_scene: bool,
    pub ignore_if_not_found: bool,

    pub opts: RawGeometryCacheOpts,

    pub ator_tmp: *mut Allocator,
    pub ator_result: Allocator,

    pub result: Buf,
    pub tmp: Buf,
    pub tmp_stack: Buf,

    pub channels: *mut CacheTmpChannel,
    pub num_channels: usize,

    // Temporary array
    pub tmp_arr: *mut u8,
    pub tmp_arr_size: usize,

    pub string_pool: StringPool,

    pub open_file_cb: RawOpenFileCb,

    pub frames_per_second: f64,

    pub stream_filename: String,
    pub stream: RawStream,

    pub mc_for8: bool,

    pub xml_filename: String,
    pub xml_ticks_per_frame: u32,
    pub xml_type: CacheXmlType,
    pub xml_format: CacheXmlFormat,

    pub channel_name: String,

    pub name_buf: *mut u8,
    pub name_cap: usize,

    pub file_offset: u64,
    pub pos: *const u8,
    pub pos_end: *const u8,

    pub cache: GeometryCache,
    pub imp: *mut GeometryCacheImp,

    pub buffer: [u8; 128],
}

// Safe `&CacheContext` handle over the fields-struct `InnerCacheContext`, mirroring
// the `Context`/`InnerContext` seam in `parse.rs`. `MaybeUninit` because
// `InnerCacheContext` embeds the public `GeometryCache` (enum-bearing, so a plain
// `&InnerCacheContext` could not be formed soundly); `UnsafeCell` gives the
// interior mutability every `&CacheContext` call site relies on.
#[repr(transparent)]
#[cfg(feature = "geometry-cache")]
pub(crate) struct CacheContext(core::cell::UnsafeCell<core::mem::MaybeUninit<InnerCacheContext>>);

// Typed interior-mutable VIEW over the `opts` field, reinterpreted in place
// (approach A). Generated ABI-fixed `RawGeometryCacheOpts` plays the `Inner` role;
// `MaybeUninit` makes forming `&GeometryCacheOptsView` assert no validity — each leaf getter
// asserts only the field it reads.
#[cfg(feature = "geometry-cache")]
pub(crate) type GeometryCacheOptsView = crate::native::view::View<RawGeometryCacheOpts>;

#[cfg(feature = "geometry-cache")]
impl GeometryCacheOptsView {
    #[inline(always)]
    pub(crate) fn mirror_axis(&self) -> crate::generated::MirrorAxis {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.mirror_axis` read already makes.
        unsafe { (*self.get()).mirror_axis }
    }

    #[inline(always)]
    pub(crate) fn scale_factor(&self) -> Real {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.scale_factor` read already makes.
        unsafe { (*self.get()).scale_factor }
    }

    #[inline(always)]
    pub(crate) fn use_scale_factor(&self) -> bool {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.use_scale_factor` read already makes.
        unsafe { (*self.get()).use_scale_factor }
    }

    #[inline(always)]
    pub(crate) fn set_mirror_axis(&self, mirror_axis: crate::generated::MirrorAxis) {
        // SAFETY: interior-mutable write of a POD opts field.
        unsafe {
            (*self.get()).mirror_axis = mirror_axis;
        }
    }

    #[inline(always)]
    pub(crate) fn set_scale_factor(&self, scale_factor: Real) {
        // SAFETY: interior-mutable write of a POD opts field.
        unsafe {
            (*self.get()).scale_factor = scale_factor;
        }
    }

    #[inline(always)]
    pub(crate) fn set_use_scale_factor(&self, use_scale_factor: bool) {
        // SAFETY: interior-mutable write of a POD opts field.
        unsafe {
            (*self.get()).use_scale_factor = use_scale_factor;
        }
    }
}

// Typed interior-mutable VIEW over `CacheContext.cache` (approach A). List fields
// recurse into `ListView`; the whole-`String` field uses value getter + setter.
#[cfg(feature = "geometry-cache")]
pub(crate) type GeometryCacheView = crate::native::view::View<GeometryCache>;

#[cfg(feature = "geometry-cache")]
impl GeometryCacheView {
    #[inline(always)]
    pub(crate) fn frames_view(&self) -> &crate::prelude::ListView<crate::generated::CacheFrame> {
        // SAFETY: reinterpret the non-Copy `List` field in place as a view.
        unsafe {
            &*(&raw mut (*self.get()).frames
                as *mut crate::prelude::ListView<crate::generated::CacheFrame>)
        }
    }

    #[inline(always)]
    pub(crate) fn extra_info_view(&self) -> &crate::prelude::ListView<crate::prelude::String> {
        // SAFETY: reinterpret the non-Copy `List` field in place as a view.
        unsafe {
            &*(&raw mut (*self.get()).extra_info
                as *mut crate::prelude::ListView<crate::prelude::String>)
        }
    }

    #[inline(always)]
    pub(crate) fn channels_view(
        &self,
    ) -> &crate::prelude::ListView<crate::generated::CacheChannel> {
        // SAFETY: reinterpret the non-Copy `List` field in place as a view.
        unsafe {
            &*(&raw mut (*self.get()).channels
                as *mut crate::prelude::ListView<crate::generated::CacheChannel>)
        }
    }

    #[inline(always)]
    pub(crate) fn root_filename(&self) -> crate::prelude::String {
        unsafe { (*self.get()).root_filename }
    }

    #[inline(always)]
    pub(crate) fn set_root_filename(&self, root_filename: crate::prelude::String) {
        unsafe {
            (*self.get()).root_filename = root_filename;
        }
    }
}

#[cfg(feature = "geometry-cache")]
impl CacheContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerCacheContext {
        self.0.get().cast()
    }

    #[inline(always)]
    pub(crate) fn cache_mut_ptr(&self) -> *mut crate::generated::GeometryCache {
        unsafe { &raw mut (*self.get()).cache }
    }

    #[inline(always)]
    /// Moves the field out by bitwise read (`ptr::read`). C does this as plain
    /// struct assignment; the source field still holds the stale bits (no
    /// `Drop`), so the caller must overwrite it or treat it as moved-from.
    pub(crate) fn take_result(&self) -> crate::native::buf::Buf {
        unsafe { core::ptr::read(&raw const (*self.get()).result) }
    }

    #[inline(always)]
    pub(crate) fn set_result(&self, result: crate::native::buf::Buf) {
        unsafe {
            (*self.get()).result = result;
        }
    }

    #[inline(always)]
    pub(crate) fn ator_result(&self) -> crate::native::allocator::Allocator {
        unsafe { (*self.get()).ator_result }
    }

    // `result` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn result_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).result as *mut crate::native::buf::BufView) }
    }

    // `tmp` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp as *mut crate::native::buf::BufView) }
    }

    // `tmp_stack` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_stack_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_stack as *mut crate::native::buf::BufView) }
    }

    // `cache` — typed VIEW handle (reinterpret-in-place); accessors on `GeometryCacheView`.
    #[inline(always)]
    pub(crate) fn cache_view(&self) -> &GeometryCacheView {
        // SAFETY: repr(transparent) over the `cache` field inside this context's outer
        // UnsafeCell; shared interior-mutable view, asserts no validity.
        unsafe { &*(&raw mut (*self.get()).cache as *mut GeometryCacheView) }
    }

    // `error` — typed VIEW handle (reinterpret-in-place); accessors on `ErrorView`.
    #[inline(always)]
    pub(crate) fn error_view(&self) -> &crate::native::error::ErrorView {
        // SAFETY: repr(transparent) over the `error` field inside this context's outer
        // UnsafeCell; shared interior-mutable view, asserts no validity.
        unsafe { &*(&raw mut (*self.get()).error as *mut crate::native::error::ErrorView) }
    }

    // `channel_name`/`xml_filename` (String) — typed VIEW handles (reinterpret-in-place).
    #[inline(always)]
    pub(crate) fn channel_name_view(&self) -> &crate::prelude::StringView {
        unsafe { &*(&raw mut (*self.get()).channel_name as *mut crate::prelude::StringView) }
    }
    #[inline(always)]
    pub(crate) fn xml_filename_view(&self) -> &crate::prelude::StringView {
        unsafe { &*(&raw mut (*self.get()).xml_filename as *mut crate::prelude::StringView) }
    }
    // `open_file_cb` (RawOpenFileCb) — typed VIEW handle; `.fn_` is read + written.
    #[inline(always)]
    pub(crate) fn open_file_cb_view(&self) -> &crate::prelude::RawOpenFileCbView {
        unsafe { &*(&raw mut (*self.get()).open_file_cb as *mut crate::prelude::RawOpenFileCbView) }
    }
    // `open_file_cb` — raw-ptr getter (const address for `*const RawOpenFileCb` params).
    #[inline(always)]
    pub(crate) fn open_file_cb_ptr(&self) -> *const crate::generated::RawOpenFileCb {
        unsafe { &raw const (*self.get()).open_file_cb }
    }
    // `stream` (RawStream) — typed VIEW handle (reinterpret-in-place); callback leaves read-only.
    #[inline(always)]
    pub(crate) fn stream_view(&self) -> &crate::prelude::RawStreamView {
        unsafe { &*(&raw mut (*self.get()).stream as *mut crate::prelude::RawStreamView) }
    }
    // `buffer` (`[u8; 128]`) — whole-array raw-ptr getters + byte size (mirrors `sizeof`).
    #[inline(always)]
    pub(crate) fn buffer_ptr(&self) -> *const u8 {
        unsafe { (&raw mut (*self.get()).buffer) as *const u8 }
    }
    #[inline(always)]
    pub(crate) fn buffer_mut_ptr(&self) -> *mut u8 {
        unsafe { (&raw mut (*self.get()).buffer) as *mut u8 }
    }
    #[inline(always)]
    pub(crate) fn buffer_size(&self) -> usize {
        unsafe { core::mem::size_of_val(&(*self.get()).buffer) }
    }
    // `stream_filename` (String) / `xml_type`/`xml_format` (Copy enums) — value getter/setter.
    #[inline(always)]
    pub(crate) fn stream_filename(&self) -> String {
        unsafe { (*self.get()).stream_filename }
    }
    #[inline(always)]
    pub(crate) fn set_stream_filename(&self, stream_filename: String) {
        unsafe {
            (*self.get()).stream_filename = stream_filename;
        }
    }
    #[inline(always)]
    pub(crate) fn xml_type(&self) -> CacheXmlType {
        unsafe { (*self.get()).xml_type }
    }
    #[inline(always)]
    pub(crate) fn set_xml_type(&self, xml_type: CacheXmlType) {
        unsafe {
            (*self.get()).xml_type = xml_type;
        }
    }
    #[inline(always)]
    pub(crate) fn xml_format(&self) -> CacheXmlFormat {
        unsafe { (*self.get()).xml_format }
    }
    #[inline(always)]
    pub(crate) fn set_xml_format(&self, xml_format: CacheXmlFormat) {
        unsafe {
            (*self.get()).xml_format = xml_format;
        }
    }
    // Whole-field accessors for fields that also expose a `_view` (subfield access):
    // `channel_name` read, `xml_filename` write (String), `open_file_cb` write (Copy),
    // `opts` write (RawGeometryCacheOpts is non-Copy — the setter moves the value in).
    #[inline(always)]
    pub(crate) fn channel_name(&self) -> String {
        unsafe { (*self.get()).channel_name }
    }
    #[inline(always)]
    pub(crate) fn set_xml_filename(&self, xml_filename: String) {
        unsafe {
            (*self.get()).xml_filename = xml_filename;
        }
    }
    #[inline(always)]
    pub(crate) fn set_open_file_cb(&self, open_file_cb: crate::generated::RawOpenFileCb) {
        unsafe {
            (*self.get()).open_file_cb = open_file_cb;
        }
    }
    #[inline(always)]
    pub(crate) fn set_opts(&self, opts: crate::generated::RawGeometryCacheOpts) {
        unsafe {
            (*self.get()).opts = opts;
        }
    }

    // `opts` — typed VIEW handle (reinterpret-in-place); leaf accessors on `GeometryCacheOptsView`.
    #[inline(always)]
    pub(crate) fn opts_view(&self) -> &GeometryCacheOptsView {
        // SAFETY: `GeometryCacheOptsView` is repr(transparent) over the `opts` field's layout,
        // which lives in this context's outer UnsafeCell; a shared interior-mutable
        // `&GeometryCacheOptsView` is sound and asserts no validity.
        unsafe { &*(&raw mut (*self.get()).opts as *mut GeometryCacheOptsView) }
    }

    // `tmp_stack` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_stack_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_stack }
    }

    // `tmp_arr_size` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_arr_size_mut_ptr(&self) -> *mut usize {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_arr_size }
    }

    // `tmp_arr` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_arr_mut_ptr(&self) -> *mut *mut u8 {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_arr }
    }

    // `tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp }
    }

    // `string_pool` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn string_pool_mut_ptr(&self) -> *mut StringPool {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).string_pool }
    }
    // `string_pool` (StringPool) — typed VIEW handle (reinterpret-in-place) for nested
    // access; whole value getter/setter for the faithful C struct-copy sites.
    #[inline(always)]
    pub(crate) fn string_pool_view(&self) -> &crate::native::string_pool::StringPoolView {
        unsafe {
            &*(&raw mut (*self.get()).string_pool
                as *mut crate::native::string_pool::StringPoolView)
        }
    }
    #[inline(always)]
    /// Moves the field out by bitwise read (`ptr::read`). C does this as plain
    /// struct assignment; the source field still holds the stale bits (no
    /// `Drop`), so the caller must overwrite it or treat it as moved-from.
    pub(crate) fn take_string_pool(&self) -> StringPool {
        unsafe { core::ptr::read(&raw const (*self.get()).string_pool) }
    }
    #[inline(always)]
    pub(crate) fn set_string_pool(&self, string_pool: StringPool) {
        unsafe {
            (*self.get()).string_pool = string_pool;
        }
    }

    // `stream_filename` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn stream_filename_mut_ptr(&self) -> *mut String {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).stream_filename }
    }

    // `stream` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn stream_mut_ptr(&self) -> *mut RawStream {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).stream }
    }

    // `result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn result_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).result }
    }

    // `name_cap` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn name_cap_mut_ptr(&self) -> *mut usize {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).name_cap }
    }

    // `name_buf` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn name_buf_mut_ptr(&self) -> *mut *mut u8 {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).name_buf }
    }

    // `error` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn error_mut_ptr(&self) -> *mut Error {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).error }
    }

    // `channel_name` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn channel_name_mut_ptr(&self) -> *mut String {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).channel_name }
    }

    // `ator_result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_result_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_result }
    }

    // `imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn imp(&self) -> *mut GeometryCacheImp {
        // SAFETY: reading a scalar field; all bit patterns of `*mut GeometryCacheImp` are valid.
        unsafe { (*self.get()).imp }
    }

    #[inline(always)]
    pub(crate) fn set_imp(&self, imp: *mut GeometryCacheImp) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).imp = imp;
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

    // `file_offset` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn file_offset(&self) -> u64 {
        // SAFETY: reading a scalar field; all bit patterns of `u64` are valid.
        unsafe { (*self.get()).file_offset }
    }

    #[inline(always)]
    pub(crate) fn set_file_offset(&self, file_offset: u64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).file_offset = file_offset;
        }
    }

    // `name_cap` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn name_cap(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).name_cap }
    }

    // `name_buf` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn name_buf(&self) -> *mut u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u8` are valid.
        unsafe { (*self.get()).name_buf }
    }

    // `xml_ticks_per_frame` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn xml_ticks_per_frame(&self) -> u32 {
        // SAFETY: reading a scalar field; all bit patterns of `u32` are valid.
        unsafe { (*self.get()).xml_ticks_per_frame }
    }

    #[inline(always)]
    pub(crate) fn set_xml_ticks_per_frame(&self, xml_ticks_per_frame: u32) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).xml_ticks_per_frame = xml_ticks_per_frame;
        }
    }

    // `mc_for8` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn mc_for8(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).mc_for8 }
    }

    #[inline(always)]
    pub(crate) fn set_mc_for8(&self, mc_for8: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).mc_for8 = mc_for8;
        }
    }

    // `frames_per_second` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn frames_per_second(&self) -> f64 {
        // SAFETY: reading a scalar field; all bit patterns of `f64` are valid.
        unsafe { (*self.get()).frames_per_second }
    }

    #[inline(always)]
    pub(crate) fn set_frames_per_second(&self, frames_per_second: f64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).frames_per_second = frames_per_second;
        }
    }

    // `tmp_arr_size` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_arr_size(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).tmp_arr_size }
    }

    // `tmp_arr` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_arr(&self) -> *mut u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u8` are valid.
        unsafe { (*self.get()).tmp_arr }
    }

    // `num_channels` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn num_channels(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).num_channels }
    }

    #[inline(always)]
    pub(crate) fn set_num_channels(&self, num_channels: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).num_channels = num_channels;
        }
    }

    // `channels` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn channels(&self) -> *mut CacheTmpChannel {
        // SAFETY: reading a scalar field; all bit patterns of `*mut CacheTmpChannel` are valid.
        unsafe { (*self.get()).channels }
    }

    #[inline(always)]
    pub(crate) fn set_channels(&self, channels: *mut CacheTmpChannel) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).channels = channels;
        }
    }

    // `ator_tmp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn ator_tmp(&self) -> *mut Allocator {
        // SAFETY: reading a scalar field; all bit patterns of `*mut Allocator` are valid.
        unsafe { (*self.get()).ator_tmp }
    }

    #[inline(always)]
    pub(crate) fn set_ator_tmp(&self, ator_tmp: *mut Allocator) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).ator_tmp = ator_tmp;
        }
    }

    // `owned_by_scene` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn owned_by_scene(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).owned_by_scene }
    }

    #[inline(always)]
    pub(crate) fn set_owned_by_scene(&self, owned_by_scene: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).owned_by_scene = owned_by_scene;
        }
    }
}

// ufbx.c:24036-24078 `ufbxi_cache_read`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_read(
    cc: &CacheContext,
    dst: *mut c_void,
    mut size: usize,
    allow_eof: bool,
) -> Result<(), Fail> {
    let mut dst: *mut c_void = dst;
    // SAFETY: `pos` and `pos_end` bracket the same read buffer — `pos_end` is
    // derived from `pos`'s allocation and never precedes it — so they are two
    // pointers into one object, which is what `offset_from` requires.
    let buffered: usize = min_sz(to_size(unsafe { cc.pos_end().offset_from(cc.pos()) }), size);
    // SAFETY: `buffered` is bounded by `pos_end - pos` readable bytes and by
    // `size`, the caller's guarantee of writable bytes at `dst`; the read
    // buffer and the caller's destination are distinct objects.
    unsafe { core::ptr::copy_nonoverlapping(cc.pos(), dst as *mut u8, buffered) };
    // SAFETY: `buffered <= pos_end - pos`, so the advanced `pos` lands at or
    // before the one-past-the-end `pos_end`.
    cc.set_pos(unsafe { cc.pos().add(buffered) });
    size -= buffered;
    cc.set_file_offset(cc.file_offset().wrapping_add(buffered as u64));
    if size == 0 {
        return Ok(());
    }
    // SAFETY: `buffered` bytes of the caller's `size`-byte destination are
    // consumed, so `dst + buffered` is at most one past its end.
    dst = unsafe { (dst as *mut u8).add(buffered) } as *mut c_void;

    if size >= cc.buffer_size() {
        // SAFETY: the stream's `read_fn` is non-null for the whole lifetime of
        // an opened `cc.stream` (C: every `ufbx_stream` handed to the cache
        // reader has one); calling it is the C-callback contract, with `dst`
        // writable for the `size` bytes the caller guarantees.
        let num_read: usize = unsafe {
            (cc.stream_view().read_fn().unwrap_unchecked())(cc.stream_view().user(), dst, size)
        };
        ufbxi_check_err_msg!(
            cc.error_view(),
            num_read <= size,
            "IO error",
            "num_read <= size"
        );
        if !allow_eof {
            ufbxi_check_err_msg!(
                cc.error_view(),
                num_read == size,
                "Truncated file",
                "num_read == size"
            );
        }
        cc.set_file_offset(cc.file_offset().wrapping_add(num_read as u64));
        size -= num_read;
        // SAFETY: `num_read <= size` (checked just above), so the advance stays
        // within the caller's `size`-byte destination.
        dst = unsafe { (dst as *mut u8).add(num_read) } as *mut c_void;
    } else {
        // SAFETY: the stream's `read_fn` is non-null for the whole lifetime of
        // an opened `cc.stream`; the destination is `cc`'s own buffer, read
        // with exactly its own `buffer_size`.
        let num_read: usize = unsafe {
            (cc.stream_view().read_fn().unwrap_unchecked())(
                cc.stream_view().user(),
                cc.buffer_mut_ptr() as *mut c_void,
                cc.buffer_size(),
            )
        };
        ufbxi_check_err_msg!(
            cc.error_view(),
            num_read <= cc.buffer_size(),
            "IO error",
            "num_read <= sizeof(cc->buffer)"
        );
        if !allow_eof {
            ufbxi_check_err_msg!(
                cc.error_view(),
                num_read >= size,
                "Truncated file",
                "num_read >= size"
            );
        }
        cc.set_pos(cc.buffer_ptr());
        // SAFETY: `buffer_size` is the length of `cc`'s own buffer, so this is
        // its one-past-the-end pointer.
        cc.set_pos_end(unsafe { cc.buffer_ptr().add(cc.buffer_size()) });

        // SAFETY: this arm runs with `size < cc.buffer_size()`, so `size` bytes
        // are readable from the freshly refilled buffer at `pos` and writable
        // at `dst` (the caller's remaining destination); buffer and
        // destination are distinct objects.
        unsafe { core::ptr::copy_nonoverlapping(cc.pos(), dst as *mut u8, size) };
        // SAFETY: `size < cc.buffer_size()`, so the advanced `pos` stays inside
        // the buffer `pos_end` bounds.
        cc.set_pos(unsafe { cc.pos().add(size) });
        cc.set_file_offset(cc.file_offset().wrapping_add(size as u64));

        let num_written: usize = min_sz(size, num_read);
        size -= num_written;
        // SAFETY: `num_written <= size`, the caller's remaining writable bytes.
        dst = unsafe { (dst as *mut u8).add(num_written) } as *mut c_void;
    }

    if size > 0 {
        // SAFETY: `size` tracks the caller's still-unwritten destination bytes,
        // every advance of `dst` above having been subtracted from it.
        unsafe { core::ptr::write_bytes(dst as *mut u8, 0, size) };
    }

    Ok(())
}

// ufbx.c:24080-24116 `ufbxi_cache_skip`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) fn cache_skip(cc: &CacheContext, mut size: u64) -> Result<(), Fail> {
    // C-parity: `cc->file_offset += size;` is uint64_t addition, which wraps.
    // `ufbxi_cache_load_pc2` (ufbx.c:24270) passes `total_points * 12 - 1`
    // guarded only by `total_points < UINT64_MAX / 12` (ufbx.c:24262), so a
    // crafted PC2 header reaches this with `size` near `UINT64_MAX`.
    cc.set_file_offset(cc.file_offset().wrapping_add(size));

    // SAFETY: `pos..pos_end` is cc's buffered read window (one allocation, cc
    // construction invariant); `buffered` is clamped to what remains, so the
    // advance stays in bounds.
    let buffered: u64 = unsafe { min64(cc.pos_end().offset_from(cc.pos()) as u64, size) };
    unsafe { cc.set_pos(cc.pos().add(buffered as usize)) };
    size -= buffered;

    if cc.stream_view().skip_fn().is_some() {
        while size >= MAX_SKIP_SIZE as u64 {
            size -= MAX_SKIP_SIZE as u64;
            ufbxi_check_err_msg!(
                cc.error_view(),
                // SAFETY: `skip_fn` is Some (checked by the enclosing branch);
                // invoking a stream callback with its paired `user` pointer is
                // the C stream contract.
                unsafe {
                    (cc.stream_view().skip_fn().unwrap_unchecked())(
                        cc.stream_view().user(),
                        MAX_SKIP_SIZE - 1,
                    )
                },
                "Truncated file",
                "cc->stream.skip_fn(cc->stream.user, UFBXI_MAX_SKIP_SIZE - 1)"
            );

            // Check that we can read at least one byte in case the file is broken
            // and causes us to seek indefinitely forwards as `fseek()` does not
            // report if we hit EOF...
            let mut single_byte = MaybeUninit::<[u8; 1]>::uninit(); // ufbxi_uninit
                                                                    // SAFETY: an open cache stream always has `read_fn` (stream-open
                                                                    // invariant); local 1-byte buffer; C stream-callback contract.
            let num_read: usize = unsafe {
                (cc.stream_view().read_fn().unwrap_unchecked())(
                    cc.stream_view().user(),
                    single_byte.as_mut_ptr() as *mut c_void,
                    1,
                )
            };
            ufbxi_check_err_msg!(cc.error_view(), num_read <= 1, "IO error", "num_read <= 1");
            ufbxi_check_err_msg!(
                cc.error_view(),
                num_read == 1,
                "Truncated file",
                "num_read == 1"
            );
        }

        if size > 0 {
            ufbxi_check_err_msg!(
                cc.error_view(),
                // SAFETY: `skip_fn` is Some (enclosing branch); C stream
                // callback contract as above.
                unsafe {
                    (cc.stream_view().skip_fn().unwrap_unchecked())(
                        cc.stream_view().user(),
                        size as usize,
                    )
                },
                "Truncated file",
                "cc->stream.skip_fn(cc->stream.user, (size_t)size)"
            );
        }
    } else {
        let mut skip_buf = MaybeUninit::<[u8; 2048]>::uninit(); // ufbxi_uninit
        while size > 0 {
            let to_skip: usize = min64(size, size_of::<[u8; 2048]>() as u64) as usize;
            size -= to_skip as u64;
            ufbxi_check_err_msg!(
                cc.error_view(),
                // SAFETY: an open cache stream always has `read_fn`
                // (stream-open invariant); local 2048-byte buffer with
                // `to_skip` clamped to its size; C stream-callback contract.
                unsafe {
                    (cc.stream_view().read_fn().unwrap_unchecked())(
                        cc.stream_view().user(),
                        skip_buf.as_mut_ptr() as *mut c_void,
                        to_skip,
                    )
                } != 0,
                "Truncated file",
                "cc->stream.read_fn(cc->stream.user, skip_buf, to_skip)"
            );
        }
    }

    Ok(())
}

// ufbx.c:24118 `#define ufbxi_cache_mc_tag(a,b,c,d)`
#[cfg(feature = "geometry-cache")]
#[inline(always)]
pub(crate) const fn cache_mc_tag(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) << 24u32 | (b as u32) << 16 | (c as u32) << 8u32 | (d as u32)
}

// ufbx.c:24120-24129 `ufbxi_cache_mc_read_tag`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
// Safe fn: the out-param is an unaliased caller local, spelled `&mut u32`
// (the panic-param policy applied to the cache readers).
pub(crate) fn cache_mc_read_tag(cc: &CacheContext, p_tag: &mut u32) -> Result<(), Fail> {
    let mut buf = MaybeUninit::<[u8; 4]>::uninit(); // ufbxi_uninit
    let buf: *mut u8 = buf.as_mut_ptr() as *mut u8;
    // SAFETY: `buf` is a local 4-byte buffer; `cache_read` writes exactly the
    // 4 bytes it is asked for before Ok, so the byte reads below are of
    // initialized memory.
    unsafe {
        cache_read(cc, buf as *mut c_void, 4, true)?;
        *p_tag = (*buf.add(0) as u32) << 24u32
            | (*buf.add(1) as u32) << 16
            | (*buf.add(2) as u32) << 8u32
            | (*buf.add(3) as u32);
    }
    if *p_tag == cache_mc_tag(b'F', b'O', b'R', b'8') {
        cc.set_mc_for8(true);
    }
    Ok(())
}

// ufbx.c:24131-24140 `ufbxi_cache_mc_read_u32`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) fn cache_mc_read_u32(cc: &CacheContext, p_value: &mut u32) -> Result<(), Fail> {
    let mut buf = MaybeUninit::<[u8; 4]>::uninit(); // ufbxi_uninit
    let buf: *mut u8 = buf.as_mut_ptr() as *mut u8;
    // SAFETY: local 4-byte buffer, fully written by `cache_read` before Ok.
    unsafe {
        cache_read(cc, buf as *mut c_void, 4, false)?;
        *p_value = (*buf.add(0) as u32) << 24u32
            | (*buf.add(1) as u32) << 16
            | (*buf.add(2) as u32) << 8u32
            | (*buf.add(3) as u32);
        if cc.mc_for8() {
            cache_read(cc, buf as *mut c_void, 4, false)?;
        }
    }
    Ok(())
}

// ufbx.c:24142-24156 `ufbxi_cache_mc_read_u64`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) fn cache_mc_read_u64(cc: &CacheContext, p_value: &mut u64) -> Result<(), Fail> {
    if !cc.mc_for8() {
        let mut v32: u32 = 0; // C: ufbxi_uninit (fully written before the read)
        cache_mc_read_u32(cc, &mut v32)?;
        *p_value = v32 as u64;
    } else {
        let mut buf = MaybeUninit::<[u8; 8]>::uninit(); // ufbxi_uninit
        let buf: *mut u8 = buf.as_mut_ptr() as *mut u8;
        // SAFETY: local 8-byte buffer, fully written by `cache_read` before Ok.
        unsafe {
            cache_read(cc, buf as *mut c_void, 8, false)?;
            let hi: u32 = (*buf.add(0) as u32) << 24u32
                | (*buf.add(1) as u32) << 16
                | (*buf.add(2) as u32) << 8u32
                | (*buf.add(3) as u32);
            let lo: u32 = (*buf.add(4) as u32) << 24u32
                | (*buf.add(5) as u32) << 16
                | (*buf.add(6) as u32) << 8u32
                | (*buf.add(7) as u32);
            *p_value = (hi as u64) << 32u32 | (lo as u64);
        }
    }
    Ok(())
}

// ufbx.c:24158-24160 `ufbxi_cache_data_format_size`
#[cfg(feature = "geometry-cache")]
static CACHE_DATA_FORMAT_SIZE: [u8; 5] = [0, 4, 12, 8, 24];

// ufbx.c:24162-24243 `ufbxi_cache_load_mc`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) fn cache_load_mc(cc: &CacheContext) -> Result<(), Fail> {
    const TAG_CACH: u32 = cache_mc_tag(b'C', b'A', b'C', b'H');
    const TAG_MYCH: u32 = cache_mc_tag(b'M', b'Y', b'C', b'H');
    const TAG_FOR4: u32 = cache_mc_tag(b'F', b'O', b'R', b'4');
    const TAG_FOR8: u32 = cache_mc_tag(b'F', b'O', b'R', b'8');
    const TAG_VRSN: u32 = cache_mc_tag(b'V', b'R', b'S', b'N');
    const TAG_STIM: u32 = cache_mc_tag(b'S', b'T', b'I', b'M');
    const TAG_ETIM: u32 = cache_mc_tag(b'E', b'T', b'I', b'M');
    const TAG_TIME: u32 = cache_mc_tag(b'T', b'I', b'M', b'E');
    const TAG_CHNM: u32 = cache_mc_tag(b'C', b'H', b'N', b'M');
    const TAG_SIZE: u32 = cache_mc_tag(b'S', b'I', b'Z', b'E');
    const TAG_FVCA: u32 = cache_mc_tag(b'F', b'V', b'C', b'A');
    const TAG_DVCA: u32 = cache_mc_tag(b'D', b'V', b'C', b'A');
    const TAG_FBCA: u32 = cache_mc_tag(b'F', b'B', b'C', b'A');
    const TAG_DBCA: u32 = cache_mc_tag(b'D', b'B', b'C', b'A');
    const TAG_DBLA: u32 = cache_mc_tag(b'D', b'B', b'L', b'A');

    let mut version: u32 = 0;
    let mut time_start: u32 = 0;
    let mut time_end: u32 = 0;
    let mut count: u32 = 0;
    let mut time: u32 = 0;
    let mut skip_buf = MaybeUninit::<[u8; 8]>::uninit(); // ufbxi_uninit

    loop {
        let mut tag: u32 = 0; // C: ufbxi_uninit (written before every read)
        let mut size: u64 = 0; // C: ufbxi_uninit (written before every read)
        cache_mc_read_tag(cc, &mut tag)?;
        if tag == 0 {
            break;
        }

        if tag == TAG_CACH || tag == TAG_MYCH {
            continue;
        }
        if cc.mc_for8() {
            // SAFETY: local 8-byte skip buffer, 4 bytes requested.
            unsafe { cache_read(cc, skip_buf.as_mut_ptr() as *mut c_void, 4, false)? };
        }

        cache_mc_read_u64(cc, &mut size)?;
        let begin: u64 = cc.file_offset();

        let alignment: usize = if cc.mc_for8() { 8 } else { 4 };

        let mut format: CacheDataFormat = CacheDataFormat::Unknown;
        match tag {
            TAG_FOR4 => cc.set_mc_for8(false),
            TAG_FOR8 => cc.set_mc_for8(true),
            TAG_VRSN => cache_mc_read_u32(cc, &mut version)?,
            TAG_STIM => {
                cache_mc_read_u32(cc, &mut time_start)?;
                time = time_start;
            }
            TAG_ETIM => cache_mc_read_u32(cc, &mut time_end)?,
            TAG_TIME => cache_mc_read_u32(cc, &mut time)?,
            TAG_CHNM => {
                ufbxi_check_err!(
                    cc.error_view(),
                    size > 0 && size < usize::MAX as u64,
                    "size > 0 && size < SIZE_MAX"
                );
                let length: usize = size as usize - 1;
                let padded_length: usize =
                    (size as usize).wrapping_add(alignment).wrapping_sub(1) & !(alignment - 1);
                ufbxi_check_err!(
                    cc.error_view(),
                    // SAFETY: growing cc's own paired `name_buf`/`name_cap`
                    // growth state through its temp allocator (cc construction
                    // invariant).
                    unsafe {
                        grow_array::<u8>(
                            cc.ator_tmp(),
                            cc.name_buf_mut_ptr(),
                            cc.name_cap_mut_ptr(),
                            padded_length
                        )
                    },
                    "ufbxi_grow_array_size((cc->ator_tmp), sizeof(**(&cc->name_buf)), (&cc->name_buf), (&cc->name_cap), (padded_length))"
                );
                // SAFETY: `name_buf` was just grown to `padded_length`; the
                // string-pool intern reads cc's own channel-name storage
                // through its raw-ptr getters (cc construction invariant).
                unsafe {
                    cache_read(cc, cc.name_buf() as *mut c_void, padded_length, false)?;
                    cc.channel_name_view().set_data(cc.name_buf());
                    cc.channel_name_view().set_length(length);
                    push_string_place_str(
                        cc.string_pool_mut_ptr(),
                        cc.channel_name_mut_ptr(),
                        false,
                    )?;
                }
            }
            TAG_SIZE => cache_mc_read_u32(cc, &mut count)?,
            TAG_FVCA => format = CacheDataFormat::Vec3Float,
            TAG_DVCA => format = CacheDataFormat::Vec3Double,
            TAG_FBCA => format = CacheDataFormat::RealFloat,
            TAG_DBCA => format = CacheDataFormat::RealDouble,
            TAG_DBLA => format = CacheDataFormat::RealDouble,
            _ => ufbxi_fail_err!(cc.error_view(), "Unknown tag"),
        }

        if format != CacheDataFormat::Unknown {
            let frame: *mut CacheFrame = cc.tmp_stack_view().push_zero(1);
            ufbxi_check_err!(cc.error_view(), !frame.is_null(), "frame");

            let elem_size: u32 = CACHE_DATA_FORMAT_SIZE[format as u32 as usize] as u32;
            let total_size: u64 = elem_size as u64 * count as u64;
            // C: `size >= elem_size * count` — `uint32_t * uint32_t` wraps mod
            // 2^32 BEFORE the comparison widens it to `uint64_t`.
            ufbxi_check_err!(
                cc.error_view(),
                size >= elem_size.wrapping_mul(count) as u64,
                "size >= elem_size * count"
            );

            // SAFETY: `frame` is the fresh non-null result of the push above.
            unsafe {
                (*frame).channel = cc.channel_name();
                (*frame).time = time as f64 * (1.0 / 6000.0);
                (*frame).filename = cc.stream_filename();
                (*frame).data_format = format;
                (*frame).data_encoding = CacheDataEncoding::BigEndian;
                (*frame).data_offset = cc.file_offset();
                (*frame).data_count = count;
                (*frame).data_element_bytes = elem_size;
                (*frame).data_total_bytes = total_size;
                (*frame).file_format = CacheFileFormat::Mc;
            }

            let end: u64 = begin.wrapping_add(
                size.wrapping_add(alignment as u64).wrapping_sub(1) & !((alignment - 1) as u64),
            );
            ufbxi_check_err!(
                cc.error_view(),
                end >= cc.file_offset(),
                "end >= cc->file_offset"
            );
            let left: u64 = end - cc.file_offset();
            cache_skip(cc, left)?;
        }
    }

    Ok(())
}

// ufbx.c:24245-24292 `ufbxi_cache_load_pc2`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) fn cache_load_pc2(cc: &CacheContext) -> Result<(), Fail> {
    let mut header = MaybeUninit::<[u8; 32]>::uninit(); // ufbxi_uninit
    let header: *mut u8 = header.as_mut_ptr() as *mut u8;
    // SAFETY: local 32-byte header buffer, fully written by `cache_read`
    // before Ok; the field reads below stay within its 32 bytes.
    let (version, num_points, start_frame, frames_per_sample, num_samples) = unsafe {
        cache_read(cc, header as *mut c_void, size_of::<[u8; 32]>(), false)?;
        (
            read_u32(header.add(12)),
            read_u32(header.add(16)),
            read_f32(header.add(20)) as f64,
            read_f32(header.add(24)) as f64,
            read_u32(header.add(28)),
        )
    };

    let _ = version;

    let frames: *mut CacheFrame = cc.tmp_stack_view().push_zero(num_samples as usize);
    ufbxi_check_err!(cc.error_view(), !frames.is_null(), "frames");

    let total_points: u64 = num_points as u64 * num_samples as u64;
    ufbxi_check_err!(
        cc.error_view(),
        total_points < u64::MAX / 12,
        "total_points < UINT64_MAX / 12"
    );

    let mut offset: u64 = cc.file_offset();

    // Skip almost to the end of the data and try to read one byte as there's
    // nothing after the data so we can't detect EOF..
    if total_points > 0 {
        let mut last_byte = MaybeUninit::<[u8; 1]>::uninit(); // ufbxi_uninit
        cache_skip(cc, total_points * 12 - 1)?;
        // SAFETY: local 1-byte buffer.
        unsafe { cache_read(cc, last_byte.as_mut_ptr() as *mut c_void, 1, false)? };
    }

    let mut i: u32 = 0;
    while i < num_samples {
        // SAFETY: `i < num_samples`, indexing the fresh non-null
        // `num_samples`-element push result above.
        let frame: *mut CacheFrame = unsafe { frames.add(i as usize) };

        let sample_frame: f64 = start_frame + i as f64 * frames_per_sample;
        // SAFETY: `frame` is in bounds of the fresh push result (above).
        unsafe {
            (*frame).channel = cc.channel_name();
            (*frame).time = sample_frame / cc.frames_per_second();
            (*frame).filename = cc.stream_filename();
            (*frame).data_format = CacheDataFormat::Vec3Float;
            (*frame).data_encoding = CacheDataEncoding::LittleEndian;
            (*frame).data_offset = offset;
            (*frame).data_count = num_points;
            (*frame).data_element_bytes = 12;
            // C: `num_points * 12` is `uint32_t` arithmetic (wraps mod 2^32)
            // that is only then widened to `uint64_t`.
            (*frame).data_total_bytes = num_points.wrapping_mul(12) as u64;
            (*frame).file_format = CacheFileFormat::Pc2;
        }
        offset = offset.wrapping_add(num_points.wrapping_mul(12) as u64);
        i += 1;
    }

    Ok(())
}

// ufbx.c:24294-24299 `ufbxi_tmp_channel_less`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe extern "C" fn tmp_channel_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    let _ = user;
    let a: *const CacheTmpChannel = va as *const CacheTmpChannel;
    let b: *const CacheTmpChannel = vb as *const CacheTmpChannel;
    // SAFETY: the sort's comparator contract is that `va`/`vb` address live
    // elements of the array being sorted, which `cache_sort_tmp_channels`
    // instantiates with `CacheTmpChannel`; `str_less` in turn requires two
    // valid `String` runs, which those elements' `name` fields are.
    unsafe { str_less_raw((*a).name, (*b).name) }
}

// ufbx.c:24301-24306 `ufbxi_cache_sort_tmp_channels`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_sort_tmp_channels(
    cc: &CacheContext,
    channels: *mut CacheTmpChannel,
    count: usize,
) -> Result<(), Fail> {
    // SAFETY: the growth targets are `cc`'s own `tmp_arr` pointer/size pair,
    // grown through `cc`'s own temp allocator — the pairing `grow_array`
    // requires. The verbatim C condition text is supplied, so wrapping the
    // condition does not perturb the recorded error string.
    ufbxi_check_err!(
        cc.error_view(),
        unsafe {
            grow_array::<u8>(
                cc.ator_tmp(),
                cc.tmp_arr_mut_ptr(),
                cc.tmp_arr_size_mut_ptr(),
                count * size_of::<CacheTmpChannel>()
            )
        },
        "ufbxi_grow_array_size((cc->ator_tmp), sizeof(**(&cc->tmp_arr)), (&cc->tmp_arr), (&cc->tmp_arr_size), (count * sizeof(ufbxi_cache_tmp_channel)))"
    );
    // SAFETY: the caller's contract is that `channels` addresses `count` live
    // `CacheTmpChannel`s; the scratch buffer is `cc.tmp_arr`, just grown to
    // `count * size_of::<CacheTmpChannel>()` bytes, and the element size /
    // comparator match that type.
    unsafe {
        stable_sort(
            size_of::<CacheTmpChannel>(),
            16,
            channels as *mut c_void,
            cc.tmp_arr() as *mut c_void,
            count,
            tmp_channel_less,
            core::ptr::null_mut(),
        );
    }
    Ok(())
}

// ufbx.c:24308-24394 `ufbxi_cache_load_xml_imp`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) fn cache_load_xml_imp(cc: &CacheContext, doc: &XmlDocumentView) -> Result<(), Fail> {
    cc.set_xml_ticks_per_frame(250);
    cc.set_xml_filename(cc.stream_filename());

    // SAFETY: `doc.root()` is a valid arena `XmlTag`, stable for the document.
    let root: &XmlTagView = unsafe { XmlTagView::from_ptr(doc.root()) };
    if let Some(tag_root) = xml_find_child(root, c"Autodesk_Cache_File") {
        let tag_type = xml_find_child(tag_root, c"cacheType");
        let tag_fps = xml_find_child(tag_root, c"cacheTimePerFrame");
        let tag_channels = xml_find_child(tag_root, c"Channels");

        let mut num_extra: usize = 0;
        // C: `ufbxi_for(ufbxi_xml_tag, tag, tag_root->children, tag_root->num_children)`
        // SAFETY: contiguous arena run stable for the document's lifetime.
        let tags =
            unsafe { SliceViewIter::from_raw_parts(tag_root.children(), tag_root.num_children()) };
        for tag in tags {
            if tag.num_children() != 1 {
                continue;
            }
            if crate::native::error::c_strcmp(tag.name_view().bytes(), b"extra\0") != 0 {
                continue;
            }
            let extra: *mut String = cc.tmp_stack_view().push(1);
            ufbxi_check_err!(cc.error_view(), !extra.is_null(), "extra");
            // C: `tag->children[0].text` — `num_children == 1` guarantees the child.
            // SAFETY: `extra` is the fresh non-null push result; the child is the
            // first element of a valid arena run of length >= 1; the intern goes
            // through cc's own string pool.
            unsafe {
                *extra = XmlTagView::from_ptr(tag.children()).text();
                push_string_place_str(cc.string_pool_mut_ptr(), extra, false)?;
            }
            num_extra += 1;
        }
        cc.cache_view().extra_info_view().set_count(num_extra);
        // Pops the `num_extra` strings pushed above from cc's tmp stack into
        // cc's result buffer.
        cc.cache_view().extra_info_view().set_data(
            cc.result_view()
                .push_pop::<String>(cc.tmp_stack_view(), num_extra),
        );
        ufbxi_check_err!(
            cc.error_view(),
            !cc.cache_view().extra_info_view().data().is_null(),
            "cc->cache.extra_info.data"
        );

        if let Some(tag_type) = tag_type {
            let type_ = xml_find_attrib(tag_type, c"Type");
            let format = xml_find_attrib(tag_type, c"Format");
            if let Some(type_) = type_ {
                if crate::native::error::c_strcmp(type_.value_view().bytes(), b"OneFilePerFrame\0")
                    == 0
                {
                    cc.set_xml_type(CacheXmlType::FilePerFrame);
                } else if crate::native::error::c_strcmp(type_.value_view().bytes(), b"OneFile\0")
                    == 0
                {
                    cc.set_xml_type(CacheXmlType::SingleFile);
                }
            }
            if let Some(format) = format {
                if crate::native::error::c_strcmp(format.value_view().bytes(), b"mcc\0") == 0 {
                    cc.set_xml_format(CacheXmlFormat::Mcc);
                } else if crate::native::error::c_strcmp(format.value_view().bytes(), b"mcx\0") == 0
                {
                    cc.set_xml_format(CacheXmlFormat::Mcx);
                }
            }
        }

        if let Some(tag_fps) = tag_fps {
            if let Some(fps) = xml_find_attrib(tag_fps, c"TimePerFrame") {
                // SAFETY: attrib values are NUL-terminated arena strings
                // (xml-parser invariant), which is what the radix parse scans.
                let value: u32 =
                    unsafe { crate::native::float_parse::parse_uint32_radix(fps.value().data, 10) };
                if value > 0 {
                    cc.set_xml_ticks_per_frame(value);
                }
            }
        }

        if let Some(tag_channels) = tag_channels {
            cc.set_channels(cc.tmp_view().push_zero(tag_channels.num_children()));
            ufbxi_check_err!(cc.error_view(), !cc.channels().is_null(), "cc->channels");

            // C: `ufbxi_for(ufbxi_xml_tag, tag, tag_channels->children, tag_channels->num_children)`
            // SAFETY: contiguous arena run stable for the document's lifetime.
            let tags = unsafe {
                SliceViewIter::from_raw_parts(tag_channels.children(), tag_channels.num_children())
            };
            for tag in tags {
                let name = xml_find_attrib(tag, c"ChannelName");
                let type_ = xml_find_attrib(tag, c"ChannelType");
                let interpretation = xml_find_attrib(tag, c"ChannelInterpretation");
                let (Some(name), Some(_channel_type), Some(interpretation)) =
                    (name, type_, interpretation)
                else {
                    continue;
                };

                // C: `&cc->channels[cc->num_channels++]`
                // SAFETY: at most one channel is appended per child tag, so
                // `num_channels` stays within the `num_children`-element run
                // pushed above; `channel` is that fresh in-bounds slot, and
                // the interns go through cc's own string pool.
                let channel: *mut CacheTmpChannel = unsafe { cc.channels().add(cc.num_channels()) };
                cc.set_num_channels(cc.num_channels() + 1);
                unsafe {
                    (*channel).name = name.value();
                    (*channel).interpretation = interpretation.value();
                    push_string_place_str(cc.string_pool_mut_ptr(), &mut (*channel).name, false)?;
                    push_string_place_str(
                        cc.string_pool_mut_ptr(),
                        &mut (*channel).interpretation,
                        false,
                    )?;
                }

                let sampling_rate = xml_find_attrib(tag, c"SamplingRate");
                let start_time = xml_find_attrib(tag, c"StartTime");
                let end_time = xml_find_attrib(tag, c"EndTime");
                if let (Some(sampling_rate), Some(start_time), Some(end_time)) =
                    (sampling_rate, start_time, end_time)
                {
                    // SAFETY: `channel` is the in-bounds slot from above; the
                    // parses scan NUL-terminated arena attrib values.
                    unsafe {
                        (*channel).sample_rate = crate::native::float_parse::parse_uint32_radix(
                            sampling_rate.value().data,
                            10,
                        );
                        (*channel).start_time = crate::native::float_parse::parse_uint32_radix(
                            start_time.value().data,
                            10,
                        );
                        (*channel).end_time = crate::native::float_parse::parse_uint32_radix(
                            end_time.value().data,
                            10,
                        );
                        (*channel).current_time = (*channel).start_time;
                        (*channel).try_load = true;
                    }
                }
            }
        }
    }

    // SAFETY: sorting cc's own `channels` run of `num_channels` entries
    // (populated above / empty when no Channels tag).
    unsafe { cache_sort_tmp_channels(cc, cc.channels(), cc.num_channels())? };
    Ok(())
}

// ufbx.c:24396-24412 `ufbxi_cache_load_xml`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) fn cache_load_xml(cc: &CacheContext) -> Result<(), Fail> {
    // C: `ufbxi_xml_load_opts opts = { 0 };`
    // SAFETY: all-zero `XmlLoadOpts` is valid (null pointers, `None`
    // callbacks, zero lengths).
    let mut opts: XmlLoadOpts = unsafe { core::mem::zeroed() };
    opts.ator = cc.ator_tmp();
    opts.read_fn = cc.stream_view().read_fn();
    opts.read_user = cc.stream_view().user();
    opts.prefix = cc.pos();
    // SAFETY: `pos..pos_end` is cc's buffered read window (one allocation).
    opts.prefix_length = unsafe { to_size(cc.pos_end().offset_from(cc.pos())) };
    // SAFETY: `opts` is a valid local; the error out-pointer is cc's own
    // error storage via its raw-ptr getter.
    let doc: *mut XmlDocument = unsafe { load_xml(&mut opts, cc.error_mut_ptr()) };
    ufbxi_check_err!(cc.error_view(), !doc.is_null(), "doc");

    // Bridge the raw `load_xml` result once; the view anchors the document for
    // the `cache_load_xml_imp` call, then `free_xml` reclaims the raw pointer.
    // SAFETY: `doc` is non-null and points to a valid `XmlDocument` stable until
    // `free_xml`.
    let xml_ok = cache_load_xml_imp(cc, unsafe { XmlDocumentView::from_ptr(doc) });
    // SAFETY: `doc` is the non-null `load_xml` result, not used after this.
    unsafe { free_xml(doc) };
    ufbxi_check_err!(cc.error_view(), xml_ok.is_ok(), "xml_ok");

    Ok(())
}

// ufbx.c:24414-24437 `ufbxi_cache_load_file`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_load_file(cc: &CacheContext, filename: String) -> Result<(), Fail> {
    cc.set_stream_filename(filename);
    // SAFETY: both pointers address `cc`'s own fields — the string pool it
    // owns and the `stream_filename` just stored — so they are live and
    // properly aligned for the in-place interning `push_string_place_str` does.
    unsafe {
        push_string_place_str(
            cc.string_pool_mut_ptr(),
            cc.stream_filename_mut_ptr(),
            false,
        )?;
    }

    // Assume all files have at least 16 bytes of header
    // SAFETY: the stream's `read_fn` is non-null for the whole lifetime of an
    // opened `cc.stream` (the caller opened it before calling); the
    // destination is `cc`'s own buffer, which is larger than the 16-byte
    // header read here.
    let magic_len: usize = unsafe {
        (cc.stream_view().read_fn().unwrap_unchecked())(
            cc.stream_view().user(),
            cc.buffer_mut_ptr() as *mut c_void,
            16,
        )
    };
    ufbxi_check_err_msg!(
        cc.error_view(),
        magic_len <= 16,
        "IO error",
        "magic_len <= 16"
    );
    ufbxi_check_err_msg!(
        cc.error_view(),
        magic_len == 16,
        "Truncated file",
        "magic_len == 16"
    );
    cc.set_pos(cc.buffer_ptr());
    // SAFETY: `cc`'s buffer is larger than the 16-byte header, so this is an
    // interior pointer of that buffer.
    cc.set_pos_end(unsafe { cc.buffer_ptr().add(16) });

    cc.set_file_offset(0);

    // SAFETY (all three): the checks above established that the read filled 16
    // bytes of `cc`'s buffer, so the 11 and 4 byte compares are in bounds; the
    // right-hand operands are byte literals at least that long.
    if unsafe { crate::native::error::memcmp(cc.buffer_ptr(), b"POINTCACHE2".as_ptr(), 11) } == 0 {
        cache_load_pc2(cc)?;
    } else if unsafe { crate::native::error::memcmp(cc.buffer_ptr(), b"FOR4".as_ptr(), 4) } == 0
        || unsafe { crate::native::error::memcmp(cc.buffer_ptr(), b"FOR8".as_ptr(), 4) } == 0
    {
        cache_load_mc(cc)?;
    } else {
        cache_load_xml(cc)?;
    }

    Ok(())
}

// ufbx.c:24439-24455 `ufbxi_cache_try_open_file`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_try_open_file(
    cc: &CacheContext,
    filename: String,
    original_filename: *const crate::prelude::Blob,
    p_found: *mut bool,
) -> Result<(), Fail> {
    // SAFETY: `stream_mut_ptr` addresses `cc`'s own `RawStream` field, so one
    // `RawStream` worth of bytes is writable there.
    unsafe { core::ptr::write_bytes(cc.stream_mut_ptr(), 0, 1) };
    // SAFETY: `filename.data` is a NUL-terminated buffer at every caller —
    // either the `ufbxi_snprintf!`-formatted `name_buf` of
    // `cache_load_frame_files` or the explicitly NUL-terminated arena copy from
    // `cache_load_imp` — so `strlen` stops within `length` bytes.
    ufbxi_regression_assert!(unsafe { strlen(filename.data) } == filename.length);
    // SAFETY: the callback and stream pointers address `cc`'s own fields;
    // `filename.data`/`.length` is a live, NUL-terminated caller buffer (the
    // formatted `name_buf` or the arena copy), `original_filename` is the
    // caller's `Blob` pointer, and the allocator is `cc`'s own temp one.
    if !unsafe {
        open_file(
            cc.open_file_cb_ptr(),
            cc.stream_mut_ptr(),
            filename.data,
            filename.length,
            original_filename,
            cc.ator_tmp(),
            OpenFileType::GeometryCache,
        )
    } {
        return Ok(());
    }

    // SAFETY: `open_file` returned true, so `cc.stream` is an opened stream —
    // what `cache_load_file` requires to read its header.
    let ok = unsafe { cache_load_file(cc, filename) };
    // SAFETY: the caller's contract is that `p_found` points at a writable
    // `bool` out-param.
    unsafe { *p_found = true };

    if let Some(close_fn) = cc.stream_view().close_fn() {
        // SAFETY: the C-callback contract — `close_fn` came from the stream
        // `open_file` opened above, and is invoked once with that stream's own
        // `user` pointer.
        unsafe { close_fn(cc.stream_view().user()) };
    }

    ok
}

// ufbx.c:24457-24540 `ufbxi_cache_load_frame_files`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) fn cache_load_frame_files(cc: &CacheContext) -> Result<(), Fail> {
    if cc.xml_filename_view().length() == 0 {
        return Ok(());
    }

    let extension: *const u8;
    match cc.xml_format() {
        CacheXmlFormat::Mcc => extension = b"mc\0".as_ptr(),
        CacheXmlFormat::Mcx => extension = b"mcx\0".as_ptr(),
        _ => return Ok(()),
    }

    // Ensure worst case space for `path/filenameFrame123Tick456.mcx`
    let name_buf_len: usize = cc.xml_filename_view().length() + 64;
    let name_buf: *mut u8 = cc.tmp_view().push(name_buf_len);
    ufbxi_check_err!(cc.error_view(), !name_buf.is_null(), "name_buf");

    // Find the prefix before `.xml`
    let mut prefix_len: usize = cc.xml_filename_view().length();
    let mut i: usize = prefix_len;
    while i > 0 {
        // SAFETY: `i - 1 < length`, an in-bounds byte of the interned filename.
        if unsafe { *cc.xml_filename_view().data().add(i - 1) } == b'.' {
            prefix_len = i - 1;
            break;
        }
        i -= 1;
    }
    // SAFETY: `prefix_len <= length` and `name_buf` holds `length + 64` bytes
    // (the fresh non-null push above), so both source and destination ranges
    // are in bounds and disjoint.
    unsafe { core::ptr::copy_nonoverlapping(cc.xml_filename_view().data(), name_buf, prefix_len) };

    // SAFETY: `prefix_len < name_buf_len`, in bounds of the push result.
    let suffix_data: *mut u8 = unsafe { name_buf.add(prefix_len) };
    let suffix_len: usize = name_buf_len - prefix_len;

    // C: `ufbx_string filename;` — both members are written before any read,
    // and upstream carries no partial-init marker here.
    let mut filename = MaybeUninit::<String>::uninit();
    let filename: *mut String = filename.as_mut_ptr();
    // SAFETY: `filename` is a local; `data` is written here and `length` on
    // every path before the value is read.
    unsafe { (*filename).data = name_buf };

    if cc.xml_type() == CacheXmlType::SingleFile {
        // SAFETY: formats into the `suffix_len` bytes remaining in `name_buf`
        // (sized for the worst-case name above); `extension` is a
        // NUL-terminated literal; `*filename` is fully initialized by the
        // writes above/here before `cache_try_open_file` reads it.
        unsafe {
            (*filename).length =
                prefix_len + ufbxi_snprintf!(suffix_data, suffix_len, ".%s", extension) as usize;
            let mut found: bool = false;
            cache_try_open_file(cc, *filename, core::ptr::null(), &mut found)?;
        }
    } else if cc.xml_type() == CacheXmlType::FilePerFrame {
        let mut lowest_time: u32 = 0;
        loop {
            // Find the first `time >= lowest_time` value that has data in some channel
            let mut time: u32 = u32::MAX;
            // C: `ufbxi_for(ufbxi_cache_tmp_channel, chan, cc->channels, cc->num_channels)`
            // SAFETY: `cc.channels` is a contiguous `push_zero` run of
            // `cc.num_channels` `CacheTmpChannel`, stable for this load.
            let chans = unsafe { SliceViewIter::from_raw_parts(cc.channels(), cc.num_channels()) };
            for chan in chans {
                if !chan.try_load() || chan.consecutive_fails() > 10 {
                    continue;
                }
                let sample_rate: u32 = if chan.sample_rate() != 0 {
                    chan.sample_rate()
                } else {
                    cc.xml_ticks_per_frame()
                };
                if chan.current_time() < lowest_time {
                    let delta: u32 = (lowest_time - chan.current_time() - 1) / sample_rate;
                    chan.set_current_time(
                        chan.current_time()
                            .wrapping_add(delta.wrapping_mul(sample_rate)),
                    );
                    if u32::MAX - chan.current_time() >= sample_rate {
                        chan.set_current_time(chan.current_time().wrapping_add(sample_rate));
                    } else {
                        chan.set_try_load(false);
                        continue;
                    }
                }
                if chan.current_time() <= chan.end_time() {
                    time = min32(time, chan.current_time());
                }
            }
            if time == u32::MAX {
                break;
            }

            // Try to load a file at the specified frame/tick
            let frame: u32 = time / cc.xml_ticks_per_frame();
            let tick: u32 = time % cc.xml_ticks_per_frame();
            let mut found: bool = false;
            // SAFETY: same contract as the SingleFile arm — formats into the
            // remaining `suffix_len` bytes of `name_buf`, then reads the fully
            // initialized `*filename`.
            unsafe {
                if tick == 0 {
                    (*filename).length = prefix_len
                        + ufbxi_snprintf!(suffix_data, suffix_len, "Frame%u.%s", frame, extension)
                            as usize;
                } else {
                    (*filename).length = prefix_len
                        + ufbxi_snprintf!(
                            suffix_data,
                            suffix_len,
                            "Frame%uTick%u.%s",
                            frame,
                            tick,
                            extension
                        ) as usize;
                }
                cache_try_open_file(cc, *filename, core::ptr::null(), &mut found)?;
            }

            // Update channel status
            // C: `ufbxi_for(ufbxi_cache_tmp_channel, chan, cc->channels, cc->num_channels)`
            // SAFETY: same contiguous `cc.channels` run as above.
            let chans = unsafe { SliceViewIter::from_raw_parts(cc.channels(), cc.num_channels()) };
            for chan in chans {
                if chan.current_time() == time {
                    chan.set_consecutive_fails(if found {
                        0
                    } else {
                        chan.consecutive_fails().wrapping_add(1)
                    });
                }
            }

            lowest_time = time.wrapping_add(1);
        }
    }

    Ok(())
}

// ufbx.c:24542-24552 `ufbxi_cmp_cache_frame_less`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe extern "C" fn cmp_cache_frame_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    let _ = user;
    let a: *const CacheFrame = va as *const CacheFrame;
    let b: *const CacheFrame = vb as *const CacheFrame;
    // SAFETY (every deref of `a`/`b` below): the sort's comparator contract is
    // that `va`/`vb` address live elements of the array being sorted, which
    // `cache_sort_frames` instantiates with `CacheFrame`. The `str_equal` /
    // `str_less` calls in turn require valid `String` runs, which those
    // elements' interned `channel` fields are.
    if unsafe { (*a).channel.data != (*b).channel.data } {
        // Channel names should be interned
        unsafe {
            ufbxi_regression_assert!(!str_equal_raw((*a).channel, (*b).channel));
            return str_less_raw((*a).channel, (*b).channel);
        }
    }
    unsafe { (*a).time < (*b).time }
}

// ufbx.c:24554-24559 `ufbxi_cache_sort_frames`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_sort_frames(
    cc: &CacheContext,
    frames: *mut CacheFrame,
    count: usize,
) -> Result<(), Fail> {
    // SAFETY: the growth targets are `cc`'s own `tmp_arr` pointer/size pair,
    // grown through `cc`'s own temp allocator — the pairing `grow_array`
    // requires. The verbatim C condition text is supplied, so wrapping the
    // condition does not perturb the recorded error string.
    ufbxi_check_err!(
        cc.error_view(),
        unsafe {
            grow_array::<u8>(
                cc.ator_tmp(),
                cc.tmp_arr_mut_ptr(),
                cc.tmp_arr_size_mut_ptr(),
                count * size_of::<CacheFrame>()
            )
        },
        "ufbxi_grow_array_size((cc->ator_tmp), sizeof(**(&cc->tmp_arr)), (&cc->tmp_arr), (&cc->tmp_arr_size), (count * sizeof(ufbx_cache_frame)))"
    );
    // SAFETY: the caller's contract is that `frames` addresses `count` live
    // `CacheFrame`s; the scratch buffer is `cc.tmp_arr`, just grown to
    // `count * size_of::<CacheFrame>()` bytes, and the element size /
    // comparator match that type.
    unsafe {
        stable_sort(
            size_of::<CacheFrame>(),
            16,
            frames as *mut c_void,
            cc.tmp_arr() as *mut c_void,
            count,
            cmp_cache_frame_less,
            core::ptr::null_mut(),
        );
    }
    Ok(())
}

// ufbx.c:24561-24564 `ufbxi_cache_interpretation_name`
#[cfg(feature = "geometry-cache")]
#[repr(C)]
pub(crate) struct CacheInterpretationName {
    pub interpretation: CacheInterpretation,
    pub pattern: *const u8,
}
// The table below is immutable and its `const char *` member references an
// immutable string literal, so sharing is sound (same rationale as
// `LegacyProp` in `native::read`).
#[cfg(feature = "geometry-cache")]
unsafe impl Sync for CacheInterpretationName {}

// ufbx.c:24566-24570 `ufbxi_cache_interpretation_names`
#[cfg(feature = "geometry-cache")]
static CACHE_INTERPRETATION_NAMES: [CacheInterpretationName; 3] = [
    CacheInterpretationName {
        interpretation: CacheInterpretation::Points,
        pattern: b"\\cpoints?\0".as_ptr(),
    },
    CacheInterpretationName {
        interpretation: CacheInterpretation::VertexPosition,
        pattern: b"\\cpositions?\0".as_ptr(),
    },
    CacheInterpretationName {
        interpretation: CacheInterpretation::VertexNormal,
        pattern: b"\\cnormals?\0".as_ptr(),
    },
];

// ufbx.c:24572-24634 `ufbxi_cache_setup_channels`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) fn cache_setup_channels(cc: &CacheContext) -> Result<(), Fail> {
    let mut tmp_chan: *mut CacheTmpChannel = cc.channels();
    let tmp_end: *mut CacheTmpChannel = add_ptr(tmp_chan, cc.num_channels());

    let mut begin: usize = 0;
    let mut num_channels: usize = 0;
    while begin < cc.cache_view().frames_view().count() {
        // SAFETY: `begin < count` (loop guard) and `end` is bounds-checked
        // before every deref, so the group scan stays inside the cache's
        // materialized frames run.
        let (frame, end) = unsafe {
            let frame: *mut CacheFrame =
                (cc.cache_view().frames_view().data() as *mut CacheFrame).add(begin);
            let mut end: usize = begin + 1;
            while end < cc.cache_view().frames_view().count()
                && (*(cc.cache_view().frames_view().data() as *mut CacheFrame).add(end))
                    .channel
                    .data
                    == (*frame).channel.data
            {
                end += 1;
            }
            (frame, end)
        };

        let chan: *mut CacheChannel = cc.tmp_stack_view().push_zero(1);
        ufbxi_check_err!(cc.error_view(), !chan.is_null(), "chan");

        // SAFETY: `chan` is the fresh non-null push result; `tmp_chan` walks
        // cc's own channels run bounds-checked against `tmp_end`; the interned
        // channel-name Strings satisfy `str_less`/`str_equal`.
        unsafe {
            (*chan).name = (*frame).channel;
            (*chan).interpretation_name = EMPTY_STRING.0;
            (*chan).frames.data = frame;
            (*chan).frames.count = end - begin;

            while tmp_chan < tmp_end && str_less_raw((*tmp_chan).name, (*chan).name) {
                tmp_chan = tmp_chan.add(1);
            }
            if tmp_chan < tmp_end && str_equal_raw((*tmp_chan).name, (*chan).name) {
                (*chan).interpretation_name = (*tmp_chan).interpretation;
            }

            if (*frame).file_format == CacheFileFormat::Pc2 {
                (*chan).interpretation = CacheInterpretation::VertexPosition;
            } else {
                // C: `ufbxi_for(const ufbxi_cache_interpretation_name, name, ufbxi_cache_interpretation_names, ufbxi_arraycount(...))`
                // The patterns are NUL-terminated literals in a static array.
                let mut name: *const CacheInterpretationName = CACHE_INTERPRETATION_NAMES.as_ptr();
                let name_end: *const CacheInterpretationName =
                    name.add(CACHE_INTERPRETATION_NAMES.len());
                while name != name_end {
                    if r#match(&(*chan).interpretation_name, (*name).pattern) {
                        (*chan).interpretation = (*name).interpretation;
                        break;
                    }
                    name = name.add(1);
                }
            }
        }

        let mut mirror_axis: MirrorAxis = MirrorAxis::None;
        let mut scale_factor: Real = 1.0f32 as Real;
        // SAFETY: `chan` is the in-bounds push result from above.
        if unsafe { (*chan).interpretation } != CacheInterpretation::Unknown {
            mirror_axis = cc.opts_view().mirror_axis();
            if cc.opts_view().use_scale_factor() {
                scale_factor = cc.opts_view().scale_factor();
            }
        }
        // C: `ufbxi_for_list(ufbx_cache_frame, f, chan->frames)`
        // SAFETY: `chan` as above; `chan->frames` is the contiguous
        // `end - begin` slice of the materialized frames run, stable here.
        let frames = unsafe {
            (*chan).mirror_axis = mirror_axis;
            (*chan).scale_factor = scale_factor;
            SliceViewIter::from_raw_parts(
                (*chan).frames.data as *mut CacheFrame,
                (*chan).frames.count,
            )
        };
        for f in frames {
            f.set_mirror_axis(mirror_axis);
            f.set_scale_factor(scale_factor);
        }

        num_channels += 1;
        begin = end;
    }

    // Pops the `num_channels` channels pushed above from cc's tmp stack into
    // cc's result buffer.
    cc.cache_view().channels_view().set_data(
        cc.result_view()
            .push_pop::<CacheChannel>(cc.tmp_stack_view(), num_channels),
    );
    ufbxi_check_err!(
        cc.error_view(),
        !cc.cache_view().channels_view().data().is_null(),
        "cc->cache.channels.data"
    );
    cc.cache_view().channels_view().set_count(num_channels);

    Ok(())
}

// ufbx.c:24637-24691 `ufbxi_cache_load_imp`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_load_imp(
    cc: &CacheContext,
    filename: String,
) -> Result<crate::native::parse::FinishedImp<GeometryCacheImp>, Fail> {
    cc.tmp_view().set_ator(cc.ator_tmp());
    cc.tmp_stack_view().set_ator(cc.ator_tmp());

    cc.channel_name_view().set_data(EMPTY_CHAR.as_ptr());

    if cc.open_file_cb_view().fn_().is_none() {
        cc.open_file_cb_view()
            .set_fn_(Some(crate::native::api::default_open_file));
    }

    // Make sure the filename we pass to `open_file_fn()` is NULL-terminated
    let filename_data: *mut u8 = cc.tmp_view().push(filename.length + 1);
    ufbxi_check_err!(cc.error_view(), !filename_data.is_null(), "filename_data");
    // SAFETY: `filename_data` is a fresh non-null `length + 1` byte arena
    // allocation (checked above), distinct from the caller's `filename` run of
    // `length` readable bytes.
    unsafe { core::ptr::copy_nonoverlapping(filename.data, filename_data, filename.length) };
    // SAFETY: the allocation is `length + 1` bytes, so index `length` is its
    // last writable byte.
    unsafe { *filename_data.add(filename.length) = b'\0' };
    let filename_copy: String = String::new_c(filename_data, filename.length);

    // TODO: NULL termination!
    let mut found: bool = false;
    // SAFETY: `filename_copy` is the NUL-terminated arena copy built just
    // above, and `&mut found` is a live local out-param.
    unsafe { cache_try_open_file(cc, filename_copy, core::ptr::null(), &mut found)? };
    if !found {
        // SAFETY: `error_mut_ptr` addresses `cc`'s own `Error` field, and
        // `filename.data`/`.length` is the caller's live string run.
        unsafe { set_err_info(cc.error_mut_ptr(), filename.data, filename.length) };
        ufbxi_fail_err_msg!(cc.error_view(), "open_file_fn()", "File not found");
    }

    cc.cache_view().set_root_filename(cc.stream_filename());

    cache_load_frame_files(cc)?;

    let num_frames: usize = cc.tmp_stack_view().num_items();
    cc.cache_view().frames_view().set_count(num_frames);
    cc.cache_view().frames_view().set_data(
        cc.result_view()
            .push_pop::<CacheFrame>(cc.tmp_stack_view(), num_frames),
    );
    ufbxi_check_err!(
        cc.error_view(),
        !cc.cache_view().frames_view().data().is_null(),
        "cc->cache.frames.data"
    );

    // SAFETY: the frames run was just pushed into the result buf and checked
    // non-null, and `count` is the item count that push used — so the pointer
    // addresses exactly `count` live `CacheFrame`s.
    unsafe {
        cache_sort_frames(
            cc,
            cc.cache_view().frames_view().data() as *mut CacheFrame,
            cc.cache_view().frames_view().count(),
        )?;
    }
    cache_setup_channels(cc)?;

    // Must be last allocation!
    cc.set_imp(cc.result_view().push(1));
    ufbxi_check_err!(cc.error_view(), !cc.imp().is_null(), "cc->imp");

    // C: `ufbxi_init_ref(...)` / `cc->imp->cache = cc->cache` /
    // `cc->imp->magic = ...` / `cc->imp->refcount.ator = cc->ator_result` /
    // `cc->imp->refcount.buf = cc->result` — the shared imp-finalization group.
    //
    // SAFETY (the `finish_imp` call and the store run below): `cc.imp()` is the
    // non-null single-element result-buf allocation pushed and checked just
    // above; it is the last allocation, so nothing else aliases it while these
    // header fields are stamped. `cache_mut_ptr` addresses `cc`'s own
    // `GeometryCache` field — a distinct allocation — and the helper moves it
    // into the fresh `imp` slot; `cc.cache` is not read again (PORTING.md
    // "Copy vs non-Copy internal structs": an explicit move).
    let finished_imp = unsafe {
        finish_imp(
            cc.imp(),
            core::ptr::null_mut(),
            cc.cache_mut_ptr(),
            cc.ator_result(),
            cc.take_result(),
        )
    };

    unsafe {
        (*cc.imp()).owned_by_scene = cc.owned_by_scene();
        (*cc.imp()).refcount.buf.ator = &raw mut (*cc.imp()).refcount.ator;
        (*cc.imp()).string_buf = cc.string_pool_view().take_buf();
        (*cc.imp()).string_buf.ator = &raw mut (*cc.imp()).refcount.ator;
    }

    Ok(finished_imp)
}

// ufbx.c:24693-24716 `ufbxi_cache_load`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_load(cc: &CacheContext, filename: String) -> *mut GeometryCache {
    // SAFETY: `cc` is the initialized cache context the caller set up, which is
    // what `cache_load_imp` requires; `filename` is forwarded unchanged. On
    // success the `FinishedImp` carries the finished imp through the shared
    // teardown to the return below.
    let result = unsafe { cache_load_imp(cc, filename) };

    // SAFETY: every pointer addresses one of `cc`'s own fields, each paired
    // with the allocator that produced it — the temp bufs and the
    // `name_buf`/`tmp_arr` runs with `cc.ator_tmp`. Each is freed once, and the
    // context is not used for allocation afterwards.
    unsafe {
        buf_free(cc.tmp_mut_ptr());
        buf_free(cc.tmp_stack_mut_ptr());
        free::<u8>(cc.ator_tmp(), cc.name_buf(), cc.name_cap());
        free::<u8>(cc.ator_tmp(), cc.tmp_arr(), cc.tmp_arr_size());
    }
    if !cc.owned_by_scene() {
        // SAFETY: the temp allocator and its string pool belong to `cc` alone
        // when the cache is not owned by a scene, so freeing them here is the
        // single release of that state.
        unsafe {
            string_pool_temp_free(cc.string_pool_mut_ptr());
            free_ator(cc.ator_tmp());
        }
    }

    if let Ok(finished_imp) = result {
        // C: `return &cc->imp->cache;` — commit the finished imp across the ABI.
        finished_imp.into_payload()
    } else {
        // SAFETY: `error_mut_ptr` addresses `cc`'s own live `Error` field, and
        // the default description is a NUL-terminated byte literal.
        unsafe {
            fix_error_type(
                cc.error_mut_ptr(),
                b"Failed to load geometry cache\0",
                core::ptr::null_mut(),
            );
        }
        if !cc.owned_by_scene() {
            // SAFETY: on the failure path the result buf never reached an
            // `imp`, so `cc` still owns the string-pool buf and the result
            // allocator — both its own fields, freed exactly once here.
            unsafe { buf_free(cc.string_pool_view().buf_mut_ptr()) };
            unsafe { free_ator(cc.ator_result_mut_ptr()) };
        }
        core::ptr::null_mut()
    }
}

// ufbx.c:24718-24755 `ufbxi_load_geometry_cache` (UFBXI_FEATURE_GEOMETRY_CACHE)
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn load_geometry_cache(
    filename: String,
    user_opts: *const RawGeometryCacheOpts,
    p_error: *mut Error,
) -> *mut GeometryCache {
    // C: `ufbx_geometry_cache_opts opts; // ufbxi_uninit`
    let opts: RawGeometryCacheOpts = if !user_opts.is_null() {
        // SAFETY: `user_opts` is non-null and the caller's contract is that it
        // points at a readable, initialized options struct; the read copies it
        // and leaves the caller's copy intact (the struct is plain data).
        unsafe { core::ptr::read(user_opts) }
    } else {
        // SAFETY: `RawGeometryCacheOpts` is plain data whose all-zero bit
        // pattern is the C `= { 0 }` default.
        unsafe { core::mem::zeroed() }
    };

    // C: `ufbxi_cache_context cc = { UFBX_ERROR_NONE };` / `ufbxi_allocator ator_tmp = { 0 };`
    // SAFETY: both are plain-data structs whose all-zero bit pattern is the C
    // zero-initializer these lines port.
    let cc: CacheContext = unsafe { core::mem::zeroed() };
    let mut ator_tmp: Allocator = unsafe { core::mem::zeroed() };
    // SAFETY: the error pointer addresses `cc`'s own `Error` field, the
    // allocators are the zeroed locals/fields being initialized here, the opts
    // references are live, and the names are NUL-terminated byte literals.
    unsafe {
        init_ator(
            cc.error_mut_ptr(),
            &mut ator_tmp,
            &opts.temp_allocator,
            c"temp",
        );
        init_ator(
            cc.error_mut_ptr(),
            cc.ator_result_mut_ptr(),
            &opts.result_allocator,
            c"result",
        );
    }
    cc.set_ator_tmp(&mut ator_tmp);

    // SAFETY: `&opts` is a live local; the read copies this plain-data struct,
    // which stays valid to use afterwards.
    cc.set_opts(unsafe { core::ptr::read(&opts) });

    cc.set_open_file_cb(opts.open_file_cb);

    cc.string_pool_view().set_error(cc.error_mut_ptr());
    // SAFETY: the map addressed is `cc`'s own string-pool map, paired with
    // `cc`'s temp allocator (initialized above) — the pairing `map_init`
    // requires; `map_cmp_string` is the comparator for its string keys.
    unsafe {
        map_init(
            cc.string_pool_view().map_mut_ptr(),
            cc.ator_tmp(),
            map_cmp_string,
            core::ptr::null_mut(),
        );
    }
    cc.string_pool_view()
        .buf_view()
        .set_ator(cc.ator_result_mut_ptr());
    cc.string_pool_view().buf_view().set_unordered(true);
    cc.string_pool_view().set_initial_size(64);
    cc.result_view().set_ator(cc.ator_result_mut_ptr());

    cc.set_frames_per_second(if opts.frames_per_second > 0.0 {
        opts.frames_per_second
    } else {
        30.0
    });

    // SAFETY: `cc` is fully initialized above (allocators, string pool, opts) —
    // the state `cache_load` consumes.
    let cache: *mut GeometryCache = unsafe { cache_load(&cc, filename) };
    if !p_error.is_null() {
        if !cache.is_null() {
            // SAFETY: `p_error` is non-null and the caller's contract is that
            // it points at a writable `Error` out-param.
            unsafe { clear_error(p_error) };
        } else {
            // SAFETY: same writable-out-param contract for `p_error`;
            // `error_mut_ptr` addresses `cc`'s own live `Error`, and the two
            // are distinct objects. `Error` is plain data, so the read leaves
            // `cc`'s copy usable.
            unsafe { core::ptr::write(p_error, core::ptr::read(cc.error_mut_ptr())) };
        }
    }
    cache
}

// ufbx.c:24757-24761 `ufbxi_free_geometry_cache_imp` (UFBXI_FEATURE_GEOMETRY_CACHE)
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn free_geometry_cache_imp(imp: *mut GeometryCacheImp) {
    // SAFETY: the caller's contract is that `imp` points at a live
    // `GeometryCacheImp` — the magic read then confirms it is one of ours, and
    // `string_buf` is that header's own buf, freed once as the cache dies.
    unsafe {
        ufbx_assert!((*imp).magic == CACHE_IMP_MAGIC);
        buf_free(&mut (*imp).string_buf);
    }
}

// ufbx.c:24765-24769 `ufbxi_geometry_cache_imp` (`#else` branch — feature disabled)
#[cfg(not(feature = "geometry-cache"))]
#[repr(C)]
pub(crate) struct GeometryCacheImp {
    pub refcount: Refcount,
    pub magic: u32,
    pub owned_by_scene: bool,
}

// SAFETY: `#[repr(C)]` with `refcount` leading, `CACHE_IMP_MAGIC` is the magic
// `ufbxi_get_imp(ufbxi_geometry_cache_imp, ...)` users check, and
// `header_parts` projects the two named fields of the passed `imp`. This arm
// has NO payload field (this build never creates a cache), but
// `ufbx_free_geometry_cache`/`ufbx_retain_geometry_cache` still compile their
// recovery against it, exactly like the C `#else` struct — recovery-only, no
// `ImpHeader`.
#[cfg(not(feature = "geometry-cache"))]
unsafe impl crate::native::parse::ImpRecover for GeometryCacheImp {
    type Payload = crate::generated::GeometryCache;
    const MAGIC: u32 = crate::native::allocator::CACHE_IMP_MAGIC;

    #[inline(always)]
    unsafe fn header_parts(imp: *mut Self) -> (*mut Refcount, *mut u32) {
        // SAFETY: the caller vouches `imp` addresses a live `GeometryCacheImp`,
        // so these field projections stay inside that allocation.
        unsafe { (&raw mut (*imp).refcount, &raw mut (*imp).magic) }
    }
}

// ufbx.c:24771-24779 `ufbxi_load_geometry_cache` (`#else` branch — feature
// disabled). C parity, NOT a stub: a build without `feature =
// "geometry-cache"` must report `UFBX_ERROR_FEATURE_DISABLED` exactly like a C
// build with `UFBX_MINIMAL`.
#[cfg(not(feature = "geometry-cache"))]
#[inline(never)]
pub(crate) unsafe fn load_geometry_cache(
    filename: String,
    user_opts: *const RawGeometryCacheOpts,
    p_error: *mut Error,
) -> *mut GeometryCache {
    // C: `filename`/`user_opts` are unreferenced in the `#else` arm.
    let _ = (filename, user_opts);
    if !p_error.is_null() {
        // SAFETY: `p_error` is non-null and the caller's contract is that it
        // points at a writable `Error` out-param, so exactly
        // `size_of::<Error>()` bytes are writable there.
        unsafe { core::ptr::write_bytes(p_error as *mut u8, 0, size_of::<Error>()) };
        // SAFETY: `p_error` is the non-null writable `Error` out-param
        // zero-filled just above, and the format string is a literal.
        unsafe { ufbxi_fmt_err_info!(p_error, "UFBX_ENABLE_GEOMETRY_CACHE") };
        ufbxi_report_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(p_error) },
            "UFBXI_FEATURE_GEOMETRY_CACHE",
            "Feature disabled"
        );
    }
    core::ptr::null_mut()
}

// ufbx.c:24781-24783 `ufbxi_free_geometry_cache_imp` (`#else` branch — feature disabled)
#[cfg(not(feature = "geometry-cache"))]
#[inline(always)]
pub(crate) fn free_geometry_cache_imp(imp: *mut GeometryCacheImp) {
    let _ = imp;
}

// -- External files (ufbx.c:24787-25010)
//
// Outside C's geometry-cache `#if` — always compiled; only the body of
// `ufbxi_load_external_cache` forks on the feature.

// ufbx.c:24789-24791 `ufbxi_external_file_type`
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ExternalFileType {
    GeometryCache = 0,
}

// ufbx.c:24793-24800 `ufbxi_external_file`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ExternalFile {
    pub type_: ExternalFileType,
    pub filename: String,
    pub absolute_filename: String,
    pub index: usize,
    pub data: *mut c_void,
    pub data_size: usize,
}

// ufbx.c:24802-24811 `ufbxi_less_external_file`
pub(crate) unsafe extern "C" fn less_external_file(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    let _ = user;
    let a: *const ExternalFile = va as *const ExternalFile;
    let b: *const ExternalFile = vb as *const ExternalFile;
    // SAFETY (every deref of `a`/`b` below): the sort's comparator contract is
    // that `va`/`vb` address live elements of the array being sorted, which the
    // external-file sort instantiates with `ExternalFile`. `str_cmp` in turn
    // requires two valid `String` runs, which those elements' interned
    // `filename` fields are.
    if unsafe { (*a).type_ != (*b).type_ } {
        return unsafe { (*a).type_ < (*b).type_ };
    }
    let cmp: i32 = unsafe { str_cmp_raw((*a).filename, (*b).filename) };
    if cmp != 0 {
        return cmp < 0;
    }
    if unsafe { (*a).index != (*b).index } {
        return unsafe { (*a).index < (*b).index };
    }
    false
}

// ufbx.c:24813-24867 `ufbxi_load_external_cache`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn load_external_cache(
    uc: &Context,
    file: *mut ExternalFile,
) -> Result<(), Fail> {
    // C: `ufbxi_cache_context cc = { UFBX_ERROR_NONE };`
    // SAFETY: `CacheContext` is plain data whose all-zero bit pattern is the C
    // zero-initializer this line ports.
    let cc: CacheContext = unsafe { core::mem::zeroed() };
    cc.set_owned_by_scene(true);

    cc.set_open_file_cb(uc.opts_view().open_file_cb());
    cc.set_frames_per_second(uc.scene_view().settings_view().frames_per_second());

    // Temporarily "borrow" allocators for the geometry cache
    cc.set_ator_tmp(uc.ator_tmp_mut_ptr());
    cc.set_string_pool(uc.take_string_pool());
    cc.set_result(uc.take_result());

    cc.opts_view().set_mirror_axis(uc.mirror_axis());
    cc.opts_view().set_use_scale_factor(true);
    cc.opts_view()
        .set_scale_factor(uc.scene_view().metadata_view().geometry_scale());

    // SAFETY: `cc` is initialized above with the borrowed allocators and string
    // pool `cache_load` consumes; the caller's contract is that `file` points
    // at a live `ExternalFile`, whose `filename` is an interned pool string.
    let mut cache: *mut GeometryCache = unsafe { cache_load(&cc, (*file).filename) };
    if cache.is_null() {
        if cc.error_view().type_() == ErrorType::FileNotFound {
            // SAFETY: `error_mut_ptr` addresses `cc`'s own `Error` field, so
            // one `Error` worth of bytes is writable there.
            unsafe { core::ptr::write_bytes(cc.error_mut_ptr(), 0, 1) };
            // SAFETY: same live-`ExternalFile` and initialized-`cc` argument as
            // the first attempt; the error was just cleared for the retry.
            cache = unsafe { cache_load(&cc, (*file).absolute_filename) };
        }
    }

    // Return the "borrowed" allocators
    uc.set_string_pool(cc.take_string_pool());
    uc.set_result(cc.take_result());

    if cache.is_null() {
        if cc.error_view().type_() == ErrorType::FileNotFound {
            if uc.opts_view().ignore_missing_external_files() {
                // SAFETY: the caller's contract is that `file` points at a live
                // `ExternalFile`, whose `filename` is a NUL-terminated interned
                // pool string — what the `%s` conversion reads. The verbatim C
                // condition text is supplied, so wrapping the condition does
                // not perturb the recorded error string.
                ufbxi_check!(
                    uc,
                    unsafe {
                        ufbxi_warnf!(
                            uc,
                            WarningType::MissingExternalFile,
                            "Failed to open geometry cache: %s",
                            (*file).filename.data
                        )
                    }
                    .is_ok(),
                    "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_MISSING_EXTERNAL_FILE, ~0u, \"Failed to open geometry cache: %s\", file->filename.data)"
                );
                return Ok(());
            } else {
                cc.error_view().set_type_(ErrorType::ExternalFileNotFound);
                cc.error_view()
                    .description_view()
                    .set_data(b"External file not found\0".as_ptr());
                // SAFETY: the `strlen` argument is a NUL-terminated byte
                // literal.
                cc.error_view()
                    .description_view()
                    .set_length(unsafe { strlen(b"External file not found\0".as_ptr()) });
            }
        }

        // SAFETY: both pointers address their context's own `Error` field —
        // distinct objects — and `Error` is plain data, so the read leaves
        // `cc`'s copy usable (it is not read again).
        unsafe { core::ptr::write(uc.error_mut_ptr(), core::ptr::read(cc.error_mut_ptr())) };
        return Err(Fail);
    }

    // SAFETY: the caller's contract is that `file` points at a live, writable
    // `ExternalFile`.
    unsafe { (*file).data = cache as *mut c_void };
    Ok(())
}

// ufbx.c:24862-24865 (`#else` branch of `UFBXI_FEATURE_GEOMETRY_CACHE`)
#[cfg(not(feature = "geometry-cache"))]
#[inline(never)]
pub(crate) fn load_external_cache(uc: &Context, file: *mut ExternalFile) -> Result<(), Fail> {
    // C: `file` is unreferenced in the `#else` arm.
    let _ = file;
    if uc.opts_view().ignore_missing_external_files() {
        return Ok(());
    }

    // SAFETY: `uc.error_mut_ptr()` addresses the context's own live `Error`,
    // unaliased here, and the format string is a literal.
    unsafe { ufbxi_fmt_err_info!(uc.error_mut_ptr(), "UFBX_ENABLE_GEOMETRY_CACHE") };
    ufbxi_fail_msg!(uc, "UFBXI_FEATURE_GEOMETRY_CACHE", "Feature disabled");
}

// ufbx.c:24869-24876 `ufbxi_find_external_file`
#[inline(never)]
pub(crate) unsafe fn find_external_file(
    files: *mut ExternalFile,
    num_files: usize,
    type_: ExternalFileType,
    name: *const u8,
) -> *mut ExternalFile {
    let mut ix: usize = usize::MAX;
    // SAFETY (every deref in the two predicates): the search hands each
    // predicate a pointer to an element of the `files` run, so the deref is in
    // bounds; `strcmp` compares that element's NUL-terminated interned
    // filename against the caller's NUL-terminated `name`.
    let less = |a: *const ExternalFile| {
        if type_ != unsafe { (*a).type_ } {
            type_ < unsafe { (*a).type_ }
        } else {
            unsafe { crate::native::error::strcmp((*a).filename.data, name) < 0 }
        }
    };
    let equal =
        |a: *const ExternalFile| unsafe { (*a).type_ == type_ && (*a).filename.data == name };
    // SAFETY: the caller's contract is that `files` addresses `num_files` live
    // `ExternalFile`s sorted by the same key the predicates test — what the
    // binary search requires.
    unsafe {
        macro_lower_bound_eq::<ExternalFile>(32, &mut ix, files, 0, num_files, less, equal);
    }
    if ix != usize::MAX {
        // SAFETY: `ix < num_files` whenever the search set it, so this is an
        // element of the caller's run.
        unsafe { files.add(ix) }
    } else {
        core::ptr::null_mut()
    }
}

// ufbx.c:24878-24944 `ufbxi_load_external_files`
#[inline(never)]
pub(crate) fn load_external_files(uc: &Context) -> Result<(), Fail> {
    let mut num_files: usize = 0;

    // Gather external files to deduplicate them
    // C: `ufbxi_for_ptr_list(ufbx_cache_file, p_cache, uc->scene.cache_files)`
    // SAFETY: walking the stored `cache_files` element-pointer run of the
    // uc-owned scene (write-provenance arena, `count` entries); each pushed
    // `file` is the fresh non-null result of a push onto uc's own tmp stack.
    unsafe {
        let mut p_cache: *mut *mut CacheFile =
            uc.scene_view().cache_files_view().data() as *mut *mut CacheFile;
        let p_cache_end: *mut *mut CacheFile =
            add_ptr(p_cache, uc.scene_view().cache_files_view().count());
        while p_cache != p_cache_end {
            let cache: *mut CacheFile = *p_cache;
            if (*cache).filename.length > 0 {
                let file: *mut ExternalFile = uc.tmp_stack_view().push_zero(1);
                ufbxi_check!(uc, !file.is_null(), "file");
                // C: `file->index = num_files++;`
                (*file).index = num_files;
                num_files += 1;
                (*file).type_ = ExternalFileType::GeometryCache;
                (*file).filename = (*cache).filename;
                (*file).absolute_filename = (*cache).absolute_filename;
            }
            p_cache = p_cache.add(1);
        }
    }

    // Sort and load the external files
    // Pops the `num_files` entries pushed above from uc's tmp stack into
    // uc's own tmp buffer.
    let files: *mut ExternalFile = uc.tmp_view().push_pop(uc.tmp_stack_view(), num_files);
    ufbxi_check!(uc, !files.is_null(), "files");
    // SAFETY: sorts the fresh non-null run in place with the matching
    // element size.
    unsafe {
        unstable_sort(
            files as *mut c_void,
            num_files,
            size_of::<ExternalFile>(),
            less_external_file,
            core::ptr::null_mut(),
        );
    }

    let mut prev_type: ExternalFileType = ExternalFileType::GeometryCache;
    let mut prev_name: *const u8 = core::ptr::null();
    // C: `ufbxi_for(ufbxi_external_file, file, files, num_files)`
    // SAFETY: walking the fresh `num_files`-element run materialized above;
    // `load_external_cache` receives an in-bounds element of it.
    unsafe {
        let mut file: *mut ExternalFile = files;
        let file_end: *mut ExternalFile = add_ptr(file, num_files);
        while file != file_end {
            if (*file).filename.data == prev_name && (*file).type_ == prev_type {
                file = file.add(1);
                continue;
            }
            if (*file).type_ == ExternalFileType::GeometryCache {
                load_external_cache(uc, file)?;
            }
            prev_name = (*file).filename.data;
            prev_type = (*file).type_;
            file = file.add(1);
        }
    }

    // Patch the loaded files
    // C: `ufbxi_for_ptr_list(ufbx_cache_file, p_cache, uc->scene.cache_files)`
    // SAFETY: same stored `cache_files` run as above; `find_external_file`
    // searches the `num_files` run materialized above; `(*file).data` is
    // null-checked before the `Ref` wrap.
    unsafe {
        let mut p_cache: *mut *mut CacheFile =
            uc.scene_view().cache_files_view().data() as *mut *mut CacheFile;
        let p_cache_end: *mut *mut CacheFile =
            add_ptr(p_cache, uc.scene_view().cache_files_view().count());
        while p_cache != p_cache_end {
            let cache: *mut CacheFile = *p_cache;
            let file: *mut ExternalFile = find_external_file(
                files,
                num_files,
                ExternalFileType::GeometryCache,
                (*cache).filename.data,
            );
            if !file.is_null() && !(*file).data.is_null() {
                (*cache).external_cache = Some(Ref::from_ptr((*file).data as *mut GeometryCache));
            }
            p_cache = p_cache.add(1);
        }
    }

    // Patch the geometry deformers
    // C: `ufbxi_for_ptr_list(ufbx_cache_deformer, p_deformer, uc->scene.cache_deformers)`
    // SAFETY: walking the stored `cache_deformers` element-pointer run of the
    // uc-owned scene; every deref below is either null-checked (`opt_ptr`
    // results) or an in-bounds element of the deformer's channel list
    // (`count == 1` fast path / lower-bound hit `ix < count`).
    unsafe {
        let mut p_deformer: *mut *mut CacheDeformer =
            uc.scene_view().cache_deformers_view().data() as *mut *mut CacheDeformer;
        let p_deformer_end: *mut *mut CacheDeformer =
            add_ptr(p_deformer, uc.scene_view().cache_deformers_view().count());
        while p_deformer != p_deformer_end {
            let deformer: *mut CacheDeformer = *p_deformer;
            let file: *mut CacheFile = opt_ptr(&(*deformer).file);
            if file.is_null() || opt_ptr(&(*file).external_cache).is_null() {
                p_deformer = p_deformer.add(1);
                continue;
            }
            let cache: *mut GeometryCache = opt_ptr(&(*file).external_cache);
            (*deformer).external_cache = Some(Ref::from_ptr(cache));

            // HACK: It seems like channels may be connected even if the name is wrong
            // and they work when exporting from Marvelous to Maya...
            if (*cache).channels.count == 1 {
                (*deformer).external_channel = Some(Ref::from_ptr(
                    ((*cache).channels.data as *mut CacheChannel).add(0),
                ));
            } else {
                let channel: String = (*deformer).channel;
                // C: `size_t ix = SIZE_MAX;` — pre-initialized because
                // `ufbxi_macro_lower_bound_eq` does NOT write the out-param on a miss.
                let mut ix: usize = usize::MAX;
                macro_lower_bound_eq::<CacheChannel>(
                    16,
                    &mut ix,
                    (*cache).channels.data,
                    0,
                    (*cache).channels.count,
                    |a: *const CacheChannel| str_less_raw((*a).name, channel),
                    |a: *const CacheChannel| (*a).name.data == channel.data,
                );
                if ix != usize::MAX {
                    (*deformer).external_channel = Some(Ref::from_ptr(
                        ((*cache).channels.data as *mut CacheChannel).add(ix),
                    ));
                }
            }
            p_deformer = p_deformer.add(1);
        }
    }

    Ok(())
}

// ufbx.c:24946-24981 `ufbxi_transform_to_axes`
#[inline(never)]
pub(crate) fn transform_to_axes(uc: &Context, dst_axes: CoordinateAxes) {
    if !coordinate_axes_valid(uc.scene_view().settings_view().axes()) {
        return;
    }
    // SAFETY: writes uc's own `axis_matrix` storage through its raw-ptr
    // getter; the axes arguments are by-value.
    if !unsafe {
        axis_matrix(
            uc.axis_matrix_mut_ptr(),
            uc.scene_view().settings_view().axes(),
            dst_axes,
        )
    } {
        return;
    }

    // SAFETY: pure value math over a local copy of the matrix.
    if unsafe { matrix_determinant(&uc.axis_matrix()) } < 0.0f32 as Real {
        if uc.opts_view().handedness_conversion_axis() != MirrorAxis::None {
            let mirror_axis: MirrorAxis = uc.opts_view().handedness_conversion_axis();
            uc.set_mirror_axis(mirror_axis);
            uc.scene_view()
                .metadata_view()
                .set_mirror_axis(uc.mirror_axis());

            // SAFETY: in-place update of uc's own `axis_matrix` storage,
            // then a pure value read of it.
            unsafe {
                mirror_matrix_dst(uc.axis_matrix_mut_ptr(), uc.mirror_axis());
                ufbxi_dev_assert!(matrix_determinant(&uc.axis_matrix()) >= 0.0f32 as Real);
            }

            // C: `ufbxi_for_ptr_list(ufbx_node, p_node, uc->scene.nodes)`
            // SAFETY: walking the stored `nodes` element-pointer run of the
            // uc-owned scene (write-provenance arena, `count` entries).
            unsafe {
                let mut p_node: *mut *mut Node =
                    uc.scene_view().nodes_view().data() as *mut *mut Node;
                let p_node_end: *mut *mut Node =
                    add_ptr(p_node, uc.scene_view().nodes_view().count());
                while p_node != p_node_end {
                    let node: *mut Node = *p_node;
                    if !(*node).is_root {
                        (*node).adjust_mirror_axis = mirror_axis;
                    }
                    p_node = p_node.add(1);
                }
            }
        }
    }

    if uc.opts_view().space_conversion() == SpaceConversion::TransformRoot {
        let mut axis_mat: Matrix = uc.axis_matrix();
        // SAFETY: `root_node` is the scene's stored (always-resolved) root
        // reference from the uc arena; the matrix helpers are pure value math
        // over locals and the root's own transform fields.
        unsafe {
            let root_node: *mut Node = ref_ptr(uc.scene_view().root_node_ptr());
            if !is_transform_identity(&(*root_node).local_transform) {
                let root_mat: Matrix = transform_to_matrix(&(*root_node).local_transform);
                axis_mat = matrix_mul(&root_mat, &axis_mat);
            }

            mirror_matrix(&mut axis_mat, uc.mirror_axis());

            (*root_node).local_transform = matrix_to_transform(&axis_mat);
            (*root_node).node_to_parent = axis_mat;
        }
    }
}

// ufbx.c:24983-25010 `ufbxi_scale_units`
#[inline(never)]
pub(crate) fn scale_units(uc: &Context, mut target_meters: Real) -> Result<(), Fail> {
    if uc.scene_view().settings_view().unit_meters() <= 0.0f32 as Real {
        return Ok(());
    }
    // SAFETY: scans the static `POW10_TARGETS` array with its own length.
    target_meters =
        unsafe { round_if_near(POW10_TARGETS.as_ptr(), POW10_TARGETS.len(), target_meters) };

    let mut ratio: Real = uc.scene_view().settings_view().unit_meters() / target_meters;
    // SAFETY: scans the same static array with its own length.
    ratio = unsafe { round_if_near(POW10_TARGETS.as_ptr(), POW10_TARGETS.len(), ratio) };
    if ratio == 1.0f32 as Real {
        return Ok(());
    }

    uc.set_unit_scale(ratio);

    if uc.opts_view().space_conversion() == SpaceConversion::TransformRoot {
        // SAFETY: `root_node` is the scene's stored (always-resolved) root
        // reference from the uc arena; the writes stay within its fields.
        unsafe {
            let root_node: *mut Node = ref_ptr(uc.scene_view().root_node_ptr());
            (*root_node).local_transform.scale.x *= ratio;
            (*root_node).local_transform.scale.y *= ratio;
            (*root_node).local_transform.scale.z *= ratio;
            (*root_node).node_to_parent.m00 *= ratio;
            (*root_node).node_to_parent.m01 *= ratio;
            (*root_node).node_to_parent.m02 *= ratio;
            (*root_node).node_to_parent.m10 *= ratio;
            (*root_node).node_to_parent.m11 *= ratio;
            (*root_node).node_to_parent.m12 *= ratio;
            (*root_node).node_to_parent.m20 *= ratio;
            (*root_node).node_to_parent.m21 *= ratio;
            (*root_node).node_to_parent.m22 *= ratio;
        }
    }

    Ok(())
}

// CONTINUATION POINT: `// -- Geometry caches` (ufbx.c:23946-24785) and
// `// -- External files` (ufbx.c:24787-25010) are ported in FULL, both feature
// arms. `ufbxi_load_external_files`, `ufbxi_transform_to_axes` and
// `ufbxi_scale_units` still have no callers — all three are called only from
// `ufbxi_load_imp` (ufbx.c:25204+, unported).
//
// Next banner: ufbx.c:25012 `// -- Curve evaluation` (owned by
// `native/evaluate.rs`).
