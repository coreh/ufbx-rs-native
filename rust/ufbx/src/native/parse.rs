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
use crate::native::view::{Mode, SliceViewIter, View};
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

// Reinterpret-in-place view over an arena-allocated `Node` (Rust-port
// infrastructure; see `native::view`). ufbx materializes each node's children as
// a contiguous `push_pop` run walked in C by `ufbxi_for`, so `SliceViewIter` over
// `(children, num_children)` is the safe navigation form. `View<Node>` supplies
// `get()` / `from_ptr()`; the accessors below are the per-struct residue. `name`
// is POOLED — it is compared with `==` by pointer value and never dereferenced.
pub(crate) type NodeView = crate::native::view::View<Node>;

impl NodeView {
    #[inline(always)]
    pub(crate) fn name(&self) -> *const u8 {
        // SAFETY: reading the pooled `name` pointer field of a valid arena `Node`.
        unsafe { (*self.get()).name }
    }
    #[inline(always)]
    pub(crate) fn num_children(&self) -> u32 {
        // SAFETY: reading a `u32` count field of a valid arena `Node`.
        unsafe { (*self.get()).num_children }
    }
    #[inline(always)]
    pub(crate) fn name_len(&self) -> u8 {
        // SAFETY: reading a `u8` length field of a valid arena `Node`.
        unsafe { (*self.get()).name_len }
    }
    #[inline(always)]
    pub(crate) fn value_type_mask(&self) -> u16 {
        // SAFETY: reading a `u16` scalar field of a valid arena `Node`.
        unsafe { (*self.get()).value_type_mask }
    }
    #[inline(always)]
    pub(crate) fn children(&self) -> *mut Node {
        // SAFETY: reading the `children` run pointer of a valid arena `Node`.
        unsafe { (*self.get()).children }
    }
    #[inline(always)]
    pub(crate) fn vals(&self) -> *mut Value {
        // SAFETY: reading the `vals` arm of the `content` union of a valid arena
        // `Node` (PORTING.md "Unions"); a raw pointer, all bit patterns valid.
        unsafe { (*self.get()).content.vals }
    }
    #[inline(always)]
    pub(crate) fn array(&self) -> *mut ValueArray {
        // SAFETY: reading the `array` arm of the `content` union of a valid arena
        // `Node` (PORTING.md "Unions"); a raw pointer, all bit patterns valid.
        unsafe { (*self.get()).content.array }
    }
}

// Rust-port infrastructure (not a ufbx.c section): views over the property
// tables the finders walk. `Props` is the sorted `ufbx_prop` table plus a
// `defaults` chain; `Prop` is a single entry. The finders (`ufbxi_find_prop` /
// `ufbx_find_prop_len` and their wrappers) navigate a `&PropsView` and return a
// `&PropView` correlated to it, so a found entry never outlives the table it was
// found in. The binary-search / key-hash mechanics stay raw inside the leaves
// (they are the leaf); the views supply the safe navigation surface.
pub(crate) type PropsView = View<Props>;
pub(crate) type PropView = View<Prop>;

// Mode-generic (`M: Mode`): the finders serve internal `Mut` views AND
// public-boundary `Const` views minted from a caller's `&Props` — these
// accessors only read, so one body serves both modes via `as_ptr()`.
impl<M: Mode> View<Props, M> {
    #[inline(always)]
    pub(crate) fn props_data(&self) -> *mut Prop {
        // SAFETY: reading the `props.data` run pointer of a valid arena `Props`.
        unsafe { (*self.as_ptr()).props.data as *mut Prop }
    }
    #[inline(always)]
    pub(crate) fn props_count(&self) -> usize {
        // SAFETY: reading the `props.count` field of a valid arena `Props`.
        unsafe { (*self.as_ptr()).props.count }
    }
    /// The `defaults` fallback table, viewed with the same lifetime and MODE as
    /// `self` (a chain rooted `Const` stays `Const`).
    #[inline(always)]
    pub(crate) fn defaults(&self) -> Option<&View<Props, M>> {
        // SAFETY: reads the `defaults: Option<Ref<Props>>` field as its
        // niche-packed bare pointer (like `read::opt_ptr`), NOT through
        // `Ref::as_ref`. `as_ref` would form a SharedReadOnly `&Props` and then
        // reinterpreting THAT as an interior-mutable view retags for write and
        // trips Stacked Borrows; reading the pointer bits keeps the STORED
        // provenance (write-capable for arena tables), which is adequate for
        // either mode. The viewed table lives as long as `self`.
        unsafe {
            let defaults_ptr: *mut Props =
                *(&raw const (*self.as_ptr()).defaults as *const *mut Props);
            if defaults_ptr.is_null() {
                None
            } else {
                Some(View::<Props, M>::mint(defaults_ptr))
            }
        }
    }
}

impl<M: Mode> View<Prop, M> {
    #[inline(always)]
    pub(crate) fn value_vec4(&self) -> Vec4 {
        // SAFETY: reading the `value_vec4` field of a valid arena `Prop`.
        unsafe { (*self.as_ptr()).value_vec4 }
    }
    #[inline(always)]
    pub(crate) fn value_vec3(&self) -> Vec3 {
        // C-parity: the `ufbx_prop` value union's 3-real view; the generated
        // struct keeps only `value_vec4` (same mapping as `find_vec3`).
        // SAFETY: reading the first three reals of a valid arena `Prop`.
        unsafe { *(&(*self.as_ptr()).value_vec4 as *const Vec4 as *const Vec3) }
    }
    #[inline(always)]
    pub(crate) fn value_int(&self) -> i64 {
        // SAFETY: reading the `value_int` field of a valid arena `Prop`.
        unsafe { (*self.as_ptr()).value_int }
    }
    #[inline(always)]
    pub(crate) fn value_str(&self) -> String {
        // SAFETY: reading the `value_str` field of a valid arena `Prop`.
        unsafe { (*self.as_ptr()).value_str }
    }
    #[inline(always)]
    pub(crate) fn value_blob(&self) -> Blob {
        // SAFETY: reading the `value_blob` field of a valid arena `Prop`.
        unsafe { (*self.as_ptr()).value_blob }
    }
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        // SAFETY: reading the `name` field of a valid arena `Prop`.
        unsafe { (*self.as_ptr()).name }
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> PropType {
        // SAFETY: reading the `type_` field of a valid arena `Prop`.
        unsafe { (*self.as_ptr()).type_ }
    }
    #[inline(always)]
    pub(crate) fn flags(&self) -> PropFlags {
        // SAFETY: reading the `flags` field of a valid arena `Prop`.
        unsafe { (*self.as_ptr()).flags }
    }
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
pub(crate) fn get_imp<T>(ptr: *mut c_void) -> *mut T {
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

// Typed interior-mutable VIEW over an `AsciiToken` field, reinterpreted in place.
pub(crate) type AsciiTokenView = crate::native::view::View<AsciiToken>;

impl AsciiTokenView {
    #[inline(always)]
    pub(crate) fn str_data(&self) -> *mut u8 {
        unsafe { (*self.get()).str_data }
    }
    #[inline(always)]
    pub(crate) fn str_cap(&self) -> usize {
        unsafe { (*self.get()).str_cap }
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> u8 {
        unsafe { (*self.get()).type_ }
    }
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

// Typed interior-mutable VIEW over the owned `ascii` field (Ascii), reinterpreted in
// place. `token`/`prev_token` recurse into `AsciiTokenView`; scalars are getters/setters;
// `token` is also whole-addr'd (`_mut_ptr`).
pub(crate) type AsciiView = crate::native::view::View<Ascii>;

impl AsciiView {
    #[inline(always)]
    pub(crate) fn token_view(&self) -> &AsciiTokenView {
        unsafe { &*(&raw mut (*self.get()).token as *mut AsciiTokenView) }
    }
    #[inline(always)]
    pub(crate) fn prev_token_view(&self) -> &AsciiTokenView {
        unsafe { &*(&raw mut (*self.get()).prev_token as *mut AsciiTokenView) }
    }
    #[inline(always)]
    pub(crate) fn token_mut_ptr(&self) -> *mut AsciiToken {
        unsafe { &raw mut (*self.get()).token }
    }
    #[inline(always)]
    pub(crate) fn src_buf(&self) -> *mut Buf {
        unsafe { (*self.get()).src_buf }
    }
    #[inline(always)]
    pub(crate) fn set_src_buf(&self, src_buf: *mut Buf) {
        unsafe {
            (*self.get()).src_buf = src_buf;
        }
    }
    #[inline(always)]
    pub(crate) fn retain_buf(&self) -> *mut Buf {
        unsafe { (*self.get()).retain_buf }
    }
    #[inline(always)]
    pub(crate) fn src(&self) -> *const u8 {
        unsafe { (*self.get()).src }
    }
    #[inline(always)]
    pub(crate) fn src_yield(&self) -> *const u8 {
        unsafe { (*self.get()).src_yield }
    }
    #[inline(always)]
    pub(crate) fn src_end(&self) -> *const u8 {
        unsafe { (*self.get()).src_end }
    }
    #[inline(always)]
    pub(crate) fn read_first_comment(&self) -> bool {
        unsafe { (*self.get()).read_first_comment }
    }
    #[inline(always)]
    pub(crate) fn set_read_first_comment(&self, read_first_comment: bool) {
        unsafe {
            (*self.get()).read_first_comment = read_first_comment;
        }
    }
    #[inline(always)]
    pub(crate) fn set_src(&self, src: *const u8) {
        unsafe {
            (*self.get()).src = src;
        }
    }
    #[inline(always)]
    pub(crate) fn set_src_yield(&self, src_yield: *const u8) {
        unsafe {
            (*self.get()).src_yield = src_yield;
        }
    }
    #[inline(always)]
    pub(crate) fn set_src_end(&self, src_end: *const u8) {
        unsafe {
            (*self.get()).src_end = src_end;
        }
    }
    #[inline(always)]
    pub(crate) fn set_src_is_retained(&self, src_is_retained: bool) {
        unsafe {
            (*self.get()).src_is_retained = src_is_retained;
        }
    }
    #[inline(always)]
    pub(crate) fn set_found_version(&self, found_version: bool) {
        unsafe {
            (*self.get()).found_version = found_version;
        }
    }
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
pub(crate) struct InnerObjContext {
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

// Safe `&ObjContext` handle over the fields-struct `InnerObjContext`, mirroring the
// `Context`/`InnerContext` seam. ObjContext is the embedded `Context.obj` sub-context
// (ufbxi_obj_context); reached via `Context::obj()` (a field, not a threaded param).
#[repr(transparent)]
pub(crate) struct ObjContext(core::cell::UnsafeCell<core::mem::MaybeUninit<InnerObjContext>>);

impl ObjContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerObjContext {
        self.0.get().cast()
    }

    #[inline(always)]
    pub(crate) fn group(&self) -> String {
        // SAFETY: Copy String/Blob value read.
        unsafe { (*self.get()).group }
    }
    #[inline(always)]
    pub(crate) fn set_group(&self, group: String) {
        unsafe {
            (*self.get()).group = group;
        }
    }
    #[inline(always)]
    pub(crate) fn group_mut_ptr(&self) -> *mut String {
        unsafe { &raw mut (*self.get()).group }
    }

    #[inline(always)]
    pub(crate) fn set_object(&self, object: String) {
        unsafe {
            (*self.get()).object = object;
        }
    }
    #[inline(always)]
    pub(crate) fn object_mut_ptr(&self) -> *mut String {
        unsafe { &raw mut (*self.get()).object }
    }

    #[inline(always)]
    pub(crate) fn line_mut_ptr(&self) -> *mut String {
        unsafe { &raw mut (*self.get()).line }
    }

    #[inline(always)]
    pub(crate) fn mtllib_relative_path(&self) -> crate::prelude::Blob {
        // SAFETY: Copy String/Blob value read.
        unsafe { (*self.get()).mtllib_relative_path }
    }
    #[inline(always)]
    pub(crate) fn mtllib_relative_path_mut_ptr(&self) -> *mut crate::prelude::Blob {
        unsafe { &raw mut (*self.get()).mtllib_relative_path }
    }

    #[inline(always)]
    pub(crate) fn group_view(&self) -> &crate::prelude::StringView {
        unsafe { &*(&raw mut (*self.get()).group as *mut crate::prelude::StringView) }
    }

    #[inline(always)]
    pub(crate) fn object_view(&self) -> &crate::prelude::StringView {
        unsafe { &*(&raw mut (*self.get()).object as *mut crate::prelude::StringView) }
    }

    #[inline(always)]
    pub(crate) fn line_view(&self) -> &crate::prelude::StringView {
        unsafe { &*(&raw mut (*self.get()).line as *mut crate::prelude::StringView) }
    }

    #[inline(always)]
    pub(crate) fn mtllib_relative_path_view(&self) -> &crate::prelude::BlobView {
        unsafe { &*(&raw mut (*self.get()).mtllib_relative_path as *mut crate::prelude::BlobView) }
    }

    #[inline(always)]
    pub(crate) fn group_map_mut_ptr(&self) -> *mut Map {
        unsafe { &raw mut (*self.get()).group_map }
    }

    #[inline(always)]
    pub(crate) fn tmp_color_valid_mut_ptr(&self) -> *mut crate::native::buf::Buf {
        unsafe { &raw mut (*self.get()).tmp_color_valid }
    }
    #[inline(always)]
    pub(crate) fn tmp_color_valid_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_color_valid as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_faces_mut_ptr(&self) -> *mut crate::native::buf::Buf {
        unsafe { &raw mut (*self.get()).tmp_faces }
    }
    #[inline(always)]
    pub(crate) fn tmp_faces_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_faces as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_props_mut_ptr(&self) -> *mut crate::native::buf::Buf {
        unsafe { &raw mut (*self.get()).tmp_props }
    }
    #[inline(always)]
    pub(crate) fn tmp_props_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_props as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_meshes_mut_ptr(&self) -> *mut crate::native::buf::Buf {
        unsafe { &raw mut (*self.get()).tmp_meshes }
    }
    #[inline(always)]
    pub(crate) fn tmp_meshes_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_meshes as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_face_smoothing_mut_ptr(&self) -> *mut crate::native::buf::Buf {
        unsafe { &raw mut (*self.get()).tmp_face_smoothing }
    }
    #[inline(always)]
    pub(crate) fn tmp_face_smoothing_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_face_smoothing as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_face_group_mut_ptr(&self) -> *mut crate::native::buf::Buf {
        unsafe { &raw mut (*self.get()).tmp_face_group }
    }
    #[inline(always)]
    pub(crate) fn tmp_face_group_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_face_group as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_face_group_infos_mut_ptr(&self) -> *mut crate::native::buf::Buf {
        unsafe { &raw mut (*self.get()).tmp_face_group_infos }
    }
    #[inline(always)]
    pub(crate) fn tmp_face_group_infos_view(&self) -> &crate::native::buf::BufView {
        unsafe {
            &*(&raw mut (*self.get()).tmp_face_group_infos as *mut crate::native::buf::BufView)
        }
    }

    #[inline(always)]
    pub(crate) fn tmp_face_material_mut_ptr(&self) -> *mut crate::native::buf::Buf {
        unsafe { &raw mut (*self.get()).tmp_face_material }
    }
    #[inline(always)]
    pub(crate) fn tmp_face_material_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_face_material as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tokens_mut_ptr(&self) -> *mut *mut String {
        unsafe { &raw mut (*self.get()).tokens }
    }

    #[inline(always)]
    pub(crate) fn tokens_cap_mut_ptr(&self) -> *mut usize {
        unsafe { &raw mut (*self.get()).tokens_cap }
    }

    #[inline(always)]
    pub(crate) fn tmp_materials_mut_ptr(&self) -> *mut *mut *mut crate::generated::Material {
        unsafe { &raw mut (*self.get()).tmp_materials }
    }

    #[inline(always)]
    pub(crate) fn tmp_materials_cap_mut_ptr(&self) -> *mut usize {
        unsafe { &raw mut (*self.get()).tmp_materials_cap }
    }

    #[inline(always)]
    pub(crate) fn usemtl_index(&self) -> u32 {
        // SAFETY: reading a scalar; all bit patterns of `u32` are valid.
        unsafe { (*self.get()).usemtl_index }
    }

    #[inline(always)]
    pub(crate) fn set_usemtl_index(&self, usemtl_index: u32) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).usemtl_index = usemtl_index;
        }
    }

    #[inline(always)]
    pub(crate) fn usemtl_fbx_id(&self) -> u64 {
        // SAFETY: reading a scalar; all bit patterns of `u64` are valid.
        unsafe { (*self.get()).usemtl_fbx_id }
    }

    #[inline(always)]
    pub(crate) fn set_usemtl_fbx_id(&self, usemtl_fbx_id: u64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).usemtl_fbx_id = usemtl_fbx_id;
        }
    }

    #[inline(always)]
    pub(crate) fn tokens_cap(&self) -> usize {
        // SAFETY: reading a scalar; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).tokens_cap }
    }

    #[inline(always)]
    pub(crate) fn tokens(&self) -> *mut String {
        // SAFETY: reading a scalar; all bit patterns of `*mut String` are valid.
        unsafe { (*self.get()).tokens }
    }

    #[inline(always)]
    pub(crate) fn tmp_materials_cap(&self) -> usize {
        // SAFETY: reading a scalar; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).tmp_materials_cap }
    }

    #[inline(always)]
    pub(crate) fn tmp_materials(&self) -> *mut *mut crate::generated::Material {
        // SAFETY: reading a scalar; all bit patterns of `*mut *mut crate::generated::Material` are valid.
        unsafe { (*self.get()).tmp_materials }
    }

