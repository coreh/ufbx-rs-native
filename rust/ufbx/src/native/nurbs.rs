//! Port of the `// -- NURBS` banner section of ufbx.c (ufbx.c:27769-28241):
//! the basis weight/derivative helpers, `ufbxi_line_curve_imp`, and — under
//! `feature = "tessellation"` (C: `UFBXI_FEATURE_TESSELLATION`) — the curve /
//! surface tessellation contexts and `ufbxi_tessellate_nurbs_*_imp`. The
//! public entry points that drive these (`ufbx_evaluate_nurbs_basis`,
//! `ufbx_evaluate_nurbs_curve`, `ufbx_evaluate_nurbs_surface`,
//! `ufbx_tessellate_nurbs_curve`, `ufbx_tessellate_nurbs_surface`,
//! `ufbx_free_line_curve`, `ufbx_retain_line_curve`) live in `native::api`
//! with the rest of the `// -- API` section.
//!
//! Float parity: all basis/tessellation math is hash-oracle-observable
//! (tessellated positions/normals feed scene hashes) — operation order and the
//! C `f`-suffixed literals (`0.0f`, `1.0f`, `0.0000001f` ported as
//! `0.0f32 as Real`, ...) are verbatim.
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]

use core::mem::size_of;

#[cfg(feature = "tessellation")]
use crate::generated::Error;
use crate::generated::LineCurve;
#[cfg(feature = "tessellation")]
use crate::generated::{CurvePoint, SurfacePoint};
#[cfg(feature = "tessellation")]
use crate::generated::{
    ElementType, Face, LineSegment, Material, Mesh, MeshPart, NurbsCurve, NurbsSurface,
    NurbsTopology, RawTessellateCurveOpts, RawTessellateSurfaceOpts, Vec2, Vec3,
};
use crate::native::allocator::LINE_CURVE_IMP_MAGIC;
#[cfg(feature = "tessellation")]
use crate::native::allocator::{does_overflow, init_ator, Allocator};
#[cfg(feature = "tessellation")]
use crate::native::api::{
    compute_normals, evaluate_nurbs_curve_view, evaluate_nurbs_surface_view, EMPTY_STRING,
    ZERO_VEC2, ZERO_VEC3,
};
#[cfg(feature = "tessellation")]
use crate::native::buf::Buf;
#[cfg(feature = "tessellation")]
use crate::native::error::Fail;
#[cfg(feature = "tessellation")]
use crate::native::error::{ufbxi_check_err, ufbxi_check_err_msg};
#[cfg(feature = "tessellation")]
use crate::native::hash::Map;
#[cfg(feature = "tessellation")]
use crate::native::parse::{finish_imp, FinishedImp, ImpHandle, ImpHeader, MeshImp, SceneImp};
use crate::native::parse::{ImpRecover, Refcount};
#[cfg(feature = "tessellation")]
use crate::native::platform::ufbx_assert;
#[cfg(feature = "tessellation")]
use crate::native::read::finalize_mesh;
#[cfg(feature = "tessellation")]
use crate::native::scene_process::finalize_mesh_material;
#[cfg(feature = "tessellation")]
use crate::native::string_pool::slow_normalize3;
#[cfg(feature = "tessellation")]
use crate::native::view::view_raw_mut;
use crate::native::view::View;
use crate::native::view::{view_project, view_read, view_write};
use crate::native::view::{Const, Run};
use crate::prelude::Real;
#[cfg(feature = "tessellation")]
use crate::prelude::Ref;

// ufbx.c:64-66 `UFBXI_MAX_NURBS_ORDER` (top-of-file config constant, owned by
// this section — only the NURBS evaluation entry points read it)
pub(crate) const MAX_NURBS_ORDER: usize = 128;

#[cfg(feature = "tessellation")]
impl View<Vec2> {
    #[inline(always)]
    pub(crate) fn set_x(&self, value: Real) {
        view_write!(self, x, value)
    }

    #[inline(always)]
    pub(crate) fn set_y(&self, value: Real) {
        view_write!(self, y, value)
    }
}

// ufbx.c:27771-27780 `ufbxi_nurbs_weight`
// C copies `ufbx_real_list` by value at the call sites and passes `&knots`;
// the Rust caller passes a bounded read-only run over the same stored list.
#[inline(always)]
pub(crate) fn nurbs_weight(
    knots: Run<'_, Real, Const>,
    knot: usize,
    degree: usize,
    u: Real,
) -> Real {
    if knot >= knots.len() {
        return 0.0f32 as Real;
    }
    if knots.len() - knot < degree {
        return 0.0f32 as Real;
    }
    // C's `< degree` early-out admits the one-past boundary when the remaining
    // count equals `degree`; valid basis spans are strictly inside the run.
    assert!(degree < knots.len() - knot);
    let prev_u: Real = knots.copy_at(knot);
    let next_u: Real = knots.copy_at(knot + degree);
    if prev_u >= next_u {
        return 0.0f32 as Real;
    }
    if u <= prev_u {
        return 0.0f32 as Real;
    }
    if u >= next_u {
        return 1.0f32 as Real;
    }
    (u - prev_u) / (next_u - prev_u)
}

// ufbx.c:27782-27789 `ufbxi_nurbs_deriv`
#[inline(always)]
pub(crate) fn nurbs_deriv(knots: Run<'_, Real, Const>, knot: usize, degree: usize) -> Real {
    if knot >= knots.len() {
        return 0.0f32 as Real;
    }
    if knots.len() - knot < degree {
        return 0.0f32 as Real;
    }
    assert!(degree < knots.len() - knot);
    let prev_u: Real = knots.copy_at(knot);
    let next_u: Real = knots.copy_at(knot + degree);
    if prev_u >= next_u {
        return 0.0f32 as Real;
    }
    degree as Real / (next_u - prev_u)
}

