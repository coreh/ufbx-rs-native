//! Port of the `// -- .obj file` banner section (ufbx.c:16767-18065).
//!
//! Coverage: ufbx.c:16767-17167 — the attribute stride table, the property
//! pop/dedup helper, the `ufbxi_obj_mesh` push/flush pair, the OBJ context
//! init/free, the line reader with its continuation handling, the tokenizer
//! and the vertex / index token parsers. The `#else` (feature-disabled) entry
//! points at ufbx.c:18048-18064 are ported here as well so the module's
//! contract is complete in both feature configurations.
//!
//! Coverage: ufbx.c:17168-17497 — the face/index line reader
//! (`ufbxi_obj_parse_indices` with its mesh flushing, `usemtl` material
//! binding and face-group dedup), the sliding-window multi-index form, the
//! hex digit parser, `usemtl` material creation, the `ufbxi_obj_cmdN` command
//! packers and the vertex/index finalizers (`ufbxi_obj_pop_vertices`,
//! `ufbxi_obj_setup_attrib`, `ufbxi_obj_pad_colors`).
//!
//! Coverage: ufbx.c:17326-17365 (`ufbxi_obj_parse_comment`, unblocked by the
//! `ufbxi_match` family landing in `native/parse.rs`) and ufbx.c:17498-18046 —
//! the mesh finalizer (`ufbxi_obj_pop_meshes`), the `.obj` directive loop
//! (`ufbxi_obj_parse_file`), the `.mtl` parser (`ufbxi_obj_flush_material`,
//! `ufbxi_obj_parse_prop`, `ufbxi_obj_parse_mtl_map`, `ufbxi_obj_parse_mtl`),
//! the `.mtl` locator/loader (`ufbxi_obj_load_mtl`) and the two entry points
//! (`ufbxi_obj_load`, `ufbxi_mtl_load`).
//!
//! The whole section is gated on `UFBXI_FEATURE_FORMAT_OBJ`
//! (`#[cfg(feature = "obj")]`).
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
#[cfg(feature = "obj")]
use crate::generated::{
    ElementType, Face, FaceGroup, Material, Mesh, MeshPart, Node as UfbxNode, OpenFileType, Prop,
    PropFlags, RawStream, ShaderType, Texture, Vec4, VertexAttrib, VoidList, WarningType,
};
#[cfg(feature = "obj")]
use crate::native::allocator::{free, grow_array};
#[cfg(feature = "obj")]
use crate::native::api::EMPTY_STRING;
#[cfg(feature = "obj")]
use crate::native::buf::{buf_free, pop, BufView};
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
use crate::native::hash::{hash_ptr, map_cmp_const_char_ptr, map_free, map_init};
#[cfg(feature = "obj")]
use crate::native::io::refill;
#[cfg(feature = "obj")]
use crate::native::parse::{
    get_name_key, r#match, report_progress, Context, ElementInfo, FbxIdEntry, ObjAttrib,
    ObjFastIndices, ObjGroupEntry, ObjMesh, OBJ_NUM_ATTRIBS, OBJ_NUM_ATTRIBS_EXT,
};
// The `#else`-branch stubs still take `&Context`.
#[cfg(not(feature = "obj"))]
use crate::native::parse::Context;
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
use crate::native::scene_process::MaterialView;
#[cfg(feature = "obj")]
use crate::native::string_pool::{push_string_place_blob, push_string_place_str, str_c, str_equal};
#[cfg(feature = "obj")]
use crate::native::view::{view_read, view_write};
#[cfg(feature = "obj")]
use crate::native::view::{Mut, Run, SliceViewIter, View};
#[cfg(feature = "obj")]
use crate::native::warnings::ufbxi_warnf;
#[cfg(feature = "obj")]
use crate::prelude::as_f64;
#[cfg(feature = "obj")]
use crate::prelude::{Blob, BlobView, List, ListView, Real, ScalarView, String, StringView};
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

// Reinterpret-in-place view over an arena-allocated `ObjMesh` (the obj parser's
// own per-mesh scratch struct, living in `uc.obj().tmp_meshes` / popped into
// `uc.tmp`). Threaded through the mesh-navigation cluster (`obj_parse_index`,
// `obj_setup_attrib`) and minted at the roots (`uc.obj().mesh()` in
// `obj_parse_indices`; the popped `meshes` run in `obj_pop_meshes`) so the
// leaf ops read/write mesh fields through anchored accessors instead of raw
// `(*mesh).field` navigation. `ObjMesh` is `Copy`-field-only scratch with
// interior mutability via the view's `UnsafeCell`, so a shared `&ObjMeshView`
// may coexist with `&Context`/`&ObjContext` (distinct arenas), never forming
// a `&ObjMesh`.
#[cfg(feature = "obj")]
pub(crate) type ObjMeshView = View<ObjMesh>;

#[cfg(feature = "obj")]
impl ObjMeshView {
    #[inline(always)]
    pub(crate) fn num_faces(&self) -> usize {
        view_read!(self, num_faces)
    }
    #[inline(always)]
    pub(crate) fn set_num_faces(&self, v: usize) {
        view_write!(self, num_faces, v)
    }
    #[inline(always)]
    pub(crate) fn num_indices(&self) -> usize {
        view_read!(self, num_indices)
    }
    #[inline(always)]
    pub(crate) fn set_num_indices(&self, v: usize) {
        view_write!(self, num_indices, v)
    }
    #[inline(always)]
    pub(crate) fn num_groups(&self) -> u32 {
        view_read!(self, num_groups)
    }
    #[inline(always)]
    pub(crate) fn set_num_groups(&self, v: u32) {
        view_write!(self, num_groups, v)
    }
    #[inline(always)]
    pub(crate) fn usemtl_base(&self) -> u32 {
        view_read!(self, usemtl_base)
    }
    #[inline(always)]
    pub(crate) fn set_usemtl_base(&self, v: u32) {
        view_write!(self, usemtl_base, v)
    }
    #[inline(always)]
    pub(crate) fn fbx_mesh(&self) -> *mut Mesh {
        view_read!(self, fbx_mesh)
    }
    #[inline(always)]
    pub(crate) fn fbx_node(&self) -> *mut UfbxNode {
        view_read!(self, fbx_node)
    }
    #[inline(always)]
    pub(crate) fn fbx_node_id(&self) -> u64 {
        view_read!(self, fbx_node_id)
    }
    #[inline(always)]
    pub(crate) fn fbx_mesh_id(&self) -> u64 {
        view_read!(self, fbx_mesh_id)
    }
    #[inline(always)]
    pub(crate) fn vertex_range_min(&self, attrib: usize) -> u64 {
        // SAFETY: `attrib < OBJ_NUM_ATTRIBS` (caller invariant); reads a scalar
        // of the fixed-size `vertex_range` array of a valid arena `ObjMesh`.
        unsafe { (*self.get()).vertex_range[attrib].min_ix }
    }
    #[inline(always)]
    pub(crate) fn vertex_range_max(&self, attrib: usize) -> u64 {
        // SAFETY: as `vertex_range_min`.
        unsafe { (*self.get()).vertex_range[attrib].max_ix }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_range_min(&self, attrib: usize, v: u64) {
        // SAFETY: as `vertex_range_min`; scalar store.
        unsafe {
            (*self.get()).vertex_range[attrib].min_ix = v;
        }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_range_max(&self, attrib: usize, v: u64) {
        // SAFETY: as `vertex_range_min`; scalar store.
        unsafe {
            (*self.get()).vertex_range[attrib].max_ix = v;
        }
    }
}

// ufbx.c:16777-16805 `ufbxi_obj_pop_props`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) unsafe fn obj_pop_props(
    uc: &Context,
    dst: *mut List<Prop>,
    count: usize,
) -> Result<(), Fail> {
    // C: `ufbx_prop_list props; // ufbxi_uninit`
    // SAFETY: `List<Prop>` is plain C data whose all-zero bit pattern is the
    // valid `{ 0 }` initializer.
    let mut props: List<Prop> = unsafe { core::mem::zeroed() }; // ufbxi_uninit
    props.count = count;
    props.data = uc
        .result_view()
        .push_pop::<Prop>(uc.obj().tmp_props_view(), count);
    ufbxi_check!(uc, !props.data.is_null(), "props.data");

    // C: `ufbxi_for_list(ufbx_prop, prop, props)`
    // SAFETY: `props.data` is the fresh non-null run popped above, holding
    // `props.count` contiguous `Prop` on uc's result arena (write-capable
    // provenance), so the whole walk stays inside that one allocation.
    let prop_run =
        unsafe { SliceViewIter::<Prop>::from_raw_parts(props.data as *mut Prop, props.count) };
    for prop in prop_run {
        prop.set_internal_key(get_name_key(prop.name_view().bytes()));

        let mut value_str: String = prop.value_str();
        if value_str.length == 0 {
            value_str.data = EMPTY_CHAR.as_ptr();
            prop.set_value_str(value_str);
        }

        if prop.value_int() == 0 {
            // C: `prop->value_real` — the first `ufbx_real` of the value
            // union (`value_vec4.x` in the generated struct).
            prop.set_value_int(f64_to_i64(as_f64!(prop.value_vec4().x)));
        }

        let mut value_blob: Blob = prop.value_blob();
        if value_blob.size == 0 && value_str.length > 0 {
            value_blob.data = value_str.data;
            value_blob.size = value_str.length;
            prop.set_value_blob(value_blob);
        }
    }

    if props.count > 1 {
        // `props` is the local descriptor of the run just sorted, which the
        // dedup compacts in place.
        let props_view = ListView::<Prop>::from_mut(&mut props);
        sort_properties(uc, Run::from_list(props_view))?;
        deduplicate_properties(props_view);
    }

    // C: `*dst = props;`
    // SAFETY: caller contract — `dst` is a writable `List<Prop>` out-param.
    unsafe { core::ptr::write(dst, props) };
    Ok(())
}

// ufbx.c:16807-16843 `ufbxi_obj_push_mesh`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_push_mesh(uc: &Context) -> Result<(), Fail> {
    let mesh: *mut ObjMesh = uc.obj().tmp_meshes_view().push_zero::<ObjMesh>(1);
    ufbxi_check!(uc, !mesh.is_null(), "mesh");
    uc.obj().set_mesh(mesh);
    // SAFETY: `mesh` is the fresh non-null push result above; anchoring it as a
    // view puts the field access below on the accessor path.
    let mesh_view: &ObjMeshView = unsafe { ObjMeshView::from_ptr(mesh) };

    // C: `ufbxi_nounroll for (size_t i = 0; i < UFBXI_OBJ_NUM_ATTRIBS; i++)`
    for i in 0..OBJ_NUM_ATTRIBS {
        // `i < OBJ_NUM_ATTRIBS` bounds the fixed `vertex_range` array.
        mesh_view.set_vertex_range_min(i, u64::MAX);
    }

    // C: `const char *name = "";`
    let mut name: *const u8 = b"\0".as_ptr();
    if uc.opts_view().obj_split_groups() && uc.obj().group_view().length() > 0 {
        name = uc.obj().group_view().data();
    } else if !uc.opts_view().obj_merge_objects() && uc.obj().object_view().length() > 0 {
        name = uc.obj().object_view().data();
    } else if !uc.opts_view().obj_merge_groups() && uc.obj().group_view().length() > 0 {
        name = uc.obj().group_view().data();
    }

    // SAFETY: `mesh` is the fresh push result, so its own `fbx_node_id` /
    // `fbx_mesh_id` fields are unaliased out-params for the two element pushes,
    // each viewed for its call as an interior-mutable scalar cell —
    // `ScalarView<u64>` is `repr(transparent)` over `u64`.
    unsafe {
        (*mesh).fbx_node = push_synthetic_element::<UfbxNode>(
            uc,
            &*(&raw mut (*mesh).fbx_node_id as *const ScalarView<u64>),
            None,
            name,
            ElementType::Node,
        );
        (*mesh).fbx_mesh = push_synthetic_element::<Mesh>(
            uc,
            &*(&raw mut (*mesh).fbx_mesh_id as *const ScalarView<u64>),
            None,
            name,
            ElementType::Mesh,
        );
    }
    ufbxi_check!(
        uc,
        !mesh_view.fbx_node().is_null() && !mesh_view.fbx_mesh().is_null(),
        "mesh->fbx_node && mesh->fbx_mesh"
    );