    #[inline(always)]
    pub(crate) fn eof(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).eof }
    }

    #[inline(always)]
    pub(crate) fn set_eof(&self, eof: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).eof = eof;
        }
    }

    #[inline(always)]
    pub(crate) fn read_progress(&self) -> usize {
        // SAFETY: reading a scalar; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).read_progress }
    }

    #[inline(always)]
    pub(crate) fn set_read_progress(&self, read_progress: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).read_progress = read_progress;
        }
    }

    #[inline(always)]
    pub(crate) fn object_dirty(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).object_dirty }
    }

    #[inline(always)]
    pub(crate) fn set_object_dirty(&self, object_dirty: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).object_dirty = object_dirty;
        }
    }

    #[inline(always)]
    pub(crate) fn num_tokens(&self) -> usize {
        // SAFETY: reading a scalar; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).num_tokens }
    }

    #[inline(always)]
    pub(crate) fn set_num_tokens(&self, num_tokens: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).num_tokens = num_tokens;
        }
    }

    #[inline(always)]
    pub(crate) fn mrgb_vertex_count(&self) -> usize {
        // SAFETY: reading a scalar; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).mrgb_vertex_count }
    }

    #[inline(always)]
    pub(crate) fn mesh(&self) -> *mut ObjMesh {
        // SAFETY: reading a scalar; all bit patterns of `*mut ObjMesh` are valid.
        unsafe { (*self.get()).mesh }
    }

    #[inline(always)]
    pub(crate) fn set_mesh(&self, mesh: *mut ObjMesh) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).mesh = mesh;
        }
    }

    #[inline(always)]
    pub(crate) fn material_dirty(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).material_dirty }
    }

    #[inline(always)]
    pub(crate) fn set_material_dirty(&self, material_dirty: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).material_dirty = material_dirty;
        }
    }

    #[inline(always)]
    pub(crate) fn initialized(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).initialized }
    }

    #[inline(always)]
    pub(crate) fn set_initialized(&self, initialized: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).initialized = initialized;
        }
    }

    #[inline(always)]
    pub(crate) fn has_vertex_color(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).has_vertex_color }
    }

    #[inline(always)]
    pub(crate) fn set_has_vertex_color(&self, has_vertex_color: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).has_vertex_color = has_vertex_color;
        }
    }

    #[inline(always)]
    pub(crate) fn has_face_smoothing(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).has_face_smoothing }
    }

    #[inline(always)]
    pub(crate) fn set_has_face_smoothing(&self, has_face_smoothing: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).has_face_smoothing = has_face_smoothing;
        }
    }

    #[inline(always)]
    pub(crate) fn has_face_group(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).has_face_group }
    }

    #[inline(always)]
    pub(crate) fn set_has_face_group(&self, has_face_group: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).has_face_group = has_face_group;
        }
    }

    #[inline(always)]
    pub(crate) fn group_dirty(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).group_dirty }
    }

    #[inline(always)]
    pub(crate) fn set_group_dirty(&self, group_dirty: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).group_dirty = group_dirty;
        }
    }

    #[inline(always)]
    pub(crate) fn face_smoothing(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).face_smoothing }
    }

    #[inline(always)]
    pub(crate) fn set_face_smoothing(&self, face_smoothing: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).face_smoothing = face_smoothing;
        }
    }

    #[inline(always)]
    pub(crate) fn face_material(&self) -> u32 {
        // SAFETY: reading a scalar; all bit patterns of `u32` are valid.
        unsafe { (*self.get()).face_material }
    }

    #[inline(always)]
    pub(crate) fn set_face_material(&self, face_material: u32) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).face_material = face_material;
        }
    }

    #[inline(always)]
    pub(crate) fn face_group_dirty(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).face_group_dirty }
    }

    #[inline(always)]
    pub(crate) fn set_face_group_dirty(&self, face_group_dirty: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).face_group_dirty = face_group_dirty;
        }
    }

    #[inline(always)]
    pub(crate) fn face_group(&self) -> u32 {
        // SAFETY: reading a scalar; all bit patterns of `u32` are valid.
        unsafe { (*self.get()).face_group }
    }

    #[inline(always)]
    pub(crate) fn set_face_group(&self, face_group: u32) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).face_group = face_group;
        }
    }

    #[inline(always)]
    pub(crate) fn tmp_vertices_at(&self, i: usize) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_vertices[i] as *mut crate::native::buf::BufView) }
    }
    #[inline(always)]
    pub(crate) fn tmp_vertices_mut_ptr(&self, i: usize) -> *mut crate::native::buf::Buf {
        unsafe { &raw mut (*self.get()).tmp_vertices[i] }
    }

    #[inline(always)]
    pub(crate) fn tmp_indices_at(&self, i: usize) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_indices[i] as *mut crate::native::buf::BufView) }
    }
    #[inline(always)]
    pub(crate) fn tmp_indices_mut_ptr(&self, i: usize) -> *mut crate::native::buf::Buf {
        unsafe { &raw mut (*self.get()).tmp_indices[i] }
    }

    #[inline(always)]
    pub(crate) fn vertex_count_at(&self, i: usize) -> &crate::prelude::ScalarView<usize> {
        unsafe {
            &*(&raw mut (*self.get()).vertex_count[i] as *mut crate::prelude::ScalarView<usize>)
        }
    }

    #[inline(always)]
    pub(crate) fn fast_indices_at(&self, i: usize) -> &ObjFastIndicesView {
        unsafe { &*(&raw mut (*self.get()).fast_indices[i] as *mut ObjFastIndicesView) }
    }
    #[inline(always)]
    pub(crate) fn fast_indices_mut_ptr(&self, i: usize) -> *mut ObjFastIndices {
        unsafe { &raw mut (*self.get()).fast_indices[i] }
    }
}

// Typed interior-mutable VIEW over `ObjFastIndices` (Copy, but subfields written).
pub(crate) type ObjFastIndicesView = crate::native::view::View<ObjFastIndices>;

impl ObjFastIndicesView {
    #[inline(always)]
    pub(crate) fn set_indices(&self, indices: *mut u64) {
        unsafe {
            (*self.get()).indices = indices;
        }
    }
    #[inline(always)]
    pub(crate) fn num_left(&self) -> usize {
        unsafe { (*self.get()).num_left }
    }
    #[inline(always)]
    pub(crate) fn set_num_left(&self, num_left: usize) {
        unsafe {
            (*self.get()).num_left = num_left;
        }
    }
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

// Typed interior-mutable VIEW over `Context.opts` (RawLoadOpts). Non-Copy nested
// substructs (RawString/RawBlob/RawThreadOpts) recurse into their *View; Copy ones
// (open_file_cb/progress_cb) use value getters; addr-of fields use _ptr.
pub(crate) type LoadOptsView = crate::native::view::View<RawLoadOpts>;

impl LoadOptsView {
    #[inline(always)]
    pub(crate) fn root_transform(&self) -> crate::generated::Transform {
        unsafe { (*self.get()).root_transform }
    }

    #[inline(always)]
    pub(crate) fn open_file_cb_view(&self) -> &crate::prelude::RawOpenFileCbView {
        unsafe { &*(&raw mut (*self.get()).open_file_cb as *mut crate::prelude::RawOpenFileCbView) }
    }

    #[inline(always)]
    pub(crate) fn filename_mut_ptr(&self) -> *mut crate::prelude::RawString {
        unsafe { &raw mut (*self.get()).filename }
    }

    #[inline(always)]
    pub(crate) fn obj_mtl_path_mut_ptr(&self) -> *mut crate::prelude::RawString {
        unsafe { &raw mut (*self.get()).obj_mtl_path }
    }

    #[inline(always)]
    pub(crate) fn geometry_transform_helper_name_mut_ptr(&self) -> *mut crate::prelude::RawString {
        unsafe { &raw mut (*self.get()).geometry_transform_helper_name }
    }

    #[inline(always)]
    pub(crate) fn scale_helper_name_mut_ptr(&self) -> *mut crate::prelude::RawString {
        unsafe { &raw mut (*self.get()).scale_helper_name }
    }

    #[inline(always)]
    pub(crate) fn allow_empty_faces(&self) -> bool {
        unsafe { (*self.get()).allow_empty_faces }
    }

    #[inline(always)]
    pub(crate) fn allow_missing_vertex_position(&self) -> bool {
        unsafe { (*self.get()).allow_missing_vertex_position }
    }

    #[inline(always)]
    pub(crate) fn allow_nodes_out_of_root(&self) -> bool {
        unsafe { (*self.get()).allow_nodes_out_of_root }
    }

    #[inline(always)]
    pub(crate) fn allow_unsafe(&self) -> bool {
        unsafe { (*self.get()).allow_unsafe }
    }

    #[inline(always)]
    pub(crate) fn clean_skin_weights(&self) -> bool {
        unsafe { (*self.get()).clean_skin_weights }
    }

    #[inline(always)]
    pub(crate) fn connect_broken_elements(&self) -> bool {
        unsafe { (*self.get()).connect_broken_elements }
    }

    #[inline(always)]
    pub(crate) fn disable_quirks(&self) -> bool {
        unsafe { (*self.get()).disable_quirks }
    }

    #[inline(always)]
    pub(crate) fn evaluate_caches(&self) -> bool {
        unsafe { (*self.get()).evaluate_caches }
    }

    #[inline(always)]
    pub(crate) fn evaluate_skinning(&self) -> bool {
        unsafe { (*self.get()).evaluate_skinning }
    }

    #[inline(always)]
    pub(crate) fn file_format(&self) -> crate::generated::FileFormat {
        unsafe { (*self.get()).file_format }
    }

    #[inline(always)]
    pub(crate) fn file_format_lookahead(&self) -> usize {
        unsafe { (*self.get()).file_format_lookahead }
    }
    #[inline(always)]
    pub(crate) fn set_file_format_lookahead(&self, file_format_lookahead: usize) {
        unsafe {
            (*self.get()).file_format_lookahead = file_format_lookahead;
        }
    }

    #[inline(always)]
    pub(crate) fn file_size_estimate(&self) -> u64 {
        unsafe { (*self.get()).file_size_estimate }
    }

    #[inline(always)]
    pub(crate) fn filename_view(&self) -> &crate::prelude::RawStringView {
        unsafe { &*(&raw mut (*self.get()).filename as *mut crate::prelude::RawStringView) }
    }

    #[inline(always)]
    pub(crate) fn force_single_thread_ascii_parsing(&self) -> bool {
        unsafe { (*self.get()).force_single_thread_ascii_parsing }
    }

    #[inline(always)]
    pub(crate) fn generate_missing_normals(&self) -> bool {
        unsafe { (*self.get()).generate_missing_normals }
    }

    #[inline(always)]
    pub(crate) fn geometry_transform_handling(
        &self,
    ) -> crate::generated::GeometryTransformHandling {
        unsafe { (*self.get()).geometry_transform_handling }
    }

    #[inline(always)]
    pub(crate) fn geometry_transform_helper_name_view(&self) -> &crate::prelude::RawStringView {
        unsafe {
            &*(&raw mut (*self.get()).geometry_transform_helper_name
                as *mut crate::prelude::RawStringView)
        }
    }

    #[inline(always)]
    pub(crate) fn handedness_conversion_axis(&self) -> crate::generated::MirrorAxis {
        unsafe { (*self.get()).handedness_conversion_axis }
    }

    #[inline(always)]
    pub(crate) fn handedness_conversion_retain_winding(&self) -> bool {
        unsafe { (*self.get()).handedness_conversion_retain_winding }
    }

    #[inline(always)]
    pub(crate) fn ignore_all_content(&self) -> bool {
        unsafe { (*self.get()).ignore_all_content }
    }

    #[inline(always)]
    pub(crate) fn ignore_animation(&self) -> bool {
        unsafe { (*self.get()).ignore_animation }
    }
    #[inline(always)]
    pub(crate) fn set_ignore_animation(&self, ignore_animation: bool) {
        unsafe {
            (*self.get()).ignore_animation = ignore_animation;
        }
    }

    #[inline(always)]
    pub(crate) fn ignore_embedded(&self) -> bool {
        unsafe { (*self.get()).ignore_embedded }
    }
    #[inline(always)]
    pub(crate) fn set_ignore_embedded(&self, ignore_embedded: bool) {
        unsafe {
            (*self.get()).ignore_embedded = ignore_embedded;
        }
    }

    #[inline(always)]
    pub(crate) fn ignore_geometry(&self) -> bool {
        unsafe { (*self.get()).ignore_geometry }
    }
    #[inline(always)]
    pub(crate) fn set_ignore_geometry(&self, ignore_geometry: bool) {
        unsafe {
            (*self.get()).ignore_geometry = ignore_geometry;
        }
    }

    #[inline(always)]
    pub(crate) fn ignore_missing_external_files(&self) -> bool {
        unsafe { (*self.get()).ignore_missing_external_files }
    }

    #[inline(always)]
    pub(crate) fn index_error_handling(&self) -> crate::generated::IndexErrorHandling {
        unsafe { (*self.get()).index_error_handling }
    }

    #[inline(always)]
    pub(crate) fn inherit_mode_handling(&self) -> crate::generated::InheritModeHandling {
        unsafe { (*self.get()).inherit_mode_handling }
    }

    #[inline(always)]
    pub(crate) fn key_clamp_threshold(&self) -> f64 {
        unsafe { (*self.get()).key_clamp_threshold }
    }

    #[inline(always)]
    pub(crate) fn load_external_files(&self) -> bool {
        unsafe { (*self.get()).load_external_files }
    }

    #[inline(always)]
    pub(crate) fn no_format_from_content(&self) -> bool {
        unsafe { (*self.get()).no_format_from_content }
    }

    #[inline(always)]
    pub(crate) fn no_format_from_extension(&self) -> bool {
        unsafe { (*self.get()).no_format_from_extension }
    }

    #[inline(always)]
    pub(crate) fn node_depth_limit(&self) -> u32 {
        unsafe { (*self.get()).node_depth_limit }
    }

    #[inline(always)]
    pub(crate) fn normalize_normals(&self) -> bool {
        unsafe { (*self.get()).normalize_normals }
    }

    #[inline(always)]
    pub(crate) fn normalize_tangents(&self) -> bool {
        unsafe { (*self.get()).normalize_tangents }
    }

    #[inline(always)]
    pub(crate) fn obj_axes(&self) -> crate::generated::CoordinateAxes {
        unsafe { (*self.get()).obj_axes }
    }

    #[inline(always)]
    pub(crate) fn obj_merge_groups(&self) -> bool {
        unsafe { (*self.get()).obj_merge_groups }
    }
    #[inline(always)]
    pub(crate) fn set_obj_merge_groups(&self, obj_merge_groups: bool) {
        unsafe {
            (*self.get()).obj_merge_groups = obj_merge_groups;
        }
    }

    #[inline(always)]
    pub(crate) fn obj_merge_objects(&self) -> bool {
        unsafe { (*self.get()).obj_merge_objects }
    }

    #[inline(always)]
    pub(crate) fn obj_mtl_data_view(&self) -> &crate::prelude::RawBlobView {
        unsafe { &*(&raw mut (*self.get()).obj_mtl_data as *mut crate::prelude::RawBlobView) }
    }

    #[inline(always)]
    pub(crate) fn obj_mtl_path_view(&self) -> &crate::prelude::RawStringView {
        unsafe { &*(&raw mut (*self.get()).obj_mtl_path as *mut crate::prelude::RawStringView) }
    }

    #[inline(always)]
    pub(crate) fn obj_search_mtl_by_filename(&self) -> bool {
        unsafe { (*self.get()).obj_search_mtl_by_filename }
    }

    #[inline(always)]
    pub(crate) fn obj_split_groups(&self) -> bool {
        unsafe { (*self.get()).obj_split_groups }
    }

    #[inline(always)]
    pub(crate) fn obj_unit_meters(&self) -> crate::prelude::Real {
        unsafe { (*self.get()).obj_unit_meters }
    }

    #[inline(always)]
    pub(crate) fn open_file_cb_ptr(&self) -> *const crate::generated::RawOpenFileCb {
        unsafe { &raw const (*self.get()).open_file_cb }
    }

    #[inline(always)]
    pub(crate) fn open_file_cb(&self) -> crate::generated::RawOpenFileCb {
        unsafe { (*self.get()).open_file_cb }
    }

    #[inline(always)]
    pub(crate) fn open_main_file_with_default(&self) -> bool {
        unsafe { (*self.get()).open_main_file_with_default }
    }

    #[inline(always)]
    pub(crate) fn path_separator(&self) -> u8 {
        unsafe { (*self.get()).path_separator }
    }
    #[inline(always)]
    pub(crate) fn set_path_separator(&self, path_separator: u8) {
        unsafe {
            (*self.get()).path_separator = path_separator;
        }
    }

    #[inline(always)]
    pub(crate) fn pivot_handling(&self) -> crate::generated::PivotHandling {
        unsafe { (*self.get()).pivot_handling }
    }

    #[inline(always)]
    pub(crate) fn pivot_handling_retain_empties(&self) -> bool {
        unsafe { (*self.get()).pivot_handling_retain_empties }
    }

    #[inline(always)]
    pub(crate) fn progress_cb(&self) -> crate::generated::RawProgressCb {
        unsafe { (*self.get()).progress_cb }
    }

    #[inline(always)]
    pub(crate) fn progress_interval_hint(&self) -> u64 {
        unsafe { (*self.get()).progress_interval_hint }
    }

    #[inline(always)]
    pub(crate) fn raw_filename_view(&self) -> &crate::prelude::RawBlobView {
        unsafe { &*(&raw mut (*self.get()).raw_filename as *mut crate::prelude::RawBlobView) }
    }

    #[inline(always)]
    pub(crate) fn read_buffer_size(&self) -> usize {
        unsafe { (*self.get()).read_buffer_size }
    }
    #[inline(always)]
    pub(crate) fn set_read_buffer_size(&self, read_buffer_size: usize) {
        unsafe {
            (*self.get()).read_buffer_size = read_buffer_size;
        }
    }

    #[inline(always)]
    pub(crate) fn result_allocator_ptr(&self) -> *const crate::generated::RawAllocatorOpts {
        unsafe { &raw const (*self.get()).result_allocator }
    }

    #[inline(always)]
    pub(crate) fn retain_dom(&self) -> bool {
        unsafe { (*self.get()).retain_dom }
    }

    #[inline(always)]
    pub(crate) fn retain_vertex_attrib_w(&self) -> bool {
        unsafe { (*self.get()).retain_vertex_attrib_w }
    }

    #[inline(always)]
    pub(crate) fn reverse_winding(&self) -> bool {
        unsafe { (*self.get()).reverse_winding }
    }

    #[inline(always)]
    pub(crate) fn root_transform_ptr(&self) -> *const crate::generated::Transform {
        unsafe { &raw const (*self.get()).root_transform }
    }

    #[inline(always)]
    pub(crate) fn scale_helper_name_view(&self) -> &crate::prelude::RawStringView {
        unsafe {
            &*(&raw mut (*self.get()).scale_helper_name as *mut crate::prelude::RawStringView)
        }
    }

    #[inline(always)]
    pub(crate) fn skip_mesh_parts(&self) -> bool {
        unsafe { (*self.get()).skip_mesh_parts }
    }

    #[inline(always)]
    pub(crate) fn skip_skin_vertices(&self) -> bool {
        unsafe { (*self.get()).skip_skin_vertices }
    }

    #[inline(always)]
    pub(crate) fn space_conversion(&self) -> crate::generated::SpaceConversion {
        unsafe { (*self.get()).space_conversion }
    }

    #[inline(always)]
    pub(crate) fn strict(&self) -> bool {
        unsafe { (*self.get()).strict }
    }

    #[inline(always)]
    pub(crate) fn target_axes(&self) -> crate::generated::CoordinateAxes {
        unsafe { (*self.get()).target_axes }
    }

    #[inline(always)]
    pub(crate) fn target_camera_axes(&self) -> crate::generated::CoordinateAxes {
        unsafe { (*self.get()).target_camera_axes }
    }

    #[inline(always)]
    pub(crate) fn target_light_axes(&self) -> crate::generated::CoordinateAxes {
        unsafe { (*self.get()).target_light_axes }
    }

    #[inline(always)]
    pub(crate) fn target_unit_meters(&self) -> crate::prelude::Real {
        unsafe { (*self.get()).target_unit_meters }
    }

    #[inline(always)]
    pub(crate) fn temp_allocator_ptr(&self) -> *const crate::generated::RawAllocatorOpts {
        unsafe { &raw const (*self.get()).temp_allocator }
    }

