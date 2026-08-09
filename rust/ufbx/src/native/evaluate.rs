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
    RawAnimOpts, RawGeometryCacheDataOpts, RawLoadOpts, RawOpenFileCb, RawOpenFileOpts,
    RawPropOverrideDesc, RawStream, RotationOrder, Scene, Tangent, TransformOverride,
    UnicodeErrorHandling, Vec3, Warning, WarningType,
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
    free, free_ator, init_ator, Allocator, ANIM_IMP_MAGIC, SCENE_IMP_MAGIC, ZERO_SIZE_BUFFER,
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
use crate::native::buf::{buf_clear, pop, push_fast};
use crate::native::buf::{buf_free, push, push_copy, push_pop, push_zero, Buf};
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
    begin_parse, determine_format, get_imp, get_name_key, get_name_key_c, load_maps, load_strings,
    Context, Node, Refcount, SceneImp, ELEMENT_TYPE_COUNT, MIN_FILE_FORMAT_LOOKAHEAD,
};
#[cfg(feature = "baking")]
use crate::native::parse::{find_prop, is_vec3_zero};
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
use crate::native::warnings::{pop_warnings, ufbxi_warnf};
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
#[must_use]
pub(crate) unsafe fn evaluate_skinning(
    scene: *mut Scene,
    error: *mut Error,
    buf_result: *mut Buf,
    buf_tmp: *mut Buf,
    time: f64,
    load_caches: bool,
    cache_opts: *mut RawGeometryCacheDataOpts,
) -> Result<(), Fail> {
    let mut max_skinned_indices: usize = 0;

    // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, scene->meshes)`
    let mut p_mesh: *mut *mut Mesh = (*scene).meshes.data as *mut *mut Mesh;
    let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, (*scene).meshes.count);
    while p_mesh != p_mesh_end {
        let mesh: *mut Mesh = *p_mesh;
        if (*mesh).blend_deformers.count == 0
            && (*mesh).skin_deformers.count == 0
            && ((*mesh).cache_deformers.count == 0 || !load_caches)
        {
            p_mesh = p_mesh.add(1);
            continue;
        }
        max_skinned_indices = max_sz(max_skinned_indices, (*mesh).num_indices);
        p_mesh = p_mesh.add(1);
    }

    let topo: *mut TopoEdge = push::<TopoEdge>(buf_tmp, max_skinned_indices);
    ufbxi_check_err!(error, !topo.is_null(), "topo");

    // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, scene->meshes)`
    let mut p_mesh: *mut *mut Mesh = (*scene).meshes.data as *mut *mut Mesh;
    let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, (*scene).meshes.count);
    while p_mesh != p_mesh_end {
        let mesh: *mut Mesh = *p_mesh;
        if (*mesh).blend_deformers.count == 0
            && (*mesh).skin_deformers.count == 0
            && ((*mesh).cache_deformers.count == 0 || !load_caches)
        {
            p_mesh = p_mesh.add(1);
            continue;
        }
        if (*mesh).num_vertices == 0 {
            p_mesh = p_mesh.add(1);
            continue;
        }

        let num_vertices: usize = (*mesh).num_vertices;
        let mut result_pos: *mut Vec3 = push::<Vec3>(buf_result, num_vertices + 1);
        ufbxi_check_err!(error, !result_pos.is_null(), "result_pos");

        // C: `result_pos[0] = ufbx_zero_vec3; result_pos++;`
        *result_pos = ZERO_VEC3;
        result_pos = result_pos.add(1);

        let mut cached_position: bool = false;
        let mut cached_normals: bool = false;
        if load_caches && (*mesh).cache_deformers.count > 0 {
            // C: `ufbxi_for_ptr_list(ufbx_cache_deformer, p_cache, mesh->cache_deformers)`
            let mut p_cache: *mut *mut CacheDeformer =
                (*mesh).cache_deformers.data as *mut *mut CacheDeformer;
            let p_cache_end: *mut *mut CacheDeformer =
                add_ptr(p_cache, (*mesh).cache_deformers.count);
            while p_cache != p_cache_end {
                let channel: *mut CacheChannel =
                    opt_ptr(ptr::addr_of!((*(*p_cache)).external_channel));
                if channel.is_null() {
                    p_cache = p_cache.add(1);
                    continue;
                }

                if ((*channel).interpretation == CacheInterpretation::VertexPosition
                    || (*channel).interpretation == CacheInterpretation::Points)
                    && !cached_position
                {
                    let num_read: usize = sample_geometry_cache_vec3(
                        channel,
                        time,
                        result_pos,
                        num_vertices,
                        cache_opts,
                    );
                    if num_read == num_vertices {
                        (*mesh).skinned_is_local = true;
                        cached_position = true;
                    }
                } else if (*channel).interpretation == CacheInterpretation::VertexNormal
                    && !cached_normals
                {
                    // TODO: Is this right at all?
                    let num_normals: usize = (*mesh).skinned_normal.values.count;
                    let mut normal_data: *mut Vec3 = push::<Vec3>(buf_result, num_normals + 1);
                    ufbxi_check_err!(error, !normal_data.is_null(), "normal_data");
                    // C: `normal_data[0] = ufbx_zero_vec3; normal_data++;`
                    *normal_data = ZERO_VEC3;
                    normal_data = normal_data.add(1);

                    let num_read: usize = sample_geometry_cache_vec3(
                        channel,
                        time,
                        normal_data,
                        num_normals,
                        cache_opts,
                    );
                    if num_read == num_normals {
                        cached_normals = true;
                        (*mesh).skinned_normal.values.data = normal_data as *const Vec3;
                    }
                }
                p_cache = p_cache.add(1);
            }
        }

        if !cached_position {
            // C: `memcpy(result_pos, mesh->vertices.data, num_vertices * sizeof(ufbx_vec3));`
            ptr::copy_nonoverlapping((*mesh).vertices.data, result_pos, num_vertices);

            // C: `ufbxi_for_ptr_list(ufbx_blend_deformer, p_blend, mesh->blend_deformers)`
            let mut p_blend: *mut *mut BlendDeformer =
                (*mesh).blend_deformers.data as *mut *mut BlendDeformer;
            let p_blend_end: *mut *mut BlendDeformer =
                add_ptr(p_blend, (*mesh).blend_deformers.count);
            while p_blend != p_blend_end {
                add_blend_vertex_offsets(*p_blend, result_pos, num_vertices, 1.0);
                p_blend = p_blend.add(1);
            }

            // TODO: What should we do about multiple skins??
            if (*mesh).skin_deformers.count > 0 {
                // C: `ufbx_matrix *fallback = mesh->instances.count > 0 ? &mesh->instances.data[0]->geometry_to_world : NULL;`
                // (`mesh->instances` reads through the anonymous `ufbx_element`
                // union member; the generated bindings spell it out.)
                let fallback: *mut Matrix = if (*mesh).element.instances.count > 0 {
                    ptr::addr_of_mut!(
                        (*ref_ptr::<UfbxNode>((*mesh).element.instances.data.add(0)))
                            .geometry_to_world
                    )
                } else {
                    ptr::null_mut()
                };
                let skin: *mut SkinDeformer = ref_ptr((*mesh).skin_deformers.data.add(0));
                for i in 0..num_vertices {
                    // C: `ufbx_get_skin_vertex_matrix(skin, i, fallback)` — the
                    // `ufbx_inline` wrapper in ufbx.h (5601-5603) forwarding to
                    // the catch impl with a NULL panic.
                    let mat: Matrix =
                        catch_get_skin_vertex_matrix(ptr::null_mut(), skin, i, fallback);
                    *result_pos.add(i) = transform_position(&mat, *result_pos.add(i));
                }

                (*mesh).skinned_is_local = false;
            }
        }

        (*mesh).skinned_position.values.data = result_pos as *const Vec3;

        if !cached_normals {
            let num_indices: usize = (*mesh).num_indices;
            let normal_indices: *mut u32 = push::<u32>(buf_result, num_indices);
            ufbxi_check_err!(error, !normal_indices.is_null(), "normal_indices");

            compute_topology(mesh, topo, num_indices);
            let num_normals: usize = generate_normal_mapping(
                mesh,
                topo,
                num_indices,
                normal_indices,
                num_indices,
                false,
            );

            if num_normals == (*mesh).num_vertices {
                (*mesh).skinned_normal.unique_per_vertex = true;
            }

            let mut normal_data: *mut Vec3 = push::<Vec3>(buf_result, num_normals + 1);
            ufbxi_check_err!(error, !normal_data.is_null(), "normal_data");

            // C: `normal_data[0] = ufbx_zero_vec3; normal_data++;`
            *normal_data = ZERO_VEC3;
            normal_data = normal_data.add(1);

            compute_normals(
                mesh,
                ptr::addr_of!((*mesh).skinned_position),
                normal_indices,
                num_indices,
                normal_data,
                num_normals,
            );

            (*mesh).generated_normals = true;
            (*mesh).skinned_normal.exists = true;
            (*mesh).skinned_normal.values.data = normal_data as *const Vec3;
            (*mesh).skinned_normal.values.count = num_normals;
            (*mesh).skinned_normal.indices.data = normal_indices as *const u32;
            (*mesh).skinned_normal.indices.count = num_indices;
            (*mesh).skinned_normal.value_reals = 3;
        }

        p_mesh = p_mesh.add(1);
    }

    Ok(())
}

// ufbx.c:25164-25168 `ufbxi_evaluate_skinning` (`#else` branch — feature
// disabled). C parity, NOT a stub: `ufbxi_report_err_msg` records the error
// and KEEPS GOING (PORTING.md trap #16); the `return 0` that follows becomes
// `Err(Fail)`.
#[cfg(not(feature = "skinning-eval"))]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn evaluate_skinning(
    scene: *mut Scene,
    error: *mut Error,
    buf_result: *mut Buf,
    buf_tmp: *mut Buf,
    time: f64,
    load_caches: bool,
    cache_opts: *mut RawGeometryCacheDataOpts,
) -> Result<(), Fail> {
    // C: all parameters other than `error` are unreferenced in the `#else` arm.
    let _ = (scene, buf_result, buf_tmp, time, load_caches, cache_opts);
    ufbxi_fmt_err_info!(error, "UFBX_ENABLE_SKINNING_EVALUATION");
    ufbxi_report_err_msg!(
        error,
        "UFBXI_FEATURE_SKINNING_EVALUATION",
        "Feature disabled"
    );
    Err(Fail)
}

// ufbx.c:25171-25185 `ufbxi_fixup_opts_string`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fixup_opts_string(
    uc: &Context,
    str: *mut String,
    push: bool,
) -> Result<(), Fail> {
    if (*str).length > 0 {
        if (*str).length == usize::MAX {
            // C: `str->length = str->data ? strlen(str->data) : 0;`
            (*str).length = if !(*str).data.is_null() {
                strlen((*str).data)
            } else {
                0
            };
        }
        if push {
            push_string_place_str(uc.string_pool_mut_ptr(), str, false)?;
        }
    } else {
        (*str).data = EMPTY_CHAR.as_ptr();
    }

    Ok(())
}

// ufbx.c:25187-25202 `ufbxi_resolve_warning_elements`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn resolve_warning_elements(uc: &Context) -> Result<(), Fail> {
    let num_elements: usize = (*uc.get()).tmp_element_id.num_items;
    let element_ids: *mut u32 = push_pop::<u32>(
        uc.tmp_mut_ptr(),
        &mut (*uc.get()).tmp_element_id,
        num_elements,
    );
    ufbxi_check!(uc, !element_ids.is_null(), "element_ids");

    // C: `ufbxi_for_list(ufbx_warning, warning, uc->scene.metadata.warnings)`
    let mut warning: *mut Warning = (*uc.get()).scene.metadata.warnings.data as *mut Warning;
    let warning_end: *mut Warning = add_ptr(warning, (*uc.get()).scene.metadata.warnings.count);
    while warning != warning_end {
        let element_id: u32 = (*warning).element_id;
        // Decode `element_id`, see HACK(warning-element) in `ufbxi_vwarnf_imp()` for the encoding.
        if (element_id & 0x80000000u32) != 0 && element_id != !0u32 {
            (*warning).element_id = *element_ids.add((element_id & !0x80000000u32) as usize);
        }
        warning = warning.add(1);
    }

    Ok(())
}

// ufbx.c:25204-25410 `ufbxi_load_imp`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn load_imp(uc: &Context) -> Result<(), Fail> {
    // Check for deferred failure
    if uc.deferred_failure() {
        return Err(Fail);
    }
    if uc.deferred_load() {
        // C: `ufbx_stream stream = { 0 };` / `ufbx_open_file_opts opts = { 0 };`
        let mut stream: RawStream = MaybeUninit::zeroed().assume_init();
        let mut opts: RawOpenFileOpts = MaybeUninit::zeroed().assume_init();
        let filename: *const u8 = uc.load_filename();
        let mut filename_len: usize = uc.load_filename_len();
        let ok: bool;
        if filename_len == usize::MAX {
            opts.filename_null_terminated = true;
            filename_len = strlen(filename);
        }
        if (*uc.get()).opts.filename.length == 0 || (*uc.get()).opts.filename.data.is_null() {
            (*uc.get()).opts.filename.data = filename;
            (*uc.get()).opts.filename.length = filename_len;
        }
        // C: `ufbx_error error; error.type = UFBX_ERROR_NONE;` — C initializes
        // only `type`; the struct is only copied below after the open-file
        // path fully wrote it (zero-filled here, C leaves the rest uninit).
        let mut error: Error = MaybeUninit::zeroed().assume_init();
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
        let open_with_default = (*uc.get()).opts.open_main_file_with_default
            || (*uc.get()).opts.open_file_cb.fn_ == Some(default_fn);
        if open_with_default {
            let ctx: OpenFileContext = uc.ator_tmp_mut_ptr() as OpenFileContext;
            ok = open_file_ctx(&mut stream, ctx, filename, filename_len, &opts, &mut error);
        } else {
            ok = open_file(
                ptr::addr_of!((*uc.get()).opts.open_file_cb),
                &mut stream,
                uc.load_filename(),
                filename_len,
                ptr::null(),
                uc.ator_tmp_mut_ptr(),
                OpenFileType::MainModel,
            );
        }
        if !ok {
            if error.type_ != ErrorType::None {
                // cppcheck-suppress uninitStructMember
                // C: `uc->error = error;` (struct copy)
                ptr::copy_nonoverlapping(&error, uc.error_mut_ptr(), 1);
            } else {
                set_err_info(uc.error_mut_ptr(), filename, filename_len);
            }
            ufbxi_fail_msg!(uc, "open_file_fn()", "File not found");
        }
        uc.set_read_fn(stream.read_fn);
        uc.set_skip_fn(stream.skip_fn);
        uc.set_size_fn(stream.size_fn);
        uc.set_close_fn(stream.close_fn);
        uc.set_read_user(stream.user);
    }

    if (*uc.get()).opts.progress_cb.fn_.is_some()
        && uc.progress_bytes_total() == 0
        && uc.size_fn().is_some()
    {
        let total: u64 = (uc.size_fn().unwrap())(uc.read_user());
        ufbxi_check!(uc, total != u64::MAX, "total != UINT64_MAX");
        uc.set_progress_bytes_total(total);
    }

    ufbxi_check!(
        uc,
        (*uc.get()).opts.path_separator >= 0x20 && (*uc.get()).opts.path_separator <= 0x7e,
        "uc->opts.path_separator >= 0x20 && uc->opts.path_separator <= 0x7e"
    );

    // C: `ufbxi_check(ufbxi_<callee>(uc))` — the caller-side check pushes its
    // own error-stack frame (function/line/#cond) on top of the callee's; a
    // bare `?` would drop that frame and shorten `ufbx_error.stack_size`
    // (checklist #13; test `error_format_long` asserts `stack_size >= 2`).
    ufbxi_check!(
        uc,
        fixup_opts_string(
            uc,
            ptr::addr_of_mut!((*uc.get()).opts.filename) as *mut String,
            false
        )
        .is_ok(),
        "ufbxi_fixup_opts_string(uc, &uc->opts.filename, false)"
    );
    ufbxi_check!(
        uc,
        fixup_opts_string(
            uc,
            ptr::addr_of_mut!((*uc.get()).opts.obj_mtl_path) as *mut String,
            true
        )
        .is_ok(),
        "ufbxi_fixup_opts_string(uc, &uc->opts.obj_mtl_path, true)"
    );
    ufbxi_check!(
        uc,
        fixup_opts_string(
            uc,
            ptr::addr_of_mut!((*uc.get()).opts.geometry_transform_helper_name) as *mut String,
            true,
        )
        .is_ok(),
        "ufbxi_fixup_opts_string(uc, &uc->opts.geometry_transform_helper_name, true)"
    );
    ufbxi_check!(
        uc,
        fixup_opts_string(
            uc,
            ptr::addr_of_mut!((*uc.get()).opts.scale_helper_name) as *mut String,
            true
        )
        .is_ok(),
        "ufbxi_fixup_opts_string(uc, &uc->opts.scale_helper_name, true)"
    );

    ufbxi_check!(
        uc,
        thread_pool_init(
            uc.thread_pool_mut_ptr(),
            uc.error_mut_ptr(),
            uc.ator_tmp_mut_ptr(),
            ptr::addr_of!((*uc.get()).opts.thread_opts),
        )
        .is_ok(),
        "ufbxi_thread_pool_init(&uc->thread_pool, &uc->error, &uc->ator_tmp, &uc->opts.thread_opts)"
    );

    if !(*uc.get()).opts.allow_unsafe {
        ufbxi_check_msg!(
            uc,
            (*uc.get()).opts.index_error_handling != IndexErrorHandling::UnsafeIgnore,
            "Unsafe options",
            "uc->opts.index_error_handling != UFBX_INDEX_ERROR_HANDLING_UNSAFE_IGNORE"
        );
        ufbxi_check_msg!(
            uc,
            (*uc.get()).opts.unicode_error_handling != UnicodeErrorHandling::UnsafeIgnore,
            "Unsafe options",
            "uc->opts.unicode_error_handling != UFBX_UNICODE_ERROR_HANDLING_UNSAFE_IGNORE"
        );
    } else {
        (*uc.get()).scene.metadata.is_unsafe = true;
    }

    if (*uc.get()).opts.index_error_handling == IndexErrorHandling::NoIndex {
        (*uc.get()).scene.metadata.may_contain_no_index = true;
    }

    uc.set_retain_mesh_parts(
        !(*uc.get()).opts.ignore_geometry && !(*uc.get()).opts.skip_mesh_parts,
    );
    (*uc.get())
        .scene
        .metadata
        .may_contain_missing_vertex_position = (*uc.get()).opts.allow_missing_vertex_position;
    (*uc.get()).scene.metadata.may_contain_broken_elements =
        (*uc.get()).opts.connect_broken_elements;

    (*uc.get()).scene.metadata.creator.data = EMPTY_CHAR.as_ptr();

    uc.set_unit_scale(1.0);
    if uc.data().is_null() {
        ufbxi_dev_assert!(uc.data_begin().is_null());
        // C: `uc->data_begin = uc->data = ufbxi_zero_size_buffer;`
        uc.set_data(ZERO_SIZE_BUFFER.as_ptr());
        uc.set_data_begin(uc.data());
    }

    uc.set_retain_vertex_w(
        ((*uc.get()).opts.retain_dom || (*uc.get()).opts.retain_vertex_attrib_w)
            && !(*uc.get()).opts.ignore_geometry,
    );

    ufbxi_check!(uc, load_strings(uc).is_ok(), "ufbxi_load_strings(uc)");
    ufbxi_check!(uc, load_maps(uc).is_ok(), "ufbxi_load_maps(uc)");
    ufbxi_check!(
        uc,
        determine_format(uc).is_ok(),
        "ufbxi_determine_format(uc)"
    );

    let format: FileFormat = (*uc.get()).scene.metadata.file_format;

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
        update_scene_metadata(ptr::addr_of_mut!((*uc.get()).scene.metadata));
        ufbxi_check!(uc, init_file_paths(uc).is_ok(), "ufbxi_init_file_paths(uc)");
    } else if format == FileFormat::Obj {
        ufbxi_check!(uc, obj_load(uc).is_ok(), "ufbxi_obj_load(uc)");
        update_scene_metadata(ptr::addr_of_mut!((*uc.get()).scene.metadata));
    } else if format == FileFormat::Mtl {
        ufbxi_check!(uc, mtl_load(uc).is_ok(), "ufbxi_mtl_load(uc)");
        update_scene_metadata(ptr::addr_of_mut!((*uc.get()).scene.metadata));
    }

    // Fake DOM root if necessary
    if (*uc.get()).opts.retain_dom && (*uc.get()).scene.dom_root.is_none() {
        let dom_root: *mut DomNode = push_zero::<DomNode>(uc.result_mut_ptr(), 1);
        ufbxi_check!(uc, !dom_root.is_null(), "dom_root");
        (*dom_root).name.data = EMPTY_CHAR.as_ptr();
        (*uc.get()).scene.dom_root = Some(Ref::from_ptr(dom_root));
    }

    ufbxi_check!(
        uc,
        pre_finalize_scene(uc).is_ok(),
        "ufbxi_pre_finalize_scene(uc)"
    );

    // We can free `tmp_parse` already here as all parsing is done by now.
    buf_free(uc.tmp_parse_mut_ptr());

    ufbxi_check!(uc, finalize_scene(uc).is_ok(), "ufbxi_finalize_scene(uc)");

    update_scene_settings(ptr::addr_of_mut!((*uc.get()).scene.settings));
    if (*uc.get()).scene.metadata.file_format == FileFormat::Obj {
        update_scene_settings_obj(uc);
    }

    // Axis conversion
    if coordinate_axes_valid((*uc.get()).opts.target_axes) {
        transform_to_axes(uc, (*uc.get()).opts.target_axes);
    }

    // Unit conversion
    if (*uc.get()).opts.target_unit_meters > 0.0 {
        ufbxi_check!(
            uc,
            scale_units(uc, (*uc.get()).opts.target_unit_meters).is_ok(),
            "ufbxi_scale_units(uc, uc->opts.target_unit_meters)"
        );
    }

    // TODO: This could be done in evaluate as well with refactoring
    update_adjust_transforms(uc, ptr::addr_of_mut!((*uc.get()).scene));

    ufbxi_check!(uc, modify_geometry(uc).is_ok(), "ufbxi_modify_geometry(uc)");
    postprocess_scene(uc);

    update_scene(ptr::addr_of_mut!((*uc.get()).scene), true, ptr::null(), 0);

    // Force a non-NULL anim pointer
    if ref_ptr(ptr::addr_of!((*uc.get()).scene.anim)).is_null() {
        // C: `uc->scene.anim = ufbxi_push_zero(&uc->result, ufbx_anim, 1);`
        // (NOT `ufbxi_check`ed in C — a failed allocation leaves it NULL).
        *(ptr::addr_of_mut!((*uc.get()).scene.anim) as *mut *mut Anim) =
            push_zero::<Anim>(uc.result_mut_ptr(), 1);
    }

    if (*uc.get()).opts.load_external_files {
        ufbxi_check!(
            uc,
            load_external_files(uc).is_ok(),
            "ufbxi_load_external_files(uc)"
        );
    }

    // Evaluate skinning if requested
    if (*uc.get()).opts.evaluate_skinning {
        // C: `ufbx_geometry_cache_data_opts cache_opts = { 0 };`
        let mut cache_opts: RawGeometryCacheDataOpts = MaybeUninit::zeroed().assume_init();
        cache_opts.open_file_cb =
            ptr::read(ptr::addr_of!((*uc.get()).opts.open_file_cb) as *const RawOpenFileCb);
        ufbxi_check!(
            uc,
            evaluate_skinning(
                ptr::addr_of_mut!((*uc.get()).scene),
                uc.error_mut_ptr(),
                uc.result_mut_ptr(),
                uc.tmp_mut_ptr(),
                0.0,
                (*uc.get()).opts.load_external_files && (*uc.get()).opts.evaluate_caches,
                &mut cache_opts,
            )
            .is_ok(),
            "ufbxi_evaluate_skinning(&uc->scene, &uc->error, &uc->result, &uc->tmp, 0.0, uc->opts.load_external_files && uc->opts.evaluate_caches, &cache_opts)"
        );
    }

    // Pop warnings to metadata
    ufbxi_check!(
        uc,
        pop_warnings(
            ptr::addr_of_mut!((*uc.get()).warnings),
            ptr::addr_of_mut!((*uc.get()).scene.metadata.warnings),
            (*uc.get()).scene.metadata.has_warning.as_mut_ptr(),
        )
        .is_ok(),
        "ufbxi_pop_warnings(&uc->warnings, &uc->scene.metadata.warnings, uc->scene.metadata.has_warning)"
    );
    ufbxi_check!(
        uc,
        resolve_warning_elements(uc).is_ok(),
        "ufbxi_resolve_warning_elements(uc)"
    );

    // Copy local data to the scene
    (*uc.get()).scene.metadata.version = uc.version();
    (*uc.get()).scene.metadata.ascii = uc.from_ascii();
    (*uc.get()).scene.metadata.big_endian = uc.file_big_endian();
    (*uc.get()).scene.metadata.geometry_ignored = (*uc.get()).opts.ignore_geometry;
    (*uc.get()).scene.metadata.animation_ignored = (*uc.get()).opts.ignore_animation;
    (*uc.get()).scene.metadata.embedded_ignored = (*uc.get()).opts.ignore_embedded;

    // Retain the scene, this must be the final allocation as we copy
    // `ator_result` to `ufbx_scene_imp`.
    let imp: *mut SceneImp = push::<SceneImp>(uc.result_mut_ptr(), 1);
    ufbxi_check!(uc, !imp.is_null(), "imp");

    // Expose the wide allocation so `get_imp` can recover this header from a
    // (possibly narrowed) public `&Scene` pointer via exposed provenance.
    (imp as *mut u8).expose_provenance();

    init_ref(&mut (*imp).refcount, SCENE_IMP_MAGIC, ptr::null_mut());

    (*imp).magic = SCENE_IMP_MAGIC;
    // C: `imp->scene = uc->scene;` (struct copy)
    ptr::copy_nonoverlapping(
        ptr::addr_of!((*uc.get()).scene),
        ptr::addr_of_mut!((*imp).scene),
        1,
    );
    (*imp).refcount.ator = (*uc.get()).ator_result;
    (*imp).refcount.ator.error = ptr::null_mut();

    // Copy retained buffers and translate the allocator struct to the one
    // contained within `ufbxi_scene_imp`
    (*imp).refcount.buf = (*uc.get()).result;
    (*imp).refcount.buf.ator = ptr::addr_of_mut!((*imp).refcount.ator);
    (*imp).string_buf = (*uc.get()).string_pool.buf;
    (*imp).string_buf.ator = ptr::addr_of_mut!((*imp).refcount.ator);

    (*imp).scene.metadata.result_memory_used = (*imp).refcount.ator.current_size;
    (*imp).scene.metadata.temp_memory_used = (*uc.ator_tmp_mut_ptr()).current_size;
    (*imp).scene.metadata.result_allocs = (*imp).refcount.ator.num_allocs;
    (*imp).scene.metadata.temp_allocs = (*uc.ator_tmp_mut_ptr()).num_allocs;

    // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, imp->scene.elements)`
    let mut p_elem: *mut *mut Element = (*imp).scene.elements.data as *mut *mut Element;
    let p_elem_end: *mut *mut Element = add_ptr(p_elem, (*imp).scene.elements.count);
    while p_elem != p_elem_end {
        // C: `(*p_elem)->scene = &imp->scene;`
        *(ptr::addr_of_mut!((**p_elem).scene) as *mut *mut Scene) = ptr::addr_of_mut!((*imp).scene);
        p_elem = p_elem.add(1);
    }

    uc.set_scene_imp(imp);

    Ok(())
}

