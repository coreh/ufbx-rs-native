//! Port of the `// -- Topology` banner section (ufbx.c:28243-28819).
//!
//! Three C blocks live here:
//!   * `#if UFBXI_FEATURE_KD` (ufbx.c:28245-28470) — the ngon context, the
//!     KD-tree node/triangle types, the 2D projection + orientation helpers and
//!     the KD build/query pair.
//!   * `#if UFBXI_FEATURE_TRIANGULATION` (ufbx.c:28472-28690) — the ear-clipping
//!     ngon triangulator and its triangle-quality weight.
//!   * ungated (ufbx.c:28692-28819) — the half-edge topology builder
//!     (`ufbxi_compute_topology`) with its two comparators, and the smooth-edge
//!     predicate `ufbxi_is_edge_smooth`.
//!
//! `UFBXI_FEATURE_KD` is DERIVED from `UFBXI_FEATURE_TRIANGULATION`
//! (ufbx.c:182-186), so both gated blocks carry `#[cfg(feature =
//! "triangulation")]` — there is no separate `kd` cargo feature.
//!
//! The public entry points that drive this section
//! (`ufbx_catch_triangulate_face`, `ufbx_catch_compute_topology`,
//! `ufbx_catch_generate_normal_mapping`) live in `native/api.rs` with the rest
//! of the API surface.
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]

use crate::generated::{Edge, Face, Mesh, TopoEdge, TopoFlags, Vec3};
#[cfg(feature = "triangulation")]
use crate::generated::{Vec2, VertexVec3};
use crate::native::api::get_vertex_vec3;
use crate::native::platform::{macro_lower_bound_eq, ufbx_assert, unstable_sort, NO_INDEX};
#[cfg(feature = "triangulation")]
use crate::native::platform::{math, max_real, min_real, stable_sort, ufbxi_ignore, KD_FAST_DEPTH};
#[cfg(feature = "triangulation")]
use crate::native::string_pool::{distsq2, dot3, length3, mul3, slow_normalized_cross3};
#[cfg(feature = "triangulation")]
use crate::prelude::Real;
use core::ffi::c_void;
use core::mem::size_of;

// -- KD tree (ufbx.c:28245-28470, `#if UFBXI_FEATURE_KD`)

// ufbx.c:28247-28253 `ufbxi_kd_node`
#[cfg(feature = "triangulation")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct KdNode {
    pub(crate) split: Real,
    pub(crate) index_plus_one: u32, // 0 for empty
    pub(crate) slow_left: u32,
    pub(crate) slow_right: u32,
    pub(crate) slow_end: u32,
}

// ufbx.c:28259 `ufbxi_kd_node kd_nodes[1 << (UFBXI_KD_FAST_DEPTH + 1)]`
#[cfg(feature = "triangulation")]
pub(crate) const KD_NODES_LEN: usize = 1usize << (KD_FAST_DEPTH + 1);

// ufbx.c:28255-28265 `ufbxi_ngon_context`
#[cfg(feature = "triangulation")]
#[repr(C)]
pub(crate) struct InnerNgonContext {
    pub(crate) face: Face,
    pub(crate) positions: VertexVec3,
    pub(crate) axes: [Vec3; 3],
    pub(crate) kd_nodes: [KdNode; KD_NODES_LEN],
    pub(crate) kd_indices: *mut u32,

    // Temporary
    pub(crate) cur_axis_dir: Vec3,
    pub(crate) cur_face: Face,
}

// Safe `&NgonContext` handle over the fields-struct `InnerNgonContext`, mirroring
// the `Context`/`InnerContext` seam in `parse.rs`. `MaybeUninit` keeps it uniform
// with the other context wrappers; `UnsafeCell` gives the interior mutability every
// `&NgonContext` site needs. The type-erased KD-query callback pointer round-trips
// through the wrapper address. Field is `pub(crate)` — the sole construction site
// lives in `native::api`.
#[cfg(feature = "triangulation")]
#[repr(transparent)]
pub(crate) struct NgonContext(
    pub(crate) core::cell::UnsafeCell<core::mem::MaybeUninit<InnerNgonContext>>,
);

// Typed interior-mutable VIEW over a `Face` field, reinterpreted in place.
#[cfg(feature = "triangulation")]
#[repr(transparent)]
pub(crate) struct FaceView(core::cell::UnsafeCell<core::mem::MaybeUninit<Face>>);

#[cfg(feature = "triangulation")]
impl FaceView {
    #[inline(always)]
    fn get(&self) -> *mut Face {
        self.0.get().cast()
    }
    #[inline(always)]
    pub(crate) fn index_begin(&self) -> u32 {
        unsafe { (*self.get()).index_begin }
    }
}