    #[inline(always)]
    pub(crate) fn thread_opts_view(&self) -> &crate::prelude::RawThreadOptsView {
        unsafe { &*(&raw mut (*self.get()).thread_opts as *mut crate::prelude::RawThreadOptsView) }
    }

    #[inline(always)]
    pub(crate) fn thread_opts_ptr(&self) -> *const crate::generated::RawThreadOpts {
        unsafe { &raw const (*self.get()).thread_opts }
    }

    #[inline(always)]
    pub(crate) fn unicode_error_handling(&self) -> crate::generated::UnicodeErrorHandling {
        unsafe { (*self.get()).unicode_error_handling }
    }

    #[inline(always)]
    pub(crate) fn use_blender_pbr_material(&self) -> bool {
        unsafe { (*self.get()).use_blender_pbr_material }
    }

    #[inline(always)]
    pub(crate) fn use_root_transform(&self) -> bool {
        unsafe { (*self.get()).use_root_transform }
    }
}

// Typed interior-mutable VIEW over a `Scene` field (the public `ufbx_scene`),
// reinterpreted in place. The public `Scene` type is untouched; this is a
// pub(crate) internal handle. Reachable from any context that owns a `Scene`
// field (`Context.scene`, `EvalContext.scene`/`src_scene`). Sub-structs recurse
// into their own *View; List/RefList fields use ListView/RefListView; Copy
// scalars/Refs use value getters/setters or _ptr/_mut_ptr for addr-of sites.
pub(crate) type SceneView = crate::native::view::View<crate::generated::Scene>;

impl SceneView {
    // `metadata` (Metadata) — typed VIEW handle (reinterpret-in-place).
    #[inline(always)]
    pub(crate) fn metadata_view(&self) -> &SceneMetadataView {
        // SAFETY: reinterpret the Metadata field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).metadata as *mut SceneMetadataView) }
    }

    // `settings` (SceneSettings) — typed VIEW handle (reinterpret-in-place).
    #[inline(always)]
    pub(crate) fn settings_view(&self) -> &SceneSettingsView {
        // SAFETY: reinterpret the SceneSettings field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).settings as *mut SceneSettingsView) }
    }
    #[inline(always)]
    pub(crate) fn settings_mut_ptr(&self) -> *mut crate::generated::SceneSettings {
        unsafe { &raw mut (*self.get()).settings }
    }

    #[inline(always)]
    pub(crate) fn anim_curves_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::AnimCurve> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).anim_curves
                as *mut crate::prelude::RefListView<crate::generated::AnimCurve>)
        }
    }

    #[inline(always)]
    pub(crate) fn anim_layers_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::AnimLayer> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).anim_layers
                as *mut crate::prelude::RefListView<crate::generated::AnimLayer>)
        }
    }

    #[inline(always)]
    pub(crate) fn anim_stacks_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::AnimStack> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).anim_stacks
                as *mut crate::prelude::RefListView<crate::generated::AnimStack>)
        }
    }

    #[inline(always)]
    pub(crate) fn anim_values_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::AnimValue> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).anim_values
                as *mut crate::prelude::RefListView<crate::generated::AnimValue>)
        }
    }

    #[inline(always)]
    pub(crate) fn audio_clips_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::AudioClip> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).audio_clips
                as *mut crate::prelude::RefListView<crate::generated::AudioClip>)
        }
    }

    #[inline(always)]
    pub(crate) fn audio_layers_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::AudioLayer> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).audio_layers
                as *mut crate::prelude::RefListView<crate::generated::AudioLayer>)
        }
    }

    #[inline(always)]
    pub(crate) fn blend_channels_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::BlendChannel> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).blend_channels
                as *mut crate::prelude::RefListView<crate::generated::BlendChannel>)
        }
    }

    #[inline(always)]
    pub(crate) fn blend_deformers_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::BlendDeformer> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).blend_deformers
                as *mut crate::prelude::RefListView<crate::generated::BlendDeformer>)
        }
    }

    #[inline(always)]
    pub(crate) fn blend_shapes_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::BlendShape> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).blend_shapes
                as *mut crate::prelude::RefListView<crate::generated::BlendShape>)
        }
    }

    #[inline(always)]
    pub(crate) fn cache_deformers_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::CacheDeformer> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).cache_deformers
                as *mut crate::prelude::RefListView<crate::generated::CacheDeformer>)
        }
    }

    #[inline(always)]
    pub(crate) fn cache_files_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::CacheFile> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).cache_files
                as *mut crate::prelude::RefListView<crate::generated::CacheFile>)
        }
    }

    #[inline(always)]
    pub(crate) fn connections_dst_view(
        &self,
    ) -> &crate::prelude::ListView<crate::generated::Connection> {
        // SAFETY: reinterpret the List field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).connections_dst
                as *mut crate::prelude::ListView<crate::generated::Connection>)
        }
    }

    #[inline(always)]
    pub(crate) fn connections_src_view(
        &self,
    ) -> &crate::prelude::ListView<crate::generated::Connection> {
        // SAFETY: reinterpret the List field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).connections_src
                as *mut crate::prelude::ListView<crate::generated::Connection>)
        }
    }

    #[inline(always)]
    pub(crate) fn constraints_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::Constraint> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).constraints
                as *mut crate::prelude::RefListView<crate::generated::Constraint>)
        }
    }

    #[inline(always)]
    pub(crate) fn display_layers_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::DisplayLayer> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).display_layers
                as *mut crate::prelude::RefListView<crate::generated::DisplayLayer>)
        }
    }

    #[inline(always)]
    pub(crate) fn elements_view(&self) -> &crate::prelude::RefListView<crate::generated::Element> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).elements
                as *mut crate::prelude::RefListView<crate::generated::Element>)
        }
    }

    #[inline(always)]
    pub(crate) fn elements_by_name_view(
        &self,
    ) -> &crate::prelude::ListView<crate::generated::NameElement> {
        // SAFETY: reinterpret the List field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).elements_by_name
                as *mut crate::prelude::ListView<crate::generated::NameElement>)
        }
    }

    #[inline(always)]
    pub(crate) fn line_curves_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::LineCurve> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).line_curves
                as *mut crate::prelude::RefListView<crate::generated::LineCurve>)
        }
    }

    #[inline(always)]
    pub(crate) fn lod_groups_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::LodGroup> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).lod_groups
                as *mut crate::prelude::RefListView<crate::generated::LodGroup>)
        }
    }

    #[inline(always)]
    pub(crate) fn materials_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::Material> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).materials
                as *mut crate::prelude::RefListView<crate::generated::Material>)
        }
    }

    #[inline(always)]
    pub(crate) fn meshes_view(&self) -> &crate::prelude::RefListView<crate::generated::Mesh> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).meshes
                as *mut crate::prelude::RefListView<crate::generated::Mesh>)
        }
    }

    #[inline(always)]
    pub(crate) fn nodes_view(&self) -> &crate::prelude::RefListView<crate::generated::Node> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).nodes
                as *mut crate::prelude::RefListView<crate::generated::Node>)
        }
    }

    #[inline(always)]
    pub(crate) fn nurbs_curves_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::NurbsCurve> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).nurbs_curves
                as *mut crate::prelude::RefListView<crate::generated::NurbsCurve>)
        }
    }

    #[inline(always)]
    pub(crate) fn nurbs_surfaces_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::NurbsSurface> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).nurbs_surfaces
                as *mut crate::prelude::RefListView<crate::generated::NurbsSurface>)
        }
    }

    #[inline(always)]
    pub(crate) fn poses_view(&self) -> &crate::prelude::RefListView<crate::generated::Pose> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).poses
                as *mut crate::prelude::RefListView<crate::generated::Pose>)
        }
    }

    #[inline(always)]
    pub(crate) fn selection_nodes_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::SelectionNode> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).selection_nodes
                as *mut crate::prelude::RefListView<crate::generated::SelectionNode>)
        }
    }

    #[inline(always)]
    pub(crate) fn selection_sets_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::SelectionSet> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).selection_sets
                as *mut crate::prelude::RefListView<crate::generated::SelectionSet>)
        }
    }

    #[inline(always)]
    pub(crate) fn shaders_view(&self) -> &crate::prelude::RefListView<crate::generated::Shader> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).shaders
                as *mut crate::prelude::RefListView<crate::generated::Shader>)
        }
    }

    #[inline(always)]
    pub(crate) fn skin_clusters_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::SkinCluster> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).skin_clusters
                as *mut crate::prelude::RefListView<crate::generated::SkinCluster>)
        }
    }

    #[inline(always)]
    pub(crate) fn skin_deformers_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::SkinDeformer> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).skin_deformers
                as *mut crate::prelude::RefListView<crate::generated::SkinDeformer>)
        }
    }

    #[inline(always)]
    pub(crate) fn stereo_cameras_view(
        &self,
    ) -> &crate::prelude::RefListView<crate::generated::StereoCamera> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).stereo_cameras
                as *mut crate::prelude::RefListView<crate::generated::StereoCamera>)
        }
    }

    #[inline(always)]
    pub(crate) fn texture_files_view(
        &self,
    ) -> &crate::prelude::ListView<crate::generated::TextureFile> {
        // SAFETY: reinterpret the List field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).texture_files
                as *mut crate::prelude::ListView<crate::generated::TextureFile>)
        }
    }

    #[inline(always)]
    pub(crate) fn textures_view(&self) -> &crate::prelude::RefListView<crate::generated::Texture> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).textures
                as *mut crate::prelude::RefListView<crate::generated::Texture>)
        }
    }

    #[inline(always)]
    pub(crate) fn videos_view(&self) -> &crate::prelude::RefListView<crate::generated::Video> {
        // SAFETY: reinterpret the RefList field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).videos
                as *mut crate::prelude::RefListView<crate::generated::Video>)
        }
    }

    #[inline(always)]
    pub(crate) fn unknowns_mut_ptr(
        &self,
    ) -> *mut crate::prelude::RefList<crate::generated::Unknown> {
        unsafe { &raw mut (*self.get()).unknowns }
    }

    #[inline(always)]
    pub(crate) fn root_node(&self) -> crate::prelude::Ref<crate::generated::Node> {
        unsafe { (*self.get()).root_node }
    }
    #[inline(always)]
    pub(crate) fn set_root_node(&self, root_node: crate::prelude::Ref<crate::generated::Node>) {
        unsafe {
            (*self.get()).root_node = root_node;
        }
    }
    #[inline(always)]
    pub(crate) fn root_node_ptr(&self) -> *const crate::prelude::Ref<crate::generated::Node> {
        unsafe { &raw const (*self.get()).root_node }
    }
    #[inline(always)]
    pub(crate) fn root_node_mut_ptr(&self) -> *mut crate::prelude::Ref<crate::generated::Node> {
        unsafe { &raw mut (*self.get()).root_node }
    }
    #[inline(always)]
    pub(crate) fn anim_ptr(&self) -> *const crate::prelude::Ref<crate::generated::Anim> {
        unsafe { &raw const (*self.get()).anim }
    }
    #[inline(always)]
    pub(crate) fn anim_mut_ptr(&self) -> *mut crate::prelude::Ref<crate::generated::Anim> {
        unsafe { &raw mut (*self.get()).anim }
    }
    #[inline(always)]
    pub(crate) fn dom_root(&self) -> Option<crate::prelude::Ref<crate::generated::DomNode>> {
        unsafe { (*self.get()).dom_root }
    }
    #[inline(always)]
    pub(crate) fn set_dom_root(
        &self,
        dom_root: Option<crate::prelude::Ref<crate::generated::DomNode>>,
    ) {
        unsafe {
            (*self.get()).dom_root = dom_root;
        }
    }
}

// Typed interior-mutable VIEW over `Scene.metadata` (the public `ufbx_metadata`),
// reinterpreted in place. String leaves recurse into StringView, Blob leaves into
// BlobView, the warnings List into ListView; Copy scalars use value getters/setters;
// addr-of sites use _ptr (const) / _mut_ptr (mut).
pub(crate) type SceneMetadataView = crate::native::view::View<crate::generated::Metadata>;

impl SceneMetadataView {
    // `scene_props` (Props) — property table VIEW correlated to `&self` (<= uc),
    // so tables found here never outlive the metadata borrow.
    #[inline(always)]
    pub(crate) fn props_view(&self) -> &PropsView {
        // SAFETY: reinterpret the `scene_props` field in place; never a `&mut`,
        // interior-mutable, asserts no validity.
        unsafe { PropsView::from_ptr(&raw mut (*self.get()).scene_props) }
    }
    // --- scalar value getters / setters ---
    #[inline(always)]
    pub(crate) fn file_format(&self) -> crate::generated::FileFormat {
        unsafe { (*self.get()).file_format }
    }
    #[inline(always)]
    pub(crate) fn set_file_format(&self, file_format: crate::generated::FileFormat) {
        unsafe {
            (*self.get()).file_format = file_format;
        }
    }
    #[inline(always)]
    pub(crate) fn geometry_scale(&self) -> crate::prelude::Real {
        unsafe { (*self.get()).geometry_scale }
    }
    #[inline(always)]
    pub(crate) fn set_bone_prop_size_unit(&self, bone_prop_size_unit: crate::prelude::Real) {
        unsafe {
            (*self.get()).bone_prop_size_unit = bone_prop_size_unit;
        }
    }
    #[inline(always)]
    pub(crate) fn set_bone_prop_limb_length_relative(&self, bone_prop_limb_length_relative: bool) {
        unsafe {
            (*self.get()).bone_prop_limb_length_relative = bone_prop_limb_length_relative;
        }
    }
    #[inline(always)]
    pub(crate) fn set_mirror_axis(&self, mirror_axis: crate::generated::MirrorAxis) {
        unsafe {
            (*self.get()).mirror_axis = mirror_axis;
        }
    }
    #[inline(always)]
    pub(crate) fn set_is_unsafe(&self, is_unsafe: bool) {
        unsafe {
            (*self.get()).is_unsafe = is_unsafe;
        }
    }
    #[inline(always)]
    pub(crate) fn set_may_contain_no_index(&self, may_contain_no_index: bool) {
        unsafe {
            (*self.get()).may_contain_no_index = may_contain_no_index;
        }
    }
    #[inline(always)]
    pub(crate) fn set_may_contain_missing_vertex_position(
        &self,
        may_contain_missing_vertex_position: bool,
    ) {
        unsafe {
            (*self.get()).may_contain_missing_vertex_position = may_contain_missing_vertex_position;
        }
    }
    #[inline(always)]
    pub(crate) fn set_may_contain_broken_elements(&self, may_contain_broken_elements: bool) {
        unsafe {
            (*self.get()).may_contain_broken_elements = may_contain_broken_elements;
        }
    }
    #[inline(always)]
    pub(crate) fn set_version(&self, version: u32) {
        unsafe {
            (*self.get()).version = version;
        }
    }
    #[inline(always)]
    pub(crate) fn set_ascii(&self, ascii: bool) {
        unsafe {
            (*self.get()).ascii = ascii;
        }
    }
    #[inline(always)]
    pub(crate) fn set_big_endian(&self, big_endian: bool) {
        unsafe {
            (*self.get()).big_endian = big_endian;
        }
    }
    #[inline(always)]
    pub(crate) fn set_geometry_ignored(&self, geometry_ignored: bool) {
        unsafe {
            (*self.get()).geometry_ignored = geometry_ignored;
        }
    }
    #[inline(always)]
    pub(crate) fn set_animation_ignored(&self, animation_ignored: bool) {
        unsafe {
            (*self.get()).animation_ignored = animation_ignored;
        }
    }
    #[inline(always)]
    pub(crate) fn set_embedded_ignored(&self, embedded_ignored: bool) {
        unsafe {
            (*self.get()).embedded_ignored = embedded_ignored;
        }
    }
    #[inline(always)]
    pub(crate) fn set_exporter(&self, exporter: crate::generated::Exporter) {
        unsafe {
            (*self.get()).exporter = exporter;
        }
    }
    #[inline(always)]
    pub(crate) fn set_exporter_version(&self, exporter_version: u32) {
        unsafe {
            (*self.get()).exporter_version = exporter_version;
        }
    }
    #[inline(always)]
    pub(crate) fn num_shader_textures(&self) -> usize {
        unsafe { (*self.get()).num_shader_textures }
    }
    #[inline(always)]
    pub(crate) fn set_num_shader_textures(&self, num_shader_textures: usize) {
        unsafe {
            (*self.get()).num_shader_textures = num_shader_textures;
        }
    }
    #[inline(always)]
    pub(crate) fn set_ortho_size_unit(&self, ortho_size_unit: crate::prelude::Real) {
        unsafe {
            (*self.get()).ortho_size_unit = ortho_size_unit;
        }
    }
    #[inline(always)]
    pub(crate) fn element_buffer_size(&self) -> usize {
        unsafe { (*self.get()).element_buffer_size }
    }
    #[inline(always)]
    pub(crate) fn set_element_buffer_size(&self, element_buffer_size: usize) {
        unsafe {
            (*self.get()).element_buffer_size = element_buffer_size;
        }
    }
    #[inline(always)]
    pub(crate) fn max_face_triangles(&self) -> usize {
        unsafe { (*self.get()).max_face_triangles }
    }
    #[inline(always)]
    pub(crate) fn set_max_face_triangles(&self, max_face_triangles: usize) {
        unsafe {
            (*self.get()).max_face_triangles = max_face_triangles;
        }
    }
    #[inline(always)]
    pub(crate) fn set_ktime_second(&self, ktime_second: i64) {
        unsafe {
            (*self.get()).ktime_second = ktime_second;
        }
    }

    // --- String leaves: whole value getter/setter + StringView sub-view + _mut_ptr ---
    #[inline(always)]
    pub(crate) fn filename(&self) -> crate::prelude::String {
        unsafe { (*self.get()).filename }
    }
    #[inline(always)]
    pub(crate) fn set_filename(&self, filename: crate::prelude::String) {
        unsafe {
            (*self.get()).filename = filename;
        }
    }
    #[inline(always)]
    pub(crate) fn filename_view(&self) -> &crate::prelude::StringView {
        unsafe { &*(&raw mut (*self.get()).filename as *mut crate::prelude::StringView) }
    }
    #[inline(always)]
    pub(crate) fn filename_mut_ptr(&self) -> *mut crate::prelude::String {
        unsafe { &raw mut (*self.get()).filename }
    }
    #[inline(always)]
    pub(crate) fn creator(&self) -> crate::prelude::String {
        unsafe { (*self.get()).creator }
    }
    #[inline(always)]
    pub(crate) fn creator_view(&self) -> &crate::prelude::StringView {
        unsafe { &*(&raw mut (*self.get()).creator as *mut crate::prelude::StringView) }
    }
    #[inline(always)]
    pub(crate) fn creator_mut_ptr(&self) -> *mut crate::prelude::String {
        unsafe { &raw mut (*self.get()).creator }
    }
    #[inline(always)]
    pub(crate) fn relative_root_view(&self) -> &crate::prelude::StringView {
        unsafe { &*(&raw mut (*self.get()).relative_root as *mut crate::prelude::StringView) }
    }
    #[inline(always)]
    pub(crate) fn relative_root_mut_ptr(&self) -> *mut crate::prelude::String {
        unsafe { &raw mut (*self.get()).relative_root }
    }
    #[inline(always)]
    pub(crate) fn set_original_file_path(&self, original_file_path: crate::prelude::String) {
        unsafe {
            (*self.get()).original_file_path = original_file_path;
        }
    }
    #[inline(always)]
    pub(crate) fn original_file_path_ptr(&self) -> *const crate::prelude::String {
        unsafe { &raw const (*self.get()).original_file_path }
    }

