//! Port of the `// -- API` banner section of ufbx.c (ufbx.c:30333+), preceded
//! by the refcount lifecycle functions (`ufbxi_free_scene_imp`
//! ufbx.c:30243-30247 and `ufbxi_init_ref`/`ufbxi_retain_ref`/
//! `ufbxi_release_ref` ufbx.c:30249-30300 — C forward-declares the first two
//! next to `ufbxi_refcount` at ufbx.c:6229-6230 but defines them here).
//!
//! Ported so far: the `ufbx_abi_data` globals (ufbx.c:30339-30404), the
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
//! (31723-31926, ported ahead of order by earlier units — this unit adds the
//! missing `capi.rs` shims), `ufbx_catch_get_skin_vertex_matrix`
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
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
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
    Face, GeometryCache, Light, LineCurve, LodGroup, Marker, Material, MaterialTexture, Matrix,
    Mesh, MetadataObject, NameElement, Node, NurbsBasis, NurbsCurve, NurbsSurface,
    NurbsTrimBoundary, NurbsTrimSurface, OpenFileInfo, Panic, Pose, ProceduralGeometry, Prop,
    Props, Quat, RawAllocatorOpts, RawGeometryCacheDataOpts, RawGeometryCacheOpts, RawLoadOpts,
    RawOpenFileOpts, RawOpenMemoryOpts, RawStream, RawVertexStream, RotationOrder, Scene,
    SelectionNode, SelectionSet, Shader, ShaderBinding, ShaderPropBinding, ShaderTexture,
    ShaderTextureInput, SkinCluster, SkinDeformer, SkinVertex, SkinWeight, StereoCamera,
    SurfacePoint, Texture, TopoEdge, Transform, Unknown, Vec2, Vec3, Vec4, VertexReal, VertexVec2,
    VertexVec3, VertexVec4, Video,
};
#[cfg(feature = "geometry-cache")]
use crate::generated::{CacheDataEncoding, CacheDataFormat, OpenFileType, RawOpenFileCb};
use crate::generated::{
    EvaluateFlags, Interpolation, Keyframe, PropFlags, RawAnimOpts, RawEvaluateOpts, TransformFlags,
};
#[cfg(feature = "tessellation")]
use crate::generated::{RawTessellateCurveOpts, RawTessellateSurfaceOpts};
#[cfg(feature = "baking")]
use crate::native::allocator::free;
use crate::native::allocator::{
    align_to_mask, alloc, free_ator, Allocator, ANIM_IMP_MAGIC, BAKED_ANIM_IMP_MAGIC,
    CACHE_IMP_MAGIC, LINE_CURVE_IMP_MAGIC, MESH_IMP_MAGIC, REFCOUNT_IMP_MAGIC, SCENE_IMP_MAGIC,
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
use crate::native::view::{Const, Mode, View};
// Used by the feature-enabled arms of `ufbx_bake_anim` /
// `ufbx_tessellate_nurbs_curve` / `_surface` and unconditionally by
// `ufbx_subdivide_mesh` / `ufbx_load_geometry_cache_len`.
use crate::native::error::ufbxi_check_opts_ptr;
use crate::native::error::{clear_error, fix_error_type};
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
    find_enum, find_real as ufbxi_find_real, get_imp, get_name_key, Context, InnerContext, MeshImp,
    PropsView, Refcount, SceneImp, ELEMENT_TYPE_COUNT,
};
use crate::native::platform::{
    add_ptr, atomic_counter_dec, atomic_counter_free, atomic_counter_inc, atomic_counter_init,
    macro_lower_bound_eq, macro_upper_bound_eq, math, min_sz, ufbx_assert, ufbxi_ignore,
    ufbxi_unreachable, NO_INDEX, SOURCE_VERSION, THREAD_SAFE,
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
    ufbx_assert!((*imp).magic == SCENE_IMP_MAGIC);
    buf_free(&mut (*imp).string_buf);
}

// ufbx.c:30249-30259 `ufbxi_init_ref`
#[inline(never)]
pub(crate) unsafe fn init_ref(refcount: *mut Refcount, magic: u32, parent: *mut Refcount) {
    if !parent.is_null() {
        retain_ref(parent);
    }

    atomic_counter_init(&mut (*refcount).refcount);
    (*refcount).self_magic = REFCOUNT_IMP_MAGIC;
    (*refcount).type_magic = magic;
    (*refcount).parent = parent;
}

// ufbx.c:30261-30267 `ufbxi_retain_ref`
#[inline(never)]
pub(crate) unsafe fn retain_ref(refcount: *mut Refcount) {
    ufbx_assert!((*refcount).self_magic == REFCOUNT_IMP_MAGIC);
    let count: usize = atomic_counter_inc(&mut (*refcount).refcount);
    ufbxi_ignore!(count);
    ufbx_assert!(count < usize::MAX / 2);
}

// ufbx.c:30269-30300 `ufbxi_release_ref`
#[inline(never)]
pub(crate) unsafe fn release_ref(mut refcount: *mut Refcount) {
    while !refcount.is_null() {
        ufbx_assert!((*refcount).self_magic == REFCOUNT_IMP_MAGIC);
        if atomic_counter_dec(&mut (*refcount).refcount) > 0 {
            return;
        }
        atomic_counter_free(&mut (*refcount).refcount);

        let parent: *mut Refcount = (*refcount).parent;
        let type_magic: u32 = (*refcount).type_magic;

        (*refcount).self_magic = 0;
        (*refcount).type_magic = 0;

        // Type-specific cleanup
        match type_magic {
            SCENE_IMP_MAGIC => free_scene_imp(refcount as *mut SceneImp),
            CACHE_IMP_MAGIC => free_geometry_cache_imp(refcount as *mut GeometryCacheImp),
            _ => {}
        }

        // We need to free `data_buf` last and be careful to copy it to
        // the stack since the `ufbxi_refcount` that contains it is allocated
        // from the same result buffer!
        let mut ator: Allocator = (*refcount).ator;
        let mut buf: Buf = (*refcount).buf;
        buf.ator = &raw mut ator;
        buf_free(&mut buf);
        free_ator(&mut ator);

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
    open_file_ctx(
        stream,
        (*info).context,
        path,
        path_len,
        core::ptr::null(),
        core::ptr::null_mut(),
    )
}

// ufbx.c:30412-30415 `ufbx_open_file`
pub(crate) unsafe fn open_file(
    stream: *mut RawStream,
    path: *const u8,
    path_len: usize,
    opts: *const RawOpenFileOpts,
    error: *mut Error,
) -> bool {
    open_file_ctx(stream, 0 as OpenFileContext, path, path_len, opts, error)
}

// ufbx.c:30417-30435 `ufbx_open_file_ctx`
pub(crate) unsafe fn open_file_ctx(
    stream: *mut RawStream,
    ctx: OpenFileContext,
    path: *const u8,
    mut path_len: usize,
    opts: *const RawOpenFileOpts,
    error: *mut Error,
) -> bool {
    // C: `ufbxi_file_context fc; // ufbxi_uninit`
    let fc = FileContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit()));
    begin_file_context(&fc, ctx, core::ptr::null());
    if path_len == usize::MAX {
        path_len = strlen(path);
    }
    // C: `#if !defined(UFBX_NO_STDIO)` — always taken (no matching feature);
    // the disabled branch reports `"UFBX_NO_STDIO", "Feature disabled"`.
    let ok: bool = stdio_open(
        &fc,
        stream,
        path,
        path_len,
        if !opts.is_null() {
            (*opts).filename_null_terminated
        } else {
            false
        },
    );
    end_file_context(&fc, error, ok);
    ok
}

// ufbx.c:30437-30440 `ufbx_open_memory`
pub(crate) unsafe fn open_memory(
    stream: *mut RawStream,
    data: *const c_void,
    data_size: usize,
    opts: *const RawOpenMemoryOpts,
    error: *mut Error,
) -> bool {
    open_memory_ctx(stream, 0 as OpenFileContext, data, data_size, opts, error)
}

// ufbx.c:30442-30495 `ufbx_open_memory_ctx`
pub(crate) unsafe fn open_memory_ctx(
    stream: *mut RawStream,
    ctx: OpenFileContext,
    data: *const c_void,
    data_size: usize,
    opts: *const RawOpenMemoryOpts,
    error: *mut Error,
) -> bool {
    let mut local_opts = MaybeUninit::<RawOpenMemoryOpts>::uninit(); // ufbxi_uninit
    let mut opts = opts;
    if opts.is_null() {
        core::ptr::write_bytes(
            local_opts.as_mut_ptr() as *mut u8,
            0,
            size_of::<RawOpenMemoryOpts>(),
        );
        opts = local_opts.as_ptr();
    }
    ufbx_assert!((*opts)._begin_zero == 0 && (*opts)._end_zero == 0);

    // C: `ufbxi_file_context fc; // ufbxi_uninit`
    let fc = FileContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit()));
    begin_file_context(&fc, ctx, &(*opts).allocator);

    let copy_size: usize = if (*opts).no_copy { 0 } else { data_size };

    // Align the allocation size to 8 bytes to make sure the header is aligned.
    let self_size: usize = align_to_mask(size_of::<MemoryStream>().wrapping_add(copy_size), 7);

    let memory: *mut u8 = alloc::<u8>(fc.ator_mut_ptr(), self_size);
    if memory.is_null() {
        end_file_context(&fc, error, false);
        return false;
    }

    let mem = memory as *mut MemoryStream;
    core::ptr::write_bytes(mem as *mut u8, 0, size_of::<MemoryStream>());

    (*mem).size = data_size;
    (*mem).self_size = self_size;
    (*mem).close_cb = (*opts).close_cb;

    if (*opts).no_copy {
        (*mem).data = data;
    } else {
        // C: `memcpy(mem->data_copy, data, data_size)` — the flexible array
        // member starts right after the header (see `MemoryStream`).
        let data_copy: *mut u8 = (mem as *mut u8).add(size_of::<MemoryStream>());
        core::ptr::copy_nonoverlapping(data as *const u8, data_copy, data_size);
        (*mem).data = data_copy as *const c_void;
    }

    // Transplant the allocator in the result blob
    if !fc.parent_ator().is_null() {
        (*mem).parent_ator = fc.parent_ator();
    } else {
        fc.set_parent_ator(&mut (*mem).local_ator);
    }

    (*stream).read_fn = Some(memory_read);
    (*stream).skip_fn = Some(memory_skip);
    (*stream).size_fn = Some(memory_size);
    (*stream).close_fn = Some(memory_close);
    (*stream).user = mem as *mut c_void;

    end_file_context(&fc, error, true);

    true
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
    error: *mut Error,
) -> *mut Scene {
    ufbxi_check_opts_ptr!(Scene, opts, error);
    // C: `ufbxi_context uc; // ufbxi_uninit` + `memset(&uc, 0, sizeof(ufbxi_context));`
    let uc_storage = Context(core::cell::UnsafeCell::new(MaybeUninit::uninit())); // ufbxi_uninit
    let uc: &Context = &uc_storage;
    core::ptr::write_bytes(uc.get() as *mut u8, 0, size_of::<InnerContext>());
    // C: `uc.data_begin = uc.data = (const char *)data;`
    uc.set_data(data as *const u8);
    uc.set_data_begin(uc.data());
    uc.set_data_size(size);
    uc.set_progress_bytes_total(size as u64);
    evaluate::load(uc, opts, error)
}

// ufbx.c:30513-30516 `ufbx_load_file`
pub(crate) unsafe fn load_file(
    filename: *const u8,
    opts: *const RawLoadOpts,
    error: *mut Error,
) -> *mut Scene {
    load_file_len(filename, usize::MAX, opts, error)
}

// ufbx.c:30518-30527 `ufbx_load_file_len`
pub(crate) unsafe fn load_file_len(
    filename: *const u8,
    filename_len: usize,
    opts: *const RawLoadOpts,
    error: *mut Error,
) -> *mut Scene {
    ufbxi_check_opts_ptr!(Scene, opts, error);
    let uc_storage = Context(core::cell::UnsafeCell::new(MaybeUninit::uninit())); // ufbxi_uninit
    let uc: &Context = &uc_storage;
    core::ptr::write_bytes(uc.get() as *mut u8, 0, size_of::<InnerContext>());
    uc.set_deferred_load(true);
    uc.set_load_filename(filename);
    uc.set_load_filename_len(filename_len);
    evaluate::load(uc, opts, error)
}

// ufbx.c:30529-30532 `ufbx_load_stdio`
pub(crate) unsafe fn load_stdio(
    file_void: *mut c_void,
    opts: *const RawLoadOpts,
    error: *mut Error,
) -> *mut Scene {
    load_stdio_prefix(file_void, core::ptr::null(), 0, opts, error)
}

// ufbx.c:30534-30554 `ufbx_load_stdio_prefix`
pub(crate) unsafe fn load_stdio_prefix(
    file_void: *mut c_void,
    prefix: *const c_void,
    prefix_size: usize,
    opts: *const RawLoadOpts,
    error: *mut Error,
) -> *mut Scene {
    // C: `#if !defined(UFBX_NO_STDIO)` — always taken (no matching feature);
    // the disabled `#else` arm reports `"UFBX_NO_STDIO", "Feature disabled"`
    // through a deferred-failure `ufbxi_load`.
    if file_void.is_null() {
        return core::ptr::null_mut();
    }
    // C: `ufbx_stream stream = { 0 };`
    let mut stream: RawStream = MaybeUninit::zeroed().assume_init();
    stdio_init(&mut stream, file_void, false);
    load_stream_prefix(&stream, prefix, prefix_size, opts, error)
}

// ufbx.c:30556-30559 `ufbx_load_stream`
pub(crate) unsafe fn load_stream(
    stream: *const RawStream,
    opts: *const RawLoadOpts,
    error: *mut Error,
) -> *mut Scene {
    load_stream_prefix(stream, core::ptr::null(), 0, opts, error)
}

// ufbx.c:30561-30576 `ufbx_load_stream_prefix`
pub(crate) unsafe fn load_stream_prefix(
    stream: *const RawStream,
    prefix: *const c_void,
    prefix_size: usize,
    opts: *const RawLoadOpts,
    error: *mut Error,
) -> *mut Scene {
    ufbxi_check_opts_ptr!(Scene, opts, error);
    let uc_storage = Context(core::cell::UnsafeCell::new(MaybeUninit::uninit())); // ufbxi_uninit
    let uc: &Context = &uc_storage;
    core::ptr::write_bytes(uc.get() as *mut u8, 0, size_of::<InnerContext>());
    // C: `uc.data_begin = uc.data = (const char *)prefix;`
    uc.set_data(prefix as *const u8);
    uc.set_data_begin(uc.data());
    uc.set_data_size(prefix_size);
    uc.set_read_fn((*stream).read_fn);
    uc.set_skip_fn((*stream).skip_fn);
    uc.set_size_fn((*stream).size_fn);
    uc.set_close_fn((*stream).close_fn);
    uc.set_read_user((*stream).user);

    let scene: *mut Scene = evaluate::load(uc, opts, error);
    scene
}

// ufbx.c:30578-30586 `ufbx_free_scene`
// C has no `ufbxi_noinline` here (unlike `ufbx_format_error` below).
pub(crate) unsafe fn free_scene(scene: *mut Scene) {
    if scene.is_null() {
        return;
    }

    let imp: *mut SceneImp = get_imp(scene as *mut c_void);
    ufbx_assert!((*imp).magic == SCENE_IMP_MAGIC);
    if (*imp).magic != SCENE_IMP_MAGIC {
        return;
    }
    release_ref(&raw mut (*imp).refcount);
}

// ufbx.c:30588-30596 `ufbx_retain_scene`
// C has no `ufbxi_noinline` here (unlike `ufbx_format_error` below).
pub(crate) unsafe fn retain_scene(scene: *mut Scene) {
    if scene.is_null() {
        return;
    }

    let imp: *mut SceneImp = get_imp(scene as *mut c_void);
    ufbx_assert!((*imp).magic == SCENE_IMP_MAGIC);
    if (*imp).magic != SCENE_IMP_MAGIC {
        return;
    }
    retain_ref(&raw mut (*imp).refcount);
}

// ufbx.c:30598-30633 `ufbx_format_error`
#[inline(never)]
pub(crate) unsafe fn format_error(dst: *mut u8, dst_size: usize, error: *const Error) -> usize {
    if dst.is_null() || dst_size == 0 {
        return 0;
    }
    if error.is_null() {
        *dst = b'\0';
        return 0;
    }

    let mut offset: usize = 0;

    {
        let num: i32;
        if (*error).info_length > 0 && (*error).info_length < ERROR_INFO_LENGTH {
            num = ufbxi_snprintf!(
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
            );
        } else {
            num = ufbxi_snprintf!(
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
            );
        }

        if num > 0 {
            offset = min_sz(offset.wrapping_add(num as usize), dst_size - 1);
        }
    }

    let stack_size: usize = min_sz((*error).stack_size as usize, ERROR_STACK_MAX_DEPTH);
    let line_width: i32 = 6;
    for i in 0..stack_size {
        // C: `const ufbx_error_frame *frame = &error->stack[i];`
        let frame: *const ErrorFrame = (&raw const (*error).stack as *const ErrorFrame).add(i);
        let num: i32 = ufbxi_snprintf!(
            dst.add(offset),
            dst_size - offset,
            "%*u:%s: %s\n",
            line_width,
            (*frame).source_line,
            (*frame).function.data,
            (*frame).description.data,
        );
        if num > 0 {
            offset = min_sz(offset.wrapping_add(num as usize), dst_size - 1);
        }
    }

    offset
}

// ufbx.c:30635-32095 is ported in C order below. Past ufbx.c:32214 some
// API-section entry points sit out of C order, ahead of their own unit,
// because the `// -- Scene processing` unit calls `ufbx_get_bone_pose` /
// `ufbx_euler_to_quat` and friends. All entry points are ported (203/203
// ufbx_abi exports).

// ufbx.c:30635-30650 `ufbx_find_prop_len`
pub(crate) unsafe fn find_prop_len<'a, M: Mode>(
    props: &'a View<Props, M>,
    name: *const u8,
    name_len: usize,
) -> Option<&'a View<Prop, M>> {
    let key = get_name_key(name, name_len);
    let name_str = safe_string(name, name_len);

    let mut props: Option<&'a View<Props, M>> = Some(props);
    while let Some(cur) = props {
        let mut index: usize = usize::MAX;
        macro_lower_bound_eq::<Prop>(
            4,
            &mut index,
            cur.props_data(),
            0,
            cur.props_count(),
            |a| cmp_prop_less_ref(a, name_str, key),
            |a| (*a)._internal_key == key && str_equal((*a).name, name_str),
        );
        if index != usize::MAX {
            // Mode-generic mint: `props_data()` is a VALUE read of the stored
            // run pointer, so this carries the table's stored (arena, write)
            // provenance — adequate for either mode.
            return Some(View::<Prop, M>::mint(cur.props_data().add(index)));
        }

        props = cur.defaults();
    }

    None
}

// ufbx.c:30652-30660 `ufbx_find_real_len`
pub(crate) unsafe fn find_real_len<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    name_len: usize,
    def: Real,
) -> Real {
    match find_prop_len(props, name, name_len) {
        // C-parity: `prop->value_real` is the `ufbx_prop` value union's first
        // real; the generated struct keeps only `value_vec4` (same mapping as
        // `native::parse::find_real`).
        Some(prop) => prop.value_vec4().x,
        None => def,
    }
}