#[cfg(feature = "triangulation")]
impl NgonContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerNgonContext {
        self.0.get().cast()
    }

    // `face`/`cur_face` (Face) — typed VIEW handles (reinterpret-in-place).
    #[inline(always)]
    pub(crate) fn face_view(&self) -> &FaceView {
        unsafe { &*(&raw mut (*self.get()).face as *mut FaceView) }
    }
    #[inline(always)]
    pub(crate) fn cur_face_view(&self) -> &FaceView {
        unsafe { &*(&raw mut (*self.get()).cur_face as *mut FaceView) }
    }
    // `positions` (VertexVec3) — typed VIEW handle; reuse the shared VertexVec3View.
    #[inline(always)]
    pub(crate) fn positions_view(&self) -> &crate::native::subdivision::VertexVec3View {
        unsafe {
            &*(&raw mut (*self.get()).positions as *mut crate::native::subdivision::VertexVec3View)
        }
    }
    // `face`/`cur_face` (Copy `Face`) / `cur_axis_dir` (Copy `Vec3`) — value getter/setter.
    #[inline(always)]
    pub(crate) fn face(&self) -> Face {
        unsafe { (*self.get()).face }
    }
    #[inline(always)]
    pub(crate) fn set_face(&self, face: Face) {
        unsafe {
            (*self.get()).face = face;
        }
    }
    #[inline(always)]
    pub(crate) fn set_cur_face(&self, cur_face: Face) {
        unsafe {
            (*self.get()).cur_face = cur_face;
        }
    }
    #[inline(always)]
    pub(crate) fn cur_axis_dir(&self) -> Vec3 {
        unsafe { (*self.get()).cur_axis_dir }
    }
    #[inline(always)]
    pub(crate) fn set_cur_axis_dir(&self, cur_axis_dir: Vec3) {
        unsafe {
            (*self.get()).cur_axis_dir = cur_axis_dir;
        }
    }
    // `axes` ([Vec3; 3]) / `kd_nodes` ([KdNode; N]) — per-element `ScalarView` (Cell)
    // handles: `.get()`/`.set()` for the Copy elements, `.as_ptr()` for addr-of sites.
    #[inline(always)]
    pub(crate) fn axes_at(&self, i: usize) -> &crate::prelude::ScalarView<Vec3> {
        unsafe { &*(&raw mut (*self.get()).axes[i] as *mut crate::prelude::ScalarView<Vec3>) }
    }
    #[inline(always)]
    pub(crate) fn kd_nodes_at(&self, i: usize) -> &crate::prelude::ScalarView<KdNode> {
        unsafe { &*(&raw mut (*self.get()).kd_nodes[i] as *mut crate::prelude::ScalarView<KdNode>) }
    }

    // `positions` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn positions_mut_ptr(&self) -> *mut VertexVec3 {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).positions }
    }

    // `kd_indices` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn kd_indices(&self) -> *mut u32 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u32` are valid.
        unsafe { (*self.get()).kd_indices }
    }

    #[inline(always)]
    pub(crate) fn set_kd_indices(&self, kd_indices: *mut u32) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).kd_indices = kd_indices;
        }
    }
}

// ufbx.c:28267-28272 `ufbxi_kd_triangle`
#[cfg(feature = "triangulation")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct KdTriangle {
    pub(crate) min_t: [Real; 2],
    pub(crate) max_t: [Real; 2],
    pub(crate) points: [Vec2; 3],
    pub(crate) indices: [u32; 3],
}

// ufbx.c:28274-28282 `ufbxi_ngon_project`
// C: `ufbxi_noinline static`.
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn ngon_project(nc: &NgonContext, index: u32) -> Vec2 {
    let point: Vec3 = *nc.positions_view().values_view().data().add(
        *nc.positions_view()
            .indices_view()
            .data()
            .add(nc.face_view().index_begin().wrapping_add(index) as usize) as usize,
    );

    // C: `ufbx_vec2 p;` — both fields are assigned below.
    let mut p: Vec2 = core::mem::zeroed();
    p.x = dot3(nc.axes_at(0).get(), point);
    p.y = dot3(nc.axes_at(1).get(), point);
    p
}

// ufbx.c:28284-28287 `ufbxi_orient2d`
// C: `ufbxi_forceinline static`.
#[cfg(feature = "triangulation")]
#[inline(always)]
pub(crate) fn orient2d(a: Vec2, b: Vec2, c: Vec2) -> Real {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

// ufbx.c:28289-28301 `ufbxi_kd_check_point`
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn kd_check_point(nc: &NgonContext, tri: *const KdTriangle, index: u32) -> bool {
    if index == (*tri).indices[0] || index == (*tri).indices[1] || index == (*tri).indices[2] {
        return false;
    }
    let p: Vec2 = ngon_project(nc, index);

    let u: Real = orient2d(p, (*tri).points[0], (*tri).points[1]);
    let v: Real = orient2d(p, (*tri).points[1], (*tri).points[2]);
    let w: Real = orient2d(p, (*tri).points[2], (*tri).points[0]);

    if u <= 0.0 && v <= 0.0 && w <= 0.0 {
        return true;
    }
    if u >= 0.0 && v >= 0.0 && w >= 0.0 {
        return true;
    }
    false
}

// Recursion limited by 32-bit indices in input, minus halvings from `ufbxi_kd_check_fast()`
// ufbx.c:28303-28342 `ufbxi_kd_check_slow`
// `ufbxi_recursive_function(bool, ufbxi_kd_check_slow, ..., 32 -
// UFBXI_KD_FAST_DEPTH, ...)` (ufbx.c:28305-28306): under regression, a
// thread-local depth guard wraps the recursive body (which C splits into
// `ufbxi_kd_check_slow_rec`); otherwise the macro is empty and the wrapper is a
// plain call.
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn kd_check_slow(
    nc: &NgonContext,
    tri: *const KdTriangle,
    begin: u32,
    count: u32,
    axis: u32,
) -> bool {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|depth| {
            ufbx_assert!(depth.get() < (32 - KD_FAST_DEPTH) as u32);
            depth.set(depth.get() + 1);
        });
        let ret = kd_check_slow_rec(nc, tri, begin, count, axis);
        UFBXI_RECURSION_DEPTH.with(|depth| depth.set(depth.get() - 1));
        ret
    }
    #[cfg(not(feature = "regression"))]
    {
        kd_check_slow_rec(nc, tri, begin, count, axis)
    }
}

