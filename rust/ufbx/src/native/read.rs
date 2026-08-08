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
//! The rest of the section is NOT YET PORTED; see the continuation marker at
//! the end of this file.
#![allow(dead_code)]

use core::ffi::c_void;
use core::mem::size_of;

use crate::generated::{
    BlendChannel, BlendDeformer, BlendShape, ColorSet, Element, ElementType, Error, Exporter, Face,
    FaceGroup, IndexErrorHandling, InheritMode, InheritModeHandling, Mesh, MeshPart,
    Node as UfbxNode, Prop, PropFlags, PropType, Props, Thumbnail, ThumbnailFormat, Unknown, UvSet,
    Vec3, Vec4, VertexAttrib, VertexReal, VertexVec2, VertexVec3, VertexVec4, WarningType,
};
use crate::native::allocator::grow_array;
use crate::native::api::{
    find_int as api_find_int, find_prop as api_find_prop, find_prop_len, EMPTY_BLOB, EMPTY_STRING,
};
use crate::native::buf::{
    pop, push, push_copy, push_copy_fast, push_pop, push_size, push_zero, Buf,
};
use crate::native::error::{
    memcmp, strcmp, strlen, strncmp, ufbxi_check, ufbxi_check_err, ufbxi_check_return,
    ufbxi_fail_msg, ufbxi_fmt_err_info, Fail, EMPTY_CHAR,
};
use crate::native::hash::{hash64, hash_ptr_id, map_find, map_insert, PtrId};
use crate::native::parse::{
    array_type_size, find_array, find_child, find_child_strcmp, find_int, find_prop, find_val1,
    find_vec3, get_array, get_dom_node, get_name_key, get_name_key_c, get_prop_type, get_val1,
    get_val2, get_val_at, get_val_type, is_vec3_one, is_vec3_zero, parse_toplevel_child,
    push_element_extra, Context, ElementInfo, FbxAttrEntry, FbxIdEntry, Node, PtrFbxIdEntry,
    Template, TmpConnection, ValueArray, ValueType,
};
use crate::native::platform::{
    add_ptr, f64_to_i64, macro_lower_bound_eq, macro_stable_sort, max_sz, pack_version,
    stable_sort, to_size, ufbx_assert, ufbxi_dev_assert, ufbxi_ignore, ufbxi_maybe_null,
    ufbxi_unreachable, unstable_sort, FACE_GROUP_HASH_BITS, NO_INDEX,
};
use crate::native::string_pool as sp;
use crate::native::string_pool::push_string_place_str;
use crate::native::warnings::ufbxi_warnf;
use crate::prelude::{Blob, List, Real, Ref, String};

// ufbx.h:3618 `UFBX_ENUM_TYPE(ufbx_thumbnail_format, UFBX_THUMBNAIL_FORMAT, UFBX_THUMBNAIL_FORMAT_RGBA_32);`
// expanding to `enum { UFBX_THUMBNAIL_FORMAT_COUNT = UFBX_THUMBNAIL_FORMAT_RGBA_32 + 1 }`.
// Hand-duplicated from the generated enum's last variant so an upstream enum
// change tracks automatically through regen (precedent: `ELEMENT_TYPE_COUNT`
// in `native::parse`). `i32` because C compares it against an `int32_t`.
pub(crate) const THUMBNAIL_FORMAT_COUNT: i32 = ThumbnailFormat::Rgba32 as i32 + 1;

// ufbx.c:11764-11796 `ufbxi_read_embedded_blob`
#[inline(never)]
pub(crate) unsafe fn read_embedded_blob(
    uc: *mut Context,
    dst_blob: *mut Blob,
    node: *mut Node,
) -> Result<(), Fail> {
    if node.is_null() {
        return Ok(());
    }

    let content_arr: *mut ValueArray = get_array(node, b'C');
    if !content_arr.is_null() && (*content_arr).size > 0 {
        let content: String;
        let num_parts = (*content_arr).size;
        let parts: *mut String = (*content_arr).data as *mut String;

        if num_parts == 1 && !(*uc).from_ascii {
            content = *parts;
        } else {
            let mut total_size: usize = 0;
            // C: `ufbxi_for(ufbx_string, part, parts, num_parts)`
            let mut part = parts;
            let part_end = add_ptr(parts, num_parts);
            while part != part_end {
                total_size = total_size.wrapping_add((*part).length);
                part = part.add(1);
            }
            let dst_begin: *mut u8 = push::<u8>(&mut (*uc).result, total_size);
            ufbxi_check!(uc, !dst_begin.is_null(), "dst");
            content = String::new_c(dst_begin, total_size);
            let mut dst = dst_begin;
            let mut part = parts;
            while part != part_end {
                core::ptr::copy_nonoverlapping((*part).data, dst, (*part).length);
                dst = dst.add((*part).length);
                part = part.add(1);
            }
        }

        (*dst_blob).data = content.data;
        (*dst_blob).size = content.length;
    }

    Ok(())
}