    // --- Blob leaves: whole setter + BlobView sub-view + _mut_ptr / _ptr ---
    #[inline(always)]
    pub(crate) fn set_raw_filename(&self, raw_filename: crate::prelude::Blob) {
        unsafe {
            (*self.get()).raw_filename = raw_filename;
        }
    }
    #[inline(always)]
    pub(crate) fn raw_filename_view(&self) -> &crate::prelude::BlobView {
        unsafe { &*(&raw mut (*self.get()).raw_filename as *mut crate::prelude::BlobView) }
    }
    #[inline(always)]
    pub(crate) fn raw_filename_mut_ptr(&self) -> *mut crate::prelude::Blob {
        unsafe { &raw mut (*self.get()).raw_filename }
    }
    #[inline(always)]
    pub(crate) fn raw_relative_root_view(&self) -> &crate::prelude::BlobView {
        unsafe { &*(&raw mut (*self.get()).raw_relative_root as *mut crate::prelude::BlobView) }
    }
    #[inline(always)]
    pub(crate) fn raw_relative_root_mut_ptr(&self) -> *mut crate::prelude::Blob {
        unsafe { &raw mut (*self.get()).raw_relative_root }
    }
    #[inline(always)]
    pub(crate) fn set_raw_original_file_path(&self, raw_original_file_path: crate::prelude::Blob) {
        unsafe {
            (*self.get()).raw_original_file_path = raw_original_file_path;
        }
    }
    #[inline(always)]
    pub(crate) fn raw_original_file_path_ptr(&self) -> *const crate::prelude::Blob {
        unsafe { &raw const (*self.get()).raw_original_file_path }
    }

    // --- warnings (List<Warning>): ListView sub-view + whole-addr _mut_ptr ---
    #[inline(always)]
    pub(crate) fn warnings_view(&self) -> &crate::prelude::ListView<crate::generated::Warning> {
        unsafe {
            &*(&raw mut (*self.get()).warnings
                as *mut crate::prelude::ListView<crate::generated::Warning>)
        }
    }
    #[inline(always)]
    pub(crate) fn warnings_mut_ptr(&self) -> *mut crate::prelude::List<crate::generated::Warning> {
        unsafe { &raw mut (*self.get()).warnings }
    }
    #[inline(always)]
    pub(crate) fn has_warning_mut_ptr(&self) -> *mut bool {
        unsafe { (&raw mut (*self.get()).has_warning) as *mut bool }
    }

    // --- scene_props (Props) / thumbnail (Thumbnail): addr-of only ---
    #[inline(always)]
    pub(crate) fn scene_props_ptr(&self) -> *const crate::generated::Props {
        unsafe { &raw const (*self.get()).scene_props }
    }
    #[inline(always)]
    pub(crate) fn scene_props_mut_ptr(&self) -> *mut crate::generated::Props {
        unsafe { &raw mut (*self.get()).scene_props }
    }
    #[inline(always)]
    pub(crate) fn thumbnail_mut_ptr(&self) -> *mut crate::generated::Thumbnail {
        unsafe { &raw mut (*self.get()).thumbnail }
    }
}

// Typed interior-mutable VIEW over `Scene.settings` (the public `ufbx_scene_settings`),
// reinterpreted in place. Copy scalars use value getters; `props` (Props aggregate)
// uses a raw-ptr getter for its addr-of / nested-read sites.
pub(crate) type SceneSettingsView = crate::native::view::View<crate::generated::SceneSettings>;

impl SceneSettingsView {
    // `props` (Props) — property table VIEW correlated to `&self` (<= uc), so
    // tables found here never outlive the settings borrow.
    #[inline(always)]
    pub(crate) fn props_view(&self) -> &PropsView {
        // SAFETY: reinterpret the `props` field in place; never a `&mut`,
        // interior-mutable, asserts no validity.
        unsafe { PropsView::from_ptr(&raw mut (*self.get()).props) }
    }
    #[inline(always)]
    pub(crate) fn axes(&self) -> crate::generated::CoordinateAxes {
        unsafe { (*self.get()).axes }
    }
    #[inline(always)]
    pub(crate) fn unit_meters(&self) -> crate::prelude::Real {
        unsafe { (*self.get()).unit_meters }
    }
    #[inline(always)]
    pub(crate) fn frames_per_second(&self) -> f64 {
        unsafe { (*self.get()).frames_per_second }
    }
    #[inline(always)]
    pub(crate) fn props_mut_ptr(&self) -> *mut crate::generated::Props {
        unsafe { &raw mut (*self.get()).props }
    }
}

impl Context {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerContext {
        self.0.get().cast()
    }

    #[inline(always)]
    pub(crate) fn axis_matrix(&self) -> Matrix {
        unsafe { (*self.get()).axis_matrix }
    }

    #[inline(always)]
    pub(crate) fn result(&self) -> crate::native::buf::Buf {
        unsafe { (*self.get()).result }
    }

    #[inline(always)]
    pub(crate) fn set_result(&self, result: crate::native::buf::Buf) {
        unsafe {
            (*self.get()).result = result;
        }
    }

    #[inline(always)]
    pub(crate) fn ator_result(&self) -> crate::native::allocator::Allocator {
        unsafe { (*self.get()).ator_result }
    }

    // `obj` — the embedded ObjContext sub-context handle (Context.obj field).
    #[inline(always)]
    pub(crate) fn obj(&self) -> &ObjContext {
        // SAFETY: the `obj` field IS an ObjContext (repr(transparent) UnsafeCell wrapper)
        // inside this context's outer UnsafeCell; a shared interior-mutable ref is sound.
        unsafe { &(*self.get()).obj }
    }

    // `opts` — typed VIEW handle (reinterpret-in-place); accessors on LoadOptsView.
    #[inline(always)]
    pub(crate) fn opts_view(&self) -> &LoadOptsView {
        // SAFETY: repr(transparent) over the `opts` field inside the outer UnsafeCell;
        // shared interior-mutable view, asserts no validity.
        unsafe { &*(&raw mut (*self.get()).opts as *mut LoadOptsView) }
    }

    // `scene` (Scene) — typed VIEW handle (reinterpret-in-place); accessors on SceneView.
    #[inline(always)]
    pub(crate) fn scene_view(&self) -> &SceneView {
        // SAFETY: repr(transparent) over the `scene` field inside the outer UnsafeCell;
        // shared interior-mutable view, asserts no validity.
        unsafe { &*(&raw mut (*self.get()).scene as *mut SceneView) }
    }

    // `ascii` (Ascii) — typed VIEW handle (reinterpret-in-place); accessors on AsciiView.
    #[inline(always)]
    pub(crate) fn ascii_view(&self) -> &AsciiView {
        unsafe { &*(&raw mut (*self.get()).ascii as *mut AsciiView) }
    }
    // `tmp_typed_element_offsets`/`tmp_thread_parse` ([Buf; N]) — per-element accessors:
    // `_at(i)` → `&BufView` (subfields), `_mut_ptr(i)` → whole-element `*mut Buf` (buf-op out-param).
    #[inline(always)]
    pub(crate) fn tmp_typed_element_offsets_at(&self, i: usize) -> &crate::native::buf::BufView {
        unsafe {
            &*(&raw mut (*self.get()).tmp_typed_element_offsets[i]
                as *mut crate::native::buf::BufView)
        }
    }
    #[inline(always)]
    pub(crate) fn tmp_typed_element_offsets_mut_ptr(&self, i: usize) -> *mut Buf {
        unsafe { &raw mut (*self.get()).tmp_typed_element_offsets[i] }
    }
    #[inline(always)]
    pub(crate) fn tmp_thread_parse_at(&self, i: usize) -> &crate::native::buf::BufView {
        unsafe {
            &*(&raw mut (*self.get()).tmp_thread_parse[i] as *mut crate::native::buf::BufView)
        }
    }
    #[inline(always)]
    pub(crate) fn tmp_thread_parse_mut_ptr(&self, i: usize) -> *mut Buf {
        unsafe { &raw mut (*self.get()).tmp_thread_parse[i] }
    }
    // `exporter`/`mirror_axis` (Copy enums) — value getter/setter.
    #[inline(always)]
    pub(crate) fn exporter(&self) -> Exporter {
        unsafe { (*self.get()).exporter }
    }
    #[inline(always)]
    pub(crate) fn set_exporter(&self, exporter: Exporter) {
        unsafe {
            (*self.get()).exporter = exporter;
        }
    }
    #[inline(always)]
    pub(crate) fn mirror_axis(&self) -> MirrorAxis {
        unsafe { (*self.get()).mirror_axis }
    }
    #[inline(always)]
    pub(crate) fn set_mirror_axis(&self, mirror_axis: MirrorAxis) {
        unsafe {
            (*self.get()).mirror_axis = mirror_axis;
        }
    }
    // `node_prop_set`/`texture_file_map` (Map) — typed VIEW handles (reinterpret-in-place).
    #[inline(always)]
    pub(crate) fn node_prop_set_view(&self) -> &crate::native::hash::MapView {
        unsafe { &*(&raw mut (*self.get()).node_prop_set as *mut crate::native::hash::MapView) }
    }
    #[inline(always)]
    pub(crate) fn texture_file_map_view(&self) -> &crate::native::hash::MapView {
        unsafe { &*(&raw mut (*self.get()).texture_file_map as *mut crate::native::hash::MapView) }
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

    // `tmp_ascii_spans` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_ascii_spans_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_ascii_spans as *mut crate::native::buf::BufView) }
    }