// The recursive body (attached directly to `ufbxi_kd_check_slow` in
// non-regression C builds; recursive calls go through the guarded wrapper).
#[cfg(feature = "triangulation")]
unsafe fn kd_check_slow_rec(
    nc: &NgonContext,
    tri: *const KdTriangle,
    begin: u32,
    count: u32,
    axis: u32,
) -> bool {
    let mut begin = begin;
    let mut count = count;
    let mut axis = axis;

    // C: `ufbx_vertex_vec3 pos = nc->positions;` — a struct memcpy.
    let pos: VertexVec3 = core::ptr::read(nc.positions_mut_ptr());
    let kd_indices: *mut u32 = nc.kd_indices();

    while count > 0 {
        let num_left: u32 = count / 2;
        let begin_right: u32 = begin.wrapping_add(num_left).wrapping_add(1);
        let num_right: u32 = count.wrapping_sub(num_left.wrapping_add(1));

        let index: u32 = *kd_indices.add(begin.wrapping_add(num_left) as usize);
        let point: Vec3 = *pos.values.data.add(
            *pos.indices
                .data
                .add(nc.face_view().index_begin().wrapping_add(index) as usize)
                as usize,
        );
        let split: Real = dot3(point, nc.axes_at(axis as usize).get());
        let hit_left: bool = (*tri).min_t[axis as usize] <= split;
        let hit_right: bool = (*tri).max_t[axis as usize] >= split;

        if hit_left && hit_right {
            if kd_check_point(nc, tri, index) {
                return true;
            }

            if kd_check_slow(nc, tri, begin_right, num_right, axis ^ 1) {
                return true;
            }
        }

        axis ^= 1;
        if hit_left {
            count = num_left;
        } else {
            begin = begin_right;
            count = num_right;
        }
    }

    false
}

// Recursion limited by `UFBXI_KD_FAST_DEPTH`
// ufbx.c:28344-28390 `ufbxi_kd_check_fast`
// `ufbxi_recursive_function(bool, ufbxi_kd_check_fast, ...,
// UFBXI_KD_FAST_DEPTH, ...)` (ufbx.c:28346-28347) — same guard split as
// `ufbxi_kd_check_slow` above.
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn kd_check_fast(
    nc: &NgonContext,
    tri: *const KdTriangle,
    kd_index: u32,
    axis: u32,
    depth: u32,
) -> bool {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|d| {
            ufbx_assert!(d.get() < KD_FAST_DEPTH as u32);
            d.set(d.get() + 1);
        });
        let ret = kd_check_fast_rec(nc, tri, kd_index, axis, depth);
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        ret
    }
    #[cfg(not(feature = "regression"))]
    {
        kd_check_fast_rec(nc, tri, kd_index, axis, depth)
    }
}

// The recursive body (see `kd_check_slow_rec`).
#[cfg(feature = "triangulation")]
unsafe fn kd_check_fast_rec(
    nc: &NgonContext,
    tri: *const KdTriangle,
    kd_index: u32,
    axis: u32,
    depth: u32,
) -> bool {
    let mut kd_index = kd_index;
    let mut axis = axis;
    let mut depth = depth;

    loop {
        let node: KdNode = nc.kd_nodes_at(kd_index as usize).get();
        if node.index_plus_one == 0 {
            return false;
        }

        let hit_left: bool = (*tri).min_t[axis as usize] <= node.split;
        let hit_right: bool = (*tri).max_t[axis as usize] >= node.split;

        let side: u32 = if hit_left { 0 } else { 1 };
        let child_kd_index: u32 = kd_index.wrapping_mul(2).wrapping_add(1).wrapping_add(side);
        if hit_left && hit_right {
            // Check for the point on the split plane
            let index: u32 = node.index_plus_one.wrapping_sub(1);
            if kd_check_point(nc, tri, index) {
                return true;
            }

            // Recurse always to the right if we hit both sides
            if depth.wrapping_add(1) == KD_FAST_DEPTH as u32 {
                if kd_check_slow(
                    nc,
                    tri,
                    node.slow_right,
                    node.slow_end.wrapping_sub(node.slow_right),
                    axis ^ 1,
                ) {
                    return true;
                }
            } else {
                if kd_check_fast(
                    nc,
                    tri,
                    child_kd_index.wrapping_add(1),
                    axis ^ 1,
                    depth.wrapping_add(1),
                ) {
                    return true;
                }
            }
        }

        depth = depth.wrapping_add(1);
        axis ^= 1;
        kd_index = child_kd_index;

        if depth == KD_FAST_DEPTH as u32 {
            if hit_left {
                return kd_check_slow(
                    nc,
                    tri,
                    node.slow_left,
                    node.slow_right.wrapping_sub(node.slow_left),
                    axis,
                );
            } else {
                return kd_check_slow(
                    nc,
                    tri,
                    node.slow_right,
                    node.slow_end.wrapping_sub(node.slow_right),
                    axis,
                );
            }
        }
    }
}