// ufbx.c:11798-11869 `ufbxi_read_property`
#[inline(never)]
pub(crate) unsafe fn read_property(
    uc: *mut Context,
    node: *mut Node,
    prop: *mut Prop,
    version: i32,
) -> Result<(), Fail> {
    let mut type_str: *const u8 = core::ptr::null();
    let mut subtype_str: *const u8 = core::ptr::null();
    ufbxi_check!(
        uc,
        get_val2(
            node,
            b"SC\0".as_ptr(),
            &mut (*prop).name as *mut String as *mut c_void,
            &mut type_str as *mut *const u8 as *mut c_void,
        ),
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
            get_val_at(
                node,
                ix as usize,
                b'C',
                &mut subtype_str as *mut *const u8 as *mut c_void,
            ),
            "ufbxi_get_val_at(node, val_ix++, 'C', (char**)&subtype_str)"
        );
    }

    let mut flags: u32 = 0;
    (*prop)._internal_key = get_name_key((*prop).name.data, (*prop).name.length);

    // C leaves `flags_str` uninitialized; it is only read when the `'S'` fetch
    // below succeeds, which fully writes it.
    let mut flags_str: String = String::new_c(core::ptr::null(), 0);
    let ix = val_ix;
    val_ix = val_ix.wrapping_add(1);
    if get_val_at(
        node,
        ix as usize,
        b'S',
        &mut flags_str as *mut String as *mut c_void,
    ) {
        let mut i: usize = 0;
        while i < flags_str.length {
            // C-parity: `char next` reads a `const char *` — signed bytes on
            // the oracle targets (PORTING.md `char` value row).
            let next: i8 = if i + 1 < flags_str.length {
                *(flags_str.data.add(i + 1) as *const i8)
            } else {
                b'0' as i8
            };
            match *flags_str.data.add(i) {
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

    (*prop).type_ = get_prop_type(uc, type_str);
    if (*prop).type_ == PropType::Unknown && !subtype_str.is_null() {
        (*prop).type_ = get_prop_type(uc, subtype_str);
    }

    if get_val_at(
        node,
        val_ix as usize,
        b'L',
        &mut (*prop).value_int as *mut i64 as *mut c_void,
    ) {
        flags |= PropFlags::VALUE_INT.raw();
    }

    // C-parity: `prop->value_real_arr[]` is the `ufbx_prop` value union's
    // 4-real view (ufbx.h); the generated struct keeps only the `value_vec4`
    // member, so the array view is reached by pointer cast.
    let value_real_arr: *mut Real = &mut (*prop).value_vec4 as *mut _ as *mut Real;
    let mut real_ix: usize = 0;
    while real_ix < 4 {
        if !get_val_at(
            node,
            (val_ix as usize).wrapping_add(real_ix),
            b'R',
            value_real_arr.add(real_ix) as *mut c_void,
        ) {
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

    if get_val_at(
        node,
        val_ix as usize,
        b'S',
        &mut (*prop).value_str as *mut String as *mut c_void,
    ) {
        if (*prop).value_str.length > 0 {
            ufbxi_ignore!(get_val_at(
                node,
                val_ix as usize,
                b'b',
                &mut (*prop).value_blob as *mut Blob as *mut c_void,
            ));
        }
        flags |= PropFlags::VALUE_STR.raw();
    } else {
        (*prop).value_str = EMPTY_STRING.0;
    }

    // Very unlikely, seems to only exist in some "non standard" FBX files
    if (*node).num_children > 0 {
        let binary: *mut Node = find_child(node, sp::BinaryData.as_ptr());
        read_embedded_blob(uc, &mut (*prop).value_blob, binary)?;
        flags |= PropFlags::VALUE_BLOB.raw();
    }

    (*prop).flags = PropFlags::from_raw(flags);

    Ok(())
}

// ufbx.c:11871-11876 `ufbxi_prop_less`
#[inline(always)]
pub(crate) unsafe fn prop_less(a: *mut Prop, b: *mut Prop) -> bool {
    if (*a)._internal_key < (*b)._internal_key {
        return true;
    }
    if (*a)._internal_key > (*b)._internal_key {
        return false;
    }
    strcmp((*a).name.data, (*b).name.data) < 0
}

// ufbx.c:11878-11883 `ufbxi_sort_properties`
#[inline(never)]
pub(crate) unsafe fn sort_properties(
    uc: *mut Context,
    props: *mut Prop,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            count.wrapping_mul(size_of::<Prop>()),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, count * sizeof(ufbx_prop))"
    );
    macro_stable_sort::<Prop>(32, props, (*uc).tmp_arr as *mut Prop, count, |a, b| {
        prop_less(a as *mut Prop, b as *mut Prop)
    });
    Ok(())
}

// ufbx.c:11885-11901 `ufbxi_deduplicate_properties`
#[inline(never)]
pub(crate) unsafe fn deduplicate_properties(list: *mut List<Prop>) {
    if (*list).count >= 2 {
        let ps: *mut Prop = (*list).data as *mut Prop;
        let mut dst: usize = 0;
        let mut src: usize = 0;
        let end: usize = (*list).count;
        while src < end {
            if src + 1 < end && (*ps.add(src)).name.data == (*ps.add(src + 1)).name.data {
                src += 1;
            } else if dst != src {
                *ps.add(dst) = *ps.add(src);
                dst += 1;
                src += 1;
            } else {
                dst += 1;
                src += 1;
            }
        }
        (*list).count = dst;
    }
}

// ufbx.c:11903-11932 `ufbxi_read_properties`
#[inline(never)]
pub(crate) unsafe fn read_properties(
    uc: *mut Context,
    parent: *mut Node,
    props: *mut Props,
) -> Result<(), Fail> {
    (*props).defaults = None;

    let mut version: i32 = 70;
    let mut node: *mut Node = find_child(parent, sp::Properties70.as_ptr());
    if node.is_null() {
        node = find_child(parent, sp::Properties60.as_ptr());
        if node.is_null() {
            // No properties found, not an error
            (*props).props.data = core::ptr::null();
            (*props).props.count = 0;
            return Ok(());
        }
        version = 60;
    }

    (*props).props.data = push_zero::<Prop>(&mut (*uc).result, (*node).num_children as usize);
    (*props).props.count = (*node).num_children as usize;
    ufbxi_check!(uc, !(*props).props.data.is_null(), "props->props.data");

    let mut i: usize = 0;
    while i < (*props).props.count {
        read_property(
            uc,
            (*node).children.add(i),
            (*props).props.data.add(i) as *mut Prop,
            version,
        )?;
        i += 1;
    }

    sort_properties(uc, (*props).props.data as *mut Prop, (*props).props.count)?;
    deduplicate_properties(&mut (*props).props);

    Ok(())
}

// ufbx.c:11934-11967 `ufbxi_read_thumbnail`
#[inline(never)]
pub(crate) unsafe fn read_thumbnail(
    uc: *mut Context,
    node: *mut Node,
    thumbnail: *mut Thumbnail,
) -> Result<(), Fail> {
    read_properties(uc, node, &mut (*thumbnail).props)?;

    let custom_width: i64 = api_find_int(&(*thumbnail).props, b"CustomWidth\0".as_ptr(), 0);
    let custom_height: i64 = api_find_int(&(*thumbnail).props, b"CustomHeight\0".as_ptr(), 0);

    let mut format: i32 = 0;
    let format_node: *mut Node = find_child_strcmp(node, b"Format\0".as_ptr());
    if !format_node.is_null()
        && get_val1(
            format_node,
            b"I\0".as_ptr(),
            &mut format as *mut i32 as *mut c_void,
        )
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
            (*thumbnail).format = thumbnail_format_from_raw(format + 1);
        }
    }

    let mut size: i32 = 0;
    if find_val1(
        node,
        sp::Size.as_ptr(),
        b"I\0".as_ptr(),
        &mut size as *mut i32 as *mut c_void,
    ) {
        if size > 0 {
            (*thumbnail).width = size as u32;
            (*thumbnail).height = size as u32;
        } else if size < 0 && custom_width > 0 && custom_height > 0 {
            (*thumbnail).width = custom_width as u32;
            (*thumbnail).height = custom_height as u32;
        }
    }

    let data_arr: *mut ValueArray = find_array(node, sp::ImageData.as_ptr(), b'c');
    if !data_arr.is_null() {
        (*thumbnail).data.data = (*data_arr).data as *const u8;
        (*thumbnail).data.size = (*data_arr).size;
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
pub(crate) unsafe fn read_scene_info(uc: *mut Context, node: *mut Node) -> Result<(), Fail> {
    read_properties(uc, node, &mut (*uc).scene.metadata.scene_props)?;

    let thumbnail: *mut Node = find_child(node, sp::Thumbnail.as_ptr());
    if !thumbnail.is_null() {
        read_thumbnail(uc, thumbnail, &mut (*uc).scene.metadata.thumbnail)?;
    }

    Ok(())
}

// ufbx.c:11981-12033 `ufbxi_read_header_extension`
#[inline(never)]
pub(crate) unsafe fn read_header_extension(uc: *mut Context) -> Result<(), Fail> {
    let mut has_tc_definition = false;
    let mut tc_definition: i32 = 0;
    let mut header_version: i32 = 0;

    loop {
        let mut child: *mut Node = core::ptr::null_mut();
        parse_toplevel_child(uc, &mut child, core::ptr::null_mut())?;
        if child.is_null() {
            break;
        }

        if (*child).name == sp::Creator.as_ptr() {
            ufbxi_ignore!(get_val1(
                child,
                b"S\0".as_ptr(),
                &mut (*uc).scene.metadata.creator as *mut String as *mut c_void,
            ));
        }

        if (*uc).version < 6000 && (*child).name == sp::FBXVersion.as_ptr() {
            let mut version: i32 = 0;
            if get_val1(
                child,
                b"I\0".as_ptr(),
                &mut version as *mut i32 as *mut c_void,
            ) {
                if version > 0 && version < 6000 && (version as u32) > (*uc).version {
                    (*uc).version = version as u32;
                }
            }
        }

        if (*child).name == sp::FBXHeaderVersion.as_ptr() {
            ufbxi_ignore!(get_val1(
                child,
                b"I\0".as_ptr(),
                &mut header_version as *mut i32 as *mut c_void,
            ));
        }

        if (*child).name == sp::OtherFlags.as_ptr() {
            if find_val1(
                child,
                sp::TCDefinition.as_ptr(),
                b"I\0".as_ptr(),
                &mut tc_definition as *mut i32 as *mut c_void,
            ) {
                has_tc_definition = true;
            }
        }

        if (*child).name == sp::SceneInfo.as_ptr() {
            read_scene_info(uc, child)?;
        }
    }

    // FBX 8000 will change the KTime units and the new units are opt-in currently via `TCDefinition`.
    // `TCDefinition` seems be accounted in all versions, as long as `FBXHeaderVersion >= 1004`.
    // The old KTime units are specified as the value `127` and all other values seem to use the new definition.
    let mut use_v7_ktime = (*uc).version < 8000;
    if header_version >= 1004 && has_tc_definition {
        use_v7_ktime = tc_definition == 127;
    }

    (*uc).ktime_sec = if use_v7_ktime { 46186158000 } else { 141120000 };
    (*uc).ktime_sec_double = (*uc).ktime_sec as f64;

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
    while *fmt != 0 {
        // C-parity: `char c = *fmt++;` / `char s = str.data[pos];` are `const
        // char *` dereferences — signed bytes on the oracle targets
        // (PORTING.md `char` value row).
        let c: i8 = *(fmt as *const i8);
        fmt = fmt.add(1);
        if c >= b'a' as i8 && c <= b'z' as i8 {
            if pos >= str_.length {
                return false;
            }
            let s: i8 = *(str_.data.add(pos) as *const i8);
            if s != c && s as i32 + (b'a' as i32 - b'A' as i32) != c as i32 {
                return false;
            }
            pos += 1;
        } else if c == b' ' as i8 {
            while pos < str_.length {
                let s: i8 = *(str_.data.add(pos) as *const i8);
                if s != b' ' as i8 && s != b'\t' as i8 {
                    break;
                }
                pos += 1;
            }
        } else if c == b'-' as i8 {
            while pos < str_.length {
                let s: i8 = *(str_.data.add(pos) as *const i8);
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
            if *(str_.data.add(pos) as *const i8) != c {
                return false;
            }
            pos += 1;
        } else if c == b'?' as i8 {
            let mut num: u32 = 0;
            let mut len: usize = 0;
            while pos < str_.length {
                let s: i8 = *(str_.data.add(pos) as *const i8);
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
            *p_version.add(num_ix) = num;
            num_ix += 1;
        } else {
            ufbxi_unreachable!("Unhandled match character");
        }
    }

    true
}

// ufbx.c:12084-12128 `ufbxi_match_exporter`
#[inline(never)]
pub(crate) unsafe fn match_exporter(uc: *mut Context) -> Result<(), Fail> {
    let creator: String = (*uc).scene.metadata.creator;
    let mut version: [u32; 3] = [0; 3];
    if match_version_string(b"blender-- ?.?.?\0".as_ptr(), creator, version.as_mut_ptr()) {
        (*uc).exporter = Exporter::BlenderBinary;
        (*uc).exporter_version = pack_version(version[0], version[1], version[2]);
    } else if match_version_string(b"blender- ?.?\0".as_ptr(), creator, version.as_mut_ptr()) {
        (*uc).exporter = Exporter::BlenderBinary;
        (*uc).exporter_version = pack_version(version[0], version[1], 0);
    } else if match_version_string(
        b"blender version ?.?\0".as_ptr(),
        creator,
        version.as_mut_ptr(),
    ) {
        (*uc).exporter = Exporter::BlenderAscii;
        (*uc).exporter_version = pack_version(version[0], version[1], 0);
    } else if match_version_string(
        b"fbx sdk/fbx plugins version ?.?\0".as_ptr(),
        creator,
        version.as_mut_ptr(),
    ) {
        (*uc).exporter = Exporter::FbxSdk;
        (*uc).exporter_version = pack_version(version[0], version[1], 0);
    } else if match_version_string(
        b"fbx sdk/fbx plugins build ?\0".as_ptr(),
        creator,
        version.as_mut_ptr(),
    ) {
        (*uc).exporter = Exporter::FbxSdk;
        (*uc).exporter_version = pack_version(
            version[0] / 10000u32,
            version[0] / 100u32 % 100u32,
            version[0] % 100u32,
        );
    } else if match_version_string(
        b"motionbuilder version ?.?\0".as_ptr(),
        creator,
        version.as_mut_ptr(),
    ) {
        (*uc).exporter = Exporter::MotionBuilder;
        (*uc).exporter_version = pack_version(version[0], version[1], 0);
    } else if match_version_string(
        b"motionbuilder/mocap/online version ?.?\0".as_ptr(),
        creator,
        version.as_mut_ptr(),
    ) {
        (*uc).exporter = Exporter::MotionBuilder;
        (*uc).exporter_version = pack_version(version[0], version[1], 0);
    } else if match_version_string(b"ufbx_write\0".as_ptr(), creator, version.as_mut_ptr()) {
        (*uc).exporter = Exporter::UfbxWrite;
        (*uc).exporter_version = pack_version(0, 0, 1);
    }

    (*uc).scene.metadata.exporter = (*uc).exporter;
    (*uc).scene.metadata.exporter_version = (*uc).exporter_version;

    // Un-detect the exporter in `ufbxi_context` to disable special cases
    if (*uc).opts.disable_quirks {
        (*uc).exporter = Exporter::Unknown;
        (*uc).exporter_version = 0;
    }

    if (*uc).exporter == Exporter::BlenderBinary {
        (*uc).blender_full_weights = true;
    }

    Ok(())
}

// ufbx.c:12130-12149 `ufbxi_read_document`
#[inline(never)]
pub(crate) unsafe fn read_document(uc: *mut Context) -> Result<(), Fail> {
    let mut found_root_id = false;

    loop {
        let mut child: *mut Node = core::ptr::null_mut();
        parse_toplevel_child(uc, &mut child, core::ptr::null_mut())?;
        if child.is_null() {
            break;
        }

        if (*child).name == sp::Document.as_ptr() && !found_root_id {
            // Post-7000: Try to find the first document node and root ID.
            // TODO: Multiple documents / roots?
            if find_val1(
                child,
                sp::RootNode.as_ptr(),
                b"L\0".as_ptr(),
                &mut (*uc).root_id as *mut u64 as *mut c_void,
            ) {
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
pub(crate) unsafe fn read_definitions(uc: *mut Context) -> Result<(), Fail> {
    loop {
        let mut object: *mut Node = core::ptr::null_mut();
        parse_toplevel_child(uc, &mut object, core::ptr::null_mut())?;
        if object.is_null() {
            break;
        }

        if (*object).name != sp::ObjectType.as_ptr() {
            continue;
        }

        let tmpl: *mut Template = push_zero::<Template>(&mut (*uc).tmp_stack, 1);
        (*uc).num_templates = (*uc).num_templates.wrapping_add(1);
        ufbxi_check!(uc, !tmpl.is_null(), "tmpl");
        ufbxi_check!(
            uc,
            get_val1(
                object,
                b"C\0".as_ptr(),
                &mut (*tmpl).type_ as *mut *const u8 as *mut c_void,
            ),
            "ufbxi_get_val1(object, \"C\", (char**)&tmpl->type)"
        );

        // Pre-7000 FBX versions don't have property templates, they just have
        // the object counts by themselves.
        let props: *mut Node = find_child(object, sp::PropertyTemplate.as_ptr());
        if !props.is_null() {
            ufbxi_check!(
                uc,
                get_val1(
                    props,
                    b"S\0".as_ptr(),
                    &mut (*tmpl).sub_type as *mut String as *mut c_void,
                ),
                "ufbxi_get_val1(props, \"S\", &tmpl->sub_type)"
            );

            // Remove the "Fbx" prefix from sub-types, remember to re-intern!
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

                push_string_place_str(&mut (*uc).string_pool, &mut (*tmpl).sub_type, false)?;
            }

            read_properties(uc, props, &mut (*tmpl).props)?;
        }
    }

    // TODO: Preserve only the `props` part of the templates
    (*uc).templates =
        push_pop::<Template>(&mut (*uc).result, &mut (*uc).tmp_stack, (*uc).num_templates);
    ufbxi_check!(uc, !(*uc).templates.is_null(), "uc->templates");

    Ok(())
}

// ufbx.c:12195-12218 `ufbxi_find_template`
#[must_use]
pub(crate) unsafe fn find_template(
    uc: *mut Context,
    name: *const u8,
    sub_type: *const u8,
) -> *mut Props {
    // TODO: Binary search
    // C: `ufbxi_for(ufbxi_template, tmpl, uc->templates, uc->num_templates)`
    let mut tmpl = (*uc).templates;
    let tmpl_end = add_ptr(tmpl, (*uc).num_templates);
    while tmpl != tmpl_end {
        if (*tmpl).type_ == name {
            // Check that sub_type matches unless the type is Material, Model, AnimationStack, AnimationLayer.
            // Those match to all sub-types.
            if (*tmpl).type_ != sp::Material.as_ptr()
                && (*tmpl).type_ != sp::Model.as_ptr()
                && (*tmpl).type_ != sp::AnimationStack.as_ptr()
                && (*tmpl).type_ != sp::AnimationLayer.as_ptr()
            {
                if (*tmpl).sub_type.data != sub_type {
                    return core::ptr::null_mut();
                }
            }

            if (*tmpl).props.props.count > 0 {
                return &mut (*tmpl).props;
            } else {
                return core::ptr::null_mut();
            }
        }
        tmpl = tmpl.add(1);
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
pub(crate) unsafe fn push_synthetic_id(uc: *mut Context) -> u64 {
    // C: `return ++uc->synthetic_id_counter;` — pre-increment yields the NEW value.
    (*uc).synthetic_id_counter = (*uc).synthetic_id_counter.wrapping_add(1);
    (*uc).synthetic_id_counter
}

// ufbx.c:12234-12248 `ufbxi_synthetic_id_from_ptr_id`
#[inline(never)]
pub(crate) unsafe fn synthetic_id_from_ptr_id(uc: *mut Context, ptr: usize, id: u64) -> u64 {
    let ptr_id = PtrId { ptr, id };
    let hash = hash_ptr_id(ptr_id);
    let mut entry: *mut PtrFbxIdEntry = map_find(
        &mut (*uc).ptr_fbx_id_map,
        hash,
        &ptr_id as *const PtrId as *const c_void,
    );

    if entry.is_null() {
        entry = map_insert(
            &mut (*uc).ptr_fbx_id_map,
            hash,
            &ptr_id as *const PtrId as *const c_void,
        );
        ufbxi_check_return!(uc, !entry.is_null(), 0, "entry");
        (*entry).ptr_id = ptr_id;
        (*entry).fbx_id = push_synthetic_id(uc);
    }

    (*entry).fbx_id
}

// ufbx.c:12250-12258 `ufbxi_synthetic_id_from_string`
#[inline(always)]
pub(crate) unsafe fn synthetic_id_from_string(uc: *mut Context, str_: *const u8) -> u64 {
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
pub(crate) unsafe fn validate_fbx_id(uc: *mut Context, p_fbx_id: *mut u64) -> Result<(), Fail> {
    let mut fbx_id: u64 = *p_fbx_id;
    if fbx_id >= POINTER_ID_START {
        fbx_id = synthetic_id_from_ptr_id(uc, 0, fbx_id);
        ufbxi_check!(uc, fbx_id != 0, "fbx_id");
        *p_fbx_id = fbx_id;
    }
    Ok(())
}

// ufbx.c:12271-12305 `ufbxi_split_type_and_name`
#[inline(never)]
pub(crate) unsafe fn split_type_and_name(
    uc: *mut Context,
    type_and_name: String,
    type_: *mut String,
    name: *mut String,
) -> Result<(), Fail> {
    // Name and type are packed in a single property as Type::Name (in ASCII)
    // or Name\x00\x01Type (in binary)
    let sep: *const u8 = if (*uc).from_ascii {
        b"::\0".as_ptr()
    } else {
        b"\x00\x01\0".as_ptr()
    };
    let mut type_end: usize = 2;
    while type_end <= type_and_name.length {
        let ch: *const u8 = type_and_name.data.add(type_end - 2);
        if *ch.add(0) == *sep.add(0) && *ch.add(1) == *sep.add(1) {
            break;
        }
        type_end += 1;
    }

    // ???: ASCII and binary store type and name in different order
    if type_end <= type_and_name.length {
        if (*uc).from_ascii {
            (*name).data = type_and_name.data.add(type_end);
            (*name).length = type_and_name.length - type_end;
            (*type_).data = type_and_name.data;
            (*type_).length = type_end - 2;
        } else {
            (*name).data = type_and_name.data;
            (*name).length = type_end - 2;
            (*type_).data = type_and_name.data.add(type_end);
            (*type_).length = type_and_name.length - type_end;
        }
    } else {
        *name = type_and_name;
        (*type_).data = EMPTY_CHAR.as_ptr();
        (*type_).length = 0;
    }

    push_string_place_str(&mut (*uc).string_pool, type_, false)?;
    push_string_place_str(&mut (*uc).string_pool, name, false)?;

    Ok(())
}

// ufbx.c:12307-12323 `ufbxi_insert_fbx_id`
#[inline(never)]
pub(crate) unsafe fn insert_fbx_id(
    uc: *mut Context,
    fbx_id: u64,
    element_id: u32,
) -> Result<(), Fail> {
    let hash = hash64(fbx_id);
    let mut entry: *mut FbxIdEntry = map_find(
        &mut (*uc).fbx_id_map,
        hash,
        &fbx_id as *const u64 as *const c_void,
    );

    if entry.is_null() {
        entry = map_insert(
            &mut (*uc).fbx_id_map,
            hash,
            &fbx_id as *const u64 as *const c_void,
        );
        ufbxi_check!(uc, !entry.is_null(), "entry");
        (*entry).fbx_id = fbx_id;
        (*entry).element_id = element_id;
        (*entry).user_id = 0;
    } else {
        ufbxi_check!(
            uc,
            ufbxi_warnf!(uc, WarningType::DuplicateObjectId, "Duplicate object ID").is_ok(),
            "ufbxi_warnf(UFBX_WARNING_DUPLICATE_OBJECT_ID, \"Duplicate object ID\")"
        );
    }

    Ok(())
}

// ufbx.c:12325-12329 `ufbxi_find_fbx_id`
#[inline(never)]
pub(crate) unsafe fn find_fbx_id(uc: *mut Context, fbx_id: u64) -> *mut FbxIdEntry {
    let hash = hash64(fbx_id);
    map_find(
        &mut (*uc).fbx_id_map,
        hash,
        &fbx_id as *const u64 as *const c_void,
    )
}

// ufbx.c:12331-12334 `ufbxi_fbx_id_exists`
#[inline(always)]
pub(crate) unsafe fn fbx_id_exists(uc: *mut Context, fbx_id: u64) -> bool {
    !find_fbx_id(uc, fbx_id).is_null()
}

// ufbx.c:12336-12350 `ufbxi_insert_fbx_attr`
#[inline(never)]
pub(crate) unsafe fn insert_fbx_attr(
    uc: *mut Context,
    fbx_id: u64,
    attrib_fbx_id: u64,
) -> Result<(), Fail> {
    let hash = hash64(fbx_id);
    let mut entry: *mut FbxAttrEntry = map_find(
        &mut (*uc).fbx_attr_map,
        hash,
        &fbx_id as *const u64 as *const c_void,
    );
    // TODO: Strict / warn about duplicate objects

    if entry.is_null() {
        entry = map_insert(
            &mut (*uc).fbx_attr_map,
            hash,
            &fbx_id as *const u64 as *const c_void,
        );
        ufbxi_check!(uc, !entry.is_null(), "entry");
        (*entry).node_fbx_id = fbx_id;
        (*entry).attr_fbx_id = attrib_fbx_id;
    }

    Ok(())
}

// C assigns a possibly-null pointer into a field the generated bindings type as
// `Option<Ref<T>>`; `Ref<T>` is `#[repr(transparent)]` over `NonNull<T>`, so the
// option is niche-packed to a bare pointer and NULL is `None`.
#[inline(always)]
unsafe fn opt_ref<T>(ptr: *mut T) -> Option<Ref<T>> {
    if ptr.is_null() {
        None
    } else {
        Some(Ref::from_ptr(ptr))
    }
}

// ufbx.c:12352-12382 `ufbxi_push_element_size`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_element_size(
    uc: *mut Context,
    info: *mut ElementInfo,
    size: usize,
    type_: ElementType,
) -> *mut Element {
    // C-parity: `~0x7u` in `(size + 7u) & ~0x7u` is an `unsigned int`
    // (0xFFFFFFF8) zero-extended to `size_t`, so the mask also clears the upper
    // 32 bits. Unreachable in practice (`size` is always a `sizeof`), ported
    // verbatim anyway.
    let aligned_size: usize = size.wrapping_add(7) & (!0x7u32 as usize);

    let typed_id: u32 = (*uc).tmp_typed_element_offsets[type_ as usize].num_items as u32;
    // C: `uint32_t element_id = uc->num_elements++;` — post-increment yields
    // the OLD value.
    let element_id: u32 = (*uc).num_elements;
    (*uc).num_elements = (*uc).num_elements.wrapping_add(1);

    ufbxi_check_return!(
        uc,
        !push_copy_fast::<usize>(
            &mut (*uc).tmp_typed_element_offsets[type_ as usize],
            1,
            &(*uc).tmp_element_byte_offset
        )
        .is_null(),
        core::ptr::null_mut(),
        "ufbxi_push_copy_fast(&uc->tmp_typed_element_offsets[type], size_t, 1, &uc->tmp_element_byte_offset)"
    );
    ufbxi_check_return!(
        uc,
        !push_copy_fast::<usize>(
            &mut (*uc).tmp_element_offsets,
            1,
            &(*uc).tmp_element_byte_offset
        )
        .is_null(),
        core::ptr::null_mut(),
        "ufbxi_push_copy_fast(&uc->tmp_element_offsets, size_t, 1, &uc->tmp_element_byte_offset)"
    );
    ufbxi_check_return!(
        uc,
        !push_copy_fast::<u64>(&mut (*uc).tmp_element_fbx_ids, 1, &(*info).fbx_id).is_null(),
        core::ptr::null_mut(),
        "ufbxi_push_copy_fast(&uc->tmp_element_fbx_ids, uint64_t, 1, &info->fbx_id)"
    );
    (*uc).tmp_element_byte_offset = (*uc).tmp_element_byte_offset.wrapping_add(aligned_size);

    let elem: *mut Element =
        push_zero::<u64>(&mut (*uc).tmp_elements, aligned_size / 8) as *mut Element;
    ufbxi_check_return!(uc, !elem.is_null(), core::ptr::null_mut(), "elem");
    (*elem).type_ = type_;
    (*elem).element_id = element_id;
    (*elem).typed_id = typed_id;
    (*elem).name = (*info).name;
    // C: `elem->props = info->props;` is a struct memcpy; `Props` is not `Copy`
    // (it holds an `Option<Ref<Props>>`) but has no drop glue.
    core::ptr::write(&mut (*elem).props, core::ptr::read(&(*info).props));
    (*elem).dom_node = opt_ref((*info).dom_node);

    if !(*uc).p_element_id.is_null() {
        *(*uc).p_element_id = element_id;
    }

    ufbxi_check_return!(
        uc,
        !push_copy_fast::<*mut Element>(&mut (*uc).tmp_element_ptrs, 1, &elem).is_null(),
        core::ptr::null_mut(),
        "ufbxi_push_copy_fast(&uc->tmp_element_ptrs, ufbx_element*, 1, &elem)"
    );

    ufbxi_check_return!(
        uc,
        insert_fbx_id(uc, (*info).fbx_id, element_id).is_ok(),
        core::ptr::null_mut(),
        "ufbxi_insert_fbx_id(uc, info->fbx_id, element_id)"
    );

    elem
}

// ufbx.c:12384-12414 `ufbxi_push_synthetic_element_size`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_synthetic_element_size(
    uc: *mut Context,
    p_fbx_id: *mut u64,
    node: *mut Node,
    name: *const u8,
    size: usize,
    type_: ElementType,
) -> *mut Element {
    // C-parity: `~0x7u` in `(size + 7u) & ~0x7u` is an `unsigned int`
    // (0xFFFFFFF8) zero-extended to `size_t`, so the mask also clears the upper
    // 32 bits. Unreachable in practice (`size` is always a `sizeof`), ported
    // verbatim anyway.
    let aligned_size: usize = size.wrapping_add(7) & (!0x7u32 as usize);

    let typed_id: u32 = (*uc).tmp_typed_element_offsets[type_ as usize].num_items as u32;
    let element_id: u32 = (*uc).num_elements;
    (*uc).num_elements = (*uc).num_elements.wrapping_add(1);

    ufbxi_check_return!(
        uc,
        !push_copy_fast::<usize>(
            &mut (*uc).tmp_typed_element_offsets[type_ as usize],
            1,
            &(*uc).tmp_element_byte_offset
        )
        .is_null(),
        core::ptr::null_mut(),
        "ufbxi_push_copy_fast(&uc->tmp_typed_element_offsets[type], size_t, 1, &uc->tmp_element_byte_offset)"
    );
    ufbxi_check_return!(
        uc,
        !push_copy_fast::<usize>(
            &mut (*uc).tmp_element_offsets,
            1,
            &(*uc).tmp_element_byte_offset
        )
        .is_null(),
        core::ptr::null_mut(),
        "ufbxi_push_copy_fast(&uc->tmp_element_offsets, size_t, 1, &uc->tmp_element_byte_offset)"
    );
    (*uc).tmp_element_byte_offset = (*uc).tmp_element_byte_offset.wrapping_add(aligned_size);

    let elem: *mut Element =
        push_zero::<u64>(&mut (*uc).tmp_elements, aligned_size / 8) as *mut Element;
    ufbxi_check_return!(uc, !elem.is_null(), core::ptr::null_mut(), "elem");
    (*elem).type_ = type_;
    (*elem).element_id = element_id;
    (*elem).typed_id = typed_id;
    (*elem).dom_node = opt_ref(get_dom_node(uc, node));
    if !name.is_null() {
        (*elem).name.data = name;
        (*elem).name.length = strlen(name);
    }

    ufbxi_check_return!(
        uc,
        !push_copy_fast::<*mut Element>(&mut (*uc).tmp_element_ptrs, 1, &elem).is_null(),
        core::ptr::null_mut(),
        "ufbxi_push_copy_fast(&uc->tmp_element_ptrs, ufbx_element*, 1, &elem)"
    );

    *p_fbx_id = push_synthetic_id(uc);

    ufbxi_check_return!(
        uc,
        !push_copy_fast::<u64>(&mut (*uc).tmp_element_fbx_ids, 1, p_fbx_id).is_null(),
        core::ptr::null_mut(),
        "ufbxi_push_copy_fast(&uc->tmp_element_fbx_ids, uint64_t, 1, p_fbx_id)"
    );
    ufbxi_check_return!(
        uc,
        insert_fbx_id(uc, *p_fbx_id, element_id).is_ok(),
        core::ptr::null_mut(),
        "ufbxi_insert_fbx_id(uc, *p_fbx_id, element_id)"
    );

    elem
}

// ufbx.c:12416 `#define ufbxi_push_element(uc, info, type_name, type_enum)`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn push_element<T>(
    uc: *mut Context,
    info: *mut ElementInfo,
    type_enum: ElementType,
) -> *mut T {
    ufbxi_maybe_null!(push_element_size(uc, info, size_of::<T>(), type_enum) as *mut T)
}

// ufbx.c:12417 `#define ufbxi_push_synthetic_element(uc, p_fbx_id, node, name, type_name, type_enum)`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn push_synthetic_element<T>(
    uc: *mut Context,
    p_fbx_id: *mut u64,
    node: *mut Node,
    name: *const u8,
    type_enum: ElementType,
) -> *mut T {
    ufbxi_maybe_null!(push_synthetic_element_size(
        uc,
        p_fbx_id,
        node,
        name,
        size_of::<T>(),
        type_enum
    ) as *mut T)
}

// ufbx.c:12419-12427 `ufbxi_connect_oo`
#[inline(never)]
pub(crate) unsafe fn connect_oo(uc: *mut Context, src: u64, dst: u64) -> Result<(), Fail> {
    let conn: *mut TmpConnection = push::<TmpConnection>(&mut (*uc).tmp_connections, 1);
    ufbxi_check!(uc, !conn.is_null(), "conn");
    (*conn).src = src;
    (*conn).dst = dst;
    // C: `conn->src_prop = conn->dst_prop = ufbx_empty_string;`
    (*conn).dst_prop = EMPTY_STRING.0;
    (*conn).src_prop = (*conn).dst_prop;
    Ok(())
}

// ufbx.c:12429-12438 `ufbxi_connect_op`
#[inline(never)]
pub(crate) unsafe fn connect_op(
    uc: *mut Context,
    src: u64,
    dst: u64,
    prop: String,
) -> Result<(), Fail> {
    let conn: *mut TmpConnection = push::<TmpConnection>(&mut (*uc).tmp_connections, 1);
    ufbxi_check!(uc, !conn.is_null(), "conn");
    (*conn).src = src;
    (*conn).dst = dst;
    (*conn).src_prop = EMPTY_STRING.0;
    (*conn).dst_prop = prop;
    Ok(())
}

// ufbx.c:12440-12449 `ufbxi_connect_pp`
#[inline(never)]
pub(crate) unsafe fn connect_pp(
    uc: *mut Context,
    src: u64,
    dst: u64,
    src_prop: String,
    dst_prop: String,
) -> Result<(), Fail> {
    let conn: *mut TmpConnection = push::<TmpConnection>(&mut (*uc).tmp_connections, 1);
    ufbxi_check!(uc, !conn.is_null(), "conn");
    (*conn).src = src;
    (*conn).dst = dst;
    (*conn).src_prop = src_prop;
    (*conn).dst_prop = dst_prop;
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
    (*dst).type_ = type_;
    (*dst).name.data = name;
    (*dst).name.length = strlen(name);
    // C-parity: `dst->value_real` is the `ufbx_prop` value union's first real
    // (`value_vec4.x` in the generated struct).
    (*dst).value_vec4.x = value as Real;
    (*dst).flags = PropFlags::from_raw(
        PropFlags::SYNTHETIC.raw() | PropFlags::VALUE_REAL.raw() | PropFlags::VALUE_INT.raw(),
    );
    (*dst).value_int = value;
    (*dst).value_str.data = EMPTY_CHAR.as_ptr();

    ufbxi_dev_assert!((*dst).name.length >= 4);
    (*dst)._internal_key = get_name_key(name, 4);
}

// ufbx.c:12465-12477 `ufbxi_init_synthetic_real_prop`
#[inline(never)]
pub(crate) unsafe fn init_synthetic_real_prop(
    dst: *mut Prop,
    name: *const u8,
    value: Real,
    type_: PropType,
) {
    (*dst).type_ = type_;
    (*dst).name.data = name;
    (*dst).name.length = strlen(name);
    (*dst).value_vec4.x = value;
    (*dst).flags = PropFlags::from_raw(PropFlags::SYNTHETIC.raw() | PropFlags::VALUE_REAL.raw());
    // C-parity: bare `(int64_t)` cast on a float operand — `as` (saturating),
    // per PORTING.md "Integer semantics".
    (*dst).value_int = value as i64;
    (*dst).value_str.data = EMPTY_CHAR.as_ptr();

    ufbxi_dev_assert!((*dst).name.length >= 4);
    (*dst)._internal_key = get_name_key(name, 4);
}

// ufbx.c:12479-12491 `ufbxi_init_synthetic_vec3_prop`
#[inline(never)]
pub(crate) unsafe fn init_synthetic_vec3_prop(
    dst: *mut Prop,
    name: *const u8,
    value: *const Vec3,
    type_: PropType,
) {
    (*dst).type_ = type_;
    (*dst).name.data = name;
    (*dst).name.length = strlen(name);
    // C: `dst->value_vec3 = *value;` writes only x/y/z of the value union.
    *(&mut (*dst).value_vec4 as *mut Vec4 as *mut Vec3) = *value;
    (*dst).flags = PropFlags::from_raw(PropFlags::SYNTHETIC.raw() | PropFlags::VALUE_VEC3.raw());
    (*dst).value_int = f64_to_i64((*dst).value_vec4.x);
    (*dst).value_str.data = EMPTY_CHAR.as_ptr();

    ufbxi_dev_assert!((*dst).name.length >= 4);
    (*dst)._internal_key = get_name_key(name, 4);
}

// ufbx.c:12493-12505 `ufbxi_set_own_prop_vec3_uniform`
#[inline(never)]
pub(crate) unsafe fn set_own_prop_vec3_uniform(props: *mut Props, name: *const u8, value: Real) {
    // C: `ufbx_props local_props = *props;` — struct memcpy; `Props` is not
    // `Copy` but has no drop glue.
    let mut local_props: Props = core::ptr::read(props);
    local_props.defaults = None;
    let prop: *mut Prop = api_find_prop(&local_props, name);
    if !prop.is_null() {
        (*prop).value_vec4.x = value;
        (*prop).value_vec4.y = value;
        (*prop).value_vec4.z = value;
        (*prop).value_vec4.w = 0.0;
        // C-parity: bare `(int64_t)` cast on a float operand (saturating `as`).
        (*prop).value_int = value as i64;
    }
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
    uc: *mut Context,
    node: *mut UfbxNode,
    node_fbx_id: u64,
) -> Result<(), Fail> {
    let geo_translation: Vec3 = find_vec3(
        &(*node).element.props,
        sp::GeometricTranslation.as_ptr(),
        0.0,
        0.0,
        0.0,
    );
    let geo_rotation: Vec3 = find_vec3(
        &(*node).element.props,
        sp::GeometricRotation.as_ptr(),
        0.0,
        0.0,
        0.0,
    );
    let geo_scaling: Vec3 = find_vec3(
        &(*node).element.props,
        sp::GeometricScaling.as_ptr(),
        1.0,
        1.0,
        1.0,
    );
    if !is_vec3_zero(geo_translation) || !is_vec3_zero(geo_rotation) || !is_vec3_one(geo_scaling) {
        // C: `uint64_t geo_fbx_id;` — written by `ufbxi_push_synthetic_element`
        // before any read; zero-initialized here (no upstream `ufbxi_uninit` marker).
        let mut geo_fbx_id: u64 = 0;
        let geo_node: *mut UfbxNode = push_synthetic_element::<UfbxNode>(
            uc,
            &mut geo_fbx_id,
            core::ptr::null_mut(),
            (*uc).opts.geometry_transform_helper_name.data,
            ElementType::Node,
        );
        ufbxi_check!(uc, !geo_node.is_null(), "geo_node");
        ufbxi_check!(
            uc,
            !push_copy::<u32>(&mut (*uc).tmp_node_ids, 1, &(*geo_node).element.element_id)
                .is_null(),
            "ufbxi_push_copy(&uc->tmp_node_ids, uint32_t, 1, &geo_node->element.element_id)"
        );
        // C: `geo_node->element.dom_node = node->element.dom_node;` — pointer
        // copy; `Option<Ref<T>>` is niche-packed to a bare pointer.
        (*geo_node).element.dom_node = core::ptr::read(&(*node).element.dom_node);

        let props: *mut Prop = push_zero::<Prop>(&mut (*uc).result, 3);
        ufbxi_check!(uc, !props.is_null(), "props");
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

        (*geo_node).element.props.props.data = props;
        (*geo_node).element.props.props.count = 3;

        (*node).has_geometry_transform = true;
        (*geo_node).is_geometry_transform_helper = true;

        connect_oo(uc, geo_fbx_id, node_fbx_id)?;
        (*uc).has_geometry_transform_nodes = true;

        let extra: *mut NodeExtra = push_element_extra(uc, (*node).element.element_id);
        ufbxi_check!(uc, !extra.is_null(), "extra");
        (*extra).geometry_helper_id = (*geo_node).element.element_id;
    }

    Ok(())
}

// ufbx.c:12548-12551 `ufbxi_scale_helper_prop`
#[repr(C)]
pub(crate) struct ScaleHelperProp {
    pub name: *const u8,
    pub default_value: Vec3,
}
// The table below is immutable and its `name` pointers reference immutable
// statics, so sharing is sound (precedent: `native::api::EmptyString`).
unsafe impl Sync for ScaleHelperProp {}

// ufbx.c:12553-12558 `ufbxi_scale_helper_props`
static SCALE_HELPER_PROPS: [ScaleHelperProp; 4] = [
    ScaleHelperProp {
        name: sp::GeometricRotation.as_ptr(),
        default_value: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    },
    ScaleHelperProp {
        name: sp::GeometricScaling.as_ptr(),
        default_value: Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    },
    ScaleHelperProp {
        name: sp::GeometricTranslation.as_ptr(),
        default_value: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    },
    ScaleHelperProp {
        name: sp::Lcl_Scaling.as_ptr(),
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
    uc: *mut Context,
    node: *mut UfbxNode,
    node_fbx_id: u64,
) -> Result<(), Fail> {
    // C: `uint64_t scale_fbx_id;` — written by `ufbxi_push_synthetic_element`
    // before any read; zero-initialized here (no upstream `ufbxi_uninit` marker).
    let mut scale_fbx_id: u64 = 0;
    let scale_node: *mut UfbxNode = push_synthetic_element::<UfbxNode>(
        uc,
        &mut scale_fbx_id,
        core::ptr::null_mut(),
        (*uc).opts.scale_helper_name.data,
        ElementType::Node,
    );
    ufbxi_check!(uc, !scale_node.is_null(), "scale_node");
    ufbxi_check!(
        uc,
        !push_copy::<u32>(
            &mut (*uc).tmp_node_ids,
            1,
            &(*scale_node).element.element_id
        )
        .is_null(),
        "ufbxi_push_copy(&uc->tmp_node_ids, uint32_t, 1, &scale_node->element.element_id)"
    );
    // C: `scale_node->element.dom_node = node->element.dom_node;`
    (*scale_node).element.dom_node = core::ptr::read(&(*node).element.dom_node);

    (*node).scale_helper = Some(Ref::from_ptr(scale_node));
    (*scale_node).is_scale_helper = true;

    connect_oo(uc, scale_fbx_id, node_fbx_id)?;
    (*uc).has_scale_helper_nodes = true;

    let extra: *mut NodeExtra = push_element_extra(uc, (*node).element.element_id);
    ufbxi_check!(uc, !extra.is_null(), "extra");
    (*extra).scale_helper_id = (*scale_node).element.element_id;

    let max_props: usize = SCALE_HELPER_PROPS.len();
    let helper_props: *mut Prop = push::<Prop>(&mut (*uc).result, max_props);
    ufbxi_check!(uc, !helper_props.is_null(), "helper_props");

    let mut num_props: usize = 0;
    // C: `ufbx_props props_copy = node->props;` — struct memcpy.
    let mut props_copy: Props = core::ptr::read(&(*node).element.props);
    props_copy.defaults = None;
    let mut i: usize = 0;
    while i < max_props {
        let hp: *const ScaleHelperProp = &SCALE_HELPER_PROPS[i];
        let src_prop: *mut Prop = find_prop(&props_copy, (*hp).name);
        if src_prop.is_null() {
            i += 1;
            continue;
        }

        *helper_props.add(num_props) = *src_prop;
        num_props += 1;
        // C: `src_prop->value_vec3 = hp->default_value;`
        *(&mut (*src_prop).value_vec4 as *mut Vec4 as *mut Vec3) = (*hp).default_value;
        // C-parity: bare `(int64_t)` cast on a float operand (saturating `as`).
        (*src_prop).value_int = (*src_prop).value_vec4.x as i64;
        i += 1;
    }
    core::mem::forget(props_copy);

    (*scale_node).element.props.props.data = helper_props;
    (*scale_node).element.props.props.count = num_props;

    Ok(())
}

// ufbx.c:12601-12627 `ufbxi_read_model`
#[inline(never)]
pub(crate) unsafe fn read_model(
    uc: *mut Context,
    node: *mut Node,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    ufbxi_ignore!(node);
    let elem_node: *mut UfbxNode = push_element::<UfbxNode>(uc, info, ElementType::Node);
    ufbxi_check!(uc, !elem_node.is_null(), "elem_node");
    ufbxi_check!(
        uc,
        !push_copy::<u32>(&mut (*uc).tmp_node_ids, 1, &(*elem_node).element.element_id).is_null(),
        "ufbxi_push_copy(&uc->tmp_node_ids, uint32_t, 1, &elem_node->element.element_id)"
    );

    let inherit_type: i64 = find_int(&(*elem_node).element.props, sp::InheritType.as_ptr(), -1);
    match inherit_type {
        // RrSs
        0 => (*elem_node).original_inherit_mode = InheritMode::ComponentwiseScale,
        // Rrs
        2 => (*elem_node).original_inherit_mode = InheritMode::IgnoreParentScale,
        _ => {}
    }

    if (*uc).opts.inherit_mode_handling == InheritModeHandling::Preserve {
        (*elem_node).inherit_mode = (*elem_node).original_inherit_mode;
    } else if (*uc).opts.inherit_mode_handling == InheritModeHandling::Ignore {
        (*elem_node).original_inherit_mode = InheritMode::Normal;
        (*elem_node).inherit_mode = InheritMode::Normal;
    }

    Ok(())
}

// ufbx.c:12629-12635 `ufbxi_read_element`
#[inline(never)]
pub(crate) unsafe fn read_element(
    uc: *mut Context,
    node: *mut Node,
    info: *mut ElementInfo,
    size: usize,
    type_: ElementType,
) -> Result<(), Fail> {
    ufbxi_ignore!(node);
    let elem: *mut Element = push_element_size(uc, info, size, type_);
    ufbxi_check!(uc, !elem.is_null(), "elem");
    Ok(())
}

// ufbx.c:12637-12653 `ufbxi_read_unknown`
#[inline(never)]
pub(crate) unsafe fn read_unknown(
    uc: *mut Context,
    node: *mut Node,
    element: *mut ElementInfo,
    type_: String,
    sub_type: String,
    node_name: *const u8,
) -> Result<(), Fail> {
    ufbxi_ignore!(node);
    let unknown: *mut Unknown = push_element::<Unknown>(uc, element, ElementType::Unknown);
    ufbxi_check!(uc, !unknown.is_null(), "unknown");
    (*unknown).type_ = type_;
    (*unknown).sub_type = sub_type;
    (*unknown).super_type.data = node_name;
    (*unknown).super_type.length = strlen(node_name);

    // `type`, `sub_type` and `node_name` are raw strings so they may need to be sanitized.
    push_string_place_str(&mut (*uc).string_pool, &mut (*unknown).type_, false)?;
    push_string_place_str(&mut (*uc).string_pool, &mut (*unknown).sub_type, false)?;
    push_string_place_str(&mut (*uc).string_pool, &mut (*unknown).super_type, false)?;

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
static SENTINEL_INDEX_ZERO: [u32; 1] = [100000000];
static SENTINEL_INDEX_CONSECUTIVE: [u32; 1] = [123456789];

// ufbx.c:12666-12690 `ufbxi_fix_index`
#[inline(never)]
pub(crate) unsafe fn fix_index(
    uc: *mut Context,
    p_dst: *mut u32,
    index: u32,
    one_past_max_val: usize,
) -> Result<(), Fail> {
    match (*uc).opts.index_error_handling {
        IndexErrorHandling::Clamp => {
            ufbxi_check!(uc, one_past_max_val > 0);
            ufbxi_check!(
                uc,
                one_past_max_val <= u32::MAX as usize,
                "one_past_max_val <= UINT32_MAX"
            );
            *p_dst = (one_past_max_val as u32).wrapping_sub(1);
            ufbxi_check!(
                uc,
                ufbxi_warnf!(uc, WarningType::IndexClamped, "Clamped index").is_ok(),
                "ufbxi_warnf(UFBX_WARNING_INDEX_CLAMPED, \"Clamped index\")"
            );
        }
        IndexErrorHandling::NoIndex => {
            *p_dst = NO_INDEX;
        }
        IndexErrorHandling::AbortLoading => {
            // C-parity: `one_past_max_val` is a `size_t` passed through `%u`,
            // which reads an `unsigned int` — the low 32 bits on the oracle
            // targets. The `as u32` narrowing reproduces that exactly.
            ufbxi_fmt_err_info!(
                &mut (*uc).error,
                "%u (max %u)",
                index,
                (if one_past_max_val != 0 {
                    one_past_max_val - 1
                } else {
                    0
                }) as u32
            );
            ufbxi_fail_msg!(uc, "UFBX_INDEX_ERROR_HANDLING_ABORT_LOADING", "Bad index");
            // C: no `break` here — `ufbxi_fail_msg()` returns, so the
            // fallthrough into `UNSAFE_IGNORE` below is unreachable.
        }
        IndexErrorHandling::UnsafeIgnore => {
            *p_dst = index;
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
    uc: *mut Context,
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
        let new_indices: *mut u32 = push::<u32>(&mut (*uc).result, num_indexers);
        ufbxi_check!(uc, !new_indices.is_null(), "new_indices");

        core::ptr::copy_nonoverlapping(indices, new_indices, num_indices);
        for i in num_indices..num_indexers {
            *new_indices.add(i) = NO_INDEX;
        }

        indices = new_indices;
        num_indices = num_indexers;
        owns_indices = true;
    }

    // Normalize out-of-bounds indices to `invalid_index`
    for i in 0..num_indices {
        let ix: u32 = *indices.add(i);
        if ix as usize >= num_elems {
            // If the indices refer to an external buffer we need to
            // allocate a separate buffer for them
            if !owns_indices {
                indices = push_copy::<u32>(&mut (*uc).result, num_indices, indices);
                ufbxi_check!(uc, !indices.is_null(), "indices");
                owns_indices = true;
            }
            fix_index(uc, indices.add(i), ix, num_elems)?;
        }
    }

    *p_dst = indices;

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
    uc: *mut Context,
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
        "ufbxi_warnf(UFBX_WARNING_MISSING_POLYGON_MAPPING, \"Ignoring geometry '%s' with bad mapping mode '%s'\", data_name, mapping)"
    );
    Ok(())
}

// ufbx.c:12739-12908 `ufbxi_read_vertex_element`
#[inline(never)]
pub(crate) unsafe fn read_vertex_element(
    uc: *mut Context,
    mesh: *mut Mesh,
    node: *mut Node,
    attrib: *mut VertexAttrib,
    data_name: *const u8,
    index_name: *const u8,
    w_name: *const u8,
    data_type: u8,
    num_components: usize,
) -> Result<(), Fail> {
    let p_dst_data: *mut *mut Real =
        &mut (*attrib).values.data as *mut *mut c_void as *mut *mut Real;

    let data: *mut ValueArray = find_array(node, data_name, data_type);
    let indices: *mut ValueArray = find_array(node, index_name, b'i');

    if !(*uc).opts.strict {
        if data.is_null() {
            return Ok(());
        }
    }

    ufbxi_check!(uc, !data.is_null(), "data");
    ufbxi_check!(
        uc,
        (*data).size % num_components == 0,
        "data->size % num_components == 0"
    );

    let num_elems: usize = (*data).size / num_components;

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

    (*attrib).exists = true;
    (*attrib).indices.count = (*mesh).num_indices;

    // C: `const char *mapping = "";` — an anonymous empty literal, never
    // pointer-equal to any interned `ufbxi_*` name constant.
    let mut mapping: *const u8 = EMPTY_CHAR.as_ptr();
    ufbxi_ignore!(find_val1(
        node,
        sp::MappingInformationType.as_ptr(),
        b"C\0".as_ptr(),
        &mut mapping as *mut *const u8 as *mut c_void,
    ));

    (*attrib).values.count = if num_elems != 0 { num_elems } else { 1 };

    // Data array is always used as-is, if empty set the data to a global
    // zero buffer so invalid zero index can point to some valid data.
    // The zero data is offset by 4 elements to accommodate for invalid index (-1)
    if num_elems > 0 {
        *p_dst_data = (*data).data as *mut Real;
    } else {
        *p_dst_data = (ZERO_ELEMENT.0.get() as *mut Real).add(4);
    }

    // HACK: Some old exporters seem to use ByPolygon to mean ByPolygonVertex,
    // it should be quite safe to remap this
    if mapping == sp::ByPolygon.as_ptr() {
        let num_indices: usize = if !indices.is_null() {
            (*indices).size
        } else {
            num_elems
        };
        if num_indices == (*mesh).num_indices {
            mapping = sp::ByPolygonVertex.as_ptr();
        }
    }

    if !indices.is_null() {
        let num_indices: usize = (*indices).size;
        let index_data: *mut u32 = (*indices).data as *mut u32;

        if mapping == sp::ByPolygonVertex.as_ptr() {
            // Indexed by polygon vertex: We can use the provided indices directly.
            check_indices(
                uc,
                &mut (*attrib).indices.data as *mut *const u32 as *mut *mut u32,
                index_data,
                true,
                num_indices,
                (*mesh).num_indices,
                num_elems,
            )?;
        } else if mapping == sp::ByVertex.as_ptr() || mapping == sp::ByVertice.as_ptr() {
            // Indexed by vertex: Follow through the position index mapping to get the final indices.
            let new_index_data: *mut u32 = push::<u32>(&mut (*uc).result, (*mesh).num_indices);
            ufbxi_check!(uc, !new_index_data.is_null(), "new_index_data");

            let vert_ix: *mut u32 = (*mesh).vertex_indices.data as *mut u32;
            for i in 0..(*mesh).num_indices {
                let ix: u32 = *vert_ix.add(i);
                if (ix as usize) < num_indices {
                    *new_index_data.add(i) = *index_data.add(ix as usize);
                } else {
                    fix_index(uc, new_index_data.add(i), ix, num_elems)?;
                }
            }

            check_indices(
                uc,
                &mut (*attrib).indices.data as *mut *const u32 as *mut *mut u32,
                new_index_data,
                true,
                (*mesh).num_indices,
                (*mesh).num_indices,
                num_elems,
            )?;
            (*attrib).unique_per_vertex = true;
        } else if mapping == sp::ByPolygon.as_ptr() {
            // Indexed by polygon: Generate new indices based on polygons
            let new_index_data: *mut u32 = push::<u32>(&mut (*uc).result, (*mesh).num_indices);
            ufbxi_check!(uc, !new_index_data.is_null(), "new_index_data");

            let num_faces: usize = (*mesh).num_faces;
            for face_ix in 0..num_faces {
                let face: Face = *(*mesh).faces.data.add(face_ix);
                let mut index: u32 = NO_INDEX;
                if face_ix < num_indices {
                    index = *index_data.add(face_ix);
                }
                if index as usize >= num_elems {
                    fix_index(uc, &mut index, index, num_elems)?;
                }
                for i in 0..face.num_indices as usize {
                    *new_index_data.add(face.index_begin as usize + i) = index;
                }
            }

            (*attrib).indices.data = new_index_data;
        } else if mapping == sp::AllSame.as_ptr() {
            // Indexed by all same: ??? This could be possibly used for making
            // holes with invalid indices, but that seems really fringe.
            // Just use the shared zero index buffer for this.
            (*uc).max_zero_indices = max_sz((*uc).max_zero_indices, (*mesh).num_indices);
            (*attrib).indices.data = SENTINEL_INDEX_ZERO.as_ptr();
            (*attrib).unique_per_vertex = true;
        } else {
            core::ptr::write_bytes(attrib as *mut u8, 0, size_of::<VertexAttrib>());
            warn_polygon_mapping(uc, data_name, mapping)?;
            return Ok(());
        }
    } else {
        if mapping == sp::ByPolygonVertex.as_ptr() {
            // Direct by polygon index: Use shared consecutive array if there's enough
            // elements, otherwise use a unique truncated consecutive index array.
            if num_elems >= (*mesh).num_indices {
                (*uc).max_consecutive_indices =
                    max_sz((*uc).max_consecutive_indices, (*mesh).num_indices);
                (*attrib).indices.data = SENTINEL_INDEX_CONSECUTIVE.as_ptr();
            } else {
                let index_data: *mut u32 = push::<u32>(&mut (*uc).result, (*mesh).num_indices);
                ufbxi_check!(uc, !index_data.is_null(), "index_data");
                for i in 0..(*mesh).num_indices {
                    *index_data.add(i) = i as u32;
                }
                check_indices(
                    uc,
                    &mut (*attrib).indices.data as *mut *const u32 as *mut *mut u32,
                    index_data,
                    true,
                    (*mesh).num_indices,
                    (*mesh).num_indices,
                    num_elems,
                )?;
            }
        } else if mapping == sp::ByVertex.as_ptr() || mapping == sp::ByVertice.as_ptr() {
            // Direct by vertex: We can re-use the position indices..
            check_indices(
                uc,
                &mut (*attrib).indices.data as *mut *const u32 as *mut *mut u32,
                (*mesh).vertex_position.indices.data as *mut u32,
                false,
                (*mesh).num_indices,
                (*mesh).num_indices,
                num_elems,
            )?;
            (*attrib).unique_per_vertex = true;
        } else if mapping == sp::ByPolygon.as_ptr() {
            // Direct by polygon: Generate new indices based on polygons
            let new_index_data: *mut u32 = push::<u32>(&mut (*uc).result, (*mesh).num_indices);
            ufbxi_check!(uc, !new_index_data.is_null(), "new_index_data");

            let num_faces: u32 = (*mesh).num_faces as u32;
            for face_ix in 0..num_faces {
                let face: Face = *(*mesh).faces.data.add(face_ix as usize);
                for i in 0..face.num_indices as usize {
                    *new_index_data.add(face.index_begin as usize + i) = face_ix;
                }
            }

            check_indices(
                uc,
                &mut (*attrib).indices.data as *mut *const u32 as *mut *mut u32,
                new_index_data,
                true,
                (*mesh).num_indices,
                (*mesh).num_indices,
                num_elems,
            )?;
        } else if mapping == sp::AllSame.as_ptr() {
            // Direct by all same: This cannot fail as the index list is just zero.
            (*uc).max_zero_indices = max_sz((*uc).max_zero_indices, (*mesh).num_indices);
            (*attrib).indices.data = SENTINEL_INDEX_ZERO.as_ptr();
            (*attrib).unique_per_vertex = true;
        } else {
            core::ptr::write_bytes(attrib as *mut u8, 0, size_of::<VertexAttrib>());
            warn_polygon_mapping(uc, data_name, mapping)?;
            return Ok(());
        }
    }

    if (*uc).opts.retain_vertex_attrib_w && !w_name.is_null() {
        let w_data: *mut ValueArray = find_array(node, w_name, b'r');
        if !w_data.is_null() {
            if (*w_data).size == num_elems {
                (*attrib).values_w.count = (*w_data).size;
                (*attrib).values_w.data = (*w_data).data as *mut Real;
            } else {
                ufbxi_check!(
                    uc,
                    ufbxi_warnf!(
                        uc,
                        WarningType::BadVertexWAttribute,
                        "Bad W array size %s=%zu, %s=%zu",
                        w_name,
                        (*w_data).size,
                        data_name,
                        num_elems,
                    )
                    .is_ok(),
                    "ufbxi_warnf(UFBX_WARNING_BAD_VERTEX_W_ATTRIBUTE, \"Bad W array size %s=%zu, %s=%zu\", w_name, w_data->size, data_name, num_elems)"
                );
            }
        }
    }

    Ok(())
}

// ufbx.c:12910-12941 `ufbxi_read_truncated_array`
#[inline(never)]
pub(crate) unsafe fn read_truncated_array(
    uc: *mut Context,
    p_data: *mut c_void,
    p_count: *mut usize,
    node: *mut Node,
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
            "ufbxi_warnf(UFBX_WARNING_MISSING_GEOMETRY_DATA, \"Missing geometry data: %s\", name)"
        );
        return Ok(());
    }

    *p_count = size;

    let mut data: *mut c_void = (*arr).data;
    if (*arr).size < size {
        ufbxi_check!(
            uc,
            ufbxi_warnf!(uc, WarningType::TruncatedArray, "Truncated array: %s", name).is_ok(),
            "ufbxi_warnf(UFBX_WARNING_TRUNCATED_ARRAY, \"Truncated array: %s\", name)"
        );

        let elem_size: usize = array_type_size(fmt);
        let new_data: *mut c_void = push_size(&mut (*uc).result, elem_size, size);
        ufbxi_check!(uc, !new_data.is_null(), "new_data");
        core::ptr::copy_nonoverlapping(
            data as *const u8,
            new_data as *mut u8,
            (*arr).size * elem_size,
        );
        // Extend the array with the last element if possible
        if (*arr).size > 0 {
            let first_elem: *mut u8 = (data as *mut u8).add(((*arr).size - 1) * elem_size);
            for i in (*arr).size..size {
                core::ptr::copy_nonoverlapping(
                    first_elem as *const u8,
                    (new_data as *mut u8).add(i * elem_size),
                    elem_size,
                );
            }
        } else {
            core::ptr::write_bytes(new_data as *mut u8, 0, size * elem_size);
        }
        data = new_data;
    }

    *(p_data as *mut *mut c_void) = data;
    Ok(())
}

// ufbx.c:12943-12948 `ufbxi_uv_set_less`
#[inline(never)]
pub(crate) unsafe extern "C" fn uv_set_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const UvSet = va as *const UvSet;
    let b: *const UvSet = vb as *const UvSet;
    (*a).index < (*b).index
}

// ufbx.c:12950-12955 `ufbxi_color_set_less`
#[inline(never)]
pub(crate) unsafe extern "C" fn color_set_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const ColorSet = va as *const ColorSet;
    let b: *const ColorSet = vb as *const ColorSet;
    (*a).index < (*b).index
}

// ufbx.c:12957-12962 `ufbxi_sort_uv_sets`
#[inline(never)]
pub(crate) unsafe fn sort_uv_sets(
    uc: *mut Context,
    sets: *mut UvSet,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            count * size_of::<UvSet>(),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, count * sizeof(ufbx_uv_set))"
    );
    stable_sort(
        size_of::<UvSet>(),
        32,
        sets as *mut c_void,
        (*uc).tmp_arr as *mut c_void,
        count,
        uv_set_less,
        core::ptr::null_mut(),
    );
    Ok(())
}

// ufbx.c:12964-12969 `ufbxi_sort_color_sets`
#[inline(never)]
pub(crate) unsafe fn sort_color_sets(
    uc: *mut Context,
    sets: *mut ColorSet,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            count * size_of::<ColorSet>(),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, count * sizeof(ufbx_color_set))"
    );
    stable_sort(
        size_of::<ColorSet>(),
        32,
        sets as *mut c_void,
        (*uc).tmp_arr as *mut c_void,
        count,
        color_set_less,
        core::ptr::null_mut(),
    );
    Ok(())
}