    // SAFETY: `mesh->fbx_mesh` is the element push result, non-null past the
    // check above, living in uc's own element arena (write-capable provenance).
    let fbx_mesh: &View<Mesh> = unsafe { View::<Mesh>::from_ptr(mesh_view.fbx_mesh()) };
    fbx_mesh.vertex_position().set_unique_per_vertex(true);

    ufbxi_check!(
        uc,
        // Copies the fresh node element's own `element_id` field onto uc's
        // `tmp_node_ids` arena.
        // SAFETY: `fbx_node` is non-null past the check above, so borrowing its
        // `element_id` field is valid.
        !uc.tmp_node_ids_view()
            .push_copy_ref(unsafe { &(*mesh_view.fbx_node()).element.element_id })
            .is_null(),
        "((uint32_t*)ufbxi_push_size_copy((&uc->tmp_node_ids), sizeof(uint32_t), (1), (&mesh->fbx_node->element_id)))"
    );

    uc.obj().set_face_material(NO_INDEX);
    uc.obj().set_face_group(0);
    uc.obj().set_face_group_dirty(true);
    uc.obj().set_material_dirty(true);

    // Connects the fresh mesh's own synthetic ids (read through the anchored
    // view) in uc's connection arena.
    connect_oo(uc, mesh_view.fbx_mesh_id(), mesh_view.fbx_node_id())?;
    connect_oo(uc, mesh_view.fbx_node_id(), 0)?;

    Ok(())
}

// ufbx.c:16845-16860 `ufbxi_obj_flush_mesh`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_flush_mesh(uc: &Context) -> Result<(), Fail> {
    if uc.obj().mesh().is_null() {
        return Ok(());
    }

    // SAFETY: the current mesh is a live `tmp_meshes` entry, non-null past the
    // guard above; anchoring it as a view resolves `fbx_mesh` through the
    // accessor.
    let mesh: &ObjMeshView = unsafe { ObjMeshView::from_ptr(uc.obj().mesh()) };
    // SAFETY: `mesh->fbx_mesh` is the mesh's own element (non-null since
    // `obj_push_mesh`) in uc's element arena (write-capable provenance).
    let fbx_mesh: &View<Mesh> = unsafe { View::<Mesh>::from_ptr(mesh.fbx_mesh()) };

    let num_props: usize = uc.obj().tmp_props_view().num_items();
    // SAFETY: the mesh element's own prop list is an unaliased destination.
    unsafe { obj_pop_props(uc, fbx_mesh.element().props().props_raw(), num_props)? };

    let num_groups: usize = uc.obj().tmp_face_group_infos_view().num_items();
    // Pops the obj parser's own `tmp_face_group_infos` arena into uc's result
    // arena.
    let groups: *mut FaceGroup = uc
        .result_view()
        .push_pop::<FaceGroup>(uc.obj().tmp_face_group_infos_view(), num_groups);
    ufbxi_check!(uc, !groups.is_null(), "groups");

    // `groups` is the fresh non-null `num_groups` run popped just above.
    fbx_mesh.face_groups_view().set_data(groups);
    fbx_mesh.face_groups_view().set_count(num_groups);

    Ok(())
}

// ufbx.c:16862-16900 `ufbxi_obj_init`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_init(uc: &Context) -> Result<(), Fail> {
    uc.set_from_ascii(true);
    uc.obj().set_initialized(true);

    // C: `ufbxi_nounroll for (size_t i = 0; i < UFBXI_OBJ_NUM_ATTRIBS_EXT; i++)`
    for i in 0..OBJ_NUM_ATTRIBS_EXT {
        uc.obj().tmp_vertices_at(i).set_ator(uc.ator_tmp_mut_ptr());
        uc.obj().tmp_indices_at(i).set_ator(uc.ator_tmp_mut_ptr());
    }
    uc.obj()
        .tmp_color_valid_view()
        .set_ator(uc.ator_tmp_mut_ptr());
    uc.obj().tmp_faces_view().set_ator(uc.ator_tmp_mut_ptr());
    uc.obj()
        .tmp_face_material_view()
        .set_ator(uc.ator_tmp_mut_ptr());
    uc.obj()
        .tmp_face_smoothing_view()
        .set_ator(uc.ator_tmp_mut_ptr());
    uc.obj()
        .tmp_face_group_view()
        .set_ator(uc.ator_tmp_mut_ptr());
    uc.obj()
        .tmp_face_group_infos_view()
        .set_ator(uc.ator_tmp_mut_ptr());
    uc.obj().tmp_meshes_view().set_ator(uc.ator_tmp_mut_ptr());
    uc.obj().tmp_props_view().set_ator(uc.ator_tmp_mut_ptr());

    // .obj parsing does its own yield logic
    uc.set_data_size(uc.data_size() + uc.yield_size());

    uc.obj().object_view().set_data(EMPTY_CHAR.as_ptr());
    uc.obj().group_view().set_data(EMPTY_CHAR.as_ptr());

    // SAFETY: `map_cmp_const_char_ptr` reads no user data, so the null
    // `cmp_user` meets its contract.
    unsafe {
        map_init(
            uc.obj().group_map_view(),
            uc.ator_tmp_view(),
            map_cmp_const_char_ptr,
            core::ptr::null_mut(),
        );
    }

    // Add a nameless root node with the root ID
    {
        // C: `ufbxi_element_info root_info = { uc->root_id };`
        // SAFETY: `ElementInfo` is plain C data whose all-zero bit pattern is
        // the valid `{ 0 }` initializer.
        let mut root_info: ElementInfo = unsafe { core::mem::zeroed() };
        root_info.fbx_id = uc.root_id();
        root_info.name = EMPTY_STRING.0;
        // SAFETY: the push targets uc's own element arenas; `root_info`'s
        // `name` is the static NUL-terminated empty string and its
        // `props`/`dom_node` are zeroed, so the pointers the element stores
        // outlive the scene; `UfbxNode` is the element struct for
        // `ElementType::Node`.
        let root: *mut UfbxNode = unsafe {
            push_element::<UfbxNode>(
                uc,
                View::<ElementInfo>::from_mut(&mut root_info),
                ElementType::Node,
            )
        };
        ufbxi_check!(uc, !root.is_null(), "root");
        // SAFETY: `root` is the fresh non-null element push result, living in
        // uc's own write-capable element arena.
        setup_root_node(uc, unsafe { View::<UfbxNode>::from_ptr(root) });
        ufbxi_check!(
            uc,
            // Copies `root`'s own `element_id` field onto uc's `tmp_node_ids`
            // arena.
            // SAFETY: `root` is the fresh non-null element push result, so
            // borrowing its `element_id` field is valid.
            !uc.tmp_node_ids_view()
                .push_copy_ref(unsafe { &(*root).element.element_id })
                .is_null(),
            "((uint32_t*)ufbxi_push_size_copy((&uc->tmp_node_ids), sizeof(uint32_t), (1), (&root->element.element_id)))"
        );
    }

    Ok(())
}

// ufbx.c:16902-16923 `ufbxi_obj_free`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_free(uc: &Context) {
    if !uc.obj().initialized() {
        return;
    }

    // Releases the obj parser's own buffers and its group map, reached only
    // once the context is initialized (guard above).
    // C: `ufbxi_nounroll for (size_t i = 0; i < UFBXI_OBJ_NUM_ATTRIBS_EXT; i++)`
    for i in 0..OBJ_NUM_ATTRIBS_EXT {
        buf_free(uc.obj().tmp_vertices_at(i));
        buf_free(uc.obj().tmp_indices_at(i));
    }
    buf_free(uc.obj().tmp_color_valid_view());
    buf_free(uc.obj().tmp_faces_view());
    buf_free(uc.obj().tmp_face_material_view());
    buf_free(uc.obj().tmp_face_smoothing_view());
    buf_free(uc.obj().tmp_face_group_view());
    buf_free(uc.obj().tmp_face_group_infos_view());
    buf_free(uc.obj().tmp_meshes_view());
    buf_free(uc.obj().tmp_props_view());

    map_free(uc.obj().group_map_view());

    // SAFETY: each array is freed with the capacity it was grown to, through
    // the same temp allocator that allocated it.
    unsafe {
        free::<String>(
            Some(uc.ator_tmp_view()),
            uc.obj().tokens(),
            uc.obj().tokens_cap(),
        );
        free::<*mut Material>(
            Some(uc.ator_tmp_view()),
            uc.obj().tmp_materials(),
            uc.obj().tmp_materials_cap(),
        );
    }
}

// ufbx.c:16925-16981 `ufbxi_obj_read_line`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_read_line(uc: &Context) -> Result<(), Fail> {
    ufbxi_dev_assert!(!uc.obj().eof());

    let mut offset: usize = 0;

    loop {
        let begin: *const u8 = add_ptr(uc.data() as *mut u8, offset) as *const u8;
        // SAFETY: `begin` is `offset` bytes into uc's read window and `offset`
        // only ever advances to a scanned line end within it, so the remaining
        // `data_size - offset` bytes searched are in bounds.
        let end: *const u8 = if !begin.is_null() {
            unsafe { memchr(begin, b'\n', uc.data_size() - offset) }
        } else {
            core::ptr::null()
        };
        if end.is_null() {
            if uc.eof() {
                offset = uc.data_size();
                uc.obj().set_eof(true);
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
        // SAFETY: `esc` steps back from the newline `memchr` found only while
        // it stays strictly above `begin`, so both byte reads are inside the
        // read window.
        unsafe {
            if esc > begin && *esc.offset(-1) == b'\r' {
                esc = esc.offset(-1);
            }
            if esc > begin && *esc.offset(-1) == b'\\' {
                continue;
            }
        }

        break;
    }

    let line_len: usize = offset;

    uc.obj().line_view().set_data(uc.data());
    uc.obj().line_view().set_length(line_len);
    // SAFETY: `line_len` bytes were just scanned out of uc's read window, so
    // advancing `data` past them stays inside the buffer.
    unsafe { uc.set_data(uc.data().add(line_len)) };
    uc.set_data_size(uc.data_size() - line_len);

    uc.obj()
        .set_read_progress(uc.obj().read_progress() + line_len);
    if uc.obj().read_progress() >= uc.progress_interval() {
        report_progress(uc)?;
        uc.obj()
            .set_read_progress(uc.obj().read_progress() % uc.progress_interval());
    }

    if uc.obj().eof() {
        let new_data: *mut u8 = uc.tmp_view().push::<u8>(line_len + 1);
        ufbxi_check!(uc, !new_data.is_null(), "new_data");
        // SAFETY: `new_data` is the fresh non-null `line_len + 1` byte run,
        // disjoint from the `line_len`-byte source line still in the read
        // window; the two writes fill it exactly.
        unsafe {
            core::ptr::copy_nonoverlapping(uc.obj().line_view().data(), new_data, line_len);
            *new_data.add(line_len) = b'\n';
        }
        uc.obj().line_view().set_data(new_data);
        uc.obj()
            .line_view()
            .set_length(uc.obj().line_view().length() + 1);
    }

    Ok(())
}

// ufbx.c:16983-16997 `ufbxi_obj_span_token`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_span_token(uc: &Context, start_token: usize, end_token: usize) -> String {
    ufbx_assert!(start_token < uc.obj().num_tokens());
    let end_token = min_sz(end_token, uc.obj().num_tokens() - 1);

    ufbx_assert!(start_token <= end_token);
    // SAFETY: both indices are `< num_tokens` (asserted above / clamped into
    // range), so they index the tokenizer's stored token run.
    let (start, end): (String, String) = unsafe {
        (
            *uc.obj().tokens().add(start_token),
            *uc.obj().tokens().add(end_token),
        )
    };
    let num_between: usize = to_size(end.data as isize - start.data as isize);

    // SAFETY: `String` is plain C data whose all-zero bit pattern is the valid
    // `{ 0 }` initializer.
    let mut result: String = unsafe { core::mem::zeroed() };
    result.data = start.data;
    result.length = num_between + end.length;
    result
}

