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
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
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
use crate::native::view::{view_raw_mut, view_read, view_write};
use crate::native::view::{Mode, View};
#[cfg(feature = "triangulation")]
use crate::prelude::as_f64;
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

// Typed interior-mutable VIEW over a `Face` field, reinterpreted in place;
// field accessors are generated (src/generated_views.rs).
#[cfg(feature = "triangulation")]
pub(crate) type FaceView = crate::native::view::View<Face>;

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
        view_read!(self, face)
    }
    #[inline(always)]
    pub(crate) fn set_face(&self, face: Face) {
        view_write!(self, face, face)
    }
    #[inline(always)]
    pub(crate) fn set_cur_face(&self, cur_face: Face) {
        view_write!(self, cur_face, cur_face)
    }
    #[inline(always)]
    pub(crate) fn cur_axis_dir(&self) -> Vec3 {
        view_read!(self, cur_axis_dir)
    }
    #[inline(always)]
    pub(crate) fn set_cur_axis_dir(&self, cur_axis_dir: Vec3) {
        view_write!(self, cur_axis_dir, cur_axis_dir)
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
        view_raw_mut!(self, positions)
    }

    // `kd_indices` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn kd_indices(&self) -> *mut u32 {
        view_read!(self, kd_indices)
    }

    #[inline(always)]
    pub(crate) fn set_kd_indices(&self, kd_indices: *mut u32) {
        view_write!(self, kd_indices, kd_indices)
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
// Stays `unsafe fn`: `index` is an unchecked index contract. It is added to
// `face.index_begin` and used to read `positions.indices` with no bounds
// check, and the value read is then used just as unchecked to index
// `positions.values`. Only the caller knows `index < face.num_indices`, so the
// obligation cannot be discharged here.
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn ngon_project(nc: &NgonContext, index: u32) -> Vec2 {
    // SAFETY: the caller guarantees `index < face.num_indices`, so
    // `index_begin + index` selects a live `indices` slot, whose value in turn
    // is an in-range `values` slot — the C gather this mirrors — so both the
    // inner and outer `add`/deref stay inside `nc.positions`' arrays.
    let point: Vec3 = unsafe {
        *nc.positions_view().values_view().data().add(
            *nc.positions_view()
                .indices_view()
                .data()
                .add(nc.face_view().index_begin().wrapping_add(index) as usize)
                as usize,
        )
    };

    // C: `ufbx_vec2 p;` — both fields are assigned below.
    // SAFETY: `Vec2` is two `Real`s; an all-zero bit pattern is a valid value.
    let mut p: Vec2 = unsafe { core::mem::zeroed() };
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
// Stays `unsafe fn`: `index` is an unchecked index contract carried through to
// `ngon_project` — only the caller knows it is in range for `nc`'s face.
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn kd_check_point(nc: &NgonContext, tri: &KdTriangle, index: u32) -> bool {
    if index == tri.indices[0] || index == tri.indices[1] || index == tri.indices[2] {
        return false;
    }
    // SAFETY: `index` is a corner index the caller vouches is in range for
    // `nc`'s face, which is `ngon_project`'s contract.
    let p: Vec2 = unsafe { ngon_project(nc, index) };

    let u: Real = orient2d(p, tri.points[0], tri.points[1]);
    let v: Real = orient2d(p, tri.points[1], tri.points[2]);
    let w: Real = orient2d(p, tri.points[2], tri.points[0]);

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
    tri: &KdTriangle,
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
        // SAFETY: forwards the caller's `begin`/`count` KD-index range vouch to
        // the recursive body unchanged.
        let ret = unsafe { kd_check_slow_rec(nc, tri, begin, count, axis) };
        UFBXI_RECURSION_DEPTH.with(|depth| depth.set(depth.get() - 1));
        ret
    }
    #[cfg(not(feature = "regression"))]
    {
        // SAFETY: forwards the caller's `begin`/`count` KD-index range vouch to
        // the recursive body unchanged.
        unsafe { kd_check_slow_rec(nc, tri, begin, count, axis) }
    }
}

// The recursive body (attached directly to `ufbxi_kd_check_slow` in
// non-regression C builds; recursive calls go through the guarded wrapper).
#[cfg(feature = "triangulation")]
unsafe fn kd_check_slow_rec(
    nc: &NgonContext,
    tri: &KdTriangle,
    begin: u32,
    count: u32,
    axis: u32,
) -> bool {
    let mut begin = begin;
    let mut count = count;
    let mut axis = axis;

    // C: `ufbx_vertex_vec3 pos = nc->positions;` — a struct memcpy.
    // SAFETY: `positions_mut_ptr()` is the address of `nc`'s own live
    // `positions` field; `ptr::read` copies the `VertexVec3` value out by value.
    let pos: VertexVec3 = unsafe { core::ptr::read(nc.positions_mut_ptr()) };
    let kd_indices: *mut u32 = nc.kd_indices();

    while count > 0 {
        let num_left: u32 = count / 2;
        let begin_right: u32 = begin.wrapping_add(num_left).wrapping_add(1);
        let num_right: u32 = count.wrapping_sub(num_left.wrapping_add(1));

        // SAFETY: `num_left = count/2 < count`, so `begin + num_left` lies inside
        // the `[begin, begin+count)` span of live `kd_indices` entries the caller
        // vouches for.
        let index: u32 = unsafe { *kd_indices.add(begin.wrapping_add(num_left) as usize) };
        // SAFETY: `index` is a corner index from `nc`'s KD index buffer, so
        // `index_begin + index` selects a live `indices` slot whose value is an
        // in-range `values` slot — the same gather as `ngon_project`.
        let point: Vec3 = unsafe {
            *pos.values.data.add(
                *pos.indices
                    .data
                    .add(nc.face_view().index_begin().wrapping_add(index) as usize)
                    as usize,
            )
        };
        let split: Real = dot3(point, nc.axes_at(axis as usize).get());
        // `axis` is 0 or 1, in bounds for the length-2 `min_t`/`max_t` arrays.
        let hit_left: bool = tri.min_t[axis as usize] <= split;
        let hit_right: bool = tri.max_t[axis as usize] >= split;

        if hit_left && hit_right {
            // SAFETY: `index` comes from `nc`'s KD index buffer, so it is a corner
            // index in range for `nc`'s face — `kd_check_point`'s contract.
            if unsafe { kd_check_point(nc, tri, index) } {
                return true;
            }

            // SAFETY: `[begin_right, begin_right + num_right)` is the right half of
            // the caller-vouched `[begin, begin + count)` KD-index span.
            if unsafe { kd_check_slow(nc, tri, begin_right, num_right, axis ^ 1) } {
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
// Stays `unsafe fn`: the KD-tree node fields it forwards (`slow_left`,
// `slow_right`, `slow_end`, `index_plus_one`) are unchecked index contracts on
// `nc`'s built KD tree — `kd_check_slow`'s run vouch and `kd_check_point`'s
// in-range corner index — which `&NgonContext` cannot express.
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn kd_check_fast(
    nc: &NgonContext,
    tri: &KdTriangle,
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
        // SAFETY: forwards the caller's `nc` KD-tree contract to the recursive
        // body unchanged.
        let ret = unsafe { kd_check_fast_rec(nc, tri, kd_index, axis, depth) };
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        ret
    }
    #[cfg(not(feature = "regression"))]
    {
        // SAFETY: forwards the caller's `nc` KD-tree contract to the recursive
        // body unchanged.
        unsafe { kd_check_fast_rec(nc, tri, kd_index, axis, depth) }
    }
}

// The recursive body (see `kd_check_slow_rec`).
#[cfg(feature = "triangulation")]
unsafe fn kd_check_fast_rec(
    nc: &NgonContext,
    tri: &KdTriangle,
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

        // `axis` is 0 or 1, in bounds for the length-2 `min_t`/`max_t` arrays.
        let hit_left: bool = tri.min_t[axis as usize] <= node.split;
        let hit_right: bool = tri.max_t[axis as usize] >= node.split;

        let side: u32 = if hit_left { 0 } else { 1 };
        let child_kd_index: u32 = kd_index.wrapping_mul(2).wrapping_add(1).wrapping_add(side);
        if hit_left && hit_right {
            // Check for the point on the split plane
            let index: u32 = node.index_plus_one.wrapping_sub(1);
            // SAFETY: `index` is `node.index_plus_one - 1`, a corner index stored
            // in `nc`'s KD tree — `kd_check_point`'s in-range contract.
            if unsafe { kd_check_point(nc, tri, index) } {
                return true;
            }

            // Recurse always to the right if we hit both sides
            if depth.wrapping_add(1) == KD_FAST_DEPTH as u32 {
                // SAFETY: `[slow_right, slow_end)` is a live span of `nc`'s KD index
                // buffer recorded in the node.
                if unsafe {
                    kd_check_slow(
                        nc,
                        tri,
                        node.slow_right,
                        node.slow_end.wrapping_sub(node.slow_right),
                        axis ^ 1,
                    )
                } {
                    return true;
                }
            } else {
                // SAFETY: `child_kd_index + 1` is a child of a live node in
                // `nc`'s KD tree, so `nc`'s tree contract carries into the
                // recursive `kd_check_fast`.
                if unsafe {
                    kd_check_fast(
                        nc,
                        tri,
                        child_kd_index.wrapping_add(1),
                        axis ^ 1,
                        depth.wrapping_add(1),
                    )
                } {
                    return true;
                }
            }
        }

        depth = depth.wrapping_add(1);
        axis ^= 1;
        kd_index = child_kd_index;

        if depth == KD_FAST_DEPTH as u32 {
            if hit_left {
                // SAFETY: `[slow_left, slow_right)` is a live span of `nc`'s KD index
                // buffer recorded in the node.
                return unsafe {
                    kd_check_slow(
                        nc,
                        tri,
                        node.slow_left,
                        node.slow_right.wrapping_sub(node.slow_left),
                        axis,
                    )
                };
            } else {
                // SAFETY: `[slow_right, slow_end)` is a live span of `nc`'s KD index
                // buffer recorded in the node.
                return unsafe {
                    kd_check_slow(
                        nc,
                        tri,
                        node.slow_right,
                        node.slow_end.wrapping_sub(node.slow_right),
                        axis,
                    )
                };
            }
        }
    }
}

// ufbx.c:28392-28406 `ufbxi_kd_check`
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn kd_check(nc: &NgonContext, points: *const Vec2, indices: *const u32) -> bool {
    // SAFETY: `KdTriangle` is plain arithmetic/index scalars; an all-zero bit
    // pattern is a valid value.
    let mut tri: KdTriangle = unsafe { core::mem::zeroed() }; // ufbxi_uninit

    // SAFETY: the caller passes `points`/`indices` addressing the 3 corners of
    // the candidate triangle, so offsets 0..=2 are in bounds and readable.
    unsafe {
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
    }
    // SAFETY: forwards the caller's `nc` KD-tree contract to `kd_check_fast`.
    unsafe { kd_check_fast(nc, &tri, 0, 0, 0) }
}

// ufbx.c:28408-28416 `ufbxi_kd_index_less` (an `ufbxi_less_fn`)
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe extern "C" fn kd_index_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    // SAFETY: the sort's comparator contract is that `user` is the value
    // `kd_build_rec` passed — the `&NgonContext` address — so this forms a
    // shared borrow of that live context.
    let nc: &NgonContext = unsafe { &*(user as *const NgonContext) };
    let pos: &crate::native::subdivision::VertexVec3View = nc.positions_view();
    // SAFETY: the comparator contract is that `va`/`vb` address live `u32`
    // elements of the KD index buffer being sorted.
    let (a, b) = unsafe { (*(va as *const u32), *(vb as *const u32)) };
    // SAFETY: `a` is a corner index from the KD index buffer, so
    // `cur_face.index_begin + a` selects a live `indices` slot whose value is an
    // in-range `values` slot; `pos` is `nc`'s own live `positions`.
    let da: Real = dot3(nc.cur_axis_dir(), unsafe {
        *pos.values().data.add(
            *pos.indices()
                .data
                .add(nc.cur_face_view().index_begin().wrapping_add(a) as usize)
                as usize,
        )
    });
    // SAFETY: as above, for corner index `b`.
    let db: Real = dot3(nc.cur_axis_dir(), unsafe {
        *pos.values().data.add(
            *pos.indices()
                .data
                .add(nc.cur_face_view().index_begin().wrapping_add(b) as usize)
                as usize,
        )
    });
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
        // SAFETY: forwards the caller's `nc`/`indices`/`tmp` validity contract
        // to the recursive body unchanged.
        unsafe { kd_build_rec(nc, indices, tmp, num, axis, fast_index, depth) };
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
    }
    #[cfg(not(feature = "regression"))]
    {
        // SAFETY: forwards the caller's `nc`/`indices`/`tmp` validity contract
        // to the recursive body unchanged.
        unsafe { kd_build_rec(nc, indices, tmp, num, axis, fast_index, depth) };
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
    // SAFETY: `positions_mut_ptr()` is the address of `nc`'s own live
    // `positions` field; `ptr::read` copies the `VertexVec3` value out by value.
    let pos: VertexVec3 = unsafe { core::ptr::read(nc.positions_mut_ptr()) };
    let axis_dir: Vec3 = nc.axes_at(axis as usize).get();
    let face: Face = nc.face();

    nc.set_cur_axis_dir(axis_dir);
    nc.set_cur_face(face);

    // Sort the remaining indices based on the axis
    // SAFETY: `indices` addresses `num` live `u32`s and `tmp` is a scratch
    // buffer of matching size (caller contract), element size / comparator match
    // `u32` / `kd_index_less`, and the comparator's `user` is `nc`'s own address.
    unsafe {
        stable_sort(
            size_of::<u32>(),
            16,
            indices as *mut c_void,
            tmp as *mut c_void,
            num as usize,
            kd_index_less,
            (nc as *const NgonContext) as *mut c_void,
        );
    }

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

        // SAFETY: `num_left = num/2 < num`, so this reads a live entry of the
        // `num`-element `indices` span the caller vouches for.
        let index: u32 = unsafe { *indices.add(num_left as usize) };
        let kd: *mut KdNode = nc.kd_nodes_at(fast_index as usize).as_ptr();

        // SAFETY: `kd` is the address of `nc`'s own live `kd_nodes[fast_index]`
        // slot; `index` is a corner index so `index_begin + index` gathers a
        // live `indices`→`values` slot from `pos` (`nc`'s own positions).
        unsafe {
            (*kd).split = dot3(
                axis_dir,
                *pos.values.data.add(
                    *pos.indices
                        .data
                        .add(face.index_begin.wrapping_add(index) as usize)
                        as usize,
                ),
            );
        }
        // SAFETY: `kd` addresses `nc`'s own live `kd_nodes` slot (as above).
        unsafe {
            (*kd).index_plus_one = index.wrapping_add(1);
        }

        if depth.wrapping_add(1) == KD_FAST_DEPTH as u32 {
            // SAFETY: `indices` and `nc.kd_indices()` both point into the single
            // KD index scratch buffer, so `offset_from` is well defined; `kd`
            // addresses `nc`'s own live `kd_nodes` slot.
            unsafe {
                (*kd).slow_left = indices.offset_from(nc.kd_indices()) as u32;
                (*kd).slow_right = (*kd).slow_left.wrapping_add(num_left);
                (*kd).slow_end = (*kd).slow_right.wrapping_add(num_right);
            }
        } else {
            // SAFETY: `kd` addresses `nc`'s own live `kd_nodes` slot (as above).
            unsafe {
                (*kd).slow_left = u32::MAX;
                (*kd).slow_right = u32::MAX;
                (*kd).slow_end = u32::MAX;
            }
        }
    }

    let child_fast: u32 = fast_index.wrapping_mul(2).wrapping_add(1);
    // SAFETY: the left partition `[0, num_left)` stays within the `num`-element
    // `indices` span; forwards `nc`/`tmp` validity to the recursive call.
    unsafe {
        kd_build(
            nc,
            indices,
            tmp,
            num_left,
            axis ^ 1,
            child_fast.wrapping_add(0),
            depth.wrapping_add(1),
        );
    }

    if dst_right != begin_right {
        // C: `memmove(indices + dst_right, indices + begin_right, num_right * sizeof(uint32_t));`
        // SAFETY: `begin_right + num_right = num` and `dst_right <= begin_right`,
        // so both `num_right`-element runs lie inside the `num`-element `indices`
        // span; `ptr::copy` tolerates the overlap (a `memmove`).
        unsafe {
            core::ptr::copy(
                indices.add(begin_right as usize),
                indices.add(dst_right as usize),
                num_right as usize,
            );
        }
    }

    // SAFETY: the right partition starts at `indices + dst_right` and spans
    // `num_right` elements, all inside the `num`-element `indices` span
    // (`dst_right + num_right <= num`); forwards `nc`/`tmp` validity.
    unsafe {
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
}

// -- Triangulation (ufbx.c:28472-28690, `#if UFBXI_FEATURE_TRIANGULATION`)

// ufbx.c:28474-28487 `ufbxi_ngon_tri_weight`
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn ngon_tri_weight(points: *const Vec2) -> Real {
    // SAFETY: the caller passes `points` addressing 3 consecutive `Vec2`s (the
    // candidate triangle corners), so offsets 0..=2 are in bounds and readable.
    let (p0, p1, p2) = unsafe { (*points.add(0), *points.add(1), *points.add(2)) };
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
        as_f64!(math::EPSILON),
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
    // SAFETY: `positions_mut_ptr()` is the address of `nc`'s own live
    // `positions`, and `face` is `nc`'s own face — the pairing
    // `get_weighted_face_normal` requires.
    let mut normal: Vec3 =
        unsafe { crate::native::api::get_weighted_face_normal(nc.positions_mut_ptr(), face) };
    let len: Real = length3(normal);
    if len > math::EPSILON {
        normal = mul3(normal, 1.0 / len);
    } else {
        normal.x = 1.0;
        normal.y = 0.0;
        normal.z = 0.0;
    }

    // SAFETY: `Vec3` is three `Real`s; an all-zero bit pattern is a valid value.
    let mut axis: Vec3 = unsafe { core::mem::zeroed() }; // ufbxi_uninit
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
        .set(slow_normalized_cross3(&normal, &nc.axes_at(0).get()));
    nc.axes_at(2).set(normal);

    let kd_indices: *mut u32 = indices;
    nc.set_kd_indices(kd_indices);

    // SAFETY: the caller sizes `indices` for at least `num_indices` u32s with
    // `num_indices >= face.num_indices * 2`, so `indices + face.num_indices`
    // stays within that allocation.
    let kd_tmp: *mut u32 = unsafe { indices.add(face.num_indices as usize) };

    // Collect all the reflex corners for intersection testing.
    let mut num_kd_indices: u32 = 0;
    {
        // SAFETY: `face.num_indices - 1 < face.num_indices` (non-zero:
        // `face.num_indices > 4` asserted above) is a valid corner index for
        // `ngon_project`.
        let mut a: Vec2 = unsafe { ngon_project(nc, face.num_indices.wrapping_sub(1)) };
        // SAFETY: `0` is a valid corner index for `ngon_project`.
        let mut b: Vec2 = unsafe { ngon_project(nc, 0) };
        let mut i: u32 = 0;
        while i < face.num_indices {
            let next: u32 = if i.wrapping_add(1) < face.num_indices {
                i.wrapping_add(1)
            } else {
                0
            };
            // SAFETY: `next < face.num_indices` (either `i+1` under the guard or
            // wrapped to `0`) is a valid corner index for `ngon_project`.
            let c: Vec2 = unsafe { ngon_project(nc, next) };

            if orient2d(a, b, c) <= 0.0 {
                // C: `kd_indices[num_kd_indices++] = i;`
                // SAFETY: at most one entry is pushed per corner, so
                // `num_kd_indices < face.num_indices` slots of the `indices`
                // scratch (aliased by `kd_indices`) stay in bounds.
                unsafe { *kd_indices.add(num_kd_indices as usize) = i };
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
    // SAFETY: `kd_indices` holds `num_kd_indices` live entries and `kd_tmp`
    // is the matching scratch region `indices + face.num_indices`, both inside
    // the caller's `indices` allocation.
    unsafe { kd_build(nc, kd_indices, kd_tmp, num_kd_indices, 0, 0, 0) };

    // C: `uint32_t *edges = indices + num_indices - face.num_indices * 2;`
    // SAFETY: `num_indices >= face.num_indices * 2` (asserted above), so both
    // `indices + num_indices` (one past the caller's allocation) and the
    // `- 2*face.num_indices` back-off land inside that same allocation.
    let edges: *mut u32 = unsafe {
        indices
            .add(num_indices as usize)
            .sub(face.num_indices.wrapping_mul(2) as usize)
    };

    // Initialize `edges` to be a connectivity structure where:
    //  `edges[2*i + 0]` is the previous vertex of `i`
    //  `edges[2*i + 1]` is the next vertex of `i`
    // When clipped we mark indices with the high bit (0x80000000)
    {
        let mut i: u32 = 0;
        while i < face.num_indices {
            // SAFETY: `i < face.num_indices`, so `2*i + {0,1}` index the
            // `2*face.num_indices`-element `edges` region in bounds.
            unsafe {
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
            }
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
        // SAFETY: `[Real; 2]` is plain floats; an all-zero bit pattern is valid.
        let mut weights: [Real; 2] = unsafe { core::mem::zeroed() }; // ufbxi_uninit
                                                                     // SAFETY: `[Vec2; 4]` is plain floats; an all-zero bit pattern is valid.
        let mut points: [Vec2; 4] = unsafe { core::mem::zeroed() }; // ufbxi_uninit

        let mut num_steps: u32 = 0;
        while indices_left > 3 {
            // SAFETY: `point_indices` are corner indices in `[0, face.num_indices)`,
            // valid corner indices for `ngon_project`.
            unsafe {
                points[0] = ngon_project(nc, point_indices[0]);
                points[1] = ngon_project(nc, point_indices[1]);
                points[2] = ngon_project(nc, point_indices[2]);
                points[3] = ngon_project(nc, point_indices[3]);
            }

            // SAFETY: `points` is a 4-element array, so offset 0 leaves 3 readable
            // corners — `ngon_tri_weight`'s requirement.
            weights[0] = unsafe { ngon_tri_weight(points.as_ptr().add(0)) };
            // SAFETY: offset 1 of the 4-element `points` leaves 3 readable corners.
            weights[1] = unsafe { ngon_tri_weight(points.as_ptr().add(1)) };

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
                // SAFETY: `side` is 0 or 1, so `points`/`point_indices` (both
                // length-4) offset by `side` leave 3 readable corners —
                // `kd_check`'s requirement — and `nc` is the live context.
                if !unsafe {
                    kd_check(
                        nc,
                        points.as_ptr().add(side as usize),
                        point_indices.as_ptr().add(side as usize),
                    )
                } {
                    let ia: u32 = point_indices[side.wrapping_add(0) as usize];
                    let ib: u32 = point_indices[side.wrapping_add(1) as usize];
                    let ic: u32 = point_indices[side.wrapping_add(2) as usize];

                    // Mark as clipped
                    // SAFETY: `ib`/`ic`/`ia` are corner indices in
                    // `[0, face.num_indices)`, so `2*idx + {0,1}` index the
                    // `2*face.num_indices`-element `edges` region in bounds.
                    unsafe {
                        *edges.add(ib.wrapping_mul(2).wrapping_add(0) as usize) |= 0x80000000;
                        *edges.add(ib.wrapping_mul(2).wrapping_add(1) as usize) |= 0x80000000;

                        *edges.add(ic.wrapping_mul(2).wrapping_add(0) as usize) = ia;
                        *edges.add(ia.wrapping_mul(2).wrapping_add(1) as usize) = ic;
                    }

                    indices_left = indices_left.wrapping_sub(1);

                    // TODO: This may cause O(n^2) behavior!
                    num_steps = 0;

                    if side == 1 {
                        point_indices[2] = point_indices[3];
                        // SAFETY: `point_indices[3] < face.num_indices`, so
                        // `2*idx + 1` indexes `edges` in bounds.
                        point_indices[3] = unsafe {
                            *edges.add(point_indices[3].wrapping_mul(2).wrapping_add(1) as usize)
                        };
                    } else {
                        point_indices[1] = point_indices[0];
                        // SAFETY: `point_indices[0] < face.num_indices`, so
                        // `2*idx + 0` indexes `edges` in bounds.
                        point_indices[0] = unsafe {
                            *edges.add(point_indices[0].wrapping_mul(2).wrapping_add(0) as usize)
                        };
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
            // SAFETY: `point_indices[3] < face.num_indices`, so `2*idx + 1`
            // indexes the `2*face.num_indices`-element `edges` region in bounds.
            point_indices[3] =
                unsafe { *edges.add(point_indices[3].wrapping_mul(2).wrapping_add(1) as usize) };
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
            // SAFETY: `ix` is a corner index in `[0, face.num_indices)`, so
            // `2*ix + {0,1}` index the `2*face.num_indices`-element `edges` in
            // bounds; the `prev`/`next` read back are themselves corner indices.
            let (prev, next) = unsafe {
                (
                    *edges.add(ix.wrapping_mul(2).wrapping_add(0) as usize),
                    *edges.add(ix.wrapping_mul(2).wrapping_add(1) as usize),
                )
            };

            // Mark as clipped
            // SAFETY: `ix`/`prev`/`next` are corner indices in
            // `[0, face.num_indices)`, so `2*idx + {0,1}` index `edges` in bounds.
            unsafe {
                *edges.add(ix.wrapping_mul(2).wrapping_add(0) as usize) |= 0x80000000;
                *edges.add(ix.wrapping_mul(2).wrapping_add(1) as usize) |= 0x80000000;

                *edges.add(prev.wrapping_mul(2).wrapping_add(1) as usize) = next;
                *edges.add(next.wrapping_mul(2).wrapping_add(0) as usize) = prev;
            }

            indices_left = indices_left.wrapping_sub(1);
            ix = next;
        }

        // Now we have a single triangle left at `ix`.
        // SAFETY: `ix < face.num_indices`, so `2*ix + {0,1}` index `edges` in bounds.
        unsafe {
            *edges.add(ix.wrapping_mul(2).wrapping_add(0) as usize) |= 0x80000000;
            *edges.add(ix.wrapping_mul(2).wrapping_add(1) as usize) |= 0x80000000;
        }
    }

    // Expand the adjacency information `edges` into proper triangles.
    // Care needs to be taken here as both refer to the same memory area:
    // The last 4 triangles may overlap in source and destination so we write
    // them to a stack buffer and copy them over in the end.
    let max_triangles: u32 = face.num_indices.wrapping_sub(2);
    let mut num_triangles: u32 = 0;
    let mut num_last_triangles: u32 = 0;
    // SAFETY: `[u32; 12]` is plain ints; an all-zero bit pattern is valid.
    let mut last_triangles: [u32; 4 * 3] = unsafe { core::mem::zeroed() }; // ufbxi_uninit

    let index_begin: u32 = face.index_begin;
    let mut ix: u32 = 0;
    while ix < face.num_indices {
        // SAFETY: `ix < face.num_indices`, so `2*ix + {0,1}` index the
        // `2*face.num_indices`-element `edges` region in bounds.
        let (prev, next) = unsafe {
            (
                *edges.add(ix.wrapping_mul(2).wrapping_add(0) as usize),
                *edges.add(ix.wrapping_mul(2).wrapping_add(1) as usize),
            )
        };
        if (prev & 0x80000000) == 0 {
            ix = ix.wrapping_add(1);
            continue;
        }

        // SAFETY: `num_triangles < max_triangles = face.num_indices - 2`, so
        // `num_triangles * 3` stays within the caller's `indices` allocation.
        let mut dst: *mut u32 = unsafe { indices.add(num_triangles.wrapping_mul(3) as usize) };
        if num_triangles.wrapping_add(4) >= max_triangles {
            // SAFETY: this arm runs at most 4 times, so `num_last_triangles < 4`
            // and `num_last_triangles * 3 < 12`, in bounds for `last_triangles`.
            dst = unsafe {
                last_triangles
                    .as_mut_ptr()
                    .add(num_last_triangles.wrapping_mul(3) as usize)
            };
            num_last_triangles = num_last_triangles.wrapping_add(1);
        }

        // SAFETY: `dst` addresses 3 writable slots — either a triangle slot in
        // `indices` or a `last_triangles` slot selected above.
        unsafe {
            *dst.add(0) = index_begin.wrapping_add(prev & 0x7fffffff);
            *dst.add(1) = index_begin.wrapping_add(ix);
            *dst.add(2) = index_begin.wrapping_add(next & 0x7fffffff);
        }
        num_triangles = num_triangles.wrapping_add(1);
        ix = ix.wrapping_add(1);
    }

    // Copy over the last triangles
    ufbx_assert!(num_triangles == max_triangles);
    // SAFETY: `num_last_triangles * 3` elements of the `last_triangles` source
    // are copied to `indices + (max_triangles - num_last_triangles) * 3`, the
    // tail of the `max_triangles`-triangle output region in `indices`; source
    // (stack) and destination (caller buffer) are distinct objects.
    unsafe {
        core::ptr::copy_nonoverlapping(
            last_triangles.as_ptr(),
            indices.add(
                max_triangles
                    .wrapping_sub(num_last_triangles)
                    .wrapping_mul(3) as usize,
            ),
            num_last_triangles.wrapping_mul(3) as usize,
        );
    }

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
    // SAFETY: the sort's comparator contract is that `va`/`vb` address live
    // `TopoEdge` elements of the array being sorted (`compute_topology`'s `topo`).
    unsafe {
        if (*a).prev as i32 != (*b).prev as i32 {
            return ((*a).prev as i32) < ((*b).prev as i32);
        }
        ((*a).next as i32) < ((*b).next as i32)
    }
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
    // SAFETY: the sort's comparator contract is that `va`/`vb` address live
    // `TopoEdge` elements of the array being sorted (`compute_topology`'s `topo`).
    unsafe { ((*a).index as i32) < ((*b).index as i32) }
}

// ufbx.c:28707-28784 `ufbxi_compute_topology`
#[inline(never)]
pub(crate) unsafe fn compute_topology<M: Mode>(mesh: &View<Mesh, M>, topo: *mut TopoEdge) {
    let num_indices: usize = mesh.num_indices();

    // Temporarily use `prev` and `next` for vertices
    let mut fi: u32 = 0;
    while (fi as usize) < mesh.num_faces() {
        // SAFETY: `fi < num_faces`, so `faces.data[fi]` is a live face.
        let face: Face = unsafe { *mesh.faces().data.add(fi as usize) };
        let mut pi: u32 = 0;
        while pi < face.num_indices {
            // SAFETY: `topo` addresses `num_indices` live `TopoEdge`s and
            // `index_begin + pi < num_indices` for a valid face, so the offset
            // is in bounds; this only computes the address.
            let te: *mut TopoEdge = unsafe { topo.add(face.index_begin.wrapping_add(pi) as usize) };
            let ni: u32 = pi.wrapping_add(1) % face.num_indices;
            // SAFETY: `index_begin + pi` is a live `vertex_indices` slot for the
            // face (as above); `mesh` owns that array.
            let mut va: u32 = unsafe {
                *mesh
                    .vertex_indices()
                    .data
                    .add(face.index_begin.wrapping_add(pi) as usize)
            };
            // SAFETY: `ni < face.num_indices`, so `index_begin + ni` is a live
            // `vertex_indices` slot for the face.
            let mut vb: u32 = unsafe {
                *mesh
                    .vertex_indices()
                    .data
                    .add(face.index_begin.wrapping_add(ni) as usize)
            };

            if vb < va {
                std::mem::swap(&mut va, &mut vb);
            }
            // SAFETY: `te` addresses a live `TopoEdge` slot (computed above).
            unsafe {
                (*te).index = face.index_begin.wrapping_add(pi);
                (*te).twin = NO_INDEX;
                (*te).edge = NO_INDEX;
                (*te).prev = va;
                (*te).next = vb;
                (*te).face = fi;
                (*te).flags = TopoFlags::from_raw(0);
            }
            pi = pi.wrapping_add(1);
        }
        fi = fi.wrapping_add(1);
    }

    // SAFETY: `topo` addresses `num_indices` live `TopoEdge`s, the element size
    // and comparator match that type, and the comparator takes no `user` data.
    unsafe {
        unstable_sort(
            topo as *mut c_void,
            num_indices,
            size_of::<TopoEdge>(),
            topo_less_index_prev_next,
            core::ptr::null_mut(),
        );
    }

    if !mesh.edges().data.is_null() {
        let mut ei: u32 = 0;
        while (ei as usize) < mesh.num_edges() {
            // SAFETY: `ei < num_edges`, so `edges.data[ei]` is a live edge.
            let edge: Edge = unsafe { *mesh.edges().data.add(ei as usize) };
            // SAFETY: `edge.a`/`edge.b` index the mesh's own `vertex_indices`
            // array, live for a mesh whose `edges` are populated.
            let (mut va, mut vb) = unsafe {
                (
                    *mesh.vertex_indices().data.add(edge.a as usize),
                    *mesh.vertex_indices().data.add(edge.b as usize),
                )
            };
            if vb < va {
                std::mem::swap(&mut va, &mut vb);
            }

            let mut ix: usize = num_indices;
            // SAFETY: `topo` addresses `num_indices` live `TopoEdge`s and
            // `result_ptr` is the writable local `ix`; the two comparator
            // closures (lexically inside this block) dereference only the
            // in-range element pointer the search hands them.
            unsafe {
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
            }

            // SAFETY: `topo.add(ix)` with `ix < num_indices` is a live `TopoEdge`.
            while ix < num_indices
                && unsafe { (*topo.add(ix)).prev } == va
                && unsafe { (*topo.add(ix)).next } == vb
            {
                // SAFETY: `ix < num_indices`, so `topo.add(ix)` is live.
                unsafe {
                    (*topo.add(ix)).edge = ei;
                }
                ix += 1;
            }
            ei = ei.wrapping_add(1);
        }
    }

    // Connect paired edges
    let mut i0: usize = 0;
    while i0 < num_indices {
        let mut i1: usize = i0;

        // SAFETY: `i0 < num_indices`, so `topo.add(i0)` is a live `TopoEdge`.
        let (a, b) = unsafe { ((*topo.add(i0)).prev, (*topo.add(i0)).next) };
        // SAFETY: the guard keeps `i1 + 1 < num_indices`, so `topo.add(i1 + 1)`
        // is a live `TopoEdge`.
        while i1 + 1 < num_indices
            && unsafe { (*topo.add(i1 + 1)).prev } == a
            && unsafe { (*topo.add(i1 + 1)).next } == b
        {
            i1 += 1;
        }

        if i1 == i0 + 1 {
            // SAFETY: `i0`/`i1` are both `< num_indices` here, so both
            // `topo.add(..)` are live `TopoEdge`s.
            unsafe {
                (*topo.add(i0)).twin = (*topo.add(i1)).index;
                (*topo.add(i1)).twin = (*topo.add(i0)).index;
            }
        } else if i1 > i0 + 1 {
            let mut i: usize = i0;
            while i <= i1 {
                // SAFETY: `i <= i1 < num_indices`, so `topo.add(i)` is live.
                unsafe {
                    (*topo.add(i)).flags = (*topo.add(i)).flags | TopoFlags::NON_MANIFOLD;
                }
                i += 1;
            }
        }

        i0 = i1 + 1;
    }

    // SAFETY: `topo` addresses `num_indices` live `TopoEdge`s, the element size
    // and comparator match that type, and the comparator takes no `user` data.
    unsafe {
        unstable_sort(
            topo as *mut c_void,
            num_indices,
            size_of::<TopoEdge>(),
            topo_less_index_index,
            core::ptr::null_mut(),
        );
    }

    // Fix `prev` and `next` to the actual index values
    let mut fi: u32 = 0;
    while (fi as usize) < mesh.num_faces() {
        // SAFETY: `fi < num_faces`, so `faces.data[fi]` is a live face.
        let face: Face = unsafe { *mesh.faces().data.add(fi as usize) };
        let mut i: u32 = 0;
        while i < face.num_indices {
            // SAFETY: `index_begin + i < num_indices` for a valid face, so the
            // offset is in bounds for the `num_indices`-element `topo`.
            let to: *mut TopoEdge = unsafe { topo.add(face.index_begin.wrapping_add(i) as usize) };
            // SAFETY: `to` addresses a live `TopoEdge` slot (computed above).
            unsafe {
                (*to).prev = face.index_begin.wrapping_add(
                    (i.wrapping_add(face.num_indices).wrapping_sub(1)) % face.num_indices,
                );
                (*to).next = face
                    .index_begin
                    .wrapping_add(i.wrapping_add(1) % face.num_indices);
            }
            i = i.wrapping_add(1);
        }
        fi = fi.wrapping_add(1);
    }
}

// ufbx.c:28786-28819 `ufbxi_is_edge_smooth`
pub(crate) unsafe fn is_edge_smooth<M: Mode>(
    mesh: &View<Mesh, M>,
    topo: *const TopoEdge,
    num_topo: usize,
    index: u32,
    assume_smooth: bool,
) -> bool {
    // C: `ufbxi_ignore(num_topo);`
    let _ = num_topo;
    ufbx_assert!((index as usize) < num_topo);
    if !mesh.edge_smoothing().data.is_null() {
        // SAFETY: `index < num_topo` (asserted), so `topo.add(index)` is a live
        // `TopoEdge`.
        let edge: u32 = unsafe { (*topo.add(index as usize)).edge };
        // SAFETY: this branch has non-null `edge_smoothing`, which holds one
        // entry per edge; `edge` (guarded `!= NO_INDEX`) is a valid edge index.
        if edge != NO_INDEX && unsafe { *mesh.edge_smoothing().data.add(edge as usize) } {
            return true;
        }
    }

    if !mesh.face_smoothing().data.is_null() {
        // SAFETY: `index < num_topo`, so `topo.add(index)` is a live `TopoEdge`
        // whose `face` is a valid index into the non-null per-face
        // `face_smoothing` array.
        if unsafe {
            *mesh
                .face_smoothing()
                .data
                .add((*topo.add(index as usize)).face as usize)
        } {
            return true;
        }
        // SAFETY: `index < num_topo`, so `topo.add(index)` is a live `TopoEdge`.
        let twin: u32 = unsafe { (*topo.add(index as usize)).twin };
        if twin != NO_INDEX {
            // SAFETY: a non-`NO_INDEX` `twin` is a valid `topo` index, so
            // `topo.add(twin)` is live and its `face` indexes the non-null
            // per-face `face_smoothing` array.
            if unsafe {
                *mesh
                    .face_smoothing()
                    .data
                    .add((*topo.add(twin as usize)).face as usize)
            } {
                return true;
            }
        }
    }

    if mesh.edge_smoothing().data.is_null()
        && mesh.face_smoothing().data.is_null()
        && mesh.vertex_normal().exists()
    {
        // SAFETY: `index < num_topo`, so `topo.add(index)` is a live `TopoEdge`.
        let twin: u32 = unsafe { (*topo.add(index as usize)).twin };
        if twin != NO_INDEX && mesh.vertex_normal().exists() {
            ufbx_assert!((twin as usize) < num_topo);
            let a0: Vec3 = get_vertex_vec3(mesh.vertex_normal(), index as usize);
            // SAFETY: `index < num_topo`, so `topo.add(index)` is a live
            // `TopoEdge`; its `next` is the attribute index being read.
            let a1: Vec3 =
                get_vertex_vec3(
                    mesh.vertex_normal(),
                    unsafe { (*topo.add(index as usize)).next } as usize,
                );
            // SAFETY: `twin < num_topo` (asserted), so `topo.add(twin)` is a live
            // `TopoEdge`; its `next` is the attribute index being read.
            let b0: Vec3 =
                get_vertex_vec3(
                    mesh.vertex_normal(),
                    unsafe { (*topo.add(twin as usize)).next } as usize,
                );
            let b1: Vec3 = get_vertex_vec3(mesh.vertex_normal(), twin as usize);
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
