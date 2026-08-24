//! Port of the `// -- Subdivision` banner section (ufbx.c:28821-30082).
//!
//! The whole section is gated on `UFBXI_FEATURE_SUBDIVISION`
//! (`#[cfg(feature = "subdivision")]`); the `#else` arm at ufbx.c:30069-30081
//! keeps `ufbxi_subdivide_mesh` present and reporting
//! `UFBX_ERROR_FEATURE_DISABLED`, ported here as well so the module's contract
//! is complete in both feature configurations.
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
#[cfg(feature = "subdivision")]
use crate::generated::{
    ColorSet, Edge, Error, Face, Mesh, MeshPart, RawSubdivideOpts, SkinDeformer, SkinVertex,
    SkinWeight, SubdivisionBoundary, SubdivisionResult, SubdivisionWeight, SubdivisionWeightRange,
    TopoEdge, TopoFlags, UvSet, VertexAttrib, VertexReal, VertexVec3,
};
#[cfg(not(feature = "subdivision"))]
use crate::generated::{Error, Mesh, RawSubdivideOpts};
#[cfg(feature = "subdivision")]
use crate::generated::{Vec2, Vec3, Vec4};
#[cfg(feature = "subdivision")]
use crate::native::allocator::{free, free_ator, grow_array, init_ator, Allocator};
#[cfg(feature = "subdivision")]
use crate::native::api::{
    compute_normals, compute_topology, generate_normal_mapping, get_vertex_real,
    topo_next_vertex_edge, topo_prev_vertex_edge, ZERO_VEC3,
};
#[cfg(feature = "subdivision")]
use crate::native::buf::{buf_free, push_size, Buf};
#[cfg(feature = "subdivision")]
use crate::native::error::{fix_error_type, memcmp, ufbxi_check_err, ufbxi_check_return_err};
#[cfg(not(feature = "subdivision"))]
use crate::native::error::{ufbxi_fmt_err_info, ufbxi_report_err_msg};
#[cfg(feature = "subdivision")]
use crate::native::parse::{finish_imp, FinishedImp, ImpHandle, MeshImp, Refcount, SceneImp};
#[cfg(feature = "subdivision")]
use crate::native::platform::{
    max_sz, min_sz, ufbx_assert, ufbxi_dev_assert, ufbxi_unreachable, unstable_sort, NO_INDEX,
};
#[cfg(feature = "subdivision")]
use crate::native::read::{
    finalize_mesh, opt_ptr, opt_ref, patch_mesh_reals, ref_ptr, update_face_groups,
};
#[cfg(feature = "subdivision")]
use crate::native::scene_process::finalize_mesh_material;
#[cfg(feature = "subdivision")]
use crate::native::string_pool::slow_normalize3;
#[cfg(feature = "subdivision")]
use crate::native::view::view_raw_mut;
use crate::native::view::{view_read, view_write};
#[cfg(feature = "subdivision")]
use crate::native::view::{Const, Mode, SliceViewIter, View};
#[cfg(feature = "subdivision")]
use crate::prelude::{ListView, Real};
#[cfg(feature = "subdivision")]
use core::ffi::c_void;
#[cfg(feature = "subdivision")]
use core::mem::{size_of, MaybeUninit};

// -- Subdivision

// ufbx.c:28825-28828 `ufbxi_subdivide_input`
#[cfg(feature = "subdivision")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct SubdivideInput {
    pub data: *const c_void,
    pub weight: Real,
}

// ufbx.c:28830 `typedef int ufbxi_subdivide_sum_fn(void *user, void *output,
// const ufbxi_subdivide_input *inputs, size_t num_inputs);`
// C passes function designators (`&ufbxi_subdivide_sum_vec3`) — fn pointers,
// never closures (PORTING.md "Callbacks"). The `int` return is the C check
// convention (0 = failure), kept verbatim so `ufbxi_check_err(sum_fn(...))`
// ports 1:1.
#[cfg(feature = "subdivision")]
pub(crate) type SubdivideSumFn = unsafe extern "C" fn(
    user: *mut c_void,
    output: *mut c_void,
    inputs: *const SubdivideInput,
    num_inputs: usize,
) -> i32;

// ufbx.c:28832-28846 `ufbxi_subdivide_layer_input`
// C: `sum_fn` is a possibly-NULL function pointer (`ufbxi_real_sum_fns[0]`),
// hence `Option<...>` — the niche makes the layout identical.
#[cfg(feature = "subdivision")]
#[repr(C)]
pub(crate) struct SubdivideLayerInput {
    pub sum_fn: Option<SubdivideSumFn>,
    pub sum_user: *mut c_void,

    pub values: *const c_void,
    pub stride: usize,

    pub indices: *const u32,

    pub check_split_data: bool,
    pub ignore_indices: bool,

    pub boundary: SubdivisionBoundary,
}

// ufbx.c:28848-28854 `ufbxi_subdivide_layer_output`
#[cfg(feature = "subdivision")]
#[repr(C)]
pub(crate) struct SubdivideLayerOutput {
    pub values: *mut c_void,
    pub num_values: usize,
    pub indices: *mut u32,
    pub num_indices: usize,
    pub unique_per_vertex: bool,
}

// ufbx.c:28856-28859 `ufbxi_subdivision_vertex_weights`
#[cfg(feature = "subdivision")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct SubdivisionVertexWeights {
    pub weights: *mut SubdivisionWeight,
    pub num_weights: usize,
}

// ufbx.c:28861-28889 `ufbxi_subdivide_context`
#[cfg(feature = "subdivision")]
#[repr(C)]
pub(crate) struct InnerSubdivideContext {
    pub imp: *mut MeshImp,

    pub error: Error,

    pub src_mesh_ptr: *mut Mesh,
    pub src_mesh: Mesh,
    pub dst_mesh: Mesh,
    pub topo: *mut TopoEdge,
    pub num_topo: usize,

    pub opts: RawSubdivideOpts,

    pub ator_result: Allocator,
    pub ator_tmp: Allocator,

    pub result: Buf,
    pub tmp: Buf,
    pub source: Buf,

    pub inputs: *mut SubdivideInput,
    pub inputs_cap: usize,

    pub tmp_vertex_weights: *mut Real,
    pub tmp_weights: *mut SubdivisionWeight,
    pub total_weights: usize,
    pub max_vertex_weights: usize,
}

// Safe `&SubdivideContext` handle over the fields-struct `InnerSubdivideContext`,
// mirroring the `Context`/`InnerContext` seam in `parse.rs`. `MaybeUninit` because
// it embeds the public `Mesh` (enum-bearing) in `src_mesh`/`dst_mesh`, so a plain
// `&InnerSubdivideContext` could not be formed soundly; `UnsafeCell` gives the
// interior mutability every `&SubdivideContext` site relies on. The type-erased
// `sum_user` task-callback pointer round-trips through the wrapper address.
#[repr(transparent)]
#[cfg(feature = "subdivision")]
pub(crate) struct SubdivideContext(
    core::cell::UnsafeCell<core::mem::MaybeUninit<InnerSubdivideContext>>,
);

// Typed interior-mutable view over the `opts` field, reinterpreted in place.
// Generated ABI-fixed `RawSubdivideOpts` plays the inner-storage role;
// `MaybeUninit` makes forming `&SubdivideOptsView` assert no validity — each leaf getter
// asserts only the field it reads.
pub(crate) type SubdivideOptsView = crate::native::view::View<RawSubdivideOpts>;

impl SubdivideOptsView {
    #[inline(always)]
    pub(crate) fn boundary(&self) -> crate::generated::SubdivisionBoundary {
        view_read!(self, boundary)
    }

    #[inline(always)]
    pub(crate) fn evaluate_skin_weights(&self) -> bool {
        view_read!(self, evaluate_skin_weights)
    }

    #[inline(always)]
    pub(crate) fn evaluate_source_vertices(&self) -> bool {
        view_read!(self, evaluate_source_vertices)
    }

    #[inline(always)]
    pub(crate) fn ignore_normals(&self) -> bool {
        view_read!(self, ignore_normals)
    }

    #[inline(always)]
    pub(crate) fn interpolate_normals(&self) -> bool {
        view_read!(self, interpolate_normals)
    }

    #[inline(always)]
    pub(crate) fn interpolate_tangents(&self) -> bool {
        view_read!(self, interpolate_tangents)
    }

    #[inline(always)]
    pub(crate) fn max_skin_weights(&self) -> usize {
        view_read!(self, max_skin_weights)
    }

    #[inline(always)]
    pub(crate) fn max_source_vertices(&self) -> usize {
        view_read!(self, max_source_vertices)
    }

    #[inline(always)]
    pub(crate) fn result_allocator(&self) -> crate::generated::RawAllocatorOpts {
        view_read!(self, result_allocator)
    }

    #[inline(always)]
    pub(crate) fn skin_deformer_index(&self) -> usize {
        view_read!(self, skin_deformer_index)
    }

    #[inline(always)]
    pub(crate) fn temp_allocator(&self) -> crate::generated::RawAllocatorOpts {
        view_read!(self, temp_allocator)
    }

    #[inline(always)]
    pub(crate) fn uv_boundary(&self) -> crate::generated::SubdivisionBoundary {
        view_read!(self, uv_boundary)
    }

    #[inline(always)]
    pub(crate) fn set_boundary(&self, boundary: crate::generated::SubdivisionBoundary) {
        view_write!(self, boundary, boundary)
    }

    #[inline(always)]
    pub(crate) fn set_uv_boundary(&self, uv_boundary: crate::generated::SubdivisionBoundary) {
        view_write!(self, uv_boundary, uv_boundary)
    }
}

// Typed interior-mutable VIEW over `VertexVec3` (non-Copy: has List fields);
// field accessors are generated (src/generated_views.rs).
pub(crate) type VertexVec3View = crate::native::view::View<crate::generated::VertexVec3>;

// Typed interior-mutable VIEW over a `Mesh` field (public struct); field
// accessors are generated (src/generated_views.rs).
pub(crate) type MeshView = crate::native::view::View<crate::generated::Mesh>;

impl MeshView {
    // `vertex_position` — typed VIEW handle over the aggregate field (the
    // generated projection `vertex_position()` is the same lens; this alias
    // keeps the concrete `&VertexVec3View` type at existing call sites).
    #[inline(always)]
    pub(crate) fn vertex_position_view(&self) -> &VertexVec3View {
        unsafe { &*(&raw mut (*self.get()).vertex_position as *mut VertexVec3View) }
    }
}

#[cfg(feature = "subdivision")]
impl SubdivideContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerSubdivideContext {
        self.0.get().cast()
    }

    #[inline(always)]
    /// Moves the field out by bitwise read (`ptr::read`). C does this as plain
    /// struct assignment; the source field still holds the stale bits (no
    /// `Drop`), so the caller must overwrite it or treat it as moved-from.
    pub(crate) fn take_result(&self) -> crate::native::buf::Buf {
        unsafe { core::ptr::read(&raw const (*self.get()).result) }
    }

    #[inline(always)]
    pub(crate) fn ator_result(&self) -> crate::native::allocator::Allocator {
        view_read!(self, ator_result)
    }

    #[inline(always)]
    pub(crate) fn set_source(&self, source: crate::native::buf::Buf) {
        view_write!(self, source, source)
    }

    #[inline(always)]
    pub(crate) fn src_mesh_view(&self) -> &MeshView {
        unsafe { &*(&raw mut (*self.get()).src_mesh as *mut MeshView) }
    }

    #[inline(always)]
    pub(crate) fn dst_mesh_view(&self) -> &MeshView {
        unsafe { &*(&raw mut (*self.get()).dst_mesh as *mut MeshView) }
    }

    // `result` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn result_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).result as *mut crate::native::buf::BufView) }
    }

    // `source` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn source_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).source as *mut crate::native::buf::BufView) }
    }

    // `tmp` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp as *mut crate::native::buf::BufView) }
    }

    // `opts` — typed VIEW handle (reinterpret-in-place); leaf accessors on `SubdivideOptsView`.
    #[inline(always)]
    pub(crate) fn opts_view(&self) -> &SubdivideOptsView {
        // SAFETY: `SubdivideOptsView` is repr(transparent) over the `opts` field's layout,
        // which lives in this context's outer UnsafeCell; a shared interior-mutable
        // `&SubdivideOptsView` is sound and asserts no validity.
        unsafe { &*(&raw mut (*self.get()).opts as *mut SubdivideOptsView) }
    }

    // `tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_mut_ptr(&self) -> *mut Buf {
        view_raw_mut!(self, tmp)
    }

    // `src_mesh` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn src_mesh_mut_ptr(&self) -> *mut Mesh {
        view_raw_mut!(self, src_mesh)
    }

    // `source` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn source_mut_ptr(&self) -> *mut Buf {
        view_raw_mut!(self, source)
    }

    // `result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn result_mut_ptr(&self) -> *mut Buf {
        view_raw_mut!(self, result)
    }

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawSubdivideOpts {
        view_raw_mut!(self, opts)
    }

    // `inputs_cap` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn inputs_cap_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, inputs_cap)
    }

    // `inputs` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn inputs_mut_ptr(&self) -> *mut *mut SubdivideInput {
        view_raw_mut!(self, inputs)
    }

    // `error` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn error_mut_ptr(&self) -> *mut Error {
        view_raw_mut!(self, error)
    }

    // `error` — anchored VIEW handle; accessors on `ErrorView`. Routes the
    // error-form check macros through the SAFE `fail_err`/`fail_err_no_stack`.
    #[inline(always)]
    pub(crate) fn error_view(&self) -> &crate::native::error::ErrorView {
        // SAFETY: the context-owned `error` field is interior-mutable arena memory;
        // `&raw mut` keeps write provenance (never `&T`); borrow of `self` anchors `'a <= self`.
        unsafe { crate::native::error::ErrorView::from_ptr(&raw mut (*self.get()).error) }
    }

    // `dst_mesh` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn dst_mesh_mut_ptr(&self) -> *mut Mesh {
        view_raw_mut!(self, dst_mesh)
    }

    // `ator_tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_tmp_mut_ptr(&self) -> *mut Allocator {
        view_raw_mut!(self, ator_tmp)
    }

    // `ator_tmp` (Allocator) — typed VIEW handle (reinterpret-in-place); accessors on AllocatorView.
    #[inline(always)]
    pub(crate) fn ator_tmp_view(&self) -> &crate::native::allocator::AllocatorView {
        // SAFETY: reinterpret the owned Allocator field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).ator_tmp as *mut crate::native::allocator::AllocatorView)
        }
    }

    // `ator_result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_result_mut_ptr(&self) -> *mut Allocator {
        view_raw_mut!(self, ator_result)
    }

    // `ator_result` (Allocator) — typed VIEW handle (reinterpret-in-place); accessors on AllocatorView.
    #[inline(always)]
    pub(crate) fn ator_result_view(&self) -> &crate::native::allocator::AllocatorView {
        // SAFETY: reinterpret the owned Allocator field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).ator_result as *mut crate::native::allocator::AllocatorView)
        }
    }

    // `max_vertex_weights` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn max_vertex_weights(&self) -> usize {
        view_read!(self, max_vertex_weights)
    }

    #[inline(always)]
    pub(crate) fn set_max_vertex_weights(&self, max_vertex_weights: usize) {
        view_write!(self, max_vertex_weights, max_vertex_weights)
    }

    // `total_weights` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn total_weights(&self) -> usize {
        view_read!(self, total_weights)
    }

    #[inline(always)]
    pub(crate) fn set_total_weights(&self, total_weights: usize) {
        view_write!(self, total_weights, total_weights)
    }

    // `tmp_weights` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_weights(&self) -> *mut SubdivisionWeight {
        view_read!(self, tmp_weights)
    }

    #[inline(always)]
    pub(crate) fn set_tmp_weights(&self, tmp_weights: *mut SubdivisionWeight) {
        view_write!(self, tmp_weights, tmp_weights)
    }

    // `tmp_vertex_weights` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_vertex_weights(&self) -> *mut Real {
        view_read!(self, tmp_vertex_weights)
    }

    #[inline(always)]
    pub(crate) fn set_tmp_vertex_weights(&self, tmp_vertex_weights: *mut Real) {
        view_write!(self, tmp_vertex_weights, tmp_vertex_weights)
    }

    // `inputs_cap` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn inputs_cap(&self) -> usize {
        view_read!(self, inputs_cap)
    }

    // `inputs` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn inputs(&self) -> *mut SubdivideInput {
        view_read!(self, inputs)
    }

    // `num_topo` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn num_topo(&self) -> usize {
        view_read!(self, num_topo)
    }

    #[inline(always)]
    pub(crate) fn set_num_topo(&self, num_topo: usize) {
        view_write!(self, num_topo, num_topo)
    }

    // `topo` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn topo(&self) -> *mut TopoEdge {
        view_read!(self, topo)
    }

    #[inline(always)]
    pub(crate) fn set_topo(&self, topo: *mut TopoEdge) {
        view_write!(self, topo, topo)
    }

    // `src_mesh_ptr` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn src_mesh_ptr(&self) -> *mut Mesh {
        view_read!(self, src_mesh_ptr)
    }

    #[inline(always)]
    pub(crate) fn set_src_mesh_ptr(&self, src_mesh_ptr: *mut Mesh) {
        view_write!(self, src_mesh_ptr, src_mesh_ptr)
    }

    // `imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn imp(&self) -> *mut MeshImp {
        view_read!(self, imp)
    }

    #[inline(always)]
    pub(crate) fn set_imp(&self, imp: *mut MeshImp) {
        view_write!(self, imp, imp)
    }
}

