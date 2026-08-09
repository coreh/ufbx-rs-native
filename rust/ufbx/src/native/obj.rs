//! Port of the `// -- .obj file` banner section (ufbx.c:16767-18065).
//!
//! FIRST UNIT: ufbx.c:16767-17167 — the attribute stride table, the property
//! pop/dedup helper, the `ufbxi_obj_mesh` push/flush pair, the OBJ context
//! init/free, the line reader with its continuation handling, the tokenizer
//! and the vertex / index token parsers. The `#else` (feature-disabled) entry
//! points at ufbx.c:18048-18064 are ported here as well so the module's
//! contract is complete in both feature configurations.
//!
//! SECOND UNIT: ufbx.c:17168-17497 — the face/index line reader
//! (`ufbxi_obj_parse_indices` with its mesh flushing, `usemtl` material
//! binding and face-group dedup), the sliding-window multi-index form, the
//! hex digit parser, `usemtl` material creation, the `ufbxi_obj_cmdN` command
//! packers and the vertex/index finalizers (`ufbxi_obj_pop_vertices`,
//! `ufbxi_obj_setup_attrib`, `ufbxi_obj_pad_colors`).
//!
//! THIRD UNIT: ufbx.c:17326-17365 (`ufbxi_obj_parse_comment`, unblocked by the
//! `ufbxi_match` family landing in `native/parse.rs`) and ufbx.c:17498-18046 —
//! the mesh finalizer (`ufbxi_obj_pop_meshes`), the `.obj` directive loop
//! (`ufbxi_obj_parse_file`), the `.mtl` parser (`ufbxi_obj_flush_material`,
//! `ufbxi_obj_parse_prop`, `ufbxi_obj_parse_mtl_map`, `ufbxi_obj_parse_mtl`),
//! the `.mtl` locator/loader (`ufbxi_obj_load_mtl`) and the two entry points
//! (`ufbxi_obj_load`, `ufbxi_mtl_load`).
//!
//! The whole section is gated on `UFBXI_FEATURE_FORMAT_OBJ`
//! (`#[cfg(feature = "obj")]`).
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]

#[cfg(feature = "obj")]
use crate::generated::{
    ElementType, Face, FaceGroup, Material, Mesh, MeshPart, Node as UfbxNode, OpenFileType, Prop,
    PropFlags, RawStream, ShaderType, Texture, Vec4, VertexAttrib, VertexVec2, VertexVec3,
    WarningType,
};
#[cfg(feature = "obj")]
use crate::native::allocator::{free, grow_array};
#[cfg(feature = "obj")]
use crate::native::api::EMPTY_STRING;
#[cfg(feature = "obj")]
use crate::native::buf::{buf_free, pop, push, push_copy, push_fast, push_pop, push_zero, Buf};
use crate::native::error::Fail;
#[cfg(feature = "obj")]
use crate::native::error::{
    memchr, memcmp, set_err_info, strcmp, ufbxi_check, ufbxi_fail_msg, EMPTY_CHAR,
};
#[cfg(not(feature = "obj"))]
use crate::native::error::{ufbxi_fail_msg, ufbxi_fmt_err_info};
#[cfg(feature = "obj")]
use crate::native::float_parse::parse_double;
#[cfg(feature = "obj")]
use crate::native::hash::{
    hash_ptr, map_cmp_const_char_ptr, map_find, map_free, map_init, map_insert,
};
#[cfg(feature = "obj")]
use crate::native::io::refill;
#[cfg(feature = "obj")]
use crate::native::parse::{
    get_name_key, r#match, report_progress, Context, ElementInfo, FbxIdEntry, ObjAttrib,
    ObjFastIndices, ObjGroupEntry, ObjIndexRange, ObjMesh, OBJ_NUM_ATTRIBS, OBJ_NUM_ATTRIBS_EXT,
};
#[cfg(feature = "obj")]
use crate::native::parse_ascii::is_space;
#[cfg(feature = "obj")]
use crate::native::platform::{
    add_ptr, f64_to_i64, max64, max_sz, min64, min_sz, to_size, ufbx_assert, ufbxi_analysis_assert,
    ufbxi_dev_assert, NO_INDEX,
};
#[cfg(feature = "obj")]
use crate::native::read::{
    connect_oo, connect_op, deduplicate_properties, finalize_mesh, find_fbx_id, fix_index,
    init_file_paths, open_file, push_element, push_synthetic_element, resolve_relative_filename,
    setup_root_node, sort_properties, synthetic_id_from_string, update_face_groups, Strblob,
    SENTINEL_INDEX_CONSECUTIVE, SENTINEL_INDEX_ZERO,
};
#[cfg(feature = "obj")]
use crate::native::string_pool::{push_string_place_blob, push_string_place_str, str_c, str_equal};
#[cfg(feature = "obj")]
use crate::native::warnings::ufbxi_warnf;
#[cfg(feature = "obj")]
use crate::prelude::{Blob, List, Real, String};
#[cfg(feature = "obj")]
use core::ffi::c_void;
#[cfg(feature = "obj")]
use core::mem::MaybeUninit;

// -- .obj file

// ufbx.c:16771-16773 `ufbxi_obj_attrib_stride`
#[cfg(feature = "obj")]
static OBJ_ATTRIB_STRIDE: [u8; 4] = [3, 2, 3, 4];

// ufbx.c:16775 `ufbx_static_assert(obj_attrib_strides, ufbxi_arraycount(ufbxi_obj_attrib_stride) == UFBXI_OBJ_NUM_ATTRIBS_EXT);`
#[cfg(feature = "obj")]
const _: () = assert!(OBJ_ATTRIB_STRIDE.len() == OBJ_NUM_ATTRIBS_EXT);

// ufbx.c:16777-16805 `ufbxi_obj_pop_props`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_pop_props(
    uc: &Context,
    dst: *mut List<Prop>,
    count: usize,
) -> Result<(), Fail> {
    // C: `ufbx_prop_list props; // ufbxi_uninit`
    let mut props: List<Prop> = core::mem::zeroed(); // ufbxi_uninit
    props.count = count;
    props.data = push_pop::<Prop>(uc.result_mut_ptr(), &mut (*uc.obj().get()).tmp_props, count);
    ufbxi_check!(uc, !props.data.is_null(), "props.data");

    // C: `ufbxi_for_list(ufbx_prop, prop, props)`
    let mut prop: *mut Prop = props.data as *mut Prop;
    let prop_end = add_ptr(prop, props.count);
    while prop != prop_end {
        (*prop)._internal_key = get_name_key((*prop).name.data, (*prop).name.length);
        if (*prop).value_str.length == 0 {
            (*prop).value_str.data = EMPTY_CHAR.as_ptr();
        }
        if (*prop).value_int == 0 {
            // C: `prop->value_real` — the first `ufbx_real` of the value union
            // (`value_vec4.x` in the generated struct).
            (*prop).value_int = f64_to_i64((*prop).value_vec4.x as f64);
        }
        if (*prop).value_blob.size == 0 && (*prop).value_str.length > 0 {
            (*prop).value_blob.data = (*prop).value_str.data;
            (*prop).value_blob.size = (*prop).value_str.length;
        }
        prop = prop.add(1);
    }

    if props.count > 1 {
        sort_properties(uc, props.data as *mut Prop, props.count)?;
        deduplicate_properties(&mut props);
    }

    // C: `*dst = props;`
    core::ptr::write(dst, props);
    Ok(())
}

// ufbx.c:16807-16843 `ufbxi_obj_push_mesh`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_push_mesh(uc: &Context) -> Result<(), Fail> {
    let mesh: *mut ObjMesh = push_zero::<ObjMesh>(&mut (*uc.obj().get()).tmp_meshes, 1);
    ufbxi_check!(uc, !mesh.is_null(), "mesh");
    (*uc.obj().get()).mesh = mesh;

    // C: `ufbxi_nounroll for (size_t i = 0; i < UFBXI_OBJ_NUM_ATTRIBS; i++)`
    for i in 0..OBJ_NUM_ATTRIBS {
        (*mesh).vertex_range[i].min_ix = u64::MAX;
    }

    // C: `const char *name = "";`
    let mut name: *const u8 = b"\0".as_ptr();
    if uc.opts_view().obj_split_groups() && (*uc.obj().get()).group.length > 0 {
        name = (*uc.obj().get()).group.data;
    } else if !uc.opts_view().obj_merge_objects() && (*uc.obj().get()).object.length > 0 {
        name = (*uc.obj().get()).object.data;
    } else if !uc.opts_view().obj_merge_groups() && (*uc.obj().get()).group.length > 0 {
        name = (*uc.obj().get()).group.data;
    }

    (*mesh).fbx_node = push_synthetic_element::<UfbxNode>(
        uc,
        &mut (*mesh).fbx_node_id,
        core::ptr::null_mut(),
        name,
        ElementType::Node,
    );
    (*mesh).fbx_mesh = push_synthetic_element::<Mesh>(
        uc,
        &mut (*mesh).fbx_mesh_id,
        core::ptr::null_mut(),
        name,
        ElementType::Mesh,
    );
    ufbxi_check!(
        uc,
        !(*mesh).fbx_node.is_null() && !(*mesh).fbx_mesh.is_null(),
        "mesh->fbx_node && mesh->fbx_mesh"
    );

    (*(*mesh).fbx_mesh).vertex_position.unique_per_vertex = true;

    ufbxi_check!(
        uc,
        !push_copy::<u32>(
            uc.tmp_node_ids_mut_ptr(),
            1,
            &(*(*mesh).fbx_node).element.element_id
        )
        .is_null(),
        "((uint32_t*)ufbxi_push_size_copy((&uc->tmp_node_ids), sizeof(uint32_t), (1), (&mesh->fbx_node->element_id)))"
    );

    uc.obj().set_face_material(NO_INDEX);
    uc.obj().set_face_group(0);
    uc.obj().set_face_group_dirty(true);
    (*uc.obj().get()).material_dirty = true;

    connect_oo(uc, (*mesh).fbx_mesh_id, (*mesh).fbx_node_id)?;
    connect_oo(uc, (*mesh).fbx_node_id, 0)?;

    Ok(())
}

