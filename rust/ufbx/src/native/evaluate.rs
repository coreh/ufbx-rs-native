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
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
use core::ffi::c_void;
use core::mem::{size_of, MaybeUninit};
use core::ptr;

use crate::generated::{
    Anim, AnimCurve, AnimLayer, AnimProp, BakedAnim, Connection, DomNode, Element, Error,
    ErrorType, Extrapolation, ExtrapolationMode, FileFormat, IndexErrorHandling, InflateRetain,
    Keyframe, OpenFileInfo, OpenFileType, Prop, PropFlags, PropOverride, PropType, Quat,
    RawAnimOpts, RawGeometryCacheDataOpts, RawLoadOpts, RawOpenFileOpts, RawPropOverrideDesc,
    RawStream, RotationOrder, Scene, Tangent, TransformOverride, UnicodeErrorHandling, Vec3, Vec4,
    Warning, WarningType,
};
#[cfg(feature = "scene-eval")]
use crate::generated::{
    AnimStack, AudioLayer, BlendChannel, BlendKeyframe, BlendShape, BonePose, CacheFile, Camera,
    Constraint, ConstraintTarget, DisplayLayer, Material, MaterialMap, MaterialTexture,
    NameElement, Pose, RawEvaluateOpts, SelectionNode, SelectionSet, Shader, ShaderTexture,
    ShaderTextureInput, SkinCluster, StereoCamera, Texture, TextureLayer, Video,
};
#[cfg(any(feature = "scene-eval", feature = "baking"))]
use crate::generated::{AnimValue, Node as UfbxNode};
#[cfg(feature = "baking")]
use crate::generated::{
    BakeStepHandling, BakedElement, BakedKeyFlags, BakedNode, BakedProp, BakedQuat, BakedVec3,
    ElementType, EvaluateFlags, Interpolation, RawBakeOpts, Transform, TransformFlags,
};
#[cfg(feature = "scene-eval")]
use crate::generated::{BlendDeformer, CacheDeformer, Mesh, SkinDeformer};
#[cfg(feature = "skinning-eval")]
use crate::generated::{CacheChannel, CacheInterpretation, Matrix, TopoEdge};
use crate::native::allocator::{
    free, free_ator, init_ator, Allocator, AllocatorView, SCENE_IMP_MAGIC, ZERO_SIZE_BUFFER,
};
#[cfg(feature = "baking")]
use crate::native::allocator::{grow_array, BAKED_ANIM_IMP_MAGIC};
#[cfg(feature = "skinning-eval")]
use crate::native::api::{
    add_blend_vertex_offsets_run, catch_get_skin_vertex_matrix, compute_normals, compute_topology,
    generate_normal_mapping, sample_geometry_cache_vec3, transform_position, ZERO_VEC3,
};
use crate::native::api::{coordinate_axes_valid, default_open_file, open_file_ctx, EMPTY_STRING};
use crate::native::api::{
    euler_to_quat, evaluate_anim_value_real_flags, evaluate_anim_value_vec3_flags,
    evaluate_curve_flags, evaluate_prop_flags_len_view, evaluate_prop_len_view, init_ref,
    quat_slerp, quat_to_euler, EvalPropName, IDENTITY_QUAT,
};
#[cfg(feature = "baking")]
use crate::native::api::{evaluate_baked_vec3_slice, evaluate_transform_flags, quat_fix_antipodal};
#[cfg(feature = "scene-eval")]
use crate::native::api::{evaluate_props_flags, ELEMENT_TYPE_SIZE};
#[cfg(feature = "baking")]
use crate::native::buf::{buf_clear, pop};
use crate::native::buf::{buf_free, Buf, BufView};
use crate::native::cache::{load_external_files, scale_units, transform_to_axes};
#[cfg(not(feature = "skinning-eval"))]
use crate::native::error::ufbxi_report_err_msg;
use crate::native::error::{
    fix_error_type, set_err_info, strcmp, strlen, ufbxi_check, ufbxi_check_err,
    ufbxi_check_err_msg, ufbxi_check_msg, ufbxi_fail_err_msg, ufbxi_fail_msg, ufbxi_fmt_err_info,
    utf8_valid_length, Fail, EMPTY_CHAR,
};
use crate::native::float_parse::parse_double_init_flags;
use crate::native::hash::{
    map_cmp_const_char_ptr, map_cmp_ptr_id, map_cmp_uint64, map_cmp_uintptr, map_free, map_init,
};
use crate::native::obj::{mtl_load, obj_free, obj_load};
use crate::native::parse::{
    begin_parse, determine_format, finish_imp, get_name_key, get_name_key_c, load_maps,
    load_strings, Context, FinishedImp, ImpHandle, Node, Refcount, SceneImp, SceneMetadataView,
    SceneView, ELEMENT_TYPE_COUNT, MIN_FILE_FORMAT_LOOKAHEAD,
};
#[cfg(feature = "baking")]
use crate::native::parse::{find_prop, is_vec3_zero};
#[cfg(feature = "skinning-eval")]
use crate::native::platform::max_sz;
use crate::native::platform::{
    add_ptr, f64_to_i64, math, ufbx_assert, ufbxi_dev_assert, ufbxi_ignore, unstable_sort,
    PATH_SEPARATOR,
};
#[cfg(feature = "baking")]
use crate::native::platform::{macro_stable_sort, ufbxi_unreachable};
#[cfg(feature = "scene-eval")]
use crate::native::read::opt_ref;
use crate::native::read::{
    init_file_paths, open_file, read_legacy_root, read_root, ref_ptr, supports_version,
    SYNTHETIC_ID_START,
};
use crate::native::scene_process::{
    finalize_scene, find_anim_prop_start, find_prop_connection, modify_geometry, mul_quat,
    postprocess_scene, pre_finalize_scene, update_adjust_transforms, update_scene,
    update_scene_metadata, update_scene_settings, update_scene_settings_obj, AnimImp,
    ConnectionPropKey,
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
use crate::native::view::Run;
use crate::native::view::SliceViewIter;
use crate::native::view::{
    view_project, view_raw_const, view_raw_mut, view_raw_shared, view_read, view_read_shared,
    view_write,
};
use crate::native::view::{Const, Mode, Mut, View};
use crate::native::warnings::{pop_warnings, ufbxi_warnf};
use crate::prelude::as_f64;
#[cfg(feature = "baking")]
use crate::prelude::ListView;
#[cfg(any(feature = "scene-eval", feature = "baking"))]
use crate::prelude::ScalarView;
use crate::prelude::{List, OpenFileContext, RawStringView, Real, Ref, String, StringView};

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

// Hand view accessor for `ufbxi_evaluate_skinning`: `ufbx_cache_channel`
// carries no generated `View` impl, so the one field this function navigates
// (C: `channel->interpretation`) gets a single-level leaf accessor here.
#[cfg(feature = "skinning-eval")]
impl View<CacheChannel, Mut> {
    #[inline(always)]
    pub(crate) fn interpretation(&self) -> CacheInterpretation {
        view_read!(self, interpretation)
    }
}

// ufbx.c:25055-25169 `ufbxi_evaluate_skinning`
// C forks on `#if UFBXI_FEATURE_SKINNING_EVALUATION` inside the function; the
// arms are split into cfg-gated fns (the same split `ufbxi_obj_load` uses in
// `native::obj`). C return type `int` (1 = success) → `Result<(), Fail>`.
#[cfg(feature = "skinning-eval")]
#[inline(never)]
pub(crate) fn evaluate_skinning(
    scene: &SceneView,
    error: &crate::native::error::ErrorView,
    buf_result: &BufView,
    buf_tmp: &BufView,
    time: f64,
    load_caches: bool,
    cache_opts: &RawGeometryCacheDataOpts,
) -> Result<(), Fail> {
    let mut max_skinned_indices: usize = 0;

    // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, scene->meshes)`
    let meshes = scene.meshes_view();
    for i_mesh in 0..meshes.count() {
        let mesh = meshes.at(i_mesh);
        if mesh.blend_deformers().count == 0
            && mesh.skin_deformers().count == 0
            && (mesh.cache_deformers().count == 0 || !load_caches)
        {
            continue;
        }
        max_skinned_indices = max_sz(max_skinned_indices, mesh.num_indices());
    }

    let topo: *mut TopoEdge = buf_tmp.push::<TopoEdge>(max_skinned_indices);
    ufbxi_check_err!(error, !topo.is_null(), "topo");

    // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, scene->meshes)`
    let meshes = scene.meshes_view();
    for i_mesh in 0..meshes.count() {
        let mesh = meshes.at(i_mesh);
        if mesh.blend_deformers().count == 0
            && mesh.skin_deformers().count == 0
            && (mesh.cache_deformers().count == 0 || !load_caches)
        {
            continue;
        }
        if mesh.num_vertices() == 0 {
            continue;
        }

        let num_vertices: usize = mesh.num_vertices();
        let mut result_pos: *mut Vec3 = buf_result.push::<Vec3>(num_vertices.wrapping_add(1));
        ufbxi_check_err!(error, !result_pos.is_null(), "result_pos");

        // C: `result_pos[0] = ufbx_zero_vec3; result_pos++;`
        // SAFETY: valid mesh counts satisfy the C allocation invariant
        // `num_vertices < SIZE_MAX`; the checked non-null result therefore has
        // a writable sentinel slot followed by `num_vertices` result slots.
        unsafe { *result_pos = ZERO_VEC3 };
        // SAFETY: under that same invariant the one-element advance remains in
        // the allocation (or reaches its one-past pointer for an empty mesh).
        result_pos = unsafe { result_pos.add(1) };

        let mut cached_position: bool = false;
        let mut cached_normals: bool = false;
        if load_caches && mesh.cache_deformers().count > 0 {
            // C: `ufbxi_for_ptr_list(ufbx_cache_deformer, p_cache, mesh->cache_deformers)`
            let cache_deformers = mesh.cache_deformers_view();
            for i_cache in 0..cache_deformers.count() {
                let p_cache = cache_deformers.at(i_cache);
                // C: `ufbx_cache_channel *channel = (*p_cache)->external_channel;
                // if (!channel) continue;` — the nullable ref reads as `None`.
                let Some(channel) = p_cache.external_channel() else {
                    continue;
                };
                let channel_view: &View<CacheChannel, Mut> = channel.view::<Mut>();

                if (channel_view.interpretation() == CacheInterpretation::VertexPosition
                    || channel_view.interpretation() == CacheInterpretation::Points)
                    && !cached_position
                {
                    // SAFETY: `channel` is a live scene-owned cache channel,
                    // `result_pos` addresses `num_vertices` writable `ufbx_vec3`
                    // slots of the result allocation, and `cache_opts` is
                    // derived from this fn's borrow of the caller's options —
                    // `sample_geometry_cache_vec3`'s contract.
                    let num_read: usize = unsafe {
                        sample_geometry_cache_vec3(
                            channel.ptr(),
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
                } else if channel_view.interpretation() == CacheInterpretation::VertexNormal
                    && !cached_normals
                {
                    // TODO: Is this right at all?
                    let num_normals: usize = mesh.skinned_normal().values().count;
                    let mut normal_data: *mut Vec3 =
                        buf_result.push::<Vec3>(num_normals.wrapping_add(1));
                    ufbxi_check_err!(error, !normal_data.is_null(), "normal_data");
                    // C: `normal_data[0] = ufbx_zero_vec3; normal_data++;`
                    // SAFETY: valid attribute counts satisfy the C allocation
                    // invariant `num_normals < SIZE_MAX`; the checked non-null
                    // result therefore has a writable sentinel slot.
                    unsafe { *normal_data = ZERO_VEC3 };
                    // SAFETY: under that same invariant this advance remains in
                    // the allocation or reaches its one-past pointer.
                    normal_data = unsafe { normal_data.add(1) };

                    // SAFETY: `channel` is a live scene-owned cache channel,
                    // `normal_data` addresses `num_normals` writable `ufbx_vec3`
                    // slots of the result allocation, and `cache_opts` is
                    // derived from this fn's borrow of the caller's options.
                    let num_read: usize = unsafe {
                        sample_geometry_cache_vec3(
                            channel.ptr(),
                            time,
                            normal_data,
                            num_normals,
                            cache_opts,
                        )
                    };
                    if num_read == num_normals {
                        cached_normals = true;
                        mesh.skinned_normal()
                            .values_view()
                            .set_data(normal_data as *const Vec3);
                    }
                }
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
            unsafe {
                ptr::copy_nonoverlapping(mesh.vertices_view().data(), result_pos, num_vertices)
            };

            // SAFETY: the freshly pushed `result_pos` run was initialized by
            // the copy above and stays allocated and writable for evaluation.
            // The capability forms no exclusive slice over arena memory.
            let result_vertices = unsafe { Run::<Vec3>::from_raw_parts(result_pos, num_vertices) };

            // C: `ufbxi_for_ptr_list(ufbx_blend_deformer, p_blend, mesh->blend_deformers)`
            let blend_deformers = mesh.blend_deformers_view();
            for i_blend in 0..blend_deformers.count() {
                let p_blend = blend_deformers.at(i_blend);
                add_blend_vertex_offsets_run(p_blend, Some(result_vertices), 1.0);
            }

            // TODO: What should we do about multiple skins??
            if mesh.skin_deformers().count > 0 {
                // C: `ufbx_matrix *fallback = mesh->instances.count > 0 ? &mesh->instances.data[0]->geometry_to_world : NULL;`
                // (`mesh->instances` reads through the anonymous `ufbx_element`
                // union member; the generated bindings spell it out.)
                let fallback: Option<&View<Matrix>> = if mesh.element().instances_view().count() > 0
                {
                    Some(view_project!(
                        mesh.element().instances_view().at(0),
                        geometry_to_world
                    ))
                } else {
                    None
                };
                let skin = mesh.skin_deformers_view().at(0);
                for i in 0..num_vertices {
                    // C: `ufbx_get_skin_vertex_matrix(skin, i, fallback)` — the
                    // `ufbx_inline` wrapper in ufbx.h (5601-5603) forwarding to
                    // the catch impl with a NULL panic.
                    // The optional view projects the first instance node's own
                    // matrix once before the loop; its bytes are read only if
                    // the skin vertex has no effective weight.
                    let mat: Matrix = catch_get_skin_vertex_matrix(None, skin, i, fallback);
                    // SAFETY: `i < num_vertices`, so `result_pos + i` is inside
                    // the pushed result allocation, readable and writable.
                    unsafe {
                        *result_pos.add(i) = transform_position(
                            View::<Matrix, Const>::from_ref(&mat),
                            *result_pos.add(i),
                        )
                    };
                }

                mesh.set_skinned_is_local(false);
            }
        }

        mesh.skinned_position()
            .values_view()
            .set_data(result_pos as *const Vec3);

        if !cached_normals {
            let num_indices: usize = mesh.num_indices();
            let normal_indices: *mut u32 = buf_result.push::<u32>(num_indices);
            ufbxi_check_err!(error, !normal_indices.is_null(), "normal_indices");

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

            let mut normal_data: *mut Vec3 = buf_result.push::<Vec3>(num_normals.wrapping_add(1));
            ufbxi_check_err!(error, !normal_data.is_null(), "normal_data");

            // C: `normal_data[0] = ufbx_zero_vec3; normal_data++;`
            // SAFETY: valid attribute counts satisfy the C allocation invariant
            // `num_normals < SIZE_MAX`; the checked non-null result therefore
            // has a writable sentinel slot.
            unsafe { *normal_data = ZERO_VEC3 };
            // SAFETY: under that same invariant this advance remains in the
            // allocation or reaches its one-past pointer.
            normal_data = unsafe { normal_data.add(1) };

            // SAFETY: `mesh.as_ptr()` and the `skinned_position` projection both
            // address the scene-owned mesh this view was minted over (read-only
            // use); `normal_indices` holds the `num_indices` indices
            // `generate_normal_mapping` wrote and `normal_data` addresses
            // `num_normals` writable `ufbx_vec3` slots. `compute_normals`
            // initializes that whole values run before both stable result-buffer
            // runs are captured in the returned descriptors.
            let (normal_values, normal_indices_list) = unsafe {
                compute_normals(
                    mesh.as_ptr(),
                    mesh.skinned_position().as_ptr(),
                    normal_indices,
                    num_indices,
                    normal_data,
                    num_normals,
                );
                (
                    List::from_raw_parts(normal_data, num_normals),
                    List::from_raw_parts(normal_indices, num_indices),
                )
            };

            mesh.set_generated_normals(true);
            mesh.skinned_normal().set_exists(true);
            mesh.skinned_normal().values_view().set(normal_values);
            mesh.skinned_normal()
                .indices_view()
                .set(normal_indices_list);
            mesh.skinned_normal().set_value_reals(3);
        }
    }

    Ok(())
}

// ufbx.c:25164-25168 `ufbxi_evaluate_skinning` (`#else` branch — feature
// disabled). C parity, NOT a stub: `ufbxi_report_err_msg` records the error
// and KEEPS GOING (PORTING.md trap #16); the `return 0` that follows carries
// the report's recording witness.
#[cfg(not(feature = "skinning-eval"))]
#[inline(never)]
pub(crate) fn evaluate_skinning(
    scene: &SceneView,
    error: &crate::native::error::ErrorView,
    buf_result: &BufView,
    buf_tmp: &BufView,
    time: f64,
    load_caches: bool,
    cache_opts: &RawGeometryCacheDataOpts,
) -> Result<(), Fail> {
    // C: all parameters other than `error` are unreferenced in the `#else` arm.
    let _ = (scene, buf_result, buf_tmp, time, load_caches, cache_opts);
    // SAFETY: the format string is a NUL-terminated literal with no
    // conversions.
    unsafe { ufbxi_fmt_err_info!(Some(error), "UFBX_ENABLE_SKINNING_EVALUATION") };
    Err(ufbxi_report_err_msg!(
        error,
        "UFBXI_FEATURE_SKINNING_EVALUATION",
        "Feature disabled"
    ))
}

// ufbx.c:25171-25185 `ufbxi_fixup_opts_string`
#[inline(never)]
pub(crate) fn fixup_opts_string(uc: &Context, str: &RawStringView, push: bool) -> Result<(), Fail> {
    if str.length() > 0 {
        if str.length() == usize::MAX {
            // C: `str->length = str->data ? strlen(str->data) : 0;`
            str.set_length(if !str.data().is_null() {
                // SAFETY: `str->data` is non-null (checked) and, with
                // `length == SIZE_MAX`, the options author declares it a
                // NUL-terminated C string.
                unsafe { strlen(str.data()) }
            } else {
                0
            });
        }
        if push {
            // SAFETY: `str.get()` addresses the live `ufbx_string` this view
            // borrows (`RawString` and `String` are the same `#[repr(C)]` pair
            // of `data`/`length` fields), which the pool rewrites in place.
            let str_ = unsafe { StringView::from_ptr(str.get() as *mut String) };
            push_string_place_str(uc.string_pool_view(), str_, false)?;
        }
    } else {
        str.set_data(EMPTY_CHAR.as_ptr());
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

// Hand view accessors for `ufbxi_load_imp`: `ufbxi_scene_imp` and
// `ufbxi_refcount` are internal `#[repr(C)]` structs with no generated `View`
// impl, so the header fields this function navigates get single-level leaf
// accessors here (the `View<CacheChannel>` precedent above). The four
// `ufbx_metadata` memory-accounting setters join them for the same reason:
// they are written only here, so the generated-side handle never grew them.
impl View<SceneImp, Mut> {
    #[inline(always)]
    pub(crate) fn refcount_view(&self) -> &View<Refcount, Mut> {
        view_project!(self, refcount)
    }
    #[inline(always)]
    pub(crate) fn refcount_mut_ptr(&self) -> *mut Refcount {
        view_raw_mut!(self, refcount)
    }
    #[inline(always)]
    pub(crate) fn scene_view(&self) -> &SceneView {
        view_project!(self, scene)
    }
    #[inline(always)]
    pub(crate) fn scene_mut_ptr(&self) -> *mut Scene {
        view_raw_mut!(self, scene)
    }
    #[inline(always)]
    pub(crate) fn set_magic(&self, magic: u32) {
        view_write!(self, magic, magic)
    }
    #[inline(always)]
    pub(crate) fn string_buf_view(&self) -> &BufView {
        view_project!(self, string_buf)
    }
    #[inline(always)]
    pub(crate) fn set_string_buf(&self, string_buf: Buf) {
        view_write!(self, string_buf, string_buf)
    }
}

impl View<Refcount, Mut> {
    #[inline(always)]
    pub(crate) fn ator_view(&self) -> &AllocatorView {
        view_project!(self, ator)
    }
    #[inline(always)]
    pub(crate) fn set_ator(&self, ator: Allocator) {
        view_write!(self, ator, ator)
    }
    #[inline(always)]
    pub(crate) fn buf_view(&self) -> &BufView {
        view_project!(self, buf)
    }
    #[inline(always)]
    pub(crate) fn set_buf(&self, buf: Buf) {
        view_write!(self, buf, buf)
    }
}

impl SceneMetadataView {
    #[inline(always)]
    pub(crate) fn set_result_memory_used(&self, result_memory_used: usize) {
        view_write!(self, result_memory_used, result_memory_used)
    }
    #[inline(always)]
    pub(crate) fn set_temp_memory_used(&self, temp_memory_used: usize) {
        view_write!(self, temp_memory_used, temp_memory_used)
    }
    #[inline(always)]
    pub(crate) fn set_result_allocs(&self, result_allocs: usize) {
        view_write!(self, result_allocs, result_allocs)
    }
    #[inline(always)]
    pub(crate) fn set_temp_allocs(&self, temp_allocs: usize) {
        view_write!(self, temp_allocs, temp_allocs)
    }
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
        return Err(Fail::unrecorded());
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
            ok = match unsafe {
                open_file_ctx(
                    &raw mut stream,
                    ctx,
                    filename,
                    filename_len,
                    &raw const opts,
                )
            } {
                Ok(()) => true,
                Err(e) => {
                    // C wrote the fixed error into the local slot; the
                    // `Result` shape hands it back by value.
                    error = e;
                    false
                }
            };
        } else {
            // SAFETY: the callback pointer is `uc`'s own field (live for the
            // borrow), `stream` is a live local, and
            // `uc.load_filename()`/`filename_len` describe the caller's filename
            // run — `open_file`'s contract.
            ok = unsafe {
                open_file::<Mut>(
                    uc.opts_view().open_file_cb_ptr(),
                    &raw mut stream,
                    uc.load_filename(),
                    filename_len,
                    None,
                    Some(uc.ator_tmp_view()),
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
                unsafe { ptr::copy_nonoverlapping(&raw const error, uc.error_mut_ptr(), 1) };
            } else {
                // SAFETY: `filename`/`filename_len` describe the caller's
                // filename run.
                unsafe { set_err_info(Some(uc.error_view()), filename, filename_len) };
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
        fixup_opts_string(uc, uc.opts_view().filename_view(), false).is_ok(),
        "ufbxi_fixup_opts_string(uc, &uc->opts.filename, false)"
    );
    ufbxi_check!(
        uc,
        fixup_opts_string(uc, uc.opts_view().obj_mtl_path_view(), true).is_ok(),
        "ufbxi_fixup_opts_string(uc, &uc->opts.obj_mtl_path, true)"
    );
    ufbxi_check!(
        uc,
        fixup_opts_string(
            uc,
            uc.opts_view().geometry_transform_helper_name_view(),
            true
        )
        .is_ok(),
        "ufbxi_fixup_opts_string(uc, &uc->opts.geometry_transform_helper_name, true)"
    );
    ufbxi_check!(
        uc,
        fixup_opts_string(uc, uc.opts_view().scale_helper_name_view(), true).is_ok(),
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
        .set(EMPTY_STRING.0);

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
        // `ufbx_dom_node` pushed into `uc`'s result buffer — a live,
        // write-capable arena allocation that outlives this frame.
        let dom_root_view: &View<DomNode, Mut> =
            unsafe { View::<DomNode, Mut>::from_ptr(dom_root) };
        dom_root_view.name_view().set(EMPTY_STRING.0);
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
    buf_free(uc.tmp_parse_view());

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
        ufbxi_check!(
            uc,
            evaluate_skinning(
                uc.scene_view(),
                uc.error_view(),
                uc.result_view(),
                uc.tmp_view(),
                0.0,
                uc.opts_view().load_external_files() && uc.opts_view().evaluate_caches(),
                &cache_opts,
            )
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

    // SAFETY: `imp` is the non-null (checked just above) `ufbxi_scene_imp`
    // pushed into `uc`'s result buffer — a live, write-capable arena
    // allocation that outlives this frame; the `Mut` view's `MaybeUninit`
    // storage tolerates the still-uninitialized push. The two addresses C
    // keeps alive PAST this frame (`&imp->refcount.ator`, stored into both
    // bufs, and `&imp->scene`, stored into every element) stay derived from
    // `imp` itself below, so they outlive this navigation handle.
    let imp_view: &View<SceneImp, Mut> = unsafe { View::<SceneImp, Mut>::from_ptr(imp) };

    // SAFETY: the projected pointer addresses `imp`'s own leading `Refcount`
    // field, live for the view above — `init_ref`'s raw-pointer contract; the
    // null parent is the "no parent to retain" case it handles.
    unsafe {
        init_ref(
            imp_view.refcount_mut_ptr(),
            SCENE_IMP_MAGIC,
            ptr::null_mut(),
        )
    };

    imp_view.set_magic(SCENE_IMP_MAGIC);
    // C: `imp->scene = uc->scene;` (struct copy)
    // SAFETY: the source is `uc`'s own scene field (live for the borrow) and the
    // destination is `imp`'s own scene field; the context and the freshly pushed
    // header are distinct allocations.
    unsafe { ptr::copy_nonoverlapping(uc.scene_mut_ptr(), imp_view.scene_mut_ptr(), 1) };
    imp_view.refcount_view().set_ator(uc.ator_result());
    // SAFETY: this retained allocator is used only to own/account and finally
    // free the transferred result buffers; no later path allocates or reports
    // an error through it, so its error sink may be null.
    unsafe {
        imp_view
            .refcount_view()
            .ator_view()
            .set_error(ptr::null_mut());
    }

    // Copy retained buffers and translate the allocator struct to the one
    // contained within `ufbxi_scene_imp`
    // C: `&imp->refcount.ator` — an address that outlives this frame (both bufs
    // free through it), so it keeps `imp`'s own whole-header provenance.
    // SAFETY: `refcount.ator` is a field of the live `ufbxi_scene_imp` above,
    // so the projection stays inside that allocation.
    let imp_ator: *mut Allocator = unsafe { &raw mut (*imp).refcount.ator };
    imp_view.refcount_view().set_buf(uc.take_result());
    // SAFETY: the populated result buffer was just moved into the imp, and
    // `imp_ator` points to the allocator state copied into the same stable imp
    // header above. The imp outlives the buffer, so ownership of every existing
    // chunk and its allocator state are translated together.
    unsafe { imp_view.refcount_view().buf_view().set_ator(imp_ator) };
    imp_view.set_string_buf(uc.string_pool_view().take_buf());
    // SAFETY: the populated string buffer was just moved into the same imp and
    // is retargeted to that identical retained allocator state before any
    // buffer operation. The allocator remains live until imp teardown.
    unsafe { imp_view.string_buf_view().set_ator(imp_ator) };

    let ator_tmp: &AllocatorView = uc.ator_tmp_view();
    imp_view
        .scene_view()
        .metadata_view()
        .set_result_memory_used(imp_view.refcount_view().ator_view().current_size());
    imp_view
        .scene_view()
        .metadata_view()
        .set_temp_memory_used(ator_tmp.current_size());
    imp_view
        .scene_view()
        .metadata_view()
        .set_result_allocs(imp_view.refcount_view().ator_view().num_allocs());
    imp_view
        .scene_view()
        .metadata_view()
        .set_temp_allocs(ator_tmp.num_allocs());

    // C: `&imp->scene` — stored into every element and read for the scene's
    // whole lifetime, so it too keeps `imp`'s own whole-header provenance.
    // SAFETY: `scene` is a field of the live `ufbxi_scene_imp` above.
    let imp_scene: *mut Scene = unsafe { &raw mut (*imp).scene };
    // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, imp->scene.elements)`
    let elements = imp_view.scene_view().elements_view();
    for i in 0..elements.count() {
        // C: `(*p_elem)->scene = &imp->scene;`
        // SAFETY: `imp_scene` addresses the scene header inside the live
        // result-buffer `ufbxi_scene_imp`, which outlives the scene itself.
        elements
            .at(i)
            .set_scene(unsafe { Ref::from_ptr(imp_scene) });
    }

    uc.set_scene_imp(imp);

    Ok(())
}

// ufbx.c:25412-25462 `ufbxi_free_temp`
#[inline(never)]
pub(crate) fn free_temp(uc: &Context) {
    // SAFETY: `thread_pool` is uc's own thread pool, valid by construction, and
    // this teardown is its last use (mirrors C `ufbxi_free_temp`).
    unsafe {
        thread_pool_free(uc.thread_pool_mut_ptr());
    }

    // SAFETY: single teardown — `free_temp` is reached exactly once per load
    // context, on the single teardown path of `load` after `load_imp` returns,
    // so this is the only release of `uc`'s string-pool `temp_str`.
    unsafe {
        string_pool_temp_free(uc.string_pool_view());
    }

    // SAFETY: every allocator and pointer/capacity pair released through the
    // raw `free` calls below is uc's own temp-side state, reached through uc's
    // accessors and valid by construction; this is the last use of all of it
    // (mirrors C `ufbxi_free_temp`). The buf and map teardowns interleaved
    // here in C order are safe calls that need no vouch.
    unsafe {
        buf_free(uc.warnings_view().tmp_stack_view());

        map_free(uc.prop_type_map_view());
        map_free(uc.fbx_id_map_view());
        map_free(uc.ptr_fbx_id_map_view());
        map_free(uc.texture_file_map_view());
        map_free(uc.anim_stack_map_view());
        map_free(uc.fbx_attr_map_view());
        map_free(uc.node_prop_set_view());
        map_free(uc.dom_node_map_view());

        buf_free(uc.tmp_view());
        buf_free(uc.tmp_parse_view());
        for i in 0..THREAD_GROUP_COUNT {
            buf_free(uc.tmp_thread_parse_at(i));
        }
        buf_free(uc.tmp_stack_view());
        buf_free(uc.tmp_connections_view());
        buf_free(uc.tmp_node_ids_view());
        buf_free(uc.tmp_elements_view());
        buf_free(uc.tmp_element_offsets_view());
        buf_free(uc.tmp_element_fbx_ids_view());
        buf_free(uc.tmp_element_ptrs_view());
        for i in 0..ELEMENT_TYPE_COUNT {
            buf_free(uc.tmp_typed_element_offsets_at(i));
        }
        buf_free(uc.tmp_mesh_textures_view());
        buf_free(uc.tmp_full_weights_view());
        buf_free(uc.tmp_dom_nodes_view());
        buf_free(uc.tmp_element_id_view());
        buf_free(uc.tmp_ascii_spans_view());

        free::<Node>(Some(uc.ator_tmp_view()), uc.top_nodes(), uc.top_nodes_cap());
        free::<*mut c_void>(
            Some(uc.ator_tmp_view()),
            uc.element_extra_arr(),
            uc.element_extra_cap(),
        );

        free::<u8>(
            Some(uc.ator_tmp_view()),
            uc.ascii_view().token_view().str_data(),
            uc.ascii_view().token_view().str_cap(),
        );
        free::<u8>(
            Some(uc.ator_tmp_view()),
            uc.ascii_view().prev_token_view().str_data(),
            uc.ascii_view().prev_token_view().str_cap(),
        );

        free::<u8>(
            Some(uc.ator_tmp_view()),
            uc.read_buffer(),
            uc.read_buffer_size(),
        );
        free::<u8>(Some(uc.ator_tmp_view()), uc.tmp_arr(), uc.tmp_arr_size());
        free::<u8>(Some(uc.ator_tmp_view()), uc.swap_arr(), uc.swap_arr_size());

        obj_free(uc);
    }

    // SAFETY: `uc.ator_tmp_view()` is the context's own temp allocator, live
    // for the borrow, torn down exactly once here.
    unsafe { free_ator(uc.ator_tmp_view()) };
}

// ufbx.c:25464-25470 `ufbxi_free_result`
#[inline(never)]
pub(crate) fn free_result(uc: &Context) {
    // The buffers come from `uc` accessors; mirrors C `ufbxi_free_result`'s
    // teardown.
    buf_free(uc.result_view());
    buf_free(uc.string_pool_view().buf_view());

    // SAFETY: `uc.ator_result_view()` is the context's own result allocator,
    // live for the borrow, torn down exactly once here.
    unsafe { free_ator(uc.ator_result_view()) };
}

// ufbx.c:25472-25625 `ufbxi_load`
#[inline(never)]
pub(crate) unsafe fn load(
    uc: &Context,
    user_opts: *const RawLoadOpts,
) -> Result<*mut Scene, Error> {
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

    init_ator(
        uc.error_mut_ptr(),
        uc.ator_tmp_view(),
        Some(uc.opts_view().temp_allocator_view()),
        c"temp",
    );
    init_ator(
        uc.error_mut_ptr(),
        uc.ator_result_view(),
        Some(uc.opts_view().result_allocator_view()),
        c"result",
    );

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

    // SAFETY: the pool and error sink are fields of the same live, unmoved
    // context; the error field remains write-capable for the pool's lifetime.
    unsafe { uc.string_pool_view().set_error(uc.error_mut_ptr()) };
    // SAFETY: the string map is still fresh and empty. `uc`'s initialized temp
    // allocator remains live, unmoved, and write-capable until `free_temp()`
    // releases this map through it; `map_cmp_string` reads no user data, so the
    // null `cmp_user` meets its contract.
    unsafe {
        map_init(
            uc.string_pool_view().map_view(),
            uc.ator_tmp_view(),
            map_cmp_string,
            ptr::null_mut(),
        );
    }
    // SAFETY: the empty string buffer is owned by `uc` and wired to its live,
    // initialized result allocator; the allocator outlives all pool use and
    // teardown, and all chunks are allocated through this stored pointer.
    unsafe {
        uc.string_pool_view()
            .buf_view()
            .set_ator(uc.ator_result_mut_ptr())
    };
    uc.string_pool_view().buf_view().set_unordered(true);
    uc.string_pool_view().set_initial_size(1024);
    uc.string_pool_view()
        .set_error_handling(uc.opts_view().unicode_error_handling());

    // SAFETY (every `map_init` below): these maps are still fresh and empty.
    // `uc`'s initialized temp allocator remains live, unmoved, and
    // write-capable until `free_temp()` releases every map through it. Each
    // comparator reads no user data, so the null `cmp_user` meets its contract.
    unsafe {
        map_init(
            uc.prop_type_map_view(),
            uc.ator_tmp_view(),
            map_cmp_const_char_ptr,
            ptr::null_mut(),
        );
        map_init(
            uc.fbx_id_map_view(),
            uc.ator_tmp_view(),
            map_cmp_uint64,
            ptr::null_mut(),
        );
        map_init(
            uc.ptr_fbx_id_map_view(),
            uc.ator_tmp_view(),
            map_cmp_ptr_id,
            ptr::null_mut(),
        );
        map_init(
            uc.texture_file_map_view(),
            uc.ator_tmp_view(),
            map_cmp_const_char_ptr,
            ptr::null_mut(),
        );
        map_init(
            uc.anim_stack_map_view(),
            uc.ator_tmp_view(),
            map_cmp_const_char_ptr,
            ptr::null_mut(),
        );
        map_init(
            uc.fbx_attr_map_view(),
            uc.ator_tmp_view(),
            map_cmp_uint64,
            ptr::null_mut(),
        );
        map_init(
            uc.node_prop_set_view(),
            uc.ator_tmp_view(),
            map_cmp_const_char_ptr,
            ptr::null_mut(),
        );
        map_init(
            uc.dom_node_map_view(),
            uc.ator_tmp_view(),
            map_cmp_uintptr,
            ptr::null_mut(),
        );
    }

    // SAFETY: every buffer below is an empty field of the live, unmoved load
    // context. Scratch buffers use the initialized temp allocator and result
    // uses the initialized result allocator; both sibling allocators outlive
    // all buffer operations and are torn down only after their buffers.
    unsafe {
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
    }

    uc.tmp_view().set_unordered(true);
    uc.tmp_parse_view().set_unordered(true);
    uc.tmp_parse_view().set_clearable(true);
    uc.result_view().set_unordered(true);

    // SAFETY: the warning state, error sink, and result buffer are fields of
    // the same live, unmoved context and remain write-capable while warnings
    // are collected.
    unsafe {
        uc.warnings_view().set_error(uc.error_mut_ptr());
        uc.warnings_view().set_result(uc.result_mut_ptr());
    }
    // SAFETY: the empty warning stack is owned by the same context and uses
    // its live temp allocator until warning teardown.
    unsafe {
        uc.warnings_view()
            .tmp_stack_view()
            .set_ator(uc.ator_tmp_mut_ptr())
    };
    // SAFETY: the context-owned warnings sink stays live, unmoved, and
    // write-capable for every later use of the context-owned string pool.
    unsafe { uc.string_pool_view().set_warnings(uc.warnings_mut_ptr()) };

    // Set zero size `swap_arr` to a non-NULL buffer so we can tell the difference between empty
    // array and an allocation failure.
    // C: `uc->swap_arr = (char*)ufbxi_zero_size_buffer;` — the const cast is
    // C-parity: the buffer is replaced by `ufbxi_grow_array` before any write.
    uc.set_swap_arr(ZERO_SIZE_BUFFER.as_ptr() as *mut u8);

    // NOTE: Though `inflate_retain` leaks out of the scope we don't use it outside this function.
    // cppcheck-suppress autoVariables
    // SAFETY: `inflate_retain` is an allocated, aligned stack place reached
    // through its write-capable `MaybeUninit::as_mut_ptr`; its `initialized`
    // leaf was written to the valid value `false` above. The local is not moved
    // or dropped until after `load_imp` returns and `free_temp` has joined/freed
    // the thread pool, so synchronous inflation and every deferred task that
    // copies the pointer finish while the storage is live and unmoved.
    unsafe { uc.set_inflate_retain(inflate_retain.as_mut_ptr()) };

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
        // SAFETY: `ok` means `load_imp` succeeded, and its last act is storing
        // the retained `ufbxi_scene_imp` into `uc`, so `scene_imp()` is the live
        // result-buffer header whose own `scene` field is projected here. (The
        // success-path `clear_error` of the caller's slot lives in the boundary
        // shim — PORTING.md "Trailing `ufbx_error *error`".)
        Ok(unsafe { &raw mut (*uc.scene_imp()).scene })
    } else {
        // C copies the fixed error into the caller's slot; the `Result` shape
        // carries it by value (the shim owns the slot writes; C's
        // unsupported-version rewrite ran only with a non-null slot, but the
        // rewrite is observable only through the slot, so applying it to the
        // carried value is byte-equivalent).
        let mut fixed: Error = Error::default();
        let fixed_view = crate::native::error::ErrorView::from_mut(&mut fixed);
        fix_error_type(uc.error_view(), b"Failed to load\0", Some(fixed_view));
        if fixed.type_ == ErrorType::Unknown
            && uc.scene_view().metadata_view().file_format() == FileFormat::Fbx
            && !supports_version(uc.version())
        {
            fixed.description = crate::prelude::String::new_c(
                b"Unsupported version\0".as_ptr(),
                b"Unsupported version".len(),
            );
            fixed.type_ = ErrorType::UnsupportedVersion;
            // SAFETY: the single `%u` conversion is matched by the `u32`
            // argument.
            unsafe {
                ufbxi_fmt_err_info!(
                    Some(crate::native::error::ErrorView::from_mut(&mut fixed)),
                    "%u",
                    uc.version()
                )
            };
        }
        free_result(uc);
        Err(fixed)
    }
}

// -- Animation evaluation (ufbx.c:25627)

// ufbx.c:25629-25634 `ufbxi_override_less_than_prop`
// C: `ufbxi_forceinline`.
#[inline(always)]
pub(crate) fn override_less_than_prop<MO: Mode, MP: Mode>(
    over: &View<PropOverride, MO>,
    element_id: u32,
    prop: &View<Prop, MP>,
) -> bool {
    if over.element_id() != element_id {
        return over.element_id() < element_id;
    }
    if over._internal_key() != prop._internal_key() {
        return over._internal_key() < prop._internal_key();
    }
    // C: `return strcmp(over->prop_name.data, prop->name.data);` — the `int`
    // result converts to `bool` (nonzero == true), so ANY name difference
    // reports "less".
    // PORT DIVERGENCE (ufbx.c:25633): upstream `strcmp` scans `prop.name` to a
    // NUL, but on the NOT_FOUND path of `evaluate_prop_flags_len` it is the
    // caller's raw `_len` name and need not be NUL-terminated (over-read on an
    // element_id + `_internal_key` collision). `str_cmp` reads only `min(len)`
    // bytes with the same ordering for NUL-terminated names; reconcile on sync.
    sp::str_cmp(over.prop_name_view().bytes(), prop.name_view().bytes()) != 0
}

// ufbx.c:25636-25641 `ufbxi_override_equals_to_prop`
// C: `ufbxi_forceinline`.
#[inline(always)]
pub(crate) fn override_equals_to_prop<MO: Mode, MP: Mode>(
    over: &View<PropOverride, MO>,
    element_id: u32,
    prop: &View<Prop, MP>,
) -> bool {
    if over.element_id() != element_id {
        return false;
    }
    if over._internal_key() != prop._internal_key() {
        return false;
    }
    // PORT DIVERGENCE (ufbx.c:25640): as in `override_less_than_prop` — the
    // upstream `strcmp` over-reads a non-NUL `_len` `prop.name`; use the
    // length-bounded `str_cmp` instead; reconcile once upstream lands the fix.
    sp::str_cmp(over.prop_name_view().bytes(), prop.name_view().bytes()) == 0
}

// ufbx.c:25643-25664 `ufbxi_find_prop_override`
#[inline(never)]
pub(crate) fn find_prop_override(
    overrides: &View<List<PropOverride>, Const>,
    element_id: u32,
    prop: &View<Prop, Mut>,
) -> bool {
    let ix = overrides.lower_bound_eq(
        16,
        |a| override_less_than_prop(a, element_id, prop),
        |a| override_equals_to_prop(a, element_id, prop),
    );

    if let Some(ix) = ix {
        let over = overrides.at(ix);
        // C: `const uint32_t clear_flags = UFBX_PROP_FLAG_NO_VALUE | UFBX_PROP_FLAG_NOT_FOUND;`
        let clear_flags: u32 = PropFlags::NO_VALUE.raw() | PropFlags::NOT_FOUND.raw();
        prop.set_flags(PropFlags::from_raw(
            (prop.flags().raw() & !clear_flags) | PropFlags::OVERRIDDEN.raw(),
        ));
        // C: `prop->value_vec4 = over->value;` then `prop->value_real_arr[3] = 0.0f;`
        // — the union's four-real view; its trailing lane is `value_vec4.w`.
        let mut value = over.value();
        value.w = 0.0;
        prop.set_value_vec4(value);
        prop.set_value_int(over.value_int());
        prop.set_value_str(over.value_str());
        // C: `prop->value_blob.data = prop->value_str.data;` + `.size = ....length;`
        let mut blob = prop.value_blob();
        blob.data = prop.value_str().data;
        blob.size = prop.value_str().length;
        prop.set_value_blob(blob);
        true
    } else {
        false
    }
}

// ufbx.c:25666-25679 `ufbxi_find_element_prop_overrides`
#[inline(never)]
pub(crate) fn find_element_prop_overrides(
    overrides: &View<List<PropOverride>, Const>,
    element_id: u32,
) -> List<PropOverride> {
    // C: `size_t begin = overrides->count, end = begin;` — the lower bound does
    // not write on a miss; `unwrap_or` reproduces the pre-init.
    let begin: usize = overrides
        .lower_bound_eq(
            32,
            |a| a.element_id() < element_id,
            |a| a.element_id() == element_id,
        )
        .unwrap_or(overrides.count());
    let end: usize = overrides.upper_bound_eq(32, begin, |a| a.element_id() == element_id);

    // C: `ufbx_prop_override_list result = { overrides->data + begin, end - begin };`
    // (`List<T>` carries a private `PhantomData` marker, so the aggregate
    // initializer becomes a zeroed value with both public fields written.)
    // SAFETY: `ufbx_prop_override_list` is a pointer plus a count (plus a
    // zero-sized marker), for which the all-zero pattern is a valid inhabitant.
    let mut result: List<PropOverride> = unsafe { MaybeUninit::zeroed().assume_init() };
    // C writes `data + begin` even for an empty range (`begin <= count`, so it
    // is at most one past the end); the wrapping projection keeps the address
    // without an in-bounds dereference claim.
    result.data = overrides.data().wrapping_add(begin);
    result.count = end - begin;
    result
}

// ufbx.c:25681-25687 `ufbxi_anim_layer_combine_ctx`
// C's `const ufbx_anim *anim` / `const ufbx_element *element` are read-only
// borrows of the objects being evaluated, so the ctx carries them as frozen
// `Const` views: the liveness contract is discharged once, at the view mint in
// `evaluate_props`, instead of riding untyped inside raw pointers that any safe
// construction site could fill with garbage — which is what lets
// `combine_anim_layer` be an honest safe fn.
#[repr(C)]
pub(crate) struct AnimLayerCombineCtx<'a> {
    pub anim: &'a View<Anim, Const>,
    pub element: &'a View<Element, Const>,
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

// `ufbx_vec3` in/out slot: `ufbxi_combine_anim_layer`'s `ufbx_vec3 *result`
// addresses the `ufbx_prop` value union inside the caller's prop run, which is
// arena memory, so its components are read and written one at a time through a
// view instead of a `&mut`.
impl<M: Mode> View<Vec3, M> {
    #[inline(always)]
    pub(crate) fn x(&self) -> Real {
        view_read_shared!(self, x)
    }
    #[inline(always)]
    pub(crate) fn y(&self) -> Real {
        view_read_shared!(self, y)
    }
    #[inline(always)]
    pub(crate) fn z(&self) -> Real {
        view_read_shared!(self, z)
    }
    // C reads the slot as a whole struct (`ufbx_euler_to_quat(*result, ...)`);
    // `ufbx_vec3` is three `ufbx_real`s with no padding, so the field-wise copy
    // moves the same bytes.
    #[inline(always)]
    pub(crate) fn vec3(&self) -> Vec3 {
        Vec3 {
            x: self.x(),
            y: self.y(),
            z: self.z(),
        }
    }
}

impl View<Vec3, Mut> {
    #[inline(always)]
    pub(crate) fn set_x(&self, value: Real) {
        view_write!(self, x, value)
    }
    #[inline(always)]
    pub(crate) fn set_y(&self, value: Real) {
        view_write!(self, y, value)
    }
    #[inline(always)]
    pub(crate) fn set_z(&self, value: Real) {
        view_write!(self, z, value)
    }
    // C assigns the slot as a whole struct (`*result = *value;`); field-wise
    // for the same reason as `vec3()`.
    #[inline(always)]
    pub(crate) fn set_vec3(&self, value: Vec3) {
        self.set_x(value.x);
        self.set_y(value.y);
        self.set_z(value.z);
    }
}

// Recursion is limited by the fact that we recurse only when the property name is "Lcl Rotation"
// and when recursing we always evaluate the property "RotationOrder"
// ufbx.c:25697-25749 `ufbxi_combine_anim_layer`
// `ufbxi_recursive_function_void(ufbxi_combine_anim_layer, ..., 2, ...)`
// (ufbx.c:25699-25700): under regression a thread-local depth guard wraps the
// recursive body; otherwise the macro is empty and the wrapper is a plain call.
// C's `const char *prop_name` is a string-pool pointer compared for interned
// identity only and never dereferenced, so it carries no contract and stays a
// bare `*const u8`.
#[inline(never)]
pub(crate) fn combine_anim_layer(
    ctx: &mut AnimLayerCombineCtx<'_>,
    layer: &View<AnimLayer>,
    weight: Real,
    prop_name: *const u8,
    result: &View<Vec3, Mut>,
    value: &Vec3,
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
    combine_anim_layer_rec(ctx, layer, weight, prop_name, result, value);
}

// ufbx.c:25702-25749 `ufbxi_combine_anim_layer` body (the `_rec` half of the
// `ufbxi_recursive_function` body; see the wrapper above)
#[inline(never)]
fn combine_anim_layer_rec(
    ctx: &mut AnimLayerCombineCtx<'_>,
    layer: &View<AnimLayer>,
    weight: Real,
    prop_name: *const u8,
    result: &View<Vec3, Mut>,
    value: &Vec3,
) {
    if layer.compose_rotation()
        && layer.blended()
        && prop_name == sp::Lcl_Rotation.as_ptr()
        && !ctx.has_rotation_order
    {
        let rp: Prop = evaluate_prop_len_view(
            ctx.anim,
            ctx.element,
            EvalPropName::from_slice(&sp::RotationOrder[..sp::RotationOrder.len() - 1]),
            ctx.time,
        );
        // NOTE: Defaults to 0 (UFBX_ROTATION_XYZ) gracefully if property is not found
        if rp.value_int >= 0 && rp.value_int <= RotationOrder::Spheric as i64 {
            // C: `(ufbx_rotation_order)rp.value_int` — in-range by the guard.
            // SAFETY: the guard bounds `value_int` to `0..=Spheric`, which are
            // exactly the discriminants of the `repr(u32)` `ufbx_rotation_order`,
            // so the transmuted value is a valid variant.
            ctx.rotation_order =
                unsafe { core::mem::transmute::<u32, RotationOrder>(rp.value_int as u32) };
        } else {
            ctx.rotation_order = RotationOrder::Xyz;
        }
        ctx.has_rotation_order = true;
    }

    if layer.additive() {
        if layer.compose_scale() && prop_name == sp::Lcl_Scaling.as_ptr() {
            // C: `result->x *= (ufbx_real)ufbxi_pow_abs(value->x, weight);`
            // — `ufbxi_pow_abs` takes `double`, so both args promote to double
            // and the result narrows back to `ufbx_real` before the multiply.
            result.set_x(result.x() * pow_abs(as_f64!(value.x), as_f64!(weight)) as Real);
            result.set_y(result.y() * pow_abs(as_f64!(value.y), as_f64!(weight)) as Real);
            result.set_z(result.z() * pow_abs(as_f64!(value.z), as_f64!(weight)) as Real);
        } else if layer.compose_rotation() && prop_name == sp::Lcl_Rotation.as_ptr() {
            let a: Quat = euler_to_quat(result.vec3(), ctx.rotation_order);
            let mut b: Quat = euler_to_quat(*value, ctx.rotation_order);
            b = quat_slerp(IDENTITY_QUAT, b, weight);
            let res: Quat = mul_quat(a, b);
            result.set_vec3(quat_to_euler(res, ctx.rotation_order));
        } else {
            result.set_x(result.x() + value.x * weight);
            result.set_y(result.y() + value.y * weight);
            result.set_z(result.z() + value.z * weight);
        }
    } else if layer.blended() {
        // C: `ufbx_real res_weight = 1.0f - weight;`
        let res_weight: Real = 1.0 - weight;
        if layer.compose_scale() && prop_name == sp::Lcl_Scaling.as_ptr() {
            // C: `result->x = (ufbx_real)(ufbxi_pow_abs(result->x, res_weight) * ufbxi_pow_abs(value->x, weight));`
            // — `ufbxi_pow_abs` takes `double`; the product stays in double and
            // narrows to `ufbx_real` only on the assignment.
            result.set_x(
                (pow_abs(as_f64!(result.x()), res_weight as f64)
                    * pow_abs(as_f64!(value.x), as_f64!(weight))) as Real,
            );
            result.set_y(
                (pow_abs(as_f64!(result.y()), res_weight as f64)
                    * pow_abs(as_f64!(value.y), as_f64!(weight))) as Real,
            );
            result.set_z(
                (pow_abs(as_f64!(result.z()), res_weight as f64)
                    * pow_abs(as_f64!(value.z), as_f64!(weight))) as Real,
            );
        } else if layer.compose_rotation() && prop_name == sp::Lcl_Rotation.as_ptr() {
            let a: Quat = euler_to_quat(result.vec3(), ctx.rotation_order);
            let b: Quat = euler_to_quat(*value, ctx.rotation_order);
            let res: Quat = quat_slerp(a, b, weight);
            result.set_vec3(quat_to_euler(res, ctx.rotation_order));
        } else {
            result.set_x(result.x() * res_weight + value.x * weight);
            result.set_y(result.y() * res_weight + value.y * weight);
            result.set_z(result.z() * res_weight + value.z * weight);
        }
    } else {
        result.set_vec3(*value);
    }
}

// ufbx.c:25751-25757 `ufbxi_anim_layer_might_contain_id`
// C: `ufbxi_forceinline`.
#[inline(always)]
pub(crate) fn anim_layer_might_contain_id<M: Mode>(layer: &View<AnimLayer, M>, id: u32) -> bool {
    let element_id_bitmask = layer._element_id_bitmask();
    // C: `uint32_t id_mask = ufbxi_arraycount(layer->_element_id_bitmask) - 1;`
    let id_mask: u32 = (element_id_bitmask.len() - 1) as u32;
    // C: `bool ok = id - layer->_min_element_id <= (layer->_max_element_id - layer->_min_element_id);`
    // — unsigned wrapping subtraction.
    let mut ok: bool = id.wrapping_sub(layer._min_element_id())
        <= layer
            ._max_element_id()
            .wrapping_sub(layer._min_element_id());
    // `id_mask` is the bitmask array's length minus one and that length is a
    // power of two, so `(id >> 5) & id_mask` indexes it.
    ok &= (element_id_bitmask[((id >> 5) & id_mask) as usize] & (1u32 << (id & 31))) != 0;
    ok
}

// Sentinel-tolerant read of `ufbx_anim_prop.element`: `layer->anim_props` is
// terminated by a zeroed sentinel entry (ufbx.c:22257, `num_anim_props + 1`)
// whose `element` slot is NULL, so the walk in `evaluate_props` reads the field
// as bare pointer bits instead of through the non-null `Ref<Element>`.
impl<M: Mode> View<AnimProp, M> {
    #[inline(always)]
    pub(crate) fn element_target_ptr(&self) -> *mut Element {
        // SAFETY: `element_ptr()` addresses the viewed prop's own `element`
        // field, which is `repr(transparent)` over `NonNull<Element>`, so
        // reading it as `*mut Element` reinterprets the same bytes in place and
        // tolerates the run's NULL sentinel.
        unsafe { ref_ptr(self.element_ptr()) }
    }
}

// ufbx.c:25759-25818 `ufbxi_evaluate_props`
#[inline(never)]
pub(crate) unsafe fn evaluate_props(
    anim: *const Anim,
    element: *const Element,
    time: f64,
    props: Run<'_, Prop>,
    flags: u32,
) {
    // SAFETY: `anim` is the caller's live `ufbx_anim` — the raw-pointer contract
    // of this `unsafe fn` — and evaluation only reads it, so the frozen `Const`
    // tag stays valid for the whole body.
    let anim_view: &View<Anim, Const> = unsafe { View::<Anim, Const>::from_ptr(anim) };

    // SAFETY: `element` is the caller's live `ufbx_element` (the same
    // raw-pointer contract); evaluation reads the element and writes only the
    // caller's separate `props` run, so the frozen `Const` tag stays valid for
    // the whole body.
    let element_view: &View<Element, Const> = unsafe { View::<Element, Const>::from_ptr(element) };

    // C: `ufbxi_anim_layer_combine_ctx combine_ctx = { anim, element, time };`
    let mut combine_ctx = AnimLayerCombineCtx {
        anim: anim_view,
        element: element_view,
        time,
        rotation_order: RotationOrder::Xyz,
        has_rotation_order: false,
    };

    let element_id: u32 = element_view.element_id();
    let num_layers: usize = anim_view.layers_view().count();
    for layer_ix in 0..num_layers {
        // C: `ufbx_anim_layer *layer = anim->layers.data[layer_ix];` — loaded
        // as the STORED pointer out of the ref list (never re-derived from a
        // read-only view), so it keeps the scene's write-capable provenance
        // that the callees' non-const `ufbx_anim_layer *` parameters
        // (ufbx.c:19329, ufbx.c:25699) are entitled to.
        // SAFETY: `layer_ix < num_layers` is the anim's own layer count, so the
        // slot is inside the list's `[data, data + count)` run; the non-null
        // `Ref<AnimLayer>` is read as bare pointer bits and names a live
        // scene-owned layer.
        let layer: *mut AnimLayer =
            unsafe { *(anim_view.layers_view().data() as *const *mut AnimLayer).add(layer_ix) };
        // SAFETY: `layer` is that live, unmoved scene-owned `ufbx_anim_layer`,
        // reached through the stored `*mut` — write-capable provenance, the
        // `Mut` mint's contract — and no `&mut` to it is ever formed.
        let layer_view: &View<AnimLayer> = unsafe { View::<AnimLayer>::from_ptr(layer) };
        if !anim_layer_might_contain_id(layer_view, element_id) {
            continue;
        }

        // Find the weight for the current layer
        // TODO: Should this be searched from multiple layers?
        let mut weight: Real = if layer_ix < anim_view.override_layer_weights_view().count() {
            // SAFETY: the branch condition bounds `layer_ix` inside the anim's
            // override-weight run, and the offset is taken from that list's own
            // base pointer, so it stays inside the run.
            unsafe { *anim_view.override_layer_weights_view().data().add(layer_ix) }
        } else {
            layer_view.weight()
        };
        if layer_view.weight_is_animated() && layer_view.blended() {
            // C: `ufbx_anim_prop *weight_aprop = ufbxi_find_anim_prop_start(layer, &layer->element);`
            let weight_aprop: *mut AnimProp =
                find_anim_prop_start(layer_view, layer_view.element());
            if !weight_aprop.is_null() {
                // SAFETY: `weight_aprop` is a non-null (checked above) anim prop
                // of `layer` — live and unwritten during evaluation, the `Const`
                // view mint's contract.
                let weight_aprop_view: &View<AnimProp, Const> =
                    unsafe { View::<AnimProp, Const>::from_ptr(weight_aprop) };
                // C: `weight = ufbx_evaluate_anim_value_real_flags(...) / (ufbx_real)100.0;`
                weight = evaluate_anim_value_real_flags(
                    Some(weight_aprop_view.anim_value().view::<Const>()),
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

        let mut aprop: *mut AnimProp = find_anim_prop_start(layer_view, element_view);
        if aprop.is_null() {
            continue;
        }

        // C: `for (size_t i = 0; i < num_props; i++) { ufbx_prop *prop = &props[i]; ... }`
        for prop in props.iter() {
            // Don't evaluate on top of overridden properties
            if (prop.flags().raw() & PropFlags::OVERRIDDEN.raw()) != 0 {
                continue;
            }

            // Connections override animation by default
            if (prop.flags().raw() & PropFlags::CONNECTED.raw()) != 0
                && !anim_view.ignore_connections()
            {
                continue;
            }

            // Skip until we reach `aprop >= prop`
            // NOTE: No need to check for end as `anim_props` is terminated with a NULL sentinel.
            // SAFETY: `aprop` is inside `layer`'s sorted anim-prop run — live
            // and unwritten during evaluation, the `Const` view mint's contract.
            let mut aprop_view: &View<AnimProp, Const> =
                unsafe { View::<AnimProp, Const>::from_ptr(aprop) };
            while std::ptr::eq(aprop_view.element_target_ptr(), element)
                && aprop_view._internal_key() < prop._internal_key()
            {
                // SAFETY: the condition just matched a non-sentinel entry of
                // the run, so the next slot is still inside it; `aprop` carries
                // the anim-prop run's own provenance.
                aprop = unsafe { aprop.add(1) };
                // SAFETY: as above — `aprop` addresses an entry of `layer`'s
                // anim-prop run, live and unwritten during evaluation.
                aprop_view = unsafe { View::<AnimProp, Const>::from_ptr(aprop) };
            }
            if aprop_view.prop_name_view().data() != prop.name().data {
                while std::ptr::eq(aprop_view.element_target_ptr(), element)
                    // SAFETY: both names are string-pool `ufbx_string`s, which
                    // are stored NUL-terminated.
                    && unsafe { strcmp(aprop_view.prop_name_view().data(), prop.name().data) } < 0
                {
                    // SAFETY: the condition just matched a non-sentinel entry of
                    // the run, so the next slot is still inside it; `aprop`
                    // carries the anim-prop run's own provenance.
                    aprop = unsafe { aprop.add(1) };
                    // SAFETY: as above — `aprop` addresses an entry of `layer`'s
                    // anim-prop run, live and unwritten during evaluation.
                    aprop_view = unsafe { View::<AnimProp, Const>::from_ptr(aprop) };
                }
            }

            // TODO: Should we skip the blending for the first layer _per property_
            // This could be done by having `UFBX_PROP_FLAG_ANIMATION_EVALUATED`
            // that gets set for the first layer of animation that is applied.
            if aprop_view.prop_name_view().data() == prop.name().data {
                let v: Vec3 = evaluate_anim_value_vec3_flags(
                    Some(aprop_view.anim_value().view::<Const>()),
                    time,
                    flags,
                );
                if layer_ix == 0 {
                    // C: `prop->value_vec3 = v;` — the `ufbx_prop` value
                    // union's 3-real view over `value_vec4`.
                    // SAFETY: `value_vec4` is four `ufbx_real`s of the viewed
                    // prop, so writing the union's leading `ufbx_vec3` view
                    // stays inside it.
                    unsafe { *(prop.value_vec4_raw() as *mut Vec3) = v };
                } else {
                    // C: the in/out slot is `prop`'s own `value_vec4` union
                    // viewed as a `ufbx_vec3`.
                    // SAFETY: `value_vec4` is four `ufbx_real`s of the viewed
                    // prop, so the leading `ufbx_vec3` stays inside it; the
                    // prop is reached through the caller's `*mut` prop run,
                    // which is the write-capable provenance the `Mut` mint
                    // requires.
                    let result: &View<Vec3, Mut> =
                        unsafe { View::<Vec3, Mut>::from_ptr(prop.value_vec4_raw() as *mut Vec3) };
                    combine_anim_layer(
                        &mut combine_ctx,
                        layer_view,
                        weight,
                        prop.name().data,
                        result,
                        &v,
                    );
                }
            }
        }
    }

    // C: `ufbxi_for(ufbx_prop, prop, props, num_props)`
    for prop in props.iter() {
        if (prop.flags().raw() & PropFlags::OVERRIDDEN.raw()) != 0 {
            continue;
        }
        // C: `prop->value_int = ufbxi_f64_to_i64(prop->value_real);` — the
        // value union's first real is `value_vec4.x`; `ufbxi_f64_to_i64` takes
        // `double`, so the `ufbx_real` argument promotes.
        prop.set_value_int(f64_to_i64(as_f64!(prop.value_vec4().x)));
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
    // SAFETY (every mint below): `prop` is the caller's live `ufbx_prop`, reached
    // through `*mut` (write-capable provenance for `Mut`); `anim`/`element` are
    // the caller's live anim and element, read-only for the whole call (nothing
    // below writes either), so they anchor frozen `Const` views. All three are
    // this `unsafe fn`'s own raw-pointer contract. `name` is measured once
    // below under this function's NUL-terminated-string contract.
    let prop: &View<Prop, Mut> = unsafe { View::<Prop, Mut>::from_ptr(prop) };
    let anim: &View<Anim, Const> = unsafe { View::<Anim, Const>::from_ptr(anim) };
    let element: &View<Element, Const> = unsafe { View::<Element, Const>::from_ptr(element) };
    // SAFETY: a non-null `name` is NUL-terminated by this unsafe function's
    // caller contract. Forming the span directly preserves its base address
    // for the connection lookup's interned-pointer identity check.
    let name: ConnectionPropKey<'_> = if name.is_null() {
        ConnectionPropKey::from_option(None)
    } else {
        ConnectionPropKey::from_option(Some(unsafe {
            core::slice::from_raw_parts(name, strlen(name))
        }))
    };

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
fn evaluate_connected_prop_rec(
    prop: &View<Prop, Mut>,
    anim: &View<Anim, Const>,
    element: &View<Element, Const>,
    name: ConnectionPropKey<'_>,
    time: f64,
    flags: u32,
) {
    let mut conn: Option<&View<Connection, Const>> = find_prop_connection(element, name);

    // C: `for (size_t i = 0; i < 1000 && conn; i++)`
    let mut i: usize = 0;
    while i < 1000 && conn.is_some() {
        let current = conn.expect("connection checked by loop condition");
        let next_conn = find_prop_connection(current.src_view(), connection_src_prop_key(current));
        if next_conn.is_none() {
            break;
        }
        conn = next_conn;
        i += 1;
    }

    // Found a non-cyclic connection
    let terminal = match conn {
        Some(conn) => {
            find_prop_connection(conn.src_view(), connection_src_prop_key(conn)).is_none()
        }
        None => false,
    };
    if terminal {
        let conn = conn.expect("terminal connection checked above");
        let ep: Prop = evaluate_prop_flags_len_view(
            anim,
            conn.src_view(),
            EvalPropName::from_string(conn.src_prop_view()),
            time,
            flags,
        );
        prop.set_value_vec4(ep.value_vec4);
        prop.set_value_int(ep.value_int);
        prop.set_value_str(ep.value_str);
        prop.set_value_blob(ep.value_blob);
    } else {
        // Connection not found, maybe it's animated?
        prop.set_flags(PropFlags::from_raw(
            prop.flags().raw() & !PropFlags::CONNECTED.raw(),
        ));
    }
}

#[inline(always)]
fn connection_src_prop_key<'a>(conn: &'a View<Connection, Const>) -> ConnectionPropKey<'a> {
    let src_prop = conn.src_prop_view();
    if src_prop.data().is_null() {
        ConnectionPropKey::from_option(None)
    } else {
        ConnectionPropKey::from_string(src_prop)
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

impl View<PropIter, Mut> {
    #[inline(always)]
    pub(crate) fn prop(&self) -> *const Prop {
        view_read!(self, prop)
    }
    #[inline(always)]
    pub(crate) fn set_prop(&self, prop: *const Prop) {
        view_write!(self, prop, prop)
    }
    #[inline(always)]
    pub(crate) fn prop_end(&self) -> *const Prop {
        view_read!(self, prop_end)
    }
    #[inline(always)]
    pub(crate) fn over(&self) -> *const PropOverride {
        view_read!(self, over)
    }
    #[inline(always)]
    pub(crate) fn set_over(&self, over: *const PropOverride) {
        view_write!(self, over, over)
    }
    #[inline(always)]
    pub(crate) fn over_end(&self) -> *const PropOverride {
        view_read!(self, over_end)
    }
    #[inline(always)]
    pub(crate) fn tmp_view(&self) -> &View<Prop, Mut> {
        view_project!(self, tmp)
    }
}

// ufbx.c:25853-25864 `ufbxi_init_prop_iter_slow`
#[inline(never)]
pub(crate) unsafe fn init_prop_iter_slow(
    iter: *mut PropIter,
    anim: &View<Anim, Const>,
    element: &View<Element, Const>,
) {
    // C: `iter->prop_end = element->props.props.data + element->props.props.count;`
    // SAFETY (every `*iter` access in this fn): `iter` is the caller's live
    // `PropIter` storage — the raw-pointer contract of this `unsafe fn`.
    // `data`/`count` describe the element's own prop run, so the end pointer is
    // one past its last element.
    unsafe {
        (*iter).prop = element.props().props().data;
        (*iter).prop_end = element
            .props()
            .props()
            .data
            .add(element.props().props().count);
    }

    let over: List<PropOverride> =
        find_element_prop_overrides(anim.prop_overrides_view(), element.element_id());
    // SAFETY: as above for `iter`; `over` is a sub-run of the anim's override
    // list, so `data + count` is one past its last element.
    unsafe {
        (*iter).over = over.data;
        (*iter).over_end = over.data.add(over.count);
    }
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
    anim: &View<Anim, Const>,
    element: &View<Element, Const>,
) {
    // C: `iter->over = iter->over_end = NULL;`
    // SAFETY (every `*iter` access in this fn): `iter` is the caller's live
    // `PropIter` storage — the raw-pointer contract of this `unsafe fn`.
    unsafe {
        (*iter).prop = element.props().props().data;
        (*iter).prop_end = add_ptr(
            element.props().props().data as *mut Prop,
            element.props().props().count,
        );
        (*iter).over = ptr::null();
        (*iter).over_end = ptr::null();
    }
    if anim.prop_overrides_view().count() > 0 {
        // SAFETY: `iter` is forwarded unchanged from this fn's own parameter, so
        // the callee inherits the caller's contract.
        unsafe { init_prop_iter_slow(iter, anim, element) };
    }
}

// ufbx.c:25876-25914 `ufbxi_next_prop_slow`
///
/// # Safety
/// `iter`'s cursors must be as `init_prop_iter` left them: its `prop`/`over`
/// cursors sit inside the element's prop run and the anim's override run and
/// stop at the matching `*_end`. This fn dereferences those raw run cursors, an
/// obligation the `&View<PropIter, Mut>` type cannot carry.
#[inline(never)]
pub(crate) unsafe fn next_prop_slow(iter: &View<PropIter, Mut>) -> *const Prop {
    let prop: *const Prop = iter.prop();
    let over: *const PropOverride = iter.over();
    if prop == iter.prop_end() && over == iter.over_end() {
        return ptr::null();
    }

    // We can use `UINT32_MAX` as a terminating key (aka prefix) as prop names must
    // be valid UTF-8 and the byte sequence "\xff\xff\xff\xff" is not valid.
    let prop_key: u32 = if prop != iter.prop_end() {
        // SAFETY: `prop` is short of `prop_end`, so it addresses an element of
        // the prop run.
        unsafe { (*prop)._internal_key }
    } else {
        u32::MAX
    };
    let over_key: u32 = if over != iter.over_end() {
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
        // C: `ufbx_prop *dst = &iter->tmp;`
        let dst: &View<Prop, Mut> = iter.tmp_view();
        // SAFETY: `over` addresses an element of the override run — `cmp >= 0`
        // is only reachable when `over_key` was read from a live element, since
        // a `UINT32_MAX` `over_key` with a live `prop` compares greater. The
        // override run is scene memory that outlives this call, and this fn
        // writes only `iter`'s own `tmp`, so the frozen tag stays valid.
        let over_view: &View<PropOverride, Const> =
            unsafe { View::<PropOverride, Const>::from_ptr(over) };
        dst.set_name(over_view.prop_name());
        dst.set_internal_key(over_view._internal_key());
        dst.set_type(PropType::Unknown);
        dst.set_flags(PropFlags::OVERRIDDEN);
        dst.set_value_str(over_view.value_str());
        // C: `dst->value_blob.data = dst->value_str.data;` + `.size = ....length;`
        let mut blob = dst.value_blob();
        blob.data = dst.value_str().data;
        blob.size = dst.value_str().length;
        dst.set_value_blob(blob);
        dst.set_value_int(over_view.value_int());
        dst.set_value_vec4(over_view.value());
        // SAFETY: `over` is inside the override run, so `over + 1` is at most
        // one past its end.
        iter.set_over(unsafe { over.add(1) });
        if cmp == 0 {
            // SAFETY: `cmp == 0` came from comparing live names, so `prop` is
            // inside the prop run and `prop + 1` is at most one past its end.
            iter.set_prop(unsafe { prop.add(1) });
        }
        dst.as_ptr()
    } else {
        // SAFETY: `cmp < 0` requires a `prop_key` read from a live element, so
        // `prop` is inside the prop run and `prop + 1` is at most one past its
        // end.
        iter.set_prop(unsafe { prop.add(1) });
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
        // SAFETY: `iter` is the caller's live `PropIter` — the raw-pointer
        // contract of this `unsafe fn` — reached through `*mut`, so its
        // provenance is write-capable.
        let iter_view: &View<PropIter, Mut> = unsafe { View::<PropIter, Mut>::from_ptr(iter) };
        // SAFETY: `iter` is as `init_prop_iter` left it, so its `prop`/`over`
        // cursors sit inside the element's prop run and the anim's override run
        // and stop at the matching `*_end` — the run-bounds contract of
        // `next_prop_slow`.
        unsafe { next_prop_slow(iter_view) }
    }
}

// ufbx.c:25926-25974 `ufbxi_evaluate_selected_props`
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
    // Public-boundary roots: `anim`/`element` are caller-owned `*const` pointers
    // whose provenance can be a read-only borrow, so mint read-only `Const`
    // views; the iterator only reads through them and the frozen tags end with
    // this call.
    // SAFETY: `iter` addresses the live local `MaybeUninit<PropIter>` storage,
    // which `init_prop_iter` only writes through; `anim`/`element` are the
    // caller's live scene objects — the contract of this `unsafe fn`.
    unsafe {
        init_prop_iter(
            iter,
            View::<Anim, Const>::from_ptr(anim),
            View::<Element, Const>::from_ptr(element),
        )
    };
    // C: `while ((prop = ufbxi_next_prop(&iter)) != NULL)`
    loop {
        // SAFETY: `iter` is the storage `init_prop_iter` initialized above.
        let prop_ptr: *const Prop = unsafe { next_prop(iter) };
        if prop_ptr.is_null() {
            break;
        }
        // Read-only `Const` view over the yielded prop: it is reached through a
        // `*const Prop` whose provenance can be a read-only `&Element`'s prop
        // run, so `Mut` is not mintable here. The frozen tag is confined to one
        // iteration, which is what the `Const` mode requires: when the anim
        // carries overrides, `next_prop_slow` yields `&raw const (*iter).tmp` and the next
        // call writes those exact bytes through `iter`, but this view's last use
        // is inside this iteration's body — no read through the frozen tag
        // outlives that write. Nothing in the body writes the viewed prop
        // either: the copies land in the caller's separate `props` output
        // buffer.
        // SAFETY (every `prop` access in this loop): `next_prop` returns either
        // an element of the element's own prop run or the iterator's `tmp` prop,
        // both live here, and it is non-null (checked above).
        let prop: &View<Prop, Const> = unsafe { View::<Prop, Const>::from_ptr(prop_ptr) };
        while name_ix < max_props {
            if key > prop._internal_key() {
                break;
            }
            if name == prop.name().data {
                // SAFETY: `anim` is the caller's live anim.
                if (prop.flags().raw() & PropFlags::CONNECTED.raw()) != 0
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
                    // the copy duplicates no ownership. `dst` is then the prop
                    // just written, `anim`/`element` are the caller's live scene
                    // objects and `name` an entry of the caller's name table.
                    unsafe {
                        *dst = *prop.as_ptr();
                        evaluate_connected_prop(dst, anim, element, name, time, flags);
                    }
                } else if (prop.flags().raw()
                    & (PropFlags::ANIMATED.raw() | PropFlags::OVERRIDDEN.raw()))
                    != 0
                {
                    // C: `props[num_props++] = *prop;`
                    // SAFETY: as the `dst` write above — `num_props` is below
                    // `max_props`, and `prop` is a live source prop.
                    unsafe { *props.add(num_props) = *prop.as_ptr() };
                    num_props += 1;
                }
                break;
            // SAFETY: `name` is an entry of the caller's name table and
            // `prop.name` is either a string-pool string (element prop) or an
            // override name interned by `create_anim_imp` — both NUL-terminated.
            } else if unsafe { strcmp(name, prop.name().data) } < 0 {
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

    // The raw destination remains unviewed while it is filled above. At this
    // checkpoint its first `num_props` slots are initialized and stay live and
    // unmoved through both evaluation passes.
    // SAFETY: `props` is the caller's write-capable `max_props`-slot output run,
    // and the loops above initialized exactly its `num_props`-slot prefix;
    // `anim`/`element` are the caller's live scene objects.
    unsafe {
        evaluate_props(
            anim,
            element,
            time,
            Run::from_raw_parts(props, num_props),
            flags,
        )
    };

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
pub(crate) fn extrapolate_curve(
    curve: &View<AnimCurve, Const>,
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
        ret
    }
    #[cfg(not(feature = "regression"))]
    extrapolate_curve_rec(curve, real_time, flags)
}

// ufbx.c:25979-26042 `ufbxi_extrapolate_curve` body (the `_rec` half of the
// `ufbxi_recursive_function` body; see the wrapper above)
#[inline(never)]
fn extrapolate_curve_rec(curve: &View<AnimCurve, Const>, real_time: f64, flags: u32) -> Real {
    let keys = curve.keyframes_view();
    let pre: bool = real_time < curve.min_time();
    let key: &View<Keyframe, Const>;
    // C: `ufbx_extrapolation ext;` — copied by value, matching C.
    let ext: Extrapolation;
    if pre {
        // `ufbx_evaluate_curve_flags` only extrapolates once it has established
        // `keyframes.count > 1`, so slot 0 is in bounds of the `at` check.
        key = keys.at(0);
        ext = curve.pre_extrapolation();
    } else {
        // As above — the keyframe list holds more than one element, so
        // `count - 1` does not underflow and indexes the last slot.
        key = keys.at(keys.count() - 1);
        ext = curve.post_extrapolation();
    }

    if ext.mode == ExtrapolationMode::Constant {
        return key.value();
    } else if ext.mode == ExtrapolationMode::Slope {
        // C: `ufbx_tangent tangent = *(pre ? &key->right : &key->left);`
        let tangent: Tangent = if pre { key.right() } else { key.left() };
        // C: `key->value + (ufbx_real)(tangent.dy * ((real_time - key->time) / tangent.dx))`
        // — `dx`/`dy` are float, promoted to double in the expression.
        return key.value()
            + (tangent.dy as f64 * ((real_time - key.time()) / tangent.dx as f64)) as Real;
    } else if ext.repeat_count == 0 {
        return key.value();
    }

    // Perform all operations in KTime ticks to be frame perfect
    let scene: Ref<Scene> = curve.element().scene();
    // SAFETY: `element.scene` is the non-null owning-scene `Ref` of the live
    // viewed curve (read just above per the mint's per-leaf discipline), so its
    // address is that live `ufbx_scene`, whose metadata the deref reads.
    let scale: f64 = unsafe { (*scene.ptr()).metadata.ktime_second } as f64;
    let min_time: f64 = math::rint(curve.min_time() * scale);
    let max_time: f64 = math::rint(curve.max_time() * scale);
    let time: f64 = real_time * scale;

    let delta: f64 = if pre {
        min_time - time
    } else {
        time - max_time
    };
    let duration: f64 = max_time - min_time;

    // Require at least one KTime unit
    if !(duration >= 1.0) {
        return key.value();
    }

    let rep: f64 = delta / duration;
    let mut rep_n: f64 = math::floor(rep);
    let mut rep_d: f64 = delta - rep_n * duration;

    if ext.repeat_count > 0 && rep_n >= ext.repeat_count as f64 {
        // Clamp to the repeat count to handle mirroring
        rep_n = (ext.repeat_count - 1) as f64;
        rep_d = duration;
    }

    if ext.mode == ExtrapolationMode::Mirror {
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
        Some(curve),
        new_time,
        key.value(),
        flags | crate::generated::EvaluateFlags::NO_EXTRAPOLATION.raw(),
    );

    if ext.mode == ExtrapolationMode::RepeatRelative {
        // The keyframe list holds more than one element (the caller only
        // extrapolates past that check), so both slots are in bounds.
        let mut val_delta: Real = keys.at(keys.count() - 1).value() - keys.at(0).value();
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

// Typed interior-mutable view over the `opts` field, reinterpreted in place.
// Generated ABI-fixed `RawEvaluateOpts` is the inner storage;
// `MaybeUninit` makes forming `&EvaluateOptsView` assert no validity — each leaf getter
// asserts only the field it reads.
#[cfg(feature = "scene-eval")]
pub(crate) type EvaluateOptsView = crate::native::view::View<RawEvaluateOpts>;

#[cfg(feature = "scene-eval")]
impl EvaluateOptsView {
    #[inline(always)]
    pub(crate) fn evaluate_caches(&self) -> bool {
        view_read!(self, evaluate_caches)
    }

    #[inline(always)]
    pub(crate) fn evaluate_flags(&self) -> u32 {
        view_read!(self, evaluate_flags)
    }

    #[inline(always)]
    pub(crate) fn evaluate_skinning(&self) -> bool {
        view_read!(self, evaluate_skinning)
    }

    #[inline(always)]
    pub(crate) fn load_external_files(&self) -> bool {
        view_read!(self, load_external_files)
    }

    #[inline(always)]
    pub(crate) fn open_file_cb_ptr(&self) -> *const crate::generated::RawOpenFileCb {
        view_raw_const!(self, open_file_cb)
    }
}

// Mode-generic nested views over the allocator descriptors: `init_ator` only
// reads them, so the accessor serves a `Mut` context field and a `Const`
// boundary mint alike.
#[cfg(feature = "scene-eval")]
impl<M: crate::native::view::Mode> View<RawEvaluateOpts, M> {
    #[inline(always)]
    pub(crate) fn temp_allocator_view(&self) -> &View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, temp_allocator)
    }
    #[inline(always)]
    pub(crate) fn result_allocator_view(&self) -> &View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, result_allocator)
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
        view_read!(self, ator_result)
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

    // `src_scene` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn src_scene_mut_ptr(&self) -> *mut Scene {
        view_raw_mut!(self, src_scene)
    }

    // `scene` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn scene_mut_ptr(&self) -> *mut Scene {
        view_raw_mut!(self, scene)
    }

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawEvaluateOpts {
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

    // `anim` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn anim_mut_ptr(&self) -> *mut *mut Anim {
        view_raw_mut!(self, anim)
    }

    // `scene_imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn scene_imp(&self) -> *mut SceneImp {
        view_read!(self, scene_imp)
    }

    #[inline(always)]
    pub(crate) fn set_scene_imp(&self, scene_imp: *mut SceneImp) {
        view_write!(self, scene_imp, scene_imp)
    }

    // `time` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn time(&self) -> f64 {
        view_read!(self, time)
    }

    #[inline(always)]
    pub(crate) fn set_time(&self, time: f64) {
        view_write!(self, time, time)
    }

    // `anim` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn anim(&self) -> *mut Anim {
        view_read!(self, anim)
    }

    #[inline(always)]
    pub(crate) fn set_anim(&self, anim: *mut Anim) {
        view_write!(self, anim, anim)
    }

    // `src_imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn src_imp(&self) -> *mut SceneImp {
        view_read!(self, src_imp)
    }

    #[inline(always)]
    pub(crate) fn set_src_imp(&self, src_imp: *mut SceneImp) {
        view_write!(self, src_imp, src_imp)
    }

    #[inline(always)]
    pub(crate) fn dst_element(&self) -> *mut u8 {
        view_read!(self, dst_element)
    }

    #[inline(always)]
    pub(crate) fn set_dst_element(&self, dst_element: *mut u8) {
        view_write!(self, dst_element, dst_element)
    }

    // `src_element` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn src_element(&self) -> *mut u8 {
        view_read!(self, src_element)
    }

    #[inline(always)]
    pub(crate) fn set_src_element(&self, src_element: *mut u8) {
        view_write!(self, src_element, src_element)
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
    // `ufbx_element_list` of the source scene, so its count and element array
    // base are its own fields.
    let (count, src): (usize, *mut *mut Element) =
        unsafe { ((*list).count, (*list).data as *mut *mut Element) };
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
// Stays `unsafe fn`: `Run` carries the initialized map slots, but not the
// relational invariant that every stored texture belongs to `ec`'s source
// scene, which is what `translate_element` requires.
#[cfg(feature = "scene-eval")]
#[inline(never)]
pub(crate) unsafe fn translate_maps(ec: &EvalContext, maps: Run<'_, MaterialMap>) {
    // C: `ufbxi_nounroll ufbxi_for(ufbx_material_map, map, maps, count)`
    for map in maps.iter() {
        // SAFETY: each map's `texture` belongs to the source scene, which is
        // `translate_element`'s contract; a non-null result names the matching
        // destination-scene texture, which is `opt_ref`'s contract.
        unsafe {
            map.set_texture(opt_ref(translate_element(
                ec,
                map.texture().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut Texture));
        }
    }
}

// ufbx.c:26096-26103 `ufbxi_translate_anim`
// Stays `unsafe fn`: the slot type carries the slot's own liveness and write
// capability, but not the one obligation the body leans on — that the
// `ufbx_anim*` the slot HOLDS addresses a live source-scene `ufbx_anim`, which
// `push_copy_raw` reads a whole struct through and which no pointer type
// expresses.
//
// The slot type is C's `ufbx_anim **p_anim` verbatim, so callers whose field is
// the niche-restricted `Ref<Anim>` (`#[repr(transparent)]` `NonNull<Anim>`) pun
// its slot as `ScalarView<*mut Anim>`. Two obligations that pun creates, both
// discharged here for every caller: the only value this fn ever stores is the
// pushed copy, checked non-null before the store, so a punned `Ref` slot never
// receives an invalid (null) value; and the shared `&ScalarView` (a `Cell`)
// writes through the slot, so callers must hand over a slot in
// context/arena-owned interior-mutable storage — i.e. one reached through a
// `Mut` view, never a place whose declared type has no `UnsafeCell`.
#[cfg(feature = "scene-eval")]
#[inline(never)]
pub(crate) unsafe fn translate_anim(
    ec: &EvalContext,
    p_anim: &ScalarView<*mut Anim>,
) -> Result<(), Fail> {
    // SAFETY: the buf side is the eval context's own result buffer (the view
    // invariant); the source run is the anim the slot holds, which this
    // `unsafe fn` requires to be a live source-scene `ufbx_anim` — source-scene
    // memory, disjoint from the result chunks the push writes.
    let anim: *mut Anim = unsafe { ec.result_view().push_copy_raw::<Anim>(1, p_anim.get()) };
    ufbxi_check_err!(ec.error_view(), !anim.is_null(), "anim");
    // SAFETY: `anim` is the non-null (checked just above) freshly pushed copy, so
    // its `layers` field is a live `ufbx_element_list` of source-scene layers —
    // `translate_element_list`'s contract.
    unsafe { translate_element_list(ec, &raw mut (*anim).layers as *mut c_void) }?;
    // C: `*p_anim = anim;`
    p_anim.set(anim);
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
    // C: `for (size_t i = 0; i < UFBX_ELEMENT_TYPE_COUNT; i++)` — one vouch for
    // the whole run, then a safe walk.
    // SAFETY: `by_type` is the destination scene's `ELEMENT_TYPE_COUNT`-long
    // per-type list array, a contiguous run of live `ufbx_element_list`s inside
    // `ec`'s own scene struct, reached through `*mut` — write-capable
    // provenance for `Mut`.
    let by_type_run = unsafe {
        SliceViewIter::<crate::prelude::RefList<Element>>::from_raw_parts(
            by_type,
            ELEMENT_TYPE_COUNT,
        )
    };
    for list in by_type_run {
        // Slot `i`'s count is read, then it is retargeted at the array just
        // pushed for it.
        list.set_data(ec.result_view().push::<*mut Element>(list.count()) as *const Ref<Element>);
        ufbxi_check_err!(
            ec.error_view(),
            !list.data().is_null(),
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
        // C: `ufbx_connection *src = &ec->scene.connections_src.data[i];` and
        // the `connections_dst` sibling — the destination slots as views: both
        // runs were pushed with `num_connections` elements (non-null, checked
        // just above) and per the source-scene premise both lists carry that
        // count, so the indexing is in bounds of each.
        let src: &View<Connection> = ec.scene_view().connections_src_view().at(i);
        let dst: &View<Connection> = ec.scene_view().connections_dst_view().at(i);
        // C: `*src = ec->src_scene.connections_src.data[i];` (struct assignment)
        // SAFETY: per the source-scene premise the source `connections_src` run
        // holds `num_connections` live `ufbx_connection`s, so slot `i` is
        // readable; `src` views the matching slot of the freshly pushed
        // destination run, a separate allocation; the same holds for the
        // `connections_dst` runs.
        unsafe {
            ptr::copy_nonoverlapping(
                ec.src_scene_view().connections_src_view().data().add(i),
                src.get(),
                1,
            );
            ptr::copy_nonoverlapping(
                ec.src_scene_view().connections_dst_view().data().add(i),
                dst.get(),
                1,
            );
        }
        // SAFETY (these four stores): `src` and `dst` view live connection
        // slots holding the copies made just above, whose `src`/`dst` are
        // non-null `Ref`s to source-scene elements — `translate_element`'s
        // contract — read by value and written back in place.
        unsafe {
            *(src.src_raw() as *mut *mut Element) =
                translate_element(ec, src.src().ptr() as *mut c_void);
        }
        unsafe {
            *(src.dst_raw() as *mut *mut Element) =
                translate_element(ec, src.dst().ptr() as *mut c_void);
        }
        unsafe {
            *(dst.src_raw() as *mut *mut Element) =
                translate_element(ec, dst.src().ptr() as *mut c_void);
        }
        unsafe {
            *(dst.dst_raw() as *mut *mut Element) =
                translate_element(ec, dst.dst().ptr() as *mut c_void);
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
            translate_element(ec, ec.scene_view().root_node().ptr() as *mut c_void)
                as *mut UfbxNode;
    }
    // SAFETY: `anim` is a field of the destination scene struct inside `ec`,
    // reached through a `Mut` view, so the pointer addresses a live,
    // write-capable `ufbx_anim*` slot in context-owned interior-mutable storage
    // — the storage the shared `&ScalarView` (`Cell`) writes through. The
    // field's declared type is `Ref<Anim>`, so the pun widens a non-null slot
    // into a nullable one; `translate_anim` only ever stores its non-null-
    // checked push result, so the slot keeps a valid `Ref`. Per the
    // source-scene premise its byte copy names the source scene's anim —
    // `translate_anim`'s contract.
    unsafe {
        translate_anim(
            ec,
            &*(ec.scene_view().anim_mut_ptr() as *const ScalarView<*mut Anim>),
        )
    }?;

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

        // The destination element copy as a view for the field work below. A
        // view over an element covers the `ufbx_element` HEADER only — which is
        // where every field touched from here on lives — so the whole-element
        // `memcpy` above stays on the raw pointer.
        // SAFETY: `dst` addresses the element copy written just above, reached
        // through `*mut` from the freshly pushed destination buffer —
        // write-capable provenance for `Mut`.
        let dst_view: &View<Element> = unsafe { View::<Element>::from_ptr(dst) };

        // C: `dst->connections_src.data = ec->scene.connections_src.data + (dst->connections_src.data - ec->src_scene.connections_src.data);`
        // SAFETY: `dst` holds the byte copy of `src` made above, so its
        // `connections_src`/`connections_dst` data still points into the source
        // scene's matching run — one allocation with the base being subtracted,
        // making `offset_from` well defined — and the same index lands inside the
        // equally sized destination run pushed above.
        let connections_src_data: *const Connection = unsafe {
            ec.scene_view().connections_src_view().data().offset(
                dst_view
                    .connections_src_view()
                    .data()
                    .offset_from(ec.src_scene_view().connections_src_view().data()),
            )
        };
        dst_view
            .connections_src_view()
            .set_data(connections_src_data);
        // SAFETY: as above, for the `connections_dst` runs.
        let connections_dst_data: *const Connection = unsafe {
            ec.scene_view().connections_dst_view().data().offset(
                dst_view
                    .connections_dst_view()
                    .data()
                    .offset_from(ec.src_scene_view().connections_dst_view().data()),
            )
        };
        dst_view
            .connections_dst_view()
            .set_data(connections_dst_data);
        if dst_view.instances_view().count() > 0 {
            // SAFETY: `instances` is the destination element copy's own
            // `ufbx_element_list`, whose byte copy still lists source-scene
            // elements — `translate_element_list`'s contract.
            unsafe { translate_element_list(ec, dst_view.instances_raw() as *mut c_void) }?;
        }

        // C: `ufbx_name_element named = ec->src_scene.elements_by_name.data[i];`
        // then `named.element = ...; ec->scene.elements_by_name.data[i] = named;`
        // — copied straight into the destination slot here (same writes).
        // The destination entry as a view: the array was pushed with
        // `num_elements` entries (non-null, checked above) and per the
        // source-scene premise the list carries that count, so the indexing is
        // in bounds.
        let named: &View<NameElement> = ec.scene_view().elements_by_name_view().at(i);
        // SAFETY: per the source-scene premise the source `elements_by_name` run
        // holds `num_elements` live entries, so slot `i` is readable; `named`
        // views the matching slot of the freshly pushed destination array, a
        // separate allocation.
        unsafe {
            ptr::copy_nonoverlapping(
                ec.src_scene_view().elements_by_name_view().data().add(i),
                named.get(),
                1,
            );
        }
        // SAFETY: `named` views the live entry copied just above, whose
        // `element` is a non-null `Ref` to a source-scene element —
        // `translate_element`'s contract — read by value and written back in
        // place.
        unsafe {
            *(named.element_raw() as *mut *mut Element) =
                translate_element(ec, named.element().ptr() as *mut c_void);
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_node, p_node, ec->scene.nodes)` — the walk is
    // the destination scene's own node list, so each `*p_node` is one of the
    // element copies the loop above wrote into the destination buffer, viewed
    // in place.
    let nodes = ec.scene_view().nodes_view();
    for i in 0..nodes.count() {
        // C: `ufbx_node *node = *p_node;`
        let node: &View<UfbxNode> = nodes.at(i);
        // SAFETY (this store and the ones below it): each named field is
        // `node`'s own nullable `Option<Ref<..>>`. The byte copy left every
        // one of them still pointing at the source-scene element it named —
        // `translate_element`'s contract — and the translated pointer is
        // written back into the same field.
        unsafe {
            *(node.parent_raw() as *mut *mut UfbxNode) = translate_element(
                ec,
                node.parent().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut UfbxNode;
        }
        // SAFETY: `children` is the node's own `ufbx_element_list`, still
        // listing source-scene elements — `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, node.children_raw() as *mut c_void) }?;

        // SAFETY: as for `parent` above.
        unsafe {
            *(node.attrib_raw() as *mut *mut Element) = translate_element(
                ec,
                node.attrib().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            );
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(node.mesh_raw() as *mut *mut Mesh) = translate_element(
                ec,
                node.mesh().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut Mesh;
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(node.light_raw() as *mut *mut crate::generated::Light) = translate_element(
                ec,
                node.light().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            )
                as *mut crate::generated::Light;
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(node.camera_raw() as *mut *mut Camera) = translate_element(
                ec,
                node.camera().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut Camera;
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(node.bone_raw() as *mut *mut crate::generated::Bone) = translate_element(
                ec,
                node.bone().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            )
                as *mut crate::generated::Bone;
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(node.inherit_scale_node_raw() as *mut *mut UfbxNode) = translate_element(
                ec,
                node.inherit_scale_node()
                    .map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut UfbxNode;
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(node.scale_helper_raw() as *mut *mut UfbxNode) = translate_element(
                ec,
                node.scale_helper().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut UfbxNode;
        }
        // SAFETY: as for `parent` above.
        unsafe {
            *(node.bind_pose_raw() as *mut *mut Pose) = translate_element(
                ec,
                node.bind_pose().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut Pose;
        }

        if node.all_attribs_view().count() > 1 {
            // SAFETY: `all_attribs` is the node's own `ufbx_element_list`, still
            // naming source-scene elements — `translate_element_list`'s
            // contract.
            unsafe { translate_element_list(ec, node.all_attribs_raw() as *mut c_void) }?;
        } else if node.all_attribs_view().count() == 1 {
            // C: `node->all_attribs.data = &node->attrib;` — the single-element
            // list is retargeted at that node's own already translated `attrib`
            // slot, addressed through the node's own view.
            node.all_attribs_view()
                .set_data(node.attrib_ptr() as *const Ref<Element>);
        }

        // SAFETY: as for `parent` above.
        unsafe {
            *(node.geometry_transform_helper_raw() as *mut *mut UfbxNode) = translate_element(
                ec,
                node.geometry_transform_helper()
                    .map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            )
                as *mut UfbxNode;
        }

        // SAFETY: `materials` is the node's own `ufbx_element_list`, still
        // listing source-scene elements — `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, node.materials_raw() as *mut c_void) }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, ec->scene.meshes)` — see the
    // node walk above for the list-view iteration.
    let meshes = ec.scene_view().meshes_view();
    for i in 0..meshes.count() {
        // C: `ufbx_mesh *mesh = *p_mesh;`
        let mesh: &View<Mesh> = meshes.at(i);

        // SAFETY (these five calls): each `*_raw` projection addresses the
        // destination mesh's own `ufbx_element_list`, whose byte copy still
        // lists source-scene elements — `translate_element_list`'s contract.
        unsafe {
            translate_element_list(ec, mesh.materials_raw() as *mut c_void)?;
            translate_element_list(ec, mesh.skin_deformers_raw() as *mut c_void)?;
            translate_element_list(ec, mesh.blend_deformers_raw() as *mut c_void)?;
            translate_element_list(ec, mesh.cache_deformers_raw() as *mut c_void)?;
            translate_element_list(ec, mesh.all_deformers_raw() as *mut c_void)?;
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_stereo_camera, p_stereo, ec->scene.stereo_cameras)`
    let stereo_cameras = ec.scene_view().stereo_cameras_view();
    for i in 0..stereo_cameras.count() {
        // C: `ufbx_stereo_camera *stereo = *p_stereo;`
        let stereo: &View<StereoCamera> = stereo_cameras.at(i);
        // SAFETY (both stores): `left`/`right` are the stereo camera's own
        // nullable `Option<Ref<Camera>>` fields; the byte copy left them
        // naming source-scene elements — `translate_element`'s contract — and
        // the result is written back in place.
        unsafe {
            *(stereo.left_raw() as *mut *mut Camera) = translate_element(
                ec,
                stereo.left().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut Camera;
            *(stereo.right_raw() as *mut *mut Camera) = translate_element(
                ec,
                stereo.right().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut Camera;
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_skin_deformer, p_skin, ec->scene.skin_deformers)`
    let skin_deformers = ec.scene_view().skin_deformers_view();
    for i in 0..skin_deformers.count() {
        // C: `ufbx_skin_deformer *skin = *p_skin;`
        let skin: &View<SkinDeformer> = skin_deformers.at(i);
        // SAFETY: `clusters` is that deformer's own `ufbx_element_list`, whose
        // byte copy still lists source-scene elements —
        // `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, skin.clusters_raw() as *mut c_void) }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, ec->scene.skin_clusters)`
    let skin_clusters = ec.scene_view().skin_clusters_view();
    for i in 0..skin_clusters.count() {
        // C: `ufbx_skin_cluster *cluster = *p_cluster;`
        let cluster: &View<SkinCluster> = skin_clusters.at(i);
        // SAFETY: `bone_node` is that cluster's own nullable
        // `Option<Ref<UfbxNode>>` field; the byte copy left it naming a
        // source-scene element — `translate_element`'s contract — and the
        // result is written back in place.
        unsafe {
            *(cluster.bone_node_raw() as *mut *mut UfbxNode) = translate_element(
                ec,
                cluster.bone_node().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut UfbxNode;
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_blend_deformer, p_blend, ec->scene.blend_deformers)`
    let blend_deformers = ec.scene_view().blend_deformers_view();
    for i in 0..blend_deformers.count() {
        // C: `ufbx_blend_deformer *blend = *p_blend;`
        let blend: &View<BlendDeformer> = blend_deformers.at(i);
        // SAFETY: `channels` is that deformer's own `ufbx_element_list`, whose
        // byte copy still lists source-scene elements —
        // `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, blend.channels_raw() as *mut c_void) }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_blend_channel, p_chan, ec->scene.blend_channels)`
    let blend_channels = ec.scene_view().blend_channels_view();
    for i_chan in 0..blend_channels.count() {
        // C: `ufbx_blend_channel *chan = *p_chan;`
        let chan: &View<BlendChannel> = blend_channels.at(i_chan);

        // `keyframes` is that channel's own list, whose byte copy still
        // describes the source scene's keyframe run.
        let count = chan.keyframes_view().count();
        let keys: *mut BlendKeyframe = ec.result_view().push::<BlendKeyframe>(count);
        ufbxi_check_err!(ec.error_view(), !keys.is_null(), "keys");
        // C: `for (size_t i = 0; i < chan->keyframes.count; i++)` — one vouch
        // for the pushed run, then a safe walk.
        // SAFETY: `keys` is the contiguous `keyframes.count`-element allocation
        // just pushed (non-null, checked above), reached through `*mut` —
        // write-capable provenance for `Mut`; `from_raw_parts` admits such a
        // still-uninitialized run, and the loop below initializes every slot
        // before reading it.
        let keys_run = unsafe { SliceViewIter::<BlendKeyframe>::from_raw_parts(keys, count) };
        for (i, key) in keys_run.enumerate() {
            // C: `keys[i] = chan->keyframes.data[i];` (struct assignment)
            // SAFETY: `i` is below the keyframe count, so slot `i` is in bounds
            // of the source keyframe run; `key` views the matching slot of the
            // pushed allocation, a separate one.
            unsafe {
                ptr::copy_nonoverlapping(chan.keyframes_view().data().add(i), key.get(), 1);
            }
            // SAFETY: `keys[i]` holds the copy made just above, whose `shape`
            // is a non-null `Ref` to a source-scene element —
            // `translate_element`'s contract — read by value and written back
            // in place.
            unsafe {
                *(key.shape_raw() as *mut *mut BlendShape) =
                    translate_element(ec, key.shape().ptr() as *mut c_void) as *mut BlendShape;
            }
        }
        // SAFETY: all `count` slots were initialized and translated above; the
        // result-buffer run remains stable with the evaluated scene.
        let keys = unsafe { List::from_raw_parts(keys, count) };
        // C: `chan->keyframes.data = keys;` — the destination channel retargeted
        // at the translated keyframe array, retaining its inherited count.
        chan.keyframes_view().set(keys);
        // SAFETY: `target_shape` is that channel's own nullable
        // `Option<Ref<BlendShape>>` field; the byte copy left it naming a
        // source-scene element — `translate_element`'s contract — and the
        // result is written back in place.
        unsafe {
            *(chan.target_shape_raw() as *mut *mut BlendShape) = translate_element(
                ec,
                chan.target_shape().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut BlendShape;
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_cache_deformer, p_deformer, ec->scene.cache_deformers)`
    let cache_deformers = ec.scene_view().cache_deformers_view();
    for i in 0..cache_deformers.count() {
        // C: `ufbx_cache_deformer *deformer = *p_deformer;`
        let deformer: &View<CacheDeformer> = cache_deformers.at(i);
        // SAFETY: `file` is that deformer's own nullable
        // `Option<Ref<CacheFile>>` field; the byte copy left it naming a
        // source-scene element — `translate_element`'s contract — and the
        // result is written back in place.
        unsafe {
            *(deformer.file_raw() as *mut *mut CacheFile) = translate_element(
                ec,
                deformer.file().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut CacheFile;
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_material, p_material, ec->scene.materials)`
    let materials = ec.scene_view().materials_view();
    for i_material in 0..materials.count() {
        // C: `ufbx_material *material = *p_material;`
        let material: &View<Material> = materials.at(i_material);

        // SAFETY: `shader` is that material's own nullable
        // `Option<Ref<Shader>>` field; the byte copy left it naming a
        // source-scene element — `translate_element`'s contract — and the
        // result is written back in place.
        unsafe {
            *(material.shader_raw() as *mut *mut Shader) = translate_element(
                ec,
                material.shader().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut Shader;
        }
        // C: `material->fbx.maps` / `material->pbr.maps` — the flat `maps[]`
        // union view; the generated struct keeps only the named branch, whose
        // base is the aggregate itself (layout pinned in `native::scene_process`).
        // SAFETY: each aggregate is `MATERIAL_FBX_MAP_COUNT` /
        // `MATERIAL_PBR_MAP_COUNT` live `MaterialMap`s of the destination
        // material, still naming source-scene textures — `translate_maps`'
        // contract.
        unsafe {
            translate_maps(
                ec,
                Run::from_raw_parts(
                    material.fbx_raw() as *mut MaterialMap,
                    MATERIAL_FBX_MAP_COUNT,
                ),
            );
            translate_maps(
                ec,
                Run::from_raw_parts(
                    material.pbr_raw() as *mut MaterialMap,
                    MATERIAL_PBR_MAP_COUNT,
                ),
            );
        }

        // `textures` is that material's own list, whose byte copy still
        // describes the source scene's material-texture run.
        let count = material.textures_view().count();
        let textures: *mut MaterialTexture = ec.result_view().push::<MaterialTexture>(count);
        ufbxi_check_err!(ec.error_view(), !textures.is_null(), "textures");
        // C: `for (size_t i = 0; i < material->textures.count; i++)` — one vouch
        // for the pushed run, then a safe walk.
        // SAFETY: `textures` is the contiguous `textures.count`-element
        // allocation just pushed (non-null, checked above), reached through
        // `*mut` — write-capable provenance for `Mut`; `from_raw_parts` admits
        // such a still-uninitialized run, and the loop below initializes every
        // slot before reading it.
        let textures_run =
            unsafe { SliceViewIter::<MaterialTexture>::from_raw_parts(textures, count) };
        for (i, texture) in textures_run.enumerate() {
            // C: `textures[i] = material->textures.data[i];` (struct assignment)
            // SAFETY: `i` is below the texture count, so slot `i` is in bounds
            // of the source run; `texture` views the matching slot of the pushed
            // allocation, a separate one.
            unsafe {
                ptr::copy_nonoverlapping(material.textures_view().data().add(i), texture.get(), 1);
            }
            // SAFETY: `textures[i]` holds the copy made just above, whose
            // `texture` is a non-null `Ref` to a source-scene element —
            // `translate_element`'s contract — read by value and written back
            // in place.
            unsafe {
                *(texture.texture_raw() as *mut *mut Texture) =
                    translate_element(ec, texture.texture().ptr() as *mut c_void) as *mut Texture;
            }
        }
        // SAFETY: all `count` slots were initialized and translated above; the
        // result-buffer run remains stable with the evaluated scene.
        let textures = unsafe { List::from_raw_parts(textures, count) };
        // C: `material->textures.data = textures;` — the destination material
        // retargeted at the translated texture array, retaining its count.
        material.textures_view().set(textures);
    }

    // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, ec->scene.textures)`
    let textures = ec.scene_view().textures_view();
    for i_texture in 0..textures.count() {
        // C: `ufbx_texture *texture = *p_texture;`
        let texture: &View<Texture> = textures.at(i_texture);
        // SAFETY: `video` is that texture's own nullable `Option<Ref<Video>>`
        // field; the byte copy left it naming a source-scene element —
        // `translate_element`'s contract — and the result is written back in
        // place.
        unsafe {
            *(texture.video_raw() as *mut *mut Video) = translate_element(
                ec,
                texture.video().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut Video;
        }

        // `layers` is that texture's own list, whose byte copy still describes
        // the source scene's texture-layer run.
        let count = texture.layers_view().count();
        let layers: *mut TextureLayer = ec.result_view().push::<TextureLayer>(count);
        ufbxi_check_err!(ec.error_view(), !layers.is_null(), "layers");
        // C: `for (size_t i = 0; i < texture->layers.count; i++)` — one vouch
        // for the pushed run, then a safe walk.
        // SAFETY: `layers` is the contiguous `layers.count`-element allocation
        // just pushed (non-null, checked above), reached through `*mut` —
        // write-capable provenance for `Mut`; `from_raw_parts` admits such a
        // still-uninitialized run, and the loop below initializes every slot
        // before reading it.
        let layers_run = unsafe { SliceViewIter::<TextureLayer>::from_raw_parts(layers, count) };
        for (i, layer) in layers_run.enumerate() {
            // C: `layers[i] = texture->layers.data[i];` (struct assignment)
            // SAFETY: `i` is below the layer count, so slot `i` is in bounds of
            // the source run; `layer` views the matching slot of the pushed
            // allocation, a separate one.
            unsafe {
                ptr::copy_nonoverlapping(texture.layers_view().data().add(i), layer.get(), 1);
            }
            // SAFETY: `layers[i]` holds the copy made just above, whose
            // `texture` is a non-null `Ref` to a source-scene element —
            // `translate_element`'s contract — read by value and written back
            // in place.
            unsafe {
                *(layer.texture_raw() as *mut *mut Texture) =
                    translate_element(ec, layer.texture().ptr() as *mut c_void) as *mut Texture;
            }
        }
        // SAFETY: all `count` slots were initialized and translated above; the
        // result-buffer run remains stable with the evaluated scene.
        let layers = unsafe { List::from_raw_parts(layers, count) };
        // C: `texture->layers.data = layers;` — the destination texture
        // retargeted at the translated layer array, retaining its count.
        texture.layers_view().set(layers);

        // SAFETY: `file_textures` is that texture's own `ufbx_element_list`,
        // whose byte copy still lists source-scene elements —
        // `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, texture.file_textures_raw() as *mut c_void) }?;

        // C: `if (texture->shader) { ... }` — the texture's own nullable
        // `Option<Ref<ShaderTexture>>` field, whose `Some` case carries the
        // source-scene shader texture's address.
        if let Some(shader_ref) = texture.shader() {
            let mut shader: *mut ShaderTexture = shader_ref.ptr();
            // SAFETY: `ec.result_mut_ptr()` is the eval context's own result
            // buffer and `shader` is the address of the `Some` branch's
            // non-null `Ref`, the source-scene shader texture `push_copy`
            // copies.
            shader = unsafe { ec.result_view().push_copy_raw::<ShaderTexture>(1, shader) };
            ufbxi_check_err!(ec.error_view(), !shader.is_null(), "shader");
            // SAFETY: `texture` views the destination texture, retargeted at the
            // copy just pushed.
            unsafe { *(texture.shader_raw() as *mut *mut ShaderTexture) = shader };

            // The pushed shader-texture copy as a view for the input work below.
            // SAFETY: `shader` is the non-null (checked just above) copy pushed
            // into the result buffer, reached through `*mut` — write-capable
            // provenance for `Mut`.
            let shader_view: &View<ShaderTexture> =
                unsafe { View::<ShaderTexture>::from_ptr(shader) };
            let count = shader_view.inputs_view().count();
            // SAFETY: `shader_view` views that freshly pushed copy, so its
            // `inputs` list still describes the source scene's initialized
            // input run. The checked result-buffer copy remains stable with the
            // evaluated scene.
            let inputs = unsafe {
                let data = ec
                    .result_view()
                    .push_copy_raw::<ShaderTextureInput>(count, shader_view.inputs_view().data());
                ufbxi_check_err!(ec.error_view(), !data.is_null(), "inputs");
                List::from_raw_parts(data, count)
            };
            // C: `shader->inputs.data = inputs;` — the pushed copy retargeted at
            // the pushed input array, retaining its inherited count.
            shader_view.inputs_view().set(inputs);
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_shader, p_shader, ec->scene.shaders)` — the
    // walk is the destination scene's own shader list, so each `*p_shader` is one
    // of the element copies written into the destination buffer above, viewed in
    // place (see the node walk earlier for the list-view iteration).
    let shaders = ec.scene_view().shaders_view();
    for i in 0..shaders.count() {
        // C: `ufbx_shader *shader = *p_shader;`
        let shader: &View<Shader> = shaders.at(i);
        // SAFETY: `bindings` is that shader's own `ufbx_element_list`, whose byte
        // copy still lists source-scene elements — `translate_element_list`'s
        // contract.
        unsafe { translate_element_list(ec, shader.bindings_raw() as *mut c_void) }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_display_layer, p_layer, ec->scene.display_layers)`
    let display_layers = ec.scene_view().display_layers_view();
    for i in 0..display_layers.count() {
        // C: `ufbx_display_layer *layer = *p_layer;`
        let layer: &View<DisplayLayer> = display_layers.at(i);

        // SAFETY: `nodes` is that layer's own `ufbx_element_list`, whose byte
        // copy still lists source-scene elements — `translate_element_list`'s
        // contract.
        unsafe { translate_element_list(ec, layer.nodes_raw() as *mut c_void) }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_selection_set, p_set, ec->scene.selection_sets)`
    let selection_sets = ec.scene_view().selection_sets_view();
    for i in 0..selection_sets.count() {
        // C: `ufbx_selection_set *set = *p_set;`
        let set: &View<SelectionSet> = selection_sets.at(i);

        // SAFETY: `nodes` is that set's own `ufbx_element_list`, whose byte copy
        // still lists source-scene elements — `translate_element_list`'s
        // contract.
        unsafe { translate_element_list(ec, set.nodes_raw() as *mut c_void) }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_selection_node, p_node, ec->scene.selection_nodes)`
    let selection_nodes = ec.scene_view().selection_nodes_view();
    for i in 0..selection_nodes.count() {
        // C: `ufbx_selection_node *node = *p_node;`
        let node: &View<SelectionNode> = selection_nodes.at(i);

        // SAFETY (both stores): each named field is that selection node's own
        // nullable `Option<Ref<..>>`; the byte copy left it naming a
        // source-scene element — `translate_element`'s contract — and the
        // result is written back in place.
        unsafe {
            *(node.target_node_raw() as *mut *mut UfbxNode) = translate_element(
                ec,
                node.target_node().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut UfbxNode;
            *(node.target_mesh_raw() as *mut *mut Mesh) = translate_element(
                ec,
                node.target_mesh().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut Mesh;
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_constraint, p_constraint, ec->scene.constraints)`
    let constraints = ec.scene_view().constraints_view();
    for i_constraint in 0..constraints.count() {
        // C: `ufbx_constraint *constraint = *p_constraint;`
        let constraint: &View<Constraint> = constraints.at(i_constraint);

        // SAFETY (these four stores): each named field is that constraint's
        // own nullable `Option<Ref<UfbxNode>>`; the byte copy left it naming a
        // source-scene element — `translate_element`'s contract — and the
        // result is written back in place.
        unsafe {
            *(constraint.node_raw() as *mut *mut UfbxNode) = translate_element(
                ec,
                constraint.node().map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut UfbxNode;
            *(constraint.aim_up_node_raw() as *mut *mut UfbxNode) = translate_element(
                ec,
                constraint
                    .aim_up_node()
                    .map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut UfbxNode;
            *(constraint.ik_effector_raw() as *mut *mut UfbxNode) = translate_element(
                ec,
                constraint
                    .ik_effector()
                    .map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut UfbxNode;
            *(constraint.ik_end_node_raw() as *mut *mut UfbxNode) = translate_element(
                ec,
                constraint
                    .ik_end_node()
                    .map_or(ptr::null_mut(), |r| r.ptr()) as *mut c_void,
            ) as *mut UfbxNode;
        }

        // `targets` is that constraint's own list, whose byte copy still
        // describes the source scene's target run.
        let count = constraint.targets_view().count();
        let targets: *mut ConstraintTarget = ec.result_view().push::<ConstraintTarget>(count);
        ufbxi_check_err!(ec.error_view(), !targets.is_null(), "targets");
        // C: `for (size_t i = 0; i < constraint->targets.count; i++)` — one vouch
        // for the pushed run, then a safe walk.
        // SAFETY: `targets` is the contiguous `targets.count`-element allocation
        // just pushed (non-null, checked above), reached through `*mut` —
        // write-capable provenance for `Mut`; `from_raw_parts` admits such a
        // still-uninitialized run, and the loop below initializes every slot
        // before reading it.
        let targets_run =
            unsafe { SliceViewIter::<ConstraintTarget>::from_raw_parts(targets, count) };
        for (i, target) in targets_run.enumerate() {
            // C: `targets[i] = constraint->targets.data[i];` (struct assignment)
            // SAFETY: `i` is below the target count, so slot `i` is in bounds of
            // the source run; `target` views the matching slot of the pushed
            // allocation, a separate one.
            unsafe {
                ptr::copy_nonoverlapping(constraint.targets_view().data().add(i), target.get(), 1);
            }
            // SAFETY: `targets[i]` holds the copy made just above, whose
            // `node` is a non-null `Ref` to a source-scene element —
            // `translate_element`'s contract — read by value and written back
            // in place.
            unsafe {
                *(target.node_raw() as *mut *mut UfbxNode) =
                    translate_element(ec, target.node().ptr() as *mut c_void) as *mut UfbxNode;
            }
        }
        // SAFETY: all `count` slots were initialized and translated above; the
        // result-buffer run remains stable with the evaluated scene.
        let targets = unsafe { List::from_raw_parts(targets, count) };
        // C: `constraint->targets.data = targets;` — the destination constraint
        // retargeted at the translated target array, retaining its count.
        constraint.targets_view().set(targets);
    }

    // C: `ufbxi_for_ptr_list(ufbx_audio_layer, p_layer, ec->scene.audio_layers)`
    let audio_layers = ec.scene_view().audio_layers_view();
    for i in 0..audio_layers.count() {
        // C: `ufbx_audio_layer *layer = *p_layer;`
        let layer: &View<AudioLayer> = audio_layers.at(i);

        // SAFETY: `clips` is that layer's own `ufbx_element_list`, whose byte
        // copy still lists source-scene elements — `translate_element_list`'s
        // contract.
        unsafe { translate_element_list(ec, layer.clips_raw() as *mut c_void) }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_stack, p_stack, ec->scene.anim_stacks)`
    let anim_stacks = ec.scene_view().anim_stacks_view();
    for i in 0..anim_stacks.count() {
        // C: `ufbx_anim_stack *stack = *p_stack;`
        let stack: &View<AnimStack> = anim_stacks.at(i);

        // SAFETY: `layers` is that stack's own `ufbx_element_list`, whose byte
        // copy still lists source-scene elements — `translate_element_list`'s
        // contract.
        unsafe { translate_element_list(ec, stack.layers_raw() as *mut c_void) }?;
        // SAFETY: `anim` is that stack's own `ufbx_anim*` field, reached through
        // a `Mut` view, so the pointer addresses a live, write-capable slot in
        // arena-owned interior-mutable storage — the storage the shared
        // `&ScalarView` (`Cell`) writes through. The field's declared type is
        // `Ref<Anim>`, so the pun widens a non-null slot into a nullable one;
        // `translate_anim` only ever stores its non-null-checked push result, so
        // the slot keeps a valid `Ref`. Its byte copy still names the source
        // scene's anim — `translate_anim`'s contract.
        unsafe { translate_anim(ec, &*(stack.anim_raw() as *const ScalarView<*mut Anim>)) }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_layer, p_layer, ec->scene.anim_layers)`
    let anim_layers = ec.scene_view().anim_layers_view();
    for i_layer in 0..anim_layers.count() {
        // C: `ufbx_anim_layer *layer = *p_layer;`
        let layer: &View<AnimLayer> = anim_layers.at(i_layer);

        // SAFETY: `anim_values` is that layer's own `ufbx_element_list`, whose
        // byte copy still lists source-scene elements —
        // `translate_element_list`'s contract.
        unsafe { translate_element_list(ec, layer.anim_values_raw() as *mut c_void) }?;
        // `anim_props` is that layer's own list, whose byte copy still describes
        // the source scene's anim-prop run.
        let count = layer.anim_props_view().count();
        let props: *mut AnimProp = ec.result_view().push::<AnimProp>(count + 1);
        ufbxi_check_err!(ec.error_view(), !props.is_null(), "props");
        // C: `for (size_t i = 0; i < layer->anim_props.count; i++)` — one vouch
        // for the pushed run, then a safe walk over its first `count` slots.
        // SAFETY: `props` is the contiguous `anim_props.count + 1`-element
        // allocation just pushed (non-null, checked above), reached through
        // `*mut` — write-capable provenance for `Mut`; `from_raw_parts` admits
        // such a still-uninitialized run, and the loop below initializes every
        // slot it walks before reading it.
        let props_run = unsafe { SliceViewIter::<AnimProp>::from_raw_parts(props, count) };
        for (i, prop) in props_run.enumerate() {
            // C: `props[i] = layer->anim_props.data[i];` (struct assignment)
            // SAFETY: `i` is below the anim-prop count, so slot `i` is in bounds
            // of the source run; `prop` views the matching slot of the pushed
            // allocation, a separate one.
            unsafe {
                ptr::copy_nonoverlapping(layer.anim_props_view().data().add(i), prop.get(), 1);
            }
            // SAFETY: `props[i]` holds the copy made just above, whose
            // `element` and `anim_value` are non-null `Ref`s to source-scene
            // elements — `translate_element`'s contract — read by value and
            // written back in place.
            unsafe {
                *(prop.element_raw() as *mut *mut Element) =
                    translate_element(ec, prop.element().ptr() as *mut c_void);
                *(prop.anim_value_raw() as *mut *mut AnimValue) =
                    translate_element(ec, prop.anim_value().ptr() as *mut c_void) as *mut AnimValue;
            }
        }
        // Maintain NULL sentinel
        // SAFETY: `props` is the base of that same pushed run of
        // `anim_props.count + 1` elements, so the slot at index `count` is the
        // spare one in bounds, and one `AnimProp` worth of bytes fits in it.
        let props = unsafe {
            ptr::write_bytes(props.add(count) as *mut u8, 0, size_of::<AnimProp>());
            // The logical list excludes the zeroed trailing sentinel bytes.
            List::from_raw_parts(props, count)
        };
        // C: `layer->anim_props.data = props;` — the destination layer retargeted
        // at the translated anim-prop array, retaining its count and trailing
        // sentinel.
        layer.anim_props_view().set(props);
    }

    // C: `ufbxi_for_ptr_list(ufbx_pose, p_pose, ec->scene.poses)`
    let poses = ec.scene_view().poses_view();
    for i_pose in 0..poses.count() {
        // C: `ufbx_pose *pose = *p_pose;`
        let pose: &View<Pose> = poses.at(i_pose);

        // `bone_poses` is that pose's own list, whose byte copy still describes
        // the source scene's bone-pose run.
        let count = pose.bone_poses_view().count();
        let bones: *mut BonePose = ec.result_view().push::<BonePose>(count);
        ufbxi_check_err!(ec.error_view(), !bones.is_null(), "bones");
        // C: `for (size_t i = 0; i < pose->bone_poses.count; i++)` — one vouch
        // for the pushed run, then a safe walk.
        // SAFETY: `bones` is the contiguous `bone_poses.count`-element allocation
        // just pushed (non-null, checked above), reached through `*mut` —
        // write-capable provenance for `Mut`; `from_raw_parts` admits such a
        // still-uninitialized run, and the loop below initializes every slot
        // before reading it.
        let bones_run = unsafe { SliceViewIter::<BonePose>::from_raw_parts(bones, count) };
        for (i, bone) in bones_run.enumerate() {
            // C: `bones[i] = pose->bone_poses.data[i];` (struct assignment)
            // SAFETY: `i` is below the bone-pose count, so slot `i` is in bounds
            // of the source run; `bone` views the matching slot of the pushed
            // allocation, a separate one.
            unsafe {
                ptr::copy_nonoverlapping(pose.bone_poses_view().data().add(i), bone.get(), 1);
            }
            // SAFETY: `bones[i]` holds the copy made just above, whose
            // `bone_node` is a non-null `Ref` to a source-scene element —
            // `translate_element`'s contract — read by value and written back
            // in place.
            unsafe {
                *(bone.bone_node_raw() as *mut *mut UfbxNode) =
                    translate_element(ec, bone.bone_node().ptr() as *mut c_void) as *mut UfbxNode;
            }
        }
        // SAFETY: all `count` slots were initialized and translated above; the
        // result-buffer run remains stable with the evaluated scene.
        let bones = unsafe { List::from_raw_parts(bones, count) };
        // C: `pose->bone_poses.data = bones;` — the destination pose retargeted
        // at the translated bone-pose array, retaining its count.
        pose.bone_poses_view().set(bones);
    }

    // SAFETY: `anim` is a field of `ec`'s own live context struct, declared
    // `*mut Anim` and reached through a `Mut` view, so the pointer addresses a
    // live, write-capable `ufbx_anim*` slot in context-owned interior-mutable
    // storage — the storage the shared `&ScalarView` (`Cell`) writes through,
    // and a nullable slot already, so the reinterpret asserts no validity the
    // field does not have. It holds the anim `evaluate_scene` resolved out of
    // the source scene — `translate_anim`'s contract.
    unsafe { translate_anim(ec, &*(ec.anim_mut_ptr() as *const ScalarView<*mut Anim>)) }?;

    // C: `ufbxi_for_ptr_list(ufbx_anim_value, p_value, ec->scene.anim_values)`
    let anim_values = ec.scene_view().anim_values_view();
    for i in 0..anim_values.count() {
        // C: `ufbx_anim_value *value = *p_value;`
        let value: &View<AnimValue> = anim_values.at(i);
        // C: `value->curves[i] = (ufbx_anim_curve*)ufbxi_translate_element(...)`
        // — the byte copy left each slot naming a source-scene element, and the
        // translated pointer is stored back in place.
        // SAFETY: `translate_element` maps each (nullable) source-scene element
        // pointer to its destination copy under `ec`'s live context — its own
        // contract; slot reads/writes go through the view accessors.
        unsafe {
            value.set_curve_ref(
                0,
                opt_ref(translate_element(ec, value.curve_ptr(0) as *mut c_void) as *mut AnimCurve),
            );
            value.set_curve_ref(
                1,
                opt_ref(translate_element(ec, value.curve_ptr(1) as *mut c_void) as *mut AnimCurve),
            );
            value.set_curve_ref(
                2,
                opt_ref(translate_element(ec, value.curve_ptr(2) as *mut c_void) as *mut AnimCurve),
            );
        }
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
    let elements = ec.scene_view().elements_view();
    for i_elem in 0..elements.count() {
        // C: `ufbx_element *elem = *p_elem;`
        let elem: &View<Element> = elements.at(i_elem);
        let mut num_animated: usize = elem.props().num_animated();
        let mut num_override: usize = 0;

        // Setup the overrides for this element if found
        // SAFETY (this condition): `over` is only read when it has not reached
        // `over_end`, so it addresses a live entry of the anim's override run.
        while over != over_end && unsafe { (*over).element_id } == elem.element_id() {
            num_override += 1;
            // SAFETY: `over` is inside the override run, so `over + 1` is at most
            // one past its end.
            over = unsafe { over.add(1) };
        }

        num_animated += num_override;
        if num_animated == 0 {
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
                elem.as_ptr(),
                ec.time(),
                props,
                num_animated,
                ec.opts_view().evaluate_flags(),
            )
        };
        elem.set_props(new_props);
        // C: `elem->props.defaults = &ec->src_scene.elements.data[elem->element_id]->props;`
        // Per the source-scene premise `elem`'s `element_id` indexes the source
        // element list, whose slot holds the source-scene element this one was
        // copied from — so its `props` field outlives the destination scene it is
        // stored into.
        // SAFETY: the store writes the source element's `props` address into
        // `elem`'s own `defaults` slot, read back as the bare pointer bits the
        // `Option<Ref<..>>` slot holds.
        unsafe {
            *(elem.props().defaults_raw() as *mut *const crate::generated::Props) = ec
                .src_scene_view()
                .elements_view()
                .at(elem.element_id() as usize)
                .props_ptr();
        }
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
        evaluate_skinning(
            ec.scene_view(),
            ec.error_view(),
            ec.result_view(),
            ec.tmp_view(),
            ec.time(),
            ec.opts_view().load_external_files() && ec.opts_view().evaluate_caches(),
            &cache_opts,
        )?;
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
    unsafe {
        (*imp).refcount.ator = ec.ator_result();
        (*imp).refcount.ator.error = ptr::null_mut();
    }

    // Copy retained buffers and translate the allocator struct to the one
    // contained within `ufbxi_scene_imp`
    // SAFETY: as above — the buffer is retargeted at the allocator embedded in
    // the same header, which the header keeps alive for as long as the buffer.
    unsafe {
        (*imp).refcount.buf = ec.take_result();
        (*imp).refcount.buf.ator = &raw mut (*imp).refcount.ator;
    }

    // SAFETY: `imp` is the live pushed header, whose `scene` and
    // `refcount.ator` were filled in just above.
    unsafe {
        (*imp).scene.metadata.result_memory_used = (*imp).refcount.ator.current_size;
        (*imp).scene.metadata.temp_memory_used = ec.ator_tmp_view().current_size();
        (*imp).scene.metadata.result_allocs = (*imp).refcount.ator.num_allocs;
        (*imp).scene.metadata.temp_allocs = ec.ator_tmp_view().num_allocs();
    }

    // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, imp->scene.elements)`
    // SAFETY: `imp` is the live pushed header, so addressing its `scene` field is
    // in bounds of that allocation (no read happens).
    let imp_scene_ptr: *mut Scene = unsafe { &raw mut (*imp).scene };
    // The header's own scene copy as a view for the element walk below.
    // SAFETY: `imp` is the live pushed header holding the copy of the destination
    // scene, reached through `*mut` into the result buffer — write-capable
    // provenance for `Mut` — and its element list is the array pushed and filled
    // in above.
    let imp_scene: &crate::native::parse::SceneView =
        unsafe { crate::native::view::View::<Scene>::from_ptr(imp_scene_ptr) };
    let imp_elements = imp_scene.elements_view();
    for i in 0..imp_elements.count() {
        // C: `(*p_elem)->scene = &imp->scene;`
        let elem: &View<Element> = imp_elements.at(i);
        // SAFETY: the store writes into that element's own `scene` slot, as the
        // bare pointer bits the `Ref<Scene>` slot holds; the scene it is pointed
        // at is the header's own copy, which owns that element buffer and so
        // outlives it.
        unsafe { *(elem.scene_raw() as *mut *mut Scene) = imp_scene_ptr };
    }

    ec.set_scene_imp(imp);
    // SAFETY: `result` has just been moved into the retained imp. This write
    // only restores the moved-from context header's allocator back-pointer to
    // its still-live sibling allocator; the moved-from buffer is not operated
    // on or freed afterwards.
    unsafe { ec.result_view().set_ator(ec.ator_result_mut_ptr()) };

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
) -> Result<*mut Scene, Error> {
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

    // SAFETY: `scene` is the caller's live scene payload behind a `SceneImp`
    // handed out by this library (this `unsafe fn`'s contract), live for the
    // whole evaluation.
    ec.set_src_imp(unsafe { ImpHandle::<SceneImp>::from_payload(scene) }.as_ptr());
    // C: `ec->src_scene = *scene;` (struct assignment)
    // SAFETY: `scene` is the caller's live `ufbx_scene` (this `unsafe fn`'s
    // contract) and `ec.src_scene_mut_ptr()` is `ec`'s own `src_scene` field, a
    // distinct non-overlapping allocation.
    unsafe { ptr::copy_nonoverlapping(scene as *const Scene, ec.src_scene_mut_ptr(), 1) };
    ec.set_anim(if !anim.is_null() {
        anim as *mut Anim
    } else {
        // SAFETY: `scene` is the caller's live scene; its `anim` slot is read
        // as bare pointer bits (`ref_ptr`), NOT as a `Ref`, because the
        // unchecked default-anim push at load (`ufbxi_push_zero`, no
        // `ufbxi_check`) can leave it NULL — C copies the NULL along here.
        unsafe { ref_ptr(&raw const (*scene).anim) }
    });
    ec.set_time(time);

    init_ator(
        ec.error_mut_ptr(),
        ec.ator_tmp_view(),
        Some(ec.opts_view().temp_allocator_view()),
        c"temp",
    );
    init_ator(
        ec.error_mut_ptr(),
        ec.ator_result_view(),
        Some(ec.opts_view().result_allocator_view()),
        c"result",
    );

    // SAFETY: the empty evaluation buffers are context fields wired to their
    // initialized sibling allocators. Each allocator remains live until its
    // buffer is transferred or freed on the matching success/failure path.
    unsafe {
        ec.result_view().set_ator(ec.ator_result_mut_ptr());
        ec.tmp_view().set_ator(ec.ator_tmp_mut_ptr());
    }

    ec.result_view().set_unordered(true);
    ec.tmp_view().set_unordered(true);

    // SAFETY: `evaluate_imp` takes the same `&EvalContext` this fn was handed,
    // now fully initialized by the setup above.
    if unsafe { evaluate_imp(ec) }.is_ok() {
        buf_free(ec.tmp_view());
        // SAFETY: `ec`'s temp allocator is its own field, live for the borrow,
        // and this is the last use of it.
        unsafe { free_ator(ec.ator_tmp_view()) };
        // SAFETY: `evaluate_imp` succeeded, and its last act is storing the
        // retained `ufbxi_scene_imp` into `ec`, so `scene_imp()` is the live
        // result-buffer header whose own `scene` field is projected here.
        // (The success-path `clear_error` of the caller's slot lives in the
        // boundary shim — PORTING.md "Trailing `ufbx_error *error`".)
        Ok(unsafe { &raw mut (*ec.scene_imp()).scene })
    } else {
        // C copies the fixed error into the caller's slot; the `Result` shape
        // carries it by value instead (the shim owns the slot writes).
        let mut fixed: Error = Error::default();
        let fixed_view = crate::native::error::ErrorView::from_mut(&mut fixed);
        fix_error_type(ec.error_view(), b"Failed to evaluate\0", Some(fixed_view));
        buf_free(ec.tmp_view());
        buf_free(ec.result_view());
        // SAFETY: `ec`'s temp and result allocators are its own fields, live
        // for the borrow; the failure path discards the result, so this is the
        // last use of each.
        unsafe {
            free_ator(ec.ator_tmp_view());
            free_ator(ec.ator_result_view());
        }
        Err(fixed)
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

// Typed interior-mutable view over `CreateAnimContext.opts`. Non-Copy
// list fields recurse into `RawListView`; addr-of fields use `_ptr` getters.
pub(crate) type AnimOptsView = crate::native::view::View<RawAnimOpts>;

impl AnimOptsView {
    #[inline(always)]
    pub(crate) fn ignore_connections(&self) -> bool {
        view_read!(self, ignore_connections)
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
        view_raw_const!(self, prop_overrides)
    }
}

// Mode-generic nested views over the allocator descriptors: `init_ator` only
// reads them, so the accessor serves a `Mut` context field and a `Const`
// boundary mint alike.
impl<M: crate::native::view::Mode> View<RawAnimOpts, M> {
    #[inline(always)]
    pub(crate) fn result_allocator_view(&self) -> &View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, result_allocator)
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
        view_read!(self, ator_result)
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

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawAnimOpts {
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

    // `anim` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn anim_mut_ptr(&self) -> *mut Anim {
        view_raw_mut!(self, anim)
    }

    // `anim` — typed VIEW handle (reinterpret-in-place); accessors on the
    // generated `View<Anim, M>` surface.
    #[inline(always)]
    pub(crate) fn anim_view(&self) -> &View<Anim> {
        // SAFETY: reinterpret the `anim` field in place inside this context's
        // outer UnsafeCell; shared interior-mutable view, asserts no validity.
        unsafe { &*(&raw mut (*self.get()).anim as *mut View<Anim>) }
    }

    // `imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn imp(&self) -> *mut AnimImp {
        view_read!(self, imp)
    }

    #[inline(always)]
    pub(crate) fn set_imp(&self, imp: *mut AnimImp) {
        view_write!(self, imp, imp)
    }

    // `scene` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn scene(&self) -> *const Scene {
        view_read!(self, scene)
    }

    #[inline(always)]
    pub(crate) fn set_scene(&self, scene: *const Scene) {
        view_write!(self, scene, scene)
    }
}

// Typed view over one caller-supplied `ufbx_prop_override_desc`
// (`ac->opts.prop_overrides.data[i]`). The generator emits views for the public
// `ufbx_*` structs only, so the `Raw*` boundary twin carries hand accessors:
// one leaf each, mode-generic so the frozen `Const` mint over caller memory is
// served by the same surface.
impl<M: Mode> View<RawPropOverrideDesc, M> {
    #[inline(always)]
    pub(crate) fn element_id(&self) -> u32 {
        view_read_shared!(self, element_id)
    }
    #[inline(always)]
    pub(crate) fn prop_name_ptr(&self) -> *const crate::prelude::RawString {
        view_raw_shared!(self, prop_name)
    }
    #[inline(always)]
    pub(crate) fn value(&self) -> Vec4 {
        view_read_shared!(self, value)
    }
    #[inline(always)]
    pub(crate) fn value_str_ptr(&self) -> *const crate::prelude::RawString {
        view_raw_shared!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn value_int(&self) -> i64 {
        view_read_shared!(self, value_int)
    }
}

// ufbx.c:26498-26510 `ufbxi_check_string`
///
/// # Safety
/// `src` must hold one of the two shapes this entry accepts: `length !=
/// SIZE_MAX` with `data` readable for `length` bytes, or the `SIZE_MAX`
/// sentinel with `data` addressing a NUL-terminated run. `RawString` asserts
/// neither, which is why the source is viewed in that form: both obligations
/// are the caller's, and `dst` receives the normalized `String` pair.
#[inline(never)]
pub(crate) unsafe fn check_string(
    error: &crate::native::error::ErrorView,
    dst: &StringView,
    src: &View<crate::prelude::RawString, Const>,
) -> Result<(), Fail> {
    let length: usize = if src.length() != usize::MAX {
        src.length()
    } else {
        // SAFETY: `strlen` requires the NUL-terminated buffer the `SIZE_MAX`
        // sentinel promises — this `unsafe fn`'s contract.
        unsafe { strlen(src.data()) }
    };
    let data: *const u8 = if length != 0 {
        src.data()
    } else {
        EMPTY_CHAR.as_ptr()
    };
    if length > 0 {
        // SAFETY: `data` is the source string's own data pointer and `length` is
        // that string's length (or its `strlen`), so the whole span is readable
        // under either shape of this `unsafe fn`'s contract.
        let data_bytes = unsafe { crate::prelude::slice_from_ptr(data, length) };
        let valid_length: usize = utf8_valid_length(data_bytes);
        ufbxi_check_err_msg!(error, valid_length == length, "Invalid UTF-8");
    }

    // C: `dst->data = data; dst->length = length;` — publish the validated
    // descriptor as one complete value.
    dst.set(String::new_c(data, length));
    Ok(())
}

// ufbx.c:26512-26526 `ufbxi_push_anim_string`
#[inline(never)]
pub(crate) fn push_anim_string(ac: &CreateAnimContext, str_: &StringView) -> Result<(), Fail> {
    let length: usize = str_.length();
    if length > 0 {
        let copy: *mut u8 = ac.result_view().push::<u8>(length + 1);
        ufbxi_check_err!(ac.error_view(), !copy.is_null(), "copy");
        // C: `memcpy(copy, str->data, length);`
        // SAFETY: `str_.data()` covers `length` readable bytes (the view's leaf
        // discipline) and `copy` is the freshly pushed `length + 1` byte run, a
        // distinct non-overlapping result-buffer allocation.
        unsafe { ptr::copy_nonoverlapping(str_.data(), copy, length) };
        // C: `copy[str->length] = '\0';`
        // SAFETY: `str_.length()` is `length`, the last slot of the
        // `length + 1` byte run just pushed.
        unsafe { *copy.add(str_.length()) = b'\0' };
        str_.set(String::new_c(copy, length));
    } else {
        ufbx_assert!(str_.data() == EMPTY_CHAR.as_ptr());
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
    // SAFETY: the sort comparator contract guarantees `va` and `vb` each point
    // to a live `ufbx_prop_override` element of the array being sorted, so both
    // `_internal_key` fields are readable.
    if unsafe { (*a)._internal_key != (*b)._internal_key } {
        // SAFETY: as above.
        return unsafe { (*a)._internal_key < (*b)._internal_key };
    }
    // SAFETY: as above, reading each element's `prop_name` string by value;
    // `str_less` only compares the spans those strings describe.
    unsafe { str_less((*a).prop_name.as_bytes(), (*b).prop_name.as_bytes()) }
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
    // SAFETY: the sort comparator contract guarantees `va` and `vb` each point
    // to a live `ufbx_prop_override` element of the array being sorted, so both
    // `element_id` fields are readable.
    if unsafe { (*a).element_id != (*b).element_id } {
        // SAFETY: as above.
        return unsafe { (*a).element_id < (*b).element_id };
    }
    // SAFETY: as above, for the `_internal_key` fields.
    if unsafe { (*a)._internal_key != (*b)._internal_key } {
        // SAFETY: as above.
        return unsafe { (*a)._internal_key < (*b)._internal_key };
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
pub(crate) fn create_anim_imp(ac: &CreateAnimContext) -> Result<FinishedImp<AnimImp>, Fail> {
    let scene: *const Scene = ac.scene();
    let anim: &View<Anim> = ac.anim_view();

    // Initializing ac's own result allocator from ac's own error slot and ac's
    // own opts allocator descriptor, named by a `'static` NUL-terminated
    // literal.
    init_ator(
        ac.error_mut_ptr(),
        ac.ator_result_view(),
        Some(ac.opts_view().result_allocator_view()),
        c"result",
    );
    ac.result_view().set_unordered(true);
    // SAFETY: this empty result buffer and its initialized allocator are
    // sibling fields of the stable create-anim context; chunks and allocator
    // state are transferred together into the finished imp.
    unsafe { ac.result_view().set_ator(ac.ator_result_mut_ptr()) };

    anim.set_ignore_connections(ac.opts_view().ignore_connections());
    anim.set_custom(true);

    let num_layers: usize = ac.opts_view().layer_ids_view().count();
    anim.layers_view().set_count(num_layers);
    anim.layers_view()
        .set_data(ac.result_view().push_zero::<*mut AnimLayer>(num_layers) as *const Ref<AnimLayer>);
    ufbxi_check_err!(
        ac.error_view(),
        !anim.layers_view().data().is_null(),
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
        // fresh push on ac's own result buf. After the non-null check, that run
        // remains live with the finished animation.
        let override_layer_weights = unsafe {
            let data = ac.result_view().push_copy_raw::<Real>(
                num_layers,
                ac.opts_view().override_layer_weights_view().data(),
            );
            ufbxi_check_err!(
                ac.error_view(),
                !data.is_null(),
                "anim->override_layer_weights.data"
            );
            List::from_raw_parts(data, num_layers)
        };
        anim.override_layer_weights_view()
            .set(override_layer_weights);
    }

    // C: `scene->anim_layers` — the scene is the caller's finished, immutable
    // scene, so the list field is read through a frozen view.
    // SAFETY: `scene` points at the live scene this anim is created for (ac
    // construction invariant); the projection covers only the `anim_layers`
    // field, which nothing writes for the duration of this call.
    let scene_anim_layers: &View<crate::prelude::RefList<AnimLayer>, Const> = unsafe {
        View::<crate::prelude::RefList<AnimLayer>, Const>::from_ptr(&raw const (*scene).anim_layers)
    };

    for i in 0..num_layers {
        // SAFETY: `i < num_layers` is the opts' own `layer_ids` count, so the
        // read stays inside the caller's `layer_ids` run.
        let index: u32 = unsafe { *ac.opts_view().layer_ids_view().data().add(i) };
        ufbxi_check_err_msg!(
            ac.error_view(),
            (index as usize) < scene_anim_layers.count(),
            "layer_ids out of bounds",
            "index < scene->anim_layers.count"
        );
        // C: `anim->layers.data[i] = ac->scene->anim_layers.data[index];`
        // SAFETY: `index` was just bounds-checked against the live scene's
        // `anim_layers`, and `i < num_layers` indexes the fresh layer push;
        // both element addresses are derived from their own list base.
        unsafe {
            *(anim.layers_view().data() as *mut *mut AnimLayer).add(i) =
                *(scene_anim_layers.data() as *const *mut AnimLayer).add(index as usize);
        }
    }

    // C: `ufbx_const_prop_override_desc_list prop_overrides = ac->opts.prop_overrides;`
    // SAFETY: reading the opts' own list header (pointer + count) by value.
    let prop_overrides: crate::prelude::RawList<RawPropOverrideDesc> =
        unsafe { ptr::read(ac.opts_view().prop_overrides_ptr()) };
    if prop_overrides.count > 0 {
        anim.prop_overrides_view().set_count(prop_overrides.count);
        anim.prop_overrides_view().set_data(
            ac.result_view()
                .push_zero::<PropOverride>(prop_overrides.count),
        );
        ufbxi_check_err!(
            ac.error_view(),
            !anim.prop_overrides_view().data().is_null(),
            "anim->prop_overrides.data"
        );

        for i in 0..prop_overrides.count {
            // C: `const ufbx_prop_override_desc *src = &prop_overrides.data[i];`
            // SAFETY: `i < prop_overrides.count` indexes the caller's own
            // override run from its list base; the frozen view covers that one
            // descriptor, which nothing writes during this call.
            let src: &View<RawPropOverrideDesc, Const> =
                unsafe { View::<RawPropOverrideDesc, Const>::from_ptr(prop_overrides.data.add(i)) };
            // C: `ufbx_prop_override *dst = &anim->prop_overrides.data[i];`
            let dst: &View<PropOverride> = anim.prop_overrides_view().at(i);

            dst.set_element_id(src.element_id());
            dst.set_value(src.value());
            dst.set_value_int(src.value_int());

            if dst.value().x != 0.0 && dst.value_int() == 0 {
                // C: `(int64_t)dst->value.x` — bare float→int cast; Rust `as`
                // saturates (PORTING.md integer-semantics table, accepted
                // divergence class).
                dst.set_value_int(dst.value().x as i64);
            } else if dst.value_int() != 0 && dst.value().x == 0.0 {
                // C assigns the single `.x` lane; the view surface carries no
                // sub-field setter for it, so the write goes through the
                // element's own `value` place.
                // SAFETY: `value_raw()` is this element view's own `Vec4`
                // field address, write-capable by the view's `Mut` mode.
                unsafe {
                    (*dst.value_raw()).x = dst.value_int() as Real;
                }
            }

            // C: `ufbxi_check_err(&ac->error, ufbxi_check_string(&ac->error, &dst->prop_name, &src->prop_name));`
            // SAFETY (both calls): each mint reinterprets one `RawString` field
            // address of the frozen descriptor view above, and that view's own
            // freeze is what keeps the member unwritten for the call.
            // `check_string`'s string-shape obligation is NOT discharged here:
            // `ufbx_prop_override_desc` (ufbx.h:4967-4979) documents no
            // `SIZE_MAX` sentinel for these members, unlike `ufbx_load_opts`
            // (ufbx.h:4794, 4919), yet ufbx.c:26500 `strlen`s that case
            // regardless — so the obligation is the one `create_anim`'s own
            // `*const RawAnimOpts` contract passes through to the API caller,
            // exactly as the C does.
            unsafe {
                check_string(
                    ac.error_view(),
                    dst.prop_name_view(),
                    View::<crate::prelude::RawString, Const>::from_ptr(src.prop_name_ptr()),
                )?;
                check_string(
                    ac.error_view(),
                    dst.value_str_view(),
                    View::<crate::prelude::RawString, Const>::from_ptr(src.value_str_ptr()),
                )?;
            }

            dst.set_internal_key(get_name_key(dst.prop_name_view().bytes()));
        }

        // Sort `anim->prop_overrides` first by `prop_name` only so we can deduplicate and
        // convert them to global strings in `ufbxi_strings[]` if possible.
        // SAFETY: the run is the fresh non-null push of `prop_overrides.count`
        // elements addressed from its own list base; the comparator takes no
        // user data, so the null `user` is what it expects.
        unsafe {
            unstable_sort(
                anim.prop_overrides_view().data() as *mut PropOverride as *mut c_void,
                anim.prop_overrides_view().count(),
                size_of::<PropOverride>(),
                prop_override_prop_name_less,
                ptr::null_mut(),
            );
        }

        // C: `const ufbx_string *global_str = ufbxi_strings, *global_end = global_str + ufbxi_arraycount(ufbxi_strings);`
        // The C cursor pair becomes an index pair over the table viewed as
        // frozen strings.
        // SAFETY: `STRINGS` is a `'static` table of valid `ufbx_string`s that
        // nothing ever writes, so its whole run reinterprets in place as
        // frozen string views.
        let globals: &[View<String, Const>] = unsafe {
            crate::prelude::slice_from_ptr(
                STRINGS.0.as_ptr() as *const View<String, Const>,
                STRINGS.0.len(),
            )
        };
        let mut global_str: usize = 0;
        let global_end: usize = globals.len();
        // C: `ufbx_string prev_name = { ufbxi_empty_char };`
        let mut prev_name: String = String::new_c(EMPTY_CHAR.as_ptr(), 0);
        // C: `ufbxi_for_list(ufbx_prop_override, over, anim->prop_overrides)`
        // SAFETY: the list run is the contiguous `push_zero` allocation checked
        // above — fully initialized by the copy loop — live and unmoved on ac's
        // own result buf, and write-capable as a result-buf allocation.
        let overs = unsafe {
            SliceViewIter::<PropOverride>::from_raw_parts(
                anim.prop_overrides_view().data() as *mut PropOverride,
                anim.prop_overrides_view().count(),
            )
        };
        for over in overs {
            if over.value_str_view().length() > 0 {
                // C: `ufbxi_check_err(&ac->error, ufbxi_push_anim_string(ac, &over->value_str));`
                push_anim_string(ac, over.value_str_view())?;
            }

            // SAFETY: `prev_name` is the empty-string literal or a `prop_name`
            // interned earlier in this walk — live and unwritten here.
            if str_equal(over.prop_name_view().bytes(), unsafe {
                prev_name.as_bytes()
            }) {
                over.set_prop_name(prev_name);
                continue;
            }

            while global_str != global_end
                && str_less(globals[global_str].bytes(), over.prop_name_view().bytes())
            {
                global_str += 1;
            }

            if global_str != global_end
                && str_equal(globals[global_str].bytes(), over.prop_name_view().bytes())
            {
                over.set_prop_name(STRINGS.0[global_str]);
            } else {
                push_anim_string(ac, over.prop_name_view())?;
            }

            prev_name = over.prop_name();
        }

        // Sort `anim->prop_overrides` to the actual order expected by evaluation.
        // SAFETY: as for the first sort — the same fresh run, addressed from
        // its own list base, with a user-data-free comparator.
        unsafe {
            unstable_sort(
                anim.prop_overrides_view().data() as *mut PropOverride as *mut c_void,
                anim.prop_overrides_view().count(),
                size_of::<PropOverride>(),
                prop_override_less,
                ptr::null_mut(),
            );
        }

        for i in 1..prop_overrides.count {
            // C: `const ufbx_prop_override *prev/next = &anim->prop_overrides.data[i - 1 / i];`
            let prev: &View<PropOverride> = anim.prop_overrides_view().at(i - 1);
            let next: &View<PropOverride> = anim.prop_overrides_view().at(i);
            if prev.element_id() == next.element_id()
                && prev.prop_name_view().data() == next.prop_name_view().data()
            {
                // SAFETY: the `%s` argument is `prev`'s interned,
                // NUL-terminated `prop_name.data`.
                unsafe {
                    ufbxi_fmt_err_info!(
                        Some(ac.error_view()),
                        "element %u prop \"%s\"",
                        prev.element_id(),
                        prev.prop_name_view().data()
                    );
                }
                ufbxi_fail_err_msg!(ac.error_view(), "Duplicate override", "Duplicate override");
            }
        }
    }

    if ac.opts_view().transform_overrides_view().count() > 0 {
        let count = ac.opts_view().transform_overrides_view().count();
        // SAFETY: the copy reads exactly the caller's own transform-override
        // run into a fresh push of the same length on ac's own result buf.
        // After the non-null check, that run remains live with the finished
        // animation.
        let transform_overrides = unsafe {
            let data = ac.result_view().push_copy_raw::<TransformOverride>(
                count,
                ac.opts_view().transform_overrides_view().data(),
            );
            ufbxi_check_err!(
                ac.error_view(),
                !data.is_null(),
                "anim->transform_overrides.data"
            );
            List::from_raw_parts(data, count)
        };
        anim.transform_overrides_view().set(transform_overrides);
        // SAFETY: sorting the fresh non-null run just checked, over its own
        // count and from its own list base; the comparator takes no user data.
        unsafe {
            unstable_sort(
                anim.transform_overrides_view().data() as *mut TransformOverride as *mut c_void,
                anim.transform_overrides_view().count(),
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
    let finished_imp = unsafe {
        finish_imp(
            ac.imp(),
            ImpHandle::<SceneImp>::from_payload(scene as *mut Scene).refcount_ptr(),
            ac.anim_mut_ptr(),
            ac.ator_result(),
            ac.take_result(),
        )
    };

    Ok(finished_imp)
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

// SAFETY: `#[repr(C)]` with `refcount` leading, `BAKED_ANIM_IMP_MAGIC` is the
// magic `ufbxi_get_imp(ufbxi_baked_anim_imp, ...)` users check, `Payload` is
// the public struct at the pinned offset, and `header_parts` projects the two
// named fields of the passed `imp`. Recovery-only: baked anims are finalized
// manually (the C statement group is interleaved with the metadata writes in
// `bake_anim_imp`), not through `finish_imp`.
unsafe impl crate::native::parse::ImpRecover for BakedAnimImp {
    type Payload = BakedAnim;
    const MAGIC: u32 = crate::native::allocator::BAKED_ANIM_IMP_MAGIC;

    #[inline(always)]
    unsafe fn header_parts(imp: *mut Self) -> (*mut Refcount, *mut u32) {
        // SAFETY: the caller vouches `imp` addresses a live `BakedAnimImp`, so
        // these field projections stay inside that allocation.
        unsafe { (&raw mut (*imp).refcount, &raw mut (*imp).magic) }
    }
}

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

#[cfg(feature = "baking")]
impl BakeTimeList {
    /// Construct a complete mutable bake-time list descriptor.
    ///
    /// # Safety
    /// When `count > 0`, `data` must address `count` contiguous, aligned, fully
    /// initialized `BakeTime` values that remain live and unmoved for every use
    /// of the returned descriptor. Null is permitted only when `count == 0`.
    #[inline(always)]
    pub(crate) const unsafe fn from_raw_parts(data: *mut BakeTime, count: usize) -> BakeTimeList {
        debug_assert!(count == 0 || !data.is_null());
        BakeTimeList { data, count }
    }
}

// Typed interior-mutable VIEW over a `BakeTimeList` field, reinterpreted in place
// (getters + complete-descriptor publication).
#[cfg(feature = "baking")]
pub(crate) type BakeTimeListView = crate::native::view::View<BakeTimeList>;

#[cfg(feature = "baking")]
#[cfg(feature = "baking")]
impl BakeTimeListView {
    #[inline(always)]
    pub(crate) fn count(&self) -> usize {
        view_read!(self, count)
    }
    #[inline(always)]
    pub(crate) fn data(&self) -> *mut BakeTime {
        view_read!(self, data)
    }
    #[inline(always)]
    pub(crate) fn set(&self, value: BakeTimeList) {
        self.write_value(value)
    }
}

// Typed interior-mutable VIEW over one `BakeTime` element of such a run
// (leaf getters + setters over the two Copy fields).
#[cfg(feature = "baking")]
pub(crate) type BakeTimeView = crate::native::view::View<BakeTime>;

#[cfg(feature = "baking")]
impl BakeTimeView {
    #[inline(always)]
    pub(crate) fn time(&self) -> f64 {
        view_read!(self, time)
    }
    #[inline(always)]
    pub(crate) fn flags(&self) -> u32 {
        view_read!(self, flags)
    }
    #[inline(always)]
    pub(crate) fn set_time(&self, time: f64) {
        view_write!(self, time, time)
    }
    #[inline(always)]
    pub(crate) fn set_flags(&self, flags: u32) {
        view_write!(self, flags, flags)
    }
    /// The C `ufbxi_bake_time` value copy (`t = times[i]`), assembled from the
    /// two leaf reads.
    #[inline(always)]
    pub(crate) fn value(&self) -> BakeTime {
        BakeTime {
            time: self.time(),
            flags: self.flags(),
        }
    }
    /// The C `ufbxi_bake_time` value assignment (`times[i] = t`), as the two
    /// leaf writes.
    #[inline(always)]
    pub(crate) fn set_value(&self, value: BakeTime) {
        self.set_time(value.time);
        self.set_flags(value.flags);
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

// Typed interior-mutable view over the `opts` field of `BakeContext`, reinterpreted in
// place. The generated ABI-fixed `RawBakeOpts` is the inner storage;
// `MaybeUninit` makes forming `&BakeOptsView` assert no validity — each leaf getter
// asserts only the field it reads.
#[cfg(feature = "baking")]
pub(crate) type BakeOptsView = crate::native::view::View<RawBakeOpts>;

#[cfg(feature = "baking")]
impl BakeOptsView {
    #[inline(always)]
    pub(crate) fn trim_start_time(&self) -> bool {
        view_read!(self, trim_start_time)
    }

    #[inline(always)]
    pub(crate) fn resample_rate(&self) -> f64 {
        view_read!(self, resample_rate)
    }

    #[inline(always)]
    pub(crate) fn minimum_sample_rate(&self) -> f64 {
        view_read!(self, minimum_sample_rate)
    }

    #[inline(always)]
    pub(crate) fn maximum_sample_rate(&self) -> f64 {
        view_read!(self, maximum_sample_rate)
    }

    #[inline(always)]
    pub(crate) fn bake_transform_props(&self) -> bool {
        view_read!(self, bake_transform_props)
    }

    #[inline(always)]
    pub(crate) fn skip_node_transforms(&self) -> bool {
        view_read!(self, skip_node_transforms)
    }

    #[inline(always)]
    pub(crate) fn no_resample_rotation(&self) -> bool {
        view_read!(self, no_resample_rotation)
    }

    #[inline(always)]
    pub(crate) fn ignore_layer_weight_animation(&self) -> bool {
        view_read!(self, ignore_layer_weight_animation)
    }

    #[inline(always)]
    pub(crate) fn max_keyframe_segments(&self) -> usize {
        view_read!(self, max_keyframe_segments)
    }

    #[inline(always)]
    pub(crate) fn step_handling(&self) -> BakeStepHandling {
        view_read!(self, step_handling)
    }

    #[inline(always)]
    pub(crate) fn step_custom_duration(&self) -> f64 {
        view_read!(self, step_custom_duration)
    }

    #[inline(always)]
    pub(crate) fn step_custom_epsilon(&self) -> f64 {
        view_read!(self, step_custom_epsilon)
    }

    #[inline(always)]
    pub(crate) fn evaluate_flags(&self) -> u32 {
        view_read!(self, evaluate_flags)
    }

    #[inline(always)]
    pub(crate) fn key_reduction_enabled(&self) -> bool {
        view_read!(self, key_reduction_enabled)
    }

    #[inline(always)]
    pub(crate) fn key_reduction_rotation(&self) -> bool {
        view_read!(self, key_reduction_rotation)
    }

    #[inline(always)]
    pub(crate) fn key_reduction_threshold(&self) -> f64 {
        view_read!(self, key_reduction_threshold)
    }

    #[inline(always)]
    pub(crate) fn key_reduction_passes(&self) -> usize {
        view_read!(self, key_reduction_passes)
    }

    #[inline(always)]
    pub(crate) fn set_resample_rate(&self, resample_rate: f64) {
        view_write!(self, resample_rate, resample_rate)
    }

    #[inline(always)]
    pub(crate) fn set_minimum_sample_rate(&self, minimum_sample_rate: f64) {
        view_write!(self, minimum_sample_rate, minimum_sample_rate)
    }

    #[inline(always)]
    pub(crate) fn set_max_keyframe_segments(&self, max_keyframe_segments: usize) {
        view_write!(self, max_keyframe_segments, max_keyframe_segments)
    }

    #[inline(always)]
    pub(crate) fn set_key_reduction_threshold(&self, key_reduction_threshold: f64) {
        view_write!(self, key_reduction_threshold, key_reduction_threshold)
    }

    #[inline(always)]
    pub(crate) fn set_key_reduction_passes(&self, key_reduction_passes: usize) {
        view_write!(self, key_reduction_passes, key_reduction_passes)
    }
}

// Mode-generic nested views over the allocator descriptors: `init_ator` only
// reads them, so the accessor serves a `Mut` context field and a `Const`
// boundary mint alike.
#[cfg(feature = "baking")]
impl<M: crate::native::view::Mode> View<RawBakeOpts, M> {
    #[inline(always)]
    pub(crate) fn temp_allocator_view(&self) -> &View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, temp_allocator)
    }
    #[inline(always)]
    pub(crate) fn result_allocator_view(&self) -> &View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, result_allocator)
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
        view_write!(self, result_memory_used, result_memory_used)
    }
    #[inline(always)]
    pub(crate) fn set_temp_memory_used(&self, temp_memory_used: usize) {
        view_write!(self, temp_memory_used, temp_memory_used)
    }
    #[inline(always)]
    pub(crate) fn set_result_allocs(&self, result_allocs: usize) {
        view_write!(self, result_allocs, result_allocs)
    }
    #[inline(always)]
    pub(crate) fn set_temp_allocs(&self, temp_allocs: usize) {
        view_write!(self, temp_allocs, temp_allocs)
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
        view_write!(self, key_time_min, key_time_min)
    }
    #[inline(always)]
    pub(crate) fn set_key_time_max(&self, key_time_max: f64) {
        view_write!(self, key_time_max, key_time_max)
    }
    #[inline(always)]
    pub(crate) fn set_playback_time_begin(&self, playback_time_begin: f64) {
        view_write!(self, playback_time_begin, playback_time_begin)
    }
    #[inline(always)]
    pub(crate) fn set_playback_time_end(&self, playback_time_end: f64) {
        view_write!(self, playback_time_end, playback_time_end)
    }
    #[inline(always)]
    pub(crate) fn set_playback_duration(&self, playback_duration: f64) {
        view_write!(self, playback_duration, playback_duration)
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
        view_raw_mut!(self, bake)
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

    // `tmp_arr_size` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_arr_size_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, tmp_arr_size)
    }

    // `tmp_arr` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_arr_mut_ptr(&self) -> *mut *mut u8 {
        view_raw_mut!(self, tmp_arr)
    }

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawBakeOpts {
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

    // `scene` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn scene(&self) -> *const Scene {
        view_read!(self, scene)
    }

    #[inline(always)]
    pub(crate) fn set_scene(&self, scene: *const Scene) {
        view_write!(self, scene, scene)
    }

    // `anim` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn anim(&self) -> *const Anim {
        view_read!(self, anim)
    }

    #[inline(always)]
    pub(crate) fn set_anim(&self, anim: *const Anim) {
        view_write!(self, anim, anim)
    }

    // `imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn imp(&self) -> *mut BakedAnimImp {
        view_read!(self, imp)
    }

    #[inline(always)]
    pub(crate) fn set_imp(&self, imp: *mut BakedAnimImp) {
        view_write!(self, imp, imp)
    }

    // `time_max` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn time_max(&self) -> f64 {
        view_read!(self, time_max)
    }

    #[inline(always)]
    pub(crate) fn set_time_max(&self, time_max: f64) {
        view_write!(self, time_max, time_max)
    }

    // `time_min` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn time_min(&self) -> f64 {
        view_read!(self, time_min)
    }

    #[inline(always)]
    pub(crate) fn set_time_min(&self, time_min: f64) {
        view_write!(self, time_min, time_min)
    }

    // `time_end` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn time_end(&self) -> f64 {
        view_read!(self, time_end)
    }

    #[inline(always)]
    pub(crate) fn set_time_end(&self, time_end: f64) {
        view_write!(self, time_end, time_end)
    }

    // `time_begin` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn time_begin(&self) -> f64 {
        view_read!(self, time_begin)
    }

    #[inline(always)]
    pub(crate) fn set_time_begin(&self, time_begin: f64) {
        view_write!(self, time_begin, time_begin)
    }

    // `ktime_offset` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn ktime_offset(&self) -> f64 {
        view_read!(self, ktime_offset)
    }

    #[inline(always)]
    pub(crate) fn set_ktime_offset(&self, ktime_offset: f64) {
        view_write!(self, ktime_offset, ktime_offset)
    }

    // `tmp_arr_size` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_arr_size(&self) -> usize {
        view_read!(self, tmp_arr_size)
    }

    // `tmp_arr` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_arr(&self) -> *mut u8 {
        view_read!(self, tmp_arr)
    }

    // `nodes_to_bake` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn nodes_to_bake(&self) -> *mut bool {
        view_read!(self, nodes_to_bake)
    }

    #[inline(always)]
    pub(crate) fn set_nodes_to_bake(&self, nodes_to_bake: *mut bool) {
        view_write!(self, nodes_to_bake, nodes_to_bake)
    }

    // `baked_nodes` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn baked_nodes(&self) -> *mut *mut BakedNode {
        view_read!(self, baked_nodes)
    }

    #[inline(always)]
    pub(crate) fn set_baked_nodes(&self, baked_nodes: *mut *mut BakedNode) {
        view_write!(self, baked_nodes, baked_nodes)
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
// contiguous `push_pop`-materialized `BakeProp` run; `Run<BakeProp>` carries
// that pair through the baking pipeline and yields `&BakePropView`, replacing
// raw `prop.add(1)` navigation with safe indexing, sub-runs, and iteration
// after one root construction vouch.
#[cfg(feature = "baking")]
#[cfg(feature = "baking")]
pub(crate) type BakePropView = crate::native::view::View<BakeProp>;

#[cfg(feature = "baking")]
#[cfg(feature = "baking")]
impl BakePropView {
    #[inline(always)]
    pub(crate) fn prop_name(&self) -> *const u8 {
        view_read!(self, prop_name)
    }

    #[inline(always)]
    pub(crate) fn anim_value(&self) -> *mut AnimValue {
        view_read!(self, anim_value)
    }

    #[inline(always)]
    pub(crate) fn element_id(&self) -> u32 {
        view_read!(self, element_id)
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
    // SAFETY: the sort comparator contract guarantees `va` and `vb` each point
    // to a live `ufbxi_bake_prop` element of the array being sorted, so both
    // `sort_id` fields are readable.
    if unsafe { (*a).sort_id != (*b).sort_id } {
        // SAFETY: as above.
        return unsafe { (*a).sort_id < (*b).sort_id };
    }
    // SAFETY: as above, for the `element_id` fields.
    if unsafe { (*a).element_id != (*b).element_id } {
        // SAFETY: as above.
        return unsafe { (*a).element_id < (*b).element_id };
    }
    // SAFETY: as above, comparing the two `prop_name` pointers themselves.
    if unsafe { (*a).prop_name != (*b).prop_name } {
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
pub(crate) fn bake_times(
    bc: &BakeContext,
    anim_value: &View<AnimValue, Const>,
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
        let Some(curve) = anim_value.curve_view(curve_ix) else {
            continue;
        };

        let keys = curve.keyframes_view();
        let num_keys: usize = keys.count();
        for key_ix in 0..num_keys {
            let a = keys.at(key_ix);
            let a_time: f64 = a.time();
            ufbxi_check_err!(
                bc.error_view(),
                bake_push_time(bc, a_time, key_flag),
                "ufbxi_bake_push_time(bc, a_time, key_flag)"
            );
            if key_ix + 1 >= num_keys {
                break;
            }
            // The `break` above establishes `key_ix + 1 < num_keys`, in bounds
            // of the `at` check.
            let b = keys.at(key_ix + 1);
            let b_time: f64 = b.time();

            // Skip fully flat sections
            if a.value() == b.value() && a.right().dy == 0.0f32 && b.left().dy == 0.0f32 {
                continue;
            }

            if a.interpolation() as u32 == Interpolation::ConstantPrev as u32 {
                ufbxi_check_err!(
                    bc.error_view(),
                    bake_push_time(bc, b_time, BakedKeyFlags::STEP_LEFT.raw()),
                    "ufbxi_bake_push_time(bc, b_time, UFBX_BAKED_KEY_STEP_LEFT)"
                );
            } else if a.interpolation() as u32 == Interpolation::ConstantNext as u32 {
                ufbxi_check_err!(
                    bc.error_view(),
                    bake_push_time(bc, a_time, BakedKeyFlags::STEP_RIGHT.raw()),
                    "ufbxi_bake_push_time(bc, a_time, UFBX_BAKED_KEY_STEP_RIGHT)"
                );
            } else if (resample_linear || a.interpolation() as u32 == Interpolation::Cubic as u32)
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
pub(crate) fn in_list(items: &[*const u8], item: *const u8) -> bool {
    for &candidate in items {
        if candidate == item {
            return true;
        }
    }
    false
}

// ufbx.c:26840-26845 `ufbxi_sort_bake_times`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) fn sort_bake_times(bc: &BakeContext, times: Run<'_, BakeTime>) -> Result<(), Fail> {
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
                bc.ator_tmp_view(),
                bc.tmp_arr_mut_ptr(),
                bc.tmp_arr_size_mut_ptr(),
                times.len().wrapping_mul(size_of::<BakeTime>()),
            )
        },
        "ufbxi_grow_array_size((&bc->ator_tmp), sizeof(**(&bc->tmp_arr)), (&bc->tmp_arr), (&bc->tmp_arr_size), (count * sizeof(ufbxi_bake_time)))"
    );
    // SAFETY: `times` carries the live writable input run, and the grow above
    // sized `bc.tmp_arr()` to hold the same number as the disjoint scratch run
    // `macro_stable_sort` needs. The comparator is handed pointers to two live
    // elements of that run or its scratch, so its derefs are in bounds.
    unsafe {
        macro_stable_sort::<BakeTime>(
            32,
            times.as_mut_ptr(),
            bc.tmp_arr() as *mut BakeTime,
            times.len(),
            |a, b| cmp_bake_time(*a, *b) < 0,
        )
    };
    Ok(())
}

// ufbx.c:26847-26968 `ufbxi_finalize_bake_times`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) fn finalize_bake_times(bc: &BakeContext, p_dst: &mut BakeTimeList) -> Result<(), Fail> {
    if bc.layer_weight_times_view().count() > 0 {
        ufbxi_check_err!(
            bc.error_view(),
            // SAFETY: `bc.tmp_times` is `bc`'s own buffer, and
            // `bc.layer_weight_times` is `bc`'s own list, whose base addresses
            // exactly the `count` elements copied from.
            !unsafe {
                bc.tmp_times_view().push_copy_raw::<BakeTime>(bc.layer_weight_times_view().count(),
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
    let times_ptr: *mut BakeTime = bc
        .tmp_prop_view()
        .push_pop::<BakeTime>(bc.tmp_times_view(), num_times);
    ufbxi_check_err!(bc.error_view(), !times_ptr.is_null(), "times");

    // SAFETY: `times_ptr` is the non-null (checked), initialized `num_times`
    // element run just popped into `bc.tmp_prop`; that buffer keeps it stable
    // for the rest of this function. The run retains its full allocation
    // length while the passes below shrink the logical `num_times` prefix.
    let times: Run<'_, BakeTime> = unsafe { Run::from_raw_parts(times_ptr, num_times) };
    sort_bake_times(bc, times)?;

    // Deduplicate times
    if num_times > 0 {
        let mut dst: usize = 0;
        let mut prev: BakeTime = times.at(0).value();
        let mut src: usize = 1;
        while src < num_times {
            let mut next: BakeTime = times.at(src).value();
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

            times.at(dst).set_value(prev);
            dst += 1;
            prev = next;
            src += 1;
        }
        times.at(dst).set_value(prev);
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
            let cur: BakeTime = times.at(src).value();
            let mut delta: f64 = math::INFINITY;

            let mut keep: bool = true;
            if (cur.flags & keep_flags) == 0 {
                if dst > 0 {
                    delta = cur.time - times.at(dst - 1).time();
                }
                if src + 1 < num_times {
                    delta = math::fmin(delta, times.at(src + 1).time() - cur.time);
                }
                if delta < min_dist {
                    keep = false;
                }
            }
            if keep {
                times.at(dst).set_value(cur);
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
            if (times.at(i).flags()
                & (BakedKeyFlags::STEP_LEFT.raw() | BakedKeyFlags::STEP_RIGHT.raw()))
                != 0
            {
                let sign: f64 = if (times.at(i).flags() & BakedKeyFlags::STEP_LEFT.raw()) != 0 {
                    -1.0
                } else {
                    1.0
                };
                let mut time: f64 = times.at(i).time() + sign * max_interval;
                if i > 0 {
                    time = math::fmax(time, times.at(i - 1).time());
                }
                if i + 1 < num_times {
                    time = math::fmin(time, times.at(i + 1).time());
                }
                times.at(i).set_time(time);
                times.at(i).set_flags(BakedKeyFlags::REDUCED.raw());
            }
        }

        // C: `ufbxi_bake_time prev_time = { -UFBX_INFINITY };`
        let mut prev_time: BakeTime = BakeTime {
            time: -math::INFINITY,
            flags: 0,
        };
        while src < num_times {
            let src_time: BakeTime = times.at(src).value();
            src += 1;

            let start_src: usize = src;
            // C: `ufbxi_bake_time next_time;` — both members assigned below.
            let mut next_time: BakeTime = BakeTime {
                time: 0.0,
                flags: 0,
            };
            next_time.time = math::ceil(src_time.time * sample_rate - epsilon) / sample_rate;
            next_time.flags = BakedKeyFlags::REDUCED.raw();
            while src < num_times && times.at(src).time() <= next_time.time + epsilon {
                src += 1;
            }

            if src != start_src || src_time.time - prev_time.time <= min_interval {
                prev_time = next_time;
            } else {
                prev_time = src_time;
            }

            if dst == 0 || prev_time.time > times.at(dst - 1).time() {
                times.at(dst).set_value(prev_time);
                dst += 1;
            }
        }

        num_times = dst;
    }

    if num_times > 0 {
        if times.at(0).time() < bc.time_min() {
            bc.set_time_min(times.at(0).time());
        }
        if times.at(num_times - 1).time() > bc.time_max() {
            bc.set_time_max(times.at(num_times - 1).time());
        }
    }

    p_dst.data = times.as_mut_ptr();
    p_dst.count = num_times;

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
pub(crate) fn postprocess_step(
    bc: &BakeContext,
    prev_time: f64,
    next_time: f64,
    p_time: &mut f64,
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
//
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) fn bake_postprocess_vec3(
    bc: &BakeContext,
    p_dst: &ListView<BakedVec3>,
    p_constant: &ScalarView<bool>,
    src: &mut [BakedVec3],
) -> Result<(), Fail> {
    if src.is_empty() {
        return Ok(());
    }

    let mut count = src.len();

    // Offset times
    if bc.ktime_offset() != 0.0 {
        // SAFETY: `bc.scene()` is the source `ufbx_scene` `bake_anim_imp` stored
        // into `bc`, live for the bake.
        let scale: f64 = unsafe { (*bc.scene()).metadata.ktime_second } as f64;
        let offset: f64 = bc.ktime_offset();
        for i in 0..count {
            src[i].time = math::rint(src[i].time * scale + offset) / scale;
        }
    }

    // Postprocess stepped tangents
    {
        let mut dst: usize = 0;
        let mut prev_time: f64 = src[0].time;
        for i in 0..count {
            let mut cur = BakedVec3 {
                time: src[i].time,
                value: src[i].value,
                flags: src[i].flags,
            };
            let next_time: f64 = if i + 1 < count {
                src[i + 1].time
            } else {
                math::INFINITY
            };
            let mut keep: bool = true;
            if (cur.flags.raw()
                & (BakedKeyFlags::STEP_LEFT.raw() | BakedKeyFlags::STEP_RIGHT.raw()))
                != 0
            {
                keep = postprocess_step(bc, prev_time, next_time, &mut cur.time, cur.flags);
            }
            if keep {
                // C: `src.data[dst] = cur; dst++; prev_time = cur.time;`
                let cur_time: f64 = cur.time;
                src[dst] = cur;
                dst += 1;
                prev_time = cur_time;
            }
        }
        count = dst;
    }

    if bc.opts_view().key_reduction_enabled() {
        let threshold: f64 =
            bc.opts_view().key_reduction_threshold() * bc.opts_view().key_reduction_threshold();
        for _pass in 0..bc.opts_view().key_reduction_passes() {
            let mut dst: usize = 1;
            let mut i: usize = 1;
            while i < count {
                let prev = BakedVec3 {
                    time: src[i - 1].time,
                    value: src[i - 1].value,
                    flags: src[i - 1].flags,
                };
                let cur = BakedVec3 {
                    time: src[i].time,
                    value: src[i].value,
                    flags: src[i].flags,
                };
                if i + 1 < count {
                    let next = BakedVec3 {
                        time: src[i + 1].time,
                        value: src[i + 1].value,
                        flags: src[i + 1].flags,
                    };
                    let delta: f64 = (cur.time - prev.time) / (next.time - prev.time);
                    let tmp: Vec3 = lerp3(prev.value, next.value, delta as Real);
                    let mut error: f64 = 0.0;
                    error += (as_f64!(tmp.x) - as_f64!(cur.value.x))
                        * (as_f64!(tmp.x) - as_f64!(cur.value.x));
                    error += (as_f64!(tmp.y) - as_f64!(cur.value.y))
                        * (as_f64!(tmp.y) - as_f64!(cur.value.y));
                    error += (as_f64!(tmp.z) - as_f64!(cur.value.z))
                        * (as_f64!(tmp.z) - as_f64!(cur.value.z));
                    if error <= threshold {
                        src[dst] = next;
                        i += 1;
                        dst += 1;
                        // C: `continue` — the `for` increment still runs.
                        i += 1;
                        continue;
                    }
                }

                src[dst] = cur;
                dst += 1;
                i += 1;
            }
            if dst == count {
                break;
            }
            count = dst;
        }
    }

    let mut constant: bool = true;
    let ref_: Vec3 = src[0].value;
    for i in 1..count {
        let v: Vec3 = src[i].value;
        if v.x != ref_.x || v.y != ref_.y || v.z != ref_.z {
            constant = false;
            break;
        }
    }
    p_constant.set(constant);

    let data = bc.result_view().push_copy_slice(&src[..count]);
    ufbxi_check_err!(bc.error_view(), !data.is_null(), "p_dst->data");
    // SAFETY: `data` is the fresh non-null result-buffer copy checked above and
    // remains live with the finished baked animation.
    let keys = unsafe { List::from_raw_parts(data, count) };
    p_dst.set(keys);

    Ok(())
}

// ufbx.c:27099-27199 `ufbxi_bake_postprocess_quat`
//
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) fn bake_postprocess_quat(
    bc: &BakeContext,
    p_dst: &ListView<BakedQuat>,
    p_constant: &ScalarView<bool>,
    src: &mut [BakedQuat],
) -> Result<(), Fail> {
    if src.is_empty() {
        return Ok(());
    }

    let mut count = src.len();

    // Offset times
    if bc.ktime_offset() != 0.0 {
        // SAFETY: `bc.scene()` is the source `ufbx_scene` `bake_anim_imp` stored
        // into `bc`, live for the bake.
        let scale: f64 = unsafe { (*bc.scene()).metadata.ktime_second } as f64;
        let offset: f64 = bc.ktime_offset();
        for i in 0..count {
            src[i].time = math::rint(src[i].time * scale + offset) / scale;
        }
    }

    // Postprocess stepped tangents
    {
        let mut dst: usize = 0;
        let mut prev_time: f64 = src[0].time;
        for i in 0..count {
            let mut cur = BakedQuat {
                time: src[i].time,
                value: src[i].value,
                flags: src[i].flags,
            };
            let next_time: f64 = if i + 1 < count {
                src[i + 1].time
            } else {
                math::INFINITY
            };
            let mut keep: bool = true;
            if (cur.flags.raw()
                & (BakedKeyFlags::STEP_LEFT.raw() | BakedKeyFlags::STEP_RIGHT.raw()))
                != 0
            {
                keep = postprocess_step(bc, prev_time, next_time, &mut cur.time, cur.flags);
            }
            if keep {
                prev_time = cur.time;
                src[dst] = cur;
                dst += 1;
            }
        }
        count = dst;
    }

    // Fix quaternion antipodality
    for i in 1..count {
        let value = src[i].value;
        let prev_value = src[i - 1].value;
        src[i].value = quat_fix_antipodal(value, prev_value);
    }

    if bc.opts_view().key_reduction_enabled() {
        let threshold: f64 =
            bc.opts_view().key_reduction_threshold() * bc.opts_view().key_reduction_threshold();
        for _pass in 0..bc.opts_view().key_reduction_passes() {
            let mut dst: usize = 1;
            let mut i: usize = 1;
            while i < count {
                let prev = BakedQuat {
                    time: src[i - 1].time,
                    value: src[i - 1].value,
                    flags: src[i - 1].flags,
                };
                let cur = BakedQuat {
                    time: src[i].time,
                    value: src[i].value,
                    flags: src[i].flags,
                };
                if i + 1 < count {
                    let next = BakedQuat {
                        time: src[i + 1].time,
                        value: src[i + 1].value,
                        flags: src[i + 1].flags,
                    };
                    let delta: f64 = (cur.time - prev.time) / (next.time - prev.time);
                    let mut error: f64 = 0.0;

                    if bc.opts_view().key_reduction_rotation() {
                        let tmp: Quat = quat_slerp(prev.value, next.value, delta as Real);
                        error += (as_f64!(tmp.x) - as_f64!(cur.value.x))
                            * (as_f64!(tmp.x) - as_f64!(cur.value.x));
                        error += (as_f64!(tmp.y) - as_f64!(cur.value.y))
                            * (as_f64!(tmp.y) - as_f64!(cur.value.y));
                        error += (as_f64!(tmp.z) - as_f64!(cur.value.z))
                            * (as_f64!(tmp.z) - as_f64!(cur.value.z));
                        error += (as_f64!(tmp.w) - as_f64!(cur.value.w))
                            * (as_f64!(tmp.w) - as_f64!(cur.value.w));
                    } else {
                        error += (as_f64!(prev.value.x) - as_f64!(cur.value.x))
                            * (as_f64!(prev.value.x) - as_f64!(cur.value.x));
                        error += (as_f64!(prev.value.y) - as_f64!(cur.value.y))
                            * (as_f64!(prev.value.y) - as_f64!(cur.value.y));
                        error += (as_f64!(prev.value.z) - as_f64!(cur.value.z))
                            * (as_f64!(prev.value.z) - as_f64!(cur.value.z));
                        error += (as_f64!(prev.value.w) - as_f64!(cur.value.w))
                            * (as_f64!(prev.value.w) - as_f64!(cur.value.w));
                        error += (as_f64!(next.value.x) - as_f64!(cur.value.x))
                            * (as_f64!(next.value.x) - as_f64!(cur.value.x));
                        error += (as_f64!(next.value.y) - as_f64!(cur.value.y))
                            * (as_f64!(next.value.y) - as_f64!(cur.value.y));
                        error += (as_f64!(next.value.z) - as_f64!(cur.value.z))
                            * (as_f64!(next.value.z) - as_f64!(cur.value.z));
                        error += (as_f64!(next.value.w) - as_f64!(cur.value.w))
                            * (as_f64!(next.value.w) - as_f64!(cur.value.w));
                        error *= 0.5;
                    }

                    if error <= threshold {
                        src[dst] = next;
                        i += 1;
                        dst += 1;
                        // C: `continue` — the `for` increment still runs.
                        i += 1;
                        continue;
                    }
                }

                src[dst] = cur;
                dst += 1;
                i += 1;
            }
            if dst == count {
                break;
            }
            count = dst;
        }
    }

    let mut constant: bool = true;
    let ref_: Quat = src[0].value;
    for i in 1..count {
        let v: Quat = src[i].value;
        if v.x != ref_.x || v.y != ref_.y || v.z != ref_.z || v.w != ref_.w {
            constant = false;
            break;
        }
    }
    p_constant.set(constant);

    let data = bc.result_view().push_copy_slice(&src[..count]);
    ufbxi_check_err!(bc.error_view(), !data.is_null(), "p_dst->data");
    // SAFETY: `data` is the fresh non-null result-buffer copy checked above and
    // remains live with the finished baked animation.
    let keys = unsafe { List::from_raw_parts(data, count) };
    p_dst.set(keys);

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
pub(crate) fn push_resampled_times<M: Mode>(
    bc: &BakeContext,
    p_keys: &View<List<BakedVec3>, M>,
) -> Result<(), Fail> {
    // C: `ufbx_baked_vec3_list keys = *p_keys;` — the by-value copy of the list
    // header collapses into the view; nothing writes the list while it is read.
    let keys: &View<List<BakedVec3>, M> = p_keys;

    let times: *mut BakeTime = bc.tmp_times_view().push::<BakeTime>(keys.count());
    ufbxi_check_err!(bc.error_view(), !times.is_null(), "times");
    for i in 0..keys.count() {
        let flags: BakedKeyFlags = keys.at(i).flags();
        let mut time: f64 = keys.at(i).time();
        if (flags.raw() & BakedKeyFlags::STEP_LEFT.raw()) != 0
            && i + 1 < keys.count()
            && (keys.at(i + 1).flags().raw() & BakedKeyFlags::STEP_KEY.raw()) != 0
        {
            time = keys.at(i + 1).time();
        } else if (flags.raw() & BakedKeyFlags::STEP_RIGHT.raw()) != 0
            && i > 0
            && (keys.at(i - 1).flags().raw() & BakedKeyFlags::STEP_KEY.raw()) != 0
        {
            time = keys.at(i - 1).time();
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

// Rust-port infra (not a ufbx.c type): reinterpret-in-place VIEWs over one
// baked key of a `ufbx_baked_vec3_list` / `ufbx_baked_quat_list`, so the merge
// loop below writes `keys.data[ix]` through `View<List<T>, Mut>::at` instead of
// walking the run with raw pointers. Leaf accessors only — the generator emits
// no view for these two public key structs.
#[cfg(feature = "baking")]
impl<M: Mode> View<BakedVec3, M> {
    #[inline(always)]
    pub(crate) fn time(&self) -> f64 {
        view_read_shared!(self, time)
    }
    #[inline(always)]
    pub(crate) fn flags(&self) -> BakedKeyFlags {
        view_read_shared!(self, flags)
    }
}

#[cfg(feature = "baking")]
impl View<BakedVec3, Mut> {
    #[inline(always)]
    pub(crate) fn set_time(&self, time: f64) {
        view_write!(self, time, time)
    }
    #[inline(always)]
    pub(crate) fn set_value(&self, value: Vec3) {
        view_write!(self, value, value)
    }
    #[inline(always)]
    pub(crate) fn set_flags(&self, flags: BakedKeyFlags) {
        view_write!(self, flags, flags)
    }
}

#[cfg(feature = "baking")]
impl View<BakedQuat, Mut> {
    #[inline(always)]
    pub(crate) fn set_time(&self, time: f64) {
        view_write!(self, time, time)
    }
    #[inline(always)]
    pub(crate) fn set_value(&self, value: Quat) {
        view_write!(self, value, value)
    }
    #[inline(always)]
    pub(crate) fn set_flags(&self, flags: BakedKeyFlags) {
        view_write!(self, flags, flags)
    }
}

// ufbx.c:27233-27490 `ufbxi_bake_node_imp`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn bake_node_imp(
    bc: &BakeContext,
    element_id: u32,
    props: Run<'_, BakeProp>,
) -> Result<(), Fail> {
    ufbx_assert!(!bc.baked_nodes().is_null() && !bc.nodes_to_bake().is_null());

    // C: `ufbx_node *node = (ufbx_node*)bc->scene->elements.data[element_id];`
    // SAFETY: `bc.scene()` is the source `ufbx_scene` `bake_anim_imp` stored into
    // `bc`, live for the bake; this `unsafe fn` requires `element_id` to be one
    // of that scene's element ids, so the slot is in bounds of `elements` and
    // holds a live element pointer, which C downcasts to the `ufbx_node` that
    // element header opens. The scene is not written during the bake, so the
    // read-only tag holds for the whole body.
    let node: &View<UfbxNode, Const> = unsafe {
        View::<UfbxNode, Const>::from_ptr(
            *((*bc.scene()).elements.data as *const *const UfbxNode).add(element_id as usize),
        )
    };
    ufbxi_dev_assert!(node.element().type_() as u32 == ElementType::Node as u32);

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
        let prop: Option<&View<Prop, Const>> = find_prop(node.element().props(), name_bytes);
        // C: `prop->value_vec3` — the `ufbx_prop` value union's 3-real view
        // over `value_vec4`.
        if prop.is_some_and(|prop| !is_vec3_zero(prop.value_vec3())) {
            complex_translation = true;
        }
        // C: `ufbxi_for(ufbxi_bake_prop, bprop, props, count)`
        for bprop in props.iter() {
            if bprop.prop_name() == name {
                complex_translation = true;
            }
        }
    }

    for i in 0..COMPLEX_ROTATION_PROPS.0.len() {
        let name: *const u8 = COMPLEX_ROTATION_PROPS.0[i];
        for bprop in props.iter() {
            if bprop.prop_name() == name {
                complex_rotation = true;
            }
        }
    }

    // C: `ufbxi_bake_time_list times_t, times_r, times_s;` — each is filled in
    // by the `ufbxi_finalize_bake_times` call below.
    // SAFETY: `BakeTimeList` is a pointer/length pair, and an all-zero pattern
    // (null pointer, zero count) is a valid inhabitant of it.
    let (mut times_t, mut times_r, mut times_s): (BakeTimeList, BakeTimeList, BakeTimeList) = unsafe {
        (
            MaybeUninit::zeroed().assume_init(),
            MaybeUninit::zeroed().assume_init(),
            MaybeUninit::zeroed().assume_init(),
        )
    };

    // Translation
    let mut resample_translation: bool = false;

    // Account for the _resampled_ scale helper scale animation to keep the
    // translation scale consistent with the parent scaling.
    let mut scale_helper_t: Option<&View<BakedNode>> = None;
    let mut constant_scale_t: Vec3 = Vec3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    // C: `node->parent`, the parent every later `node->parent->…` chain starts
    // from.
    let parent: Option<&View<UfbxNode, Const>> = node.parent().map(|parent| parent.view::<Const>());
    // C: `!node->is_scale_helper && node->parent && node->parent->scale_helper`
    // — a short-circuit chain, so each load only happens once the preceding
    // test passes.
    let parent_scale_helper: Option<&View<UfbxNode, Const>> = if !node.is_scale_helper() {
        parent
            .and_then(|parent| parent.scale_helper())
            .map(|scale_helper| scale_helper.view::<Const>())
    } else {
        None
    };
    if let Some(parent_scale_helper) = parent_scale_helper {
        // SAFETY: `parent_scale_helper` is a live scene node, so its `typed_id`
        // indexes `bc.baked_nodes()`, which `bake_anim` sizes with one slot per
        // node of the scene; a non-null slot holds the `ufbx_baked_node` already
        // baked for that helper, pushed onto `bc.tmp_nodes`.
        scale_helper_t = unsafe {
            let baked: *mut BakedNode = *bc
                .baked_nodes()
                .add(parent_scale_helper.element().typed_id() as usize);
            if baked.is_null() {
                None
            } else {
                Some(View::<BakedNode>::from_ptr(baked))
            }
        };
        if let Some(scale_helper_t) = scale_helper_t {
            if !scale_helper_t.constant_scale() {
                resample_translation = true;
            }
            push_resampled_times(bc, scale_helper_t.scale_keys_view())?;
        } else {
            constant_scale_t = parent_scale_helper.inherit_scale();
        }
    }

    if complex_translation {
        // C: `ufbxi_for(ufbxi_bake_prop, prop, props, count)`
        for prop in props.iter() {
            // Literally any transform related property can affect complex translation
            if in_list(&TRANSFORM_PROPS.0, prop.prop_name()) {
                let resample_linear: bool =
                    resample_translation || prop.prop_name() != sp::Lcl_Translation.as_ptr();
                let key_flag: u32 = if prop.prop_name() == sp::Lcl_Translation.as_ptr() {
                    BakedKeyFlags::KEYFRAME.raw()
                } else {
                    0
                };
                // SAFETY: `prop` is one of the live bake props, whose
                // `anim_value` is the `ufbx_anim_value` it was collected from.
                unsafe {
                    bake_times(
                        bc,
                        View::<AnimValue, Const>::from_ptr(prop.anim_value()),
                        resample_linear,
                        key_flag,
                    )
                }?;
            }
        }
    } else {
        for prop in props.iter() {
            if prop.prop_name() == sp::Lcl_Translation.as_ptr() {
                // SAFETY: `prop.anim_value()` is the live `ufbx_anim_value` this
                // bake prop was collected from.
                unsafe {
                    bake_times(
                        bc,
                        View::<AnimValue, Const>::from_ptr(prop.anim_value()),
                        resample_translation,
                        BakedKeyFlags::KEYFRAME.raw(),
                    )
                }?;
            }
        }
    }

    finalize_bake_times(bc, &mut times_t)?;

    // Rotation
    if complex_rotation {
        for prop in props.iter() {
            if in_list(&COMPLEX_ROTATION_SOURCES.0, prop.prop_name()) {
                let resample_linear: bool = !bc.opts_view().no_resample_rotation()
                    || prop.prop_name() != sp::Lcl_Rotation.as_ptr();
                let key_flag: u32 = if prop.prop_name() == sp::Lcl_Rotation.as_ptr() {
                    BakedKeyFlags::KEYFRAME.raw()
                } else {
                    0
                };
                // SAFETY: `prop.anim_value()` is the live `ufbx_anim_value` this
                // bake prop was collected from.
                unsafe {
                    bake_times(
                        bc,
                        View::<AnimValue, Const>::from_ptr(prop.anim_value()),
                        resample_linear,
                        key_flag,
                    )
                }?;
            }
        }
    } else {
        for prop in props.iter() {
            if prop.prop_name() == sp::Lcl_Rotation.as_ptr() {
                // SAFETY: `prop.anim_value()` is the live `ufbx_anim_value` this
                // bake prop was collected from.
                unsafe {
                    bake_times(
                        bc,
                        View::<AnimValue, Const>::from_ptr(prop.anim_value()),
                        !bc.opts_view().no_resample_rotation(),
                        BakedKeyFlags::KEYFRAME.raw(),
                    )
                }?;
            }
        }
    }
    finalize_bake_times(bc, &mut times_r)?;

    // Scaling
    let mut resample_scale: bool = false;

    // Account for the resampled scale
    let mut scale_helper_s: Option<&View<BakedNode>> = None;
    let mut constant_scale_s: Vec3 = Vec3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    // C: `node->is_scale_helper && node->parent && node->parent->inherit_scale_node
    // && node->parent->inherit_scale_node->scale_helper`, then
    // `ufbx_node *inherit_helper = node->parent->inherit_scale_node->scale_helper;`
    // — a short-circuit chain, so each load only happens once the preceding
    // test passes.
    let inherit_helper: Option<&View<UfbxNode, Const>> = if node.is_scale_helper() {
        parent
            .and_then(|parent| parent.inherit_scale_node())
            .map(|inherit_scale_node| inherit_scale_node.view::<Const>())
            .and_then(|inherit_scale_node| inherit_scale_node.scale_helper())
            .map(|scale_helper| scale_helper.view::<Const>())
    } else {
        None
    };
    if let Some(inherit_helper) = inherit_helper {
        // SAFETY: `inherit_helper` is a live scene node, so its `typed_id`
        // indexes `bc.baked_nodes()`, which `bake_anim` sizes with one slot per
        // node of the scene; a non-null slot holds the `ufbx_baked_node` already
        // baked for that helper, pushed onto `bc.tmp_nodes`.
        scale_helper_s = unsafe {
            let baked: *mut BakedNode = *bc
                .baked_nodes()
                .add(inherit_helper.element().typed_id() as usize);
            if baked.is_null() {
                None
            } else {
                Some(View::<BakedNode>::from_ptr(baked))
            }
        };
        if let Some(scale_helper_s) = scale_helper_s {
            if !scale_helper_s.constant_scale() {
                resample_scale = true;
            }
            push_resampled_times(bc, scale_helper_s.scale_keys_view())?;
        } else {
            constant_scale_s = inherit_helper.local_transform().scale;
        }
    }

    {
        for prop in props.iter() {
            if prop.prop_name() == sp::Lcl_Scaling.as_ptr() {
                // SAFETY: `prop.anim_value()` is the live `ufbx_anim_value` this
                // bake prop was collected from.
                unsafe {
                    bake_times(
                        bc,
                        View::<AnimValue, Const>::from_ptr(prop.anim_value()),
                        resample_scale,
                        BakedKeyFlags::KEYFRAME.raw(),
                    )
                }?;
            }
        }
    }
    finalize_bake_times(bc, &mut times_s)?;

    // C: `ufbx_baked_vec3_list keys_t; ufbx_baked_quat_list keys_r; ufbx_baked_vec3_list keys_s;`
    // SAFETY: these lists are pointer/length pairs, and an all-zero pattern
    // (null pointer, zero count) is a valid inhabitant of each.
    let (mut keys_t, mut keys_r, mut keys_s): (List<BakedVec3>, List<BakedQuat>, List<BakedVec3>) = unsafe {
        (
            MaybeUninit::zeroed().assume_init(),
            MaybeUninit::zeroed().assume_init(),
            MaybeUninit::zeroed().assume_init(),
        )
    };

    keys_t.count = times_t.count;
    keys_t.data = bc.tmp_prop_view().push::<BakedVec3>(keys_t.count);
    ufbxi_check_err!(bc.error_view(), !keys_t.data.is_null(), "keys_t.data");

    keys_r.count = times_r.count;
    keys_r.data = bc.tmp_prop_view().push::<BakedQuat>(keys_r.count);
    ufbxi_check_err!(bc.error_view(), !keys_r.data.is_null(), "keys_r.data");

    keys_s.count = times_s.count;
    keys_s.data = bc.tmp_prop_view().push::<BakedVec3>(keys_s.count);
    ufbxi_check_err!(bc.error_view(), !keys_s.data.is_null(), "keys_s.data");

    // C indexes `keys_t.data[ix_t]` etc. below; each list view addresses its own
    // local list, so `at(ix)` is the bounds-checked form of that indexing.
    // Each list is a local `ufbx_baked_*_list` whose `data` is the non-null
    // (checked) run of `count` slots just pushed onto `bc.tmp_prop`.
    let (keys_t_view, keys_r_view, keys_s_view) = (
        View::<List<BakedVec3>>::from_mut(&mut keys_t),
        View::<List<BakedQuat>>::from_mut(&mut keys_r),
        View::<List<BakedVec3>>::from_mut(&mut keys_s),
    );

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
        // SAFETY: `bc.anim()` is the live anim stored into `bc`, and `node`
        // views a live scene element. Both remain frozen for this evaluation.
        let mut transform: Transform = unsafe {
            evaluate_transform_flags(
                Some(View::<Anim, Const>::from_ptr(bc.anim())),
                Some(View::<UfbxNode, Const>::from_ptr(node.as_ptr())),
                eval_time,
                flags,
            )
        };

        if (flags & TransformFlags::INCLUDE_TRANSLATION.raw()) != 0 {
            if let Some(scale_helper_t) = scale_helper_t {
                let scale: Vec3 =
                    evaluate_baked_vec3_slice(scale_helper_t.scale_keys().as_ref(), eval_time);
                transform.translation.x *= scale.x;
                transform.translation.y *= scale.y;
                transform.translation.z *= scale.z;
            }

            transform.translation.x *= constant_scale_t.x;
            transform.translation.y *= constant_scale_t.y;
            transform.translation.z *= constant_scale_t.z;

            // C: `keys_t.data[ix_t]` — `INCLUDE_TRANSLATION` is only set where
            // `ix_t` was found below `times_t.count`, which is `keys_t.count`.
            let key: &View<BakedVec3> = keys_t_view.at(ix_t);
            key.set_time(bake_time.time);
            key.set_value(transform.translation);
            key.set_flags(BakedKeyFlags::from_raw(bake_time.flags | flags_t));
            ix_t += 1;
        }
        if (flags & TransformFlags::INCLUDE_ROTATION.raw()) != 0 {
            // C: `keys_r.data[ix_r]` — `INCLUDE_ROTATION` is only set where
            // `ix_r` was found below `times_r.count`, which is `keys_r.count`.
            let key: &View<BakedQuat> = keys_r_view.at(ix_r);
            key.set_time(bake_time.time);
            key.set_value(transform.rotation);
            key.set_flags(BakedKeyFlags::from_raw(bake_time.flags | flags_r));
            ix_r += 1;
        }
        if (flags & TransformFlags::INCLUDE_SCALE.raw()) != 0 {
            if let Some(scale_helper_s) = scale_helper_s {
                let scale: Vec3 =
                    evaluate_baked_vec3_slice(scale_helper_s.scale_keys().as_ref(), eval_time);
                transform.scale.x *= scale.x;
                transform.scale.y *= scale.y;
                transform.scale.z *= scale.z;
            }

            transform.scale.x *= constant_scale_s.x;
            transform.scale.y *= constant_scale_s.y;
            transform.scale.z *= constant_scale_s.z;

            // C: `keys_s.data[ix_s]` — `INCLUDE_SCALE` is only set where `ix_s`
            // was found below `times_s.count`, which is `keys_s.count`.
            let key: &View<BakedVec3> = keys_s_view.at(ix_s);
            key.set_time(bake_time.time);
            key.set_value(transform.scale);
            key.set_flags(BakedKeyFlags::from_raw(bake_time.flags | flags_s));
            ix_s += 1;
        }
    }

    let baked_node: *mut BakedNode = bc.tmp_nodes_view().push_zero::<BakedNode>(1);
    ufbxi_check_err!(bc.error_view(), !baked_node.is_null(), "baked_node");

    // SAFETY: `baked_node` is the non-null (checked) zeroed `ufbx_baked_node`
    // just pushed onto `bc.tmp_nodes`, arena memory with write-capable
    // provenance that stays put for the bake.
    let baked_node_view: &View<BakedNode> = unsafe { View::<BakedNode>::from_ptr(baked_node) };

    baked_node_view.set_element_id(node.element().element_id());
    baked_node_view.set_typed_id(node.element().typed_id());
    // SAFETY: the projection addresses the pushed baked node's own
    // `constant_translation` field, reached through a `Mut` view, so it is a
    // live write-capable `bool` slot in arena-owned interior-mutable storage —
    // the storage the shared `&ScalarView` (`Cell`) writes through.
    let constant_translation: &ScalarView<bool> =
        unsafe { &*(baked_node_view.constant_translation_raw() as *const ScalarView<bool>) };
    // SAFETY: as above, for the pushed baked node's own `constant_scale` field.
    let constant_scale: &ScalarView<bool> =
        unsafe { &*(baked_node_view.constant_scale_raw() as *const ScalarView<bool>) };
    // SAFETY: `keys_t` describes a live run of `keys_t.count == ix_t`
    // initialized keys pushed onto `bc.tmp_prop`; no other access to that run
    // overlaps the mutable slice for the duration of the call.
    unsafe {
        let keys_t = core::slice::from_raw_parts_mut(keys_t.data as *mut BakedVec3, keys_t.count);
        bake_postprocess_vec3(
            bc,
            baked_node_view.translation_keys_view(),
            constant_translation,
            keys_t,
        )
    }?;
    // SAFETY: the projection addresses the pushed baked node's own
    // `constant_rotation` field, reached through a `Mut` view, so it is a live
    // write-capable `bool` slot in arena-owned interior-mutable storage — the
    // storage the shared `&ScalarView` (`Cell`) writes through.
    let constant_rotation: &ScalarView<bool> =
        unsafe { &*(baked_node_view.constant_rotation_raw() as *const ScalarView<bool>) };
    // SAFETY: `keys_r` describes a live run of `keys_r.count == ix_r`
    // initialized keys pushed onto `bc.tmp_prop`; no other access to that run
    // overlaps the mutable slice for the duration of the call.
    unsafe {
        let keys_r = core::slice::from_raw_parts_mut(keys_r.data as *mut BakedQuat, keys_r.count);
        bake_postprocess_quat(
            bc,
            baked_node_view.rotation_keys_view(),
            constant_rotation,
            keys_r,
        )
    }?;
    // SAFETY: `keys_s` describes a live run of `keys_s.count == ix_s`
    // initialized keys pushed onto `bc.tmp_prop`; no other access to that run
    // overlaps the mutable slice for the duration of the call.
    unsafe {
        let keys_s = core::slice::from_raw_parts_mut(keys_s.data as *mut BakedVec3, keys_s.count);
        bake_postprocess_vec3(
            bc,
            baked_node_view.scale_keys_view(),
            constant_scale,
            keys_s,
        )
    }?;

    // SAFETY: `node` views a live scene node, so its `typed_id` indexes
    // `bc.baked_nodes()`, which `bake_anim` sizes with one slot per scene node.
    unsafe { *bc.baked_nodes().add(node.element().typed_id() as usize) = baked_node };

    buf_clear(bc.tmp_prop_view());

    // If this node is a scale helper, make sure to bake its siblings and
    // potentially their scale helpers if they are not a part of the animation.
    if node.is_scale_helper() {
        ufbx_assert!(parent.is_some());
        // C: `ufbxi_for_ptr_list(ufbx_node, p_child, node->parent->children)` —
        // a scale helper always has a parent (asserted above), and the walk is
        // over that parent's own children ref list.
        if let Some(parent) = parent {
            let children = parent.children_view();
            for i in 0..children.count() {
                let child: &View<UfbxNode, Const> = children.at(i);
                if child.as_ptr() == node.as_ptr() {
                    continue;
                }
                // SAFETY (this condition): `child` is a live scene node, so its
                // `typed_id` indexes `bc.nodes_to_bake()`, which `bake_anim`
                // sizes with one slot per scene node.
                if !unsafe { *bc.nodes_to_bake().add(child.element().typed_id() as usize) } {
                    // SAFETY: as above.
                    unsafe { *bc.nodes_to_bake().add(child.element().typed_id() as usize) = true };
                    ufbxi_check_err!(
                        bc.error_view(),
                        !bc.tmp_bake_stack_view()
                            .push_copy_ref::<u32>(&child.element().element_id())
                            .is_null(),
                        "((uint32_t*)ufbxi_push_size_copy((&bc->tmp_bake_stack), sizeof(uint32_t), (1), (&child->element_id)))"
                    );
                }
                // C: `child->inherit_scale_node && child->inherit_scale_node->scale_helper && child->scale_helper
                // && bc->nodes_to_bake[child->inherit_scale_node->scale_helper->typed_id]`
                // — one short-circuit chain, nested here because each later
                // term needs the pointer the earlier one produced.
                let child_inherit_scale_helper: Option<&View<UfbxNode, Const>> = child
                    .inherit_scale_node()
                    .map(|inherit_scale_node| inherit_scale_node.view::<Const>())
                    .and_then(|inherit_scale_node| inherit_scale_node.scale_helper())
                    .map(|scale_helper| scale_helper.view::<Const>());
                let child_scale_helper: Option<&View<UfbxNode, Const>> = child
                    .scale_helper()
                    .map(|scale_helper| scale_helper.view::<Const>());
                if let (Some(child_inherit_scale_helper), Some(child_scale_helper)) =
                    (child_inherit_scale_helper, child_scale_helper)
                {
                    // SAFETY: `child_inherit_scale_helper` is a live scene node,
                    // so its `typed_id` indexes `bc.nodes_to_bake()`.
                    if unsafe {
                        *bc.nodes_to_bake()
                            .add(child_inherit_scale_helper.element().typed_id() as usize)
                    } {
                        // SAFETY: as above; the same `typed_id` indexes the
                        // equally sized `bc.baked_nodes()`.
                        ufbx_assert!(!unsafe {
                            *bc.baked_nodes()
                                .add(child_inherit_scale_helper.element().typed_id() as usize)
                        }
                        .is_null());
                        // SAFETY (this condition): `child_scale_helper` is a live
                        // scene node, so its `typed_id` indexes
                        // `bc.nodes_to_bake()`.
                        if !unsafe {
                            *bc.nodes_to_bake()
                                .add(child_scale_helper.element().typed_id() as usize)
                        } {
                            // SAFETY: as above.
                            unsafe {
                                *bc.nodes_to_bake()
                                    .add(child_scale_helper.element().typed_id() as usize) = true
                            };
                            ufbxi_check_err!(
                                bc.error_view(),
                                !bc.tmp_bake_stack_view()
                                    .push_copy_ref::<u32>(
                                        &child_scale_helper.element().element_id()
                                    )
                                    .is_null(),
                                "((uint32_t*)ufbxi_push_size_copy((&bc->tmp_bake_stack), sizeof(uint32_t), (1), (&child->scale_helper->element_id)))"
                            );
                        }
                    }
                }
            }
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
    props: Run<'_, BakeProp>,
) -> Result<(), Fail> {
    // SAFETY: `element_id` is this `unsafe fn`'s own scene-element parameter;
    // `props` already carries the bake-prop run contract.
    unsafe { bake_node_imp(bc, element_id, props) }?;

    // Baking a node may cause further nodes to be baked, so keep going
    // until all dependencies are baked.
    while bc.tmp_bake_stack_view().num_items() > 0 {
        let mut child_id: u32 = 0;
        // SAFETY: `bc.tmp_bake_stack` is `bc`'s own buffer, holding at least one
        // `uint32_t` (the loop condition), and `child_id` is a live local slot
        // for the popped value.
        unsafe { pop::<u32>(bc.tmp_bake_stack_view(), 1, &raw mut child_id) };
        // SAFETY: `child_id` was pushed as a scene element id by
        // `bake_node_imp`; this dependency has no explicit bake properties.
        unsafe { bake_node_imp(bc, child_id, Run::empty()) }?;
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
    props: Run<'_, BakeProp>,
) -> Result<(), Fail> {
    // C: `ufbxi_for(ufbxi_bake_prop, prop, props, count)`
    for prop in props.iter() {
        // SAFETY: `prop.anim_value()` is the live `ufbx_anim_value` this bake
        // prop was collected from.
        unsafe {
            bake_times(
                bc,
                View::<AnimValue, Const>::from_ptr(prop.anim_value()),
                false,
                BakedKeyFlags::KEYFRAME.raw(),
            )
        }?;
    }

    // C: `ufbxi_bake_time_list times;`
    // SAFETY: `BakeTimeList` is a pointer/length pair, and an all-zero pattern
    // (null pointer, zero count) is a valid inhabitant of it.
    let mut times: BakeTimeList = unsafe { MaybeUninit::zeroed().assume_init() };
    finalize_bake_times(bc, &mut times)?;

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
        // `bc`, and `element` is the caller's live scene element (this
        // `unsafe fn`'s contract). Both are read-only during evaluation.
        let prop: Prop = unsafe {
            evaluate_prop_flags_len_view(
                View::<Anim, Const>::from_ptr(bc.anim()),
                View::<Element, Const>::from_ptr(element),
                EvalPropName::from_string(View::<String, Const>::from_ref(&name)),
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
    // `strlen` stays inside it and the `length + 1` bytes `push_copy` reads are
    // its own bytes plus the terminator; `bc.result` is `bc`'s own buffer.
    unsafe {
        (*baked_prop).name.length = strlen(prop_name);
        (*baked_prop).name.data = bc
            .result_view()
            .push_copy_raw::<u8>((*baked_prop).name.length + 1, prop_name);
    }
    ufbxi_check_err!(
        bc.error_view(),
        // SAFETY: `baked_prop` is the live pushed prop, just written.
        !unsafe { (*baked_prop).name.data }.is_null(),
        "baked_prop->name.data"
    );

    // SAFETY: `baked_prop` is the non-null (checked) zeroed `ufbx_baked_prop`
    // pushed onto `bc.tmp_props`, arena memory with write-capable provenance
    // that stays put for the bake.
    let baked_prop_view: &View<BakedProp> = unsafe { View::<BakedProp>::from_ptr(baked_prop) };
    // SAFETY: the projection addresses the pushed baked prop's own
    // `constant_value` field, reached through a `Mut` view, so it is a live
    // write-capable `bool` slot in arena-owned interior-mutable storage — the
    // storage the shared `&ScalarView` (`Cell`) writes through.
    let constant_value: &ScalarView<bool> =
        unsafe { &*(baked_prop_view.constant_value_raw() as *const ScalarView<bool>) };
    // SAFETY: `keys` describes the exact initialized run written above; no
    // other access to that run overlaps the mutable slice for the duration of
    // the call.
    unsafe {
        let keys = core::slice::from_raw_parts_mut(keys.data as *mut BakedVec3, keys.count);
        bake_postprocess_vec3(bc, baked_prop_view.keys_view(), constant_value, keys)
    }?;

    buf_clear(bc.tmp_prop_view());

    Ok(())
}

// ufbx.c:27548-27585 `ufbxi_bake_element`
#[cfg(feature = "baking")]
#[inline(never)]
pub(crate) unsafe fn bake_element(
    bc: &BakeContext,
    element_id: u32,
    props: Run<'_, BakeProp>,
) -> Result<(), Fail> {
    // SAFETY: `bc.scene()` is the source `ufbx_scene` `bake_anim_imp` stored into
    // `bc`, live for the bake; this `unsafe fn` requires `element_id` to be one
    // of that scene's element ids, so the slot is in bounds of `elements` and
    // holds a stored `*mut Element` into the scene's own element buffer, which
    // carries write-capable provenance for a `Mut` view.
    let element: &View<Element> = unsafe {
        View::<Element>::from_ptr(
            *((*bc.scene()).elements.data as *const *mut Element).add(element_id as usize),
        )
    };
    if element.type_() as u32 == ElementType::Node as u32 && !bc.opts_view().skip_node_transforms()
    {
        // SAFETY: `element_id` is this fn's own scene-element parameter;
        // `props` already carries the bake-prop run contract.
        unsafe { bake_node(bc, element_id, props) }?;
    }

    let mut begin: usize = 0;
    while begin < props.len() {
        let prop_name: *const u8 = props.at(begin).prop_name();
        let mut end: usize = begin + 1;
        while end < props.len() && props.at(end).prop_name() == prop_name {
            end += 1;
        }

        // Don't bake transform related props for nodes unless specifically requested
        if element.type_() as u32 == ElementType::Node as u32
            && !bc.opts_view().bake_transform_props()
            && in_list(&TRANSFORM_PROPS.0, prop_name)
        {
            begin = end;
            continue;
        }

        // SAFETY: `element` is the live scene element and `prop_name` is one of
        // its interned NUL-terminated property names. The bounded sub-run
        // carries the former `props + begin, end - begin` contract.
        unsafe {
            bake_anim_prop(
                bc,
                element.get(),
                prop_name,
                props.subrun(begin, end - begin),
            )
        }?;
        begin = end;
    }

    let num_props: usize = bc.tmp_props_view().num_items();
    if num_props > 0 {
        let baked_elem: *mut BakedElement = bc.tmp_elements_view().push_zero::<BakedElement>(1);
        ufbxi_check_err!(bc.error_view(), !baked_elem.is_null(), "baked_elem");

        // SAFETY: `baked_elem` is the non-null (checked) zeroed
        // `ufbx_baked_element` just pushed onto `bc.tmp_elements`; `bc.tmp_props`
        // holds `num_props` items (read just above), which is what the pop moves
        // into `bc.result`.
        unsafe {
            (*baked_elem).element_id = element.element_id();
            (*baked_elem).props.count = num_props;
            (*baked_elem).props.data = bc
                .result_view()
                .push_pop::<BakedProp>(bc.tmp_props_view(), num_props);
        }
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

                // The anim prop's `element` is a stored `Ref` into the scene's own
                // element buffer, so it carries write-capable provenance.
                let element: &View<Element> =
                    ptr::read(&raw const (*anim_prop).element).view::<Mut>();

                // Sort nodes by `typed_id` to make sure we process them in order.
                if element.type_() as u32 == ElementType::Node as u32 {
                    if !bc.nodes_to_bake().is_null() {
                        *bc.nodes_to_bake().add(element.typed_id() as usize) = true;
                    }
                    (*prop).sort_id = element.typed_id();
                } else {
                    (*prop).sort_id = u32::MAX;
                }

                (*prop).element_id = element.element_id();
                (*prop).prop_name = (*anim_prop).prop_name.data;
                (*prop).anim_value = ptr::read(&raw const (*anim_prop).anim_value).ptr();

                anim_prop = anim_prop.add(1);
            }

            p_layer = p_layer.add(1);
        }
    }

    let num_props: usize = bc.tmp_bake_props_view().num_items();
    // Pops bc's bake-prop stack into bc's tmp buf; `num_props` is that stack's
    // own item count, so the pop is exact, and the sort is then over exactly
    // the popped run.
    let props_ptr: *mut BakeProp = bc
        .tmp_view()
        .push_pop::<BakeProp>(bc.tmp_bake_props_view(), num_props);
    ufbxi_check_err!(bc.error_view(), !props_ptr.is_null(), "props");

    // SAFETY: `props_ptr` is the fresh non-null `num_props`-element run just
    // checked, allocated in `bc.tmp` and stable for the remainder of the bake.
    // Each slot was initialized while collecting the bake properties above.
    let props: Run<'_, BakeProp> = unsafe { Run::from_raw_parts(props_ptr, num_props) };

    // SAFETY: `bake_prop_less` compares two slots of `props` and takes no user
    // data, so the null `user` is what it expects. `Mut` mode supplies the
    // write-capable base required by the in-place sort.
    unsafe {
        unstable_sort(
            props.as_mut_ptr() as *mut c_void,
            props.len(),
            size_of::<BakeProp>(),
            bake_prop_less,
            ptr::null_mut(),
        );
    }

    // Pre-bake layer weight times
    if !bc.opts_view().ignore_layer_weight_animation() {
        let mut has_weight_times: bool = false;
        // C: `ufbxi_for(ufbxi_bake_prop, prop, props, num_props)`
        // Each entry's `element_id` was copied from a live scene element, so it
        // indexes the scene's own `elements` list.
        unsafe {
            for prop in props.iter() {
                if prop.prop_name() != sp::Weight.as_ptr() {
                    continue;
                }
                let element: &View<Element> = View::<Element>::from_ptr(
                    *((*scene).elements.data as *const *mut Element)
                        .add(prop.element_id() as usize),
                );
                if element.type_() as u32 == ElementType::AnimLayer as u32 {
                    bake_times(
                        bc,
                        View::<AnimValue, Const>::from_ptr(prop.anim_value()),
                        true,
                        0,
                    )?;
                    has_weight_times = true;
                }
            }
        }

        if has_weight_times {
            // C: `ufbxi_bake_time_list weight_times = { 0 };`
            // SAFETY: `BakeTimeList` is a data pointer plus a count, for which
            // all-zero is the valid empty list.
            let mut weight_times: BakeTimeList = unsafe { MaybeUninit::zeroed().assume_init() };
            finalize_bake_times(bc, &mut weight_times)?;

            let count = weight_times.count;
            // SAFETY: `finalize_bake_times` filled `weight_times` with a run of
            // exactly `count` `ufbxi_bake_time` items live in bc's own
            // `tmp_prop` buf. The checked copy is an initialized, stable run in
            // bc's `tmp` buffer for every use of the published descriptor.
            let layer_weight_times = unsafe {
                let data = bc
                    .tmp_view()
                    .push_copy_raw::<BakeTime>(count, weight_times.data);
                ufbxi_check_err!(
                    bc.error_view(),
                    !data.is_null(),
                    "bc->layer_weight_times.data"
                );
                BakeTimeList::from_raw_parts(data, count)
            };
            bc.layer_weight_times_view().set(layer_weight_times);

            buf_clear(bc.tmp_prop_view());
        }
    }

    let mut begin: usize = 0;
    while begin < props.len() {
        let element_id: u32 = props.at(begin).element_id();
        let mut end: usize = begin + 1;
        while end < props.len() && props.at(end).element_id() == element_id {
            end += 1;
        }
        // SAFETY: `element_id` came from a live scene element while collecting
        // this run; the bounded sub-run carries the property-span contract.
        unsafe { bake_element(bc, element_id, props.subrun(begin, end - begin)) }?;
        begin = end;
    }

    let num_nodes: usize = bc.tmp_nodes_view().num_items();
    let num_elements: usize = bc.tmp_elements_view().num_items();

    // Pops bc's node stack into bc's result buf; `num_nodes` is that stack's
    // own item count, so the pop is exact.
    let nodes = bc
        .result_view()
        .push_pop::<BakedNode>(bc.tmp_nodes_view(), num_nodes);
    ufbxi_check_err!(bc.error_view(), !nodes.is_null(), "bc->bake.nodes.data");

    // Pops bc's element stack into bc's result buf; `num_elements` is that
    // stack's own item count, so the pop is exact.
    let elements = bc
        .result_view()
        .push_pop::<BakedElement>(bc.tmp_elements_view(), num_elements);
    ufbxi_check_err!(
        bc.error_view(),
        !elements.is_null(),
        "bc->bake.elements.data"
    );

    // SAFETY: both runs are the fresh non-null initialized pops just checked
    // and remain stable in the result buffer for the finished baked animation.
    // The sorts cover their respective complete published runs; neither
    // comparator takes user data, so the null `user` is what they expect.
    unsafe {
        bc.bake_view()
            .nodes_view()
            .set(List::from_raw_parts(nodes, num_nodes));
        bc.bake_view()
            .elements_view()
            .set(List::from_raw_parts(elements, num_elements));
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
        bc.set_ktime_offset(unsafe {
            -(*anim).time_begin * (*bc.scene()).metadata.ktime_second as f64
        });
    }

    init_ator(
        bc.error_mut_ptr(),
        bc.ator_tmp_view(),
        Some(bc.opts_view().temp_allocator_view()),
        c"temp",
    );
    init_ator(
        bc.error_mut_ptr(),
        bc.ator_result_view(),
        Some(bc.opts_view().result_allocator_view()),
        c"result",
    );

    // SAFETY: all buffers below are empty fields of the stable bake context.
    // `result` uses the initialized result allocator and every scratch buffer
    // uses the initialized temp allocator; each allocator outlives buffer use
    // and teardown, and result chunks move with allocator state into the imp.
    unsafe {
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
    }

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
    // SAFETY: the copy source is `bc`'s own `bake` field (live for the borrow)
    // and the destination is the pushed header's own `bake` field; the bake
    // context and the pushed header are distinct allocations. The moved-out
    // result allocator and buffer then take ownership in that header's refcount,
    // which `init_ref` set up.
    unsafe {
        ptr::copy_nonoverlapping(bc.bake_mut_ptr(), &raw mut (*bc.imp()).bake, 1);
        (*bc.imp()).refcount.ator = bc.ator_result();
        (*bc.imp()).refcount.buf = bc.take_result();
    }

    Ok(())
}