// ufbx.c:28891-28904 `ufbxi_subdivide_sum_vec2`
#[cfg(feature = "subdivision")]
pub(crate) unsafe extern "C" fn subdivide_sum_vec2(
    user: *mut c_void,
    output: *mut c_void,
    inputs: *const SubdivideInput,
    num_inputs: usize,
) -> i32 {
    let _ = user;
    let mut dst: Vec2 = Vec2 { x: 0.0, y: 0.0 };
    // C: `ufbxi_nounroll for (size_t i = 0; i != num_inputs; i++)`
    let mut i: usize = 0;
    while i != num_inputs {
        // SAFETY: the sum-fn callback contract guarantees `inputs` points to
        // `num_inputs` live `SubdivideInput` entries and `i < num_inputs`, so
        // `inputs.add(i)` is in-bounds; each entry's `.data` points to a live
        // `Vec2` (this fn is registered as the vec2 summer), read through `src`.
        let src: *const Vec2 = unsafe { (*inputs.add(i)).data } as *const Vec2;
        // SAFETY: `i < num_inputs`, so `inputs.add(i)` is an in-bounds live entry.
        let weight: Real = unsafe { (*inputs.add(i)).weight };
        // SAFETY: `src` points to a live `Vec2` per the entry's `.data` contract.
        unsafe {
            dst.x += (*src).x * weight;
            dst.y += (*src).y * weight;
        }
        i += 1;
    }
    // SAFETY: the callback contract guarantees `output` points to a writable
    // `Vec2` sized destination.
    unsafe { *(output as *mut Vec2) = dst };

    1
}

// ufbx.c:28906-28920 `ufbxi_subdivide_sum_vec3`
#[cfg(feature = "subdivision")]
pub(crate) unsafe extern "C" fn subdivide_sum_vec3(
    user: *mut c_void,
    output: *mut c_void,
    inputs: *const SubdivideInput,
    num_inputs: usize,
) -> i32 {
    let _ = user;
    let mut dst: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    // C: `ufbxi_nounroll for (size_t i = 0; i != num_inputs; i++)`
    let mut i: usize = 0;
    while i != num_inputs {
        // SAFETY: the sum-fn callback contract guarantees `inputs` points to
        // `num_inputs` live `SubdivideInput` entries and `i < num_inputs`, so
        // `inputs.add(i)` is in-bounds; each entry's `.data` points to a live
        // `Vec3` (this fn is registered as the vec3 summer), read through `src`.
        let src: *const Vec3 = unsafe { (*inputs.add(i)).data } as *const Vec3;
        // SAFETY: `i < num_inputs`, so `inputs.add(i)` is an in-bounds live entry.
        let weight: Real = unsafe { (*inputs.add(i)).weight };
        // SAFETY: `src` points to a live `Vec3` per the entry's `.data` contract.
        unsafe {
            dst.x += (*src).x * weight;
            dst.y += (*src).y * weight;
            dst.z += (*src).z * weight;
        }
        i += 1;
    }
    // SAFETY: the callback contract guarantees `output` points to a writable
    // `Vec3` sized destination.
    unsafe { *(output as *mut Vec3) = dst };

    1
}

// ufbx.c:28922-28937 `ufbxi_subdivide_sum_vec4`
#[cfg(feature = "subdivision")]
pub(crate) unsafe extern "C" fn subdivide_sum_vec4(
    user: *mut c_void,
    output: *mut c_void,
    inputs: *const SubdivideInput,
    num_inputs: usize,
) -> i32 {
    let _ = user;
    let mut dst: Vec4 = Vec4 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };
    // C: `ufbxi_nounroll for (size_t i = 0; i != num_inputs; i++)`
    let mut i: usize = 0;
    while i != num_inputs {
        // SAFETY: the sum-fn callback contract guarantees `inputs` points to
        // `num_inputs` live `SubdivideInput` entries and `i < num_inputs`, so
        // `inputs.add(i)` is in-bounds; each entry's `.data` points to a live
        // `Vec4` (this fn is registered as the vec4 summer), read through `src`.
        let src: *const Vec4 = unsafe { (*inputs.add(i)).data } as *const Vec4;
        // SAFETY: `i < num_inputs`, so `inputs.add(i)` is an in-bounds live entry.
        let weight: Real = unsafe { (*inputs.add(i)).weight };
        // SAFETY: `src` points to a live `Vec4` per the entry's `.data` contract.
        unsafe {
            dst.x += (*src).x * weight;
            dst.y += (*src).y * weight;
            dst.z += (*src).z * weight;
            dst.w += (*src).w * weight;
        }
        i += 1;
    }
    // SAFETY: the callback contract guarantees `output` points to a writable
    // `Vec4` sized destination.
    unsafe { *(output as *mut Vec4) = dst };

    1
}

// ufbx.c:28939-28946 `ufbxi_subdivision_weight_less`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) unsafe extern "C" fn subdivision_weight_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    let _ = user;
    // SAFETY: the sort comparator contract guarantees `va` and `vb` each point
    // to a live `SubdivisionWeight` element; `ptr::read` copies each out by value.
    let (a, b) = unsafe {
        (
            core::ptr::read(va as *const SubdivisionWeight),
            core::ptr::read(vb as *const SubdivisionWeight),
        )
    };
    ufbxi_dev_assert!(a.index != b.index);
    if a.weight != b.weight {
        return a.weight > b.weight;
    }
    a.index < b.index
}

// ufbx.c:28948-29005 `ufbxi_subdivide_sum_vertex_weights`
#[cfg(feature = "subdivision")]
pub(crate) unsafe extern "C" fn subdivide_sum_vertex_weights(
    user: *mut c_void,
    output: *mut c_void,
    inputs: *const SubdivideInput,
    num_inputs: usize,
) -> i32 {
    // SAFETY: the sum-fn callback contract passes the `SubdivideContext` as
    // `user`, so `user` points to a live `SubdivideContext` for this call.
    let sc: &SubdivideContext = unsafe { &*(user as *const SubdivideContext) };

    let vertex_weights: *mut Real = sc.tmp_vertex_weights();
    let tmp_weights: *mut SubdivisionWeight = sc.tmp_weights();
    let mut num_weights: usize = 0;

    // C: `ufbxi_nounroll for (size_t input_ix = 0; input_ix != num_inputs; input_ix++)`
    let mut input_ix: usize = 0;
    while input_ix != num_inputs {
        // SAFETY: the callback contract guarantees `inputs` points to
        // `num_inputs` live entries and `input_ix < num_inputs`, so
        // `inputs.add(input_ix)` is in-bounds; its `.data` points to a live
        // `SubdivisionVertexWeights` (this fn is the vertex-weights summer),
        // copied out by value.
        let src: SubdivisionVertexWeights =
            unsafe { *((*inputs.add(input_ix)).data as *const SubdivisionVertexWeights) };
        // SAFETY: `input_ix < num_inputs`, so `inputs.add(input_ix)` is live.
        let input_weight: Real = unsafe { (*inputs.add(input_ix)).weight };

        let mut weight_ix: usize = 0;
        while weight_ix < src.num_weights {
            // SAFETY: `src.weights` points to `src.num_weights` live entries and
            // `weight_ix < src.num_weights`, so `.add(weight_ix)` is in-bounds.
            let weight: Real = input_weight * unsafe { (*src.weights.add(weight_ix)).weight };
            // C: `if (weight < 1.175494351e-38f) continue;` — a `float` literal
            // widened to `ufbx_real`.
            if weight < 1.175494351e-38f32 as Real {
                weight_ix += 1;
                continue;
            }

            // SAFETY: `weight_ix < src.num_weights`, so `.add(weight_ix)` is live.
            let vx: u32 = unsafe { (*src.weights.add(weight_ix)).index };
            ufbxi_dev_assert!((vx as usize) < sc.src_mesh_view().num_vertices());

            // SAFETY: `vx` is `< num_vertices` by loaded-data consistency —
            // the same invariant upstream relies on for both the
            // source-vertex path (where `vx` is a vertex index) and the skin
            // path (where it is a cluster index), dev-asserted above in
            // dev/regression builds. `vertex_weights` (`tmp_vertex_weights`)
            // holds one `Real` per source vertex, so `.add(vx)` is in-bounds.
            let prev: Real = unsafe { *vertex_weights.add(vx as usize) };
            // SAFETY: as above; in-bounds write to the accumulator slot.
            unsafe { *vertex_weights.add(vx as usize) = prev + weight };
            if prev == 0.0 {
                // SAFETY: at most one entry is pushed per distinct index, and
                // the distinct indices of each path (source vertices or skin
                // clusters) are bounded by the corresponding count folded into
                // the `max_weights` sizing of `tmp_weights`, keeping
                // `.add(num_weights)` in-bounds.
                unsafe { (*tmp_weights.add(num_weights)).index = vx };
                num_weights += 1;
            }
            weight_ix += 1;
        }
        input_ix += 1;
    }

    // C: `ufbxi_nounroll for (size_t i = 0; i != num_weights; i++)`
    let mut i: usize = 0;
    while i != num_weights {
        // SAFETY: `i < num_weights` and `tmp_weights` holds `num_weights` live
        // entries populated above, so `.add(i)` is in-bounds.
        let vx: u32 = unsafe { (*tmp_weights.add(i)).index };
        // SAFETY: `.add(i)` is in-bounds (as above); `vx` was recorded above as
        // an index `< num_vertices`, so `vertex_weights.add(vx)` is in-bounds.
        unsafe { (*tmp_weights.add(i)).weight = *vertex_weights.add(vx as usize) };
        // SAFETY: `vx` is `< num_vertices`, so `.add(vx)` is in-bounds; the
        // accumulator is reset to zero for reuse.
        unsafe { *vertex_weights.add(vx as usize) = 0.0 };
        i += 1;
    }

    // SAFETY: `tmp_weights` holds `num_weights` live `SubdivisionWeight` entries
    // of the passed element size; `subdivision_weight_less` is a matching
    // comparator and the `null` user pointer is unused by it.
    unsafe {
        unstable_sort(
            tmp_weights as *mut c_void,
            num_weights,
            size_of::<SubdivisionWeight>(),
            subdivision_weight_less,
            core::ptr::null_mut(),
        );
    }

    if sc.max_vertex_weights() != usize::MAX {
        num_weights = min_sz(sc.max_vertex_weights(), num_weights);

        // Normalize weights
        let mut prefix_weight: Real = 0.0;
        // C: `ufbxi_nounroll for (size_t i = 0; i != num_weights; i++)`
        let mut i: usize = 0;
        while i != num_weights {
            // SAFETY: `i < num_weights` (post-clamp) and `tmp_weights` holds at
            // least that many live entries, so `.add(i)` is in-bounds.
            prefix_weight += unsafe { (*tmp_weights.add(i)).weight };
            i += 1;
        }
        let mut i: usize = 0;
        while i != num_weights {
            // SAFETY: `i < num_weights` and `.add(i)` is in-bounds (as above).
            unsafe { (*tmp_weights.add(i)).weight /= prefix_weight };
            i += 1;
        }
    }

    sc.set_total_weights(sc.total_weights().wrapping_add(num_weights));
    // SAFETY: `tmp_mut_ptr()` is `sc`'s own live scratch `Buf`; `tmp_weights`
    // holds `num_weights` live `SubdivisionWeight` entries to copy from.
    let weights: *mut SubdivisionWeight = unsafe {
        sc.tmp_view()
            .push_copy_raw::<SubdivisionWeight>(num_weights, tmp_weights)
    };
    // C: `ufbxi_check_err(&sc->error, weights);` — this function returns `int`,
    // so the C macro's `return 0` is a plain 0 here.
    ufbxi_check_return_err!(sc.error_view(), !weights.is_null(), 0, "weights");

    let dst: *mut SubdivisionVertexWeights = output as *mut SubdivisionVertexWeights;
    // SAFETY: the callback contract guarantees `output` points to a writable
    // `SubdivisionVertexWeights` destination.
    unsafe {
        (*dst).weights = weights;
        (*dst).num_weights = num_weights;
    }

    1
}

// ufbx.c:29007-29012 `ufbxi_real_sum_fns`
#[cfg(feature = "subdivision")]
pub(crate) static REAL_SUM_FNS: [Option<SubdivideSumFn>; 4] = [
    None,
    Some(subdivide_sum_vec2),
    Some(subdivide_sum_vec3),
    Some(subdivide_sum_vec4),
];

// ufbx.c:29014-29034 `ufbxi_is_edge_split`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) unsafe fn is_edge_split(
    input: *const SubdivideLayerInput,
    topo: *const TopoEdge,
    index: u32,
) -> bool {
    // SAFETY: the caller passes `index` as a corner index within `topo`'s live
    // `TopoEdge` array, so `topo.add(index)` is in-bounds.
    let twin: u32 = unsafe { (*topo.add(index as usize)).twin };
    if twin != NO_INDEX {
        // SAFETY: `input` points to a live `SubdivideLayerInput` (fn contract)
        // whose `indices` array is indexed by corner; `index` is an in-range
        // corner, so `.add(index)` is in-bounds.
        let a0: u32 = unsafe { *(*input).indices.add(index as usize) };
        // SAFETY: `topo.add(index)` is in-bounds (as above); its `.next` is a
        // sibling corner index in range for `input`'s `indices` array.
        let a1: u32 = unsafe {
            *(*input)
                .indices
                .add((*topo.add(index as usize)).next as usize)
        };
        // SAFETY: `twin != NO_INDEX` is a valid corner index, so `topo.add(twin)`
        // is in-bounds; its `.next` is a corner index in range for `indices`.
        let b0: u32 = unsafe {
            *(*input)
                .indices
                .add((*topo.add(twin as usize)).next as usize)
        };
        // SAFETY: `twin` is a valid corner index in range for `indices`.
        let b1: u32 = unsafe { *(*input).indices.add(twin as usize) };
        if a0 == b0 && a1 == b1 {
            return false;
        }
        // SAFETY (this group): `input` points to a live `SubdivideLayerInput`.
        if !unsafe { (*input).check_split_data } {
            return true;
        }
        let stride: usize = unsafe { (*input).stride };
        // SAFETY: `input.values` is a byte-addressed attribute buffer holding
        // `stride` bytes per value; `a0`/`a1`/`b0`/`b1` come from `input.indices`,
        // which maps corners to indices into that value array (not to mesh
        // vertices), so every `.add(ix*stride)` byte offset stays within the buffer.
        let (da0, da1, db0, db1) = unsafe {
            (
                ((*input).values as *const u8).add((a0 as usize).wrapping_mul(stride)),
                ((*input).values as *const u8).add((a1 as usize).wrapping_mul(stride)),
                ((*input).values as *const u8).add((b0 as usize).wrapping_mul(stride)),
                ((*input).values as *const u8).add((b1 as usize).wrapping_mul(stride)),
            )
        };
        // SAFETY: `da0`/`db0` and `da1`/`db1` each point to `stride` live bytes
        // within `input.values`, the length `memcmp` compares.
        if unsafe { memcmp(da0, db0, stride) == 0 && memcmp(da1, db1, stride) == 0 } {
            return false;
        }
        return true;
    }

    false
}

// ufbx.c:29036-29042 `ufbxi_edge_crease`
#[cfg(feature = "subdivision")]
pub(crate) unsafe fn edge_crease(
    mesh: &MeshView,
    split: bool,
    topo: *const TopoEdge,
    index: u32,
) -> Real {
    // SAFETY: the caller passes `index` as a corner index within `topo`'s live
    // `TopoEdge` array, so `topo.add(index)` is in-bounds.
    if unsafe { (*topo.add(index as usize)).twin } == NO_INDEX {
        return 1.0;
    }
    if split {
        return 1.0;
    }
    // SAFETY: `topo.add(index)` is in-bounds (as above).
    if !mesh.edge_crease_view().data().is_null()
        && unsafe { (*topo.add(index as usize)).edge } != NO_INDEX
    {
        // SAFETY: the guard proved `edge_crease` data is non-null and this
        // corner's `.edge != NO_INDEX`; a non-sentinel `.edge` is an in-range
        // edge index by topology/mesh consistency (`compute_topology` fills
        // `.edge` from the source mesh's edge list, and `edge_crease` spans
        // `num_edges`), so the `.add(edge)` read is in-bounds.
        return unsafe {
            *mesh
                .edge_crease_view()
                .data()
                .add((*topo.add(index as usize)).edge as usize)
        } * (10.0 as Real);
    }
    0.0
}