// ufbx.c:16845-16860 `ufbxi_obj_flush_mesh`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_flush_mesh(uc: &Context) -> Result<(), Fail> {
    if (*uc.obj().get()).mesh.is_null() {
        return Ok(());
    }

    let num_props: usize = (*uc.obj().get()).tmp_props.num_items;
    obj_pop_props(
        uc,
        &mut (*(*(*uc.obj().get()).mesh).fbx_mesh).element.props.props,
        num_props,
    )?;

    let num_groups: usize = (*uc.obj().get()).tmp_face_group_infos.num_items;
    let groups: *mut FaceGroup = push_pop::<FaceGroup>(
        uc.result_mut_ptr(),
        &mut (*uc.obj().get()).tmp_face_group_infos,
        num_groups,
    );
    ufbxi_check!(uc, !groups.is_null(), "groups");

    (*(*(*uc.obj().get()).mesh).fbx_mesh).face_groups.data = groups;
    (*(*(*uc.obj().get()).mesh).fbx_mesh).face_groups.count = num_groups;

    Ok(())
}

// ufbx.c:16862-16900 `ufbxi_obj_init`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_init(uc: &Context) -> Result<(), Fail> {
    uc.set_from_ascii(true);
    (*uc.obj().get()).initialized = true;

    // C: `ufbxi_nounroll for (size_t i = 0; i < UFBXI_OBJ_NUM_ATTRIBS_EXT; i++)`
    for i in 0..OBJ_NUM_ATTRIBS_EXT {
        uc.obj().tmp_vertices_at(i).set_ator(uc.ator_tmp_mut_ptr());
        uc.obj().tmp_indices_at(i).set_ator(uc.ator_tmp_mut_ptr());
    }
    (*uc.obj().get()).tmp_color_valid.ator = uc.ator_tmp_mut_ptr();
    (*uc.obj().get()).tmp_faces.ator = uc.ator_tmp_mut_ptr();
    (*uc.obj().get()).tmp_face_material.ator = uc.ator_tmp_mut_ptr();
    (*uc.obj().get()).tmp_face_smoothing.ator = uc.ator_tmp_mut_ptr();
    (*uc.obj().get()).tmp_face_group.ator = uc.ator_tmp_mut_ptr();
    (*uc.obj().get()).tmp_face_group_infos.ator = uc.ator_tmp_mut_ptr();
    (*uc.obj().get()).tmp_meshes.ator = uc.ator_tmp_mut_ptr();
    (*uc.obj().get()).tmp_props.ator = uc.ator_tmp_mut_ptr();

    // .obj parsing does its own yield logic
    uc.set_data_size(uc.data_size() + uc.yield_size());

    (*uc.obj().get()).object.data = EMPTY_CHAR.as_ptr();
    (*uc.obj().get()).group.data = EMPTY_CHAR.as_ptr();

    map_init(
        &mut (*uc.obj().get()).group_map,
        uc.ator_tmp_mut_ptr(),
        map_cmp_const_char_ptr,
        core::ptr::null_mut(),
    );

    // Add a nameless root node with the root ID
    {
        // C: `ufbxi_element_info root_info = { uc->root_id };`
        let mut root_info: ElementInfo = core::mem::zeroed();
        root_info.fbx_id = uc.root_id();
        root_info.name = EMPTY_STRING.0;
        let root: *mut UfbxNode = push_element::<UfbxNode>(uc, &mut root_info, ElementType::Node);
        ufbxi_check!(uc, !root.is_null(), "root");
        setup_root_node(uc, root);
        ufbxi_check!(
            uc,
            !push_copy::<u32>(uc.tmp_node_ids_mut_ptr(), 1, &(*root).element.element_id).is_null(),
            "((uint32_t*)ufbxi_push_size_copy((&uc->tmp_node_ids), sizeof(uint32_t), (1), (&root->element.element_id)))"
        );
    }

    Ok(())
}

// ufbx.c:16902-16923 `ufbxi_obj_free`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) unsafe fn obj_free(uc: &Context) {
    if !(*uc.obj().get()).initialized {
        return;
    }

    // C: `ufbxi_nounroll for (size_t i = 0; i < UFBXI_OBJ_NUM_ATTRIBS_EXT; i++)`
    for i in 0..OBJ_NUM_ATTRIBS_EXT {
        buf_free(uc.obj().tmp_vertices_mut_ptr(i));
        buf_free(uc.obj().tmp_indices_mut_ptr(i));
    }
    buf_free(&mut (*uc.obj().get()).tmp_color_valid);
    buf_free(&mut (*uc.obj().get()).tmp_faces);
    buf_free(&mut (*uc.obj().get()).tmp_face_material);
    buf_free(&mut (*uc.obj().get()).tmp_face_smoothing);
    buf_free(&mut (*uc.obj().get()).tmp_face_group);
    buf_free(&mut (*uc.obj().get()).tmp_face_group_infos);
    buf_free(&mut (*uc.obj().get()).tmp_meshes);
    buf_free(&mut (*uc.obj().get()).tmp_props);

    map_free(&mut (*uc.obj().get()).group_map);

    free::<String>(
        uc.ator_tmp_mut_ptr(),
        (*uc.obj().get()).tokens,
        (*uc.obj().get()).tokens_cap,
    );
    free::<*mut Material>(
        uc.ator_tmp_mut_ptr(),
        (*uc.obj().get()).tmp_materials,
        (*uc.obj().get()).tmp_materials_cap,
    );
}

// ufbx.c:16925-16981 `ufbxi_obj_read_line`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_read_line(uc: &Context) -> Result<(), Fail> {
    ufbxi_dev_assert!(!(*uc.obj().get()).eof);

    let mut offset: usize = 0;

    loop {
        let begin: *const u8 = add_ptr(uc.data() as *mut u8, offset) as *const u8;
        let end: *const u8 = if !begin.is_null() {
            memchr(begin, b'\n', uc.data_size() - offset)
        } else {
            core::ptr::null()
        };
        if end.is_null() {
            if uc.eof() {
                offset = uc.data_size();
                (*uc.obj().get()).eof = true;
                break;
            } else {
                let new_cap: usize = max_sz(1, uc.data_size().wrapping_mul(2));
                ufbxi_check!(
                    uc,
                    !refill(uc, new_cap, false).is_null(),
                    "ufbxi_refill(uc, new_cap, false)"
                );
                continue;
            }
        }

        offset += to_size(end as isize - begin as isize) + 1;

        // Handle line continuations
        let mut esc: *const u8 = end;
        if esc > begin && *esc.offset(-1) == b'\r' {
            esc = esc.offset(-1);
        }
        if esc > begin && *esc.offset(-1) == b'\\' {
            continue;
        }

        break;
    }

    let line_len: usize = offset;

    (*uc.obj().get()).line.data = uc.data();
    (*uc.obj().get()).line.length = line_len;
    uc.set_data(uc.data().add(line_len));
    uc.set_data_size(uc.data_size() - line_len);

    (*uc.obj().get()).read_progress += line_len;
    if (*uc.obj().get()).read_progress >= uc.progress_interval() {
        report_progress(uc)?;
        (*uc.obj().get()).read_progress %= uc.progress_interval();
    }

    if (*uc.obj().get()).eof {
        let new_data: *mut u8 = push::<u8>(uc.tmp_mut_ptr(), line_len + 1);
        ufbxi_check!(uc, !new_data.is_null(), "new_data");
        core::ptr::copy_nonoverlapping((*uc.obj().get()).line.data, new_data, line_len);
        *new_data.add(line_len) = b'\n';
        (*uc.obj().get()).line.data = new_data;
        (*uc.obj().get()).line.length += 1;
    }

    Ok(())
}

// ufbx.c:16983-16997 `ufbxi_obj_span_token`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) unsafe fn obj_span_token(uc: &Context, start_token: usize, end_token: usize) -> String {
    ufbx_assert!(start_token < (*uc.obj().get()).num_tokens);
    let end_token = min_sz(end_token, (*uc.obj().get()).num_tokens - 1);

    ufbx_assert!(start_token <= end_token);
    let start: String = *(*uc.obj().get()).tokens.add(start_token);
    let end: String = *(*uc.obj().get()).tokens.add(end_token);
    let num_between: usize = to_size(end.data as isize - start.data as isize);

    let mut result: String = core::mem::zeroed();
    result.data = start.data;
    result.length = num_between + end.length;
    result
}

// ufbx.c:16999-17065 `ufbxi_obj_tokenize`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_tokenize(uc: &Context) -> Result<(), Fail> {
    let mut ptr: *const u8 = (*uc.obj().get()).line.data;
    let end: *const u8 = ptr.add((*uc.obj().get()).line.length);
    (*uc.obj().get()).num_tokens = 0;

    loop {
        let mut c: u8;

        // Skip whitespace
        loop {
            c = *ptr;
            if c == b' ' || c == b'\t' || c == b'\r' {
                ptr = ptr.add(1);
                continue;
            }

            // Treat line continuations as whitespace
            if c == b'\\' {
                let mut p: *const u8 = ptr.add(1);
                if *p == b'\r' {
                    p = p.add(1);
                }
                if *p == b'\n' && p < end.offset(-1) {
                    ptr = p.add(1);
                    continue;
                }
            }

            break;
        }

        c = *ptr;
        if c == b'\n' {
            break;
        }
        if c == b'#' && (*uc.obj().get()).num_tokens > 0 {
            break;
        }

        let index: usize = (*uc.obj().get()).num_tokens;
        (*uc.obj().get()).num_tokens += 1;
        ufbxi_check!(
            uc,
            grow_array::<String>(
                uc.ator_tmp_mut_ptr(),
                &mut (*uc.obj().get()).tokens,
                &mut (*uc.obj().get()).tokens_cap,
                index + 1
            ),
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->obj.tokens)), (&uc->obj.tokens), (&uc->obj.tokens_cap), (index + 1))"
        );

        let tok: *mut String = (*uc.obj().get()).tokens.add(index);
        (*tok).data = ptr;

        // Treat comment start as a single token
        if c == b'#' {
            ptr = ptr.add(1);
            (*tok).length = 1;
            continue;
        }

        loop {
            // C: `c = *++ptr;`
            ptr = ptr.add(1);
            c = *ptr;

            if is_space(c) {
                break;
            }

            if c == b'\\' {
                let mut p: *const u8 = ptr.add(1);
                if *p == b'\r' {
                    p = p.add(1);
                }
                if *p == b'\n' && p < end.offset(-1) {
                    break;
                }
            }
        }

        (*tok).length = to_size(ptr as isize - (*tok).data as isize);
    }

    Ok(())
}

