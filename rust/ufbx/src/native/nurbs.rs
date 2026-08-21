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
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]

#[cfg(feature = "tessellation")]
use core::ffi::c_void;
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
#[cfg(feature = "tessellation")]
use crate::native::allocator::{does_overflow, init_ator, Allocator, LINE_CURVE_IMP_MAGIC};
#[cfg(feature = "tessellation")]
use crate::native::api::{
    compute_normals, evaluate_nurbs_curve, evaluate_nurbs_surface, ZERO_VEC2, ZERO_VEC3,
};
#[cfg(feature = "tessellation")]
use crate::native::buf::Buf;
#[cfg(feature = "tessellation")]
use crate::native::error::Fail;
#[cfg(feature = "tessellation")]
use crate::native::error::{ufbxi_check_err, ufbxi_check_err_msg, EMPTY_CHAR};
#[cfg(feature = "tessellation")]
use crate::native::hash::Map;
use crate::native::parse::Refcount;
#[cfg(feature = "tessellation")]
use crate::native::parse::{finish_imp, get_imp, ImpHeader, MeshImp, SceneImp};
#[cfg(feature = "tessellation")]
use crate::native::platform::{add_ptr, ufbx_assert};
#[cfg(feature = "tessellation")]
use crate::native::read::{finalize_mesh, opt_ptr, ref_ptr};
#[cfg(feature = "tessellation")]
use crate::native::scene_process::finalize_mesh_material;
#[cfg(feature = "tessellation")]
use crate::native::string_pool::slow_normalize3;
#[cfg(feature = "tessellation")]
use crate::native::view::View;
#[cfg(feature = "tessellation")]
use crate::prelude::Ref;
use crate::prelude::{List, Real};

// ufbx.c:64-66 `UFBXI_MAX_NURBS_ORDER` (top-of-file config constant, owned by
// this section — only the NURBS evaluation entry points read it)
pub(crate) const MAX_NURBS_ORDER: usize = 128;

