//! Port of the `// -- Type definitions` banner section (ufbx.c:6175-6673), the
//! `// -- Progress` banner section (ufbx.c:6676-6712), the
//! `// -- FBX value type information` (ufbx.c:7684-7709),
//! `// -- Node operations` (ufbx.c:7711-7877) and
//! `// -- Element extra data allocation` (ufbx.c:7879-7907) and
//! `// -- Parsing state machine` (ufbx.c:7909-8606) and
//! `// -- DOM retention` (ufbx.c:10696-10854) banner sections.
//!
//! This unit owns the internal parse-time type definitions (`ufbxi_node`,
//! `ufbxi_refcount`, `ufbxi_scene_imp`/`ufbxi_mesh_imp`, `ufbxi_ascii`, the
//! temporary entry structs, `ufbxi_obj_context` and the big `ufbxi_context`),
//! plus `ufbxi_fail_imp`/`ufbxi_fail_imp_no_stack` (ufbx.c:6652-6662) — the
//! expansion targets the `uc`-context check macros in `native::error` name —
//! and the progress machinery.
//!
//! The `uc`-context check macros themselves (C: ufbx.c:6664-6671, defined next
//! to `ufbxi_context`) live in `native::error` with the rest of the macro
//! family; the `ufbxi_warnf`/`ufbxi_warnf_tag` wrappers (ufbx.c:6673-6674) live
//! in `native::warnings`.
//!
//! The refcount lifecycle functions (`ufbxi_init_ref`/`ufbxi_retain_ref`/
//! `ufbxi_release_ref`, C prototypes ufbx.c:6229-6230) are defined by C in the
//! API section (ufbx.c:30248-30300) — ported in `native::api` following the
//! C's own placement.
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]

use core::ffi::c_void;
use core::mem::size_of;

use crate::generated::{
    DomNode, DomValue, DomValueType, ElementType, Error, Exporter, FileFormat, InflateRetain,
    Matrix, MirrorAxis, Progress, ProgressResult, Prop, PropFlags, PropType, Props, Quat,
    RawLoadOpts, Scene, TextureFile, Transform, Vec3, Vec4,
};
use crate::native::allocator::{grow_array, Allocator};
use crate::native::buf::{
    buf_clear, buf_free, pop, push_copy, push_pop, push_size_zero, push_zero, Buf,
};
use crate::native::error::{
    memchr, memcmp, strcmp, strncmp, ufbxi_check, ufbxi_check_msg, ufbxi_check_return, ufbxi_fail,
    Fail, EMPTY_CHAR,
};
use crate::native::hash::{hash_uptr, map_find, map_insert, Map, PtrId};
use crate::native::parse_ascii::is_space;
use crate::native::parse_binary::{BINARY_HEADER_SIZE, BINARY_MAGIC, BINARY_MAGIC_SIZE};
use crate::native::platform::{
    add_ptr, min_sz, read_u32, to_size, ufbx_assert, ufbxi_dev_assert, ufbxi_ignore,
    ufbxi_unreachable, AtomicCounter,
};
use crate::native::string_pool as sp;
use crate::native::string_pool::{SanitizedString, StringPool};
use crate::native::thread::{ThreadPool, THREAD_GROUP_COUNT};
use crate::native::warnings::Warnings;
use crate::prelude::{Blob, Real, Ref, String};

// ufbx.h:744 `UFBX_ENUM_TYPE(ufbx_element_type, UFBX_ELEMENT_TYPE, UFBX_ELEMENT_METADATA_OBJECT);`
// expanding via ufbx.h:235-236 to `enum { UFBX_ELEMENT_TYPE_COUNT = UFBX_ELEMENT_METADATA_OBJECT + 1 }`.
// Hand-duplicated from the generated enum's last variant so an upstream enum
// change tracks automatically through regen (precedent: `WARNING_TYPE_COUNT`
// in `native::warnings`).
pub(crate) const ELEMENT_TYPE_COUNT: usize = ElementType::MetadataObject as usize + 1;

// ufbx.c:51 `#define UFBXI_MAX_NON_ARRAY_VALUES 8` (no UFBX_REGRESSION
// override) — owned here as the `ufbxi_node`/`ufbxi_value` unit uses it
// (ufbx.c:7733) and the parsers index `vals` against it.
pub(crate) const MAX_NON_ARRAY_VALUES: usize = 8;

// ufbx.c:52 `#define UFBXI_MAX_NODE_DEPTH 32` — owned here alongside
// `MAX_NON_ARRAY_VALUES`; both binary and ASCII node parsers bound their
// recursion with it.
pub(crate) const MAX_NODE_DEPTH: u32 = 32;

// -- Type definitions

// ufbx.c:6177 `typedef struct ufbxi_node ufbxi_node;` (forward declaration —
// collapses into the struct definition below)

// ufbx.c:6179-6184 `ufbxi_value_type`
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ValueType {
    None,
    Number,
    String,
    Array,
}

// ufbx.c:6186-6189 `ufbxi_value`
// Untagged union discriminated by `value_type_mask` in the parent node —
// stays a `#[repr(C)] union`, never a Rust `enum` (PORTING.md "Unions"). The
// anonymous C struct `{ double f; int64_t i; }` becomes the named `ValueNum`
// member: `f` and `i` are SEQUENTIAL fields (both written and both read — see
// the "False positive" comments at ufbx.c:10467/10495/10542), not overlays of
// each other.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ValueNum {
    pub f: f64,
    pub i: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) union Value {
    pub num: ValueNum,      // < if `UFBXI_PROP_NUMBER`
    pub s: SanitizedString, // < if `UFBXI_PROP_STRING`
}

// ufbx.c:6191-6195 `ufbxi_value_array`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ValueArray {
    pub data: *mut c_void, // < Pointer to `size` bool/int32_t/int64_t/float/double elements
    pub size: usize,       // < Number of elements
    pub type_: u8,         // < FBX type code: b/i/l/f/d
}

// ufbx.c:6197-6213 `ufbxi_node`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Node {
    pub name: *const u8, // < Name of the node (pooled, compare with == to ufbxi_* strings)
    pub num_children: u32, // < Number of child nodes
    pub name_len: u8,    // < Length of `name` in bytes

    // If `value_type_mask == UFBXI_PROP_ARRAY` then the node is an array
    // (`array` field is valid) otherwise the node has N values in `vals`
    // where the type of each value is stored in 2 bits per value from LSB.
    // ie. `vals[ix]` type is `(value_type_mask >> (ix*2)) & 0x3`
    pub value_type_mask: u16,

    pub children: *mut Node,
    pub content: NodeContent,
}

// ufbx.c:6209-6212 (anonymous union inside `ufbxi_node`) — untagged overlay
// discriminated by `value_type_mask` (PORTING.md "Unions").
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) union NodeContent {
    pub array: *mut ValueArray, // if `prop_type_mask == UFBXI_PROP_ARRAY`
    pub vals: *mut Value,       // otherwise
}

// ufbx.c:6215 `typedef struct ufbxi_refcount ufbxi_refcount;` (forward
// declaration — collapses into the struct definition below)

// ufbx.c:6217-6227 `ufbxi_refcount`
// NOT `Copy`: `refcount` is an atomic; C never copies the struct by value
// (`ufbxi_release_ref` copies the `ator`/`buf` FIELDS to the stack, see
// `native::api::release_ref`).
#[repr(C)]
pub(crate) struct Refcount {
    pub parent: *mut Refcount,
    pub align_0: *mut c_void,
    pub self_magic: u32,
    pub type_magic: u32,
    pub buf: Buf,
    pub ator: Allocator,
    pub zero_pad_pre: [u64; 8],
    pub refcount: AtomicCounter,
    pub zero_pad_post: [u64; 8],
}

// ufbx.c:6229-6230 — forward declarations of `ufbxi_init_ref` and
// `ufbxi_retain_ref`; the definitions live in the API section (ported as
// `native::api::init_ref` / `retain_ref` / `release_ref`).

// ufbx.c:6232 `#define ufbxi_get_imp(type, ptr) ((type*)((char*)ptr - sizeof(ufbxi_refcount)))`
// The refcount header lives immediately before the public struct inside the
// same allocation — recover the `*_imp` pointer by subtracting the header
// size (layout pinned by the const asserts on the `*_imp` structs below).
//
// The input `ptr` is a public API pointer (e.g. `&Scene`-derived) whose
// provenance may be narrowed to the sub-object, which would make reaching the
// refcount header (outside it) UB. Reconstitute a wildcard pointer from the
// bare address via exposed provenance: every `*_imp` allocation site exposes
// its wide (allocation-covering) pointer once at creation, so the header falls
// within an exposed allocation and the recovered pointer can legally reach it.
#[inline(always)]
pub(crate) unsafe fn get_imp<T>(ptr: *mut c_void) -> *mut T {
    let addr = (ptr as *mut u8).addr();
    core::ptr::with_exposed_provenance_mut::<u8>(addr - size_of::<Refcount>()) as *mut T
}

// ufbx.c:6234-6240 `ufbxi_scene_imp`
#[repr(C)]
pub(crate) struct SceneImp {
    pub refcount: Refcount,
    pub scene: Scene,
    pub magic: u32,

    pub string_buf: Buf,
}

// ufbx.c:6242 `ufbx_static_assert(scene_imp_offset, offsetof(ufbxi_scene_imp, scene) == sizeof(ufbxi_refcount));`
const _: () = assert!(core::mem::offset_of!(SceneImp, scene) == size_of::<Refcount>());

// ufbx.c:6244-6248 `ufbxi_mesh_imp`
#[repr(C)]
pub(crate) struct MeshImp {
    pub refcount: Refcount,
    pub mesh: crate::generated::Mesh,
    pub magic: u32,
}

// ufbx.c:6250 `ufbx_static_assert(mesh_imp_offset, offsetof(ufbxi_mesh_imp, mesh) == sizeof(ufbxi_refcount));`
const _: () = assert!(core::mem::offset_of!(MeshImp, mesh) == size_of::<Refcount>());

// ufbx.c:6252-6272 `ufbxi_ascii_token`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct AsciiToken {
    // Semantic string data and length eg. for a string token
    // this string doesn't include the quotes.
    pub str_data: *mut u8,
    pub str_len: usize,
    pub str_cap: usize,

    // Type of the token, either single character such as '{' or ':'
    // or one of UFBXI_ASCII_* defines.
    pub type_: u8,

    // Sign for integer if negative.
    pub negative: bool,

    // Parsed semantic value
    pub value: AsciiTokenValue,
}

// ufbx.c:6267-6271 (anonymous `value` union inside `ufbxi_ascii_token`) —
// untagged overlay discriminated by `type` (PORTING.md "Unions").
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) union AsciiTokenValue {
    pub f64_: f64,
    pub i64_: i64,
    pub name_len: usize,
}

// ufbx.c:6274-6291 `ufbxi_ascii`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Ascii {
    pub max_token_length: usize,

    pub src: *const u8,
    pub src_yield: *const u8,
    pub src_end: *const u8,

    pub read_first_comment: bool,
    pub found_version: bool,
    pub parse_as_f32: bool,
    pub src_is_retained: bool,

    pub retain_buf: *mut Buf,
    pub src_buf: *mut Buf,

    pub prev_token: AsciiToken,
    pub token: AsciiToken,
}

// ufbx.c:6293-6297 `ufbxi_template`
// No `Copy`: `Props` (generated) is not `Copy`.
#[repr(C)]
pub(crate) struct Template {
    pub type_: *const u8,
    pub sub_type: String,
    pub props: Props,
}

// ufbx.c:6299-6303 `ufbxi_fbx_id_entry`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct FbxIdEntry {
    pub fbx_id: u64,
    pub element_id: u32,
    pub user_id: u32,
}

// ufbx.c:6305-6308 `ufbxi_ptr_fbx_id_entry`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PtrFbxIdEntry {
    pub ptr_id: PtrId,
    pub fbx_id: u64,
}

// ufbx.c:6310-6313 `ufbxi_fbx_attr_entry`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct FbxAttrEntry {
    pub node_fbx_id: u64,
    pub attr_fbx_id: u64,
}

// ufbx.c:6315-6320 `ufbxi_tmp_connection`
// Temporary connection before we resolve the element pointers
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct TmpConnection {
    pub src: u64,
    pub dst: u64,
    pub src_prop: String,
    pub dst_prop: String,
}

// ufbx.c:6322-6327 `ufbxi_element_info`
// No `Copy`: `Props` (generated) is not `Copy`.
#[repr(C)]
pub(crate) struct ElementInfo {
    pub fbx_id: u64,
    pub name: String,
    pub props: Props,
    pub dom_node: *mut DomNode,
}

// ufbx.c:6329-6332 `ufbxi_tmp_bone_pose`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct TmpBonePose {
    pub bone_fbx_id: u64,
    pub bone_to_world: Matrix,
}

// ufbx.c:6334-6339 `ufbxi_tmp_mesh_texture`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct TmpMeshTexture {
    pub prop_name: String,
    pub face_texture: *mut u32,
    pub num_faces: usize,
    pub all_same: bool,
}

// ufbx.c:6341-6344 `ufbxi_mesh_extra`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct MeshExtra {
    pub texture_arr: *mut TmpMeshTexture,
    pub texture_count: usize,
}

// ufbx.c:6346-6350 `ufbxi_tmp_material_texture`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct TmpMaterialTexture {
    pub material_id: i32,
    pub texture_id: i32,
    pub prop_name: String,
}

// ufbx.c:6352-6358 `ufbxi_texture_extra`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct TextureExtra {
    pub blend_modes: *mut i32,
    pub num_blend_modes: usize,

    pub alphas: *mut Real,
    pub num_alphas: usize,
}

// ufbx.c:6360-6365 `ufbxi_obj_attrib`
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ObjAttrib {
    Position,
    Uv,
    Normal,
    Color,
}

// ufbx.c:6367 `#define UFBXI_OBJ_NUM_ATTRIBS 3`
pub(crate) const OBJ_NUM_ATTRIBS: usize = 3;
// ufbx.c:6368 `#define UFBXI_OBJ_NUM_ATTRIBS_EXT 4`
pub(crate) const OBJ_NUM_ATTRIBS_EXT: usize = 4;

// ufbx.c:6370-6372 `ufbxi_obj_index_range`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ObjIndexRange {
    pub min_ix: u64,
    pub max_ix: u64,
}

// ufbx.c:6374-6388 `ufbxi_obj_mesh`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ObjMesh {
    pub num_faces: usize,
    pub num_indices: usize,
    pub vertex_range: [ObjIndexRange; OBJ_NUM_ATTRIBS],

    pub fbx_node: *mut crate::generated::Node,
    pub fbx_mesh: *mut crate::generated::Mesh,

    pub fbx_node_id: u64,
    pub fbx_mesh_id: u64,

    pub usemtl_base: u32,

    pub num_groups: u32,
}

// ufbx.c:6390-6394 `ufbxi_obj_group_entry`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ObjGroupEntry {
    pub name: *const u8,
    pub local_id: u32,
    pub mesh_id: u32,
}

// ufbx.c:6396-6399 `ufbxi_obj_fast_indices`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ObjFastIndices {
    pub indices: *mut u64,
    pub num_left: usize,
}

// ufbx.c:6401-6406 `ufbxi_tmp_anim_stack`
// Temporary pointer to a `ufbx_anim_stack` by name used to patch start/stop
// time from "Takes" if necessary.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct TmpAnimStack {
    pub name: *const u8,
    pub stack: *mut crate::generated::AnimStack,
}

// ufbx.c:6408-6411 `ufbxi_file_content`
// No `Copy`: `Blob` (prelude) is not `Copy`.
#[repr(C)]
pub(crate) struct FileContent {
    pub absolute_filename: String,
    pub content: Blob,
}

// ufbx.c:6413-6467 `ufbxi_obj_context`
#[repr(C)]
pub(crate) struct ObjContext {
    // Current line and tokens.
    // NOTE: `line` and `tokens` are not NULL-terminated nor UTF-8!
    // `line` is guaranteed to be terminated by a `\n`
    pub line: String,
    pub tokens: *mut String,
    pub tokens_cap: usize,
    pub num_tokens: usize,

    pub fast_indices: [ObjFastIndices; OBJ_NUM_ATTRIBS],

    pub vertex_count: [usize; OBJ_NUM_ATTRIBS_EXT],
    pub tmp_vertices: [Buf; OBJ_NUM_ATTRIBS_EXT],
    pub tmp_indices: [Buf; OBJ_NUM_ATTRIBS_EXT],
    pub tmp_color_valid: Buf,
    pub tmp_faces: Buf,
    pub tmp_face_smoothing: Buf,
    pub tmp_face_group: Buf,
    pub tmp_face_group_infos: Buf,
    pub tmp_face_material: Buf,
    pub tmp_meshes: Buf,
    pub tmp_props: Buf,

    pub group_map: Map,

    pub read_progress: usize,

    pub mesh: *mut ObjMesh,

    pub usemtl_fbx_id: u64,
    pub usemtl_index: u32,

    pub face_material: u32,

    pub face_group: u32,
    pub has_face_group: bool,

    pub face_smoothing: bool,
    pub has_face_smoothing: bool,

    pub has_vertex_color: bool,
    pub mrgb_vertex_count: usize,

    pub eof: bool,
    pub initialized: bool,

    pub mtllib_relative_path: Blob,

    pub tmp_materials: *mut *mut crate::generated::Material,
    pub tmp_materials_cap: usize,

    pub object: String,
    pub group: String,
    pub material_dirty: bool,
    pub object_dirty: bool,
    pub group_dirty: bool,
    pub face_group_dirty: bool,
}