// ufbx.c:17067-17072 `ufbxi_obj_tokenize_line`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_tokenize_line(uc: &Context) -> Result<(), Fail> {
    obj_read_line(uc)?;
    obj_tokenize(uc)?;
    Ok(())
}

// ufbx.c:17074-17108 `ufbxi_obj_parse_vertex`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) unsafe fn obj_parse_vertex(
    uc: &Context,
    attrib: ObjAttrib,
    offset: usize,
) -> Result<(), Fail> {
    if uc.opts_view().ignore_geometry() {
        return Ok(());
    }

    let dst: *mut Buf = uc.obj().tmp_vertices_mut_ptr(attrib as usize);
    let num_values: usize = OBJ_ATTRIB_STRIDE[attrib as usize] as usize;
    uc.obj()
        .vertex_count_at(attrib as usize)
        .set(uc.obj().vertex_count_at(attrib as usize).get() + 1);

    let mut read_values: usize = num_values;
    if attrib == ObjAttrib::Color {
        if offset + read_values > (*uc.obj().get()).num_tokens {
            read_values = 3;
        }
    }
    ufbxi_check!(
        uc,
        offset + read_values <= (*uc.obj().get()).num_tokens,
        "offset + read_values <= uc->obj.num_tokens"
    );

    let parse_flags: u32 = uc.double_parse_flags();
    let vals: *mut Real = push_fast::<Real>(dst, num_values);
    ufbxi_check!(uc, !vals.is_null(), "vals");
    for i in 0..read_values {
        let str_: String = *(*uc.obj().get()).tokens.add(offset + i);
        // C: `char *end; // ufbxi_uninit`
        let mut end: *const u8 = core::ptr::null(); // ufbxi_uninit
        let val: f64 = parse_double(str_.data, str_.length, &mut end, parse_flags);
        ufbxi_check!(
            uc,
            end == str_.data.add(str_.length),
            "end == str.data + str.length"
        );
        *vals.add(i) = val as Real;
    }

    if read_values < num_values {
        ufbx_assert!(read_values + 1 == num_values);
        ufbx_assert!(attrib == ObjAttrib::Color);
        // C: `vals[read_values] = 1.0f;`
        *vals.add(read_values) = 1.0f32 as Real;
    }

    Ok(())
}

// ufbx.c:17110-17166 `ufbxi_obj_parse_index`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_parse_index(
    uc: &Context,
    s: *mut String,
    attrib: u32,
) -> Result<(), Fail> {
    let mut ptr: *const u8 = (*s).data;
    let end: *const u8 = ptr.add((*s).length);

    let mut negative: bool = false;
    if *ptr == b'-' {
        negative = true;
        ptr = ptr.add(1);
    }

    // As .obj indices are never zero we can detect missing indices
    // by simply not writing to it.
    let mut index: u64 = 0;
    while ptr != end {
        let c: u8 = *ptr;
        if c >= b'0' && c <= b'9' {
            ufbxi_check!(
                uc,
                index < u64::MAX / 10 - 10,
                "index < UINT64_MAX / 10 - 10"
            );
            index = index * 10 + (c as i32 - b'0' as i32) as u64;
        } else if c == b'/' {
            ptr = ptr.add(1);
            break;
        }
        ptr = ptr.add(1);
    }

    if negative {
        let count: usize = uc.obj().vertex_count_at(attrib as usize).get();
        index = if index <= count as u64 {
            count as u64 - index
        } else {
            u64::MAX
        };
    } else {
        // Corrects to zero based indices and wraps 0 to UINT64_MAX (missing)
        index = index.wrapping_sub(1);
    }

    let fast_indices: *mut ObjFastIndices = uc.obj().fast_indices_mut_ptr(attrib as usize);
    if (*fast_indices).num_left == 0 {
        let num_push: usize = 128;
        let dst: *mut u64 = push::<u64>(uc.obj().tmp_indices_mut_ptr(attrib as usize), num_push);
        ufbxi_check!(uc, !dst.is_null(), "dst");
        uc.obj().fast_indices_at(attrib as usize).set_indices(dst);
        uc.obj()
            .fast_indices_at(attrib as usize)
            .set_num_left(num_push);
    }

    // C: `*fast_indices->indices++ = index;`
    *(*fast_indices).indices = index;
    (*fast_indices).indices = (*fast_indices).indices.add(1);
    (*fast_indices).num_left -= 1;

    let mesh: *mut ObjMesh = (*uc.obj().get()).mesh;

    if index != u64::MAX {
        let range: *mut ObjIndexRange = &mut (*mesh).vertex_range[attrib as usize];
        (*range).min_ix = min64((*range).min_ix, index);
        (*range).max_ix = max64((*range).max_ix, index);
    }

    (*s).data = ptr;
    (*s).length = to_size(end as isize - ptr as isize);

    Ok(())
}

// ufbx.c:17168-17296 `ufbxi_obj_parse_indices`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_parse_indices(
    uc: &Context,
    token_begin: usize,
    num_tokens: usize,
) -> Result<(), Fail> {
    let mut flush_mesh: bool = false;
    if (*uc.obj().get()).object_dirty {
        if !uc.opts_view().obj_merge_objects() {
            flush_mesh = true;
        }
        (*uc.obj().get()).object_dirty = false;
    }

    if (*uc.obj().get()).group_dirty {
        if (((*uc.obj().get()).object.length == 0 || uc.opts_view().obj_merge_objects())
            && !uc.opts_view().obj_merge_groups())
            || uc.opts_view().obj_split_groups()
        {
            flush_mesh = true;
        }
        (*uc.obj().get()).group_dirty = false;
        uc.obj().set_face_group_dirty(true);
    }

    if (*uc.obj().get()).mesh.is_null() || flush_mesh {
        obj_flush_mesh(uc)?;
        obj_push_mesh(uc)?;
    }
    let mesh: *mut ObjMesh = (*uc.obj().get()).mesh;

    if (*uc.obj().get()).material_dirty {
        if (*uc.obj().get()).usemtl_fbx_id != 0 {
            let entry: *mut FbxIdEntry = find_fbx_id(uc, (*uc.obj().get()).usemtl_fbx_id);
            ufbx_assert!(!entry.is_null());
            if (*mesh).usemtl_base == 0 || (*entry).user_id < (*mesh).usemtl_base {
                connect_oo(uc, (*uc.obj().get()).usemtl_fbx_id, (*mesh).fbx_node_id)?;

                // C: `uint32_t index = ++uc->obj.usemtl_index;`
                (*uc.obj().get()).usemtl_index = (*uc.obj().get()).usemtl_index.wrapping_add(1);
                let index: u32 = (*uc.obj().get()).usemtl_index;
                ufbxi_check!(uc, index < u32::MAX, "index < UINT32_MAX");
                (*entry).user_id = index;

                if (*mesh).usemtl_base == 0 {
                    (*mesh).usemtl_base = index;
                }
                uc.obj()
                    .set_face_material(index.wrapping_sub((*mesh).usemtl_base));
            }
            // C-parity: the assignment above is immediately overwritten here;
            // both are in the C source and both are kept.
            uc.obj()
                .set_face_material((*entry).user_id.wrapping_sub((*mesh).usemtl_base));
        } else {
            uc.obj().set_face_material(NO_INDEX);
        }
    }

    // EARLY RETURN: Rest of the function should only be related to geometry!
    if uc.opts_view().ignore_geometry() {
        return Ok(());
    }

    if num_tokens == 0 && !uc.opts_view().allow_empty_faces() {
        ufbxi_check!(
            uc,
            ufbxi_warnf!(
                uc,
                WarningType::EmptyFaceRemoved,
                "Empty face has been removed"
            )
            .is_ok(),
            "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_EMPTY_FACE_REMOVED, ~0u, \"Empty face has been removed\")"
        );
        return Ok(());
    }

    if uc.obj().face_group_dirty() {
        let mut name: String = EMPTY_STRING.0;
        if (*uc.obj().get()).group.length > 0
            && ((*uc.obj().get()).object.length > 0 || uc.opts_view().obj_merge_groups())
            && !uc.opts_view().obj_split_groups()
        {
            name = (*uc.obj().get()).group;
        }

        let hash: u32 = hash_ptr!(name.data);
        let mut entry: *mut ObjGroupEntry = map_find(
            &mut (*uc.obj().get()).group_map,
            hash,
            &name.data as *const *const u8 as *const c_void,
        );
        if entry.is_null() {
            entry = map_insert(
                &mut (*uc.obj().get()).group_map,
                hash,
                &name.data as *const *const u8 as *const c_void,
            );
            ufbxi_check!(uc, !entry.is_null(), "entry");
            (*entry).name = name.data;
            (*entry).mesh_id = 0;
            (*entry).local_id = 0;
        }

        let mesh_id: u32 = (*(*mesh).fbx_mesh).element.element_id;
        if (*entry).mesh_id != mesh_id {
            // C: `uint32_t id = mesh->num_groups++;`
            let id: u32 = (*mesh).num_groups;
            (*mesh).num_groups = (*mesh).num_groups.wrapping_add(1);
            (*entry).mesh_id = mesh_id;
            (*entry).local_id = id;

            let group: *mut FaceGroup =
                push_zero::<FaceGroup>(&mut (*uc.obj().get()).tmp_face_group_infos, 1);
            ufbxi_check!(uc, !group.is_null(), "group");
            (*group).id = 0;
            (*group).name = name;
        }

        uc.obj().set_face_group((*entry).local_id);

        if !(*uc.obj().get()).has_face_group {
            (*uc.obj().get()).has_face_group = true;
            ufbxi_check!(
                uc,
                !push_zero::<u32>(&mut (*uc.obj().get()).tmp_face_group, (*uc.obj().get()).tmp_faces.num_items)
                    .is_null(),
                "((uint32_t*)ufbxi_push_size_zero((&uc->obj.tmp_face_group), sizeof(uint32_t), (uc->obj.tmp_faces.num_items)))"
            );
        }

        uc.obj().set_face_group_dirty(false);
    }

    let num_indices: usize = num_tokens;
    ufbxi_check!(
        uc,
        (u32::MAX as usize).wrapping_sub((*mesh).num_indices) >= num_indices,
        "UINT32_MAX - mesh->num_indices >= num_indices"
    );

    let face: *mut Face = push_fast::<Face>(&mut (*uc.obj().get()).tmp_faces, 1);
    ufbxi_check!(uc, !face.is_null(), "face");

    (*face).index_begin = (*mesh).num_indices as u32;
    (*face).num_indices = num_indices as u32;

    (*mesh).num_faces += 1;
    (*mesh).num_indices += num_indices;

    let p_face_mat: *mut u32 = push_fast::<u32>(&mut (*uc.obj().get()).tmp_face_material, 1);
    ufbxi_check!(uc, !p_face_mat.is_null(), "p_face_mat");
    *p_face_mat = uc.obj().face_material();

    if (*uc.obj().get()).has_face_smoothing {
        let p_face_smooth: *mut bool =
            push_fast::<bool>(&mut (*uc.obj().get()).tmp_face_smoothing, 1);
        ufbxi_check!(uc, !p_face_smooth.is_null(), "p_face_smooth");
        *p_face_smooth = uc.obj().face_smoothing();
    }

    if (*uc.obj().get()).has_face_group {
        let p_face_group: *mut u32 = push_fast::<u32>(&mut (*uc.obj().get()).tmp_face_group, 1);
        ufbxi_check!(uc, !p_face_group.is_null(), "p_face_group");
        *p_face_group = uc.obj().face_group();
    }

    for ix in 0..num_indices {
        let mut tok: String = *(*uc.obj().get()).tokens.add(token_begin + ix);
        for attrib in 0..OBJ_NUM_ATTRIBS as u32 {
            obj_parse_index(uc, &mut tok, attrib)?;
        }
    }

    Ok(())
}