// ufbx.c:27771-27780 `ufbxi_nurbs_weight`
// C copies `ufbx_real_list` by value at the call sites and passes `&knots`;
// `List` is not `Copy`, so the Rust callers pass a pointer to the same data.
#[inline(always)]
pub(crate) unsafe fn nurbs_weight(
    knots: *const List<Real>,
    knot: usize,
    degree: usize,
    u: Real,
) -> Real {
    // SAFETY: caller passes a live, valid knot list (fn raw-param contract).
    let knots: &List<Real> = unsafe { &*knots };
    if knot >= knots.count {
        return 0.0f32 as Real;
    }
    if knots.count - knot < degree {
        return 0.0f32 as Real;
    }
    // SAFETY: both indices are within the knot run per the count/degree
    // early-outs above (same bounds discipline as the C code).
    let prev_u: Real = unsafe { *knots.data.add(knot) };
    let next_u: Real = unsafe { *knots.data.add(knot + degree) };
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
pub(crate) unsafe fn nurbs_deriv(knots: *const List<Real>, knot: usize, degree: usize) -> Real {
    // SAFETY: caller passes a live, valid knot list (fn raw-param contract).
    let knots: &List<Real> = unsafe { &*knots };
    if knot >= knots.count {
        return 0.0f32 as Real;
    }
    if knots.count - knot < degree {
        return 0.0f32 as Real;
    }
    // SAFETY: both indices are within the knot run per the count/degree
    // early-outs above (same bounds discipline as the C code).
    let prev_u: Real = unsafe { *knots.data.add(knot) };
    let next_u: Real = unsafe { *knots.data.add(knot + degree) };
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
// magic `ufbxi_get_imp(ufbxi_line_curve_imp, ...)` users check, and `parts`
// projects the three named fields of the passed `imp`.
#[cfg(feature = "tessellation")]
unsafe impl ImpHeader for LineCurveImp {
    type Payload = LineCurve;
    const MAGIC: u32 = LINE_CURVE_IMP_MAGIC;

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
// Typed interior-mutable VIEW over the tessellate opts (approach A).
pub(crate) type TessellateCurveOptsView =
    crate::native::view::View<crate::generated::RawTessellateCurveOpts>;

impl TessellateCurveOptsView {
    #[inline(always)]
    pub(crate) fn span_subdivision(&self) -> usize {
        unsafe { (*self.get()).span_subdivision }
    }
    #[inline(always)]
    pub(crate) fn set_span_subdivision(&self, span_subdivision: usize) {
        unsafe {
            (*self.get()).span_subdivision = span_subdivision;
        }
    }
    #[inline(always)]
    pub(crate) fn temp_allocator_ptr(&self) -> *const crate::generated::RawAllocatorOpts {
        unsafe { &raw const (*self.get()).temp_allocator }
    }
    #[inline(always)]
    pub(crate) fn result_allocator_ptr(&self) -> *const crate::generated::RawAllocatorOpts {
        unsafe { &raw const (*self.get()).result_allocator }
    }
}

// Typed interior-mutable VIEW over the tessellate opts (approach A).
pub(crate) type TessellateSurfaceOptsView =
    crate::native::view::View<crate::generated::RawTessellateSurfaceOpts>;

impl TessellateSurfaceOptsView {
    #[inline(always)]
    pub(crate) fn span_subdivision_u(&self) -> usize {
        unsafe { (*self.get()).span_subdivision_u }
    }
    #[inline(always)]
    pub(crate) fn set_span_subdivision_u(&self, span_subdivision_u: usize) {
        unsafe {
            (*self.get()).span_subdivision_u = span_subdivision_u;
        }
    }
    #[inline(always)]
    pub(crate) fn span_subdivision_v(&self) -> usize {
        unsafe { (*self.get()).span_subdivision_v }
    }
    #[inline(always)]
    pub(crate) fn set_span_subdivision_v(&self, span_subdivision_v: usize) {
        unsafe {
            (*self.get()).span_subdivision_v = span_subdivision_v;
        }
    }
    #[inline(always)]
    pub(crate) fn skip_mesh_parts(&self) -> bool {
        unsafe { (*self.get()).skip_mesh_parts }
    }
    #[inline(always)]
    pub(crate) fn temp_allocator_ptr(&self) -> *const crate::generated::RawAllocatorOpts {
        unsafe { &raw const (*self.get()).temp_allocator }
    }
    #[inline(always)]
    pub(crate) fn result_allocator_ptr(&self) -> *const crate::generated::RawAllocatorOpts {
        unsafe { &raw const (*self.get()).result_allocator }
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
        unsafe { (*self.get()).ator_result }
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
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).line }
    }

    // `result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn result_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).result }
    }

    // `ator_result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_result_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_result }
    }

    // `ator_tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_tmp_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_tmp }
    }

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawTessellateCurveOpts {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).opts }
    }

    // `error` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn error_mut_ptr(&self) -> *mut Error {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).error }
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
        // SAFETY: reading a scalar field; all bit patterns of `*mut LineCurveImp` are valid.
        unsafe { (*self.get()).imp }
    }

    #[inline(always)]
    pub(crate) fn set_imp(&self, imp: *mut LineCurveImp) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).imp = imp;
        }
    }

    // `curve` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn curve(&self) -> *const NurbsCurve {
        // SAFETY: reading a scalar field; all bit patterns of `*const NurbsCurve` are valid.
        unsafe { (*self.get()).curve }
    }

    #[inline(always)]
    pub(crate) fn set_curve(&self, curve: *const NurbsCurve) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).curve = curve;
        }
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
        unsafe { (*self.get()).ator_result }
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
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).mesh }
    }

    // `position_map` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn position_map_mut_ptr(&self) -> *mut Map {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).position_map }
    }

    // `tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp }
    }

    // `result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn result_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).result }
    }

    // `ator_result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_result_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_result }
    }

    // `ator_tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_tmp_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_tmp }
    }

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawTessellateSurfaceOpts {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).opts }
    }

    // `error` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn error_mut_ptr(&self) -> *mut Error {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).error }
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

    // `surface` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn surface(&self) -> *const NurbsSurface {
        // SAFETY: reading a scalar field; all bit patterns of `*const NurbsSurface` are valid.
        unsafe { (*self.get()).surface }
    }

    #[inline(always)]
    pub(crate) fn set_surface(&self, surface: *const NurbsSurface) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).surface = surface;
        }
    }
}