// ufbx.c:28392-28406 `ufbxi_kd_check`
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn kd_check(nc: &NgonContext, points: *const Vec2, indices: *const u32) -> bool {
    let mut tri: KdTriangle = core::mem::zeroed(); // ufbxi_uninit
    tri.points[0] = *points.add(0);
    tri.points[1] = *points.add(1);
    tri.points[2] = *points.add(2);
    tri.indices[0] = *indices.add(0);
    tri.indices[1] = *indices.add(1);
    tri.indices[2] = *indices.add(2);
    tri.min_t[0] = min_real(
        min_real((*points.add(0)).x, (*points.add(1)).x),
        (*points.add(2)).x,
    );
    tri.min_t[1] = min_real(
        min_real((*points.add(0)).y, (*points.add(1)).y),
        (*points.add(2)).y,
    );
    tri.max_t[0] = max_real(
        max_real((*points.add(0)).x, (*points.add(1)).x),
        (*points.add(2)).x,
    );
    tri.max_t[1] = max_real(
        max_real((*points.add(0)).y, (*points.add(1)).y),
        (*points.add(2)).y,
    );
    kd_check_fast(nc, &tri, 0, 0, 0)
}

// ufbx.c:28408-28416 `ufbxi_kd_index_less` (an `ufbxi_less_fn`)
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe extern "C" fn kd_index_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    let nc: &NgonContext = &*(user as *const NgonContext);
    let pos: *mut VertexVec3 = nc.positions_mut_ptr();
    let a: u32 = *(va as *const u32);
    let b: u32 = *(vb as *const u32);
    let da: Real = dot3(
        nc.cur_axis_dir(),
        *(*pos).values.data.add(
            *(*pos)
                .indices
                .data
                .add(nc.cur_face_view().index_begin().wrapping_add(a) as usize)
                as usize,
        ),
    );
    let db: Real = dot3(
        nc.cur_axis_dir(),
        *(*pos).values.data.add(
            *(*pos)
                .indices
                .data
                .add(nc.cur_face_view().index_begin().wrapping_add(b) as usize)
                as usize,
        ),
    );
    da < db
}

// Recursion limited by 32-bit indices in input
// ufbx.c:28418-28468 `ufbxi_kd_build`
// `ufbxi_recursive_function_void(ufbxi_kd_build, ..., 32, ...)`
// (ufbx.c:28420-28421) — same guard split as `ufbxi_kd_check_slow` above.
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn kd_build(
    nc: &NgonContext,
    indices: *mut u32,
    tmp: *mut u32,
    num: u32,
    axis: u32,
    fast_index: u32,
    depth: u32,
) {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|d| {
            ufbx_assert!(d.get() < 32);
            d.set(d.get() + 1);
        });
        kd_build_rec(nc, indices, tmp, num, axis, fast_index, depth);
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
    }
    #[cfg(not(feature = "regression"))]
    {
        kd_build_rec(nc, indices, tmp, num, axis, fast_index, depth);
    }
}

// The recursive body (see `kd_check_slow_rec`).
#[cfg(feature = "triangulation")]
unsafe fn kd_build_rec(
    nc: &NgonContext,
    indices: *mut u32,
    tmp: *mut u32,
    num: u32,
    axis: u32,
    fast_index: u32,
    depth: u32,
) {
    if num == 0 {
        return;
    }

    // C: `ufbx_vertex_vec3 pos = nc->positions;` — a struct memcpy.
    let pos: VertexVec3 = core::ptr::read(nc.positions_mut_ptr());
    let axis_dir: Vec3 = nc.axes_at(axis as usize).get();
    let face: Face = nc.face();

    nc.set_cur_axis_dir(axis_dir);
    nc.set_cur_face(face);

    // Sort the remaining indices based on the axis
    stable_sort(
        size_of::<u32>(),
        16,
        indices as *mut c_void,
        tmp as *mut c_void,
        num as usize,
        kd_index_less,
        (nc as *const NgonContext) as *mut c_void,
    );

    let num_left: u32 = num / 2;
    let begin_right: u32 = num_left.wrapping_add(1);
    let num_right: u32 = num.wrapping_sub(begin_right);
    let mut dst_right: u32 = num_left.wrapping_add(1);
    if depth < KD_FAST_DEPTH as u32 {
        let skip_left: u32 = 1u32 << (KD_FAST_DEPTH as u32).wrapping_sub(depth).wrapping_sub(1);
        dst_right = if dst_right > skip_left {
            dst_right.wrapping_sub(skip_left)
        } else {
            0
        };

        let index: u32 = *indices.add(num_left as usize);
        let kd: *mut KdNode = nc.kd_nodes_at(fast_index as usize).as_ptr();

        (*kd).split = dot3(
            axis_dir,
            *pos.values.data.add(
                *pos.indices
                    .data
                    .add(face.index_begin.wrapping_add(index) as usize) as usize,
            ),
        );
        (*kd).index_plus_one = index.wrapping_add(1);

        if depth.wrapping_add(1) == KD_FAST_DEPTH as u32 {
            (*kd).slow_left = indices.offset_from(nc.kd_indices()) as u32;
            (*kd).slow_right = (*kd).slow_left.wrapping_add(num_left);
            (*kd).slow_end = (*kd).slow_right.wrapping_add(num_right);
        } else {
            (*kd).slow_left = u32::MAX;
            (*kd).slow_right = u32::MAX;
            (*kd).slow_end = u32::MAX;
        }
    }

    let child_fast: u32 = fast_index.wrapping_mul(2).wrapping_add(1);
    kd_build(
        nc,
        indices,
        tmp,
        num_left,
        axis ^ 1,
        child_fast.wrapping_add(0),
        depth.wrapping_add(1),
    );

    if dst_right != begin_right {
        // C: `memmove(indices + dst_right, indices + begin_right, num_right * sizeof(uint32_t));`
        core::ptr::copy(
            indices.add(begin_right as usize),
            indices.add(dst_right as usize),
            num_right as usize,
        );
    }

    kd_build(
        nc,
        indices.add(dst_right as usize),
        tmp,
        num_right,
        axis ^ 1,
        child_fast.wrapping_add(1),
        depth.wrapping_add(1),
    );
}

