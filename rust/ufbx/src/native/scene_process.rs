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
//! DEFERRED from this unit: `ufbxi_generate_normals` (ufbx.c:20360-20403) —
//! see the note at its C-order slot below.
#![allow(dead_code)]

use core::ffi::c_void;
use core::mem::{size_of, MaybeUninit};
use core::ptr;

use crate::generated::{
    AnimCurve, AnimLayer, AnimProp, AnimValue, BlendKeyframe, BlendMode, BlendShape, BonePose,
    Connection, Constraint, ConstraintTarget, Element, ElementType, FileFormat,
    GeometryTransformHandling, InheritMode, InheritModeHandling, LodDisplay, LodGroup, LodLevel,
    Material, MaterialFbxMap, MaterialFbxMaps, MaterialFeature, MaterialFeatureInfo,
    MaterialFeatures, MaterialMap, MaterialPbrMap, MaterialPbrMaps, MaterialTexture, NameElement,
    Node, NurbsBasis, NurbsTopology, PivotHandling, Pose, Prop, PropFlags, PropType, Scene, Shader,
    ShaderPropBinding, ShaderType, SkinDeformer, SkinVertex, SkinWeight, Texture, TextureLayer,
    Vec3, Vec4, WarningType,
};
use crate::native::allocator::grow_array;
use crate::native::api::{
    find_bool as api_find_bool, find_int_len as api_find_int_len, find_prop_len,
    find_prop_texture_len, find_real as api_find_real, find_real_len as api_find_real_len,
    find_shader_prop_bindings_len, EMPTY_BLOB, EMPTY_STRING, IDENTITY_TRANSFORM, ZERO_VEC3,
};
use crate::native::buf::{buf_free, pop, push, push_copy, push_peek, push_pop, push_zero};
use crate::native::error::{
    strcmp, ufbxi_check, ufbxi_check_msg, ufbxi_snprintf, Fail, EMPTY_CHAR,
};
use crate::native::hash::{hash64, map_find};
use crate::native::parse::{
    find_enum, find_prop_with_key, find_real, find_vec3, get_element_extra, get_name_key,
    is_node_property_name, is_vec3_zero, is_vec4_zero, name_key_less, Context, FbxAttrEntry,
    FbxIdEntry, TmpConnection, TmpMaterialTexture,
};
use crate::native::platform::{
    add_ptr, f64_to_i64, macro_lower_bound_eq, macro_stable_sort, macro_upper_bound_eq, math,
    max32, max_sz, stable_sort, to_size, ufbx_assert, ufbxi_ignore, ufbxi_string_literal,
    ufbxi_unreachable,
};
use crate::native::read::{
    deduplicate_properties, find_fbx_id, init_synthetic_vec3_prop, opt_ptr, opt_ref, ref_ptr,
    setup_geometry_transform_helper, setup_scale_helper, sort_properties, NodeExtra,
    SENTINEL_INDEX_CONSECUTIVE, SENTINEL_INDEX_ZERO,
};
use crate::native::string_pool::{self as sp, add3, concat_str_cmp, neg3, str_cmp, str_less, sub3};
use crate::native::warnings::ufbxi_warnf_tag;
use crate::prelude::{List, Real, Ref, RefList, String};

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
    math::fabs(offset.x) >= epsilon
        || math::fabs(offset.y) >= epsilon
        || math::fabs(offset.z) >= epsilon
}

