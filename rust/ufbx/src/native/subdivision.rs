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
    ColorSet, Edge, Error, Face, Mesh, MeshPart, RawSubdivideOpts, SkinDeformer,
    SubdivisionBoundary, SubdivisionResult, SubdivisionWeight, SubdivisionWeightRange, TopoEdge,
    TopoFlags, UvSet, VertexAttrib, VertexReal, VertexVec3,
};
#[cfg(not(feature = "subdivision"))]
use crate::generated::{Error, Mesh, RawSubdivideOpts};
#[cfg(feature = "subdivision")]
use crate::generated::{Vec2, Vec3, Vec4};
#[cfg(feature = "subdivision")]
use crate::native::allocator::{does_overflow, free, free_ator, grow_array, init_ator, Allocator};
#[cfg(feature = "subdivision")]
use crate::native::api::{
    catch_topo_next_vertex_edge_run, catch_topo_prev_vertex_edge_run, compute_normals,
    compute_topology, generate_normal_mapping, get_vertex_real, ZERO_VEC3,
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
use crate::native::read::{finalize_mesh, patch_mesh_reals, ref_ptr, update_face_groups};
#[cfg(feature = "subdivision")]
use crate::native::scene_process::finalize_mesh_material;
#[cfg(feature = "subdivision")]
use crate::native::string_pool::slow_normalize3;
#[cfg(feature = "subdivision")]
use crate::native::view::view_raw_mut;
#[cfg(feature = "subdivision")]
use crate::native::view::view_read_shared;
use crate::native::view::{view_project, view_read, view_write};
#[cfg(feature = "subdivision")]
use crate::native::view::{Const, Mode, Run, View};
#[cfg(feature = "subdivision")]
use crate::prelude::{List, ListView, Real};
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

#[cfg(feature = "subdivision")]
impl<M: Mode> View<SubdivideInput, M> {
    #[inline(always)]
    pub(crate) fn data(&self) -> *const c_void {
        view_read_shared!(self, data)
    }

    #[inline(always)]
    pub(crate) fn weight(&self) -> Real {
        view_read_shared!(self, weight)
    }
}

#[cfg(feature = "subdivision")]
impl View<SubdivideInput> {
    #[inline(always)]
    pub(crate) fn set_data(&self, data: *const c_void) {
        view_write!(self, data, data)
    }

    #[inline(always)]
    pub(crate) fn set_weight(&self, weight: Real) {
        view_write!(self, weight, weight)
    }
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

// Mode-generic read accessors over a `SubdivideLayerInput` view: the struct is
// read-only for the whole of `ufbxi_subdivide_layer`, so a `Const`-rooted view
// serves every consumer.
#[cfg(feature = "subdivision")]
impl<M: Mode> View<SubdivideLayerInput, M> {
    #[inline(always)]
    pub(crate) fn values(&self) -> *const c_void {
        view_read_shared!(self, values)
    }

    #[inline(always)]
    pub(crate) fn stride(&self) -> usize {
        view_read_shared!(self, stride)
    }

    #[inline(always)]
    pub(crate) fn indices(&self) -> *const u32 {
        view_read_shared!(self, indices)
    }

    #[inline(always)]
    pub(crate) fn check_split_data(&self) -> bool {
        view_read_shared!(self, check_split_data)
    }

    #[inline(always)]
    pub(crate) fn ignore_indices(&self) -> bool {
        view_read_shared!(self, ignore_indices)
    }

    #[inline(always)]
    pub(crate) fn boundary(&self) -> SubdivisionBoundary {
        view_read_shared!(self, boundary)
    }

    #[inline(always)]
    pub(crate) fn sum_fn(&self) -> Option<SubdivideSumFn> {
        view_read_shared!(self, sum_fn)
    }

    #[inline(always)]
    pub(crate) fn sum_user(&self) -> *mut c_void {
        view_read_shared!(self, sum_user)
    }
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

// Checked read/write surface over the subdivision layer out-struct. Callers
// read these fields only after a successful `subdivide_layer()` initializes
// them; the layer itself writes them in C statement order through the setters.
#[cfg(feature = "subdivision")]
impl<M: Mode> View<SubdivideLayerOutput, M> {
    #[inline(always)]
    pub(crate) fn values(&self) -> *mut c_void {
        view_read_shared!(self, values)
    }

    #[inline(always)]
    pub(crate) fn num_values(&self) -> usize {
        view_read_shared!(self, num_values)
    }

    #[inline(always)]
    pub(crate) fn indices(&self) -> *mut u32 {
        view_read_shared!(self, indices)
    }

    #[inline(always)]
    pub(crate) fn num_indices(&self) -> usize {
        view_read_shared!(self, num_indices)
    }
}

#[cfg(feature = "subdivision")]
impl View<SubdivideLayerOutput> {
    #[inline(always)]
    pub(crate) fn set_values(&self, values: *mut c_void) {
        view_write!(self, values, values)
    }

    #[inline(always)]
    pub(crate) fn set_num_values(&self, num_values: usize) {
        view_write!(self, num_values, num_values)
    }

    #[inline(always)]
    pub(crate) fn set_indices(&self, indices: *mut u32) {
        view_write!(self, indices, indices)
    }

    #[inline(always)]
    pub(crate) fn set_num_indices(&self, num_indices: usize) {
        view_write!(self, num_indices, num_indices)
    }

    #[inline(always)]
    pub(crate) fn set_unique_per_vertex(&self, unique_per_vertex: bool) {
        view_write!(self, unique_per_vertex, unique_per_vertex)
    }
}

// ufbx.c:28856-28859 `ufbxi_subdivision_vertex_weights`
#[cfg(feature = "subdivision")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct SubdivisionVertexWeights {
    pub weights: *mut SubdivisionWeight,
    pub num_weights: usize,
}

// Checked read/write surfaces for the subdivision-only weight carriers.
#[cfg(feature = "subdivision")]
impl<M: Mode> View<SubdivisionWeightRange, M> {
    #[inline(always)]
    pub(crate) fn weight_begin(&self) -> u32 {
        view_read_shared!(self, weight_begin)
    }

    #[inline(always)]
    pub(crate) fn num_weights(&self) -> u32 {
        view_read_shared!(self, num_weights)
    }
}

#[cfg(feature = "subdivision")]
impl<M: Mode> View<SubdivisionWeight, M> {
    #[inline(always)]
    pub(crate) fn weight(&self) -> Real {
        view_read_shared!(self, weight)
    }

    #[inline(always)]
    pub(crate) fn index(&self) -> u32 {
        view_read_shared!(self, index)
    }
}

#[cfg(feature = "subdivision")]
impl View<SubdivisionWeight> {
    #[inline(always)]
    pub(crate) fn set_weight(&self, weight: Real) {
        view_write!(self, weight, weight)
    }

    #[inline(always)]
    pub(crate) fn set_index(&self, index: u32) {
        view_write!(self, index, index)
    }
}

#[cfg(feature = "subdivision")]
impl View<SubdivisionVertexWeights> {
    #[inline(always)]
    pub(crate) fn set_weights(&self, weights: *mut SubdivisionWeight) {
        view_write!(self, weights, weights)
    }

    #[inline(always)]
    pub(crate) fn set_num_weights(&self, num_weights: usize) {
        view_write!(self, num_weights, num_weights)
    }
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
    pub(crate) fn skin_deformer_index(&self) -> usize {
        view_read!(self, skin_deformer_index)
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

// Mode-generic nested views over the two allocator descriptors: `init_ator`
// only reads them, so the accessor serves a `Mut` context field and a `Const`
// boundary mint alike.
impl<M: crate::native::view::Mode> crate::native::view::View<RawSubdivideOpts, M> {
    #[inline(always)]
    pub(crate) fn temp_allocator_view(
        &self,
    ) -> &crate::native::view::View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, temp_allocator)
    }
    #[inline(always)]
    pub(crate) fn result_allocator_view(
        &self,
    ) -> &crate::native::view::View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, result_allocator)
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

    // `src_mesh` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn src_mesh_mut_ptr(&self) -> *mut Mesh {
        view_raw_mut!(self, src_mesh)
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
    // SAFETY: the callback contract supplies `num_inputs` initialized, aligned
    // input records that stay stable and frozen through the walk, plus one
    // stable, aligned and write-capable `Vec2` output slot. The record span and
    // output slot are distinct.
    let (inputs, output) = unsafe {
        (
            Run::<SubdivideInput, Const>::from_const_raw_parts(inputs, num_inputs),
            Run::<Vec2>::from_raw_parts(output.cast::<Vec2>(), 1),
        )
    };
    // C: `ufbxi_nounroll for (size_t i = 0; i != num_inputs; i++)`
    let mut i: usize = 0;
    while i != num_inputs {
        let input = inputs.at(i);
        let src_ptr = input.data().cast::<Vec2>();
        let weight = input.weight();
        // SAFETY: each type-erased data pointer addresses one initialized,
        // aligned `Vec2` that stays stable and frozen through this scoped copy
        // because this is the registered Vec2 sum callback.
        let src = {
            let src = unsafe { Run::<Vec2, Const>::from_const_raw_parts(src_ptr, 1) };
            src.copy_at(0)
        };
        dst.x += src.x * weight;
        dst.y += src.y * weight;
        i += 1;
    }
    output.write_at(0, dst);

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
    // SAFETY: the callback contract supplies `num_inputs` initialized, aligned
    // input records that stay stable and frozen through the walk, plus one
    // stable, aligned and write-capable `Vec3` output slot. The record span and
    // output slot are distinct.
    let (inputs, output) = unsafe {
        (
            Run::<SubdivideInput, Const>::from_const_raw_parts(inputs, num_inputs),
            Run::<Vec3>::from_raw_parts(output.cast::<Vec3>(), 1),
        )
    };
    // C: `ufbxi_nounroll for (size_t i = 0; i != num_inputs; i++)`
    let mut i: usize = 0;
    while i != num_inputs {
        let input = inputs.at(i);
        let src_ptr = input.data().cast::<Vec3>();
        let weight = input.weight();
        // SAFETY: each type-erased data pointer addresses one initialized,
        // aligned `Vec3` that stays stable and frozen through this scoped copy
        // because this is the registered Vec3 sum callback.
        let src = {
            let src = unsafe { Run::<Vec3, Const>::from_const_raw_parts(src_ptr, 1) };
            src.copy_at(0)
        };
        dst.x += src.x * weight;
        dst.y += src.y * weight;
        dst.z += src.z * weight;
        i += 1;
    }
    output.write_at(0, dst);

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
    // SAFETY: the callback contract supplies `num_inputs` initialized, aligned
    // input records that stay stable and frozen through the walk, plus one
    // stable, aligned and write-capable `Vec4` output slot. The record span and
    // output slot are distinct.
    let (inputs, output) = unsafe {
        (
            Run::<SubdivideInput, Const>::from_const_raw_parts(inputs, num_inputs),
            Run::<Vec4>::from_raw_parts(output.cast::<Vec4>(), 1),
        )
    };
    // C: `ufbxi_nounroll for (size_t i = 0; i != num_inputs; i++)`
    let mut i: usize = 0;
    while i != num_inputs {
        let input = inputs.at(i);
        let src_ptr = input.data().cast::<Vec4>();
        let weight = input.weight();
        // SAFETY: each type-erased data pointer addresses one initialized,
        // aligned `Vec4` that stays stable and frozen through this scoped copy
        // because this is the registered Vec4 sum callback.
        let src = {
            let src = unsafe { Run::<Vec4, Const>::from_const_raw_parts(src_ptr, 1) };
            src.copy_at(0)
        };
        dst.x += src.x * weight;
        dst.y += src.y * weight;
        dst.z += src.z * weight;
        dst.w += src.w * weight;
        i += 1;
    }
    output.write_at(0, dst);

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
    // SAFETY: the callback contract supplies a live `SubdivideContext` and
    // `num_inputs` initialized, aligned input records that stay stable and
    // frozen through the walk. This callback is registered only after the
    // context's checked zero-push allocates one initialized accumulator per
    // source vertex; tmp-arena allocations stay stable during the callback.
    let (sc, inputs, vertex_weights) = unsafe {
        let sc = &*(user as *const SubdivideContext);
        let inputs = Run::<SubdivideInput, Const>::from_const_raw_parts(inputs, num_inputs);
        let vertex_weights =
            Run::<Real>::from_raw_parts(sc.tmp_vertex_weights(), sc.src_mesh_view().num_vertices());
        (sc, inputs, vertex_weights)
    };

    let tmp_weights: *mut SubdivisionWeight = sc.tmp_weights();
    let mut num_weights: usize = 0;

    // C: `ufbxi_nounroll for (size_t input_ix = 0; input_ix != num_inputs; input_ix++)`
    let mut input_ix: usize = 0;
    while input_ix != num_inputs {
        let input = inputs.at(input_ix);
        // SAFETY: this callback's type-erased input data addresses one
        // initialized, aligned `SubdivisionVertexWeights` record that stays
        // stable and frozen through this scoped copy.
        let src = {
            let src = unsafe {
                Run::<SubdivisionVertexWeights, Const>::from_const_raw_parts(
                    input.data().cast::<SubdivisionVertexWeights>(),
                    1,
                )
            };
            src.copy_at(0)
        };
        let input_weight = input.weight();
        // SAFETY: the source record describes an initialized, aligned weight
        // run that stays stable and frozen for this input's accumulation. It
        // does not overlap the mutable callback scratch or output slot.
        let src_weights = unsafe {
            Run::<SubdivisionWeight, Const>::from_const_raw_parts(src.weights, src.num_weights)
        };

        let mut weight_ix: usize = 0;
        while weight_ix < src.num_weights {
            let src_weight = src_weights.at(weight_ix);
            let weight: Real = input_weight * src_weight.weight();
            // C: `if (weight < 1.175494351e-38f) continue;` — a `float` literal
            // widened to `ufbx_real`.
            if weight < 1.175494351e-38f32 as Real {
                weight_ix += 1;
                continue;
            }

            let vx: u32 = src_weight.index();
            ufbxi_dev_assert!((vx as usize) < sc.src_mesh_view().num_vertices());

            // SAFETY: the checked accumulator slot was initialized by the
            // zero-push and stays initialized between updates. A malformed
            // out-of-range `vx` is rejected by `subrun()` before this read.
            let prev: Real = unsafe { *vertex_weights.subrun(vx as usize, 1).as_ptr() };
            vertex_weights.write_at(vx as usize, prev + weight);
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

    // Every appended record has an initialized index leaf; weight leaves are
    // initialized by the following loop. Distinct accepted indices are bounded
    // by the active source-vertex or skin-cluster domain used to size the
    // `tmp_weights` allocation.
    // SAFETY: the callback accumulation above appended exactly `num_weights`
    // dense entries within that stable, write-capable scratch allocation.
    let tmp_weights_write =
        unsafe { Run::<SubdivisionWeight>::from_raw_parts(tmp_weights, num_weights) };

    // C: `ufbxi_nounroll for (size_t i = 0; i != num_weights; i++)`
    let mut i: usize = 0;
    while i != num_weights {
        let tmp_weight = tmp_weights_write.at(i);
        let vx = tmp_weight.index();
        // SAFETY: `vx` was accepted into the checked accumulator run above, and
        // that initialized slot has not been reset yet.
        let weight = unsafe { *vertex_weights.subrun(vx as usize, 1).as_ptr() };
        tmp_weight.set_weight(weight);
        vertex_weights.write_at(vx as usize, 0.0);
        i += 1;
    }

    // SAFETY: `tmp_weights` holds `num_weights` live `SubdivisionWeight` entries
    // of the passed element size; `subdivision_weight_less` is a matching
    // comparator and the `null` user pointer is unused by it.
    unsafe {
        unstable_sort(
            tmp_weights_write.as_mut_ptr() as *mut c_void,
            num_weights,
            size_of::<SubdivisionWeight>(),
            subdivision_weight_less,
            core::ptr::null_mut(),
        );
    }

    if sc.max_vertex_weights() != usize::MAX {
        num_weights = min_sz(sc.max_vertex_weights(), num_weights);
        let retained_weights = tmp_weights_write.subrun(0, num_weights);

        // Normalize weights
        let mut prefix_weight: Real = 0.0;
        // C: `ufbxi_nounroll for (size_t i = 0; i != num_weights; i++)`
        let mut i: usize = 0;
        while i != num_weights {
            prefix_weight += retained_weights.at(i).weight();
            i += 1;
        }
        let mut i: usize = 0;
        while i != num_weights {
            let weight = retained_weights.at(i);
            weight.set_weight(weight.weight() / prefix_weight);
            i += 1;
        }
    }

    let retained_weights = tmp_weights_write.subrun(0, num_weights);
    sc.set_total_weights(sc.total_weights().wrapping_add(num_weights));
    // SAFETY: `tmp_view()` is `sc`'s own live scratch `Buf`; `tmp_weights`
    // holds the checked, fully initialized retained prefix to copy from.
    let weights: *mut SubdivisionWeight = unsafe {
        sc.tmp_view()
            .push_copy_raw::<SubdivisionWeight>(num_weights, retained_weights.as_ptr())
    };
    // C: `ufbxi_check_err(&sc->error, weights);` — this function returns `int`,
    // so the C macro's `return 0` is a plain 0 here.
    ufbxi_check_return_err!(sc.error_view(), !weights.is_null(), 0, "weights");

    // SAFETY: after the fallible allocation succeeds, the callback contract
    // supplies one stable, aligned, write-capable output slot. It is distinct
    // from the input records, source weight runs and callback scratch.
    let output = unsafe {
        Run::<SubdivisionVertexWeights>::from_raw_parts(
            output.cast::<SubdivisionVertexWeights>(),
            1,
        )
    };
    let dst = output.at(0);
    dst.set_weights(weights);
    dst.set_num_weights(num_weights);

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
/// # Safety
/// Each index stored in `input_indices` addresses a `stride`-byte value in
/// `input.values`. That value-buffer length is a caller promise the parameter
/// types cannot carry.
pub(crate) unsafe fn is_edge_split(
    input: &View<SubdivideLayerInput, Const>,
    input_indices: Run<'_, u32, Const>,
    topo: Run<'_, TopoEdge, Const>,
    index: u32,
) -> bool {
    let topo_edge = topo.at(index as usize);
    let twin = topo_edge.twin();
    if twin != NO_INDEX {
        let a0 = input_indices.copy_at(index as usize);
        let a1 = input_indices.copy_at(topo_edge.next() as usize);
        let twin_topo = topo.at(twin as usize);
        let b0 = input_indices.copy_at(twin_topo.next() as usize);
        let b1 = input_indices.copy_at(twin as usize);
        if a0 == b0 && a1 == b1 {
            return false;
        }
        if !input.check_split_data() {
            return true;
        }
        let stride: usize = input.stride();
        // SAFETY: `input.values()` is a byte-addressed attribute buffer holding
        // `stride` bytes per value; `a0`/`a1`/`b0`/`b1` come from `input_indices`,
        // which maps corners to indices into that value array (not to mesh
        // vertices), so every `.add(ix*stride)` byte offset stays within the buffer.
        let (da0, da1, db0, db1) = unsafe {
            let values = input.values() as *const u8;
            (
                crate::prelude::slice_from_ptr(
                    values.add((a0 as usize).wrapping_mul(stride)),
                    stride,
                ),
                crate::prelude::slice_from_ptr(
                    values.add((a1 as usize).wrapping_mul(stride)),
                    stride,
                ),
                crate::prelude::slice_from_ptr(
                    values.add((b0 as usize).wrapping_mul(stride)),
                    stride,
                ),
                crate::prelude::slice_from_ptr(
                    values.add((b1 as usize).wrapping_mul(stride)),
                    stride,
                ),
            )
        };
        if memcmp(da0, db0) == 0 && memcmp(da1, db1) == 0 {
            return false;
        }
        return true;
    }

    false
}

// ufbx.c:29036-29042 `ufbxi_edge_crease`
#[cfg(feature = "subdivision")]
pub(crate) fn edge_crease(
    mesh: &MeshView,
    split: bool,
    topo: Run<'_, TopoEdge, Const>,
    index: u32,
) -> Real {
    let topo_edge = topo.at(index as usize);
    if topo_edge.twin() == NO_INDEX {
        return 1.0;
    }
    if split {
        return 1.0;
    }
    if !mesh.edge_crease_view().data().is_null() {
        let edge = topo_edge.edge();
        if edge != NO_INDEX {
            return mesh.edge_crease_view().copy_at(edge as usize) * (10.0 as Real);
        }
    }
    0.0
}

// ufbx.c:29044-29462 `ufbxi_subdivide_layer`
// Safe `fn`: the layer input/output arrive as views and checked runs carry the
// topology, corner-index and initialized face-value reads. Residual raw ops
// address owned arena pushes, dynamically grown scratch inputs, uncounted value
// buffers and callbacks — each vouched at its own block.
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) fn subdivide_layer(
    sc: &SubdivideContext,
    output: &View<SubdivideLayerOutput>,
    input: &View<SubdivideLayerInput, Const>,
) -> Result<(), crate::native::error::Fail> {
    let boundary: SubdivisionBoundary = input.boundary();

    let mesh: &MeshView = sc.src_mesh_view();
    let topo: *const TopoEdge = sc.topo();
    let num_topo: usize = sc.num_topo();
    // SAFETY: `sc` owns an initialized `num_topo`-element topology run that is
    // read-only and stable throughout layer subdivision.
    let topo_view = unsafe { Run::<TopoEdge, Const>::from_const_raw_parts(topo, num_topo) };
    // SAFETY: `input.indices` is an initialized `mesh.num_indices()`
    // source-corner run, stable and frozen throughout layer subdivision.
    let input_indices =
        unsafe { Run::<u32, Const>::from_const_raw_parts(input.indices(), mesh.num_indices()) };

    let edge_indices: *mut u32 = sc.result_view().push::<u32>(mesh.num_indices());
    ufbxi_check_err!(sc.error_view(), !edge_indices.is_null(), "edge_indices");

    let mut num_edge_values: usize = 0;
    {
        // SAFETY: the checked result-arena push owns `num_indices` stable,
        // write-capable slots. This phase progressively initializes them.
        let edge_indices_write =
            unsafe { Run::<u32>::from_raw_parts(edge_indices, mesh.num_indices()) };
        // C: `for (uint32_t ix = 0; ix < (uint32_t)mesh->num_indices; ix++)` — the
        // bound is truncated to `uint32_t` here (unlike the edge-point loop below).
        let mut ix: u32 = 0;
        while ix < mesh.num_indices() as u32 {
            let twin = topo_view.at(ix as usize).twin();
            // SAFETY: every stored index in this topology neighborhood addresses a
            // live `stride`-byte value in the input buffer.
            if twin < ix && !unsafe { is_edge_split(input, input_indices, topo_view, ix) } {
                // SAFETY: `twin < ix`, so the sequential loop initialized this
                // checked slot before the current write.
                let twin_index = unsafe { *edge_indices_write.subrun(twin as usize, 1).as_ptr() };
                edge_indices_write.write_at(ix as usize, twin_index);
            } else {
                edge_indices_write.write_at(ix as usize, num_edge_values as u32);
                num_edge_values += 1;
            }
            ix += 1;
        }
    }
    // C initializes exactly the u32-truncated prefix in the loop above. No
    // later stage writes edge indices, so that prefix is frozen for checked
    // reads. Checked reads stop at the initialized-prefix boundary.
    let num_edge_indices = (mesh.num_indices() as u32) as usize;
    // SAFETY: the sequential loop initialized this full prefix, which stays
    // stable and frozen for the rest of layer subdivision.
    let edge_indices_read =
        unsafe { Run::<u32, Const>::from_const_raw_parts(edge_indices, num_edge_indices) };

    let stride: usize = input.stride();
    let num_initial_values: usize = num_edge_values
        .wrapping_add(mesh.num_faces())
        .wrapping_add(mesh.num_indices());
    let values: *mut u8 = push_size(sc.tmp_view(), stride, num_initial_values) as *mut u8;
    ufbxi_check_err!(sc.error_view(), !values.is_null(), "values");

    let num_value_bytes = num_initial_values.wrapping_mul(stride);
    let num_face_value_bytes = mesh.num_faces().wrapping_mul(stride);
    let num_edge_value_bytes = num_edge_values.wrapping_mul(stride);
    let num_vertex_value_bytes = mesh.num_indices().wrapping_mul(stride);
    assert!(!does_overflow(
        num_face_value_bytes,
        mesh.num_faces(),
        stride,
    ));
    assert!(!does_overflow(
        num_edge_value_bytes,
        num_edge_values,
        stride,
    ));
    assert!(!does_overflow(
        num_vertex_value_bytes,
        mesh.num_indices(),
        stride,
    ));
    // SAFETY: the checked tmp-arena push owns exactly
    // `num_initial_values*stride` stable, write-capable bytes. `push_size`
    // rejects multiplication overflow and supplies the alignment selected for
    // `stride`; registered sum callbacks use an output type of this size whose
    // alignment divides that stride.
    let value_bytes = unsafe { Run::<u8>::from_raw_parts(values, num_value_bytes) };
    let face_values_write = value_bytes.subrun(0, num_face_value_bytes);
    let remaining_value_bytes = value_bytes.subrun(
        num_face_value_bytes,
        value_bytes.len() - num_face_value_bytes,
    );
    let edge_values_write = remaining_value_bytes.subrun(0, num_edge_value_bytes);
    let remaining_value_bytes = remaining_value_bytes.subrun(
        num_edge_value_bytes,
        remaining_value_bytes.len() - num_edge_value_bytes,
    );
    assert!(remaining_value_bytes.len() == num_vertex_value_bytes);
    let vertex_values_write = remaining_value_bytes.subrun(0, num_vertex_value_bytes);

    let mut num_vertex_values: usize = 0;

    let vertex_indices: *mut u32 = sc.result_view().push::<u32>(mesh.num_indices());
    ufbxi_check_err!(sc.error_view(), !vertex_indices.is_null(), "vertex_indices");
    // SAFETY: the checked result-arena push owns `num_indices` stable,
    // write-capable slots. The sentinel pass initializes all of them before
    // later guarded claims.
    let vertex_indices_write =
        unsafe { Run::<u32>::from_raw_parts(vertex_indices, mesh.num_indices()) };

    let min_inputs: usize = max_sz(32, mesh.max_face_triangles().wrapping_add(2));
    ufbxi_check_err!(
        sc.error_view(),
        // SAFETY: the three `*_mut_ptr` accessors return `sc`'s own live
        // allocator/inputs/cap fields, the array-growth contract `grow_array`
        // requires.
        unsafe {
            grow_array::<SubdivideInput>(
                sc.ator_tmp_view(),
                sc.inputs_mut_ptr(),
                sc.inputs_cap_mut_ptr(),
                min_inputs,
            )
        },
        "ufbxi_grow_array_size((&sc->ator_tmp), sizeof(**(&sc->inputs)), (&sc->inputs), (&sc->inputs_cap), (min_inputs))"
    );
    let mut inputs: *mut SubdivideInput = sc.inputs();
    // SAFETY: the successful grow owns a stable, write-capable prefix of
    // `min_inputs` records until the first later grow in vertex processing.
    // `min_inputs >= max_face_triangles+2` covers every face and
    // `min_inputs >= 32` covers every edge callback.
    let inputs_write = unsafe { Run::<SubdivideInput>::from_raw_parts(inputs, min_inputs) };

    // Assume initially unique per vertex, remove if not the case
    output.set_unique_per_vertex(true);

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
    // SAFETY: the comment above vouches slot 0 (the `None` slot) is never
    // selected here, so `sum_fn` is `Some`, making `unwrap_unchecked` sound.
    let sum_fn: SubdivideSumFn = unsafe { input.sum_fn().unwrap_unchecked() };
    let sum_user: *mut c_void = input.sum_user();

    // Mark unused indices as `UFBX_NO_INDEX` so we can patch non-manifold
    // C: `ufbxi_nounroll for (size_t i = 0; i < mesh->num_indices; i++)`
    let mut i: usize = 0;
    while i < mesh.num_indices() {
        vertex_indices_write.write_at(i, NO_INDEX);
        i += 1;
    }

    // Face points
    let mut fi: usize = 0;
    while fi < mesh.num_faces() {
        let face = mesh.faces_view().copy_at(fi);
        let dst = face_values_write
            .subrun(fi.wrapping_mul(stride), stride)
            .as_mut_ptr();

        let weight: Real = 1.0 / (face.num_indices as Real);
        let mut ci: u32 = 0;
        while ci < face.num_indices {
            let ix: u32 = face.index_begin.wrapping_add(ci);
            // SAFETY: the checked stored index addresses a live input value
            // whose `stride`-sized entry the byte offset addresses.
            let data = unsafe {
                (input.values() as *const u8)
                    .add((input_indices.copy_at(ix as usize) as usize).wrapping_mul(stride))
                    as *const c_void
            };
            let input_slot = inputs_write.at(ci as usize);
            input_slot.set_data(data);
            input_slot.set_weight(weight);
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
                    inputs_write.subrun(0, face.num_indices as usize).as_ptr(),
                    face.num_indices as usize,
                )
            } != 0,
            "sum_fn(sum_user, dst, inputs, face.num_indices)"
        );
        fi += 1;
    }
    // SAFETY: the face loop above initialized the full `num_faces*stride` byte
    // segment; later subdivision stages only read these stable tmp-arena bytes.
    let face_value_bytes = unsafe {
        Run::<u8, Const>::from_const_raw_parts(face_values_write.as_ptr(), num_face_value_bytes)
    };

    // Edge points
    // C: `for (uint32_t ix = 0; ix < mesh->num_indices; ix++)` — `ix` is
    // promoted to `size_t` for the comparison here (no truncation).
    let mut ix: u32 = 0;
    while (ix as usize) < mesh.num_indices() {
        let edge_index = edge_indices_read.copy_at(ix as usize);
        let dst = edge_values_write
            .subrun((edge_index as usize).wrapping_mul(stride), stride)
            .as_mut_ptr();

        let topo_edge = topo_view.at(ix as usize);
        let twin = topo_edge.twin();
        // SAFETY: every stored index in this topology neighborhood addresses a
        // live `stride`-byte value in the input buffer.
        let split: bool = unsafe { is_edge_split(input, input_indices, topo_view, ix) };

        if split || topo_edge.flags().has_any(TopoFlags::NON_MANIFOLD) {
            output.set_unique_per_vertex(false);
        }

        let mut crease: Real = 0.0;
        if split || twin == NO_INDEX {
            crease = 1.0;
        } else if topo_edge.edge() != NO_INDEX && !mesh.edge_crease().data.is_null() {
            crease = mesh.edge_crease_view().copy_at(topo_edge.edge() as usize) * (10.0 as Real);
        }
        if sharp_all {
            crease = 1.0;
        }

        // SAFETY: `input`'s buffers are live; `indices[ix]` and
        // `indices[topo[ix].next]` are in-range indices into the value array
        // whose `stride`-sized entries the byte offsets into `values` address.
        let (v0, v1) = unsafe {
            (
                (input.values() as *const u8)
                    .add((input_indices.copy_at(ix as usize) as usize).wrapping_mul(stride)),
                (input.values() as *const u8).add(
                    (input_indices.copy_at(topo_edge.next() as usize) as usize)
                        .wrapping_mul(stride),
                ),
            )
        };

        // TODO: Unify
        if twin < ix && !split {
            // Already calculated
        } else if crease <= 0.0 {
            let f0 = face_value_bytes
                .subrun((topo_edge.face() as usize).wrapping_mul(stride), stride)
                .as_ptr();
            let f1 = face_value_bytes
                .subrun(
                    (topo_view.at(twin as usize).face() as usize).wrapping_mul(stride),
                    stride,
                )
                .as_ptr();
            let input_slot = inputs_write.at(0);
            input_slot.set_data(v0 as *const c_void);
            input_slot.set_weight(0.25);
            let input_slot = inputs_write.at(1);
            input_slot.set_data(v1 as *const c_void);
            input_slot.set_weight(0.25);
            let input_slot = inputs_write.at(2);
            input_slot.set_data(f0 as *const c_void);
            input_slot.set_weight(0.25);
            let input_slot = inputs_write.at(3);
            input_slot.set_data(f1 as *const c_void);
            input_slot.set_weight(0.25);
            ufbxi_check_err!(
                sc.error_view(),
                // SAFETY: `sum_fn`/`sum_user`/`dst` and the 4 inputs satisfy the
                // summer callback contract.
                unsafe {
                    sum_fn(
                        sum_user,
                        dst as *mut c_void,
                        inputs_write.subrun(0, 4).as_ptr(),
                        4,
                    )
                } != 0,
                "sum_fn(sum_user, dst, inputs, 4)"
            );
        } else if crease >= 1.0 {
            let input_slot = inputs_write.at(0);
            input_slot.set_data(v0 as *const c_void);
            input_slot.set_weight(0.5);
            let input_slot = inputs_write.at(1);
            input_slot.set_data(v1 as *const c_void);
            input_slot.set_weight(0.5);
            ufbxi_check_err!(
                sc.error_view(),
                // SAFETY: `sum_fn`/`sum_user`/`dst` and the 2 inputs satisfy the
                // summer callback contract.
                unsafe {
                    sum_fn(
                        sum_user,
                        dst as *mut c_void,
                        inputs_write.subrun(0, 2).as_ptr(),
                        2,
                    )
                } != 0,
                "sum_fn(sum_user, dst, inputs, 2)"
            );
        } else if crease < 1.0 {
            let f0 = face_value_bytes
                .subrun((topo_edge.face() as usize).wrapping_mul(stride), stride)
                .as_ptr();
            let f1 = face_value_bytes
                .subrun(
                    (topo_view.at(twin as usize).face() as usize).wrapping_mul(stride),
                    stride,
                )
                .as_ptr();
            let w0: Real = 0.25 + 0.25 * crease;
            let w1: Real = 0.25 - 0.25 * crease;

            let input_slot = inputs_write.at(0);
            input_slot.set_data(v0 as *const c_void);
            input_slot.set_weight(w0);
            let input_slot = inputs_write.at(1);
            input_slot.set_data(v1 as *const c_void);
            input_slot.set_weight(w0);
            let input_slot = inputs_write.at(2);
            input_slot.set_data(f0 as *const c_void);
            input_slot.set_weight(w1);
            let input_slot = inputs_write.at(3);
            input_slot.set_data(f1 as *const c_void);
            input_slot.set_weight(w1);
            ufbxi_check_err!(
                sc.error_view(),
                // SAFETY: `sum_fn`/`sum_user`/`dst` and the 4 inputs satisfy the
                // summer callback contract.
                unsafe {
                    sum_fn(
                        sum_user,
                        dst as *mut c_void,
                        inputs_write.subrun(0, 4).as_ptr(),
                        4,
                    )
                } != 0,
                "sum_fn(sum_user, dst, inputs, 4)"
            );
        }
        ix = ix.wrapping_add(1);
    }

    // The bounded face/edge input epoch ends before vertex processing. Vertex
    // grows may replace the allocation, and one upstream path deliberately
    // retains the pre-grow local pointer, so the raw `inputs` carrier below is
    // kept separate from `inputs_write`.

    // Vertex points
    let mut vi: usize = 0;
    while vi < mesh.num_vertices() {
        let mut original_start = mesh.vertex_first_index_view().copy_at(vi);
        if original_start == NO_INDEX {
            vi += 1;
            continue;
        }

        // Find a topological boundary, or if not found a split edge
        let mut start: u32 = original_start;
        // C: `for (uint32_t cur = start;;)`
        let mut cur: u32 = start;
        loop {
            let prev = catch_topo_prev_vertex_edge_run(None, topo_view, cur);
            if prev == NO_INDEX {
                start = cur;
                break;
            } // Topological boundary: Stop and use as start
              // SAFETY: every stored index in this topology neighborhood
              // addresses a live `stride`-byte value in the input buffer.
            if unsafe { is_edge_split(input, input_indices, topo_view, prev) } {
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
                output.set_unique_per_vertex(false);
            }

            let value_index: u32 = num_vertex_values as u32;
            num_vertex_values += 1;
            let vertex_value_begin = (value_index as usize).wrapping_mul(stride);

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
            let start_topo = topo_view.at(start as usize);
            let start_prev = start_topo.prev();
            let start_prev_topo = topo_view.at(start_prev as usize);
            let end_edge = start_prev_topo.twin();
            let mut valence: usize = 2;

            non_manifold |= start_topo.flags().has_any(TopoFlags::NON_MANIFOLD);
            non_manifold |= start_prev_topo.flags().has_any(TopoFlags::NON_MANIFOLD);

            // SAFETY: `input`'s buffers are live; `indices[start]` is an
            // in-range index into the value array whose `stride`-sized entry the
            // byte offset addresses.
            let v0: *const u8 = unsafe {
                (input.values() as *const u8)
                    .add((input_indices.copy_at(start as usize) as usize).wrapping_mul(stride))
            };

            let mut num_inputs: usize = 4;

            {
                // SAFETY: `input`'s buffers and `topo` are live; the `.next`
                // sibling corner's and `start_prev`'s vertices are in-range,
                // addressing their `stride`-sized attributes in `values`.
                let (e0, e1) = unsafe {
                    (
                        (input.values() as *const u8).add(
                            (input_indices.copy_at(start_topo.next() as usize) as usize)
                                .wrapping_mul(stride),
                        ),
                        (input.values() as *const u8).add(
                            (input_indices.copy_at(start_prev as usize) as usize)
                                .wrapping_mul(stride),
                        ),
                    )
                };
                let f0 = face_value_bytes
                    .subrun((start_topo.face() as usize).wrapping_mul(stride), stride)
                    .as_ptr();
                // SAFETY: `inputs` holds at least 4 live slots; these are the
                // first four sum-fn inputs.
                unsafe {
                    (*inputs.add(0)).data = v0 as *const c_void;
                    (*inputs.add(1)).data = e0 as *const c_void;
                    (*inputs.add(2)).data = e1 as *const c_void;
                    (*inputs.add(3)).data = f0 as *const c_void;
                }
            }

            // SAFETY: every stored index reached from `start` and, when taken,
            // the non-sentinel `end_edge` addresses a live `stride`-byte input
            // value.
            let (start_split, prev_split) = unsafe {
                (
                    is_edge_split(input, input_indices, topo_view, start),
                    end_edge != NO_INDEX
                        && is_edge_split(input, input_indices, topo_view, end_edge),
                )
            };

            // Either of the first two edges may be creased
            let start_crease = edge_crease(mesh, start_split, topo_view, start);
            if start_crease > 0.0 {
                total_crease += start_crease;
                crease_input_indices[num_crease] = 1;
                num_crease += 1;
            }
            let prev_crease = edge_crease(mesh, prev_split, topo_view, start_prev);
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
                // SAFETY: `start` is checked into the destination run, and the
                // sentinel pass initialized every slot before guarded claims.
                unsafe { *vertex_indices_write.subrun(start as usize, 1).as_ptr() } == NO_INDEX,
                "vertex_indices[start] == UFBX_NO_INDEX"
            );
            vertex_indices_write.write_at(start as usize, value_index);

            if start_split {
                // We need to special case if the first edge is split as we have
                // handled it already in the code above..
                start = catch_topo_next_vertex_edge_run(None, topo_view, start);
                num_split += 1;
            } else {
                // Follow vertex edges until we either hit a topological/split boundary
                // or loop back to the left edge we accounted for in `start_prev`
                let mut cur: u32 = start;
                loop {
                    cur = catch_topo_next_vertex_edge_run(None, topo_view, cur);

                    // Topological boundary: Finished
                    if cur == NO_INDEX {
                        on_boundary = true;
                        start = NO_INDEX;
                        break;
                    }
                    let cur_topo = topo_view.at(cur as usize);

                    non_manifold |= cur_topo.flags().has_any(TopoFlags::NON_MANIFOLD);
                    ufbxi_check_err!(
                        sc.error_view(),
                        // SAFETY: `cur` is checked into the destination run, and
                        // the sentinel pass initialized every slot before claims.
                        unsafe { *vertex_indices_write.subrun(cur as usize, 1).as_ptr() }
                            == NO_INDEX,
                        "vertex_indices[cur] == UFBX_NO_INDEX"
                    );
                    vertex_indices_write.write_at(cur as usize, value_index);

                    // SAFETY: every stored index in this topology neighborhood
                    // addresses a live `stride`-byte value in the input buffer.
                    let split: bool =
                        unsafe { is_edge_split(input, input_indices, topo_view, cur) };

                    // Looped: Add the face from the other side still if not split
                    if cur == end_edge && !split {
                        ufbxi_check_err!(
                            sc.error_view(),
                            // SAFETY: the `*_mut_ptr` accessors return `sc`'s own
                            // live allocator/inputs/cap fields per the grow contract.
                            unsafe {
                                grow_array::<SubdivideInput>(
                                    sc.ator_tmp_view(),
                                    sc.inputs_mut_ptr(),
                                    sc.inputs_cap_mut_ptr(),
                                    num_inputs.wrapping_add(1),
                                )
                            },
                            "ufbxi_grow_array_size((&sc->ator_tmp), sizeof(**(&sc->inputs)), (&sc->inputs), (&sc->inputs_cap), (num_inputs + 1))"
                        );
                        let f0 = face_value_bytes
                            .subrun((cur_topo.face() as usize).wrapping_mul(stride), stride)
                            .as_ptr();
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
                    let cur_crease = edge_crease(mesh, split, topo_view, cur);
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
                                    sc.ator_tmp_view(),
                                    sc.inputs_mut_ptr(),
                                    sc.inputs_cap_mut_ptr(),
                                    num_inputs.wrapping_add(2),
                                )
                            },
                            "ufbxi_grow_array_size((&sc->ator_tmp), sizeof(**(&sc->inputs)), (&sc->inputs), (&sc->inputs_cap), (num_inputs + 2))"
                        );
                        inputs = sc.inputs();

                        // SAFETY: `input`'s buffers and `topo` are live; the
                        // index `indices` holds for `cur`'s `.next` sibling
                        // corner is in range for the value array, addressing its
                        // `stride`-sized entry in `values`.
                        let e0: *const u8 = unsafe {
                            (input.values() as *const u8).add(
                                (input_indices.copy_at(cur_topo.next() as usize) as usize)
                                    .wrapping_mul(stride),
                            )
                        };
                        let f0 = face_value_bytes
                            .subrun((cur_topo.face() as usize).wrapping_mul(stride), stride)
                            .as_ptr();
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
                        start = catch_topo_next_vertex_edge_run(None, topo_view, cur);
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
                // A successful corner claim makes this dense destination slot
                // part of the initialized vertex prefix. The registered
                // callback writes its complete `stride`-byte output value.
                // SAFETY: `sum_fn`/`sum_user`, the checked destination slot,
                // and the first `num_inputs` entries were set up above per the
                // summer contract.
                unsafe {
                    sum_fn(
                        sum_user,
                        vertex_values_write
                            .subrun(vertex_value_begin, stride)
                            .as_mut_ptr() as *mut c_void,
                        inputs,
                        num_inputs,
                    )
                } != 0,
                "sum_fn(sum_user, dst, inputs, num_inputs)"
            );
        }
        vi += 1;
    }

    // Copy non-manifold vertex values as-is
    let mut old_ix: usize = 0;
    while old_ix < mesh.num_indices() {
        // SAFETY: `old_ix` is checked into the run and the sentinel pass
        // initialized every slot before any claims or this sweep.
        let mut ix: u32 = unsafe { *vertex_indices_write.subrun(old_ix, 1).as_ptr() };
        if ix == NO_INDEX {
            ix = num_vertex_values as u32;
            num_vertex_values += 1;
            vertex_indices_write.write_at(old_ix, ix);
            // SAFETY: `input`'s buffers are live; `indices[old_ix]` is an
            // in-range index into the value array whose `stride`-sized entry the
            // byte offset into `values` addresses.
            let src: *const u8 = unsafe {
                (input.values() as *const u8)
                    .add((input_indices.copy_at(old_ix) as usize).wrapping_mul(stride))
            };
            let dst = vertex_values_write
                .subrun((ix as usize).wrapping_mul(stride), stride)
                .as_mut_ptr();

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
    let new_values: *mut u8 =
        push_size(sc.result_view(), stride, num_values.wrapping_add(1)) as *mut u8;
    ufbxi_check_err!(sc.error_view(), !new_values.is_null(), "new_values");

    let num_new_value_bytes = num_values.wrapping_add(1).wrapping_mul(stride);
    // SAFETY: the checked result-arena push owns exactly
    // `(num_values+1)*stride` stable, write-capable bytes; `push_size` rejects
    // multiplication overflow.
    let new_value_bytes = unsafe { Run::<u8>::from_raw_parts(new_values, num_new_value_bytes) };
    // SAFETY: the checked leading subrun contains the complete sentinel value.
    unsafe { core::ptr::write_bytes(new_value_bytes.subrun(0, stride).as_mut_ptr(), 0, stride) };

    let num_copied_value_bytes = num_values.wrapping_mul(stride);
    let initialized_value_bytes = value_bytes.subrun(0, num_copied_value_bytes);
    // SAFETY: successful callbacks initialize every byte of their output slot.
    // Face slots are sequential; first-seen edges assign dense indices and
    // initialize them before a twin can reuse one; vertex slots form the dense
    // `0..num_vertex_values` prefix. No scratch-value writes occur after this
    // point.
    let initialized_value_bytes = unsafe {
        Run::<u8, Const>::from_const_raw_parts(
            initialized_value_bytes.as_ptr(),
            num_copied_value_bytes,
        )
    };
    let new_values = new_value_bytes
        .subrun(stride, num_copied_value_bytes)
        .as_mut_ptr();

    // SAFETY: the checked source and destination runs have equal length and
    // belong to the distinct tmp and result arenas, so they do not overlap.
    unsafe {
        core::ptr::copy_nonoverlapping(
            initialized_value_bytes.as_ptr(),
            new_values,
            num_copied_value_bytes,
        )
    };

    output.set_values(new_values as *mut c_void);
    output.set_num_values(num_values);

    if !input.ignore_indices() {
        let num_new_indices = mesh.num_indices().wrapping_mul(4);
        let new_indices: *mut u32 = sc.result_view().push::<u32>(num_new_indices);
        ufbxi_check_err!(sc.error_view(), !new_indices.is_null(), "new_indices");
        // SAFETY: the non-manifold sweep completed the initialized, stable
        // vertex-index run and no later stage writes it. The checked push owns
        // `num_new_indices` stable, write-capable destination slots.
        let (vertex_indices_read, new_indices_write) = unsafe {
            (
                Run::<u32, Const>::from_const_raw_parts(vertex_indices, mesh.num_indices()),
                Run::<u32>::from_raw_parts(new_indices, num_new_indices),
            )
        };

        let face_start: u32 = 0;
        let edge_start: u32 = face_start.wrapping_add(mesh.num_faces() as u32);
        let vert_start: u32 = edge_start.wrapping_add(num_edge_values as u32);
        let mut ix: usize = 0;
        while ix < mesh.num_indices() {
            let topo_edge = topo_view.at(ix);
            let quad = ix.wrapping_mul(4);
            new_indices_write.write_at(
                quad,
                vert_start.wrapping_add(vertex_indices_read.copy_at(ix)),
            );
            new_indices_write.write_at(
                quad.wrapping_add(1),
                edge_start.wrapping_add(edge_indices_read.copy_at(ix)),
            );
            new_indices_write.write_at(
                quad.wrapping_add(2),
                face_start.wrapping_add(topo_edge.face()),
            );
            // C reads `topo[ix].prev` for the fourth assignment, after the
            // first three destination stores.
            let prev = topo_edge.prev() as usize;
            assert!(prev < mesh.num_indices());
            new_indices_write.write_at(
                quad.wrapping_add(3),
                edge_start.wrapping_add(edge_indices_read.copy_at(prev)),
            );
            ix += 1;
        }
        output.set_indices(new_indices);
        output.set_num_indices(num_new_indices);
    } else {
        output.set_indices(core::ptr::null_mut());
        output.set_num_indices(0);
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
// Safe `fn`: the attribute arrives as a view; the residual raw ops initialize
// and mint views over the two `MaybeUninit` locals this fn owns. Completed
// output fields and attribute list headers use checked view accessors.
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
    // SAFETY: `output`/`input` address the stack locals `output_mem`/`input_mem`,
    // exclusively owned by this fn — write-capable provenance for the `Mut` mint,
    // readable for the `Const` one — and the fields of `input` were fully
    // initialized above; nothing writes through `input` while the frozen `Const`
    // view is live.
    let (output_view, input_view) = unsafe {
        (
            View::<SubdivideLayerOutput>::from_ptr(output),
            View::<SubdivideLayerInput, Const>::from_ptr(input),
        )
    };
    subdivide_layer(sc, output_view, input_view)?;

    attrib.values_view().set_data(output_view.values());
    attrib.indices_view().set_data(output_view.indices());
    attrib.values_view().set_count(output_view.num_values());
    attrib.indices_view().set_count(output_view.num_indices());

    Ok(())
}

// ufbx.c:29491-29503 `ufbxi_subdivision_copy_weights`
// Safe `fn`: source ranges and weights arrive as viewed lists, every stored
// span is bounds-checked, and the sole raw mint covers the fresh destination
// run initialized below.
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) fn subdivision_copy_weights<RM: Mode, WM: Mode>(
    sc: &SubdivideContext,
    ranges: &View<List<SubdivisionWeightRange>, RM>,
    weights: &View<List<SubdivisionWeight>, WM>,
) -> *mut SubdivisionVertexWeights {
    let dst: *mut SubdivisionVertexWeights = sc
        .tmp_view()
        .push::<SubdivisionVertexWeights>(ranges.count());
    ufbxi_check_return_err!(
        sc.error_view(),
        !dst.is_null(),
        core::ptr::null_mut(),
        "dst"
    );
    // SAFETY: `dst` is the fresh non-null `ranges.count()`-element tmp-arena
    // push checked above; its slots are write-capable and stable for this call.
    let dst_run = unsafe { Run::<SubdivisionVertexWeights>::from_raw_parts(dst, ranges.count()) };
    let weights_run = Run::from_list(weights);

    // C: `ufbxi_nounroll for (size_t i = 0; i != ranges.count; i++)`
    let mut i: usize = 0;
    while i != ranges.count() {
        let range = ranges.at(i);
        let weight_span =
            weights_run.subrun(range.weight_begin() as usize, range.num_weights() as usize);
        let out = dst_run.at(i);
        // The C-shaped carrier stores a mutable-typed pointer, but this borrowed
        // source span remains frozen and every downstream consumer reads it.
        out.set_weights(weight_span.as_ptr().cast_mut());
        out.set_num_weights(weight_span.len());
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
    // SAFETY: both checked tmp-arena pushes own `num_vertices` stable,
    // write-capable slots. This loop initializes every destination record and
    // weight entry in C field-assignment order.
    let (dst_run, weights_run) = unsafe {
        (
            Run::<SubdivisionVertexWeights>::from_raw_parts(dst, num_vertices),
            Run::<SubdivisionWeight>::from_raw_parts(weights, num_vertices),
        )
    };

    // C: `ufbxi_nounroll for (size_t i = 0; i != num_vertices; i++)`
    let mut i: usize = 0;
    while i != num_vertices {
        let out = dst_run.at(i);
        out.set_weights(weights_run.subrun(i, 1).as_mut_ptr());
        out.set_num_weights(1);
        let weight = weights_run.at(i);
        weight.set_index(i as u32);
        weight.set_weight(1.0);
        i += 1;
    }

    dst
}

// ufbx.c:29521-29546 `ufbxi_init_skin_weights`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) fn init_skin_weights<M: Mode>(
    sc: &SubdivideContext,
    num_vertices: usize,
    skin: &View<SkinDeformer, M>,
) -> *mut SubdivisionVertexWeights {
    let dst: *mut SubdivisionVertexWeights =
        sc.tmp_view().push::<SubdivisionVertexWeights>(num_vertices);
    ufbxi_check_return_err!(
        sc.error_view(),
        !dst.is_null(),
        core::ptr::null_mut(),
        "dst"
    );
    // SAFETY: the checked tmp-arena push owns `num_vertices` stable,
    // write-capable destination slots. Each loop iteration initializes one.
    let dst_run = unsafe { Run::<SubdivisionVertexWeights>::from_raw_parts(dst, num_vertices) };

    let vertices = skin.vertices_view();
    let source_weights = Run::from_list(skin.weights_view());
    assert!(num_vertices <= vertices.count());

    let mut i: usize = 0;
    while i < num_vertices {
        ufbxi_dev_assert!(i < skin.vertices().count);
        let vertex = vertices.copy_at(i);
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
        // SAFETY: the checked per-vertex tmp-arena push owns `num_weights`
        // stable, write-capable entries initialized by the inner loop.
        let weights_run = unsafe { Run::<SubdivisionWeight>::from_raw_parts(weights, num_weights) };

        let weight_begin = vertex.weight_begin as usize;
        let source_span = source_weights.subrun(weight_begin, num_weights);

        let out = dst_run.at(i);
        out.set_weights(weights_run.as_mut_ptr());
        out.set_num_weights(num_weights);
        // C: `ufbxi_nounroll for (size_t wi = 0; wi != num_weights; wi++)`
        let mut wi: usize = 0;
        while wi != num_weights {
            let skin_weight = source_span.at(wi);
            let cluster_index = skin_weight.cluster_index();
            ufbxi_check_return_err!(
                sc.error_view(),
                cluster_index <= i32::MAX as u32,
                core::ptr::null_mut(),
                "skin_weights[wi].cluster_index <= INT32_MAX"
            );
            let weight = weights_run.at(wi);
            weight.set_index(skin_weight.cluster_index());
            weight.set_weight(skin_weight.weight());
            wi += 1;
        }
        i += 1;
    }

    dst
}

// ufbx.c:29548-29594 `ufbxi_subdivide_weights`
#[cfg(feature = "subdivision")]
#[inline(never)]
/// # Safety
/// `src` must address at least `sc.src_mesh.num_vertices` contiguous initialized
/// `SubdivisionVertexWeights` records that stay alive, unmoved and frozen
/// throughout layer subdivision. `sc.src_mesh.vertex_indices` indexes that run
/// at `size_of::<SubdivisionVertexWeights>()` stride. Each record's `weights`
/// must likewise address `num_weights` contiguous initialized entries that stay
/// alive, unmoved and frozen while the weight callback reads them. These nested
/// relational promises are not carried by the parameter types.
pub(crate) unsafe fn subdivide_weights(
    sc: &SubdivideContext,
    ranges: &ListView<SubdivisionWeightRange>,
    weights: &ListView<SubdivisionWeight>,
    src: *const SubdivisionVertexWeights,
) -> Result<(), crate::native::error::Fail> {
    ufbxi_check_err!(sc.error_view(), !src.is_null(), "src");
    // SAFETY: the function contract supplies one initialized source record per
    // source vertex, stable and frozen throughout layer subdivision.
    let src_run = unsafe {
        Run::<SubdivisionVertexWeights, Const>::from_const_raw_parts(
            src,
            sc.src_mesh_view().num_vertices(),
        )
    };

    let mut input_mem = MaybeUninit::<SubdivideLayerInput>::uninit(); // ufbxi_uninit
    let input: *mut SubdivideLayerInput = input_mem.as_mut_ptr();
    // SAFETY: `input` addresses the local `input_mem`, so every field write is
    // in-bounds; the accessor RHS values are safe reads of `sc`'s own state.
    unsafe {
        (*input).sum_fn = Some(subdivide_sum_vertex_weights);
        (*input).sum_user = (sc as *const SubdivideContext) as *mut c_void;
        (*input).values = src_run.as_ptr().cast::<c_void>();
        (*input).indices = sc.src_mesh_view().vertex_indices_view().data();
        (*input).stride = size_of::<SubdivisionVertexWeights>();
        (*input).boundary = sc.opts_view().boundary();
        (*input).check_split_data = false;
        (*input).ignore_indices = true;
    }

    sc.set_total_weights(0);

    let mut output_mem = MaybeUninit::<SubdivideLayerOutput>::uninit(); // ufbxi_uninit
    let output: *mut SubdivideLayerOutput = output_mem.as_mut_ptr();
    // SAFETY: `output`/`input` address the stack locals `output_mem`/`input_mem`,
    // exclusively owned by this fn — write-capable provenance for the `Mut` mint,
    // readable for the `Const` one — and the fields of `input` were fully
    // initialized above; nothing writes through `input` while the frozen `Const`
    // view is live.
    let (output_view, input_view) = unsafe {
        (
            View::<SubdivideLayerOutput>::from_ptr(output),
            View::<SubdivideLayerInput, Const>::from_ptr(input),
        )
    };
    subdivide_layer(sc, output_view, input_view)?;

    let num_vertices: usize = output_view.num_values();
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
    // freshly pushed `dst_ranges && dst_weights` (ufbx.c:29573); both call
    // sites pass the address of a list field, so the condition holds for every
    // caller and the reference parameters carry it in the type.

    // SAFETY: after successful layer subdivision, `output.values` addresses the
    // initialized `num_vertices`-element `SubdivisionVertexWeights` result run.
    // That region stays stable and unwritten throughout this copy phase; the
    // destination pushes above occupy distinct result-arena regions.
    let src_weights = unsafe {
        Run::<SubdivisionVertexWeights, Const>::from_const_raw_parts(
            output_view.values().cast::<SubdivisionVertexWeights>(),
            num_vertices,
        )
    };

    let mut weight_offset: usize = 0;
    let mut vi: usize = 0;
    while vi < num_vertices {
        let ws = src_weights.copy_at(vi);
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

    ranges.set_data(dst_ranges);
    ranges.set_count(num_vertices);
    weights.set_data(dst_weights);
    weights.set_count(sc.total_weights());

    Ok(())
}

// ufbx.c:29596-29629 `ufbxi_subdivide_vertex_crease`
// Safe `fn`: both crease attributes arrive as views; source reads use checked
// list access and the fresh destination runs use bounds-checked writes.
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
    let dst_values = Run::from_list(dst.values_view());
    dst_values.write_at(src_values, 0.0);

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
        let mut crease = src.values_view().copy_at(i);
        // C: `0.999f` / `0.1f` are `float` literals widened to `ufbx_real`.
        if crease < 0.999f32 as Real {
            crease -= 0.1f32 as Real;
        }
        if crease < 0.0 {
            crease = 0.0;
        }
        dst_values.write_at(i, crease);
        i += 1;
    }

    // Write the crease at the vertex corner and zero (at `src_values`) on other ones
    let zero_index: u32 = src_values as u32;
    let dst_indices = Run::from_list(dst.indices_view());
    // C: `ufbxi_nounroll for (size_t i = 0; i < src_indices; i++)`
    let mut i: usize = 0;
    while i < src_indices {
        let quad = i.wrapping_mul(4);
        dst_indices.write_at(quad, src.indices_view().copy_at(i));
        dst_indices.write_at(quad.wrapping_add(1), zero_index);
        dst_indices.write_at(quad.wrapping_add(2), zero_index);
        dst_indices.write_at(quad.wrapping_add(3), zero_index);
        i += 1;
    }

    Ok(())
}