// -- Triangulation (ufbx.c:28472-28690, `#if UFBXI_FEATURE_TRIANGULATION`)

// ufbx.c:28474-28487 `ufbxi_ngon_tri_weight`
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn ngon_tri_weight(points: *const Vec2) -> Real {
    let p0: Vec2 = *points.add(0);
    let p1: Vec2 = *points.add(1);
    let p2: Vec2 = *points.add(2);
    let orient: Real = orient2d(p0, p1, p2);
    if orient <= 0.0 {
        return -1.0;
    }

    let a: Real = distsq2(p0, p1);
    let b: Real = distsq2(p1, p2);
    let c: Real = distsq2(p2, p0);
    // C: `4.0f * a * b` is `ufbx_real` arithmetic (the `float` literal promotes to
    // `ufbx_real`); only the `ufbx_sqrt()` call widens to `double`, and the result is
    // cast straight back with `(ufbx_real)`.
    let ab: Real = (a + b - c) / math::sqrt((4.0 * a * b) as f64) as Real;
    let bc: Real = (b + c - a) / math::sqrt((4.0 * b * c) as f64) as Real;
    let ca: Real = (c + a - b) / math::sqrt((4.0 * c * a) as f64) as Real;
    // C: `ufbx_fmax()` takes `double`, so `ab`/`bc`/`ca` and `UFBX_EPSILON` promote and
    // `2.0f - ufbx_fmax(...)` computes in `double`; the `(ufbx_real)` cast narrows.
    math::fmax(
        math::EPSILON as f64,
        2.0 - math::fmax(math::fmax(ab as f64, bc as f64), ca as f64),
    ) as Real
}

