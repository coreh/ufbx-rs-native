//! Port of the `// -- Reading the parsed data` banner section (ufbx.c:11762-15311).
//!
//! Coverage: ufbx.c:11762-12218 — embedded blobs, the property reader with
//! its sort/dedup helpers, thumbnails, `SceneInfo`, the header extension
//! (including the KTime-unit decision), exporter matching from the `Creator`
//! string, the document root ID and the `Definitions` property templates.
//!
//! Coverage: ufbx.c:12220-12653 — the name-ID categories and synthetic-ID
//! allocation, `Type::Name` splitting, the FBX-ID / FBX-attr hash maps, the
//! element push foundations (`ufbxi_push_element_size` /
//! `ufbxi_push_synthetic_element_size`), the tmp-connection builders, the
//! synthetic property initializers, the geometry-transform and scale helper
//! node setup, and the first per-type readers (`Model`, the generic element
//! reader and `Unknown`).
//!
//! Coverage: ufbx.c:12655-13137 — the index sentinels and index-error
//! handling (`ufbxi_fix_index` / `ufbxi_check_indices`), the vertex-attribute
//! reader with its mapping-mode dispatch, truncated-array reading, the
//! UV/color-set and blend-offset sorts, and the blend shape readers
//! (`ufbxi_read_shape` / `ufbxi_read_synthetic_blend_shapes`).
//!
//! Coverage: ufbx.c:13139-13432 — index processing (`ufbxi_process_indices`
//! builds the face list, vertex-first-index table and the consecutive/zero
//! index hints), `ufbxi_patch_mesh_reals`, and the face-group machinery
//! (`ufbxi_assign_face_groups` / `ufbxi_update_face_groups`) with its loose
//! hash dedup, `ufbxi_less_int32` unstable sort and mesh-part accounting.
//!
//! Coverage: ufbx.c:13434-13894 — `ufbxi_read_mesh` (the geometry node
//! reader: vertices/indices, edges, the `LayerElement*` dispatch loop with the
//! 6x00 mesh-texture layers, the `Layer` tangent/bitangent-to-UV-set binding
//! and the subdivision properties) plus the NURBS readers
//! (`ufbxi_read_nurbs_topology` / `ufbxi_read_nurbs_curve` /
//! `ufbxi_read_nurbs_surface`).
//!
//! Coverage: ufbx.c:13896-14255 — `ufbxi_read_line` (segment splitting on
//! the complemented end-point indices), `ufbxi_read_transform_matrix` and the
//! deformer readers (`ufbxi_read_bone`, `ufbxi_read_marker`,
//! `ufbxi_read_skin`, `ufbxi_read_skin_cluster`, `ufbxi_read_blend_channel`),
//! plus the animation-curve tangent foundations: the `ufbxi_key_flags` bits,
//! the three auto-tangent solvers, `ufbxi_solve_tcb` and
//! `ufbxi_read_extrapolation`.
//!
//! Coverage: ufbx.c:14257-14771 — `ufbxi_read_animation_curve` (the
//! run-length-encoded keyframe/tangent decoder), the material/texture/video
//! readers with their filename fallback ladders, `ufbxi_read_anim_stack` (name
//! map), `ufbxi_read_pose`, the shader binding table with its stable sort, and
//! the selection set/node readers.
//!
//! Coverage: ufbx.c:14773-15310 (end of section) — the remaining leaf
//! readers (`ufbxi_read_character`, `ufbxi_read_audio_clip`,
//! `ufbxi_read_constraint` with its type-name table), the 6x00
//! node-attribute splitter `ufbxi_read_synthetic_attribute`,
//! `ufbxi_read_global_settings`, the big `ufbxi_read_object` type dispatch,
//! the serial and threaded object loops (`ufbxi_read_objects` /
//! `ufbxi_read_objects_threaded` with `ufbxi_object_batch`) and
//! `ufbxi_read_connections`.
//!
//! The `// -- Reading the parsed data` section is now fully ported.
//!
//! Coverage: ufbx.c:15312-15764 — the start of the
//! `// -- Pre-7000 "Take" based animation` banner section:
//! `ufbxi_double_to_char` and the heterogenous-`double`-array keyframe decoder
//! `ufbxi_read_take_anim_channel` (slope/weight mode ladder), the recursive
//! `ufbxi_read_take_prop_channel` (`Transform` flattening and the 1-3 compound
//! channel scan), `ufbxi_read_take_object`, `ufbxi_read_take` (Take → anim
//! stack + `BaseLayer` synthesis, plus the post-7000 time-only fallback) and
//! the `ufbxi_read_takes` top-level loop.
//!
//! Coverage: ufbx.c:15766-16332 — `ufbxi_read_legacy_settings` (the pre-6000
//! `Version5/Settings` frame-rate override), the root-node setup helpers
//! (`ufbxi_unscaled_transform_to_matrix`, `ufbxi_setup_root_node`,
//! `ufbxi_supports_version`) and the pre-6000 "legacy" object readers: the
//! `ufbxi_legacy_prop` format-string tables plus `ufbxi_read_legacy_prop` /
//! `ufbxi_read_legacy_props`, then `ufbxi_read_legacy_material`,
//! `ufbxi_read_legacy_link`, `ufbxi_read_legacy_light`,
//! `ufbxi_read_legacy_camera`, `ufbxi_read_legacy_limb_node` and
//! `ufbxi_read_legacy_mesh`, and the top-level driver `ufbxi_read_root`.
//!
//! Coverage: ufbx.c:16333-16765 (end of section) — the remaining pre-6000
//! readers `ufbxi_read_legacy_media` / `ufbxi_read_legacy_model`, the filename
//! manipulation block (`ufbxi_trim_delimiters`, `ufbxi_init_file_paths`, the
//! `ufbxi_strblob` string/blob overlay with its `raw`-discriminated accessors,
//! `ufbxi_is_absolute_path` and `ufbxi_resolve_relative_filename`), the
//! `ufbxi_open_file` callback shim, and the shared mesh finalizer
//! (`ufbxi_patch_zero`, `ufbxi_update_vertex_first_index`,
//! `ufbxi_finalize_mesh`), and the legacy driver `ufbxi_read_legacy_root`.
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
use core::ffi::c_void;
use core::mem::size_of;
use core::mem::MaybeUninit;

use crate::generated::{
    AnimCurve, AnimLayer, AnimStack, AnimValue, AudioClip, AudioLayer, BlendChannel, BlendDeformer,
    BlendShape, Bone, BonePose, CacheDeformer, CacheFile, Camera, CameraSwitcher, Character,
    ColorSet, Constraint, ConstraintType, DisplayLayer, Edge, Element, ElementType, Empty,
    Exporter, Extrapolation, ExtrapolationMode, Face, FaceGroup, IndexErrorHandling, InheritMode,
    InheritModeHandling, Interpolation, Keyframe, Light, LineCurve, LineSegment, LodGroup, Marker,
    MarkerType, Material, Matrix, Mesh, MeshPart, MetadataObject, Node as UfbxNode, NurbsCurve,
    NurbsSurface, NurbsTopology, NurbsTrimBoundary, NurbsTrimSurface, OpenFileInfo, OpenFileType,
    Pose, Prop, PropFlags, PropType, Props, RawOpenFileCb, RawStream, SelectionNode, SelectionSet,
    Shader, ShaderBinding, ShaderPropBinding, SkinCluster, SkinDeformer, SkinningMethod,
    StereoCamera, SubdivisionBoundary, SubdivisionDisplayMode, Tangent, Texture, TextureType,
    Thumbnail, ThumbnailFormat, TimeMode, Transform, Unknown, UvSet, Vec3, Vec4, VertexAttrib,
    VertexReal, VertexVec2, VertexVec3, VertexVec4, Video, VoidList, WarningType,
};
use crate::native::allocator::{grow_array, AllocatorView};
use crate::native::api::{
    find_int_len as api_find_int_len, find_prop_len, transform_to_matrix, EMPTY_BLOB, EMPTY_STRING,
    IDENTITY_MATRIX, IDENTITY_TRANSFORM,
};
use crate::native::buf::{buf_clear, pop, push_size, BufView};
use crate::native::error::{
    c_strcmp, memchr, memcmp, strcmp, strlen, strncmp, ufbxi_check, ufbxi_check_err,
    ufbxi_check_msg, ufbxi_check_return, ufbxi_check_some, ufbxi_fail, ufbxi_fail_msg,
    ufbxi_fmt_err_info, Fail, EMPTY_CHAR,
};
use crate::native::float_parse::parse_double;
use crate::native::hash::{hash64, hash_ptr_id, PtrId};
use crate::native::parse::{
    array_type_size, find_array, find_child, find_child_strcmp, find_int, find_prop, find_val1,
    find_val2, find_vec3, get_array, get_dom_node, get_name_key, get_prop_type, get_val1, get_val2,
    get_val3, get_val4, get_val5, get_val_at, get_val_type, init_node_prop_names,
    is_node_property_name, is_vec3_one, is_vec3_zero, parse_legacy_toplevel, parse_toplevel,
    parse_toplevel_child, push_element_extra, retain_toplevel, AsReal, AsciiView, Checked, Context,
    ElementInfo, FbxAttrEntry, FbxIdEntry, Ignore, MeshExtra, Node, NodeView, PropView, PropsView,
    PtrFbxIdEntry, Template, TextureExtra, TmpAnimStack, TmpBonePose, TmpConnection,
    TmpMeshTexture, Unchecked, ValueArray, ValueType,
};
use crate::native::platform::{
    add_ptr, f64_to_i64, macro_stable_sort, macro_stable_sort_views, math, max_real, max_sz, min32,
    min_real, min_sz, pack_version, stable_sort, to_size, ufbx_assert, ufbxi_ignore,
    ufbxi_maybe_null, ufbxi_unreachable, unstable_sort, FACE_GROUP_HASH_BITS, NO_INDEX,
};
use crate::native::string_pool as sp;
use crate::native::string_pool::{push_string_place_blob, push_string_place_str};
use crate::native::thread::{
    thread_pool_available_tasks, thread_pool_flush_group, thread_pool_wait_all,
    thread_pool_wait_group, THREAD_GROUP_COUNT,
};
use crate::native::view::{view_project, view_raw_mut, view_read, view_read_shared, view_write};
use crate::native::view::{Const, Mode, Mut, Run, SliceViewIter, View};
use crate::native::warnings::ufbxi_warnf;
use crate::prelude::as_f64;
use crate::prelude::{
    slice_from_ptr, Blob, BlobView, List, ListView, OpenFileContext, Real, Ref, ScalarView, String,
    StringView,
};

// ufbx.h:3618 `UFBX_ENUM_TYPE(ufbx_thumbnail_format, UFBX_THUMBNAIL_FORMAT, UFBX_THUMBNAIL_FORMAT_RGBA_32);`
// expanding to `enum { UFBX_THUMBNAIL_FORMAT_COUNT = UFBX_THUMBNAIL_FORMAT_RGBA_32 + 1 }`.
// Hand-duplicated from the generated enum's last variant so an upstream enum
// change tracks automatically through regen (precedent: `ELEMENT_TYPE_COUNT`
// in `native::parse`). `i32` because C compares it against an `int32_t`.
pub(crate) const THUMBNAIL_FORMAT_COUNT: i32 = ThumbnailFormat::Rgba32 as i32 + 1;

// ufbx.c:11764-11796 `ufbxi_read_embedded_blob`
#[inline(never)]
pub(crate) fn read_embedded_blob(
    uc: &Context,
    dst_blob: &BlobView,
    node: Option<&NodeView>,
) -> Result<(), Fail> {
    let node: &NodeView = match node {
        Some(node) => node,
        None => return Ok(()),
    };

    let content_arr: *mut ValueArray = get_array(node, b'C');
    if !content_arr.is_null() {
        // SAFETY: `get_array` returned the node's own initialized array
        // descriptor, which stays live and unwritten with the parse tree.
        let content_arr = unsafe { View::<ValueArray, Const>::from_ptr(content_arr) };
        if content_arr.size() == 0 {
            return Ok(());
        }

        let content: String;
        let num_parts = content_arr.size();
        let parts_data = content_arr.data() as *const String;
        // SAFETY: the nonempty `'C'` array payload contains `num_parts`
        // initialized string descriptors and stays live and unwritten with the
        // parse tree.
        let parts = unsafe { Run::<String, Const>::from_const_raw_parts(parts_data, num_parts) };

        if num_parts == 1 && !uc.from_ascii() {
            content = parts.copy_at(0);
        } else {
            let mut total_size: usize = 0;
            // C: `ufbxi_for(ufbx_string, part, parts, num_parts)`
            for part in parts.iter() {
                total_size = total_size.wrapping_add(part.length());
            }
            let dst_begin: *mut u8 = uc.result_view().push::<u8>(total_size);
            ufbxi_check!(uc, !dst_begin.is_null(), "dst");
            content = String::new_c(dst_begin, total_size);
            let mut dst = dst_begin;
            for part in parts.iter() {
                let part_data = part.data();
                let part_length = part.length();
                // SAFETY: each part spans `part_length` readable bytes; `dst`
                // walks the disjoint result-arena buffer sized to the wrapping
                // sum above. The parser-owned part allocations coexist, so
                // their valid lengths cannot overflow the address space; the
                // wrapping sum retains C arithmetic. The copy and pointer
                // advance therefore stay within the destination.
                unsafe {
                    core::ptr::copy_nonoverlapping(part_data, dst, part_length);
                    dst = dst.add(part_length);
                }
            }
        }

        // `content` either borrows the retained parse-tree string run or the
        // result-arena copy assembled above; the Blob interpretation preserves
        // that descriptor's storage and lifetime.
        dst_blob.set(Blob::from_string(content));
    }

    Ok(())
}

// ufbx.c:11798-11869 `ufbxi_read_property`
#[inline(never)]
pub(crate) fn read_property(
    uc: &Context,
    node: &NodeView,
    prop: &PropView,
    version: i32,
) -> Result<(), Fail> {
    let mut subtype_str: *const u8 = core::ptr::null();
    let (Checked(name), Checked(type_str)) = ufbxi_check_some!(
        uc,
        get_val2::<Checked<String>, Checked<*const u8>>(node),
        "ufbxi_get_val2(node, \"SC\", &prop->name, (char**)&type_str)"
    );
    prop.set_name(name);
    let mut val_ix: u32 = 2;
    if version == 70 {
        // C: `ufbxi_get_val_at(node, val_ix++, ...)` — the post-increment
        // happens while evaluating the (single) check condition.
        let ix = val_ix;
        val_ix = val_ix.wrapping_add(1);
        subtype_str = ufbxi_check_some!(
            uc,
            get_val_at::<Checked<*const u8>>(node, ix as usize),
            "ufbxi_get_val_at(node, val_ix++, 'C', (char**)&subtype_str)"
        )
        .0;
    }

    let mut flags: u32 = 0;
    // `name` was filled in by the `"SC"` fetch above (an interned pool string).
    prop.set_internal_key(get_name_key(prop.name_view().bytes()));

    // C leaves `flags_str` uninitialized; it is only read when the `'S'` fetch
    // below succeeds, which fully writes it.
    let ix = val_ix;
    val_ix = val_ix.wrapping_add(1);
    if let Some(Checked(flags_str)) = get_val_at::<Checked<String>>(node, ix as usize) {
        let mut i: usize = 0;
        while i < flags_str.length {
            // C-parity: `char next` reads a `const char *` — signed bytes on
            // the oracle targets (PORTING.md `char` value row).
            let next: i8 = if i + 1 < flags_str.length {
                // SAFETY: the `'S'` fetch succeeded, so `flags_str.data` spans
                // `flags_str.length` readable bytes and `i + 1 < length`.
                unsafe { *(flags_str.data.add(i + 1) as *const i8) }
            } else {
                b'0' as i8
            };
            // SAFETY: as above, with `i < flags_str.length`.
            match unsafe { *flags_str.data.add(i) } {
                b'A' => flags |= PropFlags::ANIMATABLE.raw(),
                b'U' => flags |= PropFlags::USER_DEFINED.raw(),
                b'H' => flags |= PropFlags::HIDDEN.raw(),
                // UFBX_PROP_FLAG_LOCK_*
                b'L' => flags |= (((next as i32 - b'0' as i32) as u32) & 0xf) << 4,
                // UFBX_PROP_FLAG_MUTE_*
                b'M' => flags |= (((next as i32 - b'0' as i32) as u32) & 0xf) << 8,
                _ => {} // Ignore unknown flags
            }
            i += 1;
        }
    }

    // `type_str` was written by the `"SC"` fetch above as an interned
    // parse-tree string — the address-only key `get_prop_type` looks up.
    prop.set_type(get_prop_type(uc, type_str));
    if prop.type_() == PropType::Unknown && !subtype_str.is_null() {
        // `subtype_str`: as above, from the `'C'` fetch.
        prop.set_type(get_prop_type(uc, subtype_str));
    }

    if let Some(got) = get_val_at::<i64>(node, val_ix as usize) {
        prop.set_value_int(got);
        flags |= PropFlags::VALUE_INT.raw();
    }

    // C-parity: `prop->value_real_arr[]` is the `ufbx_prop` value union's
    // 4-real view (ufbx.h); the generated struct keeps only the `value_vec4`
    // member, so the array view is reached by pointer cast.
    let value_real_arr: *mut Real = prop.value_vec4_raw() as *mut Real;
    let mut real_ix: usize = 0;
    while real_ix < 4 {
        if let Some(got) = get_val_at::<AsReal>(node, (val_ix as usize).wrapping_add(real_ix)) {
            // SAFETY: `real_ix < 4` keeps `value_real_arr.add(real_ix)` inside the
            // 4-`Real` value union arm.
            unsafe {
                *value_real_arr.add(real_ix) = got.0;
            }
        } else {
            break;
        }
        real_ix += 1;
    }
    if real_ix > 0 {
        flags |= PropFlags::VALUE_REAL.raw() << (real_ix - 1);
    }

    // Skip one value forward in case the current value is not a string, as some properties
    // contain mixed numbers and strings. Currenltly known cases:
    //   Lod Distance:    P: "Thresholds|Level0", "Distance", "", "",64, "cm"
    //   User Enum:       P: "User_Enum", "Enum", "", "A+U",1, "ValueA~ValueB~ValueC"
    if get_val_type(node, val_ix as usize) != ValueType::String {
        val_ix = val_ix.wrapping_add(1);
    }

    if let Some(got) = get_val_at::<Checked<String>>(node, val_ix as usize) {
        prop.set_value_str(got.0);
        if prop.value_str().length > 0 {
            if let Some(got) = get_val_at::<Blob>(node, val_ix as usize) {
                prop.set_value_blob(got);
            }
        }
        flags |= PropFlags::VALUE_STR.raw();
    } else {
        prop.set_value_str(EMPTY_STRING.0);
    }

    // Very unlikely, seems to only exist in some "non standard" FBX files
    if node.num_children() > 0 {
        let binary = find_child(node, sp::BinaryData.as_ptr());
        read_embedded_blob(uc, prop.value_blob_view(), binary)?;
        flags |= PropFlags::VALUE_BLOB.raw();
    }

    prop.set_flags(PropFlags::from_raw(flags));

    Ok(())
}

// ufbx.c:11871-11876 `ufbxi_prop_less`
// Comparator over views: the sort adapter mints them (PORTING.md "Sorting").
#[inline(always)]
pub(crate) fn prop_less<M: Mode>(a: &View<Prop, M>, b: &View<Prop, M>) -> bool {
    if a._internal_key() < b._internal_key() {
        return true;
    }
    if a._internal_key() > b._internal_key() {
        return false;
    }
    // C: `strcmp(a->name.data, b->name.data)` over interned (NUL-terminated)
    // names — `c_strcmp` stops at the first NUL like `strcmp`.
    let cmp: i32 = c_strcmp(a.name_view().bytes(), b.name_view().bytes());
    cmp < 0
}

// ufbx.c:11878-11883 `ufbxi_sort_properties`
#[inline(never)]
pub(crate) fn sort_properties(uc: &Context, props: Run<'_, Prop>) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the allocator, data pointer and size slots are uc's own
        // `ator_tmp`/`tmp_arr`/`tmp_arr_size` fields, reached through its
        // views — the matched triple `grow_array` requires.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_view(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                props.len().wrapping_mul(size_of::<Prop>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_prop)))"
    );
    // SAFETY: `props` carries a live `Prop` run and `uc.tmp_arr()` was just
    // grown to the run's byte size, so both the input run and merge buffer are
    // in bounds; the comparator only ever sees elements of that run.
    unsafe {
        macro_stable_sort_views::<Prop>(
            32,
            props.as_mut_ptr(),
            uc.tmp_arr() as *mut Prop,
            props.len(),
            prop_less,
        )
    };
    Ok(())
}

// ufbx.c:11885-11901 `ufbxi_deduplicate_properties`
#[inline(never)]
pub(crate) fn deduplicate_properties(list: &ListView<Prop>) {
    if list.count() >= 2 {
        // C: `ufbx_prop *ps = list->data;` — the run is reached through the
        // list view's own bounds-checked element accessor instead.
        let mut dst: usize = 0;
        let mut src: usize = 0;
        let end: usize = list.count();
        while src < end {
            if src + 1 < end
                && list.at(src).name_view().data() == list.at(src + 1).name_view().data()
            {
                src += 1;
            } else if dst != src {
                // SAFETY: C `ps[dst] = ps[src]` — a whole-`Prop` copy between
                // two live elements of this list's own run (`at` bounds-checks
                // both against `count`), non-overlapping because `dst != src`;
                // `Prop` is a `Copy` POD, and the destination pointer comes
                // from a `Mut` element view, so it is write-capable.
                unsafe {
                    core::ptr::copy_nonoverlapping(list.at(src).as_ptr(), list.at(dst).get(), 1)
                };
                dst += 1;
                src += 1;
            } else {
                dst += 1;
                src += 1;
            }
        }
        list.set_count(dst);
    }
}

// ufbx.c:11903-11932 `ufbxi_read_properties`
#[inline(never)]
pub(crate) fn read_properties(
    uc: &Context,
    parent: &NodeView,
    props: &PropsView,
) -> Result<(), Fail> {
    props.set_defaults(None);

    let mut version: i32 = 70;
    let mut node: Option<&NodeView> = find_child(parent, sp::Properties70.as_ptr());
    if node.is_none() {
        node = find_child(parent, sp::Properties60.as_ptr());
        if node.is_none() {
            // No properties found, not an error
            // SAFETY: `props_raw()` addresses the viewed table's own live
            // `ufbx_prop_list` field.
            unsafe {
                (*props.props_raw()).data = core::ptr::null();
                (*props.props_raw()).count = 0;
            }
            return Ok(());
        }
        version = 60;
    }
    let node: &NodeView = node.unwrap();

    // SAFETY: `props_raw()` addresses the viewed table's own live
    // `ufbx_prop_list` field.
    unsafe {
        (*props.props_raw()).data = uc
            .result_view()
            .push_zero::<Prop>(node.num_children() as usize);
        (*props.props_raw()).count = node.num_children() as usize;
    }
    ufbxi_check!(uc, !props.props_data().is_null(), "props->props.data");

    let mut i: usize = 0;
    while i < props.props_count() {
        // SAFETY: `count` equals `node.num_children()`, so `i` indexes both a
        // live child of `node` and a live element of the zeroed `Prop` run
        // pushed above; `NodeView::from_ptr` mints a view over that child and
        // `PropView::from_ptr` one over that element — both write-capable
        // result-arena memory, stable for the call.
        let (child, prop): (&NodeView, &PropView) = unsafe {
            (
                NodeView::from_ptr(node.children().add(i)),
                PropView::from_ptr(props.props_data().add(i)),
            )
        };
        read_property(uc, child, prop, version)?;
        i += 1;
    }

    // SAFETY: `props.data` spans `props.count` live `Prop` values filled by the
    // loop above.
    let prop_run = unsafe { Run::from_raw_parts(props.props_data(), props.props_count()) };
    sort_properties(uc, prop_run)?;
    deduplicate_properties(props.props_view());

    Ok(())
}

// Rust-port infrastructure (not a ufbx.c section): the write surface
// `ufbxi_read_thumbnail` needs over the scene metadata's `ufbx_thumbnail` slot
// — the property table it fills (as a sub-view, which is what
// `read_properties` takes), the scalar leaves and the `data` blob.
pub(crate) type ThumbnailView = View<Thumbnail>;

impl ThumbnailView {
    #[inline(always)]
    pub(crate) fn props_view(&self) -> &PropsView {
        view_project!(self, props)
    }
    #[inline(always)]
    pub(crate) fn set_width(&self, width: u32) {
        view_write!(self, width, width)
    }
    #[inline(always)]
    pub(crate) fn set_height(&self, height: u32) {
        view_write!(self, height, height)
    }
    #[inline(always)]
    pub(crate) fn set_format(&self, format: ThumbnailFormat) {
        view_write!(self, format, format)
    }
    #[inline(always)]
    pub(crate) fn data_view(&self) -> &BlobView {
        view_project!(self, data)
    }
}

// ufbx.c:11934-11967 `ufbxi_read_thumbnail`
#[inline(never)]
pub(crate) fn read_thumbnail(
    uc: &Context,
    node: &NodeView,
    thumbnail: &ThumbnailView,
) -> Result<(), Fail> {
    read_properties(uc, node, thumbnail.props_view())?;

    let props: &PropsView = thumbnail.props_view();
    let custom_width: i64 = api_find_int_len(props, b"CustomWidth", 0);
    let custom_height: i64 = api_find_int_len(props, b"CustomHeight", 0);

    let format_node = find_child_strcmp(node, b"Format");
    // C: `format_node && ufbxi_get_val1(format_node, "I", &format)` — the
    // `&&` short-circuit becomes `and_then`.
    if let Some(format) = format_node.and_then(get_val1::<i32>) {
        // C-parity: C's guard is `format >= 0 && format + 1 < UFBX_THUMBNAIL_FORMAT_COUNT`,
        // a *signed* `int32_t` addition. At `format == INT32_MAX` that addition
        // is UB: clang's `nsw` folds the pair of comparisons to
        // `(unsigned)format < UFBX_THUMBNAIL_FORMAT_COUNT - 1`, i.e. the value
        // is rejected. Port the folded form. A `wrapping_add(1)` here would
        // instead yield `INT32_MIN`, pass `< 3`, and store a format; the
        // `-fwrapv` build's alternative (storing the raw out-of-range
        // `INT32_MIN` in `thumbnail->format`) is unrepresentable in the
        // generated `ThumbnailFormat` enum, so the optimized build is the
        // oracle here (same accepted-divergence class as the f64->i64
        // boundary, PORTING.md "Integer semantics").
        if format >= 0 && format < THUMBNAIL_FORMAT_COUNT - 1 {
            thumbnail.set_format(thumbnail_format_from_raw(format + 1));
        }
    }

    if let Some(size) = find_val1::<i32>(node, sp::Size.as_ptr()) {
        if size > 0 {
            thumbnail.set_width(size as u32);
            thumbnail.set_height(size as u32);
        } else if size < 0 && custom_width > 0 && custom_height > 0 {
            thumbnail.set_width(custom_width as u32);
            thumbnail.set_height(custom_height as u32);
        }
    }

    let data_arr: *mut ValueArray = find_array(node, sp::ImageData.as_ptr(), b'c');
    if !data_arr.is_null() {
        // SAFETY: `data_arr` is non-null (checked) and points at the node's own
        // retained array descriptor; its payload spans `size` readable bytes
        // and stays live and unwritten while the thumbnail references it.
        let data = unsafe { Blob::new_c((*data_arr).data as *const u8, (*data_arr).size) };
        thumbnail.data_view().set(data);
    }

    Ok(())
}

// C casts the guarded `int32_t` straight to `ufbx_thumbnail_format`; Rust needs
// an explicit mapping. The caller's guard (`format >= 0 &&
// format + 1 < UFBX_THUMBNAIL_FORMAT_COUNT`) leaves only 1 and 2.
#[inline(always)]
fn thumbnail_format_from_raw(raw: i32) -> ThumbnailFormat {
    match raw {
        1 => ThumbnailFormat::Rgb24,
        _ => ThumbnailFormat::Rgba32,
    }
}

// ufbx.c:11969-11979 `ufbxi_read_scene_info`
#[inline(never)]
pub(crate) fn read_scene_info(uc: &Context, node: &NodeView) -> Result<(), Fail> {
    read_properties(uc, node, uc.scene_view().metadata_view().props_view())?;

    let thumbnail = find_child(node, sp::Thumbnail.as_ptr());
    if let Some(thumbnail) = thumbnail {
        read_thumbnail(
            uc,
            thumbnail,
            uc.scene_view().metadata_view().thumbnail_view(),
        )?;
    }

    Ok(())
}

// ufbx.c:11981-12033 `ufbxi_read_header_extension`
#[inline(never)]
pub(crate) fn read_header_extension(uc: &Context) -> Result<(), Fail> {
    let mut has_tc_definition = false;
    let mut tc_definition: i32 = 0;
    let mut header_version: i32 = 0;

    // C: `ufbxi_node *child;` — the `None` `tmp_buf` selects uc's own temp buffer,
    // as in the C call; the C `for(;;) { ...; if (!child) break; ... }` loop reads
    // as a `while let` over the `None` end signal.
    while let Some(child) = parse_toplevel_child(uc, None)? {
        if child.name() == sp::Creator.as_ptr() {
            if let Some(got) = get_val1::<Checked<String>>(child) {
                // SAFETY: `creator_mut_ptr()` addresses uc's own metadata `creator` slot.
                unsafe {
                    *uc.scene_view().metadata_view().creator_mut_ptr() = got.0;
                }
            }
        }

        if uc.version() < 6000 && child.name() == sp::FBXVersion.as_ptr() {
            if let Some(version) = get_val1::<i32>(child) {
                if version > 0 && version < 6000 && (version as u32) > uc.version() {
                    uc.set_version(version as u32);
                }
            }
        }

        if child.name() == sp::FBXHeaderVersion.as_ptr() {
            if let Some(got) = get_val1::<i32>(child) {
                header_version = got;
            }
        }

        if child.name() == sp::OtherFlags.as_ptr() {
            if let Some(got) = find_val1::<i32>(child, sp::TCDefinition.as_ptr()) {
                tc_definition = got;
                has_tc_definition = true;
            }
        }

        if child.name() == sp::SceneInfo.as_ptr() {
            read_scene_info(uc, child)?;
        }
    }

    // FBX 8000 will change the KTime units and the new units are opt-in currently via `TCDefinition`.
    // `TCDefinition` seems be accounted in all versions, as long as `FBXHeaderVersion >= 1004`.
    // The old KTime units are specified as the value `127` and all other values seem to use the new definition.
    let mut use_v7_ktime = uc.version() < 8000;
    if header_version >= 1004 && has_tc_definition {
        use_v7_ktime = tc_definition == 127;
    }

    uc.set_ktime_sec(if use_v7_ktime { 46186158000 } else { 141120000 });
    uc.set_ktime_sec_double(uc.ktime_sec() as f64);

    Ok(())
}

// ufbx.c:12035-12082 `ufbxi_match_version_string`
//
// `p_version` holds one slot per `?` marker a pattern can carry (three); C
// indexes the caller's `uint32_t version[3]` the same way.
pub(crate) fn match_version_string(fmt: &[u8], str_: &[u8], p_version: &mut [u32; 3]) -> bool {
    let mut num_ix: usize = 0;
    let mut pos: usize = 0;
    // C-parity: an exhausted `fmt` slice reads as the NUL terminator, so this
    // walks the pattern exactly as C's `while (*fmt)` over a `const char *`.
    let mut fmt = fmt;
    while !fmt.is_empty() && fmt[0] != 0 {
        // C-parity: `char c = *fmt++;` / `char s = str.data[pos];` are `const
        // char *` dereferences — signed bytes on the oracle targets
        // (PORTING.md `char` value row).
        let c: i8 = fmt[0] as i8;
        fmt = &fmt[1..];
        if c >= b'a' as i8 && c <= b'z' as i8 {
            if pos >= str_.len() {
                return false;
            }
            let s: i8 = str_[pos] as i8;
            if s != c && s as i32 + (b'a' as i32 - b'A' as i32) != c as i32 {
                return false;
            }
            pos += 1;
        } else if c == b' ' as i8 {
            while pos < str_.len() {
                let s: i8 = str_[pos] as i8;
                if s != b' ' as i8 && s != b'\t' as i8 {
                    break;
                }
                pos += 1;
            }
        } else if c == b'-' as i8 {
            while pos < str_.len() {
                let s: i8 = str_[pos] as i8;
                if s == b'-' as i8 {
                    break;
                }
                pos += 1;
            }
            if pos >= str_.len() {
                return false;
            }
            pos += 1;
        } else if c == b'/' as i8
            || c == b'.' as i8
            || c == b'(' as i8
            || c == b')' as i8
            || c == b'_' as i8
        {
            if pos >= str_.len() {
                return false;
            }
            if str_[pos] as i8 != c {
                return false;
            }
            pos += 1;
        } else if c == b'?' as i8 {
            let mut num: u32 = 0;
            let mut len: usize = 0;
            while pos < str_.len() {
                let s: i8 = str_[pos] as i8;
                if !(s >= b'0' as i8 && s <= b'9' as i8) {
                    break;
                }
                num = num
                    .wrapping_mul(10)
                    .wrapping_add((s as i32 - b'0' as i32) as u32);
                pos += 1;
                len += 1;
            }
            if len == 0 {
                return false;
            }
            p_version[num_ix] = num;
            num_ix += 1;
        } else {
            ufbxi_unreachable!("Unhandled match character");
        }
    }

    true
}

// ufbx.c:12084-12128 `ufbxi_match_exporter`
#[inline(never)]
pub(crate) fn match_exporter(uc: &Context) -> Result<(), Fail> {
    let creator_bytes: &[u8] = uc.scene_view().metadata_view().creator_view().bytes();
    let mut version: [u32; 3] = [0; 3];
    if match_version_string(b"blender-- ?.?.?\0", creator_bytes, &mut version) {
        uc.set_exporter(Exporter::BlenderBinary);
        uc.set_exporter_version(pack_version(version[0], version[1], version[2]));
    } else if match_version_string(b"blender- ?.?\0", creator_bytes, &mut version) {
        uc.set_exporter(Exporter::BlenderBinary);
        uc.set_exporter_version(pack_version(version[0], version[1], 0));
    } else if match_version_string(b"blender version ?.?\0", creator_bytes, &mut version) {
        uc.set_exporter(Exporter::BlenderAscii);
        uc.set_exporter_version(pack_version(version[0], version[1], 0));
    } else if match_version_string(
        b"fbx sdk/fbx plugins version ?.?\0",
        creator_bytes,
        &mut version,
    ) {
        uc.set_exporter(Exporter::FbxSdk);
        uc.set_exporter_version(pack_version(version[0], version[1], 0));
    } else if match_version_string(
        b"fbx sdk/fbx plugins build ?\0",
        creator_bytes,
        &mut version,
    ) {
        uc.set_exporter(Exporter::FbxSdk);
        uc.set_exporter_version(pack_version(
            version[0] / 10000u32,
            version[0] / 100u32 % 100u32,
            version[0] % 100u32,
        ));
    } else if match_version_string(b"motionbuilder version ?.?\0", creator_bytes, &mut version) {
        uc.set_exporter(Exporter::MotionBuilder);
        uc.set_exporter_version(pack_version(version[0], version[1], 0));
    } else if match_version_string(
        b"motionbuilder/mocap/online version ?.?\0",
        creator_bytes,
        &mut version,
    ) {
        uc.set_exporter(Exporter::MotionBuilder);
        uc.set_exporter_version(pack_version(version[0], version[1], 0));
    } else if match_version_string(b"ufbx_write\0", creator_bytes, &mut version) {
        uc.set_exporter(Exporter::UfbxWrite);
        uc.set_exporter_version(pack_version(0, 0, 1));
    }

    uc.scene_view().metadata_view().set_exporter(uc.exporter());
    uc.scene_view()
        .metadata_view()
        .set_exporter_version(uc.exporter_version());

    // Un-detect the exporter in `ufbxi_context` to disable special cases
    if uc.opts_view().disable_quirks() {
        uc.set_exporter(Exporter::Unknown);
        uc.set_exporter_version(0);
    }

    if uc.exporter() == Exporter::BlenderBinary {
        uc.set_blender_full_weights(true);
    }

    Ok(())
}

// ufbx.c:12130-12149 `ufbxi_read_document`
#[inline(never)]
pub(crate) fn read_document(uc: &Context) -> Result<(), Fail> {
    let mut found_root_id = false;

    // C: `ufbxi_node *child;` — the `None` `tmp_buf` selects uc's own temp buffer,
    // as in the C call; the C `for(;;) { ...; if (!child) break; ... }` loop reads
    // as a `while let` over the `None` end signal.
    while let Some(child) = parse_toplevel_child(uc, None)? {
        if child.name() == sp::Document.as_ptr() && !found_root_id {
            if let Some(got) = find_val1::<i64>(child, sp::RootNode.as_ptr()) {
                // SAFETY: `root_id_mut_ptr()` addresses uc's own `root_id` field.
                unsafe {
                    *uc.root_id_mut_ptr() = got as u64;
                }
                found_root_id = true;
            }
        }
    }

    Ok(())
}

// C string literal assigned to `tmpl->sub_type.data` at ufbx.c:12178 — the
// pointer must outlive the load, so it is a static here too.
static LOD_GROUP: [u8; b"LodGroup\0".len()] = *b"LodGroup\0";

// ufbx.c:12151-12193 `ufbxi_read_definitions`
#[inline(never)]
pub(crate) fn read_definitions(uc: &Context) -> Result<(), Fail> {
    // C: `ufbxi_node *object;` — the `None` `tmp_buf` selects uc's own temp buffer,
    // as in the C call; the C `for(;;) { ...; if (!object) break; ... }` loop reads
    // as a `while let` over the `None` end signal.
    while let Some(object) = parse_toplevel_child(uc, None)? {
        if object.name() != sp::ObjectType.as_ptr() {
            continue;
        }

        let tmpl: *mut Template = uc.tmp_stack_view().push_zero::<Template>(1);
        uc.set_num_templates(uc.num_templates().wrapping_add(1));
        ufbxi_check!(uc, !tmpl.is_null(), "tmpl");
        // SAFETY: `tmpl` is the fresh non-null push result above; the fetch
        // yields the value, so the only raw op is the write into its own field.
        unsafe {
            (*tmpl).type_ = ufbxi_check_some!(
                uc,
                get_val1::<Checked<*const u8>>(object),
                "ufbxi_get_val1(object, \"C\", (char**)&tmpl->type)"
            )
            .0;
        }

        // Pre-7000 FBX versions don't have property templates, they just have
        // the object counts by themselves.
        let props = find_child(object, sp::PropertyTemplate.as_ptr());
        if let Some(props) = props {
            // SAFETY: as above, for `tmpl`'s own `sub_type` field.
            unsafe {
                (*tmpl).sub_type = ufbxi_check_some!(
                    uc,
                    get_val1::<Checked<String>>(props),
                    "ufbxi_get_val1(props, \"S\", &tmpl->sub_type)"
                )
                .0;
            }

            // Remove the "Fbx" prefix from sub-types, remember to re-intern!
            // SAFETY: `tmpl` is the fresh push result above; `sub_type` was
            // just filled by the `S` read, so it is a pooled NUL-terminated
            // string of `length` bytes and the compares/advance below stay
            // inside it (each is guarded by a length check). The re-intern
            // views that same live field in place.
            unsafe {
                if (*tmpl).sub_type.length > 3
                    && strncmp((*tmpl).sub_type.data, b"Fbx\0".as_ptr(), 3) == 0
                {
                    (*tmpl).sub_type.data = (*tmpl).sub_type.data.add(3);
                    (*tmpl).sub_type.length -= 3;

                    // HACK: LOD groups use LODGroup for Template, LodGroup for Object?
                    if (*tmpl).sub_type.length == 8
                        && memcmp(slice_from_ptr((*tmpl).sub_type.data, 8), b"LODGroup") == 0
                    {
                        (*tmpl).sub_type.data = LOD_GROUP.as_ptr();
                    }

                    let sub_type = StringView::from_ptr(&raw mut (*tmpl).sub_type);
                    push_string_place_str(uc.string_pool_view(), sub_type, false)?;
                }

                // SAFETY: `&raw mut (*tmpl).props` addresses the freshly pushed
                // template's own `ufbx_props` field in uc's tmp stack —
                // write-capable provenance, stable for this call.
                read_properties(uc, props, PropsView::from_ptr(&raw mut (*tmpl).props))?;
            }
        }
    }

    // TODO: Preserve only the `props` part of the templates
    // Pops the `num_templates` entries just pushed onto uc's tmp stack into
    // uc's own result buffer.
    uc.set_templates(
        uc.result_view()
            .push_pop::<Template>(uc.tmp_stack_view(), uc.num_templates()),
    );
    ufbxi_check!(uc, !uc.templates().is_null(), "uc->templates");

    Ok(())
}

// ufbx.c:12195-12218 `ufbxi_find_template`
// C-parity: `name` and `sub_type` are matched by POINTER IDENTITY against the
// interned `tmpl->type` / `tmpl->sub_type.data` runs, so each slice borrows the
// interned run itself; their lengths are never read.
#[must_use]
pub(crate) fn find_template(uc: &Context, name: &[u8], sub_type: &[u8]) -> *mut Props {
    // TODO: Binary search
    // C: `ufbxi_for(ufbxi_template, tmpl, uc->templates, uc->num_templates)`
    // SAFETY: `read_definitions` publishes `templates` as a contiguous,
    // initialized result-buffer run; its stored pointer is write-capable, which
    // the successful raw `Props` result must retain.
    let templates = unsafe { Run::from_raw_parts(uc.templates(), uc.num_templates()) };
    for tmpl in templates.iter() {
        if tmpl.type_() == name.as_ptr() {
            // Check that sub_type matches unless the type is Material, Model, AnimationStack, AnimationLayer.
            // Those match to all sub-types.
            if tmpl.type_() != sp::Material.as_ptr()
                && tmpl.type_() != sp::Model.as_ptr()
                && tmpl.type_() != sp::AnimationStack.as_ptr()
                && tmpl.type_() != sp::AnimationLayer.as_ptr()
            {
                if tmpl.sub_type_view().data() != sub_type.as_ptr() {
                    return core::ptr::null_mut();
                }
            }

            if tmpl.props_view().props_view().count() > 0 {
                return tmpl.props_raw();
            } else {
                return core::ptr::null_mut();
            }
        }
    }
    core::ptr::null_mut()
}

// Name ID categories
// ufbx.c:12220-12227
#[cfg(feature = "regression")]
pub(crate) const MAXIMUM_FAST_POINTER_ID: u64 = 0x100;
#[cfg(not(feature = "regression"))]
pub(crate) const MAXIMUM_FAST_POINTER_ID: u64 = 0x4000000000000000;
pub(crate) const POINTER_ID_START: u64 = 0x8000000000000000;
pub(crate) const SYNTHETIC_ID_START: u64 = POINTER_ID_START + MAXIMUM_FAST_POINTER_ID;

// ufbx.c:12229-12232 `ufbxi_push_synthetic_id`
#[inline(always)]
pub(crate) fn push_synthetic_id(uc: &Context) -> u64 {
    // C: `return ++uc->synthetic_id_counter;` — pre-increment yields the NEW value.
    uc.set_synthetic_id_counter(uc.synthetic_id_counter().wrapping_add(1));
    uc.synthetic_id_counter()
}

// ufbx.c:12234-12248 `ufbxi_synthetic_id_from_ptr_id`
#[inline(never)]
pub(crate) fn synthetic_id_from_ptr_id(uc: &Context, ptr: usize, id: u64) -> u64 {
    let ptr_id = PtrId { ptr, id };
    let hash = hash_ptr_id(ptr_id);
    // SAFETY: `ptr_fbx_id_map` stores `PtrFbxIdEntry` items keyed by a `PtrId`
    // value pair; `ptr_id` is a complete local key.
    let mut entry: *mut PtrFbxIdEntry = unsafe { uc.ptr_fbx_id_map_view().find(hash, &ptr_id) };

    if entry.is_null() {
        // SAFETY: as for the `find` above — same map, same key.
        entry = unsafe { uc.ptr_fbx_id_map_view().insert(hash, &ptr_id) };
        ufbxi_check_return!(uc, !entry.is_null(), 0, "entry");
        // SAFETY: `entry` is the fresh non-null insert result checked above.
        unsafe {
            (*entry).ptr_id = ptr_id;
            (*entry).fbx_id = push_synthetic_id(uc);
        }
    }

    // SAFETY: `entry` is a non-null entry of uc's own map — either found above
    // or freshly inserted and initialized.
    unsafe { (*entry).fbx_id }
}

// ufbx.c:12250-12258 `ufbxi_synthetic_id_from_string`
#[inline(always)]
// Safe fn: `str_` is used address-only (the interned pointer itself becomes the
// synthetic id, directly or via `synthetic_id_from_ptr_id`); no byte behind it
// is ever read here.
pub(crate) fn synthetic_id_from_string(uc: &Context, str_: *const u8) -> u64 {
    let uptr: usize = str_ as usize;
    // C: `UINTPTR_MAX < UFBXI_MAXIMUM_FAST_POINTER_ID ? UINTPTR_MAX : UFBXI_MAXIMUM_FAST_POINTER_ID`
    // — the ternary's common type is `uint64_t`, so the comparison happens in
    // 64 bits on every target.
    let limit: u64 = if (usize::MAX as u64) < MAXIMUM_FAST_POINTER_ID {
        usize::MAX as u64
    } else {
        MAXIMUM_FAST_POINTER_ID
    };
    if (uptr as u64) < limit {
        uptr as u64
    } else {
        synthetic_id_from_ptr_id(uc, uptr, 0)
    }
}

// ufbx.c:12260-12269 `ufbxi_validate_fbx_id`
#[inline(always)]
pub(crate) fn validate_fbx_id(uc: &Context, p_fbx_id: &mut u64) -> Result<(), Fail> {
    let mut fbx_id: u64 = *p_fbx_id;
    if fbx_id >= POINTER_ID_START {
        fbx_id = synthetic_id_from_ptr_id(uc, 0, fbx_id);
        ufbxi_check!(uc, fbx_id != 0, "fbx_id");
        *p_fbx_id = fbx_id;
    }
    Ok(())
}

// ufbx.c:12271-12305 `ufbxi_split_type_and_name`
// The input view anchors the pooled string's readable byte run while the two
// output locals receive re-interned slices of it.
#[inline(never)]
pub(crate) fn split_type_and_name<M: Mode>(
    uc: &Context,
    type_and_name: &View<String, M>,
    type_: &mut String,
    name: &mut String,
) -> Result<(), Fail> {
    // Name and type are packed in a single property as Type::Name (in ASCII)
    // or Name\x00\x01Type (in binary)
    let sep: &[u8; 2] = if uc.from_ascii() { b"::" } else { b"\x00\x01" };
    let type_and_name_bytes = type_and_name.bytes();
    let mut type_end: usize = 2;
    while type_end <= type_and_name_bytes.len() {
        let ch = &type_and_name_bytes[type_end - 2..type_end];
        if ch[0] == sep[0] && ch[1] == sep[1] {
            break;
        }
        type_end += 1;
    }

    // ???: ASCII and binary store type and name in different order
    if type_end <= type_and_name_bytes.len() {
        if uc.from_ascii() {
            name.data = type_and_name.data().wrapping_add(type_end);
            name.length = type_and_name_bytes.len() - type_end;
            type_.data = type_and_name.data();
            type_.length = type_end - 2;
        } else {
            name.data = type_and_name.data();
            name.length = type_end - 2;
            type_.data = type_and_name.data().wrapping_add(type_end);
            type_.length = type_and_name_bytes.len() - type_end;
        }
    } else {
        // Preserve the STORED pointer for empty strings: `bytes().as_ptr()` may
        // be the canonical dangling pointer for a zero-length run.
        *name = String::new_c(type_and_name.data(), type_and_name.length());
        type_.data = EMPTY_CHAR.as_ptr();
        type_.length = 0;
    }

    // `type_`/`name` are exclusive borrows of live `ufbx_string` slots, each
    // holding a data/length pair written above.
    push_string_place_str(uc.string_pool_view(), StringView::from_mut(type_), false)?;
    push_string_place_str(uc.string_pool_view(), StringView::from_mut(name), false)?;

    Ok(())
}

// ufbx.c:12307-12323 `ufbxi_insert_fbx_id`
#[inline(never)]
pub(crate) fn insert_fbx_id(uc: &Context, fbx_id: u64, element_id: u32) -> Result<(), Fail> {
    let hash = hash64(fbx_id);
    // SAFETY: `fbx_id_map` stores `FbxIdEntry` items keyed by a `u64` value.
    let mut entry: *mut FbxIdEntry = unsafe { uc.fbx_id_map_view().find(hash, &fbx_id) };

    if entry.is_null() {
        // SAFETY: as for the `find` above — same map, same key.
        entry = unsafe { uc.fbx_id_map_view().insert(hash, &fbx_id) };
        ufbxi_check!(uc, !entry.is_null(), "entry");
        // SAFETY: `entry` is the fresh non-null insert result checked above.
        unsafe {
            (*entry).fbx_id = fbx_id;
            (*entry).element_id = element_id;
            (*entry).user_id = 0;
        }
    } else {
        ufbxi_check!(
            uc,
            ufbxi_warnf!(uc, WarningType::DuplicateObjectId, "Duplicate object ID").is_ok(),
            "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_DUPLICATE_OBJECT_ID, ~0u, \"Duplicate object ID\")"
        );
    }

    Ok(())
}

// ufbx.c:12325-12329 `ufbxi_find_fbx_id`
#[inline(never)]
pub(crate) fn find_fbx_id(uc: &Context, fbx_id: u64) -> *mut FbxIdEntry {
    let hash = hash64(fbx_id);
    // SAFETY: `fbx_id_map` stores `FbxIdEntry` items keyed by a `u64` value.
    unsafe { uc.fbx_id_map_view().find(hash, &fbx_id) }
}

// ufbx.c:12331-12334 `ufbxi_fbx_id_exists`
#[inline(always)]
pub(crate) fn fbx_id_exists(uc: &Context, fbx_id: u64) -> bool {
    !find_fbx_id(uc, fbx_id).is_null()
}

// ufbx.c:12336-12350 `ufbxi_insert_fbx_attr`
#[inline(never)]
pub(crate) fn insert_fbx_attr(uc: &Context, fbx_id: u64, attrib_fbx_id: u64) -> Result<(), Fail> {
    let hash = hash64(fbx_id);
    // SAFETY: `fbx_attr_map` stores `FbxAttrEntry` items keyed by a `u64` value.
    let mut entry: *mut FbxAttrEntry = unsafe { uc.fbx_attr_map_view().find(hash, &fbx_id) };
    // TODO: Strict / warn about duplicate objects

    if entry.is_null() {
        // SAFETY: as for the `find` above — same map, same key.
        entry = unsafe { uc.fbx_attr_map_view().insert(hash, &fbx_id) };
        ufbxi_check!(uc, !entry.is_null(), "entry");
        // SAFETY: `entry` is the fresh non-null insert result checked above.
        unsafe {
            (*entry).node_fbx_id = fbx_id;
            (*entry).attr_fbx_id = attrib_fbx_id;
        }
    }

    Ok(())
}

// C assigns a possibly-null pointer into a field the generated bindings type as
// `Option<Ref<T>>`; `Ref<T>` is `#[repr(transparent)]` over `NonNull<T>`, so the
// option is niche-packed to a bare pointer and NULL is `None`.
#[inline(always)]
pub(crate) unsafe fn opt_ref<T>(ptr: *mut T) -> Option<Ref<T>> {
    if ptr.is_null() {
        None
    } else {
        // SAFETY: `ptr` is non-null (checked) and the caller vouches that it
        // addresses a live, stable, unmoved `T` with write-capable provenance
        // for as long as the returned `Ref` or its containing struct exists.
        Some(unsafe { Ref::from_ptr(ptr) })
    }
}

// Reads a non-optional `Ref<T>` field (C: a plain `ufbx_element*`) back as the
// bare pointer it is at the ABI level, WITHOUT asserting the `NonNull`
// invariant — for the slots ufbx leaves NULL despite the non-nullable public
// type: the sentinel-terminated `anim_props` run's `element`, `scene.anim`
// after an unchecked default-anim push, and `element.scene` on a standalone
// tessellated mesh. Following a valid `Ref<T>` is `Ref::view`.
#[inline(always)]
pub(crate) unsafe fn ref_ptr<T>(p: *const Ref<T>) -> *mut T {
    // SAFETY: `p` addresses a live `Ref<T>` field (fn contract), which is
    // `repr(transparent)` over `NonNull<T>`, so reading it as `*mut T`
    // reinterprets the same bytes in place.
    unsafe { *(p as *const *mut T) }
}

// ufbx.c:12352-12382 `ufbxi_push_element_size`
///
/// # Safety
/// - `info` addresses a live, initialized `ufbxi_element_info`.
/// - `size` is the size of the element struct C's `type_name` names for
///   `type_`. The element allocation is sized from `size` ALONE
///   (`aligned_size`, ufbx.c:12354) while the header records `type_`
///   (ufbx.c:12366), and finalize later casts that allocation to the struct
///   `type_` names and writes through it — a `size` smaller than that struct
///   turns those writes into out-of-bounds writes.
/// - `info->name`, `info->props` and `info->dom_node` are stored into the
///   element BY POINTER (ufbx.c:12369-12371) and are read for as long as the
///   scene lives, `name.data` past `name.length` (`strcmp` in
///   `ufbxi_cmp_name_element_less`, ufbx.c:18559, and every C-ABI reader of the
///   public `ufbx_string`). So `name.data` must be NUL-terminated and, with
///   `props` and `dom_node`, must stay live and unmoved for the scene's
///   lifetime — pooled strings and uc-owned buffers.
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_element_size(
    uc: &Context,
    info: *mut ElementInfo,
    size: usize,
    type_: ElementType,
) -> *mut Element {
    // C-parity: `~0x7u` in `(size + 7u) & ~0x7u` is an `unsigned int`
    // (0xFFFFFFF8) zero-extended to `size_t`, so the mask also clears the upper
    // 32 bits. Unreachable in practice (`size` is always a `sizeof`), ported
    // verbatim anyway.
    let aligned_size: usize = size.wrapping_add(7) & (!0x7u32 as usize);

    let typed_id: u32 = uc.tmp_typed_element_offsets_at(type_ as usize).num_items() as u32;
    // C: `uint32_t element_id = uc->num_elements++;` — post-increment yields
    // the OLD value.
    let element_id: u32 = uc.num_elements();
    uc.set_num_elements(uc.num_elements().wrapping_add(1));

    ufbxi_check_return!(
        uc,
        !uc.tmp_typed_element_offsets_at(type_ as usize)
            .push_copy_fast_ref(&uc.tmp_element_byte_offset())
            .is_null(),
        core::ptr::null_mut(),
        "((size_t*)ufbxi_push_size_copy_fast((&uc->tmp_typed_element_offsets[type]), sizeof(size_t), (1), (&uc->tmp_element_byte_offset)))"
    );
    ufbxi_check_return!(
        uc,
        !uc.tmp_element_offsets_view()
            .push_copy_fast_ref(&uc.tmp_element_byte_offset())
            .is_null(),
        core::ptr::null_mut(),
        "((size_t*)ufbxi_push_size_copy_fast((&uc->tmp_element_offsets), sizeof(size_t), (1), (&uc->tmp_element_byte_offset)))"
    );
    ufbxi_check_return!(
        uc,
        // SAFETY: `info` is the caller's live `ufbxi_element_info` (fn
        // contract), so `&(*info).fbx_id` borrows its `fbx_id` field.
        !unsafe { uc.tmp_element_fbx_ids_view().push_copy_fast_ref(&(*info).fbx_id) }.is_null(),
        core::ptr::null_mut(),
        "((uint64_t*)ufbxi_push_size_copy_fast((&uc->tmp_element_fbx_ids), sizeof(uint64_t), (1), (&info->fbx_id)))"
    );
    uc.set_tmp_element_byte_offset(uc.tmp_element_byte_offset().wrapping_add(aligned_size));

    let elem: *mut Element =
        uc.tmp_elements_view().push_zero::<u64>(aligned_size / 8) as *mut Element;
    ufbxi_check_return!(uc, !elem.is_null(), core::ptr::null_mut(), "elem");
    // SAFETY: `elem` is the fresh non-null zeroed push result checked above,
    // sized to hold an `ufbx_element` header; `info` is the caller's live
    // `ufbxi_element_info`.
    unsafe {
        (*elem).type_ = type_;
        (*elem).element_id = element_id;
        (*elem).typed_id = typed_id;
        (*elem).name = (*info).name;
    }
    // C: `elem->props = info->props;` is a struct memcpy; `Props` is not `Copy`
    // (it holds an `Option<Ref<Props>>`) but has no drop glue.
    // SAFETY: both projections address live, properly aligned `Props` fields —
    // `elem`'s zeroed header and `info`'s initialized one; `Props` has no drop
    // glue, so the bitwise read/write duplicates it without double-free.
    unsafe {
        core::ptr::write(
            &raw mut (*elem).props,
            core::ptr::read(&raw const (*info).props),
        )
    };
    // SAFETY: `info.dom_node` is either null or a live DOM node owned by uc's
    // buffers — `opt_ref`'s contract; `elem` is the fresh push result.
    unsafe { (*elem).dom_node = opt_ref((*info).dom_node) };

    if !uc.p_element_id().is_null() {
        // SAFETY: `p_element_id` is non-null (checked) and points at the
        // caller-supplied `uint32_t` slot uc was threaded with.
        unsafe { *uc.p_element_id() = element_id };
    }

    ufbxi_check_return!(
        uc,
        !uc.tmp_element_ptrs_view().push_copy_fast_ref(&elem).is_null(),
        core::ptr::null_mut(),
        "((ufbx_element**)ufbxi_push_size_copy_fast((&uc->tmp_element_ptrs), sizeof(ufbx_element*), (1), (&elem)))"
    );

    ufbxi_check_return!(
        uc,
        // SAFETY: `info` is the caller's live `ufbxi_element_info`.
        insert_fbx_id(uc, unsafe { (*info).fbx_id }, element_id).is_ok(),
        core::ptr::null_mut(),
        "ufbxi_insert_fbx_id(uc, info->fbx_id, element_id)"
    );

    elem
}

// ufbx.c:12384-12414 `ufbxi_push_synthetic_element_size`
///
/// # Safety
/// `name` stays a raw pointer: it is null or NUL-terminated, and the pointer
/// ITSELF is stored in `elem->name.data`, whose bytes are read past
/// `name.length` — at minimum the terminator, by `ufbxi_cmp_name_element_less`
/// (`strcmp`, ufbx.c:18559) and by every C-ABI reader of the public
/// NUL-terminated `ufbx_string`. A `&[u8]` parameter would narrow the escaping
/// pointer's provenance to the `strlen` run and could not carry the
/// scene-long lifetime either: the bytes must stay live and unmoved for as
/// long as the scene. `size` must be the size of the element struct `type_`
/// selects (the `ufbxi_push_synthetic_element` macro's `sizeof(type_name)`).
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_synthetic_element_size(
    uc: &Context,
    p_fbx_id: &ScalarView<u64>,
    node: Option<&NodeView>,
    name: *const u8,
    size: usize,
    type_: ElementType,
) -> *mut Element {
    // C-parity: `~0x7u` in `(size + 7u) & ~0x7u` is an `unsigned int`
    // (0xFFFFFFF8) zero-extended to `size_t`, so the mask also clears the upper
    // 32 bits. Unreachable in practice (`size` is always a `sizeof`), ported
    // verbatim anyway.
    let aligned_size: usize = size.wrapping_add(7) & (!0x7u32 as usize);

    let typed_id: u32 = uc.tmp_typed_element_offsets_at(type_ as usize).num_items() as u32;
    let element_id: u32 = uc.num_elements();
    uc.set_num_elements(uc.num_elements().wrapping_add(1));

    ufbxi_check_return!(
        uc,
        !uc.tmp_typed_element_offsets_at(type_ as usize)
            .push_copy_fast_ref(&uc.tmp_element_byte_offset())
            .is_null(),
        core::ptr::null_mut(),
        "((size_t*)ufbxi_push_size_copy_fast((&uc->tmp_typed_element_offsets[type]), sizeof(size_t), (1), (&uc->tmp_element_byte_offset)))"
    );
    ufbxi_check_return!(
        uc,
        !uc.tmp_element_offsets_view()
            .push_copy_fast_ref(&uc.tmp_element_byte_offset())
            .is_null(),
        core::ptr::null_mut(),
        "((size_t*)ufbxi_push_size_copy_fast((&uc->tmp_element_offsets), sizeof(size_t), (1), (&uc->tmp_element_byte_offset)))"
    );
    uc.set_tmp_element_byte_offset(uc.tmp_element_byte_offset().wrapping_add(aligned_size));

    let elem: *mut Element =
        uc.tmp_elements_view().push_zero::<u64>(aligned_size / 8) as *mut Element;
    ufbxi_check_return!(uc, !elem.is_null(), core::ptr::null_mut(), "elem");
    // SAFETY: `elem` is the fresh non-null zeroed push result checked above,
    // sized to hold an `ufbx_element` header.
    unsafe {
        (*elem).type_ = type_;
        (*elem).element_id = element_id;
        (*elem).typed_id = typed_id;
    }
    // SAFETY: `get_dom_node` returns either null or a DOM node owned by uc's
    // buffers — `opt_ref`'s contract; `elem` is the fresh push result.
    unsafe { (*elem).dom_node = opt_ref(get_dom_node(uc, node)) };
    if !name.is_null() {
        // SAFETY: `name` is non-null (checked) and NUL-terminated (fn
        // contract), which is what `strlen` requires; the pointer stored keeps
        // its original whole-object provenance, as C's
        // `elem->name.data = name` does; `elem` is the fresh push result.
        unsafe {
            (*elem).name.data = name;
            (*elem).name.length = strlen(name);
        }
    }

    ufbxi_check_return!(
        uc,
        !uc.tmp_element_ptrs_view().push_copy_fast_ref(&elem).is_null(),
        core::ptr::null_mut(),
        "((ufbx_element**)ufbxi_push_size_copy_fast((&uc->tmp_element_ptrs), sizeof(ufbx_element*), (1), (&elem)))"
    );

    p_fbx_id.set(push_synthetic_id(uc));

    ufbxi_check_return!(
        uc,
        // SAFETY: the buffer is uc's own `tmp_element_fbx_ids` and the source
        // run is `p_fbx_id`'s own cell — the one `u64` copied from it.
        !unsafe {
            uc.tmp_element_fbx_ids_view()
                .push_copy_fast_raw::<u64>(1, p_fbx_id.as_ptr())
        }
        .is_null(),
        core::ptr::null_mut(),
        "((uint64_t*)ufbxi_push_size_copy_fast((&uc->tmp_element_fbx_ids), sizeof(uint64_t), (1), (p_fbx_id)))"
    );
    ufbxi_check_return!(
        uc,
        insert_fbx_id(uc, p_fbx_id.get(), element_id).is_ok(),
        core::ptr::null_mut(),
        "ufbxi_insert_fbx_id(uc, *p_fbx_id, element_id)"
    );

    elem
}

// ufbx.c:12416 `#define ufbxi_push_element(uc, info, type_name, type_enum)`
///
/// # Safety
/// The two obligations `push_element_size` documents, with `size` fixed to
/// `size_of::<T>()`: `T` must be the element struct C's `type_name` names for
/// `type_enum`, and the strings/pointers `info` carries must outlive the scene.
/// The view discharges only `info`'s own liveness, not what it points at.
#[inline(always)]
#[must_use]
pub(crate) unsafe fn push_element<T>(
    uc: &Context,
    info: &View<ElementInfo, Mut>,
    type_enum: ElementType,
) -> *mut T {
    // SAFETY: `info` views a live `ufbxi_element_info` (its mint invariant);
    // `size_of::<T>()` is the size of the element struct for `type_enum` and
    // `info`'s escaping pointers outlive the scene — this fn's own contract,
    // forwarded to `push_element_size`.
    ufbxi_maybe_null!(
        unsafe { push_element_size(uc, info.get(), size_of::<T>(), type_enum) } as *mut T
    )
}

// ufbx.c:12417 `#define ufbxi_push_synthetic_element(uc, p_fbx_id, node, name, type_name, type_enum)`
///
/// # Safety
/// `name` stays a raw pointer for the reason `push_synthetic_element_size`
/// documents: it is null or NUL-terminated, the pointer ITSELF is stored in
/// `elem->name.data`, and its bytes must stay live and unmoved for as long as
/// the scene.
#[inline(always)]
#[must_use]
pub(crate) unsafe fn push_synthetic_element<T>(
    uc: &Context,
    p_fbx_id: &ScalarView<u64>,
    node: Option<&NodeView>,
    name: *const u8,
    type_enum: ElementType,
) -> *mut T {
    // SAFETY: `name` is null or NUL-terminated and its bytes are the caller's
    // interned/pooled string, live for the scene — the pointer is passed
    // through unnarrowed because the element stores it; `T` is the element
    // struct whose `size_of` is passed as the element size —
    // `push_synthetic_element_size`'s contract.
    ufbxi_maybe_null!(unsafe {
        push_synthetic_element_size(uc, p_fbx_id, node, name, size_of::<T>(), type_enum)
    } as *mut T)
}

// ufbx.c:12419-12427 `ufbxi_connect_oo`
#[inline(never)]
pub(crate) fn connect_oo(uc: &Context, src: u64, dst: u64) -> Result<(), Fail> {
    let conn: *mut TmpConnection = uc.tmp_connections_view().push::<TmpConnection>(1);
    ufbxi_check!(uc, !conn.is_null(), "conn");
    // SAFETY: `conn` is the fresh non-null push result checked above; the
    // fields are fully initialized here, and `ufbx_empty_string` is static.
    unsafe {
        (*conn).src = src;
        (*conn).dst = dst;
        // C: `conn->src_prop = conn->dst_prop = ufbx_empty_string;`
        (*conn).dst_prop = EMPTY_STRING.0;
        (*conn).src_prop = (*conn).dst_prop;
    }
    Ok(())
}

// ufbx.c:12429-12438 `ufbxi_connect_op`
#[inline(never)]
pub(crate) unsafe fn connect_op(
    uc: &Context,
    src: u64,
    dst: u64,
    prop: String,
) -> Result<(), Fail> {
    let conn: *mut TmpConnection = uc.tmp_connections_view().push::<TmpConnection>(1);
    ufbxi_check!(uc, !conn.is_null(), "conn");
    // SAFETY: `conn` is the fresh non-null push result checked above; every
    // field is initialized here before any read.
    unsafe {
        (*conn).src = src;
        (*conn).dst = dst;
        (*conn).src_prop = EMPTY_STRING.0;
        (*conn).dst_prop = prop;
    }
    Ok(())
}

// ufbx.c:12440-12449 `ufbxi_connect_pp`
#[inline(never)]
pub(crate) unsafe fn connect_pp(
    uc: &Context,
    src: u64,
    dst: u64,
    src_prop: String,
    dst_prop: String,
) -> Result<(), Fail> {
    let conn: *mut TmpConnection = uc.tmp_connections_view().push::<TmpConnection>(1);
    ufbxi_check!(uc, !conn.is_null(), "conn");
    // SAFETY: `conn` is the fresh non-null push result checked above; every
    // field is initialized here before any read.
    unsafe {
        (*conn).src = src;
        (*conn).dst = dst;
        (*conn).src_prop = src_prop;
        (*conn).dst_prop = dst_prop;
    }
    Ok(())
}

// Synthetic property names are interned static byte runs: the stored pointer
// carries their identity and remains live with the resulting scene property.
// Their first in-run NUL supplies C's `strlen` result; four preceding bytes are
// required for the property key.
#[inline(always)]
fn synthetic_name_length(name: &'static [u8]) -> usize {
    name.iter()
        .position(|&c| c == b'\0')
        .expect("synthetic property name must contain NUL")
}

// ufbx.c:12451-12463 `ufbxi_init_synthetic_int_prop`
#[inline(never)]
pub(crate) fn init_synthetic_int_prop(
    dst: &mut Prop,
    name: &'static [u8],
    value: i64,
    type_: PropType,
) {
    dst.type_ = type_;
    dst.name.data = name.as_ptr();
    dst.name.length = synthetic_name_length(name);
    // C-parity: `dst->value_real` is the `ufbx_prop` value union's first real
    // (`value_vec4.x` in the generated struct).
    dst.value_vec4.x = value as Real;
    dst.flags = PropFlags::from_raw(
        PropFlags::SYNTHETIC.raw() | PropFlags::VALUE_REAL.raw() | PropFlags::VALUE_INT.raw(),
    );
    dst.value_int = value;
    dst.value_str.data = EMPTY_CHAR.as_ptr();

    assert!(
        dst.name.length >= 4,
        "synthetic property name must be at least four bytes"
    );
    dst._internal_key = get_name_key(&name[..4]);
}

// ufbx.c:12465-12477 `ufbxi_init_synthetic_real_prop`
#[inline(never)]
pub(crate) fn init_synthetic_real_prop(
    dst: &mut Prop,
    name: &'static [u8],
    value: Real,
    type_: PropType,
) {
    dst.type_ = type_;
    dst.name.data = name.as_ptr();
    dst.name.length = synthetic_name_length(name);
    // C-parity: bare `(int64_t)` cast on a float operand — `as` (saturating),
    // per PORTING.md "Integer semantics".
    dst.value_vec4.x = value;
    dst.flags = PropFlags::from_raw(PropFlags::SYNTHETIC.raw() | PropFlags::VALUE_REAL.raw());
    dst.value_int = value as i64;
    dst.value_str.data = EMPTY_CHAR.as_ptr();

    assert!(
        dst.name.length >= 4,
        "synthetic property name must be at least four bytes"
    );
    dst._internal_key = get_name_key(&name[..4]);
}

// ufbx.c:12479-12491 `ufbxi_init_synthetic_vec3_prop`
#[inline(never)]
pub(crate) fn init_synthetic_vec3_prop(
    dst: &PropView,
    name: &'static [u8],
    value: &Vec3,
    type_: PropType,
) {
    dst.set_type(type_);
    dst.name_view()
        .set(String::new_c(name.as_ptr(), synthetic_name_length(name)));
    // C: `dst->value_vec3 = *value;` writes only x/y/z of the value union.
    // SAFETY: `value_vec4_raw()` addresses `dst`'s own 4-`Real` value union
    // arm; `Vec3` is its 3-real prefix, so the projected write stays inside it.
    unsafe { *(dst.value_vec4_raw() as *mut Vec3) = *value };
    dst.set_flags(PropFlags::from_raw(
        PropFlags::SYNTHETIC.raw() | PropFlags::VALUE_VEC3.raw(),
    ));
    // C: `ufbxi_f64_to_i64(dst->value_real)` — `ufbx_real` argument promoted to
    // the `double` parameter.
    // SAFETY: `value_vec4_raw()` addresses `dst`'s own value union arm, whose
    // `x` (C's `value_real`) the vec3 store above wrote.
    let value_real: Real = unsafe { (*dst.value_vec4_raw()).x };
    dst.set_value_int(f64_to_i64(as_f64!(value_real)));
    dst.value_str_view().set(EMPTY_STRING.0);

    assert!(
        dst.name_view().length() >= 4,
        "synthetic property name must be at least four bytes"
    );
    dst.set_internal_key(get_name_key(&name[..4]));
}

// ufbx.c:12493-12505 `ufbxi_set_own_prop_vec3_uniform`
#[inline(never)]
pub(crate) fn set_own_prop_vec3_uniform(props: &PropsView, name: &[u8], value: Real) {
    // C: `ufbx_props local_props = *props;` — struct memcpy; `Props` is not
    // `Copy` but has no drop glue.
    // SAFETY: `props.get()` addresses the viewed live `ufbx_props` (the view's
    // mint vouch); `Props` has no drop glue, so the bitwise read duplicates it
    // without double-free (the copy is forgotten below).
    let mut local_props: Props = unsafe { core::ptr::read(props.get()) };
    local_props.defaults = None;
    // `name` is the interned static run itself — `find_prop`'s
    // pointer-identity carrier (its length is never read).
    let prop: Option<&PropView> =
        crate::native::parse::find_prop(PropsView::from_mut(&mut local_props), name);
    if let Some(prop) = prop {
        // SAFETY: `value_vec4_raw()` addresses the found prop's own 4-`Real`
        // value union arm, which C writes component by component.
        unsafe {
            let value_vec4: *mut Vec4 = prop.value_vec4_raw();
            (*value_vec4).x = value;
            (*value_vec4).y = value;
            (*value_vec4).z = value;
            (*value_vec4).w = 0.0;
        }
        // C-parity: bare `(int64_t)` cast on a float operand (saturating `as`).
        prop.set_value_int(value as i64);
    }
    // `local_props` is a bitwise copy sharing the original's pointers; forgetting it
    // documents that this copy must not be dropped as an owner (defensive against a
    // future `Drop`), even though the type currently has no drop glue.
    #[allow(clippy::forget_non_drop)]
    core::mem::forget(local_props);
}

// ufbx.c:12507-12510 `ufbxi_node_extra`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct NodeExtra {
    pub geometry_helper_id: u32,
    pub scale_helper_id: u32,
}

// ufbx.c:12512-12546 `ufbxi_setup_geometry_transform_helper`
#[inline(never)]
pub(crate) fn setup_geometry_transform_helper(
    uc: &Context,
    node: &View<UfbxNode, Mut>,
    node_fbx_id: u64,
) -> Result<(), Fail> {
    let node_props: &PropsView = node.element().props();
    let geo_translation: Vec3 = find_vec3(node_props, &sp::GeometricTranslation, 0.0, 0.0, 0.0);
    let geo_rotation: Vec3 = find_vec3(node_props, &sp::GeometricRotation, 0.0, 0.0, 0.0);
    let geo_scaling: Vec3 = find_vec3(node_props, &sp::GeometricScaling, 1.0, 1.0, 1.0);
    if !is_vec3_zero(geo_translation) || !is_vec3_zero(geo_rotation) || !is_vec3_one(geo_scaling) {
        // C: `uint64_t geo_fbx_id;` — written by `ufbxi_push_synthetic_element`
        // before any read; zero-initialized here (no upstream `ufbxi_uninit` marker).
        let mut geo_fbx_id: u64 = 0;
        // SAFETY: `geo_fbx_id` is an unaliased local the callee writes, the
        // helper name is uc's own NUL-terminated option string, and `UfbxNode`
        // is the element struct for `ElementType::Node`.
        let geo_node: *mut UfbxNode = unsafe {
            push_synthetic_element::<UfbxNode>(
                uc,
                ScalarView::from_mut(&mut geo_fbx_id),
                None,
                uc.opts_view().geometry_transform_helper_name_view().data(),
                ElementType::Node,
            )
        };
        ufbxi_check!(uc, !geo_node.is_null(), "geo_node");
        ufbxi_check!(
            uc,
            // SAFETY: `geo_node` is the fresh non-null element checked above,
            // so the borrow addresses its own `element.element_id`.
            !unsafe {
                uc.tmp_node_ids_view()
                    .push_copy_ref(&(*geo_node).element.element_id)
            }
            .is_null(),
            "((uint32_t*)ufbxi_push_size_copy((&uc->tmp_node_ids), sizeof(uint32_t), (1), (&geo_node->element.element_id)))"
        );
        // C: `geo_node->element.dom_node = node->element.dom_node;` — pointer
        // copy; `Option<Ref<T>>` is niche-packed to a bare pointer.
        // SAFETY: the projection addresses `geo_node`'s own live `dom_node`
        // field — `geo_node` is the fresh element above; the field has no drop
        // glue, so the bitwise copy stores safely.
        unsafe {
            (*geo_node).element.dom_node = node.element().dom_node();
        }

        let props: *mut Prop = uc.result_view().push_zero::<Prop>(3);
        ufbxi_check!(uc, !props.is_null(), "props");
        // SAFETY: `props` is the fresh non-null 3-element zeroed run checked
        // above, so indices 0..3 are live `ufbx_prop` slots with the result
        // arena's write-capable provenance — each anchors a `PropView`; each
        // name is one of the static `sp::*` property names retained by the
        // resulting scene property.
        unsafe {
            init_synthetic_vec3_prop(
                PropView::from_ptr(props.add(0)),
                &sp::Lcl_Rotation,
                &geo_rotation,
                PropType::Rotation,
            );
            init_synthetic_vec3_prop(
                PropView::from_ptr(props.add(1)),
                &sp::Lcl_Scaling,
                &geo_scaling,
                PropType::Scaling,
            );
            init_synthetic_vec3_prop(
                PropView::from_ptr(props.add(2)),
                &sp::Lcl_Translation,
                &geo_translation,
                PropType::Translation,
            );
        }

        // SAFETY: `geo_node` is the fresh non-null element above.
        unsafe {
            (*geo_node).element.props.props.data = props;
            (*geo_node).element.props.props.count = 3;
        }

        node.set_has_geometry_transform(true);
        // SAFETY: `geo_node` is the fresh non-null element above.
        unsafe { (*geo_node).is_geometry_transform_helper = true };

        connect_oo(uc, geo_fbx_id, node_fbx_id)?;
        uc.set_has_geometry_transform_nodes(true);

        let extra: *mut NodeExtra = push_element_extra(uc, node.element().element_id());
        ufbxi_check!(uc, !extra.is_null(), "extra");
        // SAFETY: `extra` is the fresh non-null extra-data slot checked above
        // and `geo_node` the fresh element above.
        unsafe { (*extra).geometry_helper_id = (*geo_node).element.element_id };
    }

    Ok(())
}

// ufbx.c:12548-12551 `ufbxi_scale_helper_prop`
#[repr(C)]
pub(crate) struct ScaleHelperProp {
    // C: `const char *name` — a borrow of the interned `ufbxi_*` static, which
    // `ufbxi_find_prop` matches by ADDRESS.
    pub name: &'static [u8],
    pub default_value: Vec3,
}

// ufbx.c:12553-12558 `ufbxi_scale_helper_props`
static SCALE_HELPER_PROPS: [ScaleHelperProp; 4] = [
    ScaleHelperProp {
        name: &sp::GeometricRotation,
        default_value: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    },
    ScaleHelperProp {
        name: &sp::GeometricScaling,
        default_value: Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    },
    ScaleHelperProp {
        name: &sp::GeometricTranslation,
        default_value: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    },
    ScaleHelperProp {
        name: &sp::Lcl_Scaling,
        default_value: Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    },
];

// ufbx.c:12560-12599 `ufbxi_setup_scale_helper`
#[inline(never)]
pub(crate) fn setup_scale_helper(
    uc: &Context,
    node: &View<UfbxNode, Mut>,
    node_fbx_id: u64,
) -> Result<(), Fail> {
    // C: `uint64_t scale_fbx_id;` — written by `ufbxi_push_synthetic_element`
    // before any read; zero-initialized here (no upstream `ufbxi_uninit` marker).
    let mut scale_fbx_id: u64 = 0;
    // SAFETY: `scale_fbx_id` is an unaliased local the callee writes, the
    // helper name is uc's own NUL-terminated option string, and `UfbxNode` is
    // the element struct for `ElementType::Node`.
    let scale_node: *mut UfbxNode = unsafe {
        push_synthetic_element::<UfbxNode>(
            uc,
            ScalarView::from_mut(&mut scale_fbx_id),
            None,
            uc.opts_view().scale_helper_name_view().data(),
            ElementType::Node,
        )
    };
    ufbxi_check!(uc, !scale_node.is_null(), "scale_node");
    // SAFETY: `scale_node` is the fresh non-null element just pushed into uc's
    // own element arena — reached through `*mut` (write-capable provenance for
    // `Mut`), live and unmoved for the rest of the load; the fields read below
    // are the ones the push and this function initialize.
    let scale_node: &View<UfbxNode> = unsafe { View::<UfbxNode>::from_ptr(scale_node) };
    ufbxi_check!(
        uc,
        // C copies the four `element_id` bytes out of the element; reading the
        // leaf and pushing a copy of that value stores the same bytes.
        !uc.tmp_node_ids_view()
            .push_copy_ref(&scale_node.element().element_id())
            .is_null(),
        "((uint32_t*)ufbxi_push_size_copy((&uc->tmp_node_ids), sizeof(uint32_t), (1), (&scale_node->element.element_id)))"
    );
    // C: `scale_node->element.dom_node = node->element.dom_node;` — pointer
    // copy; `Option<Ref<T>>` is niche-packed to a bare pointer.
    scale_node.element().set_dom_node(node.element().dom_node());

    // SAFETY: `scale_node` views the fresh non-null element checked above,
    // which lives in the element arena and outlives this scene's nodes — the
    // storage-lifetime half of `to_ref`'s contract.
    node.set_scale_helper(Some(unsafe { scale_node.to_ref() }));
    scale_node.set_is_scale_helper(true);

    connect_oo(uc, scale_fbx_id, node_fbx_id)?;
    uc.set_has_scale_helper_nodes(true);

    let extra: *mut NodeExtra = push_element_extra(uc, node.element().element_id());
    ufbxi_check!(uc, !extra.is_null(), "extra");
    // SAFETY: `extra` is the fresh non-null extra-data slot checked above.
    unsafe { (*extra).scale_helper_id = scale_node.element().element_id() };

    let max_props: usize = SCALE_HELPER_PROPS.len();
    let helper_props: *mut Prop = uc.result_view().push::<Prop>(max_props);
    ufbxi_check!(uc, !helper_props.is_null(), "helper_props");

    let mut num_props: usize = 0;
    // C: `ufbx_props props_copy = node->props;` — struct memcpy.
    // SAFETY: `props_ptr()` addresses the viewed node's own live
    // `element.props` (the view's mint vouch); `Props` has no drop glue, so the
    // bitwise read duplicates it without double-free (forgotten below).
    let mut props_copy: Props = unsafe { core::ptr::read(node.element().props_ptr()) };
    props_copy.defaults = None;
    let mut i: usize = 0;
    while i < max_props {
        let hp: *const ScaleHelperProp = &SCALE_HELPER_PROPS[i];
        // SAFETY: `hp` points into the `SCALE_HELPER_PROPS` static, whose
        // `name` is an interned `ufbxi_*` string.
        let src_prop: *mut Prop =
            match unsafe { find_prop(PropsView::from_mut(&mut props_copy), (*hp).name) } {
                Some(prop) => prop.get(),
                None => {
                    i += 1;
                    continue;
                }
            };

        // SAFETY: `num_props` counts the matches found so far, at most one per
        // `SCALE_HELPER_PROPS` entry, so it stays inside the `max_props`-long
        // `helper_props` run; `src_prop` is a live property of `node` and
        // `Prop` is `Copy`.
        unsafe { *helper_props.add(num_props) = *src_prop };
        num_props += 1;
        // C: `src_prop->value_vec3 = hp->default_value;`
        // SAFETY: `src_prop` is a live `ufbx_prop` of `node` and `Vec3` is the
        // 3-real prefix of its `Vec4` value union arm; `hp` points into the
        // `SCALE_HELPER_PROPS` static.
        unsafe { *(&raw mut (*src_prop).value_vec4 as *mut Vec3) = (*hp).default_value };
        // C-parity: bare `(int64_t)` cast on a float operand (saturating `as`).
        // SAFETY: `src_prop` is a live `ufbx_prop` of `node`.
        unsafe { (*src_prop).value_int = (*src_prop).value_vec4.x as i64 };
        i += 1;
    }
    // `props_copy` is a bitwise copy sharing the original's pointers; forgetting it
    // documents that this copy must not be dropped as an owner (defensive against a
    // future `Drop`), even though the type currently has no drop glue.
    #[allow(clippy::forget_non_drop)]
    core::mem::forget(props_copy);

    // `helper_props` is the result-buffer run whose first `num_props` entries
    // were filled in above; the C's field-write order is kept.
    let scale_props = scale_node.element().props().props_view();
    scale_props.set_data(helper_props);
    scale_props.set_count(num_props);

    Ok(())
}

// ufbx.c:12601-12627 `ufbxi_read_model`
#[inline(never)]
pub(crate) fn read_model(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    ufbxi_ignore!(node);
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `UfbxNode` is the element struct for `ElementType::Node`.
    let elem_node: *mut UfbxNode = unsafe { push_element::<UfbxNode>(uc, info, ElementType::Node) };
    ufbxi_check!(uc, !elem_node.is_null(), "elem_node");
    ufbxi_check!(
        uc,
        // SAFETY: `elem_node` is the fresh non-null element checked above, so
        // the borrow addresses its own `element.element_id`.
        !unsafe {
            uc.tmp_node_ids_view()
                .push_copy_ref(&(*elem_node).element.element_id)
        }
        .is_null(),
        "((uint32_t*)ufbxi_push_size_copy((&uc->tmp_node_ids), sizeof(uint32_t), (1), (&elem_node->element.element_id)))"
    );

    let inherit_type: i64 = find_int(
        // SAFETY: `elem_node` is the fresh non-null element above, so the
        // projection addresses its own `element.props`.
        unsafe { PropsView::from_ptr(&raw mut (*elem_node).element.props) },
        &sp::InheritType,
        -1,
    );
    // SAFETY: `elem_node` is the fresh non-null element above.
    unsafe {
        match inherit_type {
            // RrSs
            0 => (*elem_node).original_inherit_mode = InheritMode::ComponentwiseScale,
            // Rrs
            2 => (*elem_node).original_inherit_mode = InheritMode::IgnoreParentScale,
            _ => {}
        }
    }

    if uc.opts_view().inherit_mode_handling() == InheritModeHandling::Preserve {
        // SAFETY: `elem_node` is the fresh non-null element above.
        unsafe { (*elem_node).inherit_mode = (*elem_node).original_inherit_mode };
    } else if uc.opts_view().inherit_mode_handling() == InheritModeHandling::Ignore {
        // SAFETY: as above.
        unsafe {
            (*elem_node).original_inherit_mode = InheritMode::Normal;
            (*elem_node).inherit_mode = InheritMode::Normal;
        }
    }

    Ok(())
}

// ufbx.c:12629-12635 `ufbxi_read_element`
///
/// # Safety
/// `push_element_size`'s contract, passed straight through: `info` addresses a
/// live `ufbxi_element_info` whose escaping `name`/`props`/`dom_node` outlive
/// the scene, and `size` is the size of the element struct for `type_`.
#[inline(never)]
pub(crate) unsafe fn read_element(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
    size: usize,
    type_: ElementType,
) -> Result<(), Fail> {
    ufbxi_ignore!(node);
    // SAFETY: `info` is the caller's live `ufbxi_element_info` whose escaping
    // pointers outlive the scene and `size` is the size of the element struct
    // for `type_` — this fn's contract, which is `push_element_size`'s.
    let elem: *mut Element = unsafe { push_element_size(uc, info, size, type_) };
    ufbxi_check!(uc, !elem.is_null(), "elem");
    Ok(())
}

// ufbx.c:12637-12653 `ufbxi_read_unknown`
// `node_name` is the interned run itself: its pointer is stored into
// `unknown->super_type.data` (the pointer-identity carrier) and its length is
// measured by `strlen`, exactly as C does.
//
// # Safety
// `node_name` must be NUL-terminated within its own run — `strlen` walks it
// from `node_name.as_ptr()`, an obligation `&[u8]` does not carry.
#[inline(never)]
pub(crate) unsafe fn read_unknown(
    uc: &Context,
    node: &NodeView,
    element: &View<ElementInfo, Mut>,
    type_: String,
    sub_type: String,
    node_name: &[u8],
) -> Result<(), Fail> {
    ufbxi_ignore!(node);
    // SAFETY: `element` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Unknown` is the element struct for `ElementType::Unknown`.
    let unknown: *mut Unknown =
        unsafe { push_element::<Unknown>(uc, element, ElementType::Unknown) };
    ufbxi_check!(uc, !unknown.is_null(), "unknown");
    // SAFETY: `unknown` is the fresh non-null element checked above; `node_name`
    // is NUL-terminated within its own run (fn contract) — `strlen`'s contract.
    unsafe {
        (*unknown).type_ = type_;
        (*unknown).sub_type = sub_type;
        (*unknown).super_type.data = node_name.as_ptr();
        (*unknown).super_type.length = strlen(node_name.as_ptr());
    }

    // `type`, `sub_type` and `node_name` are raw strings so they may need to be sanitized.
    // SAFETY: each view is minted over a field of the fresh element above,
    // holding the data/length pair written just now.
    let (type_view, sub_type_view, super_type_view) = unsafe {
        (
            StringView::from_ptr(&raw mut (*unknown).type_),
            StringView::from_ptr(&raw mut (*unknown).sub_type),
            StringView::from_ptr(&raw mut (*unknown).super_type),
        )
    };
    push_string_place_str(uc.string_pool_view(), type_view, false)?;
    push_string_place_str(uc.string_pool_view(), sub_type_view, false)?;
    push_string_place_str(uc.string_pool_view(), super_type_view, false)?;

    Ok(())
}

// ufbx.c:12655-12658 `typedef struct { ufbx_vertex_vec3 elem; uint32_t index; } ufbxi_tangent_layer;`
#[repr(C)]
pub(crate) struct TangentLayer {
    pub elem: VertexVec3,
    pub index: u32,
}

// Navigation surface over a tmp-stack tangent/bitangent layer slot: `elem()`
// is the by-value C struct assignment (`uv_set->vertex_bitangent = layer->elem`),
// `elem_view()` the in-place projection for the `exists` read and `elem_raw()`
// the address-of-parity projection the vertex-element reader writes through.
#[allow(dead_code)]
impl<M: Mode> View<TangentLayer, M> {
    #[inline(always)]
    pub(crate) fn elem(&self) -> VertexVec3 {
        view_read_shared!(self, elem)
    }
    #[inline(always)]
    pub(crate) fn elem_view(&self) -> &View<VertexVec3, M> {
        view_project!(self, elem)
    }
    #[inline(always)]
    pub(crate) fn index(&self) -> u32 {
        view_read_shared!(self, index)
    }
}

#[allow(dead_code)]
impl View<TangentLayer> {
    #[inline(always)]
    pub(crate) fn set_index(&self, value: u32) {
        view_write!(self, index, value)
    }
    #[inline(always)]
    pub(crate) fn elem_raw(&self) -> *mut VertexVec3 {
        view_raw_mut!(self, elem)
    }
}

// ufbx.c:12660 `static ufbx_real ufbxi_zero_element[8] = { 0 };`
// C-parity: the C datum is NOT `const` — its address is stored into
// `attrib->values.data` (a non-const `ufbx_real *`), so the port needs an
// interior-mutable static rather than a plain immutable one. Nothing ever
// writes through it. `Sync` wrapper as for `EMPTY_STRING` in `native::api`.
#[repr(transparent)]
struct ZeroElement(core::cell::UnsafeCell<[Real; 8]>);
unsafe impl Sync for ZeroElement {}
static ZERO_ELEMENT: ZeroElement = ZeroElement(core::cell::UnsafeCell::new([0.0; 8]));

// Sentinel pointers used for zero/sequential index buffers
// ufbx.c:12662-12664 `ufbxi_sentinel_index_zero` / `ufbxi_sentinel_index_consecutive`
// (compared by ADDRESS, never dereferenced through the attribute; the payload
// values are debug tells).
pub(crate) static SENTINEL_INDEX_ZERO: [u32; 1] = [100000000];
pub(crate) static SENTINEL_INDEX_CONSECUTIVE: [u32; 1] = [123456789];

// ufbx.c:12666-12690 `ufbxi_fix_index`
#[inline(never)]
pub(crate) fn fix_index(
    uc: &Context,
    p_dst: &View<u32>,
    index: u32,
    one_past_max_val: usize,
) -> Result<(), Fail> {
    match uc.opts_view().index_error_handling() {
        IndexErrorHandling::Clamp => {
            ufbxi_check!(uc, one_past_max_val > 0);
            ufbxi_check!(
                uc,
                one_past_max_val <= u32::MAX as usize,
                "one_past_max_val <= UINT32_MAX"
            );
            // SAFETY: `p_dst` is a `Mut` view over a live, unmoved,
            // write-capable `uint32_t` index slot (its mint contract); the slot
            // need not hold an initialized value, and the write initializes it.
            unsafe { p_dst.get().write((one_past_max_val as u32).wrapping_sub(1)) };
            ufbxi_check!(
                uc,
                ufbxi_warnf!(uc, WarningType::IndexClamped, "Clamped index").is_ok(),
                "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_INDEX_CLAMPED, ~0u, \"Clamped index\")"
            );
        }
        IndexErrorHandling::NoIndex => {
            // SAFETY: `p_dst` is a `Mut` view over a live, unmoved,
            // write-capable `uint32_t` index slot (its mint contract).
            unsafe { p_dst.get().write(NO_INDEX) };
        }
        IndexErrorHandling::AbortLoading => {
            // C-parity: `one_past_max_val` is a `size_t` passed through `%u`,
            // which reads an `unsigned int` — the low 32 bits on the oracle
            // targets. The `as u32` narrowing reproduces that exactly.
            // SAFETY: the format string is a NUL-terminated literal whose two
            // `%u` conversions are matched by the two `u32` arguments —
            // `fmt_err_info`'s contract.
            unsafe {
                ufbxi_fmt_err_info!(
                    Some(uc.error_view()),
                    "%u (max %u)",
                    index,
                    (if one_past_max_val != 0 {
                        one_past_max_val - 1
                    } else {
                        0
                    }) as u32
                );
            }
            ufbxi_fail_msg!(uc, "UFBX_INDEX_ERROR_HANDLING_ABORT_LOADING", "Bad index");
            // C: no `break` here — `ufbxi_fail_msg()` returns, so the
            // fallthrough into `UNSAFE_IGNORE` below is unreachable.
        }
        IndexErrorHandling::UnsafeIgnore => {
            // SAFETY: `p_dst` is a `Mut` view over a live, unmoved,
            // write-capable `uint32_t` index slot (its mint contract).
            unsafe { p_dst.get().write(index) };
        }
        // C `default:` — unreachable in Rust because the match above is
        // exhaustive over the enum, but kept for diff parity.
        #[allow(unreachable_patterns)]
        _ => {
            ufbxi_unreachable!("Unhandled index_error_handling");
            return Err(Fail::unrecorded());
        }
    }

    Ok(())
}

// ufbx.c:12692-12728 `ufbxi_check_indices`
/// # Safety
/// `indices` must address `num_indices` initialized `u32`s and be writable when
/// `owns_indices` is true. `dst.count()` must equal `num_indexers`. Any input
/// run published without copying must stay live and unmoved with `dst`.
#[inline(never)]
pub(crate) unsafe fn check_indices(
    uc: &Context,
    dst: &ListView<u32>,
    indices: *mut u32,
    owns_indices: bool,
    num_indices: usize,
    num_indexers: usize,
    num_elems: usize,
) -> Result<(), Fail> {
    let mut indices = indices;
    let mut num_indices = num_indices;
    let mut owns_indices = owns_indices;

    // If the indices are truncated extend them with `UFBX_NO_INDEX`, the following normalization pass
    // will handle them the same way as other out-of-bounds indices.
    if num_indices < num_indexers {
        let new_indices: *mut u32 = uc.result_view().push::<u32>(num_indexers);
        ufbxi_check!(uc, !new_indices.is_null(), "new_indices");

        // SAFETY: `indices` spans `num_indices` readable `u32`s (fn contract)
        // and `new_indices` is the fresh `num_indexers`-long result-buffer run
        // checked above, with `num_indices < num_indexers`; the two buffers are
        // distinct objects (the destination was allocated just now).
        unsafe { core::ptr::copy_nonoverlapping(indices, new_indices, num_indices) };
        for i in num_indices..num_indexers {
            // SAFETY: `i < num_indexers`, the length of the run at
            // `new_indices`.
            unsafe { *new_indices.add(i) = NO_INDEX };
        }

        indices = new_indices;
        num_indices = num_indexers;
        owns_indices = true;
    }

    // Normalize out-of-bounds indices to `invalid_index`
    for i in 0..num_indices {
        // SAFETY: `i < num_indices`, the length of the run at `indices` — the
        // caller's buffer, or the `num_indexers`-long extension made above.
        let ix: u32 = unsafe { *indices.add(i) };
        if ix as usize >= num_elems {
            // If the indices refer to an external buffer we need to
            // allocate a separate buffer for them
            if !owns_indices {
                // SAFETY: the buffer is uc's own result buffer and `indices`
                // spans the `num_indices` `u32`s being copied out of it.
                indices = unsafe { uc.result_view().push_copy_raw::<u32>(num_indices, indices) };
                ufbxi_check!(uc, !indices.is_null(), "indices");
                owns_indices = true;
            }
            // SAFETY: `i < num_indices`, so `indices.add(i)` is a live,
            // write-capable slot of the owned index run — an adequate mint for
            // the `Mut` index-slot view `fix_index` writes through.
            let p_dst: &View<u32> = unsafe { View::<u32, Mut>::from_ptr(indices.add(i)) };
            fix_index(uc, p_dst, ix, num_elems)?;
        }
    }

    // SAFETY: the normalization above leaves `indices` addressing at least the
    // destination's already-established logical count of initialized `u32`s;
    // the caller vouches that the input or result-buffer run stays live for
    // the enclosing attribute.
    dst.set(unsafe { List::from_raw_parts(indices, dst.count()) });

    Ok(())
}

// ufbx.c:12730-12733 `ufbx_static_assert(vertex_{real,vec2,vec3,vec4}_size, ...)`
const _: () = assert!(size_of::<VertexReal>() == size_of::<VertexAttrib>());
const _: () = assert!(size_of::<VertexVec2>() == size_of::<VertexAttrib>());
const _: () = assert!(size_of::<VertexVec3>() == size_of::<VertexAttrib>());
const _: () = assert!(size_of::<VertexVec4>() == size_of::<VertexAttrib>());

// ufbx.c:12735-12739 `ufbxi_warn_polygon_mapping`
//
// `data_name` and `mapping` stay raw: they feed the two `%s` conversions and
// nothing else, so the printf scans each to its terminator and neither a
// length nor a provenance-carrying borrow expresses that obligation
// (PORTING.md "Fn boundaries take borrows, not raw pointers" names printf
// `%s` as the case where an honest `unsafe fn` beats a safe signature over a
// slice the callee still walks past).
//
// # Safety
// `data_name` and `mapping` must each point at a NUL-terminated run — a
// string-pool-interned name, a `ufbxi_*` name constant or the empty literal —
// readable from the pointer through its terminator.
#[inline(never)]
pub(crate) unsafe fn warn_polygon_mapping(
    uc: &Context,
    data_name: *const u8,
    mapping: *const u8,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        ufbxi_warnf!(
            uc,
            WarningType::MissingPolygonMapping,
            "Ignoring geometry '%s' with bad mapping mode '%s'",
            data_name,
            mapping,
        )
        .is_ok(),
        "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_MISSING_POLYGON_MAPPING, ~0u, \"Ignoring geometry '%s' with bad mapping mode '%s'\", data_name, mapping)"
    );
    Ok(())
}

// Navigation surface over a `ufbx_void_list` field — the type-erased sibling of
// `View<List<T>>`. The generator emits list views for the typed `ufbx_*_list`
// members only, so `ufbx_vertex_attrib.values` gets its accessors by hand.
#[allow(dead_code)]
impl<M: Mode> View<VoidList, M> {
    #[inline(always)]
    pub(crate) fn data(&self) -> *mut c_void {
        view_read_shared!(self, data)
    }
    #[inline(always)]
    pub(crate) fn count(&self) -> usize {
        view_read_shared!(self, count)
    }
}

#[allow(dead_code)]
impl View<VoidList> {
    #[inline(always)]
    pub(crate) fn set_data(&self, value: *mut c_void) {
        view_write!(self, data, value)
    }
    #[inline(always)]
    pub(crate) fn set_count(&self, value: usize) {
        view_write!(self, count, value)
    }
}

// In-place projection of the untyped value list, which the generated
// `View<VertexAttrib, M>` surface reaches only as a raw pointer (`values_raw`).
impl<M: Mode> View<VertexAttrib, M> {
    #[inline(always)]
    pub(crate) fn values_view(&self) -> &View<VoidList, M> {
        view_project!(self, values)
    }
}

// ufbx.c:12741-12926 `ufbxi_read_vertex_element`
//
// `data_name` / `index_name` / `w_name` are the interned name runs themselves:
// their POINTERS are the identity keys `ufbxi_find_array` matches on, and
// `data_name` / `w_name` additionally reach the `%s` conversions of the warning
// formats. `w_name` is `Option` for C's nullable `w_name`.
//
// # Safety
// `data_name` and `w_name` must be NUL-terminated within their own run — the
// `%s` conversions walk them from `as_ptr()`, an obligation `&[u8]` does not
// carry.
#[inline(never)]
pub(crate) unsafe fn read_vertex_element(
    uc: &Context,
    mesh: &View<Mesh>,
    node: &NodeView,
    attrib: &View<VertexAttrib>,
    data_name: &[u8],
    index_name: &[u8],
    w_name: Option<&[u8]>,
    data_type: u8,
    num_components: usize,
) -> Result<(), Fail> {
    // C: `ufbx_real **p_dst_data = (ufbx_real**)&attrib->values.data;` — the
    // destination is the attribute's own `values.data` slot, reached in place;
    // the `ufbx_real**` cast survives at the two writes through it below.
    let p_dst_data: &View<VoidList> = attrib.values_view();

    let data: *mut ValueArray = find_array(node, data_name.as_ptr(), data_type);
    let indices: *mut ValueArray = find_array(node, index_name.as_ptr(), b'i');

    if !uc.opts_view().strict() {
        if data.is_null() {
            return Ok(());
        }
    }

    ufbxi_check!(uc, !data.is_null(), "data");
    // SAFETY: `data` is non-null (checked just above) and `find_array` returns
    // the node's own array descriptor, live for as long as the parse tree and
    // reached through `*mut` (write-capable provenance for `Mut`).
    let data: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(data) };
    // SAFETY: the view is minted only in the non-null arm, where `indices` is
    // likewise a live parse-tree array descriptor reached through `*mut`.
    let indices: Option<&View<ValueArray>> = if indices.is_null() {
        None
    } else {
        Some(unsafe { View::<ValueArray>::from_ptr(indices) })
    };
    ufbxi_check!(
        uc,
        data.size() % num_components == 0,
        "data->size % num_components == 0"
    );

    let num_elems: usize = data.size() / num_components;

    // HACK: If there's no elements at all keep the attribute as NULL
    // TODO: Strict mode for this?
    if num_elems == 0 {
        return Ok(());
    }

    ufbxi_check!(
        uc,
        num_elems > 0 && num_elems < i32::MAX as usize,
        "num_elems > 0 && num_elems < INT32_MAX"
    );

    attrib.set_exists(true);
    attrib.indices_view().set_count(mesh.num_indices());

    // C: `const char *mapping = "";` — an anonymous empty literal, never
    // pointer-equal to any interned `ufbxi_*` name constant.
    let mut mapping: *const u8 = EMPTY_CHAR.as_ptr();
    if let Some(got) = find_val1::<Checked<*const u8>>(node, sp::MappingInformationType.as_ptr()) {
        mapping = got.0;
    }

    attrib
        .values_view()
        .set_count(if num_elems != 0 { num_elems } else { 1 });

    // Data array is always used as-is, if empty set the data to a global
    // zero buffer so invalid zero index can point to some valid data.
    // The zero data is offset by 4 elements to accommodate for invalid index (-1)
    if num_elems > 0 {
        p_dst_data.set_data(data.data() as *mut Real as *mut c_void);
    } else {
        // SAFETY: `ZERO_ELEMENT` is a static 8-`Real` buffer, so offsetting by
        // 4 stays inside it.
        let zero: *mut Real = unsafe { (ZERO_ELEMENT.0.get() as *mut Real).add(4) };
        p_dst_data.set_data(zero as *mut c_void);
    }

    // HACK: Some old exporters seem to use ByPolygon to mean ByPolygonVertex,
    // it should be quite safe to remap this
    if mapping == sp::ByPolygon.as_ptr() {
        let num_indices: usize = if let Some(indices) = indices {
            indices.size()
        } else {
            num_elems
        };
        if num_indices == mesh.num_indices() {
            mapping = sp::ByPolygonVertex.as_ptr();
        }
    }

    if let Some(indices) = indices {
        let num_indices: usize = indices.size();
        // The `'i'` array's payload is a run of `size` `u32`s.
        let index_data: *mut u32 = indices.data() as *mut u32;

        if mapping == sp::ByPolygonVertex.as_ptr() {
            // Indexed by polygon vertex: We can use the provided indices directly.
            // SAFETY: `index_data` spans `num_indices` initialized `u32`s (the
            // array descriptor's payload), and the destination already carries
            // its `mesh.num_indices()` logical count.
            unsafe {
                check_indices(
                    uc,
                    attrib.indices_view(),
                    index_data,
                    true,
                    num_indices,
                    mesh.num_indices(),
                    num_elems,
                )
            }?;
        } else if mapping == sp::ByVertex.as_ptr() || mapping == sp::ByVertice.as_ptr() {
            // Indexed by vertex: Follow through the position index mapping to get the final indices.
            let new_index_data: *mut u32 = uc.result_view().push::<u32>(mesh.num_indices());
            ufbxi_check!(uc, !new_index_data.is_null(), "new_index_data");
            // SAFETY: the non-null `mesh.num_indices()`-element allocation just
            // pushed on uc's own result buf — one contiguous, write-capable run
            // that stays alive and unmoved for the loop below (`fix_index`
            // pushes only warnings, and an arena push never moves a live run).
            // Its slots are still uninitialized, which the run tolerates.
            let new_index_run: Run<'_, u32, Mut> =
                unsafe { Run::<u32, Mut>::from_raw_parts(new_index_data, mesh.num_indices()) };

            // `vertex_indices` is the mesh's own initialized list; the loop
            // reads it through the bounded list accessor and writes only the
            // disjoint fresh `new_index_data` run, whose length is exactly the
            // `mesh.num_indices()` the loop counts to.
            // SAFETY: `index_data` is the array descriptor's own contiguous
            // payload of `num_indices` `u32`s, live for the parse tree and
            // likewise unwritten by the loop.
            let index_run: &[u32] = unsafe { slice_from_ptr(index_data, num_indices) };
            for i in 0..mesh.num_indices() {
                let ix: u32 = mesh.vertex_indices_view().copy_at(i);
                if (ix as usize) < num_indices {
                    new_index_run.write_at(i, index_run[ix as usize]);
                } else {
                    let p_dst: &View<u32> = new_index_run.at(i);
                    fix_index(uc, p_dst, ix, num_elems)?;
                }
            }

            // SAFETY: `new_index_data` is the fresh `mesh.num_indices()`-long
            // run filled in by the loop above, and the destination already
            // carries that logical count.
            unsafe {
                check_indices(
                    uc,
                    attrib.indices_view(),
                    new_index_data,
                    true,
                    mesh.num_indices(),
                    mesh.num_indices(),
                    num_elems,
                )
            }?;
            attrib.set_unique_per_vertex(true);
        } else if mapping == sp::ByPolygon.as_ptr() {
            // Indexed by polygon: Generate new indices based on polygons
            let new_index_data: *mut u32 = uc.result_view().push::<u32>(mesh.num_indices());
            ufbxi_check!(uc, !new_index_data.is_null(), "new_index_data");

            let num_faces: usize = mesh.num_faces();
            // `faces` is the mesh's own initialized list; the loop reads it
            // through the bounded list accessor and writes only the disjoint
            // fresh `new_index_data` run.
            // SAFETY: `index_data` is the array descriptor's own contiguous
            // payload of `num_indices` `u32`s, live for the parse tree and
            // likewise unwritten by the loop.
            let index_run: &[u32] = unsafe { slice_from_ptr(index_data, num_indices) };
            for face_ix in 0..num_faces {
                let face: Face = mesh.faces_view().copy_at(face_ix);
                let mut index: u32 = NO_INDEX;
                if face_ix < num_indices {
                    index = index_run[face_ix];
                }
                if index as usize >= num_elems {
                    let invalid_index = index;
                    let p_dst: &View<u32> = View::<u32, Mut>::from_mut(&mut index);
                    fix_index(uc, p_dst, invalid_index, num_elems)?;
                }
                for i in 0..face.num_indices as usize {
                    // SAFETY: every face's `index_begin + num_indices` stays
                    // within the mesh's `num_indices`, the length of the fresh
                    // `new_index_data` run.
                    unsafe { *new_index_data.add(face.index_begin as usize + i) = index };
                }
            }

            attrib.indices_view().set_data(new_index_data);
        } else if mapping == sp::AllSame.as_ptr() {
            // Indexed by all same: ??? This could be possibly used for making
            // holes with invalid indices, but that seems really fringe.
            // Just use the shared zero index buffer for this.
            uc.set_max_zero_indices(max_sz(uc.max_zero_indices(), mesh.num_indices()));
            attrib.indices_view().set_data(SENTINEL_INDEX_ZERO.as_ptr());
            attrib.set_unique_per_vertex(true);
        } else {
            // SAFETY: `get()` addresses the viewed attribute, so its own
            // `size_of::<VertexAttrib>()` bytes are writable.
            unsafe {
                core::ptr::write_bytes(attrib.get() as *mut u8, 0, size_of::<VertexAttrib>())
            };
            // SAFETY: `data_name` is the caller's NUL-terminated name and
            // `mapping` is either the empty literal or an interned `ufbxi_*`
            // name — both NUL-terminated, as the `%s` conversions require.
            unsafe { warn_polygon_mapping(uc, data_name.as_ptr(), mapping) }?;
            return Ok(());
        }
    } else {
        if mapping == sp::ByPolygonVertex.as_ptr() {
            // Direct by polygon index: Use shared consecutive array if there's enough
            // elements, otherwise use a unique truncated consecutive index array.
            if num_elems >= mesh.num_indices() {
                uc.set_max_consecutive_indices(max_sz(
                    uc.max_consecutive_indices(),
                    mesh.num_indices(),
                ));
                attrib
                    .indices_view()
                    .set_data(SENTINEL_INDEX_CONSECUTIVE.as_ptr());
            } else {
                let index_data: *mut u32 = uc.result_view().push::<u32>(mesh.num_indices());
                ufbxi_check!(uc, !index_data.is_null(), "index_data");
                for i in 0..mesh.num_indices() {
                    // SAFETY: `i < mesh.num_indices`, the length of the fresh
                    // run at `index_data`.
                    unsafe { *index_data.add(i) = i as u32 };
                }
                // SAFETY: `index_data` is the fresh `mesh.num_indices()`-long
                // run filled in by the loop above, and the destination already
                // carries that logical count.
                unsafe {
                    check_indices(
                        uc,
                        attrib.indices_view(),
                        index_data,
                        true,
                        mesh.num_indices(),
                        mesh.num_indices(),
                        num_elems,
                    )
                }?;
            }
        } else if mapping == sp::ByVertex.as_ptr() || mapping == sp::ByVertice.as_ptr() {
            // Direct by vertex: We can re-use the position indices..
            // SAFETY: the mesh's `vertex_position.indices` spans its own
            // `num_indices` entries and stays owned by the mesh
            // (`owns_indices` is `false`); the destination already carries the
            // same logical count.
            unsafe {
                check_indices(
                    uc,
                    attrib.indices_view(),
                    mesh.vertex_position().indices().data as *mut u32,
                    false,
                    mesh.num_indices(),
                    mesh.num_indices(),
                    num_elems,
                )
            }?;
            attrib.set_unique_per_vertex(true);
        } else if mapping == sp::ByPolygon.as_ptr() {
            // Direct by polygon: Generate new indices based on polygons
            let new_index_data: *mut u32 = uc.result_view().push::<u32>(mesh.num_indices());
            ufbxi_check!(uc, !new_index_data.is_null(), "new_index_data");

            let num_faces: u32 = mesh.num_faces() as u32;
            // `faces` is the mesh's own initialized list; the loop reads it
            // through the bounded list accessor and writes only the disjoint
            // fresh `new_index_data` run.
            for face_ix in 0..num_faces {
                let face: Face = mesh.faces_view().copy_at(face_ix as usize);
                for i in 0..face.num_indices as usize {
                    // SAFETY: every face's `index_begin + num_indices` stays
                    // within the mesh's `num_indices`, the length of the fresh
                    // `new_index_data` run.
                    unsafe { *new_index_data.add(face.index_begin as usize + i) = face_ix };
                }
            }

            // SAFETY: `new_index_data` is the fresh `mesh.num_indices()`-long
            // run filled in by the loop above, and the destination already
            // carries that logical count.
            unsafe {
                check_indices(
                    uc,
                    attrib.indices_view(),
                    new_index_data,
                    true,
                    mesh.num_indices(),
                    mesh.num_indices(),
                    num_elems,
                )
            }?;
        } else if mapping == sp::AllSame.as_ptr() {
            // Direct by all same: This cannot fail as the index list is just zero.
            uc.set_max_zero_indices(max_sz(uc.max_zero_indices(), mesh.num_indices()));
            attrib.indices_view().set_data(SENTINEL_INDEX_ZERO.as_ptr());
            attrib.set_unique_per_vertex(true);
        } else {
            // SAFETY: `get()` addresses the viewed attribute, so its own
            // `size_of::<VertexAttrib>()` bytes are writable.
            unsafe {
                core::ptr::write_bytes(attrib.get() as *mut u8, 0, size_of::<VertexAttrib>())
            };
            // SAFETY: `data_name` is the caller's NUL-terminated name and
            // `mapping` is either the empty literal or an interned `ufbxi_*`
            // name — both NUL-terminated, as the `%s` conversions require.
            unsafe { warn_polygon_mapping(uc, data_name.as_ptr(), mapping) }?;
            return Ok(());
        }
    }

    if uc.opts_view().retain_vertex_attrib_w() {
        if let Some(w_name) = w_name {
            let w_data: *mut ValueArray = find_array(node, w_name.as_ptr(), b'r');
            if !w_data.is_null() {
                // SAFETY: `w_data` is non-null (checked just above) and `find_array`
                // returns the node's own array descriptor, live for as long as the
                // parse tree and reached through `*mut` (write-capable provenance).
                let w_data: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(w_data) };
                if w_data.size() == num_elems {
                    // The `'r'` array's payload is a run of `size` `ufbx_real`s.
                    attrib.values_w_view().set_count(w_data.size());
                    attrib.values_w_view().set_data(w_data.data() as *mut Real);
                } else {
                    ufbxi_check!(
                        uc,
                        ufbxi_warnf!(
                            uc,
                            WarningType::BadVertexWAttribute,
                            "Bad W array size %s=%zu, %s=%zu",
                            w_name.as_ptr(),
                            w_data.size(),
                            data_name.as_ptr(),
                            num_elems,
                        )
                        .is_ok(),
                        "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_BAD_VERTEX_W_ATTRIBUTE, ~0u, \"Bad W array size %s=%zu, %s=%zu\", w_name, w_data->size, data_name, num_elems)"
                    );
                }
            }
        }
    }

    Ok(())
}

// ufbx.c:12928-12960 `ufbxi_read_truncated_array`
/// # Safety
/// `fmt` must select array elements with the same size, alignment and value
/// validity as `T`. The source array payload, or the result-buffer copy made
/// here, must remain live and unmoved for every later use of `dst`.
#[inline(never)]
pub(crate) unsafe fn read_truncated_array<T>(
    uc: &Context,
    dst: &ListView<T>,
    node: &NodeView,
    name: *const u8,
    fmt: u8,
    size: usize,
) -> Result<(), Fail> {
    let arr: *mut ValueArray = find_array(node, name, fmt);
    if arr.is_null() {
        ufbxi_check!(
            uc,
            ufbxi_warnf!(
                uc,
                WarningType::MissingGeometryData,
                "Missing geometry data: %s",
                name,
            )
            .is_ok(),
            "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_MISSING_GEOMETRY_DATA, ~0u, \"Missing geometry data: %s\", name)"
        );
        return Ok(());
    }

    // SAFETY: `arr` is non-null (the null case returned above) and points at
    // the node's own array descriptor, live for as long as the parse tree and
    // reached through `*mut` (write-capable provenance for `Mut`).
    let arr: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(arr) };
    let mut data: *mut c_void = arr.data();
    if arr.size() < size {
        ufbxi_check!(
            uc,
            ufbxi_warnf!(uc, WarningType::TruncatedArray, "Truncated array: %s", name).is_ok(),
            "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_TRUNCATED_ARRAY, ~0u, \"Truncated array: %s\", name)"
        );

        let elem_size: usize = array_type_size(fmt);
        let new_data: *mut c_void = push_size(uc.result_view(), elem_size, size);
        ufbxi_check!(uc, !new_data.is_null(), "new_data");
        // SAFETY: `arr.data` spans `arr.size * elem_size` readable bytes and
        // `new_data` is the fresh `size * elem_size`-byte run checked above,
        // with `arr.size < size`; the two are distinct objects.
        unsafe {
            core::ptr::copy_nonoverlapping(
                data as *const u8,
                new_data as *mut u8,
                arr.size() * elem_size,
            );
        }
        // Extend the array with the last element if possible
        if arr.size() > 0 {
            // SAFETY: `arr.size > 0`, so the last element starts at
            // `(arr.size - 1) * elem_size` inside `arr.data`'s payload.
            let first_elem: *mut u8 =
                unsafe { (data as *mut u8).add((arr.size() - 1) * elem_size) };
            for i in arr.size()..size {
                // SAFETY: `first_elem` spans one `elem_size` element of the
                // source array, and `i < size` bounds the destination slot
                // inside the `size * elem_size`-byte run at `new_data`.
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        first_elem as *const u8,
                        (new_data as *mut u8).add(i * elem_size),
                        elem_size,
                    );
                }
            }
        } else {
            // SAFETY: `new_data` is the fresh `size * elem_size`-byte run.
            unsafe { core::ptr::write_bytes(new_data as *mut u8, 0, size * elem_size) };
        }
        data = new_data;
    }

    // SAFETY: the `fmt`/`T` compatibility and escaping lifetime are the
    // caller's contract. The source payload already contains `size` elements,
    // or the truncated branch initialized exactly that many in `new_data`.
    dst.set(unsafe { List::from_raw_parts(data.cast::<T>(), size) });
    Ok(())
}

// ufbx.c:12962-12967 `ufbxi_uv_set_less`
#[inline(never)]
pub(crate) unsafe extern "C" fn uv_set_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const UvSet = va as *const UvSet;
    let b: *const UvSet = vb as *const UvSet;
    // SAFETY: the sort passes two live elements of the `ufbx_uv_set` run it was
    // handed (the comparator contract this fn is registered under).
    unsafe { (*a).index < (*b).index }
}

// ufbx.c:12969-12974 `ufbxi_color_set_less`
#[inline(never)]
pub(crate) unsafe extern "C" fn color_set_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const ColorSet = va as *const ColorSet;
    let b: *const ColorSet = vb as *const ColorSet;
    // SAFETY: the sort passes two live elements of the `ufbx_color_set` run it
    // was handed (the comparator contract this fn is registered under).
    unsafe { (*a).index < (*b).index }
}

// ufbx.c:12976-12981 `ufbxi_sort_uv_sets`
#[inline(never)]
pub(crate) fn sort_uv_sets(uc: &Context, sets: Run<'_, UvSet>) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the allocator, data pointer and size slots are uc's own
        // `ator_tmp`/`tmp_arr`/`tmp_arr_size` fields, reached through its
        // views — the matched triple `grow_array` requires.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_view(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                sets.len() * size_of::<UvSet>(),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_uv_set)))"
    );
    // SAFETY: `sets` carries a live `UvSet` run and `uc.tmp_arr()` was just
    // grown to the run's byte size, so both the input run and merge buffer are
    // in bounds; `uv_set_less` is the comparator for that element type.
    unsafe {
        stable_sort(
            size_of::<UvSet>(),
            32,
            sets.as_mut_ptr() as *mut c_void,
            uc.tmp_arr() as *mut c_void,
            sets.len(),
            uv_set_less,
            core::ptr::null_mut(),
        )
    };
    Ok(())
}

// ufbx.c:12983-12988 `ufbxi_sort_color_sets`
#[inline(never)]
pub(crate) fn sort_color_sets(uc: &Context, sets: Run<'_, ColorSet>) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the allocator, data pointer and size slots are uc's own
        // `ator_tmp`/`tmp_arr`/`tmp_arr_size` fields, reached through its
        // views — the matched triple `grow_array` requires.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_view(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                sets.len() * size_of::<ColorSet>(),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_color_set)))"
    );
    // SAFETY: `sets` carries a live `ColorSet` run and `uc.tmp_arr()` was just
    // grown to the run's byte size, so both the input run and merge buffer are
    // in bounds; `color_set_less` is the comparator for that element type.
    unsafe {
        stable_sort(
            size_of::<ColorSet>(),
            32,
            sets.as_mut_ptr() as *mut c_void,
            uc.tmp_arr() as *mut c_void,
            sets.len(),
            color_set_less,
            core::ptr::null_mut(),
        )
    };
    Ok(())
}

// ufbx.c:12990-12994 `ufbxi_blend_offset`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct BlendOffset {
    pub vertex: u32,
    pub position_offset: Vec3,
    pub normal_offset: Vec3,
}

// ufbx.c:12996-13001 `ufbxi_blend_offset_less`
#[inline(never)]
pub(crate) unsafe extern "C" fn blend_offset_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const BlendOffset = va as *const BlendOffset;
    let b: *const BlendOffset = vb as *const BlendOffset;
    // SAFETY: `stable_sort` invokes this `LessFn` with `va`/`vb` pointing at two
    // live elements of the `BlendOffset` run it was handed (see
    // `sort_blend_offsets`, whose stride is `size_of::<BlendOffset>()`).
    unsafe { (*a).vertex < (*b).vertex }
}

// ufbx.c:13003-13008 `ufbxi_sort_blend_offsets`
#[inline(never)]
pub(crate) fn sort_blend_offsets(uc: &Context, offsets: Run<'_, BlendOffset>) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the allocator, data pointer and size slots are uc's own
        // `ator_tmp`/`tmp_arr`/`tmp_arr_size` fields, reached through its
        // views — the matched triple `grow_array` requires.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_view(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                offsets.len() * size_of::<BlendOffset>(),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbxi_blend_offset)))"
    );
    // SAFETY: `offsets` carries a live `BlendOffset` run and `uc.tmp_arr()` was
    // just grown to the run's byte size, so both the input run and merge
    // buffer are in bounds; `blend_offset_less` is its comparator.
    unsafe {
        stable_sort(
            size_of::<BlendOffset>(),
            16,
            offsets.as_mut_ptr() as *mut c_void,
            uc.tmp_arr() as *mut c_void,
            offsets.len(),
            blend_offset_less,
            core::ptr::null_mut(),
        )
    };
    Ok(())
}

// ufbx.c:13010-13075 `ufbxi_read_shape`
#[inline(never)]
pub(crate) fn read_shape(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    let node_vertices = find_child(node, sp::Vertices.as_ptr());
    let node_indices = find_child(node, sp::Indexes.as_ptr());
    let node_normals = find_child(node, sp::Normals.as_ptr());
    if node_vertices.is_none() || node_indices.is_none() {
        return Ok(());
    }
    let node_vertices: &NodeView = node_vertices.unwrap();
    let node_indices: &NodeView = node_indices.unwrap();

    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `BlendShape` is the element struct for `ElementType::BlendShape`.
    let shape: *mut BlendShape =
        unsafe { push_element::<BlendShape>(uc, info, ElementType::BlendShape) };
    ufbxi_check!(uc, !shape.is_null(), "shape");
    // SAFETY: `shape` is the fresh non-null element just pushed into uc's
    // `tmp_elements` arena (elements live there until finalize copies them into
    // the result arena) — reached through `*mut` (write-capable provenance for
    // `Mut`) and live for the borrow; the fields read below are the ones this
    // function fills in first.
    let shape: &View<BlendShape> = unsafe { View::<BlendShape>::from_ptr(shape) };

    if uc.opts_view().ignore_geometry() {
        return Ok(());
    }

    let vertices: *mut ValueArray = get_array(node_vertices, b'r');
    let indices: *mut ValueArray = get_array(node_indices, b'i');

    ufbxi_check!(
        uc,
        !vertices.is_null() && !indices.is_null(),
        "vertices && indices"
    );
    // SAFETY: `vertices` is non-null (checked above) and `get_array` returns the
    // node's own array descriptor, live for as long as the parse tree and
    // reached through `*mut` (write-capable provenance for `Mut`).
    let vertices: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(vertices) };
    // SAFETY: as above, for the `'i'` descriptor, likewise non-null.
    let indices: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(indices) };
    ufbxi_check!(uc, vertices.size() % 3 == 0, "vertices->size % 3 == 0");
    ufbxi_check!(
        uc,
        indices.size() == vertices.size() / 3,
        "indices->size == vertices->size / 3"
    );

    // The `'i'` array's payload is a run of `size` `u32`s.
    let (num_offsets, vertex_indices): (usize, *mut u32) =
        (indices.size(), indices.data() as *mut u32);

    // `vertices`'s `'r'` payload is `size == num_offsets * 3` reals, i.e.
    // `num_offsets` `ufbx_vec3` values, and `vertex_indices` the parallel index
    // run; the C's field-write order is kept.
    shape.set_num_offsets(num_offsets);
    shape
        .position_offsets_view()
        .set_data(vertices.data() as *const Vec3);
    shape.offset_vertices_view().set_data(vertex_indices);
    shape.position_offsets_view().set_count(num_offsets);
    shape.offset_vertices_view().set_count(num_offsets);

    if let Some(node_normals) = node_normals {
        let normals: *mut ValueArray = get_array(node_normals, b'r');
        // SAFETY: the view is minted only in the non-null arm, where `normals`
        // is likewise a live parse-tree array descriptor reached through `*mut`.
        let normals: Option<&View<ValueArray>> = if normals.is_null() {
            None
        } else {
            Some(unsafe { View::<ValueArray>::from_ptr(normals) })
        };
        ufbxi_check!(
            uc,
            normals.is_some_and(|normals| normals.size() == vertices.size()),
            "normals && normals->size == vertices->size"
        );
        // The check above returned unless `normals` is a live descriptor whose
        // `'r'` payload matches `vertices`, i.e. `num_offsets` `ufbx_vec3`s.
        let normals: &View<ValueArray> = normals.unwrap();
        shape
            .normal_offsets_view()
            .set_data(normals.data() as *const Vec3);
        shape.normal_offsets_view().set_count(num_offsets);
    }

    // Sort the blend shape vertices only if absolutely necessary
    let mut sorted: bool = true;
    for i in 1..num_offsets {
        // `offset_vertices` is the `num_offsets`-long index run published
        // above, so both bounded reads are in bounds.
        let offset_vertices = shape.offset_vertices_view();
        if offset_vertices.copy_at(i - 1) > offset_vertices.copy_at(i) {
            sorted = false;
            break;
        }
    }

    if !sorted {
        let offsets: *mut BlendOffset = uc.tmp_stack_view().push::<BlendOffset>(num_offsets);
        ufbxi_check!(uc, !offsets.is_null(), "offsets");
        // SAFETY: `offsets` is the non-null `num_offsets`-element allocation
        // just pushed on uc's own `tmp_stack` — one contiguous, write-capable
        // run that stays alive and unmoved until the pop below (nothing pushes
        // or pops that buf in between; `sort_blend_offsets` only grows
        // `uc->tmp_arr`). Its slots are still uninitialized, which the run
        // tolerates.
        let offset_run: Run<'_, BlendOffset, Mut> =
            unsafe { Run::<BlendOffset, Mut>::from_raw_parts(offsets, num_offsets) };

        for i in 0..num_offsets {
            // The `shape` lists were published above over the parse-tree
            // payloads of `num_offsets` entries each, so the bounded reads are
            // in bounds; `at(i)` bounds the destination slot in the fresh run.
            // C: the three fields are assigned in this order, and
            // `normal_offset` is left untouched without a `Normals` node.
            // SAFETY: `at(i)` names a live, write-capable slot of the vouched
            // run; the slot need not be initialized, and these writes are what
            // initialize the fields the sort and the write-back read.
            unsafe {
                let dst: *mut BlendOffset = offset_run.at(i).get();
                (*dst).vertex = shape.offset_vertices_view().copy_at(i);
                (*dst).position_offset = shape.position_offsets_view().copy_at(i);
                if node_normals.is_some() {
                    (*dst).normal_offset = shape.normal_offsets_view().copy_at(i);
                }
            }
        }

        sort_blend_offsets(uc, offset_run)?;

        for i in 0..num_offsets {
            // The `shape` lists carry the same `num_offsets` count as the run,
            // and they point into the parse tree's own mutable array payloads,
            // so `at(i)` writes back through their original provenance.
            // SAFETY: `at(i)` names a live slot of the vouched run, whose
            // `vertex`/`position_offset` (and, with a `Normals` node,
            // `normal_offset`) were initialized by the fill loop above; only
            // those fields are read, and the sort permuted whole elements.
            unsafe {
                let src: *const BlendOffset = offset_run.at(i).get();
                shape
                    .offset_vertices_view()
                    .at(i)
                    .write_value((*src).vertex);
                shape
                    .position_offsets_view()
                    .at(i)
                    .write_value((*src).position_offset);
                if node_normals.is_some() {
                    shape
                        .normal_offsets_view()
                        .at(i)
                        .write_value((*src).normal_offset);
                }
            }
        }
        // SAFETY: the `num_offsets` `BlendOffset` values pushed above are
        // still the top of uc's own `tmp_stack` — `pop`'s depth obligation;
        // a null `dst` discards them.
        unsafe { pop::<BlendOffset>(uc.tmp_stack_view(), num_offsets, core::ptr::null_mut()) };
    }

    Ok(())
}

// Rust-port infrastructure (not a ufbx.c section): the read surface the
// per-element readers need on a `ufbxi_element_info` they receive as a view —
// the FBX id leaf, and the name and property table as sub-views.
pub(crate) type ElementInfoView = View<ElementInfo>;

impl ElementInfoView {
    #[inline(always)]
    pub(crate) fn fbx_id(&self) -> u64 {
        view_read!(self, fbx_id)
    }
    #[inline(always)]
    pub(crate) fn props_view(&self) -> &PropsView {
        view_project!(self, props)
    }
    #[inline(always)]
    pub(crate) fn name_view(&self) -> &StringView {
        view_project!(self, name)
    }
}

// ufbx.c:13077-13137 `ufbxi_read_synthetic_blend_shapes`
#[inline(never)]
pub(crate) fn read_synthetic_blend_shapes(
    uc: &Context,
    node: &NodeView,
    info: &ElementInfoView,
) -> Result<(), Fail> {
    let mut deformer: *mut BlendDeformer = core::ptr::null_mut();
    let mut deformer_fbx_id: u64 = 0;

    // C: `ufbxi_for (ufbxi_node, n, node->children, node->num_children)`
    let children = node.children_iter();
    for n in children {
        if n.name() != sp::Shape.as_ptr() {
            continue;
        }

        // C: `ufbx_string name;` — fully written by `ufbxi_get_val1` before any
        // read; zero-initialized here (no upstream `ufbxi_uninit` marker).
        // SAFETY: `ufbx_string` is a plain pointer/length pair, for which the
        // all-zero bit pattern is a valid (empty, null-data) value.
        let name: String = ufbxi_check_some!(
            uc,
            get_val1::<Checked<String>>(n),
            "ufbxi_get_val1(n, \"S\", &name)"
        )
        .0;

        if deformer.is_null() {
            // SAFETY: `deformer_fbx_id` is a live local `u64` out-slot;
            // `name.data` was written by the `'S'` fetch above as a
            // NUL-terminated parse-tree string; `BlendDeformer` is the element
            // struct for `ElementType::BlendDeformer`.
            deformer = unsafe {
                push_synthetic_element::<BlendDeformer>(
                    uc,
                    ScalarView::from_mut(&mut deformer_fbx_id),
                    Some(n),
                    name.data,
                    ElementType::BlendDeformer,
                )
            };
            ufbxi_check!(uc, !deformer.is_null(), "deformer");
            connect_oo(uc, deformer_fbx_id, info.fbx_id())?;
        }

        let mut channel_fbx_id: u64 = 0;
        // SAFETY: `channel_fbx_id` is a live local `u64` out-slot; `name.data`
        // is the NUL-terminated parse-tree string fetched above;
        // `BlendChannel` is the element struct for `ElementType::BlendChannel`.
        let channel: *mut BlendChannel = unsafe {
            push_synthetic_element::<BlendChannel>(
                uc,
                ScalarView::from_mut(&mut channel_fbx_id),
                Some(n),
                name.data,
                ElementType::BlendChannel,
            )
        };
        ufbxi_check!(uc, !channel.is_null(), "channel");

        // C: `ufbx_real_list weight_list = { NULL, 0 };`
        // SAFETY: `ufbx_real_list` is a plain pointer/count pair, for which the
        // all-zero bit pattern is a valid (empty, null-data) value.
        let weight_list: List<Real> = unsafe { core::mem::zeroed() };
        ufbxi_check!(
            uc,
            !uc.tmp_full_weights_view()
                .push_copy_ref(&weight_list)
                .is_null(),
            "((ufbx_real_list*)ufbxi_push_size_copy((&uc->tmp_full_weights), sizeof(ufbx_real_list), (1), (&weight_list)))"
        );

        let num_shape_props: usize = 1;
        let shape_props: *mut Prop = uc.result_view().push_zero::<Prop>(num_shape_props);
        ufbxi_check!(uc, !shape_props.is_null(), "shape_props");
        // C-parity: `shape_props[0].value_real` is the `ufbx_prop` value
        // union's first real (`value_vec4.x` in the generated struct).
        // SAFETY: `shape_props` is the non-null run of `num_shape_props == 1`
        // zeroed `ufbx_prop`s pushed above, so index `0` is in bounds;
        // `sp::DeformPercent` is a NUL-terminated static — `get_name_key_c`'s
        // contract.
        unsafe {
            (*shape_props.add(0)).name.data = sp::DeformPercent.as_ptr();
            (*shape_props.add(0)).name.length = sp::DeformPercent.len() - 1;
            (*shape_props.add(0))._internal_key = get_name_key(&sp::DeformPercent);
            (*shape_props.add(0)).type_ = PropType::Number;
            (*shape_props.add(0)).value_vec4.x = 0.0 as Real;
            (*shape_props.add(0)).value_str = EMPTY_STRING.0;
            (*shape_props.add(0)).value_blob = EMPTY_BLOB.0;
        }

        // SAFETY: `name` is the interned string fetched above, readable for its
        // length (`as_bytes`).
        let self_prop: Option<&PropView> =
            find_prop_len(info.props_view(), unsafe { name.as_bytes() });
        if self_prop.is_some_and(|prop| {
            prop.type_() == PropType::Number || prop.type_() == PropType::Integer
        }) {
            // `is_some_and` above guarantees `self_prop` is `Some`.
            // SAFETY: index `0` of the pushed `ufbx_prop` run.
            unsafe {
                (*shape_props.add(0)).value_vec4.x = self_prop.unwrap().value_vec4().x;
            }
            // SAFETY: index `0` of the pushed `ufbx_prop` run holds the name
            // set above.
            unsafe {
                connect_pp(
                    uc,
                    info.fbx_id(),
                    channel_fbx_id,
                    name,
                    (*shape_props.add(0)).name,
                )
            }?;
        } else if uc.version() < 6000 {
            // SAFETY: as the branch above.
            unsafe {
                connect_pp(
                    uc,
                    info.fbx_id(),
                    channel_fbx_id,
                    name,
                    (*shape_props.add(0)).name,
                )
            }?;
        }

        // SAFETY: `channel` is the fresh non-null element checked above, and
        // `shape_props` is the `num_shape_props`-long run pushed on the result
        // arena, which outlives the scene.
        unsafe {
            (*channel).element.name = name;
            (*channel).element.props.props.data = shape_props;
            (*channel).element.props.props.count = num_shape_props;
        }

        // C: `ufbxi_element_info shape_info = { 0 };`
        // SAFETY: `ufbxi_element_info` holds only integers, pointer/length
        // pairs and a nullable dom-node pointer, so all-zero is a valid value.
        let mut shape_info: ElementInfo = unsafe { core::mem::zeroed() };

        shape_info.fbx_id = push_synthetic_id(uc);
        shape_info.name = name;
        shape_info.dom_node = get_dom_node(uc, Some(n));

        read_shape(uc, n, View::<ElementInfo, Mut>::from_mut(&mut shape_info))?;

        connect_oo(uc, channel_fbx_id, deformer_fbx_id)?;
        connect_oo(uc, shape_info.fbx_id, channel_fbx_id)?;
    }

    Ok(())
}

// ufbx.c:13139-13217 `ufbxi_process_indices`
#[inline(never)]
pub(crate) fn process_indices(uc: &Context, mesh: &View<Mesh>) -> Result<(), Fail> {
    // The mesh header describes the mutable polygon-index run installed by the
    // reader; deriving the carrier here keeps its count tied to
    // `mesh->num_indices` and preserves null-with-zero.
    let index_list = mesh.vertex_indices_view();
    let index_data = Run::from_list(index_list);

    // Count the number of faces and allocate the index list
    // Indices less than zero (~actual_index) ends a polygon
    let mut num_total_faces: usize = 0;
    // C: `ufbxi_for (uint32_t, p_ix, index_data, mesh->num_indices)`
    let mut index_ix: usize = 0;
    while index_ix < index_data.len() {
        num_total_faces =
            num_total_faces.wrapping_add(if (index_list.copy_at(index_ix) as i32) < 0 {
                1usize
            } else {
                0usize
            });
        index_ix += 1;
    }
    mesh.faces_view()
        .set_data(uc.result_view().push::<Face>(num_total_faces));
    ufbxi_check!(uc, !mesh.faces().data.is_null(), "mesh->faces.data");

    // SAFETY: `mesh.faces.data` is the checked result-arena allocation of
    // `num_total_faces` contiguous, write-capable `Face` slots. The slots are
    // deliberately still uninitialized: the local Run permits that, while
    // `mesh.faces.count` remains unpublished until every slot is written.
    let faces = unsafe {
        Run::<Face, Mut>::from_raw_parts(mesh.faces().data as *mut Face, num_total_faces)
    };

    let mut num_triangles: usize = 0;
    let mut max_face_triangles: usize = 0;
    let mut num_bad_faces: [usize; 3] = [0; 3];

    let mut face_ix: usize = 0;
    let mut face_begin_ix: usize = 0;
    // C: `ufbxi_for (uint32_t, p_ix, index_data, mesh->num_indices)`
    let mut index_ix: usize = 0;
    while index_ix < index_data.len() {
        let mut ix: u32 = index_list.copy_at(index_ix);
        // Un-negate final indices of polygons
        if (ix as i32) < 0 {
            ix = !ix;
            index_data.write_at(index_ix, ix);
            let num_indices: u32 = (index_ix - face_begin_ix + 1) as u32;
            faces.write_at(
                face_ix,
                Face {
                    index_begin: face_begin_ix as u32,
                    num_indices,
                },
            );
            if num_indices >= 3 {
                num_triangles = num_triangles.wrapping_add((num_indices - 2) as usize);
                max_face_triangles = max_sz(max_face_triangles, (num_indices - 2) as usize);
            } else {
                num_bad_faces[num_indices as usize] =
                    num_bad_faces[num_indices as usize].wrapping_add(1);
            }
            face_ix += 1;
            face_begin_ix = index_ix + 1;
        }
        ufbxi_check!(
            uc,
            (ix as usize) < mesh.num_vertices(),
            "(size_t)ix < mesh->num_vertices"
        );
        index_ix += 1;
    }

    mesh.vertex_position()
        .indices_view()
        .set_data(index_data.as_ptr());
    mesh.set_num_faces(to_size(face_ix as isize));
    mesh.faces_view().set_count(mesh.num_faces());
    mesh.set_num_triangles(num_triangles);
    mesh.set_max_face_triangles(max_face_triangles);
    mesh.set_num_empty_faces(num_bad_faces[0]);
    mesh.set_num_point_faces(num_bad_faces[1]);
    mesh.set_num_line_faces(num_bad_faces[2]);

    mesh.vertex_first_index_view()
        .set_count(mesh.num_vertices());
    mesh.vertex_first_index_view()
        .set_data(uc.result_view().push::<u32>(mesh.num_vertices()));
    ufbxi_check!(
        uc,
        !mesh.vertex_first_index().data.is_null(),
        "mesh->vertex_first_index.data"
    );

    // C: `ufbxi_for_list(uint32_t, p_vx_ix, mesh->vertex_first_index)`
    // The stored count/data pair is a checked result-arena run.
    let vertex_first_index_list = mesh.vertex_first_index_view();
    let vertex_first_index = Run::from_list(vertex_first_index_list);
    let mut vertex_ix: usize = 0;
    while vertex_ix < vertex_first_index.len() {
        vertex_first_index.write_at(vertex_ix, NO_INDEX);
        vertex_ix += 1;
    }

    {
        let num_vertices: usize = mesh.num_vertices();
        let mut ix: usize = 0;
        while ix < index_data.len() {
            let vx: u32 = index_list.copy_at(ix);
            if (vx as usize) < num_vertices {
                if vertex_first_index_list.copy_at(vx as usize) == NO_INDEX {
                    vertex_first_index.write_at(vx as usize, ix as u32);
                }
            } else {
                fix_index(uc, index_data.at(ix), vx, mesh.num_vertices())?;
            }
            ix += 1;
        }
    }

    // HACK(consecutive-faces): Prepare for finalize to re-use a consecutive/zero
    // index buffer for face materials..
    uc.set_max_zero_indices(max_sz(uc.max_zero_indices(), mesh.num_faces()));
    uc.set_max_consecutive_indices(max_sz(uc.max_consecutive_indices(), mesh.num_faces()));

    Ok(())
}

// ufbx.c:13219-13240 `ufbxi_patch_mesh_reals`
#[inline(never)]
pub(crate) fn patch_mesh_reals(mesh: &View<Mesh>) {
    mesh.vertex_position().set_value_reals(3);
    mesh.vertex_normal().set_value_reals(3);
    mesh.vertex_uv().set_value_reals(2);
    mesh.vertex_tangent().set_value_reals(3);
    mesh.vertex_bitangent().set_value_reals(3);
    mesh.vertex_color().set_value_reals(4);
    mesh.vertex_crease().set_value_reals(1);
    mesh.skinned_position().set_value_reals(3);
    mesh.skinned_normal().set_value_reals(3);

    // C: `ufbxi_nounroll ufbxi_for_list(ufbx_uv_set, set, mesh->uv_sets)`
    for set in Run::from_list(mesh.uv_sets_view()).iter() {
        set.vertex_uv().set_value_reals(2);
        set.vertex_tangent().set_value_reals(3);
        set.vertex_bitangent().set_value_reals(3);
    }

    // C: `ufbxi_nounroll ufbxi_for_list(ufbx_color_set, set, mesh->color_sets)`
    for set in Run::from_list(mesh.color_sets_view()).iter() {
        set.vertex_color().set_value_reals(4);
    }
}

// ufbx.c:13242-13244 `typedef struct { uint32_t id, index; } ufbxi_id_group;`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct IdGroup {
    pub id: u32,
    pub index: u32,
}

// ufbx.c:13246-13251 `ufbxi_less_int32`
pub(crate) unsafe extern "C" fn less_int32(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    // SAFETY: `unstable_sort` invokes this `LessFn` with `va`/`vb` pointing at
    // two live elements of the run it was handed — the `uint32_t` id run in
    // `assign_face_groups`, whose stride is `size_of::<u32>()`; C-parity reads
    // each element as `int32_t`, which has the same size and alignment.
    let a: i32 = unsafe { *(va as *const i32) };
    // SAFETY: as `va` above.
    let b: i32 = unsafe { *(vb as *const i32) };
    a < b
}

// ufbx.c:13253 `ufbx_static_assert(mesh_mat_point_faces, ...)`
const _: () = assert!(
    core::mem::offset_of!(MeshPart, num_point_faces)
        - core::mem::offset_of!(MeshPart, num_empty_faces)
        == size_of::<usize>()
);
// ufbx.c:13254 `ufbx_static_assert(mesh_mat_line_faces, ...)`
const _: () = assert!(
    core::mem::offset_of!(MeshPart, num_line_faces)
        - core::mem::offset_of!(MeshPart, num_empty_faces)
        == 2 * size_of::<usize>()
);

// ufbx.c:13255-13265 `ufbxi_mesh_part_add_face`
#[inline(always)]
pub(crate) fn mesh_part_add_face(part: &View<MeshPart>, num_indices: u32) {
    part.set_num_faces(part.num_faces().wrapping_add(1));
    if num_indices >= 3 {
        part.set_num_triangles(
            part.num_triangles()
                .wrapping_add((num_indices - 2) as usize),
        );
    } else {
        // `num_empty/point/line_faces` are consecutive, see static asserts above.
        // cppcheck-suppress objectIndex
        // C-parity: indexing off one field into its two siblings (the static
        // asserts above pin the offsets); ported as pointer arithmetic from the
        // field address rather than a match on `num_indices`.
        let p_empty: *mut usize = part.num_empty_faces_raw();
        // SAFETY: this branch has `num_indices < 3`, and the static asserts
        // above pin `num_point_faces`/`num_line_faces` at exactly one and two
        // `usize` past `num_empty_faces`, so the offset stays inside `part`.
        let p: *mut usize = unsafe { p_empty.add(num_indices as usize) };
        // SAFETY: `p` addresses one of those three live `usize` fields.
        unsafe { *p = (*p).wrapping_add(1) };
    }
}

// C: `ufbxi_id_group seen_ids[1 << UFBXI_FACE_GROUP_HASH_BITS];`
const SEEN_IDS_COUNT: usize = 1usize << FACE_GROUP_HASH_BITS;

// ufbx.c:13267-13397 `ufbxi_assign_face_groups`
#[inline(never)]
pub(crate) fn assign_face_groups(
    buf: &BufView,
    error: &crate::native::error::ErrorView,
    mesh: &View<Mesh>,
    p_consecutive_indices: Option<&ScalarView<usize>>,
    retain_parts: bool,
) -> Result<(), Fail> {
    let num_faces: usize = mesh.num_faces();
    ufbxi_check_err!(error, num_faces > 0);
    ufbxi_check_err!(
        error,
        num_faces < u32::MAX as usize,
        "num_faces < UINT32_MAX"
    );
    ufbxi_check_err!(
        error,
        mesh.face_group().count == num_faces,
        "mesh->face_group.count == num_faces"
    );

    let ids: *mut u32 = buf.push::<u32>(num_faces);
    ufbxi_check_err!(error, !ids.is_null(), "ids");

    let mut num_ids: u32 = 0;

    // C declares `seen_ids` uninitialized and zeroes it with the `memset` below.
    // SAFETY: `ufbxi_id_group` is a pair of `u32`s, for which the all-zero bit
    // pattern is a valid value.
    let mut seen_ids: [IdGroup; SEEN_IDS_COUNT] = unsafe { core::mem::zeroed() };
    // SAFETY: `seen_ids` is a live local array of exactly `SEEN_IDS_COUNT`
    // `ufbxi_id_group`s, so the zero fill covers exactly it.
    unsafe { core::ptr::write_bytes(seen_ids.as_mut_ptr(), 0, SEEN_IDS_COUNT) };

    let mut seed: u32 = 2654435769u32;
    let mut rehash_threshold: u32 = 256;

    // The mesh's own `count`-long run of `uint32_t` group IDs, as per-element
    // scalar handles: C walks it three times (`ufbxi_for_list`), reading and
    // rewriting entries in place.
    // SAFETY: `face_group` is a contiguous run of `count` live, initialized
    // `uint32_t`s owned by the mesh, reached through a write-capable mesh view;
    // the only write to that memory NOT going through these handles is the
    // `memset` on the single-group path below, which returns immediately.
    let face_group: &[ScalarView<u32>] = unsafe {
        slice_from_ptr(
            mesh.face_group().data as *const ScalarView<u32>,
            mesh.face_group().count,
        )
    };

    // Loosely deduplicate group IDs
    // C: `ufbxi_for_list(uint32_t, p_id, mesh->face_group)`
    for p_id in face_group {
        let id: u32 = p_id.get();
        let id_hash: u32 = id.wrapping_mul(seed) >> (32u32 - FACE_GROUP_HASH_BITS);
        let slot = &mut seen_ids[id_hash as usize];
        if slot.id != id || slot.index == 0 {
            slot.id = id;
            // C: `if (++seen_ids[id_hash].index > rehash_threshold)`
            slot.index = slot.index.wrapping_add(1);
            if slot.index > rehash_threshold {
                seed = seed.wrapping_mul(seed);
                rehash_threshold = rehash_threshold.wrapping_mul(2);
            }
            // C: `ids[num_ids++] = id;`
            // SAFETY: `ids` is the non-null `num_faces`-long run pushed above;
            // this write happens at most once per `face_group` entry and
            // `face_group.count == num_faces`, so `num_ids < num_faces`. The
            // run is uninitialized until written, so it is filled through the
            // raw pointer — a scalar handle's `set` would read the old value.
            unsafe { *ids.add(num_ids as usize) = id };
            num_ids = num_ids.wrapping_add(1);
        }
    }

    // Sort and deduplicate remaining IDs
    // SAFETY: `ids` spans `num_faces >= num_ids` live `u32`s and `less_int32`
    // compares elements of exactly that stride.
    unsafe {
        unstable_sort(
            ids as *mut c_void,
            num_ids as usize,
            size_of::<u32>(),
            less_int32,
            core::ptr::null_mut(),
        )
    };

    // The written prefix of `ids`, as per-element scalar handles.
    // SAFETY: the loop above initialized `ids[0..num_ids]` and the sort left
    // exactly that prefix live and initialized; the handles are minted after
    // the sort's own writes through `ids`, and every later parent write to the
    // run happens after their last use.
    let id_slots: &[ScalarView<u32>] =
        unsafe { slice_from_ptr(ids as *const ScalarView<u32>, num_ids as usize) };

    let mut num_groups: usize = 0;
    let mut i: usize = 0;
    while i < num_ids as usize {
        let id: u32 = id_slots[i].get();
        // C: `ids[num_groups++] = id;` — `num_groups <= i < num_ids` since it
        // advances at most once per outer iteration.
        id_slots[num_groups].set(id);
        num_groups += 1;
        // C: `do { i++; } while (i < num_ids && ids[i] == id);`
        loop {
            i += 1;
            if !(i < num_ids as usize && id_slots[i].get() == id) {
                break;
            }
        }
    }

    // Allocate group info structs
    let groups: *mut FaceGroup = buf.push_zero::<FaceGroup>(num_groups);
    ufbxi_check_err!(error, !groups.is_null(), "groups");
    // C: `for (size_t i = 0; i < num_groups; i++)` — `num_groups <= num_ids <=
    // num_faces`, so the zip stops at `num_groups`.
    // SAFETY: `groups` is the non-null contiguous `num_groups`-long
    // `ufbx_face_group` run just pushed into `buf`, live for this call.
    let group_views = unsafe { SliceViewIter::<FaceGroup>::from_raw_parts(groups, num_groups) };
    for (group, id) in group_views.zip(id_slots.iter()) {
        group.set_id(id.get() as i32);
        group.name_view().set(EMPTY_STRING.0);
    }

    // `groups` is the `num_groups`-long run pushed on `buf`.
    mesh.face_groups_view().set_data(groups);
    mesh.face_groups_view().set_count(num_groups);

    let mut parts: *mut MeshPart = core::ptr::null_mut();
    if retain_parts {
        parts = buf.push_zero::<MeshPart>(num_groups);
        ufbxi_check_err!(error, !parts.is_null(), "parts");
        // `parts` is the non-null `num_groups`-long run just pushed on `buf`.
        mesh.face_group_parts_view().set_data(parts);
        mesh.face_group_parts_view().set_count(num_groups);
    }

    // Optimization: Use `consecutive_indices` for a single group
    // C: `if (p_consecutive_indices && num_groups == 1)` — the nullable slot's
    // presence is the `Option` discriminant, tested first as C short-circuits.
    if let Some(p_consecutive_indices) = p_consecutive_indices {
        if num_groups == 1 {
            // SAFETY: `face_group` was checked above to hold exactly `num_faces`
            // writable `uint32_t`s.
            unsafe { core::ptr::write_bytes(mesh.face_group().data as *mut u32, 0, num_faces) };

            if !parts.is_null() {
                // C: `parts[0]` — `num_groups == 1` here.
                // SAFETY: `parts` is the non-null `num_groups`-long run pushed
                // above, so it addresses a live, initialized `ufbx_mesh_part`.
                let part: &View<MeshPart> = unsafe { View::<MeshPart>::from_ptr(parts) };
                part.face_indices_view()
                    .set_data(SENTINEL_INDEX_CONSECUTIVE.as_ptr());
                part.face_indices_view().set_count(num_faces);
                part.set_num_empty_faces(mesh.num_empty_faces());
                part.set_num_point_faces(mesh.num_point_faces());
                part.set_num_line_faces(mesh.num_line_faces());
                part.set_num_faces(num_faces);
                part.set_num_triangles(mesh.num_triangles());
            }

            p_consecutive_indices.set(max_sz(p_consecutive_indices.get(), num_faces));
            return Ok(());
        }
    }

    // SAFETY: `seen_ids` is a live local array of exactly `SEEN_IDS_COUNT`
    // `ufbxi_id_group`s, so the zero fill covers exactly it.
    unsafe { core::ptr::write_bytes(seen_ids.as_mut_ptr(), 0, SEEN_IDS_COUNT) };

    // Count faces and triangles per group and reassign IDs
    // C: `const ufbx_face *p_face = mesh->faces.data;` — advanced in lockstep
    // with the `face_group` walk below, whose count equals `num_faces`.
    // SAFETY: `faces` is the mesh's own contiguous run of `num_faces` live,
    // initialized `ufbx_face`s.
    let p_faces =
        unsafe { SliceViewIter::<Face>::from_raw_parts(mesh.faces().data as *mut Face, num_faces) };
    // `face_groups` holds the `groups` run assigned above; `face_group_parts`
    // holds `parts` where `retain_parts` is set.
    let face_groups = mesh.face_groups_view();
    let part_list = mesh.face_group_parts_view();
    // C: `ufbxi_for_list(uint32_t, p_id, mesh->face_group)`
    for (p_id, p_face) in face_group.iter().zip(p_faces) {
        let id: u32 = p_id.get();
        let id_hash: u32 = id.wrapping_mul(seed) >> (32u32 - FACE_GROUP_HASH_BITS);

        let num_indices: u32 = p_face.num_indices();

        let mut index: usize;
        if seen_ids[id_hash as usize].id == id && seen_ids[id_hash as usize].index > 0 {
            index = (seen_ids[id_hash as usize].index - 1) as usize;
            p_id.set(index as u32);
        } else {
            let signed_id: i32 = id as i32;
            index = usize::MAX;
            // C: `ufbxi_macro_lower_bound_eq(ufbx_face_group, 8, &index, groups,
            // 0, num_groups, ...)` — the search window is the whole `groups`
            // run, which `mesh->face_groups` was just set to. C leaves `index`
            // untouched on a miss.
            if let Some(found) =
                face_groups.lower_bound_eq(8, |a| a.id() < signed_id, |a| a.id() == signed_id)
            {
                index = found;
            }
            ufbx_assert!(index < num_groups);
            seen_ids[id_hash as usize].id = id;
            seen_ids[id_hash as usize].index = (index as u32).wrapping_add(1);
        }

        if !parts.is_null() {
            // C: `ufbxi_mesh_part_add_face(&parts[index], num_indices);`
            mesh_part_add_face(part_list.at(index), num_indices);
        }

        p_id.set(index as u32);
    }

    if parts.is_null() {
        return Ok(());
    }

    // Subdivide `ids` for per-group `face_indices`
    let mut face_indices: *mut u32 = ids;
    let mut part_index: u32 = 0;
    // C: `ufbxi_for(ufbx_mesh_part, part, parts, num_groups)`
    // SAFETY: `parts` is the contiguous `num_groups`-long `ufbx_mesh_part` run
    // pushed into `buf` above, live for this call.
    for part in unsafe { SliceViewIter::<MeshPart>::from_raw_parts(parts, num_groups) } {
        part.set_index(part_index);
        part_index = part_index.wrapping_add(1);
        // `face_indices` sub-divides the `num_faces`-long `ids` run, whose total
        // length is the sum of the parts' `num_faces`.
        part.face_indices_view().set_data(face_indices);
        face_indices = add_ptr(face_indices, part.num_faces());
    }
    ufbx_assert!(face_indices == add_ptr(ids, num_faces));

    // Collect per-group faces
    let mut face_index: u32 = 0;
    // C: `ufbxi_for_list(uint32_t, p_id, mesh->face_group)`
    for p_id in face_group {
        // C: `ufbx_mesh_part *part = &parts[*p_id];` — the loop above rewrote
        // every entry to a group index below `num_groups`.
        let part_face_indices = part_list.at(p_id.get() as usize).face_indices_view();
        // C: `part->face_indices.data[part->face_indices.count++] = face_index++;`
        // SAFETY: the part's `face_indices` is the sub-range of `ids` assigned
        // above, sized to the part's `num_faces`; `count` starts at zero and is
        // bumped once per face belonging to this part, so it stays within it.
        unsafe {
            *(part_face_indices.data() as *mut u32).add(part_face_indices.count()) = face_index;
        }
        part_face_indices.set_count(part_face_indices.count() + 1);
        face_index = face_index.wrapping_add(1);
    }

    Ok(())
}

// ufbx.c:13399-13432 `ufbxi_update_face_groups`
#[inline(never)]
pub(crate) fn update_face_groups(
    buf: &BufView,
    error: &crate::native::error::ErrorView,
    mesh: &View<Mesh>,
    need_copy: bool,
) -> Result<(), Fail> {
    let num_faces: usize = mesh.faces().count;
    let num_groups: usize = mesh.face_group_parts().count;
    if num_groups == 0 {
        return Ok(());
    }

    if need_copy {
        mesh.face_group_parts_view()
            .set_data(buf.push_zero::<MeshPart>(num_groups));
        ufbxi_check_err!(
            error,
            !mesh.face_group_parts().data.is_null(),
            "mesh->face_group_parts.data"
        );
    }

    let mut face_indices: *mut u32 = buf.push::<u32>(num_faces);
    ufbxi_check_err!(error, !face_indices.is_null(), "face_indices");

    // C: `ufbxi_nounroll for (size_t i = 0; i < num_faces; i++)`
    for i in 0..num_faces {
        // `i < num_faces` bounds the parallel per-face runs, and every
        // `face_group` entry is a group index below
        // `face_group_parts.count == num_groups`.
        let group_ix = mesh.face_group_view().copy_at(i) as usize;
        let part: &View<MeshPart> = mesh.face_group_parts_view().at(group_ix);
        let num_indices: u32 = mesh.faces_view().at(i).num_indices();
        mesh_part_add_face(part, num_indices);
    }

    let mut part_index: u32 = 0;
    // C: `ufbxi_for_list(ufbx_mesh_part, part, mesh->face_group_parts)`
    for part in Run::from_list(mesh.face_group_parts_view()).iter() {
        part.set_index(part_index);
        part_index = part_index.wrapping_add(1);
        // SAFETY: `face_indices_raw()` addresses the part's own index list;
        // `face_indices` sub-divides the `num_faces`-long run pushed above,
        // whose length is the sum of the parts' `num_faces` counted by the loop
        // above.
        unsafe {
            (*part.face_indices_raw()).data = face_indices;
            (*part.face_indices_raw()).count = 0;
        }
        face_indices = add_ptr(face_indices, part.num_faces());
    }

    // C: `ufbxi_nounroll for (uint32_t i = 0; i < num_faces; i++)`
    let mut i: u32 = 0;
    while (i as usize) < num_faces {
        // As in the counting loop, `i < num_faces` and the stored group index
        // is below `num_groups`.
        let group_ix = mesh.face_group_view().copy_at(i as usize) as usize;
        let part: &View<MeshPart> = mesh.face_group_parts_view().at(group_ix);
        // C: `part->face_indices.data[part->face_indices.count++] = i;`
        // SAFETY: `part`'s `face_indices` is the sub-range assigned above,
        // sized to the part's `num_faces`; `count` was reset to zero and is
        // bumped once per face belonging to this part, so it stays within it.
        unsafe {
            *(part.face_indices().data as *mut u32).add(part.face_indices().count) = i;
            (*part.face_indices_raw()).count += 1;
        }
        i = i.wrapping_add(1);
    }

    Ok(())
}

// C casts the guarded `int32_t` straight to `ufbx_subdivision_display_mode`;
// Rust needs an explicit mapping. The caller's guard (`smoothness >= 0 &&
// smoothness <= UFBX_SUBDIVISION_DISPLAY_SMOOTH`) leaves 0..3.
#[inline(always)]
fn subdivision_display_mode_from_raw(raw: i32) -> SubdivisionDisplayMode {
    match raw {
        0 => SubdivisionDisplayMode::Disabled,
        1 => SubdivisionDisplayMode::Hull,
        2 => SubdivisionDisplayMode::HullAndSmooth,
        _ => SubdivisionDisplayMode::Smooth,
    }
}

// C casts the guarded `int32_t` straight to `ufbx_subdivision_boundary`; Rust
// needs an explicit mapping. The caller's guard (`boundary >= 0 && boundary <=
// UFBX_SUBDIVISION_BOUNDARY_SHARP_CORNERS - 1`) leaves 0..1, and the value
// stored is `boundary + 1`, ie. 1..2.
#[inline(always)]
fn subdivision_boundary_from_raw(raw: i32) -> SubdivisionBoundary {
    match raw {
        1 => SubdivisionBoundary::Legacy,
        _ => SubdivisionBoundary::SharpCorners,
    }
}

// ufbx.c:13434-13811 `ufbxi_read_mesh`
// C-parity: the C local is `ufbx_mesh *ufbxi_restrict mesh` — `restrict` has no
// Rust analogue and collapses away.
#[inline(never)]
pub(crate) fn read_mesh(uc: &Context, node: &NodeView, info: &ElementInfoView) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Mesh` is the element struct for `ElementType::Mesh`.
    let mesh: *mut Mesh = unsafe { push_element::<Mesh>(uc, info, ElementType::Mesh) };
    ufbxi_check!(uc, !mesh.is_null(), "mesh");
    // SAFETY: `mesh` is the fresh non-null element just pushed into uc's
    // `tmp_elements` arena (elements live there until finalize copies them into
    // the result arena) — reached through `*mut` (write-capable provenance for
    // `Mut`) and live for the borrow; the fields accessed below are initialized
    // at each use site, as this function fills them in.
    let mesh = unsafe { View::<Mesh>::from_ptr(mesh) };

    // In up to version 7100 FBX files blend shapes are contained within the same geometry node
    if uc.version() <= 7100 {
        read_synthetic_blend_shapes(uc, node, info)?;
    }

    patch_mesh_reals(mesh);

    // Sometimes there are empty meshes in FBX files?
    // TODO: Should these be included in output? option? strict mode?
    let node_vertices = find_child(node, sp::Vertices.as_ptr());
    let node_indices = find_child(node, sp::PolygonVertexIndex.as_ptr());
    if node_vertices.is_none() {
        return Ok(());
    }
    let node_vertices: &NodeView = node_vertices.unwrap();

    if uc.opts_view().ignore_geometry() {
        return Ok(());
    }

    let vertices: *mut ValueArray = get_array(node_vertices, b'r');
    let indices: *mut ValueArray = if let Some(node_indices) = node_indices {
        get_array(node_indices, b'i')
    } else {
        core::ptr::null_mut()
    };
    let edge_indices: *mut ValueArray = find_array(node, sp::Edges.as_ptr(), b'i');
    ufbxi_check!(uc, !vertices.is_null(), "vertices");
    // If node_indices exists, it must be an array
    ufbxi_check!(
        uc,
        node_indices.is_none() || !indices.is_null(),
        "!node_indices || indices"
    );
    // SAFETY: `vertices` is non-null (checked above) and `get_array` returns the
    // node's own array descriptor, live for as long as the parse tree and
    // reached through `*mut` (write-capable provenance for `Mut`).
    let vertices: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(vertices) };
    // SAFETY: the view is minted only in the non-null arm, where `indices` is
    // likewise a live parse-tree array descriptor reached through `*mut`.
    let indices: Option<&View<ValueArray>> = if indices.is_null() {
        None
    } else {
        Some(unsafe { View::<ValueArray>::from_ptr(indices) })
    };
    ufbxi_check!(uc, vertices.size() % 3 == 0, "vertices->size % 3 == 0");

    mesh.set_num_vertices(vertices.size() / 3);
    mesh.set_num_indices(if let Some(indices) = indices {
        indices.size()
    } else {
        0
    });

    // The `'i'` array's payload is a run of `size` `u32`s.
    let mut index_data: *mut u32 = if let Some(indices) = indices {
        indices.data() as *mut u32
    } else {
        core::ptr::null_mut()
    };

    // Duplicate `index_data` for modification if we retain DOM
    if uc.opts_view().retain_dom() {
        // SAFETY: `uc.result_mut_ptr()` is uc's own live result buf, and
        // `index_data` spans `mesh->num_indices` `u32`s (it is the `'i'` array's
        // payload, or null when `num_indices` is 0).
        index_data = unsafe {
            uc.result_view()
                .push_copy_raw::<u32>(mesh.num_indices(), index_data)
        };
        ufbxi_check!(uc, !index_data.is_null(), "index_data");
    }

    // The `'r'` payload of `vertices` is `size == num_vertices * 3` reals, i.e.
    // `num_vertices` `ufbx_vec3` values.
    mesh.vertices_view()
        .set_data(vertices.data() as *const Vec3);
    mesh.vertices_view().set_count(mesh.num_vertices());
    mesh.vertex_indices_view().set_data(index_data);
    mesh.vertex_indices_view().set_count(mesh.num_indices());

    mesh.vertex_position().set_exists(true);
    mesh.vertex_position()
        .values_view()
        .set_data(vertices.data() as *const Vec3);
    mesh.vertex_position()
        .values_view()
        .set_count(mesh.num_vertices());
    mesh.vertex_position().indices_view().set_data(index_data);
    mesh.vertex_position()
        .indices_view()
        .set_count(mesh.num_indices());
    mesh.vertex_position().set_unique_per_vertex(true);

    // Check/make sure that the last index is negated (last of polygon)
    let vertex_indices = mesh.vertex_indices_view();
    if mesh.num_indices() > 0 {
        let last = mesh.num_indices() - 1;
        if vertex_indices.copy_at(last) as i32 >= 0 {
            if uc.opts_view().strict() {
                ufbxi_fail!(uc, "Non-negated last index");
            }
            let value = vertex_indices.copy_at(last);
            vertex_indices.at(last).write_value(!value);
        }
    }

    // Read edges before un-negating the indices
    if !edge_indices.is_null() {
        // SAFETY: `edge_indices` is non-null (checked) and `find_array` returns
        // a node's own array descriptor, live for as long as the parse tree and
        // reached through `*mut` (write-capable provenance for `Mut`).
        let edge_indices: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(edge_indices) };
        let num_edges: usize = edge_indices.size();
        let edges: *mut Edge = uc.result_view().push::<Edge>(num_edges);
        ufbxi_check!(uc, !edges.is_null(), "edges");

        // SAFETY: the value array supplies `num_edges` initialized indices and
        // `edges` is the checked fresh result-arena allocation of the same
        // length. The output count remains unpublished until the filled prefix
        // has been initialized.
        let (edge_data, edges_write) = unsafe {
            (
                Run::<u32, Const>::from_const_raw_parts(
                    edge_indices.data() as *const u32,
                    num_edges,
                ),
                Run::<Edge>::from_raw_parts(edges, num_edges),
            )
        };

        let mut dst_ix: usize = 0;

        // Edges are represented using a single index into PolygonVertexIndex.
        // The edge is between two consecutive vertices in the polygon.
        // The `'i'` array's payload is a run of `size` `u32`s.
        for i in 0..num_edges {
            let mut index_ix: u32 = edge_data.copy_at(i);
            if index_ix as usize >= mesh.num_indices() {
                if uc.opts_view().strict() {
                    ufbxi_fail!(uc, "Edge index out of bounds");
                }
                continue;
            }
            let edge = edges_write.at(dst_ix);
            edge.set_a(index_ix);
            if (vertex_indices.copy_at(index_ix as usize) as i32) < 0 {
                // Previous index is the last one of this polygon, rewind to first index.
                while index_ix > 0 && vertex_indices.copy_at(index_ix as usize - 1) as i32 >= 0 {
                    index_ix = index_ix.wrapping_sub(1);
                }
            } else {
                // Connect to the next index in the same polygon
                index_ix = index_ix.wrapping_add(1);
            }
            ufbxi_check!(
                uc,
                (index_ix as usize) < mesh.num_indices(),
                "index_ix < mesh->num_indices"
            );
            edge.set_b(index_ix);
            dst_ix += 1;
        }

        // `edges` is the `num_edges`-long result-arena run, of which `dst_ix`
        // were filled.
        mesh.edges_view().set_data(edges);
        mesh.edges_view().set_count(dst_ix);
        mesh.set_num_edges(mesh.edges().count);
    }

    process_indices(uc, mesh)?;

    // Count the number of UV/color sets
    let mut num_uv: usize = 0;
    let mut num_color: usize = 0;
    let mut num_bitangents: usize = 0;
    let mut num_tangents: usize = 0;
    // C: `ufbxi_for (ufbxi_node, n, node->children, node->num_children)`
    for n in node.children_iter() {
        if n.name() == sp::LayerElementUV.as_ptr() {
            num_uv += 1;
        }
        if n.name() == sp::LayerElementColor.as_ptr() {
            num_color += 1;
        }
        if n.name() == sp::LayerElementBinormal.as_ptr() {
            num_bitangents += 1;
        }
        if n.name() == sp::LayerElementTangent.as_ptr() {
            num_tangents += 1;
        }
    }

    let mut num_textures: usize = 0;

    let bitangents: *mut TangentLayer = uc
        .tmp_stack_view()
        .push_zero::<TangentLayer>(num_bitangents);
    let tangents: *mut TangentLayer = uc.tmp_stack_view().push_zero::<TangentLayer>(num_tangents);
    ufbxi_check!(uc, !bitangents.is_null(), "bitangents");
    ufbxi_check!(uc, !tangents.is_null(), "tangents");
    // SAFETY (both): the non-null zeroed allocation just pushed on uc's own
    // `tmp_stack` — one contiguous, write-capable run of `num_bitangents` /
    // `num_tangents` initialized `ufbxi_tangent_layer` slots that stays alive
    // and unmoved for the rest of this function: nothing below pushes or pops
    // that buf (the layer readers below allocate from `result`, the string
    // pool, `tmp_mesh_textures` and `tmp_arr` only).
    let bitangents_run: Run<'_, TangentLayer, Mut> =
        unsafe { Run::<TangentLayer, Mut>::from_raw_parts(bitangents, num_bitangents) };
    // SAFETY: as above, for the `LayerElementTangent` run.
    let tangents_run: Run<'_, TangentLayer, Mut> =
        unsafe { Run::<TangentLayer, Mut>::from_raw_parts(tangents, num_tangents) };

    mesh.uv_sets_view()
        .set_data(uc.result_view().push_zero::<UvSet>(num_uv));
    mesh.color_sets_view()
        .set_data(uc.result_view().push_zero::<ColorSet>(num_color));
    ufbxi_check!(uc, !mesh.uv_sets().data.is_null(), "mesh->uv_sets.data");
    ufbxi_check!(
        uc,
        !mesh.color_sets().data.is_null(),
        "mesh->color_sets.data"
    );

    let mut num_bitangents_read: usize = 0;
    let mut num_tangents_read: usize = 0;
    // C: `ufbxi_for (ufbxi_node, n, node->children, node->num_children)`
    for n in node.children_iter() {
        // SAFETY: `n.name()` is a NUL-terminated interned parse-tree name, so
        // its first byte is readable.
        if unsafe { *n.name().add(0) } != b'L' {
            // All names start with 'LayerElement*'
            continue;
        }

        if n.name() == sp::LayerElementNormal.as_ptr() {
            if mesh.vertex_normal().exists() {
                continue;
            }
            // SAFETY: `vertex_normal_raw()` addresses the mesh's own live
            // `ufbx_vertex_vec3` field, reached through `*mut` (write-capable
            // provenance for `Mut`); the static asserts pin
            // `ufbx_vertex_vec3` to `ufbx_vertex_attrib`'s layout.
            let attrib: &View<VertexAttrib> = unsafe {
                View::<VertexAttrib>::from_ptr(mesh.vertex_normal_raw() as *mut VertexAttrib)
            };
            // SAFETY: `read_vertex_element` is an `unsafe fn`; the interned name
            // runs are NUL-terminated (its contract) and the `3` value-real
            // count matches that attribute type.
            unsafe {
                read_vertex_element(
                    uc,
                    mesh,
                    n,
                    attrib,
                    &sp::Normals,
                    &sp::NormalsIndex,
                    Some(&sp::NormalsW[..]),
                    b'r',
                    3,
                )
            }?;
        } else if n.name() == sp::LayerElementBinormal.as_ptr() {
            // The counting pass above found exactly `num_bitangents`
            // `LayerElementBinormal` children, and this branch consumes one slot
            // per such child, so `num_bitangents_read < num_bitangents` — the
            // bound `at` checks against the run's own length.
            let layer: &View<TangentLayer> = bitangents_run.at(num_bitangents_read);
            num_bitangents_read += 1;

            if let Some(got) = get_val1::<i32>(n) {
                layer.set_index(got as u32);
            }
            // SAFETY: `layer.elem_raw()` addresses the in-bounds slot's live
            // `ufbx_vertex_vec3`, tmp-stack memory reached through `*mut`
            // (write-capable provenance for `Mut`); the static asserts pin
            // `ufbx_vertex_vec3` to `ufbx_vertex_attrib`'s layout.
            let attrib: &View<VertexAttrib> =
                unsafe { View::<VertexAttrib>::from_ptr(layer.elem_raw() as *mut VertexAttrib) };
            // SAFETY: `read_vertex_element` is an `unsafe fn`; the interned name
            // runs are NUL-terminated (its contract) and the `3` value-real
            // count matches the attribute.
            unsafe {
                read_vertex_element(
                    uc,
                    mesh,
                    n,
                    attrib,
                    &sp::Binormals,
                    &sp::BinormalsIndex,
                    Some(&sp::BinormalsW[..]),
                    b'r',
                    3,
                )
            }?;
            if !layer.elem_view().exists() {
                num_bitangents_read -= 1;
            }
        } else if n.name() == sp::LayerElementTangent.as_ptr() {
            // The counting pass above found exactly `num_tangents`
            // `LayerElementTangent` children, and this branch consumes one slot
            // per such child, so `num_tangents_read < num_tangents` — the bound
            // `at` checks against the run's own length.
            let layer: &View<TangentLayer> = tangents_run.at(num_tangents_read);
            num_tangents_read += 1;

            if let Some(got) = get_val1::<i32>(n) {
                layer.set_index(got as u32);
            }
            // SAFETY: `layer.elem_raw()` addresses the in-bounds slot's live
            // `ufbx_vertex_vec3`, tmp-stack memory reached through `*mut`
            // (write-capable provenance for `Mut`); the static asserts pin
            // `ufbx_vertex_vec3` to `ufbx_vertex_attrib`'s layout.
            let attrib: &View<VertexAttrib> =
                unsafe { View::<VertexAttrib>::from_ptr(layer.elem_raw() as *mut VertexAttrib) };
            // SAFETY: `read_vertex_element` is an `unsafe fn`; the interned name
            // runs are NUL-terminated (its contract) and the `3` value-real
            // count matches the attribute.
            unsafe {
                read_vertex_element(
                    uc,
                    mesh,
                    n,
                    attrib,
                    &sp::Tangents,
                    &sp::TangentsIndex,
                    Some(&sp::TangentsW[..]),
                    b'r',
                    3,
                )
            }?;
            if !layer.elem_view().exists() {
                num_tangents_read -= 1;
            }
        } else if n.name() == sp::LayerElementUV.as_ptr() {
            // SAFETY: `uv_sets.data` is the `num_uv`-long result-arena run
            // pushed above (reached through `*mut`, write-capable for `Mut`) and
            // the counting pass found exactly `num_uv` `LayerElementUV`
            // children, so `uv_sets.count` (bumped at most once per such child)
            // bounds the offset derived from the list base.
            let set: &View<UvSet> = unsafe {
                View::<UvSet>::from_ptr(
                    (mesh.uv_sets().data as *mut UvSet).add(mesh.uv_sets().count),
                )
            };
            mesh.uv_sets_view().set_count(mesh.uv_sets().count + 1);

            if let Some(got) = get_val1::<i32>(n) {
                set.set_index(got as u32);
            }
            if let Some(got) = find_val1::<Checked<String>>(n, sp::Name.as_ptr()) {
                set.set_name(got.0);
            } else {
                set.set_name(EMPTY_STRING.0);
            }

            // SAFETY: `set.vertex_uv_raw()` addresses the in-bounds slot's live
            // `ufbx_vertex_vec2`, result-arena memory reached through `*mut`
            // (write-capable provenance for `Mut`); the static asserts pin
            // `ufbx_vertex_vec2` to `ufbx_vertex_attrib`'s layout.
            let attrib: &View<VertexAttrib> =
                unsafe { View::<VertexAttrib>::from_ptr(set.vertex_uv_raw() as *mut VertexAttrib) };
            // SAFETY: `read_vertex_element` is an `unsafe fn`; the interned name
            // runs are NUL-terminated (its contract) and the `2` value-real
            // count matches the attribute.
            unsafe {
                read_vertex_element(uc, mesh, n, attrib, &sp::UV, &sp::UVIndex, None, b'r', 2)
            }?;
            if !set.vertex_uv().exists() {
                mesh.uv_sets_view().set_count(mesh.uv_sets().count - 1);
            }
        } else if n.name() == sp::LayerElementColor.as_ptr() {
            // SAFETY: `color_sets.data` is the `num_color`-long result-arena run
            // pushed above (reached through `*mut`, write-capable for `Mut`) and
            // the counting pass found exactly `num_color` `LayerElementColor`
            // children, so `color_sets.count` (bumped at most once per such
            // child) bounds the offset derived from the list base.
            let set: &View<ColorSet> = unsafe {
                View::<ColorSet>::from_ptr(
                    (mesh.color_sets().data as *mut ColorSet).add(mesh.color_sets().count),
                )
            };
            mesh.color_sets_view()
                .set_count(mesh.color_sets().count + 1);

            if let Some(got) = get_val1::<i32>(n) {
                set.set_index(got as u32);
            }
            if let Some(got) = find_val1::<Checked<String>>(n, sp::Name.as_ptr()) {
                set.set_name(got.0);
            } else {
                set.set_name(EMPTY_STRING.0);
            }

            // SAFETY: `set.vertex_color_raw()` addresses the in-bounds slot's
            // live `ufbx_vertex_vec4`, result-arena memory reached through
            // `*mut` (write-capable provenance for `Mut`); the static asserts
            // pin `ufbx_vertex_vec4` to `ufbx_vertex_attrib`'s layout.
            let attrib: &View<VertexAttrib> = unsafe {
                View::<VertexAttrib>::from_ptr(set.vertex_color_raw() as *mut VertexAttrib)
            };
            // SAFETY: `read_vertex_element` is an `unsafe fn`; the interned name
            // runs are NUL-terminated (its contract) and the `4` value-real
            // count matches the attribute.
            unsafe {
                read_vertex_element(
                    uc,
                    mesh,
                    n,
                    attrib,
                    &sp::Colors,
                    &sp::ColorIndex,
                    None,
                    b'r',
                    4,
                )
            }?;
            if !set.vertex_color().exists() {
                mesh.color_sets_view()
                    .set_count(mesh.color_sets().count - 1);
            }
        } else if n.name() == sp::LayerElementVertexCrease.as_ptr() {
            // SAFETY: `vertex_crease_raw()` addresses the mesh's own live
            // `ufbx_vertex_real` field, reached through `*mut` (write-capable
            // provenance for `Mut`); the static asserts pin `ufbx_vertex_real`
            // to `ufbx_vertex_attrib`'s layout.
            let attrib: &View<VertexAttrib> = unsafe {
                View::<VertexAttrib>::from_ptr(mesh.vertex_crease_raw() as *mut VertexAttrib)
            };
            // SAFETY: `read_vertex_element` is an `unsafe fn`; the interned name
            // runs are NUL-terminated (its contract) and the `1` value-real
            // count matches the attribute.
            unsafe {
                read_vertex_element(
                    uc,
                    mesh,
                    n,
                    attrib,
                    &sp::VertexCrease,
                    &sp::VertexCreaseIndex,
                    None,
                    b'r',
                    1,
                )
            }?;
        } else if n.name() == sp::LayerElementEdgeCrease.as_ptr() {
            // C: `const char *mapping = "";`
            let mut mapping: *const u8 = EMPTY_CHAR.as_ptr();
            if let Some(got) =
                find_val1::<Unchecked<*const u8>>(n, sp::MappingInformationType.as_ptr())
            {
                mapping = got.0;
            }
            if mapping == sp::ByEdge.as_ptr() {
                if mesh.edge_crease().count != 0 {
                    continue;
                }
                // SAFETY: `b'r'` has the size, alignment and value validity of
                // the explicit `T = Real`; its payload stays live with the
                // loader, and the destination is the mesh's own list field.
                unsafe {
                    read_truncated_array::<Real>(
                        uc,
                        mesh.edge_crease_view(),
                        n,
                        sp::EdgeCrease.as_ptr(),
                        b'r',
                        mesh.num_edges(),
                    )
                }?;
            } else {
                // SAFETY: `mapping` is either the empty static or a NUL-terminated
                // interned parse-tree string written by the `'c'` fetch above.
                unsafe { warn_polygon_mapping(uc, sp::EdgeCrease.as_ptr(), mapping) }?;
            }
        } else if n.name() == sp::LayerElementSmoothing.as_ptr() {
            // C: `const char *mapping = "";`
            let mut mapping: *const u8 = EMPTY_CHAR.as_ptr();
            if let Some(got) =
                find_val1::<Unchecked<*const u8>>(n, sp::MappingInformationType.as_ptr())
            {
                mapping = got.0;
            }
            if mapping == sp::ByEdge.as_ptr() {
                if mesh.edge_smoothing().count != 0 {
                    continue;
                }
                // SAFETY: `b'b'` has the size, alignment and value validity of
                // the explicit `T = bool`; its payload stays live with the
                // loader, and the destination is the mesh's own list field.
                unsafe {
                    read_truncated_array::<bool>(
                        uc,
                        mesh.edge_smoothing_view(),
                        n,
                        sp::Smoothing.as_ptr(),
                        b'b',
                        mesh.num_edges(),
                    )
                }?;
            } else if mapping == sp::ByPolygon.as_ptr() {
                if mesh.face_smoothing().count != 0 {
                    continue;
                }
                // SAFETY: `b'b'` has the size, alignment and value validity of
                // the explicit `T = bool`; its payload stays live with the
                // loader, and the destination is the mesh's own list field.
                unsafe {
                    read_truncated_array::<bool>(
                        uc,
                        mesh.face_smoothing_view(),
                        n,
                        sp::Smoothing.as_ptr(),
                        b'b',
                        mesh.num_faces(),
                    )
                }?;
            } else {
                // SAFETY: `mapping` is either the empty static or a NUL-terminated
                // interned parse-tree string written by the `'c'` fetch above.
                unsafe { warn_polygon_mapping(uc, sp::Smoothing.as_ptr(), mapping) }?;
            }
        } else if n.name() == sp::LayerElementVisibility.as_ptr() {
            // C: `const char *mapping = "";`
            let mut mapping: *const u8 = EMPTY_CHAR.as_ptr();
            if let Some(got) =
                find_val1::<Unchecked<*const u8>>(n, sp::MappingInformationType.as_ptr())
            {
                mapping = got.0;
            }
            if mapping == sp::ByEdge.as_ptr() {
                if mesh.edge_visibility().count != 0 {
                    continue;
                }
                // SAFETY: `b'b'` has the size, alignment and value validity of
                // the explicit `T = bool`; its payload stays live with the
                // loader, and the destination is the mesh's own list field.
                unsafe {
                    read_truncated_array::<bool>(
                        uc,
                        mesh.edge_visibility_view(),
                        n,
                        sp::Visibility.as_ptr(),
                        b'b',
                        mesh.num_edges(),
                    )
                }?;
            } else {
                // SAFETY: `mapping` is either the empty static or a NUL-terminated
                // interned parse-tree string written by the `'c'` fetch above.
                unsafe { warn_polygon_mapping(uc, sp::Visibility.as_ptr(), mapping) }?;
            }
        } else if n.name() == sp::LayerElementMaterial.as_ptr() {
            if mesh.face_material().count != 0 {
                continue;
            }
            // C: `const char *mapping = "";`
            let mut mapping: *const u8 = EMPTY_CHAR.as_ptr();
            if let Some(got) =
                find_val1::<Unchecked<*const u8>>(n, sp::MappingInformationType.as_ptr())
            {
                mapping = got.0;
            }
            if mapping == sp::ByPolygon.as_ptr() {
                // SAFETY: `b'i'` has the size, alignment and value validity of
                // the explicit `T = u32`; its payload stays live with the
                // loader, and the destination is the mesh's own list field.
                unsafe {
                    read_truncated_array::<u32>(
                        uc,
                        mesh.face_material_view(),
                        n,
                        sp::Materials.as_ptr(),
                        b'i',
                        mesh.num_faces(),
                    )
                }?;
            } else if mapping == sp::AllSame.as_ptr() {
                let arr: *mut ValueArray = find_array(n, sp::Materials.as_ptr(), b'i');
                // SAFETY: the view is minted only in the non-null arm, where
                // `find_array` returned a node's own array descriptor, live for
                // as long as the parse tree and reached through `*mut`
                // (write-capable provenance for `Mut`).
                let arr: Option<&View<ValueArray>> = if arr.is_null() {
                    None
                } else {
                    Some(unsafe { View::<ValueArray>::from_ptr(arr) })
                };
                ufbxi_check!(
                    uc,
                    arr.is_some_and(|arr| arr.size() >= 1),
                    "arr && arr->size >= 1"
                );
                // The check above returned unless `arr` is a live descriptor
                // with `size >= 1`, whose `'i'` payload is a run of `size` `u32`s.
                let arr: &View<ValueArray> = arr.unwrap();
                // SAFETY: `arr.data()` names that `size >= 1` run of `u32`, so
                // its first element is readable.
                let material: u32 = unsafe { *(arr.data() as *mut u32) };
                mesh.face_material_view().set_count(mesh.num_faces());
                if material == 0 {
                    // The zero sentinel stands in for a `num_faces` run of zeros.
                    mesh.face_material_view()
                        .set_data(SENTINEL_INDEX_ZERO.as_ptr());
                } else {
                    mesh.face_material_view()
                        .set_data(uc.result_view().push::<u32>(mesh.num_faces()));
                    ufbxi_check!(
                        uc,
                        !mesh.face_material().data.is_null(),
                        "mesh->face_material.data"
                    );
                    // C: `ufbxi_for_list(uint32_t, p_mat, mesh->face_material)`
                    for p_mat in Run::from_list(mesh.face_material_view()).iter() {
                        p_mat.write_value(material);
                    }
                }
            } else {
                // SAFETY: `mapping` is either the empty static or a NUL-terminated
                // interned parse-tree string written by the `'c'` fetch above.
                unsafe { warn_polygon_mapping(uc, sp::Materials.as_ptr(), mapping) }?;
            }
        } else if n.name() == sp::LayerElementPolygonGroup.as_ptr() {
            if mesh.face_group().count != 0 {
                continue;
            }
            // C: `const char *mapping = NULL;`
            let mapping: *const u8 = ufbxi_check_some!(
                uc,
                find_val1::<Unchecked<*const u8>>(n, sp::MappingInformationType.as_ptr()),
                "ufbxi_find_val1(n, ufbxi_MappingInformationType, \"c\", (char**)&mapping)"
            )
            .0;
            if mapping == sp::ByPolygon.as_ptr() {
                // SAFETY: `b'i'` has the size, alignment and value validity of
                // the explicit `T = u32`; its payload stays live with the
                // loader, and the destination is the mesh's own list field.
                unsafe {
                    read_truncated_array::<u32>(
                        uc,
                        mesh.face_group_view(),
                        n,
                        sp::PolygonGroup.as_ptr(),
                        b'i',
                        mesh.num_faces(),
                    )
                }?;
            }
        } else if n.name() == sp::LayerElementHole.as_ptr() {
            if mesh.face_group().count != 0 {
                continue;
            }
            // C: `const char *mapping = NULL;`
            let mapping: *const u8 = ufbxi_check_some!(
                uc,
                find_val1::<Unchecked<*const u8>>(n, sp::MappingInformationType.as_ptr()),
                "ufbxi_find_val1(n, ufbxi_MappingInformationType, \"c\", (char**)&mapping)"
            )
            .0;
            if mapping == sp::ByPolygon.as_ptr() {
                // SAFETY: `b'b'` has the size, alignment and value validity of
                // the explicit `T = bool`; its payload stays live with the
                // loader, and the destination is the mesh's own list field.
                unsafe {
                    read_truncated_array::<bool>(
                        uc,
                        mesh.face_hole_view(),
                        n,
                        sp::Hole.as_ptr(),
                        b'b',
                        mesh.num_faces(),
                    )
                }?;
            }
        // SAFETY: `n.name()` is a NUL-terminated interned parse-tree name, so
        // the comparison stops at its terminator, within 12 bytes or earlier.
        } else if unsafe { strncmp(n.name(), b"LayerElement\0".as_ptr(), 12) } == 0 {
            // Make sure the name has no internal zero bytes
            // SAFETY: `n.name()` spans `n.name_len()` readable bytes before its
            // terminator.
            let name_bytes = unsafe { slice_from_ptr(n.name(), n.name_len() as usize) };
            ufbxi_check!(
                uc,
                memchr(name_bytes, b'\0').is_null(),
                "!memchr(n->name, '\\0', n->name_len)"
            );

            // What?! 6x00 stores textures in mesh geometry, eg. "LayerElementTexture",
            // "LayerElementDiffuseFactorTextures", "LayerElementEmissive_Textures"...
            let mut prop_name: String = EMPTY_STRING.0;
            // SAFETY: the `name_len() > 20` test short-circuits first, so
            // `name().add(name_len() - 8)` stays inside the name's bytes, and
            // the name is NUL-terminated — `strcmp`'s contract.
            if n.name_len() > 20
                && unsafe {
                    strcmp(
                        n.name().add(n.name_len() as usize - 8),
                        b"Textures\0".as_ptr(),
                    )
                } == 0
            {
                // SAFETY: `name_len() > 20`, so offset 12 is inside the name and
                // the remaining `name_len() - 20` bytes are readable.
                prop_name.data = unsafe { n.name().add(12) };
                prop_name.length = n.name_len() as usize - 20;
                // SAFETY: `prop_name` spans `length` readable name bytes, and
                // `length >= 1` because `name_len() > 20`.
                if unsafe { *prop_name.data.add(prop_name.length - 1) } == b'_' {
                    prop_name.length -= 1;
                }
            // SAFETY: `n.name()` is a NUL-terminated interned parse-tree name.
            } else if unsafe { strcmp(n.name(), b"LayerElementTexture\0".as_ptr()) } == 0 {
                prop_name.data = b"Diffuse\0".as_ptr();
                prop_name.length = 7;
            }

            if prop_name.length > 0 {
                // `prop_name` is a live local whose `data` spans `length`
                // readable bytes.
                push_string_place_str(
                    uc.string_pool_view(),
                    StringView::from_mut(&mut prop_name),
                    false,
                )?;
                // C: `const char *mapping = NULL;`
                if let Some(Unchecked(mapping)) =
                    find_val1::<Unchecked<*const u8>>(n, sp::MappingInformationType.as_ptr())
                {
                    let arr: *mut ValueArray = find_array(n, sp::TextureId.as_ptr(), b'i');

                    let tex: *mut TmpMeshTexture =
                        uc.tmp_mesh_textures_view().push_zero::<TmpMeshTexture>(1);
                    ufbxi_check!(uc, !tex.is_null(), "tex");
                    // SAFETY: `tex` is the fresh non-null single-element push
                    // checked above, zero-filled on uc's own `tmp_mesh_textures`
                    // buf and reached through `*mut` (write-capable for `Mut`).
                    let tex: &View<TmpMeshTexture> =
                        unsafe { View::<TmpMeshTexture>::from_ptr(tex) };
                    if !arr.is_null() {
                        // SAFETY: `arr` is non-null (checked) — a node's own
                        // array descriptor, live for as long as the parse tree
                        // and reached through `*mut` (write-capable for `Mut`);
                        // its `'i'` payload is a run of `size` `u32`s.
                        let arr: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(arr) };
                        tex.set_face_texture(arr.data() as *mut u32);
                        tex.set_num_faces(arr.size());
                    }
                    tex.set_prop_name(prop_name);
                    tex.set_all_same(mapping == sp::AllSame.as_ptr());
                    num_textures += 1;
                }
            }
        }
    }

    // Always use a default zero material, this will be removed if no materials are found
    if mesh.face_material().count == 0 {
        uc.set_max_zero_indices(max_sz(uc.max_zero_indices(), mesh.num_faces()));
        // The zero sentinel stands in for a `num_faces` run of zeros.
        mesh.face_material_view()
            .set_data(SENTINEL_INDEX_ZERO.as_ptr());
        mesh.face_material_view().set_count(mesh.num_faces());
    }

    if uc.opts_view().strict() {
        ufbxi_check!(
            uc,
            mesh.uv_sets().count == num_uv,
            "mesh->uv_sets.count == num_uv"
        );
        ufbxi_check!(
            uc,
            mesh.color_sets().count == num_color,
            "mesh->color_sets.count == num_color"
        );
        ufbxi_check!(
            uc,
            num_bitangents_read == num_bitangents,
            "num_bitangents_read == num_bitangents"
        );
        ufbxi_check!(
            uc,
            num_tangents_read == num_tangents,
            "num_tangents_read == num_tangents"
        );
    }

    // Connect bitangents/tangents to UV sets
    // C: `ufbxi_for (ufbxi_node, n, node->children, node->num_children)`
    for n in node.children_iter() {
        if n.name() != sp::Layer.as_ptr() {
            continue;
        }
        let mut uv_set: Option<&View<UvSet>> = None;
        let mut bitangent_layer: Option<&View<TangentLayer>> = None;
        let mut tangent_layer: Option<&View<TangentLayer>> = None;

        // C: `ufbxi_for (ufbxi_node, c, n->children, n->num_children)`
        for c in n.children_iter() {
            if c.name() != sp::LayerElement.as_ptr() {
                continue;
            }
            // C: `uint32_t index; const char *type;` — both written by the
            // guarded `ufbxi_find_val1` calls before any read.
            let Some(index) = find_val1::<i32>(c, sp::TypedIndex.as_ptr()) else {
                continue;
            };
            let index: u32 = index as u32;
            let Some(Checked(type_)) = find_val1::<Checked<*const u8>>(c, sp::Type.as_ptr()) else {
                continue;
            };

            if type_ == sp::LayerElementUV.as_ptr() {
                // C: `ufbxi_for(ufbx_uv_set, set, mesh->uv_sets.data, mesh->uv_sets.count)`
                for set in Run::from_list(mesh.uv_sets_view()).iter() {
                    if set.index() == index {
                        uv_set = Some(set);
                        break;
                    }
                }
            } else if type_ == sp::LayerElementBinormal.as_ptr() {
                // C: `ufbxi_for(ufbxi_tangent_layer, layer, bitangents, num_bitangents_read)`
                // The layer loop filled the run's first
                // `num_bitangents_read <= num_bitangents` entries, which is the
                // bound `subrun` checks against the run's own length.
                for layer in bitangents_run.subrun(0, num_bitangents_read).iter() {
                    if layer.index() == index {
                        bitangent_layer = Some(layer);
                        break;
                    }
                }
            } else if type_ == sp::LayerElementTangent.as_ptr() {
                // C: `ufbxi_for(ufbxi_tangent_layer, layer, tangents, num_tangents_read)`
                // The layer loop filled the run's first
                // `num_tangents_read <= num_tangents` entries, which is the
                // bound `subrun` checks against the run's own length.
                for layer in tangents_run.subrun(0, num_tangents_read).iter() {
                    if layer.index() == index {
                        tangent_layer = Some(layer);
                        break;
                    }
                }
            }
        }

        if let Some(uv_set) = uv_set {
            if let Some(bitangent_layer) = bitangent_layer {
                uv_set.set_vertex_bitangent(bitangent_layer.elem());
            }
            if let Some(tangent_layer) = tangent_layer {
                uv_set.set_vertex_tangent(tangent_layer.elem());
            }
        }
    }

    mesh.set_skinned_is_local(true);
    // SAFETY (both reads): `vertex_position`/`vertex_normal` are the mesh's own
    // initialized `ufbx_vertex_vec3` fields, and each destination is a distinct
    // field, so the copy does not overlap.
    mesh.set_skinned_position(unsafe { core::ptr::read(mesh.vertex_position_ptr()) });
    mesh.set_skinned_normal(unsafe { core::ptr::read(mesh.vertex_normal_ptr()) });

    patch_mesh_reals(mesh);

    if mesh.face_group().count > 0 && mesh.face_groups().count == 0 {
        // C: `&uc->max_consecutive_indices` — uc's own live, initialized
        // `size_t` counter slot.
        // SAFETY: `max_consecutive_indices_mut_ptr()` addresses that slot with
        // write-capable provenance, and `ScalarView` is `repr(transparent)`
        // interior-mutable storage over a `usize`.
        let p_consecutive_indices: &ScalarView<usize> =
            unsafe { &*(uc.max_consecutive_indices_mut_ptr() as *const ScalarView<usize>) };
        assign_face_groups(
            uc.result_view(),
            uc.error_view(),
            mesh,
            Some(p_consecutive_indices),
            uc.retain_mesh_parts(),
        )?;
    }

    // Sort UV and color sets by set index
    sort_uv_sets(uc, Run::from_list(mesh.uv_sets_view()))?;
    sort_color_sets(uc, Run::from_list(mesh.color_sets_view()))?;

    if num_textures > 0 {
        // `element.element_id` is the mesh's own id — the element
        // `push_element_extra` attaches to.
        let extra: *mut MeshExtra = push_element_extra(uc, mesh.element().element_id());
        ufbxi_check!(uc, !extra.is_null(), "extra");
        // SAFETY: `extra` is the fresh non-null extra checked above — uc's own
        // element-extra storage, reached through `*mut` (write-capable for `Mut`).
        let extra: &View<MeshExtra> = unsafe { View::<MeshExtra>::from_ptr(extra) };
        extra.set_texture_count(num_textures);
        extra.set_texture_arr(
            uc.tmp_view()
                .push_pop::<TmpMeshTexture>(uc.tmp_mesh_textures_view(), num_textures),
        );
        ufbxi_check!(uc, !extra.texture_arr().is_null(), "extra->texture_arr");
    }

    // Subdivision

    if let Some(got) = find_val1::<i32>(node, sp::PreviewDivisionLevels.as_ptr()) {
        mesh.set_subdivision_preview_levels(got as u32);
    }
    if let Some(got) = find_val1::<i32>(node, sp::RenderDivisionLevels.as_ptr()) {
        mesh.set_subdivision_render_levels(got as u32);
    }

    // C: `int32_t smoothness, boundary;` — written by the guarded
    // `ufbxi_find_val1` calls below.
    if let Some(smoothness) = find_val1::<i32>(node, sp::Smoothness.as_ptr()) {
        if smoothness >= 0 && smoothness <= SubdivisionDisplayMode::Smooth as i32 {
            mesh.set_subdivision_display_mode(subdivision_display_mode_from_raw(smoothness));
        }
    }
    if let Some(boundary) = find_val1::<i32>(node, sp::BoundaryRule.as_ptr()) {
        if boundary >= 0 && boundary < SubdivisionBoundary::SharpCorners as i32 {
            mesh.set_subdivision_boundary(subdivision_boundary_from_raw(boundary + 1));
        }
    }

    Ok(())
}

// ufbx.c:13813-13823 `ufbxi_read_nurbs_topology`
#[inline(never)]
pub(crate) fn read_nurbs_topology(form: &[u8]) -> NurbsTopology {
    // C: `strcmp(form, "Open")` over the NUL-terminated `'C'` value — `c_strcmp`
    // stops at the first NUL like `strcmp`, and an exhausted slice reads as NUL.
    if c_strcmp(form, b"Open\0") == 0 {
        return NurbsTopology::Open;
    } else if c_strcmp(form, b"Closed\0") == 0 {
        return NurbsTopology::Closed;
    } else if c_strcmp(form, b"Periodic\0") == 0 {
        return NurbsTopology::Periodic;
    }
    NurbsTopology::Open
}

// ufbx.c:13825-13853 `ufbxi_read_nurbs_curve`
#[inline(never)]
pub(crate) fn read_nurbs_curve(
    uc: &Context,
    node: &NodeView,
    info: &ElementInfoView,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `NurbsCurve` is the element struct for `ElementType::NurbsCurve`.
    let nurbs: *mut NurbsCurve =
        unsafe { push_element::<NurbsCurve>(uc, info, ElementType::NurbsCurve) };
    ufbxi_check!(uc, !nurbs.is_null(), "nurbs");

    let mut dimension: i32 = 3;

    // SAFETY: `nurbs` is the fresh non-null element pushed above; the fetch
    // yields the value, so the only raw op is the write into its own field.
    unsafe {
        (*nurbs).basis.order = ufbxi_check_some!(
            uc,
            find_val1::<i32>(node, sp::Order.as_ptr()),
            "ufbxi_find_val1(node, ufbxi_Order, \"I\", &nurbs->basis.order)"
        ) as u32;
    }
    if let Some(got) = find_val1::<i32>(node, sp::Dimension.as_ptr()) {
        dimension = got;
    }
    let Checked(form) = ufbxi_check_some!(
        uc,
        find_val1::<Checked<*const u8>>(node, sp::Form.as_ptr()),
        "ufbxi_find_val1(node, ufbxi_Form, \"C\", (char**)&form)"
    );
    // SAFETY: the `'C'` fetch above succeeded (checked), so `form` points at the
    // NUL-terminated parse-tree string, whose terminator bounds `strlen` and
    // whose bytes stay live and unwritten for the parse tree's lifetime.
    let form: &[u8] = unsafe { slice_from_ptr(form, strlen(form)) };
    // SAFETY: `nurbs` is the fresh non-null element pushed above.
    unsafe {
        (*nurbs).basis.topology = read_nurbs_topology(form);
    }
    // SAFETY: `nurbs` is the fresh non-null element pushed above.
    unsafe {
        (*nurbs).basis.is_2d = dimension == 2;
    }

    if !uc.opts_view().ignore_geometry() {
        let points: *mut ValueArray = find_array(node, sp::Points.as_ptr(), b'r');
        let knot: *mut ValueArray = find_array(node, sp::KnotVector.as_ptr(), b'r');
        ufbxi_check!(uc, !points.is_null(), "points");
        ufbxi_check!(uc, !knot.is_null(), "knot");
        // SAFETY: `points` is non-null (checked above) and `find_array` returns
        // the node's own array descriptor, live for as long as the parse tree.
        ufbxi_check!(
            uc,
            unsafe { (*points).size } % 4 == 0,
            "points->size % 4 == 0"
        );

        // SAFETY: `nurbs` is the fresh non-null element; `points`/`knot` are the
        // live array descriptors checked non-null above, whose `'r'` payloads are
        // `size` reals — `points.size` being a multiple of 4 (checked) makes it
        // `size / 4` `ufbx_vec4` control points.
        unsafe {
            (*nurbs).control_points.count = (*points).size / 4;
            (*nurbs).control_points.data = (*points).data as *const Vec4;
            (*nurbs).basis.knot_vector.data = (*knot).data as *const Real;
            (*nurbs).basis.knot_vector.count = (*knot).size;
        }
    }

    Ok(())
}

// ufbx.c:13855-13894 `ufbxi_read_nurbs_surface`
#[inline(never)]
pub(crate) fn read_nurbs_surface(
    uc: &Context,
    node: &NodeView,
    info: &ElementInfoView,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `NurbsSurface` is the element struct for `ElementType::NurbsSurface`.
    let nurbs: *mut NurbsSurface =
        unsafe { push_element::<NurbsSurface>(uc, info, ElementType::NurbsSurface) };
    ufbxi_check!(uc, !nurbs.is_null(), "nurbs");
    // SAFETY: `nurbs` is the fresh non-null element just pushed into uc's own
    // element arena — reached through `*mut` (write-capable provenance for
    // `Mut`), live and unmoved for the rest of the load; the fields below are
    // the ones this function fills in.
    let nurbs: &View<NurbsSurface> = unsafe { View::<NurbsSurface>::from_ptr(nurbs) };

    let (order_u, order_v) = ufbxi_check_some!(
        uc,
        find_val2::<i32, i32>(node, sp::NurbsSurfaceOrder.as_ptr()),
        "ufbxi_find_val2(node, ufbxi_NurbsSurfaceOrder, \"II\", &nurbs->basis_u.order, &nurbs->basis_v.order)"
    );
    nurbs.basis_u().set_order(order_u as u32);
    nurbs.basis_v().set_order(order_v as u32);
    let (dimension_u, dimension_v) = ufbxi_check_some!(
        uc,
        find_val2::<usize, usize>(node, sp::Dimensions.as_ptr()),
        "ufbxi_find_val2(node, ufbxi_Dimensions, \"ZZ\", &dimension_u, &dimension_v)"
    );
    let (step_u, step_v) = ufbxi_check_some!(
        uc,
        find_val2::<i32, i32>(node, sp::Step.as_ptr()),
        "ufbxi_find_val2(node, ufbxi_Step, \"II\", &step_u, &step_v)"
    );
    let (Checked(form_u), Checked(form_v)) = ufbxi_check_some!(
        uc,
        find_val2::<Checked<*const u8>, Checked<*const u8>>(node, sp::Form.as_ptr()),
        "ufbxi_find_val2(node, ufbxi_Form, \"CC\", (char**)&form_u, (char**)&form_v)"
    );
    if let Some(flip_normals) = find_val1::<bool>(node, sp::FlipNormals.as_ptr()) {
        nurbs.set_flip_normals(flip_normals);
    }
    // SAFETY: the `"CC"` fetch above succeeded (checked), so `form_u`/`form_v`
    // point at NUL-terminated parse-tree strings, whose terminators bound
    // `strlen` and whose bytes stay live and unwritten for the parse tree's
    // lifetime.
    let form_u: &[u8] = unsafe { slice_from_ptr(form_u, strlen(form_u)) };
    // SAFETY: as `form_u` — the same checked `"CC"` fetch.
    let form_v: &[u8] = unsafe { slice_from_ptr(form_v, strlen(form_v)) };
    nurbs.basis_u().set_topology(read_nurbs_topology(form_u));
    nurbs.basis_v().set_topology(read_nurbs_topology(form_v));
    nurbs.set_num_control_points_u(dimension_u);
    nurbs.set_num_control_points_v(dimension_v);
    nurbs.set_span_subdivision_u(if step_u > 0 { step_u as u32 } else { 4u32 });
    nurbs.set_span_subdivision_v(if step_v > 0 { step_v as u32 } else { 4u32 });

    if !uc.opts_view().ignore_geometry() {
        let points: *mut ValueArray = find_array(node, sp::Points.as_ptr(), b'r');
        let knot_u: *mut ValueArray = find_array(node, sp::KnotVectorU.as_ptr(), b'r');
        let knot_v: *mut ValueArray = find_array(node, sp::KnotVectorV.as_ptr(), b'r');
        ufbxi_check!(uc, !points.is_null(), "points");
        ufbxi_check!(uc, !knot_u.is_null(), "knot_u");
        ufbxi_check!(uc, !knot_v.is_null(), "knot_v");
        // SAFETY: `points` is non-null (checked above) and `find_array` returns
        // the node's own array descriptor, live for as long as the parse tree
        // and reached through `*mut` (write-capable provenance for `Mut`).
        let points: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(points) };
        // SAFETY: as `points` — likewise a live, non-null descriptor.
        let knot_u: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(knot_u) };
        // SAFETY: as `points` — likewise a live, non-null descriptor.
        let knot_v: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(knot_v) };
        ufbxi_check!(uc, points.size() % 4 == 0, "points->size % 4 == 0");
        ufbxi_check!(
            uc,
            points.size() / 4 == dimension_u.wrapping_mul(dimension_v),
            "points->size / 4 == (size_t)dimension_u * (size_t)dimension_v"
        );

        // The `'r'` payloads are `size` reals — `points.size` being a multiple
        // of 4 (checked) makes it `size / 4` `ufbx_vec4` control points; the
        // C's field-write order is kept.
        nurbs.control_points_view().set_count(points.size() / 4);
        nurbs
            .control_points_view()
            .set_data(points.data() as *const Vec4);
        nurbs
            .basis_u()
            .knot_vector_view()
            .set_data(knot_u.data() as *const Real);
        nurbs.basis_u().knot_vector_view().set_count(knot_u.size());
        nurbs
            .basis_v()
            .knot_vector_view()
            .set_data(knot_v.data() as *const Real);
        nurbs.basis_v().knot_vector_view().set_count(knot_v.size());
    }

    Ok(())
}

// ufbx.c:13896-13955 `ufbxi_read_line`
#[inline(never)]
pub(crate) fn read_line(uc: &Context, node: &NodeView, info: &ElementInfoView) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `LineCurve` is the element struct for `ElementType::LineCurve`.
    let line: *mut LineCurve =
        unsafe { push_element::<LineCurve>(uc, info, ElementType::LineCurve) };
    ufbxi_check!(uc, !line.is_null(), "line");

    if !uc.opts_view().ignore_geometry() {
        let points: *mut ValueArray = find_array(node, sp::Points.as_ptr(), b'r');
        let points_index: *mut ValueArray = find_array(node, sp::PointsIndex.as_ptr(), b'i');
        ufbxi_check!(uc, !points.is_null(), "points");
        ufbxi_check!(uc, !points_index.is_null(), "points_index");
        // SAFETY: all three pointers are non-null (checked above). `line` is
        // the fresh element pushed into uc's stable temporary arena, while
        // `points`/`points_index` are the node's own live array descriptors;
        // all were reached through write-capable pointers.
        let (line, points, points_index): (&View<LineCurve>, &View<ValueArray>, &View<ValueArray>) = unsafe {
            (
                View::<LineCurve>::from_ptr(line),
                View::<ValueArray>::from_ptr(points),
                View::<ValueArray>::from_ptr(points_index),
            )
        };
        ufbxi_check!(uc, points.size() % 3 == 0, "points->size % 3 == 0");

        if points.size() > 0 {
            let num_control_points: usize = points.size() / 3;
            let num_point_indices: usize = points_index.size();

            // SAFETY: the `'r'`/`'i'` payloads are initialized runs of `size`
            // reals/u32s owned by the parse tree and retained by the scene.
            // `points.size` is a multiple of three (checked), so its payload is
            // exactly `num_control_points` repr(C) three-Real `Vec3` values;
            // both payload pointers retain their original stable provenance.
            unsafe {
                line.control_points_view().set(List::from_raw_parts(
                    points.data().cast::<Vec3>(),
                    num_control_points,
                ));
                line.point_indices_view().set(List::from_raw_parts(
                    points_index.data().cast::<u32>(),
                    num_point_indices,
                ));
            }

            ufbxi_check!(
                uc,
                line.control_points_view().count() < i32::MAX as usize,
                "line->control_points.count < INT32_MAX"
            );

            // Count end points
            let mut num_segments: usize = 1;
            let point_indices = line.point_indices_view();
            if num_point_indices > 0 {
                for i in 0..num_point_indices - 1 {
                    let ix: u32 = point_indices.copy_at(i);
                    num_segments =
                        num_segments.wrapping_add(if (ix as i32) < 0 { 1usize } else { 0usize });
                }
            }

            let mut prev_end: usize = 0;
            let segment_data: *mut LineSegment = uc.result_view().push::<LineSegment>(num_segments);
            ufbxi_check!(uc, !segment_data.is_null(), "line->segments.data");
            // SAFETY: the non-null push above allocated `num_segments`
            // contiguous, write-capable `LineSegment` slots in uc's stable
            // result arena. The slots may be initialized through `Mut` views.
            let segments: Run<'_, LineSegment> =
                unsafe { Run::from_raw_parts(segment_data, num_segments) };
            let mut num_segments_written: usize = 0;

            for i in 0..num_point_indices {
                let p_dst: &View<u32> = point_indices.at(i);
                let mut ix: u32 = point_indices.copy_at(i);
                if (ix as i32) < 0 {
                    ix = !ix;
                    if i + 1 < num_point_indices {
                        // C: `&line->segments.data[line->segments.count++]` —
                        // the index uses the pre-increment value.
                        let segment: &View<LineSegment> = segments.at(num_segments_written);
                        num_segments_written += 1;
                        segment.set_index_begin(prev_end as u32);
                        segment.set_num_indices(i.wrapping_sub(prev_end) as u32);
                        prev_end = i;
                    }
                }

                if (ix as usize) < num_control_points {
                    p_dst.write_value(ix);
                } else {
                    fix_index(uc, p_dst, ix, num_control_points)?;
                }
            }

            let segment: &View<LineSegment> = segments.at(num_segments_written);
            num_segments_written += 1;
            segment.set_index_begin(prev_end as u32);
            segment
                .set_num_indices(to_size(num_point_indices.wrapping_sub(prev_end) as isize) as u32);
            ufbx_assert!(num_segments_written == num_segments);

            // SAFETY: every slot in the result-arena run was fully initialized
            // above, the count assertion proves the complete allocated run was
            // consumed, and result storage remains stable for the scene.
            unsafe {
                line.segments_view()
                    .set(List::from_raw_parts(segment_data, num_segments));
            }
        }
    }

    Ok(())
}

// Rust-port infrastructure (not a ufbx.c section): the write surface
// `read_transform_matrix` needs on the `ufbx_matrix` destination its callers
// hand it — the twelve leaves C assigns.
pub(crate) type MatrixView = View<Matrix>;

impl MatrixView {
    #[inline(always)]
    pub(crate) fn set_m00(&self, value: Real) {
        view_write!(self, m00, value);
    }
    #[inline(always)]
    pub(crate) fn set_m10(&self, value: Real) {
        view_write!(self, m10, value);
    }
    #[inline(always)]
    pub(crate) fn set_m20(&self, value: Real) {
        view_write!(self, m20, value);
    }
    #[inline(always)]
    pub(crate) fn set_m01(&self, value: Real) {
        view_write!(self, m01, value);
    }
    #[inline(always)]
    pub(crate) fn set_m11(&self, value: Real) {
        view_write!(self, m11, value);
    }
    #[inline(always)]
    pub(crate) fn set_m21(&self, value: Real) {
        view_write!(self, m21, value);
    }
    #[inline(always)]
    pub(crate) fn set_m02(&self, value: Real) {
        view_write!(self, m02, value);
    }
    #[inline(always)]
    pub(crate) fn set_m12(&self, value: Real) {
        view_write!(self, m12, value);
    }
    #[inline(always)]
    pub(crate) fn set_m22(&self, value: Real) {
        view_write!(self, m22, value);
    }
    #[inline(always)]
    pub(crate) fn set_m03(&self, value: Real) {
        view_write!(self, m03, value);
    }
    #[inline(always)]
    pub(crate) fn set_m13(&self, value: Real) {
        view_write!(self, m13, value);
    }
    #[inline(always)]
    pub(crate) fn set_m23(&self, value: Real) {
        view_write!(self, m23, value);
    }
}

// ufbx.c:13957-13963 `ufbxi_read_transform_matrix`
#[inline(never)]
pub(crate) fn read_transform_matrix(m: &MatrixView, data: &[Real; 16]) {
    m.set_m00(data[0]);
    m.set_m10(data[1]);
    m.set_m20(data[2]);
    m.set_m01(data[4]);
    m.set_m11(data[5]);
    m.set_m21(data[6]);
    m.set_m02(data[8]);
    m.set_m12(data[9]);
    m.set_m22(data[10]);
    m.set_m03(data[12]);
    m.set_m13(data[13]);
    m.set_m23(data[14]);
}

// ufbx.c:13965-13977 `ufbxi_read_bone`
#[inline(never)]
pub(crate) fn read_bone(
    uc: &Context,
    node: &NodeView,
    info: &ElementInfoView,
    sub_type: &[u8],
) -> Result<(), Fail> {
    let _ = node; // C: `(void)node;`

    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Bone` is the element struct for `ElementType::Bone`.
    let bone: *mut Bone = unsafe { push_element::<Bone>(uc, info, ElementType::Bone) };
    ufbxi_check!(uc, !bone.is_null(), "bone");

    // C-parity: `sub_type` is matched by POINTER IDENTITY against the interned
    // `ufbxi_Root` constant, so compare the borrowed run's own address.
    if sub_type.as_ptr() == sp::Root.as_ptr() {
        // SAFETY: `bone` is the fresh non-null element pushed above.
        unsafe {
            (*bone).is_root = true;
        }
    }

    Ok(())
}

// ufbx.c:13979-13990 `ufbxi_read_marker`
#[inline(never)]
pub(crate) fn read_marker(
    uc: &Context,
    node: &NodeView,
    info: &ElementInfoView,
    sub_type: &[u8],
    type_: MarkerType,
) -> Result<(), Fail> {
    let _ = node; // C: `(void)node;`
    let _ = sub_type; // C: `(void)sub_type;`

    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Marker` is the element struct for `ElementType::Marker`.
    let marker: *mut Marker = unsafe { push_element::<Marker>(uc, info, ElementType::Marker) };
    ufbxi_check!(uc, !marker.is_null(), "marker");

    // SAFETY: `marker` is the fresh non-null element pushed above.
    unsafe {
        (*marker).type_ = type_;
    }

    Ok(())
}

// ufbx.c:13992-14022 `ufbxi_read_skin`
#[inline(never)]
pub(crate) fn read_skin(uc: &Context, node: &NodeView, info: &ElementInfoView) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `SkinDeformer` is the element struct for
    // `ElementType::SkinDeformer`.
    let skin: *mut SkinDeformer =
        unsafe { push_element::<SkinDeformer>(uc, info, ElementType::SkinDeformer) };
    ufbxi_check!(uc, !skin.is_null(), "skin");
    // SAFETY: `skin` is the fresh non-null element pushed above, owned by uc's
    // element buffer — write-capable provenance.
    let skin: &View<SkinDeformer> = unsafe { View::<SkinDeformer>::from_ptr(skin) };

    if let Some(Checked(skinning_type)) =
        find_val1::<Checked<*const u8>>(node, sp::SkinningType.as_ptr())
    {
        // SAFETY: the `'C'` fetch succeeded, so `skinning_type` points at the
        // NUL-terminated parse-tree string `strcmp` requires.
        unsafe {
            if strcmp(skinning_type, b"Rigid\0".as_ptr()) == 0 {
                skin.set_skinning_method(SkinningMethod::Rigid);
            } else if strcmp(skinning_type, b"Linear\0".as_ptr()) == 0 {
                skin.set_skinning_method(SkinningMethod::Linear);
            } else if strcmp(skinning_type, b"DualQuaternion\0".as_ptr()) == 0 {
                skin.set_skinning_method(SkinningMethod::DualQuaternion);
            } else if strcmp(skinning_type, b"Blend\0".as_ptr()) == 0 {
                skin.set_skinning_method(SkinningMethod::BlendedDqLinear);
            }
        }
    }

    let indices: *mut ValueArray = find_array(node, sp::Indexes.as_ptr(), b'i');
    let weights: *mut ValueArray = find_array(node, sp::BlendWeights.as_ptr(), b'r');
    if !indices.is_null() && !weights.is_null() {
        // TODO strict: ufbxi_check(indices->size == weights->size);
        // SAFETY: `dq_vertices_raw()`/`dq_weights_raw()` address the viewed
        // deformer's own lists; `indices`/`weights` are the live array
        // descriptors checked non-null above, whose `'i'`/`'r'` payloads are
        // `size` `u32`s and `size` reals, so the shorter of the two sizes bounds
        // both runs.
        unsafe {
            skin.set_num_dq_weights(min_sz((*indices).size, (*weights).size));
            (*skin.dq_vertices_raw()).data = (*indices).data as *const u32;
            (*skin.dq_weights_raw()).data = (*weights).data as *const Real;
            (*skin.dq_vertices_raw()).count = skin.num_dq_weights();
            (*skin.dq_weights_raw()).count = skin.num_dq_weights();
        }
    }

    Ok(())
}

// ufbx.c:14024-14052 `ufbxi_read_skin_cluster`
#[inline(never)]
pub(crate) fn read_skin_cluster(
    uc: &Context,
    node: &NodeView,
    info: &ElementInfoView,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `SkinCluster` is the element struct for
    // `ElementType::SkinCluster`.
    let cluster: *mut SkinCluster =
        unsafe { push_element::<SkinCluster>(uc, info, ElementType::SkinCluster) };
    ufbxi_check!(uc, !cluster.is_null(), "cluster");

    let indices: *mut ValueArray = find_array(node, sp::Indexes.as_ptr(), b'i');
    let weights: *mut ValueArray = find_array(node, sp::Weights.as_ptr(), b'r');

    if !indices.is_null() && !weights.is_null() {
        // SAFETY: `indices`/`weights` are non-null (checked above) and
        // `find_array` returns the node's own array descriptors, live for as long
        // as the parse tree.
        ufbxi_check!(
            uc,
            unsafe { (*indices).size } == unsafe { (*weights).size },
            "indices->size == weights->size"
        );
        // SAFETY: `cluster` is the fresh non-null element; `indices`/`weights`
        // are the live descriptors checked above, whose `'i'`/`'r'` payloads are
        // `size` `u32`s and `size` reals, of equal length (just checked).
        unsafe {
            (*cluster).num_weights = (*indices).size;
            (*cluster).vertices.data = (*indices).data as *const u32;
            (*cluster).weights.data = (*weights).data as *const Real;
            (*cluster).vertices.count = (*cluster).num_weights;
            (*cluster).weights.count = (*cluster).num_weights;
        }
    }

    let transform: *mut ValueArray = find_array(node, sp::Transform.as_ptr(), b'r');
    let transform_link: *mut ValueArray = find_array(node, sp::TransformLink.as_ptr(), b'r');
    if !transform.is_null() && !transform_link.is_null() {
        // SAFETY: `transform` is non-null (checked above) and is the node's own
        // live array descriptor.
        ufbxi_check!(
            uc,
            unsafe { (*transform).size } >= 16,
            "transform->size >= 16"
        );
        // SAFETY: as above, for `transform_link`.
        ufbxi_check!(
            uc,
            unsafe { (*transform_link).size } >= 16,
            "transform_link->size >= 16"
        );

        // SAFETY: `cluster` is the fresh non-null element, so the field
        // projections view its live `ufbx_matrix` fields with the element's own
        // write-capable provenance; the `transform` and `transform_link`
        // payloads each hold `size >= 16` reals (just checked), the runs
        // `read_transform_matrix` requires.
        unsafe {
            read_transform_matrix(
                View::<Matrix>::from_ptr(&raw mut (*cluster).mesh_node_to_bone),
                &*((*transform).data as *const [Real; 16]),
            );
            read_transform_matrix(
                View::<Matrix>::from_ptr(&raw mut (*cluster).bind_to_world),
                &*((*transform_link).data as *const [Real; 16]),
            );
        }
    }

    Ok(())
}

// ufbx.c:14054-14086 `ufbxi_read_blend_channel`
#[inline(never)]
pub(crate) fn read_blend_channel(
    uc: &Context,
    node: &NodeView,
    info: &ElementInfoView,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `BlendChannel` is the element struct for
    // `ElementType::BlendChannel`.
    let channel: *mut BlendChannel =
        unsafe { push_element::<BlendChannel>(uc, info, ElementType::BlendChannel) };
    ufbxi_check!(uc, !channel.is_null(), "channel");

    // C: `ufbx_real_list list = { NULL, 0 };`
    // SAFETY: `ufbx_real_list` is a plain pointer/count pair, for which the
    // all-zero bit pattern is a valid (empty, null-data) value.
    let mut list: List<Real> = unsafe { core::mem::zeroed() };
    let full_weights: *mut ValueArray = find_array(node, sp::FullWeights.as_ptr(), b'r');
    if !full_weights.is_null() {
        // SAFETY: `full_weights` is non-null (checked above) and `find_array`
        // returns the node's own array descriptor, live for as long as the parse
        // tree; its `'r'` payload is a run of `size` reals.
        unsafe {
            list.data = (*full_weights).data as *const Real;
            list.count = (*full_weights).size;
        }
    }
    ufbxi_check!(
        uc,
        !uc.tmp_full_weights_view().push_copy_ref(&list).is_null(),
        // C-parity: verbatim post-expansion `#cond` text (see the C11 6.10.3.1
        // note in `sort_shader_prop_bindings`).
        "((ufbx_real_list*)ufbxi_push_size_copy((&uc->tmp_full_weights), sizeof(ufbx_real_list), (1), (&list)))"
    );

    // Blender saves blend shapes with DeformPercent as a field, not a property.
    // However, the animations are mapped to the DeformPercent property.
    let deform_percent = find_child(node, sp::DeformPercent.as_ptr());
    // C: `if (channel->element.props.props.count == 0 && deform_percent)` — the
    // `count == 0` operand is checked first, then the `deform_percent` presence;
    // both are side-effect free, so the nested form preserves the `&&` order.
    // SAFETY: `channel` is the fresh non-null element pushed above.
    if unsafe { (*channel).element.props.props.count } == 0 {
        if let Some(deform_percent) = deform_percent {
            let num_shape_props: usize = 1;
            let shape_props: *mut Prop = uc.result_view().push_zero::<Prop>(num_shape_props);
            ufbxi_check!(uc, !shape_props.is_null(), "shape_props");
            // SAFETY: `shape_props` is the non-null `num_shape_props == 1`
            // element run just zero-pushed on the result buffer, so index 0 is in
            // bounds; `sp::DeformPercent` is a NUL-terminated static string, as
            // `get_name_key_c` requires.
            unsafe {
                (*shape_props.add(0)).name.data = sp::DeformPercent.as_ptr();
                (*shape_props.add(0)).name.length = sp::DeformPercent.len() - 1;
                (*shape_props.add(0))._internal_key = get_name_key(&sp::DeformPercent);
                (*shape_props.add(0)).type_ = PropType::Number;
                (*shape_props.add(0)).value_str = EMPTY_STRING.0;
                // C-parity: `shape_props[0].value_real` is the `ufbx_prop` value
                // union's first real (`value_vec4.x` in the generated struct).
                (*shape_props.add(0)).value_vec4.x = 100.0 as Real;
            }
            if let Some(got) = get_val1::<AsReal>(deform_percent) {
                // SAFETY: `shape_props` is the non-null one-element run pushed above.
                unsafe {
                    (*shape_props.add(0)).value_vec4.x = got.0;
                }
            }
            // SAFETY: `channel` is the fresh non-null element and `shape_props`
            // is the result-owned run of `num_shape_props` props filled in above.
            unsafe {
                (*channel).element.props.props.data = shape_props;
                (*channel).element.props.props.count = num_shape_props;
            }
        }
    }

    Ok(())
}

// ufbx.c:14088-14104 `typedef enum { ... } ufbxi_key_flags;`
// C-parity: `UFBXI_KEY_INTERPOLATION_LINEAR` (ufbx.c:14090) and
// `UFBXI_KEY_TANGENT_AUTO` (ufbx.c:14092) are enumerators defined for
// completeness with zero references in ufbx.c, and `UFBXI_KEY_VELOCITY_RIGHT` /
// `UFBXI_KEY_VELOCITY_NEXT_LEFT` (ufbx.c:14102-14103) are referenced only from
// the `#if 0` block at ufbx.c:14356-14372 (ported as the commented-out block
// below). C does not warn on unreferenced enumerators.
pub(crate) const KEY_INTERPOLATION_CONSTANT: u32 = 0x2;
#[allow(dead_code)]
pub(crate) const KEY_INTERPOLATION_LINEAR: u32 = 0x4;
pub(crate) const KEY_INTERPOLATION_CUBIC: u32 = 0x8;
#[allow(dead_code)]
pub(crate) const KEY_TANGENT_AUTO: u32 = 0x100;
pub(crate) const KEY_TANGENT_TCB: u32 = 0x200;
pub(crate) const KEY_TANGENT_USER: u32 = 0x400;
pub(crate) const KEY_TANGENT_BROKEN: u32 = 0x800;
pub(crate) const KEY_CONSTANT_NEXT: u32 = 0x100;
pub(crate) const KEY_CLAMP: u32 = 0x1000;
pub(crate) const KEY_TIME_INDEPENDENT: u32 = 0x2000;
pub(crate) const KEY_CLAMP_PROGRESSIVE: u32 = 0x4000;
pub(crate) const KEY_WEIGHTED_RIGHT: u32 = 0x1000000;
pub(crate) const KEY_WEIGHTED_NEXT_LEFT: u32 = 0x2000000;
#[allow(dead_code)]
pub(crate) const KEY_VELOCITY_RIGHT: u32 = 0x10000000;
#[allow(dead_code)]
pub(crate) const KEY_VELOCITY_NEXT_LEFT: u32 = 0x20000000;

// ufbx.c:14106-14167 `ufbxi_solve_auto_tangent`
#[inline(never)]
pub(crate) fn solve_auto_tangent(
    uc: &Context,
    prev_time: f64,
    time: f64,
    next_time: f64,
    prev_value: Real,
    value: Real,
    next_value: Real,
    weight_left: f32,
    weight_right: f32,
    auto_bias: f32,
    flags: u32,
) -> f32 {
    // Clamp tangent to zero if near either left or right key
    if flags & KEY_CLAMP != 0 {
        if math::fmin(
            math::fabs(as_f64!(prev_value - value)),
            math::fabs(as_f64!(next_value - value)),
        ) <= uc.opts_view().key_clamp_threshold()
        {
            return 0.0f32;
        }
    }

    // Time-independent: Set the initial slope to be the difference between the two keyframes.
    // C: the `ufbx_real` numerator is promoted to `double` by the `double`
    // denominator; the division itself happens in `double`.
    let mut slope: f64 = as_f64!(next_value - prev_value) / (next_time - prev_time);

    // Non-time-independent tangents seem to blend between left/right tangent and the total difference.
    if (flags & KEY_TIME_INDEPENDENT) == 0 {
        let slope_left: f64 = as_f64!(value - prev_value) / (time - prev_time);
        let slope_right: f64 = as_f64!(next_value - value) / (next_time - time);
        let delta: f64 = (time - prev_time) / (next_time - prev_time);
        slope = slope * 0.5 + (slope_left * (1.0 - delta) + slope_right * delta) * 0.5;

        let bias_weight: f64 = math::fabs(auto_bias as f64) / 100.0;
        if bias_weight > 0.0001 {
            let bias_target: f64 = if auto_bias as f64 > 0.0 {
                slope_right
            } else {
                slope_left
            };
            let bias_delta: f64 = bias_target - slope;
            slope = slope * (1.0 - bias_weight) + bias_target * bias_weight;

            // Auto bias larger than 500 (positive or negative) adds an absolute
            // value to the slope, determined by `((bias-500) / 100)^2 * 40`.
            let abs_bias_weight: f64 = bias_weight - 5.0;
            if abs_bias_weight > 0.0 {
                let mut bias_sign: f64 = if math::fabs(bias_delta) > 0.00001 {
                    bias_delta
                } else {
                    auto_bias as f64
                };
                bias_sign = if bias_sign > 0.0 { 1.0 } else { -1.0 };
                slope += abs_bias_weight * abs_bias_weight * bias_sign * 40.0;
            }
        }
    }

    // Prevent overshooting by clamping the slope in case either
    // tangent goes above/below the endpoints.
    if flags & KEY_CLAMP_PROGRESSIVE != 0 {
        // Split the slope to sign and a non-negative absolute value
        let slope_sign: f64 = if slope >= 0.0 { 1.0 } else { -1.0 };
        let mut abs_slope: f64 = slope_sign * slope;

        // Find limits for the absolute value of the slope
        let range_left: f64 = weight_left as f64 * (time - prev_time);
        let range_right: f64 = weight_right as f64 * (next_time - time);
        let mut max_left: f64 = if range_left > 0.0 {
            slope_sign * as_f64!(value - prev_value) / range_left
        } else {
            0.0
        };
        let mut max_right: f64 = if range_right > 0.0 {
            slope_sign * as_f64!(next_value - value) / range_right
        } else {
            0.0
        };

        // Clamp negative values and NaNs to zero
        if !(max_left > 0.0) {
            max_left = 0.0;
        }
        if !(max_right > 0.0) {
            max_right = 0.0;
        }

        // Clamp the absolute slope from both sides
        if abs_slope > max_left {
            abs_slope = max_left;
        }
        if abs_slope > max_right {
            abs_slope = max_right;
        }

        slope = slope_sign * abs_slope;
    }

    slope as f32
}

// ufbx.c:14169-14190 `ufbxi_solve_auto_tangent_left`
pub(crate) fn solve_auto_tangent_left(
    uc: &Context,
    prev_time: f64,
    time: f64,
    prev_value: Real,
    value: Real,
    weight_left: f32,
    auto_bias: f32,
    flags: u32,
) -> f32 {
    let _ = weight_left; // C: `(void)weight_left;`
    if flags & KEY_CLAMP_PROGRESSIVE != 0 {
        return 0.0f32;
    }
    if flags & KEY_CLAMP != 0 {
        if math::fabs(as_f64!(prev_value - value)) <= uc.opts_view().key_clamp_threshold() {
            return 0.0f32;
        }
    }

    let mut slope: f64 = as_f64!(value - prev_value) / (time - prev_time);

    if (flags & KEY_TIME_INDEPENDENT) == 0 {
        let abs_bias_weight: f64 = math::fabs(auto_bias as f64) / 100.0 - 5.0;
        if abs_bias_weight > 0.0 {
            let bias_sign: f64 = if auto_bias as f64 > 0.0 { 1.0 } else { -1.0 };
            slope += abs_bias_weight * abs_bias_weight * bias_sign * 40.0;
        }
    }

    slope as f32
}

// ufbx.c:14192-14213 `ufbxi_solve_auto_tangent_right`
pub(crate) fn solve_auto_tangent_right(
    uc: &Context,
    time: f64,
    next_time: f64,
    value: Real,
    next_value: Real,
    weight_right: f32,
    auto_bias: f32,
    flags: u32,
) -> f32 {
    let _ = weight_right; // C: `(void)weight_right;`
    if flags & KEY_CLAMP_PROGRESSIVE != 0 {
        return 0.0f32;
    }
    if flags & KEY_CLAMP != 0 {
        if math::fabs(as_f64!(next_value - value)) <= uc.opts_view().key_clamp_threshold() {
            return 0.0f32;
        }
    }

    let mut slope: f64 = as_f64!(next_value - value) / (next_time - time);

    if (flags & KEY_TIME_INDEPENDENT) == 0 {
        let abs_bias_weight: f64 = math::fabs(auto_bias as f64) / 100.0 - 5.0;
        if abs_bias_weight > 0.0 {
            let bias_sign: f64 = if auto_bias as f64 > 0.0 { 1.0 } else { -1.0 };
            slope += abs_bias_weight * abs_bias_weight * bias_sign * 40.0;
        }
    }

    slope as f32
}

// ufbx.c:14215-14225 `ufbxi_solve_tcb`
pub(crate) fn solve_tcb(
    p_slope_left: &mut f32,
    p_slope_right: &mut f32,
    tension: f64,
    continuity: f64,
    bias: f64,
    slope_left: f64,
    slope_right: f64,
    edge: bool,
) {
    let factor: f64 = if edge { 1.0 } else { 0.5 };
    let d00: f64 = factor * (1.0 - tension) * (1.0 + bias) * (1.0 - continuity);
    let d01: f64 = factor * (1.0 - tension) * (1.0 - bias) * (1.0 + continuity);
    let d10: f64 = factor * (1.0 - tension) * (1.0 + bias) * (1.0 + continuity);
    let d11: f64 = factor * (1.0 - tension) * (1.0 - bias) * (1.0 - continuity);

    *p_slope_left = (d00 * slope_left + d01 * slope_right) as f32;
    *p_slope_right = (d10 * slope_left + d11 * slope_right) as f32;
}

// Rust-port infrastructure (not a ufbx.c section): the `ufbx_extrapolation *`
// out-parameter of `ufbxi_read_extrapolation` travels as a view over the curve's
// own extrapolation fields, projected in place out of the (scene-arena) element.
impl View<AnimCurve> {
    #[inline(always)]
    pub(crate) fn pre_extrapolation_view(&self) -> &View<Extrapolation> {
        view_project!(self, pre_extrapolation)
    }

    #[inline(always)]
    pub(crate) fn post_extrapolation_view(&self) -> &View<Extrapolation> {
        view_project!(self, post_extrapolation)
    }
}

impl View<Extrapolation> {
    #[inline(always)]
    pub(crate) fn set_mode(&self, value: ExtrapolationMode) {
        view_write!(self, mode, value)
    }

    #[inline(always)]
    pub(crate) fn set_repeat_count(&self, value: i32) {
        view_write!(self, repeat_count, value)
    }
}

// ufbx.c:14227-14255 `ufbxi_read_extrapolation`
// `name` is the interned static run itself: `ufbxi_find_child` compares names by
// pointer VALUE, so the run's own address is the identity and the bytes are never
// dereferenced.
#[inline(never)]
pub(crate) fn read_extrapolation(
    p_extrapolation: &View<Extrapolation>,
    node: &NodeView,
    name: &[u8],
) {
    let child = find_child(node, name.as_ptr());
    let mut mode: ExtrapolationMode = ExtrapolationMode::Constant;
    let mut repeat_count: i32 = -1;

    if let Some(child) = child {
        // C: `int32_t mode_ch;` — uninitialized, only read when
        // `ufbxi_find_val1()` succeeded and wrote it.
        if let Some(mode_ch) = find_val1::<i32>(child, sp::Type.as_ptr()) {
            // C `switch (mode_ch)` over character literals; Rust patterns
            // cannot contain casts, so the `case` labels are named consts.
            const CASE_A: i32 = b'A' as i32;
            const CASE_C: i32 = b'C' as i32;
            const CASE_K: i32 = b'K' as i32;
            const CASE_M: i32 = b'M' as i32;
            const CASE_R: i32 = b'R' as i32;
            match mode_ch {
                CASE_A => mode = ExtrapolationMode::RepeatRelative,
                CASE_C => mode = ExtrapolationMode::Constant,
                CASE_K => mode = ExtrapolationMode::Slope,
                CASE_M => mode = ExtrapolationMode::Mirror,
                CASE_R => mode = ExtrapolationMode::Repeat,
                _ => { /* Unknown */ }
            }
            if let Some(got) = find_val1::<i32>(child, sp::Repetition.as_ptr()) {
                repeat_count = got;
                if repeat_count < 0 {
                    repeat_count = -1;
                }
            }
        }
    }

    p_extrapolation.set_mode(mode);
    p_extrapolation.set_repeat_count(repeat_count);
}

// ufbx.c:14257-14532 `ufbxi_read_animation_curve`
#[inline(never)]
pub(crate) fn read_animation_curve(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `AnimCurve` is the element struct for `ElementType::AnimCurve`.
    let curve: *mut AnimCurve =
        unsafe { push_element::<AnimCurve>(uc, info, ElementType::AnimCurve) };
    ufbxi_check!(uc, !curve.is_null(), "curve");
    // SAFETY: `curve` is the fresh non-null element pushed above, owned by uc's
    // element buffer — write-capable provenance, live and unmoved for the rest of
    // the load.
    let curve: &View<AnimCurve> = unsafe { View::<AnimCurve>::from_ptr(curve) };

    read_extrapolation(curve.pre_extrapolation_view(), node, &sp::Pre_Extrapolation);
    read_extrapolation(
        curve.post_extrapolation_view(),
        node,
        &sp::Post_Extrapolation,
    );

    if uc.opts_view().ignore_animation() {
        return Ok(());
    }

    // C: `ufbxi_value_array *times, *values, *attr_flags, *attrs, *refs;`
    // — declared uninitialized, each written by the `ufbxi_check(x = ...)`
    // assignment-in-condition below. Each descriptor is minted as a view right
    // after its own null check.
    let times: *mut ValueArray = find_array(node, sp::KeyTime.as_ptr(), b'l');
    ufbxi_check!(
        uc,
        !times.is_null(),
        "times = ufbxi_find_array(node, ufbxi_KeyTime, 'l')"
    );
    // SAFETY: `times` is non-null (checked above) and is the parse node's own
    // arena-owned array descriptor — write-capable provenance, live for as long
    // as the parse tree.
    let times: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(times) };
    let values: *mut ValueArray = find_array(node, sp::KeyValueFloat.as_ptr(), b'r');
    ufbxi_check!(
        uc,
        !values.is_null(),
        "values = ufbxi_find_array(node, ufbxi_KeyValueFloat, 'r')"
    );
    // SAFETY: as for `times` — the node's own live array descriptor.
    let values: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(values) };
    let attr_flags: *mut ValueArray = find_array(node, sp::KeyAttrFlags.as_ptr(), b'i');
    ufbxi_check!(
        uc,
        !attr_flags.is_null(),
        "attr_flags = ufbxi_find_array(node, ufbxi_KeyAttrFlags, 'i')"
    );
    // SAFETY: as for `times` — the node's own live array descriptor.
    let attr_flags: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(attr_flags) };
    let attrs: *mut ValueArray = find_array(node, sp::KeyAttrDataFloat.as_ptr(), b'?');
    ufbxi_check!(
        uc,
        !attrs.is_null(),
        "attrs = ufbxi_find_array(node, ufbxi_KeyAttrDataFloat, '?')"
    );
    // SAFETY: as for `times` — the node's own live array descriptor.
    let attrs: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(attrs) };
    let refs: *mut ValueArray = find_array(node, sp::KeyAttrRefCount.as_ptr(), b'i');
    ufbxi_check!(
        uc,
        !refs.is_null(),
        "refs = ufbxi_find_array(node, ufbxi_KeyAttrRefCount, 'i')"
    );
    // SAFETY: as for `times` — the node's own live array descriptor.
    let refs: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(refs) };

    // Time and value arrays that define the keyframes should be parallel
    ufbxi_check!(
        uc,
        times.size() == values.size(),
        "times->size == values->size"
    );

    // Flags and attributes are run-length encoded where KeyAttrRefCount (refs)
    // is an array that describes how many times to repeat a given flag/attribute.
    // Attributes consist of 4 32-bit floating point values per key.
    ufbxi_check!(
        uc,
        attr_flags.size() == refs.size(),
        "attr_flags->size == refs->size"
    );
    ufbxi_check!(
        uc,
        attrs.size() == refs.size().wrapping_mul(4u32 as usize),
        "attrs->size == refs->size * 4u"
    );

    let num_keys: usize = times.size();
    let keys: *mut Keyframe = uc.result_view().push::<Keyframe>(num_keys);
    ufbxi_check!(uc, !keys.is_null(), "keys");

    // C: `int64_t *p_time = (int64_t*)times->data;`
    //    `ufbx_real *p_value = (ufbx_real*)values->data;`
    //    `int32_t *p_flag = (int32_t*)attr_flags->data;`
    //    `float *p_attr = (float*)attrs->data;`
    //    `int32_t *p_ref = (int32_t*)refs->data, *p_ref_end = p_ref + refs->size;`
    // The five walking cursors are two indices over the payload runs: `i`, the
    // loop counter the `p_time`/`p_value` cursors advance with (one step per
    // key), and `run_ix`, the run-length cursor `p_flag`/`p_attr`/`p_ref` share
    // (`p_attr` addresses the four-float group at `run_ix * 4`). C's
    // `p_ref < p_ref_end` is `run_ix < refs_data.len()`.
    // SAFETY: the `'l'` fetch succeeded, so the `times` payload is a run of
    // `size` `int64_t` written by the array parser (which allocates it
    // 8-aligned), live for as long as the parse tree and never written again
    // during this read.
    let times_data: &[i64] = unsafe { slice_from_ptr(times.data() as *const i64, times.size()) };
    // SAFETY: as above — the `'r'` fetch makes the `values` payload a run of
    // `size` `ufbx_real`.
    let values_data: &[Real] =
        unsafe { slice_from_ptr(values.data() as *const Real, values.size()) };
    // SAFETY: as above — the `'i'` fetch makes the `attr_flags` payload a run of
    // `size` `int32_t`.
    let flags_data: &[i32] =
        unsafe { slice_from_ptr(attr_flags.data() as *const i32, attr_flags.size()) };
    // C-parity: `(float*)attrs->data` reinterprets the `'?'` (any-type) payload.
    // SAFETY: the array parser types a `KeyAttrDataFloat` array as `'i'` or `'f'`
    // (ufbx.c:8188-8191), both four-byte elements laid out 8-aligned, so `size`
    // `f32`s cover exactly the bytes of the payload run.
    let attrs_data: &[f32] = unsafe { slice_from_ptr(attrs.data() as *const f32, attrs.size()) };
    // SAFETY: as for `attr_flags` — the `'i'` payload is a run of `size` `int32_t`.
    let refs_data: &[i32] = unsafe { slice_from_ptr(refs.data() as *const i32, refs.size()) };

    // The previous key defines the weight/slope of the left tangent
    let mut slope_left: f32 = 0.0f32;
    let mut weight_left: f32 = 0.333333f32;
    // float velocity_left = 0.0f;

    let mut prev_time: f64 = 0.0;
    let mut next_time: f64 = 0.0;

    let mut refs_left: i32 = 0;
    let mut run_ix: usize = 0;
    if num_keys > 0 {
        // `times_data` holds `num_keys` entries, which is non-empty in this branch.
        next_time = times_data[0] as f64 / uc.ktime_sec_double();
        if run_ix < refs_data.len() {
            refs_left = refs_data[run_ix];
        }
    }

    // SAFETY: `keys` is the non-null `num_keys`-element run just pushed on the
    // result buffer — one contiguous allocated, write-capable run, live and
    // unmoved for the rest of this function. Its slots may remain
    // uninitialized until their loop iteration writes every keyframe field.
    let keys_run = unsafe { Run::<Keyframe, Mut>::from_raw_parts(keys, num_keys) };
    // C: `for (size_t i = 0; i < num_keys; i++) { ufbx_keyframe *key = &keys[i]; ... }`
    for (i, key) in keys_run.iter().enumerate() {
        ufbxi_check!(uc, refs_left > 0, "refs_left > 0");

        let value: Real = values_data[i];
        if i == 0 {
            curve.set_min_value(value);
            curve.set_max_value(value);
        } else {
            curve.set_min_value(min_real(curve.min_value(), value));
            curve.set_max_value(max_real(curve.max_value(), value));
        }

        key.set_time(next_time);
        key.set_value(value);

        if i + 1 < num_keys {
            next_time = times_data[i + 1] as f64 / uc.ktime_sec_double();
        }

        // `refs_left > 0` (checked above) holds only while `run_ix` is still
        // below `refs_data.len()`, and `flags_data`/`attrs_data` are `refs.size`
        // and `refs.size * 4` long, so the current run's flag and four-float
        // group are in bounds.
        let flags: u32 = flags_data[run_ix] as u32;

        // C: `p_attr[k]` — the four-float group of the current run.
        let attr_ix: usize = run_ix * 4;
        let mut slope_right: f32 = attrs_data[attr_ix];
        let mut weight_right: f32 = 0.333333f32;
        //float velocity_right = 0.0f;
        let mut next_slope_left: f32 = attrs_data[attr_ix + 1];
        let mut next_weight_left: f32 = 0.333333f32;
        // float next_velocity_left = 0.0f;

        if (flags & (KEY_WEIGHTED_RIGHT | KEY_WEIGHTED_NEXT_LEFT)) != 0 {
            // At least one of the tangents is weighted. The weights are encoded as
            // two 0.4 _decimal_ fixed point values that are packed into 32 bits and
            // interpreted as a 32-bit float.
            // C: `uint32_t packed_weights;` + `memcpy(&packed_weights, &p_attr[2], sizeof(uint32_t));`
            // SAFETY: `attrs_data[attr_ix + 2]` is an in-bounds element of the
            // slice vouched above, so its address is a readable four-byte slot;
            // `read_unaligned` copies those bytes without an alignment claim, the
            // `memcpy` C performs.
            let packed_weights: u32 = unsafe {
                (&raw const attrs_data[attr_ix + 2])
                    .cast::<u32>()
                    .read_unaligned()
            };

            if flags & KEY_WEIGHTED_RIGHT != 0 {
                // Right tangent is weighted
                weight_right = (packed_weights & 0xffff) as f32 * 0.0001f32;
            }

            if flags & KEY_WEIGHTED_NEXT_LEFT != 0 {
                // Next left tangent is weighted
                next_weight_left = (packed_weights >> 16) as f32 * 0.0001f32;
            }
        }
        // C: `#if 0` (velocities are parsed but unused)
        //
        //     if ((flags & (UFBXI_KEY_VELOCITY_RIGHT|UFBXI_KEY_VELOCITY_NEXT_LEFT)) != 0) {
        //         // Velocities are encoded in the same way as weights, see above.
        //         uint32_t packed_velocities;
        //         memcpy(&packed_velocities, &p_attr[3], sizeof(uint32_t));
        //
        //         if (flags & UFBXI_KEY_VELOCITY_RIGHT) {
        //             // Right tangent has velocity
        //             velocity_right = (float)(int16_t)(packed_velocities & 0xffff) * 0.0001f;
        //         }
        //
        //         if (flags & UFBXI_KEY_VELOCITY_NEXT_LEFT) {
        //             // Next left tangent has velocity
        //             next_velocity_left = (float)(int16_t)(packed_velocities >> 16) * 0.0001f;
        //         }
        //     }
        //
        // C: `#endif`

        if flags & KEY_INTERPOLATION_CONSTANT != 0 {
            // Constant interpolation: Set cubic tangents to flat.

            if flags & KEY_CONSTANT_NEXT != 0 {
                // Take constant value from next key
                key.set_interpolation(Interpolation::ConstantNext);
            } else {
                // Take constant value from the previous key
                key.set_interpolation(Interpolation::ConstantPrev);
            }

            // C: `weight_right = next_weight_left = 0.333333f;`
            next_weight_left = 0.333333f32;
            weight_right = next_weight_left;
            // C: `slope_right = next_slope_left = 0.0f;`
            next_slope_left = 0.0f32;
            slope_right = next_slope_left;
        } else if flags & KEY_INTERPOLATION_CUBIC != 0 {
            // Cubic interpolation
            key.set_interpolation(Interpolation::Cubic);

            if flags & KEY_TANGENT_TCB != 0 {
                let mut tcb_slope_left: f64 = 0.0;
                let mut tcb_slope_right: f64 = 0.0;
                let mut tcb_edge: bool = false;
                if i > 0 && key.time() > prev_time {
                    // `i > 0` bounds the element preceding element `i` of the
                    // `num_keys`-long `values` run.
                    tcb_slope_left =
                        as_f64!(key.value() - values_data[i - 1]) / (key.time() - prev_time);
                } else {
                    tcb_edge = true;
                }
                if i + 1 < num_keys && next_time > key.time() {
                    // `i + 1 < num_keys` bounds the element following element `i`
                    // of the `values` run.
                    tcb_slope_right =
                        as_f64!(values_data[i + 1] - key.value()) / (next_time - key.time());
                } else {
                    tcb_edge = true;
                }

                solve_tcb(
                    &mut slope_left,
                    &mut slope_right,
                    attrs_data[attr_ix] as f64,
                    attrs_data[attr_ix + 1] as f64,
                    attrs_data[attr_ix + 2] as f64,
                    tcb_slope_left,
                    tcb_slope_right,
                    tcb_edge,
                );

                // TODO: How to handle these?
                next_slope_left = 0.0f32;
                next_weight_left = 0.333333f32;
                // next_velocity_left = 0.0f;
            } else if flags & KEY_TANGENT_USER != 0 {
                // User tangents

                if flags & KEY_TANGENT_BROKEN != 0 {
                    // Broken tangents: No need to modify slopes
                } else {
                    // Unified tangents: Use right slope for both sides
                    // TODO: ??? slope_left = slope_right;
                }
            } else {
                // TODO: Auto break (0x800)

                if i > 0 && i + 1 < num_keys && key.time() > prev_time && next_time > key.time() {
                    if math::fabs((slope_left + slope_right) as f64) <= 0.0001f32 as f64 {
                        // C: `slope_left = slope_right = ufbxi_solve_auto_tangent(...)`
                        // `i > 0` / `i + 1 < num_keys` bound the neighbouring
                        // elements of the `values` run around element `i`.
                        slope_right = solve_auto_tangent(
                            uc,
                            prev_time,
                            key.time(),
                            next_time,
                            values_data[i - 1],
                            key.value(),
                            values_data[i + 1],
                            weight_left,
                            weight_right,
                            slope_right,
                            flags,
                        );
                        slope_left = slope_right;
                    } else {
                        slope_left = solve_auto_tangent(
                            uc,
                            prev_time,
                            key.time(),
                            next_time,
                            values_data[i - 1],
                            key.value(),
                            values_data[i + 1],
                            weight_left,
                            weight_right,
                            -slope_left,
                            flags,
                        );
                        slope_right = solve_auto_tangent(
                            uc,
                            prev_time,
                            key.time(),
                            next_time,
                            values_data[i - 1],
                            key.value(),
                            values_data[i + 1],
                            weight_left,
                            weight_right,
                            slope_right,
                            flags,
                        );
                    }
                } else if i > 0 && key.time() > prev_time {
                    // C: `slope_left = slope_right = ufbxi_solve_auto_tangent_left(...)`
                    // `i > 0` bounds the element preceding element `i`.
                    slope_right = solve_auto_tangent_left(
                        uc,
                        prev_time,
                        key.time(),
                        values_data[i - 1],
                        key.value(),
                        weight_left,
                        -slope_left,
                        flags,
                    );
                    slope_left = slope_right;
                } else if i + 1 < num_keys && next_time > key.time() {
                    // C: `slope_left = slope_right = ufbxi_solve_auto_tangent_right(...)`
                    // `i + 1 < num_keys` bounds the element following element `i`.
                    slope_right = solve_auto_tangent_right(
                        uc,
                        key.time(),
                        next_time,
                        key.value(),
                        values_data[i + 1],
                        weight_right,
                        slope_right,
                        flags,
                    );
                    slope_left = slope_right;
                } else {
                    // Only / invalid keyframe: Set both slopes to zero
                    // C: `slope_left = slope_right = 0.0f;`
                    slope_right = 0.0f32;
                    slope_left = slope_right;
                }

                // ??? Looks like at least MotionBuilder adjusts weight and auto bias to
                // implement velocity and the velocity information in the file is purely
                // for UI (?) If auto bias is not accounted for the velocity computation
                // below results in the correct tangents, but with auto bias the velocity
                // seems to be accounted for twice resulting in incorrect values...
                //
                // C: `#if 0`
                //
                //     if (weight_left >= UFBX_EPSILON) {
                //         slope_left *= (float)(1.0 - ufbx_fmin(velocity_left / weight_left, 1.0));
                //     }
                //     if (weight_right >= UFBX_EPSILON) {
                //         slope_right *= (float)(1.0 - ufbx_fmin(velocity_right / weight_right, 1.0));
                //     }
                //
                // C: `#endif`
            }
        } else {
            // Linear or unknown interpolation: Set cubic tangents to match
            // the linear interpolation with weights of 1/3.
            key.set_interpolation(Interpolation::Linear);

            weight_right = 0.333333f32;
            next_weight_left = 0.333333f32;

            if next_time > key.time() {
                let delta_time: f64 = next_time - key.time();
                if delta_time > 0.0 {
                    // `next_time` still equals `key.time()` on the last key (it is
                    // only advanced while `i + 1 < num_keys`), so reaching this
                    // branch implies `i + 1 < num_keys`, bounding the next element
                    // of the `values` run.
                    let slope: f64 = as_f64!(values_data[i + 1] - key.value()) / delta_time;
                    // C: `slope_right = next_slope_left = (float)slope;`
                    next_slope_left = slope as f32;
                    slope_right = next_slope_left;
                } else {
                    // C: `slope_right = next_slope_left = 0.0f;`
                    next_slope_left = 0.0f32;
                    slope_right = next_slope_left;
                }
            } else {
                // C: `slope_right = next_slope_left = 0.0f;`
                next_slope_left = 0.0f32;
                slope_right = next_slope_left;
            }
        }

        // Set the tangents based on weights (dx relative to the time difference
        // between the previous/next key) and slope (simply d = slope * dx)
        if key.time() > prev_time {
            let delta: f64 = key.time() - prev_time;
            // C: `key->left.dx = (float)(weight_left * delta);`
            //    `key->left.dy = key->left.dx * slope_left;` — `dx` holds the
            //    stored `(float)` value the `dy` product reads back.
            let dx: f32 = (weight_left as f64 * delta) as f32;
            key.set_left(Tangent {
                dx,
                dy: dx * slope_left,
            });
        } else {
            key.set_left(Tangent {
                dx: 0.0f32,
                dy: 0.0f32,
            });
        }

        if next_time > key.time() {
            let delta: f64 = next_time - key.time();
            // C: `key->right.dx = (float)(weight_right * delta);`
            //    `key->right.dy = key->right.dx * slope_right;`
            let dx: f32 = (weight_right as f64 * delta) as f32;
            key.set_right(Tangent {
                dx,
                dy: dx * slope_right,
            });
        } else {
            key.set_right(Tangent {
                dx: 0.0f32,
                dy: 0.0f32,
            });
        }

        slope_left = next_slope_left;
        weight_left = next_weight_left;
        // velocity_left = next_velocity_left;
        prev_time = key.time();

        // Decrement attribute refcount and potentially move to the next one.
        // C: `if (--refs_left == 0)`
        refs_left = refs_left.wrapping_sub(1);
        if refs_left == 0 {
            // C: `p_flag++; p_attr += 4; p_ref++;` — the three cursors share
            // `run_ix`, which the `refs_data.len()` comparison below re-bounds
            // before any read.
            run_ix += 1;
            if run_ix < refs_data.len() {
                refs_left = refs_data[run_ix];
            }
        }
        // C: `p_time++; p_value++;` — the `enumerate` counter advances both.
    }

    // SAFETY: every slot in the result-owned run was fully initialized by the
    // completed loop above, and the result buffer keeps it live and unmoved for
    // every subsequent use of the curve.
    curve
        .keyframes_view()
        .set(unsafe { List::from_raw_parts(keys, num_keys) });

    Ok(())
}

// ufbx.c:14534-14546 `ufbxi_read_material`
#[inline(never)]
pub(crate) fn read_material(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Material` is the element struct for `ElementType::Material`.
    let material: *mut Material =
        unsafe { push_element::<Material>(uc, info, ElementType::Material) };
    ufbxi_check!(uc, !material.is_null(), "material");

    if let Some(got) = find_val1::<Checked<String>>(node, sp::ShadingModel.as_ptr()) {
        // SAFETY: `material` is the fresh non-null element pushed above.
        unsafe {
            (*material).shading_model_name = got.0;
        }
    } else {
        // SAFETY: `material` is the fresh non-null element.
        unsafe {
            (*material).shading_model_name = EMPTY_STRING.0;
        }
    }

    // SAFETY: `material` is the fresh non-null element.
    unsafe {
        (*material).shader_prop_prefix = EMPTY_STRING.0;
    }

    Ok(())
}

// ufbx.c:14548-14570 `ufbxi_read_texture`
#[inline(never)]
pub(crate) fn read_texture(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Texture` is the element struct for `ElementType::Texture`.
    let texture: *mut Texture = unsafe { push_element::<Texture>(uc, info, ElementType::Texture) };
    ufbxi_check!(uc, !texture.is_null(), "texture");

    // SAFETY: `texture` is the fresh non-null element pushed above.
    unsafe {
        (*texture).type_ = TextureType::File;

        (*texture).filename = EMPTY_STRING.0;
        (*texture).absolute_filename = EMPTY_STRING.0;
        (*texture).relative_filename = EMPTY_STRING.0;
    }

    // SAFETY: `texture` is the fresh non-null element pushed above; each fetch
    // yields its value, so the only raw op is the write into its own field.
    unsafe {
        if let Some(got) = find_val1::<Checked<String>>(node, sp::FileName.as_ptr()) {
            (*texture).absolute_filename = got.0;
        }
        if let Some(got) = find_val1::<Checked<String>>(node, sp::Filename.as_ptr()) {
            (*texture).absolute_filename = got.0;
        }
    }
    // SAFETY: as above.
    unsafe {
        if let Some(got) = find_val1::<Checked<String>>(node, sp::RelativeFileName.as_ptr()) {
            (*texture).relative_filename = got.0;
        }
        if let Some(got) = find_val1::<Checked<String>>(node, sp::RelativeFilename.as_ptr()) {
            (*texture).relative_filename = got.0;
        }
    }

    // SAFETY: as above.
    unsafe {
        if let Some(got) = find_val1::<Blob>(node, sp::FileName.as_ptr()) {
            (*texture).raw_absolute_filename = got;
        }
        if let Some(got) = find_val1::<Blob>(node, sp::Filename.as_ptr()) {
            (*texture).raw_absolute_filename = got;
        }
    }
    // SAFETY: as above.
    unsafe {
        if let Some(got) = find_val1::<Blob>(node, sp::RelativeFileName.as_ptr()) {
            (*texture).raw_relative_filename = got;
        }
        if let Some(got) = find_val1::<Blob>(node, sp::RelativeFilename.as_ptr()) {
            (*texture).raw_relative_filename = got;
        }
    }

    Ok(())
}

// ufbx.c:14572-14599 `ufbxi_read_layered_texture`
#[inline(never)]
pub(crate) fn read_layered_texture(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Texture` is the element struct for `ElementType::Texture`.
    let texture: *mut Texture = unsafe { push_element::<Texture>(uc, info, ElementType::Texture) };
    ufbxi_check!(uc, !texture.is_null(), "texture");

    // SAFETY: `texture` is the fresh non-null element pushed above.
    unsafe {
        (*texture).type_ = TextureType::Layered;

        (*texture).filename = EMPTY_STRING.0;
        (*texture).absolute_filename = EMPTY_STRING.0;
        (*texture).relative_filename = EMPTY_STRING.0;
    }

    // SAFETY: `texture` is the fresh non-null element, so its `element_id` is the
    // id `push_element` just assigned to it.
    let extra: *mut TextureExtra =
        unsafe { push_element_extra::<TextureExtra>(uc, (*texture).element.element_id) };
    ufbxi_check!(uc, !extra.is_null(), "extra");

    let alphas: *mut ValueArray = find_array(node, sp::Alphas.as_ptr(), b'r');
    if !alphas.is_null() {
        // SAFETY: `extra` is the fresh non-null extra checked above; `alphas` is
        // non-null (checked) and is the node's own live array descriptor, whose
        // `'r'` payload is a run of `size` reals.
        unsafe {
            (*extra).alphas = (*alphas).data as *mut Real;
            (*extra).num_alphas = (*alphas).size;
        }
    }

    let blend_modes: *mut ValueArray = find_array(node, sp::BlendModes.as_ptr(), b'i');
    if !blend_modes.is_null() {
        // SAFETY: `extra` is the fresh non-null extra; `blend_modes` is non-null
        // (checked) and is the node's own live array descriptor, whose `'i'`
        // payload is a run of `size` `i32`s.
        unsafe {
            (*extra).blend_modes = (*blend_modes).data as *mut i32;
            (*extra).num_blend_modes = (*blend_modes).size;
        }
    }

    Ok(())
}

// ufbx.c:14601-14624 `ufbxi_read_video`
#[inline(never)]
pub(crate) fn read_video(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Video` is the element struct for `ElementType::Video`.
    let video: *mut Video = unsafe { push_element::<Video>(uc, info, ElementType::Video) };
    ufbxi_check!(uc, !video.is_null(), "video");

    // SAFETY: `video` is the fresh non-null element pushed above.
    unsafe {
        (*video).filename = EMPTY_STRING.0;
        (*video).absolute_filename = EMPTY_STRING.0;
        (*video).relative_filename = EMPTY_STRING.0;
    }

    // SAFETY: `video` is the fresh non-null element pushed above; each fetch
    // yields its value, so the only raw op is the write into its own field.
    unsafe {
        if let Some(got) = find_val1::<Checked<String>>(node, sp::FileName.as_ptr()) {
            (*video).absolute_filename = got.0;
        }
        if let Some(got) = find_val1::<Checked<String>>(node, sp::Filename.as_ptr()) {
            (*video).absolute_filename = got.0;
        }
    }
    // SAFETY: as above.
    unsafe {
        if let Some(got) = find_val1::<Checked<String>>(node, sp::RelativeFileName.as_ptr()) {
            (*video).relative_filename = got.0;
        }
        if let Some(got) = find_val1::<Checked<String>>(node, sp::RelativeFilename.as_ptr()) {
            (*video).relative_filename = got.0;
        }
    }

    // SAFETY: as above.
    unsafe {
        if let Some(got) = find_val1::<Blob>(node, sp::FileName.as_ptr()) {
            (*video).raw_absolute_filename = got;
        }
        if let Some(got) = find_val1::<Blob>(node, sp::Filename.as_ptr()) {
            (*video).raw_absolute_filename = got;
        }
    }
    // SAFETY: as above.
    unsafe {
        if let Some(got) = find_val1::<Blob>(node, sp::RelativeFileName.as_ptr()) {
            (*video).raw_relative_filename = got;
        }
        if let Some(got) = find_val1::<Blob>(node, sp::RelativeFilename.as_ptr()) {
            (*video).raw_relative_filename = got;
        }
    }

    let content_node = find_child(node, sp::Content.as_ptr());
    // SAFETY: `video` is the fresh non-null element, so `&raw mut (*video).content`
    // is the live `ufbx_blob` out-slot `read_embedded_blob` writes — arena memory
    // with write-capable provenance, live and unmoved for the call.
    let content_blob: &BlobView = unsafe { BlobView::from_ptr(&raw mut (*video).content) };
    read_embedded_blob(uc, content_blob, content_node)?;

    Ok(())
}

// ufbx.c:14626-14643 `ufbxi_read_anim_stack`
#[inline(never)]
pub(crate) fn read_anim_stack(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    let _ = node; // C: `(void)node;`

    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `AnimStack` is the element struct for `ElementType::AnimStack`.
    let stack: *mut AnimStack =
        unsafe { push_element::<AnimStack>(uc, info, ElementType::AnimStack) };
    ufbxi_check!(uc, !stack.is_null(), "stack");

    let hash: u32 = crate::native::hash::hash_ptr!(info.name_view().data());
    // The map's comparator (`map_cmp_const_char_ptr`) only READS the key slot,
    // so a temporary holding the same interned name pointer stands in for C's
    // `&info->name.data`.
    // SAFETY: `anim_stack_map` stores `TmpAnimStack` items keyed by interned
    // name pointer (`map_cmp_const_char_ptr` follows the stored NUL-terminated
    // string), and `info.name` is an interned string.
    let mut entry: *mut TmpAnimStack = unsafe {
        uc.anim_stack_map_view()
            .find::<TmpAnimStack, _>(hash, &info.name_view().data())
    };
    if entry.is_null() {
        // SAFETY: as for the `find` above — same map, same key.
        entry = unsafe {
            uc.anim_stack_map_view()
                .insert::<TmpAnimStack, _>(hash, &info.name_view().data())
        };
        ufbxi_check!(uc, !entry.is_null(), "entry");
        // SAFETY: `entry` is the fresh non-null map entry checked above, an
        // arena `ufbxi_tmp_anim_stack` whose two fields are written here.
        unsafe {
            (*entry).name = info.name_view().data();
            (*entry).stack = stack;
        }
    }

    Ok(())
}

// ufbx.c:14645-14687 `ufbxi_read_pose`
#[inline(never)]
pub(crate) fn read_pose(
    uc: &Context,
    node: &NodeView,
    info: &ElementInfoView,
    sub_type: &[u8],
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Pose` is the element struct for `ElementType::Pose`.
    let pose: *mut Pose = unsafe { push_element::<Pose>(uc, info, ElementType::Pose) };
    ufbxi_check!(uc, !pose.is_null(), "pose");

    // TODO: What are the actual other types?
    // C-parity: `sub_type` is matched by POINTER IDENTITY against the interned
    // `ufbxi_BindPose` constant, so compare the borrowed run's own address.
    // SAFETY: `pose` is the fresh non-null element pushed above.
    unsafe {
        (*pose).is_bind_pose = sub_type.as_ptr() == sp::BindPose.as_ptr();
    }

    let mut num_bones: usize = 0;
    // C: `ufbxi_for(ufbxi_node, n, node->children, node->num_children)`
    for n in node.children_iter() {
        if n.name() != sp::PoseNode.as_ptr() {
            continue;
        }

        // Bones are linked with FBX names/IDs bypassing the connection system (!?)
        // C: `uint64_t fbx_id;` — written on every path that does not `continue`.
        let mut fbx_id: u64;
        if uc.version() < 7000 {
            let Some(Unchecked(name)) = find_val1::<Unchecked<*const u8>>(n, sp::Node.as_ptr())
            else {
                continue;
            };
            fbx_id = synthetic_id_from_string(uc, name);
            ufbxi_check!(uc, fbx_id != 0, "fbx_id");
        } else {
            let Some(got) = find_val1::<i64>(n, sp::Node.as_ptr()) else {
                continue;
            };
            fbx_id = got as u64;
            validate_fbx_id(uc, &mut fbx_id)?;
        }

        let matrix: *mut ValueArray = find_array(n, sp::Matrix.as_ptr(), b'r');
        if matrix.is_null() {
            continue;
        }
        // SAFETY: `matrix` is non-null (the null case continued above) and
        // `find_array` returns the node's own array descriptor, live for as long
        // as the parse tree.
        ufbxi_check!(uc, unsafe { (*matrix).size } >= 16, "matrix->size >= 16");

        let tmp_pose: *mut TmpBonePose = uc.tmp_stack_view().push::<TmpBonePose>(1);
        ufbxi_check!(uc, !tmp_pose.is_null(), "tmp_pose");

        num_bones += 1;
        // SAFETY: `tmp_pose` is the non-null one-element run just pushed on
        // `tmp_stack`.
        unsafe {
            (*tmp_pose).bone_fbx_id = fbx_id;
        }
        // SAFETY: `tmp_pose` is non-null, so the field projection views a live
        // `ufbx_matrix` with the pushed slot's own write-capable provenance;
        // the `matrix` payload holds `size >= 16` reals (just checked), the run
        // `read_transform_matrix` requires.
        unsafe {
            read_transform_matrix(
                View::<Matrix>::from_ptr(&raw mut (*tmp_pose).bone_to_world),
                &*((*matrix).data as *const [Real; 16]),
            )
        };
    }

    // HACK: Transport `ufbxi_tmp_bone_pose` array through the `ufbx_bone_pose` pointer
    // SAFETY: `pose` is the fresh non-null element pushed above, and the
    // `num_bones` `TmpBonePose` values pushed by the loop are the top of
    // `tmp_stack`, so `push_pop` moves exactly that run into `tmp`.
    unsafe {
        (*pose).bone_poses.count = num_bones;
        (*pose).bone_poses.data = uc
            .tmp_view()
            .push_pop::<TmpBonePose>(uc.tmp_stack_view(), num_bones)
            as *const BonePose;
    }
    // SAFETY: `pose` is the fresh non-null element.
    ufbxi_check!(
        uc,
        !unsafe { (*pose).bone_poses.data }.is_null(),
        "pose->bone_poses.data"
    );

    Ok(())
}

// ufbx.c:14689-14695 `ufbxi_sort_shader_prop_bindings`
#[inline(never)]
pub(crate) fn sort_shader_prop_bindings(
    uc: &Context,
    bindings: Run<'_, ShaderPropBinding>,
) -> Result<(), Fail> {
    // SAFETY: the allocator, data pointer and size slots are uc's own
    // `ator_tmp`/`tmp_arr`/`tmp_arr_size` fields, reached through its views —
    // the matched triple `grow_array` requires.
    ufbxi_check!(
        uc,
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_view(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                bindings
                    .len()
                    .wrapping_mul(size_of::<ShaderPropBinding>()),
            )
        },
        // C-parity: `ufbxi_check`'s `cond` is not preceded by `#`/`##` in its own
        // replacement list, so argument prescan expands `ufbxi_grow_array` before
        // `ufbxi_cond_str` stringifies it (C11 6.10.3.1). Verbatim post-expansion text.
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_shader_prop_binding)))"
    );
    // SAFETY: `bindings` carries a live `ShaderPropBinding` run and
    // `uc.tmp_arr()` was just grown to the run's byte size, so both the input
    // run and merge buffer are in bounds; the comparator only sees elements
    // whose `shader_prop` fields are interned strings.
    unsafe {
        macro_stable_sort::<ShaderPropBinding>(
            32,
            bindings.as_mut_ptr(),
            uc.tmp_arr() as *mut ShaderPropBinding,
            bindings.len(),
            |a, b| sp::str_less((*a).shader_prop.as_bytes(), (*b).shader_prop.as_bytes()),
        )
    };
    Ok(())
}

// ufbx.c:14698-14735 `ufbxi_read_binding_table`
#[inline(never)]
pub(crate) fn read_binding_table(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `ShaderBinding` is the element struct for
    // `ElementType::ShaderBinding`.
    let bindings: *mut ShaderBinding =
        unsafe { push_element::<ShaderBinding>(uc, info, ElementType::ShaderBinding) };
    ufbxi_check!(uc, !bindings.is_null(), "bindings");
    // SAFETY: `bindings` is the fresh non-null result-arena element checked
    // above and stays live and unmoved for the rest of the load.
    let bindings = unsafe { View::<ShaderBinding>::from_ptr(bindings) };

    let mut num_entries: usize = 0;
    // C: `ufbxi_for (ufbxi_node, n, node->children, node->num_children)`
    for n in node.children_iter() {
        if n.name() != sp::Entry.as_ptr() {
            continue;
        }

        // C: `ufbx_string src, dst; const char *src_type, *dst_type;` — all
        // four written by the `"SCSC"` fetch before any read.
        let Some((Checked(src), Checked(src_type), Checked(dst), Checked(dst_type))) =
            get_val4::<Checked<String>, Checked<*const u8>, Checked<String>, Checked<*const u8>>(n)
        else {
            continue;
        };

        if src_type == sp::FbxPropertyEntry.as_ptr() && dst_type == sp::FbxSemanticEntry.as_ptr() {
            let bind = ShaderPropBinding {
                material_prop: src,
                shader_prop: dst,
            };
            ufbxi_check!(
                uc,
                !uc.tmp_stack_view().push_copy_ref(&bind).is_null(),
                "bind"
            );
            num_entries += 1;
        } else if src_type == sp::FbxSemanticEntry.as_ptr()
            && dst_type == sp::FbxPropertyEntry.as_ptr()
        {
            let bind = ShaderPropBinding {
                material_prop: dst,
                shader_prop: src,
            };
            ufbxi_check!(
                uc,
                !uc.tmp_stack_view().push_copy_ref(&bind).is_null(),
                "bind"
            );
            num_entries += 1;
        }
    }

    let prop_bindings = uc
        .result_view()
        .push_pop::<ShaderPropBinding>(uc.tmp_stack_view(), num_entries);
    ufbxi_check!(uc, !prop_bindings.is_null(), "bindings->prop_bindings.data");
    // SAFETY: the `num_entries` initialized values pushed by the loop were the
    // top of `tmp_stack`; `push_pop` moved that complete run into the stable
    // result arena, and its returned base was checked non-null above.
    bindings
        .prop_bindings_view()
        .set(unsafe { List::from_raw_parts(prop_bindings, num_entries) });

    sort_shader_prop_bindings(uc, Run::from_list(bindings.prop_bindings_view()))?;

    Ok(())
}

// ufbx.c:14737-14745 `ufbxi_read_selection_set`
#[inline(never)]
pub(crate) fn read_selection_set(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    let _ = node; // C: `(void)node;`

    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `SelectionSet` is the element struct for
    // `ElementType::SelectionSet`.
    let set: *mut SelectionSet =
        unsafe { push_element::<SelectionSet>(uc, info, ElementType::SelectionSet) };
    ufbxi_check!(uc, !set.is_null(), "set");

    Ok(())
}

// ufbx.c:14747-14754 `ufbxi_find_uint32_list`
// `name` stays a raw `*const u8`: it is an interned pooled-string pointer
// compared by IDENTITY inside `find_child` (never dereferenced), the same
// convention `find_array`/`find_val1` carry as safe fns.
#[inline(never)]
pub(crate) fn find_uint32_list(dst: &View<List<u32>, Mut>, node: &NodeView, name: *const u8) {
    let arr: *mut ValueArray = find_array(node, name, b'i');
    if !arr.is_null() {
        // SAFETY: `arr` is non-null (checked) and `find_array` returns the node's
        // own array descriptor — arena memory, live and unmoved for as long as
        // the parse tree, with write-capable provenance.
        let arr: &View<ValueArray, Mut> = unsafe { View::<ValueArray, Mut>::from_ptr(arr) };
        dst.set_data(arr.data() as *const u32);
        dst.set_count(arr.size());
    }
}

// ufbx.c:14756-14771 `ufbxi_read_selection_node`
#[inline(never)]
pub(crate) fn read_selection_node(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `SelectionNode` is the element struct for
    // `ElementType::SelectionNode`.
    let sel: *mut SelectionNode =
        unsafe { push_element::<SelectionNode>(uc, info, ElementType::SelectionNode) };
    ufbxi_check!(uc, !sel.is_null(), "sel");
    // SAFETY: `sel` is the fresh non-null element pushed above (checked) — scene
    // arena memory, live and unmoved, with write-capable provenance.
    let sel: &View<SelectionNode, Mut> = unsafe { View::<SelectionNode, Mut>::from_ptr(sel) };

    let mut in_set: i32 = 0;
    if let Some(got) = find_val1::<i32>(node, sp::IsTheNodeInSet.as_ptr()) {
        in_set = got;
    }
    // C: `if (ufbxi_find_val1(...) && in_set != 0)` — the write above happens
    // exactly when the fetch succeeds, so the combined test reads as follows.
    if in_set != 0 {
        sel.set_include_node(true);
    }

    find_uint32_list(sel.vertices_view(), node, sp::VertexIndexArray.as_ptr());
    find_uint32_list(sel.edges_view(), node, sp::EdgeIndexArray.as_ptr());
    find_uint32_list(sel.faces_view(), node, sp::PolygonIndexArray.as_ptr());

    Ok(())
}

// ufbx.c:14773-14783 `ufbxi_read_character`
#[inline(never)]
pub(crate) fn read_character(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    let _ = node; // C: `(void)node;`

    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Character` is the element struct for `ElementType::Character`.
    let character: *mut Character =
        unsafe { push_element::<Character>(uc, info, ElementType::Character) };
    ufbxi_check!(uc, !character.is_null(), "character");

    // TODO: There's some extremely cursed all-caps data in characters

    Ok(())
}

// ufbx.c:14785-14798 `ufbxi_read_audio_clip`
#[inline(never)]
pub(crate) fn read_audio_clip(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `AudioClip` is the element struct for `ElementType::AudioClip`.
    let audio: *mut AudioClip =
        unsafe { push_element::<AudioClip>(uc, info, ElementType::AudioClip) };
    ufbxi_check!(uc, !audio.is_null(), "audio");

    // SAFETY: `audio` is the fresh non-null element pushed above.
    unsafe {
        (*audio).filename = EMPTY_STRING.0;
        (*audio).absolute_filename = EMPTY_STRING.0;
        (*audio).relative_filename = EMPTY_STRING.0;
    }

    let content_node = find_child(node, sp::Content.as_ptr());
    // SAFETY: `audio` is the fresh non-null element pushed above, so
    // `&raw mut (*audio).content` is a live `ufbx_blob` destination with
    // write-capable provenance, live and unmoved for the call.
    let content_blob: &BlobView = unsafe { BlobView::from_ptr(&raw mut (*audio).content) };
    read_embedded_blob(uc, content_blob, content_node)?;

    Ok(())
}

// ufbx.c:14800-14803 `typedef struct { ufbx_constraint_type type; const char *name; } ufbxi_constraint_type;`
// Named `ConstraintTypeEntry` rather than the mechanical `ConstraintType`
// because the public `ufbx_constraint_type` already owns that Rust name in
// `generated.rs`.
#[repr(C)]
pub(crate) struct ConstraintTypeEntry {
    pub type_: ConstraintType,
    pub name: *const u8,
}
// The table below is immutable and its `name` pointers reference immutable
// string literals, so sharing is sound (same rationale as `ScaleHelperProp`).
unsafe impl Sync for ConstraintTypeEntry {}

// ufbx.c:14805-14812 `ufbxi_constraint_types`
static CONSTRAINT_TYPES: [ConstraintTypeEntry; 6] = [
    ConstraintTypeEntry {
        type_: ConstraintType::Aim,
        name: b"Aim\0".as_ptr(),
    },
    ConstraintTypeEntry {
        type_: ConstraintType::Parent,
        name: b"Parent-Child\0".as_ptr(),
    },
    ConstraintTypeEntry {
        type_: ConstraintType::Position,
        name: b"Position From Positions\0".as_ptr(),
    },
    ConstraintTypeEntry {
        type_: ConstraintType::Rotation,
        name: b"Rotation From Rotations\0".as_ptr(),
    },
    ConstraintTypeEntry {
        type_: ConstraintType::Scale,
        name: b"Scale From Scales\0".as_ptr(),
    },
    ConstraintTypeEntry {
        type_: ConstraintType::SingleChainIk,
        name: b"Single Chain IK\0".as_ptr(),
    },
];

// ufbx.c:14814-14835 `ufbxi_read_constraint`
#[inline(never)]
pub(crate) fn read_constraint(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    let _ = node; // C: `(void)node;`

    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Constraint` is the element struct for
    // `ElementType::Constraint`.
    let constraint: *mut Constraint =
        unsafe { push_element::<Constraint>(uc, info, ElementType::Constraint) };
    ufbxi_check!(uc, !constraint.is_null(), "constraint");

    if let Some(got) = find_val1::<Checked<String>>(node, sp::Type.as_ptr()) {
        // SAFETY: `constraint` is the fresh non-null element pushed above.
        unsafe {
            (*constraint).type_name = got.0;
        }
    } else {
        // SAFETY: `constraint` is the fresh non-null element pushed above.
        unsafe {
            (*constraint).type_name = EMPTY_STRING.0;
        }
    }

    // C: `ufbxi_for(const ufbxi_constraint_type, ctype, ufbxi_constraint_types, ufbxi_arraycount(ufbxi_constraint_types))`
    let mut ctype: *mut ConstraintTypeEntry = CONSTRAINT_TYPES.as_ptr() as *mut ConstraintTypeEntry;
    let ctype_end = add_ptr(ctype, CONSTRAINT_TYPES.len());
    while ctype != ctype_end {
        // SAFETY: `ctype` walks `CONSTRAINT_TYPES` and stops at `ctype_end`, so it
        // points at a live table entry whose `name` is a NUL-terminated literal;
        // `constraint` is the fresh non-null element and its `type_name.data` is
        // either the fetched interned string or `EMPTY_STRING`, both
        // NUL-terminated.
        if unsafe { strcmp((*constraint).type_name.data, (*ctype).name) } == 0 {
            // SAFETY: as above — `ctype` is in bounds and `constraint` is live.
            unsafe {
                (*constraint).type_ = (*ctype).type_;
            }
            break;
        }
        // SAFETY: `ctype` is in bounds of `CONSTRAINT_TYPES` and stepping it one
        // past the last entry reaches `ctype_end`, the one-past-the-end pointer.
        ctype = unsafe { ctype.add(1) };
    }

    // TODO: There's some extremely cursed all-caps data in characters

    Ok(())
}

// ufbx.c:14837-14939 `ufbxi_read_synthetic_attribute`
//
// # Safety
// `sub_type` and `super_type` must each be NUL-terminated within their own run:
// `strlen` walks `sub_type` from `as_ptr()` for the unknown-attribute fallback
// and `read_unknown` walks `super_type` the same way — an obligation `&[u8]`
// does not carry.
#[inline(never)]
pub(crate) unsafe fn read_synthetic_attribute(
    uc: &Context,
    node: &NodeView,
    info: &ElementInfoView,
    type_str: String,
    sub_type: &[u8],
    super_type: &[u8],
) -> Result<(), Fail> {
    let mut sub_type: &[u8] = sub_type;

    // Some legacy (version 6000) files store mesh nodes without any `sub_type`
    // There seems to be no robust indicator, so detect it from `Vertices` and `PolygonVertexIndex`
    // C-parity: `sub_type` is matched by POINTER IDENTITY against the interned
    // constants, so compare the borrowed run's own address; the interned
    // `ufbxi_Mesh` static carries its own terminator, matching the run-plus-NUL
    // span this parameter takes.
    if sub_type.as_ptr() == EMPTY_CHAR.as_ptr() {
        let node_vertices = find_child(node, sp::Vertices.as_ptr());
        let node_indices = find_child(node, sp::PolygonVertexIndex.as_ptr());
        if node_vertices.is_some() && node_indices.is_some() {
            sub_type = &sp::Mesh[..];
        }
    }

    if (sub_type.as_ptr() == EMPTY_CHAR.as_ptr() || sub_type.as_ptr() == sp::Model.as_ptr())
        && type_str.data == sp::Model.as_ptr()
    {
        // Plain model
        return Ok(());
    }

    // C: `ufbxi_element_info attrib_info = *info;` — struct assignment is a
    // memcpy; `ufbxi_element_info` has no Rust `Copy` (it embeds the generated
    // `ufbx_props`), so read the bytes through the view instead.
    // SAFETY: `info` views the caller's live, initialized `ufbxi_element_info`;
    // the bitwise copy mirrors the C struct assignment and both copies stay in
    // scope only for the duration of this call, matching C's aliasing of the two.
    let mut attrib_info: ElementInfo = unsafe { core::ptr::read(info.get()) };

    attrib_info.fbx_id = push_synthetic_id(uc);

    // Use type and name from NodeAttributeName if it exists *uniquely*
    // C: `ufbx_string type_and_name;` — fully written by `ufbxi_find_val1`
    // before any read; zero-initialized here (no upstream `ufbxi_uninit` marker).
    // SAFETY: `ufbx_string` is a plain pointer/length pair, for which the all-zero
    // bit pattern is a valid (empty, null-data) value.
    if let Some(Unchecked(type_and_name)) =
        find_val1::<Unchecked<String>>(node, sp::NodeAttributeName.as_ptr())
    {
        // C: `ufbx_string attrib_type_str, attrib_name_str;` — both written by
        // `ufbxi_split_type_and_name`; zero-initialized here.
        // SAFETY: `ufbx_string` is a plain pointer/length pair, for which the
        // all-zero bit pattern is a valid (empty, null-data) value.
        let (mut attrib_type_str, mut attrib_name_str): (String, String) =
            unsafe { (core::mem::zeroed(), core::mem::zeroed()) };
        split_type_and_name(
            uc,
            View::from_ref(&type_and_name),
            &mut attrib_type_str,
            &mut attrib_name_str,
        )?;
        if attrib_name_str.length > 0 {
            attrib_info.name = attrib_name_str;
            let attrib_id: u64 = synthetic_id_from_string(uc, type_and_name.data);
            ufbxi_check!(uc, attrib_id != 0, "attrib_id");
            if info.fbx_id() != attrib_id && !fbx_id_exists(uc, attrib_id) {
                attrib_info.fbx_id = attrib_id;
            }
        }
    }

    // 6x00: Link the node to the node attribute so property connections can be
    // redirected from connections if necessary.
    insert_fbx_attr(uc, info.fbx_id(), attrib_info.fbx_id)?;

    // Split properties between the node and the attribute.
    // Consider all user properties as node properties.
    let ps: *mut Prop = info.props_view().props_data();
    let mut dst: usize = 0;
    let mut src: usize = 0;
    let end: usize = info.props_view().props_count();
    while src < end {
        // SAFETY: `src < end` bounds `ps.add(src)` inside the `end`-element prop
        // run `info->props.props` points at, and each prop's `name.data` is a
        // NUL-terminated interned string.
        if unsafe {
            !is_node_property_name(uc, (*ps.add(src)).name.data)
                && ((*ps.add(src)).flags.raw() & PropFlags::USER_DEFINED.raw()) == 0
        } {
            // SAFETY: `uc.tmp_stack_mut_ptr()` is uc's own live `tmp_stack` buf and
            // `src < end` bounds `ps.add(src)`, one readable `ufbx_prop` to copy.
            ufbxi_check!(
                uc,
                !unsafe { uc.tmp_stack_view().push_copy_raw::<Prop>(1, ps.add(src)) }.is_null(),
                // C-parity: verbatim post-expansion `#cond` text (see the C11
                // 6.10.3.1 note in `sort_shader_prop_bindings`).
                "((ufbx_prop*)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbx_prop), (1), (&ps[src])))"
            );
            src += 1;
        } else if dst != src {
            // C: `ps[dst++] = ps[src++];`
            // SAFETY: `dst < src < end` bounds both `ps.add(src)` and `ps.add(dst)`
            // inside the prop run, and `dst != src` makes the two disjoint.
            unsafe { core::ptr::copy_nonoverlapping(ps.add(src), ps.add(dst), 1) };
            dst += 1;
            src += 1;
        } else {
            dst += 1;
            src += 1;
        }
    }
    attrib_info.props.props.count = end - dst;
    attrib_info.props.props.data = uc
        .result_view()
        .push_pop::<Prop>(uc.tmp_stack_view(), attrib_info.props.props.count);
    ufbxi_check!(
        uc,
        !attrib_info.props.props.data.is_null(),
        "attrib_info.props.props.data"
    );
    // SAFETY: `info` views the caller's live `ufbxi_element_info` with
    // write-capable provenance, so the `props.props.count` leaf inside it is
    // writable through the view's pointer.
    unsafe {
        (*info.get()).props.props.count = dst;
    }

    // SAFETY (this whole dispatch): every arm hands the same parse-tree NodeView
    // `node`, a handle on the local `attrib_info`, and interned NUL-terminated
    // `type_str` / `sub_type` / `super_type` strings to the per-type reader
    // selected by the pointer-identity comparisons; each `read_element` arm
    // pairs `size_of::<T>()` with `T`'s own `ElementType`.
    unsafe {
        if sub_type.as_ptr() == sp::Mesh.as_ptr() {
            read_mesh(
                uc,
                node,
                View::<ElementInfo, Mut>::from_mut(&mut attrib_info),
            )?;
        } else if sub_type.as_ptr() == sp::Light.as_ptr() {
            read_element(
                uc,
                node,
                &raw mut attrib_info,
                size_of::<Light>(),
                ElementType::Light,
            )?;
        } else if sub_type.as_ptr() == sp::Camera.as_ptr() {
            read_element(
                uc,
                node,
                &raw mut attrib_info,
                size_of::<Camera>(),
                ElementType::Camera,
            )?;
        } else if sub_type.as_ptr() == sp::LimbNode.as_ptr()
            || sub_type.as_ptr() == sp::Limb.as_ptr()
            || sub_type.as_ptr() == sp::Root.as_ptr()
        {
            read_bone(
                uc,
                node,
                View::<ElementInfo, Mut>::from_mut(&mut attrib_info),
                sub_type,
            )?;
        } else if sub_type.as_ptr() == sp::Null.as_ptr() || sub_type.as_ptr() == sp::Marker.as_ptr()
        {
            read_element(
                uc,
                node,
                &raw mut attrib_info,
                size_of::<Empty>(),
                ElementType::Empty,
            )?;
        } else if sub_type.as_ptr() == sp::NurbsCurve.as_ptr() {
            if find_child(node, sp::KnotVector.as_ptr()).is_none() {
                return Ok(());
            }
            read_nurbs_curve(
                uc,
                node,
                View::<ElementInfo, Mut>::from_mut(&mut attrib_info),
            )?;
        } else if sub_type.as_ptr() == sp::NurbsSurface.as_ptr() {
            if find_child(node, sp::KnotVectorU.as_ptr()).is_none() {
                return Ok(());
            }
            if find_child(node, sp::KnotVectorV.as_ptr()).is_none() {
                return Ok(());
            }
            read_nurbs_surface(
                uc,
                node,
                View::<ElementInfo, Mut>::from_mut(&mut attrib_info),
            )?;
        } else if sub_type.as_ptr() == sp::Line.as_ptr() {
            if find_child(node, sp::Points.as_ptr()).is_none() {
                return Ok(());
            }
            if find_child(node, sp::PointsIndex.as_ptr()).is_none() {
                return Ok(());
            }
            read_line(
                uc,
                node,
                View::<ElementInfo, Mut>::from_mut(&mut attrib_info),
            )?;
        } else if sub_type.as_ptr() == sp::TrimNurbsSurface.as_ptr() {
            if find_child(node, sp::Layer.as_ptr()).is_none() {
                return Ok(());
            }
            read_element(
                uc,
                node,
                &raw mut attrib_info,
                size_of::<NurbsTrimSurface>(),
                ElementType::NurbsTrimSurface,
            )?;
        } else if sub_type.as_ptr() == sp::Boundary.as_ptr() {
            read_element(
                uc,
                node,
                &raw mut attrib_info,
                size_of::<NurbsTrimBoundary>(),
                ElementType::NurbsTrimBoundary,
            )?;
        } else if sub_type.as_ptr() == sp::CameraStereo.as_ptr() {
            read_element(
                uc,
                node,
                &raw mut attrib_info,
                size_of::<StereoCamera>(),
                ElementType::StereoCamera,
            )?;
        } else if sub_type.as_ptr() == sp::CameraSwitcher.as_ptr() {
            read_element(
                uc,
                node,
                &raw mut attrib_info,
                size_of::<CameraSwitcher>(),
                ElementType::CameraSwitcher,
            )?;
        } else if sub_type.as_ptr() == sp::FKEffector.as_ptr() {
            read_marker(
                uc,
                node,
                View::<ElementInfo, Mut>::from_mut(&mut attrib_info),
                sub_type,
                MarkerType::FkEffector,
            )?;
        } else if sub_type.as_ptr() == sp::IKEffector.as_ptr() {
            // SAFETY: as above.
            read_marker(
                uc,
                node,
                View::<ElementInfo, Mut>::from_mut(&mut attrib_info),
                sub_type,
                MarkerType::IkEffector,
            )?;
        } else if sub_type.as_ptr() == sp::LodGroup.as_ptr() {
            read_element(
                uc,
                node,
                &raw mut attrib_info,
                size_of::<LodGroup>(),
                ElementType::LodGroup,
            )?;
        } else {
            // C-parity: the length is `strlen`, not the borrowed run's own
            // length; `sub_type` is NUL-terminated within its run (fn contract).
            let sub_type_str: String = String::new_c(sub_type.as_ptr(), strlen(sub_type.as_ptr()));
            // SAFETY: `super_type` is NUL-terminated within its own run (fn
            // contract), which is what `read_unknown`'s `strlen` walks.
            read_unknown(
                uc,
                node,
                View::<ElementInfo, Mut>::from_mut(&mut attrib_info),
                type_str,
                sub_type_str,
                super_type,
            )?;
        }
    }

    connect_oo(uc, attrib_info.fbx_id, info.fbx_id())?;
    Ok(())
}

// ufbx.c:14941-14945 `ufbxi_read_global_settings`
#[inline(never)]
pub(crate) fn read_global_settings(uc: &Context, node: &NodeView) -> Result<(), Fail> {
    read_properties(uc, node, uc.scene_view().settings_view().props_view())?;
    Ok(())
}

// ufbx.c:14947-15099 `ufbxi_read_object`
#[inline(never)]
pub(crate) fn read_object(uc: &Context, node: &NodeView) -> Result<(), Fail> {
    // SAFETY: `ElementInfo` is a plain C aggregate of scalars, pointers and
    // `Option<Ref<_>>` niches, so the all-zero bit pattern is valid; `node` is
    // a parse-tree NodeView, which is what the DOM lookup expects.
    let mut info: ElementInfo = unsafe { core::mem::zeroed() };
    info.dom_node = get_dom_node(uc, Some(node));

    if node.name() == sp::GlobalSettings.as_ptr() {
        read_global_settings(uc, node)?;
        return Ok(());
    }

    // C: `ufbx_string type_and_name, sub_type_str;` — both written by the
    // `ufbxi_get_val*` fetches below before any read (a failed fetch returns
    // early).
    let type_and_name: String;
    let mut sub_type_str: String;

    // Failing to parse the object properties is not an error since
    // there's some weird objects mixed in every now and then.
    // FBX version 7000 and up uses 64-bit unique IDs per object,
    // older FBX versions just use name/type pairs, which we can
    // use as IDs since all strings are interned into a string pool.
    if uc.version() >= 7000 {
        let Some((fbx_id, Unchecked(tn), Unchecked(st))) =
            get_val3::<i64, Unchecked<String>, Unchecked<String>>(node)
        else {
            return Ok(());
        };
        info.fbx_id = fbx_id as u64;
        type_and_name = tn;
        sub_type_str = st;
        validate_fbx_id(uc, &mut info.fbx_id)?;
    } else {
        let Some((Unchecked(tn), Unchecked(st))) =
            get_val2::<Unchecked<String>, Unchecked<String>>(node)
        else {
            return Ok(());
        };
        type_and_name = tn;
        sub_type_str = st;
        info.fbx_id = synthetic_id_from_string(uc, type_and_name.data);
        ufbxi_check!(uc, info.fbx_id != 0, "info.fbx_id");
    }

    // Remove the "Fbx" prefix from sub-types, remember to re-intern!
    if sub_type_str.length > 3 {
        // SAFETY: `sub_type_str` was filled by the reads above, so it is a
        // pooled string of `length` readable bytes.
        let sub_type_bytes = unsafe { slice_from_ptr(sub_type_str.data, sub_type_str.length) };
        if memcmp(&sub_type_bytes[..3], b"Fbx") == 0 {
            sub_type_str.data = unsafe { sub_type_str.data.add(3) };
            sub_type_str.length -= 3;
            push_string_place_str(
                uc.string_pool_view(),
                StringView::from_mut(&mut sub_type_str),
                false,
            )?;
        }
    }

    // C: `ufbx_string type_str;` — fully written by `ufbxi_split_type_and_name`.
    // SAFETY: all-zero is a valid `ufbx_string`.
    let mut type_str: String = unsafe { core::mem::zeroed() };
    split_type_and_name(
        uc,
        View::from_ref(&type_and_name),
        &mut type_str,
        &mut info.name,
    )?;

    let name: *const u8 = node.name();
    let sub_type: *const u8 = sub_type_str.data;
    read_properties(uc, node, PropsView::from_mut(&mut info.props))?;
    // `find_template` matches on the interned runs' ADDRESSES, so borrow the
    // bytes `name` / `sub_type` already point at. `slice::from_raw_parts` (not
    // `slice_from_ptr`) keeps a zero-length run's own pointer, which the
    // identity comparison needs.
    // SAFETY: `name` is the parse node's pooled name, readable for
    // `node.name_len()` bytes, and `sub_type` is the pooled `sub_type_str`,
    // readable for its own `length` bytes; interned pool strings are non-null
    // and are never moved or rewritten.
    let name_bytes: &[u8] = unsafe { core::slice::from_raw_parts(name, node.name_len() as usize) };
    // `read_unknown` stores the run's own pointer into `unknown->super_type.data`
    // and measures it with `strlen`, so its borrow spans the pooled name plus the
    // terminator that walk ends on.
    // SAFETY: as above, plus the trailing NUL the string pool writes after every
    // interned run.
    let name_run: &[u8] =
        unsafe { core::slice::from_raw_parts(name, node.name_len() as usize + 1) };
    // SAFETY: as above.
    let sub_type_bytes: &[u8] =
        unsafe { core::slice::from_raw_parts(sub_type, sub_type_str.length) };
    // `read_synthetic_attribute` measures `sub_type` with `strlen`, so its
    // borrow spans the pooled run plus the terminator that walk ends on.
    // SAFETY: as above, plus the trailing NUL the string pool writes after every
    // interned run.
    let sub_type_run: &[u8] =
        unsafe { core::slice::from_raw_parts(sub_type, sub_type_str.length + 1) };
    // SAFETY: the template lookup's result points into uc's own template array
    // (or is null, mapped to `None`), which outlives `info`.
    unsafe {
        info.props.defaults = opt_ref(find_template(uc, name_bytes, sub_type_bytes));
    }

    // SAFETY (this whole dispatch): every arm hands the same parse-tree
    // NodeView `node`, a handle on the local `info`, and pooled `type_str` /
    // `sub_type_str` / `name` / `sub_type` strings to the per-type reader
    // selected by the pointer-identity comparisons — one logical dispatch, no
    // pointer arithmetic of its own.
    unsafe {
        if name == sp::Model.as_ptr() {
            if uc.version() < 7000 {
                // SAFETY: `sub_type_run` and `name_run` each span a pooled run plus
                // the terminator the callee's `strlen` walks end on.
                read_synthetic_attribute(
                    uc,
                    node,
                    View::<ElementInfo, Mut>::from_mut(&mut info),
                    type_str,
                    sub_type_run,
                    name_run,
                )?;
            }
            read_model(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
        } else if name == sp::NodeAttribute.as_ptr() {
            if sub_type == sp::Light.as_ptr() {
                read_element(
                    uc,
                    node,
                    &raw mut info,
                    size_of::<Light>(),
                    ElementType::Light,
                )?;
            } else if sub_type == sp::Camera.as_ptr() {
                read_element(
                    uc,
                    node,
                    &raw mut info,
                    size_of::<Camera>(),
                    ElementType::Camera,
                )?;
            } else if sub_type == sp::LimbNode.as_ptr()
                || sub_type == sp::Limb.as_ptr()
                || sub_type == sp::Root.as_ptr()
            {
                read_bone(
                    uc,
                    node,
                    View::<ElementInfo, Mut>::from_mut(&mut info),
                    sub_type_bytes,
                )?;
            } else if sub_type == sp::Null.as_ptr() || sub_type == sp::Marker.as_ptr() {
                read_element(
                    uc,
                    node,
                    &raw mut info,
                    size_of::<Empty>(),
                    ElementType::Empty,
                )?;
            } else if sub_type == sp::CameraStereo.as_ptr() {
                read_element(
                    uc,
                    node,
                    &raw mut info,
                    size_of::<StereoCamera>(),
                    ElementType::StereoCamera,
                )?;
            } else if sub_type == sp::CameraSwitcher.as_ptr() {
                read_element(
                    uc,
                    node,
                    &raw mut info,
                    size_of::<CameraSwitcher>(),
                    ElementType::CameraSwitcher,
                )?;
            } else if sub_type == sp::FKEffector.as_ptr() {
                read_marker(
                    uc,
                    node,
                    View::<ElementInfo, Mut>::from_mut(&mut info),
                    sub_type_bytes,
                    MarkerType::FkEffector,
                )?;
            } else if sub_type == sp::IKEffector.as_ptr() {
                // SAFETY: as above.
                read_marker(
                    uc,
                    node,
                    View::<ElementInfo, Mut>::from_mut(&mut info),
                    sub_type_bytes,
                    MarkerType::IkEffector,
                )?;
            } else if sub_type == sp::LodGroup.as_ptr() {
                read_element(
                    uc,
                    node,
                    &raw mut info,
                    size_of::<LodGroup>(),
                    ElementType::LodGroup,
                )?;
            } else {
                // SAFETY: `name_run` spans the pooled node name plus the terminator
                // `read_unknown`'s `strlen` walks to.
                read_unknown(
                    uc,
                    node,
                    View::<ElementInfo, Mut>::from_mut(&mut info),
                    type_str,
                    sub_type_str,
                    name_run,
                )?;
            }
        } else if name == sp::Geometry.as_ptr() {
            if sub_type == sp::Mesh.as_ptr() {
                read_mesh(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
            } else if sub_type == sp::Shape.as_ptr() {
                read_shape(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
            } else if sub_type == sp::NurbsCurve.as_ptr() {
                read_nurbs_curve(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
            } else if sub_type == sp::NurbsSurface.as_ptr() {
                read_nurbs_surface(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
            } else if sub_type == sp::Line.as_ptr() {
                read_line(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
            } else if sub_type == sp::TrimNurbsSurface.as_ptr() {
                read_element(
                    uc,
                    node,
                    &raw mut info,
                    size_of::<NurbsTrimSurface>(),
                    ElementType::NurbsTrimSurface,
                )?;
            } else if sub_type == sp::Boundary.as_ptr() {
                read_element(
                    uc,
                    node,
                    &raw mut info,
                    size_of::<NurbsTrimBoundary>(),
                    ElementType::NurbsTrimBoundary,
                )?;
            } else {
                // SAFETY: `name_run` spans the pooled node name plus the terminator
                // `read_unknown`'s `strlen` walks to.
                read_unknown(
                    uc,
                    node,
                    View::<ElementInfo, Mut>::from_mut(&mut info),
                    type_str,
                    sub_type_str,
                    name_run,
                )?;
            }
        } else if name == sp::Deformer.as_ptr() {
            if sub_type == sp::Skin.as_ptr() {
                read_skin(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
            } else if sub_type == sp::Cluster.as_ptr() {
                read_skin_cluster(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
            } else if sub_type == sp::BlendShape.as_ptr() {
                read_element(
                    uc,
                    node,
                    &raw mut info,
                    size_of::<BlendDeformer>(),
                    ElementType::BlendDeformer,
                )?;
            } else if sub_type == sp::BlendShapeChannel.as_ptr() {
                read_blend_channel(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
            } else if sub_type == sp::VertexCacheDeformer.as_ptr() {
                read_element(
                    uc,
                    node,
                    &raw mut info,
                    size_of::<CacheDeformer>(),
                    ElementType::CacheDeformer,
                )?;
            } else {
                // SAFETY: `name_run` spans the pooled node name plus the terminator
                // `read_unknown`'s `strlen` walks to.
                read_unknown(
                    uc,
                    node,
                    View::<ElementInfo, Mut>::from_mut(&mut info),
                    type_str,
                    sub_type_str,
                    name_run,
                )?;
            }
        } else if name == sp::Material.as_ptr() {
            read_material(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
        } else if name == sp::Texture.as_ptr() {
            read_texture(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
        } else if name == sp::LayeredTexture.as_ptr() {
            read_layered_texture(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
        } else if name == sp::Video.as_ptr() {
            read_video(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
        } else if name == sp::AnimationStack.as_ptr() {
            read_anim_stack(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
        } else if name == sp::AnimationLayer.as_ptr() {
            read_element(
                uc,
                node,
                &raw mut info,
                size_of::<AnimLayer>(),
                ElementType::AnimLayer,
            )?;
        } else if name == sp::AnimationCurveNode.as_ptr() {
            read_element(
                uc,
                node,
                &raw mut info,
                size_of::<AnimValue>(),
                ElementType::AnimValue,
            )?;
        } else if name == sp::AnimationCurve.as_ptr() {
            read_animation_curve(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
        } else if name == sp::Pose.as_ptr() {
            read_pose(
                uc,
                node,
                View::<ElementInfo, Mut>::from_mut(&mut info),
                sub_type_bytes,
            )?;
        } else if name == sp::Implementation.as_ptr() {
            read_element(
                uc,
                node,
                &raw mut info,
                size_of::<Shader>(),
                ElementType::Shader,
            )?;
        } else if name == sp::BindingTable.as_ptr() {
            read_binding_table(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
        } else if name == sp::Collection.as_ptr() {
            if sub_type == sp::SelectionSet.as_ptr() {
                read_selection_set(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
            }
        } else if name == sp::CollectionExclusive.as_ptr() {
            if sub_type == sp::DisplayLayer.as_ptr() {
                read_element(
                    uc,
                    node,
                    &raw mut info,
                    size_of::<DisplayLayer>(),
                    ElementType::DisplayLayer,
                )?;
            }
        } else if name == sp::SelectionNode.as_ptr() {
            read_selection_node(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
        } else if name == sp::Constraint.as_ptr() {
            if sub_type == sp::Character.as_ptr() {
                read_character(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
            } else {
                read_constraint(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
            }
        } else if name == sp::SceneInfo.as_ptr() {
            read_scene_info(uc, node)?;
        } else if name == sp::Cache.as_ptr() {
            read_element(
                uc,
                node,
                &raw mut info,
                size_of::<CacheFile>(),
                ElementType::CacheFile,
            )?;
        } else if name == sp::ObjectMetaData.as_ptr() {
            read_element(
                uc,
                node,
                &raw mut info,
                size_of::<MetadataObject>(),
                ElementType::MetadataObject,
            )?;
        } else if name == sp::AudioLayer.as_ptr() {
            read_element(
                uc,
                node,
                &raw mut info,
                size_of::<AudioLayer>(),
                ElementType::AudioLayer,
            )?;
        } else if name == sp::Audio.as_ptr() {
            read_audio_clip(uc, node, View::<ElementInfo, Mut>::from_mut(&mut info))?;
        } else {
            // SAFETY: `name_run` spans the pooled node name plus the terminator
            // `read_unknown`'s `strlen` walks to.
            read_unknown(
                uc,
                node,
                View::<ElementInfo, Mut>::from_mut(&mut info),
                type_str,
                sub_type_str,
                name_run,
            )?;
        }
    }

    Ok(())
}

// ufbx.c:15101-15121 `ufbxi_read_objects`
#[inline(never)]
pub(crate) fn read_objects(uc: &Context) -> Result<(), Fail> {
    loop {
        // Push a deferred element ID for tagging warnings
        uc.set_p_element_id(uc.tmp_element_id_view().push::<u32>(1));
        ufbxi_check!(uc, !uc.p_element_id().is_null(), "uc->p_element_id");
        // SAFETY: `p_element_id` is the fresh non-null push result checked above.
        unsafe { *uc.p_element_id() = NO_INDEX };
        uc.warnings_view()
            .set_deferred_element_id_plus_one(uc.tmp_element_id_view().num_items() as u32);

        // C: `ufbxi_node *node;` — the `None` `tmp_buf` selects uc's own temp
        // buffer, as in the C call; `None` is the C `node == NULL` end signal.
        let Some(node) = parse_toplevel_child(uc, None)? else {
            break;
        };

        read_object(uc, node)?;

        uc.warnings_view().set_deferred_element_id_plus_one(0);
        uc.set_p_element_id(core::ptr::null_mut());
    }

    Ok(())
}

// ufbx.c:15123-15127 `typedef struct { ufbxi_node **nodes; size_t num_nodes; uint32_t task_index; } ufbxi_object_batch;`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ObjectBatch {
    pub nodes: *mut *mut Node,
    pub num_nodes: usize,
    pub task_index: u32,
}

// ufbx.c:15129-15234 `ufbxi_read_objects_threaded`
//
// Stays `unsafe fn`: it drives the thread pool. `ufbxi_thread_pool` is read and
// written by pool threads while this loop runs, so it is reached ONLY through
// raw field projections (never a view or a whole-struct reference), and the
// ASCII source window is retargeted at `uc->read_buffer` by raw pointer
// arithmetic. The obligation that all of that state is coherent — in particular
// that `tmp_buf` is not cleared while a batch still refers to it — is a
// whole-function invariant with no narrow seam to name.
#[inline(never)]
pub(crate) unsafe fn read_objects_threaded(uc: &Context) -> Result<(), Fail> {
    uc.set_parse_threaded(true);

    let mut parsed_to_end = false;
    // C: `ufbxi_object_batch batches[UFBX_THREAD_GROUP_COUNT]; // ufbxi_uninit`
    // + `memset(batches, 0, sizeof(batches));` — collapsed into a zero
    // initializer (precedent: `bits_counts` in `native::deflate`).
    // SAFETY: `ObjectBatch` holds only integers and a nullable node-run pointer,
    // so the all-zero bit pattern is a valid (empty) batch.
    let mut batches: [ObjectBatch; THREAD_GROUP_COUNT] = unsafe { core::mem::zeroed() }; // ufbxi_uninit

    let mut empty_count: usize = 0;
    let mut batch_index: usize = 0;
    while empty_count < THREAD_GROUP_COUNT {
        // C: `ufbxi_object_batch *batch = &batches[batch_index];` — `batches` is
        // a loader-thread stack local no pool thread can reach, and C's `&`
        // makes no aliasing claim, so the element is addressed in place at each
        // use rather than bound to a `&mut` held live across the pool calls.

        // SAFETY: `uc.thread_pool_mut_ptr()` is uc's own live `thread_pool`.
        unsafe { thread_pool_wait_group(uc.thread_pool_mut_ptr()) }?;

        if batches[batch_index].num_nodes > 0 {
            // C: `ufbxi_for_ptr(ufbxi_node, p_node, batch->nodes, batch->num_nodes)`
            // SAFETY: `batch->nodes` is the `push_pop`-materialized contiguous
            // run of `num_nodes` node pointers this batch owns; the group's
            // tasks are joined (`thread_pool_wait_group` above) and `tmp_buf` is
            // cleared only after the walk, so the run is live and unmoved for
            // the borrow. `ScalarView` (interior-mutable `Cell`) is the element
            // handle, so the borrow coexists with the writes the parse does
            // through uc.
            let p_nodes: &[ScalarView<*mut Node>] = unsafe {
                slice_from_ptr(
                    batches[batch_index].nodes as *const ScalarView<*mut Node>,
                    batches[batch_index].num_nodes,
                )
            };
            for p_node in p_nodes {
                buf_clear(uc.tmp_parse_view());

                // Push a deferred element ID for tagging warnings
                uc.set_p_element_id(uc.tmp_element_id_view().push::<u32>(1));
                ufbxi_check!(uc, !uc.p_element_id().is_null(), "uc->p_element_id");
                // SAFETY: `uc->p_element_id` is the slot just pushed onto
                // `tmp_element_id` and checked non-null, live until that buf is
                // popped.
                unsafe { *uc.p_element_id() = NO_INDEX };
                uc.warnings_view()
                    .set_deferred_element_id_plus_one(uc.tmp_element_id_view().num_items() as u32);

                // SAFETY: the run entry points at a `tmp_buf`-allocated parse
                // node kept live until the batch is retired.
                read_object(uc, unsafe { NodeView::from_ptr(p_node.get()) })?;

                uc.warnings_view().set_deferred_element_id_plus_one(0);
                uc.set_p_element_id(core::ptr::null_mut());
            }
            batches[batch_index].num_nodes = 0;
        }

        let tmp_buf: &BufView = uc.tmp_thread_parse_at(batch_index);

        // ASCII data may be in `tmp_buf`, so copy it to safety in case
        if uc.ascii_view().src_buf() == tmp_buf.get() {
            // C: `ufbxi_ascii *ua = &uc->ascii;`
            let ua: &AsciiView = uc.ascii_view();
            // SAFETY: `src`/`src_end` delimit one source window, so the two are
            // derived from the same allocation and their difference is well
            // defined.
            let size: usize = to_size(unsafe { ua.src_end().offset_from(ua.src()) });
            if uc.read_buffer_size() < size {
                // SAFETY: `uc.ator_tmp_mut_ptr()`, `uc.read_buffer_mut_ptr()` and
                // `uc.read_buffer_size_mut_ptr()` are uc's own live `ator_tmp`,
                // `read_buffer` and `read_buffer_size` slots, and the buffer's
                // element type `u8` matches the `T` requested.
                ufbxi_check!(
                    uc,
                    unsafe {
                        grow_array::<u8>(
                            uc.ator_tmp_view(),
                            uc.read_buffer_mut_ptr(),
                            uc.read_buffer_size_mut_ptr(),
                            size,
                        )
                    },
                    // C-parity: verbatim post-expansion `#cond` text (see the C11
                    // 6.10.3.1 note in `sort_shader_prop_bindings`).
                    "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->read_buffer)), (&uc->read_buffer), (&uc->read_buffer_size), (size))"
                );
            }
            // SAFETY: `ua.src()` spans the `size` bytes computed from the window
            // above, `uc.read_buffer()` has room for `size` bytes (grown when it
            // was short), and the source window lives in `tmp_buf` while the read
            // buffer is a separate `ator_tmp` allocation, so the two are disjoint.
            unsafe { core::ptr::copy_nonoverlapping(ua.src(), uc.read_buffer(), size) };
            // C: `uc->data = uc->data_begin = ua->src = uc->read_buffer;`
            ua.set_src(uc.read_buffer());
            uc.set_data_begin(ua.src());
            uc.set_data(uc.data_begin());
            ua.set_src_end(uc.read_buffer().wrapping_add(size));
            ua.set_src_is_retained(false);
            ua.set_src_buf(core::ptr::null_mut());
            // SAFETY: `src`/`src_end` address the same `read_buffer` allocation,
            // so their difference is well defined.
            if to_size(unsafe { ua.src_end().offset_from(ua.src()) }) < uc.progress_interval() {
                ua.set_src_yield(ua.src_end());
            } else {
                ua.set_src_yield(ua.src().wrapping_add(uc.progress_interval()));
            }
            uc.set_data(ua.src());
        }

        buf_clear(tmp_buf);

        if !parsed_to_end {
            let mut num_nodes: usize = 0;
            // SAFETY: `uc.get()` is the live, initialized context this call runs
            // on; pool threads read and write the same `thread_pool`, so these
            // are raw field projections, never a view over the sub-struct.
            let (task_start, mut max_tasks): (u32, u32) = unsafe {
                (
                    (*uc.get()).thread_pool.start_index,
                    (*uc.get()).thread_pool.num_tasks / THREAD_GROUP_COUNT as u32,
                )
            };
            // SAFETY: `uc.thread_pool_mut_ptr()` is uc's own live `thread_pool`.
            max_tasks = min32(max_tasks, unsafe {
                thread_pool_available_tasks(uc.thread_pool_mut_ptr())
            });
            let max_memory: usize =
                uc.opts_view().thread_opts_view().memory_limit() / THREAD_GROUP_COUNT;

            loop {
                // C: `ufbxi_node *node;` — `None` is the C `node == NULL` end
                // signal. The batch array stores raw node pointers, so the
                // returned view is unwrapped back to one right away.
                let Some(node) = parse_toplevel_child(uc, Some(tmp_buf))? else {
                    parsed_to_end = true;
                    break;
                };
                let node: *mut Node = node.get();
                ufbxi_check!(
                    uc,
                    !uc.tmp_stack_view().push_copy_ref(&node).is_null(),
                    // C-parity: verbatim post-expansion `#cond` text (see the C11
                    // 6.10.3.1 note in `sort_shader_prop_bindings`).
                    "((ufbxi_node**)ufbxi_push_size_copy((&uc->tmp_stack), sizeof(ufbxi_node*), (1), (&node)))"
                );
                num_nodes += 1;

                // SAFETY: as for the `start_index` read above — a raw field
                // projection into the pool-shared `thread_pool`.
                let num_tasks: u32 =
                    unsafe { (*uc.get()).thread_pool.start_index }.wrapping_sub(task_start);
                if num_tasks >= max_tasks {
                    break;
                }

                let memory_used: usize = tmp_buf.pushed_size() + tmp_buf.pos();
                if memory_used >= max_memory {
                    break;
                }
            }

            batches[batch_index].num_nodes = num_nodes;
            batches[batch_index].nodes =
                tmp_buf.push_pop::<*mut Node>(uc.tmp_stack_view(), num_nodes);
            ufbxi_check!(uc, !batches[batch_index].nodes.is_null(), "batch->nodes");
            // SAFETY: as for the `start_index` read above — a raw field
            // projection into the pool-shared `thread_pool`.
            batches[batch_index].task_index = unsafe { (*uc.get()).thread_pool.start_index };
        }

        // Not safe to refer to this buffer anymore
        uc.ascii_view().set_src_is_retained(false);

        // SAFETY: `uc.thread_pool_mut_ptr()` is uc's own live `thread_pool`.
        unsafe { thread_pool_flush_group(uc.thread_pool_mut_ptr()) };

        if batches[batch_index].num_nodes == 0 {
            empty_count += 1;
        }

        batch_index = (batch_index + 1) % THREAD_GROUP_COUNT;
    }

    // SAFETY: `uc.thread_pool_mut_ptr()` is uc's own live `thread_pool`.
    unsafe { thread_pool_wait_all(uc.thread_pool_mut_ptr()) }?;

    uc.set_parse_threaded(false);

    Ok(())
}

// ufbx.c:15236-15310 `ufbxi_read_connections`
#[inline(never)]
pub(crate) fn read_connections(uc: &Context) -> Result<(), Fail> {
    // Read the connections to the list first
    // C: `ufbxi_node *node;` — the `None` `tmp_buf` selects uc's own temp buffer,
    // as in the C call; the C `for(;;) { ...; if (!node) break; ... }` loop reads
    // as a `while let` over the `None` end signal.
    while let Some(node) = parse_toplevel_child(uc, None)? {
        // C: `uint64_t src_id, dst_id;` — written on every path that does not
        // `continue`.
        let mut src_id: u64;
        let mut dst_id: u64;
        let mut src_prop: String = EMPTY_STRING.0;
        let mut dst_prop: String = EMPTY_STRING.0;

        if uc.version() < 7000 {
            // C: `const char *src_name, *dst_name;` — written on every path
            // that does not `continue`.
            let src_name: *const u8;
            let dst_name: *const u8;
            // Pre-7000 versions use Type::Name pairs as identifiers

            // This branch: `node` is a parse-tree NodeView; the strings the
            // fetches yield are pooled and NUL-terminated, which is what the
            // re-intern and the synthetic-id hashes below require.
            {
                let Some(Unchecked(type_)) = get_val1::<Unchecked<*const u8>>(node) else {
                    continue;
                };

                if type_ == sp::OO.as_ptr() {
                    let Some((Ignore, Unchecked(v1), Unchecked(v2))) =
                        get_val3::<Ignore, Unchecked<*const u8>, Unchecked<*const u8>>(node)
                    else {
                        continue;
                    };
                    src_name = v1;
                    dst_name = v2;
                } else if type_ == sp::OP.as_ptr() {
                    let Some((Ignore, Unchecked(v1), Unchecked(v2), Unchecked(v3))) =
                        get_val4::<
                            Ignore,
                            Unchecked<*const u8>,
                            Unchecked<*const u8>,
                            Unchecked<String>,
                        >(node)
                    else {
                        continue;
                    };
                    src_name = v1;
                    dst_name = v2;
                    dst_prop = v3;
                } else if type_ == sp::PO.as_ptr() {
                    let Some((Ignore, Unchecked(v1), Unchecked(v2), Unchecked(v3))) =
                        get_val4::<
                            Ignore,
                            Unchecked<*const u8>,
                            Unchecked<String>,
                            Unchecked<*const u8>,
                        >(node)
                    else {
                        continue;
                    };
                    src_name = v1;
                    src_prop = v2;
                    dst_name = v3;
                } else if type_ == sp::PP.as_ptr() {
                    let Some((Ignore, Unchecked(v1), Unchecked(v2), Unchecked(v3), Unchecked(v4))) =
                        get_val5::<
                            Ignore,
                            Unchecked<*const u8>,
                            Unchecked<String>,
                            Unchecked<*const u8>,
                            Unchecked<String>,
                        >(node)
                    else {
                        continue;
                    };
                    src_name = v1;
                    src_prop = v2;
                    dst_name = v3;
                    dst_prop = v4;
                } else {
                    // TODO: Strict mode?
                    continue;
                }

                if src_prop.length > 0 {
                    push_string_place_str(
                        uc.string_pool_view(),
                        StringView::from_mut(&mut src_prop),
                        false,
                    )?;
                }
                if dst_prop.length > 0 {
                    push_string_place_str(
                        uc.string_pool_view(),
                        StringView::from_mut(&mut dst_prop),
                        false,
                    )?;
                }

                src_id = synthetic_id_from_string(uc, src_name);
                dst_id = synthetic_id_from_string(uc, dst_name);
                ufbxi_check!(uc, src_id != 0 && dst_id != 0, "src_id && dst_id");
            }
        } else {
            // Post-7000 versions use proper unique 64-bit IDs

            let Some(Checked(type_)) = get_val1::<Checked<*const u8>>(node) else {
                continue;
            };

            if type_ == sp::OO.as_ptr() {
                let Some((Ignore, v1, v2)) = get_val3::<Ignore, i64, i64>(node) else {
                    continue;
                };
                src_id = v1 as u64;
                dst_id = v2 as u64;
            } else if type_ == sp::OP.as_ptr() {
                let Some((Ignore, v1, v2, Checked(v3))) =
                    get_val4::<Ignore, i64, i64, Checked<String>>(node)
                else {
                    continue;
                };
                src_id = v1 as u64;
                dst_id = v2 as u64;
                dst_prop = v3;
            } else if type_ == sp::PO.as_ptr() {
                let Some((Ignore, v1, Checked(v2), v3)) =
                    get_val4::<Ignore, i64, Checked<String>, i64>(node)
                else {
                    continue;
                };
                src_id = v1 as u64;
                src_prop = v2;
                dst_id = v3 as u64;
            } else if type_ == sp::PP.as_ptr() {
                let Some((Ignore, v1, Checked(v2), v3, Checked(v4))) =
                    get_val5::<Ignore, i64, Checked<String>, i64, Checked<String>>(node)
                else {
                    continue;
                };
                src_id = v1 as u64;
                src_prop = v2;
                dst_id = v3 as u64;
                dst_prop = v4;
            } else {
                // TODO: Strict mode?
                continue;
            }

            validate_fbx_id(uc, &mut src_id)?;
            validate_fbx_id(uc, &mut dst_id)?;
        }

        let conn: *mut TmpConnection = uc.tmp_connections_view().push::<TmpConnection>(1);
        ufbxi_check!(uc, !conn.is_null(), "conn");
        // SAFETY: `conn` is the fresh non-null push result checked above and is
        // fully initialized here; the two prop strings are pooled (interned
        // above), so they outlive the connection list.
        unsafe {
            (*conn).src = src_id;
            (*conn).dst = dst_id;
            (*conn).src_prop = src_prop;
            (*conn).dst_prop = dst_prop;
        }
    }

    Ok(())
}

// -- Pre-7000 "Take" based animation
// ufbx.c:15312 banner. The `// -- Reading the parsed data` section above ENDS
// at ufbx.c:15310 (`ufbxi_read_connections`).

// ufbx.c:15314-15321 `ufbxi_double_to_char`
#[inline(always)]
pub(crate) fn double_to_char(value: f64) -> u8 {
    if value >= 0.0 && value <= 127.0 {
        // C-parity: `(char)(int)value`. The guard bounds `value` to [0, 127]
        // so the float→int conversion is in range (no saturation divergence)
        // and the narrowing to `char` is exact. C `char` storage → `u8`
        // (PORTING.md "Naming"); the value is never negative here, so the
        // signed-`char` value rule does not apply.
        value as i32 as u8
    } else {
        0
    }
}

// ufbx.c:15323-15583 `ufbxi_read_take_anim_channel`
///
/// # Safety
/// `name` stays a raw pointer for the reason `push_synthetic_element`
/// documents: it is null or NUL-terminated, the pointer ITSELF is stored in
/// the pushed curve's `element.name.data`, and its bytes must stay live and
/// unmoved for as long as the scene — an obligation no borrow in this port
/// expresses.
#[inline(never)]
pub(crate) unsafe fn read_take_anim_channel(
    uc: &Context,
    node: &NodeView,
    value_fbx_id: u64,
    name: *const u8,
    p_default: &ScalarView<Real>,
) -> Result<(), Fail> {
    if let Some(got) = find_val1::<AsReal>(node, sp::Default.as_ptr()) {
        p_default.set(got.0);
    }

    // Find the key array, early return with success if not found as we may have only a default
    let keys: *mut ValueArray = find_array(node, sp::Key.as_ptr(), b'd');
    if keys.is_null() {
        return Ok(());
    }
    // SAFETY: `keys` is non-null (checked above) and `find_array` returns the
    // node's own array descriptor, live for as long as the parse tree and
    // reached through `*mut` (write-capable provenance for `Mut`).
    let keys: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(keys) };

    let mut curve_fbx_id: u64 = 0;
    // SAFETY: `&raw mut curve_fbx_id` is a live local `uint64_t` slot, `name` is the
    // caller's NUL-terminated channel name (fn contract), and `AnimCurve` is the
    // element struct for `ElementType::AnimCurve`.
    let curve: *mut AnimCurve = unsafe {
        push_synthetic_element::<AnimCurve>(
            uc,
            ScalarView::from_mut(&mut curve_fbx_id),
            Some(node),
            name,
            ElementType::AnimCurve,
        )
    };
    ufbxi_check!(uc, !curve.is_null(), "curve");
    // SAFETY: `curve` is the fresh non-null element pushed above — a live
    // `ufbx_anim_curve` in uc's own result arena, reached through `*mut`
    // (write-capable provenance for `Mut`).
    let curve: &View<AnimCurve> = unsafe { View::<AnimCurve>::from_ptr(curve) };

    // SAFETY: `connect_op` reads the `prop` string it is handed; `curve`'s
    // `element.name` is the interned scene string the push installed.
    unsafe { connect_op(uc, curve_fbx_id, value_fbx_id, curve.element().name()) }?;

    read_extrapolation(curve.pre_extrapolation_view(), node, &sp::Pre_Extrapolation);
    read_extrapolation(
        curve.post_extrapolation_view(),
        node,
        &sp::Post_Extrapolation,
    );

    if uc.opts_view().ignore_animation() {
        return Ok(());
    }

    let mut key_ver: i32 = 0;
    if let Some(got) = find_val1::<i32>(node, sp::KeyVer.as_ptr()) {
        key_ver = got;
    }
    if key_ver <= 0 {
        if uc.version() < 5000 {
            key_ver = 4003;
        } else if uc.version() < 6000 {
            key_ver = 4004;
        } else {
            key_ver = 4005;
        }
    }

    let num_keys: usize = ufbxi_check_some!(
        uc,
        find_val1::<usize>(node, sp::KeyCount.as_ptr()),
        "ufbxi_find_val1(node, ufbxi_KeyCount, \"Z\", &num_keys)"
    );
    let keyframes_data: *mut Keyframe = uc.result_view().push::<Keyframe>(num_keys);
    ufbxi_check!(uc, !keyframes_data.is_null(), "curve->keyframes.data");
    // SAFETY: `keyframes_data` is the non-null, contiguous `num_keys`-slot run
    // just pushed on the result buffer. It is allocated and write-capable, and
    // remains live and unmoved while this function initializes it and for the
    // lifetime of the published curve list.
    let keyframes = unsafe { Run::<Keyframe, Mut>::from_raw_parts(keyframes_data, num_keys) };

    let mut slope_left: f32 = 0.0f32;
    let mut weight_left: f32 = 0.333333f32;

    let mut next_time: f64 = 0.0;
    let mut next_value: f64 = 0.0;
    let mut prev_time: f64 = 0.0;

    // The pre-7000 keyframe data is stored as a _heterogenous_ array containing 64-bit integers,
    // floating point values, and _bare characters_. We cast all values to double and interpret them.
    // C's `double *data, *data_end` pointer pair is an index cursor into the
    // payload run: `data` is the current offset, `data_end` the run length, so
    // `data_end - data` and `data == data_end` read exactly as the C does.
    // SAFETY: `find_array` matched the `'d'` format code, so the descriptor's
    // `data` addresses that payload's `size` contiguous `double`s — a
    // parse-materialized run live and unwritten for as long as the parse tree.
    let data_all: &[f64] = unsafe { slice_from_ptr(keys.data() as *const f64, keys.size()) };
    let mut data: usize = 0;
    let data_end: usize = data_all.len();

    if num_keys > 0 {
        ufbxi_check!(uc, data_end - data >= 2, "data_end - data >= 2");
        next_time = data_all[data] / uc.ktime_sec_double();
        next_value = data_all[data + 1];
    }

    for i in 0..num_keys {
        let key: &View<Keyframe> = keyframes.at(i);

        if i == 0 {
            curve.set_min_value(next_value as Real);
            curve.set_max_value(next_value as Real);
        } else {
            curve.set_min_value(min_real(curve.min_value(), next_value as Real));
            curve.set_max_value(max_real(curve.max_value(), next_value as Real));
        }

        // First three values: Time, Value, InterpolationMode
        ufbxi_check!(uc, data_end - data >= 3, "data_end - data >= 3");
        key.set_time(next_time);
        key.set_value(next_value as Real);
        let mode: u8 = double_to_char(data_all[data + 2]);
        data += 3;

        let mut slope_right: f32 = 0.0f32;
        let mut weight_right: f32 = 0.333333f32;
        let mut next_slope_left: f32 = 0.0f32;
        let mut next_weight_left: f32 = 0.333333f32;
        let mut auto_slope: bool = false;

        if mode == b'U' {
            // Cubic interpolation
            key.set_interpolation(Interpolation::Cubic);

            ufbxi_check!(uc, data_end - data >= 1, "data_end - data >= 1");
            let slope_mode: u8 = double_to_char(data_all[data]);
            data += 1;

            let mut num_weights: usize = 1;
            if slope_mode == b's' || slope_mode == b'b' {
                // Slope mode 's'/'b' (standard? broken?) always have two explicit slopes
                // TODO: `b` might actually be some kind of TCB curve
                ufbxi_check!(uc, data_end - data >= 2, "data_end - data >= 2");
                slope_right = data_all[data] as f32;
                next_slope_left = data_all[data + 1] as f32;
                data += 2;
                // TODO: This looks very suspicious, but we have observed files with
                // KeyVer=4002 -> followed by 'n', then next key
                // KeyVer=4003 -> no weight mode, directly followed by key
                // KeyVer=4004 -> followed by 'n', then next key
                if key_ver == 4003 {
                    num_weights = 0;
                }
            } else if slope_mode == b'a' {
                // Parameterless slope mode 'a' seems to appear in baked animations. Let's just assume
                // automatic tangents for now as they're the least likely to break with
                // objectionable artifacts. We need to defer the automatic tangent resolve
                // until we have read the next time/value.
                // TODO: Solve what this is more thoroughly, using auto slope for now to reduce artifacts
                auto_slope = true;
                if key_ver <= 4004 {
                    num_weights = 0;
                }
            } else if slope_mode == b'p' {
                // TODO: What is this mode? It seems to have negative values sometimes?
                // Also it seems to have _two_ trailing weights values, currently observed:
                // `n,n` and `a,X,Y,n`...
                // Ignore unknown values for now
                // TODO: Solve what this is more thoroughly, using auto slope for now to reduce artifacts
                auto_slope = true;
                ufbxi_check!(uc, data_end - data >= 2, "data_end - data >= 2");
                data += 2;
                if key_ver <= 4004 {
                    num_weights = 1;
                } else {
                    num_weights = 2;
                }
            } else if slope_mode == b'q' {
                // TODO: What is this mode? It seems to have negative values sometimes?
                // Also it seems to have _two_ trailing weights values, currently observed:
                // `d,d` and `n`...
                // Ignore unknown values for now
                // TODO: This has only been observed with KeyVer=4003/4005, it might have two weights in 4004
                // TODO: Solve what this is more thoroughly, using auto slope for now to reduce artifacts
                auto_slope = true;
                ufbxi_check!(uc, data_end - data >= 2, "data_end - data >= 2");
                data += 2;
                if key_ver <= 4004 {
                    num_weights = 1;
                } else {
                    num_weights = 2;
                }
            } else if slope_mode == b't' {
                // TODO: What is this mode? It seems that it does not have any weights and the
                // third value seems _tiny_ (around 1e-30?)
                // TODO: This looks like simple TCB parameters, currently falling back to auto.
                auto_slope = true;
                ufbxi_check!(uc, data_end - data >= 3, "data_end - data >= 3");
                data += 3;
                num_weights = 0;
            } else if slope_mode == b'd' {
                // TODO: What is this mode? It has a single parameter (currently observed `0`)
                // and a single weight.
                // TODO: Solve what this is more thoroughly, using auto slope for now to reduce artifacts
                auto_slope = true;
                ufbxi_check!(uc, data_end - data >= 1, "data_end - data >= 1");
                data += 1;
            } else {
                ufbxi_fail!(uc, "Unknown slope mode");
            }

            // C: `for (; num_weights > 0; num_weights--)`
            while num_weights > 0 {
                ufbxi_check!(uc, data_end - data >= 1, "data_end - data >= 1");
                let weight_mode: u8 = double_to_char(data_all[data]);
                data += 1;

                if weight_mode == b'n' {
                    // Automatic weights (0.3333...)
                } else if weight_mode == b'a' {
                    // Manual weights: RightWeight, NextLeftWeight
                    ufbxi_check!(uc, data_end - data >= 2, "data_end - data >= 2");
                    weight_right = data_all[data] as f32;
                    next_weight_left = data_all[data + 1] as f32;
                    data += 2;
                } else if weight_mode == b'l' {
                    // Next left tangent is weighted
                    ufbxi_check!(uc, data_end - data >= 1, "data_end - data >= 1");
                    next_weight_left = data_all[data] as f32;
                    data += 1;
                } else if weight_mode == b'r' {
                    // Right tangent is weighted
                    ufbxi_check!(uc, data_end - data >= 1, "data_end - data >= 1");
                    weight_right = data_all[data] as f32;
                    data += 1;
                } else if weight_mode == b'c' {
                    // TODO: What is this mode? At least it has no parameters so let's
                    // just assume automatic weights for the time being (0.3333...)
                } else {
                    ufbxi_fail!(uc, "Unknown weight mode");
                }

                num_weights -= 1;
            }
        } else if mode == b'L' {
            // Linear interpolation: No parameters
            key.set_interpolation(Interpolation::Linear);
        } else if mode == b'C' {
            // Constant interpolation: Single parameter (use prev/next)
            if key_ver >= 4004 {
                ufbxi_check!(uc, data_end - data >= 1, "data_end - data >= 1");
                key.set_interpolation(if double_to_char(data_all[data]) == b'n' {
                    Interpolation::ConstantNext
                } else {
                    Interpolation::ConstantPrev
                });
                data += 1;
            } else {
                key.set_interpolation(Interpolation::ConstantPrev);
            }
        } else {
            ufbxi_fail!(uc, "Unknown key mode");
        }

        // Retrieve next key and value
        if i + 1 < num_keys {
            ufbxi_check!(uc, data_end - data >= 2, "data_end - data >= 2");
            next_time = data_all[data] / uc.ktime_sec_double();
            next_value = data_all[data + 1];
        }

        if auto_slope {
            if i > 0 {
                // C: `slope_left = slope_right = ufbxi_solve_auto_tangent(...)`
                // C's `key[-1]` is the previous keyframe of the same run, viewed
                // from the list base — already written by the previous iteration.
                slope_right = solve_auto_tangent(
                    uc,
                    prev_time,
                    key.time(),
                    next_time,
                    keyframes.at(i - 1).value(),
                    key.value(),
                    next_value as Real,
                    weight_left,
                    weight_right,
                    0.0f32,
                    KEY_CLAMP_PROGRESSIVE | KEY_TIME_INDEPENDENT,
                );
                slope_left = slope_right;
            } else {
                // C: `slope_left = slope_right = 0.0f;`
                slope_right = 0.0f32;
                slope_left = slope_right;
            }
        }

        // Set up linear cubic tangents if necessary
        if key.interpolation() == Interpolation::Linear {
            if next_time > key.time() {
                let slope: f64 = (next_value - as_f64!(key.value())) / (next_time - key.time());
                // C: `slope_right = next_slope_left = (float)slope;`
                next_slope_left = slope as f32;
                slope_right = next_slope_left;
            } else {
                // C: `slope_right = next_slope_left = 0.0f;`
                next_slope_left = 0.0f32;
                slope_right = next_slope_left;
            }
        }

        if key.time() > prev_time {
            let delta: f64 = key.time() - prev_time;
            // C: `key->left.dx = (float)(weight_left * delta);`
            //    `key->left.dy = key->left.dx * slope_left;`
            let left_dx: f32 = (weight_left as f64 * delta) as f32;
            key.set_left(Tangent {
                dx: left_dx,
                dy: left_dx * slope_left,
            });
        } else {
            key.set_left(Tangent {
                dx: 0.0f32,
                dy: 0.0f32,
            });
        }

        if next_time > key.time() {
            let delta: f64 = next_time - key.time();
            // C: `key->right.dx = (float)(weight_right * delta);`
            //    `key->right.dy = key->right.dx * slope_right;`
            let right_dx: f32 = (weight_right as f64 * delta) as f32;
            key.set_right(Tangent {
                dx: right_dx,
                dy: right_dx * slope_right,
            });
        } else {
            key.set_right(Tangent {
                dx: 0.0f32,
                dy: 0.0f32,
            });
        }

        slope_left = next_slope_left;
        weight_left = next_weight_left;
        prev_time = key.time();
    }

    ufbxi_check!(uc, data == data_end, "data == data_end");

    // SAFETY: every keyframe slot was fully initialized by the completed loop,
    // the terminal payload-consumption check above succeeded, and the result
    // buffer keeps the run live and unmoved for every later curve use.
    curve
        .keyframes_view()
        .set(unsafe { List::from_raw_parts(keyframes_data, num_keys) });

    Ok(())
}

// Recursion limited as it is further called only for `name="T"/"R"/"S"` and
// cannot enter the `name=="Transform"` branch.
// ufbx.c:15587-15664 `ufbxi_read_take_prop_channel`
// `ufbxi_recursive_function(int, ufbxi_read_take_prop_channel, ..., 2, ...)`
// (ufbx.c:15588-15589): under regression a thread-local depth guard wraps the
// recursive body; otherwise the macro is empty and the wrapper is a plain call.
#[inline(never)]
pub(crate) unsafe fn read_take_prop_channel(
    uc: &Context,
    node: &NodeView,
    target_fbx_id: u64,
    layer_fbx_id: u64,
    name: String,
) -> Result<(), Fail> {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|d| {
            ufbx_assert!(d.get() < 2);
            d.set(d.get() + 1);
        });
        // SAFETY: the wrapper forwards its own arguments unchanged, so `name` is
        // the caller's string over `length` readable bytes (fn contract).
        let ret =
            unsafe { read_take_prop_channel_rec(uc, node, target_fbx_id, layer_fbx_id, name) };
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        ret
    }
    // SAFETY: the wrapper forwards its own arguments unchanged, so `name` is the
    // caller's string over `length` readable bytes (fn contract).
    #[cfg(not(feature = "regression"))]
    unsafe {
        read_take_prop_channel_rec(uc, node, target_fbx_id, layer_fbx_id, name)
    }
}

// ufbx.c:15590-15664 `ufbxi_read_take_prop_channel_rec` (the
// `ufbxi_recursive_function` body; see the wrapper above)
#[inline(never)]
unsafe fn read_take_prop_channel_rec(
    uc: &Context,
    node: &NodeView,
    target_fbx_id: u64,
    layer_fbx_id: u64,
    mut name: String,
) -> Result<(), Fail> {
    if name.data == sp::Transform.as_ptr() {
        // Pre-7000 have transform keyframes in a deeply nested structure,
        // flatten it to make it resemble post-7000 structure a bit closer:
        // old: Model: { Channel: "Transform" { Channel: "T" { Channel "X": { ... } } } }
        // new: Model: { Channel: "Lcl Translation" { Channel "X": { ... } } }

        // C: `ufbxi_for(ufbxi_node, child, node->children, node->num_children)`
        for child in node.children_iter() {
            if child.name() != sp::Channel.as_ptr() {
                continue;
            }

            let old_name: *const u8 = ufbxi_check_some!(
                uc,
                get_val1::<Checked<*const u8>>(child),
                "ufbxi_get_val1(child, \"C\", (char**)&old_name)"
            )
            .0;

            // C: `ufbx_string new_name;` — both fields written in every branch
            // that does not `continue`.
            let new_name: String;
            if old_name == sp::T.as_ptr() {
                new_name =
                    String::new_c(sp::Lcl_Translation.as_ptr(), sp::Lcl_Translation.len() - 1);
            } else if old_name == sp::R.as_ptr() {
                new_name = String::new_c(sp::Lcl_Rotation.as_ptr(), sp::Lcl_Rotation.len() - 1);
            } else if old_name == sp::S.as_ptr() {
                new_name = String::new_c(sp::Lcl_Scaling.as_ptr(), sp::Lcl_Scaling.len() - 1);
            } else {
                continue;
            }

            // Read child as a top-level property channel
            // SAFETY: `new_name` is one of the interned `sp::` literals above, so
            // its `data` spans `length` readable bytes.
            unsafe { read_take_prop_channel(uc, child, target_fbx_id, layer_fbx_id, new_name) }?;
        }
    } else {
        // Pre-6000 FBX files store blend shape keys with a " (Shape)" suffix
        if uc.version() < 6000 {
            let suffix = b" (Shape)";
            let suffix_len = suffix.len();
            // SAFETY: `name` is a live parser string readable for its length.
            let name_bytes = unsafe { name.as_bytes() };
            if name.length > suffix_len
                && memcmp(&name_bytes[name.length - suffix_len..], suffix) == 0
            {
                name.length -= suffix_len;
                // `name` is a live local whose `data` spans the shortened
                // `length` bytes.
                push_string_place_str(
                    uc.string_pool_view(),
                    StringView::from_mut(&mut name),
                    false,
                )?;
            }
        }

        // Find 1-3 channel nodes that contain a `Key:` node
        let mut channel_nodes: [Option<&NodeView>; 3] = [None; 3];
        let mut channel_names: [*const u8; 3] = [core::ptr::null(); 3];
        let mut num_channel_nodes: usize = 0;

        if find_child(node, sp::Key.as_ptr()).is_some()
            || find_child(node, sp::Default.as_ptr()).is_some()
        {
            // Channel has only a single curve
            channel_nodes[0] = Some(node);
            channel_names[0] = name.data;
            num_channel_nodes = 1;
        } else {
            // Channel is a compound of multiple curves
            // C: `ufbxi_for(ufbxi_node, child, node->children, node->num_children)`
            for child in node.children_iter() {
                if child.name() != sp::Channel.as_ptr() {
                    continue;
                }
                if find_child(child, sp::Key.as_ptr()).is_none()
                    && find_child(child, sp::Default.as_ptr()).is_none()
                {
                    continue;
                }
                if let Some(got) = get_val1::<Checked<*const u8>>(child) {
                    channel_names[num_channel_nodes] = got.0;
                } else {
                    continue;
                }
                channel_nodes[num_channel_nodes] = Some(child);
                // C: `if (++num_channel_nodes == 3) break;`
                num_channel_nodes += 1;
                if num_channel_nodes == 3 {
                    break;
                }
            }
        }

        // Early return: No valid channels found, not an error
        if num_channel_nodes == 0 {
            return Ok(());
        }

        let mut value_fbx_id: u64 = 0;
        // Upstream does NOT `ufbxi_check(value)` here (ufbx.c:15650); the
        // deliberate null check after this call is documented below.
        // SAFETY: `&raw mut value_fbx_id` is a live local `uint64_t` slot and
        // `name.data` is the caller's NUL-terminated interned name; `AnimValue` is
        // the element struct for `ElementType::AnimValue`.
        let value: *mut AnimValue = unsafe {
            push_synthetic_element::<AnimValue>(
                uc,
                ScalarView::from_mut(&mut value_fbx_id),
                Some(node),
                name.data,
                ElementType::AnimValue,
            )
        };
        // PORT DIVERGENCE (ufbx.c:15652): C omits this check, so a null `value`
        // from an allocation failure (a failed `push_synthetic_element` does not
        // force the `connect_oo` below to fail) reaches the projection in the
        // loop and is dereferenced. Guard it here, matching every other
        // `push_synthetic_element` site; reconcile once upstream lands the fix.
        ufbxi_check!(uc, !value.is_null(), "value");

        // Add a "virtual" connection between the animated property and the layer/target
        connect_oo(uc, value_fbx_id, layer_fbx_id)?;
        // SAFETY: `name` is the caller's string over `length` readable bytes (fn
        // contract).
        unsafe { connect_op(uc, value_fbx_id, target_fbx_id, name) }?;

        for i in 0..num_channel_nodes {
            // C-parity: `&value->default_value.v[i]` — `ufbx_vec3` is a union
            // of `{ x, y, z }` and `ufbx_real v[3]`; the generator emits only
            // the named-struct member, so the array view is reached by cast.
            // SAFETY: `value` is the live `ufbx_anim_value` null-checked above,
            // `default_value` is a three-`ufbx_real` union and
            // `i < num_channel_nodes <= 3` bounds the step, so the slot is that
            // element's own `ufbx_real`; `ScalarView<Real>` is `repr(transparent)`
            // over `Real` and the arena element is written only through it here.
            let p_default: &ScalarView<Real> = unsafe {
                &*((&raw mut (*value).default_value as *mut Real).add(i) as *const ScalarView<Real>)
            };
            // SAFETY: `channel_names[i]` is the NUL-terminated `'C'` value fetched
            // for that channel — an interned pool string live for the scene.
            unsafe {
                read_take_anim_channel(
                    uc,
                    channel_nodes[i].unwrap(),
                    value_fbx_id,
                    channel_names[i],
                    p_default,
                )
            }?;
        }
    }

    Ok(())
}

// ufbx.c:15666-15686 `ufbxi_read_take_object`
#[inline(never)]
pub(crate) fn read_take_object(
    uc: &Context,
    node: &NodeView,
    layer_fbx_id: u64,
) -> Result<(), Fail> {
    // Takes are used only in pre-7000 FBX versions so objects are identified
    // by their unique Type::Name pair that we use as unique IDs through the
    // pooled interned string pointers.
    let type_and_name: *const u8 = ufbxi_check_some!(
        uc,
        get_val1::<Unchecked<*const u8>>(node),
        "ufbxi_get_val1(node, \"c\", (char**)&type_and_name)"
    )
    .0;
    let target_fbx_id: u64 = synthetic_id_from_string(uc, type_and_name);
    ufbxi_check!(uc, target_fbx_id != 0, "target_fbx_id");

    // Add all suitable Channels as animated properties
    // C: `ufbxi_for(ufbxi_node, child, node->children, node->num_children)`
    for child in node.children_iter() {
        // C: `ufbx_string name;` — written by the `ufbxi_get_val1` guard below.
        // SAFETY: all-zero is a valid `ufbx_string`; `child` is a NodeView from
        // `node`'s own child run and `name` is an unaliased local of exactly
        // the type the `S` format writes, so on success it is pooled and safe
        // to hand to the channel reader.
        if child.name() != sp::Channel.as_ptr() {
            continue;
        }
        unsafe {
            let Some(Checked(name)) = get_val1::<Checked<String>>(child) else {
                continue;
            };

            read_take_prop_channel(uc, child, target_fbx_id, layer_fbx_id, name)?;
        }
    }

    Ok(())
}

// ufbx.c:15688-15749 `ufbxi_read_take`
#[inline(never)]
pub(crate) fn read_take(uc: &Context, node: &NodeView) -> Result<(), Fail> {
    // SAFETY: `ufbx_prop` is a plain C aggregate of strings, scalars and enum
    // tags whose all-zero bit pattern is valid.
    let mut tmp_props: [Prop; 4] = unsafe { core::mem::zeroed() };
    let mut num_props: u32 = 0;

    // C: `int64_t start, stop;` — written by each successful `"LL"` fetch.
    // The synthetic props are initialized in place into the local `tmp_props`
    // array — at most four are ever written, which is its length — from `sp::*`
    // static property names retained by the resulting props.
    if let Some((start, stop)) = find_val2::<i64, i64>(node, sp::LocalTime.as_ptr()) {
        init_synthetic_int_prop(
            &mut tmp_props[num_props as usize],
            &sp::LocalStart,
            start,
            PropType::Integer,
        );
        num_props += 1;
        init_synthetic_int_prop(
            &mut tmp_props[num_props as usize],
            &sp::LocalStop,
            stop,
            PropType::Integer,
        );
        num_props += 1;
    }
    if let Some((start, stop)) = find_val2::<i64, i64>(node, sp::ReferenceTime.as_ptr()) {
        init_synthetic_int_prop(
            &mut tmp_props[num_props as usize],
            &sp::ReferenceStart,
            start,
            PropType::Integer,
        );
        num_props += 1;
        init_synthetic_int_prop(
            &mut tmp_props[num_props as usize],
            &sp::ReferenceStop,
            stop,
            PropType::Integer,
        );
        num_props += 1;
    }

    // C: `const char *name;` — written by the `ufbxi_get_val1` check below.
    let name: *const u8 = ufbxi_check_some!(
        uc,
        get_val1::<Checked<*const u8>>(node),
        "ufbxi_get_val1(node, \"C\", (char**)&name)"
    )
    .0;

    // Hack: For post-7000 files we are only interested in the animation times
    // for fallback in case the information is missing in the stacks.
    if uc.version() >= 7000 {
        let hash: u32 = crate::native::hash::hash_ptr!(name);
        // SAFETY: `anim_stack_map` stores `TmpAnimStack` items keyed by
        // interned name pointer; `name` is the interned take name.
        let entry: *mut TmpAnimStack = unsafe {
            uc.anim_stack_map_view()
                .find::<TmpAnimStack, _>(hash, &name)
        };

        if !entry.is_null() {
            // SAFETY: a non-null entry was filled by `read_anim_stack` with
            // `stack` pointing at a live element in uc's own `tmp_elements`
            // arena, so it may be dereferenced and its `props` run filled.
            unsafe {
                let stack: *mut AnimStack = (*entry).stack;
                if (*stack).element.props.props.count == 0 {
                    (*stack).element.props.props.count = num_props as usize;
                    (*stack).element.props.props.data = uc
                        .result_view()
                        .push_copy_slice(&tmp_props[..num_props as usize]);
                    ufbxi_check!(
                        uc,
                        !(*stack).element.props.props.data.is_null(),
                        "stack->props.props.data"
                    );
                }
            }
        }

        return Ok(());
    }

    let mut stack_fbx_id: u64 = 0;
    let mut layer_fbx_id: u64 = 0;

    // Treat the Take as a post-7000 version animation stack and layer.
    // SAFETY: `stack_fbx_id` is an unaliased local out-param, `node` is a
    // parse-tree NodeView and `name` is the pooled string read above.
    let stack: *mut AnimStack = unsafe {
        push_synthetic_element::<AnimStack>(
            uc,
            ScalarView::from_mut(&mut stack_fbx_id),
            Some(node),
            name,
            ElementType::AnimStack,
        )
    };
    ufbxi_check!(uc, !stack.is_null(), "stack");

    // SAFETY: `stack` is the fresh non-null element checked above.
    unsafe {
        (*stack).element.props.props.count = num_props as usize;
        (*stack).element.props.props.data = uc
            .result_view()
            .push_copy_slice(&tmp_props[..num_props as usize]);
        ufbxi_check!(
            uc,
            !(*stack).element.props.props.data.is_null(),
            "stack->props.props.data"
        );
    }

    // SAFETY: `layer_fbx_id` is an unaliased local out-param, `node` is a
    // parse-tree NodeView and the name is a static pooled string.
    let layer: *mut AnimLayer = unsafe {
        push_synthetic_element::<AnimLayer>(
            uc,
            ScalarView::from_mut(&mut layer_fbx_id),
            Some(node),
            sp::BaseLayer.as_ptr(),
            ElementType::AnimLayer,
        )
    };
    ufbxi_check!(uc, !layer.is_null(), "layer");

    connect_oo(uc, layer_fbx_id, stack_fbx_id)?;

    // Read all properties of objects included in the take
    // C: `ufbxi_for(ufbxi_node, child, node->children, node->num_children)`
    for child in node.children_iter() {
        // TODO: Do some object types have another name?
        if child.name() != sp::Model.as_ptr() {
            continue;
        }

        read_take_object(uc, child, layer_fbx_id)?;
    }

    Ok(())
}

// ufbx.c:15751-15764 `ufbxi_read_takes`
#[inline(never)]
pub(crate) fn read_takes(uc: &Context) -> Result<(), Fail> {
    // C: `ufbxi_node *node;` — the `None` `tmp_buf` selects uc's own temp buffer,
    // as in the C call; the C `for(;;) { ...; if (!node) break; ... }` loop reads
    // as a `while let` over the `None` end signal.
    while let Some(node) = parse_toplevel_child(uc, None)? {
        if node.name() == sp::Take.as_ptr() {
            read_take(uc, node)?;
        }
    }

    Ok(())
}

// ufbx.c:15766-15816 `ufbxi_read_legacy_settings`
#[inline(never)]
pub(crate) fn read_legacy_settings(uc: &Context, node: &NodeView) -> Result<(), Fail> {
    if uc.read_legacy_settings() {
        return Ok(());
    }
    uc.set_read_legacy_settings(true);

    // SAFETY: `ufbx_prop` is a plain C aggregate whose all-zero bit pattern is
    // valid.
    let mut tmp_props: [Prop; 2] = unsafe { core::mem::zeroed() };
    let mut num_props: u32 = 0;

    let frame_rate = find_child_strcmp(node, b"FrameRate");
    if let Some(frame_rate) = frame_rate {
        let mut fps: f64 = 0.0;
        // SAFETY: `frame_rate` is a child NodeView of `node`; `fps` and `str_`
        // are unaliased locals of exactly the types the `D` / `S` formats
        // write, and on success `str_` is a pooled `data`/`length` pair, so the
        // double parse and the end-of-string compare stay inside it. The two
        // synthetic props are written into the local 2-element `tmp_props` from
        // `sp::*` static property names retained by the resulting props.
        unsafe {
            if let Some(got) = get_val1::<f64>(frame_rate) {
                fps = got;
            } else {
                // C: `ufbx_string str;` — written by the `ufbxi_get_val1()` below.
                if let Some(Checked(str_)) = get_val1::<Checked<String>>(frame_rate) {
                    // C: `char *end;` — written by `ufbxi_parse_double()`.
                    let mut end: *const u8 = core::ptr::null();
                    let input = str_.as_bytes();
                    let val: f64 = parse_double(input, &mut end, uc.double_parse_flags());
                    if end == input.as_ptr().wrapping_add(input.len()) {
                        fps = val;
                    }
                }
            }
            if fps > 0.0 {
                init_synthetic_real_prop(
                    &mut tmp_props[num_props as usize],
                    &sp::CustomFrameRate,
                    fps as Real,
                    PropType::Number,
                );
                num_props += 1;
                init_synthetic_real_prop(
                    &mut tmp_props[num_props as usize],
                    &sp::TimeMode,
                    // C: `UFBX_TIME_MODE_CUSTOM` implicitly converted to `ufbx_real`.
                    TimeMode::Custom as u32 as Real,
                    PropType::Integer,
                );
                num_props += 1;
            }
        }
    }

    if num_props > 0 {
        let props: &PropsView = uc.scene_view().settings_view().props_view();
        // SAFETY: `new_props` is a fresh `new_count`-element push into uc's
        // result arena, checked non-null, and the two copies fill it exactly:
        // `num_props` entries from the local `tmp_props` (that many were written
        // above) followed by the `num_existing` entries of the current table.
        // The sort/dedup then operate on that run before it is published back
        // into `props` through the view's own list pointer.
        unsafe {
            let num_existing: usize = props.props_count();

            let new_count: usize = num_props as usize + num_existing;
            let new_props: *mut Prop = uc.result_view().push::<Prop>(new_count);
            ufbxi_check!(uc, !new_props.is_null(), "new_props");

            core::ptr::copy_nonoverlapping(tmp_props.as_ptr(), new_props, num_props as usize);
            if num_existing > 0 {
                core::ptr::copy_nonoverlapping(
                    props.props_data(),
                    new_props.add(num_props as usize),
                    num_existing,
                );
            }

            sort_properties(uc, Run::from_raw_parts(new_props, new_count))?;
            (*props.props_raw()).data = new_props;
            (*props.props_raw()).count = new_count;
            deduplicate_properties(props.props_view());
        }
        ufbxi_check!(
            uc,
            !props.props_data().is_null(),
            "uc->scene.settings.props.props.data"
        );
    }

    Ok(())
}

// ufbx.c:15818-15825 `ufbxi_unscaled_transform_to_matrix`
#[inline(never)]
pub(crate) fn unscaled_transform_to_matrix<M: Mode>(t: &View<Transform, M>) -> Matrix {
    // C: `ufbx_transform transform = *t;` — the three leaves copied off the view.
    let mut transform: Transform = Transform {
        translation: t.translation(),
        rotation: t.rotation(),
        scale: t.scale(),
    };
    transform.scale.x = 1.0;
    transform.scale.y = 1.0;
    transform.scale.z = 1.0;
    transform_to_matrix(View::<Transform, Const>::from_ref(&transform))
}

// ufbx.c:15827-15837 `ufbxi_setup_root_node`
#[inline(never)]
pub(crate) fn setup_root_node(uc: &Context, root: &View<UfbxNode, Mut>) {
    if uc.opts_view().use_root_transform() {
        root.set_local_transform(uc.opts_view().root_transform());
        // SAFETY: the projected transform belongs to the live context options
        // view and remains frozen for this value computation.
        let root_transform =
            unsafe { View::<Transform, Const>::from_ptr(uc.opts_view().root_transform_ptr()) };
        root.set_node_to_parent(transform_to_matrix(root_transform));
    } else {
        root.set_local_transform(IDENTITY_TRANSFORM);
        root.set_node_to_parent(IDENTITY_MATRIX);
    }
    root.set_is_root(true);
}

// ufbx.c:15839-15842 `ufbxi_supports_version`
#[inline(always)]
pub(crate) fn supports_version(version: u32) -> bool {
    version >= 3000 && version <= 7700
}

// ufbx.c:15844-15936 `ufbxi_read_root`
#[inline(never)]
pub(crate) fn read_root(uc: &Context) -> Result<(), Fail> {
    // FBXHeaderExtension: Some metadata (optional)
    // SAFETY: every `parse_toplevel` in this function names a static pooled
    // top-level section (or NULL, meaning "no such section") and drives uc's
    // own parse state.
    unsafe { parse_toplevel(uc, sp::FBXHeaderExtension.as_ptr())? };
    read_header_extension(uc)?;

    // The ASCII exporter version is stored in top-level
    if uc.exporter() == Exporter::BlenderAscii {
        // SAFETY: static pooled section name; `top_node_view()` yields the
        // parsed top-level node as a NodeView, and the out-param is uc's own
        // metadata `creator` string slot.
        unsafe { parse_toplevel(uc, sp::Creator.as_ptr())? };
        if let Some(top_node) = uc.top_node_view() {
            if let Some(got) = get_val1::<Checked<String>>(top_node) {
                // SAFETY: `creator_mut_ptr()` addresses uc's own metadata `creator` slot.
                unsafe {
                    *uc.scene_view().metadata_view().creator_mut_ptr() = got.0;
                }
            }
        }
    }

    // Resolve the exporter before continuing
    match_exporter(uc)?;
    if uc.version() < 7000 {
        init_node_prop_names(uc)?;
    }
    // Don't allow changing version from this point onwards
    uc.ascii_view().set_found_version(true);

    // Document: Read root ID
    if uc.version() >= 7000 {
        // SAFETY: static pooled section name (see above).
        unsafe { parse_toplevel(uc, sp::Documents.as_ptr())? };
        read_document(uc)?;
    } else {
        // Pre-7000: Root node has a specific type-name pair "Model::Scene"
        // (or reversed in binary). Use the interned name as ID as usual.
        let mut root_name: *const u8 = if uc.from_ascii() {
            b"Model::Scene\0".as_ptr()
        } else {
            b"Scene\x00\x01Model\0".as_ptr()
        };
        // SAFETY: `root_name` is one of the two byte-string literals above,
        // each 12 bytes plus the NUL, so the requested length is in bounds and
        // the `'static` bytes outlive the pool — which is what makes the no-copy
        // (`copy == false`) intern sound; `p_out_length` is null, which the
        // `raw == true` path never writes. Its result — checked non-null — is
        // the pooled string the synthetic-id hash reads.
        unsafe {
            root_name = sp::push_string_imp(
                uc.string_pool_view(),
                root_name,
                12,
                core::ptr::null_mut(),
                false,
                true,
            );
            ufbxi_check!(uc, !root_name.is_null(), "root_name");
            uc.set_root_id(synthetic_id_from_string(uc, root_name));
        }
        ufbxi_check!(uc, uc.root_id() != 0, "uc->root_id");
    }

    // Add a nameless root node with the root ID
    {
        // C: `ufbxi_element_info root_info = { uc->root_id };`
        // SAFETY: all-zero is a valid `ufbxi_element_info`; `root_info` is an
        // unaliased local whose `name` is the static NUL-terminated empty string
        // and whose `props`/`dom_node` are zeroed, so the pointers the element
        // stores outlive the scene, and `UfbxNode` is the element struct for
        // `ElementType::Node`; `root` is the fresh non-null element the push
        // returns — checked before it is initialized and before its
        // `element_id` is copied into uc's own `tmp_node_ids` buffer.
        unsafe {
            let mut root_info: ElementInfo = core::mem::zeroed();
            root_info.fbx_id = uc.root_id();
            root_info.name = EMPTY_STRING.0;
            let root: *mut UfbxNode = push_element::<UfbxNode>(
                uc,
                View::<ElementInfo>::from_mut(&mut root_info),
                ElementType::Node,
            );
            ufbxi_check!(uc, !root.is_null(), "root");
            // SAFETY: `root` is the fresh non-null element the push returns,
            // living in uc's own write-capable element arena.
            setup_root_node(uc, View::<UfbxNode>::from_ptr(root));
            ufbxi_check!(
            uc,
            !uc.tmp_node_ids_view()
                .push_copy_ref(&(*root).element.element_id)
                .is_null(),
            // C-parity: verbatim post-expansion `#cond` text (see the C11
            // 6.10.3.1 note in `sort_shader_prop_bindings`).
            "((uint32_t*)ufbxi_push_size_copy((&uc->tmp_node_ids), sizeof(uint32_t), (1), (&root->element.element_id)))"
        );
        }
    }

    // Definitions: Object type counts and property templates (optional)
    // SAFETY: static pooled section name (see above).
    unsafe { parse_toplevel(uc, sp::Definitions.as_ptr())? };
    read_definitions(uc)?;

    // Objects: Actual scene data
    // SAFETY: static pooled section name (see above).
    unsafe { parse_toplevel(uc, sp::Objects.as_ptr())? };
    if !uc.sure_fbx() {
        // If the file is a bit iffy about being a real FBX file reject it if
        // even the objects are not found.
        ufbxi_check_msg!(
            uc,
            !uc.top_node().is_null(),
            "Not an FBX file",
            "uc->top_node"
        );
    }
    // SAFETY: reading uc's own `thread_pool.enabled` flag; the threaded reader
    // is entered only with that pool live, which is its own precondition.
    if unsafe { (*uc.get()).thread_pool.enabled } {
        unsafe { read_objects_threaded(uc)? };
    } else {
        read_objects(uc)?;
    }

    // Connections: Relationships between nodes
    // SAFETY: static pooled section name (see above).
    unsafe { parse_toplevel(uc, sp::Connections.as_ptr())? };
    read_connections(uc)?;

    // Takes: Pre-7000 animation data
    // SAFETY: static pooled section name (see above).
    unsafe { parse_toplevel(uc, sp::Takes.as_ptr())? };
    read_takes(uc)?;

    // Check if there's a top-level GlobalSettings that we skimmed over
    // SAFETY: static pooled section name (see above).
    unsafe { parse_toplevel(uc, sp::GlobalSettings.as_ptr())? };
    if let Some(top_node) = uc.top_node_view() {
        read_global_settings(uc, top_node)?;
    }

    // Version5: Pre-6000 settings
    // SAFETY: static pooled section name (see above).
    unsafe { parse_toplevel(uc, sp::Version5.as_ptr())? };
    if let Some(top_node) = uc.top_node_view() {
        let settings = find_child_strcmp(top_node, b"Settings");
        if let Some(settings) = settings {
            read_legacy_settings(uc, settings)?;
        }
    }

    // Force parsing all the nodes by parsing a toplevel that cannot be found
    if uc.opts_view().retain_dom() {
        // SAFETY: a NULL name is the documented "no such section" request.
        unsafe { parse_toplevel(uc, core::ptr::null())? };
    }

    Ok(())
}

// ufbx.c:15938-15943 `typedef struct { const char *prop_name; ufbx_prop_type prop_type; const char *node_name; const char *node_fmt; } ufbxi_legacy_prop;`
#[repr(C)]
pub(crate) struct LegacyProp {
    prop_name: *const u8,
    prop_type: PropType,
    node_name: *const u8,
    node_fmt: *const u8,
}
// Type invariant: every instance is an entry of one of the immutable tables
// below; `prop_name`, `node_name`, and `node_fmt` name NUL-terminated immutable
// statics, and `prop_name` remains live for every scene property that stores it.
// The pointer fields are private so safe code outside this module cannot create
// an entry that violates the invariant. Sharing therefore follows the same
// rationale as `ScaleHelperProp`.
unsafe impl Sync for LegacyProp {}

impl LegacyProp {
    #[inline(always)]
    fn node_name_bytes(&self) -> &[u8] {
        // SAFETY: the type invariant makes `node_name` an immutable
        // NUL-terminated string.
        unsafe { slice_from_ptr(self.node_name, strlen(self.node_name)) }
    }

    #[inline(always)]
    fn format(&self) -> &[u8] {
        // SAFETY: the type invariant makes `node_fmt` an immutable
        // NUL-terminated string.
        unsafe { slice_from_ptr(self.node_fmt, strlen(self.node_fmt)) }
    }
}

// C: `ufbxi_arraycount(ufbxi_legacy_light_props)`
const LEGACY_LIGHT_PROPS_COUNT: usize = 7;

// ufbx.c:15946-15954 `ufbxi_legacy_light_props`
// Must be alphabetically sorted!
static LEGACY_LIGHT_PROPS: [LegacyProp; LEGACY_LIGHT_PROPS_COUNT] = [
    LegacyProp {
        prop_name: sp::CastLight.as_ptr(),
        prop_type: PropType::Boolean,
        node_name: sp::CastLight.as_ptr(),
        node_fmt: b"L\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::CastShadows.as_ptr(),
        prop_type: PropType::Boolean,
        node_name: sp::CastShadows.as_ptr(),
        node_fmt: b"L\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::Color.as_ptr(),
        prop_type: PropType::Color,
        node_name: sp::Color.as_ptr(),
        node_fmt: b"RRR\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::ConeAngle.as_ptr(),
        prop_type: PropType::Number,
        node_name: sp::ConeAngle.as_ptr(),
        node_fmt: b"R\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::HotSpot.as_ptr(),
        prop_type: PropType::Number,
        node_name: sp::HotSpot.as_ptr(),
        node_fmt: b"R\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::Intensity.as_ptr(),
        prop_type: PropType::Number,
        node_name: sp::Intensity.as_ptr(),
        node_fmt: b"R\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::LightType.as_ptr(),
        prop_type: PropType::Integer,
        node_name: sp::LightType.as_ptr(),
        node_fmt: b"L\0".as_ptr(),
    },
];

// C: `ufbxi_arraycount(ufbxi_legacy_camera_props)`
const LEGACY_CAMERA_PROPS_COUNT: usize = 11;

// ufbx.c:15957-15969 `ufbxi_legacy_camera_props`
// Must be alphabetically sorted!
static LEGACY_CAMERA_PROPS: [LegacyProp; LEGACY_CAMERA_PROPS_COUNT] = [
    LegacyProp {
        prop_name: sp::ApertureMode.as_ptr(),
        prop_type: PropType::Integer,
        node_name: sp::ApertureMode.as_ptr(),
        node_fmt: b"L\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::AspectH.as_ptr(),
        prop_type: PropType::Number,
        node_name: sp::AspectH.as_ptr(),
        node_fmt: b"R\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::AspectRatioMode.as_ptr(),
        prop_type: PropType::Integer,
        node_name: b"AspectType\0".as_ptr(),
        node_fmt: b"L\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::AspectW.as_ptr(),
        prop_type: PropType::Number,
        node_name: sp::AspectW.as_ptr(),
        node_fmt: b"R\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::FieldOfView.as_ptr(),
        prop_type: PropType::Number,
        node_name: b"Aperture\0".as_ptr(),
        node_fmt: b"R\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::FieldOfViewX.as_ptr(),
        prop_type: PropType::Number,
        node_name: b"FieldOfViewXProperty\0".as_ptr(),
        node_fmt: b"R\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::FieldOfViewY.as_ptr(),
        prop_type: PropType::Number,
        node_name: b"FieldOfViewYProperty\0".as_ptr(),
        node_fmt: b"R\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::FilmHeight.as_ptr(),
        prop_type: PropType::Number,
        node_name: b"CameraAperture\0".as_ptr(),
        node_fmt: b"_R\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::FilmSqueezeRatio.as_ptr(),
        prop_type: PropType::Number,
        node_name: b"SqueezeRatio\0".as_ptr(),
        node_fmt: b"R\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::FilmWidth.as_ptr(),
        prop_type: PropType::Number,
        node_name: b"CameraAperture\0".as_ptr(),
        node_fmt: b"R_\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::FocalLength.as_ptr(),
        prop_type: PropType::Number,
        node_name: sp::FocalLength.as_ptr(),
        node_fmt: b"R\0".as_ptr(),
    },
];

// C: `ufbxi_arraycount(ufbxi_legacy_bone_props)`
const LEGACY_BONE_PROPS_COUNT: usize = 1;

// ufbx.c:15972-15974 `ufbxi_legacy_bone_props`
// Must be alphabetically sorted!
static LEGACY_BONE_PROPS: [LegacyProp; LEGACY_BONE_PROPS_COUNT] = [LegacyProp {
    prop_name: sp::Size.as_ptr(),
    prop_type: PropType::Number,
    node_name: sp::Size.as_ptr(),
    node_fmt: b"R\0".as_ptr(),
}];

// C: `ufbxi_arraycount(ufbxi_legacy_material_props)`
const LEGACY_MATERIAL_PROPS_COUNT: usize = 6;

// ufbx.c:15977-15984 `ufbxi_legacy_material_props`
// Must be alphabetically sorted!
static LEGACY_MATERIAL_PROPS: [LegacyProp; LEGACY_MATERIAL_PROPS_COUNT] = [
    LegacyProp {
        prop_name: sp::AmbientColor.as_ptr(),
        prop_type: PropType::Color,
        node_name: b"Ambient\0".as_ptr(),
        node_fmt: b"RRR\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::DiffuseColor.as_ptr(),
        prop_type: PropType::Color,
        node_name: b"Diffuse\0".as_ptr(),
        node_fmt: b"RRR\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::EmissiveColor.as_ptr(),
        prop_type: PropType::Color,
        node_name: b"Emissive\0".as_ptr(),
        node_fmt: b"RRR\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::ShadingModel.as_ptr(),
        prop_type: PropType::Color,
        node_name: sp::ShadingModel.as_ptr(),
        node_fmt: b"S\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::Shininess.as_ptr(),
        prop_type: PropType::Number,
        node_name: b"Shininess\0".as_ptr(),
        node_fmt: b"R\0".as_ptr(),
    },
    LegacyProp {
        prop_name: sp::SpecularColor.as_ptr(),
        prop_type: PropType::Color,
        node_name: b"Specular\0".as_ptr(),
        node_fmt: b"RRR\0".as_ptr(),
    },
];

// ufbx.c:15986-16050 `ufbxi_read_legacy_prop`
// C returns `int` 0/1 without ever touching `uc->error` (callers do
// `if (!ufbxi_read_legacy_prop(...)) continue;`), so this is a predicate, not
// a `Result` — same shape as `ufbxi_get_val_at`.
#[inline(never)]
#[must_use]
pub(crate) fn read_legacy_prop(node: &NodeView, prop: &PropView, legacy_prop: &LegacyProp) -> bool {
    let mut value_ix: usize = 0;
    let mut flags: u32 = 0;

    for (fmt_ix, &c) in legacy_prop.format().iter().enumerate() {
        match c {
            b'L' => {
                ufbx_assert!(value_ix == 0);
                if let Some(got) = get_val_at::<i64>(node, fmt_ix) {
                    prop.set_value_int(got);
                } else {
                    return false;
                }
                prop.set_value_vec4(Vec4 {
                    x: prop.value_int() as Real,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                });
                prop.set_value_str(EMPTY_STRING.0);
                prop.set_value_blob(EMPTY_BLOB.0);
                flags |= PropFlags::VALUE_INT.raw();
                value_ix += 1;
            }
            b'R' => {
                ufbx_assert!(value_ix < 4);
                if let Some(got) = get_val_at::<AsReal>(node, fmt_ix) {
                    // The first component initializes the whole collapsed
                    // union arm; later components preserve the values written
                    // by preceding format characters.
                    let mut value = if value_ix == 0 {
                        Vec4 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                            w: 0.0,
                        }
                    } else {
                        prop.value_vec4()
                    };
                    match value_ix {
                        0 => value.x = got.0,
                        1 => value.y = got.0,
                        2 => value.z = got.0,
                        3 => value.w = got.0,
                        _ => ufbxi_unreachable!("Invalid legacy property value index"),
                    }
                    if value_ix == 0 {
                        // C: `ufbxi_f64_to_i64(prop->value_real)` — `ufbx_real`
                        // argument promoted to the `double` parameter.
                        prop.set_value_int(f64_to_i64(as_f64!(value.x)));
                        prop.set_value_str(EMPTY_STRING.0);
                        prop.set_value_blob(EMPTY_BLOB.0);
                    }
                    prop.set_value_vec4(value);
                } else {
                    return false;
                }
                flags &= !(PropFlags::VALUE_REAL.raw()
                    | PropFlags::VALUE_VEC2.raw()
                    | PropFlags::VALUE_VEC3.raw()
                    | PropFlags::VALUE_VEC4.raw());
                flags |= PropFlags::VALUE_REAL.raw() << value_ix;
                value_ix += 1;
            }
            b'S' => {
                ufbx_assert!(value_ix == 0);
                if let Some(got) = get_val_at::<Checked<String>>(node, fmt_ix) {
                    prop.set_value_str(got.0);
                } else {
                    return false;
                }
                if prop.value_str().length > 0 {
                    let found: Option<Blob> = get_val_at::<Blob>(node, fmt_ix);
                    if let Some(blob) = found {
                        prop.set_value_blob(blob);
                    }
                    ufbx_assert!(found.is_some());
                } else {
                    prop.set_value_blob(EMPTY_BLOB.0);
                }
                prop.set_value_vec4(Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                });
                prop.set_value_int(0);
                flags |= PropFlags::VALUE_STR.raw();
                value_ix += 1;
            }
            b'_' => {}
            _ => {
                ufbxi_unreachable!("Unhandled legacy fmt");
            }
        }
    }

    prop.set_flags(PropFlags::from_raw(flags));

    true
}

// ufbx.c:16052-16072 `ufbxi_read_legacy_props`
#[inline(never)]
#[must_use]
pub(crate) fn read_legacy_props(
    node: &NodeView,
    props: &mut [Prop],
    legacy_props: &[LegacyProp],
) -> usize {
    assert!(props.len() >= legacy_props.len());
    let mut num_props: usize = 0;
    for legacy_prop in legacy_props {
        let n: &NodeView = match find_child_strcmp(node, legacy_prop.node_name_bytes()) {
            Some(n) => n,
            None => continue,
        };
        let prop = PropView::from_mut(&mut props[num_props]);
        if !read_legacy_prop(n, prop, legacy_prop) {
            continue;
        }

        // SAFETY: `LegacyProp`'s invariant makes `prop_name` NUL-terminated and
        // live for the resulting scene property.
        let name = String::new_c(legacy_prop.prop_name, unsafe {
            strlen(legacy_prop.prop_name)
        });
        prop.set_name(name);
        prop.set_internal_key(get_name_key(prop.name_view().bytes()));
        prop.set_flags(PropFlags::from_raw(0));
        prop.set_type(legacy_prop.prop_type);
        num_props += 1;
    }

    num_props
}

// ufbx.c:16074-16090 `ufbxi_read_legacy_material`
///
/// # Safety
/// `name` stays a raw pointer for the reason `push_synthetic_element`
/// documents: it is null or NUL-terminated, the pointer ITSELF is stored in
/// the pushed material's `element.name.data`, and its bytes must stay live and
/// unmoved for as long as the scene — an obligation no borrow in this port
/// expresses.
#[inline(never)]
pub(crate) unsafe fn read_legacy_material(
    uc: &Context,
    node: &NodeView,
    p_fbx_id: &ScalarView<u64>,
    name: *const u8,
) -> Result<(), Fail> {
    // SAFETY: `name` is null or NUL-terminated and its bytes stay live for the
    // scene (fn contract); `Material` is the element struct for
    // `ElementType::Material`.
    let material: *mut Material = unsafe {
        push_synthetic_element::<Material>(uc, p_fbx_id, Some(node), name, ElementType::Material)
    };
    ufbxi_check!(uc, !material.is_null(), "material");

    // C: `ufbx_prop tmp_props[ufbxi_arraycount(ufbxi_legacy_material_props)];`
    // — uninitialized in C; only the `num_props` prefix that
    // `ufbxi_read_legacy_props()` wrote is ever read (upstream carries no
    // `// ufbxi_uninit` marker here).
    // SAFETY: `ufbx_prop` is a C aggregate of pointer/length pairs, scalars and
    // enum tags, for which the all-zero bit pattern is a valid value.
    let mut tmp_props: [Prop; LEGACY_MATERIAL_PROPS_COUNT] = unsafe { core::mem::zeroed() };
    let num_props: usize = read_legacy_props(node, &mut tmp_props, &LEGACY_MATERIAL_PROPS);

    // SAFETY: `material` is the fresh non-null element pushed above.
    unsafe {
        (*material).shading_model_name = EMPTY_STRING.0;
        (*material).element.props.props.count = num_props;
        (*material).element.props.props.data =
            uc.result_view().push_copy_slice(&tmp_props[..num_props]);
        ufbxi_check!(
            uc,
            !(*material).element.props.props.data.is_null(),
            "material->props.props.data"
        );
    }

    // SAFETY: `material` is the fresh non-null element pushed above.
    unsafe {
        (*material).shader_prop_prefix = EMPTY_STRING.0;
    }

    Ok(())
}

// ufbx.c:16092-16121 `ufbxi_read_legacy_link`
///
/// # Safety
/// `name` stays a raw pointer for the reason `push_synthetic_element`
/// documents: it is null or NUL-terminated, the pointer ITSELF is stored in
/// the pushed skin cluster's `element.name.data`, and its bytes must stay live
/// and unmoved for as long as the scene — an obligation no borrow in this port
/// expresses.
#[inline(never)]
pub(crate) unsafe fn read_legacy_link(
    uc: &Context,
    node: &NodeView,
    p_fbx_id: &ScalarView<u64>,
    name: *const u8,
) -> Result<(), Fail> {
    // SAFETY: `name` is null or NUL-terminated and its bytes stay live for the
    // scene (fn contract); `SkinCluster` is the element struct for
    // `ElementType::SkinCluster`.
    let cluster: *mut SkinCluster = unsafe {
        push_synthetic_element::<SkinCluster>(
            uc,
            p_fbx_id,
            Some(node),
            name,
            ElementType::SkinCluster,
        )
    };
    ufbxi_check!(uc, !cluster.is_null(), "cluster");

    // TODO: Merge with ufbxi_read_skin_cluster(), at least partially?
    let indices: *mut ValueArray = find_array(node, sp::Indexes.as_ptr(), b'i');
    let weights: *mut ValueArray = find_array(node, sp::Weights.as_ptr(), b'r');

    if !indices.is_null() && !weights.is_null() {
        // SAFETY: `indices` and `weights` are non-null (checked) and `find_array`
        // returns the node's own array descriptors, live for as long as the parse
        // tree.
        ufbxi_check!(
            uc,
            unsafe { (*indices).size } == unsafe { (*weights).size },
            "indices->size == weights->size"
        );
        // SAFETY: `cluster` is the fresh non-null element pushed above, and the
        // two live array descriptors carry `size` `uint32_t` / `ufbx_real` values
        // respectively, with the two sizes equal (checked above).
        unsafe {
            (*cluster).num_weights = (*indices).size;
            (*cluster).vertices.data = (*indices).data as *const u32;
            (*cluster).weights.data = (*weights).data as *const Real;
            (*cluster).vertices.count = (*cluster).num_weights;
            (*cluster).weights.count = (*cluster).num_weights;
        }
    }

    let transform: *mut ValueArray = find_array(node, sp::Transform.as_ptr(), b'r');
    let transform_link: *mut ValueArray = find_array(node, sp::TransformLink.as_ptr(), b'r');
    if !transform.is_null() && !transform_link.is_null() {
        // SAFETY: `transform` is non-null (checked) and `find_array` returns the
        // node's own array descriptor, live for as long as the parse tree.
        ufbxi_check!(
            uc,
            unsafe { (*transform).size } >= 16,
            "transform->size >= 16"
        );
        // SAFETY: as above, for `transform_link`.
        ufbxi_check!(
            uc,
            unsafe { (*transform_link).size } >= 16,
            "transform_link->size >= 16"
        );

        // SAFETY: `cluster` is the fresh non-null element pushed above, so the
        // field projections view its live `ufbx_matrix` fields with the
        // element's own write-capable provenance, and `transform`'s `'r'`
        // payload holds at least 16 reals (checked), the matrix element count
        // `read_transform_matrix` reads.
        unsafe {
            read_transform_matrix(
                View::<Matrix>::from_ptr(&raw mut (*cluster).mesh_node_to_bone),
                &*((*transform).data as *const [Real; 16]),
            );
            read_transform_matrix(
                View::<Matrix>::from_ptr(&raw mut (*cluster).bind_to_world),
                &*((*transform_link).data as *const [Real; 16]),
            );
        }
    }

    Ok(())
}

// ufbx.c:16123-16136 `ufbxi_read_legacy_light`
#[inline(never)]
pub(crate) fn read_legacy_light(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Light` is the element struct for `ElementType::Light`.
    let light: *mut Light = unsafe { push_element::<Light>(uc, info, ElementType::Light) };
    ufbxi_check!(uc, !light.is_null(), "light");

    // C: `ufbx_prop tmp_props[ufbxi_arraycount(ufbxi_legacy_light_props)];`
    // SAFETY: `ufbx_prop` is a C aggregate of pointer/length pairs, scalars and
    // enum tags, for which the all-zero bit pattern is a valid value.
    let mut tmp_props: [Prop; LEGACY_LIGHT_PROPS_COUNT] = unsafe { core::mem::zeroed() };
    let num_props: usize = read_legacy_props(node, &mut tmp_props, &LEGACY_LIGHT_PROPS);

    // SAFETY: `light` is the fresh non-null element pushed above.
    unsafe {
        (*light).element.props.props.count = num_props;
        (*light).element.props.props.data =
            uc.result_view().push_copy_slice(&tmp_props[..num_props]);
        ufbxi_check!(
            uc,
            !(*light).element.props.props.data.is_null(),
            "light->props.props.data"
        );
    }

    Ok(())
}

// ufbx.c:16138-16151 `ufbxi_read_legacy_camera`
#[inline(never)]
pub(crate) fn read_legacy_camera(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Camera` is the element struct for `ElementType::Camera`.
    let camera: *mut Camera = unsafe { push_element::<Camera>(uc, info, ElementType::Camera) };
    ufbxi_check!(uc, !camera.is_null(), "camera");

    // C: `ufbx_prop tmp_props[ufbxi_arraycount(ufbxi_legacy_camera_props)];`
    // SAFETY: `ufbx_prop` is a C aggregate of pointer/length pairs, scalars and
    // enum tags, for which the all-zero bit pattern is a valid value.
    let mut tmp_props: [Prop; LEGACY_CAMERA_PROPS_COUNT] = unsafe { core::mem::zeroed() };
    let num_props: usize = read_legacy_props(node, &mut tmp_props, &LEGACY_CAMERA_PROPS);

    // SAFETY: `camera` is the fresh non-null element pushed above.
    unsafe {
        (*camera).element.props.props.count = num_props;
        (*camera).element.props.props.data =
            uc.result_view().push_copy_slice(&tmp_props[..num_props]);
        ufbxi_check!(
            uc,
            !(*camera).element.props.props.data.is_null(),
            "camera->props.props.data"
        );
    }

    Ok(())
}

// ufbx.c:16153-16171 `ufbxi_read_legacy_limb_node`
#[inline(never)]
pub(crate) fn read_legacy_limb_node(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Bone` is the element struct for `ElementType::Bone`.
    let bone: *mut Bone = unsafe { push_element::<Bone>(uc, info, ElementType::Bone) };
    ufbxi_check!(uc, !bone.is_null(), "bone");

    // C: `ufbx_prop tmp_props[ufbxi_arraycount(ufbxi_legacy_bone_props)];`
    // SAFETY: `ufbx_prop` is a C aggregate of pointer/length pairs, scalars and
    // enum tags, for which the all-zero bit pattern is a valid value.
    let mut tmp_props: [Prop; LEGACY_BONE_PROPS_COUNT] = unsafe { core::mem::zeroed() };
    let mut num_props: usize = 0;

    let prop_node = find_child_strcmp(node, b"Properties");
    if let Some(prop_node) = prop_node {
        num_props = read_legacy_props(prop_node, &mut tmp_props, &LEGACY_BONE_PROPS);
    }

    // SAFETY: `bone` is the fresh non-null element pushed above.
    unsafe {
        (*bone).element.props.props.count = num_props;
        (*bone).element.props.props.data =
            uc.result_view().push_copy_slice(&tmp_props[..num_props]);
        ufbxi_check!(
            uc,
            !(*bone).element.props.props.data.is_null(),
            "bone->props.props.data"
        );
    }

    Ok(())
}

// ufbx.c:16173-16331 `ufbxi_read_legacy_mesh`
#[inline(never)]
pub(crate) fn read_legacy_mesh(
    uc: &Context,
    node: &NodeView,
    info: &View<ElementInfo, Mut>,
) -> Result<(), Fail> {
    // Only read polygon meshes, ignore eg. NURBS without error
    let node_vertices = find_child(node, sp::Vertices.as_ptr());
    let node_indices = find_child(node, sp::PolygonVertexIndex.as_ptr());
    if node_vertices.is_none() || node_indices.is_none() {
        return Ok(());
    }
    let node_vertices: &NodeView = node_vertices.unwrap();
    let node_indices: &NodeView = node_indices.unwrap();

    // SAFETY: `info` views the caller's live `ufbxi_element_info`, whose `name`
    // is a pooled NUL-terminated string and whose `props`/`dom_node` point into
    // uc's own buffers, so all three survive being stored into the element by
    // pointer; `Mesh` is the element struct for `ElementType::Mesh`.
    let mesh: *mut Mesh = unsafe { push_element::<Mesh>(uc, info, ElementType::Mesh) };
    ufbxi_check!(uc, !mesh.is_null(), "mesh");
    // SAFETY: `mesh` is the fresh non-null element just pushed into uc's
    // `tmp_elements` arena (elements live there until finalize copies them into
    // the result arena) — reached through `*mut` (write-capable provenance for
    // `Mut`) and live for the borrow; the fields accessed below are initialized
    // at each use site, as this function fills them in.
    let mesh = unsafe { View::<Mesh>::from_ptr(mesh) };

    read_synthetic_blend_shapes(uc, node, info)?;

    patch_mesh_reals(mesh);

    if uc.opts_view().ignore_geometry() {
        return Ok(());
    }

    let vertices: *mut ValueArray = get_array(node_vertices, b'r');
    let indices: *mut ValueArray = get_array(node_indices, b'i');
    ufbxi_check!(
        uc,
        !vertices.is_null() && !indices.is_null(),
        "vertices && indices"
    );
    // SAFETY: `vertices` and `indices` are non-null (checked above) and
    // `get_array` returns the node's own array descriptor, live for as long as
    // the parse tree and reached through `*mut` (write-capable provenance for
    // `Mut`).
    let vertices: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(vertices) };
    // SAFETY: as above, for the `'i'` descriptor.
    let indices: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(indices) };
    ufbxi_check!(uc, vertices.size() % 3 == 0, "vertices->size % 3 == 0");

    // `vertices`'s `'r'` payload holds `size` reals, a multiple of 3 (checked),
    // hence `size / 3` `ufbx_vec3` positions.
    mesh.set_num_vertices(vertices.size() / 3);
    mesh.set_num_indices(indices.size());

    // The `'i'` array's payload is a run of `size` `u32`s.
    let mut index_data: *mut u32 = indices.data() as *mut u32;

    // Duplicate `index_data` for modification if we retain DOM
    if uc.opts_view().retain_dom() {
        // SAFETY: `uc.result_mut_ptr()` is uc's own live `result` buf and
        // `index_data` spans the `indices->size` `uint32_t` values copied.
        index_data = unsafe {
            uc.result_view()
                .push_copy_raw::<u32>(indices.size(), index_data)
        };
        ufbxi_check!(uc, !index_data.is_null(), "index_data");
    }

    // `vertices`'s payload is the `num_vertices` `ufbx_vec3` run and
    // `index_data` the `num_indices` index run.
    mesh.vertices_view()
        .set_data(vertices.data() as *const Vec3);
    mesh.vertex_indices_view().set_data(index_data);
    mesh.vertices_view().set_count(mesh.num_vertices());
    mesh.vertex_indices_view().set_count(mesh.num_indices());

    mesh.vertex_position().set_exists(true);
    mesh.vertex_position()
        .values_view()
        .set_data(vertices.data() as *const Vec3);
    mesh.vertex_position()
        .values_view()
        .set_count(mesh.num_vertices());
    mesh.vertex_position().indices_view().set_data(index_data);
    mesh.vertex_position()
        .indices_view()
        .set_count(mesh.num_indices());
    mesh.vertex_position().set_unique_per_vertex(true);

    // Check/make sure that the last index is negated (last of polygon)
    let vertex_indices = mesh.vertex_indices_view();
    if mesh.num_indices() > 0 {
        let last = mesh.num_indices() - 1;
        if vertex_indices.copy_at(last) as i32 >= 0 {
            if uc.opts_view().strict() {
                ufbxi_fail!(uc, "Non-negated last index");
            }
            let value = vertex_indices.copy_at(last);
            vertex_indices.at(last).write_value(!value);
        }
    }

    process_indices(uc, mesh)?;

    // Normals are either per-vertex or per-index in legacy FBX files?
    // If the version is 5000 prefer per-vertex, otherwise per-index...
    let normals: *mut ValueArray = find_array(node, sp::Normals.as_ptr(), b'r');
    if !normals.is_null() {
        // SAFETY: `normals` is non-null (checked) and `find_array` returns the
        // node's own array descriptor, live for as long as the parse tree and
        // reached through `*mut` (write-capable provenance for `Mut`).
        let normals: &View<ValueArray> = unsafe { View::<ValueArray>::from_ptr(normals) };
        let num_normals: usize = normals.size() / 3;
        let per_vertex: bool = num_normals == mesh.num_vertices();
        let per_index: bool = num_normals == mesh.num_indices();
        if per_vertex && (!per_index || uc.version() == 5000) {
            // `normals`'s `'r'` payload holds `num_normals` `ufbx_vec3` normals,
            // one per vertex in this branch, and `vertex_indices.data` is the
            // index run installed above.
            mesh.vertex_normal().set_exists(true);
            mesh.vertex_normal().values_view().set_count(num_normals);
            mesh.vertex_normal()
                .indices_view()
                .set_count(mesh.num_indices());
            mesh.vertex_normal().set_unique_per_vertex(true);
            mesh.vertex_normal()
                .values_view()
                .set_data(normals.data() as *const Vec3);
            mesh.vertex_normal()
                .indices_view()
                .set_data(mesh.vertex_indices().data);
        } else if per_index {
            uc.set_max_consecutive_indices(max_sz(
                uc.max_consecutive_indices(),
                mesh.num_indices(),
            ));
            // `normals`'s `'r'` payload holds `num_normals` `ufbx_vec3` normals,
            // one per index in this branch, addressed through the
            // consecutive-index sentinel.
            mesh.vertex_normal().set_exists(true);
            mesh.vertex_normal().values_view().set_count(num_normals);
            mesh.vertex_normal()
                .indices_view()
                .set_count(mesh.num_indices());
            mesh.vertex_normal().set_unique_per_vertex(false);
            mesh.vertex_normal()
                .values_view()
                .set_data(normals.data() as *const Vec3);
            mesh.vertex_normal()
                .indices_view()
                .set_data(SENTINEL_INDEX_CONSECUTIVE.as_ptr());
        }
    }

    // Optional UV values are stored pretty much like a modern vertex element
    let uv_info = find_child(node, sp::GeometryUVInfo.as_ptr());
    if let Some(uv_info) = uv_info {
        let set: *mut UvSet = uc.result_view().push_zero::<UvSet>(1);
        ufbxi_check!(uc, !set.is_null(), "set");
        // SAFETY: `set` is the fresh zeroed `ufbx_uv_set` pushed into uc's own
        // `result` buf above, checked non-null — live and unmoved for the rest
        // of the load, and reached through `*mut` (write-capable provenance for
        // `Mut`).
        let set: &View<UvSet> = unsafe { View::<UvSet>::from_ptr(set) };
        set.set_index(0);
        set.name_view().set(EMPTY_STRING.0);
        // SAFETY: `set.vertex_uv_raw()` is the `ufbx_vertex_vec2` slot of the
        // fresh non-null set, result-arena memory reached through `*mut`
        // (write-capable provenance for `Mut`); the static asserts pin
        // `ufbx_vertex_vec2` to `ufbx_vertex_attrib`'s layout.
        let attrib: &View<VertexAttrib> =
            unsafe { View::<VertexAttrib>::from_ptr(set.vertex_uv_raw() as *mut VertexAttrib) };
        // SAFETY: `read_vertex_element` is an `unsafe fn`; the interned name
        // runs are NUL-terminated (its contract) and the `'r'`/2 attribute
        // shape matches the slot.
        unsafe {
            read_vertex_element(
                uc,
                mesh,
                uv_info,
                attrib,
                &sp::TextureUV,
                &sp::TextureUVVerticeIndex,
                None,
                b'r',
                2,
            )
        }?;

        // `set` is the fresh single-element UV-set allocation pushed above.
        mesh.uv_sets_view().set_data(set.get());
        mesh.uv_sets_view().set_count(1);
        // C: `mesh->vertex_uv = set->vertex_uv;` — struct assignment is a
        // memcpy; `VertexVec2` is not `Copy` in `generated.rs`.
        // SAFETY: `set.vertex_uv_ptr()` names the live set's own `vertex_uv`
        // slot, distinct from the mesh; it is initialized — zeroed by the
        // `push_zero` above, then possibly overwritten by `read_vertex_element`
        // (which returns Ok without touching it when the UV data array is
        // missing or empty).
        mesh.set_vertex_uv(unsafe { core::ptr::read(set.vertex_uv_ptr()) });
    }

    // Material indices
    {
        // C: `const char *mapping = NULL;`
        let mapping: *const u8 = ufbxi_check_some!(
            uc,
            find_val1::<Checked<*const u8>>(node, sp::MaterialAssignation.as_ptr()),
            "ufbxi_find_val1(node, ufbxi_MaterialAssignation, \"C\", (char**)&mapping)"
        )
        .0;
        if mapping == sp::ByPolygon.as_ptr() {
            // SAFETY: `b'i'` has the size, alignment and value validity of the
            // explicit `T = u32`; its payload stays live with the loader, and
            // the destination is the mesh's own list field.
            unsafe {
                read_truncated_array::<u32>(
                    uc,
                    mesh.face_material_view(),
                    node,
                    sp::Materials.as_ptr(),
                    b'i',
                    mesh.num_faces(),
                )
            }?;
        } else if mapping == sp::AllSame.as_ptr() {
            let arr: *mut ValueArray = find_array(node, sp::Materials.as_ptr(), b'i');
            // SAFETY: the view is minted only in the non-null arm, where
            // `find_array` returned the node's own array descriptor, live for
            // as long as the parse tree and reached through `*mut`
            // (write-capable provenance for `Mut`).
            let arr: Option<&View<ValueArray>> = if arr.is_null() {
                None
            } else {
                Some(unsafe { View::<ValueArray>::from_ptr(arr) })
            };
            let mut material: u32 = 0;
            if arr.is_some_and(|arr| arr.size() >= 1) {
                // The guard above admits only a live descriptor with `size >= 1`,
                // whose `'i'` payload is a run of `size` `u32`s.
                let arr: &View<ValueArray> = arr.unwrap();
                // SAFETY: `arr.data()` names that `size >= 1` run of `u32`, so
                // its first element is readable.
                material = unsafe { *(arr.data() as *mut u32) };
            }

            mesh.face_material_view().set_count(mesh.num_faces());
            if material == 0 {
                mesh.face_material_view()
                    .set_data(SENTINEL_INDEX_ZERO.as_ptr());
            } else {
                mesh.face_material_view()
                    .set_data(uc.result_view().push::<u32>(mesh.num_faces()));
                ufbxi_check!(
                    uc,
                    !mesh.face_material().data.is_null(),
                    "mesh->face_material.data"
                );
                // C: `ufbxi_for_list(uint32_t, p_mat, mesh->face_material)`
                for p_mat in Run::from_list(mesh.face_material_view()).iter() {
                    p_mat.write_value(material);
                }
            }
        }
    }

    let mut skin_fbx_id: u64 = 0;
    let mut skin: *mut SkinDeformer = core::ptr::null_mut();

    // Materials, Skin Clusters
    // C: `ufbxi_for(ufbxi_node, child, node->children, node->num_children)`
    for child in node.children_iter() {
        if child.name() == sp::Material.as_ptr() {
            let mut fbx_id: u64 = 0;
            // C: `ufbx_string type_and_name, type, name;` — `type`/`name` are
            // written by `split_type_and_name` below.
            // SAFETY: `ufbx_string` is a plain pointer/length pair, for which
            // the all-zero bit pattern is a valid (empty, null-data) value.
            let (mut type_, mut name): (String, String) =
                unsafe { (core::mem::zeroed(), core::mem::zeroed()) };
            let Unchecked(type_and_name) = ufbxi_check_some!(
                uc,
                get_val1::<Unchecked<String>>(child),
                "ufbxi_get_val1(child, \"s\", &type_and_name)"
            );
            split_type_and_name(uc, View::from_ref(&type_and_name), &mut type_, &mut name)?;
            // SAFETY: `name.data` is the NUL-terminated interned name the split
            // produced, live for as long as the scene.
            unsafe {
                read_legacy_material(uc, child, ScalarView::from_mut(&mut fbx_id), name.data)
            }?;
            connect_oo(uc, fbx_id, info.fbx_id())?;
        } else if child.name() == sp::Link.as_ptr() {
            let mut fbx_id: u64 = 0;
            // C: `ufbx_string type_and_name, type, name;` — `type`/`name` are
            // written by `split_type_and_name` below.
            // SAFETY: `ufbx_string` is a plain pointer/length pair, for which
            // the all-zero bit pattern is a valid (empty, null-data) value.
            let (mut type_, mut name): (String, String) =
                unsafe { (core::mem::zeroed(), core::mem::zeroed()) };
            let Unchecked(type_and_name) = ufbxi_check_some!(
                uc,
                get_val1::<Unchecked<String>>(child),
                "ufbxi_get_val1(child, \"s\", &type_and_name)"
            );
            split_type_and_name(uc, View::from_ref(&type_and_name), &mut type_, &mut name)?;
            // SAFETY: `name.data` is the NUL-terminated interned name the split
            // produced, live for as long as the scene.
            unsafe { read_legacy_link(uc, child, ScalarView::from_mut(&mut fbx_id), name.data) }?;

            let node_fbx_id: u64 = synthetic_id_from_string(uc, type_and_name.data);
            ufbxi_check!(uc, node_fbx_id != 0, "node_fbx_id");
            connect_oo(uc, node_fbx_id, fbx_id)?;
            if skin.is_null() {
                // SAFETY: `&raw mut skin_fbx_id` is a live local `uint64_t` slot,
                // `info`'s `name.data` is a NUL-terminated interned name, and
                // `SkinDeformer` is the element struct for
                // `ElementType::SkinDeformer`.
                skin = unsafe {
                    push_synthetic_element::<SkinDeformer>(
                        uc,
                        ScalarView::from_mut(&mut skin_fbx_id),
                        None,
                        info.name_view().data(),
                        ElementType::SkinDeformer,
                    )
                };
                ufbxi_check!(uc, !skin.is_null(), "skin");
                connect_oo(uc, skin_fbx_id, info.fbx_id())?;
            }
            connect_oo(uc, fbx_id, skin_fbx_id)?;
        }
    }

    mesh.set_skinned_is_local(true);
    // SAFETY: `vertex_position` was fully written above; source and destination
    // are distinct fields of the mesh.
    mesh.set_skinned_position(unsafe { core::ptr::read(mesh.vertex_position_ptr()) });
    // SAFETY: as above — `vertex_normal` is either the run installed above or the
    // zeroed field of the freshly pushed element.
    mesh.set_skinned_normal(unsafe { core::ptr::read(mesh.vertex_normal_ptr()) });

    patch_mesh_reals(mesh);

    Ok(())
}

// ufbx.c:16333-16348 `ufbxi_read_legacy_media`
#[inline(never)]
pub(crate) fn read_legacy_media(uc: &Context, node: &NodeView) -> Result<(), Fail> {
    let videos = find_child(node, sp::Video.as_ptr());
    if let Some(videos) = videos {
        // C: `ufbxi_for(ufbxi_node, child, videos->children, videos->num_children)`
        for child in videos.children_iter() {
            // SAFETY: all-zero is a valid `ufbxi_element_info`; `child` is a
            // NodeView from `videos`'s own child run, `video_info` is an
            // unaliased local — write-capable provenance, live and unmoved
            // across the view mint and the call — whose `name` is exactly the
            // `ufbx_string` the `S` format writes, and `node` is the parse-tree
            // NodeView the DOM lookup expects.
            unsafe {
                let mut video_info: ElementInfo = core::mem::zeroed();
                video_info.name = ufbxi_check_some!(
                    uc,
                    get_val1::<Checked<String>>(child),
                    "ufbxi_get_val1(child, \"S\", &video_info.name)"
                )
                .0;
                video_info.fbx_id = push_synthetic_id(uc);
                video_info.dom_node = get_dom_node(uc, Some(node));

                read_video(
                    uc,
                    child,
                    View::<ElementInfo, Mut>::from_mut(&mut video_info),
                )?;
            }
        }
    }

    Ok(())
}

// ufbx.c:16350-16421 `ufbxi_read_legacy_model`
#[inline(never)]
pub(crate) fn read_legacy_model(uc: &Context, node: &NodeView) -> Result<(), Fail> {
    // C: `ufbx_string type_and_name, type, name;` — all three are written
    // before use by the two calls below.
    // SAFETY: all-zero is a valid `ufbx_string`; `node` is a parse-tree
    // NodeView and `type_and_name` is an unaliased local of exactly the type
    // the `s` format writes, so on success it is a pooled `data`/`length` pair
    // — which is what the split and the synthetic-id hash below read.
    let mut type_: String = unsafe { core::mem::zeroed() };
    let mut name: String = unsafe { core::mem::zeroed() };
    let type_and_name: String = ufbxi_check_some!(
        uc,
        get_val1::<Unchecked<String>>(node),
        "ufbxi_get_val1(node, \"s\", &type_and_name)"
    )
    .0;
    split_type_and_name(uc, View::from_ref(&type_and_name), &mut type_, &mut name)?;

    // SAFETY: all-zero is a valid `ufbxi_element_info`; `info` is an unaliased
    // local, `type_and_name.data` is the pooled string read above, `node` is a
    // parse-tree NodeView. `info`'s `name` is the pooled NUL-terminated name and
    // its `dom_node` points into uc's own DOM buffer, so the pointers the element
    // stores outlive the scene, and `UfbxNode` is the element struct for
    // `ElementType::Node`; `elem_node` is the fresh element the push
    // returns — checked non-null before its `element_id` is copied into uc's
    // own `tmp_node_ids` buffer.
    let mut info: ElementInfo = unsafe { core::mem::zeroed() };
    unsafe {
        info.fbx_id = synthetic_id_from_string(uc, type_and_name.data);
        ufbxi_check!(uc, info.fbx_id != 0, "info.fbx_id");
        info.name = name;
        info.dom_node = get_dom_node(uc, Some(node));

        let elem_node: *mut UfbxNode = push_element::<UfbxNode>(
            uc,
            View::<ElementInfo>::from_mut(&mut info),
            ElementType::Node,
        );
        ufbxi_check!(uc, !elem_node.is_null(), "elem_node");
        ufbxi_check!(
            uc,
            !uc.tmp_node_ids_view()
                .push_copy_ref(&(*elem_node).element.element_id)
                .is_null(),
            "((uint32_t*)ufbxi_push_size_copy((&uc->tmp_node_ids), sizeof(uint32_t), (1), (&elem_node->element.element_id)))"
        );
    }

    // SAFETY: all-zero is a valid `ufbxi_element_info`; `name` is the pooled
    // string split out above.
    let mut attrib_info: ElementInfo = unsafe { core::mem::zeroed() };
    attrib_info.fbx_id = push_synthetic_id(uc);
    attrib_info.name = name;
    attrib_info.dom_node = info.dom_node;

    // If we make unused connections it doesn't matter..
    connect_oo(uc, attrib_info.fbx_id, info.fbx_id)?;

    let mut attrib_type: *const u8 = EMPTY_CHAR.as_ptr();
    if let Some(got) = find_val1::<Checked<*const u8>>(node, sp::Type.as_ptr()) {
        attrib_type = got.0;
    }

    // Each arm hands the same parse-tree NodeView and a handle on the local
    // `attrib_info` to the legacy attribute reader selected by pointer-identity
    // comparison of the pooled `attrib_type`.
    let mut has_attrib: bool = true;
    if attrib_type == sp::Light.as_ptr() {
        read_legacy_light(
            uc,
            node,
            View::<ElementInfo, Mut>::from_mut(&mut attrib_info),
        )?;
    } else if attrib_type == sp::Camera.as_ptr() {
        read_legacy_camera(
            uc,
            node,
            View::<ElementInfo, Mut>::from_mut(&mut attrib_info),
        )?;
    } else if attrib_type == sp::LimbNode.as_ptr() {
        read_legacy_limb_node(
            uc,
            node,
            View::<ElementInfo, Mut>::from_mut(&mut attrib_info),
        )?;
    } else if find_child(node, sp::Vertices.as_ptr()).is_some() {
        read_legacy_mesh(
            uc,
            node,
            View::<ElementInfo, Mut>::from_mut(&mut attrib_info),
        )?;
    } else {
        has_attrib = false;
    }

    // Mark the node as having an attribute so property connections can be forwarded
    if has_attrib {
        insert_fbx_attr(uc, info.fbx_id, attrib_info.fbx_id)?;
    }

    // Children are represented as an array of strings
    let children: *mut ValueArray = find_array(node, sp::Children.as_ptr(), b's');
    if !children.is_null() {
        // SAFETY: `children` is the non-null `'s'`-typed value array the search
        // returned, so its `data` is a run of `size` pooled `ufbx_string`s;
        // `i < size` keeps the indexing in bounds.
        unsafe {
            let names: *mut String = (*children).data as *mut String;
            let mut i: usize = 0;
            while i < (*children).size {
                let child_fbx_id: u64 = synthetic_id_from_string(uc, (*names.add(i)).data);
                ufbxi_check!(uc, child_fbx_id != 0, "child_fbx_id");
                connect_oo(uc, child_fbx_id, info.fbx_id)?;
                i += 1;
            }
        }
    }

    // Non-take animation channels
    // C: `ufbxi_for(ufbxi_node, child, node->children, node->num_children)`
    for child in node.children_iter() {
        if child.name() == sp::Channel.as_ptr() {
            // C: `ufbx_string channel_name;` — written by the guard below.
            // SAFETY: all-zero is a valid `ufbx_string`; `child` is a NodeView
            // from `node`'s own child run and `channel_name` is an unaliased
            // local of exactly the type the `S` format writes, so on success it
            // is pooled and safe to hand to the channel reader.
            unsafe {
                if let Some(Checked(channel_name)) = get_val1::<Checked<String>>(child) {
                    if uc.legacy_implicit_anim_layer_id() == 0 {
                        // Defer creation so we won't be the first animation stack..
                        uc.set_legacy_implicit_anim_layer_id(push_synthetic_id(uc));
                    }
                    read_take_prop_channel(
                        uc,
                        child,
                        info.fbx_id,
                        uc.legacy_implicit_anim_layer_id(),
                        channel_name,
                    )?;
                }
            }
        }
    }

    Ok(())
}

// ufbx.c:16424-16483 `ufbxi_read_legacy_root`
#[inline(never)]
pub(crate) fn read_legacy_root(uc: &Context) -> Result<(), Fail> {
    init_node_prop_names(uc)?;

    // Some legacy FBX files have an `Fbx_Root` node that could be used as the
    // root node. However no other formats have root node with transforms so it
    // might be better to leave it as-is and create an empty one.
    {
        // SAFETY: the id out-param is uc's own `root_id` field, viewed for the
        // call as an interior-mutable scalar cell — `ScalarView<u64>` is
        // `repr(transparent)` over `u64` — the name is the
        // static empty string, and `root` is the fresh element the push returns
        // — checked non-null before it is set up and before its `element_id` is
        // copied into uc's own `tmp_node_ids` buffer.
        unsafe {
            let root: *mut UfbxNode = push_synthetic_element::<UfbxNode>(
                uc,
                &*(uc.root_id_mut_ptr() as *const ScalarView<u64>),
                None,
                EMPTY_CHAR.as_ptr(),
                ElementType::Node,
            );
            ufbxi_check!(uc, !root.is_null(), "root");
            // SAFETY: `root` is the fresh non-null element the push returns,
            // living in uc's own write-capable element arena.
            setup_root_node(uc, View::<UfbxNode>::from_ptr(root));
            ufbxi_check!(
            uc,
            !uc.tmp_node_ids_view()
                .push_copy_ref(&(*root).element.element_id)
                .is_null(),
            // C-parity: verbatim post-expansion `#cond` text (see the C11
            // 6.10.3.1 note in `sort_shader_prop_bindings`).
            "((uint32_t*)ufbxi_push_size_copy((&uc->tmp_node_ids), sizeof(uint32_t), (1), (&root->element.element_id)))"
        );
        }
    }

    // NOTE: `ufbxi_read_header_extension()` is optional so use default KTime definition
    uc.set_ktime_sec(46186158000);
    uc.set_ktime_sec_double(uc.ktime_sec() as f64);

    loop {
        parse_legacy_toplevel(uc)?;
        let Some(node) = uc.top_node_view() else {
            break;
        };
        if node.name() == sp::FBXHeaderExtension.as_ptr() {
            read_header_extension(uc)?;
        } else if node.name() == sp::Media.as_ptr() {
            read_legacy_media(uc, node)?;
        } else if node.name() == sp::Takes.as_ptr() {
            read_takes(uc)?;
        } else if node.name() == sp::Model.as_ptr() {
            read_legacy_model(uc, node)?;
        // SAFETY: `node.name()` is a pooled NUL-terminated string and the
        // literal is NUL-terminated too, so the C compare stays in bounds.
        } else if unsafe { strcmp(node.name(), b"Settings\0".as_ptr()) } == 0 {
            read_legacy_settings(uc, node)?;
        }
    }

    if uc.opts_view().retain_dom() {
        // C: a NULL node retains uc's whole current top-level run.
        retain_toplevel(uc, None)?;
    }

    // Create the implicit animation stack if necessary
    if uc.legacy_implicit_anim_layer_id() != 0 {
        // C: `ufbxi_element_info layer_info = { 0 };`
        // SAFETY: all-zero is a valid `ufbxi_element_info`. The name is a
        // NUL-terminated literal, so `strlen` stays in bounds, and the intern
        // into uc's own pool makes it outlive the elements — the pointer the
        // element stores stays live and NUL-terminated for the scene, with
        // `props`/`dom_node` zeroed, and `AnimLayer`/`AnimStack` are the element
        // structs for their pushed types. Both pushes take
        // unaliased locals and their results are checked non-null; the
        // `stack_info` copy is a plain bitwise struct copy of `layer_info`,
        // which is fully initialized and holds no owning handles.
        unsafe {
            let mut layer_info: ElementInfo = core::mem::zeroed();
            layer_info.fbx_id = uc.legacy_implicit_anim_layer_id();
            layer_info.name.data = b"(internal)\0".as_ptr();
            layer_info.name.length = strlen(layer_info.name.data);
            push_string_place_str(
                uc.string_pool_view(),
                StringView::from_mut(&mut layer_info.name),
                true,
            )?;
            let layer: *mut AnimLayer = push_element::<AnimLayer>(
                uc,
                View::<ElementInfo>::from_mut(&mut layer_info),
                ElementType::AnimLayer,
            );
            ufbxi_check!(uc, !layer.is_null(), "layer");

            // C: `ufbxi_element_info stack_info = layer_info;` (struct copy)
            let mut stack_info: ElementInfo = core::ptr::read(&layer_info);
            stack_info.fbx_id = push_synthetic_id(uc);
            let stack: *mut AnimStack = push_element::<AnimStack>(
                uc,
                View::<ElementInfo>::from_mut(&mut stack_info),
                ElementType::AnimStack,
            );
            ufbxi_check!(uc, !stack.is_null(), "stack");

            connect_oo(uc, layer_info.fbx_id, stack_info.fbx_id)?;
        }
    }

    Ok(())
}

// Filename manipulation

// ufbx.c:16487-16498 `ufbxi_trim_delimiters`
#[inline(never)]
#[must_use]
pub(crate) fn trim_delimiters(uc: &Context, data: &[u8]) -> usize {
    let mut length = data.len();
    while length > 0 {
        // C-parity: `char c = data[length - 1];` — `c` is only compared
        // against ASCII separators, so the signedness of C `char` is not
        // observable here (PORTING.md "char (value…)").
        let c: u8 = data[length - 1];
        let is_separator: bool = c == b'/' || c == uc.opts_view().path_separator();
        if is_separator {
            length -= 1;
            break;
        }
        length -= 1;
    }
    length
}

// ufbx.c:16500-16529 `ufbxi_init_file_paths`
#[inline(never)]
pub(crate) fn init_file_paths(uc: &Context) -> Result<(), Fail> {
    let filename = uc.opts_view().filename_view();
    let raw_filename = uc.opts_view().raw_filename_view();

    if filename.length() > 0 {
        let value = String::new_c(filename.data(), filename.length());
        uc.scene_view().metadata_view().set_filename(value);
    } else if raw_filename.size() > 0 {
        let value = String::new_c(raw_filename.data(), raw_filename.size());
        uc.scene_view().metadata_view().set_filename(value);
    }

    if raw_filename.size() > 0 {
        // SAFETY: the raw load-option filename is a caller-owned byte run that
        // stays live for the load; this metadata copy is interned below before
        // the load can return.
        let raw_filename = unsafe { Blob::new_c(raw_filename.data(), raw_filename.size()) };
        uc.scene_view()
            .metadata_view()
            .set_raw_filename(raw_filename);
    } else if filename.length() > 0 {
        let filename = String::new_c(filename.data(), filename.length());
        uc.scene_view()
            .metadata_view()
            .set_raw_filename(Blob::from_string(filename));
    }

    push_string_place_str(
        uc.string_pool_view(),
        uc.scene_view().metadata_view().filename_view(),
        false,
    )?;
    push_string_place_blob(
        uc.string_pool_view(),
        uc.scene_view().metadata_view().raw_filename_view(),
        true,
    )?;

    let filename = uc.scene_view().metadata_view().filename_view();
    let relative_root = String::new_c(filename.data(), trim_delimiters(uc, filename.bytes()));
    uc.scene_view()
        .metadata_view()
        .relative_root_view()
        .set(relative_root);

    let raw_filename = uc.scene_view().metadata_view().raw_filename_view();
    // SAFETY: the metadata raw filename's blob pointer is readable for its
    // stored size. `trim_delimiters` returns a prefix length within that run,
    // which therefore forms a valid blob descriptor over the same storage.
    let raw_relative_root = unsafe {
        let bytes = slice_from_ptr(raw_filename.data(), raw_filename.size());
        Blob::new_c(raw_filename.data(), trim_delimiters(uc, bytes))
    };
    uc.scene_view()
        .metadata_view()
        .raw_relative_root_view()
        .set(raw_relative_root);

    push_string_place_str(
        uc.string_pool_view(),
        uc.scene_view().metadata_view().relative_root_view(),
        false,
    )?;
    push_string_place_blob(
        uc.string_pool_view(),
        uc.scene_view().metadata_view().raw_relative_root_view(),
        true,
    )?;

    Ok(())
}

// ufbx.c:16531-16534 `ufbxi_strblob` — untagged overlay discriminated by the
// sibling `raw` argument threaded through every accessor (PORTING.md "Unions").
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) union Strblob {
    pub str_: String,
    pub blob: Blob,
}

// Every accessor selects the same member from the shared `raw` flag. These
// assertions pin the layout identity that the C union relies on.
const _: () = assert!(size_of::<Strblob>() == size_of::<String>());
const _: () = assert!(size_of::<Strblob>() == size_of::<Blob>());

// ufbx.c:16536-16545 `ufbxi_strblob_set` string member
#[inline(never)]
pub(crate) fn strblob_set_string(dst: &View<Strblob>, value: String) {
    // C canonicalizes an empty string to `ufbxi_empty_char`, regardless of the
    // input pointer. The projected member covers the whole layout-identical
    // union descriptor.
    let value = if value.length == 0 {
        EMPTY_STRING.0
    } else {
        value
    };
    let str_: &StringView = view_project!(dst, str_);
    str_.set(value);
}

// ufbx.c:16536-16545 `ufbxi_strblob_set` blob member
#[inline(never)]
pub(crate) fn strblob_set_blob(dst: &View<Strblob>, value: Blob) {
    // The projected member covers the whole layout-identical union descriptor.
    let blob: &BlobView = view_project!(dst, blob);
    blob.set(value);
}

// ufbx.c:16547-16550 `ufbxi_strblob_data`
#[inline(always)]
pub(crate) fn strblob_data<M: Mode>(strblob: &View<Strblob, M>, raw: bool) -> *const u8 {
    // `raw` selects the same member used by the typed `strblob_set_*` calls.
    if raw {
        view_read_shared!(strblob, blob).data
    } else {
        view_read_shared!(strblob, str_).data
    }
}

// ufbx.c:16552-16555 `ufbxi_strblob_length`
#[inline(always)]
pub(crate) fn strblob_length<M: Mode>(strblob: &View<Strblob, M>, raw: bool) -> usize {
    // `raw` selects the same member used by the typed `strblob_set_*` calls.
    if raw {
        view_read_shared!(strblob, blob).size
    } else {
        view_read_shared!(strblob, str_).length
    }
}

// ufbx.c:16557-16565 `ufbxi_is_absolute_path`
#[inline(never)]
#[must_use]
pub(crate) fn is_absolute_path(path: &[u8]) -> bool {
    if !path.is_empty() && (path[0] == b'/' || path[0] == b'\\') {
        return true;
    } else if path.len() > 2 && path[1] == b':' && (path[2] == b'\\' || path[2] == b'/') {
        return true;
    }
    false
}

// ufbx.c:16567-16650 `ufbxi_resolve_relative_filename`
#[inline(never)]
pub(crate) fn resolve_relative_filename<M: Mode>(
    uc: &Context,
    p_dst: &View<Strblob>,
    p_src: &View<Strblob, M>,
    raw: bool,
) -> Result<(), Fail> {
    // C: `const char *src` / `size_t src_length` — the walking `src` cursor is
    // carried as `src_ix`, an offset into the source path run, so that
    // `src_ix + src_length == src_run.len()` holds at every step below.
    // SAFETY: `strblob_data`/`strblob_length` name the source path run
    // described by `p_src`: `src_length` initialized bytes, live and unmoved
    // for the borrow — nothing here writes the source bytes (the only writes
    // are into the fresh scratch run and `p_dst`'s own header).
    let src_run: Run<'_, u8, Const> = unsafe {
        Run::<u8, Const>::from_const_raw_parts(strblob_data(p_src, raw), strblob_length(p_src, raw))
    };
    let mut src_ix: usize = 0;
    let mut src_length: usize = src_run.len();

    // Skip leading directory separators and early return if the relative path is empty
    while src_length > 0 && (src_run.copy_at(src_ix) == b'/' || src_run.copy_at(src_ix) == b'\\') {
        src_ix += 1;
        src_length -= 1;
    }
    if src_length == 0 {
        if raw {
            strblob_set_blob(p_dst, Blob::empty());
        } else {
            strblob_set_string(p_dst, EMPTY_STRING.0);
        }
        return Ok(());
    }

    let prefix_data: *const u8;
    let mut prefix_length: usize;
    if raw {
        prefix_data = uc
            .scene_view()
            .metadata_view()
            .raw_relative_root_view()
            .data();
        prefix_length = uc
            .scene_view()
            .metadata_view()
            .raw_relative_root_view()
            .size();
    } else {
        prefix_data = uc.scene_view().metadata_view().relative_root_view().data();
        prefix_length = uc
            .scene_view()
            .metadata_view()
            .relative_root_view()
            .length();
    }

    // SAFETY: `prefix_data`/`prefix_length` name the scene metadata's own
    // relative-root run: `prefix_length` initialized bytes, live and unmoved
    // for the borrow — the run is scene-owned and nothing here writes it. The
    // local `prefix_length` only ever shrinks below the vouched length, so
    // every index taken from it below stays inside the run.
    let prefix_run: Run<'_, u8, Const> =
        unsafe { Run::<u8, Const>::from_const_raw_parts(prefix_data, prefix_length) };

    // Retain absolute paths. The temporary shared slice ends with this test;
    // later code may publish through `p_dst`, which can alias an input header.
    // SAFETY: the checked sub-run addresses exactly the remaining `src_length`
    // source-path bytes.
    if is_absolute_path(unsafe {
        slice_from_ptr(src_run.subrun(src_ix, src_length).as_ptr(), src_length)
    }) {
        prefix_length = 0;
    }

    // Undo directories from `prefix` for every `..`
    while prefix_length > 0
        && src_length >= 3
        && src_run.copy_at(src_ix) == b'.'
        && src_run.copy_at(src_ix + 1) == b'.'
        && (src_run.copy_at(src_ix + 2) == b'/' || src_run.copy_at(src_ix + 2) == b'\\')
    {
        let mut part_start: usize = prefix_length;
        while part_start > 0
            && !(prefix_run.copy_at(part_start - 1) == b'/'
                || prefix_run.copy_at(part_start - 1) == b'\\')
        {
            part_start -= 1;
        }
        let part_len: usize = prefix_length - part_start;

        if part_len == 2
            && prefix_run.copy_at(part_start) == b'.'
            && prefix_run.copy_at(part_start + 1) == b'.'
        {
            // Prefix itself ends in `..`, cannot cancel out a leading `../`
            break;
        }

        // Eat the leading '/' before the part segment
        prefix_length = if part_start > 0 { part_start - 1 } else { 0 };

        if part_len == 1 && prefix_run.copy_at(part_start) == b'.' {
            // Single '.' -> remove and continue without cancelling out a leading `../`
            continue;
        }

        src_ix += 3;
        src_length -= 3;
    }

    let result_cap: usize = prefix_length + src_length + 1;
    let result: *mut u8 = uc.tmp_stack_view().push::<u8>(result_cap);
    ufbxi_check!(uc, !result.is_null(), "result");
    // C: `char *ptr = result;` — the walking write cursor is carried as
    // `out_ix`, an offset into the scratch run, so `ptr == result + out_ix`.
    // SAFETY: `result` is the non-null `result_cap`-byte allocation just pushed
    // on uc's own tmp stack — one contiguous, write-capable run that stays
    // alive and unmoved until the pop below (nothing pushes or pops that buf in
    // between). Its bytes are still uninitialized, which the run tolerates.
    let out: Run<'_, u8, Mut> = unsafe { Run::<u8, Mut>::from_raw_parts(result, result_cap) };
    let mut out_ix: usize = 0;

    // Copy prefix and suffix converting separators in the process
    if prefix_length > 0 {
        // SAFETY: `prefix_run` vouches for at least `prefix_length` readable
        // bytes and `out` for `result_cap == prefix_length + src_length + 1`
        // writable ones starting at `out_ix == 0`; the scratch run is a
        // distinct allocation from the metadata relative root.
        unsafe {
            core::ptr::copy_nonoverlapping(prefix_run.as_ptr(), out.as_mut_ptr(), prefix_length);
        }
        out.write_at(prefix_length, uc.opts_view().path_separator());
        out_ix = prefix_length + 1;
    }
    let mut i: usize = 0;
    while i < src_length {
        let mut c: u8 = src_run.copy_at(src_ix + i);
        if c == b'/' || c == b'\\' {
            c = uc.opts_view().path_separator();
        }
        out.write_at(out_ix, c);
        out_ix += 1;
        i += 1;
    }

    // Intern the string and pop the temporary buffer
    // C: `ufbxi_to_size(ptr - result)` — `out_ix` is that same distance, the
    // count of bytes written into the scratch run above.
    let mut dst: String = String::new_c(result, out_ix);
    ufbx_assert!(dst.length <= result_cap);
    // `dst` is a live local naming the bytes written into the scratch run above.
    push_string_place_str(uc.string_pool_view(), StringView::from_mut(&mut dst), raw)?;
    // SAFETY: the temporary stack is uc's own and `result_cap` bytes were pushed
    // onto it above and are still its topmost allocation; a null destination
    // discards them.
    unsafe {
        pop::<u8>(uc.tmp_stack_view(), result_cap, core::ptr::null_mut());
    }

    // `dst` is the interned string, which outlives the popped scratch run. The
    // discriminator visibly selects the matching union member for publication.
    if raw {
        strblob_set_blob(p_dst, Blob::from_string(dst));
    } else {
        strblob_set_string(p_dst, dst);
    }

    Ok(())
}

// Open file utility

// ufbx.c:16654-16669 `ufbxi_open_file`
///
/// # Safety
/// `cb` must be null or point at a live `ufbx_open_file_cb`, `stream` at a live
/// `ufbx_stream` slot the callback may write, and `path` at a live run of
/// `path_len` readable bytes that the callback additionally reads as a
/// NUL-terminated C string — obligations the raw pointer and the separate
/// length cannot express.
#[inline(never)]
pub(crate) unsafe fn open_file<M: Mode>(
    cb: *const RawOpenFileCb,
    stream: *mut RawStream,
    path: *const u8,
    path_len: usize,
    original_filename: Option<&View<Blob, M>>,
    ator: Option<&AllocatorView>,
    type_: OpenFileType,
) -> bool {
    // SAFETY: the null check short-circuits first, so `cb` is the caller's live
    // `ufbx_open_file_cb` here.
    if cb.is_null() || unsafe { (*cb).fn_.is_none() } {
        return false;
    }

    let mut info = MaybeUninit::<OpenFileInfo>::uninit(); // ufbxi_uninit
    let info: *mut OpenFileInfo = info.as_mut_ptr();
    // SAFETY: `info` points at the live local `MaybeUninit` storage; the field
    // is written, not read, and `ufbx_open_file_info` is plain C data with no
    // value to drop, so writing into uninitialized storage is well-defined.
    unsafe {
        (*info).context = ator.map_or(0, |ator| ator.get() as OpenFileContext);
    }
    if let Some(original_filename) = original_filename {
        // C: `info.original_filename = *original_filename;` — `ufbx_blob` is
        // the `{ data, size }` pair the two field reads cover.
        // SAFETY: the destination is the local `info` storage as above.
        unsafe {
            (*info).original_filename.data = original_filename.data();
            (*info).original_filename.size = original_filename.size();
        }
    } else {
        // SAFETY: `info` points at the live local storage as above.
        unsafe {
            (*info).original_filename.data = path;
            (*info).original_filename.size = path_len;
        }
    }
    // SAFETY: `info` points at the live local storage as above.
    unsafe {
        (*info).type_ = type_;
    }

    // SAFETY: `cb` is live with `fn_` present (both checked above); `path` /
    // `path_len` and `stream` come from the caller, and every field of
    // `ufbx_open_file_info` — `context`, `original_filename`, `type` — is
    // written above, so `info` is fully initialized.
    unsafe { ((*cb).fn_.unwrap())((*cb).user, stream, path, path_len, info) }
}

// ufbx.c:16671-16674 `#define ufbxi_patch_zero(dst, src)`
macro_rules! ufbxi_patch_zero {
    ($dst:expr, $src:expr) => {{
        let src = $src;
        $crate::native::platform::ufbx_assert!($dst == 0 || $dst == src);
        $dst = src;
    }};
}

// ufbx.c:16676-16689 `ufbxi_update_vertex_first_index`
pub(crate) fn update_vertex_first_index(mesh: &View<Mesh>) {
    // C: `ufbxi_for_list(uint32_t, p_vx_ix, mesh->vertex_first_index)`
    let vertex_first_index = Run::from_list(mesh.vertex_first_index_view());
    let mut vx_ix: usize = 0;
    while vx_ix < vertex_first_index.len() {
        vertex_first_index.write_at(vx_ix, NO_INDEX);
        vx_ix += 1;
    }

    // The fill above initializes the complete run before this lookup phase
    // reads any destination slot through the list's initialized read surface.
    let num_vertices: u32 = mesh.num_vertices() as u32;
    let mut ix: usize = 0;
    while ix < mesh.num_indices() {
        let vx: u32 = mesh.vertex_indices_view().copy_at(ix);
        if vx < num_vertices && mesh.vertex_first_index_view().copy_at(vx as usize) == NO_INDEX {
            vertex_first_index.write_at(vx as usize, ix as u32);
        }
        ix += 1;
    }
}

// ufbx.c:16691-16765 `ufbxi_finalize_mesh`
#[inline(never)]
pub(crate) fn finalize_mesh(
    buf: &BufView,
    error: &crate::native::error::ErrorView,
    mesh: &View<Mesh>,
) -> Result<(), Fail> {
    if mesh.vertices().count == 0 {
        // The bitwise copy mirrors C's struct assignment of two non-`Copy` list
        // structs, and both stay owned by the same mesh.
        mesh.set_vertices(mesh.vertex_position().values());
    }
    if mesh.vertex_indices().count == 0 {
        // As above, for the index list.
        mesh.set_vertex_indices(mesh.vertex_position().indices());
    }

    // SAFETY: `*_raw()` addresses the mesh's own live field, which the macro
    // reads and asserts is zero or already equal before writing it.
    unsafe {
        ufbxi_patch_zero!(*mesh.num_vertices_raw(), mesh.vertices().count);
        ufbxi_patch_zero!(*mesh.num_indices_raw(), mesh.vertex_indices().count);
        ufbxi_patch_zero!(*mesh.num_faces_raw(), mesh.faces().count);
    }

    if mesh.num_triangles() == 0 || mesh.max_face_triangles() == 0 {
        let mut num_triangles: usize = 0;
        let mut max_face_triangles: usize = 0;
        let mut num_bad_faces: [usize; 3] = [0; 3];
        // C: `ufbxi_nounroll ufbxi_for_list(ufbx_face, face, mesh->faces)`
        for face in Run::from_list(mesh.faces_view()).iter() {
            if face.num_indices() >= 3 {
                let tris: usize = face.num_indices() as usize - 2;
                num_triangles = num_triangles.wrapping_add(tris);
                max_face_triangles = max_sz(max_face_triangles, tris);
            } else {
                // The `< 3` branch bounds the index into `num_bad_faces`.
                num_bad_faces[face.num_indices() as usize] += 1;
            }
        }

        // SAFETY: `*_raw()` addresses the mesh's own live field, which the
        // macro asserts is zero or already equal before writing it.
        unsafe {
            ufbxi_patch_zero!(*mesh.num_triangles_raw(), num_triangles);
            ufbxi_patch_zero!(*mesh.max_face_triangles_raw(), max_face_triangles);
            ufbxi_patch_zero!(*mesh.num_empty_faces_raw(), num_bad_faces[0]);
            ufbxi_patch_zero!(*mesh.num_point_faces_raw(), num_bad_faces[1]);
            ufbxi_patch_zero!(*mesh.num_line_faces_raw(), num_bad_faces[2]);
        }
    }

    if !mesh.skinned_position().exists() {
        mesh.set_skinned_is_local(true);
        // SAFETY (both reads): `vertex_position`/`vertex_normal` are the mesh's
        // own initialized attribute fields; the bitwise copy mirrors C's struct
        // assignment of two non-`Copy` structs, and both stay owned by the same
        // mesh.
        mesh.set_skinned_position(unsafe { core::ptr::read(mesh.vertex_position_ptr()) });
        mesh.set_skinned_normal(unsafe { core::ptr::read(mesh.vertex_normal_ptr()) });
    }

    if mesh.vertex_first_index().count == 0 {
        mesh.vertex_first_index_view()
            .set_count(mesh.num_vertices());
        // The push result is a fresh `num_vertices`-long run owned by `buf`.
        mesh.vertex_first_index_view()
            .set_data(buf.push::<u32>(mesh.num_vertices()));
        ufbxi_check_err!(
            error,
            !mesh.vertex_first_index().data.is_null(),
            "mesh->vertex_first_index.data"
        );
        update_vertex_first_index(mesh);
    }

    if mesh.uv_sets().count == 0 && mesh.vertex_uv().exists() {
        let uv_set: *mut UvSet = buf.push_zero::<UvSet>(1);
        ufbxi_check_err!(error, !uv_set.is_null(), "uv_set");

        // SAFETY: `uv_set` is the fresh non-null single-element run pushed
        // above, owned by `buf` — write-capable provenance.
        let uv_set_view: &View<UvSet> = unsafe { View::<UvSet>::from_ptr(uv_set) };
        // SAFETY: `name_raw()` addresses the fresh set's own `ufbx_string`; each
        // attribute lens addresses its own field, and the bitwise copies mirror
        // C's struct assignment of non-`Copy` attribute structs read from the
        // mesh's own fields, which the mesh keeps owning too.
        unsafe {
            (*uv_set_view.name_raw()).data = EMPTY_CHAR.as_ptr();
            core::ptr::write(
                uv_set_view.vertex_uv_raw(),
                core::ptr::read(mesh.vertex_uv_ptr()),
            );
            core::ptr::write(
                uv_set_view.vertex_tangent_raw(),
                core::ptr::read(mesh.vertex_tangent_ptr()),
            );
            core::ptr::write(
                uv_set_view.vertex_bitangent_raw(),
                core::ptr::read(mesh.vertex_bitangent_ptr()),
            );
        }

        mesh.uv_sets_view().set_data(uv_set);
        mesh.uv_sets_view().set_count(1);
    }

    if mesh.color_sets().count == 0 && mesh.vertex_color().exists() {
        let color_set: *mut ColorSet = buf.push_zero::<ColorSet>(1);
        ufbxi_check_err!(error, !color_set.is_null(), "color_set");

        // SAFETY: `color_set` is the fresh non-null single-element run pushed
        // above, owned by `buf` — write-capable provenance.
        let color_set_view: &View<ColorSet> = unsafe { View::<ColorSet>::from_ptr(color_set) };
        // SAFETY: `name_raw()` addresses the fresh set's own `ufbx_string` and
        // the attribute lens its own field; the bitwise copy mirrors C's struct
        // assignment of a non-`Copy` attribute struct read from the mesh's own
        // field, which the mesh keeps owning too.
        unsafe {
            (*color_set_view.name_raw()).data = EMPTY_CHAR.as_ptr();
            core::ptr::write(
                color_set_view.vertex_color_raw(),
                core::ptr::read(mesh.vertex_color_ptr()),
            );
        }

        mesh.color_sets_view().set_data(color_set);
        mesh.color_sets_view().set_count(1);
    }

    patch_mesh_reals(mesh);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_prop() -> Prop {
        Prop {
            name: EMPTY_STRING.0,
            _internal_key: 0,
            type_: PropType::Unknown,
            flags: PropFlags::NONE,
            value_str: EMPTY_STRING.0,
            value_blob: EMPTY_BLOB.0,
            value_int: 0,
            value_vec4: Vec4::default(),
        }
    }

    #[test]
    fn synthetic_static_property_initializers() {
        static INT_NAME: &[u8] = b"Abcd integer\0ignored";
        let mut int_prop = empty_prop();
        init_synthetic_int_prop(&mut int_prop, INT_NAME, -37, PropType::Integer);
        assert_eq!(int_prop.name.data, INT_NAME.as_ptr());
        assert_eq!(int_prop.name.length, 12);
        assert_eq!(int_prop._internal_key, get_name_key(b"Abcd"));
        assert_eq!(int_prop.type_, PropType::Integer);
        assert_eq!(
            int_prop.flags.raw(),
            PropFlags::SYNTHETIC.raw() | PropFlags::VALUE_REAL.raw() | PropFlags::VALUE_INT.raw()
        );
        assert_eq!(int_prop.value_vec4.x, -37.0);
        assert_eq!(int_prop.value_int, -37);
        assert_eq!(int_prop.value_str.data, EMPTY_CHAR.as_ptr());

        static REAL_NAME: &[u8] = b"Efgh real\0tail";
        let mut real_prop = empty_prop();
        init_synthetic_real_prop(&mut real_prop, REAL_NAME, 3.75, PropType::Number);
        assert_eq!(real_prop.name.data, REAL_NAME.as_ptr());
        assert_eq!(real_prop.name.length, 9);
        assert_eq!(real_prop._internal_key, get_name_key(b"Efgh"));
        assert_eq!(real_prop.type_, PropType::Number);
        assert_eq!(
            real_prop.flags.raw(),
            PropFlags::SYNTHETIC.raw() | PropFlags::VALUE_REAL.raw()
        );
        assert_eq!(real_prop.value_vec4.x, 3.75);
        assert_eq!(real_prop.value_int, 3);
        assert_eq!(real_prop.value_str.data, EMPTY_CHAR.as_ptr());

        static VEC3_NAME: &[u8] = b"Ijkl vector\0suffix";
        let mut vec3_prop = empty_prop();
        vec3_prop.value_vec4.w = 91.0;
        init_synthetic_vec3_prop(
            PropView::from_mut(&mut vec3_prop),
            VEC3_NAME,
            &Vec3 {
                x: 2.25,
                y: -4.5,
                z: 8.75,
            },
            PropType::Vector,
        );
        assert_eq!(vec3_prop.name.data, VEC3_NAME.as_ptr());
        assert_eq!(vec3_prop.name.length, 11);
        assert_eq!(vec3_prop._internal_key, get_name_key(b"Ijkl"));
        assert_eq!(vec3_prop.type_, PropType::Vector);
        assert_eq!(
            vec3_prop.flags.raw(),
            PropFlags::SYNTHETIC.raw() | PropFlags::VALUE_VEC3.raw()
        );
        assert_eq!(vec3_prop.value_vec4.x, 2.25);
        assert_eq!(vec3_prop.value_vec4.y, -4.5);
        assert_eq!(vec3_prop.value_vec4.z, 8.75);
        assert_eq!(vec3_prop.value_vec4.w, 91.0);
        assert_eq!(vec3_prop.value_int, 2);
        assert_eq!(vec3_prop.value_str.data, EMPTY_CHAR.as_ptr());

        static NO_NUL: &[u8] = b"No NUL here";
        let mut no_nul_prop = empty_prop();
        let no_nul = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            init_synthetic_int_prop(&mut no_nul_prop, NO_NUL, 17, PropType::Distance)
        }));
        assert!(no_nul.is_err());
        assert_eq!(no_nul_prop.type_, PropType::Distance);
        assert_eq!(no_nul_prop.name.data, NO_NUL.as_ptr());
        assert_eq!(no_nul_prop.name.length, 0);
        assert_eq!(no_nul_prop.value_int, 0);

        static SHORT: &[u8] = b"abc\0";
        let mut short_prop = empty_prop();
        let short = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            init_synthetic_real_prop(&mut short_prop, SHORT, 6.5, PropType::Number)
        }));
        assert!(short.is_err());
        assert_eq!(short_prop.name.length, 3);
        assert_eq!(short_prop.value_vec4.x, 6.5);
        assert_eq!(short_prop.value_int, 6);
        assert_eq!(short_prop._internal_key, 0);
    }

    #[test]
    fn typed_strblob_publication_preserves_member_and_empty_policy() {
        static STRING_BYTES: &[u8] = b"string";
        static BLOB_BYTES: &[u8] = b"blob\0bytes";

        let mut storage = Strblob {
            str_: EMPTY_STRING.0,
        };
        let storage = View::<Strblob, Mut>::from_mut(&mut storage);

        let string = String::new_c(STRING_BYTES.as_ptr(), STRING_BYTES.len());
        strblob_set_string(storage, string);
        assert_eq!(strblob_data(storage, false), STRING_BYTES.as_ptr());
        assert_eq!(strblob_length(storage, false), STRING_BYTES.len());

        strblob_set_string(storage, String::new_c(core::ptr::null(), 0));
        assert_eq!(strblob_data(storage, false), EMPTY_CHAR.as_ptr());
        assert_eq!(strblob_length(storage, false), 0);

        let blob_string = String::new_c(BLOB_BYTES.as_ptr(), BLOB_BYTES.len());
        strblob_set_blob(storage, Blob::from_string(blob_string));
        assert_eq!(strblob_data(storage, true), BLOB_BYTES.as_ptr());
        assert_eq!(strblob_length(storage, true), BLOB_BYTES.len());

        strblob_set_blob(storage, Blob::empty());
        assert!(strblob_data(storage, true).is_null());
        assert_eq!(strblob_length(storage, true), 0);
    }

    #[test]
    fn filename_options_publish_sanitized_and_raw_metadata() {
        let path = format!(
            "{}/../../data/maya_cube_7500_binary.fbx",
            env!("CARGO_MANIFEST_DIR")
        );
        let data = std::fs::read(path).expect("read filename metadata fixture");

        let scene = crate::load_memory(
            &data,
            crate::LoadOpts {
                filename: crate::StringOpt::Ref("path\0/file.fbx"),
                ..Default::default()
            },
        )
        .expect("load with string filename option");
        assert_eq!(scene.metadata.filename.as_ref(), "path\u{fffd}/file.fbx");
        assert_eq!(scene.metadata.filename.length, 16);
        assert_eq!(&*scene.metadata.raw_filename, b"path\0/file.fbx");
        assert_eq!(scene.metadata.relative_root.as_ref(), "path\u{fffd}");
        assert_eq!(&*scene.metadata.raw_relative_root, b"path\0");
        assert!(!scene.nodes.is_empty());

        let scene = crate::load_memory(
            &data,
            crate::LoadOpts {
                raw_filename: crate::BlobOpt::Ref(b"path\0/file.fbx"),
                ..Default::default()
            },
        )
        .expect("load with raw filename option");
        assert_eq!(scene.metadata.filename.as_ref(), "path\u{fffd}/file.fbx");
        assert_eq!(scene.metadata.filename.length, 16);
        assert_eq!(&*scene.metadata.raw_filename, b"path\0/file.fbx");
        assert_eq!(scene.metadata.relative_root.as_ref(), "path\u{fffd}");
        assert_eq!(&*scene.metadata.raw_relative_root, b"path\0");
        assert!(!scene.nodes.is_empty());
    }
}