// ufbx.c:12971-12975 `ufbxi_blend_offset`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct BlendOffset {
    pub vertex: u32,
    pub position_offset: Vec3,
    pub normal_offset: Vec3,
}

// ufbx.c:12977-12982 `ufbxi_blend_offset_less`
#[inline(never)]
pub(crate) unsafe extern "C" fn blend_offset_less(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> bool {
    ufbxi_ignore!(user);
    let a: *const BlendOffset = va as *const BlendOffset;
    let b: *const BlendOffset = vb as *const BlendOffset;
    (*a).vertex < (*b).vertex
}

// ufbx.c:12984-12989 `ufbxi_sort_blend_offsets`
#[inline(never)]
pub(crate) unsafe fn sort_blend_offsets(
    uc: *mut Context,
    offsets: *mut BlendOffset,
    count: usize,
) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        grow_array::<u8>(
            &mut (*uc).ator_tmp,
            &mut (*uc).tmp_arr,
            &mut (*uc).tmp_arr_size,
            count * size_of::<BlendOffset>(),
        ),
        "ufbxi_grow_array(&uc->ator_tmp, &uc->tmp_arr, &uc->tmp_arr_size, count * sizeof(ufbxi_blend_offset))"
    );
    stable_sort(
        size_of::<BlendOffset>(),
        16,
        offsets as *mut c_void,
        (*uc).tmp_arr as *mut c_void,
        count,
        blend_offset_less,
        core::ptr::null_mut(),
    );
    Ok(())
}