// ufbx.c:28489-28688 `ufbxi_triangulate_ngon`
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn triangulate_ngon(
    nc: &NgonContext,
    indices: *mut u32,
    num_indices: u32,
) -> u32 {
    let face: Face = nc.face();
    ufbx_assert!(face.num_indices > 4);

    // Form an orthonormal basis to project the polygon into a 2D plane
    let mut normal: Vec3 =
        crate::native::api::get_weighted_face_normal(nc.positions_mut_ptr(), face);
    let len: Real = length3(normal);
    if len > math::EPSILON {
        normal = mul3(normal, 1.0 / len);
    } else {
        normal.x = 1.0;
        normal.y = 0.0;
        normal.z = 0.0;
    }

    let mut axis: Vec3 = core::mem::zeroed(); // ufbxi_uninit
    if normal.x * normal.x < 0.5 {
        axis.x = 1.0;
        axis.y = 0.0;
        axis.z = 0.0;
    } else {
        axis.x = 0.0;
        axis.y = 1.0;
        axis.z = 0.0;
    }
    nc.axes_at(0).set(slow_normalized_cross3(&axis, &normal));
    nc.axes_at(1)
        .set(slow_normalized_cross3(&normal, nc.axes_at(0).as_ptr()));
    nc.axes_at(2).set(normal);

    let kd_indices: *mut u32 = indices;
    nc.set_kd_indices(kd_indices);

    let kd_tmp: *mut u32 = indices.add(face.num_indices as usize);

    // Collect all the reflex corners for intersection testing.
    let mut num_kd_indices: u32 = 0;
    {
        let mut a: Vec2 = ngon_project(nc, face.num_indices.wrapping_sub(1));
        let mut b: Vec2 = ngon_project(nc, 0);
        let mut i: u32 = 0;
        while i < face.num_indices {
            let next: u32 = if i.wrapping_add(1) < face.num_indices {
                i.wrapping_add(1)
            } else {
                0
            };
            let c: Vec2 = ngon_project(nc, next);

            if orient2d(a, b, c) <= 0.0 {
                // C: `kd_indices[num_kd_indices++] = i;`
                *kd_indices.add(num_kd_indices as usize) = i;
                num_kd_indices = num_kd_indices.wrapping_add(1);
            }

            a = b;
            b = c;
            i = i.wrapping_add(1);
        }
    }

    // Build a KD-tree of the vertices.
    let num_skip_indices: u32 = (1u32 << (KD_FAST_DEPTH + 1)).wrapping_sub(1);
    let kd_slow_indices: u32 = if num_kd_indices > num_skip_indices {
        num_kd_indices.wrapping_sub(num_skip_indices)
    } else {
        0
    };
    ufbxi_ignore!(kd_slow_indices);
    ufbx_assert!(kd_slow_indices.wrapping_add(face.num_indices.wrapping_mul(2)) <= num_indices);
    kd_build(nc, kd_indices, kd_tmp, num_kd_indices, 0, 0, 0);

    // C: `uint32_t *edges = indices + num_indices - face.num_indices * 2;`
    let edges: *mut u32 = indices
        .add(num_indices as usize)
        .sub(face.num_indices.wrapping_mul(2) as usize);

    // Initialize `edges` to be a connectivity structure where:
    //  `edges[2*i + 0]` is the previous vertex of `i`
    //  `edges[2*i + 1]` is the next vertex of `i`
    // When clipped we mark indices with the high bit (0x80000000)
    {
        let mut i: u32 = 0;
        while i < face.num_indices {
            *edges.add(i.wrapping_mul(2).wrapping_add(0) as usize) = if i > 0 {
                i.wrapping_sub(1)
            } else {
                face.num_indices.wrapping_sub(1)
            };
            *edges.add(i.wrapping_mul(2).wrapping_add(1) as usize) =
                if i.wrapping_add(1) < face.num_indices {
                    i.wrapping_add(1)
                } else {
                    0
                };
            i = i.wrapping_add(1);
        }
    }

    // Core of the ear clipping algorithm.
    // Iterate through the polygon corners looking for potential ears satisfying:
    //   - Angle must be less than 180deg
    //   - The triangle formed by the two edges must be contained within the polygon
    // As these properties change only locally between modifications we only need
    // to iterate the polygon once if we move backwards one step every time we clip an ear.
    let mut indices_left: u32 = face.num_indices;
    {
        let mut point_indices: [u32; 4] = [0, 1, 2, 3];
        let mut weights: [Real; 2] = core::mem::zeroed(); // ufbxi_uninit
        let mut points: [Vec2; 4] = core::mem::zeroed(); // ufbxi_uninit

        let mut num_steps: u32 = 0;
        while indices_left > 3 {
            points[0] = ngon_project(nc, point_indices[0]);
            points[1] = ngon_project(nc, point_indices[1]);
            points[2] = ngon_project(nc, point_indices[2]);
            points[3] = ngon_project(nc, point_indices[3]);

            weights[0] = ngon_tri_weight(points.as_ptr().add(0));
            weights[1] = ngon_tri_weight(points.as_ptr().add(1));

            let first_side: u32 = if weights[1] > weights[0] { 1 } else { 0 };
            let mut clipped: bool = false;
            // C: `ufbxi_nounroll for (uint32_t side_ix = 0; side_ix < 2; side_ix++)`
            // — the no-unroll pragma is optimizer-only and has no Rust analogue.
            let mut side_ix: u32 = 0;
            while side_ix < 2 {
                let side: u32 = side_ix ^ first_side;
                if !(weights[side as usize] >= 0.0) {
                    break;
                }

                // If there is no reflex angle contained within the triangle formed
                // by `{ a, b, c }` connect the vertices `a - c` (prev, next) directly.
                if !kd_check(
                    nc,
                    points.as_ptr().add(side as usize),
                    point_indices.as_ptr().add(side as usize),
                ) {
                    let ia: u32 = point_indices[side.wrapping_add(0) as usize];
                    let ib: u32 = point_indices[side.wrapping_add(1) as usize];
                    let ic: u32 = point_indices[side.wrapping_add(2) as usize];

                    // Mark as clipped
                    *edges.add(ib.wrapping_mul(2).wrapping_add(0) as usize) |= 0x80000000;
                    *edges.add(ib.wrapping_mul(2).wrapping_add(1) as usize) |= 0x80000000;

                    *edges.add(ic.wrapping_mul(2).wrapping_add(0) as usize) = ia;
                    *edges.add(ia.wrapping_mul(2).wrapping_add(1) as usize) = ic;

                    indices_left = indices_left.wrapping_sub(1);

                    // TODO: This may cause O(n^2) behavior!
                    num_steps = 0;

                    if side == 1 {
                        point_indices[2] = point_indices[3];
                        point_indices[3] =
                            *edges.add(point_indices[3].wrapping_mul(2).wrapping_add(1) as usize);
                    } else {
                        point_indices[1] = point_indices[0];
                        point_indices[0] =
                            *edges.add(point_indices[0].wrapping_mul(2).wrapping_add(0) as usize);
                    }

                    clipped = true;
                    break;
                }
                side_ix = side_ix.wrapping_add(1);
            }
            if clipped {
                continue;
            }

            // Continue forward
            point_indices[0] = point_indices[1];
            point_indices[1] = point_indices[2];
            point_indices[2] = point_indices[3];
            point_indices[3] =
                *edges.add(point_indices[3].wrapping_mul(2).wrapping_add(1) as usize);
            num_steps = num_steps.wrapping_add(1);

            // If we have walked around the entire polygon it is irregular and
            // ear cutting won't find any more triangles.
            // TODO: This could be stricter?
            if num_steps >= face.num_indices.wrapping_mul(2) {
                break;
            }
        }

        // Fallback: Cut non-ears until the polygon is completed.
        // TODO: Could do something better here..
        let mut ix: u32 = point_indices[1];
        while indices_left > 3 {
            let prev: u32 = *edges.add(ix.wrapping_mul(2).wrapping_add(0) as usize);
            let next: u32 = *edges.add(ix.wrapping_mul(2).wrapping_add(1) as usize);

            // Mark as clipped
            *edges.add(ix.wrapping_mul(2).wrapping_add(0) as usize) |= 0x80000000;
            *edges.add(ix.wrapping_mul(2).wrapping_add(1) as usize) |= 0x80000000;

            *edges.add(prev.wrapping_mul(2).wrapping_add(1) as usize) = next;
            *edges.add(next.wrapping_mul(2).wrapping_add(0) as usize) = prev;

            indices_left = indices_left.wrapping_sub(1);
            ix = next;
        }

        // Now we have a single triangle left at `ix`.
        *edges.add(ix.wrapping_mul(2).wrapping_add(0) as usize) |= 0x80000000;
        *edges.add(ix.wrapping_mul(2).wrapping_add(1) as usize) |= 0x80000000;
    }

    // Expand the adjacency information `edges` into proper triangles.
    // Care needs to be taken here as both refer to the same memory area:
    // The last 4 triangles may overlap in source and destination so we write
    // them to a stack buffer and copy them over in the end.
    let max_triangles: u32 = face.num_indices.wrapping_sub(2);
    let mut num_triangles: u32 = 0;
    let mut num_last_triangles: u32 = 0;
    let mut last_triangles: [u32; 4 * 3] = core::mem::zeroed(); // ufbxi_uninit

    let index_begin: u32 = face.index_begin;
    let mut ix: u32 = 0;
    while ix < face.num_indices {
        let prev: u32 = *edges.add(ix.wrapping_mul(2).wrapping_add(0) as usize);
        let next: u32 = *edges.add(ix.wrapping_mul(2).wrapping_add(1) as usize);
        if (prev & 0x80000000) == 0 {
            ix = ix.wrapping_add(1);
            continue;
        }

        let mut dst: *mut u32 = indices.add(num_triangles.wrapping_mul(3) as usize);
        if num_triangles.wrapping_add(4) >= max_triangles {
            dst = last_triangles
                .as_mut_ptr()
                .add(num_last_triangles.wrapping_mul(3) as usize);
            num_last_triangles = num_last_triangles.wrapping_add(1);
        }

        *dst.add(0) = index_begin.wrapping_add(prev & 0x7fffffff);
        *dst.add(1) = index_begin.wrapping_add(ix);
        *dst.add(2) = index_begin.wrapping_add(next & 0x7fffffff);
        num_triangles = num_triangles.wrapping_add(1);
        ix = ix.wrapping_add(1);
    }

    // Copy over the last triangles
    ufbx_assert!(num_triangles == max_triangles);
    core::ptr::copy_nonoverlapping(
        last_triangles.as_ptr(),
        indices.add(
            max_triangles
                .wrapping_sub(num_last_triangles)
                .wrapping_mul(3) as usize,
        ),
        num_last_triangles.wrapping_mul(3) as usize,
    );

    num_triangles
}