// ufbx.c:27791-27795 `ufbxi_line_curve_imp`
// NOT gated: like the struct in C, it sits before `#if UFBXI_FEATURE_TESSELLATION`
// (`ufbx_free_line_curve`/`ufbx_retain_line_curve` dereference it unconditionally).
#[repr(C)]
pub(crate) struct LineCurveImp {
    pub refcount: Refcount,
    pub curve: LineCurve,
    pub magic: u32,
}

// ufbx.c:27797 `ufbx_static_assert(line_curve_imp_offset, offsetof(ufbxi_line_curve_imp, curve) == sizeof(ufbxi_refcount));`
const _: () = assert!(core::mem::offset_of!(LineCurveImp, curve) == size_of::<Refcount>());

// SAFETY: `#[repr(C)]` with `refcount` leading, `LINE_CURVE_IMP_MAGIC` is the
// magic `ufbxi_get_imp(ufbxi_line_curve_imp, ...)` users check, `Payload` is
// the public struct at the pinned offset, and `header_parts` projects the two
// named fields of the passed `imp`. NOT gated, like the struct:
// `ufbx_free_line_curve`/`ufbx_retain_line_curve` recover it unconditionally.
unsafe impl ImpRecover for LineCurveImp {
    type Payload = LineCurve;
    const MAGIC: u32 = LINE_CURVE_IMP_MAGIC;

    #[inline(always)]
    unsafe fn header_parts(imp: *mut Self) -> (*mut Refcount, *mut u32) {
        // SAFETY: the caller vouches `imp` addresses a live `LineCurveImp`, so
        // these field projections stay inside that allocation.
        unsafe { (&raw mut (*imp).refcount, &raw mut (*imp).magic) }
    }
}

// SAFETY: `parts` projects the three named fields of the passed `imp` (layout
// pinned by the `offset_of` assert above).
#[cfg(feature = "tessellation")]
unsafe impl ImpHeader for LineCurveImp {
    #[inline(always)]
    unsafe fn parts(imp: *mut Self) -> (*mut Refcount, *mut Self::Payload, *mut u32) {
        // SAFETY: the caller vouches `imp` addresses a live `LineCurveImp`, so
        // these field projections stay inside that allocation.
        unsafe {
            (
                &raw mut (*imp).refcount,
                &raw mut (*imp).curve,
                &raw mut (*imp).magic,
            )
        }
    }
}

// ufbx.c:27799 `#if UFBXI_FEATURE_TESSELLATION`

// ufbx.c:27801-27817 `ufbxi_tessellate_curve_context`
#[cfg(feature = "tessellation")]
#[repr(C)]
pub(crate) struct InnerTessellateCurveContext {
    pub error: Error,

    pub opts: RawTessellateCurveOpts,

    pub curve: *const NurbsCurve,

    pub ator_tmp: Allocator,
    pub ator_result: Allocator,

    pub result: Buf,

    pub line: LineCurve,

    pub imp: *mut LineCurveImp,
}

// Safe `&TessellateCurveContext` handle over `InnerTessellateCurveContext`,
// mirroring the `Context`/`InnerContext` seam in `parse.rs`. `MaybeUninit` because
// it embeds the public `LineCurve` (enum-bearing) in `line`; `UnsafeCell` gives the
// interior mutability every `&TessellateCurveContext` site needs. Field is
// `pub(crate)` — the sole construction site lives in `native::api`.
// Typed interior-mutable view over the tessellation options.
pub(crate) type TessellateCurveOptsView =
    crate::native::view::View<crate::generated::RawTessellateCurveOpts>;

impl TessellateCurveOptsView {
    #[inline(always)]
    pub(crate) fn span_subdivision(&self) -> usize {
        view_read!(self, span_subdivision)
    }
    #[inline(always)]
    pub(crate) fn set_span_subdivision(&self, span_subdivision: usize) {
        view_write!(self, span_subdivision, span_subdivision)
    }
}

// Mode-generic nested views over the two allocator descriptors: `init_ator`
// only reads them, so the accessor serves a `Mut` context field and a `Const`
// boundary mint alike.
impl<M: crate::native::view::Mode> View<crate::generated::RawTessellateCurveOpts, M> {
    #[inline(always)]
    pub(crate) fn temp_allocator_view(&self) -> &View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, temp_allocator)
    }
    #[inline(always)]
    pub(crate) fn result_allocator_view(&self) -> &View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, result_allocator)
    }
}

// Typed interior-mutable view over the tessellation options.
pub(crate) type TessellateSurfaceOptsView =
    crate::native::view::View<crate::generated::RawTessellateSurfaceOpts>;

impl TessellateSurfaceOptsView {
    #[inline(always)]
    pub(crate) fn span_subdivision_u(&self) -> usize {
        view_read!(self, span_subdivision_u)
    }
    #[inline(always)]
    pub(crate) fn set_span_subdivision_u(&self, span_subdivision_u: usize) {
        view_write!(self, span_subdivision_u, span_subdivision_u)
    }
    #[inline(always)]
    pub(crate) fn span_subdivision_v(&self) -> usize {
        view_read!(self, span_subdivision_v)
    }
    #[inline(always)]
    pub(crate) fn set_span_subdivision_v(&self, span_subdivision_v: usize) {
        view_write!(self, span_subdivision_v, span_subdivision_v)
    }
    #[inline(always)]
    pub(crate) fn skip_mesh_parts(&self) -> bool {
        view_read!(self, skip_mesh_parts)
    }
}

// Mode-generic nested views over the two allocator descriptors (see the curve
// counterpart above).
impl<M: crate::native::view::Mode> View<crate::generated::RawTessellateSurfaceOpts, M> {
    #[inline(always)]
    pub(crate) fn temp_allocator_view(&self) -> &View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, temp_allocator)
    }
    #[inline(always)]
    pub(crate) fn result_allocator_view(&self) -> &View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, result_allocator)
    }
}