// ufbx.c:30662-30670 `ufbx_find_vec3_len`
// Ported ahead of its banner section because `ufbxi_update_constraint`
// (ufbx.c:23416, `native::scene_process`) calls `ufbx_find_vec3`.
#[inline(never)]
pub(crate) unsafe fn find_vec3_len<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    name_len: usize,
    def: Vec3,
) -> Vec3 {
    match find_prop_len(props, name, name_len) {
        // C-parity: `prop->value_vec3` is the `ufbx_prop` value union's 3-real
        // view; the generated struct keeps only `value_vec4` (same mapping as
        // `native::parse::find_vec3`).
        Some(prop) => prop.value_vec3(),
        None => def,
    }
}

// ufbx.c:30672-30680 `ufbx_find_int_len`
#[inline(never)]
pub(crate) unsafe fn find_int_len<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    name_len: usize,
    def: i64,
) -> i64 {
    match find_prop_len(props, name, name_len) {
        Some(prop) => prop.value_int(),
        None => def,
    }
}

// ufbx.c:30682-30690 `ufbx_find_bool_len`
pub(crate) unsafe fn find_bool_len<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    name_len: usize,
    def: bool,
) -> bool {
    match find_prop_len(props, name, name_len) {
        Some(prop) => prop.value_int() != 0,
        None => def,
    }
}

// ufbx.c:30692-30700 `ufbx_find_string_len`
#[inline(never)]
pub(crate) unsafe fn find_string_len<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    name_len: usize,
    def: String,
) -> String {
    match find_prop_len(props, name, name_len) {
        Some(prop) => prop.value_str(),
        None => def,
    }
}

// ufbx.c:30702-30710 `ufbx_find_blob_len`
// C has no `ufbxi_noinline` here (unlike `ufbx_find_string_len` above).
pub(crate) unsafe fn find_blob_len<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    name_len: usize,
    def: Blob,
) -> Blob {
    match find_prop_len(props, name, name_len) {
        Some(prop) => prop.value_blob(),
        None => def,
    }
}

// ufbx.c:30712-30728 `ufbx_find_prop_concat`
// Ported ahead of its banner section because `ufbxi_update_constraint`
// (ufbx.c:23416, `native::scene_process`) calls it.
pub(crate) unsafe fn find_prop_concat<'a, M: Mode>(
    props: &'a View<Props, M>,
    parts: *const String,
    num_parts: usize,
) -> Option<&'a View<Prop, M>> {
    let key: u32 = get_concat_key(parts, num_parts);

    let mut props: Option<&'a View<Props, M>> = Some(props);
    while let Some(cur) = props {
        let mut index: usize = usize::MAX;

        macro_lower_bound_eq::<Prop>(
            2,
            &mut index,
            cur.props_data(),
            0,
            cur.props_count(),
            |a| cmp_prop_less_concat(a, parts, num_parts, key),
            |a| (*a)._internal_key == key && concat_str_cmp(&(*a).name, parts, num_parts) == 0,
        );
        if index != usize::MAX {
            // Same stored-provenance mint as `find_prop_len`.
            return Some(View::<Prop, M>::mint(cur.props_data().add(index)));
        }

        props = cur.defaults();
    }

    None
}