// ufbx.c:18099-18107 `ufbxi_pivot_div`
pub(crate) fn pivot_div(offset: Real, initial_scale: Real) -> Real {
    let epsilon: f64 = 0.0078125;
    if math::fabs(initial_scale) >= epsilon {
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
pub(crate) unsafe fn pre_finalize_scene(uc: *mut Context) -> Result<(), Fail> {
    let mut required: bool = false;
    if (*uc).opts.geometry_transform_handling == GeometryTransformHandling::HelperNodes
        || (*uc).opts.geometry_transform_handling == GeometryTransformHandling::ModifyGeometry
    {
        required = true;
    }
    if (*uc).opts.inherit_mode_handling == InheritModeHandling::HelperNodes
        || (*uc).opts.inherit_mode_handling == InheritModeHandling::Compensate
        || (*uc).opts.inherit_mode_handling == InheritModeHandling::CompensateNoFallback
    {
        required = true;
    }
    if (*uc).opts.pivot_handling == PivotHandling::AdjustToPivot
        || (*uc).opts.pivot_handling == PivotHandling::AdjustToRotationPivot
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

    let num_elements: u32 = (*uc).num_elements;
    let num_nodes: usize = (*uc).tmp_node_ids.num_items;
    let elements: *mut *mut Element = push_pop::<*mut Element>(
        &mut (*uc).tmp_parse,
        &mut (*uc).tmp_element_ptrs,
        num_elements as usize,
    );
    ufbxi_check!(uc, !elements.is_null(), "elements");

    let num_connections: usize = (*uc).tmp_connections.num_items;
    let tmp_connections: *mut TmpConnection = push_peek::<TmpConnection>(
        &mut (*uc).tmp_parse,
        &mut (*uc).tmp_connections,
        num_connections,
    );
    ufbxi_check!(uc, !tmp_connections.is_null(), "tmp_connections");

    let pre_connections: *mut PreConnection =
        push::<PreConnection>(&mut (*uc).tmp_parse, num_connections);
    ufbxi_check!(uc, !pre_connections.is_null(), "pre_connections");

    let instance_counts: *mut u32 = push_zero::<u32>(&mut (*uc).tmp_parse, num_elements as usize);
    ufbxi_check!(uc, !instance_counts.is_null(), "instance_counts");

    let modify_not_supported: *mut bool =
        push_zero::<bool>(&mut (*uc).tmp_parse, num_elements as usize);
    ufbxi_check!(uc, !modify_not_supported.is_null(), "modify_not_supported");

    let node_attrib_type: *mut ElementType =
        push_zero::<ElementType>(&mut (*uc).tmp_parse, num_nodes);
    ufbxi_check!(uc, !node_attrib_type.is_null(), "node_attrib_type");

    let has_unscaled_children: *mut bool = push_zero::<bool>(&mut (*uc).tmp_parse, num_nodes);
    ufbxi_check!(
        uc,
        !has_unscaled_children.is_null(),
        "has_unscaled_children"
    );

    let has_scale_animation: *mut bool = push_zero::<bool>(&mut (*uc).tmp_parse, num_nodes);
    ufbxi_check!(uc, !has_scale_animation.is_null(), "has_scale_animation");
    // C-parity: `has_scale_animation` is allocated and checked but never read
    // upstream; the allocation is observable so it stays.

    let pre_nodes: *mut PreNode = push_zero::<PreNode>(&mut (*uc).tmp_parse, num_nodes);
    ufbxi_check!(uc, !pre_nodes.is_null(), "pre_nodes");

    let num_meshes: usize = (*uc).tmp_typed_element_offsets[ElementType::Mesh as usize].num_items;
    let pre_meshes: *mut PreMesh = push_zero::<PreMesh>(&mut (*uc).tmp_parse, num_meshes);
    ufbxi_check!(uc, !pre_meshes.is_null(), "pre_meshes");

    let num_anim_values: usize =
        (*uc).tmp_typed_element_offsets[ElementType::AnimValue as usize].num_items;
    let pre_anim_values: *mut PreAnimValue =
        push_zero::<PreAnimValue>(&mut (*uc).tmp_parse, num_anim_values);
    ufbxi_check!(uc, !pre_anim_values.is_null(), "pre_anim_values");

    let fbx_ids: *mut u64 = push_pop::<u64>(
        &mut (*uc).tmp_parse,
        &mut (*uc).tmp_element_fbx_ids,
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
            (*pre_value).constant_value.x = find_real(&(*element).props, sp::X.as_ptr(), math::NAN);
            (*pre_value).constant_value.x = find_real(
                &(*element).props,
                sp::d_X.as_ptr(),
                (*pre_value).constant_value.x,
            );
            (*pre_value).constant_value.y = find_real(&(*element).props, sp::Y.as_ptr(), math::NAN);
            (*pre_value).constant_value.y = find_real(
                &(*element).props,
                sp::d_Y.as_ptr(),
                (*pre_value).constant_value.y,
            );
            (*pre_value).constant_value.z = find_real(&(*element).props, sp::Z.as_ptr(), math::NAN);
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

                    if (*uc).opts.inherit_mode_handling != InheritModeHandling::Preserve {
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
                    if math::isnan(*v) {
                        *v = constant_value;
                    }
                    if math::fabs(*v - constant_value) > scale_epsilon {
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
                                let mut error: Real = 0.0;
                                error += math::fabs(
                                    (*pre_value).constant_value.x - (*pre_node).constant_scale.x,
                                );
                                error += math::fabs(
                                    (*pre_value).constant_value.y - (*pre_node).constant_scale.y,
                                );
                                error += math::fabs(
                                    (*pre_value).constant_value.z - (*pre_node).constant_scale.z,
                                );
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

    if (*uc).opts.pivot_handling == PivotHandling::AdjustToPivot
        || (*uc).opts.pivot_handling == PivotHandling::AdjustToRotationPivot
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
            if (*uc).opts.pivot_handling == PivotHandling::AdjustToPivot {
                should_modify_pivot = !is_vec3_zero(rotation_pivot);
            } else if (*uc).opts.pivot_handling == PivotHandling::AdjustToRotationPivot {
                should_modify_pivot = pivot_nonzero(rotation_pivot)
                    || pivot_nonzero(scaling_pivot)
                    || pivot_nonzero(scaling_offset);
            }

            if should_modify_pivot {
                let mut skip_geometry_transform: bool = false;
                let mut can_modify_geometry_transform: bool = true;
                if (*uc).opts.pivot_handling == PivotHandling::AdjustToRotationPivot {
                    if *node_attrib_type.add((*node).element.typed_id as usize)
                        == ElementType::Empty
                    {
                        if !(*uc).opts.pivot_handling_retain_empties {
                            skip_geometry_transform = true;
                        } else {
                            can_modify_geometry_transform = false;
                        }
                    }
                }

                if (*uc).opts.geometry_transform_handling
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
                if (*uc).opts.pivot_handling == PivotHandling::AdjustToPivot {
                    let mut err: Real = 0.0;
                    err += math::fabs(rotation_pivot.x - scaling_pivot.x);
                    err += math::fabs(rotation_pivot.y - scaling_pivot.y);
                    err += math::fabs(rotation_pivot.z - scaling_pivot.z);
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
                    if (*uc).opts.pivot_handling == PivotHandling::AdjustToPivot {
                        ufbx_assert!(!skip_geometry_transform); // not supporeted in legacy mode
                        child_offset = neg3(rotation_pivot);
                        geometric_translation = add3(geometric_translation, child_offset);

                        new_props = push_zero::<Prop>(&mut (*uc).result, num_props + 3);
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
                    } else if (*uc).opts.pivot_handling == PivotHandling::AdjustToRotationPivot {
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

                        new_props = push_zero::<Prop>(&mut (*uc).result, num_props + 4);
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
            if (*uc).opts.geometry_transform_handling == GeometryTransformHandling::HelperNodes {
                requires_helper_node = true;
            } else if (*uc).opts.geometry_transform_handling
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
                    if (*uc).opts.inherit_mode_handling == InheritModeHandling::Compensate {
                        (*pre_node).constant_scale.x
                    } else {
                        1.0
                    };
                let scale: Vec3 = (*pre_node).constant_scale;
                let dx: Real = math::fabs(scale.x - r#ref);
                let dy: Real = math::fabs(scale.y - r#ref);
                let dz: Real = math::fabs(scale.z - r#ref);
                if (dx + dy + dz >= scale_epsilon
                    || !(*pre_node).has_constant_scale
                    || math::fabs(scale.x) <= compensate_epsilon)
                    && (*uc).opts.inherit_mode_handling != InheritModeHandling::CompensateNoFallback
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
                } else if (*uc).opts.inherit_mode_handling == InheritModeHandling::Compensate
                    || (*uc).opts.inherit_mode_handling == InheritModeHandling::CompensateNoFallback
                {
                    if math::fabs(scale.x - 1.0) >= scale_epsilon {
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
pub(crate) unsafe fn find_element_by_fbx_id(uc: *mut Context, fbx_id: u64) -> *mut Element {
    let entry: *mut FbxIdEntry = find_fbx_id(uc, fbx_id);
    if !entry.is_null() {
        return *((*uc).scene.elements.data as *mut *mut Element).add((*entry).element_id as usize);
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
    uc: *mut Context,
    name_elems: *mut NameElement,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            count.wrapping_mul(size_of::<NameElement>()),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, count * sizeof(ufbx_name_element))"
    );
    macro_stable_sort::<NameElement>(
        32,
        name_elems,
        (*uc).tmp_arr as *mut NameElement,
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
    uc: *mut Context,
    nodes: *mut *mut Node,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            count.wrapping_mul(size_of::<*mut Node>()),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, count * sizeof(ufbx_node*))"
    );
    macro_stable_sort::<*mut Node>(32, nodes, (*uc).tmp_arr as *mut *mut Node, count, |a, b| {
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
    uc: *mut Context,
    mat_texs: *mut TmpMaterialTexture,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            count.wrapping_mul(size_of::<TmpMaterialTexture>()),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, count * sizeof(ufbxi_tmp_material_texture))"
    );
    macro_stable_sort::<TmpMaterialTexture>(
        32,
        mat_texs,
        (*uc).tmp_arr as *mut TmpMaterialTexture,
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
    let a_src: *const Ref<Element> = &(*a).src;
    let b_src: *const Ref<Element> = &(*b).src;
    let a_elem: *mut Element = ref_ptr(a_src.add(index));
    let b_elem: *mut Element = ref_ptr(b_src.add(index));
    if a_elem != b_elem {
        return a_elem < b_elem;
    }
    let a_prop: *const String = &(*a).src_prop;
    let b_prop: *const String = &(*b).src_prop;
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
    uc: *mut Context,
    connections: *mut Connection,
    count: usize,
    index: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            count.wrapping_mul(size_of::<Connection>()),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, count * sizeof(ufbx_connection))"
    );
    macro_stable_sort::<Connection>(
        32,
        connections,
        (*uc).tmp_arr as *mut Connection,
        count,
        |a, b| cmp_connection_less(a as *mut Connection, b as *mut Connection, index),
    );
    Ok(())
}

// ufbx.c:18655-18663 `ufbxi_find_attribute_fbx_id`
pub(crate) unsafe fn find_attribute_fbx_id(uc: *mut Context, node_fbx_id: u64) -> u64 {
    let hash: u32 = hash64(node_fbx_id);
    let entry: *mut FbxAttrEntry = map_find(
        &mut (*uc).fbx_attr_map,
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
pub(crate) unsafe fn resolve_connections(uc: *mut Context) -> Result<(), Fail> {
    let num_connections: usize = (*uc).tmp_connections.num_items;
    let tmp_connections: *mut TmpConnection =
        push_pop(&mut (*uc).tmp, &mut (*uc).tmp_connections, num_connections);
    buf_free(&mut (*uc).tmp_connections);
    ufbxi_check!(uc, !tmp_connections.is_null(), "tmp_connections");

    // NOTE: We truncate this array in case not all connections are resolved
    (*uc).scene.connections_src.data = push::<Connection>(&mut (*uc).result, num_connections);
    ufbxi_check!(
        uc,
        !(*uc).scene.connections_src.data.is_null(),
        "uc->scene.connections_src.data"
    );

    // HACK: Translate property connections from node to attribute if the property name is not included
    // in the known node properties and is not a property of the node.
    if (*uc).version > 0 && (*uc).version < 7000 {
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

        if !(*uc).opts.disable_quirks {
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
                    "ufbxi_warnf_tag(UFBX_WARNING_BAD_ELEMENT_CONNECTED_TO_ROOT, src->element_id, \"Non-node element connected to root\")"
                );
                continue;
            }
        }

        // Remap connections to geometry transform helpers if necessary, see `ufbxi_setup_geometry_transform_helper()` for how these are setup.
        if (*uc).has_geometry_transform_nodes {
            if (*dst).type_ == ElementType::Node
                && (*src).type_ as u32 >= ELEMENT_TYPE_FIRST_ATTRIB
                && (*src).type_ as u32 <= ELEMENT_TYPE_LAST_ATTRIB
            {
                let node: *mut Node = dst as *mut Node;
                if (*node).has_geometry_transform {
                    let extra: *mut NodeExtra =
                        get_element_extra(uc, (*node).element.element_id) as *mut NodeExtra;
                    ufbx_assert!(!extra.is_null());
                    dst = *((*uc).scene.elements.data as *mut *mut Element)
                        .add((*extra).geometry_helper_id as usize);
                    ufbx_assert!(
                        (*dst).type_ == ElementType::Node
                            && (*(dst as *mut Node)).is_geometry_transform_helper
                    );
                }
            }
        }

        // Remap connections to scale helpers if necessary, see `ufbxi_setup_scale_helper()` for how these are setup.
        if (*uc).has_scale_helper_nodes {
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
        if (*uc).version > 0 && (*uc).version < 7000 && (*dst).type_ == ElementType::Node {
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

        let conn: *mut Connection = ((*uc).scene.connections_src.data as *mut Connection)
            .add((*uc).scene.connections_src.count);
        (*uc).scene.connections_src.count = (*uc).scene.connections_src.count.wrapping_add(1);
        (*conn).src = Ref::from_ptr(src);
        (*conn).dst = Ref::from_ptr(dst);
        (*conn).src_prop = (*tmp_conn).src_prop;
        (*conn).dst_prop = (*tmp_conn).dst_prop;
    }

    (*uc).scene.connections_dst.count = (*uc).scene.connections_src.count;
    (*uc).scene.connections_dst.data = push_copy::<Connection>(
        &mut (*uc).result,
        (*uc).scene.connections_src.count,
        (*uc).scene.connections_src.data,
    );
    ufbxi_check!(
        uc,
        !(*uc).scene.connections_dst.data.is_null(),
        "uc->scene.connections_dst.data"
    );

    sort_connections(
        uc,
        (*uc).scene.connections_src.data as *mut Connection,
        (*uc).scene.connections_src.count,
        0,
    )?;
    sort_connections(
        uc,
        (*uc).scene.connections_dst.data as *mut Connection,
        (*uc).scene.connections_dst.count,
        1,
    )?;

    // We don't need the temporary connections at this point anymore
    buf_free(&mut (*uc).tmp_connections);

    Ok(())
}

// ufbx.c:18782-18912 `ufbxi_add_connections_to_elements`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn add_connections_to_elements(uc: *mut Context) -> Result<(), Fail> {
    let mut conn_src: *mut Connection = (*uc).scene.connections_src.data as *mut Connection;
    let conn_src_end: *mut Connection = add_ptr(conn_src, (*uc).scene.connections_src.count);
    let mut conn_dst: *mut Connection = (*uc).scene.connections_dst.data as *mut Connection;
    let conn_dst_end: *mut Connection = add_ptr(conn_dst, (*uc).scene.connections_dst.count);

    // C: `ufbxi_for_ptr(ufbx_element, p_elem, uc->scene.elements.data, uc->scene.elements.count)`
    let mut p_elem: *mut *mut Element = (*uc).scene.elements.data as *mut *mut Element;
    let p_elem_end: *mut *mut Element = p_elem.add((*uc).scene.elements.count);
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
                            &mut (*uc).tmp_stack,
                            to_size(prop.offset_from(copy_start)),
                            copy_start,
                        )
                        .is_null(),
                        "ufbxi_push_copy(&uc->tmp_stack, ufbx_prop, ufbxi_to_size(prop - copy_start), copy_start)"
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
                        if (*uc).version < 6000 {
                            *(&mut (*anim_def).value_vec4 as *mut Vec4 as *mut Vec3) =
                                (*anim_value).default_value;
                        }
                        (*anim_def).type_ = type_;
                        def_prop = anim_def;
                    } else {
                        flags |= PropFlags::NO_VALUE.raw();
                    }

                    let new_prop: *mut Prop = push_zero(&mut (*uc).tmp_stack, 1);
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
                        &mut (*uc).tmp_stack,
                        to_size(prop_end.offset_from(copy_start)),
                        copy_start,
                    )
                    .is_null(),
                    "ufbxi_push_copy(&uc->tmp_stack, ufbx_prop, ufbxi_to_size(prop_end - copy_start), copy_start)"
                );
                (*elem).props.props.data =
                    push_pop::<Prop>(&mut (*uc).result, &mut (*uc).tmp_stack, num_new_props);
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
pub(crate) unsafe fn linearize_nodes(uc: *mut Context) -> Result<(), Fail> {
    let num_nodes: usize = (*uc).tmp_node_ids.num_items;
    let node_ids: *mut u32 = push_pop(&mut (*uc).tmp, &mut (*uc).tmp_node_ids, num_nodes);
    buf_free(&mut (*uc).tmp_node_ids);
    ufbxi_check!(uc, !node_ids.is_null(), "node_ids");

    let node_ptrs: *mut *mut Node = push(&mut (*uc).tmp_stack, num_nodes);
    ufbxi_check!(uc, !node_ptrs.is_null(), "node_ptrs");

    // Fetch the node pointers
    for i in 0..num_nodes {
        *node_ptrs.add(i) = *((*uc).scene.elements.data as *mut *mut Element)
            .add(*node_ids.add(i) as usize) as *mut Node;
        ufbx_assert!((**node_ptrs.add(i)).element.type_ == ElementType::Node);
    }

    // C reads `node_ptrs[0]` unconditionally; there is always at least the root
    // node in `tmp_node_ids` by the time this runs.
    (*uc).scene.root_node = Ref::from_ptr(*node_ptrs.add(0));

    let node_offsets: *mut usize = push_pop(
        &mut (*uc).tmp_stack,
        &mut (*uc).tmp_typed_element_offsets[ElementType::Node as usize],
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
            && !((*uc).opts.allow_nodes_out_of_root && (*uc).version >= 6000)
        {
            if node != ref_ptr(&(*uc).scene.root_node) {
                (*node).parent = Some((*uc).scene.root_node);
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

        if (*uc).opts.node_depth_limit > 0 {
            ufbxi_check_msg!(
                uc,
                depth <= (*uc).opts.node_depth_limit,
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
            &mut (*uc).tmp_typed_element_offsets[ElementType::Node as usize],
            1,
        );
        ufbxi_check!(uc, !p_offset.is_null(), "p_offset");
        let node: *mut Node = *node_ptrs.add(i as usize);

        let original_id: u32 = (*node).element.typed_id;
        (*node).element.typed_id = i;
        *p_offset = *node_offsets.add(original_id as usize);
    }

    // Pop the temporary arrays
    pop::<usize>(&mut (*uc).tmp_stack, num_nodes, ptr::null_mut());
    pop::<*mut Node>(&mut (*uc).tmp_stack, num_nodes, ptr::null_mut());

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
            &mut (*ref_ptr((*element).instances.data)).element as *mut Element
        } else {
            ptr::null_mut()
        }
    }
}

// ufbx.c:19047-19083 `ufbxi_fetch_dst_elements`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_dst_elements(
    uc: *mut Context,
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
                    if *(*uc).tmp_element_flag.add(element_id as usize) != 0 {
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
                            "ufbxi_warnf_tag(UFBX_WARNING_DUPLICATE_CONNECTION, element_id, \"Duplicate connection to %u\", element->element_id)"
                        );
                        continue;
                    }
                    *(*uc).tmp_element_flag.add(element_id as usize) = 1;
                }
                let p_elem: *mut *mut Element = push(&mut (*uc).tmp_stack, 1);
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
    (*list).data = push_pop::<*mut Element>(&mut (*uc).result, &mut (*uc).tmp_stack, num_elements)
        as *const Ref<Element>;
    (*list).count = num_elements;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    if ignore_duplicates {
        // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, *list)`
        let mut p_elem: *mut *mut Element = (*list).data as *mut *mut Element;
        let p_elem_end: *mut *mut Element = add_ptr(p_elem, (*list).count);
        while p_elem != p_elem_end {
            *(*uc).tmp_element_flag.add((**p_elem).element_id as usize) = 0;
            p_elem = p_elem.add(1);
        }
    }

    Ok(())
}

// ufbx.c:19085-19121 `ufbxi_fetch_src_elements`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_src_elements(
    uc: *mut Context,
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
                    if *(*uc).tmp_element_flag.add(element_id as usize) != 0 {
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
                            "ufbxi_warnf_tag(UFBX_WARNING_DUPLICATE_CONNECTION, element_id, \"Duplicate connection to %u\", element->element_id)"
                        );
                        continue;
                    }
                    *(*uc).tmp_element_flag.add(element_id as usize) = 1;
                }
                let p_elem: *mut *mut Element = push(&mut (*uc).tmp_stack, 1);
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
    (*list).data = push_pop::<*mut Element>(&mut (*uc).result, &mut (*uc).tmp_stack, num_elements)
        as *const Ref<Element>;
    (*list).count = num_elements;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    if ignore_duplicates {
        // C: `ufbxi_for_ptr_list(ufbx_element, p_elem, *list)`
        let mut p_elem: *mut *mut Element = (*list).data as *mut *mut Element;
        let p_elem_end: *mut *mut Element = add_ptr(p_elem, (*list).count);
        while p_elem != p_elem_end {
            *(*uc).tmp_element_flag.add((**p_elem).element_id as usize) = 0;
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
    uc: *mut Context,
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
                let tex: *mut MaterialTexture = push(&mut (*uc).tmp_stack, 1);
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
        push_pop::<MaterialTexture>(&mut (*uc).result, &mut (*uc).tmp_stack, num_textures);
    (*list).count = num_textures;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    Ok(())
}

// ufbx.c:19175-19197 `ufbxi_fetch_mesh_materials`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_mesh_materials(
    uc: *mut Context,
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
                    !push_copy::<*mut Material>(&mut (*uc).tmp_stack, 1, &mat).is_null(),
                    "ufbxi_push_copy(&uc->tmp_stack, ufbx_material*, 1, &mat)"
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

    (*list).data = push_pop::<*mut Material>(&mut (*uc).result, &mut (*uc).tmp_stack, num_materials)
        as *const Ref<Material>;
    (*list).count = num_materials;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    Ok(())
}

// ufbx.c:19199-19219 `ufbxi_fetch_deformers`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_deformers(
    uc: *mut Context,
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
                        &mut (*uc).tmp_stack,
                        1,
                        &(*conn).src as *const Ref<Element> as *const *mut Element,
                    )
                    .is_null(),
                    "ufbxi_push_copy(&uc->tmp_stack, ufbx_element*, 1, &conn->src)"
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

    (*list).data = push_pop::<*mut Element>(&mut (*uc).result, &mut (*uc).tmp_stack, num_deformers)
        as *const Ref<Element>;
    (*list).count = num_deformers;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    Ok(())
}

// ufbx.c:19221-19239 `ufbxi_fetch_blend_keyframes`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_blend_keyframes(
    uc: *mut Context,
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
                !push_copy::<BlendKeyframe>(&mut (*uc).tmp_stack, 1, &key).is_null(),
                "ufbxi_push_copy(&uc->tmp_stack, ufbx_blend_keyframe, 1, &key)"
            );
            num_keyframes += 1;
        }
        conn = conn.add(1);
    }

    (*list).data =
        push_pop::<BlendKeyframe>(&mut (*uc).result, &mut (*uc).tmp_stack, num_keyframes);
    (*list).count = num_keyframes;
    ufbxi_check!(uc, !(*list).data.is_null(), "list->data");

    Ok(())
}