// ufbx.c:12991-13060 `ufbxi_read_shape`
#[inline(never)]
pub(crate) unsafe fn read_shape(
    uc: *mut Context,
    node: *mut Node,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    let node_vertices: *mut Node = find_child(node, sp::Vertices.as_ptr());
    let node_indices: *mut Node = find_child(node, sp::Indexes.as_ptr());
    let node_normals: *mut Node = find_child(node, sp::Normals.as_ptr());
    if node_vertices.is_null() || node_indices.is_null() {
        return Ok(());
    }

    let shape: *mut BlendShape = push_element::<BlendShape>(uc, info, ElementType::BlendShape);
    ufbxi_check!(uc, !shape.is_null(), "shape");

    if (*uc).opts.ignore_geometry {
        return Ok(());
    }

    let vertices: *mut ValueArray = get_array(node_vertices, b'r');
    let indices: *mut ValueArray = get_array(node_indices, b'i');

    ufbxi_check!(
        uc,
        !vertices.is_null() && !indices.is_null(),
        "vertices && indices"
    );
    ufbxi_check!(uc, (*vertices).size % 3 == 0, "vertices->size % 3 == 0");
    ufbxi_check!(
        uc,
        (*indices).size == (*vertices).size / 3,
        "indices->size == vertices->size / 3"
    );

    let num_offsets: usize = (*indices).size;
    let vertex_indices: *mut u32 = (*indices).data as *mut u32;

    (*shape).num_offsets = num_offsets;
    (*shape).position_offsets.data = (*vertices).data as *const Vec3;
    (*shape).offset_vertices.data = vertex_indices;
    (*shape).position_offsets.count = num_offsets;
    (*shape).offset_vertices.count = num_offsets;

    if !node_normals.is_null() {
        let normals: *mut ValueArray = get_array(node_normals, b'r');
        ufbxi_check!(
            uc,
            !normals.is_null() && (*normals).size == (*vertices).size,
            "normals && normals->size == vertices->size"
        );
        (*shape).normal_offsets.data = (*normals).data as *const Vec3;
        (*shape).normal_offsets.count = num_offsets;
    }

    // Sort the blend shape vertices only if absolutely necessary
    let mut sorted: bool = true;
    for i in 1..num_offsets {
        if *vertex_indices.add(i - 1) > *vertex_indices.add(i) {
            sorted = false;
            break;
        }
    }

    if !sorted {
        let offsets: *mut BlendOffset = push::<BlendOffset>(&mut (*uc).tmp_stack, num_offsets);
        ufbxi_check!(uc, !offsets.is_null(), "offsets");

        for i in 0..num_offsets {
            (*offsets.add(i)).vertex = *(*shape).offset_vertices.data.add(i);
            (*offsets.add(i)).position_offset = *(*shape).position_offsets.data.add(i);
            if !node_normals.is_null() {
                (*offsets.add(i)).normal_offset = *(*shape).normal_offsets.data.add(i);
            }
        }

        sort_blend_offsets(uc, offsets, num_offsets)?;

        for i in 0..num_offsets {
            *((*shape).offset_vertices.data as *mut u32).add(i) = (*offsets.add(i)).vertex;
            *((*shape).position_offsets.data as *mut Vec3).add(i) =
                (*offsets.add(i)).position_offset;
            if !node_normals.is_null() {
                *((*shape).normal_offsets.data as *mut Vec3).add(i) =
                    (*offsets.add(i)).normal_offset;
            }
        }
        pop::<BlendOffset>(&mut (*uc).tmp_stack, num_offsets, core::ptr::null_mut());
    }

    Ok(())
}