// ufbx.c:29044-29462 `ufbxi_subdivide_layer`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) unsafe fn subdivide_layer(
    sc: &SubdivideContext,
    output: *mut SubdivideLayerOutput,
    input: *const SubdivideLayerInput,
) -> Result<(), crate::native::error::Fail> {
    // SAFETY: `input` points to a live `SubdivideLayerInput` (fn contract).
    let boundary: SubdivisionBoundary = unsafe { (*input).boundary };

    let mesh: &MeshView = sc.src_mesh_view();
    let topo: *const TopoEdge = sc.topo();
    let num_topo: usize = sc.num_topo();

    let edge_indices: *mut u32 = sc.result_view().push::<u32>(mesh.num_indices());
    ufbxi_check_err!(sc.error_view(), !edge_indices.is_null(), "edge_indices");

    let mut num_edge_values: usize = 0;
    // C: `for (uint32_t ix = 0; ix < (uint32_t)mesh->num_indices; ix++)` — the
    // bound is truncated to `uint32_t` here (unlike the edge-point loop below).
    let mut ix: u32 = 0;
    while ix < mesh.num_indices() as u32 {
        // SAFETY: `ix < num_indices`, so it is a valid corner index into `topo`,
        // which holds one live `TopoEdge` per index of the source mesh.
        let twin: u32 = unsafe { (*topo.add(ix as usize)).twin };
        // SAFETY: `input`/`topo` are live and `ix` is an in-range corner index.
        if twin < ix && !unsafe { is_edge_split(input, topo, ix) } {
            // SAFETY: `edge_indices` holds `num_indices` live slots; `ix` and
            // `twin < ix` are both in-range corner indices.
            unsafe { *edge_indices.add(ix as usize) = *edge_indices.add(twin as usize) };
        } else {
            // SAFETY: `ix < num_indices` is an in-range slot of `edge_indices`.
            unsafe { *edge_indices.add(ix as usize) = num_edge_values as u32 };
            num_edge_values += 1;
        }
        ix += 1;
    }

    // SAFETY: `input` points to a live `SubdivideLayerInput`.
    let stride: usize = unsafe { (*input).stride };
    let num_initial_values: usize = num_edge_values
        .wrapping_add(mesh.num_faces())
        .wrapping_add(mesh.num_indices());
    // SAFETY: `tmp_mut_ptr()` is `sc`'s own live scratch `Buf`, the push contract.
    let values: *mut u8 =
        unsafe { push_size(sc.tmp_mut_ptr(), stride, num_initial_values) } as *mut u8;
    ufbxi_check_err!(sc.error_view(), !values.is_null(), "values");

    let face_values: *mut u8 = values;
    // SAFETY: `values` is a `num_initial_values`-element `stride`-sized buffer
    // laid out faces|edges|vertices, so `num_faces*stride` and
    // `num_edge_values*stride` are the intended in-range segment offsets.
    let edge_values: *mut u8 = unsafe { face_values.add(mesh.num_faces().wrapping_mul(stride)) };
    // SAFETY: as above; the vertex segment begins after the edge segment.
    let vertex_values: *mut u8 = unsafe { edge_values.add(num_edge_values.wrapping_mul(stride)) };

    let mut num_vertex_values: usize = 0;

    let vertex_indices: *mut u32 = sc.result_view().push::<u32>(mesh.num_indices());
    ufbxi_check_err!(sc.error_view(), !vertex_indices.is_null(), "vertex_indices");

    let min_inputs: usize = max_sz(32, mesh.max_face_triangles().wrapping_add(2));
    ufbxi_check_err!(
        sc.error_view(),
        // SAFETY: the three `*_mut_ptr` accessors return `sc`'s own live
        // allocator/inputs/cap fields, the array-growth contract `grow_array`
        // requires.
        unsafe {
            grow_array::<SubdivideInput>(
                sc.ator_tmp_mut_ptr(),
                sc.inputs_mut_ptr(),
                sc.inputs_cap_mut_ptr(),
                min_inputs,
            )
        },
        "ufbxi_grow_array_size((&sc->ator_tmp), sizeof(**(&sc->inputs)), (&sc->inputs), (&sc->inputs_cap), (min_inputs))"
    );
    let mut inputs: *mut SubdivideInput = sc.inputs();

    // Assume initially unique per vertex, remove if not the case
    // SAFETY: `output` points to a live `SubdivideLayerOutput` (fn contract).
    unsafe { (*output).unique_per_vertex = true };

    let mut sharp_corners: bool = false;
    let mut sharp_splits: bool = false;
    let mut sharp_all: bool = false;

    match boundary {
        SubdivisionBoundary::Default
        | SubdivisionBoundary::SharpNone
        | SubdivisionBoundary::Legacy => {
            // All smooth
        }
        SubdivisionBoundary::SharpCorners => {
            sharp_corners = true;
        }
        SubdivisionBoundary::SharpBoundary => {
            sharp_corners = true;
            sharp_splits = true;
        }
        SubdivisionBoundary::SharpInterior => {
            sharp_all = true;
        }
        // C `default:` (ufbx.c:29106-29107) — unreachable in Rust because the
        // match above is exhaustive over the enum, but kept for diff parity.
        #[allow(unreachable_patterns)]
        _ => {
            ufbxi_unreachable!("Bad boundary mode");
        }
    }

    // C: `ufbxi_real_sum_fns[]` has a NULL slot at index 0; the callers assert
    // `value_reals >= 2` before indexing, so the taken pointer is never NULL —
    // calling through a NULL slot is UB in C too.
    // SAFETY: `input` is live; the comment above vouches slot 0 (the `None`
    // slot) is never selected here, so `sum_fn` is `Some`, making
    // `unwrap_unchecked` sound.
    let sum_fn: SubdivideSumFn = unsafe { (*input).sum_fn.unwrap_unchecked() };
    // SAFETY: `input` points to a live `SubdivideLayerInput`.
    let sum_user: *mut c_void = unsafe { (*input).sum_user };

    // Mark unused indices as `UFBX_NO_INDEX` so we can patch non-manifold
    // C: `ufbxi_nounroll for (size_t i = 0; i < mesh->num_indices; i++)`
    let mut i: usize = 0;
    while i < mesh.num_indices() {
        // SAFETY: `i < num_indices`, an in-range slot of the `num_indices`-sized
        // `vertex_indices` push.
        unsafe { *vertex_indices.add(i) = NO_INDEX };
        i += 1;
    }

    // Face points
    let mut fi: usize = 0;
    while fi < mesh.num_faces() {
        // SAFETY: `fi < num_faces`, in range for the live `faces` array.
        let face: Face = unsafe { *mesh.faces().data.add(fi) };
        // SAFETY: `fi < num_faces`, so `fi*stride` is within the `num_faces`
        // face-value segment at the head of `values`.
        let dst: *mut u8 = unsafe { face_values.add(fi.wrapping_mul(stride)) };

        let weight: Real = 1.0 / (face.num_indices as Real);
        let mut ci: u32 = 0;
        while ci < face.num_indices {
            let ix: u32 = face.index_begin.wrapping_add(ci);
            // SAFETY: `inputs` was grown to hold at least `max_face_triangles+2`
            // and `>= 32` entries, so `ci < face.num_indices` indexes a live
            // slot; `input.values`/`input.indices` are live, and `indices[ix]`
            // is an in-range index into the value array whose `stride`-sized
            // entry the byte offset addresses.
            unsafe {
                (*inputs.add(ci as usize)).data = ((*input).values as *const u8)
                    .add((*(*input).indices.add(ix as usize) as usize).wrapping_mul(stride))
                    as *const c_void;
                (*inputs.add(ci as usize)).weight = weight;
            }
            ci += 1;
        }

        ufbxi_check_err!(
            sc.error_view(),
            // SAFETY: `sum_fn` is the attribute's registered summer; `sum_user`,
            // `dst`, and the first `face.num_indices` `inputs` entries were set
            // up above per its callback contract.
            unsafe {
                sum_fn(
                    sum_user,
                    dst as *mut c_void,
                    inputs,
                    face.num_indices as usize,
                )
            } != 0,
            "sum_fn(sum_user, dst, inputs, face.num_indices)"
        );
        fi += 1;
    }

    // Edge points
    // C: `for (uint32_t ix = 0; ix < mesh->num_indices; ix++)` — `ix` is
    // promoted to `size_t` for the comparison here (no truncation).
    let mut ix: u32 = 0;
    while (ix as usize) < mesh.num_indices() {
        // SAFETY: `ix < num_indices` indexes the live `edge_indices` array;
        // the stored edge-value index times `stride` stays within the edge
        // segment of `values` that `edge_values` heads.
        let dst: *mut u8 = unsafe {
            edge_values.add((*edge_indices.add(ix as usize) as usize).wrapping_mul(stride))
        };

        // SAFETY: `ix < num_indices` is an in-range corner index into `topo`.
        let twin: u32 = unsafe { (*topo.add(ix as usize)).twin };
        // SAFETY: `input`/`topo` live, `ix` an in-range corner index.
        let split: bool = unsafe { is_edge_split(input, topo, ix) };

        // SAFETY: `topo.add(ix)` is in-bounds (as above); `.flags` is read by value.
        if split || unsafe { (*topo.add(ix as usize)).flags }.has_any(TopoFlags::NON_MANIFOLD) {
            // SAFETY: `output` points to a live `SubdivideLayerOutput`.
            unsafe { (*output).unique_per_vertex = false };
        }

        let mut crease: Real = 0.0;
        if split || twin == NO_INDEX {
            crease = 1.0;
        // SAFETY: `topo.add(ix)` is in-bounds.
        } else if unsafe { (*topo.add(ix as usize)).edge } != NO_INDEX
            && !mesh.edge_crease().data.is_null()
        {
            // SAFETY: the guard proved this corner's `.edge != NO_INDEX` and
            // `edge_crease.data` is non-null; a non-sentinel `.edge` is an
            // in-range edge index by topology/mesh consistency (`edge_crease`
            // spans `num_edges`), so the `.add(edge)` read is in-bounds.
            crease = unsafe {
                *mesh
                    .edge_crease()
                    .data
                    .add((*topo.add(ix as usize)).edge as usize)
            } * (10.0 as Real);
        }
        if sharp_all {
            crease = 1.0;
        }

        // SAFETY: `input` is live; `indices[ix]` and `indices[topo[ix].next]` are
        // in-range indices into the value array whose `stride`-sized entries the
        // byte offsets into `values` address.
        let (v0, v1) = unsafe {
            (
                ((*input).values as *const u8)
                    .add((*(*input).indices.add(ix as usize) as usize).wrapping_mul(stride)),
                ((*input).values as *const u8).add(
                    (*(*input).indices.add((*topo.add(ix as usize)).next as usize) as usize)
                        .wrapping_mul(stride),
                ),
            )
        };

        // TODO: Unify
        if twin < ix && !split {
            // Already calculated
        } else if crease <= 0.0 {
            // SAFETY: this corner's and its twin's `.face` are in-range face
            // indices (`twin != NO_INDEX` in this arm), so `face*stride` stays
            // within the face segment of `values`.
            let (f0, f1) = unsafe {
                (
                    face_values.add(((*topo.add(ix as usize)).face as usize).wrapping_mul(stride)),
                    face_values
                        .add(((*topo.add(twin as usize)).face as usize).wrapping_mul(stride)),
                )
            };
            // SAFETY: `inputs` holds at least 4 live slots (grown `>= 32`); the
            // four `data`/`weight` fields are the sum-fn's inputs.
            unsafe {
                (*inputs.add(0)).data = v0 as *const c_void;
                (*inputs.add(0)).weight = 0.25;
                (*inputs.add(1)).data = v1 as *const c_void;
                (*inputs.add(1)).weight = 0.25;
                (*inputs.add(2)).data = f0 as *const c_void;
                (*inputs.add(2)).weight = 0.25;
                (*inputs.add(3)).data = f1 as *const c_void;
                (*inputs.add(3)).weight = 0.25;
            }
            ufbxi_check_err!(
                sc.error_view(),
                // SAFETY: `sum_fn`/`sum_user`/`dst` and the 4 inputs satisfy the
                // summer callback contract.
                unsafe { sum_fn(sum_user, dst as *mut c_void, inputs, 4) } != 0,
                "sum_fn(sum_user, dst, inputs, 4)"
            );
        } else if crease >= 1.0 {
            // SAFETY: `inputs` holds at least 2 live slots; these are the two
            // sum-fn inputs.
            unsafe {
                (*inputs.add(0)).data = v0 as *const c_void;
                (*inputs.add(0)).weight = 0.5;
                (*inputs.add(1)).data = v1 as *const c_void;
                (*inputs.add(1)).weight = 0.5;
            }
            ufbxi_check_err!(
                sc.error_view(),
                // SAFETY: `sum_fn`/`sum_user`/`dst` and the 2 inputs satisfy the
                // summer callback contract.
                unsafe { sum_fn(sum_user, dst as *mut c_void, inputs, 2) } != 0,
                "sum_fn(sum_user, dst, inputs, 2)"
            );
        } else if crease < 1.0 {
            // SAFETY: this corner's and its twin's `.face` are in-range face
            // indices, so `face*stride` stays within the face segment of `values`.
            let (f0, f1) = unsafe {
                (
                    face_values.add(((*topo.add(ix as usize)).face as usize).wrapping_mul(stride)),
                    face_values
                        .add(((*topo.add(twin as usize)).face as usize).wrapping_mul(stride)),
                )
            };
            let w0: Real = 0.25 + 0.25 * crease;
            let w1: Real = 0.25 - 0.25 * crease;

            // SAFETY: `inputs` holds at least 4 live slots; these are the sum-fn
            // inputs.
            unsafe {
                (*inputs.add(0)).data = v0 as *const c_void;
                (*inputs.add(0)).weight = w0;
                (*inputs.add(1)).data = v1 as *const c_void;
                (*inputs.add(1)).weight = w0;
                (*inputs.add(2)).data = f0 as *const c_void;
                (*inputs.add(2)).weight = w1;
                (*inputs.add(3)).data = f1 as *const c_void;
                (*inputs.add(3)).weight = w1;
            }
            ufbxi_check_err!(
                sc.error_view(),
                // SAFETY: `sum_fn`/`sum_user`/`dst` and the 4 inputs satisfy the
                // summer callback contract.
                unsafe { sum_fn(sum_user, dst as *mut c_void, inputs, 4) } != 0,
                "sum_fn(sum_user, dst, inputs, 4)"
            );
        }
        ix = ix.wrapping_add(1);
    }

    // Vertex points
    let mut vi: usize = 0;
    while vi < mesh.num_vertices() {
        // SAFETY: `vi < num_vertices` indexes the live `vertex_first_index` array.
        let mut original_start: u32 = unsafe { *mesh.vertex_first_index().data.add(vi) };
        if original_start == NO_INDEX {
            vi += 1;
            continue;
        }

        // Find a topological boundary, or if not found a split edge
        let mut start: u32 = original_start;
        // C: `for (uint32_t cur = start;;)`
        let mut cur: u32 = start;
        loop {
            // SAFETY: `topo`/`num_topo` are `sc`'s live topology and length;
            // `cur` is a corner index reachable from a valid vertex corner.
            let prev: u32 = unsafe { topo_prev_vertex_edge(topo, num_topo, cur) };
            if prev == NO_INDEX {
                start = cur;
                break;
            } // Topological boundary: Stop and use as start
              // SAFETY: `input`/`topo` live; `prev` is an in-range corner index.
            if unsafe { is_edge_split(input, topo, prev) } {
                start = cur;
            } // Split edge: Consider as start
            if prev == original_start {
                break;
            } // Loop: Stop, use original start or split if found
            cur = prev;
        }

        original_start = start;
        while start != NO_INDEX {
            if start != original_start {
                // SAFETY: `output` points to a live `SubdivideLayerOutput`.
                unsafe { (*output).unique_per_vertex = false };
            }

            let value_index: u32 = num_vertex_values as u32;
            num_vertex_values += 1;
            // SAFETY: every completed iteration claims a fresh corner through
            // the runtime-checked `vertex_indices[..] == NO_INDEX` guard below,
            // so `value_index` is at most the claimed-corner count and stays
            // `<= num_indices`; `value_index*stride` therefore addresses at
            // worst one-past the vertex segment of `values` that
            // `vertex_values` heads, and this iteration's own claim (which
            // errors out otherwise) makes it strictly in-bounds before `dst` is
            // written through.
            let dst: *mut u8 =
                unsafe { vertex_values.add((value_index as usize).wrapping_mul(stride)) };

            // We need to compute the average crease value and keep track of
            // two creased edges, if there's more we use the corner rule that
            // does not need the information.
            let mut total_crease: Real = 0.0;
            let mut num_crease: usize = 0;
            let mut num_split: usize = 0;
            let mut on_boundary: bool = false;
            let mut non_manifold: bool = false;
            // C: `size_t crease_input_indices[2]; // ufbxi_uninit` — both slots
            // are written before either is read (only `num_crease == 2` reads
            // them), so the zero fill is observationally identical.
            let mut crease_input_indices: [usize; 2] = [0, 0]; // ufbxi_uninit

            // At start we always have two edges and a single face
            // SAFETY: `start` is a valid vertex corner index into the live `topo`.
            let start_prev: u32 = unsafe { (*topo.add(start as usize)).prev };
            // SAFETY: `start_prev` is a sibling corner index, in range for `topo`.
            let end_edge: u32 = unsafe { (*topo.add(start_prev as usize)).twin };
            let mut valence: usize = 2;

            // SAFETY: `topo.add(start)` is in-bounds; `.flags` is read by value.
            non_manifold |=
                unsafe { (*topo.add(start as usize)).flags }.has_any(TopoFlags::NON_MANIFOLD);
            // SAFETY: `topo.add(start_prev)` is in-bounds; `.flags` read by value.
            non_manifold |=
                unsafe { (*topo.add(start_prev as usize)).flags }.has_any(TopoFlags::NON_MANIFOLD);

            // SAFETY: `input` live; `indices[start]` is an in-range index into
            // the value array whose `stride`-sized entry the byte offset
            // addresses.
            let v0: *const u8 = unsafe {
                ((*input).values as *const u8)
                    .add((*(*input).indices.add(start as usize) as usize).wrapping_mul(stride))
            };

            let mut num_inputs: usize = 4;

            {
                // SAFETY: `input`/`topo` live; the `.next` sibling corner's and
                // `start_prev`'s vertices are in-range, addressing their
                // `stride`-sized attributes in `values`.
                let (e0, e1) = unsafe {
                    (
                        ((*input).values as *const u8).add(
                            (*(*input)
                                .indices
                                .add((*topo.add(start as usize)).next as usize)
                                as usize)
                                .wrapping_mul(stride),
                        ),
                        ((*input).values as *const u8).add(
                            (*(*input).indices.add(start_prev as usize) as usize)
                                .wrapping_mul(stride),
                        ),
                    )
                };
                // SAFETY: this corner's `.face` is an in-range face index, so
                // `face*stride` stays within the face segment of `values`.
                let f0: *const u8 = unsafe {
                    face_values
                        .add(((*topo.add(start as usize)).face as usize).wrapping_mul(stride))
                };
                // SAFETY: `inputs` holds at least 4 live slots; these are the
                // first four sum-fn inputs.
                unsafe {
                    (*inputs.add(0)).data = v0 as *const c_void;
                    (*inputs.add(1)).data = e0 as *const c_void;
                    (*inputs.add(2)).data = e1 as *const c_void;
                    (*inputs.add(3)).data = f0 as *const c_void;
                }
            }

            // SAFETY: `input`/`topo` live; `start` is an in-range corner index, and
            // the short-circuiting `end_edge != NO_INDEX` guard keeps the second
            // call's corner index in range too.
            let (start_split, prev_split) = unsafe {
                (
                    is_edge_split(input, topo, start),
                    end_edge != NO_INDEX && is_edge_split(input, topo, end_edge),
                )
            };

            // Either of the first two edges may be creased
            // SAFETY: `topo` live; `start` is an in-range corner index.
            let start_crease: Real = unsafe { edge_crease(mesh, start_split, topo, start) };
            if start_crease > 0.0 {
                total_crease += start_crease;
                crease_input_indices[num_crease] = 1;
                num_crease += 1;
            }
            // SAFETY: as above, using `start_prev` as the in-range corner index.
            let prev_crease: Real = unsafe { edge_crease(mesh, prev_split, topo, start_prev) };
            if prev_crease > 0.0 {
                total_crease += prev_crease;
                crease_input_indices[num_crease] = 2;
                num_crease += 1;
            }

            if end_edge != NO_INDEX {
                if prev_split {
                    num_split += 1;
                }
            } else {
                on_boundary = true;
            }

            ufbxi_check_err!(
                sc.error_view(),
                // SAFETY: `start` is a valid vertex corner index into the live
                // `num_indices`-sized `vertex_indices` array.
                unsafe { *vertex_indices.add(start as usize) } == NO_INDEX,
                "vertex_indices[start] == UFBX_NO_INDEX"
            );
            // SAFETY: `start` is an in-range corner slot of `vertex_indices`.
            unsafe { *vertex_indices.add(start as usize) = value_index };

            if start_split {
                // We need to special case if the first edge is split as we have
                // handled it already in the code above..
                // SAFETY: `topo`/`num_topo` live; `start` is an in-range corner.
                start = unsafe { topo_next_vertex_edge(topo, num_topo, start) };
                num_split += 1;
            } else {
                // Follow vertex edges until we either hit a topological/split boundary
                // or loop back to the left edge we accounted for in `start_prev`
                let mut cur: u32 = start;
                loop {
                    // SAFETY: `topo`/`num_topo` live; `cur` is an in-range corner.
                    cur = unsafe { topo_next_vertex_edge(topo, num_topo, cur) };

                    // Topological boundary: Finished
                    if cur == NO_INDEX {
                        on_boundary = true;
                        start = NO_INDEX;
                        break;
                    }

                    // SAFETY: `cur != NO_INDEX` is an in-range corner into `topo`;
                    // `.flags` is read by value.
                    non_manifold |=
                        unsafe { (*topo.add(cur as usize)).flags }.has_any(TopoFlags::NON_MANIFOLD);
                    ufbxi_check_err!(
                        sc.error_view(),
                        // SAFETY: `cur` is an in-range corner slot of `vertex_indices`.
                        unsafe { *vertex_indices.add(cur as usize) } == NO_INDEX,
                        "vertex_indices[cur] == UFBX_NO_INDEX"
                    );
                    // SAFETY: `cur` is an in-range corner slot of `vertex_indices`.
                    unsafe { *vertex_indices.add(cur as usize) = value_index };

                    // SAFETY: `input`/`topo` live; `cur` is an in-range corner.
                    let split: bool = unsafe { is_edge_split(input, topo, cur) };

                    // Looped: Add the face from the other side still if not split
                    if cur == end_edge && !split {
                        ufbxi_check_err!(
                            sc.error_view(),
                            // SAFETY: the `*_mut_ptr` accessors return `sc`'s own
                            // live allocator/inputs/cap fields per the grow contract.
                            unsafe {
                                grow_array::<SubdivideInput>(
                                    sc.ator_tmp_mut_ptr(),
                                    sc.inputs_mut_ptr(),
                                    sc.inputs_cap_mut_ptr(),
                                    num_inputs.wrapping_add(1),
                                )
                            },
                            "ufbxi_grow_array_size((&sc->ator_tmp), sizeof(**(&sc->inputs)), (&sc->inputs), (&sc->inputs_cap), (num_inputs + 1))"
                        );
                        // SAFETY: `cur`'s `.face` is an in-range face index, so
                        // `face*stride` stays within the face segment of `values`.
                        let f0: *const u8 = unsafe {
                            face_values
                                .add(((*topo.add(cur as usize)).face as usize).wrapping_mul(stride))
                        };
                        // C-parity: C does NOT refresh the local `inputs` from
                        // `sc->inputs` after this grow (unlike the paired grow
                        // below) — the stale pointer is written through
                        // verbatim.
                        // SAFETY: the grow above sized `sc`'s array to at least
                        // `num_inputs + 1` slots, but this write goes through
                        // the pre-grow `inputs` local, so slot `num_inputs` is
                        // live only when the grow did not reallocate
                        // (`inputs_cap == num_inputs` on entry is reachable,
                        // making the pointer stale) — the upstream
                        // stale-pointer write is mirrored verbatim.
                        unsafe { (*inputs.add(num_inputs)).data = f0 as *const c_void };
                        start = NO_INDEX;
                        num_inputs += 1;
                        break;
                    }

                    // Add the edge crease, this also handles boundaries as they
                    // have an implicit crease of 1.0 using `ufbxi_edge_crease()`
                    // SAFETY: `topo` live; `cur` is an in-range corner.
                    let cur_crease: Real = unsafe { edge_crease(mesh, split, topo, cur) };
                    if cur_crease > 0.0 {
                        total_crease += cur_crease;
                        if num_crease < 2 {
                            crease_input_indices[num_crease] = num_inputs;
                        }
                        num_crease += 1;
                    }

                    // Add the new edge and face to the sum
                    {
                        ufbxi_check_err!(
                            sc.error_view(),
                            // SAFETY: the `*_mut_ptr` accessors return `sc`'s own
                            // live allocator/inputs/cap fields per the grow contract.
                            unsafe {
                                grow_array::<SubdivideInput>(
                                    sc.ator_tmp_mut_ptr(),
                                    sc.inputs_mut_ptr(),
                                    sc.inputs_cap_mut_ptr(),
                                    num_inputs.wrapping_add(2),
                                )
                            },
                            "ufbxi_grow_array_size((&sc->ator_tmp), sizeof(**(&sc->inputs)), (&sc->inputs), (&sc->inputs_cap), (num_inputs + 2))"
                        );
                        inputs = sc.inputs();

                        // SAFETY: `input`/`topo` live; the index `indices` holds
                        // for `cur`'s `.next` sibling corner is in range for
                        // the value array, addressing its `stride`-sized entry
                        // in `values`.
                        let e0: *const u8 = unsafe {
                            ((*input).values as *const u8).add(
                                (*(*input)
                                    .indices
                                    .add((*topo.add(cur as usize)).next as usize)
                                    as usize)
                                    .wrapping_mul(stride),
                            )
                        };
                        // SAFETY: `cur`'s `.face` is an in-range face index, so
                        // `face*stride` stays within the face segment of `values`.
                        let f0: *const u8 = unsafe {
                            face_values
                                .add(((*topo.add(cur as usize)).face as usize).wrapping_mul(stride))
                        };
                        // SAFETY: the grow above ensured `inputs` holds at least
                        // `num_inputs + 2` slots, so both slots are live.
                        unsafe {
                            (*inputs.add(num_inputs + 0)).data = e0 as *const c_void;
                            (*inputs.add(num_inputs + 1)).data = f0 as *const c_void;
                        }
                        num_inputs += 2;
                    }
                    valence += 1;

                    // If we landed at a split edge advance to the next one
                    // and continue from there in the outer loop
                    if split {
                        // SAFETY: `topo`/`num_topo` live; `cur` is an in-range corner.
                        start = unsafe { topo_next_vertex_edge(topo, num_topo, cur) };
                        num_split += 1;
                        break;
                    }
                }
            }

            if start == original_start {
                start = NO_INDEX;
            }

            // Weights for various subdivision masks
            let fe_weight: Real = 1.0 / (valence.wrapping_mul(valence) as Real);
            let v_weight: Real = (valence.wrapping_sub(2) as Real) / (valence as Real);

            // Select the right subdivision mask depending on valence and crease
            if num_crease > 2
                || (sharp_corners && valence == 2 && (num_split > 0 || on_boundary))
                || (sharp_splits && (num_split > 0 || on_boundary))
                || sharp_all
                || non_manifold
            {
                // Corner: Copy as-is
                // SAFETY: `inputs` holds at least one live slot.
                unsafe {
                    (*inputs.add(0)).data = v0 as *const c_void;
                    (*inputs.add(0)).weight = 1.0;
                }
                num_inputs = 1;
            } else if num_crease == 2 {
                // Boundary: Interpolate edge
                total_crease *= 0.5;
                // Explicit min/max mirror the C source; `f64::clamp` differs on NaN
                // (and panics if min > max), so it is not a faithful substitute.
                #[allow(clippy::manual_clamp)]
                if total_crease < 0.0 {
                    total_crease = 0.0;
                }
                if total_crease > 1.0 {
                    total_crease = 1.0;
                }

                // SAFETY: `inputs` holds at least one live slot.
                unsafe {
                    (*inputs.add(0)).weight = v_weight * (1.0 - total_crease) + 0.75 * total_crease;
                }
                let few: Real = fe_weight * (1.0 - total_crease);
                let mut i: usize = 1;
                while i < num_inputs {
                    // SAFETY: `i < num_inputs`, an in-range live `inputs` slot.
                    unsafe { (*inputs.add(i)).weight = few };
                    i += 1;
                }

                // Add weight to the creased edges
                // SAFETY: `crease_input_indices[0]`/`[1]` were recorded as input
                // slot indices below `num_inputs`, so both are live `inputs` slots.
                unsafe {
                    (*inputs.add(crease_input_indices[0])).weight += 0.125 * total_crease;
                    (*inputs.add(crease_input_indices[1])).weight += 0.125 * total_crease;
                }
            } else {
                // Regular: Weighted sum with the accumulated edge/face points
                // SAFETY: `inputs` holds at least one live slot.
                unsafe { (*inputs.add(0)).weight = v_weight };
                let mut i: usize = 1;
                while i < num_inputs {
                    // SAFETY: `i < num_inputs`, an in-range live `inputs` slot.
                    unsafe { (*inputs.add(i)).weight = fe_weight };
                    i += 1;
                }
            }

            if mesh.vertex_crease().exists() {
                let mut v: Real = get_vertex_real(mesh.vertex_crease(), original_start as usize);
                v *= 10.0 as Real;
                if v > 0.0 {
                    if v > 1.0 {
                        v = 1.0;
                    }

                    let iv: Real = 1.0 - v;
                    // SAFETY: `inputs` holds at least one live slot.
                    unsafe {
                        (*inputs.add(0)).weight = 1.0 * v + ((*inputs.add(0)).weight) * iv;
                    }
                    let mut i: usize = 1;
                    while i < num_inputs {
                        // SAFETY: `i < num_inputs`, an in-range live `inputs` slot.
                        unsafe { (*inputs.add(i)).weight *= iv };
                        i += 1;
                    }
                }
            }

            // C: `#if defined(UFBX_REGRESSION)`
            #[cfg(feature = "regression")]
            {
                let mut total_weight: Real = 0.0;
                let mut i: usize = 0;
                while i < num_inputs {
                    // SAFETY: `i < num_inputs`, an in-range live `inputs` slot.
                    total_weight += unsafe { (*inputs.add(i)).weight };
                    i += 1;
                }
                // C subtracts in `ufbx_real`, then `ufbx_fabs` (double-only)
                // promotes; `0.001f` promotes to double for the compare.
                ufbx_assert!(
                    crate::native::platform::math::fabs((total_weight - 1.0) as f64)
                        < 0.001f32 as f64
                );
            }

            ufbxi_check_err!(
                sc.error_view(),
                // SAFETY: `sum_fn`/`sum_user`/`dst` and the first `num_inputs`
                // `inputs` entries were set up above per the summer contract.
                unsafe { sum_fn(sum_user, dst as *mut c_void, inputs, num_inputs) } != 0,
                "sum_fn(sum_user, dst, inputs, num_inputs)"
            );
        }
        vi += 1;
    }

    // Copy non-manifold vertex values as-is
    let mut old_ix: usize = 0;
    while old_ix < mesh.num_indices() {
        // SAFETY: `old_ix < num_indices` is an in-range slot of `vertex_indices`.
        let mut ix: u32 = unsafe { *vertex_indices.add(old_ix) };
        if ix == NO_INDEX {
            ix = num_vertex_values as u32;
            num_vertex_values += 1;
            // SAFETY: `old_ix` is an in-range slot of `vertex_indices`.
            unsafe { *vertex_indices.add(old_ix) = ix };
            // SAFETY: `input` live; `indices[old_ix]` is an in-range index into
            // the value array whose `stride`-sized entry the byte offset into
            // `values` addresses.
            let src: *const u8 = unsafe {
                ((*input).values as *const u8)
                    .add((*(*input).indices.add(old_ix) as usize).wrapping_mul(stride))
            };
            // SAFETY: `ix < num_vertex_values <= num_indices`, so `ix*stride` is
            // within the vertex segment of `values` that `vertex_values` heads.
            let dst: *mut u8 = unsafe { vertex_values.add((ix as usize).wrapping_mul(stride)) };

            // SAFETY: `inputs` holds at least one live slot.
            unsafe {
                (*inputs.add(0)).data = src as *const c_void;
                (*inputs.add(0)).weight = 1.0;
            }
            ufbxi_check_err!(
                sc.error_view(),
                // SAFETY: `sum_fn`/`sum_user`/`dst` and the single input satisfy
                // the summer callback contract.
                unsafe { sum_fn(sum_user, dst as *mut c_void, inputs, 1) } != 0,
                "sum_fn(sum_user, dst, inputs, 1)"
            );
        }
        old_ix += 1;
    }

    ufbx_assert!(num_vertex_values <= mesh.num_indices());
    let num_values: usize = num_edge_values
        .wrapping_add(mesh.num_faces())
        .wrapping_add(num_vertex_values);
    // SAFETY: `result_mut_ptr()` is `sc`'s own live result `Buf`, the push contract.
    let mut new_values: *mut u8 =
        unsafe { push_size(sc.result_mut_ptr(), stride, num_values.wrapping_add(1)) } as *mut u8;
    ufbxi_check_err!(sc.error_view(), !new_values.is_null(), "new_values");

    // SAFETY: `new_values` is a `(num_values+1)*stride`-byte push, so the leading
    // `stride` bytes are writable.
    unsafe { core::ptr::write_bytes(new_values, 0, stride) };
    // SAFETY: advancing by the leading zero element keeps `new_values` within the
    // `num_values+1`-element buffer.
    new_values = unsafe { new_values.add(stride) };

    // SAFETY: `values` holds `num_initial_values >= num_values` `stride`-sized
    // elements and `new_values` now has room for `num_values` elements; the two
    // buffers are distinct allocations, so the copy is non-overlapping.
    unsafe { core::ptr::copy_nonoverlapping(values, new_values, num_values.wrapping_mul(stride)) };

    // SAFETY: `output` points to a live `SubdivideLayerOutput`.
    unsafe {
        (*output).values = new_values as *mut c_void;
        (*output).num_values = num_values;
    }

    // SAFETY: `input` points to a live `SubdivideLayerInput`.
    if !unsafe { (*input).ignore_indices } {
        let new_indices: *mut u32 = sc
            .result_view()
            .push::<u32>(mesh.num_indices().wrapping_mul(4));
        ufbxi_check_err!(sc.error_view(), !new_indices.is_null(), "new_indices");

        let face_start: u32 = 0;
        let edge_start: u32 = face_start.wrapping_add(mesh.num_faces() as u32);
        let vert_start: u32 = edge_start.wrapping_add(num_edge_values as u32);
        let mut p_ix: *mut u32 = new_indices;
        let mut ix: usize = 0;
        while ix < mesh.num_indices() {
            // SAFETY: the loop runs `num_indices` iterations advancing `p_ix` by 4
            // each time over the `num_indices*4`-element `new_indices` push, so
            // `p_ix.add(0..=3)` stay in-bounds; `ix < num_indices` indexes the
            // live `vertex_indices`/`edge_indices` arrays and `topo`, whose
            // `.prev` sibling corner is also an in-range `edge_indices` slot.
            unsafe {
                *p_ix.add(0) = vert_start.wrapping_add(*vertex_indices.add(ix));
                *p_ix.add(1) = edge_start.wrapping_add(*edge_indices.add(ix));
                *p_ix.add(2) = face_start.wrapping_add((*topo.add(ix)).face);
                *p_ix.add(3) =
                    edge_start.wrapping_add(*edge_indices.add((*topo.add(ix)).prev as usize));
                p_ix = p_ix.add(4);
            }
            ix += 1;
        }
        // SAFETY: `output` points to a live `SubdivideLayerOutput`.
        unsafe {
            (*output).indices = new_indices;
            (*output).num_indices = mesh.num_indices().wrapping_mul(4);
        }
    } else {
        // SAFETY: `output` points to a live `SubdivideLayerOutput`.
        unsafe {
            (*output).indices = core::ptr::null_mut();
            (*output).num_indices = 0;
        }
    }

    Ok(())
}