// Normal arrays are initialized mutable list runs. Read each value by copy
// before replacing the same slot, matching the element-at-a-time C walk.
#[cfg(feature = "subdivision")]
fn normalize_vec3_list(values: &ListView<Vec3>) {
    let dst = Run::from_list(values);
    let mut i: usize = 0;
    while i < dst.len() {
        dst.write_at(i, slow_normalize3(&values.copy_at(i)));
        i += 1;
    }
}

// ufbx.c:29631-29925 `ufbxi_subdivide_mesh_level`
// Stays `unsafe fn`: the mesh fields run through `MeshView`, but topology
// construction and subdivision-weight propagation consume raw source runs
// whose relational validity comes from the source mesh/subdivision data.
// Source face ranges must also tile `num_indices` so topology construction is
// valid and the optional replicated face-attribute runs are fully initialized;
// the narrow blocks below cite their local obligations.
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
    // SAFETY: `compute_topology` initialized the full `num_indices`-element
    // topology run above; tmp-arena storage is stable and the run is read-only
    // for the remainder of this subdivision level.
    let topo_view =
        unsafe { Run::<TopoEdge, Const>::from_const_raw_parts(topo, mesh.num_indices()) };

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

    // The set-list views serve the arena copies, checked walks and guarded
    // first-element reads below.
    let uv_sets = result.uv_sets_view();
    let color_sets = result.color_sets_view();

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
        for set in Run::from_list(uv_sets).iter() {
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
        for set in Run::from_list(color_sets).iter() {
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
        let set0 = uv_sets.at(0);
        unsafe {
            core::ptr::copy_nonoverlapping(set0.vertex_uv_ptr(), result.vertex_uv_raw(), 1);
            core::ptr::copy_nonoverlapping(
                set0.vertex_bitangent_ptr(),
                result.vertex_bitangent_raw(),
                1,
            );
            core::ptr::copy_nonoverlapping(
                set0.vertex_tangent_ptr(),
                result.vertex_tangent_raw(),
                1,
            );
        }
    }
    if color_sets.count() > 0 {
        // SAFETY: the count is > 0, so element 0 of the run is live; the
        // destination is a distinct live field of `result`, so the copy is
        // non-overlapping.
        let set0 = color_sets.at(0);
        unsafe {
            core::ptr::copy_nonoverlapping(set0.vertex_color_ptr(), result.vertex_color_raw(), 1);
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
        normalize_vec3_list(result.vertex_normal().values_view());
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
            normalize_vec3_list(result.skinned_normal().values_view());
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
    // SAFETY: `result_sub` is the fresh non-null result-arena push above, so the
    // view carries that buffer's write-capable provenance, and the result arena
    // outlives the destination mesh the ref is stored into (`to_ref` contract).
    result.set_subdivision_result(Some(unsafe {
        View::<SubdivisionResult>::from_ptr(result_sub).to_ref()
    }));

    if sc.opts_view().evaluate_source_vertices() || sc.opts_view().evaluate_skin_weights() {
        // The source mesh's previous-level result is read-only in this scope, so
        // it is viewed `Const` (the frozen tag holds: every write below targets
        // `result_sub`, a distinct fresh allocation).
        let mesh_sub_view: Option<&View<SubdivisionResult, Const>> =
            mesh.subdivision_result().map(|sub| sub.view::<Const>());
        // SAFETY: `result_sub` is the fresh non-null result-arena push above, so
        // the view carries that buffer's write-capable provenance; no other
        // reference to those bytes is formed while it is live.
        let result_sub_view: &View<SubdivisionResult> =
            unsafe { View::<SubdivisionResult>::from_ptr(result_sub) };

        let mut skin: Option<&View<SkinDeformer>> = None;
        if sc.opts_view().evaluate_skin_weights() {
            if mesh.skin_deformers().count > 0 {
                ufbxi_check_err!(
                    sc.error_view(),
                    sc.opts_view().skin_deformer_index() < mesh.skin_deformers().count,
                    "sc->opts.skin_deformer_index < mesh->skin_deformers.count"
                );
                skin = Some(
                    mesh.skin_deformers_view()
                        .at(sc.opts_view().skin_deformer_index()),
                );
            }
        }

        let mut max_weights: usize = 0;
        if sc.opts_view().evaluate_source_vertices() {
            max_weights = max_sz(max_weights, mesh.num_vertices());
        }
        if let Some(skin_view) = skin {
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
                weights = subdivision_copy_weights(
                    sc,
                    sub.source_vertex_ranges_view(),
                    sub.source_vertex_weights_view(),
                );
            } else {
                weights = init_source_vertex_weights(sc, mesh.num_vertices());
            }

            // SAFETY: `weights` has `mesh.num_vertices()` initialized records:
            // either the fresh initializer's exact count or the previous
            // subdivision result's per-level range-count invariant. The record
            // run and every nested weight span stay stable and frozen while
            // `subdivide_weights` reads them. The out-param views are safe
            // projections of the distinct fresh `result_sub_view`.
            unsafe {
                subdivide_weights(
                    sc,
                    result_sub_view.source_vertex_ranges_view(),
                    result_sub_view.source_vertex_weights_view(),
                    weights,
                )
            }?;
        }

        if let Some(skin_view) = skin {
            sc.set_max_vertex_weights(if sc.opts_view().max_skin_weights() != 0 {
                sc.opts_view().max_skin_weights()
            } else {
                usize::MAX
            });

            let weights: *mut SubdivisionVertexWeights;
            // C-parity: the guard reads `source_vertex_ranges` here too
            // (ufbx.c:29750), not `skin_cluster_ranges`.
            if let Some(sub) = mesh_sub_view.filter(|sub| sub.source_vertex_ranges().count > 0) {
                weights = subdivision_copy_weights(
                    sc,
                    sub.skin_cluster_ranges_view(),
                    sub.skin_cluster_weights_view(),
                );
            } else {
                weights = init_skin_weights(sc, mesh.num_vertices(), skin_view);
            }

            // SAFETY: `weights` has `mesh.num_vertices()` initialized records:
            // either the fresh initializer's exact count or the previous
            // subdivision result's per-level skin-range-count invariant. The
            // record run and every nested weight span stay stable and frozen
            // while `subdivide_weights` reads them. The out-param views are safe
            // projections of the distinct fresh `result_sub_view`.
            unsafe {
                subdivide_weights(
                    sc,
                    result_sub_view.skin_cluster_ranges_view(),
                    result_sub_view.skin_cluster_weights_view(),
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
    let vertices: &ListView<Vec3> = result.vertices_view();
    vertices.set_data(result.vertex_position().values().data);
    vertices.set_count(result.num_vertices());

    let faces: &ListView<Face> = result.faces_view();
    faces.set_count(result.num_faces());
    faces.set_data(sc.result_view().push::<Face>(result.num_faces()));
    ufbxi_check_err!(
        sc.error_view(),
        !faces.data().is_null(),
        "result->faces.data"
    );

    let mut i: usize = 0;
    while i < result.num_faces() {
        let face = faces.at(i);
        face.set_index_begin(i.wrapping_mul(4) as u32);
        face.set_num_indices(4);
        i += 1;
    }

    if !mesh.edges().data.is_null() {
        let edges: &ListView<Edge> = result.edges_view();
        let edge_crease: &ListView<Real> = result.edge_crease_view();
        let edge_smoothing: &ListView<bool> = result.edge_smoothing_view();
        let edge_visibility: &ListView<bool> = result.edge_visibility_view();

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
            let edge = mesh.edges_view().copy_at(i);
            let face_ix = topo_view.at(edge.a as usize).face();
            let face = mesh.faces_view().copy_at(face_ix as usize);
            let offset: u32 = edge.a.wrapping_sub(face.index_begin);
            let next: u32 = (offset.wrapping_add(1)) % face.num_indices;

            let a: u32 = (face.index_begin.wrapping_add(offset)).wrapping_mul(4);
            let b: u32 = (face.index_begin.wrapping_add(next)).wrapping_mul(4);

            let (e0, e1) = (edges.at(di + 0), edges.at(di + 1));
            e0.set_a(a);
            e0.set_b(a.wrapping_add(1));
            e1.set_a(b.wrapping_add(3));
            e1.set_b(b);

            if !mesh.edge_crease().data.is_null() {
                let mut crease = mesh.edge_crease_view().copy_at(i);
                // C: `0.999f` is a `float` literal; `(ufbx_real)0.1` is not.
                if crease < 0.999f32 as Real {
                    crease -= 0.1 as Real;
                }
                if crease < 0.0 {
                    crease = 0.0;
                }
                let run = Run::from_list(edge_crease);
                run.write_at(di + 0, crease);
                run.write_at(di + 1, crease);
            }

            if !mesh.edge_smoothing().data.is_null() {
                let smoothing = mesh.edge_smoothing_view().copy_at(i);
                let run = Run::from_list(edge_smoothing);
                run.write_at(di + 0, smoothing);
                run.write_at(di + 1, smoothing);
            }

            if !mesh.edge_visibility().data.is_null() {
                let visibility = mesh.edge_visibility_view().copy_at(i);
                let run = Run::from_list(edge_visibility);
                run.write_at(di + 0, visibility);
                run.write_at(di + 1, visibility);
            }

            di += 2;
            i += 1;
        }

        let mut fi: usize = 0;
        while fi < result.num_faces() {
            let e = edges.at(di);
            e.set_a(fi.wrapping_mul(4).wrapping_add(1) as u32);
            e.set_b(fi.wrapping_mul(4).wrapping_add(2) as u32);

            if !edge_crease.data().is_null() {
                Run::from_list(edge_crease).write_at(di, 0.0);
            }

            if !edge_smoothing.data().is_null() {
                Run::from_list(edge_smoothing).write_at(di + 0, true);
            }

            if !edge_visibility.data().is_null() {
                Run::from_list(edge_visibility).write_at(di + 0, false);
            }

            di += 1;
            fi += 1;
        }
    }

    let face_material: &ListView<u32> = result.face_material_view();
    let face_smoothing: &ListView<bool> = result.face_smoothing_view();
    let face_group: &ListView<u32> = result.face_group_view();
    let face_hole: &ListView<bool> = result.face_hole_view();

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
        let material_parts: &ListView<MeshPart> = result.material_parts_view();
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
        let face = mesh.faces_view().copy_at(i);

        let mut mat: u32 = 0;
        if !mesh.face_material().data.is_null() {
            mat = mesh.face_material_view().copy_at(i);
            let run = Run::from_list(face_material);
            let mut ci: usize = 0;
            while ci < face.num_indices as usize {
                run.write_at(index_offset.wrapping_add(ci), mat);
                ci += 1;
            }
        }
        // C: `mat` is otherwise unused (assigned and read only in the branch).
        let _ = mat;
        if !mesh.face_smoothing().data.is_null() {
            let flag = mesh.face_smoothing_view().copy_at(i);
            let run = Run::from_list(face_smoothing);
            let mut ci: usize = 0;
            while ci < face.num_indices as usize {
                run.write_at(index_offset.wrapping_add(ci), flag);
                ci += 1;
            }
        }
        if !mesh.face_group().data.is_null() {
            let group = mesh.face_group_view().copy_at(i);
            let run = Run::from_list(face_group);
            let mut ci: usize = 0;
            while ci < face.num_indices as usize {
                run.write_at(index_offset.wrapping_add(ci), group);
                ci += 1;
            }
        }
        if !mesh.face_hole().data.is_null() {
            let flag = mesh.face_hole_view().copy_at(i);
            let run = Run::from_list(face_hole);
            let mut ci: usize = 0;
            while ci < face.num_indices as usize {
                run.write_at(index_offset.wrapping_add(ci), flag);
                ci += 1;
            }
        }
        index_offset = index_offset.wrapping_add(face.num_indices as usize);
        i += 1;
    }

    // Will be filled in by `ufbxi_finalize_mesh()`.
    result.vertex_first_index_view().set_count(0);

    finalize_mesh_material(sc.result_view(), sc.error_view(), result)?;
    finalize_mesh(sc.result_view(), sc.error_view(), result)?;
    update_face_groups(sc.result_view(), sc.error_view(), result, true)?;

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

    // Initializing sc's own two allocators from sc's own error slot and sc's
    // own opts allocator descriptors, named by `'static` NUL-terminated
    // literals.
    init_ator(
        sc.error_mut_ptr(),
        sc.ator_tmp_view(),
        Some(sc.opts_view().temp_allocator_view()),
        c"temp",
    );
    init_ator(
        sc.error_mut_ptr(),
        sc.ator_result_view(),
        Some(sc.opts_view().result_allocator_view()),
        c"result",
    );

    sc.result_view().set_unordered(true);
    sc.source_view().set_unordered(true);
    sc.tmp_view().set_unordered(true);

    // SAFETY: both empty scratch buffers are owned by `sc` and use its live,
    // initialized temp allocator until teardown.
    unsafe {
        sc.source_view().set_ator(sc.ator_tmp_mut_ptr());
        sc.tmp_view().set_ator(sc.ator_tmp_mut_ptr());
    }

    let mut i: usize = 1;
    while i < level {
        // SAFETY: `result` is empty at the start of this level. Its allocations
        // are moved into `source` before the header is zeroed and reused; the
        // temp allocator stays live through that ownership rotation.
        unsafe { sc.result_view().set_ator(sc.ator_tmp_mut_ptr()) };

        // SAFETY: `sc` is a valid subdivide context (construction invariant);
        // its `src_mesh`/`dst_mesh` slots are two distinct fields of that
        // context, so the copy is non-overlapping, and the bufs rotated here
        // are sc's own. `Buf` is a plain pointer/integer/bool
        // aggregate, so zeroing `result` leaves a valid empty buffer.
        unsafe {
            subdivide_mesh_level(sc)?;

            // C: `sc->src_mesh = sc->dst_mesh;` — struct assignment (memcpy).
            core::ptr::copy_nonoverlapping(sc.dst_mesh_mut_ptr(), sc.src_mesh_mut_ptr(), 1);

            buf_free(sc.source_view());
            buf_free(sc.tmp_view());
            sc.set_source(sc.take_result());
            core::ptr::write_bytes(sc.result_mut_ptr() as *mut u8, 0, size_of::<Buf>());
        }
        i += 1;
    }

    // SAFETY: the prior intermediate result was moved to `source` and this
    // result header was zeroed. Final-level chunks are owned by the live result
    // allocator and transferred together with its state into the result imp.
    unsafe { sc.result_view().set_ator(sc.ator_result_mut_ptr()) };
    // SAFETY: the final level acts on sc's own state (construction invariant).
    unsafe { subdivide_mesh_level(sc)? };
    buf_free(sc.tmp_view());

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
    // encode, and either parent outlives this call. `element.scene` is read as
    // bare pointer bits (`ref_ptr`), NOT as a `Ref`: a standalone tessellated
    // mesh never has it set, and C feeds the NULL into the same arithmetic.
    let parent: *mut Refcount = unsafe {
        if src_mesh.subdivision_evaluated() && src_mesh.from_tessellated_nurbs() {
            ImpHandle::<MeshImp>::from_payload(sc.src_mesh_ptr()).refcount_ptr()
        } else {
            ImpHandle::<SceneImp>::from_payload(ref_ptr(src_mesh.element().scene_ptr()))
                .refcount_ptr()
        }
    };

    // Patch sc's own destination mesh in place.
    patch_mesh_reals(mesh);

    sc.set_imp(sc.result_view().push::<MeshImp>(1));
    ufbxi_check_err!(sc.error_view(), !sc.imp().is_null(), "sc->imp");

    let dst_sub: *mut SubdivisionResult = mesh
        .subdivision_result()
        .map_or(core::ptr::null_mut(), |r| r.ptr());
    // SAFETY: `subdivide_mesh_level` always installs a `SubdivisionResult` on
    // the destination mesh (the `result_sub` push there), so `dst_sub` is
    // non-null and points into sc's own result arena.
    unsafe {
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

    // SAFETY: `ator_tmp_view()`/`inputs()`/`inputs_cap()` are `sc`'s own
    // allocator and the `inputs` array (with its capacity) it allocated, the
    // free contract.
    unsafe { free::<SubdivideInput>(Some(sc.ator_tmp_view()), sc.inputs(), sc.inputs_cap()) };
    buf_free(sc.tmp_view());
    buf_free(sc.source_view());

    if let Ok(finished_imp) = result {
        // SAFETY: `ator_tmp_view()` is `sc`'s own live temp allocator, torn
        // down exactly once here.
        unsafe { free_ator(sc.ator_tmp_view()) };

        // C: `return &sc->imp->mesh;` — commit the finished imp across the ABI.
        // (The success-path `clear_error` of the caller's slot lives in the
        // boundary shim.)
        Ok(finished_imp.into_payload())
    } else {
        // C copies the fixed error into the caller's slot; the `Result` shape
        // carries it by value (the shim owns the slot writes).
        let mut fixed: Error = Error::default();
        let fixed_view = crate::native::error::ErrorView::from_mut(&mut fixed);
        fix_error_type(sc.error_view(), b"Failed to subdivide\0", Some(fixed_view));
        buf_free(sc.result_view());
        // SAFETY: both allocators are `sc`'s own live temp/result allocators,
        // torn down exactly once here.
        unsafe {
            free_ator(sc.ator_tmp_view());
            free_ator(sc.ator_result_view());
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
    // SAFETY: the format string is a literal with no conversions.
    unsafe {
        ufbxi_fmt_err_info!(
            Some(crate::native::error::ErrorView::from_mut(&mut error)),
            "UFBX_ENABLE_SUBDIVISION"
        )
    };
    ufbxi_report_err_msg!(
        crate::native::error::ErrorView::from_mut(&mut error),
        "UFBXI_FEATURE_SUBDIVISION",
        "Feature disabled"
    );
    Err(error)
}