// ufbx.c:13062-13137 `ufbxi_read_synthetic_blend_shapes`
#[inline(never)]
pub(crate) unsafe fn read_synthetic_blend_shapes(
    uc: *mut Context,
    node: *mut Node,
    info: *mut ElementInfo,
) -> Result<(), Fail> {
    let mut deformer: *mut BlendDeformer = core::ptr::null_mut();
    let mut deformer_fbx_id: u64 = 0;

    // C: `ufbxi_for (ufbxi_node, n, node->children, node->num_children)`
    let mut n = (*node).children;
    let n_end = add_ptr(n, (*node).num_children as usize);
    while n != n_end {
        if (*n).name != sp::Shape.as_ptr() {
            n = n.add(1);
            continue;
        }

        // C: `ufbx_string name;` — fully written by `ufbxi_get_val1` before any
        // read; zero-initialized here (no upstream `ufbxi_uninit` marker).
        let mut name: String = core::mem::zeroed();
        ufbxi_check!(
            uc,
            get_val1(n, b"S\0".as_ptr(), &mut name as *mut String as *mut c_void),
            "ufbxi_get_val1(n, \"S\", &name)"
        );

        if deformer.is_null() {
            deformer = push_synthetic_element::<BlendDeformer>(
                uc,
                &mut deformer_fbx_id,
                n,
                name.data,
                ElementType::BlendDeformer,
            );
            ufbxi_check!(uc, !deformer.is_null(), "deformer");
            connect_oo(uc, deformer_fbx_id, (*info).fbx_id)?;
        }

        let mut channel_fbx_id: u64 = 0;
        let channel: *mut BlendChannel = push_synthetic_element::<BlendChannel>(
            uc,
            &mut channel_fbx_id,
            n,
            name.data,
            ElementType::BlendChannel,
        );
        ufbxi_check!(uc, !channel.is_null(), "channel");

        // C: `ufbx_real_list weight_list = { NULL, 0 };`
        let weight_list: List<Real> = core::mem::zeroed();
        ufbxi_check!(
            uc,
            !push_copy::<List<Real>>(&mut (*uc).tmp_full_weights, 1, &weight_list).is_null(),
            "ufbxi_push_copy(&uc->tmp_full_weights, ufbx_real_list, 1, &weight_list)"
        );

        let num_shape_props: usize = 1;
        let shape_props: *mut Prop = push_zero::<Prop>(&mut (*uc).result, num_shape_props);
        ufbxi_check!(uc, !shape_props.is_null(), "shape_props");
        (*shape_props.add(0)).name.data = sp::DeformPercent.as_ptr();
        (*shape_props.add(0)).name.length = sp::DeformPercent.len() - 1;
        (*shape_props.add(0))._internal_key = get_name_key_c(sp::DeformPercent.as_ptr());
        (*shape_props.add(0)).type_ = PropType::Number;
        // C-parity: `shape_props[0].value_real` is the `ufbx_prop` value
        // union's first real (`value_vec4.x` in the generated struct).
        (*shape_props.add(0)).value_vec4.x = 0.0 as Real;
        (*shape_props.add(0)).value_str = EMPTY_STRING.0;
        (*shape_props.add(0)).value_blob = EMPTY_BLOB.0;

        let self_prop: *mut Prop = find_prop_len(&(*info).props, name.data, name.length);
        if !self_prop.is_null()
            && ((*self_prop).type_ == PropType::Number || (*self_prop).type_ == PropType::Integer)
        {
            (*shape_props.add(0)).value_vec4.x = (*self_prop).value_vec4.x;
            connect_pp(
                uc,
                (*info).fbx_id,
                channel_fbx_id,
                name,
                (*shape_props.add(0)).name,
            )?;
        } else if (*uc).version < 6000 {
            connect_pp(
                uc,
                (*info).fbx_id,
                channel_fbx_id,
                name,
                (*shape_props.add(0)).name,
            )?;
        }

        (*channel).element.name = name;
        (*channel).element.props.props.data = shape_props;
        (*channel).element.props.props.count = num_shape_props;

        // C: `ufbxi_element_info shape_info = { 0 };`
        let mut shape_info: ElementInfo = core::mem::zeroed();

        shape_info.fbx_id = push_synthetic_id(uc);
        shape_info.name = name;
        shape_info.dom_node = get_dom_node(uc, n);

        read_shape(uc, n, &mut shape_info)?;

        connect_oo(uc, channel_fbx_id, deformer_fbx_id)?;
        connect_oo(uc, shape_info.fbx_id, channel_fbx_id)?;

        n = n.add(1);
    }

    Ok(())
}

