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
    coordinate_axes_valid, init_ref, matrix_determinant, matrix_mul, matrix_to_transform,
    transform_to_matrix, EMPTY_STRING,
};
use crate::native::buf::{buf_free, push, push_pop, push_zero, Buf};
use crate::native::error::{
    clear_error, fix_error_type, set_err_info, strlen, ufbxi_check, ufbxi_check_err,
    ufbxi_check_err_msg, ufbxi_fail_err, ufbxi_fail_err_msg, ufbxi_fail_msg, ufbxi_fmt_err_info,
    ufbxi_report_err_msg, ufbxi_snprintf, Fail, EMPTY_CHAR,
};
use crate::native::hash::map_init;
use crate::native::parse::{is_transform_identity, r#match, Context, InnerContext, Refcount};
use crate::native::platform::{
    add_ptr, macro_lower_bound_eq, min32, min64, min_sz, read_f32, read_u32, stable_sort, to_size,
    ufbx_assert, ufbxi_dev_assert, ufbxi_regression_assert, unstable_sort, MAX_SKIP_SIZE,
};
use crate::native::read::{open_file, opt_ptr, ref_ptr};
use crate::native::scene_process::{
    axis_matrix, mirror_matrix, mirror_matrix_dst, round_if_near, POW10_TARGETS,
};
use crate::native::string_pool::{
    map_cmp_string, push_string_place_str, str_cmp, str_equal, str_less, string_pool_temp_free,
    StringPool,
};
use crate::native::warnings::ufbxi_warnf;
#[cfg(feature = "geometry-cache")]
use crate::native::xml::{
    free_xml, load_xml, xml_find_attrib, xml_find_child, XmlDocument, XmlLoadOpts, XmlTag,
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
pub(crate) struct CacheContext(core::cell::UnsafeCell<core::mem::MaybeUninit<InnerCacheContext>>);

// Typed interior-mutable VIEW over the `opts` field, reinterpreted in place
// (approach A). Generated ABI-fixed `RawGeometryCacheOpts` plays the `Inner` role;
// `MaybeUninit` makes forming `&GeometryCacheOptsView` assert no validity — each leaf getter
// asserts only the field it reads.
#[repr(transparent)]
pub(crate) struct GeometryCacheOptsView(
    core::cell::UnsafeCell<core::mem::MaybeUninit<RawGeometryCacheOpts>>,
);

impl GeometryCacheOptsView {
    #[inline(always)]
    fn get(&self) -> *mut RawGeometryCacheOpts {
        self.0.get().cast()
    }

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
#[repr(transparent)]
pub(crate) struct GeometryCacheView(core::cell::UnsafeCell<core::mem::MaybeUninit<GeometryCache>>);

impl GeometryCacheView {
    #[inline(always)]
    fn get(&self) -> *mut GeometryCache {
        self.0.get().cast()
    }

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

impl CacheContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerCacheContext {
        self.0.get().cast()
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
    let buffered: usize = min_sz(to_size(cc.pos_end().offset_from(cc.pos())), size);
    core::ptr::copy_nonoverlapping(cc.pos(), dst as *mut u8, buffered);
    cc.set_pos(cc.pos().add(buffered));
    size -= buffered;
    cc.set_file_offset(cc.file_offset().wrapping_add(buffered as u64));
    if size == 0 {
        return Ok(());
    }
    dst = (dst as *mut u8).add(buffered) as *mut c_void;

    if size >= size_of_val(&(*cc.get()).buffer) {
        let num_read: usize =
            ((*cc.get()).stream.read_fn.unwrap_unchecked())((*cc.get()).stream.user, dst, size);
        ufbxi_check_err_msg!(
            cc.error_mut_ptr(),
            num_read <= size,
            "IO error",
            "num_read <= size"
        );
        if !allow_eof {
            ufbxi_check_err_msg!(
                cc.error_mut_ptr(),
                num_read == size,
                "Truncated file",
                "num_read == size"
            );
        }
        cc.set_file_offset(cc.file_offset().wrapping_add(num_read as u64));
        size -= num_read;
        dst = (dst as *mut u8).add(num_read) as *mut c_void;
    } else {
        let num_read: usize = ((*cc.get()).stream.read_fn.unwrap_unchecked())(
            (*cc.get()).stream.user,
            (*cc.get()).buffer.as_mut_ptr() as *mut c_void,
            size_of_val(&(*cc.get()).buffer),
        );
        ufbxi_check_err_msg!(
            cc.error_mut_ptr(),
            num_read <= size_of_val(&(*cc.get()).buffer),
            "IO error",
            "num_read <= sizeof(cc->buffer)"
        );
        if !allow_eof {
            ufbxi_check_err_msg!(
                cc.error_mut_ptr(),
                num_read >= size,
                "Truncated file",
                "num_read >= size"
            );
        }
        cc.set_pos((*cc.get()).buffer.as_ptr());
        cc.set_pos_end(
            (*cc.get())
                .buffer
                .as_ptr()
                .add(size_of_val(&(*cc.get()).buffer)),
        );

        core::ptr::copy_nonoverlapping(cc.pos(), dst as *mut u8, size);
        cc.set_pos(cc.pos().add(size));
        cc.set_file_offset(cc.file_offset().wrapping_add(size as u64));

        let num_written: usize = min_sz(size, num_read);
        size -= num_written;
        dst = (dst as *mut u8).add(num_written) as *mut c_void;
    }

    if size > 0 {
        core::ptr::write_bytes(dst as *mut u8, 0, size);
    }

    Ok(())
}

// ufbx.c:24080-24116 `ufbxi_cache_skip`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_skip(cc: &CacheContext, mut size: u64) -> Result<(), Fail> {
    // C-parity: `cc->file_offset += size;` is uint64_t addition, which wraps.
    // `ufbxi_cache_load_pc2` (ufbx.c:24270) passes `total_points * 12 - 1`
    // guarded only by `total_points < UINT64_MAX / 12` (ufbx.c:24262), so a
    // crafted PC2 header reaches this with `size` near `UINT64_MAX`.
    cc.set_file_offset(cc.file_offset().wrapping_add(size));

    let buffered: u64 = min64(cc.pos_end().offset_from(cc.pos()) as u64, size);
    cc.set_pos(cc.pos().add(buffered as usize));
    size -= buffered;

    if (*cc.get()).stream.skip_fn.is_some() {
        while size >= MAX_SKIP_SIZE as u64 {
            size -= MAX_SKIP_SIZE as u64;
            ufbxi_check_err_msg!(
                cc.error_mut_ptr(),
                ((*cc.get()).stream.skip_fn.unwrap_unchecked())(
                    (*cc.get()).stream.user,
                    MAX_SKIP_SIZE - 1
                ),
                "Truncated file",
                "cc->stream.skip_fn(cc->stream.user, UFBXI_MAX_SKIP_SIZE - 1)"
            );

            // Check that we can read at least one byte in case the file is broken
            // and causes us to seek indefinitely forwards as `fseek()` does not
            // report if we hit EOF...
            let mut single_byte = MaybeUninit::<[u8; 1]>::uninit(); // ufbxi_uninit
            let num_read: usize = ((*cc.get()).stream.read_fn.unwrap_unchecked())(
                (*cc.get()).stream.user,
                single_byte.as_mut_ptr() as *mut c_void,
                1,
            );
            ufbxi_check_err_msg!(
                cc.error_mut_ptr(),
                num_read <= 1,
                "IO error",
                "num_read <= 1"
            );
            ufbxi_check_err_msg!(
                cc.error_mut_ptr(),
                num_read == 1,
                "Truncated file",
                "num_read == 1"
            );
        }

        if size > 0 {
            ufbxi_check_err_msg!(
                cc.error_mut_ptr(),
                ((*cc.get()).stream.skip_fn.unwrap_unchecked())(
                    (*cc.get()).stream.user,
                    size as usize
                ),
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
                cc.error_mut_ptr(),
                ((*cc.get()).stream.read_fn.unwrap_unchecked())(
                    (*cc.get()).stream.user,
                    skip_buf.as_mut_ptr() as *mut c_void,
                    to_skip
                ) != 0,
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
pub(crate) unsafe fn cache_mc_read_tag(cc: &CacheContext, p_tag: *mut u32) -> Result<(), Fail> {
    let mut buf = MaybeUninit::<[u8; 4]>::uninit(); // ufbxi_uninit
    let buf: *mut u8 = buf.as_mut_ptr() as *mut u8;
    cache_read(cc, buf as *mut c_void, 4, true)?;
    *p_tag = (*buf.add(0) as u32) << 24u32
        | (*buf.add(1) as u32) << 16
        | (*buf.add(2) as u32) << 8u32
        | (*buf.add(3) as u32);
    if *p_tag == cache_mc_tag(b'F', b'O', b'R', b'8') {
        cc.set_mc_for8(true);
    }
    Ok(())
}

// ufbx.c:24131-24140 `ufbxi_cache_mc_read_u32`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_mc_read_u32(cc: &CacheContext, p_value: *mut u32) -> Result<(), Fail> {
    let mut buf = MaybeUninit::<[u8; 4]>::uninit(); // ufbxi_uninit
    let buf: *mut u8 = buf.as_mut_ptr() as *mut u8;
    cache_read(cc, buf as *mut c_void, 4, false)?;
    *p_value = (*buf.add(0) as u32) << 24u32
        | (*buf.add(1) as u32) << 16
        | (*buf.add(2) as u32) << 8u32
        | (*buf.add(3) as u32);
    if cc.mc_for8() {
        cache_read(cc, buf as *mut c_void, 4, false)?;
    }
    Ok(())
}

// ufbx.c:24142-24156 `ufbxi_cache_mc_read_u64`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_mc_read_u64(cc: &CacheContext, p_value: *mut u64) -> Result<(), Fail> {
    if !cc.mc_for8() {
        let mut v32 = MaybeUninit::<u32>::uninit(); // ufbxi_uninit
        cache_mc_read_u32(cc, v32.as_mut_ptr())?;
        *p_value = v32.assume_init() as u64;
    } else {
        let mut buf = MaybeUninit::<[u8; 8]>::uninit(); // ufbxi_uninit
        let buf: *mut u8 = buf.as_mut_ptr() as *mut u8;
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
    Ok(())
}

// ufbx.c:24158-24160 `ufbxi_cache_data_format_size`
#[cfg(feature = "geometry-cache")]
static CACHE_DATA_FORMAT_SIZE: [u8; 5] = [0, 4, 12, 8, 24];

// ufbx.c:24162-24243 `ufbxi_cache_load_mc`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_load_mc(cc: &CacheContext) -> Result<(), Fail> {
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
        let mut tag = MaybeUninit::<u32>::uninit(); // ufbxi_uninit
        let mut size = MaybeUninit::<u64>::uninit(); // ufbxi_uninit
        cache_mc_read_tag(cc, tag.as_mut_ptr())?;
        let tag: u32 = tag.assume_init();
        if tag == 0 {
            break;
        }

        if tag == TAG_CACH || tag == TAG_MYCH {
            continue;
        }
        if cc.mc_for8() {
            cache_read(cc, skip_buf.as_mut_ptr() as *mut c_void, 4, false)?;
        }

        cache_mc_read_u64(cc, size.as_mut_ptr())?;
        let size: u64 = size.assume_init();
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
                    cc.error_mut_ptr(),
                    size > 0 && size < usize::MAX as u64,
                    "size > 0 && size < SIZE_MAX"
                );
                let length: usize = size as usize - 1;
                let padded_length: usize =
                    (size as usize).wrapping_add(alignment).wrapping_sub(1) & !(alignment - 1);
                ufbxi_check_err!(
                    cc.error_mut_ptr(),
                    grow_array::<u8>(
                        cc.ator_tmp(),
                        cc.name_buf_mut_ptr(),
                        cc.name_cap_mut_ptr(),
                        padded_length
                    ),
                    "ufbxi_grow_array_size((cc->ator_tmp), sizeof(**(&cc->name_buf)), (&cc->name_buf), (&cc->name_cap), (padded_length))"
                );
                cache_read(cc, cc.name_buf() as *mut c_void, padded_length, false)?;
                (*cc.get()).channel_name.data = cc.name_buf();
                (*cc.get()).channel_name.length = length;
                push_string_place_str(cc.string_pool_mut_ptr(), cc.channel_name_mut_ptr(), false)?;
            }
            TAG_SIZE => cache_mc_read_u32(cc, &mut count)?,
            TAG_FVCA => format = CacheDataFormat::Vec3Float,
            TAG_DVCA => format = CacheDataFormat::Vec3Double,
            TAG_FBCA => format = CacheDataFormat::RealFloat,
            TAG_DBCA => format = CacheDataFormat::RealDouble,
            TAG_DBLA => format = CacheDataFormat::RealDouble,
            _ => ufbxi_fail_err!(cc.error_mut_ptr(), "Unknown tag"),
        }

        if format != CacheDataFormat::Unknown {
            let frame: *mut CacheFrame = push_zero(cc.tmp_stack_mut_ptr(), 1);
            ufbxi_check_err!(cc.error_mut_ptr(), !frame.is_null(), "frame");

            let elem_size: u32 = CACHE_DATA_FORMAT_SIZE[format as u32 as usize] as u32;
            let total_size: u64 = elem_size as u64 * count as u64;
            // C: `size >= elem_size * count` — `uint32_t * uint32_t` wraps mod
            // 2^32 BEFORE the comparison widens it to `uint64_t`.
            ufbxi_check_err!(
                cc.error_mut_ptr(),
                size >= elem_size.wrapping_mul(count) as u64,
                "size >= elem_size * count"
            );

            (*frame).channel = (*cc.get()).channel_name;
            (*frame).time = time as f64 * (1.0 / 6000.0);
            (*frame).filename = (*cc.get()).stream_filename;
            (*frame).data_format = format;
            (*frame).data_encoding = CacheDataEncoding::BigEndian;
            (*frame).data_offset = cc.file_offset();
            (*frame).data_count = count;
            (*frame).data_element_bytes = elem_size;
            (*frame).data_total_bytes = total_size;
            (*frame).file_format = CacheFileFormat::Mc;

            let end: u64 = begin.wrapping_add(
                size.wrapping_add(alignment as u64).wrapping_sub(1) & !((alignment - 1) as u64),
            );
            ufbxi_check_err!(
                cc.error_mut_ptr(),
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
pub(crate) unsafe fn cache_load_pc2(cc: &CacheContext) -> Result<(), Fail> {
    let mut header = MaybeUninit::<[u8; 32]>::uninit(); // ufbxi_uninit
    let header: *mut u8 = header.as_mut_ptr() as *mut u8;
    cache_read(cc, header as *mut c_void, size_of::<[u8; 32]>(), false)?;

    let version: u32 = read_u32(header.add(12));
    let num_points: u32 = read_u32(header.add(16));
    let start_frame: f64 = read_f32(header.add(20)) as f64;
    let frames_per_sample: f64 = read_f32(header.add(24)) as f64;
    let num_samples: u32 = read_u32(header.add(28));

    let _ = version;

    let frames: *mut CacheFrame = push_zero(cc.tmp_stack_mut_ptr(), num_samples as usize);
    ufbxi_check_err!(cc.error_mut_ptr(), !frames.is_null(), "frames");

    let total_points: u64 = num_points as u64 * num_samples as u64;
    ufbxi_check_err!(
        cc.error_mut_ptr(),
        total_points < u64::MAX / 12,
        "total_points < UINT64_MAX / 12"
    );

    let mut offset: u64 = cc.file_offset();

    // Skip almost to the end of the data and try to read one byte as there's
    // nothing after the data so we can't detect EOF..
    if total_points > 0 {
        let mut last_byte = MaybeUninit::<[u8; 1]>::uninit(); // ufbxi_uninit
        cache_skip(cc, total_points * 12 - 1)?;
        cache_read(cc, last_byte.as_mut_ptr() as *mut c_void, 1, false)?;
    }

    let mut i: u32 = 0;
    while i < num_samples {
        let frame: *mut CacheFrame = frames.add(i as usize);

        let sample_frame: f64 = start_frame + i as f64 * frames_per_sample;
        (*frame).channel = (*cc.get()).channel_name;
        (*frame).time = sample_frame / cc.frames_per_second();
        (*frame).filename = (*cc.get()).stream_filename;
        (*frame).data_format = CacheDataFormat::Vec3Float;
        (*frame).data_encoding = CacheDataEncoding::LittleEndian;
        (*frame).data_offset = offset;
        (*frame).data_count = num_points;
        (*frame).data_element_bytes = 12;
        // C: `num_points * 12` is `uint32_t` arithmetic (wraps mod 2^32) that
        // is only then widened to `uint64_t`.
        (*frame).data_total_bytes = num_points.wrapping_mul(12) as u64;
        (*frame).file_format = CacheFileFormat::Pc2;
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
    str_less((*a).name, (*b).name)
}

// ufbx.c:24301-24306 `ufbxi_cache_sort_tmp_channels`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_sort_tmp_channels(
    cc: &CacheContext,
    channels: *mut CacheTmpChannel,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check_err!(
        cc.error_mut_ptr(),
        grow_array::<u8>(
            cc.ator_tmp(),
            cc.tmp_arr_mut_ptr(),
            cc.tmp_arr_size_mut_ptr(),
            count * size_of::<CacheTmpChannel>()
        ),
        "ufbxi_grow_array_size((cc->ator_tmp), sizeof(**(&cc->tmp_arr)), (&cc->tmp_arr), (&cc->tmp_arr_size), (count * sizeof(ufbxi_cache_tmp_channel)))"
    );
    stable_sort(
        size_of::<CacheTmpChannel>(),
        16,
        channels as *mut c_void,
        cc.tmp_arr() as *mut c_void,
        count,
        tmp_channel_less,
        core::ptr::null_mut(),
    );
    Ok(())
}

// ufbx.c:24308-24394 `ufbxi_cache_load_xml_imp`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_load_xml_imp(
    cc: &CacheContext,
    doc: *mut XmlDocument,
) -> Result<(), Fail> {
    cc.set_xml_ticks_per_frame(250);
    (*cc.get()).xml_filename = (*cc.get()).stream_filename;

    let tag_root: *mut XmlTag = xml_find_child((*doc).root, b"Autodesk_Cache_File\0".as_ptr());
    if !tag_root.is_null() {
        let tag_type: *mut XmlTag = xml_find_child(tag_root, b"cacheType\0".as_ptr());
        let tag_fps: *mut XmlTag = xml_find_child(tag_root, b"cacheTimePerFrame\0".as_ptr());
        let tag_channels: *mut XmlTag = xml_find_child(tag_root, b"Channels\0".as_ptr());

        let mut num_extra: usize = 0;
        // C: `ufbxi_for(ufbxi_xml_tag, tag, tag_root->children, tag_root->num_children)`
        let mut tag: *mut XmlTag = (*tag_root).children;
        let tag_end: *mut XmlTag = add_ptr(tag, (*tag_root).num_children);
        while tag != tag_end {
            if (*tag).num_children != 1 {
                tag = tag.add(1);
                continue;
            }
            if crate::native::error::strcmp((*tag).name.data, b"extra\0".as_ptr()) != 0 {
                tag = tag.add(1);
                continue;
            }
            let extra: *mut String = push(cc.tmp_stack_mut_ptr(), 1);
            ufbxi_check_err!(cc.error_mut_ptr(), !extra.is_null(), "extra");
            *extra = (*(*tag).children.add(0)).text;
            push_string_place_str(cc.string_pool_mut_ptr(), extra, false)?;
            num_extra += 1;
            tag = tag.add(1);
        }
        cc.cache_view().extra_info_view().set_count(num_extra);
        cc.cache_view()
            .extra_info_view()
            .set_data(push_pop::<String>(
                cc.result_mut_ptr(),
                cc.tmp_stack_mut_ptr(),
                num_extra,
            ));
        ufbxi_check_err!(
            cc.error_mut_ptr(),
            !cc.cache_view().extra_info_view().data().is_null(),
            "cc->cache.extra_info.data"
        );

        if !tag_type.is_null() {
            let type_ = xml_find_attrib(tag_type, b"Type\0".as_ptr());
            let format = xml_find_attrib(tag_type, b"Format\0".as_ptr());
            if !type_.is_null() {
                if crate::native::error::strcmp((*type_).value.data, b"OneFilePerFrame\0".as_ptr())
                    == 0
                {
                    (*cc.get()).xml_type = CacheXmlType::FilePerFrame;
                } else if crate::native::error::strcmp((*type_).value.data, b"OneFile\0".as_ptr())
                    == 0
                {
                    (*cc.get()).xml_type = CacheXmlType::SingleFile;
                }
            }
            if !format.is_null() {
                if crate::native::error::strcmp((*format).value.data, b"mcc\0".as_ptr()) == 0 {
                    (*cc.get()).xml_format = CacheXmlFormat::Mcc;
                } else if crate::native::error::strcmp((*format).value.data, b"mcx\0".as_ptr()) == 0
                {
                    (*cc.get()).xml_format = CacheXmlFormat::Mcx;
                }
            }
        }

        if !tag_fps.is_null() {
            let fps = xml_find_attrib(tag_fps, b"TimePerFrame\0".as_ptr());
            if !fps.is_null() {
                let value: u32 =
                    crate::native::float_parse::parse_uint32_radix((*fps).value.data, 10);
                if value > 0 {
                    cc.set_xml_ticks_per_frame(value);
                }
            }
        }

        if !tag_channels.is_null() {
            cc.set_channels(push_zero(cc.tmp_mut_ptr(), (*tag_channels).num_children));
            ufbxi_check_err!(cc.error_mut_ptr(), !cc.channels().is_null(), "cc->channels");

            // C: `ufbxi_for(ufbxi_xml_tag, tag, tag_channels->children, tag_channels->num_children)`
            let mut tag: *mut XmlTag = (*tag_channels).children;
            let tag_end: *mut XmlTag = add_ptr(tag, (*tag_channels).num_children);
            while tag != tag_end {
                let name = xml_find_attrib(tag, b"ChannelName\0".as_ptr());
                let type_ = xml_find_attrib(tag, b"ChannelType\0".as_ptr());
                let interpretation = xml_find_attrib(tag, b"ChannelInterpretation\0".as_ptr());
                if !(!name.is_null() && !type_.is_null() && !interpretation.is_null()) {
                    tag = tag.add(1);
                    continue;
                }

                // C: `&cc->channels[cc->num_channels++]`
                let channel: *mut CacheTmpChannel = cc.channels().add(cc.num_channels());
                cc.set_num_channels(cc.num_channels() + 1);
                (*channel).name = (*name).value;
                (*channel).interpretation = (*interpretation).value;
                push_string_place_str(cc.string_pool_mut_ptr(), &mut (*channel).name, false)?;
                push_string_place_str(
                    cc.string_pool_mut_ptr(),
                    &mut (*channel).interpretation,
                    false,
                )?;

                let sampling_rate = xml_find_attrib(tag, b"SamplingRate\0".as_ptr());
                let start_time = xml_find_attrib(tag, b"StartTime\0".as_ptr());
                let end_time = xml_find_attrib(tag, b"EndTime\0".as_ptr());
                if !sampling_rate.is_null() && !start_time.is_null() && !end_time.is_null() {
                    (*channel).sample_rate = crate::native::float_parse::parse_uint32_radix(
                        (*sampling_rate).value.data,
                        10,
                    );
                    (*channel).start_time = crate::native::float_parse::parse_uint32_radix(
                        (*start_time).value.data,
                        10,
                    );
                    (*channel).end_time =
                        crate::native::float_parse::parse_uint32_radix((*end_time).value.data, 10);
                    (*channel).current_time = (*channel).start_time;
                    (*channel).try_load = true;
                }
                tag = tag.add(1);
            }
        }
    }

    cache_sort_tmp_channels(cc, cc.channels(), cc.num_channels())?;
    Ok(())
}

// ufbx.c:24396-24412 `ufbxi_cache_load_xml`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_load_xml(cc: &CacheContext) -> Result<(), Fail> {
    // C: `ufbxi_xml_load_opts opts = { 0 };`
    let mut opts: XmlLoadOpts = core::mem::zeroed();
    opts.ator = cc.ator_tmp();
    opts.read_fn = (*cc.get()).stream.read_fn;
    opts.read_user = (*cc.get()).stream.user;
    opts.prefix = cc.pos();
    opts.prefix_length = to_size(cc.pos_end().offset_from(cc.pos()));
    let doc: *mut XmlDocument = load_xml(&mut opts, cc.error_mut_ptr());
    ufbxi_check_err!(cc.error_mut_ptr(), !doc.is_null(), "doc");

    let xml_ok = cache_load_xml_imp(cc, doc);
    free_xml(doc);
    ufbxi_check_err!(cc.error_mut_ptr(), xml_ok.is_ok(), "xml_ok");

    Ok(())
}

// ufbx.c:24414-24437 `ufbxi_cache_load_file`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_load_file(cc: &CacheContext, filename: String) -> Result<(), Fail> {
    (*cc.get()).stream_filename = filename;
    push_string_place_str(
        cc.string_pool_mut_ptr(),
        cc.stream_filename_mut_ptr(),
        false,
    )?;

    // Assume all files have at least 16 bytes of header
    let magic_len: usize = ((*cc.get()).stream.read_fn.unwrap_unchecked())(
        (*cc.get()).stream.user,
        (*cc.get()).buffer.as_mut_ptr() as *mut c_void,
        16,
    );
    ufbxi_check_err_msg!(
        cc.error_mut_ptr(),
        magic_len <= 16,
        "IO error",
        "magic_len <= 16"
    );
    ufbxi_check_err_msg!(
        cc.error_mut_ptr(),
        magic_len == 16,
        "Truncated file",
        "magic_len == 16"
    );
    cc.set_pos((*cc.get()).buffer.as_ptr());
    cc.set_pos_end((*cc.get()).buffer.as_ptr().add(16));

    cc.set_file_offset(0);

    if crate::native::error::memcmp((*cc.get()).buffer.as_ptr(), b"POINTCACHE2".as_ptr(), 11) == 0 {
        cache_load_pc2(cc)?;
    } else if crate::native::error::memcmp((*cc.get()).buffer.as_ptr(), b"FOR4".as_ptr(), 4) == 0
        || crate::native::error::memcmp((*cc.get()).buffer.as_ptr(), b"FOR8".as_ptr(), 4) == 0
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
    core::ptr::write_bytes(cc.stream_mut_ptr() as *mut RawStream, 0, 1);
    ufbxi_regression_assert!(strlen(filename.data) == filename.length);
    if !open_file(
        &(*cc.get()).open_file_cb,
        cc.stream_mut_ptr(),
        filename.data,
        filename.length,
        original_filename,
        cc.ator_tmp(),
        OpenFileType::GeometryCache,
    ) {
        return Ok(());
    }

    let ok = cache_load_file(cc, filename);
    *p_found = true;

    if let Some(close_fn) = (*cc.get()).stream.close_fn {
        close_fn((*cc.get()).stream.user);
    }

    ok
}

// ufbx.c:24457-24540 `ufbxi_cache_load_frame_files`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_load_frame_files(cc: &CacheContext) -> Result<(), Fail> {
    if (*cc.get()).xml_filename.length == 0 {
        return Ok(());
    }

    let extension: *const u8;
    match (*cc.get()).xml_format {
        CacheXmlFormat::Mcc => extension = b"mc\0".as_ptr(),
        CacheXmlFormat::Mcx => extension = b"mcx\0".as_ptr(),
        _ => return Ok(()),
    }

    // Ensure worst case space for `path/filenameFrame123Tick456.mcx`
    let name_buf_len: usize = (*cc.get()).xml_filename.length + 64;
    let name_buf: *mut u8 = push(cc.tmp_mut_ptr(), name_buf_len);
    ufbxi_check_err!(cc.error_mut_ptr(), !name_buf.is_null(), "name_buf");

    // Find the prefix before `.xml`
    let mut prefix_len: usize = (*cc.get()).xml_filename.length;
    let mut i: usize = prefix_len;
    while i > 0 {
        if *(*cc.get()).xml_filename.data.add(i - 1) == b'.' {
            prefix_len = i - 1;
            break;
        }
        i -= 1;
    }
    core::ptr::copy_nonoverlapping((*cc.get()).xml_filename.data, name_buf, prefix_len);

    let suffix_data: *mut u8 = name_buf.add(prefix_len);
    let suffix_len: usize = name_buf_len - prefix_len;

    // C: `ufbx_string filename;` — both members are written before any read,
    // and upstream carries no partial-init marker here.
    let mut filename = MaybeUninit::<String>::uninit();
    let filename: *mut String = filename.as_mut_ptr();
    (*filename).data = name_buf;

    if (*cc.get()).xml_type == CacheXmlType::SingleFile {
        (*filename).length =
            prefix_len + ufbxi_snprintf!(suffix_data, suffix_len, ".%s", extension) as usize;
        let mut found: bool = false;
        cache_try_open_file(cc, *filename, core::ptr::null(), &mut found)?;
    } else if (*cc.get()).xml_type == CacheXmlType::FilePerFrame {
        let mut lowest_time: u32 = 0;
        loop {
            // Find the first `time >= lowest_time` value that has data in some channel
            let mut time: u32 = u32::MAX;
            // C: `ufbxi_for(ufbxi_cache_tmp_channel, chan, cc->channels, cc->num_channels)`
            let mut chan: *mut CacheTmpChannel = cc.channels();
            let chan_end: *mut CacheTmpChannel = add_ptr(chan, cc.num_channels());
            while chan != chan_end {
                if !(*chan).try_load || (*chan).consecutive_fails > 10 {
                    chan = chan.add(1);
                    continue;
                }
                let sample_rate: u32 = if (*chan).sample_rate != 0 {
                    (*chan).sample_rate
                } else {
                    cc.xml_ticks_per_frame()
                };
                if (*chan).current_time < lowest_time {
                    let delta: u32 = (lowest_time - (*chan).current_time - 1) / sample_rate;
                    (*chan).current_time = (*chan)
                        .current_time
                        .wrapping_add(delta.wrapping_mul(sample_rate));
                    if u32::MAX - (*chan).current_time >= sample_rate {
                        (*chan).current_time = (*chan).current_time.wrapping_add(sample_rate);
                    } else {
                        (*chan).try_load = false;
                        chan = chan.add(1);
                        continue;
                    }
                }
                if (*chan).current_time <= (*chan).end_time {
                    time = min32(time, (*chan).current_time);
                }
                chan = chan.add(1);
            }
            if time == u32::MAX {
                break;
            }

            // Try to load a file at the specified frame/tick
            let frame: u32 = time / cc.xml_ticks_per_frame();
            let tick: u32 = time % cc.xml_ticks_per_frame();
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
            let mut found: bool = false;
            cache_try_open_file(cc, *filename, core::ptr::null(), &mut found)?;

            // Update channel status
            // C: `ufbxi_for(ufbxi_cache_tmp_channel, chan, cc->channels, cc->num_channels)`
            let mut chan: *mut CacheTmpChannel = cc.channels();
            let chan_end: *mut CacheTmpChannel = add_ptr(chan, cc.num_channels());
            while chan != chan_end {
                if (*chan).current_time == time {
                    (*chan).consecutive_fails = if found {
                        0
                    } else {
                        (*chan).consecutive_fails.wrapping_add(1)
                    };
                }
                chan = chan.add(1);
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
    if (*a).channel.data != (*b).channel.data {
        // Channel names should be interned
        ufbxi_regression_assert!(!str_equal((*a).channel, (*b).channel));
        return str_less((*a).channel, (*b).channel);
    }
    (*a).time < (*b).time
}

// ufbx.c:24554-24559 `ufbxi_cache_sort_frames`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_sort_frames(
    cc: &CacheContext,
    frames: *mut CacheFrame,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check_err!(
        cc.error_mut_ptr(),
        grow_array::<u8>(
            cc.ator_tmp(),
            cc.tmp_arr_mut_ptr(),
            cc.tmp_arr_size_mut_ptr(),
            count * size_of::<CacheFrame>()
        ),
        "ufbxi_grow_array_size((cc->ator_tmp), sizeof(**(&cc->tmp_arr)), (&cc->tmp_arr), (&cc->tmp_arr_size), (count * sizeof(ufbx_cache_frame)))"
    );
    stable_sort(
        size_of::<CacheFrame>(),
        16,
        frames as *mut c_void,
        cc.tmp_arr() as *mut c_void,
        count,
        cmp_cache_frame_less,
        core::ptr::null_mut(),
    );
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
pub(crate) unsafe fn cache_setup_channels(cc: &CacheContext) -> Result<(), Fail> {
    let mut tmp_chan: *mut CacheTmpChannel = cc.channels();
    let tmp_end: *mut CacheTmpChannel = add_ptr(tmp_chan, cc.num_channels());

    let mut begin: usize = 0;
    let mut num_channels: usize = 0;
    while begin < cc.cache_view().frames_view().count() {
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

        let chan: *mut CacheChannel = push_zero(cc.tmp_stack_mut_ptr(), 1);
        ufbxi_check_err!(cc.error_mut_ptr(), !chan.is_null(), "chan");

        (*chan).name = (*frame).channel;
        (*chan).interpretation_name = EMPTY_STRING.0;
        (*chan).frames.data = frame;
        (*chan).frames.count = end - begin;

        while tmp_chan < tmp_end && str_less((*tmp_chan).name, (*chan).name) {
            tmp_chan = tmp_chan.add(1);
        }
        if tmp_chan < tmp_end && str_equal((*tmp_chan).name, (*chan).name) {
            (*chan).interpretation_name = (*tmp_chan).interpretation;
        }

        if (*frame).file_format == CacheFileFormat::Pc2 {
            (*chan).interpretation = CacheInterpretation::VertexPosition;
        } else {
            // C: `ufbxi_for(const ufbxi_cache_interpretation_name, name, ufbxi_cache_interpretation_names, ufbxi_arraycount(...))`
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

        let mut mirror_axis: MirrorAxis = MirrorAxis::None;
        let mut scale_factor: Real = 1.0f32 as Real;
        if (*chan).interpretation != CacheInterpretation::Unknown {
            mirror_axis = cc.opts_view().mirror_axis();
            if cc.opts_view().use_scale_factor() {
                scale_factor = cc.opts_view().scale_factor();
            }
        }
        (*chan).mirror_axis = mirror_axis;
        (*chan).scale_factor = scale_factor;
        // C: `ufbxi_for_list(ufbx_cache_frame, f, chan->frames)`
        let mut f: *mut CacheFrame = (*chan).frames.data as *mut CacheFrame;
        let f_end: *mut CacheFrame = add_ptr(f, (*chan).frames.count);
        while f != f_end {
            (*f).mirror_axis = mirror_axis;
            (*f).scale_factor = scale_factor;
            f = f.add(1);
        }

        num_channels += 1;
        begin = end;
    }

    cc.cache_view()
        .channels_view()
        .set_data(push_pop::<CacheChannel>(
            cc.result_mut_ptr(),
            cc.tmp_stack_mut_ptr(),
            num_channels,
        ));
    ufbxi_check_err!(
        cc.error_mut_ptr(),
        !cc.cache_view().channels_view().data().is_null(),
        "cc->cache.channels.data"
    );
    cc.cache_view().channels_view().set_count(num_channels);

    Ok(())
}

// ufbx.c:24637-24691 `ufbxi_cache_load_imp`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_load_imp(cc: &CacheContext, filename: String) -> Result<(), Fail> {
    (*cc.get()).tmp.ator = cc.ator_tmp();
    (*cc.get()).tmp_stack.ator = cc.ator_tmp();

    (*cc.get()).channel_name.data = EMPTY_CHAR.as_ptr();

    if (*cc.get()).open_file_cb.fn_.is_none() {
        (*cc.get()).open_file_cb.fn_ = Some(crate::native::api::default_open_file);
    }

    // Make sure the filename we pass to `open_file_fn()` is NULL-terminated
    let filename_data: *mut u8 = push(cc.tmp_mut_ptr(), filename.length + 1);
    ufbxi_check_err!(
        cc.error_mut_ptr(),
        !filename_data.is_null(),
        "filename_data"
    );
    core::ptr::copy_nonoverlapping(filename.data, filename_data, filename.length);
    *filename_data.add(filename.length) = b'\0';
    let filename_copy: String = String::new_c(filename_data, filename.length);

    // TODO: NULL termination!
    let mut found: bool = false;
    cache_try_open_file(cc, filename_copy, core::ptr::null(), &mut found)?;
    if !found {
        set_err_info(cc.error_mut_ptr(), filename.data, filename.length);
        ufbxi_fail_err_msg!(cc.error_mut_ptr(), "open_file_fn()", "File not found");
    }

    cc.cache_view()
        .set_root_filename((*cc.get()).stream_filename);

    cache_load_frame_files(cc)?;

    let num_frames: usize = (*cc.get()).tmp_stack.num_items;
    cc.cache_view().frames_view().set_count(num_frames);
    cc.cache_view()
        .frames_view()
        .set_data(push_pop::<CacheFrame>(
            cc.result_mut_ptr(),
            cc.tmp_stack_mut_ptr(),
            num_frames,
        ));
    ufbxi_check_err!(
        cc.error_mut_ptr(),
        !cc.cache_view().frames_view().data().is_null(),
        "cc->cache.frames.data"
    );

    cache_sort_frames(
        cc,
        cc.cache_view().frames_view().data() as *mut CacheFrame,
        cc.cache_view().frames_view().count(),
    )?;
    cache_setup_channels(cc)?;

    // Must be last allocation!
    cc.set_imp(push(cc.result_mut_ptr(), 1));
    ufbxi_check_err!(cc.error_mut_ptr(), !cc.imp().is_null(), "cc->imp");

    // Expose the wide allocation so `get_imp` can recover this header from a
    // (possibly narrowed) public `&GeometryCache` pointer via exposed provenance.
    (cc.imp() as *mut u8).expose_provenance();

    init_ref(
        &mut (*cc.imp()).refcount,
        CACHE_IMP_MAGIC,
        core::ptr::null_mut(),
    );

    core::ptr::write(&mut (*cc.imp()).cache, core::ptr::read(&(*cc.get()).cache));
    (*cc.imp()).magic = CACHE_IMP_MAGIC;
    (*cc.imp()).owned_by_scene = cc.owned_by_scene();
    (*cc.imp()).refcount.ator = (*cc.get()).ator_result;
    (*cc.imp()).refcount.buf = (*cc.get()).result;
    (*cc.imp()).refcount.buf.ator = &raw mut (*cc.imp()).refcount.ator;
    (*cc.imp()).string_buf = (*cc.get()).string_pool.buf;
    (*cc.imp()).string_buf.ator = &raw mut (*cc.imp()).refcount.ator;

    Ok(())
}

// ufbx.c:24693-24716 `ufbxi_cache_load`
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn cache_load(cc: &CacheContext, filename: String) -> *mut GeometryCache {
    let ok = cache_load_imp(cc, filename).is_ok();

    buf_free(cc.tmp_mut_ptr());
    buf_free(cc.tmp_stack_mut_ptr());
    free::<u8>(cc.ator_tmp(), cc.name_buf(), cc.name_cap());
    free::<u8>(cc.ator_tmp(), cc.tmp_arr(), cc.tmp_arr_size());
    if !cc.owned_by_scene() {
        string_pool_temp_free(cc.string_pool_mut_ptr());
        free_ator(cc.ator_tmp());
    }

    if ok {
        &raw mut (*cc.imp()).cache
    } else {
        fix_error_type(
            cc.error_mut_ptr(),
            b"Failed to load geometry cache\0".as_ptr(),
            core::ptr::null_mut(),
        );
        if !cc.owned_by_scene() {
            buf_free(&mut (*cc.get()).string_pool.buf);
            free_ator(cc.ator_result_mut_ptr());
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
        core::ptr::read(user_opts)
    } else {
        core::mem::zeroed()
    };

    // C: `ufbxi_cache_context cc = { UFBX_ERROR_NONE };` / `ufbxi_allocator ator_tmp = { 0 };`
    let cc: CacheContext = core::mem::zeroed();
    let mut ator_tmp: Allocator = core::mem::zeroed();
    init_ator(
        cc.error_mut_ptr(),
        &mut ator_tmp,
        &opts.temp_allocator,
        b"temp\0".as_ptr(),
    );
    init_ator(
        cc.error_mut_ptr(),
        cc.ator_result_mut_ptr(),
        &opts.result_allocator,
        b"result\0".as_ptr(),
    );
    cc.set_ator_tmp(&mut ator_tmp);

    (*cc.get()).opts = core::ptr::read(&opts);

    (*cc.get()).open_file_cb = opts.open_file_cb;

    (*cc.get()).string_pool.error = cc.error_mut_ptr();
    map_init(
        &mut (*cc.get()).string_pool.map,
        cc.ator_tmp(),
        map_cmp_string,
        core::ptr::null_mut(),
    );
    (*cc.get()).string_pool.buf.ator = cc.ator_result_mut_ptr();
    (*cc.get()).string_pool.buf.unordered = true;
    (*cc.get()).string_pool.initial_size = 64;
    (*cc.get()).result.ator = cc.ator_result_mut_ptr();

    cc.set_frames_per_second(if opts.frames_per_second > 0.0 {
        opts.frames_per_second
    } else {
        30.0
    });

    let cache: *mut GeometryCache = cache_load(&cc, filename);
    if !p_error.is_null() {
        if !cache.is_null() {
            clear_error(p_error);
        } else {
            core::ptr::write(p_error, core::ptr::read(&(*cc.get()).error));
        }
    }
    cache
}

// ufbx.c:24757-24761 `ufbxi_free_geometry_cache_imp` (UFBXI_FEATURE_GEOMETRY_CACHE)
#[cfg(feature = "geometry-cache")]
#[inline(never)]
pub(crate) unsafe fn free_geometry_cache_imp(imp: *mut GeometryCacheImp) {
    ufbx_assert!((*imp).magic == CACHE_IMP_MAGIC);
    buf_free(&mut (*imp).string_buf);
}

// ufbx.c:24765-24769 `ufbxi_geometry_cache_imp` (`#else` branch — feature disabled)
#[cfg(not(feature = "geometry-cache"))]
#[repr(C)]
pub(crate) struct GeometryCacheImp {
    pub refcount: Refcount,
    pub magic: u32,
    pub owned_by_scene: bool,
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
        core::ptr::write_bytes(p_error as *mut u8, 0, size_of::<Error>());
        ufbxi_fmt_err_info!(p_error, "UFBX_ENABLE_GEOMETRY_CACHE");
        ufbxi_report_err_msg!(p_error, "UFBXI_FEATURE_GEOMETRY_CACHE", "Feature disabled");
    }
    core::ptr::null_mut()
}

// ufbx.c:24781-24783 `ufbxi_free_geometry_cache_imp` (`#else` branch — feature disabled)
#[cfg(not(feature = "geometry-cache"))]
#[inline(always)]
pub(crate) unsafe fn free_geometry_cache_imp(imp: *mut GeometryCacheImp) {
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
    if (*a).type_ != (*b).type_ {
        return (*a).type_ < (*b).type_;
    }
    let cmp: i32 = str_cmp((*a).filename, (*b).filename);
    if cmp != 0 {
        return cmp < 0;
    }
    if (*a).index != (*b).index {
        return (*a).index < (*b).index;
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
    let cc: CacheContext = core::mem::zeroed();
    cc.set_owned_by_scene(true);

    (*cc.get()).open_file_cb = (*uc.get()).opts.open_file_cb;
    cc.set_frames_per_second((*uc.get()).scene.settings.frames_per_second);

    // Temporarily "borrow" allocators for the geometry cache
    cc.set_ator_tmp(uc.ator_tmp_mut_ptr());
    (*cc.get()).string_pool = (*uc.get()).string_pool;
    (*cc.get()).result = (*uc.get()).result;

    cc.opts_view().set_mirror_axis((*uc.get()).mirror_axis);
    cc.opts_view().set_use_scale_factor(true);
    cc.opts_view()
        .set_scale_factor((*uc.get()).scene.metadata.geometry_scale);

    let mut cache: *mut GeometryCache = cache_load(&cc, (*file).filename);
    if cache.is_null() {
        if cc.error_view().type_() == ErrorType::FileNotFound {
            core::ptr::write_bytes(cc.error_mut_ptr() as *mut Error, 0, 1);
            cache = cache_load(&cc, (*file).absolute_filename);
        }
    }

    // Return the "borrowed" allocators
    (*uc.get()).string_pool = (*cc.get()).string_pool;
    (*uc.get()).result = (*cc.get()).result;

    if cache.is_null() {
        if cc.error_view().type_() == ErrorType::FileNotFound {
            if (*uc.get()).opts.ignore_missing_external_files {
                ufbxi_check!(
                    uc,
                    ufbxi_warnf!(
                        uc,
                        WarningType::MissingExternalFile,
                        "Failed to open geometry cache: %s",
                        (*file).filename.data
                    )
                    .is_ok(),
                    "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_MISSING_EXTERNAL_FILE, ~0u, \"Failed to open geometry cache: %s\", file->filename.data)"
                );
                return Ok(());
            } else {
                cc.error_view().set_type_(ErrorType::ExternalFileNotFound);
                cc.error_view()
                    .description_view()
                    .set_data(b"External file not found\0".as_ptr());
                cc.error_view()
                    .description_view()
                    .set_length(strlen(b"External file not found\0".as_ptr()));
            }
        }

        core::ptr::write(uc.error_mut_ptr(), core::ptr::read(&(*cc.get()).error));
        return Err(Fail);
    }

    (*file).data = cache as *mut c_void;
    Ok(())
}

// ufbx.c:24862-24865 (`#else` branch of `UFBXI_FEATURE_GEOMETRY_CACHE`)
#[cfg(not(feature = "geometry-cache"))]
#[inline(never)]
pub(crate) unsafe fn load_external_cache(
    uc: &Context,
    file: *mut ExternalFile,
) -> Result<(), Fail> {
    // C: `file` is unreferenced in the `#else` arm.
    let _ = file;
    if (*uc.get()).opts.ignore_missing_external_files {
        return Ok(());
    }

    ufbxi_fmt_err_info!(uc.error_ptr(), "UFBX_ENABLE_GEOMETRY_CACHE");
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
    macro_lower_bound_eq::<ExternalFile>(
        32,
        &mut ix,
        files,
        0,
        num_files,
        |a: *const ExternalFile| {
            if type_ != (*a).type_ {
                type_ < (*a).type_
            } else {
                crate::native::error::strcmp((*a).filename.data, name) < 0
            }
        },
        |a: *const ExternalFile| (*a).type_ == type_ && (*a).filename.data == name,
    );
    if ix != usize::MAX {
        files.add(ix)
    } else {
        core::ptr::null_mut()
    }
}

// ufbx.c:24878-24944 `ufbxi_load_external_files`
#[inline(never)]
pub(crate) unsafe fn load_external_files(uc: &Context) -> Result<(), Fail> {
    let mut num_files: usize = 0;

    // Gather external files to deduplicate them
    // C: `ufbxi_for_ptr_list(ufbx_cache_file, p_cache, uc->scene.cache_files)`
    let mut p_cache: *mut *mut CacheFile =
        (*uc.get()).scene.cache_files.data as *mut *mut CacheFile;
    let p_cache_end: *mut *mut CacheFile = add_ptr(p_cache, (*uc.get()).scene.cache_files.count);
    while p_cache != p_cache_end {
        let cache: *mut CacheFile = *p_cache;
        if (*cache).filename.length > 0 {
            let file: *mut ExternalFile = push_zero(uc.tmp_stack_mut_ptr(), 1);
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

    // Sort and load the external files
    let files: *mut ExternalFile = push_pop(uc.tmp_mut_ptr(), uc.tmp_stack_mut_ptr(), num_files);
    ufbxi_check!(uc, !files.is_null(), "files");
    unstable_sort(
        files as *mut c_void,
        num_files,
        size_of::<ExternalFile>(),
        less_external_file,
        core::ptr::null_mut(),
    );

    let mut prev_type: ExternalFileType = ExternalFileType::GeometryCache;
    let mut prev_name: *const u8 = core::ptr::null();
    // C: `ufbxi_for(ufbxi_external_file, file, files, num_files)`
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

    // Patch the loaded files
    // C: `ufbxi_for_ptr_list(ufbx_cache_file, p_cache, uc->scene.cache_files)`
    let mut p_cache: *mut *mut CacheFile =
        (*uc.get()).scene.cache_files.data as *mut *mut CacheFile;
    let p_cache_end: *mut *mut CacheFile = add_ptr(p_cache, (*uc.get()).scene.cache_files.count);
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

    // Patch the geometry deformers
    // C: `ufbxi_for_ptr_list(ufbx_cache_deformer, p_deformer, uc->scene.cache_deformers)`
    let mut p_deformer: *mut *mut CacheDeformer =
        (*uc.get()).scene.cache_deformers.data as *mut *mut CacheDeformer;
    let p_deformer_end: *mut *mut CacheDeformer =
        add_ptr(p_deformer, (*uc.get()).scene.cache_deformers.count);
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
                |a: *const CacheChannel| str_less((*a).name, channel),
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

    Ok(())
}

// ufbx.c:24946-24981 `ufbxi_transform_to_axes`
#[inline(never)]
pub(crate) unsafe fn transform_to_axes(uc: &Context, dst_axes: CoordinateAxes) {
    if !coordinate_axes_valid((*uc.get()).scene.settings.axes) {
        return;
    }
    if !axis_matrix(
        uc.axis_matrix_mut_ptr(),
        (*uc.get()).scene.settings.axes,
        dst_axes,
    ) {
        return;
    }

    if matrix_determinant(&(*uc.get()).axis_matrix) < 0.0f32 as Real {
        if (*uc.get()).opts.handedness_conversion_axis != MirrorAxis::None {
            let mirror_axis: MirrorAxis = (*uc.get()).opts.handedness_conversion_axis;
            (*uc.get()).mirror_axis = mirror_axis;
            (*uc.get()).scene.metadata.mirror_axis = (*uc.get()).mirror_axis;

            mirror_matrix_dst(uc.axis_matrix_mut_ptr(), (*uc.get()).mirror_axis);
            ufbxi_dev_assert!(matrix_determinant(&(*uc.get()).axis_matrix) >= 0.0f32 as Real);

            // C: `ufbxi_for_ptr_list(ufbx_node, p_node, uc->scene.nodes)`
            let mut p_node: *mut *mut Node = (*uc.get()).scene.nodes.data as *mut *mut Node;
            let p_node_end: *mut *mut Node = add_ptr(p_node, (*uc.get()).scene.nodes.count);
            while p_node != p_node_end {
                let node: *mut Node = *p_node;
                if !(*node).is_root {
                    (*node).adjust_mirror_axis = mirror_axis;
                }
                p_node = p_node.add(1);
            }
        }
    }

    if (*uc.get()).opts.space_conversion == SpaceConversion::TransformRoot {
        let mut axis_mat: Matrix = (*uc.get()).axis_matrix;
        let root_node: *mut Node = ref_ptr(&(*uc.get()).scene.root_node);
        if !is_transform_identity(&(*root_node).local_transform) {
            let root_mat: Matrix = transform_to_matrix(&(*root_node).local_transform);
            axis_mat = matrix_mul(&root_mat, &axis_mat);
        }

        mirror_matrix(&mut axis_mat, (*uc.get()).mirror_axis);

        (*root_node).local_transform = matrix_to_transform(&axis_mat);
        (*root_node).node_to_parent = axis_mat;
    }
}

// ufbx.c:24983-25010 `ufbxi_scale_units`
#[inline(never)]
pub(crate) unsafe fn scale_units(uc: &Context, mut target_meters: Real) -> Result<(), Fail> {
    if (*uc.get()).scene.settings.unit_meters <= 0.0f32 as Real {
        return Ok(());
    }
    target_meters = round_if_near(POW10_TARGETS.as_ptr(), POW10_TARGETS.len(), target_meters);

    let mut ratio: Real = (*uc.get()).scene.settings.unit_meters / target_meters;
    ratio = round_if_near(POW10_TARGETS.as_ptr(), POW10_TARGETS.len(), ratio);
    if ratio == 1.0f32 as Real {
        return Ok(());
    }

    uc.set_unit_scale(ratio);

    if (*uc.get()).opts.space_conversion == SpaceConversion::TransformRoot {
        let root_node: *mut Node = ref_ptr(&(*uc.get()).scene.root_node);
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