// ufbx.c:19241-19262 `ufbxi_fetch_texture_layers`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn fetch_texture_layers(
    uc: *mut Context,
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
                !push_copy::<TextureLayer>(&mut (*uc).tmp_stack, 1, &layer).is_null(),
                "ufbxi_push_copy(&uc->tmp_stack, ufbx_texture_layer, 1, &layer)"
            );
            num_layers += 1;
        }
        conn = conn.add(1);
    }

    (*list).data = push_pop::<TextureLayer>(&mut (*uc).result, &mut (*uc).tmp_stack, num_layers);
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
pub(crate) unsafe fn patch_index_pointer(uc: *mut Context, p_index: *mut *mut u32) {
    if *p_index == SENTINEL_INDEX_ZERO.as_ptr() as *mut u32 {
        *p_index = (*uc).zero_indices;
    } else if *p_index == SENTINEL_INDEX_CONSECUTIVE.as_ptr() as *mut u32 {
        *p_index = (*uc).consecutive_indices;
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
    uc: *mut Context,
    aprops: *mut AnimProp,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            count.wrapping_mul(size_of::<AnimProp>()),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, count * sizeof(ufbx_anim_prop))"
    );
    macro_stable_sort::<AnimProp>(32, aprops, (*uc).tmp_arr as *mut AnimProp, count, |a, b| {
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
    uc: *mut Context,
    textures: *mut MaterialTexture,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            count.wrapping_mul(size_of::<MaterialTexture>()),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, count * sizeof(ufbx_material_texture))"
    );
    stable_sort(
        size_of::<MaterialTexture>(),
        32,
        textures as *mut c_void,
        (*uc).tmp_arr as *mut c_void,
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
pub(crate) unsafe fn sort_bone_poses(uc: *mut Context, pose: *mut Pose) -> Result<(), Fail> {
    let count: usize = (*pose).bone_poses.count;
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            (*pose).bone_poses.count.wrapping_mul(size_of::<BonePose>()),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, pose->bone_poses.count * sizeof(ufbx_bone_pose))"
    );
    stable_sort(
        size_of::<BonePose>(),
        16,
        (*pose).bone_poses.data as *mut c_void,
        (*uc).tmp_arr as *mut c_void,
        count,
        bone_pose_less,
        ptr::null_mut(),
    );
    Ok(())
}