// ufbx.c:13139-13216 `ufbxi_process_indices`
#[inline(never)]
pub(crate) unsafe fn process_indices(
    uc: *mut Context,
    mesh: *mut Mesh,
    index_data: *mut u32,
) -> Result<(), Fail> {
    // Count the number of faces and allocate the index list
    // Indices less than zero (~actual_index) ends a polygon
    let mut num_total_faces: usize = 0;
    // C: `ufbxi_for (uint32_t, p_ix, index_data, mesh->num_indices)`
    let mut p_ix = index_data;
    let p_ix_end = add_ptr(index_data, (*mesh).num_indices);
    while p_ix != p_ix_end {
        num_total_faces =
            num_total_faces.wrapping_add(if (*p_ix as i32) < 0 { 1usize } else { 0usize });
        p_ix = p_ix.add(1);
    }
    (*mesh).faces.data = push::<Face>(&mut (*uc).result, num_total_faces);
    ufbxi_check!(uc, !(*mesh).faces.data.is_null(), "mesh->faces.data");

    let mut num_triangles: usize = 0;
    let mut max_face_triangles: usize = 0;
    let mut num_bad_faces: [usize; 3] = [0; 3];

    let mut dst_face: *mut Face = (*mesh).faces.data as *mut Face;
    let mut p_face_begin: *mut u32 = index_data;
    // C: `ufbxi_for (uint32_t, p_ix, index_data, mesh->num_indices)`
    let mut p_ix = index_data;
    let p_ix_end = add_ptr(index_data, (*mesh).num_indices);
    while p_ix != p_ix_end {
        let mut ix: u32 = *p_ix;
        // Un-negate final indices of polygons
        if (ix as i32) < 0 {
            ix = !ix;
            *p_ix = ix;
            let num_indices: u32 = (p_ix.offset_from(p_face_begin) + 1) as u32;
            (*dst_face).index_begin = p_face_begin.offset_from(index_data) as u32;
            (*dst_face).num_indices = num_indices;
            if num_indices >= 3 {
                num_triangles = num_triangles.wrapping_add((num_indices - 2) as usize);
                max_face_triangles = max_sz(max_face_triangles, (num_indices - 2) as usize);
            } else {
                num_bad_faces[num_indices as usize] =
                    num_bad_faces[num_indices as usize].wrapping_add(1);
            }
            dst_face = dst_face.add(1);
            p_face_begin = p_ix.add(1);
        }
        ufbxi_check!(
            uc,
            (ix as usize) < (*mesh).num_vertices,
            "(size_t)ix < mesh->num_vertices"
        );
        p_ix = p_ix.add(1);
    }

    (*mesh).vertex_position.indices.data = index_data;
    (*mesh).num_faces = to_size(dst_face.offset_from((*mesh).faces.data as *mut Face));
    (*mesh).faces.count = (*mesh).num_faces;
    (*mesh).num_triangles = num_triangles;
    (*mesh).max_face_triangles = max_face_triangles;
    (*mesh).num_empty_faces = num_bad_faces[0];
    (*mesh).num_point_faces = num_bad_faces[1];
    (*mesh).num_line_faces = num_bad_faces[2];

    (*mesh).vertex_first_index.count = (*mesh).num_vertices;
    (*mesh).vertex_first_index.data = push::<u32>(&mut (*uc).result, (*mesh).num_vertices);
    ufbxi_check!(
        uc,
        !(*mesh).vertex_first_index.data.is_null(),
        "mesh->vertex_first_index.data"
    );

    // C: `ufbxi_for_list(uint32_t, p_vx_ix, mesh->vertex_first_index)`
    let mut p_vx_ix = (*mesh).vertex_first_index.data as *mut u32;
    let p_vx_ix_end = add_ptr(p_vx_ix, (*mesh).vertex_first_index.count);
    while p_vx_ix != p_vx_ix_end {
        *p_vx_ix = NO_INDEX;
        p_vx_ix = p_vx_ix.add(1);
    }

    {
        let num_indices: usize = (*mesh).num_indices;
        let num_vertices: usize = (*mesh).num_vertices;
        let vertex_indices: *mut u32 = (*mesh).vertex_indices.data as *mut u32;
        let vertex_first_index: *mut u32 = (*mesh).vertex_first_index.data as *mut u32;
        let mut ix: usize = 0;
        while ix < num_indices {
            let vx: u32 = *vertex_indices.add(ix);
            if (vx as usize) < num_vertices {
                if *vertex_first_index.add(vx as usize) == NO_INDEX {
                    *vertex_first_index.add(vx as usize) = ix as u32;
                }
            } else {
                fix_index(uc, vertex_indices.add(ix), vx, (*mesh).num_vertices)?;
            }
            ix += 1;
        }
    }

    // HACK(consecutive-faces): Prepare for finalize to re-use a consecutive/zero
    // index buffer for face materials..
    (*uc).max_zero_indices = max_sz((*uc).max_zero_indices, (*mesh).num_faces);
    (*uc).max_consecutive_indices = max_sz((*uc).max_consecutive_indices, (*mesh).num_faces);

    Ok(())
}

