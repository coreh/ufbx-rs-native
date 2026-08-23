//! Port of the `// -- Reading the parsed data` banner section (ufbx.c:11762-15311).
//!
//! FIRST UNIT: ufbx.c:11762-12218 — embedded blobs, the property reader with
//! its sort/dedup helpers, thumbnails, `SceneInfo`, the header extension
//! (including the KTime-unit decision), exporter matching from the `Creator`
//! string, the document root ID and the `Definitions` property templates.
//!
//! SECOND UNIT: ufbx.c:12220-12653 — the name-ID categories and synthetic-ID
//! allocation, `Type::Name` splitting, the FBX-ID / FBX-attr hash maps, the
//! element push foundations (`ufbxi_push_element_size` /
//! `ufbxi_push_synthetic_element_size`), the tmp-connection builders, the
//! synthetic property initializers, the geometry-transform and scale helper
//! node setup, and the first per-type readers (`Model`, the generic element
//! reader and `Unknown`).
//!
//! THIRD UNIT: ufbx.c:12655-13137 — the index sentinels and index-error
//! handling (`ufbxi_fix_index` / `ufbxi_check_indices`), the vertex-attribute
//! reader with its mapping-mode dispatch, truncated-array reading, the
//! UV/color-set and blend-offset sorts, and the blend shape readers
//! (`ufbxi_read_shape` / `ufbxi_read_synthetic_blend_shapes`).
//!
//! FOURTH UNIT: ufbx.c:13139-13432 — index processing (`ufbxi_process_indices`
//! builds the face list, vertex-first-index table and the consecutive/zero
//! index hints), `ufbxi_patch_mesh_reals`, and the face-group machinery
//! (`ufbxi_assign_face_groups` / `ufbxi_update_face_groups`) with its loose
//! hash dedup, `ufbxi_less_int32` unstable sort and mesh-part accounting.
//!
//! FIFTH UNIT: ufbx.c:13434-13894 — `ufbxi_read_mesh` (the geometry node
//! reader: vertices/indices, edges, the `LayerElement*` dispatch loop with the
//! 6x00 mesh-texture layers, the `Layer` tangent/bitangent-to-UV-set binding
//! and the subdivision properties) plus the NURBS readers
//! (`ufbxi_read_nurbs_topology` / `ufbxi_read_nurbs_curve` /
//! `ufbxi_read_nurbs_surface`).
//!
//! SIXTH UNIT: ufbx.c:13896-14255 — `ufbxi_read_line` (segment splitting on
//! the complemented end-point indices), `ufbxi_read_transform_matrix` and the
//! deformer readers (`ufbxi_read_bone`, `ufbxi_read_marker`,
//! `ufbxi_read_skin`, `ufbxi_read_skin_cluster`, `ufbxi_read_blend_channel`),
//! plus the animation-curve tangent foundations: the `ufbxi_key_flags` bits,
//! the three auto-tangent solvers, `ufbxi_solve_tcb` and
//! `ufbxi_read_extrapolation`.
//!
//! SEVENTH UNIT: ufbx.c:14257-14771 — `ufbxi_read_animation_curve` (the
//! run-length-encoded keyframe/tangent decoder), the material/texture/video
//! readers with their filename fallback ladders, `ufbxi_read_anim_stack` (name
//! map), `ufbxi_read_pose`, the shader binding table with its stable sort, and
//! the selection set/node readers.
//!
//! EIGHTH UNIT: ufbx.c:14773-15310 (end of section) — the remaining leaf
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
//! NINTH UNIT: ufbx.c:15312-15764 — the start of the
//! `// -- Pre-7000 "Take" based animation` banner section:
//! `ufbxi_double_to_char` and the heterogenous-`double`-array keyframe decoder
//! `ufbxi_read_take_anim_channel` (slope/weight mode ladder), the recursive
//! `ufbxi_read_take_prop_channel` (`Transform` flattening and the 1-3 compound
//! channel scan), `ufbxi_read_take_object`, `ufbxi_read_take` (Take → anim
//! stack + `BaseLayer` synthesis, plus the post-7000 time-only fallback) and
//! the `ufbxi_read_takes` top-level loop.
//!
//! TENTH UNIT: ufbx.c:15766-16332 — `ufbxi_read_legacy_settings` (the pre-6000
//! `Version5/Settings` frame-rate override), the root-node setup helpers
//! (`ufbxi_unscaled_transform_to_matrix`, `ufbxi_setup_root_node`,
//! `ufbxi_supports_version`) and the pre-6000 "legacy" object readers: the
//! `ufbxi_legacy_prop` format-string tables plus `ufbxi_read_legacy_prop` /
//! `ufbxi_read_legacy_props`, then `ufbxi_read_legacy_material`,
//! `ufbxi_read_legacy_link`, `ufbxi_read_legacy_light`,
//! `ufbxi_read_legacy_camera`, `ufbxi_read_legacy_limb_node` and
//! `ufbxi_read_legacy_mesh`, and the top-level driver `ufbxi_read_root`.
//!
//! ELEVENTH UNIT: ufbx.c:16333-16765 (end of section) — the remaining pre-6000
//! readers `ufbxi_read_legacy_media` / `ufbxi_read_legacy_model`, the filename
//! manipulation block (`ufbxi_trim_delimiters`, `ufbxi_init_file_paths`, the
//! `ufbxi_strblob` string/blob overlay with its `raw`-discriminated accessors,
//! `ufbxi_is_absolute_path` and `ufbxi_resolve_relative_filename`), the
//! `ufbxi_open_file` callback shim, and the shared mesh finalizer
//! (`ufbxi_patch_zero`, `ufbxi_update_vertex_first_index`,
//! `ufbxi_finalize_mesh`), and the legacy driver `ufbxi_read_legacy_root`.
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
use core::ffi::c_void;
use core::mem::size_of;
use core::mem::MaybeUninit;

use crate::generated::{
    AnimCurve, AnimLayer, AnimStack, AnimValue, AudioClip, AudioLayer, BlendChannel, BlendDeformer,
    BlendShape, Bone, BonePose, CacheDeformer, CacheFile, Camera, CameraSwitcher, Character,
    ColorSet, Constraint, ConstraintType, DisplayLayer, Edge, Element, ElementType, Empty, Error,
    Exporter, Extrapolation, ExtrapolationMode, Face, FaceGroup, IndexErrorHandling, InheritMode,
    InheritModeHandling, Interpolation, Keyframe, Light, LineCurve, LineSegment, LodGroup, Marker,
    MarkerType, Material, Matrix, Mesh, MeshPart, MetadataObject, Node as UfbxNode, NurbsCurve,
    NurbsSurface, NurbsTopology, NurbsTrimBoundary, NurbsTrimSurface, OpenFileInfo, OpenFileType,
    Pose, Prop, PropFlags, PropType, Props, RawOpenFileCb, RawStream, SelectionNode, SelectionSet,
    Shader, ShaderBinding, ShaderPropBinding, SkinCluster, SkinDeformer, SkinningMethod,
    StereoCamera, SubdivisionBoundary, SubdivisionDisplayMode, Texture, TextureType, Thumbnail,
    ThumbnailFormat, TimeMode, Transform, Unknown, UvSet, Vec3, Vec4, VertexAttrib, VertexReal,
    VertexVec2, VertexVec3, VertexVec4, Video, WarningType,
};
use crate::native::allocator::{grow_array, Allocator};
use crate::native::api::{
    find_int_len as api_find_int_len, find_prop as api_find_prop, find_prop_len,
    transform_to_matrix, EMPTY_BLOB, EMPTY_STRING, IDENTITY_MATRIX, IDENTITY_TRANSFORM,
};
use crate::native::buf::{buf_clear, pop, push_copy, push_copy_fast, push_size, BufView};
use crate::native::error::{
    memchr, memcmp, strcmp, strlen, strncmp, ufbxi_check, ufbxi_check_err, ufbxi_check_msg,
    ufbxi_check_return, ufbxi_fail, ufbxi_fail_msg, ufbxi_fmt_err_info, Fail, EMPTY_CHAR,
};
use crate::native::float_parse::parse_double;
use crate::native::hash::{hash64, hash_ptr_id, map_find, map_insert, PtrId};
use crate::native::parse::{
    array_type_size, find_array, find_child, find_child_strcmp, find_int, find_prop, find_val1,
    find_val2, find_vec3, get_array, get_dom_node, get_name_key, get_name_key_c, get_prop_type,
    get_val1, get_val2, get_val3, get_val4, get_val5, get_val_at, get_val_type,
    init_node_prop_names, is_node_property_name, is_vec3_one, is_vec3_zero, parse_legacy_toplevel,
    parse_toplevel, parse_toplevel_child, push_element_extra, retain_toplevel, Ascii, Context,
    ElementInfo, FbxAttrEntry, FbxIdEntry, MeshExtra, Node, NodeView, PropView, PropsView,
    PtrFbxIdEntry, Template, TextureExtra, TmpAnimStack, TmpBonePose, TmpConnection,
    TmpMeshTexture, ValueArray, ValueType,
};
use crate::native::platform::{
    add_ptr, f64_to_i64, macro_lower_bound_eq, macro_stable_sort, math, max_real, max_sz, min32,
    min_real, min_sz, pack_version, stable_sort, to_size, ufbx_assert, ufbxi_dev_assert,
    ufbxi_ignore, ufbxi_maybe_null, ufbxi_unreachable, unstable_sort, FACE_GROUP_HASH_BITS,
    NO_INDEX,
};
use crate::native::string_pool as sp;
use crate::native::string_pool::{push_string_place_blob, push_string_place_str};
use crate::native::thread::{
    thread_pool_available_tasks, thread_pool_flush_group, thread_pool_wait_all,
    thread_pool_wait_group, THREAD_GROUP_COUNT,
};
use crate::native::view::{Const, SliceViewIter, View};
use crate::native::warnings::ufbxi_warnf;
use crate::prelude::as_f64;
use crate::prelude::{Blob, List, OpenFileContext, Real, Ref, String};

// ufbx.h:3618 `UFBX_ENUM_TYPE(ufbx_thumbnail_format, UFBX_THUMBNAIL_FORMAT, UFBX_THUMBNAIL_FORMAT_RGBA_32);`
// expanding to `enum { UFBX_THUMBNAIL_FORMAT_COUNT = UFBX_THUMBNAIL_FORMAT_RGBA_32 + 1 }`.
// Hand-duplicated from the generated enum's last variant so an upstream enum
// change tracks automatically through regen (precedent: `ELEMENT_TYPE_COUNT`
// in `native::parse`). `i32` because C compares it against an `int32_t`.
pub(crate) const THUMBNAIL_FORMAT_COUNT: i32 = ThumbnailFormat::Rgba32 as i32 + 1;

// ufbx.c:11764-11796 `ufbxi_read_embedded_blob`
#[inline(never)]
pub(crate) unsafe fn read_embedded_blob(
    uc: &Context,
    dst_blob: *mut Blob,
    node: Option<&NodeView>,
) -> Result<(), Fail> {
    let node: &NodeView = match node {
        Some(node) => node,
        None => return Ok(()),
    };

    let content_arr: *mut ValueArray = get_array(node, b'C');
    // SAFETY: `content_arr` is non-null (checked) and `get_array` returns the
    // node's own array descriptor, live for as long as the parse tree.
    if !content_arr.is_null() && unsafe { (*content_arr).size } > 0 {
        let content: String;
        // SAFETY: as above — `content_arr` is a live array descriptor whose
        // `'C'` payload is a run of `size` `ufbx_string` values.
        let (num_parts, parts): (usize, *mut String) =
            unsafe { ((*content_arr).size, (*content_arr).data as *mut String) };

        if num_parts == 1 && !uc.from_ascii() {
            // SAFETY: `num_parts == 1`, so `parts` addresses one live `String`.
            content = unsafe { *parts };
        } else {
            let mut total_size: usize = 0;
            // C: `ufbxi_for(ufbx_string, part, parts, num_parts)`
            let mut part = parts;
            let part_end = add_ptr(parts, num_parts);
            while part != part_end {
                // SAFETY: `part` walks `parts..parts + num_parts`, all live
                // `String` entries of the `'C'` array, and is before `part_end`,
                // so the advance lands at most one past the array's end.
                unsafe {
                    total_size = total_size.wrapping_add((*part).length);
                    part = part.add(1);
                }
            }
            let dst_begin: *mut u8 = uc.result_view().push::<u8>(total_size);
            ufbxi_check!(uc, !dst_begin.is_null(), "dst");
            content = String::new_c(dst_begin, total_size);
            let mut dst = dst_begin;
            let mut part = parts;
            while part != part_end {
                // SAFETY: `part` addresses a live `String` whose `data` spans
                // `length` bytes; `dst` walks the freshly pushed `total_size`
                // buffer, which is the sum of every part's `length`, so the copy
                // fits, its bytes are consumed from that destination, and the
                // result arena is disjoint from the parts; `part` is before
                // `part_end`, so the advance lands at most one past the end.
                unsafe {
                    core::ptr::copy_nonoverlapping((*part).data, dst, (*part).length);
                    dst = dst.add((*part).length);
                    part = part.add(1);
                }
            }
        }

        // SAFETY: `dst_blob` is the caller's writable `ufbx_blob` out-pointer.
        unsafe {
            (*dst_blob).data = content.data;
            (*dst_blob).size = content.length;
        }
    }

    Ok(())
}

// ufbx.c:11798-11869 `ufbxi_read_property`
#[inline(never)]
pub(crate) unsafe fn read_property(
    uc: &Context,
    node: &NodeView,
    prop: *mut Prop,
    version: i32,
) -> Result<(), Fail> {
    // SAFETY: `prop` is the caller's writable `ufbx_prop` slot in the result
    // arena (fn contract) — write-capable provenance, stable for this call.
    let prop: &PropView = unsafe { PropView::from_ptr(prop) };

    let mut type_str: *const u8 = core::ptr::null();
    let mut subtype_str: *const u8 = core::ptr::null();
    ufbxi_check!(
        uc,
        // SAFETY: the format string `"SC"` matches the `name`/`type_str`
        // out-pointer types that `get_val2` writes through; `name_raw()` is the
        // viewed prop's own live `ufbx_string` field.
        unsafe {
            get_val2(
                node,
                b"SC\0".as_ptr(),
                prop.name_raw() as *mut c_void,
                &mut type_str as *mut *const u8 as *mut c_void,
            )
        },
        "ufbxi_get_val2(node, \"SC\", &prop->name, (char**)&type_str)"
    );
    let mut val_ix: u32 = 2;
    if version == 70 {
        // C: `ufbxi_get_val_at(node, val_ix++, ...)` — the post-increment
        // happens while evaluating the (single) check condition.
        let ix = val_ix;
        val_ix = val_ix.wrapping_add(1);
        ufbxi_check!(
            uc,
            // SAFETY: fmt `'C'` pairs with the `*mut *const u8` out-pointer
            // `&mut subtype_str`, which is a live local.
            unsafe {
                get_val_at(
                    node,
                    ix as usize,
                    b'C',
                    &mut subtype_str as *mut *const u8 as *mut c_void,
                )
            },
            "ufbxi_get_val_at(node, val_ix++, 'C', (char**)&subtype_str)"
        );
    }

    let mut flags: u32 = 0;
    // `name` was filled in by the `"SC"` fetch above (an interned pool string).
    prop.set_internal_key(get_name_key(prop.name_view().bytes()));

    // C leaves `flags_str` uninitialized; it is only read when the `'S'` fetch
    // below succeeds, which fully writes it.
    let mut flags_str: String = String::new_c(core::ptr::null(), 0);
    let ix = val_ix;
    val_ix = val_ix.wrapping_add(1);
    // SAFETY: fmt `'S'` pairs with the `*mut String` out-pointer
    // `&mut flags_str`, which is a live local.
    if unsafe {
        get_val_at(
            node,
            ix as usize,
            b'S',
            &mut flags_str as *mut String as *mut c_void,
        )
    } {
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

    // SAFETY: `type_str` was written by the `"SC"` fetch above as a
    // NUL-terminated string owned by the parse tree — `get_prop_type`'s contract.
    prop.set_type(unsafe { get_prop_type(uc, type_str) });
    if prop.type_() == PropType::Unknown && !subtype_str.is_null() {
        // SAFETY: `subtype_str` is non-null (checked) and was written by the
        // `'C'` fetch above as a NUL-terminated parse-tree string.
        prop.set_type(unsafe { get_prop_type(uc, subtype_str) });
    }

    // SAFETY: fmt `'L'` pairs with the `*mut i64` out-pointer `value_int_raw()`,
    // the viewed prop's own live `int64_t` field.
    if unsafe {
        get_val_at(
            node,
            val_ix as usize,
            b'L',
            prop.value_int_raw() as *mut c_void,
        )
    } {
        flags |= PropFlags::VALUE_INT.raw();
    }

    // C-parity: `prop->value_real_arr[]` is the `ufbx_prop` value union's
    // 4-real view (ufbx.h); the generated struct keeps only the `value_vec4`
    // member, so the array view is reached by pointer cast.
    let value_real_arr: *mut Real = prop.value_vec4_raw() as *mut Real;
    let mut real_ix: usize = 0;
    while real_ix < 4 {
        // SAFETY: `real_ix < 4` keeps `value_real_arr.add(real_ix)` inside the
        // 4-`Real` value union arm, and fmt `'R'` pairs with a `*mut Real`.
        if !unsafe {
            get_val_at(
                node,
                (val_ix as usize).wrapping_add(real_ix),
                b'R',
                value_real_arr.add(real_ix) as *mut c_void,
            )
        } {
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

    // SAFETY: fmt `'S'` pairs with the `*mut String` out-pointer
    // `value_str_raw()`, the viewed prop's own live `ufbx_string` field.
    if unsafe {
        get_val_at(
            node,
            val_ix as usize,
            b'S',
            prop.value_str_raw() as *mut c_void,
        )
    } {
        if prop.value_str().length > 0 {
            // SAFETY: fmt `'b'` pairs with the `*mut Blob` out-pointer
            // `value_blob_raw()`, the viewed prop's own live `ufbx_blob` field.
            ufbxi_ignore!(unsafe {
                get_val_at(
                    node,
                    val_ix as usize,
                    b'b',
                    prop.value_blob_raw() as *mut c_void,
                )
            });
        }
        flags |= PropFlags::VALUE_STR.raw();
    } else {
        prop.set_value_str(EMPTY_STRING.0);
    }

    // Very unlikely, seems to only exist in some "non standard" FBX files
    if node.num_children() > 0 {
        let binary = find_child(node, sp::BinaryData.as_ptr());
        // SAFETY: `value_blob_raw()` is the viewed prop's own `value_blob`
        // field, which is what `read_embedded_blob` writes.
        unsafe { read_embedded_blob(uc, prop.value_blob_raw(), binary) }?;
        flags |= PropFlags::VALUE_BLOB.raw();
    }

    prop.set_flags(PropFlags::from_raw(flags));

    Ok(())
}

// ufbx.c:11871-11876 `ufbxi_prop_less`
#[inline(always)]
pub(crate) unsafe fn prop_less(a: *mut Prop, b: *mut Prop) -> bool {
    // SAFETY: `a` and `b` are live `ufbx_prop` elements handed to the sort
    // comparator from the property array being sorted (fn contract). The sorts
    // hand them out through `&T`-derived pointers, so they may only anchor
    // read-only `Const` views; the comparator never writes.
    let (a, b): (&View<Prop, Const>, &View<Prop, Const>) = unsafe {
        (
            View::<Prop, Const>::from_ptr(a),
            View::<Prop, Const>::from_ptr(b),
        )
    };
    if a._internal_key() < b._internal_key() {
        return true;
    }
    if a._internal_key() > b._internal_key() {
        return false;
    }
    // SAFETY: both `name.data` pointers are NUL-terminated strings interned in
    // the string pool — `strcmp`'s contract.
    let cmp: i32 = unsafe { strcmp(a.name().data, b.name().data) };
    cmp < 0
}

// ufbx.c:11878-11883 `ufbxi_sort_properties`
#[inline(never)]
pub(crate) unsafe fn sort_properties(
    uc: &Context,
    props: *mut Prop,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the allocator, data pointer and size slots are uc's own
        // `ator_tmp`/`tmp_arr`/`tmp_arr_size` fields, reached through its
        // views — the matched triple `grow_array` requires.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count.wrapping_mul(size_of::<Prop>()),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_prop)))"
    );
    // SAFETY: `props` spans `count` live `ufbx_prop` values (fn contract) and
    // `uc.tmp_arr()` was just grown to `count * size_of::<Prop>()` bytes of
    // scratch, so both the input run and the merge buffer are in bounds; the
    // comparator only ever sees elements of that run.
    unsafe {
        macro_stable_sort::<Prop>(32, props, uc.tmp_arr() as *mut Prop, count, |a, b| {
            prop_less(a as *mut Prop, b as *mut Prop)
        })
    };
    Ok(())
}

// ufbx.c:11885-11901 `ufbxi_deduplicate_properties`
#[inline(never)]
pub(crate) unsafe fn deduplicate_properties(list: *mut List<Prop>) {
    // SAFETY: `list` is the caller's live `ufbx_prop_list` (fn contract).
    if unsafe { (*list).count } >= 2 {
        // SAFETY: as above; `data` spans `count` live `ufbx_prop` values.
        let ps: *mut Prop = unsafe { (*list).data } as *mut Prop;
        let mut dst: usize = 0;
        let mut src: usize = 0;
        // SAFETY: as above.
        let end: usize = unsafe { (*list).count };
        while src < end {
            // SAFETY: `src < end` and the `src + 1 < end` guard short-circuits
            // first, so both indices address live elements of the `end`-long
            // property run at `ps`.
            if unsafe { src + 1 < end && (*ps.add(src)).name.data == (*ps.add(src + 1)).name.data }
            {
                src += 1;
            } else if dst != src {
                // SAFETY: `src < end` and `dst <= src`, so both index live
                // elements of the `end`-long run at `ps`; `Prop` is `Copy`.
                unsafe { *ps.add(dst) = *ps.add(src) };
                dst += 1;
                src += 1;
            } else {
                dst += 1;
                src += 1;
            }
        }
        // SAFETY: `list` is the caller's live `ufbx_prop_list`.
        unsafe { (*list).count = dst };
    }
}

// ufbx.c:11903-11932 `ufbxi_read_properties`
#[inline(never)]
pub(crate) unsafe fn read_properties(
    uc: &Context,
    parent: &NodeView,
    props: *mut Props,
) -> Result<(), Fail> {
    // SAFETY: `props` is the caller's writable `ufbx_props` slot (fn contract),
    // owned by uc's result arena — write-capable provenance, stable for this call.
    let props: &PropsView = unsafe { PropsView::from_ptr(props) };
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
        // pushed above; `NodeView::from_ptr` mints a view over that child.
        unsafe {
            read_property(
                uc,
                NodeView::from_ptr(node.children().add(i)),
                props.props_data().add(i),
                version,
            )
        }?;
        i += 1;
    }

    // SAFETY: `props.data` spans `props.count` live `ufbx_prop` values, just
    // filled in by the loop above — `sort_properties`' contract.
    unsafe { sort_properties(uc, props.props_data(), props.props_count()) }?;
    // SAFETY: `props_raw()` is the viewed table's own live property list.
    unsafe { deduplicate_properties(props.props_raw()) };

    Ok(())
}

// ufbx.c:11934-11967 `ufbxi_read_thumbnail`
#[inline(never)]
pub(crate) unsafe fn read_thumbnail(
    uc: &Context,
    node: &NodeView,
    thumbnail: *mut Thumbnail,
) -> Result<(), Fail> {
    // SAFETY: `thumbnail` is the caller's writable `ufbx_thumbnail` slot (fn
    // contract), so `&mut (*thumbnail).props` is its live `props` field.
    unsafe { read_properties(uc, node, &mut (*thumbnail).props) }?;

    // SAFETY: `&raw mut (*thumbnail).props` projects the live `props` field just
    // filled in above, which `PropsView::from_ptr` mints a view over; both names
    // are NUL-terminated literals — `find_int`'s contract.
    let (custom_width, custom_height): (i64, i64) = unsafe {
        let props: &PropsView = PropsView::from_ptr(&raw mut (*thumbnail).props);
        (
            api_find_int_len(props, b"CustomWidth", 0),
            api_find_int_len(props, b"CustomHeight", 0),
        )
    };

    let mut format: i32 = 0;
    // SAFETY: the name is a NUL-terminated literal — `find_child_strcmp`'s
    // contract.
    let format_node = unsafe { find_child_strcmp(node, b"Format\0".as_ptr()) };
    if format_node.is_some()
        // SAFETY: `format_node` is `Some` (checked, and `&&` short-circuits);
        // fmt `"I"` pairs with the `*mut i32` out-pointer `&mut format`.
        && unsafe {
            get_val1(
                format_node.unwrap(),
                b"I\0".as_ptr(),
                &mut format as *mut i32 as *mut c_void,
            )
        }
    {
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
            // SAFETY: `thumbnail` is the caller's writable slot.
            unsafe { (*thumbnail).format = thumbnail_format_from_raw(format + 1) };
        }
    }

    let mut size: i32 = 0;
    // SAFETY: the child name is a NUL-terminated interned string and fmt `"I"`
    // pairs with the `*mut i32` out-pointer `&mut size`.
    if unsafe {
        find_val1(
            node,
            sp::Size.as_ptr(),
            b"I\0".as_ptr(),
            &mut size as *mut i32 as *mut c_void,
        )
    } {
        if size > 0 {
            // SAFETY: `thumbnail` is the caller's writable slot.
            unsafe {
                (*thumbnail).width = size as u32;
                (*thumbnail).height = size as u32;
            }
        } else if size < 0 && custom_width > 0 && custom_height > 0 {
            // SAFETY: as above.
            unsafe {
                (*thumbnail).width = custom_width as u32;
                (*thumbnail).height = custom_height as u32;
            }
        }
    }

    let data_arr: *mut ValueArray = find_array(node, sp::ImageData.as_ptr(), b'c');
    if !data_arr.is_null() {
        // SAFETY: `data_arr` is non-null (checked) and points at the node's own
        // array descriptor; `thumbnail` is the caller's writable slot.
        unsafe {
            (*thumbnail).data.data = (*data_arr).data as *const u8;
            (*thumbnail).data.size = (*data_arr).size;
        }
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
    // SAFETY: `node` is a parse-tree NodeView and the destination is uc's own
    // scene metadata `props` slot, reached through its element views.
    unsafe {
        read_properties(
            uc,
            node,
            uc.scene_view().metadata_view().scene_props_mut_ptr(),
        )?;
    }

    let thumbnail = find_child(node, sp::Thumbnail.as_ptr());
    if let Some(thumbnail) = thumbnail {
        // SAFETY: `thumbnail` is a child NodeView of `node`; the destination is
        // uc's own scene metadata `thumbnail` slot, reached through its views.
        unsafe {
            read_thumbnail(
                uc,
                thumbnail,
                uc.scene_view().metadata_view().thumbnail_mut_ptr(),
            )?;
        }
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
            // SAFETY: `child` is a parse-tree NodeView; the out-param is uc's
            // own metadata `creator` string slot, reached through its views.
            ufbxi_ignore!(unsafe {
                get_val1(
                    child,
                    b"S\0".as_ptr(),
                    uc.scene_view().metadata_view().creator_mut_ptr() as *mut c_void,
                )
            });
        }

        if uc.version() < 6000 && child.name() == sp::FBXVersion.as_ptr() {
            let mut version: i32 = 0;
            // SAFETY: `child` is a parse-tree NodeView; `version` is an
            // unaliased local matching the `I` format's `int32_t` out-param.
            if unsafe {
                get_val1(
                    child,
                    b"I\0".as_ptr(),
                    &mut version as *mut i32 as *mut c_void,
                )
            } {
                if version > 0 && version < 6000 && (version as u32) > uc.version() {
                    uc.set_version(version as u32);
                }
            }
        }

        if child.name() == sp::FBXHeaderVersion.as_ptr() {
            // SAFETY: `child` is a parse-tree NodeView; `header_version` is an
            // unaliased local matching the `I` format's `int32_t` out-param.
            ufbxi_ignore!(unsafe {
                get_val1(
                    child,
                    b"I\0".as_ptr(),
                    &mut header_version as *mut i32 as *mut c_void,
                )
            });
        }

        if child.name() == sp::OtherFlags.as_ptr() {
            // SAFETY: `child` is a parse-tree NodeView; `tc_definition` is an
            // unaliased local matching the `I` format's `int32_t` out-param.
            if unsafe {
                find_val1(
                    child,
                    sp::TCDefinition.as_ptr(),
                    b"I\0".as_ptr(),
                    &mut tc_definition as *mut i32 as *mut c_void,
                )
            } {
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
pub(crate) unsafe fn match_version_string(
    fmt: *const u8,
    str_: String,
    p_version: *mut u32,
) -> bool {
    let mut num_ix: usize = 0;
    let mut pos: usize = 0;
    let mut fmt = fmt;
    // SAFETY: `fmt` walks a NUL-terminated pattern string (fn contract), so
    // every byte up to and including the terminator is readable.
    while unsafe { *fmt } != 0 {
        // C-parity: `char c = *fmt++;` / `char s = str.data[pos];` are `const
        // char *` dereferences — signed bytes on the oracle targets
        // (PORTING.md `char` value row).
        // SAFETY: as above — `fmt` addresses a non-NUL pattern byte.
        let c: i8 = unsafe { *(fmt as *const i8) };
        // SAFETY: the byte at `fmt` is not the terminator, so the advance stays
        // inside the pattern string.
        fmt = unsafe { fmt.add(1) };
        if c >= b'a' as i8 && c <= b'z' as i8 {
            if pos >= str_.length {
                return false;
            }
            // SAFETY: `str_` is the caller's string, so `data` spans `length`
            // readable bytes, and `pos < length` (checked just above).
            let s: i8 = unsafe { *(str_.data.add(pos) as *const i8) };
            if s != c && s as i32 + (b'a' as i32 - b'A' as i32) != c as i32 {
                return false;
            }
            pos += 1;
        } else if c == b' ' as i8 {
            while pos < str_.length {
                // SAFETY: `pos < str_.length` bounds the read inside the
                // caller's string.
                let s: i8 = unsafe { *(str_.data.add(pos) as *const i8) };
                if s != b' ' as i8 && s != b'\t' as i8 {
                    break;
                }
                pos += 1;
            }
        } else if c == b'-' as i8 {
            while pos < str_.length {
                // SAFETY: `pos < str_.length` bounds the read inside the
                // caller's string.
                let s: i8 = unsafe { *(str_.data.add(pos) as *const i8) };
                if s == b'-' as i8 {
                    break;
                }
                pos += 1;
            }
            if pos >= str_.length {
                return false;
            }
            pos += 1;
        } else if c == b'/' as i8
            || c == b'.' as i8
            || c == b'(' as i8
            || c == b')' as i8
            || c == b'_' as i8
        {
            if pos >= str_.length {
                return false;
            }
            // SAFETY: `pos < str_.length` (checked just above) bounds the read
            // inside the caller's string.
            if unsafe { *(str_.data.add(pos) as *const i8) } != c {
                return false;
            }
            pos += 1;
        } else if c == b'?' as i8 {
            let mut num: u32 = 0;
            let mut len: usize = 0;
            while pos < str_.length {
                // SAFETY: `pos < str_.length` bounds the read inside the
                // caller's string.
                let s: i8 = unsafe { *(str_.data.add(pos) as *const i8) };
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
            // SAFETY: `num_ix` counts the `?` markers consumed from `fmt` so
            // far, and `p_version` points at storage for at least as many
            // `u32`s as `fmt` holds `?` markers (fn contract).
            unsafe { *p_version.add(num_ix) = num };
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
    let creator: String = uc.scene_view().metadata_view().creator();
    let mut version: [u32; 3] = [0; 3];
    // SAFETY (this whole chain): every `fmt` is a NUL-terminated byte-string
    // literal, `creator` is uc's own pooled metadata string (data/length pair
    // maintained by the string pool), and `version` is an unaliased local
    // 3-element array — each pattern holds at most three `?` numbers, so the
    // `p_version` writes stay inside it.
    unsafe {
        if match_version_string(b"blender-- ?.?.?\0".as_ptr(), creator, version.as_mut_ptr()) {
            uc.set_exporter(Exporter::BlenderBinary);
            uc.set_exporter_version(pack_version(version[0], version[1], version[2]));
        } else if match_version_string(b"blender- ?.?\0".as_ptr(), creator, version.as_mut_ptr()) {
            uc.set_exporter(Exporter::BlenderBinary);
            uc.set_exporter_version(pack_version(version[0], version[1], 0));
        } else if match_version_string(
            b"blender version ?.?\0".as_ptr(),
            creator,
            version.as_mut_ptr(),
        ) {
            uc.set_exporter(Exporter::BlenderAscii);
            uc.set_exporter_version(pack_version(version[0], version[1], 0));
        } else if match_version_string(
            b"fbx sdk/fbx plugins version ?.?\0".as_ptr(),
            creator,
            version.as_mut_ptr(),
        ) {
            uc.set_exporter(Exporter::FbxSdk);
            uc.set_exporter_version(pack_version(version[0], version[1], 0));
        } else if match_version_string(
            b"fbx sdk/fbx plugins build ?\0".as_ptr(),
            creator,
            version.as_mut_ptr(),
        ) {
            uc.set_exporter(Exporter::FbxSdk);
            uc.set_exporter_version(pack_version(
                version[0] / 10000u32,
                version[0] / 100u32 % 100u32,
                version[0] % 100u32,
            ));
        } else if match_version_string(
            b"motionbuilder version ?.?\0".as_ptr(),
            creator,
            version.as_mut_ptr(),
        ) {
            uc.set_exporter(Exporter::MotionBuilder);
            uc.set_exporter_version(pack_version(version[0], version[1], 0));
        } else if match_version_string(
            b"motionbuilder/mocap/online version ?.?\0".as_ptr(),
            creator,
            version.as_mut_ptr(),
        ) {
            uc.set_exporter(Exporter::MotionBuilder);
            uc.set_exporter_version(pack_version(version[0], version[1], 0));
        } else if match_version_string(b"ufbx_write\0".as_ptr(), creator, version.as_mut_ptr()) {
            uc.set_exporter(Exporter::UfbxWrite);
            uc.set_exporter_version(pack_version(0, 0, 1));
        }
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
            // Post-7000: Try to find the first document node and root ID.
            // TODO: Multiple documents / roots?
            // SAFETY: `child` is a parse-tree NodeView; the out-param is uc's
            // own `root_id` field, whose `u64` matches the `L` format.
            if unsafe {
                find_val1(
                    child,
                    sp::RootNode.as_ptr(),
                    b"L\0".as_ptr(),
                    uc.root_id_mut_ptr() as *mut c_void,
                )
            } {
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
        // SAFETY: `tmpl` is the fresh non-null push result checked just above,
        // so its `type` field is a valid out-param for the `C` format; `object`
        // is a parse-tree NodeView.
        ufbxi_check!(
            uc,
            unsafe {
                get_val1(
                    object,
                    b"C\0".as_ptr(),
                    &mut (*tmpl).type_ as *mut *const u8 as *mut c_void,
                )
            },
            "ufbxi_get_val1(object, \"C\", (char**)&tmpl->type)"
        );

        // Pre-7000 FBX versions don't have property templates, they just have
        // the object counts by themselves.
        let props = find_child(object, sp::PropertyTemplate.as_ptr());
        if let Some(props) = props {
            // SAFETY: `props` is a child NodeView of `object`; `tmpl` is the
            // fresh push result above, so `&mut (*tmpl).sub_type` is a valid
            // `ufbx_string` out-param for the `S` format.
            ufbxi_check!(
                uc,
                unsafe {
                    get_val1(
                        props,
                        b"S\0".as_ptr(),
                        &mut (*tmpl).sub_type as *mut String as *mut c_void,
                    )
                },
                "ufbxi_get_val1(props, \"S\", &tmpl->sub_type)"
            );

            // Remove the "Fbx" prefix from sub-types, remember to re-intern!
            // SAFETY: `tmpl` is the fresh push result above; `sub_type` was
            // just filled by the `S` read, so it is a pooled NUL-terminated
            // string of `length` bytes and the compares/advance below stay
            // inside it (each is guarded by a length check). The re-intern
            // hands uc's own string pool the same field.
            unsafe {
                if (*tmpl).sub_type.length > 3
                    && strncmp((*tmpl).sub_type.data, b"Fbx\0".as_ptr(), 3) == 0
                {
                    (*tmpl).sub_type.data = (*tmpl).sub_type.data.add(3);
                    (*tmpl).sub_type.length -= 3;

                    // HACK: LOD groups use LODGroup for Template, LodGroup for Object?
                    if (*tmpl).sub_type.length == 8
                        && memcmp((*tmpl).sub_type.data, b"LODGroup\0".as_ptr(), 8) == 0
                    {
                        (*tmpl).sub_type.data = LOD_GROUP.as_ptr();
                    }

                    push_string_place_str(uc.string_pool_mut_ptr(), &mut (*tmpl).sub_type, false)?;
                }

                read_properties(uc, props, &mut (*tmpl).props)?;
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
#[must_use]
pub(crate) unsafe fn find_template(
    uc: &Context,
    name: *const u8,
    sub_type: *const u8,
) -> *mut Props {
    // TODO: Binary search
    // C: `ufbxi_for(ufbxi_template, tmpl, uc->templates, uc->num_templates)`
    let mut tmpl = uc.templates();
    let tmpl_end = add_ptr(tmpl, uc.num_templates());
    while tmpl != tmpl_end {
        // SAFETY: `tmpl` walks `uc.templates()..+ uc.num_templates()`, uc's own
        // result-buffer run of live `ufbxi_template` values.
        if unsafe { (*tmpl).type_ } == name {
            // Check that sub_type matches unless the type is Material, Model, AnimationStack, AnimationLayer.
            // Those match to all sub-types.
            // SAFETY: as above — `tmpl` addresses a live template.
            if unsafe {
                (*tmpl).type_ != sp::Material.as_ptr()
                    && (*tmpl).type_ != sp::Model.as_ptr()
                    && (*tmpl).type_ != sp::AnimationStack.as_ptr()
                    && (*tmpl).type_ != sp::AnimationLayer.as_ptr()
            } {
                // SAFETY: as above.
                if unsafe { (*tmpl).sub_type.data } != sub_type {
                    return core::ptr::null_mut();
                }
            }

            // SAFETY: as above.
            if unsafe { (*tmpl).props.props.count } > 0 {
                // SAFETY: as above; the projection borrows the template's own
                // `props` field, which lives as long as uc's result buffer.
                return unsafe { &raw mut (*tmpl).props };
            } else {
                return core::ptr::null_mut();
            }
        }
        // SAFETY: `tmpl` is before `tmpl_end`, so the advance lands at most one
        // past the template run's end.
        tmpl = unsafe { tmpl.add(1) };
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
    // SAFETY: the key is a local `PtrId` passed by pointer to uc's own
    // `ptr_fbx_id_map`, whose entries are `PtrFbxIdEntry` keyed by exactly that
    // type; the lookup either returns one of its entries or null.
    let mut entry: *mut PtrFbxIdEntry = unsafe {
        map_find(
            uc.ptr_fbx_id_map_mut_ptr(),
            hash,
            &ptr_id as *const PtrId as *const c_void,
        )
    };

    if entry.is_null() {
        // SAFETY: same map and key type as the lookup above; the fresh
        // non-null insert result is initialized right here before use.
        entry = unsafe {
            map_insert(
                uc.ptr_fbx_id_map_mut_ptr(),
                hash,
                &ptr_id as *const PtrId as *const c_void,
            )
        };
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
pub(crate) unsafe fn validate_fbx_id(uc: &Context, p_fbx_id: *mut u64) -> Result<(), Fail> {
    // SAFETY: `p_fbx_id` is the caller's live, initialized `uint64_t` slot (fn
    // contract).
    let mut fbx_id: u64 = unsafe { *p_fbx_id };
    if fbx_id >= POINTER_ID_START {
        fbx_id = synthetic_id_from_ptr_id(uc, 0, fbx_id);
        ufbxi_check!(uc, fbx_id != 0, "fbx_id");
        // SAFETY: as above — `p_fbx_id` is the caller's writable slot.
        unsafe { *p_fbx_id = fbx_id };
    }
    Ok(())
}

// ufbx.c:12271-12305 `ufbxi_split_type_and_name`
#[inline(never)]
pub(crate) unsafe fn split_type_and_name(
    uc: &Context,
    type_and_name: String,
    type_: *mut String,
    name: *mut String,
) -> Result<(), Fail> {
    // Name and type are packed in a single property as Type::Name (in ASCII)
    // or Name\x00\x01Type (in binary)
    let sep: *const u8 = if uc.from_ascii() {
        b"::\0".as_ptr()
    } else {
        b"\x00\x01\0".as_ptr()
    };
    let mut type_end: usize = 2;
    while type_end <= type_and_name.length {
        // SAFETY: `type_end <= type_and_name.length`, so `type_end - 2` is at
        // most `length - 2` and the two-byte window read below stays inside the
        // caller's `type_and_name` string.
        let ch: *const u8 = unsafe { type_and_name.data.add(type_end - 2) };
        // SAFETY: `ch[0..2]` sits inside `type_and_name` as argued above, and
        // `sep` is a two-byte NUL-terminated literal.
        if unsafe { *ch.add(0) } == unsafe { *sep.add(0) }
            && unsafe { *ch.add(1) } == unsafe { *sep.add(1) }
        {
            break;
        }
        type_end += 1;
    }

    // ???: ASCII and binary store type and name in different order
    if type_end <= type_and_name.length {
        if uc.from_ascii() {
            // SAFETY: `name` and `type_` are the caller's writable `ufbx_string`
            // out-params; `type_end <= type_and_name.length`, so the split
            // pointer lands at most one past the string's end.
            unsafe {
                (*name).data = type_and_name.data.add(type_end);
                (*name).length = type_and_name.length - type_end;
                (*type_).data = type_and_name.data;
                (*type_).length = type_end - 2;
            }
        } else {
            // SAFETY: as above, with the type/name halves swapped.
            unsafe {
                (*name).data = type_and_name.data;
                (*name).length = type_end - 2;
                (*type_).data = type_and_name.data.add(type_end);
                (*type_).length = type_and_name.length - type_end;
            }
        }
    } else {
        // SAFETY: `name` and `type_` are the caller's writable `ufbx_string`
        // out-params.
        unsafe {
            *name = type_and_name;
            (*type_).data = EMPTY_CHAR.as_ptr();
            (*type_).length = 0;
        }
    }

    // SAFETY: `type_`/`name` are the caller's `ufbx_string` slots, each holding
    // a data/length pair written above, and the pool is uc's own string pool —
    // `push_string_place_str`'s contract.
    unsafe {
        push_string_place_str(uc.string_pool_mut_ptr(), type_, false)?;
        push_string_place_str(uc.string_pool_mut_ptr(), name, false)?;
    }

    Ok(())
}

// ufbx.c:12307-12323 `ufbxi_insert_fbx_id`
#[inline(never)]
pub(crate) fn insert_fbx_id(uc: &Context, fbx_id: u64, element_id: u32) -> Result<(), Fail> {
    let hash = hash64(fbx_id);
    // SAFETY: the key is a local `u64` passed by pointer to uc's own
    // `fbx_id_map`, whose entries are `FbxIdEntry` keyed by that `u64`.
    let mut entry: *mut FbxIdEntry = unsafe {
        map_find(
            uc.fbx_id_map_mut_ptr(),
            hash,
            &fbx_id as *const u64 as *const c_void,
        )
    };

    if entry.is_null() {
        // SAFETY: same map and key as the lookup above.
        entry = unsafe {
            map_insert(
                uc.fbx_id_map_mut_ptr(),
                hash,
                &fbx_id as *const u64 as *const c_void,
            )
        };
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
    // SAFETY: `fbx_id_map_mut_ptr()` is a valid map by construction; `hash` and
    // the key pointer are valid local values.
    unsafe {
        map_find(
            uc.fbx_id_map_mut_ptr(),
            hash,
            &fbx_id as *const u64 as *const c_void,
        )
    }
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
    // SAFETY: the key is a local `u64` passed by pointer to uc's own
    // `fbx_attr_map`, whose entries are `FbxAttrEntry` keyed by that `u64`.
    let mut entry: *mut FbxAttrEntry = unsafe {
        map_find(
            uc.fbx_attr_map_mut_ptr(),
            hash,
            &fbx_id as *const u64 as *const c_void,
        )
    };
    // TODO: Strict / warn about duplicate objects

    if entry.is_null() {
        // SAFETY: same map and key as the lookup above.
        entry = unsafe {
            map_insert(
                uc.fbx_attr_map_mut_ptr(),
                hash,
                &fbx_id as *const u64 as *const c_void,
            )
        };
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
        // SAFETY: `ptr` is non-null (checked) and points to a live `T` the
        // caller keeps alive for the returned `Ref`'s use (fn contract).
        Some(unsafe { Ref::from_ptr(ptr) })
    }
}

// Inverse of `opt_ref`: reads an `Option<Ref<T>>` field back as the bare
// (possibly NULL) C pointer the field is at the ABI level.
#[inline(always)]
pub(crate) unsafe fn opt_ptr<T>(p: *const Option<Ref<T>>) -> *mut T {
    // SAFETY: `p` addresses a live `Option<Ref<T>>` field (fn contract), which
    // is niche-packed to a bare pointer, so reading it as `*mut T` reinterprets
    // the same bytes in place.
    unsafe { *(p as *const *mut T) }
}

// Same for a non-optional `Ref<T>` field (C: a plain `ufbx_element*`).
#[inline(always)]
pub(crate) unsafe fn ref_ptr<T>(p: *const Ref<T>) -> *mut T {
    // SAFETY: `p` addresses a live `Ref<T>` field (fn contract), which is
    // `repr(transparent)` over `NonNull<T>`, so reading it as `*mut T`
    // reinterprets the same bytes in place.
    unsafe { *(p as *const *mut T) }
}

// ufbx.c:12352-12382 `ufbxi_push_element_size`
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
    unsafe { core::ptr::write(&mut (*elem).props, core::ptr::read(&(*info).props)) };
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
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_synthetic_element_size(
    uc: &Context,
    p_fbx_id: *mut u64,
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
        // contract), which is what `strlen` requires; `elem` is the fresh push
        // result.
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

    // SAFETY: `p_fbx_id` is the caller's writable `uint64_t` slot (fn contract).
    unsafe { *p_fbx_id = push_synthetic_id(uc) };

    ufbxi_check_return!(
        uc,
        // SAFETY: the buffer is uc's own `tmp_element_fbx_ids` and `p_fbx_id`
        // is the caller's slot, holding the one `u64` copied from it.
        !unsafe { push_copy_fast::<u64>(uc.tmp_element_fbx_ids_mut_ptr(), 1, p_fbx_id) }.is_null(),
        core::ptr::null_mut(),
        "((uint64_t*)ufbxi_push_size_copy_fast((&uc->tmp_element_fbx_ids), sizeof(uint64_t), (1), (p_fbx_id)))"
    );
    ufbxi_check_return!(
        uc,
        // SAFETY: `p_fbx_id` is the caller's slot, written just above.
        insert_fbx_id(uc, unsafe { *p_fbx_id }, element_id).is_ok(),
        core::ptr::null_mut(),
        "ufbxi_insert_fbx_id(uc, *p_fbx_id, element_id)"
    );

    elem
}

// ufbx.c:12416 `#define ufbxi_push_element(uc, info, type_name, type_enum)`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn push_element<T>(
    uc: &Context,
    info: *mut ElementInfo,
    type_enum: ElementType,
) -> *mut T {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `T` is the
    // element struct whose `size_of` is passed as the element size — the size
    // and type must agree, which is `push_element_size`'s contract.
    ufbxi_maybe_null!(unsafe { push_element_size(uc, info, size_of::<T>(), type_enum) } as *mut T)
}

// ufbx.c:12417 `#define ufbxi_push_synthetic_element(uc, p_fbx_id, node, name, type_name, type_enum)`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn push_synthetic_element<T>(
    uc: &Context,
    p_fbx_id: *mut u64,
    node: Option<&NodeView>,
    name: *const u8,
    type_enum: ElementType,
) -> *mut T {
    // SAFETY: `p_fbx_id` is the caller's writable `uint64_t` slot, `name` is
    // null or NUL-terminated, and `T` is the element struct whose `size_of` is
    // passed as the element size — `push_synthetic_element_size`'s contract.
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

// ufbx.c:12451-12463 `ufbxi_init_synthetic_int_prop`
#[inline(never)]
pub(crate) unsafe fn init_synthetic_int_prop(
    dst: *mut Prop,
    name: *const u8,
    value: i64,
    type_: PropType,
) {
    // SAFETY: `dst` is the caller's writable `ufbx_prop` slot and `name` is a
    // NUL-terminated interned string (fn contract) — `strlen`'s contract.
    unsafe {
        (*dst).type_ = type_;
        (*dst).name.data = name;
        (*dst).name.length = strlen(name);
    }
    // C-parity: `dst->value_real` is the `ufbx_prop` value union's first real
    // (`value_vec4.x` in the generated struct).
    // SAFETY: `dst` is the caller's writable `ufbx_prop` slot.
    unsafe {
        (*dst).value_vec4.x = value as Real;
        (*dst).flags = PropFlags::from_raw(
            PropFlags::SYNTHETIC.raw() | PropFlags::VALUE_REAL.raw() | PropFlags::VALUE_INT.raw(),
        );
        (*dst).value_int = value;
        (*dst).value_str.data = EMPTY_CHAR.as_ptr();
    }

    // SAFETY: `dst.name.length` was written just above.
    ufbxi_dev_assert!(unsafe { (*dst).name.length } >= 4);
    // SAFETY: every caller passes a `ufbxi_*` interned property name of at
    // least 4 characters (the dev assert above guards that contract), so the
    // 4-byte key read stays inside `name`.
    unsafe { (*dst)._internal_key = get_name_key(crate::prelude::slice_from_ptr(name, 4)) };
}

// ufbx.c:12465-12477 `ufbxi_init_synthetic_real_prop`
#[inline(never)]
pub(crate) unsafe fn init_synthetic_real_prop(
    dst: *mut Prop,
    name: *const u8,
    value: Real,
    type_: PropType,
) {
    // SAFETY: `dst` is the caller's writable `ufbx_prop` slot and `name` is a
    // NUL-terminated interned string (fn contract) — `strlen`'s contract.
    unsafe {
        (*dst).type_ = type_;
        (*dst).name.data = name;
        (*dst).name.length = strlen(name);
    }
    // C-parity: bare `(int64_t)` cast on a float operand — `as` (saturating),
    // per PORTING.md "Integer semantics".
    // SAFETY: `dst` is the caller's writable `ufbx_prop` slot.
    unsafe {
        (*dst).value_vec4.x = value;
        (*dst).flags =
            PropFlags::from_raw(PropFlags::SYNTHETIC.raw() | PropFlags::VALUE_REAL.raw());
        (*dst).value_int = value as i64;
        (*dst).value_str.data = EMPTY_CHAR.as_ptr();
    }

    // SAFETY: `dst.name.length` was written just above.
    ufbxi_dev_assert!(unsafe { (*dst).name.length } >= 4);
    // SAFETY: every caller passes a `ufbxi_*` interned property name of at
    // least 4 characters (the dev assert above guards that contract), so the
    // 4-byte key read stays inside `name`.
    unsafe { (*dst)._internal_key = get_name_key(crate::prelude::slice_from_ptr(name, 4)) };
}

// ufbx.c:12479-12491 `ufbxi_init_synthetic_vec3_prop`
#[inline(never)]
pub(crate) unsafe fn init_synthetic_vec3_prop(
    dst: *mut Prop,
    name: *const u8,
    value: *const Vec3,
    type_: PropType,
) {
    // SAFETY: `dst` is the caller's writable `ufbx_prop` slot and `name` is a
    // NUL-terminated interned string (fn contract) — `strlen`'s contract.
    unsafe {
        (*dst).type_ = type_;
        (*dst).name.data = name;
        (*dst).name.length = strlen(name);
    }
    // C: `dst->value_vec3 = *value;` writes only x/y/z of the value union.
    // SAFETY: `value` points at a live `ufbx_vec3` (fn contract); `Vec3` is the
    // 3-real prefix of the `Vec4` value union arm in `dst`'s writable slot, so
    // the projected write stays inside it.
    unsafe { *(&mut (*dst).value_vec4 as *mut Vec4 as *mut Vec3) = *value };
    // C: `ufbxi_f64_to_i64(dst->value_real)` — `ufbx_real` argument promoted to
    // the `double` parameter.
    // SAFETY: `dst` is the caller's writable `ufbx_prop` slot, whose
    // `value_vec4.x` was written by the vec3 store above.
    unsafe {
        (*dst).flags =
            PropFlags::from_raw(PropFlags::SYNTHETIC.raw() | PropFlags::VALUE_VEC3.raw());
        (*dst).value_int = f64_to_i64(as_f64!((*dst).value_vec4.x));
        (*dst).value_str.data = EMPTY_CHAR.as_ptr();
    }

    // SAFETY: `dst.name.length` was written just above.
    ufbxi_dev_assert!(unsafe { (*dst).name.length } >= 4);
    // SAFETY: every caller passes a `ufbxi_*` interned property name of at
    // least 4 characters (the dev assert above guards that contract), so the
    // 4-byte key read stays inside `name`.
    unsafe { (*dst)._internal_key = get_name_key(crate::prelude::slice_from_ptr(name, 4)) };
}

// ufbx.c:12493-12505 `ufbxi_set_own_prop_vec3_uniform`
#[inline(never)]
pub(crate) unsafe fn set_own_prop_vec3_uniform(props: *mut Props, name: *const u8, value: Real) {
    // C: `ufbx_props local_props = *props;` — struct memcpy; `Props` is not
    // `Copy` but has no drop glue.
    // SAFETY: `props` addresses the caller's live `ufbx_props` (fn contract);
    // `Props` has no drop glue, so the bitwise read duplicates it without
    // double-free (the copy is forgotten below).
    let mut local_props: Props = unsafe { core::ptr::read(props) };
    local_props.defaults = None;
    // SAFETY: `&raw mut local_props` addresses this frame's live, fully
    // initialized `Props`, and `name` is a NUL-terminated interned property
    // name — `find_prop`'s contract.
    let prop: Option<&PropView> =
        unsafe { api_find_prop(PropsView::from_ptr(&raw mut local_props), name) };
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
pub(crate) unsafe fn setup_geometry_transform_helper(
    uc: &Context,
    node: *mut UfbxNode,
    node_fbx_id: u64,
) -> Result<(), Fail> {
    // SAFETY: `node` is the caller's live `ufbx_node` (fn contract), so the
    // projection addresses its own `element.props`, which `PropsView::from_ptr`
    // mints a view over.
    let node_props: &PropsView = unsafe { PropsView::from_ptr(&raw mut (*node).element.props) };
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
                &mut geo_fbx_id,
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
        // SAFETY: both projections address live `dom_node` fields — `node` is
        // the caller's node and `geo_node` the fresh element above; the field
        // has no drop glue, so the bitwise read duplicates it safely.
        unsafe {
            (*geo_node).element.dom_node = core::ptr::read(&(*node).element.dom_node);
        }

        let props: *mut Prop = uc.result_view().push_zero::<Prop>(3);
        ufbxi_check!(uc, !props.is_null(), "props");
        // SAFETY: `props` is the fresh non-null 3-element zeroed run checked
        // above, so indices 0..3 are live `ufbx_prop` slots; each name is an
        // interned `ufbxi_*` string and each value a live local `Vec3`.
        unsafe {
            init_synthetic_vec3_prop(
                props.add(0),
                sp::Lcl_Rotation.as_ptr(),
                &geo_rotation,
                PropType::Rotation,
            );
            init_synthetic_vec3_prop(
                props.add(1),
                sp::Lcl_Scaling.as_ptr(),
                &geo_scaling,
                PropType::Scaling,
            );
            init_synthetic_vec3_prop(
                props.add(2),
                sp::Lcl_Translation.as_ptr(),
                &geo_translation,
                PropType::Translation,
            );
        }

        // SAFETY: `geo_node` is the fresh non-null element above.
        unsafe {
            (*geo_node).element.props.props.data = props;
            (*geo_node).element.props.props.count = 3;
        }

        // SAFETY: `node` is the caller's live `ufbx_node`.
        unsafe { (*node).has_geometry_transform = true };
        // SAFETY: `geo_node` is the fresh non-null element above.
        unsafe { (*geo_node).is_geometry_transform_helper = true };

        connect_oo(uc, geo_fbx_id, node_fbx_id)?;
        uc.set_has_geometry_transform_nodes(true);

        // SAFETY: `node` is the caller's live `ufbx_node`.
        let extra: *mut NodeExtra = push_element_extra(uc, unsafe { (*node).element.element_id });
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
pub(crate) unsafe fn setup_scale_helper(
    uc: &Context,
    node: *mut UfbxNode,
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
            &mut scale_fbx_id,
            None,
            uc.opts_view().scale_helper_name_view().data(),
            ElementType::Node,
        )
    };
    ufbxi_check!(uc, !scale_node.is_null(), "scale_node");
    ufbxi_check!(
        uc,
        // SAFETY: `scale_node` is the fresh non-null element checked above, so
        // the borrow addresses its own `element.element_id`.
        !unsafe {
            uc.tmp_node_ids_view()
                .push_copy_ref(&(*scale_node).element.element_id)
        }
        .is_null(),
        "((uint32_t*)ufbxi_push_size_copy((&uc->tmp_node_ids), sizeof(uint32_t), (1), (&scale_node->element.element_id)))"
    );
    // C: `scale_node->element.dom_node = node->element.dom_node;`
    // SAFETY: both projections address live `dom_node` fields — `node` is the
    // caller's node and `scale_node` the fresh element above; the field has no
    // drop glue, so the bitwise read duplicates it safely.
    unsafe {
        (*scale_node).element.dom_node = core::ptr::read(&(*node).element.dom_node);
    }

    // SAFETY: `node` is the caller's live `ufbx_node`; `scale_node` is the
    // fresh non-null element above, which outlives this scene's nodes.
    unsafe { (*node).scale_helper = Some(Ref::from_ptr(scale_node)) };
    // SAFETY: `scale_node` is the fresh non-null element above.
    unsafe { (*scale_node).is_scale_helper = true };

    connect_oo(uc, scale_fbx_id, node_fbx_id)?;
    uc.set_has_scale_helper_nodes(true);

    // SAFETY: `node` is the caller's live `ufbx_node`.
    let extra: *mut NodeExtra = push_element_extra(uc, unsafe { (*node).element.element_id });
    ufbxi_check!(uc, !extra.is_null(), "extra");
    // SAFETY: `extra` is the fresh non-null extra-data slot checked above and
    // `scale_node` the fresh element above.
    unsafe { (*extra).scale_helper_id = (*scale_node).element.element_id };

    let max_props: usize = SCALE_HELPER_PROPS.len();
    let helper_props: *mut Prop = uc.result_view().push::<Prop>(max_props);
    ufbxi_check!(uc, !helper_props.is_null(), "helper_props");

    let mut num_props: usize = 0;
    // C: `ufbx_props props_copy = node->props;` — struct memcpy.
    // SAFETY: `node` is the caller's live `ufbx_node`, so the projection
    // addresses its own `element.props`; `Props` has no drop glue, so the
    // bitwise read duplicates it without double-free (forgotten below).
    let mut props_copy: Props = unsafe { core::ptr::read(&(*node).element.props) };
    props_copy.defaults = None;
    let mut i: usize = 0;
    while i < max_props {
        let hp: *const ScaleHelperProp = &SCALE_HELPER_PROPS[i];
        // SAFETY: `&raw mut props_copy` addresses this frame's live, fully
        // initialized `Props`; `hp` points into the `SCALE_HELPER_PROPS`
        // static, whose `name` is an interned `ufbxi_*` string.
        let src_prop: *mut Prop =
            match unsafe { find_prop(PropsView::from_ptr(&raw mut props_copy), (*hp).name) } {
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
        unsafe { *(&mut (*src_prop).value_vec4 as *mut Vec4 as *mut Vec3) = (*hp).default_value };
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

    // SAFETY: `scale_node` is the fresh non-null element above; `helper_props`
    // is the result-buffer run whose first `num_props` entries were filled in.
    unsafe {
        (*scale_node).element.props.props.data = helper_props;
        (*scale_node).element.props.props.count = num_props;
    }

    Ok(())
}

// ufbx.c:12601-12627 `ufbxi_read_model`
#[inline(never)]
pub(crate) unsafe fn read_model(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    ufbxi_ignore!(node);
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `UfbxNode`
    // is the element struct for `ElementType::Node`.
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
#[inline(never)]
pub(crate) unsafe fn read_element(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
    size: usize,
    type_: ElementType,
) -> Result<(), Fail> {
    ufbxi_ignore!(node);
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `size` is
    // the size of the element struct for `type_` — the caller's contract, which
    // `push_element_size` inherits.
    let elem: *mut Element = unsafe { push_element_size(uc, info, size, type_) };
    ufbxi_check!(uc, !elem.is_null(), "elem");
    Ok(())
}

// ufbx.c:12637-12653 `ufbxi_read_unknown`
#[inline(never)]
pub(crate) unsafe fn read_unknown(
    uc: &Context,
    node: &NodeView,
    element: *mut ElementInfo,
    type_: String,
    sub_type: String,
    node_name: *const u8,
) -> Result<(), Fail> {
    ufbxi_ignore!(node);
    // SAFETY: `element` is the caller's live `ufbxi_element_info` and `Unknown`
    // is the element struct for `ElementType::Unknown`.
    let unknown: *mut Unknown =
        unsafe { push_element::<Unknown>(uc, element, ElementType::Unknown) };
    ufbxi_check!(uc, !unknown.is_null(), "unknown");
    // SAFETY: `unknown` is the fresh non-null element checked above; `node_name`
    // is the caller's NUL-terminated node name — `strlen`'s contract.
    unsafe {
        (*unknown).type_ = type_;
        (*unknown).sub_type = sub_type;
        (*unknown).super_type.data = node_name;
        (*unknown).super_type.length = strlen(node_name);
    }

    // `type`, `sub_type` and `node_name` are raw strings so they may need to be sanitized.
    // SAFETY: each argument is a field of the fresh element above, holding the
    // data/length pair written just now, and the pool is uc's own string pool.
    unsafe {
        push_string_place_str(uc.string_pool_mut_ptr(), &mut (*unknown).type_, false)?;
        push_string_place_str(uc.string_pool_mut_ptr(), &mut (*unknown).sub_type, false)?;
        push_string_place_str(uc.string_pool_mut_ptr(), &mut (*unknown).super_type, false)?;
    }

    Ok(())
}

// ufbx.c:12655-12658 `typedef struct { ufbx_vertex_vec3 elem; uint32_t index; } ufbxi_tangent_layer;`
#[repr(C)]
pub(crate) struct TangentLayer {
    pub elem: VertexVec3,
    pub index: u32,
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
pub(crate) unsafe fn fix_index(
    uc: &Context,
    p_dst: *mut u32,
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
            // SAFETY: `p_dst` is the caller's writable `uint32_t` index slot
            // (fn contract).
            unsafe { *p_dst = (one_past_max_val as u32).wrapping_sub(1) };
            ufbxi_check!(
                uc,
                ufbxi_warnf!(uc, WarningType::IndexClamped, "Clamped index").is_ok(),
                "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_INDEX_CLAMPED, ~0u, \"Clamped index\")"
            );
        }
        IndexErrorHandling::NoIndex => {
            // SAFETY: `p_dst` is the caller's writable `uint32_t` index slot.
            unsafe { *p_dst = NO_INDEX };
        }
        IndexErrorHandling::AbortLoading => {
            // C-parity: `one_past_max_val` is a `size_t` passed through `%u`,
            // which reads an `unsigned int` — the low 32 bits on the oracle
            // targets. The `as u32` narrowing reproduces that exactly.
            // SAFETY: the error slot is uc's own `ufbx_error` and the format
            // string is a NUL-terminated literal whose two `%u` conversions are
            // matched by the two `u32` arguments — `fmt_err_info`'s contract.
            unsafe {
                ufbxi_fmt_err_info!(
                    uc.error_mut_ptr(),
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
            // SAFETY: `p_dst` is the caller's writable `uint32_t` index slot.
            unsafe { *p_dst = index };
        }
        // C `default:` — unreachable in Rust because the match above is
        // exhaustive over the enum, but kept for diff parity.
        #[allow(unreachable_patterns)]
        _ => {
            ufbxi_unreachable!("Unhandled index_error_handling");
            return Err(Fail);
        }
    }

    Ok(())
}

// ufbx.c:12692-12726 `ufbxi_check_indices`
#[inline(never)]
pub(crate) unsafe fn check_indices(
    uc: &Context,
    p_dst: *mut *mut u32,
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
                indices = unsafe { push_copy::<u32>(uc.result_mut_ptr(), num_indices, indices) };
                ufbxi_check!(uc, !indices.is_null(), "indices");
                owns_indices = true;
            }
            // SAFETY: `i < num_indices`, so `indices.add(i)` is a writable slot
            // of the owned index run — `fix_index`'s out-pointer contract.
            unsafe { fix_index(uc, indices.add(i), ix, num_elems) }?;
        }
    }

    // SAFETY: `p_dst` is the caller's writable `uint32_t *` out-pointer.
    unsafe { *p_dst = indices };

    Ok(())
}

// ufbx.c:12728-12731 `ufbx_static_assert(vertex_{real,vec2,vec3,vec4}_size, ...)`
const _: () = assert!(size_of::<VertexReal>() == size_of::<VertexAttrib>());
const _: () = assert!(size_of::<VertexVec2>() == size_of::<VertexAttrib>());
const _: () = assert!(size_of::<VertexVec3>() == size_of::<VertexAttrib>());
const _: () = assert!(size_of::<VertexVec4>() == size_of::<VertexAttrib>());

// ufbx.c:12733-12737 `ufbxi_warn_polygon_mapping`
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

// ufbx.c:12739-12908 `ufbxi_read_vertex_element`
#[inline(never)]
pub(crate) unsafe fn read_vertex_element(
    uc: &Context,
    mesh: &View<Mesh>,
    node: &NodeView,
    attrib: *mut VertexAttrib,
    data_name: *const u8,
    index_name: *const u8,
    w_name: *const u8,
    data_type: u8,
    num_components: usize,
) -> Result<(), Fail> {
    // SAFETY: `attrib` is the caller's writable `ufbx_vertex_attrib` (fn
    // contract) — an arena-owned `ufbx_vertex_attrib`-shaped slot (mesh field,
    // tmp-stack tangent layer, or result-arena UV/color set), so write-capable
    // provenance.
    let attrib: &View<VertexAttrib> = unsafe { View::<VertexAttrib>::from_ptr(attrib) };
    // SAFETY: `values_raw()` addresses the viewed attribute's own value list, so
    // the projection addresses its `data` slot.
    let p_dst_data: *mut *mut Real =
        unsafe { &raw mut (*attrib.values_raw()).data } as *mut *mut Real;

    let data: *mut ValueArray = find_array(node, data_name, data_type);
    let indices: *mut ValueArray = find_array(node, index_name, b'i');

    if !uc.opts_view().strict() {
        if data.is_null() {
            return Ok(());
        }
    }

    ufbxi_check!(uc, !data.is_null(), "data");
    ufbxi_check!(
        uc,
        // SAFETY: `data` is non-null (checked just above) and points at the
        // node's own array descriptor, live for as long as the parse tree.
        unsafe { (*data).size } % num_components == 0,
        "data->size % num_components == 0"
    );

    // SAFETY: as above — `data` is a live, non-null array descriptor.
    let num_elems: usize = unsafe { (*data).size } / num_components;

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
    // SAFETY: `indices_raw()` addresses the viewed attribute's own index list.
    unsafe {
        (*attrib.indices_raw()).count = mesh.num_indices();
    }

    // C: `const char *mapping = "";` — an anonymous empty literal, never
    // pointer-equal to any interned `ufbxi_*` name constant.
    let mut mapping: *const u8 = EMPTY_CHAR.as_ptr();
    // SAFETY: the child name is a NUL-terminated interned string and fmt `"C"`
    // pairs with the `*mut *const u8` out-pointer `&mut mapping`.
    ufbxi_ignore!(unsafe {
        find_val1(
            node,
            sp::MappingInformationType.as_ptr(),
            b"C\0".as_ptr(),
            &mut mapping as *mut *const u8 as *mut c_void,
        )
    });

    // SAFETY: `values_raw()` addresses the viewed attribute's own value list.
    unsafe { (*attrib.values_raw()).count = if num_elems != 0 { num_elems } else { 1 } };

    // Data array is always used as-is, if empty set the data to a global
    // zero buffer so invalid zero index can point to some valid data.
    // The zero data is offset by 4 elements to accommodate for invalid index (-1)
    if num_elems > 0 {
        // SAFETY: `p_dst_data` is the attribute's own `values.data` slot and
        // `data` is the live, non-null array descriptor checked above.
        unsafe { *p_dst_data = (*data).data as *mut Real };
    } else {
        // SAFETY: `ZERO_ELEMENT` is a static 8-`Real` buffer, so offsetting by
        // 4 stays inside it; `p_dst_data` is the attribute's own slot.
        unsafe { *p_dst_data = (ZERO_ELEMENT.0.get() as *mut Real).add(4) };
    }

    // HACK: Some old exporters seem to use ByPolygon to mean ByPolygonVertex,
    // it should be quite safe to remap this
    if mapping == sp::ByPolygon.as_ptr() {
        let num_indices: usize = if !indices.is_null() {
            // SAFETY: `indices` is non-null (checked) and points at the node's
            // own array descriptor.
            unsafe { (*indices).size }
        } else {
            num_elems
        };
        if num_indices == mesh.num_indices() {
            mapping = sp::ByPolygonVertex.as_ptr();
        }
    }

    if !indices.is_null() {
        // SAFETY: `indices` is non-null (checked) and points at the node's own
        // array descriptor, whose `data` spans `size` `int32_t` indices.
        let (num_indices, index_data): (usize, *mut u32) =
            unsafe { ((*indices).size, (*indices).data as *mut u32) };

        if mapping == sp::ByPolygonVertex.as_ptr() {
            // Indexed by polygon vertex: We can use the provided indices directly.
            // SAFETY: the out-pointer is the attribute's own `indices.data`
            // slot and `index_data` spans `num_indices` `u32`s (the array
            // descriptor's payload).
            unsafe {
                check_indices(
                    uc,
                    &raw mut (*attrib.indices_raw()).data as *mut *mut u32,
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

            let vert_ix: *mut u32 = mesh.vertex_indices().data as *mut u32;
            for i in 0..mesh.num_indices() {
                // SAFETY: `i < mesh.num_indices`, the length of the run at
                // `vert_ix`.
                let ix: u32 = unsafe { *vert_ix.add(i) };
                if (ix as usize) < num_indices {
                    // SAFETY: `i < mesh.num_indices` bounds the write inside
                    // the fresh `new_index_data` run, and `ix < num_indices`
                    // bounds the read inside `index_data`.
                    unsafe { *new_index_data.add(i) = *index_data.add(ix as usize) };
                } else {
                    // SAFETY: `i < mesh.num_indices`, so `new_index_data.add(i)`
                    // is a writable slot of the fresh run.
                    unsafe { fix_index(uc, new_index_data.add(i), ix, num_elems) }?;
                }
            }

            // SAFETY: the out-pointer is the attribute's own `indices.data`
            // slot and `new_index_data` is the fresh `num_indices`-long run
            // filled in by the loop above.
            unsafe {
                check_indices(
                    uc,
                    &raw mut (*attrib.indices_raw()).data as *mut *mut u32,
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
            for face_ix in 0..num_faces {
                // SAFETY: `face_ix < num_faces`, the length of the mesh's own
                // `faces` run.
                let face: Face = unsafe { *mesh.faces().data.add(face_ix) };
                let mut index: u32 = NO_INDEX;
                if face_ix < num_indices {
                    // SAFETY: `face_ix < num_indices`, the length of the run at
                    // `index_data`.
                    index = unsafe { *index_data.add(face_ix) };
                }
                if index as usize >= num_elems {
                    // SAFETY: `&mut index` is an unaliased local — a writable
                    // `uint32_t` slot for `fix_index`.
                    unsafe { fix_index(uc, &mut index, index, num_elems) }?;
                }
                for i in 0..face.num_indices as usize {
                    // SAFETY: every face's `index_begin + num_indices` stays
                    // within the mesh's `num_indices`, the length of the fresh
                    // `new_index_data` run.
                    unsafe { *new_index_data.add(face.index_begin as usize + i) = index };
                }
            }

            // SAFETY: `indices_raw()` addresses the viewed attribute's own
            // index list.
            unsafe { (*attrib.indices_raw()).data = new_index_data };
        } else if mapping == sp::AllSame.as_ptr() {
            // Indexed by all same: ??? This could be possibly used for making
            // holes with invalid indices, but that seems really fringe.
            // Just use the shared zero index buffer for this.
            uc.set_max_zero_indices(max_sz(uc.max_zero_indices(), mesh.num_indices()));
            // SAFETY: `indices_raw()` addresses the viewed attribute's own
            // index list; the sentinel is a static compared by address, never
            // dereferenced.
            unsafe {
                (*attrib.indices_raw()).data = SENTINEL_INDEX_ZERO.as_ptr();
            }
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
            unsafe { warn_polygon_mapping(uc, data_name, mapping) }?;
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
                // SAFETY: `indices_raw()` addresses the viewed attribute's
                // own index list; the sentinel is a static compared by address.
                unsafe { (*attrib.indices_raw()).data = SENTINEL_INDEX_CONSECUTIVE.as_ptr() };
            } else {
                let index_data: *mut u32 = uc.result_view().push::<u32>(mesh.num_indices());
                ufbxi_check!(uc, !index_data.is_null(), "index_data");
                for i in 0..mesh.num_indices() {
                    // SAFETY: `i < mesh.num_indices`, the length of the fresh
                    // run at `index_data`.
                    unsafe { *index_data.add(i) = i as u32 };
                }
                // SAFETY: the out-pointer is the attribute's own `indices.data`
                // slot and `index_data` is the fresh `num_indices`-long run
                // filled in by the loop above.
                unsafe {
                    check_indices(
                        uc,
                        &raw mut (*attrib.indices_raw()).data as *mut *mut u32,
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
            // SAFETY: the out-pointer is the attribute's own `indices.data`
            // slot; the mesh's `vertex_position.indices` spans its own
            // `num_indices` entries and stays owned by the mesh
            // (`owns_indices` is `false`).
            unsafe {
                check_indices(
                    uc,
                    &raw mut (*attrib.indices_raw()).data as *mut *mut u32,
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
            for face_ix in 0..num_faces {
                // SAFETY: `face_ix < num_faces`, the length of the mesh's own
                // `faces` run.
                let face: Face = unsafe { *mesh.faces().data.add(face_ix as usize) };
                for i in 0..face.num_indices as usize {
                    // SAFETY: every face's `index_begin + num_indices` stays
                    // within the mesh's `num_indices`, the length of the fresh
                    // `new_index_data` run.
                    unsafe { *new_index_data.add(face.index_begin as usize + i) = face_ix };
                }
            }

            // SAFETY: the out-pointer is the attribute's own `indices.data`
            // slot and `new_index_data` is the fresh `num_indices`-long run
            // filled in by the loop above.
            unsafe {
                check_indices(
                    uc,
                    &raw mut (*attrib.indices_raw()).data as *mut *mut u32,
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
            // SAFETY: `indices_raw()` addresses the viewed attribute's own
            // index list; the sentinel is a static compared by address, never
            // dereferenced.
            unsafe {
                (*attrib.indices_raw()).data = SENTINEL_INDEX_ZERO.as_ptr();
            }
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
            unsafe { warn_polygon_mapping(uc, data_name, mapping) }?;
            return Ok(());
        }
    }

    if uc.opts_view().retain_vertex_attrib_w() && !w_name.is_null() {
        let w_data: *mut ValueArray = find_array(node, w_name, b'r');
        if !w_data.is_null() {
            // SAFETY: `w_data` is non-null (checked) and points at the node's
            // own array descriptor.
            if unsafe { (*w_data).size } == num_elems {
                // SAFETY: as above; `values_w_raw()` addresses the viewed
                // attribute's own W list, and the `'r'` array's payload is a run
                // of `size` `ufbx_real` values.
                unsafe {
                    (*attrib.values_w_raw()).count = (*w_data).size;
                    (*attrib.values_w_raw()).data = (*w_data).data as *mut Real;
                }
            } else {
                ufbxi_check!(
                    uc,
                    ufbxi_warnf!(
                        uc,
                        WarningType::BadVertexWAttribute,
                        "Bad W array size %s=%zu, %s=%zu",
                        w_name,
                        // SAFETY: `w_data` is the non-null array descriptor
                        // checked above.
                        unsafe { (*w_data).size },
                        data_name,
                        num_elems,
                    )
                    .is_ok(),
                    "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_BAD_VERTEX_W_ATTRIBUTE, ~0u, \"Bad W array size %s=%zu, %s=%zu\", w_name, w_data->size, data_name, num_elems)"
                );
            }
        }
    }

    Ok(())
}

// ufbx.c:12910-12941 `ufbxi_read_truncated_array`
#[inline(never)]
pub(crate) unsafe fn read_truncated_array(
    uc: &Context,
    p_data: *mut c_void,
    p_count: *mut usize,
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

    // SAFETY: `p_count` is the caller's writable `size_t` out-slot (fn
    // contract).
    unsafe { *p_count = size };

    // SAFETY (this group): `arr` is non-null (the null case returned above) and points at
    // the node's own array descriptor, live for as long as the parse tree.
    let mut data: *mut c_void = unsafe { (*arr).data };
    if unsafe { (*arr).size } < size {
        ufbxi_check!(
            uc,
            ufbxi_warnf!(uc, WarningType::TruncatedArray, "Truncated array: %s", name).is_ok(),
            "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_TRUNCATED_ARRAY, ~0u, \"Truncated array: %s\", name)"
        );

        let elem_size: usize = array_type_size(fmt);
        // SAFETY: the buffer is uc's own result buffer; `elem_size` is the
        // element size `fmt` denotes.
        let new_data: *mut c_void = unsafe { push_size(uc.result_mut_ptr(), elem_size, size) };
        ufbxi_check!(uc, !new_data.is_null(), "new_data");
        // SAFETY: `arr.data` spans `arr.size * elem_size` readable bytes and
        // `new_data` is the fresh `size * elem_size`-byte run checked above,
        // with `arr.size < size`; the two are distinct objects.
        unsafe {
            core::ptr::copy_nonoverlapping(
                data as *const u8,
                new_data as *mut u8,
                (*arr).size * elem_size,
            );
        }
        // Extend the array with the last element if possible
        // SAFETY: `arr` is the live array descriptor.
        if unsafe { (*arr).size } > 0 {
            // SAFETY: `arr.size > 0`, so the last element starts at
            // `(arr.size - 1) * elem_size` inside `arr.data`'s payload.
            let first_elem: *mut u8 =
                unsafe { (data as *mut u8).add(((*arr).size - 1) * elem_size) };
            // SAFETY: `arr` is the live array descriptor.
            for i in unsafe { (*arr).size }..size {
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

    // SAFETY: `p_data` is the caller's writable pointer out-slot (fn contract).
    unsafe { *(p_data as *mut *mut c_void) = data };
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
pub(crate) unsafe fn sort_uv_sets(
    uc: &Context,
    sets: *mut UvSet,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the allocator, data pointer and size slots are uc's own
        // `ator_tmp`/`tmp_arr`/`tmp_arr_size` fields, reached through its
        // views — the matched triple `grow_array` requires.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count * size_of::<UvSet>(),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_uv_set)))"
    );
    // SAFETY: `sets` spans `count` live `ufbx_uv_set` values (fn contract) and
    // `uc.tmp_arr()` was just grown to `count * size_of::<UvSet>()` bytes of
    // scratch, so both the input run and the merge buffer are in bounds;
    // `uv_set_less` is the comparator for that element type.
    unsafe {
        stable_sort(
            size_of::<UvSet>(),
            32,
            sets as *mut c_void,
            uc.tmp_arr() as *mut c_void,
            count,
            uv_set_less,
            core::ptr::null_mut(),
        )
    };
    Ok(())
}

// ufbx.c:12983-12988 `ufbxi_sort_color_sets`
#[inline(never)]
pub(crate) unsafe fn sort_color_sets(
    uc: &Context,
    sets: *mut ColorSet,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the allocator, data pointer and size slots are uc's own
        // `ator_tmp`/`tmp_arr`/`tmp_arr_size` fields, reached through its
        // views — the matched triple `grow_array` requires.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count * size_of::<ColorSet>(),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_color_set)))"
    );
    // SAFETY: `sets` spans `count` live `ufbx_color_set` values (fn contract)
    // and `uc.tmp_arr()` was just grown to `count * size_of::<ColorSet>()`
    // bytes of scratch, so both the input run and the merge buffer are in
    // bounds; `color_set_less` is the comparator for that element type.
    unsafe {
        stable_sort(
            size_of::<ColorSet>(),
            32,
            sets as *mut c_void,
            uc.tmp_arr() as *mut c_void,
            count,
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
pub(crate) unsafe fn sort_blend_offsets(
    uc: &Context,
    offsets: *mut BlendOffset,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: the allocator, data pointer and size slots are uc's own
        // `ator_tmp`/`tmp_arr`/`tmp_arr_size` fields, reached through its
        // views — the matched triple `grow_array` requires.
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count * size_of::<BlendOffset>(),
            )
        },
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbxi_blend_offset)))"
    );
    // SAFETY: `offsets` spans `count` live `ufbxi_blend_offset` values (fn
    // contract) and `uc.tmp_arr()` was just grown to
    // `count * size_of::<BlendOffset>()` bytes of scratch, so both the input
    // run and the merge buffer are in bounds; `blend_offset_less` is the
    // comparator for that element type.
    unsafe {
        stable_sort(
            size_of::<BlendOffset>(),
            16,
            offsets as *mut c_void,
            uc.tmp_arr() as *mut c_void,
            count,
            blend_offset_less,
            core::ptr::null_mut(),
        )
    };
    Ok(())
}

// ufbx.c:13010-13075 `ufbxi_read_shape`
#[inline(never)]
pub(crate) unsafe fn read_shape(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    let node_vertices = find_child(node, sp::Vertices.as_ptr());
    let node_indices = find_child(node, sp::Indexes.as_ptr());
    let node_normals = find_child(node, sp::Normals.as_ptr());
    if node_vertices.is_none() || node_indices.is_none() {
        return Ok(());
    }
    let node_vertices: &NodeView = node_vertices.unwrap();
    let node_indices: &NodeView = node_indices.unwrap();

    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `BlendShape`
    // is the element struct for `ElementType::BlendShape`.
    let shape: *mut BlendShape =
        unsafe { push_element::<BlendShape>(uc, info, ElementType::BlendShape) };
    ufbxi_check!(uc, !shape.is_null(), "shape");

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
    // node's own array descriptor, live for as long as the parse tree.
    ufbxi_check!(
        uc,
        unsafe { (*vertices).size } % 3 == 0,
        "vertices->size % 3 == 0"
    );
    // SAFETY: as above, and `indices` is likewise non-null and live.
    ufbxi_check!(
        uc,
        unsafe { (*indices).size == (*vertices).size / 3 },
        "indices->size == vertices->size / 3"
    );

    // SAFETY: `indices` is a live array descriptor (checked non-null above),
    // whose `'i'` payload is a run of `size` `u32`s.
    let (num_offsets, vertex_indices): (usize, *mut u32) =
        unsafe { ((*indices).size, (*indices).data as *mut u32) };

    // SAFETY: `shape` is the fresh non-null element pushed above; `vertices` is
    // the live array descriptor checked above, whose `'r'` payload is
    // `size == num_offsets * 3` reals, i.e. `num_offsets` `ufbx_vec3` values.
    unsafe {
        (*shape).num_offsets = num_offsets;
        (*shape).position_offsets.data = (*vertices).data as *const Vec3;
        (*shape).offset_vertices.data = vertex_indices;
        (*shape).position_offsets.count = num_offsets;
        (*shape).offset_vertices.count = num_offsets;
    }

    if let Some(node_normals) = node_normals {
        let normals: *mut ValueArray = get_array(node_normals, b'r');
        // SAFETY: `normals` is dereferenced only after the non-null test
        // short-circuits, and `vertices` is the live descriptor checked above.
        ufbxi_check!(
            uc,
            !normals.is_null() && unsafe { (*normals).size } == unsafe { (*vertices).size },
            "normals && normals->size == vertices->size"
        );
        // SAFETY: `shape` is the fresh non-null element; `normals` is non-null
        // (checked) with a `'r'` payload of `size` reals, matching `vertices`,
        // i.e. `num_offsets` `ufbx_vec3` values.
        unsafe {
            (*shape).normal_offsets.data = (*normals).data as *const Vec3;
            (*shape).normal_offsets.count = num_offsets;
        }
    }

    // Sort the blend shape vertices only if absolutely necessary
    let mut sorted: bool = true;
    for i in 1..num_offsets {
        // SAFETY: `vertex_indices` is the `'i'` array's payload of `num_offsets`
        // `u32`s and `1 <= i < num_offsets`, so both reads are in bounds.
        if unsafe { *vertex_indices.add(i - 1) } > unsafe { *vertex_indices.add(i) } {
            sorted = false;
            break;
        }
    }

    if !sorted {
        let offsets: *mut BlendOffset = uc.tmp_stack_view().push::<BlendOffset>(num_offsets);
        ufbxi_check!(uc, !offsets.is_null(), "offsets");

        for i in 0..num_offsets {
            // SAFETY: `offsets` is the non-null `num_offsets`-element run just
            // pushed on `tmp_stack`, and the `shape` arrays were set above to
            // the parse-tree payloads of `num_offsets` entries each, so every
            // `.add(i)` with `i < num_offsets` stays in bounds.
            unsafe {
                (*offsets.add(i)).vertex = *(*shape).offset_vertices.data.add(i);
                (*offsets.add(i)).position_offset = *(*shape).position_offsets.data.add(i);
            }
            if node_normals.is_some() {
                // SAFETY: as above; `node_normals` being present is exactly the
                // branch that set `normal_offsets` to a `num_offsets` run.
                unsafe {
                    (*offsets.add(i)).normal_offset = *(*shape).normal_offsets.data.add(i);
                }
            }
        }

        // SAFETY: `offsets` spans `num_offsets` live `ufbxi_blend_offset`
        // values, just filled in by the loop above.
        unsafe { sort_blend_offsets(uc, offsets, num_offsets) }?;

        for i in 0..num_offsets {
            // SAFETY: as the fill loop — `i < num_offsets` bounds both the
            // `offsets` run and the `shape` arrays. The `shape` arrays point
            // into the parse tree's own mutable array payloads, so the
            // const-to-mut casts write back through their original provenance.
            unsafe {
                *((*shape).offset_vertices.data as *mut u32).add(i) = (*offsets.add(i)).vertex;
                *((*shape).position_offsets.data as *mut Vec3).add(i) =
                    (*offsets.add(i)).position_offset;
            }
            if node_normals.is_some() {
                // SAFETY: as above; `node_normals` being present is exactly the
                // branch that set `normal_offsets` to a `num_offsets` run.
                unsafe {
                    *((*shape).normal_offsets.data as *mut Vec3).add(i) =
                        (*offsets.add(i)).normal_offset;
                }
            }
        }
        // SAFETY: `uc.tmp_stack_mut_ptr()` is uc's own live `tmp_stack` buf and
        // the `num_offsets` `BlendOffset` values pushed above are still its top;
        // a null `dst` discards them.
        unsafe { pop::<BlendOffset>(uc.tmp_stack_mut_ptr(), num_offsets, core::ptr::null_mut()) };
    }

    Ok(())
}

// ufbx.c:13077-13137 `ufbxi_read_synthetic_blend_shapes`
#[inline(never)]
pub(crate) unsafe fn read_synthetic_blend_shapes(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    let mut deformer: *mut BlendDeformer = core::ptr::null_mut();
    let mut deformer_fbx_id: u64 = 0;

    // C: `ufbxi_for (ufbxi_node, n, node->children, node->num_children)`
    // SAFETY: `children`/`num_children` describe a contiguous arena run (built via
    // `push_pop`), valid and stable for `node`'s borrow.
    let children =
        unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) };
    for n in children {
        if n.name() != sp::Shape.as_ptr() {
            continue;
        }

        // C: `ufbx_string name;` — fully written by `ufbxi_get_val1` before any
        // read; zero-initialized here (no upstream `ufbxi_uninit` marker).
        // SAFETY: `ufbx_string` is a plain pointer/length pair, for which the
        // all-zero bit pattern is a valid (empty, null-data) value.
        let mut name: String = unsafe { core::mem::zeroed() };
        // SAFETY: fmt `'S'` pairs with the `*mut String` out-pointer
        // `&mut name`, which is a live local.
        ufbxi_check!(
            uc,
            unsafe { get_val1(n, b"S\0".as_ptr(), &mut name as *mut String as *mut c_void) },
            "ufbxi_get_val1(n, \"S\", &name)"
        );

        if deformer.is_null() {
            // SAFETY: `deformer_fbx_id` is a live local `u64` out-slot;
            // `name.data` was written by the `'S'` fetch above as a
            // NUL-terminated parse-tree string; `BlendDeformer` is the element
            // struct for `ElementType::BlendDeformer`.
            deformer = unsafe {
                push_synthetic_element::<BlendDeformer>(
                    uc,
                    &mut deformer_fbx_id,
                    Some(n),
                    name.data,
                    ElementType::BlendDeformer,
                )
            };
            ufbxi_check!(uc, !deformer.is_null(), "deformer");
            // SAFETY: `info` is the caller's live `ufbxi_element_info`.
            connect_oo(uc, deformer_fbx_id, unsafe { (*info).fbx_id })?;
        }

        let mut channel_fbx_id: u64 = 0;
        // SAFETY: `channel_fbx_id` is a live local `u64` out-slot; `name.data`
        // is the NUL-terminated parse-tree string fetched above;
        // `BlendChannel` is the element struct for `ElementType::BlendChannel`.
        let channel: *mut BlendChannel = unsafe {
            push_synthetic_element::<BlendChannel>(
                uc,
                &mut channel_fbx_id,
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
            (*shape_props.add(0))._internal_key = get_name_key_c(sp::DeformPercent.as_ptr());
            (*shape_props.add(0)).type_ = PropType::Number;
            (*shape_props.add(0)).value_vec4.x = 0.0 as Real;
            (*shape_props.add(0)).value_str = EMPTY_STRING.0;
            (*shape_props.add(0)).value_blob = EMPTY_BLOB.0;
        }

        // SAFETY: `info` is the caller's live `ufbxi_element_info`, so
        // `&raw mut (*info).props` addresses its live `ufbx_props` field and
        // `PropsView::from_ptr` may anchor to it; `name` is the interned
        // string fetched above, readable for its length (`as_bytes`).
        let self_prop: Option<&PropView> =
            unsafe { find_prop_len(PropsView::from_ptr(&raw mut (*info).props), name.as_bytes()) };
        if self_prop.is_some_and(|prop| {
            prop.type_() == PropType::Number || prop.type_() == PropType::Integer
        }) {
            // `is_some_and` above guarantees `self_prop` is `Some`.
            // SAFETY: index `0` of the pushed `ufbx_prop` run.
            unsafe {
                (*shape_props.add(0)).value_vec4.x = self_prop.unwrap().value_vec4().x;
            }
            // SAFETY: `info` is the caller's live `ufbxi_element_info` and
            // index `0` of the pushed `ufbx_prop` run holds the name set above.
            unsafe {
                connect_pp(
                    uc,
                    (*info).fbx_id,
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
                    (*info).fbx_id,
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

        // SAFETY: `&mut shape_info` is a live local `ufbxi_element_info`.
        unsafe { read_shape(uc, n, &mut shape_info) }?;

        connect_oo(uc, channel_fbx_id, deformer_fbx_id)?;
        connect_oo(uc, shape_info.fbx_id, channel_fbx_id)?;
    }

    Ok(())
}

// ufbx.c:13139-13216 `ufbxi_process_indices`
#[inline(never)]
pub(crate) unsafe fn process_indices(
    uc: &Context,
    mesh: &View<Mesh>,
    index_data: *mut u32,
) -> Result<(), Fail> {
    // Count the number of faces and allocate the index list
    // Indices less than zero (~actual_index) ends a polygon
    let mut num_total_faces: usize = 0;
    // C: `ufbxi_for (uint32_t, p_ix, index_data, mesh->num_indices)`
    let mut p_ix = index_data;
    let p_ix_end = add_ptr(index_data, mesh.num_indices());
    while p_ix != p_ix_end {
        // SAFETY: `p_ix` walks `index_data..index_data + mesh->num_indices`, the
        // caller's index run, and is short of `p_ix_end` here.
        num_total_faces = num_total_faces.wrapping_add(if (unsafe { *p_ix } as i32) < 0 {
            1usize
        } else {
            0usize
        });
        // SAFETY: `p_ix` is before `p_ix_end`, so the advance lands at most one
        // past the run's end.
        p_ix = unsafe { p_ix.add(1) };
    }
    mesh.faces_view()
        .set_data(uc.result_view().push::<Face>(num_total_faces));
    ufbxi_check!(uc, !mesh.faces().data.is_null(), "mesh->faces.data");

    let mut num_triangles: usize = 0;
    let mut max_face_triangles: usize = 0;
    let mut num_bad_faces: [usize; 3] = [0; 3];

    // `faces.data` is the non-null `num_total_faces` run pushed above.
    let mut dst_face: *mut Face = mesh.faces().data as *mut Face;
    let mut p_face_begin: *mut u32 = index_data;
    // C: `ufbxi_for (uint32_t, p_ix, index_data, mesh->num_indices)`
    let mut p_ix = index_data;
    let p_ix_end = add_ptr(index_data, mesh.num_indices());
    while p_ix != p_ix_end {
        // SAFETY: `p_ix` is inside the caller's index run, short of `p_ix_end`.
        let mut ix: u32 = unsafe { *p_ix };
        // Un-negate final indices of polygons
        if (ix as i32) < 0 {
            ix = !ix;
            // SAFETY: as above — `p_ix` addresses a live, writable index.
            unsafe { *p_ix = ix };
            // SAFETY: `p_face_begin` and `p_ix` are both inside the same
            // `index_data` run (`p_face_begin` starts at its base and only ever
            // advances to a position `p_ix` already reached).
            let num_indices: u32 = (unsafe { p_ix.offset_from(p_face_begin) } + 1) as u32;
            // SAFETY: `dst_face` walks the `num_total_faces` run pushed above,
            // one slot per negative index, of which the first pass counted
            // exactly `num_total_faces`; `p_face_begin` is inside the
            // `index_data` run.
            unsafe {
                (*dst_face).index_begin = p_face_begin.offset_from(index_data) as u32;
                (*dst_face).num_indices = num_indices;
            }
            if num_indices >= 3 {
                num_triangles = num_triangles.wrapping_add((num_indices - 2) as usize);
                max_face_triangles = max_sz(max_face_triangles, (num_indices - 2) as usize);
            } else {
                num_bad_faces[num_indices as usize] =
                    num_bad_faces[num_indices as usize].wrapping_add(1);
            }
            // SAFETY: this is the `num_total_faces`-th slot at the very latest,
            // so the advance lands at most one past the face run's end.
            dst_face = unsafe { dst_face.add(1) };
            // SAFETY: `p_ix` is before `p_ix_end`, so the advance lands at most
            // one past the index run's end.
            p_face_begin = unsafe { p_ix.add(1) };
        }
        ufbxi_check!(
            uc,
            (ix as usize) < mesh.num_vertices(),
            "(size_t)ix < mesh->num_vertices"
        );
        // SAFETY: `p_ix` is before `p_ix_end`, so the advance lands at most one
        // past the index run's end.
        p_ix = unsafe { p_ix.add(1) };
    }

    mesh.vertex_position().indices_view().set_data(index_data);
    // SAFETY: `dst_face` and `faces.data` are one-past-the-last and the base of
    // the same face run.
    mesh.set_num_faces(to_size(unsafe {
        dst_face.offset_from(mesh.faces().data as *mut Face)
    }));
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
    // `vertex_first_index` is the non-null `count`-long run pushed above.
    let mut p_vx_ix = mesh.vertex_first_index().data as *mut u32;
    let p_vx_ix_end = add_ptr(p_vx_ix, mesh.vertex_first_index().count);
    while p_vx_ix != p_vx_ix_end {
        // SAFETY: `p_vx_ix` is inside that run, short of `p_vx_ix_end`.
        unsafe { *p_vx_ix = NO_INDEX };
        // SAFETY: `p_vx_ix` is before `p_vx_ix_end`, so the advance lands at
        // most one past the run's end.
        p_vx_ix = unsafe { p_vx_ix.add(1) };
    }

    {
        // `vertex_indices` is the mesh's `num_indices`-long index run and
        // `vertex_first_index` the `num_vertices`-long run pushed above.
        let num_indices: usize = mesh.num_indices();
        let num_vertices: usize = mesh.num_vertices();
        let vertex_indices: *mut u32 = mesh.vertex_indices().data as *mut u32;
        let vertex_first_index: *mut u32 = mesh.vertex_first_index().data as *mut u32;
        let mut ix: usize = 0;
        while ix < num_indices {
            // SAFETY: `ix < num_indices` bounds the read in the index run.
            let vx: u32 = unsafe { *vertex_indices.add(ix) };
            if (vx as usize) < num_vertices {
                // SAFETY: `vx < num_vertices` bounds both accesses in the
                // `vertex_first_index` run.
                if unsafe { *vertex_first_index.add(vx as usize) } == NO_INDEX {
                    unsafe { *vertex_first_index.add(vx as usize) = ix as u32 };
                }
            } else {
                // SAFETY: `ix < num_indices` bounds the slot handed to
                // `fix_index`.
                unsafe { fix_index(uc, vertex_indices.add(ix), vx, mesh.num_vertices()) }?;
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
// Safe `fn`: the mesh view is the only parameter, and each residual raw op
// walks a run described by that same mesh's own list fields — which every
// minted `View<Mesh>` keeps either zeroed (fresh `tmp_elements` element) or
// pointing at a live `count`-long arena run, so the walks stay in bounds.
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
    // `uv_sets` is the mesh's own `count`-long run of `ufbx_uv_set`.
    let mut set: *mut UvSet = mesh.uv_sets().data as *mut UvSet;
    let set_end = add_ptr(set, mesh.uv_sets().count);
    while set != set_end {
        // SAFETY: `set` is inside the `uv_sets` run, short of `set_end`.
        unsafe {
            (*set).vertex_uv.value_reals = 2;
            (*set).vertex_tangent.value_reals = 3;
            (*set).vertex_bitangent.value_reals = 3;
        }
        // SAFETY: `set` is before `set_end`, so the advance lands at most one
        // past the run's end.
        set = unsafe { set.add(1) };
    }

    // C: `ufbxi_nounroll ufbxi_for_list(ufbx_color_set, set, mesh->color_sets)`
    // `color_sets` is the mesh's own `count`-long run of `ufbx_color_set`.
    let mut set: *mut ColorSet = mesh.color_sets().data as *mut ColorSet;
    let set_end = add_ptr(set, mesh.color_sets().count);
    while set != set_end {
        // SAFETY: `set` is inside the `color_sets` run, short of `set_end`.
        unsafe { (*set).vertex_color.value_reals = 4 };
        // SAFETY: `set` is before `set_end`, so the advance lands at most one
        // past the run's end.
        set = unsafe { set.add(1) };
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
pub(crate) unsafe fn mesh_part_add_face(part: *mut MeshPart, num_indices: u32) {
    // SAFETY: `part` is the caller's live arena `ufbx_mesh_part` (fn contract),
    // reached through the mesh's own part run — write-capable provenance.
    let part: &View<MeshPart> = unsafe { View::<MeshPart>::from_ptr(part) };
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
pub(crate) unsafe fn assign_face_groups(
    buf: &BufView,
    error: *mut Error,
    mesh: &View<Mesh>,
    p_consecutive_indices: *mut usize,
    retain_parts: bool,
) -> Result<(), Fail> {
    let num_faces: usize = mesh.num_faces();
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr(error) },
        num_faces > 0
    );
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr(error) },
        num_faces < u32::MAX as usize,
        "num_faces < UINT32_MAX"
    );
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr(error) },
        mesh.face_group().count == num_faces,
        "mesh->face_group.count == num_faces"
    );

    let ids: *mut u32 = buf.push::<u32>(num_faces);
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr(error) },
        !ids.is_null(),
        "ids"
    );

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

    // Loosely deduplicate group IDs
    // C: `ufbxi_for_list(uint32_t, p_id, mesh->face_group)`
    // `face_group` is the mesh's own `count`-long run of `uint32_t`.
    let mut p_id: *mut u32 = mesh.face_group().data as *mut u32;
    let p_id_end = add_ptr(p_id, mesh.face_group().count);
    while p_id != p_id_end {
        // SAFETY: `p_id` is inside the `face_group` run, short of `p_id_end`.
        let id: u32 = unsafe { *p_id };
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
            // `face_group.count == num_faces`, so `num_ids < num_faces`.
            unsafe { *ids.add(num_ids as usize) = id };
            num_ids = num_ids.wrapping_add(1);
        }
        // SAFETY: `p_id` is before `p_id_end`, so the advance lands at most one
        // past the run's end.
        p_id = unsafe { p_id.add(1) };
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

    let mut num_groups: usize = 0;
    let mut i: usize = 0;
    while i < num_ids as usize {
        // SAFETY: `i < num_ids <= num_faces` bounds the read in the `ids` run.
        let id: u32 = unsafe { *ids.add(i) };
        // C: `ids[num_groups++] = id;`
        // SAFETY: `num_groups <= i < num_ids`, since it advances at most once
        // per outer iteration, so the write is in the `ids` run.
        unsafe { *ids.add(num_groups) = id };
        num_groups += 1;
        // C: `do { i++; } while (i < num_ids && ids[i] == id);`
        loop {
            i += 1;
            // SAFETY: the `i < num_ids` test short-circuits before the read, so
            // `.add(i)` stays inside the `ids` run.
            if !(i < num_ids as usize && unsafe { *ids.add(i) } == id) {
                break;
            }
        }
    }

    // Allocate group info structs
    let groups: *mut FaceGroup = buf.push_zero::<FaceGroup>(num_groups);
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr(error) },
        !groups.is_null(),
        "groups"
    );
    for i in 0..num_groups {
        // SAFETY: `groups` is the non-null `num_groups`-long run pushed above
        // and `num_groups <= num_ids <= num_faces` bounds `ids.add(i)` too.
        unsafe {
            (*groups.add(i)).id = *ids.add(i) as i32;
            (*groups.add(i)).name.data = EMPTY_CHAR.as_ptr();
        }
    }

    // `groups` is the `num_groups`-long run pushed on `buf`.
    mesh.face_groups_view().set_data(groups);
    mesh.face_groups_view().set_count(num_groups);

    let mut parts: *mut MeshPart = core::ptr::null_mut();
    if retain_parts {
        parts = buf.push_zero::<MeshPart>(num_groups);
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            !parts.is_null(),
            "parts"
        );
        // `parts` is the non-null `num_groups`-long run just pushed on `buf`.
        mesh.face_group_parts_view().set_data(parts);
        mesh.face_group_parts_view().set_count(num_groups);
    }

    // Optimization: Use `consecutive_indices` for a single group
    if !p_consecutive_indices.is_null() && num_groups == 1 {
        // SAFETY: `face_group` was checked above to hold exactly `num_faces`
        // writable `uint32_t`s.
        unsafe { core::ptr::write_bytes(mesh.face_group().data as *mut u32, 0, num_faces) };

        if !parts.is_null() {
            // SAFETY: `parts` is non-null (checked) with `num_groups == 1`
            // entry, so index `0` is in bounds.
            unsafe {
                (*parts.add(0)).face_indices.data = SENTINEL_INDEX_CONSECUTIVE.as_ptr() as *mut u32;
                (*parts.add(0)).face_indices.count = num_faces;
                (*parts.add(0)).num_empty_faces = mesh.num_empty_faces();
                (*parts.add(0)).num_point_faces = mesh.num_point_faces();
                (*parts.add(0)).num_line_faces = mesh.num_line_faces();
                (*parts.add(0)).num_faces = num_faces;
                (*parts.add(0)).num_triangles = mesh.num_triangles();
            }
        }

        // SAFETY: `p_consecutive_indices` is non-null (checked) and is the
        // caller's writable `size_t` slot.
        unsafe { *p_consecutive_indices = max_sz(*p_consecutive_indices, num_faces) };
        return Ok(());
    }

    // SAFETY: `seen_ids` is a live local array of exactly `SEEN_IDS_COUNT`
    // `ufbxi_id_group`s, so the zero fill covers exactly it.
    unsafe { core::ptr::write_bytes(seen_ids.as_mut_ptr(), 0, SEEN_IDS_COUNT) };

    // Count faces and triangles per group and reassign IDs
    // `faces` is the mesh's own `num_faces`-long run and `face_group` its
    // `count`-long `uint32_t` run (checked equal to `num_faces` above).
    let mut p_face: *const Face = mesh.faces().data;
    // C: `ufbxi_for_list(uint32_t, p_id, mesh->face_group)`
    let mut p_id: *mut u32 = mesh.face_group().data as *mut u32;
    let p_id_end = add_ptr(p_id, mesh.face_group().count);
    while p_id != p_id_end {
        // SAFETY: `p_id` is inside the `face_group` run, short of `p_id_end`.
        let id: u32 = unsafe { *p_id };
        let id_hash: u32 = id.wrapping_mul(seed) >> (32u32 - FACE_GROUP_HASH_BITS);

        // SAFETY: `p_face` advances in lockstep with `p_id`, so it stays inside
        // the equally long `faces` run.
        let num_indices: u32 = unsafe { (*p_face).num_indices };

        let mut index: usize;
        if seen_ids[id_hash as usize].id == id && seen_ids[id_hash as usize].index > 0 {
            index = (seen_ids[id_hash as usize].index - 1) as usize;
            // SAFETY: `p_id` addresses a live, writable `face_group` entry.
            unsafe { *p_id = index as u32 };
        } else {
            let signed_id: i32 = id as i32;
            index = usize::MAX;
            // SAFETY: `groups` is the non-null `num_groups`-long run pushed
            // above, so the `[0, num_groups)` search window is in bounds, and
            // the two predicates only read the `id` field of an element the
            // search hands them from inside that window.
            unsafe {
                macro_lower_bound_eq::<FaceGroup>(
                    8,
                    &mut index,
                    groups,
                    0,
                    num_groups,
                    |a| (*a).id < signed_id,
                    |a| (*a).id == signed_id,
                )
            };
            ufbx_assert!(index < num_groups);
            seen_ids[id_hash as usize].id = id;
            seen_ids[id_hash as usize].index = (index as u32).wrapping_add(1);
        }

        if !parts.is_null() {
            // SAFETY: `parts` is non-null (checked) with `num_groups` entries
            // and `index < num_groups` — the `groups` slot the id resolved to.
            unsafe { mesh_part_add_face(parts.add(index), num_indices) };
        }

        // SAFETY: `p_id` addresses a live, writable `face_group` entry.
        unsafe { *p_id = index as u32 };
        // SAFETY: `p_face` is short of the `faces` run's end, so the advance
        // lands at most one past it.
        p_face = unsafe { p_face.add(1) };
        // SAFETY: `p_id` is before `p_id_end`, so the advance lands at most one
        // past the run's end.
        p_id = unsafe { p_id.add(1) };
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
        // SAFETY: `face_indices_raw()` addresses the part's own index list;
        // `face_indices` sub-divides the `num_faces`-long `ids` run, whose total
        // length is the sum of the parts' `num_faces`.
        unsafe {
            (*part.face_indices_raw()).data = face_indices;
        }
        face_indices = add_ptr(face_indices, part.num_faces());
    }
    ufbx_assert!(face_indices == add_ptr(ids, num_faces));

    // Collect per-group faces
    let mut face_index: u32 = 0;
    // C: `ufbxi_for_list(uint32_t, p_id, mesh->face_group)`
    // `face_group` is the mesh's own `count`-long run of `uint32_t`.
    let mut p_id: *mut u32 = mesh.face_group().data as *mut u32;
    let p_id_end = add_ptr(p_id, mesh.face_group().count);
    while p_id != p_id_end {
        // SAFETY: `p_id` is inside the `face_group` run, short of `p_id_end`,
        // and the loop above rewrote every entry to a group index below
        // `num_groups`, which bounds it in the `parts` run.
        let part: &View<MeshPart> =
            unsafe { View::<MeshPart>::from_ptr(parts.add(*p_id as usize)) };
        // C: `part->face_indices.data[part->face_indices.count++] = face_index++;`
        // SAFETY: `part`'s `face_indices` is the sub-range of `ids` assigned
        // above, sized to the part's `num_faces`; `count` starts at zero and is
        // bumped once per face belonging to this part, so it stays within it.
        unsafe {
            *(part.face_indices().data as *mut u32).add(part.face_indices().count) = face_index;
            (*part.face_indices_raw()).count += 1;
        }
        face_index = face_index.wrapping_add(1);
        // SAFETY: `p_id` is before `p_id_end`, so the advance lands at most one
        // past the run's end.
        p_id = unsafe { p_id.add(1) };
    }

    Ok(())
}

// ufbx.c:13399-13432 `ufbxi_update_face_groups`
#[inline(never)]
pub(crate) unsafe fn update_face_groups(
    buf: &BufView,
    error: *mut Error,
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
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            !mesh.face_group_parts().data.is_null(),
            "mesh->face_group_parts.data"
        );
    }

    let mut face_indices: *mut u32 = buf.push::<u32>(num_faces);
    ufbxi_check_err!(
        unsafe { crate::native::error::ErrorView::from_ptr(error) },
        !face_indices.is_null(),
        "face_indices"
    );

    // C: `ufbxi_nounroll for (size_t i = 0; i < num_faces; i++)`
    for i in 0..num_faces {
        // SAFETY: `i < num_faces` bounds the `face_group` and `faces` reads
        // (both runs are `num_faces` long), and every `face_group` entry is a
        // group index below `face_group_parts.count == num_groups`, which
        // bounds the part slot.
        let part: *mut MeshPart = unsafe {
            (mesh.face_group_parts().data as *mut MeshPart)
                .add(*mesh.face_group().data.add(i) as usize)
        };
        // SAFETY: `part` is that in-bounds `ufbx_mesh_part`, and `i < num_faces`
        // bounds the `faces` read.
        unsafe { mesh_part_add_face(part, (*mesh.faces().data.add(i)).num_indices) };
    }

    let mut part_index: u32 = 0;
    // C: `ufbxi_for_list(ufbx_mesh_part, part, mesh->face_group_parts)`
    // SAFETY: `face_group_parts` is the mesh's own contiguous `count`-long run
    // of `ufbx_mesh_part`, live for this call.
    for part in unsafe {
        SliceViewIter::<MeshPart>::from_raw_parts(
            mesh.face_group_parts().data as *mut MeshPart,
            mesh.face_group_parts().count,
        )
    } {
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
        // SAFETY: as the counting loop above — `i < num_faces` bounds the
        // `face_group` read and its entry is a group index below `num_groups`.
        let part: &View<MeshPart> = unsafe {
            View::<MeshPart>::from_ptr(
                (mesh.face_group_parts().data as *mut MeshPart)
                    .add(*mesh.face_group().data.add(i as usize) as usize),
            )
        };
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
pub(crate) unsafe fn read_mesh(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Mesh` is the
    // element struct for `ElementType::Mesh`.
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
        // SAFETY: `info` is the caller's live `ufbxi_element_info`.
        unsafe { read_synthetic_blend_shapes(uc, node, info) }?;
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
    // node's own array descriptor, live for as long as the parse tree.
    ufbxi_check!(
        uc,
        unsafe { (*vertices).size } % 3 == 0,
        "vertices->size % 3 == 0"
    );

    // SAFETY: `vertices` is the live array descriptor checked above, and
    // `indices` is dereferenced only in the non-null arm — where it is likewise
    // a live array descriptor.
    unsafe {
        mesh.set_num_vertices((*vertices).size / 3);
        mesh.set_num_indices(if !indices.is_null() {
            (*indices).size
        } else {
            0
        });
    }

    // SAFETY: as above — `indices` is only dereferenced in its non-null arm, and
    // the `'i'` array's payload is a run of `size` `u32`s.
    let mut index_data: *mut u32 = unsafe {
        if !indices.is_null() {
            (*indices).data as *mut u32
        } else {
            core::ptr::null_mut()
        }
    };

    // Duplicate `index_data` for modification if we retain DOM
    if uc.opts_view().retain_dom() {
        // SAFETY: `uc.result_mut_ptr()` is uc's own live result buf, and
        // `index_data` spans `mesh->num_indices` `u32`s (it is the `'i'` array's
        // payload, or null when `num_indices` is 0).
        index_data =
            unsafe { push_copy::<u32>(uc.result_mut_ptr(), mesh.num_indices(), index_data) };
        ufbxi_check!(uc, !index_data.is_null(), "index_data");
    }

    // SAFETY (both `vertices` reads): `vertices` is the live array descriptor
    // whose `'r'` payload is `size == num_vertices * 3` reals, i.e.
    // `num_vertices` `ufbx_vec3` values.
    mesh.vertices_view()
        .set_data(unsafe { (*vertices).data } as *const Vec3);
    mesh.vertices_view().set_count(mesh.num_vertices());
    mesh.vertex_indices_view().set_data(index_data);
    mesh.vertex_indices_view().set_count(mesh.num_indices());

    mesh.vertex_position().set_exists(true);
    mesh.vertex_position()
        .values_view()
        .set_data(unsafe { (*vertices).data } as *const Vec3);
    mesh.vertex_position()
        .values_view()
        .set_count(mesh.num_vertices());
    mesh.vertex_position().indices_view().set_data(index_data);
    mesh.vertex_position()
        .indices_view()
        .set_count(mesh.num_indices());
    mesh.vertex_position().set_unique_per_vertex(true);

    // Check/make sure that the last index is negated (last of polygon)
    if mesh.num_indices() > 0 {
        // SAFETY: `num_indices > 0` here, so `index_data` is the non-null run of
        // that many `u32`s and its last slot is in bounds.
        if unsafe { *index_data.add(mesh.num_indices() - 1) } as i32 >= 0 {
            if uc.opts_view().strict() {
                ufbxi_fail!(uc, "Non-negated last index");
            }
            // SAFETY: as above — the last slot of the `index_data` run.
            unsafe {
                *index_data.add(mesh.num_indices() - 1) = !*index_data.add(mesh.num_indices() - 1);
            }
        }
    }

    // Read edges before un-negating the indices
    if !edge_indices.is_null() {
        // SAFETY: `edge_indices` is non-null (checked) and `find_array` returns
        // a node's own array descriptor, live for as long as the parse tree.
        let num_edges: usize = unsafe { (*edge_indices).size };
        let edges: *mut Edge = uc.result_view().push::<Edge>(num_edges);
        ufbxi_check!(uc, !edges.is_null(), "edges");

        let mut dst_ix: usize = 0;

        // Edges are represented using a single index into PolygonVertexIndex.
        // The edge is between two consecutive vertices in the polygon.
        // SAFETY: as above; the `'i'` array's payload is a run of `size` `u32`s.
        let edge_data: *mut u32 = unsafe { (*edge_indices).data } as *mut u32;
        for i in 0..num_edges {
            // SAFETY: `i < num_edges` bounds the read in the `edge_data` run.
            let mut index_ix: u32 = unsafe { *edge_data.add(i) };
            if index_ix as usize >= mesh.num_indices() {
                if uc.opts_view().strict() {
                    ufbxi_fail!(uc, "Edge index out of bounds");
                }
                continue;
            }
            // SAFETY: `edges` is the non-null `num_edges`-long run pushed above
            // and `dst_ix` advances at most once per edge, so it is in bounds.
            unsafe { (*edges.add(dst_ix)).a = index_ix };
            // SAFETY: the test above leaves `index_ix < mesh->num_indices`,
            // which bounds the read in the `index_data` run.
            if (unsafe { *index_data.add(index_ix as usize) } as i32) < 0 {
                // Previous index is the last one of this polygon, rewind to first index.
                // SAFETY: `index_ix > 0` short-circuits before the read, and
                // `index_ix` only decreases from a value below `num_indices`, so
                // `index_ix - 1` stays inside the `index_data` run.
                while index_ix > 0 && unsafe { *index_data.add(index_ix as usize - 1) } as i32 >= 0
                {
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
            // SAFETY: `dst_ix` is still the in-bounds `edges` slot written above.
            unsafe { (*edges.add(dst_ix)).b = index_ix };
            dst_ix += 1;
        }

        // `edges` is the `num_edges`-long result-arena run, of which `dst_ix`
        // were filled.
        mesh.edges_view().set_data(edges);
        mesh.edges_view().set_count(dst_ix);
        mesh.set_num_edges(mesh.edges().count);
    }

    // SAFETY: `index_data` spans the mesh's `num_indices` `u32`s.
    unsafe { process_indices(uc, mesh, index_data) }?;

    // Count the number of UV/color sets
    let mut num_uv: usize = 0;
    let mut num_color: usize = 0;
    let mut num_bitangents: usize = 0;
    let mut num_tangents: usize = 0;
    // C: `ufbxi_for (ufbxi_node, n, node->children, node->num_children)`
    // SAFETY: contiguous push_pop child run, valid for `node`'s borrow.
    for n in unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) }
    {
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
    // SAFETY: contiguous push_pop child run, valid for `node`'s borrow.
    for n in unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) }
    {
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
            // `ufbx_vertex_vec3` field; the `3` value-real count matches that
            // attribute type.
            unsafe {
                read_vertex_element(
                    uc,
                    mesh,
                    n,
                    mesh.vertex_normal_raw() as *mut VertexAttrib,
                    sp::Normals.as_ptr(),
                    sp::NormalsIndex.as_ptr(),
                    sp::NormalsW.as_ptr(),
                    b'r',
                    3,
                )
            }?;
        } else if n.name() == sp::LayerElementBinormal.as_ptr() {
            // SAFETY: the counting pass above found exactly `num_bitangents`
            // `LayerElementBinormal` children, and this branch consumes one slot
            // per such child, so `num_bitangents_read < num_bitangents` bounds
            // the offset in the `bitangents` run.
            let layer: *mut TangentLayer = unsafe { bitangents.add(num_bitangents_read) };
            num_bitangents_read += 1;

            // SAFETY: fmt `'I'` pairs with the `*mut u32` out-pointer
            // `&mut (*layer).index`, a field of the in-bounds `layer` slot.
            ufbxi_ignore!(unsafe {
                get_val1(
                    n,
                    b"I\0".as_ptr(),
                    &mut (*layer).index as *mut u32 as *mut c_void,
                )
            });
            // SAFETY: `&mut (*layer).elem` addresses the in-bounds slot's live
            // `ufbx_vertex_vec3`; the `3` value-real count matches it.
            unsafe {
                read_vertex_element(
                    uc,
                    mesh,
                    n,
                    &mut (*layer).elem as *mut VertexVec3 as *mut VertexAttrib,
                    sp::Binormals.as_ptr(),
                    sp::BinormalsIndex.as_ptr(),
                    sp::BinormalsW.as_ptr(),
                    b'r',
                    3,
                )
            }?;
            // SAFETY: `layer` is that in-bounds slot, written by the call above.
            if !unsafe { (*layer).elem.exists } {
                num_bitangents_read -= 1;
            }
        } else if n.name() == sp::LayerElementTangent.as_ptr() {
            // SAFETY: the counting pass above found exactly `num_tangents`
            // `LayerElementTangent` children, and this branch consumes one slot
            // per such child, so `num_tangents_read < num_tangents` bounds the
            // offset in the `tangents` run.
            let layer: *mut TangentLayer = unsafe { tangents.add(num_tangents_read) };
            num_tangents_read += 1;

            // SAFETY: fmt `'I'` pairs with the `*mut u32` out-pointer
            // `&mut (*layer).index`, a field of the in-bounds `layer` slot.
            ufbxi_ignore!(unsafe {
                get_val1(
                    n,
                    b"I\0".as_ptr(),
                    &mut (*layer).index as *mut u32 as *mut c_void,
                )
            });
            // SAFETY: `&mut (*layer).elem` addresses the in-bounds slot's live
            // `ufbx_vertex_vec3`; the `3` value-real count matches it.
            unsafe {
                read_vertex_element(
                    uc,
                    mesh,
                    n,
                    &mut (*layer).elem as *mut VertexVec3 as *mut VertexAttrib,
                    sp::Tangents.as_ptr(),
                    sp::TangentsIndex.as_ptr(),
                    sp::TangentsW.as_ptr(),
                    b'r',
                    3,
                )
            }?;
            // SAFETY: `layer` is that in-bounds slot, written by the call above.
            if !unsafe { (*layer).elem.exists } {
                num_tangents_read -= 1;
            }
        } else if n.name() == sp::LayerElementUV.as_ptr() {
            // SAFETY: `uv_sets.data` is the `num_uv`-long run pushed above and
            // the counting pass found exactly `num_uv` `LayerElementUV`
            // children, so `uv_sets.count` (bumped at most once per such child)
            // is in bounds.
            let set: *mut UvSet =
                unsafe { (mesh.uv_sets().data as *mut UvSet).add(mesh.uv_sets().count) };
            mesh.uv_sets_view().set_count(mesh.uv_sets().count + 1);

            // SAFETY: fmt `'I'` pairs with the `*mut u32` out-pointer
            // `&mut (*set).index`, a field of the in-bounds `set` slot.
            ufbxi_ignore!(unsafe {
                get_val1(
                    n,
                    b"I\0".as_ptr(),
                    &mut (*set).index as *mut u32 as *mut c_void,
                )
            });
            // SAFETY: fmt `'S'` pairs with the `*mut String` out-pointer
            // `&mut (*set).name`, a field of the in-bounds `set` slot.
            if !unsafe {
                find_val1(
                    n,
                    sp::Name.as_ptr(),
                    b"S\0".as_ptr(),
                    &mut (*set).name as *mut String as *mut c_void,
                )
            } {
                // SAFETY: `set` is that in-bounds slot.
                unsafe { (*set).name = EMPTY_STRING.0 };
            }

            // SAFETY: `&mut (*set).vertex_uv` addresses the in-bounds slot's
            // live `ufbx_vertex_vec2`; the `2` value-real count matches it.
            unsafe {
                read_vertex_element(
                    uc,
                    mesh,
                    n,
                    &mut (*set).vertex_uv as *mut VertexVec2 as *mut VertexAttrib,
                    sp::UV.as_ptr(),
                    sp::UVIndex.as_ptr(),
                    core::ptr::null(),
                    b'r',
                    2,
                )
            }?;
            // SAFETY: `set` is that in-bounds slot, written by the call above.
            if !unsafe { (*set).vertex_uv.exists } {
                mesh.uv_sets_view().set_count(mesh.uv_sets().count - 1);
            }
        } else if n.name() == sp::LayerElementColor.as_ptr() {
            // SAFETY: `color_sets.data` is the `num_color`-long run pushed
            // above and the counting pass found exactly `num_color`
            // `LayerElementColor` children, so `color_sets.count` (bumped at
            // most once per such child) is in bounds.
            let set: *mut ColorSet =
                unsafe { (mesh.color_sets().data as *mut ColorSet).add(mesh.color_sets().count) };
            mesh.color_sets_view()
                .set_count(mesh.color_sets().count + 1);

            // SAFETY: fmt `'I'` pairs with the `*mut u32` out-pointer
            // `&mut (*set).index`, a field of the in-bounds `set` slot.
            ufbxi_ignore!(unsafe {
                get_val1(
                    n,
                    b"I\0".as_ptr(),
                    &mut (*set).index as *mut u32 as *mut c_void,
                )
            });
            // SAFETY: fmt `'S'` pairs with the `*mut String` out-pointer
            // `&mut (*set).name`, a field of the in-bounds `set` slot.
            if !unsafe {
                find_val1(
                    n,
                    sp::Name.as_ptr(),
                    b"S\0".as_ptr(),
                    &mut (*set).name as *mut String as *mut c_void,
                )
            } {
                // SAFETY: `set` is that in-bounds slot.
                unsafe { (*set).name = EMPTY_STRING.0 };
            }

            // SAFETY: `&mut (*set).vertex_color` addresses the in-bounds slot's
            // live `ufbx_vertex_vec4`; the `4` value-real count matches it.
            unsafe {
                read_vertex_element(
                    uc,
                    mesh,
                    n,
                    &mut (*set).vertex_color as *mut VertexVec4 as *mut VertexAttrib,
                    sp::Colors.as_ptr(),
                    sp::ColorIndex.as_ptr(),
                    core::ptr::null(),
                    b'r',
                    4,
                )
            }?;
            // SAFETY: `set` is that in-bounds slot, written by the call above.
            if !unsafe { (*set).vertex_color.exists } {
                mesh.color_sets_view()
                    .set_count(mesh.color_sets().count - 1);
            }
        } else if n.name() == sp::LayerElementVertexCrease.as_ptr() {
            // SAFETY: `vertex_crease_raw()` addresses the mesh's own live
            // `ufbx_vertex_real` field; the `1` value-real count matches it.
            unsafe {
                read_vertex_element(
                    uc,
                    mesh,
                    n,
                    mesh.vertex_crease_raw() as *mut VertexAttrib,
                    sp::VertexCrease.as_ptr(),
                    sp::VertexCreaseIndex.as_ptr(),
                    core::ptr::null(),
                    b'r',
                    1,
                )
            }?;
        } else if n.name() == sp::LayerElementEdgeCrease.as_ptr() {
            // C: `const char *mapping = "";`
            let mut mapping: *const u8 = EMPTY_CHAR.as_ptr();
            // SAFETY: fmt `'c'` pairs with the `*mut *const u8` out-pointer
            // `&mut mapping`, which is a live local.
            ufbxi_ignore!(unsafe {
                find_val1(
                    n,
                    sp::MappingInformationType.as_ptr(),
                    b"c\0".as_ptr(),
                    &mut mapping as *mut *const u8 as *mut c_void,
                )
            });
            if mapping == sp::ByEdge.as_ptr() {
                if mesh.edge_crease().count != 0 {
                    continue;
                }
                // SAFETY: `edge_crease_raw()` addresses the mesh's own live
                // list field, so `&raw mut` projects its `data`/`count` slots;
                // the `b'r'` element format matches their `Real` payload and
                // `mesh.num_edges()` is the truncation limit.
                unsafe {
                    read_truncated_array(
                        uc,
                        &raw mut (*mesh.edge_crease_raw()).data as *mut c_void,
                        &raw mut (*mesh.edge_crease_raw()).count,
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
            // SAFETY: fmt `'c'` pairs with the `*mut *const u8` out-pointer
            // `&mut mapping`, which is a live local.
            ufbxi_ignore!(unsafe {
                find_val1(
                    n,
                    sp::MappingInformationType.as_ptr(),
                    b"c\0".as_ptr(),
                    &mut mapping as *mut *const u8 as *mut c_void,
                )
            });
            if mapping == sp::ByEdge.as_ptr() {
                if mesh.edge_smoothing().count != 0 {
                    continue;
                }
                // SAFETY: `edge_smoothing_raw()` addresses the mesh's own live
                // list field, so `&raw mut` projects its `data`/`count` slots;
                // the `b'b'` element format matches their `bool` payload and
                // `mesh.num_edges()` is the truncation limit.
                unsafe {
                    read_truncated_array(
                        uc,
                        &raw mut (*mesh.edge_smoothing_raw()).data as *mut c_void,
                        &raw mut (*mesh.edge_smoothing_raw()).count,
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
                // SAFETY: `face_smoothing_raw()` addresses the mesh's own live
                // list field, so `&raw mut` projects its `data`/`count` slots;
                // the `b'b'` element format matches their `bool` payload and
                // `mesh.num_faces()` is the truncation limit.
                unsafe {
                    read_truncated_array(
                        uc,
                        &raw mut (*mesh.face_smoothing_raw()).data as *mut c_void,
                        &raw mut (*mesh.face_smoothing_raw()).count,
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
            // SAFETY: fmt `'c'` pairs with the `*mut *const u8` out-pointer
            // `&mut mapping`, which is a live local.
            ufbxi_ignore!(unsafe {
                find_val1(
                    n,
                    sp::MappingInformationType.as_ptr(),
                    b"c\0".as_ptr(),
                    &mut mapping as *mut *const u8 as *mut c_void,
                )
            });
            if mapping == sp::ByEdge.as_ptr() {
                if mesh.edge_visibility().count != 0 {
                    continue;
                }
                // SAFETY: `edge_visibility_raw()` addresses the mesh's own live
                // list field, so `&raw mut` projects its `data`/`count` slots;
                // the `b'b'` element format matches their `bool` payload and
                // `mesh.num_edges()` is the truncation limit.
                unsafe {
                    read_truncated_array(
                        uc,
                        &raw mut (*mesh.edge_visibility_raw()).data as *mut c_void,
                        &raw mut (*mesh.edge_visibility_raw()).count,
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
            // SAFETY: fmt `'c'` pairs with the `*mut *const u8` out-pointer
            // `&mut mapping`, which is a live local.
            ufbxi_ignore!(unsafe {
                find_val1(
                    n,
                    sp::MappingInformationType.as_ptr(),
                    b"c\0".as_ptr(),
                    &mut mapping as *mut *const u8 as *mut c_void,
                )
            });
            if mapping == sp::ByPolygon.as_ptr() {
                // SAFETY: `face_material_raw()` addresses the mesh's own live
                // list field, so `&raw mut` projects its `data`/`count` slots;
                // the `b'i'` element format matches their `u32` payload and
                // `mesh.num_faces()` is the truncation limit.
                unsafe {
                    read_truncated_array(
                        uc,
                        &raw mut (*mesh.face_material_raw()).data as *mut c_void,
                        &raw mut (*mesh.face_material_raw()).count,
                        n,
                        sp::Materials.as_ptr(),
                        b'i',
                        mesh.num_faces(),
                    )
                }?;
            } else if mapping == sp::AllSame.as_ptr() {
                let arr: *mut ValueArray = find_array(n, sp::Materials.as_ptr(), b'i');
                // SAFETY: `arr` is dereferenced only after the non-null test
                // short-circuits, and `find_array` returns a node's own array
                // descriptor, live for as long as the parse tree.
                ufbxi_check!(
                    uc,
                    !arr.is_null() && unsafe { (*arr).size } >= 1,
                    "arr && arr->size >= 1"
                );
                // SAFETY: `arr` is that live descriptor with `size >= 1`, whose
                // `'i'` payload is a run of `size` `u32`s.
                let material: u32 = unsafe { *((*arr).data as *mut u32) };
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
                    // `face_material` is the non-null `count`-long run just
                    // pushed on the result arena.
                    let mut p_mat: *mut u32 = mesh.face_material().data as *mut u32;
                    let p_mat_end = add_ptr(p_mat, mesh.face_material().count);
                    while p_mat != p_mat_end {
                        // SAFETY: `p_mat` is inside that run, short of the end.
                        unsafe { *p_mat = material };
                        // SAFETY: `p_mat` is before `p_mat_end`, so the advance
                        // lands at most one past the run's end.
                        p_mat = unsafe { p_mat.add(1) };
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
            let mut mapping: *const u8 = core::ptr::null();
            // SAFETY: fmt `'c'` pairs with the `*mut *const u8` out-pointer
            // `&mut mapping`, which is a live local.
            ufbxi_check!(
                uc,
                unsafe {
                    find_val1(
                        n,
                        sp::MappingInformationType.as_ptr(),
                        b"c\0".as_ptr(),
                        &mut mapping as *mut *const u8 as *mut c_void,
                    )
                },
                "ufbxi_find_val1(n, ufbxi_MappingInformationType, \"c\", (char**)&mapping)"
            );
            if mapping == sp::ByPolygon.as_ptr() {
                // SAFETY: `face_group_raw()` addresses the mesh's own live
                // list field, so `&raw mut` projects its `data`/`count` slots;
                // the `b'i'` element format matches their `u32` payload and
                // `mesh.num_faces()` is the truncation limit.
                unsafe {
                    read_truncated_array(
                        uc,
                        &raw mut (*mesh.face_group_raw()).data as *mut c_void,
                        &raw mut (*mesh.face_group_raw()).count,
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
            let mut mapping: *const u8 = core::ptr::null();
            // SAFETY: fmt `'c'` pairs with the `*mut *const u8` out-pointer
            // `&mut mapping`, which is a live local.
            ufbxi_check!(
                uc,
                unsafe {
                    find_val1(
                        n,
                        sp::MappingInformationType.as_ptr(),
                        b"c\0".as_ptr(),
                        &mut mapping as *mut *const u8 as *mut c_void,
                    )
                },
                "ufbxi_find_val1(n, ufbxi_MappingInformationType, \"c\", (char**)&mapping)"
            );
            if mapping == sp::ByPolygon.as_ptr() {
                // SAFETY: `face_hole_raw()` addresses the mesh's own live list
                // field, so `&raw mut` projects its `data`/`count` slots; the
                // `b'b'` element format matches their `bool` payload and
                // `mesh.num_faces()` is the truncation limit.
                unsafe {
                    read_truncated_array(
                        uc,
                        &raw mut (*mesh.face_hole_raw()).data as *mut c_void,
                        &raw mut (*mesh.face_hole_raw()).count,
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
            // terminator — the run `memchr` scans.
            ufbxi_check!(
                uc,
                unsafe { memchr(n.name(), b'\0', n.name_len() as usize) }.is_null(),
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
                // SAFETY: `uc.string_pool_mut_ptr()` is uc's own live string
                // pool and `prop_name` is a live local whose `data` spans
                // `length` readable bytes.
                unsafe { push_string_place_str(uc.string_pool_mut_ptr(), &mut prop_name, false) }?;
                // C: `const char *mapping = NULL;`
                let mut mapping: *const u8 = core::ptr::null();
                // SAFETY: fmt `'c'` pairs with the `*mut *const u8` out-pointer
                // `&mut mapping`, which is a live local.
                if unsafe {
                    find_val1(
                        n,
                        sp::MappingInformationType.as_ptr(),
                        b"c\0".as_ptr(),
                        &mut mapping as *mut *const u8 as *mut c_void,
                    )
                } {
                    let arr: *mut ValueArray = find_array(n, sp::TextureId.as_ptr(), b'i');

                    let tex: *mut TmpMeshTexture =
                        uc.tmp_mesh_textures_view().push_zero::<TmpMeshTexture>(1);
                    ufbxi_check!(uc, !tex.is_null(), "tex");
                    if !arr.is_null() {
                        // SAFETY: `tex` is the fresh non-null single-element
                        // push checked above; `arr` is non-null (checked) — a
                        // node's own array descriptor whose `'i'` payload is a
                        // run of `size` `u32`s.
                        unsafe {
                            (*tex).face_texture = (*arr).data as *mut u32;
                            (*tex).num_faces = (*arr).size;
                        }
                    }
                    // SAFETY: `tex` is the fresh non-null single-element push.
                    unsafe {
                        (*tex).prop_name = prop_name;
                        (*tex).all_same = mapping == sp::AllSame.as_ptr();
                    }
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
    // SAFETY: contiguous push_pop child run, valid for `node`'s borrow.
    for n in unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) }
    {
        if n.name() != sp::Layer.as_ptr() {
            continue;
        }
        let mut uv_set: *mut UvSet = core::ptr::null_mut();
        let mut bitangent_layer: *mut TangentLayer = core::ptr::null_mut();
        let mut tangent_layer: *mut TangentLayer = core::ptr::null_mut();

        // C: `ufbxi_for (ufbxi_node, c, n->children, n->num_children)`
        // SAFETY: contiguous push_pop child run, valid for `n`'s borrow.
        for c in unsafe { SliceViewIter::from_raw_parts(n.children(), n.num_children() as usize) } {
            // C: `uint32_t index; const char *type;` — both are fully written by
            // the guarded `ufbxi_find_val1` calls below before any read; zeroed
            // here (no upstream `ufbxi_uninit` marker).
            let mut index: u32 = 0;
            let mut type_: *const u8 = core::ptr::null();
            if c.name() != sp::LayerElement.as_ptr() {
                continue;
            }
            // SAFETY: fmt `'I'` pairs with the `*mut u32` out-pointer
            // `&mut index`, which is a live local.
            if !unsafe {
                find_val1(
                    c,
                    sp::TypedIndex.as_ptr(),
                    b"I\0".as_ptr(),
                    &mut index as *mut u32 as *mut c_void,
                )
            } {
                continue;
            }
            // SAFETY: fmt `'C'` pairs with the `*mut *const u8` out-pointer
            // `&mut type_`, which is a live local.
            if !unsafe {
                find_val1(
                    c,
                    sp::Type.as_ptr(),
                    b"C\0".as_ptr(),
                    &mut type_ as *mut *const u8 as *mut c_void,
                )
            } {
                continue;
            }

            if type_ == sp::LayerElementUV.as_ptr() {
                // C: `ufbxi_for(ufbx_uv_set, set, mesh->uv_sets.data, mesh->uv_sets.count)`
                // `uv_sets` is the mesh's own `count`-long run of `ufbx_uv_set`.
                let mut set: *mut UvSet = mesh.uv_sets().data as *mut UvSet;
                let set_end = add_ptr(set, mesh.uv_sets().count);
                while set != set_end {
                    // SAFETY: `set` is inside that run, short of `set_end`.
                    if unsafe { (*set).index } == index {
                        uv_set = set;
                        break;
                    }
                    // SAFETY: `set` is before `set_end`, so the advance lands at
                    // most one past the run's end.
                    set = unsafe { set.add(1) };
                }
            } else if type_ == sp::LayerElementBinormal.as_ptr() {
                // C: `ufbxi_for(ufbxi_tangent_layer, layer, bitangents, num_bitangents_read)`
                let mut layer: *mut TangentLayer = bitangents;
                let layer_end = add_ptr(layer, num_bitangents_read);
                while layer != layer_end {
                    // SAFETY: `layer` is inside the `bitangents` run, whose
                    // first `num_bitangents_read <= num_bitangents` entries were
                    // filled above, and is short of `layer_end`.
                    if unsafe { (*layer).index } == index {
                        bitangent_layer = layer;
                        break;
                    }
                    // SAFETY: `layer` is before `layer_end`, so the advance
                    // lands at most one past that prefix's end.
                    layer = unsafe { layer.add(1) };
                }
            } else if type_ == sp::LayerElementTangent.as_ptr() {
                // C: `ufbxi_for(ufbxi_tangent_layer, layer, tangents, num_tangents_read)`
                let mut layer: *mut TangentLayer = tangents;
                let layer_end = add_ptr(layer, num_tangents_read);
                while layer != layer_end {
                    // SAFETY: `layer` is inside the `tangents` run, whose first
                    // `num_tangents_read <= num_tangents` entries were filled
                    // above, and is short of `layer_end`.
                    if unsafe { (*layer).index } == index {
                        tangent_layer = layer;
                        break;
                    }
                    // SAFETY: `layer` is before `layer_end`, so the advance
                    // lands at most one past that prefix's end.
                    layer = unsafe { layer.add(1) };
                }
            }
        }

        if !uv_set.is_null() {
            // SAFETY: `uv_set` is non-null (checked) and an in-bounds slot of
            // the mesh's own `uv_sets` run — write-capable provenance.
            let uv_set: &View<UvSet> = unsafe { View::<UvSet>::from_ptr(uv_set) };
            if !bitangent_layer.is_null() {
                // SAFETY: `bitangent_layer` is non-null (checked) and an
                // in-bounds slot of the `bitangents` run; both fields are
                // initialized `ufbx_vertex_vec3`s and the two allocations are
                // disjoint.
                unsafe {
                    core::ptr::write(
                        uv_set.vertex_bitangent_raw(),
                        core::ptr::read(&(*bitangent_layer).elem),
                    );
                }
            }
            if !tangent_layer.is_null() {
                // SAFETY: as above, with `tangent_layer` an in-bounds slot of
                // the `tangents` run.
                unsafe {
                    core::ptr::write(
                        uv_set.vertex_tangent_raw(),
                        core::ptr::read(&(*tangent_layer).elem),
                    );
                }
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
        // SAFETY: `uc.error_mut_ptr()`/`max_consecutive_indices_mut_ptr()` are
        // uc's own live error and counter slots.
        unsafe {
            assign_face_groups(
                uc.result_view(),
                uc.error_mut_ptr(),
                mesh,
                uc.max_consecutive_indices_mut_ptr(),
                uc.retain_mesh_parts(),
            )
        }?;
    }

    // Sort UV and color sets by set index
    // SAFETY: `uv_sets`/`color_sets` span `count` live `ufbx_uv_set` /
    // `ufbx_color_set` values — the two sorts' contract.
    unsafe {
        sort_uv_sets(uc, mesh.uv_sets().data as *mut UvSet, mesh.uv_sets().count)?;
        sort_color_sets(
            uc,
            mesh.color_sets().data as *mut ColorSet,
            mesh.color_sets().count,
        )?;
    }

    if num_textures > 0 {
        // `element.element_id` is the mesh's own id — the element
        // `push_element_extra` attaches to.
        let extra: *mut MeshExtra = push_element_extra(uc, mesh.element().element_id());
        ufbxi_check!(uc, !extra.is_null(), "extra");
        // SAFETY: `extra` is the fresh non-null extra checked above.
        unsafe {
            (*extra).texture_count = num_textures;
            (*extra).texture_arr = uc
                .tmp_view()
                .push_pop::<TmpMeshTexture>(uc.tmp_mesh_textures_view(), num_textures);
        }
        // SAFETY: `extra` is the fresh non-null extra checked above.
        ufbxi_check!(
            uc,
            !unsafe { (*extra).texture_arr }.is_null(),
            "extra->texture_arr"
        );
    }

    // Subdivision

    // SAFETY: fmt `'I'` pairs with the `*mut u32` out-pointer
    // `subdivision_preview_levels_raw()`, a field of the fresh non-null element
    // pushed above.
    ufbxi_ignore!(unsafe {
        find_val1(
            node,
            sp::PreviewDivisionLevels.as_ptr(),
            b"I\0".as_ptr(),
            mesh.subdivision_preview_levels_raw() as *mut c_void,
        )
    });
    // SAFETY: fmt `'I'` pairs with the `*mut u32` out-pointer
    // `subdivision_render_levels_raw()`, a field of the fresh non-null element.
    ufbxi_ignore!(unsafe {
        find_val1(
            node,
            sp::RenderDivisionLevels.as_ptr(),
            b"I\0".as_ptr(),
            mesh.subdivision_render_levels_raw() as *mut c_void,
        )
    });

    // C: `int32_t smoothness, boundary;` — written by the guarded
    // `ufbxi_find_val1` calls below (no upstream `ufbxi_uninit` marker).
    let mut smoothness: i32 = 0;
    let mut boundary: i32 = 0;
    // SAFETY: fmt `'I'` pairs with the `*mut i32` out-pointer `&mut smoothness`,
    // which is a live local.
    if unsafe {
        find_val1(
            node,
            sp::Smoothness.as_ptr(),
            b"I\0".as_ptr(),
            &mut smoothness as *mut i32 as *mut c_void,
        )
    } {
        if smoothness >= 0 && smoothness <= SubdivisionDisplayMode::Smooth as i32 {
            mesh.set_subdivision_display_mode(subdivision_display_mode_from_raw(smoothness));
        }
    }
    // SAFETY: fmt `'I'` pairs with the `*mut i32` out-pointer `&mut boundary`,
    // which is a live local.
    if unsafe {
        find_val1(
            node,
            sp::BoundaryRule.as_ptr(),
            b"I\0".as_ptr(),
            &mut boundary as *mut i32 as *mut c_void,
        )
    } {
        if boundary >= 0 && boundary < SubdivisionBoundary::SharpCorners as i32 {
            mesh.set_subdivision_boundary(subdivision_boundary_from_raw(boundary + 1));
        }
    }

    Ok(())
}

// ufbx.c:13813-13823 `ufbxi_read_nurbs_topology`
#[inline(never)]
pub(crate) unsafe fn read_nurbs_topology(form: *const u8) -> NurbsTopology {
    // SAFETY: `form` is a NUL-terminated string (fn contract) — callers pass the
    // `'C'` value fetched from the parse tree, whose payload is NUL-terminated.
    unsafe {
        if strcmp(form, b"Open\0".as_ptr()) == 0 {
            return NurbsTopology::Open;
        } else if strcmp(form, b"Closed\0".as_ptr()) == 0 {
            return NurbsTopology::Closed;
        } else if strcmp(form, b"Periodic\0".as_ptr()) == 0 {
            return NurbsTopology::Periodic;
        }
    }
    NurbsTopology::Open
}

// ufbx.c:13825-13853 `ufbxi_read_nurbs_curve`
#[inline(never)]
pub(crate) unsafe fn read_nurbs_curve(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `NurbsCurve`
    // is the element struct for `ElementType::NurbsCurve`.
    let nurbs: *mut NurbsCurve =
        unsafe { push_element::<NurbsCurve>(uc, info, ElementType::NurbsCurve) };
    ufbxi_check!(uc, !nurbs.is_null(), "nurbs");

    let mut dimension: i32 = 3;

    let mut form: *const u8 = core::ptr::null();
    // SAFETY: fmt `'I'` pairs with the `*mut u32` out-pointer
    // `&mut (*nurbs).basis.order`, a field of the fresh non-null element pushed
    // above.
    ufbxi_check!(
        uc,
        unsafe {
            find_val1(
                node,
                sp::Order.as_ptr(),
                b"I\0".as_ptr(),
                &mut (*nurbs).basis.order as *mut u32 as *mut c_void,
            )
        },
        "ufbxi_find_val1(node, ufbxi_Order, \"I\", &nurbs->basis.order)"
    );
    // SAFETY: fmt `'I'` pairs with the `*mut i32` out-pointer `&mut dimension`,
    // which is a live local.
    ufbxi_ignore!(unsafe {
        find_val1(
            node,
            sp::Dimension.as_ptr(),
            b"I\0".as_ptr(),
            &mut dimension as *mut i32 as *mut c_void,
        )
    });
    // SAFETY: fmt `'C'` pairs with the `*mut *const u8` out-pointer `&mut form`,
    // which is a live local.
    ufbxi_check!(
        uc,
        unsafe {
            find_val1(
                node,
                sp::Form.as_ptr(),
                b"C\0".as_ptr(),
                &mut form as *mut *const u8 as *mut c_void,
            )
        },
        "ufbxi_find_val1(node, ufbxi_Form, \"C\", (char**)&form)"
    );
    // SAFETY: the `'C'` fetch above succeeded (checked), so `form` points at the
    // NUL-terminated parse-tree string `read_nurbs_topology` requires; `nurbs` is
    // the fresh non-null element pushed above.
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
pub(crate) unsafe fn read_nurbs_surface(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `NurbsSurface`
    // is the element struct for `ElementType::NurbsSurface`.
    let nurbs: *mut NurbsSurface =
        unsafe { push_element::<NurbsSurface>(uc, info, ElementType::NurbsSurface) };
    ufbxi_check!(uc, !nurbs.is_null(), "nurbs");

    let mut form_u: *const u8 = core::ptr::null();
    let mut form_v: *const u8 = core::ptr::null();
    let mut dimension_u: usize = 0;
    let mut dimension_v: usize = 0;
    let mut step_u: i32 = 0;
    let mut step_v: i32 = 0;
    // SAFETY: fmt `"II"` pairs with the two `*mut u32` out-pointers
    // `&mut (*nurbs).basis_u.order` / `&mut (*nurbs).basis_v.order`, fields of the
    // fresh non-null element pushed above.
    ufbxi_check!(
        uc,
        unsafe {
            find_val2(
                node,
                sp::NurbsSurfaceOrder.as_ptr(),
                b"II\0".as_ptr(),
                &mut (*nurbs).basis_u.order as *mut u32 as *mut c_void,
                &mut (*nurbs).basis_v.order as *mut u32 as *mut c_void,
            )
        },
        "ufbxi_find_val2(node, ufbxi_NurbsSurfaceOrder, \"II\", &nurbs->basis_u.order, &nurbs->basis_v.order)"
    );
    // SAFETY: fmt `"ZZ"` pairs with the two `*mut usize` out-pointers
    // `&mut dimension_u` / `&mut dimension_v`, which are live locals.
    ufbxi_check!(
        uc,
        unsafe {
            find_val2(
                node,
                sp::Dimensions.as_ptr(),
                b"ZZ\0".as_ptr(),
                &mut dimension_u as *mut usize as *mut c_void,
                &mut dimension_v as *mut usize as *mut c_void,
            )
        },
        "ufbxi_find_val2(node, ufbxi_Dimensions, \"ZZ\", &dimension_u, &dimension_v)"
    );
    // SAFETY: fmt `"II"` pairs with the two `*mut i32` out-pointers
    // `&mut step_u` / `&mut step_v`, which are live locals.
    ufbxi_check!(
        uc,
        unsafe {
            find_val2(
                node,
                sp::Step.as_ptr(),
                b"II\0".as_ptr(),
                &mut step_u as *mut i32 as *mut c_void,
                &mut step_v as *mut i32 as *mut c_void,
            )
        },
        "ufbxi_find_val2(node, ufbxi_Step, \"II\", &step_u, &step_v)"
    );
    // SAFETY: fmt `"CC"` pairs with the two `*mut *const u8` out-pointers
    // `&mut form_u` / `&mut form_v`, which are live locals.
    ufbxi_check!(
        uc,
        unsafe {
            find_val2(
                node,
                sp::Form.as_ptr(),
                b"CC\0".as_ptr(),
                &mut form_u as *mut *const u8 as *mut c_void,
                &mut form_v as *mut *const u8 as *mut c_void,
            )
        },
        "ufbxi_find_val2(node, ufbxi_Form, \"CC\", (char**)&form_u, (char**)&form_v)"
    );
    // SAFETY: fmt `'B'` pairs with the `*mut bool` out-pointer
    // `&mut (*nurbs).flip_normals`, a field of the fresh non-null element.
    ufbxi_ignore!(unsafe {
        find_val1(
            node,
            sp::FlipNormals.as_ptr(),
            b"B\0".as_ptr(),
            &mut (*nurbs).flip_normals as *mut bool as *mut c_void,
        )
    });
    // SAFETY: the `"CC"` fetch above succeeded (checked), so `form_u`/`form_v`
    // point at NUL-terminated parse-tree strings; `nurbs` is the fresh non-null
    // element pushed above.
    unsafe {
        (*nurbs).basis_u.topology = read_nurbs_topology(form_u);
        (*nurbs).basis_v.topology = read_nurbs_topology(form_v);
    }
    // SAFETY: `nurbs` is the fresh non-null element pushed above.
    unsafe {
        (*nurbs).num_control_points_u = dimension_u;
        (*nurbs).num_control_points_v = dimension_v;
        (*nurbs).span_subdivision_u = if step_u > 0 { step_u as u32 } else { 4u32 };
        (*nurbs).span_subdivision_v = if step_v > 0 { step_v as u32 } else { 4u32 };
    }

    if !uc.opts_view().ignore_geometry() {
        let points: *mut ValueArray = find_array(node, sp::Points.as_ptr(), b'r');
        let knot_u: *mut ValueArray = find_array(node, sp::KnotVectorU.as_ptr(), b'r');
        let knot_v: *mut ValueArray = find_array(node, sp::KnotVectorV.as_ptr(), b'r');
        ufbxi_check!(uc, !points.is_null(), "points");
        ufbxi_check!(uc, !knot_u.is_null(), "knot_u");
        ufbxi_check!(uc, !knot_v.is_null(), "knot_v");
        // SAFETY: `points` is non-null (checked above) and `find_array` returns
        // the node's own array descriptor, live for as long as the parse tree.
        unsafe {
            ufbxi_check!(uc, (*points).size % 4 == 0, "points->size % 4 == 0");
            ufbxi_check!(
                uc,
                (*points).size / 4 == dimension_u.wrapping_mul(dimension_v),
                "points->size / 4 == (size_t)dimension_u * (size_t)dimension_v"
            );
        }

        // SAFETY: `nurbs` is the fresh non-null element; `points`/`knot_u`/
        // `knot_v` are the live array descriptors checked non-null above, whose
        // `'r'` payloads are `size` reals — `points.size` being a multiple of 4
        // (checked) makes it `size / 4` `ufbx_vec4` control points.
        unsafe {
            (*nurbs).control_points.count = (*points).size / 4;
            (*nurbs).control_points.data = (*points).data as *const Vec4;
            (*nurbs).basis_u.knot_vector.data = (*knot_u).data as *const Real;
            (*nurbs).basis_u.knot_vector.count = (*knot_u).size;
            (*nurbs).basis_v.knot_vector.data = (*knot_v).data as *const Real;
            (*nurbs).basis_v.knot_vector.count = (*knot_v).size;
        }
    }

    Ok(())
}

// ufbx.c:13896-13955 `ufbxi_read_line`
#[inline(never)]
pub(crate) unsafe fn read_line(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `LineCurve` is
    // the element struct for `ElementType::LineCurve`.
    let line: *mut LineCurve =
        unsafe { push_element::<LineCurve>(uc, info, ElementType::LineCurve) };
    ufbxi_check!(uc, !line.is_null(), "line");

    if !uc.opts_view().ignore_geometry() {
        let points: *mut ValueArray = find_array(node, sp::Points.as_ptr(), b'r');
        let points_index: *mut ValueArray = find_array(node, sp::PointsIndex.as_ptr(), b'i');
        ufbxi_check!(uc, !points.is_null(), "points");
        ufbxi_check!(uc, !points_index.is_null(), "points_index");
        // SAFETY: `points` is non-null (checked above) and `find_array` returns
        // the node's own array descriptor, live for as long as the parse tree.
        ufbxi_check!(
            uc,
            unsafe { (*points).size } % 3 == 0,
            "points->size % 3 == 0"
        );

        // SAFETY: as above.
        if unsafe { (*points).size } > 0 {
            // SAFETY: `line` is the fresh non-null element pushed above;
            // `points`/`points_index` are the live array descriptors checked
            // non-null above, whose `'r'`/`'i'` payloads are `size` reals and
            // `size` `u32`s — `points.size` being a multiple of 3 (checked) makes
            // it `size / 3` `ufbx_vec3` control points.
            unsafe {
                (*line).control_points.count = (*points).size / 3;
                (*line).control_points.data = (*points).data as *const Vec3;
                (*line).point_indices.count = (*points_index).size;
                (*line).point_indices.data = (*points_index).data as *const u32;
            }

            // SAFETY: `line` is the fresh non-null element.
            ufbxi_check!(
                uc,
                unsafe { (*line).control_points.count } < i32::MAX as usize,
                "line->control_points.count < INT32_MAX"
            );

            // Count end points
            let mut num_segments: usize = 1;
            // SAFETY: `line` is the fresh non-null element.
            if unsafe { (*line).point_indices.count } > 0 {
                // SAFETY: as above.
                for i in 0..unsafe { (*line).point_indices.count } - 1 {
                    // SAFETY: `point_indices` was set above to the
                    // `points_index` payload of `count` `u32`s and
                    // `i < count - 1`, so the read is in bounds.
                    let ix: u32 = unsafe { *(*line).point_indices.data.add(i) };
                    num_segments =
                        num_segments.wrapping_add(if (ix as i32) < 0 { 1usize } else { 0usize });
                }
            }

            let mut prev_end: usize = 0;
            // SAFETY: `line` is the fresh non-null element and `result` is uc's
            // own result buffer, so the freshly pushed `num_segments` run is
            // owned by the scene being built.
            unsafe {
                (*line).segments.data =
                    uc.result_view().push::<LineSegment>(num_segments) as *const LineSegment;
            }
            // SAFETY: `line` is the fresh non-null element.
            ufbxi_check!(
                uc,
                !unsafe { (*line).segments.data }.is_null(),
                "line->segments.data"
            );
            // SAFETY: `line` is the fresh non-null element.
            for i in 0..unsafe { (*line).point_indices.count } {
                // SAFETY: `point_indices` spans `count` `u32`s and `i < count`.
                let mut ix: u32 = unsafe { *(*line).point_indices.data.add(i) };
                if (ix as i32) < 0 {
                    ix = !ix;
                    // SAFETY: `line` is the fresh non-null element.
                    if i + 1 < unsafe { (*line).point_indices.count } {
                        // C: `&line->segments.data[line->segments.count++]` —
                        // the index uses the pre-increment value.
                        // SAFETY: `segments.data` is the non-null `num_segments`
                        // run allocated above; this branch is taken once per
                        // negative index bar the last, so `segments.count` stays
                        // below `num_segments` and the offset is in bounds.
                        let segment: *mut LineSegment = unsafe {
                            ((*line).segments.data as *mut LineSegment).add((*line).segments.count)
                        };
                        // SAFETY: `line` is the fresh non-null element and
                        // `segment` is the in-bounds slot just computed.
                        unsafe {
                            (*line).segments.count += 1;
                            (*segment).index_begin = prev_end as u32;
                            (*segment).num_indices = i.wrapping_sub(prev_end) as u32;
                        }
                        prev_end = i;
                    }
                }

                // SAFETY: `line` is the fresh non-null element.
                if (ix as usize) < unsafe { (*line).control_points.count } {
                    // SAFETY: `point_indices` points into the parse tree's own
                    // mutable `'i'` payload of `count` `u32`s and `i < count`, so
                    // the const-to-mut cast writes back through its original
                    // provenance, in bounds.
                    unsafe { *((*line).point_indices.data as *mut u32).add(i) = ix };
                } else {
                    // SAFETY: as above — the `i`-th index slot of the
                    // `point_indices` payload is a live, writable `u32`.
                    unsafe {
                        fix_index(
                            uc,
                            ((*line).point_indices.data as *mut u32).add(i),
                            ix,
                            (*line).control_points.count,
                        )
                    }?;
                }
            }

            // SAFETY: `segments.data` is the non-null `num_segments` run and the
            // loop above consumed at most `num_segments - 1` slots, so this final
            // slot is in bounds.
            let segment: *mut LineSegment =
                unsafe { ((*line).segments.data as *mut LineSegment).add((*line).segments.count) };
            // SAFETY: `line` is the fresh non-null element and `segment` is the
            // in-bounds slot just computed.
            unsafe {
                (*line).segments.count += 1;
                (*segment).index_begin = prev_end as u32;
                (*segment).num_indices =
                    to_size((*line).point_indices.count.wrapping_sub(prev_end) as isize) as u32;
            }
            // SAFETY: `line` is the fresh non-null element.
            ufbx_assert!(unsafe { (*line).segments.count } == num_segments);
        }
    }

    Ok(())
}

// ufbx.c:13957-13963 `ufbxi_read_transform_matrix`
#[inline(never)]
pub(crate) unsafe fn read_transform_matrix(m: *mut Matrix, data: *mut Real) {
    // SAFETY: `m` points at a live `ufbx_matrix` and `data` at a run of at least
    // 16 reals — the 4x4 column-major transform its callers check for before
    // calling (fn contract).
    unsafe {
        (*m).m00 = *data.add(0);
        (*m).m10 = *data.add(1);
        (*m).m20 = *data.add(2);
        (*m).m01 = *data.add(4);
        (*m).m11 = *data.add(5);
        (*m).m21 = *data.add(6);
        (*m).m02 = *data.add(8);
        (*m).m12 = *data.add(9);
        (*m).m22 = *data.add(10);
        (*m).m03 = *data.add(12);
        (*m).m13 = *data.add(13);
        (*m).m23 = *data.add(14);
    }
}

// ufbx.c:13965-13977 `ufbxi_read_bone`
#[inline(never)]
pub(crate) unsafe fn read_bone(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
    sub_type: *const u8,
) -> Result<(), Fail> {
    let _ = node; // C: `(void)node;`

    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Bone` is the
    // element struct for `ElementType::Bone`.
    let bone: *mut Bone = unsafe { push_element::<Bone>(uc, info, ElementType::Bone) };
    ufbxi_check!(uc, !bone.is_null(), "bone");

    if sub_type == sp::Root.as_ptr() {
        // SAFETY: `bone` is the fresh non-null element pushed above.
        unsafe {
            (*bone).is_root = true;
        }
    }

    Ok(())
}

// ufbx.c:13979-13990 `ufbxi_read_marker`
#[inline(never)]
pub(crate) unsafe fn read_marker(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
    sub_type: *const u8,
    type_: MarkerType,
) -> Result<(), Fail> {
    let _ = node; // C: `(void)node;`
    let _ = sub_type; // C: `(void)sub_type;`

    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Marker` is the
    // element struct for `ElementType::Marker`.
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
pub(crate) unsafe fn read_skin(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `SkinDeformer`
    // is the element struct for `ElementType::SkinDeformer`.
    let skin: *mut SkinDeformer =
        unsafe { push_element::<SkinDeformer>(uc, info, ElementType::SkinDeformer) };
    ufbxi_check!(uc, !skin.is_null(), "skin");
    // SAFETY: `skin` is the fresh non-null element pushed above, owned by uc's
    // element buffer — write-capable provenance.
    let skin: &View<SkinDeformer> = unsafe { View::<SkinDeformer>::from_ptr(skin) };

    let mut skinning_type: *const u8 = core::ptr::null();
    // SAFETY: fmt `'C'` pairs with the `*mut *const u8` out-pointer
    // `&mut skinning_type`, which is a live local.
    if unsafe {
        find_val1(
            node,
            sp::SkinningType.as_ptr(),
            b"C\0".as_ptr(),
            &mut skinning_type as *mut *const u8 as *mut c_void,
        )
    } {
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
pub(crate) unsafe fn read_skin_cluster(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `SkinCluster`
    // is the element struct for `ElementType::SkinCluster`.
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

        // SAFETY: `cluster` is the fresh non-null element, so the borrows
        // address its live `ufbx_matrix` fields; the `transform` and
        // `transform_link` payloads each hold `size >= 16` reals (just checked),
        // the runs `read_transform_matrix` requires.
        unsafe {
            read_transform_matrix(
                &mut (*cluster).mesh_node_to_bone,
                (*transform).data as *mut Real,
            );
            read_transform_matrix(
                &mut (*cluster).bind_to_world,
                (*transform_link).data as *mut Real,
            );
        }
    }

    Ok(())
}

// ufbx.c:14054-14086 `ufbxi_read_blend_channel`
#[inline(never)]
pub(crate) unsafe fn read_blend_channel(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `BlendChannel`
    // is the element struct for `ElementType::BlendChannel`.
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
                (*shape_props.add(0))._internal_key = get_name_key_c(sp::DeformPercent.as_ptr());
                (*shape_props.add(0)).type_ = PropType::Number;
                (*shape_props.add(0)).value_str = EMPTY_STRING.0;
                // C-parity: `shape_props[0].value_real` is the `ufbx_prop` value
                // union's first real (`value_vec4.x` in the generated struct).
                (*shape_props.add(0)).value_vec4.x = 100.0 as Real;
            }
            // SAFETY: fmt `'R'` pairs with the `*mut Real` out-pointer
            // `&mut (*shape_props.add(0)).value_vec4.x`, a field of the
            // one-element run's only entry.
            ufbxi_ignore!(unsafe {
                get_val1(
                    deform_percent,
                    b"R\0".as_ptr(),
                    &mut (*shape_props.add(0)).value_vec4.x as *mut Real as *mut c_void,
                )
            });
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
pub(crate) unsafe fn solve_tcb(
    p_slope_left: *mut f32,
    p_slope_right: *mut f32,
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

    // SAFETY: `p_slope_left` is a live, writable `f32` out-slot (fn contract).
    unsafe { *p_slope_left = (d00 * slope_left + d01 * slope_right) as f32 };
    // SAFETY: `p_slope_right` is a live, writable `f32` out-slot (fn contract).
    unsafe { *p_slope_right = (d10 * slope_left + d11 * slope_right) as f32 };
}

// ufbx.c:14227-14255 `ufbxi_read_extrapolation`
#[inline(never)]
pub(crate) unsafe fn read_extrapolation(
    p_extrapolation: *mut Extrapolation,
    node: &NodeView,
    name: *const u8,
) {
    let child = find_child(node, name);
    let mut mode: ExtrapolationMode = ExtrapolationMode::Constant;
    let mut repeat_count: i32 = -1;

    if let Some(child) = child {
        // C: `int32_t mode_ch;` — uninitialized, only read when
        // `ufbxi_find_val1()` succeeded and wrote it.
        let mut mode_ch: i32 = 0;
        // SAFETY: fmt `'I'` pairs with the `*mut i32` out-pointer `&mut mode_ch`,
        // which is a live local.
        if unsafe {
            find_val1(
                child,
                sp::Type.as_ptr(),
                b"I\0".as_ptr(),
                &mut mode_ch as *mut i32 as *mut c_void,
            )
        } {
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
            // SAFETY: fmt `'I'` pairs with the `*mut i32` out-pointer
            // `&mut repeat_count`, which is a live local.
            if unsafe {
                find_val1(
                    child,
                    sp::Repetition.as_ptr(),
                    b"I\0".as_ptr(),
                    &mut repeat_count as *mut i32 as *mut c_void,
                )
            } {
                if repeat_count < 0 {
                    repeat_count = -1;
                }
            }
        }
    }

    // SAFETY: `p_extrapolation` is a live, writable `ufbx_extrapolation` out-slot
    // (fn contract).
    unsafe {
        (*p_extrapolation).mode = mode;
        (*p_extrapolation).repeat_count = repeat_count;
    }
}

// ufbx.c:14257-14532 `ufbxi_read_animation_curve`
#[inline(never)]
pub(crate) unsafe fn read_animation_curve(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `AnimCurve` is
    // the element struct for `ElementType::AnimCurve`.
    let curve: *mut AnimCurve =
        unsafe { push_element::<AnimCurve>(uc, info, ElementType::AnimCurve) };
    ufbxi_check!(uc, !curve.is_null(), "curve");

    // SAFETY: `curve` is the fresh non-null element pushed above, so
    // `&mut (*curve).pre_extrapolation` is the live out-slot `read_extrapolation`
    // requires; `sp::Pre_Extrapolation` is a NUL-terminated static name.
    unsafe {
        read_extrapolation(
            &mut (*curve).pre_extrapolation,
            node,
            sp::Pre_Extrapolation.as_ptr(),
        );
        read_extrapolation(
            &mut (*curve).post_extrapolation,
            node,
            sp::Post_Extrapolation.as_ptr(),
        );
    }

    if uc.opts_view().ignore_animation() {
        return Ok(());
    }

    // C: `ufbxi_value_array *times, *values, *attr_flags, *attrs, *refs;`
    // — declared uninitialized, each written by the `ufbxi_check(x = ...)`
    // assignment-in-condition below.
    let times: *mut ValueArray = find_array(node, sp::KeyTime.as_ptr(), b'l');
    ufbxi_check!(
        uc,
        !times.is_null(),
        "times = ufbxi_find_array(node, ufbxi_KeyTime, 'l')"
    );
    let values: *mut ValueArray = find_array(node, sp::KeyValueFloat.as_ptr(), b'r');
    ufbxi_check!(
        uc,
        !values.is_null(),
        "values = ufbxi_find_array(node, ufbxi_KeyValueFloat, 'r')"
    );
    let attr_flags: *mut ValueArray = find_array(node, sp::KeyAttrFlags.as_ptr(), b'i');
    ufbxi_check!(
        uc,
        !attr_flags.is_null(),
        "attr_flags = ufbxi_find_array(node, ufbxi_KeyAttrFlags, 'i')"
    );
    let attrs: *mut ValueArray = find_array(node, sp::KeyAttrDataFloat.as_ptr(), b'?');
    ufbxi_check!(
        uc,
        !attrs.is_null(),
        "attrs = ufbxi_find_array(node, ufbxi_KeyAttrDataFloat, '?')"
    );
    let refs: *mut ValueArray = find_array(node, sp::KeyAttrRefCount.as_ptr(), b'i');
    ufbxi_check!(
        uc,
        !refs.is_null(),
        "refs = ufbxi_find_array(node, ufbxi_KeyAttrRefCount, 'i')"
    );

    // Time and value arrays that define the keyframes should be parallel
    // SAFETY: `times`/`values` are non-null (checked above) and `find_array`
    // returns the node's own array descriptors, live for as long as the parse
    // tree.
    ufbxi_check!(
        uc,
        unsafe { (*times).size } == unsafe { (*values).size },
        "times->size == values->size"
    );

    // Flags and attributes are run-length encoded where KeyAttrRefCount (refs)
    // is an array that describes how many times to repeat a given flag/attribute.
    // Attributes consist of 4 32-bit floating point values per key.
    // SAFETY: `attr_flags`/`refs` are the live array descriptors checked non-null
    // above.
    ufbxi_check!(
        uc,
        unsafe { (*attr_flags).size } == unsafe { (*refs).size },
        "attr_flags->size == refs->size"
    );
    // SAFETY: `attrs`/`refs` are the live array descriptors checked non-null
    // above.
    ufbxi_check!(
        uc,
        unsafe { (*attrs).size } == unsafe { (*refs).size }.wrapping_mul(4u32 as usize),
        "attrs->size == refs->size * 4u"
    );

    // SAFETY: `times` is the live array descriptor checked non-null above.
    let num_keys: usize = unsafe { (*times).size };
    let keys: *mut Keyframe = uc.result_view().push::<Keyframe>(num_keys);
    ufbxi_check!(uc, !keys.is_null(), "keys");

    // SAFETY: `curve` is the fresh non-null element and `keys` is the non-null
    // `num_keys` run just pushed on the result buffer.
    unsafe {
        (*curve).keyframes.data = keys;
        (*curve).keyframes.count = num_keys;
    }

    // SAFETY: each descriptor was checked non-null above and is live for as long
    // as the parse tree; the `'l'`/`'r'`/`'i'`/`'?'` payloads are runs of `size`
    // `i64`s, reals, `i32`s and `f32`s respectively.
    let (mut p_time, mut p_value, mut p_flag, mut p_attr, mut p_ref) = unsafe {
        (
            (*times).data as *mut i64,
            (*values).data as *mut Real,
            (*attr_flags).data as *mut i32,
            (*attrs).data as *mut f32,
            (*refs).data as *mut i32,
        )
    };
    // SAFETY: as above — `refs.size` is exactly the length of the `p_ref` run, so
    // the one-past-the-end pointer is in range.
    let p_ref_end: *mut i32 = unsafe { add_ptr(p_ref, (*refs).size) };

    // The previous key defines the weight/slope of the left tangent
    let mut slope_left: f32 = 0.0f32;
    let mut weight_left: f32 = 0.333333f32;
    // float velocity_left = 0.0f;

    let mut prev_time: f64 = 0.0;
    let mut next_time: f64 = 0.0;

    let mut refs_left: i32 = 0;
    if num_keys > 0 {
        // SAFETY: `p_time` heads the `times` payload of `num_keys` `i64`s, which
        // is non-empty in this branch.
        next_time = unsafe { *p_time.add(0) } as f64 / uc.ktime_sec_double();
        if p_ref < p_ref_end {
            // SAFETY: `p_ref` is below `p_ref_end`, so it addresses a live entry
            // of the `refs` payload.
            refs_left = unsafe { *p_ref };
        }
    }

    let mut i: usize = 0;
    while i < num_keys {
        // SAFETY: `keys` is the non-null `num_keys` run pushed above and
        // `i < num_keys`.
        let key: *mut Keyframe = unsafe { keys.add(i) };
        ufbxi_check!(uc, refs_left > 0, "refs_left > 0");

        // SAFETY: `p_value` walks the `values` payload one step per iteration
        // from its start, so at iteration `i < num_keys` it addresses element
        // `i` of that `num_keys`-long run.
        let value: Real = unsafe { *p_value };
        if i == 0 {
            // SAFETY: `curve` is the fresh non-null element pushed above.
            unsafe {
                (*curve).min_value = value;
                (*curve).max_value = value;
            }
        } else {
            // SAFETY: `curve` is the fresh non-null element.
            unsafe {
                (*curve).min_value = min_real((*curve).min_value, value);
                (*curve).max_value = max_real((*curve).max_value, value);
            }
        }

        // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
        unsafe {
            (*key).time = next_time;
            (*key).value = value;
        }

        if i + 1 < num_keys {
            // SAFETY: `p_time` addresses element `i` of the `num_keys`-long
            // `times` payload, and `i + 1 < num_keys` bounds the next one.
            next_time = unsafe { *p_time.add(1) } as f64 / uc.ktime_sec_double();
        }

        // SAFETY: `refs_left > 0` (checked above) holds only while `p_ref` is
        // still below `p_ref_end`, and `p_flag` advances in lockstep with `p_ref`
        // over the equally long `attr_flags` payload, so it is in bounds.
        let flags: u32 = unsafe { *p_flag } as u32;

        // SAFETY: as above, `p_attr` advances four floats per `p_ref` step over
        // the `attrs` payload of `refs.size * 4` floats, so the current key's
        // four-float group is in bounds.
        let mut slope_right: f32 = unsafe { *p_attr.add(0) };
        let mut weight_right: f32 = 0.333333f32;
        //float velocity_right = 0.0f;
        // SAFETY: as above — the second float of the current group.
        let mut next_slope_left: f32 = unsafe { *p_attr.add(1) };
        let mut next_weight_left: f32 = 0.333333f32;
        // float next_velocity_left = 0.0f;

        if (flags & (KEY_WEIGHTED_RIGHT | KEY_WEIGHTED_NEXT_LEFT)) != 0 {
            // At least one of the tangents is weighted. The weights are encoded as
            // two 0.4 _decimal_ fixed point values that are packed into 32 bits and
            // interpreted as a 32-bit float.
            // C: `uint32_t packed_weights;` + `memcpy(&packed_weights, &p_attr[2], sizeof(uint32_t));`
            // SAFETY: `p_attr`'s four-float group is in bounds (see the group
            // reads above), so its third float is a readable four-byte slot;
            // `read_unaligned` copies those bytes without an alignment claim.
            let packed_weights: u32 = unsafe { (p_attr.add(2) as *const u32).read_unaligned() };

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
                // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
                unsafe {
                    (*key).interpolation = Interpolation::ConstantNext;
                }
            } else {
                // Take constant value from the previous key
                // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
                unsafe {
                    (*key).interpolation = Interpolation::ConstantPrev;
                }
            }

            // C: `weight_right = next_weight_left = 0.333333f;`
            next_weight_left = 0.333333f32;
            weight_right = next_weight_left;
            // C: `slope_right = next_slope_left = 0.0f;`
            next_slope_left = 0.0f32;
            slope_right = next_slope_left;
        } else if flags & KEY_INTERPOLATION_CUBIC != 0 {
            // Cubic interpolation
            // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
            unsafe {
                (*key).interpolation = Interpolation::Cubic;
            }

            if flags & KEY_TANGENT_TCB != 0 {
                let mut tcb_slope_left: f64 = 0.0;
                let mut tcb_slope_right: f64 = 0.0;
                let mut tcb_edge: bool = false;
                // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
                if i > 0 && unsafe { (*key).time } > prev_time {
                    // SAFETY: `key` is in bounds, and `p_value` addresses element
                    // `i` of the `values` payload with `i > 0`, so the preceding
                    // element is in bounds too.
                    tcb_slope_left = unsafe {
                        as_f64!((*key).value - *p_value.offset(-1)) / ((*key).time - prev_time)
                    };
                } else {
                    tcb_edge = true;
                }
                // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
                if i + 1 < num_keys && next_time > unsafe { (*key).time } {
                    // SAFETY: `key` is in bounds, and `i + 1 < num_keys` bounds
                    // the next element of the `values` payload.
                    tcb_slope_right = unsafe {
                        as_f64!(*p_value.add(1) - (*key).value) / (next_time - (*key).time)
                    };
                } else {
                    tcb_edge = true;
                }

                // SAFETY: `slope_left`/`slope_right` are live `f32` locals, the
                // out-slots `solve_tcb` writes; `p_attr`'s four-float group is in
                // bounds (see the group reads above).
                unsafe {
                    solve_tcb(
                        &mut slope_left,
                        &mut slope_right,
                        *p_attr.add(0) as f64,
                        *p_attr.add(1) as f64,
                        *p_attr.add(2) as f64,
                        tcb_slope_left,
                        tcb_slope_right,
                        tcb_edge,
                    )
                };

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

                // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
                if unsafe {
                    i > 0 && i + 1 < num_keys && (*key).time > prev_time && next_time > (*key).time
                } {
                    if math::fabs((slope_left + slope_right) as f64) <= 0.0001f32 as f64 {
                        // C: `slope_left = slope_right = ufbxi_solve_auto_tangent(...)`
                        // SAFETY: `key` is in bounds, and `i > 0` /
                        // `i + 1 < num_keys` bound the neighbouring elements of
                        // the `values` payload around `p_value`'s element `i`.
                        slope_right = unsafe {
                            solve_auto_tangent(
                                uc,
                                prev_time,
                                (*key).time,
                                next_time,
                                *p_value.offset(-1),
                                (*key).value,
                                *p_value.add(1),
                                weight_left,
                                weight_right,
                                slope_right,
                                flags,
                            )
                        };
                        slope_left = slope_right;
                    } else {
                        // SAFETY: as above.
                        unsafe {
                            slope_left = solve_auto_tangent(
                                uc,
                                prev_time,
                                (*key).time,
                                next_time,
                                *p_value.offset(-1),
                                (*key).value,
                                *p_value.add(1),
                                weight_left,
                                weight_right,
                                -slope_left,
                                flags,
                            );
                            slope_right = solve_auto_tangent(
                                uc,
                                prev_time,
                                (*key).time,
                                next_time,
                                *p_value.offset(-1),
                                (*key).value,
                                *p_value.add(1),
                                weight_left,
                                weight_right,
                                slope_right,
                                flags,
                            );
                        }
                    }
                // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
                } else if i > 0 && unsafe { (*key).time } > prev_time {
                    // C: `slope_left = slope_right = ufbxi_solve_auto_tangent_left(...)`
                    // SAFETY: `key` is in bounds and `i > 0` bounds the element
                    // preceding `p_value`'s element `i` of the `values` payload.
                    slope_right = unsafe {
                        solve_auto_tangent_left(
                            uc,
                            prev_time,
                            (*key).time,
                            *p_value.offset(-1),
                            (*key).value,
                            weight_left,
                            -slope_left,
                            flags,
                        )
                    };
                    slope_left = slope_right;
                // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
                } else if i + 1 < num_keys && next_time > unsafe { (*key).time } {
                    // C: `slope_left = slope_right = ufbxi_solve_auto_tangent_right(...)`
                    // SAFETY: `key` is in bounds and `i + 1 < num_keys` bounds the
                    // element following `p_value`'s element `i`.
                    slope_right = unsafe {
                        solve_auto_tangent_right(
                            uc,
                            (*key).time,
                            next_time,
                            (*key).value,
                            *p_value.add(1),
                            weight_right,
                            slope_right,
                            flags,
                        )
                    };
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
            // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
            unsafe {
                (*key).interpolation = Interpolation::Linear;
            }

            weight_right = 0.333333f32;
            next_weight_left = 0.333333f32;

            // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
            if next_time > unsafe { (*key).time } {
                // SAFETY: `key` is in bounds.
                let delta_time: f64 = next_time - unsafe { (*key).time };
                if delta_time > 0.0 {
                    // SAFETY: `key` is in bounds; `next_time` still equals
                    // `(*key).time` on the last key (it is only advanced while
                    // `i + 1 < num_keys`), so reaching this branch implies
                    // `i + 1 < num_keys`, bounding `p_value`'s next element.
                    let slope: f64 =
                        unsafe { as_f64!(*p_value.add(1) - (*key).value) } / delta_time;
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
        // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
        if unsafe { (*key).time } > prev_time {
            // SAFETY: `key` is in bounds.
            let delta: f64 = unsafe { (*key).time } - prev_time;
            // SAFETY: `key` is in bounds.
            unsafe {
                (*key).left.dx = (weight_left as f64 * delta) as f32;
                (*key).left.dy = (*key).left.dx * slope_left;
            }
        } else {
            // SAFETY: `key` is in bounds.
            unsafe {
                (*key).left.dx = 0.0f32;
                (*key).left.dy = 0.0f32;
            }
        }

        // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
        if next_time > unsafe { (*key).time } {
            // SAFETY: `key` is in bounds.
            let delta: f64 = next_time - unsafe { (*key).time };
            // SAFETY: `key` is in bounds.
            unsafe {
                (*key).right.dx = (weight_right as f64 * delta) as f32;
                (*key).right.dy = (*key).right.dx * slope_right;
            }
        } else {
            // SAFETY: `key` is in bounds.
            unsafe {
                (*key).right.dx = 0.0f32;
                (*key).right.dy = 0.0f32;
            }
        }

        slope_left = next_slope_left;
        weight_left = next_weight_left;
        // velocity_left = next_velocity_left;
        // SAFETY: `key` is the in-bounds `i`-th slot of the `keys` run.
        prev_time = unsafe { (*key).time };

        // Decrement attribute refcount and potentially move to the next one.
        // C: `if (--refs_left == 0)`
        refs_left = refs_left.wrapping_sub(1);
        if refs_left == 0 {
            // SAFETY: the run this iteration consumed was in bounds (see the
            // `refs_left > 0` check), so stepping each cursor one run forward
            // lands at most one past the end of its payload — the `p_ref`
            // comparison below re-establishes in-bounds before any read.
            unsafe {
                p_flag = p_flag.add(1);
                p_attr = p_attr.add(4);
                p_ref = p_ref.add(1);
            }
            if p_ref < p_ref_end {
                // SAFETY: `p_ref` is below `p_ref_end`, so it addresses a live
                // entry of the `refs` payload.
                refs_left = unsafe { *p_ref };
            }
        }
        // SAFETY: `p_time`/`p_value` address element `i` of their `num_keys`-long
        // payloads, so stepping one forward lands at most one past the end — the
        // `i < num_keys` loop condition re-establishes in-bounds before any read.
        unsafe {
            p_time = p_time.add(1);
            p_value = p_value.add(1);
        }

        i += 1;
    }

    Ok(())
}

// ufbx.c:14534-14546 `ufbxi_read_material`
#[inline(never)]
pub(crate) unsafe fn read_material(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Material` is
    // the element struct for `ElementType::Material`.
    let material: *mut Material =
        unsafe { push_element::<Material>(uc, info, ElementType::Material) };
    ufbxi_check!(uc, !material.is_null(), "material");

    // SAFETY: fmt `'S'` pairs with the `*mut String` out-pointer
    // `&mut (*material).shading_model_name`, a field of the fresh non-null
    // element pushed above.
    if !unsafe {
        find_val1(
            node,
            sp::ShadingModel.as_ptr(),
            b"S\0".as_ptr(),
            &mut (*material).shading_model_name as *mut String as *mut c_void,
        )
    } {
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
pub(crate) unsafe fn read_texture(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Texture` is
    // the element struct for `ElementType::Texture`.
    let texture: *mut Texture = unsafe { push_element::<Texture>(uc, info, ElementType::Texture) };
    ufbxi_check!(uc, !texture.is_null(), "texture");

    // SAFETY: `texture` is the fresh non-null element pushed above.
    unsafe {
        (*texture).type_ = TextureType::File;

        (*texture).filename = EMPTY_STRING.0;
        (*texture).absolute_filename = EMPTY_STRING.0;
        (*texture).relative_filename = EMPTY_STRING.0;
    }

    // SAFETY: fmt `'S'` pairs with the `*mut String` out-pointer
    // `&mut (*texture).absolute_filename`, a field of the fresh non-null element.
    unsafe {
        ufbxi_ignore!(find_val1(
            node,
            sp::FileName.as_ptr(),
            b"S\0".as_ptr(),
            &mut (*texture).absolute_filename as *mut String as *mut c_void,
        ));
        ufbxi_ignore!(find_val1(
            node,
            sp::Filename.as_ptr(),
            b"S\0".as_ptr(),
            &mut (*texture).absolute_filename as *mut String as *mut c_void,
        ));
    }
    // SAFETY: as above, with the `*mut String` out-pointer
    // `&mut (*texture).relative_filename`.
    unsafe {
        ufbxi_ignore!(find_val1(
            node,
            sp::RelativeFileName.as_ptr(),
            b"S\0".as_ptr(),
            &mut (*texture).relative_filename as *mut String as *mut c_void,
        ));
        ufbxi_ignore!(find_val1(
            node,
            sp::RelativeFilename.as_ptr(),
            b"S\0".as_ptr(),
            &mut (*texture).relative_filename as *mut String as *mut c_void,
        ));
    }

    // SAFETY: fmt `'b'` pairs with the `*mut Blob` out-pointer
    // `&mut (*texture).raw_absolute_filename`, a field of the fresh non-null
    // element.
    unsafe {
        ufbxi_ignore!(find_val1(
            node,
            sp::FileName.as_ptr(),
            b"b\0".as_ptr(),
            &mut (*texture).raw_absolute_filename as *mut Blob as *mut c_void,
        ));
        ufbxi_ignore!(find_val1(
            node,
            sp::Filename.as_ptr(),
            b"b\0".as_ptr(),
            &mut (*texture).raw_absolute_filename as *mut Blob as *mut c_void,
        ));
    }
    // SAFETY: as above, with the `*mut Blob` out-pointer
    // `&mut (*texture).raw_relative_filename`.
    unsafe {
        ufbxi_ignore!(find_val1(
            node,
            sp::RelativeFileName.as_ptr(),
            b"b\0".as_ptr(),
            &mut (*texture).raw_relative_filename as *mut Blob as *mut c_void,
        ));
        ufbxi_ignore!(find_val1(
            node,
            sp::RelativeFilename.as_ptr(),
            b"b\0".as_ptr(),
            &mut (*texture).raw_relative_filename as *mut Blob as *mut c_void,
        ));
    }

    Ok(())
}

// ufbx.c:14572-14599 `ufbxi_read_layered_texture`
#[inline(never)]
pub(crate) unsafe fn read_layered_texture(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Texture` is
    // the element struct for `ElementType::Texture`.
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
pub(crate) unsafe fn read_video(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Video` is the
    // element struct for `ElementType::Video`.
    let video: *mut Video = unsafe { push_element::<Video>(uc, info, ElementType::Video) };
    ufbxi_check!(uc, !video.is_null(), "video");

    // SAFETY: `video` is the fresh non-null element pushed above.
    unsafe {
        (*video).filename = EMPTY_STRING.0;
        (*video).absolute_filename = EMPTY_STRING.0;
        (*video).relative_filename = EMPTY_STRING.0;
    }

    // SAFETY: fmt `'S'` pairs with the `*mut String` out-pointer
    // `&mut (*video).absolute_filename`, a field of the fresh non-null element.
    unsafe {
        ufbxi_ignore!(find_val1(
            node,
            sp::FileName.as_ptr(),
            b"S\0".as_ptr(),
            &mut (*video).absolute_filename as *mut String as *mut c_void,
        ));
        ufbxi_ignore!(find_val1(
            node,
            sp::Filename.as_ptr(),
            b"S\0".as_ptr(),
            &mut (*video).absolute_filename as *mut String as *mut c_void,
        ));
    }
    // SAFETY: as above, with the `*mut String` out-pointer
    // `&mut (*video).relative_filename`.
    unsafe {
        ufbxi_ignore!(find_val1(
            node,
            sp::RelativeFileName.as_ptr(),
            b"S\0".as_ptr(),
            &mut (*video).relative_filename as *mut String as *mut c_void,
        ));
        ufbxi_ignore!(find_val1(
            node,
            sp::RelativeFilename.as_ptr(),
            b"S\0".as_ptr(),
            &mut (*video).relative_filename as *mut String as *mut c_void,
        ));
    }

    // SAFETY: fmt `'b'` pairs with the `*mut Blob` out-pointer
    // `&mut (*video).raw_absolute_filename`, a field of the fresh non-null
    // element.
    unsafe {
        ufbxi_ignore!(find_val1(
            node,
            sp::FileName.as_ptr(),
            b"b\0".as_ptr(),
            &mut (*video).raw_absolute_filename as *mut Blob as *mut c_void,
        ));
        ufbxi_ignore!(find_val1(
            node,
            sp::Filename.as_ptr(),
            b"b\0".as_ptr(),
            &mut (*video).raw_absolute_filename as *mut Blob as *mut c_void,
        ));
    }
    // SAFETY: as above, with the `*mut Blob` out-pointer
    // `&mut (*video).raw_relative_filename`.
    unsafe {
        ufbxi_ignore!(find_val1(
            node,
            sp::RelativeFileName.as_ptr(),
            b"b\0".as_ptr(),
            &mut (*video).raw_relative_filename as *mut Blob as *mut c_void,
        ));
        ufbxi_ignore!(find_val1(
            node,
            sp::RelativeFilename.as_ptr(),
            b"b\0".as_ptr(),
            &mut (*video).raw_relative_filename as *mut Blob as *mut c_void,
        ));
    }

    let content_node = find_child(node, sp::Content.as_ptr());
    // SAFETY: `video` is the fresh non-null element, so `&mut (*video).content`
    // is the live `ufbx_blob` out-slot `read_embedded_blob` writes.
    unsafe { read_embedded_blob(uc, &mut (*video).content, content_node) }?;

    Ok(())
}

// ufbx.c:14626-14643 `ufbxi_read_anim_stack`
#[inline(never)]
pub(crate) unsafe fn read_anim_stack(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    let _ = node; // C: `(void)node;`

    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `AnimStack` is
    // the element struct for `ElementType::AnimStack`.
    let stack: *mut AnimStack =
        unsafe { push_element::<AnimStack>(uc, info, ElementType::AnimStack) };
    ufbxi_check!(uc, !stack.is_null(), "stack");

    // SAFETY: `info` is the caller's live `ufbxi_element_info`, so
    // `(*info).name.data` is its interned name pointer.
    let hash: u32 = unsafe { crate::native::hash::hash_ptr!((*info).name.data) };
    // SAFETY: `info` is live, so `&(*info).name.data` borrows a live `*const u8`
    // key; the map is uc's own `anim_stack_map`, keyed by that same interned
    // name-pointer type.
    let mut entry: *mut TmpAnimStack = unsafe {
        uc.anim_stack_map_view()
            .find::<TmpAnimStack, _>(hash, &(*info).name.data)
    };
    if entry.is_null() {
        // SAFETY: as the `find` above — same live key and same map.
        entry = unsafe {
            uc.anim_stack_map_view()
                .insert::<TmpAnimStack, _>(hash, &(*info).name.data)
        };
        ufbxi_check!(uc, !entry.is_null(), "entry");
        // SAFETY: `entry` is the fresh non-null map entry checked above and
        // `info` is the caller's live `ufbxi_element_info`.
        unsafe {
            (*entry).name = (*info).name.data;
            (*entry).stack = stack;
        }
    }

    Ok(())
}

// ufbx.c:14645-14687 `ufbxi_read_pose`
#[inline(never)]
pub(crate) unsafe fn read_pose(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
    sub_type: *const u8,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Pose` is the
    // element struct for `ElementType::Pose`.
    let pose: *mut Pose = unsafe { push_element::<Pose>(uc, info, ElementType::Pose) };
    ufbxi_check!(uc, !pose.is_null(), "pose");

    // TODO: What are the actual other types?
    // SAFETY: `pose` is the fresh non-null element pushed above.
    unsafe {
        (*pose).is_bind_pose = sub_type == sp::BindPose.as_ptr();
    }

    let mut num_bones: usize = 0;
    // C: `ufbxi_for(ufbxi_node, n, node->children, node->num_children)`
    // SAFETY: contiguous push_pop child run, valid for `node`'s borrow.
    for n in unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) }
    {
        if n.name() != sp::PoseNode.as_ptr() {
            continue;
        }

        // Bones are linked with FBX names/IDs bypassing the connection system (!?)
        let mut fbx_id: u64 = 0;
        if uc.version() < 7000 {
            let mut name: *mut u8 = core::ptr::null_mut();
            // SAFETY: fmt `'c'` pairs with the `*mut *mut u8` out-pointer
            // `&mut name`, which is a live local.
            if !unsafe {
                find_val1(
                    n,
                    sp::Node.as_ptr(),
                    b"c\0".as_ptr(),
                    &mut name as *mut *mut u8 as *mut c_void,
                )
            } {
                continue;
            }
            fbx_id = synthetic_id_from_string(uc, name);
            ufbxi_check!(uc, fbx_id != 0, "fbx_id");
        } else {
            // SAFETY: fmt `'L'` writes one 64-bit integer through the
            // out-pointer, and `&mut fbx_id` is a live `u64` local of that
            // layout (C passes the `uint64_t` slot the same way).
            if !unsafe {
                find_val1(
                    n,
                    sp::Node.as_ptr(),
                    b"L\0".as_ptr(),
                    &mut fbx_id as *mut u64 as *mut c_void,
                )
            } {
                continue;
            }
            // SAFETY: `&mut fbx_id` is a live `u64` local, the in/out slot
            // `validate_fbx_id` reads and rewrites.
            unsafe { validate_fbx_id(uc, &mut fbx_id) }?;
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
        // SAFETY: `tmp_pose` is non-null, so `&mut (*tmp_pose).bone_to_world` is
        // a live `ufbx_matrix`; the `matrix` payload holds `size >= 16` reals
        // (just checked), the run `read_transform_matrix` requires.
        unsafe {
            read_transform_matrix(&mut (*tmp_pose).bone_to_world, (*matrix).data as *mut Real)
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
pub(crate) unsafe fn sort_shader_prop_bindings(
    uc: &Context,
    bindings: *mut ShaderPropBinding,
    count: usize,
) -> Result<(), Fail> {
    // SAFETY: the allocator, data pointer and size slots are uc's own
    // `ator_tmp`/`tmp_arr`/`tmp_arr_size` fields, reached through its views —
    // the matched triple `grow_array` requires.
    ufbxi_check!(
        uc,
        unsafe {
            grow_array::<u8>(
                uc.ator_tmp_mut_ptr(),
                uc.tmp_arr_mut_ptr(),
                uc.tmp_arr_size_mut_ptr(),
                count.wrapping_mul(size_of::<ShaderPropBinding>()),
            )
        },
        // C-parity: `ufbxi_check`'s `cond` is not preceded by `#`/`##` in its own
        // replacement list, so argument prescan expands `ufbxi_grow_array` before
        // `ufbxi_cond_str` stringifies it (C11 6.10.3.1). Verbatim post-expansion text.
        "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->tmp_arr)), (&uc->tmp_arr), (&uc->tmp_arr_size), (count * sizeof(ufbx_shader_prop_binding)))"
    );
    // SAFETY: `bindings` spans `count` live `ufbx_shader_prop_binding` values (fn
    // contract) and `uc.tmp_arr()` was just grown to
    // `count * size_of::<ShaderPropBinding>()` bytes of scratch, so both the
    // input run and the merge buffer are in bounds; the comparator only ever
    // sees elements of that run, whose `shader_prop` fields are interned strings.
    unsafe {
        macro_stable_sort::<ShaderPropBinding>(
            32,
            bindings,
            uc.tmp_arr() as *mut ShaderPropBinding,
            count,
            |a, b| sp::str_less((*a).shader_prop.as_bytes(), (*b).shader_prop.as_bytes()),
        )
    };
    Ok(())
}

// ufbx.c:14698-14735 `ufbxi_read_binding_table`
#[inline(never)]
pub(crate) unsafe fn read_binding_table(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `ShaderBinding`
    // is the element struct for `ElementType::ShaderBinding`.
    let bindings: *mut ShaderBinding =
        unsafe { push_element::<ShaderBinding>(uc, info, ElementType::ShaderBinding) };
    ufbxi_check!(uc, !bindings.is_null(), "bindings");

    let mut num_entries: usize = 0;
    // C: `ufbxi_for (ufbxi_node, n, node->children, node->num_children)`
    // SAFETY: contiguous push_pop child run, valid for `node`'s borrow.
    for n in unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) }
    {
        if n.name() != sp::Entry.as_ptr() {
            continue;
        }

        // C: `ufbx_string src, dst;` — fully written by `ufbxi_get_val4` before
        // any read; zero-initialized here (no upstream `ufbxi_uninit` marker).
        // SAFETY: `ufbx_string` is a plain pointer/length pair, for which the
        // all-zero bit pattern is a valid (empty, null-data) value.
        let (mut src, mut dst): (String, String) =
            unsafe { (core::mem::zeroed(), core::mem::zeroed()) };
        let mut src_type: *const u8 = core::ptr::null();
        let mut dst_type: *const u8 = core::ptr::null();
        // SAFETY: fmt `"SCSC"` pairs with the `*mut String` / `*mut *const u8`
        // out-pointers `&mut src`, `&mut src_type`, `&mut dst`, `&mut dst_type`,
        // which are live locals in that order.
        if !unsafe {
            get_val4(
                n,
                b"SCSC\0".as_ptr(),
                &mut src as *mut String as *mut c_void,
                &mut src_type as *mut *const u8 as *mut c_void,
                &mut dst as *mut String as *mut c_void,
                &mut dst_type as *mut *const u8 as *mut c_void,
            )
        } {
            continue;
        }

        if src_type == sp::FbxPropertyEntry.as_ptr() && dst_type == sp::FbxSemanticEntry.as_ptr() {
            let bind: *mut ShaderPropBinding = uc.tmp_stack_view().push::<ShaderPropBinding>(1);
            ufbxi_check!(uc, !bind.is_null(), "bind");
            // SAFETY: `bind` is the non-null one-element run just pushed on
            // `tmp_stack`; `src`/`dst` were written by the `"SCSC"` fetch above.
            unsafe {
                (*bind).material_prop = src;
                (*bind).shader_prop = dst;
            }
            num_entries += 1;
        } else if src_type == sp::FbxSemanticEntry.as_ptr()
            && dst_type == sp::FbxPropertyEntry.as_ptr()
        {
            let bind: *mut ShaderPropBinding = uc.tmp_stack_view().push::<ShaderPropBinding>(1);
            ufbxi_check!(uc, !bind.is_null(), "bind");
            // SAFETY: as above, with the roles of `src`/`dst` swapped.
            unsafe {
                (*bind).material_prop = dst;
                (*bind).shader_prop = src;
            }
            num_entries += 1;
        }
    }

    // SAFETY: `bindings` is the fresh non-null element pushed above, and the
    // `num_entries` `ShaderPropBinding` values pushed by the loop are the top of
    // `tmp_stack`, so `push_pop` moves exactly that run into the result buffer.
    unsafe {
        (*bindings).prop_bindings.count = num_entries;
        (*bindings).prop_bindings.data = uc
            .result_view()
            .push_pop::<ShaderPropBinding>(uc.tmp_stack_view(), num_entries);
    }
    // SAFETY: `bindings` is the fresh non-null element.
    ufbxi_check!(
        uc,
        !unsafe { (*bindings).prop_bindings.data }.is_null(),
        "bindings->prop_bindings.data"
    );

    // SAFETY: `bindings` is the fresh non-null element and its `prop_bindings`
    // list is the non-null result-owned run of `count` live bindings set just
    // above, which is what `sort_shader_prop_bindings` requires.
    unsafe {
        sort_shader_prop_bindings(
            uc,
            (*bindings).prop_bindings.data as *mut ShaderPropBinding,
            (*bindings).prop_bindings.count,
        )
    }?;

    Ok(())
}

// ufbx.c:14737-14745 `ufbxi_read_selection_set`
#[inline(never)]
pub(crate) unsafe fn read_selection_set(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    let _ = node; // C: `(void)node;`

    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `SelectionSet`
    // is the element struct for `ElementType::SelectionSet`.
    let set: *mut SelectionSet =
        unsafe { push_element::<SelectionSet>(uc, info, ElementType::SelectionSet) };
    ufbxi_check!(uc, !set.is_null(), "set");

    Ok(())
}

// ufbx.c:14747-14754 `ufbxi_find_uint32_list`
#[inline(never)]
pub(crate) unsafe fn find_uint32_list(dst: *mut List<u32>, node: &NodeView, name: *const u8) {
    let arr: *mut ValueArray = find_array(node, name, b'i');
    if !arr.is_null() {
        // SAFETY: `dst` is the caller's live `ufbx_uint32_list`; `arr` is non-null
        // (checked) and `find_array` returns the node's own array descriptor, live
        // for as long as the parse tree, whose `'i'` payload is `size` `uint32_t`.
        unsafe {
            (*dst).data = (*arr).data as *const u32;
            (*dst).count = (*arr).size;
        }
    }
}

// ufbx.c:14756-14771 `ufbxi_read_selection_node`
#[inline(never)]
pub(crate) unsafe fn read_selection_node(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `SelectionNode`
    // is the element struct for `ElementType::SelectionNode`.
    let sel: *mut SelectionNode =
        unsafe { push_element::<SelectionNode>(uc, info, ElementType::SelectionNode) };
    ufbxi_check!(uc, !sel.is_null(), "sel");

    let mut in_set: i32 = 0;
    // SAFETY: fmt `'I'` pairs with the `*mut i32` out-pointer `&mut in_set`, which
    // is a live local.
    if unsafe {
        find_val1(
            node,
            sp::IsTheNodeInSet.as_ptr(),
            b"I\0".as_ptr(),
            &mut in_set as *mut i32 as *mut c_void,
        )
    } && in_set != 0
    {
        // SAFETY: `sel` is the fresh non-null element pushed above.
        unsafe {
            (*sel).include_node = true;
        }
    }

    // SAFETY: `sel` is the fresh non-null element pushed above, so
    // `&mut (*sel).vertices` is a live `ufbx_uint32_list`; the name is a
    // NUL-terminated interned string pointer.
    unsafe {
        find_uint32_list(&mut (*sel).vertices, node, sp::VertexIndexArray.as_ptr());
        find_uint32_list(&mut (*sel).edges, node, sp::EdgeIndexArray.as_ptr());
        find_uint32_list(&mut (*sel).faces, node, sp::PolygonIndexArray.as_ptr());
    }

    Ok(())
}

// ufbx.c:14773-14783 `ufbxi_read_character`
#[inline(never)]
pub(crate) unsafe fn read_character(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    let _ = node; // C: `(void)node;`

    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Character` is
    // the element struct for `ElementType::Character`.
    let character: *mut Character =
        unsafe { push_element::<Character>(uc, info, ElementType::Character) };
    ufbxi_check!(uc, !character.is_null(), "character");

    // TODO: There's some extremely cursed all-caps data in characters

    Ok(())
}

// ufbx.c:14785-14798 `ufbxi_read_audio_clip`
#[inline(never)]
pub(crate) unsafe fn read_audio_clip(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `AudioClip` is
    // the element struct for `ElementType::AudioClip`.
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
    // `&mut (*audio).content` is a live `ufbx_blob` destination.
    unsafe { read_embedded_blob(uc, &mut (*audio).content, content_node) }?;

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
pub(crate) unsafe fn read_constraint(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    let _ = node; // C: `(void)node;`

    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Constraint` is
    // the element struct for `ElementType::Constraint`.
    let constraint: *mut Constraint =
        unsafe { push_element::<Constraint>(uc, info, ElementType::Constraint) };
    ufbxi_check!(uc, !constraint.is_null(), "constraint");

    // SAFETY: fmt `'S'` pairs with the `*mut ufbx_string` out-pointer
    // `&mut (*constraint).type_name`, a field of the fresh non-null element pushed
    // above.
    if !unsafe {
        find_val1(
            node,
            sp::Type.as_ptr(),
            b"S\0".as_ptr(),
            &mut (*constraint).type_name as *mut String as *mut c_void,
        )
    } {
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
#[inline(never)]
pub(crate) unsafe fn read_synthetic_attribute(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
    type_str: String,
    sub_type: *const u8,
    super_type: *const u8,
) -> Result<(), Fail> {
    let mut sub_type = sub_type;

    // Some legacy (version 6000) files store mesh nodes without any `sub_type`
    // There seems to be no robust indicator, so detect it from `Vertices` and `PolygonVertexIndex`
    if sub_type == EMPTY_CHAR.as_ptr() {
        let node_vertices = find_child(node, sp::Vertices.as_ptr());
        let node_indices = find_child(node, sp::PolygonVertexIndex.as_ptr());
        if node_vertices.is_some() && node_indices.is_some() {
            sub_type = sp::Mesh.as_ptr();
        }
    }

    if (sub_type == EMPTY_CHAR.as_ptr() || sub_type == sp::Model.as_ptr())
        && type_str.data == sp::Model.as_ptr()
    {
        // Plain model
        return Ok(());
    }

    // C: `ufbxi_element_info attrib_info = *info;` — struct assignment is a
    // memcpy; `ufbxi_element_info` has no Rust `Copy` (it embeds the generated
    // `ufbx_props`), so read the bytes through the pointer instead.
    // SAFETY: `info` is the caller's live, initialized `ufbxi_element_info`; the
    // bitwise copy mirrors the C struct assignment and both copies stay in scope
    // only for the duration of this call, matching C's aliasing of the two.
    let mut attrib_info: ElementInfo = unsafe { core::ptr::read(info) };

    attrib_info.fbx_id = push_synthetic_id(uc);

    // Use type and name from NodeAttributeName if it exists *uniquely*
    // C: `ufbx_string type_and_name;` — fully written by `ufbxi_find_val1`
    // before any read; zero-initialized here (no upstream `ufbxi_uninit` marker).
    // SAFETY: `ufbx_string` is a plain pointer/length pair, for which the all-zero
    // bit pattern is a valid (empty, null-data) value.
    let mut type_and_name: String = unsafe { core::mem::zeroed() };
    // SAFETY: fmt `'s'` pairs with the `*mut ufbx_string` out-pointer
    // `&mut type_and_name`, which is a live local.
    if unsafe {
        find_val1(
            node,
            sp::NodeAttributeName.as_ptr(),
            b"s\0".as_ptr(),
            &mut type_and_name as *mut String as *mut c_void,
        )
    } {
        // C: `ufbx_string attrib_type_str, attrib_name_str;` — both written by
        // `ufbxi_split_type_and_name`; zero-initialized here.
        // SAFETY: `ufbx_string` is a plain pointer/length pair, for which the
        // all-zero bit pattern is a valid (empty, null-data) value.
        let (mut attrib_type_str, mut attrib_name_str): (String, String) =
            unsafe { (core::mem::zeroed(), core::mem::zeroed()) };
        // SAFETY: `type_and_name` was fully written by the `'s'` fetch above, so
        // it spans `length` readable bytes; the two out-pointers are live locals.
        unsafe {
            split_type_and_name(
                uc,
                type_and_name,
                &mut attrib_type_str,
                &mut attrib_name_str,
            )
        }?;
        if attrib_name_str.length > 0 {
            attrib_info.name = attrib_name_str;
            let attrib_id: u64 = synthetic_id_from_string(uc, type_and_name.data);
            ufbxi_check!(uc, attrib_id != 0, "attrib_id");
            // SAFETY: `info` is the caller's live `ufbxi_element_info`.
            if unsafe { (*info).fbx_id } != attrib_id && !fbx_id_exists(uc, attrib_id) {
                attrib_info.fbx_id = attrib_id;
            }
        }
    }

    // 6x00: Link the node to the node attribute so property connections can be
    // redirected from connections if necessary.
    // SAFETY: `info` is the caller's live `ufbxi_element_info`.
    insert_fbx_attr(uc, unsafe { (*info).fbx_id }, attrib_info.fbx_id)?;

    // Split properties between the node and the attribute.
    // Consider all user properties as node properties.
    // SAFETY: `info` is the caller's live `ufbxi_element_info`.
    let ps: *mut Prop = unsafe { (*info).props.props.data } as *mut Prop;
    let mut dst: usize = 0;
    let mut src: usize = 0;
    // SAFETY: as above.
    let end: usize = unsafe { (*info).props.props.count };
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
                !unsafe { push_copy::<Prop>(uc.tmp_stack_mut_ptr(), 1, ps.add(src)) }.is_null(),
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
    // SAFETY: `info` is the caller's live `ufbxi_element_info`.
    unsafe {
        (*info).props.props.count = dst;
    }

    // SAFETY (this whole dispatch): every arm hands the same parse-tree NodeView
    // `node`, the local `&mut attrib_info`, and interned NUL-terminated
    // `type_str` / `sub_type` / `super_type` string pointers to the per-type
    // reader selected by the pointer-identity comparisons; each `read_element`
    // arm pairs `size_of::<T>()` with `T`'s own `ElementType`.
    unsafe {
        if sub_type == sp::Mesh.as_ptr() {
            read_mesh(uc, node, &mut attrib_info)?;
        } else if sub_type == sp::Light.as_ptr() {
            read_element(
                uc,
                node,
                &mut attrib_info,
                size_of::<Light>(),
                ElementType::Light,
            )?;
        } else if sub_type == sp::Camera.as_ptr() {
            read_element(
                uc,
                node,
                &mut attrib_info,
                size_of::<Camera>(),
                ElementType::Camera,
            )?;
        } else if sub_type == sp::LimbNode.as_ptr()
            || sub_type == sp::Limb.as_ptr()
            || sub_type == sp::Root.as_ptr()
        {
            read_bone(uc, node, &mut attrib_info, sub_type)?;
        } else if sub_type == sp::Null.as_ptr() || sub_type == sp::Marker.as_ptr() {
            read_element(
                uc,
                node,
                &mut attrib_info,
                size_of::<Empty>(),
                ElementType::Empty,
            )?;
        } else if sub_type == sp::NurbsCurve.as_ptr() {
            if find_child(node, sp::KnotVector.as_ptr()).is_none() {
                return Ok(());
            }
            read_nurbs_curve(uc, node, &mut attrib_info)?;
        } else if sub_type == sp::NurbsSurface.as_ptr() {
            if find_child(node, sp::KnotVectorU.as_ptr()).is_none() {
                return Ok(());
            }
            if find_child(node, sp::KnotVectorV.as_ptr()).is_none() {
                return Ok(());
            }
            read_nurbs_surface(uc, node, &mut attrib_info)?;
        } else if sub_type == sp::Line.as_ptr() {
            if find_child(node, sp::Points.as_ptr()).is_none() {
                return Ok(());
            }
            if find_child(node, sp::PointsIndex.as_ptr()).is_none() {
                return Ok(());
            }
            read_line(uc, node, &mut attrib_info)?;
        } else if sub_type == sp::TrimNurbsSurface.as_ptr() {
            if find_child(node, sp::Layer.as_ptr()).is_none() {
                return Ok(());
            }
            read_element(
                uc,
                node,
                &mut attrib_info,
                size_of::<NurbsTrimSurface>(),
                ElementType::NurbsTrimSurface,
            )?;
        } else if sub_type == sp::Boundary.as_ptr() {
            read_element(
                uc,
                node,
                &mut attrib_info,
                size_of::<NurbsTrimBoundary>(),
                ElementType::NurbsTrimBoundary,
            )?;
        } else if sub_type == sp::CameraStereo.as_ptr() {
            read_element(
                uc,
                node,
                &mut attrib_info,
                size_of::<StereoCamera>(),
                ElementType::StereoCamera,
            )?;
        } else if sub_type == sp::CameraSwitcher.as_ptr() {
            read_element(
                uc,
                node,
                &mut attrib_info,
                size_of::<CameraSwitcher>(),
                ElementType::CameraSwitcher,
            )?;
        } else if sub_type == sp::FKEffector.as_ptr() {
            read_marker(uc, node, &mut attrib_info, sub_type, MarkerType::FkEffector)?;
        } else if sub_type == sp::IKEffector.as_ptr() {
            read_marker(uc, node, &mut attrib_info, sub_type, MarkerType::IkEffector)?;
        } else if sub_type == sp::LodGroup.as_ptr() {
            read_element(
                uc,
                node,
                &mut attrib_info,
                size_of::<LodGroup>(),
                ElementType::LodGroup,
            )?;
        } else {
            let sub_type_str: String = String::new_c(sub_type, strlen(sub_type));
            read_unknown(
                uc,
                node,
                &mut attrib_info,
                type_str,
                sub_type_str,
                super_type,
            )?;
        }
    }

    // SAFETY: `info` is the caller's live `ufbxi_element_info`.
    connect_oo(uc, attrib_info.fbx_id, unsafe { (*info).fbx_id })?;
    Ok(())
}

// ufbx.c:14941-14945 `ufbxi_read_global_settings`
#[inline(never)]
pub(crate) fn read_global_settings(uc: &Context, node: &NodeView) -> Result<(), Fail> {
    // SAFETY: `node` is a parse-tree NodeView and the destination is uc's own
    // scene settings `props` slot, reached through its element views.
    unsafe { read_properties(uc, node, uc.scene_view().settings_view().props_mut_ptr())? };
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

    // C: `ufbx_string type_and_name, sub_type_str;` — both fully written by the
    // `ufbxi_get_val*` calls below before any read (a partial failure returns
    // early); zero-initialized here (no upstream `ufbxi_uninit` marker).
    // SAFETY: `ufbx_string` is a `{ data, length }` pair, so all-zero (a null
    // pointer and a zero length) is a valid bit pattern.
    let mut type_and_name: String = unsafe { core::mem::zeroed() };
    let mut sub_type_str: String = unsafe { core::mem::zeroed() };

    // Failing to parse the object properties is not an error since
    // there's some weird objects mixed in every now and then.
    // FBX version 7000 and up uses 64-bit unique IDs per object,
    // older FBX versions just use name/type pairs, which we can
    // use as IDs since all strings are interned into a string pool.
    // SAFETY: `node` is a parse-tree NodeView; the `Lss` / `ss` out-params are
    // unaliased locals (and `info`'s own `fbx_id`) of exactly the `uint64_t` /
    // `ufbx_string` types those format characters write. On success
    // `type_and_name.data` is a pooled, NUL-terminated string, which is what
    // the synthetic-id hash below reads.
    unsafe {
        if uc.version() >= 7000 {
            if !get_val3(
                node,
                b"Lss\0".as_ptr(),
                &mut info.fbx_id as *mut u64 as *mut c_void,
                &mut type_and_name as *mut String as *mut c_void,
                &mut sub_type_str as *mut String as *mut c_void,
            ) {
                return Ok(());
            }
            validate_fbx_id(uc, &mut info.fbx_id)?;
        } else {
            if !get_val2(
                node,
                b"ss\0".as_ptr(),
                &mut type_and_name as *mut String as *mut c_void,
                &mut sub_type_str as *mut String as *mut c_void,
            ) {
                return Ok(());
            }
            info.fbx_id = synthetic_id_from_string(uc, type_and_name.data);
            ufbxi_check!(uc, info.fbx_id != 0, "info.fbx_id");
        }
    }

    // Remove the "Fbx" prefix from sub-types, remember to re-intern!
    // SAFETY: `sub_type_str` was filled by the reads above, so it is a pooled
    // string of `length` bytes; the compare and the 3-byte advance are guarded
    // by the `length > 3` check, and the re-intern goes to uc's own pool.
    unsafe {
        if sub_type_str.length > 3 && memcmp(sub_type_str.data, b"Fbx".as_ptr(), 3) == 0 {
            sub_type_str.data = sub_type_str.data.add(3);
            sub_type_str.length -= 3;
            push_string_place_str(uc.string_pool_mut_ptr(), &mut sub_type_str, false)?;
        }
    }

    // C: `ufbx_string type_str;` — fully written by `ufbxi_split_type_and_name`.
    // SAFETY: all-zero is a valid `ufbx_string`; `type_and_name` is the pooled
    // string read above, and the two out-params are an unaliased local and
    // `info`'s own `name` field.
    let mut type_str: String = unsafe { core::mem::zeroed() };
    unsafe { split_type_and_name(uc, type_and_name, &mut type_str, &mut info.name)? };

    let name: *const u8 = node.name();
    let sub_type: *const u8 = sub_type_str.data;
    // SAFETY: `node` is a parse-tree NodeView and `&mut info.props` is a local
    // `ufbx_props` out-param; `name`/`sub_type` are pooled strings, which is
    // what the template lookup compares by pointer identity, and its result
    // points into uc's own template array (or is null, mapped to `None`).
    unsafe {
        read_properties(uc, node, &mut info.props)?;
        info.props.defaults = opt_ref(find_template(uc, name, sub_type));
    }

    // SAFETY (this whole dispatch): every arm hands the same parse-tree
    // NodeView `node`, the local `&mut info`, and pooled `type_str` /
    // `sub_type_str` / `name` / `sub_type` strings to the per-type reader
    // selected by the pointer-identity comparisons — one logical dispatch, no
    // pointer arithmetic of its own.
    unsafe {
        if name == sp::Model.as_ptr() {
            if uc.version() < 7000 {
                read_synthetic_attribute(uc, node, &mut info, type_str, sub_type, name)?;
            }
            read_model(uc, node, &mut info)?;
        } else if name == sp::NodeAttribute.as_ptr() {
            if sub_type == sp::Light.as_ptr() {
                read_element(uc, node, &mut info, size_of::<Light>(), ElementType::Light)?;
            } else if sub_type == sp::Camera.as_ptr() {
                read_element(
                    uc,
                    node,
                    &mut info,
                    size_of::<Camera>(),
                    ElementType::Camera,
                )?;
            } else if sub_type == sp::LimbNode.as_ptr()
                || sub_type == sp::Limb.as_ptr()
                || sub_type == sp::Root.as_ptr()
            {
                read_bone(uc, node, &mut info, sub_type)?;
            } else if sub_type == sp::Null.as_ptr() || sub_type == sp::Marker.as_ptr() {
                read_element(uc, node, &mut info, size_of::<Empty>(), ElementType::Empty)?;
            } else if sub_type == sp::CameraStereo.as_ptr() {
                read_element(
                    uc,
                    node,
                    &mut info,
                    size_of::<StereoCamera>(),
                    ElementType::StereoCamera,
                )?;
            } else if sub_type == sp::CameraSwitcher.as_ptr() {
                read_element(
                    uc,
                    node,
                    &mut info,
                    size_of::<CameraSwitcher>(),
                    ElementType::CameraSwitcher,
                )?;
            } else if sub_type == sp::FKEffector.as_ptr() {
                read_marker(uc, node, &mut info, sub_type, MarkerType::FkEffector)?;
            } else if sub_type == sp::IKEffector.as_ptr() {
                read_marker(uc, node, &mut info, sub_type, MarkerType::IkEffector)?;
            } else if sub_type == sp::LodGroup.as_ptr() {
                read_element(
                    uc,
                    node,
                    &mut info,
                    size_of::<LodGroup>(),
                    ElementType::LodGroup,
                )?;
            } else {
                read_unknown(uc, node, &mut info, type_str, sub_type_str, name)?;
            }
        } else if name == sp::Geometry.as_ptr() {
            if sub_type == sp::Mesh.as_ptr() {
                read_mesh(uc, node, &mut info)?;
            } else if sub_type == sp::Shape.as_ptr() {
                read_shape(uc, node, &mut info)?;
            } else if sub_type == sp::NurbsCurve.as_ptr() {
                read_nurbs_curve(uc, node, &mut info)?;
            } else if sub_type == sp::NurbsSurface.as_ptr() {
                read_nurbs_surface(uc, node, &mut info)?;
            } else if sub_type == sp::Line.as_ptr() {
                read_line(uc, node, &mut info)?;
            } else if sub_type == sp::TrimNurbsSurface.as_ptr() {
                read_element(
                    uc,
                    node,
                    &mut info,
                    size_of::<NurbsTrimSurface>(),
                    ElementType::NurbsTrimSurface,
                )?;
            } else if sub_type == sp::Boundary.as_ptr() {
                read_element(
                    uc,
                    node,
                    &mut info,
                    size_of::<NurbsTrimBoundary>(),
                    ElementType::NurbsTrimBoundary,
                )?;
            } else {
                read_unknown(uc, node, &mut info, type_str, sub_type_str, name)?;
            }
        } else if name == sp::Deformer.as_ptr() {
            if sub_type == sp::Skin.as_ptr() {
                read_skin(uc, node, &mut info)?;
            } else if sub_type == sp::Cluster.as_ptr() {
                read_skin_cluster(uc, node, &mut info)?;
            } else if sub_type == sp::BlendShape.as_ptr() {
                read_element(
                    uc,
                    node,
                    &mut info,
                    size_of::<BlendDeformer>(),
                    ElementType::BlendDeformer,
                )?;
            } else if sub_type == sp::BlendShapeChannel.as_ptr() {
                read_blend_channel(uc, node, &mut info)?;
            } else if sub_type == sp::VertexCacheDeformer.as_ptr() {
                read_element(
                    uc,
                    node,
                    &mut info,
                    size_of::<CacheDeformer>(),
                    ElementType::CacheDeformer,
                )?;
            } else {
                read_unknown(uc, node, &mut info, type_str, sub_type_str, name)?;
            }
        } else if name == sp::Material.as_ptr() {
            read_material(uc, node, &mut info)?;
        } else if name == sp::Texture.as_ptr() {
            read_texture(uc, node, &mut info)?;
        } else if name == sp::LayeredTexture.as_ptr() {
            read_layered_texture(uc, node, &mut info)?;
        } else if name == sp::Video.as_ptr() {
            read_video(uc, node, &mut info)?;
        } else if name == sp::AnimationStack.as_ptr() {
            read_anim_stack(uc, node, &mut info)?;
        } else if name == sp::AnimationLayer.as_ptr() {
            read_element(
                uc,
                node,
                &mut info,
                size_of::<AnimLayer>(),
                ElementType::AnimLayer,
            )?;
        } else if name == sp::AnimationCurveNode.as_ptr() {
            read_element(
                uc,
                node,
                &mut info,
                size_of::<AnimValue>(),
                ElementType::AnimValue,
            )?;
        } else if name == sp::AnimationCurve.as_ptr() {
            read_animation_curve(uc, node, &mut info)?;
        } else if name == sp::Pose.as_ptr() {
            read_pose(uc, node, &mut info, sub_type)?;
        } else if name == sp::Implementation.as_ptr() {
            read_element(
                uc,
                node,
                &mut info,
                size_of::<Shader>(),
                ElementType::Shader,
            )?;
        } else if name == sp::BindingTable.as_ptr() {
            read_binding_table(uc, node, &mut info)?;
        } else if name == sp::Collection.as_ptr() {
            if sub_type == sp::SelectionSet.as_ptr() {
                read_selection_set(uc, node, &mut info)?;
            }
        } else if name == sp::CollectionExclusive.as_ptr() {
            if sub_type == sp::DisplayLayer.as_ptr() {
                read_element(
                    uc,
                    node,
                    &mut info,
                    size_of::<DisplayLayer>(),
                    ElementType::DisplayLayer,
                )?;
            }
        } else if name == sp::SelectionNode.as_ptr() {
            read_selection_node(uc, node, &mut info)?;
        } else if name == sp::Constraint.as_ptr() {
            if sub_type == sp::Character.as_ptr() {
                read_character(uc, node, &mut info)?;
            } else {
                read_constraint(uc, node, &mut info)?;
            }
        } else if name == sp::SceneInfo.as_ptr() {
            read_scene_info(uc, node)?;
        } else if name == sp::Cache.as_ptr() {
            read_element(
                uc,
                node,
                &mut info,
                size_of::<CacheFile>(),
                ElementType::CacheFile,
            )?;
        } else if name == sp::ObjectMetaData.as_ptr() {
            read_element(
                uc,
                node,
                &mut info,
                size_of::<MetadataObject>(),
                ElementType::MetadataObject,
            )?;
        } else if name == sp::AudioLayer.as_ptr() {
            read_element(
                uc,
                node,
                &mut info,
                size_of::<AudioLayer>(),
                ElementType::AudioLayer,
            )?;
        } else if name == sp::Audio.as_ptr() {
            read_audio_clip(uc, node, &mut info)?;
        } else {
            read_unknown(uc, node, &mut info, type_str, sub_type_str, name)?;
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
// Stays `unsafe fn`: the body is raw end-to-end. It drives a local
// `ObjectBatch` array through raw `batch` pointers, performs pointer surgery on
// uc's ASCII source window (`ua->src`/`src_end`/`src_yield` retargeted at
// `uc->read_buffer` after a `memcpy`), reads `uc->thread_pool` task counters and
// `tmp_buf` fill levels through raw derefs, and hands `tmp_buf`-allocated node
// runs to the worker tasks. The obligation that all of that state is coherent —
// in particular that `tmp_buf` is not cleared while a batch still refers to it —
// is a whole-function invariant with no narrow seam to name.
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
        let batch: *mut ObjectBatch = &mut batches[batch_index];

        // SAFETY: `uc.thread_pool_mut_ptr()` is uc's own live `thread_pool`.
        unsafe { thread_pool_wait_group(uc.thread_pool_mut_ptr()) }?;

        // SAFETY: `batch` points into the live `batches` local.
        if unsafe { (*batch).num_nodes } > 0 {
            // C: `ufbxi_for_ptr(ufbxi_node, p_node, batch->nodes, batch->num_nodes)`
            // SAFETY: as above.
            let (mut p_node, p_node_end) =
                unsafe { ((*batch).nodes, add_ptr((*batch).nodes, (*batch).num_nodes)) };
            while p_node != p_node_end {
                // SAFETY: `uc.tmp_parse_mut_ptr()` is uc's own live `tmp_parse` buf.
                unsafe { buf_clear(uc.tmp_parse_mut_ptr()) };

                // Push a deferred element ID for tagging warnings
                uc.set_p_element_id(uc.tmp_element_id_view().push::<u32>(1));
                ufbxi_check!(uc, !uc.p_element_id().is_null(), "uc->p_element_id");
                // SAFETY: `uc->p_element_id` is the slot just pushed onto
                // `tmp_element_id` and checked non-null, live until that buf is
                // popped.
                unsafe { *uc.p_element_id() = NO_INDEX };
                uc.warnings_view()
                    .set_deferred_element_id_plus_one(uc.tmp_element_id_view().num_items() as u32);

                // SAFETY: `p_node` walks the batch's `num_nodes`-long run of node
                // pointers and stops at `p_node_end`, so it is in bounds; each
                // entry points at a `tmp_buf`-allocated parse node kept live until
                // the batch is retired.
                read_object(uc, unsafe { NodeView::from_ptr(*p_node) })?;

                uc.warnings_view().set_deferred_element_id_plus_one(0);
                uc.set_p_element_id(core::ptr::null_mut());

                // SAFETY: `p_node` is in bounds of the node-pointer run and
                // stepping it one past the last entry reaches `p_node_end`, the
                // one-past-the-end pointer.
                p_node = unsafe { p_node.add(1) };
            }
            // SAFETY: `batch` points into the live `batches` local.
            unsafe {
                (*batch).num_nodes = 0;
            }
        }

        let tmp_buf: &BufView = uc.tmp_thread_parse_at(batch_index);

        // ASCII data may be in `tmp_buf`, so copy it to safety in case
        if uc.ascii_view().src_buf() == tmp_buf.get() {
            let ua: *mut Ascii = uc.ascii_mut_ptr();
            // SAFETY: `ua` is uc's own live `ascii` state, whose `src`/`src_end`
            // delimit one source window, so the two are derived from the same
            // allocation and their difference is well defined.
            let size: usize = to_size(unsafe { (*ua).src_end.offset_from((*ua).src) });
            if uc.read_buffer_size() < size {
                // SAFETY: `uc.ator_tmp_mut_ptr()`, `uc.read_buffer_mut_ptr()` and
                // `uc.read_buffer_size_mut_ptr()` are uc's own live `ator_tmp`,
                // `read_buffer` and `read_buffer_size` slots, and the buffer's
                // element type `u8` matches the `T` requested.
                ufbxi_check!(
                    uc,
                    unsafe {
                        grow_array::<u8>(
                            uc.ator_tmp_mut_ptr(),
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
            // SAFETY: `(*ua).src` spans the `size` bytes computed from the window
            // above, `uc.read_buffer()` has room for `size` bytes (grown when it
            // was short), and the source window lives in `tmp_buf` while the read
            // buffer is a separate `ator_tmp` allocation, so the two are disjoint.
            unsafe { core::ptr::copy_nonoverlapping((*ua).src, uc.read_buffer(), size) };
            // C: `uc->data = uc->data_begin = ua->src = uc->read_buffer;`
            // SAFETY: `ua` is uc's own live `ascii` state.
            unsafe {
                (*ua).src = uc.read_buffer();
                uc.set_data_begin((*ua).src);
            }
            uc.set_data(uc.data_begin());
            // SAFETY: `ua` is uc's own live `ascii` state; `read_buffer` holds at
            // least `size` bytes, so stepping `size` reaches its one-past-the-end.
            unsafe {
                (*ua).src_end = uc.read_buffer().add(size);
            }
            // SAFETY: `ua` is uc's own live `ascii` state.
            unsafe {
                (*ua).src_is_retained = false;
                (*ua).src_buf = core::ptr::null_mut();
            }
            // SAFETY: `src`/`src_end` were just retargeted at the same
            // `read_buffer` allocation, so their difference is well defined.
            if to_size(unsafe { (*ua).src_end.offset_from((*ua).src) }) < uc.progress_interval() {
                // SAFETY: `ua` is uc's own live `ascii` state.
                unsafe {
                    (*ua).src_yield = (*ua).src_end;
                }
            } else {
                // SAFETY: `ua` is uc's own live `ascii` state; the branch condition
                // establishes `progress_interval <= src_end - src`, so the step
                // stays inside the read buffer.
                unsafe {
                    (*ua).src_yield = (*ua).src.add(uc.progress_interval());
                }
            }
            // SAFETY: `ua` is uc's own live `ascii` state.
            uc.set_data(unsafe { (*ua).src });
        }

        // SAFETY: `tmp_buf` is one of uc's own live `tmp_thread_parse` bufs.
        unsafe { buf_clear(tmp_buf.get()) };

        if !parsed_to_end {
            let mut num_nodes: usize = 0;
            // SAFETY: `uc.get()` is the live, initialized context this call runs
            // on, so its `thread_pool` sub-struct is readable.
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

                // SAFETY: `uc.get()` is the live, initialized context this call
                // runs on, so its `thread_pool` sub-struct is readable.
                let num_tasks: u32 =
                    unsafe { (*uc.get()).thread_pool.start_index }.wrapping_sub(task_start);
                if num_tasks >= max_tasks {
                    break;
                }

                // SAFETY: `tmp_buf` is one of uc's own live `tmp_thread_parse`
                // bufs.
                let memory_used: usize =
                    unsafe { (*tmp_buf.get()).pushed_size } + unsafe { (*tmp_buf.get()).pos };
                if memory_used >= max_memory {
                    break;
                }
            }

            // SAFETY: `batch` points into the live `batches` local.
            unsafe {
                (*batch).num_nodes = num_nodes;
                (*batch).nodes = tmp_buf.push_pop::<*mut Node>(uc.tmp_stack_view(), num_nodes);
                ufbxi_check!(uc, !(*batch).nodes.is_null(), "batch->nodes");
            }
            // SAFETY: `batch` points into the live `batches` local and `uc.get()`
            // is the live, initialized context this call runs on.
            unsafe {
                (*batch).task_index = (*uc.get()).thread_pool.start_index;
            }
        }

        // Not safe to refer to this buffer anymore
        uc.ascii_view().set_src_is_retained(false);

        // SAFETY: `uc.thread_pool_mut_ptr()` is uc's own live `thread_pool`.
        unsafe { thread_pool_flush_group(uc.thread_pool_mut_ptr()) };

        // SAFETY: `batch` points into the live `batches` local.
        if unsafe { (*batch).num_nodes } == 0 {
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
        // C: `char *type;` — written by the `ufbxi_get_val1` guards below.
        let mut type_: *const u8 = core::ptr::null();

        let mut src_id: u64 = 0;
        let mut dst_id: u64 = 0;
        let mut src_prop: String = EMPTY_STRING.0;
        let mut dst_prop: String = EMPTY_STRING.0;

        if uc.version() < 7000 {
            let mut src_name: *const u8 = core::ptr::null();
            let mut dst_name: *const u8 = core::ptr::null();
            // Pre-7000 versions use Type::Name pairs as identifiers

            // SAFETY (this branch): `node` is a parse-tree NodeView; every
            // out-param is an unaliased local of exactly the type its format
            // character writes (`c` → `char*`, `s` → `ufbx_string`, `_` →
            // skipped, so the matching NULL is never written). The strings the
            // reads produce are pooled and NUL-terminated, which is what the
            // re-intern and the synthetic-id hashes below require.
            unsafe {
                if !get_val1(
                    node,
                    b"c\0".as_ptr(),
                    &mut type_ as *mut *const u8 as *mut c_void,
                ) {
                    continue;
                }

                if type_ == sp::OO.as_ptr() {
                    if !get_val3(
                        node,
                        b"_cc\0".as_ptr(),
                        core::ptr::null_mut(),
                        &mut src_name as *mut *const u8 as *mut c_void,
                        &mut dst_name as *mut *const u8 as *mut c_void,
                    ) {
                        continue;
                    }
                } else if type_ == sp::OP.as_ptr() {
                    if !get_val4(
                        node,
                        b"_ccs\0".as_ptr(),
                        core::ptr::null_mut(),
                        &mut src_name as *mut *const u8 as *mut c_void,
                        &mut dst_name as *mut *const u8 as *mut c_void,
                        &mut dst_prop as *mut String as *mut c_void,
                    ) {
                        continue;
                    }
                } else if type_ == sp::PO.as_ptr() {
                    if !get_val4(
                        node,
                        b"_csc\0".as_ptr(),
                        core::ptr::null_mut(),
                        &mut src_name as *mut *const u8 as *mut c_void,
                        &mut src_prop as *mut String as *mut c_void,
                        &mut dst_name as *mut *const u8 as *mut c_void,
                    ) {
                        continue;
                    }
                } else if type_ == sp::PP.as_ptr() {
                    if !get_val5(
                        node,
                        b"_cscs\0".as_ptr(),
                        core::ptr::null_mut(),
                        &mut src_name as *mut *const u8 as *mut c_void,
                        &mut src_prop as *mut String as *mut c_void,
                        &mut dst_name as *mut *const u8 as *mut c_void,
                        &mut dst_prop as *mut String as *mut c_void,
                    ) {
                        continue;
                    }
                } else {
                    // TODO: Strict mode?
                    continue;
                }

                if src_prop.length > 0 {
                    push_string_place_str(uc.string_pool_mut_ptr(), &mut src_prop, false)?;
                }
                if dst_prop.length > 0 {
                    push_string_place_str(uc.string_pool_mut_ptr(), &mut dst_prop, false)?;
                }

                src_id = synthetic_id_from_string(uc, src_name);
                dst_id = synthetic_id_from_string(uc, dst_name);
                ufbxi_check!(uc, src_id != 0 && dst_id != 0, "src_id && dst_id");
            }
        } else {
            // Post-7000 versions use proper unique 64-bit IDs

            // SAFETY (this branch): `node` is a parse-tree NodeView; every
            // out-param is an unaliased local of exactly the type its format
            // character writes (`C` → `char*`, `L` → `uint64_t`, `S` →
            // `ufbx_string`, `_` → skipped, so the matching NULL is never
            // written), and the id validators take those same locals.
            unsafe {
                if !get_val1(
                    node,
                    b"C\0".as_ptr(),
                    &mut type_ as *mut *const u8 as *mut c_void,
                ) {
                    continue;
                }

                if type_ == sp::OO.as_ptr() {
                    if !get_val3(
                        node,
                        b"_LL\0".as_ptr(),
                        core::ptr::null_mut(),
                        &mut src_id as *mut u64 as *mut c_void,
                        &mut dst_id as *mut u64 as *mut c_void,
                    ) {
                        continue;
                    }
                } else if type_ == sp::OP.as_ptr() {
                    if !get_val4(
                        node,
                        b"_LLS\0".as_ptr(),
                        core::ptr::null_mut(),
                        &mut src_id as *mut u64 as *mut c_void,
                        &mut dst_id as *mut u64 as *mut c_void,
                        &mut dst_prop as *mut String as *mut c_void,
                    ) {
                        continue;
                    }
                } else if type_ == sp::PO.as_ptr() {
                    if !get_val4(
                        node,
                        b"_LSL\0".as_ptr(),
                        core::ptr::null_mut(),
                        &mut src_id as *mut u64 as *mut c_void,
                        &mut src_prop as *mut String as *mut c_void,
                        &mut dst_id as *mut u64 as *mut c_void,
                    ) {
                        continue;
                    }
                } else if type_ == sp::PP.as_ptr() {
                    if !get_val5(
                        node,
                        b"_LSLS\0".as_ptr(),
                        core::ptr::null_mut(),
                        &mut src_id as *mut u64 as *mut c_void,
                        &mut src_prop as *mut String as *mut c_void,
                        &mut dst_id as *mut u64 as *mut c_void,
                        &mut dst_prop as *mut String as *mut c_void,
                    ) {
                        continue;
                    }
                } else {
                    // TODO: Strict mode?
                    continue;
                }

                validate_fbx_id(uc, &mut src_id)?;
                validate_fbx_id(uc, &mut dst_id)?;
            }
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

// ufbx.c:15323-15584 `ufbxi_read_take_anim_channel`
#[inline(never)]
pub(crate) unsafe fn read_take_anim_channel(
    uc: &Context,
    node: &NodeView,
    value_fbx_id: u64,
    name: *const u8,
    p_default: *mut Real,
) -> Result<(), Fail> {
    // SAFETY: fmt `'R'` pairs with the `*mut ufbx_real` out-pointer `p_default`,
    // which is the caller's live, writable default slot (fn contract).
    ufbxi_ignore!(unsafe {
        find_val1(
            node,
            sp::Default.as_ptr(),
            b"R\0".as_ptr(),
            p_default as *mut c_void,
        )
    });

    // Find the key array, early return with success if not found as we may have only a default
    let keys: *mut ValueArray = find_array(node, sp::Key.as_ptr(), b'd');
    if keys.is_null() {
        return Ok(());
    }

    let mut curve_fbx_id: u64 = 0;
    // SAFETY: `&mut curve_fbx_id` is a live local `uint64_t` slot, `name` is the
    // caller's NUL-terminated channel name (fn contract), and `AnimCurve` is the
    // element struct for `ElementType::AnimCurve`.
    let curve: *mut AnimCurve = unsafe {
        push_synthetic_element::<AnimCurve>(
            uc,
            &mut curve_fbx_id,
            Some(node),
            name,
            ElementType::AnimCurve,
        )
    };
    ufbxi_check!(uc, !curve.is_null(), "curve");

    // SAFETY: `curve` is the fresh non-null element pushed above, so its
    // `element.name` is the interned scene string the push installed.
    unsafe { connect_op(uc, curve_fbx_id, value_fbx_id, (*curve).element.name) }?;

    // SAFETY: `curve` is the fresh non-null element pushed above, so
    // `&mut (*curve).pre_extrapolation` is a live out-slot; the name is an
    // interned NUL-terminated string pointer.
    unsafe {
        read_extrapolation(
            &mut (*curve).pre_extrapolation,
            node,
            sp::Pre_Extrapolation.as_ptr(),
        );
        read_extrapolation(
            &mut (*curve).post_extrapolation,
            node,
            sp::Post_Extrapolation.as_ptr(),
        );
    }

    if uc.opts_view().ignore_animation() {
        return Ok(());
    }

    let mut key_ver: i32 = 0;
    // SAFETY: fmt `'I'` pairs with the `*mut i32` out-pointer `&mut key_ver`,
    // which is a live local.
    ufbxi_ignore!(unsafe {
        find_val1(
            node,
            sp::KeyVer.as_ptr(),
            b"I\0".as_ptr(),
            &mut key_ver as *mut i32 as *mut c_void,
        )
    });
    if key_ver <= 0 {
        if uc.version() < 5000 {
            key_ver = 4003;
        } else if uc.version() < 6000 {
            key_ver = 4004;
        } else {
            key_ver = 4005;
        }
    }

    let mut num_keys: usize = 0;
    // SAFETY: fmt `'Z'` pairs with the `*mut usize` out-pointer `&mut num_keys`,
    // which is a live local.
    ufbxi_check!(
        uc,
        unsafe {
            find_val1(
                node,
                sp::KeyCount.as_ptr(),
                b"Z\0".as_ptr(),
                &mut num_keys as *mut usize as *mut c_void,
            )
        },
        "ufbxi_find_val1(node, ufbxi_KeyCount, \"Z\", &num_keys)"
    );
    // SAFETY: `curve` is the fresh non-null element pushed above.
    unsafe {
        (*curve).keyframes.data = uc.result_view().push::<Keyframe>(num_keys);
        (*curve).keyframes.count = num_keys;
        ufbxi_check!(
            uc,
            !(*curve).keyframes.data.is_null(),
            "curve->keyframes.data"
        );
    }

    let mut slope_left: f32 = 0.0f32;
    let mut weight_left: f32 = 0.333333f32;

    let mut next_time: f64 = 0.0;
    let mut next_value: f64 = 0.0;
    let mut prev_time: f64 = 0.0;

    // The pre-7000 keyframe data is stored as a _heterogenous_ array containing 64-bit integers,
    // floating point values, and _bare characters_. We cast all values to double and interpret them.
    // SAFETY: `keys` is non-null (checked above) and `find_array` returns the
    // node's own array descriptor, live for as long as the parse tree, whose `'d'`
    // payload is `size` `double`s — `size` being that payload's element count, so
    // the step reaches its one-past-the-end.
    let (mut data, data_end): (*mut f64, *mut f64) = unsafe {
        let data: *mut f64 = (*keys).data as *mut f64;
        (data, add_ptr(data, (*keys).size))
    };

    if num_keys > 0 {
        // SAFETY: `data` and `data_end` delimit the same `'d'` payload, so their
        // difference is well defined.
        ufbxi_check!(
            uc,
            unsafe { data_end.offset_from(data) } >= 2,
            "data_end - data >= 2"
        );
        // SAFETY: the check above leaves at least two doubles between `data` and
        // `data_end`, so offsets 0 and 1 are in bounds of the payload.
        unsafe {
            next_time = *data.add(0) / uc.ktime_sec_double();
            next_value = *data.add(1);
        }
    }

    for i in 0..num_keys {
        // SAFETY: `curve->keyframes.data` is the `num_keys`-element run allocated
        // and checked non-null above, and `i < num_keys` bounds the step.
        let key: *mut Keyframe = unsafe { ((*curve).keyframes.data as *mut Keyframe).add(i) };

        if i == 0 {
            // SAFETY: `curve` is the fresh non-null element pushed above.
            unsafe {
                (*curve).min_value = next_value as Real;
                (*curve).max_value = next_value as Real;
            }
        } else {
            // SAFETY: as above.
            unsafe {
                (*curve).min_value = min_real((*curve).min_value, next_value as Real);
                (*curve).max_value = max_real((*curve).max_value, next_value as Real);
            }
        }

        // First three values: Time, Value, InterpolationMode
        // SAFETY: `data` and `data_end` delimit the same `'d'` payload, so their
        // difference is well defined.
        ufbxi_check!(
            uc,
            unsafe { data_end.offset_from(data) } >= 3,
            "data_end - data >= 3"
        );
        // SAFETY: `key` is the in-bounds keyframe computed above.
        unsafe {
            (*key).time = next_time;
            (*key).value = next_value as Real;
        }
        // SAFETY: the check above leaves at least three doubles between `data` and
        // `data_end`, so offset 2 is in bounds of the payload.
        let mode: u8 = double_to_char(unsafe { *data.add(2) });
        // SAFETY: as above — three doubles remain, so stepping three stays within
        // the payload (at most one past its end).
        data = unsafe { data.add(3) };

        let mut slope_right: f32 = 0.0f32;
        let mut weight_right: f32 = 0.333333f32;
        let mut next_slope_left: f32 = 0.0f32;
        let mut next_weight_left: f32 = 0.333333f32;
        let mut auto_slope: bool = false;

        if mode == b'U' {
            // Cubic interpolation
            // SAFETY: `key` is the in-bounds keyframe computed above.
            unsafe {
                (*key).interpolation = Interpolation::Cubic;
            }

            // SAFETY: `data` and `data_end` delimit the same `'d'` payload, so
            // their difference is well defined.
            ufbxi_check!(
                uc,
                unsafe { data_end.offset_from(data) } >= 1,
                "data_end - data >= 1"
            );
            // SAFETY: the check above leaves at least one double between `data`
            // and `data_end`, so offset 0 is in bounds of the payload.
            let slope_mode: u8 = double_to_char(unsafe { *data.add(0) });
            // SAFETY: as above — one double remains, so stepping one stays within
            // the payload (at most one past its end).
            data = unsafe { data.add(1) };

            let mut num_weights: usize = 1;
            if slope_mode == b's' || slope_mode == b'b' {
                // Slope mode 's'/'b' (standard? broken?) always have two explicit slopes
                // TODO: `b` might actually be some kind of TCB curve
                // SAFETY: `data` and `data_end` delimit the same `'d'` payload, so
                // their difference is well defined.
                ufbxi_check!(
                    uc,
                    unsafe { data_end.offset_from(data) } >= 2,
                    "data_end - data >= 2"
                );
                // SAFETY: the check above leaves at least two doubles between
                // `data` and `data_end`, so offsets 0 and 1 are in bounds of the
                // payload and stepping two stays within it (at most one past its
                // end).
                unsafe {
                    slope_right = *data.add(0) as f32;
                    next_slope_left = *data.add(1) as f32;
                    data = data.add(2);
                }
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
                // SAFETY: `data` and `data_end` delimit the same `'d'` payload, so
                // their difference is well defined.
                ufbxi_check!(
                    uc,
                    unsafe { data_end.offset_from(data) } >= 2,
                    "data_end - data >= 2"
                );
                // SAFETY: the check above leaves at least two doubles between
                // `data` and `data_end`, so stepping two stays within the payload
                // (at most one past its end).
                data = unsafe { data.add(2) };
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
                // SAFETY: `data` and `data_end` delimit the same `'d'` payload, so
                // their difference is well defined.
                ufbxi_check!(
                    uc,
                    unsafe { data_end.offset_from(data) } >= 2,
                    "data_end - data >= 2"
                );
                // SAFETY: the check above leaves at least two doubles between
                // `data` and `data_end`, so stepping two stays within the payload
                // (at most one past its end).
                data = unsafe { data.add(2) };
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
                // SAFETY: `data` and `data_end` delimit the same `'d'` payload, so
                // their difference is well defined.
                ufbxi_check!(
                    uc,
                    unsafe { data_end.offset_from(data) } >= 3,
                    "data_end - data >= 3"
                );
                // SAFETY: the check above leaves at least three doubles between
                // `data` and `data_end`, so stepping three stays within the payload
                // (at most one past its end).
                data = unsafe { data.add(3) };
                num_weights = 0;
            } else if slope_mode == b'd' {
                // TODO: What is this mode? It has a single parameter (currently observed `0`)
                // and a single weight.
                // TODO: Solve what this is more thoroughly, using auto slope for now to reduce artifacts
                auto_slope = true;
                // SAFETY: `data` and `data_end` delimit the same `'d'` payload, so
                // their difference is well defined.
                ufbxi_check!(
                    uc,
                    unsafe { data_end.offset_from(data) } >= 1,
                    "data_end - data >= 1"
                );
                // SAFETY: the check above leaves at least one double between
                // `data` and `data_end`, so stepping one stays within the payload
                // (at most one past its end).
                data = unsafe { data.add(1) };
            } else {
                ufbxi_fail!(uc, "Unknown slope mode");
            }

            // C: `for (; num_weights > 0; num_weights--)`
            while num_weights > 0 {
                // SAFETY: `data` and `data_end` delimit the same `'d'` payload, so
                // their difference is well defined.
                ufbxi_check!(
                    uc,
                    unsafe { data_end.offset_from(data) } >= 1,
                    "data_end - data >= 1"
                );
                // SAFETY: the check above leaves at least one double between
                // `data` and `data_end`, so offset 0 is in bounds of the payload.
                let weight_mode: u8 = double_to_char(unsafe { *data.add(0) });
                // SAFETY: as above — one double remains, so stepping one stays
                // within the payload (at most one past its end).
                data = unsafe { data.add(1) };

                if weight_mode == b'n' {
                    // Automatic weights (0.3333...)
                } else if weight_mode == b'a' {
                    // Manual weights: RightWeight, NextLeftWeight
                    // SAFETY: `data` and `data_end` delimit the same `'d'` payload,
                    // so their difference is well defined.
                    ufbxi_check!(
                        uc,
                        unsafe { data_end.offset_from(data) } >= 2,
                        "data_end - data >= 2"
                    );
                    // SAFETY: the check above leaves at least two doubles between
                    // `data` and `data_end`, so offsets 0 and 1 are in bounds and
                    // stepping two stays within the payload (at most one past its
                    // end).
                    unsafe {
                        weight_right = *data.add(0) as f32;
                        next_weight_left = *data.add(1) as f32;
                        data = data.add(2);
                    }
                } else if weight_mode == b'l' {
                    // Next left tangent is weighted
                    // SAFETY: `data` and `data_end` delimit the same `'d'` payload,
                    // so their difference is well defined.
                    ufbxi_check!(
                        uc,
                        unsafe { data_end.offset_from(data) } >= 1,
                        "data_end - data >= 1"
                    );
                    // SAFETY: the check above leaves at least one double between
                    // `data` and `data_end`, so offset 0 is in bounds and stepping
                    // one stays within the payload (at most one past its end).
                    unsafe {
                        next_weight_left = *data.add(0) as f32;
                        data = data.add(1);
                    }
                } else if weight_mode == b'r' {
                    // Right tangent is weighted
                    // SAFETY: `data` and `data_end` delimit the same `'d'` payload,
                    // so their difference is well defined.
                    ufbxi_check!(
                        uc,
                        unsafe { data_end.offset_from(data) } >= 1,
                        "data_end - data >= 1"
                    );
                    // SAFETY: the check above leaves at least one double between
                    // `data` and `data_end`, so offset 0 is in bounds and stepping
                    // one stays within the payload (at most one past its end).
                    unsafe {
                        weight_right = *data.add(0) as f32;
                        data = data.add(1);
                    }
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
            // SAFETY: `key` is the in-bounds keyframe computed above.
            unsafe {
                (*key).interpolation = Interpolation::Linear;
            }
        } else if mode == b'C' {
            // Constant interpolation: Single parameter (use prev/next)
            if key_ver >= 4004 {
                // SAFETY: `data` and `data_end` delimit the same `'d'` payload, so
                // their difference is well defined.
                ufbxi_check!(
                    uc,
                    unsafe { data_end.offset_from(data) } >= 1,
                    "data_end - data >= 1"
                );
                // SAFETY: `key` is the in-bounds keyframe computed above, and the
                // check above leaves at least one double between `data` and
                // `data_end`, so offset 0 is in bounds of the payload.
                unsafe {
                    (*key).interpolation = if double_to_char(*data.add(0)) == b'n' {
                        Interpolation::ConstantNext
                    } else {
                        Interpolation::ConstantPrev
                    };
                    data = data.add(1);
                }
            } else {
                // SAFETY: `key` is the in-bounds keyframe computed above.
                unsafe {
                    (*key).interpolation = Interpolation::ConstantPrev;
                }
            }
        } else {
            ufbxi_fail!(uc, "Unknown key mode");
        }

        // Retrieve next key and value
        if i + 1 < num_keys {
            // SAFETY: `data` and `data_end` delimit the same `'d'` payload, so
            // their difference is well defined.
            ufbxi_check!(
                uc,
                unsafe { data_end.offset_from(data) } >= 2,
                "data_end - data >= 2"
            );
            // SAFETY: the check above leaves at least two doubles between `data`
            // and `data_end`, so offsets 0 and 1 are in bounds of the payload.
            unsafe {
                next_time = *data.add(0) / uc.ktime_sec_double();
                next_value = *data.add(1);
            }
        }

        if auto_slope {
            if i > 0 {
                // C: `slope_left = slope_right = ufbxi_solve_auto_tangent(...)`
                // SAFETY: `key` is the in-bounds keyframe for index `i`, and this
                // branch runs only for `i > 0`, so `key.offset(-1)` is the
                // previous keyframe of the same run — also in bounds and already
                // written by the previous iteration.
                slope_right = solve_auto_tangent(
                    uc,
                    prev_time,
                    unsafe { (*key).time },
                    next_time,
                    unsafe { (*key.offset(-1)).value },
                    unsafe { (*key).value },
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
        // SAFETY: `key` is the in-bounds keyframe computed above, whose
        // `interpolation` was written by the mode dispatch earlier in this
        // iteration.
        if unsafe { (*key).interpolation } == Interpolation::Linear {
            // SAFETY: `key` is the in-bounds keyframe computed above, whose `time`
            // and `value` were written earlier in this iteration.
            if next_time > unsafe { (*key).time } {
                let slope: f64 = (next_value - as_f64!(unsafe { (*key).value }))
                    / (next_time - unsafe { (*key).time });
                // C: `slope_right = next_slope_left = (float)slope;`
                next_slope_left = slope as f32;
                slope_right = next_slope_left;
            } else {
                // C: `slope_right = next_slope_left = 0.0f;`
                next_slope_left = 0.0f32;
                slope_right = next_slope_left;
            }
        }

        // SAFETY: `key` is the in-bounds keyframe computed above, whose `time` was
        // written earlier in this iteration.
        if unsafe { (*key).time } > prev_time {
            // SAFETY: as above.
            let delta: f64 = unsafe { (*key).time } - prev_time;
            // SAFETY: `key` is the in-bounds keyframe computed above; `left.dx`
            // is written before it is read on the next line.
            unsafe {
                (*key).left.dx = (weight_left as f64 * delta) as f32;
                (*key).left.dy = (*key).left.dx * slope_left;
            }
        } else {
            // SAFETY: `key` is the in-bounds keyframe computed above.
            unsafe {
                (*key).left.dx = 0.0f32;
                (*key).left.dy = 0.0f32;
            }
        }

        // SAFETY: `key` is the in-bounds keyframe computed above, whose `time` was
        // written earlier in this iteration.
        if next_time > unsafe { (*key).time } {
            // SAFETY: as above.
            let delta: f64 = next_time - unsafe { (*key).time };
            // SAFETY: `key` is the in-bounds keyframe computed above; `right.dx`
            // is written before it is read on the next line.
            unsafe {
                (*key).right.dx = (weight_right as f64 * delta) as f32;
                (*key).right.dy = (*key).right.dx * slope_right;
            }
        } else {
            // SAFETY: `key` is the in-bounds keyframe computed above.
            unsafe {
                (*key).right.dx = 0.0f32;
                (*key).right.dy = 0.0f32;
            }
        }

        slope_left = next_slope_left;
        weight_left = next_weight_left;
        // SAFETY: `key` is the in-bounds keyframe computed above, whose `time` was
        // written earlier in this iteration.
        prev_time = unsafe { (*key).time };
    }

    ufbxi_check!(uc, data == data_end, "data == data_end");

    Ok(())
}

// Recursion limited as it is further called only for `name="T"/"R"/"S"` and
// cannot enter the `name=="Transform"` branch.
// ufbx.c:15586-15662 `ufbxi_read_take_prop_channel`
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

// ufbx.c:15590-15662 `ufbxi_read_take_prop_channel_rec` (the
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
        // SAFETY: contiguous push_pop child run, valid for `node`'s borrow.
        for child in
            unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) }
        {
            if child.name() != sp::Channel.as_ptr() {
                continue;
            }

            let mut old_name: *const u8 = core::ptr::null();
            // SAFETY: fmt `'C'` pairs with the `*mut *const u8` out-pointer
            // `&mut old_name`, which is a live local.
            ufbxi_check!(
                uc,
                unsafe {
                    get_val1(
                        child,
                        b"C\0".as_ptr(),
                        &mut old_name as *mut *const u8 as *mut c_void,
                    )
                },
                "ufbxi_get_val1(child, \"C\", (char**)&old_name)"
            );

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
            let suffix: *const u8 = b" (Shape)\0".as_ptr();
            // SAFETY: `suffix` is a NUL-terminated literal, so `strlen` walks to
            // its terminator.
            let suffix_len: usize = unsafe { strlen(suffix) };
            if name.length > suffix_len
                // SAFETY: `name.data` spans `name.length` readable bytes (fn
                // contract) and `name.length > suffix_len` (checked first), so the
                // compared range is that string's last `suffix_len` bytes;
                // `suffix` spans `suffix_len` bytes by construction.
                && unsafe {
                    memcmp(
                        add_ptr(name.data as *mut u8, name.length).wrapping_sub(suffix_len)
                            as *const u8,
                        suffix,
                        suffix_len,
                    )
                } == 0
            {
                name.length -= suffix_len;
                // SAFETY: `uc.string_pool_mut_ptr()` is uc's own live `string_pool`
                // and `&mut name` is a live local whose `data` spans the shortened
                // `length` bytes.
                unsafe { push_string_place_str(uc.string_pool_mut_ptr(), &mut name, false) }?;
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
            // SAFETY: contiguous push_pop child run, valid for `node`'s borrow.
            for child in unsafe {
                SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize)
            } {
                if child.name() != sp::Channel.as_ptr() {
                    continue;
                }
                if find_child(child, sp::Key.as_ptr()).is_none()
                    && find_child(child, sp::Default.as_ptr()).is_none()
                {
                    continue;
                }
                // SAFETY: fmt `'C'` pairs with the `*mut *const u8` out-pointer
                // `&mut channel_names[num_channel_nodes]`, an element of a live
                // local array — `num_channel_nodes < 3` holds because the loop
                // breaks as soon as it reaches 3.
                if !unsafe {
                    get_val1(
                        child,
                        b"C\0".as_ptr(),
                        &mut channel_names[num_channel_nodes] as *mut *const u8 as *mut c_void,
                    )
                } {
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
        // C-parity: upstream does NOT `ufbxi_check(value)` here (ufbx.c:15650);
        // on allocation failure `ufbxi_connect_oo` below fails first. Kept as-is.
        // SAFETY: `&mut value_fbx_id` is a live local `uint64_t` slot and
        // `name.data` is the caller's NUL-terminated interned name; `AnimValue` is
        // the element struct for `ElementType::AnimValue`.
        let value: *mut AnimValue = unsafe {
            push_synthetic_element::<AnimValue>(
                uc,
                &mut value_fbx_id,
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
            // SAFETY: `value` is the live `ufbx_anim_value` null-checked above.
            // `default_value` is a three-`ufbx_real` union and
            // `i < num_channel_nodes <= 3` bounds the step; `channel_names[i]` is
            // the NUL-terminated `'C'` value fetched for that channel.
            unsafe {
                read_take_anim_channel(
                    uc,
                    channel_nodes[i].unwrap(),
                    value_fbx_id,
                    channel_names[i],
                    (&raw mut (*value).default_value as *mut Real).add(i),
                )
            }?;
        }
    }

    Ok(())
}

// ufbx.c:15664-15686 `ufbxi_read_take_object`
#[inline(never)]
pub(crate) fn read_take_object(
    uc: &Context,
    node: &NodeView,
    layer_fbx_id: u64,
) -> Result<(), Fail> {
    // Takes are used only in pre-7000 FBX versions so objects are identified
    // by their unique Type::Name pair that we use as unique IDs through the
    // pooled interned string pointers.
    let mut type_and_name: *const u8 = core::ptr::null();
    // SAFETY: `node` is a parse-tree NodeView and `type_and_name` is an
    // unaliased local `char*` slot, matching the `c` format; on success it
    // holds a pooled NUL-terminated string, which is what the id hash reads.
    ufbxi_check!(
        uc,
        unsafe {
            get_val1(
                node,
                b"c\0".as_ptr(),
                &mut type_and_name as *mut *const u8 as *mut c_void,
            )
        },
        "ufbxi_get_val1(node, \"c\", (char**)&type_and_name)"
    );
    let target_fbx_id: u64 = synthetic_id_from_string(uc, type_and_name);
    ufbxi_check!(uc, target_fbx_id != 0, "target_fbx_id");

    // Add all suitable Channels as animated properties
    // C: `ufbxi_for(ufbxi_node, child, node->children, node->num_children)`
    // SAFETY: contiguous push_pop child run, valid for `node`'s borrow.
    for child in
        unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) }
    {
        // C: `ufbx_string name;` — written by the `ufbxi_get_val1` guard below.
        // SAFETY: all-zero is a valid `ufbx_string`; `child` is a NodeView from
        // `node`'s own child run and `name` is an unaliased local of exactly
        // the type the `S` format writes, so on success it is pooled and safe
        // to hand to the channel reader.
        let mut name: String = unsafe { core::mem::zeroed() };
        if child.name() != sp::Channel.as_ptr() {
            continue;
        }
        unsafe {
            if !get_val1(
                child,
                b"S\0".as_ptr(),
                &mut name as *mut String as *mut c_void,
            ) {
                continue;
            }

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

    let mut start: i64 = 0;
    let mut stop: i64 = 0;
    // SAFETY: `node` is a parse-tree NodeView; `start`/`stop` are unaliased
    // locals of the `int64_t` type the `LL` format writes, and the synthetic
    // props are initialized in place into the local `tmp_props` array — at most
    // four are ever written, which is its length — from static pooled names.
    unsafe {
        if find_val2(
            node,
            sp::LocalTime.as_ptr(),
            b"LL\0".as_ptr(),
            &mut start as *mut i64 as *mut c_void,
            &mut stop as *mut i64 as *mut c_void,
        ) {
            init_synthetic_int_prop(
                &mut tmp_props[num_props as usize],
                sp::LocalStart.as_ptr(),
                start,
                PropType::Integer,
            );
            num_props += 1;
            init_synthetic_int_prop(
                &mut tmp_props[num_props as usize],
                sp::LocalStop.as_ptr(),
                stop,
                PropType::Integer,
            );
            num_props += 1;
        }
        if find_val2(
            node,
            sp::ReferenceTime.as_ptr(),
            b"LL\0".as_ptr(),
            &mut start as *mut i64 as *mut c_void,
            &mut stop as *mut i64 as *mut c_void,
        ) {
            init_synthetic_int_prop(
                &mut tmp_props[num_props as usize],
                sp::ReferenceStart.as_ptr(),
                start,
                PropType::Integer,
            );
            num_props += 1;
            init_synthetic_int_prop(
                &mut tmp_props[num_props as usize],
                sp::ReferenceStop.as_ptr(),
                stop,
                PropType::Integer,
            );
            num_props += 1;
        }
    }

    // C: `const char *name;` — written by the `ufbxi_get_val1` check below.
    let mut name: *const u8 = core::ptr::null();
    // SAFETY: `node` is a parse-tree NodeView and `name` is an unaliased local
    // `char*` slot, matching the `C` format; on success it is a pooled string.
    ufbxi_check!(
        uc,
        unsafe {
            get_val1(
                node,
                b"C\0".as_ptr(),
                &mut name as *mut *const u8 as *mut c_void,
            )
        },
        "ufbxi_get_val1(node, \"C\", (char**)&name)"
    );

    // Hack: For post-7000 files we are only interested in the animation times
    // for fallback in case the information is missing in the stacks.
    if uc.version() >= 7000 {
        let hash: u32 = crate::native::hash::hash_ptr!(name);
        let entry: *mut TmpAnimStack = uc
            .anim_stack_map_view()
            .find::<TmpAnimStack, _>(hash, &name);

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
            &mut stack_fbx_id,
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
            &mut layer_fbx_id,
            Some(node),
            sp::BaseLayer.as_ptr(),
            ElementType::AnimLayer,
        )
    };
    ufbxi_check!(uc, !layer.is_null(), "layer");

    connect_oo(uc, layer_fbx_id, stack_fbx_id)?;

    // Read all properties of objects included in the take
    // C: `ufbxi_for(ufbxi_node, child, node->children, node->num_children)`
    // SAFETY: contiguous push_pop child run, valid for `node`'s borrow.
    for child in
        unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) }
    {
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

    // SAFETY: `node` is a parse-tree NodeView; the name is a NUL-terminated
    // literal, which is what the by-content child search compares against.
    let frame_rate = unsafe { find_child_strcmp(node, b"FrameRate\0".as_ptr()) };
    if let Some(frame_rate) = frame_rate {
        let mut fps: f64 = 0.0;
        // SAFETY: `frame_rate` is a child NodeView of `node`; `fps` and `str_`
        // are unaliased locals of exactly the types the `D` / `S` formats
        // write, and on success `str_` is a pooled `data`/`length` pair, so the
        // double parse and the end-of-string compare stay inside it. The two
        // synthetic props are written into the local 2-element `tmp_props`.
        unsafe {
            if !get_val1(
                frame_rate,
                b"D\0".as_ptr(),
                &mut fps as *mut f64 as *mut c_void,
            ) {
                // C: `ufbx_string str;` — written by the `ufbxi_get_val1()` below.
                let mut str_: String = core::mem::zeroed();
                if get_val1(
                    frame_rate,
                    b"S\0".as_ptr(),
                    &mut str_ as *mut String as *mut c_void,
                ) {
                    // C: `char *end;` — written by `ufbxi_parse_double()`.
                    let mut end: *const u8 = core::ptr::null();
                    let val: f64 =
                        parse_double(str_.data, str_.length, &mut end, uc.double_parse_flags());
                    if end == str_.data.add(str_.length) {
                        fps = val;
                    }
                }
            }
            if fps > 0.0 {
                init_synthetic_real_prop(
                    &mut tmp_props[num_props as usize],
                    sp::CustomFrameRate.as_ptr(),
                    fps as Real,
                    PropType::Number,
                );
                num_props += 1;
                init_synthetic_real_prop(
                    &mut tmp_props[num_props as usize],
                    sp::TimeMode.as_ptr(),
                    // C: `UFBX_TIME_MODE_CUSTOM` implicitly converted to `ufbx_real`.
                    TimeMode::Custom as u32 as Real,
                    PropType::Integer,
                );
                num_props += 1;
            }
        }
    }

    if num_props > 0 {
        // SAFETY: `props_mut_ptr()` addresses uc's own scene-settings
        // `ufbx_props`, reached through its element views — write-capable
        // provenance, stable for this call.
        let props: &PropsView =
            unsafe { PropsView::from_ptr(uc.scene_view().settings_view().props_mut_ptr()) };
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

            sort_properties(uc, new_props, new_count)?;
            (*props.props_raw()).data = new_props;
            (*props.props_raw()).count = new_count;
            deduplicate_properties(props.props_raw());
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
pub(crate) unsafe fn unscaled_transform_to_matrix(t: *const Transform) -> Matrix {
    // SAFETY: `t` is a live, initialized `ufbx_transform` (fn contract).
    let mut transform: Transform = unsafe { *t };
    transform.scale.x = 1.0;
    transform.scale.y = 1.0;
    transform.scale.z = 1.0;
    // SAFETY: `&transform` is a live, fully initialized local.
    unsafe { transform_to_matrix(&transform) }
}

// ufbx.c:15827-15837 `ufbxi_setup_root_node`
#[inline(never)]
pub(crate) unsafe fn setup_root_node(uc: &Context, root: *mut UfbxNode) {
    if uc.opts_view().use_root_transform() {
        // SAFETY: `root` is the caller's live root `ufbx_node` (fn contract);
        // `root_transform_ptr()` points at uc's own live, initialized
        // `opts.root_transform`.
        unsafe {
            (*root).local_transform = uc.opts_view().root_transform();
            (*root).node_to_parent = transform_to_matrix(uc.opts_view().root_transform_ptr());
        }
    } else {
        // SAFETY: `root` is the caller's live root `ufbx_node` (fn contract).
        unsafe {
            (*root).local_transform = IDENTITY_TRANSFORM;
            (*root).node_to_parent = IDENTITY_MATRIX;
        }
    }
    // SAFETY: as above.
    unsafe {
        (*root).is_root = true;
    }
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
            ufbxi_ignore!(unsafe {
                get_val1(
                    top_node,
                    b"S\0".as_ptr(),
                    uc.scene_view().metadata_view().creator_mut_ptr() as *mut core::ffi::c_void,
                )
            });
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
        // each 12 bytes plus the NUL, so the requested length is in bounds; the
        // intern goes to uc's own string pool and its result — checked non-null
        // — is the pooled string the synthetic-id hash reads.
        unsafe {
            root_name = sp::push_string_imp(
                uc.string_pool_mut_ptr(),
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
        // unaliased local, and `root` is the fresh non-null element the push
        // returns — checked before it is initialized and before its
        // `element_id` is copied into uc's own `tmp_node_ids` buffer.
        unsafe {
            let mut root_info: ElementInfo = core::mem::zeroed();
            root_info.fbx_id = uc.root_id();
            root_info.name = EMPTY_STRING.0;
            let root: *mut UfbxNode =
                push_element::<UfbxNode>(uc, &mut root_info, ElementType::Node);
            ufbxi_check!(uc, !root.is_null(), "root");
            setup_root_node(uc, root);
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
        // SAFETY: `top_node` is the parsed top-level NodeView; the name is a
        // NUL-terminated literal for the by-content child search.
        let settings = unsafe { find_child_strcmp(top_node, b"Settings\0".as_ptr()) };
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
    pub prop_name: *const u8,
    pub prop_type: PropType,
    pub node_name: *const u8,
    pub node_fmt: *const u8,
}
// The tables below are immutable and their `const char *` members reference
// immutable statics/string literals, so sharing is sound (same rationale as
// `ScaleHelperProp`).
unsafe impl Sync for LegacyProp {}

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
pub(crate) unsafe fn read_legacy_prop(
    node: &NodeView,
    prop: *mut Prop,
    legacy_prop: *const LegacyProp,
) -> bool {
    let mut value_ix: usize = 0;
    let mut flags: u32 = 0;

    // SAFETY: `prop` is the caller's live, writable `ufbx_prop` scratch slot (fn
    // contract) — write-capable provenance, stable for this call.
    let prop: &PropView = unsafe { PropView::from_ptr(prop) };
    // C-parity: `prop->value_real_arr` / `prop->value_real` are the first
    // members of `ufbx_prop`'s value union, which `generated.rs` collapses to
    // `value_vec4` — the four-`ufbx_real` union arm the C code indexes as
    // `value_real_arr`.
    let value_real_arr: *mut Real = prop.value_vec4_raw() as *mut Real;

    // SAFETY: `legacy_prop` is the caller's live `ufbxi_legacy_prop` (fn
    // contract).
    let fmt: *const u8 = unsafe { (*legacy_prop).node_fmt };
    let mut fmt_ix: usize = 0;
    // SAFETY: `fmt` is a NUL-terminated format literal from the legacy-prop
    // table, and the loop stops at that terminator, so `fmt_ix` stays in bounds.
    while unsafe { *fmt.add(fmt_ix) } != 0 {
        // SAFETY: as above — `fmt_ix` addresses the non-NUL byte just tested.
        let c: u8 = unsafe { *fmt.add(fmt_ix) };
        match c {
            b'L' => {
                ufbx_assert!(value_ix == 0);
                // SAFETY: fmt `'L'` pairs with the `*mut i64` out-pointer
                // `value_int_raw()`, the viewed prop's own `int64_t` field.
                if !unsafe { get_val_at(node, fmt_ix, b'L', prop.value_int_raw() as *mut c_void) } {
                    return false;
                }
                // SAFETY: `value_real_arr` spans the prop's four-`ufbx_real` union
                // arm, so offsets 0..3 are in bounds.
                unsafe {
                    *value_real_arr.add(0) = prop.value_int() as Real;
                    *value_real_arr.add(1) = 0.0;
                    *value_real_arr.add(2) = 0.0;
                    *value_real_arr.add(3) = 0.0;
                }
                prop.set_value_str(EMPTY_STRING.0);
                prop.set_value_blob(EMPTY_BLOB.0);
                flags |= PropFlags::VALUE_INT.raw();
                value_ix += 1;
            }
            b'R' => {
                ufbx_assert!(value_ix < 4);
                // SAFETY: fmt `'R'` pairs with a `*mut ufbx_real` out-pointer, and
                // `value_ix < 4` bounds the step inside the prop's four-`ufbx_real`
                // union arm: the `'R'` arm is the only one that advances `value_ix`
                // past 1, and the longest `node_fmt` in the legacy-prop tables is
                // `b"RRR\0"`, so `value_ix` reaches at most 2 at this point.
                if !unsafe {
                    get_val_at(
                        node,
                        fmt_ix,
                        b'R',
                        value_real_arr.add(value_ix) as *mut c_void,
                    )
                } {
                    return false;
                }
                if value_ix == 0 {
                    // C: `ufbxi_f64_to_i64(prop->value_real)` — `ufbx_real`
                    // argument promoted to the `double` parameter.
                    // SAFETY: `value_real_arr` spans the prop's four-`ufbx_real`
                    // union arm, so offsets 0..3 are in bounds, and offset 0 was
                    // just written by the fetch above.
                    unsafe {
                        prop.set_value_int(f64_to_i64(as_f64!(*value_real_arr.add(0))));
                        *value_real_arr.add(1) = 0.0;
                        *value_real_arr.add(2) = 0.0;
                        *value_real_arr.add(3) = 0.0;
                    }
                    prop.set_value_str(EMPTY_STRING.0);
                    prop.set_value_blob(EMPTY_BLOB.0);
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
                // SAFETY: fmt `'S'` pairs with the `*mut ufbx_string` out-pointer
                // `value_str_raw()`, the viewed prop's own `ufbx_string` field.
                if !unsafe { get_val_at(node, fmt_ix, b'S', prop.value_str_raw() as *mut c_void) } {
                    return false;
                }
                if prop.value_str().length > 0 {
                    // SAFETY: fmt `'b'` pairs with the `*mut ufbx_blob`
                    // out-pointer `value_blob_raw()`, the viewed prop's own
                    // `ufbx_blob` field.
                    let found: bool = unsafe {
                        get_val_at(node, fmt_ix, b'b', prop.value_blob_raw() as *mut c_void)
                    };
                    ufbxi_ignore!(found);
                    ufbx_assert!(found);
                } else {
                    prop.set_value_blob(EMPTY_BLOB.0);
                }
                // SAFETY: `value_real_arr` spans the prop's four-`ufbx_real` union
                // arm, so offsets 0..3 are in bounds.
                unsafe {
                    *value_real_arr.add(0) = 0.0;
                    *value_real_arr.add(1) = 0.0;
                    *value_real_arr.add(2) = 0.0;
                    *value_real_arr.add(3) = 0.0;
                }
                prop.set_value_int(0);
                flags |= PropFlags::VALUE_STR.raw();
                value_ix += 1;
            }
            b'_' => {}
            _ => {
                ufbxi_unreachable!("Unhandled legacy fmt");
            }
        }
        fmt_ix += 1;
    }

    prop.set_flags(PropFlags::from_raw(flags));

    true
}

// ufbx.c:16052-16072 `ufbxi_read_legacy_props`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn read_legacy_props(
    node: &NodeView,
    props: *mut Prop,
    legacy_props: *const LegacyProp,
    num_legacy: usize,
) -> usize {
    let mut num_props: usize = 0;
    for legacy_ix in 0..num_legacy {
        // SAFETY: `legacy_props` points at `num_legacy` entries (fn contract) and
        // `legacy_ix < num_legacy` bounds the step.
        let legacy_prop: *const LegacyProp = unsafe { legacy_props.add(legacy_ix) };
        // SAFETY: `props` has room for `num_legacy` props (fn contract) and
        // `num_props <= legacy_ix < num_legacy`, since it grows by at most one per
        // iteration.
        let prop: *mut Prop = unsafe { props.add(num_props) };

        // SAFETY: `legacy_prop` is the in-bounds table entry computed above, whose
        // `node_name` is a NUL-terminated literal.
        let n: &NodeView = match unsafe { find_child_strcmp(node, (*legacy_prop).node_name) } {
            Some(n) => n,
            None => continue,
        };
        // SAFETY: `prop` is the in-bounds destination slot and `legacy_prop` the
        // in-bounds table entry, both computed above.
        if !unsafe { read_legacy_prop(n, prop, legacy_prop) } {
            continue;
        }

        // SAFETY: `prop` is the in-bounds destination slot and `legacy_prop` the
        // in-bounds table entry, whose `prop_name` is a NUL-terminated literal so
        // `strlen` walks to its terminator and the `name` written from it spans
        // `length` readable bytes for the key hash.
        unsafe {
            (*prop).name.data = (*legacy_prop).prop_name;
            (*prop).name.length = strlen((*legacy_prop).prop_name);
            (*prop)._internal_key = get_name_key((*prop).name.as_bytes());
            (*prop).flags = PropFlags::from_raw(0);
            (*prop).type_ = (*legacy_prop).prop_type;
        }
        num_props += 1;
    }

    num_props
}

// ufbx.c:16074-16090 `ufbxi_read_legacy_material`
#[inline(never)]
pub(crate) unsafe fn read_legacy_material(
    uc: &Context,
    node: &NodeView,
    p_fbx_id: *mut u64,
    name: *const u8,
) -> Result<(), Fail> {
    // SAFETY: `p_fbx_id` is the caller's writable `uint64_t` slot and `name` is
    // NUL-terminated (fn contract); `Material` is the element struct for
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
    // SAFETY: `tmp_props` has exactly `LEGACY_MATERIAL_PROPS_COUNT` slots, which
    // is also the entry count of the `LEGACY_MATERIAL_PROPS` table passed here.
    let num_props: usize = unsafe {
        read_legacy_props(
            node,
            tmp_props.as_mut_ptr(),
            LEGACY_MATERIAL_PROPS.as_ptr(),
            LEGACY_MATERIAL_PROPS_COUNT,
        )
    };

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
#[inline(never)]
pub(crate) unsafe fn read_legacy_link(
    uc: &Context,
    node: &NodeView,
    p_fbx_id: *mut u64,
    name: *const u8,
) -> Result<(), Fail> {
    // SAFETY: `p_fbx_id` is the caller's writable `uint64_t` slot and `name` is
    // NUL-terminated (fn contract); `SkinCluster` is the element struct for
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

        // SAFETY: `cluster` is the fresh non-null element pushed above, and
        // `transform`'s `'r'` payload holds at least 16 reals (checked), the
        // matrix element count `read_transform_matrix` reads.
        unsafe {
            read_transform_matrix(
                &mut (*cluster).mesh_node_to_bone,
                (*transform).data as *mut Real,
            );
            read_transform_matrix(
                &mut (*cluster).bind_to_world,
                (*transform_link).data as *mut Real,
            );
        }
    }

    Ok(())
}

// ufbx.c:16123-16136 `ufbxi_read_legacy_light`
#[inline(never)]
pub(crate) unsafe fn read_legacy_light(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Light` is the
    // element struct for `ElementType::Light`.
    let light: *mut Light = unsafe { push_element::<Light>(uc, info, ElementType::Light) };
    ufbxi_check!(uc, !light.is_null(), "light");

    // C: `ufbx_prop tmp_props[ufbxi_arraycount(ufbxi_legacy_light_props)];`
    // SAFETY: `ufbx_prop` is a C aggregate of pointer/length pairs, scalars and
    // enum tags, for which the all-zero bit pattern is a valid value.
    let mut tmp_props: [Prop; LEGACY_LIGHT_PROPS_COUNT] = unsafe { core::mem::zeroed() };
    // SAFETY: `tmp_props` has exactly `LEGACY_LIGHT_PROPS_COUNT` slots, which is
    // also the entry count of the `LEGACY_LIGHT_PROPS` table passed here.
    let num_props: usize = unsafe {
        read_legacy_props(
            node,
            tmp_props.as_mut_ptr(),
            LEGACY_LIGHT_PROPS.as_ptr(),
            LEGACY_LIGHT_PROPS_COUNT,
        )
    };

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
pub(crate) unsafe fn read_legacy_camera(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Camera` is the
    // element struct for `ElementType::Camera`.
    let camera: *mut Camera = unsafe { push_element::<Camera>(uc, info, ElementType::Camera) };
    ufbxi_check!(uc, !camera.is_null(), "camera");

    // C: `ufbx_prop tmp_props[ufbxi_arraycount(ufbxi_legacy_camera_props)];`
    // SAFETY: `ufbx_prop` is a C aggregate of pointer/length pairs, scalars and
    // enum tags, for which the all-zero bit pattern is a valid value.
    let mut tmp_props: [Prop; LEGACY_CAMERA_PROPS_COUNT] = unsafe { core::mem::zeroed() };
    // SAFETY: `tmp_props` has exactly `LEGACY_CAMERA_PROPS_COUNT` slots, which is
    // also the entry count of the `LEGACY_CAMERA_PROPS` table passed here.
    let num_props: usize = unsafe {
        read_legacy_props(
            node,
            tmp_props.as_mut_ptr(),
            LEGACY_CAMERA_PROPS.as_ptr(),
            LEGACY_CAMERA_PROPS_COUNT,
        )
    };

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
pub(crate) unsafe fn read_legacy_limb_node(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Bone` is the
    // element struct for `ElementType::Bone`.
    let bone: *mut Bone = unsafe { push_element::<Bone>(uc, info, ElementType::Bone) };
    ufbxi_check!(uc, !bone.is_null(), "bone");

    // C: `ufbx_prop tmp_props[ufbxi_arraycount(ufbxi_legacy_bone_props)];`
    // SAFETY: `ufbx_prop` is a C aggregate of pointer/length pairs, scalars and
    // enum tags, for which the all-zero bit pattern is a valid value.
    let mut tmp_props: [Prop; LEGACY_BONE_PROPS_COUNT] = unsafe { core::mem::zeroed() };
    let mut num_props: usize = 0;

    // SAFETY: the name is a NUL-terminated literal.
    let prop_node = unsafe { find_child_strcmp(node, b"Properties\0".as_ptr()) };
    if let Some(prop_node) = prop_node {
        // SAFETY: `tmp_props` has exactly `LEGACY_BONE_PROPS_COUNT` slots, which
        // is also the entry count of the `LEGACY_BONE_PROPS` table passed here.
        num_props = unsafe {
            read_legacy_props(
                prop_node,
                tmp_props.as_mut_ptr(),
                LEGACY_BONE_PROPS.as_ptr(),
                LEGACY_BONE_PROPS_COUNT,
            )
        };
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
pub(crate) unsafe fn read_legacy_mesh(
    uc: &Context,
    node: &NodeView,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    // Only read polygon meshes, ignore eg. NURBS without error
    let node_vertices = find_child(node, sp::Vertices.as_ptr());
    let node_indices = find_child(node, sp::PolygonVertexIndex.as_ptr());
    if node_vertices.is_none() || node_indices.is_none() {
        return Ok(());
    }
    let node_vertices: &NodeView = node_vertices.unwrap();
    let node_indices: &NodeView = node_indices.unwrap();

    // SAFETY: `info` is the caller's live `ufbxi_element_info` and `Mesh` is the
    // element struct for `ElementType::Mesh`.
    let mesh: *mut Mesh = unsafe { push_element::<Mesh>(uc, info, ElementType::Mesh) };
    ufbxi_check!(uc, !mesh.is_null(), "mesh");
    // SAFETY: `mesh` is the fresh non-null element just pushed into uc's
    // `tmp_elements` arena (elements live there until finalize copies them into
    // the result arena) — reached through `*mut` (write-capable provenance for
    // `Mut`) and live for the borrow; the fields accessed below are initialized
    // at each use site, as this function fills them in.
    let mesh = unsafe { View::<Mesh>::from_ptr(mesh) };

    // SAFETY: `info` is the caller's live `ufbxi_element_info`.
    unsafe { read_synthetic_blend_shapes(uc, node, info) }?;

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
    // SAFETY: `vertices` is non-null (checked above) and `get_array` returns the
    // node's own array descriptor, live for as long as the parse tree.
    ufbxi_check!(
        uc,
        unsafe { (*vertices).size } % 3 == 0,
        "vertices->size % 3 == 0"
    );

    // SAFETY: `vertices` is the live array descriptor whose `'r'` payload holds
    // `size` reals, a multiple of 3 (checked), hence `size / 3` `ufbx_vec3`
    // positions; `indices` is likewise a live descriptor checked non-null above.
    unsafe {
        mesh.set_num_vertices((*vertices).size / 3);
        mesh.set_num_indices((*indices).size);
    }

    // SAFETY: as above — `indices`'s `'i'` payload is `size` `uint32_t`.
    let mut index_data: *mut u32 = unsafe { (*indices).data } as *mut u32;

    // Duplicate `index_data` for modification if we retain DOM
    if uc.opts_view().retain_dom() {
        // SAFETY: `uc.result_mut_ptr()` is uc's own live `result` buf and
        // `index_data` spans the `(*indices).size` `uint32_t` values copied.
        index_data = unsafe { push_copy::<u32>(uc.result_mut_ptr(), (*indices).size, index_data) };
        ufbxi_check!(uc, !index_data.is_null(), "index_data");
    }

    // SAFETY (both `vertices` reads): `vertices`'s payload is the
    // `num_vertices` `ufbx_vec3` run and `index_data` the `num_indices` index
    // run.
    mesh.vertices_view()
        .set_data(unsafe { (*vertices).data } as *const Vec3);
    mesh.vertex_indices_view().set_data(index_data);
    mesh.vertices_view().set_count(mesh.num_vertices());
    mesh.vertex_indices_view().set_count(mesh.num_indices());

    mesh.vertex_position().set_exists(true);
    mesh.vertex_position()
        .values_view()
        .set_data(unsafe { (*vertices).data } as *const Vec3);
    mesh.vertex_position()
        .values_view()
        .set_count(mesh.num_vertices());
    mesh.vertex_position().indices_view().set_data(index_data);
    mesh.vertex_position()
        .indices_view()
        .set_count(mesh.num_indices());
    mesh.vertex_position().set_unique_per_vertex(true);

    // Check/make sure that the last index is negated (last of polygon)
    if mesh.num_indices() > 0 {
        // SAFETY: `index_data` spans `num_indices` `uint32_t` values, so the last
        // one is in bounds under `num_indices > 0`.
        if unsafe { *index_data.add(mesh.num_indices() - 1) } as i32 >= 0 {
            if uc.opts_view().strict() {
                ufbxi_fail!(uc, "Non-negated last index");
            }
            // SAFETY: as above — `index_data` is writable, being either the
            // parse-tree array payload or the `result`-buf copy made above.
            unsafe {
                *index_data.add(mesh.num_indices() - 1) = !*index_data.add(mesh.num_indices() - 1);
            }
        }
    }

    // SAFETY: `index_data` is the mesh's `num_indices`-long index run,
    // installed above.
    unsafe { process_indices(uc, mesh, index_data) }?;

    // Normals are either per-vertex or per-index in legacy FBX files?
    // If the version is 5000 prefer per-vertex, otherwise per-index...
    let normals: *mut ValueArray = find_array(node, sp::Normals.as_ptr(), b'r');
    if !normals.is_null() {
        // SAFETY: `normals` is non-null (checked) and `find_array` returns the
        // node's own array descriptor, live for as long as the parse tree.
        let num_normals: usize = unsafe { (*normals).size } / 3;
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
            // SAFETY: `normals` is the live array descriptor checked non-null
            // above.
            mesh.vertex_normal()
                .values_view()
                .set_data(unsafe { (*normals).data } as *const Vec3);
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
            // SAFETY: `normals` is the live array descriptor checked non-null
            // above.
            mesh.vertex_normal()
                .values_view()
                .set_data(unsafe { (*normals).data } as *const Vec3);
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
        // SAFETY: `set` is the fresh zeroed `ufbx_uv_set` pushed above, checked
        // non-null.
        unsafe {
            (*set).index = 0;
            (*set).name.data = EMPTY_CHAR.as_ptr();
        }
        // SAFETY: `&mut (*set).vertex_uv` is the `ufbx_vertex_vec2` slot of the
        // fresh non-null set, which the `'r'`/2 attribute shape matches.
        unsafe {
            read_vertex_element(
                uc,
                mesh,
                uv_info,
                &mut (*set).vertex_uv as *mut VertexVec2 as *mut VertexAttrib,
                sp::TextureUV.as_ptr(),
                sp::TextureUVVerticeIndex.as_ptr(),
                core::ptr::null(),
                b'r',
                2,
            )
        }?;

        // `set` is the fresh single-element UV-set allocation pushed above.
        mesh.uv_sets_view().set_data(set);
        mesh.uv_sets_view().set_count(1);
        // C: `mesh->vertex_uv = set->vertex_uv;` — struct assignment is a
        // memcpy; `VertexVec2` is not `Copy` in `generated.rs`.
        // SAFETY: `set` is the live single-element allocation, distinct from the
        // mesh; its `vertex_uv` is initialized — zeroed by the `push_zero` above,
        // then possibly overwritten by `read_vertex_element` (which returns Ok
        // without touching it when the UV data array is missing or empty).
        mesh.set_vertex_uv(unsafe { core::ptr::read(&raw const (*set).vertex_uv) });
    }

    // Material indices
    {
        // C: `const char *mapping = NULL;`
        let mut mapping: *const u8 = core::ptr::null();
        // SAFETY: fmt `'C'` pairs with the `*mut *const u8` out-pointer
        // `&mut mapping`, which is a live local.
        ufbxi_check!(
            uc,
            unsafe {
                find_val1(
                    node,
                    sp::MaterialAssignation.as_ptr(),
                    b"C\0".as_ptr(),
                    &mut mapping as *mut *const u8 as *mut c_void,
                )
            },
            "ufbxi_find_val1(node, ufbxi_MaterialAssignation, \"C\", (char**)&mapping)"
        );
        if mapping == sp::ByPolygon.as_ptr() {
            // SAFETY: `face_material_raw()` addresses the mesh's own live list
            // field, so `&raw mut` projects the `data`/`count` slots the `'i'`
            // element type matches.
            unsafe {
                read_truncated_array(
                    uc,
                    &raw mut (*mesh.face_material_raw()).data as *mut c_void,
                    &raw mut (*mesh.face_material_raw()).count,
                    node,
                    sp::Materials.as_ptr(),
                    b'i',
                    mesh.num_faces(),
                )
            }?;
        } else if mapping == sp::AllSame.as_ptr() {
            let arr: *mut ValueArray = find_array(node, sp::Materials.as_ptr(), b'i');
            let mut material: u32 = 0;
            // SAFETY: `arr` is non-null (checked) and `find_array` returns the
            // node's own array descriptor, live for as long as the parse tree.
            if !arr.is_null() && unsafe { (*arr).size } >= 1 {
                // SAFETY: as above — the `'i'` payload holds `size >= 1`
                // `uint32_t`, so its first element is in bounds.
                material = unsafe { *((*arr).data as *mut u32) };
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
                // The run just pushed holds `num_faces` `uint32_t`, which is also
                // `face_material.count`.
                let mut p_mat: *mut u32 = mesh.face_material().data as *mut u32;
                let p_mat_end = add_ptr(p_mat, mesh.face_material().count);
                while p_mat != p_mat_end {
                    // SAFETY: `p_mat` walks the `face_material` run and stops at
                    // `p_mat_end`, so it is in bounds; stepping one past the last
                    // entry reaches `p_mat_end`, the one-past-the-end pointer.
                    unsafe {
                        *p_mat = material;
                        p_mat = p_mat.add(1);
                    }
                }
            }
        }
    }

    let mut skin_fbx_id: u64 = 0;
    let mut skin: *mut SkinDeformer = core::ptr::null_mut();

    // Materials, Skin Clusters
    // C: `ufbxi_for(ufbxi_node, child, node->children, node->num_children)`
    // SAFETY: contiguous push_pop child run, valid for `node`'s borrow.
    for child in
        unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) }
    {
        if child.name() == sp::Material.as_ptr() {
            let mut fbx_id: u64 = 0;
            // C: `ufbx_string type_and_name, type, name;` — written below.
            // SAFETY: `ufbx_string` is a plain pointer/length pair, for which
            // the all-zero bit pattern is a valid (empty, null-data) value.
            let (mut type_and_name, mut type_, mut name): (String, String, String) = unsafe {
                (
                    core::mem::zeroed(),
                    core::mem::zeroed(),
                    core::mem::zeroed(),
                )
            };
            // SAFETY: fmt `'s'` pairs with the `*mut ufbx_string` out-pointer
            // `&mut type_and_name`, which is a live local.
            ufbxi_check!(
                uc,
                unsafe {
                    get_val1(
                        child,
                        b"s\0".as_ptr(),
                        &mut type_and_name as *mut String as *mut c_void,
                    )
                },
                "ufbxi_get_val1(child, \"s\", &type_and_name)"
            );
            // SAFETY: `type_and_name` was fully written by the `'s'` fetch above,
            // so it spans `length` readable bytes; the out-pointers are live
            // locals.
            unsafe { split_type_and_name(uc, type_and_name, &mut type_, &mut name) }?;
            // SAFETY: `&mut fbx_id` is a live local `uint64_t` slot and `name.data`
            // is the NUL-terminated interned name the split produced.
            unsafe { read_legacy_material(uc, child, &mut fbx_id, name.data) }?;
            // SAFETY: `info` is the caller's live `ufbxi_element_info`.
            connect_oo(uc, fbx_id, unsafe { (*info).fbx_id })?;
        } else if child.name() == sp::Link.as_ptr() {
            let mut fbx_id: u64 = 0;
            // C: `ufbx_string type_and_name, type, name;` — written below.
            // SAFETY: `ufbx_string` is a plain pointer/length pair, for which
            // the all-zero bit pattern is a valid (empty, null-data) value.
            let (mut type_and_name, mut type_, mut name): (String, String, String) = unsafe {
                (
                    core::mem::zeroed(),
                    core::mem::zeroed(),
                    core::mem::zeroed(),
                )
            };
            // SAFETY: fmt `'s'` pairs with the `*mut ufbx_string` out-pointer
            // `&mut type_and_name`, which is a live local.
            ufbxi_check!(
                uc,
                unsafe {
                    get_val1(
                        child,
                        b"s\0".as_ptr(),
                        &mut type_and_name as *mut String as *mut c_void,
                    )
                },
                "ufbxi_get_val1(child, \"s\", &type_and_name)"
            );
            // SAFETY: `type_and_name` was fully written by the `'s'` fetch above,
            // so it spans `length` readable bytes; the out-pointers are live
            // locals.
            unsafe { split_type_and_name(uc, type_and_name, &mut type_, &mut name) }?;
            // SAFETY: `&mut fbx_id` is a live local `uint64_t` slot and `name.data`
            // is the NUL-terminated interned name the split produced.
            unsafe { read_legacy_link(uc, child, &mut fbx_id, name.data) }?;

            let node_fbx_id: u64 = synthetic_id_from_string(uc, type_and_name.data);
            ufbxi_check!(uc, node_fbx_id != 0, "node_fbx_id");
            connect_oo(uc, node_fbx_id, fbx_id)?;
            if skin.is_null() {
                // SAFETY: `&mut skin_fbx_id` is a live local `uint64_t` slot,
                // `info` is the caller's live `ufbxi_element_info` whose
                // `name.data` is a NUL-terminated interned name, and
                // `SkinDeformer` is the element struct for
                // `ElementType::SkinDeformer`.
                skin = unsafe {
                    push_synthetic_element::<SkinDeformer>(
                        uc,
                        &mut skin_fbx_id,
                        None,
                        (*info).name.data,
                        ElementType::SkinDeformer,
                    )
                };
                ufbxi_check!(uc, !skin.is_null(), "skin");
                // SAFETY: `info` is the caller's live `ufbxi_element_info`.
                connect_oo(uc, skin_fbx_id, unsafe { (*info).fbx_id })?;
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
        // SAFETY: contiguous push_pop child run, valid for `videos`'s borrow.
        for child in unsafe {
            SliceViewIter::from_raw_parts(videos.children(), videos.num_children() as usize)
        } {
            // SAFETY: all-zero is a valid `ufbxi_element_info`; `child` is a
            // NodeView from `videos`'s own child run, `video_info` is an
            // unaliased local whose `name` is exactly the `ufbx_string` the `S`
            // format writes, and `node` is the parse-tree NodeView the DOM
            // lookup expects.
            unsafe {
                let mut video_info: ElementInfo = core::mem::zeroed();
                ufbxi_check!(
                    uc,
                    get_val1(
                        child,
                        b"S\0".as_ptr(),
                        &mut video_info.name as *mut String as *mut c_void
                    ),
                    "ufbxi_get_val1(child, \"S\", &video_info.name)"
                );
                video_info.fbx_id = push_synthetic_id(uc);
                video_info.dom_node = get_dom_node(uc, Some(node));

                read_video(uc, child, &mut video_info)?;
            }
        }
    }

    Ok(())
}

// ufbx.c:16350-16422 `ufbxi_read_legacy_model`
#[inline(never)]
pub(crate) fn read_legacy_model(uc: &Context, node: &NodeView) -> Result<(), Fail> {
    // C: `ufbx_string type_and_name, type, name;` — all three are written
    // before use by the two calls below.
    // SAFETY: all-zero is a valid `ufbx_string`; `node` is a parse-tree
    // NodeView and `type_and_name` is an unaliased local of exactly the type
    // the `s` format writes, so on success it is a pooled `data`/`length` pair
    // — which is what the split and the synthetic-id hash below read.
    let mut type_and_name: String = unsafe { core::mem::zeroed() };
    let mut type_: String = unsafe { core::mem::zeroed() };
    let mut name: String = unsafe { core::mem::zeroed() };
    ufbxi_check!(
        uc,
        unsafe {
            get_val1(
                node,
                b"s\0".as_ptr(),
                &mut type_and_name as *mut String as *mut c_void,
            )
        },
        "ufbxi_get_val1(node, \"s\", &type_and_name)"
    );
    unsafe { split_type_and_name(uc, type_and_name, &mut type_, &mut name)? };

    // SAFETY: all-zero is a valid `ufbxi_element_info`; `info` is an unaliased
    // local, `type_and_name.data` is the pooled string read above, `node` is a
    // parse-tree NodeView, and `elem_node` is the fresh element the push
    // returns — checked non-null before its `element_id` is copied into uc's
    // own `tmp_node_ids` buffer.
    let mut info: ElementInfo = unsafe { core::mem::zeroed() };
    unsafe {
        info.fbx_id = synthetic_id_from_string(uc, type_and_name.data);
        ufbxi_check!(uc, info.fbx_id != 0, "info.fbx_id");
        info.name = name;
        info.dom_node = get_dom_node(uc, Some(node));

        let elem_node: *mut UfbxNode = push_element::<UfbxNode>(uc, &mut info, ElementType::Node);
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
    // SAFETY: `node` is a parse-tree NodeView; `attrib_type` is an unaliased
    // local `char*` slot matching the `C` format, left at the static empty
    // string when the child is absent.
    ufbxi_ignore!(unsafe {
        find_val1(
            node,
            sp::Type.as_ptr(),
            b"C\0".as_ptr(),
            &mut attrib_type as *mut *const u8 as *mut c_void,
        )
    });

    // SAFETY (this dispatch): each arm hands the same parse-tree NodeView and
    // the local `&mut attrib_info` to the legacy attribute reader selected by
    // pointer-identity comparison of the pooled `attrib_type`.
    let mut has_attrib: bool = true;
    unsafe {
        if attrib_type == sp::Light.as_ptr() {
            read_legacy_light(uc, node, &mut attrib_info)?;
        } else if attrib_type == sp::Camera.as_ptr() {
            read_legacy_camera(uc, node, &mut attrib_info)?;
        } else if attrib_type == sp::LimbNode.as_ptr() {
            read_legacy_limb_node(uc, node, &mut attrib_info)?;
        } else if find_child(node, sp::Vertices.as_ptr()).is_some() {
            read_legacy_mesh(uc, node, &mut attrib_info)?;
        } else {
            has_attrib = false;
        }
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
    // SAFETY: contiguous push_pop child run, valid for `node`'s borrow.
    for child in
        unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) }
    {
        if child.name() == sp::Channel.as_ptr() {
            // C: `ufbx_string channel_name;` — written by the guard below.
            // SAFETY: all-zero is a valid `ufbx_string`; `child` is a NodeView
            // from `node`'s own child run and `channel_name` is an unaliased
            // local of exactly the type the `S` format writes, so on success it
            // is pooled and safe to hand to the channel reader.
            unsafe {
                let mut channel_name: String = core::mem::zeroed();
                if get_val1(
                    child,
                    b"S\0".as_ptr(),
                    &mut channel_name as *mut String as *mut c_void,
                ) {
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
        // SAFETY: the id out-param is uc's own `root_id` field, the name is the
        // static empty string, and `root` is the fresh element the push returns
        // — checked non-null before it is set up and before its `element_id` is
        // copied into uc's own `tmp_node_ids` buffer.
        unsafe {
            let root: *mut UfbxNode = push_synthetic_element::<UfbxNode>(
                uc,
                uc.root_id_mut_ptr(),
                None,
                EMPTY_CHAR.as_ptr(),
                ElementType::Node,
            );
            ufbxi_check!(uc, !root.is_null(), "root");
            setup_root_node(uc, root);
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
        // SAFETY: a NULL node retains uc's whole current top-level run.
        unsafe { retain_toplevel(uc, core::ptr::null_mut())? };
    }

    // Create the implicit animation stack if necessary
    if uc.legacy_implicit_anim_layer_id() != 0 {
        // C: `ufbxi_element_info layer_info = { 0 };`
        // SAFETY: all-zero is a valid `ufbxi_element_info`. The name is a
        // NUL-terminated literal, so `strlen` stays in bounds, and the intern
        // into uc's own pool makes it outlive the elements. Both pushes take
        // unaliased locals and their results are checked non-null; the
        // `stack_info` copy is a plain bitwise struct copy of `layer_info`,
        // which is fully initialized and holds no owning handles.
        unsafe {
            let mut layer_info: ElementInfo = core::mem::zeroed();
            layer_info.fbx_id = uc.legacy_implicit_anim_layer_id();
            layer_info.name.data = b"(internal)\0".as_ptr();
            layer_info.name.length = strlen(layer_info.name.data);
            push_string_place_str(uc.string_pool_mut_ptr(), &mut layer_info.name, true)?;
            let layer: *mut AnimLayer =
                push_element::<AnimLayer>(uc, &mut layer_info, ElementType::AnimLayer);
            ufbxi_check!(uc, !layer.is_null(), "layer");

            // C: `ufbxi_element_info stack_info = layer_info;` (struct copy)
            let mut stack_info: ElementInfo = core::ptr::read(&layer_info);
            stack_info.fbx_id = push_synthetic_id(uc);
            let stack: *mut AnimStack =
                push_element::<AnimStack>(uc, &mut stack_info, ElementType::AnimStack);
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
pub(crate) unsafe fn trim_delimiters(uc: &Context, data: *const u8, length: usize) -> usize {
    let mut length = length;
    while length > 0 {
        // C-parity: `char c = data[length - 1];` — `c` is only compared
        // against ASCII separators, so the signedness of C `char` is not
        // observable here (PORTING.md "char (value…)").
        // SAFETY: `data` .. `data + length` is the caller's live byte run and
        // the loop only reads at `length - 1` while `length > 0`, so the offset
        // stays inside it.
        let c: u8 = unsafe { *data.add(length - 1) };
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
    if uc.opts_view().filename_view().length() > 0 {
        uc.scene_view().metadata_view().set_filename(String::new_c(
            uc.opts_view().filename_view().data(),
            uc.opts_view().filename_view().length(),
        ));
    } else if uc.opts_view().raw_filename_view().size() > 0 {
        uc.scene_view()
            .metadata_view()
            .filename_view()
            .set_data(uc.opts_view().raw_filename_view().data());
        uc.scene_view()
            .metadata_view()
            .filename_view()
            .set_length(uc.opts_view().raw_filename_view().size());
    }

    if uc.opts_view().raw_filename_view().size() > 0 {
        uc.scene_view()
            .metadata_view()
            .set_raw_filename(Blob::new_c(
                uc.opts_view().raw_filename_view().data(),
                uc.opts_view().raw_filename_view().size(),
            ));
    } else if uc.opts_view().filename_view().length() > 0 {
        uc.scene_view()
            .metadata_view()
            .raw_filename_view()
            .set_data(uc.opts_view().filename_view().data());
        uc.scene_view()
            .metadata_view()
            .raw_filename_view()
            .set_size(uc.opts_view().filename_view().length());
    }

    // SAFETY: interning uc's own metadata `filename` / `raw_filename` slots,
    // reached through its element views, into uc's own string pool.
    unsafe {
        push_string_place_str(
            uc.string_pool_mut_ptr(),
            uc.scene_view().metadata_view().filename_mut_ptr(),
            false,
        )?;
        push_string_place_blob(
            uc.string_pool_mut_ptr(),
            uc.scene_view().metadata_view().raw_filename_mut_ptr(),
            true,
        )?;
    }

    uc.scene_view()
        .metadata_view()
        .relative_root_view()
        .set_data(uc.scene_view().metadata_view().filename_view().data());
    uc.scene_view()
        .metadata_view()
        .relative_root_view()
        // SAFETY: the scan reads uc's own metadata `filename`, interned just
        // above, over exactly its own `length` bytes.
        .set_length(unsafe {
            trim_delimiters(
                uc,
                uc.scene_view().metadata_view().filename_view().data(),
                uc.scene_view().metadata_view().filename_view().length(),
            )
        });

    uc.scene_view()
        .metadata_view()
        .raw_relative_root_view()
        .set_data(uc.scene_view().metadata_view().raw_filename_view().data());
    uc.scene_view()
        .metadata_view()
        .raw_relative_root_view()
        // SAFETY: the scan reads uc's own metadata `raw_filename`, interned
        // just above, over exactly its own `size` bytes.
        .set_size(unsafe {
            trim_delimiters(
                uc,
                uc.scene_view().metadata_view().raw_filename_view().data(),
                uc.scene_view().metadata_view().raw_filename_view().size(),
            )
        });

    // SAFETY: interning uc's own metadata `relative_root` / `raw_relative_root`
    // slots — prefixes of the filenames interned above — into uc's string pool.
    unsafe {
        push_string_place_str(
            uc.string_pool_mut_ptr(),
            uc.scene_view().metadata_view().relative_root_mut_ptr(),
            false,
        )?;
        push_string_place_blob(
            uc.string_pool_mut_ptr(),
            uc.scene_view().metadata_view().raw_relative_root_mut_ptr(),
            true,
        )?;
    }

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

// C: the union is written through one member and read through the other
// (`ufbxi_strblob_set` writes `.blob` and `ufbxi_strblob_data` may read
// `.str`), which is only sound because the two members are layout-identical.
const _: () = assert!(size_of::<Strblob>() == size_of::<String>());
const _: () = assert!(size_of::<Strblob>() == size_of::<Blob>());

// ufbx.c:16536-16544 `ufbxi_strblob_set`
#[inline(never)]
pub(crate) unsafe fn strblob_set(dst: *mut Strblob, data: *const u8, length: usize, raw: bool) {
    // SAFETY: `dst` is the caller's live `ufbxi_strblob`; the two members are
    // layout-identical pointer/length pairs (asserted above), so writing through
    // either is well-defined whichever member the caller reads.
    unsafe {
        if raw {
            (*dst).blob.data = data;
            (*dst).blob.size = length;
        } else {
            (*dst).str_.data = if length == 0 {
                EMPTY_CHAR.as_ptr()
            } else {
                data
            };
            (*dst).str_.length = length;
        }
    }
}

// ufbx.c:16546-16549 `ufbxi_strblob_data`
#[inline(always)]
pub(crate) unsafe fn strblob_data(strblob: *const Strblob, raw: bool) -> *const u8 {
    // SAFETY: `strblob` is the caller's live `ufbxi_strblob`; the two members
    // are layout-identical pointer/length pairs (asserted above), so reading
    // either is well-defined whichever member was written.
    unsafe {
        if raw {
            (*strblob).blob.data
        } else {
            (*strblob).str_.data
        }
    }
}

// ufbx.c:16551-16554 `ufbxi_strblob_length`
#[inline(always)]
pub(crate) unsafe fn strblob_length(strblob: *const Strblob, raw: bool) -> usize {
    // SAFETY: `strblob` is the caller's live `ufbxi_strblob`; the two members
    // are layout-identical pointer/length pairs (asserted above), so reading
    // either is well-defined whichever member was written.
    unsafe {
        if raw {
            (*strblob).blob.size
        } else {
            (*strblob).str_.length
        }
    }
}

// ufbx.c:16556-16565 `ufbxi_is_absolute_path`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn is_absolute_path(path: *const u8, length: usize) -> bool {
    // SAFETY: `path` .. `path + length` is the caller's live byte run, and the
    // `length` guards bound the index-0, index-1 and index-2 reads.
    unsafe {
        if length > 0 && (*path.add(0) == b'/' || *path.add(0) == b'\\') {
            return true;
        } else if length > 2
            && *path.add(1) == b':'
            && (*path.add(2) == b'\\' || *path.add(2) == b'/')
        {
            return true;
        }
    }
    false
}

// ufbx.c:16567-16650 `ufbxi_resolve_relative_filename`
#[inline(never)]
pub(crate) unsafe fn resolve_relative_filename(
    uc: &Context,
    p_dst: *mut Strblob,
    p_src: *const Strblob,
    raw: bool,
) -> Result<(), Fail> {
    // SAFETY: `p_src` is the caller's live `ufbxi_strblob` source, read through
    // the member selected by the same `raw` flag the caller threads everywhere.
    let (mut src, mut src_length): (*const u8, usize) =
        unsafe { (strblob_data(p_src, raw), strblob_length(p_src, raw)) };

    // Skip leading directory separators and early return if the relative path is empty
    // SAFETY: `src` .. `src + src_length` is the source path run described by
    // `p_src`, and `src_length > 0` bounds the index-0 reads.
    while unsafe { src_length > 0 && (*src.add(0) == b'/' || *src.add(0) == b'\\') } {
        // SAFETY: `src_length > 0`, so advancing one byte lands at most one past
        // the end of the source run.
        src = unsafe { src.add(1) };
        src_length -= 1;
    }
    if src_length == 0 {
        // SAFETY: `p_dst` is the caller's live `ufbxi_strblob` destination,
        // written through the member selected by `raw`.
        unsafe {
            strblob_set(p_dst, core::ptr::null(), 0, raw);
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

    // Retain absolute paths
    // SAFETY: `src` .. `src + src_length` is the remaining source path run.
    if unsafe { is_absolute_path(src, src_length) } {
        prefix_length = 0;
    }

    // Undo directories from `prefix` for every `..`
    while prefix_length > 0
        && src_length >= 3
        // SAFETY: `src` .. `src + src_length` is the source path run and
        // `src_length >= 3` bounds the index-0, index-1 and index-2 reads.
        && unsafe { *src.add(0) } == b'.'
        && unsafe { *src.add(1) } == b'.'
        && (unsafe { *src.add(2) } == b'/' || unsafe { *src.add(2) } == b'\\')
    {
        let mut part_start: usize = prefix_length;
        // SAFETY: `prefix_data` .. `prefix_data + prefix_length` is the scene
        // metadata relative-root run (`prefix_length` only ever shrinks below its
        // initial length), and `0 < part_start <= prefix_length` bounds the
        // `part_start - 1` read.
        while unsafe {
            part_start > 0
                && !(*prefix_data.add(part_start - 1) == b'/'
                    || *prefix_data.add(part_start - 1) == b'\\')
        } {
            part_start -= 1;
        }
        let part_len: usize = prefix_length - part_start;

        // SAFETY: as above — `part_start + part_len == prefix_length`, so with
        // `part_len == 2` both `part_start` and `part_start + 1` are inside the
        // relative-root run.
        if unsafe {
            part_len == 2
                && *prefix_data.add(part_start) == b'.'
                && *prefix_data.add(part_start + 1) == b'.'
        } {
            // Prefix itself ends in `..`, cannot cancel out a leading `../`
            break;
        }

        // Eat the leading '/' before the part segment
        prefix_length = if part_start > 0 { part_start - 1 } else { 0 };

        // SAFETY: `part_start + part_len` is the pre-update `prefix_length`, so
        // with `part_len == 1` the index `part_start` is inside the
        // relative-root run.
        if part_len == 1 && unsafe { *prefix_data.add(part_start) } == b'.' {
            // Single '.' -> remove and continue without cancelling out a leading `../`
            continue;
        }

        // SAFETY: `src_length >= 3` (loop condition), so advancing three bytes
        // lands at most one past the end of the source run.
        src = unsafe { src.add(3) };
        src_length -= 3;
    }

    let result_cap: usize = prefix_length + src_length + 1;
    let result: *mut u8 = uc.tmp_stack_view().push::<u8>(result_cap);
    ufbxi_check!(uc, !result.is_null(), "result");
    let mut ptr: *mut u8 = result;

    // Copy prefix and suffix converting separators in the process
    if prefix_length > 0 {
        // SAFETY: `prefix_data` has `prefix_length` readable bytes and `ptr` is
        // the head of the freshly pushed `result_cap`-byte scratch run, with
        // `result_cap == prefix_length + src_length + 1`; the scratch run is a
        // distinct allocation from the metadata relative root.
        unsafe {
            core::ptr::copy_nonoverlapping(prefix_data, ptr, prefix_length);
        }
        // SAFETY: `prefix_length < result_cap`, so the separator slot is inside
        // the scratch run.
        unsafe {
            *ptr.add(prefix_length) = uc.opts_view().path_separator();
        }
        // SAFETY: `prefix_length + 1 <= result_cap`, so the advance lands at
        // most one past the end of the scratch run.
        ptr = unsafe { ptr.add(prefix_length + 1) };
    }
    let mut i: usize = 0;
    while i < src_length {
        // SAFETY: `i < src_length` bounds the read in the source path run.
        let mut c: u8 = unsafe { *src.add(i) };
        if c == b'/' || c == b'\\' {
            c = uc.opts_view().path_separator();
        }
        // SAFETY: `ptr` has consumed the prefix (`prefix_length + 1` bytes, or
        // none) plus `i` bytes of the `prefix_length + src_length + 1`-byte
        // scratch run, and `i < src_length`, so one more byte fits.
        unsafe {
            *ptr = c;
            ptr = ptr.add(1);
        }
        i += 1;
    }

    // Intern the string and pop the temporary buffer
    // SAFETY: `ptr` and `result` are derived from the same scratch run, with
    // `ptr` at or after `result`.
    let mut dst: String = String::new_c(result, to_size(unsafe { ptr.offset_from(result) }));
    ufbx_assert!(dst.length <= result_cap);
    // SAFETY: the string pool is uc's own, and `dst` is a live local naming the
    // bytes written into the scratch run above.
    unsafe {
        push_string_place_str(uc.string_pool_mut_ptr(), &mut dst, raw)?;
    }
    // SAFETY: the temporary stack is uc's own and `result_cap` bytes were pushed
    // onto it above and are still its topmost allocation; a null destination
    // discards them.
    unsafe {
        pop::<u8>(uc.tmp_stack_mut_ptr(), result_cap, core::ptr::null_mut());
    }

    // SAFETY: `p_dst` is the caller's live `ufbxi_strblob` destination, written
    // through the member selected by `raw`; `dst` is the interned string, which
    // outlives the popped scratch run.
    unsafe {
        strblob_set(p_dst, dst.data, dst.length, raw);
    }

    Ok(())
}

// Open file utility

// ufbx.c:16654-16669 `ufbxi_open_file`
#[inline(never)]
pub(crate) unsafe fn open_file(
    cb: *const RawOpenFileCb,
    stream: *mut RawStream,
    path: *const u8,
    path_len: usize,
    original_filename: *const Blob,
    ator: *mut Allocator,
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
        (*info).context = ator as OpenFileContext;
    }
    if !original_filename.is_null() {
        // SAFETY: `original_filename` is checked non-null and is the caller's
        // live `ufbx_blob`; the destination is the local `info` storage as
        // above.
        unsafe {
            (*info).original_filename = *original_filename;
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
pub(crate) unsafe fn update_vertex_first_index(mesh: &View<Mesh>) {
    // C: `ufbxi_for_list(uint32_t, p_vx_ix, mesh->vertex_first_index)`
    // `vertex_first_index` is the mesh's `count`-long run.
    let mut p_vx_ix = mesh.vertex_first_index().data as *mut u32;
    let p_vx_ix_end = add_ptr(p_vx_ix, mesh.vertex_first_index().count);
    while p_vx_ix != p_vx_ix_end {
        // SAFETY: `p_vx_ix` is inside that run, short of `p_vx_ix_end`.
        unsafe {
            *p_vx_ix = NO_INDEX;
        }
        // SAFETY: `p_vx_ix` is before `p_vx_ix_end`, so the advance lands at
        // most one past the run's end.
        p_vx_ix = unsafe { p_vx_ix.add(1) };
    }

    let num_vertices: u32 = mesh.num_vertices() as u32;
    let mut ix: usize = 0;
    while ix < mesh.num_indices() {
        // SAFETY: `vertex_indices` is the mesh's `num_indices`-long index run
        // and `ix < num_indices` bounds the read.
        let vx: u32 = unsafe { *mesh.vertex_indices().data.add(ix) };
        if vx < num_vertices
            // SAFETY: every caller establishes `vertex_first_index.count ==
            // num_vertices` (set by `process_indices`, or by `finalize_mesh`'s
            // count==0 branch) before this runs, so `vx < num_vertices` bounds
            // the read.
            && unsafe { *(mesh.vertex_first_index().data as *mut u32).add(vx as usize) }
                == NO_INDEX
        {
            // SAFETY: as above — `vx < num_vertices` bounds the write in the
            // `vertex_first_index` run.
            unsafe {
                *(mesh.vertex_first_index().data as *mut u32).add(vx as usize) = ix as u32;
            }
        }
        ix += 1;
    }
}

// ufbx.c:16691-16765 `ufbxi_finalize_mesh`
#[inline(never)]
pub(crate) unsafe fn finalize_mesh(
    buf: &BufView,
    error: *mut Error,
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
        // SAFETY: `faces` is the mesh's own contiguous `count`-long face run,
        // live for this call.
        for face in unsafe {
            SliceViewIter::<Face>::from_raw_parts(
                mesh.faces().data as *mut Face,
                mesh.faces().count,
            )
        } {
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
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            !mesh.vertex_first_index().data.is_null(),
            "mesh->vertex_first_index.data"
        );
        // SAFETY: the mesh's `vertex_first_index` is the non-null
        // `num_vertices`-long run pushed just above — the count contract
        // `update_vertex_first_index` rests on.
        unsafe {
            update_vertex_first_index(mesh);
        }
    }

    if mesh.uv_sets().count == 0 && mesh.vertex_uv().exists() {
        let uv_set: *mut UvSet = buf.push_zero::<UvSet>(1);
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            !uv_set.is_null(),
            "uv_set"
        );

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
        ufbxi_check_err!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            !color_set.is_null(),
            "color_set"
        );

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

// CONTINUATION POINT (milestone 6i ends here): ufbx.c:16766 — the
// `// -- Pre-7000 "Take" based animation` banner section is fully ported.
// The `// -- .obj file` banner section (ufbx.c:16767) is owned by
// `native/obj.rs`.
//
// `// -- Reading the parsed data` section complete (ufbx.c:11762-16765),
// including `ufbxi_read_root` (ufbx.c:15844-15936) and
// `ufbxi_read_legacy_root` (ufbx.c:16424-16483) at their C-order slots above.