// -- Topology (ufbx.c:28692-28819, ungated)

// ufbx.c:28692-28698 `ufbxi_topo_less_index_prev_next`
pub(crate) unsafe extern "C" fn topo_less_index_prev_next(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    let _ = user;
    let a: *const TopoEdge = va as *const TopoEdge;
    let b: *const TopoEdge = vb as *const TopoEdge;
    // C: the `prev`/`next` fields temporarily hold vertex indices and are
    // compared as `int32_t` (a `UFBX_NO_INDEX` sorts as -1, i.e. first).
    if (*a).prev as i32 != (*b).prev as i32 {
        return ((*a).prev as i32) < ((*b).prev as i32);
    }
    ((*a).next as i32) < ((*b).next as i32)
}

// ufbx.c:28700-28705 `ufbxi_topo_less_index_index`
pub(crate) unsafe extern "C" fn topo_less_index_index(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    let _ = user;
    let a: *const TopoEdge = va as *const TopoEdge;
    let b: *const TopoEdge = vb as *const TopoEdge;
    ((*a).index as i32) < ((*b).index as i32)
}

// ufbx.c:28707-28784 `ufbxi_compute_topology`
#[inline(never)]
pub(crate) unsafe fn compute_topology(mesh: *const Mesh, topo: *mut TopoEdge) {
    let num_indices: usize = (*mesh).num_indices;

    // Temporarily use `prev` and `next` for vertices
    let mut fi: u32 = 0;
    while (fi as usize) < (*mesh).num_faces {
        let face: Face = *(*mesh).faces.data.add(fi as usize);
        let mut pi: u32 = 0;
        while pi < face.num_indices {
            let te: *mut TopoEdge = topo.add(face.index_begin.wrapping_add(pi) as usize);
            let ni: u32 = pi.wrapping_add(1) % face.num_indices;
            let mut va: u32 = *(*mesh)
                .vertex_indices
                .data
                .add(face.index_begin.wrapping_add(pi) as usize);
            let mut vb: u32 = *(*mesh)
                .vertex_indices
                .data
                .add(face.index_begin.wrapping_add(ni) as usize);

            if vb < va {
                let vt: u32 = va;
                va = vb;
                vb = vt;
            }
            (*te).index = face.index_begin.wrapping_add(pi);
            (*te).twin = NO_INDEX;
            (*te).edge = NO_INDEX;
            (*te).prev = va;
            (*te).next = vb;
            (*te).face = fi;
            (*te).flags = TopoFlags::from_raw(0);
            pi = pi.wrapping_add(1);
        }
        fi = fi.wrapping_add(1);
    }

    unstable_sort(
        topo as *mut c_void,
        num_indices,
        size_of::<TopoEdge>(),
        topo_less_index_prev_next,
        core::ptr::null_mut(),
    );

    if !(*mesh).edges.data.is_null() {
        let mut ei: u32 = 0;
        while (ei as usize) < (*mesh).num_edges {
            let edge: Edge = *(*mesh).edges.data.add(ei as usize);
            let mut va: u32 = *(*mesh).vertex_indices.data.add(edge.a as usize);
            let mut vb: u32 = *(*mesh).vertex_indices.data.add(edge.b as usize);
            if vb < va {
                let vt: u32 = va;
                va = vb;
                vb = vt;
            }

            let mut ix: usize = num_indices;
            macro_lower_bound_eq::<TopoEdge>(
                32,
                &mut ix,
                topo,
                0,
                num_indices,
                // C: `(a->prev == va ? a->next < vb : a->prev < va)`
                |a| {
                    if (*a).prev == va {
                        (*a).next < vb
                    } else {
                        (*a).prev < va
                    }
                },
                // C: `(a->prev == va && a->next == vb)`
                |a| (*a).prev == va && (*a).next == vb,
            );

            while ix < num_indices && (*topo.add(ix)).prev == va && (*topo.add(ix)).next == vb {
                (*topo.add(ix)).edge = ei;
                ix += 1;
            }
            ei = ei.wrapping_add(1);
        }
    }

    // Connect paired edges
    let mut i0: usize = 0;
    while i0 < num_indices {
        let mut i1: usize = i0;

        let a: u32 = (*topo.add(i0)).prev;
        let b: u32 = (*topo.add(i0)).next;
        while i1 + 1 < num_indices && (*topo.add(i1 + 1)).prev == a && (*topo.add(i1 + 1)).next == b
        {
            i1 += 1;
        }

        if i1 == i0 + 1 {
            (*topo.add(i0)).twin = (*topo.add(i1)).index;
            (*topo.add(i1)).twin = (*topo.add(i0)).index;
        } else if i1 > i0 + 1 {
            let mut i: usize = i0;
            while i <= i1 {
                (*topo.add(i)).flags = (*topo.add(i)).flags | TopoFlags::NON_MANIFOLD;
                i += 1;
            }
        }

        i0 = i1 + 1;
    }

    unstable_sort(
        topo as *mut c_void,
        num_indices,
        size_of::<TopoEdge>(),
        topo_less_index_index,
        core::ptr::null_mut(),
    );

    // Fix `prev` and `next` to the actual index values
    let mut fi: u32 = 0;
    while (fi as usize) < (*mesh).num_faces {
        let face: Face = *(*mesh).faces.data.add(fi as usize);
        let mut i: u32 = 0;
        while i < face.num_indices {
            let to: *mut TopoEdge = topo.add(face.index_begin.wrapping_add(i) as usize);
            (*to).prev = face.index_begin.wrapping_add(
                (i.wrapping_add(face.num_indices).wrapping_sub(1)) % face.num_indices,
            );
            (*to).next = face
                .index_begin
                .wrapping_add(i.wrapping_add(1) % face.num_indices);
            i = i.wrapping_add(1);
        }
        fi = fi.wrapping_add(1);
    }
}