    // `tmp_connections` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_connections_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_connections as *mut crate::native::buf::BufView) }
    }

    // `tmp_dom_nodes` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_dom_nodes_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_dom_nodes as *mut crate::native::buf::BufView) }
    }

    // `tmp_element_fbx_ids` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_element_fbx_ids_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).tmp_element_fbx_ids as *mut crate::native::buf::BufView)
        }
    }

    // `tmp_element_id` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_element_id_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_element_id as *mut crate::native::buf::BufView) }
    }

    // `tmp_element_offsets` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_element_offsets_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).tmp_element_offsets as *mut crate::native::buf::BufView)
        }
    }

    // `tmp_element_ptrs` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_element_ptrs_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_element_ptrs as *mut crate::native::buf::BufView) }
    }

    // `tmp_elements` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_elements_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_elements as *mut crate::native::buf::BufView) }
    }

    // `tmp_full_weights` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_full_weights_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_full_weights as *mut crate::native::buf::BufView) }
    }

    // `tmp_mesh_textures` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_mesh_textures_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_mesh_textures as *mut crate::native::buf::BufView) }
    }

    // `tmp_node_ids` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_node_ids_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_node_ids as *mut crate::native::buf::BufView) }
    }

    // `tmp_parse` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_parse_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_parse as *mut crate::native::buf::BufView) }
    }

    // `tmp_stack` (Buf) — typed VIEW handle (reinterpret-in-place); accessors on BufView.
    #[inline(always)]
    pub(crate) fn tmp_stack_view(&self) -> &crate::native::buf::BufView {
        // SAFETY: reinterpret the Buf field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).tmp_stack as *mut crate::native::buf::BufView) }
    }

    // `error` — const raw-ptr getter (read-only sites); see `error_mut_ptr` for mutation.
    // Reached only from the feature-disabled `UFBX_ENABLE_*` error stubs (`not(feature =
    // "geometry-cache")` in cache.rs, `not(feature = "format-obj")` in obj.rs), so it is
    // legitimately unreachable in the full-feature build where `dead_code` is armed — the
    // inverse of the module-level `cfg_attr(not(all(c-abi, dev)), allow(dead_code))`.
    #[cfg_attr(all(feature = "c-abi", feature = "dev"), allow(dead_code))]
    #[inline(always)]
    pub(crate) fn error_ptr(&self) -> *const Error {
        // SAFETY: `&raw const` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw const (*self.get()).error }
    }

    // `swap_arr_size` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn swap_arr_size_mut_ptr(&self) -> *mut usize {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).swap_arr_size }
    }

    // `swap_arr` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn swap_arr_mut_ptr(&self) -> *mut *mut u8 {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).swap_arr }
    }

    // `top_child` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn top_child_mut_ptr(&self) -> *mut Node {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).top_child }
    }

    // `top_nodes_cap` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn top_nodes_cap_mut_ptr(&self) -> *mut usize {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).top_nodes_cap }
    }

    // `top_nodes` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn top_nodes_mut_ptr(&self) -> *mut *mut Node {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).top_nodes }
    }

    // `dom_parse_toplevel` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn dom_parse_toplevel_mut_ptr(&self) -> *mut *mut DomNode {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).dom_parse_toplevel }
    }

    // `element_extra_cap` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn element_extra_cap_mut_ptr(&self) -> *mut usize {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).element_extra_cap }
    }

    // `element_extra_arr` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn element_extra_arr_mut_ptr(&self) -> *mut *mut *mut c_void {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).element_extra_arr }
    }

    // `max_consecutive_indices` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn max_consecutive_indices_mut_ptr(&self) -> *mut usize {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).max_consecutive_indices }
    }

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawLoadOpts {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).opts }
    }

    // `warnings` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn warnings_mut_ptr(&self) -> *mut Warnings {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).warnings }
    }
    // `warnings` (Warnings) — typed VIEW handle (reinterpret-in-place); accessors on WarningsView.
    #[inline(always)]
    pub(crate) fn warnings_view(&self) -> &crate::native::warnings::WarningsView {
        unsafe { &*(&raw mut (*self.get()).warnings as *mut crate::native::warnings::WarningsView) }
    }

    // `read_buffer_size` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn read_buffer_size_mut_ptr(&self) -> *mut usize {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).read_buffer_size }
    }

    // `read_buffer` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn read_buffer_mut_ptr(&self) -> *mut *mut u8 {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).read_buffer }
    }

    // `root_id` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn root_id_mut_ptr(&self) -> *mut u64 {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).root_id }
    }

    // `axis_matrix` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn axis_matrix_mut_ptr(&self) -> *mut Matrix {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).axis_matrix }
    }

    // `tmp_ascii_spans` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_ascii_spans_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_ascii_spans }
    }

    // `legacy_node` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn legacy_node_mut_ptr(&self) -> *mut Node {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).legacy_node }
    }

    // `tmp_mesh_textures` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_mesh_textures_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_mesh_textures }
    }

    // `ator_result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_result_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator_result }
    }

    // `tmp_element_id` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_element_id_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_element_id }
    }

    // `ptr_fbx_id_map` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ptr_fbx_id_map_mut_ptr(&self) -> *mut Map {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ptr_fbx_id_map }
    }

    // `texture_file_map` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn texture_file_map_mut_ptr(&self) -> *mut Map {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).texture_file_map }
    }

    // `tmp_element_fbx_ids` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_element_fbx_ids_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_element_fbx_ids }
    }

    // `tmp_element_ptrs` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_element_ptrs_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_element_ptrs }
    }

    // `node_prop_set` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn node_prop_set_mut_ptr(&self) -> *mut Map {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).node_prop_set }
    }

    // `prop_type_map` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn prop_type_map_mut_ptr(&self) -> *mut Map {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).prop_type_map }
    }

    // `tmp_dom_nodes` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_dom_nodes_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_dom_nodes }
    }

    // `dom_node_map` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn dom_node_map_mut_ptr(&self) -> *mut Map {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).dom_node_map }
    }

    // `anim_stack_map` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn anim_stack_map_mut_ptr(&self) -> *mut Map {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).anim_stack_map }
    }

    // `fbx_id_map` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn fbx_id_map_mut_ptr(&self) -> *mut Map {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).fbx_id_map }
    }

    // `scene` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn scene_mut_ptr(&self) -> *mut Scene {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).scene }
    }

    // `tmp_full_weights` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_full_weights_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_full_weights }
    }

    // `tmp_elements` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_elements_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_elements }
    }

    // `fbx_attr_map` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn fbx_attr_map_mut_ptr(&self) -> *mut Map {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).fbx_attr_map }
    }

    // `tmp_element_offsets` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_element_offsets_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_element_offsets }
    }

    // `tmp_connections` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_connections_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_connections }
    }

    // `thread_pool` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn thread_pool_mut_ptr(&self) -> *mut ThreadPool {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).thread_pool }
    }

    // `tmp_node_ids` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_node_ids_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_node_ids }
    }

    // `ascii` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ascii_mut_ptr(&self) -> *mut Ascii {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ascii }
    }

    // `tmp_arr_size` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_arr_size_mut_ptr(&self) -> *mut usize {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_arr_size }
    }
    // Value getter for the read-only size sites (the `&mut`/grow-array out-param sites
    // keep using `tmp_arr_size_mut_ptr`).
    #[inline(always)]
    pub(crate) fn tmp_arr_size(&self) -> usize {
        unsafe { (*self.get()).tmp_arr_size }
    }

    // `tmp_arr` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_arr_mut_ptr(&self) -> *mut *mut u8 {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_arr }
    }

    // `tmp_parse` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_parse_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_parse }
    }

    // `error` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn error_mut_ptr(&self) -> *mut Error {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).error }
    }

    // `error` — anchored VIEW handle; accessors on `ErrorView`. Routes the
    // error-form check macros through the SAFE `fail_err`/`fail_err_no_stack`.
    #[inline(always)]
    pub(crate) fn error_view(&self) -> &crate::native::error::ErrorView {
        // SAFETY: the context-owned `error` field is interior-mutable arena memory;
        // `&raw mut` keeps write provenance (never `&T`); borrow of `self` anchors `'a <= self`.
        unsafe { crate::native::error::ErrorView::from_ptr(&raw mut (*self.get()).error) }
    }

    // `tmp` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp }
    }

    // `string_pool` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn string_pool_mut_ptr(&self) -> *mut StringPool {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).string_pool }
    }
    // `string_pool` (StringPool) — typed VIEW handle (reinterpret-in-place) for nested
    // access; whole value getter/setter for the faithful C struct-copy sites.
    #[inline(always)]
    pub(crate) fn string_pool_view(&self) -> &crate::native::string_pool::StringPoolView {
        unsafe {
            &*(&raw mut (*self.get()).string_pool
                as *mut crate::native::string_pool::StringPoolView)
        }
    }
    #[inline(always)]
    pub(crate) fn string_pool(&self) -> StringPool {
        unsafe { (*self.get()).string_pool }
    }
    #[inline(always)]
    pub(crate) fn set_string_pool(&self, string_pool: StringPool) {
        unsafe {
            (*self.get()).string_pool = string_pool;
        }
    }

    // `result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn result_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).result }
    }

    // `tmp_stack` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_stack_mut_ptr(&self) -> *mut Buf {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).tmp_stack }
    }

    // Reborrow a raw `*mut InnerContext` as `&Context` (layout-identical via
    // `repr(transparent)`). For the nullable-context (`maybe_uc`) call paths.
    // SAFETY: `ptr` must be non-null and point to a live context allocation.
    #[inline(always)]
    pub(crate) unsafe fn from_ptr<'a>(ptr: *mut InnerContext) -> &'a Context {
        &*(ptr as *const Context)
    }

    // `base64_table` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn base64_table(&self) -> *mut u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u8` are valid.
        unsafe { (*self.get()).base64_table }
    }

    #[inline(always)]
    pub(crate) fn set_base64_table(&self, base64_table: *mut u8) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).base64_table = base64_table;
        }
    }

    // `parse_threaded` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn parse_threaded(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).parse_threaded }
    }

    #[inline(always)]
    pub(crate) fn set_parse_threaded(&self, parse_threaded: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).parse_threaded = parse_threaded;
        }
    }

    // `load_filename_len` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn load_filename_len(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).load_filename_len }
    }

    #[inline(always)]
    pub(crate) fn set_load_filename_len(&self, load_filename_len: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).load_filename_len = load_filename_len;
        }
    }

    // `load_filename` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn load_filename(&self) -> *const u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*const u8` are valid.
        unsafe { (*self.get()).load_filename }
    }

    #[inline(always)]
    pub(crate) fn set_load_filename(&self, load_filename: *const u8) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).load_filename = load_filename;
        }
    }

    // `deferred_load` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn deferred_load(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).deferred_load }
    }

    #[inline(always)]
    pub(crate) fn set_deferred_load(&self, deferred_load: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).deferred_load = deferred_load;
        }
    }

    // `deferred_failure` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn deferred_failure(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).deferred_failure }
    }

    // `unit_scale` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn unit_scale(&self) -> Real {
        // SAFETY: reading a scalar field; all bit patterns of `Real` are valid.
        unsafe { (*self.get()).unit_scale }
    }

    #[inline(always)]
    pub(crate) fn set_unit_scale(&self, unit_scale: Real) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).unit_scale = unit_scale;
        }
    }

    // `ktime_sec_double` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn ktime_sec_double(&self) -> f64 {
        // SAFETY: reading a scalar field; all bit patterns of `f64` are valid.
        unsafe { (*self.get()).ktime_sec_double }
    }

    #[inline(always)]
    pub(crate) fn set_ktime_sec_double(&self, ktime_sec_double: f64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).ktime_sec_double = ktime_sec_double;
        }
    }

    // `ktime_sec` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn ktime_sec(&self) -> i64 {
        // SAFETY: reading a scalar field; all bit patterns of `i64` are valid.
        unsafe { (*self.get()).ktime_sec }
    }

    #[inline(always)]
    pub(crate) fn set_ktime_sec(&self, ktime_sec: i64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).ktime_sec = ktime_sec;
        }
    }

    // `num_file_content` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn num_file_content(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).num_file_content }
    }

    #[inline(always)]
    pub(crate) fn set_num_file_content(&self, num_file_content: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).num_file_content = num_file_content;
        }
    }

    // `file_content` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn file_content(&self) -> *mut FileContent {
        // SAFETY: reading a scalar field; all bit patterns of `*mut FileContent` are valid.
        unsafe { (*self.get()).file_content }
    }

    #[inline(always)]
    pub(crate) fn set_file_content(&self, file_content: *mut FileContent) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).file_content = file_content;
        }
    }

    // `legacy_implicit_anim_layer_id` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn legacy_implicit_anim_layer_id(&self) -> u64 {
        // SAFETY: reading a scalar field; all bit patterns of `u64` are valid.
        unsafe { (*self.get()).legacy_implicit_anim_layer_id }
    }

    #[inline(always)]
    pub(crate) fn set_legacy_implicit_anim_layer_id(&self, legacy_implicit_anim_layer_id: u64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).legacy_implicit_anim_layer_id = legacy_implicit_anim_layer_id;
        }
    }

    // `num_elements` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn num_elements(&self) -> u32 {
        // SAFETY: reading a scalar field; all bit patterns of `u32` are valid.
        unsafe { (*self.get()).num_elements }
    }

    #[inline(always)]
    pub(crate) fn set_num_elements(&self, num_elements: u32) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).num_elements = num_elements;
        }
    }

    // `root_id` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn root_id(&self) -> u64 {
        // SAFETY: reading a scalar field; all bit patterns of `u64` are valid.
        unsafe { (*self.get()).root_id }
    }

    #[inline(always)]
    pub(crate) fn set_root_id(&self, root_id: u64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).root_id = root_id;
        }
    }

    // `tmp_mesh_consecutive_indices` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_mesh_consecutive_indices(&self) -> *mut u32 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u32` are valid.
        unsafe { (*self.get()).tmp_mesh_consecutive_indices }
    }

    #[inline(always)]
    pub(crate) fn set_tmp_mesh_consecutive_indices(&self, tmp_mesh_consecutive_indices: *mut u32) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).tmp_mesh_consecutive_indices = tmp_mesh_consecutive_indices;
        }
    }

    // `inflate_retain` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn inflate_retain(&self) -> *mut InflateRetain {
        // SAFETY: reading a scalar field; all bit patterns of `*mut InflateRetain` are valid.
        unsafe { (*self.get()).inflate_retain }
    }

    #[inline(always)]
    pub(crate) fn set_inflate_retain(&self, inflate_retain: *mut InflateRetain) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).inflate_retain = inflate_retain;
        }
    }

    // `scene_imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn scene_imp(&self) -> *mut SceneImp {
        // SAFETY: reading a scalar field; all bit patterns of `*mut SceneImp` are valid.
        unsafe { (*self.get()).scene_imp }
    }

    #[inline(always)]
    pub(crate) fn set_scene_imp(&self, scene_imp: *mut SceneImp) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).scene_imp = scene_imp;
        }
    }

    // `blender_full_weights` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn blender_full_weights(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).blender_full_weights }
    }

    #[inline(always)]
    pub(crate) fn set_blender_full_weights(&self, blender_full_weights: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).blender_full_weights = blender_full_weights;
        }
    }

    // `retain_vertex_w` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn retain_vertex_w(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).retain_vertex_w }
    }

    #[inline(always)]
    pub(crate) fn set_retain_vertex_w(&self, retain_vertex_w: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).retain_vertex_w = retain_vertex_w;
        }
    }

    // `has_scale_helper_nodes` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn has_scale_helper_nodes(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).has_scale_helper_nodes }
    }

    #[inline(always)]
    pub(crate) fn set_has_scale_helper_nodes(&self, has_scale_helper_nodes: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).has_scale_helper_nodes = has_scale_helper_nodes;
        }
    }

    // `has_geometry_transform_nodes` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn has_geometry_transform_nodes(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).has_geometry_transform_nodes }
    }

    #[inline(always)]
    pub(crate) fn set_has_geometry_transform_nodes(&self, has_geometry_transform_nodes: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).has_geometry_transform_nodes = has_geometry_transform_nodes;
        }
    }

    // `synthetic_id_counter` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn synthetic_id_counter(&self) -> u64 {
        // SAFETY: reading a scalar field; all bit patterns of `u64` are valid.
        unsafe { (*self.get()).synthetic_id_counter }
    }

    #[inline(always)]
    pub(crate) fn set_synthetic_id_counter(&self, synthetic_id_counter: u64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).synthetic_id_counter = synthetic_id_counter;
        }
    }

    // `size_fn` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn size_fn(&self) -> Option<unsafe extern "C" fn(*mut c_void) -> u64> {
        // SAFETY: reading a scalar field; all bit patterns of `Option<unsafe extern "C" fn(*mut c_void) -> u64>` are valid.
        unsafe { (*self.get()).size_fn }
    }

    #[inline(always)]
    pub(crate) fn set_size_fn(&self, size_fn: Option<unsafe extern "C" fn(*mut c_void) -> u64>) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).size_fn = size_fn;
        }
    }

    // `close_fn` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn close_fn(&self) -> Option<unsafe extern "C" fn(*mut c_void)> {
        // SAFETY: reading a scalar field; all bit patterns of `Option<unsafe extern "C" fn(*mut c_void)>` are valid.
        unsafe { (*self.get()).close_fn }
    }

    #[inline(always)]
    pub(crate) fn set_close_fn(&self, close_fn: Option<unsafe extern "C" fn(*mut c_void)>) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).close_fn = close_fn;
        }
    }

    // `tmp_element_flag` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_element_flag(&self) -> *mut u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u8` are valid.
        unsafe { (*self.get()).tmp_element_flag }
    }

    #[inline(always)]
    pub(crate) fn set_tmp_element_flag(&self, tmp_element_flag: *mut u8) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).tmp_element_flag = tmp_element_flag;
        }
    }

    // `element_extra_cap` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn element_extra_cap(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).element_extra_cap }
    }

    // `element_extra_arr` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn element_extra_arr(&self) -> *mut *mut c_void {
        // SAFETY: reading a scalar field; all bit patterns of `*mut *mut c_void` are valid.
        unsafe { (*self.get()).element_extra_arr }
    }

    // `latest_progress_bytes` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn latest_progress_bytes(&self) -> u64 {
        // SAFETY: reading a scalar field; all bit patterns of `u64` are valid.
        unsafe { (*self.get()).latest_progress_bytes }
    }

    #[inline(always)]
    pub(crate) fn set_latest_progress_bytes(&self, latest_progress_bytes: u64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).latest_progress_bytes = latest_progress_bytes;
        }
    }

    // `progress_bytes_total` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn progress_bytes_total(&self) -> u64 {
        // SAFETY: reading a scalar field; all bit patterns of `u64` are valid.
        unsafe { (*self.get()).progress_bytes_total }
    }

    #[inline(always)]
    pub(crate) fn set_progress_bytes_total(&self, progress_bytes_total: u64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).progress_bytes_total = progress_bytes_total;
        }
    }

    // `progress_timer` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn progress_timer(&self) -> isize {
        // SAFETY: reading a scalar field; all bit patterns of `isize` are valid.
        unsafe { (*self.get()).progress_timer }
    }

    #[inline(always)]
    pub(crate) fn set_progress_timer(&self, progress_timer: isize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).progress_timer = progress_timer;
        }
    }

    // `consecutive_indices` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn consecutive_indices(&self) -> *mut u32 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u32` are valid.
        unsafe { (*self.get()).consecutive_indices }
    }

    #[inline(always)]
    pub(crate) fn set_consecutive_indices(&self, consecutive_indices: *mut u32) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).consecutive_indices = consecutive_indices;
        }
    }

    // `zero_indices` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn zero_indices(&self) -> *mut u32 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u32` are valid.
        unsafe { (*self.get()).zero_indices }
    }

    #[inline(always)]
    pub(crate) fn set_zero_indices(&self, zero_indices: *mut u32) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).zero_indices = zero_indices;
        }
    }

    // `has_next_child` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn has_next_child(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).has_next_child }
    }

    #[inline(always)]
    pub(crate) fn set_has_next_child(&self, has_next_child: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).has_next_child = has_next_child;
        }
    }

    // `top_child_index` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn top_child_index(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).top_child_index }
    }

    #[inline(always)]
    pub(crate) fn set_top_child_index(&self, top_child_index: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).top_child_index = top_child_index;
        }
    }

    // `parsed_to_end` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn parsed_to_end(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).parsed_to_end }
    }

    #[inline(always)]
    pub(crate) fn set_parsed_to_end(&self, parsed_to_end: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).parsed_to_end = parsed_to_end;
        }
    }

    // `top_nodes_cap` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn top_nodes_cap(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).top_nodes_cap }
    }

    // `top_nodes_len` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn top_nodes_len(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).top_nodes_len }
    }

    #[inline(always)]
    pub(crate) fn set_top_nodes_len(&self, top_nodes_len: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).top_nodes_len = top_nodes_len;
        }
    }

    // `top_nodes` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn top_nodes(&self) -> *mut Node {
        // SAFETY: reading a scalar field; all bit patterns of `*mut Node` are valid.
        unsafe { (*self.get()).top_nodes }
    }

    // `p_element_id` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn p_element_id(&self) -> *mut u32 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u32` are valid.
        unsafe { (*self.get()).p_element_id }
    }

    #[inline(always)]
    pub(crate) fn set_p_element_id(&self, p_element_id: *mut u32) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).p_element_id = p_element_id;
        }
    }

    // `dom_parse_num_children` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn dom_parse_num_children(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).dom_parse_num_children }
    }

    #[inline(always)]
    pub(crate) fn set_dom_parse_num_children(&self, dom_parse_num_children: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).dom_parse_num_children = dom_parse_num_children;
        }
    }

    // `dom_parse_toplevel` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn dom_parse_toplevel(&self) -> *mut DomNode {
        // SAFETY: reading a scalar field; all bit patterns of `*mut DomNode` are valid.
        unsafe { (*self.get()).dom_parse_toplevel }
    }

    #[inline(always)]
    pub(crate) fn set_dom_parse_toplevel(&self, dom_parse_toplevel: *mut DomNode) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).dom_parse_toplevel = dom_parse_toplevel;
        }
    }

    // `num_templates` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn num_templates(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).num_templates }
    }

    #[inline(always)]
    pub(crate) fn set_num_templates(&self, num_templates: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).num_templates = num_templates;
        }
    }

    // `templates` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn templates(&self) -> *mut Template {
        // SAFETY: reading a scalar field; all bit patterns of `*mut Template` are valid.
        unsafe { (*self.get()).templates }
    }

    #[inline(always)]
    pub(crate) fn set_templates(&self, templates: *mut Template) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).templates = templates;
        }
    }

    // `tmp_element_byte_offset` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_element_byte_offset(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).tmp_element_byte_offset }
    }

    #[inline(always)]
    pub(crate) fn set_tmp_element_byte_offset(&self, tmp_element_byte_offset: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).tmp_element_byte_offset = tmp_element_byte_offset;
        }
    }

    // `tmp_arr` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_arr(&self) -> *mut u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u8` are valid.
        unsafe { (*self.get()).tmp_arr }
    }

    // `max_consecutive_indices` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn max_consecutive_indices(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).max_consecutive_indices }
    }

    #[inline(always)]
    pub(crate) fn set_max_consecutive_indices(&self, max_consecutive_indices: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).max_consecutive_indices = max_consecutive_indices;
        }
    }

    // `max_zero_indices` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn max_zero_indices(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).max_zero_indices }
    }

    #[inline(always)]
    pub(crate) fn set_max_zero_indices(&self, max_zero_indices: usize) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).max_zero_indices = max_zero_indices;
        }
    }

    // `swap_arr_size` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn swap_arr_size(&self) -> usize {
        // SAFETY: reading a scalar field; all bit patterns of `usize` are valid.
        unsafe { (*self.get()).swap_arr_size }
    }

    // `swap_arr` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn swap_arr(&self) -> *mut u8 {
        // SAFETY: reading a scalar field; all bit patterns of `*mut u8` are valid.
        unsafe { (*self.get()).swap_arr }
    }

    #[inline(always)]
    pub(crate) fn set_swap_arr(&self, swap_arr: *mut u8) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).swap_arr = swap_arr;
        }
    }

    // `skip_fn` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn skip_fn(&self) -> Option<unsafe extern "C" fn(*mut c_void, usize) -> bool> {
        // SAFETY: reading a scalar field; all bit patterns of `Option<unsafe extern "C" fn(*mut c_void, usize) -> bool>` are valid.
        unsafe { (*self.get()).skip_fn }
    }

    #[inline(always)]
    pub(crate) fn set_skip_fn(
        &self,
        skip_fn: Option<unsafe extern "C" fn(*mut c_void, usize) -> bool>,
    ) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).skip_fn = skip_fn;
        }
    }

    // `data_offset` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn data_offset(&self) -> u64 {
        // SAFETY: reading a scalar field; all bit patterns of `u64` are valid.
        unsafe { (*self.get()).data_offset }
    }

    #[inline(always)]
    pub(crate) fn set_data_offset(&self, data_offset: u64) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).data_offset = data_offset;
        }
    }

    // `double_parse_flags` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn double_parse_flags(&self) -> u32 {
        // SAFETY: reading a scalar field; all bit patterns of `u32` are valid.
        unsafe { (*self.get()).double_parse_flags }
    }

    #[inline(always)]
    pub(crate) fn set_double_parse_flags(&self, double_parse_flags: u32) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).double_parse_flags = double_parse_flags;
        }
    }

    // `read_legacy_settings` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn read_legacy_settings(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).read_legacy_settings }
    }

    #[inline(always)]
    pub(crate) fn set_read_legacy_settings(&self, read_legacy_settings: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).read_legacy_settings = read_legacy_settings;
        }
    }

    // `retain_mesh_parts` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn retain_mesh_parts(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).retain_mesh_parts }
    }

    #[inline(always)]
    pub(crate) fn set_retain_mesh_parts(&self, retain_mesh_parts: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).retain_mesh_parts = retain_mesh_parts;
        }
    }

    // `sure_fbx` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn sure_fbx(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).sure_fbx }
    }

    #[inline(always)]
    pub(crate) fn set_sure_fbx(&self, sure_fbx: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).sure_fbx = sure_fbx;
        }
    }

    // `file_big_endian` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn file_big_endian(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).file_big_endian }
    }

    #[inline(always)]
    pub(crate) fn set_file_big_endian(&self, file_big_endian: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).file_big_endian = file_big_endian;
        }
    }

    // `local_big_endian` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn local_big_endian(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).local_big_endian }
    }

    #[inline(always)]
    pub(crate) fn set_local_big_endian(&self, local_big_endian: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).local_big_endian = local_big_endian;
        }
    }

    // `from_ascii` — scalar value accessor. Named after the C field it reads; the
    // `from_*(&self)` query shape is intentional, not a conversion constructor.
    #[allow(clippy::wrong_self_convention)]
    #[inline(always)]
    pub(crate) fn from_ascii(&self) -> bool {
        // SAFETY: reading a `bool` we only ever store valid bools into.
        unsafe { (*self.get()).from_ascii }
    }

    #[inline(always)]
    pub(crate) fn set_from_ascii(&self, from_ascii: bool) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).from_ascii = from_ascii;
        }
    }

    // `exporter_version` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn exporter_version(&self) -> u32 {
        // SAFETY: reading a scalar field; all bit patterns of `u32` are valid.
        unsafe { (*self.get()).exporter_version }
    }

    #[inline(always)]
    pub(crate) fn set_exporter_version(&self, exporter_version: u32) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).exporter_version = exporter_version;
        }
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
        unsafe {
            (*self.get()).version = version;
        }
    }

    // Temp-arena allocator. `Allocator` is aliased (copied by raw pointer into
    // sibling contexts) and mutated by `alloc`, so the honest accessor is a raw
    // pointer, not a reference — passing it onward is a safe operation.
    #[inline(always)]
    pub(crate) fn ator_tmp_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: projecting a field pointer; no deref, no reference formed.
        unsafe { &raw mut (*self.get()).ator_tmp }
    }

    // Input read cursor. Scalar raw pointer: value getter + setter. Copying a
    // `*const u8` out is safe; any later deref/`.add` stays the caller's
    // (unchanged) unsafe obligation.
    #[inline(always)]
    pub(crate) fn data(&self) -> *const u8 {
        // SAFETY: reading a `*const u8` field; all bit patterns valid.
        unsafe { (*self.get()).data }
    }

    #[inline(always)]
    pub(crate) fn set_data(&self, data: *const u8) {
        // SAFETY: storing a `*const u8`; cannot violate validity.
        unsafe {
            (*self.get()).data = data;
        }
    }

    // Remaining bytes at the read cursor. Scalar `usize`: value getter + setter.
    #[inline(always)]
    pub(crate) fn data_size(&self) -> usize {
        // SAFETY: reading a `usize` field; all bit patterns valid.
        unsafe { (*self.get()).data_size }
    }

    #[inline(always)]
    pub(crate) fn set_data_size(&self, data_size: usize) {
        // SAFETY: storing a `usize`; cannot violate validity.
        unsafe {
            (*self.get()).data_size = data_size;
        }
    }

    // Bytes remaining before the next progress-yield checkpoint. Scalar `usize`.
    #[inline(always)]
    pub(crate) fn yield_size(&self) -> usize {
        // SAFETY: reading a `usize` field; all bit patterns valid.
        unsafe { (*self.get()).yield_size }
    }

    #[inline(always)]
    pub(crate) fn set_yield_size(&self, yield_size: usize) {
        // SAFETY: storing a `usize`; cannot violate validity.
        unsafe {
            (*self.get()).yield_size = yield_size;
        }
    }

    // Stream read callback. Scalar `Option<fn>` (a nullable fn pointer): value
    // getter + setter. Copies the option out; invoking it stays unsafe.
    #[inline(always)]
    pub(crate) fn read_fn(
        &self,
    ) -> Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize> {
        // SAFETY: reading an `Option<fn>` field; all bit patterns valid.
        unsafe { (*self.get()).read_fn }
    }

    #[inline(always)]
    pub(crate) fn set_read_fn(
        &self,
        read_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>,
    ) {
        // SAFETY: storing an `Option<fn>`; cannot violate validity.
        unsafe {
            (*self.get()).read_fn = read_fn;
        }
    }

    // User pointer passed to `read_fn`. Scalar `*mut c_void`: value getter + setter.
    #[inline(always)]
    pub(crate) fn read_user(&self) -> *mut c_void {
        // SAFETY: reading a `*mut c_void` field; all bit patterns valid.
        unsafe { (*self.get()).read_user }
    }

    #[inline(always)]
    pub(crate) fn set_read_user(&self, read_user: *mut c_void) {
        // SAFETY: storing a `*mut c_void`; cannot violate validity.
        unsafe {
            (*self.get()).read_user = read_user;
        }
    }

    // Start of the current read buffer. Scalar `*const u8`: value getter + setter.
    #[inline(always)]
    pub(crate) fn data_begin(&self) -> *const u8 {
        // SAFETY: reading a `*const u8` field; all bit patterns valid.
        unsafe { (*self.get()).data_begin }
    }

    #[inline(always)]
    pub(crate) fn set_data_begin(&self, data_begin: *const u8) {
        // SAFETY: storing a `*const u8`; cannot violate validity.
        unsafe {
            (*self.get()).data_begin = data_begin;
        }
    }

    // End-of-input flag. Scalar `bool` (only `0`/`1` ever stored): value getter
    // + setter.
    #[inline(always)]
    pub(crate) fn eof(&self) -> bool {
        // SAFETY: reading a `bool` we only ever write valid bools into.
        unsafe { (*self.get()).eof }
    }

    #[inline(always)]
    pub(crate) fn set_eof(&self, eof: bool) {
        // SAFETY: storing a `bool`; cannot violate validity.
        unsafe {
            (*self.get()).eof = eof;
        }
    }

    // Progress-callback byte interval. Scalar `usize`: value getter + setter.
    #[inline(always)]
    pub(crate) fn progress_interval(&self) -> usize {
        // SAFETY: reading a `usize` field; all bit patterns valid.
        unsafe { (*self.get()).progress_interval }
    }

    #[inline(always)]
    pub(crate) fn set_progress_interval(&self, progress_interval: usize) {
        // SAFETY: storing a `usize`; cannot violate validity.
        unsafe {
            (*self.get()).progress_interval = progress_interval;
        }
    }

    // Deepest open node on the parse stack. Scalar `*mut Node`: value getter +
    // setter. Copies the pointer out; any deref stays the caller's obligation.
    #[inline(always)]
    pub(crate) fn top_node(&self) -> *mut Node {
        // SAFETY: reading a `*mut Node` field; all bit patterns valid.
        unsafe { (*self.get()).top_node }
    }

    #[inline(always)]
    pub(crate) fn set_top_node(&self, top_node: *mut Node) {
        // SAFETY: storing a `*mut Node`; cannot violate validity.
        unsafe {
            (*self.get()).top_node = top_node;
        }
    }

    // Rust-port: the top node borrowed AS A VIEW. The returned `&NodeView` borrows
    // `self`, so its lifetime is `<= uc` — this is the "mint root views from
    // &Context" rule that anchors a whole navigation chain to a single `uc`
    // lifetime instead of a free `from_ptr` lifetime. `None` mirrors the C
    // `uc->top_node != NULL` guard at every call site.
    #[inline(always)]
    pub(crate) fn top_node_view(&self) -> Option<&NodeView> {
        let node = self.top_node();
        if node.is_null() {
            None
        } else {
            // SAFETY: a non-null top node points into `uc`'s DOM arena, valid and
            // stable for the borrow of `self`.
            Some(unsafe { NodeView::from_ptr(node) })
        }
    }

    // Backing read buffer. Scalar `*mut u8`: value getter + setter. The paired
    // `&mut uc.read_buffer` out-param sites in `refill` stay raw (a value getter
    // cannot express writing back through the field).
    #[inline(always)]
    pub(crate) fn read_buffer(&self) -> *mut u8 {
        // SAFETY: reading a `*mut u8` field; all bit patterns valid.
        unsafe { (*self.get()).read_buffer }
    }

    #[inline(always)]
    pub(crate) fn set_read_buffer(&self, read_buffer: *mut u8) {
        // SAFETY: storing a `*mut u8`; cannot violate validity.
        unsafe {
            (*self.get()).read_buffer = read_buffer;
        }
    }

    // Capacity of `read_buffer` in bytes. Scalar `usize`: value getter + setter.
    // The paired `&mut uc.read_buffer_size` out-param site in `refill` stays raw.
    #[inline(always)]
    pub(crate) fn read_buffer_size(&self) -> usize {
        // SAFETY: reading a `usize` field; all bit patterns valid.
        unsafe { (*self.get()).read_buffer_size }
    }

    #[inline(always)]
    pub(crate) fn set_read_buffer_size(&self, read_buffer_size: usize) {
        // SAFETY: storing a `usize`; cannot violate validity.
        unsafe {
            (*self.get()).read_buffer_size = read_buffer_size;
        }
    }
}