// ufbx.c:25412-25462 `ufbxi_free_temp`
#[inline(never)]
pub(crate) unsafe fn free_temp(uc: &Context) {
    thread_pool_free(uc.thread_pool_mut_ptr());

    string_pool_temp_free(uc.string_pool_mut_ptr());
    buf_free(&mut (*uc.get()).warnings.tmp_stack);

    map_free(&mut (*uc.get()).prop_type_map);
    map_free(&mut (*uc.get()).fbx_id_map);
    map_free(&mut (*uc.get()).ptr_fbx_id_map);
    map_free(&mut (*uc.get()).texture_file_map);
    map_free(&mut (*uc.get()).anim_stack_map);
    map_free(&mut (*uc.get()).fbx_attr_map);
    map_free(&mut (*uc.get()).node_prop_set);
    map_free(&mut (*uc.get()).dom_node_map);

    buf_free(uc.tmp_mut_ptr());
    buf_free(uc.tmp_parse_mut_ptr());
    for i in 0..THREAD_GROUP_COUNT {
        buf_free(&mut (*uc.get()).tmp_thread_parse[i]);
    }
    buf_free(uc.tmp_stack_mut_ptr());
    buf_free(uc.tmp_connections_mut_ptr());
    buf_free(uc.tmp_node_ids_mut_ptr());
    buf_free(&mut (*uc.get()).tmp_elements);
    buf_free(&mut (*uc.get()).tmp_element_offsets);
    buf_free(&mut (*uc.get()).tmp_element_fbx_ids);
    buf_free(&mut (*uc.get()).tmp_element_ptrs);
    for i in 0..ELEMENT_TYPE_COUNT {
        buf_free(&mut (*uc.get()).tmp_typed_element_offsets[i]);
    }
    buf_free(&mut (*uc.get()).tmp_mesh_textures);
    buf_free(&mut (*uc.get()).tmp_full_weights);
    buf_free(&mut (*uc.get()).tmp_dom_nodes);
    buf_free(&mut (*uc.get()).tmp_element_id);
    buf_free(&mut (*uc.get()).tmp_ascii_spans);

    free::<Node>(uc.ator_tmp_mut_ptr(), uc.top_nodes(), uc.top_nodes_cap());
    free::<*mut c_void>(
        uc.ator_tmp_mut_ptr(),
        uc.element_extra_arr(),
        uc.element_extra_cap(),
    );

    free::<u8>(
        uc.ator_tmp_mut_ptr(),
        (*uc.get()).ascii.token.str_data,
        (*uc.get()).ascii.token.str_cap,
    );
    free::<u8>(
        uc.ator_tmp_mut_ptr(),
        (*uc.get()).ascii.prev_token.str_data,
        (*uc.get()).ascii.prev_token.str_cap,
    );

    free::<u8>(
        uc.ator_tmp_mut_ptr(),
        uc.read_buffer(),
        uc.read_buffer_size(),
    );
    free::<u8>(
        uc.ator_tmp_mut_ptr(),
        uc.tmp_arr(),
        (*uc.get()).tmp_arr_size,
    );
    free::<u8>(uc.ator_tmp_mut_ptr(), uc.swap_arr(), uc.swap_arr_size());

    obj_free(uc);

    free_ator(uc.ator_tmp_mut_ptr());
}

