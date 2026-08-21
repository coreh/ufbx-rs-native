//! Port of the `// -- Curve evaluation` / `// -- Animation evaluation` /
//! `// -- Animation baking` banner sections of ufbx.c.
//!
//! Ported: the `// -- Curve evaluation` section (ufbx.c:25012-25626) —
//! `ufbxi_find_cubic_bezier_t`, `ufbxi_evaluate_skinning` (both
//! `UFBXI_FEATURE_SKINNING_EVALUATION` arms), `ufbxi_fixup_opts_string`,
//! `ufbxi_resolve_warning_elements`, `ufbxi_load_imp`, `ufbxi_free_temp`,
//! `ufbxi_free_result` and `ufbxi_load` (the driver behind the `ufbx_load_*`
//! entry points in `native::api`) — and the whole `// -- Animation evaluation` section
//! (ufbx.c:25627-26670): the prop-override lookups, `ufbxi_combine_anim_layer`
//! and `ufbxi_evaluate_props`, the prop iterator, `ufbxi_evaluate_selected_props`,
//! `ufbxi_extrapolate_curve`, the `UFBXI_FEATURE_SCENE_EVALUATION` block
//! (`ufbxi_eval_context` .. `ufbxi_evaluate_scene`, `feature = "scene-eval"`)
//! and the `ufbxi_create_anim_*` machinery — and the whole
//! `// -- Animation baking` section (ufbx.c:26670-27767): `ufbxi_baked_anim_imp`
//! (which C declares OUTSIDE the feature `#if`, so it exists in every build for
//! the `ufbx_retain_baked_anim` / `ufbx_free_baked_anim` pair) plus the
//! `ufbxi_bake_context` .. `ufbxi_bake_anim_imp` block under
//! `feature = "baking"`, backing `ufbx_bake_anim` in `native::api`.
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
use core::ffi::c_void;
use core::mem::{size_of, MaybeUninit};
use core::ptr;

#[cfg(any(feature = "scene-eval", feature = "baking"))]
use crate::generated::AnimValue;
#[cfg(any(feature = "skinning-eval", feature = "scene-eval", feature = "baking"))]
use crate::generated::Node as UfbxNode;
use crate::generated::{
    Anim, AnimCurve, AnimLayer, AnimProp, BakedAnim, Connection, DomNode, Element, Error,
    ErrorType, Extrapolation, ExtrapolationMode, FileFormat, IndexErrorHandling, InflateRetain,
    Keyframe, OpenFileInfo, OpenFileType, Prop, PropFlags, PropOverride, PropType, Quat,
    RawAnimOpts, RawGeometryCacheDataOpts, RawLoadOpts, RawOpenFileOpts, RawPropOverrideDesc,
    RawStream, RotationOrder, Scene, Tangent, TransformOverride, UnicodeErrorHandling, Vec3,
    Warning, WarningType,
};
#[cfg(feature = "scene-eval")]
use crate::generated::{
    AnimStack, AudioLayer, BlendChannel, BlendKeyframe, BlendShape, BonePose, CacheFile, Camera,
    Constraint, ConstraintTarget, DisplayLayer, Material, MaterialMap, MaterialTexture,
    NameElement, Pose, RawEvaluateOpts, SelectionNode, SelectionSet, Shader, ShaderTexture,
    ShaderTextureInput, SkinCluster, StereoCamera, Texture, TextureLayer, Video,
};
#[cfg(feature = "baking")]
use crate::generated::{
    BakeStepHandling, BakedElement, BakedKeyFlags, BakedNode, BakedProp, BakedQuat, BakedVec3,
    ElementType, EvaluateFlags, Interpolation, RawBakeOpts, Transform, TransformFlags,
};
#[cfg(any(feature = "skinning-eval", feature = "scene-eval"))]
use crate::generated::{BlendDeformer, CacheDeformer, Mesh, SkinDeformer};
#[cfg(feature = "skinning-eval")]
use crate::generated::{CacheChannel, CacheInterpretation, Matrix, TopoEdge};
use crate::native::allocator::{
    free, free_ator, init_ator, Allocator, SCENE_IMP_MAGIC, ZERO_SIZE_BUFFER,
};
#[cfg(feature = "baking")]
use crate::native::allocator::{grow_array, BAKED_ANIM_IMP_MAGIC};
#[cfg(feature = "skinning-eval")]
use crate::native::api::{
    add_blend_vertex_offsets, catch_get_skin_vertex_matrix, compute_normals, compute_topology,
    generate_normal_mapping, sample_geometry_cache_vec3, transform_position, ZERO_VEC3,
};
use crate::native::api::{coordinate_axes_valid, default_open_file, open_file_ctx};
use crate::native::api::{
    euler_to_quat, evaluate_anim_value_real_flags, evaluate_anim_value_vec3_flags,
    evaluate_curve_flags, evaluate_prop_flags_len, evaluate_prop_len, init_ref, quat_slerp,
    quat_to_euler, IDENTITY_QUAT,
};
#[cfg(feature = "baking")]
use crate::native::api::{evaluate_baked_vec3, evaluate_transform_flags, quat_fix_antipodal};
#[cfg(feature = "scene-eval")]
use crate::native::api::{evaluate_props_flags, ELEMENT_TYPE_SIZE};
#[cfg(feature = "baking")]
use crate::native::buf::{buf_clear, pop};
use crate::native::buf::{buf_free, push_copy, Buf, BufView};
use crate::native::cache::{load_external_files, scale_units, transform_to_axes};
#[cfg(not(feature = "skinning-eval"))]
use crate::native::error::ufbxi_report_err_msg;
use crate::native::error::{
    clear_error, fix_error_type, set_err_info, strcmp, strlen, ufbxi_check, ufbxi_check_err,
    ufbxi_check_err_msg, ufbxi_check_msg, ufbxi_fail_err_msg, ufbxi_fail_msg, ufbxi_fmt_err_info,
    utf8_valid_length, Fail, EMPTY_CHAR,
};
use crate::native::float_parse::parse_double_init_flags;
use crate::native::hash::{
    map_cmp_const_char_ptr, map_cmp_ptr_id, map_cmp_uint64, map_cmp_uintptr, map_free, map_init,
};
use crate::native::obj::{mtl_load, obj_free, obj_load};
use crate::native::parse::{
    begin_parse, determine_format, finish_imp, get_imp, get_name_key, get_name_key_c, load_maps,
    load_strings, Context, Node, Refcount, SceneImp, ELEMENT_TYPE_COUNT, MIN_FILE_FORMAT_LOOKAHEAD,
};
#[cfg(feature = "baking")]
use crate::native::parse::{find_prop, is_vec3_zero, PropView, PropsView};
#[cfg(feature = "skinning-eval")]
use crate::native::platform::max_sz;
use crate::native::platform::{
    add_ptr, f64_to_i64, macro_lower_bound_eq, macro_upper_bound_eq, math, ufbx_assert,
    ufbxi_dev_assert, ufbxi_ignore, unstable_sort, PATH_SEPARATOR,
};
#[cfg(feature = "baking")]
use crate::native::platform::{macro_stable_sort, ufbxi_unreachable};
#[cfg(any(feature = "skinning-eval", feature = "scene-eval", feature = "baking"))]
use crate::native::read::opt_ptr;
use crate::native::read::{
    init_file_paths, open_file, read_legacy_root, read_root, ref_ptr, supports_version,
    SYNTHETIC_ID_START,
};
use crate::native::scene_process::{
    finalize_scene, find_anim_prop_start, find_prop_connection, modify_geometry, mul_quat,
    postprocess_scene, pre_finalize_scene, update_adjust_transforms, update_scene,
    update_scene_metadata, update_scene_settings, update_scene_settings_obj, AnimImp,
};
#[cfg(feature = "scene-eval")]
use crate::native::scene_process::{MATERIAL_FBX_MAP_COUNT, MATERIAL_PBR_MAP_COUNT};
use crate::native::string_pool as sp;
#[cfg(feature = "baking")]
use crate::native::string_pool::lerp3;
use crate::native::string_pool::{
    map_cmp_string, push_string_place_str, str_equal, str_less, string_pool_temp_free, STRINGS,
};
use crate::native::thread::{thread_pool_free, thread_pool_init, THREAD_GROUP_COUNT};
#[cfg(feature = "baking")]
use crate::native::view::SliceViewIter;
#[cfg(any(feature = "skinning-eval", feature = "scene-eval"))]
use crate::native::view::View;
use crate::native::warnings::{pop_warnings, ufbxi_warnf};
use crate::prelude::as_f64;
use crate::prelude::{List, OpenFileContext, Real, Ref, String};

// -- Curve evaluation (ufbx.c:25012)

// ufbx.c:25014-25053 `ufbxi_find_cubic_bezier_t`
// C: `ufbxi_forceinline` — float order verbatim (PORTING.md "Floats"); all
// libm via the `platform::math` shims.
#[inline(always)]
pub(crate) fn find_cubic_bezier_t(p1: f64, p2: f64, x0: f64) -> f64 {
    use crate::native::platform::math;

    let p1_3: f64 = p1 * 3.0;
    let p2_3: f64 = p2 * 3.0;
    let a: f64 = p1_3 - p2_3 + 1.0;
    let b: f64 = p2_3 - p1_3 - p1_3;
    let c: f64 = p1_3;

    let a_3: f64 = 3.0 * a;
    let b_2: f64 = 2.0 * b;
    let mut t: f64 = x0;
    let mut x1: f64;
    let mut t2: f64;
    let mut t3: f64;

    // Manually unroll three iterations of Newton-Raphson, this is enough
    // for most tangents
    t2 = t * t;
    t3 = t2 * t;
    x1 = a * t3 + b * t2 + c * t - x0;
    t -= x1 / (a_3 * t2 + b_2 * t + c);

    t2 = t * t;
    t3 = t2 * t;
    x1 = a * t3 + b * t2 + c * t - x0;
    t -= x1 / (a_3 * t2 + b_2 * t + c);

    t2 = t * t;
    t3 = t2 * t;
    x1 = a * t3 + b * t2 + c * t - x0;
    t -= x1 / (a_3 * t2 + b_2 * t + c);

    // 4 ULP from 1.0
    // C: `const double eps = 8.881784197001252e-16;`
    let eps: f64 = 8.881784197001252e-16;
    if math::fabs(x1) <= eps {
        return t;
    }

    // Perform more iterations until we reach desired accuracy
    for _i in 0..4usize {
        t2 = t * t;
        t3 = t2 * t;
        x1 = a * t3 + b * t2 + c * t - x0;
        t -= x1 / (a_3 * t2 + b_2 * t + c);

        t2 = t * t;
        t3 = t2 * t;
        x1 = a * t3 + b * t2 + c * t - x0;
        t -= x1 / (a_3 * t2 + b_2 * t + c);

        if math::fabs(x1) <= eps {
            return t;
        }
    }

    t
}

// ufbx.c:25055-25169 `ufbxi_evaluate_skinning`
// C forks on `#if UFBXI_FEATURE_SKINNING_EVALUATION` inside the function; the
// arms are split into cfg-gated fns (the same split `ufbxi_obj_load` uses in
// `native::obj`). C return type `int` (1 = success) → `Result<(), Fail>`.
#[cfg(feature = "skinning-eval")]
#[inline(never)]
pub(crate) unsafe fn evaluate_skinning(
    scene: *mut Scene,
    error: *mut Error,
    buf_result: &BufView,
    buf_tmp: &BufView,
    time: f64,
    load_caches: bool,
    cache_opts: *mut RawGeometryCacheDataOpts,
) -> Result<(), Fail> {
    let mut max_skinned_indices: usize = 0;

    // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, scene->meshes)`
    // SAFETY: `scene` is the caller's live `ufbx_scene` — the raw-pointer
    // contract of this `unsafe fn`.
    let mut p_mesh: *mut *mut Mesh = unsafe { (*scene).meshes.data as *mut *mut Mesh };
    // SAFETY: as above, reading the same live scene's mesh-list count.
    let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, unsafe { (*scene).meshes.count });
    while p_mesh != p_mesh_end {
        // SAFETY: `p_mesh` walks the scene's mesh pointer list and stops at
        // `p_mesh_end`, so it addresses a live slot holding a scene-owned mesh.
        let mesh: *mut Mesh = unsafe { *p_mesh };
        // SAFETY: `mesh` is the scene-owned `ufbx_mesh` just read out of the
        // scene's element list — a live, initialized mesh in the scene's result
        // arena, reached through `*mut` (write-capable provenance for `Mut`).
        let mesh = unsafe { View::<Mesh>::from_ptr(mesh) };
        if mesh.blend_deformers().count == 0
            && mesh.skin_deformers().count == 0
            && (mesh.cache_deformers().count == 0 || !load_caches)
        {
            // SAFETY: `p_mesh` is inside the list, so `p_mesh + 1` is at most
            // one past its end.
            p_mesh = unsafe { p_mesh.add(1) };
            continue;
        }
        max_skinned_indices = max_sz(max_skinned_indices, mesh.num_indices());
        // SAFETY: `p_mesh` is inside the list, so `p_mesh + 1` is at most one
        // past its end.
        p_mesh = unsafe { p_mesh.add(1) };
    }

    let topo: *mut TopoEdge = buf_tmp.push::<TopoEdge>(max_skinned_indices);
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr(error) },
        !topo.is_null(),
        "topo"
    );

    // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, scene->meshes)`
    // SAFETY: `scene` is the caller's live `ufbx_scene` — the raw-pointer
    // contract of this `unsafe fn`.
    let mut p_mesh: *mut *mut Mesh = unsafe { (*scene).meshes.data as *mut *mut Mesh };
    // SAFETY: as above, reading the same live scene's mesh-list count.
    let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, unsafe { (*scene).meshes.count });
    while p_mesh != p_mesh_end {
        // SAFETY: `p_mesh` walks the scene's mesh pointer list and stops at
        // `p_mesh_end`, so it addresses a live slot holding a scene-owned mesh.
        let mesh: *mut Mesh = unsafe { *p_mesh };
        // SAFETY: `mesh` is the scene-owned `ufbx_mesh` just read out of the
        // scene's element list — a live, initialized mesh in the scene's result
        // arena, reached through `*mut` (write-capable provenance for `Mut`).
        let mesh = unsafe { View::<Mesh>::from_ptr(mesh) };
        if mesh.blend_deformers().count == 0
            && mesh.skin_deformers().count == 0
            && (mesh.cache_deformers().count == 0 || !load_caches)
        {
            // SAFETY: `p_mesh` is inside the list, so `p_mesh + 1` is at most
            // one past its end.
            p_mesh = unsafe { p_mesh.add(1) };
            continue;
        }
        if mesh.num_vertices() == 0 {
            // SAFETY: `p_mesh` is inside the list, so `p_mesh + 1` is at most
            // one past its end.
            p_mesh = unsafe { p_mesh.add(1) };
            continue;
        }

        let num_vertices: usize = mesh.num_vertices();
        let mut result_pos: *mut Vec3 = buf_result.push::<Vec3>(num_vertices + 1);
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            !result_pos.is_null(),
            "result_pos"
        );

        // C: `result_pos[0] = ufbx_zero_vec3; result_pos++;`
        // SAFETY: `result_pos` is the non-null `num_vertices + 1`-element result
        // allocation pushed just above, so slot 0 is writable.
        unsafe { *result_pos = ZERO_VEC3 };
        // SAFETY: the allocation holds `num_vertices + 1 >= 2` elements, so
        // `result_pos + 1` is in bounds.
        result_pos = unsafe { result_pos.add(1) };

        let mut cached_position: bool = false;
        let mut cached_normals: bool = false;
        if load_caches && mesh.cache_deformers().count > 0 {
            // C: `ufbxi_for_ptr_list(ufbx_cache_deformer, p_cache, mesh->cache_deformers)`
            let mut p_cache: *mut *mut CacheDeformer =
                mesh.cache_deformers().data as *mut *mut CacheDeformer;
            let p_cache_end: *mut *mut CacheDeformer =
                add_ptr(p_cache, mesh.cache_deformers().count);
            while p_cache != p_cache_end {
                // SAFETY: `p_cache` walks the mesh's cache-deformer pointer list
                // and stops at `p_cache_end`, so it addresses a live slot holding
                // a scene-owned `ufbx_cache_deformer`; `external_channel` is that
                // deformer's own `Option<Ref<CacheChannel>>` field, which
                // `opt_ptr` reads as the nullable element pointer it is.
                let channel: *mut CacheChannel =
                    unsafe { opt_ptr(&raw const (*(*p_cache)).external_channel) };
                if channel.is_null() {
                    // SAFETY: `p_cache` is inside the list, so `p_cache + 1` is at
                    // most one past its end.
                    p_cache = unsafe { p_cache.add(1) };
                    continue;
                }

                // SAFETY (this condition): `channel` is a non-null (checked just
                // above) scene-owned `ufbx_cache_channel`.
                if (unsafe { (*channel).interpretation } == CacheInterpretation::VertexPosition
                    || unsafe { (*channel).interpretation } == CacheInterpretation::Points)
                    && !cached_position
                {
                    // SAFETY: `channel` is a live scene-owned cache channel,
                    // `result_pos` addresses `num_vertices` writable `ufbx_vec3`
                    // slots of the result allocation, and `cache_opts` is the
                    // caller's options pointer — `sample_geometry_cache_vec3`'s
                    // contract.
                    let num_read: usize = unsafe {
                        sample_geometry_cache_vec3(
                            channel,
                            time,
                            result_pos,
                            num_vertices,
                            cache_opts,
                        )
                    };
                    if num_read == num_vertices {
                        mesh.set_skinned_is_local(true);
                        cached_position = true;
                    }
                // SAFETY: `channel` is a non-null scene-owned cache channel.
                } else if unsafe { (*channel).interpretation } == CacheInterpretation::VertexNormal
                    && !cached_normals
                {
                    // TODO: Is this right at all?
                    let num_normals: usize = mesh.skinned_normal().values().count;
                    let mut normal_data: *mut Vec3 = buf_result.push::<Vec3>(num_normals + 1);
                    ufbxi_check_err!(
                        unsafe { crate::native::error::ErrorView::from_ptr(error) },
                        !normal_data.is_null(),
                        "normal_data"
                    );
                    // C: `normal_data[0] = ufbx_zero_vec3; normal_data++;`
                    // SAFETY: `normal_data` is the non-null `num_normals + 1`
                    // element result allocation pushed just above, so slot 0 is
                    // writable.
                    unsafe { *normal_data = ZERO_VEC3 };
                    // SAFETY: that allocation holds `num_normals + 1 >= 1`
                    // elements, so `normal_data + 1` is at most one past its end.
                    normal_data = unsafe { normal_data.add(1) };

                    // SAFETY: `channel` is a live scene-owned cache channel,
                    // `normal_data` addresses `num_normals` writable `ufbx_vec3`
                    // slots of the result allocation, and `cache_opts` is the
                    // caller's options pointer.
                    let num_read: usize = unsafe {
                        sample_geometry_cache_vec3(
                            channel,
                            time,
                            normal_data,
                            num_normals,
                            cache_opts,
                        )
                    };
                    if num_read == num_normals {
                        cached_normals = true;
                        // SAFETY: `values_raw` projects the mesh's own
                        // `skinned_normal.values` list, inheriting the view's
                        // write-capable provenance, so its `data` field is
                        // writable in place.
                        unsafe {
                            (*mesh.skinned_normal().values_raw()).data = normal_data as *const Vec3;
                        }
                    }
                }
                // SAFETY: `p_cache` is inside the deformer list, so `p_cache + 1`
                // is at most one past its end.
                p_cache = unsafe { p_cache.add(1) };
            }
        }

        if !cached_position {
            // C: `memcpy(result_pos, mesh->vertices.data, num_vertices * sizeof(ufbx_vec3));`
            // SAFETY: the mesh's `vertices` list holds
            // `num_vertices == mesh->num_vertices` elements; `result_pos`
            // addresses that many writable slots of the freshly pushed
            // allocation, which as a fresh push cannot overlap the previously
            // allocated source vertex run (even when both live in the same
            // result buffer, as on the load path).
            unsafe { ptr::copy_nonoverlapping(mesh.vertices().data, result_pos, num_vertices) };

            // C: `ufbxi_for_ptr_list(ufbx_blend_deformer, p_blend, mesh->blend_deformers)`
            let mut p_blend: *mut *mut BlendDeformer =
                mesh.blend_deformers().data as *mut *mut BlendDeformer;
            let p_blend_end: *mut *mut BlendDeformer =
                add_ptr(p_blend, mesh.blend_deformers().count);
            while p_blend != p_blend_end {
                // SAFETY: `p_blend` walks the mesh's blend-deformer pointer list
                // and stops at `p_blend_end`, so it addresses a live slot holding
                // a scene-owned `ufbx_blend_deformer`; `result_pos` addresses
                // `num_vertices` writable `ufbx_vec3` slots.
                unsafe { add_blend_vertex_offsets(*p_blend, result_pos, num_vertices, 1.0) };
                // SAFETY: `p_blend` is inside the list, so `p_blend + 1` is at
                // most one past its end.
                p_blend = unsafe { p_blend.add(1) };
            }

            // TODO: What should we do about multiple skins??
            if mesh.skin_deformers().count > 0 {
                // C: `ufbx_matrix *fallback = mesh->instances.count > 0 ? &mesh->instances.data[0]->geometry_to_world : NULL;`
                // (`mesh->instances` reads through the anonymous `ufbx_element`
                // union member; the generated bindings spell it out.)
                // SAFETY: the `count > 0` guard puts index 0 inside the mesh's
                // instance list, whose slots hold scene-owned `ufbx_node` refs,
                // so the projection addresses that node's own
                // `geometry_to_world` field.
                let fallback: *mut Matrix = if mesh.element().instances().count > 0 {
                    unsafe {
                        &raw mut (*ref_ptr::<UfbxNode>(mesh.element().instances().data.add(0)))
                            .geometry_to_world
                    }
                } else {
                    ptr::null_mut()
                };
                // SAFETY: `skin_deformers.count > 0` (checked above) puts index 0
                // inside the mesh's skin-deformer list, whose slots hold
                // scene-owned `ufbx_skin_deformer` refs.
                let skin: *mut SkinDeformer = unsafe { ref_ptr(mesh.skin_deformers().data.add(0)) };
                for i in 0..num_vertices {
                    // C: `ufbx_get_skin_vertex_matrix(skin, i, fallback)` — the
                    // `ufbx_inline` wrapper in ufbx.h (5601-5603) forwarding to
                    // the catch impl with a NULL panic.
                    // SAFETY: `skin` is the live scene-owned deformer minted
                    // above, so the `Const` view reads a readable pointee, and
                    // `fallback` is either null or the live instance node's
                    // matrix — `catch_get_skin_vertex_matrix`'s contract.
                    let mat: Matrix = unsafe {
                        catch_get_skin_vertex_matrix(
                            None,
                            crate::native::view::View::<SkinDeformer, crate::native::view::Const>::from_ptr(skin),
                            i,
                            fallback,
                        )
                    };
                    // SAFETY: `i < num_vertices`, so `result_pos + i` is inside
                    // the pushed result allocation, readable and writable;
                    // the `*const Matrix` `transform_position` reads is derived
                    // from a borrow of the live local `mat`.
                    unsafe { *result_pos.add(i) = transform_position(&mat, *result_pos.add(i)) };
                }

                mesh.set_skinned_is_local(false);
            }
        }

        // SAFETY: `values_raw` projects the mesh's own
        // `skinned_position.values` list, inheriting the view's write-capable
        // provenance, so its `data` field is writable in place.
        unsafe { (*mesh.skinned_position().values_raw()).data = result_pos as *const Vec3 };

        if !cached_normals {
            let num_indices: usize = mesh.num_indices();
            let normal_indices: *mut u32 = buf_result.push::<u32>(num_indices);
            ufbxi_check_err!(
                unsafe { crate::native::error::ErrorView::from_ptr(error) },
                !normal_indices.is_null(),
                "normal_indices"
            );

            // SAFETY: `mesh.as_ptr()` addresses the scene-owned mesh this view
            // was minted over (read-only use) and `topo` is the non-null
            // `max_skinned_indices`-element scratch allocation, where
            // `max_skinned_indices >= mesh->num_indices` (the first loop maxed
            // over a superset of the meshes this loop skins) covers
            // `num_indices` entries.
            unsafe { compute_topology(mesh.as_ptr(), topo, num_indices) };
            // SAFETY: as above for `mesh`/`topo`; `normal_indices` is the non-null
            // `num_indices`-element result allocation pushed just above.
            let num_normals: usize = unsafe {
                generate_normal_mapping(
                    mesh.as_ptr(),
                    topo,
                    num_indices,
                    normal_indices,
                    num_indices,
                    false,
                )
            };

            if num_normals == mesh.num_vertices() {
                mesh.skinned_normal().set_unique_per_vertex(true);
            }

            let mut normal_data: *mut Vec3 = buf_result.push::<Vec3>(num_normals + 1);
            ufbxi_check_err!(
                unsafe { crate::native::error::ErrorView::from_ptr(error) },
                !normal_data.is_null(),
                "normal_data"
            );

            // C: `normal_data[0] = ufbx_zero_vec3; normal_data++;`
            // SAFETY: `normal_data` is the non-null `num_normals + 1`-element
            // result allocation pushed just above, so slot 0 is writable.
            unsafe { *normal_data = ZERO_VEC3 };
            // SAFETY: that allocation holds `num_normals + 1 >= 1` elements, so
            // `normal_data + 1` is at most one past its end.
            normal_data = unsafe { normal_data.add(1) };

            // SAFETY: `mesh.as_ptr()` and the `skinned_position` projection both
            // address the scene-owned mesh this view was minted over (read-only
            // use); `normal_indices` holds the `num_indices` indices
            // `generate_normal_mapping` wrote and `normal_data` addresses
            // `num_normals` writable `ufbx_vec3` slots.
            unsafe {
                compute_normals(
                    mesh.as_ptr(),
                    mesh.skinned_position().as_ptr(),
                    normal_indices,
                    num_indices,
                    normal_data,
                    num_normals,
                )
            };

            mesh.set_generated_normals(true);
            mesh.skinned_normal().set_exists(true);
            // SAFETY (the four writes below): `values_raw`/`indices_raw` project
            // the mesh's own `skinned_normal` lists, inheriting the view's
            // write-capable provenance; the buffers stored into them are the
            // result allocations pushed above, which live as long as the result
            // buffer the scene keeps.
            unsafe { (*mesh.skinned_normal().values_raw()).data = normal_data as *const Vec3 };
            unsafe { (*mesh.skinned_normal().values_raw()).count = num_normals };
            unsafe { (*mesh.skinned_normal().indices_raw()).data = normal_indices as *const u32 };
            unsafe { (*mesh.skinned_normal().indices_raw()).count = num_indices };
            mesh.skinned_normal().set_value_reals(3);
        }

        // SAFETY: `p_mesh` is inside the mesh list, so `p_mesh + 1` is at most
        // one past its end.
        p_mesh = unsafe { p_mesh.add(1) };
    }

    Ok(())
}

// ufbx.c:25164-25168 `ufbxi_evaluate_skinning` (`#else` branch — feature
// disabled). C parity, NOT a stub: `ufbxi_report_err_msg` records the error
// and KEEPS GOING (PORTING.md trap #16); the `return 0` that follows becomes
// `Err(Fail)`.
#[cfg(not(feature = "skinning-eval"))]
#[inline(never)]
pub(crate) unsafe fn evaluate_skinning(
    scene: *mut Scene,
    error: *mut Error,
    buf_result: &BufView,
    buf_tmp: &BufView,
    time: f64,
    load_caches: bool,
    cache_opts: *mut RawGeometryCacheDataOpts,
) -> Result<(), Fail> {
    // C: all parameters other than `error` are unreferenced in the `#else` arm.
    let _ = (scene, buf_result, buf_tmp, time, load_caches, cache_opts);
    // SAFETY: `error` is the caller's live `ufbx_error` slot — the raw-pointer
    // contract of this `unsafe fn` — which is what the macro formats into.
    unsafe { ufbxi_fmt_err_info!(error, "UFBX_ENABLE_SKINNING_EVALUATION") };
    ufbxi_report_err_msg!(
        unsafe { crate::native::error::ErrorView::from_ptr(error) },
        "UFBXI_FEATURE_SKINNING_EVALUATION",
        "Feature disabled"
    );
    Err(Fail)
}

// ufbx.c:25171-25185 `ufbxi_fixup_opts_string`
#[inline(never)]
pub(crate) unsafe fn fixup_opts_string(
    uc: &Context,
    str: *mut String,
    push: bool,
) -> Result<(), Fail> {
    // SAFETY (every access to `*str` in this fn): `str` is the caller's live
    // `ufbx_string` slot — the raw-pointer contract of this `unsafe fn`.
    if unsafe { (*str).length } > 0 {
        if unsafe { (*str).length } == usize::MAX {
            // C: `str->length = str->data ? strlen(str->data) : 0;`
            unsafe {
                (*str).length = if !(*str).data.is_null() {
                    // SAFETY: `str->data` is non-null (checked) and, with
                    // `length == SIZE_MAX`, the caller declares it a
                    // NUL-terminated C string.
                    strlen((*str).data)
                } else {
                    0
                }
            };
        }
        if push {
            // SAFETY: `uc`'s string pool is live for the borrow and `str` is the
            // caller's live string slot, which the pool rewrites in place.
            unsafe { push_string_place_str(uc.string_pool_mut_ptr(), str, false)? };
        }
    } else {
        unsafe { (*str).data = EMPTY_CHAR.as_ptr() };
    }

    Ok(())
}

// ufbx.c:25187-25202 `ufbxi_resolve_warning_elements`
#[inline(never)]
pub(crate) fn resolve_warning_elements(uc: &Context) -> Result<(), Fail> {
    let num_elements: usize = uc.tmp_element_id_view().num_items();
    // Pops uc's element-id stack into uc's tmp buf; `num_elements` is that
    // stack's own item count, so the pop is exact.
    let element_ids: *mut u32 = uc
        .tmp_view()
        .push_pop::<u32>(uc.tmp_element_id_view(), num_elements);
    ufbxi_check!(uc, !element_ids.is_null(), "element_ids");

    // C: `ufbxi_for_list(ufbx_warning, warning, uc->scene.metadata.warnings)`
    let mut warning: *mut Warning =
        uc.scene_view().metadata_view().warnings_view().data() as *mut Warning;
    let warning_end: *mut Warning = add_ptr(
        warning,
        uc.scene_view().metadata_view().warnings_view().count(),
    );
    while warning != warning_end {
        // SAFETY: `warning` walks the scene's own warning run
        // (`data`/`count`), and the tagged ids it carries were assigned by
        // `ufbxi_vwarnf_imp` as positions in the very element-id stack that
        // was just popped into `element_ids` — see HACK(warning-element) —
        // so the decoded index is `< num_elements`.
        unsafe {
            let element_id: u32 = (*warning).element_id;
            // Decode `element_id`, see HACK(warning-element) in `ufbxi_vwarnf_imp()` for the encoding.
            if (element_id & 0x80000000u32) != 0 && element_id != !0u32 {
                (*warning).element_id = *element_ids.add((element_id & !0x80000000u32) as usize);
            }
            warning = warning.add(1);
        }
    }

    Ok(())
}

// ufbx.c:25204-25410 `ufbxi_load_imp`
// Stays `unsafe fn`: this is the load orchestrator, and nearly every statement
// in it is a call to another `unsafe fn` taking the same `&Context`
// (`load_strings`, `determine_format`, `read_root`, `finalize_scene`,
// `postprocess_scene`, ...). Flipping it today would mean ~20 blocks all
// citing the one uc construction invariant, restating it rather than
// discharging anything; the honest win arrives structurally once those
// callees themselves go safe.
#[inline(never)]
pub(crate) unsafe fn load_imp(uc: &Context) -> Result<(), Fail> {
    // Check for deferred failure
    if uc.deferred_failure() {
        return Err(Fail);
    }
    if uc.deferred_load() {
        // C: `ufbx_stream stream = { 0 };` / `ufbx_open_file_opts opts = { 0 };`
        // SAFETY: `ufbx_stream` is C plain data — nullable `Option<extern fn>`
        // callbacks and a `*mut c_void` user pointer — so the all-zero pattern is
        // a valid inhabitant (`None` callbacks, null user).
        let mut stream: RawStream = unsafe { MaybeUninit::zeroed().assume_init() };
        // SAFETY: `ufbx_open_file_opts` is C plain data (a `bool` flag plus
        // POD sub-structs), for which all-zero is a valid inhabitant.
        let mut opts: RawOpenFileOpts = unsafe { MaybeUninit::zeroed().assume_init() };
        let filename: *const u8 = uc.load_filename();
        let mut filename_len: usize = uc.load_filename_len();
        let ok: bool;
        if filename_len == usize::MAX {
            opts.filename_null_terminated = true;
            // SAFETY: a `SIZE_MAX` length is the caller's declaration that
            // `filename` is a NUL-terminated C string (the `ufbx_load_file`
            // contract), so it has a readable NUL.
            filename_len = unsafe { strlen(filename) };
        }
        if uc.opts_view().filename_view().length() == 0
            || uc.opts_view().filename_view().data().is_null()
        {
            uc.opts_view().filename_view().set_data(filename);
            uc.opts_view().filename_view().set_length(filename_len);
        }
        // C: `ufbx_error error; error.type = UFBX_ERROR_NONE;` — C initializes
        // only `type`; the struct is only copied below after the open-file
        // path fully wrote it (zero-filled here, C leaves the rest uninit).
        // SAFETY: `ufbx_error` is C plain data — an enum whose `NONE` variant is
        // discriminant 0, counts, and inline character arrays — so the all-zero
        // pattern is a valid inhabitant.
        let mut error: Error = unsafe { MaybeUninit::zeroed().assume_init() };
        error.type_ = ErrorType::None;
        // C: `uc->opts.open_file_cb.fn == &ufbx_default_open_file` — compare
        // by address against the ONE exported `ufbx_default_open_file` object
        // (`native::api::default_open_file` carries the `export_name`, so the
        // default stored by `ufbxi_load` and a C caller's assignment both
        // resolve to that symbol). The lint fears CGU-local duplicates; a
        // spurious mismatch here would only skip the fast path and route the
        // same callback through `ufbxi_open_file` below, but the address is
        // link-unique for the exported item, matching the C comparison.
        let default_fn: unsafe extern "C" fn(
            *mut c_void,
            *mut RawStream,
            *const u8,
            usize,
            *const OpenFileInfo,
        ) -> bool = default_open_file;
        #[allow(unpredictable_function_pointer_comparisons)]
        let open_with_default = uc.opts_view().open_main_file_with_default()
            || uc.opts_view().open_file_cb_view().fn_() == Some(default_fn);
        if open_with_default {
            let ctx: OpenFileContext = uc.ator_tmp_mut_ptr() as OpenFileContext;
            // SAFETY: `stream`/`opts`/`error` are live locals of this frame,
            // `ctx` is `uc`'s own temp allocator (live for the `&Context`
            // borrow), and `filename`/`filename_len` describe the caller's
            // filename run — `open_file_ctx`'s contract.
            ok = unsafe {
                open_file_ctx(&mut stream, ctx, filename, filename_len, &opts, &mut error)
            };
        } else {
            // SAFETY: the callback pointer and temp allocator are `uc`'s own
            // fields (live for the borrow), `stream` is a live local, and
            // `uc.load_filename()`/`filename_len` describe the caller's filename
            // run — `open_file`'s contract.
            ok = unsafe {
                open_file(
                    uc.opts_view().open_file_cb_ptr(),
                    &mut stream,
                    uc.load_filename(),
                    filename_len,
                    ptr::null(),
                    uc.ator_tmp_mut_ptr(),
                    OpenFileType::MainModel,
                )
            };
        }
        if !ok {
            if error.type_ != ErrorType::None {
                // cppcheck-suppress uninitStructMember
                // C: `uc->error = error;` (struct copy)
                // SAFETY: source is a borrow of the live local `error`,
                // destination is `uc`'s own error field (live for the borrow),
                // and the two are distinct allocations.
                unsafe { ptr::copy_nonoverlapping(&error, uc.error_mut_ptr(), 1) };
            } else {
                // SAFETY: `uc`'s error field is live for the borrow and
                // `filename`/`filename_len` describe the caller's filename run.
                unsafe { set_err_info(uc.error_mut_ptr(), filename, filename_len) };
            }
            ufbxi_fail_msg!(uc, "open_file_fn()", "File not found");
        }
        uc.set_read_fn(stream.read_fn);
        uc.set_skip_fn(stream.skip_fn);
        uc.set_size_fn(stream.size_fn);
        uc.set_close_fn(stream.close_fn);
        uc.set_read_user(stream.user);
    }

    if uc.opts_view().progress_cb().fn_.is_some()
        && uc.progress_bytes_total() == 0
        && uc.size_fn().is_some()
    {
        // SAFETY: `size_fn` is `Some` (checked in the condition above) and is
        // called with its paired `read_user`, per the C stream-callback contract.
        let total: u64 = unsafe { (uc.size_fn().unwrap())(uc.read_user()) };
        ufbxi_check!(uc, total != u64::MAX, "total != UINT64_MAX");
        uc.set_progress_bytes_total(total);
    }

    ufbxi_check!(
        uc,
        uc.opts_view().path_separator() >= 0x20 && uc.opts_view().path_separator() <= 0x7e,
        "uc->opts.path_separator >= 0x20 && uc->opts.path_separator <= 0x7e"
    );

    // C: `ufbxi_check(ufbxi_<callee>(uc))` — the caller-side check pushes its
    // own error-stack frame (function/line/#cond) on top of the callee's; a
    // bare `?` would drop that frame and shorten `ufbx_error.stack_size`
    // (checklist #13; test `error_format_long` asserts `stack_size >= 2`).
    ufbxi_check!(
        uc,
        // SAFETY: the string is `uc`'s own `opts.filename` field, live for the
        // `&Context` borrow — `fixup_opts_string`'s raw-pointer contract.
        unsafe { fixup_opts_string(uc, uc.opts_view().filename_mut_ptr() as *mut String, false) }
            .is_ok(),
        "ufbxi_fixup_opts_string(uc, &uc->opts.filename, false)"
    );
    ufbxi_check!(
        uc,
        // SAFETY: the string is `uc`'s own `opts.obj_mtl_path` field, live for
        // the `&Context` borrow.
        unsafe {
            fixup_opts_string(
                uc,
                uc.opts_view().obj_mtl_path_mut_ptr() as *mut String,
                true,
            )
        }
        .is_ok(),
        "ufbxi_fixup_opts_string(uc, &uc->opts.obj_mtl_path, true)"
    );
    ufbxi_check!(
        uc,
        // SAFETY: the string is `uc`'s own `opts.geometry_transform_helper_name`
        // field, live for the `&Context` borrow.
        unsafe {
            fixup_opts_string(
                uc,
                uc.opts_view().geometry_transform_helper_name_mut_ptr() as *mut String,
                true,
            )
        }
        .is_ok(),
        "ufbxi_fixup_opts_string(uc, &uc->opts.geometry_transform_helper_name, true)"
    );
    ufbxi_check!(
        uc,
        // SAFETY: the string is `uc`'s own `opts.scale_helper_name` field, live
        // for the `&Context` borrow.
        unsafe {
            fixup_opts_string(
                uc,
                uc.opts_view().scale_helper_name_mut_ptr() as *mut String,
                true,
            )
        }
        .is_ok(),
        "ufbxi_fixup_opts_string(uc, &uc->opts.scale_helper_name, true)"
    );

    ufbxi_check!(
        uc,
        // SAFETY: every argument is one of `uc`'s own fields — thread pool,
        // error slot, temp allocator and `opts.thread_opts` — all live for the
        // `&Context` borrow.
        unsafe {
            thread_pool_init(
                uc.thread_pool_mut_ptr(),
                uc.error_mut_ptr(),
                uc.ator_tmp_mut_ptr(),
                uc.opts_view().thread_opts_ptr(),
            )
        }
        .is_ok(),
        "ufbxi_thread_pool_init(&uc->thread_pool, &uc->error, &uc->ator_tmp, &uc->opts.thread_opts)"
    );

    if !uc.opts_view().allow_unsafe() {
        ufbxi_check_msg!(
            uc,
            uc.opts_view().index_error_handling() != IndexErrorHandling::UnsafeIgnore,
            "Unsafe options",
            "uc->opts.index_error_handling != UFBX_INDEX_ERROR_HANDLING_UNSAFE_IGNORE"
        );
        ufbxi_check_msg!(
            uc,
            uc.opts_view().unicode_error_handling() != UnicodeErrorHandling::UnsafeIgnore,
            "Unsafe options",
            "uc->opts.unicode_error_handling != UFBX_UNICODE_ERROR_HANDLING_UNSAFE_IGNORE"
        );
    } else {
        uc.scene_view().metadata_view().set_is_unsafe(true);
    }

    if uc.opts_view().index_error_handling() == IndexErrorHandling::NoIndex {
        uc.scene_view()
            .metadata_view()
            .set_may_contain_no_index(true);
    }

    uc.set_retain_mesh_parts(
        !uc.opts_view().ignore_geometry() && !uc.opts_view().skip_mesh_parts(),
    );
    uc.scene_view()
        .metadata_view()
        .set_may_contain_missing_vertex_position(uc.opts_view().allow_missing_vertex_position());
    uc.scene_view()
        .metadata_view()
        .set_may_contain_broken_elements(uc.opts_view().connect_broken_elements());

    uc.scene_view()
        .metadata_view()
        .creator_view()
        .set_data(EMPTY_CHAR.as_ptr());

    uc.set_unit_scale(1.0);
    if uc.data().is_null() {
        ufbxi_dev_assert!(uc.data_begin().is_null());
        // C: `uc->data_begin = uc->data = ufbxi_zero_size_buffer;`
        uc.set_data(ZERO_SIZE_BUFFER.as_ptr());
        uc.set_data_begin(uc.data());
    }

    uc.set_retain_vertex_w(
        (uc.opts_view().retain_dom() || uc.opts_view().retain_vertex_attrib_w())
            && !uc.opts_view().ignore_geometry(),
    );

    ufbxi_check!(uc, load_strings(uc).is_ok(), "ufbxi_load_strings(uc)");
    ufbxi_check!(uc, load_maps(uc).is_ok(), "ufbxi_load_maps(uc)");
    ufbxi_check!(
        uc,
        determine_format(uc).is_ok(),
        "ufbxi_determine_format(uc)"
    );

    let format: FileFormat = uc.scene_view().metadata_view().file_format();

    if format == FileFormat::Fbx {
        ufbxi_check!(uc, begin_parse(uc).is_ok(), "ufbxi_begin_parse(uc)");
        if uc.version() < 6000 {
            ufbxi_check!(
                uc,
                read_legacy_root(uc).is_ok(),
                "ufbxi_read_legacy_root(uc)"
            );
        } else {
            ufbxi_check!(uc, read_root(uc).is_ok(), "ufbxi_read_root(uc)");
        }
        if !supports_version(uc.version()) {
            ufbxi_check!(
                uc,
                ufbxi_warnf!(
                    uc,
                    WarningType::UnsupportedVersion,
                    "Unsupported FBX version (%u)",
                    uc.version()
                )
                .is_ok(),
                "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_UNSUPPORTED_VERSION, ~0u, \"Unsupported FBX version (%u)\", uc->version)"
            );
        }
        update_scene_metadata(uc.scene_view().metadata_view());
        ufbxi_check!(uc, init_file_paths(uc).is_ok(), "ufbxi_init_file_paths(uc)");
    } else if format == FileFormat::Obj {
        ufbxi_check!(uc, obj_load(uc).is_ok(), "ufbxi_obj_load(uc)");
        update_scene_metadata(uc.scene_view().metadata_view());
    } else if format == FileFormat::Mtl {
        ufbxi_check!(uc, mtl_load(uc).is_ok(), "ufbxi_mtl_load(uc)");
        update_scene_metadata(uc.scene_view().metadata_view());
    }

    // Fake DOM root if necessary
    if uc.opts_view().retain_dom() && uc.scene_view().dom_root().is_none() {
        let dom_root: *mut DomNode = uc.result_view().push_zero::<DomNode>(1);
        ufbxi_check!(uc, !dom_root.is_null(), "dom_root");
        // SAFETY: `dom_root` is the non-null (checked just above) zeroed
        // `ufbx_dom_node` pushed into `uc`'s result buffer.
        unsafe { (*dom_root).name.data = EMPTY_CHAR.as_ptr() };
        // SAFETY: `dom_root` is that same non-null result-buffer node, which
        // lives as long as the scene the `Ref` is stored into.
        uc.scene_view()
            .set_dom_root(Some(unsafe { Ref::from_ptr(dom_root) }));
    }

    ufbxi_check!(
        uc,
        pre_finalize_scene(uc).is_ok(),
        "ufbxi_pre_finalize_scene(uc)"
    );

    // We can free `tmp_parse` already here as all parsing is done by now.
    // SAFETY: `tmp_parse` is `uc`'s own buffer field, live for the `&Context`
    // borrow.
    unsafe { buf_free(uc.tmp_parse_mut_ptr()) };

    // SAFETY: `finalize_scene` takes the same `&Context` this fn was handed.
    ufbxi_check!(
        uc,
        unsafe { finalize_scene(uc) }.is_ok(),
        "ufbxi_finalize_scene(uc)"
    );

    update_scene_settings(uc.scene_view().settings_view());
    if uc.scene_view().metadata_view().file_format() == FileFormat::Obj {
        update_scene_settings_obj(uc);
    }

    // Axis conversion
    if coordinate_axes_valid(uc.opts_view().target_axes()) {
        transform_to_axes(uc, uc.opts_view().target_axes());
    }

    // Unit conversion
    if uc.opts_view().target_unit_meters() > 0.0 {
        ufbxi_check!(
            uc,
            scale_units(uc, uc.opts_view().target_unit_meters()).is_ok(),
            "ufbxi_scale_units(uc, uc->opts.target_unit_meters)"
        );
    }

    // TODO: This could be done in evaluate as well with refactoring
    update_adjust_transforms(uc, uc.scene_view());

    ufbxi_check!(uc, modify_geometry(uc).is_ok(), "ufbxi_modify_geometry(uc)");
    postprocess_scene(uc);

    // SAFETY: the scene is `uc`'s own field (live for the borrow) and the
    // element run is passed as the empty `(NULL, 0)` pair, which
    // `update_scene` accepts.
    unsafe { update_scene(uc.scene_view(), true, ptr::null(), 0) };

    // Force a non-NULL anim pointer
    // SAFETY: `anim_ptr()` addresses `uc`'s own `scene.anim` field, live for the
    // `&Context` borrow.
    if unsafe { ref_ptr(uc.scene_view().anim_ptr()) }.is_null() {
        // C: `uc->scene.anim = ufbxi_push_zero(&uc->result, ufbx_anim, 1);`
        // (NOT `ufbxi_check`ed in C — a failed allocation leaves it NULL).
        // SAFETY: as above, writing the same live `scene.anim` field.
        unsafe {
            *(uc.scene_view().anim_mut_ptr() as *mut *mut Anim) =
                uc.result_view().push_zero::<Anim>(1)
        };
    }

    if uc.opts_view().load_external_files() {
        ufbxi_check!(
            uc,
            load_external_files(uc).is_ok(),
            "ufbxi_load_external_files(uc)"
        );
    }

    // Evaluate skinning if requested
    if uc.opts_view().evaluate_skinning() {
        // C: `ufbx_geometry_cache_data_opts cache_opts = { 0 };`
        // SAFETY: `ufbx_geometry_cache_data_opts` is C plain data — POD fields
        // plus nullable callback/user pointers — so all-zero is a valid
        // inhabitant.
        let mut cache_opts: RawGeometryCacheDataOpts =
            unsafe { MaybeUninit::zeroed().assume_init() };
        // SAFETY: the source is `uc`'s own `opts.open_file_cb` field, live and
        // initialized for the `&Context` borrow; `ufbx_open_file_cb` is `Copy`,
        // so the read does not duplicate ownership.
        cache_opts.open_file_cb = unsafe { ptr::read(uc.opts_view().open_file_cb_ptr()) };
        // SAFETY: the scene, error slot, result and tmp buffers are all `uc`'s
        // own fields (live for the borrow) and `cache_opts` is the live local
        // just initialized above.
        ufbxi_check!(
            uc,
            unsafe { evaluate_skinning(
                uc.scene_mut_ptr(),
                uc.error_mut_ptr(),
                uc.result_view(),
                uc.tmp_view(),
                0.0,
                uc.opts_view().load_external_files() && uc.opts_view().evaluate_caches(),
                &mut cache_opts,
            ) }
            .is_ok(),
            "ufbxi_evaluate_skinning(&uc->scene, &uc->error, &uc->result, &uc->tmp, 0.0, uc->opts.load_external_files && uc->opts.evaluate_caches, &cache_opts)"
        );
    }

    // Pop warnings to metadata
    ufbxi_check!(
        uc,
        // SAFETY: the warning list and the scene metadata's warning run and
        // `has_warning` array are all `uc`'s own fields, live for the borrow.
        unsafe {
            pop_warnings(
                uc.warnings_mut_ptr(),
                uc.scene_view().metadata_view().warnings_mut_ptr(),
                uc.scene_view().metadata_view().has_warning_mut_ptr(),
            )
        }
        .is_ok(),
        "ufbxi_pop_warnings(&uc->warnings, &uc->scene.metadata.warnings, uc->scene.metadata.has_warning)"
    );
    ufbxi_check!(
        uc,
        resolve_warning_elements(uc).is_ok(),
        "ufbxi_resolve_warning_elements(uc)"
    );

    // Copy local data to the scene
    uc.scene_view().metadata_view().set_version(uc.version());
    uc.scene_view().metadata_view().set_ascii(uc.from_ascii());
    uc.scene_view()
        .metadata_view()
        .set_big_endian(uc.file_big_endian());
    uc.scene_view()
        .metadata_view()
        .set_geometry_ignored(uc.opts_view().ignore_geometry());
    uc.scene_view()
        .metadata_view()
        .set_animation_ignored(uc.opts_view().ignore_animation());
    uc.scene_view()
        .metadata_view()
        .set_embedded_ignored(uc.opts_view().ignore_embedded());

    // Retain the scene, this must be the final allocation as we copy
    // `ator_result` to `ufbx_scene_imp`.
    let imp: *mut SceneImp = uc.result_view().push::<SceneImp>(1);
    ufbxi_check!(uc, !imp.is_null(), "imp");

    // Expose the wide allocation so `get_imp` can recover this header from a
    // (possibly narrowed) public `&Scene` pointer via exposed provenance.
    (imp as *mut u8).expose_provenance();

    // SAFETY (this fn's remaining `*imp` accesses): `imp` is the non-null
    // (checked just above) `ufbxi_scene_imp` pushed into `uc`'s result buffer,
    // so every projection below addresses that live allocation's own fields.
    unsafe { init_ref(&mut (*imp).refcount, SCENE_IMP_MAGIC, ptr::null_mut()) };

    unsafe { (*imp).magic = SCENE_IMP_MAGIC };
    // C: `imp->scene = uc->scene;` (struct copy)
    // SAFETY: the source is `uc`'s own scene field (live for the borrow) and the
    // destination is `imp`'s own scene field; the context and the freshly pushed
    // header are distinct allocations.
    unsafe { ptr::copy_nonoverlapping(uc.scene_mut_ptr(), &raw mut (*imp).scene, 1) };
    unsafe { (*imp).refcount.ator = uc.ator_result() };
    unsafe { (*imp).refcount.ator.error = ptr::null_mut() };

    // Copy retained buffers and translate the allocator struct to the one
    // contained within `ufbxi_scene_imp`
    unsafe { (*imp).refcount.buf = uc.take_result() };
    unsafe { (*imp).refcount.buf.ator = &raw mut (*imp).refcount.ator };
    unsafe { (*imp).string_buf = uc.string_pool_view().take_buf() };
    unsafe { (*imp).string_buf.ator = &raw mut (*imp).refcount.ator };

    unsafe { (*imp).scene.metadata.result_memory_used = (*imp).refcount.ator.current_size };
    // SAFETY: as above for `imp`; `ator_tmp` is `uc`'s own allocator field, live
    // for the `&Context` borrow.
    unsafe { (*imp).scene.metadata.temp_memory_used = (*uc.ator_tmp_mut_ptr()).current_size };
    unsafe { (*imp).scene.metadata.result_allocs = (*imp).refcount.ator.num_allocs };
    // SAFETY: as the `temp_memory_used` write above.
    unsafe { (*imp).scene.metadata.temp_allocs = (*uc.ator_tmp_mut_ptr()).num_allocs };

    // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, imp->scene.elements)`
    let mut p_elem: *mut *mut Element = unsafe { (*imp).scene.elements.data as *mut *mut Element };
    let p_elem_end: *mut *mut Element = add_ptr(p_elem, unsafe { (*imp).scene.elements.count });
    while p_elem != p_elem_end {
        // C: `(*p_elem)->scene = &imp->scene;`
        // SAFETY: `p_elem` walks the scene's element pointer list and stops at
        // `p_elem_end`, so it addresses a live slot holding a result-buffer
        // `ufbx_element` whose own `scene` back-pointer is written here.
        unsafe { *(&raw mut (**p_elem).scene as *mut *mut Scene) = &raw mut (*imp).scene };
        // SAFETY: `p_elem` is inside the list, so `p_elem + 1` is at most one
        // past its end.
        p_elem = unsafe { p_elem.add(1) };
    }

    uc.set_scene_imp(imp);

    Ok(())
}

// ufbx.c:25412-25462 `ufbxi_free_temp`
#[inline(never)]
pub(crate) fn free_temp(uc: &Context) {
    // SAFETY: every buffer, map, allocator and growth-state pair torn down
    // here is uc's own temp-side state, reached through uc's accessors and
    // valid by construction; this is the last use of all of it (mirrors C
    // `ufbxi_free_temp`).
    unsafe {
        thread_pool_free(uc.thread_pool_mut_ptr());

        string_pool_temp_free(uc.string_pool_mut_ptr());
        buf_free(uc.warnings_view().tmp_stack_mut_ptr());

        map_free(uc.prop_type_map_mut_ptr());
        map_free(uc.fbx_id_map_mut_ptr());
        map_free(uc.ptr_fbx_id_map_mut_ptr());
        map_free(uc.texture_file_map_mut_ptr());
        map_free(uc.anim_stack_map_mut_ptr());
        map_free(uc.fbx_attr_map_mut_ptr());
        map_free(uc.node_prop_set_mut_ptr());
        map_free(uc.dom_node_map_mut_ptr());

        buf_free(uc.tmp_mut_ptr());
        buf_free(uc.tmp_parse_mut_ptr());
        for i in 0..THREAD_GROUP_COUNT {
            buf_free(uc.tmp_thread_parse_mut_ptr(i));
        }
        buf_free(uc.tmp_stack_mut_ptr());
        buf_free(uc.tmp_connections_mut_ptr());
        buf_free(uc.tmp_node_ids_mut_ptr());
        buf_free(uc.tmp_elements_mut_ptr());
        buf_free(uc.tmp_element_offsets_mut_ptr());
        buf_free(uc.tmp_element_fbx_ids_mut_ptr());
        buf_free(uc.tmp_element_ptrs_mut_ptr());
        for i in 0..ELEMENT_TYPE_COUNT {
            buf_free(uc.tmp_typed_element_offsets_mut_ptr(i));
        }
        buf_free(uc.tmp_mesh_textures_mut_ptr());
        buf_free(uc.tmp_full_weights_mut_ptr());
        buf_free(uc.tmp_dom_nodes_mut_ptr());
        buf_free(uc.tmp_element_id_mut_ptr());
        buf_free(uc.tmp_ascii_spans_mut_ptr());

        free::<Node>(uc.ator_tmp_mut_ptr(), uc.top_nodes(), uc.top_nodes_cap());
        free::<*mut c_void>(
            uc.ator_tmp_mut_ptr(),
            uc.element_extra_arr(),
            uc.element_extra_cap(),
        );

        free::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.ascii_view().token_view().str_data(),
            uc.ascii_view().token_view().str_cap(),
        );
        free::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.ascii_view().prev_token_view().str_data(),
            uc.ascii_view().prev_token_view().str_cap(),
        );

        free::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.read_buffer(),
            uc.read_buffer_size(),
        );
        free::<u8>(uc.ator_tmp_mut_ptr(), uc.tmp_arr(), uc.tmp_arr_size());
        free::<u8>(uc.ator_tmp_mut_ptr(), uc.swap_arr(), uc.swap_arr_size());

        obj_free(uc);

        free_ator(uc.ator_tmp_mut_ptr());
    }
}

// ufbx.c:25464-25470 `ufbxi_free_result`
#[inline(never)]
pub(crate) fn free_result(uc: &Context) {
    // SAFETY: the buffer/allocator pointers come from `uc` accessors and are
    // valid by construction; mirrors C `ufbxi_free_result`'s teardown.
    unsafe {
        buf_free(uc.result_mut_ptr());
        buf_free(uc.string_pool_view().buf_mut_ptr());

        free_ator(uc.ator_result_mut_ptr());
    }
}

// ufbx.c:25472-25625 `ufbxi_load`
#[inline(never)]
pub(crate) unsafe fn load(
    uc: &Context,
    user_opts: *const RawLoadOpts,
    p_error: *mut Error,
) -> *mut Scene {
    // Test endianness
    {
        // C: `uint8_t buf[2]; uint16_t val = 0xbbaa; memcpy(buf, &val, 2);`
        let val: u16 = 0xbbaa;
        let buf: [u8; 2] = val.to_ne_bytes();
        uc.set_local_big_endian(buf[0] == 0xbb);
    }

    uc.set_double_parse_flags(parse_double_init_flags());

    if !user_opts.is_null() {
        // C: `uc->opts = *user_opts;` (struct copy)
        // SAFETY: `user_opts` is non-null (checked above) and points to the
        // caller's initialized `ufbx_load_opts` — the contract of this
        // `unsafe fn`; the destination is `uc`'s own opts field, live for the
        // `&Context` borrow and distinct from the caller's struct.
        unsafe { ptr::copy_nonoverlapping(user_opts, uc.opts_mut_ptr(), 1) };
    } else {
        // C: `memset(&uc->opts, 0, sizeof(uc->opts));`
        // SAFETY: `uc.opts_mut_ptr()` addresses `uc`'s own `ufbx_load_opts`
        // field, so exactly `size_of::<RawLoadOpts>()` bytes are writable there.
        unsafe { ptr::write_bytes(uc.opts_mut_ptr() as *mut u8, 0, size_of::<RawLoadOpts>()) };
    }

    if uc.opts_view().file_size_estimate() != 0 {
        uc.set_progress_bytes_total(uc.opts_view().file_size_estimate());
    }

    if uc.opts_view().ignore_all_content() {
        uc.opts_view().set_ignore_geometry(true);
        uc.opts_view().set_ignore_animation(true);
        uc.opts_view().set_ignore_embedded(true);
    }

    // C: `ufbx_inflate_retain inflate_retain; inflate_retain.initialized = false;`
    // — only `initialized` is written before use.
    let mut inflate_retain = MaybeUninit::<InflateRetain>::uninit();
    // SAFETY: the projection addresses the `initialized` field of the live local
    // `MaybeUninit<InflateRetain>` storage, and the write initializes that field
    // without reading any of the still-uninitialized bytes around it.
    unsafe { (&raw mut (*inflate_retain.as_mut_ptr()).initialized).write(false) };

    // SAFETY: the error slot, temp allocator and `opts.temp_allocator` are all
    // `uc`'s own fields, live for the `&Context` borrow.
    unsafe {
        init_ator(
            uc.error_mut_ptr(),
            uc.ator_tmp_mut_ptr(),
            uc.opts_view().temp_allocator_ptr(),
            c"temp",
        )
    };
    // SAFETY: as above, for `uc`'s result allocator and `opts.result_allocator`.
    unsafe {
        init_ator(
            uc.error_mut_ptr(),
            uc.ator_result_mut_ptr(),
            uc.opts_view().result_allocator_ptr(),
            c"result",
        )
    };

    if uc.opts_view().read_buffer_size() == 0 {
        uc.opts_view().set_read_buffer_size(0x4000);
    }
    if uc.opts_view().read_buffer_size() <= 32 {
        uc.opts_view().set_read_buffer_size(32);
    }

    if uc.opts_view().file_format_lookahead() == 0 {
        uc.opts_view().set_file_format_lookahead(0x4000);
    } else if uc.opts_view().file_format_lookahead() < MIN_FILE_FORMAT_LOOKAHEAD {
        uc.opts_view()
            .set_file_format_lookahead(MIN_FILE_FORMAT_LOOKAHEAD);
    }

    if uc.opts_view().path_separator() == 0 {
        uc.opts_view().set_path_separator(PATH_SEPARATOR);
    }

    if uc.opts_view().progress_cb().fn_.is_none()
        || uc.opts_view().progress_interval_hint() >= usize::MAX as u64
    {
        uc.set_progress_interval(usize::MAX);
    } else if uc.opts_view().progress_interval_hint() > 0 {
        uc.set_progress_interval(uc.opts_view().progress_interval_hint() as usize);
    } else {
        uc.set_progress_interval(0x4000);
    }

    if uc.opts_view().open_file_cb_view().fn_().is_none() {
        // C: `uc->opts.open_file_cb.fn = &ufbx_default_open_file;`
        uc.opts_view()
            .open_file_cb_view()
            .set_fn_(Some(default_open_file));
    }

    if uc.opts_view().thread_opts_view().memory_limit() == 0 {
        uc.opts_view()
            .thread_opts_view()
            .set_memory_limit(32 * 1024 * 1024);
    }

    uc.set_synthetic_id_counter(SYNTHETIC_ID_START);

    uc.string_pool_view().set_error(uc.error_mut_ptr());
    // SAFETY: the string pool's map and `ator_tmp` are `uc`'s own fields, live
    // for the `&Context` borrow; the comparator is a plain fn item taking the
    // null user pointer `map_init` stores alongside it.
    unsafe {
        map_init(
            uc.string_pool_view().map_mut_ptr(),
            uc.ator_tmp_mut_ptr(),
            map_cmp_string,
            ptr::null_mut(),
        )
    };
    uc.string_pool_view()
        .buf_view()
        .set_ator(uc.ator_result_mut_ptr());
    uc.string_pool_view().buf_view().set_unordered(true);
    uc.string_pool_view().set_initial_size(1024);
    uc.string_pool_view()
        .set_error_handling(uc.opts_view().unicode_error_handling());

    // SAFETY (every `map_init` below): the map and `ator_tmp` are `uc`'s own
    // fields, live for the `&Context` borrow; each comparator is a plain fn item
    // taking the null user pointer `map_init` stores alongside it.
    unsafe {
        map_init(
            uc.prop_type_map_mut_ptr(),
            uc.ator_tmp_mut_ptr(),
            map_cmp_const_char_ptr,
            ptr::null_mut(),
        )
    };
    unsafe {
        map_init(
            uc.fbx_id_map_mut_ptr(),
            uc.ator_tmp_mut_ptr(),
            map_cmp_uint64,
            ptr::null_mut(),
        )
    };
    unsafe {
        map_init(
            uc.ptr_fbx_id_map_mut_ptr(),
            uc.ator_tmp_mut_ptr(),
            map_cmp_ptr_id,
            ptr::null_mut(),
        )
    };
    unsafe {
        map_init(
            uc.texture_file_map_mut_ptr(),
            uc.ator_tmp_mut_ptr(),
            map_cmp_const_char_ptr,
            ptr::null_mut(),
        )
    };
    unsafe {
        map_init(
            uc.anim_stack_map_mut_ptr(),
            uc.ator_tmp_mut_ptr(),
            map_cmp_const_char_ptr,
            ptr::null_mut(),
        )
    };
    unsafe {
        map_init(
            uc.fbx_attr_map_mut_ptr(),
            uc.ator_tmp_mut_ptr(),
            map_cmp_uint64,
            ptr::null_mut(),
        )
    };
    unsafe {
        map_init(
            uc.node_prop_set_mut_ptr(),
            uc.ator_tmp_mut_ptr(),
            map_cmp_const_char_ptr,
            ptr::null_mut(),
        )
    };
    unsafe {
        map_init(
            uc.dom_node_map_mut_ptr(),
            uc.ator_tmp_mut_ptr(),
            map_cmp_uintptr,
            ptr::null_mut(),
        )
    };

    uc.tmp_view().set_ator(uc.ator_tmp_mut_ptr());
    uc.tmp_parse_view().set_ator(uc.ator_tmp_mut_ptr());
    uc.tmp_stack_view().set_ator(uc.ator_tmp_mut_ptr());
    uc.tmp_connections_view().set_ator(uc.ator_tmp_mut_ptr());
    uc.tmp_node_ids_view().set_ator(uc.ator_tmp_mut_ptr());
    uc.tmp_elements_view().set_ator(uc.ator_tmp_mut_ptr());
    uc.tmp_element_offsets_view()
        .set_ator(uc.ator_tmp_mut_ptr());
    uc.tmp_element_fbx_ids_view()
        .set_ator(uc.ator_tmp_mut_ptr());
    uc.tmp_element_ptrs_view().set_ator(uc.ator_tmp_mut_ptr());
    for i in 0..ELEMENT_TYPE_COUNT {
        uc.tmp_typed_element_offsets_at(i)
            .set_ator(uc.ator_tmp_mut_ptr());
    }
    uc.tmp_mesh_textures_view().set_ator(uc.ator_tmp_mut_ptr());
    uc.tmp_full_weights_view().set_ator(uc.ator_tmp_mut_ptr());
    uc.tmp_dom_nodes_view().set_ator(uc.ator_tmp_mut_ptr());
    uc.tmp_element_id_view().set_ator(uc.ator_tmp_mut_ptr());
    uc.tmp_ascii_spans_view().set_ator(uc.ator_tmp_mut_ptr());

    for i in 0..THREAD_GROUP_COUNT {
        uc.tmp_thread_parse_at(i).set_ator(uc.ator_tmp_mut_ptr());
        uc.tmp_thread_parse_at(i).set_unordered(true);
        uc.tmp_thread_parse_at(i).set_clearable(true);
    }

    uc.result_view().set_ator(uc.ator_result_mut_ptr());

    uc.tmp_view().set_unordered(true);
    uc.tmp_parse_view().set_unordered(true);
    uc.tmp_parse_view().set_clearable(true);
    uc.result_view().set_unordered(true);

    uc.warnings_view().set_error(uc.error_mut_ptr());
    uc.warnings_view().set_result(uc.result_mut_ptr());
    uc.warnings_view()
        .tmp_stack_view()
        .set_ator(uc.ator_tmp_mut_ptr());
    uc.string_pool_view().set_warnings(uc.warnings_mut_ptr());

    // Set zero size `swap_arr` to a non-NULL buffer so we can tell the difference between empty
    // array and an allocation failure.
    // C: `uc->swap_arr = (char*)ufbxi_zero_size_buffer;` — the const cast is
    // C-parity: the buffer is replaced by `ufbxi_grow_array` before any write.
    uc.set_swap_arr(ZERO_SIZE_BUFFER.as_ptr() as *mut u8);

    // NOTE: Though `inflate_retain` leaks out of the scope we don't use it outside this function.
    // cppcheck-suppress autoVariables
    uc.set_inflate_retain(inflate_retain.as_mut_ptr());

    // SAFETY: `load_imp` takes the same `&Context` this fn was handed, now fully
    // initialized by the setup above.
    let ok: bool = unsafe { load_imp(uc) }.is_ok();

    if uc.close_fn().is_some() {
        // SAFETY: `close_fn` is `Some` (checked just above) and is called with
        // its paired `read_user`, per the C stream-callback contract.
        unsafe { (uc.close_fn().unwrap())(uc.read_user()) };
    }

    free_temp(uc);

    if ok {
        if !p_error.is_null() {
            // SAFETY: `p_error` is non-null (checked above) and points to the
            // caller's `ufbx_error` slot — the contract of this `unsafe fn`.
            unsafe { clear_error(p_error) };
        }
        // SAFETY: `ok` means `load_imp` succeeded, and its last act is storing
        // the retained `ufbxi_scene_imp` into `uc`, so `scene_imp()` is the live
        // result-buffer header whose own `scene` field is projected here.
        unsafe { &raw mut (*uc.scene_imp()).scene }
    } else {
        // SAFETY: `uc`'s error field is live for the borrow and `p_error` is the
        // caller's `ufbx_error` slot or null, which `fix_error_type` accepts.
        unsafe { fix_error_type(uc.error_mut_ptr(), b"Failed to load\0".as_ptr(), p_error) };
        // SAFETY (this condition and the block it guards): `p_error` is non-null
        // (checked first, and `&&` short-circuits) and points to the caller's
        // `ufbx_error` slot, which `fix_error_type` just wrote.
        if !p_error.is_null()
            && unsafe { (*p_error).type_ } == ErrorType::Unknown
            && uc.scene_view().metadata_view().file_format() == FileFormat::Fbx
            && !supports_version(uc.version())
        {
            unsafe { (*p_error).description.data = b"Unsupported version\0".as_ptr() };
            // SAFETY: as above; the string literal is NUL-terminated, so `strlen`
            // reads within it.
            unsafe { (*p_error).description.length = strlen(b"Unsupported version\0".as_ptr()) };
            unsafe { (*p_error).type_ = ErrorType::UnsupportedVersion };
            // SAFETY: as above — the macro formats into the same live slot.
            unsafe { ufbxi_fmt_err_info!(p_error, "%u", uc.version()) };
        }
        free_result(uc);
        ptr::null_mut()
    }
}

// -- Animation evaluation (ufbx.c:25627)

// ufbx.c:25629-25634 `ufbxi_override_less_than_prop`
// C: `ufbxi_forceinline`.
#[inline(always)]
pub(crate) unsafe fn override_less_than_prop(
    over: *const PropOverride,
    element_id: u32,
    prop: *const Prop,
) -> bool {
    // SAFETY (every access in this fn): `over` and `prop` are the caller's live
    // `ufbx_prop_override` / `ufbx_prop` — the raw-pointer contract of this
    // `unsafe fn`.
    if unsafe { (*over).element_id } != element_id {
        return unsafe { (*over).element_id } < element_id;
    }
    if unsafe { (*over)._internal_key } != unsafe { (*prop)._internal_key } {
        return unsafe { (*over)._internal_key } < unsafe { (*prop)._internal_key };
    }
    // C: `return strcmp(over->prop_name.data, prop->name.data);` — the `int`
    // result converts to `bool` (nonzero == true), so ANY name difference
    // reports "less".
    // PORT DIVERGENCE (ufbx.c:25633): upstream `strcmp` scans `prop.name` to a
    // NUL, but on the NOT_FOUND path of `evaluate_prop_flags_len` it is the
    // caller's raw `_len` name and need not be NUL-terminated (over-read on an
    // element_id + `_internal_key` collision). `str_cmp` reads only `min(len)`
    // bytes with the same ordering for NUL-terminated names; reconcile on sync.
    // SAFETY: `over.prop_name` and `prop.name` are valid `String` runs of their
    // `.length` bytes; `str_cmp` bounds both reads by those lengths.
    unsafe { sp::str_cmp((*over).prop_name, (*prop).name) != 0 }
}

// ufbx.c:25636-25641 `ufbxi_override_equals_to_prop`
// C: `ufbxi_forceinline`.
#[inline(always)]
pub(crate) unsafe fn override_equals_to_prop(
    over: *const PropOverride,
    element_id: u32,
    prop: *const Prop,
) -> bool {
    // SAFETY (every access in this fn): `over` and `prop` are the caller's live
    // `ufbx_prop_override` / `ufbx_prop` — the raw-pointer contract of this
    // `unsafe fn`.
    if unsafe { (*over).element_id } != element_id {
        return false;
    }
    if unsafe { (*over)._internal_key } != unsafe { (*prop)._internal_key } {
        return false;
    }
    // PORT DIVERGENCE (ufbx.c:25640): as in `override_less_than_prop` — the
    // upstream `strcmp` over-reads a non-NUL `_len` `prop.name`; use the
    // length-bounded `str_cmp` instead; reconcile once upstream lands the fix.
    // SAFETY: `over.prop_name` and `prop.name` are valid `String` runs of their
    // `.length` bytes; `str_cmp` bounds both reads by those lengths.
    unsafe { sp::str_cmp((*over).prop_name, (*prop).name) == 0 }
}

// ufbx.c:25643-25664 `ufbxi_find_prop_override`
#[inline(never)]
pub(crate) unsafe fn find_prop_override(
    overrides: *const List<PropOverride>,
    element_id: u32,
    prop: *mut Prop,
) -> bool {
    let mut ix: usize = usize::MAX;
    // SAFETY: `overrides` is the caller's live override list — the raw-pointer
    // contract of this `unsafe fn` — so `data`/`count` describe its run, which is
    // what the search walks; the comparator closures are handed elements of that
    // run and the caller's live `prop`.
    unsafe {
        macro_lower_bound_eq::<PropOverride>(
            16,
            &mut ix,
            (*overrides).data,
            0,
            (*overrides).count,
            |a| override_less_than_prop(a, element_id, prop),
            |a| override_equals_to_prop(a, element_id, prop),
        )
    };

    if ix != usize::MAX {
        // SAFETY: a written `ix` is an index the search found inside the
        // override run, so `data + ix` addresses one of its elements.
        let over: *const PropOverride = unsafe { (*overrides).data.add(ix) };
        // C: `const uint32_t clear_flags = UFBX_PROP_FLAG_NO_VALUE | UFBX_PROP_FLAG_NOT_FOUND;`
        let clear_flags: u32 = PropFlags::NO_VALUE.raw() | PropFlags::NOT_FOUND.raw();
        // SAFETY (every access below): `prop` is the caller's live `ufbx_prop`
        // and `over` the matched override element of the caller's list.
        unsafe {
            (*prop).flags = PropFlags::from_raw(
                ((*prop).flags.raw() & !clear_flags) | PropFlags::OVERRIDDEN.raw(),
            )
        };
        unsafe { (*prop).value_vec4 = (*over).value };
        // C: `prop->value_real_arr[3] = 0.0f;` — the `ufbx_prop` value union's
        // `ufbx_real value_real_arr[4]` view; the generated struct keeps only
        // `value_vec4`.
        // SAFETY: as above; `value_vec4` is four `ufbx_real`s laid out as the C
        // union's `value_real_arr`, so index 3 is its last element.
        unsafe { *(&raw mut (*prop).value_vec4 as *mut Real).add(3) = 0.0 };
        unsafe { (*prop).value_int = (*over).value_int };
        unsafe { (*prop).value_str = (*over).value_str };
        unsafe { (*prop).value_blob.data = (*prop).value_str.data };
        unsafe { (*prop).value_blob.size = (*prop).value_str.length };
        true
    } else {
        false
    }
}

// ufbx.c:25666-25679 `ufbxi_find_element_prop_overrides`
#[inline(never)]
pub(crate) unsafe fn find_element_prop_overrides(
    overrides: *const List<PropOverride>,
    element_id: u32,
) -> List<PropOverride> {
    // C: `size_t begin = overrides->count, end = begin;` — pre-initialized
    // because `ufbxi_macro_lower_bound_eq` does NOT write on a miss.
    // SAFETY: `overrides` is the caller's live override list — the raw-pointer
    // contract of this `unsafe fn`.
    let mut begin: usize = unsafe { (*overrides).count };
    let mut end: usize = begin;

    // SAFETY: as above, `data`/`count` describe the caller's override run, which
    // is what the search walks; each comparator dereferences an element of that
    // run.
    unsafe {
        macro_lower_bound_eq::<PropOverride>(
            32,
            &mut begin,
            (*overrides).data,
            0,
            (*overrides).count,
            |a| (*a).element_id < element_id,
            |a| (*a).element_id == element_id,
        )
    };

    // SAFETY: as above; `begin` is an index the search left inside the run (or
    // its `count` end), so the `[begin, count)` window it scans stays in bounds.
    unsafe {
        macro_upper_bound_eq::<PropOverride>(
            32,
            &mut end,
            (*overrides).data,
            begin,
            (*overrides).count,
            |a| (*a).element_id == element_id,
        )
    };

    // C: `ufbx_prop_override_list result = { overrides->data + begin, end - begin };`
    // (`List<T>` carries a private `PhantomData` marker, so the aggregate
    // initializer becomes a zeroed value with both public fields written.)
    // SAFETY: `ufbx_prop_override_list` is a pointer plus a count (plus a
    // zero-sized marker), for which the all-zero pattern is a valid inhabitant.
    let mut result: List<PropOverride> = unsafe { MaybeUninit::zeroed().assume_init() };
    // SAFETY: `begin <= overrides->count`, so `data + begin` is at most one past
    // the end of the caller's override run.
    result.data = unsafe { (*overrides).data.add(begin) };
    result.count = end - begin;
    result
}

// ufbx.c:25681-25687 `ufbxi_anim_layer_combine_ctx`
#[repr(C)]
pub(crate) struct AnimLayerCombineCtx {
    pub anim: *const Anim,
    pub element: *const Element,
    pub time: f64,
    pub rotation_order: RotationOrder,
    pub has_rotation_order: bool,
}

// ufbx.c:25689-25695 `ufbxi_pow_abs`
#[inline(never)]
pub(crate) fn pow_abs(v: f64, e: f64) -> f64 {
    if e <= 0.0 {
        return 1.0;
    }
    if e >= 1.0 {
        return v;
    }
    let sign: f64 = if v < 0.0 { -1.0 } else { 1.0 };
    sign * math::pow(v * sign, e)
}

// Recursion is limited by the fact that we recurse only when the property name is "Lcl Rotation"
// and when recursing we always evaluate the property "RotationOrder"
// ufbx.c:25697-25750 `ufbxi_combine_anim_layer`
// `ufbxi_recursive_function_void(ufbxi_combine_anim_layer, ..., 2, ...)`
// (ufbx.c:25699-25700): under regression a thread-local depth guard wraps the
// recursive body; otherwise the macro is empty and the wrapper is a plain call.
#[inline(never)]
pub(crate) unsafe fn combine_anim_layer(
    ctx: *mut AnimLayerCombineCtx,
    layer: *mut AnimLayer,
    weight: Real,
    prop_name: *const u8,
    result: *mut Vec3,
    value: *const Vec3,
) {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|d| {
            ufbx_assert!(d.get() < 2);
            d.set(d.get() + 1);
        });
        // SAFETY: every pointer is forwarded unchanged from this fn's own
        // parameters, so the callee inherits the caller's contract.
        unsafe { combine_anim_layer_rec(ctx, layer, weight, prop_name, result, value) };
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
    }
    // SAFETY: every pointer is forwarded unchanged from this fn's own
    // parameters, so the callee inherits the caller's contract.
    #[cfg(not(feature = "regression"))]
    unsafe {
        combine_anim_layer_rec(ctx, layer, weight, prop_name, result, value)
    }
}

// ufbx.c:25702-25750 `ufbxi_combine_anim_layer` body (the `_rec` half of the
// `ufbxi_recursive_function` body; see the wrapper above)
#[inline(never)]
unsafe fn combine_anim_layer_rec(
    ctx: *mut AnimLayerCombineCtx,
    layer: *mut AnimLayer,
    weight: Real,
    prop_name: *const u8,
    result: *mut Vec3,
    value: *const Vec3,
) {
    // SAFETY (every `*ctx`, `*layer`, `*result` and `*value` access in this fn):
    // all four are the caller's live pointers — the raw-pointer contract of this
    // `unsafe fn`, threaded down from `ufbxi_combine_anim_layer`'s caller, which
    // passes an anim layer of the scene, a stack-local combine context and the
    // in/out `ufbx_vec3` slots it owns.
    if unsafe { (*layer).compose_rotation }
        && unsafe { (*layer).blended }
        && prop_name == sp::Lcl_Rotation.as_ptr()
        && !unsafe { (*ctx).has_rotation_order }
    {
        // SAFETY: `ctx`'s `anim`/`element` are the live scene objects its
        // constructor stored, and the name run is the interned `RotationOrder`
        // static — `evaluate_prop_len`'s contract.
        let rp: Prop = unsafe {
            evaluate_prop_len(
                (*ctx).anim,
                (*ctx).element,
                sp::RotationOrder.as_ptr(),
                sp::RotationOrder.len() - 1,
                (*ctx).time,
            )
        };
        // NOTE: Defaults to 0 (UFBX_ROTATION_XYZ) gracefully if property is not found
        if rp.value_int >= 0 && rp.value_int <= RotationOrder::Spheric as i64 {
            // C: `(ufbx_rotation_order)rp.value_int` — in-range by the guard.
            // SAFETY: the guard bounds `value_int` to `0..=Spheric`, which are
            // exactly the discriminants of the `repr(u32)` `ufbx_rotation_order`,
            // so the transmuted value is a valid variant.
            unsafe {
                (*ctx).rotation_order =
                    core::mem::transmute::<u32, RotationOrder>(rp.value_int as u32)
            };
        } else {
            unsafe { (*ctx).rotation_order = RotationOrder::Xyz };
        }
        unsafe { (*ctx).has_rotation_order = true };
    }

    if unsafe { (*layer).additive } {
        if unsafe { (*layer).compose_scale } && prop_name == sp::Lcl_Scaling.as_ptr() {
            // C: `result->x *= (ufbx_real)ufbxi_pow_abs(value->x, weight);`
            // — `ufbxi_pow_abs` takes `double`, so both args promote to double
            // and the result narrows back to `ufbx_real` before the multiply.
            unsafe { (*result).x *= pow_abs(as_f64!((*value).x), as_f64!(weight)) as Real };
            unsafe { (*result).y *= pow_abs(as_f64!((*value).y), as_f64!(weight)) as Real };
            unsafe { (*result).z *= pow_abs(as_f64!((*value).z), as_f64!(weight)) as Real };
        } else if unsafe { (*layer).compose_rotation } && prop_name == sp::Lcl_Rotation.as_ptr() {
            let a: Quat = unsafe { euler_to_quat(*result, (*ctx).rotation_order) };
            let mut b: Quat = unsafe { euler_to_quat(*value, (*ctx).rotation_order) };
            b = quat_slerp(IDENTITY_QUAT, b, weight);
            let res: Quat = mul_quat(a, b);
            unsafe { *result = quat_to_euler(res, (*ctx).rotation_order) };
        } else {
            unsafe { (*result).x += (*value).x * weight };
            unsafe { (*result).y += (*value).y * weight };
            unsafe { (*result).z += (*value).z * weight };
        }
    } else if unsafe { (*layer).blended } {
        // C: `ufbx_real res_weight = 1.0f - weight;`
        let res_weight: Real = 1.0 - weight;
        if unsafe { (*layer).compose_scale } && prop_name == sp::Lcl_Scaling.as_ptr() {
            // C: `result->x = (ufbx_real)(ufbxi_pow_abs(result->x, res_weight) * ufbxi_pow_abs(value->x, weight));`
            // — `ufbxi_pow_abs` takes `double`; the product stays in double and
            // narrows to `ufbx_real` only on the assignment.
            unsafe {
                (*result).x = (pow_abs(as_f64!((*result).x), res_weight as f64)
                    * pow_abs(as_f64!((*value).x), as_f64!(weight)))
                    as Real
            };
            unsafe {
                (*result).y = (pow_abs(as_f64!((*result).y), res_weight as f64)
                    * pow_abs(as_f64!((*value).y), as_f64!(weight)))
                    as Real
            };
            unsafe {
                (*result).z = (pow_abs(as_f64!((*result).z), res_weight as f64)
                    * pow_abs(as_f64!((*value).z), as_f64!(weight)))
                    as Real
            };
        } else if unsafe { (*layer).compose_rotation } && prop_name == sp::Lcl_Rotation.as_ptr() {
            let a: Quat = unsafe { euler_to_quat(*result, (*ctx).rotation_order) };
            let b: Quat = unsafe { euler_to_quat(*value, (*ctx).rotation_order) };
            let res: Quat = quat_slerp(a, b, weight);
            unsafe { *result = quat_to_euler(res, (*ctx).rotation_order) };
        } else {
            unsafe { (*result).x = (*result).x * res_weight + (*value).x * weight };
            unsafe { (*result).y = (*result).y * res_weight + (*value).y * weight };
            unsafe { (*result).z = (*result).z * res_weight + (*value).z * weight };
        }
    } else {
        unsafe { *result = *value };
    }
}

// ufbx.c:25751-25757 `ufbxi_anim_layer_might_contain_id`
// C: `ufbxi_forceinline`.
#[inline(always)]
pub(crate) unsafe fn anim_layer_might_contain_id(layer: *const AnimLayer, id: u32) -> bool {
    // C: `uint32_t id_mask = ufbxi_arraycount(layer->_element_id_bitmask) - 1;`
    // SAFETY (every `*layer` access in this fn): `layer` is the caller's live
    // `ufbx_anim_layer` — the raw-pointer contract of this `unsafe fn`.
    let id_mask: u32 = (unsafe { (*layer)._element_id_bitmask.len() } - 1) as u32;
    // C: `bool ok = id - layer->_min_element_id <= (layer->_max_element_id - layer->_min_element_id);`
    // — unsigned wrapping subtraction.
    let mut ok: bool = id.wrapping_sub(unsafe { (*layer)._min_element_id })
        <= unsafe { (*layer)._max_element_id }.wrapping_sub(unsafe { (*layer)._min_element_id });
    // SAFETY: as above; `id_mask` is the bitmask array's length minus one and
    // that length is a power of two, so `(id >> 5) & id_mask` indexes it.
    ok &= (unsafe { (*layer)._element_id_bitmask[((id >> 5) & id_mask) as usize] }
        & (1u32 << (id & 31)))
        != 0;
    ok
}

// ufbx.c:25759-25818 `ufbxi_evaluate_props`
#[inline(never)]
pub(crate) unsafe fn evaluate_props(
    anim: *const Anim,
    element: *const Element,
    time: f64,
    props: *mut Prop,
    num_props: usize,
    flags: u32,
) {
    // C: `ufbxi_anim_layer_combine_ctx combine_ctx = { anim, element, time };`
    let mut combine_ctx = AnimLayerCombineCtx {
        anim,
        element,
        time,
        rotation_order: RotationOrder::Xyz,
        has_rotation_order: false,
    };

    // SAFETY (every `*anim`, `*element` and `*layer` access in this fn): `anim`
    // and `element` are the caller's live scene objects and every layer comes out
    // of `anim`'s own layer list — the raw-pointer contract of this `unsafe fn`.
    let element_id: u32 = unsafe { (*element).element_id };
    let num_layers: usize = unsafe { (*anim).layers.count };
    for layer_ix in 0..num_layers {
        // SAFETY: `layer_ix < num_layers == anim->layers.count`, so the indexed
        // slot is inside the anim's layer list and holds a scene-owned layer.
        let layer: *mut AnimLayer =
            unsafe { *((*anim).layers.data as *mut *mut AnimLayer).add(layer_ix) };
        // SAFETY: `layer` is that scene-owned `ufbx_anim_layer`.
        if !unsafe { anim_layer_might_contain_id(layer, element_id) } {
            continue;
        }

        // Find the weight for the current layer
        // TODO: Should this be searched from multiple layers?
        let mut weight: Real = if layer_ix < unsafe { (*anim).override_layer_weights.count } {
            // SAFETY: the branch condition bounds `layer_ix` inside the anim's
            // override-weight run.
            unsafe { *(*anim).override_layer_weights.data.add(layer_ix) }
        } else {
            unsafe { (*layer).weight }
        };
        if unsafe { (*layer).weight_is_animated } && unsafe { (*layer).blended } {
            // SAFETY: `layer` is the scene-owned layer and the projection
            // addresses its own `element` header, which is the key
            // `find_anim_prop_start` searches its anim props for.
            let weight_aprop: *mut AnimProp =
                unsafe { find_anim_prop_start(layer, &raw const (*layer).element) };
            if !weight_aprop.is_null() {
                // C: `weight = ufbx_evaluate_anim_value_real_flags(...) / (ufbx_real)100.0;`
                // SAFETY: `weight_aprop` is a non-null (checked above) anim prop
                // of `layer`, so `anim_value` is its own field and holds a
                // scene-owned `ufbx_anim_value`.
                weight = unsafe {
                    evaluate_anim_value_real_flags(
                        ref_ptr(&(*weight_aprop).anim_value),
                        time,
                        flags,
                    )
                } / (100.0 as Real);
                // C: `if (weight < 0.0f) weight = 0.0f;`
                if weight < 0.0 {
                    weight = 0.0;
                }
                // C: `if (weight > 0.99999f) weight = 1.0f;` — the FLOAT
                // literal is compared in `ufbx_real`; when that is double the
                // conversion is (double)(float)0.99999.
                if weight > 0.99999f32 as Real {
                    weight = 1.0;
                }
            }
        }

        // SAFETY: `layer` is the scene-owned layer and `element` the caller's
        // live element, which is the key the search compares against.
        let mut aprop: *mut AnimProp = unsafe { find_anim_prop_start(layer, element) };
        if aprop.is_null() {
            continue;
        }

        for i in 0..num_props {
            // SAFETY: `i < num_props`, which is the length the caller declares
            // for the `props` run.
            let prop: *mut Prop = unsafe { props.add(i) };

            // Don't evaluate on top of overridden properties
            // SAFETY: `prop` is the element of the caller's prop run indexed
            // above.
            if (unsafe { (*prop).flags }.raw() & PropFlags::OVERRIDDEN.raw()) != 0 {
                continue;
            }

            // Connections override animation by default
            // SAFETY: as above for `prop`, and `anim` is the caller's live anim.
            if (unsafe { (*prop).flags }.raw() & PropFlags::CONNECTED.raw()) != 0
                && !unsafe { (*anim).ignore_connections }
            {
                continue;
            }

            // Skip until we reach `aprop >= prop`
            // NOTE: No need to check for end as `anim_props` is terminated with a NULL sentinel.
            // SAFETY (both loops below): `aprop` starts inside `layer`'s sorted
            // anim-prop run and only advances while its `element` still matches,
            // which the NULL-element sentinel terminating that run stops, so
            // every access stays inside the run.
            while std::ptr::eq(unsafe { ref_ptr(&(*aprop).element) }, element)
                && unsafe { (*aprop)._internal_key } < unsafe { (*prop)._internal_key }
            {
                aprop = unsafe { aprop.add(1) };
            }
            if unsafe { (*aprop).prop_name.data } != unsafe { (*prop).name.data } {
                while std::ptr::eq(unsafe { ref_ptr(&(*aprop).element) }, element)
                    // SAFETY: both names are string-pool `ufbx_string`s, which
                    // are stored NUL-terminated.
                    && unsafe { strcmp((*aprop).prop_name.data, (*prop).name.data) } < 0
                {
                    aprop = unsafe { aprop.add(1) };
                }
            }

            // TODO: Should we skip the blending for the first layer _per property_
            // This could be done by having `UFBX_PROP_FLAG_ANIMATION_EVALUATED`
            // that gets set for the first layer of animation that is applied.
            if unsafe { (*aprop).prop_name.data } == unsafe { (*prop).name.data } {
                // SAFETY: `aprop` is inside `layer`'s anim-prop run, so
                // `anim_value` is its own field and holds a scene-owned
                // `ufbx_anim_value`.
                let v: Vec3 = unsafe {
                    evaluate_anim_value_vec3_flags(ref_ptr(&(*aprop).anim_value), time, flags)
                };
                if layer_ix == 0 {
                    // C: `prop->value_vec3 = v;` — the `ufbx_prop` value
                    // union's 3-real view over `value_vec4`.
                    // SAFETY: `prop` is the caller's prop element; `value_vec4`
                    // is four `ufbx_real`s, so writing the union's leading
                    // `ufbx_vec3` view stays inside it.
                    unsafe { *(&raw mut (*prop).value_vec4 as *mut Vec3) = v };
                } else {
                    // SAFETY: the combine context is a live local of this frame,
                    // `layer` is the scene-owned layer, the name is `prop`'s own
                    // interned string, the in/out slot is `prop`'s own
                    // `value_vec4` union viewed as a `ufbx_vec3`, and `v` is a
                    // live local.
                    unsafe {
                        combine_anim_layer(
                            &mut combine_ctx,
                            layer,
                            weight,
                            (*prop).name.data,
                            &raw mut (*prop).value_vec4 as *mut Vec3,
                            &v,
                        )
                    };
                }
            }
        }
    }

    // C: `ufbxi_for(ufbx_prop, prop, props, num_props)`
    let mut prop: *mut Prop = props;
    let prop_end: *mut Prop = add_ptr(props, num_props);
    while prop != prop_end {
        // SAFETY (this loop): `prop` walks the caller's `num_props`-element prop
        // run and stops at `prop_end`, so it addresses one of its elements and
        // `prop + 1` is at most one past its end.
        if (unsafe { (*prop).flags }.raw() & PropFlags::OVERRIDDEN.raw()) != 0 {
            prop = unsafe { prop.add(1) };
            continue;
        }
        // C: `prop->value_int = ufbxi_f64_to_i64(prop->value_real);` — the
        // value union's first real is `value_vec4.x`; `ufbxi_f64_to_i64` takes
        // `double`, so the `ufbx_real` argument promotes.
        unsafe { (*prop).value_int = f64_to_i64(as_f64!((*prop).value_vec4.x)) };
        prop = unsafe { prop.add(1) };
    }
}

// Recursion limited by not calling `ufbx_evaluate_prop_len()` with a connected property,
// meaning it will never call `ufbxi_evaluate_connected_prop()` again indirectly.
// ufbx.c:25820-25845 `ufbxi_evaluate_connected_prop`
// `ufbxi_recursive_function_void(..., 3, ...)` (ufbx.c:25823-25824): see
// `combine_anim_layer` above for the guard shape.
#[inline(never)]
pub(crate) unsafe fn evaluate_connected_prop(
    prop: *mut Prop,
    anim: *const Anim,
    element: *const Element,
    name: *const u8,
    time: f64,
    flags: u32,
) {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|d| {
            ufbx_assert!(d.get() < 3);
            d.set(d.get() + 1);
        });
        // SAFETY: every pointer is forwarded unchanged from this fn's own
        // parameters, so the callee inherits the caller's contract.
        unsafe { evaluate_connected_prop_rec(prop, anim, element, name, time, flags) };
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
    }
    // SAFETY: every pointer is forwarded unchanged from this fn's own
    // parameters, so the callee inherits the caller's contract.
    #[cfg(not(feature = "regression"))]
    unsafe {
        evaluate_connected_prop_rec(prop, anim, element, name, time, flags)
    }
}

// ufbx.c:25826-25845 `ufbxi_evaluate_connected_prop` body (the `_rec` half of
// the `ufbxi_recursive_function` body; see the wrapper above)
#[inline(never)]
unsafe fn evaluate_connected_prop_rec(
    prop: *mut Prop,
    anim: *const Anim,
    element: *const Element,
    name: *const u8,
    time: f64,
    flags: u32,
) {
    // SAFETY: `element` is the caller's live element and `name` its prop-name run
    // — the raw-pointer contract of this `unsafe fn`.
    let mut conn: *mut Connection = unsafe { find_prop_connection(element, name) };

    // C: `for (size_t i = 0; i < 1000 && conn; i++)`
    let mut i: usize = 0;
    while i < 1000 && !conn.is_null() {
        // SAFETY: `conn` is non-null (loop condition) and points to a
        // scene-owned `ufbx_connection`, so `src` holds a scene-owned element
        // and `src_prop` is its own NUL-terminated string-pool name.
        let next_conn: *mut Connection =
            unsafe { find_prop_connection(ref_ptr(&(*conn).src), (*conn).src_prop.data) };
        if next_conn.is_null() {
            break;
        }
        conn = next_conn;
        i += 1;
    }

    // Found a non-cyclic connection
    // SAFETY: `conn` is non-null (checked first, and `&&` short-circuits) and
    // points to a scene-owned connection, so `src`/`src_prop` are its own fields.
    if !conn.is_null()
        && unsafe { find_prop_connection(ref_ptr(&(*conn).src), (*conn).src_prop.data) }.is_null()
    {
        // SAFETY: `anim` is the caller's live anim and `conn` the scene-owned
        // connection reached above, whose `src` element and `src_prop` name run
        // are what the evaluation reads.
        let ep: Prop = unsafe {
            evaluate_prop_flags_len(
                anim,
                ref_ptr(&(*conn).src),
                (*conn).src_prop.data,
                (*conn).src_prop.length,
                time,
                flags,
            )
        };
        // SAFETY (every write below): `prop` is the caller's live `ufbx_prop`.
        unsafe { (*prop).value_vec4 = ep.value_vec4 };
        unsafe { (*prop).value_int = ep.value_int };
        unsafe { (*prop).value_str = ep.value_str };
        unsafe { (*prop).value_blob = ep.value_blob };
    } else {
        // Connection not found, maybe it's animated?
        // SAFETY: `prop` is the caller's live `ufbx_prop`.
        unsafe {
            (*prop).flags = PropFlags::from_raw((*prop).flags.raw() & !PropFlags::CONNECTED.raw())
        };
    }
}

// ufbx.c:25847-25851 `ufbxi_prop_iter`
#[repr(C)]
pub(crate) struct PropIter {
    pub prop: *const Prop,
    pub prop_end: *const Prop,
    pub over: *const PropOverride,
    pub over_end: *const PropOverride,
    pub tmp: Prop,
}

// ufbx.c:25853-25864 `ufbxi_init_prop_iter_slow`
#[inline(never)]
pub(crate) unsafe fn init_prop_iter_slow(
    iter: *mut PropIter,
    anim: *const Anim,
    element: *const Element,
) {
    // SAFETY (every access in this fn): `iter` is the caller's live `PropIter`
    // storage and `anim`/`element` its live scene objects — the raw-pointer
    // contract of this `unsafe fn`.
    unsafe { (*iter).prop = (*element).props.props.data };
    // C: `iter->prop_end = element->props.props.data + element->props.props.count;`
    // SAFETY: `data`/`count` describe the element's own prop run, so the end
    // pointer is one past its last element.
    unsafe {
        (*iter).prop_end = (*element)
            .props
            .props
            .data
            .add((*element).props.props.count)
    };

    // SAFETY: the projection addresses `anim`'s own override list, which is what
    // the search walks.
    let over: List<PropOverride> = unsafe {
        find_element_prop_overrides(&raw const (*anim).prop_overrides, (*element).element_id)
    };
    unsafe { (*iter).over = over.data };
    // SAFETY: `over` is a sub-run of the anim's override list, so `data + count`
    // is one past its last element.
    unsafe { (*iter).over_end = over.data.add(over.count) };
    if over.count > 0 {
        // C: `memset(&iter->tmp, 0, sizeof(ufbx_prop));`
        // SAFETY: the projection addresses `iter`'s own `tmp` field, so exactly
        // `size_of::<Prop>()` bytes are writable there.
        unsafe { ptr::write_bytes(&raw mut (*iter).tmp as *mut u8, 0, size_of::<Prop>()) };
    }
}

// ufbx.c:25866-25874 `ufbxi_init_prop_iter`
// C: `ufbxi_forceinline`.
#[inline(always)]
pub(crate) unsafe fn init_prop_iter(
    iter: *mut PropIter,
    anim: *const Anim,
    element: *const Element,
) {
    // SAFETY (every access in this fn): `iter` is the caller's live `PropIter`
    // storage and `anim`/`element` its live scene objects — the raw-pointer
    // contract of this `unsafe fn`.
    unsafe { (*iter).prop = (*element).props.props.data };
    unsafe {
        (*iter).prop_end = add_ptr(
            (*element).props.props.data as *mut Prop,
            (*element).props.props.count,
        )
    };
    // C: `iter->over = iter->over_end = NULL;`
    unsafe { (*iter).over = ptr::null() };
    unsafe { (*iter).over_end = ptr::null() };
    if unsafe { (*anim).prop_overrides.count } > 0 {
        // SAFETY: the three pointers are forwarded unchanged from this fn's own
        // parameters, so the callee inherits the caller's contract.
        unsafe { init_prop_iter_slow(iter, anim, element) };
    }
}

// ufbx.c:25876-25914 `ufbxi_next_prop_slow`
#[inline(never)]
pub(crate) unsafe fn next_prop_slow(iter: *mut PropIter) -> *const Prop {
    // SAFETY (every `*iter` access in this fn): `iter` is the caller's live
    // `PropIter` — the raw-pointer contract of this `unsafe fn` — as initialized
    // by `init_prop_iter`, so its `prop`/`over` cursors sit inside the element's
    // prop run and the anim's override run and stop at the matching `*_end`.
    let prop: *const Prop = unsafe { (*iter).prop };
    let over: *const PropOverride = unsafe { (*iter).over };
    if prop == unsafe { (*iter).prop_end } && over == unsafe { (*iter).over_end } {
        return ptr::null();
    }

    // We can use `UINT32_MAX` as a terminating key (aka prefix) as prop names must
    // be valid UTF-8 and the byte sequence "\xff\xff\xff\xff" is not valid.
    let prop_key: u32 = if prop != unsafe { (*iter).prop_end } {
        // SAFETY: `prop` is short of `prop_end`, so it addresses an element of
        // the prop run.
        unsafe { (*prop)._internal_key }
    } else {
        u32::MAX
    };
    let over_key: u32 = if over != unsafe { (*iter).over_end } {
        // SAFETY: `over` is short of `over_end`, so it addresses an element of
        // the override run.
        unsafe { (*over)._internal_key }
    } else {
        u32::MAX
    };

    // C: `int cmp = 0;`
    let cmp: i32;
    if prop_key != over_key {
        cmp = if prop_key < over_key { -1 } else { 1 };
    } else {
        // SAFETY: equal keys means neither cursor took the `UINT32_MAX`
        // terminator above, so both address live elements of their runs;
        // `prop.name` is a string-pool string and `over.prop_name` was interned
        // NUL-terminated by `create_anim_imp` (STRINGS table or `push_anim_string`).
        cmp = unsafe { strcmp((*prop).name.data, (*over).prop_name.data) };
    }

    if cmp >= 0 {
        // SAFETY: the projection addresses `iter`'s own `tmp` prop.
        let dst: *mut Prop = unsafe { &raw mut (*iter).tmp };
        // SAFETY (every write below): `dst` is `iter`'s own `tmp` field and
        // `over` addresses an element of the override run — `cmp >= 0` is only
        // reachable when `over_key` was read from a live element, since a
        // `UINT32_MAX` `over_key` with a live `prop` compares greater.
        unsafe { (*dst).name = (*over).prop_name };
        unsafe { (*dst)._internal_key = (*over)._internal_key };
        unsafe { (*dst).type_ = PropType::Unknown };
        unsafe { (*dst).flags = PropFlags::OVERRIDDEN };
        unsafe { (*dst).value_str = (*over).value_str };
        unsafe { (*dst).value_blob.data = (*dst).value_str.data };
        unsafe { (*dst).value_blob.size = (*dst).value_str.length };
        unsafe { (*dst).value_int = (*over).value_int };
        unsafe { (*dst).value_vec4 = (*over).value };
        // SAFETY: `over` is inside the override run, so `over + 1` is at most
        // one past its end.
        unsafe { (*iter).over = over.add(1) };
        if cmp == 0 {
            // SAFETY: `cmp == 0` came from comparing live names, so `prop` is
            // inside the prop run and `prop + 1` is at most one past its end.
            unsafe { (*iter).prop = prop.add(1) };
        }
        dst
    } else {
        // SAFETY: `cmp < 0` requires a `prop_key` read from a live element, so
        // `prop` is inside the prop run and `prop + 1` is at most one past its
        // end.
        unsafe { (*iter).prop = prop.add(1) };
        prop
    }
}

// ufbx.c:25916-25924 `ufbxi_next_prop`
// C: `ufbxi_forceinline`.
#[inline(always)]
pub(crate) unsafe fn next_prop(iter: *mut PropIter) -> *const Prop {
    // SAFETY (every `*iter` access in this fn): `iter` is the caller's live
    // `PropIter` — the raw-pointer contract of this `unsafe fn`.
    if unsafe { (*iter).over } == unsafe { (*iter).over_end } {
        if unsafe { (*iter).prop } == unsafe { (*iter).prop_end } {
            return ptr::null();
        }
        // C: `return iter->prop++;`
        let prop: *const Prop = unsafe { (*iter).prop };
        // SAFETY: `prop` is short of `prop_end`, so it is inside the element's
        // prop run and `prop + 1` is at most one past its end.
        unsafe { (*iter).prop = prop.add(1) };
        prop
    } else {
        // SAFETY: `iter` is forwarded unchanged, so the callee inherits the
        // caller's contract.
        unsafe { next_prop_slow(iter) }
    }
}

// ufbx.c:25926-25973 `ufbxi_evaluate_selected_props`
#[inline(never)]
pub(crate) unsafe fn evaluate_selected_props(
    anim: *const Anim,
    element: *const Element,
    time: f64,
    props: *mut Prop,
    prop_names: *const *const u8,
    max_props: usize,
    flags: u32,
) -> crate::generated::Props {
    // SAFETY: `prop_names` is the caller's `max_props`-element name table; both
    // call sites (`ufbx_evaluate_transform_flags` and
    // `ufbx_evaluate_blend_weight_flags` in `native::api`) pass a fixed non-empty
    // table of interned NUL-terminated names, so index 0 is in bounds and is what
    // `get_name_key_c` reads.
    let mut name: *const u8 = unsafe { *prop_names.add(0) };
    let mut key: u32 = unsafe { get_name_key_c(name) };
    let mut num_props: usize = 0;

    // C: `#if defined(UFBX_REGRESSION)` — sorted-names assert.
    #[cfg(feature = "regression")]
    {
        for i in 1..max_props {
            // SAFETY: `i < max_props` bounds both reads inside the caller's name
            // table, and its entries are NUL-terminated interned names.
            ufbx_assert!(unsafe { strcmp(*prop_names.add(i - 1), *prop_names.add(i)) } < 0);
        }
    }

    let mut name_ix: usize = 0;

    let mut iter = MaybeUninit::<PropIter>::uninit(); // ufbxi_uninit
    let iter: *mut PropIter = iter.as_mut_ptr();
    // SAFETY: `iter` addresses the live local `MaybeUninit<PropIter>` storage,
    // which `init_prop_iter` only writes through; `anim`/`element` are the
    // caller's live scene objects — the contract of this `unsafe fn`.
    unsafe { init_prop_iter(iter, anim, element) };
    // C: `while ((prop = ufbxi_next_prop(&iter)) != NULL)`
    loop {
        // SAFETY: `iter` is the storage `init_prop_iter` initialized above.
        let prop: *const Prop = unsafe { next_prop(iter) };
        if prop.is_null() {
            break;
        }
        while name_ix < max_props {
            // SAFETY (every `*prop` access in this loop): `next_prop` returns
            // either an element of the element's own prop run or the iterator's
            // `tmp` prop, both live here, and it is non-null (checked above).
            if key > unsafe { (*prop)._internal_key } {
                break;
            }
            if name == unsafe { (*prop).name.data } {
                // SAFETY: as above for `prop`; `anim` is the caller's live anim.
                if (unsafe { (*prop).flags }.raw() & PropFlags::CONNECTED.raw()) != 0
                    && !unsafe { (*anim).ignore_connections }
                {
                    // C: `ufbx_prop *dst = &props[num_props++];`
                    // SAFETY: an append happens only on a match with the current
                    // table name and `break`s the inner loop, and the iterator
                    // yields each prop name once, so `num_props` stays below
                    // `max_props` — the number of slots both call sites size
                    // `props` for.
                    let dst: *mut Prop = unsafe { props.add(num_props) };
                    num_props += 1;
                    // SAFETY: `dst` is that in-bounds destination slot and
                    // `prop` a live source prop; `ufbx_prop` is plain data, so
                    // the copy duplicates no ownership.
                    unsafe { *dst = *prop };
                    // SAFETY: `dst` is the prop just written, `anim`/`element`
                    // are the caller's live scene objects and `name` an entry of
                    // the caller's name table.
                    unsafe { evaluate_connected_prop(dst, anim, element, name, time, flags) };
                } else if (unsafe { (*prop).flags }.raw()
                    & (PropFlags::ANIMATED.raw() | PropFlags::OVERRIDDEN.raw()))
                    != 0
                {
                    // C: `props[num_props++] = *prop;`
                    // SAFETY: as the `dst` write above — `num_props` is below
                    // `max_props`, and `prop` is a live source prop.
                    unsafe { *props.add(num_props) = *prop };
                    num_props += 1;
                }
                break;
            // SAFETY: `name` is an entry of the caller's name table and
            // `prop.name` is either a string-pool string (element prop) or an
            // override name interned by `create_anim_imp` — both NUL-terminated.
            } else if unsafe { strcmp(name, (*prop).name.data) } < 0 {
                name_ix += 1;
                if name_ix < max_props {
                    // SAFETY: `name_ix < max_props` bounds the read inside the
                    // caller's name table, whose entries are NUL-terminated
                    // interned names.
                    name = unsafe { *prop_names.add(name_ix) };
                    key = unsafe { get_name_key_c(name) };
                }
            } else {
                break;
            }
        }
    }

    // SAFETY: `anim`/`element` are the caller's live scene objects and `props`
    // now holds `num_props` initialized props written above.
    unsafe { evaluate_props(anim, element, time, props, num_props, flags) };

    // C: `ufbx_props prop_list;` — every field is written below.
    // SAFETY: `ufbx_props` is C plain data — a pointer/count run, counts and a
    // nullable defaults pointer — for which all-zero is a valid inhabitant.
    let mut prop_list: crate::generated::Props = unsafe { MaybeUninit::zeroed().assume_init() };
    prop_list.props.data = props;
    // C: `prop_list.props.count = prop_list.num_animated = num_props;`
    prop_list.props.count = num_props;
    prop_list.num_animated = num_props;
    // C: `prop_list.defaults = (ufbx_props*)&element->props;` — raw pointer
    // store into the `Option<Ref<Props>>` slot (same layout).
    // SAFETY: the destination is a field of the live local `prop_list`, reached
    // through a borrow, and the stored address is the caller's live element's own
    // `props` field.
    unsafe {
        *(&raw mut prop_list.defaults as *mut *const crate::generated::Props) =
            &raw const (*element).props
    };
    prop_list
}

// Recursion limited by not calling `ufbx_evaluate_curve()` with `UFBX_EVALUATE_FLAG_NO_EXTRAPOLATION`.
// ufbx.c:25975-26042 `ufbxi_extrapolate_curve`
// `ufbxi_recursive_function(ufbx_real, ..., 3, ...)` (ufbx.c:25977-25978): see
// `combine_anim_layer` above for the guard shape.
#[inline(never)]
pub(crate) unsafe fn extrapolate_curve(
    curve: *const AnimCurve,
    real_time: f64,
    flags: u32,
) -> Real {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|d| {
            ufbx_assert!(d.get() < 3);
            d.set(d.get() + 1);
        });
        // SAFETY: `curve` is the caller's live `ufbx_anim_curve` — the
        // raw-pointer contract of this `unsafe fn`, forwarded unchanged.
        let ret = unsafe { extrapolate_curve_rec(curve, real_time, flags) };
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        return ret;
    }
    #[cfg(not(feature = "regression"))]
    // SAFETY: `curve` is the caller's live `ufbx_anim_curve` — the raw-pointer
    // contract of this `unsafe fn`, forwarded unchanged.
    unsafe {
        extrapolate_curve_rec(curve, real_time, flags)
    }
}

// ufbx.c:25979-26042 `ufbxi_extrapolate_curve` body (the `_rec` half of the
// `ufbxi_recursive_function` body; see the wrapper above)
#[inline(never)]
unsafe fn extrapolate_curve_rec(curve: *const AnimCurve, real_time: f64, flags: u32) -> Real {
    // SAFETY: `curve` is the caller's live `ufbx_anim_curve` — the raw-pointer
    // contract of this `unsafe fn`.
    let pre: bool = real_time < unsafe { (*curve).min_time };
    let key: *const Keyframe;
    // C: `ufbx_extrapolation ext;` — copied by value; read through a pointer
    // here (`Extrapolation` carries no `Copy`), same fields, same reads.
    let ext: *const Extrapolation;
    if pre {
        // SAFETY: `curve` is the caller's live curve, and `ufbx_evaluate_curve_flags`
        // only extrapolates once it has established `keyframes.count > 1`, so the
        // keyframe list is non-empty and slot 0 is in bounds.
        key = unsafe { (*curve).keyframes.data.add(0) };
        // SAFETY: `pre_extrapolation` is a field of the caller's live curve.
        ext = unsafe { &raw const (*curve).pre_extrapolation };
    } else {
        // SAFETY: as above — the keyframe list holds more than one element, so
        // `count - 1` does not underflow and indexes the last slot.
        key = unsafe { (*curve).keyframes.data.add((*curve).keyframes.count - 1) };
        // SAFETY: `post_extrapolation` is a field of the caller's live curve.
        ext = unsafe { &raw const (*curve).post_extrapolation };
    }

    // SAFETY: `ext` addresses one of the two extrapolation fields of the live
    // curve, assigned on both arms above.
    if unsafe { (*ext).mode } == ExtrapolationMode::Constant {
        // SAFETY: `key` addresses an in-bounds keyframe of the live curve.
        return unsafe { (*key).value };
    // SAFETY: `ext` addresses an extrapolation field of the live curve.
    } else if unsafe { (*ext).mode } == ExtrapolationMode::Slope {
        // C: `ufbx_tangent tangent = *(pre ? &key->right : &key->left);`
        // SAFETY: `key` addresses an in-bounds keyframe of the live curve, so
        // both of its tangent fields are readable.
        let tangent: Tangent = unsafe {
            *(if pre {
                &raw const (*key).right
            } else {
                &raw const (*key).left
            })
        };
        // C: `key->value + (ufbx_real)(tangent.dy * ((real_time - key->time) / tangent.dx))`
        // — `dx`/`dy` are float, promoted to double in the expression.
        // SAFETY: `key` addresses an in-bounds keyframe of the live curve.
        return unsafe { (*key).value }
            + (tangent.dy as f64 * ((real_time - unsafe { (*key).time }) / tangent.dx as f64))
                as Real;
    // SAFETY: `ext` addresses an extrapolation field of the live curve.
    } else if unsafe { (*ext).repeat_count } == 0 {
        // SAFETY: `key` addresses an in-bounds keyframe of the live curve.
        return unsafe { (*key).value };
    }

    // Perform all operations in KTime ticks to be frame perfect
    // SAFETY: `element.scene` is the non-null owning-scene `Ref` of the live
    // curve, so `ref_ptr` yields that live `ufbx_scene` to read metadata from.
    let scale: f64 = unsafe { (*ref_ptr(&(*curve).element.scene)).metadata.ktime_second } as f64;
    // SAFETY: `curve` is the caller's live curve.
    let min_time: f64 = math::rint(unsafe { (*curve).min_time } * scale);
    // SAFETY: `curve` is the caller's live curve.
    let max_time: f64 = math::rint(unsafe { (*curve).max_time } * scale);
    let time: f64 = real_time * scale;

    let delta: f64 = if pre {
        min_time - time
    } else {
        time - max_time
    };
    let duration: f64 = max_time - min_time;

    // Require at least one KTime unit
    if !(duration >= 1.0) {
        // SAFETY: `key` addresses an in-bounds keyframe of the live curve.
        return unsafe { (*key).value };
    }

    let rep: f64 = delta / duration;
    let mut rep_n: f64 = math::floor(rep);
    let mut rep_d: f64 = delta - rep_n * duration;

    // SAFETY (this condition): `ext` addresses an extrapolation field of the
    // live curve.
    if unsafe { (*ext).repeat_count } > 0 && rep_n >= unsafe { (*ext).repeat_count } as f64 {
        // Clamp to the repeat count to handle mirroring
        // SAFETY: as above.
        rep_n = (unsafe { (*ext).repeat_count } - 1) as f64;
        rep_d = duration;
    }

    // SAFETY: `ext` addresses an extrapolation field of the live curve.
    if unsafe { (*ext).mode } == ExtrapolationMode::Mirror {
        let rep_parity: f64 = rep_n * 0.5 - math::floor(rep_n * 0.5);
        if rep_parity <= 0.25 {
            rep_d = duration - rep_d;
        }
    }

    if pre {
        rep_d = duration - rep_d;
    }
    let new_time: f64 = (min_time + rep_d) / scale;

    // SAFETY: `curve` is the caller's live curve — `ufbx_evaluate_curve_flags`'
    // raw-pointer contract — and `key` addresses an in-bounds keyframe of it.
    let mut value: Real = unsafe {
        evaluate_curve_flags(
            curve,
            new_time,
            (*key).value,
            flags | crate::generated::EvaluateFlags::NO_EXTRAPOLATION.raw(),
        )
    };

    // SAFETY: `ext` addresses an extrapolation field of the live curve.
    if unsafe { (*ext).mode } == ExtrapolationMode::RepeatRelative {
        // SAFETY (both reads): the live curve's keyframe list holds more than
        // one element (the caller only extrapolates past that check), so both
        // the last slot and slot 0 are in bounds.
        let mut val_delta: Real =
            unsafe { (*(*curve).keyframes.data.add((*curve).keyframes.count - 1)).value }
                - unsafe { (*(*curve).keyframes.data.add(0)).value };
        if pre {
            val_delta = -val_delta;
        }
        // C: `value += val_delta * (ufbx_real)(rep_n + 1.0);`
        value += val_delta * ((rep_n + 1.0) as Real);
    }

    value
}

// C: `#if UFBXI_FEATURE_SCENE_EVALUATION` (ufbx.c:26044) — the whole eval
// context block below is gated on `feature = "scene-eval"`.

// ufbx.c:26046-26068 `ufbxi_eval_context`
#[cfg(feature = "scene-eval")]
#[repr(C)]
pub(crate) struct InnerEvalContext {
    pub src_element: *mut u8,
    pub dst_element: *mut u8,

    pub src_imp: *mut SceneImp,
    pub src_scene: Scene,
    pub opts: RawEvaluateOpts,
    pub anim: *mut Anim,
    pub time: f64,

    pub error: Error,

    // Allocators
    pub ator_result: Allocator,
    pub ator_tmp: Allocator,

    pub result: Buf,
    pub tmp: Buf,

    pub scene: Scene,

    pub scene_imp: *mut SceneImp,
}

// Safe `&EvalContext` handle over the fields-struct `InnerEvalContext`, mirroring
// the `Context`/`InnerContext` seam in `parse.rs`. `MaybeUninit` because it embeds
// the public `Scene` (enum-bearing) in `src_scene`/`scene`, so a plain
// `&InnerEvalContext` could not be formed soundly; `UnsafeCell` gives the interior
// mutability every `&EvalContext` site needs. Field is `pub(crate)` — the sole
// construction site lives in `native::api`.
#[repr(transparent)]
#[cfg(feature = "scene-eval")]
pub(crate) struct EvalContext(
    pub(crate) core::cell::UnsafeCell<core::mem::MaybeUninit<InnerEvalContext>>,
);

// Typed interior-mutable VIEW over the `opts` field, reinterpreted in place
// (approach A). Generated ABI-fixed `RawEvaluateOpts` plays the `Inner` role;
// `MaybeUninit` makes forming `&EvaluateOptsView` assert no validity — each leaf getter
// asserts only the field it reads.
#[cfg(feature = "scene-eval")]
pub(crate) type EvaluateOptsView = crate::native::view::View<RawEvaluateOpts>;

#[cfg(feature = "scene-eval")]
impl EvaluateOptsView {
    #[inline(always)]
    pub(crate) fn evaluate_caches(&self) -> bool {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.evaluate_caches` read already makes.
        unsafe { (*self.get()).evaluate_caches }
    }

    #[inline(always)]
    pub(crate) fn evaluate_flags(&self) -> u32 {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.evaluate_flags` read already makes.
        unsafe { (*self.get()).evaluate_flags }
    }

    #[inline(always)]
    pub(crate) fn evaluate_skinning(&self) -> bool {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.evaluate_skinning` read already makes.
        unsafe { (*self.get()).evaluate_skinning }
    }

    #[inline(always)]
    pub(crate) fn load_external_files(&self) -> bool {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.load_external_files` read already makes.
        unsafe { (*self.get()).load_external_files }
    }

    #[inline(always)]
    pub(crate) fn open_file_cb_ptr(&self) -> *const crate::generated::RawOpenFileCb {
        // SAFETY: `&raw const` address of a read-only sub-struct.
        unsafe { &raw const (*self.get()).open_file_cb }
    }

    #[inline(always)]
    pub(crate) fn result_allocator_ptr(&self) -> *const crate::generated::RawAllocatorOpts {
        // SAFETY: `&raw const` address of a read-only sub-struct.
        unsafe { &raw const (*self.get()).result_allocator }
    }

    #[inline(always)]
    pub(crate) fn temp_allocator_ptr(&self) -> *const crate::generated::RawAllocatorOpts {
        // SAFETY: `&raw const` address of a read-only sub-struct.
        unsafe { &raw const (*self.get()).temp_allocator }
    }
}

#[cfg(feature = "scene-eval")]
impl EvalContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerEvalContext {
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

    // `scene` (Scene) — typed VIEW handle (reinterpret-in-place); accessors on SceneView.
    #[inline(always)]
    pub(crate) fn scene_view(&self) -> &crate::native::parse::SceneView {
        // SAFETY: repr(transparent) over the owned `scene` field inside the outer
        // UnsafeCell; shared interior-mutable view, asserts no validity.
        unsafe { &*(&raw mut (*self.get()).scene as *mut crate::native::parse::SceneView) }
    }

    // `src_scene` (Scene) — typed VIEW handle (reinterpret-in-place); accessors on SceneView.
    #[inline(always)]
    pub(crate) fn src_scene_view(&self) -> &crate::native::parse::SceneView {
        // SAFETY: repr(transparent) over the owned `src_scene` field inside the outer
        // UnsafeCell; shared interior-mutable view, asserts no validity.
        unsafe { &*(&raw mut (*self.get()).src_scene as *mut crate::native::parse::SceneView) }
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

    // `opts` — typed VIEW handle (reinterpret-in-place); leaf accessors on `EvaluateOptsView`.
    #[inline(always)]
    pub(crate) fn opts_view(&self) -> &EvaluateOptsView {
        // SAFETY: `EvaluateOptsView` is repr(transparent) over the `opts` field's layout,
        // which lives in this context's outer UnsafeCell; a shared interior-mutable
        // `&EvaluateOptsView` is sound and asserts no validity.
        unsafe { &*(&raw mut (*self.get()).opts as *mut EvaluateOptsView) }
    }

    // `tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp }
    }

    // `src_scene` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn src_scene_mut_ptr(&self) -> *mut Scene {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).src_scene }
    }

    // `scene` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn scene_mut_ptr(&self) -> *mut Scene {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).scene }
    }

    // `result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn result_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).result }
    }

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawEvaluateOpts {
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

    // `ator_tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_tmp_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_tmp }
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
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_result }
    }

    // `anim` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn anim_mut_ptr(&self) -> *mut *mut Anim {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).anim }
    }

    // `scene_imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn scene_imp(&self) -> *mut SceneImp {
        // SAFETY: reading a scalar field; all bit patterns of `*mut SceneImp` are valid.
        unsafe { (*self.get()).scene_imp }
    }

    #[inline(always)]
    pub(crate) fn set_scene_imp(&self, scene_imp: *mut SceneImp) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).scene_imp = scene_imp;
        }
    }

    // `time` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn time(&self) -> f64 {
        // SAFETY: reading a scalar field; all bit patterns of `f64` are valid.
        unsafe { (*self.get()).time }
    }

    #[inline(always)]
    pub(crate) fn set_time(&self, time: f64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).time = time;
        }
    }

    // `anim` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn anim(&self) -> *mut Anim {
        // SAFETY: reading a scalar field; all bit patterns of `*mut Anim` are valid.
        unsafe { (*self.get()).anim }
    }

    #[inline(always)]
    pub(crate) fn set_anim(&self, anim: *mut Anim) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).anim = anim;
        }
    }

    // `src_imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn src_imp(&self) -> *mut SceneImp {
        // SAFETY: reading a scalar field; all bit patterns of `*mut SceneImp` are valid.
        unsafe { (*self.get()).src_imp }
    }

    #[inline(always)]
    pub(crate) fn set_src_imp(&self, src_imp: *mut SceneImp) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).src_imp = src_imp;
        }
    }

    #[inline(always)]
    pub(crate) fn dst_element(&self) -> *mut u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u8` are valid.
        unsafe { (*self.get()).dst_element }
    }

    #[inline(always)]
    pub(crate) fn set_dst_element(&self, dst_element: *mut u8) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).dst_element = dst_element;
        }
    }

    // `src_element` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn src_element(&self) -> *mut u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u8` are valid.
        unsafe { (*self.get()).src_element }
    }

    #[inline(always)]
    pub(crate) fn set_src_element(&self, src_element: *mut u8) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).src_element = src_element;
        }
    }
}

// ufbx.c:26070-26073 `ufbxi_translate_element`
// C: `ufbxi_forceinline`.
#[cfg(feature = "scene-eval")]
#[inline(always)]
pub(crate) unsafe fn translate_element(ec: &EvalContext, elem: *mut c_void) -> *mut Element {
    // C: `elem ? (ufbx_element*)(ec->dst_element + ((char*)elem - ec->src_element)) : NULL`
    if !elem.is_null() {
        // SAFETY: this `unsafe fn` requires `elem` to be an element of the source
        // scene's element buffer, whose base `ec.src_element()` is — so both lie
        // in that one allocation and `offset_from` is well defined. The same
        // offset lands inside the freshly pushed destination buffer, which
        // `evaluate_imp` sizes from the source scene's `element_buffer_size` and
        // whose base is `ec.dst_element()`.
        unsafe {
            ec.dst_element()
                .offset((elem as *mut u8).offset_from(ec.src_element())) as *mut Element
        }
    } else {
        ptr::null_mut()
    }
}

// ufbx.c:26075-26087 `ufbxi_translate_element_list`
#[cfg(feature = "scene-eval")]
#[inline(never)]
pub(crate) unsafe fn translate_element_list(
    ec: &EvalContext,
    p_list: *mut c_void,
) -> Result<(), Fail> {
    // C: `ufbx_element_list *list = (ufbx_element_list*)p_list;`
    let list: *mut crate::prelude::RefList<Element> =
        p_list as *mut crate::prelude::RefList<Element>;
    // SAFETY: this `unsafe fn` requires `p_list` to address a live
    // `ufbx_element_list` of the source scene.
    let count: usize = unsafe { (*list).count };
    // SAFETY: as above, reading the same live list's element array base.
    let src: *mut *mut Element = unsafe { (*list).data as *mut *mut Element };
    let dst: *mut *mut Element = ec.result_view().push::<*mut Element>(count);
    ufbxi_check_err!(ec.error_view(), !dst.is_null(), "dst");
    // SAFETY: as above, retargeting the same live list at the pushed array.
    unsafe { (*list).data = dst as *const Ref<Element> };
    for i in 0..count {
        // SAFETY: `i < count`, so slot `i` is in bounds of both the source list's
        // element array and the `count`-element allocation just pushed into
        // `dst`; the source slot holds a source-scene element, which is
        // `translate_element`'s contract.
        unsafe { *dst.add(i) = translate_element(ec, *src.add(i) as *mut c_void) };
    }
    Ok(())
}

// ufbx.c:26089-26094 `ufbxi_translate_maps`
#[cfg(feature = "scene-eval")]
#[inline(never)]
pub(crate) unsafe fn translate_maps(ec: &EvalContext, maps: *mut MaterialMap, count: usize) {
    // C: `ufbxi_nounroll ufbxi_for(ufbx_material_map, map, maps, count)`
    let mut map: *mut MaterialMap = maps;
    let map_end: *mut MaterialMap = add_ptr(maps, count);
    while map != map_end {
        // SAFETY: `map` walks the caller's `count`-element `ufbx_material_map`
        // run and stops at `map_end`, so it addresses a live map; `texture` is
        // that map's own `Option<Ref<Texture>>` field, which `opt_ptr` reads as
        // the nullable element pointer it is and the store writes back in place.
        // The texture it holds belongs to the source scene, which is
        // `translate_element`'s contract.
        unsafe {
            *(&raw mut (*map).texture as *mut *mut Texture) =
                translate_element(ec, opt_ptr(&raw const (*map).texture) as *mut c_void)
                    as *mut Texture;
        }
        // SAFETY: `map` is inside the run, so `map + 1` is at most one past its
        // end.
        map = unsafe { map.add(1) };
    }
}

// ufbx.c:26096-26103 `ufbxi_translate_anim`
#[cfg(feature = "scene-eval")]
#[inline(never)]
pub(crate) unsafe fn translate_anim(ec: &EvalContext, p_anim: *mut *mut Anim) -> Result<(), Fail> {
    // SAFETY: `ec.result_mut_ptr()` is the eval context's own result buffer and
    // `p_anim` addresses the caller's live `ufbx_anim*` slot, whose pointee is
    // the source-scene anim `push_copy` copies one of.
    let anim: *mut Anim = unsafe { push_copy::<Anim>(ec.result_mut_ptr(), 1, *p_anim) };
    ufbxi_check_err!(ec.error_view(), !anim.is_null(), "anim");
    // SAFETY: `anim` is the non-null (checked just above) freshly pushed copy, so
    // its `layers` field is a live `ufbx_element_list` of source-scene layers —
    // `translate_element_list`'s contract.
    unsafe { translate_element_list(ec, &raw mut (*anim).layers as *mut c_void) }?;
    // SAFETY: `p_anim` addresses the caller's live `ufbx_anim*` slot.
    unsafe { *p_anim = anim };
    Ok(())
}

// ufbx.c:26105-26444 `ufbxi_evaluate_imp`
// Stays `unsafe fn`: the body rebuilds a whole scene by *cross-allocation
// provenance arithmetic*, rebasing every element's `connections_src/dst` with
// `offset_from` between the source scene's run and the freshly pushed one, and
// copying `ELEMENT_TYPE_SIZE[type]` bytes over a bump-allocated
// `element_buffer_size` region whose layout only the source scene's metadata
// describes. The blocks below all lean on one whole-function obligation they
// cannot establish themselves, called the SOURCE-SCENE PREMISE below: `ec` holds
// a self-consistent loaded `ufbx_scene` in `src_scene` — element list, per-type
// lists, connection runs and `elements_by_name` sized and cross-indexed as
// `ufbxi_load_scene` left them, every element living in the one
// `element_buffer_size` byte run based at `src_scene.elements.data[0]`.
#[cfg(feature = "scene-eval")]
#[inline(never)]
pub(crate) unsafe fn evaluate_imp(ec: &EvalContext) -> Result<(), Fail> {
    // C: `ec->scene = ec->src_scene;` — struct assignment (memcpy).
    // SAFETY: source and destination are two distinct `ufbx_scene` fields of
    // `ec`'s own live context struct, so both are valid, aligned and disjoint.
    unsafe { ptr::copy_nonoverlapping(ec.src_scene_mut_ptr(), ec.scene_mut_ptr(), 1) };
    let num_elements: usize = ec.scene_view().elements_view().count();

    // C: `char *element_data = (char*)ufbxi_push(&ec->result, uint64_t, ec->scene.metadata.element_buffer_size/8);`
    let element_data: *mut u8 = ec
        .result_view()
        .push::<u64>(ec.scene_view().metadata_view().element_buffer_size() / 8)
        as *mut u8;
    ufbxi_check_err!(ec.error_view(), !element_data.is_null(), "element_data");

    ec.scene_view()
        .elements_view()
        .set_data(ec.result_view().push::<*mut Element>(num_elements) as *const Ref<Element>);
    ufbxi_check_err!(
        ec.error_view(),
        !ec.scene_view().elements_view().data().is_null(),
        "ec->scene.elements.data"
    );

    // C: `ec->src_element = (char*)ec->src_scene.elements.data[0];`
    // SAFETY: per the source-scene premise the source element list is a live
    // array and holds at least the root node, so slot 0 is in bounds and yields
    // the base element of the source element buffer.
    ec.set_src_element(unsafe {
        *(ec.src_scene_view().elements_view().data() as *mut *mut u8).add(0)
    });
    ec.set_dst_element(element_data);

    // C indexes `ec->scene.elements_by_type[i]`, the `ufbx_element_list` array
    // view of the `ufbx_scene` per-type list union (ufbx.h); the generated
    // struct keeps only the named branch, whose first member (`unknowns`) is
    // the array base (same treatment as `native::scene_process`).
    let by_type: *mut crate::prelude::RefList<Element> =
        ec.scene_view().unknowns_mut_ptr() as *mut crate::prelude::RefList<Element>;
    for i in 0..ELEMENT_TYPE_COUNT {
        // SAFETY: `by_type` is the destination scene's `ELEMENT_TYPE_COUNT`-long
        // per-type list array and `i` is in range, so slot `i` is a live
        // `ufbx_element_list`: its count is read, then it is retargeted at the
        // array just pushed for it.
        unsafe {
            (*by_type.add(i)).data = ec
                .result_view()
                .push::<*mut Element>((*by_type.add(i)).count)
                as *const Ref<Element>;
        }
        ufbxi_check_err!(
            ec.error_view(),
            // SAFETY: as above, slot `i` of the per-type list array is live.
            !unsafe { (*by_type.add(i)).data }.is_null(),
            "ec->scene.elements_by_type[i].data"
        );
    }

    let num_connections: usize = ec.scene_view().connections_dst_view().count();
    ec.scene_view()
        .connections_src_view()
        .set_data(ec.result_view().push::<Connection>(num_connections));
    ec.scene_view()
        .connections_dst_view()
        .set_data(ec.result_view().push::<Connection>(num_connections));
    ufbxi_check_err!(
        ec.error_view(),
        !ec.scene_view().connections_src_view().data().is_null(),
        "ec->scene.connections_src.data"
    );
    ufbxi_check_err!(
        ec.error_view(),
        !ec.scene_view().connections_dst_view().data().is_null(),
        "ec->scene.connections_dst.data"
    );
    for i in 0..num_connections {
        // SAFETY: both destination connection arrays were pushed with
        // `num_connections` elements (non-null, checked just above) and
        // `i < num_connections`, so slot `i` is in bounds of each.
        let src: *mut Connection =
            unsafe { (ec.scene_view().connections_src_view().data() as *mut Connection).add(i) };
        // SAFETY: as above, for the `connections_dst` array.
        let dst: *mut Connection =
            unsafe { (ec.scene_view().connections_dst_view().data() as *mut Connection).add(i) };
        // C: `*src = ec->src_scene.connections_src.data[i];` (struct assignment)
        // SAFETY: per the source-scene premise the source `connections_src` run
        // holds `num_connections` live `ufbx_connection`s, so slot `i` is
        // readable; `src` addresses the matching slot of the freshly pushed
        // destination run, a separate allocation.
        unsafe {
            ptr::copy_nonoverlapping(
                ec.src_scene_view().connections_src_view().data().add(i),
                src,
                1,
            );
        }
        // SAFETY: as above, for the `connections_dst` runs.
        unsafe {
            ptr::copy_nonoverlapping(
                ec.src_scene_view().connections_dst_view().data().add(i),
                dst,
                1,
            );
        }
        // SAFETY (these four stores): `src` and `dst` address live connection
        // slots holding the copies made just above, whose `src`/`dst` are
        // non-null `Ref`s to source-scene elements — `translate_element`'s
        // contract — read through `ref_ptr` and written back in place.
        unsafe {
            *(&raw mut (*src).src as *mut *mut Element) =
                translate_element(ec, ref_ptr(&(*src).src) as *mut c_void);
        }
        unsafe {
            *(&raw mut (*src).dst as *mut *mut Element) =
                translate_element(ec, ref_ptr(&(*src).dst) as *mut c_void);
        }
        unsafe {
            *(&raw mut (*dst).src as *mut *mut Element) =
                translate_element(ec, ref_ptr(&(*dst).src) as *mut c_void);
        }
        unsafe {
            *(&raw mut (*dst).dst as *mut *mut Element) =
                translate_element(ec, ref_ptr(&(*dst).dst) as *mut c_void);
        }
    }

    ec.scene_view()
        .elements_by_name_view()
        .set_data(ec.result_view().push::<NameElement>(num_elements));
    ufbxi_check_err!(
        ec.error_view(),
        !ec.scene_view().elements_by_name_view().data().is_null(),
        "ec->scene.elements_by_name.data"
    );

    // C: `ec->scene.root_node = (ufbx_node*)ufbxi_translate_element(ec, ec->scene.root_node);`
    // SAFETY: `root_node` is a field of the destination scene struct inside `ec`,
    // holding the non-null `Ref` copied from the source scene, which per the
    // source-scene premise names a source-scene element — `translate_element`'s
    // contract.
    unsafe {
        *(ec.scene_view().root_node_mut_ptr() as *mut *mut UfbxNode) =
            translate_element(ec, ref_ptr(ec.scene_view().root_node_ptr()) as *mut c_void)
                as *mut UfbxNode;
    }
    // SAFETY: `anim` is a field of the destination scene struct inside `ec`, so
    // the pointer addresses a live `ufbx_anim*` slot — `translate_anim`'s
    // contract.
    unsafe { translate_anim(ec, ec.scene_view().anim_mut_ptr() as *mut *mut Anim) }?;

    for i in 0..num_elements {
        // SAFETY: per the source-scene premise the source element list holds
        // `num_elements` live element pointers and `i < num_elements`, so slot
        // `i` is in bounds and holds a source-scene element.
        let src: *mut Element =
            unsafe { *(ec.src_scene_view().elements_view().data() as *mut *mut Element).add(i) };
        // SAFETY: `src` is a source-scene element — `translate_element`'s
        // contract — so `dst` is the matching slot of the destination element
        // buffer.
        let dst: *mut Element = unsafe { translate_element(ec, src as *mut c_void) };
        // SAFETY: `src` is a live source-scene element, so its `type` tag is
        // readable (and, being a valid `ufbx_element_type`, indexes the table).
        let size: usize = ELEMENT_TYPE_SIZE[unsafe { (*src).type_ } as usize];
        ufbx_assert!(size > 0);
        // C: `memcpy(dst, src, size);`
        // SAFETY: `size` is the byte size of `src`'s element type, so those bytes
        // are readable at `src`; the destination buffer is a separate allocation
        // sized from the source scene's `element_buffer_size`, which per the
        // source-scene premise covers the same per-element layout at `dst`.
        unsafe { ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, size) };

        // SAFETY: the destination element list was pushed with `num_elements`
        // slots (non-null, checked above) and `i < num_elements`.
        unsafe { *(ec.scene_view().elements_view().data() as *mut *mut Element).add(i) = dst };
        // SAFETY: `src` is live, so its `type`/`typed_id` are readable; per the
        // source-scene premise `typed_id` indexes the element's own per-type
        // list, and the destination per-type array pushed above matches the
        // source counts element for element.
        unsafe {
            *((*by_type.add((*src).type_ as usize)).data as *mut *mut Element)
                .add((*src).typed_id as usize) = dst;
        }

        // C: `dst->connections_src.data = ec->scene.connections_src.data + (dst->connections_src.data - ec->src_scene.connections_src.data);`
        // SAFETY: `dst` holds the byte copy of `src` made above, so its
        // `connections_src.data` still points into the source scene's
        // `connections_src` run — one allocation with the base being subtracted,
        // making `offset_from` well defined — and the same index lands inside the
        // equally sized destination run pushed above.
        unsafe {
            (*dst).connections_src.data = ec.scene_view().connections_src_view().data().offset(
                (*dst)
                    .connections_src
                    .data
                    .offset_from(ec.src_scene_view().connections_src_view().data()),
            );
        }
        // SAFETY: as above, for the `connections_dst` runs.
        unsafe {
            (*dst).connections_dst.data = ec.scene_view().connections_dst_view().data().offset(
                (*dst)
                    .connections_dst
                    .data
                    .offset_from(ec.src_scene_view().connections_dst_view().data()),
            );
        }
        // SAFETY: `dst` is the live destination element copy, so `instances` is a
        // live `ufbx_element_list` of source-scene elements —
        // `translate_element_list`'s contract.
        if unsafe { (*dst).instances.count } > 0 {
            // SAFETY: as above.
            unsafe { translate_element_list(ec, &raw mut (*dst).instances as *mut c_void) }?;
        }

        // C: `ufbx_name_element named = ec->src_scene.elements_by_name.data[i];`
        // then `named.element = ...; ec->scene.elements_by_name.data[i] = named;`
        // — copied straight into the destination slot here (same writes).
        // SAFETY: the destination `elements_by_name` array was pushed with
        // `num_elements` entries (non-null, checked above) and `i < num_elements`.
        let named: *mut NameElement =
            unsafe { (ec.scene_view().elements_by_name_view().data() as *mut NameElement).add(i) };
        // SAFETY: per the source-scene premise the source `elements_by_name` run
        // holds `num_elements` live entries, so slot `i` is readable; `named`
        // addresses the matching slot of the freshly pushed destination array, a
        // separate allocation.
        unsafe {
            ptr::copy_nonoverlapping(
                ec.src_scene_view().elements_by_name_view().data().add(i),
                named,
                1,
            );
        }
        // SAFETY: `named` addresses the live entry copied just above, whose
        // `element` is a non-null `Ref` to a source-scene element —
        // `translate_element`'s contract — read through `ref_ptr` and written
        // back in place.
        unsafe {
            *(&raw mut (*named).element as *mut *mut Element) =
                translate_element(ec, ref_ptr(&(*named).element) as *mut c_void);
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_node, p_node, ec->scene.nodes)`
    let mut p_node: *mut *mut UfbxNode = ec.scene_view().nodes_view().data() as *mut *mut UfbxNode;
    let p_node_end: *mut *mut UfbxNode = add_ptr(p_node, ec.scene_view().nodes_view().count());
    while p_node != p_node_end {
        // SAFETY: `p_node` walks the destination scene's node pointer list and
        // stops at `p_node_end`, so it addresses a live slot holding one of the
        // element copies the loop above wrote into the destination buffer.
        let node: *mut UfbxNode = unsafe { *p_node };
        // SAFETY (this store and the ones below it): `node` is that live
        // destination node; each named field is its own nullable
        // `Option<Ref<..>>`, which `opt_ptr` reads as the element pointer it is.
        // The byte copy left every one of them still pointing at the source-scene
        // element it named — `translate_element`'s contract — and the translated
        // pointer is written back into the same field.
        unsafe {
            *(&raw mut (*node).parent as *mut *mut UfbxNode) =
                translate_element(ec, opt_ptr(&raw const (*node).parent) as *mut c_void)
                    as *mut UfbxNode;
        }
        // SAFETY: `children` is the live node's own `ufbx_element_list`, still
        // listing source-scene elements — `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, &raw mut (*node).children as *mut c_void) }?;

        // SAFETY: as for `parent` above.
        unsafe {
            *(&raw mut (*node).attrib as *mut *mut Element) =
                translate_element(ec, opt_ptr(&raw const (*node).attrib) as *mut c_void);
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(&raw mut (*node).mesh as *mut *mut Mesh) =
                translate_element(ec, opt_ptr(&raw const (*node).mesh) as *mut c_void) as *mut Mesh;
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(&raw mut (*node).light as *mut *mut crate::generated::Light) =
                translate_element(ec, opt_ptr(&raw const (*node).light) as *mut c_void)
                    as *mut crate::generated::Light;
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(&raw mut (*node).camera as *mut *mut Camera) =
                translate_element(ec, opt_ptr(&raw const (*node).camera) as *mut c_void)
                    as *mut Camera;
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(&raw mut (*node).bone as *mut *mut crate::generated::Bone) =
                translate_element(ec, opt_ptr(&raw const (*node).bone) as *mut c_void)
                    as *mut crate::generated::Bone;
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(&raw mut (*node).inherit_scale_node as *mut *mut UfbxNode) = translate_element(
                ec,
                opt_ptr(&raw const (*node).inherit_scale_node) as *mut c_void,
            )
                as *mut UfbxNode;
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(&raw mut (*node).scale_helper as *mut *mut UfbxNode) =
                translate_element(ec, opt_ptr(&raw const (*node).scale_helper) as *mut c_void)
                    as *mut UfbxNode;
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(&raw mut (*node).bind_pose as *mut *mut Pose) =
                translate_element(ec, opt_ptr(&raw const (*node).bind_pose) as *mut c_void)
                    as *mut Pose;
        }

        // SAFETY: `all_attribs` is the live node's own `ufbx_element_list`.
        if unsafe { (*node).all_attribs.count } > 1 {
            // SAFETY: that list still names source-scene elements —
            // `translate_element_list`'s contract.
            unsafe { translate_element_list(ec, &raw mut (*node).all_attribs as *mut c_void) }?;
        // SAFETY: as above, reading the same live node's list count.
        } else if unsafe { (*node).all_attribs.count } == 1 {
            // C: `node->all_attribs.data = &node->attrib;`
            // SAFETY: both fields belong to the live destination node, so the
            // single-element list is retargeted at that node's own already
            // translated `attrib` slot.
            unsafe {
                (*node).all_attribs.data = &raw const (*node).attrib as *const Ref<Element>;
            }
        }

        // SAFETY: as for `parent` above.
        unsafe {
            *(&raw mut (*node).geometry_transform_helper as *mut *mut UfbxNode) = translate_element(
                ec,
                opt_ptr(&raw const (*node).geometry_transform_helper) as *mut c_void,
            )
                as *mut UfbxNode;
        }

        // SAFETY: `materials` is the live node's own `ufbx_element_list`, still
        // listing source-scene elements — `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, &raw mut (*node).materials as *mut c_void) }?;
        // SAFETY: `p_node` is inside the node list, so `p_node + 1` is at most
        // one past its end.
        p_node = unsafe { p_node.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, ec->scene.meshes)`
    let mut p_mesh: *mut *mut Mesh = ec.scene_view().meshes_view().data() as *mut *mut Mesh;
    let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, ec.scene_view().meshes_view().count());
    while p_mesh != p_mesh_end {
        // SAFETY: `p_mesh` walks the destination scene's mesh pointer list and
        // stops at `p_mesh_end`, so it addresses a live slot holding one of the
        // element copies written into the destination buffer above.
        let mesh: *mut Mesh = unsafe { *p_mesh };
        // SAFETY: `mesh` is one of the element copies written into the
        // destination scene's result buffer above — a live, initialized mesh
        // reached through `*mut` (write-capable provenance for `Mut`).
        let mesh = unsafe { View::<Mesh>::from_ptr(mesh) };

        // SAFETY (these five calls): each `*_raw` projection addresses the live
        // destination mesh's own `ufbx_element_list`, whose byte copy still
        // lists source-scene elements — `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, mesh.materials_raw() as *mut c_void) }?;
        // SAFETY: as above.
        unsafe { translate_element_list(ec, mesh.skin_deformers_raw() as *mut c_void) }?;
        // SAFETY: as above.
        unsafe { translate_element_list(ec, mesh.blend_deformers_raw() as *mut c_void) }?;
        // SAFETY: as above.
        unsafe { translate_element_list(ec, mesh.cache_deformers_raw() as *mut c_void) }?;
        // SAFETY: as above.
        unsafe { translate_element_list(ec, mesh.all_deformers_raw() as *mut c_void) }?;
        // SAFETY: `p_mesh` is inside the mesh list, so `p_mesh + 1` is at most
        // one past its end.
        p_mesh = unsafe { p_mesh.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_stereo_camera, p_stereo, ec->scene.stereo_cameras)`
    let mut p_stereo: *mut *mut StereoCamera =
        ec.scene_view().stereo_cameras_view().data() as *mut *mut StereoCamera;
    let p_stereo_end: *mut *mut StereoCamera =
        add_ptr(p_stereo, ec.scene_view().stereo_cameras_view().count());
    while p_stereo != p_stereo_end {
        // SAFETY: `p_stereo` walks the destination scene's stereo-camera pointer
        // list and stops at `p_stereo_end`, so it addresses a live slot holding
        // one of the element copies written into the destination buffer above.
        let stereo: *mut StereoCamera = unsafe { *p_stereo };
        // SAFETY (both stores): `left`/`right` are the live stereo camera's own
        // nullable `Option<Ref<Camera>>` fields, which `opt_ptr` reads as element
        // pointers; the byte copy left them naming source-scene elements —
        // `translate_element`'s contract — and the result is written back in
        // place.
        unsafe {
            *(&raw mut (*stereo).left as *mut *mut Camera) =
                translate_element(ec, opt_ptr(&raw const (*stereo).left) as *mut c_void)
                    as *mut Camera;
        }
        // SAFETY: as above.
        unsafe {
            *(&raw mut (*stereo).right as *mut *mut Camera) =
                translate_element(ec, opt_ptr(&raw const (*stereo).right) as *mut c_void)
                    as *mut Camera;
        }
        // SAFETY: `p_stereo` is inside the list, so `p_stereo + 1` is at most one
        // past its end.
        p_stereo = unsafe { p_stereo.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_skin_deformer, p_skin, ec->scene.skin_deformers)`
    let mut p_skin: *mut *mut SkinDeformer =
        ec.scene_view().skin_deformers_view().data() as *mut *mut SkinDeformer;
    let p_skin_end: *mut *mut SkinDeformer =
        add_ptr(p_skin, ec.scene_view().skin_deformers_view().count());
    while p_skin != p_skin_end {
        // SAFETY: `p_skin` walks the destination scene's skin-deformer pointer
        // list and stops at `p_skin_end`, so it addresses a live slot holding one
        // of the element copies written into the destination buffer above.
        let skin: *mut SkinDeformer = unsafe { *p_skin };
        // SAFETY: `clusters` is that live deformer's own `ufbx_element_list`,
        // whose byte copy still lists source-scene elements —
        // `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, &raw mut (*skin).clusters as *mut c_void) }?;
        // SAFETY: `p_skin` is inside the list, so `p_skin + 1` is at most one
        // past its end.
        p_skin = unsafe { p_skin.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, ec->scene.skin_clusters)`
    let mut p_cluster: *mut *mut SkinCluster =
        ec.scene_view().skin_clusters_view().data() as *mut *mut SkinCluster;
    let p_cluster_end: *mut *mut SkinCluster =
        add_ptr(p_cluster, ec.scene_view().skin_clusters_view().count());
    while p_cluster != p_cluster_end {
        // SAFETY: `p_cluster` walks the destination scene's skin-cluster pointer
        // list and stops at `p_cluster_end`, so it addresses a live slot holding
        // one of the element copies written into the destination buffer above.
        let cluster: *mut SkinCluster = unsafe { *p_cluster };
        // SAFETY: `bone_node` is that live cluster's own nullable
        // `Option<Ref<UfbxNode>>` field, which `opt_ptr` reads as the element
        // pointer it is; the byte copy left it naming a source-scene element —
        // `translate_element`'s contract — and the result is written back in
        // place.
        unsafe {
            *(&raw mut (*cluster).bone_node as *mut *mut UfbxNode) =
                translate_element(ec, opt_ptr(&raw const (*cluster).bone_node) as *mut c_void)
                    as *mut UfbxNode;
        }
        // SAFETY: `p_cluster` is inside the list, so `p_cluster + 1` is at most
        // one past its end.
        p_cluster = unsafe { p_cluster.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_blend_deformer, p_blend, ec->scene.blend_deformers)`
    let mut p_blend: *mut *mut BlendDeformer =
        ec.scene_view().blend_deformers_view().data() as *mut *mut BlendDeformer;
    let p_blend_end: *mut *mut BlendDeformer =
        add_ptr(p_blend, ec.scene_view().blend_deformers_view().count());
    while p_blend != p_blend_end {
        // SAFETY: `p_blend` walks the destination scene's blend-deformer pointer
        // list and stops at `p_blend_end`, so it addresses a live slot holding
        // one of the element copies written into the destination buffer above.
        let blend: *mut BlendDeformer = unsafe { *p_blend };
        // SAFETY: `channels` is that live deformer's own `ufbx_element_list`,
        // whose byte copy still lists source-scene elements —
        // `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, &raw mut (*blend).channels as *mut c_void) }?;
        // SAFETY: `p_blend` is inside the list, so `p_blend + 1` is at most one
        // past its end.
        p_blend = unsafe { p_blend.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_blend_channel, p_chan, ec->scene.blend_channels)`
    let mut p_chan: *mut *mut BlendChannel =
        ec.scene_view().blend_channels_view().data() as *mut *mut BlendChannel;
    let p_chan_end: *mut *mut BlendChannel =
        add_ptr(p_chan, ec.scene_view().blend_channels_view().count());
    while p_chan != p_chan_end {
        // SAFETY: `p_chan` walks the destination scene's blend-channel pointer
        // list and stops at `p_chan_end`, so it addresses a live slot holding one
        // of the element copies written into the destination buffer above.
        let chan: *mut BlendChannel = unsafe { *p_chan };

        // SAFETY: `keyframes` is that live channel's own list, whose byte copy
        // still describes the source scene's keyframe run.
        let keys: *mut BlendKeyframe = ec
            .result_view()
            .push::<BlendKeyframe>(unsafe { (*chan).keyframes.count });
        ufbxi_check_err!(ec.error_view(), !keys.is_null(), "keys");
        // SAFETY: as above, reading the same live channel's keyframe count.
        for i in 0..unsafe { (*chan).keyframes.count } {
            // C: `keys[i] = chan->keyframes.data[i];` (struct assignment)
            // SAFETY: `i` is below that count, so slot `i` is in bounds of the
            // source keyframe run and of the equally sized allocation pushed into
            // `keys` (non-null, checked above); the two are separate allocations.
            unsafe { ptr::copy_nonoverlapping((*chan).keyframes.data.add(i), keys.add(i), 1) };
            // SAFETY: `keys[i]` holds the copy made just above, whose `shape` is a
            // non-null `Ref` to a source-scene element — `translate_element`'s
            // contract — read through `ref_ptr` and written back in place.
            unsafe {
                *(&raw mut (*keys.add(i)).shape as *mut *mut BlendShape) =
                    translate_element(ec, ref_ptr(&(*keys.add(i)).shape) as *mut c_void)
                        as *mut BlendShape;
            }
        }
        // SAFETY: `chan` is the live destination channel, retargeted at the
        // translated keyframe array.
        unsafe { (*chan).keyframes.data = keys };
        // SAFETY: `target_shape` is that live channel's own nullable
        // `Option<Ref<BlendShape>>` field, which `opt_ptr` reads as the element
        // pointer it is; the byte copy left it naming a source-scene element —
        // `translate_element`'s contract — and the result is written back in
        // place.
        unsafe {
            *(&raw mut (*chan).target_shape as *mut *mut BlendShape) =
                translate_element(ec, opt_ptr(&raw const (*chan).target_shape) as *mut c_void)
                    as *mut BlendShape;
        }
        // SAFETY: `p_chan` is inside the list, so `p_chan + 1` is at most one
        // past its end.
        p_chan = unsafe { p_chan.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_cache_deformer, p_deformer, ec->scene.cache_deformers)`
    let mut p_deformer: *mut *mut CacheDeformer =
        ec.scene_view().cache_deformers_view().data() as *mut *mut CacheDeformer;
    let p_deformer_end: *mut *mut CacheDeformer =
        add_ptr(p_deformer, ec.scene_view().cache_deformers_view().count());
    while p_deformer != p_deformer_end {
        // SAFETY: `p_deformer` walks the destination scene's cache-deformer
        // pointer list and stops at `p_deformer_end`, so it addresses a live slot
        // holding one of the element copies written into the destination buffer
        // above.
        let deformer: *mut CacheDeformer = unsafe { *p_deformer };
        // SAFETY: `file` is that live deformer's own nullable
        // `Option<Ref<CacheFile>>` field, which `opt_ptr` reads as the element
        // pointer it is; the byte copy left it naming a source-scene element —
        // `translate_element`'s contract — and the result is written back in
        // place.
        unsafe {
            *(&raw mut (*deformer).file as *mut *mut CacheFile) =
                translate_element(ec, opt_ptr(&raw const (*deformer).file) as *mut c_void)
                    as *mut CacheFile;
        }
        // SAFETY: `p_deformer` is inside the list, so `p_deformer + 1` is at most
        // one past its end.
        p_deformer = unsafe { p_deformer.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_material, p_material, ec->scene.materials)`
    let mut p_material: *mut *mut Material =
        ec.scene_view().materials_view().data() as *mut *mut Material;
    let p_material_end: *mut *mut Material =
        add_ptr(p_material, ec.scene_view().materials_view().count());
    while p_material != p_material_end {
        // SAFETY: `p_material` walks the destination scene's material pointer
        // list and stops at `p_material_end`, so it addresses a live slot holding
        // one of the element copies written into the destination buffer above.
        let material: *mut Material = unsafe { *p_material };

        // SAFETY: `shader` is that live material's own nullable
        // `Option<Ref<Shader>>` field, which `opt_ptr` reads as the element
        // pointer it is; the byte copy left it naming a source-scene element —
        // `translate_element`'s contract — and the result is written back in
        // place.
        unsafe {
            *(&raw mut (*material).shader as *mut *mut Shader) =
                translate_element(ec, opt_ptr(&raw const (*material).shader) as *mut c_void)
                    as *mut Shader;
        }
        // C: `material->fbx.maps` / `material->pbr.maps` — the flat `maps[]`
        // union view; the generated struct keeps only the named branch, whose
        // base is the aggregate itself (layout pinned in `native::scene_process`).
        // SAFETY: that aggregate is `MATERIAL_FBX_MAP_COUNT` live
        // `ufbx_material_map`s of the live destination material, still naming
        // source-scene textures — `translate_maps`' contract.
        unsafe {
            translate_maps(
                ec,
                &raw mut (*material).fbx as *mut MaterialMap,
                MATERIAL_FBX_MAP_COUNT,
            );
        }
        // SAFETY: as above, for the `MATERIAL_PBR_MAP_COUNT`-long `pbr` aggregate.
        unsafe {
            translate_maps(
                ec,
                &raw mut (*material).pbr as *mut MaterialMap,
                MATERIAL_PBR_MAP_COUNT,
            );
        }

        // SAFETY: `textures` is that live material's own list, whose byte copy
        // still describes the source scene's material-texture run.
        let textures: *mut MaterialTexture = ec
            .result_view()
            .push::<MaterialTexture>(unsafe { (*material).textures.count });
        ufbxi_check_err!(ec.error_view(), !textures.is_null(), "textures");
        // SAFETY: as above, reading the same live material's texture count.
        for i in 0..unsafe { (*material).textures.count } {
            // C: `textures[i] = material->textures.data[i];` (struct assignment)
            // SAFETY: `i` is below that count, so slot `i` is in bounds of the
            // source run and of the equally sized allocation pushed into
            // `textures` (non-null, checked above); the two are separate
            // allocations.
            unsafe {
                ptr::copy_nonoverlapping((*material).textures.data.add(i), textures.add(i), 1);
            }
            // SAFETY: `textures[i]` holds the copy made just above, whose
            // `texture` is a non-null `Ref` to a source-scene element —
            // `translate_element`'s contract — read through `ref_ptr` and written
            // back in place.
            unsafe {
                *(&raw mut (*textures.add(i)).texture as *mut *mut Texture) =
                    translate_element(ec, ref_ptr(&(*textures.add(i)).texture) as *mut c_void)
                        as *mut Texture;
            }
        }
        // SAFETY: `material` is the live destination material, retargeted at the
        // translated texture array.
        unsafe { (*material).textures.data = textures };
        // SAFETY: `p_material` is inside the list, so `p_material + 1` is at most
        // one past its end.
        p_material = unsafe { p_material.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, ec->scene.textures)`
    let mut p_texture: *mut *mut Texture =
        ec.scene_view().textures_view().data() as *mut *mut Texture;
    let p_texture_end: *mut *mut Texture =
        add_ptr(p_texture, ec.scene_view().textures_view().count());
    while p_texture != p_texture_end {
        // SAFETY: `p_texture` walks the destination scene's texture pointer list
        // and stops at `p_texture_end`, so it addresses a live slot holding one
        // of the element copies written into the destination buffer above.
        let texture: *mut Texture = unsafe { *p_texture };
        // SAFETY: `video` is that live texture's own nullable
        // `Option<Ref<Video>>` field, which `opt_ptr` reads as the element
        // pointer it is; the byte copy left it naming a source-scene element —
        // `translate_element`'s contract — and the result is written back in
        // place.
        unsafe {
            *(&raw mut (*texture).video as *mut *mut Video) =
                translate_element(ec, opt_ptr(&raw const (*texture).video) as *mut c_void)
                    as *mut Video;
        }

        // SAFETY: `layers` is that live texture's own list, whose byte copy still
        // describes the source scene's texture-layer run.
        let layers: *mut TextureLayer = ec
            .result_view()
            .push::<TextureLayer>(unsafe { (*texture).layers.count });
        ufbxi_check_err!(ec.error_view(), !layers.is_null(), "layers");
        // SAFETY: as above, reading the same live texture's layer count.
        for i in 0..unsafe { (*texture).layers.count } {
            // C: `layers[i] = texture->layers.data[i];` (struct assignment)
            // SAFETY: `i` is below that count, so slot `i` is in bounds of the
            // source run and of the equally sized allocation pushed into `layers`
            // (non-null, checked above); the two are separate allocations.
            unsafe { ptr::copy_nonoverlapping((*texture).layers.data.add(i), layers.add(i), 1) };
            // SAFETY: `layers[i]` holds the copy made just above, whose `texture`
            // is a non-null `Ref` to a source-scene element —
            // `translate_element`'s contract — read through `ref_ptr` and written
            // back in place.
            unsafe {
                *(&raw mut (*layers.add(i)).texture as *mut *mut Texture) =
                    translate_element(ec, ref_ptr(&(*layers.add(i)).texture) as *mut c_void)
                        as *mut Texture;
            }
        }
        // SAFETY: `texture` is the live destination texture, retargeted at the
        // translated layer array.
        unsafe { (*texture).layers.data = layers };

        // SAFETY: `file_textures` is that live texture's own `ufbx_element_list`,
        // whose byte copy still lists source-scene elements —
        // `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, &raw mut (*texture).file_textures as *mut c_void) }?;

        // C: `if (texture->shader) { ... }`
        // SAFETY: `shader` is the live texture's own nullable
        // `Option<Ref<ShaderTexture>>` field, which `opt_ptr` reads as the
        // pointer it is.
        if !unsafe { opt_ptr(&raw const (*texture).shader) }.is_null() {
            // SAFETY: as above.
            let mut shader: *mut ShaderTexture = unsafe { opt_ptr(&raw const (*texture).shader) };
            // SAFETY: `ec.result_mut_ptr()` is the eval context's own result
            // buffer and `shader` is the non-null (checked just above)
            // source-scene shader texture `push_copy` copies.
            shader = unsafe { push_copy::<ShaderTexture>(ec.result_mut_ptr(), 1, shader) };
            ufbxi_check_err!(ec.error_view(), !shader.is_null(), "shader");
            // SAFETY: `texture` is the live destination texture, retargeted at the
            // copy just pushed.
            unsafe { *(&raw mut (*texture).shader as *mut *mut ShaderTexture) = shader };

            // SAFETY: `shader` is the non-null (checked just above) freshly
            // pushed copy, so its `inputs` list still describes the source
            // scene's input run — the count/data pair `push_copy` copies from.
            let inputs: *mut ShaderTextureInput = unsafe {
                push_copy::<ShaderTextureInput>(
                    ec.result_mut_ptr(),
                    (*shader).inputs.count,
                    (*shader).inputs.data,
                )
            };
            ufbxi_check_err!(ec.error_view(), !inputs.is_null(), "inputs");
            // SAFETY: `shader` is that live copy, retargeted at the pushed input
            // array.
            unsafe { (*shader).inputs.data = inputs };
        }
        // SAFETY: `p_texture` is inside the list, so `p_texture + 1` is at most
        // one past its end.
        p_texture = unsafe { p_texture.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_shader, p_shader, ec->scene.shaders)`
    let mut p_shader: *mut *mut Shader = ec.scene_view().shaders_view().data() as *mut *mut Shader;
    let p_shader_end: *mut *mut Shader = add_ptr(p_shader, ec.scene_view().shaders_view().count());
    while p_shader != p_shader_end {
        // SAFETY: `p_shader` walks the destination scene's shader pointer list
        // and stops at `p_shader_end`, so it addresses a live slot holding one of
        // the element copies written into the destination buffer above.
        let shader: *mut Shader = unsafe { *p_shader };
        // SAFETY: `bindings` is that live shader's own `ufbx_element_list`, whose
        // byte copy still lists source-scene elements —
        // `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, &raw mut (*shader).bindings as *mut c_void) }?;
        // SAFETY: `p_shader` is inside the list, so `p_shader + 1` is at most one
        // past its end.
        p_shader = unsafe { p_shader.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_display_layer, p_layer, ec->scene.display_layers)`
    let mut p_layer: *mut *mut DisplayLayer =
        ec.scene_view().display_layers_view().data() as *mut *mut DisplayLayer;
    let p_layer_end: *mut *mut DisplayLayer =
        add_ptr(p_layer, ec.scene_view().display_layers_view().count());
    while p_layer != p_layer_end {
        // SAFETY: `p_layer` walks the destination scene's display-layer pointer
        // list and stops at `p_layer_end`, so it addresses a live slot holding
        // one of the element copies written into the destination buffer above.
        let layer: *mut DisplayLayer = unsafe { *p_layer };

        // SAFETY: `nodes` is that live layer's own `ufbx_element_list`, whose
        // byte copy still lists source-scene elements —
        // `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, &raw mut (*layer).nodes as *mut c_void) }?;
        // SAFETY: `p_layer` is inside the list, so `p_layer + 1` is at most one
        // past its end.
        p_layer = unsafe { p_layer.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_selection_set, p_set, ec->scene.selection_sets)`
    let mut p_set: *mut *mut SelectionSet =
        ec.scene_view().selection_sets_view().data() as *mut *mut SelectionSet;
    let p_set_end: *mut *mut SelectionSet =
        add_ptr(p_set, ec.scene_view().selection_sets_view().count());
    while p_set != p_set_end {
        // SAFETY: `p_set` walks the destination scene's selection-set pointer
        // list and stops at `p_set_end`, so it addresses a live slot holding one
        // of the element copies written into the destination buffer above.
        let set: *mut SelectionSet = unsafe { *p_set };

        // SAFETY: `nodes` is that live set's own `ufbx_element_list`, whose byte
        // copy still lists source-scene elements — `translate_element_list`'s
        // contract.
        unsafe { translate_element_list(ec, &raw mut (*set).nodes as *mut c_void) }?;
        // SAFETY: `p_set` is inside the list, so `p_set + 1` is at most one past
        // its end.
        p_set = unsafe { p_set.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_selection_node, p_node, ec->scene.selection_nodes)`
    let mut p_sel_node: *mut *mut SelectionNode =
        ec.scene_view().selection_nodes_view().data() as *mut *mut SelectionNode;
    let p_sel_node_end: *mut *mut SelectionNode =
        add_ptr(p_sel_node, ec.scene_view().selection_nodes_view().count());
    while p_sel_node != p_sel_node_end {
        // SAFETY: `p_sel_node` walks the destination scene's selection-node
        // pointer list and stops at `p_sel_node_end`, so it addresses a live slot
        // holding one of the element copies written into the destination buffer
        // above.
        let node: *mut SelectionNode = unsafe { *p_sel_node };

        // SAFETY (both stores): each named field is that live selection node's
        // own nullable `Option<Ref<..>>`, which `opt_ptr` reads as the element
        // pointer it is; the byte copy left it naming a source-scene element —
        // `translate_element`'s contract — and the result is written back in
        // place.
        unsafe {
            *(&raw mut (*node).target_node as *mut *mut UfbxNode) =
                translate_element(ec, opt_ptr(&raw const (*node).target_node) as *mut c_void)
                    as *mut UfbxNode;
        }
        // SAFETY: as above.
        unsafe {
            *(&raw mut (*node).target_mesh as *mut *mut Mesh) =
                translate_element(ec, opt_ptr(&raw const (*node).target_mesh) as *mut c_void)
                    as *mut Mesh;
        }
        // SAFETY: `p_sel_node` is inside the list, so `p_sel_node + 1` is at most
        // one past its end.
        p_sel_node = unsafe { p_sel_node.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_constraint, p_constraint, ec->scene.constraints)`
    let mut p_constraint: *mut *mut Constraint =
        ec.scene_view().constraints_view().data() as *mut *mut Constraint;
    let p_constraint_end: *mut *mut Constraint =
        add_ptr(p_constraint, ec.scene_view().constraints_view().count());
    while p_constraint != p_constraint_end {
        // SAFETY: `p_constraint` walks the destination scene's constraint pointer
        // list and stops at `p_constraint_end`, so it addresses a live slot
        // holding one of the element copies written into the destination buffer
        // above.
        let constraint: *mut Constraint = unsafe { *p_constraint };

        // SAFETY (this store and the three below it): each named field is that
        // live constraint's own nullable `Option<Ref<UfbxNode>>`, which `opt_ptr`
        // reads as the element pointer it is; the byte copy left it naming a
        // source-scene element — `translate_element`'s contract — and the result
        // is written back in place.
        unsafe {
            *(&raw mut (*constraint).node as *mut *mut UfbxNode) =
                translate_element(ec, opt_ptr(&raw const (*constraint).node) as *mut c_void)
                    as *mut UfbxNode;
        }
        // SAFETY: as above.
        unsafe {
            *(&raw mut (*constraint).aim_up_node as *mut *mut UfbxNode) = translate_element(
                ec,
                opt_ptr(&raw const (*constraint).aim_up_node) as *mut c_void,
            )
                as *mut UfbxNode;
        }
        // SAFETY: as above.
        unsafe {
            *(&raw mut (*constraint).ik_effector as *mut *mut UfbxNode) = translate_element(
                ec,
                opt_ptr(&raw const (*constraint).ik_effector) as *mut c_void,
            )
                as *mut UfbxNode;
        }
        // SAFETY: as above.
        unsafe {
            *(&raw mut (*constraint).ik_end_node as *mut *mut UfbxNode) = translate_element(
                ec,
                opt_ptr(&raw const (*constraint).ik_end_node) as *mut c_void,
            )
                as *mut UfbxNode;
        }

        // SAFETY: `targets` is that live constraint's own list, whose byte copy
        // still describes the source scene's target run.
        let targets: *mut ConstraintTarget = ec
            .result_view()
            .push::<ConstraintTarget>(unsafe { (*constraint).targets.count });
        ufbxi_check_err!(ec.error_view(), !targets.is_null(), "targets");
        // SAFETY: as above, reading the same live constraint's target count.
        for i in 0..unsafe { (*constraint).targets.count } {
            // C: `targets[i] = constraint->targets.data[i];` (struct assignment)
            // SAFETY: `i` is below that count, so slot `i` is in bounds of the
            // source run and of the equally sized allocation pushed into
            // `targets` (non-null, checked above); the two are separate
            // allocations.
            unsafe {
                ptr::copy_nonoverlapping((*constraint).targets.data.add(i), targets.add(i), 1);
            }
            // SAFETY: `targets[i]` holds the copy made just above, whose `node` is
            // a non-null `Ref` to a source-scene element — `translate_element`'s
            // contract — read through `ref_ptr` and written back in place.
            unsafe {
                *(&raw mut (*targets.add(i)).node as *mut *mut UfbxNode) =
                    translate_element(ec, ref_ptr(&(*targets.add(i)).node) as *mut c_void)
                        as *mut UfbxNode;
            }
        }
        // SAFETY: `constraint` is the live destination constraint, retargeted at
        // the translated target array.
        unsafe { (*constraint).targets.data = targets };
        // SAFETY: `p_constraint` is inside the list, so `p_constraint + 1` is at
        // most one past its end.
        p_constraint = unsafe { p_constraint.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_audio_layer, p_layer, ec->scene.audio_layers)`
    let mut p_audio_layer: *mut *mut AudioLayer =
        ec.scene_view().audio_layers_view().data() as *mut *mut AudioLayer;
    let p_audio_layer_end: *mut *mut AudioLayer =
        add_ptr(p_audio_layer, ec.scene_view().audio_layers_view().count());
    while p_audio_layer != p_audio_layer_end {
        // SAFETY: `p_audio_layer` walks the destination scene's audio-layer
        // pointer list and stops at `p_audio_layer_end`, so it addresses a live
        // slot holding one of the element copies written into the destination
        // buffer above.
        let layer: *mut AudioLayer = unsafe { *p_audio_layer };

        // SAFETY: `clips` is that live layer's own `ufbx_element_list`, whose
        // byte copy still lists source-scene elements —
        // `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, &raw mut (*layer).clips as *mut c_void) }?;
        // SAFETY: `p_audio_layer` is inside the list, so `p_audio_layer + 1` is
        // at most one past its end.
        p_audio_layer = unsafe { p_audio_layer.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_stack, p_stack, ec->scene.anim_stacks)`
    let mut p_stack: *mut *mut AnimStack =
        ec.scene_view().anim_stacks_view().data() as *mut *mut AnimStack;
    let p_stack_end: *mut *mut AnimStack =
        add_ptr(p_stack, ec.scene_view().anim_stacks_view().count());
    while p_stack != p_stack_end {
        // SAFETY: `p_stack` walks the destination scene's anim-stack pointer list
        // and stops at `p_stack_end`, so it addresses a live slot holding one of
        // the element copies written into the destination buffer above.
        let stack: *mut AnimStack = unsafe { *p_stack };

        // SAFETY: `layers` is that live stack's own `ufbx_element_list`, whose
        // byte copy still lists source-scene elements —
        // `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, &raw mut (*stack).layers as *mut c_void) }?;
        // SAFETY: `anim` is that live stack's own `ufbx_anim*` field, so the
        // pointer addresses a live slot — `translate_anim`'s contract.
        unsafe { translate_anim(ec, &raw mut (*stack).anim as *mut *mut Anim) }?;
        // SAFETY: `p_stack` is inside the list, so `p_stack + 1` is at most one
        // past its end.
        p_stack = unsafe { p_stack.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_layer, p_layer, ec->scene.anim_layers)`
    let mut p_anim_layer: *mut *mut AnimLayer =
        ec.scene_view().anim_layers_view().data() as *mut *mut AnimLayer;
    let p_anim_layer_end: *mut *mut AnimLayer =
        add_ptr(p_anim_layer, ec.scene_view().anim_layers_view().count());
    while p_anim_layer != p_anim_layer_end {
        // SAFETY: `p_anim_layer` walks the destination scene's anim-layer pointer
        // list and stops at `p_anim_layer_end`, so it addresses a live slot
        // holding one of the element copies written into the destination buffer
        // above.
        let layer: *mut AnimLayer = unsafe { *p_anim_layer };

        // SAFETY: `anim_values` is that live layer's own `ufbx_element_list`,
        // whose byte copy still lists source-scene elements —
        // `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, &raw mut (*layer).anim_values as *mut c_void) }?;
        // SAFETY: `anim_props` is that live layer's own list, whose byte copy
        // still describes the source scene's anim-prop run.
        let props: *mut AnimProp = ec
            .result_view()
            .push::<AnimProp>(unsafe { (*layer).anim_props.count } + 1);
        ufbxi_check_err!(ec.error_view(), !props.is_null(), "props");
        // SAFETY: as above, reading the same live layer's anim-prop count.
        for i in 0..unsafe { (*layer).anim_props.count } {
            // C: `props[i] = layer->anim_props.data[i];` (struct assignment)
            // SAFETY: `i` is below that count, so slot `i` is in bounds of the
            // source run and of the `count + 1`-element allocation pushed into
            // `props` (non-null, checked above); the two are separate
            // allocations.
            unsafe { ptr::copy_nonoverlapping((*layer).anim_props.data.add(i), props.add(i), 1) };
            // SAFETY: `props[i]` holds the copy made just above, whose `element`
            // is a non-null `Ref` to a source-scene element —
            // `translate_element`'s contract — read through `ref_ptr` and written
            // back in place.
            unsafe {
                *(&raw mut (*props.add(i)).element as *mut *mut Element) =
                    translate_element(ec, ref_ptr(&(*props.add(i)).element) as *mut c_void);
            }
            // SAFETY: as above, for that copy's `anim_value`.
            unsafe {
                *(&raw mut (*props.add(i)).anim_value as *mut *mut AnimValue) =
                    translate_element(ec, ref_ptr(&(*props.add(i)).anim_value) as *mut c_void)
                        as *mut AnimValue;
            }
        }
        // Maintain NULL sentinel
        // SAFETY: `props` was pushed with `anim_props.count + 1` elements, so the
        // slot at index `count` is the spare one in bounds, and one `AnimProp`
        // worth of bytes fits in it.
        unsafe {
            ptr::write_bytes(
                props.add((*layer).anim_props.count) as *mut u8,
                0,
                size_of::<AnimProp>(),
            );
        }
        // SAFETY: `layer` is the live destination layer, retargeted at the
        // translated anim-prop array.
        unsafe { (*layer).anim_props.data = props };
        // SAFETY: `p_anim_layer` is inside the list, so `p_anim_layer + 1` is at
        // most one past its end.
        p_anim_layer = unsafe { p_anim_layer.add(1) };
    }

    // C: `ufbxi_for_ptr_list(ufbx_pose, p_pose, ec->scene.poses)`
    let mut p_pose: *mut *mut Pose = ec.scene_view().poses_view().data() as *mut *mut Pose;
    let p_pose_end: *mut *mut Pose = add_ptr(p_pose, ec.scene_view().poses_view().count());
    while p_pose != p_pose_end {
        // SAFETY: `p_pose` walks the destination scene's pose pointer list and
        // stops at `p_pose_end`, so it addresses a live slot holding one of the
        // element copies written into the destination buffer above.
        let pose: *mut Pose = unsafe { *p_pose };

        // SAFETY: `bone_poses` is that live pose's own list, whose byte copy
        // still describes the source scene's bone-pose run.
        let bones: *mut BonePose = ec
            .result_view()
            .push::<BonePose>(unsafe { (*pose).bone_poses.count });
        ufbxi_check_err!(ec.error_view(), !bones.is_null(), "bones");
        // SAFETY: as above, reading the same live pose's bone-pose count.
        for i in 0..unsafe { (*pose).bone_poses.count } {
            // C: `bones[i] = pose->bone_poses.data[i];` (struct assignment)
            // SAFETY: `i` is below that count, so slot `i` is in bounds of the
            // source run and of the equally sized allocation pushed into `bones`
            // (non-null, checked above); the two are separate allocations.
            unsafe { ptr::copy_nonoverlapping((*pose).bone_poses.data.add(i), bones.add(i), 1) };
            // SAFETY: `bones[i]` holds the copy made just above, whose `bone_node`
            // is a non-null `Ref` to a source-scene element —
            // `translate_element`'s contract — read through `ref_ptr` and written
            // back in place.
            unsafe {
                *(&raw mut (*bones.add(i)).bone_node as *mut *mut UfbxNode) =
                    translate_element(ec, ref_ptr(&(*bones.add(i)).bone_node) as *mut c_void)
                        as *mut UfbxNode;
            }
        }
        // SAFETY: `pose` is the live destination pose, retargeted at the
        // translated bone-pose array.
        unsafe { (*pose).bone_poses.data = bones };
        // SAFETY: `p_pose` is inside the list, so `p_pose + 1` is at most one
        // past its end.
        p_pose = unsafe { p_pose.add(1) };
    }

    // SAFETY: `anim` is a field of `ec`'s own live context struct, so the pointer
    // addresses a live `ufbx_anim*` slot — `translate_anim`'s contract.
    unsafe { translate_anim(ec, ec.anim_mut_ptr()) }?;

    // C: `ufbxi_for_ptr_list(ufbx_anim_value, p_value, ec->scene.anim_values)`
    let mut p_value: *mut *mut AnimValue =
        ec.scene_view().anim_values_view().data() as *mut *mut AnimValue;
    let p_value_end: *mut *mut AnimValue =
        add_ptr(p_value, ec.scene_view().anim_values_view().count());
    while p_value != p_value_end {
        // SAFETY: `p_value` walks the destination scene's anim-value pointer list
        // and stops at `p_value_end`, so it addresses a live slot holding one of
        // the element copies written into the destination buffer above.
        let value: *mut AnimValue = unsafe { *p_value };
        // SAFETY (these three stores): `curves` is that live anim value's own
        // fixed three-element array of nullable `Option<Ref<AnimCurve>>`, so each
        // index is in bounds and `opt_ptr` reads it as the element pointer it is;
        // the byte copy left it naming a source-scene element —
        // `translate_element`'s contract — and the result is written back in
        // place.
        unsafe {
            *(&raw mut (*value).curves[0] as *mut *mut AnimCurve) =
                translate_element(ec, opt_ptr(&raw const (*value).curves[0]) as *mut c_void)
                    as *mut AnimCurve;
        }
        // SAFETY: as above.
        unsafe {
            *(&raw mut (*value).curves[1] as *mut *mut AnimCurve) =
                translate_element(ec, opt_ptr(&raw const (*value).curves[1]) as *mut c_void)
                    as *mut AnimCurve;
        }
        // SAFETY: as above.
        unsafe {
            *(&raw mut (*value).curves[2] as *mut *mut AnimCurve) =
                translate_element(ec, opt_ptr(&raw const (*value).curves[2]) as *mut c_void)
                    as *mut AnimCurve;
        }
        // SAFETY: `p_value` is inside the list, so `p_value + 1` is at most one
        // past its end.
        p_value = unsafe { p_value.add(1) };
    }

    // C: `ufbx_anim anim = *ec->anim;` — local working copy (memcpy).
    // SAFETY: `ec.anim()` is the anim `translate_anim` just retargeted at a copy
    // pushed into the result buffer, so it addresses a live `ufbx_anim`; the read
    // takes a bitwise copy the loop below only reads and re-points, leaving the
    // pushed original as the sole owner of everything it names.
    let mut anim: Anim = unsafe { ptr::read(ec.anim()) };
    let mut over: *const PropOverride = anim.prop_overrides.data;
    let over_end: *const PropOverride =
        add_ptr(over as *mut PropOverride, anim.prop_overrides.count);

    // Evaluate the properties
    // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, ec->scene.elements)`
    let mut p_elem: *mut *mut Element = ec.scene_view().elements_view().data() as *mut *mut Element;
    let p_elem_end: *mut *mut Element = add_ptr(p_elem, ec.scene_view().elements_view().count());
    while p_elem != p_elem_end {
        // SAFETY: `p_elem` walks the destination scene's element list and stops
        // at `p_elem_end`, so it addresses a live slot holding one of the element
        // copies written into the destination buffer above.
        let elem: *mut Element = unsafe { *p_elem };
        // SAFETY: `elem` is that live destination element.
        let mut num_animated: usize = unsafe { (*elem).props.num_animated };
        let mut num_override: usize = 0;

        // Setup the overrides for this element if found
        // SAFETY (this condition): `over` is only read when it has not reached
        // `over_end`, so it addresses a live entry of the anim's override run;
        // `elem` is the live destination element.
        while over != over_end && unsafe { (*over).element_id } == unsafe { (*elem).element_id } {
            num_override += 1;
            // SAFETY: `over` is inside the override run, so `over + 1` is at most
            // one past its end.
            over = unsafe { over.add(1) };
        }

        num_animated += num_override;
        if num_animated == 0 {
            // SAFETY: `p_elem` is inside the element list, so `p_elem + 1` is at
            // most one past its end.
            p_elem = unsafe { p_elem.add(1) };
            continue;
        }

        // C: `anim.prop_overrides.data = ufbxi_sub_ptr(over, num_override);`
        // SAFETY: the loop above advanced `over` by exactly `num_override` steps
        // from a position inside the override run, so stepping back that far
        // stays in bounds of the same run.
        anim.prop_overrides.data = unsafe { over.sub(num_override) };
        anim.prop_overrides.count = num_override;

        let props: *mut Prop = ec.result_view().push::<Prop>(num_animated);
        ufbxi_check_err!(ec.error_view(), !props.is_null(), "props");

        // C: `elem->props = ufbx_evaluate_props_flags(...)` — struct assignment.
        // SAFETY: `elem` is the live destination element, `anim` names the
        // override sub-run set just above, and `props` is the non-null (checked
        // just above) `num_animated`-element scratch array the evaluation fills —
        // `ufbx_evaluate_props_flags`' contract.
        let new_props: crate::generated::Props = unsafe {
            evaluate_props_flags(
                &anim,
                elem,
                ec.time(),
                props,
                num_animated,
                ec.opts_view().evaluate_flags(),
            )
        };
        // SAFETY: `elem` is the live destination element, so its `props` field is
        // a valid initialized `ufbx_props` the write overwrites in place.
        unsafe { ptr::write(&raw mut (*elem).props, new_props) };
        // C: `elem->props.defaults = &ec->src_scene.elements.data[elem->element_id]->props;`
        // SAFETY: `elem` is live, and per the source-scene premise its
        // `element_id` indexes the source element list, whose slot holds the
        // source-scene element this one was copied from — so its `props` field
        // outlives the destination scene it is stored into.
        unsafe {
            *(&raw mut (*elem).props.defaults as *mut *const crate::generated::Props) =
                &raw const (*(*(ec.src_scene_view().elements_view().data() as *mut *mut Element)
                    .add((*elem).element_id as usize)))
                .props;
        }
        // SAFETY: `p_elem` is inside the element list, so `p_elem + 1` is at most
        // one past its end.
        p_elem = unsafe { p_elem.add(1) };
    }

    // Update all derived values
    // SAFETY: `ec.scene_view()` is the destination scene inside `ec`, fully
    // translated by the loops above, and the transform-override run is the
    // count/data pair of the local `anim` copy — `update_scene`'s contract.
    unsafe {
        update_scene(
            ec.scene_view(),
            false,
            anim.transform_overrides.data,
            anim.transform_overrides.count,
        );
    }

    // Evaluate skinning if requested
    if ec.opts_view().evaluate_skinning() {
        // C: `ufbx_geometry_cache_data_opts cache_opts = { 0 };`
        // SAFETY: `ufbx_geometry_cache_data_opts` is a plain options struct of
        // scalars, enums with a zero discriminant and nullable callback pointers,
        // for which an all-zero bit pattern is a valid value.
        let mut cache_opts: RawGeometryCacheDataOpts =
            unsafe { MaybeUninit::zeroed().assume_init() };
        // C: `cache_opts.open_file_cb = ec->opts.open_file_cb;` (struct assignment)
        // SAFETY: the source is the `open_file_cb` field of `ec`'s own live
        // options struct and the destination is the field of the local
        // `cache_opts` just initialized — two live, disjoint one-element regions.
        unsafe {
            ptr::copy_nonoverlapping(
                ec.opts_view().open_file_cb_ptr(),
                &raw mut cache_opts.open_file_cb,
                1,
            );
        }
        // SAFETY: the scene, error and time all come from `ec`'s own live context
        // struct, the scene being the fully translated destination scene, and
        // `cache_opts` is the live local just filled in — `evaluate_skinning`'s
        // contract.
        unsafe {
            evaluate_skinning(
                ec.scene_mut_ptr(),
                ec.error_mut_ptr(),
                ec.result_view(),
                ec.tmp_view(),
                ec.time(),
                ec.opts_view().load_external_files() && ec.opts_view().evaluate_caches(),
                &mut cache_opts,
            )
        }?;
    }

    // Retain the scene, this must be the final allocation as we copy
    // `ator_result` to `ufbx_scene_imp`
    let imp: *mut SceneImp = ec.result_view().push_zero::<SceneImp>(1);
    ufbxi_check_err!(ec.error_view(), !imp.is_null(), "imp");

    // Expose the wide allocation so `get_imp` can recover this header from a
    // (possibly narrowed) public `&Scene` pointer via exposed provenance.
    (imp as *mut u8).expose_provenance();

    // SAFETY: `ec.src_imp()` is the source scene's own `ufbxi_scene_imp` header,
    // kept alive by the reference this evaluation holds on it.
    ufbx_assert!(unsafe { (*ec.src_imp()).magic } == SCENE_IMP_MAGIC);
    // SAFETY: `imp` is the non-null (checked just above) zeroed header pushed
    // into the result buffer, so its `refcount` is a live uninitialized field for
    // `init_ref` to set up, and the parent is the live source header's own
    // refcount.
    unsafe {
        init_ref(
            &raw mut (*imp).refcount,
            SCENE_IMP_MAGIC,
            &raw mut (*ec.src_imp()).refcount,
        );
    }

    // SAFETY: `imp` is that live pushed header.
    unsafe { (*imp).magic = SCENE_IMP_MAGIC };
    // C: `imp->scene = ec->scene;` (struct assignment)
    // SAFETY: the source is the destination scene inside `ec`'s own context
    // struct and `imp` is the live pushed header, two disjoint one-element
    // `ufbx_scene` regions.
    unsafe { ptr::copy_nonoverlapping(ec.scene_mut_ptr(), &raw mut (*imp).scene, 1) };
    // SAFETY: `imp` is that live pushed header.
    unsafe { (*imp).refcount.ator = ec.ator_result() };
    // SAFETY: as above.
    unsafe { (*imp).refcount.ator.error = ptr::null_mut() };

    // Copy retained buffers and translate the allocator struct to the one
    // contained within `ufbxi_scene_imp`
    // SAFETY: as above.
    unsafe { (*imp).refcount.buf = ec.take_result() };
    // SAFETY: as above — the buffer is retargeted at the allocator embedded in
    // the same header, which the header keeps alive for as long as the buffer.
    unsafe { (*imp).refcount.buf.ator = &raw mut (*imp).refcount.ator };

    // SAFETY (these four stores): `imp` is the live pushed header, whose `scene`
    // and `refcount.ator` were filled in just above.
    unsafe { (*imp).scene.metadata.result_memory_used = (*imp).refcount.ator.current_size };
    // SAFETY: as above.
    unsafe { (*imp).scene.metadata.temp_memory_used = ec.ator_tmp_view().current_size() };
    // SAFETY: as above.
    unsafe { (*imp).scene.metadata.result_allocs = (*imp).refcount.ator.num_allocs };
    // SAFETY: as above.
    unsafe { (*imp).scene.metadata.temp_allocs = ec.ator_tmp_view().num_allocs() };

    // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, imp->scene.elements)`
    // SAFETY: `imp` is the live pushed header holding the copy of the destination
    // scene, whose element list is the array pushed and filled in above.
    let mut p_elem: *mut *mut Element = unsafe { (*imp).scene.elements.data as *mut *mut Element };
    // SAFETY: as above, reading the same list's count.
    let p_elem_end: *mut *mut Element = add_ptr(p_elem, unsafe { (*imp).scene.elements.count });
    while p_elem != p_elem_end {
        // C: `(*p_elem)->scene = &imp->scene;`
        // SAFETY: `p_elem` walks that element list and stops at `p_elem_end`, so
        // it addresses a live slot holding a destination-buffer element; the
        // scene it is pointed at is the header's own copy, which owns that
        // element buffer and so outlives it.
        unsafe { *(&raw mut (*(*p_elem)).scene as *mut *mut Scene) = &raw mut (*imp).scene };
        // SAFETY: `p_elem` is inside the element list, so `p_elem + 1` is at most
        // one past its end.
        p_elem = unsafe { p_elem.add(1) };
    }

    ec.set_scene_imp(imp);
    ec.result_view().set_ator(ec.ator_result_mut_ptr());

    Ok(())
}

// ufbx.c:26446-26483 `ufbxi_evaluate_scene`
#[cfg(feature = "scene-eval")]
#[inline(never)]
pub(crate) unsafe fn evaluate_scene(
    ec: &EvalContext,
    scene: *mut Scene,
    anim: *const Anim,
    time: f64,
    user_opts: *const RawEvaluateOpts,
    p_error: *mut Error,
) -> *mut Scene {
    if !user_opts.is_null() {
        // C: `ec->opts = *user_opts;` (struct assignment)
        // SAFETY: `user_opts` is the caller's live `ufbx_evaluate_opts` (this
        // `unsafe fn`'s contract), non-null on this arm; `ec.opts_mut_ptr()` is
        // `ec`'s own distinct `opts` field, so the two spans do not overlap.
        unsafe { ptr::copy_nonoverlapping(user_opts, ec.opts_mut_ptr(), 1) };
    } else {
        // C: `memset(&ec->opts, 0, sizeof(ec->opts));`
        // SAFETY: `ec.opts_mut_ptr()` addresses `ec`'s own `opts` field, so the
        // whole `RawEvaluateOpts` span it covers is writable.
        unsafe {
            ptr::write_bytes(
                ec.opts_mut_ptr() as *mut u8,
                0,
                size_of::<RawEvaluateOpts>(),
            )
        };
    }

    ec.set_src_imp(get_imp::<SceneImp>(scene as *mut c_void));
    // C: `ec->src_scene = *scene;` (struct assignment)
    // SAFETY: `scene` is the caller's live `ufbx_scene` (this `unsafe fn`'s
    // contract) and `ec.src_scene_mut_ptr()` is `ec`'s own `src_scene` field, a
    // distinct non-overlapping allocation.
    unsafe { ptr::copy_nonoverlapping(scene as *const Scene, ec.src_scene_mut_ptr(), 1) };
    ec.set_anim(if !anim.is_null() {
        anim as *mut Anim
    } else {
        // SAFETY: `scene` is the caller's live scene, whose `anim` field is a
        // non-null `Ref` to the scene's own default animation.
        unsafe { ref_ptr(&(*scene).anim) }
    });
    ec.set_time(time);

    // SAFETY: the error slot, temp allocator and `opts.temp_allocator` are all
    // `ec`'s own fields, live for the `&EvalContext` borrow.
    unsafe {
        init_ator(
            ec.error_mut_ptr(),
            ec.ator_tmp_mut_ptr(),
            ec.opts_view().temp_allocator_ptr(),
            c"temp",
        )
    };
    // SAFETY: as above, for `ec`'s result allocator and `opts.result_allocator`.
    unsafe {
        init_ator(
            ec.error_mut_ptr(),
            ec.ator_result_mut_ptr(),
            ec.opts_view().result_allocator_ptr(),
            c"result",
        )
    };

    ec.result_view().set_ator(ec.ator_result_mut_ptr());
    ec.tmp_view().set_ator(ec.ator_tmp_mut_ptr());

    ec.result_view().set_unordered(true);
    ec.tmp_view().set_unordered(true);

    // SAFETY: `evaluate_imp` takes the same `&EvalContext` this fn was handed,
    // now fully initialized by the setup above.
    if unsafe { evaluate_imp(ec) }.is_ok() {
        // SAFETY: `ec`'s temp buffer and temp allocator are its own fields, live
        // for the borrow, and this is the last use of each.
        unsafe { buf_free(ec.tmp_mut_ptr()) };
        // SAFETY: as above, for `ec`'s temp allocator.
        unsafe { free_ator(ec.ator_tmp_mut_ptr()) };
        if !p_error.is_null() {
            // SAFETY: `p_error` is non-null (checked just above) and points to
            // the caller's `ufbx_error` slot — this `unsafe fn`'s contract.
            unsafe { clear_error(p_error) };
        }
        // SAFETY: `evaluate_imp` succeeded, and its last act is storing the
        // retained `ufbxi_scene_imp` into `ec`, so `scene_imp()` is the live
        // result-buffer header whose own `scene` field is projected here.
        unsafe { &raw mut (*ec.scene_imp()).scene }
    } else {
        // SAFETY: `ec`'s error field is live for the borrow, the message literal
        // is NUL-terminated, and `p_error` is the caller's `ufbx_error` slot or
        // null, which `fix_error_type` accepts.
        unsafe {
            fix_error_type(
                ec.error_mut_ptr(),
                b"Failed to evaluate\0".as_ptr(),
                p_error,
            )
        };
        // SAFETY: `ec`'s temp buffer is its own field, live for the borrow, and
        // this is the last use of it.
        unsafe { buf_free(ec.tmp_mut_ptr()) };
        // SAFETY: as above, for `ec`'s result buffer — the failure path discards
        // it, so this is its last use.
        unsafe { buf_free(ec.result_mut_ptr()) };
        // SAFETY: as above, for `ec`'s temp allocator.
        unsafe { free_ator(ec.ator_tmp_mut_ptr()) };
        // SAFETY: as above, for `ec`'s result allocator.
        unsafe { free_ator(ec.ator_result_mut_ptr()) };
        ptr::null_mut()
    }
}

// C: `#endif` (ufbx.c:26485) — end of the `UFBXI_FEATURE_SCENE_EVALUATION`
// block; the create-anim machinery below is unconditional in C.

// ufbx.c:26487-26496 `ufbxi_create_anim_context`
#[repr(C)]
pub(crate) struct InnerCreateAnimContext {
    pub error: Error,
    pub ator_result: Allocator,
    pub result: Buf,
    pub scene: *const Scene,
    pub opts: RawAnimOpts,

    pub anim: Anim,
    pub imp: *mut AnimImp,
}

// Safe `&CreateAnimContext` handle over `InnerCreateAnimContext`, mirroring the
// `Context`/`InnerContext` seam in `parse.rs`. `MaybeUninit` because it embeds the
// public `Anim` (enum-bearing) in `anim`; `UnsafeCell` gives the interior
// mutability every `&CreateAnimContext` site needs. Field is `pub(crate)` — the
// sole construction site lives in `native::api`.
#[repr(transparent)]
pub(crate) struct CreateAnimContext(
    pub(crate) core::cell::UnsafeCell<core::mem::MaybeUninit<InnerCreateAnimContext>>,
);

// Typed interior-mutable VIEW over `CreateAnimContext.opts` (approach A). Non-Copy
// list fields recurse into `RawListView`; addr-of fields use `_ptr` getters.
pub(crate) type AnimOptsView = crate::native::view::View<RawAnimOpts>;

impl AnimOptsView {
    #[inline(always)]
    pub(crate) fn ignore_connections(&self) -> bool {
        // SAFETY: reading a POD opts field by value.
        unsafe { (*self.get()).ignore_connections }
    }

    #[inline(always)]
    pub(crate) fn layer_ids_view(&self) -> &crate::prelude::RawListView<u32> {
        // SAFETY: reinterpret the non-Copy `RawList` field in place as a view;
        // interior-mutable, asserts no validity.
        unsafe { &*(&raw mut (*self.get()).layer_ids as *mut crate::prelude::RawListView<u32>) }
    }

    #[inline(always)]
    pub(crate) fn override_layer_weights_view(
        &self,
    ) -> &crate::prelude::RawListView<crate::prelude::Real> {
        // SAFETY: reinterpret the non-Copy `RawList` field in place as a view;
        // interior-mutable, asserts no validity.
        unsafe {
            &*(&raw const (*self.get()).override_layer_weights
                as *const crate::prelude::RawListView<crate::prelude::Real>)
        }
    }

    #[inline(always)]
    pub(crate) fn transform_overrides_view(
        &self,
    ) -> &crate::prelude::RawListView<crate::generated::TransformOverride> {
        // SAFETY: reinterpret the non-Copy `RawList` field in place as a view;
        // interior-mutable, asserts no validity.
        unsafe {
            &*(&raw const (*self.get()).transform_overrides
                as *const crate::prelude::RawListView<crate::generated::TransformOverride>)
        }
    }

    #[inline(always)]
    pub(crate) fn prop_overrides_ptr(
        &self,
    ) -> *const crate::prelude::RawList<crate::generated::RawPropOverrideDesc> {
        // SAFETY: `&raw const` address of a read-only list field.
        unsafe { &raw const (*self.get()).prop_overrides }
    }

    #[inline(always)]
    pub(crate) fn result_allocator_ptr(&self) -> *const crate::generated::RawAllocatorOpts {
        // SAFETY: `&raw const` address of a read-only sub-struct.
        unsafe { &raw const (*self.get()).result_allocator }
    }
}

impl CreateAnimContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerCreateAnimContext {
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

    // `result` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn result_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).result as *mut crate::native::buf::BufView) }
    }

    // `opts` — typed VIEW handle (reinterpret-in-place); leaf accessors on `AnimOptsView`.
    #[inline(always)]
    pub(crate) fn opts_view(&self) -> &AnimOptsView {
        // SAFETY: repr(transparent) over the `opts` field inside this context's
        // outer UnsafeCell; shared interior-mutable view, asserts no validity.
        unsafe { &*(&raw mut (*self.get()).opts as *mut AnimOptsView) }
    }

    // `result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn result_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).result }
    }

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawAnimOpts {
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

    // `ator_result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_result_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_result }
    }

    // `anim` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn anim_mut_ptr(&self) -> *mut Anim {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).anim }
    }

    // `imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn imp(&self) -> *mut AnimImp {
        // SAFETY: reading a scalar field; all bit patterns of `*mut AnimImp` are valid.
        unsafe { (*self.get()).imp }
    }

    #[inline(always)]
    pub(crate) fn set_imp(&self, imp: *mut AnimImp) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).imp = imp;
        }
    }

    // `scene` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn scene(&self) -> *const Scene {
        // SAFETY: reading a scalar field; all bit patterns of `*const Scene` are valid.
        unsafe { (*self.get()).scene }
    }

    #[inline(always)]
    pub(crate) fn set_scene(&self, scene: *const Scene) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).scene = scene;
        }
    }
}

// ufbx.c:26498-26510 `ufbxi_check_string`
#[inline(never)]
pub(crate) unsafe fn check_string(
    error: *mut Error,
    dst: *mut String,
    src: *const String,
) -> Result<(), Fail> {
    // SAFETY (each deref): `src` points to the caller's live `ufbx_string` —
    // this `unsafe fn`'s contract. The `SIZE_MAX` length sentinel means the
    // string is NUL-terminated, so `strlen` stays inside it.
    let length: usize = if unsafe { (*src).length } != usize::MAX {
        unsafe { (*src).length }
    } else {
        // SAFETY: as above; `strlen` requires the NUL-terminated buffer the
        // `SIZE_MAX` sentinel promises.
        unsafe { strlen((*src).data) }
    };
    let data: *const u8 = if length != 0 {
        // SAFETY: `src` points to the caller's live `ufbx_string`.
        unsafe { (*src).data }
    } else {
        EMPTY_CHAR.as_ptr()
    };
    if length > 0 {
        // SAFETY: `data` is the source string's own data pointer and `length` is
        // that string's length (or its `strlen`), so the whole span is readable.
        let valid_length: usize = unsafe { utf8_valid_length(data, length) };
        ufbxi_check_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            valid_length == length,
            "Invalid UTF-8"
        );
    }

    // SAFETY: `dst` points to the caller's live `ufbx_string` slot — this
    // `unsafe fn`'s contract — and the pair of writes retargets it at the
    // validated span.
    unsafe {
        (*dst).data = data;
        (*dst).length = length;
    }
    Ok(())
}

// ufbx.c:26512-26526 `ufbxi_push_anim_string`
#[inline(never)]
pub(crate) unsafe fn push_anim_string(
    ac: &CreateAnimContext,
    str_: *mut String,
) -> Result<(), Fail> {
    // SAFETY: `str_` points to a live `ufbx_string` slot — this `unsafe fn`'s
    // contract.
    let length: usize = unsafe { (*str_).length };
    if length > 0 {
        let copy: *mut u8 = ac.result_view().push::<u8>(length + 1);
        ufbxi_check_err!(ac.error_view(), !copy.is_null(), "copy");
        // C: `memcpy(copy, str->data, length);`
        // SAFETY: `str_.data` covers `length` readable bytes and `copy` is the
        // freshly pushed `length + 1` byte run, a distinct non-overlapping
        // result-buffer allocation.
        unsafe { ptr::copy_nonoverlapping((*str_).data, copy, length) };
        // C: `copy[str->length] = '\0';`
        // SAFETY: `(*str_).length` is `length`, the last slot of the
        // `length + 1` byte run just pushed.
        unsafe { *copy.add((*str_).length) = b'\0' };
        // SAFETY: `str_` is the live string slot, retargeted at the copy.
        unsafe { (*str_).data = copy };
    } else {
        // SAFETY: `str_` is the live string slot; the assert only reads its
        // `data` pointer.
        ufbx_assert!(unsafe { (*str_).data } == EMPTY_CHAR.as_ptr());
    }

    Ok(())
}

// ufbx.c:26528-26534 `ufbxi_prop_override_prop_name_less`
pub(crate) unsafe extern "C" fn prop_override_prop_name_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const PropOverride = va as *const PropOverride;
    let b: *const PropOverride = vb as *const PropOverride;
    // SAFETY (this condition): the sort comparator contract guarantees `va` and
    // `vb` each point to a live `ufbx_prop_override` element of the array being
    // sorted, so both `_internal_key` fields are readable.
    if unsafe { (*a)._internal_key } != unsafe { (*b)._internal_key } {
        // SAFETY: as above.
        return unsafe { (*a)._internal_key } < unsafe { (*b)._internal_key };
    }
    // SAFETY: as above, reading each element's `prop_name` string by value;
    // `str_less` only compares the spans those strings describe.
    unsafe { str_less((*a).prop_name, (*b).prop_name) }
}

// ufbx.c:26536-26543 `ufbxi_prop_override_less`
pub(crate) unsafe extern "C" fn prop_override_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const PropOverride = va as *const PropOverride;
    let b: *const PropOverride = vb as *const PropOverride;
    // SAFETY (this condition): the sort comparator contract guarantees `va` and
    // `vb` each point to a live `ufbx_prop_override` element of the array being
    // sorted, so both `element_id` fields are readable.
    if unsafe { (*a).element_id } != unsafe { (*b).element_id } {
        // SAFETY: as above.
        return unsafe { (*a).element_id } < unsafe { (*b).element_id };
    }
    // SAFETY: as above, for the `_internal_key` fields.
    if unsafe { (*a)._internal_key } != unsafe { (*b)._internal_key } {
        // SAFETY: as above.
        return unsafe { (*a)._internal_key } < unsafe { (*b)._internal_key };
    }
    // SAFETY: as above; at the one call site (the post-interning sort in
    // `create_anim_imp`) every `prop_name` is a STRINGS-table entry or a
    // NUL-terminated `push_anim_string` copy, so `strcmp` stays inside both.
    unsafe { strcmp((*a).prop_name.data, (*b).prop_name.data) < 0 }
}

// ufbx.c:26545-26550 `ufbxi_transform_override_less`
pub(crate) unsafe extern "C" fn transform_override_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const TransformOverride = va as *const TransformOverride;
    let b: *const TransformOverride = vb as *const TransformOverride;
    // SAFETY: the sort comparator contract guarantees `va` and `vb` each point
    // to a live `ufbx_transform_override` element of the array being sorted.
    unsafe { (*a).node_id < (*b).node_id }
}

// ufbx.c:26552-26668 `ufbxi_create_anim_imp`
#[inline(never)]
pub(crate) fn create_anim_imp(ac: &CreateAnimContext) -> Result<(), Fail> {
    let scene: *const Scene = ac.scene();
    let anim: *mut Anim = ac.anim_mut_ptr();

    // SAFETY: initializing ac's own result allocator from ac's own error slot
    // and the caller's opts allocator descriptor, named by a `'static`
    // NUL-terminated literal.
    unsafe {
        init_ator(
            ac.error_mut_ptr(),
            ac.ator_result_mut_ptr(),
            ac.opts_view().result_allocator_ptr(),
            c"result",
        );
    }
    ac.result_view().set_unordered(true);
    ac.result_view().set_ator(ac.ator_result_mut_ptr());

    // SAFETY: `anim` is ac's own output `Anim` slot (ac construction
    // invariant); the layer array is pushed onto ac's own result buf with one
    // slot per requested layer id.
    unsafe {
        (*anim).ignore_connections = ac.opts_view().ignore_connections();
        (*anim).custom = true;
    }

    let num_layers: usize = ac.opts_view().layer_ids_view().count();
    unsafe {
        (*anim).layers.count = num_layers;
        (*anim).layers.data =
            ac.result_view().push_zero::<*mut AnimLayer>(num_layers) as *const Ref<AnimLayer>;
    }
    ufbxi_check_err!(
        ac.error_view(),
        !unsafe { (*anim).layers.data }.is_null(),
        "anim->layers.data"
    );

    if ac.opts_view().override_layer_weights_view().count() > 0 {
        ufbxi_check_err_msg!(
            ac.error_view(),
            ac.opts_view().override_layer_weights_view().count() == num_layers,
            "override_layer_weights[] count must match layer_ids[] count",
            "ac->opts.override_layer_weights.count == num_layers"
        );
        // SAFETY: `override_layer_weights` was just checked to hold exactly
        // `num_layers` entries, so the copy reads that whole caller run into a
        // fresh push on ac's own result buf.
        unsafe {
            (*anim).override_layer_weights.data = push_copy::<Real>(
                ac.result_mut_ptr(),
                num_layers,
                ac.opts_view().override_layer_weights_view().data(),
            );
        }
        ufbxi_check_err!(
            ac.error_view(),
            !unsafe { (*anim).override_layer_weights.data }.is_null(),
            "anim->override_layer_weights.data"
        );
        unsafe { (*anim).override_layer_weights.count = num_layers };
    }

    for i in 0..num_layers {
        // SAFETY: `i < num_layers` is the opts' own `layer_ids` count.
        let index: u32 = unsafe { *ac.opts_view().layer_ids_view().data().add(i) };
        ufbxi_check_err_msg!(
            ac.error_view(),
            (index as usize) < unsafe { (*scene).anim_layers.count },
            "layer_ids out of bounds",
            "index < scene->anim_layers.count"
        );
        // C: `anim->layers.data[i] = ac->scene->anim_layers.data[index];`
        // SAFETY: `index` was just bounds-checked against the live scene's
        // `anim_layers`, and `i < num_layers` indexes the fresh layer push.
        unsafe {
            *((*anim).layers.data as *mut *mut AnimLayer).add(i) =
                *((*scene).anim_layers.data as *mut *mut AnimLayer).add(index as usize);
        }
    }

    // C: `ufbx_const_prop_override_desc_list prop_overrides = ac->opts.prop_overrides;`
    // SAFETY: reading the opts' own list header (pointer + count) by value.
    let prop_overrides: crate::prelude::RawList<RawPropOverrideDesc> =
        unsafe { ptr::read(ac.opts_view().prop_overrides_ptr()) };
    if prop_overrides.count > 0 {
        // SAFETY: `anim` is the caller's own out-parameter; the push reserves
        // one destination slot per caller-supplied override.
        unsafe {
            (*anim).prop_overrides.count = prop_overrides.count;
            (*anim).prop_overrides.data = ac
                .result_view()
                .push_zero::<PropOverride>(prop_overrides.count);
        }
        ufbxi_check_err!(
            ac.error_view(),
            !unsafe { (*anim).prop_overrides.data }.is_null(),
            "anim->prop_overrides.data"
        );

        for i in 0..prop_overrides.count {
            // SAFETY: `i < prop_overrides.count` indexes both the caller's own
            // override run and the fresh non-null push of the same length; the
            // two string fields written are `dst`'s own, read from `src`'s.
            unsafe {
                let src: *const RawPropOverrideDesc = prop_overrides.data.add(i);
                let dst: *mut PropOverride =
                    ((*anim).prop_overrides.data as *mut PropOverride).add(i);

                (*dst).element_id = (*src).element_id;
                (*dst).value = (*src).value;
                (*dst).value_int = (*src).value_int;

                if (*dst).value.x != 0.0 && (*dst).value_int == 0 {
                    // C: `(int64_t)dst->value.x` — bare float→int cast; Rust `as`
                    // saturates (PORTING.md integer-semantics table, accepted
                    // divergence class).
                    (*dst).value_int = (*dst).value.x as i64;
                } else if (*dst).value_int != 0 && (*dst).value.x == 0.0 {
                    (*dst).value.x = (*dst).value_int as Real;
                }

                // C: `ufbxi_check_err(&ac->error, ufbxi_check_string(&ac->error, &dst->prop_name, &src->prop_name));`
                check_string(
                    ac.error_mut_ptr(),
                    &raw mut (*dst).prop_name,
                    &raw const (*src).prop_name as *const String,
                )?;
                check_string(
                    ac.error_mut_ptr(),
                    &raw mut (*dst).value_str,
                    &raw const (*src).value_str as *const String,
                )?;

                (*dst)._internal_key = get_name_key((*dst).prop_name.data, (*dst).prop_name.length);
            }
        }

        // Sort `anim->prop_overrides` first by `prop_name` only so we can deduplicate and
        // convert them to global strings in `ufbxi_strings[]` if possible.
        // SAFETY: the run is the fresh non-null push of `prop_overrides.count`
        // elements; neither comparator takes user data, so the null `user` is
        // what they expect. The walk then stays inside that run, and
        // `global_str` walks the `'static` `STRINGS` table between its own
        // bounds. `str_equal`/`str_less` read the NUL-free `data`/`length`
        // runs of interned or caller strings already validated by
        // `check_string` above.
        unsafe {
            unstable_sort(
                (*anim).prop_overrides.data as *mut PropOverride as *mut c_void,
                (*anim).prop_overrides.count,
                size_of::<PropOverride>(),
                prop_override_prop_name_less,
                ptr::null_mut(),
            );

            let mut global_str: *const String = STRINGS.0.as_ptr();
            let global_end: *const String = global_str.add(STRINGS.0.len());
            // C: `ufbx_string prev_name = { ufbxi_empty_char };`
            let mut prev_name: String = String::new_c(EMPTY_CHAR.as_ptr(), 0);
            // C: `ufbxi_for_list(ufbx_prop_override, over, anim->prop_overrides)`
            let mut over: *mut PropOverride = (*anim).prop_overrides.data as *mut PropOverride;
            let over_end: *mut PropOverride = add_ptr(over, (*anim).prop_overrides.count);
            while over != over_end {
                if (*over).value_str.length > 0 {
                    // C: `ufbxi_check_err(&ac->error, ufbxi_push_anim_string(ac, &over->value_str));`
                    push_anim_string(ac, &raw mut (*over).value_str)?;
                }

                if str_equal((*over).prop_name, prev_name) {
                    (*over).prop_name = prev_name;
                    over = over.add(1);
                    continue;
                }

                while global_str != global_end && str_less(*global_str, (*over).prop_name) {
                    global_str = global_str.add(1);
                }

                if global_str != global_end && str_equal(*global_str, (*over).prop_name) {
                    (*over).prop_name = *global_str;
                } else {
                    push_anim_string(ac, &raw mut (*over).prop_name)?;
                }

                prev_name = (*over).prop_name;
                over = over.add(1);
            }

            // Sort `anim->prop_overrides` to the actual order expected by evaluation.
            unstable_sort(
                (*anim).prop_overrides.data as *mut PropOverride as *mut c_void,
                (*anim).prop_overrides.count,
                size_of::<PropOverride>(),
                prop_override_less,
                ptr::null_mut(),
            );
        }

        for i in 1..prop_overrides.count {
            // SAFETY: `1 <= i < prop_overrides.count`, so both `i - 1` and `i`
            // index the pushed run; `prop_name.data` is the NUL-terminated
            // interned name the duplicate message formats.
            unsafe {
                let prev: *const PropOverride = (*anim).prop_overrides.data.add(i - 1);
                let next: *const PropOverride = (*anim).prop_overrides.data.add(i);
                if (*prev).element_id == (*next).element_id
                    && (*prev).prop_name.data == (*next).prop_name.data
                {
                    ufbxi_fmt_err_info!(
                        ac.error_mut_ptr(),
                        "element %u prop \"%s\"",
                        (*prev).element_id,
                        (*prev).prop_name.data
                    );
                    ufbxi_fail_err_msg!(
                        ac.error_view(),
                        "Duplicate override",
                        "Duplicate override"
                    );
                }
            }
        }
    }

    if ac.opts_view().transform_overrides_view().count() > 0 {
        // SAFETY: the copy reads exactly the caller's own transform-override
        // run into a fresh push of the same length on ac's own result buf.
        unsafe {
            (*anim).transform_overrides.count = ac.opts_view().transform_overrides_view().count();
            (*anim).transform_overrides.data = push_copy::<TransformOverride>(
                ac.result_mut_ptr(),
                (*anim).transform_overrides.count,
                ac.opts_view().transform_overrides_view().data(),
            );
        }
        ufbxi_check_err!(
            ac.error_view(),
            !unsafe { (*anim).transform_overrides.data }.is_null(),
            "anim->transform_overrides.data"
        );
        // SAFETY: sorting the fresh non-null run just checked, over its own
        // count; the comparator takes no user data.
        unsafe {
            unstable_sort(
                (*anim).transform_overrides.data as *mut TransformOverride as *mut c_void,
                (*anim).transform_overrides.count,
                size_of::<TransformOverride>(),
                transform_override_less,
                ptr::null_mut(),
            );
        }
    }

    ac.set_imp(ac.result_view().push::<AnimImp>(1));
    ufbxi_check_err!(ac.error_view(), !ac.imp().is_null(), "ac->imp");

    // C: `ufbxi_init_ref(...)` / `ac->imp->magic = ...` / `ac->imp->anim =
    // ac->anim` / `ac->imp->refcount.ator = ac->ator_result` /
    // `ac->imp->refcount.buf = ac->result` — the shared imp-finalization group.
    //
    // SAFETY: `ac.imp()` is the fresh non-null push just checked and the last
    // allocation of `ac->result`; the parent refcount is the `SceneImp` behind
    // the scene this anim was created for, which owns it for the duration of
    // this call; and `ac.anim_mut_ptr()` is ac's own `Anim` slot, a distinct
    // allocation from the pushed imp.
    unsafe {
        finish_imp(
            ac.imp(),
            &raw mut (*get_imp::<SceneImp>(scene as *mut Scene as *mut c_void)).refcount,
            ac.anim_mut_ptr(),
            ac.ator_result(),
            ac.take_result(),
        );
    }

    Ok(())
}

// -- Animation baking (ufbx.c:26670)

// ufbx.c:26672-26676 `ufbxi_baked_anim_imp`
// C declares this OUTSIDE the `#if UFBXI_FEATURE_ANIMATION_BAKING` guard
// (opened at ufbx.c:26678) precisely so `ufbx_retain_baked_anim` /
// `ufbx_free_baked_anim` (ufbx.c:31291-31309) compile in both builds; it
// therefore carries no `cfg` here either. C declares no `ufbx_static_assert`
// for it (contrast `ufbxi_scene_imp`), but `ufbxi_get_imp(ufbxi_baked_anim_imp,
// bake)` (ufbx.c:31295) depends on the header-then-payload layout, so the
// offset is pinned here (same treatment as `ufbxi_anim_imp` in
// `native::scene_process`).
#[repr(C)]
pub(crate) struct BakedAnimImp {
    pub refcount: Refcount,
    pub bake: BakedAnim,
    pub magic: u32,
}

const _: () = assert!(core::mem::offset_of!(BakedAnimImp, bake) == size_of::<Refcount>());

// C: `#if UFBXI_FEATURE_ANIMATION_BAKING` (ufbx.c:26678, closed at
// ufbx.c:27767) — every item from here to the end of the section carries
// `#[cfg(feature = "baking")]`, the Rust spelling of that guard.

// ufbx.c:26680-26683 `ufbxi_bake_time`
#[cfg(feature = "baking")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct BakeTime {
    pub time: f64,
    pub flags: u32,
}

// ufbx.c:26685 `UFBX_LIST_TYPE(ufbxi_bake_time_list, ufbxi_bake_time)`
// An internal list over an internal element type, so it is hand-ported here
// rather than taken from `prelude`; `data` stays `*mut` because
// `ufbxi_finalize_bake_times` hands back an array its callers keep mutating.
#[cfg(feature = "baking")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct BakeTimeList {
    pub data: *mut BakeTime,
    pub count: usize,
}

// Typed interior-mutable VIEW over a `BakeTimeList` field, reinterpreted in place
// (getters + setters; the list is built by writing `.count`/`.data`).
#[cfg(feature = "baking")]
#[cfg(feature = "baking")]
pub(crate) type BakeTimeListView = crate::native::view::View<BakeTimeList>;

#[cfg(feature = "baking")]
#[cfg(feature = "baking")]
impl BakeTimeListView {
    #[inline(always)]
    pub(crate) fn count(&self) -> usize {
        unsafe { (*self.get()).count }
    }
    #[inline(always)]
    pub(crate) fn data(&self) -> *mut BakeTime {
        unsafe { (*self.get()).data }
    }
    #[inline(always)]
    pub(crate) fn set_count(&self, count: usize) {
        unsafe {
            (*self.get()).count = count;
        }
    }
    #[inline(always)]
    pub(crate) fn set_data(&self, data: *mut BakeTime) {
        unsafe {
            (*self.get()).data = data;
        }
    }
}

// ufbx.c:26687-26723 `ufbxi_bake_context`
#[cfg(feature = "baking")]
#[repr(C)]
pub(crate) struct InnerBakeContext {
    pub error: Error,
    pub ator_tmp: Allocator,
    pub ator_result: Allocator,

    pub result: Buf,
    pub tmp: Buf,
    pub tmp_prop: Buf,
    pub tmp_times: Buf,
    pub tmp_bake_props: Buf,
    pub tmp_nodes: Buf,
    pub tmp_elements: Buf,
    pub tmp_props: Buf,
    pub tmp_bake_stack: Buf,

    pub layer_weight_times: BakeTimeList,

    pub baked_nodes: *mut *mut BakedNode,
    pub nodes_to_bake: *mut bool,

    // C: `char *tmp_arr;` — the `ufbxi_macro_stable_sort` scratch.
    pub tmp_arr: *mut u8,
    pub tmp_arr_size: usize,

    pub scene: *const Scene,
    pub anim: *const Anim,
    pub opts: RawBakeOpts,

    pub ktime_offset: f64,

    pub time_begin: f64,
    pub time_end: f64,
    pub time_min: f64,
    pub time_max: f64,

    pub bake: BakedAnim,
    pub imp: *mut BakedAnimImp,
}

// Typed interior-mutable VIEW over the `opts` field of `BakeContext`, reinterpreted in
// place (approach A). The generated ABI-fixed `RawBakeOpts` plays the `Inner` role;
// `MaybeUninit` makes forming `&BakeOptsView` assert no validity — each leaf getter
// asserts only the field it reads.
#[cfg(feature = "baking")]
pub(crate) type BakeOptsView = crate::native::view::View<RawBakeOpts>;

#[cfg(feature = "baking")]
impl BakeOptsView {
    #[inline(always)]
    pub(crate) fn trim_start_time(&self) -> bool {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.trim_start_time` read already makes.
        unsafe { (*self.get()).trim_start_time }
    }

    #[inline(always)]
    pub(crate) fn resample_rate(&self) -> f64 {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.resample_rate` read already makes.
        unsafe { (*self.get()).resample_rate }
    }

    #[inline(always)]
    pub(crate) fn minimum_sample_rate(&self) -> f64 {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.minimum_sample_rate` read already makes.
        unsafe { (*self.get()).minimum_sample_rate }
    }

    #[inline(always)]
    pub(crate) fn maximum_sample_rate(&self) -> f64 {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.maximum_sample_rate` read already makes.
        unsafe { (*self.get()).maximum_sample_rate }
    }

    #[inline(always)]
    pub(crate) fn bake_transform_props(&self) -> bool {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.bake_transform_props` read already makes.
        unsafe { (*self.get()).bake_transform_props }
    }

    #[inline(always)]
    pub(crate) fn skip_node_transforms(&self) -> bool {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.skip_node_transforms` read already makes.
        unsafe { (*self.get()).skip_node_transforms }
    }

    #[inline(always)]
    pub(crate) fn no_resample_rotation(&self) -> bool {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.no_resample_rotation` read already makes.
        unsafe { (*self.get()).no_resample_rotation }
    }

    #[inline(always)]
    pub(crate) fn ignore_layer_weight_animation(&self) -> bool {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.ignore_layer_weight_animation` read already makes.
        unsafe { (*self.get()).ignore_layer_weight_animation }
    }

    #[inline(always)]
    pub(crate) fn max_keyframe_segments(&self) -> usize {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.max_keyframe_segments` read already makes.
        unsafe { (*self.get()).max_keyframe_segments }
    }

    #[inline(always)]
    pub(crate) fn step_handling(&self) -> BakeStepHandling {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.step_handling` read already makes.
        unsafe { (*self.get()).step_handling }
    }

    #[inline(always)]
    pub(crate) fn step_custom_duration(&self) -> f64 {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.step_custom_duration` read already makes.
        unsafe { (*self.get()).step_custom_duration }
    }

    #[inline(always)]
    pub(crate) fn step_custom_epsilon(&self) -> f64 {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.step_custom_epsilon` read already makes.
        unsafe { (*self.get()).step_custom_epsilon }
    }

    #[inline(always)]
    pub(crate) fn evaluate_flags(&self) -> u32 {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.evaluate_flags` read already makes.
        unsafe { (*self.get()).evaluate_flags }
    }

    #[inline(always)]
    pub(crate) fn key_reduction_enabled(&self) -> bool {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.key_reduction_enabled` read already makes.
        unsafe { (*self.get()).key_reduction_enabled }
    }

    #[inline(always)]
    pub(crate) fn key_reduction_rotation(&self) -> bool {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.key_reduction_rotation` read already makes.
        unsafe { (*self.get()).key_reduction_rotation }
    }

    #[inline(always)]
    pub(crate) fn key_reduction_threshold(&self) -> f64 {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.key_reduction_threshold` read already makes.
        unsafe { (*self.get()).key_reduction_threshold }
    }

    #[inline(always)]
    pub(crate) fn key_reduction_passes(&self) -> usize {
        // SAFETY: reading a POD/enum opts field by value — same assertion the
        // direct `.opts.key_reduction_passes` read already makes.
        unsafe { (*self.get()).key_reduction_passes }
    }

    #[inline(always)]
    pub(crate) fn set_resample_rate(&self, resample_rate: f64) {
        // SAFETY: interior-mutable write of a POD opts field; cannot violate validity.
        unsafe {
            (*self.get()).resample_rate = resample_rate;
        }
    }

    #[inline(always)]
    pub(crate) fn set_minimum_sample_rate(&self, minimum_sample_rate: f64) {
        // SAFETY: interior-mutable write of a POD opts field; cannot violate validity.
        unsafe {
            (*self.get()).minimum_sample_rate = minimum_sample_rate;
        }
    }

    #[inline(always)]
    pub(crate) fn set_max_keyframe_segments(&self, max_keyframe_segments: usize) {
        // SAFETY: interior-mutable write of a POD opts field; cannot violate validity.
        unsafe {
            (*self.get()).max_keyframe_segments = max_keyframe_segments;
        }
    }

    #[inline(always)]
    pub(crate) fn set_key_reduction_threshold(&self, key_reduction_threshold: f64) {
        // SAFETY: interior-mutable write of a POD opts field; cannot violate validity.
        unsafe {
            (*self.get()).key_reduction_threshold = key_reduction_threshold;
        }
    }

    #[inline(always)]
    pub(crate) fn set_key_reduction_passes(&self, key_reduction_passes: usize) {
        // SAFETY: interior-mutable write of a POD opts field; cannot violate validity.
        unsafe {
            (*self.get()).key_reduction_passes = key_reduction_passes;
        }
    }

    #[inline(always)]
    pub(crate) fn temp_allocator_ptr(&self) -> *const crate::generated::RawAllocatorOpts {
        // SAFETY: `&raw const` address of a read-only sub-struct; no reference formed.
        unsafe { &raw const (*self.get()).temp_allocator }
    }

    #[inline(always)]
    pub(crate) fn result_allocator_ptr(&self) -> *const crate::generated::RawAllocatorOpts {
        // SAFETY: `&raw const` address of a read-only sub-struct; no reference formed.
        unsafe { &raw const (*self.get()).result_allocator }
    }
}

// Safe `&BakeContext` handle over the fields-struct `InnerBakeContext`, mirroring
// the `Context`/`InnerContext` seam in `parse.rs`. `MaybeUninit` because it embeds
// the public `BakedAnim` (enum-bearing) in `bake`, so a plain `&InnerBakeContext`
// could not be formed soundly; `UnsafeCell` gives the interior mutability every
// `&BakeContext` site needs. Field is `pub(crate)` — the sole construction site
// lives in `native::api`.
#[repr(transparent)]
#[cfg(feature = "baking")]
pub(crate) struct BakeContext(
    pub(crate) core::cell::UnsafeCell<core::mem::MaybeUninit<InnerBakeContext>>,
);

// Typed interior-mutable VIEW over `BakedAnimMetadata` (non-Copy substruct).
pub(crate) type BakedAnimMetadataView =
    crate::native::view::View<crate::generated::BakedAnimMetadata>;

impl BakedAnimMetadataView {
    #[inline(always)]
    pub(crate) fn set_result_memory_used(&self, result_memory_used: usize) {
        unsafe {
            (*self.get()).result_memory_used = result_memory_used;
        }
    }
    #[inline(always)]
    pub(crate) fn set_temp_memory_used(&self, temp_memory_used: usize) {
        unsafe {
            (*self.get()).temp_memory_used = temp_memory_used;
        }
    }
    #[inline(always)]
    pub(crate) fn set_result_allocs(&self, result_allocs: usize) {
        unsafe {
            (*self.get()).result_allocs = result_allocs;
        }
    }
    #[inline(always)]
    pub(crate) fn set_temp_allocs(&self, temp_allocs: usize) {
        unsafe {
            (*self.get()).temp_allocs = temp_allocs;
        }
    }
}

// Typed interior-mutable VIEW over `BakeContext.bake` (public `BakedAnim`).
pub(crate) type BakedAnimView = crate::native::view::View<crate::generated::BakedAnim>;

impl BakedAnimView {
    #[inline(always)]
    pub(crate) fn nodes_view(&self) -> &crate::prelude::ListView<crate::generated::BakedNode> {
        unsafe {
            &*(&raw mut (*self.get()).nodes
                as *mut crate::prelude::ListView<crate::generated::BakedNode>)
        }
    }

    #[inline(always)]
    pub(crate) fn elements_view(
        &self,
    ) -> &crate::prelude::ListView<crate::generated::BakedElement> {
        unsafe {
            &*(&raw mut (*self.get()).elements
                as *mut crate::prelude::ListView<crate::generated::BakedElement>)
        }
    }

    #[inline(always)]
    pub(crate) fn set_key_time_min(&self, key_time_min: f64) {
        unsafe {
            (*self.get()).key_time_min = key_time_min;
        }
    }
    #[inline(always)]
    pub(crate) fn set_key_time_max(&self, key_time_max: f64) {
        unsafe {
            (*self.get()).key_time_max = key_time_max;
        }
    }
    #[inline(always)]
    pub(crate) fn set_playback_time_begin(&self, playback_time_begin: f64) {
        unsafe {
            (*self.get()).playback_time_begin = playback_time_begin;
        }
    }
    #[inline(always)]
    pub(crate) fn set_playback_time_end(&self, playback_time_end: f64) {
        unsafe {
            (*self.get()).playback_time_end = playback_time_end;
        }
    }
    #[inline(always)]
    pub(crate) fn set_playback_duration(&self, playback_duration: f64) {
        unsafe {
            (*self.get()).playback_duration = playback_duration;
        }
    }

    #[inline(always)]
    pub(crate) fn metadata_view(&self) -> &BakedAnimMetadataView {
        unsafe { &*(&raw mut (*self.get()).metadata as *mut BakedAnimMetadataView) }
    }
}

#[cfg(feature = "baking")]
impl BakeContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerBakeContext {
        self.0.get().cast()
    }

    #[inline(always)]
    pub(crate) fn bake_mut_ptr(&self) -> *mut BakedAnim {
        unsafe { &raw mut (*self.get()).bake }
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

    // `layer_weight_times` (BakeTimeList) — typed VIEW handle (reinterpret-in-place).
    #[inline(always)]
    pub(crate) fn layer_weight_times_view(&self) -> &BakeTimeListView {
        // SAFETY: reinterpret the BakeTimeList field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).layer_weight_times as *mut BakeTimeListView) }
    }

    #[inline(always)]
    pub(crate) fn bake_view(&self) -> &BakedAnimView {
        // SAFETY: reinterpret the `bake` field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).bake as *mut BakedAnimView) }
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

    // `tmp_bake_props` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_bake_props_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_bake_props as *mut crate::native::buf::BufView) }
    }

    // `tmp_bake_stack` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_bake_stack_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_bake_stack as *mut crate::native::buf::BufView) }
    }

    // `tmp_elements` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_elements_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_elements as *mut crate::native::buf::BufView) }
    }

    // `tmp_nodes` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_nodes_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_nodes as *mut crate::native::buf::BufView) }
    }

    // `tmp_prop` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_prop_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_prop as *mut crate::native::buf::BufView) }
    }

    // `tmp_props` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_props_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_props as *mut crate::native::buf::BufView) }
    }

    // `tmp_times` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_times_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_times as *mut crate::native::buf::BufView) }
    }

    // `opts` — typed VIEW handle (reinterpret-in-place). Leaf accessors live on
    // `BakeOptsView`; no validity asserted until a field is read.
    #[inline(always)]
    pub(crate) fn opts_view(&self) -> &BakeOptsView {
        // SAFETY: `BakeOptsView` is repr(transparent) over the `opts` field's layout;
        // the field lives in this context's outer UnsafeCell, so a shared
        // interior-mutable `&BakeOptsView` is sound and asserts no validity.
        unsafe { &*(&raw mut (*self.get()).opts as *mut BakeOptsView) }
    }

    // `tmp_times` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_times_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_times }
    }

    // `tmp_props` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_props_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_props }
    }

    // `tmp_prop` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_prop_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_prop }
    }

    // `tmp_nodes` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_nodes_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_nodes }
    }

    // `tmp_elements` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_elements_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_elements }
    }

    // `tmp_bake_stack` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_bake_stack_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_bake_stack }
    }

    // `tmp_bake_props` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_bake_props_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_bake_props }
    }

    // `tmp_arr_size` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_arr_size_mut_ptr(&self) -> *mut usize {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_arr_size }
    }

    // `tmp_arr` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_arr_mut_ptr(&self) -> *mut *mut u8 {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_arr }
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

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawBakeOpts {
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

    // `ator_tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_tmp_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_tmp }
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
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_result }
    }

    // `ator_result` (Allocator) — typed VIEW handle (reinterpret-in-place); accessors on AllocatorView.
    #[inline(always)]
    pub(crate) fn ator_result_view(&self) -> &crate::native::allocator::AllocatorView {
        // SAFETY: reinterpret the owned Allocator field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).ator_result as *mut crate::native::allocator::AllocatorView)
        }
    }

    // `scene` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn scene(&self) -> *const Scene {
        // SAFETY: reading a scalar field; all bit patterns of `*const Scene` are valid.
        unsafe { (*self.get()).scene }
    }

    #[inline(always)]
    pub(crate) fn set_scene(&self, scene: *const Scene) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).scene = scene;
        }
    }

    // `anim` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn anim(&self) -> *const Anim {
        // SAFETY: reading a scalar field; all bit patterns of `*const Anim` are valid.
        unsafe { (*self.get()).anim }
    }

    #[inline(always)]
    pub(crate) fn set_anim(&self, anim: *const Anim) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).anim = anim;
        }
    }

    // `imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn imp(&self) -> *mut BakedAnimImp {
        // SAFETY: reading a scalar field; all bit patterns of `*mut BakedAnimImp` are valid.
        unsafe { (*self.get()).imp }
    }

    #[inline(always)]
    pub(crate) fn set_imp(&self, imp: *mut BakedAnimImp) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).imp = imp;
        }
    }

    // `time_max` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn time_max(&self) -> f64 {
        // SAFETY: reading a scalar field; all bit patterns of `f64` are valid.
        unsafe { (*self.get()).time_max }
    }

    #[inline(always)]
    pub(crate) fn set_time_max(&self, time_max: f64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).time_max = time_max;
        }
    }

    // `time_min` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn time_min(&self) -> f64 {
        // SAFETY: reading a scalar field; all bit patterns of `f64` are valid.
        unsafe { (*self.get()).time_min }
    }

    #[inline(always)]
    pub(crate) fn set_time_min(&self, time_min: f64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).time_min = time_min;
        }
    }

    // `time_end` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn time_end(&self) -> f64 {
        // SAFETY: reading a scalar field; all bit patterns of `f64` are valid.
        unsafe { (*self.get()).time_end }
    }

    #[inline(always)]
    pub(crate) fn set_time_end(&self, time_end: f64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).time_end = time_end;
        }
    }

    // `time_begin` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn time_begin(&self) -> f64 {
        // SAFETY: reading a scalar field; all bit patterns of `f64` are valid.
        unsafe { (*self.get()).time_begin }
    }

    #[inline(always)]
    pub(crate) fn set_time_begin(&self, time_begin: f64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).time_begin = time_begin;
        }
    }

    // `ktime_offset` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn ktime_offset(&self) -> f64 {
        // SAFETY: reading a scalar field; all bit patterns of `f64` are valid.
        unsafe { (*self.get()).ktime_offset }
    }

    #[inline(always)]
    pub(crate) fn set_ktime_offset(&self, ktime_offset: f64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).ktime_offset = ktime_offset;
        }
    }

    // `tmp_arr_size` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_arr_size(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).tmp_arr_size }
    }

    // `tmp_arr` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_arr(&self) -> *mut u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u8` are valid.
        unsafe { (*self.get()).tmp_arr }
    }

    // `nodes_to_bake` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn nodes_to_bake(&self) -> *mut bool {
        // SAFETY: reading a scalar field; all bit patterns of `*mut bool` are valid.
        unsafe { (*self.get()).nodes_to_bake }
    }

    #[inline(always)]
    pub(crate) fn set_nodes_to_bake(&self, nodes_to_bake: *mut bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).nodes_to_bake = nodes_to_bake;
        }
    }

    // `baked_nodes` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn baked_nodes(&self) -> *mut *mut BakedNode {
        // SAFETY: reading a scalar field; all bit patterns of `*mut *mut BakedNode` are valid.
        unsafe { (*self.get()).baked_nodes }
    }

    #[inline(always)]
    pub(crate) fn set_baked_nodes(&self, baked_nodes: *mut *mut BakedNode) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).baked_nodes = baked_nodes;
        }
    }
}

// ufbx.c:26725-26730 `ufbxi_bake_prop`
#[cfg(feature = "baking")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct BakeProp {
    pub sort_id: u32,
    pub element_id: u32,
    pub prop_name: *const u8,
    pub anim_value: *mut AnimValue,
}

// Rust-port infra (not a ufbx.c type): reinterpret-in-place VIEW over a
// `BakeProp`. The C `ufbxi_for(ufbxi_bake_prop, prop, props, count)` loops walk a
// contiguous `push_pop`-materialized `BakeProp` run; `SliceViewIter` walks that
// `(base, count)` run yielding `&BakePropView`, replacing raw `prop.add(1)`
// navigation with a safe contiguous iteration whose only `unsafe` is the
// `from_raw_parts` run vouch.
#[cfg(feature = "baking")]
#[cfg(feature = "baking")]
pub(crate) type BakePropView = crate::native::view::View<BakeProp>;

#[cfg(feature = "baking")]
#[cfg(feature = "baking")]
impl BakePropView {
    #[inline(always)]
    pub(crate) fn prop_name(&self) -> *const u8 {
        // SAFETY: view over a valid, initialized `BakeProp`; leaf read.
        unsafe { (*self.get()).prop_name }
    }

    #[inline(always)]
    pub(crate) fn anim_value(&self) -> *mut AnimValue {
        // SAFETY: view over a valid, initialized `BakeProp`; leaf read.
        unsafe { (*self.get()).anim_value }
    }

    #[inline(always)]
    pub(crate) fn element_id(&self) -> u32 {
        // SAFETY: view over a valid, initialized `BakeProp`; leaf read.
        unsafe { (*self.get()).element_id }
    }
}

// ufbx.c:26732-26741 `ufbxi_bake_prop_less`
#[cfg(feature = "baking")]
pub(crate) unsafe extern "C" fn bake_prop_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const BakeProp = va as *const BakeProp;
    let b: *const BakeProp = vb as *const BakeProp;
    // SAFETY (this condition): the sort comparator contract guarantees `va` and
    // `vb` each point to a live `ufbxi_bake_prop` element of the array being
    // sorted, so both `sort_id` fields are readable.
    if unsafe { (*a).sort_id } != unsafe { (*b).sort_id } {
        // SAFETY: as above.
        return unsafe { (*a).sort_id } < unsafe { (*b).sort_id };
    }
    // SAFETY: as above, for the `element_id` fields.
    if unsafe { (*a).element_id } != unsafe { (*b).element_id } {
        // SAFETY: as above.
        return unsafe { (*a).element_id } < unsafe { (*b).element_id };
    }
    // SAFETY: as above, comparing the two `prop_name` pointers themselves.
    if unsafe { (*a).prop_name } != unsafe { (*b).prop_name } {
        // SAFETY: as above; both `prop_name`s are pooled NUL-terminated strings,
        // so `strcmp` stays inside them.
        return unsafe { strcmp((*a).prop_name, (*b).prop_name) } < 0;
    }
    false
}

// ufbx.c:26743-26745 `ufbx_static_assert(bake_step_left/right/key, ...)`
#[cfg(feature = "baking")]
const _: () = assert!(BakedKeyFlags::STEP_LEFT.raw() == 0x1);
#[cfg(feature = "baking")]
const _: () = assert!(BakedKeyFlags::STEP_RIGHT.raw() == 0x2);
#[cfg(feature = "baking")]
const _: () = assert!(BakedKeyFlags::STEP_KEY.raw() == 0x4);

// ufbx.c:26746-26754 `ufbxi_cmp_bake_time`
// C: `ufbxi_forceinline`.
#[cfg(feature = "baking")]
#[inline(always)]
pub(crate) fn cmp_bake_time(a: BakeTime, b: BakeTime) -> i32 {
    if a.time != b.time {
        return if a.time < b.time { -1 } else { 1 };
    }
    // Bit twiddling for a fast sorting of `0x1 (LEFT) < 0x0 < 0x2 (RIGHT)`
    // by `step ^ 1`: `0x0 (LEFT) < 0x1 < 0x3 (RIGHT)`
    let a_step: u32 = a.flags & 0x3;
    let b_step: u32 = b.flags & 0x3;
    if a_step != b_step {
        return if (a_step ^ 0x1) < (b_step ^ 0x1) {
            -1
        } else {
            1
        };
    }
    0
}

// ufbx.c:26756-26763 `ufbxi_bake_push_time`
// C returns `int` (1/0) and sets NO error of its own — the caller's
// `ufbxi_check_err` does that. Mapping this to `Result` would swallow the
// error entirely, so it stays a `bool` returned into `ufbxi_check_err!`.
#[cfg(feature = "baking")]
#[inline(always)]
#[must_use]
pub(crate) fn bake_push_time(bc: &BakeContext, time: f64, flags: u32) -> bool {
    let p_key: *mut BakeTime = bc.tmp_times_view().push_fast::<BakeTime>(1);
    if p_key.is_null() {
        return false;
    }
    // SAFETY: `p_key` is non-null (checked) and points to freshly-pushed storage.
    unsafe {
        (*p_key).time = time;
        (*p_key).flags = flags;
    }
    true
}

// ufbx.c:26765-26813 `ufbxi_bake_times`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn bake_times(
    bc: &BakeContext,
    anim_value: *const AnimValue,
    resample_linear: bool,
    key_flag: u32,
) -> Result<(), Fail> {
    let sample_rate: f64 = bc.opts_view().resample_rate();
    let min_duration: f64 = if bc.opts_view().minimum_sample_rate() > 0.0 {
        1.0 / bc.opts_view().minimum_sample_rate()
    } else {
        0.0
    };

    for curve_ix in 0..3usize {
        // SAFETY: `anim_value` is the caller's live `ufbx_anim_value` — this
        // `unsafe fn`'s contract — whose `curves` field is a fixed three-element
        // array of nullable curve refs, so `curve_ix < 3` stays in bounds and
        // `opt_ptr` reads that slot as the nullable element pointer it is.
        let curve: *mut AnimCurve = unsafe {
            opt_ptr(
                (&raw const (*anim_value).curves as *const Option<Ref<AnimCurve>>).add(curve_ix),
            )
        };
        if curve.is_null() {
            continue;
        }

        // SAFETY: `curve` is non-null (checked) and is one of the anim value's
        // own curve elements, live for the scene.
        let keys: *const Keyframe = unsafe { (*curve).keyframes.data };
        // SAFETY: as above, for the same list's count.
        let num_keys: usize = unsafe { (*curve).keyframes.count };
        for key_ix in 0..num_keys {
            // SAFETY: `key_ix < num_keys`, the length of the `keys` array.
            let a: Keyframe = unsafe { *keys.add(key_ix) };
            let a_time: f64 = a.time;
            ufbxi_check_err!(
                bc.error_view(),
                bake_push_time(bc, a_time, key_flag),
                "ufbxi_bake_push_time(bc, a_time, key_flag)"
            );
            if key_ix + 1 >= num_keys {
                break;
            }
            // SAFETY: the `break` above establishes `key_ix + 1 < num_keys`, so
            // that slot is in bounds of the `keys` array.
            let b: Keyframe = unsafe { *keys.add(key_ix + 1) };
            let b_time: f64 = b.time;

            // Skip fully flat sections
            if a.value == b.value && a.right.dy == 0.0f32 && b.left.dy == 0.0f32 {
                continue;
            }

            if a.interpolation as u32 == Interpolation::ConstantPrev as u32 {
                ufbxi_check_err!(
                    bc.error_view(),
                    bake_push_time(bc, b_time, BakedKeyFlags::STEP_LEFT.raw()),
                    "ufbxi_bake_push_time(bc, b_time, UFBX_BAKED_KEY_STEP_LEFT)"
                );
            } else if a.interpolation as u32 == Interpolation::ConstantNext as u32 {
                ufbxi_check_err!(
                    bc.error_view(),
                    bake_push_time(bc, a_time, BakedKeyFlags::STEP_RIGHT.raw()),
                    "ufbxi_bake_push_time(bc, a_time, UFBX_BAKED_KEY_STEP_RIGHT)"
                );
            } else if (resample_linear || a.interpolation as u32 == Interpolation::Cubic as u32)
                && sample_rate > 0.0
            {
                let duration: f64 = b_time - a_time;
                if duration <= min_duration {
                    continue;
                }

                let mut factor: f64 = 1.0;
                while duration * sample_rate / factor
                    >= bc.opts_view().max_keyframe_segments() as f64
                {
                    factor *= 2.0;
                }

                let padding: f64 = 0.5 / sample_rate;
                let start: f64 = math::ceil((a_time + padding) * sample_rate / factor) * factor;
                let stop: f64 = b_time - padding;
                for i in 0..bc.opts_view().max_keyframe_segments() {
                    let time: f64 = (start + i as f64 * factor) / sample_rate;
                    if time >= stop {
                        break;
                    }
                    ufbxi_check_err!(
                        bc.error_view(),
                        bake_push_time(bc, time, 0),
                        "ufbxi_bake_push_time(bc, time, 0)"
                    );
                }
            }
        }
    }

    Ok(())
}

// The `ufbxi_transform_props` / `ufbxi_complex_*` tables below hold raw
// pointers into the interned-string statics; the wrapper struct provides the
// `Sync` the raw pointers lack (same treatment as `PropNameTable` in
// `native::api` and `StringTable` in `native::string_pool`).
#[cfg(feature = "baking")]
#[repr(transparent)]
struct BakePropNameTable<const N: usize>([*const u8; N]);
#[cfg(feature = "baking")]
unsafe impl<const N: usize> Sync for BakePropNameTable<N> {}

// ufbx.c:26815-26818 `ufbxi_transform_props`
#[cfg(feature = "baking")]
static TRANSFORM_PROPS: BakePropNameTable<10> = BakePropNameTable([
    sp::Lcl_Translation.as_ptr(),
    sp::Lcl_Rotation.as_ptr(),
    sp::Lcl_Scaling.as_ptr(),
    sp::PreRotation.as_ptr(),
    sp::PostRotation.as_ptr(),
    sp::RotationOffset.as_ptr(),
    sp::ScalingOffset.as_ptr(),
    sp::RotationPivot.as_ptr(),
    sp::ScalingPivot.as_ptr(),
    sp::RotationOrder.as_ptr(),
]);

// ufbx.c:26820-26822 `ufbxi_complex_translation_props`
#[cfg(feature = "baking")]
static COMPLEX_TRANSLATION_PROPS: BakePropNameTable<4> = BakePropNameTable([
    sp::ScalingPivot.as_ptr(),
    sp::RotationPivot.as_ptr(),
    sp::RotationOffset.as_ptr(),
    sp::ScalingOffset.as_ptr(),
]);

// ufbx.c:26824-26826 `ufbxi_complex_rotation_props`
#[cfg(feature = "baking")]
static COMPLEX_ROTATION_PROPS: BakePropNameTable<3> = BakePropNameTable([
    sp::PreRotation.as_ptr(),
    sp::PostRotation.as_ptr(),
    sp::RotationOrder.as_ptr(),
]);

// ufbx.c:26828-26830 `ufbxi_complex_rotation_sources`
#[cfg(feature = "baking")]
static COMPLEX_ROTATION_SOURCES: BakePropNameTable<4> = BakePropNameTable([
    sp::Lcl_Rotation.as_ptr(),
    sp::PreRotation.as_ptr(),
    sp::PostRotation.as_ptr(),
    sp::RotationOrder.as_ptr(),
]);

// ufbx.c:26832-26838 `ufbxi_in_list`
#[cfg(feature = "baking")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn in_list(items: *const *const u8, count: usize, item: *const u8) -> bool {
    for i in 0..count {
        // SAFETY: this `unsafe fn` requires `items` to address `count` readable
        // string pointers, and `i < count`.
        if unsafe { *items.add(i) } == item {
            return true;
        }
    }
    false
}

// ufbx.c:26840-26845 `ufbxi_sort_bake_times`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn sort_bake_times(
    bc: &BakeContext,
    times: *mut BakeTime,
    count: usize,
) -> Result<(), Fail> {
    // C: `ufbxi_grow_array(&bc->ator_tmp, &bc->tmp_arr, &bc->tmp_arr_size, count * sizeof(ufbxi_bake_time))`
    // — `bc->tmp_arr` is `char*`, so the element size is 1 and the count is in
    // bytes (PORTING.md "Sorting & searching": this paired grow is the
    // allocation-parity invariant).
    ufbxi_check_err!(
        bc.error_view(),
        // SAFETY: the allocator, scratch-array slot and its size are all `bc`'s
        // own fields, live for the `&BakeContext` borrow, and they are grown as
        // the paired triple `grow_array` expects.
        unsafe {
            grow_array::<u8>(
                bc.ator_tmp_mut_ptr(),
                bc.tmp_arr_mut_ptr(),
                bc.tmp_arr_size_mut_ptr(),
                count.wrapping_mul(size_of::<BakeTime>()),
            )
        },
        "ufbxi_grow_array_size((&bc->ator_tmp), sizeof(**(&bc->tmp_arr)), (&bc->tmp_arr), (&bc->tmp_arr_size), (count * sizeof(ufbxi_bake_time)))"
    );
    // SAFETY: this `unsafe fn` requires `times` to address `count` writable
    // `ufbxi_bake_time`s, and the grow above sized `bc.tmp_arr()` to hold that
    // many as the disjoint scratch run `macro_stable_sort` needs. The comparator
    // is handed pointers to two live elements of that run or its scratch, so its
    // derefs — covered by this same block — are in bounds.
    unsafe {
        macro_stable_sort::<BakeTime>(32, times, bc.tmp_arr() as *mut BakeTime, count, |a, b| {
            cmp_bake_time(*a, *b) < 0
        })
    };
    Ok(())
}

// ufbx.c:26847-26968 `ufbxi_finalize_bake_times`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn finalize_bake_times(
    bc: &BakeContext,
    p_dst: *mut BakeTimeList,
) -> Result<(), Fail> {
    if bc.layer_weight_times_view().count() > 0 {
        ufbxi_check_err!(
            bc.error_view(),
            // SAFETY: `bc.tmp_times` is `bc`'s own buffer, and
            // `bc.layer_weight_times` is `bc`'s own list, whose base addresses
            // exactly the `count` elements copied from.
            !unsafe {
                push_copy::<BakeTime>(
                    bc.tmp_times_mut_ptr(),
                    bc.layer_weight_times_view().count(),
                    bc.layer_weight_times_view().data(),
                )
            }
            .is_null(),
            "((ufbxi_bake_time*)ufbxi_push_size_copy((&bc->tmp_times), sizeof(ufbxi_bake_time), (bc->layer_weight_times.count), (bc->layer_weight_times.data)))"
        );
    }

    if bc.tmp_times_view().num_items() == 0 {
        ufbxi_check_err!(
            bc.error_view(),
            bake_push_time(bc, bc.time_begin(), 0),
            "ufbxi_bake_push_time(bc, bc->time_begin, 0)"
        );
        ufbxi_check_err!(
            bc.error_view(),
            bake_push_time(bc, bc.time_end(), 0),
            "ufbxi_bake_push_time(bc, bc->time_end, 0)"
        );
    }

    let mut num_times: usize = bc.tmp_times_view().num_items();
    let times: *mut BakeTime = bc
        .tmp_prop_view()
        .push_pop::<BakeTime>(bc.tmp_times_view(), num_times);
    ufbxi_check_err!(bc.error_view(), !times.is_null(), "times");

    // SAFETY: `times` is the non-null (checked) `num_times`-element
    // `ufbxi_bake_time` run just popped into `bc.tmp_prop`, which is what
    // `sort_bake_times` requires.
    unsafe { sort_bake_times(bc, times, num_times) }?;

    // Deduplicate times
    if num_times > 0 {
        let mut dst: usize = 0;
        // SAFETY: `times` addresses `num_times` live `ufbxi_bake_time`s and
        // `num_times > 0` here, so slot 0 is in bounds.
        let mut prev: BakeTime = unsafe { *times.add(0) };
        let mut src: usize = 1;
        while src < num_times {
            // SAFETY: `src < num_times`, in bounds of the `times` run.
            let mut next: BakeTime = unsafe { *times.add(src) };
            // Merge keys with the same time and step flags `(0x1, 0x2)`
            if next.time == prev.time {
                if ((next.flags ^ prev.flags) & 0x3) == 0 {
                    prev.flags |= next.flags;
                    // C: `continue` — the `for` increment still runs.
                    src += 1;
                    continue;
                } else if (prev.flags & BakedKeyFlags::STEP_LEFT.raw()) != 0 {
                    next.flags |= BakedKeyFlags::STEP_KEY.raw();
                } else if (next.flags & BakedKeyFlags::STEP_RIGHT.raw()) != 0 {
                    prev.flags |= BakedKeyFlags::STEP_KEY.raw();
                }
            }

            // SAFETY: `dst` trails `src` (it advances at most once per loop
            // iteration, starting one behind), so `dst < num_times` and the slot
            // is in bounds of the `times` run.
            unsafe { *times.add(dst) = prev };
            dst += 1;
            prev = next;
            src += 1;
        }
        // SAFETY: as above — the loop leaves `dst < num_times`, so this final
        // slot is in bounds of the `times` run.
        unsafe { *times.add(dst) = prev };
        dst += 1;
        num_times = dst;
    }

    // Cull too close resampled keys, these may arise during merging multiple times
    if num_times > 0 {
        let min_dist: f64 = 0.25 / bc.opts_view().resample_rate();
        let keep_flags: u32 = BakedKeyFlags::STEP_LEFT.raw()
            | BakedKeyFlags::STEP_RIGHT.raw()
            | BakedKeyFlags::STEP_KEY.raw()
            | BakedKeyFlags::KEYFRAME.raw();

        let mut dst: usize = 0;
        for src in 0..num_times {
            // SAFETY: `src < num_times`, in bounds of the `times` run.
            let cur: BakeTime = unsafe { *times.add(src) };
            let mut delta: f64 = math::INFINITY;

            let mut keep: bool = true;
            if (cur.flags & keep_flags) == 0 {
                if dst > 0 {
                    // SAFETY: `dst > 0` (checked) and `dst <= src < num_times`,
                    // so `dst - 1` is in bounds of the `times` run.
                    delta = cur.time - unsafe { (*times.add(dst - 1)).time };
                }
                if src + 1 < num_times {
                    // SAFETY: `src + 1 < num_times` (checked), in bounds of the
                    // `times` run.
                    delta = math::fmin(delta, unsafe { (*times.add(src + 1)).time } - cur.time);
                }
                if delta < min_dist {
                    keep = false;
                }
            }
            if keep {
                // SAFETY: `dst <= src < num_times`, in bounds of the `times` run.
                unsafe { *times.add(dst) = cur };
                dst += 1;
            }
        }
        num_times = dst;
    }

    // Enforce maximum sample rate
    if bc.opts_view().maximum_sample_rate() > 0.0 {
        let epsilon: f64 = 0.0078125 / bc.opts_view().maximum_sample_rate();
        let sample_rate: f64 = bc.opts_view().maximum_sample_rate();
        let max_interval: f64 = 1.0 / bc.opts_view().maximum_sample_rate();
        let min_interval: f64 = 1.0 / bc.opts_view().maximum_sample_rate() - epsilon;
        let mut dst: usize = 0;
        let mut src: usize = 0;

        // Pre-expand constant keyframes
        for i in 0..num_times {
            // SAFETY (this condition): `i < num_times`, in bounds of the `times`
            // run.
            if (unsafe { (*times.add(i)).flags }
                & (BakedKeyFlags::STEP_LEFT.raw() | BakedKeyFlags::STEP_RIGHT.raw()))
                != 0
            {
                // SAFETY: as above.
                let sign: f64 =
                    if (unsafe { (*times.add(i)).flags } & BakedKeyFlags::STEP_LEFT.raw()) != 0 {
                        -1.0
                    } else {
                        1.0
                    };
                // SAFETY: as above.
                let mut time: f64 = unsafe { (*times.add(i)).time } + sign * max_interval;
                if i > 0 {
                    // SAFETY: `i > 0` (checked) and `i < num_times`, so `i - 1`
                    // is in bounds of the `times` run.
                    time = math::fmax(time, unsafe { (*times.add(i - 1)).time });
                }
                if i + 1 < num_times {
                    // SAFETY: `i + 1 < num_times` (checked), in bounds of the
                    // `times` run.
                    time = math::fmin(time, unsafe { (*times.add(i + 1)).time });
                }
                // SAFETY: `i < num_times`, in bounds of the `times` run, and the
                // pair of writes updates that one element in place.
                unsafe {
                    (*times.add(i)).time = time;
                    (*times.add(i)).flags = BakedKeyFlags::REDUCED.raw();
                }
            }
        }

        // C: `ufbxi_bake_time prev_time = { -UFBX_INFINITY };`
        let mut prev_time: BakeTime = BakeTime {
            time: -math::INFINITY,
            flags: 0,
        };
        while src < num_times {
            // SAFETY: `src < num_times` (loop condition), in bounds of the
            // `times` run.
            let src_time: BakeTime = unsafe { *times.add(src) };
            src += 1;

            let start_src: usize = src;
            // C: `ufbxi_bake_time next_time;` — both members assigned below.
            let mut next_time: BakeTime = BakeTime {
                time: 0.0,
                flags: 0,
            };
            next_time.time = math::ceil(src_time.time * sample_rate - epsilon) / sample_rate;
            next_time.flags = BakedKeyFlags::REDUCED.raw();
            // SAFETY (this condition): `src < num_times` is checked first and
            // `&&` short-circuits, so the slot is in bounds of the `times` run.
            while src < num_times && unsafe { (*times.add(src)).time } <= next_time.time + epsilon {
                src += 1;
            }

            if src != start_src || src_time.time - prev_time.time <= min_interval {
                prev_time = next_time;
            } else {
                prev_time = src_time;
            }

            // SAFETY (this condition): `dst != 0` is established first and `||`
            // short-circuits, and `dst` trails `src <= num_times`, so `dst - 1`
            // is in bounds of the `times` run.
            if dst == 0 || prev_time.time > unsafe { (*times.add(dst - 1)).time } {
                // SAFETY: `dst` trails `src`, which the loop condition keeps
                // below `num_times`, so this slot is in bounds of the run.
                unsafe { *times.add(dst) = prev_time };
                dst += 1;
            }
        }

        num_times = dst;
    }

    if num_times > 0 {
        // SAFETY (this condition and the call it guards): `num_times > 0`, so
        // slot 0 is in bounds of the `times` run.
        if unsafe { (*times.add(0)).time } < bc.time_min() {
            bc.set_time_min(unsafe { (*times.add(0)).time });
        }
        // SAFETY (this condition and the call it guards): `num_times > 0`, so
        // `num_times - 1` does not underflow and is the run's last slot.
        if unsafe { (*times.add(num_times - 1)).time } > bc.time_max() {
            bc.set_time_max(unsafe { (*times.add(num_times - 1)).time });
        }
    }

    // SAFETY: `p_dst` points to the caller's live `ufbxi_bake_time_list` slot —
    // this `unsafe fn`'s contract — and the pair of writes retargets it at the
    // deduplicated run.
    unsafe {
        (*p_dst).data = times;
        (*p_dst).count = num_times;
    }

    Ok(())
}

// ufbx.c:26970 `#define ufbxi_add_epsilon(a, epsilon)`
#[cfg(feature = "baking")]
#[inline(always)]
pub(crate) fn add_epsilon(a: f64, epsilon: f64) -> f64 {
    if a > 0.0 {
        a * epsilon
    } else {
        a / epsilon
    }
}

// ufbx.c:26971 `#define ufbxi_sub_epsilon(a, epsilon)`
#[cfg(feature = "baking")]
#[inline(always)]
pub(crate) fn sub_epsilon(a: f64, epsilon: f64) -> f64 {
    if a > 0.0 {
        a / epsilon
    } else {
        a * epsilon
    }
}

// ufbx.c:26973-27015 `ufbxi_postprocess_step`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn postprocess_step(
    bc: &BakeContext,
    prev_time: f64,
    next_time: f64,
    p_time: *mut f64,
    flags: BakedKeyFlags,
) -> bool {
    ufbxi_dev_assert!(
        (flags.raw() & (BakedKeyFlags::STEP_LEFT.raw() | BakedKeyFlags::STEP_RIGHT.raw())) != 0
    );
    let left: bool = (flags.raw() & BakedKeyFlags::STEP_LEFT.raw()) != 0;

    let mut step: f64 = 0.001;
    // C: `1.0 + UFBX_FLT_EPSILON * 4.0f` — the multiply is `float` arithmetic,
    // widened to `double` only for the add.
    let mut epsilon: f64 = 1.0 + (math::FLT_EPSILON * 4.0f32) as f64;

    // SAFETY: `p_time` points to the caller's live `double` slot — this
    // `unsafe fn`'s contract.
    let mut time: f64 = unsafe { *p_time };
    // C: `switch (bc->opts.step_handling)` — an if-ladder over the discriminant
    // value, with the trailing `else` standing in for C's `default:` arm, which
    // is reachable because `bc->opts` is a verbatim copy of unvalidated user
    // options. The `as u32` read does not make an out-of-range user value safe:
    // `step_handling` is typed as the generated `BakeStepHandling` enum, so an
    // invalid discriminant is already materialized by the options copy in
    // `api::bake_anim` — the generated-type/read-idiom question is tree-wide
    // (same shape as `subdivision::subdivide_mesh_imp`'s `opts.boundary`) and
    // belongs to the generator, per PORTING.md ground rule 0.
    let step_handling: u32 = bc.opts_view().step_handling() as u32;
    if step_handling == BakeStepHandling::Default as u32 {
        // C: `break;`
    } else if step_handling == BakeStepHandling::CustomDuration as u32 {
        step = bc.opts_view().step_custom_duration();
        epsilon = 1.0 + bc.opts_view().step_custom_epsilon();
    } else if step_handling == BakeStepHandling::IdenticalTime as u32 {
        return true;
    } else if step_handling == BakeStepHandling::AdjacentDouble as u32 {
        if left {
            time = math::nextafter(time, -math::INFINITY);
            // SAFETY: `p_time` is the caller's live `double` slot.
            unsafe { *p_time = time };
            return time > prev_time;
        } else {
            time = math::nextafter(time, math::INFINITY);
            // SAFETY: `p_time` is the caller's live `double` slot.
            unsafe { *p_time = time };
            return time < next_time;
        }
    } else if step_handling == BakeStepHandling::Ignore as u32 {
        return false;
    } else {
        ufbxi_unreachable!("Unhandled bake step handling");
        return false;
    }

    if left {
        let min_time: f64 = math::fmax(prev_time + step, add_epsilon(prev_time, epsilon));
        time = math::fmin(time - step, sub_epsilon(time, epsilon));
        // SAFETY: `p_time` is the caller's live `double` slot.
        unsafe { *p_time = time };
        time > min_time
    } else {
        let max_time: f64 = math::fmin(next_time - step, sub_epsilon(next_time, epsilon));
        time = math::fmax(time + step, add_epsilon(time, epsilon));
        // SAFETY: `p_time` is the caller's live `double` slot.
        unsafe { *p_time = time };
        time < max_time
    }
}

// ufbx.c:27017-27097 `ufbxi_bake_postprocess_vec3`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn bake_postprocess_vec3(
    bc: &BakeContext,
    p_dst: *mut List<BakedVec3>,
    p_constant: *mut bool,
    mut src: List<BakedVec3>,
) -> Result<(), Fail> {
    if src.count == 0 {
        return Ok(());
    }

    // C: `src.data[i]` — `ufbx_baked_vec3_list::data` is non-const in C.
    let data: *mut BakedVec3 = src.data as *mut BakedVec3;

    // Offset times
    if bc.ktime_offset() != 0.0 {
        // SAFETY: `bc.scene()` is the source `ufbx_scene` `bake_anim_imp` stored
        // into `bc`, live for the bake.
        let scale: f64 = unsafe { (*bc.scene()).metadata.ktime_second } as f64;
        let offset: f64 = bc.ktime_offset();
        for i in 0..src.count {
            // SAFETY: this `unsafe fn` requires `src` to describe a live,
            // writable run of `src.count` `ufbx_baked_vec3`s, and `i < src.count`.
            unsafe {
                (*data.add(i)).time = math::rint((*data.add(i)).time * scale + offset) / scale
            };
        }
    }

    // Postprocess stepped tangents
    {
        let mut dst: usize = 0;
        // SAFETY: `src.count != 0` (the early return above), so slot 0 is in
        // bounds of the run `src` describes.
        let mut prev_time: f64 = unsafe { (*data.add(0)).time };
        for i in 0..src.count {
            // SAFETY: `i < src.count`, in bounds of the run; `BakedVec3` is
            // `Copy`-shaped plain data, so reading it out leaves the slot valid.
            let mut cur: BakedVec3 = unsafe { ptr::read(data.add(i)) };
            let next_time: f64 = if i + 1 < src.count {
                // SAFETY: `i + 1 < src.count` (checked), in bounds of the run.
                unsafe { (*data.add(i + 1)).time }
            } else {
                math::INFINITY
            };
            let mut keep: bool = true;
            if (cur.flags.raw()
                & (BakedKeyFlags::STEP_LEFT.raw() | BakedKeyFlags::STEP_RIGHT.raw()))
                != 0
            {
                // SAFETY: `postprocess_step` needs a live `double` slot, and
                // `cur.time` is a field of this live local.
                keep = unsafe {
                    postprocess_step(bc, prev_time, next_time, &raw mut cur.time, cur.flags)
                };
            }
            if keep {
                // C: `src.data[dst] = cur; dst++; prev_time = cur.time;`
                let cur_time: f64 = cur.time;
                // SAFETY: `dst <= i < src.count`, in bounds of the run.
                unsafe { ptr::write(data.add(dst), cur) };
                dst += 1;
                prev_time = cur_time;
            }
        }
        src.count = dst;
    }

    if bc.opts_view().key_reduction_enabled() {
        let threshold: f64 =
            bc.opts_view().key_reduction_threshold() * bc.opts_view().key_reduction_threshold();
        for _pass in 0..bc.opts_view().key_reduction_passes() {
            let mut dst: usize = 1;
            let mut i: usize = 1;
            while i < src.count {
                // SAFETY: `i >= 1` and `i < src.count` (loop condition), so both
                // `i - 1` and `i` are in bounds of the run `src` describes;
                // `BakedVec3` is plain data, so reading leaves the slots valid.
                let prev: BakedVec3 = unsafe { ptr::read(data.add(i - 1)) };
                // SAFETY: as above, for slot `i`.
                let cur: BakedVec3 = unsafe { ptr::read(data.add(i)) };
                if i + 1 < src.count {
                    // SAFETY: `i + 1 < src.count` (checked), in bounds of the run.
                    let next: BakedVec3 = unsafe { ptr::read(data.add(i + 1)) };
                    let delta: f64 = (cur.time - prev.time) / (next.time - prev.time);
                    let tmp: Vec3 = lerp3(prev.value, next.value, delta as Real);
                    let mut error: f64 = 0.0;
                    error +=
                        (tmp.x as f64 - cur.value.x as f64) * (tmp.x as f64 - cur.value.x as f64);
                    error +=
                        (tmp.y as f64 - cur.value.y as f64) * (tmp.y as f64 - cur.value.y as f64);
                    error +=
                        (tmp.z as f64 - cur.value.z as f64) * (tmp.z as f64 - cur.value.z as f64);
                    if error <= threshold {
                        // SAFETY: `i + 1 < src.count` (checked) and `dst <= i`,
                        // so both slots are in bounds of the run.
                        unsafe { ptr::write(data.add(dst), ptr::read(data.add(i + 1))) };
                        i += 1;
                        dst += 1;
                        // C: `continue` — the `for` increment still runs.
                        i += 1;
                        continue;
                    }
                }

                // SAFETY: `i < src.count` (loop condition) and `dst <= i`, so
                // both slots are in bounds of the run.
                unsafe { ptr::write(data.add(dst), ptr::read(data.add(i))) };
                dst += 1;
                i += 1;
            }
            if dst == src.count {
                break;
            }
            src.count = dst;
        }
    }

    let mut constant: bool = true;
    // SAFETY: `data` addresses the original run, which holds at least one
    // element (the `src.count == 0` early return above), so slot 0 is in
    // bounds. The passes above only shrink `src.count` — they never move or
    // reallocate the run — so slot 0 stays valid regardless of how many keys
    // they keep.
    let ref_: Vec3 = unsafe { (*data.add(0)).value };
    for i in 1..src.count {
        // SAFETY: `i < src.count`, in bounds of the run.
        let v: Vec3 = unsafe { (*data.add(i)).value };
        if v.x != ref_.x || v.y != ref_.y || v.z != ref_.z {
            constant = false;
            break;
        }
    }
    // SAFETY: `p_constant` points to the caller's live `bool` slot — this
    // `unsafe fn`'s contract.
    unsafe { *p_constant = constant };

    // SAFETY: `p_dst` points to the caller's live `ufbx_baked_vec3_list` slot —
    // this `unsafe fn`'s contract.
    unsafe { (*p_dst).count = src.count };
    // SAFETY: as above for `p_dst`; `data` addresses `src.count` live elements,
    // which is the run `push_copy` reads, and `bc.result` is `bc`'s own buffer.
    unsafe { (*p_dst).data = push_copy::<BakedVec3>(bc.result_mut_ptr(), src.count, data) };
    ufbxi_check_err!(
        bc.error_view(),
        // SAFETY: as above — `p_dst` is the caller's live list slot, just written.
        !unsafe { (*p_dst).data }.is_null(),
        "p_dst->data"
    );

    Ok(())
}

// ufbx.c:27099-27199 `ufbxi_bake_postprocess_quat`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn bake_postprocess_quat(
    bc: &BakeContext,
    p_dst: *mut List<BakedQuat>,
    p_constant: *mut bool,
    mut src: List<BakedQuat>,
) -> Result<(), Fail> {
    if src.count == 0 {
        return Ok(());
    }

    let data: *mut BakedQuat = src.data as *mut BakedQuat;

    // Offset times
    if bc.ktime_offset() != 0.0 {
        // SAFETY: `bc.scene()` is the source `ufbx_scene` `bake_anim_imp` stored
        // into `bc`, live for the bake.
        let scale: f64 = unsafe { (*bc.scene()).metadata.ktime_second } as f64;
        let offset: f64 = bc.ktime_offset();
        for i in 0..src.count {
            // SAFETY: this `unsafe fn` requires `src` to describe a live,
            // writable run of `src.count` `ufbx_baked_quat`s, and `i < src.count`.
            unsafe {
                (*data.add(i)).time = math::rint((*data.add(i)).time * scale + offset) / scale
            };
        }
    }

    // Postprocess stepped tangents
    {
        let mut dst: usize = 0;
        // SAFETY: `src.count != 0` (the early return above), so slot 0 is in
        // bounds of the run `src` describes.
        let mut prev_time: f64 = unsafe { (*data.add(0)).time };
        for i in 0..src.count {
            // SAFETY: `i < src.count`, in bounds of the run; `BakedQuat` is
            // plain data, so reading it out leaves the slot valid.
            let mut cur: BakedQuat = unsafe { ptr::read(data.add(i)) };
            let next_time: f64 = if i + 1 < src.count {
                // SAFETY: `i + 1 < src.count` (checked), in bounds of the run.
                unsafe { (*data.add(i + 1)).time }
            } else {
                math::INFINITY
            };
            let mut keep: bool = true;
            if (cur.flags.raw()
                & (BakedKeyFlags::STEP_LEFT.raw() | BakedKeyFlags::STEP_RIGHT.raw()))
                != 0
            {
                // SAFETY: `postprocess_step` needs a live `double` slot, and
                // `cur.time` is a field of this live local.
                keep = unsafe {
                    postprocess_step(bc, prev_time, next_time, &raw mut cur.time, cur.flags)
                };
            }
            if keep {
                prev_time = cur.time;
                // SAFETY: `dst <= i < src.count`, in bounds of the run.
                unsafe { ptr::write(data.add(dst), cur) };
                dst += 1;
            }
        }
        src.count = dst;
    }

    // Fix quaternion antipodality
    for i in 1..src.count {
        // SAFETY: `i >= 1` and `i < src.count`, so both `i` and `i - 1` are in
        // bounds of the run `src` describes.
        unsafe {
            (*data.add(i)).value =
                quat_fix_antipodal((*data.add(i)).value, (*data.add(i - 1)).value)
        };
    }

    if bc.opts_view().key_reduction_enabled() {
        let threshold: f64 =
            bc.opts_view().key_reduction_threshold() * bc.opts_view().key_reduction_threshold();
        for _pass in 0..bc.opts_view().key_reduction_passes() {
            let mut dst: usize = 1;
            let mut i: usize = 1;
            while i < src.count {
                // SAFETY: `i >= 1` and `i < src.count` (loop condition), so both
                // `i - 1` and `i` are in bounds of the run `src` describes;
                // `BakedQuat` is plain data, so reading leaves the slots valid.
                let prev: BakedQuat = unsafe { ptr::read(data.add(i - 1)) };
                // SAFETY: as above, for slot `i`.
                let cur: BakedQuat = unsafe { ptr::read(data.add(i)) };
                if i + 1 < src.count {
                    // SAFETY: `i + 1 < src.count` (checked), in bounds of the run.
                    let next: BakedQuat = unsafe { ptr::read(data.add(i + 1)) };
                    let delta: f64 = (cur.time - prev.time) / (next.time - prev.time);
                    let mut error: f64 = 0.0;

                    if bc.opts_view().key_reduction_rotation() {
                        let tmp: Quat = quat_slerp(prev.value, next.value, delta as Real);
                        error += (tmp.x as f64 - cur.value.x as f64)
                            * (tmp.x as f64 - cur.value.x as f64);
                        error += (tmp.y as f64 - cur.value.y as f64)
                            * (tmp.y as f64 - cur.value.y as f64);
                        error += (tmp.z as f64 - cur.value.z as f64)
                            * (tmp.z as f64 - cur.value.z as f64);
                        error += (tmp.w as f64 - cur.value.w as f64)
                            * (tmp.w as f64 - cur.value.w as f64);
                    } else {
                        error += (prev.value.x as f64 - cur.value.x as f64)
                            * (prev.value.x as f64 - cur.value.x as f64);
                        error += (prev.value.y as f64 - cur.value.y as f64)
                            * (prev.value.y as f64 - cur.value.y as f64);
                        error += (prev.value.z as f64 - cur.value.z as f64)
                            * (prev.value.z as f64 - cur.value.z as f64);
                        error += (prev.value.w as f64 - cur.value.w as f64)
                            * (prev.value.w as f64 - cur.value.w as f64);
                        error += (next.value.x as f64 - cur.value.x as f64)
                            * (next.value.x as f64 - cur.value.x as f64);
                        error += (next.value.y as f64 - cur.value.y as f64)
                            * (next.value.y as f64 - cur.value.y as f64);
                        error += (next.value.z as f64 - cur.value.z as f64)
                            * (next.value.z as f64 - cur.value.z as f64);
                        error += (next.value.w as f64 - cur.value.w as f64)
                            * (next.value.w as f64 - cur.value.w as f64);
                        error *= 0.5;
                    }

                    if error <= threshold {
                        // SAFETY: `i + 1 < src.count` (checked) and `dst <= i`,
                        // so both slots are in bounds of the run.
                        unsafe { ptr::write(data.add(dst), ptr::read(data.add(i + 1))) };
                        i += 1;
                        dst += 1;
                        // C: `continue` — the `for` increment still runs.
                        i += 1;
                        continue;
                    }
                }

                // SAFETY: `i < src.count` (loop condition) and `dst <= i`, so
                // both slots are in bounds of the run.
                unsafe { ptr::write(data.add(dst), ptr::read(data.add(i))) };
                dst += 1;
                i += 1;
            }
            if dst == src.count {
                break;
            }
            src.count = dst;
        }
    }

    let mut constant: bool = true;
    // SAFETY: `data` addresses the original run, which holds at least one
    // element (the `src.count == 0` early return above), so slot 0 is in
    // bounds. The passes above only shrink `src.count` — they never move or
    // reallocate the run — so slot 0 stays valid regardless of how many keys
    // they keep.
    let ref_: Quat = unsafe { (*data.add(0)).value };
    for i in 1..src.count {
        // SAFETY: `i < src.count`, in bounds of the run.
        let v: Quat = unsafe { (*data.add(i)).value };
        if v.x != ref_.x || v.y != ref_.y || v.z != ref_.z || v.w != ref_.w {
            constant = false;
            break;
        }
    }
    // SAFETY: `p_constant` points to the caller's live `bool` slot — this
    // `unsafe fn`'s contract.
    unsafe { *p_constant = constant };

    // SAFETY: `p_dst` points to the caller's live `ufbx_baked_quat_list` slot —
    // this `unsafe fn`'s contract.
    unsafe { (*p_dst).count = src.count };
    // SAFETY: as above for `p_dst`; `data` addresses `src.count` live elements,
    // which is the run `push_copy` reads, and `bc.result` is `bc`'s own buffer.
    unsafe { (*p_dst).data = push_copy::<BakedQuat>(bc.result_mut_ptr(), src.count, data) };
    ufbxi_check_err!(
        bc.error_view(),
        // SAFETY: as above — `p_dst` is the caller's live list slot, just written.
        !unsafe { (*p_dst).data }.is_null(),
        "p_dst->data"
    );

    Ok(())
}

// ufbx.c:27201-27210 `ufbxi_bake_time_sample_time`
// C: `ufbxi_forceinline`.
#[cfg(feature = "baking")]
#[inline(always)]
pub(crate) fn bake_time_sample_time(time: BakeTime) -> f64 {
    // Move an infinitesimal step for stepped tangents
    if (time.flags & (BakedKeyFlags::STEP_LEFT.raw() | BakedKeyFlags::STEP_RIGHT.raw())) != 0 {
        let dir: f64 = if (time.flags & BakedKeyFlags::STEP_LEFT.raw()) != 0 {
            -math::INFINITY
        } else {
            math::INFINITY
        };
        math::nextafter(time.time, dir)
    } else {
        time.time
    }
}

// ufbx.c:27212-27231 `ufbxi_push_resampled_times`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn push_resampled_times(
    bc: &BakeContext,
    p_keys: *const List<BakedVec3>,
) -> Result<(), Fail> {
    // C: `ufbx_baked_vec3_list keys = *p_keys;`
    // SAFETY: `p_keys` points to the caller's live `ufbx_baked_vec3_list` — this
    // `unsafe fn`'s contract — and the list is plain data, so reading it out by
    // value leaves the caller's copy valid.
    let keys: List<BakedVec3> = unsafe { ptr::read(p_keys) };

    let times: *mut BakeTime = bc.tmp_times_view().push::<BakeTime>(keys.count);
    ufbxi_check_err!(bc.error_view(), !times.is_null(), "times");
    for i in 0..keys.count {
        // SAFETY: `keys` describes a live run of `keys.count` baked keys, and
        // `i < keys.count`.
        let flags: BakedKeyFlags = unsafe { (*keys.data.add(i)).flags };
        // SAFETY: as above, for the same key's `time`.
        let mut time: f64 = unsafe { (*keys.data.add(i)).time };
        // SAFETY (this condition): `i + 1 < keys.count` is established first and
        // `&&` short-circuits, so that slot is in bounds of the key run.
        if (flags.raw() & BakedKeyFlags::STEP_LEFT.raw()) != 0
            && i + 1 < keys.count
            && (unsafe { (*keys.data.add(i + 1)).flags }.raw() & BakedKeyFlags::STEP_KEY.raw()) != 0
        {
            // SAFETY: as above — the condition established `i + 1 < keys.count`.
            time = unsafe { (*keys.data.add(i + 1)).time };
        // SAFETY (this condition): `i > 0` is established first and `&&`
        // short-circuits, so `i - 1` does not underflow and is in bounds.
        } else if (flags.raw() & BakedKeyFlags::STEP_RIGHT.raw()) != 0
            && i > 0
            && (unsafe { (*keys.data.add(i - 1)).flags }.raw() & BakedKeyFlags::STEP_KEY.raw()) != 0
        {
            // SAFETY: as above — the condition established `i > 0`.
            time = unsafe { (*keys.data.add(i - 1)).time };
        }
        // SAFETY: `times` is the non-null (checked) `keys.count`-element run just
        // pushed onto `bc.tmp_times`, and `i < keys.count`; the pair of writes
        // initializes that one element.
        unsafe {
            (*times.add(i)).time = time;
            (*times.add(i)).flags = flags.raw() & 0x7;
        }
    }

    Ok(())
}

// ufbx.c:27233-27490 `ufbxi_bake_node_imp`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn bake_node_imp(
    bc: &BakeContext,
    element_id: u32,
    props: *mut BakeProp,
    count: usize,
) -> Result<(), Fail> {
    ufbx_assert!(!bc.baked_nodes().is_null() && !bc.nodes_to_bake().is_null());

    // SAFETY: `bc.scene()` is the source `ufbx_scene` `bake_anim_imp` stored into
    // `bc`, live for the bake; this `unsafe fn` requires `element_id` to be one
    // of that scene's element ids, so the slot is in bounds of `elements` and
    // holds a live element pointer.
    let node: *mut UfbxNode =
        unsafe { *((*bc.scene()).elements.data as *const *mut UfbxNode).add(element_id as usize) };
    // SAFETY: `node` is that live scene element.
    ufbxi_dev_assert!(unsafe { (*node).element.type_ } as u32 == ElementType::Node as u32);

    let mut complex_translation: bool = false;
    let mut complex_rotation: bool = false;

    for i in 0..COMPLEX_TRANSLATION_PROPS.0.len() {
        let name: *const u8 = COMPLEX_TRANSLATION_PROPS.0[i];
        // `find_prop` matches on the interned run's ADDRESS, so borrow `name`'s
        // own bytes: the table entries are NUL-terminated `sp::*` statics.
        // SAFETY: `name` is one of those `'static` NUL-terminated interned
        // string statics, so `strlen` stays inside it and the slice it measures
        // borrows only those bytes.
        let name_bytes: &[u8] = unsafe { core::slice::from_raw_parts(name, strlen(name)) };
        let prop: Option<&PropView> = find_prop(
            // SAFETY: the projection addresses the live scene node's own
            // `element.props`, which outlives the borrow this view carries.
            unsafe { PropsView::from_ptr(&raw mut (*node).element.props) },
            name_bytes,
        );
        // C: `prop->value_vec3` — the `ufbx_prop` value union's 3-real view
        // over `value_vec4`.
        if prop.is_some_and(|prop| !is_vec3_zero(prop.value_vec3())) {
            complex_translation = true;
        }
        // C: `ufbxi_for(ufbxi_bake_prop, bprop, props, count)`
        // SAFETY: this `unsafe fn` requires `props` to address `count` live
        // `ufbxi_bake_prop`s, which is what the iterator walks.
        for bprop in unsafe { SliceViewIter::<BakeProp>::from_raw_parts(props, count) } {
            if bprop.prop_name() == name {
                complex_translation = true;
            }
        }
    }

    for i in 0..COMPLEX_ROTATION_PROPS.0.len() {
        let name: *const u8 = COMPLEX_ROTATION_PROPS.0[i];
        // SAFETY: as above — `props` addresses `count` live `ufbxi_bake_prop`s.
        for bprop in unsafe { SliceViewIter::<BakeProp>::from_raw_parts(props, count) } {
            if bprop.prop_name() == name {
                complex_rotation = true;
            }
        }
    }

    // C: `ufbxi_bake_time_list times_t, times_r, times_s;` — each is filled in
    // by the `ufbxi_finalize_bake_times` call below.
    // SAFETY: `BakeTimeList` is a pointer/length pair, and an all-zero pattern
    // (null pointer, zero count) is a valid inhabitant of it.
    let mut times_t: BakeTimeList = unsafe { MaybeUninit::zeroed().assume_init() };
    // SAFETY: as above.
    let mut times_r: BakeTimeList = unsafe { MaybeUninit::zeroed().assume_init() };
    // SAFETY: as above.
    let mut times_s: BakeTimeList = unsafe { MaybeUninit::zeroed().assume_init() };

    // Translation
    let mut resample_translation: bool = false;

    // Account for the _resampled_ scale helper scale animation to keep the
    // translation scale consistent with the parent scaling.
    let mut scale_helper_t: *mut BakedNode = ptr::null_mut();
    let mut constant_scale_t: Vec3 = Vec3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    // C: `node->parent` / `node->parent->scale_helper` — short-circuit chain,
    // so the inner loads only happen when the outer pointer is non-NULL.
    // SAFETY: `node` is a live scene node, whose `parent` is its own nullable
    // node ref that `opt_ptr` reads as the element pointer it is.
    let parent: *mut UfbxNode = unsafe { opt_ptr(&raw const (*node).parent) };
    let parent_scale_helper: *mut UfbxNode = if !parent.is_null() {
        // SAFETY: `parent` is non-null (checked) and is a live scene node, whose
        // `scale_helper` is its own nullable node ref.
        unsafe { opt_ptr(&raw const (*parent).scale_helper) }
    } else {
        ptr::null_mut()
    };
    // SAFETY (this condition): `node` is a live scene node.
    if !unsafe { (*node).is_scale_helper } && !parent.is_null() && !parent_scale_helper.is_null() {
        // SAFETY: `parent_scale_helper` is non-null (checked) and is a live scene
        // node, so its `typed_id` indexes `bc.baked_nodes()`, which `bake_anim`
        // sizes with one slot per node of the scene.
        scale_helper_t = unsafe {
            *bc.baked_nodes()
                .add((*parent_scale_helper).element.typed_id as usize)
        };
        if !scale_helper_t.is_null() {
            // SAFETY: `scale_helper_t` is non-null (checked) and points to the
            // `ufbx_baked_node` already baked for that helper.
            if !unsafe { (*scale_helper_t).constant_scale } {
                resample_translation = true;
            }
            // SAFETY: as above; `scale_keys` is that baked node's own key list,
            // which is what `push_resampled_times` reads.
            unsafe { push_resampled_times(bc, &raw const (*scale_helper_t).scale_keys) }?;
        } else {
            // SAFETY: `parent_scale_helper` is a live scene node.
            constant_scale_t = unsafe { (*parent_scale_helper).inherit_scale };
        }
    }

    if complex_translation {
        // C: `ufbxi_for(ufbxi_bake_prop, prop, props, count)`
        // SAFETY: `props` addresses `count` live `ufbxi_bake_prop`s — this
        // `unsafe fn`'s contract.
        for prop in unsafe { SliceViewIter::<BakeProp>::from_raw_parts(props, count) } {
            // Literally any transform related property can affect complex translation
            // SAFETY: `TRANSFORM_PROPS` is a `'static` array of interned string
            // pointers, so its base addresses exactly `len()` readable entries.
            if unsafe {
                in_list(
                    TRANSFORM_PROPS.0.as_ptr(),
                    TRANSFORM_PROPS.0.len(),
                    prop.prop_name(),
                )
            } {
                let resample_linear: bool =
                    resample_translation || prop.prop_name() != sp::Lcl_Translation.as_ptr();
                let key_flag: u32 = if prop.prop_name() == sp::Lcl_Translation.as_ptr() {
                    BakedKeyFlags::KEYFRAME.raw()
                } else {
                    0
                };
                // SAFETY: `prop` is one of the live bake props, whose
                // `anim_value` is the `ufbx_anim_value` it was collected from.
                unsafe { bake_times(bc, prop.anim_value(), resample_linear, key_flag) }?;
            }
        }
    } else {
        // SAFETY: `props` addresses `count` live `ufbxi_bake_prop`s.
        for prop in unsafe { SliceViewIter::<BakeProp>::from_raw_parts(props, count) } {
            if prop.prop_name() == sp::Lcl_Translation.as_ptr() {
                // SAFETY: `prop.anim_value()` is the live `ufbx_anim_value` this
                // bake prop was collected from.
                unsafe {
                    bake_times(
                        bc,
                        prop.anim_value(),
                        resample_translation,
                        BakedKeyFlags::KEYFRAME.raw(),
                    )
                }?;
            }
        }
    }

    // SAFETY: `times_t` is a live local `ufbxi_bake_time_list`, which is the
    // output slot `finalize_bake_times` fills in.
    unsafe { finalize_bake_times(bc, &raw mut times_t) }?;

    // Rotation
    if complex_rotation {
        // SAFETY: `props` addresses `count` live `ufbxi_bake_prop`s.
        for prop in unsafe { SliceViewIter::<BakeProp>::from_raw_parts(props, count) } {
            // SAFETY: `COMPLEX_ROTATION_SOURCES` is a `'static` array of interned
            // string pointers, so its base addresses exactly `len()` entries.
            if unsafe {
                in_list(
                    COMPLEX_ROTATION_SOURCES.0.as_ptr(),
                    COMPLEX_ROTATION_SOURCES.0.len(),
                    prop.prop_name(),
                )
            } {
                let resample_linear: bool = !bc.opts_view().no_resample_rotation()
                    || prop.prop_name() != sp::Lcl_Rotation.as_ptr();
                let key_flag: u32 = if prop.prop_name() == sp::Lcl_Rotation.as_ptr() {
                    BakedKeyFlags::KEYFRAME.raw()
                } else {
                    0
                };
                // SAFETY: `prop.anim_value()` is the live `ufbx_anim_value` this
                // bake prop was collected from.
                unsafe { bake_times(bc, prop.anim_value(), resample_linear, key_flag) }?;
            }
        }
    } else {
        // SAFETY: `props` addresses `count` live `ufbxi_bake_prop`s.
        for prop in unsafe { SliceViewIter::<BakeProp>::from_raw_parts(props, count) } {
            if prop.prop_name() == sp::Lcl_Rotation.as_ptr() {
                // SAFETY: `prop.anim_value()` is the live `ufbx_anim_value` this
                // bake prop was collected from.
                unsafe {
                    bake_times(
                        bc,
                        prop.anim_value(),
                        !bc.opts_view().no_resample_rotation(),
                        BakedKeyFlags::KEYFRAME.raw(),
                    )
                }?;
            }
        }
    }
    // SAFETY: `times_r` is a live local `ufbxi_bake_time_list`, the output slot
    // `finalize_bake_times` fills in.
    unsafe { finalize_bake_times(bc, &raw mut times_r) }?;

    // Scaling
    let mut resample_scale: bool = false;

    // Account for the resampled scale
    let mut scale_helper_s: *mut BakedNode = ptr::null_mut();
    let mut constant_scale_s: Vec3 = Vec3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    // C: `node->parent->inherit_scale_node->scale_helper` — short-circuit chain.
    let parent_inherit_scale_node: *mut UfbxNode = if !parent.is_null() {
        // SAFETY: `parent` is non-null (checked) and is a live scene node, whose
        // `inherit_scale_node` is its own nullable node ref.
        unsafe { opt_ptr(&raw const (*parent).inherit_scale_node) }
    } else {
        ptr::null_mut()
    };
    let parent_inherit_scale_helper: *mut UfbxNode = if !parent_inherit_scale_node.is_null() {
        // SAFETY: non-null (checked) and a live scene node, whose `scale_helper`
        // is its own nullable node ref.
        unsafe { opt_ptr(&raw const (*parent_inherit_scale_node).scale_helper) }
    } else {
        ptr::null_mut()
    };
    // SAFETY (this condition): `node` is a live scene node.
    if unsafe { (*node).is_scale_helper }
        && !parent.is_null()
        && !parent_inherit_scale_node.is_null()
        && !parent_inherit_scale_helper.is_null()
    {
        let inherit_helper: *mut UfbxNode = parent_inherit_scale_helper;
        // SAFETY: `inherit_helper` is non-null (checked) and is a live scene
        // node, so its `typed_id` indexes `bc.baked_nodes()`, which `bake_anim`
        // sizes with one slot per node of the scene.
        scale_helper_s = unsafe {
            *bc.baked_nodes()
                .add((*inherit_helper).element.typed_id as usize)
        };
        if !scale_helper_s.is_null() {
            // SAFETY: `scale_helper_s` is non-null (checked) and points to the
            // `ufbx_baked_node` already baked for that helper.
            if !unsafe { (*scale_helper_s).constant_scale } {
                resample_scale = true;
            }
            // SAFETY: as above; `scale_keys` is that baked node's own key list.
            unsafe { push_resampled_times(bc, &raw const (*scale_helper_s).scale_keys) }?;
        } else {
            // SAFETY: `inherit_helper` is a live scene node.
            constant_scale_s = unsafe { (*inherit_helper).local_transform.scale };
        }
    }

    {
        // SAFETY: `props` addresses `count` live `ufbxi_bake_prop`s.
        for prop in unsafe { SliceViewIter::<BakeProp>::from_raw_parts(props, count) } {
            if prop.prop_name() == sp::Lcl_Scaling.as_ptr() {
                // SAFETY: `prop.anim_value()` is the live `ufbx_anim_value` this
                // bake prop was collected from.
                unsafe {
                    bake_times(
                        bc,
                        prop.anim_value(),
                        resample_scale,
                        BakedKeyFlags::KEYFRAME.raw(),
                    )
                }?;
            }
        }
    }
    // SAFETY: `times_s` is a live local `ufbxi_bake_time_list`, the output slot
    // `finalize_bake_times` fills in.
    unsafe { finalize_bake_times(bc, &raw mut times_s) }?;

    // C: `ufbx_baked_vec3_list keys_t; ufbx_baked_quat_list keys_r; ufbx_baked_vec3_list keys_s;`
    // SAFETY: these lists are pointer/length pairs, and an all-zero pattern
    // (null pointer, zero count) is a valid inhabitant of each.
    let mut keys_t: List<BakedVec3> = unsafe { MaybeUninit::zeroed().assume_init() };
    // SAFETY: as above.
    let mut keys_r: List<BakedQuat> = unsafe { MaybeUninit::zeroed().assume_init() };
    // SAFETY: as above.
    let mut keys_s: List<BakedVec3> = unsafe { MaybeUninit::zeroed().assume_init() };

    keys_t.count = times_t.count;
    keys_t.data = bc.tmp_prop_view().push::<BakedVec3>(keys_t.count);
    ufbxi_check_err!(bc.error_view(), !keys_t.data.is_null(), "keys_t.data");

    keys_r.count = times_r.count;
    keys_r.data = bc.tmp_prop_view().push::<BakedQuat>(keys_r.count);
    ufbxi_check_err!(bc.error_view(), !keys_r.data.is_null(), "keys_r.data");

    keys_s.count = times_s.count;
    keys_s.data = bc.tmp_prop_view().push::<BakedVec3>(keys_s.count);
    ufbxi_check_err!(bc.error_view(), !keys_s.data.is_null(), "keys_s.data");

    let keys_t_data: *mut BakedVec3 = keys_t.data as *mut BakedVec3;
    let keys_r_data: *mut BakedQuat = keys_r.data as *mut BakedQuat;
    let keys_s_data: *mut BakedVec3 = keys_s.data as *mut BakedVec3;

    let mut ix_t: usize = 0;
    let mut ix_r: usize = 0;
    let mut ix_s: usize = 0;
    while ix_t < times_t.count || ix_r < times_r.count || ix_s < times_s.count {
        // C: `ufbxi_bake_time bake_time = { UFBX_INFINITY };`
        let mut bake_time: BakeTime = BakeTime {
            time: math::INFINITY,
            flags: 0,
        };
        let mut flags_r: u32 = 0;
        let mut flags_t: u32 = 0;
        let mut flags_s: u32 = 0;

        let mut flags: u32 = 0;
        if ix_r < times_r.count {
            // SAFETY: `times_r` is the run `finalize_bake_times` filled in, and
            // `ix_r < times_r.count` (checked), so the slot is in bounds.
            bake_time = unsafe { *times_r.data.add(ix_r) };
            flags_r = bake_time.flags;
            bake_time.flags &= 0x7;
            flags |= TransformFlags::INCLUDE_ROTATION.raw();
        }
        if ix_t < times_t.count {
            // SAFETY: `times_t` is the run `finalize_bake_times` filled in, and
            // `ix_t < times_t.count` (checked), so the slot is in bounds.
            let t: BakeTime = unsafe { *times_t.data.add(ix_t) };
            let cmp: i32 = cmp_bake_time(t, bake_time);
            if cmp <= 0 {
                if cmp < 0 {
                    bake_time = t;
                    flags = 0;
                }
                bake_time.flags |= t.flags & 0x7;
                flags_t = t.flags;
                flags |= TransformFlags::INCLUDE_TRANSLATION.raw();
            }
        }
        if ix_s < times_s.count {
            // SAFETY: `times_s` is the run `finalize_bake_times` filled in, and
            // `ix_s < times_s.count` (checked), so the slot is in bounds.
            let t: BakeTime = unsafe { *times_s.data.add(ix_s) };
            let cmp: i32 = cmp_bake_time(t, bake_time);
            if cmp <= 0 {
                if cmp < 0 {
                    bake_time = t;
                    flags = 0;
                }
                bake_time.flags |= t.flags & 0x7;
                flags_s = t.flags;
                flags |= TransformFlags::INCLUDE_SCALE.raw();
            }
        }

        flags |= TransformFlags::IGNORE_SCALE_HELPER.raw()
            | TransformFlags::IGNORE_COMPONENTWISE_SCALE.raw()
            | TransformFlags::EXPLICIT_INCLUDES.raw();
        if (bc.opts_view().evaluate_flags() & EvaluateFlags::NO_EXTRAPOLATION.raw()) != 0 {
            flags |= TransformFlags::NO_EXTRAPOLATION.raw();
        }

        let eval_time: f64 = bake_time_sample_time(bake_time);
        // SAFETY: `bc.anim()` is the `ufbx_anim` `bake_anim_imp` stored into `bc`
        // and `node` is a live element of the scene that anim evaluates against.
        let mut transform: Transform =
            unsafe { evaluate_transform_flags(bc.anim(), node, eval_time, flags) };

        if (flags & TransformFlags::INCLUDE_TRANSLATION.raw()) != 0 {
            if !scale_helper_t.is_null() {
                // SAFETY: `scale_helper_t` is non-null (checked) and points to
                // the baked node for the parent's scale helper; `scale_keys` is
                // its own list, plain data, so reading it out by value leaves
                // that field valid.
                let scale: Vec3 = unsafe {
                    evaluate_baked_vec3(
                        ptr::read(&raw const (*scale_helper_t).scale_keys),
                        eval_time,
                    )
                };
                transform.translation.x *= scale.x;
                transform.translation.y *= scale.y;
                transform.translation.z *= scale.z;
            }

            transform.translation.x *= constant_scale_t.x;
            transform.translation.y *= constant_scale_t.y;
            transform.translation.z *= constant_scale_t.z;

            // SAFETY: `INCLUDE_TRANSLATION` is only set where `ix_t` was found
            // below `times_t.count`, and `keys_t_data` is the run of
            // `keys_t.count == times_t.count` slots pushed above, so slot `ix_t`
            // is in bounds; the three writes initialize that one key.
            unsafe {
                (*keys_t_data.add(ix_t)).time = bake_time.time;
                (*keys_t_data.add(ix_t)).value = transform.translation;
                (*keys_t_data.add(ix_t)).flags = BakedKeyFlags::from_raw(bake_time.flags | flags_t);
            }
            ix_t += 1;
        }
        if (flags & TransformFlags::INCLUDE_ROTATION.raw()) != 0 {
            // SAFETY: `INCLUDE_ROTATION` is only set where `ix_r` was found below
            // `times_r.count`, and `keys_r_data` is the run of
            // `keys_r.count == times_r.count` slots pushed above.
            unsafe {
                (*keys_r_data.add(ix_r)).time = bake_time.time;
                (*keys_r_data.add(ix_r)).value = transform.rotation;
                (*keys_r_data.add(ix_r)).flags = BakedKeyFlags::from_raw(bake_time.flags | flags_r);
            }
            ix_r += 1;
        }
        if (flags & TransformFlags::INCLUDE_SCALE.raw()) != 0 {
            if !scale_helper_s.is_null() {
                // SAFETY: `scale_helper_s` is non-null (checked) and points to
                // the baked node for the inherit-scale helper; `scale_keys` is
                // its own list, plain data, so reading it out by value leaves
                // that field valid.
                let scale: Vec3 = unsafe {
                    evaluate_baked_vec3(
                        ptr::read(&raw const (*scale_helper_s).scale_keys),
                        eval_time,
                    )
                };
                transform.scale.x *= scale.x;
                transform.scale.y *= scale.y;
                transform.scale.z *= scale.z;
            }

            transform.scale.x *= constant_scale_s.x;
            transform.scale.y *= constant_scale_s.y;
            transform.scale.z *= constant_scale_s.z;

            // SAFETY: `INCLUDE_SCALE` is only set where `ix_s` was found below
            // `times_s.count`, and `keys_s_data` is the run of
            // `keys_s.count == times_s.count` slots pushed above.
            unsafe {
                (*keys_s_data.add(ix_s)).time = bake_time.time;
                (*keys_s_data.add(ix_s)).value = transform.scale;
                (*keys_s_data.add(ix_s)).flags = BakedKeyFlags::from_raw(bake_time.flags | flags_s);
            }
            ix_s += 1;
        }
    }

    let baked_node: *mut BakedNode = bc.tmp_nodes_view().push_zero::<BakedNode>(1);
    ufbxi_check_err!(bc.error_view(), !baked_node.is_null(), "baked_node");

    // SAFETY: `baked_node` is the non-null (checked) zeroed `ufbx_baked_node`
    // just pushed onto `bc.tmp_nodes`, and `node` is a live scene node.
    unsafe {
        (*baked_node).element_id = (*node).element.element_id;
        (*baked_node).typed_id = (*node).element.typed_id;
    }
    // SAFETY: the two projections address the pushed baked node's own key list
    // and constant flag, and `keys_t` describes the live run of `ix_t` written
    // keys pushed onto `bc.tmp_prop`.
    unsafe {
        bake_postprocess_vec3(
            bc,
            &raw mut (*baked_node).translation_keys,
            &raw mut (*baked_node).constant_translation,
            keys_t,
        )
    }?;
    // SAFETY: as above, for the baked node's rotation fields and `keys_r`.
    unsafe {
        bake_postprocess_quat(
            bc,
            &raw mut (*baked_node).rotation_keys,
            &raw mut (*baked_node).constant_rotation,
            keys_r,
        )
    }?;
    // SAFETY: as above, for the baked node's scale fields and `keys_s`.
    unsafe {
        bake_postprocess_vec3(
            bc,
            &raw mut (*baked_node).scale_keys,
            &raw mut (*baked_node).constant_scale,
            keys_s,
        )
    }?;

    // SAFETY: `node` is a live scene node, so its `typed_id` indexes
    // `bc.baked_nodes()`, which `bake_anim` sizes with one slot per scene node.
    unsafe { *bc.baked_nodes().add((*node).element.typed_id as usize) = baked_node };

    // SAFETY: `bc.tmp_prop` is `bc`'s own scratch buffer, live for the borrow.
    unsafe { buf_clear(bc.tmp_prop_mut_ptr()) };

    // If this node is a scale helper, make sure to bake its siblings and
    // potentially their scale helpers if they are not a part of the animation.
    // SAFETY: `node` is a live scene node.
    if unsafe { (*node).is_scale_helper } {
        ufbx_assert!(!parent.is_null());
        // C: `ufbxi_for_ptr_list(ufbx_node, p_child, node->parent->children)`
        // SAFETY: a scale helper always has a parent (asserted above), and
        // `parent` is a live scene node whose `children` list this walks.
        let mut p_child: *mut *mut UfbxNode =
            unsafe { (*parent).children.data as *mut *mut UfbxNode };
        // SAFETY: as above, for the same list's count.
        let p_child_end: *mut *mut UfbxNode = unsafe { add_ptr(p_child, (*parent).children.count) };
        while p_child != p_child_end {
            // SAFETY: `p_child` walks the parent's children list and stops at
            // `p_child_end`, so it addresses a live slot holding a scene node.
            let child: *mut UfbxNode = unsafe { *p_child };
            if child == node {
                // SAFETY: `p_child` is inside the list, so `p_child + 1` is at
                // most one past its end.
                p_child = unsafe { p_child.add(1) };
                continue;
            }
            // SAFETY (this condition): `child` is a live scene node, so its
            // `typed_id` indexes `bc.nodes_to_bake()`, which `bake_anim` sizes
            // with one slot per scene node.
            if !unsafe { *bc.nodes_to_bake().add((*child).element.typed_id as usize) } {
                // SAFETY: as above.
                unsafe { *bc.nodes_to_bake().add((*child).element.typed_id as usize) = true };
                ufbxi_check_err!(
                    bc.error_view(),
                    // SAFETY: `bc.tmp_bake_stack` is `bc`'s own buffer and the
                    // projection addresses the live child's own `element_id`,
                    // the single `uint32_t` copied from.
                    !unsafe {
                        push_copy::<u32>(
                            bc.tmp_bake_stack_mut_ptr(),
                            1,
                            &raw const (*child).element.element_id,
                        )
                    }
                    .is_null(),
                    "((uint32_t*)ufbxi_push_size_copy((&bc->tmp_bake_stack), sizeof(uint32_t), (1), (&child->element_id)))"
                );
            }
            // C: `child->inherit_scale_node && child->inherit_scale_node->scale_helper && child->scale_helper`
            // SAFETY: `child` is a live scene node, whose `inherit_scale_node` is
            // its own nullable node ref.
            let child_inherit_scale_node: *mut UfbxNode =
                unsafe { opt_ptr(&raw const (*child).inherit_scale_node) };
            let child_inherit_scale_helper: *mut UfbxNode = if !child_inherit_scale_node.is_null() {
                // SAFETY: non-null (checked) and a live scene node, whose
                // `scale_helper` is its own nullable node ref.
                unsafe { opt_ptr(&raw const (*child_inherit_scale_node).scale_helper) }
            } else {
                ptr::null_mut()
            };
            // SAFETY: `child` is a live scene node, whose `scale_helper` is its
            // own nullable node ref.
            let child_scale_helper: *mut UfbxNode =
                unsafe { opt_ptr(&raw const (*child).scale_helper) };
            // SAFETY (the trailing operand): the null checks come first and `&&`
            // short-circuits, so `child_inherit_scale_helper` is a live scene
            // node whose `typed_id` indexes `bc.nodes_to_bake()`.
            if !child_inherit_scale_node.is_null()
                && !child_inherit_scale_helper.is_null()
                && !child_scale_helper.is_null()
                && unsafe {
                    *bc.nodes_to_bake()
                        .add((*child_inherit_scale_helper).element.typed_id as usize)
                }
            {
                // SAFETY: as above; the same `typed_id` indexes the equally sized
                // `bc.baked_nodes()`.
                ufbx_assert!(!unsafe {
                    *bc.baked_nodes()
                        .add((*child_inherit_scale_helper).element.typed_id as usize)
                }
                .is_null());
                // SAFETY (this condition): `child_scale_helper` is non-null
                // (checked above) and a live scene node, so its `typed_id`
                // indexes `bc.nodes_to_bake()`.
                if !unsafe {
                    *bc.nodes_to_bake()
                        .add((*child_scale_helper).element.typed_id as usize)
                } {
                    // SAFETY: as above.
                    unsafe {
                        *bc.nodes_to_bake()
                            .add((*child_scale_helper).element.typed_id as usize) = true
                    };
                    ufbxi_check_err!(
                        bc.error_view(),
                        // SAFETY: `bc.tmp_bake_stack` is `bc`'s own buffer and
                        // the projection addresses the live helper's own
                        // `element_id`, the single `uint32_t` copied from.
                        !unsafe {
                            push_copy::<u32>(
                                bc.tmp_bake_stack_mut_ptr(),
                                1,
                                &raw const (*child_scale_helper).element.element_id,
                            )
                        }
                        .is_null(),
                        "((uint32_t*)ufbxi_push_size_copy((&bc->tmp_bake_stack), sizeof(uint32_t), (1), (&child->scale_helper->element_id)))"
                    );
                }
            }
            // SAFETY: `p_child` is inside the parent's children list, so
            // `p_child + 1` is at most one past its end.
            p_child = unsafe { p_child.add(1) };
        }
    }

    Ok(())
}

// ufbx.c:27492-27505 `ufbxi_bake_node`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn bake_node(
    bc: &BakeContext,
    element_id: u32,
    props: *mut BakeProp,
    count: usize,
) -> Result<(), Fail> {
    // SAFETY: `element_id`, `props` and `count` are this `unsafe fn`'s own
    // parameters, forwarded unchanged under the same contract `bake_node_imp`
    // states — a scene element id and a `count`-element bake-prop run.
    unsafe { bake_node_imp(bc, element_id, props, count) }?;

    // Baking a node may cause further nodes to be baked, so keep going
    // until all dependencies are baked.
    while bc.tmp_bake_stack_view().num_items() > 0 {
        let mut child_id: u32 = 0;
        // SAFETY: `bc.tmp_bake_stack` is `bc`'s own buffer, holding at least one
        // `uint32_t` (the loop condition), and `child_id` is a live local slot
        // for the popped value.
        unsafe { pop::<u32>(bc.tmp_bake_stack_mut_ptr(), 1, &raw mut child_id) };
        // SAFETY: `child_id` was pushed as a scene element id by `bake_node_imp`,
        // and an empty prop run is described by a null base with count zero.
        unsafe { bake_node_imp(bc, child_id, ptr::null_mut(), 0) }?;
    }

    Ok(())
}

// ufbx.c:27507-27546 `ufbxi_bake_anim_prop`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn bake_anim_prop(
    bc: &BakeContext,
    element: *mut Element,
    prop_name: *const u8,
    props: *mut BakeProp,
    count: usize,
) -> Result<(), Fail> {
    // C: `ufbxi_for(ufbxi_bake_prop, prop, props, count)`
    // SAFETY: this `unsafe fn` requires `props` to address `count` live
    // `ufbxi_bake_prop`s, which is what the iterator walks.
    for prop in unsafe { SliceViewIter::<BakeProp>::from_raw_parts(props, count) } {
        // SAFETY: `prop.anim_value()` is the live `ufbx_anim_value` this bake
        // prop was collected from.
        unsafe { bake_times(bc, prop.anim_value(), false, BakedKeyFlags::KEYFRAME.raw()) }?;
    }

    // C: `ufbxi_bake_time_list times;`
    // SAFETY: `BakeTimeList` is a pointer/length pair, and an all-zero pattern
    // (null pointer, zero count) is a valid inhabitant of it.
    let mut times: BakeTimeList = unsafe { MaybeUninit::zeroed().assume_init() };
    // SAFETY: `times` is a live local `ufbxi_bake_time_list`, the output slot
    // `finalize_bake_times` fills in.
    unsafe { finalize_bake_times(bc, &raw mut times) }?;

    // C: `ufbx_baked_vec3_list keys;`
    // SAFETY: as for `times` — an all-zero pointer/length pair is valid.
    let mut keys: List<BakedVec3> = unsafe { MaybeUninit::zeroed().assume_init() };
    keys.count = times.count;
    keys.data = bc.tmp_prop_view().push::<BakedVec3>(keys.count);
    ufbxi_check_err!(bc.error_view(), !keys.data.is_null(), "keys.data");
    let keys_data: *mut BakedVec3 = keys.data as *mut BakedVec3;

    // C: `ufbx_string name; name.data = prop_name; name.length = strlen(prop_name);`
    // SAFETY: this `unsafe fn` requires `prop_name` to be a NUL-terminated
    // interned property name, so `strlen` stays inside it and the string it
    // builds describes exactly those bytes.
    let name: String = unsafe { String::new_c(prop_name, strlen(prop_name)) };

    for i in 0..times.count {
        // SAFETY: `times` is the run `finalize_bake_times` filled in, and
        // `i < times.count`.
        let bake_time: BakeTime = unsafe { *times.data.add(i) };
        let eval_time: f64 = bake_time_sample_time(bake_time);
        // SAFETY: `bc.anim()` is the `ufbx_anim` `bake_anim_imp` stored into
        // `bc`, `element` is the caller's live scene element (this `unsafe fn`'s
        // contract), and `name` describes the `prop_name` span measured above.
        let prop: Prop = unsafe {
            evaluate_prop_flags_len(
                bc.anim(),
                element,
                name.data,
                name.length,
                eval_time,
                bc.opts_view().evaluate_flags(),
            )
        };
        // SAFETY: `keys_data` is the non-null (checked) run of
        // `keys.count == times.count` slots pushed above and `i < times.count`;
        // `value_vec4` is a live field of the local `prop`, whose leading three
        // reals are the `ufbx_vec3` the union view reads.
        unsafe {
            (*keys_data.add(i)).time = bake_time.time;
            // C: `prop.value_vec3` — the value union's 3-real view over `value_vec4`.
            (*keys_data.add(i)).value = *(&raw const prop.value_vec4 as *const Vec3);
            (*keys_data.add(i)).flags = BakedKeyFlags::from_raw(bake_time.flags);
        }
    }

    let baked_prop: *mut BakedProp = bc.tmp_props_view().push_zero::<BakedProp>(1);
    ufbxi_check_err!(bc.error_view(), !baked_prop.is_null(), "baked_prop");

    // SAFETY: `baked_prop` is the non-null (checked) zeroed `ufbx_baked_prop`
    // just pushed onto `bc.tmp_props`; `prop_name` is NUL-terminated, so
    // `strlen` stays inside it.
    unsafe { (*baked_prop).name.length = strlen(prop_name) };
    // SAFETY: as above for `baked_prop`; `prop_name` is NUL-terminated with the
    // length just measured, so the `length + 1` bytes `push_copy` reads are its
    // own bytes plus the terminator, and `bc.result` is `bc`'s own buffer.
    unsafe {
        (*baked_prop).name.data = push_copy::<u8>(
            bc.result_mut_ptr(),
            (*baked_prop).name.length + 1,
            prop_name,
        )
    };
    ufbxi_check_err!(
        bc.error_view(),
        // SAFETY: `baked_prop` is the live pushed prop, just written.
        !unsafe { (*baked_prop).name.data }.is_null(),
        "baked_prop->name.data"
    );

    // SAFETY: the two projections address the pushed baked prop's own key list
    // and constant flag, and `keys` describes the run just written above.
    unsafe {
        bake_postprocess_vec3(
            bc,
            &raw mut (*baked_prop).keys,
            &raw mut (*baked_prop).constant_value,
            keys,
        )
    }?;

    // SAFETY: `bc.tmp_prop` is `bc`'s own scratch buffer, live for the borrow.
    unsafe { buf_clear(bc.tmp_prop_mut_ptr()) };

    Ok(())
}

// ufbx.c:27548-27585 `ufbxi_bake_element`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn bake_element(
    bc: &BakeContext,
    element_id: u32,
    props: *mut BakeProp,
    count: usize,
) -> Result<(), Fail> {
    // SAFETY: `bc.scene()` is the source `ufbx_scene` `bake_anim_imp` stored into
    // `bc`, live for the bake; this `unsafe fn` requires `element_id` to be one
    // of that scene's element ids, so the slot is in bounds of `elements` and
    // holds a live element pointer.
    let element: *mut Element =
        unsafe { *((*bc.scene()).elements.data as *const *mut Element).add(element_id as usize) };
    // SAFETY (this condition): `element` is that live scene element.
    if unsafe { (*element).type_ } as u32 == ElementType::Node as u32
        && !bc.opts_view().skip_node_transforms()
    {
        // SAFETY: `element_id`, `props` and `count` are this fn's own parameters,
        // forwarded unchanged under the contract `bake_node` states.
        unsafe { bake_node(bc, element_id, props, count) }?;
    }

    let mut begin: usize = 0;
    while begin < count {
        // SAFETY: this `unsafe fn` requires `props` to address `count` live
        // `ufbxi_bake_prop`s, and `begin < count` (loop condition).
        let prop_name: *const u8 = unsafe { (*props.add(begin)).prop_name };
        let mut end: usize = begin + 1;
        // SAFETY (this condition): `end < count` is checked first and `&&`
        // short-circuits, so that slot is in bounds of the prop run.
        while end < count && unsafe { (*props.add(end)).prop_name } == prop_name {
            end += 1;
        }

        // Don't bake transform related props for nodes unless specifically requested
        // SAFETY (the first operand): `element` is the live scene element.
        if unsafe { (*element).type_ } as u32 == ElementType::Node as u32
            && !bc.opts_view().bake_transform_props()
            // SAFETY: `TRANSFORM_PROPS` is a `'static` array of interned string
            // pointers, so its base addresses exactly `len()` readable entries.
            && unsafe {
                in_list(
                    TRANSFORM_PROPS.0.as_ptr(),
                    TRANSFORM_PROPS.0.len(),
                    prop_name,
                )
            }
        {
            begin = end;
            continue;
        }

        // SAFETY: `element` is the live scene element and `prop_name` is one of
        // its interned NUL-terminated prop names; `begin < end <= count`, so
        // `props + begin` addresses `end - begin` live props of the run.
        unsafe { bake_anim_prop(bc, element, prop_name, props.add(begin), end - begin) }?;
        begin = end;
    }

    let num_props: usize = bc.tmp_props_view().num_items();
    if num_props > 0 {
        let baked_elem: *mut BakedElement = bc.tmp_elements_view().push_zero::<BakedElement>(1);
        ufbxi_check_err!(bc.error_view(), !baked_elem.is_null(), "baked_elem");

        // SAFETY: `baked_elem` is the non-null (checked) zeroed
        // `ufbx_baked_element` just pushed onto `bc.tmp_elements`, and `element`
        // is the live scene element.
        unsafe {
            (*baked_elem).element_id = (*element).element_id;
            (*baked_elem).props.count = num_props;
        }
        // SAFETY: as above for `baked_elem`; `bc.tmp_props` holds `num_props`
        // items (read just above), which is what the pop moves into `bc.result`.
        unsafe {
            (*baked_elem).props.data = bc
                .result_view()
                .push_pop::<BakedProp>(bc.tmp_props_view(), num_props)
        };
        ufbxi_check_err!(
            bc.error_view(),
            // SAFETY: `baked_elem` is the live pushed element, just written.
            !unsafe { (*baked_elem).props.data }.is_null(),
            "baked_elem->props.data"
        );
    }

    Ok(())
}

// ufbx.c:27587-27592 `ufbxi_baked_node_less`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe extern "C" fn baked_node_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const BakedNode = va as *const BakedNode;
    let b: *const BakedNode = vb as *const BakedNode;
    // SAFETY: the sort comparator contract guarantees `va` and `vb` each point
    // to a live `ufbx_baked_node` element of the array being sorted.
    unsafe { (*a).typed_id < (*b).typed_id }
}

// ufbx.c:27594-27599 `ufbxi_baked_element_less`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe extern "C" fn baked_element_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const BakedElement = va as *const BakedElement;
    let b: *const BakedElement = vb as *const BakedElement;
    // SAFETY: the sort comparator contract guarantees `va` and `vb` each point
    // to a live `ufbx_baked_element` element of the array being sorted.
    unsafe { (*a).element_id < (*b).element_id }
}

// ufbx.c:27601-27705 `ufbxi_bake_anim`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) fn bake_anim(bc: &BakeContext) -> Result<(), Fail> {
    let anim: *const Anim = bc.anim();
    let scene: *const Scene = bc.scene();

    if !bc.opts_view().skip_node_transforms() {
        // `bc.scene()` is the live scene the bake context was built around (bc
        // construction invariant); both arrays hold one slot per scene node, so
        // a node's `typed_id` indexes them.
        // SAFETY: reading the live scene's node count through that invariant.
        bc.set_baked_nodes(
            bc.result_view()
                .push_zero::<*mut BakedNode>(unsafe { (*scene).nodes.count }),
        );
        ufbxi_check_err!(
            bc.error_view(),
            !bc.baked_nodes().is_null(),
            "bc->baked_nodes"
        );
        // SAFETY: reading the live scene's node count through the same invariant.
        bc.set_nodes_to_bake(
            bc.result_view()
                .push_zero::<bool>(unsafe { (*scene).nodes.count }),
        );
        ufbxi_check_err!(
            bc.error_view(),
            !bc.nodes_to_bake().is_null(),
            "bc->nodes_to_bake"
        );
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_layer, p_layer, anim->layers)`
    // SAFETY: `bc.anim()` is the live anim being baked (bc construction
    // invariant), so its `layers` run and every layer's `anim_props` run are
    // the scene's own contiguous lists. `prop` is a fresh non-null push onto
    // bc's own tmp-bake-props stack. `nodes_to_bake` holds one slot per scene
    // node, so a node element's `typed_id` is in bounds of it.
    unsafe {
        let mut p_layer: *mut *mut AnimLayer = (*anim).layers.data as *mut *mut AnimLayer;
        let p_layer_end: *mut *mut AnimLayer = add_ptr(p_layer, (*anim).layers.count);
        while p_layer != p_layer_end {
            let layer: *mut AnimLayer = *p_layer;

            // C: `ufbxi_for_list(ufbx_anim_prop, anim_prop, layer->anim_props)`
            let mut anim_prop: *mut AnimProp = (*layer).anim_props.data as *mut AnimProp;
            let anim_prop_end: *mut AnimProp = add_ptr(anim_prop, (*layer).anim_props.count);
            while anim_prop != anim_prop_end {
                let prop: *mut BakeProp = bc.tmp_bake_props_view().push::<BakeProp>(1);
                ufbxi_check_err!(bc.error_view(), !prop.is_null(), "prop");

                let element: *mut Element = ref_ptr(&raw const (*anim_prop).element);

                // Sort nodes by `typed_id` to make sure we process them in order.
                if (*element).type_ as u32 == ElementType::Node as u32 {
                    if !bc.nodes_to_bake().is_null() {
                        *bc.nodes_to_bake().add((*element).typed_id as usize) = true;
                    }
                    (*prop).sort_id = (*element).typed_id;
                } else {
                    (*prop).sort_id = u32::MAX;
                }

                (*prop).element_id = (*element).element_id;
                (*prop).prop_name = (*anim_prop).prop_name.data;
                (*prop).anim_value = ref_ptr(&raw const (*anim_prop).anim_value);

                anim_prop = anim_prop.add(1);
            }

            p_layer = p_layer.add(1);
        }
    }

    let num_props: usize = bc.tmp_bake_props_view().num_items();
    // Pops bc's bake-prop stack into bc's tmp buf; `num_props` is that stack's
    // own item count, so the pop is exact, and the sort is then over exactly
    // the popped run.
    let props: *mut BakeProp = bc
        .tmp_view()
        .push_pop::<BakeProp>(bc.tmp_bake_props_view(), num_props);
    ufbxi_check_err!(bc.error_view(), !props.is_null(), "props");

    // SAFETY: `props` is the fresh non-null `num_props`-element run just
    // checked; `bake_prop_less` compares two `BakeProp`s and takes no user
    // data, so the null `user` is what it expects.
    unsafe {
        unstable_sort(
            props as *mut c_void,
            num_props,
            size_of::<BakeProp>(),
            bake_prop_less,
            ptr::null_mut(),
        );
    }

    // Pre-bake layer weight times
    if !bc.opts_view().ignore_layer_weight_animation() {
        let mut has_weight_times: bool = false;
        // C: `ufbxi_for(ufbxi_bake_prop, prop, props, num_props)`
        // SAFETY: `props`/`num_props` is the sorted run above; each entry's
        // `element_id` was copied from a live scene element, so it indexes the
        // scene's own `elements` list.
        unsafe {
            for prop in SliceViewIter::<BakeProp>::from_raw_parts(props, num_props) {
                if prop.prop_name() != sp::Weight.as_ptr() {
                    continue;
                }
                let element: *mut Element = *((*scene).elements.data as *const *mut Element)
                    .add(prop.element_id() as usize);
                if (*element).type_ as u32 == ElementType::AnimLayer as u32 {
                    bake_times(bc, prop.anim_value(), true, 0)?;
                    has_weight_times = true;
                }
            }
        }

        if has_weight_times {
            // C: `ufbxi_bake_time_list weight_times = { 0 };`
            // SAFETY: `BakeTimeList` is a data pointer plus a count, for which
            // all-zero is the valid empty list; `finalize_bake_times` fills it
            // through the unaliased local out-pointer, and the copy that
            // follows reads exactly the `count` elements it reported into bc's
            // own tmp buf.
            let weight_times: BakeTimeList = unsafe {
                let mut weight_times: BakeTimeList = MaybeUninit::zeroed().assume_init();
                finalize_bake_times(bc, &raw mut weight_times)?;
                weight_times
            };

            bc.layer_weight_times_view().set_count(weight_times.count);
            bc.layer_weight_times_view().set_data(unsafe {
                push_copy::<BakeTime>(bc.tmp_mut_ptr(), weight_times.count, weight_times.data)
            });
            ufbxi_check_err!(
                bc.error_view(),
                !bc.layer_weight_times_view().data().is_null(),
                "bc->layer_weight_times.data"
            );

            // SAFETY: clearing bc's own per-prop tmp buf.
            unsafe { buf_clear(bc.tmp_prop_mut_ptr()) };
        }
    }

    let mut begin: usize = 0;
    while begin < num_props {
        // SAFETY: `begin < num_props` and the inner scan stops at
        // `num_props`, so every read and the `props.add(begin)` sub-run handed
        // to `bake_element` stay inside the `num_props`-element run.
        unsafe {
            let element_id: u32 = (*props.add(begin)).element_id;
            let mut end: usize = begin + 1;
            while end < num_props && (*props.add(end)).element_id == element_id {
                end += 1;
            }
            bake_element(bc, element_id, props.add(begin), end - begin)?;
            begin = end;
        }
    }

    let num_nodes: usize = bc.tmp_nodes_view().num_items();
    let num_elements: usize = bc.tmp_elements_view().num_items();

    bc.bake_view().nodes_view().set_count(num_nodes);
    // Pops bc's node stack into bc's result buf; `num_nodes` is that stack's
    // own item count, so the pop is exact.
    bc.bake_view().nodes_view().set_data(
        bc.result_view()
            .push_pop::<BakedNode>(bc.tmp_nodes_view(), num_nodes),
    );
    ufbxi_check_err!(
        bc.error_view(),
        !bc.bake_view().nodes_view().data().is_null(),
        "bc->bake.nodes.data"
    );

    bc.bake_view().elements_view().set_count(num_elements);
    // Pops bc's element stack into bc's result buf; `num_elements` is that
    // stack's own item count, so the pop is exact.
    bc.bake_view().elements_view().set_data(
        bc.result_view()
            .push_pop::<BakedElement>(bc.tmp_elements_view(), num_elements),
    );
    ufbxi_check_err!(
        bc.error_view(),
        !bc.bake_view().elements_view().data().is_null(),
        "bc->bake.elements.data"
    );

    // SAFETY: both runs are the fresh non-null pops just checked, sorted over
    // their own reported counts; neither comparator takes user data, so the
    // null `user` is what they expect.
    unsafe {
        unstable_sort(
            bc.bake_view().nodes_view().data() as *mut c_void,
            bc.bake_view().nodes_view().count(),
            size_of::<BakedNode>(),
            baked_node_less,
            ptr::null_mut(),
        );
        unstable_sort(
            bc.bake_view().elements_view().data() as *mut c_void,
            bc.bake_view().elements_view().count(),
            size_of::<BakedElement>(),
            baked_element_less,
            ptr::null_mut(),
        );
    }

    if bc.time_min() < bc.time_max() {
        bc.bake_view().set_key_time_min(bc.time_min());
        bc.bake_view().set_key_time_max(bc.time_max());
    }

    if bc.time_begin() < bc.time_end() {
        bc.bake_view().set_playback_time_begin(bc.time_begin());
        bc.bake_view().set_playback_time_end(bc.time_end());
        bc.bake_view()
            .set_playback_duration(bc.time_end() - bc.time_begin());
    }

    Ok(())
}

// ufbx.c:27707-27765 `ufbxi_bake_anim_imp`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn bake_anim_imp(bc: &BakeContext, anim: *const Anim) -> Result<(), Fail> {
    if bc.opts_view().resample_rate() <= 0.0 {
        bc.opts_view().set_resample_rate(30.0);
    }
    if bc.opts_view().minimum_sample_rate() <= 0.0 {
        bc.opts_view().set_minimum_sample_rate(19.5);
    }
    if bc.opts_view().max_keyframe_segments() == 0 {
        bc.opts_view().set_max_keyframe_segments(32);
    }
    if bc.opts_view().key_reduction_threshold() == 0.0 {
        bc.opts_view().set_key_reduction_threshold(0.000001);
    }
    if bc.opts_view().key_reduction_passes() == 0 {
        bc.opts_view().set_key_reduction_passes(4);
    }

    // SAFETY (this condition): `anim` is the caller's live `ufbx_anim` — this
    // `unsafe fn`'s contract.
    if bc.opts_view().trim_start_time() && unsafe { (*anim).time_begin } > 0.0 {
        // SAFETY: as above for `anim`; `bc.scene()` is the source `ufbx_scene`
        // the bake context was built around, live for the bake.
        bc.set_ktime_offset(
            -unsafe { (*anim).time_begin } * unsafe { (*bc.scene()).metadata.ktime_second } as f64,
        );
    }

    // SAFETY: the error slot, temp allocator and `opts.temp_allocator` are all
    // `bc`'s own fields, live for the `&BakeContext` borrow.
    unsafe {
        init_ator(
            bc.error_mut_ptr(),
            bc.ator_tmp_mut_ptr(),
            bc.opts_view().temp_allocator_ptr(),
            c"temp",
        )
    };
    // SAFETY: as above, for `bc`'s result allocator and `opts.result_allocator`.
    unsafe {
        init_ator(
            bc.error_mut_ptr(),
            bc.ator_result_mut_ptr(),
            bc.opts_view().result_allocator_ptr(),
            c"result",
        )
    };

    bc.result_view().set_unordered(true);
    bc.result_view().set_ator(bc.ator_result_mut_ptr());

    bc.tmp_view().set_unordered(true);
    bc.tmp_view().set_ator(bc.ator_tmp_mut_ptr());

    bc.tmp_prop_view().set_ator(bc.ator_tmp_mut_ptr());
    bc.tmp_prop_view().set_unordered(true);
    bc.tmp_prop_view().set_clearable(true);

    bc.tmp_times_view().set_ator(bc.ator_tmp_mut_ptr());
    bc.tmp_bake_props_view().set_ator(bc.ator_tmp_mut_ptr());
    bc.tmp_nodes_view().set_ator(bc.ator_tmp_mut_ptr());
    bc.tmp_elements_view().set_ator(bc.ator_tmp_mut_ptr());
    bc.tmp_props_view().set_ator(bc.ator_tmp_mut_ptr());
    bc.tmp_bake_stack_view().set_ator(bc.ator_tmp_mut_ptr());

    bc.set_anim(anim);
    // SAFETY (this condition and the block it guards): `anim` is the caller's
    // live `ufbx_anim`.
    if unsafe { (*anim).time_begin } < unsafe { (*anim).time_end } {
        bc.set_time_begin(unsafe { (*anim).time_begin });
        bc.set_time_end(unsafe { (*anim).time_end });
    }
    bc.set_time_min(math::INFINITY);
    bc.set_time_max(-math::INFINITY);

    bc.set_imp(bc.result_view().push::<BakedAnimImp>(1));
    ufbxi_check_err!(bc.error_view(), !bc.imp().is_null(), "bc->imp");

    // Expose the wide allocation so `get_imp` can recover this header from a
    // (possibly narrowed) public `&BakedAnim` pointer via exposed provenance.
    (bc.imp() as *mut u8).expose_provenance();

    bake_anim(bc)?;

    // SAFETY (this fn's remaining `*bc.imp()` accesses): `bc.imp()` is the
    // non-null (checked above) `ufbxi_baked_anim_imp` pushed into `bc`'s result
    // buffer, so every projection below addresses that live allocation's own
    // fields; `init_ref` sets up its `refcount` with no parent.
    unsafe {
        init_ref(
            &raw mut (*bc.imp()).refcount,
            BAKED_ANIM_IMP_MAGIC,
            ptr::null_mut(),
        );
    }

    bc.bake_view()
        .metadata_view()
        .set_result_memory_used(bc.ator_result_view().current_size());
    bc.bake_view()
        .metadata_view()
        .set_temp_memory_used(bc.ator_tmp_view().current_size());
    bc.bake_view()
        .metadata_view()
        .set_result_allocs(bc.ator_result_view().num_allocs());
    bc.bake_view()
        .metadata_view()
        .set_temp_allocs(bc.ator_tmp_view().num_allocs());

    // SAFETY: as above — `bc.imp()` is the live pushed header.
    unsafe { (*bc.imp()).magic = BAKED_ANIM_IMP_MAGIC };
    // C: `bc->imp->bake = bc->bake;` (struct assignment)
    // SAFETY: the source is `bc`'s own `bake` field (live for the borrow) and
    // the destination is the pushed header's own `bake` field; the bake context
    // and the pushed header are distinct allocations.
    unsafe { ptr::copy_nonoverlapping(bc.bake_mut_ptr(), &raw mut (*bc.imp()).bake, 1) };
    // SAFETY: as above; the moved-out result allocator and buffer take ownership
    // in the header's refcount, which `init_ref` set up.
    unsafe { (*bc.imp()).refcount.ator = bc.ator_result() };
    // SAFETY: as above, for the result buffer.
    unsafe { (*bc.imp()).refcount.buf = bc.take_result() };

    Ok(())
}

// CONTINUATION POINT: the `// -- Animation baking` section is complete
// (ufbx.c:26670-27767); the C `#endif` at ufbx.c:27767 closes the
// `feature = "baking"` gate above. Next banner: ufbx.c:27769 `// -- NURBS`
// (owned by `native::nurbs`).