// ufbx.c:17298-17304 `ufbxi_obj_parse_multi_indices`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_parse_multi_indices(uc: &Context, window: usize) -> Result<(), Fail> {
    // C: `for (size_t begin = 1; begin + window <= uc->obj.num_tokens; begin++)`
    let mut begin: usize = 1;
    while begin + window <= (*uc.obj().get()).num_tokens {
        obj_parse_indices(uc, begin, window)?;
        begin += 1;
    }
    Ok(())
}

// ufbx.c:17306-17324 `ufbxi_parse_hex`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) unsafe fn parse_hex(digits: *const u8, length: usize) -> u32 {
    let mut value: u32 = 0;

    for i in 0..length {
        // C: `char c = digits[i];` — `char` is signed on the oracle targets
        // (PORTING.md char-value rule). Every range tested below is entirely
        // below 0x80, so bytes >= 0x80 fall through to `v = 0` either way.
        let c: i8 = *(digits.add(i) as *const i8);
        let mut v: u32 = 0;
        if c >= b'0' as i8 && c <= b'9' as i8 {
            v = (c as i32 - b'0' as i32) as u32;
        } else if c >= b'A' as i8 && c <= b'F' as i8 {
            v = (c as i32 - b'A' as i32) as u32 + 10;
        } else if c >= b'a' as i8 && c <= b'f' as i8 {
            v = (c as i32 - b'a' as i32) as u32 + 10;
        }
        value = (value << 4) | v;
    }

    value
}

// ufbx.c:17326-17365 `ufbxi_obj_parse_comment`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_parse_comment(uc: &Context) -> Result<(), Fail> {
    if (*uc.obj().get()).num_tokens >= 3
        && str_equal(*(*uc.obj().get()).tokens.add(1), str_c(b"MRGB\0".as_ptr()))
    {
        let num_color: usize = uc.obj().vertex_count_at(ObjAttrib::Color as usize).get();

        // Pop standard vertex colors and replace them with MRGB colors
        if num_color > (*uc.obj().get()).mrgb_vertex_count {
            let num_pop: usize = num_color - (*uc.obj().get()).mrgb_vertex_count;
            pop::<bool>(
                &mut (*uc.obj().get()).tmp_color_valid,
                num_pop,
                core::ptr::null_mut(),
            );
            pop::<Real>(
                uc.obj().tmp_vertices_mut_ptr(ObjAttrib::Color as usize),
                num_pop * 4,
                core::ptr::null_mut(),
            );
            uc.obj()
                .vertex_count_at(ObjAttrib::Color as usize)
                .set(uc.obj().vertex_count_at(ObjAttrib::Color as usize).get() - num_pop);
        }

        let mrgb: String = *(*uc.obj().get()).tokens.add(2);
        // C: `for (size_t i = 0; i + 8 <= mrgb.length; i += 8)`
        let mut i: usize = 0;
        while i + 8 <= mrgb.length {
            let p_rgba: *mut Real =
                push::<Real>(uc.obj().tmp_vertices_mut_ptr(ObjAttrib::Color as usize), 4);
            let p_valid: *mut bool = push::<bool>(&mut (*uc.obj().get()).tmp_color_valid, 1);
            ufbxi_check!(
                uc,
                !p_rgba.is_null() && !p_valid.is_null(),
                "p_rgba && p_valid"
            );
            *p_valid = true;

            let hex: u32 = parse_hex(mrgb.data.add(i), 8);
            *p_rgba.add(0) = ((hex >> 16) & 0xff) as Real / (255.0f32 as Real);
            *p_rgba.add(1) = ((hex >> 8) & 0xff) as Real / (255.0f32 as Real);
            *p_rgba.add(2) = ((hex >> 0) & 0xff) as Real / (255.0f32 as Real);
            *p_rgba.add(3) = ((hex >> 24) & 0xff) as Real / (255.0f32 as Real);

            i += 8;
        }

        (*uc.obj().get()).has_vertex_color = true;
    }

    if !uc.opts_view().disable_quirks() {
        if r#match(
            &(*uc.obj().get()).line,
            b"\\s*#\\s*File exported by ZBrush.*\0".as_ptr(),
        ) {
            if (*uc.obj().get()).mesh.is_null() {
                uc.opts_view().set_obj_merge_groups(true);
            }
        }
    }

    Ok(())
}

// ufbx.c:17367-17406 `ufbxi_obj_parse_material`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_parse_material(uc: &Context) -> Result<(), Fail> {
    (*uc.obj().get()).material_dirty = true;

    // Allow empty `usemtl` lines to specify "no material".
    if (*uc.obj().get()).num_tokens < 2 {
        (*uc.obj().get()).usemtl_fbx_id = 0;
        return Ok(());
    }

    let mut name: String = obj_span_token(uc, 1, usize::MAX);

    push_string_place_str(uc.string_pool_mut_ptr(), &mut name, false)?;

    let fbx_id: u64 = synthetic_id_from_string(uc, name.data);
    ufbxi_check!(uc, fbx_id != 0, "fbx_id");

    let entry: *mut FbxIdEntry = find_fbx_id(uc, fbx_id);

    (*uc.obj().get()).usemtl_fbx_id = fbx_id;

    if entry.is_null() {
        // C: `ufbxi_element_info info = { 0 };`
        let mut info: ElementInfo = core::mem::zeroed();
        info.fbx_id = fbx_id;
        info.name = name;

        let material: *mut Material =
            push_element::<Material>(uc, &mut info, ElementType::Material);
        ufbxi_check!(uc, !material.is_null(), "material");

        (*material).shader_type = ShaderType::WavefrontMtl;
        (*material).shading_model_name.data = EMPTY_CHAR.as_ptr();
        (*material).shader_prop_prefix.data = EMPTY_CHAR.as_ptr();

        let id: usize = (*material).element.element_id as usize;
        ufbxi_check!(
            uc,
            grow_array::<*mut Material>(
                uc.ator_tmp_mut_ptr(),
                &mut (*uc.obj().get()).tmp_materials,
                &mut (*uc.obj().get()).tmp_materials_cap,
                id + 1
            ),
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->obj.tmp_materials)), (&uc->obj.tmp_materials), (&uc->obj.tmp_materials_cap), (id + 1))"
        );
        *(*uc.obj().get()).tmp_materials.add(id) = material;
    }

    Ok(())
}

// ufbx.c:17408 `#define ufbxi_obj_cmd1(a) ((uint32_t)(a)<<24u)`
#[cfg(feature = "obj")]
#[inline(always)]
pub(crate) const fn obj_cmd1(a: u8) -> u32 {
    (a as u32) << 24
}

// ufbx.c:17409 `#define ufbxi_obj_cmd2(a,b) ((uint32_t)(a)<<24u | (uint32_t)(b)<<16)`
#[cfg(feature = "obj")]
#[inline(always)]
pub(crate) const fn obj_cmd2(a: u8, b: u8) -> u32 {
    (a as u32) << 24 | (b as u32) << 16
}

// ufbx.c:17410 `#define ufbxi_obj_cmd3(a,b,c) ((uint32_t)(a)<<24u | (uint32_t)(b)<<16 | (uint32_t)(c)<<8u)`
// C-parity: the `ufbxi_obj_cmd3` macro has zero call sites in ufbx.c (every
// dispatched OBJ keyword is two characters); kept alongside `ufbxi_obj_cmd2`.
#[allow(dead_code)]
#[cfg(feature = "obj")]
#[inline(always)]
pub(crate) const fn obj_cmd3(a: u8, b: u8, c: u8) -> u32 {
    (a as u32) << 24 | (b as u32) << 16 | (c as u32) << 8
}