// ufbx.c:25464-25470 `ufbxi_free_result`
#[inline(never)]
pub(crate) unsafe fn free_result(uc: &Context) {
    buf_free(uc.result_mut_ptr());
    buf_free(&mut (*uc.get()).string_pool.buf);

    free_ator(&raw mut (*uc.get()).ator_result);
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
        ptr::copy_nonoverlapping(user_opts, ptr::addr_of_mut!((*uc.get()).opts), 1);
    } else {
        // C: `memset(&uc->opts, 0, sizeof(uc->opts));`
        ptr::write_bytes(
            ptr::addr_of_mut!((*uc.get()).opts) as *mut u8,
            0,
            size_of::<RawLoadOpts>(),
        );
    }

    if (*uc.get()).opts.file_size_estimate != 0 {
        uc.set_progress_bytes_total((*uc.get()).opts.file_size_estimate);
    }

    if (*uc.get()).opts.ignore_all_content {
        (*uc.get()).opts.ignore_geometry = true;
        (*uc.get()).opts.ignore_animation = true;
        (*uc.get()).opts.ignore_embedded = true;
    }

    // C: `ufbx_inflate_retain inflate_retain; inflate_retain.initialized = false;`
    // — only `initialized` is written before use.
    let mut inflate_retain = MaybeUninit::<InflateRetain>::uninit();
    ptr::addr_of_mut!((*inflate_retain.as_mut_ptr()).initialized).write(false);

    init_ator(
        uc.error_mut_ptr(),
        uc.ator_tmp_mut_ptr(),
        ptr::addr_of!((*uc.get()).opts.temp_allocator),
        b"temp\0".as_ptr(),
    );
    init_ator(
        uc.error_mut_ptr(),
        &raw mut (*uc.get()).ator_result,
        ptr::addr_of!((*uc.get()).opts.result_allocator),
        b"result\0".as_ptr(),
    );

    if (*uc.get()).opts.read_buffer_size == 0 {
        (*uc.get()).opts.read_buffer_size = 0x4000;
    }
    if (*uc.get()).opts.read_buffer_size <= 32 {
        (*uc.get()).opts.read_buffer_size = 32;
    }

    if (*uc.get()).opts.file_format_lookahead == 0 {
        (*uc.get()).opts.file_format_lookahead = 0x4000;
    } else if (*uc.get()).opts.file_format_lookahead < MIN_FILE_FORMAT_LOOKAHEAD {
        (*uc.get()).opts.file_format_lookahead = MIN_FILE_FORMAT_LOOKAHEAD;
    }

    if (*uc.get()).opts.path_separator == 0 {
        (*uc.get()).opts.path_separator = PATH_SEPARATOR;
    }

    if (*uc.get()).opts.progress_cb.fn_.is_none()
        || (*uc.get()).opts.progress_interval_hint >= usize::MAX as u64
    {
        uc.set_progress_interval(usize::MAX);
    } else if (*uc.get()).opts.progress_interval_hint > 0 {
        uc.set_progress_interval((*uc.get()).opts.progress_interval_hint as usize);
    } else {
        uc.set_progress_interval(0x4000);
    }

    if (*uc.get()).opts.open_file_cb.fn_.is_none() {
        // C: `uc->opts.open_file_cb.fn = &ufbx_default_open_file;`
        (*uc.get()).opts.open_file_cb.fn_ = Some(default_open_file);
    }

    if (*uc.get()).opts.thread_opts.memory_limit == 0 {
        (*uc.get()).opts.thread_opts.memory_limit = 32 * 1024 * 1024;
    }

    uc.set_synthetic_id_counter(SYNTHETIC_ID_START);

    (*uc.get()).string_pool.error = uc.error_mut_ptr();
    map_init(
        &mut (*uc.get()).string_pool.map,
        uc.ator_tmp_mut_ptr(),
        map_cmp_string,
        ptr::null_mut(),
    );
    (*uc.get()).string_pool.buf.ator = ptr::addr_of_mut!((*uc.get()).ator_result);
    (*uc.get()).string_pool.buf.unordered = true;
    (*uc.get()).string_pool.initial_size = 1024;
    (*uc.get()).string_pool.error_handling = (*uc.get()).opts.unicode_error_handling;

    map_init(
        &mut (*uc.get()).prop_type_map,
        uc.ator_tmp_mut_ptr(),
        map_cmp_const_char_ptr,
        ptr::null_mut(),
    );
    map_init(
        &mut (*uc.get()).fbx_id_map,
        uc.ator_tmp_mut_ptr(),
        map_cmp_uint64,
        ptr::null_mut(),
    );
    map_init(
        &mut (*uc.get()).ptr_fbx_id_map,
        uc.ator_tmp_mut_ptr(),
        map_cmp_ptr_id,
        ptr::null_mut(),
    );
    map_init(
        &mut (*uc.get()).texture_file_map,
        uc.ator_tmp_mut_ptr(),
        map_cmp_const_char_ptr,
        ptr::null_mut(),
    );
    map_init(
        &mut (*uc.get()).anim_stack_map,
        uc.ator_tmp_mut_ptr(),
        map_cmp_const_char_ptr,
        ptr::null_mut(),
    );
    map_init(
        &mut (*uc.get()).fbx_attr_map,
        uc.ator_tmp_mut_ptr(),
        map_cmp_uint64,
        ptr::null_mut(),
    );
    map_init(
        &mut (*uc.get()).node_prop_set,
        uc.ator_tmp_mut_ptr(),
        map_cmp_const_char_ptr,
        ptr::null_mut(),
    );
    map_init(
        &mut (*uc.get()).dom_node_map,
        uc.ator_tmp_mut_ptr(),
        map_cmp_uintptr,
        ptr::null_mut(),
    );

    (*uc.get()).tmp.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).tmp_parse.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).tmp_stack.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).tmp_connections.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).tmp_node_ids.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).tmp_elements.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).tmp_element_offsets.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).tmp_element_fbx_ids.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).tmp_element_ptrs.ator = uc.ator_tmp_mut_ptr();
    for i in 0..ELEMENT_TYPE_COUNT {
        (*uc.get()).tmp_typed_element_offsets[i].ator = uc.ator_tmp_mut_ptr();
    }
    (*uc.get()).tmp_mesh_textures.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).tmp_full_weights.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).tmp_dom_nodes.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).tmp_element_id.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).tmp_ascii_spans.ator = uc.ator_tmp_mut_ptr();

    for i in 0..THREAD_GROUP_COUNT {
        (*uc.get()).tmp_thread_parse[i].ator = uc.ator_tmp_mut_ptr();
        (*uc.get()).tmp_thread_parse[i].unordered = true;
        (*uc.get()).tmp_thread_parse[i].clearable = true;
    }

    (*uc.get()).result.ator = ptr::addr_of_mut!((*uc.get()).ator_result);

    (*uc.get()).tmp.unordered = true;
    (*uc.get()).tmp_parse.unordered = true;
    (*uc.get()).tmp_parse.clearable = true;
    (*uc.get()).result.unordered = true;

    (*uc.get()).warnings.error = uc.error_mut_ptr();
    (*uc.get()).warnings.result = uc.result_mut_ptr();
    (*uc.get()).warnings.tmp_stack.ator = uc.ator_tmp_mut_ptr();
    (*uc.get()).string_pool.warnings = ptr::addr_of_mut!((*uc.get()).warnings);

    // Set zero size `swap_arr` to a non-NULL buffer so we can tell the difference between empty
    // array and an allocation failure.
    // C: `uc->swap_arr = (char*)ufbxi_zero_size_buffer;` — the const cast is
    // C-parity: the buffer is replaced by `ufbxi_grow_array` before any write.
    uc.set_swap_arr(ZERO_SIZE_BUFFER.as_ptr() as *mut u8);

    // NOTE: Though `inflate_retain` leaks out of the scope we don't use it outside this function.
    // cppcheck-suppress autoVariables
    uc.set_inflate_retain(inflate_retain.as_mut_ptr());

    let ok: bool = load_imp(uc).is_ok();

    if uc.close_fn().is_some() {
        (uc.close_fn().unwrap())(uc.read_user());
    }

    free_temp(uc);

    if ok {
        if !p_error.is_null() {
            clear_error(p_error);
        }
        ptr::addr_of_mut!((*uc.scene_imp()).scene)
    } else {
        fix_error_type(uc.error_mut_ptr(), b"Failed to load\0".as_ptr(), p_error);
        if !p_error.is_null()
            && (*p_error).type_ == ErrorType::Unknown
            && (*uc.get()).scene.metadata.file_format == FileFormat::Fbx
            && !supports_version(uc.version())
        {
            (*p_error).description.data = b"Unsupported version\0".as_ptr();
            (*p_error).description.length = strlen(b"Unsupported version\0".as_ptr());
            (*p_error).type_ = ErrorType::UnsupportedVersion;
            ufbxi_fmt_err_info!(p_error, "%u", uc.version());
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
    if (*over).element_id != element_id {
        return (*over).element_id < element_id;
    }
    if (*over)._internal_key != (*prop)._internal_key {
        return (*over)._internal_key < (*prop)._internal_key;
    }
    // C: `return strcmp(over->prop_name.data, prop->name.data);` — the `int`
    // result converts to `bool` (nonzero == true), so ANY name difference
    // reports "less". Ported verbatim.
    strcmp((*over).prop_name.data, (*prop).name.data) != 0
}

// ufbx.c:25636-25641 `ufbxi_override_equals_to_prop`
// C: `ufbxi_forceinline`.
#[inline(always)]
pub(crate) unsafe fn override_equals_to_prop(
    over: *const PropOverride,
    element_id: u32,
    prop: *const Prop,
) -> bool {
    if (*over).element_id != element_id {
        return false;
    }
    if (*over)._internal_key != (*prop)._internal_key {
        return false;
    }
    strcmp((*over).prop_name.data, (*prop).name.data) == 0
}

// ufbx.c:25643-25664 `ufbxi_find_prop_override`
#[inline(never)]
pub(crate) unsafe fn find_prop_override(
    overrides: *const List<PropOverride>,
    element_id: u32,
    prop: *mut Prop,
) -> bool {
    let mut ix: usize = usize::MAX;
    macro_lower_bound_eq::<PropOverride>(
        16,
        &mut ix,
        (*overrides).data,
        0,
        (*overrides).count,
        |a| override_less_than_prop(a, element_id, prop),
        |a| override_equals_to_prop(a, element_id, prop),
    );

    if ix != usize::MAX {
        let over: *const PropOverride = (*overrides).data.add(ix);
        // C: `const uint32_t clear_flags = UFBX_PROP_FLAG_NO_VALUE | UFBX_PROP_FLAG_NOT_FOUND;`
        let clear_flags: u32 = PropFlags::NO_VALUE.raw() | PropFlags::NOT_FOUND.raw();
        (*prop).flags =
            PropFlags::from_raw(((*prop).flags.raw() & !clear_flags) | PropFlags::OVERRIDDEN.raw());
        (*prop).value_vec4 = (*over).value;
        // C: `prop->value_real_arr[3] = 0.0f;` — the `ufbx_prop` value union's
        // `ufbx_real value_real_arr[4]` view; the generated struct keeps only
        // `value_vec4`.
        *(ptr::addr_of_mut!((*prop).value_vec4) as *mut Real).add(3) = 0.0;
        (*prop).value_int = (*over).value_int;
        (*prop).value_str = (*over).value_str;
        (*prop).value_blob.data = (*prop).value_str.data;
        (*prop).value_blob.size = (*prop).value_str.length;
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
    let mut begin: usize = (*overrides).count;
    let mut end: usize = begin;

    macro_lower_bound_eq::<PropOverride>(
        32,
        &mut begin,
        (*overrides).data,
        0,
        (*overrides).count,
        |a| (*a).element_id < element_id,
        |a| (*a).element_id == element_id,
    );

    macro_upper_bound_eq::<PropOverride>(
        32,
        &mut end,
        (*overrides).data,
        begin,
        (*overrides).count,
        |a| (*a).element_id == element_id,
    );

    // C: `ufbx_prop_override_list result = { overrides->data + begin, end - begin };`
    // (`List<T>` carries a private `PhantomData` marker, so the aggregate
    // initializer becomes a zeroed value with both public fields written.)
    let mut result: List<PropOverride> = MaybeUninit::zeroed().assume_init();
    result.data = (*overrides).data.add(begin);
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
        combine_anim_layer_rec(ctx, layer, weight, prop_name, result, value);
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
    }
    #[cfg(not(feature = "regression"))]
    combine_anim_layer_rec(ctx, layer, weight, prop_name, result, value)
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
    if (*layer).compose_rotation
        && (*layer).blended
        && prop_name == sp::Lcl_Rotation.as_ptr()
        && !(*ctx).has_rotation_order
    {
        let rp: Prop = evaluate_prop_len(
            (*ctx).anim,
            (*ctx).element,
            sp::RotationOrder.as_ptr(),
            sp::RotationOrder.len() - 1,
            (*ctx).time,
        );
        // NOTE: Defaults to 0 (UFBX_ROTATION_XYZ) gracefully if property is not found
        if rp.value_int >= 0 && rp.value_int <= RotationOrder::Spheric as i64 {
            // C: `(ufbx_rotation_order)rp.value_int` — in-range by the guard.
            (*ctx).rotation_order = core::mem::transmute::<u32, RotationOrder>(rp.value_int as u32);
        } else {
            (*ctx).rotation_order = RotationOrder::Xyz;
        }
        (*ctx).has_rotation_order = true;
    }

    if (*layer).additive {
        if (*layer).compose_scale && prop_name == sp::Lcl_Scaling.as_ptr() {
            // C: `result->x *= (ufbx_real)ufbxi_pow_abs(value->x, weight);`
            // — `ufbxi_pow_abs` takes `double`, so both args promote to double
            // and the result narrows back to `ufbx_real` before the multiply.
            (*result).x *= pow_abs((*value).x as f64, weight as f64) as Real;
            (*result).y *= pow_abs((*value).y as f64, weight as f64) as Real;
            (*result).z *= pow_abs((*value).z as f64, weight as f64) as Real;
        } else if (*layer).compose_rotation && prop_name == sp::Lcl_Rotation.as_ptr() {
            let a: Quat = euler_to_quat(*result, (*ctx).rotation_order);
            let mut b: Quat = euler_to_quat(*value, (*ctx).rotation_order);
            b = quat_slerp(IDENTITY_QUAT, b, weight);
            let res: Quat = mul_quat(a, b);
            *result = quat_to_euler(res, (*ctx).rotation_order);
        } else {
            (*result).x += (*value).x * weight;
            (*result).y += (*value).y * weight;
            (*result).z += (*value).z * weight;
        }
    } else if (*layer).blended {
        // C: `ufbx_real res_weight = 1.0f - weight;`
        let res_weight: Real = 1.0 - weight;
        if (*layer).compose_scale && prop_name == sp::Lcl_Scaling.as_ptr() {
            // C: `result->x = (ufbx_real)(ufbxi_pow_abs(result->x, res_weight) * ufbxi_pow_abs(value->x, weight));`
            // — `ufbxi_pow_abs` takes `double`; the product stays in double and
            // narrows to `ufbx_real` only on the assignment.
            (*result).x = (pow_abs((*result).x as f64, res_weight as f64)
                * pow_abs((*value).x as f64, weight as f64)) as Real;
            (*result).y = (pow_abs((*result).y as f64, res_weight as f64)
                * pow_abs((*value).y as f64, weight as f64)) as Real;
            (*result).z = (pow_abs((*result).z as f64, res_weight as f64)
                * pow_abs((*value).z as f64, weight as f64)) as Real;
        } else if (*layer).compose_rotation && prop_name == sp::Lcl_Rotation.as_ptr() {
            let a: Quat = euler_to_quat(*result, (*ctx).rotation_order);
            let b: Quat = euler_to_quat(*value, (*ctx).rotation_order);
            let res: Quat = quat_slerp(a, b, weight);
            *result = quat_to_euler(res, (*ctx).rotation_order);
        } else {
            (*result).x = (*result).x * res_weight + (*value).x * weight;
            (*result).y = (*result).y * res_weight + (*value).y * weight;
            (*result).z = (*result).z * res_weight + (*value).z * weight;
        }
    } else {
        *result = *value;
    }
}

// ufbx.c:25751-25757 `ufbxi_anim_layer_might_contain_id`
// C: `ufbxi_forceinline`.
#[inline(always)]
pub(crate) unsafe fn anim_layer_might_contain_id(layer: *const AnimLayer, id: u32) -> bool {
    // C: `uint32_t id_mask = ufbxi_arraycount(layer->_element_id_bitmask) - 1;`
    let id_mask: u32 = ((*layer)._element_id_bitmask.len() - 1) as u32;
    // C: `bool ok = id - layer->_min_element_id <= (layer->_max_element_id - layer->_min_element_id);`
    // — unsigned wrapping subtraction.
    let mut ok: bool = id.wrapping_sub((*layer)._min_element_id)
        <= (*layer)
            ._max_element_id
            .wrapping_sub((*layer)._min_element_id);
    ok &= ((*layer)._element_id_bitmask[((id >> 5) & id_mask) as usize] & (1u32 << (id & 31))) != 0;
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

    let element_id: u32 = (*element).element_id;
    let num_layers: usize = (*anim).layers.count;
    for layer_ix in 0..num_layers {
        let layer: *mut AnimLayer = *((*anim).layers.data as *mut *mut AnimLayer).add(layer_ix);
        if !anim_layer_might_contain_id(layer, element_id) {
            continue;
        }

        // Find the weight for the current layer
        // TODO: Should this be searched from multiple layers?
        let mut weight: Real = if layer_ix < (*anim).override_layer_weights.count {
            *(*anim).override_layer_weights.data.add(layer_ix)
        } else {
            (*layer).weight
        };
        if (*layer).weight_is_animated && (*layer).blended {
            let weight_aprop: *mut AnimProp =
                find_anim_prop_start(layer, ptr::addr_of!((*layer).element));
            if !weight_aprop.is_null() {
                // C: `weight = ufbx_evaluate_anim_value_real_flags(...) / (ufbx_real)100.0;`
                weight = evaluate_anim_value_real_flags(
                    ref_ptr(&(*weight_aprop).anim_value),
                    time,
                    flags,
                ) / (100.0 as Real);
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

        let mut aprop: *mut AnimProp = find_anim_prop_start(layer, element);
        if aprop.is_null() {
            continue;
        }

        for i in 0..num_props {
            let prop: *mut Prop = props.add(i);

            // Don't evaluate on top of overridden properties
            if ((*prop).flags.raw() & PropFlags::OVERRIDDEN.raw()) != 0 {
                continue;
            }

            // Connections override animation by default
            if ((*prop).flags.raw() & PropFlags::CONNECTED.raw()) != 0
                && !(*anim).ignore_connections
            {
                continue;
            }

            // Skip until we reach `aprop >= prop`
            // NOTE: No need to check for end as `anim_props` is terminated with a NULL sentinel.
            while ref_ptr(&(*aprop).element) as *const Element == element
                && (*aprop)._internal_key < (*prop)._internal_key
            {
                aprop = aprop.add(1);
            }
            if (*aprop).prop_name.data != (*prop).name.data {
                while ref_ptr(&(*aprop).element) as *const Element == element
                    && strcmp((*aprop).prop_name.data, (*prop).name.data) < 0
                {
                    aprop = aprop.add(1);
                }
            }

            // TODO: Should we skip the blending for the first layer _per property_
            // This could be done by having `UFBX_PROP_FLAG_ANIMATION_EVALUATED`
            // that gets set for the first layer of animation that is applied.
            if (*aprop).prop_name.data == (*prop).name.data {
                let v: Vec3 =
                    evaluate_anim_value_vec3_flags(ref_ptr(&(*aprop).anim_value), time, flags);
                if layer_ix == 0 {
                    // C: `prop->value_vec3 = v;` — the `ufbx_prop` value
                    // union's 3-real view over `value_vec4`.
                    *(ptr::addr_of_mut!((*prop).value_vec4) as *mut Vec3) = v;
                } else {
                    combine_anim_layer(
                        &mut combine_ctx,
                        layer,
                        weight,
                        (*prop).name.data,
                        ptr::addr_of_mut!((*prop).value_vec4) as *mut Vec3,
                        &v,
                    );
                }
            }
        }
    }

    // C: `ufbxi_for(ufbx_prop, prop, props, num_props)`
    let mut prop: *mut Prop = props;
    let prop_end: *mut Prop = add_ptr(props, num_props);
    while prop != prop_end {
        if ((*prop).flags.raw() & PropFlags::OVERRIDDEN.raw()) != 0 {
            prop = prop.add(1);
            continue;
        }
        // C: `prop->value_int = ufbxi_f64_to_i64(prop->value_real);` — the
        // value union's first real is `value_vec4.x`; `ufbxi_f64_to_i64` takes
        // `double`, so the `ufbx_real` argument promotes.
        (*prop).value_int = f64_to_i64((*prop).value_vec4.x as f64);
        prop = prop.add(1);
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
        evaluate_connected_prop_rec(prop, anim, element, name, time, flags);
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
    }
    #[cfg(not(feature = "regression"))]
    evaluate_connected_prop_rec(prop, anim, element, name, time, flags)
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
    let mut conn: *mut Connection = find_prop_connection(element, name);

    // C: `for (size_t i = 0; i < 1000 && conn; i++)`
    let mut i: usize = 0;
    while i < 1000 && !conn.is_null() {
        let next_conn: *mut Connection =
            find_prop_connection(ref_ptr(&(*conn).src), (*conn).src_prop.data);
        if next_conn.is_null() {
            break;
        }
        conn = next_conn;
        i += 1;
    }

    // Found a non-cyclic connection
    if !conn.is_null()
        && find_prop_connection(ref_ptr(&(*conn).src), (*conn).src_prop.data).is_null()
    {
        let ep: Prop = evaluate_prop_flags_len(
            anim,
            ref_ptr(&(*conn).src),
            (*conn).src_prop.data,
            (*conn).src_prop.length,
            time,
            flags,
        );
        (*prop).value_vec4 = ep.value_vec4;
        (*prop).value_int = ep.value_int;
        (*prop).value_str = ep.value_str;
        (*prop).value_blob = ep.value_blob;
    } else {
        // Connection not found, maybe it's animated?
        (*prop).flags = PropFlags::from_raw((*prop).flags.raw() & !PropFlags::CONNECTED.raw());
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
    (*iter).prop = (*element).props.props.data;
    // C: `iter->prop_end = element->props.props.data + element->props.props.count;`
    (*iter).prop_end = (*element)
        .props
        .props
        .data
        .add((*element).props.props.count);

    let over: List<PropOverride> =
        find_element_prop_overrides(ptr::addr_of!((*anim).prop_overrides), (*element).element_id);
    (*iter).over = over.data;
    (*iter).over_end = over.data.add(over.count);
    if over.count > 0 {
        // C: `memset(&iter->tmp, 0, sizeof(ufbx_prop));`
        ptr::write_bytes(
            ptr::addr_of_mut!((*iter).tmp) as *mut u8,
            0,
            size_of::<Prop>(),
        );
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
    (*iter).prop = (*element).props.props.data;
    (*iter).prop_end = add_ptr(
        (*element).props.props.data as *mut Prop,
        (*element).props.props.count,
    );
    // C: `iter->over = iter->over_end = NULL;`
    (*iter).over = ptr::null();
    (*iter).over_end = ptr::null();
    if (*anim).prop_overrides.count > 0 {
        init_prop_iter_slow(iter, anim, element);
    }
}

// ufbx.c:25876-25914 `ufbxi_next_prop_slow`
#[inline(never)]
pub(crate) unsafe fn next_prop_slow(iter: *mut PropIter) -> *const Prop {
    let prop: *const Prop = (*iter).prop;
    let over: *const PropOverride = (*iter).over;
    if prop == (*iter).prop_end && over == (*iter).over_end {
        return ptr::null();
    }

    // We can use `UINT32_MAX` as a terminating key (aka prefix) as prop names must
    // be valid UTF-8 and the byte sequence "\xff\xff\xff\xff" is not valid.
    let prop_key: u32 = if prop != (*iter).prop_end {
        (*prop)._internal_key
    } else {
        u32::MAX
    };
    let over_key: u32 = if over != (*iter).over_end {
        (*over)._internal_key
    } else {
        u32::MAX
    };

    // C: `int cmp = 0;`
    let cmp: i32;
    if prop_key != over_key {
        cmp = if prop_key < over_key { -1 } else { 1 };
    } else {
        cmp = strcmp((*prop).name.data, (*over).prop_name.data);
    }

    if cmp >= 0 {
        let dst: *mut Prop = ptr::addr_of_mut!((*iter).tmp);
        (*dst).name = (*over).prop_name;
        (*dst)._internal_key = (*over)._internal_key;
        (*dst).type_ = PropType::Unknown;
        (*dst).flags = PropFlags::OVERRIDDEN;
        (*dst).value_str = (*over).value_str;
        (*dst).value_blob.data = (*dst).value_str.data;
        (*dst).value_blob.size = (*dst).value_str.length;
        (*dst).value_int = (*over).value_int;
        (*dst).value_vec4 = (*over).value;
        (*iter).over = over.add(1);
        if cmp == 0 {
            (*iter).prop = prop.add(1);
        }
        dst
    } else {
        (*iter).prop = prop.add(1);
        prop
    }
}

// ufbx.c:25916-25924 `ufbxi_next_prop`
// C: `ufbxi_forceinline`.
#[inline(always)]
pub(crate) unsafe fn next_prop(iter: *mut PropIter) -> *const Prop {
    if (*iter).over == (*iter).over_end {
        if (*iter).prop == (*iter).prop_end {
            return ptr::null();
        }
        // C: `return iter->prop++;`
        let prop: *const Prop = (*iter).prop;
        (*iter).prop = prop.add(1);
        prop
    } else {
        next_prop_slow(iter)
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
    let mut name: *const u8 = *prop_names.add(0);
    let mut key: u32 = get_name_key_c(name);
    let mut num_props: usize = 0;

    // C: `#if defined(UFBX_REGRESSION)` — sorted-names assert.
    #[cfg(feature = "regression")]
    {
        for i in 1..max_props {
            ufbx_assert!(strcmp(*prop_names.add(i - 1), *prop_names.add(i)) < 0);
        }
    }

    let mut name_ix: usize = 0;

    let mut iter = MaybeUninit::<PropIter>::uninit(); // ufbxi_uninit
    let iter: *mut PropIter = iter.as_mut_ptr();
    init_prop_iter(iter, anim, element);
    // C: `while ((prop = ufbxi_next_prop(&iter)) != NULL)`
    loop {
        let prop: *const Prop = next_prop(iter);
        if prop.is_null() {
            break;
        }
        while name_ix < max_props {
            if key > (*prop)._internal_key {
                break;
            }
            if name == (*prop).name.data {
                if ((*prop).flags.raw() & PropFlags::CONNECTED.raw()) != 0
                    && !(*anim).ignore_connections
                {
                    // C: `ufbx_prop *dst = &props[num_props++];`
                    let dst: *mut Prop = props.add(num_props);
                    num_props += 1;
                    *dst = *prop;
                    evaluate_connected_prop(dst, anim, element, name, time, flags);
                } else if ((*prop).flags.raw()
                    & (PropFlags::ANIMATED.raw() | PropFlags::OVERRIDDEN.raw()))
                    != 0
                {
                    // C: `props[num_props++] = *prop;`
                    *props.add(num_props) = *prop;
                    num_props += 1;
                }
                break;
            } else if strcmp(name, (*prop).name.data) < 0 {
                name_ix += 1;
                if name_ix < max_props {
                    name = *prop_names.add(name_ix);
                    key = get_name_key_c(name);
                }
            } else {
                break;
            }
        }
    }

    evaluate_props(anim, element, time, props, num_props, flags);

    // C: `ufbx_props prop_list;` — every field is written below.
    let mut prop_list: crate::generated::Props = MaybeUninit::zeroed().assume_init();
    prop_list.props.data = props;
    // C: `prop_list.props.count = prop_list.num_animated = num_props;`
    prop_list.props.count = num_props;
    prop_list.num_animated = num_props;
    // C: `prop_list.defaults = (ufbx_props*)&element->props;` — raw pointer
    // store into the `Option<Ref<Props>>` slot (same layout).
    *(ptr::addr_of_mut!(prop_list.defaults) as *mut *const crate::generated::Props) =
        ptr::addr_of!((*element).props);
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
        let ret = extrapolate_curve_rec(curve, real_time, flags);
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        return ret;
    }
    #[cfg(not(feature = "regression"))]
    extrapolate_curve_rec(curve, real_time, flags)
}

// ufbx.c:25979-26042 `ufbxi_extrapolate_curve` body (the `_rec` half of the
// `ufbxi_recursive_function` body; see the wrapper above)
#[inline(never)]
unsafe fn extrapolate_curve_rec(curve: *const AnimCurve, real_time: f64, flags: u32) -> Real {
    let pre: bool = real_time < (*curve).min_time;
    let key: *const Keyframe;
    // C: `ufbx_extrapolation ext;` — copied by value; read through a pointer
    // here (`Extrapolation` carries no `Copy`), same fields, same reads.
    let ext: *const Extrapolation;
    if pre {
        key = (*curve).keyframes.data.add(0);
        ext = ptr::addr_of!((*curve).pre_extrapolation);
    } else {
        key = (*curve).keyframes.data.add((*curve).keyframes.count - 1);
        ext = ptr::addr_of!((*curve).post_extrapolation);
    }

    if (*ext).mode == ExtrapolationMode::Constant {
        return (*key).value;
    } else if (*ext).mode == ExtrapolationMode::Slope {
        // C: `ufbx_tangent tangent = *(pre ? &key->right : &key->left);`
        let tangent: Tangent = *(if pre {
            ptr::addr_of!((*key).right)
        } else {
            ptr::addr_of!((*key).left)
        });
        // C: `key->value + (ufbx_real)(tangent.dy * ((real_time - key->time) / tangent.dx))`
        // — `dx`/`dy` are float, promoted to double in the expression.
        return (*key).value
            + (tangent.dy as f64 * ((real_time - (*key).time) / tangent.dx as f64)) as Real;
    } else if (*ext).repeat_count == 0 {
        return (*key).value;
    }

    // Perform all operations in KTime ticks to be frame perfect
    let scale: f64 = (*ref_ptr(&(*curve).element.scene)).metadata.ktime_second as f64;
    let min_time: f64 = math::rint((*curve).min_time * scale);
    let max_time: f64 = math::rint((*curve).max_time * scale);
    let time: f64 = real_time * scale;

    let delta: f64 = if pre {
        min_time - time
    } else {
        time - max_time
    };
    let duration: f64 = max_time - min_time;

    // Require at least one KTime unit
    if !(duration >= 1.0) {
        return (*key).value;
    }

    let rep: f64 = delta / duration;
    let mut rep_n: f64 = math::floor(rep);
    let mut rep_d: f64 = delta - rep_n * duration;

    if (*ext).repeat_count > 0 && rep_n >= (*ext).repeat_count as f64 {
        // Clamp to the repeat count to handle mirroring
        rep_n = ((*ext).repeat_count - 1) as f64;
        rep_d = duration;
    }

    if (*ext).mode == ExtrapolationMode::Mirror {
        let rep_parity: f64 = rep_n * 0.5 - math::floor(rep_n * 0.5);
        if rep_parity <= 0.25 {
            rep_d = duration - rep_d;
        }
    }

    if pre {
        rep_d = duration - rep_d;
    }
    let new_time: f64 = (min_time + rep_d) / scale;

    let mut value: Real = evaluate_curve_flags(
        curve,
        new_time,
        (*key).value,
        flags | crate::generated::EvaluateFlags::NO_EXTRAPOLATION.raw(),
    );

    if (*ext).mode == ExtrapolationMode::RepeatRelative {
        let mut val_delta: Real = (*(*curve).keyframes.data.add((*curve).keyframes.count - 1))
            .value
            - (*(*curve).keyframes.data.add(0)).value;
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
pub(crate) struct EvalContext(
    pub(crate) core::cell::UnsafeCell<core::mem::MaybeUninit<InnerEvalContext>>,
);

impl EvalContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerEvalContext {
        self.0.get().cast()
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

    // `ator_tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_tmp_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_tmp }
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
        (*ec.get())
            .dst_element
            .offset((elem as *mut u8).offset_from(ec.src_element())) as *mut Element
    } else {
        ptr::null_mut()
    }
}

// ufbx.c:26075-26087 `ufbxi_translate_element_list`
#[cfg(feature = "scene-eval")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn translate_element_list(
    ec: &EvalContext,
    p_list: *mut c_void,
) -> Result<(), Fail> {
    // C: `ufbx_element_list *list = (ufbx_element_list*)p_list;`
    let list: *mut crate::prelude::RefList<Element> =
        p_list as *mut crate::prelude::RefList<Element>;
    let count: usize = (*list).count;
    let src: *mut *mut Element = (*list).data as *mut *mut Element;
    let dst: *mut *mut Element = push::<*mut Element>(ec.result_mut_ptr(), count);
    ufbxi_check_err!(ec.error_mut_ptr(), !dst.is_null(), "dst");
    (*list).data = dst as *const Ref<Element>;
    for i in 0..count {
        *dst.add(i) = translate_element(ec, *src.add(i) as *mut c_void);
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
        *(ptr::addr_of_mut!((*map).texture) as *mut *mut Texture) =
            translate_element(ec, opt_ptr(ptr::addr_of!((*map).texture)) as *mut c_void)
                as *mut Texture;
        map = map.add(1);
    }
}

// ufbx.c:26096-26103 `ufbxi_translate_anim`
#[cfg(feature = "scene-eval")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn translate_anim(ec: &EvalContext, p_anim: *mut *mut Anim) -> Result<(), Fail> {
    let anim: *mut Anim = push_copy::<Anim>(ec.result_mut_ptr(), 1, *p_anim);
    ufbxi_check_err!(ec.error_mut_ptr(), !anim.is_null(), "anim");
    translate_element_list(ec, ptr::addr_of_mut!((*anim).layers) as *mut c_void)?;
    *p_anim = anim;
    Ok(())
}

// ufbx.c:26105-26444 `ufbxi_evaluate_imp`
#[cfg(feature = "scene-eval")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn evaluate_imp(ec: &EvalContext) -> Result<(), Fail> {
    // C: `ec->scene = ec->src_scene;` — struct assignment (memcpy).
    ptr::copy_nonoverlapping(ptr::addr_of!((*ec.get()).src_scene), ec.scene_mut_ptr(), 1);
    let num_elements: usize = (*ec.get()).scene.elements.count;

    // C: `char *element_data = (char*)ufbxi_push(&ec->result, uint64_t, ec->scene.metadata.element_buffer_size/8);`
    let element_data: *mut u8 = push::<u64>(
        ec.result_mut_ptr(),
        (*ec.get()).scene.metadata.element_buffer_size / 8,
    ) as *mut u8;
    ufbxi_check_err!(ec.error_mut_ptr(), !element_data.is_null(), "element_data");

    (*ec.get()).scene.elements.data =
        push::<*mut Element>(ec.result_mut_ptr(), num_elements) as *const Ref<Element>;
    ufbxi_check_err!(
        ec.error_mut_ptr(),
        !(*ec.get()).scene.elements.data.is_null(),
        "ec->scene.elements.data"
    );

    // C: `ec->src_element = (char*)ec->src_scene.elements.data[0];`
    ec.set_src_element(*((*ec.get()).src_scene.elements.data as *mut *mut u8).add(0));
    ec.set_dst_element(element_data);

    // C indexes `ec->scene.elements_by_type[i]`, the `ufbx_element_list` array
    // view of the `ufbx_scene` per-type list union (ufbx.h); the generated
    // struct keeps only the named branch, whose first member (`unknowns`) is
    // the array base (same treatment as `native::scene_process`).
    let by_type: *mut crate::prelude::RefList<Element> =
        ptr::addr_of_mut!((*ec.get()).scene.unknowns) as *mut crate::prelude::RefList<Element>;
    for i in 0..ELEMENT_TYPE_COUNT {
        (*by_type.add(i)).data = push::<*mut Element>(ec.result_mut_ptr(), (*by_type.add(i)).count)
            as *const Ref<Element>;
        ufbxi_check_err!(
            ec.error_mut_ptr(),
            !(*by_type.add(i)).data.is_null(),
            "ec->scene.elements_by_type[i].data"
        );
    }

    let num_connections: usize = (*ec.get()).scene.connections_dst.count;
    (*ec.get()).scene.connections_src.data =
        push::<Connection>(ec.result_mut_ptr(), num_connections);
    (*ec.get()).scene.connections_dst.data =
        push::<Connection>(ec.result_mut_ptr(), num_connections);
    ufbxi_check_err!(
        ec.error_mut_ptr(),
        !(*ec.get()).scene.connections_src.data.is_null(),
        "ec->scene.connections_src.data"
    );
    ufbxi_check_err!(
        ec.error_mut_ptr(),
        !(*ec.get()).scene.connections_dst.data.is_null(),
        "ec->scene.connections_dst.data"
    );
    for i in 0..num_connections {
        let src: *mut Connection =
            ((*ec.get()).scene.connections_src.data as *mut Connection).add(i);
        let dst: *mut Connection =
            ((*ec.get()).scene.connections_dst.data as *mut Connection).add(i);
        // C: `*src = ec->src_scene.connections_src.data[i];` (struct assignment)
        ptr::copy_nonoverlapping((*ec.get()).src_scene.connections_src.data.add(i), src, 1);
        ptr::copy_nonoverlapping((*ec.get()).src_scene.connections_dst.data.add(i), dst, 1);
        *(ptr::addr_of_mut!((*src).src) as *mut *mut Element) =
            translate_element(ec, ref_ptr(&(*src).src) as *mut c_void);
        *(ptr::addr_of_mut!((*src).dst) as *mut *mut Element) =
            translate_element(ec, ref_ptr(&(*src).dst) as *mut c_void);
        *(ptr::addr_of_mut!((*dst).src) as *mut *mut Element) =
            translate_element(ec, ref_ptr(&(*dst).src) as *mut c_void);
        *(ptr::addr_of_mut!((*dst).dst) as *mut *mut Element) =
            translate_element(ec, ref_ptr(&(*dst).dst) as *mut c_void);
    }

    (*ec.get()).scene.elements_by_name.data =
        push::<NameElement>(ec.result_mut_ptr(), num_elements);
    ufbxi_check_err!(
        ec.error_mut_ptr(),
        !(*ec.get()).scene.elements_by_name.data.is_null(),
        "ec->scene.elements_by_name.data"
    );

    // C: `ec->scene.root_node = (ufbx_node*)ufbxi_translate_element(ec, ec->scene.root_node);`
    *(ptr::addr_of_mut!((*ec.get()).scene.root_node) as *mut *mut UfbxNode) =
        translate_element(ec, ref_ptr(&(*ec.get()).scene.root_node) as *mut c_void)
            as *mut UfbxNode;
    translate_anim(
        ec,
        ptr::addr_of_mut!((*ec.get()).scene.anim) as *mut *mut Anim,
    )?;

    for i in 0..num_elements {
        let src: *mut Element = *((*ec.get()).src_scene.elements.data as *mut *mut Element).add(i);
        let dst: *mut Element = translate_element(ec, src as *mut c_void);
        let size: usize = ELEMENT_TYPE_SIZE[(*src).type_ as usize];
        ufbx_assert!(size > 0);
        // C: `memcpy(dst, src, size);`
        ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, size);

        *((*ec.get()).scene.elements.data as *mut *mut Element).add(i) = dst;
        *((*by_type.add((*src).type_ as usize)).data as *mut *mut Element)
            .add((*src).typed_id as usize) = dst;

        // C: `dst->connections_src.data = ec->scene.connections_src.data + (dst->connections_src.data - ec->src_scene.connections_src.data);`
        (*dst).connections_src.data = (*ec.get()).scene.connections_src.data.offset(
            (*dst)
                .connections_src
                .data
                .offset_from((*ec.get()).src_scene.connections_src.data),
        );
        (*dst).connections_dst.data = (*ec.get()).scene.connections_dst.data.offset(
            (*dst)
                .connections_dst
                .data
                .offset_from((*ec.get()).src_scene.connections_dst.data),
        );
        if (*dst).instances.count > 0 {
            translate_element_list(ec, ptr::addr_of_mut!((*dst).instances) as *mut c_void)?;
        }

        // C: `ufbx_name_element named = ec->src_scene.elements_by_name.data[i];`
        // then `named.element = ...; ec->scene.elements_by_name.data[i] = named;`
        // — copied straight into the destination slot here (same writes).
        let named: *mut NameElement =
            ((*ec.get()).scene.elements_by_name.data as *mut NameElement).add(i);
        ptr::copy_nonoverlapping((*ec.get()).src_scene.elements_by_name.data.add(i), named, 1);
        *(ptr::addr_of_mut!((*named).element) as *mut *mut Element) =
            translate_element(ec, ref_ptr(&(*named).element) as *mut c_void);
    }

    // C: `ufbxi_for_ptr_list(ufbx_node, p_node, ec->scene.nodes)`
    let mut p_node: *mut *mut UfbxNode = (*ec.get()).scene.nodes.data as *mut *mut UfbxNode;
    let p_node_end: *mut *mut UfbxNode = add_ptr(p_node, (*ec.get()).scene.nodes.count);
    while p_node != p_node_end {
        let node: *mut UfbxNode = *p_node;
        *(ptr::addr_of_mut!((*node).parent) as *mut *mut UfbxNode) =
            translate_element(ec, opt_ptr(ptr::addr_of!((*node).parent)) as *mut c_void)
                as *mut UfbxNode;
        translate_element_list(ec, ptr::addr_of_mut!((*node).children) as *mut c_void)?;

        *(ptr::addr_of_mut!((*node).attrib) as *mut *mut Element) =
            translate_element(ec, opt_ptr(ptr::addr_of!((*node).attrib)) as *mut c_void);
        *(ptr::addr_of_mut!((*node).mesh) as *mut *mut Mesh) =
            translate_element(ec, opt_ptr(ptr::addr_of!((*node).mesh)) as *mut c_void) as *mut Mesh;
        *(ptr::addr_of_mut!((*node).light) as *mut *mut crate::generated::Light) =
            translate_element(ec, opt_ptr(ptr::addr_of!((*node).light)) as *mut c_void)
                as *mut crate::generated::Light;
        *(ptr::addr_of_mut!((*node).camera) as *mut *mut Camera) =
            translate_element(ec, opt_ptr(ptr::addr_of!((*node).camera)) as *mut c_void)
                as *mut Camera;
        *(ptr::addr_of_mut!((*node).bone) as *mut *mut crate::generated::Bone) =
            translate_element(ec, opt_ptr(ptr::addr_of!((*node).bone)) as *mut c_void)
                as *mut crate::generated::Bone;
        *(ptr::addr_of_mut!((*node).inherit_scale_node) as *mut *mut UfbxNode) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*node).inherit_scale_node)) as *mut c_void,
        )
            as *mut UfbxNode;
        *(ptr::addr_of_mut!((*node).scale_helper) as *mut *mut UfbxNode) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*node).scale_helper)) as *mut c_void,
        )
            as *mut UfbxNode;
        *(ptr::addr_of_mut!((*node).bind_pose) as *mut *mut Pose) =
            translate_element(ec, opt_ptr(ptr::addr_of!((*node).bind_pose)) as *mut c_void)
                as *mut Pose;

        if (*node).all_attribs.count > 1 {
            translate_element_list(ec, ptr::addr_of_mut!((*node).all_attribs) as *mut c_void)?;
        } else if (*node).all_attribs.count == 1 {
            // C: `node->all_attribs.data = &node->attrib;`
            (*node).all_attribs.data = ptr::addr_of!((*node).attrib) as *const Ref<Element>;
        }

        *(ptr::addr_of_mut!((*node).geometry_transform_helper) as *mut *mut UfbxNode) =
            translate_element(
                ec,
                opt_ptr(ptr::addr_of!((*node).geometry_transform_helper)) as *mut c_void,
            ) as *mut UfbxNode;

        translate_element_list(ec, ptr::addr_of_mut!((*node).materials) as *mut c_void)?;
        p_node = p_node.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, ec->scene.meshes)`
    let mut p_mesh: *mut *mut Mesh = (*ec.get()).scene.meshes.data as *mut *mut Mesh;
    let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, (*ec.get()).scene.meshes.count);
    while p_mesh != p_mesh_end {
        let mesh: *mut Mesh = *p_mesh;

        translate_element_list(ec, ptr::addr_of_mut!((*mesh).materials) as *mut c_void)?;
        translate_element_list(ec, ptr::addr_of_mut!((*mesh).skin_deformers) as *mut c_void)?;
        translate_element_list(
            ec,
            ptr::addr_of_mut!((*mesh).blend_deformers) as *mut c_void,
        )?;
        translate_element_list(
            ec,
            ptr::addr_of_mut!((*mesh).cache_deformers) as *mut c_void,
        )?;
        translate_element_list(ec, ptr::addr_of_mut!((*mesh).all_deformers) as *mut c_void)?;
        p_mesh = p_mesh.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_stereo_camera, p_stereo, ec->scene.stereo_cameras)`
    let mut p_stereo: *mut *mut StereoCamera =
        (*ec.get()).scene.stereo_cameras.data as *mut *mut StereoCamera;
    let p_stereo_end: *mut *mut StereoCamera =
        add_ptr(p_stereo, (*ec.get()).scene.stereo_cameras.count);
    while p_stereo != p_stereo_end {
        let stereo: *mut StereoCamera = *p_stereo;
        *(ptr::addr_of_mut!((*stereo).left) as *mut *mut Camera) =
            translate_element(ec, opt_ptr(ptr::addr_of!((*stereo).left)) as *mut c_void)
                as *mut Camera;
        *(ptr::addr_of_mut!((*stereo).right) as *mut *mut Camera) =
            translate_element(ec, opt_ptr(ptr::addr_of!((*stereo).right)) as *mut c_void)
                as *mut Camera;
        p_stereo = p_stereo.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_skin_deformer, p_skin, ec->scene.skin_deformers)`
    let mut p_skin: *mut *mut SkinDeformer =
        (*ec.get()).scene.skin_deformers.data as *mut *mut SkinDeformer;
    let p_skin_end: *mut *mut SkinDeformer =
        add_ptr(p_skin, (*ec.get()).scene.skin_deformers.count);
    while p_skin != p_skin_end {
        let skin: *mut SkinDeformer = *p_skin;
        translate_element_list(ec, ptr::addr_of_mut!((*skin).clusters) as *mut c_void)?;
        p_skin = p_skin.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, ec->scene.skin_clusters)`
    let mut p_cluster: *mut *mut SkinCluster =
        (*ec.get()).scene.skin_clusters.data as *mut *mut SkinCluster;
    let p_cluster_end: *mut *mut SkinCluster =
        add_ptr(p_cluster, (*ec.get()).scene.skin_clusters.count);
    while p_cluster != p_cluster_end {
        let cluster: *mut SkinCluster = *p_cluster;
        *(ptr::addr_of_mut!((*cluster).bone_node) as *mut *mut UfbxNode) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*cluster).bone_node)) as *mut c_void,
        )
            as *mut UfbxNode;
        p_cluster = p_cluster.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_blend_deformer, p_blend, ec->scene.blend_deformers)`
    let mut p_blend: *mut *mut BlendDeformer =
        (*ec.get()).scene.blend_deformers.data as *mut *mut BlendDeformer;
    let p_blend_end: *mut *mut BlendDeformer =
        add_ptr(p_blend, (*ec.get()).scene.blend_deformers.count);
    while p_blend != p_blend_end {
        let blend: *mut BlendDeformer = *p_blend;
        translate_element_list(ec, ptr::addr_of_mut!((*blend).channels) as *mut c_void)?;
        p_blend = p_blend.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_blend_channel, p_chan, ec->scene.blend_channels)`
    let mut p_chan: *mut *mut BlendChannel =
        (*ec.get()).scene.blend_channels.data as *mut *mut BlendChannel;
    let p_chan_end: *mut *mut BlendChannel =
        add_ptr(p_chan, (*ec.get()).scene.blend_channels.count);
    while p_chan != p_chan_end {
        let chan: *mut BlendChannel = *p_chan;

        let keys: *mut BlendKeyframe =
            push::<BlendKeyframe>(ec.result_mut_ptr(), (*chan).keyframes.count);
        ufbxi_check_err!(ec.error_mut_ptr(), !keys.is_null(), "keys");
        for i in 0..(*chan).keyframes.count {
            // C: `keys[i] = chan->keyframes.data[i];` (struct assignment)
            ptr::copy_nonoverlapping((*chan).keyframes.data.add(i), keys.add(i), 1);
            *(ptr::addr_of_mut!((*keys.add(i)).shape) as *mut *mut BlendShape) =
                translate_element(ec, ref_ptr(&(*keys.add(i)).shape) as *mut c_void)
                    as *mut BlendShape;
        }
        (*chan).keyframes.data = keys;
        *(ptr::addr_of_mut!((*chan).target_shape) as *mut *mut BlendShape) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*chan).target_shape)) as *mut c_void,
        )
            as *mut BlendShape;
        p_chan = p_chan.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_cache_deformer, p_deformer, ec->scene.cache_deformers)`
    let mut p_deformer: *mut *mut CacheDeformer =
        (*ec.get()).scene.cache_deformers.data as *mut *mut CacheDeformer;
    let p_deformer_end: *mut *mut CacheDeformer =
        add_ptr(p_deformer, (*ec.get()).scene.cache_deformers.count);
    while p_deformer != p_deformer_end {
        let deformer: *mut CacheDeformer = *p_deformer;
        *(ptr::addr_of_mut!((*deformer).file) as *mut *mut CacheFile) =
            translate_element(ec, opt_ptr(ptr::addr_of!((*deformer).file)) as *mut c_void)
                as *mut CacheFile;
        p_deformer = p_deformer.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_material, p_material, ec->scene.materials)`
    let mut p_material: *mut *mut Material = (*ec.get()).scene.materials.data as *mut *mut Material;
    let p_material_end: *mut *mut Material = add_ptr(p_material, (*ec.get()).scene.materials.count);
    while p_material != p_material_end {
        let material: *mut Material = *p_material;

        *(ptr::addr_of_mut!((*material).shader) as *mut *mut Shader) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*material).shader)) as *mut c_void,
        ) as *mut Shader;
        // C: `material->fbx.maps` / `material->pbr.maps` — the flat `maps[]`
        // union view; the generated struct keeps only the named branch, whose
        // base is the aggregate itself (layout pinned in `native::scene_process`).
        translate_maps(
            ec,
            ptr::addr_of_mut!((*material).fbx) as *mut MaterialMap,
            MATERIAL_FBX_MAP_COUNT,
        );
        translate_maps(
            ec,
            ptr::addr_of_mut!((*material).pbr) as *mut MaterialMap,
            MATERIAL_PBR_MAP_COUNT,
        );

        let textures: *mut MaterialTexture =
            push::<MaterialTexture>(ec.result_mut_ptr(), (*material).textures.count);
        ufbxi_check_err!(ec.error_mut_ptr(), !textures.is_null(), "textures");
        for i in 0..(*material).textures.count {
            // C: `textures[i] = material->textures.data[i];` (struct assignment)
            ptr::copy_nonoverlapping((*material).textures.data.add(i), textures.add(i), 1);
            *(ptr::addr_of_mut!((*textures.add(i)).texture) as *mut *mut Texture) =
                translate_element(ec, ref_ptr(&(*textures.add(i)).texture) as *mut c_void)
                    as *mut Texture;
        }
        (*material).textures.data = textures;
        p_material = p_material.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, ec->scene.textures)`
    let mut p_texture: *mut *mut Texture = (*ec.get()).scene.textures.data as *mut *mut Texture;
    let p_texture_end: *mut *mut Texture = add_ptr(p_texture, (*ec.get()).scene.textures.count);
    while p_texture != p_texture_end {
        let texture: *mut Texture = *p_texture;
        *(ptr::addr_of_mut!((*texture).video) as *mut *mut Video) =
            translate_element(ec, opt_ptr(ptr::addr_of!((*texture).video)) as *mut c_void)
                as *mut Video;

        let layers: *mut TextureLayer =
            push::<TextureLayer>(ec.result_mut_ptr(), (*texture).layers.count);
        ufbxi_check_err!(ec.error_mut_ptr(), !layers.is_null(), "layers");
        for i in 0..(*texture).layers.count {
            // C: `layers[i] = texture->layers.data[i];` (struct assignment)
            ptr::copy_nonoverlapping((*texture).layers.data.add(i), layers.add(i), 1);
            *(ptr::addr_of_mut!((*layers.add(i)).texture) as *mut *mut Texture) =
                translate_element(ec, ref_ptr(&(*layers.add(i)).texture) as *mut c_void)
                    as *mut Texture;
        }
        (*texture).layers.data = layers;

        translate_element_list(
            ec,
            ptr::addr_of_mut!((*texture).file_textures) as *mut c_void,
        )?;

        // C: `if (texture->shader) { ... }`
        if !opt_ptr(ptr::addr_of!((*texture).shader)).is_null() {
            let mut shader: *mut ShaderTexture = opt_ptr(ptr::addr_of!((*texture).shader));
            shader = push_copy::<ShaderTexture>(ec.result_mut_ptr(), 1, shader);
            ufbxi_check_err!(ec.error_mut_ptr(), !shader.is_null(), "shader");
            *(ptr::addr_of_mut!((*texture).shader) as *mut *mut ShaderTexture) = shader;

            let inputs: *mut ShaderTextureInput = push_copy::<ShaderTextureInput>(
                ec.result_mut_ptr(),
                (*shader).inputs.count,
                (*shader).inputs.data,
            );
            ufbxi_check_err!(ec.error_mut_ptr(), !inputs.is_null(), "inputs");
            (*shader).inputs.data = inputs;
        }
        p_texture = p_texture.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_shader, p_shader, ec->scene.shaders)`
    let mut p_shader: *mut *mut Shader = (*ec.get()).scene.shaders.data as *mut *mut Shader;
    let p_shader_end: *mut *mut Shader = add_ptr(p_shader, (*ec.get()).scene.shaders.count);
    while p_shader != p_shader_end {
        let shader: *mut Shader = *p_shader;
        translate_element_list(ec, ptr::addr_of_mut!((*shader).bindings) as *mut c_void)?;
        p_shader = p_shader.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_display_layer, p_layer, ec->scene.display_layers)`
    let mut p_layer: *mut *mut DisplayLayer =
        (*ec.get()).scene.display_layers.data as *mut *mut DisplayLayer;
    let p_layer_end: *mut *mut DisplayLayer =
        add_ptr(p_layer, (*ec.get()).scene.display_layers.count);
    while p_layer != p_layer_end {
        let layer: *mut DisplayLayer = *p_layer;

        translate_element_list(ec, ptr::addr_of_mut!((*layer).nodes) as *mut c_void)?;
        p_layer = p_layer.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_selection_set, p_set, ec->scene.selection_sets)`
    let mut p_set: *mut *mut SelectionSet =
        (*ec.get()).scene.selection_sets.data as *mut *mut SelectionSet;
    let p_set_end: *mut *mut SelectionSet = add_ptr(p_set, (*ec.get()).scene.selection_sets.count);
    while p_set != p_set_end {
        let set: *mut SelectionSet = *p_set;

        translate_element_list(ec, ptr::addr_of_mut!((*set).nodes) as *mut c_void)?;
        p_set = p_set.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_selection_node, p_node, ec->scene.selection_nodes)`
    let mut p_sel_node: *mut *mut SelectionNode =
        (*ec.get()).scene.selection_nodes.data as *mut *mut SelectionNode;
    let p_sel_node_end: *mut *mut SelectionNode =
        add_ptr(p_sel_node, (*ec.get()).scene.selection_nodes.count);
    while p_sel_node != p_sel_node_end {
        let node: *mut SelectionNode = *p_sel_node;

        *(ptr::addr_of_mut!((*node).target_node) as *mut *mut UfbxNode) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*node).target_node)) as *mut c_void,
        )
            as *mut UfbxNode;
        *(ptr::addr_of_mut!((*node).target_mesh) as *mut *mut Mesh) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*node).target_mesh)) as *mut c_void,
        ) as *mut Mesh;
        p_sel_node = p_sel_node.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_constraint, p_constraint, ec->scene.constraints)`
    let mut p_constraint: *mut *mut Constraint =
        (*ec.get()).scene.constraints.data as *mut *mut Constraint;
    let p_constraint_end: *mut *mut Constraint =
        add_ptr(p_constraint, (*ec.get()).scene.constraints.count);
    while p_constraint != p_constraint_end {
        let constraint: *mut Constraint = *p_constraint;

        *(ptr::addr_of_mut!((*constraint).node) as *mut *mut UfbxNode) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*constraint).node)) as *mut c_void,
        ) as *mut UfbxNode;
        *(ptr::addr_of_mut!((*constraint).aim_up_node) as *mut *mut UfbxNode) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*constraint).aim_up_node)) as *mut c_void,
        )
            as *mut UfbxNode;
        *(ptr::addr_of_mut!((*constraint).ik_effector) as *mut *mut UfbxNode) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*constraint).ik_effector)) as *mut c_void,
        )
            as *mut UfbxNode;
        *(ptr::addr_of_mut!((*constraint).ik_end_node) as *mut *mut UfbxNode) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*constraint).ik_end_node)) as *mut c_void,
        )
            as *mut UfbxNode;

        let targets: *mut ConstraintTarget =
            push::<ConstraintTarget>(ec.result_mut_ptr(), (*constraint).targets.count);
        ufbxi_check_err!(ec.error_mut_ptr(), !targets.is_null(), "targets");
        for i in 0..(*constraint).targets.count {
            // C: `targets[i] = constraint->targets.data[i];` (struct assignment)
            ptr::copy_nonoverlapping((*constraint).targets.data.add(i), targets.add(i), 1);
            *(ptr::addr_of_mut!((*targets.add(i)).node) as *mut *mut UfbxNode) =
                translate_element(ec, ref_ptr(&(*targets.add(i)).node) as *mut c_void)
                    as *mut UfbxNode;
        }
        (*constraint).targets.data = targets;
        p_constraint = p_constraint.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_audio_layer, p_layer, ec->scene.audio_layers)`
    let mut p_audio_layer: *mut *mut AudioLayer =
        (*ec.get()).scene.audio_layers.data as *mut *mut AudioLayer;
    let p_audio_layer_end: *mut *mut AudioLayer =
        add_ptr(p_audio_layer, (*ec.get()).scene.audio_layers.count);
    while p_audio_layer != p_audio_layer_end {
        let layer: *mut AudioLayer = *p_audio_layer;

        translate_element_list(ec, ptr::addr_of_mut!((*layer).clips) as *mut c_void)?;
        p_audio_layer = p_audio_layer.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_stack, p_stack, ec->scene.anim_stacks)`
    let mut p_stack: *mut *mut AnimStack =
        (*ec.get()).scene.anim_stacks.data as *mut *mut AnimStack;
    let p_stack_end: *mut *mut AnimStack = add_ptr(p_stack, (*ec.get()).scene.anim_stacks.count);
    while p_stack != p_stack_end {
        let stack: *mut AnimStack = *p_stack;

        translate_element_list(ec, ptr::addr_of_mut!((*stack).layers) as *mut c_void)?;
        translate_anim(ec, ptr::addr_of_mut!((*stack).anim) as *mut *mut Anim)?;
        p_stack = p_stack.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_layer, p_layer, ec->scene.anim_layers)`
    let mut p_anim_layer: *mut *mut AnimLayer =
        (*ec.get()).scene.anim_layers.data as *mut *mut AnimLayer;
    let p_anim_layer_end: *mut *mut AnimLayer =
        add_ptr(p_anim_layer, (*ec.get()).scene.anim_layers.count);
    while p_anim_layer != p_anim_layer_end {
        let layer: *mut AnimLayer = *p_anim_layer;

        translate_element_list(ec, ptr::addr_of_mut!((*layer).anim_values) as *mut c_void)?;
        let props: *mut AnimProp =
            push::<AnimProp>(ec.result_mut_ptr(), (*layer).anim_props.count + 1);
        ufbxi_check_err!(ec.error_mut_ptr(), !props.is_null(), "props");
        for i in 0..(*layer).anim_props.count {
            // C: `props[i] = layer->anim_props.data[i];` (struct assignment)
            ptr::copy_nonoverlapping((*layer).anim_props.data.add(i), props.add(i), 1);
            *(ptr::addr_of_mut!((*props.add(i)).element) as *mut *mut Element) =
                translate_element(ec, ref_ptr(&(*props.add(i)).element) as *mut c_void);
            *(ptr::addr_of_mut!((*props.add(i)).anim_value) as *mut *mut AnimValue) =
                translate_element(ec, ref_ptr(&(*props.add(i)).anim_value) as *mut c_void)
                    as *mut AnimValue;
        }
        // Maintain NULL sentinel
        ptr::write_bytes(
            props.add((*layer).anim_props.count) as *mut u8,
            0,
            size_of::<AnimProp>(),
        );
        (*layer).anim_props.data = props;
        p_anim_layer = p_anim_layer.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_pose, p_pose, ec->scene.poses)`
    let mut p_pose: *mut *mut Pose = (*ec.get()).scene.poses.data as *mut *mut Pose;
    let p_pose_end: *mut *mut Pose = add_ptr(p_pose, (*ec.get()).scene.poses.count);
    while p_pose != p_pose_end {
        let pose: *mut Pose = *p_pose;

        let bones: *mut BonePose = push::<BonePose>(ec.result_mut_ptr(), (*pose).bone_poses.count);
        ufbxi_check_err!(ec.error_mut_ptr(), !bones.is_null(), "bones");
        for i in 0..(*pose).bone_poses.count {
            // C: `bones[i] = pose->bone_poses.data[i];` (struct assignment)
            ptr::copy_nonoverlapping((*pose).bone_poses.data.add(i), bones.add(i), 1);
            *(ptr::addr_of_mut!((*bones.add(i)).bone_node) as *mut *mut UfbxNode) =
                translate_element(ec, ref_ptr(&(*bones.add(i)).bone_node) as *mut c_void)
                    as *mut UfbxNode;
        }
        (*pose).bone_poses.data = bones;
        p_pose = p_pose.add(1);
    }

    translate_anim(ec, ec.anim_mut_ptr())?;

    // C: `ufbxi_for_ptr_list(ufbx_anim_value, p_value, ec->scene.anim_values)`
    let mut p_value: *mut *mut AnimValue =
        (*ec.get()).scene.anim_values.data as *mut *mut AnimValue;
    let p_value_end: *mut *mut AnimValue = add_ptr(p_value, (*ec.get()).scene.anim_values.count);
    while p_value != p_value_end {
        let value: *mut AnimValue = *p_value;
        *(ptr::addr_of_mut!((*value).curves[0]) as *mut *mut AnimCurve) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*value).curves[0])) as *mut c_void,
        )
            as *mut AnimCurve;
        *(ptr::addr_of_mut!((*value).curves[1]) as *mut *mut AnimCurve) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*value).curves[1])) as *mut c_void,
        )
            as *mut AnimCurve;
        *(ptr::addr_of_mut!((*value).curves[2]) as *mut *mut AnimCurve) = translate_element(
            ec,
            opt_ptr(ptr::addr_of!((*value).curves[2])) as *mut c_void,
        )
            as *mut AnimCurve;
        p_value = p_value.add(1);
    }

    // C: `ufbx_anim anim = *ec->anim;` — local working copy (memcpy).
    let mut anim: Anim = ptr::read(ec.anim());
    let mut over: *const PropOverride = anim.prop_overrides.data;
    let over_end: *const PropOverride =
        add_ptr(over as *mut PropOverride, anim.prop_overrides.count);

    // Evaluate the properties
    // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, ec->scene.elements)`
    let mut p_elem: *mut *mut Element = (*ec.get()).scene.elements.data as *mut *mut Element;
    let p_elem_end: *mut *mut Element = add_ptr(p_elem, (*ec.get()).scene.elements.count);
    while p_elem != p_elem_end {
        let elem: *mut Element = *p_elem;
        let mut num_animated: usize = (*elem).props.num_animated;
        let mut num_override: usize = 0;

        // Setup the overrides for this element if found
        while over != over_end && (*over).element_id == (*elem).element_id {
            num_override += 1;
            over = over.add(1);
        }

        num_animated += num_override;
        if num_animated == 0 {
            p_elem = p_elem.add(1);
            continue;
        }

        // C: `anim.prop_overrides.data = ufbxi_sub_ptr(over, num_override);`
        anim.prop_overrides.data = over.sub(num_override);
        anim.prop_overrides.count = num_override;

        let props: *mut Prop = push::<Prop>(ec.result_mut_ptr(), num_animated);
        ufbxi_check_err!(ec.error_mut_ptr(), !props.is_null(), "props");

        // C: `elem->props = ufbx_evaluate_props_flags(...)` — struct assignment.
        let new_props: crate::generated::Props = evaluate_props_flags(
            &anim,
            elem,
            ec.time(),
            props,
            num_animated,
            (*ec.get()).opts.evaluate_flags,
        );
        ptr::write(ptr::addr_of_mut!((*elem).props), new_props);
        // C: `elem->props.defaults = &ec->src_scene.elements.data[elem->element_id]->props;`
        *(ptr::addr_of_mut!((*elem).props.defaults) as *mut *const crate::generated::Props) = ptr::addr_of!(
            (*(*((*ec.get()).src_scene.elements.data as *mut *mut Element)
                .add((*elem).element_id as usize)))
            .props
        );
        p_elem = p_elem.add(1);
    }

    // Update all derived values
    update_scene(
        ec.scene_mut_ptr(),
        false,
        anim.transform_overrides.data,
        anim.transform_overrides.count,
    );

    // Evaluate skinning if requested
    if (*ec.get()).opts.evaluate_skinning {
        // C: `ufbx_geometry_cache_data_opts cache_opts = { 0 };`
        let mut cache_opts: RawGeometryCacheDataOpts = MaybeUninit::zeroed().assume_init();
        // C: `cache_opts.open_file_cb = ec->opts.open_file_cb;` (struct assignment)
        ptr::copy_nonoverlapping(
            ptr::addr_of!((*ec.get()).opts.open_file_cb),
            ptr::addr_of_mut!(cache_opts.open_file_cb),
            1,
        );
        evaluate_skinning(
            ec.scene_mut_ptr(),
            ec.error_mut_ptr(),
            ec.result_mut_ptr(),
            ec.tmp_mut_ptr(),
            ec.time(),
            (*ec.get()).opts.load_external_files && (*ec.get()).opts.evaluate_caches,
            &mut cache_opts,
        )?;
    }

    // Retain the scene, this must be the final allocation as we copy
    // `ator_result` to `ufbx_scene_imp`
    let imp: *mut SceneImp = push_zero::<SceneImp>(ec.result_mut_ptr(), 1);
    ufbxi_check_err!(ec.error_mut_ptr(), !imp.is_null(), "imp");

    // Expose the wide allocation so `get_imp` can recover this header from a
    // (possibly narrowed) public `&Scene` pointer via exposed provenance.
    (imp as *mut u8).expose_provenance();

    ufbx_assert!((*ec.src_imp()).magic == SCENE_IMP_MAGIC);
    init_ref(
        ptr::addr_of_mut!((*imp).refcount),
        SCENE_IMP_MAGIC,
        ptr::addr_of_mut!((*ec.src_imp()).refcount),
    );

    (*imp).magic = SCENE_IMP_MAGIC;
    // C: `imp->scene = ec->scene;` (struct assignment)
    ptr::copy_nonoverlapping(
        ptr::addr_of!((*ec.get()).scene),
        ptr::addr_of_mut!((*imp).scene),
        1,
    );
    (*imp).refcount.ator = (*ec.get()).ator_result;
    (*imp).refcount.ator.error = ptr::null_mut();

    // Copy retained buffers and translate the allocator struct to the one
    // contained within `ufbxi_scene_imp`
    (*imp).refcount.buf = (*ec.get()).result;
    (*imp).refcount.buf.ator = ptr::addr_of_mut!((*imp).refcount.ator);

    (*imp).scene.metadata.result_memory_used = (*imp).refcount.ator.current_size;
    (*imp).scene.metadata.temp_memory_used = (*ec.get()).ator_tmp.current_size;
    (*imp).scene.metadata.result_allocs = (*imp).refcount.ator.num_allocs;
    (*imp).scene.metadata.temp_allocs = (*ec.get()).ator_tmp.num_allocs;

    // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, imp->scene.elements)`
    let mut p_elem: *mut *mut Element = (*imp).scene.elements.data as *mut *mut Element;
    let p_elem_end: *mut *mut Element = add_ptr(p_elem, (*imp).scene.elements.count);
    while p_elem != p_elem_end {
        // C: `(*p_elem)->scene = &imp->scene;`
        *(ptr::addr_of_mut!((*(*p_elem)).scene) as *mut *mut Scene) =
            ptr::addr_of_mut!((*imp).scene);
        p_elem = p_elem.add(1);
    }

    ec.set_scene_imp(imp);
    (*ec.get()).result.ator = ec.ator_result_mut_ptr();

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
        ptr::copy_nonoverlapping(user_opts, ec.opts_mut_ptr(), 1);
    } else {
        // C: `memset(&ec->opts, 0, sizeof(ec->opts));`
        ptr::write_bytes(
            ec.opts_mut_ptr() as *mut u8,
            0,
            size_of::<RawEvaluateOpts>(),
        );
    }

    ec.set_src_imp(get_imp::<SceneImp>(scene as *mut c_void));
    // C: `ec->src_scene = *scene;` (struct assignment)
    ptr::copy_nonoverlapping(scene as *const Scene, ec.src_scene_mut_ptr(), 1);
    ec.set_anim(if !anim.is_null() {
        anim as *mut Anim
    } else {
        ref_ptr(&(*scene).anim)
    });
    ec.set_time(time);

    init_ator(
        ec.error_mut_ptr(),
        ec.ator_tmp_mut_ptr(),
        ptr::addr_of!((*ec.get()).opts.temp_allocator),
        b"temp\0".as_ptr(),
    );
    init_ator(
        ec.error_mut_ptr(),
        ec.ator_result_mut_ptr(),
        ptr::addr_of!((*ec.get()).opts.result_allocator),
        b"result\0".as_ptr(),
    );

    (*ec.get()).result.ator = ec.ator_result_mut_ptr();
    (*ec.get()).tmp.ator = ec.ator_tmp_mut_ptr();

    (*ec.get()).result.unordered = true;
    (*ec.get()).tmp.unordered = true;

    if evaluate_imp(ec).is_ok() {
        buf_free(ec.tmp_mut_ptr());
        free_ator(ec.ator_tmp_mut_ptr());
        if !p_error.is_null() {
            clear_error(p_error);
        }
        ptr::addr_of_mut!((*ec.scene_imp()).scene)
    } else {
        fix_error_type(
            ec.error_mut_ptr(),
            b"Failed to evaluate\0".as_ptr(),
            p_error,
        );
        buf_free(ec.tmp_mut_ptr());
        buf_free(ec.result_mut_ptr());
        free_ator(ec.ator_tmp_mut_ptr());
        free_ator(ec.ator_result_mut_ptr());
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

impl CreateAnimContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerCreateAnimContext {
        self.0.get().cast()
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
#[must_use]
pub(crate) unsafe fn check_string(
    error: *mut Error,
    dst: *mut String,
    src: *const String,
) -> Result<(), Fail> {
    let length: usize = if (*src).length != usize::MAX {
        (*src).length
    } else {
        strlen((*src).data)
    };
    let data: *const u8 = if length != 0 {
        (*src).data
    } else {
        EMPTY_CHAR.as_ptr()
    };
    if length > 0 {
        let valid_length: usize = utf8_valid_length(data, length);
        ufbxi_check_err_msg!(error, valid_length == length, "Invalid UTF-8");
    }

    (*dst).data = data;
    (*dst).length = length;
    Ok(())
}

// ufbx.c:26512-26526 `ufbxi_push_anim_string`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_anim_string(
    ac: &CreateAnimContext,
    str_: *mut String,
) -> Result<(), Fail> {
    let length: usize = (*str_).length;
    if length > 0 {
        let copy: *mut u8 = push::<u8>(ac.result_mut_ptr(), length + 1);
        ufbxi_check_err!(ac.error_mut_ptr(), !copy.is_null(), "copy");
        // C: `memcpy(copy, str->data, length);`
        ptr::copy_nonoverlapping((*str_).data, copy, length);
        // C: `copy[str->length] = '\0';`
        *copy.add((*str_).length) = b'\0';
        (*str_).data = copy;
    } else {
        ufbx_assert!((*str_).data == EMPTY_CHAR.as_ptr());
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
    if (*a)._internal_key != (*b)._internal_key {
        return (*a)._internal_key < (*b)._internal_key;
    }
    str_less((*a).prop_name, (*b).prop_name)
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
    if (*a).element_id != (*b).element_id {
        return (*a).element_id < (*b).element_id;
    }
    if (*a)._internal_key != (*b)._internal_key {
        return (*a)._internal_key < (*b)._internal_key;
    }
    strcmp((*a).prop_name.data, (*b).prop_name.data) < 0
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
    (*a).node_id < (*b).node_id
}

// ufbx.c:26552-26668 `ufbxi_create_anim_imp`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn create_anim_imp(ac: &CreateAnimContext) -> Result<(), Fail> {
    let scene: *const Scene = ac.scene();
    let anim: *mut Anim = ac.anim_mut_ptr();

    init_ator(
        ac.error_mut_ptr(),
        ac.ator_result_mut_ptr(),
        ptr::addr_of!((*ac.get()).opts.result_allocator),
        b"result\0".as_ptr(),
    );
    (*ac.get()).result.unordered = true;
    (*ac.get()).result.ator = ac.ator_result_mut_ptr();

    (*anim).ignore_connections = (*ac.get()).opts.ignore_connections;
    (*anim).custom = true;

    let num_layers: usize = (*ac.get()).opts.layer_ids.count;
    (*anim).layers.count = num_layers;
    (*anim).layers.data =
        push_zero::<*mut AnimLayer>(ac.result_mut_ptr(), num_layers) as *const Ref<AnimLayer>;
    ufbxi_check_err!(
        ac.error_mut_ptr(),
        !(*anim).layers.data.is_null(),
        "anim->layers.data"
    );

    if (*ac.get()).opts.override_layer_weights.count > 0 {
        ufbxi_check_err_msg!(
            ac.error_mut_ptr(),
            (*ac.get()).opts.override_layer_weights.count == num_layers,
            "override_layer_weights[] count must match layer_ids[] count",
            "ac->opts.override_layer_weights.count == num_layers"
        );
        (*anim).override_layer_weights.data = push_copy::<Real>(
            ac.result_mut_ptr(),
            num_layers,
            (*ac.get()).opts.override_layer_weights.data,
        );
        ufbxi_check_err!(
            ac.error_mut_ptr(),
            !(*anim).override_layer_weights.data.is_null(),
            "anim->override_layer_weights.data"
        );
        (*anim).override_layer_weights.count = num_layers;
    }

    for i in 0..num_layers {
        let index: u32 = *(*ac.get()).opts.layer_ids.data.add(i);
        ufbxi_check_err_msg!(
            ac.error_mut_ptr(),
            (index as usize) < (*scene).anim_layers.count,
            "layer_ids out of bounds",
            "index < scene->anim_layers.count"
        );
        // C: `anim->layers.data[i] = ac->scene->anim_layers.data[index];`
        *((*anim).layers.data as *mut *mut AnimLayer).add(i) =
            *((*scene).anim_layers.data as *mut *mut AnimLayer).add(index as usize);
    }

    // C: `ufbx_const_prop_override_desc_list prop_overrides = ac->opts.prop_overrides;`
    let prop_overrides: crate::prelude::RawList<RawPropOverrideDesc> =
        ptr::read(ptr::addr_of!((*ac.get()).opts.prop_overrides));
    if prop_overrides.count > 0 {
        (*anim).prop_overrides.count = prop_overrides.count;
        (*anim).prop_overrides.data =
            push_zero::<PropOverride>(ac.result_mut_ptr(), prop_overrides.count);
        ufbxi_check_err!(
            ac.error_mut_ptr(),
            !(*anim).prop_overrides.data.is_null(),
            "anim->prop_overrides.data"
        );

        for i in 0..prop_overrides.count {
            let src: *const RawPropOverrideDesc = prop_overrides.data.add(i);
            let dst: *mut PropOverride = ((*anim).prop_overrides.data as *mut PropOverride).add(i);

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
                ptr::addr_of_mut!((*dst).prop_name),
                ptr::addr_of!((*src).prop_name) as *const String,
            )?;
            check_string(
                ac.error_mut_ptr(),
                ptr::addr_of_mut!((*dst).value_str),
                ptr::addr_of!((*src).value_str) as *const String,
            )?;

            (*dst)._internal_key = get_name_key((*dst).prop_name.data, (*dst).prop_name.length);
        }

        // Sort `anim->prop_overrides` first by `prop_name` only so we can deduplicate and
        // convert them to global strings in `ufbxi_strings[]` if possible.
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
                push_anim_string(ac, ptr::addr_of_mut!((*over).value_str))?;
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
                push_anim_string(ac, ptr::addr_of_mut!((*over).prop_name))?;
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

        for i in 1..prop_overrides.count {
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
                    ac.error_mut_ptr(),
                    "Duplicate override",
                    "Duplicate override"
                );
            }
        }
    }

    if (*ac.get()).opts.transform_overrides.count > 0 {
        (*anim).transform_overrides.count = (*ac.get()).opts.transform_overrides.count;
        (*anim).transform_overrides.data = push_copy::<TransformOverride>(
            ac.result_mut_ptr(),
            (*anim).transform_overrides.count,
            (*ac.get()).opts.transform_overrides.data,
        );
        ufbxi_check_err!(
            ac.error_mut_ptr(),
            !(*anim).transform_overrides.data.is_null(),
            "anim->transform_overrides.data"
        );
        unstable_sort(
            (*anim).transform_overrides.data as *mut TransformOverride as *mut c_void,
            (*anim).transform_overrides.count,
            size_of::<TransformOverride>(),
            transform_override_less,
            ptr::null_mut(),
        );
    }

    ac.set_imp(push::<AnimImp>(ac.result_mut_ptr(), 1));
    ufbxi_check_err!(ac.error_mut_ptr(), !ac.imp().is_null(), "ac->imp");

    // Expose the wide allocation so `get_imp` can recover this header from a
    // (possibly narrowed) public `&Anim` pointer via exposed provenance.
    (ac.imp() as *mut u8).expose_provenance();

    init_ref(
        ptr::addr_of_mut!((*ac.imp()).refcount),
        ANIM_IMP_MAGIC,
        ptr::addr_of_mut!((*get_imp::<SceneImp>(scene as *mut Scene as *mut c_void)).refcount),
    );

    (*ac.imp()).magic = ANIM_IMP_MAGIC;
    // C: `ac->imp->anim = ac->anim;` (struct assignment)
    ptr::copy_nonoverlapping(
        ptr::addr_of!((*ac.get()).anim),
        ptr::addr_of_mut!((*ac.imp()).anim),
        1,
    );
    (*ac.imp()).refcount.ator = (*ac.get()).ator_result;
    (*ac.imp()).refcount.buf = (*ac.get()).result;

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