// Rust-port infrastructure (not a ufbx.c function): every `ufbxi_subdivide_attrib`
// call site in `ufbxi_subdivide_mesh_level` (ufbx.c:29657 onwards) passes
// `(ufbx_vertex_attrib*)&x->vertex_foo` — the type-erasing cast onto the shared
// `{ exists, values, indices, value_reals, ... }` prefix of `ufbx_vertex_vec2` /
// `_vec3` / `_vec4` / `_real`. This is that cast, in one place.
//
// # Safety
// `ptr` must address a live `ufbx_vertex_*` field laid out with the
// `ufbx_vertex_attrib` prefix, owned by a context or arena that keeps it alive
// and unmoved for `'a`, and its provenance must be write-capable (the
// `View<_, Mut>` mint vouch).
#[cfg(feature = "subdivision")]
#[inline(always)]
unsafe fn attrib_view<'a, T>(ptr: *mut T) -> &'a View<VertexAttrib> {
    // SAFETY: the caller vouches for liveness, the `ufbx_vertex_attrib` layout
    // prefix and write-capable provenance (fn contract above).
    unsafe { View::<VertexAttrib>::from_ptr(ptr as *mut VertexAttrib) }
}

// ufbx.c:29464-29489 `ufbxi_subdivide_attrib`
// Safe `fn`: the attribute arrives as a view; the residual raw ops address the
// two `MaybeUninit` locals this fn owns, the viewed attribute's list headers
// via `values_raw()`/`indices_raw()`, and the runs `subdivide_layer` walks —
// each vouched at its own block.
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) fn subdivide_attrib(
    sc: &SubdivideContext,
    attrib: &View<VertexAttrib>,
    boundary: SubdivisionBoundary,
    check_split_data: bool,
) -> Result<(), crate::native::error::Fail> {
    if !attrib.exists() {
        return Ok(());
    }

    ufbx_assert!(attrib.value_reals() >= 2 && attrib.value_reals() <= 4);

    let mut input_mem = MaybeUninit::<SubdivideLayerInput>::uninit(); // ufbxi_uninit
    let input: *mut SubdivideLayerInput = input_mem.as_mut_ptr();
    // SAFETY: `input` is the address of the local `input_mem`, so every field
    // write is in-bounds; the assert above bounds `value_reals` in 2..=4, keeping
    // the `REAL_SUM_FNS[value_reals-1]` index in 1..=3.
    unsafe {
        (*input).sum_fn = REAL_SUM_FNS[attrib.value_reals() - 1];
        (*input).sum_user = core::ptr::null_mut();
        (*input).values = attrib.values().data;
        (*input).indices = attrib.indices().data;
        (*input).stride = attrib.value_reals().wrapping_mul(size_of::<Real>());
        (*input).boundary = boundary;
        (*input).check_split_data = check_split_data;
        (*input).ignore_indices = false;
    }

    let mut output_mem = MaybeUninit::<SubdivideLayerOutput>::uninit(); // ufbxi_uninit
    let output: *mut SubdivideLayerOutput = output_mem.as_mut_ptr();
    // SAFETY: `output`/`input` address the local `output_mem`/`input_mem`; the
    // fields of `input` were fully initialized above, satisfying `subdivide_layer`.
    unsafe { subdivide_layer(sc, output, input) }?;

    // SAFETY: `output` addresses the fully-populated `output_mem` after a
    // successful `subdivide_layer`; `values_raw()`/`indices_raw()` address the
    // viewed attribute's own list headers.
    unsafe {
        (*attrib.values_raw()).data = (*output).values;
        (*attrib.indices_raw()).data = (*output).indices;
        (*attrib.values_raw()).count = (*output).num_values;
        (*attrib.indices_raw()).count = (*output).num_indices;
    }

    Ok(())
}