// ufbx.c:16999-17065 `ufbxi_obj_tokenize`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_tokenize(uc: &Context) -> Result<(), Fail> {
    let mut ptr: *const u8 = uc.obj().line_view().data();
    // SAFETY: `line` is the window `obj_read_line` stored on the obj context,
    // so `data + length` is its one-past-the-end.
    let end: *const u8 = unsafe { ptr.add(uc.obj().line_view().length()) };
    uc.obj().set_num_tokens(0);

    loop {
        let mut c: u8;

        // Skip whitespace
        loop {
            // SAFETY: the stored line window always ends in a '\n' sentinel
            // (`obj_read_line` appends one at EOF) and the scan stops there; a
            // '\\' or '\r' therefore sits at least one byte before that
            // sentinel, so both continuation-lookahead reads (`ptr + 1`, and
            // `+ 2` past a '\r') stay inside the window. The `p < end - 1`
            // test only gates accepting the continuation, not the reads.
            unsafe {
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
            }

            break;
        }

        // SAFETY: as above — `ptr` rests on a byte of the stored line window.
        c = unsafe { *ptr };
        if c == b'\n' {
            break;
        }
        if c == b'#' && uc.obj().num_tokens() > 0 {
            break;
        }

        let index: usize = uc.obj().num_tokens();
        uc.obj().set_num_tokens(uc.obj().num_tokens() + 1);
        ufbxi_check!(
            uc,
            // SAFETY: grows the obj context's own paired `tokens` / `tokens_cap`
            // growth state through uc's temp allocator.
            unsafe {
                grow_array::<String>(
                    uc.ator_tmp_view(),
                    uc.obj().tokens_mut_ptr(),
                    uc.obj().tokens_cap_mut_ptr(),
                    index + 1
                )
            },
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->obj.tokens)), (&uc->obj.tokens), (&uc->obj.tokens_cap), (index + 1))"
        );

        // SAFETY: the grow above ensured `tokens` holds at least `index + 1`
        // slots, so slot `index` is in bounds; `ptr` points into the stored
        // line window.
        let tok: *mut String = unsafe {
            let tok: *mut String = uc.obj().tokens().add(index);
            (*tok).data = ptr;
            tok
        };

        // Treat comment start as a single token
        if c == b'#' {
            // SAFETY: `tok` is the slot ensured above; `ptr` rests on the '#'
            // of the line window, so one byte past it is still inside.
            unsafe {
                ptr = ptr.add(1);
                (*tok).length = 1;
            }
            continue;
        }

        // SAFETY: the '\n' sentinel ending the stored line window stops this
        // scan (`is_space('\n')` holds), so every byte read is inside the
        // window; `tok` is the slot ensured above.
        unsafe {
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
    }

    Ok(())
}

// ufbx.c:17067-17072 `ufbxi_obj_tokenize_line`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_tokenize_line(uc: &Context) -> Result<(), Fail> {
    obj_read_line(uc)?;
    obj_tokenize(uc)?;
    Ok(())
}

// ufbx.c:17074-17108 `ufbxi_obj_parse_vertex`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_parse_vertex(uc: &Context, attrib: ObjAttrib, offset: usize) -> Result<(), Fail> {
    if uc.opts_view().ignore_geometry() {
        return Ok(());
    }

    let dst: &BufView = uc.obj().tmp_vertices_at(attrib as usize);
    let num_values: usize = OBJ_ATTRIB_STRIDE[attrib as usize] as usize;
    uc.obj()
        .vertex_count_at(attrib as usize)
        .set(uc.obj().vertex_count_at(attrib as usize).get() + 1);

    let mut read_values: usize = num_values;
    if attrib == ObjAttrib::Color {
        if offset + read_values > uc.obj().num_tokens() {
            read_values = 3;
        }
    }
    ufbxi_check!(
        uc,
        offset + read_values <= uc.obj().num_tokens(),
        "offset + read_values <= uc->obj.num_tokens"
    );

    let parse_flags: u32 = uc.double_parse_flags();
    let vals: *mut Real = dst.push_fast::<Real>(num_values);
    ufbxi_check!(uc, !vals.is_null(), "vals");
    for i in 0..read_values {
        // SAFETY: `offset + read_values <= num_tokens` (checked above), so
        // token `offset + i` is in the stored token run and `str_.data ..
        // + length` is that token's own span; `end` is an unaliased local
        // out-param; `i < read_values <= num_values` indexes the fresh push.
        unsafe {
            let str_: String = *uc.obj().tokens().add(offset + i);
            // C: `char *end; // ufbxi_uninit`
            let mut end: *const u8 = core::ptr::null(); // ufbxi_uninit
            let val: f64 = parse_double(str_.data, str_.length, &raw mut end, parse_flags);
            ufbxi_check!(
                uc,
                end == str_.data.add(str_.length),
                "end == str.data + str.length"
            );
            *vals.add(i) = val as Real;
        }
    }

    if read_values < num_values {
        ufbx_assert!(read_values + 1 == num_values);
        ufbx_assert!(attrib == ObjAttrib::Color);
        // C: `vals[read_values] = 1.0f;`
        // SAFETY: `read_values + 1 == num_values` here (asserted above), so the
        // slot is the last of the fresh push.
        unsafe { *vals.add(read_values) = 1.0f32 as Real };
    }

    Ok(())
}

// ufbx.c:17110-17166 `ufbxi_obj_parse_index`
#[cfg(feature = "obj")]
#[inline(never)]
/// # Safety
///
/// `s` holds a token span (possibly empty) at token index >= 1 within the line
/// window `obj_tokenize` scanned, so `data .. data + length` is readable and so
/// is the byte *at* `end`: `obj_tokenize` terminates every non-'#' token on a
/// delimiter byte inside that window (in the worst case the '\n' sentinel
/// `obj_read_line` appends), and a '#' token — the one kind that ends on an
/// arbitrary byte — is only ever produced at index 0, so none reaches here.
pub(crate) unsafe fn obj_parse_index(
    uc: &Context,
    mesh: &ObjMeshView,
    s: &mut String,
    attrib: u32,
) -> Result<(), Fail> {
    // SAFETY: the span `s` describes is readable per the fn contract, so `end`
    // is one past its last byte and still within the window. The `'/'` rebasing
    // below only shrinks a span from the front, keeping the same `end`.
    let (mut ptr, end) = (s.data, unsafe { s.data.add(s.length) });

    let mut negative: bool = false;
    // SAFETY: `ptr` is either inside the span or equal to `end`; both are
    // readable per the window invariant above. (Callers do reach here with an
    // empty span: `obj_parse_faces` runs all `OBJ_NUM_ATTRIBS` attributes over
    // one token that this function rebases in place, so a position-only token
    // like "3" is exhausted after the first attribute.) The byte at `end` is a
    // delimiter, never `'-'`, so the advance below stays within the span.
    if unsafe { *ptr } == b'-' {
        negative = true;
        // SAFETY: the byte just read is `'-'`, so it is a span byte rather than
        // the delimiter at `end`; advancing lands at most on `end`.
        ptr = unsafe { ptr.add(1) };
    }

    // As .obj indices are never zero we can detect missing indices
    // by simply not writing to it.
    let mut index: u64 = 0;
    while ptr != end {
        // SAFETY: `ptr != end` (loop condition), so it rests on a byte of the
        // token span.
        let c: u8 = unsafe { *ptr };
        if c >= b'0' && c <= b'9' {
            ufbxi_check!(
                uc,
                index < u64::MAX / 10 - 10,
                "index < UINT64_MAX / 10 - 10"
            );
            index = index * 10 + (c as i32 - b'0' as i32) as u64;
        } else if c == b'/' {
            // SAFETY: `ptr != end` here, so advancing lands at most on `end`.
            ptr = unsafe { ptr.add(1) };
            break;
        }
        // SAFETY: `ptr != end` here, so advancing lands at most on `end`.
        ptr = unsafe { ptr.add(1) };
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
    // SAFETY: `fast_indices` is the obj context's own per-attribute writer
    // state, taken through its raw-ptr getter.
    if unsafe { (*fast_indices).num_left } == 0 {
        let num_push: usize = 128;
        let dst: *mut u64 = uc
            .obj()
            .tmp_indices_at(attrib as usize)
            .push::<u64>(num_push);
        ufbxi_check!(uc, !dst.is_null(), "dst");
        uc.obj().fast_indices_at(attrib as usize).set_indices(dst);
        uc.obj()
            .fast_indices_at(attrib as usize)
            .set_num_left(num_push);
    }

    // C: `*fast_indices->indices++ = index;`
    // SAFETY: the writer state is the obj context's own (as above), and its
    // `indices` cursor has `num_left > 0` slots of the reserved `tmp_indices`
    // run ahead of it — the refill above restores that when it hits zero — so
    // the store and the one-slot advance stay inside that run.
    unsafe {
        *(*fast_indices).indices = index;
        (*fast_indices).indices = (*fast_indices).indices.add(1);
    }
    // SAFETY: the writer state is the obj context's own; `num_left > 0` here.
    unsafe { (*fast_indices).num_left -= 1 };

    if index != u64::MAX {
        let a: usize = attrib as usize;
        mesh.set_vertex_range_min(a, min64(mesh.vertex_range_min(a), index));
        mesh.set_vertex_range_max(a, max64(mesh.vertex_range_max(a), index));
    }

    // `ptr` rests inside `s`'s own span at or before `end`, so the rebased span
    // is a suffix of the original.
    s.data = ptr;
    s.length = to_size(end as isize - ptr as isize);

    Ok(())
}

// ufbx.c:17168-17296 `ufbxi_obj_parse_indices`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_parse_indices(
    uc: &Context,
    token_begin: usize,
    num_tokens: usize,
) -> Result<(), Fail> {
    let mut flush_mesh: bool = false;
    if uc.obj().object_dirty() {
        if !uc.opts_view().obj_merge_objects() {
            flush_mesh = true;
        }
        uc.obj().set_object_dirty(false);
    }

    if uc.obj().group_dirty() {
        if ((uc.obj().object_view().length() == 0 || uc.opts_view().obj_merge_objects())
            && !uc.opts_view().obj_merge_groups())
            || uc.opts_view().obj_split_groups()
        {
            flush_mesh = true;
        }
        uc.obj().set_group_dirty(false);
        uc.obj().set_face_group_dirty(true);
    }

    if uc.obj().mesh().is_null() || flush_mesh {
        obj_flush_mesh(uc)?;
        obj_push_mesh(uc)?;
    }
    // Anchor the current-mesh view at the ObjContext root; thread it into the
    // per-index leaf (`obj_parse_index`) below.
    // SAFETY: the current mesh is a live `tmp_meshes` entry, non-null after the
    // push above.
    let mesh: &ObjMeshView = unsafe { ObjMeshView::from_ptr(uc.obj().mesh()) };

    if uc.obj().material_dirty() {
        if uc.obj().usemtl_fbx_id() != 0 {
            let entry: *mut FbxIdEntry = find_fbx_id(uc, uc.obj().usemtl_fbx_id());
            ufbx_assert!(!entry.is_null());
            // SAFETY: `entry` is uc's own fbx-id map entry for the active
            // `usemtl` id, non-null past the assert; the connection targets
            // uc's connection arena and the mesh reads go through the anchored
            // view.
            unsafe {
                if mesh.usemtl_base() == 0 || (*entry).user_id < mesh.usemtl_base() {
                    connect_oo(uc, uc.obj().usemtl_fbx_id(), mesh.fbx_node_id())?;

                    // C: `uint32_t index = ++uc->obj.usemtl_index;`
                    uc.obj()
                        .set_usemtl_index(uc.obj().usemtl_index().wrapping_add(1));
                    let index: u32 = uc.obj().usemtl_index();
                    ufbxi_check!(uc, index < u32::MAX, "index < UINT32_MAX");
                    (*entry).user_id = index;

                    if mesh.usemtl_base() == 0 {
                        mesh.set_usemtl_base(index);
                    }
                    uc.obj()
                        .set_face_material(index.wrapping_sub(mesh.usemtl_base()));
                }
                // C-parity: the assignment above is immediately overwritten
                // here; both are in the C source and both are kept.
                uc.obj()
                    .set_face_material((*entry).user_id.wrapping_sub(mesh.usemtl_base()));
            }
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
        if uc.obj().group_view().length() > 0
            && (uc.obj().object_view().length() > 0 || uc.opts_view().obj_merge_groups())
            && !uc.opts_view().obj_split_groups()
        {
            name = uc.obj().group();
        }

        let hash: u32 = hash_ptr!(name.data);
        // SAFETY: looks the interned name pointer up in the obj parser's own
        // group map (keyed by that pointer, whose address is taken from an
        // unaliased local).
        let mut entry: *mut ObjGroupEntry = uc.obj().group_map_view().find(hash, &name.data);
        if entry.is_null() {
            entry = uc.obj().group_map_view().insert(hash, &name.data);
            ufbxi_check!(uc, !entry.is_null(), "entry");
            // SAFETY: `entry` is the fresh non-null insert result.
            unsafe {
                (*entry).name = name.data;
                (*entry).mesh_id = 0;
                (*entry).local_id = 0;
            }
        }

        // SAFETY: `mesh.fbx_mesh()` is the mesh's own element (non-null since
        // `obj_push_mesh`) in uc's element arena (write-capable provenance).
        let fbx_mesh: &View<Mesh> = unsafe { View::<Mesh>::from_ptr(mesh.fbx_mesh()) };
        let mesh_id: u32 = fbx_mesh.element().element_id();

        // SAFETY: `entry` is the group-map entry found or inserted above;
        // `group` is the fresh result of the arena push, checked non-null, and
        // lives on the obj parser's own `tmp_face_group_infos` arena.
        unsafe {
            if (*entry).mesh_id != mesh_id {
                // C: `uint32_t id = mesh->num_groups++;`
                let id: u32 = mesh.num_groups();
                mesh.set_num_groups(mesh.num_groups().wrapping_add(1));
                (*entry).mesh_id = mesh_id;
                (*entry).local_id = id;

                let group: *mut FaceGroup = uc
                    .obj()
                    .tmp_face_group_infos_view()
                    .push_zero::<FaceGroup>(1);
                ufbxi_check!(uc, !group.is_null(), "group");
                let group: &View<FaceGroup> = View::<FaceGroup>::from_ptr(group);
                group.set_id(0);
                group.set_name(name);
            }

            uc.obj().set_face_group((*entry).local_id);
        }

        if !uc.obj().has_face_group() {
            uc.obj().set_has_face_group(true);
            ufbxi_check!(
                uc,
                // Zero-fills one face-group slot per already-recorded face on
                // the obj parser's own `tmp_face_group` arena.
                !uc.obj()
                    .tmp_face_group_view()
                    .push_zero::<u32>(uc.obj().tmp_faces_view().num_items())
                    .is_null(),
                "((uint32_t*)ufbxi_push_size_zero((&uc->obj.tmp_face_group), sizeof(uint32_t), (uc->obj.tmp_faces.num_items)))"
            );
        }

        uc.obj().set_face_group_dirty(false);
    }

    let num_indices: usize = num_tokens;
    ufbxi_check!(
        uc,
        (u32::MAX as usize).wrapping_sub(mesh.num_indices()) >= num_indices,
        "UINT32_MAX - mesh->num_indices >= num_indices"
    );

    let face: *mut Face = uc.obj().tmp_faces_view().push_fast::<Face>(1);
    ufbxi_check!(uc, !face.is_null(), "face");

    // SAFETY: `face` is the fresh non-null push result on the obj parser's own
    // `tmp_faces` arena (write-capable provenance).
    let face: &View<Face> = unsafe { View::<Face>::from_ptr(face) };
    face.set_index_begin(mesh.num_indices() as u32);
    face.set_num_indices(num_indices as u32);

    mesh.set_num_faces(mesh.num_faces() + 1);
    mesh.set_num_indices(mesh.num_indices() + num_indices);

    let p_face_mat: *mut u32 = uc.obj().tmp_face_material_view().push_fast::<u32>(1);
    ufbxi_check!(uc, !p_face_mat.is_null(), "p_face_mat");
    // SAFETY: fresh push result, non-null past the check.
    unsafe { *p_face_mat = uc.obj().face_material() };

    if uc.obj().has_face_smoothing() {
        let p_face_smooth: *mut bool = uc.obj().tmp_face_smoothing_view().push_fast::<bool>(1);
        ufbxi_check!(uc, !p_face_smooth.is_null(), "p_face_smooth");
        // SAFETY: fresh push result, non-null past the check.
        unsafe { *p_face_smooth = uc.obj().face_smoothing() };
    }

    if uc.obj().has_face_group() {
        let p_face_group: *mut u32 = uc.obj().tmp_face_group_view().push_fast::<u32>(1);
        ufbxi_check!(uc, !p_face_group.is_null(), "p_face_group");
        // SAFETY: fresh push result, non-null past the check.
        unsafe { *p_face_group = uc.obj().face_group() };
    }

    for ix in 0..num_indices {
        // SAFETY: `token_begin + ix` is inside the caller's token window (its
        // end is bounded by `num_tokens`), so it indexes the stored token run;
        // `tok` is an unaliased local copy of such a token — a span at token
        // index >= 1 within the line window, which is `obj_parse_index`'s
        // contract — that the per-attribute parser advances, and `mesh` is the
        // anchored current mesh.
        unsafe {
            let mut tok: String = *uc.obj().tokens().add(token_begin + ix);
            for attrib in 0..OBJ_NUM_ATTRIBS as u32 {
                obj_parse_index(uc, mesh, &mut tok, attrib)?;
            }
        }
    }

    Ok(())
}

// ufbx.c:17298-17304 `ufbxi_obj_parse_multi_indices`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_parse_multi_indices(uc: &Context, window: usize) -> Result<(), Fail> {
    // C: `for (size_t begin = 1; begin + window <= uc->obj.num_tokens; begin++)`
    let mut begin: usize = 1;
    while begin + window <= uc.obj().num_tokens() {
        // `begin + window <= num_tokens` (loop condition) keeps the window in
        // range of the tokenizer's stored token run.
        obj_parse_indices(uc, begin, window)?;
        begin += 1;
    }
    Ok(())
}