// ufbx.c:17412-17432 `ufbxi_obj_pop_vertices`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_pop_vertices(
    uc: &Context,
    dst: *mut List<Real>,
    attrib: u32,
    min_index: u64,
) -> Result<(), Fail> {
    let stride: usize = OBJ_ATTRIB_STRIDE[attrib as usize] as usize;
    ufbxi_check!(
        uc,
        min_index < (uc.obj().tmp_vertices_at(attrib as usize).num_items() / stride) as u64,
        "min_index < uc->obj.tmp_vertices[attrib].num_items / stride"
    );

    let count: usize =
        uc.obj().tmp_vertices_at(attrib as usize).num_items() - (min_index as usize) * stride;
    let mut data: *mut Real = push::<Real>(uc.result_mut_ptr(), count + 4);
    ufbxi_check!(uc, !data.is_null(), "data");

    *data.add(0) = 0.0f32 as Real;
    *data.add(1) = 0.0f32 as Real;
    *data.add(2) = 0.0f32 as Real;
    *data.add(3) = 0.0f32 as Real;
    data = data.add(4);

    pop::<Real>(uc.obj().tmp_vertices_mut_ptr(attrib as usize), count, data);

    (*dst).data = data;
    (*dst).count = count;
    Ok(())
}

// ufbx.c:17434-17481 `ufbxi_obj_setup_attrib`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_setup_attrib(
    uc: &Context,
    mesh: *mut ObjMesh,
    tmp_indices: *mut u64,
    dst: *mut VertexAttrib,
    p_data: *const List<Real>,
    attrib: u32,
    non_disjoint: bool,
    required: bool,
) -> Result<(), Fail> {
    // C: `ufbx_real_list data = *p_data;`
    let data: List<Real> = core::ptr::read(p_data);

    let num_indices: usize = (*mesh).num_indices;
    let stride: usize = OBJ_ATTRIB_STRIDE[attrib as usize] as usize;
    let num_values: usize = data.count / stride;

    let mesh_min_ix: u64 = (*mesh).vertex_range[attrib as usize].min_ix;
    if num_indices == 0 || num_values == 0 || mesh_min_ix == u64::MAX {
        ufbxi_check!(
            uc,
            num_indices == 0 || !required,
            "num_indices == 0 || !required"
        );

        // Pop indices without copying if the attribute is not used
        pop::<u64>(
            uc.obj().tmp_indices_mut_ptr(attrib as usize),
            num_indices,
            core::ptr::null_mut(),
        );
        return Ok(());
    }

    let min_index: u64 = if non_disjoint { 0 } else { mesh_min_ix };

    pop::<u64>(
        uc.obj().tmp_indices_mut_ptr(attrib as usize),
        num_indices,
        tmp_indices,
    );

    let dst_indices: *mut u32 = push::<u32>(uc.result_mut_ptr(), num_indices);
    ufbxi_check!(uc, !dst_indices.is_null(), "dst_indices");

    (*dst).exists = true;

    (*dst).values.data = data.data as *mut c_void;
    (*dst).values.count = num_values;

    (*dst).indices.data = dst_indices;
    (*dst).indices.count = num_indices;

    // C: `ufbxi_nounroll for (size_t i = 0; i < num_indices; i++)`
    for i in 0..num_indices {
        let mut ix: u64 = *tmp_indices.add(i);
        if ix != u64::MAX {
            ix = ix.wrapping_sub(min_index);
            ufbxi_check!(uc, ix < u32::MAX as u64, "ix < UINT32_MAX");
        }
        if ix < num_values as u64 {
            *dst_indices.add(i) = ix as u32;
        } else {
            fix_index(uc, dst_indices.add(i), ix as u32, num_values)?;
        }
    }

    Ok(())
}

// ufbx.c:17483-17496 `ufbxi_obj_pad_colors`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_pad_colors(uc: &Context, num_vertices: usize) -> Result<(), Fail> {
    if uc.opts_view().ignore_geometry() {
        return Ok(());
    }

    let num_colors: usize = uc.obj().vertex_count_at(ObjAttrib::Color as usize).get();
    if num_vertices > num_colors {
        let num_pad: usize = num_vertices - num_colors;
        ufbxi_check!(
            uc,
            !push_zero::<Real>(
                uc.obj().tmp_vertices_mut_ptr(ObjAttrib::Color as usize),
                num_pad * 4
            )
            .is_null(),
            "((ufbx_real*)ufbxi_push_size_zero((&uc->obj.tmp_vertices[UFBXI_OBJ_ATTRIB_COLOR]), sizeof(ufbx_real), (num_pad * 4)))"
        );
        ufbxi_check!(
            uc,
            !push_zero::<bool>(&mut (*uc.obj().get()).tmp_color_valid, num_pad).is_null(),
            "((bool*)ufbxi_push_size_zero((&uc->obj.tmp_color_valid), sizeof(bool), (num_pad)))"
        );
        uc.obj()
            .vertex_count_at(ObjAttrib::Color as usize)
            .set(uc.obj().vertex_count_at(ObjAttrib::Color as usize).get() + num_pad);
    }

    Ok(())
}