// ufbx.c:19345-19356 `ufbxi_sort_skin_weights`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn sort_skin_weights(
    uc: *mut Context,
    skin: *mut SkinDeformer,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            (*skin)
                .max_weights_per_vertex
                .wrapping_mul(size_of::<SkinWeight>()),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, skin->max_weights_per_vertex * sizeof(ufbx_skin_weight))"
    );

    for i in 0..(*skin).vertices.count {
        let v: SkinVertex = *(*skin).vertices.data.add(i);
        macro_stable_sort::<SkinWeight>(
            32,
            ((*skin).weights.data as *mut SkinWeight).add(v.weight_begin as usize),
            (*uc).tmp_arr as *mut SkinWeight,
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
    uc: *mut Context,
    keyframes: *mut BlendKeyframe,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            count.wrapping_mul(size_of::<BlendKeyframe>()),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, count * sizeof(ufbx_blend_keyframe))"
    );
    stable_sort(
        size_of::<BlendKeyframe>(),
        32,
        keyframes as *mut c_void,
        (*uc).tmp_arr as *mut c_void,
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
pub(crate) const MAT_MATTE: u32 = 1 << MaterialFeature::Matte as u32;
pub(crate) const MAT_UNLIT: u32 = 1 << MaterialFeature::Unlit as u32;
pub(crate) const MAT_IOR: u32 = 1 << MaterialFeature::Ior as u32;
pub(crate) const MAT_DIFFUSE_ROUGHNESS: u32 = 1 << MaterialFeature::DiffuseRoughness as u32;
pub(crate) const MAT_TRANSMISSION_ROUGHNESS: u32 =
    1 << MaterialFeature::TransmissionRoughness as u32;
pub(crate) const MAT_THIN_WALLED: u32 = 1 << MaterialFeature::ThinWalled as u32;
pub(crate) const MAT_CAUSTICS: u32 = 1 << MaterialFeature::Caustics as u32;
pub(crate) const MAT_EXIT_TO_BACKGROUND: u32 = 1 << MaterialFeature::ExitToBackground as u32;
pub(crate) const MAT_INTERNAL_REFLECTIONS: u32 = 1 << MaterialFeature::InternalReflections as u32;
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
                        (*map).value_int = f64_to_i64((*map).value_vec4.x);
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
    uc: *mut Context,
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
                let target: *mut ConstraintTarget = push_zero(&mut (*uc).tmp_stack, 1);
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
    uc: *mut Context,
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
            let spans: *mut Real = push(&mut (*uc).result, max_spans);
            ufbxi_check!(uc, !spans.is_null(), "spans");

            let mut prev: Real = -math::INFINITY;
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
pub(crate) unsafe fn finalize_lod_group(uc: *mut Context, lod: *mut LodGroup) -> Result<(), Fail> {
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

    let levels: *mut LodLevel = push_zero(&mut (*uc).result, num_levels);
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

// DEFERRED: `ufbxi_generate_normals` (ufbx.c:20364-20403) — it calls the
// public topology entry points `ufbx_compute_topology`,
// `ufbx_generate_normal_mapping` and `ufbx_compute_normals`
// (ufbx.c:32477-32617), which sit in the not-yet-ported
// `// -- Topology` banner section (`native::topology`). Port it here, in this
// C-order slot, once that section lands. Its only caller is
// `ufbxi_finalize_scene` (ufbx.c:21641), called at ufbx.c:22063, which is
// itself unported.

// ufbx.c:20405-20427 `ufbxi_push_prop_prefix`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_prop_prefix(
    uc: *mut Context,
    dst: *mut String,
    mut prefix: String,
) -> Result<(), Fail> {
    let mut stack_size: usize = 0;
    if prefix.length > 0 && *prefix.data.add(prefix.length - 1) != b'|' {
        stack_size = prefix.length + 1;
        let copy: *mut u8 = push(&mut (*uc).tmp_stack, stack_size);
        ufbxi_check!(uc, !copy.is_null(), "copy");
        ptr::copy_nonoverlapping(prefix.data, copy, prefix.length);
        *copy.add(prefix.length) = b'|';

        prefix.data = copy;
        prefix.length += 1;
    }

    sp::push_string_place_str(&mut (*uc).string_pool, &mut prefix, false)?;
    *dst = prefix;

    if stack_size > 0 {
        pop::<u8>(&mut (*uc).tmp_stack, stack_size, ptr::null_mut());
    }

    Ok(())
}

// CONTINUATION POINT (milestone 7b): `// -- Scene processing` ported through
// `ufbxi_push_prop_prefix` (ufbx.c:18997-20427). Next:
// `ufbxi_shader_texture_find_prefix` (ufbx.c:20429) and the rest of the
// shader-texture / texture-file finalization.
// DEFERRED and still owed from this range: `ufbxi_generate_normals`
// (ufbx.c:20364-20403) — see the note at its C-order slot above.

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
}
