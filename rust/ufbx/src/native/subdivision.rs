//! Port of the `// -- Subdivision` banner section (ufbx.c:28821-30082).
//!
//! The whole section is gated on `UFBXI_FEATURE_SUBDIVISION`
//! (`#[cfg(feature = "subdivision")]`); the `#else` arm at ufbx.c:30069-30081
//! keeps `ufbxi_subdivide_mesh` present and reporting
//! `UFBX_ERROR_FEATURE_DISABLED`, ported here as well so the module's contract
//! is complete in both feature configurations.
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
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
use crate::native::allocator::{free, free_ator, grow_array, init_ator, Allocator, MESH_IMP_MAGIC};
#[cfg(feature = "subdivision")]
use crate::native::api::{
    compute_normals, compute_topology, generate_normal_mapping, get_vertex_real, init_ref,
    topo_next_vertex_edge, topo_prev_vertex_edge, ZERO_VEC3,
};
#[cfg(feature = "subdivision")]
use crate::native::buf::{buf_free, push, push_copy, push_size, push_zero, Buf};
#[cfg(feature = "subdivision")]
use crate::native::error::{
    clear_error, fix_error_type, memcmp, ufbxi_check_err, ufbxi_check_return_err,
};
#[cfg(not(feature = "subdivision"))]
use crate::native::error::{ufbxi_fmt_err_info, ufbxi_report_err_msg};
#[cfg(feature = "subdivision")]
use crate::native::parse::{get_imp, MeshImp, Refcount, SceneImp};
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
use crate::prelude::Real;
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
pub(crate) struct SubdivideContext(
    core::cell::UnsafeCell<core::mem::MaybeUninit<InnerSubdivideContext>>,
);

impl SubdivideContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerSubdivideContext {
        self.0.get().cast()
    }

    // `tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_mut(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp }
    }

    // `src_mesh` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn src_mesh_mut(&self) -> *mut Mesh {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).src_mesh }
    }

    // `source` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn source_mut(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).source }
    }

    // `result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn result_mut(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).result }
    }

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut(&self) -> *mut RawSubdivideOpts {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).opts }
    }

    // `inputs_cap` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn inputs_cap_mut(&self) -> *mut usize {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).inputs_cap }
    }

    // `inputs` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn inputs_mut(&self) -> *mut *mut SubdivideInput {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).inputs }
    }

    // `error` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn error_mut(&self) -> *mut Error {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).error }
    }

    // `dst_mesh` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn dst_mesh_mut(&self) -> *mut Mesh {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).dst_mesh }
    }

    // `ator_tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_tmp_mut(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_tmp }
    }

    // `ator_result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_result_mut(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_result }
    }

    // `max_vertex_weights` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn max_vertex_weights(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).max_vertex_weights }
    }

    #[inline(always)]
    pub(crate) fn set_max_vertex_weights(&self, max_vertex_weights: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).max_vertex_weights = max_vertex_weights;
        }
    }

    // `total_weights` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn total_weights(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).total_weights }
    }

    #[inline(always)]
    pub(crate) fn set_total_weights(&self, total_weights: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).total_weights = total_weights;
        }
    }

    // `tmp_weights` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_weights(&self) -> *mut SubdivisionWeight {
        // SAFETY: reading a scalar field; all bit patterns of `*mut SubdivisionWeight` are valid.
        unsafe { (*self.get()).tmp_weights }
    }

    #[inline(always)]
    pub(crate) fn set_tmp_weights(&self, tmp_weights: *mut SubdivisionWeight) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).tmp_weights = tmp_weights;
        }
    }

    // `tmp_vertex_weights` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_vertex_weights(&self) -> *mut Real {
        // SAFETY: reading a scalar field; all bit patterns of `*mut Real` are valid.
        unsafe { (*self.get()).tmp_vertex_weights }
    }

    #[inline(always)]
    pub(crate) fn set_tmp_vertex_weights(&self, tmp_vertex_weights: *mut Real) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).tmp_vertex_weights = tmp_vertex_weights;
        }
    }

    // `inputs_cap` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn inputs_cap(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).inputs_cap }
    }

    // `inputs` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn inputs(&self) -> *mut SubdivideInput {
        // SAFETY: reading a scalar field; all bit patterns of `*mut SubdivideInput` are valid.
        unsafe { (*self.get()).inputs }
    }

    // `num_topo` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn num_topo(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).num_topo }
    }

    #[inline(always)]
    pub(crate) fn set_num_topo(&self, num_topo: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).num_topo = num_topo;
        }
    }

    // `topo` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn topo(&self) -> *mut TopoEdge {
        // SAFETY: reading a scalar field; all bit patterns of `*mut TopoEdge` are valid.
        unsafe { (*self.get()).topo }
    }

    #[inline(always)]
    pub(crate) fn set_topo(&self, topo: *mut TopoEdge) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).topo = topo;
        }
    }

    // `src_mesh_ptr` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn src_mesh_ptr(&self) -> *mut Mesh {
        // SAFETY: reading a scalar field; all bit patterns of `*mut Mesh` are valid.
        unsafe { (*self.get()).src_mesh_ptr }
    }

    #[inline(always)]
    pub(crate) fn set_src_mesh_ptr(&self, src_mesh_ptr: *mut Mesh) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).src_mesh_ptr = src_mesh_ptr;
        }
    }

    // `imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn imp(&self) -> *mut MeshImp {
        // SAFETY: reading a scalar field; all bit patterns of `*mut MeshImp` are valid.
        unsafe { (*self.get()).imp }
    }

    #[inline(always)]
    pub(crate) fn set_imp(&self, imp: *mut MeshImp) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).imp = imp;
        }
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
        let src: *const Vec2 = (*inputs.add(i)).data as *const Vec2;
        let weight: Real = (*inputs.add(i)).weight;
        dst.x += (*src).x * weight;
        dst.y += (*src).y * weight;
        i += 1;
    }
    *(output as *mut Vec2) = dst;

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
        let src: *const Vec3 = (*inputs.add(i)).data as *const Vec3;
        let weight: Real = (*inputs.add(i)).weight;
        dst.x += (*src).x * weight;
        dst.y += (*src).y * weight;
        dst.z += (*src).z * weight;
        i += 1;
    }
    *(output as *mut Vec3) = dst;

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
        let src: *const Vec4 = (*inputs.add(i)).data as *const Vec4;
        let weight: Real = (*inputs.add(i)).weight;
        dst.x += (*src).x * weight;
        dst.y += (*src).y * weight;
        dst.z += (*src).z * weight;
        dst.w += (*src).w * weight;
        i += 1;
    }
    *(output as *mut Vec4) = dst;

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
    let a: SubdivisionWeight = core::ptr::read(va as *const SubdivisionWeight);
    let b: SubdivisionWeight = core::ptr::read(vb as *const SubdivisionWeight);
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
    let sc: &SubdivideContext = &*(user as *const SubdivideContext);

    let vertex_weights: *mut Real = sc.tmp_vertex_weights();
    let tmp_weights: *mut SubdivisionWeight = sc.tmp_weights();
    let mut num_weights: usize = 0;

    // C: `ufbxi_nounroll for (size_t input_ix = 0; input_ix != num_inputs; input_ix++)`
    let mut input_ix: usize = 0;
    while input_ix != num_inputs {
        let src: SubdivisionVertexWeights =
            *((*inputs.add(input_ix)).data as *const SubdivisionVertexWeights);
        let input_weight: Real = (*inputs.add(input_ix)).weight;

        let mut weight_ix: usize = 0;
        while weight_ix < src.num_weights {
            let weight: Real = input_weight * (*src.weights.add(weight_ix)).weight;
            // C: `if (weight < 1.175494351e-38f) continue;` — a `float` literal
            // widened to `ufbx_real`.
            if weight < 1.175494351e-38f32 as Real {
                weight_ix += 1;
                continue;
            }

            let vx: u32 = (*src.weights.add(weight_ix)).index;
            ufbxi_dev_assert!((vx as usize) < (*sc.get()).src_mesh.num_vertices);

            let prev: Real = *vertex_weights.add(vx as usize);
            *vertex_weights.add(vx as usize) = prev + weight;
            if prev == 0.0 {
                (*tmp_weights.add(num_weights)).index = vx;
                num_weights += 1;
            }
            weight_ix += 1;
        }
        input_ix += 1;
    }

    // C: `ufbxi_nounroll for (size_t i = 0; i != num_weights; i++)`
    let mut i: usize = 0;
    while i != num_weights {
        let vx: u32 = (*tmp_weights.add(i)).index;
        (*tmp_weights.add(i)).weight = *vertex_weights.add(vx as usize);
        *vertex_weights.add(vx as usize) = 0.0;
        i += 1;
    }

    unstable_sort(
        tmp_weights as *mut c_void,
        num_weights,
        size_of::<SubdivisionWeight>(),
        subdivision_weight_less,
        core::ptr::null_mut(),
    );

    if sc.max_vertex_weights() != usize::MAX {
        num_weights = min_sz(sc.max_vertex_weights(), num_weights);

        // Normalize weights
        let mut prefix_weight: Real = 0.0;
        // C: `ufbxi_nounroll for (size_t i = 0; i != num_weights; i++)`
        let mut i: usize = 0;
        while i != num_weights {
            prefix_weight += (*tmp_weights.add(i)).weight;
            i += 1;
        }
        let mut i: usize = 0;
        while i != num_weights {
            (*tmp_weights.add(i)).weight /= prefix_weight;
            i += 1;
        }
    }

    sc.set_total_weights(sc.total_weights().wrapping_add(num_weights));
    let weights: *mut SubdivisionWeight =
        push_copy::<SubdivisionWeight>(sc.tmp_mut(), num_weights, tmp_weights);
    // C: `ufbxi_check_err(&sc->error, weights);` — this function returns `int`,
    // so the C macro's `return 0` is a plain 0 here.
    ufbxi_check_return_err!(sc.error_mut(), !weights.is_null(), 0, "weights");

    let dst: *mut SubdivisionVertexWeights = output as *mut SubdivisionVertexWeights;
    (*dst).weights = weights;
    (*dst).num_weights = num_weights;

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
    let twin: u32 = (*topo.add(index as usize)).twin;
    if twin != NO_INDEX {
        let a0: u32 = *(*input).indices.add(index as usize);
        let a1: u32 = *(*input)
            .indices
            .add((*topo.add(index as usize)).next as usize);
        let b0: u32 = *(*input)
            .indices
            .add((*topo.add(twin as usize)).next as usize);
        let b1: u32 = *(*input).indices.add(twin as usize);
        if a0 == b0 && a1 == b1 {
            return false;
        }
        if !(*input).check_split_data {
            return true;
        }
        let stride: usize = (*input).stride;
        let da0: *const u8 = ((*input).values as *const u8).add((a0 as usize).wrapping_mul(stride));
        let da1: *const u8 = ((*input).values as *const u8).add((a1 as usize).wrapping_mul(stride));
        let db0: *const u8 = ((*input).values as *const u8).add((b0 as usize).wrapping_mul(stride));
        let db1: *const u8 = ((*input).values as *const u8).add((b1 as usize).wrapping_mul(stride));
        if memcmp(da0, db0, stride) == 0 && memcmp(da1, db1, stride) == 0 {
            return false;
        }
        return true;
    }

    false
}