// ufbx.c:29491-29503 `ufbxi_subdivision_copy_weights`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) unsafe fn subdivision_copy_weights(
    sc: &SubdivideContext,
    ranges: crate::prelude::List<SubdivisionWeightRange>,
    weights: crate::prelude::List<SubdivisionWeight>,
) -> *mut SubdivisionVertexWeights {
    let dst: *mut SubdivisionVertexWeights =
        sc.tmp_view().push::<SubdivisionVertexWeights>(ranges.count);
    ufbxi_check_return_err!(
        sc.error_view(),
        !dst.is_null(),
        core::ptr::null_mut(),
        "dst"
    );

    // C: `ufbxi_nounroll for (size_t i = 0; i != ranges.count; i++)`
    let mut i: usize = 0;
    while i != ranges.count {
        // SAFETY: `i < ranges.count`, so `ranges.data.add(i)` is a live element,
        // read out by value.
        let range: SubdivisionWeightRange = unsafe { core::ptr::read(ranges.data.add(i)) };
        // SAFETY: `dst` is a fresh `ranges.count`-element push and `i` is in
        // range; `range.weight_begin` is an offset into the `weights` list this
        // range references, so `weights.data.add(weight_begin)` is in-bounds.
        unsafe {
            (*dst.add(i)).weights =
                (weights.data as *mut SubdivisionWeight).add(range.weight_begin as usize);
            (*dst.add(i)).num_weights = range.num_weights as usize;
        }
        i += 1;
    }

    dst
}

// ufbx.c:29505-29519 `ufbxi_init_source_vertex_weights`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) fn init_source_vertex_weights(
    sc: &SubdivideContext,
    num_vertices: usize,
) -> *mut SubdivisionVertexWeights {
    let (dst, weights): (*mut SubdivisionVertexWeights, *mut SubdivisionWeight) = (
        sc.tmp_view().push::<SubdivisionVertexWeights>(num_vertices),
        sc.tmp_view().push::<SubdivisionWeight>(num_vertices),
    );
    ufbxi_check_return_err!(
        sc.error_view(),
        !dst.is_null() && !weights.is_null(),
        core::ptr::null_mut(),
        "dst && weights"
    );

    // C: `ufbxi_nounroll for (size_t i = 0; i != num_vertices; i++)`
    let mut i: usize = 0;
    while i != num_vertices {
        // SAFETY: `i < num_vertices`, and both `dst` and `weights` are fresh
        // non-null `num_vertices`-element pushes (checked above), so each
        // element write is in bounds; the stored `weights.add(i)` points into
        // the same tmp arena `dst` lives in.
        unsafe {
            (*dst.add(i)).weights = weights.add(i);
            (*dst.add(i)).num_weights = 1;
            (*weights.add(i)).index = i as u32;
            (*weights.add(i)).weight = 1.0;
        }
        i += 1;
    }

    dst
}

// ufbx.c:29521-29546 `ufbxi_init_skin_weights`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) unsafe fn init_skin_weights(
    sc: &SubdivideContext,
    num_vertices: usize,
    skin: *const SkinDeformer,
) -> *mut SubdivisionVertexWeights {
    let dst: *mut SubdivisionVertexWeights =
        sc.tmp_view().push::<SubdivisionVertexWeights>(num_vertices);
    ufbxi_check_return_err!(
        sc.error_view(),
        !dst.is_null(),
        core::ptr::null_mut(),
        "dst"
    );

    // C: `const ufbx_skin_deformer *skin` — read-only for the whole call, so a
    // `Const` view (mintable from any readable provenance) is the honest mode.
    // SAFETY: `skin` points to a live `SkinDeformer` (fn contract) that nothing
    // in this call writes, satisfying the frozen-tag requirement.
    let skin: &View<SkinDeformer, Const> = unsafe { View::<SkinDeformer, Const>::from_ptr(skin) };

    let mut i: usize = 0;
    while i < num_vertices {
        ufbxi_dev_assert!(i < skin.vertices().count);
        // SAFETY: `skin.vertices.count >= num_vertices` by loaded-scene
        // consistency — a mesh's skin deformer carries an entry per vertex,
        // the assumption ufbx.c makes too — so `.add(i)` is a live element,
        // copied out by value; dev-asserted in dev/regression builds.
        let vertex: SkinVertex = unsafe { *skin.vertices().data.add(i) };
        let num_weights: usize = min_sz(sc.max_vertex_weights(), vertex.num_weights as usize);

        let weights: *mut SubdivisionWeight = sc.tmp_view().push::<SubdivisionWeight>(num_weights);
        // C: `ufbxi_check_err(&sc->error, weights);` — pointer-returning
        // function, so the C macro's `return 0` is NULL here.
        ufbxi_check_return_err!(
            sc.error_view(),
            !weights.is_null(),
            core::ptr::null_mut(),
            "weights"
        );

        // SAFETY: `vertex.weight_begin` indexes into the deformer's `weights`
        // list, so `.add(weight_begin)` points at this vertex's run.
        let skin_weights: *const SkinWeight =
            unsafe { skin.weights().data.add(vertex.weight_begin as usize) };

        // SAFETY: `dst` is a fresh `num_vertices`-element push and `i` is in
        // range, so slot `i` is live.
        unsafe {
            (*dst.add(i)).weights = weights;
            (*dst.add(i)).num_weights = num_weights;
        }
        // C: `ufbxi_nounroll for (size_t wi = 0; wi != num_weights; wi++)`
        let mut wi: usize = 0;
        while wi != num_weights {
            ufbxi_check_return_err!(
                sc.error_view(),
                // SAFETY: `num_weights <= vertex.num_weights`, so `wi < num_weights`
                // indexes within this vertex's `skin_weights` run.
                unsafe { (*skin_weights.add(wi)).cluster_index } <= i32::MAX as u32,
                core::ptr::null_mut(),
                "skin_weights[wi].cluster_index <= INT32_MAX"
            );
            // SAFETY: `wi < num_weights`, in range for both the fresh
            // `num_weights`-element `weights` push and the `skin_weights` run.
            unsafe {
                (*weights.add(wi)).index = (*skin_weights.add(wi)).cluster_index;
                (*weights.add(wi)).weight = (*skin_weights.add(wi)).weight;
            }
            wi += 1;
        }
        i += 1;
    }

    dst
}

// ufbx.c:29548-29594 `ufbxi_subdivide_weights`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) unsafe fn subdivide_weights(
    sc: &SubdivideContext,
    ranges: *mut crate::prelude::List<SubdivisionWeightRange>,
    weights: *mut crate::prelude::List<SubdivisionWeight>,
    src: *const SubdivisionVertexWeights,
) -> Result<(), crate::native::error::Fail> {
    ufbxi_check_err!(sc.error_view(), !src.is_null(), "src");

    let mut input_mem = MaybeUninit::<SubdivideLayerInput>::uninit(); // ufbxi_uninit
    let input: *mut SubdivideLayerInput = input_mem.as_mut_ptr();
    // SAFETY: `input` addresses the local `input_mem`, so every field write is
    // in-bounds; the accessor RHS values are safe reads of `sc`'s own state.
    unsafe {
        (*input).sum_fn = Some(subdivide_sum_vertex_weights);
        (*input).sum_user = (sc as *const SubdivideContext) as *mut c_void;
        (*input).values = src as *const c_void;
        (*input).indices = sc.src_mesh_view().vertex_indices_view().data();
        (*input).stride = size_of::<SubdivisionVertexWeights>();
        (*input).boundary = sc.opts_view().boundary();
        (*input).check_split_data = false;
        (*input).ignore_indices = true;
    }

    sc.set_total_weights(0);

    let mut output_mem = MaybeUninit::<SubdivideLayerOutput>::uninit(); // ufbxi_uninit
    let output: *mut SubdivideLayerOutput = output_mem.as_mut_ptr();
    // SAFETY: `output`/`input` address the local `output_mem`/`input_mem`; the
    // fields of `input` were fully initialized above, satisfying `subdivide_layer`.
    unsafe { subdivide_layer(sc, output, input) }?;

    // SAFETY: `output` addresses the populated `output_mem` after success.
    let num_vertices: usize = unsafe { (*output).num_values };
    ufbx_assert!(
        num_vertices
            == sc
                .dst_mesh_view()
                .vertex_position_view()
                .values_view()
                .count()
    );

    let dst_ranges: *mut SubdivisionWeightRange = sc
        .result_view()
        .push::<SubdivisionWeightRange>(num_vertices);
    let dst_weights: *mut SubdivisionWeight = sc
        .result_view()
        .push::<SubdivisionWeight>(sc.total_weights());
    // C-parity: upstream checks the OUT parameters `ranges && weights`, not the
    // freshly pushed `dst_ranges && dst_weights` (ufbx.c:29573) — ported
    // verbatim.
    ufbxi_check_err!(
        sc.error_view(),
        !ranges.is_null() && !weights.is_null(),
        "ranges && weights"
    );

    // SAFETY: `output` addresses the populated `output_mem`; its `values` is the
    // `num_vertices`-element `SubdivisionVertexWeights` buffer `subdivide_layer`
    // produced.
    let src_weights: *mut SubdivisionVertexWeights =
        unsafe { (*output).values as *mut SubdivisionVertexWeights };

    let mut weight_offset: usize = 0;
    let mut vi: usize = 0;
    while vi < num_vertices {
        // SAFETY: `vi < num_vertices`, in range for the `src_weights` buffer.
        let ws: SubdivisionVertexWeights = unsafe { *src_weights.add(vi) };
        ufbxi_check_err!(
            sc.error_view(),
            (u32::MAX as usize).wrapping_sub(weight_offset) >= ws.num_weights,
            "(size_t)UINT32_MAX - weight_offset >= ws.num_weights"
        );

        // SAFETY: `dst_ranges` is a fresh `num_vertices`-element push and `vi`
        // is in range; `sc.total_weights()` is the summer-accumulated sum of
        // every output value's `num_weights`, so `weight_offset +
        // ws.num_weights` stays within the `total_weights`-slot `dst_weights`
        // push and the copy of `ws.num_weights` from this vertex's
        // `ws.weights` run is in-bounds. The two pushes are distinct
        // allocations (non-overlapping). The check above is only a `u32`
        // overflow guard for `weight_begin`.
        unsafe {
            (*dst_ranges.add(vi)).weight_begin = weight_offset as u32;
            (*dst_ranges.add(vi)).num_weights = ws.num_weights as u32;
            core::ptr::copy_nonoverlapping(
                ws.weights,
                dst_weights.add(weight_offset),
                ws.num_weights,
            );
        }
        weight_offset = weight_offset.wrapping_add(ws.num_weights);
        vi += 1;
    }

    // SAFETY: `ranges`/`weights` are the non-null out-parameters checked above.
    unsafe {
        (*ranges).data = dst_ranges;
        (*ranges).count = num_vertices;
        (*weights).data = dst_weights;
        (*weights).count = sc.total_weights();
    }

    Ok(())
}