// ufbx.c:17306-17324 `ufbxi_parse_hex`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn parse_hex(digits: &[u8]) -> u32 {
    let mut value: u32 = 0;

    for &c in digits {
        // C: `char c = digits[i];` — `char` is signed on the oracle targets
        // (PORTING.md char-value rule). Every range tested below is entirely
        // below 0x80, so bytes >= 0x80 fall through to `v = 0` either way.
        let c: i8 = c as i8;
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
pub(crate) fn obj_parse_comment(uc: &Context) -> Result<(), Fail> {
    // SAFETY: the length guard runs first, so token 1 is in the tokenizer's
    // stored token run; the literal is NUL-terminated for `str_c`.
    if uc.obj().num_tokens() >= 3
        && unsafe { str_equal((*uc.obj().tokens().add(1)).as_bytes(), b"MRGB") }
    {
        let num_color: usize = uc.obj().vertex_count_at(ObjAttrib::Color as usize).get();

        // Pop standard vertex colors and replace them with MRGB colors
        if num_color > uc.obj().mrgb_vertex_count() {
            let num_pop: usize = num_color - uc.obj().mrgb_vertex_count();
            // SAFETY: discards `num_pop` entries from the obj parser's own
            // color arenas (`num_pop` is the surplus over the MRGB count, so
            // the arenas hold at least that much); null destination drops them.
            unsafe {
                pop::<bool>(
                    uc.obj().tmp_color_valid_view(),
                    num_pop,
                    core::ptr::null_mut(),
                );
                pop::<Real>(
                    uc.obj().tmp_vertices_at(ObjAttrib::Color as usize),
                    num_pop * 4,
                    core::ptr::null_mut(),
                );
            }
            uc.obj()
                .vertex_count_at(ObjAttrib::Color as usize)
                .set(uc.obj().vertex_count_at(ObjAttrib::Color as usize).get() - num_pop);
        }

        // SAFETY: `num_tokens >= 3`, so token 2 is in the stored token run.
        let mrgb: String = unsafe { *uc.obj().tokens().add(2) };
        // SAFETY: the token is an interned OBJ input string, readable and
        // unwritten for its stored length throughout this parse.
        let mrgb_bytes = unsafe { mrgb.as_bytes() };
        // C: `for (size_t i = 0; i + 8 <= mrgb.length; i += 8)`
        let mut i: usize = 0;
        while i + 8 <= mrgb.length {
            let p_rgba: *mut Real = uc
                .obj()
                .tmp_vertices_at(ObjAttrib::Color as usize)
                .push::<Real>(4);
            let p_valid: *mut bool = uc.obj().tmp_color_valid_view().push::<bool>(1);
            ufbxi_check!(
                uc,
                !p_rgba.is_null() && !p_valid.is_null(),
                "p_rgba && p_valid"
            );
            // SAFETY: fills the fresh non-null runs pushed above exactly.
            unsafe {
                *p_valid = true;

                let hex: u32 = parse_hex(&mrgb_bytes[i..i + 8]);
                *p_rgba.add(0) = ((hex >> 16) & 0xff) as Real / (255.0f32 as Real);
                *p_rgba.add(1) = ((hex >> 8) & 0xff) as Real / (255.0f32 as Real);
                *p_rgba.add(2) = ((hex >> 0) & 0xff) as Real / (255.0f32 as Real);
                *p_rgba.add(3) = ((hex >> 24) & 0xff) as Real / (255.0f32 as Real);
            }

            i += 8;
        }

        uc.obj().set_has_vertex_color(true);
    }

    if !uc.opts_view().disable_quirks() {
        // SAFETY: the pattern literal is NUL-terminated — `r#match`'s contract.
        if unsafe {
            r#match(
                uc.obj().line_view().bytes(),
                b"\\s*#\\s*File exported by ZBrush.*\0".as_ptr(),
            )
        } {
            if uc.obj().mesh().is_null() {
                uc.opts_view().set_obj_merge_groups(true);
            }
        }
    }

    Ok(())
}

// ufbx.c:17367-17406 `ufbxi_obj_parse_material`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_parse_material(uc: &Context) -> Result<(), Fail> {
    uc.obj().set_material_dirty(true);

    // Allow empty `usemtl` lines to specify "no material".
    if uc.obj().num_tokens() < 2 {
        uc.obj().set_usemtl_fbx_id(0);
        return Ok(());
    }

    let mut name: String = obj_span_token(uc, 1, usize::MAX);

    // Interns the span through an unaliased local.
    push_string_place_str(
        uc.string_pool_view(),
        StringView::from_mut(&mut name),
        false,
    )?;
    // Derives the synthetic id from the interned (pool-owned, NUL-terminated)
    // pointer.
    let fbx_id: u64 = synthetic_id_from_string(uc, name.data);
    ufbxi_check!(uc, fbx_id != 0, "fbx_id");

    let entry: *mut FbxIdEntry = find_fbx_id(uc, fbx_id);

    uc.obj().set_usemtl_fbx_id(fbx_id);

    if entry.is_null() {
        // C: `ufbxi_element_info info = { 0 };`
        // SAFETY: `ElementInfo` is plain C data whose all-zero bit pattern is
        // the valid `{ 0 }` initializer.
        let mut info: ElementInfo = unsafe { core::mem::zeroed() };
        info.fbx_id = fbx_id;
        info.name = name;

        // SAFETY: the element push targets uc's own element arenas; `info`'s
        // `name` is the pooled NUL-terminated material name and its
        // `props`/`dom_node` are zeroed, so the pointers the element stores
        // outlive the scene; `Material` is the element struct for
        // `ElementType::Material`.
        let material: *mut Material = unsafe {
            push_element::<Material>(
                uc,
                View::<ElementInfo>::from_mut(&mut info),
                ElementType::Material,
            )
        };
        ufbxi_check!(uc, !material.is_null(), "material");

        // SAFETY: `material` is the fresh non-null element push result.
        let id: usize = unsafe {
            (*material).shader_type = ShaderType::WavefrontMtl;
            (*material).shading_model_name.data = EMPTY_CHAR.as_ptr();
            (*material).shader_prop_prefix.data = EMPTY_CHAR.as_ptr();
            (*material).element.element_id as usize
        };
        ufbxi_check!(
            uc,
            // SAFETY: grows the obj context's own paired `tmp_materials` /
            // `tmp_materials_cap` growth state through uc's temp allocator.
            unsafe {
                grow_array::<*mut Material>(
                    uc.ator_tmp_view(),
                    uc.obj().tmp_materials_mut_ptr(),
                    uc.obj().tmp_materials_cap_mut_ptr(),
                    id + 1
                )
            },
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->obj.tmp_materials)), (&uc->obj.tmp_materials), (&uc->obj.tmp_materials_cap), (id + 1))"
        );
        // SAFETY: the grow above ensured slot `id` exists in `tmp_materials`.
        unsafe { *uc.obj().tmp_materials().add(id) = material };
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
// C-parity: the `ufbxi_obj_cmd3` macro has zero call sites in ufbx.c (no
// dispatched OBJ keyword needs three packed bytes); kept with the other macros.
#[allow(dead_code)]
#[cfg(feature = "obj")]
#[inline(always)]
pub(crate) const fn obj_cmd3(a: u8, b: u8, c: u8) -> u32 {
    (a as u32) << 24 | (b as u32) << 16 | (c as u32) << 8
}

