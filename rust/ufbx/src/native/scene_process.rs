//! Port of the `// -- Scene pre-processing` banner section (ufbx.c:18066-18543)
//! and the head of `// -- Scene processing` (ufbx.c:18545-18995).
//!
//! FIRST UNIT: ufbx.c:18066-18543 — the `ufbxi_pre_*` scratch records, the two
//! pivot helpers and `ufbxi_pre_finalize_scene`: the multi-pass connection
//! walk that runs between parsing and `ufbxi_finalize_scene()` and builds the
//! parent/child linked list, the per-attribute instance counts, the constant
//! scale/anim-value analysis, the pivot adjustment (synthetic
//! `RotationPivot`/`ScalingPivot`/`ScalingOffset`/`GeometricTranslation`
//! properties + `adjust_pre_translation` fixups) and the geometry-transform /
//! scale helper node setup.
//!
//! SECOND UNIT: ufbx.c:18545-18995 — the element-graph builders: the
//! comparators and their paired `ufbxi_grow_array`+`ufbxi_macro_stable_sort`
//! wrappers, `ufbxi_resolve_connections` (fbx-id → element pointers, with the
//! pre-7000 property-to-attribute hack and the geometry-transform / scale
//! helper remapping), `ufbxi_add_connections_to_elements` (per-element
//! connection ranges + synthetic animated properties) and
//! `ufbxi_linearize_nodes` (parent hookup, depths, node ordering and the
//! `tmp_typed_element_offsets` bookkeeping that `ufbxi_finalize_scene()` later
//! turns into typed element pointers).
//!
//! THIRD UNIT: ufbx.c:18997-19441 — the connection-query layer the finalize
//! pass is built on: the `ufbxi_find_(dst|src)_connections` bounded searches
//! over the per-element connection ranges, the `ufbxi_fetch_*` helpers that
//! materialize typed element/texture/material/deformer/keyframe/layer lists
//! into `uc->result`, `ufbxi_find_prop_connection`, the index-sentinel patcher,
//! the remaining comparator + paired `ufbxi_grow_array`+sort wrappers, and the
//! head of the material tables (transform functions, mapping/feature flags and
//! the `ufbxi_shader_mapping(_list)` record types).
//!
//! FOURTH UNIT: ufbx.c:19443-19960 — the material mapping tables themselves:
//! the `ufbxi_mat_string` initializer helper, the 21 per-shader
//! `ufbxi_shader_mapping` property/feature tables (FBX, OBJ/MTL, OSL/Arnold
//! standard surface, 3ds Max physical + PBR, glTF, OpenPBR, ShaderFX, Blender
//! phong), the `UFBXI_MAT_*` feature bit constants and the
//! `ufbxi_shader_pbr_mappings` per-`ufbx_shader_type` dispatch table that
//! `ufbxi_fetch_mapping_maps()` walks. Pure data: no allocation and no control
//! flow, so the parity surface is entry order, the flag/transform values and
//! the NULL-vs-empty texture prefix/suffix strings.
//!
//! FIFTH UNIT: ufbx.c:19962-20427 — the table consumers and the first
//! per-element finalizers: the `UFBXI_MAPPING_FETCH_*` flags and
//! `ufbxi_fetch_mapping_maps()` (the prefix/suffix name assembly into a
//! 512-byte stack buffer, the shader-prop-binding indirection with its
//! identity fallback, and the value/texture/texture-enabled/feature arms),
//! `ufbxi_update_factor`, the `ufbxi_glossiness_remap` table and
//! `ufbxi_fetch_maps()` that drives all of it, then
//! `ufbxi_add_constraint_prop` (+ its `ufbxi_constraint_props` name table),
//! `ufbxi_finalize_nurbs_basis`, `ufbxi_finalize_lod_group` and
//! `ufbxi_push_prop_prefix`.
//!
//! SIXTH UNIT: ufbx.c:20429-20867 — the shader-texture and texture-file
//! finalizers: `ufbxi_shader_texture_find_prefix` (the two-pass compound /
//! pre-7000 property-prefix search), the `ufbxi_file_shader` quirk table,
//! `ufbxi_update_shader_texture`, `ufbxi_finalize_shader_texture` (3ds Max
//! ClassID / MaxTexture shader-type detection, the `uc->tmp_arr`-backed
//! input list that is searched while it is being built, and the
//! `ufbx_texture_file` promotion quirk), `ufbxi_propagate_main_textures`,
//! the texture-file interning pair `ufbxi_insert_texture_file` /
//! `ufbxi_pop_texture_files` and the `ufbxi_ordered_texture` comparators +
//! `ufbxi_deduplicate_textures`.
//!
//! SEVENTH UNIT: ufbx.c:20869-21450 — `ufbxi_fetch_file_textures` (the
//! `tmp_stack`-as-worklist two-visit walk over the texture graph that tolerates
//! cycles via the compressed `ufbxi_file_texture_fetch_state` byte array and
//! reuses `tmp_parse` as scratch for the two `ufbxi_deduplicate_textures`
//! passes), the geometry-transform / vec3-list helpers
//! (`ufbxi_get_geometry_transform_node`, `ufbxi_mirror_vec3_list`,
//! `ufbxi_scale_vec3_list`, `ufbxi_transform_vec3_list`,
//! `ufbxi_normalize_vec3_list`), the winding flip pair
//! (`ufbxi_flip_attrib_winding` / `ufbxi_flip_winding` with its
//! `index_mapping[-1] = UFBX_NO_INDEX` sentinel slot for edge endpoints),
//! `ufbxi_postprocess_scene`, and the filename relativization trio
//! `ufbxi_next_path_segment` / `ufbxi_absolute_to_relative_path` /
//! `ufbxi_resolve_filenames`. `ufbxi_modify_geometry` (ufbx.c:21165-21332)
//! lives at its C-order slot; the eighth unit below carries its dependencies.
//!
//! EIGHTH UNIT: ufbx.c:21452-21638 — the last helpers `ufbxi_finalize_scene()`
//! is built on: the `ufbxi_file_content` interning family
//! (`ufbxi_file_content_less` + its paired `ufbxi_grow_array`/
//! `ufbxi_stable_sort` wrapper, `ufbxi_push_file_content`,
//! `ufbxi_fetch_file_content`'s pointer-identity `lower_bound` and
//! `ufbxi_resolve_file_content`, which pushes every video/audio-clip blob onto
//! `tmp_stack`, pops the run into `uc->tmp`, sorts it and then hands the
//! deduplicated content back to the elements), `ufbxi_validate_indices`,
//! `ufbxi_finalize_mesh_material` (the two-pass per-material face-index
//! partition plus the `ufbxi_unstable_sort`ed usage order) and the
//! `ufbxi_anim_imp` refcount record with `ufbxi_push_anim`.
//! The same unit jumps the `ufbxi_finalize_scene` hole (ufbx.c:21640-22624, a
//! single ~985-line function that gets its own unit) to port
//! `// -- Interpret the read scene` in full (ufbx.c:22626-22741: the transform
//! composition family `ufbxi_add_translate` … `ufbxi_mul_inv_rotate`) plus the
//! head of `// -- Updating state from properties` (ufbx.c:22743-22784:
//! `ufbxi_mirror_translation` / `ufbxi_mirror_rotation` /
//! `ufbxi_get_geometry_transform`), which is what the deferred
//! `ufbxi_modify_geometry` (ufbx.c:21165-21332, now ported at its C-order slot)
//! was waiting on. Public leaves pulled forward for it: `ufbx_find_blob(_len)`,
//! `ufbx_quat_rotate_vec3`, `ufbx_euler_to_quat`, `ufbx_matrix_determinant` and
//! `ufbx_matrix_for_normals` in `native::api`; `ufbxi_matrix_all_zero`,
//! `ufbxi_is_quat_identity`, `ufbxi_is_vec3_equal`, `ufbxi_is_quat_equal` and
//! `ufbxi_is_transform_identity` fill the ufbx.c:11566-11607 gap in
//! `native::parse`.
//!
//! NINTH UNIT: ufbx.c:21641-22624 — `ufbxi_finalize_scene` alone, the single
//! ~985-line pass that materializes the public element graph: the
//! `tmp_elements`/`tmp_element_offsets` byte blob turned into `ufbx_element*`
//! pointers (with the `scale_helper` self-reference patched through
//! `ufbxi_node_extra`), the per-type `tmp_typed_element_offsets` drained into
//! the `elements_by_type[]` union view, the sorted `elements_by_name` table,
//! the connection-driven node children/attribute hookup, the bind-pose bone
//! filter, the skin vertex/weight prefix-sum build, the blend-channel full
//! weights, the procedural zero/consecutive index buffers and the per-mesh
//! material/deformer fetch, then the anim stack/layer/value/curve, shader,
//! material, legacy LayerElement-texture, texture-file, display-layer,
//! selection, constraint, audio and LOD passes and the closing metadata.
//! Every `ufbxi_push*`/`ufbxi_buf_free` and every paired
//! `ufbxi_grow_array`+sort inside it is allocation-observable, so the
//! statement order is verbatim.
//!
//! TENTH UNIT: ufbx.c:22786-23062 — the node-transform derivation head of
//! `// -- Updating state from properties`: `ufbxi_get_rotation` /
//! `ufbxi_get_scale` (the rotation-only and scale-only fast paths that
//! `ufbxi_get_transform`'s `ufbxi_regression_assert`s pin to it),
//! `ufbxi_get_transform` itself (the
//! `T * Roff * Rp * Rpre * R * Rpost * Rp-1 * Soff * Sp * S * Sp-1`
//! composition with the inverted PostRotation, the `use_rotation_space`
//! fork and the `has_adjust_transform` pre/post fixups),
//! `ufbxi_get_texture_transform`, `ufbxi_get_constraint_transform`,
//! `ufbxi_update_node` (rotation order, the scale-helper /
//! `inherit_scale_node` chain, the sorted `ufbx_transform_override` lookup and
//! the three inherit-mode matrix products) and `ufbxi_update_light`. Pure float
//! composition: no allocation, so the parity surface is the operation ORDER
//! (PORTING.md "Floats") — every `ufbxi_mul_*`/`ufbxi_add_translate` call and
//! every `ufbx_matrix_mul` operand order is verbatim.
//! Public leaf pulled forward for it: `ufbx_matrix_mul` in `native::api`.
//!
//! ELEVENTH UNIT: ufbx.c:23064-23495 — the rest of
//! `// -- Updating state from properties`'s per-element finalizers: the
//! `ufbxi_aperture_format` record + `ufbxi_aperture_formats` fixed-point table
//! and `ufbxi_update_camera` (the aspect/film-size/gate-fit/aperture-mode
//! cascade that derives `resolution`, `aspect_ratio`, `aperture_size_inch`,
//! `orthographic_size`, `field_of_view_deg`/`_tan` and `projection_plane`),
//! then `ufbxi_update_bone`, `ufbxi_update_line_curve`, `ufbxi_update_pose`
//! (the bind-pose parent walk through `ufbx_get_bone_pose`),
//! `ufbxi_update_skin_cluster`, `ufbxi_update_blend_channel` (the
//! split-around-zero keyframe scan and its `{ NULL }` sentinel keyframe),
//! `ufbxi_update_material`, `ufbxi_update_texture`, `ufbxi_update_anim_stack`,
//! `ufbxi_update_display_layer`, `ufbxi_find_bool3` (the `X`/`Y`/`Z`-suffixed
//! name assembly in a 64-byte stack buffer), `ufbxi_update_constraint` (the
//! `ufbx_find_prop_concat` per-target `.Weight`/`.Offset T|R|S` lookups and the
//! per-constraint-type `constrain_*` flag fills) and `ufbxi_update_anim`.
//! Pure property reads and float composition: no allocation, so the parity
//! surface is the operation ORDER plus `ufbxi_find_*` (internal, 4-byte key)
//! vs `ufbx_find_*` (public, `ufbxi_get_name_key`) at every call site.
//! Public leaves pulled forward for it: `ufbx_find_vec3(_len)`,
//! `ufbx_find_prop_concat`, `ufbx_get_bone_pose`, `ufbx_matrix_invert`,
//! `ufbx_matrix_to_transform` and the `ufbx_identity_quat` datum in
//! `native::api`.
//!
//! TWELFTH UNIT: ufbx.c:23497-23944 — the tail of
//! `// -- Updating state from properties`, i.e. the scene-wide drivers: the
//! `ufbxi_mirror_matrix_dst`/`_src`/`ufbxi_mirror_matrix` trio (dst mirrors a
//! ROW across all four columns, src mirrors ONE column — not the same walk),
//! `ufbxi_update_initial_clusters` (bind-matrix space conversion followed by
//! the `mesh_node_to_bone` patch-up through `ufbxi_fetch_src_element` and the
//! geometry-transform-helper HACK), `ufbxi_find_axis`, the
//! `ufbxi_time_mode_fps` table, `ufbxi_axis_matrix` (the axis-enum remap whose
//! `>> 1` selects the axis and whose parity selects the sign),
//! `ufbxi_update_adjust_transforms` (the root/unit-scale split between
//! `geometry_scale` and `root_scale`, the per-node `adjust_*` fills and the
//! light/camera post-rotations), `ufbxi_update_scene` (the fixed per-list
//! update ORDER, with `ufbxi_update_initial_clusters` + `ufbxi_update_pose`
//! only under `initial`), `ufbxi_update_scene_metadata`, the
//! `ufbxi_pow10_targets` table + `ufbxi_round_if_near`,
//! `ufbxi_update_scene_settings` and `ufbxi_update_scene_settings_obj`.
//! Still no allocation, so the parity surface is operation ORDER plus two
//! literal-width traps: `ufbxi_time_mode_fps` is an `ufbx_real[]` initialized
//! from `float` constants (29.97/23.976/59.94 are `(double)(float)x`, NOT the
//! `double` nearest values) while `ufbxi_pow10_targets` uses `double`
//! constants.
//! Public leaves pulled forward for it: `ufbx_coordinate_axes_valid` and
//! `ufbx_transform_direction` in `native::api`.
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]

use core::ffi::c_void;
use core::mem::{size_of, MaybeUninit};
use core::ptr;

use crate::generated::{
    Anim, AnimCurve, AnimLayer, AnimProp, AnimStack, AnimValue, ApertureFormat, ApertureMode,
    AspectMode, AudioClip, AudioLayer, BlendChannel, BlendDeformer, BlendKeyframe, BlendMode,
    BlendShape, Bone, BonePose, CacheDeformer, CacheFile, CacheFileFormat, Camera, ColorSet,
    Connection, Constraint, ConstraintAimUpType, ConstraintTarget, ConstraintType, CoordinateAxes,
    CoordinateAxis, DisplayLayer, Edge, Element, ElementType, Error, Exporter, Face, FileFormat,
    GateFit, GeometryTransformHandling, IndexErrorHandling, InheritMode, InheritModeHandling,
    Light, LightAreaShape, LightDecay, LightType, LineCurve, LodDisplay, LodGroup, LodLevel,
    Material, MaterialFbxMap, MaterialFbxMaps, MaterialFeature, MaterialFeatureInfo,
    MaterialFeatures, MaterialMap, MaterialPbrMap, MaterialPbrMaps, MaterialTexture, Matrix, Mesh,
    MeshPart, Metadata, MirrorAxis, NameElement, Node, NurbsBasis, NurbsCurve, NurbsSurface,
    NurbsTopology, PivotHandling, Pose, ProjectionMode, Prop, PropFlags, PropType, Props, Quat,
    RotationOrder, Scene, SceneSettings, SelectionNode, SelectionSet, Shader, ShaderBinding,
    ShaderPropBinding, ShaderTexture, ShaderTextureInput, ShaderTextureType, ShaderType,
    SkinCluster, SkinDeformer, SkinVertex, SkinWeight, SkinningMethod, SnapMode, SpaceConversion,
    StereoCamera, Texture, TextureFile, TextureLayer, TextureType, TimeMode, TimeProtocol,
    TopoEdge, Transform, TransformOverride, UvSet, Vec2, Vec3, Vec4, Video, VoidList, WarningType,
    WrapMode,
};
use crate::native::allocator::grow_array;
use crate::native::api::{
    compute_normals, compute_topology, coordinate_axes_valid, euler_to_quat, find_blob,
    find_bool as api_find_bool, find_int as api_find_int, find_int_len as api_find_int_len,
    find_prop as api_find_prop, find_prop_concat, find_prop_len, find_prop_texture_len,
    find_real as api_find_real, find_real_len as api_find_real_len, find_shader_prop_bindings_len,
    find_shader_texture_input, find_shader_texture_input_len, find_string,
    find_vec3 as api_find_vec3, generate_normal_mapping, get_bone_pose, get_prop_element,
    matrix_for_normals, matrix_invert, matrix_mul, matrix_to_transform, quat_rotate_vec3,
    transform_direction, transform_position, transform_to_matrix, EMPTY_BLOB, EMPTY_STRING,
    IDENTITY_MATRIX, IDENTITY_QUAT, IDENTITY_TRANSFORM, ZERO_VEC3,
};
use crate::native::buf::{
    buf_clear, buf_free, pop, push, push_copy, push_peek, push_pop, push_zero, Buf,
};
use crate::native::error::{
    memcmp, strcmp, strlen, ufbxi_check, ufbxi_check_err, ufbxi_check_msg, ufbxi_snprintf, Fail,
    EMPTY_CHAR,
};
use crate::native::hash::{hash64, hash_ptr, map_find, map_insert};
use crate::native::parse::{
    find_enum, find_int, find_prop, find_prop_with_key, find_real, find_vec3, get_element_extra,
    get_name_key, is_node_property_name, is_quat_identity, is_transform_identity, is_vec3_zero,
    is_vec4_zero, matrix_all_zero, name_key_less, Context, FbxAttrEntry, FbxIdEntry, FileContent,
    MeshExtra, Refcount, TextureExtra, TextureFileEntry, TmpBonePose, TmpConnection,
    TmpMaterialTexture, TmpMeshTexture, ELEMENT_TYPE_COUNT,
};
// Only reachable from the two `ufbxi_regression_assert`s in `ufbxi_get_transform`
// (ufbx.c:22901-22902), which is why C marks both `ufbxi_unused` (11594/11599).
#[cfg(feature = "regression")]
use crate::native::parse::{is_quat_equal, is_vec3_equal, Context};
use crate::native::platform::{
    add_ptr, f64_to_i64, macro_lower_bound_eq, macro_stable_sort, macro_upper_bound_eq, math,
    max32, max_sz, min32, min_sz, pack_version, stable_sort, to_size, ufbx_assert,
    ufbxi_dev_assert, ufbxi_ignore, ufbxi_regression_assert, ufbxi_string_literal,
    ufbxi_unreachable, unstable_sort, NO_INDEX,
};
use crate::native::read::{
    deduplicate_properties, find_fbx_id, fix_index, init_synthetic_vec3_prop, mesh_part_add_face,
    opt_ptr, opt_ref, ref_ptr, resolve_relative_filename, set_own_prop_vec3_uniform,
    setup_geometry_transform_helper, setup_scale_helper, sort_properties, strblob_data,
    strblob_length, strblob_set, unscaled_transform_to_matrix, update_vertex_first_index,
    NodeExtra, Strblob, SENTINEL_INDEX_CONSECUTIVE, SENTINEL_INDEX_ZERO,
};
use crate::native::string_pool::{
    self as sp, add3, concat_str_cmp, min3, neg3, normalize3, str_cmp, str_less, sub3, ONE_VEC3,
};
use crate::native::warnings::ufbxi_warnf_tag;
use crate::prelude::{Blob, List, Real, Ref, RefList, String};

// -- Scene pre-processing (ufbx.c:18066-18543)

// ufbx.h:738 `UFBX_ELEMENT_TYPE_FIRST_ATTRIB = UFBX_ELEMENT_MESH`
pub(crate) const ELEMENT_TYPE_FIRST_ATTRIB: u32 = ElementType::Mesh as u32;
// ufbx.h:739 `UFBX_ELEMENT_TYPE_LAST_ATTRIB = UFBX_ELEMENT_LOD_GROUP`
pub(crate) const ELEMENT_TYPE_LAST_ATTRIB: u32 = ElementType::LodGroup as u32;

// ufbx.c:18068-18070 `ufbxi_pre_connection`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PreConnection {
    pub src: *mut Element,
    pub dst: *mut Element,
}

// ufbx.c:18072-18081 `ufbxi_pre_node`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PreNode {
    pub has_constant_scale: bool,
    pub has_recursive_scale_helper: bool,
    pub has_skin_deformer: bool,
    pub constant_scale: Vec3,
    pub element_id: u32,
    pub first_child: u32,
    pub next_child: u32,
    pub parent: u32,
}

// ufbx.c:18083-18085 `ufbxi_pre_mesh`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PreMesh {
    pub has_skin_deformer: bool,
}

// ufbx.c:18087-18090 `ufbxi_pre_anim_value`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PreAnimValue {
    pub has_constant_value: bool,
    pub constant_value: Vec3,
}

// ufbx.c:18092-18097 `ufbxi_pivot_nonzero`
pub(crate) fn pivot_nonzero(offset: Vec3) -> bool {
    // TODO: Expose this as a setting?
    let epsilon: f64 = 0.0009765625;
    // C: `ufbx_fabs(offset.x) >= epsilon` — `ufbx_fabs` takes `double`, so each
    // component promotes to double and the comparison is in double.
    math::fabs(offset.x as f64) >= epsilon
        || math::fabs(offset.y as f64) >= epsilon
        || math::fabs(offset.z as f64) >= epsilon
}

// ufbx.c:18099-18107 `ufbxi_pivot_div`
pub(crate) fn pivot_div(offset: Real, initial_scale: Real) -> Real {
    let epsilon: f64 = 0.0078125;
    if math::fabs(initial_scale as f64) >= epsilon {
        offset / initial_scale
    } else {
        offset
    }
}

// Called between parsing and `ufbxi_finalize_scene()`.
// This is a very messy function reminiscent of the _old_ ufbx, where we do
// multiple passes over connections without having a proper scene graph.
// This, however gives us the advantage of allowing us to modify elements
// and connections. We can, for example, add new helper nodes and redirect
// animated properties from source nodes to the helpers. The rest of ufbx
// will treat these as if they were a part of the source file.
// ufbx.c:18109-18543 `ufbxi_pre_finalize_scene`
#[inline(never)]
// The `UFBX_REGRESSION` `required = true` (ufbx.c:18122-18124) makes the three
// preceding option tests dead in regression builds — kept verbatim.
#[cfg_attr(feature = "regression", allow(unused_assignments))]
pub(crate) unsafe fn pre_finalize_scene(uc: &Context) -> Result<(), Fail> {
    let mut required: bool = false;
    if (*uc.get()).opts.geometry_transform_handling == GeometryTransformHandling::HelperNodes
        || (*uc.get()).opts.geometry_transform_handling == GeometryTransformHandling::ModifyGeometry
    {
        required = true;
    }
    if (*uc.get()).opts.inherit_mode_handling == InheritModeHandling::HelperNodes
        || (*uc.get()).opts.inherit_mode_handling == InheritModeHandling::Compensate
        || (*uc.get()).opts.inherit_mode_handling == InheritModeHandling::CompensateNoFallback
    {
        required = true;
    }
    if (*uc.get()).opts.pivot_handling == PivotHandling::AdjustToPivot
        || (*uc.get()).opts.pivot_handling == PivotHandling::AdjustToRotationPivot
    {
        required = true;
    }
    #[cfg(feature = "regression")]
    {
        required = true;
    }

    if !required {
        return Ok(());
    }

    let num_elements: u32 = uc.num_elements();
    let num_nodes: usize = (*uc.get()).tmp_node_ids.num_items;
    let elements: *mut *mut Element = push_pop::<*mut Element>(
        uc.tmp_parse_mut_ptr(),
        &mut (*uc.get()).tmp_element_ptrs,
        num_elements as usize,
    );
    ufbxi_check!(uc, !elements.is_null(), "elements");

    let num_connections: usize = (*uc.get()).tmp_connections.num_items;
    let tmp_connections: *mut TmpConnection = push_peek::<TmpConnection>(
        uc.tmp_parse_mut_ptr(),
        uc.tmp_connections_mut_ptr(),
        num_connections,
    );
    ufbxi_check!(uc, !tmp_connections.is_null(), "tmp_connections");

    let pre_connections: *mut PreConnection =
        push::<PreConnection>(uc.tmp_parse_mut_ptr(), num_connections);
    ufbxi_check!(uc, !pre_connections.is_null(), "pre_connections");

    let instance_counts: *mut u32 = push_zero::<u32>(uc.tmp_parse_mut_ptr(), num_elements as usize);
    ufbxi_check!(uc, !instance_counts.is_null(), "instance_counts");

    let modify_not_supported: *mut bool =
        push_zero::<bool>(uc.tmp_parse_mut_ptr(), num_elements as usize);
    ufbxi_check!(uc, !modify_not_supported.is_null(), "modify_not_supported");

    let node_attrib_type: *mut ElementType =
        push_zero::<ElementType>(uc.tmp_parse_mut_ptr(), num_nodes);
    ufbxi_check!(uc, !node_attrib_type.is_null(), "node_attrib_type");

    let has_unscaled_children: *mut bool = push_zero::<bool>(uc.tmp_parse_mut_ptr(), num_nodes);
    ufbxi_check!(
        uc,
        !has_unscaled_children.is_null(),
        "has_unscaled_children"
    );

    let has_scale_animation: *mut bool = push_zero::<bool>(uc.tmp_parse_mut_ptr(), num_nodes);
    ufbxi_check!(uc, !has_scale_animation.is_null(), "has_scale_animation");
    // C-parity: `has_scale_animation` is allocated and checked but never read
    // upstream; the allocation is observable so it stays.

    let pre_nodes: *mut PreNode = push_zero::<PreNode>(uc.tmp_parse_mut_ptr(), num_nodes);
    ufbxi_check!(uc, !pre_nodes.is_null(), "pre_nodes");

    let num_meshes: usize =
        (*uc.get()).tmp_typed_element_offsets[ElementType::Mesh as usize].num_items;
    let pre_meshes: *mut PreMesh = push_zero::<PreMesh>(uc.tmp_parse_mut_ptr(), num_meshes);
    ufbxi_check!(uc, !pre_meshes.is_null(), "pre_meshes");

    let num_anim_values: usize =
        (*uc.get()).tmp_typed_element_offsets[ElementType::AnimValue as usize].num_items;
    let pre_anim_values: *mut PreAnimValue =
        push_zero::<PreAnimValue>(uc.tmp_parse_mut_ptr(), num_anim_values);
    ufbxi_check!(uc, !pre_anim_values.is_null(), "pre_anim_values");

    let fbx_ids: *mut u64 = push_pop::<u64>(
        uc.tmp_parse_mut_ptr(),
        &mut (*uc.get()).tmp_element_fbx_ids,
        num_elements as usize,
    );
    ufbxi_check!(uc, !fbx_ids.is_null(), "fbx_ids");

    // TODO
    // C-parity: `0.001f`/`0.01f` are `float` literals widened to `ufbx_real`
    // (double) — NOT the decimal values (PORTING.md "Floats").
    let scale_epsilon: Real = 0.001f32 as Real;
    let pivot_epsilon: Real = 0.001f32 as Real;
    let compensate_epsilon: Real = 0.01f32 as Real;

    for i in 0..num_elements as usize {
        let element: *mut Element = *elements.add(i);
        let id: u32 = (*element).typed_id;

        if (*element).type_ == ElementType::Node {
            let pre_node: *mut PreNode = pre_nodes.add(id as usize);
            (*pre_node).has_constant_scale = true;
            (*pre_node).constant_scale =
                find_vec3(&(*element).props, sp::Lcl_Scaling.as_ptr(), 1.0, 1.0, 1.0);
            (*pre_node).element_id = (*element).element_id;
            (*pre_node).first_child = !0u32;
            (*pre_node).next_child = !0u32;
            (*pre_node).parent = !0u32;
        }
        // C-parity: ufbx.c:18186 is `} if (...)`, not `} else if (...)` — the
        // two element-type tests are independent statements.
        if (*element).type_ == ElementType::AnimValue {
            let pre_value: *mut PreAnimValue = pre_anim_values.add(id as usize);
            (*pre_value).has_constant_value = true;
            (*pre_value).constant_value.x =
                find_real(&(*element).props, sp::X.as_ptr(), math::NAN as Real);
            (*pre_value).constant_value.x = find_real(
                &(*element).props,
                sp::d_X.as_ptr(),
                (*pre_value).constant_value.x,
            );
            (*pre_value).constant_value.y =
                find_real(&(*element).props, sp::Y.as_ptr(), math::NAN as Real);
            (*pre_value).constant_value.y = find_real(
                &(*element).props,
                sp::d_Y.as_ptr(),
                (*pre_value).constant_value.y,
            );
            (*pre_value).constant_value.z =
                find_real(&(*element).props, sp::Z.as_ptr(), math::NAN as Real);
            (*pre_value).constant_value.z = find_real(
                &(*element).props,
                sp::d_Z.as_ptr(),
                (*pre_value).constant_value.z,
            );
        }
    }

    for i in 0..num_connections {
        let tmp: *mut TmpConnection = tmp_connections.add(i);
        let pre: *mut PreConnection = pre_connections.add(i);

        let src_entry: *mut FbxIdEntry = find_fbx_id(uc, (*tmp).src);
        let dst_entry: *mut FbxIdEntry = find_fbx_id(uc, (*tmp).dst);

        let src: *mut Element = if !src_entry.is_null() {
            *elements.add((*src_entry).element_id as usize)
        } else {
            ptr::null_mut()
        };
        let dst: *mut Element = if !dst_entry.is_null() {
            *elements.add((*dst_entry).element_id as usize)
        } else {
            ptr::null_mut()
        };
        (*pre).src = src;
        (*pre).dst = dst;
        if src.is_null() || dst.is_null() {
            continue;
        }

        if (*tmp).src_prop.length == 0 && (*tmp).dst_prop.length == 0 {
            // Count number of instances of each attribute
            if (*dst).type_ == ElementType::Node {
                let dst_node: *mut Node = dst as *mut Node;

                if (*src).type_ as u32 >= ELEMENT_TYPE_FIRST_ATTRIB
                    && (*src).type_ as u32 <= ELEMENT_TYPE_LAST_ATTRIB
                {
                    let p_count: *mut u32 = instance_counts.add((*src).element_id as usize);
                    *p_count = (*p_count).wrapping_add(1);
                    let count: u32 = *p_count;
                    *node_attrib_type.add((*dst).typed_id as usize) = if count == 1 {
                        (*src).type_
                    } else {
                        ElementType::Unknown
                    };

                    // These must match what can be trasnsformed in `ufbxi_modify_geometry()`
                    match (*src).type_ {
                        ElementType::Mesh
                        | ElementType::LineCurve
                        | ElementType::NurbsCurve
                        | ElementType::NurbsSurface => {} // Nop, supported
                        _ => {
                            *modify_not_supported.add((*dst).element_id as usize) = true;
                        }
                    }
                }

                if (*src).type_ == ElementType::Node {
                    let src_node: *mut Node = src as *mut Node;
                    let pre_dst: *mut PreNode =
                        pre_nodes.add((*dst_node).element.typed_id as usize);
                    let pre_src: *mut PreNode =
                        pre_nodes.add((*src_node).element.typed_id as usize);

                    // Remember parent and add children into a linked list
                    if (*pre_src).parent == !0u32 {
                        (*pre_src).parent = (*dst_node).element.typed_id;
                        (*pre_src).next_child = (*pre_dst).first_child;
                        (*pre_dst).first_child = (*src_node).element.typed_id;
                    }

                    if (*uc.get()).opts.inherit_mode_handling != InheritModeHandling::Preserve {
                        if !(*dst_node).is_root
                            && (*src_node).original_inherit_mode != InheritMode::Normal
                        {
                            *has_unscaled_children.add((*dst).typed_id as usize) = true;
                        }
                    }
                }
            } else if (*dst).type_ == ElementType::Mesh {
                if (*src).type_ == ElementType::SkinDeformer {
                    let pre_mesh: *mut PreMesh = pre_meshes.add((*dst).typed_id as usize);
                    (*pre_mesh).has_skin_deformer = true;
                }
            }
        } else if (*tmp).src_prop.length == 0 && (*tmp).dst_prop.length != 0 {
            let dst_prop: *const u8 = (*tmp).dst_prop.data;
            if (*dst).type_ == ElementType::AnimValue && (*src).type_ == ElementType::AnimCurve {
                let src_curve: *mut AnimCurve = src as *mut AnimCurve;
                let mut index: u32 = 0;
                if dst_prop == sp::Y.as_ptr() || dst_prop == sp::d_Y.as_ptr() {
                    index = 1;
                } else if dst_prop == sp::Z.as_ptr() || dst_prop == sp::d_Z.as_ptr() {
                    index = 2;
                }

                let pre_value: *mut PreAnimValue = pre_anim_values.add((*dst).typed_id as usize);
                if (*src_curve).max_value - (*src_curve).min_value >= scale_epsilon {
                    (*pre_value).has_constant_value = false;
                } else {
                    let constant_value: Real =
                        ((*src_curve).min_value + (*src_curve).max_value) * 0.5;
                    // C: `pre_value->constant_value.v[index]` — the `ufbx_vec3`
                    // union's array view.
                    let v: *mut Real = (&mut (*pre_value).constant_value as *mut Vec3 as *mut Real)
                        .add(index as usize);
                    if math::isnan(*v as f64) {
                        *v = constant_value;
                    }
                    // C: `(ufbx_real)ufbx_fabs(v - constant_value) > scale_epsilon`
                    // — the subtraction is in `ufbx_real`, only `fabs` runs in
                    // double, and its result narrows back before the compare.
                    if math::fabs((*v - constant_value) as f64) as Real > scale_epsilon {
                        (*pre_value).has_constant_value = false;
                    }
                }
            }
        }
    }

    for i in 0..num_connections {
        let tmp: *mut TmpConnection = tmp_connections.add(i);
        let pre: *mut PreConnection = pre_connections.add(i);
        let src: *mut Element = (*pre).src;
        let dst: *mut Element = (*pre).dst;
        if src.is_null() || dst.is_null() {
            continue;
        }

        if (*tmp).src_prop.length == 0 && (*tmp).dst_prop.length == 0 {
            // Count maximum number of instanced attributes in a node
            if (*dst).type_ == ElementType::Node {
                if (*src).type_ as u32 >= ELEMENT_TYPE_FIRST_ATTRIB
                    && (*src).type_ as u32 <= ELEMENT_TYPE_LAST_ATTRIB
                {
                    *instance_counts.add((*dst).element_id as usize) = max32(
                        *instance_counts.add((*dst).element_id as usize),
                        *instance_counts.add((*src).element_id as usize),
                    );
                    if (*src).type_ == ElementType::Mesh {
                        let pre_mesh: *mut PreMesh = pre_meshes.add((*src).typed_id as usize);
                        if (*pre_mesh).has_skin_deformer {
                            (*pre_nodes.add((*dst).typed_id as usize)).has_skin_deformer = true;
                        }
                    }
                } else if (*src).type_ == ElementType::SkinDeformer {
                    (*pre_nodes.add((*dst).typed_id as usize)).has_skin_deformer = true;
                }
            }
        } else if (*tmp).src_prop.length == 0 && (*tmp).dst_prop.length != 0 {
            if (*dst).type_ == ElementType::Node {
                if (*src).type_ == ElementType::AnimValue {
                    if (*tmp).dst_prop.data == sp::Lcl_Scaling.as_ptr() {
                        let pre_node: *mut PreNode = pre_nodes.add((*dst).typed_id as usize);
                        if (*pre_node).has_constant_scale {
                            let pre_value: *mut PreAnimValue =
                                pre_anim_values.add((*src).typed_id as usize);
                            if !(*pre_value).has_constant_value {
                                (*pre_node).has_constant_scale = false;
                            } else {
                                // C: `error += (ufbx_real)ufbx_fabs(a - b)` — real
                                // subtraction, double `fabs`, narrowed back to
                                // real before accumulating.
                                let mut error: Real = 0.0;
                                error += math::fabs(
                                    ((*pre_value).constant_value.x - (*pre_node).constant_scale.x)
                                        as f64,
                                ) as Real;
                                error += math::fabs(
                                    ((*pre_value).constant_value.y - (*pre_node).constant_scale.y)
                                        as f64,
                                ) as Real;
                                error += math::fabs(
                                    ((*pre_value).constant_value.z - (*pre_node).constant_scale.z)
                                        as f64,
                                ) as Real;
                                if error >= scale_epsilon {
                                    (*pre_node).has_constant_scale = false;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (*uc.get()).opts.pivot_handling == PivotHandling::AdjustToPivot
        || (*uc.get()).opts.pivot_handling == PivotHandling::AdjustToRotationPivot
    {
        for i in 0..num_nodes {
            let pre_node: *mut PreNode = pre_nodes.add(i);
            let node: *mut Node = *elements.add((*pre_node).element_id as usize) as *mut Node;

            let rotation_pivot: Vec3 = find_vec3(
                &(*node).element.props,
                sp::RotationPivot.as_ptr(),
                0.0,
                0.0,
                0.0,
            );
            let scaling_pivot: Vec3 = find_vec3(
                &(*node).element.props,
                sp::ScalingPivot.as_ptr(),
                0.0,
                0.0,
                0.0,
            );
            let scaling_offset: Vec3 = find_vec3(
                &(*node).element.props,
                sp::ScalingOffset.as_ptr(),
                0.0,
                0.0,
                0.0,
            );

            let mut should_modify_pivot: bool = false;
            if (*uc.get()).opts.pivot_handling == PivotHandling::AdjustToPivot {
                should_modify_pivot = !is_vec3_zero(rotation_pivot);
            } else if (*uc.get()).opts.pivot_handling == PivotHandling::AdjustToRotationPivot {
                should_modify_pivot = pivot_nonzero(rotation_pivot)
                    || pivot_nonzero(scaling_pivot)
                    || pivot_nonzero(scaling_offset);
            }

            if should_modify_pivot {
                let mut skip_geometry_transform: bool = false;
                let mut can_modify_geometry_transform: bool = true;
                if (*uc.get()).opts.pivot_handling == PivotHandling::AdjustToRotationPivot {
                    if *node_attrib_type.add((*node).element.typed_id as usize)
                        == ElementType::Empty
                    {
                        if !(*uc.get()).opts.pivot_handling_retain_empties {
                            skip_geometry_transform = true;
                        } else {
                            can_modify_geometry_transform = false;
                        }
                    }
                }

                if (*uc.get()).opts.geometry_transform_handling
                    == GeometryTransformHandling::ModifyGeometryNoFallback
                {
                    if *instance_counts.add((*node).element.element_id as usize) > 1
                        || *modify_not_supported.add((*node).element.element_id as usize)
                    {
                        can_modify_geometry_transform = false;
                    }
                }
                // Currently, geometry transform messes up skinning
                if (*pre_node).has_skin_deformer {
                    can_modify_geometry_transform = false;
                }

                let mut can_modify_pivot: bool = true;
                if (*uc.get()).opts.pivot_handling == PivotHandling::AdjustToPivot {
                    // C: `err += (ufbx_real)ufbx_fabs(a - b)` — real subtraction,
                    // double `fabs`, narrowed back to real before accumulating.
                    let mut err: Real = 0.0;
                    err += math::fabs((rotation_pivot.x - scaling_pivot.x) as f64) as Real;
                    err += math::fabs((rotation_pivot.y - scaling_pivot.y) as f64) as Real;
                    err += math::fabs((rotation_pivot.z - scaling_pivot.z) as f64) as Real;
                    if err > pivot_epsilon {
                        can_modify_pivot = false;
                    }
                }

                if can_modify_pivot && (can_modify_geometry_transform || skip_geometry_transform) {
                    let mut geometric_translation: Vec3 = find_vec3(
                        &(*node).element.props,
                        sp::GeometricTranslation.as_ptr(),
                        0.0,
                        0.0,
                        0.0,
                    );

                    let mut child_offset: Vec3 = Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    };
                    let mut new_props: *mut Prop = ptr::null_mut();
                    let num_props: usize = (*node).element.props.props.count;
                    let mut new_prop_count: usize = num_props;
                    if (*uc.get()).opts.pivot_handling == PivotHandling::AdjustToPivot {
                        ufbx_assert!(!skip_geometry_transform); // not supporeted in legacy mode
                        child_offset = neg3(rotation_pivot);
                        geometric_translation = add3(geometric_translation, child_offset);

                        new_props = push_zero::<Prop>(uc.result_mut_ptr(), num_props + 3);
                        ufbxi_check!(uc, !new_props.is_null(), "new_props");
                        ptr::copy_nonoverlapping(
                            (*node).element.props.props.data,
                            new_props,
                            num_props,
                        );

                        init_synthetic_vec3_prop(
                            new_props.add(new_prop_count),
                            sp::RotationPivot.as_ptr(),
                            &ZERO_VEC3,
                            PropType::Vector,
                        );
                        new_prop_count += 1;
                        init_synthetic_vec3_prop(
                            new_props.add(new_prop_count),
                            sp::ScalingPivot.as_ptr(),
                            &ZERO_VEC3,
                            PropType::Vector,
                        );
                        new_prop_count += 1;
                        init_synthetic_vec3_prop(
                            new_props.add(new_prop_count),
                            sp::GeometricTranslation.as_ptr(),
                            &geometric_translation,
                            PropType::Vector,
                        );
                        new_prop_count += 1;
                    } else if (*uc.get()).opts.pivot_handling
                        == PivotHandling::AdjustToRotationPivot
                    {
                        // We can eliminate the post-rotation translation and move it to the geometry/children as follows.
                        // Let Z be the initial value of S in the transform (aka `initial_scale`):
                        //
                        //   (Rp-1+Soff+Sp) + S * (Sp-1)
                        //   S * (Sp-1 + (Rp-1+Soff+Sp)/S)
                        //   S * (Sp-1 + (Rp-1+Soff+Sp)/S - (Rp-1+Soff+Sp)/Z + (Rp-1+Soff+Sp)/Z)
                        //
                        //   (Rp-1 + Soff + Sp) + S * (-(Rp-1 + Soff + Sp)/Z + (Sp-1 + (Rp-1 + Soff + Sp)/Z))
                        //   ^-scaled_offset--^         ^-unscaled_offset--^           ^-unscaled_offset--^
                        //   ^---------------- 0, when S=Z ----------------^   ^------- child_offset ------^
                        //
                        // We need to be careful when doing this in case any component of Z is 0. Fortunately,
                        // the above holds for all `Z != 0`, it will just result in non-zero translation in the parent.
                        let initial_scale: Vec3 = find_vec3(
                            &(*node).element.props,
                            sp::Lcl_Scaling.as_ptr(),
                            1.0,
                            1.0,
                            1.0,
                        );
                        let scaled_offset: Vec3 =
                            sub3(add3(scaling_offset, scaling_pivot), rotation_pivot);
                        // C: `ufbx_vec3 unscaled_offset;` — all three components
                        // assigned below before any read (no upstream
                        // `ufbxi_uninit` marker).
                        let mut unscaled_offset: Vec3 = Vec3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        };
                        unscaled_offset.x = pivot_div(scaled_offset.x, initial_scale.x);
                        unscaled_offset.y = pivot_div(scaled_offset.y, initial_scale.y);
                        unscaled_offset.z = pivot_div(scaled_offset.z, initial_scale.z);

                        // Convert `scaled_offset + S*unscaled_offset` to FBX scaling pivot and offset.
                        let new_scaling_pivot: Vec3 = unscaled_offset;
                        let new_scaling_offset: Vec3 = sub3(scaled_offset, new_scaling_pivot);
                        child_offset = sub3(unscaled_offset, scaling_pivot);

                        new_props = push_zero::<Prop>(uc.result_mut_ptr(), num_props + 4);
                        ufbxi_check!(uc, !new_props.is_null(), "new_props");
                        ptr::copy_nonoverlapping(
                            (*node).element.props.props.data,
                            new_props,
                            num_props,
                        );

                        init_synthetic_vec3_prop(
                            new_props.add(new_prop_count),
                            sp::RotationPivot.as_ptr(),
                            &ZERO_VEC3,
                            PropType::Vector,
                        );
                        new_prop_count += 1;
                        init_synthetic_vec3_prop(
                            new_props.add(new_prop_count),
                            sp::ScalingPivot.as_ptr(),
                            &new_scaling_pivot,
                            PropType::Vector,
                        );
                        new_prop_count += 1;
                        init_synthetic_vec3_prop(
                            new_props.add(new_prop_count),
                            sp::ScalingOffset.as_ptr(),
                            &new_scaling_offset,
                            PropType::Vector,
                        );
                        new_prop_count += 1;
                        if !skip_geometry_transform {
                            geometric_translation = add3(geometric_translation, child_offset);
                            init_synthetic_vec3_prop(
                                new_props.add(new_prop_count),
                                sp::GeometricTranslation.as_ptr(),
                                &geometric_translation,
                                PropType::Vector,
                            );
                            new_prop_count += 1;
                        }
                    }

                    (*node).element.props.props.data = new_props;
                    (*node).element.props.props.count = new_prop_count;
                    sort_properties(
                        uc,
                        (*node).element.props.props.data as *mut Prop,
                        (*node).element.props.props.count,
                    )?;
                    deduplicate_properties(&mut (*node).element.props.props);

                    (*node).adjust_pre_translation =
                        add3((*node).adjust_pre_translation, rotation_pivot);
                    (*node).has_adjust_transform = true;
                    let mut ix: u32 = (*pre_node).first_child;
                    while ix != !0u32 {
                        let pre_child: *mut PreNode = pre_nodes.add(ix as usize);
                        let child: *mut Node =
                            *elements.add((*pre_child).element_id as usize) as *mut Node;

                        (*child).adjust_pre_translation =
                            add3((*child).adjust_pre_translation, child_offset);
                        (*child).has_adjust_transform = true;

                        ix = (*pre_child).next_child;
                    }
                }
            }
        }
    }

    for i in 0..num_elements as usize {
        let element: *mut Element = *elements.add(i);
        let fbx_id: u64 = *fbx_ids.add(i);

        if (*element).type_ == ElementType::Node {
            let node: *mut Node = element as *mut Node;
            let mut requires_helper_node: bool = false;
            if (*uc.get()).opts.geometry_transform_handling
                == GeometryTransformHandling::HelperNodes
            {
                requires_helper_node = true;
            } else if (*uc.get()).opts.geometry_transform_handling
                == GeometryTransformHandling::ModifyGeometry
            {
                // Setup a geometry transform helper for nodes that have instanced attributes
                requires_helper_node = *instance_counts.add(i) > 1 || *modify_not_supported.add(i);
            }
            if requires_helper_node {
                setup_geometry_transform_helper(uc, node, fbx_id)?;
            }
        }
    }

    for i in 0..num_elements as usize {
        let element: *mut Element = *elements.add(i);
        let fbx_id: u64 = *fbx_ids.add(i);

        if (*element).type_ == ElementType::Node {
            let node: *mut Node = element as *mut Node;
            if *has_unscaled_children.add((*node).element.typed_id as usize)
                && (*node).scale_helper.is_none()
            {
                let pre_node: *mut PreNode = pre_nodes.add((*node).element.typed_id as usize);
                let r#ref: Real =
                    if (*uc.get()).opts.inherit_mode_handling == InheritModeHandling::Compensate {
                        (*pre_node).constant_scale.x
                    } else {
                        1.0
                    };
                let scale: Vec3 = (*pre_node).constant_scale;
                // C: `(ufbx_real)ufbx_fabs(scale.x - ref)` — real subtraction,
                // double `fabs`, narrowed back to real.
                let dx: Real = math::fabs((scale.x - r#ref) as f64) as Real;
                let dy: Real = math::fabs((scale.y - r#ref) as f64) as Real;
                let dz: Real = math::fabs((scale.z - r#ref) as f64) as Real;
                if (dx + dy + dz >= scale_epsilon
                    || !(*pre_node).has_constant_scale
                    || math::fabs(scale.x as f64) as Real <= compensate_epsilon)
                    && (*uc.get()).opts.inherit_mode_handling
                        != InheritModeHandling::CompensateNoFallback
                {
                    setup_scale_helper(uc, node, fbx_id)?;

                    // If we added a geometry transform helper that may scale further helpers
                    // recursively for all child nodes using `UFBX_INHERIT_MODE_COMPONENTWISE_SCALE`
                    // This is guaranteed to terminate as `ufbxi_pre_node` may only have one parent,
                    // meaning any cycles must contain `node` itself.
                    let mut ix: u32 = (*pre_node).first_child;
                    while ix != !0u32 && ix != (*node).element.typed_id {
                        let mut pre_child: *mut PreNode = pre_nodes.add(ix as usize);
                        let child: *mut Node =
                            *elements.add((*pre_child).element_id as usize) as *mut Node;

                        if (*pre_child).parent != (*node).element.typed_id
                            || (*child).original_inherit_mode == InheritMode::ComponentwiseScale
                        {
                            if !(*pre_child).has_recursive_scale_helper
                                && (*child).original_inherit_mode != InheritMode::Normal
                            {
                                (*pre_child).has_recursive_scale_helper = true;

                                let child_fbx_id: u64 =
                                    *fbx_ids.add((*pre_child).element_id as usize);
                                setup_scale_helper(uc, child, child_fbx_id)?;
                                (*child).is_scale_compensate_parent = false;

                                // Traverse to children if any
                                if (*pre_child).first_child != !0u32 {
                                    ix = (*pre_child).first_child;
                                    continue;
                                }
                            }
                        }

                        // Move to next child, popping parents until we find one
                        while (*pre_child).next_child == !0u32 {
                            ix = (*pre_child).parent;
                            if ix == (*node).element.typed_id {
                                break;
                            }
                            pre_child = pre_nodes.add(ix as usize);
                        }
                        if ix != (*node).element.typed_id {
                            ix = (*pre_child).next_child;
                        }
                    }
                } else if (*uc.get()).opts.inherit_mode_handling == InheritModeHandling::Compensate
                    || (*uc.get()).opts.inherit_mode_handling
                        == InheritModeHandling::CompensateNoFallback
                {
                    // C: `(ufbx_real)ufbx_fabs(scale.x - 1.0f)` — real
                    // subtraction, double `fabs`, narrowed back to real.
                    if math::fabs((scale.x - 1.0) as f64) as Real >= scale_epsilon {
                        (*node).is_scale_compensate_parent = true;
                    }
                }
            }
        }
    }

    Ok(())
}

// `// -- Scene pre-processing` section complete (ufbx.c:18066-18543).

// -- Scene processing (ufbx.c:18545-...)
//
// PARTIAL: see the CONTINUATION POINT marker at the end of this file for the
// exact ufbx.c line the section resumes at.

// ufbx.c:18547-18554 `ufbxi_find_element_by_fbx_id`
#[inline(never)]
pub(crate) unsafe fn find_element_by_fbx_id(uc: &Context, fbx_id: u64) -> *mut Element {
    let entry: *mut FbxIdEntry = find_fbx_id(uc, fbx_id);
    if !entry.is_null() {
        return *((*uc.get()).scene.elements.data as *mut *mut Element)
            .add((*entry).element_id as usize);
    }
    ptr::null_mut()
}

// ufbx.c:18556-18562 `ufbxi_cmp_name_element_less`
#[inline(always)]
pub(crate) unsafe fn cmp_name_element_less(a: *const NameElement, b: *const NameElement) -> bool {
    if (*a)._internal_key != (*b)._internal_key {
        return (*a)._internal_key < (*b)._internal_key;
    }
    let cmp: i32 = strcmp((*a).name.data, (*b).name.data);
    if cmp != 0 {
        return cmp < 0;
    }
    ((*a).type_ as u32) < (*b).type_ as u32
}

// ufbx.c:18564-18570 `ufbxi_cmp_name_element_less_ref`
#[inline(always)]
pub(crate) unsafe fn cmp_name_element_less_ref(
    a: *const NameElement,
    name: String,
    type_: ElementType,
    key: u32,
) -> bool {
    if (*a)._internal_key != key {
        return (*a)._internal_key < key;
    }
    let cmp: i32 = str_cmp((*a).name, name);
    if cmp != 0 {
        return cmp < 0;
    }
    ((*a).type_ as u32) < type_ as u32
}

// ufbx.c:18572-18576 `ufbxi_cmp_prop_less_ref`
#[inline(always)]
pub(crate) unsafe fn cmp_prop_less_ref(a: *const Prop, name: String, key: u32) -> bool {
    if (*a)._internal_key != key {
        return (*a)._internal_key < key;
    }
    str_less((*a).name, name)
}

// ufbx.c:18578-18582 `ufbxi_cmp_prop_less_concat`
#[inline(always)]
pub(crate) unsafe fn cmp_prop_less_concat(
    a: *const Prop,
    parts: *const String,
    num_parts: usize,
    key: u32,
) -> bool {
    if (*a)._internal_key != key {
        return (*a)._internal_key < key;
    }
    concat_str_cmp(&(*a).name, parts, num_parts) < 0
}

// ufbx.c:18584-18590 `ufbxi_sort_name_elements`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn sort_name_elements(
    uc: &Context,
    name_elems: *mut NameElement,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.tmp_arr_mut_ptr(),
            uc.tmp_arr_size_mut_ptr(),
            count.wrapping_mul(size_of::<NameElement>()),
        ),
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_name_element)))"
    );
    macro_stable_sort::<NameElement>(
        32,
        name_elems,
        uc.tmp_arr() as *mut NameElement,
        count,
        |a, b| cmp_name_element_less(a, b),
    );
    Ok(())
}

// ufbx.c:18592-18610 `ufbxi_cmp_node_less`
#[inline(never)]
pub(crate) unsafe fn cmp_node_less(a: *mut Node, b: *mut Node) -> bool {
    if (*a).node_depth != (*b).node_depth {
        return (*a).node_depth < (*b).node_depth;
    }
    let a_parent: *mut Node = opt_ptr(&(*a).parent);
    let b_parent: *mut Node = opt_ptr(&(*b).parent);
    if !a_parent.is_null() && !b_parent.is_null() {
        let a_pid: u32 = (*a_parent).element.element_id;
        let b_pid: u32 = (*b_parent).element.element_id;
        if a_pid != b_pid {
            return a_pid < b_pid;
        }
    } else {
        ufbx_assert!(a_parent.is_null() && b_parent.is_null());
    }
    if (*a).is_geometry_transform_helper != (*b).is_geometry_transform_helper {
        // Sort geometry transform helpers always before rest of the children.
        return (*a).is_geometry_transform_helper as u32 > (*b).is_geometry_transform_helper as u32;
    }
    if (*a).is_scale_helper != (*b).is_scale_helper {
        // Sort scale helpers after geometry transform helpers.
        return (*a).is_scale_helper as u32 > (*b).is_scale_helper as u32;
    }
    (*a).element.element_id < (*b).element.element_id
}

// ufbx.c:18612-18618 `ufbxi_sort_node_ptrs`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn sort_node_ptrs(
    uc: &Context,
    nodes: *mut *mut Node,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.tmp_arr_mut_ptr(),
            uc.tmp_arr_size_mut_ptr(),
            count.wrapping_mul(size_of::<*mut Node>()),
        ),
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_node*)))"
    );
    macro_stable_sort::<*mut Node>(32, nodes, uc.tmp_arr() as *mut *mut Node, count, |a, b| {
        cmp_node_less(*a, *b)
    });
    Ok(())
}

// ufbx.c:18620-18625 `ufbxi_cmp_tmp_material_texture_less`
// C declares this `int`-returning, but every `return` is a boolean expression
// and the only caller is a sort comparator.
#[inline(never)]
#[must_use]
pub(crate) unsafe fn cmp_tmp_material_texture_less(
    a: *const TmpMaterialTexture,
    b: *const TmpMaterialTexture,
) -> bool {
    if (*a).material_id != (*b).material_id {
        return (*a).material_id < (*b).material_id;
    }
    if (*a).texture_id != (*b).texture_id {
        return (*a).texture_id < (*b).texture_id;
    }
    str_less((*a).prop_name, (*b).prop_name)
}

// ufbx.c:18627-18633 `ufbxi_sort_tmp_material_textures`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn sort_tmp_material_textures(
    uc: &Context,
    mat_texs: *mut TmpMaterialTexture,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.tmp_arr_mut_ptr(),
            uc.tmp_arr_size_mut_ptr(),
            count.wrapping_mul(size_of::<TmpMaterialTexture>()),
        ),
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbxi_tmp_material_texture)))"
    );
    macro_stable_sort::<TmpMaterialTexture>(
        32,
        mat_texs,
        uc.tmp_arr() as *mut TmpMaterialTexture,
        count,
        |a, b| cmp_tmp_material_texture_less(a, b),
    );
    Ok(())
}

// We need to be able to assume no padding!
// ufbx.c:18636 `ufbx_static_assert(connection_size, sizeof(ufbx_connection) == sizeof(ufbx_element*)*2 + sizeof(ufbx_string)*2);`
const _: () =
    assert!(size_of::<Connection>() == size_of::<*mut Element>() * 2 + size_of::<String>() * 2);

// ufbx.c:18638-18646 `ufbxi_cmp_connection_less`
#[inline(always)]
pub(crate) unsafe fn cmp_connection_less(
    a: *mut Connection,
    b: *mut Connection,
    index: usize,
) -> bool {
    // C-parity: `(&a->src)[index]` / `(&a->src_prop)[index]` index across the
    // two adjacent element pointers and the two adjacent strings of
    // `ufbx_connection`, which the static assert above pins as unpadded.
    let a_src: *const Ref<Element> = &raw const (*a).src;
    let b_src: *const Ref<Element> = &raw const (*b).src;
    let a_elem: *mut Element = ref_ptr(a_src.add(index));
    let b_elem: *mut Element = ref_ptr(b_src.add(index));
    if a_elem != b_elem {
        return a_elem < b_elem;
    }
    let a_prop: *const String = &raw const (*a).src_prop;
    let b_prop: *const String = &raw const (*b).src_prop;
    let mut cmp: i32 = strcmp((*a_prop.add(index)).data, (*b_prop.add(index)).data);
    if cmp != 0 {
        return cmp < 0;
    }
    cmp = strcmp((*a_prop.add(index ^ 1)).data, (*b_prop.add(index ^ 1)).data);
    cmp < 0
}

// ufbx.c:18648-18653 `ufbxi_sort_connections`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn sort_connections(
    uc: &Context,
    connections: *mut Connection,
    count: usize,
    index: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.tmp_arr_mut_ptr(),
            uc.tmp_arr_size_mut_ptr(),
            count.wrapping_mul(size_of::<Connection>()),
        ),
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_connection)))"
    );
    macro_stable_sort::<Connection>(
        32,
        connections,
        uc.tmp_arr() as *mut Connection,
        count,
        |a, b| cmp_connection_less(a as *mut Connection, b as *mut Connection, index),
    );
    Ok(())
}

// ufbx.c:18655-18663 `ufbxi_find_attribute_fbx_id`
pub(crate) unsafe fn find_attribute_fbx_id(uc: &Context, node_fbx_id: u64) -> u64 {
    let hash: u32 = hash64(node_fbx_id);
    let entry: *mut FbxAttrEntry = map_find(
        uc.fbx_attr_map_mut_ptr(),
        hash,
        &node_fbx_id as *const u64 as *const c_void,
    );
    if !entry.is_null() {
        return (*entry).attr_fbx_id;
    }
    node_fbx_id
}

// ufbx.c:18665-18780 `ufbxi_resolve_connections`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn resolve_connections(uc: &Context) -> Result<(), Fail> {
    let num_connections: usize = (*uc.get()).tmp_connections.num_items;
    let tmp_connections: *mut TmpConnection = push_pop(
        uc.tmp_mut_ptr(),
        uc.tmp_connections_mut_ptr(),
        num_connections,
    );
    buf_free(uc.tmp_connections_mut_ptr());
    ufbxi_check!(uc, !tmp_connections.is_null(), "tmp_connections");

    // NOTE: We truncate this array in case not all connections are resolved
    (*uc.get()).scene.connections_src.data =
        push::<Connection>(uc.result_mut_ptr(), num_connections);
    ufbxi_check!(
        uc,
        !(*uc.get()).scene.connections_src.data.is_null(),
        "uc->scene.connections_src.data"
    );

    // HACK: Translate property connections from node to attribute if the property name is not included
    // in the known node properties and is not a property of the node.
    if uc.version() > 0 && uc.version() < 7000 {
        // C: `ufbxi_for(ufbxi_tmp_connection, tmp_conn, tmp_connections, num_connections)`
        let mut tmp_conn: *mut TmpConnection = tmp_connections;
        let tmp_conn_end: *mut TmpConnection = tmp_connections.add(num_connections);
        while tmp_conn != tmp_conn_end {
            if (*tmp_conn).src_prop.length > 0
                && !is_node_property_name(uc, (*tmp_conn).src_prop.data)
            {
                let src: *mut Element = find_element_by_fbx_id(uc, (*tmp_conn).src);
                if src.is_null()
                    || find_prop_len(
                        &(*src).props,
                        (*tmp_conn).src_prop.data,
                        (*tmp_conn).src_prop.length,
                    )
                    .is_null()
                {
                    (*tmp_conn).src = find_attribute_fbx_id(uc, (*tmp_conn).src);
                }
            }
            if (*tmp_conn).dst_prop.length > 0
                && !is_node_property_name(uc, (*tmp_conn).dst_prop.data)
            {
                let dst: *mut Element = find_element_by_fbx_id(uc, (*tmp_conn).dst);
                if dst.is_null()
                    || find_prop_len(
                        &(*dst).props,
                        (*tmp_conn).dst_prop.data,
                        (*tmp_conn).dst_prop.length,
                    )
                    .is_null()
                {
                    (*tmp_conn).dst = find_attribute_fbx_id(uc, (*tmp_conn).dst);
                }
            }
            tmp_conn = tmp_conn.add(1);
        }
    }

    // C: `ufbxi_for(ufbxi_tmp_connection, tmp_conn, tmp_connections, num_connections)`
    // — indexed here because the body `continue`s (the C `for` advances the
    // iterator in its increment clause).
    for conn_ix in 0..num_connections {
        let tmp_conn: *mut TmpConnection = tmp_connections.add(conn_ix);
        let mut src: *mut Element = find_element_by_fbx_id(uc, (*tmp_conn).src);
        let mut dst: *mut Element = find_element_by_fbx_id(uc, (*tmp_conn).dst);
        if src.is_null() || dst.is_null() {
            continue;
        }

        if !(*uc.get()).opts.disable_quirks {
            // Some exporters connect arbitrary non-nodes to root breaking further code, ignore those connections here!
            if (*dst).type_ == ElementType::Node
                && (*src).type_ != ElementType::Node
                && (*(dst as *mut Node)).is_root
            {
                ufbxi_check!(
                    uc,
                    ufbxi_warnf_tag!(
                        uc,
                        WarningType::BadElementConnectedToRoot,
                        (*src).element_id,
                        "Non-node element connected to root"
                    )
                    .is_ok(),
                    "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_BAD_ELEMENT_CONNECTED_TO_ROOT, (src->element_id), \"Non-node element connected to root\")"
                );
                continue;
            }
        }

        // Remap connections to geometry transform helpers if necessary, see `ufbxi_setup_geometry_transform_helper()` for how these are setup.
        if uc.has_geometry_transform_nodes() {
            if (*dst).type_ == ElementType::Node
                && (*src).type_ as u32 >= ELEMENT_TYPE_FIRST_ATTRIB
                && (*src).type_ as u32 <= ELEMENT_TYPE_LAST_ATTRIB
            {
                let node: *mut Node = dst as *mut Node;
                if (*node).has_geometry_transform {
                    let extra: *mut NodeExtra =
                        get_element_extra(uc, (*node).element.element_id) as *mut NodeExtra;
                    ufbx_assert!(!extra.is_null());
                    dst = *((*uc.get()).scene.elements.data as *mut *mut Element)
                        .add((*extra).geometry_helper_id as usize);
                    ufbx_assert!(
                        (*dst).type_ == ElementType::Node
                            && (*(dst as *mut Node)).is_geometry_transform_helper
                    );
                }
            }
        }

        // Remap connections to scale helpers if necessary, see `ufbxi_setup_scale_helper()` for how these are setup.
        if uc.has_scale_helper_nodes() {
            if (*dst).type_ == ElementType::Node {
                let dst_node: *mut Node = dst as *mut Node;
                let scale_helper: *mut Node = opt_ptr(&(*dst_node).scale_helper);
                if !scale_helper.is_null() {
                    if (*src).type_ == ElementType::Node {
                        let src_node: *mut Node = src as *mut Node;
                        if !(*src_node).is_scale_helper
                            && (*src_node).original_inherit_mode == InheritMode::Normal
                        {
                            dst = &mut (*scale_helper).element as *mut Element;
                        }
                    } else if (*src).type_ == ElementType::AnimValue {
                        if (*tmp_conn).dst_prop.data == sp::Lcl_Scaling.as_ptr() {
                            dst = &mut (*scale_helper).element as *mut Element;
                        }
                    } else {
                        dst = &mut (*scale_helper).element as *mut Element;
                    }
                }
            } else if (*src).type_ == ElementType::Node {
                let src_node: *mut Node = src as *mut Node;
                let scale_helper: *mut Node = opt_ptr(&(*src_node).scale_helper);
                if !scale_helper.is_null() {
                    if (*dst).type_ == ElementType::SkinCluster {
                        src = &mut (*scale_helper).element as *mut Element;
                    }
                }
            }
        }

        // Translate deformers to point to the geometry in 6100, we don't need to worry about
        // blend shapes here as they're always connected synthetically in older files.
        if uc.version() > 0 && uc.version() < 7000 && (*dst).type_ == ElementType::Node {
            if (*src).type_ == ElementType::SkinDeformer
                || (*src).type_ == ElementType::CacheDeformer
            {
                let dst_id: u64 = find_attribute_fbx_id(uc, (*tmp_conn).dst);
                let dst_elem: *mut Element = find_element_by_fbx_id(uc, dst_id);
                if !dst_elem.is_null() {
                    dst = dst_elem;
                }
            }
        }

        let conn: *mut Connection = ((*uc.get()).scene.connections_src.data as *mut Connection)
            .add((*uc.get()).scene.connections_src.count);
        (*uc.get()).scene.connections_src.count =
            (*uc.get()).scene.connections_src.count.wrapping_add(1);
        (*conn).src = Ref::from_ptr(src);
        (*conn).dst = Ref::from_ptr(dst);
        (*conn).src_prop = (*tmp_conn).src_prop;
        (*conn).dst_prop = (*tmp_conn).dst_prop;
    }

    (*uc.get()).scene.connections_dst.count = (*uc.get()).scene.connections_src.count;
    (*uc.get()).scene.connections_dst.data = push_copy::<Connection>(
        uc.result_mut_ptr(),
        (*uc.get()).scene.connections_src.count,
        (*uc.get()).scene.connections_src.data,
    );
    ufbxi_check!(
        uc,
        !(*uc.get()).scene.connections_dst.data.is_null(),
        "uc->scene.connections_dst.data"
    );

    sort_connections(
        uc,
        (*uc.get()).scene.connections_src.data as *mut Connection,
        (*uc.get()).scene.connections_src.count,
        0,
    )?;
    sort_connections(
        uc,
        (*uc.get()).scene.connections_dst.data as *mut Connection,
        (*uc.get()).scene.connections_dst.count,
        1,
    )?;

    // We don't need the temporary connections at this point anymore
    buf_free(uc.tmp_connections_mut_ptr());

    Ok(())
}

// ufbx.c:18782-18912 `ufbxi_add_connections_to_elements`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn add_connections_to_elements(uc: &Context) -> Result<(), Fail> {
    let mut conn_src: *mut Connection = (*uc.get()).scene.connections_src.data as *mut Connection;
    let conn_src_end: *mut Connection = add_ptr(conn_src, (*uc.get()).scene.connections_src.count);
    let mut conn_dst: *mut Connection = (*uc.get()).scene.connections_dst.data as *mut Connection;
    let conn_dst_end: *mut Connection = add_ptr(conn_dst, (*uc.get()).scene.connections_dst.count);

    // C: `ufbxi_for_ptr(ufbx_element, p_elem, uc->scene.elements.data, uc->scene.elements.count)`
    let mut p_elem: *mut *mut Element = (*uc.get()).scene.elements.data as *mut *mut Element;
    let p_elem_end: *mut *mut Element = p_elem.add((*uc.get()).scene.elements.count);
    while p_elem != p_elem_end {
        let elem: *mut Element = *p_elem;
        let id: u32 = (*elem).element_id;

        while conn_src < conn_src_end && (*ref_ptr(&(*conn_src).src)).element_id < id {
            conn_src = conn_src.add(1);
        }
        while conn_dst < conn_dst_end && (*ref_ptr(&(*conn_dst).dst)).element_id < id {
            conn_dst = conn_dst.add(1);
        }
        let mut src_end: *mut Connection = conn_src;
        let mut dst_end: *mut Connection = conn_dst;

        while src_end < conn_src_end && (*ref_ptr(&(*src_end).src)).element_id == id {
            src_end = src_end.add(1);
        }
        while dst_end < conn_dst_end && (*ref_ptr(&(*dst_end).dst)).element_id == id {
            dst_end = dst_end.add(1);
        }

        (*elem).connections_src.data = conn_src;
        (*elem).connections_src.count = to_size(src_end.offset_from(conn_src));
        (*elem).connections_dst.data = conn_dst;
        (*elem).connections_dst.count = to_size(dst_end.offset_from(conn_dst));

        // Setup animated properties
        // TODO: It seems we're invalidating a lot of properties here actually, maybe they
        // should be initially pushed to `tmp` instead of result if this happens so much..
        {
            let mut prop: *mut Prop = (*elem).props.props.data as *mut Prop;
            let prop_end: *mut Prop = add_ptr(prop, (*elem).props.props.count);
            let mut copy_start: *mut Prop = prop;
            let mut needs_copy: bool = false;
            let mut num_animated: usize = 0;
            let mut num_synthetic: usize = 0;

            loop {
                // Scan to the next animation connection
                while conn_dst < dst_end {
                    if (*conn_dst).dst_prop.length == 0 {
                        conn_dst = conn_dst.add(1);
                        continue;
                    }
                    if (*conn_dst).src_prop.length > 0 {
                        break;
                    }
                    if (*ref_ptr(&(*conn_dst).src)).type_ == ElementType::AnimValue {
                        break;
                    }
                    conn_dst = conn_dst.add(1);
                }

                let mut name: String = EMPTY_STRING.0;
                if conn_dst < dst_end {
                    name = (*conn_dst).dst_prop;
                }
                if name.length == 0 {
                    break;
                }

                // NOTE: "Animated" properties also include connected ones as we need
                // to resolve them during evaluation
                num_animated = num_animated.wrapping_add(1);

                let mut anim_value: *mut AnimValue = ptr::null_mut();
                let mut flags: u32 = 0;
                while conn_dst < dst_end && (*conn_dst).dst_prop.data == name.data {
                    if (*conn_dst).src_prop.length > 0 {
                        flags |= PropFlags::CONNECTED.raw();
                    } else if (*ref_ptr(&(*conn_dst).src)).type_ == ElementType::AnimValue {
                        anim_value = ref_ptr(&(*conn_dst).src) as *mut AnimValue;
                        flags |= PropFlags::ANIMATED.raw();
                    }
                    conn_dst = conn_dst.add(1);
                }

                let key: u32 = get_name_key(name.data, name.length);
                while prop != prop_end && name_key_less(prop, name.data, name.length, key) {
                    prop = prop.add(1);
                }

                if prop != prop_end && (*prop).name.data == name.data {
                    (*prop).flags = PropFlags::from_raw((*prop).flags.raw() | flags);
                } else {
                    // Animated property that is not in the element property list
                    // Copy the preceding properties to the stack, then push a
                    // synthetic property for the animated property.
                    ufbxi_check!(
                        uc,
                        !push_copy::<Prop>(
                            uc.tmp_stack_mut_ptr(),
                            to_size(prop.offset_from(copy_start)),
                            copy_start,
                        )
                        .is_null(),
                        "((ufbx_prop*)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_prop), (((size_t)(prop - copy_start))), (copy_start)))"
                    );
                    copy_start = prop;
                    needs_copy = true;

                    // Let's hope we can find the property in the defaults at least
                    // C: `ufbx_prop anim_def_prop;` — only read after the
                    // `memset` below (no upstream `ufbxi_uninit` marker).
                    let mut anim_def_prop = MaybeUninit::<Prop>::uninit();
                    let mut def_prop: *mut Prop = ptr::null_mut();
                    if (*elem).props.defaults.is_some() {
                        def_prop =
                            find_prop_with_key(opt_ptr(&(*elem).props.defaults), name.data, key);
                    } else if !anim_value.is_null() {
                        let anim_def: *mut Prop = anim_def_prop.as_mut_ptr();
                        ptr::write_bytes(anim_def as *mut u8, 0, size_of::<Prop>());
                        // Hack a couple of common types
                        let mut type_: PropType = PropType::Unknown;
                        if name.data == sp::Lcl_Translation.as_ptr() {
                            type_ = PropType::Translation;
                        } else if name.data == sp::Lcl_Rotation.as_ptr() {
                            type_ = PropType::Rotation;
                        } else if name.data == sp::Lcl_Scaling.as_ptr() {
                            type_ = PropType::Scaling;
                            // C-parity: `value_vec3` is the `ufbx_prop` value union's
                            // 3-real view; the generated struct keeps only `value_vec4`.
                            let value_vec3: *mut Vec3 =
                                &mut (*anim_def).value_vec4 as *mut Vec4 as *mut Vec3;
                            (*value_vec3).x = 1.0;
                            (*value_vec3).y = 1.0;
                            (*value_vec3).z = 1.0;
                        }
                        // Property values are only defined in anim_props on legacy files
                        if uc.version() < 6000 {
                            *(&mut (*anim_def).value_vec4 as *mut Vec4 as *mut Vec3) =
                                (*anim_value).default_value;
                        }
                        (*anim_def).type_ = type_;
                        def_prop = anim_def;
                    } else {
                        flags |= PropFlags::NO_VALUE.raw();
                    }

                    let new_prop: *mut Prop = push_zero(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !new_prop.is_null(), "new_prop");
                    if !def_prop.is_null() {
                        *new_prop = *def_prop;
                    }
                    flags |= (*new_prop).flags.raw();
                    (*new_prop).flags = PropFlags::from_raw(
                        PropFlags::ANIMATABLE.raw() | PropFlags::SYNTHETIC.raw() | flags,
                    );
                    (*new_prop).name = name;
                    (*new_prop)._internal_key = key;
                    (*new_prop).value_str = EMPTY_STRING.0;
                    (*new_prop).value_blob = EMPTY_BLOB.0;
                    num_synthetic = num_synthetic.wrapping_add(1);
                }
            }

            // Copy the properties if necessary
            if needs_copy {
                let num_new_props: usize = (*elem).props.props.count.wrapping_add(num_synthetic);
                ufbxi_check!(
                    uc,
                    !push_copy::<Prop>(
                        uc.tmp_stack_mut_ptr(),
                        to_size(prop_end.offset_from(copy_start)),
                        copy_start,
                    )
                    .is_null(),
                    "((ufbx_prop*)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_prop), (((size_t)(prop_end - copy_start))), (copy_start)))"
                );
                (*elem).props.props.data =
                    push_pop::<Prop>(uc.result_mut_ptr(), uc.tmp_stack_mut_ptr(), num_new_props);
                ufbxi_check!(
                    uc,
                    !(*elem).props.props.data.is_null(),
                    "elem->props.props.data"
                );
                (*elem).props.props.count = num_new_props;
            }
            (*elem).props.num_animated = num_animated;
        }

        conn_src = src_end;
        conn_dst = dst_end;
        p_elem = p_elem.add(1);
    }

    Ok(())
}

// ufbx.c:18914-18994 `ufbxi_linearize_nodes`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn linearize_nodes(uc: &Context) -> Result<(), Fail> {
    let num_nodes: usize = (*uc.get()).tmp_node_ids.num_items;
    let node_ids: *mut u32 = push_pop(uc.tmp_mut_ptr(), uc.tmp_node_ids_mut_ptr(), num_nodes);
    buf_free(uc.tmp_node_ids_mut_ptr());
    ufbxi_check!(uc, !node_ids.is_null(), "node_ids");

    let node_ptrs: *mut *mut Node = push(uc.tmp_stack_mut_ptr(), num_nodes);
    ufbxi_check!(uc, !node_ptrs.is_null(), "node_ptrs");

    // Fetch the node pointers
    for i in 0..num_nodes {
        *node_ptrs.add(i) = *((*uc.get()).scene.elements.data as *mut *mut Element)
            .add(*node_ids.add(i) as usize) as *mut Node;
        ufbx_assert!((**node_ptrs.add(i)).element.type_ == ElementType::Node);
    }

    // C reads `node_ptrs[0]` unconditionally; there is always at least the root
    // node in `tmp_node_ids` by the time this runs.
    (*uc.get()).scene.root_node = Ref::from_ptr(*node_ptrs.add(0));

    let node_offsets: *mut usize = push_pop(
        uc.tmp_stack_mut_ptr(),
        &mut (*uc.get()).tmp_typed_element_offsets[ElementType::Node as usize],
        num_nodes,
    );
    ufbxi_check!(uc, !node_offsets.is_null(), "node_offsets");

    // Hook up the parent nodes, we'll assume that there's no cycles at this point
    // C: `ufbxi_for_ptr(ufbx_node, p_node, node_ptrs, num_nodes)`
    let mut p_node: *mut *mut Node = node_ptrs;
    let p_node_end: *mut *mut Node = node_ptrs.add(num_nodes);
    while p_node != p_node_end {
        let node: *mut Node = *p_node;

        // Pre-6000 files don't have any explicit root connections so they must always
        // be connected to the root..
        if opt_ptr(&(*node).parent).is_null()
            && !((*uc.get()).opts.allow_nodes_out_of_root && uc.version() >= 6000)
        {
            if node != ref_ptr(&(*uc.get()).scene.root_node) {
                (*node).parent = Some((*uc.get()).scene.root_node);
            }
        }

        // C: `ufbxi_for_list(ufbx_connection, conn, node->element.connections_dst)`
        let mut conn: *mut Connection = (*node).element.connections_dst.data as *mut Connection;
        let conn_end: *mut Connection = conn.add((*node).element.connections_dst.count);
        while conn != conn_end {
            if (*conn).src_prop.length > 0 || (*conn).dst_prop.length > 0 {
                conn = conn.add(1);
                continue;
            }
            if (*ref_ptr(&(*conn).src)).type_ != ElementType::Node {
                conn = conn.add(1);
                continue;
            }
            (*(ref_ptr(&(*conn).src) as *mut Node)).parent = opt_ref(node);
            conn = conn.add(1);
        }
        p_node = p_node.add(1);
    }

    // Count the parent depths and child amounts
    let mut p_node: *mut *mut Node = node_ptrs;
    while p_node != p_node_end {
        let node: *mut Node = *p_node;
        let mut depth: u32 = 0;

        // C: `for (ufbx_node *p = node->parent; p; p = p->parent)`
        let mut p: *mut Node = opt_ptr(&(*node).parent);
        while !p.is_null() {
            depth = depth.wrapping_add((*p).node_depth.wrapping_add(1));
            if (*p).node_depth > 0 {
                break;
            }
            ufbxi_check_msg!(
                uc,
                depth as usize <= num_nodes,
                "Cyclic node hierarchy",
                "depth <= num_nodes"
            );
            p = opt_ptr(&(*p).parent);
        }

        if (*uc.get()).opts.node_depth_limit > 0 {
            ufbxi_check_msg!(
                uc,
                depth <= (*uc.get()).opts.node_depth_limit,
                "Node depth limit exceeded",
                "depth <= uc->opts.node_depth_limit"
            );
        }
        (*node).node_depth = depth;

        // Second pass to cache the depths to avoid O(n^2)
        let mut p: *mut Node = opt_ptr(&(*node).parent);
        while !p.is_null() {
            depth = depth.wrapping_sub(1);
            if depth <= (*p).node_depth {
                break;
            }
            (*p).node_depth = depth;
            p = opt_ptr(&(*p).parent);
        }
        p_node = p_node.add(1);
    }

    sort_node_ptrs(uc, node_ptrs, num_nodes)?;

    for i in 0..num_nodes as u32 {
        let p_offset: *mut usize = push(
            &mut (*uc.get()).tmp_typed_element_offsets[ElementType::Node as usize],
            1,
        );
        ufbxi_check!(uc, !p_offset.is_null(), "p_offset");
        let node: *mut Node = *node_ptrs.add(i as usize);

        let original_id: u32 = (*node).element.typed_id;
        (*node).element.typed_id = i;
        *p_offset = *node_offsets.add(original_id as usize);
    }

    // Pop the temporary arrays
    pop::<usize>(uc.tmp_stack_mut_ptr(), num_nodes, ptr::null_mut());
    pop::<*mut Node>(uc.tmp_stack_mut_ptr(), num_nodes, ptr::null_mut());

    Ok(())
}

// ufbx.c:18997-19014 `ufbxi_find_dst_connections`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn find_dst_connections(
    element: *mut Element,
    prop: *const u8,
) -> List<Connection> {
    let mut prop: *const u8 = prop;
    if prop.is_null() {
        prop = EMPTY_CHAR.as_ptr();
    }

    let mut begin: usize = (*element).connections_dst.count;
    let mut end: usize = begin;

    macro_lower_bound_eq(
        32,
        &mut begin,
        (*element).connections_dst.data,
        0,
        (*element).connections_dst.count,
        |a| strcmp((*a).dst_prop.data, prop) < 0,
        |a| (*a).dst_prop.data == prop && (*a).src_prop.length == 0,
    );

    macro_upper_bound_eq(
        32,
        &mut end,
        (*element).connections_dst.data,
        begin,
        (*element).connections_dst.count,
        |a| (*a).dst_prop.data == prop && (*a).src_prop.length == 0,
    );

    // C: `ufbx_connection_list result = { element->connections_dst.data + begin, end - begin };`
    // `List<T>` carries a private `PhantomData` marker, so the C aggregate
    // initializer becomes a zeroed value with both public fields written.
    let mut result: List<Connection> = MaybeUninit::zeroed().assume_init();
    result.data = (*element).connections_dst.data.add(begin);
    result.count = end - begin;
    result
}

// ufbx.c:19016-19033 `ufbxi_find_src_connections`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn find_src_connections(
    element: *mut Element,
    prop: *const u8,
) -> List<Connection> {
    let mut prop: *const u8 = prop;
    if prop.is_null() {
        prop = EMPTY_CHAR.as_ptr();
    }

    let mut begin: usize = (*element).connections_src.count;
    let mut end: usize = begin;

    macro_lower_bound_eq(
        32,
        &mut begin,
        (*element).connections_src.data,
        0,
        (*element).connections_src.count,
        |a| strcmp((*a).src_prop.data, prop) < 0,
        |a| (*a).src_prop.data == prop && (*a).dst_prop.length == 0,
    );

    macro_upper_bound_eq(
        32,
        &mut end,
        (*element).connections_src.data,
        begin,
        (*element).connections_src.count,
        |a| (*a).src_prop.data == prop && (*a).dst_prop.length == 0,
    );

    // C: `ufbx_connection_list result = { element->connections_src.data + begin, end - begin };`
    let mut result: List<Connection> = MaybeUninit::zeroed().assume_init();
    result.data = (*element).connections_src.data.add(begin);
    result.count = end - begin;
    result
}

// ufbx.c:19035-19045 `ufbxi_get_element_node`
#[must_use]
pub(crate) unsafe fn get_element_node(element: *mut Element) -> *mut Element {
    if element.is_null() {
        return ptr::null_mut();
    }
    if (*element).type_ == ElementType::Node {
        let node: *mut Node = element as *mut Node;
        if (*node).is_geometry_transform_helper {
            return opt_ptr(&(*node).parent) as *mut Element;
        }
        ptr::null_mut()
    } else {
        // C: `return element->instances.count > 0 ? &element->instances.data[0]->element : NULL;`
        if (*element).instances.count > 0 {
            &raw mut (*ref_ptr((*element).instances.data)).element
        } else {
            ptr::null_mut()
        }
    }
}

// ufbx.c:19047-19083 `ufbxi_fetch_dst_elements`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_dst_elements(
    uc: &Context,
    p_dst_list: *mut c_void,
    element: *mut Element,
    search_node: bool,
    ignore_duplicates: bool,
    prop: *const u8,
    src_type: ElementType,
) -> Result<(), Fail> {
    let mut element: *mut Element = element;
    let mut num_elements: usize = 0;

    loop {
        let conns: List<Connection> = find_dst_connections(element, prop);
        // C: `ufbxi_for_list(ufbx_connection, conn, conns)` — indexed here
        // because the body `continue`s (the C `for` advances the iterator in
        // its increment clause).
        for conn_ix in 0..conns.count {
            let conn: *mut Connection = (conns.data as *mut Connection).add(conn_ix);
            if (*ref_ptr(&(*conn).src)).type_ == src_type {
                if ignore_duplicates {
                    let element_id: u32 = (*ref_ptr(&(*conn).src)).element_id;
                    if *uc.tmp_element_flag().add(element_id as usize) != 0 {
                        ufbxi_check!(
                            uc,
                            ufbxi_warnf_tag!(
                                uc,
                                WarningType::DuplicateConnection,
                                element_id,
                                "Duplicate connection to %u",
                                (*element).element_id
                            )
                            .is_ok(),
                            "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_DUPLICATE_CONNECTION, (element_id), \"Duplicate connection to %u\", element->element_id)"
                        );
                        continue;
                    }
                    *uc.tmp_element_flag().add(element_id as usize) = 1;
                }
                let p_elem: *mut *mut Element = push(uc.tmp_stack_mut_ptr(), 1);
                ufbxi_check!(uc, !p_elem.is_null(), "p_elem");
                *p_elem = ref_ptr(&(*conn).src);
                num_elements += 1;
            }
        }

        if !(search_node && {
            element = get_element_node(element);
            !element.is_null()
        }) {
            break;
        }
    }

    let list: *mut RefList<Element> = p_dst_list as *mut RefList<Element>;
    (*list).data =
        push_pop::<*mut Element>(uc.result_mut_ptr(), uc.tmp_stack_mut_ptr(), num_elements)
            as *const Ref<Element>;
    (*list).count = num_elements;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    if ignore_duplicates {
        // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, *list)`
        let mut p_elem: *mut *mut Element = (*list).data as *mut *mut Element;
        let p_elem_end: *mut *mut Element = add_ptr(p_elem, (*list).count);
        while p_elem != p_elem_end {
            *(*uc.get())
                .tmp_element_flag
                .add((**p_elem).element_id as usize) = 0;
            p_elem = p_elem.add(1);
        }
    }

    Ok(())
}

// ufbx.c:19085-19121 `ufbxi_fetch_src_elements`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_src_elements(
    uc: &Context,
    p_dst_list: *mut c_void,
    element: *mut Element,
    search_node: bool,
    ignore_duplicates: bool,
    prop: *const u8,
    dst_type: ElementType,
) -> Result<(), Fail> {
    let mut element: *mut Element = element;
    let mut num_elements: usize = 0;

    loop {
        let conns: List<Connection> = find_src_connections(element, prop);
        // C: `ufbxi_for_list(ufbx_connection, conn, conns)` — indexed here
        // because the body `continue`s.
        for conn_ix in 0..conns.count {
            let conn: *mut Connection = (conns.data as *mut Connection).add(conn_ix);
            if (*ref_ptr(&(*conn).dst)).type_ == dst_type {
                if ignore_duplicates {
                    let element_id: u32 = (*ref_ptr(&(*conn).dst)).element_id;
                    if *uc.tmp_element_flag().add(element_id as usize) != 0 {
                        ufbxi_check!(
                            uc,
                            ufbxi_warnf_tag!(
                                uc,
                                WarningType::DuplicateConnection,
                                element_id,
                                "Duplicate connection to %u",
                                (*element).element_id
                            )
                            .is_ok(),
                            "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_DUPLICATE_CONNECTION, (element_id), \"Duplicate connection to %u\", element->element_id)"
                        );
                        continue;
                    }
                    *uc.tmp_element_flag().add(element_id as usize) = 1;
                }
                let p_elem: *mut *mut Element = push(uc.tmp_stack_mut_ptr(), 1);
                ufbxi_check!(uc, !p_elem.is_null(), "p_elem");
                *p_elem = ref_ptr(&(*conn).dst);
                num_elements += 1;
            }
        }

        if !(search_node && {
            element = get_element_node(element);
            !element.is_null()
        }) {
            break;
        }
    }

    let list: *mut RefList<Element> = p_dst_list as *mut RefList<Element>;
    (*list).data =
        push_pop::<*mut Element>(uc.result_mut_ptr(), uc.tmp_stack_mut_ptr(), num_elements)
            as *const Ref<Element>;
    (*list).count = num_elements;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    if ignore_duplicates {
        // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, *list)`
        let mut p_elem: *mut *mut Element = (*list).data as *mut *mut Element;
        let p_elem_end: *mut *mut Element = add_ptr(p_elem, (*list).count);
        while p_elem != p_elem_end {
            *(*uc.get())
                .tmp_element_flag
                .add((**p_elem).element_id as usize) = 0;
            p_elem = p_elem.add(1);
        }
    }

    Ok(())
}

// ufbx.c:19123-19135 `ufbxi_fetch_dst_element`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_dst_element(
    element: *mut Element,
    search_node: bool,
    prop: *const u8,
    src_type: ElementType,
) -> *mut Element {
    let mut element: *mut Element = element;

    loop {
        let conns: List<Connection> = find_dst_connections(element, prop);
        // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
        let mut conn: *mut Connection = conns.data as *mut Connection;
        let conn_end: *mut Connection = add_ptr(conn, conns.count);
        while conn != conn_end {
            if (*ref_ptr(&(*conn).src)).type_ == src_type {
                return ref_ptr(&(*conn).src);
            }
            conn = conn.add(1);
        }

        if !(search_node && {
            element = get_element_node(element);
            !element.is_null()
        }) {
            break;
        }
    }

    ptr::null_mut()
}

// ufbx.c:19137-19149 `ufbxi_fetch_src_element`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_src_element(
    element: *mut Element,
    search_node: bool,
    prop: *const u8,
    dst_type: ElementType,
) -> *mut Element {
    let mut element: *mut Element = element;

    loop {
        let conns: List<Connection> = find_src_connections(element, prop);
        // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
        let mut conn: *mut Connection = conns.data as *mut Connection;
        let conn_end: *mut Connection = add_ptr(conn, conns.count);
        while conn != conn_end {
            if (*ref_ptr(&(*conn).dst)).type_ == dst_type {
                return ref_ptr(&(*conn).dst);
            }
            conn = conn.add(1);
        }

        if !(search_node && {
            element = get_element_node(element);
            !element.is_null()
        }) {
            break;
        }
    }

    ptr::null_mut()
}

// ufbx.c:19151-19173 `ufbxi_fetch_textures`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_textures(
    uc: &Context,
    list: *mut List<MaterialTexture>,
    element: *mut Element,
    search_node: bool,
) -> Result<(), Fail> {
    let mut element: *mut Element = element;
    let mut num_textures: usize = 0;

    loop {
        // C: `ufbxi_for_list(ufbx_connection, conn, element->connections_dst)`
        // — indexed here because the body `continue`s.
        for conn_ix in 0..(*element).connections_dst.count {
            let conn: *mut Connection =
                ((*element).connections_dst.data as *mut Connection).add(conn_ix);
            if (*conn).src_prop.length > 0 {
                continue;
            }
            if (*ref_ptr(&(*conn).src)).type_ == ElementType::Texture {
                let tex: *mut MaterialTexture = push(uc.tmp_stack_mut_ptr(), 1);
                ufbxi_check!(uc, !tex.is_null(), "tex");
                // C: `tex->shader_prop = tex->material_prop = conn->dst_prop;`
                (*tex).material_prop = (*conn).dst_prop;
                (*tex).shader_prop = (*tex).material_prop;
                (*tex).texture = Ref::from_ptr(ref_ptr(&(*conn).src) as *mut Texture);
                num_textures += 1;
            }
        }

        if !(search_node && {
            element = get_element_node(element);
            !element.is_null()
        }) {
            break;
        }
    }

    (*list).data =
        push_pop::<MaterialTexture>(uc.result_mut_ptr(), uc.tmp_stack_mut_ptr(), num_textures);
    (*list).count = num_textures;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    Ok(())
}

// ufbx.c:19175-19197 `ufbxi_fetch_mesh_materials`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_mesh_materials(
    uc: &Context,
    list: *mut RefList<Material>,
    element: *mut Element,
    search_node: bool,
) -> Result<(), Fail> {
    let mut element: *mut Element = element;
    let mut num_materials: usize = 0;

    loop {
        let conns: List<Connection> = find_dst_connections(element, ptr::null());
        // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
        let mut conn: *mut Connection = conns.data as *mut Connection;
        let conn_end: *mut Connection = add_ptr(conn, conns.count);
        while conn != conn_end {
            if (*ref_ptr(&(*conn).src)).type_ == ElementType::Material {
                let mat: *mut Material = ref_ptr(&(*conn).src) as *mut Material;
                ufbxi_check!(
                    uc,
                    !push_copy::<*mut Material>(uc.tmp_stack_mut_ptr(), 1, &mat).is_null(),
                    "((ufbx_material**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_material*), (1), (&mat)))"
                );
                num_materials += 1;
            }
            conn = conn.add(1);
        }

        if num_materials > 0 {
            break;
        }

        if !(search_node && {
            element = get_element_node(element);
            !element.is_null()
        }) {
            break;
        }
    }

    (*list).data =
        push_pop::<*mut Material>(uc.result_mut_ptr(), uc.tmp_stack_mut_ptr(), num_materials)
            as *const Ref<Material>;
    (*list).count = num_materials;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    Ok(())
}

// ufbx.c:19199-19219 `ufbxi_fetch_deformers`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_deformers(
    uc: &Context,
    list: *mut RefList<Element>,
    element: *mut Element,
    search_node: bool,
) -> Result<(), Fail> {
    let mut element: *mut Element = element;
    let mut num_deformers: usize = 0;

    loop {
        // C: `ufbxi_for_list(ufbx_connection, conn, element->connections_dst)`
        // — indexed here because the body `continue`s.
        for conn_ix in 0..(*element).connections_dst.count {
            let conn: *mut Connection =
                ((*element).connections_dst.data as *mut Connection).add(conn_ix);
            if (*conn).src_prop.length > 0 {
                continue;
            }
            let type_: ElementType = (*ref_ptr(&(*conn).src)).type_;
            if type_ == ElementType::SkinDeformer
                || type_ == ElementType::BlendDeformer
                || type_ == ElementType::CacheDeformer
            {
                ufbxi_check!(
                    uc,
                    !push_copy::<*mut Element>(
                        uc.tmp_stack_mut_ptr(),
                        1,
                        &(*conn).src as *const Ref<Element> as *const *mut Element,
                    )
                    .is_null(),
                    "((ufbx_element**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_element*), (1), (&conn->src)))"
                );
                num_deformers += 1;
            }
        }

        if !(search_node && {
            element = get_element_node(element);
            !element.is_null()
        }) {
            break;
        }
    }

    (*list).data =
        push_pop::<*mut Element>(uc.result_mut_ptr(), uc.tmp_stack_mut_ptr(), num_deformers)
            as *const Ref<Element>;
    (*list).count = num_deformers;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    Ok(())
}

// ufbx.c:19221-19239 `ufbxi_fetch_blend_keyframes`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_blend_keyframes(
    uc: &Context,
    list: *mut List<BlendKeyframe>,
    element: *mut Element,
) -> Result<(), Fail> {
    let mut num_keyframes: usize = 0;

    let conns: List<Connection> = find_dst_connections(element, ptr::null());
    // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
    let mut conn: *mut Connection = conns.data as *mut Connection;
    let conn_end: *mut Connection = add_ptr(conn, conns.count);
    while conn != conn_end {
        if (*ref_ptr(&(*conn).src)).type_ == ElementType::BlendShape {
            // C: `ufbx_blend_keyframe key = { (ufbx_blend_shape*)conn->src };`
            // — the remaining fields are zero-initialized.
            let key = BlendKeyframe {
                shape: Ref::from_ptr(ref_ptr(&(*conn).src) as *mut BlendShape),
                target_weight: 0.0,
                effective_weight: 0.0,
            };
            ufbxi_check!(
                uc,
                !push_copy::<BlendKeyframe>(uc.tmp_stack_mut_ptr(), 1, &key).is_null(),
                "((ufbx_blend_keyframe*)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_blend_keyframe), (1), (&key)))"
            );
            num_keyframes += 1;
        }
        conn = conn.add(1);
    }

    (*list).data =
        push_pop::<BlendKeyframe>(uc.result_mut_ptr(), uc.tmp_stack_mut_ptr(), num_keyframes);
    (*list).count = num_keyframes;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    Ok(())
}

// ufbx.c:19241-19262 `ufbxi_fetch_texture_layers`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_texture_layers(
    uc: &Context,
    list: *mut List<TextureLayer>,
    element: *mut Element,
) -> Result<(), Fail> {
    let mut num_layers: usize = 0;

    let conns: List<Connection> = find_dst_connections(element, ptr::null());
    // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
    let mut conn: *mut Connection = conns.data as *mut Connection;
    let conn_end: *mut Connection = add_ptr(conn, conns.count);
    while conn != conn_end {
        if (*ref_ptr(&(*conn).src)).type_ == ElementType::Texture {
            let texture: *mut Texture = ref_ptr(&(*conn).src) as *mut Texture;
            // C: `ufbx_texture_layer layer = { texture };` — the remaining
            // fields are zero-initialized (`UFBX_BLEND_TRANSLUCENT` == 0).
            let mut layer = TextureLayer {
                texture: Ref::from_ptr(texture),
                blend_mode: BlendMode::Translucent,
                alpha: 0.0,
            };
            layer.alpha = find_real(&(*texture).element.props, sp::Texture_alpha.as_ptr(), 1.0);
            // C: `(ufbx_blend_mode)ufbxi_find_enum(...)` — `ufbxi_find_enum`
            // clamps the result to `[0, UFBX_BLEND_OVERLAY]`, every value of
            // which is a valid `ufbx_blend_mode`.
            layer.blend_mode = core::mem::transmute::<u32, BlendMode>(find_enum(
                &(*texture).element.props,
                sp::BlendMode.as_ptr(),
                BlendMode::Replace as i64,
                BlendMode::Overlay as i64,
            ) as u32);
            ufbxi_check!(
                uc,
                !push_copy::<TextureLayer>(uc.tmp_stack_mut_ptr(), 1, &layer).is_null(),
                "((ufbx_texture_layer*)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_texture_layer), (1), (&layer)))"
            );
            num_layers += 1;
        }
        conn = conn.add(1);
    }

    (*list).data =
        push_pop::<TextureLayer>(uc.result_mut_ptr(), uc.tmp_stack_mut_ptr(), num_layers);
    (*list).count = num_layers;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    Ok(())
}

// ufbx.c:19264-19269 `ufbxi_prop_connection_less`
#[inline(always)]
pub(crate) unsafe fn prop_connection_less(a: *const Connection, prop: *const u8) -> bool {
    let cmp: i32 = strcmp((*a).dst_prop.data, prop);
    if cmp != 0 {
        return cmp < 0;
    }
    (*a).src_prop.length == 0
}

// ufbx.c:19271-19283 `ufbxi_find_prop_connection`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn find_prop_connection(
    element: *const Element,
    prop: *const u8,
) -> *mut Connection {
    let mut prop: *const u8 = prop;
    if prop.is_null() {
        prop = EMPTY_CHAR.as_ptr();
    }

    let mut index: usize = usize::MAX;

    macro_lower_bound_eq(
        32,
        &mut index,
        (*element).connections_dst.data,
        0,
        (*element).connections_dst.count,
        |a| prop_connection_less(a, prop),
        |a| (*a).dst_prop.data == prop && (*a).src_prop.length > 0,
    );

    if index < usize::MAX {
        (*element).connections_dst.data.add(index) as *mut Connection
    } else {
        ptr::null_mut()
    }
}

// ufbx.c:19285-19292 `ufbxi_patch_index_pointer`
#[inline(always)]
pub(crate) unsafe fn patch_index_pointer(uc: &Context, p_index: *mut *mut u32) {
    if *p_index == SENTINEL_INDEX_ZERO.as_ptr() as *mut u32 {
        *p_index = uc.zero_indices();
    } else if *p_index == SENTINEL_INDEX_CONSECUTIVE.as_ptr() as *mut u32 {
        *p_index = uc.consecutive_indices();
    }
}

// ufbx.c:19294-19299 `ufbxi_cmp_anim_prop_less`
#[must_use]
pub(crate) unsafe fn cmp_anim_prop_less(a: *const AnimProp, b: *const AnimProp) -> bool {
    let a_element: *mut Element = ref_ptr(&(*a).element);
    let b_element: *mut Element = ref_ptr(&(*b).element);
    if a_element != b_element {
        return a_element < b_element;
    }
    if (*a)._internal_key != (*b)._internal_key {
        return (*a)._internal_key < (*b)._internal_key;
    }
    str_less((*a).prop_name, (*b).prop_name)
}

// ufbx.c:19301-19306 `ufbxi_sort_anim_props`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn sort_anim_props(
    uc: &Context,
    aprops: *mut AnimProp,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.tmp_arr_mut_ptr(),
            uc.tmp_arr_size_mut_ptr(),
            count.wrapping_mul(size_of::<AnimProp>()),
        ),
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_anim_prop)))"
    );
    macro_stable_sort::<AnimProp>(32, aprops, uc.tmp_arr() as *mut AnimProp, count, |a, b| {
        cmp_anim_prop_less(a, b)
    });
    Ok(())
}

// ufbx.c:19308-19313 `ufbxi_material_texture_less`
#[inline(never)]
pub(crate) unsafe extern "C" fn material_texture_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const MaterialTexture = va as *const MaterialTexture;
    let b: *const MaterialTexture = vb as *const MaterialTexture;
    str_less((*a).material_prop, (*b).material_prop)
}

// ufbx.c:19315-19320 `ufbxi_sort_material_textures`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn sort_material_textures(
    uc: &Context,
    textures: *mut MaterialTexture,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.tmp_arr_mut_ptr(),
            uc.tmp_arr_size_mut_ptr(),
            count.wrapping_mul(size_of::<MaterialTexture>()),
        ),
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_material_texture)))"
    );
    stable_sort(
        size_of::<MaterialTexture>(),
        32,
        textures as *mut c_void,
        uc.tmp_arr() as *mut c_void,
        count,
        material_texture_less,
        ptr::null_mut(),
    );
    Ok(())
}

// ufbx.c:19322-19327 `ufbxi_bone_pose_less`
#[inline(never)]
pub(crate) unsafe extern "C" fn bone_pose_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const BonePose = va as *const BonePose;
    let b: *const BonePose = vb as *const BonePose;
    (*ref_ptr(&(*a).bone_node)).element.typed_id < (*ref_ptr(&(*b).bone_node)).element.typed_id
}

// ufbx.c:19329-19335 `ufbxi_find_anim_prop_start`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn find_anim_prop_start(
    layer: *mut AnimLayer,
    element: *const Element,
) -> *mut AnimProp {
    let mut index: usize = usize::MAX;
    macro_lower_bound_eq(
        16,
        &mut index,
        (*layer).anim_props.data,
        0,
        (*layer).anim_props.count,
        |a| (ref_ptr(&(*a).element) as *const Element) < element,
        |a| (ref_ptr(&(*a).element) as *const Element) == element,
    );
    if index != usize::MAX {
        (*layer).anim_props.data.add(index) as *mut AnimProp
    } else {
        ptr::null_mut()
    }
}

// ufbx.c:19337-19343 `ufbxi_sort_bone_poses`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn sort_bone_poses(uc: &Context, pose: *mut Pose) -> Result<(), Fail> {
    let count: usize = (*pose).bone_poses.count;
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.tmp_arr_mut_ptr(),
            uc.tmp_arr_size_mut_ptr(),
            (*pose).bone_poses.count.wrapping_mul(size_of::<BonePose>()),
        ),
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (pose->bone_poses.count * sizeof(ufbx_bone_pose)))"
    );
    stable_sort(
        size_of::<BonePose>(),
        16,
        (*pose).bone_poses.data as *mut c_void,
        uc.tmp_arr() as *mut c_void,
        count,
        bone_pose_less,
        ptr::null_mut(),
    );
    Ok(())
}

// ufbx.c:19345-19356 `ufbxi_sort_skin_weights`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn sort_skin_weights(uc: &Context, skin: *mut SkinDeformer) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.tmp_arr_mut_ptr(),
            uc.tmp_arr_size_mut_ptr(),
            (*skin)
                .max_weights_per_vertex
                .wrapping_mul(size_of::<SkinWeight>()),
        ),
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (skin->max_weights_per_vertex * sizeof(ufbx_skin_weight)))"
    );

    for i in 0..(*skin).vertices.count {
        let v: SkinVertex = *(*skin).vertices.data.add(i);
        macro_stable_sort::<SkinWeight>(
            32,
            ((*skin).weights.data as *mut SkinWeight).add(v.weight_begin as usize),
            uc.tmp_arr() as *mut SkinWeight,
            v.num_weights as usize,
            |a, b| (*a).weight > (*b).weight,
        );
    }

    Ok(())
}

// ufbx.c:19358-19363 `ufbxi_blend_keyframe_less`
#[inline(never)]
pub(crate) unsafe extern "C" fn blend_keyframe_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const BlendKeyframe = va as *const BlendKeyframe;
    let b: *const BlendKeyframe = vb as *const BlendKeyframe;
    (*a).target_weight < (*b).target_weight
}

// ufbx.c:19365-19370 `ufbxi_sort_blend_keyframes`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn sort_blend_keyframes(
    uc: &Context,
    keyframes: *mut BlendKeyframe,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.tmp_arr_mut_ptr(),
            uc.tmp_arr_size_mut_ptr(),
            count.wrapping_mul(size_of::<BlendKeyframe>()),
        ),
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_blend_keyframe)))"
    );
    stable_sort(
        size_of::<BlendKeyframe>(),
        32,
        keyframes as *mut c_void,
        uc.tmp_arr() as *mut c_void,
        count,
        blend_keyframe_less,
        ptr::null_mut(),
    );
    Ok(())
}

// Material tables
// (ufbx.c:19372)

// ufbx.c:19374 `typedef void (*ufbxi_mat_transform_fn)(ufbx_vec4 *a);`
pub(crate) type MatTransformFn = unsafe extern "C" fn(a: *mut Vec4);

// ufbx.c:19376 `ufbxi_mat_transform_invert_x`
pub(crate) unsafe extern "C" fn mat_transform_invert_x(v: *mut Vec4) {
    (*v).x = 1.0 - (*v).x;
}
// ufbx.c:19377 `ufbxi_mat_transform_unknown_shininess`
// C-parity: `ufbx_sqrt` takes/returns `double`, so the product and the
// subtraction are evaluated in `double` regardless of `ufbx_real`'s width; the
// `(ufbx_real)0.1` cast happens before the widening back to `double`.
pub(crate) unsafe extern "C" fn mat_transform_unknown_shininess(v: *mut Vec4) {
    if (*v).x >= 0.0 {
        (*v).x = (1.0f64 - math::sqrt((*v).x as f64) * (0.1f64 as Real) as f64) as Real;
    }
    if !((*v).x >= 0.0) {
        (*v).x = 0.0;
    }
}
// ufbx.c:19378 `ufbxi_mat_transform_blender_opacity`
pub(crate) unsafe extern "C" fn mat_transform_blender_opacity(v: *mut Vec4) {
    (*v).x = 1.0 - (*v).x;
}
// ufbx.c:19379 `ufbxi_mat_transform_blender_shininess`
pub(crate) unsafe extern "C" fn mat_transform_blender_shininess(v: *mut Vec4) {
    if (*v).x >= 0.0 {
        (*v).x = (1.0f64 - math::sqrt((*v).x as f64) * (0.1f64 as Real) as f64) as Real;
    }
    if !((*v).x >= 0.0) {
        (*v).x = 0.0;
    }
}

// ufbx.c:19381-19389 `ufbxi_mat_transform`
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MatTransform {
    // C-parity: `UFBXI_MAT_TRANSFORM_IDENTITY` (ufbx.c:19382) is never named in
    // ufbx.c either — the mapping tables spell the identity transform as the
    // literal `0` and ufbx.c:20052 tests `if (mapping->transform)`. C does not
    // warn on an unreferenced enumerator.
    #[allow(dead_code)]
    Identity,
    InvertX,
    UnknownShininess,
    BlenderOpacity,
    BlenderShininess,

    Count,
}

// ufbx.c:19391-19398 `ufbxi_shader_mapping_flag`
// C: `typedef enum { ... } ufbxi_shader_mapping_flag;` — plain bit-flag
// constants, held in the `uint8_t` field `ufbxi_shader_mapping.flags`.
// Set `value_vec4.w` (usually alpha) to 1.0 if not defined by the property
pub(crate) const SHADER_MAPPING_DEFAULT_W_1: u8 = 0x1;
// Widen values to RGB if only a single value is present.
pub(crate) const SHADER_MAPPING_WIDEN_TO_RGB: u8 = 0x2;
// Multiply the existing value.
pub(crate) const SHADER_MAPPING_MULTIPLY_VALUE: u8 = 0x4;

// ufbx.c:19400-19411 `ufbxi_shader_feature_flag`
// Invert the feature flag
pub(crate) const SHADER_FEATURE_INVERTED: u8 = 0x1;
// Enable the feature if the given property exists
pub(crate) const SHADER_FEATURE_IF_EXISTS: u8 = 0x2;
// Enable the feature if the given property has a texture
pub(crate) const SHADER_FEATURE_IF_TEXTURE: u8 = 0x4;
// Enable if the feature is in [0.5, 1.5], (ie. 2 won't enable this feature)
pub(crate) const SHADER_FEATURE_IF_AROUND_1: u8 = 0x8;

pub(crate) const SHADER_FEATURE_IF_EXISTS_OR_TEXTURE: u8 =
    SHADER_FEATURE_IF_EXISTS | SHADER_FEATURE_IF_TEXTURE;

// ufbx.c:19413-19419 `ufbxi_mat_transform_fns`
// C: the `NULL` entry for `UFBXI_MAT_TRANSFORM_IDENTITY` → `Option<fn>`.
pub(crate) static MAT_TRANSFORM_FNS: [Option<MatTransformFn>; 5] = [
    None,
    Some(mat_transform_invert_x),
    Some(mat_transform_unknown_shininess),
    Some(mat_transform_blender_opacity),
    Some(mat_transform_blender_shininess),
];

// ufbx.c:19421 `ufbx_static_assert(transform_count, ufbxi_arraycount(ufbxi_mat_transform_fns) == UFBXI_MAT_TRANSFORM_COUNT);`
const _: () = assert!(MAT_TRANSFORM_FNS.len() == MatTransform::Count as usize);

// ufbx.c:19423-19429 `ufbxi_shader_mapping`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ShaderMapping {
    pub index: u8,       // < `ufbx_material_(fbx|pbr)_map`
    pub flags: u8,       // < Combination of `ufbxi_shader_mapping_flag`
    pub transform: u8,   // < `ufbxi_mat_transform`
    pub prop_len: u8,    // < Length of `prop` not including NULL terminator
    pub prop: *const u8, // < Name of FBX material property or shader mapping
}
// The mapping tables below are immutable and their `const char *` member
// references immutable string literals, so sharing is sound (same rationale as
// `LegacyProp` in `native::read`).
unsafe impl Sync for ShaderMapping {}

// ufbx.c:19431-19441 `ufbxi_shader_mapping_list`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ShaderMappingList {
    pub data: *const ShaderMapping,
    pub count: usize,
    pub features: *const ShaderMapping,
    pub feature_count: usize,
    pub default_features: u32,
    pub texture_prefix: String,
    pub texture_suffix: String,
    pub texture_enabled_prefix: String,
    pub texture_enabled_suffix: String,
}
// `ufbxi_shader_pbr_mappings` is a static table of these; the `ufbx_string`
// and `ufbxi_shader_mapping` pointers it holds all reference immutable
// statics.
unsafe impl Sync for ShaderMappingList {}

// ufbx.h:2388 `UFBX_ENUM_TYPE(ufbx_shader_type, UFBX_SHADER_TYPE, UFBX_SHADER_WAVEFRONT_MTL);`
// expanding via ufbx.h:235-236 to `enum { UFBX_SHADER_TYPE_COUNT = UFBX_SHADER_WAVEFRONT_MTL + 1 }`.
// Hand-duplicated from the generated enum's last variant so an upstream enum
// change tracks automatically through regen (precedent: `ELEMENT_TYPE_COUNT`
// in `native::parse`).
pub(crate) const SHADER_TYPE_COUNT: usize = ShaderType::WavefrontMtl as usize + 1;

// ufbx.c:19443 `#define ufbxi_mat_string(str) sizeof(str) - 1, str`
// C: expands to the trailing `prop_len, prop` initializer PAIR of an
// `ufbxi_shader_mapping`. A Rust macro cannot expand to two struct fields, so
// the pair is folded into the whole-entry macro below; the `mat_string(...)`
// spelling is kept as literal macro syntax to preserve the C line shape. The
// argument is the C string literal with its NUL made explicit, so
// `$prop.len() - 1` is `sizeof(str) - 1` and `$prop.as_ptr()` is `str`.
macro_rules! mat_mapping {
    ($index:expr, $flags:expr, $transform:expr, mat_string($prop:literal)) => {
        ShaderMapping {
            index: $index as u8,
            flags: $flags,
            transform: $transform as u8,
            prop_len: ($prop.len() - 1) as u8,
            prop: $prop.as_ptr(),
        }
    };
}

// The `#[rustfmt::skip]` on every table below keeps one Rust line per C
// initializer line; without it rustfmt wraps the over-long `mat_mapping!`
// invocations and the C↔Rust line correspondence is lost.

// ufbx.c:19445-19474 `ufbxi_base_fbx_mapping`
#[rustfmt::skip]
static BASE_FBX_MAPPING: [ShaderMapping; 28] = [
    mat_mapping!(MaterialFbxMap::DiffuseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Diffuse\0")),
    mat_mapping!(MaterialFbxMap::DiffuseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"DiffuseColor\0")),
    mat_mapping!(MaterialFbxMap::DiffuseFactor, 0, 0, mat_string(b"DiffuseFactor\0")),
    mat_mapping!(MaterialFbxMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Specular\0")),
    mat_mapping!(MaterialFbxMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"SpecularColor\0")),
    mat_mapping!(MaterialFbxMap::SpecularFactor, 0, 0, mat_string(b"SpecularFactor\0")),
    mat_mapping!(MaterialFbxMap::SpecularExponent, 0, 0, mat_string(b"Shininess\0")),
    mat_mapping!(MaterialFbxMap::SpecularExponent, 0, 0, mat_string(b"ShininessExponent\0")),
    mat_mapping!(MaterialFbxMap::ReflectionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Reflection\0")),
    mat_mapping!(MaterialFbxMap::ReflectionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"ReflectionColor\0")),
    mat_mapping!(MaterialFbxMap::ReflectionFactor, 0, 0, mat_string(b"ReflectionFactor\0")),
    mat_mapping!(MaterialFbxMap::TransparencyColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Transparent\0")),
    mat_mapping!(MaterialFbxMap::TransparencyColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"TransparentColor\0")),
    mat_mapping!(MaterialFbxMap::TransparencyFactor, 0, 0, mat_string(b"TransparentFactor\0")),
    mat_mapping!(MaterialFbxMap::TransparencyFactor, 0, 0, mat_string(b"TransparencyFactor\0")),
    mat_mapping!(MaterialFbxMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Emissive\0")),
    mat_mapping!(MaterialFbxMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"EmissiveColor\0")),
    mat_mapping!(MaterialFbxMap::EmissionFactor, 0, 0, mat_string(b"EmissiveFactor\0")),
    mat_mapping!(MaterialFbxMap::AmbientColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Ambient\0")),
    mat_mapping!(MaterialFbxMap::AmbientColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"AmbientColor\0")),
    mat_mapping!(MaterialFbxMap::AmbientFactor, 0, 0, mat_string(b"AmbientFactor\0")),
    mat_mapping!(MaterialFbxMap::NormalMap, 0, 0, mat_string(b"NormalMap\0")),
    mat_mapping!(MaterialFbxMap::Bump, 0, 0, mat_string(b"Bump\0")),
    mat_mapping!(MaterialFbxMap::BumpFactor, 0, 0, mat_string(b"BumpFactor\0")),
    mat_mapping!(MaterialFbxMap::Displacement, 0, 0, mat_string(b"Displacement\0")),
    mat_mapping!(MaterialFbxMap::DisplacementFactor, 0, 0, mat_string(b"DisplacementFactor\0")),
    mat_mapping!(MaterialFbxMap::VectorDisplacement, 0, 0, mat_string(b"VectorDisplacement\0")),
    mat_mapping!(MaterialFbxMap::VectorDisplacementFactor, 0, 0, mat_string(b"VectorDisplacementFactor\0")),
];

// ufbx.c:19476-19487 `ufbxi_obj_fbx_mapping`
#[rustfmt::skip]
static OBJ_FBX_MAPPING: [ShaderMapping; 10] = [
    mat_mapping!(MaterialFbxMap::AmbientColor, SHADER_MAPPING_DEFAULT_W_1 | SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"Ka\0")),
    mat_mapping!(MaterialFbxMap::DiffuseColor, SHADER_MAPPING_DEFAULT_W_1 | SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"Kd\0")),
    mat_mapping!(MaterialFbxMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1 | SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"Ks\0")),
    mat_mapping!(MaterialFbxMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1 | SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"Ke\0")),
    mat_mapping!(MaterialFbxMap::SpecularExponent, 0, 0, mat_string(b"Ns\0")),
    mat_mapping!(MaterialFbxMap::TransparencyFactor, 0, MatTransform::InvertX, mat_string(b"d\0")),
    mat_mapping!(MaterialFbxMap::NormalMap, 0, 0, mat_string(b"norm\0")),
    mat_mapping!(MaterialFbxMap::Displacement, 0, 0, mat_string(b"disp\0")),
    mat_mapping!(MaterialFbxMap::Bump, 0, 0, mat_string(b"bump\0")),
    mat_mapping!(MaterialFbxMap::Bump, 0, 0, mat_string(b"Bump\0")),
];

// ufbx.c:19489-19501 `ufbxi_fbx_lambert_shader_pbr_mapping`
#[rustfmt::skip]
static FBX_LAMBERT_SHADER_PBR_MAPPING: [ShaderMapping; 11] = [
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Diffuse\0")),
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"DiffuseColor\0")),
    mat_mapping!(MaterialPbrMap::BaseFactor, 0, 0, mat_string(b"DiffuseFactor\0")),
    mat_mapping!(MaterialPbrMap::TransmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Transparent\0")),
    mat_mapping!(MaterialPbrMap::TransmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"TransparentColor\0")),
    mat_mapping!(MaterialPbrMap::TransmissionFactor, 0, 0, mat_string(b"TransparentFactor\0")),
    mat_mapping!(MaterialPbrMap::TransmissionFactor, 0, 0, mat_string(b"TransparencyFactor\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Emissive\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"EmissiveColor\0")),
    mat_mapping!(MaterialPbrMap::EmissionFactor, 0, 0, mat_string(b"EmissiveFactor\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"NormalMap\0")),
];

// ufbx.c:19503-19520 `ufbxi_fbx_phong_shader_pbr_mapping`
#[rustfmt::skip]
static FBX_PHONG_SHADER_PBR_MAPPING: [ShaderMapping; 16] = [
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Diffuse\0")),
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"DiffuseColor\0")),
    mat_mapping!(MaterialPbrMap::BaseFactor, 0, 0, mat_string(b"DiffuseFactor\0")),
    mat_mapping!(MaterialPbrMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Specular\0")),
    mat_mapping!(MaterialPbrMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"SpecularColor\0")),
    mat_mapping!(MaterialPbrMap::SpecularFactor, 0, 0, mat_string(b"SpecularFactor\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, MatTransform::UnknownShininess, mat_string(b"Shininess\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, MatTransform::UnknownShininess, mat_string(b"ShininessExponent\0")),
    mat_mapping!(MaterialPbrMap::TransmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Transparent\0")),
    mat_mapping!(MaterialPbrMap::TransmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"TransparentColor\0")),
    mat_mapping!(MaterialPbrMap::TransmissionFactor, 0, 0, mat_string(b"TransparentFactor\0")),
    mat_mapping!(MaterialPbrMap::TransmissionFactor, 0, 0, mat_string(b"TransparencyFactor\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Emissive\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"EmissiveColor\0")),
    mat_mapping!(MaterialPbrMap::EmissionFactor, 0, 0, mat_string(b"EmissiveFactor\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"NormalMap\0")),
];

// ufbx.c:19522-19565 `ufbxi_osl_standard_shader_pbr_mapping`
#[rustfmt::skip]
static OSL_STANDARD_SHADER_PBR_MAPPING: [ShaderMapping; 42] = [
    mat_mapping!(MaterialPbrMap::BaseFactor, 0, 0, mat_string(b"base\0")),
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"base_color\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, 0, mat_string(b"specular_roughness\0")),
    mat_mapping!(MaterialPbrMap::DiffuseRoughness, 0, 0, mat_string(b"diffuse_roughness\0")),
    mat_mapping!(MaterialPbrMap::Metalness, 0, 0, mat_string(b"metalness\0")),
    mat_mapping!(MaterialPbrMap::SpecularFactor, 0, 0, mat_string(b"specular\0")),
    mat_mapping!(MaterialPbrMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"specular_color\0")),
    mat_mapping!(MaterialPbrMap::SpecularIor, 0, 0, mat_string(b"specular_IOR\0")),
    mat_mapping!(MaterialPbrMap::SpecularAnisotropy, 0, 0, mat_string(b"specular_anisotropy\0")),
    mat_mapping!(MaterialPbrMap::SpecularRotation, 0, 0, mat_string(b"specular_rotation\0")),
    mat_mapping!(MaterialPbrMap::TransmissionFactor, 0, 0, mat_string(b"transmission\0")),
    mat_mapping!(MaterialPbrMap::TransmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"transmission_color\0")),
    mat_mapping!(MaterialPbrMap::TransmissionDepth, 0, 0, mat_string(b"transmission_depth\0")),
    mat_mapping!(MaterialPbrMap::TransmissionScatter, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"transmission_scatter\0")),
    mat_mapping!(MaterialPbrMap::TransmissionScatterAnisotropy, 0, 0, mat_string(b"transmission_scatter_anisotropy\0")),
    mat_mapping!(MaterialPbrMap::TransmissionDispersion, 0, 0, mat_string(b"transmission_dispersion\0")),
    mat_mapping!(MaterialPbrMap::TransmissionExtraRoughness, 0, 0, mat_string(b"transmission_extra_roughness\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceFactor, 0, 0, mat_string(b"subsurface\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"subsurface_color\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceRadius, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"subsurface_radius\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceScale, 0, 0, mat_string(b"subsurface_scale\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceAnisotropy, 0, 0, mat_string(b"subsurface_anisotropy\0")),
    mat_mapping!(MaterialPbrMap::SheenFactor, 0, 0, mat_string(b"sheen\0")),
    mat_mapping!(MaterialPbrMap::SheenColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"sheen_color\0")),
    mat_mapping!(MaterialPbrMap::SheenRoughness, 0, 0, mat_string(b"sheen_roughness\0")),
    mat_mapping!(MaterialPbrMap::CoatFactor, 0, 0, mat_string(b"coat\0")),
    mat_mapping!(MaterialPbrMap::CoatColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"coat_color\0")),
    mat_mapping!(MaterialPbrMap::CoatRoughness, 0, 0, mat_string(b"coat_roughness\0")),
    mat_mapping!(MaterialPbrMap::CoatIor, 0, 0, mat_string(b"coat_IOR\0")),
    mat_mapping!(MaterialPbrMap::CoatAnisotropy, 0, 0, mat_string(b"coat_anisotropy\0")),
    mat_mapping!(MaterialPbrMap::CoatRotation, 0, 0, mat_string(b"coat_rotation\0")),
    mat_mapping!(MaterialPbrMap::CoatNormal, 0, 0, mat_string(b"coat_normal\0")),
    mat_mapping!(MaterialPbrMap::CoatAffectBaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"coat_affect_color\0")),
    mat_mapping!(MaterialPbrMap::CoatAffectBaseRoughness, 0, 0, mat_string(b"coat_affect_roughness\0")),
    mat_mapping!(MaterialPbrMap::ThinFilmThickness, 0, 0, mat_string(b"thin_film_thickness\0")),
    mat_mapping!(MaterialPbrMap::ThinFilmIor, 0, 0, mat_string(b"thin_film_IOR\0")),
    mat_mapping!(MaterialPbrMap::EmissionFactor, 0, 0, mat_string(b"emission\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"emission_color\0")),
    mat_mapping!(MaterialPbrMap::Opacity, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"opacity\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"NormalMap\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"normalCamera\0")),
    mat_mapping!(MaterialPbrMap::TangentMap, 0, 0, mat_string(b"tangent\0")),
];

// ufbx.c:19567-19569 `ufbxi_osl_standard_shader_features`
#[rustfmt::skip]
static OSL_STANDARD_SHADER_FEATURES: [ShaderMapping; 1] = [
    mat_mapping!(MaterialFeature::ThinWalled, 0, 0, mat_string(b"thin_walled\0")),
];

// ufbx.c:19571-19619 `ufbxi_arnold_shader_pbr_mapping`
#[rustfmt::skip]
static ARNOLD_SHADER_PBR_MAPPING: [ShaderMapping; 47] = [
    mat_mapping!(MaterialPbrMap::BaseFactor, 0, 0, mat_string(b"base\0")),
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"baseColor\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, 0, mat_string(b"specularRoughness\0")),
    mat_mapping!(MaterialPbrMap::DiffuseRoughness, 0, 0, mat_string(b"diffuseRoughness\0")),
    mat_mapping!(MaterialPbrMap::Metalness, 0, 0, mat_string(b"metalness\0")),
    mat_mapping!(MaterialPbrMap::SpecularFactor, 0, 0, mat_string(b"specular\0")),
    mat_mapping!(MaterialPbrMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"specularColor\0")),
    mat_mapping!(MaterialPbrMap::SpecularIor, 0, 0, mat_string(b"specularIOR\0")),
    mat_mapping!(MaterialPbrMap::SpecularAnisotropy, 0, 0, mat_string(b"specularAnisotropy\0")),
    mat_mapping!(MaterialPbrMap::SpecularRotation, 0, 0, mat_string(b"specularRotation\0")),
    mat_mapping!(MaterialPbrMap::TransmissionFactor, 0, 0, mat_string(b"transmission\0")),
    mat_mapping!(MaterialPbrMap::TransmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"transmissionColor\0")),
    mat_mapping!(MaterialPbrMap::TransmissionDepth, 0, 0, mat_string(b"transmissionDepth\0")),
    mat_mapping!(MaterialPbrMap::TransmissionScatter, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"transmissionScatter\0")),
    mat_mapping!(MaterialPbrMap::TransmissionScatterAnisotropy, 0, 0, mat_string(b"transmissionScatterAnisotropy\0")),
    mat_mapping!(MaterialPbrMap::TransmissionDispersion, 0, 0, mat_string(b"transmissionDispersion\0")),
    mat_mapping!(MaterialPbrMap::TransmissionExtraRoughness, 0, 0, mat_string(b"transmissionExtraRoughness\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceFactor, 0, 0, mat_string(b"subsurface\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"subsurfaceColor\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceRadius, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"subsurfaceRadius\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceScale, 0, 0, mat_string(b"subsurfaceScale\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceAnisotropy, 0, 0, mat_string(b"subsurfaceAnisotropy\0")),
    mat_mapping!(MaterialPbrMap::SheenFactor, 0, 0, mat_string(b"sheen\0")),
    mat_mapping!(MaterialPbrMap::SheenColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"sheenColor\0")),
    mat_mapping!(MaterialPbrMap::SheenRoughness, 0, 0, mat_string(b"sheenRoughness\0")),
    mat_mapping!(MaterialPbrMap::CoatFactor, 0, 0, mat_string(b"coat\0")),
    mat_mapping!(MaterialPbrMap::CoatColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"coatColor\0")),
    mat_mapping!(MaterialPbrMap::CoatRoughness, 0, 0, mat_string(b"coatRoughness\0")),
    mat_mapping!(MaterialPbrMap::CoatIor, 0, 0, mat_string(b"coatIOR\0")),
    mat_mapping!(MaterialPbrMap::CoatAnisotropy, 0, 0, mat_string(b"coatAnisotropy\0")),
    mat_mapping!(MaterialPbrMap::CoatRotation, 0, 0, mat_string(b"coatRotation\0")),
    mat_mapping!(MaterialPbrMap::CoatNormal, 0, 0, mat_string(b"coatNormal\0")),
    mat_mapping!(MaterialPbrMap::ThinFilmThickness, 0, 0, mat_string(b"thinFilmThickness\0")),
    mat_mapping!(MaterialPbrMap::ThinFilmIor, 0, 0, mat_string(b"thinFilmIOR\0")),
    mat_mapping!(MaterialPbrMap::EmissionFactor, 0, 0, mat_string(b"emission\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"emissionColor\0")),
    mat_mapping!(MaterialPbrMap::Opacity, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"opacity\0")),
    mat_mapping!(MaterialPbrMap::IndirectDiffuse, 0, 0, mat_string(b"indirectDiffuse\0")),
    mat_mapping!(MaterialPbrMap::IndirectSpecular, 0, 0, mat_string(b"indirectSpecular\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"NormalMap\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"normalCamera\0")),
    mat_mapping!(MaterialPbrMap::TangentMap, 0, 0, mat_string(b"tangent\0")),
    mat_mapping!(MaterialPbrMap::MatteColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"aiMatteColor\0")),
    mat_mapping!(MaterialPbrMap::MatteFactor, 0, 0, mat_string(b"aiMatteColorA\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceType, 0, 0, mat_string(b"subsurfaceType\0")),
    mat_mapping!(MaterialPbrMap::TransmissionPriority, 0, 0, mat_string(b"dielectricPriority\0")),
    mat_mapping!(MaterialPbrMap::TransmissionEnableInAov, 0, 0, mat_string(b"transmitAovs\0")),
];

// ufbx.c:19621-19627 `ufbxi_arnold_shader_features`
#[rustfmt::skip]
static ARNOLD_SHADER_FEATURES: [ShaderMapping; 5] = [
    mat_mapping!(MaterialFeature::Matte, 0, 0, mat_string(b"aiEnableMatte\0")),
    mat_mapping!(MaterialFeature::ThinWalled, 0, 0, mat_string(b"thinWalled\0")),
    mat_mapping!(MaterialFeature::Caustics, 0, 0, mat_string(b"caustics\0")),
    mat_mapping!(MaterialFeature::InternalReflections, 0, 0, mat_string(b"internalReflections\0")),
    mat_mapping!(MaterialFeature::ExitToBackground, 0, 0, mat_string(b"exitToBackground\0")),
];

// ufbx.c:19629-19670 `ufbxi_3ds_max_physical_material_pbr_mapping`
#[rustfmt::skip]
static E3DS_MAX_PHYSICAL_MATERIAL_PBR_MAPPING: [ShaderMapping; 40] = [
    mat_mapping!(MaterialPbrMap::BaseFactor, 0, 0, mat_string(b"base_weight\0")),
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"base_color\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, 0, mat_string(b"roughness\0")),
    mat_mapping!(MaterialPbrMap::DiffuseRoughness, 0, 0, mat_string(b"diff_rough\0")),
    mat_mapping!(MaterialPbrMap::DiffuseRoughness, 0, 0, mat_string(b"diff_roughness\0")),
    mat_mapping!(MaterialPbrMap::Metalness, 0, 0, mat_string(b"metalness\0")),
    mat_mapping!(MaterialPbrMap::SpecularFactor, 0, 0, mat_string(b"reflectivity\0")),
    mat_mapping!(MaterialPbrMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"refl_color\0")),
    mat_mapping!(MaterialPbrMap::SpecularAnisotropy, 0, 0, mat_string(b"anisotropy\0")),
    mat_mapping!(MaterialPbrMap::SpecularRotation, 0, 0, mat_string(b"aniso_angle\0")),
    mat_mapping!(MaterialPbrMap::SpecularRotation, 0, 0, mat_string(b"anisoangle\0")),
    mat_mapping!(MaterialPbrMap::SpecularIor, 0, 0, mat_string(b"trans_ior\0")), // NOTE: Not a typo, IOR is same for transparency/specular
    mat_mapping!(MaterialPbrMap::TransmissionFactor, 0, 0, mat_string(b"transparency\0")),
    mat_mapping!(MaterialPbrMap::TransmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"trans_color\0")),
    mat_mapping!(MaterialPbrMap::TransmissionDepth, 0, 0, mat_string(b"trans_depth\0")),
    mat_mapping!(MaterialPbrMap::TransmissionRoughness, 0, 0, mat_string(b"trans_rough\0")),
    mat_mapping!(MaterialPbrMap::TransmissionRoughness, 0, 0, mat_string(b"trans_roughness\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceFactor, 0, 0, mat_string(b"scattering\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceTintColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"sss_color\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"sss_scatter_color\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceRadius, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"sss_depth\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceScale, 0, 0, mat_string(b"sss_scale\0")),
    mat_mapping!(MaterialPbrMap::CoatFactor, 0, 0, mat_string(b"coat\0")),
    mat_mapping!(MaterialPbrMap::CoatFactor, 0, 0, mat_string(b"coating\0")),
    mat_mapping!(MaterialPbrMap::CoatColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"coat_color\0")),
    mat_mapping!(MaterialPbrMap::CoatRoughness, 0, 0, mat_string(b"coat_rough\0")),
    mat_mapping!(MaterialPbrMap::CoatRoughness, 0, 0, mat_string(b"coat_roughness\0")),
    mat_mapping!(MaterialPbrMap::CoatIor, 0, 0, mat_string(b"coat_ior\0")),
    mat_mapping!(MaterialPbrMap::CoatNormal, 0, 0, mat_string(b"coat_bump\0")),
    mat_mapping!(MaterialPbrMap::CoatNormal, 0, 0, mat_string(b"clearcoat_bump_map_amt\0")),
    mat_mapping!(MaterialPbrMap::CoatAffectBaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"coat_affect_color\0")),
    mat_mapping!(MaterialPbrMap::CoatAffectBaseRoughness, 0, 0, mat_string(b"coat_affect_roughness\0")),
    mat_mapping!(MaterialPbrMap::EmissionFactor, 0, 0, mat_string(b"emission\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"emit_color\0")),
    mat_mapping!(MaterialPbrMap::Opacity, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"cutout\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"bump\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"bump_map_amt\0")),
    mat_mapping!(MaterialPbrMap::DisplacementMap, 0, 0, mat_string(b"displacement\0")),
    mat_mapping!(MaterialPbrMap::DisplacementMap, 0, 0, mat_string(b"displacement_map_amt\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceType, 0, 0, mat_string(b"subsurfaceType\0")),
];

// ufbx.c:19672-19680 `ufbxi_3ds_max_physical_material_features`
#[rustfmt::skip]
static E3DS_MAX_PHYSICAL_MATERIAL_FEATURES: [ShaderMapping; 7] = [
    mat_mapping!(MaterialFeature::ThinWalled, 0, 0, mat_string(b"thin_walled\0")),
    mat_mapping!(MaterialFeature::Specular, 0, 0, mat_string(b"material_mode\0")),
    mat_mapping!(MaterialFeature::DiffuseRoughness, 0, 0, mat_string(b"material_mode\0")),
    mat_mapping!(MaterialFeature::TransmissionRoughness, SHADER_FEATURE_INVERTED, 0, mat_string(b"trans_roughness_lock\0")),
    mat_mapping!(MaterialFeature::RoughnessAsGlossiness, 0, 0, mat_string(b"roughness_inv\0")),
    mat_mapping!(MaterialFeature::TransmissionRoughnessAsGlossiness, 0, 0, mat_string(b"trans_roughness_inv\0")),
    mat_mapping!(MaterialFeature::CoatRoughnessAsGlossiness, 0, 0, mat_string(b"coat_roughness_inv\0")),
];

// ufbx.c:19682-19702 `ufbxi_gltf_material_pbr_mapping`
#[rustfmt::skip]
static GLTF_MATERIAL_PBR_MAPPING: [ShaderMapping; 19] = [
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"main|baseColor\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, 0, mat_string(b"main|roughness\0")),
    mat_mapping!(MaterialPbrMap::Metalness, 0, 0, mat_string(b"main|metalness\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"main|normal\0")),
    mat_mapping!(MaterialPbrMap::AmbientOcclusion, 0, 0, mat_string(b"main|ambientOcclusion\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"main|emission\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"main|emissionColor\0")),
    mat_mapping!(MaterialPbrMap::Opacity, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"main|Alpha\0")),
    mat_mapping!(MaterialPbrMap::CoatFactor, 0, 0, mat_string(b"extension|clearcoat\0")),
    mat_mapping!(MaterialPbrMap::CoatRoughness, 0, 0, mat_string(b"extension|clearcoatRoughness\0")),
    mat_mapping!(MaterialPbrMap::CoatNormal, 0, 0, mat_string(b"extension|clearcoatNormal\0")),
    mat_mapping!(MaterialPbrMap::SheenColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"extension|sheenColor\0")),
    mat_mapping!(MaterialPbrMap::SheenRoughness, 0, 0, mat_string(b"extension|sheenRoughness\0")),
    mat_mapping!(MaterialPbrMap::SpecularFactor, 0, 0, mat_string(b"extension|specular\0")),
    mat_mapping!(MaterialPbrMap::SpecularFactor, 0, 0, mat_string(b"extension|Specular\0")),
    mat_mapping!(MaterialPbrMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"extension|specularcolor\0")),
    mat_mapping!(MaterialPbrMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"extension|specularColor\0")),
    mat_mapping!(MaterialPbrMap::TransmissionFactor, 0, 0, mat_string(b"extension|transmission\0")),
    mat_mapping!(MaterialPbrMap::SpecularIor, 0, 0, mat_string(b"extension|indexOfRefraction\0")),
];

// ufbx.c:19704-19748 `ufbxi_openpbr_material_pbr_mapping`
#[rustfmt::skip]
static OPENPBR_MATERIAL_PBR_MAPPING: [ShaderMapping; 43] = [
    mat_mapping!(MaterialPbrMap::BaseFactor, 0, 0, mat_string(b"base_weight\0")),
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"base_color\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, 0, mat_string(b"specular_roughness\0")),
    mat_mapping!(MaterialPbrMap::DiffuseRoughness, 0, 0, mat_string(b"base_diffuse_roughness\0")),
    mat_mapping!(MaterialPbrMap::Metalness, 0, 0, mat_string(b"base_metalness\0")),
    mat_mapping!(MaterialPbrMap::SpecularFactor, 0, 0, mat_string(b"specular_weight\0")),
    mat_mapping!(MaterialPbrMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"specular_color\0")),
    mat_mapping!(MaterialPbrMap::SpecularAnisotropy, 0, 0, mat_string(b"specular_roughness_anisotropy\0")),
    mat_mapping!(MaterialPbrMap::SpecularIor, 0, 0, mat_string(b"specular_ior\0")),
    mat_mapping!(MaterialPbrMap::TransmissionFactor, 0, 0, mat_string(b"transmission_weight\0")),
    mat_mapping!(MaterialPbrMap::TransmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"transmission_color\0")),
    mat_mapping!(MaterialPbrMap::TransmissionDepth, 0, 0, mat_string(b"transmission_depth\0")),
    mat_mapping!(MaterialPbrMap::TransmissionScatter, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"transmission_scatter\0")),
    mat_mapping!(MaterialPbrMap::TransmissionScatterAnisotropy, 0, 0, mat_string(b"transmission_scatter_anisotropy\0")),
    mat_mapping!(MaterialPbrMap::TransmissionDispersion, 0, 0, mat_string(b"transmission_dispersion_scale\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceFactor, 0, 0, mat_string(b"subsurface_weight\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"subsurface_color\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceRadius, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"subsurface_radius_scale\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceScale, 0, 0, mat_string(b"subsurface_radius\0")),
    mat_mapping!(MaterialPbrMap::SubsurfaceAnisotropy, 0, 0, mat_string(b"subsurface_scatter_anisotropy\0")),
    mat_mapping!(MaterialPbrMap::CoatFactor, 0, 0, mat_string(b"coat_weight\0")),
    mat_mapping!(MaterialPbrMap::CoatColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"coat_color\0")),
    mat_mapping!(MaterialPbrMap::CoatRoughness, 0, 0, mat_string(b"coat_roughness\0")),
    mat_mapping!(MaterialPbrMap::CoatAnisotropy, 0, 0, mat_string(b"coat_roughness_anisotropy\0")),
    mat_mapping!(MaterialPbrMap::CoatIor, 0, 0, mat_string(b"coat_ior\0")),
    mat_mapping!(MaterialPbrMap::CoatNormal, 0, 0, mat_string(b"coat_normal_map\0")),
    mat_mapping!(MaterialPbrMap::SheenFactor, 0, 0, mat_string(b"fuzz_weight\0")),
    mat_mapping!(MaterialPbrMap::SheenColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"fuzz_color\0")),
    mat_mapping!(MaterialPbrMap::SheenRoughness, 0, 0, mat_string(b"fuzz_roughness\0")),
    mat_mapping!(MaterialPbrMap::EmissionFactor, 0, 0, mat_string(b"emission_weight\0")),
    mat_mapping!(MaterialPbrMap::EmissionFactor, SHADER_MAPPING_MULTIPLY_VALUE, 0, mat_string(b"emission_luminance\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"emission_color\0")),
    mat_mapping!(MaterialPbrMap::ThinFilmFactor, 0, 0, mat_string(b"thin_film_weight\0")),
    mat_mapping!(MaterialPbrMap::ThinFilmThickness, 0, 0, mat_string(b"thin_film_thickness\0")),
    mat_mapping!(MaterialPbrMap::ThinFilmIor, 0, 0, mat_string(b"thin_film_ior\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"bump\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"bump_map_amt\0")),
    mat_mapping!(MaterialPbrMap::DisplacementMap, 0, 0, mat_string(b"displacement\0")),
    mat_mapping!(MaterialPbrMap::DisplacementMap, 0, 0, mat_string(b"displacement_map_amt\0")),
    mat_mapping!(MaterialPbrMap::CoatNormal, 0, 0, mat_string(b"coat_bump\0")),
    mat_mapping!(MaterialPbrMap::CoatNormal, 0, 0, mat_string(b"coat_bump_map_amt\0")),
    mat_mapping!(MaterialPbrMap::TangentMap, 0, 0, mat_string(b"geometry_tangent_map\0")),
    mat_mapping!(MaterialPbrMap::Opacity, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"geometry_opacity\0")),
];

// ufbx.c:19750-19752 `ufbxi_openpbr_material_features`
#[rustfmt::skip]
static OPENPBR_MATERIAL_FEATURES: [ShaderMapping; 1] = [
    mat_mapping!(MaterialFeature::ThinWalled, 0, 0, mat_string(b"geometry_thin_walled\0")),
];

// ufbx.c:19754-19766 `ufbxi_3ds_max_pbr_metal_rough_pbr_mapping`
#[rustfmt::skip]
static E3DS_MAX_PBR_METAL_ROUGH_PBR_MAPPING: [ShaderMapping; 11] = [
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"base_color\0")),
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"baseColor\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, 0, mat_string(b"roughness\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, 0, mat_string(b"Roughness_Map\0")),
    mat_mapping!(MaterialPbrMap::Metalness, 0, 0, mat_string(b"metalness\0")),
    mat_mapping!(MaterialPbrMap::AmbientOcclusion, 0, 0, mat_string(b"ao\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"norm\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"emit_color\0")),
    mat_mapping!(MaterialPbrMap::DisplacementMap, 0, 0, mat_string(b"displacement\0")),
    mat_mapping!(MaterialPbrMap::DisplacementMap, 0, 0, mat_string(b"displacement_amt\0")),
    mat_mapping!(MaterialPbrMap::Opacity, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"opacity\0")),
];

// ufbx.c:19768-19780 `ufbxi_3ds_max_pbr_spec_gloss_pbr_mapping`
#[rustfmt::skip]
static E3DS_MAX_PBR_SPEC_GLOSS_PBR_MAPPING: [ShaderMapping; 11] = [
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"base_color\0")),
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"baseColor\0")),
    mat_mapping!(MaterialPbrMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"Specular\0")),
    mat_mapping!(MaterialPbrMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"specular\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, 0, mat_string(b"glossiness\0")),
    mat_mapping!(MaterialPbrMap::AmbientOcclusion, 0, 0, mat_string(b"ao\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"norm\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"emit_color\0")),
    mat_mapping!(MaterialPbrMap::DisplacementMap, 0, 0, mat_string(b"displacement\0")),
    mat_mapping!(MaterialPbrMap::DisplacementMap, 0, 0, mat_string(b"displacement_amt\0")),
    mat_mapping!(MaterialPbrMap::Opacity, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"opacity\0")),
];

// ufbx.c:19782-19784 `ufbxi_3ds_max_pbr_features`
#[rustfmt::skip]
static E3DS_MAX_PBR_FEATURES: [ShaderMapping; 1] = [
    mat_mapping!(MaterialFeature::RoughnessAsGlossiness, SHADER_FEATURE_IF_AROUND_1, 0, mat_string(b"useGlossiness\0")),
];

// ufbx.c:19786-19794 `ufbxi_gltf_material_features`
#[rustfmt::skip]
static GLTF_MATERIAL_FEATURES: [ShaderMapping; 7] = [
    mat_mapping!(MaterialFeature::DoubleSided, 0, 0, mat_string(b"main|DoubleSided\0")),
    mat_mapping!(MaterialFeature::Sheen, 0, 0, mat_string(b"extension|enableSheen\0")),
    mat_mapping!(MaterialFeature::Coat, 0, 0, mat_string(b"extension|enableClearCoat\0")),
    mat_mapping!(MaterialFeature::Transmission, 0, 0, mat_string(b"extension|enableTransmission\0")),
    mat_mapping!(MaterialFeature::Ior, 0, 0, mat_string(b"extension|enableIndexOfRefraction\0")),
    mat_mapping!(MaterialFeature::Specular, 0, 0, mat_string(b"extension|enableSpecular\0")),
    mat_mapping!(MaterialFeature::Unlit, 0, 0, mat_string(b"extension|unlit\0")),
];

// ufbx.c:19798-19807 `ufbxi_shaderfx_graph_pbr_mapping`
// NOTE: These are just the names used by the standard PBS "preset".
// In _theory_ we could walk ShaderGraph but that's a bit out of scope for ufbx.
#[rustfmt::skip]
static SHADERFX_GRAPH_PBR_MAPPING: [ShaderMapping; 8] = [
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"color\0")),
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"base_color\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, 0, mat_string(b"roughness\0")),
    mat_mapping!(MaterialPbrMap::Metalness, 0, 0, mat_string(b"metallic\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"normal\0")),
    mat_mapping!(MaterialPbrMap::EmissionFactor, 0, 0, mat_string(b"emissive_intensity\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"emissive\0")),
    mat_mapping!(MaterialPbrMap::AmbientOcclusion, 0, 0, mat_string(b"ao\0")),
];

// ufbx.c:19809-19818 `ufbxi_blender_phong_shader_pbr_mapping`
#[rustfmt::skip]
static BLENDER_PHONG_SHADER_PBR_MAPPING: [ShaderMapping; 8] = [
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"DiffuseColor\0")),
    mat_mapping!(MaterialPbrMap::Opacity, SHADER_MAPPING_WIDEN_TO_RGB, MatTransform::BlenderOpacity, mat_string(b"TransparencyFactor\0")),
    mat_mapping!(MaterialPbrMap::EmissionFactor, 0, 0, mat_string(b"EmissiveFactor\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1, 0, mat_string(b"EmissiveColor\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, MatTransform::BlenderShininess, mat_string(b"Shininess\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, MatTransform::BlenderShininess, mat_string(b"ShininessExponent\0")),
    mat_mapping!(MaterialPbrMap::Metalness, 0, 0, mat_string(b"ReflectionFactor\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"NormalMap\0")),
];

// ufbx.c:19820-19839 `ufbxi_obj_pbr_mapping`
#[rustfmt::skip]
static OBJ_PBR_MAPPING: [ShaderMapping; 18] = [
    mat_mapping!(MaterialPbrMap::BaseColor, SHADER_MAPPING_DEFAULT_W_1 | SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"Kd\0")),
    mat_mapping!(MaterialPbrMap::SpecularColor, SHADER_MAPPING_DEFAULT_W_1 | SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"Ks\0")),
    mat_mapping!(MaterialPbrMap::EmissionColor, SHADER_MAPPING_DEFAULT_W_1 | SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"Ke\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, MatTransform::UnknownShininess, mat_string(b"Ns\0")),
    mat_mapping!(MaterialPbrMap::Roughness, 0, 0, mat_string(b"Pr\0")),
    mat_mapping!(MaterialPbrMap::SpecularIor, 0, 0, mat_string(b"Ni\0")),
    mat_mapping!(MaterialPbrMap::Metalness, 0, 0, mat_string(b"Pm\0")),
    mat_mapping!(MaterialPbrMap::Opacity, SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"d\0")),
    mat_mapping!(MaterialPbrMap::TransmissionColor, SHADER_MAPPING_DEFAULT_W_1 | SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"Tf\0")),
    mat_mapping!(MaterialPbrMap::DisplacementMap, 0, 0, mat_string(b"disp\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"bump\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"Bump\0")),
    mat_mapping!(MaterialPbrMap::NormalMap, 0, 0, mat_string(b"norm\0")),
    mat_mapping!(MaterialPbrMap::SheenColor, SHADER_MAPPING_DEFAULT_W_1 | SHADER_MAPPING_WIDEN_TO_RGB, 0, mat_string(b"Ps\0")),
    mat_mapping!(MaterialPbrMap::CoatFactor, 0, 0, mat_string(b"Pc\0")),
    mat_mapping!(MaterialPbrMap::CoatRoughness, 0, 0, mat_string(b"Pcr\0")),
    mat_mapping!(MaterialPbrMap::SpecularAnisotropy, 0, 0, mat_string(b"aniso\0")),
    mat_mapping!(MaterialPbrMap::SpecularRotation, 0, 0, mat_string(b"anisor\0")),
];

// ufbx.c:19841-19851 `ufbxi_obj_features`
#[rustfmt::skip]
static OBJ_FEATURES: [ShaderMapping; 9] = [
    mat_mapping!(MaterialFeature::Pbr, SHADER_FEATURE_IF_EXISTS_OR_TEXTURE, 0, mat_string(b"Pr\0")),
    mat_mapping!(MaterialFeature::Pbr, SHADER_FEATURE_IF_EXISTS_OR_TEXTURE, 0, mat_string(b"Pm\0")),
    mat_mapping!(MaterialFeature::Sheen, SHADER_FEATURE_IF_EXISTS_OR_TEXTURE, 0, mat_string(b"Ps\0")),
    mat_mapping!(MaterialFeature::Coat, SHADER_FEATURE_IF_EXISTS_OR_TEXTURE, 0, mat_string(b"Pc\0")),
    mat_mapping!(MaterialFeature::Metalness, SHADER_FEATURE_IF_EXISTS_OR_TEXTURE, 0, mat_string(b"Pm\0")),
    mat_mapping!(MaterialFeature::Ior, SHADER_FEATURE_IF_EXISTS_OR_TEXTURE, 0, mat_string(b"Ni\0")),
    mat_mapping!(MaterialFeature::Opacity, SHADER_FEATURE_IF_EXISTS_OR_TEXTURE, 0, mat_string(b"d\0")),
    mat_mapping!(MaterialFeature::Transmission, SHADER_FEATURE_IF_EXISTS_OR_TEXTURE, 0, mat_string(b"Tf\0")),
    mat_mapping!(MaterialFeature::Emission, SHADER_FEATURE_IF_EXISTS_OR_TEXTURE, 0, mat_string(b"Ke\0")),
];

// ufbx.c:19853-19874 (anonymous `enum { UFBXI_MAT_* }`)
// C: `int` enumerators; every use site is `(uint32_t)(... | ...)` in
// `ufbxi_shader_pbr_mappings`, so they are declared `u32` here and the cast
// collapses. `ufbx_material_feature_flags` bit positions.
pub(crate) const MAT_PBR: u32 = 1 << MaterialFeature::Pbr as u32;
pub(crate) const MAT_METALNESS: u32 = 1 << MaterialFeature::Metalness as u32;
pub(crate) const MAT_DIFFUSE: u32 = 1 << MaterialFeature::Diffuse as u32;
pub(crate) const MAT_SPECULAR: u32 = 1 << MaterialFeature::Specular as u32;
pub(crate) const MAT_EMISSION: u32 = 1 << MaterialFeature::Emission as u32;
pub(crate) const MAT_COAT: u32 = 1 << MaterialFeature::Coat as u32;
pub(crate) const MAT_SHEEN: u32 = 1 << MaterialFeature::Sheen as u32;
pub(crate) const MAT_TRANSMISSION: u32 = 1 << MaterialFeature::Transmission as u32;
pub(crate) const MAT_OPACITY: u32 = 1 << MaterialFeature::Opacity as u32;
pub(crate) const MAT_AMBIENT_OCCLUSION: u32 = 1 << MaterialFeature::AmbientOcclusion as u32;
// C-parity: the following `UFBXI_MAT_*` enumerators (ufbx.c:19864-19873) mirror
// the full `ufbx_material_feature_flags` bit set but are never referenced in
// ufbx.c — no shader mapping table sets them. C does not warn on unreferenced
// enumerators; they are kept so the bit set stays 1:1 with the public enum.
#[allow(dead_code)]
pub(crate) const MAT_MATTE: u32 = 1 << MaterialFeature::Matte as u32;
#[allow(dead_code)]
pub(crate) const MAT_UNLIT: u32 = 1 << MaterialFeature::Unlit as u32;
pub(crate) const MAT_IOR: u32 = 1 << MaterialFeature::Ior as u32;
pub(crate) const MAT_DIFFUSE_ROUGHNESS: u32 = 1 << MaterialFeature::DiffuseRoughness as u32;
#[allow(dead_code)]
pub(crate) const MAT_TRANSMISSION_ROUGHNESS: u32 =
    1 << MaterialFeature::TransmissionRoughness as u32;
#[allow(dead_code)]
pub(crate) const MAT_THIN_WALLED: u32 = 1 << MaterialFeature::ThinWalled as u32;
#[allow(dead_code)]
pub(crate) const MAT_CAUSTICS: u32 = 1 << MaterialFeature::Caustics as u32;
#[allow(dead_code)]
pub(crate) const MAT_EXIT_TO_BACKGROUND: u32 = 1 << MaterialFeature::ExitToBackground as u32;
#[allow(dead_code)]
pub(crate) const MAT_INTERNAL_REFLECTIONS: u32 = 1 << MaterialFeature::InternalReflections as u32;
#[allow(dead_code)]
pub(crate) const MAT_DOUBLE_SIDED: u32 = 1 << MaterialFeature::DoubleSided as u32;

// C: `{ NULL, 0 }` — the zero-initialized `ufbx_string` the tables below use
// for absent texture prefixes/suffixes. NOT `ufbx_empty_string`, which points
// at `ufbxi_empty_char`; the consumers test `length > 0`, but the pointer is
// observable if that ever changes.
const NULL_STRING: String = String::new_c(ptr::null(), 0);

// ufbx.c:19876-19958 `ufbxi_shader_pbr_mappings`
// C omits the trailing `ufbx_string` members in most entries; C aggregate
// initialization zero-fills them, which is spelled out as `NULL_STRING` here.
// `#[rustfmt::skip]`: keeps the `default_features` feature-bit lists broken
// where C breaks them instead of one bit per line.
#[rustfmt::skip]
static SHADER_PBR_MAPPINGS: [ShaderMappingList; 13] = [
    // UFBX_SHADER_UNKNOWN
    ShaderMappingList {
        data: FBX_PHONG_SHADER_PBR_MAPPING.as_ptr(),
        count: FBX_PHONG_SHADER_PBR_MAPPING.len(),
        features: ptr::null(),
        feature_count: 0,
        default_features: MAT_DIFFUSE | MAT_SPECULAR | MAT_EMISSION | MAT_TRANSMISSION,
        texture_prefix: NULL_STRING,
        texture_suffix: NULL_STRING,
        texture_enabled_prefix: NULL_STRING,
        texture_enabled_suffix: NULL_STRING,
    },
    // UFBX_SHADER_FBX_LAMBERT
    ShaderMappingList {
        data: FBX_LAMBERT_SHADER_PBR_MAPPING.as_ptr(),
        count: FBX_LAMBERT_SHADER_PBR_MAPPING.len(),
        features: ptr::null(),
        feature_count: 0,
        default_features: MAT_DIFFUSE | MAT_EMISSION | MAT_TRANSMISSION,
        texture_prefix: NULL_STRING,
        texture_suffix: NULL_STRING,
        texture_enabled_prefix: NULL_STRING,
        texture_enabled_suffix: NULL_STRING,
    },
    // UFBX_SHADER_FBX_PHONG
    ShaderMappingList {
        data: FBX_PHONG_SHADER_PBR_MAPPING.as_ptr(),
        count: FBX_PHONG_SHADER_PBR_MAPPING.len(),
        features: ptr::null(),
        feature_count: 0,
        default_features: MAT_DIFFUSE | MAT_SPECULAR | MAT_EMISSION | MAT_TRANSMISSION,
        texture_prefix: NULL_STRING,
        texture_suffix: NULL_STRING,
        texture_enabled_prefix: NULL_STRING,
        texture_enabled_suffix: NULL_STRING,
    },
    // UFBX_SHADER_OSL_STANDARD_SURFACE
    ShaderMappingList {
        data: OSL_STANDARD_SHADER_PBR_MAPPING.as_ptr(),
        count: OSL_STANDARD_SHADER_PBR_MAPPING.len(),
        features: OSL_STANDARD_SHADER_FEATURES.as_ptr(),
        feature_count: OSL_STANDARD_SHADER_FEATURES.len(),
        default_features: MAT_PBR | MAT_METALNESS | MAT_DIFFUSE | MAT_SPECULAR | MAT_COAT
            | MAT_SHEEN | MAT_TRANSMISSION | MAT_OPACITY | MAT_IOR | MAT_DIFFUSE_ROUGHNESS,
        texture_prefix: NULL_STRING,
        texture_suffix: NULL_STRING,
        texture_enabled_prefix: NULL_STRING,
        texture_enabled_suffix: NULL_STRING,
    },
    // UFBX_SHADER_ARNOLD_STANDARD_SURFACE
    ShaderMappingList {
        data: ARNOLD_SHADER_PBR_MAPPING.as_ptr(),
        count: ARNOLD_SHADER_PBR_MAPPING.len(),
        features: ARNOLD_SHADER_FEATURES.as_ptr(),
        feature_count: ARNOLD_SHADER_FEATURES.len(),
        default_features: MAT_PBR | MAT_METALNESS | MAT_DIFFUSE | MAT_SPECULAR | MAT_COAT
            | MAT_SHEEN | MAT_TRANSMISSION | MAT_OPACITY | MAT_IOR | MAT_DIFFUSE_ROUGHNESS,
        texture_prefix: NULL_STRING,
        texture_suffix: NULL_STRING,
        texture_enabled_prefix: NULL_STRING,
        texture_enabled_suffix: NULL_STRING,
    },
    // UFBX_SHADER_3DS_MAX_PHYSICAL_MATERIAL
    ShaderMappingList {
        data: E3DS_MAX_PHYSICAL_MATERIAL_PBR_MAPPING.as_ptr(),
        count: E3DS_MAX_PHYSICAL_MATERIAL_PBR_MAPPING.len(),
        features: E3DS_MAX_PHYSICAL_MATERIAL_FEATURES.as_ptr(),
        feature_count: E3DS_MAX_PHYSICAL_MATERIAL_FEATURES.len(),
        default_features: MAT_PBR | MAT_METALNESS | MAT_DIFFUSE | MAT_COAT
            | MAT_SHEEN | MAT_TRANSMISSION | MAT_OPACITY | MAT_IOR,
        texture_prefix: NULL_STRING,
        texture_suffix: ufbxi_string_literal!(b"_map\0"), // texture_prefix/suffix
        texture_enabled_prefix: NULL_STRING,
        texture_enabled_suffix: ufbxi_string_literal!(b"_map_on\0"), // texture_enabled_prefix/suffix
    },
    // UFBX_SHADER_3DS_MAX_PBR_METAL_ROUGH
    ShaderMappingList {
        data: E3DS_MAX_PBR_METAL_ROUGH_PBR_MAPPING.as_ptr(),
        count: E3DS_MAX_PBR_METAL_ROUGH_PBR_MAPPING.len(),
        features: E3DS_MAX_PBR_FEATURES.as_ptr(),
        feature_count: E3DS_MAX_PBR_FEATURES.len(),
        default_features: MAT_PBR | MAT_METALNESS | MAT_DIFFUSE | MAT_OPACITY,
        texture_prefix: NULL_STRING,
        texture_suffix: ufbxi_string_literal!(b"_map\0"), // texture_prefix/suffix
        texture_enabled_prefix: NULL_STRING,
        texture_enabled_suffix: NULL_STRING, // texture_enabled_prefix/suffix
    },
    // UFBX_SHADER_3DS_MAX_PBR_SPEC_GLOSS
    ShaderMappingList {
        data: E3DS_MAX_PBR_SPEC_GLOSS_PBR_MAPPING.as_ptr(),
        count: E3DS_MAX_PBR_SPEC_GLOSS_PBR_MAPPING.len(),
        features: E3DS_MAX_PBR_FEATURES.as_ptr(),
        feature_count: E3DS_MAX_PBR_FEATURES.len(),
        default_features: MAT_PBR | MAT_SPECULAR | MAT_DIFFUSE | MAT_OPACITY,
        texture_prefix: NULL_STRING,
        texture_suffix: ufbxi_string_literal!(b"_map\0"), // texture_prefix/suffix
        texture_enabled_prefix: NULL_STRING,
        texture_enabled_suffix: NULL_STRING, // texture_enabled_prefix/suffix
    },
    // UFBX_SHADER_GLTF_MATERIAL
    ShaderMappingList {
        data: GLTF_MATERIAL_PBR_MAPPING.as_ptr(),
        count: GLTF_MATERIAL_PBR_MAPPING.len(),
        features: GLTF_MATERIAL_FEATURES.as_ptr(),
        feature_count: GLTF_MATERIAL_FEATURES.len(),
        default_features: MAT_PBR | MAT_METALNESS | MAT_DIFFUSE | MAT_EMISSION | MAT_OPACITY | MAT_AMBIENT_OCCLUSION,
        texture_prefix: NULL_STRING,
        texture_suffix: ufbxi_string_literal!(b"Map\0"), // texture_prefix/suffix
        texture_enabled_prefix: NULL_STRING,
        texture_enabled_suffix: NULL_STRING, // texture_enabled_prefix/suffix
    },
    // UFBX_SHADER_OPENPBR_MATERIAL
    ShaderMappingList {
        data: OPENPBR_MATERIAL_PBR_MAPPING.as_ptr(),
        count: OPENPBR_MATERIAL_PBR_MAPPING.len(),
        features: OPENPBR_MATERIAL_FEATURES.as_ptr(),
        feature_count: OPENPBR_MATERIAL_FEATURES.len(),
        default_features: MAT_PBR | MAT_METALNESS | MAT_DIFFUSE | MAT_SPECULAR | MAT_COAT
            | MAT_SHEEN | MAT_TRANSMISSION | MAT_OPACITY | MAT_IOR | MAT_DIFFUSE_ROUGHNESS,
        texture_prefix: NULL_STRING,
        texture_suffix: ufbxi_string_literal!(b"_map\0"), // texture_prefix/suffix
        texture_enabled_prefix: NULL_STRING,
        texture_enabled_suffix: ufbxi_string_literal!(b"_map_on\0"), // texture_enabled_prefix/suffix
    },
    // UFBX_SHADER_SHADERFX_GRAPH
    ShaderMappingList {
        data: SHADERFX_GRAPH_PBR_MAPPING.as_ptr(),
        count: SHADERFX_GRAPH_PBR_MAPPING.len(),
        features: ptr::null(),
        feature_count: 0,
        default_features: MAT_PBR | MAT_METALNESS | MAT_DIFFUSE | MAT_EMISSION | MAT_AMBIENT_OCCLUSION,
        texture_prefix: ufbxi_string_literal!(b"TEX_\0"),
        texture_suffix: ufbxi_string_literal!(b"_map\0"), // texture_prefix/suffix
        texture_enabled_prefix: ufbxi_string_literal!(b"use_\0"),
        texture_enabled_suffix: ufbxi_string_literal!(b"_map\0"), // texture_enabled_prefix/suffix
    },
    // UFBX_SHADER_BLENDER_PHONG
    ShaderMappingList {
        data: BLENDER_PHONG_SHADER_PBR_MAPPING.as_ptr(),
        count: BLENDER_PHONG_SHADER_PBR_MAPPING.len(),
        features: ptr::null(),
        feature_count: 0,
        default_features: MAT_PBR | MAT_METALNESS | MAT_DIFFUSE | MAT_EMISSION,
        texture_prefix: NULL_STRING,
        texture_suffix: NULL_STRING,
        texture_enabled_prefix: NULL_STRING,
        texture_enabled_suffix: NULL_STRING,
    },
    // UFBX_SHADER_WAVEFRONT_MTL
    ShaderMappingList {
        data: OBJ_PBR_MAPPING.as_ptr(),
        count: OBJ_PBR_MAPPING.len(),
        features: OBJ_FEATURES.as_ptr(),
        feature_count: OBJ_FEATURES.len(),
        default_features: MAT_DIFFUSE | MAT_SPECULAR,
        texture_prefix: NULL_STRING,
        texture_suffix: NULL_STRING,
        texture_enabled_prefix: NULL_STRING,
        texture_enabled_suffix: NULL_STRING,
    },
];

// ufbx.c:19960 `ufbx_static_assert(shader_pbr_mapping_list, ufbxi_arraycount(ufbxi_shader_pbr_mappings) == UFBX_SHADER_TYPE_COUNT);`
const _: () = assert!(SHADER_PBR_MAPPINGS.len() == SHADER_TYPE_COUNT);

// ufbx.c:19962-19967 `enum { UFBXI_MAPPING_FETCH_* }`
// C: an anonymous bit-flag enum; the values are passed as the `uint32_t flags`
// argument of `ufbxi_fetch_mapping_maps()`.
pub(crate) const MAPPING_FETCH_VALUE: u32 = 0x1;
pub(crate) const MAPPING_FETCH_TEXTURE: u32 = 0x2;
pub(crate) const MAPPING_FETCH_TEXTURE_ENABLED: u32 = 0x4;
pub(crate) const MAPPING_FETCH_FEATURE: u32 = 0x8;

// ufbx.c:19969-20094 `ufbxi_fetch_mapping_maps`
#[inline(never)]
pub(crate) unsafe fn fetch_mapping_maps(
    material: *mut Material,
    maps: *mut MaterialMap,
    features: *mut MaterialFeatureInfo,
    shader: *mut Shader,
    mappings: *const ShaderMapping,
    count: usize,
    prefix: String,
    prefix2: String,
    suffix: String,
    flags: u32,
) {
    // C: `char combined_name[512];` — an uninitialized local (upstream carries
    // no `// ufbxi_uninit` marker at ufbx.c:19972); only the bytes the memcpy
    // chain below writes are ever read back.
    let mut combined_name_storage = MaybeUninit::<[u8; 512]>::uninit();
    let combined_name: *mut u8 = combined_name_storage.as_mut_ptr() as *mut u8;
    // C: `ufbx_shader_prop_binding identity_binding;` — likewise uninitialized
    // (ufbx.c:19973); both members are assigned before its address is taken.
    let mut identity_binding_storage = MaybeUninit::<ShaderPropBinding>::uninit();
    let identity_binding: *mut ShaderPropBinding = identity_binding_storage.as_mut_ptr();

    // C: `ufbxi_for(const ufbxi_shader_mapping, mapping, mappings, count)`
    let mut mapping: *const ShaderMapping = mappings;
    let mapping_end: *const ShaderMapping = mappings.wrapping_add(count);
    while mapping != mapping_end {
        // C: `ufbx_string prop_name = { mapping->prop, mapping->prop_len };`
        let mut prop_name: String = String::new_c((*mapping).prop, (*mapping).prop_len as usize);
        if prefix.length > 0 || prefix2.length > 0 || suffix.length > 0 {
            // C: `sizeof(combined_name)` — the array is `char[512]`.
            if prop_name.length + prefix.length + prefix2.length + suffix.length
                <= size_of::<[u8; 512]>()
            {
                let mut dst: *mut u8 = combined_name;

                if prefix.length > 0 {
                    ptr::copy_nonoverlapping(prefix.data, dst, prefix.length);
                    dst = dst.add(prefix.length);
                }
                if prefix2.length > 0 {
                    ptr::copy_nonoverlapping(prefix2.data, dst, prefix2.length);
                    dst = dst.add(prefix2.length);
                }
                if prop_name.length > 0 {
                    ptr::copy_nonoverlapping(prop_name.data, dst, prop_name.length);
                    dst = dst.add(prop_name.length);
                }
                if suffix.length > 0 {
                    ptr::copy_nonoverlapping(suffix.data, dst, suffix.length);
                    dst = dst.add(suffix.length);
                }

                prop_name.data = combined_name;
                prop_name.length = to_size(dst.offset_from(combined_name));
            }
        }

        let mut bindings: List<ShaderPropBinding> =
            find_shader_prop_bindings_len(shader, prop_name.data, prop_name.length);
        if bindings.count == 0 {
            (*identity_binding).material_prop = prop_name;
            (*identity_binding).shader_prop = EMPTY_STRING.0;
            bindings.data = identity_binding;
            bindings.count = 1;
        }

        let mapping_flags: u32 = (*mapping).flags as u32;
        // C: `ufbxi_for_list(ufbx_shader_prop_binding, binding, bindings)`
        let mut binding: *const ShaderPropBinding = bindings.data;
        let binding_end: *const ShaderPropBinding = bindings.data.wrapping_add(bindings.count);
        while binding != binding_end {
            let name: String = (*binding).material_prop;

            let prop: *mut Prop = find_prop_len(&(*material).element.props, name.data, name.length);
            if (flags & MAPPING_FETCH_FEATURE) != 0 {
                let feature: *mut MaterialFeatureInfo = features.add((*mapping).index as usize);
                if !prop.is_null() && (*prop).type_ != PropType::Reference {
                    (*feature).enabled = (*prop).value_int != 0;
                    (*feature).is_explicit = true;
                    if (mapping_flags & SHADER_FEATURE_IF_AROUND_1 as u32) != 0 {
                        // C-parity: `prop->value_real` is the `ufbx_prop` value
                        // union's first real (`value_vec4.x` here).
                        (*feature).enabled = (*prop).value_vec4.x >= 0.5f32 as Real
                            && (*prop).value_vec4.x <= 1.5f32 as Real;
                    }
                    if (mapping_flags & SHADER_FEATURE_INVERTED as u32) != 0 {
                        (*feature).enabled = !(*feature).enabled;
                    }
                    if (mapping_flags & SHADER_FEATURE_IF_EXISTS as u32) != 0 {
                        (*feature).enabled = true;
                    }
                }
                if (mapping_flags & SHADER_FEATURE_IF_TEXTURE as u32) != 0 {
                    let texture: *mut Texture =
                        find_prop_texture_len(material, name.data, name.length);
                    if !texture.is_null() {
                        (*feature).enabled = true;
                    }
                }
                binding = binding.add(1);
                continue;
            }

            let map: *mut MaterialMap = maps.add((*mapping).index as usize);

            if (flags & MAPPING_FETCH_VALUE) != 0 {
                if !prop.is_null() && (*prop).type_ != PropType::Reference {
                    if ((*mapping).flags & SHADER_MAPPING_MULTIPLY_VALUE) != 0 {
                        (*map).value_vec4.x *= (*prop).value_vec4.x;
                        // C: `ufbxi_f64_to_i64(map->value_vec4.x)` — the real
                        // argument promotes to double at the call.
                        (*map).value_int = f64_to_i64((*map).value_vec4.x as f64);
                    } else {
                        (*map).value_vec4 = (*prop).value_vec4;
                        (*map).value_int = (*prop).value_int;
                    }
                    (*map).has_value = true;
                    if (*mapping).transform != 0 {
                        let transform_fn: MatTransformFn =
                            MAT_TRANSFORM_FNS[(*mapping).transform as usize].unwrap_unchecked();
                        transform_fn(&mut (*map).value_vec4);
                    }

                    let prop_flags: u32 = (*prop).flags.raw();
                    if ((*mapping).flags & SHADER_MAPPING_DEFAULT_W_1) != 0
                        && (prop_flags & PropFlags::VALUE_VEC4.raw()) == 0
                    {
                        (*map).value_vec4.w = 1.0f32 as Real;
                    }
                    if ((*mapping).flags & SHADER_MAPPING_WIDEN_TO_RGB) != 0
                        && (prop_flags & PropFlags::VALUE_REAL.raw()) != 0
                    {
                        // C-parity: `map->value_vec3` is the `ufbx_material_map`
                        // value union's 3-real view; the generated struct keeps
                        // only `value_vec4`, whose x/y/z overlay it exactly.
                        (*map).value_vec4.y = (*map).value_vec4.x;
                        (*map).value_vec4.z = (*map).value_vec4.x;
                    }
                    if (prop_flags & PropFlags::VALUE_REAL.raw()) != 0 {
                        (*map).value_components = 1;
                    } else if (prop_flags & PropFlags::VALUE_VEC2.raw()) != 0 {
                        (*map).value_components = 2;
                    } else if (prop_flags & PropFlags::VALUE_VEC3.raw()) != 0 {
                        (*map).value_components = 3;
                    } else if (prop_flags & PropFlags::VALUE_VEC4.raw()) != 0 {
                        (*map).value_components = 4;
                    } else {
                        (*map).value_components = 0;
                    }
                }
            }

            if (flags & MAPPING_FETCH_TEXTURE) != 0 {
                let texture: *mut Texture = find_prop_texture_len(material, name.data, name.length);
                if !texture.is_null() {
                    (*map).texture = opt_ref(texture);
                    (*map).texture_enabled = true;
                }
            }

            if (flags & MAPPING_FETCH_TEXTURE_ENABLED) != 0 {
                if !prop.is_null() {
                    (*map).texture_enabled = (*prop).value_int != 0;
                }
            }
            binding = binding.add(1);
        }
        mapping = mapping.add(1);
    }
}

// ufbx.c:20096-20107 `ufbxi_update_factor`
#[inline(never)]
pub(crate) unsafe fn update_factor(factor_map: *mut MaterialMap, color_map: *mut MaterialMap) {
    if !(*factor_map).has_value {
        if (*color_map).has_value && !is_vec4_zero((*color_map).value_vec4) {
            // C-parity: `factor_map->value_real` is the value union's first
            // real (`value_vec4.x` in the generated struct).
            (*factor_map).value_vec4.x = 1.0f32 as Real;
            (*factor_map).value_int = 1;
        } else {
            (*factor_map).value_vec4.x = 0.0f32 as Real;
            (*factor_map).value_int = 0;
        }
    }
}

// Some material modes have toggleable roughness/glossiness mode, we read it initially
// always as roughness and if a matching feature such as `roughness_as_glossiness` is set
// we transfer the data into the glossiness and invert the roughness.
// (ufbx.c:20109-20116 `ufbxi_glossiness_remap`)
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct GlossinessRemap {
    pub feature: u8,
    pub roughness_map: u8,
    pub glossiness_map: u8,
}

// ufbx.c:20118-20122 `ufbxi_glossiness_remaps`
#[rustfmt::skip]
static GLOSSINESS_REMAPS: [GlossinessRemap; 3] = [
    GlossinessRemap { feature: MaterialFeature::RoughnessAsGlossiness as u8, roughness_map: MaterialPbrMap::Roughness as u8, glossiness_map: MaterialPbrMap::Glossiness as u8 },
    GlossinessRemap { feature: MaterialFeature::CoatRoughnessAsGlossiness as u8, roughness_map: MaterialPbrMap::CoatRoughness as u8, glossiness_map: MaterialPbrMap::CoatGlossiness as u8 },
    GlossinessRemap { feature: MaterialFeature::TransmissionRoughnessAsGlossiness as u8, roughness_map: MaterialPbrMap::TransmissionRoughness as u8, glossiness_map: MaterialPbrMap::TransmissionGlossiness as u8 },
];

// ufbx.h:2511 `UFBX_ENUM_TYPE(ufbx_material_feature, UFBX_MATERIAL_FEATURE, UFBX_MATERIAL_FEATURE_TRANSMISSION_ROUGHNESS_AS_GLOSSINESS);`
// expanding via ufbx.h:235-236 to
// `enum { UFBX_MATERIAL_FEATURE_COUNT = UFBX_MATERIAL_FEATURE_TRANSMISSION_ROUGHNESS_AS_GLOSSINESS + 1 }`
// (same hand-duplication as `SHADER_TYPE_COUNT` above).
pub(crate) const MATERIAL_FEATURE_COUNT: u32 =
    MaterialFeature::TransmissionRoughnessAsGlossiness as u32 + 1;

// ufbx.h:2416 `UFBX_ENUM_TYPE(ufbx_material_fbx_map, UFBX_MATERIAL_FBX_MAP, UFBX_MATERIAL_FBX_VECTOR_DISPLACEMENT);`
pub(crate) const MATERIAL_FBX_MAP_COUNT: usize = MaterialFbxMap::VectorDisplacement as usize + 1;
// ufbx.h:2480 `UFBX_ENUM_TYPE(ufbx_material_pbr_map, UFBX_MATERIAL_PBR_MAP, UFBX_MATERIAL_PBR_TRANSMISSION_GLOSSINESS);`
pub(crate) const MATERIAL_PBR_MAP_COUNT: usize =
    MaterialPbrMap::TransmissionGlossiness as usize + 1;

// Layout pin for the flat `maps[]` / `features[]` union members of
// `ufbx_material_fbx_maps` (ufbx.h:2513-2539), `ufbx_material_pbr_maps`
// (2541-2603) and `ufbx_material_features` (2605-2634): the generator emits
// only the named-member struct arm, so `ufbxi_fetch_maps` recovers the array
// view by casting the whole aggregate (PORTING.md "Unions and flexible array
// members" — overlays stay overlays, pinned by a const assert).
const _: () =
    assert!(size_of::<MaterialFbxMaps>() == size_of::<MaterialMap>() * MATERIAL_FBX_MAP_COUNT);
const _: () =
    assert!(size_of::<MaterialPbrMaps>() == size_of::<MaterialMap>() * MATERIAL_PBR_MAP_COUNT);
const _: () = assert!(
    size_of::<MaterialFeatures>()
        == size_of::<MaterialFeatureInfo>() * MATERIAL_FEATURE_COUNT as usize
);

// ufbx.c:20124-20216 `ufbxi_fetch_maps`
#[inline(never)]
pub(crate) unsafe fn fetch_maps(scene: *mut Scene, material: *mut Material) {
    ufbxi_ignore!(scene);

    let shader: *mut Shader = opt_ptr(&(*material).shader);
    ufbx_assert!(((*material).shader_type as u32) < SHADER_TYPE_COUNT as u32);

    // C-parity: `ufbx_material_fbx_maps` / `_pbr_maps` / `ufbx_material_features`
    // are unions of a named-member struct and a flat array; the generator keeps
    // only the named struct, so the array view is recovered by casting the
    // whole aggregate (identical layout, PORTING.md "Unions").
    ptr::write_bytes(
        ptr::addr_of_mut!((*material).fbx) as *mut u8,
        0,
        size_of::<MaterialFbxMaps>(),
    );
    ptr::write_bytes(
        ptr::addr_of_mut!((*material).pbr) as *mut u8,
        0,
        size_of::<MaterialPbrMaps>(),
    );
    ptr::write_bytes(
        ptr::addr_of_mut!((*material).features) as *mut u8,
        0,
        size_of::<MaterialFeatures>(),
    );

    // These array views stay live for the rest of the function (the glossiness
    // remap loop below writes through `pbr_maps`/`feature_infos`), so every
    // other access to `material` in this body must be a raw place projection or
    // an `addr_of_mut!` — never a `&mut`, which would retag and invalidate them.
    let fbx_maps: *mut MaterialMap = ptr::addr_of_mut!((*material).fbx) as *mut MaterialMap;
    let pbr_maps: *mut MaterialMap = ptr::addr_of_mut!((*material).pbr) as *mut MaterialMap;
    let feature_infos: *mut MaterialFeatureInfo =
        ptr::addr_of_mut!((*material).features) as *mut MaterialFeatureInfo;

    let mut base_mapping: *const ShaderMapping = BASE_FBX_MAPPING.as_ptr();
    let mut num_base_mapping: usize = BASE_FBX_MAPPING.len();

    if (*scene).metadata.file_format == FileFormat::Obj
        || (*scene).metadata.file_format == FileFormat::Mtl
    {
        base_mapping = OBJ_FBX_MAPPING.as_ptr();
        num_base_mapping = OBJ_FBX_MAPPING.len();
    }

    fetch_mapping_maps(
        material,
        fbx_maps,
        ptr::null_mut(),
        ptr::null_mut(),
        base_mapping,
        num_base_mapping,
        EMPTY_STRING.0,
        EMPTY_STRING.0,
        EMPTY_STRING.0,
        MAPPING_FETCH_VALUE | MAPPING_FETCH_TEXTURE,
    );

    let list: ShaderMappingList = SHADER_PBR_MAPPINGS[(*material).shader_type as usize];

    for i in 0..MATERIAL_FEATURE_COUNT {
        if (list.default_features & (1u32 << i)) != 0 {
            (*feature_infos.add(i as usize)).enabled = true;
        }
    }

    let mut prefix: String = EMPTY_STRING.0;
    if shader.is_null() {
        prefix = (*material).shader_prop_prefix;
    }

    if list.texture_prefix.length > 0 || list.texture_suffix.length > 0 {
        fetch_mapping_maps(
            material,
            pbr_maps,
            ptr::null_mut(),
            shader,
            list.data,
            list.count,
            prefix,
            list.texture_prefix,
            list.texture_suffix,
            MAPPING_FETCH_TEXTURE,
        );
    }

    fetch_mapping_maps(
        material,
        pbr_maps,
        ptr::null_mut(),
        shader,
        list.data,
        list.count,
        prefix,
        EMPTY_STRING.0,
        EMPTY_STRING.0,
        MAPPING_FETCH_VALUE | MAPPING_FETCH_TEXTURE,
    );

    if list.texture_enabled_prefix.length > 0 || list.texture_enabled_suffix.length > 0 {
        fetch_mapping_maps(
            material,
            pbr_maps,
            ptr::null_mut(),
            shader,
            list.data,
            list.count,
            prefix,
            list.texture_enabled_prefix,
            list.texture_enabled_suffix,
            MAPPING_FETCH_TEXTURE_ENABLED,
        );
    }

    fetch_mapping_maps(
        material,
        ptr::null_mut(),
        feature_infos,
        shader,
        list.features,
        list.feature_count,
        prefix,
        EMPTY_STRING.0,
        EMPTY_STRING.0,
        MAPPING_FETCH_FEATURE,
    );

    update_factor(
        ptr::addr_of_mut!((*material).fbx.diffuse_factor),
        ptr::addr_of_mut!((*material).fbx.diffuse_color),
    );
    update_factor(
        ptr::addr_of_mut!((*material).fbx.specular_factor),
        ptr::addr_of_mut!((*material).fbx.specular_color),
    );
    update_factor(
        ptr::addr_of_mut!((*material).fbx.reflection_factor),
        ptr::addr_of_mut!((*material).fbx.reflection_color),
    );
    update_factor(
        ptr::addr_of_mut!((*material).fbx.transparency_factor),
        ptr::addr_of_mut!((*material).fbx.transparency_color),
    );
    update_factor(
        ptr::addr_of_mut!((*material).fbx.emission_factor),
        ptr::addr_of_mut!((*material).fbx.emission_color),
    );
    update_factor(
        ptr::addr_of_mut!((*material).fbx.ambient_factor),
        ptr::addr_of_mut!((*material).fbx.ambient_color),
    );

    update_factor(
        ptr::addr_of_mut!((*material).pbr.base_factor),
        ptr::addr_of_mut!((*material).pbr.base_color),
    );
    update_factor(
        ptr::addr_of_mut!((*material).pbr.specular_factor),
        ptr::addr_of_mut!((*material).pbr.specular_color),
    );
    update_factor(
        ptr::addr_of_mut!((*material).pbr.emission_factor),
        ptr::addr_of_mut!((*material).pbr.emission_color),
    );
    update_factor(
        ptr::addr_of_mut!((*material).pbr.sheen_factor),
        ptr::addr_of_mut!((*material).pbr.sheen_color),
    );
    update_factor(
        ptr::addr_of_mut!((*material).pbr.thin_film_factor),
        ptr::addr_of_mut!((*material).pbr.thin_film_thickness),
    );
    update_factor(
        ptr::addr_of_mut!((*material).pbr.transmission_factor),
        ptr::addr_of_mut!((*material).pbr.transmission_color),
    );

    // Patch transmission roughness if only extra roughness is defined
    if !(*material).pbr.transmission_roughness.has_value
        && (*material).pbr.roughness.has_value
        && (*material).pbr.transmission_extra_roughness.has_value
    {
        // C-parity: `.value_real` is the value union's first real.
        (*material).pbr.transmission_roughness.value_vec4.x =
            (*material).pbr.roughness.value_vec4.x
                + (*material).pbr.transmission_extra_roughness.value_vec4.x;
    }

    // Map roughness to glossiness and vice versa
    // C: `ufbxi_for(const ufbxi_glossiness_remap, remap, ufbxi_glossiness_remaps, ufbxi_arraycount(ufbxi_glossiness_remaps))`
    let mut remap: *const GlossinessRemap = GLOSSINESS_REMAPS.as_ptr();
    let remap_end: *const GlossinessRemap = GLOSSINESS_REMAPS
        .as_ptr()
        .wrapping_add(GLOSSINESS_REMAPS.len());
    while remap != remap_end {
        let roughness: *mut MaterialMap = pbr_maps.add((*remap).roughness_map as usize);
        let glossiness: *mut MaterialMap = pbr_maps.add((*remap).glossiness_map as usize);
        if (*feature_infos.add((*remap).feature as usize)).enabled {
            // C: `*glossiness = *roughness;` — struct assignment is a memcpy
            // (PORTING.md checklist #15); `ufbx_material_map` is not `Copy` in
            // the generated bindings, so the copy is spelled out.
            ptr::copy_nonoverlapping(roughness, glossiness, 1);
            ptr::write_bytes(roughness as *mut u8, 0, size_of::<MaterialMap>());
            if (*glossiness).has_value {
                (*roughness).value_vec4.x = 1.0f32 as Real - (*glossiness).value_vec4.x;
            }
        } else {
            if (*roughness).has_value {
                (*glossiness).value_vec4.x = 1.0f32 as Real - (*roughness).value_vec4.x;
            }
        }
        remap = remap.add(1);
    }
}

// ufbx.c:20218-20224 `ufbxi_constraint_prop_type`
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ConstraintPropType {
    Node,
    IkEffector,
    IkEndNode,
    AimUp,
    Target,
}

// ufbx.c:20226-20229 `ufbxi_constraint_prop`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ConstraintProp {
    pub type_: ConstraintPropType,
    pub name: *const u8,
}
// The table below is immutable and its `const char *` member references an
// immutable string literal, so sharing is sound (same rationale as
// `ShaderMapping` above).
unsafe impl Sync for ConstraintProp {}

// ufbx.c:20231-20242 `ufbxi_constraint_props`
#[rustfmt::skip]
static CONSTRAINT_PROPS: [ConstraintProp; 10] = [
    ConstraintProp { type_: ConstraintPropType::Node, name: b"Constrained Object\0".as_ptr() },
    ConstraintProp { type_: ConstraintPropType::Node, name: b"Constrained object (Child)\0".as_ptr() },
    ConstraintProp { type_: ConstraintPropType::Node, name: b"First Joint\0".as_ptr() },
    ConstraintProp { type_: ConstraintPropType::Target, name: b"Source\0".as_ptr() },
    ConstraintProp { type_: ConstraintPropType::Target, name: b"Source (Parent)\0".as_ptr() },
    ConstraintProp { type_: ConstraintPropType::Target, name: b"Aim At Object\0".as_ptr() },
    ConstraintProp { type_: ConstraintPropType::Target, name: b"Pole Vector Object\0".as_ptr() },
    ConstraintProp { type_: ConstraintPropType::IkEffector, name: b"Effector\0".as_ptr() },
    ConstraintProp { type_: ConstraintPropType::IkEndNode, name: b"End Joint\0".as_ptr() },
    ConstraintProp { type_: ConstraintPropType::AimUp, name: b"World Up Object\0".as_ptr() },
];

// ufbx.c:20244-20266 `ufbxi_add_constraint_prop`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn add_constraint_prop(
    uc: &Context,
    constraint: *mut Constraint,
    node: *mut Node,
    prop: *const u8,
) -> Result<(), Fail> {
    // C: `ufbxi_for(const ufbxi_constraint_prop, cprop, ufbxi_constraint_props, ufbxi_arraycount(ufbxi_constraint_props))`
    let mut cprop: *const ConstraintProp = CONSTRAINT_PROPS.as_ptr();
    let cprop_end: *const ConstraintProp = CONSTRAINT_PROPS
        .as_ptr()
        .wrapping_add(CONSTRAINT_PROPS.len());
    while cprop != cprop_end {
        if strcmp((*cprop).name, prop) != 0 {
            cprop = cprop.add(1);
            continue;
        }
        match (*cprop).type_ {
            ConstraintPropType::Node => (*constraint).node = opt_ref(node),
            ConstraintPropType::IkEffector => (*constraint).ik_effector = opt_ref(node),
            ConstraintPropType::IkEndNode => (*constraint).ik_end_node = opt_ref(node),
            ConstraintPropType::AimUp => (*constraint).aim_up_node = opt_ref(node),
            ConstraintPropType::Target => {
                let target: *mut ConstraintTarget = push_zero(uc.tmp_stack_mut_ptr(), 1);
                ufbxi_check!(uc, !target.is_null(), "target");
                // `ufbx_constraint_target.node` is a non-nullable
                // `ufbx_node*`; the sole caller (ufbx.c:22576/22580) passes a
                // connection endpoint it already checked is a
                // `UFBX_ELEMENT_NODE`, so `node` is never NULL here.
                (*target).node = Ref::from_ptr(node);
                (*target).weight = 1.0f32 as Real;
                (*target).transform = IDENTITY_TRANSFORM;
            }
            // C `default:` (ufbx.c:20260-20261) — unreachable in Rust because
            // the match above is exhaustive over the enum, but kept for diff
            // parity (PORTING.md "Asserts": `ufbxi_unreachable` is never
            // collapsed away).
            #[allow(unreachable_patterns)]
            _ => {
                ufbxi_unreachable!("Unexpected constraint prop");
            }
        }
        cprop = cprop.add(1);
    }

    Ok(())
}

// ufbx.c:20268-20312 `ufbxi_finalize_nurbs_basis`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn finalize_nurbs_basis(
    uc: &Context,
    basis: *mut NurbsBasis,
) -> Result<(), Fail> {
    if (*basis).topology == NurbsTopology::Closed {
        (*basis).num_wrap_control_points = 1;
    } else if (*basis).topology == NurbsTopology::Periodic {
        (*basis).num_wrap_control_points = (*basis).order.wrapping_sub(1) as usize;
    } else {
        (*basis).num_wrap_control_points = 0;
    }

    if (*basis).order > 1 {
        let degree: usize = ((*basis).order - 1) as usize;
        // C: `ufbx_real_list knots = basis->knot_vector;` — `List<T>` is not
        // `Copy` in the bindings, so the value copy is spelled out.
        let mut knots: List<Real> = MaybeUninit::zeroed().assume_init();
        knots.data = (*basis).knot_vector.data;
        knots.count = (*basis).knot_vector.count;
        if knots.count >= 2 * degree + 1 {
            (*basis).t_min = *knots.data.add(degree);
            (*basis).t_max = *knots.data.add(knots.count - degree - 1);

            let max_spans: usize = knots.count - 2 * degree;
            let spans: *mut Real = push(uc.result_mut_ptr(), max_spans);
            ufbxi_check!(uc, !spans.is_null(), "spans");

            let mut prev: Real = -math::INFINITY as Real;
            let mut num_spans: usize = 0;
            for i in 0..max_spans {
                let t: Real = *knots.data.add(degree + i);
                if t != prev {
                    *spans.add(num_spans) = t;
                    num_spans += 1;
                    prev = t;
                }
            }

            (*basis).spans.data = spans;
            (*basis).spans.count = num_spans;
            (*basis).valid = true;
            for i in 1..knots.count {
                if *knots.data.add(i - 1) > *knots.data.add(i) {
                    (*basis).valid = false;
                    break;
                }
            }
        }
    }

    Ok(())
}

// ufbx.c:20314-20362 `ufbxi_finalize_lod_group`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn finalize_lod_group(uc: &Context, lod: *mut LodGroup) -> Result<(), Fail> {
    let mut num_levels: usize = 0;
    for _i in 0..(*lod).element.instances.count {
        // C-parity: the subscript really is `instances.data[0]` (not `[i]`) —
        // ufbx.c:20318.
        num_levels = max_sz(
            num_levels,
            (*ref_ptr((*lod).element.instances.data)).children.count,
        );
    }

    // C: `char prop_name[64];` — uninitialized local (no upstream
    // `// ufbxi_uninit` marker at ufbx.c:20321); `ufbxi_snprintf` writes it.
    let mut prop_name_storage = MaybeUninit::<[u8; 64]>::uninit();
    let prop_name: *mut u8 = prop_name_storage.as_mut_ptr() as *mut u8;
    let mut i: usize = 0;
    loop {
        let len: i32 = ufbxi_snprintf!(prop_name, size_of::<[u8; 64]>(), "Thresholds|Level%zu", i);
        let prop: *mut Prop = find_prop_len(&(*lod).element.props, prop_name, len as usize);
        if prop.is_null() {
            break;
        }
        num_levels = max_sz(num_levels, i + 1);
        i += 1;
    }

    let levels: *mut LodLevel = push_zero(uc.result_mut_ptr(), num_levels);
    ufbxi_check!(uc, !levels.is_null(), "levels");

    (*lod).relative_distances = api_find_bool(
        &(*lod).element.props,
        b"ThresholdsUsedAsPercentage\0".as_ptr(),
        false,
    );
    (*lod).ignore_parent_transform =
        !api_find_bool(&(*lod).element.props, b"WorldSpace\0".as_ptr(), true);

    (*lod).use_distance_limit =
        api_find_bool(&(*lod).element.props, b"MinMaxDistance\0".as_ptr(), false);
    (*lod).distance_limit_min = api_find_real(
        &(*lod).element.props,
        b"MinDistance\0".as_ptr(),
        -100.0 as Real,
    );
    (*lod).distance_limit_max = api_find_real(
        &(*lod).element.props,
        b"MaxDistance\0".as_ptr(),
        100.0 as Real,
    );

    (*lod).lod_levels.data = levels;
    (*lod).lod_levels.count = num_levels;

    for i in 0..num_levels {
        let level: *mut LodLevel = levels.add(i);

        if i > 0 {
            let len: i32 = ufbxi_snprintf!(
                prop_name,
                size_of::<[u8; 64]>(),
                "Thresholds|Level%zu",
                i - 1
            );
            (*level).distance = api_find_real_len(
                &(*lod).element.props,
                prop_name,
                len as usize,
                0.0f32 as Real,
            );
        } else if (*lod).relative_distances {
            (*level).distance = 100.0 as Real;
        }

        {
            let len: i32 = ufbxi_snprintf!(
                prop_name,
                size_of::<[u8; 64]>(),
                "DisplayLevels|Level%zu",
                i
            );
            let display: i64 = api_find_int_len(&(*lod).element.props, prop_name, len as usize, 0);
            if display >= 0 && display <= 2 {
                // C: `(ufbx_lod_display)display` — guarded to [0, 2], every
                // value of which is a valid `ufbx_lod_display`.
                (*level).display = core::mem::transmute::<u32, LodDisplay>(display as u32);
            }
        }
    }

    Ok(())
}

// ufbx.c:20363-20403 `ufbxi_generate_normals`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn generate_normals(uc: &Context, mesh: *mut Mesh) -> Result<(), Fail> {
    let num_indices: usize = (*mesh).num_indices;

    (*mesh).generated_normals = true;

    let topo: *mut TopoEdge = push::<TopoEdge>(uc.tmp_stack_mut_ptr(), num_indices);
    ufbxi_check!(uc, !topo.is_null(), "topo");

    let normal_indices: *mut u32 = push::<u32>(uc.result_mut_ptr(), num_indices);
    ufbxi_check!(uc, !normal_indices.is_null(), "normal_indices");

    compute_topology(mesh, topo, num_indices);
    let num_normals: usize =
        generate_normal_mapping(mesh, topo, num_indices, normal_indices, num_indices, false);

    if num_normals == (*mesh).num_vertices {
        (*mesh).vertex_normal.unique_per_vertex = true;
    }

    let mut normal_data: *mut Vec3 = push::<Vec3>(uc.result_mut_ptr(), num_normals + 1);
    ufbxi_check!(uc, !normal_data.is_null(), "normal_data");

    // C: `normal_data[0] = ufbx_zero_vec3; normal_data++;`
    *normal_data = ZERO_VEC3;
    normal_data = normal_data.add(1);

    compute_normals(
        mesh,
        ptr::addr_of!((*mesh).vertex_position),
        normal_indices,
        num_indices,
        normal_data,
        num_normals,
    );

    (*mesh).vertex_normal.exists = true;
    (*mesh).vertex_normal.values.data = normal_data as *const Vec3;
    (*mesh).vertex_normal.values.count = num_normals;
    (*mesh).vertex_normal.indices.data = normal_indices as *const u32;
    (*mesh).vertex_normal.indices.count = num_indices;
    (*mesh).vertex_normal.value_reals = 3;

    // C: `mesh->skinned_normal = mesh->vertex_normal;` — struct assignment
    // (memcpy); `VertexVec3` is not `Copy` in the generated bindings, so the
    // copy is spelled as a byte-identical `copy_nonoverlapping`.
    ptr::copy_nonoverlapping(
        ptr::addr_of!((*mesh).vertex_normal),
        ptr::addr_of_mut!((*mesh).skinned_normal),
        1,
    );

    pop::<TopoEdge>(uc.tmp_stack_mut_ptr(), num_indices, ptr::null_mut());

    Ok(())
}

// ufbx.c:20405-20427 `ufbxi_push_prop_prefix`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_prop_prefix(
    uc: &Context,
    dst: *mut String,
    mut prefix: String,
) -> Result<(), Fail> {
    let mut stack_size: usize = 0;
    if prefix.length > 0 && *prefix.data.add(prefix.length - 1) != b'|' {
        stack_size = prefix.length + 1;
        let copy: *mut u8 = push(uc.tmp_stack_mut_ptr(), stack_size);
        ufbxi_check!(uc, !copy.is_null(), "copy");
        ptr::copy_nonoverlapping(prefix.data, copy, prefix.length);
        *copy.add(prefix.length) = b'|';

        prefix.data = copy;
        prefix.length += 1;
    }

    sp::push_string_place_str(uc.string_pool_mut_ptr(), &mut prefix, false)?;
    *dst = prefix;

    if stack_size > 0 {
        pop::<u8>(uc.tmp_stack_mut_ptr(), stack_size, ptr::null_mut());
    }

    Ok(())
}

// ufbx.c:20429-20478 `ufbxi_shader_texture_find_prefix`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn shader_texture_find_prefix(
    uc: &Context,
    texture: *mut Texture,
    shader: *mut ShaderTexture,
) -> Result<(), Fail> {
    // C: `ufbx_string suffixes[3];` — uninitialized local (no upstream
    // `// ufbxi_uninit` marker at ufbx.c:20431); only the first
    // `num_suffixes` entries are ever written, and only those are read.
    let mut suffixes_storage = MaybeUninit::<[String; 3]>::uninit();
    let suffixes: *mut String = suffixes_storage.as_mut_ptr() as *mut String;
    let mut num_suffixes: usize = 0;

    *suffixes.add(num_suffixes) = sp::str_c(b" Parameters/Connections\0".as_ptr());
    num_suffixes += 1;
    if (*shader).shader_name.length > 0 {
        *suffixes.add(num_suffixes) = (*shader).shader_name;
        num_suffixes += 1;
    }
    *suffixes.add(num_suffixes) = sp::str_c(b"3dsMax|parameters\0".as_ptr());
    num_suffixes += 1;

    // C: `ufbx_assert(num_suffixes <= ufbxi_arraycount(suffixes));`
    ufbx_assert!(num_suffixes <= 3);

    // C: `ufbxi_for(ufbx_string, p_suffix, suffixes, num_suffixes)`
    let mut p_suffix: *mut String = suffixes;
    let p_suffix_end: *mut String = add_ptr(p_suffix, num_suffixes);
    while p_suffix != p_suffix_end {
        let suffix: String = *p_suffix;

        // C: `ufbxi_for_list(ufbx_prop, prop, texture->props.props)`
        let mut prop: *mut Prop = (*texture).element.props.props.data as *mut Prop;
        let prop_end: *mut Prop = add_ptr(prop, (*texture).element.props.props.count);
        while prop != prop_end {
            if (*prop).type_ != PropType::Compound {
                prop = prop.add(1);
                continue;
            }
            if sp::ends_with((*prop).name, suffix) {
                push_prop_prefix(uc, &mut (*shader).prop_prefix, (*prop).name)?;
                return Ok(());
            }
            prop = prop.add(1);
        }
        p_suffix = p_suffix.add(1);
    }

    // Pre-7000 files don't have explicit Compound properties, so let's look for
    // any property that has the suffix before the last `|` ...
    let mut p_suffix: *mut String = suffixes;
    let p_suffix_end: *mut String = add_ptr(p_suffix, num_suffixes);
    while p_suffix != p_suffix_end {
        let suffix: String = *p_suffix;

        // C: `ufbxi_for_list(ufbx_prop, prop, texture->props.props)`
        let mut prop: *mut Prop = (*texture).element.props.props.data as *mut Prop;
        let prop_end: *mut Prop = add_ptr(prop, (*texture).element.props.props.count);
        while prop != prop_end {
            let mut name: String = (*prop).name;
            while name.length > 0 {
                if *name.data.add(name.length - 1) == b'|' {
                    break;
                }
                name.length -= 1;
            }
            if name.length <= 1 {
                prop = prop.add(1);
                continue;
            }
            name.length -= 1;

            if sp::ends_with(name, suffix) {
                push_prop_prefix(uc, &mut (*shader).prop_prefix, name)?;
                return Ok(());
            }
            prop = prop.add(1);
        }
        p_suffix = p_suffix.add(1);
    }

    Ok(())
}

// ufbx.c:20480-20484 `ufbxi_file_shader`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct FileShader {
    pub shader_id: u64,
    pub shader_name: *const u8,
    pub input_name: *const u8,
}
// The table below is immutable and its `const char *` members reference
// immutable string literals, so sharing is sound (same rationale as
// `ConstraintProp` above).
unsafe impl Sync for FileShader {}

// ufbx.c:20486-20494 `ufbxi_file_shaders`
// Known shaders that represent sampled images.
#[rustfmt::skip]
static FILE_SHADERS: [FileShader; 6] = [
    FileShader { shader_id: 0x7e73161fad53b12a, shader_name: b"ai_image\0".as_ptr(), input_name: b"filename\0".as_ptr() },
    FileShader { shader_id: 0, shader_name: b"OSLBitmap\0".as_ptr(), input_name: sp::Filename.as_ptr() },
    FileShader { shader_id: 0, shader_name: b"OSLBitmap2\0".as_ptr(), input_name: sp::Filename.as_ptr() },
    FileShader { shader_id: 0, shader_name: b"OSLBitmap3\0".as_ptr(), input_name: sp::Filename.as_ptr() },
    FileShader { shader_id: 0, shader_name: b"UberBitmap\0".as_ptr(), input_name: sp::Filename.as_ptr() },
    FileShader { shader_id: 0, shader_name: b"UberBitmap2\0".as_ptr(), input_name: sp::Filename.as_ptr() },
];

// ufbx.c:20496-20535 `ufbxi_update_shader_texture`
#[inline(never)]
pub(crate) unsafe fn update_shader_texture(texture: *mut Texture, shader: *mut ShaderTexture) {
    // C: `ufbxi_for_list(ufbx_shader_texture_input, input, shader->inputs)`
    let mut input: *mut ShaderTextureInput = (*shader).inputs.data as *mut ShaderTextureInput;
    let input_end: *mut ShaderTextureInput = add_ptr(input, (*shader).inputs.count);
    while input != input_end {
        // C: `ufbx_prop *prop = input->prop;`
        let mut prop: *mut Prop = opt_ptr(&(*input).prop);
        if !prop.is_null() {
            prop = find_prop_len(
                &(*texture).element.props,
                (*prop).name.data,
                (*prop).name.length,
            );
            (*input).prop = opt_ref(prop);
            (*input).value_vec4 = (*prop).value_vec4;
            (*input).value_int = (*prop).value_int;
            (*input).value_str = (*prop).value_str;
            (*input).value_blob = (*prop).value_blob;
            (*input).texture = opt_ref(get_prop_element(
                &(*texture).element,
                opt_ptr(&(*input).prop),
                ElementType::Texture,
            ) as *mut Texture);
        }

        prop = opt_ptr(&(*input).texture_prop);
        if !prop.is_null() {
            prop = find_prop_len(
                &(*texture).element.props,
                (*prop).name.data,
                (*prop).name.length,
            );
            (*input).texture_prop = opt_ref(prop);
            let tex: *mut Texture =
                get_prop_element(&(*texture).element, prop, ElementType::Texture) as *mut Texture;
            if !tex.is_null() {
                (*input).texture = opt_ref(tex);
            }
        }

        (*input).texture_enabled = !opt_ptr(&(*input).texture).is_null();
        prop = opt_ptr(&(*input).texture_enabled_prop);
        if !prop.is_null() {
            prop = find_prop_len(
                &(*texture).element.props,
                (*prop).name.data,
                (*prop).name.length,
            );
            (*input).texture_enabled_prop = opt_ref(prop);
            (*input).texture_enabled = (*prop).value_int != 0;
        }
        input = input.add(1);
    }

    if (*shader).type_ == ShaderTextureType::SelectOutput {
        let map: *mut ShaderTextureInput =
            find_shader_texture_input(shader, b"sourceMap\0".as_ptr());
        let index: *mut ShaderTextureInput =
            find_shader_texture_input(shader, b"outputChannelIndex\0".as_ptr());
        if !index.is_null() {
            (*shader).main_texture_output_index = (*index).value_int;
        }
        if !map.is_null() {
            (*shader).main_texture = (*map).texture;
            (*map).texture_output_index = (*shader).main_texture_output_index;
        }
    }
}

// Carrier for C's function-local `static const char *const ...[]` tables
// (ufbx.c:20563-20569): the arrays are immutable and reference immutable string
// literals, but a bare `*const u8` is not `Sync`, which a Rust `static`
// requires (same rationale as `ShaderMapping` above).
#[repr(transparent)]
#[derive(Clone, Copy)]
pub(crate) struct CharPtr(pub *const u8);
unsafe impl Sync for CharPtr {}

// ufbx.h:2772 `UFBX_ENUM_TYPE(ufbx_shader_texture_type, UFBX_SHADER_TEXTURE_TYPE, UFBX_SHADER_TEXTURE_OSL);`
// expanding via ufbx.h:235-236 to `enum { UFBX_SHADER_TEXTURE_TYPE_COUNT = UFBX_SHADER_TEXTURE_OSL + 1 }`.
pub(crate) const SHADER_TEXTURE_TYPE_COUNT: u32 = ShaderTextureType::Osl as u32 + 1;

// ufbx.c:20537-20690 `ufbxi_finalize_shader_texture`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn finalize_shader_texture(
    uc: &Context,
    texture: *mut Texture,
) -> Result<(), Fail> {
    let classid_a: u32 =
        api_find_int(&(*texture).element.props, b"3dsMax|ClassIDa\0".as_ptr(), 0) as u64 as u32;
    let classid_b: u32 =
        api_find_int(&(*texture).element.props, b"3dsMax|ClassIDb\0".as_ptr(), 0) as u64 as u32;
    let classid: u64 = (classid_a as u64) << 32 | classid_b as u64;

    let max_texture: String = find_string(
        &(*texture).element.props,
        b"3dsMax|MaxTexture\0".as_ptr(),
        EMPTY_STRING.0,
    );

    // Check first if the texture looks like it could be a shader.
    // C: `ufbx_shader_texture_type type = (ufbx_shader_texture_type)UFBX_SHADER_TEXTURE_TYPE_COUNT;`
    // — the sentinel is out of the enum's range, so it is carried as the raw
    // `uint32_t` C stores and only transmuted once the range check passed.
    let mut type_: u32 = SHADER_TEXTURE_TYPE_COUNT;

    if strcmp(max_texture.data, b"MULTIOUTPUT_TO_OSLMap\0".as_ptr()) == 0
        || classid == 0x896ef2fc44bd743f
    {
        type_ = ShaderTextureType::SelectOutput as u32;
    } else if strcmp(max_texture.data, b"OSLMap\0".as_ptr()) == 0 || classid == 0x7f9a7b9d6fcdf00d {
        type_ = ShaderTextureType::Osl as u32;
    } else if (*texture).type_ == TextureType::File
        && (*texture).relative_filename.length == 0
        && (*texture).absolute_filename.length == 0
        && opt_ptr(&(*texture).video).is_null()
    {
        type_ = ShaderTextureType::Unknown as u32;
    }

    if type_ == SHADER_TEXTURE_TYPE_COUNT {
        return Ok(());
    }

    let shader: *mut ShaderTexture = push_zero(uc.result_mut_ptr(), 1);
    ufbxi_check!(uc, !shader.is_null(), "shader");

    (*shader).type_ = core::mem::transmute::<u32, ShaderTextureType>(type_);

    // C: `static const char *const name_props[] = { "3dsMax|params|OSLShaderName" };`
    static NAME_PROPS: [CharPtr; 1] = [CharPtr(b"3dsMax|params|OSLShaderName\0".as_ptr())];

    // C: `static const char *const source_props[] = { "3dsMax|params|OSLCode" };`
    static SOURCE_PROPS: [CharPtr; 1] = [CharPtr(b"3dsMax|params|OSLCode\0".as_ptr())];

    (*shader).shader_source.data = EMPTY_CHAR.as_ptr();
    (*shader).shader_name.data = EMPTY_CHAR.as_ptr();

    // C: `ufbxi_nounroll for (size_t i = 0; i < ufbxi_arraycount(name_props); i++)`
    for i in 0..NAME_PROPS.len() {
        let prop: *mut Prop = api_find_prop(&(*texture).element.props, NAME_PROPS[i].0);
        if !prop.is_null() {
            (*shader).shader_name = (*prop).value_str;
            break;
        }
    }

    // C: `ufbxi_nounroll for (size_t i = 0; i < ufbxi_arraycount(source_props); i++)`
    for i in 0..SOURCE_PROPS.len() {
        let prop: *mut Prop = api_find_prop(&(*texture).element.props, SOURCE_PROPS[i].0);
        if !prop.is_null() {
            (*shader).shader_source = (*prop).value_str;
            (*shader).raw_shader_source = (*prop).value_blob;
            break;
        }
    }

    shader_texture_find_prefix(uc, texture, shader)?;

    if (*shader).shader_name.length == 0 {
        let mut name: String = (*shader).prop_prefix;
        if sp::remove_suffix_c(&mut name, b" Parameters/Connections|\0".as_ptr()) {
            let mut begin: usize = name.length;
            while begin > 0 && *name.data.add(begin - 1) != b'|' {
                begin -= 1;
            }

            (*shader).shader_name.data = name.data.add(begin);
            (*shader).shader_name.length = name.length - begin;
            sp::push_string_place_str(uc.string_pool_mut_ptr(), &mut (*shader).shader_name, false)?;
        }
    }

    if (*shader).shader_name.length == 0 {
        if max_texture.length > 0 {
            (*shader).shader_name = max_texture;
        }
    }

    if classid != 0 {
        (*shader).shader_type_id = classid;
    }

    if (*shader).prop_prefix.length == 0 {
        // If we not find any shader properties so we might have guessed wrong.
        // We "leak" (freed with scene) the shader in this case but it's negligible.
        return Ok(());
    }

    // C: `ufbxi_for_list(ufbx_prop, prop, texture->props.props)`
    let mut prop: *mut Prop = (*texture).element.props.props.data as *mut Prop;
    let prop_end: *mut Prop = add_ptr(prop, (*texture).element.props.props.count);
    while prop != prop_end {
        let mut name: String = (*prop).name;
        if !sp::remove_prefix_str(&mut name, (*shader).prop_prefix) {
            prop = prop.add(1);
            continue;
        }

        // Check if this property is a modifier to an existing input.
        let mut base_name: String = name;
        if sp::remove_suffix_c(&mut base_name, b"_map\0".as_ptr())
            || sp::remove_suffix_c(&mut base_name, b".shader\0".as_ptr())
        {
            let base: *mut ShaderTextureInput =
                find_shader_texture_input_len(shader, base_name.data, base_name.length);
            if !base.is_null() {
                (*base).texture_prop = opt_ref(prop);
                prop = prop.add(1);
                continue;
            }
        } else if sp::remove_suffix_c(&mut base_name, b".connected\0".as_ptr())
            || sp::remove_suffix_c(&mut base_name, b"Enabled\0".as_ptr())
        {
            let base: *mut ShaderTextureInput =
                find_shader_texture_input_len(shader, base_name.data, base_name.length);
            if !base.is_null() {
                (*base).texture_enabled_prop = opt_ref(prop);
                prop = prop.add(1);
                continue;
            }
        }

        // Use `uc->tmp_arr` to store the texture inputs so we can search them while we insert new ones.
        ufbxi_check!(
            uc,
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                (*shader)
                    .inputs
                    .count
                    .wrapping_add(1)
                    .wrapping_mul(size_of::<ShaderTextureInput>()),
            ),
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), ((shader->inputs.count + 1) * sizeof(ufbx_shader_texture_input)))"
        );
        (*shader).inputs.data = uc.tmp_arr() as *const ShaderTextureInput;

        // Add a new property
        // C: `ufbx_shader_texture_input *input = &shader->inputs.data[shader->inputs.count++];`
        let input: *mut ShaderTextureInput =
            ((*shader).inputs.data as *mut ShaderTextureInput).add((*shader).inputs.count);
        (*shader).inputs.count += 1;
        ptr::write_bytes(input, 0, 1);

        // NOTE: This is a bit hackish, we are using a suffix of an interned string. It won't compare
        // pointer equal to the same string but that shouldn't matter..
        (*input).name = name;

        // Connect the property only, values and textures etc are fetched in `ufbxi_update_shader_texture()`.
        (*input).prop = opt_ref(prop);

        prop = prop.add(1);
    }

    // Retain the shader inputs
    (*shader).inputs.data = push_copy::<ShaderTextureInput>(
        uc.result_mut_ptr(),
        (*shader).inputs.count,
        (*shader).inputs.data,
    );
    ufbxi_check!(uc, !(*shader).inputs.data.is_null(), "shader->inputs.data");

    (*texture).shader = opt_ref(shader);
    (*texture).type_ = TextureType::Shader;
    (*uc.get()).scene.metadata.num_shader_textures += 1;

    if !(*uc.get()).opts.disable_quirks {
        // C: `ufbxi_nounroll for (size_t i = 0; i < ufbxi_arraycount(ufbxi_file_shaders); i++)`
        for i in 0..FILE_SHADERS.len() {
            let fs: *const FileShader = &FILE_SHADERS[i];

            if ((*fs).shader_id != 0 && (*shader).shader_type_id == (*fs).shader_id)
                || strcmp((*shader).shader_name.data, (*fs).shader_name) == 0
            {
                let input: *mut ShaderTextureInput =
                    find_shader_texture_input(shader, (*fs).input_name);
                if !input.is_null() {
                    // TODO: Support for specifying relative filename here if ever needed
                    let prop: *mut Prop = opt_ptr(&(*input).prop);
                    (*texture).absolute_filename = (*prop).value_str;
                    (*texture).raw_absolute_filename = (*prop).value_blob;
                    (*texture).type_ = TextureType::File;
                    break;
                }
            }
        }
    }

    update_shader_texture(texture, shader);

    Ok(())
}

// ufbx.c:20692-20752 `ufbxi_propagate_main_textures`
#[inline(never)]
pub(crate) unsafe fn propagate_main_textures(scene: *mut Scene) {
    // We need to do at least 2^(N-1) passes for N shader textures
    let mut mask: usize = (*scene).metadata.num_shader_textures;
    while mask != 0 {
        mask >>= 1;

        // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, scene->textures)`
        let mut p_texture: *mut *mut Texture = (*scene).textures.data as *mut *mut Texture;
        let p_texture_end: *mut *mut Texture = add_ptr(p_texture, (*scene).textures.count);
        while p_texture != p_texture_end {
            let texture: *mut Texture = *p_texture;
            let shader: *mut ShaderTexture = opt_ptr(&(*texture).shader);
            if shader.is_null() {
                p_texture = p_texture.add(1);
                continue;
            }

            let main_tex: *mut Texture = opt_ptr(&(*shader).main_texture);
            if main_tex.is_null() || (*shader).main_texture_output_index != 0 {
                p_texture = p_texture.add(1);
                continue;
            }

            let main_shader: *mut ShaderTexture = opt_ptr(&(*main_tex).shader);
            if main_shader.is_null() || opt_ptr(&(*main_shader).main_texture).is_null() {
                p_texture = p_texture.add(1);
                continue;
            }

            (*shader).main_texture = (*main_shader).main_texture;
            (*shader).main_texture_output_index = (*main_shader).main_texture_output_index;

            p_texture = p_texture.add(1);
        }
    }

    // Remove cyclic main textures
    // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, scene->textures)`
    let mut p_texture: *mut *mut Texture = (*scene).textures.data as *mut *mut Texture;
    let p_texture_end: *mut *mut Texture = add_ptr(p_texture, (*scene).textures.count);
    while p_texture != p_texture_end {
        let texture: *mut Texture = *p_texture;
        let shader: *mut ShaderTexture = opt_ptr(&(*texture).shader);
        if shader.is_null()
            || opt_ptr(&(*shader).main_texture).is_null()
            || (*shader).main_texture_output_index != 0
        {
            p_texture = p_texture.add(1);
            continue;
        }
        let main_tex: *mut Texture = opt_ptr(&(*shader).main_texture);
        if !main_tex.is_null()
            && !opt_ptr(&(*main_tex).shader).is_null()
            && !opt_ptr(&(*opt_ptr(&(*main_tex).shader)).main_texture).is_null()
        {
            // Should have been propagated to `texture`
            (*shader).main_texture = None;
        }
        p_texture = p_texture.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, scene->textures)`
    let mut p_texture: *mut *mut Texture = (*scene).textures.data as *mut *mut Texture;
    let p_texture_end: *mut *mut Texture = add_ptr(p_texture, (*scene).textures.count);
    while p_texture != p_texture_end {
        let texture: *mut Texture = *p_texture;
        let shader: *mut ShaderTexture = opt_ptr(&(*texture).shader);
        if shader.is_null() {
            p_texture = p_texture.add(1);
            continue;
        }

        // C: `ufbxi_for_list(ufbx_shader_texture_input, input, shader->inputs)`
        let mut input: *mut ShaderTextureInput = (*shader).inputs.data as *mut ShaderTextureInput;
        let input_end: *mut ShaderTextureInput = add_ptr(input, (*shader).inputs.count);
        while input != input_end {
            let input_texture: *mut Texture = opt_ptr(&(*input).texture);
            if input_texture.is_null() || opt_ptr(&(*input_texture).shader).is_null() {
                input = input.add(1);
                continue;
            }
            let input_shader: *mut ShaderTexture = opt_ptr(&(*input_texture).shader);
            if !opt_ptr(&(*input_shader).main_texture).is_null() {
                (*input).texture = (*input_shader).main_texture;
                (*input).texture_output_index = (*input_shader).main_texture_output_index;
            }
            input = input.add(1);
        }

        p_texture = p_texture.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_material, p_material, scene->materials)`
    let mut p_material: *mut *mut Material = (*scene).materials.data as *mut *mut Material;
    let p_material_end: *mut *mut Material = add_ptr(p_material, (*scene).materials.count);
    while p_material != p_material_end {
        let material: *mut Material = *p_material;

        // C: `ufbxi_for_list(ufbx_material_texture, tex, material->textures)`
        let mut tex: *mut MaterialTexture = (*material).textures.data as *mut MaterialTexture;
        let tex_end: *mut MaterialTexture = add_ptr(tex, (*material).textures.count);
        while tex != tex_end {
            let shader: *mut ShaderTexture = opt_ptr(&(*ref_ptr(&(*tex).texture)).shader);
            if !shader.is_null()
                && !opt_ptr(&(*shader).main_texture).is_null()
                && (*shader).main_texture_output_index == 0
            {
                // C: `tex->texture = shader->main_texture;` — `main_texture` is
                // null-checked non-NULL just above, so the non-nullable
                // `ufbx_material_texture.texture` stays valid.
                (*tex).texture = Ref::from_ptr(opt_ptr(&(*shader).main_texture));
            }
            tex = tex.add(1);
        }

        p_material = p_material.add(1);
    }
}

// ufbx.c:20754-20755 `#define ufbxi_patch_empty(m_dst, m_len, m_src)`
macro_rules! patch_empty {
    ($dst:expr, $len:ident, $src:expr) => {{
        if $dst.$len == 0 {
            $dst = $src;
        }
    }};
}

// ufbx.c:20757-20800 `ufbxi_insert_texture_file`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn insert_texture_file(uc: &Context, texture: *mut Texture) -> Result<(), Fail> {
    (*texture).file_index = NO_INDEX;

    let mut key: *const u8 = ptr::null();

    // HACK: Even the raw entries have a null terminator so we can offset the
    // pointer by one for relative filenames. This guarantees that an overlapping
    // absolute and relative filenames will get separate textures.
    if (*texture).raw_absolute_filename.size > 0 {
        key = (*texture).raw_absolute_filename.data;
    } else if (*texture).raw_relative_filename.size > 0 {
        key = (*texture).raw_relative_filename.data.add(1);
    }

    if key.is_null() {
        return Ok(());
    }
    let hash: u32 = hash_ptr!(key);
    let mut entry: *mut TextureFileEntry = map_find(
        &mut (*uc.get()).texture_file_map,
        hash,
        &key as *const *const u8 as *const c_void,
    );
    if entry.is_null() {
        entry = map_insert(
            &mut (*uc.get()).texture_file_map,
            hash,
            &key as *const *const u8 as *const c_void,
        );
        ufbxi_check!(uc, !entry.is_null(), "entry");

        let file: *mut TextureFile = push_zero(uc.tmp_mut_ptr(), 1);
        ufbxi_check!(uc, !file.is_null(), "file");

        (*file).index = (*uc.get()).texture_file_map.size - 1;

        (*entry).key = key;
        (*entry).file = file;
    }

    let file: *mut TextureFile = (*entry).file;
    (*texture).file_index = (*file).index;
    (*texture).has_file = true;
    patch_empty!((*file).filename, length, (*texture).filename);
    patch_empty!(
        (*file).relative_filename,
        length,
        (*texture).relative_filename
    );
    patch_empty!(
        (*file).absolute_filename,
        length,
        (*texture).absolute_filename
    );
    patch_empty!((*file).raw_filename, size, (*texture).raw_filename);
    patch_empty!(
        (*file).raw_relative_filename,
        size,
        (*texture).raw_relative_filename
    );
    patch_empty!(
        (*file).raw_absolute_filename,
        size,
        (*texture).raw_absolute_filename
    );
    patch_empty!((*file).content, size, (*texture).content);

    Ok(())
}

// ufbx.c:20802-20817 `ufbxi_pop_texture_files`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn pop_texture_files(uc: &Context) -> Result<(), Fail> {
    let num_files: u32 = (*uc.get()).texture_file_map.size;
    let files: *mut TextureFile = push(uc.result_mut_ptr(), num_files as usize);
    ufbxi_check!(uc, !files.is_null(), "files");

    (*uc.get()).scene.texture_files.data = files;
    (*uc.get()).scene.texture_files.count = num_files as usize;

    let entries: *mut TextureFileEntry =
        (*uc.get()).texture_file_map.items as *mut TextureFileEntry;
    for i in 0..num_files as usize {
        ptr::copy_nonoverlapping((*entries.add(i)).file, files.add(i), 1);
    }

    Ok(())
}

// ufbx.c:20819-20822 `ufbxi_ordered_texture`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct OrderedTexture {
    pub texture: *mut Texture,
    pub order: usize,
}

// ufbx.c:20824-20829 `ufbxi_ordered_texture_less_texture`
#[inline(never)]
pub(crate) unsafe extern "C" fn ordered_texture_less_texture(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const OrderedTexture = va as *const OrderedTexture;
    let b: *const OrderedTexture = vb as *const OrderedTexture;
    (*a).texture < (*b).texture
}

// ufbx.c:20831-20836 `ufbxi_ordered_texture_less_order`
#[inline(never)]
pub(crate) unsafe extern "C" fn ordered_texture_less_order(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const OrderedTexture = va as *const OrderedTexture;
    let b: *const OrderedTexture = vb as *const OrderedTexture;
    (*a).order < (*b).order
}

// ufbx.c:20838-20867 `ufbxi_deduplicate_textures`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn deduplicate_textures(
    uc: &Context,
    dst_buf: *mut Buf,
    p_dst: *mut *mut OrderedTexture,
    p_dst_count: *mut usize,
    count: usize,
) -> Result<(), Fail> {
    let textures: *mut OrderedTexture = push_pop(dst_buf, uc.tmp_stack_mut_ptr(), count);
    ufbxi_check!(uc, !textures.is_null(), "textures");

    ufbxi_check!(
        uc,
        grow_array::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.tmp_arr_mut_ptr(),
            uc.tmp_arr_size_mut_ptr(),
            count.wrapping_mul(size_of::<OrderedTexture>()),
        ),
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbxi_ordered_texture)))"
    );

    stable_sort(
        size_of::<OrderedTexture>(),
        16,
        textures as *mut c_void,
        uc.tmp_arr() as *mut c_void,
        count,
        ordered_texture_less_texture,
        ptr::null_mut(),
    );

    // Remove adjacent duplicates
    let mut dst_ix: usize = 0;
    for src_ix in 0..count {
        if src_ix > 0 && (*textures.add(src_ix - 1)).texture == (*textures.add(src_ix)).texture {
            continue;
        } else {
            if src_ix != dst_ix {
                *textures.add(dst_ix) = *textures.add(src_ix);
            }
            dst_ix += 1;
        }
    }

    let new_count: usize = dst_ix;
    stable_sort(
        size_of::<OrderedTexture>(),
        16,
        textures as *mut c_void,
        uc.tmp_arr() as *mut c_void,
        new_count,
        ordered_texture_less_order,
        ptr::null_mut(),
    );

    *p_dst_count = new_count;
    *p_dst = textures;

    Ok(())
}

// ufbx.c:20869-20873 `ufbxi_file_texture_fetch_state`
// C-parity: the states are stored "compressed" into a `uint8_t` array and read
// back through an explicit `(ufbxi_file_texture_fetch_state)` cast, so the port
// keeps plain integer constants and compares the widened byte rather than
// introducing a Rust `enum` (which would need a fallible transmute the C never
// performs).
// C-parity: `UFBXI_FILE_TEXTURE_FETCH_INITIAL` (ufbx.c:20870) is the implicit
// zero state of the `states` byte array and is never named in ufbx.c either.
#[allow(dead_code)]
pub(crate) const FILE_TEXTURE_FETCH_INITIAL: u32 = 0;
pub(crate) const FILE_TEXTURE_FETCH_STARTED: u32 = 1;
pub(crate) const FILE_TEXTURE_FETCH_FINISHED: u32 = 2;

// Populate `ufbx_texture.file_textures[]` arrays.
// ufbx.c:20875-21007 `ufbxi_fetch_file_textures`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_file_textures(uc: &Context) -> Result<(), Fail> {
    // We keep pointers to `ufbx_texture` in `tmp_stack` as a working set, since we don't know
    // how deep the shader graphs might be.

    // Start by pushing all the textures into the stack
    let mut num_stack_textures: usize = (*uc.get()).scene.textures.count;
    ufbxi_check!(
        uc,
        !push_copy::<*mut Texture>(
            uc.tmp_stack_mut_ptr(),
            num_stack_textures,
            (*uc.get()).scene.textures.data as *const *mut Texture,
        )
        .is_null(),
        "((ufbx_texture**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_texture*), (num_stack_textures), (uc->scene.textures.data)))"
    );

    // Compressed `ufbxi_file_texture_fetch_state`
    let states: *mut u8 = push_zero(uc.tmp_mut_ptr(), (*uc.get()).scene.textures.count);
    ufbxi_check!(uc, !states.is_null(), "states");

    // C: `while (num_stack_textures-- > 0)` — the post-decrement runs on every
    // evaluation of the condition, including the final failing one, so the
    // counter wraps to `SIZE_MAX` on the way out (dead after the loop).
    loop {
        let loop_cond: bool = num_stack_textures > 0;
        num_stack_textures = num_stack_textures.wrapping_sub(1);
        if !loop_cond {
            break;
        }

        let mut texture: *mut Texture = ptr::null_mut();
        pop::<*mut Texture>(uc.tmp_stack_mut_ptr(), 1, &mut texture);

        let state: u32 = *states.add((*texture).element.typed_id as usize) as u32;
        if state == FILE_TEXTURE_FETCH_FINISHED {
            continue;
        }
        let shader: *mut ShaderTexture = opt_ptr(&(*texture).shader);

        if state == FILE_TEXTURE_FETCH_STARTED {
            *states.add((*texture).element.typed_id as usize) = FILE_TEXTURE_FETCH_FINISHED as u8;

            // HACK: Reuse `tmp_parse` for storing intermediate information as we can clear it.
            buf_clear(uc.tmp_parse_mut_ptr());

            // Now all non-cyclical dependents should be processed.
            let mut num_deps: usize = 0;

            if (*texture).type_ == TextureType::File {
                let dst: *mut OrderedTexture = push(uc.tmp_stack_mut_ptr(), 1);
                ufbxi_check!(uc, !dst.is_null(), "dst");
                (*dst).texture = texture;
                (*dst).order = num_deps;
                num_deps += 1;
            }

            // C: `ufbxi_for_list(ufbx_texture_layer, layer, texture->layers)`
            let mut layer: *mut TextureLayer = (*texture).layers.data as *mut TextureLayer;
            let layer_end: *mut TextureLayer = add_ptr(layer, (*texture).layers.count);
            while layer != layer_end {
                let dep_tex: *mut Texture = ref_ptr(&(*layer).texture);
                if (*dep_tex).file_textures.count > 0 {
                    let dst: *mut OrderedTexture = push(uc.tmp_stack_mut_ptr(), 1);
                    ufbxi_check!(uc, !dst.is_null(), "dst");
                    (*dst).texture = dep_tex;
                    (*dst).order = num_deps;
                    num_deps += 1;
                }
                layer = layer.add(1);
            }

            if !shader.is_null() {
                // C: `ufbxi_for_list(ufbx_shader_texture_input, input, shader->inputs)`
                let mut input: *mut ShaderTextureInput =
                    (*shader).inputs.data as *mut ShaderTextureInput;
                let input_end: *mut ShaderTextureInput = add_ptr(input, (*shader).inputs.count);
                while input != input_end {
                    let dep_tex: *mut Texture = opt_ptr(&(*input).texture);
                    if !dep_tex.is_null() && (*dep_tex).file_textures.count > 0 {
                        let dst: *mut OrderedTexture = push(uc.tmp_stack_mut_ptr(), 1);
                        ufbxi_check!(uc, !dst.is_null(), "dst");
                        (*dst).texture = dep_tex;
                        (*dst).order = num_deps;
                        num_deps += 1;
                    }
                    input = input.add(1);
                }
            }

            // Deduplicate the direct dependencies first
            // C: `ufbxi_ordered_texture *deps;` — `ufbxi_deduplicate_textures`
            // writes it before the first read (no `// ufbxi_uninit` marker
            // upstream), so the port keeps it genuinely uninitialized.
            let mut deps: MaybeUninit<*mut OrderedTexture> = MaybeUninit::uninit();
            deduplicate_textures(
                uc,
                uc.tmp_parse_mut_ptr(),
                deps.as_mut_ptr(),
                &mut num_deps,
                num_deps,
            )?;
            let deps: *mut OrderedTexture = deps.assume_init();

            if num_deps == 1 {
                // If we have only a single dependency (that is not the same one) we can just copy the pointer
                // C: struct assignment is a memcpy; `RefList<T>` is not `Copy`,
                // so the two ABI fields are copied individually (and the source
                // may alias the destination when a texture depends on itself,
                // which rules out `copy_nonoverlapping`).
                let src: *const RefList<Texture> =
                    ptr::addr_of!((*(*deps.add(0)).texture).file_textures);
                (*texture).file_textures.data = (*src).data;
                (*texture).file_textures.count = (*src).count;
            } else {
                // Now collect all the file textures and deduplicate them
                let mut num_files: usize = 0;
                // C: `ufbxi_for(ufbxi_ordered_texture, dep, deps, num_deps)`
                let mut dep: *mut OrderedTexture = deps;
                let dep_end: *mut OrderedTexture = add_ptr(dep, num_deps);
                while dep != dep_end {
                    // C: `ufbxi_for_ptr_list(ufbx_texture, p_tex, dep->texture->file_textures)`
                    let mut p_tex: *mut *mut Texture =
                        (*(*dep).texture).file_textures.data as *mut *mut Texture;
                    let p_tex_end: *mut *mut Texture =
                        add_ptr(p_tex, (*(*dep).texture).file_textures.count);
                    while p_tex != p_tex_end {
                        let dst: *mut OrderedTexture = push(uc.tmp_stack_mut_ptr(), 1);
                        ufbxi_check!(uc, !dst.is_null(), "dst");
                        (*dst).texture = *p_tex;
                        (*dst).order = num_files;
                        num_files += 1;
                        p_tex = p_tex.add(1);
                    }
                    dep = dep.add(1);
                }

                // Deduplicate the file textures
                let mut files: MaybeUninit<*mut OrderedTexture> = MaybeUninit::uninit();
                deduplicate_textures(
                    uc,
                    uc.tmp_parse_mut_ptr(),
                    files.as_mut_ptr(),
                    &mut num_files,
                    num_files,
                )?;
                let files: *mut OrderedTexture = files.assume_init();

                (*texture).file_textures.count = num_files;
                (*texture).file_textures.data =
                    push::<*mut Texture>(uc.result_mut_ptr(), num_files) as *const Ref<Texture>;
                ufbxi_check!(
                    uc,
                    !(*texture).file_textures.data.is_null(),
                    "texture->file_textures.data"
                );

                for i in 0..num_files {
                    *((*texture).file_textures.data as *mut *mut Texture).add(i) =
                        (*files.add(i)).texture;
                }
            }
        } else {
            if (*texture).type_ == TextureType::File {
                // Simple case: Just point to self
                (*texture).file_textures.count = 1;
                (*texture).file_textures.data =
                    push::<*mut Texture>(uc.result_mut_ptr(), 1) as *const Ref<Texture>;
                ufbxi_check!(
                    uc,
                    !(*texture).file_textures.data.is_null(),
                    "texture->file_textures.data"
                );
                *((*texture).file_textures.data as *mut *mut Texture).add(0) = texture;

                // In simple cases we can quit here, for more complex file textures queue
                // the texture in case there are other file textures as inputs.
                if opt_ptr(&(*texture).shader).is_null() {
                    *states.add((*texture).element.typed_id as usize) =
                        FILE_TEXTURE_FETCH_FINISHED as u8;
                    continue;
                }
            }

            // Complex: Process all dependencies first
            *states.add((*texture).element.typed_id as usize) = FILE_TEXTURE_FETCH_STARTED as u8;

            // Push self first so we can return after processing dependencies
            ufbxi_check!(
                uc,
                !push_copy::<*mut Texture>(uc.tmp_stack_mut_ptr(), 1, &texture).is_null(),
                "((ufbx_texture**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_texture*), (1), (&texture)))"
            );
            num_stack_textures += 1;

            // C: `ufbxi_for_list(ufbx_texture_layer, layer, texture->layers)`
            let mut layer: *mut TextureLayer = (*texture).layers.data as *mut TextureLayer;
            let layer_end: *mut TextureLayer = add_ptr(layer, (*texture).layers.count);
            while layer != layer_end {
                ufbxi_check!(
                    uc,
                    !push_copy::<*mut Texture>(
                        uc.tmp_stack_mut_ptr(),
                        1,
                        &(*layer).texture as *const Ref<Texture> as *const *mut Texture,
                    )
                    .is_null(),
                    "((ufbx_texture**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_texture*), (1), (&layer->texture)))"
                );
                num_stack_textures += 1;
                layer = layer.add(1);
            }

            if !shader.is_null() {
                // C: `ufbxi_for_list(ufbx_shader_texture_input, input, shader->inputs)`
                let mut input: *mut ShaderTextureInput =
                    (*shader).inputs.data as *mut ShaderTextureInput;
                let input_end: *mut ShaderTextureInput = add_ptr(input, (*shader).inputs.count);
                while input != input_end {
                    if !opt_ptr(&(*input).texture).is_null() {
                        ufbxi_check!(
                            uc,
                            !push_copy::<*mut Texture>(
                                uc.tmp_stack_mut_ptr(),
                                1,
                                &(*input).texture as *const Option<Ref<Texture>>
                                    as *const *mut Texture,
                            )
                            .is_null(),
                            "((ufbx_texture**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_texture*), (1), (&input->texture)))"
                        );
                        num_stack_textures += 1;
                    }
                    input = input.add(1);
                }
            }
        }
    }

    Ok(())
}

// ufbx.c:21009-21016 `ufbxi_get_geometry_transform_node`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn get_geometry_transform_node(element: *mut Element) -> *mut Node {
    if (*element).instances.count == 1 {
        let node: *mut Node = ref_ptr((*element).instances.data.add(0));
        if (*node).has_geometry_transform {
            return node;
        }
    }
    ptr::null_mut()
}

// ufbx.c:21018-21031 `ufbxi_mirror_vec3_list`
#[inline(never)]
pub(crate) unsafe fn mirror_vec3_list(v_list: *const c_void, axis: MirrorAxis, stride: usize) {
    let mut stride: usize = stride;
    let list: *const VoidList = v_list as *const VoidList;
    if axis == MirrorAxis::None || list.is_null() || (*list).count == 0 {
        return;
    }
    if stride == 0 {
        stride = size_of::<Vec3>();
    }

    // C: `(char*)list->data + (size_t)((int)axis - 1) * sizeof(ufbx_real)` —
    // the enum is narrowed to `int` before the subtraction, and `axis` is
    // 1..=3 here because `UFBX_MIRROR_AXIS_NONE` returned above.
    let mut p: *mut u8 = ((*list).data as *mut u8)
        .wrapping_add(((axis as i32 - 1) as usize).wrapping_mul(size_of::<Real>()));
    let end: *mut u8 = p.wrapping_add((*list).count.wrapping_mul(stride));
    while p != end {
        let v: *mut Real = p as *mut Real;
        *v = -*v;
        p = p.wrapping_add(stride);
    }
}

// ufbx.c:21033-21047 `ufbxi_scale_vec3_list`
#[inline(never)]
pub(crate) unsafe fn scale_vec3_list(v_list: *const c_void, scale: Real, stride: usize) {
    let mut stride: usize = stride;
    let list: *const VoidList = v_list as *const VoidList;
    if list.is_null() || (*list).count == 0 {
        return;
    }
    if stride == 0 {
        stride = size_of::<Vec3>();
    }

    let mut p: *mut u8 = (*list).data as *mut u8;
    let end: *mut u8 = p.wrapping_add((*list).count.wrapping_mul(stride));
    while p != end {
        let v: *mut Vec3 = p as *mut Vec3;
        (*v).x *= scale;
        (*v).y *= scale;
        (*v).z *= scale;
        p = p.wrapping_add(stride);
    }
}

// ufbx.c:21049-21061 `ufbxi_transform_vec3_list`
#[inline(never)]
pub(crate) unsafe fn transform_vec3_list(
    v_list: *const c_void,
    matrix: *const Matrix,
    stride: usize,
) {
    let mut stride: usize = stride;
    let list: *const VoidList = v_list as *const VoidList;
    if list.is_null() || (*list).count == 0 {
        return;
    }
    if stride == 0 {
        stride = size_of::<Vec3>();
    }

    let mut p: *mut u8 = (*list).data as *mut u8;
    let end: *mut u8 = p.wrapping_add((*list).count.wrapping_mul(stride));
    while p != end {
        let v: *mut Vec3 = p as *mut Vec3;
        *v = transform_position(matrix, *v);
        p = p.wrapping_add(stride);
    }
}

// ufbx.c:21063-21068 `ufbxi_normalize_vec3_list`
#[inline(never)]
pub(crate) unsafe fn normalize_vec3_list(list: *const List<Vec3>) {
    // C: `ufbxi_nounroll ufbxi_for_list(ufbx_vec3, normal, *list)` — the
    // no-unroll pragma is optimizer-only and has no Rust analogue.
    let mut normal: *mut Vec3 = (*list).data as *mut Vec3;
    let normal_end: *mut Vec3 = add_ptr(normal, (*list).count);
    while normal != normal_end {
        *normal = normalize3(*normal);
        normal = normal.add(1);
    }
}

// ufbx.c:21070-21071 forward declaration of `ufbxi_get_geometry_transform`
// (defined at ufbx.c:22758-22784, in the `// -- Updating state from
// properties` banner that this same module owns; ported below at its C-order
// slot). The declaration exists in C only so that `ufbxi_modify_geometry`
// (ufbx.c:21165-21332, ported below) can call it; Rust needs no forward
// declaration, so this comment is the whole port of it.
// C comment: `// Forward declare as we're kind of preprocessing ata here that
// would usually happen later.`

// ufbx.c:21073-21107 `ufbxi_flip_attrib_winding`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn flip_attrib_winding(
    uc: &Context,
    mesh: *mut Mesh,
    indices: *mut List<u32>,
    is_position: bool,
) -> Result<(), Fail> {
    // All zero, no flipping needed
    if (*indices).data == uc.zero_indices() || (*indices).count == 0 {
        return Ok(());
    }

    if (*indices).data == (*mesh).vertex_position.indices.data && !is_position {
        // Sharing indices with vertex position, already flipped.
        return Ok(());
    } else if (*indices).data == uc.consecutive_indices() {
        // Need to duplicate consecutive indices, but we can cache the per mesh.
        if !uc.tmp_mesh_consecutive_indices().is_null() {
            (*indices).data = uc.tmp_mesh_consecutive_indices();
            return Ok(());
        }
        (*indices).data = push_copy::<u32>(uc.result_mut_ptr(), (*indices).count, (*indices).data);
        ufbxi_check!(uc, !(*indices).data.is_null(), "indices->data");
        uc.set_tmp_mesh_consecutive_indices((*indices).data as *mut u32);
    }

    let data: *mut u32 = (*indices).data as *mut u32;
    // C: `ufbxi_for_list(ufbx_face, face, mesh->faces)`
    let mut face: *mut Face = (*mesh).faces.data as *mut Face;
    let face_end: *mut Face = add_ptr(face, (*mesh).faces.count);
    while face != face_end {
        if (*face).num_indices == 0 {
            face = face.add(1);
            continue;
        }
        // C: both sums are `unsigned int` arithmetic (wrapping) before the
        // widening to `size_t`.
        let mut begin: usize = (*face).index_begin.wrapping_add(1) as usize;
        let mut end: usize = (*face)
            .index_begin
            .wrapping_add((*face).num_indices)
            .wrapping_sub(1) as usize;
        while begin < end {
            let tmp: u32 = *data.add(begin);
            *data.add(begin) = *data.add(end);
            *data.add(end) = tmp;
            begin += 1;
            end -= 1;
        }
        face = face.add(1);
    }

    Ok(())
}

// ufbx.c:21109-21163 `ufbxi_flip_winding`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn flip_winding(uc: &Context, mesh: *mut Mesh) -> Result<(), Fail> {
    uc.set_tmp_mesh_consecutive_indices(ptr::null_mut());
    flip_attrib_winding(
        uc,
        mesh,
        ptr::addr_of_mut!((*mesh).vertex_position.indices),
        true,
    )?;
    flip_attrib_winding(
        uc,
        mesh,
        ptr::addr_of_mut!((*mesh).vertex_normal.indices),
        false,
    )?;
    flip_attrib_winding(
        uc,
        mesh,
        ptr::addr_of_mut!((*mesh).vertex_crease.indices),
        false,
    )?;
    if (*mesh).uv_sets.count > 0 {
        // C: `ufbxi_for_list(ufbx_uv_set, set, mesh->uv_sets)`
        let mut set: *mut UvSet = (*mesh).uv_sets.data as *mut UvSet;
        let set_end: *mut UvSet = add_ptr(set, (*mesh).uv_sets.count);
        while set != set_end {
            flip_attrib_winding(uc, mesh, ptr::addr_of_mut!((*set).vertex_uv.indices), false)?;
            flip_attrib_winding(
                uc,
                mesh,
                ptr::addr_of_mut!((*set).vertex_tangent.indices),
                false,
            )?;
            flip_attrib_winding(
                uc,
                mesh,
                ptr::addr_of_mut!((*set).vertex_bitangent.indices),
                false,
            )?;
            set = set.add(1);
        }
        // C: struct assignment (memcpy) of the vertex-attribute headers; the
        // `Vertex*` structs are not `Copy` in the generated bindings, so the
        // copy is spelled as a byte-identical `copy_nonoverlapping`.
        ptr::copy_nonoverlapping(
            ptr::addr_of!((*((*mesh).uv_sets.data as *mut UvSet).add(0)).vertex_uv),
            ptr::addr_of_mut!((*mesh).vertex_uv),
            1,
        );
        ptr::copy_nonoverlapping(
            ptr::addr_of!((*((*mesh).uv_sets.data as *mut UvSet).add(0)).vertex_bitangent),
            ptr::addr_of_mut!((*mesh).vertex_bitangent),
            1,
        );
        ptr::copy_nonoverlapping(
            ptr::addr_of!((*((*mesh).uv_sets.data as *mut UvSet).add(0)).vertex_tangent),
            ptr::addr_of_mut!((*mesh).vertex_tangent),
            1,
        );
    }
    if (*mesh).color_sets.count > 0 {
        // C: `ufbxi_for_list(ufbx_color_set, set, mesh->color_sets)`
        let mut set: *mut ColorSet = (*mesh).color_sets.data as *mut ColorSet;
        let set_end: *mut ColorSet = add_ptr(set, (*mesh).color_sets.count);
        while set != set_end {
            flip_attrib_winding(
                uc,
                mesh,
                ptr::addr_of_mut!((*set).vertex_color.indices),
                false,
            )?;
            set = set.add(1);
        }
        ptr::copy_nonoverlapping(
            ptr::addr_of!((*((*mesh).color_sets.data as *mut ColorSet).add(0)).vertex_color),
            ptr::addr_of_mut!((*mesh).vertex_color),
            1,
        );
    }
    flip_attrib_winding(
        uc,
        mesh,
        ptr::addr_of_mut!((*mesh).skinned_position.indices),
        false,
    )?;
    if (*mesh).skinned_normal.indices.data != (*mesh).vertex_normal.indices.data {
        flip_attrib_winding(
            uc,
            mesh,
            ptr::addr_of_mut!((*mesh).skinned_normal.indices),
            false,
        )?;
    }

    update_vertex_first_index(mesh);

    // Mapping from old index values to flipped ones, reserve index -1
    // (aka `UFBX_NO_INDEX`) for itself.
    if (*mesh).edges.count > 0 {
        ufbxi_check!(
            uc,
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                (*mesh).num_indices.wrapping_add(1).wrapping_mul(size_of::<u32>()),
            ),
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), ((mesh->num_indices + 1) * sizeof(uint32_t)))"
        );
        let index_mapping: *mut u32 = (uc.tmp_arr() as *mut u32).add(1);
        *index_mapping.offset(-1) = NO_INDEX;
        // C: `ufbxi_for_list(ufbx_face, face, mesh->faces)`
        let mut face: *mut Face = (*mesh).faces.data as *mut Face;
        let face_end: *mut Face = add_ptr(face, (*mesh).faces.count);
        while face != face_end {
            if (*face).num_indices == 0 {
                face = face.add(1);
                continue;
            }
            let begin: u32 = (*face).index_begin;
            let count: u32 = (*face).num_indices.wrapping_sub(1);
            *index_mapping.add(begin as usize) = begin;
            let mut i: u32 = 0;
            while i < count {
                *index_mapping.add(begin.wrapping_add(1).wrapping_add(i) as usize) =
                    begin.wrapping_add(count).wrapping_sub(i);
                i += 1;
            }
            face = face.add(1);
        }

        // C: `ufbxi_for_list(ufbx_edge, p_edge, mesh->edges)`
        let mut p_edge: *mut Edge = (*mesh).edges.data as *mut Edge;
        let p_edge_end: *mut Edge = add_ptr(p_edge, (*mesh).edges.count);
        while p_edge != p_edge_end {
            // C-parity: the `(int32_t)` casts are load-bearing — a
            // `UFBX_NO_INDEX` endpoint indexes `index_mapping[-1]`, the slot
            // reserved above.
            let a: u32 = *index_mapping.offset((*p_edge).a as i32 as isize);
            let b: u32 = *index_mapping.offset((*p_edge).b as i32 as isize);
            (*p_edge).a = b;
            (*p_edge).b = a;
            p_edge = p_edge.add(1);
        }
    }

    Ok(())
}

// ufbx.c:21165-21332 `ufbxi_modify_geometry`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn modify_geometry(uc: &Context) -> Result<(), Fail> {
    let mut do_mirror: bool = false;
    let do_winding: bool = (*uc.get()).opts.reverse_winding;
    let mut do_scale: bool = false;
    let mut do_geometry_transforms: bool = false;
    if (*uc.get()).opts.geometry_transform_handling == GeometryTransformHandling::ModifyGeometry
        || (*uc.get()).opts.geometry_transform_handling
            == GeometryTransformHandling::ModifyGeometryNoFallback
    {
        // Prefetch geometry transforms for processing, they will later be overwritten in `ufbxi_update_node()`.
        // C: `ufbxi_for_ptr_list(ufbx_node, p_node, uc->scene.nodes)`
        let mut p_node: *mut *mut Node = (*uc.get()).scene.nodes.data as *mut *mut Node;
        let p_node_end: *mut *mut Node = add_ptr(p_node, (*uc.get()).scene.nodes.count);
        while p_node != p_node_end {
            let node: *mut Node = *p_node;
            if (*node).is_root {
                p_node = p_node.add(1);
                continue;
            }

            (*node).geometry_transform = get_geometry_transform(&(*node).element.props, node);
            if !is_transform_identity(ptr::addr_of!((*node).geometry_transform)) {
                (*node).geometry_to_node =
                    transform_to_matrix(ptr::addr_of!((*node).geometry_transform));
                (*node).has_geometry_transform = true;
            } else {
                (*node).geometry_to_node = IDENTITY_MATRIX;
                (*node).has_geometry_transform = false;
            }
            p_node = p_node.add(1);
        }
        do_geometry_transforms = true;
    }
    if (*uc.get()).mirror_axis != MirrorAxis::None {
        do_mirror = true;
    }
    if (*uc.get()).scene.metadata.geometry_scale != 1.0 {
        do_scale = true;
    }

    let geometry_scale: Real = (*uc.get()).scene.metadata.geometry_scale;
    let mirror_axis: MirrorAxis = (*uc.get()).mirror_axis;

    // C: `ufbxi_for_ptr_list(ufbx_blend_shape, p_shape, uc->scene.blend_shapes)`
    let mut p_shape: *mut *mut BlendShape =
        (*uc.get()).scene.blend_shapes.data as *mut *mut BlendShape;
    let p_shape_end: *mut *mut BlendShape = add_ptr(p_shape, (*uc.get()).scene.blend_shapes.count);
    while p_shape != p_shape_end {
        let shape: *mut BlendShape = *p_shape;

        if do_scale {
            scale_vec3_list(
                ptr::addr_of!((*shape).position_offsets) as *const c_void,
                geometry_scale,
                0,
            );
        }

        if do_mirror {
            mirror_vec3_list(
                ptr::addr_of!((*shape).position_offsets) as *const c_void,
                mirror_axis,
                0,
            );
            mirror_vec3_list(
                ptr::addr_of!((*shape).normal_offsets) as *const c_void,
                mirror_axis,
                0,
            );
        }
        p_shape = p_shape.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, uc->scene.meshes)`
    let mut p_mesh: *mut *mut Mesh = (*uc.get()).scene.meshes.data as *mut *mut Mesh;
    let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, (*uc.get()).scene.meshes.count);
    while p_mesh != p_mesh_end {
        let mesh: *mut Mesh = *p_mesh;

        if do_scale {
            scale_vec3_list(
                ptr::addr_of!((*mesh).vertex_position.values) as *const c_void,
                geometry_scale,
                0,
            );
        }

        let mut do_flip_winding: bool = do_winding;
        if do_mirror {
            mirror_vec3_list(
                ptr::addr_of!((*mesh).vertex_position.values) as *const c_void,
                mirror_axis,
                0,
            );
            mirror_vec3_list(
                ptr::addr_of!((*mesh).vertex_normal.values) as *const c_void,
                mirror_axis,
                0,
            );
            // C: `ufbxi_for_list(ufbx_uv_set, set, mesh->uv_sets)`
            let mut set: *mut UvSet = (*mesh).uv_sets.data as *mut UvSet;
            let set_end: *mut UvSet = add_ptr(set, (*mesh).uv_sets.count);
            while set != set_end {
                mirror_vec3_list(
                    ptr::addr_of!((*set).vertex_tangent.values) as *const c_void,
                    mirror_axis,
                    0,
                );
                mirror_vec3_list(
                    ptr::addr_of!((*set).vertex_bitangent.values) as *const c_void,
                    mirror_axis,
                    0,
                );
                set = set.add(1);
            }
            if !(*uc.get()).opts.handedness_conversion_retain_winding {
                do_flip_winding = !do_flip_winding;
            }
        }

        // Flip face winding retaining the first vertex
        if do_flip_winding {
            (*mesh).reversed_winding = true;
            flip_winding(uc, mesh)?;
        }

        let geo_node: *mut Node = get_geometry_transform_node(ptr::addr_of_mut!((*mesh).element));
        if do_geometry_transforms && !geo_node.is_null() {
            let mut tangent_matrix: Matrix = (*geo_node).geometry_to_node;
            tangent_matrix.m03 = 0.0;
            tangent_matrix.m13 = 0.0;
            tangent_matrix.m23 = 0.0;
            let normal_matrix: Matrix =
                matrix_for_normals(ptr::addr_of!((*geo_node).geometry_to_node));

            transform_vec3_list(
                ptr::addr_of!((*mesh).vertex_position.values) as *const c_void,
                ptr::addr_of!((*geo_node).geometry_to_node),
                0,
            );
            transform_vec3_list(
                ptr::addr_of!((*mesh).vertex_normal.values) as *const c_void,
                &normal_matrix,
                0,
            );
            normalize_vec3_list(ptr::addr_of!((*mesh).vertex_normal.values));

            // C: `ufbxi_for_list(ufbx_uv_set, set, mesh->uv_sets)`
            let mut set: *mut UvSet = (*mesh).uv_sets.data as *mut UvSet;
            let set_end: *mut UvSet = add_ptr(set, (*mesh).uv_sets.count);
            while set != set_end {
                transform_vec3_list(
                    ptr::addr_of!((*set).vertex_tangent.values) as *const c_void,
                    &tangent_matrix,
                    0,
                );
                transform_vec3_list(
                    ptr::addr_of!((*set).vertex_bitangent.values) as *const c_void,
                    &tangent_matrix,
                    0,
                );
                normalize_vec3_list(ptr::addr_of!((*set).vertex_tangent.values));
                normalize_vec3_list(ptr::addr_of!((*set).vertex_bitangent.values));
                set = set.add(1);
            }
        }
        p_mesh = p_mesh.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_line_curve, p_curve, uc->scene.line_curves)`
    let mut p_curve: *mut *mut LineCurve =
        (*uc.get()).scene.line_curves.data as *mut *mut LineCurve;
    let p_curve_end: *mut *mut LineCurve = add_ptr(p_curve, (*uc.get()).scene.line_curves.count);
    while p_curve != p_curve_end {
        let curve: *mut LineCurve = *p_curve;

        if do_scale {
            scale_vec3_list(
                ptr::addr_of!((*curve).control_points) as *const c_void,
                geometry_scale,
                0,
            );
        }

        if do_mirror {
            mirror_vec3_list(
                ptr::addr_of!((*curve).control_points) as *const c_void,
                mirror_axis,
                0,
            );
        }

        let geo_node: *mut Node = get_geometry_transform_node(ptr::addr_of_mut!((*curve).element));
        if do_geometry_transforms && !geo_node.is_null() {
            transform_vec3_list(
                ptr::addr_of!((*curve).control_points) as *const c_void,
                ptr::addr_of!((*geo_node).geometry_to_node),
                0,
            );
        }
        p_curve = p_curve.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_nurbs_curve, p_curve, uc->scene.nurbs_curves)`
    let mut p_curve: *mut *mut NurbsCurve =
        (*uc.get()).scene.nurbs_curves.data as *mut *mut NurbsCurve;
    let p_curve_end: *mut *mut NurbsCurve = add_ptr(p_curve, (*uc.get()).scene.nurbs_curves.count);
    while p_curve != p_curve_end {
        let curve: *mut NurbsCurve = *p_curve;

        if do_scale {
            scale_vec3_list(
                ptr::addr_of!((*curve).control_points) as *const c_void,
                geometry_scale,
                size_of::<Vec4>(),
            );
        }

        if do_mirror {
            mirror_vec3_list(
                ptr::addr_of!((*curve).control_points) as *const c_void,
                mirror_axis,
                size_of::<Vec4>(),
            );
        }

        let geo_node: *mut Node = get_geometry_transform_node(ptr::addr_of_mut!((*curve).element));
        if do_geometry_transforms && !geo_node.is_null() {
            transform_vec3_list(
                ptr::addr_of!((*curve).control_points) as *const c_void,
                ptr::addr_of!((*geo_node).geometry_to_node),
                size_of::<Vec4>(),
            );
        }
        p_curve = p_curve.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_nurbs_surface, p_surface, uc->scene.nurbs_surfaces)`
    let mut p_surface: *mut *mut NurbsSurface =
        (*uc.get()).scene.nurbs_surfaces.data as *mut *mut NurbsSurface;
    let p_surface_end: *mut *mut NurbsSurface =
        add_ptr(p_surface, (*uc.get()).scene.nurbs_surfaces.count);
    while p_surface != p_surface_end {
        let surface: *mut NurbsSurface = *p_surface;

        if do_scale {
            scale_vec3_list(
                ptr::addr_of!((*surface).control_points) as *const c_void,
                geometry_scale,
                size_of::<Vec4>(),
            );
        }

        if do_mirror {
            mirror_vec3_list(
                ptr::addr_of!((*surface).control_points) as *const c_void,
                mirror_axis,
                size_of::<Vec4>(),
            );
        }

        let geo_node: *mut Node =
            get_geometry_transform_node(ptr::addr_of_mut!((*surface).element));
        if do_geometry_transforms && !geo_node.is_null() {
            transform_vec3_list(
                ptr::addr_of!((*surface).control_points) as *const c_void,
                ptr::addr_of!((*geo_node).geometry_to_node),
                size_of::<Vec4>(),
            );
        }
        p_surface = p_surface.add(1);
    }

    if (*uc.get()).opts.geometry_transform_handling != GeometryTransformHandling::Preserve {
        // Reset all geometry transforms if we're not preserving them
        let mut defaults: *mut Props = ptr::null_mut();
        // C: `ufbxi_for_ptr_list(ufbx_node, p_node, uc->scene.nodes)`
        let mut p_node: *mut *mut Node = (*uc.get()).scene.nodes.data as *mut *mut Node;
        let p_node_end: *mut *mut Node = add_ptr(p_node, (*uc.get()).scene.nodes.count);
        while p_node != p_node_end {
            let node: *mut Node = *p_node;
            if defaults.is_null() {
                defaults = opt_ptr(&(*node).element.props.defaults);
            }

            if (*node).has_geometry_transform {
                set_own_prop_vec3_uniform(
                    ptr::addr_of_mut!((*node).element.props),
                    sp::GeometricTranslation.as_ptr(),
                    0.0,
                );
                set_own_prop_vec3_uniform(
                    ptr::addr_of_mut!((*node).element.props),
                    sp::GeometricRotation.as_ptr(),
                    0.0,
                );
                set_own_prop_vec3_uniform(
                    ptr::addr_of_mut!((*node).element.props),
                    sp::GeometricScaling.as_ptr(),
                    1.0,
                );
            }
            p_node = p_node.add(1);
        }

        if !defaults.is_null() {
            set_own_prop_vec3_uniform(defaults, sp::GeometricTranslation.as_ptr(), 0.0);
            set_own_prop_vec3_uniform(defaults, sp::GeometricRotation.as_ptr(), 0.0);
            set_own_prop_vec3_uniform(defaults, sp::GeometricScaling.as_ptr(), 1.0);
        }
    }

    Ok(())
}

// ufbx.c:21334-21356 `ufbxi_postprocess_scene`
#[inline(never)]
pub(crate) unsafe fn postprocess_scene(uc: &Context) {
    if (*uc.get()).opts.normalize_normals || (*uc.get()).opts.normalize_tangents {
        // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, uc->scene.meshes)`
        let mut p_mesh: *mut *mut Mesh = (*uc.get()).scene.meshes.data as *mut *mut Mesh;
        let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, (*uc.get()).scene.meshes.count);
        while p_mesh != p_mesh_end {
            let mesh: *mut Mesh = *p_mesh;
            if (*uc.get()).opts.normalize_normals {
                normalize_vec3_list(ptr::addr_of!((*mesh).vertex_normal.values));
            }
            if (*uc.get()).opts.normalize_tangents {
                // C-parity: the loop body normalizes the MESH-level tangent and
                // bitangent lists (not `set->...`), so it repeats the same work
                // once per UV set. Ported verbatim.
                let mut set: *mut UvSet = (*mesh).uv_sets.data as *mut UvSet;
                let set_end: *mut UvSet = add_ptr(set, (*mesh).uv_sets.count);
                while set != set_end {
                    normalize_vec3_list(ptr::addr_of!((*mesh).vertex_tangent.values));
                    normalize_vec3_list(ptr::addr_of!((*mesh).vertex_bitangent.values));
                    set = set.add(1);
                }
            }
            p_mesh = p_mesh.add(1);
        }
    }

    if (*uc.get()).exporter == Exporter::BlenderBinary {
        (*uc.get()).scene.metadata.ortho_size_unit =
            1.0 / (*uc.get()).scene.metadata.geometry_scale;
    } else {
        (*uc.get()).scene.metadata.ortho_size_unit = 30.0;
    }
}

// ufbx.c:21358-21366 `ufbxi_next_path_segment`
#[inline(never)]
pub(crate) unsafe fn next_path_segment(data: *const u8, begin: usize, length: usize) -> usize {
    let mut i: usize = begin;
    while i < length {
        if *data.add(i) == b'/' || *data.add(i) == b'\\' {
            return i;
        }
        i += 1;
    }
    length
}

// ufbx.c:21368-21435 `ufbxi_absolute_to_relative_path`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn absolute_to_relative_path(
    uc: &Context,
    p_dst: *mut Strblob,
    p_rel: *const Strblob,
    p_src: *const Strblob,
    raw: bool,
) -> Result<(), Fail> {
    let rel: *const u8 = strblob_data(p_rel, raw);
    let src: *const u8 = strblob_data(p_src, raw);
    let mut rel_length: usize = strblob_length(p_rel, raw);
    let src_length: usize = strblob_length(p_src, raw);

    if rel_length == 0 || src_length == 0 {
        return Ok(());
    }

    // Absolute paths must start with the same character (either drive or '/')
    if *rel.add(0) != *src.add(0) {
        return Ok(());
    }

    // Find the last directory of the path we want to be relative to
    while rel_length > 0 && (*rel.add(rel_length - 1) != b'/' && *rel.add(rel_length - 1) != b'\\')
    {
        rel_length -= 1;
    }

    if rel_length == 0 {
        return Ok(());
    }
    let separator: u8 = *rel.add(rel_length - 1);

    let max_length: usize = rel_length.wrapping_mul(2).wrapping_add(src_length);

    ufbxi_check!(
        uc,
        grow_array::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.tmp_arr_mut_ptr(),
            uc.tmp_arr_size_mut_ptr(),
            max_length,
        ),
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (max_length))"
    );
    let tmp: *mut u8 = uc.tmp_arr();
    let mut tmp_length: usize = 0;

    let mut rel_begin: usize = 0;
    let mut src_begin: usize = 0;
    while rel_begin < rel_length && src_begin < src_length {
        let rel_end: usize = next_path_segment(rel, rel_begin, rel_length);
        let src_end: usize = next_path_segment(src, src_begin, src_length);
        if rel_end != src_end
            || memcmp(rel.add(rel_begin), src.add(src_begin), src_end - src_begin) != 0
        {
            break;
        }
        rel_begin = rel_end + 1;
        src_begin = src_end + 1;
    }

    while rel_begin < rel_length {
        let rel_end: usize = next_path_segment(rel, rel_begin, rel_length);
        *tmp.add(tmp_length) = b'.';
        tmp_length += 1;
        *tmp.add(tmp_length) = b'.';
        tmp_length += 1;
        *tmp.add(tmp_length) = separator;
        tmp_length += 1;
        rel_begin = rel_end + 1;
    }

    while src_begin < src_length {
        let src_end: usize = next_path_segment(src, src_begin, src_length);
        let len: usize = src_end - src_begin;

        ptr::copy_nonoverlapping(src.add(src_begin), tmp.add(tmp_length), len);
        tmp_length += len;

        if src_end < src_length {
            *tmp.add(tmp_length) = separator;
            tmp_length += 1;
        }

        src_begin = src_end + 1;
    }

    ufbx_assert!(tmp_length <= max_length);

    // C-parity: `raw` is hardcoded `true` here, independent of the `raw`
    // parameter that selected the source/destination strblob members.
    let dst: *const u8 = sp::push_string(
        uc.string_pool_mut_ptr(),
        tmp,
        tmp_length,
        ptr::null_mut(),
        true,
    );
    ufbxi_check!(uc, !dst.is_null(), "dst");

    strblob_set(p_dst, dst, tmp_length, raw);

    Ok(())
}

// ufbx.c:21437-21450 `ufbxi_resolve_filenames`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn resolve_filenames(
    uc: &Context,
    filename: *mut Strblob,
    absolute_filename: *mut Strblob,
    relative_filename: *mut Strblob,
    raw: bool,
) -> Result<(), Fail> {
    if strblob_length(relative_filename, raw) == 0 {
        let original_file_path: *const Strblob = if raw {
            &(*uc.get()).scene.metadata.raw_original_file_path as *const Blob as *const Strblob
        } else {
            &(*uc.get()).scene.metadata.original_file_path as *const String as *const Strblob
        };

        absolute_to_relative_path(
            uc,
            relative_filename,
            original_file_path,
            absolute_filename,
            raw,
        )?;
    }

    resolve_relative_filename(uc, filename, relative_filename, raw)?;

    Ok(())
}

// ufbx.c:21452-21457 `ufbxi_file_content_less`
#[inline(never)]
pub(crate) unsafe extern "C" fn file_content_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const FileContent = va as *const FileContent;
    let b: *const FileContent = vb as *const FileContent;
    str_less((*a).absolute_filename, (*b).absolute_filename)
}

// ufbx.c:21459-21464 `ufbxi_sort_file_contents`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn sort_file_contents(
    uc: &Context,
    content: *mut FileContent,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            uc.ator_tmp_mut_ptr(),
            uc.tmp_arr_mut_ptr(),
            uc.tmp_arr_size_mut_ptr(),
            count.wrapping_mul(size_of::<FileContent>()),
        ),
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbxi_file_content)))"
    );
    stable_sort(
        size_of::<FileContent>(),
        32,
        content as *mut c_void,
        uc.tmp_arr() as *mut c_void,
        count,
        file_content_less,
        ptr::null_mut(),
    );
    Ok(())
}

// ufbx.c:21466-21474 `ufbxi_push_file_content`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_file_content(
    uc: &Context,
    p_filename: *mut String,
    p_data: *mut Blob,
) -> Result<(), Fail> {
    if (*p_data).size == 0 || (*p_filename).length == 0 {
        return Ok(());
    }
    let content: *mut FileContent = push::<FileContent>(uc.tmp_stack_mut_ptr(), 1);
    ufbxi_check!(uc, !content.is_null(), "content");

    (*content).absolute_filename = *p_filename;
    (*content).content = *p_data;
    Ok(())
}

// ufbx.c:21476-21487 `ufbxi_fetch_file_content`
#[inline(never)]
pub(crate) unsafe fn fetch_file_content(uc: &Context, p_filename: *mut String, p_data: *mut Blob) {
    if (*p_data).size > 0 {
        return;
    }
    let filename: String = *p_filename;
    let mut index: usize = usize::MAX;
    // C: `ufbxi_macro_lower_bound_eq(ufbxi_file_content, 8, &index,
    // uc->file_content, 0, uc->num_file_content, ...)` — does NOT write
    // `index` on a miss, hence the `SIZE_MAX` pre-initialization above.
    macro_lower_bound_eq(
        8,
        &mut index,
        uc.file_content() as *const FileContent,
        0,
        uc.num_file_content(),
        |a| str_less((*a).absolute_filename, filename),
        // C-parity: the equality lambda compares interned string POINTERS, not
        // the bytes.
        |a| (*a).absolute_filename.data == filename.data,
    );
    if index != usize::MAX {
        *p_data = (*uc.file_content().add(index)).content;
    }
}

// ufbx.c:21489-21526 `ufbxi_resolve_file_content`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn resolve_file_content(uc: &Context) -> Result<(), Fail> {
    let initial_stack: usize = (*uc.get()).tmp_stack.num_items;

    // C: `ufbxi_for_ptr_list(ufbx_video, p_video, uc->scene.videos)`
    let mut p_video: *mut *mut Video = (*uc.get()).scene.videos.data as *mut *mut Video;
    let p_video_end: *mut *mut Video = add_ptr(p_video, (*uc.get()).scene.videos.count);
    while p_video != p_video_end {
        let video: *mut Video = *p_video;
        resolve_filenames(
            uc,
            ptr::addr_of_mut!((*video).filename) as *mut Strblob,
            ptr::addr_of_mut!((*video).absolute_filename) as *mut Strblob,
            ptr::addr_of_mut!((*video).relative_filename) as *mut Strblob,
            false,
        )?;
        resolve_filenames(
            uc,
            ptr::addr_of_mut!((*video).raw_filename) as *mut Strblob,
            ptr::addr_of_mut!((*video).raw_absolute_filename) as *mut Strblob,
            ptr::addr_of_mut!((*video).raw_relative_filename) as *mut Strblob,
            true,
        )?;
        push_file_content(
            uc,
            ptr::addr_of_mut!((*video).absolute_filename),
            ptr::addr_of_mut!((*video).content),
        )?;
        p_video = p_video.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_audio_clip, p_clip, uc->scene.audio_clips)`
    let mut p_clip: *mut *mut AudioClip = (*uc.get()).scene.audio_clips.data as *mut *mut AudioClip;
    let p_clip_end: *mut *mut AudioClip = add_ptr(p_clip, (*uc.get()).scene.audio_clips.count);
    while p_clip != p_clip_end {
        let clip: *mut AudioClip = *p_clip;
        (*clip).absolute_filename =
            find_string(&(*clip).element.props, b"Path\0".as_ptr(), EMPTY_STRING.0);
        (*clip).relative_filename = find_string(
            &(*clip).element.props,
            b"RelPath\0".as_ptr(),
            EMPTY_STRING.0,
        );
        (*clip).raw_absolute_filename =
            find_blob(&(*clip).element.props, b"Path\0".as_ptr(), EMPTY_BLOB.0);
        (*clip).raw_relative_filename =
            find_blob(&(*clip).element.props, b"RelPath\0".as_ptr(), EMPTY_BLOB.0);
        resolve_filenames(
            uc,
            ptr::addr_of_mut!((*clip).filename) as *mut Strblob,
            ptr::addr_of_mut!((*clip).absolute_filename) as *mut Strblob,
            ptr::addr_of_mut!((*clip).relative_filename) as *mut Strblob,
            false,
        )?;
        resolve_filenames(
            uc,
            ptr::addr_of_mut!((*clip).raw_filename) as *mut Strblob,
            ptr::addr_of_mut!((*clip).raw_absolute_filename) as *mut Strblob,
            ptr::addr_of_mut!((*clip).raw_relative_filename) as *mut Strblob,
            true,
        )?;
        push_file_content(
            uc,
            ptr::addr_of_mut!((*clip).absolute_filename),
            ptr::addr_of_mut!((*clip).content),
        )?;
        p_clip = p_clip.add(1);
    }

    uc.set_num_file_content((*uc.get()).tmp_stack.num_items - initial_stack);
    uc.set_file_content(push_pop::<FileContent>(
        uc.tmp_mut_ptr(),
        uc.tmp_stack_mut_ptr(),
        uc.num_file_content(),
    ));
    ufbxi_check!(uc, !uc.file_content().is_null(), "uc->file_content");
    sort_file_contents(uc, uc.file_content(), uc.num_file_content())?;

    // C: `ufbxi_for_ptr_list(ufbx_video, p_video, uc->scene.videos)`
    let mut p_video: *mut *mut Video = (*uc.get()).scene.videos.data as *mut *mut Video;
    let p_video_end: *mut *mut Video = add_ptr(p_video, (*uc.get()).scene.videos.count);
    while p_video != p_video_end {
        let video: *mut Video = *p_video;
        fetch_file_content(
            uc,
            ptr::addr_of_mut!((*video).absolute_filename),
            ptr::addr_of_mut!((*video).content),
        );
        p_video = p_video.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_audio_clip, p_clip, uc->scene.audio_clips)`
    let mut p_clip: *mut *mut AudioClip = (*uc.get()).scene.audio_clips.data as *mut *mut AudioClip;
    let p_clip_end: *mut *mut AudioClip = add_ptr(p_clip, (*uc.get()).scene.audio_clips.count);
    while p_clip != p_clip_end {
        let clip: *mut AudioClip = *p_clip;
        fetch_file_content(
            uc,
            ptr::addr_of_mut!((*clip).absolute_filename),
            ptr::addr_of_mut!((*clip).content),
        );
        p_clip = p_clip.add(1);
    }

    Ok(())
}

// ufbx.c:21528-21543 `ufbxi_validate_indices`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn validate_indices(
    uc: &Context,
    indices: *mut List<u32>,
    max_index: usize,
) -> Result<(), Fail> {
    if max_index == 0 && (*uc.get()).opts.index_error_handling == IndexErrorHandling::Clamp {
        (*indices).data = ptr::null_mut();
        (*indices).count = 0;
        return Ok(());
    }

    // C: `ufbxi_nounroll ufbxi_for_list(uint32_t, p_ix, *indices)` — the
    // no-unroll pragma is optimizer-only and has no Rust analogue.
    let mut p_ix: *mut u32 = (*indices).data as *mut u32;
    let p_ix_end: *mut u32 = add_ptr(p_ix, (*indices).count);
    while p_ix != p_ix_end {
        let ix: u32 = *p_ix;
        // C: `ix >= max_index` — `ix` is promoted to `size_t` for the compare.
        if ix as usize >= max_index {
            fix_index(uc, p_ix, ix, max_index)?;
        }
        p_ix = p_ix.add(1);
    }

    Ok(())
}

// ufbx.c:21545-21556 `ufbxi_material_part_usage_less`
pub(crate) unsafe extern "C" fn material_part_usage_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    let parts: *mut MeshPart = user as *mut MeshPart;
    let a: u32 = *(va as *const u32);
    let b: u32 = *(vb as *const u32);
    let pa: *mut MeshPart = parts.add(a as usize);
    let pb: *mut MeshPart = parts.add(b as usize);
    if (*pa).face_indices.count == 0 || (*pb).face_indices.count == 0 {
        if (*pa).face_indices.count == (*pb).face_indices.count {
            return a < b;
        }
        return (*pa).face_indices.count > (*pb).face_indices.count;
    }
    *(*pa).face_indices.data.add(0) < *(*pb).face_indices.data.add(0)
}

// ufbx.c:21558-21620 `ufbxi_finalize_mesh_material`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn finalize_mesh_material(
    buf: *mut Buf,
    error: *mut Error,
    mesh: *mut Mesh,
) -> Result<(), Fail> {
    let num_materials: usize = (*mesh).materials.count;
    let num_parts: usize = (*mesh).material_parts.count;
    let num_faces: usize = (*mesh).faces.count;

    let parts: *mut MeshPart = (*mesh).material_parts.data as *mut MeshPart;
    ufbx_assert!(
        parts.is_null()
            || ((*mesh).material_parts.count == num_materials)
            || ((*mesh).material_parts.count == 1 && num_materials == 0)
    );

    let face_material: *mut u32 = (*mesh).face_material.data as *mut u32;

    // Count the number of faces and triangles per material
    // C: `ufbxi_nounroll for (size_t i = 0; i < num_faces; i++)`
    for i in 0..num_faces {
        let face: Face = *(*mesh).faces.data.add(i);
        let mut mat_ix: u32 = 0;

        if !face_material.is_null() {
            mat_ix = *face_material.add(i);
            if mat_ix as usize >= num_materials {
                *face_material.add(i) = 0;
                mat_ix = 0;
            }
        }

        if !parts.is_null() {
            mesh_part_add_face(parts.add(mat_ix as usize), face.num_indices);
        }
    }

    if !parts.is_null() {
        // Allocate per-material buffers (clear `num_faces` to 0 to re-use it as
        // an index when fetching the face indices).
        let mut part_index: u32 = 0;
        // C: `ufbxi_for(ufbx_mesh_part, part, parts, num_parts)`
        let mut part: *mut MeshPart = parts;
        let part_end: *mut MeshPart = add_ptr(part, num_parts);
        while part != part_end {
            // C: `part->index = part_index++;` — assigns the pre-increment value.
            (*part).index = part_index;
            part_index = part_index.wrapping_add(1);
            (*part).face_indices.count = (*part).num_faces;
            (*part).face_indices.data = push::<u32>(buf, (*part).num_faces);
            ufbxi_check_err!(
                error,
                !(*part).face_indices.data.is_null(),
                "part->face_indices.data"
            );
            (*part).num_faces = 0;
            part = part.add(1);
        }

        // Fetch the per-material face indices
        // C: `ufbxi_nounroll for (size_t i = 0; i < num_faces; i++)`
        for i in 0..num_faces {
            let mat_ix: u32 = if !face_material.is_null() {
                *face_material.add(i)
            } else {
                0
            };
            if (mat_ix as usize) < num_parts {
                let part: *mut MeshPart = parts.add(mat_ix as usize);
                // C: `part->face_indices.data[part->num_faces++] = (uint32_t)i;`
                *((*part).face_indices.data as *mut u32).add((*part).num_faces) = i as u32;
                (*part).num_faces = (*part).num_faces.wrapping_add(1);
            }
        }

        (*mesh).material_part_usage_order.count = num_parts;
        (*mesh).material_part_usage_order.data = push::<u32>(buf, num_parts);
        ufbxi_check_err!(
            error,
            !(*mesh).material_part_usage_order.data.is_null(),
            "mesh->material_part_usage_order.data"
        );
        for i in 0..num_parts {
            *((*mesh).material_part_usage_order.data as *mut u32).add(i) = i as u32;
        }
        unstable_sort(
            (*mesh).material_part_usage_order.data as *mut c_void,
            num_parts,
            size_of::<u32>(),
            material_part_usage_less,
            parts as *mut c_void,
        );
    }

    Ok(())
}

// ufbx.c:21622-21626 `ufbxi_anim_imp`
// C declares no `ufbx_static_assert` for this one (contrast `ufbxi_scene_imp` /
// `ufbxi_mesh_imp`), but `ufbxi_get_imp(ufbxi_anim_imp, anim)` (ufbx.c:31225)
// depends on the same header-then-payload layout, so the offset is pinned here.
#[repr(C)]
pub(crate) struct AnimImp {
    pub refcount: Refcount,
    pub anim: Anim,
    pub magic: u32,
}

const _: () = assert!(core::mem::offset_of!(AnimImp, anim) == size_of::<Refcount>());

// ufbx.c:21628-21638 `ufbxi_push_anim`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_anim(
    uc: &Context,
    p_anim: *mut *mut Anim,
    layers: *mut *mut AnimLayer,
    num_layers: usize,
) -> Result<(), Fail> {
    let anim: *mut Anim = push_zero::<Anim>(uc.result_mut_ptr(), 1);
    ufbxi_check!(uc, !anim.is_null(), "anim");

    (*anim).layers.data = layers as *const Ref<AnimLayer>;
    (*anim).layers.count = num_layers;

    *p_anim = anim;
    Ok(())
}

// ufbx.c:21641-22624 `ufbxi_finalize_scene`
// The single ~985-line pass that turns the parsed element/connection scratch
// into the public `ufbx_scene` graph. Split into no helpers upstream, so it is
// ported as one function.
#[inline(never)]
#[must_use]
pub(crate) unsafe fn finalize_scene(uc: &Context) -> Result<(), Fail> {
    let num_elements: usize = uc.num_elements() as usize;

    (*uc.get()).scene.elements.count = num_elements;
    (*uc.get()).scene.elements.data =
        push::<*mut Element>(uc.result_mut_ptr(), num_elements) as *const Ref<Element>;
    ufbxi_check!(
        uc,
        !(*uc.get()).scene.elements.data.is_null(),
        "uc->scene.elements.data"
    );

    (*uc.get()).scene.metadata.element_buffer_size = uc.tmp_element_byte_offset();
    let element_data: *mut u8 = push_pop::<u64>(
        uc.result_mut_ptr(),
        uc.tmp_elements_mut_ptr(),
        uc.tmp_element_byte_offset() / 8,
    ) as *mut u8;
    ufbxi_check!(uc, !element_data.is_null(), "element_data");

    // C reads `uc->tmp_element_offsets.num_items` as the `ufbxi_push_pop()`
    // count argument; hoisted to a local so the `&mut` borrow of the same
    // buffer does not overlap the read.
    let num_element_offsets: usize = (*uc.get()).tmp_element_offsets.num_items;
    let element_offsets: *mut usize = push_pop::<usize>(
        uc.tmp_mut_ptr(),
        uc.tmp_element_offsets_mut_ptr(),
        num_element_offsets,
    );
    buf_free(uc.tmp_element_offsets_mut_ptr());
    ufbxi_check!(uc, !element_offsets.is_null(), "element_offsets");
    for i in 0..num_elements {
        let element: *mut Element = element_data.add(*element_offsets.add(i)) as *mut Element;

        if (*element).type_ == ElementType::Node {
            let node: *mut Node = element as *mut Node;
            if !opt_ptr(&(*node).scale_helper).is_null() {
                let extra: *mut NodeExtra =
                    get_element_extra(uc, (*node).element.element_id) as *mut NodeExtra;
                ufbx_assert!(!extra.is_null());
                (*node).scale_helper = opt_ref(
                    element_data.add(*element_offsets.add((*extra).scale_helper_id as usize))
                        as *mut Node,
                );
            }
        }

        *((*uc.get()).scene.elements.data as *mut *mut Element).add(i) = element;
    }

    (*uc.get()).scene.elements.count = num_elements;
    buf_free(uc.tmp_element_offsets_mut_ptr());
    buf_free(uc.tmp_elements_mut_ptr());

    uc.set_tmp_element_flag(push_zero::<u8>(uc.tmp_mut_ptr(), num_elements));
    ufbxi_check!(uc, !uc.tmp_element_flag().is_null(), "uc->tmp_element_flag");

    (*uc.get()).scene.metadata.original_file_path = find_string(
        &(*uc.get()).scene.metadata.scene_props,
        b"DocumentUrl\0".as_ptr(),
        EMPTY_STRING.0,
    );
    (*uc.get()).scene.metadata.raw_original_file_path = find_blob(
        &(*uc.get()).scene.metadata.scene_props,
        b"DocumentUrl\0".as_ptr(),
        EMPTY_BLOB.0,
    );

    // Resolve and add the connections to elements
    resolve_connections(uc)?;
    add_connections_to_elements(uc)?;
    linearize_nodes(uc)?;

    for type_ in 0..ELEMENT_TYPE_COUNT {
        let num_typed: usize = (*uc.get()).tmp_typed_element_offsets[type_].num_items;
        let typed_offsets: *mut usize = push_pop::<usize>(
            uc.tmp_mut_ptr(),
            &mut (*uc.get()).tmp_typed_element_offsets[type_],
            num_typed,
        );
        buf_free(&mut (*uc.get()).tmp_typed_element_offsets[type_]);
        ufbxi_check!(uc, !typed_offsets.is_null(), "typed_offsets");

        // C indexes `uc->scene.elements_by_type[type]`, the `ufbx_element_list`
        // array view of the `ufbx_scene` per-type list union (ufbx.h:4015); the
        // generated struct keeps only the named branch, whose first member
        // (`unknowns`) is the array base.
        let typed_elems: *mut RefList<Element> =
            (ptr::addr_of_mut!((*uc.get()).scene.unknowns) as *mut RefList<Element>).add(type_);
        (*typed_elems).count = num_typed;
        (*typed_elems).data =
            push::<*mut Element>(uc.result_mut_ptr(), num_typed) as *const Ref<Element>;
        ufbxi_check!(uc, !(*typed_elems).data.is_null(), "typed_elems->data");

        for i in 0..num_typed {
            *((*typed_elems).data as *mut *mut Element).add(i) =
                element_data.add(*typed_offsets.add(i)) as *mut Element;
        }

        buf_free(&mut (*uc.get()).tmp_typed_element_offsets[type_]);
    }

    // Create named elements
    (*uc.get()).scene.elements_by_name.count = num_elements;
    (*uc.get()).scene.elements_by_name.data =
        push::<NameElement>(uc.result_mut_ptr(), num_elements);
    ufbxi_check!(
        uc,
        !(*uc.get()).scene.elements_by_name.data.is_null(),
        "uc->scene.elements_by_name.data"
    );

    for i in 0..num_elements {
        let elem: *mut Element = *((*uc.get()).scene.elements.data as *mut *mut Element).add(i);
        let name_elem: *mut NameElement =
            ((*uc.get()).scene.elements_by_name.data as *mut NameElement).add(i);

        (*name_elem).name = (*elem).name;
        (*name_elem).type_ = (*elem).type_;
        (*name_elem)._internal_key = get_name_key((*elem).name.data, (*elem).name.length);
        (*name_elem).element = Ref::from_ptr(elem);
    }

    sort_name_elements(
        uc,
        (*uc.get()).scene.elements_by_name.data as *mut NameElement,
        num_elements,
    )?;

    // Setup node children arrays and attribute pointers/lists
    // C: `ufbxi_for_ptr_list(ufbx_node, p_node, uc->scene.nodes)`
    let mut p_node: *mut *mut Node = (*uc.get()).scene.nodes.data as *mut *mut Node;
    let p_node_end: *mut *mut Node = add_ptr(p_node, (*uc.get()).scene.nodes.count);
    while p_node != p_node_end {
        let node: *mut Node = *p_node;
        let parent: *mut Node = opt_ptr(&(*node).parent);
        if !parent.is_null() {
            (*parent).children.count += 1;
            if (*parent).children.data.is_null() {
                (*parent).children.data = p_node as *const Ref<Node>;
            }

            if (*node).is_geometry_transform_helper {
                (*parent).geometry_transform_helper = opt_ref(node);
            }

            // Force top-level nodes to have `UFBX_INHERIT_MODE_NORMAL` to make unit scaling work.
            if (*parent).is_root
                && (*uc.get()).opts.space_conversion == SpaceConversion::TransformRoot
                && (*uc.get()).opts.inherit_mode_handling == InheritModeHandling::Preserve
            {
                (*node).original_inherit_mode = InheritMode::Normal;
                (*node).inherit_mode = InheritMode::Normal;
            }

            // RrSs nodes inherit scale from their parent, Rrs ignore the scale of
            // their _immediate_ parent, potentially multiple if chained.
            if (*node).original_inherit_mode == InheritMode::ComponentwiseScale {
                (*node).inherit_scale_node = opt_ref(parent);
            } else if (*node).original_inherit_mode == InheritMode::IgnoreParentScale {
                (*node).inherit_scale_node = (*parent).inherit_scale_node;
            }
        }

        let conns: List<Connection> = find_dst_connections(&mut (*node).element, ptr::null());

        // C: `ufbxi_for_list(ufbx_connection, conn, conns)` — indexed here
        // because the body `continue`s (the C `for` advances in its increment
        // clause).
        for conn_ix in 0..conns.count {
            let conn: *mut Connection = (conns.data as *mut Connection).add(conn_ix);
            let elem: *mut Element = ref_ptr(&(*conn).src);
            let type_: ElementType = (*elem).type_;
            if !(type_ as u32 >= ELEMENT_TYPE_FIRST_ATTRIB
                && type_ as u32 <= ELEMENT_TYPE_LAST_ATTRIB)
            {
                continue;
            }

            // C: `size_t index = node->all_attribs.count++;` — the
            // pre-increment value.
            let index: usize = (*node).all_attribs.count;
            (*node).all_attribs.count += 1;
            if index == 0 {
                (*node).attrib = opt_ref(elem);
                (*node).attrib_type = type_;
            } else {
                if index == 1 {
                    ufbxi_check!(
                        uc,
                        !push_copy::<*mut Element>(
                            uc.tmp_stack_mut_ptr(),
                            1,
                            ptr::addr_of!((*node).attrib) as *const *mut Element
                        )
                        .is_null(),
                        "((ufbx_element**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_element*), (1), (&node->attrib)))"
                    );
                }
                ufbxi_check!(
                    uc,
                    !push_copy::<*mut Element>(uc.tmp_stack_mut_ptr(), 1, &elem).is_null(),
                    "((ufbx_element**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_element*), (1), (&elem)))"
                );
            }

            match (*elem).type_ {
                ElementType::Mesh => (*node).mesh = opt_ref(elem as *mut Mesh),
                ElementType::Light => (*node).light = opt_ref(elem as *mut Light),
                ElementType::Camera => (*node).camera = opt_ref(elem as *mut Camera),
                ElementType::Bone => (*node).bone = opt_ref(elem as *mut Bone),
                _ => { /* No shorthand */ }
            }
        }

        if (*node).all_attribs.count > 1 {
            (*node).all_attribs.data = push_pop::<*mut Element>(
                uc.result_mut_ptr(),
                uc.tmp_stack_mut_ptr(),
                (*node).all_attribs.count,
            ) as *const Ref<Element>;
            ufbxi_check!(
                uc,
                !(*node).all_attribs.data.is_null(),
                "node->all_attribs.data"
            );
        } else if (*node).all_attribs.count == 1 {
            (*node).all_attribs.data = ptr::addr_of!((*node).attrib) as *const Ref<Element>;
        }

        fetch_dst_elements(
            uc,
            ptr::addr_of_mut!((*node).materials) as *mut c_void,
            &mut (*node).element,
            false,
            false,
            ptr::null(),
            ElementType::Material,
        )?;
        p_node = p_node.add(1);
    }

    // Resolve bind pose bones that don't use the normal connection system
    // C: `ufbxi_for_ptr_list(ufbx_pose, p_pose, uc->scene.poses)`
    let mut p_pose: *mut *mut Pose = (*uc.get()).scene.poses.data as *mut *mut Pose;
    let p_pose_end: *mut *mut Pose = add_ptr(p_pose, (*uc.get()).scene.poses.count);
    while p_pose != p_pose_end {
        let pose: *mut Pose = *p_pose;

        // HACK: Transport `ufbxi_tmp_bone_pose` array through the `ufbx_bone_pose` pointer
        let num_bones: usize = (*pose).bone_poses.count;
        let tmp_poses: *mut TmpBonePose = (*pose).bone_poses.data as *mut TmpBonePose;
        (*pose).bone_poses.data = push::<BonePose>(uc.result_mut_ptr(), num_bones);
        ufbxi_check!(
            uc,
            !(*pose).bone_poses.data.is_null(),
            "pose->bone_poses.data"
        );

        // Filter only found bones
        (*pose).bone_poses.count = 0;
        for i in 0..num_bones {
            let elem: *mut Element = find_element_by_fbx_id(uc, (*tmp_poses.add(i)).bone_fbx_id);
            if elem.is_null() || (*elem).type_ != ElementType::Node {
                continue;
            }

            let node: *mut Node = elem as *mut Node;
            // C: `&pose->bone_poses.data[pose->bone_poses.count++]`
            let bone: *mut BonePose =
                ((*pose).bone_poses.data as *mut BonePose).add((*pose).bone_poses.count);
            (*pose).bone_poses.count += 1;
            (*bone).bone_node = Ref::from_ptr(node);
            (*bone).bone_to_world = (*tmp_poses.add(i)).bone_to_world;

            if (*pose).is_bind_pose {
                if opt_ptr(&(*node).bind_pose).is_null() {
                    (*node).bind_pose = opt_ref(pose);
                }

                let node_conns: List<Connection> = find_src_connections(elem, ptr::null());
                // C: `ufbxi_for_list(ufbx_connection, conn, node_conns)`
                for conn_ix in 0..node_conns.count {
                    let conn: *mut Connection = (node_conns.data as *mut Connection).add(conn_ix);
                    if (*ref_ptr(&(*conn).dst)).type_ != ElementType::SkinCluster {
                        continue;
                    }
                    let cluster: *mut SkinCluster = ref_ptr(&(*conn).dst) as *mut SkinCluster;
                    if matrix_all_zero(&(*cluster).bind_to_world) {
                        (*cluster).bind_to_world = (*bone).bone_to_world;
                    }
                }
            }
        }
        sort_bone_poses(uc, pose)?;
        p_pose = p_pose.add(1);
    }

    // Fetch pointers that may break elements

    // Setup node attribute instances
    // C: `for (int type = UFBX_ELEMENT_TYPE_FIRST_ATTRIB; type <= UFBX_ELEMENT_TYPE_LAST_ATTRIB; type++)`
    let mut attrib_type: u32 = ELEMENT_TYPE_FIRST_ATTRIB;
    while attrib_type <= ELEMENT_TYPE_LAST_ATTRIB {
        let typed_elems: *mut RefList<Element> = (ptr::addr_of_mut!((*uc.get()).scene.unknowns)
            as *mut RefList<Element>)
            .add(attrib_type as usize);
        // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, uc->scene.elements_by_type[type])`
        let mut p_elem: *mut *mut Element = (*typed_elems).data as *mut *mut Element;
        let p_elem_end: *mut *mut Element = add_ptr(p_elem, (*typed_elems).count);
        while p_elem != p_elem_end {
            let elem: *mut Element = *p_elem;
            fetch_src_elements(
                uc,
                ptr::addr_of_mut!((*elem).instances) as *mut c_void,
                elem,
                false,
                true,
                ptr::null(),
                ElementType::Node,
            )?;
            p_elem = p_elem.add(1);
        }
        attrib_type += 1;
    }

    let search_node: bool = uc.version() < 7000;

    // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, uc->scene.skin_clusters)`
    let mut p_cluster: *mut *mut SkinCluster =
        (*uc.get()).scene.skin_clusters.data as *mut *mut SkinCluster;
    let p_cluster_end: *mut *mut SkinCluster =
        add_ptr(p_cluster, (*uc.get()).scene.skin_clusters.count);
    while p_cluster != p_cluster_end {
        let cluster: *mut SkinCluster = *p_cluster;
        (*cluster).bone_node = opt_ref(fetch_dst_element(
            &mut (*cluster).element,
            false,
            ptr::null(),
            ElementType::Node,
        ) as *mut Node);
        p_cluster = p_cluster.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_skin_deformer, p_skin, uc->scene.skin_deformers)`
    let mut p_skin: *mut *mut SkinDeformer =
        (*uc.get()).scene.skin_deformers.data as *mut *mut SkinDeformer;
    let p_skin_end: *mut *mut SkinDeformer =
        add_ptr(p_skin, (*uc.get()).scene.skin_deformers.count);
    while p_skin != p_skin_end {
        let skin: *mut SkinDeformer = *p_skin;
        fetch_dst_elements(
            uc,
            ptr::addr_of_mut!((*skin).clusters) as *mut c_void,
            &mut (*skin).element,
            false,
            true,
            ptr::null(),
            ElementType::SkinCluster,
        )?;

        // Remove clusters without a valid `bone`
        if !(*uc.get()).opts.connect_broken_elements {
            let clusters: *mut *mut SkinCluster = (*skin).clusters.data as *mut *mut SkinCluster;
            let mut num_broken: usize = 0;
            for i in 0..(*skin).clusters.count {
                if opt_ptr(&(**clusters.add(i)).bone_node).is_null() {
                    num_broken += 1;
                } else if num_broken > 0 {
                    *clusters.add(i - num_broken) = *clusters.add(i);
                }
            }
            (*skin).clusters.count -= num_broken;
        }

        let mut total_weights: usize = 0;
        // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, skin->clusters)`
        let mut p_cluster: *mut *mut SkinCluster = (*skin).clusters.data as *mut *mut SkinCluster;
        let p_cluster_end: *mut *mut SkinCluster = add_ptr(p_cluster, (*skin).clusters.count);
        while p_cluster != p_cluster_end {
            let cluster: *mut SkinCluster = *p_cluster;
            ufbxi_check!(
                uc,
                usize::MAX - total_weights > (*cluster).num_weights,
                "SIZE_MAX - total_weights > cluster->num_weights"
            );
            total_weights += (*cluster).num_weights;
            p_cluster = p_cluster.add(1);
        }

        let mut num_vertices: usize = 0;

        // Iterate through meshes so we can pad the vertices to the largest one
        {
            let conns: List<Connection> = find_src_connections(&mut (*skin).element, ptr::null());
            // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
            for conn_ix in 0..conns.count {
                let conn: *mut Connection = (conns.data as *mut Connection).add(conn_ix);
                let mut mesh: *mut Mesh = ptr::null_mut();
                if (*conn).dst_prop.length > 0 {
                    continue;
                }
                let dst: *mut Element = ref_ptr(&(*conn).dst);
                if (*dst).type_ == ElementType::Mesh {
                    mesh = dst as *mut Mesh;
                } else if (*dst).type_ == ElementType::Node {
                    let mut node: *mut Node = dst as *mut Node;
                    if !opt_ptr(&(*node).geometry_transform_helper).is_null() {
                        node = opt_ptr(&(*node).geometry_transform_helper);
                    }
                    mesh = opt_ptr(&(*node).mesh);
                }
                if mesh.is_null() {
                    continue;
                }
                num_vertices = max_sz(num_vertices, (*mesh).num_vertices);
            }
        }

        if !(*uc.get()).opts.skip_skin_vertices {
            (*skin).vertices.count = num_vertices;
            (*skin).vertices.data = push_zero::<SkinVertex>(uc.result_mut_ptr(), num_vertices);
            ufbxi_check!(uc, !(*skin).vertices.data.is_null(), "skin->vertices.data");

            (*skin).weights.count = total_weights;
            (*skin).weights.data = push_zero::<SkinWeight>(uc.result_mut_ptr(), total_weights);
            ufbxi_check!(uc, !(*skin).weights.data.is_null(), "skin->weights.data");

            let retain_all: bool = !(*uc.get()).opts.clean_skin_weights;

            let skin_vertices: *mut SkinVertex = (*skin).vertices.data as *mut SkinVertex;
            let skin_weights: *mut SkinWeight = (*skin).weights.data as *mut SkinWeight;

            // Count the number of weights per vertex
            // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, skin->clusters)`
            let mut p_cluster: *mut *mut SkinCluster =
                (*skin).clusters.data as *mut *mut SkinCluster;
            let p_cluster_end: *mut *mut SkinCluster = add_ptr(p_cluster, (*skin).clusters.count);
            while p_cluster != p_cluster_end {
                let cluster: *mut SkinCluster = *p_cluster;
                for i in 0..(*cluster).num_weights {
                    let vertex: u32 = *(*cluster).vertices.data.add(i);
                    if (vertex as usize) < num_vertices
                        && (retain_all || *(*cluster).weights.data.add(i) > 0.0)
                    {
                        (*skin_vertices.add(vertex as usize)).num_weights = (*skin_vertices
                            .add(vertex as usize))
                        .num_weights
                        .wrapping_add(1);
                    }
                }
                p_cluster = p_cluster.add(1);
            }

            let default_dq: Real = if (*skin).skinning_method == SkinningMethod::DualQuaternion {
                1.0f32 as Real
            } else {
                0.0f32 as Real
            };

            // Prefix sum to assign the vertex weight offsets and set up default DQ values
            let mut offset: u32 = 0;
            let mut max_weights: u32 = 0;
            for i in 0..num_vertices {
                (*skin_vertices.add(i)).weight_begin = offset;
                (*skin_vertices.add(i)).dq_weight = default_dq;
                let num_weights: u32 = (*skin_vertices.add(i)).num_weights;
                offset = offset.wrapping_add(num_weights);
                (*skin_vertices.add(i)).num_weights = 0;

                if num_weights > max_weights {
                    max_weights = num_weights;
                }
            }
            ufbx_assert!(offset as usize <= total_weights);
            (*skin).max_weights_per_vertex = max_weights as usize;

            // Copy the DQ weights to vertices
            for i in 0..(*skin).num_dq_weights {
                let vertex: u32 = *(*skin).dq_vertices.data.add(i);
                if (vertex as usize) < num_vertices {
                    (*skin_vertices.add(vertex as usize)).dq_weight =
                        *(*skin).dq_weights.data.add(i);
                }
            }

            // Copy the weights to vertices
            let mut cluster_index: u32 = 0;
            // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, skin->clusters)`
            let mut p_cluster: *mut *mut SkinCluster =
                (*skin).clusters.data as *mut *mut SkinCluster;
            let p_cluster_end: *mut *mut SkinCluster = add_ptr(p_cluster, (*skin).clusters.count);
            while p_cluster != p_cluster_end {
                let cluster: *mut SkinCluster = *p_cluster;
                for i in 0..(*cluster).num_weights {
                    let vertex: u32 = *(*cluster).vertices.data.add(i);
                    if (vertex as usize) < num_vertices
                        && (retain_all || *(*cluster).weights.data.add(i) > 0.0)
                    {
                        // C: `skin->vertices.data[vertex].num_weights++` — the
                        // pre-increment value.
                        let local_index: u32 = (*skin_vertices.add(vertex as usize)).num_weights;
                        (*skin_vertices.add(vertex as usize)).num_weights =
                            local_index.wrapping_add(1);
                        let index: u32 = (*skin_vertices.add(vertex as usize))
                            .weight_begin
                            .wrapping_add(local_index);
                        (*skin_weights.add(index as usize)).cluster_index = cluster_index;
                        (*skin_weights.add(index as usize)).weight =
                            *(*cluster).weights.data.add(i);
                    }
                }
                cluster_index = cluster_index.wrapping_add(1);
                p_cluster = p_cluster.add(1);
            }

            // Sort the vertex weights by descending weight value
            sort_skin_weights(uc, skin)?;
        }
        p_skin = p_skin.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_blend_deformer, p_blend, uc->scene.blend_deformers)`
    let mut p_blend: *mut *mut BlendDeformer =
        (*uc.get()).scene.blend_deformers.data as *mut *mut BlendDeformer;
    let p_blend_end: *mut *mut BlendDeformer =
        add_ptr(p_blend, (*uc.get()).scene.blend_deformers.count);
    while p_blend != p_blend_end {
        let blend: *mut BlendDeformer = *p_blend;
        fetch_dst_elements(
            uc,
            ptr::addr_of_mut!((*blend).channels) as *mut c_void,
            &mut (*blend).element,
            false,
            true,
            ptr::null(),
            ElementType::BlendChannel,
        )?;
        p_blend = p_blend.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_cache_deformer, p_deformer, uc->scene.cache_deformers)`
    let mut p_deformer: *mut *mut CacheDeformer =
        (*uc.get()).scene.cache_deformers.data as *mut *mut CacheDeformer;
    let p_deformer_end: *mut *mut CacheDeformer =
        add_ptr(p_deformer, (*uc.get()).scene.cache_deformers.count);
    while p_deformer != p_deformer_end {
        let deformer: *mut CacheDeformer = *p_deformer;
        (*deformer).channel = find_string(
            &(*deformer).element.props,
            b"ChannelName\0".as_ptr(),
            EMPTY_STRING.0,
        );
        (*deformer).file = opt_ref(fetch_dst_element(
            &mut (*deformer).element,
            false,
            ptr::null(),
            ElementType::CacheFile,
        ) as *mut CacheFile);
        p_deformer = p_deformer.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_cache_file, p_cache, uc->scene.cache_files)`
    let mut p_cache: *mut *mut CacheFile =
        (*uc.get()).scene.cache_files.data as *mut *mut CacheFile;
    let p_cache_end: *mut *mut CacheFile = add_ptr(p_cache, (*uc.get()).scene.cache_files.count);
    while p_cache != p_cache_end {
        let cache: *mut CacheFile = *p_cache;

        (*cache).absolute_filename = find_string(
            &(*cache).element.props,
            b"CacheAbsoluteFileName\0".as_ptr(),
            EMPTY_STRING.0,
        );
        (*cache).relative_filename = find_string(
            &(*cache).element.props,
            b"CacheFileName\0".as_ptr(),
            EMPTY_STRING.0,
        );

        (*cache).raw_absolute_filename = find_blob(
            &(*cache).element.props,
            b"CacheAbsoluteFileName\0".as_ptr(),
            EMPTY_BLOB.0,
        );
        (*cache).raw_relative_filename = find_blob(
            &(*cache).element.props,
            b"CacheFileName\0".as_ptr(),
            EMPTY_BLOB.0,
        );

        let type_: i64 = api_find_int(&(*cache).element.props, b"CacheFileType\0".as_ptr(), 0);
        if type_ >= 0 && type_ <= CacheFileFormat::Mc as i64 {
            // C: `(ufbx_cache_file_format)type` — the guard above pins `type`
            // into `0..=UFBX_CACHE_FILE_FORMAT_MC`, exactly the enum range.
            (*cache).format = core::mem::transmute::<u32, CacheFileFormat>(type_ as u32);
        }

        resolve_filenames(
            uc,
            ptr::addr_of_mut!((*cache).filename) as *mut Strblob,
            ptr::addr_of_mut!((*cache).absolute_filename) as *mut Strblob,
            ptr::addr_of_mut!((*cache).relative_filename) as *mut Strblob,
            false,
        )?;
        resolve_filenames(
            uc,
            ptr::addr_of_mut!((*cache).raw_filename) as *mut Strblob,
            ptr::addr_of_mut!((*cache).raw_absolute_filename) as *mut Strblob,
            ptr::addr_of_mut!((*cache).raw_relative_filename) as *mut Strblob,
            true,
        )?;
        p_cache = p_cache.add(1);
    }

    ufbx_assert!((*uc.get()).tmp_full_weights.num_items == (*uc.get()).scene.blend_channels.count);
    // C reads `uc->tmp_full_weights.num_items` as the `ufbxi_push_pop()` count
    // argument; hoisted so the `&mut` borrow does not overlap the read.
    let num_full_weights: usize = (*uc.get()).tmp_full_weights.num_items;
    let mut full_weights: *mut List<Real> = push_pop::<List<Real>>(
        uc.tmp_mut_ptr(),
        uc.tmp_full_weights_mut_ptr(),
        num_full_weights,
    );
    buf_free(uc.tmp_full_weights_mut_ptr());
    ufbxi_check!(uc, !full_weights.is_null(), "full_weights");

    // C: `ufbxi_for_ptr_list(ufbx_blend_channel, p_channel, uc->scene.blend_channels)`
    let mut p_channel: *mut *mut BlendChannel =
        (*uc.get()).scene.blend_channels.data as *mut *mut BlendChannel;
    let p_channel_end: *mut *mut BlendChannel =
        add_ptr(p_channel, (*uc.get()).scene.blend_channels.count);
    while p_channel != p_channel_end {
        let channel: *mut BlendChannel = *p_channel;

        fetch_blend_keyframes(
            uc,
            ptr::addr_of_mut!((*channel).keyframes),
            &mut (*channel).element,
        )?;

        for i in 0..(*channel).keyframes.count {
            let key: *mut BlendKeyframe = ((*channel).keyframes.data as *mut BlendKeyframe).add(i);
            (*key).target_weight = 1.0f32 as Real;
            if i < (*full_weights).count {
                if !uc.blender_full_weights() {
                    (*key).target_weight = *(*full_weights).data.add(i) / 100.0;
                } else if (*full_weights).count == (*ref_ptr(&(*key).shape)).num_offsets {
                    if i == 0 {
                        // Duplicate `index_data` for modification if we retain DOM
                        if (*uc.get()).opts.retain_dom {
                            (*full_weights).data = push_copy::<Real>(
                                uc.result_mut_ptr(),
                                (*full_weights).count,
                                (*full_weights).data,
                            );
                            ufbxi_check!(uc, !(*full_weights).data.is_null(), "full_weights->data");
                        }
                        // C: `ufbxi_for_list(ufbx_real, p_weight, *full_weights)`
                        let mut p_weight: *mut Real = (*full_weights).data as *mut Real;
                        let p_weight_end: *mut Real = add_ptr(p_weight, (*full_weights).count);
                        while p_weight != p_weight_end {
                            *p_weight /= 100.0;
                            p_weight = p_weight.add(1);
                        }
                    }
                    // C: struct assignment (memcpy) of the `ufbx_real_list`
                    // header; `List<T>` is not `Copy` in the generated
                    // bindings, so the copy is a byte-identical
                    // `copy_nonoverlapping`.
                    ptr::copy_nonoverlapping(
                        full_weights as *const List<Real>,
                        ptr::addr_of_mut!((*ref_ptr(&(*key).shape)).offset_weights),
                        1,
                    );
                }
            }
        }

        sort_blend_keyframes(
            uc,
            (*channel).keyframes.data as *mut BlendKeyframe,
            (*channel).keyframes.count,
        )?;
        full_weights = full_weights.add(1);

        if (*channel).keyframes.count > 0 {
            (*channel).target_shape = opt_ref(ref_ptr(
                &(*((*channel).keyframes.data as *mut BlendKeyframe)
                    .add((*channel).keyframes.count - 1))
                .shape,
            ));
        }
        p_channel = p_channel.add(1);
    }

    {
        // Generate and patch procedural index buffers
        let zero_indices: *mut u32 = push::<u32>(uc.result_mut_ptr(), uc.max_zero_indices());
        let consecutive_indices: *mut u32 =
            push::<u32>(uc.result_mut_ptr(), uc.max_consecutive_indices());
        ufbxi_check!(
            uc,
            !zero_indices.is_null() && !consecutive_indices.is_null(),
            "zero_indices && consecutive_indices"
        );

        ptr::write_bytes(zero_indices, 0, uc.max_zero_indices());
        for i in 0..uc.max_consecutive_indices() {
            *consecutive_indices.add(i) = i as u32;
        }

        uc.set_zero_indices(zero_indices);
        uc.set_consecutive_indices(consecutive_indices);

        // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, uc->scene.meshes)`
        let mut p_mesh: *mut *mut Mesh = (*uc.get()).scene.meshes.data as *mut *mut Mesh;
        let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, (*uc.get()).scene.meshes.count);
        while p_mesh != p_mesh_end {
            let mesh: *mut Mesh = *p_mesh;

            patch_index_pointer(
                uc,
                ptr::addr_of_mut!((*mesh).vertex_position.indices.data) as *mut *mut u32,
            );
            patch_index_pointer(
                uc,
                ptr::addr_of_mut!((*mesh).vertex_normal.indices.data) as *mut *mut u32,
            );
            patch_index_pointer(
                uc,
                ptr::addr_of_mut!((*mesh).vertex_color.indices.data) as *mut *mut u32,
            );
            patch_index_pointer(
                uc,
                ptr::addr_of_mut!((*mesh).vertex_crease.indices.data) as *mut *mut u32,
            );
            patch_index_pointer(
                uc,
                ptr::addr_of_mut!((*mesh).face_material.data) as *mut *mut u32,
            );
            patch_index_pointer(
                uc,
                ptr::addr_of_mut!((*mesh).face_group.data) as *mut *mut u32,
            );

            patch_index_pointer(
                uc,
                ptr::addr_of_mut!((*mesh).skinned_position.indices.data) as *mut *mut u32,
            );
            patch_index_pointer(
                uc,
                ptr::addr_of_mut!((*mesh).skinned_normal.indices.data) as *mut *mut u32,
            );

            // C: `ufbxi_for_list(ufbx_uv_set, set, mesh->uv_sets)`
            let mut set: *mut UvSet = (*mesh).uv_sets.data as *mut UvSet;
            let set_end: *mut UvSet = add_ptr(set, (*mesh).uv_sets.count);
            while set != set_end {
                patch_index_pointer(
                    uc,
                    ptr::addr_of_mut!((*set).vertex_uv.indices.data) as *mut *mut u32,
                );
                patch_index_pointer(
                    uc,
                    ptr::addr_of_mut!((*set).vertex_bitangent.indices.data) as *mut *mut u32,
                );
                patch_index_pointer(
                    uc,
                    ptr::addr_of_mut!((*set).vertex_tangent.indices.data) as *mut *mut u32,
                );
                set = set.add(1);
            }

            // C: `ufbxi_for_list(ufbx_color_set, set, mesh->color_sets)`
            let mut cset: *mut ColorSet = (*mesh).color_sets.data as *mut ColorSet;
            let cset_end: *mut ColorSet = add_ptr(cset, (*mesh).color_sets.count);
            while cset != cset_end {
                patch_index_pointer(
                    uc,
                    ptr::addr_of_mut!((*cset).vertex_color.indices.data) as *mut *mut u32,
                );
                cset = cset.add(1);
            }

            // Generate normals if necessary
            if !(*mesh).vertex_normal.exists && (*uc.get()).opts.generate_missing_normals {
                generate_normals(uc, mesh)?;
            }

            // Assign first UV and color sets as the "canonical" ones
            if (*mesh).uv_sets.count > 0 {
                // C: struct assignment (memcpy) of the vertex-attribute
                // headers; the `Vertex*` structs are not `Copy` in the
                // generated bindings, so the copy is spelled as a
                // byte-identical `copy_nonoverlapping`.
                ptr::copy_nonoverlapping(
                    ptr::addr_of!((*((*mesh).uv_sets.data as *mut UvSet).add(0)).vertex_uv),
                    ptr::addr_of_mut!((*mesh).vertex_uv),
                    1,
                );
                ptr::copy_nonoverlapping(
                    ptr::addr_of!((*((*mesh).uv_sets.data as *mut UvSet).add(0)).vertex_bitangent),
                    ptr::addr_of_mut!((*mesh).vertex_bitangent),
                    1,
                );
                ptr::copy_nonoverlapping(
                    ptr::addr_of!((*((*mesh).uv_sets.data as *mut UvSet).add(0)).vertex_tangent),
                    ptr::addr_of_mut!((*mesh).vertex_tangent),
                    1,
                );
            }
            if (*mesh).color_sets.count > 0 {
                ptr::copy_nonoverlapping(
                    ptr::addr_of!(
                        (*((*mesh).color_sets.data as *mut ColorSet).add(0)).vertex_color
                    ),
                    ptr::addr_of_mut!((*mesh).vertex_color),
                    1,
                );
            }

            if (*mesh).face_group_parts.count == 1 {
                patch_index_pointer(
                    uc,
                    ptr::addr_of_mut!(
                        (*((*mesh).face_group_parts.data as *mut MeshPart).add(0))
                            .face_indices
                            .data
                    ) as *mut *mut u32,
                );
            }

            fetch_mesh_materials(
                uc,
                ptr::addr_of_mut!((*mesh).materials),
                &mut (*mesh).element,
                true,
            )?;

            // Patch materials to instances if necessary
            if (*mesh).materials.count > 0 {
                // C: `ufbxi_for_ptr_list(ufbx_node, p_node, mesh->instances)`
                let mut p_node: *mut *mut Node = (*mesh).element.instances.data as *mut *mut Node;
                let p_node_end: *mut *mut Node = add_ptr(p_node, (*mesh).element.instances.count);
                while p_node != p_node_end {
                    let node: *mut Node = *p_node;
                    // C-parity: `mesh->materials.data[0]` may be NULL (broken
                    // element connections), so the entry is read as the bare
                    // `ufbx_material*` the `Ref` field is at the ABI level.
                    let mesh_materials: *mut *mut Material =
                        (*mesh).materials.data as *mut *mut Material;
                    if (*node).materials.count < (*mesh).materials.count
                        && !(*mesh_materials.add(0)).is_null()
                    {
                        let materials: *mut *mut Material =
                            push::<*mut Material>(uc.result_mut_ptr(), (*mesh).materials.count);
                        ufbxi_check!(uc, !materials.is_null(), "materials");
                        // C: `ufbxi_nounroll for (...)` — the no-unroll pragma
                        // is optimizer-only and has no Rust analogue.
                        for i in 0..(*node).materials.count {
                            *materials.add(i) =
                                *((*node).materials.data as *mut *mut Material).add(i);
                        }
                        for i in (*node).materials.count..(*mesh).materials.count {
                            *materials.add(i) = *mesh_materials.add(i);
                        }
                        (*node).materials.data = materials as *const Ref<Material>;
                        (*node).materials.count = (*mesh).materials.count;
                    }
                    p_node = p_node.add(1);
                }
            }

            if uc.retain_mesh_parts() {
                let num_parts: usize = max_sz((*mesh).materials.count, 1);
                (*mesh).material_parts.data = push_zero::<MeshPart>(uc.result_mut_ptr(), num_parts);
                ufbxi_check!(
                    uc,
                    !(*mesh).material_parts.data.is_null(),
                    "mesh->material_parts.data"
                );
                (*mesh).material_parts.count = num_parts;
            }

            if (*mesh).materials.count <= 1 {
                // Use the shared consecutive index buffer for mesh faces if there's only one material
                // See HACK(consecutive-faces) in `ufbxi_read_mesh()`.
                if (*mesh).material_parts.count > 0 {
                    let part: *mut MeshPart = ((*mesh).material_parts.data as *mut MeshPart).add(0);
                    (*part).num_faces = (*mesh).num_faces;
                    (*part).num_triangles = (*mesh).num_triangles;
                    (*part).num_empty_faces = (*mesh).num_empty_faces;
                    (*part).num_point_faces = (*mesh).num_point_faces;
                    (*part).num_line_faces = (*mesh).num_line_faces;
                    (*part).face_indices.data = uc.consecutive_indices();
                    (*part).face_indices.count = (*mesh).num_faces;
                    (*mesh).material_part_usage_order.data = uc.zero_indices();
                    (*mesh).material_part_usage_order.count = 1;
                }

                if (*mesh).materials.count == 1 {
                    (*mesh).face_material.data = uc.zero_indices();
                    (*mesh).face_material.count = (*mesh).num_faces;
                } else {
                    (*mesh).face_material.data = ptr::null_mut();
                    (*mesh).face_material.count = 0;
                }
            } else if (*mesh).materials.count > 0 {
                finalize_mesh_material(uc.result_mut_ptr(), uc.error_mut_ptr(), mesh)?;
            }

            // Fetch deformers
            fetch_dst_elements(
                uc,
                ptr::addr_of_mut!((*mesh).skin_deformers) as *mut c_void,
                &mut (*mesh).element,
                search_node,
                true,
                ptr::null(),
                ElementType::SkinDeformer,
            )?;
            fetch_dst_elements(
                uc,
                ptr::addr_of_mut!((*mesh).blend_deformers) as *mut c_void,
                &mut (*mesh).element,
                search_node,
                true,
                ptr::null(),
                ElementType::BlendDeformer,
            )?;
            fetch_dst_elements(
                uc,
                ptr::addr_of_mut!((*mesh).cache_deformers) as *mut c_void,
                &mut (*mesh).element,
                search_node,
                true,
                ptr::null(),
                ElementType::CacheDeformer,
            )?;
            fetch_deformers(
                uc,
                ptr::addr_of_mut!((*mesh).all_deformers),
                &mut (*mesh).element,
                search_node,
            )?;

            // Vertex position must always exist if not explicitly allowed to be missing
            if !(*mesh).vertex_position.exists && !(*uc.get()).opts.allow_missing_vertex_position {
                ufbxi_check!(uc, (*mesh).num_indices == 0, "mesh->num_indices == 0");
                (*mesh).vertex_position.exists = true;
                (*mesh).vertex_position.unique_per_vertex = true;
                (*mesh).skinned_position.exists = true;
                (*mesh).skinned_position.unique_per_vertex = true;
            }

            // Update metadata
            if (*mesh).max_face_triangles > (*uc.get()).scene.metadata.max_face_triangles {
                (*uc.get()).scene.metadata.max_face_triangles = (*mesh).max_face_triangles;
            }
            p_mesh = p_mesh.add(1);
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_stereo_camera, p_stereo, uc->scene.stereo_cameras)`
    let mut p_stereo: *mut *mut StereoCamera =
        (*uc.get()).scene.stereo_cameras.data as *mut *mut StereoCamera;
    let p_stereo_end: *mut *mut StereoCamera =
        add_ptr(p_stereo, (*uc.get()).scene.stereo_cameras.count);
    while p_stereo != p_stereo_end {
        let stereo: *mut StereoCamera = *p_stereo;
        (*stereo).left = opt_ref(fetch_dst_element(
            &mut (*stereo).element,
            search_node,
            sp::LeftCamera.as_ptr(),
            ElementType::Camera,
        ) as *mut Camera);
        (*stereo).right = opt_ref(fetch_dst_element(
            &mut (*stereo).element,
            search_node,
            sp::RightCamera.as_ptr(),
            ElementType::Camera,
        ) as *mut Camera);
        p_stereo = p_stereo.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_nurbs_curve, p_curve, uc->scene.nurbs_curves)`
    let mut p_nurbs_curve: *mut *mut NurbsCurve =
        (*uc.get()).scene.nurbs_curves.data as *mut *mut NurbsCurve;
    let p_nurbs_curve_end: *mut *mut NurbsCurve =
        add_ptr(p_nurbs_curve, (*uc.get()).scene.nurbs_curves.count);
    while p_nurbs_curve != p_nurbs_curve_end {
        let curve: *mut NurbsCurve = *p_nurbs_curve;
        finalize_nurbs_basis(uc, ptr::addr_of_mut!((*curve).basis))?;
        p_nurbs_curve = p_nurbs_curve.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_nurbs_surface, p_surface, uc->scene.nurbs_surfaces)`
    let mut p_surface: *mut *mut NurbsSurface =
        (*uc.get()).scene.nurbs_surfaces.data as *mut *mut NurbsSurface;
    let p_surface_end: *mut *mut NurbsSurface =
        add_ptr(p_surface, (*uc.get()).scene.nurbs_surfaces.count);
    while p_surface != p_surface_end {
        let surface: *mut NurbsSurface = *p_surface;
        finalize_nurbs_basis(uc, ptr::addr_of_mut!((*surface).basis_u))?;
        finalize_nurbs_basis(uc, ptr::addr_of_mut!((*surface).basis_v))?;

        (*surface).material = opt_ref(fetch_dst_element(
            &mut (*surface).element,
            true,
            ptr::null(),
            ElementType::Material,
        ) as *mut Material);
        p_surface = p_surface.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_stack, p_stack, uc->scene.anim_stacks)`
    let mut p_stack: *mut *mut AnimStack =
        (*uc.get()).scene.anim_stacks.data as *mut *mut AnimStack;
    let p_stack_end: *mut *mut AnimStack = add_ptr(p_stack, (*uc.get()).scene.anim_stacks.count);
    while p_stack != p_stack_end {
        let stack: *mut AnimStack = *p_stack;
        fetch_dst_elements(
            uc,
            ptr::addr_of_mut!((*stack).layers) as *mut c_void,
            &mut (*stack).element,
            false,
            true,
            ptr::null(),
            ElementType::AnimLayer,
        )?;

        push_anim(
            uc,
            ptr::addr_of_mut!((*stack).anim) as *mut *mut Anim,
            (*stack).layers.data as *mut *mut AnimLayer,
            (*stack).layers.count,
        )?;
        p_stack = p_stack.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_layer, p_layer, uc->scene.anim_layers)`
    let mut p_layer: *mut *mut AnimLayer =
        (*uc.get()).scene.anim_layers.data as *mut *mut AnimLayer;
    let p_layer_end: *mut *mut AnimLayer = add_ptr(p_layer, (*uc.get()).scene.anim_layers.count);
    while p_layer != p_layer_end {
        let layer: *mut AnimLayer = *p_layer;
        fetch_dst_elements(
            uc,
            ptr::addr_of_mut!((*layer).anim_values) as *mut c_void,
            &mut (*layer).element,
            false,
            true,
            ptr::null(),
            ElementType::AnimValue,
        )?;

        push_anim(
            uc,
            ptr::addr_of_mut!((*layer).anim) as *mut *mut Anim,
            p_layer,
            1,
        )?;

        let mut min_id: u32 = u32::MAX;
        let mut max_id: u32 = 0;

        // Combine the animated properties with elements (potentially duplicates!)
        let mut num_anim_props: usize = 0;
        // C: `ufbxi_for_ptr_list(ufbx_anim_value, p_value, layer->anim_values)`
        let mut p_value: *mut *mut AnimValue = (*layer).anim_values.data as *mut *mut AnimValue;
        let p_value_end: *mut *mut AnimValue = add_ptr(p_value, (*layer).anim_values.count);
        while p_value != p_value_end {
            let value: *mut AnimValue = *p_value;
            // C: `ufbxi_for_list(ufbx_connection, ac, value->element.connections_src)`
            let mut ac: *mut Connection = (*value).element.connections_src.data as *mut Connection;
            let ac_end: *mut Connection = add_ptr(ac, (*value).element.connections_src.count);
            while ac != ac_end {
                if (*ac).src_prop.length == 0 && (*ac).dst_prop.length > 0 {
                    let aprop: *mut AnimProp = push::<AnimProp>(uc.tmp_stack_mut_ptr(), 1);
                    let id: u32 = (*ref_ptr(&(*ac).dst)).element_id;
                    min_id = min32(min_id, id);
                    max_id = max32(max_id, id);
                    // C: `ufbxi_arraycount(layer->_element_id_bitmask) - 1`
                    let id_mask: u32 = (*layer)._element_id_bitmask.len() as u32 - 1;
                    (*layer)._element_id_bitmask[((id >> 5) & id_mask) as usize] |=
                        1u32 << (id & 31);
                    ufbxi_check!(uc, !aprop.is_null(), "aprop");
                    (*aprop).anim_value = Ref::from_ptr(value);
                    (*aprop).element = Ref::from_ptr(ref_ptr(&(*ac).dst));
                    (*aprop)._internal_key =
                        get_name_key((*ac).dst_prop.data, (*ac).dst_prop.length);
                    (*aprop).prop_name = (*ac).dst_prop;
                    num_anim_props += 1;
                }
                ac = ac.add(1);
            }
            p_value = p_value.add(1);
        }

        if min_id != u32::MAX {
            (*layer)._min_element_id = min_id;
            (*layer)._max_element_id = max_id;
        }

        match find_int(&(*layer).element.props, sp::BlendMode.as_ptr(), 0) {
            0 => {
                // Additive
                (*layer).blended = true;
                (*layer).additive = true;
            }
            1 => {
                // Override
                (*layer).blended = false;
                (*layer).additive = false;
            }
            2 => {
                // Override Passthrough
                (*layer).blended = true;
                (*layer).additive = false;
            }
            _ => {
                // Unknown
                (*layer).blended = false;
                (*layer).additive = false;
            }
        }

        let weight_prop: *mut Prop = find_prop(&(*layer).element.props, sp::Weight.as_ptr());
        if !weight_prop.is_null() {
            // C-parity: `prop->value_real` is the `ufbx_prop` value union's
            // first real; the generated struct keeps only `value_vec4`.
            (*layer).weight = (*weight_prop).value_vec4.x / 100.0;
            if (*layer).weight < 0.0f32 as Real {
                (*layer).weight = 0.0f32 as Real;
            }
            // C: `0.99999f` — a `float` literal promoted to `ufbx_real`, NOT
            // the double 0.99999.
            if (*layer).weight > 0.99999f32 as Real {
                (*layer).weight = 1.0f32 as Real;
            }
            (*layer).weight_is_animated =
                ((*weight_prop).flags.raw() & PropFlags::ANIMATED.raw()) != 0;
        } else {
            (*layer).weight = 1.0f32 as Real;
            (*layer).weight_is_animated = false;
        }
        (*layer).compose_rotation = find_int(
            &(*layer).element.props,
            sp::RotationAccumulationMode.as_ptr(),
            0,
        ) == 0;
        (*layer).compose_scale = find_int(
            &(*layer).element.props,
            sp::ScaleAccumulationMode.as_ptr(),
            0,
        ) == 0;

        // Add a dummy NULL element animated prop at the end so we can iterate
        // animated props without worrying about boundary conditions..
        {
            let aprop: *mut AnimProp = push_zero::<AnimProp>(uc.tmp_stack_mut_ptr(), 1);
            ufbxi_check!(uc, !aprop.is_null(), "aprop");
        }

        (*layer).anim_props.data = push_pop::<AnimProp>(
            uc.result_mut_ptr(),
            uc.tmp_stack_mut_ptr(),
            num_anim_props + 1,
        );
        ufbxi_check!(
            uc,
            !(*layer).anim_props.data.is_null(),
            "layer->anim_props.data"
        );
        (*layer).anim_props.count = num_anim_props;
        sort_anim_props(
            uc,
            (*layer).anim_props.data as *mut AnimProp,
            (*layer).anim_props.count,
        )?;
        p_layer = p_layer.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_value, p_value, uc->scene.anim_values)`
    let mut p_value: *mut *mut AnimValue =
        (*uc.get()).scene.anim_values.data as *mut *mut AnimValue;
    let p_value_end: *mut *mut AnimValue = add_ptr(p_value, (*uc.get()).scene.anim_values.count);
    while p_value != p_value_end {
        let value: *mut AnimValue = *p_value;

        // TODO: Search for things like d|Visibility with a constructed name
        (*value).default_value.x = find_real(
            &(*value).element.props,
            sp::X.as_ptr(),
            (*value).default_value.x,
        );
        (*value).default_value.x = find_real(
            &(*value).element.props,
            sp::d_X.as_ptr(),
            (*value).default_value.x,
        );
        (*value).default_value.y = find_real(
            &(*value).element.props,
            sp::Y.as_ptr(),
            (*value).default_value.y,
        );
        (*value).default_value.y = find_real(
            &(*value).element.props,
            sp::d_Y.as_ptr(),
            (*value).default_value.y,
        );
        (*value).default_value.z = find_real(
            &(*value).element.props,
            sp::Z.as_ptr(),
            (*value).default_value.z,
        );
        (*value).default_value.z = find_real(
            &(*value).element.props,
            sp::d_Z.as_ptr(),
            (*value).default_value.z,
        );

        // C: `ufbxi_for_list(ufbx_connection, conn, value->element.connections_dst)`
        let mut conn: *mut Connection = (*value).element.connections_dst.data as *mut Connection;
        let conn_end: *mut Connection = add_ptr(conn, (*value).element.connections_dst.count);
        while conn != conn_end {
            if (*ref_ptr(&(*conn).src)).type_ == ElementType::AnimCurve
                && (*conn).src_prop.length == 0
            {
                let curve: *mut AnimCurve = ref_ptr(&(*conn).src) as *mut AnimCurve;

                let mut index: u32 = 0;
                let name: *const u8 = (*conn).dst_prop.data;
                if name == sp::Y.as_ptr() || name == sp::d_Y.as_ptr() {
                    index = 1;
                }
                if name == sp::Z.as_ptr() || name == sp::d_Z.as_ptr() {
                    index = 2;
                }

                let prop: *mut Prop = find_prop_len(
                    &(*value).element.props,
                    (*conn).dst_prop.data,
                    (*conn).dst_prop.length,
                );
                if !prop.is_null() {
                    // C indexes the `ufbx_vec3` value union's `ufbx_real v[3]`
                    // view; the generated struct keeps only `x`/`y`/`z`, so the
                    // index is pointer arithmetic from the struct base.
                    let v: *mut Real = ptr::addr_of_mut!((*value).default_value) as *mut Real;
                    *v.add(index as usize) = (*prop).value_vec4.x;
                }
                (*value).curves[index as usize] = opt_ref(curve);
            }
            conn = conn.add(1);
        }
        p_value = p_value.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_curve, p_curve, uc->scene.anim_curves)`
    let mut p_anim_curve: *mut *mut AnimCurve =
        (*uc.get()).scene.anim_curves.data as *mut *mut AnimCurve;
    let p_anim_curve_end: *mut *mut AnimCurve =
        add_ptr(p_anim_curve, (*uc.get()).scene.anim_curves.count);
    while p_anim_curve != p_anim_curve_end {
        let curve: *mut AnimCurve = *p_anim_curve;
        if (*curve).keyframes.count > 0 {
            (*curve).min_time = (*(*curve).keyframes.data.add(0)).time;
            (*curve).max_time = (*(*curve).keyframes.data.add((*curve).keyframes.count - 1)).time;
        }
        p_anim_curve = p_anim_curve.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_shader, p_shader, uc->scene.shaders)`
    let mut p_shader: *mut *mut Shader = (*uc.get()).scene.shaders.data as *mut *mut Shader;
    let p_shader_end: *mut *mut Shader = add_ptr(p_shader, (*uc.get()).scene.shaders.count);
    while p_shader != p_shader_end {
        let shader: *mut Shader = *p_shader;
        fetch_dst_elements(
            uc,
            ptr::addr_of_mut!((*shader).bindings) as *mut c_void,
            &mut (*shader).element,
            false,
            false,
            ptr::null(),
            ElementType::ShaderBinding,
        )?;

        let api: *mut Prop = api_find_prop(&(*shader).element.props, b"RenderAPI\0".as_ptr());
        if !api.is_null() {
            if strcmp((*api).value_str.data, b"ARNOLD_SHADER_ID\0".as_ptr()) == 0 {
                (*shader).type_ = ShaderType::ArnoldStandardSurface;
            } else if strcmp((*api).value_str.data, b"OSL\0".as_ptr()) == 0 {
                (*shader).type_ = ShaderType::OslStandardSurface;
            } else if strcmp((*api).value_str.data, b"SFX_PBS_SHADER\0".as_ptr()) == 0 {
                (*shader).type_ = ShaderType::ShaderfxGraph;
            }
        }
        p_shader = p_shader.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_material, p_material, uc->scene.materials)`
    let mut p_material: *mut *mut Material = (*uc.get()).scene.materials.data as *mut *mut Material;
    let p_material_end: *mut *mut Material = add_ptr(p_material, (*uc.get()).scene.materials.count);
    while p_material != p_material_end {
        let material: *mut Material = *p_material;
        (*material).shader = opt_ref(fetch_src_element(
            &mut (*material).element,
            false,
            ptr::null(),
            ElementType::Shader,
        ) as *mut Shader);

        if strcmp((*material).shading_model_name.data, b"lambert\0".as_ptr()) == 0
            || strcmp((*material).shading_model_name.data, b"Lambert\0".as_ptr()) == 0
        {
            (*material).shader_type = ShaderType::FbxLambert;
        } else if strcmp((*material).shading_model_name.data, b"phong\0".as_ptr()) == 0
            || strcmp((*material).shading_model_name.data, b"Phong\0".as_ptr()) == 0
        {
            (*material).shader_type = ShaderType::FbxPhong;
        }

        let material_shader: *mut Shader = opt_ptr(&(*material).shader);
        if !material_shader.is_null() {
            (*material).shader_type = (*material_shader).type_;
        } else {
            if (*uc.get()).opts.use_blender_pbr_material
                && (*uc.get()).exporter == Exporter::BlenderBinary
                && uc.exporter_version() >= pack_version(4, 12, 0)
            {
                (*material).shader_type = ShaderType::BlenderPhong;
            }

            // TODO: Is this too strict?
            if (*material).shader_type == ShaderType::Unknown {
                let classid_a: u32 =
                    api_find_int(&(*material).element.props, b"3dsMax|ClassIDa\0".as_ptr(), 0)
                        as u64 as u32;
                let classid_b: u32 =
                    api_find_int(&(*material).element.props, b"3dsMax|ClassIDb\0".as_ptr(), 0)
                        as u64 as u32;
                if classid_a == 0x3d6b1cecu32 && classid_b == 0xdeadc001u32 {
                    (*material).shader_type = ShaderType::E3DsMaxPhysicalMaterial;
                    (*material).shader_prop_prefix = ufbxi_string_literal!(b"3dsMax|Parameters|\0");
                } else if classid_a == 0xf1551e33u32 && classid_b == 0x37fb1337u32 {
                    (*material).shader_type = ShaderType::OpenpbrMaterial;
                    (*material).shader_prop_prefix = ufbxi_string_literal!(b"3dsMax|Parameters|\0");
                } else if classid_a == 0x38420192u32 && classid_b == 0x45fe4e1bu32 {
                    (*material).shader_type = ShaderType::GltfMaterial;
                    (*material).shader_prop_prefix = ufbxi_string_literal!(b"3dsMax|\0");
                } else if classid_a == 0xd00f1e00u32 && classid_b == 0xbe77e500u32 {
                    (*material).shader_type = ShaderType::E3DsMaxPbrMetalRough;
                    (*material).shader_prop_prefix = ufbxi_string_literal!(b"3dsMax|main|\0");
                } else if classid_a == 0xd00f1e00u32 && classid_b == 0x01dbad33u32 {
                    (*material).shader_type = ShaderType::E3DsMaxPbrSpecGloss;
                    (*material).shader_prop_prefix = ufbxi_string_literal!(b"3dsMax|main|\0");
                }
            }
        }

        fetch_textures(
            uc,
            ptr::addr_of_mut!((*material).textures),
            &mut (*material).element,
            false,
        )?;
        p_material = p_material.add(1);
    }

    // Ugh.. Patch the textures from meshes for legacy LayerElement-style textures
    {
        // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, uc->scene.meshes)`
        let mut p_mesh: *mut *mut Mesh = (*uc.get()).scene.meshes.data as *mut *mut Mesh;
        let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, (*uc.get()).scene.meshes.count);
        while p_mesh != p_mesh_end {
            let mesh: *mut Mesh = *p_mesh;
            let num_materials: usize = (*mesh).materials.count;

            let extra: *mut MeshExtra =
                get_element_extra(uc, (*mesh).element.element_id) as *mut MeshExtra;
            if extra.is_null() {
                p_mesh = p_mesh.add(1);
                continue;
            }
            if num_materials == 0 {
                p_mesh = p_mesh.add(1);
                continue;
            }

            // TODO: This leaks currently to result, probably doesn't matter..
            // C: `ufbx_texture_list textures;` — uninitialized local (no
            // upstream `// ufbxi_uninit` marker); `ufbxi_fetch_dst_elements`
            // writes both fields before the first read.
            let mut textures_storage = MaybeUninit::<RefList<Texture>>::uninit();
            let textures: *mut RefList<Texture> = textures_storage.as_mut_ptr();
            fetch_dst_elements(
                uc,
                textures as *mut c_void,
                &mut (*mesh).element,
                true,
                false,
                ptr::null(),
                ElementType::Texture,
            )?;

            let mut num_material_textures: usize = 0;
            // C: `ufbxi_for(ufbxi_tmp_mesh_texture, tex, extra->texture_arr, extra->texture_count)`
            let mut tex: *mut TmpMeshTexture = (*extra).texture_arr;
            let tex_end: *mut TmpMeshTexture = add_ptr(tex, (*extra).texture_count);
            while tex != tex_end {
                if (*tex).all_same {
                    let texture_id: i32 = if (*tex).num_faces > 0 {
                        *(*tex).face_texture.add(0) as i32
                    } else {
                        0
                    };
                    if texture_id >= 0 && (texture_id as usize) < (*textures).count {
                        let mat_texs: *mut TmpMaterialTexture =
                            push::<TmpMaterialTexture>(uc.tmp_stack_mut_ptr(), num_materials);
                        ufbxi_check!(uc, !mat_texs.is_null(), "mat_texs");
                        num_material_textures += num_materials;
                        for i in 0..num_materials {
                            (*mat_texs.add(i)).material_id = i as i32;
                            (*mat_texs.add(i)).texture_id = texture_id;
                            (*mat_texs.add(i)).prop_name = (*tex).prop_name;
                        }
                    }
                } else if (*mesh).face_material.count != 0 {
                    let num_faces: usize = min_sz((*tex).num_faces, (*mesh).num_faces);
                    let mut prev_material: i32 = -1;
                    let mut prev_texture: i32 = -1;
                    for i in 0..num_faces {
                        let texture_id: i32 = *(*tex).face_texture.add(i) as i32;
                        let material_id: i32 = *(*mesh).face_material.data.add(i) as i32;
                        if texture_id < 0 || (texture_id as usize) >= (*textures).count {
                            continue;
                        }
                        if material_id < 0 || (material_id as usize) >= num_materials {
                            continue;
                        }
                        if material_id == prev_material && texture_id == prev_texture {
                            continue;
                        }
                        prev_material = material_id;
                        prev_texture = texture_id;

                        let mat_tex: *mut TmpMaterialTexture =
                            push::<TmpMaterialTexture>(uc.tmp_stack_mut_ptr(), 1);
                        ufbxi_check!(uc, !mat_tex.is_null(), "mat_tex");
                        (*mat_tex).material_id = material_id;
                        (*mat_tex).texture_id = texture_id;
                        (*mat_tex).prop_name = (*tex).prop_name;
                        num_material_textures += 1;
                    }
                }
                tex = tex.add(1);
            }

            // Push a sentinel material texture to the end so we don't need to
            // duplicate the material texture flushing code twice.
            {
                let mat_tex: *mut TmpMaterialTexture =
                    push::<TmpMaterialTexture>(uc.tmp_stack_mut_ptr(), 1);
                ufbxi_check!(uc, !mat_tex.is_null(), "mat_tex");
                (*mat_tex).material_id = -1;
                (*mat_tex).texture_id = -1;
                (*mat_tex).prop_name = EMPTY_STRING.0;
            }

            let mat_texs: *mut TmpMaterialTexture = push_pop::<TmpMaterialTexture>(
                uc.tmp_mut_ptr(),
                uc.tmp_stack_mut_ptr(),
                num_material_textures + 1,
            );
            ufbxi_check!(uc, !mat_texs.is_null(), "mat_texs");
            sort_tmp_material_textures(uc, mat_texs, num_material_textures)?;

            let mut prev_material: i32 = -2;
            let mut prev_texture: i32 = -2;
            let mut prev_prop: *const u8 = ptr::null();
            let mut num_textures_in_material: usize = 0;
            for i in 0..num_material_textures + 1 {
                let mat_tex: TmpMaterialTexture = *mat_texs.add(i);
                if mat_tex.material_id != prev_material {
                    if prev_material >= 0 && num_textures_in_material > 0 {
                        let mat: *mut Material = *((*mesh).materials.data as *mut *mut Material)
                            .add(prev_material as usize);
                        if !mat.is_null() && (*mat).textures.count == 0 {
                            let texs: *mut MaterialTexture = push_pop::<MaterialTexture>(
                                uc.result_mut_ptr(),
                                uc.tmp_stack_mut_ptr(),
                                num_textures_in_material,
                            );
                            ufbxi_check!(uc, !texs.is_null(), "texs");
                            (*mat).textures.data = texs;
                            (*mat).textures.count = num_textures_in_material;
                        } else {
                            pop::<MaterialTexture>(
                                uc.tmp_stack_mut_ptr(),
                                num_textures_in_material,
                                ptr::null_mut(),
                            );
                        }
                    }

                    if mat_tex.material_id < 0 {
                        break;
                    }
                    prev_material = mat_tex.material_id;
                    prev_texture = -1;
                    prev_prop = ptr::null();
                    num_textures_in_material = 0;
                }
                if mat_tex.texture_id == prev_texture && mat_tex.prop_name.data == prev_prop {
                    continue;
                }
                prev_texture = mat_tex.texture_id;
                prev_prop = mat_tex.prop_name.data;

                let tex: *mut MaterialTexture = push::<MaterialTexture>(uc.tmp_stack_mut_ptr(), 1);
                ufbxi_check!(uc, !tex.is_null(), "tex");
                ufbx_assert!(prev_texture >= 0 && (prev_texture as usize) < (*textures).count);
                (*tex).texture = *(*textures).data.add(prev_texture as usize);
                // C: `tex->shader_prop = tex->material_prop = mat_tex.prop_name;`
                (*tex).material_prop = mat_tex.prop_name;
                (*tex).shader_prop = (*tex).material_prop;
                num_textures_in_material += 1;
            }
            p_mesh = p_mesh.add(1);
        }
    }

    resolve_file_content(uc)?;

    // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, uc->scene.textures)`
    let mut p_texture: *mut *mut Texture = (*uc.get()).scene.textures.data as *mut *mut Texture;
    let p_texture_end: *mut *mut Texture = add_ptr(p_texture, (*uc.get()).scene.textures.count);
    while p_texture != p_texture_end {
        let texture: *mut Texture = *p_texture;
        let extra: *mut TextureExtra =
            get_element_extra(uc, (*texture).element.element_id) as *mut TextureExtra;

        let uv_set: *mut Prop = find_prop(&(*texture).element.props, sp::UVSet.as_ptr());
        if !uv_set.is_null() {
            (*texture).uv_set = (*uv_set).value_str;
        } else {
            (*texture).uv_set = EMPTY_STRING.0;
        }

        (*texture).video = opt_ref(fetch_dst_element(
            &mut (*texture).element,
            false,
            ptr::null(),
            ElementType::Video,
        ) as *mut Video);
        let texture_video: *mut Video = opt_ptr(&(*texture).video);
        if !texture_video.is_null() {
            (*texture).content = (*texture_video).content;
        }

        finalize_shader_texture(uc, texture)?;

        resolve_filenames(
            uc,
            ptr::addr_of_mut!((*texture).filename) as *mut Strblob,
            ptr::addr_of_mut!((*texture).absolute_filename) as *mut Strblob,
            ptr::addr_of_mut!((*texture).relative_filename) as *mut Strblob,
            false,
        )?;
        resolve_filenames(
            uc,
            ptr::addr_of_mut!((*texture).raw_filename) as *mut Strblob,
            ptr::addr_of_mut!((*texture).raw_absolute_filename) as *mut Strblob,
            ptr::addr_of_mut!((*texture).raw_relative_filename) as *mut Strblob,
            true,
        )?;

        // Fetch layered texture layers and patch alphas/blend modes
        if (*texture).type_ == TextureType::Layered {
            fetch_texture_layers(
                uc,
                ptr::addr_of_mut!((*texture).layers),
                &mut (*texture).element,
            )?;
            if !extra.is_null() {
                let num: usize = min_sz((*extra).num_alphas, (*texture).layers.count);
                for i in 0..num {
                    (*((*texture).layers.data as *mut TextureLayer).add(i)).alpha =
                        *(*extra).alphas.add(i);
                }
                let num: usize = min_sz((*extra).num_blend_modes, (*texture).layers.count);
                for i in 0..num {
                    let mode: i32 = *(*extra).blend_modes.add(i);
                    if mode >= 0 && mode < BlendMode::Overlay as i32 {
                        // C: `(ufbx_blend_mode)mode` — the guard above pins
                        // `mode` into the enum's range.
                        (*((*texture).layers.data as *mut TextureLayer).add(i)).blend_mode =
                            core::mem::transmute::<u32, BlendMode>(mode as u32);
                    }
                }
            }
        }

        insert_texture_file(uc, texture)?;
        p_texture = p_texture.add(1);
    }

    propagate_main_textures(&mut (*uc.get()).scene);
    pop_texture_files(uc)?;

    // Second pass to fetch material maps
    // C: `ufbxi_for_ptr_list(ufbx_material, p_material, uc->scene.materials)`
    let mut p_material: *mut *mut Material = (*uc.get()).scene.materials.data as *mut *mut Material;
    let p_material_end: *mut *mut Material = add_ptr(p_material, (*uc.get()).scene.materials.count);
    while p_material != p_material_end {
        let material: *mut Material = *p_material;

        sort_material_textures(
            uc,
            (*material).textures.data as *mut MaterialTexture,
            (*material).textures.count,
        )?;
        fetch_maps(&mut (*uc.get()).scene, material);

        // Fetch `ufbx_material_texture.shader_prop` names
        let material_shader: *mut Shader = opt_ptr(&(*material).shader);
        if !material_shader.is_null() {
            // C: `ufbxi_for_ptr_list(ufbx_shader_binding, p_binding, material->shader->bindings)`
            let mut p_binding: *mut *mut ShaderBinding =
                (*material_shader).bindings.data as *mut *mut ShaderBinding;
            let p_binding_end: *mut *mut ShaderBinding =
                add_ptr(p_binding, (*material_shader).bindings.count);
            while p_binding != p_binding_end {
                let binding: *mut ShaderBinding = *p_binding;

                // C: `ufbxi_for_list(ufbx_shader_prop_binding, prop, binding->prop_bindings)`
                let mut prop: *mut ShaderPropBinding =
                    (*binding).prop_bindings.data as *mut ShaderPropBinding;
                let prop_end: *mut ShaderPropBinding =
                    add_ptr(prop, (*binding).prop_bindings.count);
                while prop != prop_end {
                    let name: String = (*prop).material_prop;

                    let mut index: usize = usize::MAX;
                    macro_lower_bound_eq::<MaterialTexture>(
                        4,
                        &mut index,
                        (*material).textures.data,
                        0,
                        (*material).textures.count,
                        |a| str_less((*a).material_prop, name),
                        |a| (*a).material_prop.data == name.data,
                    );
                    while index < (*material).textures.count
                        && (*((*material).textures.data as *mut MaterialTexture).add(index))
                            .shader_prop
                            .data
                            == name.data
                    {
                        (*((*material).textures.data as *mut MaterialTexture).add(index))
                            .shader_prop = (*prop).shader_prop;
                        index += 1;
                    }
                    prop = prop.add(1);
                }
                p_binding = p_binding.add(1);
            }
        }
        p_material = p_material.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_display_layer, p_layer, uc->scene.display_layers)`
    let mut p_display_layer: *mut *mut DisplayLayer =
        (*uc.get()).scene.display_layers.data as *mut *mut DisplayLayer;
    let p_display_layer_end: *mut *mut DisplayLayer =
        add_ptr(p_display_layer, (*uc.get()).scene.display_layers.count);
    while p_display_layer != p_display_layer_end {
        let layer: *mut DisplayLayer = *p_display_layer;
        fetch_dst_elements(
            uc,
            ptr::addr_of_mut!((*layer).nodes) as *mut c_void,
            &mut (*layer).element,
            false,
            true,
            ptr::null(),
            ElementType::Node,
        )?;
        p_display_layer = p_display_layer.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_selection_set, p_set, uc->scene.selection_sets)`
    let mut p_set: *mut *mut SelectionSet =
        (*uc.get()).scene.selection_sets.data as *mut *mut SelectionSet;
    let p_set_end: *mut *mut SelectionSet = add_ptr(p_set, (*uc.get()).scene.selection_sets.count);
    while p_set != p_set_end {
        let set: *mut SelectionSet = *p_set;
        fetch_dst_elements(
            uc,
            ptr::addr_of_mut!((*set).nodes) as *mut c_void,
            &mut (*set).element,
            false,
            true,
            ptr::null(),
            ElementType::SelectionNode,
        )?;
        p_set = p_set.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_selection_node, p_node, uc->scene.selection_nodes)`
    let mut p_sel_node: *mut *mut SelectionNode =
        (*uc.get()).scene.selection_nodes.data as *mut *mut SelectionNode;
    let p_sel_node_end: *mut *mut SelectionNode =
        add_ptr(p_sel_node, (*uc.get()).scene.selection_nodes.count);
    while p_sel_node != p_sel_node_end {
        let node: *mut SelectionNode = *p_sel_node;
        (*node).target_node =
            opt_ref(
                fetch_dst_element(&mut (*node).element, false, ptr::null(), ElementType::Node)
                    as *mut Node,
            );
        (*node).target_mesh =
            opt_ref(
                fetch_dst_element(&mut (*node).element, false, ptr::null(), ElementType::Mesh)
                    as *mut Mesh,
            );
        if opt_ptr(&(*node).target_mesh).is_null() && !opt_ptr(&(*node).target_node).is_null() {
            (*node).target_mesh = (*opt_ptr(&(*node).target_node)).mesh;
        } else if opt_ptr(&(*node).target_node).is_null()
            && !opt_ptr(&(*node).target_mesh).is_null()
            && (*opt_ptr(&(*node).target_mesh)).element.instances.count > 0
        {
            (*node).target_node = opt_ref(
                *((*opt_ptr(&(*node).target_mesh)).element.instances.data as *mut *mut Node).add(0),
            );
        }

        let mesh: *mut Mesh = opt_ptr(&(*node).target_mesh);
        if !mesh.is_null() {
            validate_indices(
                uc,
                ptr::addr_of_mut!((*node).vertices),
                (*mesh).num_vertices,
            )?;
            validate_indices(uc, ptr::addr_of_mut!((*node).edges), (*mesh).num_edges)?;
            validate_indices(uc, ptr::addr_of_mut!((*node).faces), (*mesh).num_faces)?;
        }
        p_sel_node = p_sel_node.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_constraint, p_constraint, uc->scene.constraints)`
    let mut p_constraint: *mut *mut Constraint =
        (*uc.get()).scene.constraints.data as *mut *mut Constraint;
    let p_constraint_end: *mut *mut Constraint =
        add_ptr(p_constraint, (*uc.get()).scene.constraints.count);
    while p_constraint != p_constraint_end {
        let constraint: *mut Constraint = *p_constraint;

        let tmp_base: usize = (*uc.get()).tmp_stack.num_items;

        // Find property connections in _both_ src and dst connections as they are inconsistent
        // in pre-7000 files. For example "Constrained Object" is a "PO" connection in 6100.
        // C: `ufbxi_for_list(ufbx_connection, conn, constraint->element.connections_src)`
        for conn_ix in 0..(*constraint).element.connections_src.count {
            let conn: *mut Connection =
                ((*constraint).element.connections_src.data as *mut Connection).add(conn_ix);
            if (*conn).src_prop.length == 0 || (*ref_ptr(&(*conn).dst)).type_ != ElementType::Node {
                continue;
            }
            add_constraint_prop(
                uc,
                constraint,
                ref_ptr(&(*conn).dst) as *mut Node,
                (*conn).src_prop.data,
            )?;
        }
        // C: `ufbxi_for_list(ufbx_connection, conn, constraint->element.connections_dst)`
        for conn_ix in 0..(*constraint).element.connections_dst.count {
            let conn: *mut Connection =
                ((*constraint).element.connections_dst.data as *mut Connection).add(conn_ix);
            if (*conn).dst_prop.length == 0 || (*ref_ptr(&(*conn).src)).type_ != ElementType::Node {
                continue;
            }
            add_constraint_prop(
                uc,
                constraint,
                ref_ptr(&(*conn).src) as *mut Node,
                (*conn).dst_prop.data,
            )?;
        }

        let num_targets: usize = (*uc.get()).tmp_stack.num_items - tmp_base;
        (*constraint).targets.count = num_targets;
        (*constraint).targets.data =
            push_pop::<ConstraintTarget>(uc.result_mut_ptr(), uc.tmp_stack_mut_ptr(), num_targets);
        ufbxi_check!(
            uc,
            !(*constraint).targets.data.is_null(),
            "constraint->targets.data"
        );
        p_constraint = p_constraint.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_audio_layer, p_layer, uc->scene.audio_layers)`
    let mut p_audio_layer: *mut *mut AudioLayer =
        (*uc.get()).scene.audio_layers.data as *mut *mut AudioLayer;
    let p_audio_layer_end: *mut *mut AudioLayer =
        add_ptr(p_audio_layer, (*uc.get()).scene.audio_layers.count);
    while p_audio_layer != p_audio_layer_end {
        let layer: *mut AudioLayer = *p_audio_layer;
        fetch_dst_elements(
            uc,
            ptr::addr_of_mut!((*layer).clips) as *mut c_void,
            &mut (*layer).element,
            false,
            true,
            ptr::null(),
            ElementType::AudioClip,
        )?;
        p_audio_layer = p_audio_layer.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_lod_group, p_lod, uc->scene.lod_groups)`
    let mut p_lod: *mut *mut LodGroup = (*uc.get()).scene.lod_groups.data as *mut *mut LodGroup;
    let p_lod_end: *mut *mut LodGroup = add_ptr(p_lod, (*uc.get()).scene.lod_groups.count);
    while p_lod != p_lod_end {
        finalize_lod_group(uc, *p_lod)?;
        p_lod = p_lod.add(1);
    }

    fetch_file_textures(uc)?;

    // NOTE: This will be patched over in `ufbxi_update_scene()` if there are `anim_layers`
    if (*uc.get()).scene.anim_layers.count == 0 {
        push_anim(
            uc,
            ptr::addr_of_mut!((*uc.get()).scene.anim) as *mut *mut Anim,
            ptr::null_mut(),
            0,
        )?;
    }

    (*uc.get()).scene.metadata.ktime_second = uc.ktime_sec();

    // Maya seems to use scale of 100/3, Blender binary uses exactly 33, ASCII has always value of 1.0
    if uc.version() < 6000 {
        (*uc.get()).scene.metadata.bone_prop_size_unit = 1.0f32 as Real;
    } else if (*uc.get()).exporter == Exporter::BlenderBinary {
        (*uc.get()).scene.metadata.bone_prop_size_unit = 33.0f32 as Real;
    } else if (*uc.get()).exporter == Exporter::BlenderAscii {
        (*uc.get()).scene.metadata.bone_prop_size_unit = 1.0f32 as Real;
    } else {
        (*uc.get()).scene.metadata.bone_prop_size_unit = (100.0 / 3.0) as Real;
    }
    if (*uc.get()).exporter == Exporter::BlenderAscii {
        (*uc.get()).scene.metadata.bone_prop_limb_length_relative = false;
    } else {
        (*uc.get()).scene.metadata.bone_prop_limb_length_relative = true;
    }

    Ok(())
}

// -- Interpret the read scene (ufbx.c:22626-22741)
//
// This section was ported by the eighth unit, ahead of `ufbxi_finalize_scene`
// (ufbx.c:21641-22624, then still a hole, filled by the ninth unit and now
// sitting above at its C-order slot), because `ufbxi_modify_geometry`
// (ufbx.c:21165) needs `ufbxi_get_geometry_transform` — which is exactly why C
// forward-declares it at ufbx.c:21070-21071.

// ufbx.c:22628-22633 `ufbxi_add_translate`
#[inline(always)]
pub(crate) unsafe fn add_translate(t: *mut Transform, v: Vec3) {
    (*t).translation.x += v.x;
    (*t).translation.y += v.y;
    (*t).translation.z += v.z;
}

// ufbx.c:22635-22640 `ufbxi_sub_translate`
#[inline(always)]
pub(crate) unsafe fn sub_translate(t: *mut Transform, v: Vec3) {
    (*t).translation.x -= v.x;
    (*t).translation.y -= v.y;
    (*t).translation.z -= v.z;
}

// ufbx.c:22642-22650 `ufbxi_mul_scale`
#[inline(always)]
pub(crate) unsafe fn mul_scale(t: *mut Transform, v: Vec3) {
    (*t).translation.x *= v.x;
    (*t).translation.y *= v.y;
    (*t).translation.z *= v.z;
    (*t).scale.x *= v.x;
    (*t).scale.y *= v.y;
    (*t).scale.z *= v.z;
}

// ufbx.c:22652-22660 `ufbxi_mul_scale_real`
#[inline(always)]
pub(crate) unsafe fn mul_scale_real(t: *mut Transform, v: Real) {
    (*t).translation.x *= v;
    (*t).translation.y *= v;
    (*t).translation.z *= v;
    (*t).scale.x *= v;
    (*t).scale.y *= v;
    (*t).scale.z *= v;
}

// ufbx.c:22662-22670 `ufbxi_mul_quat`
#[inline(never)]
pub(crate) unsafe fn mul_quat(a: Quat, b: Quat) -> Quat {
    // C: `ufbx_quat r;` — every field is written below before the return, so
    // the zero-fill is inert (upstream carries no `// ufbxi_uninit` marker).
    let mut r: Quat = core::mem::zeroed();
    r.x = a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y;
    r.y = a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x;
    r.z = a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w;
    r.w = a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z;
    r
}

// ufbx.c:22672-22677 `ufbxi_add_weighted_vec3`
#[inline(always)]
pub(crate) unsafe fn add_weighted_vec3(r: *mut Vec3, b: Vec3, w: Real) {
    (*r).x += b.x * w;
    (*r).y += b.y * w;
    (*r).z += b.z * w;
}

// ufbx.c:22679-22685 `ufbxi_add_weighted_quat`
#[inline(always)]
pub(crate) unsafe fn add_weighted_quat(r: *mut Quat, b: Quat, w: Real) {
    (*r).x += b.x * w;
    (*r).y += b.y * w;
    (*r).z += b.z * w;
    (*r).w += b.w * w;
}

// ufbx.c:22687-22693 `ufbxi_add_weighted_mat`
// C indexes the `ufbx_matrix` value union's `ufbx_vec3 cols[4]` view; the
// generated struct keeps only the named `m00`..`m23` fields, which are laid out
// exactly as four consecutive `ufbx_vec3` columns.
#[inline(never)]
pub(crate) unsafe fn add_weighted_mat(r: *mut Matrix, b: *const Matrix, w: Real) {
    let r_cols: *mut Vec3 = r as *mut Vec3;
    let b_cols: *const Vec3 = b as *const Vec3;
    add_weighted_vec3(r_cols.add(0), *b_cols.add(0), w);
    add_weighted_vec3(r_cols.add(1), *b_cols.add(1), w);
    add_weighted_vec3(r_cols.add(2), *b_cols.add(2), w);
    add_weighted_vec3(r_cols.add(3), *b_cols.add(3), w);
}

// ufbx.c:22695-22709 `ufbxi_mul_rotate`
pub(crate) unsafe fn mul_rotate(t: *mut Transform, v: Vec3, order: RotationOrder) {
    if is_vec3_zero(v) {
        return;
    }

    let q: Quat = euler_to_quat(v, order);
    if (*t).rotation.w != 1.0 {
        (*t).rotation = mul_quat(q, (*t).rotation);
    } else {
        (*t).rotation = q;
    }

    if !is_vec3_zero((*t).translation) {
        (*t).translation = quat_rotate_vec3(q, (*t).translation);
    }
}

// ufbx.c:22711-22724 `ufbxi_mul_rotate_quat`
pub(crate) unsafe fn mul_rotate_quat(t: *mut Transform, q: Quat) {
    if is_quat_identity(q) {
        return;
    }

    if (*t).rotation.w != 1.0 {
        (*t).rotation = mul_quat(q, (*t).rotation);
    } else {
        (*t).rotation = q;
    }

    if !is_vec3_zero((*t).translation) {
        (*t).translation = quat_rotate_vec3(q, (*t).translation);
    }
}

// ufbx.c:22726-22741 `ufbxi_mul_inv_rotate`
pub(crate) unsafe fn mul_inv_rotate(t: *mut Transform, v: Vec3, order: RotationOrder) {
    if is_vec3_zero(v) {
        return;
    }

    let mut q: Quat = euler_to_quat(v, order);
    q.x = -q.x;
    q.y = -q.y;
    q.z = -q.z;
    if (*t).rotation.w != 1.0 {
        (*t).rotation = mul_quat(q, (*t).rotation);
    } else {
        (*t).rotation = q;
    }

    if !is_vec3_zero((*t).translation) {
        (*t).translation = quat_rotate_vec3(q, (*t).translation);
    }
}

// -- Updating state from properties (ufbx.c:22743-…)
//
// The head of this banner section (ufbx.c:22745-22784) was ported by the
// eighth unit — the three helpers `ufbxi_modify_geometry` (ufbx.c:21165)
// depends on. The tenth unit continues at `ufbxi_get_rotation`
// (ufbx.c:22786) and runs through `ufbxi_update_light` (ufbx.c:23062).

// ufbx.c:22745-22749 `ufbxi_mirror_translation`
// C indexes the `ufbx_vec3` value union's `ufbx_real v[3]` view; the generated
// struct keeps only `x`/`y`/`z`, so the index is pointer arithmetic from the
// struct base (same device as `ufbxi_mirror_vec3_list` above).
#[inline(always)]
pub(crate) unsafe fn mirror_translation(p_vec: *mut Vec3, axis: MirrorAxis) {
    // C: `ufbxi_dev_assert(axis);` — enum truthiness.
    ufbxi_dev_assert!(axis != MirrorAxis::None);
    let v: *mut Real = p_vec as *mut Real;
    // C: `axis - 1` — the enum is promoted to `int` before the subtraction.
    let i: usize = (axis as i32 - 1) as usize;
    *v.add(i) = -*v.add(i);
}

// ufbx.c:22751-22756 `ufbxi_mirror_rotation`
// Same `ufbx_quat.v[4]` union view as `ufbxi_mirror_translation` above.
#[inline(always)]
pub(crate) unsafe fn mirror_rotation(p_quat: *mut Quat, axis: MirrorAxis) {
    // C: `ufbxi_dev_assert(axis);` — enum truthiness.
    ufbxi_dev_assert!(axis != MirrorAxis::None);
    let v: *mut Real = p_quat as *mut Real;
    // C: `axis % 3` / `(axis + 1) % 3` — the enum is promoted to `int` first.
    let i0: usize = (axis as i32 % 3) as usize;
    *v.add(i0) = -*v.add(i0);
    let i1: usize = ((axis as i32 + 1) % 3) as usize;
    *v.add(i1) = -*v.add(i1);
}

// ufbx.c:22758-22784 `ufbxi_get_geometry_transform`
// (forward-declared at ufbx.c:21070-21071 for `ufbxi_modify_geometry`)
#[inline(never)]
pub(crate) unsafe fn get_geometry_transform(props: *const Props, node: *mut Node) -> Transform {
    let translation: Vec3 = find_vec3(props, sp::GeometricTranslation.as_ptr(), 0.0, 0.0, 0.0);
    let rotation: Vec3 = find_vec3(props, sp::GeometricRotation.as_ptr(), 0.0, 0.0, 0.0);
    let scaling: Vec3 = find_vec3(props, sp::GeometricScaling.as_ptr(), 1.0, 1.0, 1.0);

    // C: `ufbx_transform t = { { 0,0,0 }, { 0,0,0,1 }, { 1,1,1 }};`
    let mut t: Transform = Transform {
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

    // WorldTransform = ParentWorldTransform * T * R * S * (OT * OR * OS)

    mul_scale(&mut t, scaling);
    mul_rotate(&mut t, rotation, RotationOrder::Xyz);
    add_translate(&mut t, translation);

    if (*node).has_adjust_transform {
        t.translation.x *= (*node).adjust_translation_scale;
        t.translation.y *= (*node).adjust_translation_scale;
        t.translation.z *= (*node).adjust_translation_scale;
    }

    if (*node).adjust_mirror_axis != MirrorAxis::None {
        mirror_translation(&mut t.translation, (*node).adjust_mirror_axis);
        mirror_rotation(&mut t.rotation, (*node).adjust_mirror_axis);
    }

    t
}

// ufbx.c:22786-22815 `ufbxi_get_rotation`
// Fast path for `ufbxi_get_transform` below: the rotation-only subset of that
// function's composition chain. The two are pinned together by the
// `ufbxi_regression_assert` at ufbx.c:22901.
#[inline(never)]
pub(crate) unsafe fn get_rotation(
    props: *const Props,
    order: RotationOrder,
    node: *const Node,
) -> Quat {
    let rotation: Vec3 = find_vec3(props, sp::Lcl_Rotation.as_ptr(), 0.0, 0.0, 0.0);
    let pre_rotation: Vec3 = find_vec3(props, sp::PreRotation.as_ptr(), 0.0, 0.0, 0.0);
    let post_rotation: Vec3 = find_vec3(props, sp::PostRotation.as_ptr(), 0.0, 0.0, 0.0);

    // C: `ufbx_transform t = { { 0,0,0 }, { 0,0,0,1 }, { 1,1,1 }};`
    let mut t: Transform = Transform {
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

    if (*node).has_adjust_transform {
        mul_rotate_quat(&mut t, (*node).adjust_post_rotation);
    }

    if (*node).use_rotation_space {
        mul_inv_rotate(&mut t, post_rotation, RotationOrder::Xyz);
        mul_rotate(&mut t, rotation, order);
        mul_rotate(&mut t, pre_rotation, RotationOrder::Xyz);
    } else {
        mul_rotate(&mut t, rotation, RotationOrder::Xyz);
    }

    if (*node).has_adjust_transform {
        mul_rotate_quat(&mut t, (*node).adjust_pre_rotation);
    }

    // C: `if (node->adjust_mirror_axis)` — enum truthiness.
    if (*node).adjust_mirror_axis != MirrorAxis::None {
        mirror_rotation(&mut t.rotation, (*node).adjust_mirror_axis);
    }

    t.rotation
}

// ufbx.c:22817-22834 `ufbxi_get_scale`
// Scale-only fast path, pinned to `ufbxi_get_transform` by the
// `ufbxi_regression_assert` at ufbx.c:22902.
#[inline(never)]
pub(crate) unsafe fn get_scale(props: *const Props, node: *const Node) -> Vec3 {
    let scaling: Vec3 = find_vec3(props, sp::Lcl_Scaling.as_ptr(), 1.0, 1.0, 1.0);

    // C: `ufbx_transform t = { { 0,0,0 }, { 0,0,0,1 }, { 1,1,1 }};`
    let mut t: Transform = Transform {
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

    if (*node).has_adjust_transform {
        mul_scale_real(&mut t, (*node).adjust_post_scale);
    }

    mul_scale(&mut t, scaling);

    if (*node).has_adjust_transform {
        mul_scale_real(&mut t, (*node).adjust_pre_scale);
    }

    t.scale
}

// ufbx.c:22836-22905 `ufbxi_get_transform`
#[inline(never)]
pub(crate) unsafe fn get_transform(
    props: *const Props,
    order: RotationOrder,
    node: *const Node,
    translation_scale: *const Vec3,
) -> Transform {
    let scale_pivot: Vec3 = find_vec3(props, sp::ScalingPivot.as_ptr(), 0.0, 0.0, 0.0);
    let rot_pivot: Vec3 = find_vec3(props, sp::RotationPivot.as_ptr(), 0.0, 0.0, 0.0);
    let scale_offset: Vec3 = find_vec3(props, sp::ScalingOffset.as_ptr(), 0.0, 0.0, 0.0);
    let rot_offset: Vec3 = find_vec3(props, sp::RotationOffset.as_ptr(), 0.0, 0.0, 0.0);

    let mut translation: Vec3 = find_vec3(props, sp::Lcl_Translation.as_ptr(), 0.0, 0.0, 0.0);
    let rotation: Vec3 = find_vec3(props, sp::Lcl_Rotation.as_ptr(), 0.0, 0.0, 0.0);
    let scaling: Vec3 = find_vec3(props, sp::Lcl_Scaling.as_ptr(), 1.0, 1.0, 1.0);

    let pre_rotation: Vec3 = find_vec3(props, sp::PreRotation.as_ptr(), 0.0, 0.0, 0.0);
    let post_rotation: Vec3 = find_vec3(props, sp::PostRotation.as_ptr(), 0.0, 0.0, 0.0);

    // C: `ufbx_transform t = { { 0,0,0 }, { 0,0,0,1 }, { 1,1,1 }};`
    let mut t: Transform = Transform {
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

    // WorldTransform = ParentWorldTransform * T * Roff * Rp * Rpre * R * Rpost * Rp-1 * Soff * Sp * S * Sp-1
    // NOTE: Rpost is inverted (!) after converting from PostRotation Euler angles

    if !translation_scale.is_null() {
        translation.x *= (*translation_scale).x;
        translation.y *= (*translation_scale).y;
        translation.z *= (*translation_scale).z;
    }

    if (*node).has_adjust_transform {
        mul_rotate_quat(&mut t, (*node).adjust_post_rotation);
        mul_scale_real(&mut t, (*node).adjust_post_scale);
    }

    sub_translate(&mut t, scale_pivot);
    mul_scale(&mut t, scaling);
    add_translate(&mut t, scale_pivot);

    add_translate(&mut t, scale_offset);

    sub_translate(&mut t, rot_pivot);
    if (*node).use_rotation_space {
        mul_inv_rotate(&mut t, post_rotation, RotationOrder::Xyz);
        mul_rotate(&mut t, rotation, order);
        mul_rotate(&mut t, pre_rotation, RotationOrder::Xyz);
    } else {
        mul_rotate(&mut t, rotation, RotationOrder::Xyz);
    }
    add_translate(&mut t, rot_pivot);

    add_translate(&mut t, rot_offset);

    add_translate(&mut t, translation);

    if (*node).has_adjust_transform {
        add_translate(&mut t, (*node).adjust_pre_translation);
        mul_rotate_quat(&mut t, (*node).adjust_pre_rotation);
        mul_scale_real(&mut t, (*node).adjust_pre_scale);
        t.translation.x *= (*node).adjust_translation_scale;
        t.translation.y *= (*node).adjust_translation_scale;
        t.translation.z *= (*node).adjust_translation_scale;
    }

    // C: `if (node->adjust_mirror_axis)` — enum truthiness.
    if (*node).adjust_mirror_axis != MirrorAxis::None {
        mirror_translation(&mut t.translation, (*node).adjust_mirror_axis);
        mirror_rotation(&mut t.rotation, (*node).adjust_mirror_axis);
    }

    // Make sure the fast paths are identical to this function.
    ufbxi_regression_assert!(is_quat_equal(t.rotation, get_rotation(props, order, node)));
    ufbxi_regression_assert!(is_vec3_equal(t.scale, get_scale(props, node)));

    t
}

// ufbx.c:22907-22936 `ufbxi_get_texture_transform`
#[inline(never)]
pub(crate) unsafe fn get_texture_transform(props: *const Props) -> Transform {
    let scale_pivot: Vec3 = find_vec3(props, sp::TextureScalingPivot.as_ptr(), 0.0, 0.0, 0.0);
    let rot_pivot: Vec3 = find_vec3(props, sp::TextureRotationPivot.as_ptr(), 0.0, 0.0, 0.0);

    let translation: Vec3 = find_vec3(props, sp::Translation.as_ptr(), 0.0, 0.0, 0.0);
    let rotation: Vec3 = find_vec3(props, sp::Rotation.as_ptr(), 0.0, 0.0, 0.0);
    let scaling: Vec3 = find_vec3(props, sp::Scaling.as_ptr(), 1.0, 1.0, 1.0);

    // C: `ufbx_transform t = { { 0,0,0 }, { 0,0,0,1 }, { 1,1,1 }};`
    let mut t: Transform = Transform {
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

    sub_translate(&mut t, scale_pivot);
    mul_scale(&mut t, scaling);
    add_translate(&mut t, scale_pivot);

    sub_translate(&mut t, rot_pivot);
    mul_rotate(&mut t, rotation, RotationOrder::Xyz);
    add_translate(&mut t, rot_pivot);

    add_translate(&mut t, translation);

    if find_int(props, sp::UVSwap.as_ptr(), 0) != 0 {
        let swap_scale: Vec3 = Vec3 {
            x: -1.0,
            y: 0.0,
            z: 0.0,
        };
        let swap_rotate: Vec3 = Vec3 {
            x: 0.0,
            y: 0.0,
            z: -90.0,
        };
        mul_scale(&mut t, swap_scale);
        mul_rotate(&mut t, swap_rotate, RotationOrder::Xyz);
    }

    t
}

// ufbx.c:22938-22953 `ufbxi_get_constraint_transform`
#[inline(never)]
pub(crate) unsafe fn get_constraint_transform(props: *const Props) -> Transform {
    let translation: Vec3 = find_vec3(props, sp::Translation.as_ptr(), 0.0, 0.0, 0.0);
    let rotation: Vec3 = find_vec3(props, sp::Rotation.as_ptr(), 0.0, 0.0, 0.0);
    let rotation_offset: Vec3 = find_vec3(props, sp::RotationOffset.as_ptr(), 0.0, 0.0, 0.0);
    let scaling: Vec3 = find_vec3(props, sp::Scaling.as_ptr(), 1.0, 1.0, 1.0);

    // C: `ufbx_transform t = { { 0,0,0 }, { 0,0,0,1 }, { 1,1,1 }};`
    let mut t: Transform = Transform {
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

    mul_scale(&mut t, scaling);
    mul_rotate(&mut t, rotation, RotationOrder::Xyz);
    mul_rotate(&mut t, rotation_offset, RotationOrder::Xyz);
    add_translate(&mut t, translation);

    t
}

// ufbx.c:22955-23042 `ufbxi_update_node`
#[inline(never)]
pub(crate) unsafe fn update_node(
    node: *mut Node,
    overrides: *const TransformOverride,
    num_overrides: usize,
) {
    // C: `(ufbx_rotation_order)ufbxi_find_enum(...)` — `ufbxi_find_enum` clamps
    // the result to `[0, UFBX_ROTATION_ORDER_SPHERIC]`, every value of which is
    // a valid `ufbx_rotation_order`.
    (*node).rotation_order = core::mem::transmute::<u32, RotationOrder>(find_enum(
        &(*node).element.props,
        sp::RotationOrder.as_ptr(),
        RotationOrder::Xyz as i64,
        RotationOrder::Spheric as i64,
    ) as u32);
    (*node).euler_rotation = find_vec3(
        &(*node).element.props,
        sp::Lcl_Rotation.as_ptr(),
        0.0,
        0.0,
        0.0,
    );

    if !(*node).is_root {
        let rotation_active: bool =
            find_int(&(*node).element.props, sp::RotationActive.as_ptr(), 1) != 0;
        let rotation_limit_only: bool = find_int(
            &(*node).element.props,
            sp::RotationSpaceForLimitOnly.as_ptr(),
            0,
        ) != 0;
        (*node).use_rotation_space = rotation_active && !rotation_limit_only;

        let mut transform_scale: *const Vec3 = ptr::null();
        // C: `if (node->parent && node->parent->scale_helper)` — the field is
        // re-read in the body, as in C.
        if !opt_ptr(&(*node).parent).is_null()
            && !opt_ptr(&(*opt_ptr(&(*node).parent)).scale_helper).is_null()
        {
            transform_scale = &(*opt_ptr(&(*opt_ptr(&(*node).parent)).scale_helper))
                .local_transform
                .scale;
        }
        (*node).local_transform = get_transform(
            &(*node).element.props,
            (*node).rotation_order,
            node,
            transform_scale,
        );
        if (*node).is_scale_helper
            && !opt_ptr(&(*node).parent).is_null()
            && !opt_ptr(&(*opt_ptr(&(*node).parent)).inherit_scale_node).is_null()
        {
            let scale_parent: *mut Node = opt_ptr(&(*opt_ptr(&(*node).parent)).inherit_scale_node);
            if !opt_ptr(&(*scale_parent).scale_helper).is_null() {
                let inherit_scale: Vec3 = (*opt_ptr(&(*scale_parent).scale_helper))
                    .local_transform
                    .scale;
                (*node).local_transform.scale.x *= inherit_scale.x;
                (*node).local_transform.scale.y *= inherit_scale.y;
                (*node).local_transform.scale.z *= inherit_scale.z;
            }
        }

        if num_overrides > 0 {
            let typed_id: u32 = (*node).element.typed_id;
            let mut override_ix: usize = usize::MAX;
            // C: `ufbxi_macro_lower_bound_eq(ufbx_transform_override, 16,
            // &override_ix, overrides, 0, num_overrides,
            // ( a->node_id < typed_id ), ( a->node_id == typed_id ));`
            macro_lower_bound_eq::<TransformOverride>(
                16,
                &mut override_ix,
                overrides,
                0,
                num_overrides,
                |a| (*a).node_id < typed_id,
                |a| (*a).node_id == typed_id,
            );
            if override_ix != usize::MAX {
                (*node).local_transform = (*overrides.add(override_ix)).transform;
            }
        }
        (*node).node_to_parent = transform_to_matrix(&(*node).local_transform);
        (*node).geometry_transform = get_geometry_transform(&(*node).element.props, node);
    } else {
        (*node).geometry_transform = IDENTITY_TRANSFORM;
    }

    let unscaled_node_to_parent: Matrix = unscaled_transform_to_matrix(&(*node).local_transform);

    (*node).inherit_scale = (*node).local_transform.scale;

    let parent: *mut Node = opt_ptr(&(*node).parent);
    if !parent.is_null() {
        if (*node).inherit_mode == InheritMode::Normal {
            (*node).node_to_world = matrix_mul(&(*parent).node_to_world, &(*node).node_to_parent);
            (*node).unscaled_node_to_world =
                matrix_mul(&(*parent).node_to_world, &unscaled_node_to_parent);
        } else {
            let mut transform: Transform = (*node).local_transform;

            let mut parent_scale: Vec3 = ONE_VEC3;
            if !opt_ptr(&(*node).inherit_scale_node).is_null() {
                parent_scale = (*opt_ptr(&(*node).inherit_scale_node)).inherit_scale;
            }

            transform.scale.x *= parent_scale.x;
            transform.scale.y *= parent_scale.y;
            transform.scale.z *= parent_scale.z;
            transform.translation.x *= (*parent).inherit_scale.x;
            transform.translation.y *= (*parent).inherit_scale.y;
            transform.translation.z *= (*parent).inherit_scale.z;

            let node_to_unscaled_parent: Matrix = transform_to_matrix(&transform);
            let unscaled_node_to_unscaled_parent: Matrix = unscaled_transform_to_matrix(&transform);

            (*node).inherit_scale = transform.scale;
            (*node).node_to_world =
                matrix_mul(&(*parent).unscaled_node_to_world, &node_to_unscaled_parent);
            (*node).unscaled_node_to_world = matrix_mul(
                &(*parent).unscaled_node_to_world,
                &unscaled_node_to_unscaled_parent,
            );
        }
    } else {
        (*node).node_to_world = (*node).node_to_parent;
        (*node).unscaled_node_to_world = unscaled_node_to_parent;
    }

    if !is_transform_identity(&(*node).geometry_transform) {
        (*node).geometry_to_node = transform_to_matrix(&(*node).geometry_transform);
        (*node).geometry_to_world = matrix_mul(&(*node).node_to_world, &(*node).geometry_to_node);
        (*node).has_geometry_transform = true;
    } else {
        (*node).geometry_to_node = IDENTITY_MATRIX;
        (*node).geometry_to_world = (*node).node_to_world;
        (*node).has_geometry_transform = false;
    }

    (*node).visible = find_int(&(*node).element.props, sp::Visibility.as_ptr(), 1) != 0;
}

// ufbx.c:23044-23062 `ufbxi_update_light`
#[inline(never)]
pub(crate) unsafe fn update_light(light: *mut Light) {
    // NOTE: FBX seems to store intensities 100x of what's specified in at least
    // Maya and Blender, should there be a quirks mode to not do this for specific
    // exporters. Does the FBX SDK do this transparently as well?
    (*light).intensity = find_real(
        &(*light).element.props,
        sp::Intensity.as_ptr(),
        100.0 as Real,
    ) / (100.0 as Real);

    (*light).color = find_vec3(&(*light).element.props, sp::Color.as_ptr(), 1.0, 1.0, 1.0);
    // C: `(ufbx_light_type)ufbxi_find_enum(...)` etc — `ufbxi_find_enum` clamps
    // each result to its enum's `[0, LAST]` range.
    (*light).type_ = core::mem::transmute::<u32, LightType>(find_enum(
        &(*light).element.props,
        sp::LightType.as_ptr(),
        0,
        LightType::Volume as i64,
    ) as u32);
    (*light).decay = core::mem::transmute::<u32, LightDecay>(find_enum(
        &(*light).element.props,
        sp::DecayType.as_ptr(),
        LightDecay::None as i64,
        LightDecay::Cubic as i64,
    ) as u32);
    (*light).area_shape = core::mem::transmute::<u32, LightAreaShape>(find_enum(
        &(*light).element.props,
        sp::AreaLightShape.as_ptr(),
        0,
        LightAreaShape::Sphere as i64,
    ) as u32);
    (*light).inner_angle = find_real(&(*light).element.props, sp::HotSpot.as_ptr(), 0.0);
    (*light).inner_angle = find_real(
        &(*light).element.props,
        sp::InnerAngle.as_ptr(),
        (*light).inner_angle,
    );
    (*light).outer_angle = find_real(&(*light).element.props, sp::Cone_angle.as_ptr(), 0.0);
    (*light).outer_angle = find_real(
        &(*light).element.props,
        sp::ConeAngle.as_ptr(),
        (*light).outer_angle,
    );
    (*light).outer_angle = find_real(
        &(*light).element.props,
        sp::OuterAngle.as_ptr(),
        (*light).outer_angle,
    );
    (*light).cast_light = find_int(&(*light).element.props, sp::CastLight.as_ptr(), 1) != 0;
    (*light).cast_shadows = find_int(&(*light).element.props, sp::CastShadows.as_ptr(), 0) != 0;
}

// ufbx.c:23064-23067 `ufbxi_aperture_format`
// NAMING DEVIATION: the PORTING.md rule would spell this `ApertureFormat`,
// which is already taken by the generated public enum `ufbx_aperture_format`
// (the very enum this record is indexed by). Suffixed `Info` to resolve the
// collision; the table below keeps the C name.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ApertureFormatInfo {
    // 1/1000 decimal fixed point for size
    pub film_size_x: u16,
    pub film_size_y: u16,
}

// ufbx.c:23069-23082 `ufbxi_aperture_formats`
static APERTURE_FORMATS: [ApertureFormatInfo; 12] = [
    // UFBX_APERTURE_FORMAT_CUSTOM
    ApertureFormatInfo {
        film_size_x: 1000,
        film_size_y: 1000,
    },
    // UFBX_APERTURE_FORMAT_16MM_THEATRICAL
    ApertureFormatInfo {
        film_size_x: 404,
        film_size_y: 295,
    },
    // UFBX_APERTURE_FORMAT_SUPER_16MM
    ApertureFormatInfo {
        film_size_x: 493,
        film_size_y: 292,
    },
    // UFBX_APERTURE_FORMAT_35MM_ACADEMY
    ApertureFormatInfo {
        film_size_x: 864,
        film_size_y: 630,
    },
    // UFBX_APERTURE_FORMAT_35MM_TV_PROJECTION
    ApertureFormatInfo {
        film_size_x: 816,
        film_size_y: 612,
    },
    // UFBX_APERTURE_FORMAT_35MM_FULL_APERTURE
    ApertureFormatInfo {
        film_size_x: 980,
        film_size_y: 735,
    },
    // UFBX_APERTURE_FORMAT_35MM_185_PROJECTION
    ApertureFormatInfo {
        film_size_x: 825,
        film_size_y: 446,
    },
    // UFBX_APERTURE_FORMAT_35MM_ANAMORPHIC
    ApertureFormatInfo {
        film_size_x: 864,
        film_size_y: 732,
    },
    // UFBX_APERTURE_FORMAT_70MM_PROJECTION
    ApertureFormatInfo {
        film_size_x: 2066,
        film_size_y: 906,
    },
    // UFBX_APERTURE_FORMAT_VISTAVISION
    ApertureFormatInfo {
        film_size_x: 1485,
        film_size_y: 991,
    },
    // UFBX_APERTURE_FORMAT_DYNAVISION
    ApertureFormatInfo {
        film_size_x: 2080,
        film_size_y: 1480,
    },
    // UFBX_APERTURE_FORMAT_IMAX
    ApertureFormatInfo {
        film_size_x: 2772,
        film_size_y: 2072,
    },
];

// ufbx.c:23084-23252 `ufbxi_update_camera`
#[inline(never)]
pub(crate) unsafe fn update_camera(scene: *mut Scene, camera: *mut Camera) {
    // C: `(ufbx_projection_mode)ufbxi_find_enum(...)` etc — `ufbxi_find_enum`
    // clamps each result to its enum's `[0, LAST]` range (same device as
    // `ufbxi_update_light` above).
    (*camera).projection_mode = core::mem::transmute::<u32, ProjectionMode>(find_enum(
        &(*camera).element.props,
        sp::CameraProjectionType.as_ptr(),
        0,
        ProjectionMode::Orthographic as i64,
    ) as u32);
    (*camera).aspect_mode = core::mem::transmute::<u32, AspectMode>(find_enum(
        &(*camera).element.props,
        sp::AspectRatioMode.as_ptr(),
        0,
        AspectMode::FixedHeight as i64,
    ) as u32);
    (*camera).aperture_mode = core::mem::transmute::<u32, ApertureMode>(find_enum(
        &(*camera).element.props,
        sp::ApertureMode.as_ptr(),
        ApertureMode::Vertical as i64,
        ApertureMode::FocalLength as i64,
    ) as u32);
    (*camera).aperture_format = core::mem::transmute::<u32, ApertureFormat>(find_enum(
        &(*camera).element.props,
        sp::ApertureFormat.as_ptr(),
        ApertureFormat::Custom as i64,
        ApertureFormat::Imax as i64,
    ) as u32);
    (*camera).gate_fit = core::mem::transmute::<u32, GateFit>(find_enum(
        &(*camera).element.props,
        sp::GateFit.as_ptr(),
        0,
        GateFit::Stretch as i64,
    ) as u32);

    (*camera).near_plane = find_real(&(*camera).element.props, sp::NearPlane.as_ptr(), 0.0);
    (*camera).far_plane = find_real(&(*camera).element.props, sp::FarPlane.as_ptr(), 0.0);

    // Search both W/H and Width/Height but prefer the latter
    let mut aspect_x: Real = find_real(&(*camera).element.props, sp::AspectW.as_ptr(), 0.0);
    let mut aspect_y: Real = find_real(&(*camera).element.props, sp::AspectH.as_ptr(), 0.0);
    aspect_x = find_real(&(*camera).element.props, sp::AspectWidth.as_ptr(), aspect_x);
    aspect_y = find_real(
        &(*camera).element.props,
        sp::AspectHeight.as_ptr(),
        aspect_y,
    );

    let fov: Real = find_real(&(*camera).element.props, sp::FieldOfView.as_ptr(), 0.0);
    let fov_x: Real = find_real(&(*camera).element.props, sp::FieldOfViewX.as_ptr(), 0.0);
    let fov_y: Real = find_real(&(*camera).element.props, sp::FieldOfViewY.as_ptr(), 0.0);

    let focal_length: Real = find_real(&(*camera).element.props, sp::FocalLength.as_ptr(), 0.0);
    let mut ortho_extent: Real = (*scene).metadata.ortho_size_unit
        * find_real(&(*camera).element.props, sp::OrthoZoom.as_ptr(), 1.0);

    let format: ApertureFormatInfo = APERTURE_FORMATS[(*camera).aperture_format as usize];
    let mut film_size: Vec2 = Vec2 {
        x: format.film_size_x as Real * (0.001 as Real),
        y: format.film_size_y as Real * (0.001 as Real),
    };
    let mut squeeze_ratio: Real = if (*camera).aperture_format == ApertureFormat::E35MmAnamorphic {
        2.0
    } else {
        1.0
    };

    film_size.x = find_real(
        &(*camera).element.props,
        sp::FilmWidth.as_ptr(),
        film_size.x,
    );
    film_size.y = find_real(
        &(*camera).element.props,
        sp::FilmHeight.as_ptr(),
        film_size.y,
    );
    squeeze_ratio = find_real(
        &(*camera).element.props,
        sp::FilmSqueezeRatio.as_ptr(),
        squeeze_ratio,
    );

    if aspect_x <= 0.0 && aspect_y <= 0.0 {
        aspect_x = if film_size.x > 0.0 { film_size.x } else { 1.0 };
        aspect_y = if film_size.y > 0.0 { film_size.y } else { 1.0 };
    } else if aspect_x <= 0.0 {
        if film_size.x > 0.0 && film_size.y > 0.0 {
            aspect_x = aspect_y / film_size.y * film_size.x;
        } else {
            aspect_x = aspect_y;
        }
    } else if aspect_y <= 0.0 {
        if film_size.x > 0.0 && film_size.y > 0.0 {
            aspect_y = aspect_x / film_size.x * film_size.y;
        } else {
            aspect_y = aspect_x;
        }
    }

    film_size.y *= squeeze_ratio;

    // TODO: Should this be done always?
    ortho_extent *= (*scene).metadata.geometry_scale;
    (*camera).near_plane *= (*scene).metadata.geometry_scale;
    (*camera).far_plane *= (*scene).metadata.geometry_scale;

    (*camera).focal_length_mm = focal_length;
    (*camera).film_size_inch = film_size;
    (*camera).squeeze_ratio = squeeze_ratio;
    (*camera).orthographic_extent = ortho_extent;

    match (*camera).aspect_mode {
        AspectMode::WindowSize | AspectMode::FixedRatio => {
            (*camera).resolution_is_pixels = false;
            (*camera).resolution.x = aspect_x;
            (*camera).resolution.y = aspect_y;
        }
        AspectMode::FixedResolution => {
            (*camera).resolution_is_pixels = true;
            (*camera).resolution.x = aspect_x;
            (*camera).resolution.y = aspect_y;
        }
        AspectMode::FixedWidth => {
            (*camera).resolution_is_pixels = true;
            (*camera).resolution.x = aspect_x;
            (*camera).resolution.y = aspect_x * aspect_y;
        }
        AspectMode::FixedHeight => {
            (*camera).resolution_is_pixels = true;
            (*camera).resolution.x = aspect_y * aspect_x;
            (*camera).resolution.y = aspect_y;
        }
        // C `default:` (ufbx.c:23167-23168) — unreachable in Rust because the
        // match above is exhaustive over the enum, but kept for diff parity.
        #[allow(unreachable_patterns)]
        _ => {
            ufbxi_unreachable!("Unexpected aspect mode");
        }
    }

    let aspect_ratio: Real = (*camera).resolution.x / (*camera).resolution.y;
    let film_ratio: Real = film_size.x / film_size.y;

    (*camera).aspect_ratio = aspect_ratio;

    let mut effective_fit: GateFit = (*camera).gate_fit;
    if effective_fit == GateFit::Fill {
        effective_fit = if aspect_ratio > film_ratio {
            GateFit::Horizontal
        } else {
            GateFit::Vertical
        };
    } else if effective_fit == GateFit::Overscan {
        effective_fit = if aspect_ratio < film_ratio {
            GateFit::Horizontal
        } else {
            GateFit::Vertical
        };
    }

    match effective_fit {
        GateFit::None => {
            (*camera).aperture_size_inch = (*camera).film_size_inch;
            (*camera).orthographic_size.x = ortho_extent;
            (*camera).orthographic_size.y = ortho_extent;
        }
        GateFit::Vertical => {
            (*camera).aperture_size_inch.x = (*camera).film_size_inch.y * aspect_ratio;
            (*camera).aperture_size_inch.y = (*camera).film_size_inch.y;
            (*camera).orthographic_size.x = ortho_extent * aspect_ratio;
            (*camera).orthographic_size.y = ortho_extent;
        }
        GateFit::Horizontal => {
            (*camera).aperture_size_inch.x = (*camera).film_size_inch.x;
            (*camera).aperture_size_inch.y = (*camera).film_size_inch.x / aspect_ratio;
            (*camera).orthographic_size.x = ortho_extent;
            (*camera).orthographic_size.y = ortho_extent / aspect_ratio;
        }
        GateFit::Fill | GateFit::Overscan => {
            (*camera).aperture_size_inch = (*camera).film_size_inch;
            (*camera).orthographic_size.x = ortho_extent;
            (*camera).orthographic_size.y = ortho_extent;
            // C: `ufbxi_unreachable(...)` mid-arm — it is NOT a return, the
            // arm's assignments above it already ran (PORTING.md "Asserts").
            ufbxi_unreachable!("Unreachable, set to vertical/horizontal above");
        }
        GateFit::Stretch => {
            (*camera).aperture_size_inch = (*camera).film_size_inch;
            (*camera).orthographic_size.x = ortho_extent;
            (*camera).orthographic_size.y = ortho_extent;
            // TODO: Not sure what to do here...
        }
        // C `default:` (ufbx.c:23214-23215).
        #[allow(unreachable_patterns)]
        _ => {
            ufbxi_unreachable!("Unexpected gate fit");
        }
    }

    match (*camera).aperture_mode {
        ApertureMode::HorizontalAndVertical => {
            (*camera).field_of_view_deg.x = fov_x;
            (*camera).field_of_view_deg.y = fov_y;
            // C: `(ufbx_real)ufbx_tan((double)(...))` — the inner product is
            // real arithmetic, promoted to double only at the `tan` call.
            (*camera).field_of_view_tan.x =
                math::tan((fov_x * (sp::DEG_TO_RAD * 0.5)) as f64) as Real;
            (*camera).field_of_view_tan.y =
                math::tan((fov_y * (sp::DEG_TO_RAD * 0.5)) as f64) as Real;
        }
        ApertureMode::Horizontal => {
            (*camera).field_of_view_deg.x = fov;
            (*camera).field_of_view_tan.x =
                math::tan((fov * (sp::DEG_TO_RAD * 0.5)) as f64) as Real;
            (*camera).field_of_view_tan.y = (*camera).field_of_view_tan.x / aspect_ratio;
            (*camera).field_of_view_deg.y =
                math::atan((*camera).field_of_view_tan.y as f64) as Real * sp::RAD_TO_DEG * 2.0;
        }
        ApertureMode::Vertical => {
            (*camera).field_of_view_deg.y = fov;
            (*camera).field_of_view_tan.y =
                math::tan((fov * (sp::DEG_TO_RAD * 0.5)) as f64) as Real;
            (*camera).field_of_view_tan.x = (*camera).field_of_view_tan.y * aspect_ratio;
            (*camera).field_of_view_deg.x =
                math::atan((*camera).field_of_view_tan.x as f64) as Real * sp::RAD_TO_DEG * 2.0;
        }
        ApertureMode::FocalLength => {
            (*camera).field_of_view_tan.x =
                (*camera).aperture_size_inch.x / ((*camera).focal_length_mm * sp::MM_TO_INCH) * 0.5;
            (*camera).field_of_view_tan.y =
                (*camera).aperture_size_inch.y / ((*camera).focal_length_mm * sp::MM_TO_INCH) * 0.5;
            (*camera).field_of_view_deg.x =
                math::atan((*camera).field_of_view_tan.x as f64) as Real * sp::RAD_TO_DEG * 2.0;
            (*camera).field_of_view_deg.y =
                math::atan((*camera).field_of_view_tan.y as f64) as Real * sp::RAD_TO_DEG * 2.0;
        }
        // C `default:` (ufbx.c:23243-23244).
        #[allow(unreachable_patterns)]
        _ => {
            ufbxi_unreachable!("Unexpected aperture mode");
        }
    }

    if (*camera).projection_mode == ProjectionMode::Perspective {
        (*camera).projection_plane = (*camera).field_of_view_tan;
    } else {
        (*camera).projection_plane = (*camera).orthographic_size;
    }
}

// ufbx.c:23254-23264 `ufbxi_update_bone`
#[inline(never)]
pub(crate) unsafe fn update_bone(scene: *mut Scene, bone: *mut Bone) {
    let unit: Real = (*scene).metadata.bone_prop_size_unit;

    (*bone).radius = find_real(&(*bone).element.props, sp::Size.as_ptr(), unit) / unit;
    if (*scene).metadata.bone_prop_limb_length_relative {
        (*bone).relative_length = find_real(&(*bone).element.props, sp::LimbLength.as_ptr(), 1.0);
    } else {
        (*bone).relative_length = 1.0;
    }
}

// ufbx.c:23266-23269 `ufbxi_update_line_curve`
#[inline(never)]
pub(crate) unsafe fn update_line_curve(line: *mut LineCurve) {
    (*line).color = find_vec3(&(*line).element.props, sp::Color.as_ptr(), 1.0, 1.0, 1.0);
}

// ufbx.c:23271-23287 `ufbxi_update_pose`
#[inline(never)]
pub(crate) unsafe fn update_pose(pose: *mut Pose) {
    // C: `ufbxi_for_list(ufbx_bone_pose, bone, pose->bone_poses)`
    let mut bone: *mut BonePose = (*pose).bone_poses.data as *mut BonePose;
    let bone_end: *mut BonePose = add_ptr(bone, (*pose).bone_poses.count);
    while bone != bone_end {
        let node: *mut Node = ref_ptr(&(*bone).bone_node);

        let mut parent_to_world: *const Matrix = &IDENTITY_MATRIX;
        let bone_pose: *mut BonePose = get_bone_pose(pose, opt_ptr(&(*node).parent));
        if !bone_pose.is_null() {
            parent_to_world = &(*bone_pose).bone_to_world;
        } else if !opt_ptr(&(*node).parent).is_null() {
            parent_to_world = &(*opt_ptr(&(*node).parent)).node_to_world;
        }

        let world_to_parent: Matrix = matrix_invert(parent_to_world);
        (*bone).bone_to_parent = matrix_mul(&world_to_parent, &(*bone).bone_to_world);

        bone = bone.add(1);
    }
}

// ufbx.c:23289-23297 `ufbxi_update_skin_cluster`
#[inline(never)]
pub(crate) unsafe fn update_skin_cluster(cluster: *mut SkinCluster) {
    // C: `if (cluster->bone_node)` — pointer truthiness.
    let bone_node: *mut Node = opt_ptr(&(*cluster).bone_node);
    if !bone_node.is_null() {
        (*cluster).geometry_to_world =
            matrix_mul(&(*bone_node).node_to_world, &(*cluster).geometry_to_bone);
    } else {
        (*cluster).geometry_to_world =
            matrix_mul(&(*cluster).bind_to_world, &(*cluster).geometry_to_bone);
    }
    (*cluster).geometry_to_world_transform = matrix_to_transform(&(*cluster).geometry_to_world);
}

// ufbx.c:23299-23342 `ufbxi_update_blend_channel`
#[inline(never)]
pub(crate) unsafe fn update_blend_channel(channel: *mut BlendChannel) {
    let weight: Real =
        find_real(&(*channel).element.props, sp::DeformPercent.as_ptr(), 0.0) * (0.01 as Real);
    (*channel).weight = weight;

    let num_keys: isize = (*channel).keyframes.count as isize;
    if num_keys > 0 {
        let keys: *mut BlendKeyframe = (*channel).keyframes.data as *mut BlendKeyframe;

        // Reset the effective weights to zero and find the split around zero
        let mut last_negative: isize = -1;
        let mut i: isize = 0;
        while i < num_keys {
            (*keys.offset(i)).effective_weight = 0.0 as Real;
            if (*keys.offset(i)).target_weight < 0.0 {
                last_negative = i;
            }
            i += 1;
        }

        // C: `ufbx_blend_keyframe zero_key = { NULL };` — a zeroed keyframe used
        // only as a `{ target_weight = 0, effective_weight = 0 }` sentinel.
        // `ufbx_blend_keyframe.shape` is a non-nullable `Ref<BlendShape>`
        // (`NonNull`), so the zeroed storage stays in `MaybeUninit` and is only
        // ever reached through a raw `*mut BlendKeyframe` — the `shape` member
        // is never read (C-parity: C reads it just as little).
        let mut zero_key_storage: MaybeUninit<BlendKeyframe> = MaybeUninit::zeroed();
        let zero_key: *mut BlendKeyframe = zero_key_storage.as_mut_ptr();
        let mut prev: *mut BlendKeyframe = zero_key;
        let mut next: *mut BlendKeyframe = zero_key;
        if weight > 0.0 {
            if last_negative >= 0 {
                prev = keys.offset(last_negative);
            }
            let mut i: isize = last_negative + 1;
            while i < num_keys {
                prev = next;
                next = keys.offset(i);
                if (*next).target_weight > weight {
                    break;
                }
                i += 1;
            }
        } else {
            if last_negative + 1 < num_keys {
                prev = keys.offset(last_negative + 1);
            }
            let mut i: isize = last_negative;
            while i >= 0 {
                prev = next;
                next = keys.offset(i);
                if (*next).target_weight < weight {
                    break;
                }
                i -= 1;
            }
        }

        // Linearly interpolate between the endpoints with the weight
        let delta: Real = (*next).target_weight - (*prev).target_weight;
        if delta != 0.0 {
            let t: Real = (weight - (*prev).target_weight) / delta;
            (*prev).effective_weight = 1.0 - t;
            (*next).effective_weight = t;
        }
    }
}

// ufbx.c:23344-23349 `ufbxi_update_material`
#[inline(never)]
pub(crate) unsafe fn update_material(scene: *mut Scene, material: *mut Material) {
    if (*material).element.props.num_animated > 0 {
        fetch_maps(scene, material);
    }
}

// ufbx.c:23351-23369 `ufbxi_update_texture`
#[inline(never)]
pub(crate) unsafe fn update_texture(texture: *mut Texture) {
    (*texture).uv_transform = get_texture_transform(&(*texture).element.props);
    if !is_transform_identity(&(*texture).uv_transform) {
        (*texture).has_uv_transform = true;
        (*texture).texture_to_uv = transform_to_matrix(&(*texture).uv_transform);
        (*texture).uv_to_texture = matrix_invert(&(*texture).texture_to_uv);
    } else {
        (*texture).has_uv_transform = false;
        (*texture).texture_to_uv = IDENTITY_MATRIX;
        (*texture).uv_to_texture = IDENTITY_MATRIX;
    }
    // C: `(ufbx_wrap_mode)ufbxi_find_enum(...)` — clamped to `[0, LAST]`.
    (*texture).wrap_u = core::mem::transmute::<u32, WrapMode>(find_enum(
        &(*texture).element.props,
        sp::WrapModeU.as_ptr(),
        0,
        WrapMode::Clamp as i64,
    ) as u32);
    (*texture).wrap_v = core::mem::transmute::<u32, WrapMode>(find_enum(
        &(*texture).element.props,
        sp::WrapModeV.as_ptr(),
        0,
        WrapMode::Clamp as i64,
    ) as u32);

    // C: `if (texture->shader)` — pointer truthiness.
    let shader: *mut ShaderTexture = opt_ptr(&(*texture).shader);
    if !shader.is_null() {
        update_shader_texture(texture, shader);
    }
}

// ufbx.c:23371-23388 `ufbxi_update_anim_stack`
#[inline(never)]
pub(crate) unsafe fn update_anim_stack(scene: *mut Scene, stack: *mut AnimStack) {
    // C: `ufbx_prop *begin, *end;` — both are assigned before any read.
    let mut begin: *mut Prop;
    let mut end: *mut Prop;
    begin = find_prop(&(*stack).element.props, sp::LocalStart.as_ptr());
    end = find_prop(&(*stack).element.props, sp::LocalStop.as_ptr());
    if begin.is_null() || end.is_null() {
        begin = find_prop(&(*stack).element.props, sp::ReferenceStart.as_ptr());
        end = find_prop(&(*stack).element.props, sp::ReferenceStop.as_ptr());
    }

    if !begin.is_null() && !end.is_null() {
        (*stack).time_begin = (*begin).value_int as f64 / (*scene).metadata.ktime_second as f64;
        (*stack).time_end = (*end).value_int as f64 / (*scene).metadata.ktime_second as f64;
    }

    let anim: *mut Anim = ref_ptr(&(*stack).anim);
    (*anim).time_begin = (*stack).time_begin;
    (*anim).time_end = (*stack).time_end;
}

// ufbx.c:23390-23395 `ufbxi_update_display_layer`
#[inline(never)]
pub(crate) unsafe fn update_display_layer(layer: *mut DisplayLayer) {
    (*layer).visible = find_int(&(*layer).element.props, sp::Show.as_ptr(), 1) != 0;
    (*layer).frozen = find_int(&(*layer).element.props, sp::Freeze.as_ptr(), 1) != 0;
    // C-parity: `0.8f` is a `float` literal widened to `ufbx_real` (double) —
    // NOT the decimal value 0.8 (PORTING.md "Floats").
    (*layer).ui_color = find_vec3(
        &(*layer).element.props,
        sp::Color.as_ptr(),
        0.8f32 as Real,
        0.8f32 as Real,
        0.8f32 as Real,
    );
}

// ufbx.c:23397-23414 `ufbxi_find_bool3`
#[inline(never)]
pub(crate) unsafe fn find_bool3(
    dst: *mut bool,
    props: *mut Props,
    name: *const u8,
    default_value: bool,
) {
    let name_len: usize = strlen(name);
    // C: `char local[64];` — an uninitialized local; only `local[0..name_len]`
    // is ever read back (`local_len == name_len + 1` bytes are written first).
    let mut local_storage = MaybeUninit::<[u8; 64]>::uninit();
    let local: *mut u8 = local_storage.as_mut_ptr() as *mut u8;
    // C: `ufbx_assert(name_len < sizeof(local) - 2);`
    ufbx_assert!(name_len < size_of::<[u8; 64]>() - 2);
    ptr::copy_nonoverlapping(name, local, name_len);

    let local_len: usize = name_len + 1;
    *local.add(local_len) = b'\0';

    let def: i64 = if default_value { 1 } else { 0 };
    *local.add(name_len) = b'X';
    *dst.add(0) = api_find_int_len(props, local, local_len, def) != 0;
    *local.add(name_len) = b'Y';
    *dst.add(1) = api_find_int_len(props, local, local_len, def) != 0;
    *local.add(name_len) = b'Z';
    *dst.add(2) = api_find_int_len(props, local, local_len, def) != 0;
}

// ufbx.c:23416-23488 `ufbxi_update_constraint`
#[inline(never)]
pub(crate) unsafe fn update_constraint(constraint: *mut Constraint) {
    // C: `ufbx_props *props = &constraint->props;` — kept live across writes
    // through `constraint`, so this must be an `addr_of_mut!` and never a `&mut`
    // (which would retag and be invalidated by those writes).
    let props: *mut Props = ptr::addr_of_mut!((*constraint).element.props);
    let constraint_type: ConstraintType = (*constraint).type_;

    (*constraint).transform_offset = get_constraint_transform(props);

    // C: `ufbxi_find_real` — the internal 4-byte-key lookup, NOT `ufbx_find_real`.
    (*constraint).weight = find_real(props, sp::Weight.as_ptr(), 100.0 as Real) / (100.0 as Real);

    // C: `ufbxi_for_list(ufbx_constraint_target, target, constraint->targets)`
    let mut target: *mut ConstraintTarget = (*constraint).targets.data as *mut ConstraintTarget;
    let target_end: *mut ConstraintTarget = add_ptr(target, (*constraint).targets.count);
    while target != target_end {
        let node: *mut Node = ref_ptr(&(*target).node);

        let mut weight_scale: Real = 100.0 as Real;
        if constraint_type == ConstraintType::SingleChainIk {
            // IK weights seem to be not scaled 100x?
            weight_scale = 1.0 as Real;
        }

        let mut prop: *mut Prop; // ufbxi_uninit
        let mut parts_storage = MaybeUninit::<[String; 2]>::uninit(); // ufbxi_uninit
        let parts: *mut String = parts_storage.as_mut_ptr() as *mut String;
        *parts.add(0) = (*node).element.name;
        *parts.add(1) = sp::str_c(b".Weight\0".as_ptr());
        prop = find_prop_concat(props, parts, 2);
        // C: `prop->value_real` — the `ufbx_prop` value union's first real.
        (*target).weight = (if !prop.is_null() {
            (*prop).value_vec4.x
        } else {
            weight_scale
        }) / weight_scale;

        if constraint_type == ConstraintType::Parent {
            *parts.add(1) = sp::str_c(b".Offset T\0".as_ptr());
            prop = find_prop_concat(props, parts, 2);
            let t: Vec3 = if !prop.is_null() {
                *(&(*prop).value_vec4 as *const Vec4 as *const Vec3)
            } else {
                ZERO_VEC3
            };
            *parts.add(1) = sp::str_c(b".Offset R\0".as_ptr());
            prop = find_prop_concat(props, parts, 2);
            let r: Vec3 = if !prop.is_null() {
                *(&(*prop).value_vec4 as *const Vec4 as *const Vec3)
            } else {
                ZERO_VEC3
            };
            *parts.add(1) = sp::str_c(b".Offset S\0".as_ptr());
            prop = find_prop_concat(props, parts, 2);
            let s: Vec3 = if !prop.is_null() {
                *(&(*prop).value_vec4 as *const Vec4 as *const Vec3)
            } else {
                ONE_VEC3
            };

            (*target).transform.translation = t;
            (*target).transform.rotation = euler_to_quat(r, RotationOrder::Xyz);
            (*target).transform.scale = s;
        }

        target = target.add(1);
    }

    (*constraint).active = api_find_int(props, b"Active\0".as_ptr(), 1) != 0;
    if constraint_type == ConstraintType::Aim {
        find_bool3(
            (*constraint).constrain_rotation.as_mut_ptr(),
            props,
            b"Affect\0".as_ptr(),
            true,
        );

        let default_aim: Vec3 = Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        let default_up: Vec3 = Vec3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };

        let up_type: i64 = api_find_int(props, b"WorldUpType\0".as_ptr(), 0);
        if up_type >= 0 && up_type < ConstraintAimUpType::None as i64 {
            // C: `(ufbx_constraint_aim_up_type)up_type` — the range check above
            // admits only valid enum values.
            (*constraint).aim_up_type =
                core::mem::transmute::<u32, ConstraintAimUpType>(up_type as u32);
        }
        (*constraint).aim_vector = api_find_vec3(props, b"AimVector\0".as_ptr(), default_aim);
        (*constraint).aim_up_vector = api_find_vec3(props, b"UpVector\0".as_ptr(), default_up);
    } else if constraint_type == ConstraintType::Parent {
        find_bool3(
            (*constraint).constrain_translation.as_mut_ptr(),
            props,
            b"AffectTranslation\0".as_ptr(),
            true,
        );
        find_bool3(
            (*constraint).constrain_rotation.as_mut_ptr(),
            props,
            b"AffectRotation\0".as_ptr(),
            true,
        );
        find_bool3(
            (*constraint).constrain_scale.as_mut_ptr(),
            props,
            b"AffectScale\0".as_ptr(),
            false,
        );
    } else if constraint_type == ConstraintType::Position {
        find_bool3(
            (*constraint).constrain_translation.as_mut_ptr(),
            props,
            b"Affect\0".as_ptr(),
            true,
        );
    } else if constraint_type == ConstraintType::Rotation {
        find_bool3(
            (*constraint).constrain_rotation.as_mut_ptr(),
            props,
            b"Affect\0".as_ptr(),
            true,
        );
    } else if constraint_type == ConstraintType::Scale {
        find_bool3(
            (*constraint).constrain_scale.as_mut_ptr(),
            props,
            b"Affect\0".as_ptr(),
            true,
        );
    } else if constraint_type == ConstraintType::SingleChainIk {
        (*constraint).constrain_rotation[0] = true;
        (*constraint).constrain_rotation[1] = true;
        (*constraint).constrain_rotation[2] = true;
        (*constraint).ik_pole_vector =
            api_find_vec3(props, b"PoleVectorType\0".as_ptr(), ZERO_VEC3);
    }
}

// ufbx.c:23490-23495 `ufbxi_update_anim`
#[inline(never)]
pub(crate) unsafe fn update_anim(scene: *mut Scene) {
    if (*scene).anim_stacks.count > 0 {
        // C: `scene->anim = scene->anim_stacks.data[0]->anim;`
        let stack: *mut AnimStack = *((*scene).anim_stacks.data as *const *mut AnimStack);
        (*scene).anim = (*stack).anim;
    }
}

// ufbx.c:23497-23505 `ufbxi_mirror_matrix_dst`
// C indexes the `ufbx_matrix` value union's `ufbx_vec3 cols[4]` view and then
// each column's `ufbx_real v[3]`; the generated struct keeps only the named
// `m00`..`m23` scalars, which are laid out as exactly four consecutive
// `ufbx_vec3` columns (same device as `ufbxi_add_weighted_mat`).
#[inline(always)]
pub(crate) unsafe fn mirror_matrix_dst(m: *mut Matrix, axis: MirrorAxis) {
    // C: `if (axis == 0) return;`
    if axis as u32 == 0 {
        return;
    }
    let ax: i32 = axis as i32 - 1;
    let cols: *mut Vec3 = m as *mut Vec3;
    let c0: *mut Real = cols.add(0) as *mut Real;
    *c0.add(ax as usize) = -*c0.add(ax as usize);
    let c1: *mut Real = cols.add(1) as *mut Real;
    *c1.add(ax as usize) = -*c1.add(ax as usize);
    let c2: *mut Real = cols.add(2) as *mut Real;
    *c2.add(ax as usize) = -*c2.add(ax as usize);
    let c3: *mut Real = cols.add(3) as *mut Real;
    *c3.add(ax as usize) = -*c3.add(ax as usize);
}

// ufbx.c:23507-23514 `ufbxi_mirror_matrix_src`
// Same `cols[4]` overlay as `ufbxi_mirror_matrix_dst`, but here C names the
// column's `x`/`y`/`z` members directly.
#[inline(always)]
pub(crate) unsafe fn mirror_matrix_src(m: *mut Matrix, axis: MirrorAxis) {
    // C: `if (axis == 0) return;`
    if axis as u32 == 0 {
        return;
    }
    let ax: i32 = axis as i32 - 1;
    let cols: *mut Vec3 = m as *mut Vec3;
    let col: *mut Vec3 = cols.add(ax as usize);
    (*col).x = -(*col).x;
    (*col).y = -(*col).y;
    (*col).z = -(*col).z;
}

// ufbx.c:23516-23521 `ufbxi_mirror_matrix`
#[inline(never)]
pub(crate) unsafe fn mirror_matrix(m: *mut Matrix, axis: MirrorAxis) {
    // C: `if (axis == 0) return;`
    if axis as u32 == 0 {
        return;
    }
    mirror_matrix_src(m, axis);
    mirror_matrix_dst(m, axis);
}

// ufbx.c:23523-23619 `ufbxi_update_initial_clusters`
#[inline(never)]
pub(crate) unsafe fn update_initial_clusters(scene: *mut Scene) {
    // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, scene->skin_clusters)`
    let mut p_cluster: *mut *mut SkinCluster = (*scene).skin_clusters.data as *mut *mut SkinCluster;
    let p_cluster_end: *mut *mut SkinCluster = add_ptr(p_cluster, (*scene).skin_clusters.count);
    while p_cluster != p_cluster_end {
        let cluster: *mut SkinCluster = *p_cluster;
        (*cluster).geometry_to_bone = (*cluster).mesh_node_to_bone;
        p_cluster = p_cluster.add(1);
    }

    let mirror_axis: MirrorAxis = (*scene).metadata.mirror_axis;
    let geometry_scale: Real = (*scene).metadata.geometry_scale;

    // Space conversion for bind matrices
    {
        // C: `ufbx_matrix world_to_units;` — written by both arms of the
        // `if` below (upstream carries no `// ufbxi_uninit` marker).
        let world_to_units: Matrix;
        let mut translation_scale: Real = 1.0 as Real;

        if (*scene).metadata.space_conversion == SpaceConversion::TransformRoot
            && (*scene).metadata.mirror_axis == MirrorAxis::None
        {
            world_to_units = (*ref_ptr(&(*scene).root_node)).node_to_parent;
        } else {
            // C: `ufbx_transform root_transform;` — every member is written
            // below before the first read.
            let mut root_transform: Transform = core::mem::zeroed();
            root_transform.translation = ZERO_VEC3;
            root_transform.rotation = (*scene).metadata.root_rotation;
            root_transform.scale.x = (*scene).metadata.root_scale;
            root_transform.scale.y = (*scene).metadata.root_scale;
            root_transform.scale.z = (*scene).metadata.root_scale;
            world_to_units = transform_to_matrix(&root_transform);
            translation_scale = (*scene).metadata.geometry_scale;
        }

        // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, scene->skin_clusters)`
        let mut p_cluster: *mut *mut SkinCluster =
            (*scene).skin_clusters.data as *mut *mut SkinCluster;
        let p_cluster_end: *mut *mut SkinCluster = add_ptr(p_cluster, (*scene).skin_clusters.count);
        while p_cluster != p_cluster_end {
            let cluster: *mut SkinCluster = *p_cluster;
            (*cluster).bind_to_world = matrix_mul(&world_to_units, &(*cluster).bind_to_world);
            // C: `cluster->bind_to_world.cols[3].x` — the `cols[4]` overlay.
            let bind_cols: *mut Vec3 = &mut (*cluster).bind_to_world as *mut Matrix as *mut Vec3;
            (*bind_cols.add(3)).x *= translation_scale;
            (*bind_cols.add(3)).y *= translation_scale;
            (*bind_cols.add(3)).z *= translation_scale;
            mirror_matrix(&mut (*cluster).bind_to_world, mirror_axis);
            p_cluster = p_cluster.add(1);
        }

        // C: `ufbxi_for_ptr_list(ufbx_pose, p_pose, scene->poses)`
        let mut p_pose: *mut *mut Pose = (*scene).poses.data as *mut *mut Pose;
        let p_pose_end: *mut *mut Pose = add_ptr(p_pose, (*scene).poses.count);
        while p_pose != p_pose_end {
            // C: `ufbxi_for_list(ufbx_bone_pose, pose, (*p_pose)->bone_poses)`
            let mut pose: *mut BonePose = (**p_pose).bone_poses.data as *mut BonePose;
            let pose_end: *mut BonePose = add_ptr(pose, (**p_pose).bone_poses.count);
            while pose != pose_end {
                (*pose).bone_to_world = matrix_mul(&world_to_units, &(*pose).bone_to_world);
                let pose_cols: *mut Vec3 = &mut (*pose).bone_to_world as *mut Matrix as *mut Vec3;
                (*pose_cols.add(3)).x *= translation_scale;
                (*pose_cols.add(3)).y *= translation_scale;
                (*pose_cols.add(3)).z *= translation_scale;
                mirror_matrix(&mut (*pose).bone_to_world, mirror_axis);
                pose = pose.add(1);
            }
            p_pose = p_pose.add(1);
        }
    }

    // Patch initial `mesh_node_to_bone`
    // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, scene->skin_clusters)`
    let mut p_cluster: *mut *mut SkinCluster = (*scene).skin_clusters.data as *mut *mut SkinCluster;
    let p_cluster_end: *mut *mut SkinCluster = add_ptr(p_cluster, (*scene).skin_clusters.count);
    while p_cluster != p_cluster_end {
        let cluster: *mut SkinCluster = *p_cluster;

        let skin: *mut SkinDeformer = fetch_src_element(
            &mut (*cluster).element,
            false,
            ptr::null(),
            ElementType::SkinDeformer,
        ) as *mut SkinDeformer;
        if skin.is_null() {
            p_cluster = p_cluster.add(1);
            continue;
        }

        let mut node: *mut Node =
            fetch_src_element(&mut (*skin).element, false, ptr::null(), ElementType::Node)
                as *mut Node;
        if node.is_null() {
            let mesh: *mut Mesh =
                fetch_src_element(&mut (*skin).element, false, ptr::null(), ElementType::Mesh)
                    as *mut Mesh;
            // C: `mesh->instances` — the `ufbx_mesh` element-header union view
            // (ufbx.h), which the generated struct keeps as `element.instances`.
            if !mesh.is_null() && (*mesh).element.instances.count > 0 {
                node = *((*mesh).element.instances.data as *const *mut Node);
            }
        }
        if node.is_null() {
            p_cluster = p_cluster.add(1);
            continue;
        }

        // Normalize to the non-helper node
        if (*node).is_geometry_transform_helper {
            node = opt_ptr(&(*node).parent);
        }

        if matrix_all_zero(&(*cluster).mesh_node_to_bone) {
            // If `mesh_node_to_bone` is not explicitly specified compute it from bind pose.
            let world_to_bind: Matrix = matrix_invert(&(*cluster).bind_to_world);
            (*cluster).mesh_node_to_bone = matrix_mul(&world_to_bind, &(*node).node_to_world);
        } else {
            // If `mesh_node_to_bone` is explicit, we may need to modify it for space conversion.
            mirror_matrix(&mut (*cluster).mesh_node_to_bone, mirror_axis);
            if geometry_scale != 1.0 {
                let cols: *mut Vec3 = &mut (*cluster).mesh_node_to_bone as *mut Matrix as *mut Vec3;
                (*cols.add(3)).x *= geometry_scale;
                (*cols.add(3)).y *= geometry_scale;
                (*cols.add(3)).z *= geometry_scale;
            }
        }

        // HACK: Account for geometry transforms by looking at the transform of the
        // helper node if one is present. I don't think this is exactly how the skinning
        // matrices are formed.
        // TODO: Add a test with moving the skinned mesh root around.
        // C: `if (node->geometry_transform_helper)` — pointer truthiness.
        if !opt_ptr(&(*node).geometry_transform_helper).is_null() {
            let geo_node: *mut Node = opt_ptr(&(*node).geometry_transform_helper);
            (*cluster).geometry_to_bone =
                matrix_mul(&(*cluster).mesh_node_to_bone, &(*geo_node).node_to_parent);
        } else if (*node).has_geometry_transform {
            (*cluster).geometry_to_bone =
                matrix_mul(&(*cluster).mesh_node_to_bone, &(*node).geometry_to_node);
        } else {
            (*cluster).geometry_to_bone = (*cluster).mesh_node_to_bone;
        }

        p_cluster = p_cluster.add(1);
    }
}

// ufbx.c:23621-23632 `ufbxi_find_axis`
#[inline(never)]
pub(crate) unsafe fn find_axis(
    props: *const Props,
    axis_name: *const u8,
    sign_name: *const u8,
) -> CoordinateAxis {
    let axis: i64 = find_int(props, axis_name, 3);
    let sign: i64 = find_int(props, sign_name, 2);

    match axis {
        0 => {
            if sign > 0 {
                CoordinateAxis::PositiveX
            } else {
                CoordinateAxis::NegativeX
            }
        }
        1 => {
            if sign > 0 {
                CoordinateAxis::PositiveY
            } else {
                CoordinateAxis::NegativeY
            }
        }
        2 => {
            if sign > 0 {
                CoordinateAxis::PositiveZ
            } else {
                CoordinateAxis::NegativeZ
            }
        }
        _ => CoordinateAxis::Unknown,
    }
}

// ufbx.c:23634-23653 `ufbxi_time_mode_fps`
// C initializes an `ufbx_real[]` from `float` constants, so each entry is
// `(double)(float)literal` when `ufbx_real` is `double` — the non-exact
// entries (29.97, 23.976, 59.94) are NOT their `double` nearest values.
static TIME_MODE_FPS: [Real; 18] = [
    30.0f32 as Real,   // UFBX_TIME_MODE_DEFAULT
    120.0f32 as Real,  // UFBX_TIME_MODE_120_FPS
    100.0f32 as Real,  // UFBX_TIME_MODE_100_FPS
    60.0f32 as Real,   // UFBX_TIME_MODE_60_FPS
    50.0f32 as Real,   // UFBX_TIME_MODE_50_FPS
    48.0f32 as Real,   // UFBX_TIME_MODE_48_FPS
    30.0f32 as Real,   // UFBX_TIME_MODE_30_FPS
    30.0f32 as Real,   // UFBX_TIME_MODE_30_FPS_DROP
    29.97f32 as Real,  // UFBX_TIME_MODE_NTSC_DROP_FRAME
    29.97f32 as Real,  // UFBX_TIME_MODE_NTSC_FULL_FRAME
    25.0f32 as Real,   // UFBX_TIME_MODE_PAL
    24.0f32 as Real,   // UFBX_TIME_MODE_24_FPS
    1000.0f32 as Real, // UFBX_TIME_MODE_1000_FPS
    23.976f32 as Real, // UFBX_TIME_MODE_FILM_FULL_FRAME
    24.0f32 as Real,   // UFBX_TIME_MODE_CUSTOM
    96.0f32 as Real,   // UFBX_TIME_MODE_96_FPS
    72.0f32 as Real,   // UFBX_TIME_MODE_72_FPS
    59.94f32 as Real,  // UFBX_TIME_MODE_59_94_FPS
];

// ufbx.c:23655-23674 `ufbxi_axis_matrix`
// Returns whether a non-identity matrix was needed
#[inline(never)]
pub(crate) unsafe fn axis_matrix(
    mat: *mut Matrix,
    src: CoordinateAxes,
    dst: CoordinateAxes,
) -> bool {
    let src_x: u32 = src.right as u32;
    let dst_x: u32 = dst.right as u32;
    let src_y: u32 = src.up as u32;
    let dst_y: u32 = dst.up as u32;
    let src_z: u32 = src.front as u32;
    let dst_z: u32 = dst.front as u32;

    if src_x == dst_x && src_y == dst_y && src_z == dst_z {
        return false;
    }

    // Remap axes (axis enum divided by 2) potentially flipping if the signs (enum parity) doesn't match
    ptr::write_bytes(mat as *mut u8, 0, size_of::<Matrix>());
    // C: `mat->cols[i].v[j]` — the `cols[4]` / `v[3]` union overlay.
    let cols: *mut Vec3 = mat as *mut Vec3;
    let cx: *mut Real = cols.add((src_x >> 1) as usize) as *mut Real;
    *cx.add((dst_x >> 1) as usize) = if ((src_x ^ dst_x) & 1) == 0 {
        1.0 as Real
    } else {
        -1.0 as Real
    };
    let cy: *mut Real = cols.add((src_y >> 1) as usize) as *mut Real;
    *cy.add((dst_y >> 1) as usize) = if ((src_y ^ dst_y) & 1) == 0 {
        1.0 as Real
    } else {
        -1.0 as Real
    };
    let cz: *mut Real = cols.add((src_z >> 1) as usize) as *mut Real;
    *cz.add((dst_z >> 1) as usize) = if ((src_z ^ dst_z) & 1) == 0 {
        1.0 as Real
    } else {
        -1.0 as Real
    };

    true
}

// ufbx.c:23676-23804 `ufbxi_update_adjust_transforms`
#[inline(never)]
pub(crate) unsafe fn update_adjust_transforms(uc: &Context, scene: *mut Scene) {
    let mut root_transform: Transform = IDENTITY_TRANSFORM;
    if !matrix_all_zero(&(*uc.get()).axis_matrix) {
        root_transform = matrix_to_transform(&(*uc.get()).axis_matrix);
    }
    root_transform.scale.x *= uc.unit_scale();
    root_transform.scale.y *= uc.unit_scale();
    root_transform.scale.z *= uc.unit_scale();

    let conversion: SpaceConversion = (*uc.get()).opts.space_conversion;

    let mut light_post_rotation: Quat = IDENTITY_QUAT;
    let mut camera_post_rotation: Quat = IDENTITY_QUAT;
    let mut light_direction: Vec3 = Vec3 {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    };
    let mut has_light_transform: bool = false;
    let mut has_camera_transform: bool = false;

    if coordinate_axes_valid((*uc.get()).opts.target_light_axes) {
        let mut mat_storage = MaybeUninit::<Matrix>::uninit(); // ufbxi_uninit
        let mat: *mut Matrix = mat_storage.as_mut_ptr();
        let light_axes: CoordinateAxes = CoordinateAxes {
            right: CoordinateAxis::PositiveX,
            up: CoordinateAxis::NegativeZ,
            front: CoordinateAxis::PositiveY,
        };
        if axis_matrix(mat, (*uc.get()).opts.target_light_axes, light_axes) {
            light_post_rotation = matrix_to_transform(mat).rotation;

            let inv: Matrix = matrix_invert(mat);
            light_direction = transform_direction(&inv, light_direction);
            has_light_transform = true;
        }
    }

    if coordinate_axes_valid((*uc.get()).opts.target_camera_axes) {
        let mut mat_storage = MaybeUninit::<Matrix>::uninit(); // ufbxi_uninit
        let mat: *mut Matrix = mat_storage.as_mut_ptr();
        let camera_axes: CoordinateAxes = CoordinateAxes {
            right: CoordinateAxis::PositiveZ,
            up: CoordinateAxis::PositiveY,
            front: CoordinateAxis::NegativeX,
        };
        if axis_matrix(mat, (*uc.get()).opts.target_camera_axes, camera_axes) {
            camera_post_rotation = matrix_to_transform(mat).rotation;
            has_camera_transform = true;
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_light, p_light, scene->lights)`
    let mut p_light: *mut *mut Light = (*scene).lights.data as *mut *mut Light;
    let p_light_end: *mut *mut Light = add_ptr(p_light, (*scene).lights.count);
    while p_light != p_light_end {
        let light: *mut Light = *p_light;
        (*light).local_direction.x = 0.0;
        (*light).local_direction.y = -1.0;
        (*light).local_direction.z = 0.0;
        p_light = p_light.add(1);
    }

    (*scene).metadata.space_conversion = conversion;
    (*scene).metadata.geometry_transform_handling = (*uc.get()).opts.geometry_transform_handling;
    (*scene).metadata.inherit_mode_handling = (*uc.get()).opts.inherit_mode_handling;
    (*scene).metadata.pivot_handling = (*uc.get()).opts.pivot_handling;
    (*scene).metadata.handedness_conversion_axis = (*uc.get()).opts.handedness_conversion_axis;

    let root_scale: Real = min3(root_transform.scale);
    if conversion == SpaceConversion::ModifyGeometry {
        (*scene).metadata.geometry_scale = root_scale;
        (*scene).metadata.root_scale = 1.0 as Real;
    } else {
        (*scene).metadata.geometry_scale = 1.0 as Real;
        (*scene).metadata.root_scale = root_scale;
    }
    (*scene).metadata.root_rotation = root_transform.rotation;

    // C: `ufbxi_for_ptr_list(ufbx_node, p_node, scene->nodes)`
    let mut p_node: *mut *mut Node = (*scene).nodes.data as *mut *mut Node;
    let p_node_end: *mut *mut Node = add_ptr(p_node, (*scene).nodes.count);
    while p_node != p_node_end {
        let node: *mut Node = *p_node;

        (*node).adjust_post_rotation = IDENTITY_QUAT;
        (*node).adjust_pre_rotation = IDENTITY_QUAT;
        (*node).adjust_pre_scale = 1.0 as Real;
        (*node).adjust_post_scale = 1.0 as Real;
        (*node).adjust_translation_scale = 1.0 as Real;

        if conversion == SpaceConversion::AdjustTransforms {
            if (*node).node_depth <= 1 && !(*node).is_root {
                (*node).adjust_pre_rotation = root_transform.rotation;
                (*node).adjust_pre_scale = root_scale;
                (*node).has_adjust_transform = true;
                (*node).has_root_adjust_transform = true;
            }
        } else if conversion == SpaceConversion::ModifyGeometry {
            if !(*node).is_root {
                if (*node).node_depth <= 1 {
                    (*node).adjust_pre_rotation = root_transform.rotation;
                }
                (*node).adjust_translation_scale = root_scale;
                (*node).has_adjust_transform = true;
            }
        }

        // C: `if (node->parent)` — pointer truthiness.
        if !opt_ptr(&(*node).parent).is_null() {
            // We are not inheriting local scale, so propagate root scale manually and
            // apply scale compensation if necessary.
            let parent: *mut Node = opt_ptr(&(*node).parent);
            if (*parent).has_root_adjust_transform
                && (*node).inherit_mode == InheritMode::IgnoreParentScale
            {
                (*node).adjust_post_scale *= root_scale;
                (*node).has_adjust_transform = true;
                (*node).has_root_adjust_transform = true;
            }
            if (*parent).is_scale_compensate_parent
                && (*node).original_inherit_mode == InheritMode::IgnoreParentScale
            {
                let scale: Vec3 = find_vec3(
                    &(*parent).element.props,
                    sp::Lcl_Scaling.as_ptr(),
                    1.0,
                    1.0,
                    1.0,
                );
                let mut size: Real = scale.x;
                // C: `ufbx_fabs(scale.y - 1.0f) < ufbx_fabs(size - 1.0f)` — real
                // subtractions, promoted to double at the `fabs` calls, compared
                // in double.
                if math::fabs((scale.y - 1.0) as f64) < math::fabs((size - 1.0) as f64) {
                    size = scale.y;
                }
                if math::fabs((scale.z - 1.0) as f64) < math::fabs((size - 1.0) as f64) {
                    size = scale.z;
                }
                (*node).adjust_post_scale *= 1.0 / size;
                (*node).has_adjust_transform = true;
            }
        }

        if (*node).all_attribs.count == 1 {
            // C: `if (has_light_transform && node->light)` — pointer truthiness.
            if has_light_transform && !opt_ptr(&(*node).light).is_null() {
                (*node).adjust_post_rotation = light_post_rotation;
                (*opt_ptr(&(*node).light)).local_direction = light_direction;
                (*node).has_adjust_transform = true;
            }
            if has_camera_transform && !opt_ptr(&(*node).camera).is_null() {
                (*node).adjust_post_rotation = camera_post_rotation;
                (*opt_ptr(&(*node).camera)).projection_axes = (*uc.get()).opts.target_camera_axes;
                (*node).has_adjust_transform = true;
            }
        }

        p_node = p_node.add(1);
    }
}

// ufbx.c:23806-23867 `ufbxi_update_scene`
#[inline(never)]
pub(crate) unsafe fn update_scene(
    scene: *mut Scene,
    initial: bool,
    transform_overrides: *const TransformOverride,
    num_transform_overrides: usize,
) {
    // C: `ufbxi_for_ptr_list(ufbx_node, p_node, scene->nodes)`
    let mut p_node: *mut *mut Node = (*scene).nodes.data as *mut *mut Node;
    let p_node_end: *mut *mut Node = add_ptr(p_node, (*scene).nodes.count);
    while p_node != p_node_end {
        update_node(*p_node, transform_overrides, num_transform_overrides);
        p_node = p_node.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_light, p_light, scene->lights)`
    let mut p_light: *mut *mut Light = (*scene).lights.data as *mut *mut Light;
    let p_light_end: *mut *mut Light = add_ptr(p_light, (*scene).lights.count);
    while p_light != p_light_end {
        update_light(*p_light);
        p_light = p_light.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_camera, p_camera, scene->cameras)`
    let mut p_camera: *mut *mut Camera = (*scene).cameras.data as *mut *mut Camera;
    let p_camera_end: *mut *mut Camera = add_ptr(p_camera, (*scene).cameras.count);
    while p_camera != p_camera_end {
        update_camera(scene, *p_camera);
        p_camera = p_camera.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_bone, p_bone, scene->bones)`
    let mut p_bone: *mut *mut Bone = (*scene).bones.data as *mut *mut Bone;
    let p_bone_end: *mut *mut Bone = add_ptr(p_bone, (*scene).bones.count);
    while p_bone != p_bone_end {
        update_bone(scene, *p_bone);
        p_bone = p_bone.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_line_curve, p_line, scene->line_curves)`
    let mut p_line: *mut *mut LineCurve = (*scene).line_curves.data as *mut *mut LineCurve;
    let p_line_end: *mut *mut LineCurve = add_ptr(p_line, (*scene).line_curves.count);
    while p_line != p_line_end {
        update_line_curve(*p_line);
        p_line = p_line.add(1);
    }

    if initial {
        update_initial_clusters(scene);

        // C: `ufbxi_for_ptr_list(ufbx_pose, p_pose, scene->poses)`
        let mut p_pose: *mut *mut Pose = (*scene).poses.data as *mut *mut Pose;
        let p_pose_end: *mut *mut Pose = add_ptr(p_pose, (*scene).poses.count);
        while p_pose != p_pose_end {
            update_pose(*p_pose);
            p_pose = p_pose.add(1);
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, scene->skin_clusters)`
    let mut p_cluster: *mut *mut SkinCluster = (*scene).skin_clusters.data as *mut *mut SkinCluster;
    let p_cluster_end: *mut *mut SkinCluster = add_ptr(p_cluster, (*scene).skin_clusters.count);
    while p_cluster != p_cluster_end {
        update_skin_cluster(*p_cluster);
        p_cluster = p_cluster.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_blend_channel, p_channel, scene->blend_channels)`
    let mut p_channel: *mut *mut BlendChannel =
        (*scene).blend_channels.data as *mut *mut BlendChannel;
    let p_channel_end: *mut *mut BlendChannel = add_ptr(p_channel, (*scene).blend_channels.count);
    while p_channel != p_channel_end {
        update_blend_channel(*p_channel);
        p_channel = p_channel.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, scene->textures)`
    let mut p_texture: *mut *mut Texture = (*scene).textures.data as *mut *mut Texture;
    let p_texture_end: *mut *mut Texture = add_ptr(p_texture, (*scene).textures.count);
    while p_texture != p_texture_end {
        update_texture(*p_texture);
        p_texture = p_texture.add(1);
    }

    propagate_main_textures(scene);

    // C: `ufbxi_for_ptr_list(ufbx_material, p_material, scene->materials)`
    let mut p_material: *mut *mut Material = (*scene).materials.data as *mut *mut Material;
    let p_material_end: *mut *mut Material = add_ptr(p_material, (*scene).materials.count);
    while p_material != p_material_end {
        update_material(scene, *p_material);
        p_material = p_material.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_stack, p_stack, scene->anim_stacks)`
    let mut p_stack: *mut *mut AnimStack = (*scene).anim_stacks.data as *mut *mut AnimStack;
    let p_stack_end: *mut *mut AnimStack = add_ptr(p_stack, (*scene).anim_stacks.count);
    while p_stack != p_stack_end {
        update_anim_stack(scene, *p_stack);
        p_stack = p_stack.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_display_layer, p_layer, scene->display_layers)`
    let mut p_layer: *mut *mut DisplayLayer =
        (*scene).display_layers.data as *mut *mut DisplayLayer;
    let p_layer_end: *mut *mut DisplayLayer = add_ptr(p_layer, (*scene).display_layers.count);
    while p_layer != p_layer_end {
        update_display_layer(*p_layer);
        p_layer = p_layer.add(1);
    }

    // C: `ufbxi_for_ptr_list(ufbx_constraint, p_constraint, scene->constraints)`
    let mut p_constraint: *mut *mut Constraint = (*scene).constraints.data as *mut *mut Constraint;
    let p_constraint_end: *mut *mut Constraint = add_ptr(p_constraint, (*scene).constraints.count);
    while p_constraint != p_constraint_end {
        update_constraint(*p_constraint);
        p_constraint = p_constraint.add(1);
    }

    update_anim(scene);
}

// ufbx.c:23869-23878 `ufbxi_update_scene_metadata`
#[inline(never)]
pub(crate) unsafe fn update_scene_metadata(metadata: *mut Metadata) {
    let props: *mut Props = &mut (*metadata).scene_props;
    (*metadata).original_application.vendor = find_string(
        props,
        b"Original|ApplicationVendor\0".as_ptr(),
        EMPTY_STRING.0,
    );
    (*metadata).original_application.name = find_string(
        props,
        b"Original|ApplicationName\0".as_ptr(),
        EMPTY_STRING.0,
    );
    (*metadata).original_application.version = find_string(
        props,
        b"Original|ApplicationVersion\0".as_ptr(),
        EMPTY_STRING.0,
    );
    (*metadata).latest_application.vendor = find_string(
        props,
        b"LastSaved|ApplicationVendor\0".as_ptr(),
        EMPTY_STRING.0,
    );
    (*metadata).latest_application.name = find_string(
        props,
        b"LastSaved|ApplicationName\0".as_ptr(),
        EMPTY_STRING.0,
    );
    (*metadata).latest_application.version = find_string(
        props,
        b"LastSaved|ApplicationVersion\0".as_ptr(),
        EMPTY_STRING.0,
    );
}

// ufbx.c:23880-23887 `ufbxi_pow10_targets`
// C: the first entry is a `float` constant, the rest are `double` constants
// explicitly cast to `ufbx_real` — kept verbatim (all are exactly
// representable at both widths except the negative powers, whose `double`
// values C uses directly).
pub(crate) static POW10_TARGETS: [Real; 19] = [
    0.0f32 as Real,
    1e-8 as Real,
    1e-7 as Real,
    1e-6 as Real,
    1e-5 as Real,
    1e-4 as Real,
    1e-3 as Real,
    1e-2 as Real,
    1e-1 as Real,
    1e+0 as Real,
    1e+1 as Real,
    1e+2 as Real,
    1e+3 as Real,
    1e+4 as Real,
    1e+5 as Real,
    1e+6 as Real,
    1e+7 as Real,
    1e+8 as Real,
    1e+9 as Real,
];

// ufbx.c:23889-23901 `ufbxi_round_if_near`
#[inline(never)]
pub(crate) unsafe fn round_if_near(targets: *const Real, num_targets: usize, value: Real) -> Real {
    for i in 0..num_targets {
        // C: `double target = targets[i];` — the real target promotes to
        // double, and the range test below compares `value` in double too.
        let target: f64 = *targets.add(i) as f64;
        let mut error: f64 = target * 9.5367431640625e-7;
        if error < 0.0 {
            error = -error;
        }
        if error < 7.52316384526264005e-37 {
            error = 7.52316384526264005e-37;
        }
        if value as f64 >= target - error && value as f64 <= target + error {
            return target as Real;
        }
    }
    value
}

// ufbx.c:23903-23931 `ufbxi_update_scene_settings`
#[inline(never)]
pub(crate) unsafe fn update_scene_settings(settings: *mut SceneSettings) {
    let unit_scale_factor: Real = find_real(
        &(*settings).props,
        sp::UnitScaleFactor.as_ptr(),
        1.0 as Real,
    );
    let original_unit_scale_factor: Real = find_real(
        &(*settings).props,
        sp::OriginalUnitScaleFactor.as_ptr(),
        unit_scale_factor,
    );

    (*settings).axes.up = find_axis(
        &(*settings).props,
        sp::UpAxis.as_ptr(),
        sp::UpAxisSign.as_ptr(),
    );
    (*settings).axes.front = find_axis(
        &(*settings).props,
        sp::FrontAxis.as_ptr(),
        sp::FrontAxisSign.as_ptr(),
    );
    (*settings).axes.right = find_axis(
        &(*settings).props,
        sp::CoordAxis.as_ptr(),
        sp::CoordAxisSign.as_ptr(),
    );
    (*settings).unit_meters = round_if_near(
        POW10_TARGETS.as_ptr(),
        POW10_TARGETS.len(),
        unit_scale_factor * (0.01 as Real),
    );
    (*settings).original_unit_meters = round_if_near(
        POW10_TARGETS.as_ptr(),
        POW10_TARGETS.len(),
        original_unit_scale_factor * (0.01 as Real),
    );
    // C: `settings->frames_per_second` is `double` — the `ufbxi_find_real`
    // result promotes on assignment.
    (*settings).frames_per_second = find_real(
        &(*settings).props,
        sp::CustomFrameRate.as_ptr(),
        24.0 as Real,
    ) as f64;
    (*settings).ambient_color =
        find_vec3(&(*settings).props, sp::AmbientColor.as_ptr(), 0.0, 0.0, 0.0);
    (*settings).original_axis_up = find_axis(
        &(*settings).props,
        sp::OriginalUpAxis.as_ptr(),
        sp::OriginalUpAxisSign.as_ptr(),
    );

    let default_camera: *mut Prop = find_prop(&(*settings).props, sp::DefaultCamera.as_ptr());
    if !default_camera.is_null() {
        (*settings).default_camera = (*default_camera).value_str;
    } else {
        (*settings).default_camera = EMPTY_STRING.0;
    }

    // C: `(ufbx_time_mode)ufbxi_find_enum(...)` etc — `ufbxi_find_enum` clamps
    // each result to its enum's `[0, LAST]` range (same device as
    // `ufbxi_update_camera` above).
    (*settings).time_mode = core::mem::transmute::<u32, TimeMode>(find_enum(
        &(*settings).props,
        sp::TimeMode.as_ptr(),
        TimeMode::E24Fps as i64,
        TimeMode::E5994Fps as i64,
    ) as u32);
    (*settings).time_protocol = core::mem::transmute::<u32, TimeProtocol>(find_enum(
        &(*settings).props,
        sp::TimeProtocol.as_ptr(),
        TimeProtocol::Default as i64,
        TimeProtocol::Default as i64,
    ) as u32);
    (*settings).snap_mode = core::mem::transmute::<u32, SnapMode>(find_enum(
        &(*settings).props,
        sp::SnapOnFrameMode.as_ptr(),
        SnapMode::None as i64,
        SnapMode::SnapAndPlay as i64,
    ) as u32);

    if (*settings).time_mode != TimeMode::Custom {
        // C: real `ufbxi_time_mode_fps[]` entry promotes to the `double` field.
        (*settings).frames_per_second = TIME_MODE_FPS[(*settings).time_mode as u32 as usize] as f64;
    }
}

// ufbx.c:23933-23944 `ufbxi_update_scene_settings_obj`
#[inline(never)]
pub(crate) unsafe fn update_scene_settings_obj(uc: &Context) {
    let settings: *mut SceneSettings = &mut (*uc.get()).scene.settings;
    // C: `settings->original_unit_meters = settings->unit_meters = uc->opts.obj_unit_meters;`
    (*settings).unit_meters = (*uc.get()).opts.obj_unit_meters;
    (*settings).original_unit_meters = (*settings).unit_meters;
    if coordinate_axes_valid((*uc.get()).opts.obj_axes) {
        (*settings).axes = (*uc.get()).opts.obj_axes;
    } else {
        (*settings).axes.right = CoordinateAxis::Unknown;
        (*settings).axes.up = CoordinateAxis::Unknown;
        (*settings).axes.front = CoordinateAxis::Unknown;
    }
}

// CONTINUATION POINT: `// -- Scene processing` (ufbx.c:18545-22624) is ported
// in FULL, including `ufbxi_finalize_scene`, plus
// `// -- Interpret the read scene` in full (ufbx.c:22626-22741) and
// `// -- Updating state from properties` in FULL (ufbx.c:22743-23944) — this
// unit finished the banner section at `ufbxi_update_scene_settings_obj`.
//
// Next: `// -- Geometry caches` (ufbx.c:23946-24785), gated on
// `UFBXI_FEATURE_GEOMETRY_CACHE` and blocked on `// -- XML` (ufbx.c:7245-7682,
// `native::xml`, still a stub).
//
// NOTE: ufbx.c:22600's `ufbxi_update_scene()` mention is a COMMENT, not a call.
// `ufbxi_finalize_scene`, `ufbxi_update_scene`, `ufbxi_update_adjust_transforms`,
// `ufbxi_update_scene_metadata`, `ufbxi_update_scene_settings` and
// `ufbxi_update_scene_settings_obj` still have no callers — every one of them is
// called only from `ufbxi_load_imp` (ufbx.c:25204+, unported) or
// `ufbxi_evaluate_imp` (ufbx.c:26403, unported). `ufbxi_round_if_near` and
// `ufbxi_pow10_targets` also have call sites in `ufbxi_scale_unit_scale`
// (ufbx.c:24986/24989, unported), and `ufbxi_axis_matrix` /
// `ufbxi_mirror_matrix_dst` in `ufbxi_setup_axis_matrix` (ufbx.c:24949/24957,
// unported); all of these are ported here because C defines them here.

#[cfg(test)]
mod tests {
    use super::*;

    const FBX_MAP_COUNT: u8 = MaterialFbxMap::VectorDisplacement as u8 + 1;
    const PBR_MAP_COUNT: u8 = MaterialPbrMap::TransmissionGlossiness as u8 + 1;
    const FEATURE_COUNT: u8 = MaterialFeature::TransmissionRoughnessAsGlossiness as u8 + 1;

    // Transcription guard for the ufbx.c:19443-19960 tables. C derives
    // `prop_len` from the literal via `sizeof(str) - 1`, so it must equal the
    // NUL-terminated literal's length here; `index` and `transform` stand in
    // for enums whose ranges C checks nowhere at runtime.
    unsafe fn check_table(table: &[ShaderMapping], index_count: u8) {
        for mapping in table {
            assert!(mapping.transform < MatTransform::Count as u8);
            assert!(mapping.index < index_count);
            let mut len: usize = 0;
            while *mapping.prop.add(len) != 0 {
                len += 1;
            }
            assert_eq!(len, mapping.prop_len as usize);
        }
    }

    // `{ NULL, 0 }` or a NUL-terminated literal with its exact length. Lengths
    // are derived by `ufbxi_string_literal!` (`.len() - 1`), so `data[length]`
    // is always the literal's last byte — in bounds; the check catches a
    // literal transcribed without its explicit NUL terminator.
    unsafe fn check_string(str_: &String) {
        if str_.data.is_null() {
            assert_eq!(str_.length, 0);
        } else {
            assert_eq!(*str_.data.add(str_.length), 0);
            assert!(str_.length > 0);
        }
    }

    #[test]
    fn shader_mapping_tables_are_consistent() {
        unsafe {
            check_table(&BASE_FBX_MAPPING, FBX_MAP_COUNT);
            check_table(&OBJ_FBX_MAPPING, FBX_MAP_COUNT);
            for list in SHADER_PBR_MAPPINGS.iter() {
                assert!(!list.data.is_null() && list.count > 0);
                check_table(
                    core::slice::from_raw_parts(list.data, list.count),
                    PBR_MAP_COUNT,
                );
                if list.features.is_null() {
                    assert_eq!(list.feature_count, 0);
                } else {
                    check_table(
                        core::slice::from_raw_parts(list.features, list.feature_count),
                        FEATURE_COUNT,
                    );
                }
                check_string(&list.texture_prefix);
                check_string(&list.texture_suffix);
                check_string(&list.texture_enabled_prefix);
                check_string(&list.texture_enabled_suffix);
            }
        }
    }

    // Transcription guard for the ufbx.c:20118-20122 and ufbx.c:20231-20242
    // tables: `ufbxi_fetch_maps` and `ufbxi_add_constraint_prop` index the
    // `pbr.maps[]` / `features[]` union views with these `uint8_t` fields, and
    // `strcmp` walks the constraint names, so both the ranges and the explicit
    // NUL terminators must hold.
    #[test]
    fn glossiness_and_constraint_tables_are_consistent() {
        unsafe {
            for remap in GLOSSINESS_REMAPS.iter() {
                assert!(remap.feature < FEATURE_COUNT);
                assert!(remap.roughness_map < PBR_MAP_COUNT);
                assert!(remap.glossiness_map < PBR_MAP_COUNT);
            }
            for cprop in CONSTRAINT_PROPS.iter() {
                let mut len: usize = 0;
                while *cprop.name.add(len) != 0 {
                    len += 1;
                }
                assert!(len > 0);
            }
        }
    }

    // Transcription guard for the ufbx.c:20486-20494 table:
    // `ufbxi_finalize_shader_texture` walks `shader_name` with `strcmp` and
    // hands `input_name` to `ufbx_find_shader_texture_input` (which `strlen`s
    // it), so both literals need their explicit NUL terminator. The `0`
    // `shader_id` entries are matched by name only, which is why the guard
    // insists the names are non-empty.
    #[test]
    fn file_shader_table_is_consistent() {
        unsafe {
            for fs in FILE_SHADERS.iter() {
                let mut name_len: usize = 0;
                while *fs.shader_name.add(name_len) != 0 {
                    name_len += 1;
                }
                assert!(name_len > 0);
                let mut input_len: usize = 0;
                while *fs.input_name.add(input_len) != 0 {
                    input_len += 1;
                }
                assert!(input_len > 0);
            }
            // Only the first entry carries a class id (ufbx.c:20488).
            assert_eq!(FILE_SHADERS[0].shader_id, 0x7e73161fad53b12a);
            for fs in FILE_SHADERS[1..].iter() {
                assert_eq!(fs.shader_id, 0);
            }
        }
    }

    // `ufbxi_finalize_shader_texture` carries `UFBX_SHADER_TEXTURE_TYPE_COUNT`
    // as an out-of-range `ufbx_shader_texture_type` sentinel (ufbx.c:20546) and
    // transmutes the value back once the range check at ufbx.c:20556 passed;
    // the transmute is only sound while the enum's variants are exactly
    // `0..COUNT`.
    #[test]
    fn shader_texture_type_count_matches_enum() {
        assert_eq!(SHADER_TEXTURE_TYPE_COUNT, 3);
        assert_eq!(ShaderTextureType::Unknown as u32, 0);
        assert_eq!(ShaderTextureType::SelectOutput as u32, 1);
        assert_eq!(ShaderTextureType::Osl as u32, 2);
    }

    // `ufbxi_update_scene_settings` indexes `ufbxi_time_mode_fps` with an
    // unchecked `settings->time_mode` (ufbx.c:23929), so the table must cover
    // every `ufbx_time_mode` — `ufbxi_find_enum` only clamps to
    // `UFBX_TIME_MODE_59_94_FPS`, the last variant.
    #[test]
    fn time_mode_fps_table_covers_every_time_mode() {
        assert_eq!(TIME_MODE_FPS.len(), TimeMode::E5994Fps as usize + 1);
    }

    // C initializes `ufbxi_time_mode_fps` (an `ufbx_real[]`) from `float`
    // constants, so with `ufbx_real == double` the non-exact entries are
    // `(double)(float)literal`, NOT the `double` nearest value. Writing them
    // as bare `f64` literals would shift `frames_per_second` in the low bits —
    // straight into the scene hash.
    #[test]
    fn time_mode_fps_entries_keep_float_widening() {
        assert_eq!(TIME_MODE_FPS[8], 29.97f32 as Real);
        assert_eq!(TIME_MODE_FPS[9], 29.97f32 as Real);
        assert_eq!(TIME_MODE_FPS[13], 23.976f32 as Real);
        assert_eq!(TIME_MODE_FPS[17], 59.94f32 as Real);
        // The float-widening distinction only exists with `Real == f64`; under
        // `real-is-f32` the `f32` and widened values coincide.
        #[cfg(not(feature = "real-is-f32"))]
        {
            assert_ne!(TIME_MODE_FPS[8], 29.97f64);
            assert_ne!(TIME_MODE_FPS[13], 23.976f64);
            assert_ne!(TIME_MODE_FPS[17], 59.94f64);
        }
    }

    // Conversely `ufbxi_pow10_targets` (ufbx.c:23880-23887) casts `double`
    // constants, so every entry past the leading `0.0f` IS the `double`
    // nearest value — the two tables are transcribed with different rules.
    #[test]
    fn pow10_targets_table_is_transcribed() {
        assert_eq!(POW10_TARGETS.len(), 19);
        assert_eq!(POW10_TARGETS[0], 0.0);
        assert_eq!(POW10_TARGETS[1], 1e-8f64 as Real);
        assert_eq!(POW10_TARGETS[9], 1e+0f64 as Real);
        assert_eq!(POW10_TARGETS[18], 1e+9f64 as Real);
    }

    // `ufbxi_update_adjust_transforms` builds the light/camera target axes as
    // brace initializers over `{ right, up, front }` (ufbx.c:23696-23699 and
    // 23712-23715); a reordered field list here is invisible to the compiler.
    #[test]
    fn coordinate_axes_field_order() {
        let axes = CoordinateAxes {
            right: CoordinateAxis::PositiveX,
            up: CoordinateAxis::NegativeZ,
            front: CoordinateAxis::PositiveY,
        };
        let raw: [u32; 3] = unsafe { core::mem::transmute(axes) };
        assert_eq!(raw[0], CoordinateAxis::PositiveX as u32);
        assert_eq!(raw[1], CoordinateAxis::NegativeZ as u32);
        assert_eq!(raw[2], CoordinateAxis::PositiveY as u32);
    }
}