// ufbx.c:17498-17679 `ufbxi_obj_pop_meshes`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_pop_meshes(uc: &Context) -> Result<(), Fail> {
    let num_meshes: usize = (*uc.obj().get()).tmp_meshes.num_items;
    let meshes: *mut ObjMesh = push_pop::<ObjMesh>(
        uc.tmp_mut_ptr(),
        &mut (*uc.obj().get()).tmp_meshes,
        num_meshes,
    );
    ufbxi_check!(uc, !meshes.is_null(), "meshes");

    if (*uc.obj().get()).has_vertex_color {
        obj_pad_colors(
            uc,
            uc.obj().vertex_count_at(ObjAttrib::Position as usize).get(),
        )?;
    }

    // Pop unused fast indices
    for i in 0..OBJ_NUM_ATTRIBS {
        pop::<u64>(
            uc.obj().tmp_indices_mut_ptr(i),
            uc.obj().fast_indices_at(i).num_left(),
            core::ptr::null_mut(),
        );
    }

    // Check if the file has disjoint vertices
    let mut non_disjoint: [bool; OBJ_NUM_ATTRIBS] = [false; OBJ_NUM_ATTRIBS];
    let mut next_min: [u64; OBJ_NUM_ATTRIBS] = [0; OBJ_NUM_ATTRIBS];
    // C: `ufbx_real_list vertices[UFBXI_OBJ_NUM_ATTRIBS_EXT] = { 0 };`
    let mut vertices: [List<Real>; OBJ_NUM_ATTRIBS_EXT] = core::mem::zeroed();
    let mut color_valid: *mut bool = core::ptr::null_mut();

    let mut max_indices: usize = 0;

    for i in 0..num_meshes {
        let mesh: *mut ObjMesh = meshes.add(i);
        max_indices = max_sz(max_indices, (*mesh).num_indices);
        // C: `ufbxi_nounroll for (uint32_t attrib = 0; attrib < UFBXI_OBJ_NUM_ATTRIBS; attrib++)`
        for attrib in 0..OBJ_NUM_ATTRIBS {
            let range: ObjIndexRange = (*mesh).vertex_range[attrib];
            if range.min_ix > range.max_ix {
                continue;
            }
            if range.min_ix < next_min[attrib] {
                non_disjoint[attrib] = true;
            }
            next_min[attrib] = range.max_ix.wrapping_add(1);
        }
    }

    let tmp_indices: *mut u64 = push::<u64>(uc.tmp_mut_ptr(), max_indices);
    ufbxi_check!(uc, !tmp_indices.is_null(), "tmp_indices");

    // C: `ufbxi_nounroll for (uint32_t attrib = 0; attrib < UFBXI_OBJ_NUM_ATTRIBS; attrib++)`
    for attrib in 0..OBJ_NUM_ATTRIBS {
        if !non_disjoint[attrib] {
            continue;
        }
        obj_pop_vertices(uc, &mut vertices[attrib], attrib as u32, 0)?;
    }
    if (*uc.obj().get()).has_vertex_color && non_disjoint[ObjAttrib::Position as usize] {
        obj_pop_vertices(
            uc,
            &mut vertices[ObjAttrib::Color as usize],
            ObjAttrib::Color as u32,
            0,
        )?;
        color_valid = push_pop::<bool>(
            uc.tmp_mut_ptr(),
            &mut (*uc.obj().get()).tmp_color_valid,
            vertices[ObjAttrib::Color as usize].count / 4,
        );
        ufbxi_check!(uc, !color_valid.is_null(), "color_valid");
    }

    // C: `for (size_t i = num_meshes; i > 0; i--)`
    let mut i: usize = num_meshes;
    while i > 0 {
        let mesh: *mut ObjMesh = meshes.add(i - 1);

        let fbx_mesh: *mut Mesh = (*mesh).fbx_mesh;

        let num_faces: usize = (*mesh).num_faces;

        if !uc.opts_view().ignore_geometry() {
            // C: `ufbxi_nounroll for (uint32_t attrib = 0; attrib < UFBXI_OBJ_NUM_ATTRIBS; attrib++)`
            for attrib in 0..OBJ_NUM_ATTRIBS {
                if non_disjoint[attrib] {
                    continue;
                }
                let min_ix: u64 = (*mesh).vertex_range[attrib].min_ix;
                if min_ix < u64::MAX {
                    obj_pop_vertices(uc, &mut vertices[attrib], attrib as u32, min_ix)?;
                }
            }
            if (*uc.obj().get()).has_vertex_color && !non_disjoint[ObjAttrib::Position as usize] {
                let min_ix: u64 = (*mesh).vertex_range[ObjAttrib::Position as usize].min_ix;
                ufbxi_check!(uc, min_ix < u64::MAX, "min_ix < UINT64_MAX");
                obj_pop_vertices(
                    uc,
                    &mut vertices[ObjAttrib::Color as usize],
                    ObjAttrib::Color as u32,
                    min_ix,
                )?;
                color_valid = push_pop::<bool>(
                    uc.tmp_mut_ptr(),
                    &mut (*uc.obj().get()).tmp_color_valid,
                    vertices[ObjAttrib::Color as usize].count / 4,
                );
                ufbxi_check!(uc, !color_valid.is_null(), "color_valid");
            }

            (*fbx_mesh).faces.count = num_faces;
            (*fbx_mesh).face_material.count = num_faces;

            (*fbx_mesh).faces.data = push_pop::<Face>(
                uc.result_mut_ptr(),
                &mut (*uc.obj().get()).tmp_faces,
                num_faces,
            );
            (*fbx_mesh).face_material.data = push_pop::<u32>(
                uc.result_mut_ptr(),
                &mut (*uc.obj().get()).tmp_face_material,
                num_faces,
            );

            ufbxi_check!(
                uc,
                !(*fbx_mesh).faces.data.is_null(),
                "fbx_mesh->faces.data"
            );
            ufbxi_check!(
                uc,
                !(*fbx_mesh).face_material.data.is_null(),
                "fbx_mesh->face_material.data"
            );

            if (*uc.obj().get()).has_face_smoothing {
                (*fbx_mesh).face_smoothing.count = num_faces;
                (*fbx_mesh).face_smoothing.data = push_pop::<bool>(
                    uc.result_mut_ptr(),
                    &mut (*uc.obj().get()).tmp_face_smoothing,
                    num_faces,
                );
                ufbxi_check!(
                    uc,
                    !(*fbx_mesh).face_smoothing.data.is_null(),
                    "fbx_mesh->face_smoothing.data"
                );
            }

            if (*uc.obj().get()).has_face_group {
                if (*mesh).num_groups > 1 {
                    (*fbx_mesh).face_group.count = num_faces;
                    (*fbx_mesh).face_group.data = push_pop::<u32>(
                        uc.result_mut_ptr(),
                        &mut (*uc.obj().get()).tmp_face_group,
                        num_faces,
                    );
                    ufbxi_check!(
                        uc,
                        !(*fbx_mesh).face_group.data.is_null(),
                        "fbx_mesh->face_group.data"
                    );
                } else {
                    pop::<u32>(
                        &mut (*uc.obj().get()).tmp_face_group,
                        num_faces,
                        core::ptr::null_mut(),
                    );
                }
            }

            obj_setup_attrib(
                uc,
                mesh,
                tmp_indices,
                &mut (*fbx_mesh).vertex_position as *mut VertexVec3 as *mut VertexAttrib,
                &vertices[ObjAttrib::Position as usize],
                ObjAttrib::Position as u32,
                non_disjoint[ObjAttrib::Position as usize],
                true,
            )?;

            obj_setup_attrib(
                uc,
                mesh,
                tmp_indices,
                &mut (*fbx_mesh).vertex_uv as *mut VertexVec2 as *mut VertexAttrib,
                &vertices[ObjAttrib::Uv as usize],
                ObjAttrib::Uv as u32,
                non_disjoint[ObjAttrib::Uv as usize],
                false,
            )?;

            obj_setup_attrib(
                uc,
                mesh,
                tmp_indices,
                &mut (*fbx_mesh).vertex_normal as *mut VertexVec3 as *mut VertexAttrib,
                &vertices[ObjAttrib::Normal as usize],
                ObjAttrib::Normal as u32,
                non_disjoint[ObjAttrib::Normal as usize],
                false,
            )?;

            if (*uc.obj().get()).has_vertex_color {
                ufbx_assert!(!color_valid.is_null());
                let mut has_color: bool = false;
                let mut all_valid: bool = true;
                let max_index: usize = (*fbx_mesh).vertex_position.values.count;
                // C: `ufbxi_for_list(uint32_t, p_ix, fbx_mesh->vertex_position.indices)`
                let mut p_ix: *mut u32 = (*fbx_mesh).vertex_position.indices.data as *mut u32;
                let p_ix_end: *mut u32 = add_ptr(p_ix, (*fbx_mesh).vertex_position.indices.count);
                while p_ix != p_ix_end {
                    if (*p_ix as usize) < max_index {
                        if *color_valid.add(*p_ix as usize) {
                            has_color = true;
                        } else {
                            all_valid = false;
                        }
                    }
                    p_ix = p_ix.add(1);
                }

                if has_color {
                    (*fbx_mesh).vertex_color.exists = true;
                    (*fbx_mesh).vertex_color.values.data =
                        vertices[ObjAttrib::Color as usize].data as *const Vec4;
                    (*fbx_mesh).vertex_color.values.count =
                        vertices[ObjAttrib::Color as usize].count / 4;
                    // C: `fbx_mesh->vertex_color.indices = fbx_mesh->vertex_position.indices;`
                    core::ptr::write(
                        &mut (*fbx_mesh).vertex_color.indices,
                        core::ptr::read(&(*fbx_mesh).vertex_position.indices),
                    );
                    (*fbx_mesh).vertex_color.unique_per_vertex = true;

                    if !all_valid {
                        let mut indices: *mut u32 =
                            (*fbx_mesh).vertex_color.indices.data as *mut u32;
                        indices =
                            push_copy::<u32>(uc.result_mut_ptr(), (*mesh).num_indices, indices);
                        ufbxi_check!(uc, !indices.is_null(), "indices");

                        let num_values: usize = (*fbx_mesh).vertex_color.values.count;
                        // C: `ufbxi_for(uint32_t, p_ix, indices, mesh->num_indices)`
                        let mut p_ix: *mut u32 = indices;
                        let p_ix_end: *mut u32 = add_ptr(p_ix, (*mesh).num_indices);
                        while p_ix != p_ix_end {
                            if *p_ix as usize >= num_values || !*color_valid.add(*p_ix as usize) {
                                fix_index(uc, p_ix, *p_ix, num_values)?;
                            }
                            p_ix = p_ix.add(1);
                        }

                        (*fbx_mesh).vertex_color.indices.data = indices;
                    }
                }
            }
        }

        finalize_mesh(uc.result_mut_ptr(), uc.error_mut_ptr(), fbx_mesh)?;

        if uc.retain_mesh_parts() {
            (*fbx_mesh).face_group_parts.count = (*mesh).num_groups as usize;
            (*fbx_mesh).face_group_parts.data =
                push_zero::<MeshPart>(uc.result_mut_ptr(), (*mesh).num_groups as usize);
            ufbxi_check!(
                uc,
                !(*fbx_mesh).face_group_parts.data.is_null(),
                "fbx_mesh->face_group_parts.data"
            );
        }

        if (*mesh).num_groups > 1 {
            update_face_groups(uc.result_mut_ptr(), uc.error_mut_ptr(), fbx_mesh, false)?;
        } else if (*mesh).num_groups == 1 {
            (*fbx_mesh).face_group.data = SENTINEL_INDEX_ZERO.as_ptr();
            (*fbx_mesh).face_group.count = num_faces;
            // NOTE: Consecutive and zero indices are always allocated so we can skip doing it here,
            // see HACK(consecutiv-faces)..
            if (*fbx_mesh).face_group_parts.count > 0 {
                let part: *mut MeshPart = (*fbx_mesh).face_group_parts.data as *mut MeshPart;
                // C-parity: `part->num_faces` is assigned twice in a row
                // (ufbx.c:17662-17663); the second write wins. Both are kept.
                (*part).num_faces = (*fbx_mesh).num_faces;
                (*part).num_faces = num_faces;
                (*part).num_empty_faces = (*fbx_mesh).num_empty_faces;
                (*part).num_point_faces = (*fbx_mesh).num_point_faces;
                (*part).num_line_faces = (*fbx_mesh).num_line_faces;
                (*part).num_triangles = (*fbx_mesh).num_triangles;
                (*part).face_indices.data = SENTINEL_INDEX_CONSECUTIVE.as_ptr();
                (*part).face_indices.count = num_faces;
            }
        }

        // HACK(consecutive-faces): Prepare for finalize to re-use a consecutive/zero
        // index buffer for face materials..
        uc.set_max_zero_indices(max_sz(uc.max_zero_indices(), num_faces));
        uc.set_max_consecutive_indices(max_sz(uc.max_consecutive_indices(), num_faces));

        i -= 1;
    }

    Ok(())
}

// ufbx.c:17681-17761 `ufbxi_obj_parse_file`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_parse_file(uc: &Context) -> Result<(), Fail> {
    while !(*uc.obj().get()).eof {
        obj_tokenize_line(uc)?;
        let num_tokens: usize = (*uc.obj().get()).num_tokens;
        if num_tokens == 0 {
            continue;
        }

        let cmd: String = *(*uc.obj().get()).tokens.add(0);
        let key: u32 = get_name_key(cmd.data, cmd.length);
        if key == obj_cmd1(b'v') {
            obj_parse_vertex(uc, ObjAttrib::Position, 1)?;
            if num_tokens >= 7 {
                let num_vertices: usize =
                    uc.obj().vertex_count_at(ObjAttrib::Position as usize).get();
                (*uc.obj().get()).has_vertex_color = true;
                obj_pad_colors(uc, num_vertices.wrapping_sub(1))?;
                if uc.obj().vertex_count_at(ObjAttrib::Color as usize).get() < num_vertices {
                    ufbx_assert!(
                        uc.obj().vertex_count_at(ObjAttrib::Color as usize).get()
                            == num_vertices - 1
                    );
                    obj_parse_vertex(uc, ObjAttrib::Color, 4)?;
                    let valid: *mut bool = push::<bool>(&mut (*uc.obj().get()).tmp_color_valid, 1);
                    ufbxi_check!(uc, !valid.is_null(), "valid");
                    *valid = true;
                }
            }
        } else if key == obj_cmd2(b'v', b't') {
            obj_parse_vertex(uc, ObjAttrib::Uv, 1)?;
        } else if key == obj_cmd2(b'v', b'n') {
            obj_parse_vertex(uc, ObjAttrib::Normal, 1)?;
        } else if key == obj_cmd1(b'f') {
            obj_parse_indices(uc, 1, (*uc.obj().get()).num_tokens - 1)?;
        } else if key == obj_cmd1(b'p') {
            obj_parse_multi_indices(uc, 1)?;
        } else if key == obj_cmd1(b'l') {
            obj_parse_multi_indices(uc, 2)?;
        } else if key == obj_cmd1(b's') {
            if num_tokens >= 2 {
                (*uc.obj().get()).has_face_smoothing = true;
                uc.obj().set_face_smoothing(!str_equal(
                    *(*uc.obj().get()).tokens.add(1),
                    str_c(b"off\0".as_ptr()),
                ));

                // Fill in previously missed face smoothing data
                if (*uc.obj().get()).tmp_face_smoothing.num_items == 0
                    && (*uc.obj().get()).tmp_faces.num_items > 0
                {
                    ufbxi_check!(
                        uc,
                        !push_zero::<bool>(
                            &mut (*uc.obj().get()).tmp_face_smoothing,
                            (*uc.obj().get()).tmp_faces.num_items
                        )
                        .is_null(),
                        "((bool*)ufbxi_push_size_zero((&uc->obj.tmp_face_smoothing), sizeof(bool), (uc->obj.tmp_faces.num_items)))"
                    );
                }
            }
        } else if key == obj_cmd1(b'o') {
            if num_tokens >= 2 {
                (*uc.obj().get()).object = obj_span_token(uc, 1, usize::MAX);
                push_string_place_str(
                    uc.string_pool_mut_ptr(),
                    &mut (*uc.obj().get()).object,
                    false,
                )?;
                (*uc.obj().get()).object_dirty = true;
            }
        } else if key == obj_cmd1(b'g') {
            if num_tokens >= 2 {
                (*uc.obj().get()).group = obj_span_token(uc, 1, usize::MAX);
                push_string_place_str(
                    uc.string_pool_mut_ptr(),
                    &mut (*uc.obj().get()).group,
                    false,
                )?;
                (*uc.obj().get()).group_dirty = true;
            } else {
                (*uc.obj().get()).group = EMPTY_STRING.0;
                (*uc.obj().get()).group_dirty = true;
            }
        } else if key == obj_cmd1(b'#') {
            obj_parse_comment(uc)?;
        } else if str_equal(cmd, str_c(b"mtllib\0".as_ptr())) {
            ufbxi_check!(
                uc,
                (*uc.obj().get()).num_tokens >= 2,
                "uc->obj.num_tokens >= 2"
            );
            let mut lib: String = obj_span_token(uc, 1, usize::MAX);
            lib.data = push_copy::<u8>(uc.tmp_mut_ptr(), lib.length + 1, lib.data);
            ufbxi_check!(uc, !lib.data.is_null(), "lib.data");
            (*uc.obj().get()).mtllib_relative_path.data = lib.data;
            (*uc.obj().get()).mtllib_relative_path.size = lib.length;
        } else if str_equal(cmd, str_c(b"usemtl\0".as_ptr())) {
            obj_parse_material(uc)?;
        } else if !uc.opts_view().disable_quirks() && key == 0 {
            // ZBrush exporter seems to end the files with '\0', sometimes..
        } else {
            ufbxi_check!(
                uc,
                ufbxi_warnf!(
                    uc,
                    WarningType::UnknownObjDirective,
                    "Unknown .obj directive, skipped line"
                )
                .is_ok(),
                "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_UNKNOWN_OBJ_DIRECTIVE, ~0u, \"Unknown .obj directive, skipped line\")"
            );
        }
    }

    obj_flush_mesh(uc)?;
    obj_pop_meshes(uc)?;

    Ok(())
}