// ufbx.c:6652-6655 `ufbxi_fail_imp`
// Expansion target of the `_msg` forms of the uc-context check macros
// (`native::error::ufbxi_check_msg!` etc.) in BOTH stack modes, and of the
// no-msg forms under `error-stack`.
#[inline(never)]
pub(crate) fn fail_imp(
    uc: &Context,
    cond: Option<crate::native::error::FailStr>,
    func: Option<crate::native::error::FailStr>,
    line: u32,
) -> i32 {
    // Routes through the SAFE `fail_err` wrapper with the anchored `error_view()`;
    // the message-pointer unsafe is encapsulated inside `fail_err`/`fail_imp_err`.
    crate::native::error::fail_err(uc.error_view(), cond, func, line)
}

// ufbx.c:6657-6662 (`#else` branch of `UFBXI_FEATURE_ERROR_STACK`)
// `ufbxi_fail_imp_no_stack` — expansion target of the no-msg uc-context check
// macros when the error stack is disabled.
#[cfg(not(feature = "error-stack"))]
#[inline(never)]
pub(crate) fn fail_imp_no_stack(uc: &Context) -> i32 {
    // Routes through the SAFE `fail_err_no_stack` wrapper with the anchored
    // `error_view()`; no message pointers (no stack frame).
    crate::native::error::fail_err_no_stack(uc.error_view())
}

// -- Progress

// ufbx.c:6678-6681 `ufbxi_get_read_offset`
#[inline(always)]
pub(crate) fn get_read_offset(uc: &Context) -> u64 {
    // SAFETY: `data()` and `data_begin()` point into the same read buffer by
    // construction, so `offset_from` is in-bounds.
    uc.data_offset()
        .wrapping_add(to_size(unsafe { uc.data().offset_from(uc.data_begin()) }) as u64)
}

// ufbx.c:6683-6702 `ufbxi_report_progress`
// C: `ufbxi_nodiscard static ufbxi_noinline int` — `return 1` becomes
// `Ok(())`, the `ufbxi_check_msg` failure path returns `Err(Fail)`.
#[inline(never)]
pub(crate) fn report_progress(uc: &Context) -> Result<(), Fail> {
    if uc.opts_view().progress_cb().fn_.is_none() {
        return Ok(());
    }

    let read_offset: u64 = get_read_offset(uc);
    uc.set_latest_progress_bytes(read_offset);

    let mut progress = Progress {
        bytes_read: 0,
        bytes_total: 0,
    };
    progress.bytes_read = read_offset;
    progress.bytes_total = uc.progress_bytes_total();
    if progress.bytes_total < progress.bytes_read {
        progress.bytes_total = progress.bytes_read;
    }

    uc.set_progress_timer(1024);
    // C: `(uint32_t)uc->opts.progress_cb.fn(uc->opts.progress_cb.user, &progress)`
    // — the callback is `extern "C"`; the generated signature returns the enum
    // as a raw u32 (`RawEnum<ProgressResult>`).
    // SAFETY: `progress_cb.fn_` is `Some` (the `None` early-return above), and
    // it is invoked with the `user` pointer it was paired with in the same
    // callback struct — the C callback contract.
    let result: u32 = unsafe {
        (uc.opts_view().progress_cb().fn_.unwrap_unchecked())(
            uc.opts_view().progress_cb().user,
            &progress,
        )
    }
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
pub(crate) fn progress(uc: &Context, work_units: usize) -> Result<(), Fail> {
    if uc.opts_view().progress_cb().fn_.is_none() {
        return Ok(());
    }
    // C: `uc->progress_timer - (ptrdiff_t)work_units` — signed arithmetic on
    // values that stay tiny in practice; wrapping matches the release-build
    // C behavior if it ever did overflow.
    let left: isize = uc.progress_timer().wrapping_sub(work_units as isize);
    uc.set_progress_timer(left);
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
// Explicit loop (not `Iterator::find`) mirrors the C `ufbxi_for` control-flow for
// upstream line-correspondence and hosts the per-run SAFETY note. `name` is
// POOLED — compared with `==` by pointer VALUE, never dereferenced.
#[allow(clippy::manual_find)]
#[inline(never)]
pub(crate) fn find_child<'a>(node: &'a NodeView, name: *const u8) -> Option<&'a NodeView> {
    // C: `ufbxi_for(ufbxi_node, c, node->children, node->num_children)`
    // SAFETY: `children`/`num_children` describe a contiguous arena run (built via
    // `push_pop`), valid and stable for `node`'s lifetime `'a`.
    let children: SliceViewIter<'a, Node> =
        unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) };
    for c in children {
        if c.name() == name {
            return Some(c);
        }
    }
    None
}

// Retrieve the type of a given value
// ufbx.c:7721-7725 `ufbxi_get_val_type`
#[inline(always)]
pub(crate) fn get_val_type(node: &NodeView, ix: usize) -> ValueType {
    // C: `(ufbxi_value_type)((node->value_type_mask >> (ix*2)) & 0x3)` — the
    // 2-bits-per-value tag; the mask keeps the cast in range of the enum.
    // The `as i32` reproduces C's integer promotion of the `uint16_t` mask
    // (PORTING.md checklist 8): C shifts in `int`, so amounts of 16..31 yield 0
    // instead of overflowing a 16-bit shift.
    value_type_from_raw((((node.value_type_mask() as i32) >> (ix.wrapping_mul(2))) & 0x3) as u32)
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
pub(crate) unsafe fn get_val_at(node: &NodeView, ix: usize, fmt: u8, v: *mut c_void) -> bool {
    ufbxi_dev_assert!(ix < MAX_NON_ARRAY_VALUES);
    // `as i32` mirrors C's promotion of the `uint16_t` mask to `int`.
    let type_: ValueType = value_type_from_raw(
        (((node.value_type_mask() as i32) >> (ix.wrapping_mul(2))) & 0x3) as u32,
    );
    // `node->vals[ix]` reads the `vals` arm of the `ufbxi_node` union
    // (PORTING.md "Unions"); as in C the read happens only inside the arms that
    // need it, never for `'_'`, the `default:` arm, or a type mismatch.
    let vals: *mut Value = node.vals();
    match fmt {
        b'_' => true,
        b'I' => {
            if type_ == ValueType::Number {
                *(v as *mut i32) = (*vals.add(ix)).num.i as i32;
                true
            } else {
                false
            }
        }
        b'L' => {
            if type_ == ValueType::Number {
                *(v as *mut i64) = (*vals.add(ix)).num.i;
                true
            } else {
                false
            }
        }
        b'F' => {
            if type_ == ValueType::Number {
                *(v as *mut f32) = (*vals.add(ix)).num.f as f32;
                true
            } else {
                false
            }
        }
        b'D' => {
            if type_ == ValueType::Number {
                *(v as *mut f64) = (*vals.add(ix)).num.f;
                true
            } else {
                false
            }
        }
        b'R' => {
            if type_ == ValueType::Number {
                *(v as *mut Real) = (*vals.add(ix)).num.f as Real;
                true
            } else {
                false
            }
        }
        b'B' => {
            if type_ == ValueType::Number {
                *(v as *mut bool) = (*vals.add(ix)).num.i != 0;
                true
            } else {
                false
            }
        }
        b'Z' => {
            if type_ == ValueType::Number {
                if (*vals.add(ix)).num.i < 0 {
                    return false;
                }
                *(v as *mut usize) = (*vals.add(ix)).num.i as usize;
                true
            } else {
                false
            }
        }
        b'S' => {
            if type_ == ValueType::String {
                let src: SanitizedString = (*vals.add(ix)).s;
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
                let src: SanitizedString = (*vals.add(ix)).s;
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
                let src: SanitizedString = (*vals.add(ix)).s;
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
                let src: SanitizedString = (*vals.add(ix)).s;
                let dst: *mut *const u8 = v as *mut *const u8;
                *dst = src.raw_data;
                true
            } else {
                false
            }
        }
        b'b' => {
            if type_ == ValueType::String {
                let src: SanitizedString = (*vals.add(ix)).s;
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
pub(crate) fn get_array(node: &NodeView, fmt: u8) -> *mut ValueArray {
    if node.value_type_mask() != ValueType::Array as u16 {
        return core::ptr::null_mut();
    }
    let array: *mut ValueArray = node.array();
    let mut fmt = fmt;
    if fmt != b'?' {
        fmt = normalize_array_type(fmt, b'b');
        // SAFETY: `array` is the node's array-arm pointer to a valid arena
        // `ValueArray`; reading its `type_` byte cannot violate validity.
        if unsafe { (*array).type_ } != fmt {
            return core::ptr::null_mut();
        }
    }
    array
}

// ufbx.c:7805-7809 `ufbxi_get_val1`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn get_val1(node: &NodeView, fmt: *const u8, v0: *mut c_void) -> bool {
    if !get_val_at(node, 0, *fmt.add(0), v0) {
        return false;
    }
    true
}

// ufbx.c:7811-7816 `ufbxi_get_val2`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn get_val2(
    node: &NodeView,
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
    node: &NodeView,
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
    node: &NodeView,
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
    node: &NodeView,
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
    node: &NodeView,
    name: *const u8,
    fmt: *const u8,
    v0: *mut c_void,
) -> bool {
    let child: &NodeView = match find_child(node, name) {
        Some(child) => child,
        None => return false,
    };
    if !get_val_at(child, 0, *fmt.add(0), v0) {
        return false;
    }
    true
}

// ufbx.c:7853-7860 `ufbxi_find_val2`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn find_val2(
    node: &NodeView,
    name: *const u8,
    fmt: *const u8,
    v0: *mut c_void,
    v1: *mut c_void,
) -> bool {
    let child: &NodeView = match find_child(node, name) {
        Some(child) => child,
        None => return false,
    };
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
pub(crate) fn find_array(node: &NodeView, name: *const u8, fmt: u8) -> *mut ValueArray {
    let child: &NodeView = match find_child(node, name) {
        Some(child) => child,
        None => return core::ptr::null_mut(),
    };
    get_array(child, fmt)
}

// ufbx.c:7869-7877 `ufbxi_find_child_strcmp`
// Stays `unsafe fn`: it DEREFERENCES `name` (leading byte + `strcmp`). Returns
// `Option<&NodeView>` so results thread as views to `&NodeView`-taking callers.
#[allow(clippy::manual_find)]
pub(crate) unsafe fn find_child_strcmp<'a>(
    node: &'a NodeView,
    name: *const u8,
) -> Option<&'a NodeView> {
    let leading: u8 = *name.add(0);
    // C: `ufbxi_for(ufbxi_node, c, node->children, node->num_children)`
    // SAFETY: `children`/`num_children` describe a contiguous arena run (built via
    // `push_pop`), valid and stable for `node`'s lifetime `'a`.
    let children: SliceViewIter<'a, Node> =
        unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) };
    for c in children {
        if *c.name().add(0) != leading {
            continue;
        }
        if strcmp(c.name(), name) == 0 {
            return Some(c);
        }
    }
    None
}

// -- Element extra data allocation