// ufbx.c:6469-6650 `ufbxi_context`
// C fn-pointer typedefs from ufbx.h (`ufbx_read_fn` etc., ufbx.h:4143-4153)
// are inlined by the bindings generator — the `Option<unsafe extern "C" fn>`
// signatures below match the generated `Stream` callback fields byte-for-byte.
#[repr(C)]
pub(crate) struct InnerContext {
    pub error: Error,
    pub version: u32,
    pub exporter: Exporter,
    pub exporter_version: u32,
    pub from_ascii: bool,
    pub local_big_endian: bool,
    pub file_big_endian: bool,
    pub sure_fbx: bool,
    pub retain_mesh_parts: bool,
    pub read_legacy_settings: bool,
    pub double_parse_flags: u32,

    pub opts: RawLoadOpts,

    // IO
    pub data_offset: u64,
    pub read_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>,
    pub skip_fn: Option<unsafe extern "C" fn(*mut c_void, usize) -> bool>,
    pub read_user: *mut c_void,

    pub read_buffer: *mut u8,
    pub read_buffer_size: usize,

    pub data_begin: *const u8,
    pub data: *const u8,
    pub yield_size: usize,
    pub data_size: usize,

    // Allocators
    pub ator_result: Allocator,
    pub ator_tmp: Allocator,

    // Temporary maps
    pub prop_type_map: Map,  // < `ufbxi_prop_type_name` Property type to enum
    pub fbx_id_map: Map,     // < `ufbxi_fbx_id_entry` FBX ID to local ID
    pub ptr_fbx_id_map: Map, // < `ufbxi_ptr_fbx_id_entry` Pointer/negative ID to FBX ID
    pub texture_file_map: Map, // < `ufbxi_texture_file_entry` absolute raw filename to element ID
    pub anim_stack_map: Map, // < `ufbxi_tmp_anim_stack` anim stacks by name before finalization

    // 6x00 specific maps
    pub fbx_attr_map: Map,  // < `ufbxi_fbx_attr_entry` Node ID to attrib ID
    pub node_prop_set: Map, // < `const char*` Node property names

    // DOM nodes
    pub dom_node_map: Map, // < `const char*` Node property names

    // Temporary array
    pub tmp_arr: *mut u8,
    pub tmp_arr_size: usize,
    pub swap_arr: *mut u8,
    pub swap_arr_size: usize,

    // Generated index buffers
    pub max_zero_indices: usize,
    pub max_consecutive_indices: usize,

    // Temporary buffers
    pub tmp: Buf,
    pub tmp_parse: Buf,
    pub tmp_stack: Buf,
    pub tmp_connections: Buf,
    pub tmp_node_ids: Buf,
    pub tmp_elements: Buf,
    pub tmp_element_offsets: Buf,
    pub tmp_element_fbx_ids: Buf,
    pub tmp_element_ptrs: Buf,
    pub tmp_typed_element_offsets: [Buf; ELEMENT_TYPE_COUNT],
    pub tmp_mesh_textures: Buf,
    pub tmp_full_weights: Buf,
    pub tmp_dom_nodes: Buf,
    pub tmp_element_id: Buf,
    pub tmp_ascii_spans: Buf,
    pub tmp_thread_parse: [Buf; THREAD_GROUP_COUNT],
    pub tmp_element_byte_offset: usize,

    pub templates: *mut Template,
    pub num_templates: usize,

    pub dom_parse_toplevel: *mut DomNode,
    pub dom_parse_num_children: usize,

    pub p_element_id: *mut u32,

    // String pool
    pub string_pool: StringPool,

    // Result buffers, these are retained in `ufbx_scene` returned to user.
    pub result: Buf,

    // Top-level state
    pub top_nodes: *mut Node,
    pub top_nodes_len: usize,
    pub top_nodes_cap: usize,
    pub parsed_to_end: bool,

    // "Focused" top-level node and child index, if `top_child_index == SIZE_MAX`
    // the children are parsed on demand.
    pub top_node: *mut Node,
    pub top_child_index: usize,
    pub top_child: Node,
    pub has_next_child: bool,

    // Shared consecutive and all-zero index buffers
    pub zero_indices: *mut u32,
    pub consecutive_indices: *mut u32,

    // Call progress function periodically
    pub progress_timer: isize,
    pub progress_bytes_total: u64,
    pub latest_progress_bytes: u64,
    pub progress_interval: usize,

    // Extra data on the side of elements
    pub element_extra_arr: *mut *mut c_void,
    pub element_extra_cap: usize,

    // Temporary per-element flags
    pub tmp_element_flag: *mut u8,

    // IO (cold)
    pub close_fn: Option<unsafe extern "C" fn(*mut c_void)>,
    pub size_fn: Option<unsafe extern "C" fn(*mut c_void) -> u64>,

    pub ascii: Ascii,

    pub synthetic_id_counter: u64,

    pub has_geometry_transform_nodes: bool,
    pub has_scale_helper_nodes: bool,
    pub retain_vertex_w: bool,
    pub blender_full_weights: bool,

    pub mirror_axis: MirrorAxis,

    pub root: Node,

    pub scene: Scene,
    pub scene_imp: *mut SceneImp,

    pub inflate_retain: *mut InflateRetain,

    // Per-mesh consecutive indices used by `ufbxi_flip_winding()`.
    pub tmp_mesh_consecutive_indices: *mut u32,

    pub root_id: u64,
    pub num_elements: u32,

    pub legacy_node: Node,
    pub legacy_implicit_anim_layer_id: u64,

    pub file_content: *mut FileContent,
    pub num_file_content: usize,

    pub ktime_sec: i64,
    pub ktime_sec_double: f64,

    pub eof: bool,
    pub obj: ObjContext,

    pub axis_matrix: Matrix,
    pub unit_scale: Real,

    pub warnings: Warnings,

    pub deferred_failure: bool,
    pub deferred_load: bool,

    pub load_filename: *const u8,
    pub load_filename_len: usize,

    pub parse_threaded: bool,
    pub thread_pool: ThreadPool,

    pub base64_table: *mut u8,
}

// Shared handle to the parser context threaded through the load/read pipeline.
// `UnsafeCell` grants interior mutability through a shared `&Context`
// (many pointers alias the context and its embedded buffers at once);
// `MaybeUninit` drops the value-validity requirement, so `&Context` is
// sound even while `InnerContext` holds C/file-sourced bytes that are not always
// valid Rust values (partially-built `scene`/`root`/`legacy_node`, enums copied
// verbatim from options). `.get()` yields the raw `*mut InnerContext` the C-style
// body code dereferences; `#[repr(transparent)]` keeps the layout identical to
// `InnerContext`.
#[repr(transparent)]
pub(crate) struct Context(pub(crate) core::cell::UnsafeCell<core::mem::MaybeUninit<InnerContext>>);

impl Context {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerContext {
        self.0.get().cast()
    }

    // Reborrow a raw `*mut InnerContext` as `&Context` (layout-identical via
    // `repr(transparent)`). For the nullable-context (`maybe_uc`) call paths.
    // SAFETY: `ptr` must be non-null and point to a live context allocation.
    #[inline(always)]
    pub(crate) unsafe fn from_ptr<'a>(ptr: *mut InnerContext) -> &'a Context {
        &*(ptr as *const Context)
    }

    // FBX file format version (e.g. 7400). Scalar POD field: value getter +
    // setter, both safe (interior mutability via the `UnsafeCell` seam).
    #[inline(always)]
    pub(crate) fn version(&self) -> u32 {
        // SAFETY: `version` is a plain `u32` (all bit patterns valid); the
        // context is live for the borrow of `&self`.
        unsafe { (*self.get()).version }
    }

    #[inline(always)]
    pub(crate) fn set_version(&self, version: u32) {
        // SAFETY: as `version`; a `u32` store cannot violate validity.
        unsafe { (*self.get()).version = version; }
    }
}

// ufbx.c:6652-6655 `ufbxi_fail_imp`
// Expansion target of the `_msg` forms of the uc-context check macros
// (`native::error::ufbxi_check_msg!` etc.) in BOTH stack modes, and of the
// no-msg forms under `error-stack`.
#[inline(never)]
pub(crate) unsafe fn fail_imp(uc: &Context, cond: *const u8, func: *const u8, line: u32) -> i32 {
    crate::native::error::fail_imp_err(&mut (*uc.get()).error, cond, func, line)
}

// ufbx.c:6657-6662 (`#else` branch of `UFBXI_FEATURE_ERROR_STACK`)
// `ufbxi_fail_imp_no_stack` — expansion target of the no-msg uc-context check
// macros when the error stack is disabled.
#[cfg(not(feature = "error-stack"))]
#[inline(never)]
pub(crate) unsafe fn fail_imp_no_stack(uc: &Context) -> i32 {
    crate::native::error::fail_imp_err(
        &mut (*uc.get()).error,
        core::ptr::null(),
        core::ptr::null(),
        0,
    )
}

// -- Progress

// ufbx.c:6678-6681 `ufbxi_get_read_offset`
#[inline(always)]
pub(crate) unsafe fn get_read_offset(uc: &Context) -> u64 {
    (*uc.get())
        .data_offset
        .wrapping_add(to_size((*uc.get()).data.offset_from((*uc.get()).data_begin)) as u64)
}

// ufbx.c:6683-6702 `ufbxi_report_progress`
// C: `ufbxi_nodiscard static ufbxi_noinline int` — `return 1` becomes
// `Ok(())`, the `ufbxi_check_msg` failure path returns `Err(Fail)`.
#[inline(never)]
pub(crate) unsafe fn report_progress(uc: &Context) -> Result<(), Fail> {
    if (*uc.get()).opts.progress_cb.fn_.is_none() {
        return Ok(());
    }

    let read_offset: u64 = get_read_offset(uc);
    (*uc.get()).latest_progress_bytes = read_offset;

    let mut progress = Progress {
        bytes_read: 0,
        bytes_total: 0,
    };
    progress.bytes_read = read_offset;
    progress.bytes_total = (*uc.get()).progress_bytes_total;
    if progress.bytes_total < progress.bytes_read {
        progress.bytes_total = progress.bytes_read;
    }

    (*uc.get()).progress_timer = 1024;
    // C: `(uint32_t)uc->opts.progress_cb.fn(uc->opts.progress_cb.user, &progress)`
    // — the callback is `extern "C"`; the generated signature returns the enum
    // as a raw u32 (`RawEnum<ProgressResult>`).
    let result: u32 = ((*uc.get()).opts.progress_cb.fn_.unwrap_unchecked())(
        (*uc.get()).opts.progress_cb.user,
        &progress,
    )
    .as_raw();
    ufbx_assert!(
        result == ProgressResult::Continue as u32 || result == ProgressResult::Cancel as u32
    );
    ufbxi_check_msg!(
        uc,
        result != ProgressResult::Cancel as u32,
        "Cancelled",
        "result != UFBX_PROGRESS_CANCEL"
    );
    Ok(())
}

// TODO: Remove `ufbxi_unused` when it's not needed anymore
// ufbx.c:6704-6712 `ufbxi_progress` (C: `ufbxi_unused ufbxi_nodiscard static
// ufbxi_forceinline int`)
// C-parity: ufbx.c has zero call sites and silences the warning with its own
// `ufbxi_unused` marker, which maps to this item-level attribute.
#[allow(dead_code)]
#[inline(always)]
pub(crate) unsafe fn progress(uc: &Context, work_units: usize) -> Result<(), Fail> {
    if (*uc.get()).opts.progress_cb.fn_.is_none() {
        return Ok(());
    }
    // C: `uc->progress_timer - (ptrdiff_t)work_units` — signed arithmetic on
    // values that stay tiny in practice; wrapping matches the release-build
    // C behavior if it ever did overflow.
    let left: isize = (*uc.get()).progress_timer.wrapping_sub(work_units as isize);
    (*uc.get()).progress_timer = left;
    if left > 0 {
        return Ok(());
    }
    report_progress(uc)
}

// -- FBX value type information

// ufbx.c:7686-7692 `ufbxi_normalize_array_type`
pub(crate) fn normalize_array_type(type_: u8, bool_type: u8) -> u8 {
    match type_ {
        // C: `sizeof(ufbx_real) == sizeof(float) ? 'f' : 'd'`
        b'r' => {
            if size_of::<Real>() == size_of::<f32>() {
                b'f'
            } else {
                b'd'
            }
        }
        b'b' => bool_type,
        _ => type_,
    }
}

// ufbx.c:7694-7709 `ufbxi_array_type_size`
#[inline(never)]
pub(crate) fn array_type_size(type_: u8) -> usize {
    match type_ {
        b'r' => size_of::<Real>(),
        b'b' => size_of::<bool>(),
        b'c' => size_of::<u8>(),
        b'i' => size_of::<i32>(),
        b'l' => size_of::<i64>(),
        b'f' => size_of::<f32>(),
        b'd' => size_of::<f64>(),
        b's' => size_of::<String>(),
        b'S' => size_of::<String>(),
        b'C' => size_of::<String>(),
        _ => 1,
    }
}

// -- Node operations

// ufbx.c:7713-7719 `ufbxi_find_child`
#[inline(never)]
pub(crate) unsafe fn find_child(node: *mut Node, name: *const u8) -> *mut Node {
    // C: `ufbxi_for(ufbxi_node, c, node->children, node->num_children)`
    let mut c = (*node).children;
    let c_end = crate::native::platform::add_ptr(c, (*node).num_children as usize);
    while c != c_end {
        if (*c).name == name {
            return c;
        }
        c = c.add(1);
    }
    core::ptr::null_mut()
}

// Retrieve the type of a given value
// ufbx.c:7721-7725 `ufbxi_get_val_type`
#[inline(always)]
pub(crate) unsafe fn get_val_type(node: *mut Node, ix: usize) -> ValueType {
    // C: `(ufbxi_value_type)((node->value_type_mask >> (ix*2)) & 0x3)` — the
    // 2-bits-per-value tag; the mask keeps the cast in range of the enum.
    // The `as i32` reproduces C's integer promotion of the `uint16_t` mask
    // (PORTING.md checklist 8): C shifts in `int`, so amounts of 16..31 yield 0
    // instead of overflowing a 16-bit shift.
    value_type_from_raw(((((*node).value_type_mask as i32) >> (ix.wrapping_mul(2))) & 0x3) as u32)
}

// C casts the masked 2-bit field straight to `ufbxi_value_type`; Rust needs an
// explicit mapping (the four values are exhaustive after `& 0x3`).
#[inline(always)]
fn value_type_from_raw(raw: u32) -> ValueType {
    match raw {
        0 => ValueType::None,
        1 => ValueType::Number,
        2 => ValueType::String,
        _ => ValueType::Array,
    }
}

// Retrieve values from nodes with type codes:
// Any: '_' (ignore)
// NUMBER: 'I' int32_t 'L' int64_t 'F' float 'D' double 'R' ufbxi_real 'B' bool 'Z' size_t
// STRING: 'S' ufbx_string 'C' const char* (checked) 's' ufbx_string 'c' const char * (unchecked) 'b' ufbx_blob
// ufbx.c:7731-7792 `ufbxi_get_val_at`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn get_val_at(node: *mut Node, ix: usize, fmt: u8, v: *mut c_void) -> bool {
    ufbxi_dev_assert!(ix < MAX_NON_ARRAY_VALUES);
    // `as i32` mirrors C's promotion of the `uint16_t` mask to `int`.
    let type_: ValueType = value_type_from_raw(
        ((((*node).value_type_mask as i32) >> (ix.wrapping_mul(2))) & 0x3) as u32,
    );
    // `node->vals[ix]` reads the `vals` arm of the `ufbxi_node` union
    // (PORTING.md "Unions"); as in C the read happens only inside the arms that
    // need it, never for `'_'`, the `default:` arm, or a type mismatch.
    match fmt {
        b'_' => true,
        b'I' => {
            if type_ == ValueType::Number {
                *(v as *mut i32) = (*(*node).content.vals.add(ix)).num.i as i32;
                true
            } else {
                false
            }
        }
        b'L' => {
            if type_ == ValueType::Number {
                *(v as *mut i64) = (*(*node).content.vals.add(ix)).num.i as i64;
                true
            } else {
                false
            }
        }
        b'F' => {
            if type_ == ValueType::Number {
                *(v as *mut f32) = (*(*node).content.vals.add(ix)).num.f as f32;
                true
            } else {
                false
            }
        }
        b'D' => {
            if type_ == ValueType::Number {
                *(v as *mut f64) = (*(*node).content.vals.add(ix)).num.f as f64;
                true
            } else {
                false
            }
        }
        b'R' => {
            if type_ == ValueType::Number {
                *(v as *mut Real) = (*(*node).content.vals.add(ix)).num.f as Real;
                true
            } else {
                false
            }
        }
        b'B' => {
            if type_ == ValueType::Number {
                *(v as *mut bool) = (*(*node).content.vals.add(ix)).num.i != 0;
                true
            } else {
                false
            }
        }
        b'Z' => {
            if type_ == ValueType::Number {
                if (*(*node).content.vals.add(ix)).num.i < 0 {
                    return false;
                }
                *(v as *mut usize) = (*(*node).content.vals.add(ix)).num.i as usize;
                true
            } else {
                false
            }
        }
        b'S' => {
            if type_ == ValueType::String {
                let src: SanitizedString = (*(*node).content.vals.add(ix)).s;
                let dst: *mut String = v as *mut String;
                if src.utf8_length > 0 {
                    if src.utf8_length == u32::MAX {
                        return false;
                    }
                    (*dst).data = src.raw_data.add(src.raw_length as usize + 1);
                    (*dst).length = src.utf8_length as usize;
                } else {
                    (*dst).data = src.raw_data;
                    (*dst).length = src.raw_length as usize;
                }
                true
            } else {
                false
            }
        }
        b's' => {
            if type_ == ValueType::String {
                let src: SanitizedString = (*(*node).content.vals.add(ix)).s;
                let dst: *mut String = v as *mut String;
                (*dst).data = src.raw_data;
                (*dst).length = src.raw_length as usize;
                true
            } else {
                false
            }
        }
        b'C' => {
            if type_ == ValueType::String {
                let src: SanitizedString = (*(*node).content.vals.add(ix)).s;
                let dst: *mut *const u8 = v as *mut *const u8;
                if src.utf8_length > 0 {
                    if src.utf8_length == u32::MAX {
                        return false;
                    }
                    *dst = src.raw_data.add(src.raw_length as usize + 1);
                } else {
                    *dst = src.raw_data;
                }
                true
            } else {
                false
            }
        }
        b'c' => {
            if type_ == ValueType::String {
                let src: SanitizedString = (*(*node).content.vals.add(ix)).s;
                let dst: *mut *const u8 = v as *mut *const u8;
                *dst = src.raw_data;
                true
            } else {
                false
            }
        }
        b'b' => {
            if type_ == ValueType::String {
                let src: SanitizedString = (*(*node).content.vals.add(ix)).s;
                let dst: *mut Blob = v as *mut Blob;
                (*dst).data = src.raw_data;
                (*dst).size = src.raw_length as usize;
                true
            } else {
                false
            }
        }
        _ => {
            ufbxi_unreachable!("Bad format char");
            false
        }
    }
}