// Safe `&BakeContext` handle over the fields-struct `InnerBakeContext`, mirroring
// the `Context`/`InnerContext` seam in `parse.rs`. `MaybeUninit` because it embeds
// the public `BakedAnim` (enum-bearing) in `bake`, so a plain `&InnerBakeContext`
// could not be formed soundly; `UnsafeCell` gives the interior mutability every
// `&BakeContext` site needs. Field is `pub(crate)` — the sole construction site
// lives in `native::api`.
#[repr(transparent)]
pub(crate) struct BakeContext(
    pub(crate) core::cell::UnsafeCell<core::mem::MaybeUninit<InnerBakeContext>>,
);

impl BakeContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerBakeContext {
        self.0.get().cast()
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

    // `ator_tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_tmp_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_tmp }
    }

    // `ator_result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_result_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_result }
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
    if (*a).sort_id != (*b).sort_id {
        return (*a).sort_id < (*b).sort_id;
    }
    if (*a).element_id != (*b).element_id {
        return (*a).element_id < (*b).element_id;
    }
    if (*a).prop_name != (*b).prop_name {
        return strcmp((*a).prop_name, (*b).prop_name) < 0;
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
pub(crate) unsafe fn bake_push_time(bc: &BakeContext, time: f64, flags: u32) -> bool {
    let p_key: *mut BakeTime = push_fast::<BakeTime>(bc.tmp_times_mut_ptr(), 1);
    if p_key.is_null() {
        return false;
    }
    (*p_key).time = time;
    (*p_key).flags = flags;
    true
}

// ufbx.c:26765-26813 `ufbxi_bake_times`
#[cfg(feature = "baking")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn bake_times(
    bc: &BakeContext,
    anim_value: *const AnimValue,
    resample_linear: bool,
    key_flag: u32,
) -> Result<(), Fail> {
    let sample_rate: f64 = (*bc.get()).opts.resample_rate;
    let min_duration: f64 = if (*bc.get()).opts.minimum_sample_rate > 0.0 {
        1.0 / (*bc.get()).opts.minimum_sample_rate
    } else {
        0.0
    };

    for curve_ix in 0..3usize {
        let curve: *mut AnimCurve = opt_ptr(
            (ptr::addr_of!((*anim_value).curves) as *const Option<Ref<AnimCurve>>).add(curve_ix),
        );
        if curve.is_null() {
            continue;
        }

        let keys: *const Keyframe = (*curve).keyframes.data;
        let num_keys: usize = (*curve).keyframes.count;
        for key_ix in 0..num_keys {
            let a: Keyframe = *keys.add(key_ix);
            let a_time: f64 = a.time;
            ufbxi_check_err!(
                bc.error_mut_ptr(),
                bake_push_time(bc, a_time, key_flag),
                "ufbxi_bake_push_time(bc, a_time, key_flag)"
            );
            if key_ix + 1 >= num_keys {
                break;
            }
            let b: Keyframe = *keys.add(key_ix + 1);
            let b_time: f64 = b.time;

            // Skip fully flat sections
            if a.value == b.value && a.right.dy == 0.0f32 && b.left.dy == 0.0f32 {
                continue;
            }

            if a.interpolation as u32 == Interpolation::ConstantPrev as u32 {
                ufbxi_check_err!(
                    bc.error_mut_ptr(),
                    bake_push_time(bc, b_time, BakedKeyFlags::STEP_LEFT.raw()),
                    "ufbxi_bake_push_time(bc, b_time, UFBX_BAKED_KEY_STEP_LEFT)"
                );
            } else if a.interpolation as u32 == Interpolation::ConstantNext as u32 {
                ufbxi_check_err!(
                    bc.error_mut_ptr(),
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
                    >= (*bc.get()).opts.max_keyframe_segments as f64
                {
                    factor *= 2.0;
                }

                let padding: f64 = 0.5 / sample_rate;
                let start: f64 = math::ceil((a_time + padding) * sample_rate / factor) * factor;
                let stop: f64 = b_time - padding;
                for i in 0..(*bc.get()).opts.max_keyframe_segments {
                    let time: f64 = (start + i as f64 * factor) / sample_rate;
                    if time >= stop {
                        break;
                    }
                    ufbxi_check_err!(
                        bc.error_mut_ptr(),
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
        if *items.add(i) == item {
            return true;
        }
    }
    false
}

// ufbx.c:26840-26845 `ufbxi_sort_bake_times`
#[cfg(feature = "baking")]
#[inline(never)]
#[must_use]
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
        bc.error_mut_ptr(),
        grow_array::<u8>(
            bc.ator_tmp_mut_ptr(),
            bc.tmp_arr_mut_ptr(),
            bc.tmp_arr_size_mut_ptr(),
            count.wrapping_mul(size_of::<BakeTime>()),
        ),
        "ufbxi_grow_array_size((&bc->ator_tmp), sizeof(**(&bc->tmp_arr)), (&bc->tmp_arr), (&bc->tmp_arr_size), (count * sizeof(ufbxi_bake_time)))"
    );
    macro_stable_sort::<BakeTime>(32, times, bc.tmp_arr() as *mut BakeTime, count, |a, b| {
        cmp_bake_time(*a, *b) < 0
    });
    Ok(())
}

// ufbx.c:26847-26968 `ufbxi_finalize_bake_times`
#[cfg(feature = "baking")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn finalize_bake_times(
    bc: &BakeContext,
    p_dst: *mut BakeTimeList,
) -> Result<(), Fail> {
    if (*bc.get()).layer_weight_times.count > 0 {
        ufbxi_check_err!(
            bc.error_mut_ptr(),
            !push_copy::<BakeTime>(
                bc.tmp_times_mut_ptr(),
                (*bc.get()).layer_weight_times.count,
                (*bc.get()).layer_weight_times.data,
            )
            .is_null(),
            "((ufbxi_bake_time*)ufbxi_push_size_copy((&bc->tmp_times), sizeof(ufbxi_bake_time), (bc->layer_weight_times.count), (bc->layer_weight_times.data)))"
        );
    }

    if (*bc.get()).tmp_times.num_items == 0 {
        ufbxi_check_err!(
            bc.error_mut_ptr(),
            bake_push_time(bc, bc.time_begin(), 0),
            "ufbxi_bake_push_time(bc, bc->time_begin, 0)"
        );
        ufbxi_check_err!(
            bc.error_mut_ptr(),
            bake_push_time(bc, bc.time_end(), 0),
            "ufbxi_bake_push_time(bc, bc->time_end, 0)"
        );
    }

    let mut num_times: usize = (*bc.get()).tmp_times.num_items;
    let times: *mut BakeTime =
        push_pop::<BakeTime>(bc.tmp_prop_mut_ptr(), bc.tmp_times_mut_ptr(), num_times);
    ufbxi_check_err!(bc.error_mut_ptr(), !times.is_null(), "times");

    sort_bake_times(bc, times, num_times)?;

    // Deduplicate times
    if num_times > 0 {
        let mut dst: usize = 0;
        let mut prev: BakeTime = *times.add(0);
        let mut src: usize = 1;
        while src < num_times {
            let mut next: BakeTime = *times.add(src);
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

            *times.add(dst) = prev;
            dst += 1;
            prev = next;
            src += 1;
        }
        *times.add(dst) = prev;
        dst += 1;
        num_times = dst;
    }

    // Cull too close resampled keys, these may arise during merging multiple times
    if num_times > 0 {
        let min_dist: f64 = 0.25 / (*bc.get()).opts.resample_rate;
        let keep_flags: u32 = BakedKeyFlags::STEP_LEFT.raw()
            | BakedKeyFlags::STEP_RIGHT.raw()
            | BakedKeyFlags::STEP_KEY.raw()
            | BakedKeyFlags::KEYFRAME.raw();

        let mut dst: usize = 0;
        for src in 0..num_times {
            let cur: BakeTime = *times.add(src);
            let mut delta: f64 = math::INFINITY;

            let mut keep: bool = true;
            if (cur.flags & keep_flags) == 0 {
                if dst > 0 {
                    delta = cur.time - (*times.add(dst - 1)).time;
                }
                if src + 1 < num_times {
                    delta = math::fmin(delta, (*times.add(src + 1)).time - cur.time);
                }
                if delta < min_dist {
                    keep = false;
                }
            }
            if keep {
                *times.add(dst) = cur;
                dst += 1;
            }
        }
        num_times = dst;
    }

    // Enforce maximum sample rate
    if (*bc.get()).opts.maximum_sample_rate > 0.0 {
        let epsilon: f64 = 0.0078125 / (*bc.get()).opts.maximum_sample_rate;
        let sample_rate: f64 = (*bc.get()).opts.maximum_sample_rate;
        let max_interval: f64 = 1.0 / (*bc.get()).opts.maximum_sample_rate;
        let min_interval: f64 = 1.0 / (*bc.get()).opts.maximum_sample_rate - epsilon;
        let mut dst: usize = 0;
        let mut src: usize = 0;

        // Pre-expand constant keyframes
        for i in 0..num_times {
            if ((*times.add(i)).flags
                & (BakedKeyFlags::STEP_LEFT.raw() | BakedKeyFlags::STEP_RIGHT.raw()))
                != 0
            {
                let sign: f64 = if ((*times.add(i)).flags & BakedKeyFlags::STEP_LEFT.raw()) != 0 {
                    -1.0
                } else {
                    1.0
                };
                let mut time: f64 = (*times.add(i)).time + sign * max_interval;
                if i > 0 {
                    time = math::fmax(time, (*times.add(i - 1)).time);
                }
                if i + 1 < num_times {
                    time = math::fmin(time, (*times.add(i + 1)).time);
                }
                (*times.add(i)).time = time;
                (*times.add(i)).flags = BakedKeyFlags::REDUCED.raw();
            }
        }

        // C: `ufbxi_bake_time prev_time = { -UFBX_INFINITY };`
        let mut prev_time: BakeTime = BakeTime {
            time: -math::INFINITY,
            flags: 0,
        };
        while src < num_times {
            let src_time: BakeTime = *times.add(src);
            src += 1;

            let start_src: usize = src;
            // C: `ufbxi_bake_time next_time;` — both members assigned below.
            let mut next_time: BakeTime = BakeTime {
                time: 0.0,
                flags: 0,
            };
            next_time.time = math::ceil(src_time.time * sample_rate - epsilon) / sample_rate;
            next_time.flags = BakedKeyFlags::REDUCED.raw();
            while src < num_times && (*times.add(src)).time <= next_time.time + epsilon {
                src += 1;
            }

            if src != start_src || src_time.time - prev_time.time <= min_interval {
                prev_time = next_time;
            } else {
                prev_time = src_time;
            }

            if dst == 0 || prev_time.time > (*times.add(dst - 1)).time {
                *times.add(dst) = prev_time;
                dst += 1;
            }
        }

        num_times = dst;
    }

    if num_times > 0 {
        if (*times.add(0)).time < bc.time_min() {
            bc.set_time_min((*times.add(0)).time);
        }
        if (*times.add(num_times - 1)).time > bc.time_max() {
            bc.set_time_max((*times.add(num_times - 1)).time);
        }
    }

    (*p_dst).data = times;
    (*p_dst).count = num_times;

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

    let mut time: f64 = *p_time;
    // C: `switch (bc->opts.step_handling)` — an if-ladder over the discriminant
    // value, with the trailing `else` standing in for C's `default:` arm, which
    // is reachable because `bc->opts` is a verbatim copy of unvalidated user
    // options. The `as u32` read does not make an out-of-range user value safe:
    // `step_handling` is typed as the generated `BakeStepHandling` enum, so an
    // invalid discriminant is already materialized by the options copy in
    // `api::bake_anim` — the generated-type/read-idiom question is tree-wide
    // (same shape as `subdivision::subdivide_mesh_imp`'s `opts.boundary`) and
    // belongs to the generator, per PORTING.md ground rule 0.
    let step_handling: u32 = (*bc.get()).opts.step_handling as u32;
    if step_handling == BakeStepHandling::Default as u32 {
        // C: `break;`
    } else if step_handling == BakeStepHandling::CustomDuration as u32 {
        step = (*bc.get()).opts.step_custom_duration;
        epsilon = 1.0 + (*bc.get()).opts.step_custom_epsilon;
    } else if step_handling == BakeStepHandling::IdenticalTime as u32 {
        return true;
    } else if step_handling == BakeStepHandling::AdjacentDouble as u32 {
        if left {
            time = math::nextafter(time, -math::INFINITY);
            *p_time = time;
            return time > prev_time;
        } else {
            time = math::nextafter(time, math::INFINITY);
            *p_time = time;
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
        *p_time = time;
        time > min_time
    } else {
        let max_time: f64 = math::fmin(next_time - step, sub_epsilon(next_time, epsilon));
        time = math::fmax(time + step, add_epsilon(time, epsilon));
        *p_time = time;
        time < max_time
    }
}

// ufbx.c:27017-27097 `ufbxi_bake_postprocess_vec3`
#[cfg(feature = "baking")]
#[inline(never)]
#[must_use]
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
        let scale: f64 = (*bc.scene()).metadata.ktime_second as f64;
        let offset: f64 = bc.ktime_offset();
        for i in 0..src.count {
            (*data.add(i)).time = math::rint((*data.add(i)).time * scale + offset) / scale;
        }
    }

    // Postprocess stepped tangents
    {
        let mut dst: usize = 0;
        let mut prev_time: f64 = (*data.add(0)).time;
        for i in 0..src.count {
            let mut cur: BakedVec3 = ptr::read(data.add(i));
            let next_time: f64 = if i + 1 < src.count {
                (*data.add(i + 1)).time
            } else {
                math::INFINITY
            };
            let mut keep: bool = true;
            if (cur.flags.raw()
                & (BakedKeyFlags::STEP_LEFT.raw() | BakedKeyFlags::STEP_RIGHT.raw()))
                != 0
            {
                keep = postprocess_step(
                    bc,
                    prev_time,
                    next_time,
                    ptr::addr_of_mut!(cur.time),
                    cur.flags,
                );
            }
            if keep {
                // C: `src.data[dst] = cur; dst++; prev_time = cur.time;`
                let cur_time: f64 = cur.time;
                ptr::write(data.add(dst), cur);
                dst += 1;
                prev_time = cur_time;
            }
        }
        src.count = dst;
    }

    if (*bc.get()).opts.key_reduction_enabled {
        let threshold: f64 =
            (*bc.get()).opts.key_reduction_threshold * (*bc.get()).opts.key_reduction_threshold;
        for _pass in 0..(*bc.get()).opts.key_reduction_passes {
            let mut dst: usize = 1;
            let mut i: usize = 1;
            while i < src.count {
                let prev: BakedVec3 = ptr::read(data.add(i - 1));
                let cur: BakedVec3 = ptr::read(data.add(i));
                if i + 1 < src.count {
                    let next: BakedVec3 = ptr::read(data.add(i + 1));
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
                        ptr::write(data.add(dst), ptr::read(data.add(i + 1)));
                        i += 1;
                        dst += 1;
                        // C: `continue` — the `for` increment still runs.
                        i += 1;
                        continue;
                    }
                }

                ptr::write(data.add(dst), ptr::read(data.add(i)));
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
    let ref_: Vec3 = (*data.add(0)).value;
    for i in 1..src.count {
        let v: Vec3 = (*data.add(i)).value;
        if v.x != ref_.x || v.y != ref_.y || v.z != ref_.z {
            constant = false;
            break;
        }
    }
    *p_constant = constant;

    (*p_dst).count = src.count;
    (*p_dst).data = push_copy::<BakedVec3>(bc.result_mut_ptr(), src.count, data);
    ufbxi_check_err!(bc.error_mut_ptr(), !(*p_dst).data.is_null(), "p_dst->data");

    Ok(())
}

// ufbx.c:27099-27199 `ufbxi_bake_postprocess_quat`
#[cfg(feature = "baking")]
#[inline(never)]
#[must_use]
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
        let scale: f64 = (*bc.scene()).metadata.ktime_second as f64;
        let offset: f64 = bc.ktime_offset();
        for i in 0..src.count {
            (*data.add(i)).time = math::rint((*data.add(i)).time * scale + offset) / scale;
        }
    }

    // Postprocess stepped tangents
    {
        let mut dst: usize = 0;
        let mut prev_time: f64 = (*data.add(0)).time;
        for i in 0..src.count {
            let mut cur: BakedQuat = ptr::read(data.add(i));
            let next_time: f64 = if i + 1 < src.count {
                (*data.add(i + 1)).time
            } else {
                math::INFINITY
            };
            let mut keep: bool = true;
            if (cur.flags.raw()
                & (BakedKeyFlags::STEP_LEFT.raw() | BakedKeyFlags::STEP_RIGHT.raw()))
                != 0
            {
                keep = postprocess_step(
                    bc,
                    prev_time,
                    next_time,
                    ptr::addr_of_mut!(cur.time),
                    cur.flags,
                );
            }
            if keep {
                prev_time = cur.time;
                ptr::write(data.add(dst), cur);
                dst += 1;
            }
        }
        src.count = dst;
    }

    // Fix quaternion antipodality
    for i in 1..src.count {
        (*data.add(i)).value = quat_fix_antipodal((*data.add(i)).value, (*data.add(i - 1)).value);
    }

    if (*bc.get()).opts.key_reduction_enabled {
        let threshold: f64 =
            (*bc.get()).opts.key_reduction_threshold * (*bc.get()).opts.key_reduction_threshold;
        for _pass in 0..(*bc.get()).opts.key_reduction_passes {
            let mut dst: usize = 1;
            let mut i: usize = 1;
            while i < src.count {
                let prev: BakedQuat = ptr::read(data.add(i - 1));
                let cur: BakedQuat = ptr::read(data.add(i));
                if i + 1 < src.count {
                    let next: BakedQuat = ptr::read(data.add(i + 1));
                    let delta: f64 = (cur.time - prev.time) / (next.time - prev.time);
                    let mut error: f64 = 0.0;

                    if (*bc.get()).opts.key_reduction_rotation {
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
                        ptr::write(data.add(dst), ptr::read(data.add(i + 1)));
                        i += 1;
                        dst += 1;
                        // C: `continue` — the `for` increment still runs.
                        i += 1;
                        continue;
                    }
                }

                ptr::write(data.add(dst), ptr::read(data.add(i)));
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
    let ref_: Quat = (*data.add(0)).value;
    for i in 1..src.count {
        let v: Quat = (*data.add(i)).value;
        if v.x != ref_.x || v.y != ref_.y || v.z != ref_.z || v.w != ref_.w {
            constant = false;
            break;
        }
    }
    *p_constant = constant;

    (*p_dst).count = src.count;
    (*p_dst).data = push_copy::<BakedQuat>(bc.result_mut_ptr(), src.count, data);
    ufbxi_check_err!(bc.error_mut_ptr(), !(*p_dst).data.is_null(), "p_dst->data");

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
#[must_use]
pub(crate) unsafe fn push_resampled_times(
    bc: &BakeContext,
    p_keys: *const List<BakedVec3>,
) -> Result<(), Fail> {
    // C: `ufbx_baked_vec3_list keys = *p_keys;`
    let keys: List<BakedVec3> = ptr::read(p_keys);

    let times: *mut BakeTime = push::<BakeTime>(bc.tmp_times_mut_ptr(), keys.count);
    ufbxi_check_err!(bc.error_mut_ptr(), !times.is_null(), "times");
    for i in 0..keys.count {
        let flags: BakedKeyFlags = (*keys.data.add(i)).flags;
        let mut time: f64 = (*keys.data.add(i)).time;
        if (flags.raw() & BakedKeyFlags::STEP_LEFT.raw()) != 0
            && i + 1 < keys.count
            && ((*keys.data.add(i + 1)).flags.raw() & BakedKeyFlags::STEP_KEY.raw()) != 0
        {
            time = (*keys.data.add(i + 1)).time;
        } else if (flags.raw() & BakedKeyFlags::STEP_RIGHT.raw()) != 0
            && i > 0
            && ((*keys.data.add(i - 1)).flags.raw() & BakedKeyFlags::STEP_KEY.raw()) != 0
        {
            time = (*keys.data.add(i - 1)).time;
        }
        (*times.add(i)).time = time;
        (*times.add(i)).flags = flags.raw() & 0x7;
    }

    Ok(())
}

// ufbx.c:27233-27490 `ufbxi_bake_node_imp`
#[cfg(feature = "baking")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn bake_node_imp(
    bc: &BakeContext,
    element_id: u32,
    props: *mut BakeProp,
    count: usize,
) -> Result<(), Fail> {
    ufbx_assert!(!bc.baked_nodes().is_null() && !bc.nodes_to_bake().is_null());

    let node: *mut UfbxNode =
        *((*bc.scene()).elements.data as *const *mut UfbxNode).add(element_id as usize);
    ufbxi_dev_assert!((*node).element.type_ as u32 == ElementType::Node as u32);

    let mut complex_translation: bool = false;
    let mut complex_rotation: bool = false;

    for i in 0..COMPLEX_TRANSLATION_PROPS.0.len() {
        let name: *const u8 = COMPLEX_TRANSLATION_PROPS.0[i];
        let prop: *mut Prop = find_prop(ptr::addr_of!((*node).element.props), name);
        // C: `prop->value_vec3` — the `ufbx_prop` value union's 3-real view
        // over `value_vec4`.
        if !prop.is_null() && !is_vec3_zero(*(ptr::addr_of!((*prop).value_vec4) as *const Vec3)) {
            complex_translation = true;
        }
        // C: `ufbxi_for(ufbxi_bake_prop, bprop, props, count)`
        let mut bprop: *mut BakeProp = props;
        let bprop_end: *mut BakeProp = add_ptr(props, count);
        while bprop != bprop_end {
            if (*bprop).prop_name == name {
                complex_translation = true;
            }
            bprop = bprop.add(1);
        }
    }

    for i in 0..COMPLEX_ROTATION_PROPS.0.len() {
        let name: *const u8 = COMPLEX_ROTATION_PROPS.0[i];
        let mut bprop: *mut BakeProp = props;
        let bprop_end: *mut BakeProp = add_ptr(props, count);
        while bprop != bprop_end {
            if (*bprop).prop_name == name {
                complex_rotation = true;
            }
            bprop = bprop.add(1);
        }
    }

    // C: `ufbxi_bake_time_list times_t, times_r, times_s;` — each is filled in
    // by the `ufbxi_finalize_bake_times` call below.
    let mut times_t: BakeTimeList = MaybeUninit::zeroed().assume_init();
    let mut times_r: BakeTimeList = MaybeUninit::zeroed().assume_init();
    let mut times_s: BakeTimeList = MaybeUninit::zeroed().assume_init();

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
    let parent: *mut UfbxNode = opt_ptr(ptr::addr_of!((*node).parent));
    let parent_scale_helper: *mut UfbxNode = if !parent.is_null() {
        opt_ptr(ptr::addr_of!((*parent).scale_helper))
    } else {
        ptr::null_mut()
    };
    if !(*node).is_scale_helper && !parent.is_null() && !parent_scale_helper.is_null() {
        scale_helper_t = *(*bc.get())
            .baked_nodes
            .add((*parent_scale_helper).element.typed_id as usize);
        if !scale_helper_t.is_null() {
            if !(*scale_helper_t).constant_scale {
                resample_translation = true;
            }
            push_resampled_times(bc, ptr::addr_of!((*scale_helper_t).scale_keys))?;
        } else {
            constant_scale_t = (*parent_scale_helper).inherit_scale;
        }
    }

    if complex_translation {
        // C: `ufbxi_for(ufbxi_bake_prop, prop, props, count)`
        let mut prop: *mut BakeProp = props;
        let prop_end: *mut BakeProp = add_ptr(props, count);
        while prop != prop_end {
            // Literally any transform related property can affect complex translation
            if in_list(
                TRANSFORM_PROPS.0.as_ptr(),
                TRANSFORM_PROPS.0.len(),
                (*prop).prop_name,
            ) {
                let resample_linear: bool =
                    resample_translation || (*prop).prop_name != sp::Lcl_Translation.as_ptr();
                let key_flag: u32 = if (*prop).prop_name == sp::Lcl_Translation.as_ptr() {
                    BakedKeyFlags::KEYFRAME.raw()
                } else {
                    0
                };
                bake_times(bc, (*prop).anim_value, resample_linear, key_flag)?;
            }
            prop = prop.add(1);
        }
    } else {
        let mut prop: *mut BakeProp = props;
        let prop_end: *mut BakeProp = add_ptr(props, count);
        while prop != prop_end {
            if (*prop).prop_name == sp::Lcl_Translation.as_ptr() {
                bake_times(
                    bc,
                    (*prop).anim_value,
                    resample_translation,
                    BakedKeyFlags::KEYFRAME.raw(),
                )?;
            }
            prop = prop.add(1);
        }
    }

    finalize_bake_times(bc, ptr::addr_of_mut!(times_t))?;

    // Rotation
    if complex_rotation {
        let mut prop: *mut BakeProp = props;
        let prop_end: *mut BakeProp = add_ptr(props, count);
        while prop != prop_end {
            if in_list(
                COMPLEX_ROTATION_SOURCES.0.as_ptr(),
                COMPLEX_ROTATION_SOURCES.0.len(),
                (*prop).prop_name,
            ) {
                let resample_linear: bool = !(*bc.get()).opts.no_resample_rotation
                    || (*prop).prop_name != sp::Lcl_Rotation.as_ptr();
                let key_flag: u32 = if (*prop).prop_name == sp::Lcl_Rotation.as_ptr() {
                    BakedKeyFlags::KEYFRAME.raw()
                } else {
                    0
                };
                bake_times(bc, (*prop).anim_value, resample_linear, key_flag)?;
            }
            prop = prop.add(1);
        }
    } else {
        let mut prop: *mut BakeProp = props;
        let prop_end: *mut BakeProp = add_ptr(props, count);
        while prop != prop_end {
            if (*prop).prop_name == sp::Lcl_Rotation.as_ptr() {
                bake_times(
                    bc,
                    (*prop).anim_value,
                    !(*bc.get()).opts.no_resample_rotation,
                    BakedKeyFlags::KEYFRAME.raw(),
                )?;
            }
            prop = prop.add(1);
        }
    }
    finalize_bake_times(bc, ptr::addr_of_mut!(times_r))?;

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
        opt_ptr(ptr::addr_of!((*parent).inherit_scale_node))
    } else {
        ptr::null_mut()
    };
    let parent_inherit_scale_helper: *mut UfbxNode = if !parent_inherit_scale_node.is_null() {
        opt_ptr(ptr::addr_of!((*parent_inherit_scale_node).scale_helper))
    } else {
        ptr::null_mut()
    };
    if (*node).is_scale_helper
        && !parent.is_null()
        && !parent_inherit_scale_node.is_null()
        && !parent_inherit_scale_helper.is_null()
    {
        let inherit_helper: *mut UfbxNode = parent_inherit_scale_helper;
        scale_helper_s = *(*bc.get())
            .baked_nodes
            .add((*inherit_helper).element.typed_id as usize);
        if !scale_helper_s.is_null() {
            if !(*scale_helper_s).constant_scale {
                resample_scale = true;
            }
            push_resampled_times(bc, ptr::addr_of!((*scale_helper_s).scale_keys))?;
        } else {
            constant_scale_s = (*inherit_helper).local_transform.scale;
        }
    }

    {
        let mut prop: *mut BakeProp = props;
        let prop_end: *mut BakeProp = add_ptr(props, count);
        while prop != prop_end {
            if (*prop).prop_name == sp::Lcl_Scaling.as_ptr() {
                bake_times(
                    bc,
                    (*prop).anim_value,
                    resample_scale,
                    BakedKeyFlags::KEYFRAME.raw(),
                )?;
            }
            prop = prop.add(1);
        }
    }
    finalize_bake_times(bc, ptr::addr_of_mut!(times_s))?;

    // C: `ufbx_baked_vec3_list keys_t; ufbx_baked_quat_list keys_r; ufbx_baked_vec3_list keys_s;`
    let mut keys_t: List<BakedVec3> = MaybeUninit::zeroed().assume_init();
    let mut keys_r: List<BakedQuat> = MaybeUninit::zeroed().assume_init();
    let mut keys_s: List<BakedVec3> = MaybeUninit::zeroed().assume_init();

    keys_t.count = times_t.count;
    keys_t.data = push::<BakedVec3>(bc.tmp_prop_mut_ptr(), keys_t.count);
    ufbxi_check_err!(bc.error_mut_ptr(), !keys_t.data.is_null(), "keys_t.data");

    keys_r.count = times_r.count;
    keys_r.data = push::<BakedQuat>(bc.tmp_prop_mut_ptr(), keys_r.count);
    ufbxi_check_err!(bc.error_mut_ptr(), !keys_r.data.is_null(), "keys_r.data");

    keys_s.count = times_s.count;
    keys_s.data = push::<BakedVec3>(bc.tmp_prop_mut_ptr(), keys_s.count);
    ufbxi_check_err!(bc.error_mut_ptr(), !keys_s.data.is_null(), "keys_s.data");

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
            bake_time = *times_r.data.add(ix_r);
            flags_r = bake_time.flags;
            bake_time.flags &= 0x7;
            flags |= TransformFlags::INCLUDE_ROTATION.raw();
        }
        if ix_t < times_t.count {
            let t: BakeTime = *times_t.data.add(ix_t);
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
            let t: BakeTime = *times_s.data.add(ix_s);
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
        if ((*bc.get()).opts.evaluate_flags & EvaluateFlags::NO_EXTRAPOLATION.raw()) != 0 {
            flags |= TransformFlags::NO_EXTRAPOLATION.raw();
        }

        let eval_time: f64 = bake_time_sample_time(bake_time);
        let mut transform: Transform = evaluate_transform_flags(bc.anim(), node, eval_time, flags);

        if (flags & TransformFlags::INCLUDE_TRANSLATION.raw()) != 0 {
            if !scale_helper_t.is_null() {
                let scale: Vec3 = evaluate_baked_vec3(
                    ptr::read(ptr::addr_of!((*scale_helper_t).scale_keys)),
                    eval_time,
                );
                transform.translation.x *= scale.x;
                transform.translation.y *= scale.y;
                transform.translation.z *= scale.z;
            }

            transform.translation.x *= constant_scale_t.x;
            transform.translation.y *= constant_scale_t.y;
            transform.translation.z *= constant_scale_t.z;

            (*keys_t_data.add(ix_t)).time = bake_time.time;
            (*keys_t_data.add(ix_t)).value = transform.translation;
            (*keys_t_data.add(ix_t)).flags = BakedKeyFlags::from_raw(bake_time.flags | flags_t);
            ix_t += 1;
        }
        if (flags & TransformFlags::INCLUDE_ROTATION.raw()) != 0 {
            (*keys_r_data.add(ix_r)).time = bake_time.time;
            (*keys_r_data.add(ix_r)).value = transform.rotation;
            (*keys_r_data.add(ix_r)).flags = BakedKeyFlags::from_raw(bake_time.flags | flags_r);
            ix_r += 1;
        }
        if (flags & TransformFlags::INCLUDE_SCALE.raw()) != 0 {
            if !scale_helper_s.is_null() {
                let scale: Vec3 = evaluate_baked_vec3(
                    ptr::read(ptr::addr_of!((*scale_helper_s).scale_keys)),
                    eval_time,
                );
                transform.scale.x *= scale.x;
                transform.scale.y *= scale.y;
                transform.scale.z *= scale.z;
            }

            transform.scale.x *= constant_scale_s.x;
            transform.scale.y *= constant_scale_s.y;
            transform.scale.z *= constant_scale_s.z;

            (*keys_s_data.add(ix_s)).time = bake_time.time;
            (*keys_s_data.add(ix_s)).value = transform.scale;
            (*keys_s_data.add(ix_s)).flags = BakedKeyFlags::from_raw(bake_time.flags | flags_s);
            ix_s += 1;
        }
    }

    let baked_node: *mut BakedNode = push_zero::<BakedNode>(bc.tmp_nodes_mut_ptr(), 1);
    ufbxi_check_err!(bc.error_mut_ptr(), !baked_node.is_null(), "baked_node");

    (*baked_node).element_id = (*node).element.element_id;
    (*baked_node).typed_id = (*node).element.typed_id;
    bake_postprocess_vec3(
        bc,
        ptr::addr_of_mut!((*baked_node).translation_keys),
        ptr::addr_of_mut!((*baked_node).constant_translation),
        keys_t,
    )?;
    bake_postprocess_quat(
        bc,
        ptr::addr_of_mut!((*baked_node).rotation_keys),
        ptr::addr_of_mut!((*baked_node).constant_rotation),
        keys_r,
    )?;
    bake_postprocess_vec3(
        bc,
        ptr::addr_of_mut!((*baked_node).scale_keys),
        ptr::addr_of_mut!((*baked_node).constant_scale),
        keys_s,
    )?;

    *(*bc.get())
        .baked_nodes
        .add((*node).element.typed_id as usize) = baked_node;

    buf_clear(bc.tmp_prop_mut_ptr());

    // If this node is a scale helper, make sure to bake its siblings and
    // potentially their scale helpers if they are not a part of the animation.
    if (*node).is_scale_helper {
        ufbx_assert!(!parent.is_null());
        // C: `ufbxi_for_ptr_list(ufbx_node, p_child, node->parent->children)`
        let mut p_child: *mut *mut UfbxNode = (*parent).children.data as *mut *mut UfbxNode;
        let p_child_end: *mut *mut UfbxNode = add_ptr(p_child, (*parent).children.count);
        while p_child != p_child_end {
            let child: *mut UfbxNode = *p_child;
            if child == node {
                p_child = p_child.add(1);
                continue;
            }
            if !*(*bc.get())
                .nodes_to_bake
                .add((*child).element.typed_id as usize)
            {
                *(*bc.get())
                    .nodes_to_bake
                    .add((*child).element.typed_id as usize) = true;
                ufbxi_check_err!(
                    bc.error_mut_ptr(),
                    !push_copy::<u32>(
                        bc.tmp_bake_stack_mut_ptr(),
                        1,
                        ptr::addr_of!((*child).element.element_id),
                    )
                    .is_null(),
                    "((uint32_t*)ufbxi_push_size_copy((&bc->tmp_bake_stack), sizeof(uint32_t), (1), (&child->element_id)))"
                );
            }
            // C: `child->inherit_scale_node && child->inherit_scale_node->scale_helper && child->scale_helper`
            let child_inherit_scale_node: *mut UfbxNode =
                opt_ptr(ptr::addr_of!((*child).inherit_scale_node));
            let child_inherit_scale_helper: *mut UfbxNode = if !child_inherit_scale_node.is_null() {
                opt_ptr(ptr::addr_of!((*child_inherit_scale_node).scale_helper))
            } else {
                ptr::null_mut()
            };
            let child_scale_helper: *mut UfbxNode = opt_ptr(ptr::addr_of!((*child).scale_helper));
            if !child_inherit_scale_node.is_null()
                && !child_inherit_scale_helper.is_null()
                && !child_scale_helper.is_null()
                && *(*bc.get())
                    .nodes_to_bake
                    .add((*child_inherit_scale_helper).element.typed_id as usize)
            {
                ufbx_assert!(!(*(*bc.get())
                    .baked_nodes
                    .add((*child_inherit_scale_helper).element.typed_id as usize))
                .is_null());
                if !*(*bc.get())
                    .nodes_to_bake
                    .add((*child_scale_helper).element.typed_id as usize)
                {
                    *(*bc.get())
                        .nodes_to_bake
                        .add((*child_scale_helper).element.typed_id as usize) = true;
                    ufbxi_check_err!(
                        bc.error_mut_ptr(),
                        !push_copy::<u32>(
                            bc.tmp_bake_stack_mut_ptr(),
                            1,
                            ptr::addr_of!((*child_scale_helper).element.element_id),
                        )
                        .is_null(),
                        "((uint32_t*)ufbxi_push_size_copy((&bc->tmp_bake_stack), sizeof(uint32_t), (1), (&child->scale_helper->element_id)))"
                    );
                }
            }
            p_child = p_child.add(1);
        }
    }

    Ok(())
}

// ufbx.c:27492-27505 `ufbxi_bake_node`
#[cfg(feature = "baking")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn bake_node(
    bc: &BakeContext,
    element_id: u32,
    props: *mut BakeProp,
    count: usize,
) -> Result<(), Fail> {
    bake_node_imp(bc, element_id, props, count)?;

    // Baking a node may cause further nodes to be baked, so keep going
    // until all dependencies are baked.
    while (*bc.get()).tmp_bake_stack.num_items > 0 {
        let mut child_id: u32 = 0;
        pop::<u32>(bc.tmp_bake_stack_mut_ptr(), 1, ptr::addr_of_mut!(child_id));
        bake_node_imp(bc, child_id, ptr::null_mut(), 0)?;
    }

    Ok(())
}

// ufbx.c:27507-27546 `ufbxi_bake_anim_prop`
#[cfg(feature = "baking")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn bake_anim_prop(
    bc: &BakeContext,
    element: *mut Element,
    prop_name: *const u8,
    props: *mut BakeProp,
    count: usize,
) -> Result<(), Fail> {
    // C: `ufbxi_for(ufbxi_bake_prop, prop, props, count)`
    let mut prop: *mut BakeProp = props;
    let prop_end: *mut BakeProp = add_ptr(props, count);
    while prop != prop_end {
        bake_times(bc, (*prop).anim_value, false, BakedKeyFlags::KEYFRAME.raw())?;
        prop = prop.add(1);
    }

    // C: `ufbxi_bake_time_list times;`
    let mut times: BakeTimeList = MaybeUninit::zeroed().assume_init();
    finalize_bake_times(bc, ptr::addr_of_mut!(times))?;

    // C: `ufbx_baked_vec3_list keys;`
    let mut keys: List<BakedVec3> = MaybeUninit::zeroed().assume_init();
    keys.count = times.count;
    keys.data = push::<BakedVec3>(bc.tmp_prop_mut_ptr(), keys.count);
    ufbxi_check_err!(bc.error_mut_ptr(), !keys.data.is_null(), "keys.data");
    let keys_data: *mut BakedVec3 = keys.data as *mut BakedVec3;

    // C: `ufbx_string name; name.data = prop_name; name.length = strlen(prop_name);`
    let name: String = String::new_c(prop_name, strlen(prop_name));

    for i in 0..times.count {
        let bake_time: BakeTime = *times.data.add(i);
        let eval_time: f64 = bake_time_sample_time(bake_time);
        let prop: Prop = evaluate_prop_flags_len(
            bc.anim(),
            element,
            name.data,
            name.length,
            eval_time,
            (*bc.get()).opts.evaluate_flags,
        );
        (*keys_data.add(i)).time = bake_time.time;
        // C: `prop.value_vec3` — the value union's 3-real view over `value_vec4`.
        (*keys_data.add(i)).value = *(ptr::addr_of!(prop.value_vec4) as *const Vec3);
        (*keys_data.add(i)).flags = BakedKeyFlags::from_raw(bake_time.flags);
    }

    let baked_prop: *mut BakedProp = push_zero::<BakedProp>(bc.tmp_props_mut_ptr(), 1);
    ufbxi_check_err!(bc.error_mut_ptr(), !baked_prop.is_null(), "baked_prop");

    (*baked_prop).name.length = strlen(prop_name);
    (*baked_prop).name.data = push_copy::<u8>(
        bc.result_mut_ptr(),
        (*baked_prop).name.length + 1,
        prop_name,
    );
    ufbxi_check_err!(
        bc.error_mut_ptr(),
        !(*baked_prop).name.data.is_null(),
        "baked_prop->name.data"
    );

    bake_postprocess_vec3(
        bc,
        ptr::addr_of_mut!((*baked_prop).keys),
        ptr::addr_of_mut!((*baked_prop).constant_value),
        keys,
    )?;

    buf_clear(bc.tmp_prop_mut_ptr());

    Ok(())
}