// ufbx.c:17763-17775 `ufbxi_obj_flush_material`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_flush_material(uc: &Context) -> Result<(), Fail> {
    if (*uc.obj().get()).usemtl_fbx_id == 0 {
        return Ok(());
    }

    let entry: *mut FbxIdEntry = find_fbx_id(uc, (*uc.obj().get()).usemtl_fbx_id);
    ufbx_assert!(!entry.is_null());
    let material: *mut Material = *(*uc.obj().get())
        .tmp_materials
        .add((*entry).element_id as usize);

    let num_props: usize = (*uc.obj().get()).tmp_props.num_items;
    obj_pop_props(uc, &mut (*material).element.props.props, num_props)?;

    Ok(())
}

// ufbx.c:17777-17850 `ufbxi_obj_parse_prop`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_parse_prop(
    uc: &Context,
    name: String,
    start: usize,
    include_rest: bool,
    p_next: *mut usize,
) -> Result<(), Fail> {
    if start >= (*uc.obj().get()).num_tokens {
        if !p_next.is_null() {
            *p_next = start;
        }
        return Ok(());
    }

    let prop: *mut Prop = push_zero::<Prop>(&mut (*uc.obj().get()).tmp_props, 1);
    ufbxi_check!(uc, !prop.is_null(), "prop");
    (*prop).name = name;

    push_string_place_str(uc.string_pool_mut_ptr(), &mut (*prop).name, false)?;

    let mut flags: u32 = PropFlags::VALUE_STR.raw();

    // C-parity: `prop->value_real_arr[]` is the `ufbx_prop` value union's
    // 4-real view (ufbx.h); the generated struct keeps only the `value_vec4`
    // member (PORTING.md union table).
    let value_real_arr: *mut Real = &mut (*prop).value_vec4 as *mut Vec4 as *mut Real;

    let mut num_reals: usize = 0;
    while num_reals < 4 {
        if start + num_reals >= (*uc.obj().get()).num_tokens {
            break;
        }
        let tok: String = *(*uc.obj().get()).tokens.add(start + num_reals);

        // C: `char *end; // ufbxi_uninit`
        let mut end: *const u8 = core::ptr::null(); // ufbxi_uninit
        let val: f64 = parse_double(tok.data, tok.length, &mut end, uc.double_parse_flags());
        if end != tok.data.add(tok.length) {
            break;
        }

        *value_real_arr.add(num_reals) = val as Real;
        if num_reals == 0 {
            (*prop).value_int = f64_to_i64(val);
            flags |= PropFlags::VALUE_INT.raw();
        }

        num_reals += 1;
    }

    let mut num_args: usize = 0;
    if !include_rest {
        while start + num_args < (*uc.obj().get()).num_tokens - 1 {
            if r#match(
                (*uc.obj().get()).tokens.add(start + num_args),
                b"-[A-Za-z][\\-A-Za-z0-9_]*\0".as_ptr(),
            ) {
                break;
            }
            num_args += 1;
        }
    }

    if num_args > 0 || include_rest {
        let span: String = obj_span_token(
            uc,
            start,
            if include_rest {
                usize::MAX
            } else {
                start + num_args - 1
            },
        );
        (*prop).value_str = span;
        (*prop).value_blob.data = span.data;
        (*prop).value_blob.size = span.length;

        push_string_place_str(uc.string_pool_mut_ptr(), &mut (*prop).value_str, false)?;
        push_string_place_blob(uc.string_pool_mut_ptr(), &mut (*prop).value_blob, true)?;
    } else {
        (*prop).value_str.data = EMPTY_CHAR.as_ptr();
    }

    if num_reals > 0 {
        flags = PropFlags::VALUE_REAL.raw() << (num_reals - 1);
    } else {
        if strcmp((*prop).value_str.data, b"on\0".as_ptr()) == 0 {
            (*prop).value_int = 1;
            // C: `prop->value_real = 1.0f;` — the first `ufbx_real` of the
            // value union (`value_vec4.x` in the generated struct).
            (*prop).value_vec4.x = 1.0f32 as Real;
            flags |= PropFlags::VALUE_INT.raw();
        } else if strcmp((*prop).value_str.data, b"off\0".as_ptr()) == 0 {
            (*prop).value_int = 0;
            (*prop).value_vec4.x = 0.0f32 as Real;
            flags |= PropFlags::VALUE_INT.raw();
        }
    }

    (*prop).flags = PropFlags::from_raw(flags);

    if !p_next.is_null() {
        *p_next = start + num_args;
    }

    Ok(())
}

// ufbx.c:17852-17902 `ufbxi_obj_parse_mtl_map`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_parse_mtl_map(uc: &Context, prefix_len: usize) -> Result<(), Fail> {
    if (*uc.obj().get()).num_tokens < 2 {
        return Ok(());
    }

    let mut num_props: usize = 1;
    obj_parse_prop(
        uc,
        str_c(b"obj|args\0".as_ptr()),
        1,
        true,
        core::ptr::null_mut(),
    )?;

    let mut start: usize = 1;
    // C: `for (; start + 1 < uc->obj.num_tokens; )`
    while start + 1 < (*uc.obj().get()).num_tokens {
        let mut tok: String = *(*uc.obj().get()).tokens.add(start);
        if r#match(&tok, b"-[A-Za-z][\\-A-Za-z0-9_]*\0".as_ptr()) {
            tok.data = tok.data.add(1);
            tok.length -= 1;
            obj_parse_prop(uc, tok, start + 1, false, &mut start)?;
            num_props += 1;
        } else {
            break;
        }
    }

    let mut tex_str: String = obj_span_token(uc, start, usize::MAX);
    let mut tex_raw: Blob = Blob::new_c(tex_str.data, tex_str.length);

    push_string_place_str(uc.string_pool_mut_ptr(), &mut tex_str, false)?;
    push_string_place_blob(uc.string_pool_mut_ptr(), &mut tex_raw, true)?;

    let mut fbx_id: u64 = 0;
    let texture: *mut Texture = push_synthetic_element::<Texture>(
        uc,
        &mut fbx_id,
        core::ptr::null_mut(),
        b"\0".as_ptr(),
        ElementType::Texture,
    );
    ufbxi_check!(uc, !texture.is_null(), "texture");

    (*texture).filename.data = EMPTY_CHAR.as_ptr();
    (*texture).absolute_filename.data = EMPTY_CHAR.as_ptr();
    (*texture).uv_set.data = EMPTY_CHAR.as_ptr();

    (*texture).relative_filename = tex_str;
    (*texture).raw_relative_filename = tex_raw;

    obj_pop_props(uc, &mut (*texture).element.props.props, num_props)?;

    let mut prop: String = *(*uc.obj().get()).tokens.add(0);
    ufbx_assert!(prop.length >= prefix_len);
    prop.data = prop.data.add(prefix_len);
    prop.length -= prefix_len;
    push_string_place_str(uc.string_pool_mut_ptr(), &mut prop, false)?;

    if (*uc.obj().get()).usemtl_fbx_id != 0 {
        connect_op(uc, fbx_id, (*uc.obj().get()).usemtl_fbx_id, prop)?;
    }

    Ok(())
}