// ufbx.c:29596-29629 `ufbxi_subdivide_vertex_crease`
// Safe `fn`: both crease attributes arrive as views, and the residual raw ops
// index the freshly pushed `dst` runs and the `src` runs those views describe.
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) fn subdivide_vertex_crease<M: Mode>(
    sc: &SubdivideContext,
    dst: &View<VertexReal>,
    src: &View<VertexReal, M>,
) -> Result<(), crate::native::error::Fail> {
    let src_indices: usize = src.indices().count;
    let src_values: usize = src.values().count;

    // The pushed `values.data` is a `src_values+1`-element buffer.
    dst.values_view().set_count(src_values.wrapping_add(1));
    dst.values_view()
        .set_data(sc.result_view().push::<Real>(dst.values().count));
    ufbxi_check_err!(
        sc.error_view(),
        !dst.values().data.is_null(),
        "dst->values.data"
    );
    // SAFETY: `dst.values.data` holds `src_values+1` `Real`s, so slot `src_values`
    // (the trailing zero) is in-bounds.
    unsafe { *(dst.values().data as *mut Real).add(src_values) = 0.0 };

    // The pushed `indices.data` is a `src_indices*4`-element buffer.
    dst.indices_view().set_count(src_indices.wrapping_mul(4));
    dst.indices_view()
        .set_data(sc.result_view().push::<u32>(dst.indices().count));
    ufbxi_check_err!(
        sc.error_view(),
        !dst.indices().data.is_null(),
        "dst->indices.data"
    );

    // Reduce the amount of vertex crease on each iteration
    // C: `ufbxi_nounroll for (size_t i = 0; i < src_values; i++)`
    let mut i: usize = 0;
    while i < src_values {
        // SAFETY: `i < src_values`, in range for the live `src.values` array.
        let mut crease: Real = unsafe { *src.values().data.add(i) };
        // C: `0.999f` / `0.1f` are `float` literals widened to `ufbx_real`.
        if crease < 0.999f32 as Real {
            crease -= 0.1f32 as Real;
        }
        if crease < 0.0 {
            crease = 0.0;
        }
        // SAFETY: `i < src_values < src_values+1`, an in-range slot of the
        // freshly pushed `dst.values.data`.
        unsafe { *(dst.values().data as *mut Real).add(i) = crease };
        i += 1;
    }

    // Write the crease at the vertex corner and zero (at `src_values`) on other ones
    let zero_index: u32 = src_values as u32;
    // C: `ufbxi_nounroll for (size_t i = 0; i < src_indices; i++)`
    let mut i: usize = 0;
    while i < src_indices {
        // SAFETY: `i < src_indices`, so `i*4` addresses a live 4-slot quad within
        // the `src_indices*4`-element `dst.indices.data` push.
        let quad: *mut u32 = unsafe { (dst.indices().data as *mut u32).add(i.wrapping_mul(4)) };
        // SAFETY: `quad.add(0..=3)` are the four slots of this in-bounds quad;
        // `i < src_indices` indexes the live `src.indices` array.
        unsafe {
            *quad.add(0) = *src.indices().data.add(i);
            *quad.add(1) = zero_index;
            *quad.add(2) = zero_index;
            *quad.add(3) = zero_index;
        }
        i += 1;
    }

    Ok(())
}