// ufbx.c:28786-28819 `ufbxi_is_edge_smooth`
pub(crate) unsafe fn is_edge_smooth(
    mesh: *const Mesh,
    topo: *const TopoEdge,
    num_topo: usize,
    index: u32,
    assume_smooth: bool,
) -> bool {
    // C: `ufbxi_ignore(num_topo);`
    let _ = num_topo;
    ufbx_assert!((index as usize) < num_topo);
    if !(*mesh).edge_smoothing.data.is_null() {
        let edge: u32 = (*topo.add(index as usize)).edge;
        if edge != NO_INDEX && *(*mesh).edge_smoothing.data.add(edge as usize) {
            return true;
        }
    }

    if !(*mesh).face_smoothing.data.is_null() {
        if *(*mesh)
            .face_smoothing
            .data
            .add((*topo.add(index as usize)).face as usize)
        {
            return true;
        }
        let twin: u32 = (*topo.add(index as usize)).twin;
        if twin != NO_INDEX {
            if *(*mesh)
                .face_smoothing
                .data
                .add((*topo.add(twin as usize)).face as usize)
            {
                return true;
            }
        }
    }

    if (*mesh).edge_smoothing.data.is_null()
        && (*mesh).face_smoothing.data.is_null()
        && (*mesh).vertex_normal.exists
    {
        let twin: u32 = (*topo.add(index as usize)).twin;
        if twin != NO_INDEX && (*mesh).vertex_normal.exists {
            ufbx_assert!((twin as usize) < num_topo);
            let a0: Vec3 = get_vertex_vec3(&(*mesh).vertex_normal, index as usize);
            let a1: Vec3 = get_vertex_vec3(
                &(*mesh).vertex_normal,
                (*topo.add(index as usize)).next as usize,
            );
            let b0: Vec3 = get_vertex_vec3(
                &(*mesh).vertex_normal,
                (*topo.add(twin as usize)).next as usize,
            );
            let b1: Vec3 = get_vertex_vec3(&(*mesh).vertex_normal, twin as usize);
            if a0.x == b0.x && a0.y == b0.y && a0.z == b0.z {
                return true;
            }
            if a1.x == b1.x && a1.y == b1.y && a1.z == b1.z {
                return true;
            }
        }
    } else if assume_smooth {
        return true;
    }

    false
}