// ufbx.c:27840-27931 `ufbxi_tessellate_nurbs_curve_imp`
#[cfg(feature = "tessellation")]
#[inline(never)]
pub(crate) fn tessellate_nurbs_curve_imp(tc: &TessellateCurveContext) -> Result<(), Fail> {
    // C: `tc->opts.span_subdivision <= 0` — `span_subdivision` is `size_t`.
    if tc.opts_view().span_subdivision() == 0 {
        tc.opts_view().set_span_subdivision(4);
    }
    let num_sub: usize = tc.opts_view().span_subdivision();

    let curve: *const NurbsCurve = tc.curve();
    let line: *mut LineCurve = tc.line_mut_ptr();
    ufbxi_check_err_msg!(
        tc.error_view(),
        // SAFETY: `tc.curve()` is the curve the context was built around (tc
        // construction invariant), so reading its basis/control points is a
        // read of a live scene element.
        unsafe { (*curve).basis.valid && (*curve).control_points.count > 0 },
        "Bad NURBS geometry",
        "curve->basis.valid && curve->control_points.count > 0"
    );

    // SAFETY: initializing tc's own two allocators from tc's own error slot
    // and the caller's opts allocator descriptors, named by `'static`
    // NUL-terminated literals.
    unsafe {
        init_ator(
            tc.error_mut_ptr(),
            tc.ator_tmp_mut_ptr(),
            tc.opts_view().temp_allocator_ptr(),
            c"temp",
        );
        init_ator(
            tc.error_mut_ptr(),
            tc.ator_result_mut_ptr(),
            tc.opts_view().result_allocator_ptr(),
            c"result",
        );
    }

    tc.result_view().set_unordered(true);
    tc.result_view().set_ator(tc.ator_result_mut_ptr());

    // SAFETY: reading the live curve's basis (tc construction invariant).
    let num_spans: usize = unsafe { (*curve).basis.spans.count };

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

    // SAFETY: reading the live curve's basis (tc construction invariant).
    let is_open: bool = unsafe { (*curve).basis.topology } == NurbsTopology::Open;

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

    // The counts were just overflow-checked above.
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

    for span_ix in 0..num_spans {
        let num_splits: usize = if span_ix + 1 == num_spans { 1 } else { num_sub };

        for sub_ix in 0..num_splits {
            let ix: usize = span_ix * num_sub + sub_ix;

            if ix < num_vertices {
                // SAFETY: `span_ix < num_spans == basis.spans.count`, and the
                // `sub_ix > 0` arm only runs for a non-final span (the final
                // span has `num_splits == 1`), so `span_ix + 1` is in bounds
                // of the same span run. `ix < num_vertices <= num_indices`
                // keeps both stores within the fresh pushes above.
                unsafe {
                    let mut u: Real = *(*curve).basis.spans.data.add(span_ix);
                    if sub_ix > 0 {
                        let t: Real = sub_ix as Real / num_sub as Real;
                        u = u * (1.0f32 as Real - t)
                            + t * *(*curve).basis.spans.data.add(span_ix + 1);
                    }

                    let point: CurvePoint = evaluate_nurbs_curve(curve, u);
                    *vertices.add(ix) = point.position;
                    *indices.add(ix) = ix as u32;
                }
            } else {
                // SAFETY: `ix` peaks at `(num_spans - 1) * num_sub`, which is
                // `num_indices - 1`, so this stays inside the index push.
                unsafe { *indices.add(ix) = 0 };
            }
        }
    }

    // SAFETY: `segments` is the fresh non-null single-element push above.
    unsafe {
        (*segments.add(0)).index_begin = 0;
        (*segments.add(0)).num_indices = num_indices as u32;
    }

    // SAFETY: `tc.line_mut_ptr()` is tc's own output `LineCurve` slot (tc
    // construction invariant), not aliased for the rest of this function.
    let line: &mut LineCurve = unsafe { &mut *line };
    line.element.name.data = EMPTY_CHAR.as_ptr();
    line.element.type_ = ElementType::LineCurve;
    line.element.typed_id = u32::MAX;
    line.element.element_id = u32::MAX;

    line.color.x = 1.0f32 as Real;
    line.color.y = 1.0f32 as Real;
    line.color.z = 1.0f32 as Real;

    line.control_points.data = vertices as *const Vec3;
    line.control_points.count = num_vertices;
    line.point_indices.data = indices as *const u32;
    line.point_indices.count = num_indices;
    line.segments.data = segments as *const LineSegment;
    line.segments.count = 1;

    line.from_tessellated_nurbs = true;

    tc.set_imp(tc.result_view().push::<LineCurveImp>(1));
    ufbxi_check_err!(tc.error_view(), !tc.imp().is_null(), "tc->imp");

    // C: `ufbxi_init_ref(...)` / `tc->imp->magic = ...` / `tc->imp->curve =
    // tc->line` / `tc->imp->refcount.ator = tc->ator_result` /
    // `tc->imp->refcount.buf = tc->result` — the shared imp-finalization group.
    //
    // SAFETY: `tc.imp()` is the fresh non-null push just checked above and the
    // last allocation of `tc->result`, so filling its header is writing our own
    // allocation; the parent refcount comes from the scene the input curve
    // belongs to, which owns that curve for the duration of this call; and
    // `tc.line_mut_ptr()` is tc's own `LineCurve` slot — a distinct allocation
    // from the freshly pushed imp.
    unsafe {
        finish_imp(
            tc.imp(),
            &raw mut (*get_imp::<SceneImp>(ref_ptr(&(*curve).element.scene) as *mut c_void))
                .refcount,
            tc.line_mut_ptr(),
            tc.ator_result(),
            tc.take_result(),
        );
    }

    Ok(())
}