// ufbx.c:7794-7803 `ufbxi_get_array`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn get_array(node: *mut Node, fmt: u8) -> *mut ValueArray {
    if (*node).value_type_mask != ValueType::Array as u16 {
        return core::ptr::null_mut();
    }
    let array: *mut ValueArray = (*node).content.array;
    let mut fmt = fmt;
    if fmt != b'?' {
        fmt = normalize_array_type(fmt, b'b');
        if (*array).type_ != fmt {
            return core::ptr::null_mut();
        }
    }
    array
}

// ufbx.c:7805-7809 `ufbxi_get_val1`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn get_val1(node: *mut Node, fmt: *const u8, v0: *mut c_void) -> bool {
    if !get_val_at(node, 0, *fmt.add(0), v0) {
        return false;
    }
    true
}

// ufbx.c:7811-7816 `ufbxi_get_val2`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn get_val2(
    node: *mut Node,
    fmt: *const u8,
    v0: *mut c_void,
    v1: *mut c_void,
) -> bool {
    if !get_val_at(node, 0, *fmt.add(0), v0) {
        return false;
    }
    if !get_val_at(node, 1, *fmt.add(1), v1) {
        return false;
    }
    true
}

// ufbx.c:7818-7824 `ufbxi_get_val3`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn get_val3(
    node: *mut Node,
    fmt: *const u8,
    v0: *mut c_void,
    v1: *mut c_void,
    v2: *mut c_void,
) -> bool {
    if !get_val_at(node, 0, *fmt.add(0), v0) {
        return false;
    }
    if !get_val_at(node, 1, *fmt.add(1), v1) {
        return false;
    }
    if !get_val_at(node, 2, *fmt.add(2), v2) {
        return false;
    }
    true
}

// ufbx.c:7826-7833 `ufbxi_get_val4`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn get_val4(
    node: *mut Node,
    fmt: *const u8,
    v0: *mut c_void,
    v1: *mut c_void,
    v2: *mut c_void,
    v3: *mut c_void,
) -> bool {
    if !get_val_at(node, 0, *fmt.add(0), v0) {
        return false;
    }
    if !get_val_at(node, 1, *fmt.add(1), v1) {
        return false;
    }
    if !get_val_at(node, 2, *fmt.add(2), v2) {
        return false;
    }
    if !get_val_at(node, 3, *fmt.add(3), v3) {
        return false;
    }
    true
}

// ufbx.c:7835-7843 `ufbxi_get_val5`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn get_val5(
    node: *mut Node,
    fmt: *const u8,
    v0: *mut c_void,
    v1: *mut c_void,
    v2: *mut c_void,
    v3: *mut c_void,
    v4: *mut c_void,
) -> bool {
    if !get_val_at(node, 0, *fmt.add(0), v0) {
        return false;
    }
    if !get_val_at(node, 1, *fmt.add(1), v1) {
        return false;
    }
    if !get_val_at(node, 2, *fmt.add(2), v2) {
        return false;
    }
    if !get_val_at(node, 3, *fmt.add(3), v3) {
        return false;
    }
    if !get_val_at(node, 4, *fmt.add(4), v4) {
        return false;
    }
    true
}

// ufbx.c:7845-7851 `ufbxi_find_val1`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn find_val1(
    node: *mut Node,
    name: *const u8,
    fmt: *const u8,
    v0: *mut c_void,
) -> bool {
    let child: *mut Node = find_child(node, name);
    if child.is_null() {
        return false;
    }
    if !get_val_at(child, 0, *fmt.add(0), v0) {
        return false;
    }
    true
}

// ufbx.c:7853-7860 `ufbxi_find_val2`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn find_val2(
    node: *mut Node,
    name: *const u8,
    fmt: *const u8,
    v0: *mut c_void,
    v1: *mut c_void,
) -> bool {
    let child: *mut Node = find_child(node, name);
    if child.is_null() {
        return false;
    }
    if !get_val_at(child, 0, *fmt.add(0), v0) {
        return false;
    }
    if !get_val_at(child, 1, *fmt.add(1), v1) {
        return false;
    }
    true
}

// ufbx.c:7862-7867 `ufbxi_find_array`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn find_array(node: *mut Node, name: *const u8, fmt: u8) -> *mut ValueArray {
    let child: *mut Node = find_child(node, name);
    if child.is_null() {
        return core::ptr::null_mut();
    }
    get_array(child, fmt)
}

// ufbx.c:7869-7877 `ufbxi_find_child_strcmp`
pub(crate) unsafe fn find_child_strcmp(node: *mut Node, name: *const u8) -> *mut Node {
    let leading: u8 = *name.add(0);
    // C: `ufbxi_for(ufbxi_node, c, node->children, node->num_children)`
    let mut c = (*node).children;
    let c_end = crate::native::platform::add_ptr(c, (*node).num_children as usize);
    while c != c_end {
        if *(*c).name.add(0) != leading {
            c = c.add(1);
            continue;
        }
        if strcmp((*c).name, name) == 0 {
            return c;
        }
        c = c.add(1);
    }
    core::ptr::null_mut()
}

// -- Element extra data allocation

// ufbx.c:7881-7896 `ufbxi_push_element_extra_size`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_element_extra_size(uc: &Context, id: u32, size: usize) -> *mut c_void {
    if (*uc.get()).element_extra_cap <= id as usize {
        let old_cap: usize = (*uc.get()).element_extra_cap;
        // C: `id + 1` is `uint32_t` arithmetic before the `size_t` conversion.
        ufbxi_check_return!(
            uc,
            grow_array(
                &raw mut (*uc.get()).ator_tmp,
                &mut (*uc.get()).element_extra_arr,
                &mut (*uc.get()).element_extra_cap,
                id.wrapping_add(1) as usize
            ),
            core::ptr::null_mut(),
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->element_extra_arr)), (&uc->element_extra_arr), (&uc->element_extra_cap), (id + 1))"
        );
        core::ptr::write_bytes(
            (*uc.get()).element_extra_arr.add(old_cap) as *mut u8,
            0,
            ((*uc.get()).element_extra_cap - old_cap) * size_of::<*mut c_void>(),
        );
    }

    if !(*(*uc.get()).element_extra_arr.add(id as usize)).is_null() {
        return *(*uc.get()).element_extra_arr.add(id as usize);
    }

    let extra: *mut c_void = push_size_zero(&mut (*uc.get()).tmp, size, 1);
    ufbxi_check_return!(uc, !extra.is_null(), core::ptr::null_mut(), "extra");
    *(*uc.get()).element_extra_arr.add(id as usize) = extra;

    extra
}

// ufbx.c:7898-7905 `ufbxi_get_element_extra`
#[inline(never)]
pub(crate) unsafe fn get_element_extra(uc: &Context, id: u32) -> *mut c_void {
    if (id as usize) < (*uc.get()).element_extra_cap {
        *(*uc.get()).element_extra_arr.add(id as usize)
    } else {
        core::ptr::null_mut()
    }
}

// ufbx.c:7907 `#define ufbxi_push_element_extra(uc, id, type) (type*)ufbxi_push_element_extra_size((uc), (id), sizeof(type))`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn push_element_extra<T>(uc: &Context, id: u32) -> *mut T {
    push_element_extra_size(uc, id, size_of::<T>()) as *mut T
}

// -- Parsing state machine
//
// When reading the file we maintain a coarse representation of the structure so
// that we can resolve array info (type, included in result, etc). Using this info
// we can often read/decompress the contents directly into the right memory area.

// ufbx.c:7915-7968 `ufbxi_parse_state`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ParseState {
    Root,
    FbxHeaderExtension,
    SceneInfo,
    Thumbnail,
    Definitions,
    Objects,
    Connections,
    Relations,
    Takes,
    FbxVersion,
    Model,
    Geometry,
    NodeAttribute,
    LegacyModel,
    LegacyMedia,
    LegacyVideo,
    LegacySwitcher,
    LegacyScenePersistence,
    References,
    Reference,
    AnimationCurve,
    Deformer,
    AssociateModel,
    LegacyLink,
    Pose,
    PoseNode,
    Texture,
    Video,
    LayeredTexture,
    SelectionNode,
    Collection,
    Audio,
    UnknownObject,
    LayerElementNormal,
    LayerElementBinormal,
    LayerElementTangent,
    LayerElementUv,
    LayerElementColor,
    LayerElementVertexCrease,
    LayerElementEdgeCrease,
    LayerElementSmoothing,
    LayerElementVisibility,
    LayerElementPolygonGroup,
    LayerElementHole,
    LayerElementMaterial,
    LayerElementOther,
    GeometryUvInfo,
    Shape,
    Take,
    TakeObject,
    Channel,
    Unknown,
}

// ufbx.c:7970-7975 `ufbxi_array_flags`
// C: `typedef enum { ... } ufbxi_array_flags;` — plain bit-flag constants, held
// in the `uint8_t` field `ufbxi_array_info.flags`.
pub(crate) const ARRAY_FLAG_RESULT: u8 = 0x1; // < Allocate the array from the result buffer
pub(crate) const ARRAY_FLAG_TMP_BUF: u8 = 0x2; // < Allocate the array from the long-term temporary buffer
pub(crate) const ARRAY_FLAG_PAD_BEGIN: u8 = 0x4; // < Pad the begin of the array with 4 zero elements to guard from invalid -1 index accesses
pub(crate) const ARRAY_FLAG_ACCURATE_F32: u8 = 0x8; // < Must be parsed as bit-accurate 32-bit floats

// ufbx.c:7977-7980 `ufbxi_array_info`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ArrayInfo {
    pub type_: u8, // < FBX type code of the array: b,i,l,f,d (or 'r' meaning ufbx_real '-' ignore, 's'/'S' for strings, 'C' for content)
    pub flags: u8, // < Combination of `ufbxi_array_flags`
}

// ufbx.c:7982-8090 `ufbxi_update_parse_state`
// C compares `name` against the interned `ufbxi_*` string constants by POINTER
// (the parsers hand out pooled pointers); the `strcmp`/`strncmp` fallbacks are
// for names that have no interned constant. Both forms port verbatim.
#[inline(never)]
pub(crate) unsafe fn update_parse_state(parent: ParseState, name: *const u8) -> ParseState {
    match parent {
        ParseState::Root => {
            if name == sp::FBXHeaderExtension.as_ptr() {
                return ParseState::FbxHeaderExtension;
            }
            if name == sp::Definitions.as_ptr() {
                return ParseState::Definitions;
            }
            if name == sp::Objects.as_ptr() {
                return ParseState::Objects;
            }
            if name == sp::Connections.as_ptr() {
                return ParseState::Connections;
            }
            if name == sp::Takes.as_ptr() {
                return ParseState::Takes;
            }
            if name == sp::Model.as_ptr() {
                return ParseState::LegacyModel;
            }
            if strcmp(name, b"References\0".as_ptr()) == 0 {
                return ParseState::References;
            }
            if strcmp(name, b"Relations\0".as_ptr()) == 0 {
                return ParseState::Relations;
            }
            if name == sp::Media.as_ptr() {
                return ParseState::LegacyMedia;
            }
            if strcmp(name, b"Switcher\0".as_ptr()) == 0 {
                return ParseState::LegacySwitcher;
            }
            if strcmp(name, b"SceneGenericPersistence\0".as_ptr()) == 0 {
                return ParseState::LegacyScenePersistence;
            }
        }

        ParseState::FbxHeaderExtension => {
            if name == sp::FBXVersion.as_ptr() {
                return ParseState::FbxVersion;
            }
            if name == sp::SceneInfo.as_ptr() {
                return ParseState::SceneInfo;
            }
        }

        ParseState::SceneInfo => {
            if name == sp::Thumbnail.as_ptr() {
                return ParseState::Thumbnail;
            }
        }

        ParseState::Objects => {
            if name == sp::Model.as_ptr() {
                return ParseState::Model;
            }
            if name == sp::Geometry.as_ptr() {
                return ParseState::Geometry;
            }
            if name == sp::NodeAttribute.as_ptr() {
                return ParseState::NodeAttribute;
            }
            if name == sp::AnimationCurve.as_ptr() {
                return ParseState::AnimationCurve;
            }
            if name == sp::Deformer.as_ptr() {
                return ParseState::Deformer;
            }
            if name == sp::Pose.as_ptr() {
                return ParseState::Pose;
            }
            if name == sp::Texture.as_ptr() {
                return ParseState::Texture;
            }
            if name == sp::Video.as_ptr() {
                return ParseState::Video;
            }
            if name == sp::LayeredTexture.as_ptr() {
                return ParseState::LayeredTexture;
            }
            if name == sp::SelectionNode.as_ptr() {
                return ParseState::SelectionNode;
            }
            if name == sp::Collection.as_ptr() {
                return ParseState::Collection;
            }
            if name == sp::Audio.as_ptr() {
                return ParseState::Audio;
            }
            return ParseState::UnknownObject;
        }

        ParseState::Model | ParseState::Geometry => {
            if *name == b'L' {
                if name == sp::LayerElementNormal.as_ptr() {
                    return ParseState::LayerElementNormal;
                }
                if name == sp::LayerElementBinormal.as_ptr() {
                    return ParseState::LayerElementBinormal;
                }
                if name == sp::LayerElementTangent.as_ptr() {
                    return ParseState::LayerElementTangent;
                }
                if name == sp::LayerElementUV.as_ptr() {
                    return ParseState::LayerElementUv;
                }
                if name == sp::LayerElementColor.as_ptr() {
                    return ParseState::LayerElementColor;
                }
                if name == sp::LayerElementVertexCrease.as_ptr() {
                    return ParseState::LayerElementVertexCrease;
                }
                if name == sp::LayerElementEdgeCrease.as_ptr() {
                    return ParseState::LayerElementEdgeCrease;
                }
                if name == sp::LayerElementSmoothing.as_ptr() {
                    return ParseState::LayerElementSmoothing;
                }
                if name == sp::LayerElementVisibility.as_ptr() {
                    return ParseState::LayerElementVisibility;
                }
                if name == sp::LayerElementPolygonGroup.as_ptr() {
                    return ParseState::LayerElementPolygonGroup;
                }
                if name == sp::LayerElementHole.as_ptr() {
                    return ParseState::LayerElementHole;
                }
                if name == sp::LayerElementMaterial.as_ptr() {
                    return ParseState::LayerElementMaterial;
                }
                if strncmp(name, b"LayerElement\0".as_ptr(), 12) == 0 {
                    return ParseState::LayerElementOther;
                }
            }
            if name == sp::Shape.as_ptr() {
                return ParseState::Shape;
            }
        }

        ParseState::Deformer => {
            if strcmp(name, b"AssociateModel\0".as_ptr()) == 0 {
                return ParseState::AssociateModel;
            }
        }

        ParseState::LegacyMedia => {
            if name == sp::Video.as_ptr() {
                return ParseState::LegacyVideo;
            }
        }

        ParseState::LegacyVideo => {
            return ParseState::Video;
        }

        ParseState::LegacyModel => {
            if name == sp::GeometryUVInfo.as_ptr() {
                return ParseState::GeometryUvInfo;
            }
            if name == sp::Link.as_ptr() {
                return ParseState::LegacyLink;
            }
            if name == sp::Channel.as_ptr() {
                return ParseState::Channel;
            }
            if name == sp::Shape.as_ptr() {
                return ParseState::Shape;
            }
        }

        ParseState::Pose => {
            if name == sp::PoseNode.as_ptr() {
                return ParseState::PoseNode;
            }
        }

        ParseState::Takes => {
            if name == sp::Take.as_ptr() {
                return ParseState::Take;
            }
        }

        ParseState::Take => {
            return ParseState::TakeObject;
        }

        ParseState::TakeObject => {
            if name == sp::Channel.as_ptr() {
                return ParseState::Channel;
            }
        }

        ParseState::Channel => {
            if name == sp::Channel.as_ptr() {
                return ParseState::Channel;
            }
        }

        ParseState::References => {
            return ParseState::Reference;
        }

        _ => {}
    }

    ParseState::Unknown
}