// ufbx.c:29631-29925 `ufbxi_subdivide_mesh_level`
// Stays `unsafe fn`: the mesh fields run through `MeshView`, but the residual
// pointer arithmetic rests on index contracts carried by the *source mesh data*
// rather than anything checkable here — `topo[edge.a]`, `faces[topo[..].face]`
// and the per-face `face_material/smoothing/group/hole` writes at
// `index_offset + ci` are all only in bounds because the input mesh is
// internally consistent (`edge.a < num_indices`, face index ranges tiling
// `num_indices`). The narrow blocks below cite that one shared obligation.
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) unsafe fn subdivide_mesh_level(
    sc: &SubdivideContext,
) -> Result<(), crate::native::error::Fail> {
    let mesh: &MeshView = sc.src_mesh_view();
    let result: &MeshView = sc.dst_mesh_view();

    // C: `*result = *mesh;` — struct assignment (memcpy).
    // SAFETY: `mesh`/`result` view `sc`'s own distinct source/destination `Mesh`
    // slots, so the one-element copy between their own pointers is
    // non-overlapping.
    unsafe { core::ptr::copy_nonoverlapping(mesh.as_ptr(), result.get(), 1) };

    let topo: *mut TopoEdge = sc.tmp_view().push::<TopoEdge>(mesh.num_indices());
    ufbxi_check_err!(sc.error_view(), !topo.is_null(), "topo");
    // SAFETY: `topo` is the fresh non-null `num_indices`-element push just
    // checked, and `compute_topology` is handed exactly that mesh and count.
    unsafe { compute_topology(mesh.as_ptr(), topo, mesh.num_indices()) };
    sc.set_topo(topo);
    sc.set_num_topo(mesh.num_indices());

    subdivide_attrib(
        sc,
        // SAFETY: `vertex_position_raw()` addresses `result`'s own live
        // `vertex_position` field, reinterpreted as the type-erased attribute
        // `subdivide_attrib` subdivides in place (C's cast).
        unsafe { attrib_view(result.vertex_position_raw()) },
        sc.opts_view().boundary(),
        false,
    )?;

    // SAFETY: each `*_raw()` addresses a live vertex-attribute field of
    // `result`, zeroed in place to its own size (all-zero is a valid empty
    // `VertexVec*`).
    unsafe {
        core::ptr::write_bytes(
            result.vertex_uv_raw() as *mut u8,
            0,
            size_of::<crate::generated::VertexVec2>(),
        );
        core::ptr::write_bytes(
            result.vertex_tangent_raw() as *mut u8,
            0,
            size_of::<VertexVec3>(),
        );
        core::ptr::write_bytes(
            result.vertex_bitangent_raw() as *mut u8,
            0,
            size_of::<VertexVec3>(),
        );
        core::ptr::write_bytes(
            result.vertex_color_raw() as *mut u8,
            0,
            size_of::<crate::generated::VertexVec4>(),
        );
    }

    // One `ListView` mint per set list, serving the copy below, the
    // `ufbxi_for_list` walk and the `[0]` reads.
    // SAFETY: `uv_sets_raw()`/`color_sets_raw()` address `result`'s own live
    // list fields, carrying the context's write-capable provenance.
    let (uv_sets, color_sets) = unsafe {
        (
            ListView::from_ptr(result.uv_sets_raw()),
            ListView::from_ptr(result.color_sets_raw()),
        )
    };

    // SAFETY: `uv_sets` describes `result`'s live UV-set array, copied into
    // `sc`'s result arena via `push_copy`.
    uv_sets.set_data(unsafe {
        sc.result_view()
            .push_copy_raw::<UvSet>(uv_sets.count(), uv_sets.data())
    });
    ufbxi_check_err!(
        sc.error_view(),
        !uv_sets.data().is_null(),
        "result->uv_sets.data"
    );

    // SAFETY: `color_sets` describes `result`'s live color-set array, copied
    // into `sc`'s result arena via `push_copy`.
    color_sets.set_data(unsafe {
        sc.result_view()
            .push_copy_raw::<ColorSet>(color_sets.count(), color_sets.data())
    });
    ufbxi_check_err!(
        sc.error_view(),
        !color_sets.data().is_null(),
        "result->color_sets.data"
    );

    // C: `ufbxi_for_list(ufbx_uv_set, set, result->uv_sets)`
    {
        // SAFETY: `uv_sets` describes the contiguous run just copied into `sc`'s
        // result arena, live and unmoved for this call.
        let sets = unsafe {
            SliceViewIter::<UvSet>::from_raw_parts(uv_sets.data() as *mut UvSet, uv_sets.count())
        };
        for set in sets {
            subdivide_attrib(
                sc,
                // SAFETY: `vertex_uv_raw()` addresses this live `UvSet`'s own
                // attribute field, subdivided in place.
                unsafe { attrib_view(set.vertex_uv_raw()) },
                sc.opts_view().uv_boundary(),
                true,
            )?;
            if sc.opts_view().interpolate_tangents() {
                // SAFETY: as above, for this set's tangent and bitangent
                // attribute fields.
                unsafe {
                    subdivide_attrib(
                        sc,
                        attrib_view(set.vertex_tangent_raw()),
                        sc.opts_view().uv_boundary(),
                        true,
                    )?;
                    subdivide_attrib(
                        sc,
                        attrib_view(set.vertex_bitangent_raw()),
                        sc.opts_view().uv_boundary(),
                        true,
                    )?;
                }
            } else {
                // SAFETY: each `*_raw()` addresses a live attribute field of this
                // set, zeroed in place to its own size.
                unsafe {
                    core::ptr::write_bytes(
                        set.vertex_tangent_raw() as *mut u8,
                        0,
                        size_of::<VertexVec3>(),
                    );
                    core::ptr::write_bytes(
                        set.vertex_bitangent_raw() as *mut u8,
                        0,
                        size_of::<VertexVec3>(),
                    );
                }
            }
        }
    }

    // C: `ufbxi_for_list(ufbx_color_set, set, result->color_sets)`
    {
        // SAFETY: `color_sets` describes the contiguous run just copied into
        // `sc`'s result arena, live and unmoved for this call.
        let sets = unsafe {
            SliceViewIter::<ColorSet>::from_raw_parts(
                color_sets.data() as *mut ColorSet,
                color_sets.count(),
            )
        };
        for set in sets {
            subdivide_attrib(
                sc,
                // SAFETY: `vertex_color_raw()` addresses this live `ColorSet`'s
                // own attribute field, subdivided in place.
                unsafe { attrib_view(set.vertex_color_raw()) },
                sc.opts_view().uv_boundary(),
                true,
            )?;
        }
    }

    if uv_sets.count() > 0 {
        // C: struct assignments from `uv_sets.data[0]`.
        // SAFETY: the count is > 0, so element 0 of the run is live; each
        // destination `*_raw()` is a distinct live field of `result`, so every
        // one-element copy is non-overlapping.
        unsafe {
            let set0: &View<UvSet> = View::<UvSet>::from_ptr(uv_sets.data() as *mut UvSet);
            core::ptr::copy_nonoverlapping(set0.vertex_uv_raw(), result.vertex_uv_raw(), 1);
            core::ptr::copy_nonoverlapping(
                set0.vertex_bitangent_raw(),
                result.vertex_bitangent_raw(),
                1,
            );
            core::ptr::copy_nonoverlapping(
                set0.vertex_tangent_raw(),
                result.vertex_tangent_raw(),
                1,
            );
        }
    }
    if color_sets.count() > 0 {
        // SAFETY: the count is > 0, so element 0 of the run is live; the
        // destination is a distinct live field of `result`, so the copy is
        // non-overlapping.
        unsafe {
            let set0: &View<ColorSet> =
                View::<ColorSet>::from_ptr(color_sets.data() as *mut ColorSet);
            core::ptr::copy_nonoverlapping(set0.vertex_color_raw(), result.vertex_color_raw(), 1);
        }
    }

    if sc.opts_view().interpolate_normals() && !sc.opts_view().ignore_normals() {
        subdivide_attrib(
            sc,
            // SAFETY: `vertex_normal_raw()` addresses `result`'s own live normal
            // attribute, subdivided in place.
            unsafe { attrib_view(result.vertex_normal_raw()) },
            sc.opts_view().boundary(),
            true,
        )?;
        // C: `ufbxi_for_list(ufbx_vec3, normal, result->vertex_normal.values)`
        {
            let values = result.vertex_normal().values();
            // SAFETY: `values.data`/`.count` describe the live normal array, so
            // `normal..normal_end` spans it.
            let mut normal: *mut Vec3 = values.data as *mut Vec3;
            let normal_end: *mut Vec3 = unsafe { normal.add(values.count) };
            while normal != normal_end {
                // SAFETY: `normal` is an in-range live `Vec3`; `slow_normalize3`
                // reads it and the result is stored back in place.
                unsafe { *normal = slow_normalize3(normal) };
                // SAFETY: `normal != normal_end`, so advancing stays in the array.
                normal = unsafe { normal.add(1) };
            }
        }
        if mesh.skinned_normal().values().data == mesh.vertex_normal().values().data {
            // SAFETY: `vertex_normal`/`skinned_normal` are two distinct live
            // fields of `result`, so the one-element copy is non-overlapping.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    result.vertex_normal_raw(),
                    result.skinned_normal_raw(),
                    1,
                );
            }
        } else {
            subdivide_attrib(
                sc,
                // SAFETY: `skinned_normal_raw()` addresses `result`'s own live
                // skinned-normal attribute, subdivided in place.
                unsafe { attrib_view(result.skinned_normal_raw()) },
                sc.opts_view().boundary(),
                true,
            )?;
            // C: `ufbxi_for_list(ufbx_vec3, normal, result->skinned_normal.values)`
            let values = result.skinned_normal().values();
            // SAFETY: `values.data`/`.count` describe the live array, so
            // `normal..normal_end` spans it.
            let mut normal: *mut Vec3 = values.data as *mut Vec3;
            let normal_end: *mut Vec3 = unsafe { normal.add(values.count) };
            while normal != normal_end {
                // SAFETY: `normal` is an in-range live `Vec3`, normalized in place.
                unsafe { *normal = slow_normalize3(normal) };
                // SAFETY: `normal != normal_end`, so advancing stays in the array.
                normal = unsafe { normal.add(1) };
            }
        }
    }

    if result.vertex_crease().exists() {
        subdivide_vertex_crease(sc, result.vertex_crease(), mesh.vertex_crease())?;
    }

    if mesh.skinned_position().values().data == mesh.vertex_position().values().data {
        // SAFETY: `vertex_position`/`skinned_position` are two distinct live
        // fields of `result`, so the one-element copy is non-overlapping.
        unsafe {
            core::ptr::copy_nonoverlapping(
                result.vertex_position_raw(),
                result.skinned_position_raw(),
                1,
            );
        }
    } else {
        subdivide_attrib(
            sc,
            // SAFETY: `skinned_position_raw()` addresses `result`'s own live
            // skinned-position attribute, subdivided in place.
            unsafe { attrib_view(result.skinned_position_raw()) },
            sc.opts_view().boundary(),
            false,
        )?;
    }

    let result_sub: *mut SubdivisionResult = sc.result_view().push_zero::<SubdivisionResult>(1);
    ufbxi_check_err!(sc.error_view(), !result_sub.is_null(), "result_sub");
    // SAFETY: `result_sub` is the fresh non-null push above, wrapped into an
    // optional ref stored into the destination mesh.
    result.set_subdivision_result(unsafe { opt_ref(result_sub) });

    if sc.opts_view().evaluate_source_vertices() || sc.opts_view().evaluate_skin_weights() {
        // SAFETY: `subdivision_result_ptr()` addresses `mesh`'s own live
        // optional-ref field, from which `opt_ptr` reads the raw pointer (or null).
        let mesh_sub: *mut SubdivisionResult = unsafe { opt_ptr(mesh.subdivision_result_ptr()) };
        // The source mesh's previous-level result is read-only in this scope, so
        // it is viewed `Const` (the frozen tag holds: every write below targets
        // `result_sub`, a distinct fresh allocation).
        // SAFETY: `mesh_sub` is either null or the live `SubdivisionResult`
        // retained by `mesh`, which outlives this scope.
        let mesh_sub_view: Option<&View<SubdivisionResult, Const>> = if mesh_sub.is_null() {
            None
        } else {
            Some(unsafe { View::<SubdivisionResult, Const>::from_ptr(mesh_sub) })
        };
        // SAFETY: `result_sub` is the fresh non-null result-arena push above, so
        // the view carries that buffer's write-capable provenance; no other
        // reference to those bytes is formed while it is live.
        let result_sub_view: &View<SubdivisionResult> =
            unsafe { View::<SubdivisionResult>::from_ptr(result_sub) };

        let mut skin: *mut SkinDeformer = core::ptr::null_mut();
        if sc.opts_view().evaluate_skin_weights() {
            if mesh.skin_deformers().count > 0 {
                ufbxi_check_err!(
                    sc.error_view(),
                    sc.opts_view().skin_deformer_index() < mesh.skin_deformers().count,
                    "sc->opts.skin_deformer_index < mesh->skin_deformers.count"
                );
                // SAFETY: the check above bounds `skin_deformer_index` below
                // `skin_deformers.count`, so `.add(index)` is a live element whose
                // ref `ref_ptr` unwraps to a raw pointer.
                skin = unsafe {
                    ref_ptr(
                        mesh.skin_deformers()
                            .data
                            .add(sc.opts_view().skin_deformer_index()),
                    )
                };
            }
        }

        let mut max_weights: usize = 0;
        if sc.opts_view().evaluate_source_vertices() {
            max_weights = max_sz(max_weights, mesh.num_vertices());
        }
        if !skin.is_null() {
            // SAFETY: `skin` is the non-null live deformer resolved above, read
            // only here — a `Const` view scoped to this branch.
            let skin_view: &View<SkinDeformer, Const> =
                unsafe { View::<SkinDeformer, Const>::from_ptr(skin) };
            max_weights = max_sz(max_weights, skin_view.clusters().count);
        }

        sc.set_tmp_vertex_weights(sc.tmp_view().push_zero::<Real>(mesh.num_vertices()));
        sc.set_tmp_weights(sc.tmp_view().push::<SubdivisionWeight>(max_weights));
        ufbxi_check_err!(
            sc.error_view(),
            !sc.tmp_vertex_weights().is_null() && !sc.tmp_weights().is_null(),
            "sc->tmp_vertex_weights && sc->tmp_weights"
        );

        if sc.opts_view().evaluate_source_vertices() {
            sc.set_max_vertex_weights(if sc.opts_view().max_source_vertices() != 0 {
                sc.opts_view().max_source_vertices()
            } else {
                usize::MAX
            });

            let weights: *mut SubdivisionVertexWeights;
            if let Some(sub) = mesh_sub_view.filter(|sub| sub.source_vertex_ranges().count > 0) {
                // SAFETY: the list arguments are by-value copies of `mesh_sub`'s
                // own live source-vertex lists — `subdivision_copy_weights`'
                // contract.
                weights = unsafe {
                    subdivision_copy_weights(
                        sc,
                        sub.source_vertex_ranges(),
                        sub.source_vertex_weights(),
                    )
                };
            } else {
                weights = init_source_vertex_weights(sc, mesh.num_vertices());
            }

            // SAFETY: `weights` is the per-vertex weight array built just above,
            // and the out-params address `result_sub`'s own live source-vertex
            // list fields — `subdivide_weights`' contract.
            unsafe {
                subdivide_weights(
                    sc,
                    result_sub_view.source_vertex_ranges_raw(),
                    result_sub_view.source_vertex_weights_raw(),
                    weights,
                )
            }?;
        }

        if !skin.is_null() {
            sc.set_max_vertex_weights(if sc.opts_view().max_skin_weights() != 0 {
                sc.opts_view().max_skin_weights()
            } else {
                usize::MAX
            });

            let weights: *mut SubdivisionVertexWeights;
            // C-parity: the guard reads `source_vertex_ranges` here too
            // (ufbx.c:29750), not `skin_cluster_ranges`.
            if let Some(sub) = mesh_sub_view.filter(|sub| sub.source_vertex_ranges().count > 0) {
                // SAFETY: the list arguments are by-value copies of `mesh_sub`'s
                // own live skin-cluster lists — `subdivision_copy_weights`'
                // contract.
                weights = unsafe {
                    subdivision_copy_weights(
                        sc,
                        sub.skin_cluster_ranges(),
                        sub.skin_cluster_weights(),
                    )
                };
            } else {
                // SAFETY: `skin` is the non-null live deformer resolved above.
                weights = unsafe { init_skin_weights(sc, mesh.num_vertices(), skin) };
            }

            // SAFETY: `weights` is the per-vertex weight array built just above,
            // and the out-params address `result_sub`'s own live skin-cluster
            // list fields — `subdivide_weights`' contract.
            unsafe {
                subdivide_weights(
                    sc,
                    result_sub_view.skin_cluster_ranges_raw(),
                    result_sub_view.skin_cluster_weights_raw(),
                    weights,
                )
            }?;
        }
    }

    // The quad-subdivision counts derive from the source `num_indices`.
    result.set_num_vertices(result.vertex_position().values().count);
    result.set_num_indices(mesh.num_indices().wrapping_mul(4));
    result.set_num_faces(mesh.num_indices());
    result.set_num_triangles(mesh.num_indices().wrapping_mul(2));

    let vertex_indices: &ListView<u32> = result.vertex_indices_view();
    vertex_indices.set_data(result.vertex_position().indices().data);
    vertex_indices.set_count(result.num_indices());
    // SAFETY: `vertices_raw()` addresses `result`'s own live `vertices` list
    // field, carrying the context's write-capable provenance.
    let vertices: &ListView<Vec3> = unsafe { ListView::from_ptr(result.vertices_raw()) };
    vertices.set_data(result.vertex_position().values().data);
    vertices.set_count(result.num_vertices());

    // SAFETY: `faces_raw()` addresses `result`'s own live `faces` list field,
    // carrying the context's write-capable provenance.
    let faces: &ListView<Face> = unsafe { ListView::from_ptr(result.faces_raw()) };
    faces.set_count(result.num_faces());
    faces.set_data(sc.result_view().push::<Face>(result.num_faces()));
    ufbxi_check_err!(
        sc.error_view(),
        !faces.data().is_null(),
        "result->faces.data"
    );

    let mut i: usize = 0;
    while i < result.num_faces() {
        // SAFETY: `i < num_faces`, so `faces.data().add(i)` is a live slot of the
        // freshly pushed `num_faces`-element face array; one mint serves both
        // field writes.
        let face: &View<Face> =
            unsafe { View::<Face>::from_ptr((faces.data() as *mut Face).add(i)) };
        face.set_index_begin(i.wrapping_mul(4) as u32);
        face.set_num_indices(4);
        i += 1;
    }

    if !mesh.edges().data.is_null() {
        // One `ListView` mint per result edge list, serving the pushes below and
        // the per-edge/per-face fill loops.
        // SAFETY: each `*_raw()` addresses one of `result`'s own live list
        // fields, carrying the context's write-capable provenance.
        let edges: &ListView<Edge> = unsafe { ListView::from_ptr(result.edges_raw()) };
        let edge_crease: &ListView<Real> = result.edge_crease_view();
        // SAFETY: as above, for the smoothing and visibility lists.
        let (edge_smoothing, edge_visibility): (&ListView<bool>, &ListView<bool>) = unsafe {
            (
                ListView::from_ptr(result.edge_smoothing_raw()),
                ListView::from_ptr(result.edge_visibility_raw()),
            )
        };

        result.set_num_edges(
            mesh.num_edges()
                .wrapping_mul(2)
                .wrapping_add(result.num_faces()),
        );
        edges.set_count(result.num_edges());
        edges.set_data(sc.result_view().push::<Edge>(result.num_edges()));
        ufbxi_check_err!(
            sc.error_view(),
            !edges.data().is_null(),
            "result->edges.data"
        );

        if !mesh.edge_crease().data.is_null() {
            edge_crease.set_count(result.num_edges());
            edge_crease.set_data(sc.result_view().push::<Real>(result.num_edges()));
            ufbxi_check_err!(
                sc.error_view(),
                !edge_crease.data().is_null(),
                "result->edge_crease.data"
            );
        }
        if !mesh.edge_smoothing().data.is_null() {
            edge_smoothing.set_count(result.num_edges());
            edge_smoothing.set_data(sc.result_view().push::<bool>(result.num_edges()));
            ufbxi_check_err!(
                sc.error_view(),
                !edge_smoothing.data().is_null(),
                "result->edge_smoothing.data"
            );
        }
        if !mesh.edge_visibility().data.is_null() {
            edge_visibility.set_count(result.num_edges());
            edge_visibility.set_data(sc.result_view().push::<bool>(result.num_edges()));
            ufbxi_check_err!(
                sc.error_view(),
                !edge_visibility.data().is_null(),
                "result->edge_visibility.data"
            );
        }

        let mut di: usize = 0;
        let mut i: usize = 0;
        while i < mesh.num_edges() {
            // SAFETY: `i < num_edges` indexes the live source `edges` array;
            // `edge.a` is an in-range corner (source-mesh consistency), so
            // `topo.add(edge.a)` is live and its `.face` indexes the live `faces`
            // array; the two `di`/`di+1` targets sit within the `num_edges`-sized
            // (2 per source edge) result `edges` push.
            let edge: Edge = unsafe { *mesh.edges().data.add(i) };
            // SAFETY: as above; `edge.a` is an in-range corner into `topo`.
            let face_ix: u32 = unsafe { (*topo.add(edge.a as usize)).face };
            // SAFETY: `face_ix` is an in-range face index into the live `faces`.
            let face: Face = unsafe { *mesh.faces().data.add(face_ix as usize) };
            let offset: u32 = edge.a.wrapping_sub(face.index_begin);
            let next: u32 = (offset.wrapping_add(1)) % face.num_indices;

            let a: u32 = (face.index_begin.wrapping_add(offset)).wrapping_mul(4);
            let b: u32 = (face.index_begin.wrapping_add(next)).wrapping_mul(4);

            // SAFETY: `di`/`di+1` are within the result `edges` array (2 written
            // per source edge, `num_edges` total capacity); one mint per element
            // serves both of its field writes.
            let (e0, e1) = unsafe {
                (
                    View::<Edge>::from_ptr((edges.data() as *mut Edge).add(di + 0)),
                    View::<Edge>::from_ptr((edges.data() as *mut Edge).add(di + 1)),
                )
            };
            e0.set_a(a);
            e0.set_b(a.wrapping_add(1));
            e1.set_a(b.wrapping_add(3));
            e1.set_b(b);

            if !mesh.edge_crease().data.is_null() {
                // SAFETY: `i < num_edges` indexes the live source crease array.
                let mut crease: Real = unsafe { *mesh.edge_crease().data.add(i) };
                // C: `0.999f` is a `float` literal; `(ufbx_real)0.1` is not.
                if crease < 0.999f32 as Real {
                    crease -= 0.1 as Real;
                }
                if crease < 0.0 {
                    crease = 0.0;
                }
                // SAFETY: `di`/`di+1` are in-range slots of the `num_edges`-sized
                // result crease array pushed above.
                unsafe {
                    *(edge_crease.data() as *mut Real).add(di + 0) = crease;
                    *(edge_crease.data() as *mut Real).add(di + 1) = crease;
                }
            }

            if !mesh.edge_smoothing().data.is_null() {
                // SAFETY: `i` indexes the live source array; `di`/`di+1` are
                // in-range result slots.
                unsafe {
                    *(edge_smoothing.data() as *mut bool).add(di + 0) =
                        *mesh.edge_smoothing().data.add(i);
                    *(edge_smoothing.data() as *mut bool).add(di + 1) =
                        *mesh.edge_smoothing().data.add(i);
                }
            }

            if !mesh.edge_visibility().data.is_null() {
                // SAFETY: `i` indexes the live source array; `di`/`di+1` are
                // in-range result slots.
                unsafe {
                    *(edge_visibility.data() as *mut bool).add(di + 0) =
                        *mesh.edge_visibility().data.add(i);
                    *(edge_visibility.data() as *mut bool).add(di + 1) =
                        *mesh.edge_visibility().data.add(i);
                }
            }

            di += 2;
            i += 1;
        }

        let mut fi: usize = 0;
        while fi < result.num_faces() {
            // SAFETY: `di` continues past the `2*num_edges` per-edge writes with
            // one slot per face, staying within the `num_edges` result `edges`
            // capacity (`= 2*num_edges + num_faces`); one mint serves both field
            // writes.
            let e: &View<Edge> =
                unsafe { View::<Edge>::from_ptr((edges.data() as *mut Edge).add(di)) };
            e.set_a(fi.wrapping_mul(4).wrapping_add(1) as u32);
            e.set_b(fi.wrapping_mul(4).wrapping_add(2) as u32);

            if !edge_crease.data().is_null() {
                // SAFETY: `di` is an in-range slot of the result crease array.
                unsafe { *(edge_crease.data() as *mut Real).add(di) = 0.0 };
            }

            if !edge_smoothing.data().is_null() {
                // SAFETY: `di` is an in-range slot of the result smoothing array.
                unsafe { *(edge_smoothing.data() as *mut bool).add(di + 0) = true };
            }

            if !edge_visibility.data().is_null() {
                // SAFETY: `di` is an in-range slot of the result visibility array.
                unsafe { *(edge_visibility.data() as *mut bool).add(di + 0) = false };
            }

            di += 1;
            fi += 1;
        }
    }

    // One `ListView` mint per result face list, serving the pushes below and the
    // per-face fill loop.
    // SAFETY: each `*_raw()` addresses one of `result`'s own live list fields,
    // carrying the context's write-capable provenance.
    let (face_material, face_smoothing, face_group, face_hole) = unsafe {
        (
            ListView::from_ptr(result.face_material_raw()),
            ListView::from_ptr(result.face_smoothing_raw()),
            ListView::from_ptr(result.face_group_raw()),
            ListView::from_ptr(result.face_hole_raw()),
        )
    };

    if !mesh.face_material().data.is_null() {
        face_material.set_count(result.num_faces());
        face_material.set_data(sc.result_view().push::<u32>(result.num_faces()));
        ufbxi_check_err!(
            sc.error_view(),
            !face_material.data().is_null(),
            "result->face_material.data"
        );
    }
    if !mesh.face_smoothing().data.is_null() {
        face_smoothing.set_count(result.num_faces());
        face_smoothing.set_data(sc.result_view().push::<bool>(result.num_faces()));
        ufbxi_check_err!(
            sc.error_view(),
            !face_smoothing.data().is_null(),
            "result->face_smoothing.data"
        );
    }
    if !mesh.face_group().data.is_null() {
        face_group.set_count(result.num_faces());
        face_group.set_data(sc.result_view().push::<u32>(result.num_faces()));
        ufbxi_check_err!(
            sc.error_view(),
            !face_group.data().is_null(),
            "result->face_group.data"
        );
    }
    if !mesh.face_hole().data.is_null() {
        face_hole.set_count(result.num_faces());
        face_hole.set_data(sc.result_view().push::<bool>(result.num_faces()));
        ufbxi_check_err!(
            sc.error_view(),
            !face_hole.data().is_null(),
            "result->face_hole.data"
        );
    }

    if result.material_parts().count > 0 {
        // SAFETY: `material_parts_raw()` addresses `result`'s own live list
        // field, carrying the context's write-capable provenance.
        let material_parts: &ListView<MeshPart> =
            unsafe { ListView::from_ptr(result.material_parts_raw()) };
        material_parts.set_data(
            sc.result_view()
                .push_zero::<MeshPart>(material_parts.count()),
        );
        // C-parity: upstream checks `result->materials.data` here
        // (ufbx.c:29882), not the freshly pushed `material_parts.data`.
        ufbxi_check_err!(
            sc.error_view(),
            !result.materials().data.is_null(),
            "result->materials.data"
        );
    }

    let mut index_offset: usize = 0;
    let mut i: usize = 0;
    while i < mesh.num_faces() {
        // SAFETY: `i < num_faces` indexes the live source `faces` array.
        let face: Face = unsafe { *mesh.faces().data.add(i) };

        let mut mat: u32 = 0;
        if !mesh.face_material().data.is_null() {
            // SAFETY: `i` indexes the live source `face_material` array.
            mat = unsafe { *mesh.face_material().data.add(i) };
            let mut ci: usize = 0;
            while ci < face.num_indices as usize {
                // SAFETY: `index_offset + ci` addresses this face's contiguous
                // run within the `num_faces`-slot result array (face index ranges
                // tile the total, source-mesh consistency).
                unsafe {
                    *(face_material.data() as *mut u32).add(index_offset.wrapping_add(ci)) = mat;
                }
                ci += 1;
            }
        }
        // C: `mat` is otherwise unused (assigned and read only in the branch).
        let _ = mat;
        if !mesh.face_smoothing().data.is_null() {
            // SAFETY: `i` indexes the live source `face_smoothing` array.
            let flag: bool = unsafe { *mesh.face_smoothing().data.add(i) };
            let mut ci: usize = 0;
            while ci < face.num_indices as usize {
                // SAFETY: `index_offset + ci` is within this face's result run.
                unsafe {
                    *(face_smoothing.data() as *mut bool).add(index_offset.wrapping_add(ci)) = flag;
                }
                ci += 1;
            }
        }
        if !mesh.face_group().data.is_null() {
            // SAFETY: `i` indexes the live source `face_group` array.
            let group: u32 = unsafe { *mesh.face_group().data.add(i) };
            let mut ci: usize = 0;
            while ci < face.num_indices as usize {
                // SAFETY: `index_offset + ci` is within this face's result run.
                unsafe {
                    *(face_group.data() as *mut u32).add(index_offset.wrapping_add(ci)) = group;
                }
                ci += 1;
            }
        }
        if !mesh.face_hole().data.is_null() {
            // SAFETY: `i` indexes the live source `face_hole` array.
            let flag: bool = unsafe { *mesh.face_hole().data.add(i) };
            let mut ci: usize = 0;
            while ci < face.num_indices as usize {
                // SAFETY: `index_offset + ci` is within this face's result run.
                unsafe {
                    *(face_hole.data() as *mut bool).add(index_offset.wrapping_add(ci)) = flag;
                }
                ci += 1;
            }
        }
        index_offset = index_offset.wrapping_add(face.num_indices as usize);
        i += 1;
    }

    // Will be filled in by `ufbxi_finalize_mesh()`.
    result.vertex_first_index_view().set_count(0);

    // SAFETY: `error_mut_ptr()` is `sc`'s own live error slot, the finalize
    // contract.
    unsafe {
        finalize_mesh_material(sc.result_view(), sc.error_mut_ptr(), result)?;
        finalize_mesh(sc.result_view(), sc.error_mut_ptr(), result)?;
        update_face_groups(sc.result_view(), sc.error_mut_ptr(), result, true)?;
    }

    Ok(())
}