// ufbx.c:30730-30741 `ufbx_find_element_len`
pub(crate) unsafe fn find_element_len(
    scene: *const Scene,
    type_: ElementType,
    name: *const u8,
    name_len: usize,
) -> *mut Element {
    if scene.is_null() {
        return core::ptr::null_mut();
    }
    let name_str: String = safe_string(name, name_len);
    let key: u32 = get_name_key(name, name_len);

    let mut index: usize = usize::MAX;
    macro_lower_bound_eq::<NameElement>(
        16,
        &mut index,
        (*scene).elements_by_name.data,
        0,
        (*scene).elements_by_name.count,
        |a| cmp_name_element_less_ref(a, name_str, type_, key),
        |a| str_equal((*a).name, name_str) && (*a).type_ == type_,
    );

    if index < usize::MAX {
        ref_ptr(&(*(*scene).elements_by_name.data.add(index)).element)
    } else {
        core::ptr::null_mut()
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
    fetch_dst_element(element as *mut Element, false, (*prop).name.data, type_)
}

// ufbx.c:30750-30757 `ufbx_find_prop_element_len`
pub(crate) unsafe fn find_prop_element_len(
    element: *const Element,
    name: *const u8,
    name_len: usize,
    type_: ElementType,
) -> *mut Element {
    // Public-boundary root: `element` is a caller-owned `*const Element` whose
    // provenance can be a read-only `&Element` (safe Rust wrapper), so mint a
    // read-only `Const` view — legal for any readable provenance, unlike the
    // interior-mutable `Mut` view (Miri SB — see the topology finding).
    // `&raw` avoids forming an intermediate `&Props`.
    let props: &View<Props, Const> = View::<Props, Const>::from_ptr(&raw const (*element).props);
    match find_prop_len(props, name, name_len) {
        Some(prop) => get_prop_element(element, prop.as_ptr(), type_),
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:30760-30763 `ufbx_find_node_len`
pub(crate) unsafe fn find_node_len(
    scene: *const Scene,
    name: *const u8,
    name_len: usize,
) -> *mut Node {
    find_element_len(scene, ElementType::Node, name, name_len) as *mut Node
}

// ufbx.c:30765-30768 `ufbx_find_anim_stack_len`
pub(crate) unsafe fn find_anim_stack_len(
    scene: *const Scene,
    name: *const u8,
    name_len: usize,
) -> *mut AnimStack {
    find_element_len(scene, ElementType::AnimStack, name, name_len) as *mut AnimStack
}

// ufbx.c:30770-30773 `ufbx_find_material_len`
pub(crate) unsafe fn find_material_len(
    scene: *const Scene,
    name: *const u8,
    name_len: usize,
) -> *mut Material {
    find_element_len(scene, ElementType::Material, name, name_len) as *mut Material
}

// ufbx.c:30775-30790 `ufbx_find_anim_prop_len`
pub(crate) unsafe fn find_anim_prop_len(
    layer: *const AnimLayer,
    element: *const Element,
    prop: *const u8,
    prop_len: usize,
) -> *mut AnimProp {
    ufbx_assert!(!layer.is_null());
    ufbx_assert!(!element.is_null());
    if layer.is_null() || element.is_null() {
        return core::ptr::null_mut();
    }

    let prop_str: String = safe_string(prop, prop_len);

    let mut index: usize = usize::MAX;
    macro_lower_bound_eq::<AnimProp>(
        16,
        &mut index,
        (*layer).anim_props.data,
        0,
        (*layer).anim_props.count,
        // C: `a->element != element ? a->element < element : ufbxi_str_less(a->prop_name, prop_str)`
        // — a raw ADDRESS comparison of the owning element (the array is sorted
        // by element pointer, ufbx.c:18596), not by `element_id`.
        |a| {
            let a_element: *const Element = ref_ptr(&(*a).element);
            if a_element != element {
                a_element < element
            } else {
                str_less((*a).prop_name, prop_str)
            }
        },
        |a| std::ptr::eq(ref_ptr(&(*a).element), element) && str_equal((*a).prop_name, prop_str),
    );

    if index == usize::MAX {
        return core::ptr::null_mut();
    }
    (*layer).anim_props.data.add(index) as *mut AnimProp
}

// ufbx.c:30792-30812 `ufbx_find_anim_props`
#[inline(never)]
pub(crate) unsafe fn find_anim_props(
    layer: *const AnimLayer,
    element: *const Element,
) -> List<AnimProp> {
    // C: `ufbx_anim_prop_list result = { 0 };` — `List<T>` carries a private
    // `PhantomData` marker, so the C aggregate initializer becomes a zeroed
    // value with both public fields written (same shape as
    // `find_shader_prop_bindings_len` below).
    let mut result: List<AnimProp> = MaybeUninit::zeroed().assume_init();
    result.data = core::ptr::null();
    result.count = 0;
    ufbx_assert!(!layer.is_null());
    ufbx_assert!(!element.is_null());
    if layer.is_null() || element.is_null() {
        return result;
    }

    // C: `size_t begin = layer->anim_props.count, end = begin;` — `begin` is
    // pre-initialized because `ufbxi_macro_lower_bound_eq` does NOT write on a
    // miss (PORTING.md "Sorting & searching").
    let mut begin: usize = (*layer).anim_props.count;
    let mut end: usize = begin;
    macro_lower_bound_eq::<AnimProp>(
        16,
        &mut begin,
        (*layer).anim_props.data,
        0,
        (*layer).anim_props.count,
        |a| (ref_ptr(&(*a).element) as *const Element) < element,
        |a| std::ptr::eq(ref_ptr(&(*a).element), element),
    );

    macro_upper_bound_eq::<AnimProp>(
        16,
        &mut end,
        (*layer).anim_props.data,
        begin,
        (*layer).anim_props.count,
        |a| std::ptr::eq(ref_ptr(&(*a).element), element),
    );

    if begin != end {
        result.data = (*layer).anim_props.data.add(begin);
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
    geom_rot.rotation = (*node).geometry_transform.rotation;
    let geom_rot_mat: Matrix = transform_to_matrix(&geom_rot);

    let mut norm_mat: Matrix = matrix_mul(&raw const (*node).node_to_world, &geom_rot_mat);
    norm_mat = matrix_for_normals(&norm_mat);
    norm_mat
}

// ufbx.c:30827-30830 `ufbx_evaluate_curve`
pub(crate) unsafe fn evaluate_curve(
    curve: *const AnimCurve,
    time: f64,
    default_value: Real,
) -> Real {
    evaluate_curve_flags(curve, time, default_value, 0)
}

// ufbx.c:30832-30914 `ufbx_evaluate_curve_flags`
pub(crate) unsafe fn evaluate_curve_flags(
    curve: *const AnimCurve,
    time: f64,
    default_value: Real,
    flags: u32,
) -> Real {
    if curve.is_null() {
        return default_value;
    }
    if (*curve).keyframes.count <= 1 {
        if (*curve).keyframes.count == 1 {
            return (*(*curve).keyframes.data.add(0)).value;
        } else {
            return default_value;
        }
    }

    if (flags & EvaluateFlags::NO_EXTRAPOLATION.raw()) == 0 {
        if time < (*curve).min_time || time > (*curve).max_time {
            return evaluate::extrapolate_curve(curve, time, flags);
        }
    }

    let mut begin: usize = 0;
    let mut end: usize = (*curve).keyframes.count;
    let keys: *const Keyframe = (*curve).keyframes.data;
    while end - begin >= 8 {
        let mid: usize = (begin + end) >> 1;
        if (*keys.add(mid)).time <= time {
            begin = mid + 1;
        } else {
            end = mid;
        }
    }

    end = (*curve).keyframes.count;
    // C: `for (; begin < end; begin++)` — every switch arm returns, so the
    // increment is only reached through the `continue`.
    while begin < end {
        let next: *const Keyframe = keys.add(begin);
        if (*next).time <= time {
            begin += 1;
            continue;
        }

        // First keyframe
        if begin == 0 {
            return (*next).value;
        }

        let prev: *const Keyframe = next.sub(1);

        // Exact keyframe
        if (*prev).time == time {
            return (*prev).value;
        }

        let rcp_delta: f64 = 1.0 / ((*next).time - (*prev).time);
        let mut t: f64 = (time - (*prev).time) * rcp_delta;

        match (*prev).interpolation {
            Interpolation::ConstantPrev => return (*prev).value,

            Interpolation::ConstantNext => return (*next).value,

            Interpolation::Linear => {
                // C: `return (ufbx_real)(prev->value*(1.0 - t) + next->value*t);`
                // `1.0 - t` is double, so both `value`s promote to double.
                return (as_f64!((*prev).value) * (1.0 - t) + as_f64!((*next).value) * t) as Real;
            }

            Interpolation::Cubic => {
                // C: tangent `dx`/`dy` are float, promoted to double.
                let x1: f64 = (*prev).right.dx as f64 * rcp_delta;
                let x2: f64 = 1.0 - (*next).left.dx as f64 * rcp_delta;
                t = evaluate::find_cubic_bezier_t(x1, x2, t);

                let t2: f64 = t * t;
                let t3: f64 = t2 * t;
                let u: f64 = 1.0 - t;
                let u2: f64 = u * u;
                let u3: f64 = u2 * u;

                // C: `double y0 = prev->value;` — `ufbx_real` promoted to double.
                let y0: f64 = as_f64!((*prev).value);
                let y3: f64 = as_f64!((*next).value);
                let y1: f64 = y0 + (*prev).right.dy as f64;
                let y2: f64 = y3 - (*next).left.dy as f64;

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
    (*(*curve).keyframes.data.add((*curve).keyframes.count - 1)).value
}

// ufbx.c:30916-30919 `ufbx_evaluate_anim_value_real`
#[inline(never)]
pub(crate) unsafe fn evaluate_anim_value_real(anim_value: *const AnimValue, time: f64) -> Real {
    evaluate_anim_value_real_flags(anim_value, time, 0)
}

// ufbx.c:30921-30924 `ufbx_evaluate_anim_value_vec3`
#[inline(never)]
pub(crate) unsafe fn evaluate_anim_value_vec3(anim_value: *const AnimValue, time: f64) -> Vec3 {
    evaluate_anim_value_vec3_flags(anim_value, time, 0)
}

// ufbx.c:30926-30935 `ufbx_evaluate_anim_value_real_flags`
#[inline(never)]
pub(crate) unsafe fn evaluate_anim_value_real_flags(
    anim_value: *const AnimValue,
    time: f64,
    flags: u32,
) -> Real {
    if anim_value.is_null() {
        return 0.0;
    }

    let mut res: Real = (*anim_value).default_value.x;
    // C: `if (anim_value->curves[0]) res = ufbx_evaluate_curve_flags(anim_value->curves[0], time, res, flags);`
    let curve0: *mut AnimCurve = opt_ptr(&raw const (*anim_value).curves[0]);
    if !curve0.is_null() {
        res = evaluate_curve_flags(curve0, time, res, flags);
    }
    res
}

// ufbx.c:30937-30949 `ufbx_evaluate_anim_value_vec3_flags`
#[inline(never)]
pub(crate) unsafe fn evaluate_anim_value_vec3_flags(
    anim_value: *const AnimValue,
    time: f64,
    flags: u32,
) -> Vec3 {
    if anim_value.is_null() {
        // C: `ufbx_vec3 zero = { 0.0f };`
        let zero: Vec3 = Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        return zero;
    }

    let mut res: Vec3 = (*anim_value).default_value;
    let curve0: *mut AnimCurve = opt_ptr(&raw const (*anim_value).curves[0]);
    if !curve0.is_null() {
        res.x = evaluate_curve_flags(curve0, time, res.x, flags);
    }
    let curve1: *mut AnimCurve = opt_ptr(&raw const (*anim_value).curves[1]);
    if !curve1.is_null() {
        res.y = evaluate_curve_flags(curve1, time, res.y, flags);
    }
    let curve2: *mut AnimCurve = opt_ptr(&raw const (*anim_value).curves[2]);
    if !curve2.is_null() {
        res.z = evaluate_curve_flags(curve2, time, res.z, flags);
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
    evaluate_prop_flags_len(anim, element, name, name_len, time, 0)
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
    // `Const` view — legal for any readable provenance (Miri SB, topology finding).
    let props: &View<Props, Const> = View::<Props, Const>::from_ptr(&raw const (*element).props);
    let prop: Option<&View<Prop, Const>> = find_prop_len(props, name, name_len);
    if let Some(found) = prop {
        result = *found.as_ptr();
    } else {
        // C: `memset(&result, 0, sizeof(result));`
        result = MaybeUninit::zeroed().assume_init();
        result.name.data = name;
        result.name.length = name_len;
        result._internal_key = get_name_key(name, name_len);
        result.flags = PropFlags::NOT_FOUND;
        result.value_str.data = EMPTY_CHAR.as_ptr();
        result.value_str.length = 0;
        result.value_blob.data = core::ptr::null();
        result.value_blob.size = 0;
    }

    if (*anim).prop_overrides.count > 0 {
        evaluate::find_prop_override(
            &raw const (*anim).prop_overrides,
            (*element).element_id,
            &mut result,
        );
        return result;
    }

    if (result.flags.raw() & (PropFlags::ANIMATED.raw() | PropFlags::CONNECTED.raw())) == 0 {
        return result;
    }

    // C-parity: `prop->flags` — `prop` is non-NULL here because the NOT_FOUND
    // branch above always takes the early return.
    if (prop.unwrap().flags().raw() & PropFlags::CONNECTED.raw()) != 0
        && !(*anim).ignore_connections
    {
        evaluate::evaluate_connected_prop(
            &mut result,
            anim,
            element,
            prop.unwrap().name().data,
            time,
            flags,
        );
    }

    evaluate::evaluate_props(anim, element, time, &mut result, 1, flags);

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
    evaluate_props_flags(anim, element, time, buffer, buffer_size, 0)
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
    let mut ret: Props = MaybeUninit::zeroed().assume_init();
    if element.is_null() {
        return ret;
    }

    let mut num_anim: usize = 0;
    let mut iter = MaybeUninit::<evaluate::PropIter>::uninit(); // ufbxi_uninit
    let iter: *mut evaluate::PropIter = iter.as_mut_ptr();
    evaluate::init_prop_iter(iter, anim, element);
    // C: `while ((prop = ufbxi_next_prop(&iter)) != NULL)`
    loop {
        let prop: *const Prop = evaluate::next_prop(iter);
        if prop.is_null() {
            break;
        }
        if ((*prop).flags.raw()
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
        let dst: *mut Prop = buffer.add(num_anim);
        num_anim += 1;
        *dst = *prop;

        if ((*prop).flags.raw() & PropFlags::CONNECTED.raw()) != 0 && !(*anim).ignore_connections {
            evaluate::evaluate_connected_prop(dst, anim, element, (*prop).name.data, time, flags);
        }
    }

    evaluate::evaluate_props(anim, element, time, buffer, num_anim, flags);

    ret.props.data = buffer;
    // C: `ret.props.count = ret.num_animated = num_anim;`
    ret.props.count = num_anim;
    ret.num_animated = num_anim;
    // C: `ret.defaults = (ufbx_props*)&element->props;` — raw pointer store
    // into the `Option<Ref<Props>>` slot (same layout).
    *(&raw mut ret.defaults as *mut *const Props) = &raw const (*element).props;
    ret
}

// ufbx.c:31025-31028 `ufbx_evaluate_transform`
#[inline(never)]
pub(crate) unsafe fn evaluate_transform(
    anim: *const Anim,
    node: *const Node,
    time: f64,
) -> Transform {
    evaluate_transform_flags(anim, node, time, 0)
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
        return (*node).local_transform;
    }
    if (*node).is_root {
        return (*node).local_transform;
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

    if !opt_ptr(&raw const (*node).parent).is_null()
        && (flags
            & (TransformFlags::INCLUDE_SCALE.raw() | TransformFlags::INCLUDE_TRANSLATION.raw()))
            != 0
    {
        let parent: *mut Node = opt_ptr(&raw const (*node).parent);

        if (flags & TransformFlags::IGNORE_COMPONENTWISE_SCALE.raw()) == 0
            && !opt_ptr(&raw const (*parent).inherit_scale_node).is_null()
        {
            let mut p: *mut Node = opt_ptr(&raw const (*parent).inherit_scale_node);

            if (*node).is_scale_helper {
                use_scale_factor = true;
            }

            while !p.is_null() && !opt_ptr(&raw const (*p).scale_helper).is_null() {
                // C: `ufbx_prop scale = ufbx_evaluate_prop(anim, &p->scale_helper->element, ufbxi_Lcl_Scaling, time);`
                let scale: Prop = evaluate_prop(
                    anim,
                    &raw const (*opt_ptr(&raw const (*p).scale_helper)).element,
                    sp::Lcl_Scaling.as_ptr(),
                    time,
                );
                // C: `scale.value_vec3.{x,y,z}` — the value union's 3-real view.
                scale_factor.x *= scale.value_vec4.x;
                scale_factor.y *= scale.value_vec4.y;
                scale_factor.z *= scale.value_vec4.z;
                p = opt_ptr(&raw const (*p).inherit_scale_node);
            }
        }

        if !opt_ptr(&raw const (*parent).scale_helper).is_null()
            && (flags & TransformFlags::IGNORE_SCALE_HELPER.raw()) == 0
        {
            helper_scale.write(evaluate_prop(
                anim,
                &raw const (*opt_ptr(&raw const (*parent).scale_helper)).element,
                sp::Lcl_Scaling.as_ptr(),
                time,
            ));
            let hs: *mut Prop = helper_scale.as_mut_ptr();
            if ((*hs).flags.raw() & PropFlags::NOT_FOUND.raw()) != 0 {
                (*hs).value_vec4.x = 1.0;
                (*hs).value_vec4.y = 1.0;
                (*hs).value_vec4.z = 1.0;
            }
            (*hs).value_vec4.x *= scale_factor.x;
            (*hs).value_vec4.y *= scale_factor.y;
            (*hs).value_vec4.z *= scale_factor.z;
            // C: `translation_scale = &helper_scale.value_vec3;`
            translation_scale = &raw const (*hs).value_vec4 as *const Vec3;
        }
    }

    let mut eval_flags: u32 = 0;
    if (flags & TransformFlags::NO_EXTRAPOLATION.raw()) != 0 {
        eval_flags |= EvaluateFlags::NO_EXTRAPOLATION.raw();
    }

    // C: `ufbx_prop buf[ufbxi_arraycount(ufbxi_transform_props_all)]; // ufbxi_uninit`
    let mut buf = MaybeUninit::<[Prop; TRANSFORM_PROPS_ALL_COUNT]>::uninit(); // ufbxi_uninit
    let mut props: Props = evaluate::evaluate_selected_props(
        anim,
        &raw const (*node).element,
        time,
        buf.as_mut_ptr() as *mut Prop,
        prop_names,
        num_prop_names,
        eval_flags,
    );
    // C: `(ufbx_rotation_order)ufbxi_find_enum(...)` — clamped to the valid range.
    let order: RotationOrder = core::mem::transmute::<u32, RotationOrder>(find_enum(
        PropsView::from_ptr(&raw mut props),
        sp::RotationOrder.as_ptr(),
        RotationOrder::Xyz as i64,
        RotationOrder::Spheric as i64,
    ) as u32);

    // C: `ufbx_transform transform; // ufbxi_uninit`
    let mut transform = MaybeUninit::<Transform>::uninit(); // ufbxi_uninit
    let t: *mut Transform = transform.as_mut_ptr();
    if (components & TransformFlags::INCLUDE_TRANSLATION.raw()) != 0 {
        core::ptr::write(
            t,
            get_transform(
                PropsView::from_ptr(&raw mut props),
                order,
                node,
                translation_scale,
            ),
        );
    } else {
        (*t).translation = ZERO_VEC3;
        if (components & TransformFlags::INCLUDE_ROTATION.raw()) != 0 {
            (*t).rotation = get_rotation(PropsView::from_ptr(&raw mut props), order, node);
        } else {
            (*t).rotation = IDENTITY_QUAT;
        }
        if (components & TransformFlags::INCLUDE_SCALE.raw()) != 0 {
            (*t).scale = get_scale(PropsView::from_ptr(&raw mut props), node);
        } else {
            (*t).scale = ONE_VEC3;
        }
    }

    if use_scale_factor {
        (*t).scale.x *= scale_factor.x;
        (*t).scale.y *= scale_factor.y;
        (*t).scale.z *= scale_factor.z;
    }
    transform.assume_init()
}

// ufbx.c:31162-31165 `ufbx_evaluate_blend_weight`
pub(crate) unsafe fn evaluate_blend_weight(
    anim: *const Anim,
    channel: *const BlendChannel,
    time: f64,
) -> Real {
    evaluate_blend_weight_flags(anim, channel, time, 0)
}

// ufbx.c:31167-31176 `ufbx_evaluate_blend_weight_flags`
pub(crate) unsafe fn evaluate_blend_weight_flags(
    anim: *const Anim,
    channel: *const BlendChannel,
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
    let mut props: Props = evaluate::evaluate_selected_props(
        anim,
        &raw const (*channel).element,
        time,
        buf.as_mut_ptr() as *mut Prop,
        prop_names.as_ptr(),
        prop_names.len(),
        flags,
    );
    // C: `ufbxi_find_real(&props, ufbxi_DeformPercent, channel->weight * (ufbx_real)100.0) * (ufbx_real)0.01`
    ufbxi_find_real(
        PropsView::from_ptr(&raw mut props),
        sp::DeformPercent.as_ptr(),
        (*channel).weight * (100.0 as Real),
    ) * (0.01 as Real)
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
    error: *mut Error,
) -> *mut Scene {
    ufbxi_check_opts_ptr!(Scene, opts, error);
    // C: `ufbxi_eval_context ec = { 0 };`
    let ec = evaluate::EvalContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::zeroed()));
    evaluate::evaluate_scene(&ec, scene as *mut Scene, anim, time, opts, error)
}

#[cfg(not(feature = "scene-eval"))]
pub(crate) unsafe fn evaluate_scene(
    scene: *const Scene,
    anim: *const Anim,
    time: f64,
    opts: *const RawEvaluateOpts,
    error: *mut Error,
) -> *mut Scene {
    ufbxi_check_opts_ptr!(Scene, opts, error);
    // C: `scene`/`anim`/`time` are unreferenced in the `#else` arm.
    let _ = (scene, anim, time);
    if !error.is_null() {
        core::ptr::write_bytes(error as *mut u8, 0, size_of::<Error>());
        ufbxi_fmt_err_info!(error, "UFBX_ENABLE_SCENE_EVALUATION");
        ufbxi_report_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            "UFBXI_FEATURE_SCENE_EVALUATION",
            "Feature disabled"
        );
    }
    core::ptr::null_mut()
}

// ufbx.c:31194-31218 `ufbx_create_anim`
// No `#if` fork in C: it drives `ufbxi_create_anim_context` /
// `ufbxi_create_anim_imp` (`native::evaluate`) unconditionally.
pub(crate) unsafe fn create_anim(
    scene: *const Scene,
    opts: *const RawAnimOpts,
    error: *mut Error,
) -> *mut Anim {
    ufbxi_check_opts_ptr!(Anim, opts, error);
    ufbx_assert!(!scene.is_null());

    // C: `ufbxi_create_anim_context ac = { UFBX_ERROR_NONE };`
    let ac =
        evaluate::CreateAnimContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::zeroed()));
    if !opts.is_null() {
        // C: `ac->opts = *opts;` (struct assignment)
        core::ptr::copy_nonoverlapping(opts, ac.opts_mut_ptr(), 1);
    }

    ac.set_scene(scene);

    // C: `int ok = ufbxi_create_anim_imp(&ac);`
    let ok = evaluate::create_anim_imp(&ac);

    if ok.is_ok() {
        clear_error(error);
        let imp: *mut AnimImp = ac.imp();
        &raw mut (*imp).anim
    } else {
        fix_error_type(
            ac.error_mut_ptr(),
            b"Failed to create anim\0".as_ptr(),
            error,
        );
        buf_free(ac.result_mut_ptr());
        free_ator(ac.ator_result_mut_ptr());
        core::ptr::null_mut()
    }
}

// ufbx.c:31220-31229 `ufbx_free_anim`
pub(crate) unsafe fn free_anim(anim: *mut Anim) {
    if anim.is_null() {
        return;
    }
    if !(*anim).custom {
        return;
    }

    let imp: *mut AnimImp = get_imp(anim as *mut c_void);
    ufbx_assert!((*imp).magic == ANIM_IMP_MAGIC);
    if (*imp).magic != ANIM_IMP_MAGIC {
        return;
    }
    release_ref(&raw mut (*imp).refcount);
}

// ufbx.c:31231-31240 `ufbx_retain_anim`
pub(crate) unsafe fn retain_anim(anim: *mut Anim) {
    if anim.is_null() {
        return;
    }
    if !(*anim).custom {
        return;
    }

    let imp: *mut AnimImp = get_imp(anim as *mut c_void);
    ufbx_assert!((*imp).magic == ANIM_IMP_MAGIC);
    if (*imp).magic != ANIM_IMP_MAGIC {
        return;
    }
    retain_ref(&raw mut (*imp).refcount);
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
    error: *mut Error,
) -> *mut BakedAnim {
    ufbx_assert!(!scene.is_null());
    ufbxi_check_opts_ptr!(BakedAnim, opts, error);
    let mut anim = anim;
    if anim.is_null() {
        anim = ref_ptr(&raw const (*scene).anim);
    }

    // C: `ufbxi_bake_context bc = { UFBX_ERROR_NONE };`
    let bc = evaluate::BakeContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::zeroed()));
    if !opts.is_null() {
        // C: `bc->opts = *opts;` (struct assignment)
        core::ptr::copy_nonoverlapping(opts, bc.opts_mut_ptr(), 1);
    }

    bc.set_scene(scene);

    // C: `int ok = ufbxi_bake_anim_imp(&bc, anim);`
    let ok = evaluate::bake_anim_imp(&bc, anim);

    buf_free(bc.tmp_mut_ptr());
    buf_free(bc.tmp_prop_mut_ptr());
    buf_free(bc.tmp_times_mut_ptr());
    buf_free(bc.tmp_bake_props_mut_ptr());
    buf_free(bc.tmp_nodes_mut_ptr());
    buf_free(bc.tmp_elements_mut_ptr());
    buf_free(bc.tmp_props_mut_ptr());
    buf_free(bc.tmp_bake_stack_mut_ptr());
    // C: `ufbxi_free(&bc->ator_tmp, char, bc->tmp_arr, bc->tmp_arr_size);`
    free::<u8>(bc.ator_tmp_mut_ptr(), bc.tmp_arr(), bc.tmp_arr_size());
    free_ator(bc.ator_tmp_mut_ptr());

    if ok.is_ok() {
        clear_error(error);
        let imp: *mut BakedAnimImp = bc.imp();
        &raw mut (*imp).bake
    } else {
        fix_error_type(bc.error_mut_ptr(), b"Failed to bake anim\0".as_ptr(), error);
        buf_free(bc.result_mut_ptr());
        free_ator(bc.ator_result_mut_ptr());
        core::ptr::null_mut()
    }
}

#[cfg(not(feature = "baking"))]
pub(crate) unsafe fn bake_anim(
    scene: *const Scene,
    anim: *const Anim,
    opts: *const RawBakeOpts,
    error: *mut Error,
) -> *mut BakedAnim {
    ufbx_assert!(!scene.is_null());
    // C: `anim`/`opts` are unreferenced in the `#else` arm.
    let _ = (anim, opts);
    if !error.is_null() {
        core::ptr::write_bytes(error as *mut u8, 0, size_of::<Error>());
        ufbxi_fmt_err_info!(error, "UFBX_ENABLE_ANIMATION_BAKING");
        ufbxi_report_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            "UFBXI_FEATURE_ANIMATION_BAKING",
            "Feature disabled"
        );
    }
    core::ptr::null_mut()
}

// ufbx.c:31291-31299 `ufbx_retain_baked_anim`
// `ufbxi_baked_anim_imp` is declared outside C's baking `#if`, so this pair
// works in every build (see `native::evaluate`).
pub(crate) unsafe fn retain_baked_anim(bake: *mut BakedAnim) {
    if bake.is_null() {
        return;
    }

    let imp: *mut BakedAnimImp = get_imp(bake as *mut c_void);
    ufbx_assert!((*imp).magic == BAKED_ANIM_IMP_MAGIC);
    if (*imp).magic != BAKED_ANIM_IMP_MAGIC {
        return;
    }
    retain_ref(&raw mut (*imp).refcount);
}

// ufbx.c:31301-31309 `ufbx_free_baked_anim`
pub(crate) unsafe fn free_baked_anim(bake: *mut BakedAnim) {
    if bake.is_null() {
        return;
    }

    let imp: *mut BakedAnimImp = get_imp(bake as *mut c_void);
    ufbx_assert!((*imp).magic == BAKED_ANIM_IMP_MAGIC);
    if (*imp).magic != BAKED_ANIM_IMP_MAGIC {
        return;
    }
    release_ref(&raw mut (*imp).refcount);
}

// ufbx.c:31312-31318 `ufbx_find_baked_node_by_typed_id`
// C-parity: no null check on `bake` — the C body dereferences it directly.
pub(crate) unsafe fn find_baked_node_by_typed_id(
    bake: *mut BakedAnim,
    typed_id: u32,
) -> *mut BakedNode {
    let mut index: usize = usize::MAX;
    macro_lower_bound_eq::<BakedNode>(
        8,
        &mut index,
        (*bake).nodes.data,
        0,
        (*bake).nodes.count,
        |a| (*a).typed_id < typed_id,
        |a| (*a).typed_id == typed_id,
    );
    if index < usize::MAX {
        (*bake).nodes.data.add(index) as *mut BakedNode
    } else {
        core::ptr::null_mut()
    }
}

// ufbx.c:31320-31324 `ufbx_find_baked_node`
pub(crate) unsafe fn find_baked_node(bake: *mut BakedAnim, node: *mut Node) -> *mut BakedNode {
    if bake.is_null() || node.is_null() {
        return core::ptr::null_mut();
    }
    find_baked_node_by_typed_id(bake, (*node).element.typed_id)
}

// ufbx.c:31326-31332 `ufbx_find_baked_element_by_element_id`
// C-parity: no null check on `bake`, as above.
pub(crate) unsafe fn find_baked_element_by_element_id(
    bake: *mut BakedAnim,
    element_id: u32,
) -> *mut BakedElement {
    let mut index: usize = usize::MAX;
    macro_lower_bound_eq::<BakedElement>(
        8,
        &mut index,
        (*bake).elements.data,
        0,
        (*bake).elements.count,
        |a| (*a).element_id < element_id,
        |a| (*a).element_id == element_id,
    );
    if index < usize::MAX {
        (*bake).elements.data.add(index) as *mut BakedElement
    } else {
        core::ptr::null_mut()
    }
}

// ufbx.c:31334-31338 `ufbx_find_baked_element`
pub(crate) unsafe fn find_baked_element(
    bake: *mut BakedAnim,
    element: *mut Element,
) -> *mut BakedElement {
    if bake.is_null() || element.is_null() {
        return core::ptr::null_mut();
    }
    find_baked_element_by_element_id(bake, (*element).element_id)
}

// ufbx.c:31340-31370 `ufbx_evaluate_baked_vec3`
// C-parity: the trailing `keyframes.data[keyframes.count - 1]` is an
// out-of-bounds read for an empty list (`count - 1` wraps to `SIZE_MAX`), and
// `keys` is dereferenced without a null check — ported as pointer arithmetic
// so the Rust behaves the way the C does instead of panicking earlier.
pub(crate) unsafe fn evaluate_baked_vec3(keyframes: List<BakedVec3>, time: f64) -> Vec3 {
    let mut begin: usize = 0;
    let mut end: usize = keyframes.count;
    let keys: *const BakedVec3 = keyframes.data;
    while end - begin >= 8 {
        let mid: usize = (begin + end) >> 1;
        if (*keys.add(mid)).time <= time {
            begin = mid + 1;
        } else {
            end = mid;
        }
    }

    end = keyframes.count;
    // C: `for (; begin < end; begin++)` — every path out of the body either
    // `continue`s (the only one that advances `begin`) or returns.
    while begin < end {
        let next: *const BakedVec3 = keys.add(begin);
        if (*next).time <= time {
            begin += 1;
            continue;
        }
        if begin == 0 {
            return (*next).value;
        }

        let mut prev: *const BakedVec3 = next.sub(1);
        if prev > keys
            && ((*prev).flags & BakedKeyFlags::STEP_RIGHT).any()
            && (*prev.sub(1)).time == time
        {
            prev = prev.sub(1);
        }
        if time == (*prev).time {
            return (*prev).value;
        }
        let mut t: f64 = (time - (*prev).time) / ((*next).time - (*prev).time);
        if ((*prev).flags & BakedKeyFlags::STEP_LEFT).any() {
            t = 0.0;
        }
        if ((*next).flags & BakedKeyFlags::STEP_RIGHT).any() {
            t = 1.0;
        }
        return lerp3((*prev).value, (*next).value, t as Real);
    }

    (*keyframes.data.add(keyframes.count.wrapping_sub(1))).value
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
        if (*keys.add(mid)).time <= time {
            begin = mid + 1;
        } else {
            end = mid;
        }
    }

    end = keyframes.count;
    while begin < end {
        let next: *const BakedQuat = keys.add(begin);
        if (*next).time <= time {
            begin += 1;
            continue;
        }
        if begin == 0 {
            return (*next).value;
        }

        let mut prev: *const BakedQuat = next.sub(1);
        if prev > keys && (*prev.sub(1)).time == time {
            prev = prev.sub(1);
        }
        if time == (*prev).time {
            return (*prev).value;
        }
        let mut t: f64 = (time - (*prev).time) / ((*next).time - (*prev).time);
        if prev > keys
            && ((*prev).flags & BakedKeyFlags::STEP_RIGHT).any()
            && (*prev.sub(1)).time == time
        {
            prev = prev.sub(1);
        }
        if ((*prev).flags & BakedKeyFlags::STEP_LEFT).any() {
            t = 0.0;
        }
        if ((*next).flags & BakedKeyFlags::STEP_RIGHT).any() {
            t = 1.0;
        }
        return quat_slerp((*prev).value, (*next).value, t as Real);
    }

    (*keyframes.data.add(keyframes.count.wrapping_sub(1))).value
}

// ufbx.c:31405-31412 `ufbx_get_bone_pose`
// Was ported ahead of this unit because `ufbxi_update_pose` (ufbx.c:23271,
// `native::scene_process`) calls it.
pub(crate) unsafe fn get_bone_pose(pose: *const Pose, node: *const Node) -> *mut BonePose {
    if pose.is_null() || node.is_null() {
        return core::ptr::null_mut();
    }
    let mut index: usize = usize::MAX;
    macro_lower_bound_eq::<BonePose>(
        8,
        &mut index,
        (*pose).bone_poses.data,
        0,
        (*pose).bone_poses.count,
        |a| (*ref_ptr(&(*a).bone_node)).element.typed_id < (*node).element.typed_id,
        |a| std::ptr::eq(ref_ptr(&(*a).bone_node), node),
    );
    if index < usize::MAX {
        (*pose).bone_poses.data.add(index) as *mut BonePose
    } else {
        core::ptr::null_mut()
    }
}