// ufbx.c:8092-8506 `ufbxi_is_array_node`
// The `info->flags = ...` assignments OVERWRITE the `retain_dom` seed while the
// `info->flags |= ...` forms accumulate onto it — the split is load-bearing and
// ports verbatim.
pub(crate) unsafe fn is_array_node(
    uc: &Context,
    parent: ParseState,
    name: *const u8,
    info: *mut ArrayInfo,
) -> bool {
    (*info).flags = 0;

    // Retain all arrays if user wants the DOM representation
    if (*uc.get()).opts.retain_dom {
        (*info).flags |= ARRAY_FLAG_RESULT;
    }

    match parent {
        ParseState::Thumbnail => {
            if name == sp::ImageData.as_ptr() {
                (*info).type_ = b'c';
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::Geometry | ParseState::Model => {
            if name == sp::Vertices.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::PolygonVertexIndex.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::Edges.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                return true;
            } else if name == sp::Indexes.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::Points.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::KnotVector.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::KnotVectorU.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::KnotVectorV.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::PointsIndex.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::Normals.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::LegacyModel => {
            if name == sp::Vertices.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::Normals.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::Materials.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::PolygonVertexIndex.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::Children.as_ptr() {
                (*info).type_ = b's';
                return true;
            }
        }

        ParseState::AnimationCurve => {
            if name == sp::KeyTime.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_animation {
                    b'-'
                } else {
                    b'l'
                };
                return true;
            } else if name == sp::KeyValueFloat.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_animation {
                    b'-'
                } else {
                    b'r'
                };
                return true;
            } else if name == sp::KeyAttrFlags.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_animation {
                    b'-'
                } else {
                    b'i'
                };
                return true;
            } else if name == sp::KeyAttrDataFloat.as_ptr() {
                // The float data in a keyframe attribute array is represented as integers
                // in versions >= 7200 as some of the elements aren't actually floats (!)
                (*info).type_ = if (*uc.get()).from_ascii && uc.version() >= 7200 {
                    b'i'
                } else {
                    b'f'
                };
                if (*uc.get()).opts.ignore_animation {
                    (*info).type_ = b'-';
                }
                if (*uc.get()).from_ascii && uc.version() < 7200 {
                    (*info).flags |= ARRAY_FLAG_ACCURATE_F32;
                }
                return true;
            } else if name == sp::KeyAttrRefCount.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_animation {
                    b'-'
                } else {
                    b'i'
                };
                return true;
            }
        }

        ParseState::Texture => {
            if strcmp(name, b"ModelUVTranslation\0".as_ptr()) == 0
                || strcmp(name, b"ModelUVScaling\0".as_ptr()) == 0
                || strcmp(name, b"Cropping\0".as_ptr()) == 0
            {
                (*info).type_ = if (*uc.get()).opts.retain_dom {
                    b'r'
                } else {
                    b'-'
                };
                return true;
            }
        }

        ParseState::Video => {
            if name == sp::Content.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_embedded {
                    b'-'
                } else {
                    b'C'
                };
                return true;
            }
        }

        ParseState::LayeredTexture => {
            if name == sp::BlendModes.as_ptr() {
                (*info).type_ = b'i';
                (*info).flags |= ARRAY_FLAG_TMP_BUF;
                return true;
            } else if name == sp::Alphas.as_ptr() {
                (*info).type_ = b'r';
                (*info).flags |= ARRAY_FLAG_TMP_BUF;
                return true;
            }
        }

        ParseState::SelectionNode => {
            if name == sp::VertexIndexArray.as_ptr() {
                (*info).type_ = b'i';
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::EdgeIndexArray.as_ptr() {
                (*info).type_ = b'i';
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::PolygonIndexArray.as_ptr() {
                (*info).type_ = b'i';
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementNormal => {
            if name == sp::Normals.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::NormalsIndex.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::NormalsW.as_ptr() {
                (*info).type_ = if (*uc.get()).retain_vertex_w {
                    b'r'
                } else {
                    b'-'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::LayerElementBinormal => {
            if name == sp::Binormals.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::BinormalsIndex.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::BinormalsW.as_ptr() {
                (*info).type_ = if (*uc.get()).retain_vertex_w {
                    b'r'
                } else {
                    b'-'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::LayerElementTangent => {
            if name == sp::Tangents.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::TangentsIndex.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::TangentsW.as_ptr() {
                (*info).type_ = if (*uc.get()).retain_vertex_w {
                    b'r'
                } else {
                    b'-'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::LayerElementUv => {
            if name == sp::UV.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::UVIndex.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementColor => {
            if name == sp::Colors.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::ColorIndex.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementVertexCrease => {
            if name == sp::VertexCrease.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::VertexCreaseIndex.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementEdgeCrease => {
            if name == sp::EdgeCrease.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementSmoothing => {
            if name == sp::Smoothing.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'b'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementVisibility => {
            if name == sp::Visibility.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'b'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementPolygonGroup => {
            if name == sp::PolygonGroup.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementHole => {
            if name == sp::Hole.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'b'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementMaterial => {
            if name == sp::Materials.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementOther => {
            if name == sp::TextureId.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags |= ARRAY_FLAG_TMP_BUF;
                return true;
            } else if name == sp::UV.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.retain_dom {
                    b'r'
                } else {
                    b'-'
                };
                return true;
            } else if name == sp::UVIndex.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.retain_dom {
                    b'i'
                } else {
                    b'-'
                };
                return true;
            }
        }

        ParseState::GeometryUvInfo => {
            if name == sp::TextureUV.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::TextureUVVerticeIndex.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::Shape => {
            if name == sp::Indexes.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
            if name == sp::Vertices.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
            if name == sp::Normals.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::Deformer => {
            if name == sp::Transform.as_ptr() {
                (*info).type_ = b'r';
                return true;
            } else if name == sp::TransformLink.as_ptr() {
                (*info).type_ = b'r';
                return true;
            } else if name == sp::Indexes.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::Weights.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::BlendWeights.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::FullWeights.as_ptr() {
                (*info).type_ = b'r';
                (*info).flags = ((*info).flags
                    | (if (*uc.get()).blender_full_weights {
                        ARRAY_FLAG_RESULT
                    } else {
                        ARRAY_FLAG_TMP_BUF
                    })) as u8;
                return true;
            } else if strcmp(name, b"TransformAssociateModel\0".as_ptr()) == 0 {
                (*info).type_ = if (*uc.get()).opts.retain_dom {
                    b'r'
                } else {
                    b'-'
                };
                return true;
            }
        }

        ParseState::AssociateModel => {
            if name == sp::Transform.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.retain_dom {
                    b'r'
                } else {
                    b'-'
                };
                return true;
            }
        }

        ParseState::LegacyLink => {
            if name == sp::Transform.as_ptr() {
                (*info).type_ = b'r';
                return true;
            } else if name == sp::TransformLink.as_ptr() {
                (*info).type_ = b'r';
                return true;
            } else if name == sp::Indexes.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::Weights.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_geometry {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::PoseNode => {
            if name == sp::Matrix.as_ptr() {
                (*info).type_ = b'r';
                return true;
            }
        }

        ParseState::Channel => {
            if name == sp::Key.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_animation {
                    b'-'
                } else {
                    b'd'
                };
                return true;
            }
        }

        ParseState::Audio => {
            if name == sp::Content.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_embedded {
                    b'-'
                } else {
                    b'C'
                };
                return true;
            }
        }

        _ => {
            if name == sp::BinaryData.as_ptr() {
                (*info).type_ = if (*uc.get()).opts.ignore_embedded {
                    b'-'
                } else {
                    b'C'
                };
                return true;
            }
        }
    }

    false
}

// ufbx.c:8508-8606 `ufbxi_is_raw_string`
#[inline(never)]
pub(crate) unsafe fn is_raw_string(
    uc: &Context,
    parent: ParseState,
    name: *const u8,
    index: usize,
) -> bool {
    let _ = index;

    match parent {
        ParseState::Root => {
            if name == sp::Model.as_ptr() {
                return true;
            }
            if strcmp(name, b"FileId\0".as_ptr()) == 0 {
                return true;
            }
        }

        ParseState::FbxHeaderExtension => {
            if name == sp::SceneInfo.as_ptr() {
                return true;
            }
        }

        ParseState::Objects => {
            return true;
        }

        ParseState::Connections | ParseState::Relations => {
            // Pre-7000 needs raw strings for "Name\x00\x01Type" pairs, post-7000 uses it only
            // for properties that are non-raw by default.
            return uc.version() < 7000;
        }

        ParseState::Model => {
            if name == sp::NodeAttributeName.as_ptr() {
                return true;
            }
            if name == sp::Name.as_ptr() {
                return true;
            }
        }

        ParseState::Video => {
            if name == sp::Content.as_ptr() {
                return true;
            }
        }

        ParseState::Texture => {
            if strcmp(name, b"TextureName\0".as_ptr()) == 0 {
                return true;
            }
            if name == sp::Media.as_ptr() {
                return true;
            }
        }

        ParseState::Geometry => {
            if name == sp::NodeAttributeName.as_ptr() {
                return true;
            }
            if name == sp::Name.as_ptr() {
                return true;
            }
        }

        ParseState::NodeAttribute => {
            if name == sp::NodeAttributeName.as_ptr() {
                return true;
            }
            if name == sp::Name.as_ptr() {
                return true;
            }
        }

        ParseState::PoseNode => {
            if name == sp::Node.as_ptr() {
                return true;
            }
        }

        ParseState::SelectionNode => {
            if name == sp::Node.as_ptr() {
                return true;
            }
        }

        ParseState::UnknownObject => {
            if name == sp::NodeAttributeName.as_ptr() {
                return true;
            }
            if name == sp::Name.as_ptr() {
                return true;
            }
        }

        ParseState::Collection => {
            if strcmp(name, b"Member\0".as_ptr()) == 0 {
                return true;
            }
        }

        ParseState::Audio => {
            if name == sp::Content.as_ptr() {
                return true;
            }
        }

        ParseState::LegacyModel => {
            if name == sp::Material.as_ptr() {
                return true;
            }
            if name == sp::Link.as_ptr() {
                return true;
            }
            if name == sp::Name.as_ptr() {
                return true;
            }
        }

        ParseState::LegacySwitcher => {
            if strcmp(name, b"CameraIndexName\0".as_ptr()) == 0 {
                return true;
            }
        }

        ParseState::LegacyScenePersistence => {
            if name == sp::SceneInfo.as_ptr() {
                return true;
            }
        }

        ParseState::Reference => {
            if strcmp(name, b"Object\0".as_ptr()) == 0 {
                return true;
            }
        }

        ParseState::Take => {
            if name == sp::Model.as_ptr() {
                return true;
            }
        }

        _ => {}
    }

    false
}

// -- DOM retention

// ufbx.c:10698-10701 `ufbxi_dom_mapping`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct DomMapping {
    pub node_ptr: usize,
    pub dom_node: *mut DomNode,
}

// ufbx.c:10703-10710 `ufbxi_get_dom_node_imp`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn get_dom_node_imp(uc: &Context, node: *mut Node) -> *mut DomNode {
    if node.is_null() {
        return core::ptr::null_mut();
    }
    let mapping = DomMapping {
        node_ptr: node as usize,
        dom_node: core::ptr::null_mut(),
    };
    let hash = hash_uptr(mapping.node_ptr);
    let result: *mut DomMapping = map_find(
        &mut (*uc.get()).dom_node_map,
        hash,
        &mapping as *const DomMapping as *const c_void,
    );
    if !result.is_null() {
        (*result).dom_node
    } else {
        core::ptr::null_mut()
    }
}

// ufbx.c:10712-10716 `ufbxi_get_dom_node`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn get_dom_node(uc: &Context, node: *mut Node) -> *mut DomNode {
    if !(*uc.get()).opts.retain_dom {
        return core::ptr::null_mut();
    }
    get_dom_node_imp(uc, node)
}

// Recursion limited by check in ufbxi_[binary/ascii]_parse_node()
// ufbx.c:10718-10811 `ufbxi_retain_dom_node`
// `ufbxi_recursive_function(int, ufbxi_retain_dom_node, ..., UFBXI_MAX_NODE_DEPTH + 1, ...)`
// (ufbx.c:10720-10721): under regression a thread-local depth guard wraps the
// recursive body; otherwise the macro is empty and the wrapper is a plain call.
#[inline(never)]
pub(crate) unsafe fn retain_dom_node(
    uc: &Context,
    node: *mut Node,
    p_dom_node: *mut *mut DomNode,
) -> Result<(), Fail> {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|d| {
            ufbx_assert!(d.get() < MAX_NODE_DEPTH + 1);
            d.set(d.get() + 1);
        });
        let ret = retain_dom_node_rec(uc, node, p_dom_node);
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        ret
    }
    #[cfg(not(feature = "regression"))]
    {
        retain_dom_node_rec(uc, node, p_dom_node)
    }
}

#[inline(never)]
unsafe fn retain_dom_node_rec(
    uc: &Context,
    node: *mut Node,
    p_dom_node: *mut *mut DomNode,
) -> Result<(), Fail> {
    let dst: *mut DomNode = push_zero(&mut (*uc.get()).result, 1);
    ufbxi_check!(uc, !dst.is_null(), "dst");
    ufbxi_check!(
        uc,
        !push_copy::<*mut DomNode>(&mut (*uc.get()).tmp_dom_nodes, 1, &dst).is_null(),
        "((ufbx_dom_node**)ufbxi_push_size_copy((&uc->tmp_dom_nodes), sizeof(ufbx_dom_node*), (1), (&dst)))"
    );

    if !p_dom_node.is_null() {
        *p_dom_node = dst;
    }

    (*dst).name.data = (*node).name;
    (*dst).name.length = (*node).name_len as usize;

    {
        let mapping = DomMapping {
            node_ptr: node as usize,
            dom_node: core::ptr::null_mut(),
        };
        let hash = hash_uptr(mapping.node_ptr);
        let mut result: *mut DomMapping = map_find(
            &mut (*uc.get()).dom_node_map,
            hash,
            &mapping as *const DomMapping as *const c_void,
        );
        if result.is_null() {
            result = map_insert(
                &mut (*uc.get()).dom_node_map,
                hash,
                &mapping as *const DomMapping as *const c_void,
            );
            ufbxi_check!(uc, !result.is_null(), "result");
        }
        (*result).node_ptr = node as usize;
        (*result).dom_node = dst;
    }

    sp::push_string_place_str(&mut (*uc.get()).string_pool, &mut (*dst).name, false)?;

    if (*node).value_type_mask == ValueType::Array as u16 {
        let arr = (*node).content.array;
        let val: *mut DomValue = push_zero(&mut (*uc.get()).result, 1);
        ufbxi_check!(uc, !val.is_null(), "val");

        (*dst).values.data = val;
        (*dst).values.count = 1;

        let elem_size = array_type_size((*arr).type_);
        (*val).value_str.data = EMPTY_CHAR.as_ptr();
        (*val).value_blob.data = (*arr).data as *const u8;
        (*val).value_blob.size = (*arr).size.wrapping_mul(elem_size);
        // C: `val->value_float = (double)(val->value_int = (int64_t)arr->size);`
        (*val).value_int = (*arr).size as i64;
        (*val).value_float = (*val).value_int as f64;

        match (*arr).type_ {
            b'c' => (*val).type_ = DomValueType::Blob,
            b'b' => (*val).type_ = DomValueType::Blob,
            b'i' => (*val).type_ = DomValueType::ArrayI32,
            b'l' => (*val).type_ = DomValueType::ArrayI64,
            b'f' => (*val).type_ = DomValueType::ArrayF32,
            b'd' => (*val).type_ = DomValueType::ArrayF64,
            b's' => (*val).type_ = DomValueType::ArrayBlob,
            b'C' => (*val).type_ = DomValueType::ArrayBlob,
            b'-' => (*val).type_ = DomValueType::ArrayIgnored,
            _ => ufbxi_fail!(uc, "Bad array type"),
        }
    } else {
        let mut ix: usize = 0;
        while ix < MAX_NON_ARRAY_VALUES {
            // `as i32` mirrors C's promotion of the `uint16_t` mask to `int`.
            let mask = ((((*node).value_type_mask as i32) >> (2 * ix)) & 0x3) as u32;
            if mask == 0 {
                break;
            }
            let val: *mut DomValue = push_zero(&mut (*uc.get()).tmp_stack, 1);
            ufbxi_check!(uc, !val.is_null(), "val");
            (*val).value_str.data = EMPTY_CHAR.as_ptr();

            if mask == ValueType::String as u32 {
                (*val).type_ = DomValueType::String;
                ufbxi_ignore!(get_val_at(
                    node,
                    ix,
                    b'S',
                    &mut (*val).value_str as *mut String as *mut c_void
                ));
                ufbxi_ignore!(get_val_at(
                    node,
                    ix,
                    b'b',
                    &mut (*val).value_blob as *mut Blob as *mut c_void
                ));
            } else {
                ufbx_assert!(mask == ValueType::Number as u32);
                (*val).type_ = DomValueType::Number;
                // `node->vals[ix]` reads the `vals` arm of the `ufbxi_node`
                // union (PORTING.md "Unions"); both `i` and `f` of the
                // `ufbxi_value` overlay are read, as in C.
                (*val).value_int = (*(*node).content.vals.add(ix)).num.i;
                (*val).value_float = (*(*node).content.vals.add(ix)).num.f;
            }

            ix += 1;
        }

        (*dst).values.count = ix;
        (*dst).values.data =
            push_pop::<DomValue>(&mut (*uc.get()).result, &mut (*uc.get()).tmp_stack, ix);
        ufbxi_check!(uc, !(*dst).values.data.is_null(), "dst->values.data");
    }

    if (*node).num_children > 0 {
        // ufbxi_for(ufbxi_node, child, node->children, node->num_children)
        let mut child = (*node).children;
        let child_end =
            crate::native::platform::add_ptr((*node).children, (*node).num_children as usize);
        while child != child_end {
            retain_dom_node(uc, child, core::ptr::null_mut())?;
            child = child.add(1);
        }

        (*dst).children.count = (*node).num_children as usize;
        (*dst).children.data = push_pop::<*mut DomNode>(
            &mut (*uc.get()).result,
            &mut (*uc.get()).tmp_dom_nodes,
            (*node).num_children as usize,
        ) as *const Ref<DomNode>;
        ufbxi_check!(uc, !(*dst).children.data.is_null(), "dst->children.data");
    }

    Ok(())
}

// ufbx.c:10813-10844 `ufbxi_retain_toplevel`
#[inline(never)]
pub(crate) unsafe fn retain_toplevel(uc: &Context, node: *mut Node) -> Result<(), Fail> {
    if (*uc.get()).dom_parse_num_children > 0 {
        let children: *mut *mut DomNode = push_pop(
            &mut (*uc.get()).result,
            &mut (*uc.get()).tmp_dom_nodes,
            (*uc.get()).dom_parse_num_children,
        );
        ufbxi_check!(uc, !children.is_null(), "children");
        (*(*uc.get()).dom_parse_toplevel).children.data = children as *const Ref<DomNode>;
        (*(*uc.get()).dom_parse_toplevel).children.count = (*uc.get()).dom_parse_num_children;
        (*uc.get()).dom_parse_num_children = 0;
    }

    if !node.is_null() {
        retain_dom_node(uc, node, &mut (*uc.get()).dom_parse_toplevel)?;
    } else {
        (*uc.get()).dom_parse_toplevel = core::ptr::null_mut();

        // Called with NULL argument to finish retaining DOM, collect the final nodes to `ufbx_scene`.
        let num_top_nodes = (*uc.get()).tmp_dom_nodes.num_items;
        let nodes: *mut *mut DomNode = push_pop(
            &mut (*uc.get()).result,
            &mut (*uc.get()).tmp_dom_nodes,
            num_top_nodes,
        );
        ufbxi_check!(uc, !nodes.is_null(), "nodes");

        let dom_root: *mut DomNode = push_zero(&mut (*uc.get()).result, 1);
        ufbxi_check!(uc, !dom_root.is_null(), "dom_root");

        (*dom_root).name.data = EMPTY_CHAR.as_ptr();
        (*dom_root).children.data = nodes as *const Ref<DomNode>;
        (*dom_root).children.count = num_top_nodes;

        (*uc.get()).scene.dom_root = Some(Ref::from_ptr(dom_root));
    }

    Ok(())
}

// ufbx.c:10846-10853 `ufbxi_retain_toplevel_child`
#[inline(never)]
pub(crate) unsafe fn retain_toplevel_child(uc: &Context, child: *mut Node) -> Result<(), Fail> {
    ufbx_assert!(!(*uc.get()).dom_parse_toplevel.is_null());
    retain_dom_node(uc, child, core::ptr::null_mut())?;
    (*uc.get()).dom_parse_num_children = (*uc.get()).dom_parse_num_children.wrapping_add(1);

    Ok(())
}

// -- General parsing (ufbx.c:10855-11407)

// ufbx.c:10857-10879 `ufbxi_next_line`
#[inline(never)]
pub(crate) unsafe fn next_line(line: *mut String, buf: *mut String, skip_space: bool) -> bool {
    if (*buf).length == 0 {
        return false;
    }
    let newline: *const u8 = memchr((*buf).data, b'\n', (*buf).length);
    let length: usize = if !newline.is_null() {
        to_size(newline as isize - (*buf).data as isize) + 1
    } else {
        (*buf).length
    };

    (*line).data = (*buf).data;
    (*line).length = length;
    (*buf).data = (*buf).data.add(length);
    (*buf).length -= length;

    if skip_space {
        while (*line).length > 0 && is_space(*(*line).data) {
            (*line).data = (*line).data.add(1);
            (*line).length -= 1;
        }
        while (*line).length > 0 && is_space(*(*line).data.add((*line).length - 1)) {
            (*line).length -= 1;
        }
    }

    true
}

// Recursion limited by compile time patterns
// ufbx.c:10882-10914 `ufbxi_match_skip`
// `ufbxi_recursive_function(const char *, ufbxi_match_skip, (fmt, alternation), 4, ...)`
// (ufbx.c:10883-10884): under regression a thread-local depth guard wraps the
// recursive body; otherwise the macro is empty and the wrapper is a plain call.
#[inline(never)]
pub(crate) unsafe fn match_skip(fmt: *const u8, alternation: bool) -> *const u8 {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|d| {
            ufbx_assert!(d.get() < 4);
            d.set(d.get() + 1);
        });
        let ret = match_skip_rec(fmt, alternation);
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        return ret;
    }
    #[cfg(not(feature = "regression"))]
    match_skip_rec(fmt, alternation)
}

// ufbx.c:10885-10914 `ufbxi_match_skip` body (the `_rec` half of the
// `ufbxi_recursive_function` body; see the wrapper above)
#[inline(never)]
unsafe fn match_skip_rec(mut fmt: *const u8, alternation: bool) -> *const u8 {
    loop {
        // C-parity: `char c = *fmt++;` — C `char` is signed on the oracle
        // targets (PORTING.md char-value rule).
        let mut c: i8 = *(fmt as *const i8);
        fmt = fmt.add(1);
        match c as u8 {
            b'(' => {
                fmt = match_skip(fmt, false).add(1);
            }
            b'\\' => {
                fmt = fmt.add(1);
            }
            b'[' => {
                c = *(fmt as *const i8);
                while c != b']' as i8 {
                    c = *(fmt as *const i8);
                    fmt = fmt.add(1);
                    if c == b'\\' as i8 {
                        c = *(fmt as *const i8);
                        fmt = fmt.add(1);
                    }
                }
                fmt = fmt.add(1);
            }
            b'|' => {
                if alternation {
                    return fmt.offset(-1);
                }
            }
            b')' | b'\0' => {
                return fmt.offset(-1);
            }
            _ => {}
        }
    }
}

// Recursion limited by compile time patterns
// ufbx.c:10917-11084 `ufbxi_match_imp`
// `ufbxi_recursive_function(bool, ufbxi_match_imp, (p_str, end, p_fmt), 4, ...)`
// (ufbx.c:10918-10919): see `match_skip` above for the guard shape.
#[inline(never)]
pub(crate) unsafe fn match_imp(
    p_str: *mut *const u8,
    end: *const u8,
    p_fmt: *mut *const u8,
) -> bool {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|d| {
            ufbx_assert!(d.get() < 4);
            d.set(d.get() + 1);
        });
        let ret = match_imp_rec(p_str, end, p_fmt);
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        return ret;
    }
    #[cfg(not(feature = "regression"))]
    match_imp_rec(p_str, end, p_fmt)
}

// ufbx.c:10920-11084 `ufbxi_match_imp` body (the `_rec` half of the
// `ufbxi_recursive_function` body; see the wrapper above)
#[inline(never)]
unsafe fn match_imp_rec(p_str: *mut *const u8, end: *const u8, p_fmt: *mut *const u8) -> bool {
    let str_original_begin: *const u8 = *p_str;
    let mut str_: *const u8 = str_original_begin;
    let mut fmt_begin: *const u8 = *p_fmt;
    let mut fmt: *const u8 = fmt_begin;
    let mut case_insensitive: bool = false;

    let mut count: usize = 0;
    loop {
        // C-parity: `char c = *fmt++;` — signed `char` (PORTING.md char-value
        // rule); every literal compared against it is ASCII.
        let mut c: i8 = *(fmt as *const i8);
        fmt = fmt.add(1);
        if c == 0 {
            *p_str = str_;
            *p_fmt = fmt.offset(-1);
            return true;
        }

        let str_begin: *const u8 = str_;
        let mut ref_: i8 = if str_ != end { *(str_ as *const i8) } else { 0 };

        if case_insensitive {
            if ref_ >= b'A' as i8 && ref_ <= b'Z' as i8 {
                // C: `ref = (char)((int)(ref - 'A') + 'a');`
                ref_ = ((ref_ as i32 - b'A' as i32) + b'a' as i32) as i8;
            }
        }

        let mut ok: bool = false;
        match c as u8 {
            b'\\' => {
                let mut macro_: *const u8 = core::ptr::null();
                c = *(fmt as *const i8);
                fmt = fmt.add(1);
                match c as u8 {
                    b'd' => {
                        macro_ = b"[0-9]\0".as_ptr();
                    }
                    b'F' => {
                        macro_ = b"[\\-+]?[0-9]+(\\.[0-9]+)?([eE][\\-+]?[0-9]+)?\0".as_ptr();
                    }
                    b's' => {
                        if is_space(ref_ as u8) {
                            ok = true;
                            str_ = str_.add(1);
                        }
                    }
                    b'S' => {
                        if !is_space(ref_ as u8) {
                            ok = true;
                            str_ = str_.add(1);
                        }
                    }
                    b'c' | b'C' => {
                        case_insensitive = c == b'c' as i8;
                        ok = true;
                    }
                    _ => {
                        if ref_ == c {
                            ok = true;
                            str_ = str_.add(1);
                        }
                    }
                }
                if !macro_.is_null() {
                    ok = match_imp(&mut str_, end, &mut macro_);
                }
            }

            b'[' => {
                while *(fmt.add(0) as *const i8) != b']' as i8 {
                    if *(fmt.add(0) as *const i8) == b'\\' as i8 {
                        if ref_ == *(fmt.add(1) as *const i8) {
                            ok = true;
                        }
                        fmt = fmt.add(2);
                    } else if *(fmt.add(1) as *const i8) == b'-' as i8 {
                        if ref_ >= *(fmt.add(0) as *const i8) && ref_ <= *(fmt.add(2) as *const i8)
                        {
                            ok = true;
                        }
                        fmt = fmt.add(3);
                    } else {
                        if ref_ == *(fmt.add(0) as *const i8) {
                            ok = true;
                        }
                        fmt = fmt.add(1);
                    }
                }
                fmt = fmt.add(1);
                if ok {
                    str_ = str_.add(1);
                }
            }

            b'(' => {
                if match_imp(&mut str_, end, &mut fmt) {
                    ok = true;
                }
            }

            b'|' => {
                fmt = match_skip(fmt, false);
                ok = true;
            }

            b')' => {
                *p_str = str_;
                *p_fmt = fmt;
                return true;
            }

            b'.' => {
                if ref_ != 0 {
                    ok = true;
                    str_ = str_.add(1);
                }
            }

            _ => {
                if c == ref_ {
                    str_ = str_.add(1);
                    ok = true;
                }
            }
        }

        let mut did_fail: bool = false;
        c = *(fmt as *const i8);
        match c as u8 {
            b'*' => {
                fmt = fmt.add(1);
                if ok {
                    fmt = fmt_begin;
                    count += 1;
                    continue;
                }
            }
            b'+' => {
                fmt = fmt.add(1);
                if ok {
                    fmt = fmt_begin;
                    count += 1;
                    continue;
                } else if count == 0 {
                    did_fail = true;
                }
            }
            b'?' => {
                fmt = fmt.add(1);
            }
            _ => {
                did_fail = !ok;
            }
        }

        if did_fail {
            fmt = match_skip(fmt, true);
            if *fmt == b'|' {
                fmt = fmt.add(1);
                str_ = str_original_begin;
            } else {
                *p_fmt = match_skip(fmt, false).add(1);
                return false;
            }
        } else {
            if !ok {
                str_ = str_begin;
            }
        }

        fmt_begin = fmt;
        count = 0;
    }
}

// ufbx.c:11086-11094 `ufbxi_match`
#[inline(never)]
pub(crate) unsafe fn r#match(str_: *const String, fmt: *const u8) -> bool {
    let mut ptr: *const u8 = (*str_).data;
    let end: *const u8 = (*str_).data.add((*str_).length);
    let mut fmt: *const u8 = fmt;
    if match_imp(&mut ptr, end, &mut fmt) {
        ptr == end
    } else {
        false
    }
}

// ufbx.c:11096-11128 `ufbxi_is_format`
#[inline(never)]
pub(crate) unsafe fn is_format(data: *const u8, size: usize, format: FileFormat) -> bool {
    // C: `ufbx_string line, buf = { data, size };` — `line` is written by
    // `ufbxi_next_line` before any read.
    let mut line: String = String::new_c(core::ptr::null(), 0);
    let mut buf: String = String::new_c(data, size);

    if format == FileFormat::Fbx {
        if size >= BINARY_MAGIC_SIZE && memcmp(data, BINARY_MAGIC.as_ptr(), BINARY_MAGIC_SIZE) == 0
        {
            return true;
        }

        while next_line(&mut line, &mut buf, true) {
            if r#match(
                &line,
                b";\\s*FBX\\s*\\d+\\.\\d+\\.\\d+\\s*project\\s+file\0".as_ptr(),
            ) {
                return true;
            }
            if r#match(&line, b"FBXHeaderExtension:.*\0".as_ptr()) {
                return true;
            }
        }
    } else if format == FileFormat::Obj {
        while next_line(&mut line, &mut buf, true) {
            let pattern: *const u8 = b"(vn?\\s+\\F|vt)\\s+\\F\\s+\\F.*|f\\s+[\\-/0-9]+\\s+[\\-/0-9]+\\s*[\\-/0-9]+.*|(usemtl|mtllib)\\s+\\S.*\0".as_ptr();
            if r#match(&line, pattern) {
                return true;
            }
        }
    } else if format == FileFormat::Mtl {
        while next_line(&mut line, &mut buf, true) {
            let pattern: *const u8 = b"newmtl\\s+\\S.*\0".as_ptr();
            if r#match(&line, pattern) {
                return true;
            }
        }
    } else {
        ufbxi_unreachable!("Unhandled format");
    }

    false
}

// ufbx.h:3531 `UFBX_FILE_FORMAT_COUNT` (`UFBX_ENUM_TYPE` terminator for the
// enum at ufbx.h:3522-3529, consumed at ufbx.c:11147; derived from the
// generated enum's last variant so an upstream change tracks through
// regen — precedent: `WARNING_TYPE_COUNT` in `native::warnings`).
pub(crate) const FILE_FORMAT_COUNT: u32 = FileFormat::Mtl as u32 + 1;

// ufbx.c:58 `#define UFBXI_MIN_FILE_FORMAT_LOOKAHEAD 32`
pub(crate) const MIN_FILE_FORMAT_LOOKAHEAD: usize = 32;

// ufbx.c:11130-11191 `ufbxi_determine_format`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn determine_format(uc: &Context) -> Result<(), Fail> {
    let mut format: FileFormat = (*uc.get()).opts.file_format;

    if format == FileFormat::Unknown && !(*uc.get()).opts.no_format_from_content {
        crate::native::io::pause_progress(uc);

        let mut lookahead: usize = MIN_FILE_FORMAT_LOOKAHEAD;
        while format == FileFormat::Unknown && lookahead <= (*uc.get()).opts.file_format_lookahead {
            if lookahead > (*uc.get()).data_size {
                if (*uc.get()).eof {
                    break;
                }
                ufbxi_check!(
                    uc,
                    !crate::native::io::refill(uc, lookahead, false).is_null(),
                    "ufbxi_refill(uc, lookahead, false)"
                );
            }

            let data_size: usize = min_sz(lookahead, (*uc.get()).data_size);
            ufbxi_check_msg!(uc, data_size > 0, "Empty file");

            // C: `for (uint32_t fmt = UFBX_FILE_FORMAT_FBX; fmt < UFBX_FILE_FORMAT_COUNT; fmt++)`
            let mut fmt: u32 = FileFormat::Fbx as u32;
            while fmt < FILE_FORMAT_COUNT {
                if is_format(
                    (*uc.get()).data,
                    data_size,
                    core::mem::transmute::<u32, FileFormat>(fmt),
                ) {
                    format = core::mem::transmute::<u32, FileFormat>(fmt);
                    break;
                }
                fmt += 1;
            }

            if lookahead >= (*uc.get()).opts.file_format_lookahead {
                break;
            } else if lookahead < usize::MAX / 2 {
                lookahead = min_sz(lookahead * 2, (*uc.get()).opts.file_format_lookahead);
            } else {
                lookahead = usize::MAX;
            }
        }

        crate::native::io::resume_progress(uc)?;
    }

    if format == FileFormat::Unknown && !(*uc.get()).opts.no_format_from_extension {
        if (*uc.get()).opts.filename.length > 0 {
            // C: `ufbx_string extension = uc->opts.filename;`
            let mut extension: String = String::new_c(
                (*uc.get()).opts.filename.data,
                (*uc.get()).opts.filename.length,
            );
            let mut i: usize = extension.length;
            while i > 0 {
                if *extension.data.add(i - 1) == b'.' {
                    extension.data = extension.data.add(i - 1);
                    extension.length -= i - 1;
                    break;
                }
                i -= 1;
            }

            if r#match(&extension, b"\\c\\.fbx\0".as_ptr()) {
                format = FileFormat::Fbx;
            } else if r#match(&extension, b"\\c\\.obj\0".as_ptr()) {
                format = FileFormat::Obj;
            } else if r#match(&extension, b"\\c\\.mtl\0".as_ptr()) {
                format = FileFormat::Mtl;
            }
        }
    }

    ufbxi_check_msg!(
        uc,
        format != FileFormat::Unknown,
        "Unrecognized file format",
        "format != UFBX_FILE_FORMAT_UNKNOWN"
    );
    (*uc.get()).scene.metadata.file_format = format;

    Ok(())
}