#[cfg(feature = "tessellation")]
#[repr(transparent)]
pub(crate) struct TessellateCurveContext(
    pub(crate) core::cell::UnsafeCell<core::mem::MaybeUninit<InnerTessellateCurveContext>>,
);

#[cfg(feature = "tessellation")]
impl TessellateCurveContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerTessellateCurveContext {
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
    pub(crate) fn opts_view(&self) -> &TessellateCurveOptsView {
        unsafe { &*(&raw mut (*self.get()).opts as *mut TessellateCurveOptsView) }
    }

    // `result` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn result_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).result as *mut crate::native::buf::BufView) }
    }

    // `line` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn line_mut_ptr(&self) -> *mut LineCurve {
        view_raw_mut!(self, line)
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

    // `ator_tmp` (Allocator) — typed VIEW handle (reinterpret-in-place); accessors on AllocatorView.
    #[inline(always)]
    pub(crate) fn ator_tmp_view(&self) -> &crate::native::allocator::AllocatorView {
        // SAFETY: reinterpret the owned Allocator field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).ator_tmp as *mut crate::native::allocator::AllocatorView)
        }
    }

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawTessellateCurveOpts {
        view_raw_mut!(self, opts)
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

    // `imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn imp(&self) -> *mut LineCurveImp {
        view_read!(self, imp)
    }

    #[inline(always)]
    pub(crate) fn set_imp(&self, imp: *mut LineCurveImp) {
        view_write!(self, imp, imp)
    }

    // `curve` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn curve(&self) -> *const NurbsCurve {
        view_read!(self, curve)
    }

    #[inline(always)]
    pub(crate) fn set_curve(&self, curve: *const NurbsCurve) {
        view_write!(self, curve, curve)
    }
}

// ufbx.c:27819-27838 `ufbxi_tessellate_surface_context`
#[cfg(feature = "tessellation")]
#[repr(C)]
pub(crate) struct InnerTessellateSurfaceContext {
    pub error: Error,

    pub opts: RawTessellateSurfaceOpts,

    pub surface: *const NurbsSurface,

    pub ator_tmp: Allocator,
    pub ator_result: Allocator,

    pub tmp: Buf,
    pub result: Buf,

    pub position_map: Map,

    pub mesh: Mesh,

    pub imp: *mut MeshImp,
}

// Safe `&TessellateSurfaceContext` handle over `InnerTessellateSurfaceContext`,
// mirroring the `Context`/`InnerContext` seam in `parse.rs`. `MaybeUninit` because
// it embeds the public `Mesh` (enum-bearing) in `mesh`; `UnsafeCell` gives the
// interior mutability every `&TessellateSurfaceContext` site needs. Field is
// `pub(crate)` — the sole construction site lives in `native::api`.
#[cfg(feature = "tessellation")]
#[repr(transparent)]
pub(crate) struct TessellateSurfaceContext(
    pub(crate) core::cell::UnsafeCell<core::mem::MaybeUninit<InnerTessellateSurfaceContext>>,
);

#[cfg(feature = "tessellation")]
impl TessellateSurfaceContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerTessellateSurfaceContext {
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
    pub(crate) fn opts_view(&self) -> &TessellateSurfaceOptsView {
        unsafe { &*(&raw mut (*self.get()).opts as *mut TessellateSurfaceOptsView) }
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

    // `mesh` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn mesh_mut_ptr(&self) -> *mut Mesh {
        view_raw_mut!(self, mesh)
    }

    // `position_map` (Map) — typed VIEW handle (reinterpret-in-place); accessors on MapView.
    #[inline(always)]
    pub(crate) fn position_map_view(&self) -> &crate::native::hash::MapView {
        // SAFETY: reinterpret the Map field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).position_map as *mut crate::native::hash::MapView) }
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

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawTessellateSurfaceOpts {
        view_raw_mut!(self, opts)
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

    // `imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn imp(&self) -> *mut MeshImp {
        view_read!(self, imp)
    }

    #[inline(always)]
    pub(crate) fn set_imp(&self, imp: *mut MeshImp) {
        view_write!(self, imp, imp)
    }

    // `surface` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn surface(&self) -> *const NurbsSurface {
        view_read!(self, surface)
    }

    #[inline(always)]
    pub(crate) fn set_surface(&self, surface: *const NurbsSurface) {
        view_write!(self, surface, surface)
    }
}