// ufbx.c:7881-7896 `ufbxi_push_element_extra_size`
#[inline(never)]
#[must_use]
pub(crate) fn push_element_extra_size(uc: &Context, id: u32, size: usize) -> *mut c_void {
    if uc.element_extra_cap() <= id as usize {
        let old_cap: usize = uc.element_extra_cap();
        // C: `id + 1` is `uint32_t` arithmetic before the `size_t` conversion.
        ufbxi_check_return!(
            uc,
            // SAFETY: growing `uc`'s own paired `element_extra_arr`/
            // `element_extra_cap` growth state through its temp allocator (uc
            // construction invariant).
            unsafe {
                grow_array(
                    uc.ator_tmp_mut_ptr(),
                    uc.element_extra_arr_mut_ptr(),
                    uc.element_extra_cap_mut_ptr(),
                    id.wrapping_add(1) as usize
                )
            },
            core::ptr::null_mut(),
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->element_extra_arr)), (&uc->element_extra_arr), (&uc->element_extra_cap), (id + 1))"
        );
        // SAFETY: the grow above succeeded, so `element_extra_arr` holds
        // `element_extra_cap >= old_cap` entries; the zeroed run is exactly the
        // newly added tail.
        unsafe {
            core::ptr::write_bytes(
                uc.element_extra_arr().add(old_cap) as *mut u8,
                0,
                (uc.element_extra_cap() - old_cap) * size_of::<*mut c_void>(),
            );
        }
    }

    // SAFETY: `id < element_extra_cap()` holds here (either it already did, or
    // the grow above raised the cap past `id`), and every entry is initialized
    // — the newly grown tail by the `write_bytes` above.
    let existing: *mut c_void = unsafe { *uc.element_extra_arr().add(id as usize) };
    if !existing.is_null() {
        return existing;
    }

    // SAFETY: pushing onto `uc`'s own `tmp` buf through its raw-ptr getter.
    let extra: *mut c_void = unsafe { push_size_zero(uc.tmp_mut_ptr(), size, 1) };
    ufbxi_check_return!(uc, !extra.is_null(), core::ptr::null_mut(), "extra");
    // SAFETY: `id < element_extra_cap()` as above; `push_size_zero` touches
    // only `uc->tmp`, so the array is still that long.
    unsafe { *uc.element_extra_arr().add(id as usize) = extra };

    extra
}

// ufbx.c:7898-7905 `ufbxi_get_element_extra`
#[inline(never)]
pub(crate) fn get_element_extra(uc: &Context, id: u32) -> *mut c_void {
    if (id as usize) < uc.element_extra_cap() {
        // SAFETY: `id < element_extra_cap()` checked above, and
        // `element_extra_arr()` is valid for that many elements by construction.
        unsafe { *uc.element_extra_arr().add(id as usize) }
    } else {
        core::ptr::null_mut()
    }
}