// ufbx.c:31414-31423 `ufbx_find_prop_texture_len`
pub(crate) unsafe fn find_prop_texture_len(
    material: *const Material,
    name: *const u8,
    name_len: usize,
) -> *mut Texture {
    let name_str: String = safe_string(name, name_len);
    if material.is_null() {
        return core::ptr::null_mut();
    }

    let mut index: usize = usize::MAX;
    macro_lower_bound_eq::<MaterialTexture>(
        4,
        &mut index,
        (*material).textures.data,
        0,
        (*material).textures.count,
        |a| str_less((*a).material_prop, name_str),
        |a| str_equal((*a).material_prop, name_str),
    );
    if index < usize::MAX {
        ref_ptr(&(*(*material).textures.data.add(index)).texture)
    } else {
        core::ptr::null_mut()
    }
}

// ufbx.c:31425-31432 `ufbx_find_shader_prop_len`
pub(crate) unsafe fn find_shader_prop_len(
    shader: *const Shader,
    name: *const u8,
    name_len: usize,
) -> String {
    let bindings: List<ShaderPropBinding> = find_shader_prop_bindings_len(shader, name, name_len);
    if bindings.count > 0 {
        return (*bindings.data).material_prop;
    }
    EMPTY_STRING.0
}

// ufbx.c:31434-31461 `ufbx_find_shader_prop_bindings_len`
pub(crate) unsafe fn find_shader_prop_bindings_len(
    shader: *const Shader,
    name: *const u8,
    name_len: usize,
) -> List<ShaderPropBinding> {
    // C: `ufbx_shader_prop_binding_list bindings = { NULL, 0 };` — `List<T>`
    // carries a private `PhantomData` marker, so the C aggregate initializer
    // becomes a zeroed value with both public fields written (same shape as
    // `native::scene_process::find_dst_connections`).
    let mut bindings: List<ShaderPropBinding> = MaybeUninit::zeroed().assume_init();
    bindings.data = core::ptr::null();
    bindings.count = 0;

    let name_str: String = safe_string(name, name_len);
    if shader.is_null() {
        return bindings;
    }

    // C: `ufbxi_for_ptr_list(ufbx_shader_binding, p_bind, shader->bindings)`
    let mut p_bind: *mut *mut ShaderBinding = (*shader).bindings.data as *mut *mut ShaderBinding;
    let p_bind_end: *mut *mut ShaderBinding = add_ptr(p_bind, (*shader).bindings.count);
    while p_bind != p_bind_end {
        let bind: *mut ShaderBinding = *p_bind;

        let mut begin: usize = usize::MAX;
        macro_lower_bound_eq::<ShaderPropBinding>(
            4,
            &mut begin,
            (*bind).prop_bindings.data,
            0,
            (*bind).prop_bindings.count,
            |a| str_less((*a).shader_prop, name_str),
            |a| str_equal((*a).shader_prop, name_str),
        );

        if begin != usize::MAX {
            let mut end: usize = begin;
            macro_upper_bound_eq::<ShaderPropBinding>(
                4,
                &mut end,
                (*bind).prop_bindings.data,
                begin,
                (*bind).prop_bindings.count,
                |a| str_equal((*a).shader_prop, name_str),
            );

            bindings.data = (*bind).prop_bindings.data.add(begin);
            bindings.count = end - begin;
            break;
        }
        p_bind = p_bind.add(1);
    }

    bindings
}

// ufbx.c:31463-31476 `ufbx_find_shader_texture_input_len`
pub(crate) unsafe fn find_shader_texture_input_len(
    shader: *const ShaderTexture,
    name: *const u8,
    name_len: usize,
) -> *mut ShaderTextureInput {
    let name_str: String = safe_string(name, name_len);

    let mut index: usize = usize::MAX;
    macro_lower_bound_eq::<ShaderTextureInput>(
        4,
        &mut index,
        (*shader).inputs.data,
        0,
        (*shader).inputs.count,
        |a| str_less((*a).name, name_str),
        |a| str_equal((*a).name, name_str),
    );

    if index != usize::MAX {
        return (*shader).inputs.data.add(index) as *mut ShaderTextureInput;
    }

    core::ptr::null_mut()
}

// ufbx.c:31478-31490 `ufbx_coordinate_axes_valid`
// Ported ahead of its banner section because `ufbxi_update_adjust_transforms`
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
// Was ported ahead of this unit because `ufbxi_mul_rotate` and friends
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
// Ported ahead of its banner section because `ufbxi_mul_rotate` /
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
// Was ported ahead of its own unit because `ufbxi_update_node`
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
    let mut dst: Matrix = core::mem::zeroed();

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

    dst
}

// ufbx.c:31749-31754 `ufbx_matrix_determinant`
// Ported ahead of its banner section because `ufbx_matrix_for_normals` below
// needs it.
pub(crate) unsafe fn matrix_determinant(m: *const Matrix) -> Real {
    -(*m).m02 * (*m).m11 * (*m).m20
        + (*m).m01 * (*m).m12 * (*m).m20
        + (*m).m02 * (*m).m10 * (*m).m21
        - (*m).m00 * (*m).m12 * (*m).m21
        - (*m).m01 * (*m).m10 * (*m).m22
        + (*m).m00 * (*m).m11 * (*m).m22
}

// ufbx.c:31756-31782 `ufbx_matrix_invert`
// Ported ahead of its banner section because `ufbxi_update_pose`
// (ufbx.c:23271, `native::scene_process`) calls it.
pub(crate) unsafe fn matrix_invert(m: *const Matrix) -> Matrix {
    let det: Real = matrix_determinant(m);

    // C: `ufbx_matrix r;` — the early-out arm `memset`s it and the fall-through
    // arm writes every field, so the zero-fill is inert (upstream carries no
    // `// ufbxi_uninit` marker).
    let mut r: Matrix = core::mem::zeroed();
    // C: `ufbx_fabs(det) <= UFBX_EPSILON` — `det` promotes to double at the
    // call and `UFBX_EPSILON` (ufbx_real) promotes for the comparison.
    if math::fabs(det as f64) <= as_f64!(math::EPSILON) {
        // C: `memset(&r, 0, sizeof(r));`
        r = core::mem::zeroed();
        return r;
    }

    let rcp_det: Real = 1.0 / det;

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

    r
}

// ufbx.c:31784-31802 `ufbx_matrix_for_normals`
// Ported ahead of its banner section because `ufbxi_modify_geometry`
// (ufbx.c:21165, `native::scene_process`) calls it.
#[inline(never)]
pub(crate) unsafe fn matrix_for_normals(m: *const Matrix) -> Matrix {
    let det: Real = matrix_determinant(m);
    let det_sign: Real = if det >= 0.0 { 1.0 } else { -1.0 };

    // C: `ufbx_matrix r;` — every field is written below before the return.
    let mut r: Matrix = core::mem::zeroed();
    r.m00 = (-(*m).m12 * (*m).m21 + (*m).m11 * (*m).m22) * det_sign;
    r.m01 = ((*m).m12 * (*m).m20 - (*m).m10 * (*m).m22) * det_sign;
    r.m02 = (-(*m).m11 * (*m).m20 + (*m).m10 * (*m).m21) * det_sign;
    r.m10 = ((*m).m02 * (*m).m21 - (*m).m01 * (*m).m22) * det_sign;
    r.m11 = (-(*m).m02 * (*m).m20 + (*m).m00 * (*m).m22) * det_sign;
    r.m12 = ((*m).m01 * (*m).m20 - (*m).m00 * (*m).m21) * det_sign;
    r.m20 = (-(*m).m02 * (*m).m11 + (*m).m01 * (*m).m12) * det_sign;
    r.m21 = ((*m).m02 * (*m).m10 - (*m).m00 * (*m).m12) * det_sign;
    r.m22 = (-(*m).m01 * (*m).m10 + (*m).m00 * (*m).m11) * det_sign;
    // C: `r.m03 = r.m13 = r.m23 = 0.0f;`
    r.m23 = 0.0;
    r.m13 = r.m23;
    r.m03 = r.m13;

    r
}

// ufbx.c:31804-31814 `ufbx_transform_position`
// Ported ahead of its banner section because `ufbxi_transform_vec3_list`
// (ufbx.c:21049, `native::scene_process`) calls it.
#[inline(never)]
pub(crate) unsafe fn transform_position(m: *const Matrix, v: Vec3) -> Vec3 {
    ufbx_assert!(!m.is_null());
    if m.is_null() {
        return ZERO_VEC3;
    }

    // C: `ufbx_vec3 r;` — every field is written below before the return,
    // so the zero-fill is inert (upstream carries no `// ufbxi_uninit` marker).
    let mut r: Vec3 = core::mem::zeroed();
    r.x = (*m).m00 * v.x + (*m).m01 * v.y + (*m).m02 * v.z + (*m).m03;
    r.y = (*m).m10 * v.x + (*m).m11 * v.y + (*m).m12 * v.z + (*m).m13;
    r.z = (*m).m20 * v.x + (*m).m21 * v.y + (*m).m22 * v.z + (*m).m23;
    r
}

// ufbx.c:31816-31826 `ufbx_transform_direction`
// Ported ahead of its banner section because `ufbxi_update_adjust_transforms`
// (ufbx.c:23705, `native::scene_process`) calls it.
#[inline(never)]
pub(crate) unsafe fn transform_direction(m: *const Matrix, v: Vec3) -> Vec3 {
    ufbx_assert!(!m.is_null());
    if m.is_null() {
        return ZERO_VEC3;
    }

    // C: `ufbx_vec3 r;` — every field is written below before the return,
    // so the zero-fill is inert (upstream carries no `// ufbxi_uninit` marker).
    let mut r: Vec3 = core::mem::zeroed();
    r.x = (*m).m00 * v.x + (*m).m01 * v.y + (*m).m02 * v.z;
    r.y = (*m).m10 * v.x + (*m).m11 * v.y + (*m).m12 * v.z;
    r.z = (*m).m20 * v.x + (*m).m21 * v.y + (*m).m22 * v.z;
    r
}

// ufbx.c:31828-31852 `ufbx_transform_to_matrix`
#[inline(never)]
pub(crate) unsafe fn transform_to_matrix(t: *const Transform) -> Matrix {
    ufbx_assert!(!t.is_null());
    if t.is_null() {
        return IDENTITY_MATRIX;
    }

    let q: Quat = (*t).rotation;
    let sx: Real = 2.0 * (*t).scale.x;
    let sy: Real = 2.0 * (*t).scale.y;
    let sz: Real = 2.0 * (*t).scale.z;
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
    let mut m: Matrix = core::mem::zeroed();
    m.m00 = sx * (-yy - zz + 0.5);
    m.m10 = sx * (xy + zw);
    m.m20 = sx * (-yw + xz);
    m.m01 = sy * (-zw + xy);
    m.m11 = sy * (-xx - zz + 0.5);
    m.m21 = sy * (xw + yz);
    m.m02 = sz * (xz + yw);
    m.m12 = sz * (-xw + yz);
    m.m22 = sz * (-xx - yy + 0.5);
    m.m03 = (*t).translation.x;
    m.m13 = (*t).translation.y;
    m.m23 = (*t).translation.z;
    m
}