// ufbx.c:13219-13240 `ufbxi_patch_mesh_reals`
#[inline(never)]
pub(crate) unsafe fn patch_mesh_reals(mesh: *mut Mesh) {
    (*mesh).vertex_position.value_reals = 3;
    (*mesh).vertex_normal.value_reals = 3;
    (*mesh).vertex_uv.value_reals = 2;
    (*mesh).vertex_tangent.value_reals = 3;
    (*mesh).vertex_bitangent.value_reals = 3;
    (*mesh).vertex_color.value_reals = 4;
    (*mesh).vertex_crease.value_reals = 1;
    (*mesh).skinned_position.value_reals = 3;
    (*mesh).skinned_normal.value_reals = 3;

    // C: `ufbxi_nounroll ufbxi_for_list(ufbx_uv_set, set, mesh->uv_sets)`
    let mut set: *mut UvSet = (*mesh).uv_sets.data as *mut UvSet;
    let set_end = add_ptr(set, (*mesh).uv_sets.count);
    while set != set_end {
        (*set).vertex_uv.value_reals = 2;
        (*set).vertex_tangent.value_reals = 3;
        (*set).vertex_bitangent.value_reals = 3;
        set = set.add(1);
    }

    // C: `ufbxi_nounroll ufbxi_for_list(ufbx_color_set, set, mesh->color_sets)`
    let mut set: *mut ColorSet = (*mesh).color_sets.data as *mut ColorSet;
    let set_end = add_ptr(set, (*mesh).color_sets.count);
    while set != set_end {
        (*set).vertex_color.value_reals = 4;
        set = set.add(1);
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
    let a: i32 = *(va as *const i32);
    let b: i32 = *(vb as *const i32);
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
    (*part).num_faces = (*part).num_faces.wrapping_add(1);
    if num_indices >= 3 {
        (*part).num_triangles = (*part)
            .num_triangles
            .wrapping_add((num_indices - 2) as usize);
    } else {
        // `num_empty/point/line_faces` are consecutive, see static asserts above.
        // cppcheck-suppress objectIndex
        // C-parity: indexing off one field into its two siblings (the static
        // asserts above pin the offsets); ported as pointer arithmetic from the
        // field address rather than a match on `num_indices`.
        let p_empty: *mut usize = core::ptr::addr_of_mut!((*part).num_empty_faces);
        let p: *mut usize = p_empty.add(num_indices as usize);
        *p = (*p).wrapping_add(1);
    }
}

// C: `ufbxi_id_group seen_ids[1 << UFBXI_FACE_GROUP_HASH_BITS];`
const SEEN_IDS_COUNT: usize = 1usize << FACE_GROUP_HASH_BITS;

// ufbx.c:13267-13397 `ufbxi_assign_face_groups`
#[inline(never)]
pub(crate) unsafe fn assign_face_groups(
    buf: *mut Buf,
    error: *mut Error,
    mesh: *mut Mesh,
    p_consecutive_indices: *mut usize,
    retain_parts: bool,
) -> Result<(), Fail> {
    let num_faces: usize = (*mesh).num_faces;
    ufbxi_check_err!(error, num_faces > 0);
    ufbxi_check_err!(
        error,
        num_faces < u32::MAX as usize,
        "num_faces < UINT32_MAX"
    );
    ufbxi_check_err!(
        error,
        (*mesh).face_group.count == num_faces,
        "mesh->face_group.count == num_faces"
    );

    let ids: *mut u32 = push::<u32>(buf, num_faces);
    ufbxi_check_err!(error, !ids.is_null(), "ids");

    let mut num_ids: u32 = 0;

    // C declares `seen_ids` uninitialized and zeroes it with the `memset` below.
    let mut seen_ids: [IdGroup; SEEN_IDS_COUNT] = core::mem::zeroed();
    core::ptr::write_bytes(seen_ids.as_mut_ptr(), 0, SEEN_IDS_COUNT);

    let mut seed: u32 = 2654435769u32;
    let mut rehash_threshold: u32 = 256;

    // Loosely deduplicate group IDs
    // C: `ufbxi_for_list(uint32_t, p_id, mesh->face_group)`
    let mut p_id: *mut u32 = (*mesh).face_group.data as *mut u32;
    let p_id_end = add_ptr(p_id, (*mesh).face_group.count);
    while p_id != p_id_end {
        let id: u32 = *p_id;
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
            *ids.add(num_ids as usize) = id;
            num_ids = num_ids.wrapping_add(1);
        }
        p_id = p_id.add(1);
    }

    // Sort and deduplicate remaining IDs
    unstable_sort(
        ids as *mut c_void,
        num_ids as usize,
        size_of::<u32>(),
        less_int32,
        core::ptr::null_mut(),
    );

    let mut num_groups: usize = 0;
    let mut i: usize = 0;
    while i < num_ids as usize {
        let id: u32 = *ids.add(i);
        // C: `ids[num_groups++] = id;`
        *ids.add(num_groups) = id;
        num_groups += 1;
        // C: `do { i++; } while (i < num_ids && ids[i] == id);`
        loop {
            i += 1;
            if !(i < num_ids as usize && *ids.add(i) == id) {
                break;
            }
        }
    }

    // Allocate group info structs
    let groups: *mut FaceGroup = push_zero::<FaceGroup>(buf, num_groups);
    ufbxi_check_err!(error, !groups.is_null(), "groups");
    for i in 0..num_groups {
        (*groups.add(i)).id = *ids.add(i) as i32;
        (*groups.add(i)).name.data = EMPTY_CHAR.as_ptr();
    }

    (*mesh).face_groups.data = groups;
    (*mesh).face_groups.count = num_groups;

    let mut parts: *mut MeshPart = core::ptr::null_mut();
    if retain_parts {
        parts = push_zero::<MeshPart>(buf, num_groups);
        ufbxi_check_err!(error, !parts.is_null(), "parts");
        (*mesh).face_group_parts.data = parts;
        (*mesh).face_group_parts.count = num_groups;
    }

    // Optimization: Use `consecutive_indices` for a single group
    if !p_consecutive_indices.is_null() && num_groups == 1 {
        core::ptr::write_bytes((*mesh).face_group.data as *mut u32, 0, num_faces);

        if !parts.is_null() {
            (*parts.add(0)).face_indices.data = SENTINEL_INDEX_CONSECUTIVE.as_ptr() as *mut u32;
            (*parts.add(0)).face_indices.count = num_faces;
            (*parts.add(0)).num_empty_faces = (*mesh).num_empty_faces;
            (*parts.add(0)).num_point_faces = (*mesh).num_point_faces;
            (*parts.add(0)).num_line_faces = (*mesh).num_line_faces;
            (*parts.add(0)).num_faces = num_faces;
            (*parts.add(0)).num_triangles = (*mesh).num_triangles;
        }

        *p_consecutive_indices = max_sz(*p_consecutive_indices, num_faces);
        return Ok(());
    }

    core::ptr::write_bytes(seen_ids.as_mut_ptr(), 0, SEEN_IDS_COUNT);

    // Count faces and triangles per group and reassign IDs
    let mut p_face: *const Face = (*mesh).faces.data;
    // C: `ufbxi_for_list(uint32_t, p_id, mesh->face_group)`
    let mut p_id: *mut u32 = (*mesh).face_group.data as *mut u32;
    let p_id_end = add_ptr(p_id, (*mesh).face_group.count);
    while p_id != p_id_end {
        let id: u32 = *p_id;
        let id_hash: u32 = id.wrapping_mul(seed) >> (32u32 - FACE_GROUP_HASH_BITS);

        let num_indices: u32 = (*p_face).num_indices;

        let mut index: usize;
        if seen_ids[id_hash as usize].id == id && seen_ids[id_hash as usize].index > 0 {
            index = (seen_ids[id_hash as usize].index - 1) as usize;
            *p_id = index as u32;
        } else {
            let signed_id: i32 = id as i32;
            index = usize::MAX;
            macro_lower_bound_eq::<FaceGroup>(
                8,
                &mut index,
                groups,
                0,
                num_groups,
                |a| (*a).id < signed_id,
                |a| (*a).id == signed_id,
            );
            ufbx_assert!(index < num_groups);
            seen_ids[id_hash as usize].id = id;
            seen_ids[id_hash as usize].index = (index as u32).wrapping_add(1);
        }

        if !parts.is_null() {
            mesh_part_add_face(parts.add(index), num_indices);
        }

        *p_id = index as u32;
        p_face = p_face.add(1);
        p_id = p_id.add(1);
    }

    if parts.is_null() {
        return Ok(());
    }

    // Subdivide `ids` for per-group `face_indices`
    let mut face_indices: *mut u32 = ids;
    let mut part_index: u32 = 0;
    // C: `ufbxi_for(ufbx_mesh_part, part, parts, num_groups)`
    let mut part: *mut MeshPart = parts;
    let part_end = add_ptr(parts, num_groups);
    while part != part_end {
        (*part).index = part_index;
        part_index = part_index.wrapping_add(1);
        (*part).face_indices.data = face_indices;
        face_indices = add_ptr(face_indices, (*part).num_faces);
        part = part.add(1);
    }
    ufbx_assert!(face_indices == add_ptr(ids, num_faces));

    // Collect per-group faces
    let mut face_index: u32 = 0;
    // C: `ufbxi_for_list(uint32_t, p_id, mesh->face_group)`
    let mut p_id: *mut u32 = (*mesh).face_group.data as *mut u32;
    let p_id_end = add_ptr(p_id, (*mesh).face_group.count);
    while p_id != p_id_end {
        let part: *mut MeshPart = parts.add(*p_id as usize);
        // C: `part->face_indices.data[part->face_indices.count++] = face_index++;`
        *((*part).face_indices.data as *mut u32).add((*part).face_indices.count) = face_index;
        (*part).face_indices.count += 1;
        face_index = face_index.wrapping_add(1);
        p_id = p_id.add(1);
    }

    Ok(())
}

