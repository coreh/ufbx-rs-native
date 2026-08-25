//! Port of the `// -- API` banner section of ufbx.c (ufbx.c:30333+), preceded
//! by the refcount lifecycle functions (`ufbxi_free_scene_imp`
//! ufbx.c:30243-30247 and `ufbxi_init_ref`/`ufbxi_retain_ref`/
//! `ufbxi_release_ref` ufbx.c:30249-30300 — C forward-declares the first two
//! next to `ufbxi_refcount` at ufbx.c:6229-6230 but defines them here).
//!
//! Complete coverage includes the `ufbx_abi_data` globals (ufbx.c:30339-30404), the
//! `ufbx_open_file`/`ufbx_open_memory`/`ufbx_default_open_file` plumbing
//! (ufbx.c:30406-30495), `ufbx_is_thread_safe` (30497-30500), the
//! `ufbx_load_*` family (ufbx.c:30502-30576, backed by `ufbxi_load` /
//! `ufbxi_load_imp` in `native::evaluate`),
//! `ufbx_free_scene`/`ufbx_retain_scene` (30578-30596) and
//! `ufbx_format_error` (30598-30633).
//!
//! Sequential coverage then continues over ufbx.c:30635-31176 — the
//! `ufbx_find_*` lookup family (30635-30825, including the String API
//! wrappers at 33142-33160 that delegate to it) and the animation-evaluation
//! entry points (30827-31176, backed by `native::evaluate`) are ported.
//! ufbx.c:31178-31721 is likewise covered in C order: `ufbx_evaluate_scene`
//! (both arms; the enabled arm under `feature = "scene-eval"`),
//! `ufbx_create_anim`, the anim / baked-anim refcount pairs, `ufbx_bake_anim`
//! (both arms; the enabled arm under `feature = "baking"`, backed by
//! `native::evaluate`), the baked lookup + `ufbx_evaluate_baked_*` sampling
//! functions and the whole quaternion math family are ported.
//! ufbx.c:31723-32095
//! is then covered in C order: the `ufbx_matrix_*` / `ufbx_transform_*` math
//! (31723-31926), `ufbx_catch_get_skin_vertex_matrix`
//! (31928-32018) and the blend-shape offset family (32020-32095). The NURBS
//! evaluation entry points (`ufbx_evaluate_nurbs_basis` / `_curve` /
//! `_surface`, ufbx.c:32097-32280) and both arms of the tessellation entry
//! points + `ufbx_free_line_curve` / `ufbx_retain_line_curve`
//! (ufbx.c:32282-32379, backed by `native::nurbs`) are ported.
//! ufbx.c:32381-32687 is then covered in C order: `ufbx_find_face_index`, the
//! `ufbx_catch_topo_*` / `ufbx_catch_get_weighted_face_normal` /
//! `ufbx_catch_compute_normals` geometry helpers and the `ufbx_free_*` /
//! `ufbx_retain_*` refcount pairs for meshes and geometry caches are ported;
//! triangulation, topology
//! (`compute_topology` / `generate_normal_mapping`), subdivision, and the
//! geometry-cache loaders are fully ported (backed by `native/topology.rs`,
//! `native/subdivision.rs`, `native/cache.rs`), with the `#else`
//! FEATURE_DISABLED arms under their cfgs.
//! The `ufbx_catch_*` non-catch wrappers at ufbx.c:33165-33179 are pulled
//! forward alongside the `ufbx_find_*` string wrappers (33142-33160), each
//! riding its catch impl's cfg.
//!
//! HIGHEST-STAKES invariants (PORTING.md "Allocator + ufbxi_buf" /
//! "Atomics / refcount"):
//! - `ufbxi_release_ref` free order is VERBATIM: stack-copy `ator` and `buf`,
//!   re-point `buf.ator` to the STACK copy, then `buf_free` + `free_ator` —
//!   the `ufbxi_refcount` header lives inside the buffer being freed.
//! - The parent-chain walk is an ITERATIVE loop, not recursion.
//! - The counter starts at 0 (`init_ref` does no self-retain); inc/dec return
//!   the PREVIOUS value (SeqCst); the object is freed when the previous value
//!   was 0 (`if dec(...) > 0 { return }`).
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
use core::ffi::c_void;
use core::mem::{size_of, MaybeUninit};

use crate::generated::RawBakeOpts;
use crate::generated::{
    Anim, AnimCurve, AnimLayer, AnimProp, AnimStack, AnimValue, AudioClip, AudioLayer, BakedAnim,
    BakedElement, BakedKeyFlags, BakedNode, BakedQuat, BakedVec3, BlendChannel, BlendDeformer,
    BlendKeyframe, BlendShape, Bone, BonePose, CacheChannel, CacheDeformer, CacheFile, CacheFrame,
    Camera, CameraSwitcher, Character, Constraint, CoordinateAxes, CoordinateAxis, CurvePoint,
    DisplayLayer, DomNode, DomValue, DomValueType, Element, ElementType, Empty, Error, ErrorFrame,
    ErrorType, Face, GeometryCache, Light, LineCurve, LodGroup, Marker, Material, MaterialTexture,
    Matrix, Mesh, MetadataObject, Node, NurbsBasis, NurbsCurve, NurbsSurface, NurbsTrimBoundary,
    NurbsTrimSurface, OpenFileInfo, Panic, Pose, ProceduralGeometry, Prop, Props, Quat,
    RawAllocatorOpts, RawGeometryCacheDataOpts, RawGeometryCacheOpts, RawLoadOpts, RawOpenFileOpts,
    RawOpenMemoryOpts, RawStream, RawVertexStream, RotationOrder, Scene, SelectionNode,
    SelectionSet, Shader, ShaderBinding, ShaderPropBinding, ShaderTexture, ShaderTextureInput,
    SkinCluster, SkinDeformer, SkinVertex, SkinWeight, StereoCamera, SurfacePoint, Texture,
    TopoEdge, Transform, Unknown, Vec2, Vec3, Vec4, VertexReal, VertexVec2, VertexVec3, VertexVec4,
    Video,
};
#[cfg(feature = "geometry-cache")]
use crate::generated::{CacheDataEncoding, CacheDataFormat, OpenFileType};
use crate::generated::{
    EvaluateFlags, Interpolation, PropFlags, RawAnimOpts, RawEvaluateOpts, TransformFlags,
};
#[cfg(feature = "tessellation")]
use crate::generated::{RawTessellateCurveOpts, RawTessellateSurfaceOpts};
#[cfg(feature = "baking")]
use crate::native::allocator::free;
use crate::native::allocator::{
    align_to_mask, alloc, free_ator, Allocator, CACHE_IMP_MAGIC, REFCOUNT_IMP_MAGIC,
    SCENE_IMP_MAGIC,
};
use crate::native::buf::{buf_free, Buf};
use crate::native::cache::{free_geometry_cache_imp, GeometryCacheImp};
#[cfg(feature = "geometry-cache")]
use crate::native::error::ufbxi_check_opts_return_no_error;
use crate::native::error::{
    strlen, ufbxi_panicf, ufbxi_snprintf, EMPTY_CHAR, ERROR_INFO_LENGTH, ERROR_STACK_MAX_DEPTH,
};
#[cfg(feature = "geometry-cache")]
use crate::native::platform::{min64, to_size, MAX_SKIP_SIZE};
use crate::native::thread::ThreadPool;
use crate::native::view::view_read_shared;
use crate::native::view::{Const, Mode, Mut, View};
// Used by the feature-enabled arms of `ufbx_bake_anim` /
// `ufbx_tessellate_nurbs_curve` / `_surface` and unconditionally by
// `ufbx_subdivide_mesh` / `ufbx_load_geometry_cache_len`.
use crate::native::error::fix_error_type;
use crate::native::error::ufbxi_check_opts_res;
#[cfg(any(
    not(feature = "scene-eval"),
    not(feature = "baking"),
    not(feature = "tessellation")
))]
use crate::native::error::{ufbxi_fmt_err_info, ufbxi_report_err_msg};
use crate::native::evaluate;
use crate::native::evaluate::BakedAnimImp;
#[cfg(feature = "tessellation")]
use crate::native::hash::map_free;
use crate::native::io::{
    begin_file_context, end_file_context, memory_close, memory_read, memory_size, memory_skip,
    stdio_init, stdio_open, FileContext, MemoryStream,
};
use crate::native::nurbs::{nurbs_deriv, nurbs_weight, LineCurveImp, MAX_NURBS_ORDER};
#[cfg(feature = "tessellation")]
use crate::native::nurbs::{
    tessellate_nurbs_curve_imp, tessellate_nurbs_surface_imp, TessellateCurveContext,
    TessellateSurfaceContext,
};
use crate::native::parse::{
    find_enum, find_real as ufbxi_find_real, get_name_key, Context, ImpHandle, InnerContext,
    MeshImp, Refcount, SceneImp, ELEMENT_TYPE_COUNT,
};
use crate::native::platform::{
    add_ptr, atomic_counter_dec, atomic_counter_free, atomic_counter_inc, atomic_counter_init,
    macro_lower_bound_eq, math, min_sz, ufbx_assert, ufbxi_ignore, ufbxi_unreachable, NO_INDEX,
    SOURCE_VERSION, THREAD_SAFE,
};
use crate::native::read::{opt_ptr, ref_ptr};
use crate::native::scene_process::{
    add_weighted_mat, add_weighted_quat, add_weighted_vec3, cmp_name_element_less_ref,
    cmp_prop_less_concat, cmp_prop_less_ref, fetch_dst_element, get_rotation, get_scale,
    get_transform, mul_quat, AnimImp,
};
use crate::native::string_pool as sp;
use crate::native::string_pool::{
    add3, concat_str_cmp, cross3, get_concat_key, length3, lerp3, mul3, normalize3, safe_string,
    str_equal, str_less, sub3, DEG_TO_RAD_DOUBLE, DPI, ONE_VEC3, RAD_TO_DEG_DOUBLE,
};
// `ufbxi_dot3` is only reached from the `#if UFBXI_FEATURE_TRIANGULATION` arm of
// `ufbx_catch_triangulate_face`.
#[cfg(feature = "triangulation")]
use crate::native::string_pool::dot3;
use crate::native::topology::is_edge_smooth;
use crate::prelude::as_f64;
use crate::prelude::{Blob, List, OpenFileContext, Real, String, ThreadPoolContext};

// ufbx.c:30243-30247 `ufbxi_free_scene_imp`
#[inline(never)]
pub(crate) unsafe fn free_scene_imp(imp: *mut SceneImp) {
    // SAFETY: `imp` points at a live `SceneImp` — the raw-pointer contract of
    // this `unsafe fn`.
    ufbx_assert!(unsafe { (*imp).magic } == SCENE_IMP_MAGIC);
    // SAFETY: same live `SceneImp`; `string_buf` is its own field.
    unsafe { buf_free(&raw mut (*imp).string_buf) };
}

// ufbx.c:30249-30259 `ufbxi_init_ref`
#[inline(never)]
pub(crate) unsafe fn init_ref(refcount: *mut Refcount, magic: u32, parent: *mut Refcount) {
    if !parent.is_null() {
        // SAFETY: `parent` is non-null here and points at a live `Refcount` —
        // the raw-pointer contract of this `unsafe fn`.
        unsafe { retain_ref(parent) };
    }

    // SAFETY: `refcount` points at a live `Refcount` — the raw-pointer contract
    // of this `unsafe fn`; `refcount.refcount` is its own atomic-counter field.
    unsafe { atomic_counter_init(&raw mut (*refcount).refcount) };
    // SAFETY: same live `Refcount`; writing its own field.
    unsafe {
        (*refcount).self_magic = REFCOUNT_IMP_MAGIC;
        (*refcount).type_magic = magic;
        (*refcount).parent = parent;
    }
}

// ufbx.c:30261-30267 `ufbxi_retain_ref`
#[inline(never)]
pub(crate) unsafe fn retain_ref(refcount: *mut Refcount) {
    // SAFETY: `refcount` points at a live `Refcount` — the raw-pointer contract
    // of this `unsafe fn`.
    ufbx_assert!(unsafe { (*refcount).self_magic } == REFCOUNT_IMP_MAGIC);
    // SAFETY: same live `Refcount`; `refcount.refcount` is its own atomic field.
    let count: usize = unsafe { atomic_counter_inc(&raw mut (*refcount).refcount) };
    ufbxi_ignore!(count);
    ufbx_assert!(count < usize::MAX / 2);
}

// ufbx.c:30269-30300 `ufbxi_release_ref`
#[inline(never)]
pub(crate) unsafe fn release_ref(mut refcount: *mut Refcount) {
    while !refcount.is_null() {
        // SAFETY: `refcount` is non-null here and points at a live `Refcount` —
        // the raw-pointer contract of this `unsafe fn`.
        ufbx_assert!(unsafe { (*refcount).self_magic } == REFCOUNT_IMP_MAGIC);
        // SAFETY: same live `Refcount`; `refcount.refcount` is its own atomic.
        unsafe {
            if atomic_counter_dec(&raw mut (*refcount).refcount) > 0 {
                return;
            }
            atomic_counter_free(&raw mut (*refcount).refcount);
        }

        // SAFETY: same live `Refcount`; reading its own fields.
        let (parent, type_magic) = unsafe { ((*refcount).parent, (*refcount).type_magic) };

        // SAFETY: as above; writing its own fields.
        unsafe {
            (*refcount).self_magic = 0;
            (*refcount).type_magic = 0;
        }

        // Type-specific cleanup
        match type_magic {
            // SAFETY: the `Refcount` prefixes a `SceneImp` when its type magic is
            // `SCENE_IMP_MAGIC`, so the cast pointer is a live `SceneImp`.
            SCENE_IMP_MAGIC => unsafe { free_scene_imp(refcount as *mut SceneImp) },
            // `free_geometry_cache_imp` is an `unsafe fn` only when the
            // geometry-cache feature is enabled; the `#else` build is a safe stub.
            // SAFETY: the `Refcount` prefixes a `GeometryCacheImp` when its type
            // magic is `CACHE_IMP_MAGIC`, so the cast pointer is a live one.
            #[cfg(feature = "geometry-cache")]
            CACHE_IMP_MAGIC => unsafe {
                free_geometry_cache_imp(refcount as *mut GeometryCacheImp)
            },
            #[cfg(not(feature = "geometry-cache"))]
            CACHE_IMP_MAGIC => free_geometry_cache_imp(refcount as *mut GeometryCacheImp),
            _ => {}
        }

        // We need to free `data_buf` last and be careful to copy it to
        // the stack since the `ufbxi_refcount` that contains it is allocated
        // from the same result buffer!
        // SAFETY: same live `Refcount`; `ator` is a `Copy` field read by value.
        let mut ator: Allocator = unsafe { (*refcount).ator };
        // `Buf` is not `Copy`; this stack copy is the deliberate ownership move
        // the comment above describes (`ptr::read` = C struct assignment).
        // SAFETY: `&raw const (*refcount).buf` addresses the live `buf` field of
        // the same `Refcount`; `ptr::read` moves it out by value once.
        let mut buf: Buf = unsafe { core::ptr::read(&raw const (*refcount).buf) };
        buf.ator = &raw mut ator;
        // SAFETY: `buf` is the just-moved `Buf`, now owning its stack `ator`.
        unsafe { buf_free(&raw mut buf) };
        // SAFETY: `ator` is the stack copy of the refcount's allocator.
        unsafe { free_ator(&raw mut ator) };

        refcount = parent;
    }
}

// -- API (ufbx.c:30333)

// The `ufbx_abi_data_def` globals below (ufbx.c:30339-30404). C has exactly
// ONE object per global — internal code reads the same object the public
// header exports — so under `feature = "c-abi"` these impls ARE the exports
// (`export_name`, no separate definition in `capi.rs`), mirroring
// `ufbx_default_open_file`.

// ufbx.c:30339 `ufbx_abi_data_def const ufbx_string ufbx_empty_string = { ufbxi_empty_char, 0 };`
// `ufbx_string` holds a raw pointer (not auto-`Sync`); the datum is immutable
// and points at an immutable static, so sharing is sound. Wrapper struct
// mirrors `native::string_pool::StringTable`.
#[repr(transparent)]
pub(crate) struct EmptyString(pub String);
unsafe impl Sync for EmptyString {}
#[cfg_attr(feature = "c-abi", export_name = "ufbx_empty_string")]
pub static EMPTY_STRING: EmptyString = EmptyString(String::new_c(EMPTY_CHAR.as_ptr(), 0));

// ufbx.c:30340 `ufbx_abi_data_def const ufbx_blob ufbx_empty_blob = { NULL, 0 };`
// Same `Sync` wrapper rationale as `EMPTY_STRING` above.
#[repr(transparent)]
pub(crate) struct EmptyBlob(pub Blob);
unsafe impl Sync for EmptyBlob {}
#[cfg_attr(feature = "c-abi", export_name = "ufbx_empty_blob")]
pub static EMPTY_BLOB: EmptyBlob = EmptyBlob(Blob::new_c(core::ptr::null(), 0));

// ufbx.c:30341 `ufbx_abi_data_def const ufbx_matrix ufbx_identity_matrix = { 1,0,0, 0,1,0, 0,0,1, 0,0,0 };`
// Plain `Real` fields, so no `Sync` wrapper is needed (unlike `EMPTY_STRING`).
#[cfg_attr(feature = "c-abi", export_name = "ufbx_identity_matrix")]
pub static IDENTITY_MATRIX: Matrix = Matrix {
    m00: 1.0,
    m10: 0.0,
    m20: 0.0,
    m01: 0.0,
    m11: 1.0,
    m21: 0.0,
    m02: 0.0,
    m12: 0.0,
    m22: 1.0,
    m03: 0.0,
    m13: 0.0,
    m23: 0.0,
};

// ufbx.c:30342 `ufbx_abi_data_def const ufbx_transform ufbx_identity_transform = { {0,0,0}, {0,0,0,1}, {1,1,1} };`
#[cfg_attr(feature = "c-abi", export_name = "ufbx_identity_transform")]
pub static IDENTITY_TRANSFORM: Transform = Transform {
    translation: Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    },
    rotation: Quat {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    },
    scale: Vec3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    },
};

// ufbx.c:30343 `ufbx_abi_data_def const ufbx_vec2 ufbx_zero_vec2 = { 0,0 };`
#[cfg_attr(feature = "c-abi", export_name = "ufbx_zero_vec2")]
pub static ZERO_VEC2: Vec2 = Vec2 { x: 0.0, y: 0.0 };

// ufbx.c:30344 `ufbx_abi_data_def const ufbx_vec3 ufbx_zero_vec3 = { 0,0,0 };`
// Plain `Real` fields, so no `Sync` wrapper is needed (see `IDENTITY_MATRIX`).
#[cfg_attr(feature = "c-abi", export_name = "ufbx_zero_vec3")]
pub static ZERO_VEC3: Vec3 = Vec3 {
    x: 0.0,
    y: 0.0,
    z: 0.0,
};

// ufbx.c:30345 `ufbx_abi_data_def const ufbx_vec4 ufbx_zero_vec4 = { 0,0,0,0 };`
#[cfg_attr(feature = "c-abi", export_name = "ufbx_zero_vec4")]
pub static ZERO_VEC4: Vec4 = Vec4 {
    x: 0.0,
    y: 0.0,
    z: 0.0,
    w: 0.0,
};

// ufbx.c:30346 `ufbx_abi_data_def const ufbx_quat ufbx_identity_quat = { 0,0,0,1 };`
#[cfg_attr(feature = "c-abi", export_name = "ufbx_identity_quat")]
pub static IDENTITY_QUAT: Quat = Quat {
    x: 0.0,
    y: 0.0,
    z: 0.0,
    w: 1.0,
};

// ufbx.c:30348-30350 `ufbx_abi_data_def const ufbx_coordinate_axes ufbx_axes_right_handed_y_up`
#[cfg_attr(feature = "c-abi", export_name = "ufbx_axes_right_handed_y_up")]
pub static AXES_RIGHT_HANDED_Y_UP: CoordinateAxes = CoordinateAxes {
    right: CoordinateAxis::PositiveX,
    up: CoordinateAxis::PositiveY,
    front: CoordinateAxis::PositiveZ,
};

// ufbx.c:30351-30353 `ufbx_abi_data_def const ufbx_coordinate_axes ufbx_axes_right_handed_z_up`
#[cfg_attr(feature = "c-abi", export_name = "ufbx_axes_right_handed_z_up")]
pub static AXES_RIGHT_HANDED_Z_UP: CoordinateAxes = CoordinateAxes {
    right: CoordinateAxis::PositiveX,
    up: CoordinateAxis::PositiveZ,
    front: CoordinateAxis::NegativeY,
};

// ufbx.c:30354-30356 `ufbx_abi_data_def const ufbx_coordinate_axes ufbx_axes_left_handed_y_up`
#[cfg_attr(feature = "c-abi", export_name = "ufbx_axes_left_handed_y_up")]
pub static AXES_LEFT_HANDED_Y_UP: CoordinateAxes = CoordinateAxes {
    right: CoordinateAxis::PositiveX,
    up: CoordinateAxis::PositiveY,
    front: CoordinateAxis::NegativeZ,
};

// ufbx.c:30357-30359 `ufbx_abi_data_def const ufbx_coordinate_axes ufbx_axes_left_handed_z_up`
#[cfg_attr(feature = "c-abi", export_name = "ufbx_axes_left_handed_z_up")]
pub static AXES_LEFT_HANDED_Z_UP: CoordinateAxes = CoordinateAxes {
    right: CoordinateAxis::PositiveX,
    up: CoordinateAxis::PositiveZ,
    front: CoordinateAxis::PositiveY,
};

// ufbx.c:30361-30404 `ufbx_abi_data_def const size_t ufbx_element_type_size[UFBX_ELEMENT_TYPE_COUNT]`
// C `sizeof(ufbx_foo)` per entry, in `ufbx_element_type` order; the array
// length tracks `ELEMENT_TYPE_COUNT` so an upstream element type addition
// fails to compile here rather than silently truncating.
#[cfg_attr(feature = "c-abi", export_name = "ufbx_element_type_size")]
pub(crate) static ELEMENT_TYPE_SIZE: [usize; ELEMENT_TYPE_COUNT] = [
    size_of::<crate::generated::Unknown>(),
    size_of::<crate::generated::Node>(),
    size_of::<crate::generated::Mesh>(),
    size_of::<crate::generated::Light>(),
    size_of::<crate::generated::Camera>(),
    size_of::<crate::generated::Bone>(),
    size_of::<crate::generated::Empty>(),
    size_of::<crate::generated::LineCurve>(),
    size_of::<crate::generated::NurbsCurve>(),
    size_of::<crate::generated::NurbsSurface>(),
    size_of::<crate::generated::NurbsTrimSurface>(),
    size_of::<crate::generated::NurbsTrimBoundary>(),
    size_of::<crate::generated::ProceduralGeometry>(),
    size_of::<crate::generated::StereoCamera>(),
    size_of::<crate::generated::CameraSwitcher>(),
    size_of::<crate::generated::Marker>(),
    size_of::<crate::generated::LodGroup>(),
    size_of::<crate::generated::SkinDeformer>(),
    size_of::<crate::generated::SkinCluster>(),
    size_of::<crate::generated::BlendDeformer>(),
    size_of::<crate::generated::BlendChannel>(),
    size_of::<crate::generated::BlendShape>(),
    size_of::<crate::generated::CacheDeformer>(),
    size_of::<crate::generated::CacheFile>(),
    size_of::<crate::generated::Material>(),
    size_of::<crate::generated::Texture>(),
    size_of::<crate::generated::Video>(),
    size_of::<crate::generated::Shader>(),
    size_of::<crate::generated::ShaderBinding>(),
    size_of::<crate::generated::AnimStack>(),
    size_of::<crate::generated::AnimLayer>(),
    size_of::<crate::generated::AnimValue>(),
    size_of::<crate::generated::AnimCurve>(),
    size_of::<crate::generated::DisplayLayer>(),
    size_of::<crate::generated::SelectionSet>(),
    size_of::<crate::generated::SelectionNode>(),
    size_of::<crate::generated::Character>(),
    size_of::<crate::generated::Constraint>(),
    size_of::<crate::generated::AudioLayer>(),
    size_of::<crate::generated::AudioClip>(),
    size_of::<crate::generated::Pose>(),
    size_of::<crate::generated::MetadataObject>(),
];

// ufbx.c:30406-30410 `ufbx_default_open_file`
// `extern "C"`: this exact function pointer is stored into
// `ufbx_open_file_cb.fn` defaults and compared by address (ufbx.c:24645,
// 25224, 25532, 32712). C has exactly ONE address for it — the exported
// symbol — so under `feature = "c-abi"` this impl IS the export
// (`export_name`, no shim in `capi.rs`): a C caller that assigns
// `ufbx_default_open_file` into a callback must pass the loader's
// compare-by-address fast path (ufbx.c:25224) exactly as in C.
#[cfg_attr(feature = "c-abi", export_name = "ufbx_default_open_file")]
pub unsafe extern "C" fn default_open_file(
    user: *mut c_void,
    stream: *mut RawStream,
    path: *const u8,
    path_len: usize,
    info: *const OpenFileInfo,
) -> bool {
    let _ = user; // C: `(void)user;`
                  // SAFETY: `stream` and `info` are the live out-stream and open-file info the
                  // loader passes to this callback (the `ufbx_open_file_fn` contract); `info`
                  // is dereferenced for its own `context` field, and `path`/`path_len`
                  // describe the caller's path buffer.
                  // C passes a NULL error slot — the callback only reports success; the
                  // `Result`'s error value is dropped, matching the null-slot no-write.
    unsafe { open_file_ctx(stream, (*info).context, path, path_len, core::ptr::null()) }.is_ok()
}

// ufbx.c:30412-30415 `ufbx_open_file`
pub(crate) unsafe fn open_file(
    stream: *mut RawStream,
    path: *const u8,
    path_len: usize,
    opts: *const RawOpenFileOpts,
) -> Result<(), Error> {
    // SAFETY: the pointers are this `unsafe fn`'s own params — `stream` the live
    // out-stream, `path`/`path_len` the caller's path buffer, `opts` null-or-live
    // — forwarded unchanged to `open_file_ctx`.
    unsafe { open_file_ctx(stream, 0 as OpenFileContext, path, path_len, opts) }
}

// ufbx.c:30417-30435 `ufbx_open_file_ctx`
pub(crate) unsafe fn open_file_ctx(
    stream: *mut RawStream,
    ctx: OpenFileContext,
    path: *const u8,
    mut path_len: usize,
    opts: *const RawOpenFileOpts,
) -> Result<(), Error> {
    // C: `ufbxi_file_context fc; // ufbxi_uninit`
    let fc = FileContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit()));
    // SAFETY: `fc` is the freshly created file context; `ctx` is the caller's
    // open-file context handle, null-or-valid per this fn's contract.
    unsafe { begin_file_context(&fc, ctx, core::ptr::null()) };
    if path_len == usize::MAX {
        // SAFETY: when `path_len == usize::MAX` the caller declares `path`
        // NUL-terminated (the C sentinel), so `strlen` walks to its terminator.
        path_len = unsafe { strlen(path) };
    }
    // C: `#if !defined(UFBX_NO_STDIO)` — always taken (no matching feature);
    // the disabled branch reports `"UFBX_NO_STDIO", "Feature disabled"`.
    let ok: bool = unsafe {
        // SAFETY: `fc` is the live file context, `stream` the live out-stream,
        // `path`/`path_len` the caller's path buffer, and `opts` (dereferenced
        // for its own flag only when non-null) is null-or-live per the contract.
        stdio_open(
            &fc,
            stream,
            path,
            path_len,
            if !opts.is_null() {
                (*opts).filename_null_terminated
            } else {
                false
            },
        )
    };
    // SAFETY: `fc` is the live file context.
    unsafe { end_file_context(&fc, ok) }
}

// ufbx.c:30437-30440 `ufbx_open_memory`
pub(crate) unsafe fn open_memory(
    stream: *mut RawStream,
    data: *const c_void,
    data_size: usize,
    opts: *const RawOpenMemoryOpts,
) -> Result<(), Error> {
    // SAFETY: the pointers are this `unsafe fn`'s own params — `stream` the live
    // out-stream, `data`/`data_size` the caller's memory block, `opts` null-or-live
    // — forwarded unchanged to `open_memory_ctx`.
    unsafe { open_memory_ctx(stream, 0 as OpenFileContext, data, data_size, opts) }
}

// ufbx.c:30442-30495 `ufbx_open_memory_ctx`
pub(crate) unsafe fn open_memory_ctx(
    stream: *mut RawStream,
    ctx: OpenFileContext,
    data: *const c_void,
    data_size: usize,
    opts: *const RawOpenMemoryOpts,
) -> Result<(), Error> {
    let mut local_opts = MaybeUninit::<RawOpenMemoryOpts>::uninit(); // ufbxi_uninit
    let mut opts = opts;
    if opts.is_null() {
        // SAFETY: `local_opts` is this frame's own uninitialized storage; the
        // write zero-fills exactly its `RawOpenMemoryOpts` byte extent.
        unsafe {
            core::ptr::write_bytes(
                local_opts.as_mut_ptr() as *mut u8,
                0,
                size_of::<RawOpenMemoryOpts>(),
            );
        }
        opts = local_opts.as_ptr();
    }
    // SAFETY: `opts` is now non-null — either the caller's live opts or the
    // zero-filled `local_opts` above — so its sentinel fields are readable.
    ufbx_assert!(unsafe { (*opts)._begin_zero == 0 && (*opts)._end_zero == 0 });

    // C: `ufbxi_file_context fc; // ufbxi_uninit`
    let fc = FileContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit()));
    // SAFETY: `fc` is the fresh file context; `ctx` is the caller's handle and
    // The raw field address preserves C's address-of semantics without creating
    // a Rust reference or an aliasing claim for the caller-owned options.
    unsafe { begin_file_context(&fc, ctx, &raw const (*opts).allocator) };

    // SAFETY: live `opts` per above; reading its own `no_copy` flag.
    let copy_size: usize = if unsafe { (*opts).no_copy } {
        0
    } else {
        data_size
    };

    // Align the allocation size to 8 bytes to make sure the header is aligned.
    let self_size: usize = align_to_mask(size_of::<MemoryStream>().wrapping_add(copy_size), 7);

    // SAFETY: `fc.ator_mut_ptr()` is the file context's own allocator.
    let memory: *mut u8 = unsafe { alloc::<u8>(fc.ator_mut_ptr(), self_size) };
    if memory.is_null() {
        // SAFETY: `fc` is the live file context; `end_file_context(false)`
        // yields the fixed `Err` this path returns.
        return unsafe { end_file_context(&fc, false) };
    }

    let mem = memory as *mut MemoryStream;
    // SAFETY: `mem` is the just-allocated block of `self_size >= sizeof header`
    // bytes; the write zero-fills exactly the `MemoryStream` header extent.
    unsafe { core::ptr::write_bytes(mem as *mut u8, 0, size_of::<MemoryStream>()) };

    // SAFETY: `mem` is the live allocated `MemoryStream`; writing its own fields.
    unsafe {
        (*mem).size = data_size;
        (*mem).self_size = self_size;
    }
    // SAFETY: `mem` is the live `MemoryStream` and `opts` is live per above.
    unsafe {
        (*mem).close_cb = (*opts).close_cb;
    }

    // SAFETY: live `opts` per above; reading its own `no_copy` flag.
    if unsafe { (*opts).no_copy } {
        // SAFETY: live `mem`; writing its own `data` field.
        unsafe {
            (*mem).data = data;
        }
    } else {
        // C: `memcpy(mem->data_copy, data, data_size)` — the flexible array
        // member starts right after the header (see `MemoryStream`).
        // SAFETY: the allocation reserved `sizeof header + copy_size` bytes, so
        // offsetting past the header lands within the same block.
        let data_copy: *mut u8 = unsafe { (mem as *mut u8).add(size_of::<MemoryStream>()) };
        // SAFETY: `data`/`data_size` describe the caller's live source block and
        // `data_copy` the reserved `copy_size == data_size` tail of the block;
        // the two regions are distinct allocations, hence non-overlapping.
        unsafe { core::ptr::copy_nonoverlapping(data as *const u8, data_copy, data_size) };
        // SAFETY: live `mem`; writing its own `data` field.
        unsafe {
            (*mem).data = data_copy as *const c_void;
        }
    }

    // Transplant the allocator in the result blob
    if !fc.parent_ator().is_null() {
        // SAFETY: live `mem`; writing its own `parent_ator` field.
        unsafe {
            (*mem).parent_ator = fc.parent_ator();
        }
    } else {
        // SAFETY: the raw field address identifies the live `mem`'s own
        // allocator field, adopted as the file context's parent allocator.
        unsafe { fc.set_parent_ator(&raw mut (*mem).local_ator) };
    }

    // SAFETY: `stream` is the caller's live out-stream; writing its own fields.
    unsafe {
        (*stream).read_fn = Some(memory_read);
        (*stream).skip_fn = Some(memory_skip);
        (*stream).size_fn = Some(memory_size);
        (*stream).close_fn = Some(memory_close);
        (*stream).user = mem as *mut c_void;
    }

    // SAFETY: `fc` is the live file context.
    unsafe { end_file_context(&fc, true) }
}

// ufbx.c:30497-30500 `ufbx_is_thread_safe`
pub(crate) fn is_thread_safe() -> bool {
    THREAD_SAFE != 0
}

// ufbx.c:30502-30511 `ufbx_load_memory`
pub(crate) unsafe fn load_memory(
    data: *const c_void,
    size: usize,
    opts: *const RawLoadOpts,
) -> Result<*mut Scene, Error> {
    // SAFETY: `opts` is null-or-live per this fn's contract; the macro reads its
    // sentinel fields only when non-null.
    unsafe { ufbxi_check_opts_res!(opts) };
    // C: `ufbxi_context uc; // ufbxi_uninit` + `memset(&uc, 0, sizeof(ufbxi_context));`
    let uc_storage = Context(core::cell::UnsafeCell::new(MaybeUninit::uninit())); // ufbxi_uninit
    let uc: &Context = &uc_storage;
    // SAFETY: `uc.get()` addresses this frame's own uninitialized context
    // storage; the write zero-fills exactly its `InnerContext` byte extent.
    unsafe { core::ptr::write_bytes(uc.get() as *mut u8, 0, size_of::<InnerContext>()) };
    // C: `uc.data_begin = uc.data = (const char *)data;`
    uc.set_data(data as *const u8);
    uc.set_data_begin(uc.data());
    uc.set_data_size(size);
    uc.set_progress_bytes_total(size as u64);
    // SAFETY: `uc` is the initialized context; `opts` is null-or-live (sentinels
    // validated by the macro when non-null; `evaluate::load` zero-fills on null).
    unsafe { evaluate::load(uc, opts) }
}

// ufbx.c:30513-30516 `ufbx_load_file`
pub(crate) unsafe fn load_file(
    filename: *const u8,
    opts: *const RawLoadOpts,
) -> Result<*mut Scene, Error> {
    // SAFETY: `filename` is the caller's NUL-terminated path (the `usize::MAX`
    // length sentinel), and `opts` is null-or-live per this fn's contract,
    // forwarded unchanged.
    unsafe { load_file_len(filename, usize::MAX, opts) }
}

// ufbx.c:30518-30527 `ufbx_load_file_len`
pub(crate) unsafe fn load_file_len(
    filename: *const u8,
    filename_len: usize,
    opts: *const RawLoadOpts,
) -> Result<*mut Scene, Error> {
    // SAFETY: `opts` is null-or-live per this fn's contract; the macro reads its
    // sentinel fields only when non-null.
    unsafe { ufbxi_check_opts_res!(opts) };
    let uc_storage = Context(core::cell::UnsafeCell::new(MaybeUninit::uninit())); // ufbxi_uninit
    let uc: &Context = &uc_storage;
    // SAFETY: `uc.get()` addresses this frame's own uninitialized context
    // storage; the write zero-fills exactly its `InnerContext` byte extent.
    unsafe { core::ptr::write_bytes(uc.get() as *mut u8, 0, size_of::<InnerContext>()) };
    uc.set_deferred_load(true);
    uc.set_load_filename(filename);
    uc.set_load_filename_len(filename_len);
    // SAFETY: `uc` is the initialized context; `opts` is null-or-live (sentinels
    // validated by the macro when non-null; `evaluate::load` zero-fills on null).
    unsafe { evaluate::load(uc, opts) }
}

// ufbx.c:30529-30532 `ufbx_load_stdio`
pub(crate) unsafe fn load_stdio(
    file_void: *mut c_void,
    opts: *const RawLoadOpts,
) -> Result<*mut Scene, Error> {
    // SAFETY: `file_void` is the caller's `FILE*` handle and `opts` is
    // null-or-live per this fn's contract, forwarded unchanged.
    unsafe { load_stdio_prefix(file_void, core::ptr::null(), 0, opts) }
}

// ufbx.c:30534-30554 `ufbx_load_stdio_prefix`
pub(crate) unsafe fn load_stdio_prefix(
    file_void: *mut c_void,
    prefix: *const c_void,
    prefix_size: usize,
    opts: *const RawLoadOpts,
) -> Result<*mut Scene, Error> {
    // C: `#if !defined(UFBX_NO_STDIO)` — always taken (no matching feature);
    // the disabled `#else` arm reports `"UFBX_NO_STDIO", "Feature disabled"`
    // through a deferred-failure `ufbxi_load`.
    if file_void.is_null() {
        // C's silent NULL: no slot write on this path — the shim clears the
        // caller slot only for an `Ok` with a non-null payload.
        return Ok(core::ptr::null_mut());
    }
    // C: `ufbx_stream stream = { 0 };`
    // SAFETY: `RawStream` is a plain-data struct of pointers and integers, so the
    // all-zero bit pattern is a valid (null callbacks, null user) value.
    let mut stream: RawStream = unsafe { MaybeUninit::zeroed().assume_init() };
    // SAFETY: `stream` is the fresh zeroed out-stream and `file_void` is the
    // caller's live `FILE*` handle (non-null, checked above).
    unsafe { stdio_init(&raw mut stream, file_void, false) };
    // SAFETY: `stream` is the just-initialized stdio stream, `prefix`/
    // `prefix_size` describe the caller's optional prefix block, and `opts` is
    // null-or-live per this fn's contract.
    unsafe { load_stream_prefix(&stream, prefix, prefix_size, opts) }
}

// ufbx.c:30556-30559 `ufbx_load_stream`
pub(crate) unsafe fn load_stream(
    stream: *const RawStream,
    opts: *const RawLoadOpts,
) -> Result<*mut Scene, Error> {
    // SAFETY: `stream` is the caller's live stream and `opts` is null-or-live
    // per this fn's contract, forwarded unchanged.
    unsafe { load_stream_prefix(stream, core::ptr::null(), 0, opts) }
}

// ufbx.c:30561-30576 `ufbx_load_stream_prefix`
pub(crate) unsafe fn load_stream_prefix(
    stream: *const RawStream,
    prefix: *const c_void,
    prefix_size: usize,
    opts: *const RawLoadOpts,
) -> Result<*mut Scene, Error> {
    // SAFETY: `opts` is null-or-live per this fn's contract; the macro reads its
    // sentinel fields only when non-null.
    unsafe { ufbxi_check_opts_res!(opts) };
    let uc_storage = Context(core::cell::UnsafeCell::new(MaybeUninit::uninit())); // ufbxi_uninit
    let uc: &Context = &uc_storage;
    // SAFETY: `uc.get()` addresses this frame's own uninitialized context
    // storage; the write zero-fills exactly its `InnerContext` byte extent.
    unsafe { core::ptr::write_bytes(uc.get() as *mut u8, 0, size_of::<InnerContext>()) };
    // C: `uc.data_begin = uc.data = (const char *)prefix;`
    uc.set_data(prefix as *const u8);
    uc.set_data_begin(uc.data());
    uc.set_data_size(prefix_size);
    // SAFETY: `stream` points at the caller's live `RawStream`; each read takes
    // one of its own callback/user fields.
    unsafe {
        uc.set_read_fn((*stream).read_fn);
        uc.set_skip_fn((*stream).skip_fn);
        uc.set_size_fn((*stream).size_fn);
        uc.set_close_fn((*stream).close_fn);
        uc.set_read_user((*stream).user);
    }

    // SAFETY: `uc` is the initialized context; `opts` is null-or-live (sentinels
    // validated by the macro when non-null; `evaluate::load` zero-fills on null).
    unsafe { evaluate::load(uc, opts) }
}

// ufbx.c:30578-30586 `ufbx_free_scene`
// C has no `ufbxi_noinline` here (unlike `ufbx_format_error` below).
pub(crate) unsafe fn free_scene(scene: *mut Scene) {
    if scene.is_null() {
        return;
    }

    // SAFETY: the non-null `scene` is the payload of a live `SceneImp` handed
    // out by this library — the raw-pointer contract of this `unsafe fn`.
    let imp = unsafe { ImpHandle::<SceneImp>::from_payload(scene) };
    ufbx_assert!(imp.has_magic());
    if !imp.has_magic() {
        return;
    }
    imp.release();
}

// ufbx.c:30588-30596 `ufbx_retain_scene`
// C has no `ufbxi_noinline` here (unlike `ufbx_format_error` below).
pub(crate) unsafe fn retain_scene(scene: *mut Scene) {
    if scene.is_null() {
        return;
    }

    // SAFETY: the non-null `scene` is the payload of a live `SceneImp` handed
    // out by this library — the raw-pointer contract of this `unsafe fn`.
    let imp = unsafe { ImpHandle::<SceneImp>::from_payload(scene) };
    ufbx_assert!(imp.has_magic());
    if !imp.has_magic() {
        return;
    }
    imp.retain();
}

// ufbx.c:30598-30633 `ufbx_format_error`
#[inline(never)]
pub(crate) unsafe fn format_error(dst: *mut u8, dst_size: usize, error: *const Error) -> usize {
    if dst.is_null() || dst_size == 0 {
        return 0;
    }
    if error.is_null() {
        // SAFETY: `dst` is non-null and `dst_size >= 1` (both checked above), so
        // its first byte is writable.
        unsafe {
            *dst = b'\0';
        }
        return 0;
    }

    let mut offset: usize = 0;

    {
        let num: i32;
        // SAFETY: `error` is non-null (checked) and points at a live `Error` —
        // the raw-pointer contract of this `unsafe fn`; reading its own field.
        if unsafe { (*error).info_length > 0 && (*error).info_length < ERROR_INFO_LENGTH } {
            // SAFETY: `dst.add(offset)` stays within the `dst_size`-byte output
            // buffer (`offset <= dst_size - 1`), and each `(*error)` read takes a
            // field of the live `Error`; `ufbxi_snprintf!` honors the length cap.
            num = unsafe {
                ufbxi_snprintf!(
                    dst.add(offset),
                    dst_size - offset,
                    "ufbx v%u.%u.%u error: %s (%.*s)\n",
                    SOURCE_VERSION / 1000000,
                    SOURCE_VERSION / 1000 % 1000,
                    SOURCE_VERSION % 1000,
                    if !(*error).description.data.is_null() {
                        (*error).description.data
                    } else {
                        b"Unknown error\0".as_ptr()
                    },
                    (*error).info_length as i32,
                    (*error).info_buf.data.as_ptr() as *const u8,
                )
            };
        } else {
            // SAFETY: as above.
            num = unsafe {
                ufbxi_snprintf!(
                    dst.add(offset),
                    dst_size - offset,
                    "ufbx v%u.%u.%u error: %s\n",
                    SOURCE_VERSION / 1000000,
                    SOURCE_VERSION / 1000 % 1000,
                    SOURCE_VERSION % 1000,
                    if !(*error).description.data.is_null() {
                        (*error).description.data
                    } else {
                        b"Unknown error\0".as_ptr()
                    },
                )
            };
        }

        if num > 0 {
            offset = min_sz(offset.wrapping_add(num as usize), dst_size - 1);
        }
    }

    // SAFETY: live `Error` per above; reading its own `stack_size` field.
    let stack_size: usize = min_sz(
        unsafe { (*error).stack_size } as usize,
        ERROR_STACK_MAX_DEPTH,
    );
    let line_width: i32 = 6;
    for i in 0..stack_size {
        // C: `const ufbx_error_frame *frame = &error->stack[i];`
        // SAFETY: `&raw const (*error).stack` addresses the live `Error`'s stack
        // array and `i < stack_size <= ERROR_STACK_MAX_DEPTH` is a valid index.
        let frame: *const ErrorFrame =
            unsafe { (&raw const (*error).stack as *const ErrorFrame).add(i) };
        // SAFETY: `dst.add(offset)` stays within the `dst_size`-byte output
        // buffer, and `frame` is the live stack frame just indexed; the reads
        // take its own NUL-terminated span fields.
        let num: i32 = unsafe {
            ufbxi_snprintf!(
                dst.add(offset),
                dst_size - offset,
                "%*u:%s: %s\n",
                line_width,
                (*frame).source_line,
                (*frame).function.data,
                (*frame).description.data,
            )
        };
        if num > 0 {
            offset = min_sz(offset.wrapping_add(num as usize), dst_size - 1);
        }
    }

    offset
}

// ufbx.c:30635-33179 is covered below. The short String API wrappers
// (ufbx.c:33140-33161) and non-catching geometry wrappers (33163-33179) are
// grouped beside the implementations they delegate to. All API entry points
// in the C file are present; `ufbx_inflate` is owned by `native::deflate`.

// ufbx.c:30635-30650 `ufbx_find_prop_len`
// `name: &[u8]` carries C's `(name, name_len)` pair (the `_len` suffix IS the
// slice); C's `ufbxi_safe_string` null-guard is subsumed by the slice mint at
// the ABI shims (`slice_from_ptr` maps the null/0 case to the empty slice).
pub(crate) fn find_prop_len<'a, M: Mode>(
    props: &'a View<Props, M>,
    name: &[u8],
) -> Option<&'a View<Prop, M>> {
    let key = get_name_key(name);

    let mut props: Option<&'a View<Props, M>> = Some(props);
    while let Some(cur) = props {
        let run = cur.props_view();
        if let Some(index) = run.lower_bound_eq(
            4,
            |a| cmp_prop_less_ref(a, name, key),
            |a| a._internal_key() == key && str_equal(a.name_view().bytes(), name),
        ) {
            return Some(run.at(index));
        }

        props = cur.defaults();
    }

    None
}

// ufbx.c:30652-30660 `ufbx_find_real_len`
pub(crate) fn find_real_len<M: Mode>(props: &View<Props, M>, name: &[u8], def: Real) -> Real {
    match find_prop_len(props, name) {
        // C-parity: `prop->value_real` is the `ufbx_prop` value union's first
        // real; the generated struct keeps only `value_vec4` (same mapping as
        // `native::parse::find_real`).
        Some(prop) => prop.value_vec4().x,
        None => def,
    }
}

// ufbx.c:30662-30670 `ufbx_find_vec3_len`
// Kept here because `ufbxi_update_constraint`
// (ufbx.c:23416, `native::scene_process`) calls `ufbx_find_vec3`.
#[inline(never)]
pub(crate) fn find_vec3_len<M: Mode>(props: &View<Props, M>, name: &[u8], def: Vec3) -> Vec3 {
    match find_prop_len(props, name) {
        // C-parity: `prop->value_vec3` is the `ufbx_prop` value union's 3-real
        // view; the generated struct keeps only `value_vec4` (same mapping as
        // `native::parse::find_vec3`).
        Some(prop) => prop.value_vec3(),
        None => def,
    }
}

// ufbx.c:30672-30680 `ufbx_find_int_len`
#[inline(never)]
pub(crate) fn find_int_len<M: Mode>(props: &View<Props, M>, name: &[u8], def: i64) -> i64 {
    match find_prop_len(props, name) {
        Some(prop) => prop.value_int(),
        None => def,
    }
}

// ufbx.c:30682-30690 `ufbx_find_bool_len`
pub(crate) fn find_bool_len<M: Mode>(props: &View<Props, M>, name: &[u8], def: bool) -> bool {
    match find_prop_len(props, name) {
        Some(prop) => prop.value_int() != 0,
        None => def,
    }
}

// ufbx.c:30692-30700 `ufbx_find_string_len`
#[inline(never)]
pub(crate) fn find_string_len<M: Mode>(props: &View<Props, M>, name: &[u8], def: String) -> String {
    match find_prop_len(props, name) {
        Some(prop) => prop.value_str(),
        None => def,
    }
}

// ufbx.c:30702-30710 `ufbx_find_blob_len`
// C has no `ufbxi_noinline` here (unlike `ufbx_find_string_len` above).
pub(crate) fn find_blob_len<M: Mode>(props: &View<Props, M>, name: &[u8], def: Blob) -> Blob {
    match find_prop_len(props, name) {
        Some(prop) => prop.value_blob(),
        None => def,
    }
}

// ufbx.c:30712-30728 `ufbx_find_prop_concat`
// Kept here because `ufbxi_update_constraint`
// (ufbx.c:23416, `native::scene_process`) calls it.
pub(crate) unsafe fn find_prop_concat<'a, M: Mode>(
    props: &'a View<Props, M>,
    parts: &[String],
) -> Option<&'a View<Prop, M>> {
    // SAFETY: each part's `data` is readable for its `length` bytes — the
    // key-part contract of this `unsafe fn`.
    let key: u32 = unsafe { get_concat_key(parts) };

    let mut props: Option<&'a View<Props, M>> = Some(props);
    while let Some(cur) = props {
        let run = cur.props_view();
        // SAFETY (both inner ops): each part's `data` is readable for its
        // `length` bytes — the key-part contract of this `unsafe fn`, forwarded
        // to `cmp_prop_less_concat`/`concat_str_cmp`.
        if let Some(index) = run.lower_bound_eq(
            2,
            |a| unsafe { cmp_prop_less_concat(a, parts, key) },
            |a| a._internal_key() == key && unsafe { concat_str_cmp(a.name(), parts) } == 0,
        ) {
            return Some(run.at(index));
        }

        props = cur.defaults();
    }

    None
}

// Public-boundary root for the nullable `const ufbx_scene *` the find-by-name
// surface takes: mint a read-only `Const` view — legal for ANY readable
// provenance, including a safe wrapper's `&Scene`, unlike the interior-mutable
// `Mut` view (Miri SB — see the topology finding). One named home for the
// mint, so each raw-boundary caller states the contract once.
//
// # Safety
// `scene` is null, or points at a live `Scene` that stays alive, unmoved and
// unwritten through any parent pointer while the returned view is used.
pub(crate) unsafe fn scene_const_view<'a>(scene: *const Scene) -> Option<&'a View<Scene, Const>> {
    if scene.is_null() {
        None
    } else {
        // SAFETY: non-null (checked) and live/frozen per this fn's contract.
        Some(unsafe { View::<Scene, Const>::from_ptr(scene) })
    }
}

// ufbx.c:30730-30741 `ufbx_find_element_len`
// `name: &[u8]` carries C's `(name, name_len)` pair (see `find_prop_len`).
// `scene: Option<&View<Scene, M>>` carries C's nullable `const ufbx_scene *`
// (the `!scene` guard becomes the `None` arm).
pub(crate) fn find_element_len<M: Mode>(
    scene: Option<&View<Scene, M>>,
    type_: ElementType,
    name: &[u8],
) -> *mut Element {
    let Some(scene) = scene else {
        return core::ptr::null_mut();
    };
    let key: u32 = get_name_key(name);

    let index: Option<usize> = scene.elements_by_name_view().lower_bound_eq(
        16,
        |a| cmp_name_element_less_ref(a, name, type_, key),
        |a| str_equal(a.name_view().bytes(), name) && a.type_() == type_,
    );

    match index {
        // SAFETY: `index` is a hit, so `at(index)` is the matched live
        // `NameElement`; `element_ptr()` addresses its own `Ref<Element>`
        // field, which `ref_ptr` follows.
        Some(index) => unsafe { ref_ptr(scene.elements_by_name_view().at(index).element_ptr()) },
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:30743-30748 `ufbx_get_prop_element`
pub(crate) unsafe fn get_prop_element(
    element: *const Element,
    prop: *const Prop,
    type_: ElementType,
) -> *mut Element {
    ufbx_assert!(!element.is_null() && !prop.is_null());
    if element.is_null() || prop.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: `element` and `prop` are non-null (checked) and point at a live
    // `Element` and `Prop` — the raw-pointer contract of this `unsafe fn`;
    // `(*prop).name.data` reads the prop's own interned name pointer.
    unsafe { fetch_dst_element(element as *mut Element, false, (*prop).name.data, type_) }
}

// ufbx.c:30750-30758 `ufbx_find_prop_element_len`
// `name: &[u8]` carries C's `(name, name_len)` pair (see `find_prop_len`);
// `element: &View<Element, M>` carries C's non-nullable `const ufbx_element *`
// (C dereferences it unconditionally).
pub(crate) fn find_prop_element_len<M: Mode>(
    element: &View<Element, M>,
    name: &[u8],
    type_: ElementType,
) -> *mut Element {
    // `&element->props` is the view's own `props` projection — no intermediate
    // `&Props` is formed.
    match find_prop_len(element.props(), name) {
        // SAFETY: `element.as_ptr()` addresses the live viewed `Element` and
        // `prop.as_ptr()` the matched live `Prop` — the raw-pointer contract
        // `get_prop_element` asks for, discharged by both views.
        Some(prop) => unsafe { get_prop_element(element.as_ptr(), prop.as_ptr(), type_) },
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:30760-30763 `ufbx_find_node_len`
pub(crate) fn find_node_len<M: Mode>(scene: Option<&View<Scene, M>>, name: &[u8]) -> *mut Node {
    find_element_len(scene, ElementType::Node, name) as *mut Node
}

// ufbx.c:30765-30768 `ufbx_find_anim_stack_len`
pub(crate) fn find_anim_stack_len<M: Mode>(
    scene: Option<&View<Scene, M>>,
    name: &[u8],
) -> *mut AnimStack {
    find_element_len(scene, ElementType::AnimStack, name) as *mut AnimStack
}

// ufbx.c:30770-30773 `ufbx_find_material_len`
pub(crate) fn find_material_len<M: Mode>(
    scene: Option<&View<Scene, M>>,
    name: &[u8],
) -> *mut Material {
    find_element_len(scene, ElementType::Material, name) as *mut Material
}

// ufbx.c:30775-30790 `ufbx_find_anim_prop_len`
// `prop: &[u8]` carries C's `(prop, prop_len)` pair (see `find_prop_len`).
// `element` is ADDRESS-ONLY: the anim-prop array is sorted by owning-element
// pointer (ufbx.c:18596) and every probe compares addresses via `Ref::ptr` —
// no byte behind `element` is ever read, so the raw param carries no
// obligation (the `is_node_property_name` precedent).
pub(crate) fn find_anim_prop_len<'a, M: Mode>(
    layer: Option<&'a View<AnimLayer, M>>,
    element: *const Element,
    prop: &[u8],
) -> Option<&'a View<AnimProp, M>> {
    ufbx_assert!(layer.is_some());
    ufbx_assert!(!element.is_null());
    let layer = layer?;
    if element.is_null() {
        return None;
    }

    let run = layer.anim_props_view();
    let index = run.lower_bound_eq(
        16,
        // C: `a->element != element ? a->element < element : ufbxi_str_less(a->prop_name, prop_str)`
        // — a raw ADDRESS comparison of the owning element (the array is sorted
        // by element pointer, ufbx.c:18596), not by `element_id`.
        |a| {
            let a_element: *const Element = a.element().ptr();
            if a_element != element {
                a_element < element
            } else {
                str_less(a.prop_name_view().bytes(), prop)
            }
        },
        |a| {
            core::ptr::eq(a.element().ptr(), element) && str_equal(a.prop_name_view().bytes(), prop)
        },
    )?;
    Some(run.at(index))
}

// ufbx.c:30792-30812 `ufbx_find_anim_props`
#[inline(never)]
pub(crate) fn find_anim_props<M: Mode>(
    layer: Option<&View<AnimLayer, M>>,
    element: *const Element,
) -> List<AnimProp> {
    // C: `ufbx_anim_prop_list result = { 0 };` — `List<T>` carries a private
    // `PhantomData` marker, so the C aggregate initializer becomes a zeroed
    // value with both public fields written (same shape as
    // `find_shader_prop_bindings_len` below).
    // SAFETY: `List<AnimProp>` is a raw pointer, a `usize` and a zero-sized
    // `PhantomData`, so the all-zero bit pattern is a valid (null, empty) value.
    let mut result: List<AnimProp> = unsafe { MaybeUninit::zeroed().assume_init() };
    result.data = core::ptr::null();
    result.count = 0;
    ufbx_assert!(layer.is_some());
    ufbx_assert!(!element.is_null());
    let Some(layer) = layer else {
        return result;
    };
    if element.is_null() {
        return result;
    }

    let run = layer.anim_props_view();
    // C: `size_t begin = layer->anim_props.count, end = begin;` — the lower
    // bound does not write on a miss; `unwrap_or` reproduces the pre-init.
    // `element` is address-only, as for `find_anim_prop_len` above.
    let begin: usize = run
        .lower_bound_eq(
            16,
            |a| (a.element().ptr() as *const Element) < element,
            |a| core::ptr::eq(a.element().ptr(), element),
        )
        .unwrap_or(run.count());
    let end: usize = run.upper_bound_eq(16, begin, |a| core::ptr::eq(a.element().ptr(), element));

    if begin != end {
        // The run base must carry the WHOLE-run provenance of the stored list
        // pointer — an `at(begin).as_ptr()` base is retagged for that single
        // element only, so the caller's `data..data+count` slice would be UB
        // (Miri-SB-caught). `begin < end <= count`, so the offset is in bounds.
        result.data = run.data().wrapping_add(begin);
        result.count = end - begin;
    }

    result
}

// ufbx.c:30814-30825 `ufbx_get_compatible_matrix_for_normals`
#[inline(never)]
pub(crate) unsafe fn get_compatible_matrix_for_normals(node: *const Node) -> Matrix {
    if node.is_null() {
        return IDENTITY_MATRIX;
    }

    let mut geom_rot: Transform = IDENTITY_TRANSFORM;
    // SAFETY: `node` is non-null (checked) and points at a live `Node` — the
    // raw-pointer contract of this `unsafe fn`; reading its own transform's
    // rotation.
    geom_rot.rotation = unsafe { (*node).geometry_transform.rotation };
    // SAFETY: `&geom_rot` addresses the fully-initialized local transform.
    let geom_rot_mat: Matrix = unsafe { transform_to_matrix(&geom_rot) };

    // SAFETY: `&raw const (*node).node_to_world` addresses the live `node`'s own
    // world matrix and `&geom_rot_mat` the local matrix just computed.
    let mut norm_mat: Matrix =
        unsafe { matrix_mul(&raw const (*node).node_to_world, &geom_rot_mat) };
    // SAFETY: `&norm_mat` addresses the local matrix just computed.
    norm_mat = unsafe { matrix_for_normals(&norm_mat) };
    norm_mat
}

// Hand nav accessors for `AnimValue.curves` (`view_accessor_skip_read` in
// generate_rust.py): the field is a fixed array of three NULLABLE curve refs,
// read per-slot as the niche-packed bare pointer it is — never through
// `Ref::as_ref`, whose `&AnimCurve` formation is the Stacked Borrows trap the
// view sidesteps (same rationale as `PropsView::defaults`).
impl<M: Mode> View<AnimValue, M> {
    #[inline(always)]
    pub(crate) fn curve_view(&self, index: usize) -> Option<&View<AnimCurve, M>> {
        assert!(index < 3);
        // SAFETY: `index` is in bounds of the fixed three-slot array (checked
        // above); the slot is read as niche-packed bare pointer bits, asserting
        // only that leaf per this view's mint vouch.
        let ptr: *mut AnimCurve = unsafe {
            *(&raw const (*self.as_ptr()).curves)
                .cast::<*mut AnimCurve>()
                .add(index)
        };
        if ptr.is_null() {
            None
        } else {
            // SAFETY: a non-null slot names a live curve element of the same
            // scene as the viewed anim value, with the slot's stored provenance
            // — adequate for `M` per the anim value's own mint.
            Some(unsafe { View::mint(ptr) })
        }
    }

    /// Slot `index` as the nullable raw pointer it stores, with no view minted
    /// over it — for sites that hand the pointer straight to a raw-contract fn
    /// (e.g. `translate_element`).
    #[inline(always)]
    pub(crate) fn curve_ptr(&self, index: usize) -> *mut AnimCurve {
        assert!(index < 3);
        // SAFETY: `index` is in bounds of the fixed three-slot array (checked
        // above); the slot is read as niche-packed bare pointer bits, asserting
        // only that leaf per this view's mint vouch.
        unsafe {
            *(&raw const (*self.as_ptr()).curves)
                .cast::<*mut AnimCurve>()
                .add(index)
        }
    }
}

impl View<AnimValue, Mut> {
    /// Store a nullable curve pointer into slot `index` (the write half of
    /// `curve_view`, for the evaluated-scene patch loops).
    #[inline(always)]
    pub(crate) fn set_curve_ptr(&self, index: usize, curve: *mut AnimCurve) {
        assert!(index < 3);
        // SAFETY: in-bounds slot write (checked above) through the `Mut` view's
        // write-capable viewed memory, storing the nullable pointer as the
        // niche-packed bare bits the `Option<Ref<AnimCurve>>` slot holds.
        unsafe {
            *(&raw mut (*self.get()).curves)
                .cast::<*mut AnimCurve>()
                .add(index) = curve;
        }
    }
}

// ufbx.c:30827-30830 `ufbx_evaluate_curve`
pub(crate) fn evaluate_curve(
    curve: Option<&View<AnimCurve, Const>>,
    time: f64,
    default_value: Real,
) -> Real {
    evaluate_curve_flags(curve, time, default_value, 0)
}

// ufbx.c:30832-30914 `ufbx_evaluate_curve_flags`
// C's null-or-live `ufbx_anim_curve*` param arrives as `Option<&View<_, Const>>`
// (the boundary shims mint the view from the caller's pointer).
pub(crate) fn evaluate_curve_flags(
    curve: Option<&View<AnimCurve, Const>>,
    time: f64,
    default_value: Real,
    flags: u32,
) -> Real {
    let Some(curve) = curve else {
        return default_value;
    };
    let keys = curve.keyframes_view();
    if keys.count() <= 1 {
        if keys.count() == 1 {
            return keys.at(0).value();
        } else {
            return default_value;
        }
    }

    if (flags & EvaluateFlags::NO_EXTRAPOLATION.raw()) == 0 {
        if time < curve.min_time() || time > curve.max_time() {
            return evaluate::extrapolate_curve(curve, time, flags);
        }
    }

    let mut begin: usize = 0;
    let mut end: usize = keys.count();
    while end - begin >= 8 {
        let mid: usize = (begin + end) >> 1;
        if keys.at(mid).time() <= time {
            begin = mid + 1;
        } else {
            end = mid;
        }
    }

    end = keys.count();
    // C: `for (; begin < end; begin++)` — every switch arm returns, so the
    // increment is only reached through the `continue`.
    while begin < end {
        let next = keys.at(begin);
        if next.time() <= time {
            begin += 1;
            continue;
        }

        // First keyframe
        if begin == 0 {
            return next.value();
        }

        let prev = keys.at(begin - 1);

        // Exact keyframe
        if prev.time() == time {
            return prev.value();
        }

        let rcp_delta: f64 = 1.0 / (next.time() - prev.time());
        let mut t: f64 = (time - prev.time()) * rcp_delta;

        match prev.interpolation() {
            Interpolation::ConstantPrev => return prev.value(),

            Interpolation::ConstantNext => return next.value(),

            Interpolation::Linear => {
                // C: `return (ufbx_real)(prev->value*(1.0 - t) + next->value*t);`
                // `1.0 - t` is double, so both `value`s promote to double.
                return (as_f64!(prev.value()) * (1.0 - t) + as_f64!(next.value()) * t) as Real;
            }

            Interpolation::Cubic => {
                // C: tangent `dx`/`dy` are float, promoted to double.
                let x1: f64 = prev.right().dx as f64 * rcp_delta;
                let x2: f64 = 1.0 - next.left().dx as f64 * rcp_delta;
                t = evaluate::find_cubic_bezier_t(x1, x2, t);

                let t2: f64 = t * t;
                let t3: f64 = t2 * t;
                let u: f64 = 1.0 - t;
                let u2: f64 = u * u;
                let u3: f64 = u2 * u;

                // C: `double y0 = prev->value;` — `ufbx_real` promoted to double.
                let y0: f64 = as_f64!(prev.value());
                let y3: f64 = as_f64!(next.value());
                let y1: f64 = y0 + prev.right().dy as f64;
                let y2: f64 = y3 - next.left().dy as f64;

                // C: `return (ufbx_real)(u3*y0 + 3.0 * (u2*t*y1 + u*t2*y2) + t3*y3);`
                return (u3 * y0 + 3.0 * (u2 * t * y1 + u * t2 * y2) + t3 * y3) as Real;
            }

            // C `default:` — unreachable in Rust because the match above is
            // exhaustive over the `#[repr(u32)]` enum.
            #[allow(unreachable_patterns)]
            _ => {
                ufbxi_unreachable!("Bad interpolation mode");
                #[allow(unreachable_code)]
                return 0.0;
            }
        }
    }

    // Last keyframe
    keys.at(keys.count() - 1).value()
}

// ufbx.c:30916-30919 `ufbx_evaluate_anim_value_real`
#[inline(never)]
pub(crate) fn evaluate_anim_value_real(
    anim_value: Option<&View<AnimValue, Const>>,
    time: f64,
) -> Real {
    evaluate_anim_value_real_flags(anim_value, time, 0)
}

// ufbx.c:30921-30924 `ufbx_evaluate_anim_value_vec3`
#[inline(never)]
pub(crate) fn evaluate_anim_value_vec3(
    anim_value: Option<&View<AnimValue, Const>>,
    time: f64,
) -> Vec3 {
    evaluate_anim_value_vec3_flags(anim_value, time, 0)
}

// ufbx.c:30926-30935 `ufbx_evaluate_anim_value_real_flags`
// C's null-or-live `ufbx_anim_value*` param arrives as `Option<&View<_, Const>>`
// (the boundary shims mint the view from the caller's pointer).
#[inline(never)]
pub(crate) fn evaluate_anim_value_real_flags(
    anim_value: Option<&View<AnimValue, Const>>,
    time: f64,
    flags: u32,
) -> Real {
    let Some(anim_value) = anim_value else {
        return 0.0;
    };

    let mut res: Real = anim_value.default_value().x;
    // C: `if (anim_value->curves[0]) res = ufbx_evaluate_curve_flags(anim_value->curves[0], time, res, flags);`
    if let Some(curve0) = anim_value.curve_view(0) {
        res = evaluate_curve_flags(Some(curve0), time, res, flags);
    }
    res
}

// ufbx.c:30937-30949 `ufbx_evaluate_anim_value_vec3_flags`
#[inline(never)]
pub(crate) fn evaluate_anim_value_vec3_flags(
    anim_value: Option<&View<AnimValue, Const>>,
    time: f64,
    flags: u32,
) -> Vec3 {
    let Some(anim_value) = anim_value else {
        // C: `ufbx_vec3 zero = { 0.0f };`
        let zero: Vec3 = Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        return zero;
    };

    let mut res: Vec3 = anim_value.default_value();
    if let Some(curve0) = anim_value.curve_view(0) {
        res.x = evaluate_curve_flags(Some(curve0), time, res.x, flags);
    }
    if let Some(curve1) = anim_value.curve_view(1) {
        res.y = evaluate_curve_flags(Some(curve1), time, res.y, flags);
    }
    if let Some(curve2) = anim_value.curve_view(2) {
        res.z = evaluate_curve_flags(Some(curve2), time, res.z, flags);
    }
    res
}

// ufbx.c:30951-30954 `ufbx_evaluate_prop_len`
#[inline(never)]
pub(crate) unsafe fn evaluate_prop_len(
    anim: *const Anim,
    element: *const Element,
    name: *const u8,
    name_len: usize,
    time: f64,
) -> Prop {
    // SAFETY: the pointers are this `unsafe fn`'s own params — `anim`/`element`
    // live, `name`/`name_len` the caller's key buffer — forwarded unchanged.
    unsafe { evaluate_prop_flags_len(anim, element, name, name_len, time, 0) }
}

// ufbx.c:30956-30989 `ufbx_evaluate_prop_flags_len`
#[inline(never)]
pub(crate) unsafe fn evaluate_prop_flags_len(
    anim: *const Anim,
    element: *const Element,
    name: *const u8,
    name_len: usize,
    time: f64,
    flags: u32,
) -> Prop {
    // C: `ufbx_prop result;`
    let mut result: Prop;

    // Public-boundary root: caller-owned `*const Element` whose provenance can
    // be a read-only `&Element` (safe Rust wrapper), so mint a read-only
    // `Const` view — legal for any readable provenance (Miri SB, topology
    // finding). `ufbx_evaluate_prop_flags` takes `const ufbx_element *`, so the
    // element stays frozen for the whole body and its `props` table and
    // `element_id` are reached through the view's accessors.
    // SAFETY: `element` points at a live `Element` — the raw-pointer contract of
    // this `unsafe fn`.
    let element_view: &View<Element, Const> = unsafe { View::<Element, Const>::from_ptr(element) };
    let props: &View<Props, Const> = element_view.props();
    // SAFETY: `name`/`name_len` are the caller's key-buffer params, minted as
    // the query slice (`slice_from_ptr` maps the null/0 case to empty).
    let prop: Option<&View<Prop, Const>> = find_prop_len(props, unsafe {
        crate::prelude::slice_from_ptr(name, name_len)
    });
    if let Some(found) = prop {
        // SAFETY: `found.as_ptr()` addresses the matched live `Prop`; the read
        // copies it out by value (C struct assignment).
        result = unsafe { *found.as_ptr() };
    } else {
        // C: `memset(&result, 0, sizeof(result));`
        // SAFETY: `Prop` is a plain-data struct of pointers, spans and scalars,
        // so the all-zero bit pattern is a valid value (C `memset(&result, 0)`).
        result = unsafe { MaybeUninit::zeroed().assume_init() };
        result.name.data = name;
        result.name.length = name_len;
        // SAFETY: `name`/`name_len` describe the caller's key bytes.
        result._internal_key =
            unsafe { get_name_key(crate::prelude::slice_from_ptr(name, name_len)) };
        result.flags = PropFlags::NOT_FOUND;
        result.value_str.data = EMPTY_CHAR.as_ptr();
        result.value_str.length = 0;
        result.value_blob.data = core::ptr::null();
        result.value_blob.size = 0;
    }

    // SAFETY: `anim` points at a live `Anim` — the raw-pointer contract of this
    // `unsafe fn`; reading its own `prop_overrides.count`.
    if unsafe { (*anim).prop_overrides.count } > 0 {
        // SAFETY: `&raw const (*anim).prop_overrides` addresses the live anim's
        // own overrides list (read-only during evaluation — the `Const` mint's
        // freeze) and `&raw mut result` roots a write-capable `Mut` view over
        // the local prop.
        unsafe {
            evaluate::find_prop_override(
                View::<_, Const>::from_ptr(&raw const (*anim).prop_overrides),
                element_view.element_id(),
                View::<_, Mut>::from_ptr(&raw mut result),
            )
        };
        return result;
    }

    if (result.flags.raw() & (PropFlags::ANIMATED.raw() | PropFlags::CONNECTED.raw())) == 0 {
        return result;
    }

    // C-parity: `prop->flags` — `prop` is non-NULL here because the NOT_FOUND
    // branch above always takes the early return.
    // SAFETY: live `anim` per above; reading its own `ignore_connections` flag.
    if (prop.unwrap().flags().raw() & PropFlags::CONNECTED.raw()) != 0
        && !unsafe { (*anim).ignore_connections }
    {
        // SAFETY: the raw address identifies the local prop, `anim`/`element` are the
        // live params, and `prop.unwrap().name().data` is the matched prop's own
        // interned name pointer.
        unsafe {
            evaluate::evaluate_connected_prop(
                &raw mut result,
                anim,
                element,
                prop.unwrap().name().data,
                time,
                flags,
            )
        };
    }

    // SAFETY: `anim`/`element` are the live params and the raw address identifies the
    // local prop, evaluated as a one-element buffer.
    unsafe { evaluate::evaluate_props(anim, element, time, &raw mut result, 1, flags) };

    result
}

// ufbx.c:30991-30994 `ufbx_evaluate_props`
#[inline(never)]
pub(crate) unsafe fn evaluate_props(
    anim: *const Anim,
    element: *const Element,
    time: f64,
    buffer: *mut Prop,
    buffer_size: usize,
) -> Props {
    // SAFETY: the pointers are this `unsafe fn`'s own params — `anim`/`element`
    // live, `buffer`/`buffer_size` the caller's output array — forwarded
    // unchanged to `evaluate_props_flags`.
    unsafe { evaluate_props_flags(anim, element, time, buffer, buffer_size, 0) }
}

// ufbx.c:30996-31023 `ufbx_evaluate_props_flags`
#[inline(never)]
pub(crate) unsafe fn evaluate_props_flags(
    anim: *const Anim,
    element: *const Element,
    time: f64,
    buffer: *mut Prop,
    buffer_size: usize,
    flags: u32,
) -> Props {
    // C: `ufbx_props ret = { NULL };`
    // SAFETY: `Props` is a plain-data struct of pointers and counts, so the
    // all-zero bit pattern is a valid (null, empty) value.
    let mut ret: Props = unsafe { MaybeUninit::zeroed().assume_init() };
    if element.is_null() {
        return ret;
    }

    let mut num_anim: usize = 0;
    let mut iter = MaybeUninit::<evaluate::PropIter>::uninit(); // ufbxi_uninit
    let iter: *mut evaluate::PropIter = iter.as_mut_ptr();
    // Public-boundary roots: `anim`/`element` are the caller's `*const` pointers,
    // so mint read-only `Const` views; the iterator only reads through them and
    // the frozen tags end with this call.
    // SAFETY: `iter` addresses this frame's own uninitialized iterator storage,
    // which `init_prop_iter` fills; `anim`/`element` are the live params.
    unsafe {
        evaluate::init_prop_iter(
            iter,
            View::<Anim, Const>::from_ptr(anim),
            View::<Element, Const>::from_ptr(element),
        )
    };
    // C: `while ((prop = ufbxi_next_prop(&iter)) != NULL)`
    loop {
        // SAFETY: `iter` is the initialized iterator.
        let prop: *const Prop = unsafe { evaluate::next_prop(iter) };
        if prop.is_null() {
            break;
        }
        // `next_prop` yields either a prop in a table this loop leaves alone or,
        // when an override merges, `&raw mut (*iter).tmp` — scratch bytes inside
        // this frame's own iterator that the NEXT `next_prop` call rewrites. The
        // frozen `Const` tag therefore covers one iteration only: the view is
        // re-minted at the top of every pass and its last use (`prop_view.name()`
        // as a call argument, below) precedes the next `next_prop`, so the tag is
        // dead before any write can reach those bytes. Hoisting this mint out of
        // the loop, or reading through the view after the next `next_prop`, is UB.
        // SAFETY: `prop` is non-null (checked) and the live prop the iterator
        // yielded; nothing writes it between this mint and the view's last use.
        let prop_view: &View<Prop, Const> = unsafe { View::<Prop, Const>::from_ptr(prop) };
        if (prop_view.flags().raw()
            & (PropFlags::ANIMATED.raw()
                | PropFlags::OVERRIDDEN.raw()
                | PropFlags::CONNECTED.raw()))
            == 0
        {
            continue;
        }
        if num_anim >= buffer_size {
            break;
        }

        // C: `ufbx_prop *dst = &buffer[num_anim++];`
        // SAFETY: `num_anim < buffer_size` (checked), so `buffer.add(num_anim)`
        // addresses a live slot of the caller's output array.
        let dst: *mut Prop = unsafe { buffer.add(num_anim) };
        num_anim += 1;
        // SAFETY: `dst` is the live output slot and `prop` the live source prop;
        // the copy is a by-value struct assignment.
        unsafe {
            *dst = *prop;
        }

        // SAFETY: `anim` is the live anim; reading its own `ignore_connections`.
        if (prop_view.flags().raw() & PropFlags::CONNECTED.raw()) != 0
            && !unsafe { (*anim).ignore_connections }
        {
            // SAFETY: `dst` is the live output slot and `anim`/`element` the live
            // params; the interned name pointer is a safe read through the view.
            unsafe {
                evaluate::evaluate_connected_prop(
                    dst,
                    anim,
                    element,
                    prop_view.name().data,
                    time,
                    flags,
                )
            };
        }
    }

    // SAFETY: `anim`/`element` are the live params and `buffer` the caller's
    // output array now holding `num_anim` initialized props.
    unsafe { evaluate::evaluate_props(anim, element, time, buffer, num_anim, flags) };

    ret.props.data = buffer;
    // C: `ret.props.count = ret.num_animated = num_anim;`
    ret.props.count = num_anim;
    ret.num_animated = num_anim;
    // C: `ret.defaults = (ufbx_props*)&element->props;` — raw pointer store
    // into the `Option<Ref<Props>>` slot (same layout).
    // SAFETY: `&raw mut ret.defaults` addresses the local `ret`'s own defaults
    // slot (reinterpreted as a raw `Props` pointer), and `&raw const
    // (*element).props` addresses the live element's own props.
    unsafe {
        *(&raw mut ret.defaults as *mut *const Props) = &raw const (*element).props;
    }
    ret
}

// ufbx.c:31025-31028 `ufbx_evaluate_transform`
#[inline(never)]
pub(crate) unsafe fn evaluate_transform(
    anim: *const Anim,
    node: *const Node,
    time: f64,
) -> Transform {
    // SAFETY: `anim`/`node` are this `unsafe fn`'s own live params, forwarded
    // unchanged to `evaluate_transform_flags`.
    unsafe { evaluate_transform_flags(anim, node, time, 0) }
}

// ufbx.c:31030-31060 `ufbxi_transform_props_*` tables — raw pointers into the
// interned-string statics; the wrapper struct provides the `Sync` the raw
// pointers lack (same treatment as `StringTable` in `native::string_pool`).
#[repr(transparent)]
struct PropNameTable<const N: usize>([*const u8; N]);
unsafe impl<const N: usize> Sync for PropNameTable<N> {}

// ufbx.c:31030-31041 `ufbxi_transform_props_all` — the count const is the Rust
// spelling of `ufbxi_arraycount(ufbxi_transform_props_all)`: the initializer
// list is checked against it, and the `buf` scratch array in
// `evaluate_transform_flags` derives from it.
const TRANSFORM_PROPS_ALL_COUNT: usize = 10;
static TRANSFORM_PROPS_ALL: PropNameTable<TRANSFORM_PROPS_ALL_COUNT> = PropNameTable([
    sp::Lcl_Rotation.as_ptr(),
    sp::Lcl_Scaling.as_ptr(),
    sp::Lcl_Translation.as_ptr(),
    sp::PostRotation.as_ptr(),
    sp::PreRotation.as_ptr(),
    sp::RotationOffset.as_ptr(),
    sp::RotationOrder.as_ptr(),
    sp::RotationPivot.as_ptr(),
    sp::ScalingOffset.as_ptr(),
    sp::ScalingPivot.as_ptr(),
]);

// ufbx.c:31043-31048 `ufbxi_transform_props_rotation`
static TRANSFORM_PROPS_ROTATION: PropNameTable<4> = PropNameTable([
    sp::Lcl_Rotation.as_ptr(),
    sp::PostRotation.as_ptr(),
    sp::PreRotation.as_ptr(),
    sp::RotationOrder.as_ptr(),
]);

// ufbx.c:31050-31052 `ufbxi_transform_props_scale`
static TRANSFORM_PROPS_SCALE: PropNameTable<1> = PropNameTable([sp::Lcl_Scaling.as_ptr()]);

// ufbx.c:31054-31060 `ufbxi_transform_props_rotation_scale`
static TRANSFORM_PROPS_ROTATION_SCALE: PropNameTable<5> = PropNameTable([
    sp::Lcl_Rotation.as_ptr(),
    sp::Lcl_Scaling.as_ptr(),
    sp::PostRotation.as_ptr(),
    sp::PreRotation.as_ptr(),
    sp::RotationOrder.as_ptr(),
]);

// ufbx.c:31062-31160 `ufbx_evaluate_transform_flags`
#[inline(never)]
pub(crate) unsafe fn evaluate_transform_flags(
    anim: *const Anim,
    node: *const Node,
    time: f64,
    flags: u32,
) -> Transform {
    let mut flags = flags;
    ufbx_assert!(!anim.is_null());
    ufbx_assert!(!node.is_null());
    if node.is_null() {
        return IDENTITY_TRANSFORM;
    }
    if anim.is_null() {
        // SAFETY: `node` is non-null (checked) and points at a live `Node` — the
        // raw-pointer contract of this `unsafe fn`; reading its own transform.
        return unsafe { (*node).local_transform };
    }
    // SAFETY: live `node` per above; reading its own `is_root` flag.
    if unsafe { (*node).is_root } {
        // SAFETY: as above.
        return unsafe { (*node).local_transform };
    }

    if (flags & TransformFlags::EXPLICIT_INCLUDES.raw()) == 0 {
        flags |= TransformFlags::INCLUDE_ROTATION.raw()
            | TransformFlags::INCLUDE_SCALE.raw()
            | TransformFlags::INCLUDE_TRANSLATION.raw();
    }

    let mut prop_names: *const *const u8 = TRANSFORM_PROPS_ALL.0.as_ptr();
    let mut num_prop_names: usize = TRANSFORM_PROPS_ALL.0.len();
    let components: u32 = flags
        & (TransformFlags::INCLUDE_ROTATION.raw()
            | TransformFlags::INCLUDE_SCALE.raw()
            | TransformFlags::INCLUDE_TRANSLATION.raw());
    if components == (TransformFlags::INCLUDE_ROTATION.raw() | TransformFlags::INCLUDE_SCALE.raw())
    {
        prop_names = TRANSFORM_PROPS_ROTATION_SCALE.0.as_ptr();
        num_prop_names = TRANSFORM_PROPS_ROTATION_SCALE.0.len();
    } else if components == TransformFlags::INCLUDE_ROTATION.raw() {
        prop_names = TRANSFORM_PROPS_ROTATION.0.as_ptr();
        num_prop_names = TRANSFORM_PROPS_ROTATION.0.len();
    } else if components == TransformFlags::INCLUDE_SCALE.raw() {
        prop_names = TRANSFORM_PROPS_SCALE.0.as_ptr();
        num_prop_names = TRANSFORM_PROPS_SCALE.0.len();
    } else if components == 0 {
        return IDENTITY_TRANSFORM;
    }

    let mut translation_scale: *const Vec3 = core::ptr::null();
    let mut helper_scale = MaybeUninit::<Prop>::uninit(); // ufbxi_uninit
    let mut scale_factor: Vec3 = ONE_VEC3;
    let mut use_scale_factor: bool = false;

    // SAFETY: `&raw const (*node).parent` addresses the live `node`'s own parent
    // ref, which `opt_ptr` unwraps to a nullable node pointer.
    if !unsafe { opt_ptr(&raw const (*node).parent) }.is_null()
        && (flags
            & (TransformFlags::INCLUDE_SCALE.raw() | TransformFlags::INCLUDE_TRANSLATION.raw()))
            != 0
    {
        // SAFETY: as above; `parent` is the live parent node (non-null here).
        let parent: *mut Node = unsafe { opt_ptr(&raw const (*node).parent) };

        // SAFETY: `&raw const (*parent).inherit_scale_node` addresses the live
        // parent's own ref, unwrapped by `opt_ptr`.
        if (flags & TransformFlags::IGNORE_COMPONENTWISE_SCALE.raw()) == 0
            && !unsafe { opt_ptr(&raw const (*parent).inherit_scale_node) }.is_null()
        {
            // SAFETY: as above.
            let mut p: *mut Node = unsafe { opt_ptr(&raw const (*parent).inherit_scale_node) };

            // SAFETY: live `node` per above; reading its own `is_scale_helper`.
            if unsafe { (*node).is_scale_helper } {
                use_scale_factor = true;
            }

            // SAFETY: `p` is null-or-live and `&raw const (*p).scale_helper`
            // addresses its own ref when live, unwrapped by `opt_ptr`.
            while !p.is_null() && !unsafe { opt_ptr(&raw const (*p).scale_helper) }.is_null() {
                // C: `ufbx_prop scale = ufbx_evaluate_prop(anim, &p->scale_helper->element, ufbxi_Lcl_Scaling, time);`
                // SAFETY: `p`'s scale-helper is non-null (loop condition), so
                // `&raw const (*helper).element` addresses its live element;
                // `anim` is the live anim param.
                let scale: Prop = unsafe {
                    evaluate_prop(
                        anim,
                        &raw const (*opt_ptr(&raw const (*p).scale_helper)).element,
                        sp::Lcl_Scaling.as_ptr(),
                        time,
                    )
                };
                // C: `scale.value_vec3.{x,y,z}` — the value union's 3-real view.
                scale_factor.x *= scale.value_vec4.x;
                scale_factor.y *= scale.value_vec4.y;
                scale_factor.z *= scale.value_vec4.z;
                // SAFETY: live `p` here; `&raw const (*p).inherit_scale_node`
                // addresses its own ref, unwrapped by `opt_ptr`.
                p = unsafe { opt_ptr(&raw const (*p).inherit_scale_node) };
            }
        }

        // SAFETY: `&raw const (*parent).scale_helper` addresses the live parent's
        // own ref, unwrapped by `opt_ptr`.
        if !unsafe { opt_ptr(&raw const (*parent).scale_helper) }.is_null()
            && (flags & TransformFlags::IGNORE_SCALE_HELPER.raw()) == 0
        {
            // SAFETY: the parent's scale-helper is non-null (checked), so
            // `&raw const (*helper).element` addresses its live element; `anim` is
            // the live anim param.
            helper_scale.write(unsafe {
                evaluate_prop(
                    anim,
                    &raw const (*opt_ptr(&raw const (*parent).scale_helper)).element,
                    sp::Lcl_Scaling.as_ptr(),
                    time,
                )
            });
            let hs: *mut Prop = helper_scale.as_mut_ptr();
            // SAFETY: `hs` is the just-written local `Prop` storage; reading and
            // writing its own value fields.
            if (unsafe { (*hs).flags.raw() } & PropFlags::NOT_FOUND.raw()) != 0 {
                // SAFETY: as above.
                unsafe {
                    (*hs).value_vec4.x = 1.0;
                    (*hs).value_vec4.y = 1.0;
                    (*hs).value_vec4.z = 1.0;
                }
            }
            // SAFETY: as above.
            unsafe {
                (*hs).value_vec4.x *= scale_factor.x;
                (*hs).value_vec4.y *= scale_factor.y;
                (*hs).value_vec4.z *= scale_factor.z;
            }
            // C: `translation_scale = &helper_scale.value_vec3;`
            // SAFETY: `&raw const (*hs).value_vec4` addresses the local prop's own
            // value, reinterpreted as the `Vec3` translation scale.
            translation_scale = unsafe { &raw const (*hs).value_vec4 as *const Vec3 };
        }
    }

    let mut eval_flags: u32 = 0;
    if (flags & TransformFlags::NO_EXTRAPOLATION.raw()) != 0 {
        eval_flags |= EvaluateFlags::NO_EXTRAPOLATION.raw();
    }

    // C: `ufbx_prop buf[ufbxi_arraycount(ufbxi_transform_props_all)]; // ufbxi_uninit`
    let mut buf = MaybeUninit::<[Prop; TRANSFORM_PROPS_ALL_COUNT]>::uninit(); // ufbxi_uninit
                                                                              // SAFETY: `anim` is live, the raw field address identifies the node's own
                                                                              // element, `buf` is this frame's correctly sized scratch storage, and
                                                                              // `prop_names`/`num_prop_names` describe the static name table.
    let props: Props = unsafe {
        evaluate::evaluate_selected_props(
            anim,
            &raw const (*node).element,
            time,
            buf.as_mut_ptr() as *mut Prop,
            prop_names,
            num_prop_names,
            eval_flags,
        )
    };
    // C: `(ufbx_rotation_order)ufbxi_find_enum(...)` — clamped to the valid range.
    // The const view carries the local `props` value's read-only provenance.
    // SAFETY: `find_enum` clamps its result into `[Xyz, Spheric]`, every value of
    // which is a valid `#[repr(u32)]` `RotationOrder` discriminant; `&raw const
    // props` roots a read-only view over the local `props`.
    let order: RotationOrder = unsafe {
        core::mem::transmute::<u32, RotationOrder>(find_enum(
            View::<Props, Const>::from_ptr(&raw const props),
            &sp::RotationOrder,
            RotationOrder::Xyz as i64,
            RotationOrder::Spheric as i64,
        ) as u32)
    };

    // C: `ufbx_transform transform; // ufbxi_uninit`
    let mut transform = MaybeUninit::<Transform>::uninit(); // ufbxi_uninit
    let t: *mut Transform = transform.as_mut_ptr();
    if (components & TransformFlags::INCLUDE_TRANSLATION.raw()) != 0 {
        // SAFETY: `t` is the local transform storage; `get_transform` reads a
        // read-only view over the local `props` and the live `node`, and the
        // result is written into `t`.
        unsafe {
            core::ptr::write(
                t,
                get_transform(
                    View::<Props, Const>::from_ptr(&raw const props),
                    order,
                    node,
                    translation_scale,
                ),
            );
        }
    } else {
        // SAFETY: `t` is the local transform storage; writing its own field.
        unsafe {
            (*t).translation = ZERO_VEC3;
        }
        if (components & TransformFlags::INCLUDE_ROTATION.raw()) != 0 {
            // SAFETY: `t` is the local transform; `get_rotation` reads a read-only
            // view over the local `props` and the live `node`.
            unsafe {
                (*t).rotation = get_rotation(
                    View::<Props, Const>::from_ptr(&raw const props),
                    order,
                    node,
                );
            }
        } else {
            // SAFETY: local transform storage; writing its own field.
            unsafe {
                (*t).rotation = IDENTITY_QUAT;
            }
        }
        if (components & TransformFlags::INCLUDE_SCALE.raw()) != 0 {
            // SAFETY: `t` is the local transform; `get_scale` reads a read-only
            // view over the local `props` and the live `node`.
            unsafe {
                (*t).scale = get_scale(View::<Props, Const>::from_ptr(&raw const props), node);
            }
        } else {
            // SAFETY: local transform storage; writing its own field.
            unsafe {
                (*t).scale = ONE_VEC3;
            }
        }
    }

    if use_scale_factor {
        // SAFETY: `t` is the local transform, fully initialized above; scaling
        // its own components.
        unsafe {
            (*t).scale.x *= scale_factor.x;
            (*t).scale.y *= scale_factor.y;
            (*t).scale.z *= scale_factor.z;
        }
    }
    // SAFETY: `transform` is fully initialized on every path above.
    unsafe { transform.assume_init() }
}

// ufbx.c:31162-31165 `ufbx_evaluate_blend_weight`
// `anim`/`channel` carry C's non-nullable `const ufbx_anim *` /
// `const ufbx_blend_channel *` as mode-generic read views.
pub(crate) fn evaluate_blend_weight<M: Mode>(
    anim: &View<Anim, M>,
    channel: &View<BlendChannel, M>,
    time: f64,
) -> Real {
    evaluate_blend_weight_flags(anim, channel, time, 0)
}

// ufbx.c:31167-31176 `ufbx_evaluate_blend_weight_flags`
// `anim`/`channel` carry C's non-nullable `const ufbx_anim *` /
// `const ufbx_blend_channel *` as mode-generic read views.
pub(crate) fn evaluate_blend_weight_flags<M: Mode>(
    anim: &View<Anim, M>,
    channel: &View<BlendChannel, M>,
    time: f64,
    flags: u32,
) -> Real {
    // C: `const char *prop_names[] = { ufbxi_DeformPercent, };` — the count
    // const is the Rust spelling of `ufbxi_arraycount(prop_names)`: the array
    // literal is checked against it, and `buf` below derives from it.
    const NUM_PROP_NAMES: usize = 1;
    let prop_names: [*const u8; NUM_PROP_NAMES] = [sp::DeformPercent.as_ptr()];

    // C: `ufbx_prop buf[ufbxi_arraycount(prop_names)]; // ufbxi_uninit`
    let mut buf = MaybeUninit::<[Prop; NUM_PROP_NAMES]>::uninit(); // ufbxi_uninit
                                                                   // SAFETY: the view params root live `Anim`/`BlendChannel` objects, so
                                                                   // `as_ptr()` and `element_ptr()` (the addr-of parity for C's
                                                                   // `&channel->element`) are live read pointers; `buf` is correctly sized
                                                                   // scratch storage, and `prop_names` describes the name table.
    let props: Props = unsafe {
        evaluate::evaluate_selected_props(
            anim.as_ptr(),
            channel.element_ptr(),
            time,
            buf.as_mut_ptr() as *mut Prop,
            prop_names.as_ptr(),
            prop_names.len(),
            flags,
        )
    };
    // C: `ufbxi_find_real(&props, ufbxi_DeformPercent, channel->weight * (ufbx_real)100.0) * (ufbx_real)0.01`
    // Const view: same read-only defaults-chain provenance as above.
    // SAFETY: `&raw const props` roots a read-only view over the local `props`,
    // which is not written while the view is live.
    (unsafe {
        ufbxi_find_real(
            View::<Props, Const>::from_ptr(&raw const props),
            &sp::DeformPercent,
            channel.weight() * (100.0 as Real),
        )
    }) * (0.01 as Real)
}

// ufbx.c:31178-31192 `ufbx_evaluate_scene`
// C forks on `#if UFBXI_FEATURE_SCENE_EVALUATION`; the arms are split into
// cfg-gated fns (the same split `ufbxi_obj_load` uses in `native::obj`). The
// `#else` arm is C parity, NOT a stub: a build without `feature = "scene-eval"`
// must report `UFBX_ERROR_FEATURE_DISABLED` exactly like a C build with
// `UFBX_MINIMAL`. Note that `ufbxi_check_opts_ptr` (ufbx.c:31180) sits BEFORE
// the `#if`, so both arms run it.
#[cfg(feature = "scene-eval")]
pub(crate) unsafe fn evaluate_scene(
    scene: *const Scene,
    anim: *const Anim,
    time: f64,
    opts: *const RawEvaluateOpts,
) -> Result<*mut Scene, Error> {
    // SAFETY: `opts` is null-or-live per this fn's contract; the macro reads its
    // sentinel fields only when non-null.
    unsafe { ufbxi_check_opts_res!(opts) };
    // C: `ufbxi_eval_context ec = { 0 };`
    let ec = evaluate::EvalContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::zeroed()));
    // SAFETY: `&ec` is the fresh zeroed eval context; `scene` is live and `anim`
    // null-or-live (the callee substitutes `scene.anim` on null) per this fn's
    // contract, `opts` null-or-live, forwarded unchanged.
    unsafe { evaluate::evaluate_scene(&ec, scene as *mut Scene, anim, time, opts) }
}

#[cfg(not(feature = "scene-eval"))]
pub(crate) unsafe fn evaluate_scene(
    scene: *const Scene,
    anim: *const Anim,
    time: f64,
    opts: *const RawEvaluateOpts,
) -> Result<*mut Scene, Error> {
    // SAFETY: `opts` is null-or-live per this fn's contract; the macro reads its
    // sentinel fields only when non-null.
    unsafe { ufbxi_check_opts_res!(opts) };
    // C: `scene`/`anim`/`time` are unreferenced in the `#else` arm.
    let _ = (scene, anim, time);
    // C zero-fills the caller slot then formats into it; the `Result` shape
    // builds the same bytes in a local carried by `Err` (the shim owns the
    // slot writes).
    let mut error: Error = Error::default();
    // SAFETY: `&raw mut error` is this frame's live `Error` slot the `%s`-less
    // format writes into.
    unsafe { ufbxi_fmt_err_info!(&raw mut error, "UFBX_ENABLE_SCENE_EVALUATION") };
    ufbxi_report_err_msg!(
        // SAFETY: same live local `Error` slot, minted as a view for the report.
        unsafe { crate::native::error::ErrorView::from_ptr(&raw mut error) },
        "UFBXI_FEATURE_SCENE_EVALUATION",
        "Feature disabled"
    );
    Err(error)
}

// ufbx.c:31194-31218 `ufbx_create_anim`
// No `#if` fork in C: it drives `ufbxi_create_anim_context` /
// `ufbxi_create_anim_imp` (`native::evaluate`) unconditionally.
pub(crate) unsafe fn create_anim(
    scene: *const Scene,
    opts: *const RawAnimOpts,
) -> Result<*mut Anim, Error> {
    // SAFETY: `opts` is null-or-live per this fn's contract; the macro reads its
    // sentinel fields only when non-null.
    unsafe { ufbxi_check_opts_res!(opts) };
    ufbx_assert!(!scene.is_null());

    // C: `ufbxi_create_anim_context ac = { UFBX_ERROR_NONE };`
    let ac =
        evaluate::CreateAnimContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::zeroed()));
    if !opts.is_null() {
        // C: `ac->opts = *opts;` (struct assignment)
        // SAFETY: `opts` is non-null (checked) and the caller's live opts;
        // `ac.opts_mut_ptr()` is the context's own opts slot — distinct
        // allocations, so the single-element copy is non-overlapping.
        unsafe {
            core::ptr::copy_nonoverlapping(opts, ac.opts_mut_ptr(), 1);
        }
    }

    ac.set_scene(scene);

    // C: `int ok = ufbxi_create_anim_imp(&ac);` — on success the `FinishedImp`
    // carries the finished imp to the return below.
    let result = evaluate::create_anim_imp(&ac);

    if let Ok(finished_imp) = result {
        // C: `return &ac->imp->anim;` — commit the finished imp across the ABI.
        // (The success-path `clear_error` of the caller's slot lives in the
        // boundary shim.)
        Ok(finished_imp.into_payload())
    } else {
        // C copies the fixed error into the caller's slot; the `Result` shape
        // carries it by value (the shim owns the slot writes).
        let mut fixed: Error = Error::default();
        // SAFETY: `ac.error_mut_ptr()` is the context's own error slot and
        // `&raw mut fixed` this frame's live `Error`, which `fix_error_type`
        // accepts.
        unsafe {
            fix_error_type(
                ac.error_mut_ptr(),
                b"Failed to create anim\0",
                &raw mut fixed,
            );
        }
        // SAFETY: `ac.result_mut_ptr()` is the context's own result buffer.
        unsafe { buf_free(ac.result_mut_ptr()) };
        // SAFETY: `ac.ator_result_mut_ptr()` is the context's own result allocator.
        unsafe { free_ator(ac.ator_result_mut_ptr()) };
        Err(fixed)
    }
}

// ufbx.c:31220-31229 `ufbx_free_anim`
pub(crate) unsafe fn free_anim(anim: *mut Anim) {
    if anim.is_null() {
        return;
    }
    // SAFETY: `anim` is non-null (checked) and points at a live `Anim` — the
    // raw-pointer contract of this `unsafe fn`; reading its own `custom` flag.
    if !unsafe { (*anim).custom } {
        return;
    }

    // SAFETY: the custom `anim` is the payload of a live `AnimImp` handed out
    // by this library — the raw-pointer contract of this `unsafe fn`.
    let imp = unsafe { ImpHandle::<AnimImp>::from_payload(anim) };
    ufbx_assert!(imp.has_magic());
    if !imp.has_magic() {
        return;
    }
    imp.release();
}

// ufbx.c:31231-31240 `ufbx_retain_anim`
pub(crate) unsafe fn retain_anim(anim: *mut Anim) {
    if anim.is_null() {
        return;
    }
    // SAFETY: `anim` is non-null (checked) and points at a live `Anim` — the
    // raw-pointer contract of this `unsafe fn`; reading its own `custom` flag.
    if !unsafe { (*anim).custom } {
        return;
    }

    // SAFETY: the custom `anim` is the payload of a live `AnimImp` handed out
    // by this library — the raw-pointer contract of this `unsafe fn`.
    let imp = unsafe { ImpHandle::<AnimImp>::from_payload(anim) };
    ufbx_assert!(imp.has_magic());
    if !imp.has_magic() {
        return;
    }
    imp.retain();
}

// ufbx.c:31242-31289 `ufbx_bake_anim`
// Same `#if` split as `ufbx_evaluate_scene` above: the
// `UFBXI_FEATURE_ANIMATION_BAKING` arm uses `ufbxi_bake_context` /
// `ufbxi_bake_anim_imp` (ufbx.c:26687-26723 / 27707-27765,
// `native::evaluate`). Note that `ufbx_assert(scene)` sits BEFORE the `#if`
// (so both arms run it) while `ufbxi_check_opts_ptr` sits INSIDE the enabled
// arm — do not hoist it.
#[cfg(feature = "baking")]
pub(crate) unsafe fn bake_anim(
    scene: *const Scene,
    anim: *const Anim,
    opts: *const RawBakeOpts,
) -> Result<*mut BakedAnim, Error> {
    ufbx_assert!(!scene.is_null());
    // SAFETY: `opts` is null-or-live per this fn's contract; the macro reads its
    // sentinel fields only when non-null.
    unsafe { ufbxi_check_opts_res!(opts) };
    let mut anim = anim;
    if anim.is_null() {
        // SAFETY: `scene` is non-null (asserted) and points at a live `Scene` —
        // the raw-pointer contract of this `unsafe fn`; `&raw const (*scene).anim`
        // addresses its own default anim ref, which `ref_ptr` follows.
        anim = unsafe { ref_ptr(&raw const (*scene).anim) };
    }

    // C: `ufbxi_bake_context bc = { UFBX_ERROR_NONE };`
    let bc = evaluate::BakeContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::zeroed()));
    if !opts.is_null() {
        // C: `bc->opts = *opts;` (struct assignment)
        // SAFETY: `opts` is non-null (checked) and the caller's live opts;
        // `bc.opts_mut_ptr()` is the context's own opts slot — distinct
        // allocations, so the single-element copy is non-overlapping.
        unsafe {
            core::ptr::copy_nonoverlapping(opts, bc.opts_mut_ptr(), 1);
        }
    }

    bc.set_scene(scene);

    // C: `int ok = ufbxi_bake_anim_imp(&bc, anim);`
    // SAFETY: `&bc` is the fresh bake context and `anim` is the live anim (the
    // scene default when the param was null).
    let ok = unsafe { evaluate::bake_anim_imp(&bc, anim) };

    // SAFETY: each `*_mut_ptr()` accessor yields the bake context's own temp
    // buffer, freed once here.
    unsafe {
        buf_free(bc.tmp_mut_ptr());
        buf_free(bc.tmp_prop_mut_ptr());
        buf_free(bc.tmp_times_mut_ptr());
        buf_free(bc.tmp_bake_props_mut_ptr());
        buf_free(bc.tmp_nodes_mut_ptr());
        buf_free(bc.tmp_elements_mut_ptr());
        buf_free(bc.tmp_props_mut_ptr());
        buf_free(bc.tmp_bake_stack_mut_ptr());
    }
    // C: `ufbxi_free(&bc->ator_tmp, char, bc->tmp_arr, bc->tmp_arr_size);`
    // SAFETY: `bc.ator_tmp_mut_ptr()` is the context's own temp allocator and
    // `bc.tmp_arr()`/`bc.tmp_arr_size()` the block it allocated from it.
    unsafe { free::<u8>(bc.ator_tmp_mut_ptr(), bc.tmp_arr(), bc.tmp_arr_size()) };
    // SAFETY: `bc.ator_tmp_mut_ptr()` is the context's own temp allocator.
    unsafe { free_ator(bc.ator_tmp_mut_ptr()) };

    if ok.is_ok() {
        let imp: *mut BakedAnimImp = bc.imp();
        // SAFETY: `imp` is the context's live baked-anim imp; `&raw mut
        // (*imp).bake` addresses its own `bake` field. (The success-path
        // `clear_error` of the caller's slot lives in the boundary shim.)
        Ok(unsafe { &raw mut (*imp).bake })
    } else {
        // C copies the fixed error into the caller's slot; the `Result` shape
        // carries it by value (the shim owns the slot writes).
        let mut fixed: Error = Error::default();
        // SAFETY: `bc.error_mut_ptr()` is the context's own error slot and
        // `&raw mut fixed` this frame's live `Error`, which `fix_error_type`
        // accepts.
        unsafe {
            fix_error_type(bc.error_mut_ptr(), b"Failed to bake anim\0", &raw mut fixed);
        }
        // SAFETY: `bc.result_mut_ptr()` is the context's own result buffer.
        unsafe { buf_free(bc.result_mut_ptr()) };
        // SAFETY: `bc.ator_result_mut_ptr()` is the context's own result allocator.
        unsafe { free_ator(bc.ator_result_mut_ptr()) };
        Err(fixed)
    }
}

#[cfg(not(feature = "baking"))]
pub(crate) unsafe fn bake_anim(
    scene: *const Scene,
    anim: *const Anim,
    opts: *const RawBakeOpts,
) -> Result<*mut BakedAnim, Error> {
    ufbx_assert!(!scene.is_null());
    // C: `anim`/`opts` are unreferenced in the `#else` arm.
    let _ = (anim, opts);
    // C zero-fills the caller slot then formats into it; the `Result` shape
    // builds the same bytes in a local carried by `Err` (the shim owns the
    // slot writes).
    let mut error: Error = Error::default();
    // SAFETY: `&raw mut error` is this frame's live `Error` slot the `%s`-less
    // format writes into.
    unsafe { ufbxi_fmt_err_info!(&raw mut error, "UFBX_ENABLE_ANIMATION_BAKING") };
    ufbxi_report_err_msg!(
        // SAFETY: same live local `Error` slot, minted as a view for the report.
        unsafe { crate::native::error::ErrorView::from_ptr(&raw mut error) },
        "UFBXI_FEATURE_ANIMATION_BAKING",
        "Feature disabled"
    );
    Err(error)
}

// ufbx.c:31291-31299 `ufbx_retain_baked_anim`
// `ufbxi_baked_anim_imp` is declared outside C's baking `#if`, so this pair
// works in every build (see `native::evaluate`).
pub(crate) unsafe fn retain_baked_anim(bake: *mut BakedAnim) {
    if bake.is_null() {
        return;
    }

    // SAFETY: the non-null `bake` is the payload of a live `BakedAnimImp`
    // handed out by this library — the raw-pointer contract of this `unsafe fn`.
    let imp = unsafe { ImpHandle::<BakedAnimImp>::from_payload(bake) };
    ufbx_assert!(imp.has_magic());
    if !imp.has_magic() {
        return;
    }
    imp.retain();
}

// ufbx.c:31301-31309 `ufbx_free_baked_anim`
pub(crate) unsafe fn free_baked_anim(bake: *mut BakedAnim) {
    if bake.is_null() {
        return;
    }

    // SAFETY: the non-null `bake` is the payload of a live `BakedAnimImp`
    // handed out by this library — the raw-pointer contract of this `unsafe fn`.
    let imp = unsafe { ImpHandle::<BakedAnimImp>::from_payload(bake) };
    ufbx_assert!(imp.has_magic());
    if !imp.has_magic() {
        return;
    }
    imp.release();
}

// ufbx.c:31312-31318 `ufbx_find_baked_node_by_typed_id`
// C-parity: no null check on `bake` — the C body dereferences it directly.

// Mode-generic read accessors for the baked-anim finders.
impl<M: Mode> View<BakedAnim, M> {
    #[inline(always)]
    pub(crate) fn nodes_data(&self) -> *mut BakedNode {
        // SAFETY: reading the `nodes.data` run pointer (stored value).
        unsafe { (*self.as_ptr()).nodes.data as *mut BakedNode }
    }
    #[inline(always)]
    pub(crate) fn nodes_count(&self) -> usize {
        // SAFETY: reading the `nodes.count` field of a valid `BakedAnim`.
        unsafe { (*self.as_ptr()).nodes.count }
    }
    #[inline(always)]
    pub(crate) fn elements_data(&self) -> *mut BakedElement {
        // SAFETY: reading the `elements.data` run pointer (stored value).
        unsafe { (*self.as_ptr()).elements.data as *mut BakedElement }
    }
    #[inline(always)]
    pub(crate) fn elements_count(&self) -> usize {
        // SAFETY: reading the `elements.count` field of a valid `BakedAnim`.
        unsafe { (*self.as_ptr()).elements.count }
    }
}

impl<M: Mode> View<Node, M> {
    #[inline(always)]
    pub(crate) fn element_typed_id(&self) -> u32 {
        // SAFETY: reading the embedded header's `typed_id` of a valid `Node`.
        unsafe { (*self.as_ptr()).element.typed_id }
    }
}

pub(crate) fn find_baked_node_by_typed_id<M: Mode>(
    bake: &View<BakedAnim, M>,
    typed_id: u32,
) -> Option<&View<BakedNode, M>> {
    let mut index: usize = usize::MAX;
    // SAFETY: binary search over the stored `nodes` run of a valid `BakedAnim`
    // (in-bounds derefs of the run macro_lower_bound_eq walks).
    unsafe {
        macro_lower_bound_eq::<BakedNode>(
            8,
            &mut index,
            bake.nodes_data(),
            0,
            bake.nodes_count(),
            |a| (*a).typed_id < typed_id,
            |a| (*a).typed_id == typed_id,
        );
    }
    if index < usize::MAX {
        // SAFETY: in-bounds element of the stored (write-provenance) run,
        // correlated to `bake`'s borrow; mode-generic mint.
        Some(unsafe { View::<BakedNode, M>::mint(bake.nodes_data().add(index)) })
    } else {
        None
    }
}

// ufbx.c:31320-31324 `ufbx_find_baked_node`
pub(crate) fn find_baked_node<'a, M: Mode>(
    bake: Option<&'a View<BakedAnim, M>>,
    node: Option<&View<Node, M>>,
) -> Option<&'a View<BakedNode, M>> {
    // C: `if (!bake || !node) return NULL;`
    let (Some(bake), Some(node)) = (bake, node) else {
        return None;
    };
    find_baked_node_by_typed_id(bake, node.element_typed_id())
}

// ufbx.c:31326-31332 `ufbx_find_baked_element_by_element_id`
// C-parity: no null check on `bake`, as above.
pub(crate) fn find_baked_element_by_element_id<M: Mode>(
    bake: &View<BakedAnim, M>,
    element_id: u32,
) -> Option<&View<BakedElement, M>> {
    let mut index: usize = usize::MAX;
    // SAFETY: binary search over the stored `elements` run of a valid
    // `BakedAnim` (in-bounds derefs of the run macro_lower_bound_eq walks).
    unsafe {
        macro_lower_bound_eq::<BakedElement>(
            8,
            &mut index,
            bake.elements_data(),
            0,
            bake.elements_count(),
            |a| (*a).element_id < element_id,
            |a| (*a).element_id == element_id,
        );
    }
    if index < usize::MAX {
        // SAFETY: in-bounds element of the stored run; mode-generic mint.
        Some(unsafe { View::<BakedElement, M>::mint(bake.elements_data().add(index)) })
    } else {
        None
    }
}

// ufbx.c:31334-31338 `ufbx_find_baked_element`
pub(crate) fn find_baked_element<'a, M: Mode>(
    bake: Option<&'a View<BakedAnim, M>>,
    element: Option<&View<Element, M>>,
) -> Option<&'a View<BakedElement, M>> {
    // C: `if (!bake || !element) return NULL;`
    let (Some(bake), Some(element)) = (bake, element) else {
        return None;
    };
    find_baked_element_by_element_id(bake, element.element_id())
}

// ufbx.c:31340-31370 `ufbx_evaluate_baked_vec3`
// PORT DIVERGENCE (ufbx.c:31369): upstream's trailing
// `keyframes.data[keyframes.count - 1]` reads `data[SIZE_MAX]` for an empty
// list (`count - 1` wraps), an out-of-bounds read reachable from any caller
// passing an empty keyframe list. The empty case returns `ZERO_VEC3` here;
// reconcile once upstream lands the fix.
pub(crate) unsafe fn evaluate_baked_vec3(keyframes: List<BakedVec3>, time: f64) -> Vec3 {
    let mut begin: usize = 0;
    let mut end: usize = keyframes.count;
    let keys: *const BakedVec3 = keyframes.data;
    while end - begin >= 8 {
        let mid: usize = (begin + end) >> 1;
        // SAFETY: `mid < end <= keyframes.count`, so `keys.add(mid)` addresses a
        // live `BakedVec3` of the caller's keyframe run.
        if unsafe { (*keys.add(mid)).time } <= time {
            begin = mid + 1;
        } else {
            end = mid;
        }
    }

    end = keyframes.count;
    // C: `for (; begin < end; begin++)` — every path out of the body either
    // `continue`s (the only one that advances `begin`) or returns.
    while begin < end {
        // SAFETY: `begin < end <= keyframes.count`, so `keys.add(begin)` addresses
        // a live keyframe of the run.
        let next: *const BakedVec3 = unsafe { keys.add(begin) };
        // SAFETY: `next` is the live keyframe just indexed.
        if unsafe { (*next).time } <= time {
            begin += 1;
            continue;
        }
        if begin == 0 {
            // SAFETY: `next` is the live keyframe just indexed.
            return unsafe { (*next).value };
        }

        // SAFETY: `begin >= 1` here, so `next.sub(1)` addresses the previous live
        // keyframe of the run.
        let mut prev: *const BakedVec3 = unsafe { next.sub(1) };
        // SAFETY: `prev > keys` guards the `prev.sub(1)` read to stay within the
        // run; `prev`/`prev-1` are live keyframes.
        if prev > keys
            && unsafe {
                ((*prev).flags & BakedKeyFlags::STEP_RIGHT).any() && (*prev.sub(1)).time == time
            }
        {
            // SAFETY: `prev > keys` (checked), so `prev.sub(1)` stays in the run.
            prev = unsafe { prev.sub(1) };
        }
        // SAFETY: `prev` is a live keyframe of the run.
        if time == unsafe { (*prev).time } {
            // SAFETY: as above.
            return unsafe { (*prev).value };
        }
        // SAFETY: `prev`/`next` are live keyframes of the run.
        let mut t: f64 = (time - unsafe { (*prev).time }) / unsafe { (*next).time - (*prev).time };
        // SAFETY: `prev` is a live keyframe.
        if unsafe { ((*prev).flags & BakedKeyFlags::STEP_LEFT).any() } {
            t = 0.0;
        }
        // SAFETY: `next` is a live keyframe.
        if unsafe { ((*next).flags & BakedKeyFlags::STEP_RIGHT).any() } {
            t = 1.0;
        }
        // SAFETY: `prev`/`next` are live keyframes of the run.
        return unsafe { lerp3((*prev).value, (*next).value, t as Real) };
    }

    // PORT DIVERGENCE (ufbx.c:31369): guard the empty list (see fn header).
    if keyframes.count == 0 {
        return ZERO_VEC3;
    }
    // SAFETY: `count >= 1` (guarded above), so `count - 1` addresses the last
    // live keyframe of the run — the clamp value when no keyframe is past `time`.
    unsafe { (*keyframes.data.add(keyframes.count - 1)).value }
}

// ufbx.c:31372-31403 `ufbx_evaluate_baked_quat`
// NOT a copy of `ufbx_evaluate_baked_vec3` with the type swapped: the first
// `prev--` test (ufbx.c:31393) has no `UFBX_BAKED_KEY_STEP_RIGHT` condition
// (contrast ufbx.c:31361), and the flag-carrying `prev--` happens AFTER `t` is
// computed. On a duplicated key time the two functions therefore select
// different keys unless the middle key carries the flag — see
// `test_evaluate_baked_quat_step_asymmetry`.
//
// The second `prev--` (ufbx.c:31396) is in fact unreachable: reaching it needs
// the first test to have been false, and both tests read the same `prev[-1]
// .time == time` (if the first fired, `time == prev->time` returns right
// after it). Ported verbatim anyway — it is the C source text.
pub(crate) unsafe fn evaluate_baked_quat(keyframes: List<BakedQuat>, time: f64) -> Quat {
    let mut begin: usize = 0;
    let mut end: usize = keyframes.count;
    let keys: *const BakedQuat = keyframes.data;
    while end - begin >= 8 {
        let mid: usize = (begin + end) >> 1;
        // SAFETY: `mid < end <= keyframes.count`, so `keys.add(mid)` addresses a
        // live `BakedQuat` of the caller's keyframe run.
        if unsafe { (*keys.add(mid)).time } <= time {
            begin = mid + 1;
        } else {
            end = mid;
        }
    }

    end = keyframes.count;
    while begin < end {
        // SAFETY: `begin < end <= keyframes.count`, so `keys.add(begin)` addresses
        // a live keyframe of the run.
        let next: *const BakedQuat = unsafe { keys.add(begin) };
        // SAFETY: `next` is the live keyframe just indexed.
        if unsafe { (*next).time } <= time {
            begin += 1;
            continue;
        }
        if begin == 0 {
            // SAFETY: `next` is the live keyframe just indexed.
            return unsafe { (*next).value };
        }

        // SAFETY: `begin >= 1` here, so `next.sub(1)` addresses the previous live
        // keyframe of the run.
        let mut prev: *const BakedQuat = unsafe { next.sub(1) };
        // SAFETY: `prev > keys` guards the `prev.sub(1)` read to stay within the
        // run; `prev-1` is a live keyframe.
        if prev > keys && unsafe { (*prev.sub(1)).time } == time {
            // SAFETY: `prev > keys` (checked), so `prev.sub(1)` stays in the run.
            prev = unsafe { prev.sub(1) };
        }
        // SAFETY: `prev` is a live keyframe of the run.
        if time == unsafe { (*prev).time } {
            // SAFETY: as above.
            return unsafe { (*prev).value };
        }
        // SAFETY: `prev`/`next` are live keyframes of the run.
        let mut t: f64 = (time - unsafe { (*prev).time }) / unsafe { (*next).time - (*prev).time };
        // SAFETY: `prev > keys` guards the `prev.sub(1)` read; `prev`/`prev-1` are
        // live keyframes of the run.
        if prev > keys
            && unsafe {
                ((*prev).flags & BakedKeyFlags::STEP_RIGHT).any() && (*prev.sub(1)).time == time
            }
        {
            // SAFETY: `prev > keys` (checked), so `prev.sub(1)` stays in the run.
            prev = unsafe { prev.sub(1) };
        }
        // SAFETY: `prev` is a live keyframe.
        if unsafe { ((*prev).flags & BakedKeyFlags::STEP_LEFT).any() } {
            t = 0.0;
        }
        // SAFETY: `next` is a live keyframe.
        if unsafe { ((*next).flags & BakedKeyFlags::STEP_RIGHT).any() } {
            t = 1.0;
        }
        // SAFETY: `prev`/`next` are live keyframes of the run.
        return unsafe { quat_slerp((*prev).value, (*next).value, t as Real) };
    }

    // PORT DIVERGENCE (ufbx.c:31402): upstream's trailing
    // `keyframes.data[keyframes.count - 1]` reads `data[SIZE_MAX]` for an empty
    // list; return `IDENTITY_QUAT` for that case. Reconcile once upstream lands
    // the fix.
    if keyframes.count == 0 {
        return IDENTITY_QUAT;
    }
    // SAFETY: `count >= 1` (guarded above), so `count - 1` addresses the last
    // live keyframe of the run — the clamp value when no keyframe is past `time`.
    unsafe { (*keyframes.data.add(keyframes.count - 1)).value }
}

// ufbx.c:31405-31412 `ufbx_get_bone_pose`
// Kept here because `ufbxi_update_pose` (ufbx.c:23271,
// `native::scene_process`) calls it.
pub(crate) unsafe fn get_bone_pose(pose: *const Pose, node: *const Node) -> *mut BonePose {
    if pose.is_null() || node.is_null() {
        return core::ptr::null_mut();
    }
    let mut index: usize = usize::MAX;
    // SAFETY: `pose`/`node` are non-null (checked) and point at a live `Pose` and
    // `Node` — the raw-pointer contract of this `unsafe fn`; the search spans the
    // pose's own sorted `bone_poses` run `0..count`, every probe pointer the
    // comparators receive addresses a live `BonePose` whose `bone_node` ref is
    // readable, compared against the live `node`.
    unsafe {
        macro_lower_bound_eq::<BonePose>(
            8,
            &mut index,
            (*pose).bone_poses.data,
            0,
            (*pose).bone_poses.count,
            |a| (*ref_ptr(&raw const (*a).bone_node)).element.typed_id < (*node).element.typed_id,
            |a| std::ptr::eq(ref_ptr(&raw const (*a).bone_node), node),
        )
    };
    if index < usize::MAX {
        // SAFETY: `index < count` (a hit), so `bone_poses.data.add(index)`
        // addresses the `index`-th live `BonePose` of the pose's run.
        unsafe { (*pose).bone_poses.data.add(index) as *mut BonePose }
    } else {
        core::ptr::null_mut()
    }
}

// ufbx.c:31414-31423 `ufbx_find_prop_texture_len`
// `name: &[u8]` carries C's `(name, name_len)` pair (see `find_prop_len`).
pub(crate) unsafe fn find_prop_texture_len(material: *const Material, name: &[u8]) -> *mut Texture {
    if material.is_null() {
        return core::ptr::null_mut();
    }

    let mut index: usize = usize::MAX;
    // SAFETY: `material` is non-null here (checked above) and points at a live
    // `Material` per this fn's raw-pointer contract; `textures.data`/`.count`
    // are its own list fields, and each closure derefs a `MaterialTexture` the
    // search keeps within `[0, count)`.
    unsafe {
        macro_lower_bound_eq::<MaterialTexture>(
            4,
            &mut index,
            (*material).textures.data,
            0,
            (*material).textures.count,
            |a| str_less((*a).material_prop.as_bytes(), name),
            |a| str_equal((*a).material_prop.as_bytes(), name),
        );
    }
    if index < usize::MAX {
        // SAFETY: `index < count` here, so `textures.data.add(index)` addresses a
        // live `MaterialTexture`; the raw address identifies its own `texture` field.
        unsafe { ref_ptr(&raw const (*(*material).textures.data.add(index)).texture) }
    } else {
        core::ptr::null_mut()
    }
}

// Shared search core of the two `ufbx_find_shader_prop*` entries below
// (navigation/projection split: this is the one anchored search body; both
// public fns are thin projections over the returned positions). Locates the
// equal-range of `name` in the first binding list containing it, as the list
// view plus `[begin, end)` — `end > begin` whenever `Some` is returned.
#[allow(clippy::type_complexity)] // one-off internal (list view, begin, end) triple
fn find_shader_prop_binding_range<'a, M: Mode>(
    shader: Option<&'a View<Shader, M>>,
    name: &[u8],
) -> Option<(&'a View<List<ShaderPropBinding>, M>, usize, usize)> {
    let shader = shader?;

    // C: `ufbxi_for_ptr_list(ufbx_shader_binding, p_bind, shader->bindings)`
    let bind_list = shader.bindings_view();
    for bind_ix in 0..bind_list.count() {
        let bind = bind_list.at(bind_ix);
        let pb = bind.prop_bindings_view();

        if let Some(begin) = pb.lower_bound_eq(
            4,
            |a| str_less(a.shader_prop_view().bytes(), name),
            |a| str_equal(a.shader_prop_view().bytes(), name),
        ) {
            let end: usize =
                pb.upper_bound_eq(4, begin, |a| str_equal(a.shader_prop_view().bytes(), name));
            return Some((pb, begin, end));
        }
    }

    None
}

// ufbx.c:31425-31432 `ufbx_find_shader_prop_len`
pub(crate) fn find_shader_prop_len<M: Mode>(
    shader: Option<&View<Shader, M>>,
    name: &[u8],
) -> String {
    match find_shader_prop_binding_range(shader, name) {
        // The range is non-empty, so `begin` indexes the first matching
        // binding (in bounds of the `at` check).
        Some((pb, begin, _end)) => pb.at(begin).material_prop(),
        None => EMPTY_STRING.0,
    }
}

// ufbx.c:31434-31461 `ufbx_find_shader_prop_bindings_len`
// `name: &[u8]` carries C's `(name, name_len)` pair (see `find_prop_len`); C's
// null-or-live shader pointer arrives as `Option<&View<_, M>>`.
pub(crate) fn find_shader_prop_bindings_len<M: Mode>(
    shader: Option<&View<Shader, M>>,
    name: &[u8],
) -> List<ShaderPropBinding> {
    // C: `ufbx_shader_prop_binding_list bindings = { NULL, 0 };` — `List<T>`
    // carries a private `PhantomData` marker, so the C aggregate initializer
    // becomes a zeroed value with both public fields written (same shape as
    // `native::scene_process::find_dst_connections`).
    // SAFETY: an all-zero bit pattern is a valid `List<ShaderPropBinding>` (raw
    // data pointer plus `usize` count and a zero-size `PhantomData` marker).
    let mut bindings: List<ShaderPropBinding> = unsafe { MaybeUninit::zeroed().assume_init() };
    bindings.data = core::ptr::null();
    bindings.count = 0;

    if let Some((pb, begin, end)) = find_shader_prop_binding_range(shader, name) {
        // Whole-run provenance from the stored list pointer, as in
        // `find_anim_props` (a one-element `at` retag would make the caller's
        // multi-element slice UB). `begin < end <= count`.
        bindings.data = pb.data().wrapping_add(begin);
        bindings.count = end - begin;
    }

    bindings
}

// ufbx.c:31463-31476 `ufbx_find_shader_texture_input_len`
// `name: &[u8]` carries C's `(name, name_len)` pair (see `find_prop_len`).
pub(crate) fn find_shader_texture_input_len<'a, M: Mode>(
    shader: &'a View<ShaderTexture, M>,
    name: &[u8],
) -> Option<&'a View<ShaderTextureInput, M>> {
    let inputs = shader.inputs_view();
    if let Some(index) = inputs.lower_bound_eq(
        4,
        |a| str_less(a.name_view().bytes(), name),
        |a| str_equal(a.name_view().bytes(), name),
    ) {
        return Some(inputs.at(index));
    }

    None
}

// ufbx.c:31478-31490 `ufbx_coordinate_axes_valid`
// Kept here because `ufbxi_update_adjust_transforms`
// and `ufbxi_update_scene_settings_obj` (ufbx.c:23694/23937,
// `native::scene_process`) call it.
//
// C compares the `ufbx_coordinate_axis` enum members as `int`s; the generated
// enum is `#[repr(u32)]`, so the comparisons go through `as u32`. The
// `< UFBX_COORDINATE_AXIS_POSITIVE_X` (i.e. `< 0`) halves are dead for an
// unsigned repr but are kept verbatim — they are the C source text.
pub(crate) fn coordinate_axes_valid(axes: CoordinateAxes) -> bool {
    if (axes.right as u32) < CoordinateAxis::PositiveX as u32
        || axes.right as u32 > CoordinateAxis::NegativeZ as u32
    {
        return false;
    }
    if (axes.up as u32) < CoordinateAxis::PositiveX as u32
        || axes.up as u32 > CoordinateAxis::NegativeZ as u32
    {
        return false;
    }
    if (axes.front as u32) < CoordinateAxis::PositiveX as u32
        || axes.front as u32 > CoordinateAxis::NegativeZ as u32
    {
        return false;
    }

    // Check that all the positive/negative axes are used
    let mut mask: u32 = 0;
    mask |= 1u32 << ((axes.right as u32) >> 1);
    mask |= 1u32 << ((axes.up as u32) >> 1);
    mask |= 1u32 << ((axes.front as u32) >> 1);
    (mask & 0x7u32) == 0x7u32
}

// ufbx.c:31492-31495 `ufbx_quat_mul`
// C has no `ufbxi_noinline` on this one (unlike `ufbx_quat_dot` below).
pub(crate) fn quat_mul(a: Quat, b: Quat) -> Quat {
    mul_quat(a, b)
}

// ufbx.c:31497-31500 `ufbx_vec3_normalize`
pub(crate) fn vec3_normalize(v: Vec3) -> Vec3 {
    normalize3(v)
}

// ufbx.c:31502-31505 `ufbx_quat_dot`
#[inline(never)]
pub(crate) fn quat_dot(a: Quat, b: Quat) -> Real {
    a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w
}

// ufbx.c:31507-31517 `ufbx_quat_normalize`
#[inline(never)]
pub(crate) fn quat_normalize(mut q: Quat) -> Quat {
    let mut norm: Real = quat_dot(q, q);
    if norm == 0.0 {
        return IDENTITY_QUAT;
    }
    // C: `norm = (ufbx_real)ufbx_sqrt(norm);` — `norm` promotes to double at
    // the call, the result narrows back to `ufbx_real`.
    norm = math::sqrt(norm as f64) as Real;
    q.x /= norm;
    q.y /= norm;
    q.z /= norm;
    q.w /= norm;
    q
}

// ufbx.c:31519-31525 `ufbx_quat_fix_antipodal`
#[inline(never)]
pub(crate) fn quat_fix_antipodal(mut q: Quat, reference: Quat) -> Quat {
    if quat_dot(q, reference) < 0.0 {
        q.x = -q.x;
        q.y = -q.y;
        q.z = -q.z;
        q.w = -q.w;
    }
    q
}

// ufbx.c:31527-31552 `ufbx_quat_slerp`
#[inline(never)]
pub(crate) fn quat_slerp(a: Quat, mut b: Quat, t: Real) -> Quat {
    // C: `double dot = a.x*b.x + ...;` — every operand is `ufbx_real`, so the
    // whole sum computes in `ufbx_real` and only the result widens to double.
    let mut dot: f64 = as_f64!(a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w);
    if dot < 0.0 {
        dot = -dot;
        b.x = -b.x;
        b.y = -b.y;
        b.z = -b.z;
        b.w = -b.w;
    }
    let omega: f64 = math::acos(math::fmin(math::fmax(dot, 0.0), 1.0));
    // C: `if (omega <= 1.175494351e-38f)` — a FLOAT literal widened to double,
    // i.e. FLT_MIN, not the decimal digits as a double. Keep the `f32` cast.
    if omega <= 1.175494351e-38f32 as f64 {
        return a;
    }
    let rcp_so: f64 = 1.0 / math::sin(omega);
    // C: `(1.0 - t) * omega` — `1.0` is double, so `t` (ufbx_real) promotes.
    let af: f64 = math::sin((1.0 - as_f64!(t)) * omega) * rcp_so;
    let bf: f64 = math::sin(as_f64!(t) * omega) * rcp_so;

    // C: `af*a.x + bf*b.x` — `af`/`bf` are double, so the components promote.
    let x: f64 = af * as_f64!(a.x) + bf * as_f64!(b.x);
    let y: f64 = af * as_f64!(a.y) + bf * as_f64!(b.y);
    let z: f64 = af * as_f64!(a.z) + bf * as_f64!(b.z);
    let w: f64 = af * as_f64!(a.w) + bf * as_f64!(b.w);
    let rcp_len: f64 = 1.0 / math::sqrt(x * x + y * y + z * z + w * w);

    // C: `ufbx_quat ret;` — all four fields are written below.
    // SAFETY: `Quat` is POD (four `Real`s); the all-zero bit pattern is valid
    // and every field is overwritten before `ret` is read.
    let mut ret: Quat = unsafe { core::mem::zeroed() };
    ret.x = (x * rcp_len) as Real;
    ret.y = (y * rcp_len) as Real;
    ret.z = (z * rcp_len) as Real;
    ret.w = (w * rcp_len) as Real;
    ret
}

// ufbx.c:31554-31564 `ufbx_quat_rotate_vec3`
// Kept here because `ufbxi_mul_rotate` and friends
// (ufbx.c:22695+, `native::scene_process`) call it.
#[inline(never)]
pub(crate) fn quat_rotate_vec3(q: Quat, v: Vec3) -> Vec3 {
    let xy: Real = q.x * v.y - q.y * v.x;
    let xz: Real = q.x * v.z - q.z * v.x;
    let yz: Real = q.y * v.z - q.z * v.y;
    // C: `ufbx_vec3 r;` — every field is written below before the return.
    // SAFETY: `Vec3` is POD (three `Real`s); the all-zero bit pattern is valid
    // and every field is overwritten before `r` is read.
    let mut r: Vec3 = unsafe { core::mem::zeroed() };
    r.x = 2.0 * (q.w * yz + q.y * xy + q.z * xz) + v.x;
    r.y = 2.0 * (-(q.x * xy) - q.w * xz + q.z * yz) + v.y;
    r.z = 2.0 * (-(q.x * xz) - q.y * yz + q.w * xy) + v.z;
    r
}

// ufbx.c:31566-31620 `ufbx_euler_to_quat`
// Kept here because `ufbxi_mul_rotate` /
// `ufbxi_mul_inv_rotate` (ufbx.c:22695/22726, `native::scene_process`) call it.
#[inline(never)]
pub(crate) fn euler_to_quat(v: Vec3, order: RotationOrder) -> Quat {
    // C: `double vx = v.x * (UFBXI_DEG_TO_RAD_DOUBLE * 0.5);` — the constant is
    // double, so `v.x` (ufbx_real) promotes.
    let vx: f64 = as_f64!(v.x) * (DEG_TO_RAD_DOUBLE * 0.5);
    let vy: f64 = as_f64!(v.y) * (DEG_TO_RAD_DOUBLE * 0.5);
    let vz: f64 = as_f64!(v.z) * (DEG_TO_RAD_DOUBLE * 0.5);
    let cx: f64 = math::cos(vx);
    let sx: f64 = math::sin(vx);
    let cy: f64 = math::cos(vy);
    let sy: f64 = math::sin(vy);
    let cz: f64 = math::cos(vz);
    let sz: f64 = math::sin(vz);
    // C: `ufbx_quat q;` — every arm below writes all four fields.
    // SAFETY: `Quat` is POD (four `Real`s); the all-zero bit pattern is valid
    // and every arm overwrites all four fields before `q` is read.
    let mut q: Quat = unsafe { core::mem::zeroed() };

    // Generated by `misc/gen_rotation_order.py`
    match order {
        RotationOrder::Xyz => {
            q.x = (-(cx * sy * sz) + cy * cz * sx) as Real;
            q.y = (cx * cz * sy + cy * sx * sz) as Real;
            q.z = (cx * cy * sz - cz * sx * sy) as Real;
            q.w = (cx * cy * cz + sx * sy * sz) as Real;
        }
        RotationOrder::Xzy => {
            q.x = (cx * sy * sz + cy * cz * sx) as Real;
            q.y = (cx * cz * sy + cy * sx * sz) as Real;
            q.z = (cx * cy * sz - cz * sx * sy) as Real;
            q.w = (cx * cy * cz - sx * sy * sz) as Real;
        }
        RotationOrder::Yzx => {
            q.x = (-(cx * sy * sz) + cy * cz * sx) as Real;
            q.y = (cx * cz * sy - cy * sx * sz) as Real;
            q.z = (cx * cy * sz + cz * sx * sy) as Real;
            q.w = (cx * cy * cz + sx * sy * sz) as Real;
        }
        RotationOrder::Yxz => {
            q.x = (-(cx * sy * sz) + cy * cz * sx) as Real;
            q.y = (cx * cz * sy + cy * sx * sz) as Real;
            q.z = (cx * cy * sz + cz * sx * sy) as Real;
            q.w = (cx * cy * cz - sx * sy * sz) as Real;
        }
        RotationOrder::Zxy => {
            q.x = (cx * sy * sz + cy * cz * sx) as Real;
            q.y = (cx * cz * sy - cy * sx * sz) as Real;
            q.z = (cx * cy * sz - cz * sx * sy) as Real;
            q.w = (cx * cy * cz + sx * sy * sz) as Real;
        }
        RotationOrder::Zyx => {
            q.x = (cx * sy * sz + cy * cz * sx) as Real;
            q.y = (cx * cz * sy - cy * sx * sz) as Real;
            q.z = (cx * cy * sz + cz * sx * sy) as Real;
            q.w = (cx * cy * cz - sx * sy * sz) as Real;
        }
        _ => {
            // C: `q.x = q.y = q.z = 0.0f; q.w = 1.0f;`
            q.z = 0.0;
            q.y = q.z;
            q.x = q.y;
            q.w = 1.0;
        }
    }

    q
}

// ufbx.c:31622-31721 `ufbx_quat_to_euler`
#[inline(never)]
pub(crate) fn quat_to_euler(q: Quat, order: RotationOrder) -> Vec3 {
    // TODO: Derive these rigorously
    // C: `#if defined(UFBX_REAL_IS_FLOAT) const double eps = 0.9999999;
    //     #else const double eps = 0.999999999; #endif`
    #[cfg(feature = "real-is-f32")]
    let eps: f64 = 0.9999999;
    #[cfg(not(feature = "real-is-f32"))]
    let eps: f64 = 0.999999999;

    // C: `double vx, vy, vz;` / `double t;` — `t` is deliberately left unset
    // by the `default:` arm (it is never read there).
    let mut vx: f64;
    let mut vy: f64;
    let mut vz: f64;
    let t: f64;

    // C: `double qx = q.x, ...;` — `ufbx_real` promoted to double.
    let qx: f64 = as_f64!(q.x);
    let qy: f64 = as_f64!(q.y);
    let qz: f64 = as_f64!(q.z);
    let qw: f64 = as_f64!(q.w);

    // Generated by `misc/gen_quat_to_euler.py`
    match order {
        RotationOrder::Xyz => {
            t = 2.0 * (qw * qy - qx * qz);
            if math::fabs(t) < eps {
                vy = math::asin(t);
                vz = math::atan2(2.0 * (qw * qz + qx * qy), 2.0 * (qw * qw + qx * qx) - 1.0);
                vx = -math::atan2(-2.0 * (qw * qx + qy * qz), 2.0 * (qw * qw + qz * qz) - 1.0);
            } else {
                vy = math::copysign(DPI * 0.5, t);
                vz = math::atan2(
                    -2.0 * t * (qw * qx - qy * qz),
                    t * (2.0 * qw * qy + 2.0 * qx * qz),
                );
                vx = 0.0;
            }
        }
        RotationOrder::Xzy => {
            t = 2.0 * (qw * qz + qx * qy);
            if math::fabs(t) < eps {
                vz = math::asin(t);
                vy = math::atan2(2.0 * (qw * qy - qx * qz), 2.0 * (qw * qw + qx * qx) - 1.0);
                vx = -math::atan2(-2.0 * (qw * qx - qy * qz), 2.0 * (qw * qw + qy * qy) - 1.0);
            } else {
                vz = math::copysign(DPI * 0.5, t);
                vy = math::atan2(
                    2.0 * t * (qw * qx + qy * qz),
                    -t * (2.0 * qx * qy - 2.0 * qw * qz),
                );
                vx = 0.0;
            }
        }
        RotationOrder::Yzx => {
            t = 2.0 * (qw * qz - qx * qy);
            if math::fabs(t) < eps {
                vz = math::asin(t);
                vx = math::atan2(2.0 * (qw * qx + qy * qz), 2.0 * (qw * qw + qy * qy) - 1.0);
                vy = -math::atan2(-2.0 * (qw * qy + qx * qz), 2.0 * (qw * qw + qx * qx) - 1.0);
            } else {
                vz = math::copysign(DPI * 0.5, t);
                vx = math::atan2(
                    -2.0 * t * (qw * qy - qx * qz),
                    t * (2.0 * qw * qz + 2.0 * qx * qy),
                );
                vy = 0.0;
            }
        }
        RotationOrder::Yxz => {
            t = 2.0 * (qw * qx + qy * qz);
            if math::fabs(t) < eps {
                vx = math::asin(t);
                vz = math::atan2(2.0 * (qw * qz - qx * qy), 2.0 * (qw * qw + qy * qy) - 1.0);
                vy = -math::atan2(-2.0 * (qw * qy - qx * qz), 2.0 * (qw * qw + qz * qz) - 1.0);
            } else {
                vx = math::copysign(DPI * 0.5, t);
                vz = math::atan2(
                    2.0 * t * (qw * qy + qx * qz),
                    -t * (2.0 * qy * qz - 2.0 * qw * qx),
                );
                vy = 0.0;
            }
        }
        RotationOrder::Zxy => {
            t = 2.0 * (qw * qx - qy * qz);
            if math::fabs(t) < eps {
                vx = math::asin(t);
                vy = math::atan2(2.0 * (qw * qy + qx * qz), 2.0 * (qw * qw + qz * qz) - 1.0);
                vz = -math::atan2(-2.0 * (qw * qz + qx * qy), 2.0 * (qw * qw + qy * qy) - 1.0);
            } else {
                vx = math::copysign(DPI * 0.5, t);
                vy = math::atan2(
                    -2.0 * t * (qw * qz - qx * qy),
                    t * (2.0 * qw * qx + 2.0 * qy * qz),
                );
                vz = 0.0;
            }
        }
        RotationOrder::Zyx => {
            t = 2.0 * (qw * qy + qx * qz);
            if math::fabs(t) < eps {
                vy = math::asin(t);
                vx = math::atan2(2.0 * (qw * qx - qy * qz), 2.0 * (qw * qw + qz * qz) - 1.0);
                vz = -math::atan2(-2.0 * (qw * qz - qx * qy), 2.0 * (qw * qw + qx * qx) - 1.0);
            } else {
                vy = math::copysign(DPI * 0.5, t);
                vx = math::atan2(
                    2.0 * t * (qw * qz + qx * qy),
                    -t * (2.0 * qx * qz - 2.0 * qw * qy),
                );
                vz = 0.0;
            }
        }
        _ => {
            // C: `vx = vy = vz = 0.0;` (and `t` stays uninitialized)
            vz = 0.0;
            vy = vz;
            vx = vy;
        }
    }

    vx *= RAD_TO_DEG_DOUBLE;
    vy *= RAD_TO_DEG_DOUBLE;
    vz *= RAD_TO_DEG_DOUBLE;

    // C: `ufbx_vec3 v = { (ufbx_real)vx, (ufbx_real)vy, (ufbx_real)vz };`
    let v: Vec3 = Vec3 {
        x: vx as Real,
        y: vy as Real,
        z: vz as Real,
    };
    v
}

// ufbx.c:31723-31747 `ufbx_matrix_mul`
// Kept here because `ufbxi_update_node`
// (ufbx.c:22955, `native::scene_process`) calls it.
#[inline(never)]
pub(crate) unsafe fn matrix_mul(a: *const Matrix, b: *const Matrix) -> Matrix {
    // C: `ufbx_assert(a && b);`
    ufbx_assert!(!a.is_null() && !b.is_null());
    if a.is_null() || b.is_null() {
        return IDENTITY_MATRIX;
    }

    // C: `ufbx_matrix dst;` — every field is written below before the return,
    // so the zero-fill is inert (upstream carries no `// ufbxi_uninit` marker).
    // SAFETY: an all-zero bit pattern is a valid `Matrix` (all `Real` fields).
    let mut dst: Matrix = unsafe { core::mem::zeroed() };

    // SAFETY: `a` and `b` are non-null here (checked above) and point at live
    // `Matrix` values per this fn's contract; every field read below is one of
    // their own `mNN` fields.
    unsafe {
        dst.m03 = (*a).m00 * (*b).m03 + (*a).m01 * (*b).m13 + (*a).m02 * (*b).m23 + (*a).m03;
        dst.m13 = (*a).m10 * (*b).m03 + (*a).m11 * (*b).m13 + (*a).m12 * (*b).m23 + (*a).m13;
        dst.m23 = (*a).m20 * (*b).m03 + (*a).m21 * (*b).m13 + (*a).m22 * (*b).m23 + (*a).m23;

        dst.m00 = (*a).m00 * (*b).m00 + (*a).m01 * (*b).m10 + (*a).m02 * (*b).m20;
        dst.m10 = (*a).m10 * (*b).m00 + (*a).m11 * (*b).m10 + (*a).m12 * (*b).m20;
        dst.m20 = (*a).m20 * (*b).m00 + (*a).m21 * (*b).m10 + (*a).m22 * (*b).m20;

        dst.m01 = (*a).m00 * (*b).m01 + (*a).m01 * (*b).m11 + (*a).m02 * (*b).m21;
        dst.m11 = (*a).m10 * (*b).m01 + (*a).m11 * (*b).m11 + (*a).m12 * (*b).m21;
        dst.m21 = (*a).m20 * (*b).m01 + (*a).m21 * (*b).m11 + (*a).m22 * (*b).m21;

        dst.m02 = (*a).m00 * (*b).m02 + (*a).m01 * (*b).m12 + (*a).m02 * (*b).m22;
        dst.m12 = (*a).m10 * (*b).m02 + (*a).m11 * (*b).m12 + (*a).m12 * (*b).m22;
        dst.m22 = (*a).m20 * (*b).m02 + (*a).m21 * (*b).m12 + (*a).m22 * (*b).m22;
    }

    dst
}

// ufbx.c:31749-31754 `ufbx_matrix_determinant`
// Kept here because `ufbx_matrix_for_normals` below
// needs it.
pub(crate) unsafe fn matrix_determinant(m: *const Matrix) -> Real {
    // SAFETY: `m` points at a live `Matrix` per this fn's contract; every field
    // read is one of its own `mNN` fields.
    unsafe {
        -(*m).m02 * (*m).m11 * (*m).m20
            + (*m).m01 * (*m).m12 * (*m).m20
            + (*m).m02 * (*m).m10 * (*m).m21
            - (*m).m00 * (*m).m12 * (*m).m21
            - (*m).m01 * (*m).m10 * (*m).m22
            + (*m).m00 * (*m).m11 * (*m).m22
    }
}

// ufbx.c:31756-31782 `ufbx_matrix_invert`
// Kept here because `ufbxi_update_pose`
// (ufbx.c:23271, `native::scene_process`) calls it.
pub(crate) unsafe fn matrix_invert(m: *const Matrix) -> Matrix {
    // SAFETY: `m` points at a live `Matrix` per this fn's contract, forwarded
    // unchanged to `matrix_determinant`.
    let det: Real = unsafe { matrix_determinant(m) };

    // C: `ufbx_matrix r;` — the early-out arm `memset`s it and the fall-through
    // arm writes every field, so the zero-fill is inert (upstream carries no
    // `// ufbxi_uninit` marker).
    // SAFETY: an all-zero bit pattern is a valid `Matrix` (all `Real` fields).
    let mut r: Matrix = unsafe { core::mem::zeroed() };
    // C: `ufbx_fabs(det) <= UFBX_EPSILON` — `det` promotes to double at the
    // call and `UFBX_EPSILON` (ufbx_real) promotes for the comparison.
    if math::fabs(det as f64) <= as_f64!(math::EPSILON) {
        // C: `memset(&r, 0, sizeof(r));`
        // SAFETY: as above, an all-zero bit pattern is a valid `Matrix`.
        r = unsafe { core::mem::zeroed() };
        return r;
    }

    let rcp_det: Real = 1.0 / det;

    // SAFETY: `m` points at a live `Matrix` per this fn's contract; every field
    // read below is one of its own `mNN` fields.
    unsafe {
        r.m00 = (-(*m).m12 * (*m).m21 + (*m).m11 * (*m).m22) * rcp_det;
        r.m10 = ((*m).m12 * (*m).m20 - (*m).m10 * (*m).m22) * rcp_det;
        r.m20 = (-(*m).m11 * (*m).m20 + (*m).m10 * (*m).m21) * rcp_det;
        r.m01 = ((*m).m02 * (*m).m21 - (*m).m01 * (*m).m22) * rcp_det;
        r.m11 = (-(*m).m02 * (*m).m20 + (*m).m00 * (*m).m22) * rcp_det;
        r.m21 = ((*m).m01 * (*m).m20 - (*m).m00 * (*m).m21) * rcp_det;
        r.m02 = (-(*m).m02 * (*m).m11 + (*m).m01 * (*m).m12) * rcp_det;
        r.m12 = ((*m).m02 * (*m).m10 - (*m).m00 * (*m).m12) * rcp_det;
        r.m22 = (-(*m).m01 * (*m).m10 + (*m).m00 * (*m).m11) * rcp_det;
        r.m03 = ((*m).m03 * (*m).m12 * (*m).m21
            - (*m).m02 * (*m).m13 * (*m).m21
            - (*m).m03 * (*m).m11 * (*m).m22
            + (*m).m01 * (*m).m13 * (*m).m22
            + (*m).m02 * (*m).m11 * (*m).m23
            - (*m).m01 * (*m).m12 * (*m).m23)
            * rcp_det;
        r.m13 = ((*m).m02 * (*m).m13 * (*m).m20 - (*m).m03 * (*m).m12 * (*m).m20
            + (*m).m03 * (*m).m10 * (*m).m22
            - (*m).m00 * (*m).m13 * (*m).m22
            - (*m).m02 * (*m).m10 * (*m).m23
            + (*m).m00 * (*m).m12 * (*m).m23)
            * rcp_det;
        r.m23 = ((*m).m03 * (*m).m11 * (*m).m20
            - (*m).m01 * (*m).m13 * (*m).m20
            - (*m).m03 * (*m).m10 * (*m).m21
            + (*m).m00 * (*m).m13 * (*m).m21
            + (*m).m01 * (*m).m10 * (*m).m23
            - (*m).m00 * (*m).m11 * (*m).m23)
            * rcp_det;
    }

    r
}

// ufbx.c:31784-31802 `ufbx_matrix_for_normals`
// Kept here because `ufbxi_modify_geometry`
// (ufbx.c:21165, `native::scene_process`) calls it.
#[inline(never)]
pub(crate) unsafe fn matrix_for_normals(m: *const Matrix) -> Matrix {
    // SAFETY: `m` points at a live `Matrix` per this fn's contract, forwarded
    // unchanged to `matrix_determinant`.
    let det: Real = unsafe { matrix_determinant(m) };
    let det_sign: Real = if det >= 0.0 { 1.0 } else { -1.0 };

    // C: `ufbx_matrix r;` — every field is written below before the return.
    // SAFETY: an all-zero bit pattern is a valid `Matrix` (all `Real` fields).
    let mut r: Matrix = unsafe { core::mem::zeroed() };
    // SAFETY: `m` points at a live `Matrix` per this fn's contract; every field
    // read below is one of its own `mNN` fields.
    unsafe {
        r.m00 = (-(*m).m12 * (*m).m21 + (*m).m11 * (*m).m22) * det_sign;
        r.m01 = ((*m).m12 * (*m).m20 - (*m).m10 * (*m).m22) * det_sign;
        r.m02 = (-(*m).m11 * (*m).m20 + (*m).m10 * (*m).m21) * det_sign;
        r.m10 = ((*m).m02 * (*m).m21 - (*m).m01 * (*m).m22) * det_sign;
        r.m11 = (-(*m).m02 * (*m).m20 + (*m).m00 * (*m).m22) * det_sign;
        r.m12 = ((*m).m01 * (*m).m20 - (*m).m00 * (*m).m21) * det_sign;
        r.m20 = (-(*m).m02 * (*m).m11 + (*m).m01 * (*m).m12) * det_sign;
        r.m21 = ((*m).m02 * (*m).m10 - (*m).m00 * (*m).m12) * det_sign;
        r.m22 = (-(*m).m01 * (*m).m10 + (*m).m00 * (*m).m11) * det_sign;
    }
    // C: `r.m03 = r.m13 = r.m23 = 0.0f;`
    r.m23 = 0.0;
    r.m13 = r.m23;
    r.m03 = r.m13;

    r
}

// ufbx.c:31804-31814 `ufbx_transform_position`
// Kept here because `ufbxi_transform_vec3_list`
// (ufbx.c:21049, `native::scene_process`) calls it.
#[inline(never)]
pub(crate) unsafe fn transform_position(m: *const Matrix, v: Vec3) -> Vec3 {
    ufbx_assert!(!m.is_null());
    if m.is_null() {
        return ZERO_VEC3;
    }

    // C: `ufbx_vec3 r;` — every field is written below before the return,
    // so the zero-fill is inert (upstream carries no `// ufbxi_uninit` marker).
    // SAFETY: an all-zero bit pattern is a valid `Vec3` (all `Real` fields).
    let mut r: Vec3 = unsafe { core::mem::zeroed() };
    // SAFETY: `m` is non-null here (checked above) and points at a live `Matrix`
    // per this fn's contract; every field read is one of its own `mNN` fields.
    unsafe {
        r.x = (*m).m00 * v.x + (*m).m01 * v.y + (*m).m02 * v.z + (*m).m03;
        r.y = (*m).m10 * v.x + (*m).m11 * v.y + (*m).m12 * v.z + (*m).m13;
        r.z = (*m).m20 * v.x + (*m).m21 * v.y + (*m).m22 * v.z + (*m).m23;
    }
    r
}

// ufbx.c:31816-31826 `ufbx_transform_direction`
// Kept here because `ufbxi_update_adjust_transforms`
// (ufbx.c:23705, `native::scene_process`) calls it.
#[inline(never)]
pub(crate) unsafe fn transform_direction(m: *const Matrix, v: Vec3) -> Vec3 {
    ufbx_assert!(!m.is_null());
    if m.is_null() {
        return ZERO_VEC3;
    }

    // C: `ufbx_vec3 r;` — every field is written below before the return,
    // so the zero-fill is inert (upstream carries no `// ufbxi_uninit` marker).
    // SAFETY: an all-zero bit pattern is a valid `Vec3` (all `Real` fields).
    let mut r: Vec3 = unsafe { core::mem::zeroed() };
    // SAFETY: `m` is non-null here (checked above) and points at a live `Matrix`
    // per this fn's contract; every field read is one of its own `mNN` fields.
    unsafe {
        r.x = (*m).m00 * v.x + (*m).m01 * v.y + (*m).m02 * v.z;
        r.y = (*m).m10 * v.x + (*m).m11 * v.y + (*m).m12 * v.z;
        r.z = (*m).m20 * v.x + (*m).m21 * v.y + (*m).m22 * v.z;
    }
    r
}

// ufbx.c:31828-31852 `ufbx_transform_to_matrix`
#[inline(never)]
pub(crate) unsafe fn transform_to_matrix(t: *const Transform) -> Matrix {
    ufbx_assert!(!t.is_null());
    if t.is_null() {
        return IDENTITY_MATRIX;
    }

    // SAFETY: `t` is non-null here (checked above) and points at a live
    // `Transform` per this fn's contract; reading its own `rotation`/`scale`
    // fields.
    let (q, sx, sy, sz) = unsafe {
        (
            (*t).rotation,
            2.0 * (*t).scale.x,
            2.0 * (*t).scale.y,
            2.0 * (*t).scale.z,
        )
    };
    let xx: Real = q.x * q.x;
    let xy: Real = q.x * q.y;
    let xz: Real = q.x * q.z;
    let xw: Real = q.x * q.w;
    let yy: Real = q.y * q.y;
    let yz: Real = q.y * q.z;
    let yw: Real = q.y * q.w;
    let zz: Real = q.z * q.z;
    let zw: Real = q.z * q.w;
    // C: `ufbx_matrix m;` — every field is written below before the return,
    // so the zero-fill is inert (upstream carries no `// ufbxi_uninit` marker).
    // SAFETY: an all-zero bit pattern is a valid `Matrix` (all `Real` fields).
    let mut m: Matrix = unsafe { core::mem::zeroed() };
    m.m00 = sx * (-yy - zz + 0.5);
    m.m10 = sx * (xy + zw);
    m.m20 = sx * (-yw + xz);
    m.m01 = sy * (-zw + xy);
    m.m11 = sy * (-xx - zz + 0.5);
    m.m21 = sy * (xw + yz);
    m.m02 = sz * (xz + yw);
    m.m12 = sz * (-xw + yz);
    m.m22 = sz * (-xx - yy + 0.5);
    // SAFETY: `t` points at a live `Transform` per this fn's contract; reading
    // its own `translation` field.
    unsafe {
        m.m03 = (*t).translation.x;
        m.m13 = (*t).translation.y;
        m.m23 = (*t).translation.z;
    }
    m
}

// ufbx.c:31854-31926 `ufbx_matrix_to_transform`
// Kept here because `ufbxi_update_skin_cluster`
// (ufbx.c:23289, `native::scene_process`) calls it.
#[inline(never)]
pub(crate) unsafe fn matrix_to_transform(m: *const Matrix) -> Transform {
    ufbx_assert!(!m.is_null());
    if m.is_null() {
        return IDENTITY_TRANSFORM;
    }

    // SAFETY: `m` is non-null here (checked above) and points at a live `Matrix`
    // per this fn's contract, forwarded unchanged to `matrix_determinant`.
    let det: Real = unsafe { matrix_determinant(m) };

    // C indexes the `ufbx_matrix` value union's `ufbx_vec3 cols[4]` view; the
    // generated struct keeps only the `m00`..`m23` scalars, so the index is
    // pointer arithmetic from the struct base (same device as
    // `native::scene_process::add_weighted_matrix`).
    let m_cols: *const Vec3 = m as *const Vec3;

    // C: `ufbx_transform t;` — every member is written below before the return,
    // so the zero-fill is inert (upstream carries no `// ufbxi_uninit` marker).
    // SAFETY: an all-zero bit pattern is a valid `Transform` (all `Real`
    // sub-fields).
    let mut t: Transform = unsafe { core::mem::zeroed() };
    // SAFETY: the live `Matrix` behind `m` holds four contiguous `Vec3` columns
    // (its `m00`..`m23` scalars), so `m_cols.add(0..=3)` each address a column.
    unsafe {
        t.translation = *m_cols.add(3);
        t.scale.x = length3(*m_cols.add(0));
        t.scale.y = length3(*m_cols.add(1));
        t.scale.z = length3(*m_cols.add(2));
    }

    // Flip a single non-zero axis if negative determinant
    let mut sign_x: Real = 1.0;
    let mut sign_y: Real = 1.0;
    let mut sign_z: Real = 1.0;
    if det < 0.0 {
        if t.scale.x > 0.0 {
            sign_x = -1.0;
        } else if t.scale.y > 0.0 {
            sign_y = -1.0;
        } else if t.scale.z > 0.0 {
            sign_z = -1.0;
        }
    }

    // The three column reads stay one-per-`let`: a tuple destructure would make
    // `x`/`y`/`z` non-`LetStmt` bindings, which defeats clippy's type-alias
    // escape hatch for the `Real`-to-`f64` casts on their fields below.
    // SAFETY: the live `Matrix` behind `m` holds four contiguous `Vec3` columns,
    // so `m_cols.add(0)` addresses its first column.
    let x: Vec3 = mul3(
        unsafe { *m_cols.add(0) },
        if t.scale.x > 0.0 {
            sign_x / t.scale.x
        } else {
            0.0
        },
    );
    // SAFETY: as above, `m_cols.add(1)` addresses the second column.
    let y: Vec3 = mul3(
        unsafe { *m_cols.add(1) },
        if t.scale.y > 0.0 {
            sign_y / t.scale.y
        } else {
            0.0
        },
    );
    // SAFETY: as above, `m_cols.add(2)` addresses the third column.
    let z: Vec3 = mul3(
        unsafe { *m_cols.add(2) },
        if t.scale.z > 0.0 {
            sign_z / t.scale.z
        } else {
            0.0
        },
    );
    let trace: Real = x.x + y.y + z.z;
    if trace > 0.0 {
        // C: `ufbx_fmax(0.0, trace + 1.0)` — `1.0` is double, so `trace`
        // promotes; the double sqrt result narrows back to `ufbx_real`.
        let a: Real = math::sqrt(math::fmax(0.0, trace as f64 + 1.0)) as Real;
        let b: Real = if a != 0.0 { 0.5 / a } else { 0.0 };
        t.rotation.x = (y.z - z.y) * b;
        t.rotation.y = (z.x - x.z) * b;
        t.rotation.z = (x.y - y.x) * b;
        t.rotation.w = 0.5 * a;
    } else if x.x > y.y && x.x > z.z {
        // C: `1.0 + x.x - y.y - z.z` — `1.0` is double, so the chain computes
        // in double; the sqrt result narrows back to `ufbx_real`.
        let a: Real =
            math::sqrt(math::fmax(0.0, 1.0 + x.x as f64 - y.y as f64 - z.z as f64)) as Real;
        let b: Real = if a != 0.0 { 0.5 / a } else { 0.0 };
        t.rotation.x = 0.5 * a;
        t.rotation.y = (y.x + x.y) * b;
        t.rotation.z = (z.x + x.z) * b;
        t.rotation.w = (y.z - z.y) * b;
    } else if y.y > z.z {
        // C: double chain as above.
        let a: Real =
            math::sqrt(math::fmax(0.0, 1.0 - x.x as f64 + y.y as f64 - z.z as f64)) as Real;
        let b: Real = if a != 0.0 { 0.5 / a } else { 0.0 };
        t.rotation.x = (y.x + x.y) * b;
        t.rotation.y = 0.5 * a;
        t.rotation.z = (z.y + y.z) * b;
        t.rotation.w = (z.x - x.z) * b;
    } else {
        // C: double chain as above.
        let a: Real =
            math::sqrt(math::fmax(0.0, 1.0 - x.x as f64 - y.y as f64 + z.z as f64)) as Real;
        let b: Real = if a != 0.0 { 0.5 / a } else { 0.0 };
        t.rotation.x = (z.x + x.z) * b;
        t.rotation.y = (z.y + y.z) * b;
        t.rotation.z = 0.5 * a;
        t.rotation.w = (x.y - y.x) * b;
    }

    let len: Real = t.rotation.x * t.rotation.x
        + t.rotation.y * t.rotation.y
        + t.rotation.z * t.rotation.z
        + t.rotation.w * t.rotation.w;
    // C: `ufbx_fabs(len - 1.0f)` — `len - 1.0f` computes in `ufbx_real`, then
    // promotes to double at the call; `UFBX_EPSILON` promotes to compare.
    if math::fabs((len - 1.0) as f64) > as_f64!(math::EPSILON) {
        if math::fabs(len as f64) <= as_f64!(math::EPSILON) {
            t.rotation = IDENTITY_QUAT;
        } else {
            t.rotation.x /= len;
            t.rotation.y /= len;
            t.rotation.z /= len;
            t.rotation.w /= len;
        }
    }

    t.scale.x *= sign_x;
    t.scale.y *= sign_y;
    t.scale.z *= sign_z;

    t
}

// ufbx.c:31928-32018 `ufbx_catch_get_skin_vertex_matrix`
#[inline(never)]
pub(crate) unsafe fn catch_get_skin_vertex_matrix<M: Mode>(
    mut panic: Option<&mut Panic>,
    skin: &View<SkinDeformer, M>,
    vertex: usize,
    fallback: *const Matrix,
) -> Matrix {
    ufbx_assert!(!skin.as_ptr().is_null());
    // C-parity: the panic guard dereferences `skin` BEFORE the `!skin` test on
    // the next line — keep the order (a null `skin` is already an assert
    // violation above). The `vertices` list is a by-value read through the
    // view's accessor, so no raw dereference is spelled out here.
    if ufbxi_panicf!(
        panic,
        vertex < skin.vertices().count,
        "vertex (%zu) out of bounds (%zu)",
        vertex,
        skin.vertices().count,
    ) {
        return IDENTITY_MATRIX;
    }

    if skin.as_ptr().is_null() || vertex >= skin.vertices().count {
        return IDENTITY_MATRIX;
    }
    // SAFETY: `vertex < vertices.count` here, so `vertices.data.add(vertex)`
    // addresses a live `SkinVertex` in the deformer's vertex list.
    let skin_vertex: SkinVertex = unsafe { *skin.vertices().data.add(vertex) };

    // C: `ufbx_matrix mat = { 0.0f };` / `ufbx_quat q0 = { 0.0f }, qe = { 0.0f };`
    // / `ufbx_quat first_q0 = { 0.0f };` — partial initializers zero the rest.
    // The four zero-fills stay one-per-`let`: a tuple destructure would make
    // `q0` a non-`LetStmt` binding, which defeats clippy's type-alias escape
    // hatch for the `Real`-to-`f64` cast on its fields below.
    // SAFETY (this group): an all-zero bit pattern is a valid `Matrix`/`Quat` (all `Real`).
    let mut mat: Matrix = unsafe { core::mem::zeroed() };
    let mut q0: Quat = unsafe { core::mem::zeroed() };
    let mut qe: Quat = unsafe { core::mem::zeroed() };
    let mut first_q0: Quat = unsafe { core::mem::zeroed() };
    let mut qs: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let mut total_weight: Real = 0.0;

    for i in 0..skin_vertex.num_weights {
        // C: `skin->weights.data[skin_vertex.weight_begin + i]` — `uint32_t`
        // arithmetic, so the sum wraps before it indexes.
        // SAFETY: `weight_begin + i` indexes within the deformer's own `weights`
        // list (the cluster's weight range the C loop walks).
        let weight: SkinWeight = unsafe {
            *skin
                .weights()
                .data
                .add(skin_vertex.weight_begin.wrapping_add(i) as usize)
        };
        // SAFETY: `weight.cluster_index` indexes the deformer's own `clusters`
        // pointer list, yielding a live `*mut SkinCluster`.
        let cluster: *mut SkinCluster = unsafe {
            *(skin.clusters().data as *const *mut SkinCluster).add(weight.cluster_index as usize)
        };
        // C: `const ufbx_node *node = cluster->bone_node; if (!node) continue;`
        // SAFETY: `cluster` is a live `SkinCluster` from the list; `opt_ptr`
        // reads its own `bone_node` field.
        let node: *const Node = unsafe { opt_ptr(&raw const (*cluster).bone_node) };
        if node.is_null() {
            continue;
        }

        total_weight += weight.weight;
        if skin_vertex.dq_weight > 0.0 {
            // SAFETY: same live `SkinCluster`; reading its own
            // `geometry_to_world_transform` field.
            let t: Transform = unsafe { (*cluster).geometry_to_world_transform };
            let mut vq0: Quat = t.rotation;
            if i == 0 {
                first_q0 = vq0;
            }

            if quat_dot(first_q0, vq0) < 0.0 {
                vq0.x = -vq0.x;
                vq0.y = -vq0.y;
                vq0.z = -vq0.z;
                vq0.w = -vq0.w;
            }

            // C: `ufbx_quat vqt = { 0.5f*t.translation.x, ... };` — three
            // initializers, `w` is zeroed.
            let vqt: Quat = Quat {
                x: 0.5 * t.translation.x,
                y: 0.5 * t.translation.y,
                z: 0.5 * t.translation.z,
                w: 0.0,
            };
            let vqe: Quat = mul_quat(vqt, vq0);
            // SAFETY: `q0`/`qe`/`qs` are live stack locals accumulated in place;
            // Raw addresses preserve C's address-of semantics for the stack locals.
            unsafe {
                add_weighted_quat(&raw mut q0, vq0, weight.weight);
                add_weighted_quat(&raw mut qe, vqe, weight.weight);
                add_weighted_vec3(&raw mut qs, t.scale, weight.weight);
            }
        }

        if skin_vertex.dq_weight < 1.0 {
            // SAFETY: `mat` is a live stack local and `cluster` is a live
            // `SkinCluster`; both raw addresses mirror the C operands.
            unsafe {
                add_weighted_mat(
                    &raw mut mat,
                    &raw const (*cluster).geometry_to_world,
                    (1.0 - skin_vertex.dq_weight) * weight.weight,
                )
            };
        }
    }

    if total_weight <= 0.0 {
        if !fallback.is_null() {
            // SAFETY: `fallback` is non-null here and points at a live `Matrix`
            // per this fn's contract; reading it by value.
            return unsafe { *fallback };
        } else {
            return IDENTITY_MATRIX;
        }
    }

    // C: `ufbx_fabs(total_weight - 1.0f)` — the subtraction computes in
    // `ufbx_real`, promotes to double at the call; `UFBX_EPSILON` promotes too.
    if math::fabs((total_weight - 1.0) as f64) > as_f64!(math::EPSILON) {
        let rcp_weight: Real = if math::fabs(total_weight as f64) > as_f64!(math::EPSILON) {
            1.0 / total_weight
        } else {
            0.0
        };
        if skin_vertex.dq_weight > 0.0 {
            q0.x *= rcp_weight;
            q0.y *= rcp_weight;
            q0.z *= rcp_weight;
            q0.w *= rcp_weight;
            qe.x *= rcp_weight;
            qe.y *= rcp_weight;
            qe.z *= rcp_weight;
            qe.w *= rcp_weight;
            qs.x *= rcp_weight;
            qs.y *= rcp_weight;
            qs.z *= rcp_weight;
        }
        if skin_vertex.dq_weight < 1.0 {
            mat.m00 *= rcp_weight;
            mat.m01 *= rcp_weight;
            mat.m02 *= rcp_weight;
            mat.m03 *= rcp_weight;
            mat.m10 *= rcp_weight;
            mat.m11 *= rcp_weight;
            mat.m12 *= rcp_weight;
            mat.m13 *= rcp_weight;
            mat.m20 *= rcp_weight;
            mat.m21 *= rcp_weight;
            mat.m22 *= rcp_weight;
            mat.m23 *= rcp_weight;
        }
    }

    if skin_vertex.dq_weight > 0.0 {
        // C: `ufbx_transform dqt; // ufbxi_uninit` — all ten scalars are
        // written below before `dqt` is read, so the zero-fill is inert.
        // SAFETY: an all-zero bit pattern is a valid `Transform` (all `Real`).
        let mut dqt: Transform = unsafe { core::mem::zeroed() }; // ufbxi_uninit
        let rcp_len: Real = (1.0
            / math::sqrt((q0.x * q0.x + q0.y * q0.y + q0.z * q0.z + q0.w * q0.w) as f64))
            as Real;
        let rcp_len2x2: Real = 2.0 * rcp_len * rcp_len;
        dqt.rotation.x = q0.x * rcp_len;
        dqt.rotation.y = q0.y * rcp_len;
        dqt.rotation.z = q0.z * rcp_len;
        dqt.rotation.w = q0.w * rcp_len;
        dqt.scale.x = qs.x;
        dqt.scale.y = qs.y;
        dqt.scale.z = qs.z;
        dqt.translation.x = rcp_len2x2 * (-qe.w * q0.x + qe.x * q0.w - qe.y * q0.z + qe.z * q0.y);
        dqt.translation.y = rcp_len2x2 * (-qe.w * q0.y + qe.x * q0.z + qe.y * q0.w - qe.z * q0.x);
        dqt.translation.z = rcp_len2x2 * (-qe.w * q0.z - qe.x * q0.y + qe.y * q0.x + qe.z * q0.w);
        // SAFETY: `dqt` is a live, fully written stack `Transform`; its raw
        // address is passed to `transform_to_matrix`.
        let dqm: Matrix = unsafe { transform_to_matrix(&raw const dqt) };
        if skin_vertex.dq_weight < 1.0 {
            // SAFETY: `mat` and `dqm` are live stack matrices; their raw
            // addresses mirror C's address-of operands.
            unsafe { add_weighted_mat(&raw mut mat, &raw const dqm, skin_vertex.dq_weight) };
        } else {
            mat = dqm;
        }
    }

    mat
}

// ufbx.c:32020-32033 `ufbx_get_blend_shape_offset_index`
// C's null-or-live `const ufbx_blend_shape *` param arrives as
// `Option<&View<_, M>>` (the boundary shims mint the view from the caller's
// pointer).
#[inline(never)]
pub(crate) fn get_blend_shape_offset_index<M: Mode>(
    shape: Option<&View<BlendShape, M>>,
    vertex: usize,
) -> u32 {
    ufbx_assert!(shape.is_some());
    // C: `if (!shape) return UFBX_NO_INDEX;` — the null arm is the `None` case.
    let Some(shape) = shape else {
        return NO_INDEX;
    };

    let mut index: usize = usize::MAX;
    let vertex_ix: u32 = vertex as u32;

    // SAFETY: `offset_vertices.data`/`num_offsets` are the viewed shape's own
    // fields, so the list addresses `num_offsets` live `u32`s, and each closure
    // derefs a `u32` the search keeps within `[0, num_offsets)`.
    unsafe {
        macro_lower_bound_eq::<u32>(
            16,
            &mut index,
            shape.offset_vertices().data,
            0,
            shape.num_offsets(),
            |a| *a < vertex_ix,
            |a| *a == vertex_ix,
        );
    }
    // C: `if (index >= UINT32_MAX)` — `UINT32_MAX` widens to `size_t`.
    if index >= u32::MAX as usize {
        return NO_INDEX;
    }

    index as u32
}

// ufbx.c:32035-32040 `ufbx_get_blend_shape_vertex_offset`
#[inline(never)]
pub(crate) unsafe fn get_blend_shape_vertex_offset(
    shape: *const BlendShape,
    vertex: usize,
) -> Vec3 {
    // SAFETY: `shape` is this `unsafe fn`'s own param — null or a live
    // `BlendShape` per its contract — so a non-null pointer roots a read-only
    // `View<_, Const>` for the offset-index lookup (the `None` arm carries C's
    // null case).
    let index: u32 = unsafe {
        get_blend_shape_offset_index(
            if shape.is_null() {
                None
            } else {
                Some(View::<BlendShape, Const>::from_ptr(shape))
            },
            vertex,
        )
    };
    if index == NO_INDEX {
        return ZERO_VEC3;
    }
    // SAFETY: a non-`NO_INDEX` result is `< num_offsets`, so
    // `position_offsets.data.add(index)` addresses a live `Vec3` offset of the
    // live `BlendShape` behind `shape`.
    unsafe { *(*shape).position_offsets.data.add(index as usize) }
}

// ufbx.c:32042-32060 `ufbx_get_blend_vertex_offset`
#[inline(never)]
pub(crate) unsafe fn get_blend_vertex_offset(blend: *const BlendDeformer, vertex: usize) -> Vec3 {
    ufbx_assert!(!blend.is_null());
    if blend.is_null() {
        return ZERO_VEC3;
    }

    let mut offset: Vec3 = ZERO_VEC3;

    // C: `ufbxi_for_ptr_list(ufbx_blend_channel, p_chan, blend->channels)`
    // SAFETY: `blend` is non-null here (checked above) and points at a live
    // `BlendDeformer` per this fn's contract; `channels.data`/`.count` are its
    // own list fields, so `add_ptr` yields the one-past-end pointer.
    let mut p_chan: *mut *mut BlendChannel =
        unsafe { (*blend).channels.data } as *mut *mut BlendChannel;
    // SAFETY: same live `BlendDeformer`, reading its own `channels.count`.
    let p_chan_end: *mut *mut BlendChannel = unsafe { add_ptr(p_chan, (*blend).channels.count) };
    while p_chan != p_chan_end {
        // SAFETY: `p_chan` is in `[data, end)` of the channel pointer list, so it
        // addresses a live `*mut BlendChannel` element.
        let chan: *mut BlendChannel = unsafe { *p_chan };
        // C: `ufbxi_for_list(ufbx_blend_keyframe, key, chan->keyframes)` —
        // indexed here because the body `continue`s (the C `for` advances the
        // iterator in its increment clause).
        // SAFETY: `chan` is a live `BlendChannel`; reading its own
        // `keyframes.count`.
        for key_ix in 0..unsafe { (*chan).keyframes.count } {
            // SAFETY: same live `BlendChannel`; `key_ix < keyframes.count`, so
            // `keyframes.data.add(key_ix)` addresses a live `BlendKeyframe`.
            let key: *mut BlendKeyframe =
                unsafe { ((*chan).keyframes.data as *mut BlendKeyframe).add(key_ix) };
            // SAFETY: `key` is a live `BlendKeyframe`; reading its own
            // `effective_weight` field.
            if unsafe { (*key).effective_weight } == 0.0 {
                continue;
            }

            // SAFETY: same live `BlendKeyframe`; the raw address identifies its
            // own `shape` field and feeds `get_blend_shape_vertex_offset`.
            let key_offset: Vec3 =
                unsafe { get_blend_shape_vertex_offset(ref_ptr(&raw const (*key).shape), vertex) };
            // SAFETY: `offset` is a live stack local;
            // reading the same keyframe's `effective_weight`.
            unsafe { add_weighted_vec3(&raw mut offset, key_offset, (*key).effective_weight) };
        }
        // SAFETY: `p_chan` is before `p_chan_end`, so stepping one element stays
        // within the channel pointer list (up to one-past-end).
        p_chan = unsafe { p_chan.add(1) };
    }

    offset
}

// ufbx.c:32062-32081 `ufbx_add_blend_shape_vertex_offsets`
pub(crate) unsafe fn add_blend_shape_vertex_offsets(
    shape: *const BlendShape,
    vertices: *mut Vec3,
    num_vertices: usize,
    weight: Real,
) {
    if weight == 0.0 {
        return;
    }
    if vertices.is_null() {
        return;
    }

    // SAFETY: `shape` points at a live `BlendShape` per this fn's contract;
    // every field read below is one of its own list fields.
    let (num_offsets, vertex_indices, offsets, weights_data, weights_count) = unsafe {
        (
            (*shape).num_offsets,
            (*shape).offset_vertices.data,
            (*shape).position_offsets.data,
            (*shape).offset_weights.data,
            (*shape).offset_weights.count,
        )
    };
    for i in 0..num_offsets {
        // SAFETY: `i < num_offsets`, so `vertex_indices.add(i)` addresses a live
        // `u32` in the shape's `offset_vertices` list.
        let index: u32 = unsafe { *vertex_indices.add(i) };
        // C: `index < num_vertices` — `uint32_t` widens to `size_t`.
        if (index as usize) < num_vertices {
            let mut vertex_weight: Real = weight;
            if i < weights_count {
                // SAFETY: `i < weights_count`, so `weights_data.add(i)` addresses a
                // live `Real` in the shape's `offset_weights` list.
                vertex_weight *= unsafe { *weights_data.add(i) };
            }
            // SAFETY: `index < num_vertices` bounds `vertices.add(index)` within
            // the caller's `vertices` buffer; `i < num_offsets` bounds
            // `offsets.add(i)` within the shape's `position_offsets` list.
            unsafe {
                add_weighted_vec3(vertices.add(index as usize), *offsets.add(i), vertex_weight)
            };
        }
    }
}

// ufbx.c:32083-32095 `ufbx_add_blend_vertex_offsets`
pub(crate) unsafe fn add_blend_vertex_offsets(
    blend: *const BlendDeformer,
    vertices: *mut Vec3,
    num_vertices: usize,
    weight: Real,
) {
    ufbx_assert!(!blend.is_null());
    if blend.is_null() {
        return;
    }

    // C: `ufbxi_for_ptr_list(ufbx_blend_channel, p_chan, blend->channels)`
    // SAFETY: `blend` is non-null here (checked above) and points at a live
    // `BlendDeformer` per this fn's contract; `channels.data`/`.count` are its
    // own list fields, so `add_ptr` yields the one-past-end pointer.
    let mut p_chan: *mut *mut BlendChannel =
        unsafe { (*blend).channels.data } as *mut *mut BlendChannel;
    // SAFETY: same live `BlendDeformer`, reading its own `channels.count`.
    let p_chan_end: *mut *mut BlendChannel = unsafe { add_ptr(p_chan, (*blend).channels.count) };
    while p_chan != p_chan_end {
        // SAFETY: `p_chan` is in `[data, end)` of the channel pointer list, so it
        // addresses a live `*mut BlendChannel` element.
        let chan: *mut BlendChannel = unsafe { *p_chan };
        // C: `ufbxi_for_list(ufbx_blend_keyframe, key, chan->keyframes)` —
        // indexed here because the body `continue`s (the C `for` advances the
        // iterator in its increment clause).
        // SAFETY: `chan` is a live `BlendChannel`; reading its own
        // `keyframes.count`.
        for key_ix in 0..unsafe { (*chan).keyframes.count } {
            // SAFETY: same live `BlendChannel`; `key_ix < keyframes.count`, so
            // `keyframes.data.add(key_ix)` addresses a live `BlendKeyframe`.
            let key: *mut BlendKeyframe =
                unsafe { ((*chan).keyframes.data as *mut BlendKeyframe).add(key_ix) };
            // SAFETY: `key` is a live `BlendKeyframe`; reading its own
            // `effective_weight` field.
            if unsafe { (*key).effective_weight } == 0.0 {
                continue;
            }
            // SAFETY: same live `BlendKeyframe`; the raw address identifies its
            // own `shape` field and the weight reads its own `effective_weight`.
            unsafe {
                add_blend_shape_vertex_offsets(
                    ref_ptr(&raw const (*key).shape),
                    vertices,
                    num_vertices,
                    weight * (*key).effective_weight,
                )
            };
        }
        // SAFETY: `p_chan` is before `p_chan_end`, so stepping one element stays
        // within the channel pointer list (up to one-past-end).
        p_chan = unsafe { p_chan.add(1) };
    }
}

// ufbx.c:32097-32166 `ufbx_evaluate_nurbs_basis`
pub(crate) unsafe fn evaluate_nurbs_basis(
    basis: *const NurbsBasis,
    mut u: Real,
    weights: *mut Real,
    num_weights: usize,
    mut derivatives: *mut Real,
    num_derivatives: usize,
) -> usize {
    ufbx_assert!(!basis.is_null());
    if basis.is_null() {
        return usize::MAX;
    }
    // SAFETY: `basis` is non-null here (checked above) and points at a live
    // `NurbsBasis` per this fn's contract; reading its own `order` field.
    if unsafe { (*basis).order } == 0 {
        return usize::MAX;
    }
    // SAFETY: same live `NurbsBasis`; reading its own `valid` field.
    if unsafe { !(*basis).valid } {
        return usize::MAX;
    }

    // SAFETY: same live `NurbsBasis`; reading its own `order` field.
    let degree: usize = (unsafe { (*basis).order } - 1) as usize;
    ufbx_assert!(degree >= 1);

    // Binary search for the knot span `[min_u, max_u]` where `min_u <= u < max_u`
    // C: `ufbx_real_list knots = basis->knot_vector;` — a by-value list copy;
    // `List` is not `Copy`, so read through a pointer to the same data.
    // SAFETY: same live `NurbsBasis`; the raw field address preserves C's
    // address-of semantics without creating a Rust reference.
    let knots: *const List<Real> = unsafe { &raw const (*basis).knot_vector };
    let mut knot: usize = usize::MAX;

    // SAFETY: same live `NurbsBasis`; reading its own `t_min` field.
    if u <= unsafe { (*basis).t_min } {
        knot = degree;
        // SAFETY: as above.
        u = unsafe { (*basis).t_min };
    } else if u >= unsafe { (*basis).t_max } {
        // SAFETY: as above, reading its own `knot_vector.count` and `t_max`.
        unsafe {
            knot = (*basis)
                .knot_vector
                .count
                .wrapping_sub(degree)
                .wrapping_sub(2);
            u = (*basis).t_max;
        }
    } else {
        // SAFETY: `knots` points at the basis's live knot-vector list;
        // `(*knots).data`/`.count` are its own fields, and each closure derefs
        // knot entries the search keeps within `[0, count-1)`.
        unsafe {
            macro_lower_bound_eq::<Real>(
                8,
                &mut knot,
                (*knots).data,
                0,
                (*knots).count.wrapping_sub(1),
                // C: `( a[1] <= u )`
                |a| *a.add(1) <= u,
                // C: `( a[0] <= u && u < a[1] )`
                |a| *a.add(0) <= u && u < *a.add(1),
            );
        }
    }

    // The found effective control points are found left from `knot`, locally
    // we use `knot - ix` here as it's more convenient for the following algorithm
    // but we return it as `knot - degree` so that users can find the control points
    // at `points[knot], points[knot+1], ..., points[knot+degree]`
    if knot < degree {
        return usize::MAX;
    }

    if num_derivatives == 0 {
        derivatives = core::ptr::null_mut();
    }
    // SAFETY: `basis` points at a live `NurbsBasis`; reading its own `order`.
    if num_weights < unsafe { (*basis).order } as usize {
        return knot - degree;
    }
    if weights.is_null() {
        return knot - degree;
    }

    // SAFETY: `weights` is non-null here with `num_weights >= order` entries, so
    // index 0 is in bounds of the caller's weight buffer.
    unsafe {
        *weights.add(0) = 1.0f32 as Real;
    }
    for p in 1..=degree {
        let mut prev: Real = 0.0f32 as Real;
        // SAFETY: `knots` points at the basis's live knot-vector list; the index
        // stays within it for `p <= degree` (the C algorithm's span window).
        let mut g: Real = 1.0f32 as Real - unsafe { nurbs_weight(knots, knot - p + 1, p, u) };
        let mut dg: Real = 0.0f32 as Real;
        if !derivatives.is_null() && p == degree {
            // SAFETY: as above.
            dg = unsafe { nurbs_deriv(knots, knot - p + 1, p) };
        }

        // C: `for (size_t i = p; i > 0; i--)`
        let mut i: usize = p;
        while i > 0 {
            // SAFETY: `knots` points at the basis's live knot-vector list; the
            // index stays within it for `i <= p <= degree`.
            let f: Real = unsafe { nurbs_weight(knots, knot - p + i, p, u) };
            // SAFETY: `weights` has `num_weights >= order > degree >= i` entries,
            // so `i - 1` is in bounds of the caller's weight buffer.
            let weight: Real = unsafe { *weights.add(i - 1) };
            // SAFETY: as above, `i <= degree < order <= num_weights`.
            unsafe {
                *weights.add(i) = f * weight + g * prev;
            }

            if !derivatives.is_null() && p == degree {
                // SAFETY: `knots` points at the basis's live knot-vector list.
                let df: Real = unsafe { nurbs_deriv(knots, knot - p + i, p) };
                if i < num_derivatives {
                    // SAFETY: `derivatives` is non-null here with `num_derivatives`
                    // entries and `i < num_derivatives`, so `i` is in bounds.
                    unsafe {
                        *derivatives.add(i) = df * weight - dg * prev;
                    }
                }
                dg = df;
            }

            prev = weight;
            g = 1.0f32 as Real - f;
            i -= 1;
        }

        // SAFETY: index 0 is in bounds of the caller's weight buffer.
        unsafe {
            *weights.add(0) = g * prev;
        }
        if !derivatives.is_null() && p == degree {
            // SAFETY: `derivatives` is non-null here, which (given it was nulled
            // above when `num_derivatives == 0`) implies `num_derivatives >= 1`,
            // so index 0 is in bounds of the caller's derivative buffer.
            unsafe {
                *derivatives.add(0) = -dg * prev;
            }
        }
    }

    knot - degree
}

// ufbx.c:32168-32212 `ufbx_evaluate_nurbs_curve`
#[inline(never)]
pub(crate) unsafe fn evaluate_nurbs_curve(curve: *const NurbsCurve, u: Real) -> CurvePoint {
    // C: `ufbx_curve_point result = { false };`
    // SAFETY: an all-zero bit pattern is a valid `CurvePoint` (a `bool` flag and
    // `Real` vectors).
    let mut result: CurvePoint = unsafe { core::mem::zeroed() };

    ufbx_assert!(!curve.is_null());
    if curve.is_null() {
        return result;
    }

    // SAFETY: an all-zero bit pattern is a valid `[Real; MAX_NURBS_ORDER]`.
    let (mut weights, mut derivs): ([Real; MAX_NURBS_ORDER], [Real; MAX_NURBS_ORDER]) = unsafe {
        (
            core::mem::zeroed(), // ufbxi_uninit
            core::mem::zeroed(), // ufbxi_uninit
        )
    };
    // SAFETY: `curve` is non-null here (checked above) and points at a live
    // `NurbsCurve` per this fn's contract; the raw basis-field address and
    // `weights`/`derivs` are live buffers of length
    // `MAX_NURBS_ORDER`.
    let base: usize = unsafe {
        evaluate_nurbs_basis(
            &raw const (*curve).basis,
            u,
            weights.as_mut_ptr(),
            MAX_NURBS_ORDER,
            derivs.as_mut_ptr(),
            MAX_NURBS_ORDER,
        )
    };
    if base == usize::MAX {
        return result;
    }

    // SAFETY: an all-zero bit pattern is a valid `Vec4` (all `Real` fields).
    let (mut p, mut d): (Vec4, Vec4) = unsafe { (core::mem::zeroed(), core::mem::zeroed()) };

    // SAFETY: same live `NurbsCurve`; reading its own `basis.order`.
    let order: usize = unsafe { (*curve).basis.order } as usize;
    if order > MAX_NURBS_ORDER {
        return result;
    }
    // SAFETY: same live `NurbsCurve`; reading its own `control_points.count`.
    if unsafe { (*curve).control_points.count } == 0 {
        return result;
    }

    for i in 0..order {
        // SAFETY: same live `NurbsCurve`; reading its own `control_points.count`.
        let ix: usize = base.wrapping_add(i) % unsafe { (*curve).control_points.count };
        // SAFETY: `ix < control_points.count` (modulo above), so
        // `control_points.data.add(ix)` addresses a live `Vec4` control point.
        let cp: Vec4 = unsafe { *(*curve).control_points.data.add(ix) };
        let weight: Real = weights[i] * cp.w;
        let deriv: Real = derivs[i] * cp.w;

        p.x += cp.x * weight;
        p.y += cp.y * weight;
        p.z += cp.z * weight;
        p.w += weight;

        d.x += cp.x * deriv;
        d.y += cp.y * deriv;
        d.z += cp.z * deriv;
        d.w += deriv;
    }

    let rcp_w: Real = 1.0f32 as Real / p.w;
    result.valid = true;
    result.position.x = p.x * rcp_w;
    result.position.y = p.y * rcp_w;
    result.position.z = p.z * rcp_w;
    result.derivative.x = (d.x - d.w * result.position.x) * rcp_w;
    result.derivative.y = (d.y - d.w * result.position.y) * rcp_w;
    result.derivative.z = (d.z - d.w * result.position.z) * rcp_w;
    result
}

// ufbx.c:32214-32280 `ufbx_evaluate_nurbs_surface`
#[inline(never)]
pub(crate) unsafe fn evaluate_nurbs_surface(
    surface: *const NurbsSurface,
    u: Real,
    v: Real,
) -> SurfacePoint {
    // C: `ufbx_surface_point result = { false };`
    // SAFETY: an all-zero bit pattern is a valid `SurfacePoint` (a `bool` flag
    // and `Real` vectors).
    let mut result: SurfacePoint = unsafe { core::mem::zeroed() };

    ufbx_assert!(!surface.is_null());
    if surface.is_null() {
        return result;
    }

    // SAFETY: an all-zero bit pattern is a valid `[Real; MAX_NURBS_ORDER]`.
    let (mut weights_u, mut weights_v, mut derivs_u, mut derivs_v): (
        [Real; MAX_NURBS_ORDER],
        [Real; MAX_NURBS_ORDER],
        [Real; MAX_NURBS_ORDER],
        [Real; MAX_NURBS_ORDER],
    ) = unsafe {
        (
            core::mem::zeroed(), // ufbxi_uninit
            core::mem::zeroed(), // ufbxi_uninit
            core::mem::zeroed(), // ufbxi_uninit
            core::mem::zeroed(), // ufbxi_uninit
        )
    };
    // SAFETY: `surface` is non-null here (checked above) and points at a live
    // `NurbsSurface` per this fn's contract; the raw basis-field address and
    // `weights_u`/`derivs_u` are live stack buffers of
    // length `MAX_NURBS_ORDER`.
    let base_u: usize = unsafe {
        evaluate_nurbs_basis(
            &raw const (*surface).basis_u,
            u,
            weights_u.as_mut_ptr(),
            MAX_NURBS_ORDER,
            derivs_u.as_mut_ptr(),
            MAX_NURBS_ORDER,
        )
    };
    // SAFETY: same live `NurbsSurface`; the raw basis-field address and
    // `weights_v`/`derivs_v` are live stack buffers.
    let base_v: usize = unsafe {
        evaluate_nurbs_basis(
            &raw const (*surface).basis_v,
            v,
            weights_v.as_mut_ptr(),
            MAX_NURBS_ORDER,
            derivs_v.as_mut_ptr(),
            MAX_NURBS_ORDER,
        )
    };
    if base_u == usize::MAX || base_v == usize::MAX {
        return result;
    }

    // SAFETY: an all-zero bit pattern is a valid `Vec4` (all `Real` fields).
    let (mut p, mut du, mut dv): (Vec4, Vec4, Vec4) = unsafe {
        (
            core::mem::zeroed(),
            core::mem::zeroed(),
            core::mem::zeroed(),
        )
    };

    // SAFETY: same live `NurbsSurface`; every field read below is one of its own
    // control-point-count / basis-order fields.
    let (num_u, num_v, order_u, order_v) = unsafe {
        (
            (*surface).num_control_points_u,
            (*surface).num_control_points_v,
            (*surface).basis_u.order as usize,
            (*surface).basis_v.order as usize,
        )
    };
    if order_u > MAX_NURBS_ORDER || order_v > MAX_NURBS_ORDER {
        return result;
    }
    if num_u == 0 || num_v == 0 {
        return result;
    }

    for vi in 0..order_v {
        let vix: usize = base_v.wrapping_add(vi) % num_v;
        let weight_v: Real = weights_v[vi];
        let deriv_v: Real = derivs_v[vi];

        for ui in 0..order_u {
            let uix: usize = base_u.wrapping_add(ui) % num_u;
            let weight_u: Real = weights_u[ui];
            let deriv_u: Real = derivs_u[ui];
            // SAFETY: `uix < num_u` and `vix < num_v`, so
            // `vix*num_u + uix < num_u*num_v` is in bounds of the surface's
            // `control_points` grid; `.add(..)` addresses a live `Vec4`.
            let cp: Vec4 = unsafe {
                *(*surface)
                    .control_points
                    .data
                    .add(vix.wrapping_mul(num_u).wrapping_add(uix))
            };

            let weight: Real = weight_u * weight_v * cp.w;
            let wderiv_u: Real = deriv_u * weight_v * cp.w;
            let wderiv_v: Real = deriv_v * weight_u * cp.w;

            p.x += cp.x * weight;
            p.y += cp.y * weight;
            p.z += cp.z * weight;
            p.w += weight;

            du.x += cp.x * wderiv_u;
            du.y += cp.y * wderiv_u;
            du.z += cp.z * wderiv_u;
            du.w += wderiv_u;

            dv.x += cp.x * wderiv_v;
            dv.y += cp.y * wderiv_v;
            dv.z += cp.z * wderiv_v;
            dv.w += wderiv_v;
        }
    }

    let rcp_w: Real = 1.0f32 as Real / p.w;
    result.valid = true;
    result.position.x = p.x * rcp_w;
    result.position.y = p.y * rcp_w;
    result.position.z = p.z * rcp_w;
    result.derivative_u.x = (du.x - du.w * result.position.x) * rcp_w;
    result.derivative_u.y = (du.y - du.w * result.position.y) * rcp_w;
    result.derivative_u.z = (du.z - du.w * result.position.z) * rcp_w;
    result.derivative_v.x = (dv.x - dv.w * result.position.x) * rcp_w;
    result.derivative_v.y = (dv.y - dv.w * result.position.y) * rcp_w;
    result.derivative_v.z = (dv.z - dv.w * result.position.z) * rcp_w;
    result
}

// ufbx.c:32282-32318 `ufbx_tessellate_nurbs_curve`
// C forks on `#if UFBXI_FEATURE_TESSELLATION`; the whole body is inside the
// fork (no shared prologue), so each arm is a separate cfg'd fn — the same
// split `ufbxi_obj_load` uses in `native::obj`.
#[cfg(feature = "tessellation")]
pub(crate) unsafe fn tessellate_nurbs_curve(
    curve: *const NurbsCurve,
    opts: *const RawTessellateCurveOpts,
) -> Result<*mut LineCurve, Error> {
    // SAFETY: `opts` is this fn's raw-pointer param; the macro reads its
    // `_begin_zero`/`_end_zero` guard fields only after a null check.
    unsafe { ufbxi_check_opts_res!(opts) };
    ufbx_assert!(!curve.is_null());
    if curve.is_null() {
        // C's silent NULL: no slot write on this path — the shim clears the
        // caller slot only for an `Ok` with a non-null payload.
        return Ok(core::ptr::null_mut());
    }

    // C: `ufbxi_tessellate_curve_context tc = { UFBX_ERROR_NONE };`
    let tc = TessellateCurveContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::zeroed()));
    if !opts.is_null() {
        // C: `tc->opts = *opts` — struct assignment (memcpy).
        // SAFETY: `opts` is non-null here and points at a live
        // `RawTessellateCurveOpts` per this fn's contract; `opts_mut_ptr()`
        // addresses `tc`'s own owned opts storage, a distinct allocation.
        unsafe { core::ptr::copy_nonoverlapping(opts, tc.opts_mut_ptr(), 1) };
    }

    tc.set_curve(curve);

    // C: `int ok = ufbxi_tessellate_nurbs_curve_imp(&tc);` — on success the
    // `FinishedImp` carries the finished imp through the teardown to the return.
    let result = tessellate_nurbs_curve_imp(&tc);

    // SAFETY: `ator_tmp_mut_ptr()` addresses `tc`'s own temp allocator.
    unsafe { free_ator(tc.ator_tmp_mut_ptr()) };

    if let Ok(finished_imp) = result {
        // C: `return &tc->imp->curve;` — commit the finished imp across the ABI. (The
        // success-path `clear_error` of the caller's slot lives in the shim.)
        Ok(finished_imp.into_payload())
    } else {
        // C copies the fixed error into the caller's slot; the `Result` shape
        // carries it by value (the shim owns the slot writes).
        let mut fixed: Error = Error::default();
        // SAFETY: `error_mut_ptr()` addresses `tc`'s own error; the byte literal
        // is NUL-terminated; `&raw mut fixed` is this frame's live `Error`.
        unsafe {
            fix_error_type(
                tc.error_mut_ptr(),
                b"Failed to tessellate\0",
                &raw mut fixed,
            );
        }
        // SAFETY: `result_mut_ptr()`/`ator_result_mut_ptr()` address `tc`'s own
        // result buffer and result allocator.
        unsafe {
            buf_free(tc.result_mut_ptr());
            free_ator(tc.ator_result_mut_ptr());
        }
        Err(fixed)
    }
}

// ufbx.c:32282-32318 `ufbx_tessellate_nurbs_curve` (`#else` arm — feature
// disabled). That arm is C parity (a build without `feature = "tessellation"`
// reports `UFBX_ERROR_FEATURE_DISABLED`), NOT a stub.
#[cfg(not(feature = "tessellation"))]
pub(crate) unsafe fn tessellate_nurbs_curve(
    curve: *const crate::generated::NurbsCurve,
    opts: *const crate::generated::RawTessellateCurveOpts,
) -> Result<*mut crate::generated::LineCurve, Error> {
    // C: `curve`/`opts` are unreferenced in the `#else` arm.
    let _ = (curve, opts);
    // C zero-fills the caller slot then formats into it; the `Result` shape
    // builds the same bytes in a local carried by `Err` (the shim owns the
    // slot writes).
    let mut error: Error = Error::default();
    // SAFETY: `&raw mut error` is this frame's live `Error` slot the `%s`-less
    // format writes into.
    unsafe { ufbxi_fmt_err_info!(&raw mut error, "UFBX_ENABLE_TESSELLATION") };
    ufbxi_report_err_msg!(
        // SAFETY: same live local `Error` slot, minted as a view for the report.
        unsafe { crate::native::error::ErrorView::from_ptr(&raw mut error) },
        "UFBXI_FEATURE_TESSELLATION",
        "Feature disabled"
    );
    Err(error)
}

// ufbx.c:32320-32357 `ufbx_tessellate_nurbs_surface`
// Same `#if UFBXI_FEATURE_TESSELLATION` split as `ufbx_tessellate_nurbs_curve`
// above. C-parity notes: `ufbx_assert(surface)` sits BEFORE
// `ufbxi_check_opts_ptr` here (the curve variant has the opposite order).
#[cfg(feature = "tessellation")]
pub(crate) unsafe fn tessellate_nurbs_surface(
    surface: *const NurbsSurface,
    opts: *const RawTessellateSurfaceOpts,
) -> Result<*mut Mesh, Error> {
    ufbx_assert!(!surface.is_null());
    // SAFETY: `opts` is this fn's raw-pointer param; the macro reads its
    // `_begin_zero`/`_end_zero` guard fields only after a null check.
    unsafe { ufbxi_check_opts_res!(opts) };
    if surface.is_null() {
        // C's silent NULL: no slot write on this path — the shim clears the
        // caller slot only for an `Ok` with a non-null payload.
        return Ok(core::ptr::null_mut());
    }

    // C: `ufbxi_tessellate_surface_context tc = { UFBX_ERROR_NONE };`
    let tc =
        TessellateSurfaceContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::zeroed()));
    if !opts.is_null() {
        // C: `tc->opts = *opts` — struct assignment (memcpy).
        // SAFETY: `opts` is non-null here and points at a live
        // `RawTessellateSurfaceOpts` per this fn's contract; `opts_mut_ptr()`
        // addresses `tc`'s own owned opts storage, a distinct allocation.
        unsafe { core::ptr::copy_nonoverlapping(opts, tc.opts_mut_ptr(), 1) };
    }

    tc.set_surface(surface);

    // C: `int ok = ufbxi_tessellate_nurbs_surface_imp(&tc);` — on success the
    // `FinishedImp` carries the finished imp through the teardown to the return.
    let result = tessellate_nurbs_surface_imp(&tc);

    // SAFETY: these accessors address `tc`'s own temp buffer, position map, and
    // temp allocator.
    unsafe {
        buf_free(tc.tmp_mut_ptr());
        map_free(tc.position_map_mut_ptr());
        free_ator(tc.ator_tmp_mut_ptr());
    }

    if let Ok(finished_imp) = result {
        // C: `return &tc->imp->mesh;` — commit the finished imp across the ABI. (The
        // success-path `clear_error` of the caller's slot lives in the shim.)
        Ok(finished_imp.into_payload())
    } else {
        // C copies the fixed error into the caller's slot; the `Result` shape
        // carries it by value (the shim owns the slot writes).
        let mut fixed: Error = Error::default();
        // SAFETY: `error_mut_ptr()` addresses `tc`'s own error; the byte literal
        // is NUL-terminated; `&raw mut fixed` is this frame's live `Error`.
        unsafe {
            fix_error_type(
                tc.error_mut_ptr(),
                b"Failed to tessellate\0",
                &raw mut fixed,
            );
        }
        // SAFETY: `result_mut_ptr()`/`ator_result_mut_ptr()` address `tc`'s own
        // result buffer and result allocator.
        unsafe {
            buf_free(tc.result_mut_ptr());
            free_ator(tc.ator_result_mut_ptr());
        }
        Err(fixed)
    }
}

// ufbx.c:32320-32357 `ufbx_tessellate_nurbs_surface` (`#else` arm — feature
// disabled). C-parity note: this arm has NO `ufbxi_fmt_err_info` call (unlike
// `ufbx_tessellate_nurbs_curve` above) — do not add one.
#[cfg(not(feature = "tessellation"))]
pub(crate) unsafe fn tessellate_nurbs_surface(
    surface: *const crate::generated::NurbsSurface,
    opts: *const crate::generated::RawTessellateSurfaceOpts,
) -> Result<*mut Mesh, Error> {
    // C: `surface`/`opts` are unreferenced in the `#else` arm.
    let _ = (surface, opts);
    // C zero-fills the caller slot then formats into it; the `Result` shape
    // builds the same bytes in a local carried by `Err` (the shim owns the
    // slot writes).
    let mut error: Error = Error::default();
    ufbxi_report_err_msg!(
        // SAFETY: `&raw mut error` is this frame's live `Error` slot, minted as
        // a view for the report.
        unsafe { crate::native::error::ErrorView::from_ptr(&raw mut error) },
        "UFBXI_FEATURE_TESSELLATION",
        "Feature disabled"
    );
    Err(error)
}

// ufbx.c:32359-32368 `ufbx_free_line_curve`
// Not feature-gated in C: `ufbxi_line_curve_imp` sits before the
// `#if UFBXI_FEATURE_TESSELLATION` fork (see `native::nurbs`).
pub(crate) unsafe fn free_line_curve(line_curve: *mut LineCurve) {
    if line_curve.is_null() {
        return;
    }
    // SAFETY: `line_curve` is non-null here and points at a live `LineCurve` —
    // the raw-pointer contract of this `unsafe fn`; reading its own field.
    if !unsafe { (*line_curve).from_tessellated_nurbs } {
        return;
    }

    // SAFETY: the tessellated `line_curve` is the payload of a live
    // `LineCurveImp` handed out by this library — the raw-pointer contract of
    // this `unsafe fn`.
    let imp = unsafe { ImpHandle::<LineCurveImp>::from_payload(line_curve) };
    ufbx_assert!(imp.has_magic());
    if !imp.has_magic() {
        return;
    }
    imp.release();
}

// ufbx.c:32370-32379 `ufbx_retain_line_curve`
pub(crate) unsafe fn retain_line_curve(line_curve: *mut LineCurve) {
    if line_curve.is_null() {
        return;
    }
    // SAFETY: `line_curve` is non-null here and points at a live `LineCurve` —
    // the raw-pointer contract of this `unsafe fn`; reading its own field.
    if !unsafe { (*line_curve).from_tessellated_nurbs } {
        return;
    }

    // SAFETY: the tessellated `line_curve` is the payload of a live
    // `LineCurveImp` handed out by this library — the raw-pointer contract of
    // this `unsafe fn`.
    let imp = unsafe { ImpHandle::<LineCurveImp>::from_payload(line_curve) };
    ufbx_assert!(imp.has_magic());
    if !imp.has_magic() {
        return;
    }
    imp.retain();
}

// ufbx.c:32381-32390 `ufbx_find_face_index`
pub(crate) unsafe fn find_face_index(mesh: *mut Mesh, index: usize) -> u32 {
    // C: `!mesh || index > UINT32_MAX` — `index` is `size_t`.
    if mesh.is_null() || index > u32::MAX as usize {
        return NO_INDEX;
    }
    let ix: u32 = index as u32;

    // SAFETY: `mesh` is non-null here (checked above) and points at a live `Mesh`
    // per this fn's raw-pointer contract. C only reads the mesh here, so a
    // read-only `Const` view is minted; nothing writes those bytes while it is
    // live.
    let mesh = unsafe { View::<Mesh, Const>::from_ptr(mesh) };

    match mesh.faces_view().lower_bound_eq(
        4,
        // C: `a->index_begin + a->num_indices <= ix` — `uint32_t` arithmetic.
        |a| a.index_begin().wrapping_add(a.num_indices()) <= ix,
        // C: `ix >= a->index_begin && ix < a->index_begin + a->num_indices`.
        |a| ix >= a.index_begin() && ix < a.index_begin().wrapping_add(a.num_indices()),
    ) {
        Some(face_ix) => face_ix as u32,
        // C: `(uint32_t)face_ix` — a miss keeps `SIZE_MAX`, truncating to
        // `UFBX_NO_INDEX`.
        None => usize::MAX as u32,
    }
}

// ufbx.c:32392-32475 `ufbx_catch_triangulate_face`
// C forks on `#if UFBXI_FEATURE_TRIANGULATION`; the enabled arm drives
// `ufbxi_ngon_context` / `ufbxi_triangulate_ngon` (`native/topology.rs`), the
// `#else` arm just records a panic and returns 0. Both arms are ported.
// C: `ufbx_abi ufbxi_noinline` (ufbx.c:32392).
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn catch_triangulate_face<M: Mode>(
    mut panic: Option<&mut Panic>,
    indices: *mut u32,
    num_indices: usize,
    mesh: &View<Mesh, M>,
    face: Face,
) -> u32 {
    if face.num_indices < 3 {
        return 0;
    }

    let required_indices: usize = (face.num_indices as usize).wrapping_sub(2).wrapping_mul(3);
    if ufbxi_panicf!(
        panic,
        num_indices >= required_indices,
        "Face needs at least %zu indices for triangles, got space for %zu",
        required_indices,
        num_indices,
    ) {
        return 0;
    }
    if ufbxi_panicf!(
        panic,
        (face.index_begin as usize) < mesh.num_indices(),
        "Face index begin (%u) out of bounds (%zu)",
        face.index_begin,
        mesh.num_indices(),
    ) {
        return 0;
    }
    if ufbxi_panicf!(
        panic,
        mesh.num_indices().wrapping_sub(face.index_begin as usize) >= face.num_indices as usize,
        "Face index end (%u + %u) out of bounds (%zu)",
        face.index_begin,
        face.num_indices,
        mesh.num_indices(),
    ) {
        return 0;
    }

    if face.num_indices == 3 {
        // Fast case: Already a triangle
        // SAFETY: `num_indices >= required_indices` was guarded above (`>= 3` for
        // a triangle), so `indices.add(0..=2)` address distinct caller-reserved
        // slots.
        unsafe {
            *indices.add(0) = face.index_begin.wrapping_add(0);
            *indices.add(1) = face.index_begin.wrapping_add(1);
            *indices.add(2) = face.index_begin.wrapping_add(2);
        }
        1
    } else if face.num_indices == 4 {
        // Quad: Split along the shortest axis unless a vertex crosses the axis
        let i0: u32 = face.index_begin.wrapping_add(0);
        let i1: u32 = face.index_begin.wrapping_add(1);
        let i2: u32 = face.index_begin.wrapping_add(2);
        let i3: u32 = face.index_begin.wrapping_add(3);
        // SAFETY: `i0`..`i3` are within the face's index range (bounds guarded
        // above), keeping each `indices.data` read inside the mesh's own
        // `vertex_position.indices` run (`count == num_indices`); the fetched
        // values index `values.data` in bounds per the mesh's index-validity
        // invariant (indices sanitized at load).
        let (v0, v1, v2, v3): (Vec3, Vec3, Vec3, Vec3) = unsafe {
            let values = mesh.vertex_position().values_data();
            let indices_data = mesh.vertex_position().indices_data();
            (
                *values.add(*indices_data.add(i0 as usize) as usize),
                *values.add(*indices_data.add(i1 as usize) as usize),
                *values.add(*indices_data.add(i2 as usize) as usize),
                *values.add(*indices_data.add(i3 as usize) as usize),
            )
        };

        let a: Vec3 = sub3(v2, v0);
        let b: Vec3 = sub3(v3, v1);

        let na1: Vec3 = normalize3(cross3(a, sub3(v1, v0)));
        let na3: Vec3 = normalize3(cross3(a, sub3(v0, v3)));
        let nb0: Vec3 = normalize3(cross3(b, sub3(v1, v0)));
        let nb2: Vec3 = normalize3(cross3(b, sub3(v2, v1)));

        let dot_aa: Real = dot3(a, a);
        let dot_bb: Real = dot3(b, b);
        let dot_na: Real = dot3(na1, na3);
        let dot_nb: Real = dot3(nb0, nb2);

        let mut split_a: bool = dot_aa <= dot_bb;

        if dot_na < 0.0 || dot_nb < 0.0 {
            split_a = dot_na >= dot_nb;
        }

        // SAFETY: a quad needs `required_indices == 6` slots, guarded above, so
        // `indices.add(0..=5)` address distinct caller-reserved slots.
        if split_a {
            unsafe {
                *indices.add(0) = i0;
                *indices.add(1) = i1;
                *indices.add(2) = i2;
                *indices.add(3) = i2;
                *indices.add(4) = i3;
                *indices.add(5) = i0;
            }
        } else {
            unsafe {
                *indices.add(0) = i1;
                *indices.add(1) = i2;
                *indices.add(2) = i3;
                *indices.add(3) = i3;
                *indices.add(4) = i0;
                *indices.add(5) = i1;
            }
        }

        2
    } else {
        // C: `ufbxi_ngon_context nc = { 0 };`
        let nc = crate::native::topology::NgonContext(core::cell::UnsafeCell::new(
            core::mem::MaybeUninit::zeroed(),
        ));
        // SAFETY: `positions_mut_ptr()` addresses `nc`'s own positions slot; the
        // mesh's `vertex_position` is read by value (a `Copy` attribute) through
        // its own in-place projection and written into that slot.
        unsafe {
            core::ptr::write(
                nc.positions_mut_ptr(),
                core::ptr::read(mesh.vertex_position().as_ptr()),
            );
        }
        nc.set_face(face);

        let num_indices_u32: u32 = if num_indices < u32::MAX as usize {
            num_indices as u32
        } else {
            u32::MAX
        };

        // SAFETY: an all-zero bit pattern is a valid `[u32; 12]`.
        let mut local_indices: [u32; 12] = unsafe { core::mem::zeroed() }; // ufbxi_uninit
        if num_indices_u32 < 12 {
            // SAFETY: `local_indices` has 12 slots for `triangulate_ngon` to fill.
            let num_tris: u32 = unsafe {
                crate::native::topology::triangulate_ngon(&nc, local_indices.as_mut_ptr(), 12)
            };
            // SAFETY: `triangulate_ngon` wrote `num_tris * 3` indices into
            // `local_indices`, and `indices` has room for `required_indices`.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    local_indices.as_ptr(),
                    indices,
                    num_tris.wrapping_mul(3) as usize,
                );
            }
            num_tris
        } else {
            // SAFETY: `indices` has space for `num_indices_u32` triangle indices.
            unsafe { crate::native::topology::triangulate_ngon(&nc, indices, num_indices_u32) }
        }
    }
}

// C: `ufbx_abi ufbxi_noinline` (ufbx.c:32392).
#[cfg(not(feature = "triangulation"))]
#[inline(never)]
pub(crate) unsafe fn catch_triangulate_face<M: Mode>(
    mut panic: Option<&mut Panic>,
    indices: *mut u32,
    num_indices: usize,
    mesh: &View<Mesh, M>,
    face: Face,
) -> u32 {
    // C: `indices`/`num_indices`/`mesh`/`face` are unreferenced in the `#else`
    // arm.
    let _ = (indices, num_indices, mesh, face);
    crate::native::error::panicf_imp(
        panic.take(),
        crate::native::error::FailStr::new(b"Triangulation disabled\0"),
        &[],
    );
    0
}

// ufbx.c:32477-32482 `ufbx_catch_compute_topology`
pub(crate) unsafe fn catch_compute_topology<M: Mode>(
    mut panic: Option<&mut Panic>,
    mesh: &View<Mesh, M>,
    indices: *mut TopoEdge,
    num_indices: usize,
) {
    if ufbxi_panicf!(
        panic,
        num_indices >= mesh.num_indices(),
        "Required mesh.num_indices (%zu) indices, got %zu",
        mesh.num_indices(),
        num_indices,
    ) {
        return;
    }

    // SAFETY: `indices` has `num_indices >= mesh.num_indices` `TopoEdge` slots
    // (guarded above) for `compute_topology` to fill.
    unsafe { crate::native::topology::compute_topology(mesh, indices) };
}

// ufbx.c:32484-32492 `ufbx_catch_topo_next_vertex_edge`
pub(crate) unsafe fn catch_topo_next_vertex_edge(
    mut panic: Option<&mut Panic>,
    topo: *const TopoEdge,
    num_topo: usize,
    index: u32,
) -> u32 {
    if index == NO_INDEX {
        return NO_INDEX;
    }
    if ufbxi_panicf!(
        panic,
        (index as usize) < num_topo,
        "index (%u) out of bounds (%zu)",
        index,
        num_topo,
    ) {
        return NO_INDEX;
    }
    // SAFETY: `index < num_topo` (guarded above), so `topo.add(index)` addresses
    // a live `TopoEdge` in the caller's array; reading its own `twin` field.
    let twin: u32 = unsafe { (*topo.add(index as usize)).twin };
    if twin == NO_INDEX {
        return NO_INDEX;
    }
    if ufbxi_panicf!(
        panic,
        (twin as usize) < num_topo,
        "Corrupted topology structure"
    ) {
        return NO_INDEX;
    }
    // SAFETY: `twin < num_topo` (guarded above), so `topo.add(twin)` addresses a
    // live `TopoEdge`; reading its own `next` field.
    unsafe { (*topo.add(twin as usize)).next }
}

// ufbx.c:32494-32499 `ufbx_catch_topo_prev_vertex_edge`
pub(crate) unsafe fn catch_topo_prev_vertex_edge(
    mut panic: Option<&mut Panic>,
    topo: *const TopoEdge,
    num_topo: usize,
    index: u32,
) -> u32 {
    if index == NO_INDEX {
        return NO_INDEX;
    }
    if ufbxi_panicf!(
        panic,
        (index as usize) < num_topo,
        "index (%u) out of bounds (%zu)",
        index,
        num_topo,
    ) {
        return NO_INDEX;
    }
    // C: `topo[topo[index].prev].twin`.
    // SAFETY: `index < num_topo` (guarded above), so `topo.add(index)` addresses
    // a live `TopoEdge`; its own `prev` field indexes another live `TopoEdge`
    // whose own `twin` field is read.
    unsafe { (*topo.add((*topo.add(index as usize)).prev as usize)).twin }
}

// Mode-generic read accessors over the public vertex-attribute structs
// (`ufbx_vertex_*`): the get_vertex_* family serves safe Rust callers
// (`&VertexVec3` -> `Const`) and internal `Mut` users alike. Names are
// distinct from subdivision.rs's Mut-only `*_view` lenses on the same types.
macro_rules! vertex_attrib_views {
    ($($ty:ty => $elem:ty),* $(,)?) => {$(
        impl<M: Mode> View<$ty, M> {
            #[inline(always)]
            pub(crate) fn indices_count(&self) -> usize {
                // SAFETY: reading the `indices.count` field of a valid attrib.
                unsafe { (*self.as_ptr()).indices.count }
            }
            #[inline(always)]
            pub(crate) fn indices_data(&self) -> *const u32 {
                // SAFETY: reading the `indices.data` run pointer (stored value).
                unsafe { (*self.as_ptr()).indices.data }
            }
            #[inline(always)]
            pub(crate) fn values_count(&self) -> usize {
                // SAFETY: reading the `values.count` field of a valid attrib.
                unsafe { (*self.as_ptr()).values.count }
            }
            #[inline(always)]
            pub(crate) fn values_data(&self) -> *const $elem {
                // SAFETY: reading the `values.data` run pointer (stored value).
                unsafe { (*self.as_ptr()).values.data }
            }
        }
    )*};
}
vertex_attrib_views! {
    crate::generated::VertexReal => Real,
    crate::generated::VertexVec2 => Vec2,
    VertexVec3 => Vec3,
    crate::generated::VertexVec4 => Vec4,
}

impl<M: Mode> View<VertexVec3, M> {
    #[inline(always)]
    pub(crate) fn values_w_count(&self) -> usize {
        // SAFETY: reading the `values_w.count` field of a valid attrib.
        unsafe { (*self.as_ptr()).values_w.count }
    }
    #[inline(always)]
    pub(crate) fn values_w_data(&self) -> *const Real {
        // SAFETY: reading the `values_w.data` run pointer (stored value).
        unsafe { (*self.as_ptr()).values_w.data }
    }
}

// ufbx.h:5763 `ufbx_get_vertex_real` (an `ufbx_inline`, not `ufbx_abi`, so no
// shim); same `(int32_t)` index cast as `ufbx_get_vertex_vec3` below.
#[inline(always)]
pub(crate) fn get_vertex_real<M: Mode>(
    v: &View<crate::generated::VertexReal, M>,
    index: usize,
) -> Real {
    ufbx_assert!(index < v.indices_count());
    // SAFETY: `index < indices.count` (always-on assert above); the stored
    // index is a valid values index or NO_INDEX (== -1 as i32), which reads
    // the zero element ufbx guarantees immediately BEFORE `values.data` —
    // both in the attrib's arena allocation (scene-construction invariant;
    // attribs are not constructible from safe code).
    unsafe {
        *v.values_data()
            .offset(*v.indices_data().add(index) as i32 as isize)
    }
}

// ufbx.h:5765 `ufbx_get_vertex_vec3` (an `ufbx_inline`, not `ufbx_abi`, so no
// shim): `v->values.data[(int32_t)v->indices.data[index]]`. The `(int32_t)`
// cast is C-parity — a >= 0x80000000 index sign-extends into a negative offset.
#[inline(always)]
pub(crate) fn get_vertex_vec3<M: Mode>(v: &View<VertexVec3, M>, index: usize) -> Vec3 {
    ufbx_assert!(index < v.indices_count());
    // SAFETY: same argument as `get_vertex_real` — checked index, and the
    // NO_INDEX read lands on the guaranteed zero element before `values.data`.
    unsafe {
        *v.values_data()
            .offset(*v.indices_data().add(index) as i32 as isize)
    }
}

// ufbx.c:32501-32532 `ufbx_catch_get_weighted_face_normal`
// C: `ufbx_abi ufbxi_noinline` (ufbx.c:32501).
#[inline(never)]
pub(crate) fn catch_get_weighted_face_normal<M: Mode>(
    mut panic: Option<&mut Panic>,
    positions: &View<VertexVec3, M>,
    face: Face,
) -> Vec3 {
    if ufbxi_panicf!(
        panic,
        face.index_begin as usize <= positions.indices_count(),
        "Face index begin (%u) out of bounds (%zu)",
        face.index_begin,
        positions.indices_count(),
    ) {
        return ZERO_VEC3;
    }
    if ufbxi_panicf!(
        panic,
        positions
            .indices_count()
            .wrapping_sub(face.index_begin as usize)
            >= face.num_indices as usize,
        "Face index end (%u + %u) out of bounds (%zu)",
        face.index_begin,
        face.num_indices,
        positions.indices_count(),
    ) {
        return ZERO_VEC3;
    }

    if face.num_indices < 3 {
        ZERO_VEC3
    } else if face.num_indices == 3 {
        let a: Vec3 = get_vertex_vec3(positions, face.index_begin as usize);
        let b: Vec3 = get_vertex_vec3(positions, face.index_begin.wrapping_add(1) as usize);
        let c: Vec3 = get_vertex_vec3(positions, face.index_begin.wrapping_add(2) as usize);
        cross3(sub3(b, a), sub3(c, a))
    } else if face.num_indices == 4 {
        let a: Vec3 = get_vertex_vec3(positions, face.index_begin as usize);
        let b: Vec3 = get_vertex_vec3(positions, face.index_begin.wrapping_add(1) as usize);
        let c: Vec3 = get_vertex_vec3(positions, face.index_begin.wrapping_add(2) as usize);
        let d: Vec3 = get_vertex_vec3(positions, face.index_begin.wrapping_add(3) as usize);
        cross3(sub3(c, a), sub3(d, b))
    } else {
        // Newell's Method
        let mut result: Vec3 = ZERO_VEC3;
        for i in 0..face.num_indices as usize {
            let next: usize = if i + 1 < face.num_indices as usize {
                i + 1
            } else {
                0
            };
            let a: Vec3 = get_vertex_vec3(positions, (face.index_begin as usize).wrapping_add(i));
            let b: Vec3 =
                get_vertex_vec3(positions, (face.index_begin as usize).wrapping_add(next));
            result.x += (a.y - b.y) * (a.z + b.z);
            result.y += (a.z - b.z) * (a.x + b.x);
            result.z += (a.x - b.x) * (a.y + b.y);
        }
        result
    }
}

// ufbx.c:32534-32578 `ufbx_catch_generate_normal_mapping`
// C-parity: this one is declared WITHOUT `ufbx_abi` in ufbx.c (the `ufbx.h`
// declaration carries it) — no behavioral difference here.
pub(crate) unsafe fn catch_generate_normal_mapping<M: Mode>(
    mut panic: Option<&mut Panic>,
    mesh: &View<Mesh, M>,
    topo: *const TopoEdge,
    num_topo: usize,
    normal_indices: *mut u32,
    num_normal_indices: usize,
    assume_smooth: bool,
) -> usize {
    let mut next_index: u32 = 0;
    if ufbxi_panicf!(
        panic,
        num_normal_indices >= mesh.num_indices(),
        "Expected at least mesh.num_indices (%zu), got %zu",
        mesh.num_indices(),
        num_normal_indices,
    ) {
        return 0;
    }

    for i in 0..mesh.num_indices() {
        // SAFETY: `i < mesh.num_indices <= num_normal_indices` (guarded above),
        // so `normal_indices.add(i)` addresses a caller-reserved slot.
        unsafe { *normal_indices.add(i) = NO_INDEX };
    }

    // Walk around vertices and merge around smooth edges
    for vi in 0..mesh.num_vertices() {
        // SAFETY: `vi < mesh.num_vertices`, so `vertex_first_index.data.add(vi)`
        // addresses a live element of the mesh's own list.
        let original_start: u32 = unsafe { *mesh.vertex_first_index().data.add(vi) };
        if original_start == NO_INDEX {
            continue;
        }
        let mut start: u32 = original_start;
        let mut cur: u32 = start;

        loop {
            // SAFETY: `topo`/`num_topo` are this fn's raw-pointer contract,
            // forwarded unchanged to the topology walkers.
            let prev: u32 = unsafe { topo_next_vertex_edge(topo, num_topo, cur) };
            // SAFETY: same `topo`/`num_topo` raw-pointer contract, forwarded to
            // the smooth-edge predicate along with this fn's mesh view.
            if !unsafe { is_edge_smooth(mesh, topo, num_topo, cur, assume_smooth) } {
                start = cur;
            }
            if prev == NO_INDEX {
                start = cur;
                break;
            }
            if prev == original_start {
                break;
            }
            cur = prev;
        }

        // C: `normal_indices[start] = next_index++;`
        // SAFETY: `start` is an index within the mesh's index range, so
        // `normal_indices.add(start)` addresses a caller-reserved slot.
        unsafe { *normal_indices.add(start as usize) = next_index };
        next_index = next_index.wrapping_add(1);
        let mut next: u32 = start;
        loop {
            // SAFETY: `topo`/`num_topo` contract as above.
            next = unsafe { topo_prev_vertex_edge(topo, num_topo, next) };
            if next == NO_INDEX || next == start {
                break;
            }

            // SAFETY: same `topo`/`num_topo` raw-pointer contract, forwarded to
            // the smooth-edge predicate along with this fn's mesh view.
            if !unsafe { is_edge_smooth(mesh, topo, num_topo, next, assume_smooth) } {
                next_index = next_index.wrapping_add(1);
            }
            // SAFETY: `next` is an index within the mesh's index range.
            unsafe { *normal_indices.add(next as usize) = next_index.wrapping_sub(1) };
        }
    }

    // Assign non-manifold indices
    for i in 0..mesh.num_indices() {
        // SAFETY: `i < mesh.num_indices`, so `normal_indices.add(i)` addresses a
        // caller-reserved slot.
        if unsafe { *normal_indices.add(i) } == NO_INDEX {
            // C: `normal_indices[i] = next_index++;`
            // SAFETY: as above.
            unsafe { *normal_indices.add(i) = next_index };
            next_index = next_index.wrapping_add(1);
        }
    }

    next_index as usize
}

// ufbx.c:32580-32583 `ufbx_generate_normal_mapping`
pub(crate) unsafe fn generate_normal_mapping(
    mesh: *const Mesh,
    topo: *const TopoEdge,
    num_topo: usize,
    normal_indices: *mut u32,
    num_normal_indices: usize,
    assume_smooth: bool,
) -> usize {
    // SAFETY: `mesh` is this fn's raw-pointer param; `from_ptr` mints a `Const`
    // view over it, and `topo`/`normal_indices` are forwarded under the same
    // raw-pointer contract to `catch_generate_normal_mapping`.
    unsafe {
        catch_generate_normal_mapping(
            None,
            View::<Mesh, Const>::from_ptr(mesh),
            topo,
            num_topo,
            normal_indices,
            num_normal_indices,
            assume_smooth,
        )
    }
}

// ufbx.c:32585-32612 `ufbx_catch_compute_normals`
pub(crate) unsafe fn catch_compute_normals<M: Mode>(
    mut panic: Option<&mut Panic>,
    mesh: &View<Mesh, M>,
    positions: &View<VertexVec3, M>,
    normal_indices: *const u32,
    num_normal_indices: usize,
    normals: *mut Vec3,
    num_normals: usize,
) {
    if ufbxi_panicf!(
        panic,
        num_normal_indices >= mesh.num_indices(),
        "Expected at least mesh.num_indices (%zu), got %zu",
        mesh.num_indices(),
        num_normal_indices,
    ) {
        return;
    }

    // SAFETY: `normals` addresses `num_normals` caller-reserved `Vec3` slots;
    // the write zero-fills exactly that byte extent.
    unsafe {
        core::ptr::write_bytes(
            normals as *mut u8,
            0,
            size_of::<Vec3>().wrapping_mul(num_normals),
        );
    }

    for fi in 0..mesh.num_faces() {
        // SAFETY: `fi < mesh.num_faces`, so `faces.data.add(fi)` addresses a live
        // `Face` in the mesh's own list.
        let face: Face = unsafe { *mesh.faces().data.add(fi) };
        let normal: Vec3 = catch_get_weighted_face_normal(None, positions, face);
        for ix in 0..face.num_indices as usize {
            // SAFETY: the face's index range lies within `mesh.num_indices <=
            // num_normal_indices` (guarded above), so `normal_indices.add(..)`
            // addresses a caller-reserved slot.
            let index: u32 =
                unsafe { *normal_indices.add((face.index_begin as usize).wrapping_add(ix)) };

            if ufbxi_panicf!(
                panic,
                (index as usize) < num_normals,
                "Normal index (%u) out of bounds (%zu) at %zu",
                index,
                num_normals,
                ix,
            ) {
                return;
            }

            // SAFETY: `index < num_normals` (guarded just above), so
            // `normals.add(index)` addresses a caller-reserved `Vec3` slot.
            let n: *mut Vec3 = unsafe { normals.add(index as usize) };
            // SAFETY: `n` is the live `Vec3` slot resolved above.
            unsafe { *n = add3(*n, normal) };
        }
    }

    for i in 0..num_normals {
        // SAFETY: `i < num_normals`, so `normals.add(i)` addresses a
        // caller-reserved `Vec3` slot.
        let len: Real = unsafe { length3(*normals.add(i)) };
        if len > 0.0 {
            // SAFETY: as above.
            unsafe {
                (*normals.add(i)).x /= len;
                (*normals.add(i)).y /= len;
                (*normals.add(i)).z /= len;
            }
        }
    }
}

// ufbx.c:32614-32617 `ufbx_compute_normals`
pub(crate) unsafe fn compute_normals(
    mesh: *const Mesh,
    positions: *const VertexVec3,
    normal_indices: *const u32,
    num_normal_indices: usize,
    normals: *mut Vec3,
    num_normals: usize,
) {
    // SAFETY: `mesh`/`positions` are this fn's raw-pointer params minted into
    // `Const` views; `normal_indices`/`normals` are forwarded unchanged under
    // the same raw-pointer contract to `catch_compute_normals`.
    unsafe {
        catch_compute_normals(
            None,
            View::<Mesh, Const>::from_ptr(mesh),
            View::<VertexVec3, Const>::from_ptr(positions),
            normal_indices,
            num_normal_indices,
            normals,
            num_normals,
        );
    }
}

// ufbx.c:32619-32625 `ufbx_subdivide_mesh`
// The public function has no `#if`/`#else` fork — it always delegates to
// `ufbxi_subdivide_mesh` (native/subdivision.rs `subdivide_mesh`), which is the
// one that carries the `UFBXI_FEATURE_SUBDIVISION` split.
pub(crate) unsafe fn subdivide_mesh(
    mesh: *const Mesh,
    level: usize,
    opts: *const crate::generated::RawSubdivideOpts,
) -> Result<*mut Mesh, Error> {
    // SAFETY: `opts` is this fn's raw-pointer param; the macro reads its
    // `_begin_zero`/`_end_zero` guard fields only after a null check.
    unsafe { ufbxi_check_opts_res!(opts) };
    if mesh.is_null() {
        // C's silent NULL: no slot write — the shim clears the caller slot
        // only for an `Ok` with a payload that is not the input pointer.
        return Ok(core::ptr::null_mut());
    }
    if level == 0 {
        // C's silent passthrough: the INPUT mesh comes back with the caller's
        // slot untouched — the shim skips the success clear when the payload is
        // the input pointer.
        return Ok(mesh as *mut Mesh);
    }
    // SAFETY: `mesh`/`opts` are this fn's raw-pointer params, forwarded
    // unchanged under the same contract to the subdivision implementation.
    unsafe { crate::native::subdivision::subdivide_mesh(mesh, level, opts) }
}

// ufbx.c:32627-32636 `ufbx_free_mesh`
pub(crate) unsafe fn free_mesh(mesh: *mut Mesh) {
    if mesh.is_null() {
        return;
    }
    // `mesh` is a write-capable `*mut Mesh`: every caller reaches this fn
    // holding the mesh by raw pointer — `MeshRoot`'s stored payload pointer, or
    // the `extern "C"` `ufbx_free_mesh` shim — never through a `&Mesh`. So the
    // flag reads ride a `Mut` view, which matches that provenance and takes on
    // no frozen-tag obligation over an allocation the refcount path below can
    // deallocate.
    // SAFETY: `mesh` is non-null here and points at a live `Mesh` — the
    // raw-pointer contract of this `unsafe fn`; its provenance is the caller's
    // write-capable `*mut`, and no `&mut Mesh` is active while the view is used.
    let mesh_view: &View<Mesh, Mut> = unsafe { View::<Mesh, Mut>::from_ptr(mesh) };
    if !mesh_view.subdivision_evaluated() && !mesh_view.from_tessellated_nurbs() {
        return;
    }

    // SAFETY: the subdivided/tessellated `mesh` is the payload of a live
    // `MeshImp` handed out by this library — the raw-pointer contract of this
    // `unsafe fn`.
    let imp = unsafe { ImpHandle::<MeshImp>::from_payload(mesh) };
    ufbx_assert!(imp.has_magic());
    if !imp.has_magic() {
        return;
    }
    imp.release();
}

// ufbx.c:32638-32647 `ufbx_retain_mesh`
pub(crate) unsafe fn retain_mesh(mesh: *mut Mesh) {
    if mesh.is_null() {
        return;
    }
    // `mesh` is a write-capable `*mut Mesh`: every caller reaches this fn
    // holding the mesh by raw pointer — `MeshRoot`'s stored payload pointer, or
    // the `extern "C"` `ufbx_retain_mesh` shim — never through a `&Mesh`. So the
    // flag reads ride a `Mut` view, which matches that provenance and takes on
    // no frozen-tag obligation over an allocation the refcount path below can
    // deallocate.
    // SAFETY: `mesh` is non-null here and points at a live `Mesh` — the
    // raw-pointer contract of this `unsafe fn`; its provenance is the caller's
    // write-capable `*mut`, and no `&mut Mesh` is active while the view is used.
    let mesh_view: &View<Mesh, Mut> = unsafe { View::<Mesh, Mut>::from_ptr(mesh) };
    if !mesh_view.subdivision_evaluated() && !mesh_view.from_tessellated_nurbs() {
        return;
    }

    // SAFETY: the subdivided/tessellated `mesh` is the payload of a live
    // `MeshImp` handed out by this library — the raw-pointer contract of this
    // `unsafe fn`.
    let imp = unsafe { ImpHandle::<MeshImp>::from_payload(mesh) };
    ufbx_assert!(imp.has_magic());
    if !imp.has_magic() {
        return;
    }
    imp.retain();
}

// ufbx.c:32649-32655 `ufbx_load_geometry_cache`
pub(crate) unsafe fn load_geometry_cache(
    filename: *const u8,
    opts: *const RawGeometryCacheOpts,
) -> Result<*mut GeometryCache, Error> {
    // SAFETY: `filename` is this fn's raw-pointer param (a NUL-terminated C
    // string per contract); `strlen` measures it and both are forwarded to
    // `load_geometry_cache_len` under the same contract.
    unsafe { load_geometry_cache_len(filename, crate::native::error::strlen(filename), opts) }
}

// ufbx.c:32657-32664 `ufbx_load_geometry_cache_len`
// Both entry points delegate unconditionally — there is no feature fork here;
// `ufbxi_load_geometry_cache` (`native::cache`) owns the
// `UFBXI_FEATURE_GEOMETRY_CACHE` split and its `#else` arm reports
// `UFBX_ERROR_FEATURE_DISABLED`.
pub(crate) unsafe fn load_geometry_cache_len(
    filename: *const u8,
    filename_len: usize,
    opts: *const RawGeometryCacheOpts,
) -> Result<*mut GeometryCache, Error> {
    // SAFETY: `opts` is this fn's raw-pointer param; the macro reads its
    // `_begin_zero`/`_end_zero` guard fields only after a null check.
    unsafe { ufbxi_check_opts_res!(opts) };
    let str_: String = safe_string(filename, filename_len);
    // SAFETY: `opts`/`error` are this fn's raw-pointer params, forwarded
    // unchanged under the same contract to the cache loader.
    unsafe { crate::native::cache::load_geometry_cache(str_, opts) }
}

// ufbx.c:32666-32675 `ufbx_free_geometry_cache`
pub(crate) unsafe fn free_geometry_cache(cache: *mut GeometryCache) {
    if cache.is_null() {
        return;
    }

    // SAFETY: the non-null `cache` is the payload of a live `GeometryCacheImp`
    // handed out by this library — the raw-pointer contract of this `unsafe fn`.
    let imp = unsafe { ImpHandle::<GeometryCacheImp>::from_payload(cache) };
    ufbx_assert!(imp.has_magic());
    if !imp.has_magic() {
        return;
    }
    // SAFETY: same live imp; reading its own `owned_by_scene` field (present in
    // both cfg arms of the struct).
    if unsafe { (*imp.as_ptr()).owned_by_scene } {
        return;
    }
    imp.release();
}

// ufbx.c:32677-32686 `ufbx_retain_geometry_cache`
pub(crate) unsafe fn retain_geometry_cache(cache: *mut GeometryCache) {
    if cache.is_null() {
        return;
    }

    // SAFETY: the non-null `cache` is the payload of a live `GeometryCacheImp`
    // handed out by this library — the raw-pointer contract of this `unsafe fn`.
    let imp = unsafe { ImpHandle::<GeometryCacheImp>::from_payload(cache) };
    ufbx_assert!(imp.has_magic());
    if !imp.has_magic() {
        return;
    }
    // SAFETY: same live imp; reading its own `owned_by_scene` field (present in
    // both cfg arms of the struct).
    if unsafe { (*imp.as_ptr()).owned_by_scene } {
        return;
    }
    imp.retain();
}

// ufbx.c:32688-32694 `ufbxi_geometry_cache_buffer` — the union overlays
// `double f64[512]` / `float f32[512]` (only ONE member is read per call, keyed
// by `use_double`) plus a separate `ufbx_real dst[512]` conversion target.
#[cfg(feature = "geometry-cache")]
const GEOMETRY_CACHE_BUFFER_SIZE: usize = 512; // ufbx.c:62 `UFBXI_GEOMETRY_CACHE_BUFFER_SIZE`

#[cfg(feature = "geometry-cache")]
#[repr(C)]
union GeometryCacheBufferSrc {
    f64_: [f64; GEOMETRY_CACHE_BUFFER_SIZE],
    f32_: [f32; GEOMETRY_CACHE_BUFFER_SIZE],
}

#[cfg(feature = "geometry-cache")]
#[repr(C)]
struct GeometryCacheBuffer {
    src: GeometryCacheBufferSrc,
    dst: [Real; GEOMETRY_CACHE_BUFFER_SIZE],
}

// ufbx.c:32696-32859 `ufbx_read_geometry_cache_real`
#[inline(never)]
pub(crate) unsafe fn read_geometry_cache_real(
    frame: *const CacheFrame,
    data: *mut Real,
    count: usize,
    user_opts: *const RawGeometryCacheDataOpts,
) -> usize {
    #[cfg(feature = "geometry-cache")]
    {
        // SAFETY: `user_opts` is this fn's raw-pointer param; the macro reads its
        // `_begin_zero`/`_end_zero` guard fields only after a null check.
        unsafe { ufbxi_check_opts_return_no_error!(0usize, user_opts) };
        if frame.is_null() || count == 0 {
            return 0;
        }
        ufbx_assert!(!data.is_null());
        if data.is_null() {
            return 0;
        }

        // C: `ufbx_geometry_cache_data_opts opts;` copied from `user_opts`, else
        // `memset(&opts, 0, sizeof(opts))`.
        let mut opts: RawGeometryCacheDataOpts = if !user_opts.is_null() {
            // SAFETY: `user_opts` is non-null here and points at a live opts
            // struct per this fn's contract; read by value.
            unsafe { core::ptr::read(user_opts) }
        } else {
            // SAFETY: an all-zero bit pattern is a valid `RawGeometryCacheDataOpts`.
            unsafe { core::mem::zeroed() }
        };

        if opts.open_file_cb.fn_.is_none() {
            opts.open_file_cb.fn_ = Some(default_open_file);
        }

        let mut use_double = false;

        // C: `size_t src_count = 0;` then the switch assigns every arm (the
        // `default: ufbxi_unreachable(...)` has no counterpart — the `#[repr]`
        // enum cannot hold an out-of-range value). `frame->data_count * 3` is
        // `uint32_t` arithmetic (wraps at 2^32) before widening to `size_t`.
        let mut src_count: usize;
        // SAFETY: `frame` is non-null here (checked above) and points at a live
        // `CacheFrame` per this fn's contract; reading its own `data_format`.
        match unsafe { (*frame).data_format } {
            CacheDataFormat::Unknown => src_count = 0,
            // SAFETY: same live `CacheFrame`; reading its own `data_count`.
            CacheDataFormat::RealFloat => src_count = unsafe { (*frame).data_count } as usize,
            // SAFETY: same live `CacheFrame`; reading its own `data_count`.
            CacheDataFormat::Vec3Float => {
                src_count = unsafe { (*frame).data_count }.wrapping_mul(3) as usize
            }
            CacheDataFormat::RealDouble => {
                // SAFETY: same live `CacheFrame`; reading its own `data_count`.
                src_count = unsafe { (*frame).data_count } as usize;
                use_double = true;
            }
            CacheDataFormat::Vec3Double => {
                // SAFETY: same live `CacheFrame`; reading its own `data_count`.
                src_count = unsafe { (*frame).data_count }.wrapping_mul(3) as usize;
                use_double = true;
            }
        }

        // C: `bool src_big_endian = false;` then the switch assigns / returns.
        let src_big_endian: bool;
        // SAFETY: same live `CacheFrame`; reading its own `data_encoding`.
        match unsafe { (*frame).data_encoding } {
            CacheDataEncoding::Unknown => return 0,
            CacheDataEncoding::LittleEndian => src_big_endian = false,
            CacheDataEncoding::BigEndian => src_big_endian = true,
        }

        // Test endianness
        let dst_big_endian: bool = {
            let val: u16 = 0xbbaa;
            let buf = val.to_ne_bytes();
            buf[0] == 0xbb
        };

        if src_count == 0 {
            return 0;
        }
        src_count = min_sz(src_count, count);

        // C: `ufbx_stream stream = { 0 };`
        let mut stream: RawStream = RawStream::default();
        // SAFETY: `frame` is a live `CacheFrame`; `filename.data`/`.length` are
        // its own blob fields; `open_file` receives borrows of local `opts`/
        // `stream` plus that filename under its documented contract.
        if !unsafe {
            crate::native::read::open_file(
                &raw const opts.open_file_cb,
                &raw mut stream,
                (*frame).filename.data,
                (*frame).filename.length,
                core::ptr::null(),
                core::ptr::null_mut(),
                OpenFileType::GeometryCache,
            )
        } {
            return 0;
        }

        // Skip to the correct point in the file
        // SAFETY: same live `CacheFrame`; reading its own `data_offset`.
        let mut offset: u64 = unsafe { (*frame).data_offset };
        if stream.skip_fn.is_some() {
            while offset > 0 {
                let to_skip = min64(offset, MAX_SKIP_SIZE as u64) as usize;
                // SAFETY: `skip_fn` is `Some` (checked above); calling the
                // stream's own skip callback with its `user` pointer.
                if !unsafe { (stream.skip_fn.unwrap_unchecked())(stream.user, to_skip) } {
                    break;
                }
                offset -= to_skip as u64;
            }
        } else {
            let mut buffer = [0u8; 4096]; // ufbxi_uninit
            while offset > 0 {
                let to_skip = min64(offset, buffer.len() as u64) as usize;
                // SAFETY: an open `RawStream` from `open_file` has a `read_fn`;
                // calling it with its `user` pointer and a `to_skip`-byte buffer.
                let num_read = unsafe {
                    (stream.read_fn.unwrap_unchecked())(
                        stream.user,
                        buffer.as_mut_ptr() as *mut c_void,
                        to_skip,
                    )
                };
                if num_read != to_skip {
                    break;
                }
                offset -= to_skip as u64;
            }
        }

        // Failed to skip all the way
        if offset > 0 {
            if let Some(close_fn) = stream.close_fn {
                // SAFETY: calling the stream's own close callback with its `user`.
                unsafe { close_fn(stream.user) };
            }
            return 0;
        }

        let mut dst: *mut Real = data;
        // SAFETY: `frame` is a live `CacheFrame`; reading its own `mirror_axis`.
        let mut mirror_ix: usize = (unsafe { (*frame).mirror_axis } as usize).wrapping_sub(1);
        // C: `ufbxi_geometry_cache_buffer buffer; // ufbxi_uninit` — zero-filled
        // here (Rust forbids `assume_init` on the float array); each element is
        // overwritten before it is read within `0..num_read`, so this is
        // behavior-identical to the C uninitialized buffer.
        // SAFETY: an all-zero bit pattern is a valid `GeometryCacheBuffer` (a
        // float-array union plus a `Real` array).
        let mut buffer: GeometryCacheBuffer = unsafe { core::mem::zeroed() };
        while src_count > 0 {
            let to_read = min_sz(src_count, GEOMETRY_CACHE_BUFFER_SIZE);
            src_count -= to_read;
            let num_read: usize;
            if use_double {
                // SAFETY: the stream's `read_fn` fills the `f64_` union arm (a
                // `[f64; 512]`) with up to `to_read` doubles via its `user` ptr.
                let mut bytes_read = unsafe {
                    (stream.read_fn.unwrap_unchecked())(
                        stream.user,
                        buffer.src.f64_.as_mut_ptr() as *mut c_void,
                        to_read * size_of::<f64>(),
                    )
                };
                if bytes_read == usize::MAX {
                    bytes_read = 0;
                }
                num_read = bytes_read / size_of::<f64>();
                if src_big_endian != dst_big_endian {
                    // SAFETY: `f64_` is the union arm just written; viewing its
                    // bytes for the in-place endian swap.
                    let p = unsafe { buffer.src.f64_.as_mut_ptr() } as *mut u8;
                    for i in 0..num_read {
                        // SAFETY: `i < num_read`, so the 8 bytes at `p.add(i*8)`
                        // lie within the written portion of the 512-double array.
                        unsafe {
                            let v = p.add(i * 8);
                            let t = *v.add(0);
                            *v.add(0) = *v.add(7);
                            *v.add(7) = t;
                            let t = *v.add(1);
                            *v.add(1) = *v.add(6);
                            *v.add(6) = t;
                            let t = *v.add(2);
                            *v.add(2) = *v.add(5);
                            *v.add(5) = t;
                            let t = *v.add(3);
                            *v.add(3) = *v.add(4);
                            *v.add(4) = t;
                        }
                    }
                }
                for i in 0..num_read {
                    // SAFETY: `f64_` is the union arm written above; `i < num_read`.
                    buffer.dst[i] = unsafe { buffer.src.f64_[i] } as Real;
                }
            } else {
                // SAFETY: the stream's `read_fn` fills the `f32_` union arm (a
                // `[f32; 512]`) with up to `to_read` floats via its `user` ptr.
                let mut bytes_read = unsafe {
                    (stream.read_fn.unwrap_unchecked())(
                        stream.user,
                        buffer.src.f32_.as_mut_ptr() as *mut c_void,
                        to_read * size_of::<f32>(),
                    )
                };
                if bytes_read == usize::MAX {
                    bytes_read = 0;
                }
                num_read = bytes_read / size_of::<f32>();
                if src_big_endian != dst_big_endian {
                    // SAFETY: `f32_` is the union arm just written; viewing its
                    // bytes for the in-place endian swap.
                    let p = unsafe { buffer.src.f32_.as_mut_ptr() } as *mut u8;
                    for i in 0..num_read {
                        // SAFETY: `i < num_read`, so the 4 bytes at `p.add(i*4)`
                        // lie within the written portion of the 512-float array.
                        unsafe {
                            let v = p.add(i * 4);
                            let t = *v.add(0);
                            *v.add(0) = *v.add(3);
                            *v.add(3) = t;
                            let t = *v.add(1);
                            *v.add(1) = *v.add(2);
                            *v.add(2) = t;
                        }
                    }
                }
                for i in 0..num_read {
                    // SAFETY: `f32_` is the union arm written above; `i < num_read`.
                    buffer.dst[i] = unsafe { buffer.src.f32_[i] } as Real;
                }
            }

            if !opts.ignore_transform {
                // SAFETY: `frame` is a live `CacheFrame`; reading its own
                // `scale_factor`.
                let scale: Real = unsafe { (*frame).scale_factor };
                if scale != 1.0 {
                    for i in 0..num_read {
                        buffer.dst[i] *= scale;
                    }
                }
                // SAFETY: same live `CacheFrame`; reading its own `mirror_axis`.
                if unsafe { (*frame).mirror_axis } as u32 != 0 {
                    while mirror_ix < num_read {
                        buffer.dst[mirror_ix] = -buffer.dst[mirror_ix];
                        mirror_ix = mirror_ix.wrapping_add(3);
                    }
                    mirror_ix = mirror_ix.wrapping_sub(num_read);
                }
            }

            if !dst.is_null() {
                let weight: Real = if opts.use_weight { opts.weight } else { 1.0 };
                if opts.additive {
                    for i in 0..num_read {
                        // SAFETY: `dst` addresses caller storage with room for the
                        // total written count; `i < num_read` of this batch.
                        unsafe { *dst.add(i) += buffer.dst[i] * weight };
                    }
                } else {
                    for i in 0..num_read {
                        // SAFETY: as above.
                        unsafe { *dst.add(i) = buffer.dst[i] * weight };
                    }
                }
                // SAFETY: advancing `dst` by the batch's written count stays
                // within the caller's `count`-element storage.
                dst = unsafe { dst.add(num_read) };
            }

            if num_read != to_read {
                break;
            }
        }

        if let Some(close_fn) = stream.close_fn {
            // SAFETY: calling the stream's own close callback with its `user`.
            unsafe { close_fn(stream.user) };
        }

        // SAFETY: `dst` and `data` are both derived from the same `data`
        // allocation (`dst` advanced within it), so the distance is well-defined.
        to_size(unsafe { dst.offset_from(data) })
    }
    #[cfg(not(feature = "geometry-cache"))]
    {
        let _ = (frame, data, count, user_opts);
        0
    }
}

// ufbx.c:32861-32931 `ufbx_sample_geometry_cache_real`
#[inline(never)]
pub(crate) unsafe fn sample_geometry_cache_real(
    channel: *const CacheChannel,
    time: f64,
    data: *mut Real,
    count: usize,
    user_opts: *const RawGeometryCacheDataOpts,
) -> usize {
    #[cfg(feature = "geometry-cache")]
    {
        // SAFETY: `user_opts` is this fn's raw-pointer param; the macro reads its
        // `_begin_zero`/`_end_zero` guard fields only after a null check.
        unsafe { ufbxi_check_opts_return_no_error!(0usize, user_opts) };
        if channel.is_null() || count == 0 {
            return 0;
        }
        ufbx_assert!(!data.is_null());
        if data.is_null() {
            return 0;
        }
        // SAFETY: `channel` is non-null here (checked above) and points at a live
        // `CacheChannel` per this fn's contract; reading its own `frames.count`.
        if unsafe { (*channel).frames.count } == 0 {
            return 0;
        }

        let mut opts: RawGeometryCacheDataOpts = if !user_opts.is_null() {
            // SAFETY: `user_opts` is non-null here; read by value.
            unsafe { core::ptr::read(user_opts) }
        } else {
            // SAFETY: an all-zero bit pattern is a valid `RawGeometryCacheDataOpts`.
            unsafe { core::mem::zeroed() }
        };

        let mut begin: usize = 0;
        // SAFETY: same live `CacheChannel`; reading its own `frames.count`.
        let mut end: usize = unsafe { (*channel).frames.count };
        // SAFETY: same live `CacheChannel`; reading its own `frames.data`.
        let frames: *const CacheFrame = unsafe { (*channel).frames.data };
        while end - begin >= 8 {
            let mid = (begin + end) >> 1;
            // SAFETY: `mid < end <= frames.count`, so `frames.add(mid)` addresses
            // a live `CacheFrame`; reading its own `time` field.
            if unsafe { (*frames.add(mid)).time } < time {
                begin = mid + 1;
            } else {
                end = mid;
            }
        }

        let eps: f64 = 0.00000001;

        // SAFETY: same live `CacheChannel`; reading its own `frames.count`.
        end = unsafe { (*channel).frames.count };
        while begin < end {
            // SAFETY: `begin < end <= frames.count`, so `frames.add(begin)`
            // addresses a live `CacheFrame`.
            let next: *const CacheFrame = unsafe { frames.add(begin) };
            // SAFETY: `next` is a live `CacheFrame`; reading its own `time`.
            if unsafe { (*next).time } < time {
                begin += 1;
                continue;
            }

            // First keyframe
            if begin == 0 {
                // SAFETY: `next` is a live `CacheFrame`; `data` is caller storage.
                return unsafe { read_geometry_cache_real(next, data, count, &opts) };
            }

            // SAFETY: `begin >= 1`, so `next.sub(1)` addresses the prior live
            // `CacheFrame` in the channel's array.
            let prev: *const CacheFrame = unsafe { next.sub(1) };

            // Snap to exact frames if near
            // SAFETY: `next` is a live `CacheFrame`; reading its own `time`.
            if math::fabs(unsafe { (*next).time } - time) < eps {
                // SAFETY: `next` is a live `CacheFrame`; `data` is caller storage.
                return unsafe { read_geometry_cache_real(next, data, count, &opts) };
            }
            // SAFETY: `prev` is a live `CacheFrame`; reading its own `time`.
            if math::fabs(unsafe { (*prev).time } - time) < eps {
                // SAFETY: `prev` is a live `CacheFrame`; `data` is caller storage.
                return unsafe { read_geometry_cache_real(prev, data, count, &opts) };
            }

            // SAFETY: `next`/`prev` are live `CacheFrame`s; reading own `time`.
            let rcp_delta: f64 = 1.0 / (unsafe { (*next).time } - unsafe { (*prev).time });
            // SAFETY: `prev` is a live `CacheFrame`; reading its own `time`.
            let t: f64 = (time - unsafe { (*prev).time }) * rcp_delta;

            let original_weight: Real = if opts.use_weight { opts.weight } else { 1.0 };

            opts.use_weight = true;
            opts.weight = (original_weight as f64 * (1.0 - t)) as Real;
            // SAFETY: `prev` is a live `CacheFrame`; `data` is caller storage.
            let num_prev = unsafe { read_geometry_cache_real(prev, data, count, &opts) };

            opts.additive = true;
            opts.weight = (original_weight as f64 * t) as Real;
            // SAFETY: `next` is a live `CacheFrame`; `data` is caller storage.
            return unsafe { read_geometry_cache_real(next, data, num_prev, &opts) };
        }

        // Last frame
        // SAFETY: `end == frames.count >= 1` here, so `frames.add(end - 1)`
        // addresses the last live `CacheFrame`.
        let last: *const CacheFrame = unsafe { frames.add(end - 1) };
        // SAFETY: `last` is a live `CacheFrame`; `data` is caller storage.
        unsafe { read_geometry_cache_real(last, data, count, &opts) }
    }
    #[cfg(not(feature = "geometry-cache"))]
    {
        let _ = (channel, time, data, count, user_opts);
        0
    }
}

// ufbx.c:32933-32943 `ufbx_read_geometry_cache_vec3`
#[inline(never)]
pub(crate) unsafe fn read_geometry_cache_vec3(
    frame: *const CacheFrame,
    data: *mut Vec3,
    count: usize,
    opts: *const RawGeometryCacheDataOpts,
) -> usize {
    #[cfg(feature = "geometry-cache")]
    {
        if frame.is_null() || count == 0 {
            return 0;
        }
        ufbx_assert!(!data.is_null());
        if data.is_null() {
            return 0;
        }
        // SAFETY: `frame`/`data`/`opts` are this fn's raw-pointer params,
        // forwarded unchanged (`data` reinterpreted as `Real`, 3 per `Vec3`).
        (unsafe { read_geometry_cache_real(frame, data as *mut Real, count.wrapping_mul(3), opts) })
            / 3
    }
    #[cfg(not(feature = "geometry-cache"))]
    {
        let _ = (frame, data, count, opts);
        0
    }
}

// ufbx.c:32945-32955 `ufbx_sample_geometry_cache_vec3`
#[inline(never)]
pub(crate) unsafe fn sample_geometry_cache_vec3(
    channel: *const CacheChannel,
    time: f64,
    data: *mut Vec3,
    count: usize,
    opts: *const RawGeometryCacheDataOpts,
) -> usize {
    #[cfg(feature = "geometry-cache")]
    {
        if channel.is_null() || count == 0 {
            return 0;
        }
        ufbx_assert!(!data.is_null());
        if data.is_null() {
            return 0;
        }
        // SAFETY: `channel`/`data`/`opts` are this fn's raw-pointer params,
        // forwarded unchanged (`data` reinterpreted as `Real`, 3 per `Vec3`).
        (unsafe {
            sample_geometry_cache_real(
                channel,
                time,
                data as *mut Real,
                count.wrapping_mul(3),
                opts,
            )
        }) / 3
    }
    #[cfg(not(feature = "geometry-cache"))]
    {
        let _ = (channel, time, data, count, opts);
        0
    }
}

// Mode-generic views over the public DOM types. The dom_* API is reachable
// from safe Rust (`&DomNode` -> read-only provenance -> `Const`) and from the
// C ABI (raw pointers, minted `Const` in the capi shims); nothing mutates a
// retained DOM, but the impls stay `M: Mode`-generic for uniformity with the
// find family.
impl<M: Mode> View<DomNode, M> {
    #[inline(always)]
    pub(crate) fn children_data(&self) -> *mut *mut DomNode {
        // SAFETY: reading the `children.data` run pointer (stored value; the
        // `RefList` payload is a flat array of `ufbx_dom_node*`).
        unsafe { (*self.as_ptr()).children.data as *mut *mut DomNode }
    }
    #[inline(always)]
    pub(crate) fn children_count(&self) -> usize {
        // SAFETY: reading the `children.count` field of a valid arena `DomNode`.
        unsafe { (*self.as_ptr()).children.count }
    }
    #[inline(always)]
    pub(crate) fn values_count(&self) -> usize {
        // SAFETY: reading the `values.count` field of a valid arena `DomNode`.
        unsafe { (*self.as_ptr()).values.count }
    }
    /// First entry of `values`, viewed with the same lifetime and mode as
    /// `self`; `None` when the node has no values.
    #[inline(always)]
    pub(crate) fn first_value(&self) -> Option<&View<DomValue, M>> {
        if self.values_count() == 0 {
            return None;
        }
        // SAFETY: `values.data` points at `values.count >= 1` arena `DomValue`s
        // (scene-construction invariant); the STORED pointer carries the
        // arena's write provenance — adequate for either mode.
        unsafe {
            Some(View::<DomValue, M>::mint(
                (*self.as_ptr()).values.data as *mut DomValue,
            ))
        }
    }
}

impl<M: Mode> View<DomValue, M> {
    #[inline(always)]
    pub(crate) fn type_(&self) -> DomValueType {
        view_read_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn value_int(&self) -> i64 {
        view_read_shared!(self, value_int)
    }
    #[inline(always)]
    pub(crate) fn value_blob(&self) -> Blob {
        view_read_shared!(self, value_blob)
    }
}

// ufbx.c:32957-32964 `ufbx_dom_find_len`
pub(crate) unsafe fn dom_find_len<'a, M: Mode>(
    parent: &'a View<DomNode, M>,
    name: &[u8],
) -> Option<&'a View<DomNode, M>> {
    // C: `ufbxi_for_ptr_list(ufbx_dom_node, p_child, parent->children)` — the
    // `RefList` payload is a flat array of `ufbx_dom_node*`.
    let mut p_child: *mut *mut DomNode = parent.children_data();
    // SAFETY: `children_data()`/`children_count()` are the node's own list base
    // and length, so `.add(count)` yields the one-past-end pointer.
    let p_child_end: *mut *mut DomNode = unsafe { p_child.add(parent.children_count()) };
    while p_child != p_child_end {
        // Mode-generic mint from the STORED child pointer (arena write
        // provenance), correlated to `parent`'s borrow.
        // SAFETY: `p_child` is in `[data, end)` of the child pointer list, so it
        // addresses a live `*mut DomNode` pointer.
        let child: &View<DomNode, M> = unsafe { View::<DomNode, M>::mint(*p_child) };
        if str_equal(child.name_view().bytes(), name) {
            return Some(child);
        }
        // SAFETY: `p_child` is before `p_child_end`, so stepping one element
        // stays within the child pointer list (up to the one-past-end bound).
        p_child = unsafe { p_child.add(1) };
    }
    None
}

// ufbx.c:32966-32974 `ufbx_generate_indices` — delegates to
// `ufbxi_generate_indices` (`// -- Utility`, `native/index_gen.rs`), which
// carries the `UFBXI_FEATURE_INDEX_GENERATION` fork itself.
pub(crate) unsafe fn generate_indices(
    streams: *const RawVertexStream,
    num_streams: usize,
    indices: *mut u32,
    num_indices: usize,
    allocator: *const RawAllocatorOpts,
) -> Result<usize, Error> {
    // C substitutes a zero-filled local slot when the caller passes NULL; the
    // `Result` shape always works in a local, carried by `Err` on failure (the
    // shim owns the caller-slot writes). The core's working error target stays
    // an always-present slot, exactly as in C.
    let mut error: Error = Error::default();
    // SAFETY: `streams`/`indices`/`allocator` are this fn's raw-pointer params,
    // forwarded unchanged with `&raw mut error` — this frame's live `Error` —
    // to the implementation.
    let result = unsafe {
        crate::native::index_gen::generate_indices(
            streams,
            num_streams,
            indices,
            num_indices,
            allocator,
            &raw mut error,
        )
    };
    if error.type_ != ErrorType::None {
        // C returns the core's count here too, but the failure count is always
        // 0 (`result_vertices` is only assigned on the success arm).
        Err(error)
    } else {
        Ok(result)
    }
}

// ufbx.c:32976-32979 `ufbx_thread_pool_run_task` — delegates to
// `ufbxi_thread_pool_execute` (`// -- Threading`, `native/thread.rs`).
pub(crate) unsafe fn thread_pool_run_task(ctx: ThreadPoolContext, index: u32) {
    // SAFETY: `ctx` is this fn's opaque handle over a live `ThreadPool` per its
    // contract, forwarded to `thread_pool_execute`.
    unsafe { crate::native::thread::thread_pool_execute(ctx as *mut ThreadPool, index) };
}

// ufbx.c:32981-32985 `ufbx_thread_pool_set_user_ptr`
pub(crate) unsafe fn thread_pool_set_user_ptr(ctx: ThreadPoolContext, user: *mut c_void) {
    let pool: *mut ThreadPool = ctx as *mut ThreadPool;
    // SAFETY: `pool` is the live `ThreadPool` behind `ctx`; writing its own
    // `user_ptr` field.
    unsafe { (*pool).user_ptr = user };
}

// ufbx.c:32987-32991 `ufbx_thread_pool_get_user_ptr`
pub(crate) unsafe fn thread_pool_get_user_ptr(ctx: ThreadPoolContext) -> *mut c_void {
    let pool: *mut ThreadPool = ctx as *mut ThreadPool;
    // SAFETY: `pool` is the live `ThreadPool` behind `ctx`; reading its own
    // `user_ptr` field.
    unsafe { (*pool).user_ptr }
}

// ufbx.c:32993-32999 `ufbx_catch_get_vertex_real`
#[inline(never)]
pub(crate) fn catch_get_vertex_real<M: Mode>(
    mut panic: Option<&mut Panic>,
    v: &View<VertexReal, M>,
    index: usize,
) -> Real {
    if ufbxi_panicf!(
        panic,
        index < v.indices_count(),
        "index (%zu) out of range (%zu)",
        index,
        v.indices_count(),
    ) {
        return 0.0;
    }
    // SAFETY: `index < indices.count` just checked; `indices.data` is the
    // attrib's stored arena run.
    let ix: u32 = unsafe { *v.indices_data().add(index) };
    if ufbxi_panicf!(
        panic,
        (ix as usize) < v.values_count() || ix == NO_INDEX,
        "Corrupted or missing vertex attribute (%u) at %zu",
        ix,
        index,
    ) {
        return 0.0;
    }
    // SAFETY: `ix < values.count` (and `ix < 2^31`, so the `as i32` cast is
    // value-preserving — counts this large cannot occur in a loaded scene), or
    // `ix == NO_INDEX` (== -1 as i32) which reads the zero element ufbx guarantees
    // immediately BEFORE `values.data` (same arena allocation).
    unsafe { *v.values_data().offset(ix as i32 as isize) }
}

// ufbx.c:33001-33007 `ufbx_catch_get_vertex_vec2`
#[inline(never)]
pub(crate) fn catch_get_vertex_vec2<M: Mode>(
    mut panic: Option<&mut Panic>,
    v: &View<VertexVec2, M>,
    index: usize,
) -> Vec2 {
    if ufbxi_panicf!(
        panic,
        index < v.indices_count(),
        "index (%zu) out of range (%zu)",
        index,
        v.indices_count(),
    ) {
        return ZERO_VEC2;
    }
    // SAFETY: `index < indices.count` just checked; `indices.data` is the
    // attrib's stored arena run.
    let ix: u32 = unsafe { *v.indices_data().add(index) };
    if ufbxi_panicf!(
        panic,
        (ix as usize) < v.values_count() || ix == NO_INDEX,
        "Corrupted or missing vertex attribute (%u) at %zu",
        ix,
        index,
    ) {
        return ZERO_VEC2;
    }
    // SAFETY: `ix < values.count` (and `ix < 2^31`, so the `as i32` cast is
    // value-preserving — counts this large cannot occur in a loaded scene), or
    // `ix == NO_INDEX` (== -1 as i32) which reads the zero element ufbx guarantees
    // immediately BEFORE `values.data` (same arena allocation).
    unsafe { *v.values_data().offset(ix as i32 as isize) }
}

// ufbx.c:33009-33015 `ufbx_catch_get_vertex_vec3`
#[inline(never)]
pub(crate) fn catch_get_vertex_vec3<M: Mode>(
    mut panic: Option<&mut Panic>,
    v: &View<VertexVec3, M>,
    index: usize,
) -> Vec3 {
    if ufbxi_panicf!(
        panic,
        index < v.indices_count(),
        "index (%zu) out of range (%zu)",
        index,
        v.indices_count(),
    ) {
        return ZERO_VEC3;
    }
    // SAFETY: `index < indices.count` just checked; `indices.data` is the
    // attrib's stored arena run.
    let ix: u32 = unsafe { *v.indices_data().add(index) };
    if ufbxi_panicf!(
        panic,
        (ix as usize) < v.values_count() || ix == NO_INDEX,
        "Corrupted or missing vertex attribute (%u) at %zu",
        ix,
        index,
    ) {
        return ZERO_VEC3;
    }
    // SAFETY: `ix < values.count` (and `ix < 2^31`, so the `as i32` cast is
    // value-preserving — counts this large cannot occur in a loaded scene), or
    // `ix == NO_INDEX` (== -1 as i32) which reads the zero element ufbx guarantees
    // immediately BEFORE `values.data` (same arena allocation).
    unsafe { *v.values_data().offset(ix as i32 as isize) }
}

// ufbx.c:33017-33023 `ufbx_catch_get_vertex_vec4`
#[inline(never)]
pub(crate) fn catch_get_vertex_vec4<M: Mode>(
    mut panic: Option<&mut Panic>,
    v: &View<VertexVec4, M>,
    index: usize,
) -> Vec4 {
    if ufbxi_panicf!(
        panic,
        index < v.indices_count(),
        "index (%zu) out of range (%zu)",
        index,
        v.indices_count(),
    ) {
        return ZERO_VEC4;
    }
    // SAFETY: `index < indices.count` just checked; `indices.data` is the
    // attrib's stored arena run.
    let ix: u32 = unsafe { *v.indices_data().add(index) };
    if ufbxi_panicf!(
        panic,
        (ix as usize) < v.values_count() || ix == NO_INDEX,
        "Corrupted or missing vertex attribute (%u) at %zu",
        ix,
        index,
    ) {
        return ZERO_VEC4;
    }
    // SAFETY: `ix < values.count` (and `ix < 2^31`, so the `as i32` cast is
    // value-preserving — counts this large cannot occur in a loaded scene), or
    // `ix == NO_INDEX` (== -1 as i32) which reads the zero element ufbx guarantees
    // immediately BEFORE `values.data` (same arena allocation).
    unsafe { *v.values_data().offset(ix as i32 as isize) }
}

// ufbx.c:33025-33032 `ufbx_catch_get_vertex_w_vec3`
pub(crate) fn catch_get_vertex_w_vec3<M: Mode>(
    mut panic: Option<&mut Panic>,
    v: &View<VertexVec3, M>,
    index: usize,
) -> Real {
    if ufbxi_panicf!(
        panic,
        index < v.indices_count(),
        "index (%zu) out of range (%zu)",
        index,
        v.indices_count(),
    ) {
        return 0.0;
    }
    if v.values_w_count() == 0 {
        return 0.0;
    }
    // SAFETY: `index < indices.count` just checked; stored arena run.
    let ix: u32 = unsafe { *v.indices_data().add(index) };
    if ufbxi_panicf!(
        panic,
        (ix as usize) < v.values_count() || ix == NO_INDEX,
        "Corrupted or missing vertex attribute (%u) at %zu",
        ix,
        index,
    ) {
        return 0.0;
    }
    // SAFETY: `values_w` mirrors `values` (same counts and sentinel; C indexes
    // it with the same checked/NO_INDEX offset).
    unsafe { *v.values_w_data().offset(ix as i32 as isize) }
}

// ufbx.c:33034-33075 `ufbx_as_*` — each returns `element` reinterpreted iff its
// `type` matches, else NULL. Non-null guard AND type test, in that order.
// ufbx.c:33034 `ufbx_as_unknown`
pub(crate) unsafe fn as_unknown(element: *const Element) -> *mut Unknown {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Unknown {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Unknown` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33035 `ufbx_as_node`
pub(crate) unsafe fn as_node(element: *const Element) -> *mut Node {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Node {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Node` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33036 `ufbx_as_mesh`
pub(crate) unsafe fn as_mesh(element: *const Element) -> *mut Mesh {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Mesh {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Mesh` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33037 `ufbx_as_light`
pub(crate) unsafe fn as_light(element: *const Element) -> *mut Light {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Light {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Light` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33038 `ufbx_as_camera`
pub(crate) unsafe fn as_camera(element: *const Element) -> *mut Camera {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Camera {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Camera` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33039 `ufbx_as_bone`
pub(crate) unsafe fn as_bone(element: *const Element) -> *mut Bone {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Bone {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Bone` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33040 `ufbx_as_empty`
pub(crate) unsafe fn as_empty(element: *const Element) -> *mut Empty {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Empty {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Empty` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33041 `ufbx_as_line_curve`
pub(crate) unsafe fn as_line_curve(element: *const Element) -> *mut LineCurve {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::LineCurve {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `LineCurve` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33042 `ufbx_as_nurbs_curve`
pub(crate) unsafe fn as_nurbs_curve(element: *const Element) -> *mut NurbsCurve {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::NurbsCurve {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `NurbsCurve` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33043 `ufbx_as_nurbs_surface`
pub(crate) unsafe fn as_nurbs_surface(element: *const Element) -> *mut NurbsSurface {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::NurbsSurface {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `NurbsSurface` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33044 `ufbx_as_nurbs_trim_surface`
pub(crate) unsafe fn as_nurbs_trim_surface(element: *const Element) -> *mut NurbsTrimSurface {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::NurbsTrimSurface {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `NurbsTrimSurface` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33045 `ufbx_as_nurbs_trim_boundary`
pub(crate) unsafe fn as_nurbs_trim_boundary(element: *const Element) -> *mut NurbsTrimBoundary {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::NurbsTrimBoundary {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `NurbsTrimBoundary` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33046 `ufbx_as_procedural_geometry`
pub(crate) unsafe fn as_procedural_geometry(element: *const Element) -> *mut ProceduralGeometry {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::ProceduralGeometry {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `ProceduralGeometry` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33047 `ufbx_as_stereo_camera`
pub(crate) unsafe fn as_stereo_camera(element: *const Element) -> *mut StereoCamera {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::StereoCamera {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `StereoCamera` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33048 `ufbx_as_camera_switcher`
pub(crate) unsafe fn as_camera_switcher(element: *const Element) -> *mut CameraSwitcher {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::CameraSwitcher {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `CameraSwitcher` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33049 `ufbx_as_marker`
pub(crate) unsafe fn as_marker(element: *const Element) -> *mut Marker {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Marker {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Marker` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33050 `ufbx_as_lod_group`
pub(crate) unsafe fn as_lod_group(element: *const Element) -> *mut LodGroup {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::LodGroup {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `LodGroup` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33051 `ufbx_as_skin_deformer`
pub(crate) unsafe fn as_skin_deformer(element: *const Element) -> *mut SkinDeformer {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::SkinDeformer {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `SkinDeformer` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33052 `ufbx_as_skin_cluster`
pub(crate) unsafe fn as_skin_cluster(element: *const Element) -> *mut SkinCluster {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::SkinCluster {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `SkinCluster` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33053 `ufbx_as_blend_deformer`
pub(crate) unsafe fn as_blend_deformer(element: *const Element) -> *mut BlendDeformer {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::BlendDeformer {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `BlendDeformer` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33054 `ufbx_as_blend_channel`
pub(crate) unsafe fn as_blend_channel(element: *const Element) -> *mut BlendChannel {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::BlendChannel {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `BlendChannel` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33055 `ufbx_as_blend_shape`
pub(crate) unsafe fn as_blend_shape(element: *const Element) -> *mut BlendShape {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::BlendShape {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `BlendShape` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33056 `ufbx_as_cache_deformer`
pub(crate) unsafe fn as_cache_deformer(element: *const Element) -> *mut CacheDeformer {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::CacheDeformer {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `CacheDeformer` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33057 `ufbx_as_cache_file`
pub(crate) unsafe fn as_cache_file(element: *const Element) -> *mut CacheFile {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::CacheFile {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `CacheFile` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33058 `ufbx_as_material`
pub(crate) unsafe fn as_material(element: *const Element) -> *mut Material {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Material {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Material` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33059 `ufbx_as_texture`
pub(crate) unsafe fn as_texture(element: *const Element) -> *mut Texture {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Texture {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Texture` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33060 `ufbx_as_video`
pub(crate) unsafe fn as_video(element: *const Element) -> *mut Video {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Video {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Video` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33061 `ufbx_as_shader`
pub(crate) unsafe fn as_shader(element: *const Element) -> *mut Shader {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Shader {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Shader` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33062 `ufbx_as_shader_binding`
pub(crate) unsafe fn as_shader_binding(element: *const Element) -> *mut ShaderBinding {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::ShaderBinding {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `ShaderBinding` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33063 `ufbx_as_anim_stack`
pub(crate) unsafe fn as_anim_stack(element: *const Element) -> *mut AnimStack {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::AnimStack {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `AnimStack` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33064 `ufbx_as_anim_layer`
pub(crate) unsafe fn as_anim_layer(element: *const Element) -> *mut AnimLayer {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::AnimLayer {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `AnimLayer` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33065 `ufbx_as_anim_value`
pub(crate) unsafe fn as_anim_value(element: *const Element) -> *mut AnimValue {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::AnimValue {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `AnimValue` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33066 `ufbx_as_anim_curve`
pub(crate) unsafe fn as_anim_curve(element: *const Element) -> *mut AnimCurve {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::AnimCurve {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `AnimCurve` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33067 `ufbx_as_display_layer`
pub(crate) unsafe fn as_display_layer(element: *const Element) -> *mut DisplayLayer {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::DisplayLayer {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `DisplayLayer` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33068 `ufbx_as_selection_set`
pub(crate) unsafe fn as_selection_set(element: *const Element) -> *mut SelectionSet {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::SelectionSet {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `SelectionSet` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33069 `ufbx_as_selection_node`
pub(crate) unsafe fn as_selection_node(element: *const Element) -> *mut SelectionNode {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::SelectionNode {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `SelectionNode` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33070 `ufbx_as_character`
pub(crate) unsafe fn as_character(element: *const Element) -> *mut Character {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Character {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Character` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33071 `ufbx_as_constraint`
pub(crate) unsafe fn as_constraint(element: *const Element) -> *mut Constraint {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Constraint {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Constraint` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33072 `ufbx_as_audio_layer`
pub(crate) unsafe fn as_audio_layer(element: *const Element) -> *mut AudioLayer {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::AudioLayer {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `AudioLayer` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33073 `ufbx_as_audio_clip`
pub(crate) unsafe fn as_audio_clip(element: *const Element) -> *mut AudioClip {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::AudioClip {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `AudioClip` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33074 `ufbx_as_pose`
pub(crate) unsafe fn as_pose(element: *const Element) -> *mut Pose {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::Pose {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Pose` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33075 `ufbx_as_metadata_object`
pub(crate) unsafe fn as_metadata_object(element: *const Element) -> *mut MetadataObject {
    // SAFETY: `element` is non-null here (checked left of `&&`) and points at a
    // live `Element` per this fn's contract; reading its own `type_` discriminant.
    if !element.is_null() && unsafe { (*element).type_ } == ElementType::MetadataObject {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `MetadataObject` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element.expose_provenance())
    } else {
        core::ptr::null_mut()
    }
}

// ufbx.c:33077-33081 `ufbx_dom_is_array`
pub(crate) fn dom_is_array<M: Mode>(node: Option<&View<DomNode, M>>) -> bool {
    // C: `if (!node || node->values.count != 1) return false;` — the null arm
    // is the `None` case.
    let Some(node) = node else { return false };
    if node.values_count() != 1 {
        return false;
    }
    // C: `ufbx_dom_value v = node->values.data[0];`
    let Some(v) = node.first_value() else {
        return false;
    };
    v.type_() as u32 >= DomValueType::ArrayI32 as u32
        && v.type_() as u32 <= DomValueType::ArrayBlob as u32
}
// ufbx.c:33082-33084 `ufbx_dom_array_size`
pub(crate) fn dom_array_size<M: Mode>(node: Option<&View<DomNode, M>>) -> usize {
    if dom_is_array(node) {
        // `dom_is_array` established `node` is `Some` with exactly one value.
        match node.and_then(View::first_value) {
            Some(v) => v.value_int() as usize,
            None => 0,
        }
    } else {
        0
    }
}
// ufbx.c:33085-33093 `ufbx_dom_as_int32_list`
pub(crate) fn dom_as_int32_list<M: Mode>(node: Option<&View<DomNode, M>>) -> List<i32> {
    // SAFETY: an all-zero `List` is valid (null data, zero count).
    let mut list: List<i32> = unsafe { MaybeUninit::zeroed().assume_init() };
    list.data = core::ptr::null();
    list.count = 0;
    if let Some(node) = node {
        if node.values_count() == 1 {
            if let Some(value) = node.first_value() {
                if value.type_() == DomValueType::ArrayI32 {
                    list.data = value.value_blob().data as *const i32;
                    list.count = value.value_blob().size / size_of::<i32>();
                }
            }
        }
    }
    list
}
// ufbx.c:33094-33102 `ufbx_dom_as_int64_list`
pub(crate) fn dom_as_int64_list<M: Mode>(node: Option<&View<DomNode, M>>) -> List<i64> {
    // SAFETY: an all-zero `List` is valid (null data, zero count).
    let mut list: List<i64> = unsafe { MaybeUninit::zeroed().assume_init() };
    list.data = core::ptr::null();
    list.count = 0;
    if let Some(node) = node {
        if node.values_count() == 1 {
            if let Some(value) = node.first_value() {
                if value.type_() == DomValueType::ArrayI64 {
                    list.data = value.value_blob().data as *const i64;
                    list.count = value.value_blob().size / size_of::<i64>();
                }
            }
        }
    }
    list
}
// ufbx.c:33103-33111 `ufbx_dom_as_float_list`
pub(crate) fn dom_as_float_list<M: Mode>(node: Option<&View<DomNode, M>>) -> List<f32> {
    // SAFETY: an all-zero `List` is valid (null data, zero count).
    let mut list: List<f32> = unsafe { MaybeUninit::zeroed().assume_init() };
    list.data = core::ptr::null();
    list.count = 0;
    if let Some(node) = node {
        if node.values_count() == 1 {
            if let Some(value) = node.first_value() {
                if value.type_() == DomValueType::ArrayF32 {
                    list.data = value.value_blob().data as *const f32;
                    list.count = value.value_blob().size / size_of::<f32>();
                }
            }
        }
    }
    list
}
// ufbx.c:33112-33120 `ufbx_dom_as_double_list`
pub(crate) fn dom_as_double_list<M: Mode>(node: Option<&View<DomNode, M>>) -> List<f64> {
    // SAFETY: an all-zero `List` is valid (null data, zero count).
    let mut list: List<f64> = unsafe { MaybeUninit::zeroed().assume_init() };
    list.data = core::ptr::null();
    list.count = 0;
    if let Some(node) = node {
        if node.values_count() == 1 {
            if let Some(value) = node.first_value() {
                if value.type_() == DomValueType::ArrayF64 {
                    list.data = value.value_blob().data as *const f64;
                    list.count = value.value_blob().size / size_of::<f64>();
                }
            }
        }
    }
    list
}
// ufbx.c:33121-33129 `ufbx_dom_as_real_list`
pub(crate) fn dom_as_real_list<M: Mode>(node: Option<&View<DomNode, M>>) -> List<Real> {
    // SAFETY: an all-zero `List` is valid (null data, zero count).
    let mut list: List<Real> = unsafe { MaybeUninit::zeroed().assume_init() };
    list.data = core::ptr::null();
    list.count = 0;
    // C: `sizeof(ufbx_real) == sizeof(double) ? ARRAY_F64 : ARRAY_F32`
    let want = if size_of::<Real>() == size_of::<f64>() {
        DomValueType::ArrayF64
    } else {
        DomValueType::ArrayF32
    };
    if let Some(node) = node {
        if node.values_count() == 1 {
            if let Some(value) = node.first_value() {
                if value.type_() == want {
                    list.data = value.value_blob().data as *const Real;
                    list.count = value.value_blob().size / size_of::<Real>();
                }
            }
        }
    }
    list
}
// ufbx.c:33130-33138 `ufbx_dom_as_blob_list`
pub(crate) fn dom_as_blob_list<M: Mode>(node: Option<&View<DomNode, M>>) -> List<Blob> {
    // SAFETY: an all-zero `List` is valid (null data, zero count).
    let mut list: List<Blob> = unsafe { MaybeUninit::zeroed().assume_init() };
    list.data = core::ptr::null();
    list.count = 0;
    if let Some(node) = node {
        if node.values_count() == 1 {
            if let Some(value) = node.first_value() {
                if value.type_() == DomValueType::ArrayBlob {
                    list.data = value.value_blob().data as *const Blob;
                    list.count = value.value_blob().size / size_of::<Blob>();
                }
            }
        }
    }
    list
}

// ufbx.c:33142 `ufbx_find_prop`
pub(crate) unsafe fn find_prop<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
) -> Option<&View<Prop, M>> {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run is exactly the slice minted
    // for the `_len` impl.
    unsafe { find_prop_len(props, crate::prelude::slice_from_ptr(name, strlen(name))) }
}

// ufbx.c:33143 `ufbx_find_real`
pub(crate) unsafe fn find_real<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def: Real,
) -> Real {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run is exactly the slice minted
    // for the `_len` impl.
    unsafe {
        find_real_len(
            props,
            crate::prelude::slice_from_ptr(name, strlen(name)),
            def,
        )
    }
}

// ufbx.c:33144 `ufbx_find_vec3`
pub(crate) unsafe fn find_vec3<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def: Vec3,
) -> Vec3 {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run is exactly the slice minted
    // for the `_len` impl.
    unsafe {
        find_vec3_len(
            props,
            crate::prelude::slice_from_ptr(name, strlen(name)),
            def,
        )
    }
}

// ufbx.c:33145 `ufbx_find_int`
pub(crate) unsafe fn find_int<M: Mode>(props: &View<Props, M>, name: *const u8, def: i64) -> i64 {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run is exactly the slice minted
    // for the `_len` impl.
    unsafe {
        find_int_len(
            props,
            crate::prelude::slice_from_ptr(name, strlen(name)),
            def,
        )
    }
}

// ufbx.c:33146 `ufbx_find_bool`
pub(crate) unsafe fn find_bool<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def: bool,
) -> bool {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run is exactly the slice minted
    // for the `_len` impl.
    unsafe {
        find_bool_len(
            props,
            crate::prelude::slice_from_ptr(name, strlen(name)),
            def,
        )
    }
}

// ufbx.c:33147 `ufbx_find_string`
pub(crate) unsafe fn find_string<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def: String,
) -> String {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run is exactly the slice minted
    // for the `_len` impl.
    unsafe {
        find_string_len(
            props,
            crate::prelude::slice_from_ptr(name, strlen(name)),
            def,
        )
    }
}

// ufbx.c:33148 `ufbx_find_blob`
pub(crate) unsafe fn find_blob<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def: Blob,
) -> Blob {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run is exactly the slice minted
    // for the `_len` impl.
    unsafe {
        find_blob_len(
            props,
            crate::prelude::slice_from_ptr(name, strlen(name)),
            def,
        )
    }
}

// ufbx.c:33149 `ufbx_find_prop_element`
pub(crate) unsafe fn find_prop_element(
    element: *const Element,
    name: *const u8,
    type_: ElementType,
) -> *mut Element {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param, so
    // `strlen` measures it and the measured run is the slice minted for the
    // `_len` impl; `element` is this fn's live `*const Element` (C dereferences
    // it unconditionally), minted as the read-only `Const` view.
    unsafe {
        find_prop_element_len(
            View::<Element, Const>::from_ptr(element),
            crate::prelude::slice_from_ptr(name, strlen(name)),
            type_,
        )
    }
}

// ufbx.c:33150 `ufbx_find_element`
pub(crate) unsafe fn find_element(
    scene: *const Scene,
    type_: ElementType,
    name: *const u8,
) -> *mut Element {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run (with `scene`) is exactly the
    // slice minted for the `_len` impl.
    unsafe {
        find_element_len(
            scene_const_view(scene),
            type_,
            crate::prelude::slice_from_ptr(name, strlen(name)),
        )
    }
}

// ufbx.c:33151 `ufbx_find_node`
pub(crate) unsafe fn find_node(scene: *const Scene, name: *const u8) -> *mut Node {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run (with `scene`) is exactly the
    // slice minted for the `_len` impl.
    unsafe {
        find_node_len(
            scene_const_view(scene),
            crate::prelude::slice_from_ptr(name, strlen(name)),
        )
    }
}

// ufbx.c:33152 `ufbx_find_anim_stack`
pub(crate) unsafe fn find_anim_stack(scene: *const Scene, name: *const u8) -> *mut AnimStack {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run (with `scene`) is exactly the
    // slice minted for the `_len` impl.
    unsafe {
        find_anim_stack_len(
            scene_const_view(scene),
            crate::prelude::slice_from_ptr(name, strlen(name)),
        )
    }
}

// ufbx.c:33153 `ufbx_find_material`
pub(crate) unsafe fn find_material(scene: *const Scene, name: *const u8) -> *mut Material {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run (with `scene`) is exactly the
    // slice minted for the `_len` impl.
    unsafe {
        find_material_len(
            scene_const_view(scene),
            crate::prelude::slice_from_ptr(name, strlen(name)),
        )
    }
}

// ufbx.c:33154 `ufbx_find_anim_prop`
pub(crate) unsafe fn find_anim_prop(
    layer: *const AnimLayer,
    element: *const Element,
    prop: *const u8,
) -> *mut AnimProp {
    // SAFETY: the caller's null-or-live `layer` contract becomes the `Const`
    // view mint; `prop` is this fn's NUL-terminated raw-pointer string param —
    // `strlen` measures it, and the measured run is the slice minted for the
    // `_len` impl.
    match unsafe {
        find_anim_prop_len(
            if layer.is_null() {
                None
            } else {
                Some(View::<AnimLayer, Const>::from_ptr(layer))
            },
            element,
            crate::prelude::slice_from_ptr(prop, strlen(prop)),
        )
    } {
        Some(found) => found.as_ptr() as *mut AnimProp,
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:33155 `ufbx_evaluate_prop`
pub(crate) unsafe fn evaluate_prop(
    anim: *const Anim,
    element: *const Element,
    name: *const u8,
    time: f64,
) -> Prop {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it and all forward (with `anim`/`element`) to `_len`.
    unsafe { evaluate_prop_len(anim, element, name, strlen(name), time) }
}

// ufbx.c:33156 `ufbx_evaluate_prop_flags`
pub(crate) unsafe fn evaluate_prop_flags(
    anim: *const Anim,
    element: *const Element,
    name: *const u8,
    time: f64,
    flags: u32,
) -> Prop {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it and all forward (with `anim`/`element`) to `_len`.
    unsafe { evaluate_prop_flags_len(anim, element, name, strlen(name), time, flags) }
}

// ufbx.c:33157 `ufbx_find_prop_texture`
pub(crate) unsafe fn find_prop_texture(material: *const Material, name: *const u8) -> *mut Texture {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run (with `material`) is exactly
    // the slice minted for the `_len` impl.
    unsafe { find_prop_texture_len(material, crate::prelude::slice_from_ptr(name, strlen(name))) }
}

// ufbx.c:33158 `ufbx_find_shader_prop`
pub(crate) unsafe fn find_shader_prop(shader: *const Shader, name: *const u8) -> String {
    // SAFETY: the caller's null-or-live `shader` contract becomes the `Const`
    // view mint; `name` is this fn's NUL-terminated raw-pointer string param —
    // `strlen` measures it, and the measured run is the slice minted for the
    // `_len` impl.
    unsafe {
        find_shader_prop_len(
            if shader.is_null() {
                None
            } else {
                Some(View::<Shader, Const>::from_ptr(shader))
            },
            crate::prelude::slice_from_ptr(name, strlen(name)),
        )
    }
}

// ufbx.c:33159 `ufbx_find_shader_prop_bindings`
pub(crate) unsafe fn find_shader_prop_bindings(
    shader: *const Shader,
    name: *const u8,
) -> List<ShaderPropBinding> {
    // SAFETY: the caller's null-or-live `shader` contract becomes the `Const`
    // view mint; `name` is this fn's NUL-terminated raw-pointer string param —
    // `strlen` measures it, and the measured run is the slice minted for the
    // `_len` impl.
    unsafe {
        find_shader_prop_bindings_len(
            if shader.is_null() {
                None
            } else {
                Some(View::<Shader, Const>::from_ptr(shader))
            },
            crate::prelude::slice_from_ptr(name, strlen(name)),
        )
    }
}

// ufbx.c:33160 `ufbx_find_shader_texture_input`
pub(crate) unsafe fn find_shader_texture_input(
    shader: *const ShaderTexture,
    name: *const u8,
) -> *mut ShaderTextureInput {
    // SAFETY: the caller's live `shader` contract becomes the `Const` view
    // mint; `name` is this fn's NUL-terminated raw-pointer string param —
    // `strlen` measures it, and the measured run is the slice minted for the
    // `_len` impl.
    match unsafe {
        find_shader_texture_input_len(
            View::<ShaderTexture, Const>::from_ptr(shader),
            crate::prelude::slice_from_ptr(name, strlen(name)),
        )
    } {
        Some(input) => input.as_ptr() as *mut ShaderTextureInput,
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:33161 `ufbx_dom_find`
pub(crate) unsafe fn dom_find<M: Mode>(
    parent: &View<DomNode, M>,
    name: *const u8,
) -> Option<&View<DomNode, M>> {
    // SAFETY: `name` is this fn's NUL-terminated raw-pointer string param;
    // `strlen` measures it, and the measured run is exactly the slice minted
    // for the `_len` impl.
    unsafe { dom_find_len(parent, crate::prelude::slice_from_ptr(name, strlen(name))) }
}

// -- Catch API (ufbx.c:33163-33179): the non-catch wrappers that call their
// `ufbx_catch_*` counterparts with `panic == NULL`. Each rides the same cfg
// as the catch impl it delegates to.

// ufbx.c:33165-33167 `ufbx_triangulate_face`
pub(crate) unsafe fn triangulate_face(
    indices: *mut u32,
    num_indices: usize,
    mesh: *const Mesh,
    face: Face,
) -> u32 {
    // SAFETY: `indices`/`mesh` are this fn's raw-pointer params; `mesh` is minted
    // into a `Const` view and `indices` forwarded under the same contract.
    unsafe {
        catch_triangulate_face(
            None,
            indices,
            num_indices,
            View::<Mesh, Const>::from_ptr(mesh),
            face,
        )
    }
}

// ufbx.c:33168-33170 `ufbx_compute_topology`
pub(crate) unsafe fn compute_topology(mesh: *const Mesh, topo: *mut TopoEdge, num_topo: usize) {
    // SAFETY: `mesh`/`topo` are this fn's raw-pointer params; `mesh` is minted
    // into a `Const` view and `topo` forwarded under the same contract.
    unsafe { catch_compute_topology(None, View::<Mesh, Const>::from_ptr(mesh), topo, num_topo) }
}

// ufbx.c:33171-33173 `ufbx_topo_next_vertex_edge`
pub(crate) unsafe fn topo_next_vertex_edge(
    topo: *const TopoEdge,
    num_topo: usize,
    index: u32,
) -> u32 {
    // SAFETY: `topo`/`num_topo` are this fn's raw-pointer contract, forwarded
    // unchanged to the catch impl.
    unsafe { catch_topo_next_vertex_edge(None, topo, num_topo, index) }
}

// ufbx.c:33174-33176 `ufbx_topo_prev_vertex_edge`
pub(crate) unsafe fn topo_prev_vertex_edge(
    topo: *const TopoEdge,
    num_topo: usize,
    index: u32,
) -> u32 {
    // SAFETY: `topo`/`num_topo` are this fn's raw-pointer contract, forwarded
    // unchanged to the catch impl.
    unsafe { catch_topo_prev_vertex_edge(None, topo, num_topo, index) }
}

// ufbx.c:33177-33179 `ufbx_get_weighted_face_normal`
pub(crate) unsafe fn get_weighted_face_normal(positions: *const VertexVec3, face: Face) -> Vec3 {
    // SAFETY: `positions` is this fn's raw-pointer param, minted into a `Const`
    // view for the catch impl.
    unsafe {
        catch_get_weighted_face_normal(None, View::<VertexVec3, Const>::from_ptr(positions), face)
    }
}

#[cfg(test)]
mod tests {
    // Test scaffolding builds dummy element/prop tables with `MaybeUninit::zeroed()`
    // and only reads the POD sort-key fields; the zeroed values are never observed
    // through their non-zeroable fields, so `invalid_value` is allowed for the tests.
    #![allow(invalid_value)]
    use super::*;
    use crate::generated::Error;
    use crate::generated::NameElement;
    use crate::generated::RawAllocatorOpts;
    use crate::native::allocator::{
        init_ator, ANIM_IMP_MAGIC, BAKED_ANIM_IMP_MAGIC, MESH_IMP_MAGIC,
    };
    use crate::native::buf::push_size;
    use crate::native::parse::MeshImp;
    use crate::prelude::Ref;
    use core::ffi::c_void;
    use core::mem::size_of;

    // Build a refcounted object the way the C setup code does: an allocator
    // feeding a result buffer, with the `ufbxi_refcount` header pushed into
    // that same buffer (the header-inside-own-buffer trick `release_ref` must
    // survive).
    unsafe fn make_imp(error: *mut Error, parent: *mut Refcount) -> *mut MeshImp {
        // SAFETY: an all-zero bit pattern is a valid `Allocator` for this test
        // (only POD fields are read before `init_ator` writes it).
        let mut ator = unsafe { core::mem::MaybeUninit::<Allocator>::zeroed().assume_init() };
        let opts = RawAllocatorOpts::default();
        // SAFETY: `error` is this fn's raw-pointer param; `ator`/`opts` are live
        // locals borrowed for initialization.
        unsafe { init_ator(error, &mut ator, &opts, c"test") };

        // SAFETY: an all-zero bit pattern is a valid `Buf` for this test.
        let mut buf = unsafe { core::mem::MaybeUninit::<Buf>::zeroed().assume_init() };
        buf.ator = &raw mut ator;

        // SAFETY: `buf` is a live local backed by `ator`; `push_size` allocates
        // `MeshImp`-sized storage from it.
        let imp = unsafe { push_size(&mut buf, size_of::<MeshImp>(), 1) } as *mut MeshImp;
        assert!(!imp.is_null());
        // SAFETY: `imp` is the non-null `MeshImp`-sized allocation just made; the
        // write zero-fills exactly its byte extent.
        unsafe { core::ptr::write_bytes(imp as *mut u8, 0, size_of::<MeshImp>()) };
        // Expose the wide allocation so `get_imp` can recover this header via
        // exposed provenance from a (possibly narrowed) public pointer.
        (imp as *mut u8).expose_provenance();
        // SAFETY: `imp` is a live zeroed `MeshImp`; the raw field address
        // identifies its own refcount header, initialized with `parent`.
        unsafe { init_ref(&raw mut (*imp).refcount, MESH_IMP_MAGIC, parent) };
        // SAFETY: same live `imp`; writing its own `magic` field.
        unsafe { (*imp).magic = MESH_IMP_MAGIC };

        // Transfer the allocator/buffer into the refcount header, as the C
        // setup paths do before returning the object to the user.
        // SAFETY: same live `imp`; writing its own `refcount.ator` field.
        unsafe { (*imp).refcount.ator = ator };
        // SAFETY: same live `imp`; writing its own `refcount.buf` field.
        unsafe { (*imp).refcount.buf = buf };
        imp
    }

    #[test]
    fn test_refcount_lifecycle_and_get_imp() {
        unsafe {
            let mut error = Error::default();
            let imp = make_imp(&mut error, core::ptr::null_mut());

            // Counter starts at 0 (no self-retain); retain makes the previous
            // value 1 so one release only decrements.
            assert_eq!(
                (*imp)
                    .refcount
                    .refcount
                    .load(core::sync::atomic::Ordering::SeqCst),
                0
            );
            retain_ref(&raw mut (*imp).refcount);

            let mesh_ptr = &raw mut (*imp).mesh;
            let back = ImpHandle::<MeshImp>::from_payload(mesh_ptr);
            assert_eq!(back.as_ptr(), imp);
            assert!(back.has_magic());

            release_ref(&raw mut (*imp).refcount);
            // Still alive: previous value was 1.
            assert_eq!((*imp).refcount.self_magic, REFCOUNT_IMP_MAGIC);

            // Final release frees the object (previous value 0). The
            // header-inside-own-buffer free order is exercised for real here;
            // miri/asan-style UAF would fire on a wrong port.
            release_ref(&raw mut (*imp).refcount);
        }
    }

    #[test]
    fn test_release_ref_walks_parent_chain() {
        unsafe {
            let mut error = Error::default();
            let parent = make_imp(&mut error, core::ptr::null_mut());
            // Child holds the only reference to the parent (init_ref retains).
            let child = make_imp(&mut error, &raw mut (*parent).refcount);
            assert_eq!(
                (*parent)
                    .refcount
                    .refcount
                    .load(core::sync::atomic::Ordering::SeqCst),
                1
            );

            // Releasing the child (count 0) frees it AND iteratively releases
            // the parent, whose count drops from 1 to 0 -> freed too.
            release_ref(&raw mut (*child).refcount);
        }
    }

    use crate::generated::RawCloseMemoryCb;

    #[test]
    fn test_open_memory_ctx_copy_and_close() {
        unsafe {
            let data = *b"hello, memory stream";
            let mut stream = RawStream::default();
            assert!(open_memory(
                &mut stream,
                data.as_ptr() as *const c_void,
                data.len(),
                core::ptr::null(),
            )
            .is_ok());

            // The stream owns a copy: reads survive the original going away.
            assert_eq!((stream.size_fn.unwrap())(stream.user), data.len() as u64);
            let mut buf = [0u8; 5];
            assert_eq!(
                (stream.read_fn.unwrap())(stream.user, buf.as_mut_ptr() as *mut c_void, 5),
                5
            );
            assert_eq!(&buf, b"hello");
            assert!((stream.skip_fn.unwrap())(stream.user, 7));
            assert_eq!(
                (stream.read_fn.unwrap())(stream.user, buf.as_mut_ptr() as *mut c_void, 5),
                5
            );
            assert_eq!(&buf, b"y str");
            // Reads clamp at the end of the memory block.
            assert_eq!(
                (stream.read_fn.unwrap())(stream.user, buf.as_mut_ptr() as *mut c_void, 5),
                3
            );
            assert!(!(stream.skip_fn.unwrap())(stream.user, 1));

            (stream.close_fn.unwrap())(stream.user);
        }
    }

    #[test]
    fn test_open_memory_no_copy_close_cb() {
        unsafe extern "C" fn close_cb(user: *mut c_void, data: *mut c_void, data_size: usize) {
            let hits = user as *mut (usize, usize);
            // SAFETY: `user` is the `&mut (usize, usize)` the test passes as the
            // callback's `user` pointer; writing its own tuple fields.
            unsafe {
                (*hits).0 = data as usize;
                (*hits).1 = data_size;
            }
        }

        unsafe {
            let data = *b"no-copy";
            let mut hits: (usize, usize) = (0, 0);
            let opts = RawOpenMemoryOpts {
                no_copy: true,
                close_cb: RawCloseMemoryCb {
                    fn_: Some(close_cb),
                    user: &mut hits as *mut (usize, usize) as *mut c_void,
                },
                ..Default::default()
            };
            let mut stream = RawStream::default();
            assert!(open_memory(
                &mut stream,
                data.as_ptr() as *const c_void,
                data.len(),
                &opts,
            )
            .is_ok());
            // no_copy: the stream reads the caller's bytes in place.
            let mem = stream.user as *mut MemoryStream;
            assert_eq!((*mem).data as usize, data.as_ptr() as usize);
            (stream.close_fn.unwrap())(stream.user);
            assert_eq!(hits, (data.as_ptr() as usize, data.len()));
        }
    }

    #[test]
    fn test_open_file_missing_reports_file_not_found() {
        unsafe {
            let mut stream = RawStream::default();
            let path = b"definitely/not/a/real/file.fbx";
            let error: Error =
                open_file(&mut stream, path.as_ptr(), path.len(), core::ptr::null()).unwrap_err();
            let desc =
                core::slice::from_raw_parts(error.description.data, error.description.length);
            assert_eq!(desc, b"File not found");
            assert_eq!(error.info(), core::str::from_utf8(path).unwrap());
        }
    }

    #[test]
    fn test_open_file_reads_real_file() {
        unsafe {
            let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
            let path = std::format!("{}/Cargo.toml", dir);
            let expected = std::fs::read(&path).unwrap();

            let mut stream = RawStream::default();
            assert!(open_file(&mut stream, path.as_ptr(), path.len(), core::ptr::null()).is_ok());
            assert_eq!(
                (stream.size_fn.unwrap())(stream.user),
                expected.len() as u64
            );

            let mut got = vec![0; expected.len()];
            let mut read_total = 0usize;
            while read_total < expected.len() {
                let n = (stream.read_fn.unwrap())(
                    stream.user,
                    got.as_mut_ptr().add(read_total) as *mut c_void,
                    expected.len() - read_total,
                );
                assert!(n != 0 && n != usize::MAX);
                read_total += n;
            }
            assert_eq!(got, expected);
            (stream.close_fn.unwrap())(stream.user);
        }
    }

    #[test]
    fn test_is_thread_safe() {
        // C: `return UFBXI_THREAD_SAFE != 0;` (1 on every target we build).
        assert!(is_thread_safe());
    }

    #[test]
    fn test_element_type_size_table() {
        // Same order and values as C's `sizeof(ufbx_*)` list; the layout
        // oracle (tests/layout.rs) pins these sizes against the C structs.
        assert_eq!(ELEMENT_TYPE_SIZE.len(), ELEMENT_TYPE_COUNT);
        assert_eq!(
            ELEMENT_TYPE_SIZE[ElementType::Unknown as usize],
            size_of::<crate::generated::Unknown>()
        );
        assert_eq!(
            ELEMENT_TYPE_SIZE[ElementType::Node as usize],
            size_of::<Node>()
        );
        assert_eq!(
            ELEMENT_TYPE_SIZE[ElementType::MetadataObject as usize],
            size_of::<crate::generated::MetadataObject>()
        );
        for size in ELEMENT_TYPE_SIZE.iter() {
            assert!(*size >= size_of::<Element>());
        }
    }

    #[test]
    fn test_axes_constants() {
        assert_eq!(AXES_RIGHT_HANDED_Y_UP.front, CoordinateAxis::PositiveZ);
        assert_eq!(AXES_RIGHT_HANDED_Z_UP.front, CoordinateAxis::NegativeY);
        assert_eq!(AXES_LEFT_HANDED_Y_UP.front, CoordinateAxis::NegativeZ);
        assert_eq!(AXES_LEFT_HANDED_Z_UP.front, CoordinateAxis::PositiveY);
        assert!(coordinate_axes_valid(AXES_RIGHT_HANDED_Y_UP));
        assert!(coordinate_axes_valid(AXES_RIGHT_HANDED_Z_UP));
        assert!(coordinate_axes_valid(AXES_LEFT_HANDED_Y_UP));
        assert!(coordinate_axes_valid(AXES_LEFT_HANDED_Z_UP));
    }

    // The lookup tables the `ufbx_find_*` entry points binary-search are built
    // sorted by the loader (`ufbxi_sort_name_elements` ufbx.c:18578,
    // `ufbxi_sort_anim_props` ufbx.c:19301); these tests reproduce that order
    // with `slice::sort_by` — test scaffolding, not ported code.
    fn zeroed_elements(count: usize) -> std::vec::Vec<Element> {
        let mut v: std::vec::Vec<Element> = std::vec::Vec::new();
        for _ in 0..count {
            v.push(unsafe { MaybeUninit::zeroed().assume_init() });
        }
        v
    }

    #[test]
    fn test_find_element_len_and_typed_lookups() {
        unsafe {
            // C: `if (!scene) return NULL;`
            assert!(
                find_element_len(scene_const_view(core::ptr::null()), ElementType::Node, b"a")
                    .is_null()
            );

            let names: [&[u8]; 4] = [b"alpha", b"beta", b"gamma", b"beta"];
            let types = [
                ElementType::Node,
                ElementType::Node,
                ElementType::Material,
                ElementType::AnimStack,
            ];
            let mut elements = zeroed_elements(4);
            let ptrs: std::vec::Vec<*mut Element> = (0..4).map(|i| &raw mut elements[i]).collect();

            let mut entries: std::vec::Vec<NameElement> = (0..4)
                .map(|i| NameElement {
                    name: String::new_c(names[i].as_ptr(), names[i].len()),
                    type_: types[i],
                    _internal_key: get_name_key(names[i]),
                    element: Ref::from_ptr(ptrs[i]),
                })
                .collect();
            entries.sort_by(|a, b| {
                (a._internal_key, names_of(a), a.type_ as u32).cmp(&(
                    b._internal_key,
                    names_of(b),
                    b.type_ as u32,
                ))
            });

            let mut scene: Scene = MaybeUninit::zeroed().assume_init();
            scene.elements_by_name = List::from_slice(&entries);

            let scene_view = scene_const_view(&raw const scene);
            let find = |t, n: &[u8]| find_element_len(scene_view, t, n);
            assert_eq!(find(ElementType::Node, b"alpha"), ptrs[0]);
            // Same name, different type: the type is part of the sort key.
            assert_eq!(find(ElementType::Node, b"beta"), ptrs[1]);
            assert_eq!(find(ElementType::AnimStack, b"beta"), ptrs[3]);
            assert_eq!(find(ElementType::Material, b"gamma"), ptrs[2]);
            assert!(find(ElementType::Mesh, b"beta").is_null());
            assert!(find(ElementType::Node, b"delta").is_null());

            // The typed wrappers just pin the element type.
            assert_eq!(find_node_len(scene_view, b"alpha"), ptrs[0] as *mut Node);
            assert_eq!(
                find_material_len(scene_view, b"gamma"),
                ptrs[2] as *mut Material
            );
            assert_eq!(
                find_anim_stack_len(scene_view, b"beta"),
                ptrs[3] as *mut AnimStack
            );
            assert!(find_node_len(scene_view, b"gamma").is_null());

            // String API wrappers: same results via `strlen`.
            assert_eq!(
                find_element(&scene, ElementType::Node, b"alpha\0".as_ptr()),
                ptrs[0]
            );
            assert_eq!(find_node(&scene, b"alpha\0".as_ptr()), ptrs[0] as *mut Node);
            assert_eq!(
                find_anim_stack(&scene, b"beta\0".as_ptr()),
                ptrs[3] as *mut AnimStack
            );
            assert_eq!(
                find_material(&scene, b"gamma\0".as_ptr()),
                ptrs[2] as *mut Material
            );
        }
    }

    fn names_of(e: &NameElement) -> &[u8] {
        unsafe { core::slice::from_raw_parts(e.name.data, e.name.length) }
    }

    #[test]
    fn test_find_anim_prop_len_and_find_anim_props() {
        unsafe {
            let mut elements = zeroed_elements(3);
            let e0: *mut Element = &raw mut elements[0];
            let e1: *mut Element = &raw mut elements[1];
            let e2: *mut Element = &raw mut elements[2];
            // The `ufbx_anim_prop` array is sorted by element ADDRESS, so pin
            // the ordering assumption the test data relies on.
            assert!((e0 as *const Element) < e1 && (e1 as *const Element) < e2);

            let mut av: crate::generated::AnimValue = MaybeUninit::zeroed().assume_init();
            let anim_value: *mut crate::generated::AnimValue = &raw mut av;

            let entries: [(*mut Element, &[u8]); 3] = [
                (e0, b"Lcl Scaling"),
                (e0, b"Lcl Translation"),
                (e2, b"Lcl Rotation"),
            ];
            let props: std::vec::Vec<AnimProp> = entries
                .iter()
                .map(|&(element, name)| AnimProp {
                    element: Ref::from_ptr(element),
                    _internal_key: get_name_key(name),
                    prop_name: String::new_c(name.as_ptr(), name.len()),
                    anim_value: Ref::from_ptr(anim_value),
                })
                .collect();

            let mut layer: AnimLayer = MaybeUninit::zeroed().assume_init();
            layer.anim_props = List::from_slice(&props);
            let base: *const AnimProp = props.as_ptr();

            // `Const` mint: the test list's `data` comes from a `&Vec` borrow
            // (SharedReadOnly), so a `Mut`-mode probe mint over it would be the
            // documented Stacked Borrows trap; reads only, no writes spanned.
            let layer_view = View::<AnimLayer, Const>::from_ptr(&raw const layer);
            let find = |e: *mut Element, n: &[u8]| {
                find_anim_prop_len(Some(layer_view), e, n).map_or(core::ptr::null(), |p| p.as_ptr())
            };
            assert_eq!(find(e0, b"Lcl Scaling"), base);
            assert_eq!(find(e0, b"Lcl Translation"), base.add(1));
            assert_eq!(find(e2, b"Lcl Rotation"), base.add(2));
            // Wrong element, or an element with no animated props at all.
            assert!(find(e2, b"Lcl Scaling").is_null());
            assert!(find(e1, b"Lcl Scaling").is_null());

            assert_eq!(
                find_anim_prop(&layer, e0, b"Lcl Scaling\0".as_ptr()),
                base as *mut AnimProp
            );

            // `ufbx_find_anim_props` returns the whole per-element run.
            let run = find_anim_props(Some(layer_view), e0);
            assert_eq!(run.data, base);
            assert_eq!(run.count, 2);
            let run = find_anim_props(Some(layer_view), e2);
            assert_eq!(run.data, base.add(2));
            assert_eq!(run.count, 1);
            // C: `begin == end` leaves the `{ 0 }` initializer untouched.
            let run = find_anim_props(Some(layer_view), e1);
            assert!(run.data.is_null());
            assert_eq!(run.count, 0);
        }
    }

    fn matrix_eq(a: &Matrix, b: &Matrix) -> bool {
        a.m00 == b.m00
            && a.m01 == b.m01
            && a.m02 == b.m02
            && a.m03 == b.m03
            && a.m10 == b.m10
            && a.m11 == b.m11
            && a.m12 == b.m12
            && a.m13 == b.m13
            && a.m20 == b.m20
            && a.m21 == b.m21
            && a.m22 == b.m22
            && a.m23 == b.m23
    }

    #[test]
    fn test_get_compatible_matrix_for_normals() {
        unsafe {
            // C: `if (!node) return ufbx_identity_matrix;`
            assert!(matrix_eq(
                &get_compatible_matrix_for_normals(core::ptr::null()),
                &IDENTITY_MATRIX
            ));

            let mut node: Node = MaybeUninit::zeroed().assume_init();
            node.node_to_world = IDENTITY_MATRIX;
            node.geometry_transform = IDENTITY_TRANSFORM;
            assert!(matrix_eq(
                &get_compatible_matrix_for_normals(&node),
                &IDENTITY_MATRIX
            ));

            // Only the geometry ROTATION is folded in — geometry translation
            // and scale are ignored by design (this is the "compatible" form).
            node.geometry_transform.translation = Vec3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            };
            node.geometry_transform.scale = Vec3 {
                x: 4.0,
                y: 4.0,
                z: 4.0,
            };
            assert!(matrix_eq(
                &get_compatible_matrix_for_normals(&node),
                &IDENTITY_MATRIX
            ));

            // A non-uniform node scale inverts-transposes into the normal
            // matrix: `ufbx_matrix_for_normals` of diag(2,4,8).
            node.node_to_world.m00 = 2.0;
            node.node_to_world.m11 = 4.0;
            node.node_to_world.m22 = 8.0;
            let m = get_compatible_matrix_for_normals(&node);
            let expected = matrix_for_normals(&node.node_to_world);
            assert!(matrix_eq(&m, &expected));
        }
    }

    unsafe fn fmt_error(dst: &mut [u8], error: *const Error) -> (usize, std::string::String) {
        // SAFETY: `dst` is a live caller buffer; `error` is this fn's raw-pointer
        // param, forwarded to `format_error` which writes into `dst`.
        let len = unsafe { format_error(dst.as_mut_ptr(), dst.len(), error) };
        let nul = dst.iter().position(|&b| b == 0).unwrap();
        (
            len,
            std::string::String::from_utf8(dst[..nul].to_vec()).unwrap(),
        )
    }

    #[test]
    fn test_format_error() {
        unsafe {
            let mut dst = [0xAAu8; 256];

            // No destination / no error: C writes only the NUL and returns 0.
            assert_eq!(
                format_error(core::ptr::null_mut(), 16, core::ptr::null()),
                0
            );
            assert_eq!(format_error(dst.as_mut_ptr(), 0, core::ptr::null()), 0);
            assert_eq!(
                format_error(dst.as_mut_ptr(), dst.len(), core::ptr::null()),
                0
            );
            assert_eq!(dst[0], 0);

            // Unset description falls back to "Unknown error".
            let mut error = Error::default();
            let (_, out) = fmt_error(&mut dst, &error);
            assert_eq!(out, "ufbx v0.23.0 error: Unknown error\n");

            error.description = String::new_c(b"Test error\0".as_ptr(), 10);
            let (len, out) = fmt_error(&mut dst, &error);
            assert_eq!(out, "ufbx v0.23.0 error: Test error\n");
            assert_eq!(len, out.len());

            // `info_length > 0` switches to the "%s (%.*s)" form.
            crate::native::error::set_err_info(&mut error, b"some info\0".as_ptr(), 9);
            let (_, out) = fmt_error(&mut dst, &error);
            assert_eq!(out, "ufbx v0.23.0 error: Test error (some info)\n");

            // Stack frames: "%*u:%s: %s\n" with a 6-wide right-aligned line.
            error.stack_size = 2;
            error.stack[0] = ErrorFrame {
                source_line: 1,
                function: String::new_c(b"ufbxi_first\0".as_ptr(), 11),
                description: String::new_c(b"First\0".as_ptr(), 5),
            };
            error.stack[1] = ErrorFrame {
                source_line: 1234567,
                function: String::new_c(b"ufbxi_second\0".as_ptr(), 12),
                description: String::new_c(b"Second\0".as_ptr(), 6),
            };
            let (_, out) = fmt_error(&mut dst, &error);
            assert_eq!(
                out,
                "ufbx v0.23.0 error: Test error (some info)\n\
                 \x20    1:ufbxi_first: First\n\
                 1234567:ufbxi_second: Second\n"
            );

            // Truncation: `offset` saturates at `dst_size - 1` and the buffer
            // stays NUL-terminated.
            let mut small = [0xAAu8; 16];
            let (len, out) = fmt_error(&mut small, &error);
            assert_eq!(len, 15);
            assert_eq!(out, "ufbx v0.23.0 er");
        }
    }

    #[test]
    fn test_free_scene_and_retain_scene_null_and_magic() {
        unsafe {
            // C: `if (!scene) return;` — both are no-ops on NULL.
            free_scene(core::ptr::null_mut());
            retain_scene(core::ptr::null_mut());

            // Wrong magic: C asserts then returns without touching the
            // refcount. Asserts are compiled in under `dev`/`regression`, so
            // only exercise the release path with the RIGHT magic here.
            let mut error = Error::default();
            let mut ator = core::mem::MaybeUninit::<Allocator>::zeroed().assume_init();
            let opts = RawAllocatorOpts::default();
            init_ator(&mut error, &mut ator, &opts, c"test");
            let mut buf = core::mem::MaybeUninit::<Buf>::zeroed().assume_init();
            buf.ator = &raw mut ator;
            let imp = push_size(&mut buf, size_of::<SceneImp>(), 1) as *mut SceneImp;
            assert!(!imp.is_null());
            core::ptr::write_bytes(imp as *mut u8, 0, size_of::<SceneImp>());
            // Expose the wide allocation so `get_imp` can recover this header via
            // exposed provenance from a (possibly narrowed) public pointer.
            (imp as *mut u8).expose_provenance();
            init_ref(
                &raw mut (*imp).refcount,
                SCENE_IMP_MAGIC,
                core::ptr::null_mut(),
            );
            (*imp).magic = SCENE_IMP_MAGIC;
            (*imp).refcount.ator = ator;
            (*imp).refcount.buf = buf;

            let scene: *mut Scene = &raw mut (*imp).scene;
            retain_scene(scene);
            assert_eq!(
                (*imp)
                    .refcount
                    .refcount
                    .load(core::sync::atomic::Ordering::SeqCst),
                1
            );
            free_scene(scene);
            assert_eq!((*imp).refcount.self_magic, REFCOUNT_IMP_MAGIC);
            // Final release: frees `string_buf` via `free_scene_imp`, then the
            // result buffer holding the header itself.
            free_scene(scene);
        }
    }

    // -- ufbx.c:31178-31721 (anim / baked-anim lifetime, baked lookups,
    //    `ufbx_evaluate_baked_*`, the quaternion math family)

    // Same shape as `make_imp` above, generalized over the `ufbxi_*_imp`
    // wrapper type. `T` must lead with `ufbxi_refcount` (all of them do — the
    // `ufbxi_get_imp` pointer trick depends on it).
    unsafe fn make_typed_imp<T>(error: *mut Error, magic: u32) -> *mut T {
        // SAFETY: an all-zero bit pattern is a valid `Allocator` for this test.
        let mut ator = unsafe { core::mem::MaybeUninit::<Allocator>::zeroed().assume_init() };
        let opts = RawAllocatorOpts::default();
        // SAFETY: `error` is this fn's raw-pointer param; `ator`/`opts` are live
        // locals borrowed for initialization.
        unsafe { init_ator(error, &mut ator, &opts, c"test") };

        // SAFETY: an all-zero bit pattern is a valid `Buf` for this test.
        let mut buf = unsafe { core::mem::MaybeUninit::<Buf>::zeroed().assume_init() };
        buf.ator = &raw mut ator;

        // SAFETY: `buf` is a live local backed by `ator`; `push_size` allocates
        // `T`-sized storage from it.
        let imp = unsafe { push_size(&mut buf, size_of::<T>(), 1) } as *mut T;
        assert!(!imp.is_null());
        // SAFETY: `imp` is the non-null `T`-sized allocation just made; the write
        // zero-fills exactly its byte extent.
        unsafe { core::ptr::write_bytes(imp as *mut u8, 0, size_of::<T>()) };
        let refcount = imp as *mut Refcount;
        // SAFETY: `T` leads with `ufbxi_refcount`, so `refcount` addresses the
        // zeroed header; initialized with a null parent.
        unsafe { init_ref(refcount, magic, core::ptr::null_mut()) };
        // SAFETY: `refcount` is the live header; writing its own `ator` field.
        unsafe { (*refcount).ator = ator };
        // SAFETY: same live header; writing its own `buf` field.
        unsafe { (*refcount).buf = buf };
        imp
    }

    unsafe fn refcount_of<T>(imp: *mut T) -> usize {
        // SAFETY: `T` leads with `ufbxi_refcount`, so `imp as *mut Refcount`
        // addresses a live header; reading its own atomic `refcount` field.
        unsafe {
            (*(imp as *mut Refcount))
                .refcount
                .load(core::sync::atomic::Ordering::SeqCst)
        }
    }

    #[test]
    fn test_free_and_retain_anim_custom_gate() {
        unsafe {
            use crate::native::scene_process::AnimImp;

            // C: `if (!anim) return;`
            free_anim(core::ptr::null_mut());
            retain_anim(core::ptr::null_mut());

            let mut error = Error::default();
            let imp: *mut AnimImp = make_typed_imp(&mut error, ANIM_IMP_MAGIC);
            (*imp).magic = ANIM_IMP_MAGIC;
            let anim: *mut Anim = &raw mut (*imp).anim;

            // C: `if (!anim->custom) return;` — a non-custom `ufbx_anim` is
            // owned by its scene, so BOTH entry points bail before touching
            // the refcount. This gate is the whole difference from the
            // scene/baked-anim pairs.
            (*anim).custom = false;
            retain_anim(anim);
            assert_eq!(refcount_of(imp), 0);
            free_anim(anim);
            assert_eq!(refcount_of(imp), 0);

            (*anim).custom = true;
            retain_anim(anim);
            assert_eq!(refcount_of(imp), 1);
            free_anim(anim);
            assert_eq!((*imp).refcount.self_magic, REFCOUNT_IMP_MAGIC);
            // Previous value 0 -> frees the buffer holding the header.
            free_anim(anim);
        }
    }

    #[test]
    fn test_retain_and_free_baked_anim() {
        unsafe {
            use crate::native::evaluate::BakedAnimImp;

            // C: `if (!bake) return;`
            retain_baked_anim(core::ptr::null_mut());
            free_baked_anim(core::ptr::null_mut());

            let mut error = Error::default();
            let imp: *mut BakedAnimImp = make_typed_imp(&mut error, BAKED_ANIM_IMP_MAGIC);
            (*imp).magic = BAKED_ANIM_IMP_MAGIC;
            let bake: *mut BakedAnim = &raw mut (*imp).bake;

            // `ufbxi_get_imp` round-trip: no `custom` gate on this pair.
            let back = ImpHandle::<BakedAnimImp>::from_payload(bake);
            assert_eq!(back.as_ptr(), imp);

            retain_baked_anim(bake);
            assert_eq!(refcount_of(imp), 1);
            free_baked_anim(bake);
            assert_eq!((*imp).refcount.self_magic, REFCOUNT_IMP_MAGIC);
            free_baked_anim(bake);
        }
    }

    #[test]
    fn test_find_baked_node_and_element() {
        unsafe {
            // `ufbx_baked_node_list` / `ufbx_baked_element_list` are built
            // sorted by `typed_id` / `element_id` by the baker.
            let mut nodes: std::vec::Vec<BakedNode> = std::vec::Vec::new();
            for id in [1u32, 3, 5] {
                let mut n: BakedNode = MaybeUninit::zeroed().assume_init();
                n.typed_id = id;
                n.element_id = id + 100;
                nodes.push(n);
            }
            let mut elements: std::vec::Vec<BakedElement> = std::vec::Vec::new();
            for id in [2u32, 4, 6] {
                let mut e: BakedElement = MaybeUninit::zeroed().assume_init();
                e.element_id = id;
                elements.push(e);
            }

            let mut bake: BakedAnim = MaybeUninit::zeroed().assume_init();
            bake.nodes = List::from_slice(&nodes);
            bake.elements = List::from_slice(&elements);
            let bake_ptr: *mut BakedAnim = &mut bake;
            // Mut view over the stack-local bake (write provenance).
            let bv = View::<BakedAnim, crate::native::view::Mut>::from_ptr(bake_ptr);
            let node_base = nodes.as_ptr() as *mut BakedNode;
            let elem_base = elements.as_ptr() as *mut BakedElement;
            fn np(r: Option<&View<BakedNode, crate::native::view::Mut>>) -> *mut BakedNode {
                r.map_or(core::ptr::null_mut(), |n| n.as_ptr() as *mut BakedNode)
            }
            fn ep(r: Option<&View<BakedElement, crate::native::view::Mut>>) -> *mut BakedElement {
                r.map_or(core::ptr::null_mut(), |e| e.as_ptr() as *mut BakedElement)
            }

            assert_eq!(np(find_baked_node_by_typed_id(bv, 1)), node_base);
            assert_eq!(np(find_baked_node_by_typed_id(bv, 3)), node_base.add(1));
            assert_eq!(np(find_baked_node_by_typed_id(bv, 5)), node_base.add(2));
            // `ufbxi_macro_lower_bound_eq` does NOT write the out-param on a
            // miss; the `SIZE_MAX` pre-initializer is what makes this None.
            assert!(find_baked_node_by_typed_id(bv, 4).is_none());
            assert!(find_baked_node_by_typed_id(bv, 0).is_none());
            assert!(find_baked_node_by_typed_id(bv, 9).is_none());

            assert_eq!(ep(find_baked_element_by_element_id(bv, 2)), elem_base);
            assert_eq!(
                ep(find_baked_element_by_element_id(bv, 6)),
                elem_base.add(2)
            );
            assert!(find_baked_element_by_element_id(bv, 3).is_none());

            // The by-pointer wrappers ARE null-checked (unlike the by-id ones)
            // — the null arms are the `None` cases.
            let mut node: Node = MaybeUninit::zeroed().assume_init();
            node.element.typed_id = 5;
            node.element.element_id = 6;
            let nv = View::<Node, crate::native::view::Mut>::from_ptr(&raw mut node);
            assert_eq!(np(find_baked_node(Some(bv), Some(nv))), node_base.add(2));
            assert!(find_baked_node(None, Some(nv)).is_none());
            assert!(find_baked_node(Some(bv), None).is_none());

            let ev = View::<Element, crate::native::view::Mut>::from_ptr(&raw mut node.element);
            assert_eq!(ep(find_baked_element(Some(bv), Some(ev))), elem_base.add(2));
            assert!(find_baked_element(None, Some(ev)).is_none());
            assert!(find_baked_element(Some(bv), None).is_none());
        }
    }

    fn baked_vec3(time: f64, v: Real, flags: BakedKeyFlags) -> BakedVec3 {
        BakedVec3 {
            time,
            value: Vec3 { x: v, y: v, z: v },
            flags,
        }
    }

    fn baked_quat(time: f64, v: Real, flags: BakedKeyFlags) -> BakedQuat {
        BakedQuat {
            time,
            value: Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: v,
            },
            flags,
        }
    }

    #[test]
    fn test_evaluate_baked_vec3() {
        unsafe {
            let none = BakedKeyFlags::NONE;

            // `begin == 0`: the query lands before the first key.
            let keys = [baked_vec3(1.0, 10.0, none), baked_vec3(2.0, 20.0, none)];
            let eval = |t: f64| evaluate_baked_vec3(List::from_slice(&keys), t);
            assert_eq!(eval(0.0).x, 10.0);
            // Past the last key the loop runs out and C indexes `count - 1`.
            assert_eq!(eval(3.0).x, 20.0);
            // Exact hit on a key time.
            assert_eq!(eval(2.0).x, 20.0);

            // Linear interpolation, and the two step overrides.
            let keys = [baked_vec3(0.0, 0.0, none), baked_vec3(1.0, 10.0, none)];
            assert_eq!(evaluate_baked_vec3(List::from_slice(&keys), 0.25).x, 2.5);
            let keys = [
                baked_vec3(0.0, 0.0, BakedKeyFlags::STEP_LEFT),
                baked_vec3(1.0, 10.0, none),
            ];
            assert_eq!(evaluate_baked_vec3(List::from_slice(&keys), 0.25).x, 0.0);
            let keys = [
                baked_vec3(0.0, 0.0, none),
                baked_vec3(1.0, 10.0, BakedKeyFlags::STEP_RIGHT),
            ];
            assert_eq!(evaluate_baked_vec3(List::from_slice(&keys), 0.25).x, 10.0);

            // The `end - begin >= 8` binary-search prologue: 10 keys forces it,
            // and the linear scan must resume from the narrowed `begin`.
            let mut many: std::vec::Vec<BakedVec3> = std::vec::Vec::new();
            for i in 0..10 {
                many.push(baked_vec3(i as f64, (i as Real) * 10.0, none));
            }
            let eval = |t: f64| evaluate_baked_vec3(List::from_slice(&many), t);
            assert_eq!(eval(4.5).x, 45.0);
            assert_eq!(eval(0.5).x, 5.0);
            assert_eq!(eval(8.5).x, 85.0);
            assert_eq!(eval(100.0).x, 90.0);
        }
    }

    #[test]
    fn test_evaluate_baked_quat_step_asymmetry() {
        // The trap of this unit: `ufbx_evaluate_baked_quat`'s first `prev--`
        // (ufbx.c:31393) has NO `UFBX_BAKED_KEY_STEP_RIGHT` term, while
        // `ufbx_evaluate_baked_vec3`'s (ufbx.c:31361) does. On a duplicated key
        // time the two functions therefore pick DIFFERENT keys unless the
        // middle key carries the flag. Copying vec3's body over would make
        // both cases below return the same key.
        unsafe {
            let none = BakedKeyFlags::NONE;
            let step_right = BakedKeyFlags::STEP_RIGHT;

            // Duplicated time at indices 0 and 1, no flags.
            let v = [
                baked_vec3(1.0, 10.0, none),
                baked_vec3(1.0, 20.0, none),
                baked_vec3(2.0, 30.0, none),
            ];
            let q = [
                baked_quat(1.0, 10.0, none),
                baked_quat(1.0, 20.0, none),
                baked_quat(2.0, 30.0, none),
            ];
            // vec3: the flag is missing, so `prev` stays at index 1.
            assert_eq!(evaluate_baked_vec3(List::from_slice(&v), 1.0).x, 20.0);
            // quat: unconditional `prev--`, so it reads index 0 instead.
            assert_eq!(evaluate_baked_quat(List::from_slice(&q), 1.0).w, 10.0);

            // With `UFBX_BAKED_KEY_STEP_RIGHT` on the middle key both agree.
            let v = [
                baked_vec3(1.0, 10.0, none),
                baked_vec3(1.0, 20.0, step_right),
                baked_vec3(2.0, 30.0, none),
            ];
            let q = [
                baked_quat(1.0, 10.0, none),
                baked_quat(1.0, 20.0, step_right),
                baked_quat(2.0, 30.0, none),
            ];
            assert_eq!(evaluate_baked_vec3(List::from_slice(&v), 1.0).x, 10.0);
            assert_eq!(evaluate_baked_quat(List::from_slice(&q), 1.0).w, 10.0);

            // `prev > keys` guards both: a duplicate at index 0 with `begin == 1`
            // cannot decrement past the start.
            let q = [baked_quat(1.0, 10.0, none), baked_quat(2.0, 20.0, none)];
            assert_eq!(evaluate_baked_quat(List::from_slice(&q), 1.0).w, 10.0);
        }
    }

    #[test]
    fn test_evaluate_baked_quat_paths() {
        unsafe {
            let none = BakedKeyFlags::NONE;
            let keys = [baked_quat(1.0, 1.0, none), baked_quat(2.0, 1.0, none)];
            let eval = |t: f64| evaluate_baked_quat(List::from_slice(&keys), t);
            // `begin == 0` and the past-the-end `count - 1` read.
            assert_eq!(eval(0.0).w, 1.0);
            assert_eq!(eval(3.0).w, 1.0);

            // Interpolating arm: slerp between identity and a 90-degree Z
            // rotation, and `UFBX_BAKED_KEY_STEP_LEFT` / `_RIGHT` pinning
            // `t` to 0 / 1.
            let half = core::f64::consts::FRAC_1_SQRT_2 as Real;
            let identity = Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            };
            let rot90 = Quat {
                x: 0.0,
                y: 0.0,
                z: half,
                w: half,
            };
            let pair = |left: BakedKeyFlags, right: BakedKeyFlags| {
                let mut keys = [baked_quat(0.0, 0.0, left), baked_quat(1.0, 0.0, right)];
                keys[0].value = identity;
                keys[1].value = rot90;
                keys
            };

            let keys = pair(none, none);
            assert!(quat_close(
                evaluate_baked_quat(List::from_slice(&keys), 0.5),
                quat_slerp(identity, rot90, 0.5)
            ));

            let keys = pair(BakedKeyFlags::STEP_LEFT, none);
            assert!(quat_close(
                evaluate_baked_quat(List::from_slice(&keys), 0.5),
                identity
            ));

            let keys = pair(none, BakedKeyFlags::STEP_RIGHT);
            assert!(quat_close(
                evaluate_baked_quat(List::from_slice(&keys), 0.5),
                rot90
            ));
        }
    }

    fn quat_close(a: Quat, b: Quat) -> bool {
        let eps: Real = 1e-9;
        (a.x - b.x).abs() < eps
            && (a.y - b.y).abs() < eps
            && (a.z - b.z).abs() < eps
            && (a.w - b.w).abs() < eps
    }

    fn vec3_close(a: Vec3, b: Vec3) -> bool {
        let eps: Real = 1e-9;
        (a.x - b.x).abs() < eps && (a.y - b.y).abs() < eps && (a.z - b.z).abs() < eps
    }

    #[test]
    fn test_quat_math_family() {
        {
            let identity = Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            };
            let q = Quat {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                w: 4.0,
            };

            // ufbx_quat_dot / ufbx_quat_mul
            assert_eq!(quat_dot(q, q), 30.0);
            assert!(quat_close(quat_mul(identity, q), q));
            assert!(quat_close(quat_mul(q, identity), q));

            // ufbx_quat_normalize: `norm == 0` returns the identity verbatim
            // (NOT a division by zero).
            let zero = Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 0.0,
            };
            assert!(quat_close(quat_normalize(zero), IDENTITY_QUAT));
            let n = quat_normalize(q);
            // Tolerance scales with the Real width (f32 leaves ~eps residual).
            assert!((quat_dot(n, n) - 1.0).abs() <= 4.0 * Real::EPSILON);
            assert!(quat_close(
                quat_normalize(Quat {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 4.0
                }),
                identity
            ));

            // ufbx_quat_fix_antipodal: flips only when the dot is negative.
            let neg = Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: -1.0,
            };
            assert!(quat_close(quat_fix_antipodal(neg, identity), identity));
            assert!(quat_close(quat_fix_antipodal(identity, identity), identity));
            // Exactly-zero dot is NOT flipped (`< 0.0`, not `<= 0.0`).
            let ortho = Quat {
                x: 1.0,
                y: 0.0,
                z: 0.0,
                w: 0.0,
            };
            assert!(quat_close(quat_fix_antipodal(ortho, identity), ortho));

            // ufbx_vec3_normalize
            assert!(vec3_close(
                vec3_normalize(Vec3 {
                    x: 3.0,
                    y: 4.0,
                    z: 0.0
                }),
                Vec3 {
                    x: 0.6,
                    y: 0.8,
                    z: 0.0
                }
            ));
            // `ufbxi_normalize3` returns zero below `UFBX_EPSILON`.
            assert!(vec3_close(
                vec3_normalize(Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0
                }),
                Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0
                }
            ));

            // ufbx_quat_slerp: the antipodal branch negates `b` first, which
            // makes `omega` 0 and hits the `<= FLT_MIN` early-out returning `a`.
            assert!(quat_close(quat_slerp(identity, neg, 0.5), identity));
            let half = core::f64::consts::FRAC_1_SQRT_2 as Real;
            let rot90 = Quat {
                x: 0.0,
                y: 0.0,
                z: half,
                w: half,
            };
            assert!(quat_close(quat_slerp(identity, rot90, 0.0), identity));
            assert!(quat_close(quat_slerp(identity, rot90, 1.0), rot90));
            // Halfway is the 45-degree rotation.
            let rot45 = Quat {
                x: 0.0,
                y: 0.0,
                z: (22.5f64).to_radians().sin() as Real,
                w: (22.5f64).to_radians().cos() as Real,
            };
            assert!(quat_close(quat_slerp(identity, rot90, 0.5), rot45));
        }
    }

    #[test]
    fn test_quat_to_euler_all_orders() {
        {
            let orders = [
                RotationOrder::Xyz,
                RotationOrder::Xzy,
                RotationOrder::Yzx,
                RotationOrder::Yxz,
                RotationOrder::Zxy,
                RotationOrder::Zyx,
            ];
            // Angles small enough to stay off every gimbal-lock branch, and
            // distinct per axis so a swapped `vx`/`vy`/`vz` assignment shows up.
            let euler = Vec3 {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            };
            // Round-trip loss scales with the Real width: ~1e-6 degrees at
            // f32 (C itself loosens its gimbal-lock eps under
            // UFBX_REAL_IS_FLOAT, ufbx.c:31625-31630), far below 1e-9 at f64.
            #[cfg(not(feature = "real-is-f32"))]
            let tol: Real = 1e-9;
            #[cfg(feature = "real-is-f32")]
            let tol: Real = 1e-4;
            for order in orders {
                let q = euler_to_quat(euler, order);
                let back = quat_to_euler(q, order);
                assert!(
                    (back.x - euler.x).abs() < tol
                        && (back.y - euler.y).abs() < tol
                        && (back.z - euler.z).abs() < tol,
                    "order {:?}: {:?}",
                    order as u32,
                    (back.x, back.y, back.z)
                );
            }

            // The identity quaternion is zero rotation in every order.
            for order in orders {
                let v = quat_to_euler(IDENTITY_QUAT, order);
                assert!(vec3_close(
                    v,
                    Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0
                    }
                ));
            }
        }
    }

    #[test]
    fn test_find_shader_prop_len_falls_back_to_empty_string() {
        // C: no bindings -> `ufbx_empty_string`, not NULL.
        let s = find_shader_prop_len::<Const>(None, b"Diffuse");
        assert_eq!(s.length, 0);
        assert_eq!(s.data, EMPTY_STRING.0.data);
        // SAFETY: null shader (handled) and a NUL-terminated literal name — the
        // C-string flavor's contract.
        assert_eq!(
            unsafe { find_shader_prop(core::ptr::null(), b"Diffuse\0".as_ptr()) }.length,
            0
        );
    }

    // The `#else` (feature-disabled) arms are the only bodies these two entry
    // points have; they compile out under the default feature set, so the tests
    // carry the same `cfg` the port does.
    //
    // C-parity note: `ufbxi_report_err_msg` bottoms out in `ufbxi_fail_imp_err`
    // (ufbx.c:3411), which sets `description`/`info` and pushes a stack frame
    // but NEVER resolves `error->type` — and these arms do not call
    // `ufbxi_fix_error_type`. So `type` stays `UFBX_ERROR_NONE`; the
    // "Feature disabled" description is the whole signal. Asserting
    // `UFBX_ERROR_FEATURE_DISABLED` here would be asserting against the C.
    #[cfg(any(not(feature = "scene-eval"), not(feature = "baking")))]
    unsafe fn assert_feature_disabled(error: &Error, info: &str) {
        assert_eq!(error.type_ as u32, crate::generated::ErrorType::None as u32);
        // SAFETY: `error.description` is a live `String` set by the failing
        // feature-disabled call; `.data`/`.length` describe that many readable bytes.
        let desc = unsafe {
            core::slice::from_raw_parts(error.description.data, error.description.length)
        };
        assert_eq!(desc, b"Feature disabled");
        assert_eq!(error.info(), info);
    }

    #[test]
    #[cfg(not(feature = "scene-eval"))]
    fn test_evaluate_scene_feature_disabled() {
        unsafe {
            let error: Error =
                evaluate_scene(core::ptr::null(), core::ptr::null(), 0.0, core::ptr::null())
                    .unwrap_err();
            assert_feature_disabled(&error, "UFBX_ENABLE_SCENE_EVALUATION");
        }
    }

    #[test]
    #[cfg(not(feature = "baking"))]
    fn test_bake_anim_feature_disabled() {
        unsafe {
            // C's `#else` arm only runs `ufbx_assert(scene)` — it never
            // dereferences it — so a dangling non-null pointer is enough (and
            // `ufbx_scene` has non-null `Ref` fields, so it cannot be zeroed).
            let scene: *const Scene = core::ptr::NonNull::dangling().as_ptr();
            let error: Error = bake_anim(scene, core::ptr::null(), core::ptr::null()).unwrap_err();
            assert_feature_disabled(&error, "UFBX_ENABLE_ANIMATION_BAKING");
        }
    }

    // `ufbx_blend_shape` embeds `ufbx_element`, which holds a non-null
    // `Ref<Scene>` — the value can never be materialized zeroed, so the fields
    // the offset lookup reads are written through raw pointers into a zeroed
    // `MaybeUninit` that is never `assume_init`ed.
    unsafe fn write_blend_shape(
        storage: &mut MaybeUninit<BlendShape>,
        offset_vertices: &[u32],
        position_offsets: &[Vec3],
        offset_weights: &[Real],
    ) -> *mut BlendShape {
        let shape: *mut BlendShape = storage.as_mut_ptr();
        // SAFETY: `shape` addresses the caller's live `MaybeUninit<BlendShape>`
        // storage; each `&raw mut (*shape).field` projects one of its own fields
        // and `.write()` initializes it without reading the uninit prior value.
        unsafe {
            (&raw mut (*shape).num_offsets).write(offset_vertices.len());
            (&raw mut (*shape).offset_vertices).write(List::from_slice(offset_vertices));
            (&raw mut (*shape).position_offsets).write(List::from_slice(position_offsets));
            (&raw mut (*shape).offset_weights).write(List::from_slice(offset_weights));
        }
        shape
    }

    fn vec3(x: Real, y: Real, z: Real) -> Vec3 {
        Vec3 { x, y, z }
    }

    #[test]
    fn test_get_blend_shape_offset_index() {
        unsafe {
            let vertices: [u32; 3] = [1, 3, 7];
            let offsets: [Vec3; 3] = [
                vec3(1.0, 0.0, 0.0),
                vec3(0.0, 2.0, 0.0),
                vec3(0.0, 0.0, 3.0),
            ];
            let mut storage = MaybeUninit::<BlendShape>::zeroed();
            let shape = write_blend_shape(&mut storage, &vertices, &offsets, &[]);

            // `ufbxi_macro_lower_bound_eq` does NOT write the out-param on a
            // miss, so the pre-seeded `SIZE_MAX` is what turns into NO_INDEX.
            let view = View::<BlendShape, Const>::from_ptr(shape);
            assert_eq!(get_blend_shape_offset_index(Some(view), 1), 0);
            assert_eq!(get_blend_shape_offset_index(Some(view), 3), 1);
            assert_eq!(get_blend_shape_offset_index(Some(view), 7), 2);
            assert_eq!(get_blend_shape_offset_index(Some(view), 0), NO_INDEX);
            assert_eq!(get_blend_shape_offset_index(Some(view), 2), NO_INDEX);
            assert_eq!(get_blend_shape_offset_index(Some(view), 8), NO_INDEX);

            assert_eq!(get_blend_shape_vertex_offset(shape, 3).y, 2.0);
            let zero = get_blend_shape_vertex_offset(shape, 2);
            assert_eq!((zero.x, zero.y, zero.z), (0.0, 0.0, 0.0));
        }
    }

    #[test]
    fn test_add_blend_shape_vertex_offsets() {
        unsafe {
            let vertex_indices: [u32; 3] = [1, 3, 7];
            let offsets: [Vec3; 3] = [
                vec3(1.0, 0.0, 0.0),
                vec3(0.0, 2.0, 0.0),
                vec3(0.0, 0.0, 3.0),
            ];
            // Shorter than `num_offsets`: entries past `weights.count` take the
            // bare `weight` (C: `if (i < weights.count)`).
            let weights: [Real; 2] = [0.5, 1.0];
            let mut storage = MaybeUninit::<BlendShape>::zeroed();
            let shape = write_blend_shape(&mut storage, &vertex_indices, &offsets, &weights);

            let mut verts: [Vec3; 5] = [vec3(0.0, 0.0, 0.0); 5];
            add_blend_shape_vertex_offsets(shape, verts.as_mut_ptr(), verts.len(), 2.0);
            assert_eq!((verts[1].x, verts[1].y, verts[1].z), (1.0, 0.0, 0.0));
            assert_eq!((verts[3].x, verts[3].y, verts[3].z), (0.0, 4.0, 0.0));
            // Vertex 7 is out of the destination range and is dropped.
            assert_eq!((verts[0].x, verts[2].y, verts[4].z), (0.0, 0.0, 0.0));

            // `weight == 0.0` and a null destination both return early.
            add_blend_shape_vertex_offsets(shape, verts.as_mut_ptr(), verts.len(), 0.0);
            assert_eq!((verts[1].x, verts[3].y), (1.0, 4.0));
            add_blend_shape_vertex_offsets(shape, core::ptr::null_mut(), verts.len(), 2.0);
        }
    }
}