// ufbx.c:11193-11240 `ufbxi_begin_parse`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn begin_parse(uc: &Context) -> Result<(), Fail> {
    let header: *const u8 = crate::native::io::peek_bytes(uc, BINARY_HEADER_SIZE);
    ufbxi_check!(uc, !header.is_null(), "header");

    // If the file starts with the binary magic parse it as binary, otherwise
    // treat it as an ASCII file.
    if memcmp(header, BINARY_MAGIC.as_ptr(), BINARY_MAGIC_SIZE) == 0 {
        // The byte after the magic indicates endianness
        let endian: u8 = *header.add(BINARY_MAGIC_SIZE + 0);
        (*uc.get()).file_big_endian = endian != 0;

        // Read the version directly from the header
        let mut version_word: *const u8 = header.add(BINARY_MAGIC_SIZE + 1);
        if (*uc.get()).file_big_endian {
            version_word =
                crate::native::parse_binary::swap_endian(uc, version_word as *const c_void, 1, 4);
            ufbxi_check!(uc, !version_word.is_null(), "version_word");
        }
        uc.set_version(read_u32(version_word));

        // This is quite probably an FBX file..
        (*uc.get()).sure_fbx = true;
        crate::native::io::consume_bytes(uc, BINARY_HEADER_SIZE);
    } else {
        (*uc.get()).from_ascii = true;

        // Use the current read buffer as the initial parse buffer
        // C: `memset(&uc->ascii, 0, sizeof(uc->ascii));`
        core::ptr::write_bytes(
            core::ptr::addr_of_mut!((*uc.get()).ascii) as *mut u8,
            0,
            size_of::<Ascii>(),
        );
        (*uc.get()).ascii.src = (*uc.get()).data;
        (*uc.get()).ascii.src_yield = (*uc.get()).data.add((*uc.get()).yield_size);
        (*uc.get()).ascii.src_end = (*uc.get())
            .data
            .add((*uc.get()).data_size + (*uc.get()).yield_size);

        // Initialize the first token
        crate::native::parse_ascii::ascii_next_token(uc, &raw mut (*uc.get()).ascii.token)?;

        // Default to version 7400 if not found in header
        if uc.version() > 0 {
            (*uc.get()).sure_fbx = true;
        } else {
            if !(*uc.get()).opts.strict {
                uc.set_version(7400);
            }
            ufbxi_check_msg!(
                uc,
                uc.version() > 0,
                "Not an FBX file",
                "uc->version > 0"
            );
        }
    }

    Ok(())
}