// ufbx.c:13399-13432 `ufbxi_update_face_groups`
#[inline(never)]
pub(crate) unsafe fn update_face_groups(
    buf: *mut Buf,
    error: *mut Error,
    mesh: *mut Mesh,
    need_copy: bool,
) -> Result<(), Fail> {
    let num_faces: usize = (*mesh).faces.count;
    let num_groups: usize = (*mesh).face_group_parts.count;
    if num_groups == 0 {
        return Ok(());
    }

    if need_copy {
        (*mesh).face_group_parts.data = push_zero::<MeshPart>(buf, num_groups);
        ufbxi_check_err!(
            error,
            !(*mesh).face_group_parts.data.is_null(),
            "mesh->face_group_parts.data"
        );
    }

    let mut face_indices: *mut u32 = push::<u32>(buf, num_faces);
    ufbxi_check_err!(error, !face_indices.is_null(), "face_indices");

    // C: `ufbxi_nounroll for (size_t i = 0; i < num_faces; i++)`
    for i in 0..num_faces {
        let part: *mut MeshPart = ((*mesh).face_group_parts.data as *mut MeshPart)
            .add(*(*mesh).face_group.data.add(i) as usize);
        mesh_part_add_face(part, (*(*mesh).faces.data.add(i)).num_indices);
    }

    let mut part_index: u32 = 0;
    // C: `ufbxi_for_list(ufbx_mesh_part, part, mesh->face_group_parts)`
    let mut part: *mut MeshPart = (*mesh).face_group_parts.data as *mut MeshPart;
    let part_end = add_ptr(part, (*mesh).face_group_parts.count);
    while part != part_end {
        (*part).index = part_index;
        part_index = part_index.wrapping_add(1);
        (*part).face_indices.data = face_indices;
        (*part).face_indices.count = 0;
        face_indices = add_ptr(face_indices, (*part).num_faces);
        part = part.add(1);
    }

    // C: `ufbxi_nounroll for (uint32_t i = 0; i < num_faces; i++)`
    let mut i: u32 = 0;
    while (i as usize) < num_faces {
        let part: *mut MeshPart = ((*mesh).face_group_parts.data as *mut MeshPart)
            .add(*(*mesh).face_group.data.add(i as usize) as usize);
        // C: `part->face_indices.data[part->face_indices.count++] = i;`
        *((*part).face_indices.data as *mut u32).add((*part).face_indices.count) = i;
        (*part).face_indices.count += 1;
        i = i.wrapping_add(1);
    }

    Ok(())
}

// CONTINUATION POINT (milestone 6b ends here): ufbx.c:13434 `ufbxi_read_mesh`,
// then `ufbxi_read_nurbs_topology` (13813) onwards.
// The `// -- Reading the parsed data` banner section runs to ufbx.c:15311.