// ufbx.c:7907 `#define ufbxi_push_element_extra(uc, id, type) (type*)ufbxi_push_element_extra_size((uc), (id), sizeof(type))`
#[inline(always)]
#[must_use]
pub(crate) fn push_element_extra<T>(uc: &Context, id: u32) -> *mut T {
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
    if uc.opts_view().retain_dom() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::PolygonVertexIndex.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::Edges.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                return true;
            } else if name == sp::Indexes.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::Points.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::KnotVector.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::KnotVectorU.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::KnotVectorV.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::PointsIndex.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::Normals.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::Normals.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::Materials.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::PolygonVertexIndex.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_animation() {
                    b'-'
                } else {
                    b'l'
                };
                return true;
            } else if name == sp::KeyValueFloat.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_animation() {
                    b'-'
                } else {
                    b'r'
                };
                return true;
            } else if name == sp::KeyAttrFlags.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_animation() {
                    b'-'
                } else {
                    b'i'
                };
                return true;
            } else if name == sp::KeyAttrDataFloat.as_ptr() {
                // The float data in a keyframe attribute array is represented as integers
                // in versions >= 7200 as some of the elements aren't actually floats (!)
                (*info).type_ = if uc.from_ascii() && uc.version() >= 7200 {
                    b'i'
                } else {
                    b'f'
                };
                if uc.opts_view().ignore_animation() {
                    (*info).type_ = b'-';
                }
                if uc.from_ascii() && uc.version() < 7200 {
                    (*info).flags |= ARRAY_FLAG_ACCURATE_F32;
                }
                return true;
            } else if name == sp::KeyAttrRefCount.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_animation() {
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
                (*info).type_ = if uc.opts_view().retain_dom() {
                    b'r'
                } else {
                    b'-'
                };
                return true;
            }
        }

        ParseState::Video => {
            if name == sp::Content.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_embedded() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::NormalsIndex.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::NormalsW.as_ptr() {
                (*info).type_ = if uc.retain_vertex_w() { b'r' } else { b'-' };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::LayerElementBinormal => {
            if name == sp::Binormals.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::BinormalsIndex.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::BinormalsW.as_ptr() {
                (*info).type_ = if uc.retain_vertex_w() { b'r' } else { b'-' };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::LayerElementTangent => {
            if name == sp::Tangents.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::TangentsIndex.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::TangentsW.as_ptr() {
                (*info).type_ = if uc.retain_vertex_w() { b'r' } else { b'-' };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::LayerElementUv => {
            if name == sp::UV.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::UVIndex.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::ColorIndex.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::VertexCreaseIndex.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags |= ARRAY_FLAG_TMP_BUF;
                return true;
            } else if name == sp::UV.as_ptr() {
                (*info).type_ = if uc.opts_view().retain_dom() {
                    b'r'
                } else {
                    b'-'
                };
                return true;
            } else if name == sp::UVIndex.as_ptr() {
                (*info).type_ = if uc.opts_view().retain_dom() {
                    b'i'
                } else {
                    b'-'
                };
                return true;
            }
        }

        ParseState::GeometryUvInfo => {
            if name == sp::TextureUV.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name == sp::TextureUVVerticeIndex.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            }
            if name == sp::Vertices.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
            if name == sp::Normals.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::Weights.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::BlendWeights.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::FullWeights.as_ptr() {
                (*info).type_ = b'r';
                (*info).flags |= if uc.blender_full_weights() {
                    ARRAY_FLAG_RESULT
                } else {
                    ARRAY_FLAG_TMP_BUF
                };
                return true;
            } else if strcmp(name, b"TransformAssociateModel\0".as_ptr()) == 0 {
                (*info).type_ = if uc.opts_view().retain_dom() {
                    b'r'
                } else {
                    b'-'
                };
                return true;
            }
        }

        ParseState::AssociateModel => {
            if name == sp::Transform.as_ptr() {
                (*info).type_ = if uc.opts_view().retain_dom() {
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
                (*info).type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                (*info).flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name == sp::Weights.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_geometry() {
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
                (*info).type_ = if uc.opts_view().ignore_animation() {
                    b'-'
                } else {
                    b'd'
                };
                return true;
            }
        }

        ParseState::Audio => {
            if name == sp::Content.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_embedded() {
                    b'-'
                } else {
                    b'C'
                };
                return true;
            }
        }

        _ => {
            if name == sp::BinaryData.as_ptr() {
                (*info).type_ = if uc.opts_view().ignore_embedded() {
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

        ParseState::Take if name == sp::Model.as_ptr() => {
            return true;
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
pub(crate) fn get_dom_node_imp(uc: &Context, node: Option<&NodeView>) -> *mut DomNode {
    let Some(node) = node else {
        return core::ptr::null_mut();
    };
    let mapping = DomMapping {
        // The node's address is the DOM-mapping key; `node.get()` recovers it
        // from the view without dropping the `uc`-anchored lifetime at callers.
        node_ptr: node.get() as usize,
        dom_node: core::ptr::null_mut(),
    };
    let hash = hash_uptr(mapping.node_ptr);
    // SAFETY: looking up in `uc`'s own `dom_node_map` through its raw-ptr
    // getter, with a key that is a live local of the map's item type; a
    // non-null result points at an entry owned by that map.
    unsafe {
        let result: *mut DomMapping = map_find(
            uc.dom_node_map_mut_ptr(),
            hash,
            &mapping as *const DomMapping as *const c_void,
        );
        if !result.is_null() {
            (*result).dom_node
        } else {
            core::ptr::null_mut()
        }
    }
}

// ufbx.c:10712-10716 `ufbxi_get_dom_node`
#[inline(always)]
#[must_use]
pub(crate) fn get_dom_node(uc: &Context, node: Option<&NodeView>) -> *mut DomNode {
    if !uc.opts_view().retain_dom() {
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
    let dst: *mut DomNode = push_zero(uc.result_mut_ptr(), 1);
    ufbxi_check!(uc, !dst.is_null(), "dst");
    ufbxi_check!(
        uc,
        !push_copy::<*mut DomNode>(uc.tmp_dom_nodes_mut_ptr(), 1, &dst).is_null(),
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
            uc.dom_node_map_mut_ptr(),
            hash,
            &mapping as *const DomMapping as *const c_void,
        );
        if result.is_null() {
            result = map_insert(
                uc.dom_node_map_mut_ptr(),
                hash,
                &mapping as *const DomMapping as *const c_void,
            );
            ufbxi_check!(uc, !result.is_null(), "result");
        }
        (*result).node_ptr = node as usize;
        (*result).dom_node = dst;
    }

    sp::push_string_place_str(uc.string_pool_mut_ptr(), &mut (*dst).name, false)?;

    if (*node).value_type_mask == ValueType::Array as u16 {
        let arr = (*node).content.array;
        let val: *mut DomValue = push_zero(uc.result_mut_ptr(), 1);
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
            let val: *mut DomValue = push_zero(uc.tmp_stack_mut_ptr(), 1);
            ufbxi_check!(uc, !val.is_null(), "val");
            (*val).value_str.data = EMPTY_CHAR.as_ptr();

            if mask == ValueType::String as u32 {
                (*val).type_ = DomValueType::String;
                // Bridge the raw parse-tree `node` to a view for the `get_val_at`
                // extractor (this fn keeps the raw node for its owned derefs).
                ufbxi_ignore!(get_val_at(
                    NodeView::from_ptr(node),
                    ix,
                    b'S',
                    &mut (*val).value_str as *mut String as *mut c_void
                ));
                ufbxi_ignore!(get_val_at(
                    NodeView::from_ptr(node),
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
        (*dst).values.data = push_pop::<DomValue>(uc.result_mut_ptr(), uc.tmp_stack_mut_ptr(), ix);
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
            uc.result_mut_ptr(),
            uc.tmp_dom_nodes_mut_ptr(),
            (*node).num_children as usize,
        ) as *const Ref<DomNode>;
        ufbxi_check!(uc, !(*dst).children.data.is_null(), "dst->children.data");
    }

    Ok(())
}

// ufbx.c:10813-10844 `ufbxi_retain_toplevel`
#[inline(never)]
pub(crate) unsafe fn retain_toplevel(uc: &Context, node: *mut Node) -> Result<(), Fail> {
    if uc.dom_parse_num_children() > 0 {
        let children: *mut *mut DomNode = push_pop(
            uc.result_mut_ptr(),
            uc.tmp_dom_nodes_mut_ptr(),
            uc.dom_parse_num_children(),
        );
        ufbxi_check!(uc, !children.is_null(), "children");
        (*uc.dom_parse_toplevel()).children.data = children as *const Ref<DomNode>;
        (*uc.dom_parse_toplevel()).children.count = uc.dom_parse_num_children();
        uc.set_dom_parse_num_children(0);
    }

    if !node.is_null() {
        retain_dom_node(uc, node, uc.dom_parse_toplevel_mut_ptr())?;
    } else {
        uc.set_dom_parse_toplevel(core::ptr::null_mut());

        // Called with NULL argument to finish retaining DOM, collect the final nodes to `ufbx_scene`.
        let num_top_nodes = uc.tmp_dom_nodes_view().num_items();
        let nodes: *mut *mut DomNode = push_pop(
            uc.result_mut_ptr(),
            uc.tmp_dom_nodes_mut_ptr(),
            num_top_nodes,
        );
        ufbxi_check!(uc, !nodes.is_null(), "nodes");

        let dom_root: *mut DomNode = push_zero(uc.result_mut_ptr(), 1);
        ufbxi_check!(uc, !dom_root.is_null(), "dom_root");

        (*dom_root).name.data = EMPTY_CHAR.as_ptr();
        (*dom_root).children.data = nodes as *const Ref<DomNode>;
        (*dom_root).children.count = num_top_nodes;

        uc.scene_view().set_dom_root(Some(Ref::from_ptr(dom_root)));
    }

    Ok(())
}

// ufbx.c:10846-10853 `ufbxi_retain_toplevel_child`
#[inline(never)]
pub(crate) unsafe fn retain_toplevel_child(uc: &Context, child: *mut Node) -> Result<(), Fail> {
    ufbx_assert!(!uc.dom_parse_toplevel().is_null());
    retain_dom_node(uc, child, core::ptr::null_mut())?;
    uc.set_dom_parse_num_children(uc.dom_parse_num_children().wrapping_add(1));

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
pub(crate) fn determine_format(uc: &Context) -> Result<(), Fail> {
    let mut format: FileFormat = uc.opts_view().file_format();

    if format == FileFormat::Unknown && !uc.opts_view().no_format_from_content() {
        crate::native::io::pause_progress(uc);

        let mut lookahead: usize = MIN_FILE_FORMAT_LOOKAHEAD;
        while format == FileFormat::Unknown && lookahead <= uc.opts_view().file_format_lookahead() {
            if lookahead > uc.data_size() {
                if uc.eof() {
                    break;
                }
                ufbxi_check!(
                    uc,
                    !crate::native::io::refill(uc, lookahead, false).is_null(),
                    "ufbxi_refill(uc, lookahead, false)"
                );
            }

            let data_size: usize = min_sz(lookahead, uc.data_size());
            ufbxi_check_msg!(uc, data_size > 0, "Empty file");

            // C: `for (uint32_t fmt = UFBX_FILE_FORMAT_FBX; fmt < UFBX_FILE_FORMAT_COUNT; fmt++)`
            let mut fmt: u32 = FileFormat::Fbx as u32;
            while fmt < FILE_FORMAT_COUNT {
                // SAFETY: `data_size <= uc.data_size()` (clamped above), so
                // `is_format` reads only inside the buffered read window; `fmt`
                // ranges over `Fbx..FILE_FORMAT_COUNT`, every one of which is a
                // valid `FileFormat` discriminant.
                let fmt_enum: FileFormat = unsafe { core::mem::transmute::<u32, FileFormat>(fmt) };
                // SAFETY: as above.
                if unsafe { is_format(uc.data(), data_size, fmt_enum) } {
                    format = fmt_enum;
                    break;
                }
                fmt += 1;
            }

            if lookahead >= uc.opts_view().file_format_lookahead() {
                break;
            } else if lookahead < usize::MAX / 2 {
                lookahead = min_sz(lookahead * 2, uc.opts_view().file_format_lookahead());
            } else {
                lookahead = usize::MAX;
            }
        }

        crate::native::io::resume_progress(uc)?;
    }

    if format == FileFormat::Unknown && !uc.opts_view().no_format_from_extension() {
        if uc.opts_view().filename_view().length() > 0 {
            // C: `ufbx_string extension = uc->opts.filename;`
            let mut extension: String = String::new_c(
                uc.opts_view().filename_view().data(),
                uc.opts_view().filename_view().length(),
            );
            // SAFETY: `extension` starts as `uc->opts.filename`, a
            // `data`/`length` pair valid for `length` bytes (opts invariant);
            // the scan only ever reads indices `< length` and only ever moves
            // `data` forward inside that same run, shrinking `length` to match.
            unsafe {
                let mut i: usize = extension.length;
                while i > 0 {
                    if *extension.data.add(i - 1) == b'.' {
                        extension.data = extension.data.add(i - 1);
                        extension.length -= i - 1;
                        break;
                    }
                    i -= 1;
                }
            }

            // SAFETY: `extension` is the valid `data`/`length` run established
            // above; the format strings are NUL-terminated literals.
            unsafe {
                if r#match(&extension, b"\\c\\.fbx\0".as_ptr()) {
                    format = FileFormat::Fbx;
                } else if r#match(&extension, b"\\c\\.obj\0".as_ptr()) {
                    format = FileFormat::Obj;
                } else if r#match(&extension, b"\\c\\.mtl\0".as_ptr()) {
                    format = FileFormat::Mtl;
                }
            }
        }
    }

    ufbxi_check_msg!(
        uc,
        format != FileFormat::Unknown,
        "Unrecognized file format",
        "format != UFBX_FILE_FORMAT_UNKNOWN"
    );
    uc.scene_view().metadata_view().set_file_format(format);

    Ok(())
}

// ufbx.c:11193-11240 `ufbxi_begin_parse`
#[inline(never)]
pub(crate) fn begin_parse(uc: &Context) -> Result<(), Fail> {
    let header: *const u8 = crate::native::io::peek_bytes(uc, BINARY_HEADER_SIZE);
    ufbxi_check!(uc, !header.is_null(), "header");

    // If the file starts with the binary magic parse it as binary, otherwise
    // treat it as an ASCII file.
    // SAFETY: a non-null `peek_bytes` result is readable for the requested
    // `BINARY_HEADER_SIZE` bytes, and `BINARY_MAGIC_SIZE <= BINARY_HEADER_SIZE`.
    if unsafe { memcmp(header, BINARY_MAGIC.as_ptr(), BINARY_MAGIC_SIZE) } == 0 {
        // The byte after the magic indicates endianness
        // SAFETY: the header window is `BINARY_HEADER_SIZE` bytes and the magic
        // plus the endian byte plus the 4-byte version word fit within it.
        let endian: u8 = unsafe { *header.add(BINARY_MAGIC_SIZE + 0) };
        uc.set_file_big_endian(endian != 0);

        // Read the version directly from the header
        // SAFETY: as above — this is the 4-byte version word inside the header
        // window.
        let mut version_word: *const u8 = unsafe { header.add(BINARY_MAGIC_SIZE + 1) };
        if uc.file_big_endian() {
            // SAFETY: `version_word` addresses 4 readable header bytes, which
            // is what the `(count=1, elem_size=4)` swap is told to read.
            version_word = unsafe {
                crate::native::parse_binary::swap_endian(uc, version_word as *const c_void, 1, 4)
            };
            ufbxi_check!(uc, !version_word.is_null(), "version_word");
        }
        // SAFETY: `version_word` is 4 readable bytes — either the header slice
        // above or the swap buffer that replaced it.
        uc.set_version(unsafe { read_u32(version_word) });

        // This is quite probably an FBX file..
        uc.set_sure_fbx(true);
        crate::native::io::consume_bytes(uc, BINARY_HEADER_SIZE);
    } else {
        uc.set_from_ascii(true);

        // Use the current read buffer as the initial parse buffer
        // C: `memset(&uc->ascii, 0, sizeof(uc->ascii));`
        // SAFETY: this is the init run for `uc`'s own `ascii` field, addressed
        // through its raw-ptr getter and sized by that field's type; the
        // cursors it is seeded with span `data .. data + yield_size +
        // data_size`, the buffered read window, so each lands in bounds
        // (one-past-the-end at most).
        unsafe {
            core::ptr::write_bytes(uc.ascii_mut_ptr() as *mut u8, 0, size_of::<Ascii>());
            uc.ascii_view().set_src(uc.data());
            uc.ascii_view()
                .set_src_yield(uc.data().add(uc.yield_size()));
            uc.ascii_view()
                .set_src_end(uc.data().add(uc.data_size() + uc.yield_size()));
        }

        // Initialize the first token
        // SAFETY: the token out-param is `uc`'s own `ascii.token` field,
        // addressed through its view.
        unsafe {
            crate::native::parse_ascii::ascii_next_token(uc, uc.ascii_view().token_mut_ptr())?;
        }

        // Default to version 7400 if not found in header
        if uc.version() > 0 {
            uc.set_sure_fbx(true);
        } else {
            if !uc.opts_view().strict() {
                uc.set_version(7400);
            }
            ufbxi_check_msg!(uc, uc.version() > 0, "Not an FBX file", "uc->version > 0");
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
    if uc.from_ascii() {
        crate::native::parse_ascii::ascii_parse_node(uc, 0, state, p_end, buf, true)?;
    } else {
        crate::native::parse_binary::binary_parse_node(uc, 0, state, p_end, buf, true)?;
    }

    Ok(())
}

// ufbx.c:11253-11330 `ufbxi_parse_toplevel`
#[inline(never)]
pub(crate) unsafe fn parse_toplevel(uc: &Context, name: *const u8) -> Result<(), Fail> {
    // C: `ufbxi_for(ufbxi_node, node, uc->top_nodes, uc->top_nodes_len)`
    let mut node: *mut Node = uc.top_nodes();
    let node_end: *mut Node = add_ptr(node, uc.top_nodes_len());
    while node != node_end {
        if (*node).name == name {
            uc.set_top_node(node);
            uc.set_top_child_index(0);
            return Ok(());
        }
        node = node.add(1);
    }

    // Reached end and not found in cache
    if uc.parsed_to_end() {
        uc.set_top_node(core::ptr::null_mut());
        uc.set_top_child_index(0);
        return Ok(());
    }

    loop {
        // Parse the next top-level node
        let mut end: bool = false;
        if uc.from_ascii() {
            crate::native::parse_ascii::ascii_parse_node(
                uc,
                0,
                ParseState::Root,
                &mut end,
                uc.tmp_mut_ptr(),
                false,
            )?;
        } else {
            crate::native::parse_binary::binary_parse_node(
                uc,
                0,
                ParseState::Root,
                &mut end,
                uc.tmp_mut_ptr(),
                false,
            )?;
        }

        // Top-level node not found
        if end {
            uc.set_top_node(core::ptr::null_mut());
            uc.set_top_child_index(0);
            uc.set_parsed_to_end(true);
            if uc.opts_view().retain_dom() {
                retain_toplevel(uc, core::ptr::null_mut())?;
            }

            // Not needed anymore
            buf_free(uc.tmp_parse_mut_ptr());

            return Ok(());
        }

        uc.set_top_nodes_len(uc.top_nodes_len() + 1);
        ufbxi_check!(
            uc,
            grow_array(
                uc.ator_tmp_mut_ptr(),
                uc.top_nodes_mut_ptr(),
                uc.top_nodes_cap_mut_ptr(),
                uc.top_nodes_len()
            ),
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->top_nodes)), (&uc->top_nodes), (&uc->top_nodes_cap), (uc->top_nodes_len))"
        );
        let node: *mut Node = uc.top_nodes().add(uc.top_nodes_len() - 1);
        pop::<Node>(uc.tmp_stack_mut_ptr(), 1, node);
        if uc.opts_view().retain_dom() {
            retain_toplevel(uc, node)?;
        }

        // Return if we parsed the right one
        if (*node).name == name {
            uc.set_top_node(node);
            uc.set_top_child_index(usize::MAX);
            return Ok(());
        }

        // If not we need to parse all the children of the node for later
        let mut num_children: u32 = 0;
        let state: ParseState = update_parse_state(ParseState::Root, (*node).name);
        if uc.has_next_child() {
            loop {
                parse_toplevel_child_imp(uc, state, uc.tmp_mut_ptr(), &mut end)?;
                if end {
                    break;
                }
                num_children += 1;
            }
        }

        (*node).num_children = num_children;
        (*node).children = push_pop::<Node>(
            uc.tmp_mut_ptr(),
            uc.tmp_stack_mut_ptr(),
            num_children as usize,
        );
        ufbxi_check!(uc, !(*node).children.is_null(), "node->children");

        if uc.opts_view().retain_dom() {
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
    if uc.top_node().is_null() {
        *p_node = core::ptr::null_mut();
        return Ok(());
    }

    if uc.top_child_index() == usize::MAX {
        // Parse children on demand
        if tmp_buf.is_null() {
            buf_clear(uc.tmp_parse_mut_ptr());
        }
        let mut end = false;
        let state: ParseState = update_parse_state(ParseState::Root, (*uc.top_node()).name);
        let buf: *mut Buf = if !tmp_buf.is_null() {
            tmp_buf
        } else {
            uc.tmp_parse_mut_ptr()
        };
        parse_toplevel_child_imp(uc, state, buf, &mut end)?;
        if end {
            *p_node = core::ptr::null_mut();
        } else {
            // Parse to either reused `uc->top_child` or push if retaining to `tmp_buf`.
            let mut dst: *mut Node = uc.top_child_mut_ptr();
            if !tmp_buf.is_null() {
                dst = push_zero::<Node>(tmp_buf, 1);
                ufbxi_check!(uc, !dst.is_null(), "dst");
            }

            pop::<Node>(uc.tmp_stack_mut_ptr(), 1, dst);
            *p_node = dst;

            if uc.opts_view().retain_dom() {
                retain_toplevel_child(uc, dst)?;
            }
        }
    } else {
        // Iterate already parsed nodes
        let child_index = uc.top_child_index();
        if child_index == (*uc.top_node()).num_children as usize {
            *p_node = core::ptr::null_mut();
        } else {
            uc.set_top_child_index(uc.top_child_index().wrapping_add(1));
            *p_node = (*uc.top_node()).children.add(child_index);
        }
    }

    Ok(())
}

// ufbx.c:11379-11407 `ufbxi_parse_legacy_toplevel`
#[inline(never)]
pub(crate) fn parse_legacy_toplevel(uc: &Context) -> Result<(), Fail> {
    ufbx_assert!(uc.top_nodes_len() == 0);

    let mut end: bool = false;
    // SAFETY: the `end` out-param is an unaliased local; the destination buf is
    // `uc`'s own `tmp`, addressed through its raw-ptr getter.
    unsafe {
        if uc.from_ascii() {
            crate::native::parse_ascii::ascii_parse_node(
                uc,
                0,
                ParseState::Root,
                &mut end,
                uc.tmp_mut_ptr(),
                true,
            )?;
        } else {
            crate::native::parse_binary::binary_parse_node(
                uc,
                0,
                ParseState::Root,
                &mut end,
                uc.tmp_mut_ptr(),
                true,
            )?;
        }
    }

    // Top-level node not found
    if end {
        uc.set_top_node(core::ptr::null_mut());
        uc.set_top_child_index(0);
        uc.set_parsed_to_end(true);
        return Ok(());
    }

    // SAFETY: the parse above pushed the node onto `uc`'s own `tmp_stack`, so
    // popping one `Node` into `uc`'s own `legacy_node` field (both addressed
    // through their raw-ptr getters) matches what is stored there.
    unsafe { pop::<Node>(uc.tmp_stack_mut_ptr(), 1, uc.legacy_node_mut_ptr()) };
    uc.set_top_child_index(0);
    uc.set_top_node(uc.legacy_node_mut_ptr());

    if uc.opts_view().retain_dom() {
        // SAFETY: `legacy_node` was just populated by the `pop` above and is
        // `uc`'s own field.
        unsafe { retain_toplevel(uc, uc.legacy_node_mut_ptr())? };
    }

    Ok(())
}

// -- Setup (ufbx.c:11409-11760)

// ufbx.c:11411-11429 `ufbxi_load_strings`
#[inline(never)]
pub(crate) fn load_strings(uc: &Context) -> Result<(), Fail> {
    // C: `#if defined(UFBX_REGRESSION) ufbx_string reg_prev = ufbx_empty_string; #endif`
    #[cfg(feature = "regression")]
    let mut reg_prev: String = crate::native::api::EMPTY_STRING.0;

    // Push all the global 'ufbxi_*' strings into the pool without copying them
    // This allows us to compare name pointers to the global values
    // C: `ufbxi_for(const ufbx_string, str, ufbxi_strings, ufbxi_arraycount(ufbxi_strings))`
    for str_ in sp::STRINGS.0.iter() {
        #[cfg(feature = "regression")]
        // SAFETY: `STRINGS` holds NUL-terminated static literals paired with
        // their lengths, which is what `strlen`/`str_less` read.
        unsafe {
            ufbx_assert!(crate::native::error::strlen(str_.data) == str_.length);
            ufbx_assert!(sp::str_less(reg_prev, *str_));
            reg_prev = *str_;
        }
        ufbxi_check!(
            uc,
            // SAFETY: interning into `uc`'s own string pool through its raw-ptr
            // getter; `str_` is a `data`/`length` pair over a static literal,
            // and `'static` outlives the pool, which is why the no-copy
            // (`copy == false`) intern is sound here.
            !unsafe {
                sp::push_string_imp(
                    uc.string_pool_mut_ptr(),
                    str_.data,
                    str_.length,
                    core::ptr::null_mut(),
                    false,
                    true,
                )
            }
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
        uc.prop_type_map_mut_ptr(),
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
pub(crate) unsafe fn find_prop_with_key<'a, M: Mode>(
    props: &'a View<Props, M>,
    name: *const u8,
    key: u32,
) -> Option<&'a View<Prop, M>> {
    let mut props: Option<&'a View<Props, M>> = Some(props);
    while let Some(cur) = props {
        let prop_data: *mut Prop = cur.props_data();
        let mut begin: usize = 0;
        let mut end: usize = cur.props_count();
        while end - begin >= 16 {
            let mid: usize = (begin + end) >> 1;
            let p: *const Prop = prop_data.add(mid);
            if (*p)._internal_key < key {
                begin = mid + 1;
            } else {
                end = mid;
            }
        }

        end = cur.props_count();
        while begin < end {
            let p: *const Prop = prop_data.add(begin);
            if (*p)._internal_key > key {
                break;
            }
            if (*p).name.data == name && ((*p).flags.raw() & PropFlags::NO_VALUE.raw()) == 0 {
                // Mode-generic mint from the STORED run pointer (`props_data()`
                // value read) — adequate provenance for either mode.
                return Some(View::<Prop, M>::mint(p as *mut Prop));
            }
            begin += 1;
        }

        props = cur.defaults();
    }

    None
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
pub(crate) unsafe fn find_prop<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
) -> Option<&View<Prop, M>> {
    let key = (*name.add(0) as u32) << 24
        | (*name.add(1) as u32) << 16
        | (*name.add(2) as u32) << 8
        | (*name.add(3) as u32);
    find_prop_with_key(props, name, key)
}

// ufbx.c:11520-11528 `ufbxi_find_real`
#[inline(always)]
pub(crate) unsafe fn find_real<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def: Real,
) -> Real {
    match find_prop(props, name) {
        // C-parity: `prop->value_real` is the `ufbx_prop` value union's first
        // real; the generated struct keeps only `value_vec4` (same mapping as
        // `find_vec3` below).
        Some(prop) => prop.value_vec4().x,
        None => def,
    }
}

// ufbx.c:11530-11539 `ufbxi_find_vec3`
#[inline(always)]
pub(crate) unsafe fn find_vec3<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def_x: Real,
    def_y: Real,
    def_z: Real,
) -> Vec3 {
    match find_prop(props, name) {
        // C-parity: `prop->value_vec3` is the `ufbx_prop` value union's 3-real
        // view; the generated struct keeps only `value_vec4` (see
        // `native::read::read_property`).
        Some(prop) => prop.value_vec3(),
        None => Vec3 {
            x: def_x,
            y: def_y,
            z: def_z,
        },
    }
}

// ufbx.c:11541-11549 `ufbxi_find_int`
#[inline(always)]
pub(crate) unsafe fn find_int<M: Mode>(props: &View<Props, M>, name: *const u8, def: i64) -> i64 {
    match find_prop(props, name) {
        Some(prop) => prop.value_int(),
        None => def,
    }
}

// ufbx.c:11551-11564 `ufbxi_find_enum`
// Ported with the `// -- Scene processing` unit that first needs it
// (`ufbxi_fetch_texture_layers`, ufbx.c:19251).
#[inline(always)]
pub(crate) unsafe fn find_enum<M: Mode>(
    props: &View<Props, M>,
    name: *const u8,
    def: i64,
    max_value: i64,
) -> i64 {
    match find_prop(props, name) {
        Some(prop) => {
            let value: i64 = prop.value_int();
            if value >= 0 && value <= max_value {
                value
            } else {
                def
            }
        }
        None => def,
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
pub(crate) fn init_node_prop_names(uc: &Context) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: growing `uc`'s own `node_prop_set` map through its raw-ptr
        // getter, with the item type it was constructed for.
        unsafe {
            crate::native::hash::map_grow::<*const u8>(
                uc.node_prop_set_mut_ptr(),
                NODE_PROP_NAMES.0.len()
            )
        },
        "ufbxi_map_grow_size((&uc->node_prop_set), sizeof(const char*), ((sizeof(ufbxi_node_prop_names) / sizeof(*(ufbxi_node_prop_names)))))"
    );
    // C: `for (size_t i = 0; i < ufbxi_arraycount(ufbxi_node_prop_names); i++)`
    let mut i: usize = 0;
    while i < NODE_PROP_NAMES.0.len() {
        let name: *const u8 = NODE_PROP_NAMES.0[i];
        // SAFETY: `name` is a NUL-terminated static literal, so `strlen` finds
        // its terminator and the `'static` bytes outlive the pool — which is
        // what makes the no-copy (`copy == false`) intern into `uc`'s own
        // string pool sound.
        let pooled: *const u8 = unsafe {
            sp::push_string_imp(
                uc.string_pool_mut_ptr(),
                name,
                crate::native::error::strlen(name),
                core::ptr::null_mut(),
                false,
                true,
            )
        };
        ufbxi_check!(uc, !pooled.is_null(), "pooled");
        let hash: u32 = crate::native::hash::hash_ptr!(pooled);
        // SAFETY: inserting into `uc`'s own `node_prop_set` through its raw-ptr
        // getter, keyed by a live local of the map's item type; a non-null
        // result is a writable entry owned by that map.
        unsafe {
            let entry: *mut *const u8 = map_insert::<*const u8>(
                uc.node_prop_set_mut_ptr(),
                hash,
                &pooled as *const *const u8 as *const c_void,
            );
            ufbxi_check!(uc, !entry.is_null(), "entry");
            *entry = pooled;
        }
        i += 1;
    }

    Ok(())
}

// ufbx.c:11736-11744 `ufbxi_is_node_property_name`
pub(crate) unsafe fn is_node_property_name(uc: &Context, name: *const u8) -> bool {
    // You need to call `ufbxi_init_node_prop_names()` before calling this
    ufbx_assert!(uc.node_prop_set_view().size() > 0);

    // C takes the address of the parameter itself (`&name`) as the map key.
    let name: *const u8 = name;
    let hash = crate::native::hash::hash_ptr!(name);
    let entry: *mut *const u8 = map_find(
        uc.node_prop_set_mut_ptr(),
        hash,
        &name as *const *const u8 as *const c_void,
    );
    !entry.is_null()
}

// ufbx.c:11746-11760 `ufbxi_load_maps`
#[inline(never)]
pub(crate) fn load_maps(uc: &Context) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        // SAFETY: growing `uc`'s own `prop_type_map` through its raw-ptr
        // getter, with the item type it was constructed for.
        unsafe {
            crate::native::hash::map_grow::<PropTypeName>(
                uc.prop_type_map_mut_ptr(),
                PROP_TYPE_NAMES.0.len()
            )
        },
        "ufbxi_map_grow_size((&uc->prop_type_map), sizeof(ufbxi_prop_type_name), ((sizeof(ufbxi_prop_type_names) / sizeof(*(ufbxi_prop_type_names)))))"
    );
    // C: `ufbxi_for(const ufbxi_prop_type_name, name, ufbxi_prop_type_names, ...)`
    for name in PROP_TYPE_NAMES.0.iter() {
        // SAFETY: `name.name` is a NUL-terminated static literal, so `strlen`
        // finds its terminator and the `'static` bytes outlive the pool — what
        // makes the no-copy (`copy == false`) intern into `uc`'s own string
        // pool sound.
        let pooled: *const u8 = unsafe {
            sp::push_string_imp(
                uc.string_pool_mut_ptr(),
                name.name,
                crate::native::error::strlen(name.name),
                core::ptr::null_mut(),
                false,
                true,
            )
        };
        ufbxi_check!(uc, !pooled.is_null(), "pooled");
        let hash: u32 = crate::native::hash::hash_ptr!(pooled);
        // SAFETY: inserting into `uc`'s own `prop_type_map` through its raw-ptr
        // getter, keyed by a live local; a non-null result is a writable entry
        // owned by that map.
        unsafe {
            let entry: *mut PropTypeName = map_insert::<PropTypeName>(
                uc.prop_type_map_mut_ptr(),
                hash,
                &pooled as *const *const u8 as *const c_void,
            );
            ufbxi_check!(uc, !entry.is_null(), "entry");
            (*entry).type_ = name.type_;
            (*entry).name = pooled;
        }
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
            let mesh_ptr = &raw mut (*imp_ptr).mesh;
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
            let node = NodeView::from_ptr(&mut node);
            assert_eq!(get_val_type(node, 0), ValueType::Number);
            assert_eq!(get_val_type(node, 1), ValueType::String);
            assert_eq!(get_val_type(node, 2), ValueType::None);
            assert_eq!(get_val_type(node, 3), ValueType::Array);
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
            let node = NodeView::from_ptr(&mut node);
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
            let node = NodeView::from_ptr(&mut node);
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
            let node = NodeView::from_ptr(&mut node);
            let child0: &NodeView = NodeView::from_ptr(children.as_mut_ptr());
            let child1: &NodeView = NodeView::from_ptr(children.as_mut_ptr().add(1));
            assert_eq!(
                find_child(node, name_a.as_ptr()).map(NodeView::get),
                Some(children.as_mut_ptr())
            );
            // Pointer comparison: an equal-but-unpooled name does not match.
            assert!(find_child(node, name_b_copy.as_ptr()).is_none());
            // ...while the strcmp variant does.
            assert_eq!(
                find_child_strcmp(node, name_b_copy.as_ptr()).map(NodeView::get),
                Some(children.as_mut_ptr().add(1))
            );

            assert_eq!(get_array(child1, real_type), &mut array as *mut _);
            // 'r' normalizes to the array's concrete type in either Real mode.
            assert_eq!(get_array(child1, b'r'), &mut array as *mut _);
            assert!(get_array(child1, b'i').is_null());
            // '?' skips the type check entirely.
            assert_eq!(get_array(child1, b'?'), &mut array as *mut _);
            // A non-array node has no array.
            assert!(get_array(child0, b'?').is_null());

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
            assert!(get_dom_node(uc_ptr, Some(NodeView::from_ptr(&mut root))).is_null());
            uc.opts.retain_dom = true;
            assert_eq!(
                get_dom_node(uc_ptr, Some(NodeView::from_ptr(&mut root))),
                dom
            );
            assert_eq!(
                get_dom_node(uc_ptr, Some(NodeView::from_ptr(&mut leaf))),
                child
            );
            assert!(get_dom_node(uc_ptr, None).is_null());

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