// ufbx.c:17412-17432 `ufbxi_obj_pop_vertices`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_pop_vertices(
    uc: &Context,
    dst: &mut List<Real>,
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
    let mut data: *mut Real = uc.result_view().push::<Real>(count + 4);
    ufbxi_check!(uc, !data.is_null(), "data");

    // SAFETY: `data` is the fresh non-null `count + 4` element run, so its
    // first four slots are the padding quad written here and the run holds
    // `count` more elements past that quad.
    unsafe {
        *data.add(0) = 0.0f32 as Real;
        *data.add(1) = 0.0f32 as Real;
        *data.add(2) = 0.0f32 as Real;
        *data.add(3) = 0.0f32 as Real;
        data = data.add(4);
    }

    // SAFETY: pops `count` items off this attribute's own `tmp_vertices` arena
    // (`count` is that arena's item count above `min_index`, computed above)
    // into the `count`-element tail of the fresh run.
    unsafe { pop::<Real>(uc.obj().tmp_vertices_at(attrib as usize), count, data) };

    dst.data = data;
    dst.count = count;
    Ok(())
}

// ufbx.c:17434-17481 `ufbxi_obj_setup_attrib`
//
// # Safety
// `tmp_indices` must address a writable `u64` run holding at least
// `mesh->num_indices` elements (C's caller-sized scratch run) — a run length
// the parameter types cannot carry.
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) unsafe fn obj_setup_attrib(
    uc: &Context,
    mesh: &ObjMeshView,
    tmp_indices: *mut u64,
    dst: &View<VertexAttrib>,
    p_data: &List<Real>,
    attrib: u32,
    non_disjoint: bool,
    required: bool,
) -> Result<(), Fail> {
    // C: `ufbx_real_list data = *p_data;`
    // SAFETY: `p_data` is a live borrow of a `List<Real>`, and `List` is the
    // plain `{ data, count }` descriptor with no `Drop`, so the bitwise copy
    // out leaves the source valid (C struct assignment is memcpy).
    let data: List<Real> = unsafe { core::ptr::read(p_data) };

    let num_indices: usize = mesh.num_indices();
    let stride: usize = OBJ_ATTRIB_STRIDE[attrib as usize] as usize;
    let num_values: usize = data.count / stride;

    let mesh_min_ix: u64 = mesh.vertex_range_min(attrib as usize);
    if num_indices == 0 || num_values == 0 || mesh_min_ix == u64::MAX {
        ufbxi_check!(
            uc,
            num_indices == 0 || !required,
            "num_indices == 0 || !required"
        );

        // Pop indices without copying if the attribute is not used
        // SAFETY: discards this mesh's `num_indices` entries from the
        // attribute's own `tmp_indices` arena; a null destination drops them.
        unsafe {
            pop::<u64>(
                uc.obj().tmp_indices_at(attrib as usize),
                num_indices,
                core::ptr::null_mut(),
            );
        }
        return Ok(());
    }

    let min_index: u64 = if non_disjoint { 0 } else { mesh_min_ix };

    // SAFETY: pops this mesh's `num_indices` entries off the attribute's own
    // `tmp_indices` arena into `tmp_indices`, the caller-supplied scratch run
    // sized for the widest mesh.
    unsafe {
        pop::<u64>(
            uc.obj().tmp_indices_at(attrib as usize),
            num_indices,
            tmp_indices,
        );
    }

    let dst_indices: *mut u32 = uc.result_view().push::<u32>(num_indices);
    ufbxi_check!(uc, !dst_indices.is_null(), "dst_indices");

    // `data` is the value run the caller popped for this attribute and
    // `dst_indices` the fresh non-null `num_indices` run pushed above.
    dst.set_exists(true);

    let mut values: VoidList = dst.values();
    values.data = data.data as *mut c_void;
    values.count = num_values;
    dst.set_values(values);

    dst.indices_view().set_data(dst_indices);
    dst.indices_view().set_count(num_indices);

    // C: `ufbxi_nounroll for (size_t i = 0; i < num_indices; i++)`
    for i in 0..num_indices {
        // SAFETY: `i < num_indices`, the item count both the scratch run
        // (filled by the pop above) and the fresh `dst_indices` run were sized
        // for.
        let mut ix: u64 = unsafe { *tmp_indices.add(i) };
        if ix != u64::MAX {
            ix = ix.wrapping_sub(min_index);
            ufbxi_check!(uc, ix < u32::MAX as u64, "ix < UINT32_MAX");
        }
        if ix < num_values as u64 {
            // SAFETY: `i < num_indices` bounds the fresh run, as above.
            unsafe { *dst_indices.add(i) = ix as u32 };
        } else {
            // SAFETY: as above — the slot handed to the fixer is `dst_indices`
            // element `i`: live and write-capable, an adequate mint for the
            // `Mut` index-slot view (the slot is still uninitialized, which
            // `Mut` storage tolerates).
            let p_dst: &View<u32> = unsafe { View::<u32, Mut>::from_ptr(dst_indices.add(i)) };
            fix_index(uc, p_dst, ix as u32, num_values)?;
        }
    }

    Ok(())
}