// ufbx.c:11242-11251 `ufbxi_parse_toplevel_child_imp`
pub(crate) unsafe fn parse_toplevel_child_imp(
    uc: &Context,
    state: ParseState,
    buf: *mut Buf,
    p_end: *mut bool,
) -> Result<(), Fail> {
    if (*uc.get()).from_ascii {
        crate::native::parse_ascii::ascii_parse_node(uc, 0, state, p_end, buf, true)?;
    } else {
        crate::native::parse_binary::binary_parse_node(uc, 0, state, p_end, buf, true)?;
    }

    Ok(())
}

// ufbx.c:11253-11330 `ufbxi_parse_toplevel`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn parse_toplevel(uc: &Context, name: *const u8) -> Result<(), Fail> {
    // C: `ufbxi_for(ufbxi_node, node, uc->top_nodes, uc->top_nodes_len)`
    let mut node: *mut Node = (*uc.get()).top_nodes;
    let node_end: *mut Node = add_ptr(node, (*uc.get()).top_nodes_len);
    while node != node_end {
        if (*node).name == name {
            (*uc.get()).top_node = node;
            (*uc.get()).top_child_index = 0;
            return Ok(());
        }
        node = node.add(1);
    }

    // Reached end and not found in cache
    if (*uc.get()).parsed_to_end {
        (*uc.get()).top_node = core::ptr::null_mut();
        (*uc.get()).top_child_index = 0;
        return Ok(());
    }

    loop {
        // Parse the next top-level node
        let mut end: bool = false;
        if (*uc.get()).from_ascii {
            crate::native::parse_ascii::ascii_parse_node(
                uc,
                0,
                ParseState::Root,
                &mut end,
                &mut (*uc.get()).tmp,
                false,
            )?;
        } else {
            crate::native::parse_binary::binary_parse_node(
                uc,
                0,
                ParseState::Root,
                &mut end,
                &mut (*uc.get()).tmp,
                false,
            )?;
        }

        // Top-level node not found
        if end {
            (*uc.get()).top_node = core::ptr::null_mut();
            (*uc.get()).top_child_index = 0;
            (*uc.get()).parsed_to_end = true;
            if (*uc.get()).opts.retain_dom {
                retain_toplevel(uc, core::ptr::null_mut())?;
            }

            // Not needed anymore
            buf_free(&mut (*uc.get()).tmp_parse);

            return Ok(());
        }

        (*uc.get()).top_nodes_len += 1;
        ufbxi_check!(
            uc,
            grow_array(
                &raw mut (*uc.get()).ator_tmp,
                &mut (*uc.get()).top_nodes,
                &mut (*uc.get()).top_nodes_cap,
                (*uc.get()).top_nodes_len
            ),
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->top_nodes)), (&uc->top_nodes), (&uc->top_nodes_cap), (uc->top_nodes_len))"
        );
        let node: *mut Node = (*uc.get()).top_nodes.add((*uc.get()).top_nodes_len - 1);
        pop::<Node>(&mut (*uc.get()).tmp_stack, 1, node);
        if (*uc.get()).opts.retain_dom {
            retain_toplevel(uc, node)?;
        }

        // Return if we parsed the right one
        if (*node).name == name {
            (*uc.get()).top_node = node;
            (*uc.get()).top_child_index = usize::MAX;
            return Ok(());
        }

        // If not we need to parse all the children of the node for later
        let mut num_children: u32 = 0;
        let state: ParseState = update_parse_state(ParseState::Root, (*node).name);
        if (*uc.get()).has_next_child {
            loop {
                parse_toplevel_child_imp(uc, state, &mut (*uc.get()).tmp, &mut end)?;
                if end {
                    break;
                }
                num_children += 1;
            }
        }

        (*node).num_children = num_children;
        (*node).children = push_pop::<Node>(
            &mut (*uc.get()).tmp,
            &mut (*uc.get()).tmp_stack,
            num_children as usize,
        );
        ufbxi_check!(uc, !(*node).children.is_null(), "node->children");

        if (*uc.get()).opts.retain_dom {
            // C: `for (size_t i = 0; i < num_children; i++)`
            let mut i: usize = 0;
            while i < num_children as usize {
                retain_toplevel_child(uc, (*node).children.add(i))?;
                i += 1;
            }
        }
    }
}

// ufbx.c:11332-11377 `ufbxi_parse_toplevel_child`
#[inline(never)]
pub(crate) unsafe fn parse_toplevel_child(
    uc: &Context,
    p_node: *mut *mut Node,
    tmp_buf: *mut Buf,
) -> Result<(), Fail> {
    // Top-level node not found
    if (*uc.get()).top_node.is_null() {
        *p_node = core::ptr::null_mut();
        return Ok(());
    }

    if (*uc.get()).top_child_index == usize::MAX {
        // Parse children on demand
        if tmp_buf.is_null() {
            buf_clear(&mut (*uc.get()).tmp_parse);
        }
        let mut end = false;
        let state: ParseState = update_parse_state(ParseState::Root, (*(*uc.get()).top_node).name);
        let buf: *mut Buf = if !tmp_buf.is_null() {
            tmp_buf
        } else {
            &mut (*uc.get()).tmp_parse
        };
        parse_toplevel_child_imp(uc, state, buf, &mut end)?;
        if end {
            *p_node = core::ptr::null_mut();
        } else {
            // Parse to either reused `uc->top_child` or push if retaining to `tmp_buf`.
            let mut dst: *mut Node = &mut (*uc.get()).top_child;
            if !tmp_buf.is_null() {
                dst = push_zero::<Node>(tmp_buf, 1);
                ufbxi_check!(uc, !dst.is_null(), "dst");
            }

            pop::<Node>(&mut (*uc.get()).tmp_stack, 1, dst);
            *p_node = dst;

            if (*uc.get()).opts.retain_dom {
                retain_toplevel_child(uc, dst)?;
            }
        }
    } else {
        // Iterate already parsed nodes
        let child_index = (*uc.get()).top_child_index;
        if child_index == (*(*uc.get()).top_node).num_children as usize {
            *p_node = core::ptr::null_mut();
        } else {
            (*uc.get()).top_child_index = (*uc.get()).top_child_index.wrapping_add(1);
            *p_node = (*(*uc.get()).top_node).children.add(child_index);
        }
    }

    Ok(())
}

// ufbx.c:11379-11407 `ufbxi_parse_legacy_toplevel`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn parse_legacy_toplevel(uc: &Context) -> Result<(), Fail> {
    ufbx_assert!((*uc.get()).top_nodes_len == 0);

    let mut end: bool = false;
    if (*uc.get()).from_ascii {
        crate::native::parse_ascii::ascii_parse_node(
            uc,
            0,
            ParseState::Root,
            &mut end,
            &mut (*uc.get()).tmp,
            true,
        )?;
    } else {
        crate::native::parse_binary::binary_parse_node(
            uc,
            0,
            ParseState::Root,
            &mut end,
            &mut (*uc.get()).tmp,
            true,
        )?;
    }

    // Top-level node not found
    if end {
        (*uc.get()).top_node = core::ptr::null_mut();
        (*uc.get()).top_child_index = 0;
        (*uc.get()).parsed_to_end = true;
        return Ok(());
    }

    pop::<Node>(&mut (*uc.get()).tmp_stack, 1, &mut (*uc.get()).legacy_node);
    (*uc.get()).top_child_index = 0;
    (*uc.get()).top_node = &mut (*uc.get()).legacy_node;

    if (*uc.get()).opts.retain_dom {
        retain_toplevel(uc, &mut (*uc.get()).legacy_node)?;
    }

    Ok(())
}

// -- Setup (ufbx.c:11409-11760)

// ufbx.c:11411-11429 `ufbxi_load_strings`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn load_strings(uc: &Context) -> Result<(), Fail> {
    // C: `#if defined(UFBX_REGRESSION) ufbx_string reg_prev = ufbx_empty_string; #endif`
    #[cfg(feature = "regression")]
    let mut reg_prev: String = crate::native::api::EMPTY_STRING.0;

    // Push all the global 'ufbxi_*' strings into the pool without copying them
    // This allows us to compare name pointers to the global values
    // C: `ufbxi_for(const ufbx_string, str, ufbxi_strings, ufbxi_arraycount(ufbxi_strings))`
    for str_ in sp::STRINGS.0.iter() {
        #[cfg(feature = "regression")]
        {
            ufbx_assert!(crate::native::error::strlen(str_.data) == str_.length);
            ufbx_assert!(sp::str_less(reg_prev, *str_));
            reg_prev = *str_;
        }
        ufbxi_check!(
            uc,
            !sp::push_string_imp(
                &mut (*uc.get()).string_pool,
                str_.data,
                str_.length,
                core::ptr::null_mut(),
                false,
                true
            )
            .is_null(),
            "ufbxi_push_string_imp(&uc->string_pool, str->data, str->length, NULL, false, true)"
        );
    }

    Ok(())
}

// ufbx.c:11431-11434 `ufbxi_prop_type_name`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PropTypeName {
    pub name: *const u8,
    pub type_: PropType,
}

// ufbx.c:11436-11468 `ufbxi_prop_type_names`
// `PropTypeName` holds a raw pointer (not auto-`Sync`); the table is immutable
// data pointing at immutable statics, so sharing is sound (precedent:
// `StringTable` in `native::string_pool`).
#[repr(transparent)]
pub(crate) struct PropTypeNameTable(pub [PropTypeName; 31]);
unsafe impl Sync for PropTypeNameTable {}

pub(crate) static PROP_TYPE_NAMES: PropTypeNameTable = PropTypeNameTable([
    PropTypeName {
        name: b"Boolean\0".as_ptr(),
        type_: PropType::Boolean,
    },
    PropTypeName {
        name: b"bool\0".as_ptr(),
        type_: PropType::Boolean,
    },
    PropTypeName {
        name: b"Bool\0".as_ptr(),
        type_: PropType::Boolean,
    },
    PropTypeName {
        name: b"Integer\0".as_ptr(),
        type_: PropType::Integer,
    },
    PropTypeName {
        name: b"int\0".as_ptr(),
        type_: PropType::Integer,
    },
    PropTypeName {
        name: b"enum\0".as_ptr(),
        type_: PropType::Integer,
    },
    PropTypeName {
        name: b"Enum\0".as_ptr(),
        type_: PropType::Integer,
    },
    PropTypeName {
        name: b"Visibility\0".as_ptr(),
        type_: PropType::Integer,
    },
    PropTypeName {
        name: b"Visibility Inheritance\0".as_ptr(),
        type_: PropType::Integer,
    },
    PropTypeName {
        name: b"KTime\0".as_ptr(),
        type_: PropType::Integer,
    },
    PropTypeName {
        name: b"Number\0".as_ptr(),
        type_: PropType::Number,
    },
    PropTypeName {
        name: b"double\0".as_ptr(),
        type_: PropType::Number,
    },
    PropTypeName {
        name: b"Real\0".as_ptr(),
        type_: PropType::Number,
    },
    PropTypeName {
        name: b"Float\0".as_ptr(),
        type_: PropType::Number,
    },
    PropTypeName {
        name: b"Intensity\0".as_ptr(),
        type_: PropType::Number,
    },
    PropTypeName {
        name: b"Vector\0".as_ptr(),
        type_: PropType::Vector,
    },
    PropTypeName {
        name: b"Vector3D\0".as_ptr(),
        type_: PropType::Vector,
    },
    PropTypeName {
        name: b"Color\0".as_ptr(),
        type_: PropType::Color,
    },
    PropTypeName {
        name: b"ColorAndAlpha\0".as_ptr(),
        type_: PropType::ColorWithAlpha,
    },
    PropTypeName {
        name: b"ColorRGB\0".as_ptr(),
        type_: PropType::Color,
    },
    PropTypeName {
        name: b"String\0".as_ptr(),
        type_: PropType::String,
    },
    PropTypeName {
        name: b"KString\0".as_ptr(),
        type_: PropType::String,
    },
    PropTypeName {
        name: b"object\0".as_ptr(),
        type_: PropType::String,
    },
    PropTypeName {
        name: b"DateTime\0".as_ptr(),
        type_: PropType::DateTime,
    },
    PropTypeName {
        name: b"Lcl Translation\0".as_ptr(),
        type_: PropType::Translation,
    },
    PropTypeName {
        name: b"Lcl Rotation\0".as_ptr(),
        type_: PropType::Rotation,
    },
    PropTypeName {
        name: b"Lcl Scaling\0".as_ptr(),
        type_: PropType::Scaling,
    },
    PropTypeName {
        name: b"Distance\0".as_ptr(),
        type_: PropType::Distance,
    },
    PropTypeName {
        name: b"Compound\0".as_ptr(),
        type_: PropType::Compound,
    },
    PropTypeName {
        name: b"Blob\0".as_ptr(),
        type_: PropType::Blob,
    },
    PropTypeName {
        name: b"Reference\0".as_ptr(),
        type_: PropType::Reference,
    },
]);

// ufbx.c:11470-11478 `ufbxi_get_prop_type`
pub(crate) unsafe fn get_prop_type(uc: &Context, name: *const u8) -> PropType {
    // C takes the address of the parameter itself (`&name`) as the map key.
    let name: *const u8 = name;
    let hash = crate::native::hash::hash_ptr!(name);
    let entry: *mut PropTypeName = map_find(
        &mut (*uc.get()).prop_type_map,
        hash,
        &name as *const *const u8 as *const c_void,
    );
    if !entry.is_null() {
        return (*entry).type_;
    }
    PropType::Unknown
}

// ufbx.c:11480-11509 `ufbxi_find_prop_with_key`
#[inline(never)]
pub(crate) unsafe fn find_prop_with_key(
    props: *const Props,
    name: *const u8,
    key: u32,
) -> *mut Prop {
    let mut props = props;
    loop {
        let prop_data: *mut Prop = (*props).props.data as *mut Prop;
        let mut begin: usize = 0;
        let mut end: usize = (*props).props.count;
        while end - begin >= 16 {
            let mid: usize = (begin + end) >> 1;
            let p: *const Prop = prop_data.add(mid);
            if (*p)._internal_key < key {
                begin = mid + 1;
            } else {
                end = mid;
            }
        }

        end = (*props).props.count;
        while begin < end {
            let p: *const Prop = prop_data.add(begin);
            if (*p)._internal_key > key {
                break;
            }
            if (*p).name.data == name && ((*p).flags.raw() & PropFlags::NO_VALUE.raw()) == 0 {
                return p as *mut Prop;
            }
            begin += 1;
        }

        props = match &(*props).defaults {
            Some(defaults) => defaults.as_ref() as *const Props,
            None => core::ptr::null(),
        };
        if props.is_null() {
            break;
        }
    }

    core::ptr::null_mut()
}