// ufbx.c:31854-31926 `ufbx_matrix_to_transform`
// Ported ahead of its banner section because `ufbxi_update_skin_cluster`
// (ufbx.c:23289, `native::scene_process`) calls it.
#[inline(never)]
pub(crate) unsafe fn matrix_to_transform(m: *const Matrix) -> Transform {
    ufbx_assert!(!m.is_null());
    if m.is_null() {
        return IDENTITY_TRANSFORM;
    }

    let det: Real = matrix_determinant(m);

    // C indexes the `ufbx_matrix` value union's `ufbx_vec3 cols[4]` view; the
    // generated struct keeps only the `m00`..`m23` scalars, so the index is
    // pointer arithmetic from the struct base (same device as
    // `native::scene_process::add_weighted_matrix`).
    let m_cols: *const Vec3 = m as *const Vec3;

    // C: `ufbx_transform t;` — every member is written below before the return,
    // so the zero-fill is inert (upstream carries no `// ufbxi_uninit` marker).
    let mut t: Transform = core::mem::zeroed();
    t.translation = *m_cols.add(3);
    t.scale.x = length3(*m_cols.add(0));
    t.scale.y = length3(*m_cols.add(1));
    t.scale.z = length3(*m_cols.add(2));

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

    let x: Vec3 = mul3(
        *m_cols.add(0),
        if t.scale.x > 0.0 {
            sign_x / t.scale.x
        } else {
            0.0
        },
    );
    let y: Vec3 = mul3(
        *m_cols.add(1),
        if t.scale.y > 0.0 {
            sign_y / t.scale.y
        } else {
            0.0
        },
    );
    let z: Vec3 = mul3(
        *m_cols.add(2),
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
pub(crate) unsafe fn catch_get_skin_vertex_matrix(
    panic: *mut Panic,
    skin: *const SkinDeformer,
    vertex: usize,
    fallback: *const Matrix,
) -> Matrix {
    ufbx_assert!(!skin.is_null());
    // C-parity: the panic guard dereferences `skin` BEFORE the `!skin` test on
    // the next line — keep the order (a null `skin` is already an assert
    // violation above).
    if ufbxi_panicf!(
        panic,
        vertex < (*skin).vertices.count,
        "vertex (%zu) out of bounds (%zu)",
        vertex,
        (*skin).vertices.count,
    ) {
        return IDENTITY_MATRIX;
    }

    if skin.is_null() || vertex >= (*skin).vertices.count {
        return IDENTITY_MATRIX;
    }
    let skin_vertex: SkinVertex = *(*skin).vertices.data.add(vertex);

    // C: `ufbx_matrix mat = { 0.0f };` / `ufbx_quat q0 = { 0.0f }, qe = { 0.0f };`
    // / `ufbx_quat first_q0 = { 0.0f };` — partial initializers zero the rest.
    let mut mat: Matrix = core::mem::zeroed();
    let mut q0: Quat = core::mem::zeroed();
    let mut qe: Quat = core::mem::zeroed();
    let mut first_q0: Quat = core::mem::zeroed();
    let mut qs: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let mut total_weight: Real = 0.0;

    for i in 0..skin_vertex.num_weights {
        // C: `skin->weights.data[skin_vertex.weight_begin + i]` — `uint32_t`
        // arithmetic, so the sum wraps before it indexes.
        let weight: SkinWeight = *(*skin)
            .weights
            .data
            .add(skin_vertex.weight_begin.wrapping_add(i) as usize);
        let cluster: *mut SkinCluster =
            *((*skin).clusters.data as *const *mut SkinCluster).add(weight.cluster_index as usize);
        // C: `const ufbx_node *node = cluster->bone_node; if (!node) continue;`
        let node: *const Node = opt_ptr(&(*cluster).bone_node);
        if node.is_null() {
            continue;
        }

        total_weight += weight.weight;
        if skin_vertex.dq_weight > 0.0 {
            let t: Transform = (*cluster).geometry_to_world_transform;
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
            add_weighted_quat(&mut q0, vq0, weight.weight);
            add_weighted_quat(&mut qe, vqe, weight.weight);
            add_weighted_vec3(&mut qs, t.scale, weight.weight);
        }

        if skin_vertex.dq_weight < 1.0 {
            add_weighted_mat(
                &mut mat,
                &(*cluster).geometry_to_world,
                (1.0 - skin_vertex.dq_weight) * weight.weight,
            );
        }
    }

    if total_weight <= 0.0 {
        if !fallback.is_null() {
            return *fallback;
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
        let mut dqt: Transform = core::mem::zeroed(); // ufbxi_uninit
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
        let dqm: Matrix = transform_to_matrix(&dqt);
        if skin_vertex.dq_weight < 1.0 {
            add_weighted_mat(&mut mat, &dqm, skin_vertex.dq_weight);
        } else {
            mat = dqm;
        }
    }

    mat
}

// ufbx.c:32020-32033 `ufbx_get_blend_shape_offset_index`
#[inline(never)]
pub(crate) unsafe fn get_blend_shape_offset_index(shape: *const BlendShape, vertex: usize) -> u32 {
    ufbx_assert!(!shape.is_null());
    if shape.is_null() {
        return NO_INDEX;
    }

    let mut index: usize = usize::MAX;
    let vertex_ix: u32 = vertex as u32;

    macro_lower_bound_eq::<u32>(
        16,
        &mut index,
        (*shape).offset_vertices.data,
        0,
        (*shape).num_offsets,
        |a| *a < vertex_ix,
        |a| *a == vertex_ix,
    );
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
    let index: u32 = get_blend_shape_offset_index(shape, vertex);
    if index == NO_INDEX {
        return ZERO_VEC3;
    }
    *(*shape).position_offsets.data.add(index as usize)
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
    let mut p_chan: *mut *mut BlendChannel = (*blend).channels.data as *mut *mut BlendChannel;
    let p_chan_end: *mut *mut BlendChannel = add_ptr(p_chan, (*blend).channels.count);
    while p_chan != p_chan_end {
        let chan: *mut BlendChannel = *p_chan;
        // C: `ufbxi_for_list(ufbx_blend_keyframe, key, chan->keyframes)` —
        // indexed here because the body `continue`s (the C `for` advances the
        // iterator in its increment clause).
        for key_ix in 0..(*chan).keyframes.count {
            let key: *mut BlendKeyframe =
                ((*chan).keyframes.data as *mut BlendKeyframe).add(key_ix);
            if (*key).effective_weight == 0.0 {
                continue;
            }

            let key_offset: Vec3 = get_blend_shape_vertex_offset(ref_ptr(&(*key).shape), vertex);
            add_weighted_vec3(&mut offset, key_offset, (*key).effective_weight);
        }
        p_chan = p_chan.add(1);
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

    let num_offsets: usize = (*shape).num_offsets;
    let vertex_indices: *const u32 = (*shape).offset_vertices.data;
    let offsets: *const Vec3 = (*shape).position_offsets.data;
    let weights_data: *const Real = (*shape).offset_weights.data;
    let weights_count: usize = (*shape).offset_weights.count;
    for i in 0..num_offsets {
        let index: u32 = *vertex_indices.add(i);
        // C: `index < num_vertices` — `uint32_t` widens to `size_t`.
        if (index as usize) < num_vertices {
            let mut vertex_weight: Real = weight;
            if i < weights_count {
                vertex_weight *= *weights_data.add(i);
            }
            add_weighted_vec3(vertices.add(index as usize), *offsets.add(i), vertex_weight);
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
    let mut p_chan: *mut *mut BlendChannel = (*blend).channels.data as *mut *mut BlendChannel;
    let p_chan_end: *mut *mut BlendChannel = add_ptr(p_chan, (*blend).channels.count);
    while p_chan != p_chan_end {
        let chan: *mut BlendChannel = *p_chan;
        // C: `ufbxi_for_list(ufbx_blend_keyframe, key, chan->keyframes)` —
        // indexed here because the body `continue`s (the C `for` advances the
        // iterator in its increment clause).
        for key_ix in 0..(*chan).keyframes.count {
            let key: *mut BlendKeyframe =
                ((*chan).keyframes.data as *mut BlendKeyframe).add(key_ix);
            if (*key).effective_weight == 0.0 {
                continue;
            }
            add_blend_shape_vertex_offsets(
                ref_ptr(&(*key).shape),
                vertices,
                num_vertices,
                weight * (*key).effective_weight,
            );
        }
        p_chan = p_chan.add(1);
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
    if (*basis).order == 0 {
        return usize::MAX;
    }
    if !(*basis).valid {
        return usize::MAX;
    }

    let degree: usize = ((*basis).order - 1) as usize;
    ufbx_assert!(degree >= 1);

    // Binary search for the knot span `[min_u, max_u]` where `min_u <= u < max_u`
    // C: `ufbx_real_list knots = basis->knot_vector;` — a by-value list copy;
    // `List` is not `Copy`, so read through a pointer to the same data.
    let knots: *const List<Real> = &(*basis).knot_vector;
    let mut knot: usize = usize::MAX;

    if u <= (*basis).t_min {
        knot = degree;
        u = (*basis).t_min;
    } else if u >= (*basis).t_max {
        knot = (*basis)
            .knot_vector
            .count
            .wrapping_sub(degree)
            .wrapping_sub(2);
        u = (*basis).t_max;
    } else {
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
    if num_weights < (*basis).order as usize {
        return knot - degree;
    }
    if weights.is_null() {
        return knot - degree;
    }

    *weights.add(0) = 1.0f32 as Real;
    for p in 1..=degree {
        let mut prev: Real = 0.0f32 as Real;
        let mut g: Real = 1.0f32 as Real - nurbs_weight(knots, knot - p + 1, p, u);
        let mut dg: Real = 0.0f32 as Real;
        if !derivatives.is_null() && p == degree {
            dg = nurbs_deriv(knots, knot - p + 1, p);
        }

        // C: `for (size_t i = p; i > 0; i--)`
        let mut i: usize = p;
        while i > 0 {
            let f: Real = nurbs_weight(knots, knot - p + i, p, u);
            let weight: Real = *weights.add(i - 1);
            *weights.add(i) = f * weight + g * prev;

            if !derivatives.is_null() && p == degree {
                let df: Real = nurbs_deriv(knots, knot - p + i, p);
                if i < num_derivatives {
                    *derivatives.add(i) = df * weight - dg * prev;
                }
                dg = df;
            }

            prev = weight;
            g = 1.0f32 as Real - f;
            i -= 1;
        }

        *weights.add(0) = g * prev;
        if !derivatives.is_null() && p == degree {
            *derivatives.add(0) = -dg * prev;
        }
    }

    knot - degree
}

// ufbx.c:32168-32212 `ufbx_evaluate_nurbs_curve`
#[inline(never)]
pub(crate) unsafe fn evaluate_nurbs_curve(curve: *const NurbsCurve, u: Real) -> CurvePoint {
    // C: `ufbx_curve_point result = { false };`
    let mut result: CurvePoint = core::mem::zeroed();

    ufbx_assert!(!curve.is_null());
    if curve.is_null() {
        return result;
    }

    let mut weights: [Real; MAX_NURBS_ORDER] = core::mem::zeroed(); // ufbxi_uninit
    let mut derivs: [Real; MAX_NURBS_ORDER] = core::mem::zeroed(); // ufbxi_uninit
    let base: usize = evaluate_nurbs_basis(
        &(*curve).basis,
        u,
        weights.as_mut_ptr(),
        MAX_NURBS_ORDER,
        derivs.as_mut_ptr(),
        MAX_NURBS_ORDER,
    );
    if base == usize::MAX {
        return result;
    }

    let mut p: Vec4 = core::mem::zeroed();
    let mut d: Vec4 = core::mem::zeroed();

    let order: usize = (*curve).basis.order as usize;
    if order > MAX_NURBS_ORDER {
        return result;
    }
    if (*curve).control_points.count == 0 {
        return result;
    }

    for i in 0..order {
        let ix: usize = base.wrapping_add(i) % (*curve).control_points.count;
        let cp: Vec4 = *(*curve).control_points.data.add(ix);
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
    let mut result: SurfacePoint = core::mem::zeroed();

    ufbx_assert!(!surface.is_null());
    if surface.is_null() {
        return result;
    }

    let mut weights_u: [Real; MAX_NURBS_ORDER] = core::mem::zeroed(); // ufbxi_uninit
    let mut weights_v: [Real; MAX_NURBS_ORDER] = core::mem::zeroed(); // ufbxi_uninit
    let mut derivs_u: [Real; MAX_NURBS_ORDER] = core::mem::zeroed(); // ufbxi_uninit
    let mut derivs_v: [Real; MAX_NURBS_ORDER] = core::mem::zeroed(); // ufbxi_uninit
    let base_u: usize = evaluate_nurbs_basis(
        &(*surface).basis_u,
        u,
        weights_u.as_mut_ptr(),
        MAX_NURBS_ORDER,
        derivs_u.as_mut_ptr(),
        MAX_NURBS_ORDER,
    );
    let base_v: usize = evaluate_nurbs_basis(
        &(*surface).basis_v,
        v,
        weights_v.as_mut_ptr(),
        MAX_NURBS_ORDER,
        derivs_v.as_mut_ptr(),
        MAX_NURBS_ORDER,
    );
    if base_u == usize::MAX || base_v == usize::MAX {
        return result;
    }

    let mut p: Vec4 = core::mem::zeroed();
    let mut du: Vec4 = core::mem::zeroed();
    let mut dv: Vec4 = core::mem::zeroed();

    let num_u: usize = (*surface).num_control_points_u;
    let num_v: usize = (*surface).num_control_points_v;
    let order_u: usize = (*surface).basis_u.order as usize;
    let order_v: usize = (*surface).basis_v.order as usize;
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
            let cp: Vec4 = *(*surface)
                .control_points
                .data
                .add(vix.wrapping_mul(num_u).wrapping_add(uix));

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
    error: *mut Error,
) -> *mut LineCurve {
    ufbxi_check_opts_ptr!(LineCurve, opts, error);
    ufbx_assert!(!curve.is_null());
    if curve.is_null() {
        return core::ptr::null_mut();
    }

    // C: `ufbxi_tessellate_curve_context tc = { UFBX_ERROR_NONE };`
    let tc = TessellateCurveContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::zeroed()));
    if !opts.is_null() {
        // C: `tc->opts = *opts` — struct assignment (memcpy).
        core::ptr::copy_nonoverlapping(opts, tc.opts_mut_ptr(), 1);
    }

    tc.set_curve(curve);

    // C: `int ok = ufbxi_tessellate_nurbs_curve_imp(&tc);`
    let ok: bool = tessellate_nurbs_curve_imp(&tc).is_ok();

    free_ator(tc.ator_tmp_mut_ptr());

    if ok {
        clear_error(error);
        let imp: *mut LineCurveImp = tc.imp();
        &raw mut (*imp).curve
    } else {
        fix_error_type(
            tc.error_mut_ptr(),
            b"Failed to tessellate\0".as_ptr(),
            error,
        );
        buf_free(tc.result_mut_ptr());
        free_ator(tc.ator_result_mut_ptr());
        core::ptr::null_mut()
    }
}

// ufbx.c:32282-32318 `ufbx_tessellate_nurbs_curve` (`#else` arm — feature
// disabled). That arm is C parity (a build without `feature = "tessellation"`
// reports `UFBX_ERROR_FEATURE_DISABLED`), NOT a stub.
#[cfg(not(feature = "tessellation"))]
pub(crate) unsafe fn tessellate_nurbs_curve(
    curve: *const crate::generated::NurbsCurve,
    opts: *const crate::generated::RawTessellateCurveOpts,
    error: *mut Error,
) -> *mut crate::generated::LineCurve {
    // C: `curve`/`opts` are unreferenced in the `#else` arm.
    let _ = (curve, opts);
    if !error.is_null() {
        core::ptr::write_bytes(error as *mut u8, 0, size_of::<Error>());
        ufbxi_fmt_err_info!(error, "UFBX_ENABLE_TESSELLATION");
        ufbxi_report_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            "UFBXI_FEATURE_TESSELLATION",
            "Feature disabled"
        );
    }
    core::ptr::null_mut()
}

// ufbx.c:32320-32357 `ufbx_tessellate_nurbs_surface`
// Same `#if UFBXI_FEATURE_TESSELLATION` split as `ufbx_tessellate_nurbs_curve`
// above. C-parity notes: `ufbx_assert(surface)` sits BEFORE
// `ufbxi_check_opts_ptr` here (the curve variant has the opposite order).
#[cfg(feature = "tessellation")]
pub(crate) unsafe fn tessellate_nurbs_surface(
    surface: *const NurbsSurface,
    opts: *const RawTessellateSurfaceOpts,
    error: *mut Error,
) -> *mut Mesh {
    ufbx_assert!(!surface.is_null());
    ufbxi_check_opts_ptr!(Mesh, opts, error);
    if surface.is_null() {
        return core::ptr::null_mut();
    }

    // C: `ufbxi_tessellate_surface_context tc = { UFBX_ERROR_NONE };`
    let tc =
        TessellateSurfaceContext(core::cell::UnsafeCell::new(core::mem::MaybeUninit::zeroed()));
    if !opts.is_null() {
        // C: `tc->opts = *opts` — struct assignment (memcpy).
        core::ptr::copy_nonoverlapping(opts, tc.opts_mut_ptr(), 1);
    }

    tc.set_surface(surface);

    // C: `int ok = ufbxi_tessellate_nurbs_surface_imp(&tc);`
    let ok: bool = tessellate_nurbs_surface_imp(&tc).is_ok();

    buf_free(tc.tmp_mut_ptr());
    map_free(tc.position_map_mut_ptr());
    free_ator(tc.ator_tmp_mut_ptr());

    if ok {
        clear_error(error);
        let imp: *mut MeshImp = tc.imp();
        &raw mut (*imp).mesh
    } else {
        fix_error_type(
            tc.error_mut_ptr(),
            b"Failed to tessellate\0".as_ptr(),
            error,
        );
        buf_free(tc.result_mut_ptr());
        free_ator(tc.ator_result_mut_ptr());
        core::ptr::null_mut()
    }
}

// ufbx.c:32320-32357 `ufbx_tessellate_nurbs_surface` (`#else` arm — feature
// disabled). C-parity note: this arm has NO `ufbxi_fmt_err_info` call (unlike
// `ufbx_tessellate_nurbs_curve` above) — do not add one.
#[cfg(not(feature = "tessellation"))]
pub(crate) unsafe fn tessellate_nurbs_surface(
    surface: *const crate::generated::NurbsSurface,
    opts: *const crate::generated::RawTessellateSurfaceOpts,
    error: *mut Error,
) -> *mut Mesh {
    // C: `surface`/`opts` are unreferenced in the `#else` arm.
    let _ = (surface, opts);
    if !error.is_null() {
        core::ptr::write_bytes(error as *mut u8, 0, size_of::<Error>());
        ufbxi_report_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            "UFBXI_FEATURE_TESSELLATION",
            "Feature disabled"
        );
    }
    core::ptr::null_mut()
}

// ufbx.c:32359-32368 `ufbx_free_line_curve`
// Not feature-gated in C: `ufbxi_line_curve_imp` sits before the
// `#if UFBXI_FEATURE_TESSELLATION` fork (see `native::nurbs`).
pub(crate) unsafe fn free_line_curve(line_curve: *mut LineCurve) {
    if line_curve.is_null() {
        return;
    }
    if !(*line_curve).from_tessellated_nurbs {
        return;
    }

    let imp: *mut LineCurveImp = get_imp(line_curve as *mut c_void);
    ufbx_assert!((*imp).magic == LINE_CURVE_IMP_MAGIC);
    if (*imp).magic != LINE_CURVE_IMP_MAGIC {
        return;
    }
    release_ref(&raw mut (*imp).refcount);
}

// ufbx.c:32370-32379 `ufbx_retain_line_curve`
pub(crate) unsafe fn retain_line_curve(line_curve: *mut LineCurve) {
    if line_curve.is_null() {
        return;
    }
    if !(*line_curve).from_tessellated_nurbs {
        return;
    }

    let imp: *mut LineCurveImp = get_imp(line_curve as *mut c_void);
    ufbx_assert!((*imp).magic == LINE_CURVE_IMP_MAGIC);
    if (*imp).magic != LINE_CURVE_IMP_MAGIC {
        return;
    }
    retain_ref(&raw mut (*imp).refcount);
}

// ufbx.c:32381-32390 `ufbx_find_face_index`
pub(crate) unsafe fn find_face_index(mesh: *mut Mesh, index: usize) -> u32 {
    // C: `!mesh || index > UINT32_MAX` — `index` is `size_t`.
    if mesh.is_null() || index > u32::MAX as usize {
        return NO_INDEX;
    }
    let ix: u32 = index as u32;

    let mut face_ix: usize = usize::MAX;
    macro_lower_bound_eq::<Face>(
        4,
        &mut face_ix,
        (*mesh).faces.data,
        0,
        (*mesh).faces.count,
        // C: `a->index_begin + a->num_indices <= ix` — `uint32_t` arithmetic.
        |a| (*a).index_begin.wrapping_add((*a).num_indices) <= ix,
        // C: `ix >= a->index_begin && ix < a->index_begin + a->num_indices`.
        |a| ix >= (*a).index_begin && ix < (*a).index_begin.wrapping_add((*a).num_indices),
    );
    // C: `(uint32_t)face_ix` — a miss keeps `SIZE_MAX`, truncating to
    // `UFBX_NO_INDEX`.
    face_ix as u32
}

// ufbx.c:32392-32475 `ufbx_catch_triangulate_face`
// C forks on `#if UFBXI_FEATURE_TRIANGULATION`; the enabled arm drives
// `ufbxi_ngon_context` / `ufbxi_triangulate_ngon` (`native/topology.rs`), the
// `#else` arm just records a panic and returns 0. Both arms are ported.
// C: `ufbx_abi ufbxi_noinline` (ufbx.c:32392).
#[cfg(feature = "triangulation")]
#[inline(never)]
pub(crate) unsafe fn catch_triangulate_face(
    panic: *mut Panic,
    indices: *mut u32,
    num_indices: usize,
    mesh: *const Mesh,
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
        (face.index_begin as usize) < (*mesh).num_indices,
        "Face index begin (%u) out of bounds (%zu)",
        face.index_begin,
        (*mesh).num_indices,
    ) {
        return 0;
    }
    if ufbxi_panicf!(
        panic,
        (*mesh).num_indices.wrapping_sub(face.index_begin as usize) >= face.num_indices as usize,
        "Face index end (%u + %u) out of bounds (%zu)",
        face.index_begin,
        face.num_indices,
        (*mesh).num_indices,
    ) {
        return 0;
    }

    if face.num_indices == 3 {
        // Fast case: Already a triangle
        *indices.add(0) = face.index_begin.wrapping_add(0);
        *indices.add(1) = face.index_begin.wrapping_add(1);
        *indices.add(2) = face.index_begin.wrapping_add(2);
        1
    } else if face.num_indices == 4 {
        // Quad: Split along the shortest axis unless a vertex crosses the axis
        let i0: u32 = face.index_begin.wrapping_add(0);
        let i1: u32 = face.index_begin.wrapping_add(1);
        let i2: u32 = face.index_begin.wrapping_add(2);
        let i3: u32 = face.index_begin.wrapping_add(3);
        let v0: Vec3 = *(*mesh)
            .vertex_position
            .values
            .data
            .add(*(*mesh).vertex_position.indices.data.add(i0 as usize) as usize);
        let v1: Vec3 = *(*mesh)
            .vertex_position
            .values
            .data
            .add(*(*mesh).vertex_position.indices.data.add(i1 as usize) as usize);
        let v2: Vec3 = *(*mesh)
            .vertex_position
            .values
            .data
            .add(*(*mesh).vertex_position.indices.data.add(i2 as usize) as usize);
        let v3: Vec3 = *(*mesh)
            .vertex_position
            .values
            .data
            .add(*(*mesh).vertex_position.indices.data.add(i3 as usize) as usize);

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

        if split_a {
            *indices.add(0) = i0;
            *indices.add(1) = i1;
            *indices.add(2) = i2;
            *indices.add(3) = i2;
            *indices.add(4) = i3;
            *indices.add(5) = i0;
        } else {
            *indices.add(0) = i1;
            *indices.add(1) = i2;
            *indices.add(2) = i3;
            *indices.add(3) = i3;
            *indices.add(4) = i0;
            *indices.add(5) = i1;
        }

        2
    } else {
        // C: `ufbxi_ngon_context nc = { 0 };`
        let nc = crate::native::topology::NgonContext(core::cell::UnsafeCell::new(
            core::mem::MaybeUninit::zeroed(),
        ));
        core::ptr::write(
            nc.positions_mut_ptr(),
            core::ptr::read(&(*mesh).vertex_position),
        );
        nc.set_face(face);

        let num_indices_u32: u32 = if num_indices < u32::MAX as usize {
            num_indices as u32
        } else {
            u32::MAX
        };

        let mut local_indices: [u32; 12] = core::mem::zeroed(); // ufbxi_uninit
        if num_indices_u32 < 12 {
            let num_tris: u32 =
                crate::native::topology::triangulate_ngon(&nc, local_indices.as_mut_ptr(), 12);
            core::ptr::copy_nonoverlapping(
                local_indices.as_ptr(),
                indices,
                num_tris.wrapping_mul(3) as usize,
            );
            num_tris
        } else {
            crate::native::topology::triangulate_ngon(&nc, indices, num_indices_u32)
        }
    }
}

// C: `ufbx_abi ufbxi_noinline` (ufbx.c:32392).
#[cfg(not(feature = "triangulation"))]
#[inline(never)]
pub(crate) unsafe fn catch_triangulate_face(
    panic: *mut Panic,
    indices: *mut u32,
    num_indices: usize,
    mesh: *const Mesh,
    face: Face,
) -> u32 {
    // C: `indices`/`num_indices`/`mesh`/`face` are unreferenced in the `#else`
    // arm.
    let _ = (indices, num_indices, mesh, face);
    crate::native::error::panicf_imp(panic, "Triangulation disabled\0".as_ptr(), &[]);
    0
}

// ufbx.c:32477-32482 `ufbx_catch_compute_topology`
pub(crate) unsafe fn catch_compute_topology(
    panic: *mut Panic,
    mesh: *const Mesh,
    indices: *mut TopoEdge,
    num_indices: usize,
) {
    if ufbxi_panicf!(
        panic,
        num_indices >= (*mesh).num_indices,
        "Required mesh.num_indices (%zu) indices, got %zu",
        (*mesh).num_indices,
        num_indices,
    ) {
        return;
    }

    crate::native::topology::compute_topology(mesh, indices);
}

// ufbx.c:32484-32492 `ufbx_catch_topo_next_vertex_edge`
pub(crate) unsafe fn catch_topo_next_vertex_edge(
    panic: *mut Panic,
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
    let twin: u32 = (*topo.add(index as usize)).twin;
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
    (*topo.add(twin as usize)).next
}

// ufbx.c:32494-32499 `ufbx_catch_topo_prev_vertex_edge`
pub(crate) unsafe fn catch_topo_prev_vertex_edge(
    panic: *mut Panic,
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
    (*topo.add((*topo.add(index as usize)).prev as usize)).twin
}

// ufbx.h:5763 `ufbx_get_vertex_real` (an `ufbx_inline`, not `ufbx_abi`, so no
// shim); same `(int32_t)` index cast as `ufbx_get_vertex_vec3` below.
#[inline(always)]
pub(crate) unsafe fn get_vertex_real(v: *const crate::generated::VertexReal, index: usize) -> Real {
    ufbx_assert!(index < (*v).indices.count);
    *(*v)
        .values
        .data
        .offset(*(*v).indices.data.add(index) as i32 as isize)
}

// ufbx.h:5765 `ufbx_get_vertex_vec3` (an `ufbx_inline`, not `ufbx_abi`, so no
// shim): `v->values.data[(int32_t)v->indices.data[index]]`. The `(int32_t)`
// cast is C-parity — a >= 0x80000000 index sign-extends into a negative offset.
#[inline(always)]
pub(crate) unsafe fn get_vertex_vec3(v: *const VertexVec3, index: usize) -> Vec3 {
    ufbx_assert!(index < (*v).indices.count);
    *(*v)
        .values
        .data
        .offset(*(*v).indices.data.add(index) as i32 as isize)
}

// ufbx.c:32501-32532 `ufbx_catch_get_weighted_face_normal`
// C: `ufbx_abi ufbxi_noinline` (ufbx.c:32501).
#[inline(never)]
pub(crate) unsafe fn catch_get_weighted_face_normal(
    panic: *mut Panic,
    positions: *const VertexVec3,
    face: Face,
) -> Vec3 {
    if ufbxi_panicf!(
        panic,
        face.index_begin as usize <= (*positions).indices.count,
        "Face index begin (%u) out of bounds (%zu)",
        face.index_begin,
        (*positions).indices.count,
    ) {
        return ZERO_VEC3;
    }
    if ufbxi_panicf!(
        panic,
        (*positions)
            .indices
            .count
            .wrapping_sub(face.index_begin as usize)
            >= face.num_indices as usize,
        "Face index end (%u + %u) out of bounds (%zu)",
        face.index_begin,
        face.num_indices,
        (*positions).indices.count,
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
pub(crate) unsafe fn catch_generate_normal_mapping(
    panic: *mut Panic,
    mesh: *const Mesh,
    topo: *const TopoEdge,
    num_topo: usize,
    normal_indices: *mut u32,
    num_normal_indices: usize,
    assume_smooth: bool,
) -> usize {
    let mut next_index: u32 = 0;
    if ufbxi_panicf!(
        panic,
        num_normal_indices >= (*mesh).num_indices,
        "Expected at least mesh.num_indices (%zu), got %zu",
        (*mesh).num_indices,
        num_normal_indices,
    ) {
        return 0;
    }

    for i in 0..(*mesh).num_indices {
        *normal_indices.add(i) = NO_INDEX;
    }

    // Walk around vertices and merge around smooth edges
    for vi in 0..(*mesh).num_vertices {
        let original_start: u32 = *(*mesh).vertex_first_index.data.add(vi);
        if original_start == NO_INDEX {
            continue;
        }
        let mut start: u32 = original_start;
        let mut cur: u32 = start;

        loop {
            let prev: u32 = topo_next_vertex_edge(topo, num_topo, cur);
            if !is_edge_smooth(mesh, topo, num_topo, cur, assume_smooth) {
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
        *normal_indices.add(start as usize) = next_index;
        next_index = next_index.wrapping_add(1);
        let mut next: u32 = start;
        loop {
            next = topo_prev_vertex_edge(topo, num_topo, next);
            if next == NO_INDEX || next == start {
                break;
            }

            if !is_edge_smooth(mesh, topo, num_topo, next, assume_smooth) {
                next_index = next_index.wrapping_add(1);
            }
            *normal_indices.add(next as usize) = next_index.wrapping_sub(1);
        }
    }

    // Assign non-manifold indices
    for i in 0..(*mesh).num_indices {
        if *normal_indices.add(i) == NO_INDEX {
            // C: `normal_indices[i] = next_index++;`
            *normal_indices.add(i) = next_index;
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
    catch_generate_normal_mapping(
        core::ptr::null_mut(),
        mesh,
        topo,
        num_topo,
        normal_indices,
        num_normal_indices,
        assume_smooth,
    )
}

// ufbx.c:32585-32612 `ufbx_catch_compute_normals`
pub(crate) unsafe fn catch_compute_normals(
    panic: *mut Panic,
    mesh: *const Mesh,
    positions: *const VertexVec3,
    normal_indices: *const u32,
    num_normal_indices: usize,
    normals: *mut Vec3,
    num_normals: usize,
) {
    if ufbxi_panicf!(
        panic,
        num_normal_indices >= (*mesh).num_indices,
        "Expected at least mesh.num_indices (%zu), got %zu",
        (*mesh).num_indices,
        num_normal_indices,
    ) {
        return;
    }

    core::ptr::write_bytes(
        normals as *mut u8,
        0,
        size_of::<Vec3>().wrapping_mul(num_normals),
    );

    for fi in 0..(*mesh).num_faces {
        let face: Face = *(*mesh).faces.data.add(fi);
        let normal: Vec3 = get_weighted_face_normal(positions, face);
        for ix in 0..face.num_indices as usize {
            let index: u32 = *normal_indices.add((face.index_begin as usize).wrapping_add(ix));

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

            let n: *mut Vec3 = normals.add(index as usize);
            *n = add3(*n, normal);
        }
    }

    for i in 0..num_normals {
        let len: Real = length3(*normals.add(i));
        if len > 0.0 {
            (*normals.add(i)).x /= len;
            (*normals.add(i)).y /= len;
            (*normals.add(i)).z /= len;
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
    catch_compute_normals(
        core::ptr::null_mut(),
        mesh,
        positions,
        normal_indices,
        num_normal_indices,
        normals,
        num_normals,
    );
}

// ufbx.c:32619-32625 `ufbx_subdivide_mesh`
// The public function has no `#if`/`#else` fork — it always delegates to
// `ufbxi_subdivide_mesh` (native/subdivision.rs `subdivide_mesh`), which is the
// one that carries the `UFBXI_FEATURE_SUBDIVISION` split.
pub(crate) unsafe fn subdivide_mesh(
    mesh: *const Mesh,
    level: usize,
    opts: *const crate::generated::RawSubdivideOpts,
    error: *mut Error,
) -> *mut Mesh {
    ufbxi_check_opts_ptr!(Mesh, opts, error);
    if mesh.is_null() {
        return core::ptr::null_mut();
    }
    if level == 0 {
        return mesh as *mut Mesh;
    }
    crate::native::subdivision::subdivide_mesh(mesh, level, opts, error)
}

// ufbx.c:32627-32636 `ufbx_free_mesh`
pub(crate) unsafe fn free_mesh(mesh: *mut Mesh) {
    if mesh.is_null() {
        return;
    }
    if !(*mesh).subdivision_evaluated && !(*mesh).from_tessellated_nurbs {
        return;
    }

    let imp: *mut MeshImp = get_imp(mesh as *mut c_void);
    ufbx_assert!((*imp).magic == MESH_IMP_MAGIC);
    if (*imp).magic != MESH_IMP_MAGIC {
        return;
    }
    release_ref(&raw mut (*imp).refcount);
}

// ufbx.c:32638-32647 `ufbx_retain_mesh`
pub(crate) unsafe fn retain_mesh(mesh: *mut Mesh) {
    if mesh.is_null() {
        return;
    }
    if !(*mesh).subdivision_evaluated && !(*mesh).from_tessellated_nurbs {
        return;
    }

    let imp: *mut MeshImp = get_imp(mesh as *mut c_void);
    ufbx_assert!((*imp).magic == MESH_IMP_MAGIC);
    if (*imp).magic != MESH_IMP_MAGIC {
        return;
    }
    retain_ref(&raw mut (*imp).refcount);
}

// ufbx.c:32649-32655 `ufbx_load_geometry_cache`
pub(crate) unsafe fn load_geometry_cache(
    filename: *const u8,
    opts: *const RawGeometryCacheOpts,
    error: *mut Error,
) -> *mut GeometryCache {
    load_geometry_cache_len(
        filename,
        crate::native::error::strlen(filename),
        opts,
        error,
    )
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
    error: *mut Error,
) -> *mut GeometryCache {
    ufbxi_check_opts_ptr!(GeometryCache, opts, error);
    let str_: String = safe_string(filename, filename_len);
    crate::native::cache::load_geometry_cache(str_, opts, error)
}

// ufbx.c:32666-32675 `ufbx_free_geometry_cache`
pub(crate) unsafe fn free_geometry_cache(cache: *mut GeometryCache) {
    if cache.is_null() {
        return;
    }

    let imp: *mut GeometryCacheImp = get_imp(cache as *mut c_void);
    ufbx_assert!((*imp).magic == CACHE_IMP_MAGIC);
    if (*imp).magic != CACHE_IMP_MAGIC {
        return;
    }
    if (*imp).owned_by_scene {
        return;
    }
    release_ref(&raw mut (*imp).refcount);
}

// ufbx.c:32677-32686 `ufbx_retain_geometry_cache`
pub(crate) unsafe fn retain_geometry_cache(cache: *mut GeometryCache) {
    if cache.is_null() {
        return;
    }

    let imp: *mut GeometryCacheImp = get_imp(cache as *mut c_void);
    ufbx_assert!((*imp).magic == CACHE_IMP_MAGIC);
    if (*imp).magic != CACHE_IMP_MAGIC {
        return;
    }
    if (*imp).owned_by_scene {
        return;
    }
    retain_ref(&raw mut (*imp).refcount);
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
        ufbxi_check_opts_return_no_error!(0usize, user_opts);
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
            core::ptr::read(user_opts)
        } else {
            core::mem::zeroed()
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
        match (*frame).data_format {
            CacheDataFormat::Unknown => src_count = 0,
            CacheDataFormat::RealFloat => src_count = (*frame).data_count as usize,
            CacheDataFormat::Vec3Float => src_count = (*frame).data_count.wrapping_mul(3) as usize,
            CacheDataFormat::RealDouble => {
                src_count = (*frame).data_count as usize;
                use_double = true;
            }
            CacheDataFormat::Vec3Double => {
                src_count = (*frame).data_count.wrapping_mul(3) as usize;
                use_double = true;
            }
        }

        // C: `bool src_big_endian = false;` then the switch assigns / returns.
        let src_big_endian: bool;
        match (*frame).data_encoding {
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

        let mut stream: RawStream = RawStream::default(); // C: `ufbx_stream stream = { 0 };`
        if !crate::native::read::open_file(
            &opts.open_file_cb as *const RawOpenFileCb,
            &mut stream,
            (*frame).filename.data,
            (*frame).filename.length,
            core::ptr::null(),
            core::ptr::null_mut(),
            OpenFileType::GeometryCache,
        ) {
            return 0;
        }

        // Skip to the correct point in the file
        let mut offset: u64 = (*frame).data_offset;
        if stream.skip_fn.is_some() {
            while offset > 0 {
                let to_skip = min64(offset, MAX_SKIP_SIZE as u64) as usize;
                if !(stream.skip_fn.unwrap_unchecked())(stream.user, to_skip) {
                    break;
                }
                offset -= to_skip as u64;
            }
        } else {
            let mut buffer = [0u8; 4096]; // ufbxi_uninit
            while offset > 0 {
                let to_skip = min64(offset, buffer.len() as u64) as usize;
                let num_read = (stream.read_fn.unwrap_unchecked())(
                    stream.user,
                    buffer.as_mut_ptr() as *mut c_void,
                    to_skip,
                );
                if num_read != to_skip {
                    break;
                }
                offset -= to_skip as u64;
            }
        }

        // Failed to skip all the way
        if offset > 0 {
            if let Some(close_fn) = stream.close_fn {
                close_fn(stream.user);
            }
            return 0;
        }

        let mut dst: *mut Real = data;
        let mut mirror_ix: usize = ((*frame).mirror_axis as usize).wrapping_sub(1);
        // C: `ufbxi_geometry_cache_buffer buffer; // ufbxi_uninit` — zero-filled
        // here (Rust forbids `assume_init` on the float array); each element is
        // overwritten before it is read within `0..num_read`, so this is
        // behavior-identical to the C uninitialized buffer.
        let mut buffer: GeometryCacheBuffer = core::mem::zeroed();
        while src_count > 0 {
            let to_read = min_sz(src_count, GEOMETRY_CACHE_BUFFER_SIZE);
            src_count -= to_read;
            let num_read: usize;
            if use_double {
                let mut bytes_read = (stream.read_fn.unwrap_unchecked())(
                    stream.user,
                    buffer.src.f64_.as_mut_ptr() as *mut c_void,
                    to_read * size_of::<f64>(),
                );
                if bytes_read == usize::MAX {
                    bytes_read = 0;
                }
                num_read = bytes_read / size_of::<f64>();
                if src_big_endian != dst_big_endian {
                    let p = buffer.src.f64_.as_mut_ptr() as *mut u8;
                    for i in 0..num_read {
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
                for i in 0..num_read {
                    buffer.dst[i] = buffer.src.f64_[i] as Real;
                }
            } else {
                let mut bytes_read = (stream.read_fn.unwrap_unchecked())(
                    stream.user,
                    buffer.src.f32_.as_mut_ptr() as *mut c_void,
                    to_read * size_of::<f32>(),
                );
                if bytes_read == usize::MAX {
                    bytes_read = 0;
                }
                num_read = bytes_read / size_of::<f32>();
                if src_big_endian != dst_big_endian {
                    let p = buffer.src.f32_.as_mut_ptr() as *mut u8;
                    for i in 0..num_read {
                        let v = p.add(i * 4);
                        let t = *v.add(0);
                        *v.add(0) = *v.add(3);
                        *v.add(3) = t;
                        let t = *v.add(1);
                        *v.add(1) = *v.add(2);
                        *v.add(2) = t;
                    }
                }
                for i in 0..num_read {
                    buffer.dst[i] = buffer.src.f32_[i] as Real;
                }
            }

            if !opts.ignore_transform {
                let scale: Real = (*frame).scale_factor;
                if scale != 1.0 {
                    for i in 0..num_read {
                        buffer.dst[i] *= scale;
                    }
                }
                if (*frame).mirror_axis as u32 != 0 {
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
                        *dst.add(i) += buffer.dst[i] * weight;
                    }
                } else {
                    for i in 0..num_read {
                        *dst.add(i) = buffer.dst[i] * weight;
                    }
                }
                dst = dst.add(num_read);
            }

            if num_read != to_read {
                break;
            }
        }

        if let Some(close_fn) = stream.close_fn {
            close_fn(stream.user);
        }

        to_size(dst.offset_from(data))
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
        ufbxi_check_opts_return_no_error!(0usize, user_opts);
        if channel.is_null() || count == 0 {
            return 0;
        }
        ufbx_assert!(!data.is_null());
        if data.is_null() {
            return 0;
        }
        if (*channel).frames.count == 0 {
            return 0;
        }

        let mut opts: RawGeometryCacheDataOpts = if !user_opts.is_null() {
            core::ptr::read(user_opts)
        } else {
            core::mem::zeroed()
        };

        let mut begin: usize = 0;
        let mut end: usize = (*channel).frames.count;
        let frames: *const CacheFrame = (*channel).frames.data;
        while end - begin >= 8 {
            let mid = (begin + end) >> 1;
            if (*frames.add(mid)).time < time {
                begin = mid + 1;
            } else {
                end = mid;
            }
        }

        let eps: f64 = 0.00000001;

        end = (*channel).frames.count;
        while begin < end {
            let next: *const CacheFrame = frames.add(begin);
            if (*next).time < time {
                begin += 1;
                continue;
            }

            // First keyframe
            if begin == 0 {
                return read_geometry_cache_real(next, data, count, &opts);
            }

            let prev: *const CacheFrame = next.sub(1);

            // Snap to exact frames if near
            if math::fabs((*next).time - time) < eps {
                return read_geometry_cache_real(next, data, count, &opts);
            }
            if math::fabs((*prev).time - time) < eps {
                return read_geometry_cache_real(prev, data, count, &opts);
            }

            let rcp_delta: f64 = 1.0 / ((*next).time - (*prev).time);
            let t: f64 = (time - (*prev).time) * rcp_delta;

            let original_weight: Real = if opts.use_weight { opts.weight } else { 1.0 };

            opts.use_weight = true;
            opts.weight = (original_weight as f64 * (1.0 - t)) as Real;
            let num_prev = read_geometry_cache_real(prev, data, count, &opts);

            opts.additive = true;
            opts.weight = (original_weight as f64 * t) as Real;
            return read_geometry_cache_real(next, data, num_prev, &opts);
        }

        // Last frame
        let last: *const CacheFrame = frames.add(end - 1);
        read_geometry_cache_real(last, data, count, &opts)
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
        read_geometry_cache_real(frame, data as *mut Real, count.wrapping_mul(3), opts) / 3
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
        sample_geometry_cache_real(
            channel,
            time,
            data as *mut Real,
            count.wrapping_mul(3),
            opts,
        ) / 3
    }
    #[cfg(not(feature = "geometry-cache"))]
    {
        let _ = (channel, time, data, count, opts);
        0
    }
}

// ufbx.c:32957-32964 `ufbx_dom_find_len`
pub(crate) unsafe fn dom_find_len(
    parent: *const DomNode,
    name: *const u8,
    name_len: usize,
) -> *mut DomNode {
    let ref_: String = safe_string(name, name_len);
    // C: `ufbxi_for_ptr_list(ufbx_dom_node, p_child, parent->children)` — the
    // `RefList` payload is a flat array of `ufbx_dom_node*`.
    let mut p_child: *mut *mut DomNode = (*parent).children.data as *mut *mut DomNode;
    let p_child_end: *mut *mut DomNode = p_child.add((*parent).children.count);
    while p_child != p_child_end {
        if str_equal((**p_child).name, ref_) {
            return *p_child;
        }
        p_child = p_child.add(1);
    }
    core::ptr::null_mut()
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
    error: *mut Error,
) -> usize {
    let mut local_error = MaybeUninit::<Error>::uninit(); // ufbxi_uninit
    let mut error = error;
    if error.is_null() {
        error = local_error.as_mut_ptr();
    }
    core::ptr::write_bytes(error as *mut u8, 0, size_of::<Error>());
    crate::native::index_gen::generate_indices(
        streams,
        num_streams,
        indices,
        num_indices,
        allocator,
        error,
    )
}

// ufbx.c:32976-32979 `ufbx_thread_pool_run_task` — delegates to
// `ufbxi_thread_pool_execute` (`// -- Threading`, `native/thread.rs`).
pub(crate) unsafe fn thread_pool_run_task(ctx: ThreadPoolContext, index: u32) {
    crate::native::thread::thread_pool_execute(ctx as *mut ThreadPool, index);
}

// ufbx.c:32981-32985 `ufbx_thread_pool_set_user_ptr`
pub(crate) unsafe fn thread_pool_set_user_ptr(ctx: ThreadPoolContext, user: *mut c_void) {
    let pool: *mut ThreadPool = ctx as *mut ThreadPool;
    (*pool).user_ptr = user;
}

// ufbx.c:32987-32991 `ufbx_thread_pool_get_user_ptr`
pub(crate) unsafe fn thread_pool_get_user_ptr(ctx: ThreadPoolContext) -> *mut c_void {
    let pool: *mut ThreadPool = ctx as *mut ThreadPool;
    (*pool).user_ptr
}

// ufbx.c:32993-32999 `ufbx_catch_get_vertex_real`
#[inline(never)]
pub(crate) unsafe fn catch_get_vertex_real(
    panic: *mut Panic,
    v: *const VertexReal,
    index: usize,
) -> Real {
    if ufbxi_panicf!(
        panic,
        index < (*v).indices.count,
        "index (%zu) out of range (%zu)",
        index,
        (*v).indices.count,
    ) {
        return 0.0;
    }
    let ix: u32 = *(*v).indices.data.add(index);
    if ufbxi_panicf!(
        panic,
        (ix as usize) < (*v).values.count || ix == NO_INDEX,
        "Corrupted or missing vertex attribute (%u) at %zu",
        ix,
        index,
    ) {
        return 0.0;
    }
    *(*v).values.data.offset(ix as i32 as isize)
}

// ufbx.c:33001-33007 `ufbx_catch_get_vertex_vec2`
#[inline(never)]
pub(crate) unsafe fn catch_get_vertex_vec2(
    panic: *mut Panic,
    v: *const VertexVec2,
    index: usize,
) -> Vec2 {
    if ufbxi_panicf!(
        panic,
        index < (*v).indices.count,
        "index (%zu) out of range (%zu)",
        index,
        (*v).indices.count,
    ) {
        return ZERO_VEC2;
    }
    let ix: u32 = *(*v).indices.data.add(index);
    if ufbxi_panicf!(
        panic,
        (ix as usize) < (*v).values.count || ix == NO_INDEX,
        "Corrupted or missing vertex attribute (%u) at %zu",
        ix,
        index,
    ) {
        return ZERO_VEC2;
    }
    *(*v).values.data.offset(ix as i32 as isize)
}

// ufbx.c:33009-33015 `ufbx_catch_get_vertex_vec3`
#[inline(never)]
pub(crate) unsafe fn catch_get_vertex_vec3(
    panic: *mut Panic,
    v: *const VertexVec3,
    index: usize,
) -> Vec3 {
    if ufbxi_panicf!(
        panic,
        index < (*v).indices.count,
        "index (%zu) out of range (%zu)",
        index,
        (*v).indices.count,
    ) {
        return ZERO_VEC3;
    }
    let ix: u32 = *(*v).indices.data.add(index);
    if ufbxi_panicf!(
        panic,
        (ix as usize) < (*v).values.count || ix == NO_INDEX,
        "Corrupted or missing vertex attribute (%u) at %zu",
        ix,
        index,
    ) {
        return ZERO_VEC3;
    }
    *(*v).values.data.offset(ix as i32 as isize)
}

// ufbx.c:33017-33023 `ufbx_catch_get_vertex_vec4`
#[inline(never)]
pub(crate) unsafe fn catch_get_vertex_vec4(
    panic: *mut Panic,
    v: *const VertexVec4,
    index: usize,
) -> Vec4 {
    if ufbxi_panicf!(
        panic,
        index < (*v).indices.count,
        "index (%zu) out of range (%zu)",
        index,
        (*v).indices.count,
    ) {
        return ZERO_VEC4;
    }
    let ix: u32 = *(*v).indices.data.add(index);
    if ufbxi_panicf!(
        panic,
        (ix as usize) < (*v).values.count || ix == NO_INDEX,
        "Corrupted or missing vertex attribute (%u) at %zu",
        ix,
        index,
    ) {
        return ZERO_VEC4;
    }
    *(*v).values.data.offset(ix as i32 as isize)
}

// ufbx.c:33025-33032 `ufbx_catch_get_vertex_w_vec3`
pub(crate) unsafe fn catch_get_vertex_w_vec3(
    panic: *mut Panic,
    v: *const VertexVec3,
    index: usize,
) -> Real {
    if ufbxi_panicf!(
        panic,
        index < (*v).indices.count,
        "index (%zu) out of range (%zu)",
        index,
        (*v).indices.count,
    ) {
        return 0.0;
    }
    if (*v).values_w.count == 0 {
        return 0.0;
    }
    let ix: u32 = *(*v).indices.data.add(index);
    if ufbxi_panicf!(
        panic,
        (ix as usize) < (*v).values.count || ix == NO_INDEX,
        "Corrupted or missing vertex attribute (%u) at %zu",
        ix,
        index,
    ) {
        return 0.0;
    }
    *(*v).values_w.data.offset(ix as i32 as isize)
}

// ufbx.c:33034-33075 `ufbx_as_*` — each returns `element` reinterpreted iff its
// `type` matches, else NULL. Non-null guard AND type test, in that order.
// ufbx.c:33034 `ufbx_as_unknown`
pub(crate) unsafe fn as_unknown(element: *const Element) -> *mut Unknown {
    if !element.is_null() && (*element).type_ == ElementType::Unknown {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Unknown` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33035 `ufbx_as_node`
pub(crate) unsafe fn as_node(element: *const Element) -> *mut Node {
    if !element.is_null() && (*element).type_ == ElementType::Node {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Node` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33036 `ufbx_as_mesh`
pub(crate) unsafe fn as_mesh(element: *const Element) -> *mut Mesh {
    if !element.is_null() && (*element).type_ == ElementType::Mesh {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Mesh` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33037 `ufbx_as_light`
pub(crate) unsafe fn as_light(element: *const Element) -> *mut Light {
    if !element.is_null() && (*element).type_ == ElementType::Light {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Light` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33038 `ufbx_as_camera`
pub(crate) unsafe fn as_camera(element: *const Element) -> *mut Camera {
    if !element.is_null() && (*element).type_ == ElementType::Camera {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Camera` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33039 `ufbx_as_bone`
pub(crate) unsafe fn as_bone(element: *const Element) -> *mut Bone {
    if !element.is_null() && (*element).type_ == ElementType::Bone {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Bone` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33040 `ufbx_as_empty`
pub(crate) unsafe fn as_empty(element: *const Element) -> *mut Empty {
    if !element.is_null() && (*element).type_ == ElementType::Empty {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Empty` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33041 `ufbx_as_line_curve`
pub(crate) unsafe fn as_line_curve(element: *const Element) -> *mut LineCurve {
    if !element.is_null() && (*element).type_ == ElementType::LineCurve {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `LineCurve` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33042 `ufbx_as_nurbs_curve`
pub(crate) unsafe fn as_nurbs_curve(element: *const Element) -> *mut NurbsCurve {
    if !element.is_null() && (*element).type_ == ElementType::NurbsCurve {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `NurbsCurve` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33043 `ufbx_as_nurbs_surface`
pub(crate) unsafe fn as_nurbs_surface(element: *const Element) -> *mut NurbsSurface {
    if !element.is_null() && (*element).type_ == ElementType::NurbsSurface {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `NurbsSurface` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33044 `ufbx_as_nurbs_trim_surface`
pub(crate) unsafe fn as_nurbs_trim_surface(element: *const Element) -> *mut NurbsTrimSurface {
    if !element.is_null() && (*element).type_ == ElementType::NurbsTrimSurface {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `NurbsTrimSurface` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33045 `ufbx_as_nurbs_trim_boundary`
pub(crate) unsafe fn as_nurbs_trim_boundary(element: *const Element) -> *mut NurbsTrimBoundary {
    if !element.is_null() && (*element).type_ == ElementType::NurbsTrimBoundary {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `NurbsTrimBoundary` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33046 `ufbx_as_procedural_geometry`
pub(crate) unsafe fn as_procedural_geometry(element: *const Element) -> *mut ProceduralGeometry {
    if !element.is_null() && (*element).type_ == ElementType::ProceduralGeometry {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `ProceduralGeometry` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33047 `ufbx_as_stereo_camera`
pub(crate) unsafe fn as_stereo_camera(element: *const Element) -> *mut StereoCamera {
    if !element.is_null() && (*element).type_ == ElementType::StereoCamera {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `StereoCamera` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33048 `ufbx_as_camera_switcher`
pub(crate) unsafe fn as_camera_switcher(element: *const Element) -> *mut CameraSwitcher {
    if !element.is_null() && (*element).type_ == ElementType::CameraSwitcher {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `CameraSwitcher` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33049 `ufbx_as_marker`
pub(crate) unsafe fn as_marker(element: *const Element) -> *mut Marker {
    if !element.is_null() && (*element).type_ == ElementType::Marker {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Marker` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33050 `ufbx_as_lod_group`
pub(crate) unsafe fn as_lod_group(element: *const Element) -> *mut LodGroup {
    if !element.is_null() && (*element).type_ == ElementType::LodGroup {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `LodGroup` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33051 `ufbx_as_skin_deformer`
pub(crate) unsafe fn as_skin_deformer(element: *const Element) -> *mut SkinDeformer {
    if !element.is_null() && (*element).type_ == ElementType::SkinDeformer {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `SkinDeformer` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33052 `ufbx_as_skin_cluster`
pub(crate) unsafe fn as_skin_cluster(element: *const Element) -> *mut SkinCluster {
    if !element.is_null() && (*element).type_ == ElementType::SkinCluster {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `SkinCluster` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33053 `ufbx_as_blend_deformer`
pub(crate) unsafe fn as_blend_deformer(element: *const Element) -> *mut BlendDeformer {
    if !element.is_null() && (*element).type_ == ElementType::BlendDeformer {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `BlendDeformer` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33054 `ufbx_as_blend_channel`
pub(crate) unsafe fn as_blend_channel(element: *const Element) -> *mut BlendChannel {
    if !element.is_null() && (*element).type_ == ElementType::BlendChannel {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `BlendChannel` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33055 `ufbx_as_blend_shape`
pub(crate) unsafe fn as_blend_shape(element: *const Element) -> *mut BlendShape {
    if !element.is_null() && (*element).type_ == ElementType::BlendShape {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `BlendShape` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33056 `ufbx_as_cache_deformer`
pub(crate) unsafe fn as_cache_deformer(element: *const Element) -> *mut CacheDeformer {
    if !element.is_null() && (*element).type_ == ElementType::CacheDeformer {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `CacheDeformer` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33057 `ufbx_as_cache_file`
pub(crate) unsafe fn as_cache_file(element: *const Element) -> *mut CacheFile {
    if !element.is_null() && (*element).type_ == ElementType::CacheFile {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `CacheFile` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33058 `ufbx_as_material`
pub(crate) unsafe fn as_material(element: *const Element) -> *mut Material {
    if !element.is_null() && (*element).type_ == ElementType::Material {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Material` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33059 `ufbx_as_texture`
pub(crate) unsafe fn as_texture(element: *const Element) -> *mut Texture {
    if !element.is_null() && (*element).type_ == ElementType::Texture {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Texture` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33060 `ufbx_as_video`
pub(crate) unsafe fn as_video(element: *const Element) -> *mut Video {
    if !element.is_null() && (*element).type_ == ElementType::Video {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Video` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33061 `ufbx_as_shader`
pub(crate) unsafe fn as_shader(element: *const Element) -> *mut Shader {
    if !element.is_null() && (*element).type_ == ElementType::Shader {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Shader` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33062 `ufbx_as_shader_binding`
pub(crate) unsafe fn as_shader_binding(element: *const Element) -> *mut ShaderBinding {
    if !element.is_null() && (*element).type_ == ElementType::ShaderBinding {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `ShaderBinding` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33063 `ufbx_as_anim_stack`
pub(crate) unsafe fn as_anim_stack(element: *const Element) -> *mut AnimStack {
    if !element.is_null() && (*element).type_ == ElementType::AnimStack {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `AnimStack` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33064 `ufbx_as_anim_layer`
pub(crate) unsafe fn as_anim_layer(element: *const Element) -> *mut AnimLayer {
    if !element.is_null() && (*element).type_ == ElementType::AnimLayer {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `AnimLayer` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33065 `ufbx_as_anim_value`
pub(crate) unsafe fn as_anim_value(element: *const Element) -> *mut AnimValue {
    if !element.is_null() && (*element).type_ == ElementType::AnimValue {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `AnimValue` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33066 `ufbx_as_anim_curve`
pub(crate) unsafe fn as_anim_curve(element: *const Element) -> *mut AnimCurve {
    if !element.is_null() && (*element).type_ == ElementType::AnimCurve {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `AnimCurve` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33067 `ufbx_as_display_layer`
pub(crate) unsafe fn as_display_layer(element: *const Element) -> *mut DisplayLayer {
    if !element.is_null() && (*element).type_ == ElementType::DisplayLayer {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `DisplayLayer` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33068 `ufbx_as_selection_set`
pub(crate) unsafe fn as_selection_set(element: *const Element) -> *mut SelectionSet {
    if !element.is_null() && (*element).type_ == ElementType::SelectionSet {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `SelectionSet` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33069 `ufbx_as_selection_node`
pub(crate) unsafe fn as_selection_node(element: *const Element) -> *mut SelectionNode {
    if !element.is_null() && (*element).type_ == ElementType::SelectionNode {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `SelectionNode` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33070 `ufbx_as_character`
pub(crate) unsafe fn as_character(element: *const Element) -> *mut Character {
    if !element.is_null() && (*element).type_ == ElementType::Character {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Character` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33071 `ufbx_as_constraint`
pub(crate) unsafe fn as_constraint(element: *const Element) -> *mut Constraint {
    if !element.is_null() && (*element).type_ == ElementType::Constraint {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Constraint` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33072 `ufbx_as_audio_layer`
pub(crate) unsafe fn as_audio_layer(element: *const Element) -> *mut AudioLayer {
    if !element.is_null() && (*element).type_ == ElementType::AudioLayer {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `AudioLayer` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33073 `ufbx_as_audio_clip`
pub(crate) unsafe fn as_audio_clip(element: *const Element) -> *mut AudioClip {
    if !element.is_null() && (*element).type_ == ElementType::AudioClip {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `AudioClip` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33074 `ufbx_as_pose`
pub(crate) unsafe fn as_pose(element: *const Element) -> *mut Pose {
    if !element.is_null() && (*element).type_ == ElementType::Pose {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `Pose` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}
// ufbx.c:33075 `ufbx_as_metadata_object`
pub(crate) unsafe fn as_metadata_object(element: *const Element) -> *mut MetadataObject {
    if !element.is_null() && (*element).type_ == ElementType::MetadataObject {
        // Reconstitute a WIDE pointer via the arena allocation's exposed
        // provenance: `element` may derive from a caller's `&Element`, whose
        // retag covers only the header — reading the full `MetadataObject` through it
        // is out-of-range UB (Miri SB; tests/miri.rs downcast regression).
        core::ptr::with_exposed_provenance_mut(element as usize)
    } else {
        core::ptr::null_mut()
    }
}

// ufbx.c:33077-33081 `ufbx_dom_is_array`
pub(crate) unsafe fn dom_is_array(node: *const DomNode) -> bool {
    if node.is_null() || (*node).values.count != 1 {
        return false;
    }
    // C: `ufbx_dom_value v = node->values.data[0];`
    let v: &DomValue = &*(*node).values.data;
    v.type_ as u32 >= DomValueType::ArrayI32 as u32
        && v.type_ as u32 <= DomValueType::ArrayBlob as u32
}
// ufbx.c:33082-33084 `ufbx_dom_array_size`
pub(crate) unsafe fn dom_array_size(node: *const DomNode) -> usize {
    if dom_is_array(node) {
        (*(*node).values.data).value_int as usize
    } else {
        0
    }
}
// ufbx.c:33085-33093 `ufbx_dom_as_int32_list`
pub(crate) unsafe fn dom_as_int32_list(node: *const DomNode) -> List<i32> {
    let mut list: List<i32> = MaybeUninit::zeroed().assume_init();
    list.data = core::ptr::null();
    list.count = 0;
    if !node.is_null()
        && (*node).values.count == 1
        && (*(*node).values.data).type_ == DomValueType::ArrayI32
    {
        let value: &DomValue = &*(*node).values.data;
        list.data = value.value_blob.data as *const i32;
        list.count = value.value_blob.size / size_of::<i32>();
    }
    list
}
// ufbx.c:33094-33102 `ufbx_dom_as_int64_list`
pub(crate) unsafe fn dom_as_int64_list(node: *const DomNode) -> List<i64> {
    let mut list: List<i64> = MaybeUninit::zeroed().assume_init();
    list.data = core::ptr::null();
    list.count = 0;
    if !node.is_null()
        && (*node).values.count == 1
        && (*(*node).values.data).type_ == DomValueType::ArrayI64
    {
        let value: &DomValue = &*(*node).values.data;
        list.data = value.value_blob.data as *const i64;
        list.count = value.value_blob.size / size_of::<i64>();
    }
    list
}
// ufbx.c:33103-33111 `ufbx_dom_as_float_list`
pub(crate) unsafe fn dom_as_float_list(node: *const DomNode) -> List<f32> {
    let mut list: List<f32> = MaybeUninit::zeroed().assume_init();
    list.data = core::ptr::null();
    list.count = 0;
    if !node.is_null()
        && (*node).values.count == 1
        && (*(*node).values.data).type_ == DomValueType::ArrayF32
    {
        let value: &DomValue = &*(*node).values.data;
        list.data = value.value_blob.data as *const f32;
        list.count = value.value_blob.size / size_of::<f32>();
    }
    list
}
// ufbx.c:33112-33120 `ufbx_dom_as_double_list`
pub(crate) unsafe fn dom_as_double_list(node: *const DomNode) -> List<f64> {
    let mut list: List<f64> = MaybeUninit::zeroed().assume_init();
    list.data = core::ptr::null();
    list.count = 0;
    if !node.is_null()
        && (*node).values.count == 1
        && (*(*node).values.data).type_ == DomValueType::ArrayF64
    {
        let value: &DomValue = &*(*node).values.data;
        list.data = value.value_blob.data as *const f64;
        list.count = value.value_blob.size / size_of::<f64>();
    }
    list
}
// ufbx.c:33121-33129 `ufbx_dom_as_real_list`
pub(crate) unsafe fn dom_as_real_list(node: *const DomNode) -> List<Real> {
    let mut list: List<Real> = MaybeUninit::zeroed().assume_init();
    list.data = core::ptr::null();
    list.count = 0;
    // C: `sizeof(ufbx_real) == sizeof(double) ? ARRAY_F64 : ARRAY_F32`
    let want = if size_of::<Real>() == size_of::<f64>() {
        DomValueType::ArrayF64
    } else {
        DomValueType::ArrayF32
    };
    if !node.is_null() && (*node).values.count == 1 && (*(*node).values.data).type_ == want {
        let value: &DomValue = &*(*node).values.data;
        list.data = value.value_blob.data as *const Real;
        list.count = value.value_blob.size / size_of::<Real>();
    }
    list
}
// ufbx.c:33130-33138 `ufbx_dom_as_blob_list`
pub(crate) unsafe fn dom_as_blob_list(node: *const DomNode) -> List<Blob> {
    let mut list: List<Blob> = MaybeUninit::zeroed().assume_init();
    list.data = core::ptr::null();
    list.count = 0;
    if !node.is_null()
        && (*node).values.count == 1
        && (*(*node).values.data).type_ == DomValueType::ArrayBlob
    {
        let value: &DomValue = &*(*node).values.data;
        list.data = value.value_blob.data as *const Blob;
        list.count = value.value_blob.size / size_of::<Blob>();
    }
    list
}

// ufbx.c:33142 `ufbx_find_prop`
pub(crate) unsafe fn find_prop<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
) -> Option<&View<Prop, M>> {
    find_prop_len(props, name, strlen(name))
}

// ufbx.c:33143 `ufbx_find_real`
pub(crate) unsafe fn find_real<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def: Real,
) -> Real {
    find_real_len(props, name, strlen(name), def)
}

// ufbx.c:33144 `ufbx_find_vec3`
pub(crate) unsafe fn find_vec3<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def: Vec3,
) -> Vec3 {
    find_vec3_len(props, name, strlen(name), def)
}

// ufbx.c:33145 `ufbx_find_int`
pub(crate) unsafe fn find_int<M: Mode>(props: &View<Props, M>, name: *const u8, def: i64) -> i64 {
    find_int_len(props, name, strlen(name), def)
}

// ufbx.c:33146 `ufbx_find_bool`
pub(crate) unsafe fn find_bool<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def: bool,
) -> bool {
    find_bool_len(props, name, strlen(name), def)
}

// ufbx.c:33147 `ufbx_find_string`
pub(crate) unsafe fn find_string<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def: String,
) -> String {
    find_string_len(props, name, strlen(name), def)
}

// ufbx.c:33148 `ufbx_find_blob`
pub(crate) unsafe fn find_blob<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def: Blob,
) -> Blob {
    find_blob_len(props, name, strlen(name), def)
}

// ufbx.c:33149 `ufbx_find_prop_element`
pub(crate) unsafe fn find_prop_element(
    element: *const Element,
    name: *const u8,
    type_: ElementType,
) -> *mut Element {
    find_prop_element_len(element, name, strlen(name), type_)
}

// ufbx.c:33150 `ufbx_find_element`
pub(crate) unsafe fn find_element(
    scene: *const Scene,
    type_: ElementType,
    name: *const u8,
) -> *mut Element {
    find_element_len(scene, type_, name, strlen(name))
}

// ufbx.c:33151 `ufbx_find_node`
pub(crate) unsafe fn find_node(scene: *const Scene, name: *const u8) -> *mut Node {
    find_node_len(scene, name, strlen(name))
}

// ufbx.c:33152 `ufbx_find_anim_stack`
pub(crate) unsafe fn find_anim_stack(scene: *const Scene, name: *const u8) -> *mut AnimStack {
    find_anim_stack_len(scene, name, strlen(name))
}

// ufbx.c:33153 `ufbx_find_material`
pub(crate) unsafe fn find_material(scene: *const Scene, name: *const u8) -> *mut Material {
    find_material_len(scene, name, strlen(name))
}

// ufbx.c:33154 `ufbx_find_anim_prop`
pub(crate) unsafe fn find_anim_prop(
    layer: *const AnimLayer,
    element: *const Element,
    prop: *const u8,
) -> *mut AnimProp {
    find_anim_prop_len(layer, element, prop, strlen(prop))
}

// ufbx.c:33155 `ufbx_evaluate_prop`
pub(crate) unsafe fn evaluate_prop(
    anim: *const Anim,
    element: *const Element,
    name: *const u8,
    time: f64,
) -> Prop {
    evaluate_prop_len(anim, element, name, strlen(name), time)
}

// ufbx.c:33156 `ufbx_evaluate_prop_flags`
pub(crate) unsafe fn evaluate_prop_flags(
    anim: *const Anim,
    element: *const Element,
    name: *const u8,
    time: f64,
    flags: u32,
) -> Prop {
    evaluate_prop_flags_len(anim, element, name, strlen(name), time, flags)
}

// ufbx.c:33157 `ufbx_find_prop_texture`
pub(crate) unsafe fn find_prop_texture(material: *const Material, name: *const u8) -> *mut Texture {
    find_prop_texture_len(material, name, strlen(name))
}

// ufbx.c:33158 `ufbx_find_shader_prop`
pub(crate) unsafe fn find_shader_prop(shader: *const Shader, name: *const u8) -> String {
    find_shader_prop_len(shader, name, strlen(name))
}

// ufbx.c:33159 `ufbx_find_shader_prop_bindings`
pub(crate) unsafe fn find_shader_prop_bindings(
    shader: *const Shader,
    name: *const u8,
) -> List<ShaderPropBinding> {
    find_shader_prop_bindings_len(shader, name, strlen(name))
}

// ufbx.c:33160 `ufbx_find_shader_texture_input`
pub(crate) unsafe fn find_shader_texture_input(
    shader: *const ShaderTexture,
    name: *const u8,
) -> *mut ShaderTextureInput {
    find_shader_texture_input_len(shader, name, strlen(name))
}

// ufbx.c:33161 `ufbx_dom_find`
pub(crate) unsafe fn dom_find(parent: *const DomNode, name: *const u8) -> *mut DomNode {
    dom_find_len(parent, name, strlen(name))
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
    catch_triangulate_face(core::ptr::null_mut(), indices, num_indices, mesh, face)
}

// ufbx.c:33168-33170 `ufbx_compute_topology`
pub(crate) unsafe fn compute_topology(mesh: *const Mesh, topo: *mut TopoEdge, num_topo: usize) {
    catch_compute_topology(core::ptr::null_mut(), mesh, topo, num_topo)
}

// ufbx.c:33171-33173 `ufbx_topo_next_vertex_edge`
pub(crate) unsafe fn topo_next_vertex_edge(
    topo: *const TopoEdge,
    num_topo: usize,
    index: u32,
) -> u32 {
    catch_topo_next_vertex_edge(core::ptr::null_mut(), topo, num_topo, index)
}

// ufbx.c:33174-33176 `ufbx_topo_prev_vertex_edge`
pub(crate) unsafe fn topo_prev_vertex_edge(
    topo: *const TopoEdge,
    num_topo: usize,
    index: u32,
) -> u32 {
    catch_topo_prev_vertex_edge(core::ptr::null_mut(), topo, num_topo, index)
}

// ufbx.c:33177-33179 `ufbx_get_weighted_face_normal`
pub(crate) unsafe fn get_weighted_face_normal(positions: *const VertexVec3, face: Face) -> Vec3 {
    catch_get_weighted_face_normal(core::ptr::null_mut(), positions, face)
}

#[cfg(test)]
mod tests {
    // Test scaffolding builds dummy element/prop tables with `MaybeUninit::zeroed()`
    // and only reads the POD sort-key fields; the zeroed values are never observed
    // through their non-zeroable fields, so `invalid_value` is allowed for the tests.
    #![allow(invalid_value)]
    use super::*;
    use crate::generated::Error;
    use crate::generated::RawAllocatorOpts;
    use crate::native::allocator::{init_ator, MESH_IMP_MAGIC};
    use crate::native::buf::push_size;
    use crate::native::parse::{get_imp, MeshImp};
    use crate::prelude::Ref;
    use core::ffi::c_void;
    use core::mem::size_of;

    // Build a refcounted object the way the C setup code does: an allocator
    // feeding a result buffer, with the `ufbxi_refcount` header pushed into
    // that same buffer (the header-inside-own-buffer trick `release_ref` must
    // survive).
    unsafe fn make_imp(error: *mut Error, parent: *mut Refcount) -> *mut MeshImp {
        let mut ator = core::mem::MaybeUninit::<Allocator>::zeroed().assume_init();
        let opts = RawAllocatorOpts::default();
        init_ator(error, &mut ator, &opts, b"test\0".as_ptr());

        let mut buf = core::mem::MaybeUninit::<Buf>::zeroed().assume_init();
        buf.ator = &raw mut ator;

        let imp = push_size(&mut buf, size_of::<MeshImp>(), 1) as *mut MeshImp;
        assert!(!imp.is_null());
        core::ptr::write_bytes(imp as *mut u8, 0, size_of::<MeshImp>());
        // Expose the wide allocation so `get_imp` can recover this header via
        // exposed provenance from a (possibly narrowed) public pointer.
        (imp as *mut u8).expose_provenance();
        init_ref(&mut (*imp).refcount, MESH_IMP_MAGIC, parent);
        (*imp).magic = MESH_IMP_MAGIC;

        // Transfer the allocator/buffer into the refcount header, as the C
        // setup paths do before returning the object to the user.
        (*imp).refcount.ator = ator;
        (*imp).refcount.buf = buf;
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

            let mesh_ptr = &raw mut (*imp).mesh as *mut c_void;
            let back: *mut MeshImp = get_imp(mesh_ptr);
            assert_eq!(back, imp);

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
            let child = make_imp(&mut error, &mut (*parent).refcount);
            assert_eq!(
                (*parent)
                    .refcount
                    .refcount
                    .load(core::sync::atomic::Ordering::SeqCst),
                1
            );

            // Releasing the child (count 0) frees it AND iteratively releases
            // the parent, whose count drops from 1 to 0 -> freed too.
            release_ref(&mut (*child).refcount);
        }
    }

    use crate::generated::RawCloseMemoryCb;

    #[test]
    fn test_open_memory_ctx_copy_and_close() {
        unsafe {
            let data = *b"hello, memory stream";
            let mut stream = RawStream::default();
            let mut error = MaybeUninit::<Error>::zeroed().assume_init();
            assert!(open_memory(
                &mut stream,
                data.as_ptr() as *const c_void,
                data.len(),
                core::ptr::null(),
                &mut error,
            ));
            assert_eq!(error.type_ as u32, 0);

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
            (*hits).0 = data as usize;
            (*hits).1 = data_size;
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
                core::ptr::null_mut(),
            ));
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
            let mut error = MaybeUninit::<Error>::zeroed().assume_init();
            let path = b"definitely/not/a/real/file.fbx";
            assert!(!open_file(
                &mut stream,
                path.as_ptr(),
                path.len(),
                core::ptr::null(),
                &mut error,
            ));
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
            let mut error = MaybeUninit::<Error>::zeroed().assume_init();
            assert!(open_file(
                &mut stream,
                path.as_ptr(),
                path.len(),
                core::ptr::null(),
                &mut error,
            ));
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
                find_element_len(core::ptr::null(), ElementType::Node, b"a".as_ptr(), 1).is_null()
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
                    _internal_key: get_name_key(names[i].as_ptr(), names[i].len()),
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

            let find = |t, n: &[u8]| find_element_len(&scene, t, n.as_ptr(), n.len());
            assert_eq!(find(ElementType::Node, b"alpha"), ptrs[0]);
            // Same name, different type: the type is part of the sort key.
            assert_eq!(find(ElementType::Node, b"beta"), ptrs[1]);
            assert_eq!(find(ElementType::AnimStack, b"beta"), ptrs[3]);
            assert_eq!(find(ElementType::Material, b"gamma"), ptrs[2]);
            assert!(find(ElementType::Mesh, b"beta").is_null());
            assert!(find(ElementType::Node, b"delta").is_null());

            // The typed wrappers just pin the element type.
            assert_eq!(
                find_node_len(&scene, b"alpha".as_ptr(), 5),
                ptrs[0] as *mut Node
            );
            assert_eq!(
                find_material_len(&scene, b"gamma".as_ptr(), 5),
                ptrs[2] as *mut Material
            );
            assert_eq!(
                find_anim_stack_len(&scene, b"beta".as_ptr(), 4),
                ptrs[3] as *mut AnimStack
            );
            assert!(find_node_len(&scene, b"gamma".as_ptr(), 5).is_null());

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
                    _internal_key: get_name_key(name.as_ptr(), name.len()),
                    prop_name: String::new_c(name.as_ptr(), name.len()),
                    anim_value: Ref::from_ptr(anim_value),
                })
                .collect();

            let mut layer: AnimLayer = MaybeUninit::zeroed().assume_init();
            layer.anim_props = List::from_slice(&props);
            let base: *const AnimProp = props.as_ptr();

            let find =
                |e: *mut Element, n: &[u8]| find_anim_prop_len(&layer, e, n.as_ptr(), n.len());
            assert_eq!(find(e0, b"Lcl Scaling"), base as *mut AnimProp);
            assert_eq!(find(e0, b"Lcl Translation"), base.add(1) as *mut AnimProp);
            assert_eq!(find(e2, b"Lcl Rotation"), base.add(2) as *mut AnimProp);
            // Wrong element, or an element with no animated props at all.
            assert!(find(e2, b"Lcl Scaling").is_null());
            assert!(find(e1, b"Lcl Scaling").is_null());

            assert_eq!(
                find_anim_prop(&layer, e0, b"Lcl Scaling\0".as_ptr()),
                base as *mut AnimProp
            );

            // `ufbx_find_anim_props` returns the whole per-element run.
            let run = find_anim_props(&layer, e0);
            assert_eq!(run.data, base);
            assert_eq!(run.count, 2);
            let run = find_anim_props(&layer, e2);
            assert_eq!(run.data, base.add(2));
            assert_eq!(run.count, 1);
            // C: `begin == end` leaves the `{ 0 }` initializer untouched.
            let run = find_anim_props(&layer, e1);
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
        let len = format_error(dst.as_mut_ptr(), dst.len(), error);
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
            init_ator(&mut error, &mut ator, &opts, b"test\0".as_ptr());
            let mut buf = core::mem::MaybeUninit::<Buf>::zeroed().assume_init();
            buf.ator = &raw mut ator;
            let imp = push_size(&mut buf, size_of::<SceneImp>(), 1) as *mut SceneImp;
            assert!(!imp.is_null());
            core::ptr::write_bytes(imp as *mut u8, 0, size_of::<SceneImp>());
            // Expose the wide allocation so `get_imp` can recover this header via
            // exposed provenance from a (possibly narrowed) public pointer.
            (imp as *mut u8).expose_provenance();
            init_ref(&mut (*imp).refcount, SCENE_IMP_MAGIC, core::ptr::null_mut());
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
        let mut ator = core::mem::MaybeUninit::<Allocator>::zeroed().assume_init();
        let opts = RawAllocatorOpts::default();
        init_ator(error, &mut ator, &opts, b"test\0".as_ptr());

        let mut buf = core::mem::MaybeUninit::<Buf>::zeroed().assume_init();
        buf.ator = &raw mut ator;

        let imp = push_size(&mut buf, size_of::<T>(), 1) as *mut T;
        assert!(!imp.is_null());
        core::ptr::write_bytes(imp as *mut u8, 0, size_of::<T>());
        let refcount = imp as *mut Refcount;
        init_ref(refcount, magic, core::ptr::null_mut());
        (*refcount).ator = ator;
        (*refcount).buf = buf;
        imp
    }

    unsafe fn refcount_of<T>(imp: *mut T) -> usize {
        (*(imp as *mut Refcount))
            .refcount
            .load(core::sync::atomic::Ordering::SeqCst)
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
            let back: *mut BakedAnimImp = get_imp(bake as *mut c_void);
            assert_eq!(back, imp);

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
            let node_base = nodes.as_ptr() as *mut BakedNode;
            let elem_base = elements.as_ptr() as *mut BakedElement;

            assert_eq!(find_baked_node_by_typed_id(bake_ptr, 1), node_base);
            assert_eq!(find_baked_node_by_typed_id(bake_ptr, 3), node_base.add(1));
            assert_eq!(find_baked_node_by_typed_id(bake_ptr, 5), node_base.add(2));
            // `ufbxi_macro_lower_bound_eq` does NOT write the out-param on a
            // miss; the `SIZE_MAX` pre-initializer is what makes this NULL.
            assert!(find_baked_node_by_typed_id(bake_ptr, 4).is_null());
            assert!(find_baked_node_by_typed_id(bake_ptr, 0).is_null());
            assert!(find_baked_node_by_typed_id(bake_ptr, 9).is_null());

            assert_eq!(find_baked_element_by_element_id(bake_ptr, 2), elem_base);
            assert_eq!(
                find_baked_element_by_element_id(bake_ptr, 6),
                elem_base.add(2)
            );
            assert!(find_baked_element_by_element_id(bake_ptr, 3).is_null());

            // The by-pointer wrappers ARE null-checked (unlike the by-id ones).
            let mut node: Node = MaybeUninit::zeroed().assume_init();
            node.element.typed_id = 5;
            node.element.element_id = 6;
            assert_eq!(find_baked_node(bake_ptr, &mut node), node_base.add(2));
            assert!(find_baked_node(core::ptr::null_mut(), &mut node).is_null());
            assert!(find_baked_node(bake_ptr, core::ptr::null_mut()).is_null());

            let element: *mut Element = &raw mut node.element;
            assert_eq!(find_baked_element(bake_ptr, element), elem_base.add(2));
            assert!(find_baked_element(core::ptr::null_mut(), element).is_null());
            assert!(find_baked_element(bake_ptr, core::ptr::null_mut()).is_null());
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
        unsafe {
            // C: no bindings -> `ufbx_empty_string`, not NULL.
            let s = find_shader_prop_len(core::ptr::null(), b"Diffuse".as_ptr(), 7);
            assert_eq!(s.length, 0);
            assert_eq!(s.data, EMPTY_STRING.0.data);
            assert_eq!(
                find_shader_prop(core::ptr::null(), b"Diffuse\0".as_ptr()).length,
                0
            );
        }
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
        let desc = core::slice::from_raw_parts(error.description.data, error.description.length);
        assert_eq!(desc, b"Feature disabled");
        assert_eq!(error.info(), info);
    }

    #[test]
    #[cfg(not(feature = "scene-eval"))]
    fn test_evaluate_scene_feature_disabled() {
        unsafe {
            let mut error = MaybeUninit::<Error>::zeroed().assume_init();
            assert!(evaluate_scene(
                core::ptr::null(),
                core::ptr::null(),
                0.0,
                core::ptr::null(),
                &mut error
            )
            .is_null());
            assert_feature_disabled(&error, "UFBX_ENABLE_SCENE_EVALUATION");
            // C: `error` is optional and the arm is a no-op without it.
            assert!(evaluate_scene(
                core::ptr::null(),
                core::ptr::null(),
                0.0,
                core::ptr::null(),
                core::ptr::null_mut()
            )
            .is_null());
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
            let mut error = MaybeUninit::<Error>::zeroed().assume_init();
            assert!(bake_anim(scene, core::ptr::null(), core::ptr::null(), &mut error).is_null());
            assert_feature_disabled(&error, "UFBX_ENABLE_ANIMATION_BAKING");
            assert!(bake_anim(
                scene,
                core::ptr::null(),
                core::ptr::null(),
                core::ptr::null_mut()
            )
            .is_null());
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
        (&raw mut (*shape).num_offsets).write(offset_vertices.len());
        (&raw mut (*shape).offset_vertices).write(List::from_slice(offset_vertices));
        (&raw mut (*shape).position_offsets).write(List::from_slice(position_offsets));
        (&raw mut (*shape).offset_weights).write(List::from_slice(offset_weights));
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
            assert_eq!(get_blend_shape_offset_index(shape, 1), 0);
            assert_eq!(get_blend_shape_offset_index(shape, 3), 1);
            assert_eq!(get_blend_shape_offset_index(shape, 7), 2);
            assert_eq!(get_blend_shape_offset_index(shape, 0), NO_INDEX);
            assert_eq!(get_blend_shape_offset_index(shape, 2), NO_INDEX);
            assert_eq!(get_blend_shape_offset_index(shape, 8), NO_INDEX);

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