// ufbx.c:27548-27585 `ufbxi_bake_element`
#[cfg(feature = "baking")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn bake_element(
    bc: &BakeContext,
    element_id: u32,
    props: *mut BakeProp,
    count: usize,
) -> Result<(), Fail> {
    let element: *mut Element =
        *((*bc.scene()).elements.data as *const *mut Element).add(element_id as usize);
    if (*element).type_ as u32 == ElementType::Node as u32 && !(*bc.get()).opts.skip_node_transforms
    {
        bake_node(bc, element_id, props, count)?;
    }

    let mut begin: usize = 0;
    while begin < count {
        let prop_name: *const u8 = (*props.add(begin)).prop_name;
        let mut end: usize = begin + 1;
        while end < count && (*props.add(end)).prop_name == prop_name {
            end += 1;
        }

        // Don't bake transform related props for nodes unless specifically requested
        if (*element).type_ as u32 == ElementType::Node as u32
            && !(*bc.get()).opts.bake_transform_props
            && in_list(
                TRANSFORM_PROPS.0.as_ptr(),
                TRANSFORM_PROPS.0.len(),
                prop_name,
            )
        {
            begin = end;
            continue;
        }

        bake_anim_prop(bc, element, prop_name, props.add(begin), end - begin)?;
        begin = end;
    }

    let num_props: usize = (*bc.get()).tmp_props.num_items;
    if num_props > 0 {
        let baked_elem: *mut BakedElement = push_zero::<BakedElement>(bc.tmp_elements_mut_ptr(), 1);
        ufbxi_check_err!(bc.error_mut_ptr(), !baked_elem.is_null(), "baked_elem");

        (*baked_elem).element_id = (*element).element_id;
        (*baked_elem).props.count = num_props;
        (*baked_elem).props.data =
            push_pop::<BakedProp>(bc.result_mut_ptr(), bc.tmp_props_mut_ptr(), num_props);
        ufbxi_check_err!(
            bc.error_mut_ptr(),
            !(*baked_elem).props.data.is_null(),
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
    (*a).typed_id < (*b).typed_id
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
    (*a).element_id < (*b).element_id
}

// ufbx.c:27601-27705 `ufbxi_bake_anim`
#[cfg(feature = "baking")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn bake_anim(bc: &BakeContext) -> Result<(), Fail> {
    let anim: *const Anim = bc.anim();
    let scene: *const Scene = bc.scene();

    if !(*bc.get()).opts.skip_node_transforms {
        bc.set_baked_nodes(push_zero::<*mut BakedNode>(
            bc.result_mut_ptr(),
            (*scene).nodes.count,
        ));
        ufbxi_check_err!(
            bc.error_mut_ptr(),
            !bc.baked_nodes().is_null(),
            "bc->baked_nodes"
        );
        bc.set_nodes_to_bake(push_zero::<bool>(bc.result_mut_ptr(), (*scene).nodes.count));
        ufbxi_check_err!(
            bc.error_mut_ptr(),
            !bc.nodes_to_bake().is_null(),
            "bc->nodes_to_bake"
        );
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_layer, p_layer, anim->layers)`
    let mut p_layer: *mut *mut AnimLayer = (*anim).layers.data as *mut *mut AnimLayer;
    let p_layer_end: *mut *mut AnimLayer = add_ptr(p_layer, (*anim).layers.count);
    while p_layer != p_layer_end {
        let layer: *mut AnimLayer = *p_layer;

        // C: `ufbxi_for_list(ufbx_anim_prop, anim_prop, layer->anim_props)`
        let mut anim_prop: *mut AnimProp = (*layer).anim_props.data as *mut AnimProp;
        let anim_prop_end: *mut AnimProp = add_ptr(anim_prop, (*layer).anim_props.count);
        while anim_prop != anim_prop_end {
            let prop: *mut BakeProp = push::<BakeProp>(bc.tmp_bake_props_mut_ptr(), 1);
            ufbxi_check_err!(bc.error_mut_ptr(), !prop.is_null(), "prop");

            let element: *mut Element = ref_ptr(ptr::addr_of!((*anim_prop).element));

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
            (*prop).anim_value = ref_ptr(ptr::addr_of!((*anim_prop).anim_value));

            anim_prop = anim_prop.add(1);
        }

        p_layer = p_layer.add(1);
    }

    let num_props: usize = (*bc.get()).tmp_bake_props.num_items;
    let props: *mut BakeProp =
        push_pop::<BakeProp>(bc.tmp_mut_ptr(), bc.tmp_bake_props_mut_ptr(), num_props);
    ufbxi_check_err!(bc.error_mut_ptr(), !props.is_null(), "props");

    unstable_sort(
        props as *mut c_void,
        num_props,
        size_of::<BakeProp>(),
        bake_prop_less,
        ptr::null_mut(),
    );

    // Pre-bake layer weight times
    if !(*bc.get()).opts.ignore_layer_weight_animation {
        let mut has_weight_times: bool = false;
        // C: `ufbxi_for(ufbxi_bake_prop, prop, props, num_props)`
        let mut prop: *mut BakeProp = props;
        let prop_end: *mut BakeProp = add_ptr(props, num_props);
        while prop != prop_end {
            if (*prop).prop_name != sp::Weight.as_ptr() {
                prop = prop.add(1);
                continue;
            }
            let element: *mut Element =
                *((*scene).elements.data as *const *mut Element).add((*prop).element_id as usize);
            if (*element).type_ as u32 == ElementType::AnimLayer as u32 {
                bake_times(bc, (*prop).anim_value, true, 0)?;
                has_weight_times = true;
            }
            prop = prop.add(1);
        }

        if has_weight_times {
            // C: `ufbxi_bake_time_list weight_times = { 0 };`
            let mut weight_times: BakeTimeList = MaybeUninit::zeroed().assume_init();
            finalize_bake_times(bc, ptr::addr_of_mut!(weight_times))?;

            (*bc.get()).layer_weight_times.count = weight_times.count;
            (*bc.get()).layer_weight_times.data =
                push_copy::<BakeTime>(bc.tmp_mut_ptr(), weight_times.count, weight_times.data);
            ufbxi_check_err!(
                bc.error_mut_ptr(),
                !(*bc.get()).layer_weight_times.data.is_null(),
                "bc->layer_weight_times.data"
            );

            buf_clear(bc.tmp_prop_mut_ptr());
        }
    }

    let mut begin: usize = 0;
    while begin < num_props {
        let element_id: u32 = (*props.add(begin)).element_id;
        let mut end: usize = begin + 1;
        while end < num_props && (*props.add(end)).element_id == element_id {
            end += 1;
        }
        bake_element(bc, element_id, props.add(begin), end - begin)?;
        begin = end;
    }

    let num_nodes: usize = (*bc.get()).tmp_nodes.num_items;
    let num_elements: usize = (*bc.get()).tmp_elements.num_items;

    (*bc.get()).bake.nodes.count = num_nodes;
    (*bc.get()).bake.nodes.data =
        push_pop::<BakedNode>(bc.result_mut_ptr(), bc.tmp_nodes_mut_ptr(), num_nodes);
    ufbxi_check_err!(
        bc.error_mut_ptr(),
        !(*bc.get()).bake.nodes.data.is_null(),
        "bc->bake.nodes.data"
    );

    (*bc.get()).bake.elements.count = num_elements;
    (*bc.get()).bake.elements.data =
        push_pop::<BakedElement>(bc.result_mut_ptr(), bc.tmp_elements_mut_ptr(), num_elements);
    ufbxi_check_err!(
        bc.error_mut_ptr(),
        !(*bc.get()).bake.elements.data.is_null(),
        "bc->bake.elements.data"
    );

    unstable_sort(
        (*bc.get()).bake.nodes.data as *mut c_void,
        (*bc.get()).bake.nodes.count,
        size_of::<BakedNode>(),
        baked_node_less,
        ptr::null_mut(),
    );
    unstable_sort(
        (*bc.get()).bake.elements.data as *mut c_void,
        (*bc.get()).bake.elements.count,
        size_of::<BakedElement>(),
        baked_element_less,
        ptr::null_mut(),
    );

    if bc.time_min() < bc.time_max() {
        (*bc.get()).bake.key_time_min = bc.time_min();
        (*bc.get()).bake.key_time_max = bc.time_max();
    }

    if bc.time_begin() < bc.time_end() {
        (*bc.get()).bake.playback_time_begin = bc.time_begin();
        (*bc.get()).bake.playback_time_end = bc.time_end();
        (*bc.get()).bake.playback_duration = bc.time_end() - bc.time_begin();
    }

    Ok(())
}

// ufbx.c:27707-27765 `ufbxi_bake_anim_imp`
#[cfg(feature = "baking")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn bake_anim_imp(bc: &BakeContext, anim: *const Anim) -> Result<(), Fail> {
    if (*bc.get()).opts.resample_rate <= 0.0 {
        (*bc.get()).opts.resample_rate = 30.0;
    }
    if (*bc.get()).opts.minimum_sample_rate <= 0.0 {
        (*bc.get()).opts.minimum_sample_rate = 19.5;
    }
    if (*bc.get()).opts.max_keyframe_segments == 0 {
        (*bc.get()).opts.max_keyframe_segments = 32;
    }
    if (*bc.get()).opts.key_reduction_threshold == 0.0 {
        (*bc.get()).opts.key_reduction_threshold = 0.000001;
    }
    if (*bc.get()).opts.key_reduction_passes == 0 {
        (*bc.get()).opts.key_reduction_passes = 4;
    }

    if (*bc.get()).opts.trim_start_time && (*anim).time_begin > 0.0 {
        bc.set_ktime_offset(-(*anim).time_begin * (*bc.scene()).metadata.ktime_second as f64);
    }

    init_ator(
        bc.error_mut_ptr(),
        bc.ator_tmp_mut_ptr(),
        ptr::addr_of!((*bc.get()).opts.temp_allocator),
        b"temp\0".as_ptr(),
    );
    init_ator(
        bc.error_mut_ptr(),
        bc.ator_result_mut_ptr(),
        ptr::addr_of!((*bc.get()).opts.result_allocator),
        b"result\0".as_ptr(),
    );

    (*bc.get()).result.unordered = true;
    (*bc.get()).result.ator = bc.ator_result_mut_ptr();

    (*bc.get()).tmp.unordered = true;
    (*bc.get()).tmp.ator = bc.ator_tmp_mut_ptr();

    (*bc.get()).tmp_prop.ator = bc.ator_tmp_mut_ptr();
    (*bc.get()).tmp_prop.unordered = true;
    (*bc.get()).tmp_prop.clearable = true;

    (*bc.get()).tmp_times.ator = bc.ator_tmp_mut_ptr();
    (*bc.get()).tmp_bake_props.ator = bc.ator_tmp_mut_ptr();
    (*bc.get()).tmp_nodes.ator = bc.ator_tmp_mut_ptr();
    (*bc.get()).tmp_elements.ator = bc.ator_tmp_mut_ptr();
    (*bc.get()).tmp_props.ator = bc.ator_tmp_mut_ptr();
    (*bc.get()).tmp_bake_stack.ator = bc.ator_tmp_mut_ptr();

    bc.set_anim(anim);
    if (*anim).time_begin < (*anim).time_end {
        bc.set_time_begin((*anim).time_begin);
        bc.set_time_end((*anim).time_end);
    }
    bc.set_time_min(math::INFINITY);
    bc.set_time_max(-math::INFINITY);

    bc.set_imp(push::<BakedAnimImp>(bc.result_mut_ptr(), 1));
    ufbxi_check_err!(bc.error_mut_ptr(), !bc.imp().is_null(), "bc->imp");

    // Expose the wide allocation so `get_imp` can recover this header from a
    // (possibly narrowed) public `&BakedAnim` pointer via exposed provenance.
    (bc.imp() as *mut u8).expose_provenance();

    bake_anim(bc)?;

    init_ref(
        ptr::addr_of_mut!((*bc.imp()).refcount),
        BAKED_ANIM_IMP_MAGIC,
        ptr::null_mut(),
    );

    (*bc.get()).bake.metadata.result_memory_used = (*bc.get()).ator_result.current_size;
    (*bc.get()).bake.metadata.temp_memory_used = (*bc.get()).ator_tmp.current_size;
    (*bc.get()).bake.metadata.result_allocs = (*bc.get()).ator_result.num_allocs;
    (*bc.get()).bake.metadata.temp_allocs = (*bc.get()).ator_tmp.num_allocs;

    (*bc.imp()).magic = BAKED_ANIM_IMP_MAGIC;
    // C: `bc->imp->bake = bc->bake;` (struct assignment)
    ptr::copy_nonoverlapping(
        ptr::addr_of!((*bc.get()).bake),
        ptr::addr_of_mut!((*bc.imp()).bake),
        1,
    );
    (*bc.imp()).refcount.ator = (*bc.get()).ator_result;
    (*bc.imp()).refcount.buf = (*bc.get()).result;

    Ok(())
}

// CONTINUATION POINT: the `// -- Animation baking` section is complete
// (ufbx.c:26670-27767); the C `#endif` at ufbx.c:27767 closes the
// `feature = "baking"` gate above. Next banner: ufbx.c:27769 `// -- NURBS`
// (owned by `native::nurbs`).