// ufbx.c:29036-29042 `ufbxi_edge_crease`
#[cfg(feature = "subdivision")]
pub(crate) unsafe fn edge_crease(
    mesh: *const Mesh,
    split: bool,
    topo: *const TopoEdge,
    index: u32,
) -> Real {
    if (*topo.add(index as usize)).twin == NO_INDEX {
        return 1.0;
    }
    if split {
        return 1.0;
    }
    if !(*mesh).edge_crease.data.is_null() && (*topo.add(index as usize)).edge != NO_INDEX {
        return *(*mesh)
            .edge_crease
            .data
            .add((*topo.add(index as usize)).edge as usize)
            * (10.0 as Real);
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
    let boundary: SubdivisionBoundary = (*input).boundary;

    let mesh: *const Mesh = core::ptr::addr_of!((*sc.get()).src_mesh);
    let topo: *const TopoEdge = sc.topo();
    let num_topo: usize = sc.num_topo();

    let edge_indices: *mut u32 = push::<u32>(sc.result_mut(), (*mesh).num_indices);
    ufbxi_check_err!(sc.error_mut(), !edge_indices.is_null(), "edge_indices");

    let mut num_edge_values: usize = 0;
    // C: `for (uint32_t ix = 0; ix < (uint32_t)mesh->num_indices; ix++)` — the
    // bound is truncated to `uint32_t` here (unlike the edge-point loop below).
    let mut ix: u32 = 0;
    while ix < (*mesh).num_indices as u32 {
        let twin: u32 = (*topo.add(ix as usize)).twin;
        if twin < ix && !is_edge_split(input, topo, ix) {
            *edge_indices.add(ix as usize) = *edge_indices.add(twin as usize);
        } else {
            *edge_indices.add(ix as usize) = num_edge_values as u32;
            num_edge_values += 1;
        }
        ix += 1;
    }

    let stride: usize = (*input).stride;
    let num_initial_values: usize = num_edge_values
        .wrapping_add((*mesh).num_faces)
        .wrapping_add((*mesh).num_indices);
    let values: *mut u8 = push_size(sc.tmp_mut(), stride, num_initial_values) as *mut u8;
    ufbxi_check_err!(sc.error_mut(), !values.is_null(), "values");

    let face_values: *mut u8 = values;
    let edge_values: *mut u8 = face_values.add((*mesh).num_faces.wrapping_mul(stride));
    let vertex_values: *mut u8 = edge_values.add(num_edge_values.wrapping_mul(stride));

    let mut num_vertex_values: usize = 0;

    let vertex_indices: *mut u32 = push::<u32>(sc.result_mut(), (*mesh).num_indices);
    ufbxi_check_err!(sc.error_mut(), !vertex_indices.is_null(), "vertex_indices");

    let min_inputs: usize = max_sz(32, (*mesh).max_face_triangles.wrapping_add(2));
    ufbxi_check_err!(
        sc.error_mut(),
        grow_array::<SubdivideInput>(
            sc.ator_tmp_mut(),
            sc.inputs_mut(),
            sc.inputs_cap_mut(),
            min_inputs,
        ),
        "ufbxi_grow_array_size((&sc->ator_tmp), sizeof(**(&sc->inputs)), (&sc->inputs), (&sc->inputs_cap), (min_inputs))"
    );
    let mut inputs: *mut SubdivideInput = sc.inputs();

    // Assume initially unique per vertex, remove if not the case
    (*output).unique_per_vertex = true;

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
    let sum_fn: SubdivideSumFn = (*input).sum_fn.unwrap_unchecked();
    let sum_user: *mut c_void = (*input).sum_user;

    // Mark unused indices as `UFBX_NO_INDEX` so we can patch non-manifold
    // C: `ufbxi_nounroll for (size_t i = 0; i < mesh->num_indices; i++)`
    let mut i: usize = 0;
    while i < (*mesh).num_indices {
        *vertex_indices.add(i) = NO_INDEX;
        i += 1;
    }

    // Face points
    let mut fi: usize = 0;
    while fi < (*mesh).num_faces {
        let face: Face = *(*mesh).faces.data.add(fi);
        let dst: *mut u8 = face_values.add(fi.wrapping_mul(stride));

        let weight: Real = 1.0 / (face.num_indices as Real);
        let mut ci: u32 = 0;
        while ci < face.num_indices {
            let ix: u32 = face.index_begin.wrapping_add(ci);
            (*inputs.add(ci as usize)).data = ((*input).values as *const u8)
                .add((*(*input).indices.add(ix as usize) as usize).wrapping_mul(stride))
                as *const c_void;
            (*inputs.add(ci as usize)).weight = weight;
            ci += 1;
        }

        ufbxi_check_err!(
            sc.error_mut(),
            sum_fn(
                sum_user,
                dst as *mut c_void,
                inputs,
                face.num_indices as usize
            ) != 0,
            "sum_fn(sum_user, dst, inputs, face.num_indices)"
        );
        fi += 1;
    }

    // Edge points
    // C: `for (uint32_t ix = 0; ix < mesh->num_indices; ix++)` — `ix` is
    // promoted to `size_t` for the comparison here (no truncation).
    let mut ix: u32 = 0;
    while (ix as usize) < (*mesh).num_indices {
        let dst: *mut u8 =
            edge_values.add((*edge_indices.add(ix as usize) as usize).wrapping_mul(stride));

        let twin: u32 = (*topo.add(ix as usize)).twin;
        let split: bool = is_edge_split(input, topo, ix);

        if split
            || (*topo.add(ix as usize))
                .flags
                .has_any(TopoFlags::NON_MANIFOLD)
        {
            (*output).unique_per_vertex = false;
        }

        let mut crease: Real = 0.0;
        if split || twin == NO_INDEX {
            crease = 1.0;
        } else if (*topo.add(ix as usize)).edge != NO_INDEX && !(*mesh).edge_crease.data.is_null() {
            crease = *(*mesh)
                .edge_crease
                .data
                .add((*topo.add(ix as usize)).edge as usize)
                * (10.0 as Real);
        }
        if sharp_all {
            crease = 1.0;
        }

        let v0: *const u8 = ((*input).values as *const u8)
            .add((*(*input).indices.add(ix as usize) as usize).wrapping_mul(stride));
        let v1: *const u8 = ((*input).values as *const u8).add(
            (*(*input).indices.add((*topo.add(ix as usize)).next as usize) as usize)
                .wrapping_mul(stride),
        );

        // TODO: Unify
        if twin < ix && !split {
            // Already calculated
        } else if crease <= 0.0 {
            let f0: *const u8 =
                face_values.add(((*topo.add(ix as usize)).face as usize).wrapping_mul(stride));
            let f1: *const u8 =
                face_values.add(((*topo.add(twin as usize)).face as usize).wrapping_mul(stride));
            (*inputs.add(0)).data = v0 as *const c_void;
            (*inputs.add(0)).weight = 0.25;
            (*inputs.add(1)).data = v1 as *const c_void;
            (*inputs.add(1)).weight = 0.25;
            (*inputs.add(2)).data = f0 as *const c_void;
            (*inputs.add(2)).weight = 0.25;
            (*inputs.add(3)).data = f1 as *const c_void;
            (*inputs.add(3)).weight = 0.25;
            ufbxi_check_err!(
                sc.error_mut(),
                sum_fn(sum_user, dst as *mut c_void, inputs, 4) != 0,
                "sum_fn(sum_user, dst, inputs, 4)"
            );
        } else if crease >= 1.0 {
            (*inputs.add(0)).data = v0 as *const c_void;
            (*inputs.add(0)).weight = 0.5;
            (*inputs.add(1)).data = v1 as *const c_void;
            (*inputs.add(1)).weight = 0.5;
            ufbxi_check_err!(
                sc.error_mut(),
                sum_fn(sum_user, dst as *mut c_void, inputs, 2) != 0,
                "sum_fn(sum_user, dst, inputs, 2)"
            );
        } else if crease < 1.0 {
            let f0: *const u8 =
                face_values.add(((*topo.add(ix as usize)).face as usize).wrapping_mul(stride));
            let f1: *const u8 =
                face_values.add(((*topo.add(twin as usize)).face as usize).wrapping_mul(stride));
            let w0: Real = 0.25 + 0.25 * crease;
            let w1: Real = 0.25 - 0.25 * crease;

            (*inputs.add(0)).data = v0 as *const c_void;
            (*inputs.add(0)).weight = w0;
            (*inputs.add(1)).data = v1 as *const c_void;
            (*inputs.add(1)).weight = w0;
            (*inputs.add(2)).data = f0 as *const c_void;
            (*inputs.add(2)).weight = w1;
            (*inputs.add(3)).data = f1 as *const c_void;
            (*inputs.add(3)).weight = w1;
            ufbxi_check_err!(
                sc.error_mut(),
                sum_fn(sum_user, dst as *mut c_void, inputs, 4) != 0,
                "sum_fn(sum_user, dst, inputs, 4)"
            );
        }
        ix = ix.wrapping_add(1);
    }

    // Vertex points
    let mut vi: usize = 0;
    while vi < (*mesh).num_vertices {
        let mut original_start: u32 = *(*mesh).vertex_first_index.data.add(vi);
        if original_start == NO_INDEX {
            vi += 1;
            continue;
        }

        // Find a topological boundary, or if not found a split edge
        let mut start: u32 = original_start;
        // C: `for (uint32_t cur = start;;)`
        let mut cur: u32 = start;
        loop {
            let prev: u32 = topo_prev_vertex_edge(topo, num_topo, cur);
            if prev == NO_INDEX {
                start = cur;
                break;
            } // Topological boundary: Stop and use as start
            if is_edge_split(input, topo, prev) {
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
                (*output).unique_per_vertex = false;
            }

            let value_index: u32 = num_vertex_values as u32;
            num_vertex_values += 1;
            let dst: *mut u8 = vertex_values.add((value_index as usize).wrapping_mul(stride));

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
            let start_prev: u32 = (*topo.add(start as usize)).prev;
            let end_edge: u32 = (*topo.add(start_prev as usize)).twin;
            let mut valence: usize = 2;

            non_manifold |= (*topo.add(start as usize))
                .flags
                .has_any(TopoFlags::NON_MANIFOLD);
            non_manifold |= (*topo.add(start_prev as usize))
                .flags
                .has_any(TopoFlags::NON_MANIFOLD);

            let v0: *const u8 = ((*input).values as *const u8)
                .add((*(*input).indices.add(start as usize) as usize).wrapping_mul(stride));

            let mut num_inputs: usize = 4;

            {
                let e0: *const u8 = ((*input).values as *const u8).add(
                    (*(*input)
                        .indices
                        .add((*topo.add(start as usize)).next as usize)
                        as usize)
                        .wrapping_mul(stride),
                );
                let e1: *const u8 = ((*input).values as *const u8).add(
                    (*(*input).indices.add(start_prev as usize) as usize).wrapping_mul(stride),
                );
                let f0: *const u8 = face_values
                    .add(((*topo.add(start as usize)).face as usize).wrapping_mul(stride));
                (*inputs.add(0)).data = v0 as *const c_void;
                (*inputs.add(1)).data = e0 as *const c_void;
                (*inputs.add(2)).data = e1 as *const c_void;
                (*inputs.add(3)).data = f0 as *const c_void;
            }

            let start_split: bool = is_edge_split(input, topo, start);
            let prev_split: bool = end_edge != NO_INDEX && is_edge_split(input, topo, end_edge);

            // Either of the first two edges may be creased
            let start_crease: Real = edge_crease(mesh, start_split, topo, start);
            if start_crease > 0.0 {
                total_crease += start_crease;
                crease_input_indices[num_crease] = 1;
                num_crease += 1;
            }
            let prev_crease: Real = edge_crease(mesh, prev_split, topo, start_prev);
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
                sc.error_mut(),
                *vertex_indices.add(start as usize) == NO_INDEX,
                "vertex_indices[start] == UFBX_NO_INDEX"
            );
            *vertex_indices.add(start as usize) = value_index;

            if start_split {
                // We need to special case if the first edge is split as we have
                // handled it already in the code above..
                start = topo_next_vertex_edge(topo, num_topo, start);
                num_split += 1;
            } else {
                // Follow vertex edges until we either hit a topological/split boundary
                // or loop back to the left edge we accounted for in `start_prev`
                let mut cur: u32 = start;
                loop {
                    cur = topo_next_vertex_edge(topo, num_topo, cur);

                    // Topological boundary: Finished
                    if cur == NO_INDEX {
                        on_boundary = true;
                        start = NO_INDEX;
                        break;
                    }

                    non_manifold |= (*topo.add(cur as usize))
                        .flags
                        .has_any(TopoFlags::NON_MANIFOLD);
                    ufbxi_check_err!(
                        sc.error_mut(),
                        *vertex_indices.add(cur as usize) == NO_INDEX,
                        "vertex_indices[cur] == UFBX_NO_INDEX"
                    );
                    *vertex_indices.add(cur as usize) = value_index;

                    let split: bool = is_edge_split(input, topo, cur);

                    // Looped: Add the face from the other side still if not split
                    if cur == end_edge && !split {
                        ufbxi_check_err!(
                            sc.error_mut(),
                            grow_array::<SubdivideInput>(
                                sc.ator_tmp_mut(),
                                sc.inputs_mut(),
                                sc.inputs_cap_mut(),
                                num_inputs.wrapping_add(1),
                            ),
                            "ufbxi_grow_array_size((&sc->ator_tmp), sizeof(**(&sc->inputs)), (&sc->inputs), (&sc->inputs_cap), (num_inputs + 1))"
                        );
                        let f0: *const u8 = face_values
                            .add(((*topo.add(cur as usize)).face as usize).wrapping_mul(stride));
                        // C-parity: C does NOT refresh the local `inputs` from
                        // `sc->inputs` after this grow (unlike the paired grow
                        // below) — the stale pointer is written through
                        // verbatim.
                        (*inputs.add(num_inputs)).data = f0 as *const c_void;
                        start = NO_INDEX;
                        num_inputs += 1;
                        break;
                    }

                    // Add the edge crease, this also handles boundaries as they
                    // have an implicit crease of 1.0 using `ufbxi_edge_crease()`
                    let cur_crease: Real = edge_crease(mesh, split, topo, cur);
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
                            sc.error_mut(),
                            grow_array::<SubdivideInput>(
                                sc.ator_tmp_mut(),
                                sc.inputs_mut(),
                                sc.inputs_cap_mut(),
                                num_inputs.wrapping_add(2),
                            ),
                            "ufbxi_grow_array_size((&sc->ator_tmp), sizeof(**(&sc->inputs)), (&sc->inputs), (&sc->inputs_cap), (num_inputs + 2))"
                        );
                        inputs = sc.inputs();

                        let e0: *const u8 = ((*input).values as *const u8).add(
                            (*(*input)
                                .indices
                                .add((*topo.add(cur as usize)).next as usize)
                                as usize)
                                .wrapping_mul(stride),
                        );
                        let f0: *const u8 = face_values
                            .add(((*topo.add(cur as usize)).face as usize).wrapping_mul(stride));
                        (*inputs.add(num_inputs + 0)).data = e0 as *const c_void;
                        (*inputs.add(num_inputs + 1)).data = f0 as *const c_void;
                        num_inputs += 2;
                    }
                    valence += 1;

                    // If we landed at a split edge advance to the next one
                    // and continue from there in the outer loop
                    if split {
                        start = topo_next_vertex_edge(topo, num_topo, cur);
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
                (*inputs.add(0)).data = v0 as *const c_void;
                (*inputs.add(0)).weight = 1.0;
                num_inputs = 1;
            } else if num_crease == 2 {
                // Boundary: Interpolate edge
                total_crease *= 0.5;
                if total_crease < 0.0 {
                    total_crease = 0.0;
                }
                if total_crease > 1.0 {
                    total_crease = 1.0;
                }

                (*inputs.add(0)).weight = v_weight * (1.0 - total_crease) + 0.75 * total_crease;
                let few: Real = fe_weight * (1.0 - total_crease);
                let mut i: usize = 1;
                while i < num_inputs {
                    (*inputs.add(i)).weight = few;
                    i += 1;
                }

                // Add weight to the creased edges
                (*inputs.add(crease_input_indices[0])).weight += 0.125 * total_crease;
                (*inputs.add(crease_input_indices[1])).weight += 0.125 * total_crease;
            } else {
                // Regular: Weighted sum with the accumulated edge/face points
                (*inputs.add(0)).weight = v_weight;
                let mut i: usize = 1;
                while i < num_inputs {
                    (*inputs.add(i)).weight = fe_weight;
                    i += 1;
                }
            }

            if (*mesh).vertex_crease.exists {
                let mut v: Real = get_vertex_real(&(*mesh).vertex_crease, original_start as usize);
                v *= 10.0 as Real;
                if v > 0.0 {
                    if v > 1.0 {
                        v = 1.0;
                    }

                    let iv: Real = 1.0 - v;
                    (*inputs.add(0)).weight = 1.0 * v + ((*inputs.add(0)).weight) * iv;
                    let mut i: usize = 1;
                    while i < num_inputs {
                        (*inputs.add(i)).weight *= iv;
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
                    total_weight += (*inputs.add(i)).weight;
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
                sc.error_mut(),
                sum_fn(sum_user, dst as *mut c_void, inputs, num_inputs) != 0,
                "sum_fn(sum_user, dst, inputs, num_inputs)"
            );
        }
        vi += 1;
    }

    // Copy non-manifold vertex values as-is
    let mut old_ix: usize = 0;
    while old_ix < (*mesh).num_indices {
        let mut ix: u32 = *vertex_indices.add(old_ix);
        if ix == NO_INDEX {
            ix = num_vertex_values as u32;
            num_vertex_values += 1;
            *vertex_indices.add(old_ix) = ix;
            let src: *const u8 = ((*input).values as *const u8)
                .add((*(*input).indices.add(old_ix) as usize).wrapping_mul(stride));
            let dst: *mut u8 = vertex_values.add((ix as usize).wrapping_mul(stride));

            (*inputs.add(0)).data = src as *const c_void;
            (*inputs.add(0)).weight = 1.0;
            ufbxi_check_err!(
                sc.error_mut(),
                sum_fn(sum_user, dst as *mut c_void, inputs, 1) != 0,
                "sum_fn(sum_user, dst, inputs, 1)"
            );
        }
        old_ix += 1;
    }

    ufbx_assert!(num_vertex_values <= (*mesh).num_indices);
    let num_values: usize = num_edge_values
        .wrapping_add((*mesh).num_faces)
        .wrapping_add(num_vertex_values);
    let mut new_values: *mut u8 =
        push_size(sc.result_mut(), stride, num_values.wrapping_add(1)) as *mut u8;
    ufbxi_check_err!(sc.error_mut(), !new_values.is_null(), "new_values");

    core::ptr::write_bytes(new_values, 0, stride);
    new_values = new_values.add(stride);

    core::ptr::copy_nonoverlapping(values, new_values, num_values.wrapping_mul(stride));

    (*output).values = new_values as *mut c_void;
    (*output).num_values = num_values;

    if !(*input).ignore_indices {
        let new_indices: *mut u32 =
            push::<u32>(sc.result_mut(), (*mesh).num_indices.wrapping_mul(4));
        ufbxi_check_err!(sc.error_mut(), !new_indices.is_null(), "new_indices");

        let face_start: u32 = 0;
        let edge_start: u32 = face_start.wrapping_add((*mesh).num_faces as u32);
        let vert_start: u32 = edge_start.wrapping_add(num_edge_values as u32);
        let mut p_ix: *mut u32 = new_indices;
        let mut ix: usize = 0;
        while ix < (*mesh).num_indices {
            *p_ix.add(0) = vert_start.wrapping_add(*vertex_indices.add(ix));
            *p_ix.add(1) = edge_start.wrapping_add(*edge_indices.add(ix));
            *p_ix.add(2) = face_start.wrapping_add((*topo.add(ix)).face);
            *p_ix.add(3) =
                edge_start.wrapping_add(*edge_indices.add((*topo.add(ix)).prev as usize));
            p_ix = p_ix.add(4);
            ix += 1;
        }
        (*output).indices = new_indices;
        (*output).num_indices = (*mesh).num_indices.wrapping_mul(4);
    } else {
        (*output).indices = core::ptr::null_mut();
        (*output).num_indices = 0;
    }

    Ok(())
}

// ufbx.c:29464-29489 `ufbxi_subdivide_attrib`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) unsafe fn subdivide_attrib(
    sc: &SubdivideContext,
    attrib: *mut VertexAttrib,
    boundary: SubdivisionBoundary,
    check_split_data: bool,
) -> Result<(), crate::native::error::Fail> {
    if !(*attrib).exists {
        return Ok(());
    }

    ufbx_assert!((*attrib).value_reals >= 2 && (*attrib).value_reals <= 4);

    let mut input_mem = MaybeUninit::<SubdivideLayerInput>::uninit(); // ufbxi_uninit
    let input: *mut SubdivideLayerInput = input_mem.as_mut_ptr();
    (*input).sum_fn = REAL_SUM_FNS[(*attrib).value_reals - 1];
    (*input).sum_user = core::ptr::null_mut();
    (*input).values = (*attrib).values.data;
    (*input).indices = (*attrib).indices.data;
    (*input).stride = (*attrib).value_reals.wrapping_mul(size_of::<Real>());
    (*input).boundary = boundary;
    (*input).check_split_data = check_split_data;
    (*input).ignore_indices = false;

    let mut output_mem = MaybeUninit::<SubdivideLayerOutput>::uninit(); // ufbxi_uninit
    let output: *mut SubdivideLayerOutput = output_mem.as_mut_ptr();
    subdivide_layer(sc, output, input)?;

    (*attrib).values.data = (*output).values;
    (*attrib).indices.data = (*output).indices;
    (*attrib).values.count = (*output).num_values;
    (*attrib).indices.count = (*output).num_indices;

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
        push::<SubdivisionVertexWeights>(sc.tmp_mut(), ranges.count);
    ufbxi_check_return_err!(sc.error_mut(), !dst.is_null(), core::ptr::null_mut(), "dst");

    // C: `ufbxi_nounroll for (size_t i = 0; i != ranges.count; i++)`
    let mut i: usize = 0;
    while i != ranges.count {
        let range: SubdivisionWeightRange = core::ptr::read(ranges.data.add(i));
        (*dst.add(i)).weights =
            (weights.data as *mut SubdivisionWeight).add(range.weight_begin as usize);
        (*dst.add(i)).num_weights = range.num_weights as usize;
        i += 1;
    }

    dst
}

// ufbx.c:29505-29519 `ufbxi_init_source_vertex_weights`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) unsafe fn init_source_vertex_weights(
    sc: &SubdivideContext,
    num_vertices: usize,
) -> *mut SubdivisionVertexWeights {
    let dst: *mut SubdivisionVertexWeights =
        push::<SubdivisionVertexWeights>(sc.tmp_mut(), num_vertices);
    let weights: *mut SubdivisionWeight = push::<SubdivisionWeight>(sc.tmp_mut(), num_vertices);
    ufbxi_check_return_err!(
        sc.error_mut(),
        !dst.is_null() && !weights.is_null(),
        core::ptr::null_mut(),
        "dst && weights"
    );

    // C: `ufbxi_nounroll for (size_t i = 0; i != num_vertices; i++)`
    let mut i: usize = 0;
    while i != num_vertices {
        (*dst.add(i)).weights = weights.add(i);
        (*dst.add(i)).num_weights = 1;
        (*weights.add(i)).index = i as u32;
        (*weights.add(i)).weight = 1.0;
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
        push::<SubdivisionVertexWeights>(sc.tmp_mut(), num_vertices);
    ufbxi_check_return_err!(sc.error_mut(), !dst.is_null(), core::ptr::null_mut(), "dst");

    let mut i: usize = 0;
    while i < num_vertices {
        ufbxi_dev_assert!(i < (*skin).vertices.count);
        let vertex: SkinVertex = *(*skin).vertices.data.add(i);
        let num_weights: usize = min_sz(sc.max_vertex_weights(), vertex.num_weights as usize);

        let weights: *mut SubdivisionWeight = push::<SubdivisionWeight>(sc.tmp_mut(), num_weights);
        // C: `ufbxi_check_err(&sc->error, weights);` — pointer-returning
        // function, so the C macro's `return 0` is NULL here.
        ufbxi_check_return_err!(
            sc.error_mut(),
            !weights.is_null(),
            core::ptr::null_mut(),
            "weights"
        );

        let skin_weights: *const SkinWeight =
            (*skin).weights.data.add(vertex.weight_begin as usize);

        (*dst.add(i)).weights = weights;
        (*dst.add(i)).num_weights = num_weights;
        // C: `ufbxi_nounroll for (size_t wi = 0; wi != num_weights; wi++)`
        let mut wi: usize = 0;
        while wi != num_weights {
            ufbxi_check_return_err!(
                sc.error_mut(),
                (*skin_weights.add(wi)).cluster_index <= i32::MAX as u32,
                core::ptr::null_mut(),
                "skin_weights[wi].cluster_index <= INT32_MAX"
            );
            (*weights.add(wi)).index = (*skin_weights.add(wi)).cluster_index;
            (*weights.add(wi)).weight = (*skin_weights.add(wi)).weight;
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
    ufbxi_check_err!(sc.error_mut(), !src.is_null(), "src");

    let mut input_mem = MaybeUninit::<SubdivideLayerInput>::uninit(); // ufbxi_uninit
    let input: *mut SubdivideLayerInput = input_mem.as_mut_ptr();
    (*input).sum_fn = Some(subdivide_sum_vertex_weights);
    (*input).sum_user = (sc as *const SubdivideContext) as *mut c_void;
    (*input).values = src as *const c_void;
    (*input).indices = (*sc.get()).src_mesh.vertex_indices.data;
    (*input).stride = size_of::<SubdivisionVertexWeights>();
    (*input).boundary = (*sc.get()).opts.boundary;
    (*input).check_split_data = false;
    (*input).ignore_indices = true;

    sc.set_total_weights(0);

    let mut output_mem = MaybeUninit::<SubdivideLayerOutput>::uninit(); // ufbxi_uninit
    let output: *mut SubdivideLayerOutput = output_mem.as_mut_ptr();
    subdivide_layer(sc, output, input)?;

    let num_vertices: usize = (*output).num_values;
    ufbx_assert!(num_vertices == (*sc.get()).dst_mesh.vertex_position.values.count);

    let dst_ranges: *mut SubdivisionWeightRange =
        push::<SubdivisionWeightRange>(sc.result_mut(), num_vertices);
    let dst_weights: *mut SubdivisionWeight =
        push::<SubdivisionWeight>(sc.result_mut(), sc.total_weights());
    // C-parity: upstream checks the OUT parameters `ranges && weights`, not the
    // freshly pushed `dst_ranges && dst_weights` (ufbx.c:29573) — ported
    // verbatim.
    ufbxi_check_err!(
        sc.error_mut(),
        !ranges.is_null() && !weights.is_null(),
        "ranges && weights"
    );

    let src_weights: *mut SubdivisionVertexWeights =
        (*output).values as *mut SubdivisionVertexWeights;

    let mut weight_offset: usize = 0;
    let mut vi: usize = 0;
    while vi < num_vertices {
        let ws: SubdivisionVertexWeights = *src_weights.add(vi);
        ufbxi_check_err!(
            sc.error_mut(),
            (u32::MAX as usize).wrapping_sub(weight_offset) >= ws.num_weights,
            "(size_t)UINT32_MAX - weight_offset >= ws.num_weights"
        );

        (*dst_ranges.add(vi)).weight_begin = weight_offset as u32;
        (*dst_ranges.add(vi)).num_weights = ws.num_weights as u32;
        core::ptr::copy_nonoverlapping(ws.weights, dst_weights.add(weight_offset), ws.num_weights);
        weight_offset = weight_offset.wrapping_add(ws.num_weights);
        vi += 1;
    }

    (*ranges).data = dst_ranges;
    (*ranges).count = num_vertices;
    (*weights).data = dst_weights;
    (*weights).count = sc.total_weights();

    Ok(())
}

// ufbx.c:29596-29629 `ufbxi_subdivide_vertex_crease`
#[cfg(feature = "subdivision")]
#[must_use]
#[inline(never)]
pub(crate) unsafe fn subdivide_vertex_crease(
    sc: &SubdivideContext,
    dst: *mut VertexReal,
    src: *const VertexReal,
) -> Result<(), crate::native::error::Fail> {
    let src_indices: usize = (*src).indices.count;
    let src_values: usize = (*src).values.count;

    (*dst).values.count = src_values.wrapping_add(1);
    (*dst).values.data = push::<Real>(sc.result_mut(), (*dst).values.count);
    ufbxi_check_err!(
        sc.error_mut(),
        !(*dst).values.data.is_null(),
        "dst->values.data"
    );
    *((*dst).values.data as *mut Real).add(src_values) = 0.0;

    (*dst).indices.count = src_indices.wrapping_mul(4);
    (*dst).indices.data = push::<u32>(sc.result_mut(), (*dst).indices.count);
    ufbxi_check_err!(
        sc.error_mut(),
        !(*dst).indices.data.is_null(),
        "dst->indices.data"
    );

    // Reduce the amount of vertex crease on each iteration
    // C: `ufbxi_nounroll for (size_t i = 0; i < src_values; i++)`
    let mut i: usize = 0;
    while i < src_values {
        let mut crease: Real = *(*src).values.data.add(i);
        // C: `0.999f` / `0.1f` are `float` literals widened to `ufbx_real`.
        if crease < 0.999f32 as Real {
            crease -= 0.1f32 as Real;
        }
        if crease < 0.0 {
            crease = 0.0;
        }
        *((*dst).values.data as *mut Real).add(i) = crease;
        i += 1;
    }

    // Write the crease at the vertex corner and zero (at `src_values`) on other ones
    let zero_index: u32 = src_values as u32;
    // C: `ufbxi_nounroll for (size_t i = 0; i < src_indices; i++)`
    let mut i: usize = 0;
    while i < src_indices {
        let quad: *mut u32 = ((*dst).indices.data as *mut u32).add(i.wrapping_mul(4));
        *quad.add(0) = *(*src).indices.data.add(i);
        *quad.add(1) = zero_index;
        *quad.add(2) = zero_index;
        *quad.add(3) = zero_index;
        i += 1;
    }

    Ok(())
}

// ufbx.c:29631-29925 `ufbxi_subdivide_mesh_level`
#[cfg(feature = "subdivision")]
#[must_use]
#[inline(never)]
pub(crate) unsafe fn subdivide_mesh_level(
    sc: &SubdivideContext,
) -> Result<(), crate::native::error::Fail> {
    let mesh: *const Mesh = core::ptr::addr_of!((*sc.get()).src_mesh);
    let result: *mut Mesh = sc.dst_mesh_mut();

    // C: `*result = *mesh;` — struct assignment (memcpy).
    core::ptr::copy_nonoverlapping(mesh, result, 1);

    let topo: *mut TopoEdge = push::<TopoEdge>(sc.tmp_mut(), (*mesh).num_indices);
    ufbxi_check_err!(sc.error_mut(), !topo.is_null(), "topo");
    compute_topology(mesh, topo, (*mesh).num_indices);
    sc.set_topo(topo);
    sc.set_num_topo((*mesh).num_indices);

    subdivide_attrib(
        sc,
        (&mut (*result).vertex_position) as *mut VertexVec3 as *mut VertexAttrib,
        (*sc.get()).opts.boundary,
        false,
    )?;

    core::ptr::write_bytes(
        &mut (*result).vertex_uv as *mut _ as *mut u8,
        0,
        size_of::<crate::generated::VertexVec2>(),
    );
    core::ptr::write_bytes(
        &mut (*result).vertex_tangent as *mut _ as *mut u8,
        0,
        size_of::<VertexVec3>(),
    );
    core::ptr::write_bytes(
        &mut (*result).vertex_bitangent as *mut _ as *mut u8,
        0,
        size_of::<VertexVec3>(),
    );
    core::ptr::write_bytes(
        &mut (*result).vertex_color as *mut _ as *mut u8,
        0,
        size_of::<crate::generated::VertexVec4>(),
    );

    (*result).uv_sets.data = push_copy::<UvSet>(
        sc.result_mut(),
        (*result).uv_sets.count,
        (*result).uv_sets.data,
    );
    ufbxi_check_err!(
        sc.error_mut(),
        !(*result).uv_sets.data.is_null(),
        "result->uv_sets.data"
    );

    (*result).color_sets.data = push_copy::<ColorSet>(
        sc.result_mut(),
        (*result).color_sets.count,
        (*result).color_sets.data,
    );
    ufbxi_check_err!(
        sc.error_mut(),
        !(*result).color_sets.data.is_null(),
        "result->color_sets.data"
    );

    // C: `ufbxi_for_list(ufbx_uv_set, set, result->uv_sets)`
    {
        let mut set: *mut UvSet = (*result).uv_sets.data as *mut UvSet;
        let set_end: *mut UvSet = set.add((*result).uv_sets.count);
        while set != set_end {
            subdivide_attrib(
                sc,
                (&mut (*set).vertex_uv) as *mut crate::generated::VertexVec2 as *mut VertexAttrib,
                (*sc.get()).opts.uv_boundary,
                true,
            )?;
            if (*sc.get()).opts.interpolate_tangents {
                subdivide_attrib(
                    sc,
                    (&mut (*set).vertex_tangent) as *mut VertexVec3 as *mut VertexAttrib,
                    (*sc.get()).opts.uv_boundary,
                    true,
                )?;
                subdivide_attrib(
                    sc,
                    (&mut (*set).vertex_bitangent) as *mut VertexVec3 as *mut VertexAttrib,
                    (*sc.get()).opts.uv_boundary,
                    true,
                )?;
            } else {
                core::ptr::write_bytes(
                    &mut (*set).vertex_tangent as *mut _ as *mut u8,
                    0,
                    size_of::<VertexVec3>(),
                );
                core::ptr::write_bytes(
                    &mut (*set).vertex_bitangent as *mut _ as *mut u8,
                    0,
                    size_of::<VertexVec3>(),
                );
            }
            set = set.add(1);
        }
    }

    // C: `ufbxi_for_list(ufbx_color_set, set, result->color_sets)`
    {
        let mut set: *mut ColorSet = (*result).color_sets.data as *mut ColorSet;
        let set_end: *mut ColorSet = set.add((*result).color_sets.count);
        while set != set_end {
            subdivide_attrib(
                sc,
                (&mut (*set).vertex_color) as *mut crate::generated::VertexVec4
                    as *mut VertexAttrib,
                (*sc.get()).opts.uv_boundary,
                true,
            )?;
            set = set.add(1);
        }
    }

    if (*result).uv_sets.count > 0 {
        // C: struct assignments from `uv_sets.data[0]`.
        core::ptr::copy_nonoverlapping(
            &(*(*result).uv_sets.data.add(0)).vertex_uv,
            &mut (*result).vertex_uv,
            1,
        );
        core::ptr::copy_nonoverlapping(
            &(*(*result).uv_sets.data.add(0)).vertex_bitangent,
            &mut (*result).vertex_bitangent,
            1,
        );
        core::ptr::copy_nonoverlapping(
            &(*(*result).uv_sets.data.add(0)).vertex_tangent,
            &mut (*result).vertex_tangent,
            1,
        );
    }
    if (*result).color_sets.count > 0 {
        core::ptr::copy_nonoverlapping(
            &(*(*result).color_sets.data.add(0)).vertex_color,
            &mut (*result).vertex_color,
            1,
        );
    }

    if (*sc.get()).opts.interpolate_normals && !(*sc.get()).opts.ignore_normals {
        subdivide_attrib(
            sc,
            (&mut (*result).vertex_normal) as *mut VertexVec3 as *mut VertexAttrib,
            (*sc.get()).opts.boundary,
            true,
        )?;
        // C: `ufbxi_for_list(ufbx_vec3, normal, result->vertex_normal.values)`
        {
            let mut normal: *mut Vec3 = (*result).vertex_normal.values.data as *mut Vec3;
            let normal_end: *mut Vec3 = normal.add((*result).vertex_normal.values.count);
            while normal != normal_end {
                *normal = slow_normalize3(normal);
                normal = normal.add(1);
            }
        }
        if (*mesh).skinned_normal.values.data == (*mesh).vertex_normal.values.data {
            core::ptr::copy_nonoverlapping(
                &(*result).vertex_normal,
                &mut (*result).skinned_normal,
                1,
            );
        } else {
            subdivide_attrib(
                sc,
                (&mut (*result).skinned_normal) as *mut VertexVec3 as *mut VertexAttrib,
                (*sc.get()).opts.boundary,
                true,
            )?;
            // C: `ufbxi_for_list(ufbx_vec3, normal, result->skinned_normal.values)`
            let mut normal: *mut Vec3 = (*result).skinned_normal.values.data as *mut Vec3;
            let normal_end: *mut Vec3 = normal.add((*result).skinned_normal.values.count);
            while normal != normal_end {
                *normal = slow_normalize3(normal);
                normal = normal.add(1);
            }
        }
    }

    if (*result).vertex_crease.exists {
        subdivide_vertex_crease(sc, &mut (*result).vertex_crease, &(*mesh).vertex_crease)?;
    }

    if (*mesh).skinned_position.values.data == (*mesh).vertex_position.values.data {
        core::ptr::copy_nonoverlapping(
            &(*result).vertex_position,
            &mut (*result).skinned_position,
            1,
        );
    } else {
        subdivide_attrib(
            sc,
            (&mut (*result).skinned_position) as *mut VertexVec3 as *mut VertexAttrib,
            (*sc.get()).opts.boundary,
            false,
        )?;
    }

    let result_sub: *mut SubdivisionResult = push_zero::<SubdivisionResult>(sc.result_mut(), 1);
    ufbxi_check_err!(sc.error_mut(), !result_sub.is_null(), "result_sub");
    (*result).subdivision_result = opt_ref(result_sub);

    if (*sc.get()).opts.evaluate_source_vertices || (*sc.get()).opts.evaluate_skin_weights {
        let mesh_sub: *mut SubdivisionResult = opt_ptr(&(*mesh).subdivision_result);

        let mut skin: *mut SkinDeformer = core::ptr::null_mut();
        if (*sc.get()).opts.evaluate_skin_weights {
            if (*mesh).skin_deformers.count > 0 {
                ufbxi_check_err!(
                    sc.error_mut(),
                    (*sc.get()).opts.skin_deformer_index < (*mesh).skin_deformers.count,
                    "sc->opts.skin_deformer_index < mesh->skin_deformers.count"
                );
                skin = ref_ptr(
                    (*mesh)
                        .skin_deformers
                        .data
                        .add((*sc.get()).opts.skin_deformer_index),
                );
            }
        }

        let mut max_weights: usize = 0;
        if (*sc.get()).opts.evaluate_source_vertices {
            max_weights = max_sz(max_weights, (*mesh).num_vertices);
        }
        if !skin.is_null() {
            max_weights = max_sz(max_weights, (*skin).clusters.count);
        }

        sc.set_tmp_vertex_weights(push_zero::<Real>(sc.tmp_mut(), (*mesh).num_vertices));
        sc.set_tmp_weights(push::<SubdivisionWeight>(sc.tmp_mut(), max_weights));
        ufbxi_check_err!(
            sc.error_mut(),
            !sc.tmp_vertex_weights().is_null() && !sc.tmp_weights().is_null(),
            "sc->tmp_vertex_weights && sc->tmp_weights"
        );

        if (*sc.get()).opts.evaluate_source_vertices {
            sc.set_max_vertex_weights(if (*sc.get()).opts.max_source_vertices != 0 {
                (*sc.get()).opts.max_source_vertices
            } else {
                usize::MAX
            });

            let weights: *mut SubdivisionVertexWeights;
            if !mesh_sub.is_null() && (*mesh_sub).source_vertex_ranges.count > 0 {
                weights = subdivision_copy_weights(
                    sc,
                    core::ptr::read(&(*mesh_sub).source_vertex_ranges),
                    core::ptr::read(&(*mesh_sub).source_vertex_weights),
                );
            } else {
                weights = init_source_vertex_weights(sc, (*mesh).num_vertices);
            }

            subdivide_weights(
                sc,
                &mut (*result_sub).source_vertex_ranges,
                &mut (*result_sub).source_vertex_weights,
                weights,
            )?;
        }

        if !skin.is_null() {
            sc.set_max_vertex_weights(if (*sc.get()).opts.max_skin_weights != 0 {
                (*sc.get()).opts.max_skin_weights
            } else {
                usize::MAX
            });

            let weights: *mut SubdivisionVertexWeights;
            // C-parity: the guard reads `source_vertex_ranges` here too
            // (ufbx.c:29750), not `skin_cluster_ranges`.
            if !mesh_sub.is_null() && (*mesh_sub).source_vertex_ranges.count > 0 {
                weights = subdivision_copy_weights(
                    sc,
                    core::ptr::read(&(*mesh_sub).skin_cluster_ranges),
                    core::ptr::read(&(*mesh_sub).skin_cluster_weights),
                );
            } else {
                weights = init_skin_weights(sc, (*mesh).num_vertices, skin);
            }

            subdivide_weights(
                sc,
                &mut (*result_sub).skin_cluster_ranges,
                &mut (*result_sub).skin_cluster_weights,
                weights,
            )?;
        }
    }

    (*result).num_vertices = (*result).vertex_position.values.count;
    (*result).num_indices = (*mesh).num_indices.wrapping_mul(4);
    (*result).num_faces = (*mesh).num_indices;
    (*result).num_triangles = (*mesh).num_indices.wrapping_mul(2);

    (*result).vertex_indices.data = (*result).vertex_position.indices.data;
    (*result).vertex_indices.count = (*result).num_indices;
    (*result).vertices.data = (*result).vertex_position.values.data;
    (*result).vertices.count = (*result).num_vertices;

    (*result).faces.count = (*result).num_faces;
    (*result).faces.data = push::<Face>(sc.result_mut(), (*result).num_faces);
    ufbxi_check_err!(
        sc.error_mut(),
        !(*result).faces.data.is_null(),
        "result->faces.data"
    );

    let mut i: usize = 0;
    while i < (*result).num_faces {
        (*((*result).faces.data as *mut Face).add(i)).index_begin = i.wrapping_mul(4) as u32;
        (*((*result).faces.data as *mut Face).add(i)).num_indices = 4;
        i += 1;
    }

    if !(*mesh).edges.data.is_null() {
        (*result).num_edges = (*mesh)
            .num_edges
            .wrapping_mul(2)
            .wrapping_add((*result).num_faces);
        (*result).edges.count = (*result).num_edges;
        (*result).edges.data = push::<Edge>(sc.result_mut(), (*result).num_edges);
        ufbxi_check_err!(
            sc.error_mut(),
            !(*result).edges.data.is_null(),
            "result->edges.data"
        );

        if !(*mesh).edge_crease.data.is_null() {
            (*result).edge_crease.count = (*result).num_edges;
            (*result).edge_crease.data = push::<Real>(sc.result_mut(), (*result).num_edges);
            ufbxi_check_err!(
                sc.error_mut(),
                !(*result).edge_crease.data.is_null(),
                "result->edge_crease.data"
            );
        }
        if !(*mesh).edge_smoothing.data.is_null() {
            (*result).edge_smoothing.count = (*result).num_edges;
            (*result).edge_smoothing.data = push::<bool>(sc.result_mut(), (*result).num_edges);
            ufbxi_check_err!(
                sc.error_mut(),
                !(*result).edge_smoothing.data.is_null(),
                "result->edge_smoothing.data"
            );
        }
        if !(*mesh).edge_visibility.data.is_null() {
            (*result).edge_visibility.count = (*result).num_edges;
            (*result).edge_visibility.data = push::<bool>(sc.result_mut(), (*result).num_edges);
            ufbxi_check_err!(
                sc.error_mut(),
                !(*result).edge_visibility.data.is_null(),
                "result->edge_visibility.data"
            );
        }

        let mut di: usize = 0;
        let mut i: usize = 0;
        while i < (*mesh).num_edges {
            let edge: Edge = *(*mesh).edges.data.add(i);
            let face_ix: u32 = (*topo.add(edge.a as usize)).face;
            let face: Face = *(*mesh).faces.data.add(face_ix as usize);
            let offset: u32 = edge.a.wrapping_sub(face.index_begin);
            let next: u32 = (offset.wrapping_add(1)) % face.num_indices;

            let a: u32 = (face.index_begin.wrapping_add(offset)).wrapping_mul(4);
            let b: u32 = (face.index_begin.wrapping_add(next)).wrapping_mul(4);

            (*((*result).edges.data as *mut Edge).add(di + 0)).a = a;
            (*((*result).edges.data as *mut Edge).add(di + 0)).b = a.wrapping_add(1);
            (*((*result).edges.data as *mut Edge).add(di + 1)).a = b.wrapping_add(3);
            (*((*result).edges.data as *mut Edge).add(di + 1)).b = b;

            if !(*mesh).edge_crease.data.is_null() {
                let mut crease: Real = *(*mesh).edge_crease.data.add(i);
                // C: `0.999f` is a `float` literal; `(ufbx_real)0.1` is not.
                if crease < 0.999f32 as Real {
                    crease -= 0.1 as Real;
                }
                if crease < 0.0 {
                    crease = 0.0;
                }
                *((*result).edge_crease.data as *mut Real).add(di + 0) = crease;
                *((*result).edge_crease.data as *mut Real).add(di + 1) = crease;
            }

            if !(*mesh).edge_smoothing.data.is_null() {
                *((*result).edge_smoothing.data as *mut bool).add(di + 0) =
                    *(*mesh).edge_smoothing.data.add(i);
                *((*result).edge_smoothing.data as *mut bool).add(di + 1) =
                    *(*mesh).edge_smoothing.data.add(i);
            }

            if !(*mesh).edge_visibility.data.is_null() {
                *((*result).edge_visibility.data as *mut bool).add(di + 0) =
                    *(*mesh).edge_visibility.data.add(i);
                *((*result).edge_visibility.data as *mut bool).add(di + 1) =
                    *(*mesh).edge_visibility.data.add(i);
            }

            di += 2;
            i += 1;
        }

        let mut fi: usize = 0;
        while fi < (*result).num_faces {
            (*((*result).edges.data as *mut Edge).add(di)).a =
                fi.wrapping_mul(4).wrapping_add(1) as u32;
            (*((*result).edges.data as *mut Edge).add(di)).b =
                fi.wrapping_mul(4).wrapping_add(2) as u32;

            if !(*result).edge_crease.data.is_null() {
                *((*result).edge_crease.data as *mut Real).add(di) = 0.0;
            }

            if !(*result).edge_smoothing.data.is_null() {
                *((*result).edge_smoothing.data as *mut bool).add(di + 0) = true;
            }

            if !(*result).edge_visibility.data.is_null() {
                *((*result).edge_visibility.data as *mut bool).add(di + 0) = false;
            }

            di += 1;
            fi += 1;
        }
    }

    if !(*mesh).face_material.data.is_null() {
        (*result).face_material.count = (*result).num_faces;
        (*result).face_material.data = push::<u32>(sc.result_mut(), (*result).num_faces);
        ufbxi_check_err!(
            sc.error_mut(),
            !(*result).face_material.data.is_null(),
            "result->face_material.data"
        );
    }
    if !(*mesh).face_smoothing.data.is_null() {
        (*result).face_smoothing.count = (*result).num_faces;
        (*result).face_smoothing.data = push::<bool>(sc.result_mut(), (*result).num_faces);
        ufbxi_check_err!(
            sc.error_mut(),
            !(*result).face_smoothing.data.is_null(),
            "result->face_smoothing.data"
        );
    }
    if !(*mesh).face_group.data.is_null() {
        (*result).face_group.count = (*result).num_faces;
        (*result).face_group.data = push::<u32>(sc.result_mut(), (*result).num_faces);
        ufbxi_check_err!(
            sc.error_mut(),
            !(*result).face_group.data.is_null(),
            "result->face_group.data"
        );
    }
    if !(*mesh).face_hole.data.is_null() {
        (*result).face_hole.count = (*result).num_faces;
        (*result).face_hole.data = push::<bool>(sc.result_mut(), (*result).num_faces);
        ufbxi_check_err!(
            sc.error_mut(),
            !(*result).face_hole.data.is_null(),
            "result->face_hole.data"
        );
    }

    if (*result).material_parts.count > 0 {
        (*result).material_parts.data =
            push_zero::<MeshPart>(sc.result_mut(), (*result).material_parts.count);
        // C-parity: upstream checks `result->materials.data` here
        // (ufbx.c:29882), not the freshly pushed `material_parts.data`.
        ufbxi_check_err!(
            sc.error_mut(),
            !(*result).materials.data.is_null(),
            "result->materials.data"
        );
    }

    let mut index_offset: usize = 0;
    let mut i: usize = 0;
    while i < (*mesh).num_faces {
        let face: Face = *(*mesh).faces.data.add(i);

        let mut mat: u32 = 0;
        if !(*mesh).face_material.data.is_null() {
            mat = *(*mesh).face_material.data.add(i);
            let mut ci: usize = 0;
            while ci < face.num_indices as usize {
                *((*result).face_material.data as *mut u32).add(index_offset.wrapping_add(ci)) =
                    mat;
                ci += 1;
            }
        }
        // C: `mat` is otherwise unused (assigned and read only in the branch).
        let _ = mat;
        if !(*mesh).face_smoothing.data.is_null() {
            let flag: bool = *(*mesh).face_smoothing.data.add(i);
            let mut ci: usize = 0;
            while ci < face.num_indices as usize {
                *((*result).face_smoothing.data as *mut bool).add(index_offset.wrapping_add(ci)) =
                    flag;
                ci += 1;
            }
        }
        if !(*mesh).face_group.data.is_null() {
            let group: u32 = *(*mesh).face_group.data.add(i);
            let mut ci: usize = 0;
            while ci < face.num_indices as usize {
                *((*result).face_group.data as *mut u32).add(index_offset.wrapping_add(ci)) = group;
                ci += 1;
            }
        }
        if !(*mesh).face_hole.data.is_null() {
            let flag: bool = *(*mesh).face_hole.data.add(i);
            let mut ci: usize = 0;
            while ci < face.num_indices as usize {
                *((*result).face_hole.data as *mut bool).add(index_offset.wrapping_add(ci)) = flag;
                ci += 1;
            }
        }
        index_offset = index_offset.wrapping_add(face.num_indices as usize);
        i += 1;
    }

    // Will be filled in by `ufbxi_finalize_mesh()`.
    (*result).vertex_first_index.count = 0;

    finalize_mesh_material(sc.result_mut(), sc.error_mut(), result)?;
    finalize_mesh(sc.result_mut(), sc.error_mut(), result)?;
    update_face_groups(sc.result_mut(), sc.error_mut(), result, true)?;

    Ok(())
}

// ufbx.c:29927-30034 `ufbxi_subdivide_mesh_imp`
#[cfg(feature = "subdivision")]
#[must_use]
#[inline(never)]
pub(crate) unsafe fn subdivide_mesh_imp(
    sc: &SubdivideContext,
    level: usize,
) -> Result<(), crate::native::error::Fail> {
    if (*sc.get()).opts.boundary as u32 == SubdivisionBoundary::Default as u32 {
        (*sc.get()).opts.boundary = (*sc.get()).src_mesh.subdivision_boundary;
    }

    if (*sc.get()).opts.uv_boundary as u32 == SubdivisionBoundary::Default as u32 {
        (*sc.get()).opts.uv_boundary = (*sc.get()).src_mesh.subdivision_uv_boundary;
    }

    init_ator(
        sc.error_mut(),
        sc.ator_tmp_mut(),
        &(*sc.get()).opts.temp_allocator,
        b"temp\0".as_ptr(),
    );
    init_ator(
        sc.error_mut(),
        sc.ator_result_mut(),
        &(*sc.get()).opts.result_allocator,
        b"result\0".as_ptr(),
    );

    (*sc.get()).result.unordered = true;
    (*sc.get()).source.unordered = true;
    (*sc.get()).tmp.unordered = true;

    (*sc.get()).source.ator = sc.ator_tmp_mut();
    (*sc.get()).tmp.ator = sc.ator_tmp_mut();

    let mut i: usize = 1;
    while i < level {
        (*sc.get()).result.ator = sc.ator_tmp_mut();

        subdivide_mesh_level(sc)?;

        // C: `sc->src_mesh = sc->dst_mesh;` — struct assignment (memcpy).
        core::ptr::copy_nonoverlapping(
            core::ptr::addr_of!((*sc.get()).dst_mesh),
            sc.src_mesh_mut(),
            1,
        );

        buf_free(sc.source_mut());
        buf_free(sc.tmp_mut());
        (*sc.get()).source = (*sc.get()).result;
        core::ptr::write_bytes(sc.result_mut() as *mut Buf as *mut u8, 0, size_of::<Buf>());
        i += 1;
    }

    (*sc.get()).result.ator = sc.ator_result_mut();
    subdivide_mesh_level(sc)?;
    buf_free(sc.tmp_mut());

    let mesh: *mut Mesh = sc.dst_mesh_mut();

    // Subdivision always results in a mesh that consists only of quads
    (*mesh).max_face_triangles = 2;
    (*mesh).num_empty_faces = 0;
    (*mesh).num_point_faces = 0;
    (*mesh).num_line_faces = 0;

    if !(*sc.get()).opts.interpolate_normals {
        core::ptr::write_bytes(
            &mut (*mesh).vertex_normal as *mut _ as *mut u8,
            0,
            size_of::<VertexVec3>(),
        );
        core::ptr::write_bytes(
            &mut (*mesh).skinned_normal as *mut _ as *mut u8,
            0,
            size_of::<VertexVec3>(),
        );
    }

    if !(*sc.get()).opts.interpolate_normals && !(*sc.get()).opts.ignore_normals {
        let topo: *mut TopoEdge = push::<TopoEdge>(sc.tmp_mut(), (*mesh).num_indices);
        ufbxi_check_err!(sc.error_mut(), !topo.is_null(), "topo");
        compute_topology(mesh, topo, (*mesh).num_indices);

        let normal_indices: *mut u32 = push::<u32>(sc.result_mut(), (*mesh).num_indices);
        ufbxi_check_err!(sc.error_mut(), !normal_indices.is_null(), "normal_indices");

        let num_normals: usize = generate_normal_mapping(
            mesh,
            topo,
            (*mesh).num_indices,
            normal_indices,
            (*mesh).num_indices,
            true,
        );
        if num_normals == (*mesh).num_vertices {
            (*mesh).skinned_normal.unique_per_vertex = true;
        }

        let mut normal_data: *mut Vec3 = push::<Vec3>(sc.result_mut(), num_normals.wrapping_add(1));
        ufbxi_check_err!(sc.error_mut(), !normal_data.is_null(), "normal_data");
        *normal_data.add(0) = ZERO_VEC3;
        normal_data = normal_data.add(1);

        compute_normals(
            mesh,
            &(*mesh).skinned_position,
            normal_indices,
            (*mesh).num_indices,
            normal_data,
            num_normals,
        );

        (*mesh).generated_normals = true;
        (*mesh).vertex_normal.exists = true;
        (*mesh).vertex_normal.values.data = normal_data;
        (*mesh).vertex_normal.values.count = num_normals;
        (*mesh).vertex_normal.indices.data = normal_indices;
        (*mesh).vertex_normal.indices.count = (*mesh).num_indices;

        core::ptr::copy_nonoverlapping(&(*mesh).vertex_normal, &mut (*mesh).skinned_normal, 1);
    }

    let parent: *mut Refcount;
    if (*sc.src_mesh_ptr()).subdivision_evaluated && (*sc.src_mesh_ptr()).from_tessellated_nurbs {
        let imp: *mut MeshImp = get_imp(sc.src_mesh_ptr() as *mut c_void);
        parent = &mut (*imp).refcount;
    } else {
        let imp: *mut SceneImp =
            get_imp(ref_ptr(&(*sc.src_mesh_ptr()).element.scene) as *mut c_void);
        parent = &mut (*imp).refcount;
    }

    patch_mesh_reals(mesh);

    sc.set_imp(push::<MeshImp>(sc.result_mut(), 1));
    ufbxi_check_err!(sc.error_mut(), !sc.imp().is_null(), "sc->imp");

    // Expose the wide allocation so `get_imp` can recover this header from a
    // (possibly narrowed) public `&Mesh` pointer via exposed provenance.
    (sc.imp() as *mut u8).expose_provenance();

    let dst_sub: *mut SubdivisionResult = opt_ptr(&(*sc.get()).dst_mesh.subdivision_result);
    (*dst_sub).result_memory_used = (*sc.get()).ator_result.current_size;
    (*dst_sub).temp_memory_used = (*sc.get()).ator_tmp.current_size;
    (*dst_sub).result_allocs = (*sc.get()).ator_result.num_allocs;
    (*dst_sub).temp_allocs = (*sc.get()).ator_tmp.num_allocs;

    init_ref(&mut (*sc.imp()).refcount, MESH_IMP_MAGIC, parent);

    (*sc.imp()).magic = MESH_IMP_MAGIC;
    // C: `sc->imp->mesh = sc->dst_mesh;` — struct assignment (memcpy).
    core::ptr::copy_nonoverlapping(
        core::ptr::addr_of!((*sc.get()).dst_mesh),
        core::ptr::addr_of_mut!((*sc.imp()).mesh),
        1,
    );
    (*sc.imp()).refcount.ator = (*sc.get()).ator_result;
    (*sc.imp()).refcount.buf = (*sc.get()).result;
    (*sc.imp()).mesh.subdivision_evaluated = true;

    Ok(())
}

// ufbx.c:30036-30067 `ufbxi_subdivide_mesh`
#[cfg(feature = "subdivision")]
#[inline(never)]
pub(crate) unsafe fn subdivide_mesh(
    mesh: *const Mesh,
    level: usize,
    user_opts: *const RawSubdivideOpts,
    p_error: *mut Error,
) -> *mut Mesh {
    // C: `ufbxi_subdivide_context sc = { 0 };`
    // C: `ufbxi_subdivide_context sc = { 0 };`
    let sc = SubdivideContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::zeroed()));
    let sc = &sc;
    if !user_opts.is_null() {
        // C: `(*sc.get()).opts = *user_opts;` — struct assignment (memcpy).
        core::ptr::copy_nonoverlapping(user_opts, sc.opts_mut(), 1);
    }

    sc.set_src_mesh_ptr(mesh as *mut Mesh);
    // C: `(*sc.get()).src_mesh = *mesh;` — struct assignment (memcpy).
    core::ptr::copy_nonoverlapping(mesh, sc.src_mesh_mut(), 1);

    let ok: bool = subdivide_mesh_imp(sc, level).is_ok();

    free::<SubdivideInput>(sc.ator_tmp_mut(), sc.inputs(), sc.inputs_cap());
    buf_free(sc.tmp_mut());
    buf_free(sc.source_mut());

    if ok {
        free_ator(sc.ator_tmp_mut());
        if !p_error.is_null() {
            clear_error(p_error);
        }

        let imp: *mut MeshImp = sc.imp();
        core::ptr::addr_of_mut!((*imp).mesh)
    } else {
        fix_error_type(sc.error_mut(), b"Failed to subdivide\0".as_ptr(), p_error);
        buf_free(sc.result_mut());
        free_ator(sc.ator_tmp_mut());
        free_ator(sc.ator_result_mut());
        core::ptr::null_mut()
    }
}

// ufbx.c:30071-30079 `ufbxi_subdivide_mesh` (`#else` — feature disabled)
#[cfg(not(feature = "subdivision"))]
#[inline(never)]
pub(crate) unsafe fn subdivide_mesh(
    mesh: *const Mesh,
    level: usize,
    user_opts: *const RawSubdivideOpts,
    p_error: *mut Error,
) -> *mut Mesh {
    // C: `mesh`/`level`/`user_opts` are unreferenced in the `#else` arm.
    let _ = (mesh, level, user_opts);
    if !p_error.is_null() {
        core::ptr::write_bytes(p_error as *mut u8, 0, core::mem::size_of::<Error>());
        ufbxi_fmt_err_info!(p_error, "UFBX_ENABLE_SUBDIVISION");
        ufbxi_report_err_msg!(p_error, "UFBXI_FEATURE_SUBDIVISION", "Feature disabled");
    }
    core::ptr::null_mut()
}