// ufbx.c:17483-17496 `ufbxi_obj_pad_colors`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_pad_colors(uc: &Context, num_vertices: usize) -> Result<(), Fail> {
    if uc.opts_view().ignore_geometry() {
        return Ok(());
    }

    let num_colors: usize = uc.obj().vertex_count_at(ObjAttrib::Color as usize).get();
    if num_vertices > num_colors {
        let num_pad: usize = num_vertices - num_colors;
        ufbxi_check!(
            uc,
            // Zero-fills the padding quads on the obj parser's own color
            // vertex arena.
            !uc.obj()
                .tmp_vertices_at(ObjAttrib::Color as usize)
                .push_zero::<Real>(num_pad * 4)
                .is_null(),
            "((ufbx_real*)ufbxi_push_size_zero((&uc->obj.tmp_vertices[UFBXI_OBJ_ATTRIB_COLOR]), sizeof(ufbx_real), (num_pad * 4)))"
        );
        ufbxi_check!(
            uc,
            // Matching validity flags on the obj parser's own `tmp_color_valid`
            // arena.
            !uc.obj()
                .tmp_color_valid_view()
                .push_zero::<bool>(num_pad)
                .is_null(),
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
pub(crate) fn obj_pop_meshes(uc: &Context) -> Result<(), Fail> {
    let num_meshes: usize = uc.obj().tmp_meshes_view().num_items();
    // Moves the obj parser's whole `tmp_meshes` run (its own item count) onto
    // uc's tmp arena.
    let meshes: *mut ObjMesh = uc
        .tmp_view()
        .push_pop::<ObjMesh>(uc.obj().tmp_meshes_view(), num_meshes);
    ufbxi_check!(uc, !meshes.is_null(), "meshes");

    if uc.obj().has_vertex_color() {
        obj_pad_colors(
            uc,
            uc.obj().vertex_count_at(ObjAttrib::Position as usize).get(),
        )?;
    }

    // Pop unused fast indices
    for i in 0..OBJ_NUM_ATTRIBS {
        // SAFETY: discards the tail the fast-index writer reserved but never
        // filled (`num_left` of the attribute's own `tmp_indices` arena).
        unsafe {
            pop::<u64>(
                uc.obj().tmp_indices_at(i),
                uc.obj().fast_indices_at(i).num_left(),
                core::ptr::null_mut(),
            );
        }
    }

    // Check if the file has disjoint vertices
    let mut non_disjoint: [bool; OBJ_NUM_ATTRIBS] = [false; OBJ_NUM_ATTRIBS];
    let mut next_min: [u64; OBJ_NUM_ATTRIBS] = [0; OBJ_NUM_ATTRIBS];
    // C: `ufbx_real_list vertices[UFBXI_OBJ_NUM_ATTRIBS_EXT] = { 0 };`
    // SAFETY: `List<Real>` is plain C data whose all-zero bit pattern is the
    // valid `{ 0 }` initializer.
    let mut vertices: [List<Real>; OBJ_NUM_ATTRIBS_EXT] = unsafe { core::mem::zeroed() };
    let mut color_valid: *mut bool = core::ptr::null_mut();

    let mut max_indices: usize = 0;

    // Walk the popped `meshes` run as anchored views (contiguous `push_pop`).
    // SAFETY: `meshes` is the fresh non-null `num_meshes`-item run popped
    // above, so the whole walk stays inside that one allocation.
    for mesh in unsafe { SliceViewIter::<ObjMesh>::from_raw_parts(meshes, num_meshes) } {
        max_indices = max_sz(max_indices, mesh.num_indices());
        // C: `ufbxi_nounroll for (uint32_t attrib = 0; attrib < UFBXI_OBJ_NUM_ATTRIBS; attrib++)`
        for attrib in 0..OBJ_NUM_ATTRIBS {
            let min_ix: u64 = mesh.vertex_range_min(attrib);
            let max_ix: u64 = mesh.vertex_range_max(attrib);
            if min_ix > max_ix {
                continue;
            }
            if min_ix < next_min[attrib] {
                non_disjoint[attrib] = true;
            }
            next_min[attrib] = max_ix.wrapping_add(1);
        }
    }

    // Scratch run for the widest mesh, pushed onto uc's own tmp arena.
    let tmp_indices: *mut u64 = uc.tmp_view().push::<u64>(max_indices);
    ufbxi_check!(uc, !tmp_indices.is_null(), "tmp_indices");

    // C: `ufbxi_nounroll for (uint32_t attrib = 0; attrib < UFBXI_OBJ_NUM_ATTRIBS; attrib++)`
    for attrib in 0..OBJ_NUM_ATTRIBS {
        if !non_disjoint[attrib] {
            continue;
        }
        obj_pop_vertices(uc, &mut vertices[attrib], attrib as u32, 0)?;
    }
    if uc.obj().has_vertex_color() && non_disjoint[ObjAttrib::Position as usize] {
        obj_pop_vertices(
            uc,
            &mut vertices[ObjAttrib::Color as usize],
            ObjAttrib::Color as u32,
            0,
        )?;
        // The `color_valid` pop moves one flag per popped color quad off the
        // obj parser's own arena.
        color_valid = uc.tmp_view().push_pop::<bool>(
            uc.obj().tmp_color_valid_view(),
            vertices[ObjAttrib::Color as usize].count / 4,
        );
        ufbxi_check!(uc, !color_valid.is_null(), "color_valid");
    }

    // C: `for (size_t i = num_meshes; i > 0; i--)`
    let mut i: usize = num_meshes;
    while i > 0 {
        // SAFETY: `i - 1 < num_meshes`, so this indexes the popped `meshes`
        // run (one allocation) that anchors the view.
        let mesh: &ObjMeshView = unsafe { ObjMeshView::from_ptr(meshes.add(i - 1)) };

        // SAFETY: `mesh->fbx_mesh` is this mesh's own `ufbx_mesh` element in
        // uc's element arena (non-null since `obj_push_mesh`), so its
        // provenance is write-capable; anchoring it once puts every field
        // access below on the accessor path.
        let fbx_mesh: &View<Mesh> = unsafe { View::<Mesh>::from_ptr(mesh.fbx_mesh()) };

        let num_faces: usize = mesh.num_faces();

        if !uc.opts_view().ignore_geometry() {
            // C: `ufbxi_nounroll for (uint32_t attrib = 0; attrib < UFBXI_OBJ_NUM_ATTRIBS; attrib++)`
            for attrib in 0..OBJ_NUM_ATTRIBS {
                if non_disjoint[attrib] {
                    continue;
                }
                let min_ix: u64 = mesh.vertex_range_min(attrib);
                if min_ix < u64::MAX {
                    obj_pop_vertices(uc, &mut vertices[attrib], attrib as u32, min_ix)?;
                }
            }
            if uc.obj().has_vertex_color() && !non_disjoint[ObjAttrib::Position as usize] {
                let min_ix: u64 = mesh.vertex_range_min(ObjAttrib::Position as usize);
                ufbxi_check!(uc, min_ix < u64::MAX, "min_ix < UINT64_MAX");
                obj_pop_vertices(
                    uc,
                    &mut vertices[ObjAttrib::Color as usize],
                    ObjAttrib::Color as u32,
                    min_ix,
                )?;
                // The `color_valid` pop moves one flag per popped color quad
                // off the obj parser's own arena.
                color_valid = uc.tmp_view().push_pop::<bool>(
                    uc.obj().tmp_color_valid_view(),
                    vertices[ObjAttrib::Color as usize].count / 4,
                );
                ufbxi_check!(uc, !color_valid.is_null(), "color_valid");
            }

            // Each list `fbx_mesh` is given here is the run popped for it out
            // of the matching obj-parser arena, checked non-null right after.
            fbx_mesh.faces_view().set_count(num_faces);
            fbx_mesh.face_material_view().set_count(num_faces);

            fbx_mesh.faces_view().set_data(
                uc.result_view()
                    .push_pop::<Face>(uc.obj().tmp_faces_view(), num_faces),
            );
            fbx_mesh.face_material_view().set_data(
                uc.result_view()
                    .push_pop::<u32>(uc.obj().tmp_face_material_view(), num_faces),
            );

            ufbxi_check!(
                uc,
                !fbx_mesh.faces_view().data().is_null(),
                "fbx_mesh->faces.data"
            );
            ufbxi_check!(
                uc,
                !fbx_mesh.face_material_view().data().is_null(),
                "fbx_mesh->face_material.data"
            );

            if uc.obj().has_face_smoothing() {
                fbx_mesh.face_smoothing_view().set_count(num_faces);
                fbx_mesh.face_smoothing_view().set_data(
                    uc.result_view()
                        .push_pop::<bool>(uc.obj().tmp_face_smoothing_view(), num_faces),
                );
                ufbxi_check!(
                    uc,
                    !fbx_mesh.face_smoothing_view().data().is_null(),
                    "fbx_mesh->face_smoothing.data"
                );
            }

            if uc.obj().has_face_group() {
                if mesh.num_groups() > 1 {
                    fbx_mesh.face_group_view().set_count(num_faces);
                    fbx_mesh.face_group_view().set_data(
                        uc.result_view()
                            .push_pop::<u32>(uc.obj().tmp_face_group_view(), num_faces),
                    );
                    ufbxi_check!(
                        uc,
                        !fbx_mesh.face_group_view().data().is_null(),
                        "fbx_mesh->face_group.data"
                    );
                } else {
                    // SAFETY: the discarding pop takes this mesh's own faces
                    // off the obj parser's `tmp_face_group` arena.
                    unsafe {
                        pop::<u32>(
                            uc.obj().tmp_face_group_view(),
                            num_faces,
                            core::ptr::null_mut(),
                        );
                    }
                }
            }

            // SAFETY: `obj_setup_attrib` is an `unsafe fn` taking the raw
            // `tmp_indices` scratch run, sized for the widest mesh (its
            // contract). Each attribute view is minted over a distinct
            // vertex-attribute field of `fbx_mesh`, live and write-capable
            // through this mesh element, reinterpreted onto the shared
            // `ufbx_vertex_attrib` layout prefix (C's cast).
            unsafe {
                obj_setup_attrib(
                    uc,
                    mesh,
                    tmp_indices,
                    View::<VertexAttrib>::from_ptr(
                        fbx_mesh.vertex_position_raw() as *mut VertexAttrib
                    ),
                    &vertices[ObjAttrib::Position as usize],
                    ObjAttrib::Position as u32,
                    non_disjoint[ObjAttrib::Position as usize],
                    true,
                )?;

                obj_setup_attrib(
                    uc,
                    mesh,
                    tmp_indices,
                    View::<VertexAttrib>::from_ptr(fbx_mesh.vertex_uv_raw() as *mut VertexAttrib),
                    &vertices[ObjAttrib::Uv as usize],
                    ObjAttrib::Uv as u32,
                    non_disjoint[ObjAttrib::Uv as usize],
                    false,
                )?;

                obj_setup_attrib(
                    uc,
                    mesh,
                    tmp_indices,
                    View::<VertexAttrib>::from_ptr(
                        fbx_mesh.vertex_normal_raw() as *mut VertexAttrib
                    ),
                    &vertices[ObjAttrib::Normal as usize],
                    ObjAttrib::Normal as u32,
                    non_disjoint[ObjAttrib::Normal as usize],
                    false,
                )?;
            }

            if uc.obj().has_vertex_color() {
                ufbx_assert!(!color_valid.is_null());
                // SAFETY: both index walks run over `data .. data + count` of a
                // list this function just populated (the position indices, then
                // the fresh `push_copy` of them). `color_valid` holds at least
                // `max_index` flags for the first walk (colors are padded to
                // the position vertex count before popping and both pops share
                // the same `min_ix`, so its flag run is at least as long as
                // the popped position vertex run) and exactly `num_values`
                // flags for the second, so every guarded `color_valid` read is
                // in bounds.
                unsafe {
                    let mut has_color: bool = false;
                    let mut all_valid: bool = true;
                    let max_index: usize = fbx_mesh.vertex_position().values_view().count();
                    // C: `ufbxi_for_list(uint32_t, p_ix, fbx_mesh->vertex_position.indices)`
                    let mut p_ix: *mut u32 =
                        fbx_mesh.vertex_position().indices_view().data() as *mut u32;
                    let p_ix_end: *mut u32 =
                        add_ptr(p_ix, fbx_mesh.vertex_position().indices_view().count());
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
                        fbx_mesh.vertex_color().set_exists(true);
                        fbx_mesh
                            .vertex_color()
                            .values_view()
                            .set_data(vertices[ObjAttrib::Color as usize].data as *const Vec4);
                        fbx_mesh
                            .vertex_color()
                            .values_view()
                            .set_count(vertices[ObjAttrib::Color as usize].count / 4);
                        // C: `fbx_mesh->vertex_color.indices = fbx_mesh->vertex_position.indices;`
                        fbx_mesh
                            .vertex_color()
                            .set_indices(fbx_mesh.vertex_position().indices());
                        fbx_mesh.vertex_color().set_unique_per_vertex(true);

                        if !all_valid {
                            let mut indices: *mut u32 =
                                fbx_mesh.vertex_color().indices_view().data() as *mut u32;
                            indices = uc
                                .result_view()
                                .push_copy_raw::<u32>(mesh.num_indices(), indices);
                            ufbxi_check!(uc, !indices.is_null(), "indices");

                            let num_values: usize = fbx_mesh.vertex_color().values_view().count();
                            // C: `ufbxi_for(uint32_t, p_ix, indices, mesh->num_indices)`
                            let mut p_ix: *mut u32 = indices;
                            let p_ix_end: *mut u32 = add_ptr(p_ix, mesh.num_indices());
                            while p_ix != p_ix_end {
                                if *p_ix as usize >= num_values || !*color_valid.add(*p_ix as usize)
                                {
                                    // SAFETY: `p_ix` addresses a live,
                                    // write-capable entry of the copied index
                                    // run — an adequate mint for the `Mut`
                                    // index-slot view.
                                    let p_dst: &View<u32> = View::<u32, Mut>::from_ptr(p_ix);
                                    fix_index(uc, p_dst, *p_ix, num_values)?;
                                }
                                p_ix = p_ix.add(1);
                            }

                            fbx_mesh.vertex_color().indices_view().set_data(indices);
                        }
                    }
                }
            }
        }

        finalize_mesh(uc.result_view(), uc.error_view(), fbx_mesh)?;

        if uc.retain_mesh_parts() {
            // The part run is freshly zero-pushed onto uc's result arena,
            // checked below.
            fbx_mesh
                .face_group_parts_view()
                .set_count(mesh.num_groups() as usize);
            fbx_mesh.face_group_parts_view().set_data(
                uc.result_view()
                    .push_zero::<MeshPart>(mesh.num_groups() as usize),
            );
            ufbxi_check!(
                uc,
                !fbx_mesh.face_group_parts_view().data().is_null(),
                "fbx_mesh->face_group_parts.data"
            );
        }

        if mesh.num_groups() > 1 {
            update_face_groups(uc.result_view(), uc.error_view(), fbx_mesh, false)?;
        } else if mesh.num_groups() == 1 {
            fbx_mesh
                .face_group_view()
                .set_data(SENTINEL_INDEX_ZERO.as_ptr());
            fbx_mesh.face_group_view().set_count(num_faces);
            // NOTE: Consecutive and zero indices are always allocated so we can skip doing it here,
            // see HACK(consecutiv-faces)..
            if fbx_mesh.face_group_parts_view().count() > 0 {
                // SAFETY: `part` is the first entry of the `face_group_parts`
                // run pushed above onto uc's result arena (write-capable
                // provenance), taken only when its count is non-zero.
                let part: &View<MeshPart> = unsafe {
                    View::<MeshPart>::from_ptr(
                        fbx_mesh.face_group_parts_view().data() as *mut MeshPart
                    )
                };
                // C-parity: `part->num_faces` is assigned twice in a row
                // (ufbx.c:17662-17663); the second write wins. Both are kept.
                part.set_num_faces(fbx_mesh.num_faces());
                part.set_num_faces(num_faces);
                part.set_num_empty_faces(fbx_mesh.num_empty_faces());
                part.set_num_point_faces(fbx_mesh.num_point_faces());
                part.set_num_line_faces(fbx_mesh.num_line_faces());
                part.set_num_triangles(fbx_mesh.num_triangles());
                // The sentinel index arrays are static.
                part.face_indices_view()
                    .set_data(SENTINEL_INDEX_CONSECUTIVE.as_ptr());
                part.face_indices_view().set_count(num_faces);
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
pub(crate) fn obj_parse_file(uc: &Context) -> Result<(), Fail> {
    while !uc.obj().eof() {
        obj_tokenize_line(uc)?;
        let num_tokens: usize = uc.obj().num_tokens();
        if num_tokens == 0 {
            continue;
        }

        // SAFETY: `num_tokens > 0` past the guard above, so token 0 is in the
        // tokenizer's stored token run and `cmd.data .. + length` is its span.
        let (cmd, key): (String, u32) = unsafe {
            let cmd: String = *uc.obj().tokens().add(0);
            (cmd, get_name_key(cmd.as_bytes()))
        };
        if key == obj_cmd1(b'v') {
            obj_parse_vertex(uc, ObjAttrib::Position, 1)?;
            if num_tokens >= 7 {
                let num_vertices: usize =
                    uc.obj().vertex_count_at(ObjAttrib::Position as usize).get();
                uc.obj().set_has_vertex_color(true);
                obj_pad_colors(uc, num_vertices.wrapping_sub(1))?;
                if uc.obj().vertex_count_at(ObjAttrib::Color as usize).get() < num_vertices {
                    ufbx_assert!(
                        uc.obj().vertex_count_at(ObjAttrib::Color as usize).get()
                            == num_vertices - 1
                    );
                    obj_parse_vertex(uc, ObjAttrib::Color, 4)?;
                    let valid: *mut bool = uc.obj().tmp_color_valid_view().push::<bool>(1);
                    ufbxi_check!(uc, !valid.is_null(), "valid");
                    // SAFETY: fresh push result, non-null past the check.
                    unsafe { *valid = true };
                }
            }
        } else if key == obj_cmd2(b'v', b't') {
            obj_parse_vertex(uc, ObjAttrib::Uv, 1)?;
        } else if key == obj_cmd2(b'v', b'n') {
            obj_parse_vertex(uc, ObjAttrib::Normal, 1)?;
        } else if key == obj_cmd1(b'f') {
            obj_parse_indices(uc, 1, uc.obj().num_tokens() - 1)?;
        } else if key == obj_cmd1(b'p') {
            obj_parse_multi_indices(uc, 1)?;
        } else if key == obj_cmd1(b'l') {
            obj_parse_multi_indices(uc, 2)?;
        } else if key == obj_cmd1(b's') {
            if num_tokens >= 2 {
                uc.obj().set_has_face_smoothing(true);
                // SAFETY: `num_tokens >= 2` here, so token 1 is in the stored
                // token run; the literal is NUL-terminated for `str_c`.
                uc.obj().set_face_smoothing(unsafe {
                    !str_equal((*uc.obj().tokens().add(1)).as_bytes(), b"off")
                });

                // Fill in previously missed face smoothing data
                if uc.obj().tmp_face_smoothing_view().num_items() == 0
                    && uc.obj().tmp_faces_view().num_items() > 0
                {
                    ufbxi_check!(
                        uc,
                        // Zero-fills one smoothing flag per already-recorded
                        // face on the obj parser's own `tmp_face_smoothing`
                        // arena.
                        !uc.obj()
                            .tmp_face_smoothing_view()
                            .push_zero::<bool>(uc.obj().tmp_faces_view().num_items())
                            .is_null(),
                        "((bool*)ufbxi_push_size_zero((&uc->obj.tmp_face_smoothing), sizeof(bool), (uc->obj.tmp_faces.num_items)))"
                    );
                }
            }
        } else if key == obj_cmd1(b'o') {
            if num_tokens >= 2 {
                uc.obj().set_object(obj_span_token(uc, 1, usize::MAX));
                push_string_place_str(uc.string_pool_view(), uc.obj().object_view(), false)?;
                uc.obj().set_object_dirty(true);
            }
        } else if key == obj_cmd1(b'g') {
            if num_tokens >= 2 {
                uc.obj().set_group(obj_span_token(uc, 1, usize::MAX));
                push_string_place_str(uc.string_pool_view(), uc.obj().group_view(), false)?;
                uc.obj().set_group_dirty(true);
            } else {
                uc.obj().set_group(EMPTY_STRING.0);
                uc.obj().set_group_dirty(true);
            }
        } else if key == obj_cmd1(b'#') {
            obj_parse_comment(uc)?;
        // SAFETY: `cmd` is token 0's span and the literals are NUL-terminated
        // for `str_c`.
        } else if unsafe { str_equal(cmd.as_bytes(), b"mtllib") } {
            ufbxi_check!(uc, uc.obj().num_tokens() >= 2, "uc->obj.num_tokens >= 2");
            let mut lib: String = obj_span_token(uc, 1, usize::MAX);
            // SAFETY: copies the span (plus its terminator, still inside the
            // line window) onto uc's own tmp arena.
            lib.data = unsafe { uc.tmp_view().push_copy_raw::<u8>(lib.length + 1, lib.data) };
            ufbxi_check!(uc, !lib.data.is_null(), "lib.data");
            uc.obj().mtllib_relative_path_view().set_data(lib.data);
            uc.obj().mtllib_relative_path_view().set_size(lib.length);
        // SAFETY: as for the `mtllib` comparison above.
        } else if unsafe { str_equal(cmd.as_bytes(), b"usemtl") } {
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
pub(crate) fn obj_flush_material(uc: &Context) -> Result<(), Fail> {
    if uc.obj().usemtl_fbx_id() == 0 {
        return Ok(());
    }

    let entry: *mut FbxIdEntry = find_fbx_id(uc, uc.obj().usemtl_fbx_id());
    ufbx_assert!(!entry.is_null());
    // SAFETY: `entry` is uc's own fbx-id map entry for the active `usemtl` id
    // (non-null past the assert), and `obj_parse_material` grew the obj
    // parser's `tmp_materials` array to cover that element id when it recorded
    // the material there.
    let material: *mut Material =
        unsafe { *uc.obj().tmp_materials().add((*entry).element_id as usize) };

    // SAFETY: `material` is the element stored for this id, living in uc's own
    // element arena (write-capable provenance).
    let material: &MaterialView = unsafe { MaterialView::from_ptr(material) };

    let num_props: usize = uc.obj().tmp_props_view().num_items();
    // SAFETY: the material element's own prop list is the unaliased
    // destination.
    unsafe { obj_pop_props(uc, material.props_view().props_raw(), num_props)? };

    Ok(())
}

// ufbx.c:17777-17850 `ufbxi_obj_parse_prop`
//
// # Safety
// `name` must be a valid `ufbx_string`: `data .. data + length` readable for
// the whole call, since the name is stored into the pushed prop and then
// interned (read and hashed) by `push_string_place_str` — a readability
// contract the `String` POD cannot carry.
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) unsafe fn obj_parse_prop(
    uc: &Context,
    name: String,
    start: usize,
    include_rest: bool,
    p_next: Option<&mut usize>,
) -> Result<(), Fail> {
    if start >= uc.obj().num_tokens() {
        if let Some(p_next) = p_next {
            *p_next = start;
        }
        return Ok(());
    }

    let prop: *mut Prop = uc.obj().tmp_props_view().push_zero::<Prop>(1);
    ufbxi_check!(uc, !prop.is_null(), "prop");
    // SAFETY: `prop` is the fresh non-null zeroed push result on the obj
    // parser's own `tmp_props` arena (write-capable provenance), so every
    // access through the anchored view below lands in that arena.
    let prop: &View<Prop> = unsafe { View::<Prop>::from_ptr(prop) };
    prop.set_name(name);

    // Interns the prop's own `name` field into uc's string pool.
    push_string_place_str(uc.string_pool_view(), prop.name_view(), false)?;

    let mut flags: u32 = PropFlags::VALUE_STR.raw();

    // C-parity: `prop->value_real_arr[]` is the `ufbx_prop` value union's
    // 4-real view (ufbx.h); the generated struct keeps only the `value_vec4`
    // member (PORTING.md union table). The reinterpreted `value_vec4` field is
    // four contiguous `Real`s.
    let value_real_arr: *mut Real = prop.value_vec4_raw() as *mut Real;

    let mut num_reals: usize = 0;
    while num_reals < 4 {
        if start + num_reals >= uc.obj().num_tokens() {
            break;
        }
        // SAFETY: `start + num_reals < num_tokens` (guard above), so it indexes
        // the tokenizer's stored token run.
        let tok: String = unsafe { *uc.obj().tokens().add(start + num_reals) };

        // C: `char *end; // ufbxi_uninit`
        let mut end: *const u8 = core::ptr::null(); // ufbxi_uninit
                                                    // SAFETY: `tok.data .. + length` is that token's own span and `end` is
                                                    // an unaliased local out-param.
        let val: f64 =
            unsafe { parse_double(tok.data, tok.length, &raw mut end, uc.double_parse_flags()) };
        // SAFETY: one past the same token span.
        if end != unsafe { tok.data.add(tok.length) } {
            break;
        }

        // SAFETY: `num_reals < 4` (loop condition) bounds the four-`Real` view
        // of the prop's own `value_vec4`.
        unsafe { *value_real_arr.add(num_reals) = val as Real };
        if num_reals == 0 {
            prop.set_value_int(f64_to_i64(val));
            flags |= PropFlags::VALUE_INT.raw();
        }

        num_reals += 1;
    }

    let mut num_args: usize = 0;
    if !include_rest {
        while start + num_args < uc.obj().num_tokens() - 1 {
            // SAFETY: `start + num_args < num_tokens - 1` (loop condition), so
            // it indexes the stored token run, whose entries span their own
            // readable bytes — `String::as_bytes`' contract; the pattern
            // literal is NUL-terminated, `r#match`'s contract.
            if unsafe {
                r#match(
                    (*uc.obj().tokens().add(start + num_args)).as_bytes(),
                    b"-[A-Za-z][\\-A-Za-z0-9_]*\0".as_ptr(),
                )
            } {
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
        // `span` is the token span `obj_span_token` returned for the same
        // token run.
        prop.set_value_str(span);
        let mut value_blob: Blob = prop.value_blob();
        value_blob.data = span.data;
        value_blob.size = span.length;
        prop.set_value_blob(value_blob);

        // Interns the prop's own `value_str` field into uc's string pool.
        push_string_place_str(uc.string_pool_view(), prop.value_str_view(), false)?;
        push_string_place_blob(uc.string_pool_view(), prop.value_blob_view(), true)?;
    } else {
        let mut value_str: String = prop.value_str();
        value_str.data = EMPTY_CHAR.as_ptr();
        prop.set_value_str(value_str);
    }

    if num_reals > 0 {
        flags = PropFlags::VALUE_REAL.raw() << (num_reals - 1);
    } else {
        // SAFETY: the prop's `value_str` is either an interned pool string or
        // `EMPTY_CHAR`, NUL-terminated either way, as are the two literals.
        if unsafe { strcmp(prop.value_str().data, b"on\0".as_ptr()) } == 0 {
            prop.set_value_int(1);
            // C: `prop->value_real = 1.0f;` — the first `ufbx_real` of the
            // value union (`value_vec4.x` in the generated struct).
            let mut value_vec4: Vec4 = prop.value_vec4();
            value_vec4.x = 1.0f32 as Real;
            prop.set_value_vec4(value_vec4);
            flags |= PropFlags::VALUE_INT.raw();
        // SAFETY: as for the `"on"` comparison above.
        } else if unsafe { strcmp(prop.value_str().data, b"off\0".as_ptr()) } == 0 {
            prop.set_value_int(0);
            let mut value_vec4: Vec4 = prop.value_vec4();
            value_vec4.x = 0.0f32 as Real;
            prop.set_value_vec4(value_vec4);
            flags |= PropFlags::VALUE_INT.raw();
        }
    }

    prop.set_flags(PropFlags::from_raw(flags));

    if let Some(p_next) = p_next {
        *p_next = start + num_args;
    }

    Ok(())
}

// ufbx.c:17852-17902 `ufbxi_obj_parse_mtl_map`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_parse_mtl_map(uc: &Context, prefix_len: usize) -> Result<(), Fail> {
    if uc.obj().num_tokens() < 2 {
        return Ok(());
    }

    let mut num_props: usize = 1;
    // SAFETY: the property name is a NUL-terminated literal, so `str_c` yields
    // a readable span for the parser to intern.
    unsafe { obj_parse_prop(uc, str_c(b"obj|args\0".as_ptr()), 1, true, None)? };

    let mut start: usize = 1;
    // C: `for (; start + 1 < uc->obj.num_tokens; )`
    while start + 1 < uc.obj().num_tokens() {
        // SAFETY: `start + 1 < num_tokens` (loop condition), so token `start`
        // is in the stored token run; the match guarantees the token is at
        // least two bytes, so dropping the leading '-' keeps `tok` inside its
        // own span.
        let mut tok: String = unsafe { *uc.obj().tokens().add(start) };
        // SAFETY: `tok` is that token's own readable span — `String::as_bytes`'
        // contract; the pattern literal is NUL-terminated, `r#match`'s.
        if unsafe { r#match(tok.as_bytes(), b"-[A-Za-z][\\-A-Za-z0-9_]*\0".as_ptr()) } {
            // SAFETY: the match guarantees at least two bytes, so the
            // advanced `data` stays inside the token's own span.
            tok.data = unsafe { tok.data.add(1) };
            tok.length -= 1;
            // SAFETY: `tok` is still that token's own readable span after
            // dropping the leading '-'.
            unsafe { obj_parse_prop(uc, tok, start + 1, false, Some(&mut start))? };
            num_props += 1;
        } else {
            break;
        }
    }

    let mut tex_str: String = obj_span_token(uc, start, usize::MAX);
    let mut tex_raw: Blob = Blob::new_c(tex_str.data, tex_str.length);

    // Interns the texture path into uc's own string pool through an unaliased
    // local.
    push_string_place_str(
        uc.string_pool_view(),
        StringView::from_mut(&mut tex_str),
        false,
    )?;
    push_string_place_blob(
        uc.string_pool_view(),
        BlobView::from_mut(&mut tex_raw),
        true,
    )?;

    let mut fbx_id: u64 = 0;
    // SAFETY: `fbx_id` is an unaliased local out-param and the name is a
    // NUL-terminated literal; the push targets uc's own element arenas.
    let texture: *mut Texture = unsafe {
        push_synthetic_element::<Texture>(
            uc,
            ScalarView::from_mut(&mut fbx_id),
            None,
            b"\0".as_ptr(),
            ElementType::Texture,
        )
    };
    ufbxi_check!(uc, !texture.is_null(), "texture");

    // SAFETY: `texture` is the fresh non-null element push result, so this run
    // initializes its own fields and pops the props collected above into its
    // own prop list.
    unsafe {
        (*texture).filename.data = EMPTY_CHAR.as_ptr();
        (*texture).absolute_filename.data = EMPTY_CHAR.as_ptr();
        (*texture).uv_set.data = EMPTY_CHAR.as_ptr();

        (*texture).relative_filename = tex_str;
        (*texture).raw_relative_filename = tex_raw;

        obj_pop_props(uc, &raw mut (*texture).element.props.props, num_props)?;
    }

    // SAFETY: `num_tokens >= 2` past the guard above, so token 0 is in the
    // stored token run; `prop.length >= prefix_len` (asserted) keeps the
    // trimmed span inside that token, and `prop` is an unaliased local.
    let prop: String = unsafe {
        let mut prop: String = *uc.obj().tokens().add(0);
        ufbx_assert!(prop.length >= prefix_len);
        prop.data = prop.data.add(prefix_len);
        prop.length -= prefix_len;
        push_string_place_str(
            uc.string_pool_view(),
            StringView::from_mut(&mut prop),
            false,
        )?;
        prop
    };

    if uc.obj().usemtl_fbx_id() != 0 {
        // SAFETY: connects the fresh texture id to the active material id in
        // uc's connection arena.
        unsafe { connect_op(uc, fbx_id, uc.obj().usemtl_fbx_id(), prop)? };
    }

    Ok(())
}

// ufbx.c:17904-17934 `ufbxi_obj_parse_mtl`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_parse_mtl(uc: &Context) -> Result<(), Fail> {
    uc.obj().set_mesh(core::ptr::null_mut());
    uc.obj().set_usemtl_fbx_id(0);

    while !uc.obj().eof() {
        obj_tokenize_line(uc)?;
        let num_tokens: usize = uc.obj().num_tokens();
        if num_tokens == 0 {
            continue;
        }

        // SAFETY: `num_tokens > 0` past the guard above, so token 0 is in the
        // stored token run and every comparison below reads at most
        // `cmd.length` bytes of its own span (guarded by the length tests);
        // the literals are NUL-terminated for `str_c`, and the fallback
        // property parse gets that same readable token 0 span as its name and
        // takes no out-param.
        unsafe {
            let cmd: String = *uc.obj().tokens().add(0);
            if str_equal(cmd.as_bytes(), b"newmtl") {
                // HACK: Reuse mesh material parsing, but don't allow for empty material name
                ufbxi_check!(uc, uc.obj().num_tokens() >= 2, "uc->obj.num_tokens >= 2");
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
                obj_parse_prop(uc, *uc.obj().tokens().add(0), 1, true, None)?;
            }
        }
    }

    obj_flush_material(uc)?;

    Ok(())
}

// ufbx.c:17936-18027 `ufbxi_obj_load_mtl`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_load_mtl(uc: &Context) -> Result<(), Fail> {
    // HACK: Reset everything and switch to loading the .mtl file globally
    if let Some(close_fn) = uc.close_fn() {
        // SAFETY: invoking uc's own close callback with the `read_user` it was
        // paired with is the C stream contract.
        unsafe { close_fn(uc.read_user()) };
    }

    uc.set_read_fn(None);
    uc.set_close_fn(None);
    uc.set_read_user(core::ptr::null_mut());
    uc.set_data_begin(core::ptr::null());
    uc.set_data(core::ptr::null());
    uc.set_data_size(0);
    uc.set_yield_size(0);
    uc.set_eof(false);
    uc.obj().set_eof(false);

    if uc.opts_view().obj_mtl_data_view().size() > 0 {
        uc.set_data(uc.opts_view().obj_mtl_data_view().data());
        uc.set_data_begin(uc.data());
        uc.set_data_size(uc.opts_view().obj_mtl_data_view().size());
        obj_parse_mtl(uc)?;
        return Ok(());
    }

    // C: `ufbx_stream stream = { 0 };`
    // SAFETY: `RawStream` and `Blob` are plain C data whose all-zero bit
    // patterns are their valid `{ 0 }` initializers.
    let mut stream: RawStream = unsafe { core::mem::zeroed() };
    let mut has_stream: bool = false;
    let mut needs_stream: bool = false;
    // C: `ufbx_blob stream_path = { 0 };`
    let mut stream_path: Blob = unsafe { core::mem::zeroed() };

    if uc.opts_view().open_file_cb_view().fn_().is_some() {
        if uc.opts_view().obj_mtl_path_view().length() > 0 {
            // SAFETY: `stream` is an unaliased local out-param; the path is
            // the caller-supplied `obj_mtl_path` with its own length, and the
            // callback runs through uc's own temp allocator.
            has_stream = unsafe {
                open_file::<Mut>(
                    uc.opts_view().open_file_cb_ptr(),
                    &raw mut stream,
                    uc.opts_view().obj_mtl_path_view().data(),
                    uc.opts_view().obj_mtl_path_view().length(),
                    None,
                    Some(uc.ator_tmp_view()),
                    OpenFileType::ObjMtl,
                )
            };
            stream_path.data = uc.opts_view().obj_mtl_path_view().data();
            stream_path.size = uc.opts_view().obj_mtl_path_view().length();
            needs_stream = true;
            if !has_stream {
                ufbxi_check!(
                    uc,
                    // The `%s` argument is the caller-supplied `obj_mtl_path`,
                    // a NUL-terminated string (PrintArg pointer contract).
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
            && uc.obj().mtllib_relative_path_view().size() > 0
        {
            // C: `ufbx_blob dst; // ufbxi_uninit`
            let mut dst = MaybeUninit::<Blob>::uninit(); // ufbxi_uninit
            let dst: *mut Blob = dst.as_mut_ptr();
            // SAFETY: `dst` is the unaliased local out-param the resolver
            // fully writes before Ok, so the open below reads an initialized
            // blob; `Mut` storage tolerates the still-uninitialized slot. The
            // relative path is the obj context's own stored one, a distinct
            // live strblob of matching `raw`-ness.
            let (p_dst, p_src): (&View<Strblob>, &View<Strblob>) = unsafe {
                (
                    View::<Strblob, Mut>::from_ptr(dst as *mut Strblob),
                    View::<Strblob, Mut>::from_ptr(
                        uc.obj().mtllib_relative_path_mut_ptr() as *mut Strblob
                    ),
                )
            };
            resolve_relative_filename(uc, p_dst, p_src, true)?;
            // SAFETY: `stream` is an unaliased local, and `dst` is initialized
            // by the resolver above.
            unsafe {
                has_stream = open_file(
                    uc.opts_view().open_file_cb_ptr(),
                    &raw mut stream,
                    (*dst).data,
                    (*dst).size,
                    Some(uc.obj().mtllib_relative_path_view()),
                    Some(uc.ator_tmp_view()),
                    OpenFileType::ObjMtl,
                );
            }
            stream_path = uc.obj().mtllib_relative_path();
            needs_stream = true;
            if !has_stream {
                ufbxi_check!(
                    uc,
                    // SAFETY: appends to uc's own warning buffer; the format
                    // literal is NUL-terminated and `dst` was fully written by
                    // the resolver above, holding a NUL-terminated path.
                    unsafe {
                        ufbxi_warnf!(
                            uc,
                            WarningType::MissingExternalFile,
                            "Could not open .mtl file: %s",
                            (*dst).data
                        )
                    }
                    .is_ok(),
                    "ufbxi_warnf_imp(&uc->warnings, UFBX_WARNING_MISSING_EXTERNAL_FILE, ~0u, \"Could not open .mtl file: %s\", dst.data)"
                );
            }
        }

        let path: String = uc.scene_view().metadata_view().filename();
        if !has_stream
            && uc.opts_view().load_external_files()
            && uc.opts_view().obj_search_mtl_by_filename()
            && path.length > 4
        {
            // C: `ufbx_string ext = { path.data + path.length - 4, 4 };`
            // SAFETY: `path.length > 4` (checked above), so the 4-byte
            // extension window is inside the scene filename.
            let ext: String = unsafe { String::new_c(path.data.add(path.length - 4), 4) };
            // SAFETY: `ext` spans the filename's own last 4 bytes —
            // `String::as_bytes`' contract; the pattern literal is
            // NUL-terminated, `r#match`'s.
            if unsafe { r#match(ext.as_bytes(), b"\\c.obj\0".as_ptr()) } {
                ufbxi_analysis_assert!(path.length < usize::MAX - 1);
                // SAFETY: copies the filename (with its terminator) onto uc's
                // own tmp arena.
                let copy: *mut u8 = unsafe {
                    uc.tmp_view()
                        .push_copy_raw::<u8>(path.length + 1, path.data)
                };
                ufbxi_check!(uc, !copy.is_null(), "copy");
                // SAFETY: `copy` is the fresh non-null `path.length + 1` byte
                // run and `path.length > 4`, so the three rewritten extension
                // bytes are inside it; `stream` is an unaliased local
                // out-param for the open below.
                unsafe {
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
                    has_stream = open_file::<Mut>(
                        uc.opts_view().open_file_cb_ptr(),
                        &raw mut stream,
                        copy,
                        path.length,
                        None,
                        Some(uc.ator_tmp_view()),
                        OpenFileType::ObjMtl,
                    );
                }
                if has_stream {
                    ufbxi_check!(
                        uc,
                        // The `%s` argument `copy` is the fresh arena copy of
                        // the filename, terminator included (PrintArg pointer
                        // contract).
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
            // SAFETY: closes the stream just adopted, with the `user` pointer
            // it was paired with (C stream contract).
            unsafe { close_fn(uc.read_user()) };
        }
        uc.set_read_fn(None);
        uc.set_close_fn(None);
        uc.set_read_user(core::ptr::null_mut());

        ok?;
    } else if needs_stream && !uc.opts_view().ignore_missing_external_files() {
        // SAFETY: records the attempted path (`data .. data + size` of the
        // blob set alongside `needs_stream`) into uc's own error state.
        unsafe { set_err_info(Some(uc.error_view()), stream_path.data, stream_path.size) };
        ufbxi_fail_msg!(uc, "ufbxi_obj_load_mtl()", "External file not found");
    }

    Ok(())
}

// ufbx.c:18029-18037 `ufbxi_obj_load`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn obj_load(uc: &Context) -> Result<(), Fail> {
    obj_init(uc)?;
    obj_parse_file(uc)?;
    init_file_paths(uc)?;
    obj_load_mtl(uc)?;

    Ok(())
}

// ufbx.c:18039-18046 `ufbxi_mtl_load`
#[cfg(feature = "obj")]
#[inline(never)]
pub(crate) fn mtl_load(uc: &Context) -> Result<(), Fail> {
    obj_init(uc)?;
    init_file_paths(uc)?;
    obj_parse_mtl(uc)?;

    Ok(())
}

// ufbx.c:18049-18053 `ufbxi_obj_load` (`#else` branch — feature disabled)
#[cfg(not(feature = "obj"))]
#[inline(always)]
pub(crate) fn obj_load(uc: &Context) -> Result<(), Fail> {
    // SAFETY: the format string is a literal with no conversions.
    unsafe { ufbxi_fmt_err_info!(Some(uc.error_view()), "UFBX_ENABLE_FORMAT_OBJ") };
    ufbxi_fail_msg!(uc, "UFBXI_FEATURE_FORMAT_OBJ", "Feature disabled");
}

// ufbx.c:18055-18059 `ufbxi_mtl_load` (`#else` branch — feature disabled)
#[cfg(not(feature = "obj"))]
#[inline(always)]
pub(crate) fn mtl_load(uc: &Context) -> Result<(), Fail> {
    // SAFETY: the format string is a literal with no conversions.
    unsafe { ufbxi_fmt_err_info!(Some(uc.error_view()), "UFBX_ENABLE_FORMAT_OBJ") };
    ufbxi_fail_msg!(uc, "UFBXI_FEATURE_FORMAT_OBJ", "Feature disabled");
}

// ufbx.c:18061-18063 `ufbxi_obj_free` (`#else` branch — feature disabled)
#[cfg(not(feature = "obj"))]
#[inline(always)]
pub(crate) fn obj_free(uc: &Context) {
    let _ = uc;
}