// ufbx.c:17904-17934 `ufbxi_obj_parse_mtl`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_parse_mtl(uc: &Context) -> Result<(), Fail> {
    (*uc.obj().get()).mesh = core::ptr::null_mut();
    (*uc.obj().get()).usemtl_fbx_id = 0;

    while !(*uc.obj().get()).eof {
        obj_tokenize_line(uc)?;
        let num_tokens: usize = (*uc.obj().get()).num_tokens;
        if num_tokens == 0 {
            continue;
        }

        let cmd: String = *(*uc.obj().get()).tokens.add(0);
        if str_equal(cmd, str_c(b"newmtl\0".as_ptr())) {
            // HACK: Reuse mesh material parsing, but don't allow for empty material name
            ufbxi_check!(
                uc,
                (*uc.obj().get()).num_tokens >= 2,
                "uc->obj.num_tokens >= 2"
            );
            obj_flush_material(uc)?;
            obj_parse_material(uc)?;
        } else if cmd.length > 4 && memcmp(cmd.data, b"map_".as_ptr(), 4) == 0 {
            obj_parse_mtl_map(uc, 4)?;
        } else if cmd.length == 4
            && (memcmp(cmd.data, b"bump".as_ptr(), 4) == 0
                || memcmp(cmd.data, b"disp".as_ptr(), 4) == 0
                || memcmp(cmd.data, b"norm".as_ptr(), 4) == 0)
        {
            obj_parse_mtl_map(uc, 0)?;
        } else if cmd.length == 1 && *cmd.data.add(0) == b'#' {
            // Implement .mtl magic comment handling here if necessary
        } else {
            obj_parse_prop(
                uc,
                *(*uc.obj().get()).tokens.add(0),
                1,
                true,
                core::ptr::null_mut(),
            )?;
        }
    }

    obj_flush_material(uc)?;

    Ok(())
}

// ufbx.c:17936-18027 `ufbxi_obj_load_mtl`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_load_mtl(uc: &Context) -> Result<(), Fail> {
    // HACK: Reset everything and switch to loading the .mtl file globally
    if let Some(close_fn) = uc.close_fn() {
        close_fn(uc.read_user());
    }

    uc.set_read_fn(None);
    uc.set_close_fn(None);
    uc.set_read_user(core::ptr::null_mut());
    uc.set_data_begin(core::ptr::null());
    uc.set_data(core::ptr::null());
    uc.set_data_size(0);
    uc.set_yield_size(0);
    uc.set_eof(false);
    (*uc.obj().get()).eof = false;

    if uc.opts_view().obj_mtl_data_view().size() > 0 {
        uc.set_data(uc.opts_view().obj_mtl_data_view().data());
        uc.set_data_begin(uc.data());
        uc.set_data_size(uc.opts_view().obj_mtl_data_view().size());
        obj_parse_mtl(uc)?;
        return Ok(());
    }

    // C: `ufbx_stream stream = { 0 };`
    let mut stream: RawStream = core::mem::zeroed();
    let mut has_stream: bool = false;
    let mut needs_stream: bool = false;
    // C: `ufbx_blob stream_path = { 0 };`
    let mut stream_path: Blob = core::mem::zeroed();

    if uc.opts_view().open_file_cb_view().fn_().is_some() {
        if uc.opts_view().obj_mtl_path_view().length() > 0 {
            has_stream = open_file(
                uc.opts_view().open_file_cb_ptr(),
                &mut stream,
                uc.opts_view().obj_mtl_path_view().data(),
                uc.opts_view().obj_mtl_path_view().length(),
                core::ptr::null(),
                uc.ator_tmp_mut_ptr(),
                OpenFileType::ObjMtl,
            );
            stream_path.data = uc.opts_view().obj_mtl_path_view().data();
            stream_path.size = uc.opts_view().obj_mtl_path_view().length();
            needs_stream = true;
            if !has_stream {
                ufbxi_check!(
                    uc,
                    ufbxi_warnf!(
                        uc,
                        WarningType::MissingExternalFile,
                        "Could not open .mtl file: %s",
                        uc.opts_view().obj_mtl_path_view().data()
                    )
                    .is_ok(),
                    "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_MISSING_EXTERNAL_FILE, ~0u, \"Could not open .mtl file: %s\", uc->opts.obj_mtl_path.data)"
                );
            }
        }

        if !has_stream
            && uc.opts_view().load_external_files()
            && (*uc.obj().get()).mtllib_relative_path.size > 0
        {
            // C: `ufbx_blob dst; // ufbxi_uninit`
            let mut dst = MaybeUninit::<Blob>::uninit(); // ufbxi_uninit
            let dst: *mut Blob = dst.as_mut_ptr();
            resolve_relative_filename(
                uc,
                dst as *mut Strblob,
                &raw const (*uc.obj().get()).mtllib_relative_path as *const Strblob,
                true,
            )?;
            has_stream = open_file(
                uc.opts_view().open_file_cb_ptr(),
                &mut stream,
                (*dst).data,
                (*dst).size,
                &(*uc.obj().get()).mtllib_relative_path,
                uc.ator_tmp_mut_ptr(),
                OpenFileType::ObjMtl,
            );
            stream_path = (*uc.obj().get()).mtllib_relative_path;
            needs_stream = true;
            if !has_stream {
                ufbxi_check!(
                    uc,
                    ufbxi_warnf!(
                        uc,
                        WarningType::MissingExternalFile,
                        "Could not open .mtl file: %s",
                        (*dst).data
                    )
                    .is_ok(),
                    "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_MISSING_EXTERNAL_FILE, ~0u, \"Could not open .mtl file: %s\", dst.data)"
                );
            }
        }

        let path: String = (*uc.get()).scene.metadata.filename;
        if !has_stream
            && uc.opts_view().load_external_files()
            && uc.opts_view().obj_search_mtl_by_filename()
            && path.length > 4
        {
            // C: `ufbx_string ext = { path.data + path.length - 4, 4 };`
            let ext: String = String::new_c(path.data.add(path.length - 4), 4);
            if r#match(&ext, b"\\c.obj\0".as_ptr()) {
                ufbxi_analysis_assert!(path.length < usize::MAX - 1);
                let copy: *mut u8 = push_copy::<u8>(uc.tmp_mut_ptr(), path.length + 1, path.data);
                ufbxi_check!(uc, !copy.is_null(), "copy");
                *copy.add(path.length - 3) = if *copy.add(path.length - 3) == b'O' {
                    b'M'
                } else {
                    b'm'
                };
                *copy.add(path.length - 2) = if *copy.add(path.length - 2) == b'B' {
                    b'T'
                } else {
                    b't'
                };
                *copy.add(path.length - 1) = if *copy.add(path.length - 1) == b'J' {
                    b'L'
                } else {
                    b'l'
                };
                has_stream = open_file(
                    uc.opts_view().open_file_cb_ptr(),
                    &mut stream,
                    copy,
                    path.length,
                    core::ptr::null(),
                    uc.ator_tmp_mut_ptr(),
                    OpenFileType::ObjMtl,
                );
                if has_stream {
                    ufbxi_check!(
                        uc,
                        ufbxi_warnf!(
                            uc,
                            WarningType::ImplicitMtl,
                            "Opened .mtl file derived from .obj filename: %s",
                            copy as *const u8
                        )
                        .is_ok(),
                        "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_IMPLICIT_MTL, ~0u, \"Opened .mtl file derived from .obj filename: %s\", copy)"
                    );
                }
            }
        }
    }

    if has_stream {
        // Adopt `stream` to ufbx read callbacks
        uc.set_read_fn(stream.read_fn);
        uc.set_close_fn(stream.close_fn);
        uc.set_read_user(stream.user);

        let ok: Result<(), Fail> = obj_parse_mtl(uc);

        if let Some(close_fn) = uc.close_fn() {
            close_fn(uc.read_user());
        }
        uc.set_read_fn(None);
        uc.set_close_fn(None);
        uc.set_read_user(core::ptr::null_mut());

        ok?;
    } else if needs_stream && !uc.opts_view().ignore_missing_external_files() {
        set_err_info(uc.error_mut_ptr(), stream_path.data, stream_path.size);
        ufbxi_fail_msg!(uc, "ufbxi_obj_load_mtl()", "External file not found");
    }

    Ok(())
}

// ufbx.c:18029-18037 `ufbxi_obj_load`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn obj_load(uc: &Context) -> Result<(), Fail> {
    obj_init(uc)?;
    obj_parse_file(uc)?;
    init_file_paths(uc)?;
    obj_load_mtl(uc)?;

    Ok(())
}

// ufbx.c:18039-18046 `ufbxi_mtl_load`
#[cfg(feature = "obj")]
#[inline(never)]
#[must_use]
pub(crate) unsafe fn mtl_load(uc: &Context) -> Result<(), Fail> {
    obj_init(uc)?;
    init_file_paths(uc)?;
    obj_parse_mtl(uc)?;

    Ok(())
}

// CONTINUATION POINT: the `// -- .obj file` banner section (ufbx.c:16767-18065)
// is COMPLETE — both the `#if UFBXI_FEATURE_FORMAT_OBJ` branch and the `#else`
// stubs below. Next banner: ufbx.c:18066 `// -- Scene pre-processing`
// (owned by `native/scene_process.rs`).

// ufbx.c:18049-18053 `ufbxi_obj_load` (`#else` branch — feature disabled)
#[cfg(not(feature = "obj"))]
#[inline(always)]
#[must_use]
pub(crate) unsafe fn obj_load(uc: &Context) -> Result<(), Fail> {
    ufbxi_fmt_err_info!(uc.error_ptr(), "UFBX_ENABLE_FORMAT_OBJ");
    ufbxi_fail_msg!(uc, "UFBXI_FEATURE_FORMAT_OBJ", "Feature disabled");
}

// ufbx.c:18055-18059 `ufbxi_mtl_load` (`#else` branch — feature disabled)
#[cfg(not(feature = "obj"))]
#[inline(always)]
#[must_use]
pub(crate) unsafe fn mtl_load(uc: &Context) -> Result<(), Fail> {
    ufbxi_fmt_err_info!(uc.error_ptr(), "UFBX_ENABLE_FORMAT_OBJ");
    ufbxi_fail_msg!(uc, "UFBXI_FEATURE_FORMAT_OBJ", "Feature disabled");
}

// ufbx.c:18061-18063 `ufbxi_obj_free` (`#else` branch — feature disabled)
#[cfg(not(feature = "obj"))]
#[inline(always)]
pub(crate) unsafe fn obj_free(uc: &Context) {
    let _ = uc;
}
