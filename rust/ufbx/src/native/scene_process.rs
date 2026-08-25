//! Port of `// -- Scene pre-processing` (ufbx.c:18066-18543),
//! `// -- Scene processing` (ufbx.c:18545-22624),
//! `// -- Interpret the read scene` (ufbx.c:22626-22741), and
//! `// -- Updating state from properties` (ufbx.c:22743-23944).
//!
//! Coverage: ufbx.c:18066-18543 — the `ufbxi_pre_*` scratch records, the two
//! pivot helpers and `ufbxi_pre_finalize_scene`: the multi-pass connection
//! walk that runs between parsing and `ufbxi_finalize_scene()` and builds the
//! parent/child linked list, the per-attribute instance counts, the constant
//! scale/anim-value analysis, the pivot adjustment (synthetic
//! `RotationPivot`/`ScalingPivot`/`ScalingOffset`/`GeometricTranslation`
//! properties + `adjust_pre_translation` fixups) and the geometry-transform /
//! scale helper node setup.
//!
//! Coverage: ufbx.c:18545-18995 — the element-graph builders: the
//! comparators and their paired `ufbxi_grow_array`+`ufbxi_macro_stable_sort`
//! wrappers, `ufbxi_resolve_connections` (fbx-id → element pointers, with the
//! pre-7000 property-to-attribute hack and the geometry-transform / scale
//! helper remapping), `ufbxi_add_connections_to_elements` (per-element
//! connection ranges + synthetic animated properties) and
//! `ufbxi_linearize_nodes` (parent hookup, depths, node ordering and the
//! `tmp_typed_element_offsets` bookkeeping that `ufbxi_finalize_scene()` later
//! turns into typed element pointers).
//!
//! Coverage: ufbx.c:18997-19441 — the connection-query layer the finalize
//! pass is built on: the `ufbxi_find_(dst|src)_connections` bounded searches
//! over the per-element connection ranges, the `ufbxi_fetch_*` helpers that
//! materialize typed element/texture/material/deformer/keyframe/layer lists
//! into `uc->result`, `ufbxi_find_prop_connection`, the index-sentinel patcher,
//! the remaining comparator + paired `ufbxi_grow_array`+sort wrappers, and the
//! head of the material tables (transform functions, mapping/feature flags and
//! the `ufbxi_shader_mapping(_list)` record types).
//!
//! Coverage: ufbx.c:19443-19960 — the material mapping tables themselves:
//! the `ufbxi_mat_string` initializer helper, the 21 per-shader
//! `ufbxi_shader_mapping` property/feature tables (FBX, OBJ/MTL, OSL/Arnold
//! standard surface, 3ds Max physical + PBR, glTF, OpenPBR, ShaderFX, Blender
//! phong), the `UFBXI_MAT_*` feature bit constants and the
//! `ufbxi_shader_pbr_mappings` per-`ufbx_shader_type` dispatch table that
//! `ufbxi_fetch_mapping_maps()` walks. Pure data: no allocation and no control
//! flow, so the parity surface is entry order, the flag/transform values and
//! the NULL-vs-empty texture prefix/suffix strings.
//!
//! Coverage: ufbx.c:19962-20427 — the table consumers and the first
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
//! Coverage: ufbx.c:20429-20867 — the shader-texture and texture-file
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
//! Coverage: ufbx.c:20869-21450 — `ufbxi_fetch_file_textures` (the
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
//! lives at its C-order slot; its dependencies are defined later below.
//!
//! Coverage: ufbx.c:21452-21638 — the last helpers `ufbxi_finalize_scene()`
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
//! The related transform helpers follow `ufbxi_finalize_scene`
//! (ufbx.c:21640-22624) and cover
//! `// -- Interpret the read scene` in full (ufbx.c:22626-22741: the transform
//! composition family `ufbxi_add_translate` … `ufbxi_mul_inv_rotate`) plus the
//! head of `// -- Updating state from properties` (ufbx.c:22743-22784:
//! `ufbxi_mirror_translation` / `ufbxi_mirror_rotation` /
//! `ufbxi_get_geometry_transform`), which `ufbxi_modify_geometry`
//! (ufbx.c:21165-21332) calls. Related public leaves are `ufbx_find_blob(_len)`,
//! `ufbx_quat_rotate_vec3`, `ufbx_euler_to_quat`, `ufbx_matrix_determinant` and
//! `ufbx_matrix_for_normals` in `native::api`; `ufbxi_matrix_all_zero`,
//! `ufbxi_is_quat_identity`, `ufbxi_is_vec3_equal`, `ufbxi_is_quat_equal` and
//! `ufbxi_is_transform_identity` fill the ufbx.c:11566-11607 gap in
//! `native::parse`.
//!
//! Coverage: ufbx.c:21641-22624 — `ufbxi_finalize_scene` alone, the single
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
//! Coverage: ufbx.c:22786-23062 — the node-transform derivation head of
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
//! Coverage: ufbx.c:23064-23495 — the rest of
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
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
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
    compute_normals, compute_topology, coordinate_axes_valid, euler_to_quat, find_blob_len,
    find_bool_len as api_find_bool_len, find_int_len as api_find_int_len, find_prop_concat,
    find_prop_len, find_prop_texture_len, find_real_len as api_find_real_len,
    find_shader_prop_bindings_len, find_shader_texture_input, find_shader_texture_input_len,
    find_string_len, find_vec3_len as api_find_vec3_len, generate_normal_mapping, get_bone_pose,
    get_prop_element, matrix_for_normals, matrix_invert, matrix_mul, matrix_to_transform,
    quat_rotate_vec3, transform_direction, transform_position, transform_to_matrix, EMPTY_BLOB,
    EMPTY_STRING, IDENTITY_MATRIX, IDENTITY_QUAT, IDENTITY_TRANSFORM, ZERO_VEC3,
};
use crate::native::buf::{buf_clear, buf_free, pop, BufView};
use crate::native::error::{
    c_strcmp, memcmp, strcmp, strlen, ufbxi_check, ufbxi_check_err, ufbxi_check_msg,
    ufbxi_snprintf, Fail, EMPTY_CHAR,
};
use crate::native::hash::{hash64, hash_ptr};
use crate::native::parse::{
    find_enum, find_int, find_prop, find_prop_with_key, find_real, find_vec3, get_element_extra,
    get_name_key, is_node_property_name, is_quat_identity, is_transform_identity, is_vec3_zero,
    is_vec4_zero, matrix_all_zero, name_key_less, Context, FbxAttrEntry, FbxIdEntry, FileContent,
    MeshExtra, PropView, PropsView, Refcount, SceneMetadataView, SceneSettingsView, SceneView,
    TextureExtra, TextureFileEntry, TmpBonePose, TmpConnection, TmpMaterialTexture, TmpMeshTexture,
    ELEMENT_TYPE_COUNT,
};
// Only reachable from the two `ufbxi_regression_assert`s in `ufbxi_get_transform`
// (ufbx.c:22901-22902), which is why C marks both `ufbxi_unused` (11594/11599).
#[cfg(feature = "regression")]
use crate::native::parse::{is_quat_equal, is_vec3_equal};
use crate::native::platform::{
    add_ptr, f64_to_i64, macro_lower_bound_eq, macro_stable_sort, macro_stable_sort_ptr_views,
    macro_stable_sort_views, math, max32, max_sz, min32, min_sz, pack_version, stable_sort,
    to_size, ufbx_assert, ufbxi_dev_assert, ufbxi_ignore, ufbxi_regression_assert,
    ufbxi_string_literal, ufbxi_unreachable, unstable_sort, NO_INDEX,
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
use crate::native::view::{
    view_project, view_read, view_read_shared, view_write, Const, Mode, SliceViewIter, View,
};
use crate::native::warnings::ufbxi_warnf_tag;
use crate::prelude::as_f64;
use crate::prelude::{
    slice_from_ptr, Blob, List, ListView, Real, Ref, RefList, RefListView, String, StringView,
};

// Rust-port infrastructure (not a ufbx.c section): reinterpret-in-place VIEWS
// over the scene-graph element structs the `ufbxi_update_*` family mutates.
//
// Each alias is `View<T>` (the shared reinterpret type) plus a `props_view()`
// accessor that hands back a `&PropsView` correlated to `&self`. Because the
// returned lifetime is tied to the element borrow — which the dispatch
// (`ufbxi_update_scene`) anchors to `uc.scene_view()` — a property table found
// through an element view can never outlive the arena-anchored `uc` scope, so
// the property finders it feeds stay provably `<= uc` with no free-lifetime
// bridge minted per call site.
macro_rules! element_props_view {
    ($alias:ident, $ty:ty) => {
        pub(crate) type $alias = crate::native::view::View<$ty>;
        impl crate::native::view::View<$ty> {
            #[inline(always)]
            pub(crate) fn props_view(&self) -> &PropsView {
                // SAFETY: reinterpret the embedded `element.props` field in place
                // (never a `&mut`, so writes through the element view do not
                // retag it); interior-mutable, asserts no validity, correlated
                // to `&self` (<= uc).
                unsafe { PropsView::from_ptr(&raw mut (*self.get()).element.props) }
            }
        }
    };
}

element_props_view!(NodeView, Node);
element_props_view!(LightView, Light);
element_props_view!(CameraView, Camera);
element_props_view!(BoneView, Bone);
element_props_view!(LineCurveView, LineCurve);
element_props_view!(BlendChannelView, BlendChannel);
element_props_view!(TextureView, Texture);
element_props_view!(AnimStackView, AnimStack);
element_props_view!(DisplayLayerView, DisplayLayer);
element_props_view!(ConstraintView, Constraint);
element_props_view!(MaterialView, Material);
element_props_view!(LodGroupView, LodGroup);
element_props_view!(CacheDeformerView, CacheDeformer);
element_props_view!(CacheFileView, CacheFile);
element_props_view!(AudioClipView, AudioClip);
element_props_view!(AnimValueView, AnimValue);
element_props_view!(AnimLayerView, AnimLayer);
element_props_view!(ShaderView, Shader);

// Plain view aliases (no `props_view()` accessor) for the remaining
// `ufbxi_update_*` targets: `update_pose`/`update_skin_cluster` never consult
// the property table, and `ufbx_shader_texture`/`ufbx_material_map` are not
// elements (no `element.props` header to project).
pub(crate) type PoseView = crate::native::view::View<Pose>;
pub(crate) type SkinClusterView = crate::native::view::View<SkinCluster>;
pub(crate) type ShaderTextureView = crate::native::view::View<ShaderTexture>;
pub(crate) type MaterialMapView = crate::native::view::View<MaterialMap>;

// The finalize path also walks the untyped `ufbx_element` run (the arena
// `element_data` block); an `ElementView` reinterprets those `*mut Element`
// pointers so the property table reached through them anchors to `uc` exactly
// like the typed element views above, differing only in that `ufbx_element`
// carries its `props` field directly (no embedded `element` member).
pub(crate) type ElementView = crate::native::view::View<Element>;
impl crate::native::view::View<Element> {
    #[inline(always)]
    pub(crate) fn props_view(&self) -> &PropsView {
        // SAFETY: reinterpret the embedded `props` field in place (never a
        // `&mut`, so writes through the element view do not retag it);
        // interior-mutable, asserts no validity, correlated to `&self` (<= uc).
        unsafe { PropsView::from_ptr(&raw mut (*self.get()).props) }
    }
}

// C indexes `uc->scene.elements_by_type[type]`, the `ufbx_element_list` array
// view of the `ufbx_scene` per-type list union (ufbx.h:4015); the generated
// struct keeps only the named branch, whose first member (`unknowns`) is the
// array base. The finalize pass reaches every branch through that base, so the
// index lives here once rather than at each walk.
impl crate::native::view::View<Scene> {
    #[inline(always)]
    pub(crate) fn elements_by_type_at(
        &self,
        type_: usize,
    ) -> &crate::prelude::RefListView<Element> {
        assert!(type_ < ELEMENT_TYPE_COUNT);
        // SAFETY: `unknowns_mut_ptr` addresses the first of the scene's
        // `ELEMENT_TYPE_COUNT` per-type list members, which share one flat
        // layout of identically shaped `ufbx_element_list`s, so the
        // bounds-checked index stays inside the scene's own allocation and
        // inherits its write-capable provenance.
        unsafe {
            crate::prelude::RefListView::<Element>::from_ptr(
                (self.unknowns_mut_ptr() as *mut RefList<Element>).add(type_),
            )
        }
    }
}

// C passes each `ufbx_*_list` out-parameter to `ufbxi_fetch_dst_elements` as a
// `void *p_dst_list` and casts it back to `ufbx_element_list *` inside
// (ufbx.c:19077). The Rust fetch takes a `&RefListView<Element>`, so the same
// erasure is one named projection on the caller's own typed list view rather
// than a pun re-argued at each fetch site.
impl<T> crate::prelude::RefListView<T> {
    #[inline(always)]
    pub(crate) fn as_element_list(&self) -> &crate::prelude::RefListView<Element> {
        // SAFETY: every generated `ufbx_*_list` is layout-identical to
        // `ufbx_element_list` — a `data` pointer plus a `count` — and every
        // `Ref<T>` slot it describes names an `ufbx_element`-headed element, so
        // the re-view addresses exactly the two leaves this view already covers
        // and inherits its own write-capable provenance.
        unsafe { crate::prelude::RefListView::<Element>::mint(self.get() as *mut RefList<Element>) }
    }
}

// `ufbxi_update_scene` (ufbx.c:23806) dispatches over the scene's per-type
// element lists; the three whose `*_view()` projections are not among the
// `SceneView` accessors in `native/parse.rs` live here.
impl crate::native::view::View<Scene> {
    #[inline(always)]
    pub(crate) fn lights_view(&self) -> &crate::prelude::RefListView<Light> {
        view_project!(self, lights)
    }
    #[inline(always)]
    pub(crate) fn cameras_view(&self) -> &crate::prelude::RefListView<Camera> {
        view_project!(self, cameras)
    }
    #[inline(always)]
    pub(crate) fn bones_view(&self) -> &crate::prelude::RefListView<Bone> {
        view_project!(self, bones)
    }
}

// `ufbxi_update_initial_clusters` (ufbx.c:23523) reads four `ufbx_metadata`
// leaves whose getters are not among the `SceneMetadataView` accessors in
// `native/parse.rs`; they live here, together with the one `ufbxi_update_camera`
// needs.
impl SceneMetadataView {
    #[inline(always)]
    pub(crate) fn space_conversion(&self) -> SpaceConversion {
        view_read!(self, space_conversion)
    }
    #[inline(always)]
    pub(crate) fn root_rotation(&self) -> Quat {
        view_read!(self, root_rotation)
    }
    #[inline(always)]
    pub(crate) fn root_scale(&self) -> Real {
        view_read!(self, root_scale)
    }
    #[inline(always)]
    pub(crate) fn mirror_axis(&self) -> MirrorAxis {
        view_read!(self, mirror_axis)
    }
    // `ufbxi_update_camera` (ufbx.c:23093) reads the orthographic size unit,
    // whose getter is likewise absent from `native/parse.rs`.
    #[inline(always)]
    pub(crate) fn ortho_size_unit(&self) -> Real {
        view_read!(self, ortho_size_unit)
    }
}

// `ufbxi_node_extra` (ufbx.c:12507-12510) is an internal scratch record rather
// than a public element, so it has no generated view; the finalize pass reads
// one leaf field out of it.
impl crate::native::view::View<NodeExtra> {
    #[inline(always)]
    pub(crate) fn scale_helper_id(&self) -> u32 {
        view_read!(self, scale_helper_id)
    }
}

// `ufbxi_tmp_bone_pose` (ufbx.c:6329-6332) is likewise an internal scratch
// record with no generated view; the bind-pose filter reads both of its fields
// out of the run stashed in `pose->bone_poses`.
impl crate::native::view::View<TmpBonePose> {
    #[inline(always)]
    pub(crate) fn bone_fbx_id(&self) -> u64 {
        view_read!(self, bone_fbx_id)
    }
    #[inline(always)]
    pub(crate) fn bone_to_world(&self) -> Matrix {
        view_read!(self, bone_to_world)
    }
}

// `ufbxi_tmp_material_texture` (ufbx.c:6346-6350) is an internal scratch record
// with no generated view; the legacy LayerElement-texture patch walks the
// sorted run of them through element views.
impl<M: Mode> crate::native::view::View<TmpMaterialTexture, M> {
    #[inline(always)]
    pub(crate) fn material_id(&self) -> i32 {
        view_read_shared!(self, material_id)
    }
    #[inline(always)]
    pub(crate) fn texture_id(&self) -> i32 {
        view_read_shared!(self, texture_id)
    }
    #[inline(always)]
    pub(crate) fn prop_name(&self) -> String {
        view_read_shared!(self, prop_name)
    }
    #[inline(always)]
    pub(crate) fn prop_name_view(&self) -> &View<String, M> {
        view_project!(self, prop_name)
    }
}

impl crate::native::view::View<TmpMaterialTexture> {
    #[inline(always)]
    pub(crate) fn set_material_id(&self, value: i32) {
        view_write!(self, material_id, value)
    }
    #[inline(always)]
    pub(crate) fn set_texture_id(&self, value: i32) {
        view_write!(self, texture_id, value)
    }
    #[inline(always)]
    pub(crate) fn set_prop_name(&self, value: String) {
        view_write!(self, prop_name, value)
    }
}

// `ufbxi_texture_extra` (ufbx.c:6352-6358) is likewise an internal scratch
// record with no generated view; the finalize pass reads the two layer-patch
// runs out of it.
impl crate::native::view::View<TextureExtra> {
    #[inline(always)]
    pub(crate) fn blend_modes(&self) -> *mut i32 {
        view_read!(self, blend_modes)
    }
    #[inline(always)]
    pub(crate) fn num_blend_modes(&self) -> usize {
        view_read!(self, num_blend_modes)
    }
    #[inline(always)]
    pub(crate) fn alphas(&self) -> *mut Real {
        view_read!(self, alphas)
    }
    #[inline(always)]
    pub(crate) fn num_alphas(&self) -> usize {
        view_read!(self, num_alphas)
    }
}

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
    math::fabs(as_f64!(offset.x)) >= epsilon
        || math::fabs(as_f64!(offset.y)) >= epsilon
        || math::fabs(as_f64!(offset.z)) >= epsilon
}

// ufbx.c:18099-18107 `ufbxi_pivot_div`
pub(crate) fn pivot_div(offset: Real, initial_scale: Real) -> Real {
    let epsilon: f64 = 0.0078125;
    if math::fabs(as_f64!(initial_scale)) >= epsilon {
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
pub(crate) fn pre_finalize_scene<'a>(uc: &'a Context) -> Result<(), Fail> {
    let mut required: bool = false;
    if uc.opts_view().geometry_transform_handling() == GeometryTransformHandling::HelperNodes
        || uc.opts_view().geometry_transform_handling() == GeometryTransformHandling::ModifyGeometry
    {
        required = true;
    }
    if uc.opts_view().inherit_mode_handling() == InheritModeHandling::HelperNodes
        || uc.opts_view().inherit_mode_handling() == InheritModeHandling::Compensate
        || uc.opts_view().inherit_mode_handling() == InheritModeHandling::CompensateNoFallback
    {
        required = true;
    }
    if uc.opts_view().pivot_handling() == PivotHandling::AdjustToPivot
        || uc.opts_view().pivot_handling() == PivotHandling::AdjustToRotationPivot
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
    let num_nodes: usize = uc.tmp_node_ids_view().num_items();
    // Every allocation in this prologue pops from, or pushes onto, uc's own
    // buffers, with the element/node/connection counts read from those same
    // buffers — so each returned run is exactly as long as the loops below
    // assume.
    let elements: *mut *mut Element = uc
        .tmp_parse_view()
        .push_pop::<*mut Element>(uc.tmp_element_ptrs_view(), num_elements as usize);
    ufbxi_check!(uc, !elements.is_null(), "elements");

    let num_connections: usize = uc.tmp_connections_view().num_items();
    let tmp_connections: *mut TmpConnection = uc
        .tmp_parse_view()
        .push_peek::<TmpConnection>(uc.tmp_connections_view(), num_connections);
    ufbxi_check!(uc, !tmp_connections.is_null(), "tmp_connections");

    let pre_connections: *mut PreConnection =
        uc.tmp_parse_view().push::<PreConnection>(num_connections);
    ufbxi_check!(uc, !pre_connections.is_null(), "pre_connections");

    let instance_counts: *mut u32 = uc.tmp_parse_view().push_zero::<u32>(num_elements as usize);
    ufbxi_check!(uc, !instance_counts.is_null(), "instance_counts");

    let modify_not_supported: *mut bool =
        uc.tmp_parse_view().push_zero::<bool>(num_elements as usize);
    ufbxi_check!(uc, !modify_not_supported.is_null(), "modify_not_supported");

    let node_attrib_type: *mut ElementType =
        uc.tmp_parse_view().push_zero::<ElementType>(num_nodes);
    ufbxi_check!(uc, !node_attrib_type.is_null(), "node_attrib_type");

    let has_unscaled_children: *mut bool = uc.tmp_parse_view().push_zero::<bool>(num_nodes);
    ufbxi_check!(
        uc,
        !has_unscaled_children.is_null(),
        "has_unscaled_children"
    );

    let has_scale_animation: *mut bool = uc.tmp_parse_view().push_zero::<bool>(num_nodes);
    ufbxi_check!(uc, !has_scale_animation.is_null(), "has_scale_animation");
    // C-parity: `has_scale_animation` is allocated and checked but never read
    // upstream; the allocation is observable so it stays.

    let pre_nodes: *mut PreNode = uc.tmp_parse_view().push_zero::<PreNode>(num_nodes);
    ufbxi_check!(uc, !pre_nodes.is_null(), "pre_nodes");

    let num_meshes: usize = uc
        .tmp_typed_element_offsets_at(ElementType::Mesh as usize)
        .num_items();
    let pre_meshes: *mut PreMesh = uc.tmp_parse_view().push_zero::<PreMesh>(num_meshes);
    ufbxi_check!(uc, !pre_meshes.is_null(), "pre_meshes");

    let num_anim_values: usize = uc
        .tmp_typed_element_offsets_at(ElementType::AnimValue as usize)
        .num_items();
    let pre_anim_values: *mut PreAnimValue = uc
        .tmp_parse_view()
        .push_zero::<PreAnimValue>(num_anim_values);
    ufbxi_check!(uc, !pre_anim_values.is_null(), "pre_anim_values");

    let fbx_ids: *mut u64 = uc
        .tmp_parse_view()
        .push_pop::<u64>(uc.tmp_element_fbx_ids_view(), num_elements as usize);
    ufbxi_check!(uc, !fbx_ids.is_null(), "fbx_ids");

    // TODO
    // C-parity: `0.001f`/`0.01f` are `float` literals widened to `ufbx_real`
    // (double) — NOT the decimal values (PORTING.md "Floats").
    let scale_epsilon: Real = 0.001f32 as Real;
    let pivot_epsilon: Real = 0.001f32 as Real;
    let compensate_epsilon: Real = 0.01f32 as Real;

    // SAFETY: `i < num_elements` indexes the `elements` run popped above; each
    // element is reached through an arena-anchored view and its `typed_id` indexes
    // the matching per-type side table (`pre_nodes` / `pre_anim_values`) pushed
    // above.
    unsafe {
        for i in 0..num_elements as usize {
            let element_view: &'a ElementView = ElementView::from_ptr(*elements.add(i));
            let id: u32 = element_view.typed_id();

            if element_view.type_() == ElementType::Node {
                let pre_node: *mut PreNode = pre_nodes.add(id as usize);
                (*pre_node).has_constant_scale = true;
                (*pre_node).constant_scale =
                    find_vec3(element_view.props_view(), &sp::Lcl_Scaling, 1.0, 1.0, 1.0);
                (*pre_node).element_id = element_view.element_id();
                (*pre_node).first_child = !0u32;
                (*pre_node).next_child = !0u32;
                (*pre_node).parent = !0u32;
            }
            // C-parity: ufbx.c:18186 is `} if (...)`, not `} else if (...)` — the
            // two element-type tests are independent statements.
            if element_view.type_() == ElementType::AnimValue {
                let pre_value: *mut PreAnimValue = pre_anim_values.add(id as usize);
                (*pre_value).has_constant_value = true;
                (*pre_value).constant_value.x =
                    find_real(element_view.props_view(), &sp::X, math::NAN as Real);
                (*pre_value).constant_value.x = find_real(
                    element_view.props_view(),
                    &sp::d_X,
                    (*pre_value).constant_value.x,
                );
                (*pre_value).constant_value.y =
                    find_real(element_view.props_view(), &sp::Y, math::NAN as Real);
                (*pre_value).constant_value.y = find_real(
                    element_view.props_view(),
                    &sp::d_Y,
                    (*pre_value).constant_value.y,
                );
                (*pre_value).constant_value.z =
                    find_real(element_view.props_view(), &sp::Z, math::NAN as Real);
                (*pre_value).constant_value.z = find_real(
                    element_view.props_view(),
                    &sp::d_Z,
                    (*pre_value).constant_value.z,
                );
            }
        }
    }

    // SAFETY: `i < num_connections` indexes the `tmp_connections`/
    // `pre_connections` runs; each resolved `FbxIdEntry` carries an`element_id`
    // into the `num_elements`-entry `elements` run, `src`/`dst` are null-checked
    // before use, and every side-table write is indexed by an element's own
    // `element_id`/`typed_id`.
    unsafe {
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

                        if uc.opts_view().inherit_mode_handling() != InheritModeHandling::Preserve {
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
                if (*dst).type_ == ElementType::AnimValue && (*src).type_ == ElementType::AnimCurve
                {
                    let src_curve: *mut AnimCurve = src as *mut AnimCurve;
                    let mut index: u32 = 0;
                    if dst_prop == sp::Y.as_ptr() || dst_prop == sp::d_Y.as_ptr() {
                        index = 1;
                    } else if dst_prop == sp::Z.as_ptr() || dst_prop == sp::d_Z.as_ptr() {
                        index = 2;
                    }

                    let pre_value: *mut PreAnimValue =
                        pre_anim_values.add((*dst).typed_id as usize);
                    if (*src_curve).max_value - (*src_curve).min_value >= scale_epsilon {
                        (*pre_value).has_constant_value = false;
                    } else {
                        let constant_value: Real =
                            ((*src_curve).min_value + (*src_curve).max_value) * 0.5;
                        // C: `pre_value->constant_value.v[index]` — the `ufbx_vec3`
                        // union's array view.
                        let v: *mut Real =
                            (&raw mut (*pre_value).constant_value as *mut Real).add(index as usize);
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
    }

    // SAFETY: `i < num_connections` indexes the `tmp_connections`/
    // `pre_connections` runs; `src`/`dst` are the null-checked elements recorded
    // by the pass above, and every side-table index is one of their own
    // `element_id`/`typed_id` values, in range of the runs pushed above.
    unsafe {
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
                                    error += math::fabs(as_f64!(
                                        (*pre_value).constant_value.x
                                            - (*pre_node).constant_scale.x
                                    )) as Real;
                                    error += math::fabs(as_f64!(
                                        (*pre_value).constant_value.y
                                            - (*pre_node).constant_scale.y
                                    )) as Real;
                                    error += math::fabs(as_f64!(
                                        (*pre_value).constant_value.z
                                            - (*pre_node).constant_scale.z
                                    )) as Real;
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
    }

    // SAFETY: `i < num_nodes` indexes the `pre_nodes` run pushed above; each
    // `element_id` is an index into the `num_elements`-entry `elements` run, and
    // the node it selects is reached through an arena-anchored view; the child
    // walk follows the `first_child`/`next_child` links built above, which are
    // either `~0u32` or
    // indices `< num_nodes`.
    unsafe {
        if uc.opts_view().pivot_handling() == PivotHandling::AdjustToPivot
            || uc.opts_view().pivot_handling() == PivotHandling::AdjustToRotationPivot
        {
            for i in 0..num_nodes {
                let pre_node: *mut PreNode = pre_nodes.add(i);
                let node_view: &'a NodeView =
                    NodeView::from_ptr(*elements.add((*pre_node).element_id as usize) as *mut Node);
                let node: *mut Node = node_view.get();

                let rotation_pivot: Vec3 =
                    find_vec3(node_view.props_view(), &sp::RotationPivot, 0.0, 0.0, 0.0);
                let scaling_pivot: Vec3 =
                    find_vec3(node_view.props_view(), &sp::ScalingPivot, 0.0, 0.0, 0.0);
                let scaling_offset: Vec3 =
                    find_vec3(node_view.props_view(), &sp::ScalingOffset, 0.0, 0.0, 0.0);

                let mut should_modify_pivot: bool = false;
                if uc.opts_view().pivot_handling() == PivotHandling::AdjustToPivot {
                    should_modify_pivot = !is_vec3_zero(rotation_pivot);
                } else if uc.opts_view().pivot_handling() == PivotHandling::AdjustToRotationPivot {
                    should_modify_pivot = pivot_nonzero(rotation_pivot)
                        || pivot_nonzero(scaling_pivot)
                        || pivot_nonzero(scaling_offset);
                }

                if should_modify_pivot {
                    let mut skip_geometry_transform: bool = false;
                    let mut can_modify_geometry_transform: bool = true;
                    if uc.opts_view().pivot_handling() == PivotHandling::AdjustToRotationPivot {
                        if *node_attrib_type.add((*node).element.typed_id as usize)
                            == ElementType::Empty
                        {
                            if !uc.opts_view().pivot_handling_retain_empties() {
                                skip_geometry_transform = true;
                            } else {
                                can_modify_geometry_transform = false;
                            }
                        }
                    }

                    if uc.opts_view().geometry_transform_handling()
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
                    if uc.opts_view().pivot_handling() == PivotHandling::AdjustToPivot {
                        // C: `err += (ufbx_real)ufbx_fabs(a - b)` — real subtraction,
                        // double `fabs`, narrowed back to real before accumulating.
                        let mut err: Real = 0.0;
                        err += math::fabs(as_f64!(rotation_pivot.x - scaling_pivot.x)) as Real;
                        err += math::fabs(as_f64!(rotation_pivot.y - scaling_pivot.y)) as Real;
                        err += math::fabs(as_f64!(rotation_pivot.z - scaling_pivot.z)) as Real;
                        if err > pivot_epsilon {
                            can_modify_pivot = false;
                        }
                    }

                    if can_modify_pivot
                        && (can_modify_geometry_transform || skip_geometry_transform)
                    {
                        let mut geometric_translation: Vec3 = find_vec3(
                            node_view.props_view(),
                            &sp::GeometricTranslation,
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
                        if uc.opts_view().pivot_handling() == PivotHandling::AdjustToPivot {
                            ufbx_assert!(!skip_geometry_transform); // not supporeted in legacy mode
                            child_offset = neg3(rotation_pivot);
                            geometric_translation = add3(geometric_translation, child_offset);

                            new_props = uc.result_view().push_zero::<Prop>(num_props + 3);
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
                        } else if uc.opts_view().pivot_handling()
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
                            let initial_scale: Vec3 =
                                find_vec3(node_view.props_view(), &sp::Lcl_Scaling, 1.0, 1.0, 1.0);
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

                            new_props = uc.result_view().push_zero::<Prop>(num_props + 4);
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
                        deduplicate_properties(&raw mut (*node).element.props.props);

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
    }

    // SAFETY: `i < num_elements` indexes the `elements`/`fbx_ids` runs popped
    // above (both `num_elements` long), and the per-element side tables
    // (`instance_counts`, `modify_not_supported`) are the equally sized zeroed
    // pushes above.
    unsafe {
        for i in 0..num_elements as usize {
            let element_view: &'a ElementView = ElementView::from_ptr(*elements.add(i));
            let fbx_id: u64 = *fbx_ids.add(i);

            if element_view.type_() == ElementType::Node {
                // The widening cast to `*mut Node` uses the arena pointer, whose
                // provenance spans the whole node; `element_view` only covers the
                // `Element` header and so serves solely as the `type_()` reader.
                let node: *mut Node = *elements.add(i) as *mut Node;
                let mut requires_helper_node: bool = false;
                if uc.opts_view().geometry_transform_handling()
                    == GeometryTransformHandling::HelperNodes
                {
                    requires_helper_node = true;
                } else if uc.opts_view().geometry_transform_handling()
                    == GeometryTransformHandling::ModifyGeometry
                {
                    // Setup a geometry transform helper for nodes that have instanced attributes
                    requires_helper_node =
                        *instance_counts.add(i) > 1 || *modify_not_supported.add(i);
                }
                if requires_helper_node {
                    setup_geometry_transform_helper(uc, node, fbx_id)?;
                }
            }
        }
    }

    // SAFETY: same `elements`/`fbx_ids` indexing as the pass above; the per-node
    // side tables (`pre_nodes`, `has_unscaled_children`) are indexed by that node's
    // own `typed_id`, which is `< num_nodes` by construction.
    unsafe {
        for i in 0..num_elements as usize {
            let element_view: &'a ElementView = ElementView::from_ptr(*elements.add(i));
            let fbx_id: u64 = *fbx_ids.add(i);

            if element_view.type_() == ElementType::Node {
                // The widening cast to `*mut Node` uses the arena pointer, whose
                // provenance spans the whole node; `element_view` only covers the
                // `Element` header and so serves solely as the `type_()` reader.
                let node: *mut Node = *elements.add(i) as *mut Node;
                if *has_unscaled_children.add((*node).element.typed_id as usize)
                    && (*node).scale_helper.is_none()
                {
                    let pre_node: *mut PreNode = pre_nodes.add((*node).element.typed_id as usize);
                    let r#ref: Real = if uc.opts_view().inherit_mode_handling()
                        == InheritModeHandling::Compensate
                    {
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
                        || math::fabs(as_f64!(scale.x)) as Real <= compensate_epsilon)
                        && uc.opts_view().inherit_mode_handling()
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
                    } else if uc.opts_view().inherit_mode_handling()
                        == InheritModeHandling::Compensate
                        || uc.opts_view().inherit_mode_handling()
                            == InheritModeHandling::CompensateNoFallback
                    {
                        // C: `(ufbx_real)ufbx_fabs(scale.x - 1.0f)` — real
                        // subtraction, double `fabs`, narrowed back to real.
                        if math::fabs(as_f64!(scale.x - 1.0)) as Real >= scale_epsilon {
                            (*node).is_scale_compensate_parent = true;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// `// -- Scene pre-processing` section complete (ufbx.c:18066-18543).

// -- Scene processing (ufbx.c:18545-...)

// ufbx.c:18547-18554 `ufbxi_find_element_by_fbx_id`
#[inline(never)]
pub(crate) fn find_element_by_fbx_id(uc: &Context, fbx_id: u64) -> *mut Element {
    let entry: *mut FbxIdEntry = find_fbx_id(uc, fbx_id);
    if !entry.is_null() {
        // SAFETY: `entry` is non-null (checked) and a valid FbxIdEntry from the
        // id map; its `element_id` indexes the scene's element array by
        // construction.
        return unsafe {
            *(uc.scene_view().elements_view().data() as *mut *mut Element)
                .add((*entry).element_id as usize)
        };
    }
    ptr::null_mut()
}

// ufbx.c:18556-18562 `ufbxi_cmp_name_element_less`
// Comparator over views: the sort adapter mints them (PORTING.md "Sorting").
#[inline(always)]
pub(crate) fn cmp_name_element_less<M: crate::native::view::Mode>(
    a: &View<NameElement, M>,
    b: &View<NameElement, M>,
) -> bool {
    if a._internal_key() != b._internal_key() {
        return a._internal_key() < b._internal_key();
    }
    // C: `strcmp(a->name.data, b->name.data)` over interned (NUL-terminated)
    // names — `c_strcmp` stops at the first NUL like `strcmp`.
    let cmp: i32 = c_strcmp(a.name_view().bytes(), b.name_view().bytes());
    if cmp != 0 {
        return cmp < 0;
    }
    (a.type_() as u32) < (b.type_() as u32)
}

// ufbx.c:18564-18570 `ufbxi_cmp_name_element_less_ref`
// `name: &[u8]` carries C's `ufbx_string` query key (see `find_prop_len`).
#[inline(always)]
pub(crate) unsafe fn cmp_name_element_less_ref(
    a: *const NameElement,
    name: &[u8],
    type_: ElementType,
    key: u32,
) -> bool {
    // SAFETY: `a` points to a live, initialized `NameElement` — the array
    // element the bounded search is probing (fn contract); its `name` is an
    // interned span readable for its own length (the `as_bytes` contract).
    unsafe {
        if (*a)._internal_key != key {
            return (*a)._internal_key < key;
        }
        let cmp: i32 = str_cmp((*a).name.as_bytes(), name);
        if cmp != 0 {
            return cmp < 0;
        }
        ((*a).type_ as u32) < type_ as u32
    }
}

// ufbx.c:18572-18576 `ufbxi_cmp_prop_less_ref`
// `name: &[u8]` carries C's `ufbx_string` query key (see `find_prop_len`).
#[inline(always)]
pub(crate) fn cmp_prop_less_ref<M: crate::native::view::Mode>(
    a: &crate::native::view::View<Prop, M>,
    name: &[u8],
    key: u32,
) -> bool {
    if a._internal_key() != key {
        return a._internal_key() < key;
    }
    str_less(a.name_view().bytes(), name)
}

// ufbx.c:18578-18582 `ufbxi_cmp_prop_less_concat`
#[inline(always)]
pub(crate) unsafe fn cmp_prop_less_concat<M: crate::native::view::Mode>(
    a: &crate::native::view::View<Prop, M>,
    parts: &[String],
    key: u32,
) -> bool {
    if a._internal_key() != key {
        return a._internal_key() < key;
    }
    // SAFETY: each part's `data` is readable for its `length` bytes — the
    // key-part contract forwarded from this fn's own.
    unsafe { concat_str_cmp(a.name(), parts) < 0 }
}

// ufbx.c:18584-18590 `ufbxi_sort_name_elements`
#[inline(never)]
pub(crate) unsafe fn sort_name_elements(
    uc: &Context,
    name_elems: *mut NameElement,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the three pointers are `uc`'s own live `ator_tmp` and the
        // `tmp_arr`/`tmp_arr_size` slots that pair with it.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count.wrapping_mul(size_of::<NameElement>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_name_element)))"
    );
    // SAFETY: `name_elems` addresses `count` initialized `NameElement`s (fn
    // contract) and `tmp_arr` was just grown to `count * size_of::<NameElement>()`
    // bytes, so the two disjoint runs `macro_stable_sort` needs are in place; it
    // hands the comparator pointers to live elements of those runs.
    unsafe {
        macro_stable_sort_views::<NameElement>(
            32,
            name_elems,
            uc.tmp_arr() as *mut NameElement,
            count,
            cmp_name_element_less,
        )
    };
    Ok(())
}

// ufbx.c:18592-18610 `ufbxi_cmp_node_less`
// Comparator over views: the sort adapter mints them (PORTING.md "Sorting").
// `parent` links resolve through `Ref`'s safe deref: a non-null parent is a
// live scene node for the whole sort.
#[inline(never)]
pub(crate) fn cmp_node_less<M: crate::native::view::Mode>(
    a: &View<Node, M>,
    b: &View<Node, M>,
) -> bool {
    if a.node_depth() != b.node_depth() {
        return a.node_depth() < b.node_depth();
    }
    match (a.parent(), b.parent()) {
        (Some(a_parent), Some(b_parent)) => {
            let a_pid: u32 = a_parent.element.element_id;
            let b_pid: u32 = b_parent.element.element_id;
            if a_pid != b_pid {
                return a_pid < b_pid;
            }
        }
        (a_parent, b_parent) => {
            ufbx_assert!(a_parent.is_none() && b_parent.is_none());
        }
    }
    if a.is_geometry_transform_helper() != b.is_geometry_transform_helper() {
        // Sort geometry transform helpers always before rest of the children.
        return a.is_geometry_transform_helper() as u32 > b.is_geometry_transform_helper() as u32;
    }
    if a.is_scale_helper() != b.is_scale_helper() {
        // Sort scale helpers after geometry transform helpers.
        return a.is_scale_helper() as u32 > b.is_scale_helper() as u32;
    }
    a.element().element_id() < b.element().element_id()
}

// ufbx.c:18612-18618 `ufbxi_sort_node_ptrs`
#[inline(never)]
pub(crate) unsafe fn sort_node_ptrs(
    uc: &Context,
    nodes: *mut *mut Node,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the three pointers are `uc`'s own live `ator_tmp` and the
        // `tmp_arr`/`tmp_arr_size` slots that pair with it.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count.wrapping_mul(size_of::<*mut Node>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_node*)))"
    );
    // SAFETY: `nodes` addresses `count` initialized node pointers (fn contract)
    // and `tmp_arr` was just grown to `count * size_of::<*mut Node>()` bytes, so
    // the two disjoint runs `macro_stable_sort` needs are in place; the
    // comparator receives pointers into those runs, each holding a live `Node`
    // pointer.
    unsafe {
        macro_stable_sort_ptr_views::<Node>(
            32,
            nodes,
            uc.tmp_arr() as *mut *mut Node,
            count,
            cmp_node_less,
        )
    };
    Ok(())
}

// ufbx.c:18620-18625 `ufbxi_cmp_tmp_material_texture_less`
// C declares this `int`-returning, but every `return` is a boolean expression
// and the only caller is a sort comparator.
// Comparator over views: the sort adapter mints them (PORTING.md "Sorting").
// Both `prop_name`s are interned string-pool spans, which is what `str_less`
// compares.
#[inline(never)]
#[must_use]
pub(crate) fn cmp_tmp_material_texture_less<M: Mode>(
    a: &View<TmpMaterialTexture, M>,
    b: &View<TmpMaterialTexture, M>,
) -> bool {
    if a.material_id() != b.material_id() {
        return a.material_id() < b.material_id();
    }
    if a.texture_id() != b.texture_id() {
        return a.texture_id() < b.texture_id();
    }
    str_less(a.prop_name_view().bytes(), b.prop_name_view().bytes())
}

// ufbx.c:18627-18633 `ufbxi_sort_tmp_material_textures`
#[inline(never)]
pub(crate) unsafe fn sort_tmp_material_textures(
    uc: &Context,
    mat_texs: *mut TmpMaterialTexture,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the three pointers are `uc`'s own live `ator_tmp` and the
        // `tmp_arr`/`tmp_arr_size` slots that pair with it.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count.wrapping_mul(size_of::<TmpMaterialTexture>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbxi_tmp_material_texture)))"
    );
    // SAFETY: `mat_texs` addresses `count` initialized `TmpMaterialTexture`s (fn
    // contract) and `tmp_arr` was just grown to
    // `count * size_of::<TmpMaterialTexture>()` bytes, so the two disjoint runs
    // `macro_stable_sort` needs are in place; it hands the comparator pointers to
    // live elements of those runs.
    unsafe {
        macro_stable_sort_views::<TmpMaterialTexture>(
            32,
            mat_texs,
            uc.tmp_arr() as *mut TmpMaterialTexture,
            count,
            cmp_tmp_material_texture_less,
        )
    };
    Ok(())
}

// We need to be able to assume no padding!
// ufbx.c:18636 `ufbx_static_assert(connection_size, sizeof(ufbx_connection) == sizeof(ufbx_element*)*2 + sizeof(ufbx_string)*2);`
const _: () =
    assert!(size_of::<Connection>() == size_of::<*mut Element>() * 2 + size_of::<String>() * 2);

// ufbx.c:18638-18646 `ufbxi_cmp_connection_less`
// Comparator over views: the sort adapter mints them (PORTING.md "Sorting").
//
// # Safety
// `index` must be 0 or 1: it selects between the two adjacent element refs
// (and the two adjacent strings) of one unpadded `ufbx_connection`, a bound
// the parameter type cannot carry.
#[inline(always)]
pub(crate) unsafe fn cmp_connection_less<M: Mode>(
    a: &View<Connection, M>,
    b: &View<Connection, M>,
    index: usize,
) -> bool {
    // C-parity: `(&a->src)[index]` / `(&a->src_prop)[index]` index across the
    // two adjacent element pointers and the two adjacent strings of
    // `ufbx_connection`, which the static assert above pins as unpadded; both
    // projections are derived from the whole connection, keeping its provenance
    // (PORTING.md "Raw pointers from places").
    let a_src: *const Ref<Element> = a.src_ptr();
    let b_src: *const Ref<Element> = b.src_ptr();
    // SAFETY: `index` is 0 or 1 (fn contract), so the offset stays inside the
    // viewed connection, and the ref it names holds a live scene element.
    let a_elem: *mut Element = unsafe { ref_ptr(a_src.add(index)) };
    // SAFETY: as above, for `b`.
    let b_elem: *mut Element = unsafe { ref_ptr(b_src.add(index)) };
    if a_elem != b_elem {
        return a_elem < b_elem;
    }
    let a_prop: *const String = a.src_prop_ptr();
    let b_prop: *const String = b.src_prop_ptr();
    // SAFETY: `index` is 0 or 1 (fn contract), so both offsets stay inside the
    // two adjacent `src_prop`/`dst_prop` strings of their connection; each is
    // an interned string-pool span, hence NUL-terminated for `strcmp`.
    let mut cmp: i32 = unsafe { strcmp((*a_prop.add(index)).data, (*b_prop.add(index)).data) };
    if cmp != 0 {
        return cmp < 0;
    }
    // SAFETY: as above, with `index ^ 1` naming the other of the two strings.
    cmp = unsafe { strcmp((*a_prop.add(index ^ 1)).data, (*b_prop.add(index ^ 1)).data) };
    cmp < 0
}

// ufbx.c:18648-18653 `ufbxi_sort_connections`
#[inline(never)]
pub(crate) unsafe fn sort_connections(
    uc: &Context,
    connections: *mut Connection,
    count: usize,
    index: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the three pointers are `uc`'s own live `ator_tmp` and the
        // `tmp_arr`/`tmp_arr_size` slots that pair with it.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count.wrapping_mul(size_of::<Connection>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_connection)))"
    );
    // SAFETY: `connections` addresses `count` initialized `Connection`s (fn
    // contract) and `tmp_arr` was just grown to `count * size_of::<Connection>()`
    // bytes, so the two disjoint runs `macro_stable_sort` needs are in place; it
    // hands the comparator pointers to live elements of those runs, and `index`
    // is the caller's 0-or-1 field selector.
    unsafe {
        macro_stable_sort_views::<Connection>(
            32,
            connections,
            uc.tmp_arr() as *mut Connection,
            count,
            // The comparator call is covered by the enclosing block: `index` is
            // the caller's 0-or-1 field selector, which is
            // `cmp_connection_less`'s contract.
            |a, b| cmp_connection_less(a, b, index),
        )
    };
    Ok(())
}

// ufbx.c:18655-18663 `ufbxi_find_attribute_fbx_id`
pub(crate) fn find_attribute_fbx_id(uc: &Context, node_fbx_id: u64) -> u64 {
    let hash: u32 = hash64(node_fbx_id);
    let entry: *mut FbxAttrEntry = uc.fbx_attr_map_view().find(hash, &node_fbx_id);
    if !entry.is_null() {
        // SAFETY: `entry` is non-null (checked) and a valid FbxAttrEntry.
        return unsafe { (*entry).attr_fbx_id };
    }
    node_fbx_id
}

// ufbx.c:18665-18780 `ufbxi_resolve_connections`
#[inline(never)]
pub(crate) fn resolve_connections(uc: &Context) -> Result<(), Fail> {
    let num_connections: usize = uc.tmp_connections_view().num_items();
    // Pops the `num_connections` recorded connections from uc's
    // `tmp_connections` buffer into uc's tmp buffer.
    let tmp_connections: *mut TmpConnection = uc
        .tmp_view()
        .push_pop(uc.tmp_connections_view(), num_connections);
    // SAFETY: frees the drained buffer through uc's raw-ptr getter.
    unsafe { buf_free(uc.tmp_connections_mut_ptr()) };
    ufbxi_check!(uc, !tmp_connections.is_null(), "tmp_connections");

    // NOTE: We truncate this array in case not all connections are resolved
    uc.scene_view()
        .connections_src_view()
        .set_data(uc.result_view().push::<Connection>(num_connections));
    ufbxi_check!(
        uc,
        !uc.scene_view().connections_src_view().data().is_null(),
        "uc->scene.connections_src.data"
    );

    // HACK: Translate property connections from node to attribute if the property name is not included
    // in the known node properties and is not a property of the node.
    if uc.version() > 0 && uc.version() < 7000 {
        // SAFETY: walks the fresh `num_connections`-element `tmp_connections` run;
        // each `src`/`dst` is null-checked before its props are read through an
        // arena-anchored element view, with the prop name and length taken from the
        // connection's own interned string.
        unsafe {
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
                            // `src` is non-null here (the `||` short-circuits the
                            // null case), and it resolves to a uc-arena element, so
                            // its view anchors the lookup to `uc`.
                            ElementView::from_ptr(src).props_view(),
                            (*tmp_conn).src_prop.as_bytes(),
                        )
                        .is_none()
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
                            // `dst` is non-null here (short-circuit) and resolves to
                            // a uc-arena element, so its view anchors to `uc`.
                            ElementView::from_ptr(dst).props_view(),
                            (*tmp_conn).dst_prop.as_bytes(),
                        )
                        .is_none()
                    {
                        (*tmp_conn).dst = find_attribute_fbx_id(uc, (*tmp_conn).dst);
                    }
                }
                tmp_conn = tmp_conn.add(1);
            }
        }
    }

    // SAFETY: indexes the fresh `num_connections`-element `tmp_connections` run;
    // `src`/`dst` are null-checked arena elements (the element-type tests gate
    // every downcast), `get_element_extra`'s helper ids index the scene's own
    // element-pointer run, and `conn` is the next unused slot of the
    // `num_connections`-entry `connections_src` run pushed above.
    unsafe {
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

            if !uc.opts_view().disable_quirks() {
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
                        dst = *(uc.scene_view().elements_view().data() as *mut *mut Element)
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
                    let scale_helper: *mut Node = opt_ptr(&raw const (*dst_node).scale_helper);
                    if !scale_helper.is_null() {
                        if (*src).type_ == ElementType::Node {
                            let src_node: *mut Node = src as *mut Node;
                            if !(*src_node).is_scale_helper
                                && (*src_node).original_inherit_mode == InheritMode::Normal
                            {
                                dst = &raw mut (*scale_helper).element;
                            }
                        } else if (*src).type_ == ElementType::AnimValue {
                            if (*tmp_conn).dst_prop.data == sp::Lcl_Scaling.as_ptr() {
                                dst = &raw mut (*scale_helper).element;
                            }
                        } else {
                            dst = &raw mut (*scale_helper).element;
                        }
                    }
                } else if (*src).type_ == ElementType::Node {
                    let src_node: *mut Node = src as *mut Node;
                    let scale_helper: *mut Node = opt_ptr(&raw const (*src_node).scale_helper);
                    if !scale_helper.is_null() {
                        if (*dst).type_ == ElementType::SkinCluster {
                            src = &raw mut (*scale_helper).element;
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

            let conn: *mut Connection = (uc.scene_view().connections_src_view().data()
                as *mut Connection)
                .add(uc.scene_view().connections_src_view().count());
            uc.scene_view().connections_src_view().set_count(
                uc.scene_view()
                    .connections_src_view()
                    .count()
                    .wrapping_add(1),
            );
            (*conn).src = Ref::from_ptr(src);
            (*conn).dst = Ref::from_ptr(dst);
            (*conn).src_prop = (*tmp_conn).src_prop;
            (*conn).dst_prop = (*tmp_conn).dst_prop;
        }
    }

    uc.scene_view()
        .connections_dst_view()
        .set_count(uc.scene_view().connections_src_view().count());
    // SAFETY: copies the `connections_src` run just materialized in uc's own
    // result buffer into a second run in that same buffer, then sorts both runs
    // in place with their own counts.
    unsafe {
        uc.scene_view().connections_dst_view().set_data(
            uc.result_view().push_copy_raw::<Connection>(
                uc.scene_view().connections_src_view().count(),
                uc.scene_view().connections_src_view().data(),
            ),
        );
        ufbxi_check!(
            uc,
            !uc.scene_view().connections_dst_view().data().is_null(),
            "uc->scene.connections_dst.data"
        );

        sort_connections(
            uc,
            uc.scene_view().connections_src_view().data() as *mut Connection,
            uc.scene_view().connections_src_view().count(),
            0,
        )?;
        sort_connections(
            uc,
            uc.scene_view().connections_dst_view().data() as *mut Connection,
            uc.scene_view().connections_dst_view().count(),
            1,
        )?;
    }

    // SAFETY: frees uc's own `tmp_connections` buffer through its raw-ptr getter.
    unsafe {
        // We don't need the temporary connections at this point anymore
        buf_free(uc.tmp_connections_mut_ptr());
    }

    Ok(())
}

// ufbx.c:18782-18912 `ufbxi_add_connections_to_elements`
#[inline(never)]
pub(crate) fn add_connections_to_elements(uc: &Context) -> Result<(), Fail> {
    let mut conn_src: *mut Connection =
        uc.scene_view().connections_src_view().data() as *mut Connection;
    let conn_src_end: *mut Connection =
        add_ptr(conn_src, uc.scene_view().connections_src_view().count());
    let mut conn_dst: *mut Connection =
        uc.scene_view().connections_dst_view().data() as *mut Connection;
    let conn_dst_end: *mut Connection =
        add_ptr(conn_dst, uc.scene_view().connections_dst_view().count());

    // SAFETY: one walk over the scene's stored element-pointer run (`count`
    // entries) advancing in lockstep through the two `connections_src`/
    // `connections_dst` runs, which are sorted by element id and bounded by the
    // `*_end` cursors computed above — so every connection deref is inside its
    // run and every `src_end`/`dst_end` slice is inside the element's own range.
    // Within an element: `prop` walks that element's own property run against
    // `prop_end`; `anim_def_prop` is a local fully zeroed before use;
    // `find_prop_with_key` reads the element's own (`is_some`-checked) defaults
    // table through an arena-anchored view; and each `push_copy` copies the
    // counted properties out of the element's own property run onto uc's tmp
    // stack.
    unsafe {
        // C: `ufbxi_for_ptr(ufbx_element, p_elem, uc->scene.elements.data, uc->scene.elements.count)`
        let mut p_elem: *mut *mut Element =
            uc.scene_view().elements_view().data() as *mut *mut Element;
        let p_elem_end: *mut *mut Element = p_elem.add(uc.scene_view().elements_view().count());
        while p_elem != p_elem_end {
            let elem: *mut Element = *p_elem;
            let id: u32 = (*elem).element_id;

            while conn_src < conn_src_end && (*ref_ptr(&raw const (*conn_src).src)).element_id < id
            {
                conn_src = conn_src.add(1);
            }
            while conn_dst < conn_dst_end && (*ref_ptr(&raw const (*conn_dst).dst)).element_id < id
            {
                conn_dst = conn_dst.add(1);
            }
            let mut src_end: *mut Connection = conn_src;
            let mut dst_end: *mut Connection = conn_dst;

            while src_end < conn_src_end && (*ref_ptr(&raw const (*src_end).src)).element_id == id {
                src_end = src_end.add(1);
            }
            while dst_end < conn_dst_end && (*ref_ptr(&raw const (*dst_end).dst)).element_id == id {
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
                        if (*ref_ptr(&raw const (*conn_dst).src)).type_ == ElementType::AnimValue {
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
                        } else if (*ref_ptr(&raw const (*conn_dst).src)).type_
                            == ElementType::AnimValue
                        {
                            anim_value = ref_ptr(&raw const (*conn_dst).src) as *mut AnimValue;
                            flags |= PropFlags::ANIMATED.raw();
                        }
                        conn_dst = conn_dst.add(1);
                    }

                    let key: u32 = get_name_key(name.as_bytes());
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
                            !uc.tmp_stack_view().push_copy_raw::<Prop>(to_size(prop.offset_from(copy_start)),
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
                            def_prop = match find_prop_with_key(
                                PropsView::from_ptr(opt_ptr(&raw const (*elem).props.defaults)),
                                // `find_prop_with_key` matches on the interned
                                // run's ADDRESS, so the borrow must be over
                                // `name`'s own bytes.
                                core::slice::from_raw_parts(name.data, name.length),
                                key,
                            ) {
                                Some(prop) => prop.get(),
                                None => ptr::null_mut(),
                            };
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
                                    &raw mut (*anim_def).value_vec4 as *mut Vec3;
                                (*value_vec3).x = 1.0;
                                (*value_vec3).y = 1.0;
                                (*value_vec3).z = 1.0;
                            }
                            // Property values are only defined in anim_props on legacy files
                            if uc.version() < 6000 {
                                *(&raw mut (*anim_def).value_vec4 as *mut Vec3) =
                                    (*anim_value).default_value;
                            }
                            (*anim_def).type_ = type_;
                            def_prop = anim_def;
                        } else {
                            flags |= PropFlags::NO_VALUE.raw();
                        }

                        let new_prop: *mut Prop = uc.tmp_stack_view().push_zero(1);
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
                    let num_new_props: usize =
                        (*elem).props.props.count.wrapping_add(num_synthetic);
                    ufbxi_check!(
                        uc,
                        !uc.tmp_stack_view().push_copy_raw::<Prop>(to_size(prop_end.offset_from(copy_start)),
                            copy_start,
                        )
                        .is_null(),
                        "((ufbx_prop*)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_prop), (((size_t)(prop_end - copy_start))), (copy_start)))"
                    );
                    (*elem).props.props.data = uc
                        .result_view()
                        .push_pop::<Prop>(uc.tmp_stack_view(), num_new_props);
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
    }

    Ok(())
}

// ufbx.c:18914-18994 `ufbxi_linearize_nodes`
#[inline(never)]
pub(crate) fn linearize_nodes(uc: &Context) -> Result<(), Fail> {
    let num_nodes: usize = uc.tmp_node_ids_view().num_items();
    // Pops the `num_nodes` recorded ids from uc's `tmp_node_ids` buffer into
    // uc's tmp buffer.
    let node_ids: *mut u32 = uc.tmp_view().push_pop(uc.tmp_node_ids_view(), num_nodes);
    // SAFETY: frees the drained source buffer through uc's raw-ptr getter.
    unsafe { buf_free(uc.tmp_node_ids_mut_ptr()) };
    ufbxi_check!(uc, !node_ids.is_null(), "node_ids");

    let node_ptrs: *mut *mut Node = uc.tmp_stack_view().push(num_nodes);
    ufbxi_check!(uc, !node_ptrs.is_null(), "node_ptrs");

    // SAFETY: `node_ptrs` is the fresh `num_nodes`-element push above and each
    // `node_ids[i]` is an index into the scene's element-pointer run (the ids were
    // recorded as elements were created).
    unsafe {
        // Fetch the node pointers
        for i in 0..num_nodes {
            *node_ptrs.add(i) = *(uc.scene_view().elements_view().data() as *mut *mut Element)
                .add(*node_ids.add(i) as usize) as *mut Node;
            ufbx_assert!((**node_ptrs.add(i)).element.type_ == ElementType::Node);
        }

        // C reads `node_ptrs[0]` unconditionally; there is always at least the root
        // node in `tmp_node_ids` by the time this runs.
        uc.scene_view()
            .set_root_node(Ref::from_ptr(*node_ptrs.add(0)));
    }

    // Pops the `num_nodes` node offsets recorded in uc's typed element-offset
    // buffer onto uc's tmp stack.
    let node_offsets: *mut usize = uc.tmp_stack_view().push_pop(
        uc.tmp_typed_element_offsets_at(ElementType::Node as usize),
        num_nodes,
    );
    ufbxi_check!(uc, !node_offsets.is_null(), "node_offsets");

    // SAFETY: both passes below walk the fresh `num_nodes`-element `node_ptrs`
    // run (the second reuses the first's `p_node_end`) and, per node, that
    // node's own `connections_dst` run (`count` entries); `opt_ptr`/`ref_ptr`
    // results are null-checked or always-resolved references of the same arena,
    // and the parent-chain walk is bounded by the `num_nodes` cycle guard.
    unsafe {
        // Hook up the parent nodes, we'll assume that there's no cycles at this point
        // C: `ufbxi_for_ptr(ufbx_node, p_node, node_ptrs, num_nodes)`
        let mut p_node: *mut *mut Node = node_ptrs;
        let p_node_end: *mut *mut Node = node_ptrs.add(num_nodes);
        while p_node != p_node_end {
            let node: *mut Node = *p_node;

            // Pre-6000 files don't have any explicit root connections so they must always
            // be connected to the root..
            if opt_ptr(&raw const (*node).parent).is_null()
                && !(uc.opts_view().allow_nodes_out_of_root() && uc.version() >= 6000)
            {
                if node != ref_ptr(uc.scene_view().root_node_ptr()) {
                    (*node).parent = Some(uc.scene_view().root_node());
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
                if (*ref_ptr(&raw const (*conn).src)).type_ != ElementType::Node {
                    conn = conn.add(1);
                    continue;
                }
                (*(ref_ptr(&raw const (*conn).src) as *mut Node)).parent = opt_ref(node);
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
            let mut p: *mut Node = opt_ptr(&raw const (*node).parent);
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
                p = opt_ptr(&raw const (*p).parent);
            }

            if uc.opts_view().node_depth_limit() > 0 {
                ufbxi_check_msg!(
                    uc,
                    depth <= uc.opts_view().node_depth_limit(),
                    "Node depth limit exceeded",
                    "depth <= uc->opts.node_depth_limit"
                );
            }
            (*node).node_depth = depth;

            // Second pass to cache the depths to avoid O(n^2)
            let mut p: *mut Node = opt_ptr(&raw const (*node).parent);
            while !p.is_null() {
                depth = depth.wrapping_sub(1);
                if depth <= (*p).node_depth {
                    break;
                }
                (*p).node_depth = depth;
                p = opt_ptr(&raw const (*p).parent);
            }
            p_node = p_node.add(1);
        }
    }

    // SAFETY: sorts the fresh `num_nodes`-element `node_ptrs` run, then re-indexes
    // it; each `p_offset` is the fresh non-null result of a push onto uc's own
    // typed-offset buffer, and `original_id` is that node's index into the
    // equally sized `node_offsets` run.
    unsafe {
        sort_node_ptrs(uc, node_ptrs, num_nodes)?;

        for i in 0..num_nodes as u32 {
            let p_offset: *mut usize = uc
                .tmp_typed_element_offsets_at(ElementType::Node as usize)
                .push(1);
            ufbxi_check!(uc, !p_offset.is_null(), "p_offset");
            let node: *mut Node = *node_ptrs.add(i as usize);

            let original_id: u32 = (*node).element.typed_id;
            (*node).element.typed_id = i;
            *p_offset = *node_offsets.add(original_id as usize);
        }
    }

    // SAFETY: pops the two temporary runs pushed above off uc's own tmp stack,
    // discarding the values.
    unsafe {
        // Pop the temporary arrays
        pop::<usize>(uc.tmp_stack_mut_ptr(), num_nodes, ptr::null_mut());
        pop::<*mut Node>(uc.tmp_stack_mut_ptr(), num_nodes, ptr::null_mut());
    }

    Ok(())
}

// ufbx.c:18997-19014 `ufbxi_find_dst_connections`
// `prop: Option<&[u8]>` carries C's nullable `const char *prop`: `None` is C's
// `NULL`, `Some` the NUL-terminated interned name minted once by the caller.
#[inline(never)]
#[must_use]
pub(crate) fn find_dst_connections(element: &ElementView, prop: Option<&[u8]>) -> List<Connection> {
    // C: `if (!prop) prop = ufbxi_empty_char;` — the empty span's base address
    // is `ufbxi_empty_char` itself, which the identity probes below compare.
    let prop: &[u8] = prop.unwrap_or(&EMPTY_CHAR[..0]);
    // C compares `a->dst_prop.data == prop` by interned-pointer identity; the
    // ordering probe compares the same span as bytes (`c_strcmp` treats
    // end-of-slice as NUL, byte-exact strcmp for NUL-terminated-at-length data).
    let prop_data: *const u8 = prop.as_ptr();

    let conns = element.connections_dst_view();
    // C pre-initializes `begin = count` because the lower bound does not write
    // on a miss; `unwrap_or` reproduces that.
    let begin: usize = conns
        .lower_bound_eq(
            32,
            |a| c_strcmp(a.dst_prop_view().bytes(), prop) < 0,
            |a| a.dst_prop().data == prop_data && a.src_prop().length == 0,
        )
        .unwrap_or(conns.count());
    let end: usize = conns.upper_bound_eq(32, begin, |a| {
        a.dst_prop().data == prop_data && a.src_prop().length == 0
    });

    // C: `ufbx_connection_list result = { element->connections_dst.data + begin, end - begin };`
    // `List<T>` carries a private `PhantomData` marker, so the C aggregate
    // initializer becomes a zeroed value with both public fields written.
    // SAFETY: `List<Connection>` is a raw pointer, a `usize` and a zero-sized
    // `PhantomData`, so the all-zero bit pattern is a valid (null, empty) value.
    let mut result: List<Connection> = unsafe { MaybeUninit::zeroed().assume_init() };
    // C writes `data + begin` even for an empty range (`begin <= count`, so it
    // is at most one past the end); the wrapping projection keeps the address
    // without an in-bounds dereference claim.
    result.data = conns.data().wrapping_add(begin);
    result.count = end - begin;
    result
}

// ufbx.c:19016-19033 `ufbxi_find_src_connections`
// `prop: Option<&[u8]>` carries C's nullable `const char *prop` (see
// `find_dst_connections`).
#[inline(never)]
#[must_use]
pub(crate) fn find_src_connections(element: &ElementView, prop: Option<&[u8]>) -> List<Connection> {
    // C: `if (!prop) prop = ufbxi_empty_char;` — the empty span's base address
    // is `ufbxi_empty_char` itself, which the identity probes below compare.
    let prop: &[u8] = prop.unwrap_or(&EMPTY_CHAR[..0]);
    // C compares `a->src_prop.data == prop` by interned-pointer identity; the
    // ordering probe compares the same span as bytes (`c_strcmp` treats
    // end-of-slice as NUL, byte-exact strcmp for NUL-terminated-at-length data).
    let prop_data: *const u8 = prop.as_ptr();

    let conns = element.connections_src_view();
    // C pre-initializes `begin = count` because the lower bound does not write
    // on a miss; `unwrap_or` reproduces that.
    let begin: usize = conns
        .lower_bound_eq(
            32,
            |a| c_strcmp(a.src_prop_view().bytes(), prop) < 0,
            |a| a.src_prop().data == prop_data && a.dst_prop().length == 0,
        )
        .unwrap_or(conns.count());
    let end: usize = conns.upper_bound_eq(32, begin, |a| {
        a.src_prop().data == prop_data && a.dst_prop().length == 0
    });

    // C: `ufbx_connection_list result = { element->connections_src.data + begin, end - begin };`
    // SAFETY: `List<Connection>` is a raw pointer, a `usize` and a zero-sized
    // `PhantomData`, so the all-zero bit pattern is a valid (null, empty) value.
    let mut result: List<Connection> = unsafe { MaybeUninit::zeroed().assume_init() };
    // C writes `data + begin` even for an empty range (`begin <= count`, so it
    // is at most one past the end); the wrapping projection keeps the address
    // without an in-bounds dereference claim.
    result.data = conns.data().wrapping_add(begin);
    result.count = end - begin;
    result
}

// ufbx.c:19035-19045 `ufbxi_get_element_node`
//
// # Safety
// `element` is null or heads a live, arena-owned `ufbx_element`, and when its
// `type_` is `UFBX_ELEMENT_NODE` the same address heads the enclosing
// `ufbx_node`. The param stays a raw pointer because the `UFBX_ELEMENT_NODE`
// branch reads `is_geometry_transform_helper` / `parent`, which lie past
// `size_of::<Element>()`: a `&View<Element>` tags the header only, so a
// pointer derived from it may not address the node body.
#[must_use]
pub(crate) unsafe fn get_element_node(element: *mut Element) -> *mut Element {
    if element.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: `element` is non-null (checked) and points to a live scene element
    // in the arena (fn contract), so its pointer anchors an `ElementView`.
    let element_view: &ElementView = unsafe { ElementView::from_ptr(element) };
    if element_view.type_() == ElementType::Node {
        let node: *mut Node = element as *mut Node;
        // SAFETY: `type_ == Node` (checked) means the element is the `element`
        // prefix of a live `ufbx_node`, so the cast pointer is a valid `Node`.
        if unsafe { (*node).is_geometry_transform_helper } {
            // SAFETY: as above; `parent` is a live `Ref<Node>` field of that node,
            // which `opt_ptr` reads as a nullable pointer.
            return unsafe { opt_ptr(&raw const (*node).parent) } as *mut Element;
        }
        ptr::null_mut()
    } else {
        // C: `return element->instances.count > 0 ? &element->instances.data[0]->element : NULL;`
        if element_view.instances().count > 0 {
            // SAFETY: `instances.count > 0` (checked), so `instances.data` points
            // at a live `Ref<Node>` whose referent is a live node; taking the
            // address of its `element` prefix does not read the node.
            unsafe { &raw mut (*ref_ptr(element_view.instances().data)).element }
        } else {
            ptr::null_mut()
        }
    }
}

// ufbx.c:19047-19083 `ufbxi_fetch_dst_elements`
//
// C's `void *p_dst_list` is typed here as the element-ref list header every
// call site passes (C: `ufbx_element_list *list = (ufbx_element_list*)p_dst_list;`),
// and the nullable `const char *prop` as `Option<&[u8]>` (see
// `find_dst_connections`).
//
// # Safety
// `element` heads a live, arena-owned `ufbx_element` whose provenance spans the
// ENCLOSING element struct: with `search_node` the walk reaches
// `ufbxi_get_element_node`, which reads `ufbx_node` fields past
// `size_of::<Element>()`, so a pointer derived from a header-only
// `&View<Element>` may not address them.
#[inline(never)]
pub(crate) unsafe fn fetch_dst_elements(
    uc: &Context,
    p_dst_list: &RefListView<Element>,
    element: *mut Element,
    search_node: bool,
    ignore_duplicates: bool,
    prop: Option<&[u8]>,
    src_type: ElementType,
) -> Result<(), Fail> {
    let mut element: *mut Element = element;
    let mut num_elements: usize = 0;

    loop {
        let conns: List<Connection> = find_dst_connections(
            // SAFETY: `element` is a live scene element — the caller's on the
            // first pass, `get_element_node`'s non-null result after that.
            unsafe { ElementView::from_ptr(element) },
            prop,
        );
        // C: `ufbxi_for_list(ufbx_connection, conn, conns)` — indexed here
        // because the body `continue`s (the C `for` advances the iterator in
        // its increment clause).
        for conn_ix in 0..conns.count {
            // SAFETY: `conn_ix < conns.count` and `conns` is a `data`/`count` span
            // of live connections carved out of the element's connection array.
            let conn: *mut Connection = unsafe { (conns.data as *mut Connection).add(conn_ix) };
            // SAFETY: `conn` points to a live `Connection` whose `src` ref names
            // a live arena scene element, which anchors an `ElementView`.
            let src_view: &ElementView =
                unsafe { ElementView::from_ptr(ref_ptr(&raw const (*conn).src)) };
            if src_view.type_() == src_type {
                if ignore_duplicates {
                    let element_id: u32 = src_view.element_id();
                    // SAFETY: `tmp_element_flag` is a byte per scene element and
                    // `element_id` indexes the scene's element array.
                    if unsafe { *uc.tmp_element_flag().add(element_id as usize) } != 0 {
                        ufbxi_check!(
                            uc,
                            // SAFETY: `element` is a live scene element, and
                            // `ufbxi_warnf_tag!` formats the `%u` from the `u32`
                            // read here.
                            unsafe {
                                ufbxi_warnf_tag!(
                                    uc,
                                    WarningType::DuplicateConnection,
                                    element_id,
                                    "Duplicate connection to %u",
                                    (*element).element_id
                                )
                            }
                            .is_ok(),
                            "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_DUPLICATE_CONNECTION, (element_id), \"Duplicate connection to %u\", element->element_id)"
                        );
                        continue;
                    }
                    // SAFETY: as the read above — `element_id` indexes the
                    // per-element flag byte array.
                    unsafe { *uc.tmp_element_flag().add(element_id as usize) = 1 };
                }
                let p_elem: *mut *mut Element = uc.tmp_stack_view().push(1);
                ufbxi_check!(uc, !p_elem.is_null(), "p_elem");
                // SAFETY: `p_elem` is non-null (checked) and addresses the slot
                // just pushed on `tmp_stack`; `conn` points to a live `Connection`
                // whose `src` ref names a live scene element.
                unsafe { *p_elem = ref_ptr(&raw const (*conn).src) };
                num_elements += 1;
            }
        }

        if !(search_node && {
            // SAFETY: `element` is a live scene element (see the `find_dst_connections`
            // call above).
            element = unsafe { get_element_node(element) };
            !element.is_null()
        }) {
            break;
        }
    }

    // C: `ufbx_element_list *list = (ufbx_element_list*)p_dst_list;` — the
    // cast is the parameter type here.
    p_dst_list.set_data(
        uc.result_view()
            .push_pop::<*mut Element>(uc.tmp_stack_view(), num_elements)
            as *const Ref<Element>,
    );
    p_dst_list.set_count(num_elements);
    ufbxi_check!(uc, !p_dst_list.data().is_null(), "list->data");

    if ignore_duplicates {
        // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, *list)` — indexed here
        // over the same run of element refs the fetch just wrote.
        for elem_ix in 0..p_dst_list.count() {
            let p_elem: &ElementView = p_dst_list.at(elem_ix);
            // SAFETY: `tmp_element_flag` is a byte per scene element and the
            // listed element's `element_id` indexes the scene's element array.
            unsafe { *uc.tmp_element_flag().add(p_elem.element_id() as usize) = 0 };
        }
    }

    Ok(())
}

// ufbx.c:19085-19121 `ufbxi_fetch_src_elements`
//
// C's `void *p_dst_list` is typed here as the element-ref list header every
// call site passes (C: `ufbx_element_list *list = (ufbx_element_list*)p_dst_list;`),
// and the nullable `const char *prop` as `Option<&[u8]>` (see
// `find_src_connections`).
//
// # Safety
// `element` heads a live, arena-owned `ufbx_element`. With `search_node` set,
// its provenance must additionally span the ENCLOSING element struct: the walk
// then reaches `ufbxi_get_element_node`, which reads `ufbx_node` fields past
// `size_of::<Element>()`, so a pointer derived from a header-only
// `&View<Element>` may not address them. With `search_node` clear the walk
// stays within the `ufbx_element` header, where header-only provenance suffices.
#[inline(never)]
pub(crate) unsafe fn fetch_src_elements(
    uc: &Context,
    p_dst_list: &RefListView<Element>,
    element: *mut Element,
    search_node: bool,
    ignore_duplicates: bool,
    prop: Option<&[u8]>,
    dst_type: ElementType,
) -> Result<(), Fail> {
    let mut element: *mut Element = element;
    let mut num_elements: usize = 0;

    loop {
        let conns: List<Connection> = find_src_connections(
            // SAFETY: `element` is a live scene element — the caller's on the
            // first pass, `get_element_node`'s non-null result after that.
            unsafe { ElementView::from_ptr(element) },
            prop,
        );
        // C: `ufbxi_for_list(ufbx_connection, conn, conns)` — indexed here
        // because the body `continue`s.
        for conn_ix in 0..conns.count {
            // SAFETY: `conn_ix < conns.count` and `conns` is a `data`/`count` span
            // of live connections carved out of the element's connection array.
            let conn: *mut Connection = unsafe { (conns.data as *mut Connection).add(conn_ix) };
            // SAFETY: `conn` points to a live `Connection` whose `dst` ref names
            // a live arena scene element, which anchors an `ElementView`.
            let dst_view: &ElementView =
                unsafe { ElementView::from_ptr(ref_ptr(&raw const (*conn).dst)) };
            if dst_view.type_() == dst_type {
                if ignore_duplicates {
                    let element_id: u32 = dst_view.element_id();
                    // SAFETY: `tmp_element_flag` is a byte per scene element and
                    // `element_id` indexes the scene's element array.
                    if unsafe { *uc.tmp_element_flag().add(element_id as usize) } != 0 {
                        ufbxi_check!(
                            uc,
                            // SAFETY: `element` is a live scene element, and
                            // `ufbxi_warnf_tag!` formats the `%u` from the `u32`
                            // read here.
                            unsafe {
                                ufbxi_warnf_tag!(
                                    uc,
                                    WarningType::DuplicateConnection,
                                    element_id,
                                    "Duplicate connection to %u",
                                    (*element).element_id
                                )
                            }
                            .is_ok(),
                            "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_DUPLICATE_CONNECTION, (element_id), \"Duplicate connection to %u\", element->element_id)"
                        );
                        continue;
                    }
                    // SAFETY: as the read above — `element_id` indexes the
                    // per-element flag byte array.
                    unsafe { *uc.tmp_element_flag().add(element_id as usize) = 1 };
                }
                let p_elem: *mut *mut Element = uc.tmp_stack_view().push(1);
                ufbxi_check!(uc, !p_elem.is_null(), "p_elem");
                // SAFETY: `p_elem` is non-null (checked) and addresses the slot
                // just pushed on `tmp_stack`; `conn` points to a live `Connection`
                // whose `dst` ref names a live scene element.
                unsafe { *p_elem = ref_ptr(&raw const (*conn).dst) };
                num_elements += 1;
            }
        }

        if !(search_node && {
            // SAFETY: `element` is a live scene element (see the `find_src_connections`
            // call above).
            element = unsafe { get_element_node(element) };
            !element.is_null()
        }) {
            break;
        }
    }

    // C: `ufbx_element_list *list = (ufbx_element_list*)p_dst_list;` — the
    // cast is the parameter type here.
    p_dst_list.set_data(
        uc.result_view()
            .push_pop::<*mut Element>(uc.tmp_stack_view(), num_elements)
            as *const Ref<Element>,
    );
    p_dst_list.set_count(num_elements);
    ufbxi_check!(uc, !p_dst_list.data().is_null(), "list->data");

    if ignore_duplicates {
        // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, *list)` — indexed here
        // over the same run of element refs the fetch just wrote.
        for elem_ix in 0..p_dst_list.count() {
            let p_elem: &ElementView = p_dst_list.at(elem_ix);
            // SAFETY: `tmp_element_flag` is a byte per scene element and the
            // listed element's `element_id` indexes the scene's element array.
            unsafe { *uc.tmp_element_flag().add(p_elem.element_id() as usize) = 0 };
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

    // SAFETY: `prop` is null or a NUL-terminated interned property name (fn
    // contract) — the measure and the measured run are that contract; minted
    // once here as `find_dst_connections`' query span.
    let prop: Option<&[u8]> = if prop.is_null() {
        None
    } else {
        Some(unsafe { slice_from_ptr(prop, strlen(prop)) })
    };

    loop {
        let conns: List<Connection> = find_dst_connections(
            // SAFETY: `element` is a live scene element — the caller's on the
            // first pass, `get_element_node`'s non-null result after that.
            unsafe { ElementView::from_ptr(element) },
            prop,
        );
        // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
        let mut conn: *mut Connection = conns.data as *mut Connection;
        let conn_end: *mut Connection = add_ptr(conn, conns.count);
        while conn != conn_end {
            // SAFETY: `conn` is inside that span and its `src` ref names a live
            // scene element.
            unsafe {
                if (*ref_ptr(&raw const (*conn).src)).type_ == src_type {
                    return ref_ptr(&raw const (*conn).src);
                }
            }
            // SAFETY: `conn != conn_end`, so the advance lands at or before the
            // one-past-the-end pointer.
            conn = unsafe { conn.add(1) };
        }

        if !(search_node && {
            // SAFETY: `element` is a live scene element (see above).
            element = unsafe { get_element_node(element) };
            !element.is_null()
        }) {
            break;
        }
    }

    ptr::null_mut()
}

// ufbx.c:19137-19149 `ufbxi_fetch_src_element`
//
// C's nullable `const char *prop` is typed here as `Option<&[u8]>` (see
// `find_src_connections`).
//
// # Safety
// `element` heads a live, arena-owned `ufbx_element`. With `search_node` set,
// its provenance must additionally span the ENCLOSING element struct: the walk
// then reaches `ufbxi_get_element_node`, which reads `ufbx_node` fields past
// `size_of::<Element>()`, so a pointer derived from a header-only
// `&View<Element>` may not address them. With `search_node` clear the walk
// stays within the `ufbx_element` header, where header-only provenance suffices.
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_src_element(
    element: *mut Element,
    search_node: bool,
    prop: Option<&[u8]>,
    dst_type: ElementType,
) -> *mut Element {
    let mut element: *mut Element = element;

    loop {
        let conns: List<Connection> = find_src_connections(
            // SAFETY: `element` is a live scene element — the caller's on the
            // first pass, `get_element_node`'s non-null result after that.
            unsafe { ElementView::from_ptr(element) },
            prop,
        );
        // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
        let mut conn: *mut Connection = conns.data as *mut Connection;
        let conn_end: *mut Connection = add_ptr(conn, conns.count);
        while conn != conn_end {
            // SAFETY: `conn` is inside that span and its `dst` ref names a live
            // scene element.
            unsafe {
                if (*ref_ptr(&raw const (*conn).dst)).type_ == dst_type {
                    return ref_ptr(&raw const (*conn).dst);
                }
            }
            // SAFETY: `conn != conn_end`, so the advance lands at or before the
            // one-past-the-end pointer.
            conn = unsafe { conn.add(1) };
        }

        if !(search_node && {
            // SAFETY: `element` is a live scene element (see above).
            element = unsafe { get_element_node(element) };
            !element.is_null()
        }) {
            break;
        }
    }

    ptr::null_mut()
}

// ufbx.c:19151-19173 `ufbxi_fetch_textures`
//
// # Safety
// `element` heads a live, arena-owned `ufbx_element`. With `search_node` set,
// its provenance must additionally span the ENCLOSING element struct: the walk
// then reaches `ufbxi_get_element_node`, which reads `ufbx_node` fields past
// `size_of::<Element>()`, so a pointer derived from a header-only
// `&View<Element>` may not address them. With `search_node` clear the walk
// stays within the `ufbx_element` header, where header-only provenance suffices.
#[inline(never)]
pub(crate) unsafe fn fetch_textures(
    uc: &Context,
    list: &ListView<MaterialTexture>,
    element: *mut Element,
    search_node: bool,
) -> Result<(), Fail> {
    let mut element: *mut Element = element;
    let mut num_textures: usize = 0;

    loop {
        // SAFETY: `element` is a live scene element — the caller's on the first
        // pass, `get_element_node`'s non-null result after that.
        let element_view: &ElementView = unsafe { ElementView::from_ptr(element) };
        // C: `ufbxi_for_list(ufbx_connection, conn, element->connections_dst)`
        // — indexed here because the body `continue`s.
        for conn_ix in 0..element_view.connections_dst().count {
            // SAFETY: `conn_ix` is bounded by that element's own `connections_dst`
            // count, so the offset stays inside its connection array.
            let conn: *mut Connection =
                unsafe { (element_view.connections_dst().data as *mut Connection).add(conn_ix) };
            // SAFETY: `conn` points to a live `Connection`.
            if unsafe { (*conn).src_prop.length } > 0 {
                continue;
            }
            // SAFETY: as above; the `src` ref names a live scene element.
            if unsafe { (*ref_ptr(&raw const (*conn).src)).type_ } == ElementType::Texture {
                let tex: *mut MaterialTexture = uc.tmp_stack_view().push(1);
                ufbxi_check!(uc, !tex.is_null(), "tex");
                // C: `tex->shader_prop = tex->material_prop = conn->dst_prop;`
                // SAFETY: `tex` is non-null (checked) and addresses the
                // `MaterialTexture` slot just pushed on `tmp_stack`; `conn` points
                // to a live `Connection`.
                unsafe {
                    (*tex).material_prop = (*conn).dst_prop;
                    (*tex).shader_prop = (*tex).material_prop;
                }
                // SAFETY: as above; `src` has `type_ == Texture` (checked), so it
                // names a live `ufbx_texture`.
                unsafe {
                    (*tex).texture = Ref::from_ptr(ref_ptr(&raw const (*conn).src) as *mut Texture)
                };
                num_textures += 1;
            }
        }

        if !(search_node && {
            // SAFETY: `element` is a live scene element (see above).
            element = unsafe { get_element_node(element) };
            !element.is_null()
        }) {
            break;
        }
    }

    list.set_data(
        uc.result_view()
            .push_pop::<MaterialTexture>(uc.tmp_stack_view(), num_textures),
    );
    list.set_count(num_textures);
    ufbxi_check!(uc, !list.data().is_null(), "list->data");

    Ok(())
}

// ufbx.c:19175-19197 `ufbxi_fetch_mesh_materials`
//
// # Safety
// `element` heads a live, arena-owned `ufbx_element`. With `search_node` set,
// its provenance must additionally span the ENCLOSING element struct: the walk
// then reaches `ufbxi_get_element_node`, which reads `ufbx_node` fields past
// `size_of::<Element>()`, so a pointer derived from a header-only
// `&View<Element>` may not address them. With `search_node` clear the walk
// stays within the `ufbx_element` header, where header-only provenance suffices.
#[inline(never)]
pub(crate) unsafe fn fetch_mesh_materials(
    uc: &Context,
    list: &RefListView<Material>,
    element: *mut Element,
    search_node: bool,
) -> Result<(), Fail> {
    let mut element: *mut Element = element;
    let mut num_materials: usize = 0;

    loop {
        let conns: List<Connection> = find_dst_connections(
            // SAFETY: `element` is a live scene element — the caller's on the
            // first pass, `get_element_node`'s non-null result after that.
            unsafe { ElementView::from_ptr(element) },
            None,
        );
        // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
        let mut conn: *mut Connection = conns.data as *mut Connection;
        let conn_end: *mut Connection = add_ptr(conn, conns.count);
        while conn != conn_end {
            // SAFETY: `conn` is inside that span and its `src` ref names a live
            // scene element.
            if unsafe { (*ref_ptr(&raw const (*conn).src)).type_ } == ElementType::Material {
                // SAFETY: as above, with `type_ == Material` (checked) making the
                // referent a live `ufbx_material`.
                let mat: *mut Material =
                    unsafe { ref_ptr(&raw const (*conn).src) } as *mut Material;
                ufbxi_check!(
                    uc,
                    !uc.tmp_stack_view().push_copy_ref(&mat).is_null(),
                    "((ufbx_material**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_material*), (1), (&mat)))"
                );
                num_materials += 1;
            }
            // SAFETY: `conn != conn_end`, so the advance lands at or before the
            // one-past-the-end pointer.
            conn = unsafe { conn.add(1) };
        }

        if num_materials > 0 {
            break;
        }

        if !(search_node && {
            // SAFETY: `element` is a live scene element (see above).
            element = unsafe { get_element_node(element) };
            !element.is_null()
        }) {
            break;
        }
    }

    list.set_data(
        uc.result_view()
            .push_pop::<*mut Material>(uc.tmp_stack_view(), num_materials)
            as *const Ref<Material>,
    );
    list.set_count(num_materials);
    ufbxi_check!(uc, !list.data().is_null(), "list->data");

    Ok(())
}

// ufbx.c:19199-19219 `ufbxi_fetch_deformers`
//
// # Safety
// `element` heads a live, arena-owned `ufbx_element`. With `search_node` set,
// its provenance must additionally span the ENCLOSING element struct: the walk
// then reaches `ufbxi_get_element_node`, which reads `ufbx_node` fields past
// `size_of::<Element>()`, so a pointer derived from a header-only
// `&View<Element>` may not address them. With `search_node` clear the walk
// stays within the `ufbx_element` header, where header-only provenance suffices.
#[inline(never)]
pub(crate) unsafe fn fetch_deformers(
    uc: &Context,
    list: &RefListView<Element>,
    element: *mut Element,
    search_node: bool,
) -> Result<(), Fail> {
    let mut element: *mut Element = element;
    let mut num_deformers: usize = 0;

    loop {
        // SAFETY: `element` is a live scene element — the caller's on the first
        // pass, `get_element_node`'s non-null result after that.
        let element_view: &ElementView = unsafe { ElementView::from_ptr(element) };
        // C: `ufbxi_for_list(ufbx_connection, conn, element->connections_dst)`
        // — indexed here because the body `continue`s.
        for conn_ix in 0..element_view.connections_dst().count {
            // SAFETY: `conn_ix` is bounded by that element's own `connections_dst`
            // count, so the offset stays inside its connection array.
            let conn: *mut Connection =
                unsafe { (element_view.connections_dst().data as *mut Connection).add(conn_ix) };
            // SAFETY: `conn` points to a live `Connection`.
            if unsafe { (*conn).src_prop.length } > 0 {
                continue;
            }
            // SAFETY: as above; the `src` ref names a live scene element.
            let type_: ElementType = unsafe { (*ref_ptr(&raw const (*conn).src)).type_ };
            if type_ == ElementType::SkinDeformer
                || type_ == ElementType::BlendDeformer
                || type_ == ElementType::CacheDeformer
            {
                ufbxi_check!(
                    uc,
                    // SAFETY: `tmp_stack_mut_ptr()` is `uc`'s own live buffer, and
                    // the source is the address of `conn`'s `src` ref — one live,
                    // pointer-sized value, reinterpreted as the `*mut Element` it
                    // holds.
                    !unsafe {
                        uc.tmp_stack_view().push_copy_raw::<*mut Element>(1,
                            &raw const (*conn).src as *const *mut Element,
                        )
                    }
                    .is_null(),
                    "((ufbx_element**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_element*), (1), (&conn->src)))"
                );
                num_deformers += 1;
            }
        }

        if !(search_node && {
            // SAFETY: `element` is a live scene element (see above).
            element = unsafe { get_element_node(element) };
            !element.is_null()
        }) {
            break;
        }
    }

    list.set_data(
        uc.result_view()
            .push_pop::<*mut Element>(uc.tmp_stack_view(), num_deformers)
            as *const Ref<Element>,
    );
    list.set_count(num_deformers);
    ufbxi_check!(uc, !list.data().is_null(), "list->data");

    Ok(())
}

// ufbx.c:19221-19239 `ufbxi_fetch_blend_keyframes`
#[inline(never)]
pub(crate) fn fetch_blend_keyframes(
    uc: &Context,
    list: &ListView<BlendKeyframe>,
    element: &View<Element>,
) -> Result<(), Fail> {
    let mut num_keyframes: usize = 0;

    // `None` is C's null `prop` — the "no prop filter" argument.
    let conns: List<Connection> = find_dst_connections(element, None);
    // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
    let mut conn: *mut Connection = conns.data as *mut Connection;
    let conn_end: *mut Connection = add_ptr(conn, conns.count);
    while conn != conn_end {
        // SAFETY: `conn` is inside that span and its `src` ref names a live scene
        // element.
        if unsafe { (*ref_ptr(&raw const (*conn).src)).type_ } == ElementType::BlendShape {
            // C: `ufbx_blend_keyframe key = { (ufbx_blend_shape*)conn->src };`
            // — the remaining fields are zero-initialized.
            let key = BlendKeyframe {
                // SAFETY: as above, with `type_ == BlendShape` (checked) making the
                // referent a live `ufbx_blend_shape`.
                shape: unsafe { Ref::from_ptr(ref_ptr(&raw const (*conn).src) as *mut BlendShape) },
                target_weight: 0.0,
                effective_weight: 0.0,
            };
            ufbxi_check!(
                uc,
                !uc.tmp_stack_view().push_copy_ref(&key).is_null(),
                "((ufbx_blend_keyframe*)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_blend_keyframe), (1), (&key)))"
            );
            num_keyframes += 1;
        }
        // SAFETY: `conn != conn_end`, so the advance lands at or before the
        // one-past-the-end pointer.
        conn = unsafe { conn.add(1) };
    }

    list.set_data(
        uc.result_view()
            .push_pop::<BlendKeyframe>(uc.tmp_stack_view(), num_keyframes),
    );
    list.set_count(num_keyframes);
    ufbxi_check!(uc, !list.data().is_null(), "list->data");

    Ok(())
}

// ufbx.c:19241-19262 `ufbxi_fetch_texture_layers`
#[inline(never)]
pub(crate) fn fetch_texture_layers(
    uc: &Context,
    list: &ListView<TextureLayer>,
    element: &View<Element>,
) -> Result<(), Fail> {
    let mut num_layers: usize = 0;

    // `None` is C's null `prop` — the "no prop filter" argument.
    let conns: List<Connection> = find_dst_connections(element, None);
    // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
    let mut conn: *mut Connection = conns.data as *mut Connection;
    let conn_end: *mut Connection = add_ptr(conn, conns.count);
    while conn != conn_end {
        // SAFETY: `conn` is inside the connection span and its `src` ref names a
        // live scene element.
        if unsafe { (*ref_ptr(&raw const (*conn).src)).type_ } == ElementType::Texture {
            // The layer's source texture is an arena element reached through the
            // connection (write provenance), so its view anchors the property
            // lookups exactly like the typed element views in `finalize_scene`.
            // SAFETY: as above, with `type_ == Texture` (checked) making the
            // referent a live, context-owned `ufbx_texture` — a write-capable
            // pointer, which is what minting a `TextureView` requires.
            let texture_view: &TextureView =
                unsafe { TextureView::from_ptr(ref_ptr(&raw const (*conn).src) as *mut Texture) };
            let texture: *mut Texture = texture_view.get();
            // C: `ufbx_texture_layer layer = { texture };` — the remaining
            // fields are zero-initialized (`UFBX_BLEND_TRANSLUCENT` == 0).
            let mut layer = TextureLayer {
                // SAFETY: `texture` is that same live `ufbx_texture`.
                texture: unsafe { Ref::from_ptr(texture) },
                blend_mode: BlendMode::Translucent,
                alpha: 0.0,
            };
            layer.alpha = find_real(texture_view.props_view(), &sp::Texture_alpha, 1.0);
            // C: `(ufbx_blend_mode)ufbxi_find_enum(...)` — `ufbxi_find_enum`
            // clamps the result to `[0, UFBX_BLEND_OVERLAY]`, every value of
            // which is a valid `ufbx_blend_mode`.
            // SAFETY: the clamp bounds passed here are `BlendMode::Replace` and
            // `BlendMode::Overlay`, so the result is one of the contiguous
            // `ufbx_blend_mode` discriminants.
            layer.blend_mode = unsafe {
                core::mem::transmute::<u32, BlendMode>(find_enum(
                    texture_view.props_view(),
                    &sp::BlendMode,
                    BlendMode::Replace as i64,
                    BlendMode::Overlay as i64,
                ) as u32)
            };
            ufbxi_check!(
                uc,
                !uc.tmp_stack_view().push_copy_ref(&layer).is_null(),
                "((ufbx_texture_layer*)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_texture_layer), (1), (&layer)))"
            );
            num_layers += 1;
        }
        // SAFETY: `conn != conn_end`, so the advance lands at or before the
        // one-past-the-end pointer.
        conn = unsafe { conn.add(1) };
    }

    list.set_data(
        uc.result_view()
            .push_pop::<TextureLayer>(uc.tmp_stack_view(), num_layers),
    );
    list.set_count(num_layers);
    ufbxi_check!(uc, !list.data().is_null(), "list->data");

    Ok(())
}

// ufbx.c:19264-19269 `ufbxi_prop_connection_less`
// Probe over a view (the list-impl search mints it); `prop: &[u8]` carries C's
// NUL-terminated query string, minted once by the caller.
#[inline(always)]
pub(crate) fn prop_connection_less<M: crate::native::view::Mode>(
    a: &View<Connection, M>,
    prop: &[u8],
) -> bool {
    // C: `strcmp(a->dst_prop.data, prop)` over an interned (NUL-terminated)
    // span — `c_strcmp` stops at the first NUL like `strcmp`.
    let cmp: i32 = c_strcmp(a.dst_prop_view().bytes(), prop);
    if cmp != 0 {
        return cmp < 0;
    }
    a.src_prop_view().length() == 0
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

    // SAFETY: `element` points to a live scene element (fn contract). It is a
    // `*const` parameter, so it anchors a read-only `Const` view; nothing writes
    // the element while the search below runs.
    let element_view: &View<Element, Const> = unsafe { View::<Element, Const>::from_ptr(element) };
    // SAFETY: `prop` is a NUL-terminated C string (fn contract; the null case
    // was substituted above), minted once as the probes' query span.
    let prop_bytes: &[u8] = unsafe { crate::prelude::slice_from_ptr(prop, strlen(prop)) };

    let index: Option<usize> = element_view.connections_dst_view().lower_bound_eq(
        32,
        |a| prop_connection_less(a, prop_bytes),
        |a| a.dst_prop_view().data() == prop && a.src_prop_view().length() > 0,
    );

    if let Some(index) = index {
        // `index` is a hit position within `0..connections_dst.count`; the
        // result is derived from the list's own base so it keeps whole-run
        // provenance.
        element_view
            .connections_dst_view()
            .data()
            .wrapping_add(index) as *mut Connection
    } else {
        ptr::null_mut()
    }
}

// ufbx.c:19285-19292 `ufbxi_patch_index_pointer`
#[inline(always)]
pub(crate) unsafe fn patch_index_pointer(uc: &Context, p_index: *mut *mut u32) {
    // SAFETY: `p_index` points to a live index-pointer slot of a scene attribute
    // (fn contract).
    unsafe {
        if std::ptr::eq(*p_index, SENTINEL_INDEX_ZERO.as_ptr()) {
            *p_index = uc.zero_indices();
        } else if std::ptr::eq(*p_index, SENTINEL_INDEX_CONSECUTIVE.as_ptr()) {
            *p_index = uc.consecutive_indices();
        }
    }
}

// ufbx.c:19294-19299 `ufbxi_cmp_anim_prop_less`
// Comparator over views: the sort adapter mints them (PORTING.md "Sorting").
#[must_use]
pub(crate) fn cmp_anim_prop_less<M: crate::native::view::Mode>(
    a: &View<AnimProp, M>,
    b: &View<AnimProp, M>,
) -> bool {
    // C: `if (a->element != b->element) return a->element < b->element;` —
    // element pointer identity and address order.
    let (a_element, b_element) = (a.element().ptr(), b.element().ptr());
    if a_element != b_element {
        return a_element < b_element;
    }
    if a._internal_key() != b._internal_key() {
        return a._internal_key() < b._internal_key();
    }
    str_less(a.prop_name_view().bytes(), b.prop_name_view().bytes())
}

// ufbx.c:19301-19306 `ufbxi_sort_anim_props`
#[inline(never)]
pub(crate) unsafe fn sort_anim_props(
    uc: &Context,
    aprops: *mut AnimProp,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the three pointers are `uc`'s own live `ator_tmp` and the
        // `tmp_arr`/`tmp_arr_size` slots that pair with it.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count.wrapping_mul(size_of::<AnimProp>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_anim_prop)))"
    );
    // SAFETY: `aprops` addresses `count` initialized `AnimProp`s (fn contract)
    // and `tmp_arr` was just grown to `count * size_of::<AnimProp>()` bytes, so
    // the two disjoint runs `macro_stable_sort` needs are in place; it hands the
    // comparator pointers to live elements of those runs.
    unsafe {
        macro_stable_sort_views::<AnimProp>(
            32,
            aprops,
            uc.tmp_arr() as *mut AnimProp,
            count,
            cmp_anim_prop_less,
        )
    };
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
    // SAFETY: `va`/`vb` are the two `ufbx_material_texture` elements the sort
    // hands this C-callback comparator (the sort is instantiated over that type),
    // and both `material_prop`s are interned string-pool spans.
    unsafe { str_less((*a).material_prop.as_bytes(), (*b).material_prop.as_bytes()) }
}

// ufbx.c:19315-19320 `ufbxi_sort_material_textures`
#[inline(never)]
pub(crate) unsafe fn sort_material_textures(
    uc: &Context,
    textures: *mut MaterialTexture,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the three pointers are `uc`'s own live `ator_tmp` and the
        // `tmp_arr`/`tmp_arr_size` slots that pair with it.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count.wrapping_mul(size_of::<MaterialTexture>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_material_texture)))"
    );
    // SAFETY: `textures` addresses `count` initialized `MaterialTexture`s (fn
    // contract) and `tmp_arr` was just grown to
    // `count * size_of::<MaterialTexture>()` bytes, so both runs hold `count`
    // items of the element size passed here; `material_texture_less` is the
    // matching comparator and ignores its `user` argument.
    unsafe {
        stable_sort(
            size_of::<MaterialTexture>(),
            32,
            textures as *mut c_void,
            uc.tmp_arr() as *mut c_void,
            count,
            material_texture_less,
            ptr::null_mut(),
        )
    };
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
    // SAFETY: `va`/`vb` are the two `ufbx_bone_pose` elements the sort hands this
    // C-callback comparator (the sort is instantiated over that type), and each
    // `bone_node` ref names a live scene node.
    unsafe {
        (*ref_ptr(&raw const (*a).bone_node)).element.typed_id
            < (*ref_ptr(&raw const (*b).bone_node)).element.typed_id
    }
}

// ufbx.c:19329-19335 `ufbxi_find_anim_prop_start`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn find_anim_prop_start(
    layer: *mut AnimLayer,
    element: *const Element,
) -> *mut AnimProp {
    let mut index: usize = usize::MAX;
    // SAFETY: `layer` points to a live `ufbx_anim_layer` (fn contract) whose
    // `anim_props` is its own `data`/`count` span of initialized `AnimProp`s, so
    // the search range `0..count` is in bounds and every probe pointer the two
    // closures receive addresses a live anim prop whose `element` ref names a live
    // scene element.
    unsafe {
        macro_lower_bound_eq(
            16,
            &mut index,
            (*layer).anim_props.data,
            0,
            (*layer).anim_props.count,
            |a| (ref_ptr(&raw const (*a).element) as *const Element) < element,
            |a| std::ptr::eq(ref_ptr(&raw const (*a).element), element),
        )
    };
    if index != usize::MAX {
        // SAFETY: `index` is a hit position within `0..anim_props.count`, so the
        // offset stays inside the layer's anim-prop array.
        unsafe { (*layer).anim_props.data.add(index) as *mut AnimProp }
    } else {
        ptr::null_mut()
    }
}

// ufbx.c:19337-19343 `ufbxi_sort_bone_poses`
#[inline(never)]
pub(crate) fn sort_bone_poses(uc: &Context, pose: &View<Pose>) -> Result<(), Fail> {
    let count: usize = pose.bone_poses_view().count();
    ufbxi_check!(
        uc,
        // SAFETY: the three pointers are `uc`'s own live `ator_tmp` and the
        // `tmp_arr`/`tmp_arr_size` slots that pair with it.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                pose.bone_poses_view()
                    .count()
                    .wrapping_mul(size_of::<BonePose>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (pose->bone_poses.count * sizeof(ufbx_bone_pose)))"
    );
    // SAFETY: `bone_poses` is the pose's own `count`-element array and `tmp_arr`
    // was just grown to `count * size_of::<BonePose>()` bytes, so both runs hold
    // `count` items of the element size passed here; `bone_pose_less` is the
    // matching comparator and ignores its `user` argument.
    unsafe {
        stable_sort(
            size_of::<BonePose>(),
            16,
            pose.bone_poses_view().data() as *mut c_void,
            uc.tmp_arr() as *mut c_void,
            count,
            bone_pose_less,
            ptr::null_mut(),
        )
    };
    Ok(())
}

// ufbx.c:19345-19356 `ufbxi_sort_skin_weights`
//
// # Safety
//
// Every `skin.vertices` entry must describe a run that lies inside
// `skin.weights`: `weight_begin + num_weights <= weights.count`, with
// `num_weights <= skin.max_weights_per_vertex`. The view type vouches only that
// `vertices` and `weights` are each a valid run on their own; relating one
// list's contents to the other's bounds is the caller's obligation.
#[inline(never)]
pub(crate) unsafe fn sort_skin_weights(
    uc: &Context,
    skin: &View<SkinDeformer>,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the three pointers are `uc`'s own live `ator_tmp` and the
        // `tmp_arr`/`tmp_arr_size` slots that pair with it.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                skin.max_weights_per_vertex()
                    .wrapping_mul(size_of::<SkinWeight>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (skin->max_weights_per_vertex * sizeof(ufbx_skin_weight)))"
    );

    for i in 0..skin.vertices_view().count() {
        // C: `ufbx_skin_vertex v = skin->vertices.data[i];`
        let v: &View<SkinVertex> = skin.vertices_view().at(i);
        // SAFETY: by the fn contract the half-open range
        // `weight_begin .. weight_begin + num_weights` lies inside the deformer's
        // `weights` array, so the run sorted here is in bounds; `num_weights` is at
        // most `max_weights_per_vertex`, the count `tmp_arr` was just grown for, so
        // the scratch run is large enough and disjoint from `weights`.
        unsafe {
            macro_stable_sort::<SkinWeight>(
                32,
                (skin.weights_view().data() as *mut SkinWeight).add(v.weight_begin() as usize),
                uc.tmp_arr() as *mut SkinWeight,
                v.num_weights() as usize,
                |a, b| (*a).weight > (*b).weight,
            )
        };
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
    // SAFETY: `va`/`vb` are the two `ufbx_blend_keyframe` elements the sort hands
    // this C-callback comparator (the sort is instantiated over that type).
    unsafe { (*a).target_weight < (*b).target_weight }
}

// ufbx.c:19365-19370 `ufbxi_sort_blend_keyframes`
#[inline(never)]
pub(crate) unsafe fn sort_blend_keyframes(
    uc: &Context,
    keyframes: *mut BlendKeyframe,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the three pointers are `uc`'s own live `ator_tmp` and the
        // `tmp_arr`/`tmp_arr_size` slots that pair with it.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count.wrapping_mul(size_of::<BlendKeyframe>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_blend_keyframe)))"
    );
    // SAFETY: `keyframes` addresses `count` initialized `BlendKeyframe`s (fn
    // contract) and `tmp_arr` was just grown to
    // `count * size_of::<BlendKeyframe>()` bytes, so both runs hold `count` items
    // of the element size passed here; `blend_keyframe_less` is the matching
    // comparator and ignores its `user` argument.
    unsafe {
        stable_sort(
            size_of::<BlendKeyframe>(),
            32,
            keyframes as *mut c_void,
            uc.tmp_arr() as *mut c_void,
            count,
            blend_keyframe_less,
            ptr::null_mut(),
        )
    };
    Ok(())
}

// Material tables
// (ufbx.c:19372)

// ufbx.c:19374 `typedef void (*ufbxi_mat_transform_fn)(ufbx_vec4 *a);`
pub(crate) type MatTransformFn = unsafe extern "C" fn(a: *mut Vec4);

// ufbx.c:19376 `ufbxi_mat_transform_invert_x`
pub(crate) unsafe extern "C" fn mat_transform_invert_x(v: *mut Vec4) {
    // SAFETY: `v` is the live `ufbx_vec4` the mapping walker hands its transform
    // callback (C: `ufbxi_mat_transform_fn` is always called on `&map->value_vec4`).
    unsafe { (*v).x = 1.0 - (*v).x };
}
// ufbx.c:19377 `ufbxi_mat_transform_unknown_shininess`
// C-parity: `ufbx_sqrt` takes/returns `double`, so the product and the
// subtraction are evaluated in `double` regardless of `ufbx_real`'s width; the
// `(ufbx_real)0.1` cast happens before the widening back to `double`.
pub(crate) unsafe extern "C" fn mat_transform_unknown_shininess(v: *mut Vec4) {
    // SAFETY: `v` is the live `ufbx_vec4` the mapping walker hands its transform
    // callback (C: `ufbxi_mat_transform_fn` is always called on `&map->value_vec4`).
    unsafe {
        if (*v).x >= 0.0 {
            (*v).x = (1.0f64 - math::sqrt(as_f64!((*v).x)) * as_f64!(0.1f64 as Real)) as Real;
        }
        if !((*v).x >= 0.0) {
            (*v).x = 0.0;
        }
    }
}
// ufbx.c:19378 `ufbxi_mat_transform_blender_opacity`
pub(crate) unsafe extern "C" fn mat_transform_blender_opacity(v: *mut Vec4) {
    // SAFETY: `v` is the live `ufbx_vec4` the mapping walker hands its transform
    // callback (C: `ufbxi_mat_transform_fn` is always called on `&map->value_vec4`).
    unsafe { (*v).x = 1.0 - (*v).x };
}
// ufbx.c:19379 `ufbxi_mat_transform_blender_shininess`
pub(crate) unsafe extern "C" fn mat_transform_blender_shininess(v: *mut Vec4) {
    // SAFETY: `v` is the live `ufbx_vec4` the mapping walker hands its transform
    // callback (C: `ufbxi_mat_transform_fn` is always called on `&map->value_vec4`).
    unsafe {
        if (*v).x >= 0.0 {
            (*v).x = (1.0f64 - math::sqrt(as_f64!((*v).x)) * as_f64!(0.1f64 as Real)) as Real;
        }
        if !((*v).x >= 0.0) {
            (*v).x = 0.0;
        }
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
//
// # Safety
//
// Three raw-pointer parameters carry contracts the types cannot express:
// `mappings`/`count` must describe a run of `count` initialized
// `ShaderMapping` entries; `maps` must address a run of `ufbx_material_map`
// long enough for every `mapping->index` the table names when `flags` carries
// `MAPPING_FETCH_VALUE`/`_TEXTURE`/`_TEXTURE_ENABLED`; `features` must
// likewise address a run of `ufbx_material_feature_info` long enough for those
// indices when `flags` carries `MAPPING_FETCH_FEATURE`. Both arrays are
// written through, so they must carry write-capable provenance.
#[inline(never)]
pub(crate) unsafe fn fetch_mapping_maps(
    material: &MaterialView,
    maps: *mut MaterialMap,
    features: *mut MaterialFeatureInfo,
    shader: Option<&View<Shader, Const>>,
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
    // SAFETY: `mappings`/`count` describe one of the immutable static mapping
    // tables — `count` contiguous initialized `ShaderMapping` entries that live
    // for the whole program and are written by nobody, so the shared borrow is
    // sound for the walk. The table is read-only memory, which is why the run
    // is a plain slice rather than a `Mut`-mode `SliceViewIter`.
    let mapping_run: &[ShaderMapping] = unsafe { slice_from_ptr(mappings, count) };
    for mapping in mapping_run {
        // C: `ufbx_string prop_name = { mapping->prop, mapping->prop_len };`
        let mut prop_name: String = String::new_c(mapping.prop, mapping.prop_len as usize);
        if prefix.length > 0 || prefix2.length > 0 || suffix.length > 0 {
            // C: `sizeof(combined_name)` — the array is `char[512]`.
            if prop_name.length + prefix.length + prefix2.length + suffix.length
                <= size_of::<[u8; 512]>()
            {
                let mut dst: *mut u8 = combined_name;

                if prefix.length > 0 {
                    // SAFETY: the four lengths sum to at most 512 (checked), so
                    // `dst` has room for `prefix.length` more bytes in the 512-byte
                    // `combined_name` storage; `prefix` is a caller-supplied
                    // `ufbx_string` readable for its own `length`, the storage is
                    // a distinct local, and the advance stays within the 512 bytes.
                    unsafe {
                        ptr::copy_nonoverlapping(prefix.data, dst, prefix.length);
                        dst = dst.add(prefix.length);
                    }
                }
                if prefix2.length > 0 {
                    // SAFETY: as the `prefix` copy, with the running total still
                    // bounded by the 512-byte check.
                    unsafe {
                        ptr::copy_nonoverlapping(prefix2.data, dst, prefix2.length);
                        dst = dst.add(prefix2.length);
                    }
                }
                if prop_name.length > 0 {
                    // SAFETY: as above; `prop_name` still points at the mapping
                    // table's own `prop` bytes, readable for `prop_name.length`.
                    unsafe {
                        ptr::copy_nonoverlapping(prop_name.data, dst, prop_name.length);
                        dst = dst.add(prop_name.length);
                    }
                }
                if suffix.length > 0 {
                    // SAFETY: as the `prefix` copy, with the running total still
                    // bounded by the 512-byte check.
                    unsafe {
                        ptr::copy_nonoverlapping(suffix.data, dst, suffix.length);
                        dst = dst.add(suffix.length);
                    }
                }

                prop_name.data = combined_name;
                // SAFETY: `dst` was derived from `combined_name` by the advances
                // above, so both are pointers into that one 512-byte storage.
                prop_name.length = to_size(unsafe { dst.offset_from(combined_name) });
            }
        }

        // The search key is minted once per mapping.
        // SAFETY: `prop_name` is a `data`/`length` span readable for its length
        // — either the mapping table's `prop` bytes or the `combined_name`
        // prefix just written, neither of which is written again while the
        // slice is live.
        let prop_name_bytes: &[u8] = unsafe { prop_name.as_bytes() };
        let mut bindings: List<ShaderPropBinding> =
            find_shader_prop_bindings_len(shader, prop_name_bytes);
        if bindings.count == 0 {
            // SAFETY: `identity_binding` addresses this function's own aligned
            // `ShaderPropBinding` storage; both members are written here before
            // its address is handed to `bindings`.
            unsafe {
                (*identity_binding).material_prop = prop_name;
                (*identity_binding).shader_prop = EMPTY_STRING.0;
            }
            bindings.data = identity_binding;
            bindings.count = 1;
        }

        let mapping_flags: u32 = mapping.flags as u32;
        // C: `ufbxi_for_list(ufbx_shader_prop_binding, binding, bindings)`
        // SAFETY: `bindings` describes a contiguous run of initialized
        // `ShaderPropBinding` — either the shader's own binding span, which
        // nothing in this loop writes, or the single `identity_binding` written
        // just above.
        let binding_run: &[ShaderPropBinding] =
            unsafe { slice_from_ptr(bindings.data, bindings.count) };
        for binding in binding_run {
            let name: String = binding.material_prop;
            // The property key is minted once per binding.
            // SAFETY: `name` is a `data`/`length` span readable for its length,
            // pointing at either interned string-pool bytes or the
            // `combined_name` storage, neither written while the slice is live.
            let name_bytes: &[u8] = unsafe { name.as_bytes() };

            // C: `ufbx_find_prop_len(&material->props, ...)` — the material's
            // own property table, projected in place out of the element view;
            // every access through `prop` below is a read.
            let prop: Option<&View<Prop>> = find_prop_len(material.props_view(), name_bytes);
            if (flags & MAPPING_FETCH_FEATURE) != 0 {
                // SAFETY: with `MAPPING_FETCH_FEATURE` the caller's `features`
                // array is indexed by `mapping->index`, which the mapping tables
                // keep within the material's feature count; the entry address is
                // derived from the array base, and the view reinterprets that
                // entry in place with the caller's write-capable provenance.
                let feature: &View<MaterialFeatureInfo> = unsafe {
                    View::<MaterialFeatureInfo>::from_ptr(features.add(mapping.index as usize))
                };
                // C: `if (prop && prop->type != UFBX_PROP_REFERENCE)`
                if let Some(prop) = prop.filter(|p| p.type_() != PropType::Reference) {
                    feature.set_enabled(prop.value_int() != 0);
                    feature.set_is_explicit(true);
                    if (mapping_flags & SHADER_FEATURE_IF_AROUND_1 as u32) != 0 {
                        // C-parity: `prop->value_real` is the `ufbx_prop` value
                        // union's first real (`value_vec4.x` here).
                        feature.set_enabled(
                            prop.value_vec4().x >= 0.5f32 as Real
                                && prop.value_vec4().x <= 1.5f32 as Real,
                        );
                    }
                    if (mapping_flags & SHADER_FEATURE_INVERTED as u32) != 0 {
                        feature.set_enabled(!feature.enabled());
                    }
                    if (mapping_flags & SHADER_FEATURE_IF_EXISTS as u32) != 0 {
                        feature.set_enabled(true);
                    }
                }
                if (mapping_flags & SHADER_FEATURE_IF_TEXTURE as u32) != 0 {
                    // SAFETY: the material view's own pointer addresses a live
                    // `ufbx_material`, which is all this raw-pointer entry point
                    // requires.
                    let texture: *mut Texture =
                        unsafe { find_prop_texture_len(material.get(), name_bytes) };
                    if !texture.is_null() {
                        feature.set_enabled(true);
                    }
                }
                continue;
            }

            // C: `ufbx_material_map *map = &maps[mapping->index];`
            // SAFETY: the caller's `maps` array is indexed by `mapping->index`,
            // which the mapping tables keep within the material's map count; the
            // entry address is derived from the array base, and the view
            // reinterprets that entry in place with the caller's write-capable
            // provenance.
            let map: &MaterialMapView =
                unsafe { MaterialMapView::from_ptr(maps.add(mapping.index as usize)) };

            if (flags & MAPPING_FETCH_VALUE) != 0 {
                // C: `if (prop && prop->type != UFBX_PROP_REFERENCE)`
                if let Some(prop) = prop.filter(|p| p.type_() != PropType::Reference) {
                    if (mapping.flags & SHADER_MAPPING_MULTIPLY_VALUE) != 0 {
                        // C: `ufbxi_f64_to_i64(map->value_vec4.x)` — the real
                        // argument promotes to double at the call.
                        // SAFETY: the map view's own live `value_vec4` field; the
                        // C statement updates only its `x` lane, which no
                        // single-level view accessor can address.
                        unsafe { (*map.value_vec4_raw()).x *= prop.value_vec4().x };
                        map.set_value_int(f64_to_i64(as_f64!(map.value_vec4().x)));
                    } else {
                        map.set_value_vec4(prop.value_vec4());
                        map.set_value_int(prop.value_int());
                    }
                    map.set_has_value(true);
                    if mapping.transform != 0 {
                        // SAFETY: `mapping->transform` is a `MatTransform`
                        // discriminant below `MatTransform::Count`, and
                        // `MAT_TRANSFORM_FNS` holds a `Some` at every index above
                        // the identity slot 0, which the `!= 0` test excludes.
                        let transform_fn: MatTransformFn = unsafe {
                            MAT_TRANSFORM_FNS[mapping.transform as usize].unwrap_unchecked()
                        };
                        // SAFETY: the argument is the map view's own live
                        // `value_vec4`, which those callbacks are contracted to
                        // take.
                        unsafe { transform_fn(map.value_vec4_raw()) };
                    }

                    let prop_flags: u32 = prop.flags().raw();
                    if (mapping.flags & SHADER_MAPPING_DEFAULT_W_1) != 0
                        && (prop_flags & PropFlags::VALUE_VEC4.raw()) == 0
                    {
                        // SAFETY: the map view's own live `value_vec4` field; the
                        // C statement updates only its `w` lane.
                        unsafe { (*map.value_vec4_raw()).w = 1.0f32 as Real };
                    }
                    if (mapping.flags & SHADER_MAPPING_WIDEN_TO_RGB) != 0
                        && (prop_flags & PropFlags::VALUE_REAL.raw()) != 0
                    {
                        // C-parity: `map->value_vec3` is the `ufbx_material_map`
                        // value union's 3-real view; the generated struct keeps
                        // only `value_vec4`, whose x/y/z overlay it exactly.
                        // SAFETY: the map view's own live `value_vec4` field; the
                        // C statements update only its `y` and `z` lanes.
                        unsafe {
                            (*map.value_vec4_raw()).y = map.value_vec4().x;
                            (*map.value_vec4_raw()).z = map.value_vec4().x;
                        }
                    }
                    if (prop_flags & PropFlags::VALUE_REAL.raw()) != 0 {
                        map.set_value_components(1);
                    } else if (prop_flags & PropFlags::VALUE_VEC2.raw()) != 0 {
                        map.set_value_components(2);
                    } else if (prop_flags & PropFlags::VALUE_VEC3.raw()) != 0 {
                        map.set_value_components(3);
                    } else if (prop_flags & PropFlags::VALUE_VEC4.raw()) != 0 {
                        map.set_value_components(4);
                    } else {
                        map.set_value_components(0);
                    }
                }
            }

            if (flags & MAPPING_FETCH_TEXTURE) != 0 {
                // SAFETY: the material view's own pointer addresses a live
                // `ufbx_material`, which is all this raw-pointer entry point
                // requires.
                let texture: *mut Texture =
                    unsafe { find_prop_texture_len(material.get(), name_bytes) };
                if !texture.is_null() {
                    // SAFETY: `texture` is a non-null (checked) live
                    // `ufbx_texture`, so it is a valid element reference.
                    map.set_texture(unsafe { opt_ref(texture) });
                    map.set_texture_enabled(true);
                }
            }

            if (flags & MAPPING_FETCH_TEXTURE_ENABLED) != 0 {
                if let Some(prop) = prop {
                    map.set_texture_enabled(prop.value_int() != 0);
                }
            }
        }
    }
}

// ufbx.c:20096-20107 `ufbxi_update_factor`
#[inline(never)]
pub(crate) fn update_factor(factor_map: &MaterialMapView, color_map: &MaterialMapView) {
    if !factor_map.has_value() {
        if color_map.has_value() && !is_vec4_zero(color_map.value_vec4()) {
            // C-parity: `factor_map->value_real` is the value union's first
            // real (`value_vec4.x` in the generated struct).
            // SAFETY: the factor view's own live `value_vec4` field.
            unsafe { (*factor_map.value_vec4_raw()).x = 1.0f32 as Real };
            factor_map.set_value_int(1);
        } else {
            // SAFETY: as above.
            unsafe { (*factor_map.value_vec4_raw()).x = 0.0f32 as Real };
            factor_map.set_value_int(0);
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

// The named `ufbx_material_map` members of the two aggregates that
// `ufbxi_fetch_maps` (ufbx.c:20181-20198) touches by name: the
// `ufbxi_update_factor` pairs and the three maps of the transmission-roughness
// patch. Each projects one member of the aggregate in place.
impl crate::native::view::View<MaterialFbxMaps> {
    #[inline(always)]
    pub(crate) fn diffuse_factor_view(&self) -> &MaterialMapView {
        view_project!(self, diffuse_factor)
    }
    #[inline(always)]
    pub(crate) fn diffuse_color_view(&self) -> &MaterialMapView {
        view_project!(self, diffuse_color)
    }
    #[inline(always)]
    pub(crate) fn specular_factor_view(&self) -> &MaterialMapView {
        view_project!(self, specular_factor)
    }
    #[inline(always)]
    pub(crate) fn specular_color_view(&self) -> &MaterialMapView {
        view_project!(self, specular_color)
    }
    #[inline(always)]
    pub(crate) fn reflection_factor_view(&self) -> &MaterialMapView {
        view_project!(self, reflection_factor)
    }
    #[inline(always)]
    pub(crate) fn reflection_color_view(&self) -> &MaterialMapView {
        view_project!(self, reflection_color)
    }
    #[inline(always)]
    pub(crate) fn transparency_factor_view(&self) -> &MaterialMapView {
        view_project!(self, transparency_factor)
    }
    #[inline(always)]
    pub(crate) fn transparency_color_view(&self) -> &MaterialMapView {
        view_project!(self, transparency_color)
    }
    #[inline(always)]
    pub(crate) fn emission_factor_view(&self) -> &MaterialMapView {
        view_project!(self, emission_factor)
    }
    #[inline(always)]
    pub(crate) fn emission_color_view(&self) -> &MaterialMapView {
        view_project!(self, emission_color)
    }
    #[inline(always)]
    pub(crate) fn ambient_factor_view(&self) -> &MaterialMapView {
        view_project!(self, ambient_factor)
    }
    #[inline(always)]
    pub(crate) fn ambient_color_view(&self) -> &MaterialMapView {
        view_project!(self, ambient_color)
    }
}

// As above, for the PBR aggregate.
impl crate::native::view::View<MaterialPbrMaps> {
    #[inline(always)]
    pub(crate) fn base_factor_view(&self) -> &MaterialMapView {
        view_project!(self, base_factor)
    }
    #[inline(always)]
    pub(crate) fn base_color_view(&self) -> &MaterialMapView {
        view_project!(self, base_color)
    }
    #[inline(always)]
    pub(crate) fn roughness_view(&self) -> &MaterialMapView {
        view_project!(self, roughness)
    }
    #[inline(always)]
    pub(crate) fn specular_factor_view(&self) -> &MaterialMapView {
        view_project!(self, specular_factor)
    }
    #[inline(always)]
    pub(crate) fn specular_color_view(&self) -> &MaterialMapView {
        view_project!(self, specular_color)
    }
    #[inline(always)]
    pub(crate) fn transmission_factor_view(&self) -> &MaterialMapView {
        view_project!(self, transmission_factor)
    }
    #[inline(always)]
    pub(crate) fn transmission_color_view(&self) -> &MaterialMapView {
        view_project!(self, transmission_color)
    }
    #[inline(always)]
    pub(crate) fn transmission_roughness_view(&self) -> &MaterialMapView {
        view_project!(self, transmission_roughness)
    }
    #[inline(always)]
    pub(crate) fn transmission_extra_roughness_view(&self) -> &MaterialMapView {
        view_project!(self, transmission_extra_roughness)
    }
    #[inline(always)]
    pub(crate) fn sheen_factor_view(&self) -> &MaterialMapView {
        view_project!(self, sheen_factor)
    }
    #[inline(always)]
    pub(crate) fn sheen_color_view(&self) -> &MaterialMapView {
        view_project!(self, sheen_color)
    }
    #[inline(always)]
    pub(crate) fn thin_film_factor_view(&self) -> &MaterialMapView {
        view_project!(self, thin_film_factor)
    }
    #[inline(always)]
    pub(crate) fn thin_film_thickness_view(&self) -> &MaterialMapView {
        view_project!(self, thin_film_thickness)
    }
    #[inline(always)]
    pub(crate) fn emission_factor_view(&self) -> &MaterialMapView {
        view_project!(self, emission_factor)
    }
    #[inline(always)]
    pub(crate) fn emission_color_view(&self) -> &MaterialMapView {
        view_project!(self, emission_color)
    }
}

// `ufbxi_fetch_maps` (ufbx.c:20124) reaches the material's three map/feature
// aggregates two ways: as the flat arrays the `ufbxi_fetch_mapping_maps` calls
// take (the union arm pinned by the const asserts above, recovered from the
// generated `*_raw()` projections) and as the named members
// `ufbxi_update_factor` and the glossiness remap touch. The named members
// project in place; the two indexed entries are bounds-checked against the
// pinned array length.
impl crate::native::view::View<Material> {
    #[inline(always)]
    pub(crate) fn fbx_view(&self) -> &View<MaterialFbxMaps> {
        view_project!(self, fbx)
    }
    #[inline(always)]
    pub(crate) fn pbr_view(&self) -> &View<MaterialPbrMaps> {
        view_project!(self, pbr)
    }
    #[inline(always)]
    pub(crate) fn pbr_map_at(&self, index: usize) -> &MaterialMapView {
        assert!(index < MATERIAL_PBR_MAP_COUNT);
        // SAFETY: `pbr_raw` projects the material's own `pbr` member, whose
        // array arm is a flat run of `MATERIAL_PBR_MAP_COUNT` maps (const
        // assert above), so the bounds-checked index stays inside that member
        // and inherits its write-capable provenance.
        unsafe { MaterialMapView::from_ptr((self.pbr_raw() as *mut MaterialMap).add(index)) }
    }
    #[inline(always)]
    pub(crate) fn feature_at(&self, index: usize) -> &View<MaterialFeatureInfo> {
        assert!(index < MATERIAL_FEATURE_COUNT as usize);
        // SAFETY: as `pbr_map_at`, for the `features` member, whose array arm is
        // a flat run of `MATERIAL_FEATURE_COUNT` feature infos.
        unsafe {
            View::<MaterialFeatureInfo>::from_ptr(
                (self.features_raw() as *mut MaterialFeatureInfo).add(index),
            )
        }
    }
}

// C-parity: `ufbx_material_map`'s value union (ufbx.h:2293-2298) overlays the
// scalar `value_real` on the first component of `value_vec4`; the generator
// keeps only the vec4 arm (PORTING.md "Unions and flexible array members"), so
// the scalar member is reached as its `.x`.
impl MaterialMapView {
    #[inline(always)]
    pub(crate) fn value_real(&self) -> Real {
        // SAFETY: one level past the leaf macros: `value_vec4_raw` projects the
        // view's own live `value_vec4` member, initialized by the zero-fill in
        // `ufbxi_fetch_maps`, and `.x` is the first `ufbx_real` of it.
        unsafe { (*self.value_vec4_raw()).x }
    }
    #[inline(always)]
    pub(crate) fn set_value_real(&self, value: Real) {
        // SAFETY: as `value_real`; `value_vec4_raw` exists on `Mut` views only,
        // so the projection carries write-capable provenance.
        unsafe { (*self.value_vec4_raw()).x = value }
    }
}

// ufbx.c:20124-20216 `ufbxi_fetch_maps`
#[inline(never)]
pub(crate) fn fetch_maps(scene_view: &SceneView, material_view: &MaterialView) {
    ufbxi_ignore!(scene_view);

    // C: `ufbx_shader *shader = material->shader;` — the nullable `Ref` field
    // minted once as the view `ufbxi_fetch_mapping_maps` takes.
    // SAFETY: the material's `shader` reference is a live scene element,
    // unwritten during the map fetches below, so it mints a `Const` view.
    let shader: Option<&View<Shader, Const>> = material_view
        .shader()
        .map(|shader| unsafe { View::<Shader, Const>::from_ptr(shader.ptr()) });
    ufbx_assert!((material_view.shader_type() as u32) < SHADER_TYPE_COUNT as u32);

    // SAFETY: each `*_raw()` accessor projects the material's own aggregate
    // member, and every zero-fill spans exactly that member's own `size_of`
    // bytes.
    unsafe {
        ptr::write_bytes(
            material_view.fbx_raw() as *mut u8,
            0,
            size_of::<MaterialFbxMaps>(),
        );
        ptr::write_bytes(
            material_view.pbr_raw() as *mut u8,
            0,
            size_of::<MaterialPbrMaps>(),
        );
        ptr::write_bytes(
            material_view.features_raw() as *mut u8,
            0,
            size_of::<MaterialFeatures>(),
        );
    }

    // C-parity: `ufbx_material_fbx_maps` / `_pbr_maps` / `ufbx_material_features`
    // are unions of a named-member struct and a flat array; the generator keeps
    // only the named struct, so the array is recovered by casting the whole
    // aggregate (identical layout, PORTING.md "Unions"). These bases are what
    // the `ufbxi_fetch_mapping_maps` calls below take as C arrays.
    let fbx_maps: *mut MaterialMap = material_view.fbx_raw() as *mut MaterialMap;
    let pbr_maps: *mut MaterialMap = material_view.pbr_raw() as *mut MaterialMap;
    let feature_infos: *mut MaterialFeatureInfo =
        material_view.features_raw() as *mut MaterialFeatureInfo;

    let mut base_mapping: *const ShaderMapping = BASE_FBX_MAPPING.as_ptr();
    let mut num_base_mapping: usize = BASE_FBX_MAPPING.len();

    if scene_view.metadata_view().file_format() == FileFormat::Obj
        || scene_view.metadata_view().file_format() == FileFormat::Mtl
    {
        base_mapping = OBJ_FBX_MAPPING.as_ptr();
        num_base_mapping = OBJ_FBX_MAPPING.len();
    }

    // SAFETY: `fbx_maps` addresses the material's `fbx` member viewed as
    // `MATERIAL_FBX_MAP_COUNT` maps, and `base_mapping`/`num_base_mapping`
    // describe one of the two static mapping tables.
    unsafe {
        fetch_mapping_maps(
            material_view,
            fbx_maps,
            ptr::null_mut(),
            None,
            base_mapping,
            num_base_mapping,
            EMPTY_STRING.0,
            EMPTY_STRING.0,
            EMPTY_STRING.0,
            MAPPING_FETCH_VALUE | MAPPING_FETCH_TEXTURE,
        )
    };

    // The assert above bounds `shader_type` by `SHADER_TYPE_COUNT`, the length
    // of `SHADER_PBR_MAPPINGS`.
    let list: ShaderMappingList = SHADER_PBR_MAPPINGS[material_view.shader_type() as usize];

    for i in 0..MATERIAL_FEATURE_COUNT {
        if (list.default_features & (1u32 << i)) != 0 {
            material_view.feature_at(i as usize).set_enabled(true);
        }
    }

    let mut prefix: String = EMPTY_STRING.0;
    if shader.is_none() {
        prefix = material_view.shader_prop_prefix();
    }

    if list.texture_prefix.length > 0 || list.texture_suffix.length > 0 {
        // SAFETY: `pbr_maps` addresses the material's `pbr` member viewed as
        // `MATERIAL_PBR_MAP_COUNT` maps, and `list.data`/`list.count` describe
        // the static PBR mapping table selected above.
        unsafe {
            fetch_mapping_maps(
                material_view,
                pbr_maps,
                ptr::null_mut(),
                shader,
                list.data,
                list.count,
                prefix,
                list.texture_prefix,
                list.texture_suffix,
                MAPPING_FETCH_TEXTURE,
            )
        };
    }

    // SAFETY: as the previous `fetch_mapping_maps` call.
    unsafe {
        fetch_mapping_maps(
            material_view,
            pbr_maps,
            ptr::null_mut(),
            shader,
            list.data,
            list.count,
            prefix,
            EMPTY_STRING.0,
            EMPTY_STRING.0,
            MAPPING_FETCH_VALUE | MAPPING_FETCH_TEXTURE,
        )
    };

    if list.texture_enabled_prefix.length > 0 || list.texture_enabled_suffix.length > 0 {
        // SAFETY: as the previous `fetch_mapping_maps` call.
        unsafe {
            fetch_mapping_maps(
                material_view,
                pbr_maps,
                ptr::null_mut(),
                shader,
                list.data,
                list.count,
                prefix,
                list.texture_enabled_prefix,
                list.texture_enabled_suffix,
                MAPPING_FETCH_TEXTURE_ENABLED,
            )
        };
    }

    // SAFETY: `feature_infos` addresses the material's `features` member viewed
    // as `MATERIAL_FEATURE_COUNT` infos, and `list.features` /
    // `list.feature_count` describe the static feature-mapping table.
    unsafe {
        fetch_mapping_maps(
            material_view,
            ptr::null_mut(),
            feature_infos,
            shader,
            list.features,
            list.feature_count,
            prefix,
            EMPTY_STRING.0,
            EMPTY_STRING.0,
            MAPPING_FETCH_FEATURE,
        )
    };

    // The `ufbxi_update_factor` pairs are named members of the two aggregates,
    // each projected in place out of the material.
    let fbx: &View<MaterialFbxMaps> = material_view.fbx_view();
    update_factor(fbx.diffuse_factor_view(), fbx.diffuse_color_view());
    update_factor(fbx.specular_factor_view(), fbx.specular_color_view());
    update_factor(fbx.reflection_factor_view(), fbx.reflection_color_view());
    update_factor(
        fbx.transparency_factor_view(),
        fbx.transparency_color_view(),
    );
    update_factor(fbx.emission_factor_view(), fbx.emission_color_view());
    update_factor(fbx.ambient_factor_view(), fbx.ambient_color_view());

    let pbr: &View<MaterialPbrMaps> = material_view.pbr_view();
    update_factor(pbr.base_factor_view(), pbr.base_color_view());
    update_factor(pbr.specular_factor_view(), pbr.specular_color_view());
    update_factor(pbr.emission_factor_view(), pbr.emission_color_view());
    update_factor(pbr.sheen_factor_view(), pbr.sheen_color_view());
    update_factor(pbr.thin_film_factor_view(), pbr.thin_film_thickness_view());
    update_factor(
        pbr.transmission_factor_view(),
        pbr.transmission_color_view(),
    );

    // Patch transmission roughness if only extra roughness is defined
    if !pbr.transmission_roughness_view().has_value()
        && pbr.roughness_view().has_value()
        && pbr.transmission_extra_roughness_view().has_value()
    {
        pbr.transmission_roughness_view().set_value_real(
            pbr.roughness_view().value_real()
                + pbr.transmission_extra_roughness_view().value_real(),
        );
    }

    // Map roughness to glossiness and vice versa
    // C: `ufbxi_for(const ufbxi_glossiness_remap, remap, ufbxi_glossiness_remaps, ufbxi_arraycount(ufbxi_glossiness_remaps))`
    for remap in &GLOSSINESS_REMAPS {
        // `roughness_map`/`glossiness_map` are `ufbx_material_pbr_map`
        // discriminants and `feature` is a `ufbx_material_feature` one, so each
        // bounds check inside the indexed accessors holds by construction.
        let roughness: &MaterialMapView = material_view.pbr_map_at(remap.roughness_map as usize);
        let glossiness: &MaterialMapView = material_view.pbr_map_at(remap.glossiness_map as usize);
        if material_view.feature_at(remap.feature as usize).enabled() {
            // C: `*glossiness = *roughness;` — struct assignment is a memcpy
            // (PORTING.md checklist #15); `ufbx_material_map` is not `Copy` in
            // the generated bindings, so the copy is spelled out.
            // SAFETY: `roughness` and `glossiness` view distinct entries of the
            // material's `pbr` map array (the remap table never pairs a map with
            // itself), both initialized by the zero-fill and the fetches above,
            // and the fill spans exactly one `MaterialMap`.
            unsafe {
                ptr::copy_nonoverlapping(roughness.as_ptr(), glossiness.get(), 1);
                ptr::write_bytes(roughness.get() as *mut u8, 0, size_of::<MaterialMap>());
            }
            if glossiness.has_value() {
                roughness.set_value_real(1.0f32 as Real - glossiness.value_real());
            }
        } else if roughness.has_value() {
            glossiness.set_value_real(1.0f32 as Real - roughness.value_real());
        }
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
// `prop: &[u8]` carries C's `const char *prop` — the interned, NUL-terminated
// property name the caller minted (see `find_dst_connections`). C's `ufbx_node
// *node` is non-NULL at both call sites (each resolves a connection endpoint it
// already checked is a `UFBX_ELEMENT_NODE`), so it maps to a plain view.
#[inline(never)]
pub(crate) fn add_constraint_prop(
    uc: &Context,
    constraint: &ConstraintView,
    node: &NodeView,
    prop: &[u8],
) -> Result<(), Fail> {
    // C: `ufbxi_for(const ufbxi_constraint_prop, cprop, ufbxi_constraint_props, ufbxi_arraycount(ufbxi_constraint_props))`
    let mut cprop: *const ConstraintProp = CONSTRAINT_PROPS.as_ptr();
    let cprop_end: *const ConstraintProp = CONSTRAINT_PROPS
        .as_ptr()
        .wrapping_add(CONSTRAINT_PROPS.len());
    while cprop != cprop_end {
        // C: `strcmp(cprop->name, prop)` over two NUL-terminated strings; the
        // table name is taken as the span up to its NUL and `prop` is
        // NUL-terminated at its length, so `c_strcmp` walks the same bytes.
        // SAFETY: `cprop != cprop_end`, so it addresses a live entry of the
        // static `CONSTRAINT_PROPS` table whose `name` is a NUL-terminated
        // string literal: `strlen` bytes from it are readable.
        let name: &[u8] = unsafe { slice_from_ptr((*cprop).name, strlen((*cprop).name)) };
        if c_strcmp(name, prop) != 0 {
            // SAFETY: `cprop != cprop_end`, so the advance lands at or before the
            // one-past-the-end pointer of `CONSTRAINT_PROPS`.
            cprop = unsafe { cprop.add(1) };
            continue;
        }
        // SAFETY (this group): `cprop` addresses a live table entry (see above),
        // and `node.get()` hands back the viewed, live `ufbx_node` — the
        // liveness `opt_ref` requires of the reference it stores.
        match unsafe { (*cprop).type_ } {
            ConstraintPropType::Node => constraint.set_node(unsafe { opt_ref(node.get()) }),
            ConstraintPropType::IkEffector => {
                constraint.set_ik_effector(unsafe { opt_ref(node.get()) })
            }
            ConstraintPropType::IkEndNode => {
                constraint.set_ik_end_node(unsafe { opt_ref(node.get()) })
            }
            ConstraintPropType::AimUp => constraint.set_aim_up_node(unsafe { opt_ref(node.get()) }),
            ConstraintPropType::Target => {
                let target: *mut ConstraintTarget = uc.tmp_stack_view().push_zero(1);
                ufbxi_check!(uc, !target.is_null(), "target");
                // `ufbx_constraint_target.node` is a non-nullable
                // `ufbx_node*`; the sole caller (ufbx.c:22576/22580) passes a
                // connection endpoint it already checked is a
                // `UFBX_ELEMENT_NODE`, so `node` is never NULL here.
                // SAFETY: `target` is the fresh non-null one-element push above,
                // and `node.get()` addresses the live, non-NULL `ufbx_node` the
                // caller resolved from that connection.
                unsafe { (*target).node = Ref::from_ptr(node.get()) };
                // SAFETY: `target` is the fresh non-null push above.
                unsafe {
                    (*target).weight = 1.0f32 as Real;
                    (*target).transform = IDENTITY_TRANSFORM;
                }
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
        // SAFETY: `cprop != cprop_end`, so the advance lands at or before the
        // one-past-the-end pointer of `CONSTRAINT_PROPS`.
        cprop = unsafe { cprop.add(1) };
    }

    Ok(())
}

// ufbx.c:20268-20312 `ufbxi_finalize_nurbs_basis`
#[inline(never)]
pub(crate) fn finalize_nurbs_basis(uc: &Context, basis: &View<NurbsBasis>) -> Result<(), Fail> {
    if basis.topology() == NurbsTopology::Closed {
        basis.set_num_wrap_control_points(1);
    } else if basis.topology() == NurbsTopology::Periodic {
        basis.set_num_wrap_control_points(basis.order().wrapping_sub(1) as usize);
    } else {
        basis.set_num_wrap_control_points(0);
    }

    if basis.order() > 1 {
        // `order > 1`, so the subtraction does not underflow.
        let degree: usize = (basis.order() - 1) as usize;
        // C: `ufbx_real_list knots = basis->knot_vector;` — the by-value copy
        // of the viewed basis' own knot-vector header.
        let knots: List<Real> = basis.knot_vector();
        if knots.count > 2 * degree {
            // SAFETY: `knots` is the basis' own knot-vector span of `count`
            // initialized reals (the viewed list invariant) and
            // `degree < count`, so the first read is in bounds;
            // `count > 2 * degree` makes `count - degree - 1 < count`.
            unsafe {
                basis.set_t_min(*knots.data.add(degree));
                basis.set_t_max(*knots.data.add(knots.count - degree - 1));
            }

            let max_spans: usize = knots.count - 2 * degree;
            let spans: *mut Real = uc.result_view().push(max_spans);
            ufbxi_check!(uc, !spans.is_null(), "spans");

            let mut prev: Real = -math::INFINITY as Real;
            let mut num_spans: usize = 0;
            for i in 0..max_spans {
                // SAFETY: `i < max_spans = count - 2 * degree`, so
                // `degree + i < count` bounds the read inside the knot span.
                let t: Real = unsafe { *knots.data.add(degree + i) };
                if t != prev {
                    // SAFETY: `num_spans <= i < max_spans`, the length of the
                    // fresh non-null `spans` push above.
                    unsafe { *spans.add(num_spans) = t };
                    num_spans += 1;
                    prev = t;
                }
            }

            // `spans` is the push above, holding `num_spans` initialized reals.
            basis.spans_view().set_data(spans);
            basis.spans_view().set_count(num_spans);
            basis.set_valid(true);
            for i in 1..knots.count {
                // SAFETY: `1 <= i < count` bounds both reads inside the knot
                // span of `count` initialized reals.
                if unsafe { *knots.data.add(i - 1) > *knots.data.add(i) } {
                    basis.set_valid(false);
                    break;
                }
            }
        }
    }

    Ok(())
}

// ufbx.c:20314-20362 `ufbxi_finalize_lod_group`
#[inline(never)]
pub(crate) fn finalize_lod_group(uc: &Context, lod_view: &LodGroupView) -> Result<(), Fail> {
    // `lod_view` is the uc-anchored dispatch handle (minted in `finalize_scene`
    // from the arena `lod_groups` run); the raw `lod` is used only for the field
    // writes, while every property lookup goes through `lod_view.props_view()`
    // (<= uc), collapsing the per-call free-lifetime `PropsView` bridges.
    let lod: *mut LodGroup = lod_view.get();
    let mut num_levels: usize = 0;
    // SAFETY: reads the LOD group's own instance run; C subscripts entry `0`
    // (never `i`), which the loop guard proves present, and `ref_ptr` resolves
    // the always-set node reference.
    unsafe {
        for _i in 0..(*lod).element.instances.count {
            // C-parity: the subscript really is `instances.data[0]` (not `[i]`) —
            // ufbx.c:20318.
            num_levels = max_sz(
                num_levels,
                (*ref_ptr((*lod).element.instances.data)).children.count,
            );
        }
    }

    // C: `char prop_name[64];` — uninitialized local (no upstream
    // `// ufbxi_uninit` marker at ufbx.c:20321); `ufbxi_snprintf` writes it.
    let mut prop_name_storage = MaybeUninit::<[u8; 64]>::uninit();
    let prop_name: *mut u8 = prop_name_storage.as_mut_ptr() as *mut u8;
    let mut i: usize = 0;
    // SAFETY: `prop_name` is the local 64-byte buffer, NUL-terminated by
    // `ufbxi_snprintf` with the matching `len` before each lookup of the LOD
    // group's own props.
    unsafe {
        loop {
            let len: i32 =
                ufbxi_snprintf!(prop_name, size_of::<[u8; 64]>(), "Thresholds|Level%zu", i);
            let prop: *mut Prop = find_prop_len(
                lod_view.props_view(),
                slice_from_ptr(prop_name, len as usize),
            )
            .map_or(ptr::null_mut(), PropView::get);
            if prop.is_null() {
                break;
            }
            num_levels = max_sz(num_levels, i + 1);
            i += 1;
        }
    }

    let levels: *mut LodLevel = uc.result_view().push_zero(num_levels);
    ufbxi_check!(uc, !levels.is_null(), "levels");

    // SAFETY: `lod` is the LOD-group view's own storage and every lookup reads
    // that same view's props with a NUL-terminated literal; `levels` is the fresh
    // non-null push above.
    unsafe {
        (*lod).relative_distances =
            api_find_bool_len(lod_view.props_view(), b"ThresholdsUsedAsPercentage", false);
        (*lod).ignore_parent_transform =
            !api_find_bool_len(lod_view.props_view(), b"WorldSpace", true);

        (*lod).use_distance_limit =
            api_find_bool_len(lod_view.props_view(), b"MinMaxDistance", false);
        (*lod).distance_limit_min =
            api_find_real_len(lod_view.props_view(), b"MinDistance", -100.0 as Real);
        (*lod).distance_limit_max =
            api_find_real_len(lod_view.props_view(), b"MaxDistance", 100.0 as Real);

        (*lod).lod_levels.data = levels;
        (*lod).lod_levels.count = num_levels;
    }

    // SAFETY: `levels` is the fresh non-null `num_levels`-element push above, so
    // every `add(i)` is in bounds; `prop_name` is the local 64-byte buffer that
    // `ufbxi_snprintf` NUL-terminates with the matching `len`; the transmute is
    // guarded by the explicit `[0, 2]` check.
    unsafe {
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
                    lod_view.props_view(),
                    slice_from_ptr(prop_name, len as usize),
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
                let display: i64 = api_find_int_len(
                    lod_view.props_view(),
                    slice_from_ptr(prop_name, len as usize),
                    0,
                );
                if display >= 0 && display <= 2 {
                    // C: `(ufbx_lod_display)display` — guarded to [0, 2], every
                    // value of which is a valid `ufbx_lod_display`.
                    (*level).display = core::mem::transmute::<u32, LodDisplay>(display as u32);
                }
            }
        }
    }

    Ok(())
}

// ufbx.c:20363-20403 `ufbxi_generate_normals`
#[inline(never)]
pub(crate) unsafe fn generate_normals(uc: &Context, mesh: &View<Mesh>) -> Result<(), Fail> {
    let num_indices: usize = mesh.num_indices();

    mesh.set_generated_normals(true);

    let topo: *mut TopoEdge = uc.tmp_stack_view().push::<TopoEdge>(num_indices);
    ufbxi_check!(uc, !topo.is_null(), "topo");

    let normal_indices: *mut u32 = uc.result_view().push::<u32>(num_indices);
    ufbxi_check!(uc, !normal_indices.is_null(), "normal_indices");

    // SAFETY: `mesh.as_ptr()` reads the viewed mesh and `topo` is the fresh
    // non-null `num_indices`-edge push above, which is the output run
    // `compute_topology` fills.
    unsafe { compute_topology(mesh.as_ptr(), topo, num_indices) };
    // SAFETY: `mesh.as_ptr()` reads the viewed mesh, `topo` holds the
    // `num_indices` edges just computed, and `normal_indices` is the fresh
    // non-null `num_indices`-element push that receives the mapping.
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
        mesh.vertex_normal().set_unique_per_vertex(true);
    }

    let mut normal_data: *mut Vec3 = uc.result_view().push::<Vec3>(num_normals + 1);
    ufbxi_check!(uc, !normal_data.is_null(), "normal_data");

    // C: `normal_data[0] = ufbx_zero_vec3; normal_data++;`
    // SAFETY: `normal_data` is the fresh non-null `num_normals + 1`-element push
    // above, so element `0` exists.
    unsafe { *normal_data = ZERO_VEC3 };
    // SAFETY: the push holds `num_normals + 1 >= 1` elements, so advancing by one
    // stays inside it.
    normal_data = unsafe { normal_data.add(1) };

    // SAFETY: `mesh.as_ptr()` reads the viewed mesh and `vertex_position_ptr()`
    // addresses its own vertex-position attribute; `normal_indices` holds
    // `num_indices` mapping entries and `normal_data` the `num_normals`
    // accumulator slots the push reserved past the zero element.
    unsafe {
        compute_normals(
            mesh.as_ptr(),
            mesh.vertex_position_ptr(),
            normal_indices,
            num_indices,
            normal_data,
            num_normals,
        )
    };

    // The two runs stored here are the result-arena pushes above, which outlive
    // the scene.
    let vertex_normal = mesh.vertex_normal();
    vertex_normal.set_exists(true);
    vertex_normal
        .values_view()
        .set_data(normal_data as *const Vec3);
    vertex_normal.values_view().set_count(num_normals);
    vertex_normal
        .indices_view()
        .set_data(normal_indices as *const u32);
    vertex_normal.indices_view().set_count(num_indices);
    vertex_normal.set_value_reals(3);

    // C: `mesh->skinned_normal = mesh->vertex_normal;` — struct assignment
    // (memcpy); `VertexVec3` is not `Copy` in the generated bindings, so the
    // copy is spelled as a byte-identical `copy_nonoverlapping`.
    // SAFETY: both projections address the viewed mesh's own distinct
    // `vertex_normal` / `skinned_normal` fields; `vertex_normal` is initialized
    // (zeroed at element push, its list/exists fields rewritten just above).
    unsafe { ptr::copy_nonoverlapping(mesh.vertex_normal_ptr(), mesh.skinned_normal_raw(), 1) };

    // SAFETY: `tmp_stack` is `uc`'s own live buffer and the `num_indices`
    // `TopoEdge`s pushed above are still its topmost entries.
    unsafe { pop::<TopoEdge>(uc.tmp_stack_mut_ptr(), num_indices, ptr::null_mut()) };

    Ok(())
}

// ufbx.c:20405-20427 `ufbxi_push_prop_prefix`
///
/// # Safety
/// `prefix` must be a valid `ufbx_string`: `data` readable for `length` bytes,
/// unmoved and unwritten for the call.
#[inline(never)]
pub(crate) unsafe fn push_prop_prefix(
    uc: &Context,
    dst: &StringView,
    mut prefix: String,
) -> Result<(), Fail> {
    let mut stack_size: usize = 0;
    // SAFETY: `prefix` is a `ufbx_string` whose `data` addresses `length`
    // readable bytes (fn contract); the `length > 0` guard short-circuits ahead
    // of the read, so `length - 1` is an in-bounds index.
    if prefix.length > 0 && unsafe { *prefix.data.add(prefix.length - 1) } != b'|' {
        stack_size = prefix.length.wrapping_add(1);
        let copy: *mut u8 = uc.tmp_stack_view().push(stack_size);
        ufbxi_check!(uc, !copy.is_null(), "copy");
        // SAFETY: `copy` is the fresh non-null `prefix.length + 1`-byte push,
        // disjoint from the caller's `prefix.data` span of `length` bytes.
        unsafe { ptr::copy_nonoverlapping(prefix.data, copy, prefix.length) };
        // SAFETY: the push holds `prefix.length + 1` bytes, so index
        // `prefix.length` is its last one.
        unsafe { *copy.add(prefix.length) = b'|' };

        prefix.data = copy;
        prefix.length = prefix.length.wrapping_add(1);
    }

    // SAFETY: `string_pool_mut_ptr` is `uc`'s own live string pool, and `prefix`
    // is a local whose `data`/`length` span is either the caller's or the
    // `tmp_stack` copy above — both readable for `length` bytes.
    unsafe { sp::push_string_place_str(uc.string_pool_mut_ptr(), &raw mut prefix, false)? };
    // C: `*dst = prefix;` — the `ufbx_string` assignment (memcpy of the two
    // POD members) is spelled as the viewed slot's two leaf writes.
    dst.set_data(prefix.data);
    dst.set_length(prefix.length);

    if stack_size > 0 {
        // SAFETY: `stack_size > 0` only when the push above ran, so those bytes
        // are still `tmp_stack`'s topmost entries.
        unsafe { pop::<u8>(uc.tmp_stack_mut_ptr(), stack_size, ptr::null_mut()) };
    }

    Ok(())
}

// ufbx.c:20429-20478 `ufbxi_shader_texture_find_prefix`
#[inline(never)]
pub(crate) fn shader_texture_find_prefix(
    uc: &Context,
    texture: &TextureView,
    shader: &ShaderTextureView,
) -> Result<(), Fail> {
    // C: `ufbx_string suffixes[3];` — uninitialized local (no upstream
    // `// ufbxi_uninit` marker at ufbx.c:20431); only the first
    // `num_suffixes` entries are ever written, and only those are read.
    let mut suffixes_storage = MaybeUninit::<[String; 3]>::uninit();
    let suffixes: *mut String = suffixes_storage.as_mut_ptr() as *mut String;
    let mut num_suffixes: usize = 0;

    // SAFETY: `num_suffixes` is `0` here, in bounds of the 3-entry local array;
    // the literal is NUL-terminated, which is what `str_c` measures.
    unsafe { *suffixes.add(num_suffixes) = sp::str_c(b" Parameters/Connections\0".as_ptr()) };
    num_suffixes += 1;
    if shader.shader_name().length > 0 {
        // SAFETY: `num_suffixes` is `1` here, in bounds of the 3-entry local
        // array.
        unsafe { *suffixes.add(num_suffixes) = shader.shader_name() };
        num_suffixes += 1;
    }
    // SAFETY: `num_suffixes` is at most `2` here, in bounds of the 3-entry local
    // array; the literal is NUL-terminated, which is what `str_c` measures.
    unsafe { *suffixes.add(num_suffixes) = sp::str_c(b"3dsMax|parameters\0".as_ptr()) };
    num_suffixes += 1;

    // C: `ufbx_assert(num_suffixes <= ufbxi_arraycount(suffixes));`
    ufbx_assert!(num_suffixes <= 3);

    // C: `ufbxi_for(ufbx_string, p_suffix, suffixes, num_suffixes)`
    let mut p_suffix: *mut String = suffixes;
    let p_suffix_end: *mut String = add_ptr(p_suffix, num_suffixes);
    while p_suffix != p_suffix_end {
        // SAFETY: `p_suffix != p_suffix_end`, so it addresses one of the
        // `num_suffixes` entries written above.
        let suffix: String = unsafe { *p_suffix };

        // C: `ufbxi_for_list(ufbx_prop, prop, texture->props.props)`
        let texture_props: &PropsView = texture.props_view();
        // SAFETY: the viewed table's `props` `data`/`count` pair describes one
        // contiguous arena run of initialized, arena-owned `ufbx_prop`s (the
        // viewed-list invariant), live and unmoved for the borrow.
        let props = unsafe {
            SliceViewIter::<Prop>::from_raw_parts(
                texture_props.props_data(),
                texture_props.props_count(),
            )
        };
        for prop in props {
            if prop.type_() != PropType::Compound {
                continue;
            }
            // SAFETY: the entry's `name` and `suffix` are both `ufbx_string`
            // spans, which is what `ends_with` compares and what
            // `push_prop_prefix` copies.
            unsafe {
                if sp::ends_with(prop.name(), suffix) {
                    push_prop_prefix(uc, shader.prop_prefix_view(), prop.name())?;
                    return Ok(());
                }
            }
        }
        // SAFETY: `p_suffix != p_suffix_end`, so the advance lands at or before
        // the one-past-the-end pointer of the written suffix prefix.
        p_suffix = unsafe { p_suffix.add(1) };
    }

    // Pre-7000 files don't have explicit Compound properties, so let's look for
    // any property that has the suffix before the last `|` ...
    let mut p_suffix: *mut String = suffixes;
    let p_suffix_end: *mut String = add_ptr(p_suffix, num_suffixes);
    while p_suffix != p_suffix_end {
        // SAFETY: `p_suffix != p_suffix_end`, so it addresses one of the
        // `num_suffixes` entries written above.
        let suffix: String = unsafe { *p_suffix };

        // C: `ufbxi_for_list(ufbx_prop, prop, texture->props.props)`
        let texture_props: &PropsView = texture.props_view();
        // SAFETY: the viewed table's `props` `data`/`count` pair describes one
        // contiguous arena run of initialized, arena-owned `ufbx_prop`s (the
        // viewed-list invariant), live and unmoved for the borrow.
        let props = unsafe {
            SliceViewIter::<Prop>::from_raw_parts(
                texture_props.props_data(),
                texture_props.props_count(),
            )
        };
        for prop in props {
            let mut name: String = prop.name();
            while name.length > 0 {
                // SAFETY: `name` is the prop's interned span of `length`
                // readable bytes, and the loop guard makes `length - 1` an
                // in-bounds index; the loop only ever shrinks `length`.
                if unsafe { *name.data.add(name.length - 1) } == b'|' {
                    break;
                }
                name.length -= 1;
            }
            if name.length <= 1 {
                continue;
            }
            name.length -= 1;

            // SAFETY: `name` is a prefix of the prop's interned span and
            // `suffix` a live `ufbx_string`, which is what `ends_with` compares
            // and what `push_prop_prefix` copies.
            unsafe {
                if sp::ends_with(name, suffix) {
                    push_prop_prefix(uc, shader.prop_prefix_view(), name)?;
                    return Ok(());
                }
            }
        }
        // SAFETY: `p_suffix != p_suffix_end`, so the advance lands at or before
        // the one-past-the-end pointer of the written suffix prefix.
        p_suffix = unsafe { p_suffix.add(1) };
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
pub(crate) fn update_shader_texture(texture_view: &TextureView, shader_view: &ShaderTextureView) {
    // C: `ufbxi_for_list(ufbx_shader_texture_input, input, shader->inputs)`
    let inputs: &ListView<ShaderTextureInput> = shader_view.inputs_view();
    // SAFETY: `data`/`count` describe the shader's own input run — one
    // contiguous arena allocation of live, initialized
    // `ufbx_shader_texture_input`, arena-stable for the walk.
    let input_iter = unsafe {
        SliceViewIter::<ShaderTextureInput>::from_raw_parts(
            inputs.data() as *mut ShaderTextureInput,
            inputs.count(),
        )
    };
    for input in input_iter {
        // C: `ufbx_prop *prop = input->prop;`
        if let Some(prop) = input.prop() {
            // SAFETY: the `prop` field holds a live `ufbx_prop` of this
            // texture's own prop list — an arena element reached through a
            // pointer stored in arena memory, which carries the write-capable
            // provenance the mint asks for.
            let prop: &PropView = unsafe { PropView::from_ptr(prop.ptr()) };
            let found: Option<&PropView> =
                find_prop_len(texture_view.props_view(), prop.name_view().bytes());
            // SAFETY: `PropView::get` yields the matched prop's own arena
            // address, or null on a miss, which is what `opt_ref` wraps.
            input.set_prop(unsafe { opt_ref(found.map_or(ptr::null_mut(), PropView::get)) });
            // C-parity: the re-lookup keys on the name of a prop that came from
            // this same prop list, so it always resolves (ufbx.c:20502-20506
            // dereferences it unconditionally).
            // SAFETY: the lookup resolves (see above), so `found` holds the
            // matched live `ufbx_prop`.
            let prop: &PropView = unsafe { found.unwrap_unchecked() };
            input.set_value_vec4(prop.value_vec4());
            input.set_value_int(prop.value_int());
            input.set_value_str(prop.value_str());
            input.set_value_blob(prop.value_blob());
            // SAFETY: `element_ptr()` addresses the texture's own live element
            // header and the `prop` field was just set to that live prop, which
            // is `get_prop_element`'s raw-pointer contract.
            let tex: *mut Texture = unsafe {
                get_prop_element(
                    texture_view.element_ptr(),
                    input.prop().map_or(ptr::null(), |p| p.ptr() as *const Prop),
                    ElementType::Texture,
                ) as *mut Texture
            };
            // SAFETY: the lookup returns null or the live element it found,
            // here the `ufbx_texture` its element type pins it to.
            input.set_texture(unsafe { opt_ref(tex) });
        }

        // C: `prop = input->texture_prop;`
        if let Some(prop) = input.texture_prop() {
            // SAFETY: the `texture_prop` field holds a live `ufbx_prop` of this
            // texture's own prop list (see the `prop` field above).
            let prop: &PropView = unsafe { PropView::from_ptr(prop.ptr()) };
            let found: Option<&PropView> =
                find_prop_len(texture_view.props_view(), prop.name_view().bytes());
            let prop: *mut Prop = found.map_or(ptr::null_mut(), PropView::get);
            // SAFETY: `prop` is null or the matched prop's own arena address,
            // which is what `opt_ref` wraps.
            input.set_texture_prop(unsafe { opt_ref(prop) });
            // SAFETY: `element_ptr()` addresses the texture's own live element
            // header; `prop` is null or a live prop of that element's list.
            let tex: *mut Texture = unsafe {
                get_prop_element(texture_view.element_ptr(), prop, ElementType::Texture)
                    as *mut Texture
            };
            if !tex.is_null() {
                // SAFETY: `tex` is non-null (checked) and a live `ufbx_texture`.
                input.set_texture(unsafe { opt_ref(tex) });
            }
        }

        input.set_texture_enabled(input.texture().is_some());
        // C: `prop = input->texture_enabled_prop;`
        if let Some(prop) = input.texture_enabled_prop() {
            // SAFETY: the `texture_enabled_prop` field holds a live `ufbx_prop`
            // of this texture's own prop list (see the `prop` field above).
            let prop: &PropView = unsafe { PropView::from_ptr(prop.ptr()) };
            let found: Option<&PropView> =
                find_prop_len(texture_view.props_view(), prop.name_view().bytes());
            // SAFETY: `PropView::get` yields the matched prop's own arena
            // address, or null on a miss, which is what `opt_ref` wraps.
            input.set_texture_enabled_prop(unsafe {
                opt_ref(found.map_or(ptr::null_mut(), PropView::get))
            });
            // C-parity: the re-lookup keys on the name of a prop from this same
            // list, so it always resolves (ufbx.c:20519-20520 dereferences it
            // unconditionally).
            // SAFETY: the lookup resolves (see above), so `found` holds the
            // matched live `ufbx_prop`.
            let prop: &PropView = unsafe { found.unwrap_unchecked() };
            input.set_texture_enabled(prop.value_int() != 0);
        }
    }

    if shader_view.type_() == ShaderTextureType::SelectOutput {
        let map: Option<&View<ShaderTextureInput>> =
            find_shader_texture_input_len(shader_view, b"sourceMap");
        let index: Option<&View<ShaderTextureInput>> =
            find_shader_texture_input_len(shader_view, b"outputChannelIndex");
        if let Some(index) = index {
            shader_view.set_main_texture_output_index(index.value_int());
        }
        if let Some(map) = map {
            shader_view.set_main_texture(map.texture());
            map.set_texture_output_index(shader_view.main_texture_output_index());
        }
    }
}

// ufbx.h:2772 `UFBX_ENUM_TYPE(ufbx_shader_texture_type, UFBX_SHADER_TEXTURE_TYPE, UFBX_SHADER_TEXTURE_OSL);`
// expanding via ufbx.h:235-236 to `enum { UFBX_SHADER_TEXTURE_TYPE_COUNT = UFBX_SHADER_TEXTURE_OSL + 1 }`.
pub(crate) const SHADER_TEXTURE_TYPE_COUNT: u32 = ShaderTextureType::Osl as u32 + 1;

// ufbx.c:20537-20690 `ufbxi_finalize_shader_texture`
#[inline(never)]
pub(crate) fn finalize_shader_texture<'a>(
    uc: &'a Context,
    texture_view: &'a TextureView,
) -> Result<(), Fail> {
    let texture: *mut Texture = texture_view.get();
    let (classid_a, classid_b): (u32, u32) = (
        api_find_int_len(texture_view.props_view(), b"3dsMax|ClassIDa", 0) as u64 as u32,
        api_find_int_len(texture_view.props_view(), b"3dsMax|ClassIDb", 0) as u64 as u32,
    );
    let classid: u64 = (classid_a as u64) << 32 | classid_b as u64;

    let max_texture: String = find_string_len(
        texture_view.props_view(),
        b"3dsMax|MaxTexture",
        EMPTY_STRING.0,
    );

    // Check first if the texture looks like it could be a shader.
    // C: `ufbx_shader_texture_type type = (ufbx_shader_texture_type)UFBX_SHADER_TEXTURE_TYPE_COUNT;`
    // — the sentinel is out of the enum's range, so it is carried as the raw
    // `uint32_t` C stores and only transmuted once the range check passed.
    let mut type_: u32 = SHADER_TEXTURE_TYPE_COUNT;

    // SAFETY: `max_texture` is an interned (NUL-terminated) string compared
    // against literals, and `texture` is the texture view's own storage.
    unsafe {
        if strcmp(max_texture.data, b"MULTIOUTPUT_TO_OSLMap\0".as_ptr()) == 0
            || classid == 0x896ef2fc44bd743f
        {
            type_ = ShaderTextureType::SelectOutput as u32;
        } else if strcmp(max_texture.data, b"OSLMap\0".as_ptr()) == 0
            || classid == 0x7f9a7b9d6fcdf00d
        {
            type_ = ShaderTextureType::Osl as u32;
        } else if (*texture).type_ == TextureType::File
            && (*texture).relative_filename.length == 0
            && (*texture).absolute_filename.length == 0
            && opt_ptr(&raw const (*texture).video).is_null()
        {
            type_ = ShaderTextureType::Unknown as u32;
        }
    }

    if type_ == SHADER_TEXTURE_TYPE_COUNT {
        return Ok(());
    }

    let shader: *mut ShaderTexture = uc.result_view().push_zero(1);
    ufbxi_check!(uc, !shader.is_null(), "shader");

    // SAFETY: `shader` is the fresh non-null push above; `type_` passed the
    // range check above, so the transmute is of a valid discriminant; the prop
    // lookups use NUL-terminated static names and every `prop` is null-checked.
    unsafe {
        (*shader).type_ = core::mem::transmute::<u32, ShaderTextureType>(type_);

        // C: `static const char *const name_props[] = { "3dsMax|params|OSLShaderName" };`
        static NAME_PROPS: [&[u8]; 1] = [b"3dsMax|params|OSLShaderName"];

        // C: `static const char *const source_props[] = { "3dsMax|params|OSLCode" };`
        static SOURCE_PROPS: [&[u8]; 1] = [b"3dsMax|params|OSLCode"];

        (*shader).shader_source.data = EMPTY_CHAR.as_ptr();
        (*shader).shader_name.data = EMPTY_CHAR.as_ptr();

        // C: `ufbxi_nounroll for (size_t i = 0; i < ufbxi_arraycount(name_props); i++)`
        for i in 0..NAME_PROPS.len() {
            if let Some(prop) = find_prop_len(texture_view.props_view(), NAME_PROPS[i]) {
                (*shader).shader_name = prop.value_str();
                break;
            }
        }

        // C: `ufbxi_nounroll for (size_t i = 0; i < ufbxi_arraycount(source_props); i++)`
        for i in 0..SOURCE_PROPS.len() {
            if let Some(prop) = find_prop_len(texture_view.props_view(), SOURCE_PROPS[i]) {
                (*shader).shader_source = prop.value_str();
                (*shader).raw_shader_source = prop.value_blob();
                break;
            }
        }
    }

    // SAFETY: `shader` is the fresh non-null push above — a live, arena-owned
    // `ufbx_shader_texture` — so the mint's liveness and write-capable
    // provenance hold.
    shader_texture_find_prefix(uc, texture_view, unsafe {
        ShaderTextureView::from_ptr(shader)
    })?;

    // SAFETY: `shader` is the fresh push above; the suffix scan walks `name`
    // backwards from its own length, so `begin` stays inside the interned
    // string it slices.
    unsafe {
        if (*shader).shader_name.length == 0 {
            let mut name: String = (*shader).prop_prefix;
            if sp::remove_suffix_c(&mut name, b" Parameters/Connections|\0".as_ptr()) {
                let mut begin: usize = name.length;
                while begin > 0 && *name.data.add(begin - 1) != b'|' {
                    begin -= 1;
                }

                (*shader).shader_name.data = name.data.add(begin);
                (*shader).shader_name.length = name.length - begin;
                sp::push_string_place_str(
                    uc.string_pool_mut_ptr(),
                    &raw mut (*shader).shader_name,
                    false,
                )?;
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
    }

    // SAFETY: walks the texture's own property table (`count` entries); the
    // growth targets uc's own paired `tmp_arr`/`tmp_arr_size` state, `input` is
    // the entry the grow just made room for (and is zeroed before use), and every
    // `base` is null-checked before its deref.
    unsafe {
        // C: `ufbxi_for_list(ufbx_prop, prop, texture->props.props)`
        let props = SliceViewIter::<Prop>::from_raw_parts(
            (*texture).element.props.props.data as *mut Prop,
            (*texture).element.props.props.count,
        );
        for prop in props {
            let mut name: String = prop.name();
            if !sp::remove_prefix_str(&mut name, (*shader).prop_prefix) {
                continue;
            }

            // Check if this property is a modifier to an existing input.
            let mut base_name: String = name;
            if sp::remove_suffix_c(&mut base_name, b"_map\0".as_ptr())
                || sp::remove_suffix_c(&mut base_name, b".shader\0".as_ptr())
            {
                let base = find_shader_texture_input_len(
                    ShaderTextureView::from_ptr(shader),
                    base_name.as_bytes(),
                );
                if let Some(base) = base {
                    base.set_texture_prop(opt_ref(prop.get()));
                    continue;
                }
            } else if sp::remove_suffix_c(&mut base_name, b".connected\0".as_ptr())
                || sp::remove_suffix_c(&mut base_name, b"Enabled\0".as_ptr())
            {
                let base = find_shader_texture_input_len(
                    ShaderTextureView::from_ptr(shader),
                    base_name.as_bytes(),
                );
                if let Some(base) = base {
                    base.set_texture_enabled_prop(opt_ref(prop.get()));
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
            (*input).prop = opt_ref(prop.get());
        }
    }

    // Retain the shader inputs
    // SAFETY: copies the `inputs.count` entries accumulated in uc's own tmp array
    // into uc's own result buffer, then records the fresh run on the texture's
    // own shader.
    unsafe {
        (*shader).inputs.data = uc
            .result_view()
            .push_copy_raw::<ShaderTextureInput>((*shader).inputs.count, (*shader).inputs.data);
        ufbxi_check!(uc, !(*shader).inputs.data.is_null(), "shader->inputs.data");

        (*texture).shader = opt_ref(shader);
        (*texture).type_ = TextureType::Shader;
        uc.scene_view()
            .metadata_view()
            .set_num_shader_textures(uc.scene_view().metadata_view().num_shader_textures() + 1);
    }

    if !uc.opts_view().disable_quirks() {
        // SAFETY: `shader`/`texture` as above; `fs` indexes the static `FILE_SHADERS`
        // table whose name fields are NUL-terminated literals, and `input`/`prop` are
        // null-checked before their derefs.
        unsafe {
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
                        let prop: *mut Prop = opt_ptr(&raw const (*input).prop);
                        (*texture).absolute_filename = (*prop).value_str;
                        (*texture).raw_absolute_filename = (*prop).value_blob;
                        (*texture).type_ = TextureType::File;
                        break;
                    }
                }
            }
        }
    }

    // SAFETY: `texture` lives in the uc arena, so the reinterpret satisfies the
    // element-view construction invariant; `shader` is the fresh push above,
    // reinterpreted in place by its view.
    unsafe {
        // Anchor the texture to `'a` (= uc): `texture` lives in the uc arena, so the
        // explicit `&'a` annotation forces the reinterpret lifetime to unify with the
        // `uc` borrow rather than infer free.
        let texture_view: &'a TextureView = TextureView::from_ptr(texture);
        update_shader_texture(texture_view, ShaderTextureView::from_ptr(shader));
    }

    Ok(())
}

// ufbx.c:20692-20752 `ufbxi_propagate_main_textures`
#[inline(never)]
pub(crate) fn propagate_main_textures(scene_view: &SceneView) {
    let textures: &RefListView<Texture> = scene_view.textures_view();
    // We need to do at least 2^(N-1) passes for N shader textures
    let mut mask: usize = scene_view.metadata_view().num_shader_textures();
    while mask != 0 {
        mask >>= 1;

        // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, scene->textures)`
        for texture_ix in 0..textures.count() {
            let texture: &TextureView = textures.at(texture_ix);
            let Some(shader) = texture.shader() else {
                continue;
            };
            // SAFETY: `shader` is the viewed texture's own non-null
            // `ufbx_shader_texture` reference, so it names a live scene shader
            // texture with the scene's write-capable provenance.
            let shader: &ShaderTextureView = unsafe { ShaderTextureView::from_ptr(shader.ptr()) };

            let Some(main_tex) = shader.main_texture() else {
                continue;
            };
            if shader.main_texture_output_index() != 0 {
                continue;
            }

            // SAFETY: `main_tex` is the viewed shader texture's own non-null
            // `main_texture` reference, so it names a live scene texture with
            // the scene's write-capable provenance.
            let main_tex: &TextureView = unsafe { TextureView::from_ptr(main_tex.ptr()) };
            let Some(main_shader) = main_tex.shader() else {
                continue;
            };
            // SAFETY: `main_shader` is that texture's own non-null
            // `ufbx_shader_texture` reference (see the `shader` mint above).
            let main_shader: &ShaderTextureView =
                unsafe { ShaderTextureView::from_ptr(main_shader.ptr()) };
            if main_shader.main_texture().is_none() {
                continue;
            }

            shader.set_main_texture(main_shader.main_texture());
            shader.set_main_texture_output_index(main_shader.main_texture_output_index());
        }
    }

    // Remove cyclic main textures
    // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, scene->textures)`
    for texture_ix in 0..textures.count() {
        let texture: &TextureView = textures.at(texture_ix);
        let Some(shader) = texture.shader() else {
            continue;
        };
        // SAFETY: `shader` is the viewed texture's own non-null
        // `ufbx_shader_texture` reference (see the mint in the pass above).
        let shader: &ShaderTextureView = unsafe { ShaderTextureView::from_ptr(shader.ptr()) };
        if shader.main_texture().is_none() || shader.main_texture_output_index() != 0 {
            continue;
        }
        // C: `ufbx_texture *main_tex = shader->main_texture;` — the guard above
        // proved it non-NULL, so C's following `main_tex &&` is the same test.
        let Some(main_tex) = shader.main_texture() else {
            continue;
        };
        // SAFETY: `main_tex` is the viewed shader texture's own non-null
        // `main_texture` reference, so it names a live scene texture with the
        // scene's write-capable provenance.
        let main_tex: &TextureView = unsafe { TextureView::from_ptr(main_tex.ptr()) };
        if let Some(main_shader) = main_tex.shader() {
            // SAFETY: `main_shader` is that texture's own non-null
            // `ufbx_shader_texture` reference (see the `shader` mint above).
            let main_shader: &ShaderTextureView =
                unsafe { ShaderTextureView::from_ptr(main_shader.ptr()) };
            if main_shader.main_texture().is_some() {
                // Should have been propagated to `texture`
                shader.set_main_texture(None);
            }
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, scene->textures)`
    for texture_ix in 0..textures.count() {
        let texture: &TextureView = textures.at(texture_ix);
        let Some(shader) = texture.shader() else {
            continue;
        };
        // SAFETY: `shader` is the viewed texture's own non-null
        // `ufbx_shader_texture` reference (see the mint in the first pass).
        let shader: &ShaderTextureView = unsafe { ShaderTextureView::from_ptr(shader.ptr()) };

        // C: `ufbxi_for_list(ufbx_shader_texture_input, input, shader->inputs)`
        let inputs = shader.inputs_view();
        for input_ix in 0..inputs.count() {
            let input = inputs.at(input_ix);
            let Some(input_texture) = input.texture() else {
                continue;
            };
            // SAFETY: `input_texture` is the viewed input's own non-null
            // texture reference, so it names a live scene texture with the
            // scene's write-capable provenance.
            let input_texture: &TextureView = unsafe { TextureView::from_ptr(input_texture.ptr()) };
            // C: `if (... || !input->texture->shader) continue;` and the
            // following `input_shader` binding read the same field.
            let Some(input_shader) = input_texture.shader() else {
                continue;
            };
            // SAFETY: `input_shader` is that texture's own non-null
            // `ufbx_shader_texture` reference (see the `shader` mint above).
            let input_shader: &ShaderTextureView =
                unsafe { ShaderTextureView::from_ptr(input_shader.ptr()) };
            if input_shader.main_texture().is_some() {
                input.set_texture(input_shader.main_texture());
                input.set_texture_output_index(input_shader.main_texture_output_index());
            }
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_material, p_material, scene->materials)`
    let materials: &RefListView<Material> = scene_view.materials_view();
    for material_ix in 0..materials.count() {
        let material: &MaterialView = materials.at(material_ix);

        // C: `ufbxi_for_list(ufbx_material_texture, tex, material->textures)`
        let material_textures = material.textures_view();
        for tex_ix in 0..material_textures.count() {
            let tex = material_textures.at(tex_ix);
            // SAFETY: `ufbx_material_texture.texture` is non-nullable, so the
            // viewed entry's own reference names a live scene texture with the
            // scene's write-capable provenance.
            let tex_texture: &TextureView = unsafe { TextureView::from_ptr(tex.texture().ptr()) };
            let shader = tex_texture.shader();
            if let Some(shader) = shader {
                // SAFETY: `shader` is that texture's own non-null
                // `ufbx_shader_texture` reference (see the mints above).
                let shader: &ShaderTextureView =
                    unsafe { ShaderTextureView::from_ptr(shader.ptr()) };
                if let Some(main_texture) = shader.main_texture() {
                    if shader.main_texture_output_index() == 0 {
                        // C: `tex->texture = shader->main_texture;` —
                        // `main_texture` is checked non-NULL just above, so the
                        // non-nullable `ufbx_material_texture.texture` stays valid.
                        tex.set_texture(main_texture);
                    }
                }
            }
        }
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
pub(crate) fn insert_texture_file(uc: &Context, texture: &TextureView) -> Result<(), Fail> {
    texture.set_file_index(NO_INDEX);

    let mut key: *const u8 = ptr::null();

    // HACK: Even the raw entries have a null terminator so we can offset the
    // pointer by one for relative filenames. This guarantees that an overlapping
    // absolute and relative filenames will get separate textures.
    if texture.raw_absolute_filename().size > 0 {
        key = texture.raw_absolute_filename().data;
    } else if texture.raw_relative_filename().size > 0 {
        // SAFETY: the raw relative blob spans `size` bytes plus the NUL the
        // string pool appends, so offsetting by one stays inside its allocation.
        key = unsafe { texture.raw_relative_filename().data.add(1) };
    }

    if key.is_null() {
        return Ok(());
    }
    let hash: u32 = hash_ptr!(key);
    let mut entry: *mut TextureFileEntry = uc.texture_file_map_view().find(hash, &key);
    if entry.is_null() {
        entry = uc.texture_file_map_view().insert(hash, &key);
        ufbxi_check!(uc, !entry.is_null(), "entry");

        let file: *mut TextureFile = uc.tmp_view().push_zero(1);
        ufbxi_check!(uc, !file.is_null(), "file");

        // SAFETY: `file` is the fresh non-null one-element push above; the insert
        // grew the map, so its size is at least one.
        unsafe { (*file).index = uc.texture_file_map_view().size() - 1 };

        // SAFETY: `entry` is the fresh non-null map slot above, and `file` the
        // fresh push, which lives in `uc`'s `tmp` arena for the rest of the read.
        unsafe {
            (*entry).key = key;
            (*entry).file = file;
        }
    }

    // SAFETY: `entry` is non-null — either the hit from `map_find` or the freshly
    // filled insert above — and points to a live `TextureFileEntry`.
    let file: *mut TextureFile = unsafe { (*entry).file };
    // SAFETY: `file` is that entry's `TextureFile`, pushed on `uc`'s `tmp` arena.
    texture.set_file_index(unsafe { (*file).index });
    texture.set_has_file(true);
    // SAFETY: `file` is live (see above), so the macro's `(*file)` field
    // projections address its own members.
    unsafe {
        patch_empty!((*file).filename, length, texture.filename());
        patch_empty!(
            (*file).relative_filename,
            length,
            texture.relative_filename()
        );
        patch_empty!(
            (*file).absolute_filename,
            length,
            texture.absolute_filename()
        );
        patch_empty!((*file).raw_filename, size, texture.raw_filename());
        patch_empty!(
            (*file).raw_relative_filename,
            size,
            texture.raw_relative_filename()
        );
        patch_empty!(
            (*file).raw_absolute_filename,
            size,
            texture.raw_absolute_filename()
        );
        patch_empty!((*file).content, size, texture.content());
    }

    Ok(())
}

// ufbx.c:20802-20817 `ufbxi_pop_texture_files`
#[inline(never)]
pub(crate) fn pop_texture_files(uc: &Context) -> Result<(), Fail> {
    let num_files: u32 = uc.texture_file_map_view().size();
    let files: *mut TextureFile = uc.result_view().push(num_files as usize);
    ufbxi_check!(uc, !files.is_null(), "files");

    uc.scene_view().texture_files_view().set_data(files);
    uc.scene_view()
        .texture_files_view()
        .set_count(num_files as usize);

    let entries: *mut TextureFileEntry =
        uc.texture_file_map_view().items() as *mut TextureFileEntry;
    // SAFETY: `entries` is uc's own texture-file map storage, which holds
    // `num_files` live entries; `files` is the fresh non-null `num_files`-entry
    // push above, so both sides of every copy are in bounds and disjoint.
    unsafe {
        for i in 0..num_files as usize {
            ptr::copy_nonoverlapping((*entries.add(i)).file, files.add(i), 1);
        }
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
    // SAFETY: the sort hands its comparator two pointers to live, initialized
    // elements of the `OrderedTexture` run it was given (C-callback contract),
    // which is what `va`/`vb` are cast back to.
    unsafe { (*a).texture < (*b).texture }
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
    // SAFETY: the sort hands its comparator two pointers to live, initialized
    // elements of the `OrderedTexture` run it was given (C-callback contract),
    // which is what `va`/`vb` are cast back to.
    unsafe { (*a).order < (*b).order }
}

// ufbx.c:20838-20867 `ufbxi_deduplicate_textures`
#[inline(never)]
pub(crate) unsafe fn deduplicate_textures(
    uc: &Context,
    dst_buf: &BufView,
    p_dst: *mut *mut OrderedTexture,
    p_dst_count: *mut usize,
    count: usize,
) -> Result<(), Fail> {
    let textures: *mut OrderedTexture = dst_buf.push_pop(uc.tmp_stack_view(), count);
    ufbxi_check!(uc, !textures.is_null(), "textures");

    ufbxi_check!(
        uc,
        // SAFETY: the three pointers are `uc`'s own live `ator_tmp` and the
        // `tmp_arr`/`tmp_arr_size` slots that pair with it.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count.wrapping_mul(size_of::<OrderedTexture>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbxi_ordered_texture)))"
    );

    // SAFETY: `textures` is the fresh non-null `count`-element run popped above
    // and `tmp_arr` was just grown to `count * size_of::<OrderedTexture>()`
    // bytes, so the two disjoint runs the sort needs are in place; the comparator
    // matches the element size passed alongside.
    unsafe {
        stable_sort(
            size_of::<OrderedTexture>(),
            16,
            textures as *mut c_void,
            uc.tmp_arr() as *mut c_void,
            count,
            ordered_texture_less_texture,
            ptr::null_mut(),
        )
    };

    // Remove adjacent duplicates
    let mut dst_ix: usize = 0;
    for src_ix in 0..count {
        // SAFETY: `src_ix < count` and the `src_ix > 0` guard short-circuits ahead
        // of the reads, so both indices are in bounds of the `count`-element
        // `textures` run.
        if src_ix > 0
            && unsafe { (*textures.add(src_ix - 1)).texture == (*textures.add(src_ix)).texture }
        {
            continue;
        } else {
            if src_ix != dst_ix {
                // SAFETY: `dst_ix <= src_ix < count`, so both indices are in
                // bounds of the `textures` run.
                unsafe { *textures.add(dst_ix) = *textures.add(src_ix) };
            }
            dst_ix += 1;
        }
    }

    let new_count: usize = dst_ix;
    // SAFETY: `new_count <= count`, so the run and the `tmp_arr` scratch grown
    // above both cover the sort; the comparator matches the element size.
    unsafe {
        stable_sort(
            size_of::<OrderedTexture>(),
            16,
            textures as *mut c_void,
            uc.tmp_arr() as *mut c_void,
            new_count,
            ordered_texture_less_order,
            ptr::null_mut(),
        )
    };

    // SAFETY: `p_dst_count` and `p_dst` are the caller's out-parameter slots (fn
    // contract).
    unsafe { *p_dst_count = new_count };
    // SAFETY: as above; `textures` is the arena run that outlives the caller.
    unsafe { *p_dst = textures };

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
pub(crate) fn fetch_file_textures(uc: &Context) -> Result<(), Fail> {
    // We keep pointers to `ufbx_texture` in `tmp_stack` as a working set, since we don't know
    // how deep the shader graphs might be.

    // Start by pushing all the textures into the stack
    let mut num_stack_textures: usize = uc.scene_view().textures_view().count();
    // SAFETY: copies the scene's stored `textures` element-pointer run
    // (`count` entries) onto uc's own tmp stack through its raw-ptr getter.
    unsafe {
        ufbxi_check!(
            uc,
            !uc.tmp_stack_view().push_copy_raw::<*mut Texture>(num_stack_textures,
                uc.scene_view().textures_view().data() as *const *mut Texture,
            )
            .is_null(),
            "((ufbx_texture**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_texture*), (num_stack_textures), (uc->scene.textures.data)))"
        );
    }

    // Compressed `ufbxi_file_texture_fetch_state`
    let states: *mut u8 = uc
        .tmp_view()
        .push_zero(uc.scene_view().textures_view().count());
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
        // SAFETY: pops one texture pointer from uc's own tmp stack (the loop
        // counter guarantees an entry is there) into an unaliased local.
        unsafe { pop::<*mut Texture>(uc.tmp_stack_mut_ptr(), 1, &raw mut texture) };

        // SAFETY: `states` is the fresh zeroed run pushed above, one byte per
        // scene texture, indexed here by the texture's own `typed_id`.
        let state: u32 = unsafe { *states.add((*texture).element.typed_id as usize) } as u32;
        if state == FILE_TEXTURE_FETCH_FINISHED {
            continue;
        }
        // SAFETY: reads the texture's own optional shader reference; the result
        // is null-checked before every use.
        let shader: *mut ShaderTexture = unsafe { opt_ptr(&raw const (*texture).shader) };

        // SAFETY: `texture`/`shader` as above; `states` is indexed by the texture's own
        // `typed_id`; each `dst` is the fresh non-null result of a push onto uc's own
        // tmp stack, `deduplicate_textures` writes its out-param before returning
        // Ok, and every walked run (`layers`, `inputs`, `file_textures`, the `deps`/
        // `files` runs) is traversed with its own count.
        if state == FILE_TEXTURE_FETCH_STARTED {
            unsafe {
                *states.add((*texture).element.typed_id as usize) =
                    FILE_TEXTURE_FETCH_FINISHED as u8;

                // HACK: Reuse `tmp_parse` for storing intermediate information as we can clear it.
                buf_clear(uc.tmp_parse_mut_ptr());

                // Now all non-cyclical dependents should be processed.
                let mut num_deps: usize = 0;

                if (*texture).type_ == TextureType::File {
                    let dst: *mut OrderedTexture = uc.tmp_stack_view().push(1);
                    ufbxi_check!(uc, !dst.is_null(), "dst");
                    (*dst).texture = texture;
                    (*dst).order = num_deps;
                    num_deps += 1;
                }

                // C: `ufbxi_for_list(ufbx_texture_layer, layer, texture->layers)`
                let mut layer: *mut TextureLayer = (*texture).layers.data as *mut TextureLayer;
                let layer_end: *mut TextureLayer = add_ptr(layer, (*texture).layers.count);
                while layer != layer_end {
                    let dep_tex: *mut Texture = ref_ptr(&raw const (*layer).texture);
                    if (*dep_tex).file_textures.count > 0 {
                        let dst: *mut OrderedTexture = uc.tmp_stack_view().push(1);
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
                        let dep_tex: *mut Texture = opt_ptr(&raw const (*input).texture);
                        if !dep_tex.is_null() && (*dep_tex).file_textures.count > 0 {
                            let dst: *mut OrderedTexture = uc.tmp_stack_view().push(1);
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
                    uc.tmp_parse_view(),
                    deps.as_mut_ptr(),
                    &raw mut num_deps,
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
                        &raw const (*(*deps.add(0)).texture).file_textures;
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
                            let dst: *mut OrderedTexture = uc.tmp_stack_view().push(1);
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
                        uc.tmp_parse_view(),
                        files.as_mut_ptr(),
                        &raw mut num_files,
                        num_files,
                    )?;
                    let files: *mut OrderedTexture = files.assume_init();

                    (*texture).file_textures.count = num_files;
                    (*texture).file_textures.data =
                        uc.result_view().push::<*mut Texture>(num_files) as *const Ref<Texture>;
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
            }
        } else {
            // SAFETY: `texture`/`shader` as above; `states` is indexed by the texture's own
            // `typed_id`; the `file_textures` run is the fresh non-null push right above
            // the write into it, and every dependency pushed onto uc's own tmp stack is
            // a `&raw` place inside a live element of the same arena.
            unsafe {
                if (*texture).type_ == TextureType::File {
                    // Simple case: Just point to self
                    (*texture).file_textures.count = 1;
                    (*texture).file_textures.data =
                        uc.result_view().push::<*mut Texture>(1) as *const Ref<Texture>;
                    ufbxi_check!(
                        uc,
                        !(*texture).file_textures.data.is_null(),
                        "texture->file_textures.data"
                    );
                    *((*texture).file_textures.data as *mut *mut Texture).add(0) = texture;

                    // In simple cases we can quit here, for more complex file textures queue
                    // the texture in case there are other file textures as inputs.
                    if opt_ptr(&raw const (*texture).shader).is_null() {
                        *states.add((*texture).element.typed_id as usize) =
                            FILE_TEXTURE_FETCH_FINISHED as u8;
                        continue;
                    }
                }

                // Complex: Process all dependencies first
                *states.add((*texture).element.typed_id as usize) =
                    FILE_TEXTURE_FETCH_STARTED as u8;

                // Push self first so we can return after processing dependencies
                ufbxi_check!(
                    uc,
                    !uc.tmp_stack_view().push_copy_ref(&texture).is_null(),
                    "((ufbx_texture**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_texture*), (1), (&texture)))"
                );
                num_stack_textures += 1;

                // C: `ufbxi_for_list(ufbx_texture_layer, layer, texture->layers)`
                let mut layer: *mut TextureLayer = (*texture).layers.data as *mut TextureLayer;
                let layer_end: *mut TextureLayer = add_ptr(layer, (*texture).layers.count);
                while layer != layer_end {
                    ufbxi_check!(
                        uc,
                        !uc.tmp_stack_view().push_copy_raw::<*mut Texture>(1,
                            &raw const (*layer).texture as *const *mut Texture,
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
                        if !opt_ptr(&raw const (*input).texture).is_null() {
                            ufbxi_check!(
                                uc,
                                !uc.tmp_stack_view().push_copy_raw::<*mut Texture>(1,
                                    &raw const (*input).texture as *const *mut Texture,
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
    }

    Ok(())
}

// ufbx.c:21009-21016 `ufbxi_get_geometry_transform_node`
#[inline(never)]
#[must_use]
pub(crate) fn get_geometry_transform_node(element: &ElementView) -> *mut Node {
    if element.instances().count == 1 {
        // SAFETY: `count == 1`, so index `0` is in bounds of the element's own
        // instance run; its entries are non-nullable node references, which
        // `ref_ptr` resolves to a live scene `Node`.
        let node: *mut Node = unsafe { ref_ptr(element.instances().data.add(0)) };
        // SAFETY: `node` is that live scene node.
        if unsafe { (*node).has_geometry_transform } {
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
    // SAFETY: `v_list` is null or points to a live `ufbx_*_list` header — a
    // `data`/`count` pair over `count` `ufbx_vec3`-shaped elements spaced by
    // `stride` (fn contract); the null check short-circuits ahead of the read.
    if axis == MirrorAxis::None || list.is_null() || unsafe { (*list).count } == 0 {
        return;
    }
    if stride == 0 {
        stride = size_of::<Vec3>();
    }

    // C: `(char*)list->data + (size_t)((int)axis - 1) * sizeof(ufbx_real)` —
    // the enum is narrowed to `int` before the subtraction, and `axis` is
    // 1..=3 here because `UFBX_MIRROR_AXIS_NONE` returned above.
    // SAFETY: `list` is non-null (checked) and its header is live (see above).
    let (data, count) = unsafe { ((*list).data as *mut u8, (*list).count) };
    let mut p: *mut u8 =
        data.wrapping_add(((axis as i32 - 1) as usize).wrapping_mul(size_of::<Real>()));
    let end: *mut u8 = p.wrapping_add(count.wrapping_mul(stride));
    while p != end {
        let v: *mut Real = p as *mut Real;
        // SAFETY: `p` walks the `count` strided elements of the list; `axis` is
        // 1..=3, so the component offset stays inside each element's three reals.
        unsafe { *v = -*v };
        p = p.wrapping_add(stride);
    }
}

// ufbx.c:21033-21047 `ufbxi_scale_vec3_list`
#[inline(never)]
pub(crate) unsafe fn scale_vec3_list(v_list: *const c_void, scale: Real, stride: usize) {
    let mut stride: usize = stride;
    let list: *const VoidList = v_list as *const VoidList;
    // SAFETY: `v_list` is null or points to a live `ufbx_*_list` header — a
    // `data`/`count` pair over `count` `ufbx_vec3`-shaped elements spaced by
    // `stride` (fn contract); the null check short-circuits ahead of the read.
    if list.is_null() || unsafe { (*list).count } == 0 {
        return;
    }
    if stride == 0 {
        stride = size_of::<Vec3>();
    }

    // SAFETY: `list` is non-null (checked) and its header is live (see above).
    let (mut p, count) = unsafe { ((*list).data as *mut u8, (*list).count) };
    let end: *mut u8 = p.wrapping_add(count.wrapping_mul(stride));
    while p != end {
        let v: *mut Vec3 = p as *mut Vec3;
        // SAFETY: `p` walks the `count` strided elements of the list, each a live
        // `ufbx_vec3`.
        unsafe {
            (*v).x *= scale;
            (*v).y *= scale;
            (*v).z *= scale;
        }
        p = p.wrapping_add(stride);
    }
}

// ufbx.c:21049-21061 `ufbxi_transform_vec3_list`
/// # Safety
/// `stride` must be `0` or the size of the viewed list's own element type `T`,
/// so that every strided slot the walk visits begins a live `ufbx_vec3`-shaped
/// triple inside that list's run. That pairing is the C `void *` contract and
/// is not expressible in the parameter types. `matrix` must point to a live
/// `ufbx_matrix` for the duration of the walk; it stays a raw pointer so C's
/// `&geo_node->geometry_to_node` transcribes as an address-of over arena memory
/// rather than a frozen shared borrow held across the strided writes.
#[inline(never)]
pub(crate) unsafe fn transform_vec3_list<T>(
    v_list: Option<&ListView<T>>,
    matrix: *const Matrix,
    stride: usize,
) {
    let mut stride: usize = stride;
    let Some(list) = v_list else {
        return;
    };
    if list.count() == 0 {
        return;
    }
    if stride == 0 {
        stride = size_of::<Vec3>();
    }

    let (mut p, count) = (list.data() as *mut u8, list.count());
    let end: *mut u8 = p.wrapping_add(count.wrapping_mul(stride));
    while p != end {
        let v: *mut Vec3 = p as *mut Vec3;
        // SAFETY: `p` walks the `count` strided slots of the viewed list's own
        // run, each a live `ufbx_vec3` per the `stride` contract above; `matrix`
        // addresses a live `ufbx_matrix` per the fn contract.
        unsafe { *v = transform_position(matrix, *v) };
        p = p.wrapping_add(stride);
    }
}

// ufbx.c:21063-21068 `ufbxi_normalize_vec3_list`
#[inline(never)]
pub(crate) fn normalize_vec3_list(list: &ListView<Vec3>) {
    // C: `ufbxi_nounroll ufbxi_for_list(ufbx_vec3, normal, *list)` — the
    // no-unroll pragma is optimizer-only and has no Rust analogue.
    let (mut normal, count) = (list.data() as *mut Vec3, list.count());
    let normal_end: *mut Vec3 = add_ptr(normal, count);
    while normal != normal_end {
        // SAFETY: the viewed list's `data`/`count` describe one live run of
        // initialized vectors (list invariant), and `normal != normal_end` keeps
        // it inside that run, so the advance lands at or before the run's
        // one-past-the-end pointer.
        unsafe {
            *normal = normalize3(*normal);
            normal = normal.add(1);
        }
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
pub(crate) unsafe fn flip_attrib_winding(
    uc: &Context,
    mesh: &View<Mesh>,
    indices: &ListView<u32>,
    is_position: bool,
) -> Result<(), Fail> {
    // All zero, no flipping needed
    if indices.data() == uc.zero_indices() || indices.count() == 0 {
        return Ok(());
    }

    if indices.data() == mesh.vertex_position().indices().data && !is_position {
        // Sharing indices with vertex position, already flipped.
        return Ok(());
    } else if indices.data() == uc.consecutive_indices() {
        // Need to duplicate consecutive indices, but we can cache the per mesh.
        if !uc.tmp_mesh_consecutive_indices().is_null() {
            // The cached run is `uc`'s own result-arena copy of the consecutive
            // indices for this mesh.
            indices.set_data(uc.tmp_mesh_consecutive_indices());
            return Ok(());
        }
        // SAFETY: `result_mut_ptr` is `uc`'s own live result buffer, and
        // `indices`' `data`/`count` describe the consecutive-index run being
        // copied.
        indices.set_data(unsafe {
            uc.result_view()
                .push_copy_raw::<u32>(indices.count(), indices.data())
        });
        ufbxi_check!(uc, !indices.data().is_null(), "indices->data");
        // The push above is the fresh non-null result-arena run.
        uc.set_tmp_mesh_consecutive_indices(indices.data() as *mut u32);
    }

    // `indices`' `data` is a writable run of `count` indices — either the mesh's
    // own or the result-arena copy pushed above.
    let data: *mut u32 = indices.data() as *mut u32;
    // C: `ufbxi_for_list(ufbx_face, face, mesh->faces)`
    // SAFETY: `faces` describes one contiguous arena run of the mesh's own,
    // initialized faces, live and unmoved for this call.
    let faces = unsafe {
        SliceViewIter::<Face>::from_raw_parts(mesh.faces().data as *mut Face, mesh.faces().count)
    };
    for face in faces {
        if face.num_indices() == 0 {
            continue;
        }
        // C: both sums are `unsigned int` arithmetic (wrapping) before the
        // widening to `size_t`.
        let mut begin: usize = face.index_begin().wrapping_add(1) as usize;
        let mut end: usize = face
            .index_begin()
            .wrapping_add(face.num_indices())
            .wrapping_sub(1) as usize;
        while begin < end {
            // SAFETY: every face's `[index_begin, index_begin + num_indices)` span
            // lies inside the mesh's `num_indices` attribute indices, which is the
            // length of the `data` run; `begin < end` keeps both inside that span.
            unsafe {
                let tmp: u32 = *data.add(begin);
                *data.add(begin) = *data.add(end);
                *data.add(end) = tmp;
            }
            begin += 1;
            end -= 1;
        }
    }

    Ok(())
}

// ufbx.c:21109-21163 `ufbxi_flip_winding`
#[inline(never)]
pub(crate) unsafe fn flip_winding(uc: &Context, mesh: &View<Mesh>) -> Result<(), Fail> {
    uc.set_tmp_mesh_consecutive_indices(ptr::null_mut());
    // SAFETY: each `indices_view()` views one of the mesh's own attribute index
    // lists, so its `count` spans the face index ranges `flip_attrib_winding`
    // reverses.
    unsafe {
        flip_attrib_winding(uc, mesh, mesh.vertex_position().indices_view(), true)?;
        flip_attrib_winding(uc, mesh, mesh.vertex_normal().indices_view(), false)?;
        flip_attrib_winding(uc, mesh, mesh.vertex_crease().indices_view(), false)?;
    }
    if mesh.uv_sets().count > 0 {
        // C: `ufbxi_for_list(ufbx_uv_set, set, mesh->uv_sets)`
        // SAFETY: `uv_sets` describes one contiguous arena run of the mesh's own
        // UV sets, live and unmoved for this call.
        let sets = unsafe {
            SliceViewIter::<UvSet>::from_raw_parts(
                mesh.uv_sets().data as *mut UvSet,
                mesh.uv_sets().count,
            )
        };
        for set in sets {
            // SAFETY: each `indices_view()` views this live `UvSet`'s own
            // attribute index list, whose span matches the mesh that owns the
            // set run.
            unsafe {
                flip_attrib_winding(uc, mesh, set.vertex_uv().indices_view(), false)?;
                flip_attrib_winding(uc, mesh, set.vertex_tangent().indices_view(), false)?;
                flip_attrib_winding(uc, mesh, set.vertex_bitangent().indices_view(), false)?;
            }
        }
        // C: struct assignment (memcpy) of the vertex-attribute headers; the
        // `Vertex*` structs are not `Copy` in the generated bindings, so the
        // copy is spelled as a byte-identical `copy_nonoverlapping`.
        let set0 = mesh.uv_sets_view().at(0);
        // SAFETY: each source is element `0`'s own header field, addressed
        // through that element's view; each destination is the mesh's own header
        // of the matching name, a distinct field of the same type.
        unsafe {
            ptr::copy_nonoverlapping(set0.vertex_uv_ptr(), mesh.vertex_uv_raw(), 1);
            ptr::copy_nonoverlapping(set0.vertex_bitangent_ptr(), mesh.vertex_bitangent_raw(), 1);
            ptr::copy_nonoverlapping(set0.vertex_tangent_ptr(), mesh.vertex_tangent_raw(), 1);
        }
    }
    if mesh.color_sets().count > 0 {
        // C: `ufbxi_for_list(ufbx_color_set, set, mesh->color_sets)`
        // SAFETY: `color_sets` describes one contiguous arena run of the mesh's
        // own color sets, live and unmoved for this call.
        let sets = unsafe {
            SliceViewIter::<ColorSet>::from_raw_parts(
                mesh.color_sets().data as *mut ColorSet,
                mesh.color_sets().count,
            )
        };
        for set in sets {
            // SAFETY: `indices_view()` views this live `ColorSet`'s own attribute
            // index list, whose span matches the mesh that owns the set run.
            unsafe { flip_attrib_winding(uc, mesh, set.vertex_color().indices_view(), false) }?;
        }
        let set0 = mesh.color_sets_view().at(0);
        // SAFETY: the source is element `0`'s own `vertex_color` header,
        // addressed through that element's view; the destination is the mesh's
        // own `vertex_color` header, a distinct field of the same type.
        unsafe { ptr::copy_nonoverlapping(set0.vertex_color_ptr(), mesh.vertex_color_raw(), 1) };
    }
    // SAFETY: `skinned_position().indices_view()` views the mesh's own
    // `skinned_position` index list.
    unsafe { flip_attrib_winding(uc, mesh, mesh.skinned_position().indices_view(), false) }?;
    if mesh.skinned_normal().indices().data != mesh.vertex_normal().indices().data {
        // SAFETY: as above, for the mesh's own `skinned_normal` index list.
        unsafe { flip_attrib_winding(uc, mesh, mesh.skinned_normal().indices_view(), false) }?;
    }

    // SAFETY: every mesh reaching `modify_geometry` had `vertex_first_index`
    // sized to `num_vertices` — by `process_indices` on the FBX/legacy read
    // path, or by `finalize_mesh` on the OBJ path — the count contract
    // `update_vertex_first_index` rests on.
    unsafe { update_vertex_first_index(mesh) };

    // Mapping from old index values to flipped ones, reserve index -1
    // (aka `UFBX_NO_INDEX`) for itself.
    if mesh.edges().count > 0 {
        ufbxi_check!(
            uc,
            // SAFETY: the three `uc` accessors hand out `uc`'s own tmp allocator
            // and tmp-array header.
            unsafe {
                grow_array::<u8>(
                    uc.ator_tmp_mut_ptr(),
                    uc.tmp_arr_mut_ptr(),
                    uc.tmp_arr_size_mut_ptr(),
                    mesh.num_indices().wrapping_add(1).wrapping_mul(size_of::<u32>()),
                )
            },
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), ((mesh->num_indices + 1) * sizeof(uint32_t)))"
        );
        // SAFETY: the grow above made `uc`'s tmp array at least
        // `(num_indices + 1) * size_of::<u32>()` bytes, so offsetting one `u32`
        // in stays inside that allocation.
        let index_mapping: *mut u32 = unsafe { (uc.tmp_arr() as *mut u32).add(1) };
        // SAFETY: `index_mapping` is offset one `u32` into the grown tmp array,
        // so index `-1` is that array's first element.
        unsafe { *index_mapping.offset(-1) = NO_INDEX };
        // C: `ufbxi_for_list(ufbx_face, face, mesh->faces)`
        // SAFETY: `faces` describes one contiguous arena run of the mesh's own,
        // initialized faces, live and unmoved for this call.
        let faces = unsafe {
            SliceViewIter::<Face>::from_raw_parts(
                mesh.faces().data as *mut Face,
                mesh.faces().count,
            )
        };
        for face in faces {
            if face.num_indices() == 0 {
                continue;
            }
            let begin: u32 = face.index_begin();
            let count: u32 = face.num_indices().wrapping_sub(1);
            // SAFETY: every face's `[index_begin, index_begin + num_indices)` span
            // lies inside the mesh's `num_indices` indices, and the tmp array holds
            // `num_indices + 1` `u32`s starting one slot before `index_mapping`.
            unsafe { *index_mapping.add(begin as usize) = begin };
            let mut i: u32 = 0;
            while i < count {
                // SAFETY: as above; `i < count = num_indices - 1` keeps
                // `begin + 1 + i` inside the face's own index span.
                unsafe {
                    *index_mapping.add(begin.wrapping_add(1).wrapping_add(i) as usize) =
                        begin.wrapping_add(count).wrapping_sub(i)
                };
                i += 1;
            }
        }

        // C: `ufbxi_for_list(ufbx_edge, p_edge, mesh->edges)`
        // SAFETY: `edges` describes one contiguous arena run of the mesh's own,
        // initialized edges, live and unmoved for this call.
        let edges = unsafe {
            SliceViewIter::<Edge>::from_raw_parts(
                mesh.edges().data as *mut Edge,
                mesh.edges().count,
            )
        };
        for p_edge in edges {
            // C-parity: the `(int32_t)` casts are load-bearing — a
            // `UFBX_NO_INDEX` endpoint indexes `index_mapping[-1]`, the slot
            // reserved above.
            // SAFETY: every edge endpoint is either an index below `num_indices`
            // or `UFBX_NO_INDEX`, which the `(int32_t)` cast turns into the
            // reserved slot at offset `-1` — both inside the `num_indices + 1`
            // `u32`s of the grown tmp array.
            let (a, b) = unsafe {
                (
                    *index_mapping.offset(p_edge.a() as i32 as isize),
                    *index_mapping.offset(p_edge.b() as i32 as isize),
                )
            };
            p_edge.set_a(b);
            p_edge.set_b(a);
        }
    }

    Ok(())
}

// ufbx.c:21165-21332 `ufbxi_modify_geometry`
#[inline(never)]
pub(crate) fn modify_geometry<'a>(uc: &'a Context) -> Result<(), Fail> {
    let mut do_mirror: bool = false;
    let do_winding: bool = uc.opts_view().reverse_winding();
    let mut do_scale: bool = false;
    let mut do_geometry_transforms: bool = false;
    if uc.opts_view().geometry_transform_handling() == GeometryTransformHandling::ModifyGeometry
        || uc.opts_view().geometry_transform_handling()
            == GeometryTransformHandling::ModifyGeometryNoFallback
    {
        // Prefetch geometry transforms for processing, they will later be overwritten in `ufbxi_update_node()`.
        // SAFETY: walks the stored `nodes` element-pointer run of the uc-owned scene
        // (`count` entries); each node's props are reached through an arena-anchored
        // view, and the transform helpers read that node's own fields.
        unsafe {
            // C: `ufbxi_for_ptr_list(ufbx_node, p_node, uc->scene.nodes)`
            let mut p_node: *mut *mut Node = uc.scene_view().nodes_view().data() as *mut *mut Node;
            let p_node_end: *mut *mut Node = add_ptr(p_node, uc.scene_view().nodes_view().count());
            while p_node != p_node_end {
                let node_view: &'a NodeView = NodeView::from_ptr(*p_node);
                let node: *mut Node = node_view.get();
                if (*node).is_root {
                    p_node = p_node.add(1);
                    continue;
                }

                (*node).geometry_transform =
                    get_geometry_transform(node_view.props_view(), node_view);
                if !is_transform_identity(&raw const (*node).geometry_transform) {
                    (*node).geometry_to_node =
                        transform_to_matrix(&raw const (*node).geometry_transform);
                    (*node).has_geometry_transform = true;
                } else {
                    (*node).geometry_to_node = IDENTITY_MATRIX;
                    (*node).has_geometry_transform = false;
                }
                p_node = p_node.add(1);
            }
        }
        do_geometry_transforms = true;
    }
    if uc.mirror_axis() != MirrorAxis::None {
        do_mirror = true;
    }
    if uc.scene_view().metadata_view().geometry_scale() != 1.0 {
        do_scale = true;
    }

    let geometry_scale: Real = uc.scene_view().metadata_view().geometry_scale();
    let mirror_axis: MirrorAxis = uc.mirror_axis();

    // SAFETY: walks the stored `blend_shapes` element-pointer run of the uc-owned
    // scene (`count` entries); the list helpers take `&raw const` places of that
    // shape's own offset lists, which carry their own lengths.
    unsafe {
        // C: `ufbxi_for_ptr_list(ufbx_blend_shape, p_shape, uc->scene.blend_shapes)`
        let mut p_shape: *mut *mut BlendShape =
            uc.scene_view().blend_shapes_view().data() as *mut *mut BlendShape;
        let p_shape_end: *mut *mut BlendShape =
            add_ptr(p_shape, uc.scene_view().blend_shapes_view().count());
        while p_shape != p_shape_end {
            let shape: *mut BlendShape = *p_shape;

            if do_scale {
                scale_vec3_list(
                    &raw const (*shape).position_offsets as *const c_void,
                    geometry_scale,
                    0,
                );
            }

            if do_mirror {
                mirror_vec3_list(
                    &raw const (*shape).position_offsets as *const c_void,
                    mirror_axis,
                    0,
                );
                mirror_vec3_list(
                    &raw const (*shape).normal_offsets as *const c_void,
                    mirror_axis,
                    0,
                );
            }
            p_shape = p_shape.add(1);
        }
    }

    // SAFETY: walks the stored `meshes` element-pointer run of the uc-owned scene
    // (`count` entries) and, inside it, each mesh's own `uv_sets` run; the list
    // helpers take view projections of the mesh's own attribute lists (each
    // carrying its own length), and `geo_node` is null-checked before its
    // transform is read.
    unsafe {
        // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, uc->scene.meshes)`
        let mut p_mesh: *mut *mut Mesh = uc.scene_view().meshes_view().data() as *mut *mut Mesh;
        let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, uc.scene_view().meshes_view().count());
        while p_mesh != p_mesh_end {
            // The stored entry is a context-owned mesh element, so its
            // provenance is write-capable: `Mut` is the right mode.
            let mesh = View::<Mesh>::from_ptr(*p_mesh);

            if do_scale {
                scale_vec3_list(
                    mesh.vertex_position().values_ptr() as *const c_void,
                    geometry_scale,
                    0,
                );
            }

            let mut do_flip_winding: bool = do_winding;
            if do_mirror {
                mirror_vec3_list(
                    mesh.vertex_position().values_ptr() as *const c_void,
                    mirror_axis,
                    0,
                );
                mirror_vec3_list(
                    mesh.vertex_normal().values_ptr() as *const c_void,
                    mirror_axis,
                    0,
                );
                // C: `ufbxi_for_list(ufbx_uv_set, set, mesh->uv_sets)`
                let sets = SliceViewIter::<UvSet>::from_raw_parts(
                    mesh.uv_sets().data as *mut UvSet,
                    mesh.uv_sets().count,
                );
                for set in sets {
                    mirror_vec3_list(
                        set.vertex_tangent().values_ptr() as *const c_void,
                        mirror_axis,
                        0,
                    );
                    mirror_vec3_list(
                        set.vertex_bitangent().values_ptr() as *const c_void,
                        mirror_axis,
                        0,
                    );
                }
                if !uc.opts_view().handedness_conversion_retain_winding() {
                    do_flip_winding = !do_flip_winding;
                }
            }

            // Flip face winding retaining the first vertex
            if do_flip_winding {
                mesh.set_reversed_winding(true);
                flip_winding(uc, mesh)?;
            }

            let geo_node: *mut Node = get_geometry_transform_node(mesh.element());
            if do_geometry_transforms && !geo_node.is_null() {
                let mut tangent_matrix: Matrix = (*geo_node).geometry_to_node;
                tangent_matrix.m03 = 0.0;
                tangent_matrix.m13 = 0.0;
                tangent_matrix.m23 = 0.0;
                let normal_matrix: Matrix =
                    matrix_for_normals(&raw const (*geo_node).geometry_to_node);

                transform_vec3_list(
                    Some(mesh.vertex_position().values_view()),
                    &raw const (*geo_node).geometry_to_node,
                    0,
                );
                transform_vec3_list(
                    Some(mesh.vertex_normal().values_view()),
                    &raw const normal_matrix,
                    0,
                );
                normalize_vec3_list(mesh.vertex_normal().values_view());

                // C: `ufbxi_for_list(ufbx_uv_set, set, mesh->uv_sets)`
                let sets = SliceViewIter::<UvSet>::from_raw_parts(
                    mesh.uv_sets().data as *mut UvSet,
                    mesh.uv_sets().count,
                );
                for set in sets {
                    transform_vec3_list(
                        Some(set.vertex_tangent().values_view()),
                        &raw const tangent_matrix,
                        0,
                    );
                    transform_vec3_list(
                        Some(set.vertex_bitangent().values_view()),
                        &raw const tangent_matrix,
                        0,
                    );
                    normalize_vec3_list(set.vertex_tangent().values_view());
                    normalize_vec3_list(set.vertex_bitangent().values_view());
                }
            }
            p_mesh = p_mesh.add(1);
        }
    }

    // SAFETY: walks the stored `line_curves` element-pointer run of the uc-owned
    // scene (`count` entries); same list-helper and null-checked `geo_node`
    // contract as the loops around it.
    unsafe {
        // C: `ufbxi_for_ptr_list(ufbx_line_curve, p_curve, uc->scene.line_curves)`
        let mut p_curve: *mut *mut LineCurve =
            uc.scene_view().line_curves_view().data() as *mut *mut LineCurve;
        let p_curve_end: *mut *mut LineCurve =
            add_ptr(p_curve, uc.scene_view().line_curves_view().count());
        while p_curve != p_curve_end {
            let curve: *mut LineCurve = *p_curve;

            if do_scale {
                scale_vec3_list(
                    &raw const (*curve).control_points as *const c_void,
                    geometry_scale,
                    0,
                );
            }

            if do_mirror {
                mirror_vec3_list(
                    &raw const (*curve).control_points as *const c_void,
                    mirror_axis,
                    0,
                );
            }

            // SAFETY: `curve` is that live arena curve, so `&raw mut (*curve).element`
            // addresses its own element header, which anchors an `ElementView`.
            let geo_node: *mut Node =
                get_geometry_transform_node(ElementView::from_ptr(&raw mut (*curve).element));
            if do_geometry_transforms && !geo_node.is_null() {
                // SAFETY: `curve` is that live arena curve, so
                // `&raw mut (*curve).control_points` addresses its own
                // control-point list header, whose provenance is write-capable.
                transform_vec3_list(
                    Some(ListView::from_ptr(&raw mut (*curve).control_points)),
                    &raw const (*geo_node).geometry_to_node,
                    0,
                );
            }
            p_curve = p_curve.add(1);
        }
    }

    // SAFETY: walks the stored `nurbs_curves` element-pointer run of the uc-owned
    // scene (`count` entries); same list-helper and null-checked `geo_node`
    // contract as the loops around it.
    unsafe {
        // C: `ufbxi_for_ptr_list(ufbx_nurbs_curve, p_curve, uc->scene.nurbs_curves)`
        let mut p_curve: *mut *mut NurbsCurve =
            uc.scene_view().nurbs_curves_view().data() as *mut *mut NurbsCurve;
        let p_curve_end: *mut *mut NurbsCurve =
            add_ptr(p_curve, uc.scene_view().nurbs_curves_view().count());
        while p_curve != p_curve_end {
            let curve: *mut NurbsCurve = *p_curve;

            if do_scale {
                scale_vec3_list(
                    &raw const (*curve).control_points as *const c_void,
                    geometry_scale,
                    size_of::<Vec4>(),
                );
            }

            if do_mirror {
                mirror_vec3_list(
                    &raw const (*curve).control_points as *const c_void,
                    mirror_axis,
                    size_of::<Vec4>(),
                );
            }

            // SAFETY: `curve` is that live arena curve, so `&raw mut (*curve).element`
            // addresses its own element header, which anchors an `ElementView`.
            let geo_node: *mut Node =
                get_geometry_transform_node(ElementView::from_ptr(&raw mut (*curve).element));
            if do_geometry_transforms && !geo_node.is_null() {
                // SAFETY: `curve` is that live arena curve, so
                // `&raw mut (*curve).control_points` addresses its own
                // control-point list header, whose provenance is write-capable.
                transform_vec3_list(
                    Some(ListView::from_ptr(&raw mut (*curve).control_points)),
                    &raw const (*geo_node).geometry_to_node,
                    size_of::<Vec4>(),
                );
            }
            p_curve = p_curve.add(1);
        }
    }

    // SAFETY: walks the stored `nurbs_surfaces` element-pointer run of the
    // uc-owned scene (`count` entries); each list helper is handed a `&raw const`
    // of that surface's own control-point list, which carries its own length, and
    // `geo_node` is null-checked before its transform is read.
    unsafe {
        // C: `ufbxi_for_ptr_list(ufbx_nurbs_surface, p_surface, uc->scene.nurbs_surfaces)`
        let mut p_surface: *mut *mut NurbsSurface =
            uc.scene_view().nurbs_surfaces_view().data() as *mut *mut NurbsSurface;
        let p_surface_end: *mut *mut NurbsSurface =
            add_ptr(p_surface, uc.scene_view().nurbs_surfaces_view().count());
        while p_surface != p_surface_end {
            let surface: *mut NurbsSurface = *p_surface;

            if do_scale {
                scale_vec3_list(
                    &raw const (*surface).control_points as *const c_void,
                    geometry_scale,
                    size_of::<Vec4>(),
                );
            }

            if do_mirror {
                mirror_vec3_list(
                    &raw const (*surface).control_points as *const c_void,
                    mirror_axis,
                    size_of::<Vec4>(),
                );
            }

            // SAFETY: `surface` is that live arena surface, so
            // `&raw mut (*surface).element` addresses its own element header, which
            // anchors an `ElementView`.
            let geo_node: *mut Node =
                get_geometry_transform_node(ElementView::from_ptr(&raw mut (*surface).element));
            if do_geometry_transforms && !geo_node.is_null() {
                // SAFETY: `surface` is that live arena surface, so
                // `&raw mut (*surface).control_points` addresses its own
                // control-point list header, whose provenance is write-capable.
                transform_vec3_list(
                    Some(ListView::from_ptr(&raw mut (*surface).control_points)),
                    &raw const (*geo_node).geometry_to_node,
                    size_of::<Vec4>(),
                );
            }
            p_surface = p_surface.add(1);
        }
    }

    if uc.opts_view().geometry_transform_handling() != GeometryTransformHandling::Preserve {
        // Reset all geometry transforms if we're not preserving them
        let mut defaults: *mut Props = ptr::null_mut();
        // SAFETY: walks the stored `nodes` element-pointer run of the uc-owned scene
        // (`count` entries); `set_own_prop_vec3_uniform` receives a `&raw mut` of that
        // node's own props (or the null-checked shared `defaults` table).
        unsafe {
            // C: `ufbxi_for_ptr_list(ufbx_node, p_node, uc->scene.nodes)`
            let mut p_node: *mut *mut Node = uc.scene_view().nodes_view().data() as *mut *mut Node;
            let p_node_end: *mut *mut Node = add_ptr(p_node, uc.scene_view().nodes_view().count());
            while p_node != p_node_end {
                let node: *mut Node = *p_node;
                if defaults.is_null() {
                    defaults = opt_ptr(&raw const (*node).element.props.defaults);
                }

                if (*node).has_geometry_transform {
                    set_own_prop_vec3_uniform(
                        &raw mut (*node).element.props,
                        &sp::GeometricTranslation,
                        0.0,
                    );
                    set_own_prop_vec3_uniform(
                        &raw mut (*node).element.props,
                        &sp::GeometricRotation,
                        0.0,
                    );
                    set_own_prop_vec3_uniform(
                        &raw mut (*node).element.props,
                        &sp::GeometricScaling,
                        1.0,
                    );
                }
                p_node = p_node.add(1);
            }

            if !defaults.is_null() {
                set_own_prop_vec3_uniform(defaults, &sp::GeometricTranslation, 0.0);
                set_own_prop_vec3_uniform(defaults, &sp::GeometricRotation, 0.0);
                set_own_prop_vec3_uniform(defaults, &sp::GeometricScaling, 1.0);
            }
        }
    }

    Ok(())
}

// ufbx.c:21334-21356 `ufbxi_postprocess_scene`
#[inline(never)]
pub(crate) fn postprocess_scene(uc: &Context) {
    if uc.opts_view().normalize_normals() || uc.opts_view().normalize_tangents() {
        // SAFETY: walks the stored `meshes` element-pointer run of the uc-owned scene
        // (`count` entries) and, inside it, each mesh's own `uv_sets` run;
        // `normalize_vec3_list` is handed a view projection of the mesh's own
        // attribute list, which carries its own length.
        unsafe {
            // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, uc->scene.meshes)`
            let mut p_mesh: *mut *mut Mesh = uc.scene_view().meshes_view().data() as *mut *mut Mesh;
            let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, uc.scene_view().meshes_view().count());
            while p_mesh != p_mesh_end {
                // The stored entry is a context-owned mesh element, so its
                // provenance is write-capable: `Mut` is the right mode.
                let mesh = View::<Mesh>::from_ptr(*p_mesh);
                if uc.opts_view().normalize_normals() {
                    normalize_vec3_list(mesh.vertex_normal().values_view());
                }
                if uc.opts_view().normalize_tangents() {
                    // C-parity: the loop body normalizes the MESH-level tangent and
                    // bitangent lists (not `set->...`), so it repeats the same work
                    // once per UV set. Ported verbatim.
                    let mut set: *mut UvSet = mesh.uv_sets().data as *mut UvSet;
                    let set_end: *mut UvSet = add_ptr(set, mesh.uv_sets().count);
                    while set != set_end {
                        normalize_vec3_list(mesh.vertex_tangent().values_view());
                        normalize_vec3_list(mesh.vertex_bitangent().values_view());
                        set = set.add(1);
                    }
                }
                p_mesh = p_mesh.add(1);
            }
        }
    }

    if uc.exporter() == Exporter::BlenderBinary {
        uc.scene_view()
            .metadata_view()
            .set_ortho_size_unit(1.0 / uc.scene_view().metadata_view().geometry_scale());
    } else {
        uc.scene_view().metadata_view().set_ortho_size_unit(30.0);
    }
}

// ufbx.c:21358-21366 `ufbxi_next_path_segment`
#[inline(never)]
pub(crate) unsafe fn next_path_segment(data: *const u8, begin: usize, length: usize) -> usize {
    let mut i: usize = begin;
    while i < length {
        // SAFETY: `data` addresses `length` readable bytes (fn contract) and
        // `i < length` keeps the offset inside that run.
        if unsafe { *data.add(i) } == b'/' || unsafe { *data.add(i) } == b'\\' {
            return i;
        }
        i += 1;
    }
    length
}

// ufbx.c:21368-21435 `ufbxi_absolute_to_relative_path`
#[inline(never)]
pub(crate) unsafe fn absolute_to_relative_path(
    uc: &Context,
    p_dst: *mut Strblob,
    p_rel: *const Strblob,
    p_src: *const Strblob,
    raw: bool,
) -> Result<(), Fail> {
    // SAFETY: `p_rel` points to a live, initialized `ufbx_strblob` (fn contract),
    // so the `raw`-selected member's data pointer is readable.
    let rel: *const u8 = unsafe { strblob_data(p_rel, raw) };
    // SAFETY: as above, for `p_src`.
    let src: *const u8 = unsafe { strblob_data(p_src, raw) };
    // SAFETY: as above; the length comes from the same strblob member.
    let mut rel_length: usize = unsafe { strblob_length(p_rel, raw) };
    // SAFETY: as above, for `p_src`.
    let src_length: usize = unsafe { strblob_length(p_src, raw) };

    if rel_length == 0 || src_length == 0 {
        return Ok(());
    }

    // Absolute paths must start with the same character (either drive or '/')
    // SAFETY: `rel`/`src` address `rel_length`/`src_length` readable bytes, and
    // both lengths are non-zero here, so byte `0` of each is inside its run.
    if unsafe { *rel.add(0) != *src.add(0) } {
        return Ok(());
    }

    // Find the last directory of the path we want to be relative to
    // SAFETY: `rel` addresses `rel_length` readable bytes — the strblob's own
    // string — and the loop only reads while `rel_length > 0`, so
    // `rel_length - 1` indexes inside that run.
    while rel_length > 0
        && unsafe { *rel.add(rel_length - 1) != b'/' && *rel.add(rel_length - 1) != b'\\' }
    {
        rel_length -= 1;
    }

    if rel_length == 0 {
        return Ok(());
    }
    // SAFETY: `rel_length > 0` and it only shrank from the strblob's own length,
    // so `rel_length - 1` indexes inside the `rel` run.
    let separator: u8 = unsafe { *rel.add(rel_length - 1) };

    let max_length: usize = rel_length.wrapping_mul(2).wrapping_add(src_length);

    ufbxi_check!(
        uc,
        // SAFETY: the three `uc` accessors hand out `uc`'s own tmp allocator and
        // tmp-array header.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                max_length,
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (max_length))"
    );
    let tmp: *mut u8 = uc.tmp_arr();
    let mut tmp_length: usize = 0;

    let mut rel_begin: usize = 0;
    let mut src_begin: usize = 0;
    while rel_begin < rel_length && src_begin < src_length {
        // SAFETY: `rel` addresses `rel_length` readable bytes and
        // `rel_begin < rel_length`.
        let rel_end: usize = unsafe { next_path_segment(rel, rel_begin, rel_length) };
        // SAFETY: `src` addresses `src_length` readable bytes and
        // `src_begin < src_length`.
        let src_end: usize = unsafe { next_path_segment(src, src_begin, src_length) };
        if rel_end != src_end
            // SAFETY: `next_path_segment` returns an index in
            // `[begin, length]`, so `[rel_begin, src_end)` and
            // `[src_begin, src_end)` — equal-length spans, since
            // `rel_end == src_end` here — stay inside the two runs.
            || unsafe { memcmp(rel.add(rel_begin), src.add(src_begin), src_end - src_begin) } != 0
        {
            break;
        }
        rel_begin = rel_end + 1;
        src_begin = src_end + 1;
    }

    while rel_begin < rel_length {
        // SAFETY: `rel` addresses `rel_length` readable bytes and
        // `rel_begin < rel_length`.
        let rel_end: usize = unsafe { next_path_segment(rel, rel_begin, rel_length) };
        // SAFETY: `tmp` is the grown tmp array of `max_length == 2 * rel_length
        // + src_length` bytes. Each iteration writes three bytes and advances
        // `rel_begin` past one `rel` segment plus its separator, so as long as
        // every remaining segment is non-empty the stride is at least two and
        // this loop emits at most `1.5 * (rel_length - rel_begin)` bytes, which
        // together with the `src_length - src_begin` bytes the copy loop below
        // writes stays inside `max_length`. A run of consecutive separators in
        // `rel` shortens that stride to one and is the only shape this sizing
        // does not cover; the port keeps the C sizing (ufbx.c:21405-21411)
        // rather than diverging, and the `ufbx_assert!` after the loops reports
        // the overrun in debug builds.
        unsafe {
            *tmp.add(tmp_length) = b'.';
            tmp_length += 1;
            *tmp.add(tmp_length) = b'.';
            tmp_length += 1;
            *tmp.add(tmp_length) = separator;
            tmp_length += 1;
        }
        rel_begin = rel_end + 1;
    }

    while src_begin < src_length {
        // SAFETY: `src` addresses `src_length` readable bytes and
        // `src_begin < src_length`.
        let src_end: usize = unsafe { next_path_segment(src, src_begin, src_length) };
        let len: usize = src_end - src_begin;

        // SAFETY: `[src_begin, src_end)` lies inside the `src` run; `tmp` is the
        // grown tmp array of `max_length` bytes and the copied segments plus
        // their separators total exactly `src_length - src_begin` bytes on top
        // of the `..`-prefix written above, so the same accounting as that loop
        // applies — it fits in `max_length` for every `rel` without a run of
        // consecutive separators; the tmp array is a distinct allocation from
        // the interned `src` string.
        unsafe { ptr::copy_nonoverlapping(src.add(src_begin), tmp.add(tmp_length), len) };
        tmp_length += len;

        if src_end < src_length {
            // SAFETY: as above — the separator is the byte accounted for the
            // segment that follows.
            unsafe { *tmp.add(tmp_length) = separator };
            tmp_length += 1;
        }

        src_begin = src_end + 1;
    }

    ufbx_assert!(tmp_length <= max_length);

    // C-parity: `raw` is hardcoded `true` here, independent of the `raw`
    // parameter that selected the source/destination strblob members.
    // SAFETY: `string_pool_mut_ptr` hands out `uc`'s own live string pool, and
    // `tmp`/`tmp_length` describe the bytes just written into the tmp array.
    let dst: *const u8 = unsafe {
        sp::push_string(
            uc.string_pool_mut_ptr(),
            tmp,
            tmp_length,
            ptr::null_mut(),
            true,
        )
    };
    ufbxi_check!(uc, !dst.is_null(), "dst");

    // SAFETY: `p_dst` points to a live `ufbx_strblob` to write (fn contract), and
    // `dst`/`tmp_length` describe the string just interned in `uc`'s pool.
    unsafe { strblob_set(p_dst, dst, tmp_length, raw) };

    Ok(())
}

// ufbx.c:21437-21450 `ufbxi_resolve_filenames`
#[inline(never)]
pub(crate) fn resolve_filenames(
    uc: &Context,
    filename: &View<Strblob>,
    absolute_filename: &View<Strblob>,
    relative_filename: &View<Strblob>,
    raw: bool,
) -> Result<(), Fail> {
    // SAFETY: `relative_filename` views a live `ufbx_strblob` — the element
    // field being resolved (view mint invariant).
    if unsafe { strblob_length(relative_filename.as_ptr(), raw) } == 0 {
        let original_file_path: *const Strblob = if raw {
            uc.scene_view().metadata_view().raw_original_file_path_ptr() as *const Strblob
        } else {
            uc.scene_view().metadata_view().original_file_path_ptr() as *const Strblob
        };

        // SAFETY: `relative_filename`/`absolute_filename` view live strblobs of the
        // element being resolved (view mint invariant), and `original_file_path`
        // addresses the scene metadata's own path field, which the accessors above
        // hand out as a `Strblob`-shaped view of the matching `raw`-ness.
        unsafe {
            absolute_to_relative_path(
                uc,
                relative_filename.get(),
                original_file_path,
                absolute_filename.get(),
                raw,
            )
        }?;
    }

    // SAFETY: `filename`/`relative_filename` view live strblobs of the element
    // being resolved (view mint invariant).
    unsafe { resolve_relative_filename(uc, filename.get(), relative_filename.as_ptr(), raw) }?;

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
    // SAFETY: as the sort comparator for a `ufbxi_file_content` array, `va`/`vb`
    // address live, initialized elements of it (`stable_sort` contract), and both
    // `absolute_filename`s are interned pool strings.
    unsafe {
        str_less(
            (*a).absolute_filename.as_bytes(),
            (*b).absolute_filename.as_bytes(),
        )
    }
}

// ufbx.c:21459-21464 `ufbxi_sort_file_contents`
#[inline(never)]
pub(crate) unsafe fn sort_file_contents(
    uc: &Context,
    content: *mut FileContent,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the three `uc` accessors hand out `uc`'s own tmp allocator and
        // tmp-array header.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count.wrapping_mul(size_of::<FileContent>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbxi_file_content)))"
    );
    // SAFETY: `content` addresses `count` live, initialized `ufbxi_file_content`s
    // (fn contract) and the grow above sized `uc`'s tmp array to hold `count` of
    // them as the sort's scratch buffer; `file_content_less` is the matching
    // comparator and takes no user data.
    unsafe {
        stable_sort(
            size_of::<FileContent>(),
            32,
            content as *mut c_void,
            uc.tmp_arr() as *mut c_void,
            count,
            file_content_less,
            ptr::null_mut(),
        )
    };
    Ok(())
}

// ufbx.c:21466-21475 `ufbxi_push_file_content`
#[inline(never)]
pub(crate) fn push_file_content(
    uc: &Context,
    p_filename: &View<String>,
    p_data: &View<Blob>,
) -> Result<(), Fail> {
    if p_data.size() == 0 || p_filename.length() == 0 {
        return Ok(());
    }
    let content: *mut FileContent = uc.tmp_stack_view().push::<FileContent>(1);
    ufbxi_check!(uc, !content.is_null(), "content");

    // SAFETY: `content` is the non-null one-element push just made into `uc`'s tmp
    // stack, so it addresses writable storage for one `ufbxi_file_content`; the two
    // C struct assignments are rebuilt from the source views' own leaf reads.
    unsafe { (*content).absolute_filename = String::new_c(p_filename.data(), p_filename.length()) };
    // SAFETY: as above.
    unsafe { (*content).content = Blob::new_c(p_data.data(), p_data.size()) };
    Ok(())
}

// ufbx.c:21477-21488 `ufbxi_fetch_file_content`
#[inline(never)]
pub(crate) fn fetch_file_content(uc: &Context, p_filename: &View<String>, p_data: &View<Blob>) {
    if p_data.size() > 0 {
        return;
    }
    let filename: String = String::new_c(p_filename.data(), p_filename.length());
    let mut index: usize = usize::MAX;
    // C: `ufbxi_macro_lower_bound_eq(ufbxi_file_content, 8, &index,
    // uc->file_content, 0, uc->num_file_content, ...)` — does NOT write
    // `index` on a miss, hence the `SIZE_MAX` pre-initialization above.
    let cmp_lambda = |a: *const FileContent| {
        // SAFETY: `macro_lower_bound_eq` only passes pointers to live, initialized
        // elements of the run it was handed; both `absolute_filename`s are
        // interned pool strings.
        unsafe { str_less((*a).absolute_filename.as_bytes(), filename.as_bytes()) }
    };
    // C-parity: the equality lambda compares interned string POINTERS, not the
    // bytes.
    let eq_lambda = |a: *const FileContent| {
        // SAFETY: as `cmp_lambda`.
        (unsafe { (*a).absolute_filename.data }) == filename.data
    };
    // SAFETY: `uc.file_content()`/`num_file_content()` are `uc`'s own sorted
    // file-content run and its length, and `&mut index` is a live local.
    unsafe {
        macro_lower_bound_eq(
            8,
            &mut index,
            uc.file_content() as *const FileContent,
            0,
            uc.num_file_content(),
            cmp_lambda,
            eq_lambda,
        )
    };
    if index != usize::MAX {
        // SAFETY: `index != SIZE_MAX` means the search wrote a hit, which is an
        // index below `num_file_content`, so the offset lands on a live element of
        // `uc`'s file-content run; `p_data.get()` is that view's own write-capable
        // `ufbx_blob` place.
        unsafe { *p_data.get() = (*uc.file_content().add(index)).content };
    }
}

// ufbx.c:21490-21528 `ufbxi_resolve_file_content`
#[inline(never)]
pub(crate) fn resolve_file_content<'a>(uc: &'a Context) -> Result<(), Fail> {
    let initial_stack: usize = uc.tmp_stack_view().num_items();

    // SAFETY: walks the stored `videos` element-pointer run of the uc-owned scene
    // (`count` entries); the resolve/push helpers receive views over places inside
    // that same video, whose write-capable provenance backs every strblob, string,
    // and blob view minted over them.
    unsafe {
        // C: `ufbxi_for_ptr_list(ufbx_video, p_video, uc->scene.videos)`
        let mut p_video: *mut *mut Video = uc.scene_view().videos_view().data() as *mut *mut Video;
        let p_video_end: *mut *mut Video = add_ptr(p_video, uc.scene_view().videos_view().count());
        while p_video != p_video_end {
            let video: *mut Video = *p_video;
            resolve_filenames(
                uc,
                View::<Strblob>::from_ptr(&raw mut (*video).filename as *mut Strblob),
                View::<Strblob>::from_ptr(&raw mut (*video).absolute_filename as *mut Strblob),
                View::<Strblob>::from_ptr(&raw mut (*video).relative_filename as *mut Strblob),
                false,
            )?;
            resolve_filenames(
                uc,
                View::<Strblob>::from_ptr(&raw mut (*video).raw_filename as *mut Strblob),
                View::<Strblob>::from_ptr(&raw mut (*video).raw_absolute_filename as *mut Strblob),
                View::<Strblob>::from_ptr(&raw mut (*video).raw_relative_filename as *mut Strblob),
                true,
            )?;
            push_file_content(
                uc,
                View::<String>::from_ptr(&raw mut (*video).absolute_filename),
                View::<Blob>::from_ptr(&raw mut (*video).content),
            )?;
            p_video = p_video.add(1);
        }
    }

    // SAFETY: walks the stored `audio_clips` element-pointer run of the uc-owned
    // scene (`count` entries); each clip's props are reached through an
    // arena-anchored view with NUL-terminated literal names, and the resolve/push
    // helpers receive views over places inside that same clip, whose write-capable
    // provenance backs every strblob, string, and blob view minted over them.
    unsafe {
        // C: `ufbxi_for_ptr_list(ufbx_audio_clip, p_clip, uc->scene.audio_clips)`
        let mut p_clip: *mut *mut AudioClip =
            uc.scene_view().audio_clips_view().data() as *mut *mut AudioClip;
        let p_clip_end: *mut *mut AudioClip =
            add_ptr(p_clip, uc.scene_view().audio_clips_view().count());
        while p_clip != p_clip_end {
            let clip_view: &'a AudioClipView = AudioClipView::from_ptr(*p_clip);
            let clip: *mut AudioClip = clip_view.get();
            (*clip).absolute_filename =
                find_string_len(clip_view.props_view(), b"Path", EMPTY_STRING.0);
            (*clip).relative_filename =
                find_string_len(clip_view.props_view(), b"RelPath", EMPTY_STRING.0);
            (*clip).raw_absolute_filename =
                find_blob_len(clip_view.props_view(), b"Path", EMPTY_BLOB.0);
            (*clip).raw_relative_filename =
                find_blob_len(clip_view.props_view(), b"RelPath", EMPTY_BLOB.0);
            resolve_filenames(
                uc,
                View::<Strblob>::from_ptr(&raw mut (*clip).filename as *mut Strblob),
                View::<Strblob>::from_ptr(&raw mut (*clip).absolute_filename as *mut Strblob),
                View::<Strblob>::from_ptr(&raw mut (*clip).relative_filename as *mut Strblob),
                false,
            )?;
            resolve_filenames(
                uc,
                View::<Strblob>::from_ptr(&raw mut (*clip).raw_filename as *mut Strblob),
                View::<Strblob>::from_ptr(&raw mut (*clip).raw_absolute_filename as *mut Strblob),
                View::<Strblob>::from_ptr(&raw mut (*clip).raw_relative_filename as *mut Strblob),
                true,
            )?;
            push_file_content(
                uc,
                View::<String>::from_ptr(&raw mut (*clip).absolute_filename),
                View::<Blob>::from_ptr(&raw mut (*clip).content),
            )?;
            p_clip = p_clip.add(1);
        }
    }

    uc.set_num_file_content(uc.tmp_stack_view().num_items() - initial_stack);
    // Pops the `num_file_content` entries pushed by the loops above from uc's
    // tmp stack into uc's tmp buffer.
    // SAFETY: sorts that fresh non-null run with its own length.
    unsafe {
        uc.set_file_content(
            uc.tmp_view()
                .push_pop::<FileContent>(uc.tmp_stack_view(), uc.num_file_content()),
        );
        ufbxi_check!(uc, !uc.file_content().is_null(), "uc->file_content");
        sort_file_contents(uc, uc.file_content(), uc.num_file_content())?;
    }

    // SAFETY: walks the stored `videos` element-pointer run of the uc-owned scene
    // (`count` entries), minting views over each video's own filename/content
    // fields for the lookup.
    unsafe {
        // C: `ufbxi_for_ptr_list(ufbx_video, p_video, uc->scene.videos)`
        let mut p_video: *mut *mut Video = uc.scene_view().videos_view().data() as *mut *mut Video;
        let p_video_end: *mut *mut Video = add_ptr(p_video, uc.scene_view().videos_view().count());
        while p_video != p_video_end {
            let video: *mut Video = *p_video;
            fetch_file_content(
                uc,
                View::<String>::from_ptr(&raw mut (*video).absolute_filename),
                View::<Blob>::from_ptr(&raw mut (*video).content),
            );
            p_video = p_video.add(1);
        }
    }

    // SAFETY: walks the stored `audio_clips` element-pointer run of the uc-owned
    // scene (`count` entries), minting views over each clip's own filename/content
    // fields for the lookup.
    unsafe {
        // C: `ufbxi_for_ptr_list(ufbx_audio_clip, p_clip, uc->scene.audio_clips)`
        let mut p_clip: *mut *mut AudioClip =
            uc.scene_view().audio_clips_view().data() as *mut *mut AudioClip;
        let p_clip_end: *mut *mut AudioClip =
            add_ptr(p_clip, uc.scene_view().audio_clips_view().count());
        while p_clip != p_clip_end {
            let clip: *mut AudioClip = *p_clip;
            fetch_file_content(
                uc,
                View::<String>::from_ptr(&raw mut (*clip).absolute_filename),
                View::<Blob>::from_ptr(&raw mut (*clip).content),
            );
            p_clip = p_clip.add(1);
        }
    }

    Ok(())
}

// ufbx.c:21530-21546 `ufbxi_validate_indices`
#[inline(never)]
pub(crate) fn validate_indices(
    uc: &Context,
    indices_view: &ListView<u32>,
    max_index: usize,
) -> Result<(), Fail> {
    let indices: *mut List<u32> = indices_view.get();
    if max_index == 0 && uc.opts_view().index_error_handling() == IndexErrorHandling::Clamp {
        // SAFETY: `indices` is the view's own live `ufbx_uint32_list` header —
        // the attribute index list being validated.
        unsafe {
            (*indices).data = ptr::null_mut();
            (*indices).count = 0;
        }
        return Ok(());
    }

    // C: `ufbxi_nounroll ufbxi_for_list(uint32_t, p_ix, *indices)` — the
    // no-unroll pragma is optimizer-only and has no Rust analogue.
    // SAFETY: `indices` is live (see above), so its own list header is readable.
    // `data`/`count` describe one arena run.
    let (mut p_ix, p_ix_count) = unsafe { ((*indices).data as *mut u32, (*indices).count) };
    let p_ix_end: *mut u32 = add_ptr(p_ix, p_ix_count);
    while p_ix != p_ix_end {
        // SAFETY: `p_ix != p_ix_end`, so it addresses a live, initialized entry of
        // the index run.
        let ix: u32 = unsafe { *p_ix };
        // C: `ix >= max_index` — `ix` is promoted to `size_t` for the compare.
        if ix as usize >= max_index {
            // SAFETY: `p_ix` addresses a live, writable entry of the index run
            // (see above), and `ix` is the value just read from it.
            unsafe { fix_index(uc, p_ix, ix, max_index) }?;
        }
        // SAFETY: `p_ix != p_ix_end`, so the advance lands at or before the run's
        // one-past-the-end pointer.
        p_ix = unsafe { p_ix.add(1) };
    }

    Ok(())
}

// ufbx.c:21548-21559 `ufbxi_material_part_usage_less`
pub(crate) unsafe extern "C" fn material_part_usage_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    let parts: *mut MeshPart = user as *mut MeshPart;
    // SAFETY: as the comparator for the mesh's material-part usage order, `va`/`vb`
    // address live, initialized `u32` elements of that order array
    // (`unstable_sort` contract).
    let (a, b) = unsafe { (*(va as *const u32), *(vb as *const u32)) };
    // SAFETY: `user` is the mesh's own `material_parts` run, passed as the sort's
    // user data, and the order array holds exactly the indices `0..num_parts` of
    // that run.
    let (pa, pb) = unsafe { (parts.add(a as usize), parts.add(b as usize)) };
    // SAFETY: `pa`/`pb` address live, initialized parts of the run (see above).
    unsafe {
        if (*pa).face_indices.count == 0 || (*pb).face_indices.count == 0 {
            if (*pa).face_indices.count == (*pb).face_indices.count {
                return a < b;
            }
            return (*pa).face_indices.count > (*pb).face_indices.count;
        }
    }
    // SAFETY: `pa`/`pb` are live parts and neither `face_indices` run is empty
    // (checked above), so element `0` of each is live and initialized.
    unsafe { *(*pa).face_indices.data.add(0) < *(*pb).face_indices.data.add(0) }
}

// ufbx.c:21561-21621 `ufbxi_finalize_mesh_material`
#[inline(never)]
pub(crate) unsafe fn finalize_mesh_material(
    buf: &BufView,
    error: *mut Error,
    mesh: &View<Mesh>,
) -> Result<(), Fail> {
    let num_materials: usize = mesh.materials().count;
    let num_parts: usize = mesh.material_parts().count;
    let num_faces: usize = mesh.faces().count;

    let parts: *mut MeshPart = mesh.material_parts().data as *mut MeshPart;
    ufbx_assert!(
        parts.is_null()
            || (mesh.material_parts().count == num_materials)
            || (mesh.material_parts().count == 1 && num_materials == 0)
    );

    let face_material: *mut u32 = mesh.face_material().data as *mut u32;

    // Count the number of faces and triangles per material
    // C: `ufbxi_nounroll for (size_t i = 0; i < num_faces; i++)`
    for i in 0..num_faces {
        // SAFETY: `num_faces` is the length of the mesh's own face run, so
        // `i < num_faces` addresses a live, initialized face.
        let face: Face = unsafe { *mesh.faces().data.add(i) };
        let mut mat_ix: u32 = 0;

        if !face_material.is_null() {
            // SAFETY: a non-null `face_material` run is per-face, so it holds
            // `num_faces` writable `u32`s and `i < num_faces` stays inside it.
            unsafe {
                mat_ix = *face_material.add(i);
                if mat_ix as usize >= num_materials {
                    *face_material.add(i) = 0;
                    mat_ix = 0;
                }
            }
        }

        if !parts.is_null() {
            // SAFETY: the assert above pins a non-null `parts` run to
            // `material_parts.count` entries, with `count == num_materials` or
            // `count == 1` when `num_materials == 0`, and `mat_ix` was clamped
            // to `0` unless it is below `num_materials`, so the write lands
            // inside the run for every `count > 0`. `material_parts` is
            // allocated with `max(num_materials, 1)` entries, so the remaining
            // `count == num_materials == 0` shape the assert also admits does
            // not occur.
            unsafe { mesh_part_add_face(parts.add(mat_ix as usize), face.num_indices) };
        }
    }

    if !parts.is_null() {
        // Allocate per-material buffers (clear `num_faces` to 0 to re-use it as
        // an index when fetching the face indices).
        let mut part_index: u32 = 0;
        // C: `ufbxi_for(ufbx_mesh_part, part, parts, num_parts)`
        // SAFETY: a non-null `parts` addresses one contiguous arena run of the
        // mesh's `num_parts` live, initialized material parts.
        let part_views = unsafe { SliceViewIter::<MeshPart>::from_raw_parts(parts, num_parts) };
        for part in part_views {
            // C: `part->index = part_index++;` — assigns the pre-increment value.
            part.set_index(part_index);
            part_index = part_index.wrapping_add(1);
            part.face_indices_view().set_count(part.num_faces());
            // `buf` is the result buffer the finalized lists are pushed into.
            part.face_indices_view()
                .set_data(buf.push::<u32>(part.num_faces()));
            ufbxi_check_err!(
                unsafe { crate::native::error::ErrorView::from_ptr(error) },
                !part.face_indices_view().data().is_null(),
                "part->face_indices.data"
            );
            part.set_num_faces(0);
        }

        // Fetch the per-material face indices
        // C: `ufbxi_nounroll for (size_t i = 0; i < num_faces; i++)`
        for i in 0..num_faces {
            let mat_ix: u32 = if !face_material.is_null() {
                // SAFETY: a non-null `face_material` run is per-face, so it holds
                // `num_faces` `u32`s and `i < num_faces` stays inside it.
                unsafe { *face_material.add(i) }
            } else {
                0
            };
            if (mat_ix as usize) < num_parts {
                // SAFETY: `mat_ix < num_parts`, the length of the non-null `parts`
                // run, so the offset addresses a live, initialized material part.
                let part: &View<MeshPart> =
                    unsafe { View::<MeshPart>::from_ptr(parts.add(mat_ix as usize)) };
                // C: `part->face_indices.data[part->num_faces++] = (uint32_t)i;`
                // SAFETY: the part's `face_indices` run was pushed with its counted
                // face total and `num_faces` was reset to `0`, so it counts the
                // faces filled in so far and stays below that total.
                unsafe { *(part.face_indices().data as *mut u32).add(part.num_faces()) = i as u32 };
                part.set_num_faces(part.num_faces().wrapping_add(1));
            }
        }

        let usage_order = mesh.material_part_usage_order_view();
        usage_order.set_count(num_parts);
        // `buf` is the result buffer the finalized lists are pushed into.
        usage_order.set_data(buf.push::<u32>(num_parts));
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            !usage_order.data().is_null(),
            "mesh->material_part_usage_order.data"
        );
        for i in 0..num_parts {
            // SAFETY: the non-null usage-order run was pushed with `num_parts`
            // `u32`s, so `i < num_parts` stays inside it.
            unsafe { *(usage_order.data() as *mut u32).add(i) = i as u32 };
        }
        // SAFETY: the usage-order run holds `num_parts` `u32`s (see above),
        // `material_part_usage_less` is the matching comparator, and it reads its
        // user data as the `num_parts`-long `parts` run passed here.
        unsafe {
            unstable_sort(
                usage_order.data() as *mut c_void,
                num_parts,
                size_of::<u32>(),
                material_part_usage_less,
                parts as *mut c_void,
            )
        };
    }

    Ok(())
}

// ufbx.c:21623-21627 `ufbxi_anim_imp`
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

// SAFETY: `#[repr(C)]` with `refcount` leading, `ANIM_IMP_MAGIC` is the magic
// `ufbxi_get_imp(ufbxi_anim_imp, ...)` users check, `Payload` is the public
// struct at the pinned offset, and `header_parts` projects the two named
// fields of the passed `imp`.
unsafe impl crate::native::parse::ImpRecover for AnimImp {
    type Payload = Anim;
    const MAGIC: u32 = crate::native::allocator::ANIM_IMP_MAGIC;

    #[inline(always)]
    unsafe fn header_parts(imp: *mut Self) -> (*mut Refcount, *mut u32) {
        // SAFETY: the caller vouches `imp` addresses a live `AnimImp`, so these
        // field projections stay inside that allocation.
        unsafe { (&raw mut (*imp).refcount, &raw mut (*imp).magic) }
    }
}

// SAFETY: `parts` projects the three named fields of the passed `imp` (layout
// pinned by the `offset_of` assert above).
unsafe impl crate::native::parse::ImpHeader for AnimImp {
    #[inline(always)]
    unsafe fn parts(imp: *mut Self) -> (*mut Refcount, *mut Self::Payload, *mut u32) {
        // SAFETY: the caller vouches `imp` addresses a live `AnimImp`, so these
        // field projections stay inside that allocation.
        unsafe {
            (
                &raw mut (*imp).refcount,
                &raw mut (*imp).anim,
                &raw mut (*imp).magic,
            )
        }
    }
}

// ufbx.c:21629-21639 `ufbxi_push_anim`
#[inline(never)]
pub(crate) unsafe fn push_anim(
    uc: &Context,
    p_anim: *mut *mut Anim,
    layers: *mut *mut AnimLayer,
    num_layers: usize,
) -> Result<(), Fail> {
    let anim: *mut Anim = uc.result_view().push_zero::<Anim>(1);
    ufbxi_check!(uc, !anim.is_null(), "anim");

    // SAFETY: `anim` is the non-null one-element zeroed push just made into `uc`'s
    // result buffer, so it addresses writable storage for one `ufbx_anim`;
    // `layers`/`num_layers` describe the caller's layer-pointer run (fn contract).
    unsafe {
        (*anim).layers.data = layers as *const Ref<AnimLayer>;
        (*anim).layers.count = num_layers;
    }

    // SAFETY: `p_anim` points to a live, writable `ufbx_anim*` slot (fn contract).
    unsafe { *p_anim = anim };
    Ok(())
}

// ufbx.c:21641-22624 `ufbxi_finalize_scene`
// The single ~985-line pass that turns the parsed element/connection scratch
// into the public `ufbx_scene` graph. Split into no helpers upstream, so it is
// ported as one function.
#[inline(never)]
// Stays `unsafe fn`: ~1900 lines of raw arena work (push/pop runs, element
// pointer patching, per-element fetch/sort passes) with ~930 residual raw
// operations threaded through hundreds of escaping locals — no honest
// clustering exists, only a whole-body wrap.
pub(crate) unsafe fn finalize_scene<'a>(uc: &'a Context) -> Result<(), Fail> {
    let num_elements: usize = uc.num_elements() as usize;

    uc.scene_view().elements_view().set_count(num_elements);
    uc.scene_view()
        .elements_view()
        .set_data(uc.result_view().push::<*mut Element>(num_elements) as *const Ref<Element>);
    ufbxi_check!(
        uc,
        !uc.scene_view().elements_view().data().is_null(),
        "uc->scene.elements.data"
    );

    uc.scene_view()
        .metadata_view()
        .set_element_buffer_size(uc.tmp_element_byte_offset());
    let element_data: *mut u8 = uc
        .result_view()
        .push_pop::<u64>(uc.tmp_elements_view(), uc.tmp_element_byte_offset() / 8)
        as *mut u8;
    ufbxi_check!(uc, !element_data.is_null(), "element_data");

    // C reads `uc->tmp_element_offsets.num_items` as the `ufbxi_push_pop()`
    // count argument; hoisted to a local so the `&mut` borrow of the same
    // buffer does not overlap the read.
    let num_element_offsets: usize = uc.tmp_element_offsets_view().num_items();
    let element_offsets: *mut usize = uc
        .tmp_view()
        .push_pop::<usize>(uc.tmp_element_offsets_view(), num_element_offsets);
    // SAFETY: `tmp_element_offsets_mut_ptr` hands out `uc`'s own live element-
    // offset buffer.
    unsafe { buf_free(uc.tmp_element_offsets_mut_ptr()) };
    ufbxi_check!(uc, !element_offsets.is_null(), "element_offsets");
    // The offsets are the one `ufbxi_push_pop`-materialized block popped above,
    // one `size_t` per element: `ufbxi_push_element_size` pushes exactly one
    // offset per `uc->num_elements++` (ufbx.c:12357-12360), so the run holds
    // `num_elements` entries and every element id indexes it. Nothing writes it
    // while the walk below reads it.
    // SAFETY: the run is non-null (checked above), contiguous, and holds
    // `num_element_offsets` initialized `size_t`s that stay live for the walk.
    let element_offsets_run: &[usize] =
        unsafe { slice_from_ptr(element_offsets as *const usize, num_element_offsets) };
    // C stores into `uc->scene.elements.data[i]`; the destination run is derived
    // from the list base, which the check above proved non-null.
    let scene_elements: *mut *mut Element =
        uc.scene_view().elements_view().data() as *mut *mut Element;
    for i in 0..num_elements {
        // SAFETY: offset `i` addresses that element's header inside the
        // `element_data` blob, which was popped with `tmp_element_byte_offset()`
        // bytes, so the sum heads a live, initialized `ufbx_element`.
        let element: *mut Element =
            unsafe { element_data.add(element_offsets_run[i]) } as *mut Element;

        // SAFETY: `element` heads a live `ufbx_element` in the arena element blob
        // (see above), so it anchors an `ElementView`.
        let element_view: &ElementView = unsafe { ElementView::from_ptr(element) };
        if element_view.type_() == ElementType::Node {
            // SAFETY: `type_ == Node`, so `element` heads a live `ufbx_node`; the
            // node view is minted from the blob pointer, not from the
            // header-sized element view.
            let node: &NodeView = unsafe { NodeView::from_ptr(element as *mut Node) };
            if node.scale_helper().is_some() {
                let extra: *mut NodeExtra =
                    get_element_extra(uc, node.element().element_id()) as *mut NodeExtra;
                ufbx_assert!(!extra.is_null());
                // SAFETY: `extra` is the non-null per-element extra just fetched,
                // so it heads a live `ufbxi_node_extra`.
                let extra_view: &View<NodeExtra> = unsafe { View::<NodeExtra>::from_ptr(extra) };
                // SAFETY: `scale_helper_id` is an element index below
                // `num_elements`, so the byte offset it selects addresses that
                // element's header inside the blob, and that element is a
                // `ufbx_node` (the scale helper `ufbxi_setup_scale_helper` made).
                node.set_scale_helper(unsafe {
                    opt_ref(
                        element_data.add(element_offsets_run[extra_view.scale_helper_id() as usize])
                            as *mut Node,
                    )
                });
            }
        }

        // SAFETY: the scene's element array was pushed with `num_elements` slots
        // and checked non-null above, so `i < num_elements` stays inside it.
        unsafe { *scene_elements.add(i) = element };
    }

    uc.scene_view().elements_view().set_count(num_elements);
    // SAFETY: the two accessors hand out `uc`'s own live element-offset and
    // element buffers.
    unsafe {
        buf_free(uc.tmp_element_offsets_mut_ptr());
        buf_free(uc.tmp_elements_mut_ptr());
    }

    uc.set_tmp_element_flag(uc.tmp_view().push_zero::<u8>(num_elements));
    ufbxi_check!(uc, !uc.tmp_element_flag().is_null(), "uc->tmp_element_flag");

    uc.scene_view()
        .metadata_view()
        // SAFETY: `scene_props_ptr` addresses the scene metadata's own live
        // `ufbx_props`, and the name is a NUL-terminated literal.
        .set_original_file_path(unsafe {
            find_string_len(
                PropsView::from_ptr(uc.scene_view().metadata_view().scene_props_ptr() as *mut Props),
                b"DocumentUrl",
                EMPTY_STRING.0)
        });
    uc.scene_view()
        .metadata_view()
        // SAFETY: as above.
        .set_raw_original_file_path(unsafe {
            find_blob_len(
                PropsView::from_ptr(uc.scene_view().metadata_view().scene_props_ptr() as *mut Props),
                b"DocumentUrl",
                EMPTY_BLOB.0,
            )
        });

    // Resolve and add the connections to elements
    resolve_connections(uc)?;
    add_connections_to_elements(uc)?;
    linearize_nodes(uc)?;

    for type_ in 0..ELEMENT_TYPE_COUNT {
        let num_typed: usize = uc.tmp_typed_element_offsets_at(type_).num_items();
        let typed_offsets: *mut usize = uc
            .tmp_view()
            .push_pop::<usize>(uc.tmp_typed_element_offsets_at(type_), num_typed);
        // SAFETY: the accessor hands out `uc`'s own live typed-element-offset
        // buffer for `type_`.
        unsafe { buf_free(uc.tmp_typed_element_offsets_mut_ptr(type_)) };
        ufbxi_check!(uc, !typed_offsets.is_null(), "typed_offsets");
        // SAFETY: the run is the non-null `ufbxi_push_pop`-materialized block
        // popped above, holding `num_typed` initialized byte offsets; nothing
        // writes it while the fill below reads it.
        let typed_offsets_run: &[usize] =
            unsafe { slice_from_ptr(typed_offsets as *const usize, num_typed) };

        let typed_elems: &RefListView<Element> = uc.scene_view().elements_by_type_at(type_);
        typed_elems.set_count(num_typed);
        typed_elems
            .set_data(uc.result_view().push::<*mut Element>(num_typed) as *const Ref<Element>);
        ufbxi_check!(uc, !typed_elems.data().is_null(), "typed_elems->data");

        // C stores into `typed_elems->data[i]`; the destination run is derived
        // from the list base, which the check above proved non-null.
        let typed_data: *mut *mut Element = typed_elems.data() as *mut *mut Element;
        for i in 0..num_typed {
            // SAFETY: the run just pushed into `typed_elems.data` is non-null and
            // `num_typed` slots long, and offset `i` addresses that element's
            // header inside the `element_data` blob.
            unsafe { *typed_data.add(i) = element_data.add(typed_offsets_run[i]) as *mut Element };
        }

        // SAFETY: the accessor hands out `uc`'s own live typed-element-offset
        // buffer for `type_`.
        unsafe { buf_free(uc.tmp_typed_element_offsets_mut_ptr(type_)) };
    }

    // Create named elements
    uc.scene_view()
        .elements_by_name_view()
        .set_count(num_elements);
    uc.scene_view()
        .elements_by_name_view()
        .set_data(uc.result_view().push::<NameElement>(num_elements));
    ufbxi_check!(
        uc,
        !uc.scene_view().elements_by_name_view().data().is_null(),
        "uc->scene.elements_by_name.data"
    );

    // SAFETY: the scene's element array is the result-buffer run filled by the
    // pass above: `num_elements` initialized, non-null `Ref<ufbx_element>` slots
    // that stay live and unwritten for this walk.
    let element_refs: &[Ref<Element>] =
        unsafe { slice_from_ptr(uc.scene_view().elements_view().data(), num_elements) };
    // C writes `&uc->scene.elements_by_name.data[i]`; the destination run is
    // derived from the list base, which the check above proved non-null.
    let name_elems: *mut NameElement =
        uc.scene_view().elements_by_name_view().data() as *mut NameElement;

    for i in 0..num_elements {
        let elem: Ref<Element> = element_refs[i];
        // SAFETY: the slot names a live element header in the arena element blob
        // (see above), so it anchors an `ElementView`.
        let elem_view: &ElementView = unsafe { ElementView::from_ptr(elem.ptr()) };
        // SAFETY: the `elements_by_name` run was pushed with `num_elements` slots
        // and checked non-null above, so `i < num_elements` stays inside it.
        let name_elem: &View<NameElement> =
            unsafe { View::<NameElement>::from_ptr(name_elems.add(i)) };

        name_elem.set_name(elem_view.name());
        name_elem.set_type(elem_view.type_());
        name_elem.set_internal_key(get_name_key(elem_view.name_view().bytes()));
        name_elem.set_element(elem);
    }

    // SAFETY: the `elements_by_name` run is non-null and `num_elements` entries
    // long, all initialized by the loop above.
    unsafe {
        sort_name_elements(
            uc,
            uc.scene_view().elements_by_name_view().data() as *mut NameElement,
            num_elements,
        )
    }?;

    // Setup node children arrays and attribute pointers/lists
    // C: `ufbxi_for_ptr_list(ufbx_node, p_node, uc->scene.nodes)`
    let scene_nodes: &RefListView<Node> = uc.scene_view().nodes_view();
    for node_ix in 0..scene_nodes.count() {
        let node: &NodeView = scene_nodes.at(node_ix);
        // C keeps the ITERATOR itself as `parent->children.data`, so the slot
        // pointer is derived from the list base rather than from `node`.
        let p_node: *const Ref<Node> = scene_nodes.data().wrapping_add(node_ix);
        // SAFETY: `node_ix < count()`, so the slot is inside the scene's own
        // node run and holds the same non-null `Ref<ufbx_node>` that `at` read.
        let node_ref: Ref<Node> = unsafe { *p_node };
        if let Some(parent_ref) = node.parent() {
            // SAFETY: a non-null `parent` names another live `ufbx_node` of the
            // same scene.
            let parent: &NodeView = unsafe { NodeView::from_ptr(parent_ref.ptr()) };
            parent
                .children_view()
                .set_count(parent.children_view().count() + 1);
            if parent.children_view().data().is_null() {
                // `linearize_nodes` ordered the node-pointer run so each parent's
                // children are contiguous, and `p_node` is the first of them.
                parent.children_view().set_data(p_node);
            }

            if node.is_geometry_transform_helper() {
                parent.set_geometry_transform_helper(Some(node_ref));
            }

            // Force top-level nodes to have `UFBX_INHERIT_MODE_NORMAL` to make unit scaling work.
            if parent.is_root()
                && uc.opts_view().space_conversion() == SpaceConversion::TransformRoot
                && uc.opts_view().inherit_mode_handling() == InheritModeHandling::Preserve
            {
                node.set_original_inherit_mode(InheritMode::Normal);
                node.set_inherit_mode(InheritMode::Normal);
            }

            // RrSs nodes inherit scale from their parent, Rrs ignore the scale of
            // their _immediate_ parent, potentially multiple if chained.
            if node.original_inherit_mode() == InheritMode::ComponentwiseScale {
                node.set_inherit_scale_node(Some(parent_ref));
            } else if node.original_inherit_mode() == InheritMode::IgnoreParentScale {
                node.set_inherit_scale_node(parent.inherit_scale_node());
            }
        }

        let conns: List<Connection> = find_dst_connections(node.element(), None);

        // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
        // SAFETY: `conns` is the contiguous subrange of this element's
        // `push_pop`-materialized connection run that `find_dst_connections`
        // returned, live and unwritten for the walk.
        for conn in unsafe {
            SliceViewIter::<Connection>::from_raw_parts(conns.data as *mut Connection, conns.count)
        } {
            let elem_ref: Ref<Element> = conn.src();
            let elem: *mut Element = elem_ref.ptr();
            // SAFETY: a connection's `src` names a live element of the scene, so
            // it anchors an `ElementView`.
            let elem_view: &ElementView = unsafe { ElementView::from_ptr(elem) };
            let type_: ElementType = elem_view.type_();
            if !(type_ as u32 >= ELEMENT_TYPE_FIRST_ATTRIB
                && type_ as u32 <= ELEMENT_TYPE_LAST_ATTRIB)
            {
                continue;
            }

            // C: `size_t index = node->all_attribs.count++;` — the
            // pre-increment value.
            let index: usize = node.all_attribs_view().count();
            node.all_attribs_view().set_count(index + 1);
            if index == 0 {
                node.set_attrib(Some(elem_ref));
                node.set_attrib_type(type_);
            } else {
                if index == 1 {
                    ufbxi_check!(
                        uc,
                        // SAFETY: `attrib_ptr` addresses the live node's own single
                        // element-pointer field, which is niche-packed to the bare
                        // `ufbx_element*` the copy reads.
                        !unsafe {
                            uc.tmp_stack_view().push_copy_raw::<*mut Element>(1,
                                node.attrib_ptr() as *const *mut Element,
                            )
                        }
                        .is_null(),
                        "((ufbx_element**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_element*), (1), (&node->attrib)))"
                    );
                }
                ufbxi_check!(
                    uc,
                    !uc.tmp_stack_view().push_copy_ref(&elem).is_null(),
                    "((ufbx_element**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_element*), (1), (&elem)))"
                );
            }

            match elem_view.type_() {
                // SAFETY: the arm pins the type, so the non-null `src` element
                // header is the head of a live `ufbx_mesh`.
                ElementType::Mesh => {
                    node.set_mesh(Some(unsafe { Ref::from_ptr(elem as *mut Mesh) }))
                }
                // SAFETY: the arm pins the type, so the non-null `src` element
                // header is the head of a live `ufbx_light`.
                ElementType::Light => {
                    node.set_light(Some(unsafe { Ref::from_ptr(elem as *mut Light) }))
                }
                // SAFETY: the arm pins the type, so the non-null `src` element
                // header is the head of a live `ufbx_camera`.
                ElementType::Camera => {
                    node.set_camera(Some(unsafe { Ref::from_ptr(elem as *mut Camera) }))
                }
                // SAFETY: the arm pins the type, so the non-null `src` element
                // header is the head of a live `ufbx_bone`.
                ElementType::Bone => {
                    node.set_bone(Some(unsafe { Ref::from_ptr(elem as *mut Bone) }))
                }
                _ => { /* No shorthand */ }
            }
        }

        if node.all_attribs_view().count() > 1 {
            node.all_attribs_view().set_data(
                uc.result_view()
                    .push_pop::<*mut Element>(uc.tmp_stack_view(), node.all_attribs_view().count())
                    as *const Ref<Element>,
            );
            ufbxi_check!(
                uc,
                !node.all_attribs_view().data().is_null(),
                "node->all_attribs.data"
            );
        } else if node.all_attribs_view().count() == 1 {
            // The node's own single element-pointer field IS the one-entry list.
            node.all_attribs_view()
                .set_data(node.attrib_ptr() as *const Ref<Element>);
        }

        // SAFETY: `node.element_raw()` addresses that element's
        // header with whole-struct provenance.
        unsafe {
            fetch_dst_elements(
                uc,
                node.materials_view().as_element_list(),
                node.element_raw(),
                false,
                false,
                None,
                ElementType::Material,
            )
        }?;
    }

    // Resolve bind pose bones that don't use the normal connection system
    // C: `ufbxi_for_ptr_list(ufbx_pose, p_pose, uc->scene.poses)`
    let scene_poses: &RefListView<Pose> = uc.scene_view().poses_view();
    for pose_ix in 0..scene_poses.count() {
        let pose: &View<Pose> = scene_poses.at(pose_ix);
        // SAFETY: `pose_ix < count()`, so the slot is inside the scene's own pose
        // run and holds the same non-null `Ref<ufbx_pose>` that `at` read.
        let pose_ref: Ref<Pose> = unsafe { *scene_poses.data().add(pose_ix) };

        // HACK: Transport `ufbxi_tmp_bone_pose` array through the `ufbx_bone_pose` pointer
        let num_bones: usize = pose.bone_poses_view().count();
        let tmp_poses: *mut TmpBonePose = pose.bone_poses_view().data() as *mut TmpBonePose;
        pose.bone_poses_view()
            .set_data(uc.result_view().push::<BonePose>(num_bones));
        ufbxi_check!(
            uc,
            !pose.bone_poses_view().data().is_null(),
            "pose->bone_poses.data"
        );

        // Filter only found bones
        pose.bone_poses_view().set_count(0);
        // SAFETY: the stashed run is the one `ufbxi_push_pop`-materialized block
        // the pose parse left in this field (ufbx.c:14683), `num_bones`
        // contiguous initialized records, live and unwritten for the walk.
        for tmp_pose in
            unsafe { SliceViewIter::<TmpBonePose>::from_raw_parts(tmp_poses, num_bones) }
        {
            let elem: *mut Element = find_element_by_fbx_id(uc, tmp_pose.bone_fbx_id());
            if elem.is_null() {
                continue;
            }
            // SAFETY: a non-null `elem` names a live element of the scene, so it
            // anchors an `ElementView`.
            let elem_view: &ElementView = unsafe { ElementView::from_ptr(elem) };
            if elem_view.type_() != ElementType::Node {
                continue;
            }

            // SAFETY: `type_ == Node`, so `elem` heads a live `ufbx_node`; the node
            // view is minted from the element pointer, not from the header-sized
            // element view.
            let node: &NodeView = unsafe { NodeView::from_ptr(elem as *mut Node) };
            // C: `&pose->bone_poses.data[pose->bone_poses.count++]`
            let bone_ix: usize = pose.bone_poses_view().count();
            // SAFETY: the `bone_poses` run was pushed with `num_bones` entries and
            // `bone_poses.count` counts the bones filled in so far, one per loop
            // iteration, so it stays below `num_bones`; the slot is derived from
            // the list base.
            let bone: &View<BonePose> = unsafe {
                View::<BonePose>::from_ptr(
                    (pose.bone_poses_view().data() as *mut BonePose).add(bone_ix),
                )
            };
            pose.bone_poses_view().set_count(bone_ix + 1);
            // SAFETY: `elem` is the non-null element checked above, whose type
            // pins it to a live `ufbx_node`.
            bone.set_bone_node(unsafe { Ref::from_ptr(elem as *mut Node) });
            bone.set_bone_to_world(tmp_pose.bone_to_world());

            if pose.is_bind_pose() {
                if node.bind_pose().is_none() {
                    node.set_bind_pose(Some(pose_ref));
                }

                let node_conns: List<Connection> = find_src_connections(elem_view, None);
                // C: `ufbxi_for_list(ufbx_connection, conn, node_conns)`
                // SAFETY: `node_conns` is the contiguous subrange of this element's
                // `push_pop`-materialized connection run that
                // `find_src_connections` returned, live and unwritten for the walk.
                for conn in unsafe {
                    SliceViewIter::<Connection>::from_raw_parts(
                        node_conns.data as *mut Connection,
                        node_conns.count,
                    )
                } {
                    let dst_ref: Ref<Element> = conn.dst();
                    // SAFETY: a connection's `dst` names a live element of the
                    // scene, so it anchors an `ElementView`.
                    let dst_view: &ElementView = unsafe { ElementView::from_ptr(dst_ref.ptr()) };
                    if dst_view.type_() != ElementType::SkinCluster {
                        continue;
                    }
                    // SAFETY: the check pins the destination's type, so its
                    // non-null header is the head of a live `ufbx_skin_cluster`.
                    let cluster: &View<SkinCluster> =
                        unsafe { View::<SkinCluster>::from_ptr(dst_ref.ptr() as *mut SkinCluster) };
                    // SAFETY: `bind_to_world_ptr` addresses the cluster's own
                    // matrix field, which `ufbxi_matrix_all_zero` reads (fn
                    // contract).
                    if unsafe { matrix_all_zero(cluster.bind_to_world_ptr()) } {
                        cluster.set_bind_to_world(bone.bone_to_world());
                    }
                }
            }
        }
        sort_bone_poses(uc, pose)?;
    }

    // Fetch pointers that may break elements

    // Setup node attribute instances
    // C: `for (int type = UFBX_ELEMENT_TYPE_FIRST_ATTRIB; type <= UFBX_ELEMENT_TYPE_LAST_ATTRIB; type++)`
    let mut attrib_type: u32 = ELEMENT_TYPE_FIRST_ATTRIB;
    while attrib_type <= ELEMENT_TYPE_LAST_ATTRIB {
        let typed_elems: &RefListView<Element> =
            uc.scene_view().elements_by_type_at(attrib_type as usize);
        // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, uc->scene.elements_by_type[type])`
        for elem_ix in 0..typed_elems.count() {
            let elem: &ElementView = typed_elems.at(elem_ix);
            // SAFETY: `elem.get()` addresses that element's live header, minted
            // from the scene's element-ref list; `search_node` is `false` here
            // (C: ufbx.c:21832), so the walk never leaves the `ufbx_element`
            // header and the view's header-only provenance suffices.
            unsafe {
                fetch_src_elements(
                    uc,
                    elem.instances_view().as_element_list(),
                    elem.get(),
                    false,
                    true,
                    None,
                    ElementType::Node,
                )
            }?;
        }
        attrib_type += 1;
    }

    let search_node: bool = uc.version() < 7000;

    // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, uc->scene.skin_clusters)`
    let scene_skin_clusters: &RefListView<SkinCluster> = uc.scene_view().skin_clusters_view();
    for cluster_ix in 0..scene_skin_clusters.count() {
        let cluster: &View<SkinCluster> = scene_skin_clusters.at(cluster_ix);
        // SAFETY: `element_raw` addresses the cluster's own element header, which
        // is what `ufbxi_fetch_dst_element` reads (fn contract); the fetched
        // destination is null or a live `ufbx_node`.
        cluster.set_bone_node(unsafe {
            opt_ref(
                fetch_dst_element(cluster.element_raw(), false, ptr::null(), ElementType::Node)
                    as *mut Node,
            )
        });
    }

    // C: `ufbxi_for_ptr_list(ufbx_skin_deformer, p_skin, uc->scene.skin_deformers)`
    let scene_skin_deformers: &RefListView<SkinDeformer> = uc.scene_view().skin_deformers_view();
    for skin_ix in 0..scene_skin_deformers.count() {
        let skin_view: &View<SkinDeformer> = scene_skin_deformers.at(skin_ix);
        // SAFETY: `skin_view.element_raw()` addresses that element's
        // header with whole-struct provenance.
        unsafe {
            fetch_dst_elements(
                uc,
                skin_view.clusters_view().as_element_list(),
                skin_view.element_raw(),
                false,
                true,
                None,
                ElementType::SkinCluster,
            )
        }?;

        // Remove clusters without a valid `bone`
        if !uc.opts_view().connect_broken_elements() {
            let clusters: &RefListView<SkinCluster> = skin_view.clusters_view();
            let num_clusters: usize = clusters.count();
            // C compacts `skin->clusters.data[i - num_broken] = ...[i]`; both ends
            // are derived from the list base.
            let clusters_data: *mut Ref<SkinCluster> = clusters.data() as *mut Ref<SkinCluster>;
            let mut num_broken: usize = 0;
            for i in 0..num_clusters {
                if clusters.at(i).bone_node().is_none() {
                    num_broken += 1;
                } else if num_broken > 0 {
                    // SAFETY: `i` indexes the deformer's own cluster run and
                    // `num_broken <= i`, so the compacted destination is inside it
                    // too.
                    unsafe { *clusters_data.add(i - num_broken) = *clusters_data.add(i) };
                }
            }
            clusters.set_count(num_clusters - num_broken);
        }

        let mut total_weights: usize = 0;
        // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, skin->clusters)`
        let clusters: &RefListView<SkinCluster> = skin_view.clusters_view();
        for cluster_ix in 0..clusters.count() {
            let num_weights: usize = clusters.at(cluster_ix).num_weights();
            ufbxi_check!(
                uc,
                usize::MAX - total_weights > num_weights,
                "SIZE_MAX - total_weights > cluster->num_weights"
            );
            total_weights += num_weights;
        }

        let mut num_vertices: usize = 0;

        // Iterate through meshes so we can pad the vertices to the largest one
        {
            let conns: List<Connection> = find_src_connections(skin_view.element(), None);
            // C: `ufbxi_for_list(ufbx_connection, conn, conns)`
            // SAFETY: `conns` is the contiguous subrange of this element's
            // `push_pop`-materialized connection run that `find_src_connections`
            // returned, live and unwritten for the walk.
            for conn in unsafe {
                SliceViewIter::<Connection>::from_raw_parts(
                    conns.data as *mut Connection,
                    conns.count,
                )
            } {
                let mut mesh: Option<Ref<Mesh>> = None;
                if conn.dst_prop_view().length() > 0 {
                    continue;
                }
                let dst_ref: Ref<Element> = conn.dst();
                // SAFETY: a connection's `dst` names a live element of the scene,
                // so it anchors an `ElementView`.
                let dst_view: &ElementView = unsafe { ElementView::from_ptr(dst_ref.ptr()) };
                if dst_view.type_() == ElementType::Mesh {
                    // SAFETY: the check pins the type, so the non-null header is
                    // the head of a live `ufbx_mesh`.
                    mesh = Some(unsafe { Ref::from_ptr(dst_ref.ptr() as *mut Mesh) });
                } else if dst_view.type_() == ElementType::Node {
                    // SAFETY: the check pins the type, so the non-null header is
                    // the head of a live `ufbx_node`.
                    let mut node: &NodeView =
                        unsafe { NodeView::from_ptr(dst_ref.ptr() as *mut Node) };
                    if let Some(helper) = node.geometry_transform_helper() {
                        // SAFETY: a non-null helper names another live `ufbx_node`
                        // of the same scene.
                        node = unsafe { NodeView::from_ptr(helper.ptr()) };
                    }
                    mesh = node.mesh();
                }
                let Some(mesh) = mesh else {
                    continue;
                };
                // SAFETY: `mesh` names a live `ufbx_mesh` of the scene (see above).
                let mesh_view: &View<Mesh> = unsafe { View::<Mesh>::from_ptr(mesh.ptr()) };
                num_vertices = max_sz(num_vertices, mesh_view.num_vertices());
            }
        }

        if !uc.opts_view().skip_skin_vertices() {
            skin_view.vertices_view().set_count(num_vertices);
            skin_view
                .vertices_view()
                .set_data(uc.result_view().push_zero::<SkinVertex>(num_vertices));
            ufbxi_check!(
                uc,
                !skin_view.vertices_view().data().is_null(),
                "skin->vertices.data"
            );

            skin_view.weights_view().set_count(total_weights);
            skin_view
                .weights_view()
                .set_data(uc.result_view().push_zero::<SkinWeight>(total_weights));
            ufbxi_check!(
                uc,
                !skin_view.weights_view().data().is_null(),
                "skin->weights.data"
            );

            let retain_all: bool = !uc.opts_view().clean_skin_weights();

            // The two runs pushed above, reached as lists: `at()` bounds every
            // element access against the count each was given.
            let skin_vertices: &ListView<SkinVertex> = skin_view.vertices_view();
            let skin_weights: &ListView<SkinWeight> = skin_view.weights_view();

            // Count the number of weights per vertex
            // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, skin->clusters)`
            let clusters: &RefListView<SkinCluster> = skin_view.clusters_view();
            for cluster_ix in 0..clusters.count() {
                let cluster: &View<SkinCluster> = clusters.at(cluster_ix);
                for i in 0..cluster.num_weights() {
                    // The cluster's `vertices`/`weights` runs both hold
                    // `num_weights` entries (ufbx.c:14034, 16103), so the list
                    // index is in bounds.
                    let vertex: u32 = cluster.vertices()[i];
                    if (vertex as usize) < num_vertices
                        && (retain_all || cluster.weights()[i] > 0.0)
                    {
                        let skin_vertex: &View<SkinVertex> = skin_vertices.at(vertex as usize);
                        skin_vertex.set_num_weights(skin_vertex.num_weights().wrapping_add(1));
                    }
                }
            }

            let default_dq: Real = if skin_view.skinning_method() == SkinningMethod::DualQuaternion
            {
                1.0f32 as Real
            } else {
                0.0f32 as Real
            };

            // Prefix sum to assign the vertex weight offsets and set up default DQ values
            let mut offset: u32 = 0;
            let mut max_weights: u32 = 0;
            for i in 0..num_vertices {
                let skin_vertex: &View<SkinVertex> = skin_vertices.at(i);
                skin_vertex.set_weight_begin(offset);
                skin_vertex.set_dq_weight(default_dq);
                let num_weights: u32 = skin_vertex.num_weights();
                offset = offset.wrapping_add(num_weights);
                skin_vertex.set_num_weights(0);

                if num_weights > max_weights {
                    max_weights = num_weights;
                }
            }
            ufbx_assert!(offset as usize <= total_weights);
            skin_view.set_max_weights_per_vertex(max_weights as usize);

            // Copy the DQ weights to vertices
            for i in 0..skin_view.num_dq_weights() {
                // The skin's `dq_vertices`/`dq_weights` runs both hold
                // `num_dq_weights` entries (ufbx.c:14014-14018), so the list
                // index is in bounds.
                let vertex: u32 = skin_view.dq_vertices()[i];
                if (vertex as usize) < num_vertices {
                    skin_vertices
                        .at(vertex as usize)
                        .set_dq_weight(skin_view.dq_weights()[i]);
                }
            }

            // Copy the weights to vertices
            let mut cluster_index: u32 = 0;
            // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, skin->clusters)`
            let clusters: &RefListView<SkinCluster> = skin_view.clusters_view();
            for cluster_ix in 0..clusters.count() {
                let cluster: &View<SkinCluster> = clusters.at(cluster_ix);
                for i in 0..cluster.num_weights() {
                    // The cluster's `vertices`/`weights` runs both hold
                    // `num_weights` entries (see the counting pass above).
                    let vertex: u32 = cluster.vertices()[i];
                    if (vertex as usize) < num_vertices
                        && (retain_all || cluster.weights()[i] > 0.0)
                    {
                        // C: `skin->vertices.data[vertex].num_weights++` — the
                        // pre-increment value.
                        let skin_vertex: &View<SkinVertex> = skin_vertices.at(vertex as usize);
                        let local_index: u32 = skin_vertex.num_weights();
                        skin_vertex.set_num_weights(local_index.wrapping_add(1));
                        let index: u32 = skin_vertex.weight_begin().wrapping_add(local_index);
                        // The counting pass gave each vertex exactly
                        // `num_weights` slots starting at its `weight_begin` and
                        // `local_index` counts the weights written for this
                        // vertex so far, so `index` stays inside the
                        // `total_weights`-long `skin_weights` run.
                        let skin_weight: &View<SkinWeight> = skin_weights.at(index as usize);
                        skin_weight.set_cluster_index(cluster_index);
                        skin_weight.set_weight(cluster.weights()[i]);
                    }
                }
                cluster_index = cluster_index.wrapping_add(1);
            }

            // Sort the vertex weights by descending weight value
            // SAFETY: the counting pass above gave each `vertices` entry its own
            // `num_weights <= max_weights_per_vertex` slots starting at
            // `weight_begin` inside the `total_weights`-long `weights` run, so
            // every vertex run named here lies inside `weights`.
            unsafe { sort_skin_weights(uc, skin_view) }?;
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_blend_deformer, p_blend, uc->scene.blend_deformers)`
    let scene_blend_deformers: &RefListView<BlendDeformer> = uc.scene_view().blend_deformers_view();
    for blend_ix in 0..scene_blend_deformers.count() {
        let blend: &View<BlendDeformer> = scene_blend_deformers.at(blend_ix);
        // SAFETY: `blend.element_raw()` addresses that element's
        // header with whole-struct provenance.
        unsafe {
            fetch_dst_elements(
                uc,
                blend.channels_view().as_element_list(),
                blend.element_raw(),
                false,
                true,
                None,
                ElementType::BlendChannel,
            )
        }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_cache_deformer, p_deformer, uc->scene.cache_deformers)`
    let scene_cache_deformers: &RefListView<CacheDeformer> = uc.scene_view().cache_deformers_view();
    for deformer_ix in 0..scene_cache_deformers.count() {
        let deformer_view: &CacheDeformerView = scene_cache_deformers.at(deformer_ix);
        deformer_view.set_channel(find_string_len(
            deformer_view.props_view(),
            b"ChannelName",
            EMPTY_STRING.0,
        ));
        // SAFETY: `element_raw` addresses the deformer's own element header,
        // which is what `ufbxi_fetch_dst_element` reads (fn contract); the
        // fetched destination is null or a live `ufbx_cache_file`.
        deformer_view.set_file(unsafe {
            opt_ref(fetch_dst_element(
                deformer_view.element_raw(),
                false,
                ptr::null(),
                ElementType::CacheFile,
            ) as *mut CacheFile)
        });
    }

    // C: `ufbxi_for_ptr_list(ufbx_cache_file, p_cache, uc->scene.cache_files)`
    let scene_cache_files: &RefListView<CacheFile> = uc.scene_view().cache_files_view();
    for cache_ix in 0..scene_cache_files.count() {
        let cache_view: &CacheFileView = scene_cache_files.at(cache_ix);

        cache_view.set_absolute_filename(find_string_len(
            cache_view.props_view(),
            b"CacheAbsoluteFileName",
            EMPTY_STRING.0,
        ));
        cache_view.set_relative_filename(find_string_len(
            cache_view.props_view(),
            b"CacheFileName",
            EMPTY_STRING.0,
        ));

        cache_view.set_raw_absolute_filename(find_blob_len(
            cache_view.props_view(),
            b"CacheAbsoluteFileName",
            EMPTY_BLOB.0,
        ));
        cache_view.set_raw_relative_filename(find_blob_len(
            cache_view.props_view(),
            b"CacheFileName",
            EMPTY_BLOB.0,
        ));

        let type_: i64 = api_find_int_len(cache_view.props_view(), b"CacheFileType", 0);
        if type_ >= 0 && type_ <= CacheFileFormat::Mc as i64 {
            // C: `(ufbx_cache_file_format)type` — the guard above pins `type`
            // into `0..=UFBX_CACHE_FILE_FORMAT_MC`, exactly the enum range.
            // SAFETY: the guard pins `type_` into `0..=UFBX_CACHE_FILE_FORMAT_MC`,
            // exactly the discriminants of the `u32`-repr `CacheFileFormat`.
            cache_view
                .set_format(unsafe { core::mem::transmute::<u32, CacheFileFormat>(type_ as u32) });
        }

        // SAFETY: each `*_raw()` addresses one of the viewed cache file's own
        // `ufbx_string` filename fields, which are `Strblob`-shaped for `raw`
        // `false`, and carries the cache-file view's write-capable provenance.
        resolve_filenames(
            uc,
            unsafe { View::<Strblob>::from_ptr(cache_view.filename_raw() as *mut Strblob) },
            unsafe {
                View::<Strblob>::from_ptr(cache_view.absolute_filename_raw() as *mut Strblob)
            },
            unsafe {
                View::<Strblob>::from_ptr(cache_view.relative_filename_raw() as *mut Strblob)
            },
            false,
        )?;
        // SAFETY: as above, for its own `ufbx_blob` raw filename fields, which are
        // `Strblob`-shaped for `raw` `true`.
        resolve_filenames(
            uc,
            unsafe { View::<Strblob>::from_ptr(cache_view.raw_filename_raw() as *mut Strblob) },
            unsafe {
                View::<Strblob>::from_ptr(cache_view.raw_absolute_filename_raw() as *mut Strblob)
            },
            unsafe {
                View::<Strblob>::from_ptr(cache_view.raw_relative_filename_raw() as *mut Strblob)
            },
            true,
        )?;
    }

    ufbx_assert!(
        uc.tmp_full_weights_view().num_items() == uc.scene_view().blend_channels_view().count()
    );
    // C reads `uc->tmp_full_weights.num_items` as the `ufbxi_push_pop()` count
    // argument; hoisted so the `&mut` borrow does not overlap the read.
    let num_full_weights: usize = uc.tmp_full_weights_view().num_items();
    let full_weights_base: *mut List<Real> = uc
        .tmp_view()
        .push_pop::<List<Real>>(uc.tmp_full_weights_view(), num_full_weights);
    // SAFETY: `tmp_full_weights_mut_ptr` hands out `uc`'s own live full-weight
    // buffer.
    unsafe { buf_free(uc.tmp_full_weights_mut_ptr()) };
    ufbxi_check!(uc, !full_weights_base.is_null(), "full_weights");

    // C: `ufbxi_for_ptr_list(ufbx_blend_channel, p_channel, uc->scene.blend_channels)`
    let scene_blend_channels: &RefListView<BlendChannel> = uc.scene_view().blend_channels_view();
    for channel_ix in 0..scene_blend_channels.count() {
        let channel: &View<BlendChannel> = scene_blend_channels.at(channel_ix);

        fetch_blend_keyframes(uc, channel.keyframes_view(), channel.element())?;

        // C carries `full_weights` as a cursor advanced once per channel; the
        // popped run holds one list header per blend channel, so the per-channel
        // header is derived from the run's base.
        // SAFETY: `channel_ix` is below the blend-channel count, which the assert
        // above pins to the popped run's length, so the offset addresses a live
        // header of that `uc`-owned run — write-capable provenance for the view.
        let full_weights: &ListView<Real> =
            unsafe { ListView::<Real>::from_ptr(full_weights_base.wrapping_add(channel_ix)) };

        let keyframes: &ListView<BlendKeyframe> = channel.keyframes_view();
        for i in 0..keyframes.count() {
            let key: &View<BlendKeyframe> = keyframes.at(i);
            key.set_target_weight(1.0f32 as Real);
            if i < full_weights.count() {
                if !uc.blender_full_weights() {
                    // SAFETY: the full-weight list's own `data` run holds `count`
                    // reals and `i` is below it.
                    key.set_target_weight(unsafe { *full_weights.data().add(i) } / 100.0);
                // C: `key->shape->num_offsets` — `ufbxi_fetch_blend_keyframes`
                // fills every keyframe's `shape` from a connection source, so it
                // is a non-null live `ufbx_blend_shape` of the same scene.
                // SAFETY: as above.
                } else if full_weights.count()
                    == unsafe { View::<BlendShape>::from_ptr(key.shape().ptr()) }.num_offsets()
                {
                    if i == 0 {
                        // Duplicate `index_data` for modification if we retain DOM
                        if uc.opts_view().retain_dom() {
                            // SAFETY: `result_view()` is `uc`'s own live result
                            // buffer, and `full_weights`'s `count`/`data` describe
                            // the weight run being copied.
                            full_weights.set_data(unsafe {
                                uc.result_view().push_copy_raw::<Real>(
                                    full_weights.count(),
                                    full_weights.data(),
                                )
                            });
                            ufbxi_check!(uc, !full_weights.data().is_null(), "full_weights->data");
                        }
                        // C: `ufbxi_for_list(ufbx_real, p_weight, *full_weights)`
                        for weight_ix in 0..full_weights.count() {
                            // SAFETY: `weight_ix` is below the list's own `count`,
                            // so it addresses a live, writable entry of the run
                            // the list header describes.
                            unsafe { *(full_weights.data() as *mut Real).add(weight_ix) /= 100.0 };
                        }
                    }
                    // C: struct assignment (memcpy) of the `ufbx_real_list`
                    // header; `List<T>` is not `Copy` in the generated
                    // bindings, so the copy is a byte-identical
                    // `copy_nonoverlapping`.
                    // SAFETY: the source is this channel's own list header (see
                    // above); the destination is the live blend shape's own
                    // `offset_weights` header, a distinct field of the same type.
                    unsafe {
                        ptr::copy_nonoverlapping(
                            full_weights.as_ptr(),
                            View::<BlendShape>::from_ptr(key.shape().ptr()).offset_weights_raw(),
                            1,
                        )
                    };
                }
            }
        }

        // SAFETY: the channel's `keyframes` run holds `count` initialized
        // keyframes (the fetch above).
        unsafe {
            sort_blend_keyframes(
                uc,
                keyframes.data() as *mut BlendKeyframe,
                keyframes.count(),
            )
        }?;

        if keyframes.count() > 0 {
            // C: `channel->target_shape = ...[count - 1].shape` — the run is
            // non-empty here and every keyframe's `shape` is non-null (see above).
            channel.set_target_shape(Some(keyframes.at(keyframes.count() - 1).shape()));
        }
    }

    {
        // Generate and patch procedural index buffers
        let zero_indices: *mut u32 = uc.result_view().push::<u32>(uc.max_zero_indices());
        let consecutive_indices: *mut u32 =
            uc.result_view().push::<u32>(uc.max_consecutive_indices());
        ufbxi_check!(
            uc,
            !zero_indices.is_null() && !consecutive_indices.is_null(),
            "zero_indices && consecutive_indices"
        );

        // SAFETY: `zero_indices` is the non-null run just pushed with
        // `max_zero_indices()` `u32`s, exactly the span zeroed here.
        unsafe { ptr::write_bytes(zero_indices, 0, uc.max_zero_indices()) };
        for i in 0..uc.max_consecutive_indices() {
            // SAFETY: `consecutive_indices` is the non-null run just pushed with
            // `max_consecutive_indices()` `u32`s, and `i` is below that.
            unsafe { *consecutive_indices.add(i) = i as u32 };
        }

        uc.set_zero_indices(zero_indices);
        uc.set_consecutive_indices(consecutive_indices);

        // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, uc->scene.meshes)`
        let scene_meshes: &RefListView<Mesh> = uc.scene_view().meshes_view();
        for mesh_ix in 0..scene_meshes.count() {
            let mesh: &View<Mesh> = scene_meshes.at(mesh_ix);

            // SAFETY: `indices_raw()` addresses the viewed mesh's own attribute
            // index-list header, so `&raw mut (*..).data` is its data pointer.
            unsafe {
                patch_index_pointer(
                    uc,
                    &raw mut (*mesh.vertex_position().indices_raw()).data as *mut *mut u32,
                )
            };
            // SAFETY: as above, for the mesh's own `vertex_normal` indices.
            unsafe {
                patch_index_pointer(
                    uc,
                    &raw mut (*mesh.vertex_normal().indices_raw()).data as *mut *mut u32,
                )
            };
            // SAFETY: as above, for the mesh's own `vertex_color` indices.
            unsafe {
                patch_index_pointer(
                    uc,
                    &raw mut (*mesh.vertex_color().indices_raw()).data as *mut *mut u32,
                )
            };
            // SAFETY: as above, for the mesh's own `vertex_crease` indices.
            unsafe {
                patch_index_pointer(
                    uc,
                    &raw mut (*mesh.vertex_crease().indices_raw()).data as *mut *mut u32,
                )
            };
            // SAFETY: as above, for the mesh's own `face_material` list.
            unsafe {
                patch_index_pointer(
                    uc,
                    &raw mut (*mesh.face_material_raw()).data as *mut *mut u32,
                )
            };
            // SAFETY: as above, for the mesh's own `face_group` list.
            unsafe {
                patch_index_pointer(uc, &raw mut (*mesh.face_group_raw()).data as *mut *mut u32)
            };

            // SAFETY: as above, for the mesh's own `skinned_position` indices.
            unsafe {
                patch_index_pointer(
                    uc,
                    &raw mut (*mesh.skinned_position().indices_raw()).data as *mut *mut u32,
                )
            };
            // SAFETY: as above, for the mesh's own `skinned_normal` indices.
            unsafe {
                patch_index_pointer(
                    uc,
                    &raw mut (*mesh.skinned_normal().indices_raw()).data as *mut *mut u32,
                )
            };

            // C: `ufbxi_for_list(ufbx_uv_set, set, mesh->uv_sets)`
            // SAFETY: `uv_sets` describes one contiguous arena run of the mesh's
            // own UV sets, live and unmoved for this call.
            let sets = unsafe {
                SliceViewIter::<UvSet>::from_raw_parts(
                    mesh.uv_sets().data as *mut UvSet,
                    mesh.uv_sets().count,
                )
            };
            for set in sets {
                // SAFETY: `indices_raw()` addresses this live `UvSet`'s own
                // index-list header, so `&raw mut (*..).data` is its data pointer.
                unsafe {
                    patch_index_pointer(
                        uc,
                        &raw mut (*set.vertex_uv().indices_raw()).data as *mut *mut u32,
                    )
                };
                // SAFETY: as above, for this set's `vertex_bitangent` indices.
                unsafe {
                    patch_index_pointer(
                        uc,
                        &raw mut (*set.vertex_bitangent().indices_raw()).data as *mut *mut u32,
                    )
                };
                // SAFETY: as above, for this set's `vertex_tangent` indices.
                unsafe {
                    patch_index_pointer(
                        uc,
                        &raw mut (*set.vertex_tangent().indices_raw()).data as *mut *mut u32,
                    )
                };
            }

            // C: `ufbxi_for_list(ufbx_color_set, set, mesh->color_sets)`
            // SAFETY: `color_sets` describes one contiguous arena run of the
            // mesh's own color sets, live and unmoved for this call.
            let csets = unsafe {
                SliceViewIter::<ColorSet>::from_raw_parts(
                    mesh.color_sets().data as *mut ColorSet,
                    mesh.color_sets().count,
                )
            };
            for cset in csets {
                // SAFETY: `indices_raw()` addresses this live `ColorSet`'s own
                // index-list header, so `&raw mut (*..).data` is its data pointer.
                unsafe {
                    patch_index_pointer(
                        uc,
                        &raw mut (*cset.vertex_color().indices_raw()).data as *mut *mut u32,
                    )
                };
            }

            // Generate normals if necessary
            if !mesh.vertex_normal().exists() && uc.opts_view().generate_missing_normals() {
                // SAFETY: `uc`'s tmp/result arenas back the topology and
                // normal runs `generate_normals` pushes.
                unsafe { generate_normals(uc, mesh) }?;
            }

            // Assign first UV and color sets as the "canonical" ones
            if mesh.uv_sets().count > 0 {
                let uv_set: &View<UvSet> = mesh.uv_sets_view().at(0);
                // C: struct assignment (memcpy) of the vertex-attribute
                // headers; the `Vertex*` structs are not `Copy` in the
                // generated bindings, so the copy is spelled as a
                // byte-identical `copy_nonoverlapping`.
                // SAFETY: source and destination are the UV set's own
                // `vertex_uv` header and the mesh's own `vertex_uv` header,
                // distinct places of the same type.
                unsafe {
                    ptr::copy_nonoverlapping(uv_set.vertex_uv_ptr(), mesh.vertex_uv_raw(), 1)
                };
                // SAFETY: as above, for the `vertex_bitangent` headers.
                unsafe {
                    ptr::copy_nonoverlapping(
                        uv_set.vertex_bitangent_ptr(),
                        mesh.vertex_bitangent_raw(),
                        1,
                    )
                };
                // SAFETY: as above, for the `vertex_tangent` headers.
                unsafe {
                    ptr::copy_nonoverlapping(
                        uv_set.vertex_tangent_ptr(),
                        mesh.vertex_tangent_raw(),
                        1,
                    )
                };
            }
            if mesh.color_sets().count > 0 {
                let color_set: &View<ColorSet> = mesh.color_sets_view().at(0);
                // SAFETY: source and destination are the color set's own
                // `vertex_color` header and the mesh's own `vertex_color`
                // header, distinct places of the same type.
                unsafe {
                    ptr::copy_nonoverlapping(
                        color_set.vertex_color_ptr(),
                        mesh.vertex_color_raw(),
                        1,
                    )
                };
            }

            if mesh.face_group_parts().count == 1 {
                let part: &View<MeshPart> = mesh.face_group_parts_view().at(0);
                // SAFETY: `face_indices_raw()` addresses that part's own
                // index-list header, so `&raw mut (*..).data` is its data pointer.
                unsafe {
                    patch_index_pointer(
                        uc,
                        &raw mut (*part.face_indices_raw()).data as *mut *mut u32,
                    )
                };
            }

            // SAFETY: `element_raw()` addresses the viewed mesh's own element
            // header, and its provenance spans the enclosing `ufbx_mesh` — the
            // whole element struct the `search_node` walk may read.
            unsafe { fetch_mesh_materials(uc, mesh.materials_view(), mesh.element_raw(), true) }?;

            // Patch materials to instances if necessary
            if mesh.materials().count > 0 {
                // C: `ufbxi_for_ptr_list(ufbx_node, p_node, mesh->instances)`
                // The instance list was fetched by the pass above.
                let instances: &RefListView<Node> = mesh.element().instances_view();
                for node_ix in 0..instances.count() {
                    let node: &View<Node> = instances.at(node_ix);
                    let node_materials: &RefListView<Material> = node.materials_view();
                    // C-parity: `mesh->materials.data[0]` may be NULL (broken
                    // element connections), so the entry is read as the bare
                    // `ufbx_material*` the `Ref` field is at the ABI level.
                    let mesh_materials: *mut *mut Material =
                        mesh.materials_view().data() as *mut *mut Material;
                    // SAFETY: `materials.count > 0` makes element `0` of the
                    // mesh's own material run live.
                    if node_materials.count() < mesh.materials_view().count()
                        && !unsafe { *mesh_materials.add(0) }.is_null()
                    {
                        // `result_view()` is `uc`'s own result buffer.
                        let materials: *mut *mut Material = uc
                            .result_view()
                            .push::<*mut Material>(mesh.materials_view().count());
                        ufbxi_check!(uc, !materials.is_null(), "materials");
                        // C: `ufbxi_nounroll for (...)` — the no-unroll pragma
                        // is optimizer-only and has no Rust analogue.
                        for i in 0..node_materials.count() {
                            // SAFETY: `materials` is the non-null run just pushed
                            // with `mesh->materials.count` slots, and the loop bound
                            // `node->materials.count` is below it (checked above);
                            // the node's own material run holds that many entries,
                            // whose slots are read as bare pointer bits.
                            unsafe {
                                *materials.add(i) =
                                    *(node_materials.data() as *mut *mut Material).add(i)
                            };
                        }
                        for i in node_materials.count()..mesh.materials_view().count() {
                            // SAFETY: `i < mesh->materials.count`, the length of both
                            // the pushed `materials` run and the mesh's own material
                            // run.
                            unsafe { *materials.add(i) = *mesh_materials.add(i) };
                        }
                        // `materials` is the run just filled in.
                        node_materials.set_data(materials as *const Ref<Material>);
                        node_materials.set_count(mesh.materials_view().count());
                    }
                }
            }

            if uc.retain_mesh_parts() {
                let num_parts: usize = max_sz(mesh.materials().count, 1);
                let material_parts = mesh.material_parts_view();
                // `result_view()` is `uc`'s own result buffer.
                material_parts.set_data(uc.result_view().push_zero::<MeshPart>(num_parts));
                ufbxi_check!(
                    uc,
                    !material_parts.data().is_null(),
                    "mesh->material_parts.data"
                );
                material_parts.set_count(num_parts);
            }

            if mesh.materials().count <= 1 {
                // Use the shared consecutive index buffer for mesh faces if there's only one material
                // See HACK(consecutive-faces) in `ufbxi_read_mesh()`.
                if mesh.material_parts().count > 0 {
                    let part: &View<MeshPart> = mesh.material_parts_view().at(0);
                    part.set_num_faces(mesh.num_faces());
                    part.set_num_triangles(mesh.num_triangles());
                    part.set_num_empty_faces(mesh.num_empty_faces());
                    part.set_num_point_faces(mesh.num_point_faces());
                    part.set_num_line_faces(mesh.num_line_faces());
                    // The shared consecutive-index run is `uc`'s own, sized to cover
                    // every mesh's face count.
                    part.face_indices_view().set_data(uc.consecutive_indices());
                    part.face_indices_view().set_count(mesh.num_faces());
                    // The shared zero-index run is `uc`'s own and holds at least one
                    // entry here.
                    mesh.material_part_usage_order_view()
                        .set_data(uc.zero_indices());
                    mesh.material_part_usage_order_view().set_count(1);
                }

                if mesh.materials().count == 1 {
                    // The shared zero-index run is `uc`'s own, sized to cover every
                    // mesh's face count.
                    mesh.face_material_view().set_data(uc.zero_indices());
                    mesh.face_material_view().set_count(mesh.num_faces());
                } else {
                    mesh.face_material_view().set_data(ptr::null());
                    mesh.face_material_view().set_count(0);
                }
            } else if mesh.materials().count > 0 {
                // SAFETY: `result_view()`/`error_mut_ptr()` are `uc`'s own
                // result buffer and error slot.
                unsafe { finalize_mesh_material(uc.result_view(), uc.error_mut_ptr(), mesh) }?;
            }

            // Fetch deformers
            // SAFETY: `mesh.element_raw()` addresses that element's
            // header with whole-struct provenance.
            unsafe {
                fetch_dst_elements(
                    uc,
                    mesh.skin_deformers_view().as_element_list(),
                    mesh.element_raw(),
                    search_node,
                    true,
                    None,
                    ElementType::SkinDeformer,
                )
            }?;
            // SAFETY: `mesh.element_raw()` addresses that element's
            // header with whole-struct provenance.
            unsafe {
                fetch_dst_elements(
                    uc,
                    mesh.blend_deformers_view().as_element_list(),
                    mesh.element_raw(),
                    search_node,
                    true,
                    None,
                    ElementType::BlendDeformer,
                )
            }?;
            // SAFETY: `mesh.element_raw()` addresses that element's
            // header with whole-struct provenance.
            unsafe {
                fetch_dst_elements(
                    uc,
                    mesh.cache_deformers_view().as_element_list(),
                    mesh.element_raw(),
                    search_node,
                    true,
                    None,
                    ElementType::CacheDeformer,
                )
            }?;
            // SAFETY: as above, for its own `all_deformers` list.
            unsafe {
                fetch_deformers(
                    uc,
                    mesh.all_deformers_view(),
                    mesh.element_raw(),
                    search_node,
                )
            }?;

            // Vertex position must always exist if not explicitly allowed to be missing
            if !mesh.vertex_position().exists() && !uc.opts_view().allow_missing_vertex_position() {
                ufbxi_check!(uc, mesh.num_indices() == 0, "mesh->num_indices == 0");
                mesh.vertex_position().set_exists(true);
                mesh.vertex_position().set_unique_per_vertex(true);
                mesh.skinned_position().set_exists(true);
                mesh.skinned_position().set_unique_per_vertex(true);
            }

            // Update metadata
            if mesh.max_face_triangles() > uc.scene_view().metadata_view().max_face_triangles() {
                uc.scene_view()
                    .metadata_view()
                    .set_max_face_triangles(mesh.max_face_triangles());
            }
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_stereo_camera, p_stereo, uc->scene.stereo_cameras)`
    let stereo_cameras: &RefListView<StereoCamera> = uc.scene_view().stereo_cameras_view();
    for stereo_ix in 0..stereo_cameras.count() {
        let stereo: &View<StereoCamera> = stereo_cameras.at(stereo_ix);
        // SAFETY: `element_raw()` addresses the viewed stereo camera's own
        // element header, the prop name is an interned NUL-terminated pool
        // string, and the fetched destination is null or a live `ufbx_camera`.
        stereo.set_left(unsafe {
            opt_ref(fetch_dst_element(
                stereo.element_raw(),
                search_node,
                sp::LeftCamera.as_ptr(),
                ElementType::Camera,
            ) as *mut Camera)
        });
        // SAFETY: as above, for the right camera.
        stereo.set_right(unsafe {
            opt_ref(fetch_dst_element(
                stereo.element_raw(),
                search_node,
                sp::RightCamera.as_ptr(),
                ElementType::Camera,
            ) as *mut Camera)
        });
    }

    // C: `ufbxi_for_ptr_list(ufbx_nurbs_curve, p_curve, uc->scene.nurbs_curves)`
    let nurbs_curves: &RefListView<NurbsCurve> = uc.scene_view().nurbs_curves_view();
    for curve_ix in 0..nurbs_curves.count() {
        let curve: &View<NurbsCurve> = nurbs_curves.at(curve_ix);
        finalize_nurbs_basis(uc, curve.basis())?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_nurbs_surface, p_surface, uc->scene.nurbs_surfaces)`
    let nurbs_surfaces: &RefListView<NurbsSurface> = uc.scene_view().nurbs_surfaces_view();
    for surface_ix in 0..nurbs_surfaces.count() {
        let surface: &View<NurbsSurface> = nurbs_surfaces.at(surface_ix);
        finalize_nurbs_basis(uc, surface.basis_u())?;
        finalize_nurbs_basis(uc, surface.basis_v())?;

        // SAFETY: `element_raw()` addresses the viewed surface's own element
        // header; the fetched destination is null or a live `ufbx_material`.
        surface.set_material(unsafe {
            opt_ref(fetch_dst_element(
                surface.element_raw(),
                true,
                ptr::null(),
                ElementType::Material,
            ) as *mut Material)
        });
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_stack, p_stack, uc->scene.anim_stacks)`
    let anim_stacks: &RefListView<AnimStack> = uc.scene_view().anim_stacks_view();
    for stack_ix in 0..anim_stacks.count() {
        let stack: &AnimStackView = anim_stacks.at(stack_ix);
        // SAFETY: `stack.element_raw()` addresses that element's
        // header with whole-struct provenance.
        unsafe {
            fetch_dst_elements(
                uc,
                stack.layers_view().as_element_list(),
                stack.element_raw(),
                false,
                true,
                None,
                ElementType::AnimLayer,
            )
        }?;

        // SAFETY: `anim_raw()` addresses the stack's own anim-pointer slot
        // (`Ref<Anim>` is `repr(transparent)` over the pointer); the fetch
        // above filled its `layers` list, whose `data`/`count` describe one
        // layer-pointer run.
        unsafe {
            push_anim(
                uc,
                stack.anim_raw() as *mut *mut Anim,
                stack.layers_view().data() as *mut *mut AnimLayer,
                stack.layers_view().count(),
            )
        }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_layer, p_layer, uc->scene.anim_layers)`
    let anim_layers: &RefListView<AnimLayer> = uc.scene_view().anim_layers_view();
    for layer_ix in 0..anim_layers.count() {
        let layer: &AnimLayerView = anim_layers.at(layer_ix);
        // C: `p_layer` — this layer's own slot of the scene's layer-pointer
        // run, derived from the LIST BASE (the one-element layer list
        // `ufbxi_push_anim` copies from), never from the element view.
        let p_layer: *mut *mut AnimLayer =
            (anim_layers.data() as *mut *mut AnimLayer).wrapping_add(layer_ix);
        // SAFETY: `layer.element_raw()` addresses that element's
        // header with whole-struct provenance.
        unsafe {
            fetch_dst_elements(
                uc,
                layer.anim_values_view().as_element_list(),
                layer.element_raw(),
                false,
                true,
                None,
                ElementType::AnimValue,
            )
        }?;

        // SAFETY: `anim_raw()` addresses the layer's own anim-pointer slot;
        // `p_layer` addresses this layer's own slot of the scene's
        // layer-pointer run, the one-element layer list stored into the anim.
        unsafe { push_anim(uc, layer.anim_raw() as *mut *mut Anim, p_layer, 1) }?;

        let mut min_id: u32 = u32::MAX;
        let mut max_id: u32 = 0;

        // Combine the animated properties with elements (potentially duplicates!)
        let mut num_anim_props: usize = 0;
        // C: `ufbxi_for_ptr_list(ufbx_anim_value, p_value, layer->anim_values)`
        let anim_values: &RefListView<AnimValue> = layer.anim_values_view();
        for value_ix in 0..anim_values.count() {
            let value: &AnimValueView = anim_values.at(value_ix);
            // C: `ufbxi_for_list(ufbx_connection, ac, value->element.connections_src)`
            let connections_src: &ListView<Connection> = value.element().connections_src_view();
            for ac_ix in 0..connections_src.count() {
                let ac: &View<Connection> = connections_src.at(ac_ix);
                if ac.src_prop_view().length() == 0 && ac.dst_prop_view().length() > 0 {
                    let aprop: *mut AnimProp = uc.tmp_stack_view().push::<AnimProp>(1);
                    // SAFETY: `ac`'s `dst` is a non-null reference to a live
                    // element of the same scene.
                    let id: u32 = unsafe { ElementView::from_ptr(ac.dst().ptr()) }.element_id();
                    min_id = min32(min_id, id);
                    max_id = max32(max_id, id);
                    // C: `ufbxi_arraycount(layer->_element_id_bitmask) - 1`.
                    // The masked word is updated through the layer's own
                    // bitmask accessors (read, set the bit, write back).
                    let mut element_id_bitmask: [u32; 4] = layer._element_id_bitmask();
                    let id_mask: u32 = element_id_bitmask.len() as u32 - 1;
                    element_id_bitmask[((id >> 5) & id_mask) as usize] |= 1u32 << (id & 31);
                    layer.set_element_id_bitmask(element_id_bitmask);
                    ufbxi_check!(uc, !aprop.is_null(), "aprop");
                    // SAFETY: `aprop` is the non-null one-element push just
                    // made into `uc`'s tmp stack, so it addresses a live
                    // `ufbx_anim_prop` of that arena run.
                    let aprop: &View<AnimProp> = unsafe { View::<AnimProp>::from_ptr(aprop) };
                    // SAFETY: `value` views a live anim value of this layer's
                    // own list, so `get()` yields its non-null address.
                    aprop.set_anim_value(unsafe { Ref::from_ptr(value.get()) });
                    aprop.set_element(ac.dst());
                    aprop.set_internal_key(get_name_key(ac.dst_prop_view().bytes()));
                    aprop.set_prop_name(ac.dst_prop());
                    num_anim_props += 1;
                }
            }
        }

        if min_id != u32::MAX {
            layer.set_min_element_id(min_id);
            layer.set_max_element_id(max_id);
        }

        match find_int(layer.props_view(), &sp::BlendMode, 0) {
            0 => {
                // Additive
                layer.set_blended(true);
                layer.set_additive(true);
            }
            1 => {
                // Override
                layer.set_blended(false);
                layer.set_additive(false);
            }
            2 => {
                // Override Passthrough
                layer.set_blended(true);
                layer.set_additive(false);
            }
            _ => {
                // Unknown
                layer.set_blended(false);
                layer.set_additive(false);
            }
        }

        let weight_prop: Option<&PropView> = find_prop(layer.props_view(), &sp::Weight);
        if let Some(weight_prop) = weight_prop {
            // C-parity: `prop->value_real` is the `ufbx_prop` value union's
            // first real; the generated struct keeps only `value_vec4`.
            // C: `0.99999f` — a `float` literal promoted to `ufbx_real`, NOT
            // the double 0.99999.
            layer.set_weight(weight_prop.value_vec4().x / 100.0);
            if layer.weight() < 0.0f32 as Real {
                layer.set_weight(0.0f32 as Real);
            }
            if layer.weight() > 0.99999f32 as Real {
                layer.set_weight(1.0f32 as Real);
            }
            layer.set_weight_is_animated(
                (weight_prop.flags().raw() & PropFlags::ANIMATED.raw()) != 0,
            );
        } else {
            layer.set_weight(1.0f32 as Real);
            layer.set_weight_is_animated(false);
        }
        layer.set_compose_rotation(
            find_int(layer.props_view(), &sp::RotationAccumulationMode, 0) == 0,
        );
        layer.set_compose_scale(find_int(layer.props_view(), &sp::ScaleAccumulationMode, 0) == 0);

        // Add a dummy NULL element animated prop at the end so we can iterate
        // animated props without worrying about boundary conditions..
        {
            let aprop: *mut AnimProp = uc.tmp_stack_view().push_zero::<AnimProp>(1);
            ufbxi_check!(uc, !aprop.is_null(), "aprop");
        }

        // The loop above pushed `num_anim_props` anim props onto `uc`'s tmp
        // stack plus the terminator, and they are popped into `uc`'s own result
        // buffer here.
        layer.anim_props_view().set_data(
            uc.result_view()
                .push_pop::<AnimProp>(uc.tmp_stack_view(), num_anim_props + 1),
        );
        ufbxi_check!(
            uc,
            !layer.anim_props_view().data().is_null(),
            "layer->anim_props.data"
        );
        layer.anim_props_view().set_count(num_anim_props);
        // SAFETY: the layer's own `anim_props` run is non-null (checked above)
        // and holds `count` initialized props followed by the terminator.
        unsafe {
            sort_anim_props(
                uc,
                layer.anim_props_view().data() as *mut AnimProp,
                layer.anim_props_view().count(),
            )
        }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_value, p_value, uc->scene.anim_values)`
    let anim_values: &RefListView<AnimValue> = uc.scene_view().anim_values_view();
    for value_ix in 0..anim_values.count() {
        let value: &AnimValueView = anim_values.at(value_ix);

        // TODO: Search for things like d|Visibility with a constructed name
        // C: `value->default_value.x = ufbxi_find_real(...)` x6 — read-modify
        // through the view accessors, written back once below.
        let mut dv: Vec3 = value.default_value();
        dv.x = find_real(value.props_view(), &sp::X, dv.x);
        dv.x = find_real(value.props_view(), &sp::d_X, dv.x);
        dv.y = find_real(value.props_view(), &sp::Y, dv.y);
        dv.y = find_real(value.props_view(), &sp::d_Y, dv.y);
        dv.z = find_real(value.props_view(), &sp::Z, dv.z);
        dv.z = find_real(value.props_view(), &sp::d_Z, dv.z);
        value.set_default_value(dv);

        // C: `ufbxi_for_list(ufbx_connection, conn, value->element.connections_dst)`
        let connections_dst: &ListView<Connection> = value.element().connections_dst_view();
        for conn_ix in 0..connections_dst.count() {
            let conn: &View<Connection> = connections_dst.at(conn_ix);
            // SAFETY: `conn`'s `src` is a non-null reference to a live element
            // of the same scene.
            let src: &ElementView = unsafe { ElementView::from_ptr(conn.src().ptr()) };
            if src.type_() == ElementType::AnimCurve && conn.src_prop_view().length() == 0 {
                // The check above pins the source's type, so it heads a live
                // `ufbx_anim_curve`.
                let curve: *mut AnimCurve = conn.src().ptr() as *mut AnimCurve;

                let mut index: u32 = 0;
                let name: *const u8 = conn.dst_prop_view().data();
                if name == sp::Y.as_ptr() || name == sp::d_Y.as_ptr() {
                    index = 1;
                }
                if name == sp::Z.as_ptr() || name == sp::d_Z.as_ptr() {
                    index = 2;
                }

                let prop: Option<&PropView> =
                    find_prop_len(value.props_view(), conn.dst_prop_view().bytes());
                if let Some(prop) = prop {
                    // C indexes the `ufbx_vec3` value union's `ufbx_real v[3]`
                    // view; the generated struct keeps only `x`/`y`/`z`, so the
                    // index is pointer arithmetic from the field base.
                    // SAFETY: `default_value_raw()` addresses the anim value's
                    // own `ufbx_vec3`, whose three reals `index <= 2` bounds.
                    unsafe {
                        *(value.default_value_raw() as *mut Real).add(index as usize) =
                            prop.value_vec4().x;
                    }
                }
                // SAFETY: `curves_raw()` addresses the anim value's own
                // three-entry curve array, which `index <= 2` bounds; `curve`
                // is the non-null source element above.
                unsafe { (*value.curves_raw())[index as usize] = opt_ref(curve) };
            }
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_curve, p_curve, uc->scene.anim_curves)`
    let anim_curves: &RefListView<AnimCurve> = uc.scene_view().anim_curves_view();
    for curve_ix in 0..anim_curves.count() {
        let curve: &View<AnimCurve> = anim_curves.at(curve_ix);
        if curve.keyframes_view().count() > 0 {
            curve.set_min_time(curve.keyframes_view().at(0).time());
            curve.set_max_time(
                curve
                    .keyframes_view()
                    .at(curve.keyframes_view().count() - 1)
                    .time(),
            );
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_shader, p_shader, uc->scene.shaders)`
    let shaders: &RefListView<Shader> = uc.scene_view().shaders_view();
    for shader_ix in 0..shaders.count() {
        let shader: &ShaderView = shaders.at(shader_ix);
        // SAFETY: `shader.element_raw()` addresses that element's
        // header with whole-struct provenance.
        unsafe {
            fetch_dst_elements(
                uc,
                shader.bindings_view().as_element_list(),
                shader.element_raw(),
                false,
                false,
                None,
                ElementType::ShaderBinding,
            )
        }?;

        let api: Option<&PropView> = find_prop_len(shader.props_view(), b"RenderAPI");
        if let Some(api) = api {
            // C `strcmp` over the prop's interned `value_str`, whose bytes are
            // NUL-terminated at `length` — `c_strcmp` stops at the same first
            // NUL as C does.
            if c_strcmp(api.value_str_view().bytes(), b"ARNOLD_SHADER_ID") == 0 {
                shader.set_type(ShaderType::ArnoldStandardSurface);
            } else if c_strcmp(api.value_str_view().bytes(), b"OSL") == 0 {
                shader.set_type(ShaderType::OslStandardSurface);
            } else if c_strcmp(api.value_str_view().bytes(), b"SFX_PBS_SHADER") == 0 {
                shader.set_type(ShaderType::ShaderfxGraph);
            }
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_material, p_material, uc->scene.materials)`
    let materials: &RefListView<Material> = uc.scene_view().materials_view();
    for material_ix in 0..materials.count() {
        let material: &MaterialView = materials.at(material_ix);
        // SAFETY: `element_raw()` addresses the viewed material's own element
        // header; the fetched source is null or a live `ufbx_shader`.
        material.set_shader(unsafe {
            opt_ref(
                fetch_src_element(material.element_raw(), false, None, ElementType::Shader)
                    as *mut Shader,
            )
        });

        // C `strcmp` over the material's interned `shading_model_name`, whose
        // bytes are NUL-terminated at `length` (see the shader loop above).
        if c_strcmp(material.shading_model_name_view().bytes(), b"lambert") == 0
            || c_strcmp(material.shading_model_name_view().bytes(), b"Lambert") == 0
        {
            material.set_shader_type(ShaderType::FbxLambert);
        } else if c_strcmp(material.shading_model_name_view().bytes(), b"phong") == 0
            || c_strcmp(material.shading_model_name_view().bytes(), b"Phong") == 0
        {
            material.set_shader_type(ShaderType::FbxPhong);
        }

        if let Some(material_shader) = material.shader() {
            // SAFETY: a `Some` shader reference names a live `ufbx_shader` of
            // the same scene.
            let material_shader: &ShaderView =
                unsafe { ShaderView::from_ptr(material_shader.ptr()) };
            material.set_shader_type(material_shader.type_());
        } else {
            if uc.opts_view().use_blender_pbr_material()
                && uc.exporter() == Exporter::BlenderBinary
                && uc.exporter_version() >= pack_version(4, 12, 0)
            {
                material.set_shader_type(ShaderType::BlenderPhong);
            }

            // TODO: Is this too strict?
            if material.shader_type() == ShaderType::Unknown {
                let (classid_a, classid_b): (u32, u32) = (
                    api_find_int_len(material.props_view(), b"3dsMax|ClassIDa", 0) as u64 as u32,
                    api_find_int_len(material.props_view(), b"3dsMax|ClassIDb", 0) as u64 as u32,
                );
                if classid_a == 0x3d6b1cecu32 && classid_b == 0xdeadc001u32 {
                    material.set_shader_type(ShaderType::E3DsMaxPhysicalMaterial);
                    material.set_shader_prop_prefix(ufbxi_string_literal!(b"3dsMax|Parameters|\0"));
                } else if classid_a == 0xf1551e33u32 && classid_b == 0x37fb1337u32 {
                    material.set_shader_type(ShaderType::OpenpbrMaterial);
                    material.set_shader_prop_prefix(ufbxi_string_literal!(b"3dsMax|Parameters|\0"));
                } else if classid_a == 0x38420192u32 && classid_b == 0x45fe4e1bu32 {
                    material.set_shader_type(ShaderType::GltfMaterial);
                    material.set_shader_prop_prefix(ufbxi_string_literal!(b"3dsMax|\0"));
                } else if classid_a == 0xd00f1e00u32 && classid_b == 0xbe77e500u32 {
                    material.set_shader_type(ShaderType::E3DsMaxPbrMetalRough);
                    material.set_shader_prop_prefix(ufbxi_string_literal!(b"3dsMax|main|\0"));
                } else if classid_a == 0xd00f1e00u32 && classid_b == 0x01dbad33u32 {
                    material.set_shader_type(ShaderType::E3DsMaxPbrSpecGloss);
                    material.set_shader_prop_prefix(ufbxi_string_literal!(b"3dsMax|main|\0"));
                }
            }
        }

        // SAFETY: the projection addresses the viewed material's own element
        // header, and `search_node` is clear, so header provenance suffices.
        unsafe { fetch_textures(uc, material.textures_view(), material.element_raw(), false) }?;
    }

    // Ugh.. Patch the textures from meshes for legacy LayerElement-style textures
    {
        // C: `ufbxi_for_ptr_list(ufbx_mesh, p_mesh, uc->scene.meshes)`
        let mut p_mesh: *mut *mut Mesh = uc.scene_view().meshes_view().data() as *mut *mut Mesh;
        let p_mesh_end: *mut *mut Mesh = add_ptr(p_mesh, uc.scene_view().meshes_view().count());
        while p_mesh != p_mesh_end {
            // SAFETY: `p_mesh != p_mesh_end`, so it addresses a live, initialized
            // slot of the scene's mesh-pointer run; the stored entry is a
            // context-owned mesh element, so its provenance is write-capable and
            // `Mut` is the right mode.
            let mesh = unsafe { View::<Mesh>::from_ptr(*p_mesh) };
            let num_materials: usize = mesh.materials().count;

            let extra: *mut MeshExtra =
                get_element_extra(uc, mesh.element().element_id()) as *mut MeshExtra;
            if extra.is_null() {
                // SAFETY: `p_mesh != p_mesh_end`, so the advance lands at or before
                // the run's one-past-the-end pointer.
                p_mesh = unsafe { p_mesh.add(1) };
                continue;
            }
            if num_materials == 0 {
                // SAFETY: as above.
                p_mesh = unsafe { p_mesh.add(1) };
                continue;
            }

            // TODO: This leaks currently to result, probably doesn't matter..
            // C: `ufbx_texture_list textures;` — uninitialized local (no
            // upstream `// ufbxi_uninit` marker); `ufbxi_fetch_dst_elements`
            // writes both fields before the first read.
            let mut textures_storage = MaybeUninit::<RefList<Texture>>::uninit();
            let textures: *mut RefList<Texture> = textures_storage.as_mut_ptr();
            // SAFETY: `textures` addresses a live local `ufbx_texture_list`, which
            // shares the `ufbx_element_list` layout the fetch writes in full (C:
            // its `void *` out-parameter); `mesh.element_raw()` addresses the viewed mesh's
            // element header with whole-mesh provenance.
            unsafe {
                fetch_dst_elements(
                    uc,
                    RefListView::<Element>::from_ptr(textures as *mut RefList<Element>),
                    mesh.element_raw(),
                    true,
                    false,
                    None,
                    ElementType::Texture,
                )
            }?;

            let mut num_material_textures: usize = 0;
            // C: `ufbxi_for(ufbxi_tmp_mesh_texture, tex, extra->texture_arr, extra->texture_count)`
            // SAFETY: `extra` is the non-null per-element extra fetched above.
            // `texture_arr`/`texture_count` describe one run.
            let (mut tex, tex_count) = unsafe { ((*extra).texture_arr, (*extra).texture_count) };
            let tex_end: *mut TmpMeshTexture = add_ptr(tex, tex_count);
            while tex != tex_end {
                // SAFETY: `tex != tex_end`, so it addresses a live, initialized
                // entry of that run; with `num_faces > 0` entry `0` of that tmp
                // mesh texture's own per-face run is live.
                if unsafe { (*tex).all_same } {
                    let texture_id: i32 = unsafe {
                        if (*tex).num_faces > 0 {
                            *(*tex).face_texture.add(0) as i32
                        } else {
                            0
                        }
                    };
                    // SAFETY: `textures` was written in full by the fetch above.
                    if texture_id >= 0 && (texture_id as usize) < unsafe { (*textures).count } {
                        let mat_texs: *mut TmpMaterialTexture =
                            uc.tmp_stack_view()
                                .push::<TmpMaterialTexture>(num_materials);
                        ufbxi_check!(uc, !mat_texs.is_null(), "mat_texs");
                        num_material_textures += num_materials;
                        for i in 0..num_materials {
                            // SAFETY: `mat_texs` is the non-null push of
                            // `num_materials` entries just made into `uc`'s tmp
                            // stack, and `i < num_materials`.
                            unsafe {
                                (*mat_texs.add(i)).material_id = i as i32;
                                (*mat_texs.add(i)).texture_id = texture_id;
                            }
                            // SAFETY: as above, with `tex` live.
                            unsafe { (*mat_texs.add(i)).prop_name = (*tex).prop_name };
                        }
                    }
                } else if mesh.face_material().count != 0 {
                    // SAFETY: `tex` is live (see above).
                    let num_faces: usize = min_sz(unsafe { (*tex).num_faces }, mesh.num_faces());
                    let mut prev_material: i32 = -1;
                    let mut prev_texture: i32 = -1;
                    for i in 0..num_faces {
                        // SAFETY: `num_faces` is at most the tmp mesh texture's own
                        // `num_faces`, the length of its per-face run.
                        let texture_id: i32 = unsafe { *(*tex).face_texture.add(i) } as i32;
                        // SAFETY: `num_faces` is at most the mesh's `num_faces`; the
                        // `face_material` list is non-empty here (checked above), so
                        // it is the per-face run of that length.
                        let material_id: i32 = unsafe { *mesh.face_material().data.add(i) } as i32;
                        // SAFETY: `textures` was written in full by the fetch above.
                        if texture_id < 0 || (texture_id as usize) >= unsafe { (*textures).count } {
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
                            uc.tmp_stack_view().push::<TmpMaterialTexture>(1);
                        ufbxi_check!(uc, !mat_tex.is_null(), "mat_tex");
                        // SAFETY: `mat_tex` is the non-null one-element push just
                        // made into `uc`'s tmp stack, so it heads a live
                        // `ufbxi_tmp_material_texture` with write-capable arena
                        // provenance.
                        let mat_tex: &View<TmpMaterialTexture> =
                            unsafe { View::<TmpMaterialTexture>::from_ptr(mat_tex) };
                        mat_tex.set_material_id(material_id);
                        mat_tex.set_texture_id(texture_id);
                        // SAFETY: `tex` is live (see above).
                        mat_tex.set_prop_name(unsafe { (*tex).prop_name });
                        num_material_textures += 1;
                    }
                }
                // SAFETY: `tex != tex_end`, so the advance lands at or before the
                // run's one-past-the-end pointer.
                tex = unsafe { tex.add(1) };
            }

            // Push a sentinel material texture to the end so we don't need to
            // duplicate the material texture flushing code twice.
            {
                let mat_tex: *mut TmpMaterialTexture =
                    uc.tmp_stack_view().push::<TmpMaterialTexture>(1);
                ufbxi_check!(uc, !mat_tex.is_null(), "mat_tex");
                // SAFETY: `mat_tex` is the non-null one-element push just made into
                // `uc`'s tmp stack, so it heads a live `ufbxi_tmp_material_texture`
                // with write-capable arena provenance.
                let mat_tex: &View<TmpMaterialTexture> =
                    unsafe { View::<TmpMaterialTexture>::from_ptr(mat_tex) };
                mat_tex.set_material_id(-1);
                mat_tex.set_texture_id(-1);
                mat_tex.set_prop_name(EMPTY_STRING.0);
            }

            let mat_texs: *mut TmpMaterialTexture = uc
                .tmp_view()
                .push_pop::<TmpMaterialTexture>(uc.tmp_stack_view(), num_material_textures + 1);
            ufbxi_check!(uc, !mat_texs.is_null(), "mat_texs");
            // SAFETY: `mat_texs` is the non-null popped run holding the
            // `num_material_textures` entries pushed above plus the sentinel.
            unsafe { sort_tmp_material_textures(uc, mat_texs, num_material_textures) }?;

            // C: `ufbxi_tmp_material_texture mat_tex = mat_texs[i];` over the
            // sorted run.
            // SAFETY: `mat_texs` is the non-null (checked above) `push_pop`
            // -materialized run of `num_material_textures + 1` contiguous,
            // initialized entries, live in `uc`'s tmp buffer for this walk.
            let mat_tex_iter = unsafe {
                SliceViewIter::<TmpMaterialTexture>::from_raw_parts(
                    mat_texs,
                    num_material_textures + 1,
                )
            };
            // SAFETY: `textures_storage` is a live local that
            // `ufbxi_fetch_dst_elements` wrote in full above, so it heads a
            // valid `ufbx_texture_list` reached through write-capable
            // (`as_mut_ptr`) provenance.
            let textures_view: &RefListView<Texture> =
                unsafe { RefListView::<Texture>::from_ptr(textures) };

            let mut prev_material: i32 = -2;
            let mut prev_texture: i32 = -2;
            let mut prev_prop: *const u8 = ptr::null();
            let mut num_textures_in_material: usize = 0;
            for mat_tex in mat_tex_iter {
                // C copies the entry by value before reading its fields; the
                // three reads are hoisted here because nothing writes the run
                // inside the loop body.
                let mat_tex_material_id: i32 = mat_tex.material_id();
                let mat_tex_texture_id: i32 = mat_tex.texture_id();
                let mat_tex_prop_name: String = mat_tex.prop_name();
                if mat_tex_material_id != prev_material {
                    if prev_material >= 0 && num_textures_in_material > 0 {
                        // C admits a NULL slot here, so the entry is read as
                        // bare pointer bits off the list base rather than as a
                        // `Ref`.
                        // SAFETY: `prev_material` came from a `mat_tex` entry, which
                        // the pushes above bounded by `num_materials`, the mesh's
                        // material-run length.
                        let mat: *mut Material = unsafe {
                            *(mesh.materials_view().data() as *const *mut Material)
                                .add(prev_material as usize)
                        };
                        let mat: Option<&MaterialView> = if mat.is_null() {
                            None
                        } else {
                            // SAFETY: a non-null `mat` is a live `ufbx_material` of
                            // the uc-owned scene, so its provenance is
                            // write-capable.
                            Some(unsafe { MaterialView::from_ptr(mat) })
                        };
                        // C: `if (mat && mat->textures.count == 0)`
                        if let Some(mat) = mat.filter(|mat| mat.textures_view().count() == 0) {
                            let texs: *mut MaterialTexture =
                                uc.result_view().push_pop::<MaterialTexture>(
                                    uc.tmp_stack_view(),
                                    num_textures_in_material,
                                );
                            ufbxi_check!(uc, !texs.is_null(), "texs");
                            mat.textures_view().set_data(texs);
                            mat.textures_view().set_count(num_textures_in_material);
                        } else {
                            // SAFETY: `tmp_stack_mut_ptr` hands out `uc`'s own live
                            // tmp stack, which holds the `num_textures_in_material`
                            // material textures pushed for this material.
                            unsafe {
                                pop::<MaterialTexture>(
                                    uc.tmp_stack_mut_ptr(),
                                    num_textures_in_material,
                                    ptr::null_mut(),
                                )
                            };
                        }
                    }

                    if mat_tex_material_id < 0 {
                        break;
                    }
                    prev_material = mat_tex_material_id;
                    prev_texture = -1;
                    prev_prop = ptr::null();
                    num_textures_in_material = 0;
                }
                if mat_tex_texture_id == prev_texture && mat_tex_prop_name.data == prev_prop {
                    continue;
                }
                prev_texture = mat_tex_texture_id;
                prev_prop = mat_tex_prop_name.data;

                let tex: *mut MaterialTexture = uc.tmp_stack_view().push::<MaterialTexture>(1);
                ufbxi_check!(uc, !tex.is_null(), "tex");
                ufbx_assert!(prev_texture >= 0 && (prev_texture as usize) < textures_view.count());
                // SAFETY: `tex` is the non-null one-element push just made into
                // `uc`'s tmp stack, so it heads a live `ufbx_material_texture` with
                // write-capable arena provenance.
                let tex: &View<MaterialTexture> = unsafe { View::<MaterialTexture>::from_ptr(tex) };
                // C indexes the fetched run from its list base; the assert above
                // bounds `prev_texture` by that run's length.
                // SAFETY: the index is within the fetched texture run, whose slots
                // hold live `ufbx_texture` references.
                tex.set_texture(unsafe { *textures_view.data().add(prev_texture as usize) });
                // C: `tex->shader_prop = tex->material_prop = mat_tex.prop_name;`
                tex.set_material_prop(mat_tex_prop_name);
                tex.set_shader_prop(tex.material_prop());
                num_textures_in_material += 1;
            }
            // SAFETY: `p_mesh != p_mesh_end`, so the advance lands at or before the
            // run's one-past-the-end pointer.
            p_mesh = unsafe { p_mesh.add(1) };
        }
    }

    resolve_file_content(uc)?;

    // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, uc->scene.textures)`
    let scene_textures: &RefListView<Texture> = uc.scene_view().textures_view();
    for texture_ix in 0..scene_textures.count() {
        let texture: &'a TextureView = scene_textures.at(texture_ix);
        let extra: *mut TextureExtra =
            get_element_extra(uc, texture.element().element_id()) as *mut TextureExtra;

        let uv_set: Option<&PropView> = find_prop(texture.props_view(), &sp::UVSet);
        if let Some(uv_set) = uv_set {
            texture.set_uv_set(uv_set.value_str());
        } else {
            texture.set_uv_set(EMPTY_STRING.0);
        }

        // SAFETY: `element_raw()` addresses the viewed texture's own element
        // header; the fetched destination is null or a live `ufbx_video`.
        texture.set_video(unsafe {
            opt_ref(fetch_dst_element(
                texture.element_raw(),
                false,
                ptr::null(),
                ElementType::Video,
            ) as *mut Video)
        });
        if let Some(texture_video) = texture.video() {
            // SAFETY: a non-null `video` names a live `ufbx_video` element of
            // the uc-owned scene, so its provenance is write-capable; only the
            // `content` field is projected out of it.
            let texture_video: &View<Video> =
                unsafe { View::<Video>::from_ptr(texture_video.ptr()) };
            texture.set_content(texture_video.content());
        }

        finalize_shader_texture(uc, texture)?;

        // SAFETY: the projections address the viewed texture's own `ufbx_string`
        // filename fields, which are `Strblob`-shaped for `raw` `false`, and carry
        // the texture view's write-capable provenance.
        resolve_filenames(
            uc,
            unsafe { View::<Strblob>::from_ptr(texture.filename_raw() as *mut Strblob) },
            unsafe { View::<Strblob>::from_ptr(texture.absolute_filename_raw() as *mut Strblob) },
            unsafe { View::<Strblob>::from_ptr(texture.relative_filename_raw() as *mut Strblob) },
            false,
        )?;
        // SAFETY: as above, for its own `ufbx_blob` raw filename fields, which are
        // `Strblob`-shaped for `raw` `true`.
        resolve_filenames(
            uc,
            unsafe { View::<Strblob>::from_ptr(texture.raw_filename_raw() as *mut Strblob) },
            unsafe {
                View::<Strblob>::from_ptr(texture.raw_absolute_filename_raw() as *mut Strblob)
            },
            unsafe {
                View::<Strblob>::from_ptr(texture.raw_relative_filename_raw() as *mut Strblob)
            },
            true,
        )?;

        // Fetch layered texture layers and patch alphas/blend modes
        if texture.type_() == TextureType::Layered {
            fetch_texture_layers(uc, texture.layers_view(), texture.element())?;
            if !extra.is_null() {
                // SAFETY: `extra` is the non-null per-element extra fetched above,
                // so it heads a live `ufbxi_texture_extra` in `uc`'s tmp arena.
                let extra: &View<TextureExtra> = unsafe { View::<TextureExtra>::from_ptr(extra) };
                // SAFETY: the extra's `alphas`/`num_alphas` describe one live,
                // contiguous parsed-array run of `ufbx_real`.
                let alphas: &[Real] =
                    unsafe { slice_from_ptr(extra.alphas() as *const Real, extra.num_alphas()) };
                let num: usize = min_sz(extra.num_alphas(), texture.layers_view().count());
                for i in 0..num {
                    texture.layers_view().at(i).set_alpha(alphas[i]);
                }
                // SAFETY: the extra's `blend_modes`/`num_blend_modes` describe one
                // live, contiguous parsed-array run of `int32_t`.
                let blend_modes: &[i32] = unsafe {
                    slice_from_ptr(extra.blend_modes() as *const i32, extra.num_blend_modes())
                };
                let num: usize = min_sz(extra.num_blend_modes(), texture.layers_view().count());
                for i in 0..num {
                    let mode: i32 = blend_modes[i];
                    if mode >= 0 && mode < BlendMode::Overlay as i32 {
                        // C: `(ufbx_blend_mode)mode` — the guard above pins
                        // `mode` into the enum's range.
                        // SAFETY: the guard pins `mode` into the discriminants of
                        // the `u32`-repr `BlendMode`.
                        let mode: BlendMode =
                            unsafe { core::mem::transmute::<u32, BlendMode>(mode as u32) };
                        texture.layers_view().at(i).set_blend_mode(mode);
                    }
                }
            }
        }

        insert_texture_file(uc, texture)?;
    }

    propagate_main_textures(uc.scene_view());
    pop_texture_files(uc)?;

    // Second pass to fetch material maps
    // C: `ufbxi_for_ptr_list(ufbx_material, p_material, uc->scene.materials)`
    let scene_materials: &RefListView<Material> = uc.scene_view().materials_view();
    for material_ix in 0..scene_materials.count() {
        let material: &MaterialView = scene_materials.at(material_ix);

        // SAFETY: the viewed material's `textures` `data`/`count` describe its own
        // run of `count` initialized `ufbx_material_texture`s.
        unsafe {
            sort_material_textures(
                uc,
                material.textures_view().data() as *mut MaterialTexture,
                material.textures_view().count(),
            )
        }?;
        fetch_maps(uc.scene_view(), material);

        // Fetch `ufbx_material_texture.shader_prop` names
        // C: `if (material->shader)`
        if let Some(material_shader) = material.shader() {
            // SAFETY: a non-null `shader` names a live `ufbx_shader` element of
            // the uc-owned scene, so its provenance is write-capable.
            let material_shader: &View<Shader> =
                unsafe { View::<Shader>::from_ptr(material_shader.ptr()) };
            // C: `ufbxi_for_ptr_list(ufbx_shader_binding, p_binding, material->shader->bindings)`
            let bindings: &RefListView<ShaderBinding> = material_shader.bindings_view();
            for binding_ix in 0..bindings.count() {
                let binding: &View<ShaderBinding> = bindings.at(binding_ix);

                // C: `ufbxi_for_list(ufbx_shader_prop_binding, prop, binding->prop_bindings)`
                // SAFETY: the viewed binding's `prop_bindings` `data`/`count`
                // describe its own contiguous, initialized prop-binding run,
                // owned by the uc result arena.
                let prop_iter = unsafe {
                    SliceViewIter::<ShaderPropBinding>::from_raw_parts(
                        binding.prop_bindings_view().data() as *mut ShaderPropBinding,
                        binding.prop_bindings_view().count(),
                    )
                };
                for prop in prop_iter {
                    let name: String = prop.material_prop();

                    // C: `size_t index = SIZE_MAX;` — `ufbxi_macro_lower_bound_eq`
                    // leaves it untouched on a miss, which the loop below drops.
                    // The query key's bytes are minted ONCE for the whole
                    // search.
                    // SAFETY: `name` is an interned pool string, so its
                    // `data`/`length` describe one live, initialized byte run.
                    let name_bytes: &[u8] = unsafe { name.as_bytes() };
                    let index: Option<usize> = material.textures_view().lower_bound_eq(
                        4,
                        // SAFETY: the probed element's `material_prop` is an
                        // interned pool string, same as `name`.
                        |a| str_less(unsafe { a.material_prop().as_bytes() }, name_bytes),
                        |a| a.material_prop().data == name.data,
                    );
                    let mut index: usize = index.unwrap_or(usize::MAX);
                    while index < material.textures_view().count()
                        && material.textures_view().at(index).shader_prop().data == name.data
                    {
                        material
                            .textures_view()
                            .at(index)
                            .set_shader_prop(prop.shader_prop());
                        index += 1;
                    }
                }
            }
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_display_layer, p_layer, uc->scene.display_layers)`
    let scene_display_layers: &RefListView<DisplayLayer> = uc.scene_view().display_layers_view();
    for layer_ix in 0..scene_display_layers.count() {
        let layer: &DisplayLayerView = scene_display_layers.at(layer_ix);
        // SAFETY: `layer.element_raw()` addresses that element's
        // header with whole-struct provenance.
        unsafe {
            fetch_dst_elements(
                uc,
                layer.nodes_view().as_element_list(),
                layer.element_raw(),
                false,
                true,
                None,
                ElementType::Node,
            )
        }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_selection_set, p_set, uc->scene.selection_sets)`
    let scene_selection_sets: &RefListView<SelectionSet> = uc.scene_view().selection_sets_view();
    for set_ix in 0..scene_selection_sets.count() {
        let set: &View<SelectionSet> = scene_selection_sets.at(set_ix);
        // SAFETY: `set.element_raw()` addresses that element's
        // header with whole-struct provenance.
        unsafe {
            fetch_dst_elements(
                uc,
                set.nodes_view().as_element_list(),
                set.element_raw(),
                false,
                true,
                None,
                ElementType::SelectionNode,
            )
        }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_selection_node, p_node, uc->scene.selection_nodes)`
    let scene_selection_nodes: &RefListView<SelectionNode> = uc.scene_view().selection_nodes_view();
    for node_ix in 0..scene_selection_nodes.count() {
        let node: &View<SelectionNode> = scene_selection_nodes.at(node_ix);
        // SAFETY: `element_raw()` addresses the viewed selection node's own
        // element header; the fetched destination is null or a live `ufbx_node`.
        node.set_target_node(unsafe {
            opt_ref(
                fetch_dst_element(node.element_raw(), false, ptr::null(), ElementType::Node)
                    as *mut Node,
            )
        });
        // SAFETY: as above, for a null-or-live `ufbx_mesh` destination.
        node.set_target_mesh(unsafe {
            opt_ref(
                fetch_dst_element(node.element_raw(), false, ptr::null(), ElementType::Mesh)
                    as *mut Mesh,
            )
        });
        // C: `if (!node->target_mesh && node->target_node) ... else if
        // (!node->target_node && node->target_mesh && node->target_mesh->instances.count > 0)`
        match (node.target_mesh(), node.target_node()) {
            (None, Some(target_node)) => {
                // SAFETY: a non-null `target_node` names a live `ufbx_node`
                // element of the uc-owned scene; only its own `mesh` field is
                // projected out of it.
                let target_node: &NodeView = unsafe { NodeView::from_ptr(target_node.ptr()) };
                node.set_target_mesh(target_node.mesh());
            }
            (Some(target_mesh), None) => {
                // SAFETY: a non-null `target_mesh` names a live `ufbx_mesh`
                // element of the uc-owned scene, so its provenance is
                // write-capable.
                let target_mesh: &View<Mesh> = unsafe { View::<Mesh>::from_ptr(target_mesh.ptr()) };
                let instances: &RefListView<Node> = target_mesh.element().instances_view();
                if instances.count() > 0 {
                    // C: `node->target_mesh->instances.data[0]` — indexed from
                    // the list base.
                    // SAFETY: the instance list is non-empty, so its element `0`
                    // is a live, initialized node reference.
                    node.set_target_node(Some(unsafe { *instances.data() }));
                }
            }
            _ => {}
        }

        // C: `ufbx_mesh *mesh = node->target_mesh;`
        if let Some(mesh) = node.target_mesh() {
            // SAFETY: a non-null `target_mesh` names a live `ufbx_mesh` element of
            // the uc-owned scene, so its provenance is write-capable and `Mut` is
            // the right mode.
            let mesh: &View<Mesh> = unsafe { View::<Mesh>::from_ptr(mesh.ptr()) };
            validate_indices(uc, node.vertices_view(), mesh.num_vertices())?;
            validate_indices(uc, node.edges_view(), mesh.num_edges())?;
            validate_indices(uc, node.faces_view(), mesh.num_faces())?;
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_constraint, p_constraint, uc->scene.constraints)`
    let scene_constraints: &RefListView<Constraint> = uc.scene_view().constraints_view();
    for constraint_ix in 0..scene_constraints.count() {
        let constraint: &ConstraintView = scene_constraints.at(constraint_ix);

        let tmp_base: usize = uc.tmp_stack_view().num_items();

        // Find property connections in _both_ src and dst connections as they are inconsistent
        // in pre-7000 files. For example "Constrained Object" is a "PO" connection in 6100.
        // C: `ufbxi_for_list(ufbx_connection, conn, constraint->element.connections_src)`
        // SAFETY: the viewed constraint's `connections_src` `data`/`count` describe
        // one contiguous, initialized connection run in `uc`'s result arena.
        let conn_iter = unsafe {
            SliceViewIter::<Connection>::from_raw_parts(
                constraint.element().connections_src_view().data() as *mut Connection,
                constraint.element().connections_src_view().count(),
            )
        };
        for conn in conn_iter {
            // C: `conn->dst` — the whole element pointer; the view over it covers
            // the `ufbx_element` header only, so the cast to `ufbx_node*` below is
            // taken from the stored pointer rather than derived from the view.
            let conn_dst: *mut Element = conn.dst().ptr();
            // SAFETY: a connection's `dst` is a non-null reference to a live scene
            // element; only its own `type_` field is projected out of it.
            let dst: &ElementView = unsafe { ElementView::from_ptr(conn_dst) };
            if conn.src_prop().length == 0 || dst.type_() != ElementType::Node {
                continue;
            }
            // SAFETY: the check above pins the destination's type, so the element
            // heads a live `ufbx_node` of the uc-owned scene and its provenance is
            // write-capable.
            let node: &NodeView = unsafe { NodeView::from_ptr(conn_dst as *mut Node) };
            add_constraint_prop(uc, constraint, node, conn.src_prop_view().bytes())?;
        }
        // C: `ufbxi_for_list(ufbx_connection, conn, constraint->element.connections_dst)`
        // SAFETY: as above, for the constraint's own `connections_dst` run.
        let conn_iter = unsafe {
            SliceViewIter::<Connection>::from_raw_parts(
                constraint.element().connections_dst_view().data() as *mut Connection,
                constraint.element().connections_dst_view().count(),
            )
        };
        for conn in conn_iter {
            // C: `conn->src` — the whole element pointer (see the `dst` walk above).
            let conn_src: *mut Element = conn.src().ptr();
            // SAFETY: a connection's `src` is a non-null reference to a live scene
            // element; only its own `type_` field is projected out of it.
            let src: &ElementView = unsafe { ElementView::from_ptr(conn_src) };
            if conn.dst_prop().length == 0 || src.type_() != ElementType::Node {
                continue;
            }
            // SAFETY: the check above pins the source's type, so the element heads
            // a live `ufbx_node` of the uc-owned scene and its provenance is
            // write-capable.
            let node: &NodeView = unsafe { NodeView::from_ptr(conn_src as *mut Node) };
            add_constraint_prop(uc, constraint, node, conn.dst_prop_view().bytes())?;
        }

        let num_targets: usize = uc.tmp_stack_view().num_items() - tmp_base;
        constraint.targets_view().set_count(num_targets);
        constraint.targets_view().set_data(
            uc.result_view()
                .push_pop::<ConstraintTarget>(uc.tmp_stack_view(), num_targets),
        );
        ufbxi_check!(
            uc,
            !constraint.targets_view().data().is_null(),
            "constraint->targets.data"
        );
    }

    // C: `ufbxi_for_ptr_list(ufbx_audio_layer, p_layer, uc->scene.audio_layers)`
    let scene_audio_layers: &RefListView<AudioLayer> = uc.scene_view().audio_layers_view();
    for layer_ix in 0..scene_audio_layers.count() {
        let layer: &View<AudioLayer> = scene_audio_layers.at(layer_ix);
        // SAFETY: `layer.element_raw()` addresses that element's
        // header with whole-struct provenance.
        unsafe {
            fetch_dst_elements(
                uc,
                layer.clips_view().as_element_list(),
                layer.element_raw(),
                false,
                true,
                None,
                ElementType::AudioClip,
            )
        }?;
    }

    // C: `ufbxi_for_ptr_list(ufbx_lod_group, p_lod, uc->scene.lod_groups)`
    let scene_lod_groups: &RefListView<LodGroup> = uc.scene_view().lod_groups_view();
    for lod_ix in 0..scene_lod_groups.count() {
        let lod: &'a LodGroupView = scene_lod_groups.at(lod_ix);
        finalize_lod_group(uc, lod)?;
    }

    fetch_file_textures(uc)?;

    // NOTE: This will be patched over in `ufbxi_update_scene()` if there are `anim_layers`
    if uc.scene_view().anim_layers_view().count() == 0 {
        // SAFETY: `anim_mut_ptr` addresses the scene's own anim-pointer slot; the
        // layer run is empty, so a null pointer with count `0` describes it.
        unsafe {
            push_anim(
                uc,
                uc.scene_view().anim_mut_ptr() as *mut *mut Anim,
                ptr::null_mut(),
                0,
            )
        }?;
    }

    uc.scene_view()
        .metadata_view()
        .set_ktime_second(uc.ktime_sec());

    // Maya seems to use scale of 100/3, Blender binary uses exactly 33, ASCII has always value of 1.0
    if uc.version() < 6000 {
        uc.scene_view()
            .metadata_view()
            .set_bone_prop_size_unit(1.0f32 as Real);
    } else if uc.exporter() == Exporter::BlenderBinary {
        uc.scene_view()
            .metadata_view()
            .set_bone_prop_size_unit(33.0f32 as Real);
    } else if uc.exporter() == Exporter::BlenderAscii {
        uc.scene_view()
            .metadata_view()
            .set_bone_prop_size_unit(1.0f32 as Real);
    } else {
        uc.scene_view()
            .metadata_view()
            .set_bone_prop_size_unit((100.0 / 3.0) as Real);
    }
    if uc.exporter() == Exporter::BlenderAscii {
        uc.scene_view()
            .metadata_view()
            .set_bone_prop_limb_length_relative(false);
    } else {
        uc.scene_view()
            .metadata_view()
            .set_bone_prop_limb_length_relative(true);
    }

    Ok(())
}

// -- Interpret the read scene (ufbx.c:22626-22741)
//
// `ufbxi_modify_geometry` (ufbx.c:21165) needs
// `ufbxi_get_geometry_transform`, which C forward-declares at
// ufbx.c:21070-21071.

// ufbx.c:22628-22633 `ufbxi_add_translate`
#[inline(always)]
pub(crate) fn add_translate(t: &mut Transform, v: Vec3) {
    t.translation.x += v.x;
    t.translation.y += v.y;
    t.translation.z += v.z;
}

// ufbx.c:22635-22640 `ufbxi_sub_translate`
#[inline(always)]
pub(crate) fn sub_translate(t: &mut Transform, v: Vec3) {
    t.translation.x -= v.x;
    t.translation.y -= v.y;
    t.translation.z -= v.z;
}

// ufbx.c:22642-22650 `ufbxi_mul_scale`
#[inline(always)]
pub(crate) fn mul_scale(t: &mut Transform, v: Vec3) {
    t.translation.x *= v.x;
    t.translation.y *= v.y;
    t.translation.z *= v.z;
    t.scale.x *= v.x;
    t.scale.y *= v.y;
    t.scale.z *= v.z;
}

// ufbx.c:22652-22660 `ufbxi_mul_scale_real`
#[inline(always)]
pub(crate) fn mul_scale_real(t: &mut Transform, v: Real) {
    t.translation.x *= v;
    t.translation.y *= v;
    t.translation.z *= v;
    t.scale.x *= v;
    t.scale.y *= v;
    t.scale.z *= v;
}

// ufbx.c:22662-22670 `ufbxi_mul_quat`
#[inline(never)]
pub(crate) fn mul_quat(a: Quat, b: Quat) -> Quat {
    // C: `ufbx_quat r;` — every field is written below before the return, so
    // the zero-fill is inert (upstream carries no `// ufbxi_uninit` marker).
    // SAFETY: `Quat` is POD (four `Real`s); the all-zero bit pattern is valid
    // and every field is overwritten before `r` is read.
    let mut r: Quat = unsafe { core::mem::zeroed() };
    r.x = a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y;
    r.y = a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x;
    r.z = a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w;
    r.w = a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z;
    r
}

// ufbx.c:22672-22677 `ufbxi_add_weighted_vec3`
#[inline(always)]
pub(crate) unsafe fn add_weighted_vec3(r: *mut Vec3, b: Vec3, w: Real) {
    // SAFETY: `r` points to a live, initialized, writable `ufbx_vec3`
    // accumulator (fn contract).
    unsafe { (*r).x += b.x * w };
    // SAFETY: as above, for the accumulator's `y`.
    unsafe { (*r).y += b.y * w };
    // SAFETY: as above, for the accumulator's `z`.
    unsafe { (*r).z += b.z * w };
}

// ufbx.c:22679-22685 `ufbxi_add_weighted_quat`
#[inline(always)]
pub(crate) unsafe fn add_weighted_quat(r: *mut Quat, b: Quat, w: Real) {
    // SAFETY: `r` points to a live, initialized, writable `ufbx_quat`
    // accumulator (fn contract).
    unsafe { (*r).x += b.x * w };
    // SAFETY: as above, for the accumulator's `y`.
    unsafe { (*r).y += b.y * w };
    // SAFETY: as above, for the accumulator's `z`.
    unsafe { (*r).z += b.z * w };
    // SAFETY: as above, for the accumulator's `w`.
    unsafe { (*r).w += b.w * w };
}

// ufbx.c:22687-22693 `ufbxi_add_weighted_mat`
// C indexes the `ufbx_matrix` value union's `ufbx_vec3 cols[4]` view; the
// generated struct keeps only the named `m00`..`m23` fields, which are laid out
// exactly as four consecutive `ufbx_vec3` columns.
#[inline(never)]
pub(crate) unsafe fn add_weighted_mat(r: *mut Matrix, b: *const Matrix, w: Real) {
    let r_cols: *mut Vec3 = r as *mut Vec3;
    let b_cols: *const Vec3 = b as *const Vec3;
    // SAFETY: `r` and `b` point to live, initialized `ufbx_matrix` values (fn
    // contract, `r` writable), and each is laid out as exactly four consecutive
    // `ufbx_vec3` columns, so column `0` of both is in bounds.
    unsafe { add_weighted_vec3(r_cols.add(0), *b_cols.add(0), w) };
    // SAFETY: as above, for column `1` of both matrices.
    unsafe { add_weighted_vec3(r_cols.add(1), *b_cols.add(1), w) };
    // SAFETY: as above, for column `2` of both matrices.
    unsafe { add_weighted_vec3(r_cols.add(2), *b_cols.add(2), w) };
    // SAFETY: as above, for column `3` of both matrices.
    unsafe { add_weighted_vec3(r_cols.add(3), *b_cols.add(3), w) };
}

// ufbx.c:22695-22709 `ufbxi_mul_rotate`
pub(crate) fn mul_rotate(t: &mut Transform, v: Vec3, order: RotationOrder) {
    if is_vec3_zero(v) {
        return;
    }

    let q: Quat = euler_to_quat(v, order);
    if t.rotation.w != 1.0 {
        t.rotation = mul_quat(q, t.rotation);
    } else {
        t.rotation = q;
    }

    if !is_vec3_zero(t.translation) {
        t.translation = quat_rotate_vec3(q, t.translation);
    }
}

// ufbx.c:22711-22724 `ufbxi_mul_rotate_quat`
pub(crate) fn mul_rotate_quat(t: &mut Transform, q: Quat) {
    if is_quat_identity(q) {
        return;
    }

    if t.rotation.w != 1.0 {
        t.rotation = mul_quat(q, t.rotation);
    } else {
        t.rotation = q;
    }

    if !is_vec3_zero(t.translation) {
        t.translation = quat_rotate_vec3(q, t.translation);
    }
}

// ufbx.c:22726-22741 `ufbxi_mul_inv_rotate`
pub(crate) fn mul_inv_rotate(t: &mut Transform, v: Vec3, order: RotationOrder) {
    if is_vec3_zero(v) {
        return;
    }

    let mut q: Quat = euler_to_quat(v, order);
    q.x = -q.x;
    q.y = -q.y;
    q.z = -q.z;
    if t.rotation.w != 1.0 {
        t.rotation = mul_quat(q, t.rotation);
    } else {
        t.rotation = q;
    }

    if !is_vec3_zero(t.translation) {
        t.translation = quat_rotate_vec3(q, t.translation);
    }
}

// -- Updating state from properties (ufbx.c:22743-…)
//
// The head of this banner section (ufbx.c:22745-22784) contains the three
// helpers `ufbxi_modify_geometry` (ufbx.c:21165) depends on, followed by
// `ufbxi_get_rotation` through `ufbxi_update_light` (22786-23062).

// ufbx.c:22745-22749 `ufbxi_mirror_translation`
// C indexes the `ufbx_vec3` value union's `ufbx_real v[3]` view; the generated
// struct keeps only `x`/`y`/`z`, so the index is pointer arithmetic from the
// struct base (same device as `ufbxi_mirror_vec3_list` above).
//
// # Safety
//
// `axis` must be one of `X`/`Y`/`Z`: the C indexes `v[axis - 1]`, which reads
// out of bounds for `UFBX_MIRROR_AXIS_NONE`. The parameter type admits `None`,
// so the obligation stays with the caller (C states it as `ufbxi_dev_assert`).
#[inline(always)]
pub(crate) unsafe fn mirror_translation(p_vec: &mut Vec3, axis: MirrorAxis) {
    // C: `ufbxi_dev_assert(axis);` — enum truthiness.
    ufbxi_dev_assert!(axis != MirrorAxis::None);
    let v: *mut Real = ptr::from_mut(p_vec).cast::<Real>();
    // C: `axis - 1` — the enum is promoted to `int` before the subtraction.
    let i: usize = (axis as i32 - 1) as usize;
    // SAFETY: `p_vec` borrows a live, initialized, writable `ufbx_vec3`, which
    // is three consecutive `ufbx_real`s, and `axis` is one of `X`/`Y`/`Z` (fn
    // contract, asserted above) so `i = axis - 1` is in `0..3`.
    unsafe { *v.add(i) = -*v.add(i) };
}

// ufbx.c:22751-22756 `ufbxi_mirror_rotation`
// Same `ufbx_quat.v[4]` union view as `ufbxi_mirror_translation` above. Every
// `ufbx_mirror_axis` value keeps `axis % 3` and `(axis + 1) % 3` inside the
// quaternion's four reals, so the axis carries no safety obligation here.
#[inline(always)]
pub(crate) fn mirror_rotation(p_quat: &mut Quat, axis: MirrorAxis) {
    // C: `ufbxi_dev_assert(axis);` — enum truthiness.
    ufbxi_dev_assert!(axis != MirrorAxis::None);
    let v: *mut Real = ptr::from_mut(p_quat).cast::<Real>();
    // C: `axis % 3` / `(axis + 1) % 3` — the enum is promoted to `int` first.
    let i0: usize = (axis as i32 % 3) as usize;
    // SAFETY: `p_quat` borrows a live, initialized, writable `ufbx_quat`, which
    // is four consecutive `ufbx_real`s, and `i0 = axis % 3` is in `0..3`.
    unsafe { *v.add(i0) = -*v.add(i0) };
    let i1: usize = ((axis as i32 + 1) % 3) as usize;
    // SAFETY: as above; `i1 = (axis + 1) % 3` is in `0..3`.
    unsafe { *v.add(i1) = -*v.add(i1) };
}

// ufbx.c:22758-22784 `ufbxi_get_geometry_transform`
// (forward-declared at ufbx.c:21070-21071 for `ufbxi_modify_geometry`)
#[inline(never)]
pub(crate) fn get_geometry_transform(props: &PropsView, node: &NodeView) -> Transform {
    let translation: Vec3 = find_vec3(props, &sp::GeometricTranslation, 0.0, 0.0, 0.0);
    let rotation: Vec3 = find_vec3(props, &sp::GeometricRotation, 0.0, 0.0, 0.0);
    let scaling: Vec3 = find_vec3(props, &sp::GeometricScaling, 1.0, 1.0, 1.0);

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

    if node.has_adjust_transform() {
        t.translation.x *= node.adjust_translation_scale();
        t.translation.y *= node.adjust_translation_scale();
        t.translation.z *= node.adjust_translation_scale();
    }

    if node.adjust_mirror_axis() != MirrorAxis::None {
        // SAFETY: the branch condition established that the axis is not `None`,
        // which is `ufbxi_mirror_translation`'s contract.
        unsafe { mirror_translation(&mut t.translation, node.adjust_mirror_axis()) };
        mirror_rotation(&mut t.rotation, node.adjust_mirror_axis());
    }

    t
}

// ufbx.c:22786-22815 `ufbxi_get_rotation`
// Fast path for `ufbxi_get_transform` below: the rotation-only subset of that
// function's composition chain. The two are pinned together by the
// `ufbxi_regression_assert` at ufbx.c:22901.
#[inline(never)]
pub(crate) unsafe fn get_rotation<M: Mode>(
    props: &View<Props, M>,
    order: RotationOrder,
    node: *const Node,
) -> Quat {
    let rotation: Vec3 = find_vec3(props, &sp::Lcl_Rotation, 0.0, 0.0, 0.0);
    let pre_rotation: Vec3 = find_vec3(props, &sp::PreRotation, 0.0, 0.0, 0.0);
    let post_rotation: Vec3 = find_vec3(props, &sp::PostRotation, 0.0, 0.0, 0.0);

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

    // SAFETY: `node` points to the live, initialized `ufbx_node` whose rotation
    // is being composed (fn contract).
    unsafe {
        if (*node).has_adjust_transform {
            mul_rotate_quat(&mut t, (*node).adjust_post_rotation);
        }
    }

    // SAFETY: `node` is live (see above).
    if unsafe { (*node).use_rotation_space } {
        mul_inv_rotate(&mut t, post_rotation, RotationOrder::Xyz);
        mul_rotate(&mut t, rotation, order);
        mul_rotate(&mut t, pre_rotation, RotationOrder::Xyz);
    } else {
        mul_rotate(&mut t, rotation, RotationOrder::Xyz);
    }

    // SAFETY: `node` is live (see above).
    unsafe {
        if (*node).has_adjust_transform {
            mul_rotate_quat(&mut t, (*node).adjust_pre_rotation);
        }
    }

    // C: `if (node->adjust_mirror_axis)` — enum truthiness.
    // SAFETY: `node` is live (see above).
    unsafe {
        if (*node).adjust_mirror_axis != MirrorAxis::None {
            mirror_rotation(&mut t.rotation, (*node).adjust_mirror_axis);
        }
    }

    t.rotation
}

// ufbx.c:22817-22834 `ufbxi_get_scale`
// Scale-only fast path, pinned to `ufbxi_get_transform` by the
// `ufbxi_regression_assert` at ufbx.c:22902.
#[inline(never)]
pub(crate) unsafe fn get_scale<M: Mode>(props: &View<Props, M>, node: *const Node) -> Vec3 {
    let scaling: Vec3 = find_vec3(props, &sp::Lcl_Scaling, 1.0, 1.0, 1.0);

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

    // SAFETY: `node` points to the live, initialized `ufbx_node` whose scale is
    // being composed (fn contract).
    unsafe {
        if (*node).has_adjust_transform {
            mul_scale_real(&mut t, (*node).adjust_post_scale);
        }
    }

    mul_scale(&mut t, scaling);

    // SAFETY: `node` is live (see above).
    unsafe {
        if (*node).has_adjust_transform {
            mul_scale_real(&mut t, (*node).adjust_pre_scale);
        }
    }

    t.scale
}

// ufbx.c:22836-22905 `ufbxi_get_transform`
#[inline(never)]
pub(crate) unsafe fn get_transform<M: Mode>(
    props: &View<Props, M>,
    order: RotationOrder,
    node: *const Node,
    translation_scale: *const Vec3,
) -> Transform {
    let scale_pivot: Vec3 = find_vec3(props, &sp::ScalingPivot, 0.0, 0.0, 0.0);
    let rot_pivot: Vec3 = find_vec3(props, &sp::RotationPivot, 0.0, 0.0, 0.0);
    let scale_offset: Vec3 = find_vec3(props, &sp::ScalingOffset, 0.0, 0.0, 0.0);
    let rot_offset: Vec3 = find_vec3(props, &sp::RotationOffset, 0.0, 0.0, 0.0);

    let mut translation: Vec3 = find_vec3(props, &sp::Lcl_Translation, 0.0, 0.0, 0.0);
    let rotation: Vec3 = find_vec3(props, &sp::Lcl_Rotation, 0.0, 0.0, 0.0);
    let scaling: Vec3 = find_vec3(props, &sp::Lcl_Scaling, 1.0, 1.0, 1.0);

    let pre_rotation: Vec3 = find_vec3(props, &sp::PreRotation, 0.0, 0.0, 0.0);
    let post_rotation: Vec3 = find_vec3(props, &sp::PostRotation, 0.0, 0.0, 0.0);

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
        // SAFETY: `translation_scale` is non-null here, and the fn contract is
        // that a non-null one points to a live, initialized `ufbx_vec3`.
        unsafe {
            translation.x *= (*translation_scale).x;
            translation.y *= (*translation_scale).y;
            translation.z *= (*translation_scale).z;
        }
    }

    // SAFETY: `node` points to the live, initialized `ufbx_node` whose transform
    // is being composed (fn contract).
    unsafe {
        if (*node).has_adjust_transform {
            mul_rotate_quat(&mut t, (*node).adjust_post_rotation);
            mul_scale_real(&mut t, (*node).adjust_post_scale);
        }
    }

    sub_translate(&mut t, scale_pivot);
    mul_scale(&mut t, scaling);
    add_translate(&mut t, scale_pivot);

    add_translate(&mut t, scale_offset);

    sub_translate(&mut t, rot_pivot);
    // SAFETY: `node` is live (see above).
    if unsafe { (*node).use_rotation_space } {
        mul_inv_rotate(&mut t, post_rotation, RotationOrder::Xyz);
        mul_rotate(&mut t, rotation, order);
        mul_rotate(&mut t, pre_rotation, RotationOrder::Xyz);
    } else {
        mul_rotate(&mut t, rotation, RotationOrder::Xyz);
    }
    add_translate(&mut t, rot_pivot);

    add_translate(&mut t, rot_offset);

    add_translate(&mut t, translation);

    // SAFETY: `node` is live (see above).
    unsafe {
        if (*node).has_adjust_transform {
            add_translate(&mut t, (*node).adjust_pre_translation);
            mul_rotate_quat(&mut t, (*node).adjust_pre_rotation);
            mul_scale_real(&mut t, (*node).adjust_pre_scale);
            t.translation.x *= (*node).adjust_translation_scale;
            t.translation.y *= (*node).adjust_translation_scale;
            t.translation.z *= (*node).adjust_translation_scale;
        }
    }

    // C: `if (node->adjust_mirror_axis)` — enum truthiness.
    // SAFETY: `node` is live (see above); the branch condition established that
    // the axis is not `None`, which is `ufbxi_mirror_translation`'s contract.
    unsafe {
        if (*node).adjust_mirror_axis != MirrorAxis::None {
            mirror_translation(&mut t.translation, (*node).adjust_mirror_axis);
            mirror_rotation(&mut t.rotation, (*node).adjust_mirror_axis);
        }
    }

    // Make sure the fast paths are identical to this function.
    // SAFETY: `props` and `node` are the same arguments this fn received, so the
    // rotation-only fast path's contract is discharged by this fn's own.
    ufbxi_regression_assert!(is_quat_equal(t.rotation, unsafe {
        get_rotation(props, order, node)
    }));
    // SAFETY: as above, for the scale-only fast path.
    ufbxi_regression_assert!(is_vec3_equal(t.scale, unsafe { get_scale(props, node) }));

    t
}

// ufbx.c:22907-22936 `ufbxi_get_texture_transform`
#[inline(never)]
pub(crate) fn get_texture_transform(props: &PropsView) -> Transform {
    let (scale_pivot, rot_pivot, translation, rotation, scaling) = (
        find_vec3(props, &sp::TextureScalingPivot, 0.0, 0.0, 0.0),
        find_vec3(props, &sp::TextureRotationPivot, 0.0, 0.0, 0.0),
        find_vec3(props, &sp::Translation, 0.0, 0.0, 0.0),
        find_vec3(props, &sp::Rotation, 0.0, 0.0, 0.0),
        find_vec3(props, &sp::Scaling, 1.0, 1.0, 1.0),
    );

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

    if find_int(props, &sp::UVSwap, 0) != 0 {
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
pub(crate) fn get_constraint_transform(props: &PropsView) -> Transform {
    let (translation, rotation, rotation_offset, scaling) = (
        find_vec3(props, &sp::Translation, 0.0, 0.0, 0.0),
        find_vec3(props, &sp::Rotation, 0.0, 0.0, 0.0),
        find_vec3(props, &sp::RotationOffset, 0.0, 0.0, 0.0),
        find_vec3(props, &sp::Scaling, 1.0, 1.0, 1.0),
    );

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
// C threads the overrides as a raw (`const ufbx_transform_override *overrides`,
// `size_t num_overrides`) pair; here they arrive as one slice, formed exactly
// once from that pair at the `ufbxi_update_scene` boundary where it enters
// from the public evaluate ABI.
#[inline(never)]
pub(crate) fn update_node(node_view: &NodeView, overrides: &[TransformOverride]) {
    // C: `(ufbx_rotation_order)ufbxi_find_enum(...)` — `ufbxi_find_enum` clamps
    // the result to `[0, UFBX_ROTATION_ORDER_SPHERIC]`, every value of which is
    // a valid `ufbx_rotation_order`.
    // SAFETY: `ufbxi_find_enum` clamps its result to the `[XYZ, SPHERIC]` range
    // passed, every value of which is a valid `ufbx_rotation_order`
    // discriminant, so the transmute produces an inhabited enum value.
    let rotation_order: RotationOrder = unsafe {
        core::mem::transmute::<u32, RotationOrder>(find_enum(
            node_view.props_view(),
            &sp::RotationOrder,
            RotationOrder::Xyz as i64,
            RotationOrder::Spheric as i64,
        ) as u32)
    };
    node_view.set_rotation_order(rotation_order);
    node_view.set_euler_rotation(find_vec3(
        node_view.props_view(),
        &sp::Lcl_Rotation,
        0.0,
        0.0,
        0.0,
    ));

    if !node_view.is_root() {
        let rotation_active: bool = find_int(node_view.props_view(), &sp::RotationActive, 1) != 0;
        let rotation_limit_only: bool =
            find_int(node_view.props_view(), &sp::RotationSpaceForLimitOnly, 0) != 0;
        node_view.set_use_rotation_space(rotation_active && !rotation_limit_only);

        let mut transform_scale: *const Vec3 = ptr::null();
        // C: `if (node->parent && node->parent->scale_helper)` — the helper link
        // is reached through the parent, as in C.
        if let Some(parent) = node_view.parent() {
            // SAFETY: a non-null `parent` link points to a live, initialized
            // `ufbx_node` of the same scene, allocated from the scene arena
            // (write-capable provenance, unmoved for the update pass).
            let parent_view: &NodeView = unsafe { NodeView::from_ptr(parent.ptr()) };
            if let Some(scale_helper) = parent_view.scale_helper() {
                // SAFETY: as for `parent_view` — a non-null `scale_helper` link
                // points to a live, initialized scene node.
                let scale_helper_view: &NodeView =
                    unsafe { NodeView::from_ptr(scale_helper.ptr()) };
                // SAFETY: `local_transform_ptr()` is the helper's own live
                // transform field, so the nested `scale` member lies inside it;
                // the projection asserts nothing beyond that (no
                // `View<Transform>` impl exists to reach the member safely).
                transform_scale =
                    unsafe { &raw const (*scale_helper_view.local_transform_ptr()).scale };
            }
        }
        // SAFETY: `ufbxi_get_transform` takes the node whose transform it
        // composes by raw pointer and only reads it; `node_view.get()` is the
        // view's own live storage, and `transform_scale` is either null or the
        // live scale of the parent's scale helper (set above).
        let local_transform: Transform = unsafe {
            get_transform(
                node_view.props_view(),
                node_view.rotation_order(),
                node_view.get(),
                transform_scale,
            )
        };
        node_view.set_local_transform(local_transform);
        // C: `if (node->is_scale_helper && node->parent && node->parent->inherit_scale_node)`
        if node_view.is_scale_helper() {
            if let Some(parent) = node_view.parent() {
                // SAFETY: a non-null `parent` link points to a live,
                // initialized scene node (see above).
                let parent_view: &NodeView = unsafe { NodeView::from_ptr(parent.ptr()) };
                if let Some(inherit_scale_node) = parent_view.inherit_scale_node() {
                    // SAFETY: a non-null `inherit_scale_node` link points to a
                    // live, initialized scene node (see above).
                    let scale_parent: &NodeView =
                        unsafe { NodeView::from_ptr(inherit_scale_node.ptr()) };
                    if let Some(scale_helper) = scale_parent.scale_helper() {
                        // SAFETY: a non-null `scale_helper` link points to a
                        // live, initialized scene node (see above).
                        let scale_helper_view: &NodeView =
                            unsafe { NodeView::from_ptr(scale_helper.ptr()) };
                        let inherit_scale: Vec3 = scale_helper_view.local_transform().scale;
                        // SAFETY: `local_transform_raw()` is the node view's own
                        // live, writable transform field, so the nested `scale`
                        // members lie inside it (no `View<Transform>` impl
                        // exists to reach them safely).
                        unsafe {
                            (*node_view.local_transform_raw()).scale.x *= inherit_scale.x;
                            (*node_view.local_transform_raw()).scale.y *= inherit_scale.y;
                            (*node_view.local_transform_raw()).scale.z *= inherit_scale.z;
                        }
                    }
                }
            }
        }

        if !overrides.is_empty() {
            let typed_id: u32 = node_view.element().typed_id();
            let mut override_ix: usize = usize::MAX;
            // C: `ufbxi_macro_lower_bound_eq(ufbx_transform_override, 16,
            // &override_ix, overrides, 0, num_overrides,
            // ( a->node_id < typed_id ), ( a->node_id == typed_id ));`
            // SAFETY: the search spans exactly the `overrides` slice (sorted by
            // `node_id` per the public evaluate contract, established at the
            // `update_scene` boundary); `&mut override_ix` addresses a live
            // local, and the lambdas are handed pointers into that same run.
            unsafe {
                macro_lower_bound_eq::<TransformOverride>(
                    16,
                    &mut override_ix,
                    overrides.as_ptr(),
                    0,
                    overrides.len(),
                    |a| (*a).node_id < typed_id,
                    |a| (*a).node_id == typed_id,
                )
            };
            if override_ix != usize::MAX {
                node_view.set_local_transform(overrides[override_ix].transform);
            }
        }
        // SAFETY: `local_transform_ptr()` is the node view's own live,
        // freshly composed transform field, and `ufbx_transform_to_matrix`
        // takes it by raw pointer and only reads it.
        let node_to_parent: Matrix =
            unsafe { transform_to_matrix(node_view.local_transform_ptr()) };
        node_view.set_node_to_parent(node_to_parent);
        let geometry_transform: Transform =
            get_geometry_transform(node_view.props_view(), node_view);
        node_view.set_geometry_transform(geometry_transform);
    } else {
        node_view.set_geometry_transform(IDENTITY_TRANSFORM);
    }

    // SAFETY: `local_transform_ptr()` is the node view's own live transform
    // field, read through the raw pointer the helper takes.
    let unscaled_node_to_parent: Matrix =
        unsafe { unscaled_transform_to_matrix(node_view.local_transform_ptr()) };

    node_view.set_inherit_scale(node_view.local_transform().scale);

    if let Some(parent) = node_view.parent() {
        // SAFETY: a non-null `parent` link points to a live, initialized
        // `ufbx_node` of the same scene (see above).
        let parent_view: &NodeView = unsafe { NodeView::from_ptr(parent.ptr()) };
        if node_view.inherit_mode() == InheritMode::Normal {
            // SAFETY: both operands are live matrix fields projected from their
            // own views, and `ufbx_matrix_mul` takes them by raw pointer and
            // only reads them; `node_to_parent` was composed above.
            let node_to_world: Matrix = unsafe {
                matrix_mul(
                    parent_view.node_to_world_ptr(),
                    node_view.node_to_parent_ptr(),
                )
            };
            node_view.set_node_to_world(node_to_world);
            // SAFETY: as above; `unscaled_node_to_parent` is a live local.
            let unscaled_node_to_world: Matrix = unsafe {
                matrix_mul(
                    parent_view.node_to_world_ptr(),
                    &raw const unscaled_node_to_parent,
                )
            };
            node_view.set_unscaled_node_to_world(unscaled_node_to_world);
        } else {
            let mut transform: Transform = node_view.local_transform();

            let mut parent_scale: Vec3 = ONE_VEC3;
            if let Some(inherit_scale_node) = node_view.inherit_scale_node() {
                // SAFETY: a non-null `inherit_scale_node` link points to a
                // live, initialized scene node whose `inherit_scale` was
                // computed earlier in this update pass.
                let inherit_scale_node_view: &NodeView =
                    unsafe { NodeView::from_ptr(inherit_scale_node.ptr()) };
                parent_scale = inherit_scale_node_view.inherit_scale();
            }

            transform.scale.x *= parent_scale.x;
            transform.scale.y *= parent_scale.y;
            transform.scale.z *= parent_scale.z;
            transform.translation.x *= parent_view.inherit_scale().x;
            transform.translation.y *= parent_view.inherit_scale().y;
            transform.translation.z *= parent_view.inherit_scale().z;

            // SAFETY: both raw pointers address the live, fully initialized local
            // `ufbx_transform`.
            let (node_to_unscaled_parent, unscaled_node_to_unscaled_parent): (Matrix, Matrix) = unsafe {
                (
                    transform_to_matrix(&raw const transform),
                    unscaled_transform_to_matrix(&raw const transform),
                )
            };

            node_view.set_inherit_scale(transform.scale);
            // SAFETY: the parent's matrix field is projected from its own view
            // and the other operand is a live local; `ufbx_matrix_mul` takes
            // both by raw pointer and only reads them.
            let node_to_world: Matrix = unsafe {
                matrix_mul(
                    parent_view.unscaled_node_to_world_ptr(),
                    &raw const node_to_unscaled_parent,
                )
            };
            node_view.set_node_to_world(node_to_world);
            // SAFETY: as above.
            let unscaled_node_to_world: Matrix = unsafe {
                matrix_mul(
                    parent_view.unscaled_node_to_world_ptr(),
                    &raw const unscaled_node_to_unscaled_parent,
                )
            };
            node_view.set_unscaled_node_to_world(unscaled_node_to_world);
        }
    } else {
        node_view.set_node_to_world(node_view.node_to_parent());
        node_view.set_unscaled_node_to_world(unscaled_node_to_parent);
    }

    // SAFETY: `geometry_transform_ptr()` is the node view's own geometry
    // transform field, set on both arms above, read through the raw pointer
    // the helper takes.
    if !unsafe { is_transform_identity(node_view.geometry_transform_ptr()) } {
        // SAFETY: as above, for `ufbx_transform_to_matrix`.
        let geometry_to_node: Matrix =
            unsafe { transform_to_matrix(node_view.geometry_transform_ptr()) };
        node_view.set_geometry_to_node(geometry_to_node);
        // SAFETY: both operands are the node view's own matrix fields, set
        // above, and `ufbx_matrix_mul` takes them by raw pointer and only
        // reads them.
        let geometry_to_world: Matrix = unsafe {
            matrix_mul(
                node_view.node_to_world_ptr(),
                node_view.geometry_to_node_ptr(),
            )
        };
        node_view.set_geometry_to_world(geometry_to_world);
        node_view.set_has_geometry_transform(true);
    } else {
        node_view.set_geometry_to_node(IDENTITY_MATRIX);
        node_view.set_geometry_to_world(node_view.node_to_world());
        node_view.set_has_geometry_transform(false);
    }

    node_view.set_visible(find_int(node_view.props_view(), &sp::Visibility, 1) != 0);
}

// ufbx.c:23044-23062 `ufbxi_update_light`
#[inline(never)]
pub(crate) fn update_light(light_view: &LightView) {
    let light: *mut Light = light_view.get();
    // NOTE: FBX seems to store intensities 100x of what's specified in at least
    // Maya and Blender, should there be a quirks mode to not do this for specific
    // exporters. Does the FBX SDK do this transparently as well?
    // SAFETY: `light` is the light view's own storage; `ufbxi_find_enum` clamps
    // each result to the `[0, LAST]` range passed, so the transmutes are of
    // in-range discriminants.
    unsafe {
        (*light).intensity =
            find_real(light_view.props_view(), &sp::Intensity, 100.0 as Real) / (100.0 as Real);

        (*light).color = find_vec3(light_view.props_view(), &sp::Color, 1.0, 1.0, 1.0);
        // C: `(ufbx_light_type)ufbxi_find_enum(...)` etc — `ufbxi_find_enum` clamps
        // each result to its enum's `[0, LAST]` range.
        (*light).type_ = core::mem::transmute::<u32, LightType>(find_enum(
            light_view.props_view(),
            &sp::LightType,
            0,
            LightType::Volume as i64,
        ) as u32);
        (*light).decay = core::mem::transmute::<u32, LightDecay>(find_enum(
            light_view.props_view(),
            &sp::DecayType,
            LightDecay::None as i64,
            LightDecay::Cubic as i64,
        ) as u32);
        (*light).area_shape = core::mem::transmute::<u32, LightAreaShape>(find_enum(
            light_view.props_view(),
            &sp::AreaLightShape,
            0,
            LightAreaShape::Sphere as i64,
        ) as u32);
        (*light).inner_angle = find_real(light_view.props_view(), &sp::HotSpot, 0.0);
        (*light).inner_angle = find_real(
            light_view.props_view(),
            &sp::InnerAngle,
            (*light).inner_angle,
        );
        (*light).outer_angle = find_real(light_view.props_view(), &sp::Cone_angle, 0.0);
        (*light).outer_angle = find_real(
            light_view.props_view(),
            &sp::ConeAngle,
            (*light).outer_angle,
        );
        (*light).outer_angle = find_real(
            light_view.props_view(),
            &sp::OuterAngle,
            (*light).outer_angle,
        );
        (*light).cast_light = find_int(light_view.props_view(), &sp::CastLight, 1) != 0;
        (*light).cast_shadows = find_int(light_view.props_view(), &sp::CastShadows, 0) != 0;
    }
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
pub(crate) fn update_camera<'a>(scene: &'a SceneView, camera_view: &'a CameraView) {
    // C: `(ufbx_projection_mode)ufbxi_find_enum(...)` etc — `ufbxi_find_enum`
    // clamps each result to its enum's `[0, LAST]` range (same device as
    // `ufbxi_update_light` above).
    // SAFETY: `ufbxi_find_enum` clamps its result to the `[0, ORTHOGRAPHIC]`
    // range passed, every value of which is a valid `ufbx_projection_mode`.
    camera_view.set_projection_mode(unsafe {
        core::mem::transmute::<u32, ProjectionMode>(find_enum(
            camera_view.props_view(),
            &sp::CameraProjectionType,
            0,
            ProjectionMode::Orthographic as i64,
        ) as u32)
    });
    // SAFETY: as above; the clamp range spans the whole `ufbx_aspect_mode` enum.
    camera_view.set_aspect_mode(unsafe {
        core::mem::transmute::<u32, AspectMode>(find_enum(
            camera_view.props_view(),
            &sp::AspectRatioMode,
            0,
            AspectMode::FixedHeight as i64,
        ) as u32)
    });
    // SAFETY: as above; the clamp range spans the whole `ufbx_aperture_mode`
    // enum.
    camera_view.set_aperture_mode(unsafe {
        core::mem::transmute::<u32, ApertureMode>(find_enum(
            camera_view.props_view(),
            &sp::ApertureMode,
            ApertureMode::Vertical as i64,
            ApertureMode::FocalLength as i64,
        ) as u32)
    });
    // SAFETY: as above; the clamp range spans the whole `ufbx_aperture_format`
    // enum.
    camera_view.set_aperture_format(unsafe {
        core::mem::transmute::<u32, ApertureFormat>(find_enum(
            camera_view.props_view(),
            &sp::ApertureFormat,
            ApertureFormat::Custom as i64,
            ApertureFormat::Imax as i64,
        ) as u32)
    });
    // SAFETY: as above; the clamp range spans the whole `ufbx_gate_fit` enum.
    camera_view.set_gate_fit(unsafe {
        core::mem::transmute::<u32, GateFit>(find_enum(
            camera_view.props_view(),
            &sp::GateFit,
            0,
            GateFit::Stretch as i64,
        ) as u32)
    });

    camera_view.set_near_plane(find_real(camera_view.props_view(), &sp::NearPlane, 0.0));
    camera_view.set_far_plane(find_real(camera_view.props_view(), &sp::FarPlane, 0.0));

    // Search both W/H and Width/Height but prefer the latter
    let mut aspect_x: Real = find_real(camera_view.props_view(), &sp::AspectW, 0.0);
    let mut aspect_y: Real = find_real(camera_view.props_view(), &sp::AspectH, 0.0);
    aspect_x = find_real(camera_view.props_view(), &sp::AspectWidth, aspect_x);
    aspect_y = find_real(camera_view.props_view(), &sp::AspectHeight, aspect_y);

    let fov: Real = find_real(camera_view.props_view(), &sp::FieldOfView, 0.0);
    let fov_x: Real = find_real(camera_view.props_view(), &sp::FieldOfViewX, 0.0);
    let fov_y: Real = find_real(camera_view.props_view(), &sp::FieldOfViewY, 0.0);

    let focal_length: Real = find_real(camera_view.props_view(), &sp::FocalLength, 0.0);
    let mut ortho_extent: Real = scene.metadata_view().ortho_size_unit()
        * find_real(camera_view.props_view(), &sp::OrthoZoom, 1.0);

    // `aperture_format` was assigned above from a clamped `ufbxi_find_enum`, so
    // it indexes `APERTURE_FORMATS`.
    let format: ApertureFormatInfo = APERTURE_FORMATS[camera_view.aperture_format() as usize];
    let mut film_size: Vec2 = Vec2 {
        x: format.film_size_x as Real * (0.001 as Real),
        y: format.film_size_y as Real * (0.001 as Real),
    };
    let mut squeeze_ratio: Real =
        if camera_view.aperture_format() == ApertureFormat::E35MmAnamorphic {
            2.0
        } else {
            1.0
        };

    film_size.x = find_real(camera_view.props_view(), &sp::FilmWidth, film_size.x);
    film_size.y = find_real(camera_view.props_view(), &sp::FilmHeight, film_size.y);
    squeeze_ratio = find_real(
        camera_view.props_view(),
        &sp::FilmSqueezeRatio,
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
    ortho_extent *= scene.metadata_view().geometry_scale();
    camera_view.set_near_plane(camera_view.near_plane() * scene.metadata_view().geometry_scale());
    camera_view.set_far_plane(camera_view.far_plane() * scene.metadata_view().geometry_scale());

    camera_view.set_focal_length_mm(focal_length);
    camera_view.set_film_size_inch(film_size);
    camera_view.set_squeeze_ratio(squeeze_ratio);
    camera_view.set_orthographic_extent(ortho_extent);

    // C assigns `resolution.x` and `resolution.y` as separate statements; the
    // view's leaf field is the whole `ufbx_vec2`, so each arm computes the two
    // components in C's order and stores the pair in one write. The same holds
    // for the `ufbx_vec2` leaves of the two matches below.
    match camera_view.aspect_mode() {
        AspectMode::WindowSize | AspectMode::FixedRatio => {
            camera_view.set_resolution_is_pixels(false);
            camera_view.set_resolution(Vec2 {
                x: aspect_x,
                y: aspect_y,
            });
        }
        AspectMode::FixedResolution => {
            camera_view.set_resolution_is_pixels(true);
            camera_view.set_resolution(Vec2 {
                x: aspect_x,
                y: aspect_y,
            });
        }
        AspectMode::FixedWidth => {
            camera_view.set_resolution_is_pixels(true);
            camera_view.set_resolution(Vec2 {
                x: aspect_x,
                y: aspect_x * aspect_y,
            });
        }
        AspectMode::FixedHeight => {
            camera_view.set_resolution_is_pixels(true);
            camera_view.set_resolution(Vec2 {
                x: aspect_y * aspect_x,
                y: aspect_y,
            });
        }
        // C `default:` (ufbx.c:23167-23168) — unreachable in Rust because the
        // match above is exhaustive over the enum, but kept for diff parity.
        #[allow(unreachable_patterns)]
        _ => {
            ufbxi_unreachable!("Unexpected aspect mode");
        }
    }

    // `resolution` was assigned on every arm of the match above.
    let aspect_ratio: Real = camera_view.resolution().x / camera_view.resolution().y;
    let film_ratio: Real = film_size.x / film_size.y;

    camera_view.set_aspect_ratio(aspect_ratio);

    let mut effective_fit: GateFit = camera_view.gate_fit();
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
            camera_view.set_aperture_size_inch(camera_view.film_size_inch());
            camera_view.set_orthographic_size(Vec2 {
                x: ortho_extent,
                y: ortho_extent,
            });
        }
        GateFit::Vertical => {
            camera_view.set_aperture_size_inch(Vec2 {
                x: camera_view.film_size_inch().y * aspect_ratio,
                y: camera_view.film_size_inch().y,
            });
            camera_view.set_orthographic_size(Vec2 {
                x: ortho_extent * aspect_ratio,
                y: ortho_extent,
            });
        }
        GateFit::Horizontal => {
            camera_view.set_aperture_size_inch(Vec2 {
                x: camera_view.film_size_inch().x,
                y: camera_view.film_size_inch().x / aspect_ratio,
            });
            camera_view.set_orthographic_size(Vec2 {
                x: ortho_extent,
                y: ortho_extent / aspect_ratio,
            });
        }
        GateFit::Fill | GateFit::Overscan => {
            camera_view.set_aperture_size_inch(camera_view.film_size_inch());
            camera_view.set_orthographic_size(Vec2 {
                x: ortho_extent,
                y: ortho_extent,
            });
            // C: `ufbxi_unreachable(...)` mid-arm — it is NOT a return, the
            // arm's assignments above it already ran (PORTING.md "Asserts").
            ufbxi_unreachable!("Unreachable, set to vertical/horizontal above");
        }
        GateFit::Stretch => {
            camera_view.set_aperture_size_inch(camera_view.film_size_inch());
            camera_view.set_orthographic_size(Vec2 {
                x: ortho_extent,
                y: ortho_extent,
            });
            // TODO: Not sure what to do here...
        }
        // C `default:` (ufbx.c:23214-23215).
        #[allow(unreachable_patterns)]
        _ => {
            ufbxi_unreachable!("Unexpected gate fit");
        }
    }

    match camera_view.aperture_mode() {
        ApertureMode::HorizontalAndVertical => {
            camera_view.set_field_of_view_deg(Vec2 { x: fov_x, y: fov_y });
            // C: `(ufbx_real)ufbx_tan((double)(...))` — the inner product is
            // real arithmetic, promoted to double only at the `tan` call.
            camera_view.set_field_of_view_tan(Vec2 {
                x: math::tan((fov_x * (sp::DEG_TO_RAD * 0.5)) as f64) as Real,
                y: math::tan((fov_y * (sp::DEG_TO_RAD * 0.5)) as f64) as Real,
            });
        }
        ApertureMode::Horizontal => {
            // C assigns one `ufbx_vec2` component per statement and reads back
            // the component it wrote on the line before, so each assignment is
            // a read-modify-write of the pair through the leaf accessor.
            let mut fov_deg: Vec2 = camera_view.field_of_view_deg();
            fov_deg.x = fov;
            camera_view.set_field_of_view_deg(fov_deg);
            let mut fov_tan: Vec2 = camera_view.field_of_view_tan();
            fov_tan.x = math::tan((fov * (sp::DEG_TO_RAD * 0.5)) as f64) as Real;
            camera_view.set_field_of_view_tan(fov_tan);
            fov_tan.y = camera_view.field_of_view_tan().x / aspect_ratio;
            camera_view.set_field_of_view_tan(fov_tan);
            fov_deg.y = math::atan(as_f64!(camera_view.field_of_view_tan().y)) as Real
                * sp::RAD_TO_DEG
                * 2.0;
            camera_view.set_field_of_view_deg(fov_deg);
        }
        ApertureMode::Vertical => {
            // As the horizontal arm above: one component per C statement, each
            // read taking the component assigned on the line before it.
            let mut fov_deg: Vec2 = camera_view.field_of_view_deg();
            fov_deg.y = fov;
            camera_view.set_field_of_view_deg(fov_deg);
            let mut fov_tan: Vec2 = camera_view.field_of_view_tan();
            fov_tan.y = math::tan((fov * (sp::DEG_TO_RAD * 0.5)) as f64) as Real;
            camera_view.set_field_of_view_tan(fov_tan);
            fov_tan.x = camera_view.field_of_view_tan().y * aspect_ratio;
            camera_view.set_field_of_view_tan(fov_tan);
            fov_deg.x = math::atan(as_f64!(camera_view.field_of_view_tan().x)) as Real
                * sp::RAD_TO_DEG
                * 2.0;
            camera_view.set_field_of_view_deg(fov_deg);
        }
        ApertureMode::FocalLength => {
            // `aperture_size_inch` and `focal_length_mm` were assigned above.
            camera_view.set_field_of_view_tan(Vec2 {
                x: camera_view.aperture_size_inch().x
                    / (camera_view.focal_length_mm() * sp::MM_TO_INCH)
                    * 0.5,
                y: camera_view.aperture_size_inch().y
                    / (camera_view.focal_length_mm() * sp::MM_TO_INCH)
                    * 0.5,
            });
            // Reading the `field_of_view_tan` components assigned above.
            camera_view.set_field_of_view_deg(Vec2 {
                x: math::atan(as_f64!(camera_view.field_of_view_tan().x)) as Real
                    * sp::RAD_TO_DEG
                    * 2.0,
                y: math::atan(as_f64!(camera_view.field_of_view_tan().y)) as Real
                    * sp::RAD_TO_DEG
                    * 2.0,
            });
        }
        // C `default:` (ufbx.c:23243-23244).
        #[allow(unreachable_patterns)]
        _ => {
            ufbxi_unreachable!("Unexpected aperture mode");
        }
    }

    if camera_view.projection_mode() == ProjectionMode::Perspective {
        // `field_of_view_tan` was assigned on every arm of the match above.
        camera_view.set_projection_plane(camera_view.field_of_view_tan());
    } else {
        // `orthographic_size` was assigned on every arm of the gate-fit match
        // above.
        camera_view.set_projection_plane(camera_view.orthographic_size());
    }
}

// ufbx.c:23254-23264 `ufbxi_update_bone`
#[inline(never)]
pub(crate) fn update_bone<'a>(scene: &'a SceneView, bone_view: &'a BoneView) {
    let scene: *mut Scene = scene.get();
    let bone: *mut Bone = bone_view.get();
    // SAFETY: `scene` and `bone` are the storage of the views passed in.
    unsafe {
        let unit: Real = (*scene).metadata.bone_prop_size_unit;

        (*bone).radius = find_real(bone_view.props_view(), &sp::Size, unit) / unit;
        if (*scene).metadata.bone_prop_limb_length_relative {
            (*bone).relative_length = find_real(bone_view.props_view(), &sp::LimbLength, 1.0);
        } else {
            (*bone).relative_length = 1.0;
        }
    }
}

// ufbx.c:23266-23269 `ufbxi_update_line_curve`
#[inline(never)]
pub(crate) fn update_line_curve(line_view: &LineCurveView) {
    let line: *mut LineCurve = line_view.get();
    // SAFETY: `line` is the line-curve view's own storage.
    unsafe {
        (*line).color = find_vec3(line_view.props_view(), &sp::Color, 1.0, 1.0, 1.0);
    }
}

// ufbx.c:23271-23287 `ufbxi_update_pose`
#[inline(never)]
pub(crate) fn update_pose(pose_view: &PoseView) {
    let pose: *mut Pose = pose_view.get();
    // C: `ufbxi_for_list(ufbx_bone_pose, bone, pose->bone_poses)`
    // SAFETY: `pose` is the pose view's own live, initialized `ufbx_pose`
    // storage, so its own bone-pose list is readable.
    // `data`/`count` describe one arena run.
    let (mut bone, bone_count) = unsafe {
        (
            (*pose).bone_poses.data as *mut BonePose,
            (*pose).bone_poses.count,
        )
    };
    let bone_end: *mut BonePose = add_ptr(bone, bone_count);
    while bone != bone_end {
        // SAFETY: `bone != bone_end`, so it addresses a live, initialized entry
        // of the pose's bone-pose run, whose `bone_node` link is non-optional.
        let node: *mut Node = unsafe { ref_ptr(&raw const (*bone).bone_node) };

        let mut parent_to_world: *const Matrix = &raw const IDENTITY_MATRIX;
        // SAFETY: `node` is a resolved element link, so it points to a live,
        // initialized `ufbx_node` of the same scene, and `pose` is live.
        let bone_pose: *mut BonePose =
            unsafe { get_bone_pose(pose, opt_ptr(&raw const (*node).parent)) };
        if !bone_pose.is_null() {
            // SAFETY: `bone_pose` is non-null here, so it addresses a live entry
            // of the pose's own bone-pose run, which outlives this loop.
            parent_to_world = unsafe { &raw const (*bone_pose).bone_to_world };
        // SAFETY: `node` is live (see above).
        } else if !unsafe { opt_ptr(&raw const (*node).parent) }.is_null() {
            // SAFETY: the branch condition established that the parent link is
            // non-null, so it points to a live, initialized `ufbx_node` of the
            // same scene, which outlives this loop.
            parent_to_world =
                unsafe { &raw const (*opt_ptr(&raw const (*node).parent)).node_to_world };
        }

        // SAFETY: `parent_to_world` is either the static identity matrix or a
        // live scene matrix borrowed above.
        let world_to_parent: Matrix = unsafe { matrix_invert(parent_to_world) };
        // SAFETY: `bone` addresses a live, writable entry of the pose's own run
        // (see above).
        unsafe {
            (*bone).bone_to_parent =
                matrix_mul(&raw const world_to_parent, &raw const (*bone).bone_to_world)
        };

        // SAFETY: `bone != bone_end`, so the advance lands at or before the run's
        // one-past-the-end pointer.
        bone = unsafe { bone.add(1) };
    }
}

// ufbx.c:23289-23297 `ufbxi_update_skin_cluster`
#[inline(never)]
pub(crate) fn update_skin_cluster(cluster_view: &SkinClusterView) {
    let cluster: *mut SkinCluster = cluster_view.get();
    // C: `if (cluster->bone_node)` — pointer truthiness.
    // SAFETY: `cluster` is the cluster view's own live, initialized
    // `ufbx_skin_cluster` storage, so `&raw const (*cluster).bone_node` addresses its own
    // nullable bone link.
    let bone_node: *mut Node = unsafe { opt_ptr(&raw const (*cluster).bone_node) };
    if !bone_node.is_null() {
        // SAFETY: `bone_node` is non-null here, so it points to a live,
        // initialized `ufbx_node` of the same scene; `cluster` is live and
        // writable (see above).
        unsafe {
            (*cluster).geometry_to_world = matrix_mul(
                &raw const (*bone_node).node_to_world,
                &raw const (*cluster).geometry_to_bone,
            )
        };
    } else {
        // SAFETY: `cluster` is live and writable (see above); both operands are
        // its own matrices.
        unsafe {
            (*cluster).geometry_to_world = matrix_mul(
                &raw const (*cluster).bind_to_world,
                &raw const (*cluster).geometry_to_bone,
            )
        };
    }
    // SAFETY: `cluster` is live and writable (see above); `geometry_to_world` was
    // assigned on both arms above.
    unsafe {
        (*cluster).geometry_to_world_transform =
            matrix_to_transform(&raw const (*cluster).geometry_to_world)
    };
}

// ufbx.c:23299-23342 `ufbxi_update_blend_channel`
#[inline(never)]
pub(crate) fn update_blend_channel(channel_view: &BlendChannelView) {
    let channel: *mut BlendChannel = channel_view.get();
    let weight: Real =
        find_real(channel_view.props_view(), &sp::DeformPercent, 0.0) * (0.01 as Real);
    // SAFETY: `channel` is the blend-channel view's own storage.
    unsafe { (*channel).weight = weight };

    // SAFETY: `channel` as above.
    let num_keys: isize = unsafe { (*channel).keyframes.count } as isize;
    if num_keys > 0 {
        // SAFETY: `keys` is the channel's own `num_keys`-element keyframe run; every
        // `offset` below is bounds-checked against `num_keys` (`last_negative` is an
        // index into it, or -1), and `prev`/`next` are either such an element or the
        // local `zero_key` sentinel.
        unsafe {
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
}

// ufbx.c:23344-23349 `ufbxi_update_material`
#[inline(never)]
pub(crate) fn update_material(scene_view: &SceneView, material_view: &MaterialView) {
    if material_view.props_view().num_animated() > 0 {
        fetch_maps(scene_view, material_view);
    }
}

// ufbx.c:23351-23369 `ufbxi_update_texture`
#[inline(never)]
pub(crate) fn update_texture(texture_view: &TextureView) {
    let texture: *mut Texture = texture_view.get();
    // SAFETY: `texture` is the texture view's own storage and the transform is
    // derived from that same view's props; the matrix helpers are pure value
    // math over the texture's own fields.
    unsafe {
        (*texture).uv_transform = get_texture_transform(texture_view.props_view());
        if !is_transform_identity(&raw const (*texture).uv_transform) {
            (*texture).has_uv_transform = true;
            (*texture).texture_to_uv = transform_to_matrix(&raw const (*texture).uv_transform);
            (*texture).uv_to_texture = matrix_invert(&raw const (*texture).texture_to_uv);
        } else {
            (*texture).has_uv_transform = false;
            (*texture).texture_to_uv = IDENTITY_MATRIX;
            (*texture).uv_to_texture = IDENTITY_MATRIX;
        }
    }
    // SAFETY: `texture` as above; `ufbxi_find_enum` clamps each result to the
    // `[0, LAST]` range passed, so both transmutes are of in-range discriminants.
    unsafe {
        // C: `(ufbx_wrap_mode)ufbxi_find_enum(...)` — clamped to `[0, LAST]`.
        (*texture).wrap_u = core::mem::transmute::<u32, WrapMode>(find_enum(
            texture_view.props_view(),
            &sp::WrapModeU,
            0,
            WrapMode::Clamp as i64,
        ) as u32);
        (*texture).wrap_v = core::mem::transmute::<u32, WrapMode>(find_enum(
            texture_view.props_view(),
            &sp::WrapModeV,
            0,
            WrapMode::Clamp as i64,
        ) as u32);
    }

    // SAFETY: `texture` as above; `opt_ptr` result is null-checked before use,
    // and a non-null shader link points to a live `ufbx_shader_texture` of the
    // same scene, which its view reinterprets in place.
    unsafe {
        // C: `if (texture->shader)` — pointer truthiness.
        let shader: *mut ShaderTexture = opt_ptr(&raw const (*texture).shader);
        if !shader.is_null() {
            update_shader_texture(texture_view, ShaderTextureView::from_ptr(shader));
        }
    }
}

// ufbx.c:23371-23388 `ufbxi_update_anim_stack`
#[inline(never)]
pub(crate) fn update_anim_stack<'a>(scene: &'a SceneView, stack_view: &'a AnimStackView) {
    let scene: *mut Scene = scene.get();
    let stack: *mut AnimStack = stack_view.get();
    // C: `ufbx_prop *begin, *end;` — both are assigned before any read.
    let mut begin: *mut Prop;
    let mut end: *mut Prop;
    begin =
        find_prop(stack_view.props_view(), &sp::LocalStart).map_or(ptr::null_mut(), PropView::get);
    end = find_prop(stack_view.props_view(), &sp::LocalStop).map_or(ptr::null_mut(), PropView::get);
    if begin.is_null() || end.is_null() {
        begin = find_prop(stack_view.props_view(), &sp::ReferenceStart)
            .map_or(ptr::null_mut(), PropView::get);
        end = find_prop(stack_view.props_view(), &sp::ReferenceStop)
            .map_or(ptr::null_mut(), PropView::get);
    }

    // SAFETY: `begin`/`end` are null-checked before the derefs; `scene`, `stack`
    // and the stack's always-resolved `anim` reference are storage of the views
    // passed in.
    unsafe {
        if !begin.is_null() && !end.is_null() {
            (*stack).time_begin = (*begin).value_int as f64 / (*scene).metadata.ktime_second as f64;
            (*stack).time_end = (*end).value_int as f64 / (*scene).metadata.ktime_second as f64;
        }

        let anim: *mut Anim = ref_ptr(&raw const (*stack).anim);
        (*anim).time_begin = (*stack).time_begin;
        (*anim).time_end = (*stack).time_end;
    }
}

// ufbx.c:23390-23395 `ufbxi_update_display_layer`
#[inline(never)]
pub(crate) fn update_display_layer(layer_view: &DisplayLayerView) {
    let layer: *mut DisplayLayer = layer_view.get();
    // SAFETY: `layer` is the display-layer view's own storage.
    unsafe {
        (*layer).visible = find_int(layer_view.props_view(), &sp::Show, 1) != 0;
        (*layer).frozen = find_int(layer_view.props_view(), &sp::Freeze, 1) != 0;
        // C-parity: `0.8f` is a `float` literal widened to `ufbx_real` (double) —
        // NOT the decimal value 0.8 (PORTING.md "Floats").
        (*layer).ui_color = find_vec3(
            layer_view.props_view(),
            &sp::Color,
            0.8f32 as Real,
            0.8f32 as Real,
            0.8f32 as Real,
        );
    }
}

// Rust-port infrastructure (not a ufbx.c section): the three `bool[3]` axis
// flags `ufbxi_find_bool3` fills, projected in place so the `bool *dst`
// out-parameter travels as a view over constraint (scene-arena) memory.
impl View<Constraint> {
    #[inline(always)]
    pub(crate) fn constrain_translation_view(&self) -> &View<[bool; 3]> {
        view_project!(self, constrain_translation)
    }

    #[inline(always)]
    pub(crate) fn constrain_rotation_view(&self) -> &View<[bool; 3]> {
        view_project!(self, constrain_rotation)
    }

    #[inline(always)]
    pub(crate) fn constrain_scale_view(&self) -> &View<[bool; 3]> {
        view_project!(self, constrain_scale)
    }
}

// ufbx.c:23397-23414 `ufbxi_find_bool3`
#[inline(never)]
pub(crate) fn find_bool3(
    dst: &View<[bool; 3]>,
    props: &PropsView,
    name: &[u8],
    default_value: bool,
) {
    // C: `size_t name_len = strlen(name);` — the name-slice convention carries
    // the length the C recomputes from the NUL terminator.
    let name_len: usize = name.len();
    // C: `char local[64];` — an uninitialized local; only `local[0..name_len]`
    // is ever read back (`local_len == name_len + 1` bytes are written first).
    let mut local_storage = MaybeUninit::<[u8; 64]>::uninit();
    let local: *mut u8 = local_storage.as_mut_ptr() as *mut u8;
    // C: `ufbx_assert(name_len < sizeof(local) - 2);`
    ufbx_assert!(name_len < size_of::<[u8; 64]>() - 2);
    // SAFETY: `name` addresses `name_len` readable bytes (its own slice run),
    // the assert above established `name_len < 62`, so the copy fits in the
    // 64-byte local, and the two regions are distinct objects.
    unsafe { ptr::copy_nonoverlapping(name.as_ptr(), local, name_len) };

    let local_len: usize = name_len + 1;
    // SAFETY: the assert above established `name_len < 62`, so
    // `local_len = name_len + 1 < 63` indexes inside the 64-byte local.
    unsafe { *local.add(local_len) = b'\0' };

    let def: i64 = if default_value { 1 } else { 0 };
    // SAFETY: `name_len < 62` (asserted above) indexes inside the 64-byte local.
    unsafe { *local.add(name_len) = b'X' };
    // SAFETY: `dst.get()` addresses the viewed `[bool; 3]` (view mint invariant),
    // so element 0 is a live writable place; `local` holds `local_len`
    // initialized bytes followed by a NUL, which is what `ufbx_find_int_len`
    // reads.
    unsafe {
        (*dst.get())[0] = api_find_int_len(props, slice_from_ptr(local, local_len), def) != 0;
    };
    // SAFETY: as above, for the `Y` suffix.
    unsafe { *local.add(name_len) = b'Y' };
    // SAFETY: as above, for element 1 of `dst`.
    unsafe {
        (*dst.get())[1] = api_find_int_len(props, slice_from_ptr(local, local_len), def) != 0;
    };
    // SAFETY: as above, for the `Z` suffix.
    unsafe { *local.add(name_len) = b'Z' };
    // SAFETY: as above, for element 2 of `dst`.
    unsafe {
        (*dst.get())[2] = api_find_int_len(props, slice_from_ptr(local, local_len), def) != 0;
    };
}

// ufbx.c:23416-23488 `ufbxi_update_constraint`
#[inline(never)]
pub(crate) fn update_constraint(constraint_view: &ConstraintView) {
    let constraint: *mut Constraint = constraint_view.get();
    // C: `ufbx_props *props = &constraint->props;` — kept live across writes
    // through `constraint`, so this must be a `&raw mut` and never a `&mut`
    // (which would retag and be invalidated by those writes). Correlated to the
    // element view (<= uc) via `props_view`.
    let props: &PropsView = constraint_view.props_view();
    // SAFETY: `constraint` is the constraint view's own storage.
    let constraint_type: ConstraintType = unsafe { (*constraint).type_ };

    // SAFETY: `constraint` and `props` are the constraint view's own storage; the
    // lookup name is a NUL-terminated static.
    unsafe {
        (*constraint).transform_offset = get_constraint_transform(props);

        // C: `ufbxi_find_real` — the internal 4-byte-key lookup, NOT `ufbx_find_real`.
        (*constraint).weight = find_real(props, &sp::Weight, 100.0 as Real) / (100.0 as Real);
    }

    // SAFETY: walks the constraint's own `targets` run (`count` entries); `node`
    // is an always-resolved reference from the same arena, `parts` is a local
    // two-element array fully written before each lookup, and every `prop` is
    // null-checked before its deref (`value_vec4`'s leading three reals are the
    // `ufbx_prop` value union's `ufbx_vec3` view).
    unsafe {
        // C: `ufbxi_for_list(ufbx_constraint_target, target, constraint->targets)`
        let mut target: *mut ConstraintTarget = (*constraint).targets.data as *mut ConstraintTarget;
        let target_end: *mut ConstraintTarget = add_ptr(target, (*constraint).targets.count);
        while target != target_end {
            let node: *mut Node = ref_ptr(&raw const (*target).node);

            let mut weight_scale: Real = 100.0 as Real;
            if constraint_type == ConstraintType::SingleChainIk {
                // IK weights seem to be not scaled 100x?
                weight_scale = 1.0 as Real;
            }

            let mut prop: Option<&PropView>; // ufbxi_uninit
                                             // C: `ufbx_string parts[2];` (ufbxi_uninit) — both entries are
                                             // written before every lookup.
            let mut parts: [String; 2] = [(*node).element.name, sp::str_c(b".Weight\0".as_ptr())];
            prop = find_prop_concat(props, &parts);
            // C: `prop->value_real` — the `ufbx_prop` value union's first real.
            (*target).weight = prop.map_or(weight_scale, |p| p.value_vec4().x) / weight_scale;

            if constraint_type == ConstraintType::Parent {
                parts[1] = sp::str_c(b".Offset T\0".as_ptr());
                prop = find_prop_concat(props, &parts);
                let t: Vec3 = prop.map_or(ZERO_VEC3, PropView::value_vec3);
                parts[1] = sp::str_c(b".Offset R\0".as_ptr());
                prop = find_prop_concat(props, &parts);
                let r: Vec3 = prop.map_or(ZERO_VEC3, PropView::value_vec3);
                parts[1] = sp::str_c(b".Offset S\0".as_ptr());
                prop = find_prop_concat(props, &parts);
                let s: Vec3 = prop.map_or(ONE_VEC3, PropView::value_vec3);

                (*target).transform.translation = t;
                (*target).transform.rotation = euler_to_quat(r, RotationOrder::Xyz);
                (*target).transform.scale = s;
            }

            target = target.add(1);
        }
    }

    // SAFETY: `constraint` and `props` are the constraint view's own storage,
    // and the transmute is guarded by the explicit `[0, LAST)` range check
    // above it.
    unsafe {
        (*constraint).active = api_find_int_len(props, b"Active", 1) != 0;
        if constraint_type == ConstraintType::Aim {
            find_bool3(
                constraint_view.constrain_rotation_view(),
                props,
                b"Affect",
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

            let up_type: i64 = api_find_int_len(props, b"WorldUpType", 0);
            if up_type >= 0 && up_type < ConstraintAimUpType::None as i64 {
                // C: `(ufbx_constraint_aim_up_type)up_type` — the range check above
                // admits only valid enum values.
                (*constraint).aim_up_type =
                    core::mem::transmute::<u32, ConstraintAimUpType>(up_type as u32);
            }
            (*constraint).aim_vector = api_find_vec3_len(props, b"AimVector", default_aim);
            (*constraint).aim_up_vector = api_find_vec3_len(props, b"UpVector", default_up);
        } else if constraint_type == ConstraintType::Parent {
            find_bool3(
                constraint_view.constrain_translation_view(),
                props,
                b"AffectTranslation",
                true,
            );
            find_bool3(
                constraint_view.constrain_rotation_view(),
                props,
                b"AffectRotation",
                true,
            );
            find_bool3(
                constraint_view.constrain_scale_view(),
                props,
                b"AffectScale",
                false,
            );
        } else if constraint_type == ConstraintType::Position {
            find_bool3(
                constraint_view.constrain_translation_view(),
                props,
                b"Affect",
                true,
            );
        } else if constraint_type == ConstraintType::Rotation {
            find_bool3(
                constraint_view.constrain_rotation_view(),
                props,
                b"Affect",
                true,
            );
        } else if constraint_type == ConstraintType::Scale {
            find_bool3(
                constraint_view.constrain_scale_view(),
                props,
                b"Affect",
                true,
            );
        } else if constraint_type == ConstraintType::SingleChainIk {
            (*constraint).constrain_rotation[0] = true;
            (*constraint).constrain_rotation[1] = true;
            (*constraint).constrain_rotation[2] = true;
            (*constraint).ik_pole_vector = api_find_vec3_len(props, b"PoleVectorType", ZERO_VEC3);
        }
    }
}

// ufbx.c:23490-23495 `ufbxi_update_anim`
#[inline(never)]
pub(crate) fn update_anim(scene_view: &SceneView) {
    let scene: *mut Scene = scene_view.get();
    // SAFETY: `scene` is the scene view's own live, initialized `ufbx_scene`
    // storage, so its own anim-stack pointer list is readable.
    if unsafe { (*scene).anim_stacks.count } > 0 {
        // C: `scene->anim = scene->anim_stacks.data[0]->anim;`
        // SAFETY: `count > 0`, so element `0` of the scene's anim-stack pointer
        // run is live and initialized.
        let stack: *mut AnimStack =
            unsafe { *((*scene).anim_stacks.data as *const *mut AnimStack) };
        // SAFETY: `stack` is a resolved element pointer from the scene's own
        // list, so it points to a live, initialized `ufbx_anim_stack`; `scene` is
        // live and writable (see above).
        unsafe { (*scene).anim = (*stack).anim };
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
    // SAFETY: `m` points to a live, initialized, writable `ufbx_matrix` (fn
    // contract) laid out as four consecutive `ufbx_vec3` columns, so column `0`
    // is in bounds.
    let c0: *mut Real = unsafe { cols.add(0) } as *mut Real;
    // SAFETY: a column is three consecutive `ufbx_real`s, and the early return
    // above established `axis != None`, so `ax = axis - 1` is in `0..3`.
    unsafe { *c0.add(ax as usize) = -*c0.add(ax as usize) };
    // SAFETY: as above, for column `1`.
    let c1: *mut Real = unsafe { cols.add(1) } as *mut Real;
    // SAFETY: as above; `ax` is in `0..3`.
    unsafe { *c1.add(ax as usize) = -*c1.add(ax as usize) };
    // SAFETY: as above, for column `2`.
    let c2: *mut Real = unsafe { cols.add(2) } as *mut Real;
    // SAFETY: as above; `ax` is in `0..3`.
    unsafe { *c2.add(ax as usize) = -*c2.add(ax as usize) };
    // SAFETY: as above, for column `3`.
    let c3: *mut Real = unsafe { cols.add(3) } as *mut Real;
    // SAFETY: as above; `ax` is in `0..3`.
    unsafe { *c3.add(ax as usize) = -*c3.add(ax as usize) };
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
    // SAFETY: `m` points to a live, initialized, writable `ufbx_matrix` (fn
    // contract) laid out as four consecutive `ufbx_vec3` columns, and the early
    // return above established `axis != None`, so `ax = axis - 1` is in `0..3`.
    let col: *mut Vec3 = unsafe { cols.add(ax as usize) };
    // SAFETY: `col` addresses one of the matrix's own columns (see above).
    unsafe {
        (*col).x = -(*col).x;
        (*col).y = -(*col).y;
        (*col).z = -(*col).z;
    }
}

// ufbx.c:23516-23521 `ufbxi_mirror_matrix`
#[inline(never)]
pub(crate) unsafe fn mirror_matrix(m: *mut Matrix, axis: MirrorAxis) {
    // C: `if (axis == 0) return;`
    if axis as u32 == 0 {
        return;
    }
    // SAFETY: `m` points to a live, initialized, writable `ufbx_matrix` (fn
    // contract) — the same contract `ufbxi_mirror_matrix_src` takes.
    unsafe { mirror_matrix_src(m, axis) };
    // SAFETY: as above, for `ufbxi_mirror_matrix_dst`.
    unsafe { mirror_matrix_dst(m, axis) };
}

// ufbx.c:23523-23619 `ufbxi_update_initial_clusters`
#[inline(never)]
pub(crate) fn update_initial_clusters(scene_view: &SceneView) {
    // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, scene->skin_clusters)`
    let skin_clusters: &RefListView<SkinCluster> = scene_view.skin_clusters_view();
    for i in 0..skin_clusters.count() {
        // C: `ufbx_skin_cluster *cluster = *p_cluster;`
        let cluster: &View<SkinCluster> = skin_clusters.at(i);
        cluster.set_geometry_to_bone(cluster.mesh_node_to_bone());
    }

    let metadata: &SceneMetadataView = scene_view.metadata_view();
    let mirror_axis: MirrorAxis = metadata.mirror_axis();
    let geometry_scale: Real = metadata.geometry_scale();

    // Space conversion for bind matrices
    {
        // C: `ufbx_matrix world_to_units;` — written by both arms of the
        // `if` below (upstream carries no `// ufbxi_uninit` marker).
        let world_to_units: Matrix;
        let mut translation_scale: Real = 1.0 as Real;

        if metadata.space_conversion() == SpaceConversion::TransformRoot
            && metadata.mirror_axis() == MirrorAxis::None
        {
            // C: `world_to_units = scene->root_node->node_to_parent;`
            // SAFETY: the scene's own non-optional root link names a live,
            // initialized `ufbx_node` in the scene's arena, so its address
            // carries write-capable provenance.
            let root_node: &View<Node> =
                unsafe { View::<Node>::from_ptr(scene_view.root_node().ptr()) };
            world_to_units = root_node.node_to_parent();
        } else {
            // C: `ufbx_transform root_transform;` — every member is written
            // below before the first read.
            // SAFETY: `ufbx_transform` is a plain struct of `ufbx_real` fields,
            // for which the all-zero bit pattern is a valid value.
            let mut root_transform: Transform = unsafe { core::mem::zeroed() };
            root_transform.translation = ZERO_VEC3;
            root_transform.rotation = metadata.root_rotation();
            root_transform.scale.x = metadata.root_scale();
            root_transform.scale.y = metadata.root_scale();
            root_transform.scale.z = metadata.root_scale();
            // SAFETY: `&root_transform` borrows a live local `ufbx_transform`,
            // zero-initialized above and with every member assigned before the
            // read.
            world_to_units = unsafe { transform_to_matrix(&raw const root_transform) };
            translation_scale = metadata.geometry_scale();
        }

        // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, scene->skin_clusters)`
        for i in 0..skin_clusters.count() {
            // C: `ufbx_skin_cluster *cluster = *p_cluster;`
            let cluster: &View<SkinCluster> = skin_clusters.at(i);
            // SAFETY: `&world_to_units` borrows a live, fully written local
            // matrix and `bind_to_world_ptr()` projects the cluster's own live,
            // initialized one.
            cluster.set_bind_to_world(unsafe {
                matrix_mul(&raw const world_to_units, cluster.bind_to_world_ptr())
            });
            // C: `cluster->bind_to_world.cols[3].x` — the `cols[4]` overlay.
            let bind_cols: *mut Vec3 = cluster.bind_to_world_raw() as *mut Vec3;
            // SAFETY: an `ufbx_matrix` is laid out as exactly four consecutive
            // `ufbx_vec3` columns, so column `3` is its last one, in bounds.
            unsafe {
                (*bind_cols.add(3)).x *= translation_scale;
                (*bind_cols.add(3)).y *= translation_scale;
                (*bind_cols.add(3)).z *= translation_scale;
            }
            // SAFETY: `bind_to_world_raw()` projects the cluster's own live,
            // initialized, writable bind matrix.
            unsafe { mirror_matrix(cluster.bind_to_world_raw(), mirror_axis) };
        }

        // C: `ufbxi_for_ptr_list(ufbx_pose, p_pose, scene->poses)`
        let poses: &RefListView<Pose> = scene_view.poses_view();
        for pose_index in 0..poses.count() {
            // C: `ufbxi_for_list(ufbx_bone_pose, pose, (*p_pose)->bone_poses)`
            let bone_poses: &View<List<BonePose>> = poses.at(pose_index).bone_poses_view();
            for bone_index in 0..bone_poses.count() {
                let pose: &View<BonePose> = bone_poses.at(bone_index);
                // SAFETY: `&world_to_units` borrows a live, fully written local
                // matrix and `bone_to_world_ptr()` projects the bone pose's own
                // live, initialized one.
                pose.set_bone_to_world(unsafe {
                    matrix_mul(&raw const world_to_units, pose.bone_to_world_ptr())
                });
                // C: `pose->bone_to_world.cols[3].x` — the `cols[4]` overlay.
                let pose_cols: *mut Vec3 = pose.bone_to_world_raw() as *mut Vec3;
                // SAFETY: an `ufbx_matrix` is laid out as exactly four
                // consecutive `ufbx_vec3` columns, so column `3` is its last one,
                // in bounds.
                unsafe {
                    (*pose_cols.add(3)).x *= translation_scale;
                    (*pose_cols.add(3)).y *= translation_scale;
                    (*pose_cols.add(3)).z *= translation_scale;
                }
                // SAFETY: `bone_to_world_raw()` projects the bone pose's own
                // live, initialized, writable matrix.
                unsafe { mirror_matrix(pose.bone_to_world_raw(), mirror_axis) };
            }
        }
    }

    // Patch initial `mesh_node_to_bone`
    // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, scene->skin_clusters)`
    for i in 0..skin_clusters.count() {
        // C: `ufbx_skin_cluster *cluster = *p_cluster;`
        let cluster: &View<SkinCluster> = skin_clusters.at(i);

        // SAFETY: `element_raw()` projects the cluster's own live, initialized
        // element header, whose connection lists `ufbxi_fetch_src_element` walks.
        let skin: *mut SkinDeformer = unsafe {
            fetch_src_element(
                cluster.element_raw(),
                false,
                None,
                ElementType::SkinDeformer,
            )
        } as *mut SkinDeformer;
        if skin.is_null() {
            continue;
        }
        // SAFETY: `skin` is non-null here and was resolved from the cluster's own
        // connections, so it points to a live, initialized `ufbx_skin_deformer`
        // in the scene's arena, which carries write-capable provenance.
        let skin_view: &View<SkinDeformer> = unsafe { View::<SkinDeformer>::from_ptr(skin) };

        // SAFETY: `element_raw()` projects the deformer's own live, initialized
        // element header, whose connection lists `ufbxi_fetch_src_element` walks.
        let mut node: *mut Node =
            unsafe { fetch_src_element(skin_view.element_raw(), false, None, ElementType::Node) }
                as *mut Node;
        if node.is_null() {
            // SAFETY: as above, for the mesh connection of the same deformer.
            let mesh: *mut Mesh = unsafe {
                fetch_src_element(skin_view.element_raw(), false, None, ElementType::Mesh)
            } as *mut Mesh;
            // C: `mesh->instances` — the `ufbx_mesh` element-header union view
            // (ufbx.h), which the generated struct keeps as `element.instances`.
            if !mesh.is_null() {
                // SAFETY: `mesh` is non-null (checked), so it points to a live,
                // initialized `ufbx_mesh` in the arena that anchors a mesh view.
                let mesh_view: &View<Mesh> = unsafe { View::<Mesh>::from_ptr(mesh) };
                let instances: &View<RefList<Node>> = mesh_view.element().instances_view();
                if instances.count() > 0 {
                    // C: `node = mesh->instances.data[0];`
                    node = instances.at(0).get();
                }
            }
        }
        if node.is_null() {
            continue;
        }

        // Normalize to the non-helper node
        // SAFETY: `node` is non-null here and was resolved from the scene's own
        // element graph, so it points to a live, initialized `ufbx_node` in the
        // scene's arena, which carries write-capable provenance.
        let mut node_view: &View<Node> = unsafe { View::<Node>::from_ptr(node) };
        if node_view.is_geometry_transform_helper() {
            // C: `node = node->parent;`
            // SAFETY: a geometry transform helper is always created as a child of
            // the node it serves, so its parent link resolves to a live,
            // initialized `ufbx_node` of this scene.
            node_view = unsafe { View::<Node>::from_ptr(opt_ptr(node_view.parent_ptr())) };
        }

        // SAFETY: `mesh_node_to_bone_ptr()` projects the cluster's own live,
        // initialized matrix.
        if unsafe { matrix_all_zero(cluster.mesh_node_to_bone_ptr()) } {
            // If `mesh_node_to_bone` is not explicitly specified compute it from bind pose.
            // SAFETY: `bind_to_world_ptr()` projects the cluster's own live,
            // initialized matrix.
            let world_to_bind: Matrix = unsafe { matrix_invert(cluster.bind_to_world_ptr()) };
            // SAFETY: `&world_to_bind` borrows a live local matrix and
            // `node_to_world_ptr()` projects the node's own live, initialized one.
            cluster.set_mesh_node_to_bone(unsafe {
                matrix_mul(&raw const world_to_bind, node_view.node_to_world_ptr())
            });
        } else {
            // If `mesh_node_to_bone` is explicit, we may need to modify it for space conversion.
            // SAFETY: `mesh_node_to_bone_raw()` projects the cluster's own live,
            // initialized, writable matrix.
            unsafe { mirror_matrix(cluster.mesh_node_to_bone_raw(), mirror_axis) };
            if geometry_scale != 1.0 {
                // C: `cluster->mesh_node_to_bone.cols[3].x` — the `cols[4]` overlay.
                let cols: *mut Vec3 = cluster.mesh_node_to_bone_raw() as *mut Vec3;
                // SAFETY: an `ufbx_matrix` is laid out as exactly four
                // consecutive `ufbx_vec3` columns, so column `3` is its last one,
                // in bounds.
                unsafe {
                    (*cols.add(3)).x *= geometry_scale;
                    (*cols.add(3)).y *= geometry_scale;
                    (*cols.add(3)).z *= geometry_scale;
                }
            }
        }

        // HACK: Account for geometry transforms by looking at the transform of the
        // helper node if one is present. I don't think this is exactly how the skinning
        // matrices are formed.
        // TODO: Add a test with moving the skinned mesh root around.
        // C: `if (node->geometry_transform_helper)` — pointer truthiness.
        if node_view.geometry_transform_helper().is_some() {
            // C: `ufbx_node *geo_node = node->geometry_transform_helper;`
            // SAFETY: the branch condition established that the link is non-null,
            // so it points to a live, initialized `ufbx_node` of this scene.
            let geo_node: &View<Node> = unsafe {
                View::<Node>::from_ptr(opt_ptr(node_view.geometry_transform_helper_ptr()))
            };
            // SAFETY: both projections address their owners' own live,
            // initialized matrices.
            cluster.set_geometry_to_bone(unsafe {
                matrix_mul(
                    cluster.mesh_node_to_bone_ptr(),
                    geo_node.node_to_parent_ptr(),
                )
            });
        } else if node_view.has_geometry_transform() {
            // SAFETY: both projections address their owners' own live,
            // initialized matrices.
            cluster.set_geometry_to_bone(unsafe {
                matrix_mul(
                    cluster.mesh_node_to_bone_ptr(),
                    node_view.geometry_to_node_ptr(),
                )
            });
        } else {
            cluster.set_geometry_to_bone(cluster.mesh_node_to_bone());
        }
    }
}

// ufbx.c:23621-23632 `ufbxi_find_axis`
#[inline(never)]
pub(crate) fn find_axis(props: &PropsView, axis_name: &[u8], sign_name: &[u8]) -> CoordinateAxis {
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
    // SAFETY: `mat` points to writable — possibly uninitialized — storage for
    // one `ufbx_matrix` (fn contract), so the `size_of::<Matrix>()` bytes it
    // covers may be zeroed.
    unsafe { ptr::write_bytes(mat as *mut u8, 0, size_of::<Matrix>()) };
    // C: `mat->cols[i].v[j]` — the `cols[4]` / `v[3]` union overlay.
    let cols: *mut Vec3 = mat as *mut Vec3;
    // SAFETY: `mat` is writable `ufbx_matrix` storage (fn contract) laid out as
    // four consecutive `ufbx_vec3` columns, and `src`/`dst` carry real axes (fn
    // contract: not `UFBX_COORDINATE_AXIS_UNKNOWN`), so `src_x >> 1` is in
    // `0..3` and selects a column in bounds.
    let cx: *mut Real = unsafe { cols.add((src_x >> 1) as usize) } as *mut Real;
    // SAFETY: the column `cx` addresses is three consecutive `ufbx_real`s and
    // `dst_x >> 1` is in `0..3` (see above), so the element is in bounds — and
    // initialized, having been zeroed by the `write_bytes` above.
    unsafe {
        *cx.add((dst_x >> 1) as usize) = if ((src_x ^ dst_x) & 1) == 0 {
            1.0 as Real
        } else {
            -1.0 as Real
        }
    };
    // SAFETY: as for `cx`, for the column selected by `src_y >> 1`.
    let cy: *mut Real = unsafe { cols.add((src_y >> 1) as usize) } as *mut Real;
    // SAFETY: as for the `cx` element write, for `dst_y >> 1` within `cy`.
    unsafe {
        *cy.add((dst_y >> 1) as usize) = if ((src_y ^ dst_y) & 1) == 0 {
            1.0 as Real
        } else {
            -1.0 as Real
        }
    };
    // SAFETY: as for `cx`, for the column selected by `src_z >> 1`.
    let cz: *mut Real = unsafe { cols.add((src_z >> 1) as usize) } as *mut Real;
    // SAFETY: as for the `cx` element write, for `dst_z >> 1` within `cz`.
    unsafe {
        *cz.add((dst_z >> 1) as usize) = if ((src_z ^ dst_z) & 1) == 0 {
            1.0 as Real
        } else {
            -1.0 as Real
        }
    };

    true
}

// ufbx.c:23676-23804 `ufbxi_update_adjust_transforms`
#[inline(never)]
pub(crate) fn update_adjust_transforms<'a>(uc: &'a Context, scene: &'a SceneView) {
    let scene: *mut Scene = scene.get();
    let mut root_transform: Transform = IDENTITY_TRANSFORM;
    // SAFETY: pure value math over `uc`'s live axis-matrix field.
    unsafe {
        let axis_matrix: *const Matrix = uc.axis_matrix_mut_ptr();
        if !matrix_all_zero(axis_matrix) {
            root_transform = matrix_to_transform(axis_matrix);
        }
    }
    root_transform.scale.x *= uc.unit_scale();
    root_transform.scale.y *= uc.unit_scale();
    root_transform.scale.z *= uc.unit_scale();

    let conversion: SpaceConversion = uc.opts_view().space_conversion();

    let mut light_post_rotation: Quat = IDENTITY_QUAT;
    let mut camera_post_rotation: Quat = IDENTITY_QUAT;
    let mut light_direction: Vec3 = Vec3 {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    };
    let mut has_light_transform: bool = false;
    let mut has_camera_transform: bool = false;

    if coordinate_axes_valid(uc.opts_view().target_light_axes()) {
        let mut mat_storage = MaybeUninit::<Matrix>::uninit(); // ufbxi_uninit
        let mat: *mut Matrix = mat_storage.as_mut_ptr();
        let light_axes: CoordinateAxes = CoordinateAxes {
            right: CoordinateAxis::PositiveX,
            up: CoordinateAxis::NegativeZ,
            front: CoordinateAxis::PositiveY,
        };
        // SAFETY: `mat` is a local uninit `Matrix`; `axis_matrix` fully writes it
        // before returning true, so the reads below are of initialized memory.
        unsafe {
            if axis_matrix(mat, uc.opts_view().target_light_axes(), light_axes) {
                light_post_rotation = matrix_to_transform(mat).rotation;

                let inv: Matrix = matrix_invert(mat);
                light_direction = transform_direction(&inv, light_direction);
                has_light_transform = true;
            }
        }
    }

    if coordinate_axes_valid(uc.opts_view().target_camera_axes()) {
        let mut mat_storage = MaybeUninit::<Matrix>::uninit(); // ufbxi_uninit
        let mat: *mut Matrix = mat_storage.as_mut_ptr();
        let camera_axes: CoordinateAxes = CoordinateAxes {
            right: CoordinateAxis::PositiveZ,
            up: CoordinateAxis::PositiveY,
            front: CoordinateAxis::NegativeX,
        };
        // SAFETY: `mat` is a local uninit `Matrix`; `axis_matrix` fully writes it
        // before returning true, so the read below is of initialized memory.
        unsafe {
            if axis_matrix(mat, uc.opts_view().target_camera_axes(), camera_axes) {
                camera_post_rotation = matrix_to_transform(mat).rotation;
                has_camera_transform = true;
            }
        }
    }

    // SAFETY: walks the scene's stored `lights` element-pointer run (uc-owned
    // arena, `count` entries), resetting each light's own direction.
    unsafe {
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
    }

    // SAFETY: `scene` is the scene view's own storage; this run writes only its
    // metadata fields.
    unsafe {
        (*scene).metadata.space_conversion = conversion;
        (*scene).metadata.geometry_transform_handling =
            uc.opts_view().geometry_transform_handling();
        (*scene).metadata.inherit_mode_handling = uc.opts_view().inherit_mode_handling();
        (*scene).metadata.pivot_handling = uc.opts_view().pivot_handling();
        (*scene).metadata.handedness_conversion_axis = uc.opts_view().handedness_conversion_axis();
    }

    let root_scale: Real = min3(root_transform.scale);
    // SAFETY: `scene` metadata as above.
    unsafe {
        if conversion == SpaceConversion::ModifyGeometry {
            (*scene).metadata.geometry_scale = root_scale;
            (*scene).metadata.root_scale = 1.0 as Real;
        } else {
            (*scene).metadata.geometry_scale = 1.0 as Real;
            (*scene).metadata.root_scale = root_scale;
        }
        (*scene).metadata.root_rotation = root_transform.rotation;
    }

    // SAFETY: walks the scene's stored `nodes` element-pointer run (uc-owned arena,
    // `count` entries) and writes each node's own adjust fields; `opt_ptr`
    // results are null-checked before every deref, and `parent` is another node
    // of that same arena (hence the `&'a NodeView` anchor).
    unsafe {
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
            if !opt_ptr(&raw const (*node).parent).is_null() {
                // We are not inheriting local scale, so propagate root scale manually and
                // apply scale compensation if necessary.
                let parent: *mut Node = opt_ptr(&raw const (*node).parent);
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
                    // Anchor the traversed parent node to `'a` (= uc) via an explicit
                    // annotation: `parent` lives in the same uc arena, so its props
                    // table is provably `<= uc` with no free-lifetime bridge.
                    let parent_view: &'a NodeView = NodeView::from_ptr(parent);
                    let scale: Vec3 =
                        find_vec3(parent_view.props_view(), &sp::Lcl_Scaling, 1.0, 1.0, 1.0);
                    let mut size: Real = scale.x;
                    // C: `ufbx_fabs(scale.y - 1.0f) < ufbx_fabs(size - 1.0f)` — real
                    // subtractions, promoted to double at the `fabs` calls, compared
                    // in double.
                    if math::fabs(as_f64!(scale.y - 1.0)) < math::fabs((size - 1.0) as f64) {
                        size = scale.y;
                    }
                    if math::fabs(as_f64!(scale.z - 1.0)) < math::fabs((size - 1.0) as f64) {
                        size = scale.z;
                    }
                    (*node).adjust_post_scale *= 1.0 / size;
                    (*node).has_adjust_transform = true;
                }
            }

            if (*node).all_attribs.count == 1 {
                // C: `if (has_light_transform && node->light)` — pointer truthiness.
                if has_light_transform && !opt_ptr(&raw const (*node).light).is_null() {
                    (*node).adjust_post_rotation = light_post_rotation;
                    (*opt_ptr(&raw const (*node).light)).local_direction = light_direction;
                    (*node).has_adjust_transform = true;
                }
                if has_camera_transform && !opt_ptr(&raw const (*node).camera).is_null() {
                    (*node).adjust_post_rotation = camera_post_rotation;
                    (*opt_ptr(&raw const (*node).camera)).projection_axes =
                        uc.opts_view().target_camera_axes();
                    (*node).has_adjust_transform = true;
                }
            }

            p_node = p_node.add(1);
        }
    }
}

// ufbx.c:23806-23867 `ufbxi_update_scene`
#[inline(never)]
pub(crate) unsafe fn update_scene<'a>(
    scene_view: &'a SceneView,
    initial: bool,
    transform_overrides: *const TransformOverride,
    num_transform_overrides: usize,
) {
    // The scene VIEW is the uc-anchored dispatch root: `scene_view` is
    // `<= uc` (minted by the caller from `uc.scene_view()`), and every element
    // view dispatched below is projected out of it — each per-type list as a
    // `RefListView` whose `at()` yields the element view — so the property
    // tables reached through them stay provably within the `uc` arena scope.
    // Indexing a list view over its own `count()`, bound once before the loop,
    // is C's `ufbxi_for_ptr_list` walk of the `*mut *mut T` run.

    // The raw (`transform_overrides`, `num_transform_overrides`) pair crosses
    // the public evaluate ABI; make the slice exactly once at this boundary.
    // SAFETY: the fn contract is that `transform_overrides` addresses
    // `num_transform_overrides` readable, initialized `ufbx_transform_override`s
    // (sorted by `node_id`) live for this call; a zero count takes the empty
    // arm (the pointer may be null then).
    let overrides: &[TransformOverride] = if num_transform_overrides > 0 {
        unsafe { core::slice::from_raw_parts(transform_overrides, num_transform_overrides) }
    } else {
        &[]
    };

    // C: `ufbxi_for_ptr_list(ufbx_node, p_node, scene->nodes)`
    let nodes: &'a RefListView<Node> = scene_view.nodes_view();
    let num_nodes: usize = nodes.count();
    for i in 0..num_nodes {
        update_node(nodes.at(i), overrides);
    }

    // C: `ufbxi_for_ptr_list(ufbx_light, p_light, scene->lights)`
    let lights: &'a RefListView<Light> = scene_view.lights_view();
    let num_lights: usize = lights.count();
    for i in 0..num_lights {
        update_light(lights.at(i));
    }

    // C: `ufbxi_for_ptr_list(ufbx_camera, p_camera, scene->cameras)`
    let cameras: &'a RefListView<Camera> = scene_view.cameras_view();
    let num_cameras: usize = cameras.count();
    for i in 0..num_cameras {
        update_camera(scene_view, cameras.at(i));
    }

    // C: `ufbxi_for_ptr_list(ufbx_bone, p_bone, scene->bones)`
    let bones: &'a RefListView<Bone> = scene_view.bones_view();
    let num_bones: usize = bones.count();
    for i in 0..num_bones {
        update_bone(scene_view, bones.at(i));
    }

    // C: `ufbxi_for_ptr_list(ufbx_line_curve, p_line, scene->line_curves)`
    let line_curves: &'a RefListView<LineCurve> = scene_view.line_curves_view();
    let num_line_curves: usize = line_curves.count();
    for i in 0..num_line_curves {
        update_line_curve(line_curves.at(i));
    }

    if initial {
        update_initial_clusters(scene_view);

        // C: `ufbxi_for_ptr_list(ufbx_pose, p_pose, scene->poses)`
        let poses: &'a RefListView<Pose> = scene_view.poses_view();
        let num_poses: usize = poses.count();
        for i in 0..num_poses {
            update_pose(poses.at(i));
        }
    }

    // C: `ufbxi_for_ptr_list(ufbx_skin_cluster, p_cluster, scene->skin_clusters)`
    let skin_clusters: &'a RefListView<SkinCluster> = scene_view.skin_clusters_view();
    let num_skin_clusters: usize = skin_clusters.count();
    for i in 0..num_skin_clusters {
        update_skin_cluster(skin_clusters.at(i));
    }

    // C: `ufbxi_for_ptr_list(ufbx_blend_channel, p_channel, scene->blend_channels)`
    let blend_channels: &'a RefListView<BlendChannel> = scene_view.blend_channels_view();
    let num_blend_channels: usize = blend_channels.count();
    for i in 0..num_blend_channels {
        update_blend_channel(blend_channels.at(i));
    }

    // C: `ufbxi_for_ptr_list(ufbx_texture, p_texture, scene->textures)`
    let textures: &'a RefListView<Texture> = scene_view.textures_view();
    let num_textures: usize = textures.count();
    for i in 0..num_textures {
        update_texture(textures.at(i));
    }

    propagate_main_textures(scene_view);

    // C: `ufbxi_for_ptr_list(ufbx_material, p_material, scene->materials)`
    let materials: &'a RefListView<Material> = scene_view.materials_view();
    let num_materials: usize = materials.count();
    for i in 0..num_materials {
        update_material(scene_view, materials.at(i));
    }

    // C: `ufbxi_for_ptr_list(ufbx_anim_stack, p_stack, scene->anim_stacks)`
    let anim_stacks: &'a RefListView<AnimStack> = scene_view.anim_stacks_view();
    let num_anim_stacks: usize = anim_stacks.count();
    for i in 0..num_anim_stacks {
        update_anim_stack(scene_view, anim_stacks.at(i));
    }

    // C: `ufbxi_for_ptr_list(ufbx_display_layer, p_layer, scene->display_layers)`
    let display_layers: &'a RefListView<DisplayLayer> = scene_view.display_layers_view();
    let num_display_layers: usize = display_layers.count();
    for i in 0..num_display_layers {
        update_display_layer(display_layers.at(i));
    }

    // C: `ufbxi_for_ptr_list(ufbx_constraint, p_constraint, scene->constraints)`
    let constraints: &'a RefListView<Constraint> = scene_view.constraints_view();
    let num_constraints: usize = constraints.count();
    for i in 0..num_constraints {
        update_constraint(constraints.at(i));
    }

    update_anim(scene_view);
}

// ufbx.c:23869-23878 `ufbxi_update_scene_metadata`
#[inline(never)]
pub(crate) fn update_scene_metadata(metadata_view: &SceneMetadataView) {
    let metadata: *mut Metadata = metadata_view.get();
    let props: &PropsView = metadata_view.props_view();
    // SAFETY: `metadata` is the metadata view's own storage and `props` is that
    // same view's props; every lookup name is a NUL-terminated literal.
    unsafe {
        (*metadata).original_application.vendor =
            find_string_len(props, b"Original|ApplicationVendor", EMPTY_STRING.0);
        (*metadata).original_application.name =
            find_string_len(props, b"Original|ApplicationName", EMPTY_STRING.0);
        (*metadata).original_application.version =
            find_string_len(props, b"Original|ApplicationVersion", EMPTY_STRING.0);
        (*metadata).latest_application.vendor =
            find_string_len(props, b"LastSaved|ApplicationVendor", EMPTY_STRING.0);
        (*metadata).latest_application.name =
            find_string_len(props, b"LastSaved|ApplicationName", EMPTY_STRING.0);
        (*metadata).latest_application.version =
            find_string_len(props, b"LastSaved|ApplicationVersion", EMPTY_STRING.0);
    }
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
        // SAFETY: `targets` addresses `num_targets` initialized `ufbx_real`s (fn
        // contract) and `i` is in `0..num_targets` (loop bound).
        let target: f64 = as_f64!(unsafe { *targets.add(i) });
        let mut error: f64 = target * 9.5367431640625e-7;
        if error < 0.0 {
            error = -error;
        }
        if error < 7.52316384526264005e-37 {
            error = 7.52316384526264005e-37;
        }
        if as_f64!(value) >= target - error && as_f64!(value) <= target + error {
            return target as Real;
        }
    }
    value
}

// ufbx.c:23903-23931 `ufbxi_update_scene_settings`
#[inline(never)]
pub(crate) fn update_scene_settings(settings_view: &SceneSettingsView) {
    let settings: *mut SceneSettings = settings_view.get();
    // SAFETY: `settings` is the settings view's own storage; `round_if_near`
    // scans the static `POW10_TARGETS` with its own length.
    unsafe {
        let unit_scale_factor: Real = find_real(
            settings_view.props_view(),
            &sp::UnitScaleFactor,
            1.0 as Real,
        );
        let original_unit_scale_factor: Real = find_real(
            settings_view.props_view(),
            &sp::OriginalUnitScaleFactor,
            unit_scale_factor,
        );

        (*settings).axes.up = find_axis(settings_view.props_view(), &sp::UpAxis, &sp::UpAxisSign);
        (*settings).axes.front = find_axis(
            settings_view.props_view(),
            &sp::FrontAxis,
            &sp::FrontAxisSign,
        );
        (*settings).axes.right = find_axis(
            settings_view.props_view(),
            &sp::CoordAxis,
            &sp::CoordAxisSign,
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
            settings_view.props_view(),
            &sp::CustomFrameRate,
            24.0 as Real,
        ) as f64;
        (*settings).ambient_color =
            find_vec3(settings_view.props_view(), &sp::AmbientColor, 0.0, 0.0, 0.0);
        (*settings).original_axis_up = find_axis(
            settings_view.props_view(),
            &sp::OriginalUpAxis,
            &sp::OriginalUpAxisSign,
        );
    }

    // SAFETY: the returned prop pointer is null-checked before the deref, and
    // `settings` is the view's own storage.
    unsafe {
        let default_camera: *mut Prop = find_prop(settings_view.props_view(), &sp::DefaultCamera)
            .map_or(ptr::null_mut(), PropView::get);
        if !default_camera.is_null() {
            (*settings).default_camera = (*default_camera).value_str;
        } else {
            (*settings).default_camera = EMPTY_STRING.0;
        }
    }

    // SAFETY: `settings` as above; `ufbxi_find_enum` clamps each result to the
    // `[0, LAST]` range passed for its enum, so every transmute below is of an
    // in-range discriminant, and `time_mode` indexes `TIME_MODE_FPS` within that
    // same clamped range.
    unsafe {
        // C: `(ufbx_time_mode)ufbxi_find_enum(...)` etc — `ufbxi_find_enum` clamps
        // each result to its enum's `[0, LAST]` range (same device as
        // `ufbxi_update_camera` above).
        (*settings).time_mode = core::mem::transmute::<u32, TimeMode>(find_enum(
            settings_view.props_view(),
            &sp::TimeMode,
            TimeMode::E24Fps as i64,
            TimeMode::E5994Fps as i64,
        ) as u32);
        (*settings).time_protocol = core::mem::transmute::<u32, TimeProtocol>(find_enum(
            settings_view.props_view(),
            &sp::TimeProtocol,
            TimeProtocol::Default as i64,
            TimeProtocol::Default as i64,
        ) as u32);
        (*settings).snap_mode = core::mem::transmute::<u32, SnapMode>(find_enum(
            settings_view.props_view(),
            &sp::SnapOnFrameMode,
            SnapMode::None as i64,
            SnapMode::SnapAndPlay as i64,
        ) as u32);

        if (*settings).time_mode != TimeMode::Custom {
            // C: real `ufbxi_time_mode_fps[]` entry promotes to the `double` field.
            (*settings).frames_per_second =
                as_f64!(TIME_MODE_FPS[(*settings).time_mode as u32 as usize]);
        }
    }
}

// ufbx.c:23933-23944 `ufbxi_update_scene_settings_obj`
#[inline(never)]
pub(crate) fn update_scene_settings_obj(uc: &Context) {
    let settings: *mut SceneSettings = uc.scene_view().settings_mut_ptr();
    // SAFETY: `settings` is uc's own scene-settings storage, reached through
    // its raw-ptr getter (&Context construction invariant); this run writes
    // only that struct's fields.
    unsafe {
        // C: `settings->original_unit_meters = settings->unit_meters = uc->opts.obj_unit_meters;`
        (*settings).unit_meters = uc.opts_view().obj_unit_meters();
        (*settings).original_unit_meters = (*settings).unit_meters;
        if coordinate_axes_valid(uc.opts_view().obj_axes()) {
            (*settings).axes = uc.opts_view().obj_axes();
        } else {
            (*settings).axes.right = CoordinateAxis::Unknown;
            (*settings).axes.up = CoordinateAxis::Unknown;
            (*settings).axes.front = CoordinateAxis::Unknown;
        }
    }
}

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
    fn check_table(table: &[ShaderMapping], index_count: u8) {
        for mapping in table {
            assert!(mapping.transform < MatTransform::Count as u8);
            assert!(mapping.index < index_count);
            let mut len: usize = 0;
            // SAFETY: `prop` is a NUL-terminated static string literal in the
            // mapping table, so the scan stops inside the literal — which is
            // exactly the property this check exists to confirm.
            while unsafe { *mapping.prop.add(len) } != 0 {
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
            // SAFETY: `str_` comes from a shader mapping table (fn contract) and
            // is non-null in this branch, so it is one of the
            // `ufbxi_string_literal!` entries: a NUL-terminated literal whose
            // `length` is `.len() - 1`, so `data[length]` is in bounds.
            assert_eq!(unsafe { *str_.data.add(str_.length) }, 0);
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