// ufbx.c:27840-27931 `ufbxi_tessellate_nurbs_curve_imp`
#[cfg(feature = "tessellation")]
#[inline(never)]
pub(crate) fn tessellate_nurbs_curve_imp(
    tc: &TessellateCurveContext,
) -> Result<FinishedImp<LineCurveImp>, Fail> {
    // C: `tc->opts.span_subdivision <= 0` — `span_subdivision` is `size_t`.
    if tc.opts_view().span_subdivision() == 0 {
        tc.opts_view().set_span_subdivision(4);
    }
    let num_sub: usize = tc.opts_view().span_subdivision();

    let curve = tc.curve();
    let line = tc.line_mut_ptr();
    // SAFETY: the context was constructed around this live input curve, whose
    // bytes remain read-only for the tessellation call.
    let curve_view = unsafe { View::<NurbsCurve, Const>::from_ptr(curve) };
    ufbxi_check_err_msg!(
        tc.error_view(),
        curve_view.basis().valid() && curve_view.control_points_view().count() > 0,
        "Bad NURBS geometry",
        "curve->basis.valid && curve->control_points.count > 0"
    );

    // Initializing tc's own two allocators from tc's own error slot and tc's
    // own opts allocator descriptors, named by `'static` NUL-terminated
    // literals.
    init_ator(
        tc.error_mut_ptr(),
        tc.ator_tmp_view(),
        Some(tc.opts_view().temp_allocator_view()),
        c"temp",
    );
    init_ator(
        tc.error_mut_ptr(),
        tc.ator_result_view(),
        Some(tc.opts_view().result_allocator_view()),
        c"result",
    );

    tc.result_view().set_unordered(true);
    // SAFETY: the empty result buffer and initialized result allocator are
    // fields of `tc`; the allocator remains live through result publication or
    // failure teardown, and all chunks are transferred with its state.
    unsafe { tc.result_view().set_ator(tc.ator_result_mut_ptr()) };

    let num_spans = curve_view.basis().spans_view().count();

    // Check conservatively that we don't overflow anything
    {
        let over_spans: usize = num_spans.wrapping_mul(2).wrapping_mul(size_of::<Real>());
        let over: usize = over_spans.wrapping_mul(num_sub);
        ufbxi_check_err!(
            tc.error_view(),
            !does_overflow(over, over_spans, num_sub),
            "!ufbxi_does_overflow(over, over_spans, num_sub)"
        );
    }

    let is_open = curve_view.basis().topology() == NurbsTopology::Open;

    let num_indices: usize = num_spans.wrapping_add(
        num_spans
            .wrapping_sub(1)
            .wrapping_mul(num_sub.wrapping_sub(1)),
    );
    let num_vertices: usize = num_indices.wrapping_sub(if is_open { 0 } else { 1 });
    ufbxi_check_err!(
        tc.error_view(),
        num_indices <= i32::MAX as usize,
        "num_indices <= INT32_MAX"
    );

    // Each result-buffer push checks its own element-size/count product.
    let (indices, vertices, segments): (*mut u32, *mut Vec3, *mut LineSegment) = (
        tc.result_view().push::<u32>(num_indices),
        tc.result_view().push::<Vec3>(num_vertices),
        tc.result_view().push::<LineSegment>(1),
    );
    ufbxi_check_err!(
        tc.error_view(),
        !indices.is_null() && !vertices.is_null() && !segments.is_null(),
        "indices && vertices && segments"
    );
    let spans = Run::from_list(curve_view.basis().spans_view());
    // SAFETY: each successful push checked its own byte size, and the combined
    // check above establishes all three fresh allocations.
    let (indices_write, vertices_write, segments_write) = unsafe {
        (
            Run::<u32>::from_raw_parts(indices, num_indices),
            Run::<Vec3>::from_raw_parts(vertices, num_vertices),
            Run::<LineSegment>::from_raw_parts(segments, 1),
        )
    };

    for span_ix in 0..num_spans {
        let num_splits: usize = if span_ix + 1 == num_spans { 1 } else { num_sub };

        for sub_ix in 0..num_splits {
            let ix: usize = span_ix * num_sub + sub_ix;

            if ix < num_vertices {
                let mut u = spans.copy_at(span_ix);
                if sub_ix > 0 {
                    let t: Real = sub_ix as Real / num_sub as Real;
                    u = u * (1.0f32 as Real - t) + t * spans.copy_at(span_ix + 1);
                }

                let point: CurvePoint = evaluate_nurbs_curve_view(Some(curve_view), u);
                vertices_write.write_at(ix, point.position);
                indices_write.write_at(ix, ix as u32);
            } else {
                indices_write.write_at(ix, 0);
            }
        }
    }

    let segment = segments_write.at(0);
    segment.set_index_begin(0);
    segment.set_num_indices(num_indices as u32);

    // SAFETY: `line` is the context's live, zero-initialized output slot; its
    // nested color field has the same write-capable provenance and lifetime.
    let (line_view, color) = unsafe {
        let line_view = View::<LineCurve>::from_ptr(line);
        (line_view, View::<Vec3>::from_ptr(line_view.color_raw()))
    };
    line_view.element().name_view().set(EMPTY_STRING.0);
    line_view.element().set_type(ElementType::LineCurve);
    line_view.element().set_typed_id(u32::MAX);
    line_view.element().set_element_id(u32::MAX);

    color.set_x(1.0f32 as Real);
    color.set_y(1.0f32 as Real);
    color.set_z(1.0f32 as Real);

    line_view.control_points_view().set_data(vertices);
    line_view.control_points_view().set_count(num_vertices);
    line_view.point_indices_view().set_data(indices);
    line_view.point_indices_view().set_count(num_indices);
    line_view.segments_view().set_data(segments);
    line_view.segments_view().set_count(1);

    line_view.set_from_tessellated_nurbs(true);

    tc.set_imp(tc.result_view().push::<LineCurveImp>(1));
    ufbxi_check_err!(tc.error_view(), !tc.imp().is_null(), "tc->imp");

    // C: `ufbxi_init_ref(...)` / `tc->imp->magic = ...` / `tc->imp->curve =
    // tc->line` / `tc->imp->refcount.ator = tc->ator_result` /
    // `tc->imp->refcount.buf = tc->result` — the shared imp-finalization group.
    //
    // SAFETY: `tc.imp()` is the fresh non-null push just checked above and the
    // last allocation of `tc->result`, so filling its header is writing our own
    // allocation; the parent refcount comes from the scene the input curve
    // belongs to, which owns that curve for the duration of this call — the
    // `Const` mint reads that scene reference out of the caller-supplied live
    // curve, whose bytes nothing writes here; and
    // `tc.line_mut_ptr()` is tc's own `LineCurve` slot — a distinct allocation
    // from the freshly pushed imp.
    let finished_imp = unsafe {
        finish_imp(
            tc.imp(),
            ImpHandle::<SceneImp>::from_payload(curve_view.element().scene().ptr()).refcount_ptr(),
            line,
            tc.ator_result(),
            tc.take_result(),
        )
    };

    Ok(finished_imp)
}