// ufbx.c:29927-30034 `ufbxi_subdivide_mesh_imp`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) fn subdivide_mesh_imp(
    sc: &SubdivideContext,
    level: usize,
) -> Result<FinishedImp<MeshImp>, crate::native::error::Fail> {
    if sc.opts_view().boundary() as u32 == SubdivisionBoundary::Default as u32 {
        sc.opts_view()
            .set_boundary(sc.src_mesh_view().subdivision_boundary());
    }

    if sc.opts_view().uv_boundary() as u32 == SubdivisionBoundary::Default as u32 {
        sc.opts_view()
            .set_uv_boundary(sc.src_mesh_view().subdivision_uv_boundary());
    }

    // SAFETY: initializing sc's own two allocators from sc's own error slot
    // and its own copy of the opts allocator descriptors, named by `'static`
    // NUL-terminated literals.
    unsafe {
        init_ator(
            sc.error_mut_ptr(),
            sc.ator_tmp_mut_ptr(),
            &sc.opts_view().temp_allocator(),
            c"temp",
        );
        init_ator(
            sc.error_mut_ptr(),
            sc.ator_result_mut_ptr(),
            &sc.opts_view().result_allocator(),
            c"result",
        );
    }

    sc.result_view().set_unordered(true);
    sc.source_view().set_unordered(true);
    sc.tmp_view().set_unordered(true);

    sc.source_view().set_ator(sc.ator_tmp_mut_ptr());
    sc.tmp_view().set_ator(sc.ator_tmp_mut_ptr());

    let mut i: usize = 1;
    while i < level {
        sc.result_view().set_ator(sc.ator_tmp_mut_ptr());

        // SAFETY: `sc` is a valid subdivide context (construction invariant);
        // its `src_mesh`/`dst_mesh` slots are two distinct fields of that
        // context, so the copy is non-overlapping, and the bufs freed and
        // rotated here are sc's own. `Buf` is a plain pointer/integer/bool
        // aggregate, so zeroing `result` leaves a valid empty buffer.
        unsafe {
            subdivide_mesh_level(sc)?;

            // C: `sc->src_mesh = sc->dst_mesh;` — struct assignment (memcpy).
            core::ptr::copy_nonoverlapping(sc.dst_mesh_mut_ptr(), sc.src_mesh_mut_ptr(), 1);

            buf_free(sc.source_mut_ptr());
            buf_free(sc.tmp_mut_ptr());
            sc.set_source(sc.take_result());
            core::ptr::write_bytes(sc.result_mut_ptr() as *mut u8, 0, size_of::<Buf>());
        }
        i += 1;
    }

    sc.result_view().set_ator(sc.ator_result_mut_ptr());
    // SAFETY: the final level and the tmp-buf teardown both act on sc's own
    // state (construction invariant).
    unsafe {
        subdivide_mesh_level(sc)?;
        buf_free(sc.tmp_mut_ptr());
    }

    let mesh: &MeshView = sc.dst_mesh_view();

    // Subdivision always results in a mesh that consists only of quads
    mesh.set_max_face_triangles(2);
    mesh.set_num_empty_faces(0);
    mesh.set_num_point_faces(0);
    mesh.set_num_line_faces(0);

    if !sc.opts_view().interpolate_normals() {
        // SAFETY: each `*_raw()` addresses a live `VertexVec3` field of sc's own
        // mesh slot, zeroed in place; all-zero is a valid `VertexVec3` (null
        // data, zero counts, `false` flags).
        unsafe {
            core::ptr::write_bytes(
                mesh.vertex_normal_raw() as *mut u8,
                0,
                size_of::<VertexVec3>(),
            );
            core::ptr::write_bytes(
                mesh.skinned_normal_raw() as *mut u8,
                0,
                size_of::<VertexVec3>(),
            );
        }
    }

    if !sc.opts_view().interpolate_normals() && !sc.opts_view().ignore_normals() {
        let topo: *mut TopoEdge = sc.tmp_view().push::<TopoEdge>(mesh.num_indices());
        ufbxi_check_err!(sc.error_view(), !topo.is_null(), "topo");
        // SAFETY: `topo` is the fresh non-null `num_indices`-element push just
        // checked, and `compute_topology` is handed exactly that mesh and that
        // `num_indices`-element run.
        unsafe { compute_topology(mesh.as_ptr(), topo, mesh.num_indices()) };

        let normal_indices: *mut u32 = sc.result_view().push::<u32>(mesh.num_indices());
        ufbxi_check_err!(sc.error_view(), !normal_indices.is_null(), "normal_indices");

        // SAFETY: `topo` and `normal_indices` are the fresh non-null
        // `num_indices`-element pushes just checked, and both counts passed
        // are that same `num_indices`.
        let num_normals: usize = unsafe {
            generate_normal_mapping(
                mesh.as_ptr(),
                topo,
                mesh.num_indices(),
                normal_indices,
                mesh.num_indices(),
                true,
            )
        };
        if num_normals == mesh.num_vertices() {
            mesh.skinned_normal().set_unique_per_vertex(true);
        }

        let mut normal_data: *mut Vec3 = sc.result_view().push::<Vec3>(num_normals.wrapping_add(1));
        ufbxi_check_err!(sc.error_view(), !normal_data.is_null(), "normal_data");
        // SAFETY: the push holds `num_normals + 1` elements, so element 0
        // exists and one past it is in bounds; `compute_normals` then fills
        // exactly the remaining `num_normals` through `normal_indices`, and
        // the mapping above guarantees those indices are `< num_normals`.
        unsafe {
            *normal_data.add(0) = ZERO_VEC3;
            normal_data = normal_data.add(1);

            compute_normals(
                mesh.as_ptr(),
                mesh.skinned_position().as_ptr(),
                normal_indices,
                mesh.num_indices(),
                normal_data,
                num_normals,
            );
        }

        mesh.set_generated_normals(true);
        mesh.vertex_normal().set_exists(true);
        mesh.vertex_normal().values_view().set_data(normal_data);
        mesh.vertex_normal().values_view().set_count(num_normals);
        mesh.vertex_normal().indices_view().set_data(normal_indices);
        mesh.vertex_normal()
            .indices_view()
            .set_count(mesh.num_indices());

        // SAFETY: `vertex_normal` and `skinned_normal` are distinct live fields
        // of the same mesh, so the copy is non-overlapping.
        unsafe {
            core::ptr::copy_nonoverlapping(mesh.vertex_normal_raw(), mesh.skinned_normal_raw(), 1);
        }
    }

    // SAFETY: `src_mesh_ptr` is the mesh handed to `ufbx_subdivide_mesh`, live
    // for the call by the public-API contract; `Const` because that pointer
    // traces to a public `*const Mesh` parameter, and nothing writes those bytes
    // while the view is held.
    let src_mesh: &View<Mesh, Const> = unsafe { View::<Mesh, Const>::from_ptr(sc.src_mesh_ptr()) };
    // SAFETY: when the source is an evaluated tessellated-NURBS mesh its wide
    // allocation is a `MeshImp`, otherwise it belongs to a scene whose
    // `SceneImp` owns it — the same discrimination C's `ufbxi_get_imp` calls
    // encode, and either parent outlives this call.
    let parent: *mut Refcount = unsafe {
        if src_mesh.subdivision_evaluated() && src_mesh.from_tessellated_nurbs() {
            ImpHandle::<MeshImp>::from_payload(sc.src_mesh_ptr()).refcount_ptr()
        } else {
            ImpHandle::<SceneImp>::from_payload(ref_ptr(
                &raw const (*sc.src_mesh_ptr()).element.scene,
            ))
            .refcount_ptr()
        }
    };

    // Patch sc's own destination mesh in place.
    patch_mesh_reals(mesh);

    sc.set_imp(sc.result_view().push::<MeshImp>(1));
    ufbxi_check_err!(sc.error_view(), !sc.imp().is_null(), "sc->imp");

    // SAFETY: `subdivide_mesh_level` always installs a `SubdivisionResult` on
    // the destination mesh (the `result_sub` push there), so this ref is
    // non-null and points into sc's own result arena.
    unsafe {
        let dst_sub: *mut SubdivisionResult = opt_ptr(mesh.subdivision_result_ptr());
        (*dst_sub).result_memory_used = sc.ator_result_view().current_size();
        (*dst_sub).temp_memory_used = sc.ator_tmp_view().current_size();
        (*dst_sub).result_allocs = sc.ator_result_view().num_allocs();
        (*dst_sub).temp_allocs = sc.ator_tmp_view().num_allocs();
    }

    // C: `ufbxi_init_ref(...)` / `sc->imp->magic = ...` / `sc->imp->mesh =
    // sc->dst_mesh` / `sc->imp->refcount.ator = sc->ator_result` /
    // `sc->imp->refcount.buf = sc->result` — the shared imp-finalization group.
    //
    // SAFETY: `sc.imp()` is the fresh non-null push just checked and the last
    // allocation of `sc->result`, so filling its header writes our own
    // allocation; `parent` is the live owner picked above; and `mesh.get()`
    // addresses sc's own `Mesh` slot, a distinct allocation from the pushed imp.
    let finished_imp = unsafe {
        finish_imp(
            sc.imp(),
            parent,
            mesh.get(),
            sc.ator_result(),
            sc.take_result(),
        )
    };

    // SAFETY: the imp header is fully initialized just above, so its `mesh`
    // payload is a live `Mesh` this call owns.
    unsafe { (*sc.imp()).mesh.subdivision_evaluated = true };

    Ok(finished_imp)
}

// ufbx.c:30036-30067 `ufbxi_subdivide_mesh`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) unsafe fn subdivide_mesh(
    mesh: *const Mesh,
    level: usize,
    user_opts: *const RawSubdivideOpts,
) -> Result<*mut Mesh, Error> {
    // C: `ufbxi_subdivide_context sc = { 0 };`
    // C: `ufbxi_subdivide_context sc = { 0 };`
    let sc = SubdivideContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::zeroed()));
    let sc = &sc;
    if !user_opts.is_null() {
        // C: `sc->opts = *user_opts;` — struct assignment (memcpy).
        // SAFETY: `user_opts` is a non-null live `RawSubdivideOpts` (caller
        // contract); `opts_mut_ptr()` is `sc`'s own distinct opts slot, so the
        // one-element copy is non-overlapping.
        unsafe { core::ptr::copy_nonoverlapping(user_opts, sc.opts_mut_ptr(), 1) };
    }

    sc.set_src_mesh_ptr(mesh as *mut Mesh);
    // C: `sc->src_mesh = *mesh;` — struct assignment (memcpy).
    // SAFETY: `mesh` points to a live `Mesh` (caller contract); `src_mesh_mut_ptr`
    // is `sc`'s own distinct source slot, so the one-element copy is
    // non-overlapping.
    unsafe { core::ptr::copy_nonoverlapping(mesh, sc.src_mesh_mut_ptr(), 1) };

    // C: `int ok = ufbxi_subdivide_mesh_imp(sc, level);` — on success the
    // `FinishedImp` carries the finished imp through the shared teardown to the
    // return below.
    let result = subdivide_mesh_imp(sc, level);

    // SAFETY: `ator_tmp_mut_ptr()`/`inputs()`/`inputs_cap()` are `sc`'s own
    // allocator and the `inputs` array (with its capacity) it allocated, the
    // free contract.
    unsafe { free::<SubdivideInput>(sc.ator_tmp_mut_ptr(), sc.inputs(), sc.inputs_cap()) };
    // SAFETY: `tmp`/`source` are `sc`'s own live scratch bufs.
    unsafe {
        buf_free(sc.tmp_mut_ptr());
        buf_free(sc.source_mut_ptr());
    }

    if let Ok(finished_imp) = result {
        // SAFETY: `ator_tmp_mut_ptr()` is `sc`'s own live temp allocator.
        unsafe { free_ator(sc.ator_tmp_mut_ptr()) };

        // C: `return &sc->imp->mesh;` — commit the finished imp across the ABI.
        // (The success-path `clear_error` of the caller's slot lives in the
        // boundary shim.)
        Ok(finished_imp.into_payload())
    } else {
        // SAFETY: `error_mut_ptr()` is `sc`'s own live error slot; the description
        // C copies the fixed error into the caller's slot; the `Result` shape
        // carries it by value (the shim owns the slot writes).
        let mut fixed: Error = Error::default();
        // is a `'static` NUL-terminated literal; `&raw mut fixed` is this
        // frame's live `Error`, which `fix_error_type` accepts.
        unsafe {
            fix_error_type(sc.error_mut_ptr(), b"Failed to subdivide\0", &raw mut fixed);
        }
        // SAFETY: `result` is `sc`'s own live scratch buf.
        unsafe { buf_free(sc.result_mut_ptr()) };
        // SAFETY: both allocators are `sc`'s own live temp/result allocators.
        unsafe {
            free_ator(sc.ator_tmp_mut_ptr());
            free_ator(sc.ator_result_mut_ptr());
        }
        Err(fixed)
    }
}

// ufbx.c:30071-30079 `ufbxi_subdivide_mesh` (`#else` — feature disabled)
#[cfg(not(feature = "subdivision"))]
#[inline(never)]
pub(crate) unsafe fn subdivide_mesh(
    mesh: *const Mesh,
    level: usize,
    user_opts: *const RawSubdivideOpts,
) -> Result<*mut Mesh, Error> {
    // C: `mesh`/`level`/`user_opts` are unreferenced in the `#else` arm.
    let _ = (mesh, level, user_opts);
    // C zero-fills the caller slot then formats into it; the `Result` shape
    // builds the same bytes in a local carried by `Err` (the shim owns the
    // slot writes).
    let mut error: Error = Error::default();
    // SAFETY: `&raw mut error` is this frame's live `Error` slot whose info
    // buffer the macro formats into.
    unsafe { ufbxi_fmt_err_info!(&raw mut error, "UFBX_ENABLE_SUBDIVISION") };
    ufbxi_report_err_msg!(
        // SAFETY: same live local `Error` slot, minted as a view for the report.
        unsafe { crate::native::error::ErrorView::from_ptr(&raw mut error) },
        "UFBXI_FEATURE_SUBDIVISION",
        "Feature disabled"
    );
    Err(error)
}