// ufbx.c:11511-11514 `ufbxi_texture_file_entry`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct TextureFileEntry {
    pub key: *const u8,
    pub file: *mut TextureFile,
}

// ufbx.c:11516-11518 `#define ufbxi_find_prop(props, name)`
// C-parity: the key is assembled from `name[0..3]` unconditionally — all call
// sites pass a `ufbxi_*` string constant of at least 4 characters. This is NOT
// `ufbxi_get_name_key()`, which handles shorter names.
#[inline(always)]
pub(crate) unsafe fn find_prop(props: *const Props, name: *const u8) -> *mut Prop {
    let key = (*name.add(0) as u32) << 24
        | (*name.add(1) as u32) << 16
        | (*name.add(2) as u32) << 8
        | (*name.add(3) as u32);
    find_prop_with_key(props, name, key)
}

// ufbx.c:11520-11528 `ufbxi_find_real`
#[inline(always)]
pub(crate) unsafe fn find_real(props: *const Props, name: *const u8, def: Real) -> Real {
    let prop: *mut Prop = find_prop(props, name);
    if !prop.is_null() {
        // C-parity: `prop->value_real` is the `ufbx_prop` value union's first
        // real; the generated struct keeps only `value_vec4` (same mapping as
        // `find_vec3` below).
        (*prop).value_vec4.x
    } else {
        def
    }
}

// ufbx.c:11530-11539 `ufbxi_find_vec3`
#[inline(always)]
pub(crate) unsafe fn find_vec3(
    props: *const Props,
    name: *const u8,
    def_x: Real,
    def_y: Real,
    def_z: Real,
) -> Vec3 {
    let prop: *mut Prop = find_prop(props, name);
    if !prop.is_null() {
        // C-parity: `prop->value_vec3` is the `ufbx_prop` value union's 3-real
        // view; the generated struct keeps only `value_vec4` (see
        // `native::read::read_property`).
        *(&(*prop).value_vec4 as *const Vec4 as *const Vec3)
    } else {
        let def = Vec3 {
            x: def_x,
            y: def_y,
            z: def_z,
        };
        def
    }
}

// ufbx.c:11541-11549 `ufbxi_find_int`
#[inline(always)]
pub(crate) unsafe fn find_int(props: *const Props, name: *const u8, def: i64) -> i64 {
    let prop: *mut Prop = find_prop(props, name);
    if !prop.is_null() {
        (*prop).value_int
    } else {
        def
    }
}

// ufbx.c:11551-11564 `ufbxi_find_enum`
// Ported with the `// -- Scene processing` unit that first needs it
// (`ufbxi_fetch_texture_layers`, ufbx.c:19251).
#[inline(always)]
pub(crate) unsafe fn find_enum(
    props: *const Props,
    name: *const u8,
    def: i64,
    max_value: i64,
) -> i64 {
    let prop: *mut Prop = find_prop(props, name);
    if !prop.is_null() {
        let value: i64 = (*prop).value_int;
        if value >= 0 && value <= max_value {
            value
        } else {
            def
        }
    } else {
        def
    }
}

// ufbx.c:11566-11572 `ufbxi_matrix_all_zero`
// C indexes the `ufbx_matrix` value union's `ufbx_real v[12]` view; the
// generated struct keeps only the named `m00`..`m23` fields (which are laid out
// in exactly that order), so the walk is pointer arithmetic from the struct
// base.
#[inline(never)]
pub(crate) unsafe fn matrix_all_zero(matrix: *const Matrix) -> bool {
    for i in 0..12 {
        if *(matrix as *const Real).add(i) != 0.0 {
            return false;
        }
    }
    true
}

// ufbx.c:11574-11577 `ufbxi_is_vec3_zero`
#[inline(always)]
pub(crate) fn is_vec3_zero(v: Vec3) -> bool {
    ((v.x == 0.0) as u8 & (v.y == 0.0) as u8 & (v.z == 0.0) as u8) != 0
}

// ufbx.c:11579-11582 `ufbxi_is_vec4_zero`
// C-parity: the `w` component is deliberately NOT tested (the C body is a
// verbatim copy of `ufbxi_is_vec3_zero`'s).
#[inline(always)]
pub(crate) fn is_vec4_zero(v: Vec4) -> bool {
    ((v.x == 0.0) as u8 & (v.y == 0.0) as u8 & (v.z == 0.0) as u8) != 0
}

// ufbx.c:11584-11587 `ufbxi_is_vec3_one`
#[inline(always)]
pub(crate) fn is_vec3_one(v: Vec3) -> bool {
    ((v.x == 1.0) as u8 & (v.y == 1.0) as u8 & (v.z == 1.0) as u8) != 0
}

// ufbx.c:11589-11592 `ufbxi_is_quat_identity`
#[inline(always)]
pub(crate) fn is_quat_identity(v: Quat) -> bool {
    ((v.x == 0.0) as u8 & (v.y == 0.0) as u8 & (v.z == 0.0) as u8 & (v.w == 1.0) as u8) != 0
}

// ufbx.c:11594-11597 `ufbxi_is_vec3_equal` (C: `ufbxi_unused`)
// C's only call site is the `ufbxi_regression_assert` at ufbx.c:22902, i.e. live
// under `UFBX_REGRESSION` (`feature = "regression"`) and stranded otherwise —
// which is exactly why C marks it `ufbxi_unused`.
#[cfg_attr(not(feature = "regression"), allow(dead_code))]
#[inline(always)]
pub(crate) fn is_vec3_equal(a: Vec3, b: Vec3) -> bool {
    ((a.x == b.x) as u8 & (a.y == b.y) as u8 & (a.z == b.z) as u8) != 0
}

// ufbx.c:11599-11602 `ufbxi_is_quat_equal` (C: `ufbxi_unused`)
// C's only call site is the `ufbxi_regression_assert` at ufbx.c:22901, i.e. live
// under `UFBX_REGRESSION` (`feature = "regression"`) and stranded otherwise —
// which is exactly why C marks it `ufbxi_unused`.
#[cfg_attr(not(feature = "regression"), allow(dead_code))]
#[inline(always)]
pub(crate) fn is_quat_equal(a: Quat, b: Quat) -> bool {
    ((a.x == b.x) as u8 & (a.y == b.y) as u8 & (a.z == b.z) as u8 & (a.w == b.w) as u8) != 0
}

// ufbx.c:11604-11607 `ufbxi_is_transform_identity`
#[inline(never)]
pub(crate) unsafe fn is_transform_identity(t: *const Transform) -> bool {
    // C: `(bool)((int)ufbxi_is_vec3_zero(..) & (int)ufbxi_is_quat_identity(..)
    // & (int)ufbxi_is_vec3_one(..))` — a non-short-circuiting bitwise `&`.
    ((is_vec3_zero((*t).translation) as i32)
        & (is_quat_identity((*t).rotation) as i32)
        & (is_vec3_one((*t).scale) as i32))
        != 0
}

// ufbx.c:11609-11622 `ufbxi_get_name_key`
#[inline(always)]
pub(crate) unsafe fn get_name_key(name: *const u8, len: usize) -> u32 {
    let mut key: u32 = 0;
    if len >= 4 {
        key = (*name.add(0) as u32) << 24
            | (*name.add(1) as u32) << 16
            | (*name.add(2) as u32) << 8
            | (*name.add(3) as u32);
    } else {
        for i in 0..4usize {
            key <<= 8;
            if i < len {
                key |= *name.add(i) as u32;
            }
        }
    }
    key
}

// ufbx.c:11624-11631 `ufbxi_get_name_key_c`
#[inline(always)]
pub(crate) unsafe fn get_name_key_c(name: *const u8) -> u32 {
    if *name.add(0) == b'\0' {
        return 0;
    }
    if *name.add(1) == b'\0' {
        return (*name.add(0) as u32) << 24;
    }
    if *name.add(2) == b'\0' {
        return (*name.add(0) as u32) << 24 | (*name.add(1) as u32) << 16;
    }
    (*name.add(0) as u32) << 24
        | (*name.add(1) as u32) << 16
        | (*name.add(2) as u32) << 8
        | (*name.add(3) as u32)
}

// ufbx.c:11633-11643 `ufbxi_name_key_less`
// Ported ahead of the rest of the `// -- Setup` section because
// `ufbxi_add_connections_to_elements` (ufbx.c:18844) needs it.
#[inline(always)]
pub(crate) unsafe fn name_key_less(
    prop: *mut Prop,
    data: *const u8,
    name_len: usize,
    key: u32,
) -> bool {
    if (*prop)._internal_key < key {
        return true;
    }
    if (*prop)._internal_key > key {
        return false;
    }

    let prop_len: usize = (*prop).name.length;
    let len: usize = min_sz(prop_len, name_len);
    let cmp: i32 = memcmp((*prop).name.data, data, len);
    if cmp != 0 {
        return cmp < 0;
    }
    prop_len < name_len
}

// ufbx.c:11645-11718 `ufbxi_node_prop_names`
// Raw-pointer table behind a `Sync` wrapper — same rationale as
// `PROP_TYPE_NAMES` above.
#[repr(transparent)]
pub(crate) struct NodePropNameTable(pub [*const u8; 72]);
unsafe impl Sync for NodePropNameTable {}

pub(crate) static NODE_PROP_NAMES: NodePropNameTable = NodePropNameTable([
    b"AxisLen\0".as_ptr(),
    b"DefaultAttributeIndex\0".as_ptr(),
    b"Freeze\0".as_ptr(),
    b"GeometricRotation\0".as_ptr(),
    b"GeometricScaling\0".as_ptr(),
    b"GeometricTranslation\0".as_ptr(),
    b"InheritType\0".as_ptr(),
    b"LODBox\0".as_ptr(),
    b"Lcl Rotation\0".as_ptr(),
    b"Lcl Scaling\0".as_ptr(),
    b"Lcl Translation\0".as_ptr(),
    b"LookAtProperty\0".as_ptr(),
    b"MaxDampRangeX\0".as_ptr(),
    b"MaxDampRangeY\0".as_ptr(),
    b"MaxDampRangeZ\0".as_ptr(),
    b"MaxDampStrengthX\0".as_ptr(),
    b"MaxDampStrengthY\0".as_ptr(),
    b"MaxDampStrengthZ\0".as_ptr(),
    b"MinDampRangeX\0".as_ptr(),
    b"MinDampRangeY\0".as_ptr(),
    b"MinDampRangeZ\0".as_ptr(),
    b"MinDampStrengthX\0".as_ptr(),
    b"MinDampStrengthY\0".as_ptr(),
    b"MinDampStrengthZ\0".as_ptr(),
    b"NegativePercentShapeSupport\0".as_ptr(),
    b"PostRotation\0".as_ptr(),
    b"PreRotation\0".as_ptr(),
    b"PreferedAngleX\0".as_ptr(),
    b"PreferedAngleY\0".as_ptr(),
    b"PreferedAngleZ\0".as_ptr(),
    b"QuaternionInterpolate\0".as_ptr(),
    b"RotationActive\0".as_ptr(),
    b"RotationMax\0".as_ptr(),
    b"RotationMaxX\0".as_ptr(),
    b"RotationMaxY\0".as_ptr(),
    b"RotationMaxZ\0".as_ptr(),
    b"RotationMin\0".as_ptr(),
    b"RotationMinX\0".as_ptr(),
    b"RotationMinY\0".as_ptr(),
    b"RotationMinZ\0".as_ptr(),
    b"RotationOffset\0".as_ptr(),
    b"RotationOrder\0".as_ptr(),
    b"RotationPivot\0".as_ptr(),
    b"RotationSpaceForLimitOnly\0".as_ptr(),
    b"RotationStiffnessX\0".as_ptr(),
    b"RotationStiffnessY\0".as_ptr(),
    b"RotationStiffnessZ\0".as_ptr(),
    b"ScalingActive\0".as_ptr(),
    b"ScalingMax\0".as_ptr(),
    b"ScalingMaxX\0".as_ptr(),
    b"ScalingMaxY\0".as_ptr(),
    b"ScalingMaxZ\0".as_ptr(),
    b"ScalingMin\0".as_ptr(),
    b"ScalingMinX\0".as_ptr(),
    b"ScalingMinY\0".as_ptr(),
    b"ScalingMinZ\0".as_ptr(),
    b"ScalingOffset\0".as_ptr(),
    b"ScalingPivot\0".as_ptr(),
    b"Show\0".as_ptr(),
    b"TranslationActive\0".as_ptr(),
    b"TranslationMax\0".as_ptr(),
    b"TranslationMaxX\0".as_ptr(),
    b"TranslationMaxY\0".as_ptr(),
    b"TranslationMaxZ\0".as_ptr(),
    b"TranslationMin\0".as_ptr(),
    b"TranslationMinX\0".as_ptr(),
    b"TranslationMinY\0".as_ptr(),
    b"TranslationMinZ\0".as_ptr(),
    b"UpVectorProperty\0".as_ptr(),
    b"Visibility Inheritance\0".as_ptr(),
    b"Visibility\0".as_ptr(),
    b"notes\0".as_ptr(),
]);

// ufbx.c:11720-11734 `ufbxi_init_node_prop_names`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn init_node_prop_names(uc: &Context) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        crate::native::hash::map_grow::<*const u8>(
            &mut (*uc.get()).node_prop_set,
            NODE_PROP_NAMES.0.len()
        ),
        "ufbxi_map_grow_size((&uc->node_prop_set), sizeof(const char*), ((sizeof(ufbxi_node_prop_names) / sizeof(*(ufbxi_node_prop_names)))))"
    );
    // C: `for (size_t i = 0; i < ufbxi_arraycount(ufbxi_node_prop_names); i++)`
    let mut i: usize = 0;
    while i < NODE_PROP_NAMES.0.len() {
        let name: *const u8 = NODE_PROP_NAMES.0[i];
        let pooled: *const u8 = sp::push_string_imp(
            &mut (*uc.get()).string_pool,
            name,
            crate::native::error::strlen(name),
            core::ptr::null_mut(),
            false,
            true,
        );
        ufbxi_check!(uc, !pooled.is_null(), "pooled");
        let hash: u32 = crate::native::hash::hash_ptr!(pooled);
        let entry: *mut *const u8 = map_insert::<*const u8>(
            &mut (*uc.get()).node_prop_set,
            hash,
            &pooled as *const *const u8 as *const c_void,
        );
        ufbxi_check!(uc, !entry.is_null(), "entry");
        *entry = pooled;
        i += 1;
    }

    Ok(())
}

// ufbx.c:11736-11744 `ufbxi_is_node_property_name`
pub(crate) unsafe fn is_node_property_name(uc: &Context, name: *const u8) -> bool {
    // You need to call `ufbxi_init_node_prop_names()` before calling this
    ufbx_assert!((*uc.get()).node_prop_set.size > 0);

    // C takes the address of the parameter itself (`&name`) as the map key.
    let name: *const u8 = name;
    let hash = crate::native::hash::hash_ptr!(name);
    let entry: *mut *const u8 = map_find(
        &mut (*uc.get()).node_prop_set,
        hash,
        &name as *const *const u8 as *const c_void,
    );
    !entry.is_null()
}

// ufbx.c:11746-11760 `ufbxi_load_maps`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn load_maps(uc: &Context) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        crate::native::hash::map_grow::<PropTypeName>(
            &mut (*uc.get()).prop_type_map,
            PROP_TYPE_NAMES.0.len()
        ),
        "ufbxi_map_grow_size((&uc->prop_type_map), sizeof(ufbxi_prop_type_name), ((sizeof(ufbxi_prop_type_names) / sizeof(*(ufbxi_prop_type_names)))))"
    );
    // C: `ufbxi_for(const ufbxi_prop_type_name, name, ufbxi_prop_type_names, ...)`
    for name in PROP_TYPE_NAMES.0.iter() {
        let pooled: *const u8 = sp::push_string_imp(
            &mut (*uc.get()).string_pool,
            name.name,
            crate::native::error::strlen(name.name),
            core::ptr::null_mut(),
            false,
            true,
        );
        ufbxi_check!(uc, !pooled.is_null(), "pooled");
        let hash: u32 = crate::native::hash::hash_ptr!(pooled);
        let entry: *mut PropTypeName = map_insert::<PropTypeName>(
            &mut (*uc.get()).prop_type_map,
            hash,
            &pooled as *const *const u8 as *const c_void,
        );
        ufbxi_check!(uc, !entry.is_null(), "entry");
        (*entry).type_ = name.type_;
        (*entry).name = pooled;
    }

    Ok(())
}

// CONTINUATION POINT: `// -- Setup` section complete (ufbx.c:11409-11760).
// Next banner: ufbx.c:11762 `// -- Reading the parsed data` (owned by
// native/read.rs).

#[cfg(test)]
mod tests {
    use super::*;

    // The C static_asserts (ufbx.c:6242/6250) are mirrored as const asserts
    // above; these runtime tests additionally pin the header-trick round trip
    // and the union sizes.
    #[test]
    fn test_get_imp_roundtrip() {
        let mut imp = core::mem::MaybeUninit::<MeshImp>::uninit();
        let imp_ptr = imp.as_mut_ptr();
        unsafe {
            let mesh_ptr = core::ptr::addr_of_mut!((*imp_ptr).mesh);
            let back: *mut MeshImp = get_imp(mesh_ptr as *mut c_void);
            assert_eq!(back, imp_ptr);
        }
    }