// ufbx.c:27933-28239 `ufbxi_tessellate_nurbs_surface_imp`
#[cfg(feature = "tessellation")]
#[inline(never)]
pub(crate) fn tessellate_nurbs_surface_imp(
    tc: &TessellateSurfaceContext,
) -> Result<FinishedImp<MeshImp>, Fail> {
    // C: `tc->opts.span_subdivision_u <= 0` — `span_subdivision_u/v` are `size_t`.
    if tc.opts_view().span_subdivision_u() == 0 {
        tc.opts_view().set_span_subdivision_u(4);
    }
    if tc.opts_view().span_subdivision_v() == 0 {
        tc.opts_view().set_span_subdivision_v(4);
    }

    let sub_u: usize = tc.opts_view().span_subdivision_u();
    let sub_v: usize = tc.opts_view().span_subdivision_v();

    let surface = tc.surface();
    let mesh = tc.mesh_mut_ptr();
    // SAFETY: the context was constructed around this live input surface,
    // whose bytes remain read-only for the tessellation call.
    let surface_view = unsafe { View::<NurbsSurface, Const>::from_ptr(surface) };
    ufbxi_check_err_msg!(
        tc.error_view(),
        surface_view.basis_u().valid()
            && surface_view.basis_v().valid()
            && surface_view.num_control_points_u() > 0
            && surface_view.num_control_points_v() > 0,
        "Bad NURBS geometry",
        "surface->basis_u.valid && surface->basis_v.valid && surface->num_control_points_u > 0 && surface->num_control_points_v > 0"
    );

    // Initializing tc's own two allocators from tc's own error slot and tc's
    // own opts allocator descriptors, named by `'static` NUL-terminated
    // literals.
    init_ator(
        tc.error_mut_ptr(),
        tc.ator_tmp_view(),
        Some(tc.opts_view().temp_allocator_view()),
        c"temp",
    );
    init_ator(
        tc.error_mut_ptr(),
        tc.ator_result_view(),
        Some(tc.opts_view().result_allocator_view()),
        c"result",
    );

    tc.result_view().set_unordered(true);
    tc.tmp_view().set_unordered(true);

    // SAFETY: these empty context-owned buffers are wired to their initialized
    // sibling allocators, which remain live through result transfer or failure
    // teardown. Each buffer's chunks are owned by the allocator stored here.
    unsafe {
        tc.result_view().set_ator(tc.ator_result_mut_ptr());
        tc.tmp_view().set_ator(tc.ator_tmp_mut_ptr());
    }

    let (open_u, open_v, spans_u, spans_v) = (
        surface_view.basis_u().topology() == NurbsTopology::Open,
        surface_view.basis_v().topology() == NurbsTopology::Open,
        surface_view.basis_u().spans_view().count(),
        surface_view.basis_v().spans_view().count(),
    );

    // Check conservatively that we don't overflow anything
    {
        let over_spans_u: usize = spans_u.wrapping_mul(2).wrapping_mul(size_of::<Real>());
        let over_spans_v: usize = spans_v.wrapping_mul(2).wrapping_mul(size_of::<Real>());
        let over_u: usize = over_spans_u.wrapping_mul(sub_u);
        let over_v: usize = over_spans_v.wrapping_mul(sub_v);
        let over_uv: usize = over_u.wrapping_mul(over_v);
        ufbxi_check_err!(
            tc.error_view(),
            !does_overflow(over_u, over_spans_u, sub_u),
            "!ufbxi_does_overflow(over_u, over_spans_u, sub_u)"
        );
        ufbxi_check_err!(
            tc.error_view(),
            !does_overflow(over_v, over_spans_v, sub_v),
            "!ufbxi_does_overflow(over_v, over_spans_v, sub_v)"
        );
        ufbxi_check_err!(
            tc.error_view(),
            !does_overflow(over_uv, over_u, over_v),
            "!ufbxi_does_overflow(over_uv, over_u, over_v)"
        );
    }

    let faces_u: usize = spans_u.wrapping_sub(1).wrapping_mul(sub_u);
    let faces_v: usize = spans_v.wrapping_sub(1).wrapping_mul(sub_v);

    let indices_u: usize =
        spans_u.wrapping_add(spans_u.wrapping_sub(1).wrapping_mul(sub_u.wrapping_sub(1)));
    let indices_v: usize =
        spans_v.wrapping_add(spans_v.wrapping_sub(1).wrapping_mul(sub_v.wrapping_sub(1)));

    let num_faces: usize = faces_u.wrapping_mul(faces_v);
    let num_indices: usize = indices_u.wrapping_mul(indices_v);
    ufbxi_check_err!(
        tc.error_view(),
        num_indices <= i32::MAX as usize,
        "num_indices <= INT32_MAX"
    );

    // The counts were just overflow-checked above.
    let position_ix: *mut u32 = tc.tmp_view().push::<u32>(num_indices);
    let (mut positions, mut normals, uvs, tangents, bitangents): (
        *mut Vec3,
        *mut Vec3,
        *mut Vec2,
        *mut Vec3,
        *mut Vec3,
    ) = (
        tc.result_view().push::<Vec3>(num_indices + 1),
        tc.result_view().push::<Vec3>(num_indices + 1),
        tc.result_view().push::<Vec2>(num_indices + 1),
        tc.result_view().push::<Vec3>(num_indices + 1),
        tc.result_view().push::<Vec3>(num_indices + 1),
    );
    ufbxi_check_err!(
        tc.error_view(),
        !position_ix.is_null() && !uvs.is_null() && !tangents.is_null() && !bitangents.is_null(),
        "position_ix && uvs && tangents && bitangents"
    );

    // SAFETY: this retains C's implicit allocation obligation: `positions`
    // and `normals` must be non-null here, although the first condition does
    // not encode that and allocation failure is not sticky. This inherited
    // boundary stays isolated pending a separate failure-order decision. The
    // other four pointers are checked above.
    let (position_ix_write, uvs_all, tangents_all, bitangents_all) = unsafe {
        *positions = ZERO_VEC3;
        positions = positions.add(1);
        *normals = ZERO_VEC3;
        normals = normals.add(1);
        (
            Run::<u32>::from_raw_parts(position_ix, num_indices),
            Run::<Vec2>::from_raw_parts(uvs, num_indices + 1),
            Run::<Vec3>::from_raw_parts(tangents, num_indices + 1),
            Run::<Vec3>::from_raw_parts(bitangents, num_indices + 1),
        )
    };

    // C: index 0 of each attribute array is a reserved zero element; the live
    // sampled data starts at the incremented pointer.
    uvs_all.write_at(0, ZERO_VEC2);
    let uvs_write = uvs_all.subrun(1, num_indices);
    let uvs = uvs_write.as_mut_ptr();
    tangents_all.write_at(0, ZERO_VEC3);
    let tangents_write = tangents_all.subrun(1, num_indices);
    let tangents = tangents_write.as_mut_ptr();
    bitangents_all.write_at(0, ZERO_VEC3);
    let bitangents_write = bitangents_all.subrun(1, num_indices);
    let bitangents = bitangents_write.as_mut_ptr();

    let mut num_positions: u32 = 0;

    for span_v in 0..spans_v {
        let splits_v: usize = if span_v + 1 == spans_v { 1 } else { sub_v };

        for split_v in 0..splits_v {
            let ix_v: usize = span_v * sub_v + split_v;
            ufbx_assert!(ix_v < indices_v);

            let spans_v_read = Run::from_list(surface_view.basis_v().spans_view());
            let mut v = spans_v_read.copy_at(span_v);
            if split_v > 0 {
                let t: Real = split_v as Real / splits_v as Real;
                v = v * (1.0f32 as Real - t) + t * spans_v_read.copy_at(span_v + 1);
            }
            let original_v: Real = v;
            if span_v + 1 == spans_v && !open_v {
                v = spans_v_read.copy_at(0);
            }

            for span_u in 0..spans_u {
                let splits_u: usize = if span_u + 1 == spans_u { 1 } else { sub_u };
                for split_u in 0..splits_u {
                    let ix_u: usize = span_u * sub_u + split_u;
                    ufbx_assert!(ix_u < indices_u);

                    let spans_u_read = Run::from_list(surface_view.basis_u().spans_view());
                    let mut u = spans_u_read.copy_at(span_u);
                    if split_u > 0 {
                        let t: Real = split_u as Real / splits_u as Real;
                        u = u * (1.0f32 as Real - t) + t * spans_u_read.copy_at(span_u + 1);
                    }
                    let original_u: Real = u;
                    if span_u + 1 == spans_u && !open_u {
                        u = spans_u_read.copy_at(0);
                    }

                    let (pos, tangent_u, tangent_v) = {
                        let point: SurfacePoint =
                            evaluate_nurbs_surface_view(Some(surface_view), u, v);
                        (
                            point.position,
                            slow_normalize3(&point.derivative_u),
                            slow_normalize3(&point.derivative_v),
                        )
                    };

                    // Check if there's any wrapped positions that we could match
                    let mut neighbors: [usize; 5] = [0; 5]; // C: ufbxi_uninit (only `[..num_neighbors]` is read)
                    let mut num_neighbors: usize = 0;

                    if (span_v == 0 && (span_u > 0 || split_u > 0))
                        || (span_u == 0 && (span_v > 0 || split_v > 0))
                    {
                        // Top/left
                        neighbors[num_neighbors] = 0;
                        num_neighbors += 1;
                    }
                    if span_v + 1 == spans_v {
                        // Bottom
                        neighbors[num_neighbors] = ix_u;
                        num_neighbors += 1;
                        if span_u > 0 || split_u > 0 {
                            neighbors[num_neighbors] = ix_v * indices_u;
                            num_neighbors += 1;
                        }
                    }
                    if span_u + 1 == spans_u {
                        // Right
                        neighbors[num_neighbors] = ix_v * indices_u;
                        num_neighbors += 1;
                        if span_v > 0 || split_v > 0 {
                            neighbors[num_neighbors] = indices_u - 1;
                            num_neighbors += 1;
                        }
                    }

                    let ix: usize = ix_v * indices_u + ix_u;

                    let mut pos_ix: u32 = num_positions;
                    for i in 0..num_neighbors {
                        let nb_ix: usize = neighbors[i];
                        ufbx_assert!(nb_ix < ix);
                        // SAFETY: raster order initialized the full `[0, ix)`
                        // position-index prefix, `nb_ix < ix`, and `Run::at()`
                        // bounds-checks the slot. The stored value is below
                        // `num_positions`; `positions` remains the inherited
                        // unchecked raw allocation boundary.
                        let (nb_pos_ix, nb_pos): (u32, Vec3) = unsafe {
                            let nb_pos_ix = position_ix_write.at(nb_ix).as_ptr().read();
                            (nb_pos_ix, *positions.add(nb_pos_ix as usize))
                        };
                        let dx: Real = nb_pos.x - pos.x;
                        let dy: Real = nb_pos.y - pos.y;
                        let dz: Real = nb_pos.z - pos.z;
                        let delta: Real = dx * dx + dy * dy + dz * dz;
                        if delta < 0.0000001f32 as Real {
                            // TODO: Configurable / something more rigorous
                            pos_ix = nb_pos_ix;
                            break;
                        }
                    }

                    position_ix_write.write_at(ix, pos_ix);
                    // SAFETY: `positions` is the inherited unchecked raw
                    // allocation boundary above. Dense first-seen indices keep
                    // `pos_ix == num_positions` within the requested capacity.
                    unsafe {
                        if pos_ix == num_positions {
                            *positions.add(pos_ix as usize) = pos;
                            num_positions = pos_ix + 1;
                        }
                    }
                    let uv = uvs_write.at(ix);
                    uv.set_x(original_u);
                    uv.set_y(original_v);
                    tangents_write.write_at(ix, tangent_u);
                    bitangents_write.write_at(ix, tangent_v);
                }
            }
        }
    }

    let (faces, vertex_ix, attrib_ix): (*mut Face, *mut u32, *mut u32) = (
        tc.result_view().push::<Face>(num_faces),
        tc.result_view().push::<u32>(num_faces.wrapping_mul(4)),
        tc.result_view().push::<u32>(num_faces.wrapping_mul(4)),
    );
    ufbxi_check_err!(
        tc.error_view(),
        !faces.is_null() && !vertex_ix.is_null() && !attrib_ix.is_null(),
        "faces && vertex_ix && attrib_ix"
    );
    let corner_capacity = num_faces.wrapping_mul(4);
    // SAFETY: the sampling loops initialized all `num_indices` tmp slots and
    // never write them again. Non-empty result pushes provide stable disjoint
    // slots; zero-count pushes produce empty runs that are never indexed.
    let (position_ix_read, faces_write, vertex_ix_write, attrib_ix_write) = unsafe {
        (
            Run::<u32, Const>::from_const_raw_parts(position_ix, num_indices),
            Run::<Face>::from_raw_parts(faces, num_faces),
            Run::<u32>::from_raw_parts(vertex_ix, corner_capacity),
            Run::<u32>::from_raw_parts(attrib_ix, corner_capacity),
        )
    };

    let mut face_ix: usize = 0;
    let mut dst_index: usize = 0;

    let mut num_triangles: usize = 0;

    for face_v in 0..faces_v {
        for face_u in 0..faces_u {
            let mut attrib = [0u32; 4];
            attrib[0] = ((face_v + 0) * indices_u + (face_u + 0)) as u32;
            attrib_ix_write.write_at(dst_index + 0, attrib[0]);
            attrib[1] = ((face_v + 0) * indices_u + (face_u + 1)) as u32;
            attrib_ix_write.write_at(dst_index + 1, attrib[1]);
            attrib[2] = ((face_v + 1) * indices_u + (face_u + 1)) as u32;
            attrib_ix_write.write_at(dst_index + 2, attrib[2]);
            attrib[3] = ((face_v + 1) * indices_u + (face_u + 0)) as u32;
            attrib_ix_write.write_at(dst_index + 3, attrib[3]);

            let mut vertex = [0u32; 4];
            vertex[0] = position_ix_read.copy_at(attrib[0] as usize);
            vertex_ix_write.write_at(dst_index + 0, vertex[0]);
            vertex[1] = position_ix_read.copy_at(attrib[1] as usize);
            vertex_ix_write.write_at(dst_index + 1, vertex[1]);
            vertex[2] = position_ix_read.copy_at(attrib[2] as usize);
            vertex_ix_write.write_at(dst_index + 2, vertex[2]);
            vertex[3] = position_ix_read.copy_at(attrib[3] as usize);
            vertex_ix_write.write_at(dst_index + 3, vertex[3]);

            let mut is_triangle = false;
            for prev_ix in 0..4usize {
                let next_ix = (prev_ix + 1) % 4;
                if vertex[prev_ix] == vertex[next_ix] {
                    for i in next_ix..3 {
                        attrib[i] = attrib[i + 1];
                        attrib_ix_write.write_at(dst_index + i, attrib[i]);
                        vertex[i] = vertex[i + 1];
                        vertex_ix_write.write_at(dst_index + i, vertex[i]);
                    }
                    is_triangle = true;
                    break;
                }
            }

            let face = faces_write.at(face_ix);
            face.set_index_begin(dst_index as u32);
            face.set_num_indices(if is_triangle { 3 } else { 4 });
            dst_index += if is_triangle { 3 } else { 4 };
            num_triangles += if is_triangle { 1 } else { 2 };
            face_ix += 1;
        }
    }

    ufbxi_check_err!(
        tc.error_view(),
        !positions.is_null() && !normals.is_null(),
        "positions && normals"
    );

    // SAFETY: `mesh` is tc's own mesh slot, reached through `*mut` off the
    // context (write-capable provenance for `Mut`) and live for the borrow;
    // every field accessed below was either filled above or is zero-valid from
    // tc's zeroed construction (`tessellate_nurbs_surface` creates the whole
    // context zeroed).
    let mesh_view = unsafe { View::<Mesh>::from_ptr(mesh) };
    mesh_view.element().name_view().set(EMPTY_STRING.0);
    mesh_view.element().set_type(ElementType::Mesh);
    mesh_view.element().set_typed_id(u32::MAX);
    mesh_view.element().set_element_id(u32::MAX);

    mesh_view.vertices_view().set_data(positions);
    mesh_view.vertices_view().set_count(num_positions as usize);
    mesh_view.set_num_vertices(num_positions as usize);
    mesh_view.vertex_indices_view().set_data(vertex_ix);
    mesh_view.vertex_indices_view().set_count(dst_index);

    mesh_view.faces_view().set_data(faces);
    mesh_view.faces_view().set_count(num_faces);

    mesh_view.vertex_position().set_exists(true);
    mesh_view
        .vertex_position()
        .values_view()
        .set_data(positions);
    mesh_view
        .vertex_position()
        .values_view()
        .set_count(num_positions as usize);
    mesh_view
        .vertex_position()
        .indices_view()
        .set_data(vertex_ix);
    mesh_view
        .vertex_position()
        .indices_view()
        .set_count(dst_index);
    mesh_view.vertex_position().set_unique_per_vertex(true);

    // C publishes the compacted corner count for these three parameter-grid
    // value lists. The headers may exceed the sampled storage, so only their
    // index lists are used as bounded runs in this path.
    mesh_view.vertex_uv().set_exists(true);
    mesh_view.vertex_uv().values_view().set_data(uvs);
    mesh_view.vertex_uv().values_view().set_count(dst_index);
    mesh_view.vertex_uv().indices_view().set_data(attrib_ix);
    mesh_view.vertex_uv().indices_view().set_count(dst_index);

    mesh_view.vertex_normal().set_exists(true);
    mesh_view.vertex_normal().values_view().set_data(normals);
    mesh_view
        .vertex_normal()
        .values_view()
        .set_count(num_positions as usize);
    mesh_view.vertex_normal().indices_view().set_data(vertex_ix);
    mesh_view
        .vertex_normal()
        .indices_view()
        .set_count(dst_index);

    mesh_view.vertex_tangent().set_exists(true);
    mesh_view.vertex_tangent().values_view().set_data(tangents);
    mesh_view
        .vertex_tangent()
        .values_view()
        .set_count(dst_index);
    mesh_view
        .vertex_tangent()
        .indices_view()
        .set_data(attrib_ix);
    mesh_view
        .vertex_tangent()
        .indices_view()
        .set_count(dst_index);

    mesh_view.vertex_bitangent().set_exists(true);
    mesh_view
        .vertex_bitangent()
        .values_view()
        .set_data(bitangents);
    mesh_view
        .vertex_bitangent()
        .values_view()
        .set_count(dst_index);
    mesh_view
        .vertex_bitangent()
        .indices_view()
        .set_data(attrib_ix);
    mesh_view
        .vertex_bitangent()
        .indices_view()
        .set_count(dst_index);

    mesh_view.set_num_faces(num_faces);
    mesh_view.set_num_triangles(num_triangles);
    mesh_view.set_num_indices(dst_index);
    mesh_view.set_max_face_triangles(2);

    if surface_view.material().is_some() {
        mesh_view
            .face_material_view()
            .set_data(tc.result_view().push_zero::<u32>(num_faces) as *const u32);
        ufbxi_check_err!(
            tc.error_view(),
            !mesh_view.face_material().data.is_null(),
            "mesh->face_material.data"
        );

        let mat: *mut *mut Material = tc.result_view().push_zero::<*mut Material>(1);
        ufbxi_check_err!(tc.error_view(), !mat.is_null(), "mat");

        // SAFETY: `mat` is the fresh non-null single-element push just
        // checked; the material it receives is the surface's own live ref.
        unsafe {
            *mat = surface_view
                .material()
                .map_or(core::ptr::null_mut(), |r| r.ptr());
        }
        mesh_view
            .materials_view()
            .set_data(mat as *const Ref<Material>);
        mesh_view.materials_view().set_count(1);
    }

    if !tc.opts_view().skip_mesh_parts() {
        mesh_view.material_parts_view().set_count(1);
        mesh_view
            .material_parts_view()
            .set_data(tc.result_view().push_zero::<MeshPart>(1) as *const MeshPart);
        ufbxi_check_err!(
            tc.error_view(),
            !mesh_view.material_parts().data.is_null(),
            "mesh->material_parts.data"
        );
    }

    finalize_mesh_material(tc.result_view(), tc.error_view(), mesh_view)?;
    finalize_mesh(tc.result_view(), tc.error_view(), mesh_view)?;

    // SAFETY: the normal computation operates on tc's own mesh slot;
    // `vertex_normal.values` is the `normals` push, whose `count` elements are
    // exactly the run `compute_normals` writes.
    unsafe {
        mesh_view.set_generated_normals(true);
        compute_normals(
            mesh_view.as_ptr(),
            mesh_view.vertex_position_ptr(),
            mesh_view.vertex_normal().indices().data,
            mesh_view.vertex_normal().indices().count,
            mesh_view.vertex_normal().values().data as *mut Vec3,
            mesh_view.vertex_normal().values().count,
        );
    }

    // `compute_normals` initialized the mesh-owned normal values before this
    // optional orientation pass.
    if surface_view.flip_normals() {
        // C: `ufbxi_nounroll ufbxi_for_list(ufbx_vec3, normal, mesh->vertex_normal.values)`
        let normals_write = Run::from_list(mesh_view.vertex_normal().values_view());
        for normal in normals_write.iter() {
            normal.set_x(normal.x() * (-1.0f32 as Real));
            normal.set_y(normal.y() * (-1.0f32 as Real));
            normal.set_z(normal.z() * (-1.0f32 as Real));
        }
    }

    tc.set_imp(tc.result_view().push::<MeshImp>(1));
    ufbxi_check_err!(tc.error_view(), !tc.imp().is_null(), "tc->imp");

    // C: `ufbxi_init_ref(...)` / `tc->imp->magic = ...` / `tc->imp->mesh =
    // tc->mesh` / `tc->imp->refcount.ator = tc->ator_result` /
    // `tc->imp->refcount.buf = tc->result` — the shared imp-finalization group.
    //
    // SAFETY: `tc.imp()` is the fresh non-null push just checked and the last
    // allocation of `tc->result`, so filling its header writes our own
    // allocation; the parent refcount comes from the scene owning the input
    // surface for the duration of this call — the `Const` mint reads that scene
    // reference out of the caller-supplied live surface, whose bytes nothing
    // writes here; and `tc.mesh_mut_ptr()` is tc's own
    // `Mesh` slot, a distinct allocation from the pushed imp.
    let finished_imp = unsafe {
        finish_imp(
            tc.imp(),
            ImpHandle::<SceneImp>::from_payload(surface_view.element().scene().ptr())
                .refcount_ptr(),
            mesh,
            tc.ator_result(),
            tc.take_result(),
        )
    };

    // SAFETY: the imp header is fully initialized just above, so its `mesh`
    // payload is a live `Mesh` this call owns.
    unsafe { (*tc.imp()).mesh.subdivision_evaluated = true };

    Ok(finished_imp)
}

// ufbx.c:28241 `#endif` (UFBXI_FEATURE_TESSELLATION)