// ufbx.c:27933-28239 `ufbxi_tessellate_nurbs_surface_imp`
#[cfg(feature = "tessellation")]
#[inline(never)]
pub(crate) fn tessellate_nurbs_surface_imp(tc: &TessellateSurfaceContext) -> Result<(), Fail> {
    // C: `tc->opts.span_subdivision_u <= 0` — `span_subdivision_u/v` are `size_t`.
    if tc.opts_view().span_subdivision_u() == 0 {
        tc.opts_view().set_span_subdivision_u(4);
    }
    if tc.opts_view().span_subdivision_v() == 0 {
        tc.opts_view().set_span_subdivision_v(4);
    }

    let sub_u: usize = tc.opts_view().span_subdivision_u();
    let sub_v: usize = tc.opts_view().span_subdivision_v();

    let surface: *const NurbsSurface = tc.surface();
    let mesh: *mut Mesh = tc.mesh_mut_ptr();
    ufbxi_check_err_msg!(
        tc.error_view(),
        // SAFETY: `tc.surface()` is the surface the context was built around
        // (tc construction invariant) — a live scene element.
        unsafe {
            (*surface).basis_u.valid
                && (*surface).basis_v.valid
                && (*surface).num_control_points_u > 0
                && (*surface).num_control_points_v > 0
        },
        "Bad NURBS geometry",
        "surface->basis_u.valid && surface->basis_v.valid && surface->num_control_points_u > 0 && surface->num_control_points_v > 0"
    );

    // SAFETY: initializing tc's own two allocators from tc's own error slot
    // and the caller's opts allocator descriptors, named by `'static`
    // NUL-terminated literals.
    unsafe {
        init_ator(
            tc.error_mut_ptr(),
            tc.ator_tmp_mut_ptr(),
            tc.opts_view().temp_allocator_ptr(),
            c"temp",
        );
        init_ator(
            tc.error_mut_ptr(),
            tc.ator_result_mut_ptr(),
            tc.opts_view().result_allocator_ptr(),
            c"result",
        );
    }

    tc.result_view().set_unordered(true);
    tc.tmp_view().set_unordered(true);

    tc.result_view().set_ator(tc.ator_result_mut_ptr());
    tc.tmp_view().set_ator(tc.ator_tmp_mut_ptr());

    // SAFETY: reading the live surface's two bases (tc construction invariant).
    let (open_u, open_v, spans_u, spans_v) = unsafe {
        (
            (*surface).basis_u.topology == NurbsTopology::Open,
            (*surface).basis_v.topology == NurbsTopology::Open,
            (*surface).basis_u.spans.count,
            (*surface).basis_v.spans.count,
        )
    };

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
    let (mut positions, mut normals, mut uvs, mut tangents, mut bitangents): (
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

    // C: `*positions++ = ufbx_zero_vec3;` (index 0 of each attribute array is a
    // reserved zero element; the live data starts at the incremented pointer)
    // SAFETY: each of the five arrays holds `num_indices + 1` elements, so
    // element 0 exists and one past it is still in bounds. C-parity note: the
    // check above only names four of the five pushes (ufbx.c:27991), but
    // `positions`/`normals` are the same `Vec3 * (num_indices + 1)` request as
    // the checked `tangents`/`bitangents`, so a failure that nulls them nulls
    // those too.
    unsafe {
        *positions = ZERO_VEC3;
        positions = positions.add(1);
        *normals = ZERO_VEC3;
        normals = normals.add(1);
        *uvs = ZERO_VEC2;
        uvs = uvs.add(1);
        *tangents = ZERO_VEC3;
        tangents = tangents.add(1);
        *bitangents = ZERO_VEC3;
        bitangents = bitangents.add(1);
    }

    let mut num_positions: u32 = 0;

    for span_v in 0..spans_v {
        let splits_v: usize = if span_v + 1 == spans_v { 1 } else { sub_v };

        for split_v in 0..splits_v {
            let ix_v: usize = span_v * sub_v + split_v;
            ufbx_assert!(ix_v < indices_v);

            // SAFETY: `span_v < spans_v == basis_v.spans.count`; the
            // `split_v > 0` arm only runs for a non-final span (the final one
            // has `splits_v == 1`), so `span_v + 1` is in bounds too, and
            // index 0 exists because `spans_v > 0` guards this loop.
            let mut v: Real = unsafe { *(*surface).basis_v.spans.data.add(span_v) };
            if split_v > 0 {
                let t: Real = split_v as Real / splits_v as Real;
                v = v * (1.0f32 as Real - t)
                    + t * unsafe { *(*surface).basis_v.spans.data.add(span_v + 1) };
            }
            let original_v: Real = v;
            if span_v + 1 == spans_v && !open_v {
                v = unsafe { *(*surface).basis_v.spans.data.add(0) };
            }

            for span_u in 0..spans_u {
                let splits_u: usize = if span_u + 1 == spans_u { 1 } else { sub_u };
                for split_u in 0..splits_u {
                    let ix_u: usize = span_u * sub_u + split_u;
                    ufbx_assert!(ix_u < indices_u);

                    // SAFETY: `span_u < spans_u == basis_u.spans.count`; the
                    // `split_u > 0` arm only runs for a non-final span, so
                    // `span_u + 1` is in bounds, and index 0 exists because
                    // `spans_u > 0` guards this loop.
                    let mut u: Real = unsafe { *(*surface).basis_u.spans.data.add(span_u) };
                    if split_u > 0 {
                        let t: Real = split_u as Real / splits_u as Real;
                        u = u * (1.0f32 as Real - t)
                            + t * unsafe { *(*surface).basis_u.spans.data.add(span_u + 1) };
                    }
                    let original_u: Real = u;
                    if span_u + 1 == spans_u && !open_u {
                        u = unsafe { *(*surface).basis_u.spans.data.add(0) };
                    }

                    // SAFETY: evaluating the live surface (tc construction
                    // invariant) at a parameter taken from its own span range;
                    // the normalizations read locals.
                    let (pos, tangent_u, tangent_v) = unsafe {
                        let point: SurfacePoint = evaluate_nurbs_surface(surface, u, v);
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
                        // SAFETY: `nb_ix < ix < num_indices` (asserted above)
                        // indexes an already-written slot of the
                        // `num_indices`-element `position_ix` push, and every
                        // value stored there is `< num_positions <=
                        // num_indices`, which is in bounds of `positions`
                        // (`num_indices + 1` elements, advanced by one).
                        let (nb_pos_ix, nb_pos): (u32, Vec3) = unsafe {
                            let nb_pos_ix = *position_ix.add(nb_ix);
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

                    // SAFETY: `ix = ix_v * indices_u + ix_u < num_indices`
                    // (both factors asserted in range above), so it indexes
                    // the `num_indices`-element `position_ix` push and the
                    // `num_indices` live slots of `uvs`/`tangents`/
                    // `bitangents`. `pos_ix <= num_positions <= num_indices`
                    // likewise stays inside `positions`.
                    unsafe {
                        *position_ix.add(ix) = pos_ix;
                        if pos_ix == num_positions {
                            *positions.add(pos_ix as usize) = pos;
                            num_positions = pos_ix + 1;
                        }
                        (*uvs.add(ix)).x = original_u;
                        (*uvs.add(ix)).y = original_v;
                        *tangents.add(ix) = tangent_u;
                        *bitangents.add(ix) = tangent_v;
                    }
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

    let mut face_ix: usize = 0;
    let mut dst_index: usize = 0;

    let mut num_triangles: usize = 0;

    for face_v in 0..faces_v {
        for face_u in 0..faces_u {
            // SAFETY: each face consumes at most 4 slots and `dst_index`
            // advances by 3 or 4 per face, so `dst_index + 3` stays inside the
            // `4 * num_faces`-element `vertex_ix`/`attrib_ix` pushes, and
            // `face_ix < num_faces` indexes the `faces` push. The corner
            // indices written into `attrib_ix` peak at `num_indices - 1`
            // (`face_v + 1 <= indices_v - 1`, `face_u + 1 <= indices_u - 1`),
            // so using them to index the `num_indices`-element `position_ix`
            // is in bounds.
            let is_triangle: bool = unsafe {
                *attrib_ix.add(dst_index + 0) = ((face_v + 0) * indices_u + (face_u + 0)) as u32;
                *attrib_ix.add(dst_index + 1) = ((face_v + 0) * indices_u + (face_u + 1)) as u32;
                *attrib_ix.add(dst_index + 2) = ((face_v + 1) * indices_u + (face_u + 1)) as u32;
                *attrib_ix.add(dst_index + 3) = ((face_v + 1) * indices_u + (face_u + 0)) as u32;

                *vertex_ix.add(dst_index + 0) =
                    *position_ix.add(*attrib_ix.add(dst_index + 0) as usize);
                *vertex_ix.add(dst_index + 1) =
                    *position_ix.add(*attrib_ix.add(dst_index + 1) as usize);
                *vertex_ix.add(dst_index + 2) =
                    *position_ix.add(*attrib_ix.add(dst_index + 2) as usize);
                *vertex_ix.add(dst_index + 3) =
                    *position_ix.add(*attrib_ix.add(dst_index + 3) as usize);

                let mut is_triangle: bool = false;
                for prev_ix in 0..4usize {
                    let next_ix: usize = (prev_ix + 1) % 4;
                    if *vertex_ix.add(dst_index + prev_ix) == *vertex_ix.add(dst_index + next_ix) {
                        for i in next_ix..3 {
                            *attrib_ix.add(dst_index + i) = *attrib_ix.add(dst_index + i + 1);
                            *vertex_ix.add(dst_index + i) = *vertex_ix.add(dst_index + i + 1);
                        }
                        is_triangle = true;
                        break;
                    }
                }

                (*faces.add(face_ix)).index_begin = dst_index as u32;
                (*faces.add(face_ix)).num_indices = if is_triangle { 3 } else { 4 };
                is_triangle
            };
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

    {
        // SAFETY: `tc.mesh_mut_ptr()` is tc's own output `Mesh` slot (tc
        // construction invariant), unaliased here; the pointers stored into it
        // are the result-buf pushes above, which outlive the mesh because they
        // live in the same result arena.
        let mesh: &mut Mesh = unsafe { &mut *mesh };
        mesh.element.name.data = EMPTY_CHAR.as_ptr();
        mesh.element.type_ = ElementType::Mesh;
        mesh.element.typed_id = u32::MAX;
        mesh.element.element_id = u32::MAX;

        mesh.vertices.data = positions as *const Vec3;
        mesh.vertices.count = num_positions as usize;
        mesh.num_vertices = num_positions as usize;
        mesh.vertex_indices.data = vertex_ix as *const u32;
        mesh.vertex_indices.count = dst_index;

        mesh.faces.data = faces as *const Face;
        mesh.faces.count = num_faces;

        mesh.vertex_position.exists = true;
        mesh.vertex_position.values.data = positions as *const Vec3;
        mesh.vertex_position.values.count = num_positions as usize;
        mesh.vertex_position.indices.data = vertex_ix as *const u32;
        mesh.vertex_position.indices.count = dst_index;
        mesh.vertex_position.unique_per_vertex = true;

        mesh.vertex_uv.exists = true;
        mesh.vertex_uv.values.data = uvs as *const Vec2;
        mesh.vertex_uv.values.count = dst_index;
        mesh.vertex_uv.indices.data = attrib_ix as *const u32;
        mesh.vertex_uv.indices.count = dst_index;

        mesh.vertex_normal.exists = true;
        mesh.vertex_normal.values.data = normals as *const Vec3;
        mesh.vertex_normal.values.count = num_positions as usize;
        mesh.vertex_normal.indices.data = vertex_ix as *const u32;
        mesh.vertex_normal.indices.count = dst_index;

        mesh.vertex_tangent.exists = true;
        mesh.vertex_tangent.values.data = tangents as *const Vec3;
        mesh.vertex_tangent.values.count = dst_index;
        mesh.vertex_tangent.indices.data = attrib_ix as *const u32;
        mesh.vertex_tangent.indices.count = dst_index;

        mesh.vertex_bitangent.exists = true;
        mesh.vertex_bitangent.values.data = bitangents as *const Vec3;
        mesh.vertex_bitangent.values.count = dst_index;
        mesh.vertex_bitangent.indices.data = attrib_ix as *const u32;
        mesh.vertex_bitangent.indices.count = dst_index;

        mesh.num_faces = num_faces;
        mesh.num_triangles = num_triangles;
        mesh.num_indices = dst_index;
        mesh.max_face_triangles = 2;
    }

    // SAFETY: `mesh` is tc's own mesh slot, reached through `*mut` off the
    // context (write-capable provenance for `Mut`) and live for the borrow;
    // the fields accessed below were filled above.
    let mesh_view = unsafe { View::<Mesh>::from_ptr(mesh) };

    // SAFETY: reading the live surface's material ref (tc construction
    // invariant).
    if !unsafe { opt_ptr(&(*surface).material) }.is_null() {
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
            *mat = opt_ptr(&(*surface).material);
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

    // SAFETY: the finalizers and the normal computation all operate on tc's
    // own mesh slot, filled above, using tc's own result buf and error slot;
    // `vertex_normal.values` is the `normals` push, whose `count` elements are
    // exactly the run `compute_normals` writes.
    unsafe {
        finalize_mesh_material(tc.result_view(), tc.error_mut_ptr(), mesh_view.get())?;
        finalize_mesh(tc.result_view(), tc.error_mut_ptr(), mesh_view.get())?;

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

    // SAFETY: reading the live surface's flag, then walking the mesh's own
    // normal run (`data`/`count` set from the `normals` push above).
    if unsafe { (*surface).flip_normals } {
        // C: `ufbxi_nounroll ufbxi_for_list(ufbx_vec3, normal, mesh->vertex_normal.values)`
        unsafe {
            let mut normal: *mut Vec3 = mesh_view.vertex_normal().values().data as *mut Vec3;
            let normal_end: *mut Vec3 = add_ptr(normal, mesh_view.vertex_normal().values().count);
            while normal != normal_end {
                (*normal).x *= -1.0f32 as Real;
                (*normal).y *= -1.0f32 as Real;
                (*normal).z *= -1.0f32 as Real;
                normal = normal.add(1);
            }
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
    // surface for the duration of this call; and `tc.mesh_mut_ptr()` is tc's own
    // `Mesh` slot, a distinct allocation from the pushed imp.
    unsafe {
        finish_imp(
            tc.imp(),
            &raw mut (*get_imp::<SceneImp>(ref_ptr(&(*surface).element.scene) as *mut c_void))
                .refcount,
            tc.mesh_mut_ptr(),
            tc.ator_result(),
            tc.take_result(),
        );
    }

    // SAFETY: the imp header is fully initialized just above, so its `mesh`
    // payload is a live `Mesh` this call owns.
    unsafe { (*tc.imp()).mesh.subdivision_evaluated = true };

    Ok(())
}

// ufbx.c:28241 `#endif` (UFBXI_FEATURE_TESSELLATION)