    #[test]
    fn test_value_union_layout() {
        // union { struct { double f; int64_t i; }; ufbxi_sanitized_string s; }
        // — the struct arm (16 bytes) dominates the size on 64-bit.
        assert_eq!(size_of::<ValueNum>(), 16);
        assert_eq!(
            size_of::<Value>(),
            size_of::<ValueNum>().max(size_of::<SanitizedString>())
        );
        assert_eq!(size_of::<NodeContent>(), size_of::<*mut c_void>());
    }

    #[test]
    fn test_progress_counts_work_units() {
        unsafe extern "C" fn cb(
            user: *mut c_void,
            progress: *const Progress,
        ) -> crate::prelude::RawEnum<ProgressResult> {
            let calls = user as *mut u32;
            unsafe {
                *calls += 1;
                assert!((*progress).bytes_total >= (*progress).bytes_read);
            }
            crate::prelude::RawEnum::from_raw(ProgressResult::Continue as u32)
        }

        // A zeroed context is what C builds via `memset` before setup; only
        // the fields `ufbxi_progress` touches need real values.
        let mut uc: std::boxed::Box<InnerContext> =
            unsafe { std::boxed::Box::new_zeroed().assume_init() };
        let mut calls: u32 = 0;
        uc.opts.progress_cb.fn_ = Some(cb);
        uc.opts.progress_cb.user = &mut calls as *mut u32 as *mut c_void;
        uc.progress_timer = 1024;
        let data = [0u8; 4];
        uc.data_begin = data.as_ptr();
        uc.data = data.as_ptr();

        unsafe {
            // Under the timer threshold: no callback.
            assert!(progress(Context::from_ptr(&raw mut *uc), 4).is_ok());
            assert_eq!(calls, 0);
            // Exhausting the timer invokes the callback and resets it to 1024.
            assert!(progress(Context::from_ptr(&raw mut *uc), 2000).is_ok());
            assert_eq!(calls, 1);
            assert_eq!(uc.progress_timer, 1024);
        }
    }

    // -- FBX value type information / node operations

    #[test]
    fn test_normalize_and_type_size() {
        // 'r' normalizes to 'f' or 'd' by `sizeof(ufbx_real)` (ufbx.c:7688).
        let real_type = if size_of::<Real>() == size_of::<f32>() {
            b'f'
        } else {
            b'd'
        };
        assert_eq!(normalize_array_type(b'r', b'b'), real_type);
        assert_eq!(normalize_array_type(b'b', b'c'), b'c');
        assert_eq!(normalize_array_type(b'i', b'b'), b'i');
        assert_eq!(array_type_size(b'r'), size_of::<Real>());
        assert_eq!(array_type_size(b'b'), size_of::<bool>());
        assert_eq!(array_type_size(b'c'), 1);
        assert_eq!(array_type_size(b'i'), 4);
        assert_eq!(array_type_size(b'l'), 8);
        assert_eq!(array_type_size(b'f'), 4);
        assert_eq!(array_type_size(b'd'), 8);
        assert_eq!(array_type_size(b'S'), size_of::<String>());
        assert_eq!(array_type_size(b'x'), 1);
    }

    #[test]
    fn test_get_val_type_mask_arithmetic() {
        let mut node: Node = unsafe { core::mem::zeroed() };
        // Values 0..3 typed NUMBER, STRING, NONE, ARRAY-tag bits.
        node.value_type_mask = 0x1 | (0x2 << 2) | (0x0 << 4) | (0x3 << 6);
        unsafe {
            assert_eq!(get_val_type(&mut node, 0), ValueType::Number);
            assert_eq!(get_val_type(&mut node, 1), ValueType::String);
            assert_eq!(get_val_type(&mut node, 2), ValueType::None);
            assert_eq!(get_val_type(&mut node, 3), ValueType::Array);
        }
    }

    #[test]
    fn test_get_val_at_number_and_string() {
        let raw = b"raw\0utf8xx\0";
        let mut vals: [Value; 2] = [
            Value {
                num: ValueNum { f: -3.5, i: -7 },
            },
            Value {
                s: SanitizedString {
                    raw_data: raw.as_ptr(),
                    raw_length: 3,
                    utf8_length: 6,
                },
            },
        ];
        let mut node: Node = unsafe { core::mem::zeroed() };
        node.value_type_mask = (ValueType::Number as u16) | ((ValueType::String as u16) << 2);
        node.content.vals = vals.as_mut_ptr();

        unsafe {
            let node = &mut node as *mut Node;
            assert!(get_val_at(node, 0, b'_', core::ptr::null_mut()));

            let mut i32v: i32 = 0;
            assert!(get_val_at(
                node,
                0,
                b'I',
                &mut i32v as *mut i32 as *mut c_void
            ));
            assert_eq!(i32v, -7);

            let mut f32v: f32 = 0.0;
            assert!(get_val_at(
                node,
                0,
                b'F',
                &mut f32v as *mut f32 as *mut c_void
            ));
            assert_eq!(f32v, -3.5f32);

            let mut boolv: bool = false;
            assert!(get_val_at(
                node,
                0,
                b'B',
                &mut boolv as *mut bool as *mut c_void
            ));
            assert!(boolv);

            // 'Z' rejects negative values without writing.
            let mut szv: usize = 123;
            assert!(!get_val_at(
                node,
                0,
                b'Z',
                &mut szv as *mut usize as *mut c_void
            ));
            assert_eq!(szv, 123);

            // Number formats reject a string value and vice versa.
            assert!(!get_val_at(
                node,
                1,
                b'I',
                &mut i32v as *mut i32 as *mut c_void
            ));
            let mut strv = String::default();
            assert!(!get_val_at(
                node,
                0,
                b'S',
                &mut strv as *mut String as *mut c_void
            ));

            // 'S' picks the sanitized UTF-8 copy at `raw_length + 1`.
            assert!(get_val_at(
                node,
                1,
                b'S',
                &mut strv as *mut String as *mut c_void
            ));
            assert_eq!(
                core::slice::from_raw_parts(strv.data, strv.length),
                b"utf8xx"
            );
            // 's' always yields the raw string.
            assert!(get_val_at(
                node,
                1,
                b's',
                &mut strv as *mut String as *mut c_void
            ));
            assert_eq!(core::slice::from_raw_parts(strv.data, strv.length), b"raw");

            let mut cstr: *const u8 = core::ptr::null();
            assert!(get_val_at(
                node,
                1,
                b'C',
                &mut cstr as *mut *const u8 as *mut c_void
            ));
            assert_eq!(cstr, raw.as_ptr().add(4));
            assert!(get_val_at(
                node,
                1,
                b'c',
                &mut cstr as *mut *const u8 as *mut c_void
            ));
            assert_eq!(cstr, raw.as_ptr());

            let mut blob: Blob = core::mem::zeroed();
            assert!(get_val_at(
                node,
                1,
                b'b',
                &mut blob as *mut Blob as *mut c_void
            ));
            assert_eq!(blob.size, 3);

            // `utf8_length == UINT32_MAX` marks an unusable sanitized string.
            (*vals.as_mut_ptr().add(1)).s.utf8_length = u32::MAX;
            assert!(!get_val_at(
                node,
                1,
                b'S',
                &mut strv as *mut String as *mut c_void
            ));
            assert!(!get_val_at(
                node,
                1,
                b'C',
                &mut cstr as *mut *const u8 as *mut c_void
            ));
        }
    }

    #[test]
    fn test_get_val_n_stops_at_first_mismatch() {
        let mut vals: [Value; 2] = [
            Value {
                num: ValueNum { f: 1.0, i: 1 },
            },
            Value {
                num: ValueNum { f: 2.0, i: 2 },
            },
        ];
        let mut node: Node = unsafe { core::mem::zeroed() };
        node.value_type_mask = ValueType::Number as u16;
        node.content.vals = vals.as_mut_ptr();
        let mut a: i64 = 0;
        let mut b: i64 = 0;
        unsafe {
            let node = &mut node as *mut Node;
            assert!(get_val1(
                node,
                b"L\0".as_ptr(),
                &mut a as *mut i64 as *mut c_void
            ));
            assert_eq!(a, 1);
            // Value 1 is untyped (NONE), so the second read fails.
            assert!(!get_val2(
                node,
                b"LL\0".as_ptr(),
                &mut a as *mut i64 as *mut c_void,
                &mut b as *mut i64 as *mut c_void
            ));
        }
    }

    #[test]
    fn test_find_child_and_arrays() {
        let name_a = b"A\0";
        let name_b = b"B\0";
        // Built at runtime so it cannot share storage with `name_b` (the
        // pooled-pointer comparison in `find_child` is address equality).
        let mut name_b_copy = [0u8; 2];
        name_b_copy[0] = b'A' + 1;
        // The concrete type 'r' normalizes to: 'f' or 'd' by `sizeof(ufbx_real)`.
        let real_type = if size_of::<Real>() == size_of::<f32>() {
            b'f'
        } else {
            b'd'
        };
        let mut array = ValueArray {
            data: core::ptr::null_mut(),
            size: 4,
            type_: real_type,
        };
        let mut children: [Node; 2] = unsafe { core::mem::zeroed() };
        children[0].name = name_a.as_ptr();
        children[1].name = name_b.as_ptr();
        children[1].value_type_mask = ValueType::Array as u16;
        children[1].content.array = &mut array;
        let mut node: Node = unsafe { core::mem::zeroed() };
        node.children = children.as_mut_ptr();
        node.num_children = 2;

        unsafe {
            let node = &mut node as *mut Node;
            assert_eq!(find_child(node, name_a.as_ptr()), children.as_mut_ptr());
            // Pointer comparison: an equal-but-unpooled name does not match.
            assert!(find_child(node, name_b_copy.as_ptr()).is_null());
            // ...while the strcmp variant does.
            assert_eq!(
                find_child_strcmp(node, name_b_copy.as_ptr()),
                children.as_mut_ptr().add(1)
            );

            assert_eq!(
                get_array(children.as_mut_ptr().add(1), real_type),
                &mut array as *mut _
            );
            // 'r' normalizes to the array's concrete type in either Real mode.
            assert_eq!(
                get_array(children.as_mut_ptr().add(1), b'r'),
                &mut array as *mut _
            );
            assert!(get_array(children.as_mut_ptr().add(1), b'i').is_null());
            // '?' skips the type check entirely.
            assert_eq!(
                get_array(children.as_mut_ptr().add(1), b'?'),
                &mut array as *mut _
            );
            // A non-array node has no array.
            assert!(get_array(children.as_mut_ptr(), b'?').is_null());

            assert_eq!(
                find_array(node, name_b.as_ptr(), real_type),
                &mut array as *mut _
            );
            assert!(find_array(node, name_a.as_ptr(), real_type).is_null());
        }
    }

    #[test]
    fn test_push_element_extra_grows_and_dedups() {
        use crate::native::allocator::init_ator;
        use crate::native::buf::buf_free;

        let mut uc: std::boxed::Box<InnerContext> =
            unsafe { std::boxed::Box::new_zeroed().assume_init() };
        unsafe {
            init_ator(
                &mut uc.error,
                &mut uc.ator_tmp,
                core::ptr::null(),
                b"test\0".as_ptr(),
            );
            uc.tmp.ator = &raw mut uc.ator_tmp;

            let uc_ptr: &Context = Context::from_ptr(&raw mut *uc);
            let a = push_element_extra_size(uc_ptr, 5, 16);
            assert!(!a.is_null());
            // The gap below `id` is zero-filled, so the untouched slots stay NULL.
            assert!(get_element_extra(uc_ptr, 0).is_null());
            // The same id returns the same allocation.
            assert_eq!(push_element_extra_size(uc_ptr, 5, 16), a);
            assert_eq!(get_element_extra(uc_ptr, 5), a);
            // Out-of-range ids read as NULL.
            assert!(get_element_extra(uc_ptr, u32::MAX).is_null());

            let b: *mut TmpBonePose = push_element_extra(uc_ptr, 6);
            assert!(!b.is_null());
            assert_eq!((*b).bone_fbx_id, 0);
            assert_eq!(get_element_extra(uc_ptr, 6), b as *mut c_void);

            buf_free(&mut uc.tmp);
            crate::native::allocator::free_size(
                &mut uc.ator_tmp,
                size_of::<*mut c_void>(),
                uc.element_extra_arr as *mut c_void,
                uc.element_extra_cap,
            );
        }
    }

    #[test]
    fn test_report_progress_cancel_sets_error() {
        unsafe extern "C" fn cancel_cb(
            _user: *mut c_void,
            _progress: *const Progress,
        ) -> crate::prelude::RawEnum<ProgressResult> {
            crate::prelude::RawEnum::from_raw(ProgressResult::Cancel as u32)
        }

        let mut uc: std::boxed::Box<InnerContext> =
            unsafe { std::boxed::Box::new_zeroed().assume_init() };
        uc.opts.progress_cb.fn_ = Some(cancel_cb);
        let data = [0u8; 1];
        uc.data_begin = data.as_ptr();
        uc.data = data.as_ptr();

        unsafe {
            assert_eq!(report_progress(Context::from_ptr(&raw mut *uc)), Err(Fail));
            let desc =
                core::slice::from_raw_parts(uc.error.description.data, uc.error.description.length);
            assert_eq!(desc, b"Cancelled");
        }
    }

    #[test]
    fn test_retain_dom_node_tree() {
        use crate::native::allocator::init_ator;
        use crate::native::buf::buf_free;
        use crate::native::hash::{map_cmp_uintptr, map_free, map_init};
        use crate::native::string_pool::{map_cmp_string, string_pool_temp_free};

        let mut uc: std::boxed::Box<InnerContext> =
            unsafe { std::boxed::Box::new_zeroed().assume_init() };
        unsafe {
            init_ator(
                &mut uc.error,
                &mut uc.ator_tmp,
                core::ptr::null(),
                b"test\0".as_ptr(),
            );
            init_ator(
                &mut uc.error,
                &mut uc.ator_result,
                core::ptr::null(),
                b"test\0".as_ptr(),
            );
            let ator_tmp: *mut Allocator = &mut uc.ator_tmp;
            uc.result.ator = &raw mut uc.ator_result;
            uc.tmp_stack.ator = ator_tmp;
            uc.tmp_dom_nodes.ator = ator_tmp;
            uc.string_pool.error = &mut uc.error;
            uc.string_pool.buf.ator = ator_tmp;
            uc.string_pool.initial_size = 64;
            map_init(
                &mut uc.string_pool.map,
                ator_tmp,
                map_cmp_string,
                core::ptr::null_mut(),
            );
            map_init(
                &mut uc.dom_node_map,
                ator_tmp,
                map_cmp_uintptr,
                core::ptr::null_mut(),
            );

            // Root node with one number value and one array child.
            let name_root = b"Root\0";
            let name_leaf = b"Leaf\0";
            let mut data: [i32; 3] = [1, 2, 3];
            let mut array = ValueArray {
                data: data.as_mut_ptr() as *mut c_void,
                size: 3,
                type_: b'i',
            };
            let mut leaf: Node = core::mem::zeroed();
            leaf.name = name_leaf.as_ptr();
            leaf.name_len = 4;
            leaf.value_type_mask = ValueType::Array as u16;
            leaf.content.array = &mut array;

            let mut vals: [Value; 1] = [Value {
                num: ValueNum { f: 2.0, i: 2 },
            }];
            let mut root: Node = core::mem::zeroed();
            root.name = name_root.as_ptr();
            root.name_len = 4;
            root.value_type_mask = ValueType::Number as u16;
            root.content.vals = vals.as_mut_ptr();
            root.children = &mut leaf;
            root.num_children = 1;

            let uc_ptr: &Context = Context::from_ptr(&raw mut *uc);
            let mut dom: *mut DomNode = core::ptr::null_mut();
            assert_eq!(retain_dom_node(uc_ptr, &mut root, &mut dom), Ok(()));
            assert!(!dom.is_null());

            // The node name is interned; values and children are materialized.
            assert_eq!(
                core::slice::from_raw_parts((*dom).name.data, (*dom).name.length),
                b"Root"
            );
            assert_eq!((*dom).values.count, 1);
            let val = (*dom).values.data;
            assert_eq!((*val).type_ as u32, DomValueType::Number as u32);
            assert_eq!((*val).value_int, 2);
            assert_eq!((*val).value_float, 2.0);
            assert_eq!((*dom).children.count, 1);

            let child = *((*dom).children.data as *const *mut DomNode);
            assert_eq!((*child).values.count, 1);
            let cval = (*child).values.data;
            assert_eq!((*cval).type_ as u32, DomValueType::ArrayI32 as u32);
            assert_eq!((*cval).value_int, 3);
            assert_eq!((*cval).value_blob.size, 3 * size_of::<i32>());

            // `ufbxi_get_dom_node` is gated on `opts.retain_dom`; the mapping
            // itself is populated regardless.
            assert!(get_dom_node(uc_ptr, &mut root).is_null());
            uc.opts.retain_dom = true;
            assert_eq!(get_dom_node(uc_ptr, &mut root), dom);
            assert_eq!(get_dom_node(uc_ptr, &mut leaf), child);
            assert!(get_dom_node(uc_ptr, core::ptr::null_mut()).is_null());

            buf_free(&mut uc.result);
            buf_free(&mut uc.tmp_stack);
            buf_free(&mut uc.tmp_dom_nodes);
            buf_free(&mut uc.string_pool.buf);
            string_pool_temp_free(&mut uc.string_pool);
            map_free(&mut uc.dom_node_map);
            assert_eq!(uc.ator_tmp.current_size, 0);
            assert_eq!(uc.ator_result.current_size, 0);
        }
    }
}
