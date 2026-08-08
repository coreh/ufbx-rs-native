//! Port of the `// -- Geometry caches` banner section (ufbx.c:23945+).
//! Phase 1: only `ufbxi_geometry_cache_imp` and
//! `ufbxi_free_geometry_cache_imp` are ported (both feature branches) — they
//! are required by `ufbxi_release_ref` (`native::api::release_ref`). The rest
//! of the geometry-cache machinery is NOT YET PORTED.
#![allow(dead_code)]

#[cfg(feature = "geometry-cache")]
use crate::native::allocator::CACHE_IMP_MAGIC;
#[cfg(feature = "geometry-cache")]
use crate::native::buf::{buf_free, Buf};
use crate::native::parse::Refcount;
#[cfg(feature = "geometry-cache")]
use crate::native::platform::ufbx_assert;

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

// ufbx.c:24781-24783 `ufbxi_free_geometry_cache_imp` (`#else` branch — feature disabled)
#[cfg(not(feature = "geometry-cache"))]
#[inline(always)]
pub(crate) unsafe fn free_geometry_cache_imp(imp: *mut GeometryCacheImp) {
    let _ = imp;
}
