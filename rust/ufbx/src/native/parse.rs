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
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
use core::ffi::c_void;
use core::mem::size_of;

use crate::generated::{
    DomNode, DomValue, DomValueType, ElementType, Error, Exporter, FileFormat, InflateRetain,
    Matrix, MirrorAxis, Progress, ProgressResult, Prop, PropFlags, PropType, Props, Quat,
    RawLoadOpts, Scene, TextureFile, Transform, Vec3, Vec4,
};
use crate::native::allocator::{grow_array, Allocator};
use crate::native::buf::{buf_clear, buf_free, pop, push_size_zero, Buf, BufView};
use crate::native::error::{
    c_strcmp, memcmp, strcmp, strncmp, ufbxi_check, ufbxi_check_msg, ufbxi_check_return,
    ufbxi_fail, Fail, EMPTY_CHAR,
};
use crate::native::hash::{hash_uptr, Map, PtrId};
use crate::native::parse_ascii::is_space;
use crate::native::parse_binary::{BINARY_HEADER_SIZE, BINARY_MAGIC, BINARY_MAGIC_SIZE};
use crate::native::platform::{
    add_ptr, min_sz, read_u32, to_size, ufbx_assert, ufbxi_dev_assert, ufbxi_unreachable,
    AtomicCounter,
};
use crate::native::string_pool as sp;
use crate::native::string_pool::{SanitizedString, StringPool};
use crate::native::thread::{ThreadPool, THREAD_GROUP_COUNT};
use crate::native::view::{
    view_project, view_raw_const, view_raw_mut, view_read, view_read_shared, view_write,
};
use crate::native::view::{Mode, SliceViewIter, View};
use crate::native::warnings::Warnings;
use crate::prelude::{Blob, Real, Ref, ScalarView, String, StringView};

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

// Read surface over an array descriptor returned by `get_array`/`find_array`;
// mode-generic because the descriptor is only ever read (the payload run it
// names is addressed through the raw `data()` pointer, which carries its own
// stored provenance).
impl<M: Mode> View<ValueArray, M> {
    #[inline(always)]
    pub(crate) fn data(&self) -> *mut c_void {
        view_read_shared!(self, data)
    }
    #[inline(always)]
    pub(crate) fn size(&self) -> usize {
        view_read_shared!(self, size)
    }
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
        view_read!(self, name)
    }
    #[inline(always)]
    pub(crate) fn num_children(&self) -> u32 {
        view_read!(self, num_children)
    }
    #[inline(always)]
    pub(crate) fn name_len(&self) -> u8 {
        view_read!(self, name_len)
    }
    #[inline(always)]
    pub(crate) fn value_type_mask(&self) -> u16 {
        view_read!(self, value_type_mask)
    }
    #[inline(always)]
    pub(crate) fn children(&self) -> *mut Node {
        view_read!(self, children)
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
}

// The plain `Prop` field reads (`value_vec4`/`value_int`/`value_str`/
// `value_blob`/`name`/`type_`/`flags`) are generated accessors — see
// src/generated_views.rs; only the union-shaped read below is hand-written.
impl<M: Mode> View<Prop, M> {
    #[inline(always)]
    pub(crate) fn value_vec3(&self) -> Vec3 {
        // C-parity: the `ufbx_prop` value union's 3-real view; the generated
        // struct keeps only `value_vec4` (same mapping as `find_vec3`).
        // SAFETY: reading the first three reals of a valid arena `Prop`.
        unsafe { *(&raw const (*self.as_ptr()).value_vec4 as *const Vec3) }
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
//
// Private: the raw arithmetic is reachable only through `ImpHandle::from_payload`
// below, so every recovery in the crate goes through that one audited seam.
#[inline(always)]
fn get_imp<T>(ptr: *mut c_void) -> *mut T {
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

// Rust-side consolidation of the `ufbxi_*_imp` finalization idiom — no single
// C counterpart, but every imp creation site in ufbx.c closes with the same
// statement group, eg. ufbx.c:27871-27877 (`ufbxi_finalize_line_curve`):
//
//     ufbxi_init_ref(&tc->imp->refcount, UFBXI_LINE_CURVE_IMP_MAGIC, parent);
//     tc->imp->magic = UFBXI_LINE_CURVE_IMP_MAGIC;
//     tc->imp->curve = tc->line;
//     tc->imp->refcount.ator = tc->ator_result;
//     tc->imp->refcount.buf = tc->result;
//
// The recovery half of the imp contract: what `ImpHandle` (C's `ufbxi_get_imp`
// users) needs from a `ufbxi_*_imp` struct — the magic to check and the two
// header fields every imp carries. Split from `ImpHeader` because the
// `cfg(not(feature = "geometry-cache"))` `GeometryCacheImp` has NO payload
// field (ufbx.c:24765-24769) yet `ufbx_free_geometry_cache` /
// `ufbx_retain_geometry_cache` still recover and magic-check it in that build.
//
// # Safety
//
// Implementors are `#[repr(C)]` structs whose leading field is the `Refcount`
// header, whose `MAGIC` is the `UFBXI_*_IMP_MAGIC` that `ufbxi_get_imp` users
// check, whose `Payload` is the public struct `ufbxi_get_imp` recovers Self
// from (stored at offset `size_of::<Refcount>()` when the build has it), and
// whose `header_parts` returns exactly the `(refcount, magic)` field
// projections of the passed `imp` (so a pointer valid for the imp is valid for
// both).
pub(crate) unsafe trait ImpRecover: Sized {
    // The public struct the header wraps (C: `imp->scene`, `imp->mesh`, ...).
    type Payload;
    // C: the value stamped into both `imp->magic` and the refcount type magic.
    const MAGIC: u32;

    // The two header fields shared by every imp layout, in one projection.
    //
    // # Safety
    //
    // `imp` addresses a live (possibly uninitialized) `Self`.
    unsafe fn header_parts(imp: *mut Self) -> (*mut Refcount, *mut u32);
}

// The finalization half: `ImpHeader` pins the full `#[repr(C)]`
// header-then-payload-then-magic layout — the same layout the `offset_of`
// asserts pin for `ufbxi_get_imp` — so `finish_imp` can write that group once,
// with one safety argument, instead of five raw-pointer writes per site.
//
// # Safety
//
// As `ImpRecover`, and `parts` returns exactly the
// `(refcount, payload, magic)` field projections of the passed `imp` (so a
// pointer valid for the imp is valid for all three).
pub(crate) unsafe trait ImpHeader: ImpRecover {
    // The three fields `finish_imp` writes, in one projection.
    //
    // # Safety
    //
    // `imp` addresses a live (possibly uninitialized) `Self`.
    unsafe fn parts(imp: *mut Self) -> (*mut Refcount, *mut Self::Payload, *mut u32);
}

// SAFETY: `#[repr(C)]` with `refcount` leading, `SCENE_IMP_MAGIC` is the magic
// `ufbxi_get_imp(ufbxi_scene_imp, ...)` users check, `Payload` is the public
// struct at the pinned offset, and `header_parts` projects the two named
// fields of the passed `imp`. Recovery-only: scenes are finalized manually
// (the C statement group is interleaved with scene-specific writes, e.g.
// `string_buf`), not through `finish_imp`.
unsafe impl ImpRecover for SceneImp {
    type Payload = Scene;
    const MAGIC: u32 = crate::native::allocator::SCENE_IMP_MAGIC;

    #[inline(always)]
    unsafe fn header_parts(imp: *mut Self) -> (*mut Refcount, *mut u32) {
        // SAFETY: the caller vouches `imp` addresses a live `SceneImp`, so
        // these field projections stay inside that allocation.
        unsafe { (&raw mut (*imp).refcount, &raw mut (*imp).magic) }
    }
}

// SAFETY: `#[repr(C)]` with `refcount` leading, `MESH_IMP_MAGIC` is the magic
// `ufbxi_get_imp(ufbxi_mesh_imp, ...)` users check, `Payload` is the public
// struct at the pinned offset, and `header_parts` projects the two named
// fields of the passed `imp`.
unsafe impl ImpRecover for MeshImp {
    type Payload = crate::generated::Mesh;
    const MAGIC: u32 = crate::native::allocator::MESH_IMP_MAGIC;

    #[inline(always)]
    unsafe fn header_parts(imp: *mut Self) -> (*mut Refcount, *mut u32) {
        // SAFETY: the caller vouches `imp` addresses a live `MeshImp`, so these
        // field projections stay inside that allocation.
        unsafe { (&raw mut (*imp).refcount, &raw mut (*imp).magic) }
    }
}

// SAFETY: `parts` projects the three named fields of the passed `imp` (layout
// pinned by the `offset_of` assert above).
unsafe impl ImpHeader for MeshImp {
    #[inline(always)]
    unsafe fn parts(imp: *mut Self) -> (*mut Refcount, *mut Self::Payload, *mut u32) {
        // SAFETY: the caller vouches `imp` addresses a live `MeshImp`, so these
        // field projections stay inside that allocation.
        unsafe {
            (
                &raw mut (*imp).refcount,
                &raw mut (*imp).mesh,
                &raw mut (*imp).magic,
            )
        }
    }
}

// Non-owning recovery handle over a `ufbxi_*_imp` allocation — C's
// `ufbxi_get_imp(type, ptr)` idiom as one audited seam. Recover the imp from a
// public payload pointer once, then reach its header through typed projections
// instead of per-site pointer arithmetic behind the payload.
//
// Deliberately NOT an owning/RAII handle: imp lifetimes are parent-graph- and
// ABI-shaped (`Refcount::parent` edges, released recursively by
// `native::api::release_ref`), never Rust-scope-shaped — so there is no
// `Drop`, and `retain`/`release` mirror C's explicit `ufbxi_retain_ref` /
// `ufbxi_release_ref` calls exactly.
pub(crate) struct ImpHandle<T: ImpRecover>(*mut T);

impl<T: ImpRecover> ImpHandle<T> {
    // C: `ufbxi_get_imp(ufbxi_*_imp, ptr)`.
    //
    // # Safety
    //
    // `payload` is the payload pointer of a live `T` imp allocation handed out
    // by this library (every imp creation site exposes its wide pointer, so
    // the header BEFORE the payload is recoverable via exposed provenance),
    // and that allocation stays live for every use of the returned handle.
    #[inline(always)]
    pub(crate) unsafe fn from_payload(payload: *mut T::Payload) -> Self {
        Self(get_imp(payload as *mut c_void))
    }

    #[inline(always)]
    pub(crate) fn as_ptr(&self) -> *mut T {
        self.0
    }

    // C: `imp->magic == UFBXI_*_IMP_MAGIC` — the defensive stale-handle check
    // every public retain/free entry runs before touching the refcount.
    #[inline(always)]
    pub(crate) fn has_magic(&self) -> bool {
        // SAFETY: the imp is live per the `from_payload` vouch; reading its
        // own `magic` field.
        unsafe { *T::header_parts(self.0).1 == T::MAGIC }
    }

    #[inline(always)]
    pub(crate) fn refcount_ptr(&self) -> *mut Refcount {
        // SAFETY: the imp is live per the `from_payload` vouch; projecting its
        // own `refcount` field.
        unsafe { T::header_parts(self.0).0 }
    }

    // C: `ufbxi_retain_ref(&imp->refcount)`.
    #[inline(always)]
    pub(crate) fn retain(&self) {
        // SAFETY: the live imp's own refcount header, initialized at creation
        // per the `from_payload` vouch.
        unsafe { crate::native::api::retain_ref(self.refcount_ptr()) };
    }

    // C: `ufbxi_release_ref(&imp->refcount)`. Consumes the handle: dropping
    // the last reference frees the allocation the handle points into.
    #[inline(always)]
    pub(crate) fn release(self) {
        // SAFETY: as `retain` — and the handle is consumed, so nothing uses it
        // after the potential free.
        unsafe { crate::native::api::release_ref(self.refcount_ptr()) };
    }
}

// The must-consume proof that `finish_imp` finalized an imp: refcount 1, owned
// by nobody until the caller commits it somewhere explicit. A linear-type
// emulation with a drop bomb — `Drop` NEVER releases (imp teardown on error
// paths is arena/allocator teardown, and an automatic release would
// double-free against it); it only trips a debug-build assert, so a code path
// that loses the value fails tests/Miri while costing release builds nothing.
// The exits are `into_payload` (commit across the ABI) and `forget` (ownership
// recorded elsewhere); an explicit `release` exit can be added when an
// owning-error-path consumer appears.
#[must_use = "consume the FinishedImp explicitly (into_payload/forget) — dropping \
              it means the finished imp's ownership was never decided"]
pub(crate) struct FinishedImp<T: ImpHeader>(*mut T);

impl<T: ImpHeader> FinishedImp<T> {
    // Commit: hand the payload pointer across the public ABI
    // (C: `return &imp->payload;` at the end of every creation entry point).
    #[inline(always)]
    pub(crate) fn into_payload(self) -> *mut T::Payload {
        // SAFETY: `FinishedImp`s are created only by `finish_imp`, on an imp it
        // just finalized and left live; projecting that imp's own payload field.
        let payload = unsafe { T::parts(self.0).1 };
        core::mem::forget(self);
        payload
    }

    // Discard the value without deciding ownership here — for a site whose imp
    // pointer is already stored elsewhere (a context field) and whose caller
    // commits it later. Explicit on purpose: the call marks the handoff.
    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) fn forget(self) {
        core::mem::forget(self);
    }
}

impl<T: ImpHeader> Drop for FinishedImp<T> {
    fn drop(&mut self) {
        // NEVER releases — see the type docs. Debug-only tripwire.
        debug_assert!(
            false,
            "FinishedImp dropped: a finished imp's ownership was never explicitly decided"
        );
    }
}

// The shared tail of every `ufbxi_*_imp` creation site: stamp the refcount
// header, the magic and the payload into the freshly pushed imp, then hand the
// result arena (`ator` + `buf`) over to the refcount that owns it from here on.
//
// # Safety
//
// - `imp` addresses a live, writable `T` — an unread push from the result buf
//   qualifies; the `T` must be the LAST allocation of `buf`'s arena (C's
//   "must be the final allocation" rule), and none of its fields may be read
//   before this call.
// - `parent` is null or a live `Refcount` that outlives the new imp; it is
//   retained here (C: `ufbxi_init_ref`).
// - `payload` addresses a readable `T::Payload` in an allocation distinct from
//   `imp` (the context's own payload slot). Its value is MOVED bitwise into the
//   imp, so the caller must not use the source value afterwards.
//
// Returns the `FinishedImp` for the finished imp — the caller must thread it to
// wherever ownership is decided (see that type's docs).
#[inline(always)]
pub(crate) unsafe fn finish_imp<T: ImpHeader>(
    imp: *mut T,
    parent: *mut Refcount,
    payload: *mut T::Payload,
    ator: Allocator,
    buf: Buf,
) -> FinishedImp<T> {
    // Expose the wide allocation so `get_imp` can recover this header from a
    // (possibly narrowed) public payload pointer via exposed provenance.
    (imp as *mut u8).expose_provenance();

    // SAFETY: `imp` addresses a live `T` by this fn's contract.
    let (refcount, imp_payload, magic) = unsafe { T::parts(imp) };
    // SAFETY: `refcount` is the imp's own header field, writable by this fn's
    // contract; `parent` is null or live per the same contract.
    unsafe { crate::native::api::init_ref(refcount, T::MAGIC, parent) };
    // SAFETY: the imp's own `magic` field, writable by this fn's contract.
    unsafe { *magic = T::MAGIC };
    // SAFETY: the contract vouches that `payload` is a readable `T::Payload` in
    // a distinct allocation from `imp`, so the copy is non-overlapping and the
    // destination is the imp's own payload field.
    unsafe { core::ptr::copy_nonoverlapping(payload, imp_payload, 1) };
    // SAFETY: the header's own `ator`/`buf` fields, initialized just above; the
    // moved-in `Buf` replaces the push's uninitialized bytes (`Buf` has no
    // `Drop`, so no stale value is dropped).
    unsafe {
        (*refcount).ator = ator;
        (*refcount).buf = buf;
    }

    FinishedImp(imp)
}

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
        view_read!(self, str_data)
    }
    #[inline(always)]
    pub(crate) fn str_cap(&self) -> usize {
        view_read!(self, str_cap)
    }
    #[inline(always)]
    pub(crate) fn str_len(&self) -> usize {
        view_read!(self, str_len)
    }
    #[inline(always)]
    pub(crate) fn set_str_len(&self, str_len: usize) {
        view_write!(self, str_len, str_len);
    }
    #[inline(always)]
    pub(crate) fn str_data_mut_ptr(&self) -> *mut *mut u8 {
        view_raw_mut!(self, str_data)
    }
    #[inline(always)]
    pub(crate) fn str_cap_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, str_cap)
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> u8 {
        view_read!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn set_type_(&self, type_: u8) {
        view_write!(self, type_, type_);
    }
    #[inline(always)]
    pub(crate) fn set_negative(&self, negative: bool) {
        view_write!(self, negative, negative);
    }
    // `value.name_len` / `value.i64_` / `value.f64_` — writes of ONE member of
    // the `value` overlay. Two-level paths, so they carry their own safety
    // argument instead of the single-leaf macros.
    #[inline(always)]
    pub(crate) fn set_value_name_len(&self, name_len: usize) {
        // SAFETY: `get()` yields a pointer to this view's live, unmoved
        // `AsciiToken` (mint invariant; write-capable in `Mut` mode); the
        // projection writes its own `value.name_len` member as a raw place,
        // forming no reference to the containing struct.
        unsafe {
            (*self.get()).value.name_len = name_len;
        }
    }
    #[inline(always)]
    pub(crate) fn set_value_i64(&self, i64_: i64) {
        // SAFETY: as `set_value_name_len` — a raw-place write of this view's
        // own `value.i64_` member.
        unsafe {
            (*self.get()).value.i64_ = i64_;
        }
    }
    #[inline(always)]
    pub(crate) fn set_value_f64(&self, f64_: f64) {
        // SAFETY: as `set_value_name_len` — a raw-place write of this view's
        // own `value.f64_` member.
        unsafe {
            (*self.get()).value.f64_ = f64_;
        }
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
// place. `token`/`prev_token` recurse into `AsciiTokenView`; scalars are getters/setters.
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
    pub(crate) fn src_buf(&self) -> *mut Buf {
        view_read!(self, src_buf)
    }
    #[inline(always)]
    pub(crate) fn set_src_buf(&self, src_buf: *mut Buf) {
        view_write!(self, src_buf, src_buf)
    }
    #[inline(always)]
    pub(crate) fn retain_buf(&self) -> *mut Buf {
        view_read!(self, retain_buf)
    }
    #[inline(always)]
    pub(crate) fn src(&self) -> *const u8 {
        view_read!(self, src)
    }
    #[inline(always)]
    pub(crate) fn src_yield(&self) -> *const u8 {
        view_read!(self, src_yield)
    }
    #[inline(always)]
    pub(crate) fn src_end(&self) -> *const u8 {
        view_read!(self, src_end)
    }
    #[inline(always)]
    pub(crate) fn read_first_comment(&self) -> bool {
        view_read!(self, read_first_comment)
    }
    #[inline(always)]
    pub(crate) fn set_read_first_comment(&self, read_first_comment: bool) {
        view_write!(self, read_first_comment, read_first_comment)
    }
    #[inline(always)]
    pub(crate) fn set_src(&self, src: *const u8) {
        view_write!(self, src, src)
    }
    #[inline(always)]
    pub(crate) fn set_src_yield(&self, src_yield: *const u8) {
        view_write!(self, src_yield, src_yield)
    }
    #[inline(always)]
    pub(crate) fn set_src_end(&self, src_end: *const u8) {
        view_write!(self, src_end, src_end)
    }
    #[inline(always)]
    pub(crate) fn set_src_is_retained(&self, src_is_retained: bool) {
        view_write!(self, src_is_retained, src_is_retained)
    }
    #[inline(always)]
    pub(crate) fn set_found_version(&self, found_version: bool) {
        view_write!(self, found_version, found_version)
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

impl View<TmpMeshTexture> {
    #[inline(always)]
    pub(crate) fn set_prop_name(&self, value: String) {
        view_write!(self, prop_name, value)
    }
    #[inline(always)]
    pub(crate) fn set_face_texture(&self, value: *mut u32) {
        view_write!(self, face_texture, value)
    }
    #[inline(always)]
    pub(crate) fn set_num_faces(&self, value: usize) {
        view_write!(self, num_faces, value)
    }
    #[inline(always)]
    pub(crate) fn set_all_same(&self, value: bool) {
        view_write!(self, all_same, value)
    }
}

// ufbx.c:6341-6344 `ufbxi_mesh_extra`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct MeshExtra {
    pub texture_arr: *mut TmpMeshTexture,
    pub texture_count: usize,
}

impl<M: Mode> View<MeshExtra, M> {
    #[inline(always)]
    pub(crate) fn texture_arr(&self) -> *mut TmpMeshTexture {
        view_read_shared!(self, texture_arr)
    }
}

impl View<MeshExtra> {
    #[inline(always)]
    pub(crate) fn set_texture_arr(&self, value: *mut TmpMeshTexture) {
        view_write!(self, texture_arr, value)
    }
    #[inline(always)]
    pub(crate) fn set_texture_count(&self, value: usize) {
        view_write!(self, texture_count, value)
    }
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
        view_read!(self, group)
    }
    #[inline(always)]
    pub(crate) fn set_group(&self, group: String) {
        view_write!(self, group, group)
    }

    #[inline(always)]
    pub(crate) fn set_object(&self, object: String) {
        view_write!(self, object, object)
    }

    #[inline(always)]
    pub(crate) fn mtllib_relative_path(&self) -> crate::prelude::Blob {
        view_read!(self, mtllib_relative_path)
    }
    #[inline(always)]
    pub(crate) fn mtllib_relative_path_mut_ptr(&self) -> *mut crate::prelude::Blob {
        view_raw_mut!(self, mtllib_relative_path)
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
    pub(crate) fn group_map_view(&self) -> &crate::native::hash::MapView {
        unsafe { &*(&raw mut (*self.get()).group_map as *mut crate::native::hash::MapView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_color_valid_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_color_valid as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_faces_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_faces as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_props_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_props as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_meshes_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_meshes as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_face_smoothing_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_face_smoothing as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_face_group_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_face_group as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_face_group_infos_view(&self) -> &crate::native::buf::BufView {
        unsafe {
            &*(&raw mut (*self.get()).tmp_face_group_infos as *mut crate::native::buf::BufView)
        }
    }

    #[inline(always)]
    pub(crate) fn tmp_face_material_view(&self) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_face_material as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tokens_mut_ptr(&self) -> *mut *mut String {
        view_raw_mut!(self, tokens)
    }

    #[inline(always)]
    pub(crate) fn tokens_cap_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, tokens_cap)
    }

    #[inline(always)]
    pub(crate) fn tmp_materials_mut_ptr(&self) -> *mut *mut *mut crate::generated::Material {
        view_raw_mut!(self, tmp_materials)
    }

    #[inline(always)]
    pub(crate) fn tmp_materials_cap_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, tmp_materials_cap)
    }

    #[inline(always)]
    pub(crate) fn usemtl_index(&self) -> u32 {
        view_read!(self, usemtl_index)
    }

    #[inline(always)]
    pub(crate) fn set_usemtl_index(&self, usemtl_index: u32) {
        view_write!(self, usemtl_index, usemtl_index)
    }

    #[inline(always)]
    pub(crate) fn usemtl_fbx_id(&self) -> u64 {
        view_read!(self, usemtl_fbx_id)
    }

    #[inline(always)]
    pub(crate) fn set_usemtl_fbx_id(&self, usemtl_fbx_id: u64) {
        view_write!(self, usemtl_fbx_id, usemtl_fbx_id)
    }

    #[inline(always)]
    pub(crate) fn tokens_cap(&self) -> usize {
        view_read!(self, tokens_cap)
    }

    #[inline(always)]
    pub(crate) fn tokens(&self) -> *mut String {
        view_read!(self, tokens)
    }

    #[inline(always)]
    pub(crate) fn tmp_materials_cap(&self) -> usize {
        view_read!(self, tmp_materials_cap)
    }

    #[inline(always)]
    pub(crate) fn tmp_materials(&self) -> *mut *mut crate::generated::Material {
        view_read!(self, tmp_materials)
    }

    #[inline(always)]
    pub(crate) fn eof(&self) -> bool {
        view_read!(self, eof)
    }

    #[inline(always)]
    pub(crate) fn set_eof(&self, eof: bool) {
        view_write!(self, eof, eof)
    }

    #[inline(always)]
    pub(crate) fn read_progress(&self) -> usize {
        view_read!(self, read_progress)
    }

    #[inline(always)]
    pub(crate) fn set_read_progress(&self, read_progress: usize) {
        view_write!(self, read_progress, read_progress)
    }

    #[inline(always)]
    pub(crate) fn object_dirty(&self) -> bool {
        view_read!(self, object_dirty)
    }

    #[inline(always)]
    pub(crate) fn set_object_dirty(&self, object_dirty: bool) {
        view_write!(self, object_dirty, object_dirty)
    }

    #[inline(always)]
    pub(crate) fn num_tokens(&self) -> usize {
        view_read!(self, num_tokens)
    }

    #[inline(always)]
    pub(crate) fn set_num_tokens(&self, num_tokens: usize) {
        view_write!(self, num_tokens, num_tokens)
    }

    #[inline(always)]
    pub(crate) fn mrgb_vertex_count(&self) -> usize {
        view_read!(self, mrgb_vertex_count)
    }

    #[inline(always)]
    pub(crate) fn mesh(&self) -> *mut ObjMesh {
        view_read!(self, mesh)
    }

    #[inline(always)]
    pub(crate) fn set_mesh(&self, mesh: *mut ObjMesh) {
        view_write!(self, mesh, mesh)
    }

    #[inline(always)]
    pub(crate) fn material_dirty(&self) -> bool {
        view_read!(self, material_dirty)
    }

    #[inline(always)]
    pub(crate) fn set_material_dirty(&self, material_dirty: bool) {
        view_write!(self, material_dirty, material_dirty)
    }

    #[inline(always)]
    pub(crate) fn initialized(&self) -> bool {
        view_read!(self, initialized)
    }

    #[inline(always)]
    pub(crate) fn set_initialized(&self, initialized: bool) {
        view_write!(self, initialized, initialized)
    }

    #[inline(always)]
    pub(crate) fn has_vertex_color(&self) -> bool {
        view_read!(self, has_vertex_color)
    }

    #[inline(always)]
    pub(crate) fn set_has_vertex_color(&self, has_vertex_color: bool) {
        view_write!(self, has_vertex_color, has_vertex_color)
    }

    #[inline(always)]
    pub(crate) fn has_face_smoothing(&self) -> bool {
        view_read!(self, has_face_smoothing)
    }

    #[inline(always)]
    pub(crate) fn set_has_face_smoothing(&self, has_face_smoothing: bool) {
        view_write!(self, has_face_smoothing, has_face_smoothing)
    }

    #[inline(always)]
    pub(crate) fn has_face_group(&self) -> bool {
        view_read!(self, has_face_group)
    }

    #[inline(always)]
    pub(crate) fn set_has_face_group(&self, has_face_group: bool) {
        view_write!(self, has_face_group, has_face_group)
    }

    #[inline(always)]
    pub(crate) fn group_dirty(&self) -> bool {
        view_read!(self, group_dirty)
    }

    #[inline(always)]
    pub(crate) fn set_group_dirty(&self, group_dirty: bool) {
        view_write!(self, group_dirty, group_dirty)
    }

    #[inline(always)]
    pub(crate) fn face_smoothing(&self) -> bool {
        view_read!(self, face_smoothing)
    }

    #[inline(always)]
    pub(crate) fn set_face_smoothing(&self, face_smoothing: bool) {
        view_write!(self, face_smoothing, face_smoothing)
    }

    #[inline(always)]
    pub(crate) fn face_material(&self) -> u32 {
        view_read!(self, face_material)
    }

    #[inline(always)]
    pub(crate) fn set_face_material(&self, face_material: u32) {
        view_write!(self, face_material, face_material)
    }

    #[inline(always)]
    pub(crate) fn face_group_dirty(&self) -> bool {
        view_read!(self, face_group_dirty)
    }

    #[inline(always)]
    pub(crate) fn set_face_group_dirty(&self, face_group_dirty: bool) {
        view_write!(self, face_group_dirty, face_group_dirty)
    }

    #[inline(always)]
    pub(crate) fn face_group(&self) -> u32 {
        view_read!(self, face_group)
    }

    #[inline(always)]
    pub(crate) fn set_face_group(&self, face_group: u32) {
        view_write!(self, face_group, face_group)
    }

    #[inline(always)]
    pub(crate) fn tmp_vertices_at(&self, i: usize) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_vertices[i] as *mut crate::native::buf::BufView) }
    }

    #[inline(always)]
    pub(crate) fn tmp_indices_at(&self, i: usize) -> &crate::native::buf::BufView {
        unsafe { &*(&raw mut (*self.get()).tmp_indices[i] as *mut crate::native::buf::BufView) }
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
        view_write!(self, indices, indices)
    }
    #[inline(always)]
    pub(crate) fn num_left(&self) -> usize {
        view_read!(self, num_left)
    }
    #[inline(always)]
    pub(crate) fn set_num_left(&self, num_left: usize) {
        view_write!(self, num_left, num_left)
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
        view_read!(self, root_transform)
    }

    #[inline(always)]
    pub(crate) fn open_file_cb_view(&self) -> &crate::prelude::RawOpenFileCbView {
        unsafe { &*(&raw mut (*self.get()).open_file_cb as *mut crate::prelude::RawOpenFileCbView) }
    }

    #[inline(always)]
    pub(crate) fn allow_empty_faces(&self) -> bool {
        view_read!(self, allow_empty_faces)
    }

    #[inline(always)]
    pub(crate) fn allow_missing_vertex_position(&self) -> bool {
        view_read!(self, allow_missing_vertex_position)
    }

    #[inline(always)]
    pub(crate) fn allow_nodes_out_of_root(&self) -> bool {
        view_read!(self, allow_nodes_out_of_root)
    }

    #[inline(always)]
    pub(crate) fn allow_unsafe(&self) -> bool {
        view_read!(self, allow_unsafe)
    }

    #[inline(always)]
    pub(crate) fn clean_skin_weights(&self) -> bool {
        view_read!(self, clean_skin_weights)
    }

    #[inline(always)]
    pub(crate) fn connect_broken_elements(&self) -> bool {
        view_read!(self, connect_broken_elements)
    }

    #[inline(always)]
    pub(crate) fn disable_quirks(&self) -> bool {
        view_read!(self, disable_quirks)
    }

    #[inline(always)]
    pub(crate) fn evaluate_caches(&self) -> bool {
        view_read!(self, evaluate_caches)
    }

    #[inline(always)]
    pub(crate) fn evaluate_skinning(&self) -> bool {
        view_read!(self, evaluate_skinning)
    }

    #[inline(always)]
    pub(crate) fn file_format(&self) -> crate::generated::FileFormat {
        view_read!(self, file_format)
    }

    #[inline(always)]
    pub(crate) fn file_format_lookahead(&self) -> usize {
        view_read!(self, file_format_lookahead)
    }
    #[inline(always)]
    pub(crate) fn set_file_format_lookahead(&self, file_format_lookahead: usize) {
        view_write!(self, file_format_lookahead, file_format_lookahead)
    }

    #[inline(always)]
    pub(crate) fn file_size_estimate(&self) -> u64 {
        view_read!(self, file_size_estimate)
    }

    #[inline(always)]
    pub(crate) fn filename_view(&self) -> &crate::prelude::RawStringView {
        unsafe { &*(&raw mut (*self.get()).filename as *mut crate::prelude::RawStringView) }
    }

    #[inline(always)]
    pub(crate) fn force_single_thread_ascii_parsing(&self) -> bool {
        view_read!(self, force_single_thread_ascii_parsing)
    }

    #[inline(always)]
    pub(crate) fn generate_missing_normals(&self) -> bool {
        view_read!(self, generate_missing_normals)
    }

    #[inline(always)]
    pub(crate) fn geometry_transform_handling(
        &self,
    ) -> crate::generated::GeometryTransformHandling {
        view_read!(self, geometry_transform_handling)
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
        view_read!(self, handedness_conversion_axis)
    }

    #[inline(always)]
    pub(crate) fn handedness_conversion_retain_winding(&self) -> bool {
        view_read!(self, handedness_conversion_retain_winding)
    }

    #[inline(always)]
    pub(crate) fn ignore_all_content(&self) -> bool {
        view_read!(self, ignore_all_content)
    }

    #[inline(always)]
    pub(crate) fn ignore_animation(&self) -> bool {
        view_read!(self, ignore_animation)
    }
    #[inline(always)]
    pub(crate) fn set_ignore_animation(&self, ignore_animation: bool) {
        view_write!(self, ignore_animation, ignore_animation)
    }

    #[inline(always)]
    pub(crate) fn ignore_embedded(&self) -> bool {
        view_read!(self, ignore_embedded)
    }
    #[inline(always)]
    pub(crate) fn set_ignore_embedded(&self, ignore_embedded: bool) {
        view_write!(self, ignore_embedded, ignore_embedded)
    }

    #[inline(always)]
    pub(crate) fn ignore_geometry(&self) -> bool {
        view_read!(self, ignore_geometry)
    }
    #[inline(always)]
    pub(crate) fn set_ignore_geometry(&self, ignore_geometry: bool) {
        view_write!(self, ignore_geometry, ignore_geometry)
    }

    #[inline(always)]
    pub(crate) fn ignore_missing_external_files(&self) -> bool {
        view_read!(self, ignore_missing_external_files)
    }

    #[inline(always)]
    pub(crate) fn index_error_handling(&self) -> crate::generated::IndexErrorHandling {
        view_read!(self, index_error_handling)
    }

    #[inline(always)]
    pub(crate) fn inherit_mode_handling(&self) -> crate::generated::InheritModeHandling {
        view_read!(self, inherit_mode_handling)
    }

    #[inline(always)]
    pub(crate) fn key_clamp_threshold(&self) -> f64 {
        view_read!(self, key_clamp_threshold)
    }

    #[inline(always)]
    pub(crate) fn load_external_files(&self) -> bool {
        view_read!(self, load_external_files)
    }

    #[inline(always)]
    pub(crate) fn no_format_from_content(&self) -> bool {
        view_read!(self, no_format_from_content)
    }

    #[inline(always)]
    pub(crate) fn no_format_from_extension(&self) -> bool {
        view_read!(self, no_format_from_extension)
    }

    #[inline(always)]
    pub(crate) fn node_depth_limit(&self) -> u32 {
        view_read!(self, node_depth_limit)
    }

    #[inline(always)]
    pub(crate) fn normalize_normals(&self) -> bool {
        view_read!(self, normalize_normals)
    }

    #[inline(always)]
    pub(crate) fn normalize_tangents(&self) -> bool {
        view_read!(self, normalize_tangents)
    }

    #[inline(always)]
    pub(crate) fn obj_axes(&self) -> crate::generated::CoordinateAxes {
        view_read!(self, obj_axes)
    }

    #[inline(always)]
    pub(crate) fn obj_merge_groups(&self) -> bool {
        view_read!(self, obj_merge_groups)
    }
    #[inline(always)]
    pub(crate) fn set_obj_merge_groups(&self, obj_merge_groups: bool) {
        view_write!(self, obj_merge_groups, obj_merge_groups)
    }

    #[inline(always)]
    pub(crate) fn obj_merge_objects(&self) -> bool {
        view_read!(self, obj_merge_objects)
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
        view_read!(self, obj_search_mtl_by_filename)
    }

    #[inline(always)]
    pub(crate) fn obj_split_groups(&self) -> bool {
        view_read!(self, obj_split_groups)
    }

    #[inline(always)]
    pub(crate) fn obj_unit_meters(&self) -> crate::prelude::Real {
        view_read!(self, obj_unit_meters)
    }

    #[inline(always)]
    pub(crate) fn open_file_cb_ptr(&self) -> *const crate::generated::RawOpenFileCb {
        view_raw_const!(self, open_file_cb)
    }

    #[inline(always)]
    pub(crate) fn open_file_cb(&self) -> crate::generated::RawOpenFileCb {
        view_read!(self, open_file_cb)
    }

    #[inline(always)]
    pub(crate) fn open_main_file_with_default(&self) -> bool {
        view_read!(self, open_main_file_with_default)
    }

    #[inline(always)]
    pub(crate) fn path_separator(&self) -> u8 {
        view_read!(self, path_separator)
    }
    #[inline(always)]
    pub(crate) fn set_path_separator(&self, path_separator: u8) {
        view_write!(self, path_separator, path_separator)
    }

    #[inline(always)]
    pub(crate) fn pivot_handling(&self) -> crate::generated::PivotHandling {
        view_read!(self, pivot_handling)
    }

    #[inline(always)]
    pub(crate) fn pivot_handling_retain_empties(&self) -> bool {
        view_read!(self, pivot_handling_retain_empties)
    }

    #[inline(always)]
    pub(crate) fn progress_cb(&self) -> crate::generated::RawProgressCb {
        view_read!(self, progress_cb)
    }

    #[inline(always)]
    pub(crate) fn progress_interval_hint(&self) -> u64 {
        view_read!(self, progress_interval_hint)
    }

    #[inline(always)]
    pub(crate) fn raw_filename_view(&self) -> &crate::prelude::RawBlobView {
        unsafe { &*(&raw mut (*self.get()).raw_filename as *mut crate::prelude::RawBlobView) }
    }

    #[inline(always)]
    pub(crate) fn read_buffer_size(&self) -> usize {
        view_read!(self, read_buffer_size)
    }
    #[inline(always)]
    pub(crate) fn set_read_buffer_size(&self, read_buffer_size: usize) {
        view_write!(self, read_buffer_size, read_buffer_size)
    }

    #[inline(always)]
    pub(crate) fn retain_dom(&self) -> bool {
        view_read!(self, retain_dom)
    }

    #[inline(always)]
    pub(crate) fn retain_vertex_attrib_w(&self) -> bool {
        view_read!(self, retain_vertex_attrib_w)
    }

    #[inline(always)]
    pub(crate) fn reverse_winding(&self) -> bool {
        view_read!(self, reverse_winding)
    }

    #[inline(always)]
    pub(crate) fn root_transform_ptr(&self) -> *const crate::generated::Transform {
        view_raw_const!(self, root_transform)
    }

    #[inline(always)]
    pub(crate) fn scale_helper_name_view(&self) -> &crate::prelude::RawStringView {
        unsafe {
            &*(&raw mut (*self.get()).scale_helper_name as *mut crate::prelude::RawStringView)
        }
    }

    #[inline(always)]
    pub(crate) fn skip_mesh_parts(&self) -> bool {
        view_read!(self, skip_mesh_parts)
    }

    #[inline(always)]
    pub(crate) fn skip_skin_vertices(&self) -> bool {
        view_read!(self, skip_skin_vertices)
    }

    #[inline(always)]
    pub(crate) fn space_conversion(&self) -> crate::generated::SpaceConversion {
        view_read!(self, space_conversion)
    }

    #[inline(always)]
    pub(crate) fn strict(&self) -> bool {
        view_read!(self, strict)
    }

    #[inline(always)]
    pub(crate) fn target_axes(&self) -> crate::generated::CoordinateAxes {
        view_read!(self, target_axes)
    }

    #[inline(always)]
    pub(crate) fn target_camera_axes(&self) -> crate::generated::CoordinateAxes {
        view_read!(self, target_camera_axes)
    }

    #[inline(always)]
    pub(crate) fn target_light_axes(&self) -> crate::generated::CoordinateAxes {
        view_read!(self, target_light_axes)
    }

    #[inline(always)]
    pub(crate) fn target_unit_meters(&self) -> crate::prelude::Real {
        view_read!(self, target_unit_meters)
    }

    #[inline(always)]
    pub(crate) fn thread_opts_view(&self) -> &crate::prelude::RawThreadOptsView {
        unsafe { &*(&raw mut (*self.get()).thread_opts as *mut crate::prelude::RawThreadOptsView) }
    }

    #[inline(always)]
    pub(crate) fn thread_opts_ptr(&self) -> *const crate::generated::RawThreadOpts {
        view_raw_const!(self, thread_opts)
    }

    #[inline(always)]
    pub(crate) fn unicode_error_handling(&self) -> crate::generated::UnicodeErrorHandling {
        view_read!(self, unicode_error_handling)
    }

    #[inline(always)]
    pub(crate) fn use_blender_pbr_material(&self) -> bool {
        view_read!(self, use_blender_pbr_material)
    }

    #[inline(always)]
    pub(crate) fn use_root_transform(&self) -> bool {
        view_read!(self, use_root_transform)
    }
}

// Mode-generic nested views over the two allocator descriptors: `init_ator`
// only reads them, so the accessor serves a `Mut` context field and a `Const`
// boundary mint alike.
impl<M: crate::native::view::Mode> View<RawLoadOpts, M> {
    #[inline(always)]
    pub(crate) fn temp_allocator_view(&self) -> &View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, temp_allocator)
    }
    #[inline(always)]
    pub(crate) fn result_allocator_view(&self) -> &View<crate::generated::RawAllocatorOpts, M> {
        view_project!(self, result_allocator)
    }
}

// Typed interior-mutable VIEW over a `Scene` field (the public `ufbx_scene`),
// reinterpreted in place. The public `Scene` type is untouched; this is a
// pub(crate) internal handle. Reachable from any context that owns a `Scene`
// field (`Context.scene`, `EvalContext.scene`/`src_scene`). Sub-structs recurse
// into their own *View; List/RefList fields use ListView/RefListView; Copy
// scalars/Refs use value getters/setters or _ptr/_mut_ptr for addr-of sites.
pub(crate) type SceneView = crate::native::view::View<crate::generated::Scene>;

// Mode-generic `Scene` read accessors: served to both `Mut` (context/arena
// provenance) and `Const` (a public caller's `&Scene`) roots, so the public
// find-by-name surface can navigate a read-only scene (`native/api.rs`
// `find_element_len`) through the same accessor the loader uses.
impl<M: crate::native::view::Mode> crate::native::view::View<crate::generated::Scene, M> {
    #[inline(always)]
    pub(crate) fn elements_by_name_view(
        &self,
    ) -> &crate::native::view::View<crate::prelude::List<crate::generated::NameElement>, M> {
        view_project!(self, elements_by_name)
    }
}

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
        view_raw_mut!(self, settings)
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
        view_raw_mut!(self, unknowns)
    }

    #[inline(always)]
    pub(crate) fn root_node(&self) -> crate::prelude::Ref<crate::generated::Node> {
        view_read!(self, root_node)
    }
    #[inline(always)]
    pub(crate) fn set_root_node(&self, root_node: crate::prelude::Ref<crate::generated::Node>) {
        view_write!(self, root_node, root_node)
    }
    #[inline(always)]
    pub(crate) fn root_node_mut_ptr(&self) -> *mut crate::prelude::Ref<crate::generated::Node> {
        view_raw_mut!(self, root_node)
    }
    #[inline(always)]
    pub(crate) fn anim_ptr(&self) -> *const crate::prelude::Ref<crate::generated::Anim> {
        view_raw_const!(self, anim)
    }
    #[inline(always)]
    pub(crate) fn anim_mut_ptr(&self) -> *mut crate::prelude::Ref<crate::generated::Anim> {
        view_raw_mut!(self, anim)
    }
    #[inline(always)]
    pub(crate) fn dom_root(&self) -> Option<crate::prelude::Ref<crate::generated::DomNode>> {
        view_read!(self, dom_root)
    }
    #[inline(always)]
    pub(crate) fn set_dom_root(
        &self,
        dom_root: Option<crate::prelude::Ref<crate::generated::DomNode>>,
    ) {
        view_write!(self, dom_root, dom_root)
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
        view_read!(self, file_format)
    }
    #[inline(always)]
    pub(crate) fn set_file_format(&self, file_format: crate::generated::FileFormat) {
        view_write!(self, file_format, file_format)
    }
    #[inline(always)]
    pub(crate) fn geometry_scale(&self) -> crate::prelude::Real {
        view_read!(self, geometry_scale)
    }
    #[inline(always)]
    pub(crate) fn set_bone_prop_size_unit(&self, bone_prop_size_unit: crate::prelude::Real) {
        view_write!(self, bone_prop_size_unit, bone_prop_size_unit)
    }
    #[inline(always)]
    pub(crate) fn set_bone_prop_limb_length_relative(&self, bone_prop_limb_length_relative: bool) {
        view_write!(
            self,
            bone_prop_limb_length_relative,
            bone_prop_limb_length_relative
        )
    }
    #[inline(always)]
    pub(crate) fn set_mirror_axis(&self, mirror_axis: crate::generated::MirrorAxis) {
        view_write!(self, mirror_axis, mirror_axis)
    }
    #[inline(always)]
    pub(crate) fn set_is_unsafe(&self, is_unsafe: bool) {
        view_write!(self, is_unsafe, is_unsafe)
    }
    #[inline(always)]
    pub(crate) fn set_may_contain_no_index(&self, may_contain_no_index: bool) {
        view_write!(self, may_contain_no_index, may_contain_no_index)
    }
    #[inline(always)]
    pub(crate) fn set_may_contain_missing_vertex_position(
        &self,
        may_contain_missing_vertex_position: bool,
    ) {
        view_write!(
            self,
            may_contain_missing_vertex_position,
            may_contain_missing_vertex_position
        )
    }
    #[inline(always)]
    pub(crate) fn set_may_contain_broken_elements(&self, may_contain_broken_elements: bool) {
        view_write!(
            self,
            may_contain_broken_elements,
            may_contain_broken_elements
        )
    }
    #[inline(always)]
    pub(crate) fn set_version(&self, version: u32) {
        view_write!(self, version, version)
    }
    #[inline(always)]
    pub(crate) fn set_ascii(&self, ascii: bool) {
        view_write!(self, ascii, ascii)
    }
    #[inline(always)]
    pub(crate) fn set_big_endian(&self, big_endian: bool) {
        view_write!(self, big_endian, big_endian)
    }
    #[inline(always)]
    pub(crate) fn set_geometry_ignored(&self, geometry_ignored: bool) {
        view_write!(self, geometry_ignored, geometry_ignored)
    }
    #[inline(always)]
    pub(crate) fn set_animation_ignored(&self, animation_ignored: bool) {
        view_write!(self, animation_ignored, animation_ignored)
    }
    #[inline(always)]
    pub(crate) fn set_embedded_ignored(&self, embedded_ignored: bool) {
        view_write!(self, embedded_ignored, embedded_ignored)
    }
    #[inline(always)]
    pub(crate) fn set_exporter(&self, exporter: crate::generated::Exporter) {
        view_write!(self, exporter, exporter)
    }
    #[inline(always)]
    pub(crate) fn set_exporter_version(&self, exporter_version: u32) {
        view_write!(self, exporter_version, exporter_version)
    }
    #[inline(always)]
    pub(crate) fn num_shader_textures(&self) -> usize {
        view_read!(self, num_shader_textures)
    }
    #[inline(always)]
    pub(crate) fn set_num_shader_textures(&self, num_shader_textures: usize) {
        view_write!(self, num_shader_textures, num_shader_textures)
    }
    #[inline(always)]
    pub(crate) fn set_ortho_size_unit(&self, ortho_size_unit: crate::prelude::Real) {
        view_write!(self, ortho_size_unit, ortho_size_unit)
    }
    #[inline(always)]
    pub(crate) fn element_buffer_size(&self) -> usize {
        view_read!(self, element_buffer_size)
    }
    #[inline(always)]
    pub(crate) fn set_element_buffer_size(&self, element_buffer_size: usize) {
        view_write!(self, element_buffer_size, element_buffer_size)
    }
    #[inline(always)]
    pub(crate) fn max_face_triangles(&self) -> usize {
        view_read!(self, max_face_triangles)
    }
    #[inline(always)]
    pub(crate) fn set_max_face_triangles(&self, max_face_triangles: usize) {
        view_write!(self, max_face_triangles, max_face_triangles)
    }
    #[inline(always)]
    pub(crate) fn set_ktime_second(&self, ktime_second: i64) {
        view_write!(self, ktime_second, ktime_second)
    }

    // --- String leaves: whole value getter/setter + StringView sub-view + _mut_ptr ---
    #[inline(always)]
    pub(crate) fn filename(&self) -> crate::prelude::String {
        view_read!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn set_filename(&self, filename: crate::prelude::String) {
        view_write!(self, filename, filename)
    }
    #[inline(always)]
    pub(crate) fn filename_view(&self) -> &crate::prelude::StringView {
        unsafe { &*(&raw mut (*self.get()).filename as *mut crate::prelude::StringView) }
    }
    #[inline(always)]
    pub(crate) fn creator_view(&self) -> &crate::prelude::StringView {
        unsafe { &*(&raw mut (*self.get()).creator as *mut crate::prelude::StringView) }
    }
    #[inline(always)]
    pub(crate) fn creator_mut_ptr(&self) -> *mut crate::prelude::String {
        view_raw_mut!(self, creator)
    }
    #[inline(always)]
    pub(crate) fn relative_root_view(&self) -> &crate::prelude::StringView {
        unsafe { &*(&raw mut (*self.get()).relative_root as *mut crate::prelude::StringView) }
    }
    #[inline(always)]
    pub(crate) fn set_original_file_path(&self, original_file_path: crate::prelude::String) {
        view_write!(self, original_file_path, original_file_path)
    }
    #[inline(always)]
    pub(crate) fn original_file_path_ptr(&self) -> *const crate::prelude::String {
        view_raw_const!(self, original_file_path)
    }

    // --- Blob leaves: whole setter + BlobView sub-view + _mut_ptr / _ptr ---
    #[inline(always)]
    pub(crate) fn set_raw_filename(&self, raw_filename: crate::prelude::Blob) {
        view_write!(self, raw_filename, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_filename_view(&self) -> &crate::prelude::BlobView {
        unsafe { &*(&raw mut (*self.get()).raw_filename as *mut crate::prelude::BlobView) }
    }
    #[inline(always)]
    pub(crate) fn raw_relative_root_view(&self) -> &crate::prelude::BlobView {
        unsafe { &*(&raw mut (*self.get()).raw_relative_root as *mut crate::prelude::BlobView) }
    }
    #[inline(always)]
    pub(crate) fn set_raw_original_file_path(&self, raw_original_file_path: crate::prelude::Blob) {
        view_write!(self, raw_original_file_path, raw_original_file_path)
    }
    #[inline(always)]
    pub(crate) fn raw_original_file_path_ptr(&self) -> *const crate::prelude::Blob {
        view_raw_const!(self, raw_original_file_path)
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
        view_raw_mut!(self, warnings)
    }
    #[inline(always)]
    pub(crate) fn has_warning_mut_ptr(&self) -> *mut bool {
        unsafe { (&raw mut (*self.get()).has_warning) as *mut bool }
    }

    // --- scene_props (Props): addr-of only / thumbnail (Thumbnail): sub-view ---
    #[inline(always)]
    pub(crate) fn scene_props_ptr(&self) -> *const crate::generated::Props {
        view_raw_const!(self, scene_props)
    }
    #[inline(always)]
    pub(crate) fn thumbnail_view(&self) -> &crate::native::read::ThumbnailView {
        view_project!(self, thumbnail)
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
        view_read!(self, axes)
    }
    #[inline(always)]
    pub(crate) fn unit_meters(&self) -> crate::prelude::Real {
        view_read!(self, unit_meters)
    }
    #[inline(always)]
    pub(crate) fn frames_per_second(&self) -> f64 {
        view_read!(self, frames_per_second)
    }
    #[inline(always)]
    pub(crate) fn props_mut_ptr(&self) -> *mut crate::generated::Props {
        view_raw_mut!(self, props)
    }
}

impl Context {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerContext {
        self.0.get().cast()
    }

    #[inline(always)]
    pub(crate) fn axis_matrix(&self) -> Matrix {
        view_read!(self, axis_matrix)
    }

    #[inline(always)]
    /// Moves the field out by bitwise read (`ptr::read`). C does this as plain
    /// struct assignment; the source field still holds the stale bits (no
    /// `Drop`), so the caller must overwrite it or treat it as moved-from.
    pub(crate) fn take_result(&self) -> crate::native::buf::Buf {
        unsafe { core::ptr::read(&raw const (*self.get()).result) }
    }

    #[inline(always)]
    pub(crate) fn set_result(&self, result: crate::native::buf::Buf) {
        view_write!(self, result, result)
    }

    #[inline(always)]
    pub(crate) fn ator_result(&self) -> crate::native::allocator::Allocator {
        view_read!(self, ator_result)
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
    pub(crate) fn tmp_thread_parse_at(&self, i: usize) -> &crate::native::buf::BufView {
        unsafe {
            &*(&raw mut (*self.get()).tmp_thread_parse[i] as *mut crate::native::buf::BufView)
        }
    }
    // `exporter`/`mirror_axis` (Copy enums) — value getter/setter.
    #[inline(always)]
    pub(crate) fn exporter(&self) -> Exporter {
        view_read!(self, exporter)
    }
    #[inline(always)]
    pub(crate) fn set_exporter(&self, exporter: Exporter) {
        view_write!(self, exporter, exporter)
    }
    #[inline(always)]
    pub(crate) fn mirror_axis(&self) -> MirrorAxis {
        view_read!(self, mirror_axis)
    }
    #[inline(always)]
    pub(crate) fn set_mirror_axis(&self, mirror_axis: MirrorAxis) {
        view_write!(self, mirror_axis, mirror_axis)
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
    #[inline(always)]
    pub(crate) fn prop_type_map_view(&self) -> &crate::native::hash::MapView {
        unsafe { &*(&raw mut (*self.get()).prop_type_map as *mut crate::native::hash::MapView) }
    }
    #[inline(always)]
    pub(crate) fn anim_stack_map_view(&self) -> &crate::native::hash::MapView {
        unsafe { &*(&raw mut (*self.get()).anim_stack_map as *mut crate::native::hash::MapView) }
    }
    #[inline(always)]
    pub(crate) fn ptr_fbx_id_map_view(&self) -> &crate::native::hash::MapView {
        unsafe { &*(&raw mut (*self.get()).ptr_fbx_id_map as *mut crate::native::hash::MapView) }
    }
    #[inline(always)]
    pub(crate) fn dom_node_map_view(&self) -> &crate::native::hash::MapView {
        unsafe { &*(&raw mut (*self.get()).dom_node_map as *mut crate::native::hash::MapView) }
    }
    #[inline(always)]
    pub(crate) fn fbx_id_map_view(&self) -> &crate::native::hash::MapView {
        unsafe { &*(&raw mut (*self.get()).fbx_id_map as *mut crate::native::hash::MapView) }
    }
    #[inline(always)]
    pub(crate) fn fbx_attr_map_view(&self) -> &crate::native::hash::MapView {
        unsafe { &*(&raw mut (*self.get()).fbx_attr_map as *mut crate::native::hash::MapView) }
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
        view_raw_const!(self, error)
    }

    // `swap_arr_size` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn swap_arr_size_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, swap_arr_size)
    }

    // `swap_arr` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn swap_arr_mut_ptr(&self) -> *mut *mut u8 {
        view_raw_mut!(self, swap_arr)
    }

    // `top_child` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn top_child_mut_ptr(&self) -> *mut Node {
        view_raw_mut!(self, top_child)
    }

    // `top_nodes_cap` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn top_nodes_cap_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, top_nodes_cap)
    }

    // `top_nodes` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn top_nodes_mut_ptr(&self) -> *mut *mut Node {
        view_raw_mut!(self, top_nodes)
    }

    // `dom_parse_toplevel` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn dom_parse_toplevel_mut_ptr(&self) -> *mut *mut DomNode {
        view_raw_mut!(self, dom_parse_toplevel)
    }

    // `element_extra_cap` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn element_extra_cap_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, element_extra_cap)
    }

    // `element_extra_arr` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn element_extra_arr_mut_ptr(&self) -> *mut *mut *mut c_void {
        view_raw_mut!(self, element_extra_arr)
    }

    // `max_consecutive_indices` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn max_consecutive_indices_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, max_consecutive_indices)
    }

    // `opts` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn opts_mut_ptr(&self) -> *mut RawLoadOpts {
        view_raw_mut!(self, opts)
    }

    // `warnings` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn warnings_mut_ptr(&self) -> *mut Warnings {
        view_raw_mut!(self, warnings)
    }
    // `warnings` (Warnings) — typed VIEW handle (reinterpret-in-place); accessors on WarningsView.
    #[inline(always)]
    pub(crate) fn warnings_view(&self) -> &crate::native::warnings::WarningsView {
        unsafe { &*(&raw mut (*self.get()).warnings as *mut crate::native::warnings::WarningsView) }
    }

    // `read_buffer_size` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn read_buffer_size_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, read_buffer_size)
    }

    // `read_buffer` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn read_buffer_mut_ptr(&self) -> *mut *mut u8 {
        view_raw_mut!(self, read_buffer)
    }

    // `root_id` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn root_id_mut_ptr(&self) -> *mut u64 {
        view_raw_mut!(self, root_id)
    }

    // `axis_matrix` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn axis_matrix_mut_ptr(&self) -> *mut Matrix {
        view_raw_mut!(self, axis_matrix)
    }

    // `legacy_node` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn legacy_node_mut_ptr(&self) -> *mut Node {
        view_raw_mut!(self, legacy_node)
    }

    // `legacy_node` (Node) — typed VIEW handle (reinterpret-in-place); accessors on NodeView.
    #[inline(always)]
    pub(crate) fn legacy_node_view(&self) -> &NodeView {
        // SAFETY: repr(transparent) over the `legacy_node` field inside the outer UnsafeCell;
        // shared interior-mutable view, asserts no validity.
        unsafe { &*(&raw mut (*self.get()).legacy_node as *mut NodeView) }
    }

    // `ator_result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_result_mut_ptr(&self) -> *mut Allocator {
        view_raw_mut!(self, ator_result)
    }

    // `ator_result` (Allocator) — typed VIEW handle (reinterpret-in-place); accessors on AllocatorView.
    #[inline(always)]
    pub(crate) fn ator_result_view(&self) -> &crate::native::allocator::AllocatorView {
        // SAFETY: reinterpret the owned Allocator field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).ator_result as *mut crate::native::allocator::AllocatorView)
        }
    }

    // `scene` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn scene_mut_ptr(&self) -> *mut Scene {
        view_raw_mut!(self, scene)
    }

    // `thread_pool` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn thread_pool_mut_ptr(&self) -> *mut ThreadPool {
        view_raw_mut!(self, thread_pool)
    }

    // `ascii` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ascii_mut_ptr(&self) -> *mut Ascii {
        view_raw_mut!(self, ascii)
    }

    // `tmp_arr_size` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_arr_size_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, tmp_arr_size)
    }
    // Value getter for the read-only size sites (the `&mut`/grow-array out-param sites
    // keep using `tmp_arr_size_mut_ptr`).
    #[inline(always)]
    pub(crate) fn tmp_arr_size(&self) -> usize {
        view_read!(self, tmp_arr_size)
    }

    // `tmp_arr` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn tmp_arr_mut_ptr(&self) -> *mut *mut u8 {
        view_raw_mut!(self, tmp_arr)
    }

    // `error` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn error_mut_ptr(&self) -> *mut Error {
        view_raw_mut!(self, error)
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
        view_raw_mut!(self, tmp)
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
    /// Moves the field out by bitwise read (`ptr::read`). C does this as plain
    /// struct assignment; the source field still holds the stale bits (no
    /// `Drop`), so the caller must overwrite it or treat it as moved-from.
    pub(crate) fn take_string_pool(&self) -> StringPool {
        unsafe { core::ptr::read(&raw const (*self.get()).string_pool) }
    }
    #[inline(always)]
    pub(crate) fn set_string_pool(&self, string_pool: StringPool) {
        view_write!(self, string_pool, string_pool)
    }

    // `result` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn result_mut_ptr(&self) -> *mut Buf {
        view_raw_mut!(self, result)
    }

    // Reborrow a raw `*mut InnerContext` as `&Context` (layout-identical via
    // `repr(transparent)`). For the nullable-context (`maybe_uc`) call paths.
    // SAFETY: `ptr` must be non-null and point to a live context allocation.
    #[inline(always)]
    pub(crate) unsafe fn from_ptr<'a>(ptr: *mut InnerContext) -> &'a Context {
        // SAFETY: `ptr` is non-null and points to a live context allocation (fn
        // contract); `Context` is `repr(transparent)` over
        // `UnsafeCell<MaybeUninit<InnerContext>>`, which is layout-identical to
        // `InnerContext`, so the cast-and-deref reinterprets that live allocation
        // in place.
        unsafe { &*(ptr as *const Context) }
    }

    // `base64_table` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn base64_table(&self) -> *mut u8 {
        view_read!(self, base64_table)
    }

    #[inline(always)]
    pub(crate) fn set_base64_table(&self, base64_table: *mut u8) {
        view_write!(self, base64_table, base64_table)
    }

    // `parse_threaded` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn parse_threaded(&self) -> bool {
        view_read!(self, parse_threaded)
    }

    #[inline(always)]
    pub(crate) fn set_parse_threaded(&self, parse_threaded: bool) {
        view_write!(self, parse_threaded, parse_threaded)
    }

    // `load_filename_len` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn load_filename_len(&self) -> usize {
        view_read!(self, load_filename_len)
    }

    #[inline(always)]
    pub(crate) fn set_load_filename_len(&self, load_filename_len: usize) {
        view_write!(self, load_filename_len, load_filename_len)
    }

    // `load_filename` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn load_filename(&self) -> *const u8 {
        view_read!(self, load_filename)
    }

    #[inline(always)]
    pub(crate) fn set_load_filename(&self, load_filename: *const u8) {
        view_write!(self, load_filename, load_filename)
    }

    // `deferred_load` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn deferred_load(&self) -> bool {
        view_read!(self, deferred_load)
    }

    #[inline(always)]
    pub(crate) fn set_deferred_load(&self, deferred_load: bool) {
        view_write!(self, deferred_load, deferred_load)
    }

    // `deferred_failure` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn deferred_failure(&self) -> bool {
        view_read!(self, deferred_failure)
    }

    // `unit_scale` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn unit_scale(&self) -> Real {
        view_read!(self, unit_scale)
    }

    #[inline(always)]
    pub(crate) fn set_unit_scale(&self, unit_scale: Real) {
        view_write!(self, unit_scale, unit_scale)
    }

    // `ktime_sec_double` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn ktime_sec_double(&self) -> f64 {
        view_read!(self, ktime_sec_double)
    }

    #[inline(always)]
    pub(crate) fn set_ktime_sec_double(&self, ktime_sec_double: f64) {
        view_write!(self, ktime_sec_double, ktime_sec_double)
    }

    // `ktime_sec` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn ktime_sec(&self) -> i64 {
        view_read!(self, ktime_sec)
    }

    #[inline(always)]
    pub(crate) fn set_ktime_sec(&self, ktime_sec: i64) {
        view_write!(self, ktime_sec, ktime_sec)
    }

    // `num_file_content` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn num_file_content(&self) -> usize {
        view_read!(self, num_file_content)
    }

    #[inline(always)]
    pub(crate) fn set_num_file_content(&self, num_file_content: usize) {
        view_write!(self, num_file_content, num_file_content)
    }

    // `file_content` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn file_content(&self) -> *mut FileContent {
        view_read!(self, file_content)
    }

    #[inline(always)]
    pub(crate) fn set_file_content(&self, file_content: *mut FileContent) {
        view_write!(self, file_content, file_content)
    }

    // `legacy_implicit_anim_layer_id` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn legacy_implicit_anim_layer_id(&self) -> u64 {
        view_read!(self, legacy_implicit_anim_layer_id)
    }

    #[inline(always)]
    pub(crate) fn set_legacy_implicit_anim_layer_id(&self, legacy_implicit_anim_layer_id: u64) {
        view_write!(
            self,
            legacy_implicit_anim_layer_id,
            legacy_implicit_anim_layer_id
        )
    }

    // `num_elements` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn num_elements(&self) -> u32 {
        view_read!(self, num_elements)
    }

    #[inline(always)]
    pub(crate) fn set_num_elements(&self, num_elements: u32) {
        view_write!(self, num_elements, num_elements)
    }

    // `root_id` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn root_id(&self) -> u64 {
        view_read!(self, root_id)
    }

    #[inline(always)]
    pub(crate) fn set_root_id(&self, root_id: u64) {
        view_write!(self, root_id, root_id)
    }

    // `tmp_mesh_consecutive_indices` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_mesh_consecutive_indices(&self) -> *mut u32 {
        view_read!(self, tmp_mesh_consecutive_indices)
    }

    #[inline(always)]
    pub(crate) fn set_tmp_mesh_consecutive_indices(&self, tmp_mesh_consecutive_indices: *mut u32) {
        view_write!(
            self,
            tmp_mesh_consecutive_indices,
            tmp_mesh_consecutive_indices
        )
    }

    // `inflate_retain` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn inflate_retain(&self) -> *mut InflateRetain {
        view_read!(self, inflate_retain)
    }

    #[inline(always)]
    pub(crate) fn set_inflate_retain(&self, inflate_retain: *mut InflateRetain) {
        view_write!(self, inflate_retain, inflate_retain)
    }

    // `scene_imp` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn scene_imp(&self) -> *mut SceneImp {
        view_read!(self, scene_imp)
    }

    #[inline(always)]
    pub(crate) fn set_scene_imp(&self, scene_imp: *mut SceneImp) {
        view_write!(self, scene_imp, scene_imp)
    }

    // `blender_full_weights` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn blender_full_weights(&self) -> bool {
        view_read!(self, blender_full_weights)
    }

    #[inline(always)]
    pub(crate) fn set_blender_full_weights(&self, blender_full_weights: bool) {
        view_write!(self, blender_full_weights, blender_full_weights)
    }

    // `retain_vertex_w` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn retain_vertex_w(&self) -> bool {
        view_read!(self, retain_vertex_w)
    }

    #[inline(always)]
    pub(crate) fn set_retain_vertex_w(&self, retain_vertex_w: bool) {
        view_write!(self, retain_vertex_w, retain_vertex_w)
    }

    // `has_scale_helper_nodes` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn has_scale_helper_nodes(&self) -> bool {
        view_read!(self, has_scale_helper_nodes)
    }

    #[inline(always)]
    pub(crate) fn set_has_scale_helper_nodes(&self, has_scale_helper_nodes: bool) {
        view_write!(self, has_scale_helper_nodes, has_scale_helper_nodes)
    }

    // `has_geometry_transform_nodes` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn has_geometry_transform_nodes(&self) -> bool {
        view_read!(self, has_geometry_transform_nodes)
    }

    #[inline(always)]
    pub(crate) fn set_has_geometry_transform_nodes(&self, has_geometry_transform_nodes: bool) {
        view_write!(
            self,
            has_geometry_transform_nodes,
            has_geometry_transform_nodes
        )
    }

    // `synthetic_id_counter` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn synthetic_id_counter(&self) -> u64 {
        view_read!(self, synthetic_id_counter)
    }

    #[inline(always)]
    pub(crate) fn set_synthetic_id_counter(&self, synthetic_id_counter: u64) {
        view_write!(self, synthetic_id_counter, synthetic_id_counter)
    }

    // `size_fn` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn size_fn(&self) -> Option<unsafe extern "C" fn(*mut c_void) -> u64> {
        view_read!(self, size_fn)
    }

    #[inline(always)]
    pub(crate) fn set_size_fn(&self, size_fn: Option<unsafe extern "C" fn(*mut c_void) -> u64>) {
        view_write!(self, size_fn, size_fn)
    }

    // `close_fn` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn close_fn(&self) -> Option<unsafe extern "C" fn(*mut c_void)> {
        view_read!(self, close_fn)
    }

    #[inline(always)]
    pub(crate) fn set_close_fn(&self, close_fn: Option<unsafe extern "C" fn(*mut c_void)>) {
        view_write!(self, close_fn, close_fn)
    }

    // `tmp_element_flag` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_element_flag(&self) -> *mut u8 {
        view_read!(self, tmp_element_flag)
    }

    #[inline(always)]
    pub(crate) fn set_tmp_element_flag(&self, tmp_element_flag: *mut u8) {
        view_write!(self, tmp_element_flag, tmp_element_flag)
    }

    // `element_extra_cap` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn element_extra_cap(&self) -> usize {
        view_read!(self, element_extra_cap)
    }

    // `element_extra_arr` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn element_extra_arr(&self) -> *mut *mut c_void {
        view_read!(self, element_extra_arr)
    }

    // `latest_progress_bytes` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn latest_progress_bytes(&self) -> u64 {
        view_read!(self, latest_progress_bytes)
    }

    #[inline(always)]
    pub(crate) fn set_latest_progress_bytes(&self, latest_progress_bytes: u64) {
        view_write!(self, latest_progress_bytes, latest_progress_bytes)
    }

    // `progress_bytes_total` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn progress_bytes_total(&self) -> u64 {
        view_read!(self, progress_bytes_total)
    }

    #[inline(always)]
    pub(crate) fn set_progress_bytes_total(&self, progress_bytes_total: u64) {
        view_write!(self, progress_bytes_total, progress_bytes_total)
    }

    // `progress_timer` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn progress_timer(&self) -> isize {
        view_read!(self, progress_timer)
    }

    #[inline(always)]
    pub(crate) fn set_progress_timer(&self, progress_timer: isize) {
        view_write!(self, progress_timer, progress_timer)
    }

    // `consecutive_indices` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn consecutive_indices(&self) -> *mut u32 {
        view_read!(self, consecutive_indices)
    }

    #[inline(always)]
    pub(crate) fn set_consecutive_indices(&self, consecutive_indices: *mut u32) {
        view_write!(self, consecutive_indices, consecutive_indices)
    }

    // `zero_indices` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn zero_indices(&self) -> *mut u32 {
        view_read!(self, zero_indices)
    }

    #[inline(always)]
    pub(crate) fn set_zero_indices(&self, zero_indices: *mut u32) {
        view_write!(self, zero_indices, zero_indices)
    }

    // `has_next_child` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn has_next_child(&self) -> bool {
        view_read!(self, has_next_child)
    }

    #[inline(always)]
    pub(crate) fn set_has_next_child(&self, has_next_child: bool) {
        view_write!(self, has_next_child, has_next_child)
    }

    // `top_child_index` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn top_child_index(&self) -> usize {
        view_read!(self, top_child_index)
    }

    #[inline(always)]
    pub(crate) fn set_top_child_index(&self, top_child_index: usize) {
        view_write!(self, top_child_index, top_child_index)
    }

    // `parsed_to_end` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn parsed_to_end(&self) -> bool {
        view_read!(self, parsed_to_end)
    }

    #[inline(always)]
    pub(crate) fn set_parsed_to_end(&self, parsed_to_end: bool) {
        view_write!(self, parsed_to_end, parsed_to_end)
    }

    // `top_nodes_cap` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn top_nodes_cap(&self) -> usize {
        view_read!(self, top_nodes_cap)
    }

    // `top_nodes_len` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn top_nodes_len(&self) -> usize {
        view_read!(self, top_nodes_len)
    }

    #[inline(always)]
    pub(crate) fn set_top_nodes_len(&self, top_nodes_len: usize) {
        view_write!(self, top_nodes_len, top_nodes_len)
    }

    // `top_nodes` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn top_nodes(&self) -> *mut Node {
        view_read!(self, top_nodes)
    }

    // `p_element_id` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn p_element_id(&self) -> *mut u32 {
        view_read!(self, p_element_id)
    }

    #[inline(always)]
    pub(crate) fn set_p_element_id(&self, p_element_id: *mut u32) {
        view_write!(self, p_element_id, p_element_id)
    }

    // `dom_parse_num_children` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn dom_parse_num_children(&self) -> usize {
        view_read!(self, dom_parse_num_children)
    }

    #[inline(always)]
    pub(crate) fn set_dom_parse_num_children(&self, dom_parse_num_children: usize) {
        view_write!(self, dom_parse_num_children, dom_parse_num_children)
    }

    // `dom_parse_toplevel` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn dom_parse_toplevel(&self) -> *mut DomNode {
        view_read!(self, dom_parse_toplevel)
    }

    #[inline(always)]
    pub(crate) fn set_dom_parse_toplevel(&self, dom_parse_toplevel: *mut DomNode) {
        view_write!(self, dom_parse_toplevel, dom_parse_toplevel)
    }

    // `num_templates` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn num_templates(&self) -> usize {
        view_read!(self, num_templates)
    }

    #[inline(always)]
    pub(crate) fn set_num_templates(&self, num_templates: usize) {
        view_write!(self, num_templates, num_templates)
    }

    // `templates` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn templates(&self) -> *mut Template {
        view_read!(self, templates)
    }

    #[inline(always)]
    pub(crate) fn set_templates(&self, templates: *mut Template) {
        view_write!(self, templates, templates)
    }

    // `tmp_element_byte_offset` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_element_byte_offset(&self) -> usize {
        view_read!(self, tmp_element_byte_offset)
    }

    #[inline(always)]
    pub(crate) fn set_tmp_element_byte_offset(&self, tmp_element_byte_offset: usize) {
        view_write!(self, tmp_element_byte_offset, tmp_element_byte_offset)
    }

    // `tmp_arr` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn tmp_arr(&self) -> *mut u8 {
        view_read!(self, tmp_arr)
    }

    // `max_consecutive_indices` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn max_consecutive_indices(&self) -> usize {
        view_read!(self, max_consecutive_indices)
    }

    #[inline(always)]
    pub(crate) fn set_max_consecutive_indices(&self, max_consecutive_indices: usize) {
        view_write!(self, max_consecutive_indices, max_consecutive_indices)
    }

    // `max_zero_indices` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn max_zero_indices(&self) -> usize {
        view_read!(self, max_zero_indices)
    }

    #[inline(always)]
    pub(crate) fn set_max_zero_indices(&self, max_zero_indices: usize) {
        view_write!(self, max_zero_indices, max_zero_indices)
    }

    // `swap_arr_size` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn swap_arr_size(&self) -> usize {
        view_read!(self, swap_arr_size)
    }

    // `swap_arr` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn swap_arr(&self) -> *mut u8 {
        view_read!(self, swap_arr)
    }

    #[inline(always)]
    pub(crate) fn set_swap_arr(&self, swap_arr: *mut u8) {
        view_write!(self, swap_arr, swap_arr)
    }

    // `skip_fn` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn skip_fn(&self) -> Option<unsafe extern "C" fn(*mut c_void, usize) -> bool> {
        view_read!(self, skip_fn)
    }

    #[inline(always)]
    pub(crate) fn set_skip_fn(
        &self,
        skip_fn: Option<unsafe extern "C" fn(*mut c_void, usize) -> bool>,
    ) {
        view_write!(self, skip_fn, skip_fn)
    }

    // `data_offset` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn data_offset(&self) -> u64 {
        view_read!(self, data_offset)
    }

    #[inline(always)]
    pub(crate) fn set_data_offset(&self, data_offset: u64) {
        view_write!(self, data_offset, data_offset)
    }

    // `double_parse_flags` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn double_parse_flags(&self) -> u32 {
        view_read!(self, double_parse_flags)
    }

    #[inline(always)]
    pub(crate) fn set_double_parse_flags(&self, double_parse_flags: u32) {
        view_write!(self, double_parse_flags, double_parse_flags)
    }

    // `read_legacy_settings` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn read_legacy_settings(&self) -> bool {
        view_read!(self, read_legacy_settings)
    }

    #[inline(always)]
    pub(crate) fn set_read_legacy_settings(&self, read_legacy_settings: bool) {
        view_write!(self, read_legacy_settings, read_legacy_settings)
    }

    // `retain_mesh_parts` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn retain_mesh_parts(&self) -> bool {
        view_read!(self, retain_mesh_parts)
    }

    #[inline(always)]
    pub(crate) fn set_retain_mesh_parts(&self, retain_mesh_parts: bool) {
        view_write!(self, retain_mesh_parts, retain_mesh_parts)
    }

    // `sure_fbx` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn sure_fbx(&self) -> bool {
        view_read!(self, sure_fbx)
    }

    #[inline(always)]
    pub(crate) fn set_sure_fbx(&self, sure_fbx: bool) {
        view_write!(self, sure_fbx, sure_fbx)
    }

    // `file_big_endian` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn file_big_endian(&self) -> bool {
        view_read!(self, file_big_endian)
    }

    #[inline(always)]
    pub(crate) fn set_file_big_endian(&self, file_big_endian: bool) {
        view_write!(self, file_big_endian, file_big_endian)
    }

    // `local_big_endian` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn local_big_endian(&self) -> bool {
        view_read!(self, local_big_endian)
    }

    #[inline(always)]
    pub(crate) fn set_local_big_endian(&self, local_big_endian: bool) {
        view_write!(self, local_big_endian, local_big_endian)
    }

    // `from_ascii` — scalar value accessor. Named after the C field it reads; the
    // `from_*(&self)` query shape is intentional, not a conversion constructor.
    #[allow(clippy::wrong_self_convention)]
    #[inline(always)]
    pub(crate) fn from_ascii(&self) -> bool {
        view_read!(self, from_ascii)
    }

    #[inline(always)]
    pub(crate) fn set_from_ascii(&self, from_ascii: bool) {
        view_write!(self, from_ascii, from_ascii)
    }

    // `exporter_version` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn exporter_version(&self) -> u32 {
        view_read!(self, exporter_version)
    }

    #[inline(always)]
    pub(crate) fn set_exporter_version(&self, exporter_version: u32) {
        view_write!(self, exporter_version, exporter_version)
    }

    // FBX file format version (e.g. 7400). Scalar POD field: value getter +
    // setter, both safe (interior mutability via the `UnsafeCell` seam).
    #[inline(always)]
    pub(crate) fn version(&self) -> u32 {
        view_read!(self, version)
    }

    #[inline(always)]
    pub(crate) fn set_version(&self, version: u32) {
        view_write!(self, version, version)
    }

    // Temp-arena allocator. `Allocator` is aliased (copied by raw pointer into
    // sibling contexts) and mutated by `alloc`, so the honest accessor is a raw
    // pointer, not a reference — passing it onward is a safe operation.
    #[inline(always)]
    pub(crate) fn ator_tmp_mut_ptr(&self) -> *mut Allocator {
        view_raw_mut!(self, ator_tmp)
    }

    // `ator_tmp` (Allocator) — typed VIEW handle (reinterpret-in-place); accessors on AllocatorView.
    #[inline(always)]
    pub(crate) fn ator_tmp_view(&self) -> &crate::native::allocator::AllocatorView {
        // SAFETY: reinterpret the owned Allocator field in place; interior-mutable, no validity asserted.
        unsafe {
            &*(&raw mut (*self.get()).ator_tmp as *mut crate::native::allocator::AllocatorView)
        }
    }

    // Input read cursor. Scalar raw pointer: value getter + setter. Copying a
    // `*const u8` out is safe; any later deref/`.add` stays the caller's
    // (unchanged) unsafe obligation.
    #[inline(always)]
    pub(crate) fn data(&self) -> *const u8 {
        view_read!(self, data)
    }

    #[inline(always)]
    pub(crate) fn set_data(&self, data: *const u8) {
        view_write!(self, data, data)
    }

    // Remaining bytes at the read cursor. Scalar `usize`: value getter + setter.
    #[inline(always)]
    pub(crate) fn data_size(&self) -> usize {
        view_read!(self, data_size)
    }

    #[inline(always)]
    pub(crate) fn set_data_size(&self, data_size: usize) {
        view_write!(self, data_size, data_size)
    }

    // Bytes remaining before the next progress-yield checkpoint. Scalar `usize`.
    #[inline(always)]
    pub(crate) fn yield_size(&self) -> usize {
        view_read!(self, yield_size)
    }

    #[inline(always)]
    pub(crate) fn set_yield_size(&self, yield_size: usize) {
        view_write!(self, yield_size, yield_size)
    }

    // Stream read callback. Scalar `Option<fn>` (a nullable fn pointer): value
    // getter + setter. Copies the option out; invoking it stays unsafe.
    #[inline(always)]
    pub(crate) fn read_fn(
        &self,
    ) -> Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize> {
        view_read!(self, read_fn)
    }

    #[inline(always)]
    pub(crate) fn set_read_fn(
        &self,
        read_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>,
    ) {
        view_write!(self, read_fn, read_fn)
    }

    // User pointer passed to `read_fn`. Scalar `*mut c_void`: value getter + setter.
    #[inline(always)]
    pub(crate) fn read_user(&self) -> *mut c_void {
        view_read!(self, read_user)
    }

    #[inline(always)]
    pub(crate) fn set_read_user(&self, read_user: *mut c_void) {
        view_write!(self, read_user, read_user)
    }

    // Start of the current read buffer. Scalar `*const u8`: value getter + setter.
    #[inline(always)]
    pub(crate) fn data_begin(&self) -> *const u8 {
        view_read!(self, data_begin)
    }

    #[inline(always)]
    pub(crate) fn set_data_begin(&self, data_begin: *const u8) {
        view_write!(self, data_begin, data_begin)
    }

    // End-of-input flag. Scalar `bool` (only `0`/`1` ever stored): value getter
    // + setter.
    #[inline(always)]
    pub(crate) fn eof(&self) -> bool {
        view_read!(self, eof)
    }

    #[inline(always)]
    pub(crate) fn set_eof(&self, eof: bool) {
        view_write!(self, eof, eof)
    }

    // Progress-callback byte interval. Scalar `usize`: value getter + setter.
    #[inline(always)]
    pub(crate) fn progress_interval(&self) -> usize {
        view_read!(self, progress_interval)
    }

    #[inline(always)]
    pub(crate) fn set_progress_interval(&self, progress_interval: usize) {
        view_write!(self, progress_interval, progress_interval)
    }

    // Deepest open node on the parse stack. Scalar `*mut Node`: value getter +
    // setter. Copies the pointer out; any deref stays the caller's obligation.
    #[inline(always)]
    pub(crate) fn top_node(&self) -> *mut Node {
        view_read!(self, top_node)
    }

    #[inline(always)]
    pub(crate) fn set_top_node(&self, top_node: *mut Node) {
        view_write!(self, top_node, top_node)
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
    // `&raw mut uc.read_buffer` out-param sites in `refill` stay raw (a value getter
    // cannot express writing back through the field).
    #[inline(always)]
    pub(crate) fn read_buffer(&self) -> *mut u8 {
        view_read!(self, read_buffer)
    }

    #[inline(always)]
    pub(crate) fn set_read_buffer(&self, read_buffer: *mut u8) {
        view_write!(self, read_buffer, read_buffer)
    }

    // Capacity of `read_buffer` in bytes. Scalar `usize`: value getter + setter.
    // The paired `&raw mut uc.read_buffer_size` out-param site in `refill` stays raw.
    #[inline(always)]
    pub(crate) fn read_buffer_size(&self) -> usize {
        view_read!(self, read_buffer_size)
    }

    #[inline(always)]
    pub(crate) fn set_read_buffer_size(&self, read_buffer_size: usize) {
        view_write!(self, read_buffer_size, read_buffer_size)
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
) -> crate::native::error::Fail {
    // Routes through the SAFE `fail_err` wrapper with the anchored `error_view()`;
    // the message-pointer unsafe is encapsulated inside `fail_err`/`fail_imp_err`.
    // Forwards the recording WITNESS the wrapper mints.
    crate::native::error::fail_err(uc.error_view(), cond, func, line)
}

// ufbx.c:6657-6662 (`#else` branch of `UFBXI_FEATURE_ERROR_STACK`)
// `ufbxi_fail_imp_no_stack` — expansion target of the no-msg uc-context check
// macros when the error stack is disabled.
#[cfg(not(feature = "error-stack"))]
#[inline(never)]
pub(crate) fn fail_imp_no_stack(uc: &Context) -> crate::native::error::Fail {
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
// The untyped dispatcher: `fmt` selects the arm, `v` is the out-slot that arm
// writes. Reached only through the typed `get_val_at` below, whose `ValOut`
// bound pins the `fmt`/`v` pairing at compile time.
#[inline(always)]
#[must_use]
unsafe fn get_val_at_raw(node: &NodeView, ix: usize, fmt: u8, v: *mut c_void) -> bool {
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
                // SAFETY: `type_ == Number` means the value-type mask bit for `ix`
                // is set, which the parsers do only for `ix < num_values`, so
                // `vals.add(ix)` is inside the node's value array and `num` is the
                // stored union arm; `v` is the caller's `*mut i32` for fmt `'I'`.
                unsafe { *(v as *mut i32) = (*vals.add(ix)).num.i as i32 };
                true
            } else {
                false
            }
        }
        b'L' => {
            if type_ == ValueType::Number {
                // SAFETY: as `'I'`, with `v` the caller's `*mut i64` for fmt `'L'`.
                unsafe { *(v as *mut i64) = (*vals.add(ix)).num.i };
                true
            } else {
                false
            }
        }
        b'F' => {
            if type_ == ValueType::Number {
                // SAFETY: as `'I'`, with `v` the caller's `*mut f32` for fmt `'F'`.
                unsafe { *(v as *mut f32) = (*vals.add(ix)).num.f as f32 };
                true
            } else {
                false
            }
        }
        b'D' => {
            if type_ == ValueType::Number {
                // SAFETY: as `'I'`, with `v` the caller's `*mut f64` for fmt `'D'`.
                unsafe { *(v as *mut f64) = (*vals.add(ix)).num.f };
                true
            } else {
                false
            }
        }
        b'R' => {
            if type_ == ValueType::Number {
                // SAFETY: as `'I'`, with `v` the caller's `*mut Real` for fmt `'R'`.
                unsafe { *(v as *mut Real) = (*vals.add(ix)).num.f as Real };
                true
            } else {
                false
            }
        }
        b'B' => {
            if type_ == ValueType::Number {
                // SAFETY: as `'I'`, with `v` the caller's `*mut bool` for fmt `'B'`.
                unsafe { *(v as *mut bool) = (*vals.add(ix)).num.i != 0 };
                true
            } else {
                false
            }
        }
        b'Z' => {
            if type_ == ValueType::Number {
                // SAFETY: as `'I'` — the `Number` tag at `ix` implies
                // `ix < num_values`, bounding `vals.add(ix)`, and selects `num`.
                if unsafe { (*vals.add(ix)).num.i } < 0 {
                    return false;
                }
                // SAFETY: as above, with `v` the caller's `*mut usize` for fmt `'Z'`.
                unsafe { *(v as *mut usize) = (*vals.add(ix)).num.i as usize };
                true
            } else {
                false
            }
        }
        b'S' => {
            if type_ == ValueType::String {
                // SAFETY: as `'I'` — the `String` tag at `ix` implies
                // `ix < num_values`, bounding `vals.add(ix)`, and selects the `s`
                // union arm.
                let src: SanitizedString = unsafe { (*vals.add(ix)).s };
                let dst: *mut String = v as *mut String;
                if src.utf8_length > 0 {
                    if src.utf8_length == u32::MAX {
                        return false;
                    }
                    // SAFETY: `dst` is the caller's `*mut String` for fmt `'S'`;
                    // per `SanitizedString`'s layout invariant, the sanitized UTF-8
                    // copy is stored at `raw_data + raw_length + 1` when
                    // `utf8_length > 0`, which the guard above establishes.
                    unsafe {
                        (*dst).data = src.raw_data.add(src.raw_length as usize + 1);
                        (*dst).length = src.utf8_length as usize;
                    }
                } else {
                    // SAFETY: `dst` is the caller's `*mut String` for fmt `'S'`.
                    unsafe {
                        (*dst).data = src.raw_data;
                        (*dst).length = src.raw_length as usize;
                    }
                }
                true
            } else {
                false
            }
        }
        b's' => {
            if type_ == ValueType::String {
                // SAFETY: as `'I'` — the `String` tag at `ix` implies
                // `ix < num_values`, bounding `vals.add(ix)`, and selects `s`.
                let src: SanitizedString = unsafe { (*vals.add(ix)).s };
                let dst: *mut String = v as *mut String;
                // SAFETY: `dst` is the caller's `*mut String` for fmt `'s'`.
                unsafe {
                    (*dst).data = src.raw_data;
                    (*dst).length = src.raw_length as usize;
                }
                true
            } else {
                false
            }
        }
        b'C' => {
            if type_ == ValueType::String {
                // SAFETY: as `'I'` — the `String` tag at `ix` implies
                // `ix < num_values`, bounding `vals.add(ix)`, and selects `s`.
                let src: SanitizedString = unsafe { (*vals.add(ix)).s };
                let dst: *mut *const u8 = v as *mut *const u8;
                if src.utf8_length > 0 {
                    if src.utf8_length == u32::MAX {
                        return false;
                    }
                    // SAFETY: `dst` is the caller's `*mut *const u8` for fmt `'C'`;
                    // the sanitized string sits just past `raw_length + 1`.
                    unsafe { *dst = src.raw_data.add(src.raw_length as usize + 1) };
                } else {
                    // SAFETY: `dst` is the caller's `*mut *const u8` for fmt `'C'`.
                    unsafe { *dst = src.raw_data };
                }
                true
            } else {
                false
            }
        }
        b'c' => {
            if type_ == ValueType::String {
                // SAFETY: as `'I'` — the `String` tag at `ix` implies
                // `ix < num_values`, bounding `vals.add(ix)`, and selects `s`.
                let src: SanitizedString = unsafe { (*vals.add(ix)).s };
                let dst: *mut *const u8 = v as *mut *const u8;
                // SAFETY: `dst` is the caller's `*mut *const u8` for fmt `'c'`.
                unsafe { *dst = src.raw_data };
                true
            } else {
                false
            }
        }
        b'b' => {
            if type_ == ValueType::String {
                // SAFETY: as `'I'` — the `String` tag at `ix` implies
                // `ix < num_values`, bounding `vals.add(ix)`, and selects `s`.
                let src: SanitizedString = unsafe { (*vals.add(ix)).s };
                let dst: *mut Blob = v as *mut Blob;
                // SAFETY: `dst` is the caller's `*mut Blob` for fmt `'b'`.
                unsafe {
                    (*dst).data = src.raw_data;
                    (*dst).size = src.raw_length as usize;
                }
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

// -- Typed value fetch (port-local layer over `ufbxi_get_val_at`)
//
// C pairs a format byte with an untyped out-pointer and the pairing is prose.
// `ValOut` makes it a type: `T::FMT` is the format byte whose
// `get_val_at_raw` arm writes exactly one `T`, so a typed fetch cannot be
// handed a mismatched slot. The source-side checks — does the node hold a
// number / string at `ix`? — stay in the arms and surface as `None`.
//
// # Safety
// An impl asserts that the `get_val_at_raw` arm selected by `FMT` writes one
// fully initialized `Self` through its out-pointer whenever it returns `true`
// and writes nothing when it returns `false`.
pub(crate) unsafe trait ValOut: Sized {
    const FMT: u8;
}

// The checked (`'S'`/`'C'`: sanitized UTF-8 copy when present) and unchecked
// (`'s'`/`'c'`: raw bytes) string forms write the same Rust type; the markers
// keep the format byte recoverable from the type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Checked<T>(pub T);
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Unchecked<T>(pub T);
// `'R'` `ufbx_real`: a marker because `Real` aliases `f32` or `f64`, whose own
// impls are the `'F'` / `'D'` formats.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AsReal(pub Real);
// `'_'`: skip the slot, write nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ignore;

// SAFETY (each impl): the named arm of `get_val_at_raw` writes exactly one
// value of this type through `v` on `true` and nothing on `false`.
unsafe impl ValOut for Ignore {
    const FMT: u8 = b'_';
}
unsafe impl ValOut for i32 {
    const FMT: u8 = b'I';
}
unsafe impl ValOut for i64 {
    const FMT: u8 = b'L';
}
unsafe impl ValOut for f32 {
    const FMT: u8 = b'F';
}
unsafe impl ValOut for f64 {
    const FMT: u8 = b'D';
}
unsafe impl ValOut for AsReal {
    const FMT: u8 = b'R';
}
unsafe impl ValOut for bool {
    const FMT: u8 = b'B';
}
unsafe impl ValOut for usize {
    const FMT: u8 = b'Z';
}
unsafe impl ValOut for Checked<String> {
    const FMT: u8 = b'S';
}
unsafe impl ValOut for Unchecked<String> {
    const FMT: u8 = b's';
}
unsafe impl ValOut for Checked<*const u8> {
    const FMT: u8 = b'C';
}
unsafe impl ValOut for Unchecked<*const u8> {
    const FMT: u8 = b'c';
}
unsafe impl ValOut for Blob {
    const FMT: u8 = b'b';
}

// ufbx.c:7731-7792 `ufbxi_get_val_at` — the typed entry point. `None` is C's
// `false`: the slot is absent, of another kind, or (for `'S'`/`'C'`) an
// unusable sanitized string; nothing is written in that case.
#[inline(always)]
#[must_use]
pub(crate) fn get_val_at<T: ValOut>(node: &NodeView, ix: usize) -> Option<T> {
    let mut out = core::mem::MaybeUninit::<T>::uninit();
    // SAFETY: `ValOut` pins `T::FMT` to the arm that writes exactly one `T`
    // through `v` (the impl contract); `out` is an unaliased local of that
    // type, so the write is in bounds, and it is initialized exactly when the
    // arm reports `true`.
    unsafe {
        get_val_at_raw(node, ix, T::FMT, out.as_mut_ptr().cast::<c_void>())
            .then(|| out.assume_init())
    }
}

// ufbx.c:7805-7809 `ufbxi_get_val1`
#[inline(always)]
#[must_use]
pub(crate) fn get_val1<T0: ValOut>(node: &NodeView) -> Option<T0> {
    get_val_at::<T0>(node, 0)
}

// ufbx.c:7811-7816 `ufbxi_get_val2`
// As in C, slot `i` is fetched only after slots `0..i` succeeded.
#[inline(always)]
#[must_use]
pub(crate) fn get_val2<T0: ValOut, T1: ValOut>(node: &NodeView) -> Option<(T0, T1)> {
    let v0 = get_val_at::<T0>(node, 0)?;
    let v1 = get_val_at::<T1>(node, 1)?;
    Some((v0, v1))
}

// ufbx.c:7818-7824 `ufbxi_get_val3`
#[inline(always)]
#[must_use]
pub(crate) fn get_val3<T0: ValOut, T1: ValOut, T2: ValOut>(
    node: &NodeView,
) -> Option<(T0, T1, T2)> {
    let v0 = get_val_at::<T0>(node, 0)?;
    let v1 = get_val_at::<T1>(node, 1)?;
    let v2 = get_val_at::<T2>(node, 2)?;
    Some((v0, v1, v2))
}

// ufbx.c:7826-7833 `ufbxi_get_val4`
#[inline(always)]
#[must_use]
pub(crate) fn get_val4<T0: ValOut, T1: ValOut, T2: ValOut, T3: ValOut>(
    node: &NodeView,
) -> Option<(T0, T1, T2, T3)> {
    let v0 = get_val_at::<T0>(node, 0)?;
    let v1 = get_val_at::<T1>(node, 1)?;
    let v2 = get_val_at::<T2>(node, 2)?;
    let v3 = get_val_at::<T3>(node, 3)?;
    Some((v0, v1, v2, v3))
}

// ufbx.c:7835-7843 `ufbxi_get_val5`
#[inline(always)]
#[must_use]
pub(crate) fn get_val5<T0: ValOut, T1: ValOut, T2: ValOut, T3: ValOut, T4: ValOut>(
    node: &NodeView,
) -> Option<(T0, T1, T2, T3, T4)> {
    let v0 = get_val_at::<T0>(node, 0)?;
    let v1 = get_val_at::<T1>(node, 1)?;
    let v2 = get_val_at::<T2>(node, 2)?;
    let v3 = get_val_at::<T3>(node, 3)?;
    let v4 = get_val_at::<T4>(node, 4)?;
    Some((v0, v1, v2, v3, v4))
}

// ufbx.c:7845-7851 `ufbxi_find_val1`
#[inline(always)]
#[must_use]
pub(crate) fn find_val1<T0: ValOut>(node: &NodeView, name: *const u8) -> Option<T0> {
    let child: &NodeView = find_child(node, name)?;
    get_val_at::<T0>(child, 0)
}

// ufbx.c:7853-7860 `ufbxi_find_val2`
#[inline(always)]
#[must_use]
pub(crate) fn find_val2<T0: ValOut, T1: ValOut>(
    node: &NodeView,
    name: *const u8,
) -> Option<(T0, T1)> {
    let child: &NodeView = find_child(node, name)?;
    let v0 = get_val_at::<T0>(child, 0)?;
    let v1 = get_val_at::<T1>(child, 1)?;
    Some((v0, v1))
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
    // SAFETY: `name` points to a NUL-terminated C string (fn contract), so its
    // leading byte is readable.
    let leading: u8 = unsafe { *name.add(0) };
    // C: `ufbxi_for(ufbxi_node, c, node->children, node->num_children)`
    // SAFETY: `children`/`num_children` describe a contiguous arena run (built via
    // `push_pop`), valid and stable for `node`'s lifetime `'a`.
    let children: SliceViewIter<'a, Node> =
        unsafe { SliceViewIter::from_raw_parts(node.children(), node.num_children() as usize) };
    for c in children {
        // SAFETY: `c.name()` is a NUL-terminated interned name; its leading byte
        // is readable.
        if unsafe { *c.name().add(0) } != leading {
            continue;
        }
        // SAFETY: both `c.name()` and `name` are NUL-terminated C strings, which
        // is `strcmp`'s contract.
        if unsafe { strcmp(c.name(), name) } == 0 {
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
                    uc.ator_tmp_view(),
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

    let extra: *mut c_void = push_size_zero(uc.tmp_view(), size, 1);
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
            // SAFETY: `name` is a NUL-terminated interned parser name and each
            // literal is NUL-terminated — `strcmp`'s contract.
            unsafe {
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
            // SAFETY: `name` is a NUL-terminated interned parser name; its
            // leading byte is readable.
            if unsafe { *name } == b'L' {
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
                // SAFETY: `name` is a NUL-terminated interned parser name and the
                // literal has at least 12 bytes — `strncmp`'s contract.
                if unsafe { strncmp(name, b"LayerElement\0".as_ptr(), 12) } == 0 {
                    return ParseState::LayerElementOther;
                }
            }
            if name == sp::Shape.as_ptr() {
                return ParseState::Shape;
            }
        }

        ParseState::Deformer => {
            // SAFETY: `name` is a NUL-terminated interned parser name and the
            // literal is NUL-terminated — `strcmp`'s contract.
            if unsafe { strcmp(name, b"AssociateModel\0".as_ptr()) } == 0 {
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
// `name: &[u8]` carries the interned run itself: C compares it against the
// interned `ufbxi_*` string constants by POINTER (see `update_parse_state`), so
// the identity tests are `name.as_ptr() == sp::X.as_ptr()`; the `strcmp`
// fallbacks — for names with no interned constant — become `c_strcmp`, which
// stops at the run's end exactly where `strcmp` stops at the pool's NUL
// terminator. `info: &mut ArrayInfo` is the caller's exclusively owned
// out-slot; the body writes `flags` before reading it and every `true` path
// writes `type_`.
pub(crate) fn is_array_node(
    uc: &Context,
    parent: ParseState,
    name: &[u8],
    info: &mut ArrayInfo,
) -> bool {
    info.flags = 0;

    // Retain all arrays if user wants the DOM representation
    if uc.opts_view().retain_dom() {
        info.flags |= ARRAY_FLAG_RESULT;
    }

    match parent {
        ParseState::Thumbnail => {
            if name.as_ptr() == sp::ImageData.as_ptr() {
                info.type_ = b'c';
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::Geometry | ParseState::Model => {
            if name.as_ptr() == sp::Vertices.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name.as_ptr() == sp::PolygonVertexIndex.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::Edges.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                return true;
            } else if name.as_ptr() == sp::Indexes.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::Points.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::KnotVector.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::KnotVectorU.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::KnotVectorV.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::PointsIndex.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::Normals.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::LegacyModel => {
            if name.as_ptr() == sp::Vertices.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name.as_ptr() == sp::Normals.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name.as_ptr() == sp::Materials.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::PolygonVertexIndex.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::Children.as_ptr() {
                info.type_ = b's';
                return true;
            }
        }

        ParseState::AnimationCurve => {
            if name.as_ptr() == sp::KeyTime.as_ptr() {
                info.type_ = if uc.opts_view().ignore_animation() {
                    b'-'
                } else {
                    b'l'
                };
                return true;
            } else if name.as_ptr() == sp::KeyValueFloat.as_ptr() {
                info.type_ = if uc.opts_view().ignore_animation() {
                    b'-'
                } else {
                    b'r'
                };
                return true;
            } else if name.as_ptr() == sp::KeyAttrFlags.as_ptr() {
                info.type_ = if uc.opts_view().ignore_animation() {
                    b'-'
                } else {
                    b'i'
                };
                return true;
            } else if name.as_ptr() == sp::KeyAttrDataFloat.as_ptr() {
                // The float data in a keyframe attribute array is represented as integers
                // in versions >= 7200 as some of the elements aren't actually floats (!)
                info.type_ = if uc.from_ascii() && uc.version() >= 7200 {
                    b'i'
                } else {
                    b'f'
                };
                if uc.opts_view().ignore_animation() {
                    info.type_ = b'-';
                }
                if uc.from_ascii() && uc.version() < 7200 {
                    info.flags |= ARRAY_FLAG_ACCURATE_F32;
                }
                return true;
            } else if name.as_ptr() == sp::KeyAttrRefCount.as_ptr() {
                info.type_ = if uc.opts_view().ignore_animation() {
                    b'-'
                } else {
                    b'i'
                };
                return true;
            }
        }

        ParseState::Texture => {
            if c_strcmp(name, b"ModelUVTranslation\0") == 0
                || c_strcmp(name, b"ModelUVScaling\0") == 0
                || c_strcmp(name, b"Cropping\0") == 0
            {
                info.type_ = if uc.opts_view().retain_dom() {
                    b'r'
                } else {
                    b'-'
                };
                return true;
            }
        }

        ParseState::Video => {
            if name.as_ptr() == sp::Content.as_ptr() {
                info.type_ = if uc.opts_view().ignore_embedded() {
                    b'-'
                } else {
                    b'C'
                };
                return true;
            }
        }

        ParseState::LayeredTexture => {
            if name.as_ptr() == sp::BlendModes.as_ptr() {
                info.type_ = b'i';
                info.flags |= ARRAY_FLAG_TMP_BUF;
                return true;
            } else if name.as_ptr() == sp::Alphas.as_ptr() {
                info.type_ = b'r';
                info.flags |= ARRAY_FLAG_TMP_BUF;
                return true;
            }
        }

        ParseState::SelectionNode => {
            if name.as_ptr() == sp::VertexIndexArray.as_ptr() {
                info.type_ = b'i';
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::EdgeIndexArray.as_ptr() {
                info.type_ = b'i';
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::PolygonIndexArray.as_ptr() {
                info.type_ = b'i';
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementNormal => {
            if name.as_ptr() == sp::Normals.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name.as_ptr() == sp::NormalsIndex.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::NormalsW.as_ptr() {
                info.type_ = if uc.retain_vertex_w() { b'r' } else { b'-' };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::LayerElementBinormal => {
            if name.as_ptr() == sp::Binormals.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name.as_ptr() == sp::BinormalsIndex.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::BinormalsW.as_ptr() {
                info.type_ = if uc.retain_vertex_w() { b'r' } else { b'-' };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::LayerElementTangent => {
            if name.as_ptr() == sp::Tangents.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name.as_ptr() == sp::TangentsIndex.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::TangentsW.as_ptr() {
                info.type_ = if uc.retain_vertex_w() { b'r' } else { b'-' };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::LayerElementUv => {
            if name.as_ptr() == sp::UV.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name.as_ptr() == sp::UVIndex.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementColor => {
            if name.as_ptr() == sp::Colors.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name.as_ptr() == sp::ColorIndex.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementVertexCrease => {
            if name.as_ptr() == sp::VertexCrease.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name.as_ptr() == sp::VertexCreaseIndex.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementEdgeCrease => {
            if name.as_ptr() == sp::EdgeCrease.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementSmoothing => {
            if name.as_ptr() == sp::Smoothing.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'b'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementVisibility => {
            if name.as_ptr() == sp::Visibility.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'b'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementPolygonGroup => {
            if name.as_ptr() == sp::PolygonGroup.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementHole => {
            if name.as_ptr() == sp::Hole.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'b'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementMaterial => {
            if name.as_ptr() == sp::Materials.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::LayerElementOther => {
            if name.as_ptr() == sp::TextureId.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags |= ARRAY_FLAG_TMP_BUF;
                return true;
            } else if name.as_ptr() == sp::UV.as_ptr() {
                info.type_ = if uc.opts_view().retain_dom() {
                    b'r'
                } else {
                    b'-'
                };
                return true;
            } else if name.as_ptr() == sp::UVIndex.as_ptr() {
                info.type_ = if uc.opts_view().retain_dom() {
                    b'i'
                } else {
                    b'-'
                };
                return true;
            }
        }

        ParseState::GeometryUvInfo => {
            if name.as_ptr() == sp::TextureUV.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            } else if name.as_ptr() == sp::TextureUVVerticeIndex.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::Shape => {
            if name.as_ptr() == sp::Indexes.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
            if name.as_ptr() == sp::Vertices.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
            if name.as_ptr() == sp::Normals.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT | ARRAY_FLAG_PAD_BEGIN;
                return true;
            }
        }

        ParseState::Deformer => {
            if name.as_ptr() == sp::Transform.as_ptr() {
                info.type_ = b'r';
                return true;
            } else if name.as_ptr() == sp::TransformLink.as_ptr() {
                info.type_ = b'r';
                return true;
            } else if name.as_ptr() == sp::Indexes.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::Weights.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::BlendWeights.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::FullWeights.as_ptr() {
                info.type_ = b'r';
                info.flags |= if uc.blender_full_weights() {
                    ARRAY_FLAG_RESULT
                } else {
                    ARRAY_FLAG_TMP_BUF
                };
                return true;
            } else if c_strcmp(name, b"TransformAssociateModel\0") == 0 {
                info.type_ = if uc.opts_view().retain_dom() {
                    b'r'
                } else {
                    b'-'
                };
                return true;
            }
        }

        ParseState::AssociateModel => {
            if name.as_ptr() == sp::Transform.as_ptr() {
                info.type_ = if uc.opts_view().retain_dom() {
                    b'r'
                } else {
                    b'-'
                };
                return true;
            }
        }

        ParseState::LegacyLink => {
            if name.as_ptr() == sp::Transform.as_ptr() {
                info.type_ = b'r';
                return true;
            } else if name.as_ptr() == sp::TransformLink.as_ptr() {
                info.type_ = b'r';
                return true;
            } else if name.as_ptr() == sp::Indexes.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'i'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            } else if name.as_ptr() == sp::Weights.as_ptr() {
                info.type_ = if uc.opts_view().ignore_geometry() {
                    b'-'
                } else {
                    b'r'
                };
                info.flags = ARRAY_FLAG_RESULT;
                return true;
            }
        }

        ParseState::PoseNode => {
            if name.as_ptr() == sp::Matrix.as_ptr() {
                info.type_ = b'r';
                return true;
            }
        }

        ParseState::Channel => {
            if name.as_ptr() == sp::Key.as_ptr() {
                info.type_ = if uc.opts_view().ignore_animation() {
                    b'-'
                } else {
                    b'd'
                };
                return true;
            }
        }

        ParseState::Audio => {
            if name.as_ptr() == sp::Content.as_ptr() {
                info.type_ = if uc.opts_view().ignore_embedded() {
                    b'-'
                } else {
                    b'C'
                };
                return true;
            }
        }

        _ => {
            if name.as_ptr() == sp::BinaryData.as_ptr() {
                info.type_ = if uc.opts_view().ignore_embedded() {
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

// ufbx.c:8508-8605 `ufbxi_is_raw_string`
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
            // SAFETY: `name` is a NUL-terminated interned parser name and the
            // literal is NUL-terminated — `strcmp`'s contract.
            if unsafe { strcmp(name, b"FileId\0".as_ptr()) } == 0 {
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
            // SAFETY: `name` is a NUL-terminated interned parser name and the
            // literal is NUL-terminated — `strcmp`'s contract.
            if unsafe { strcmp(name, b"TextureName\0".as_ptr()) } == 0 {
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
            // SAFETY: `name` is a NUL-terminated interned parser name and the
            // literal is NUL-terminated — `strcmp`'s contract.
            if unsafe { strcmp(name, b"Member\0".as_ptr()) } == 0 {
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
            // SAFETY: `name` is a NUL-terminated interned parser name and the
            // literal is NUL-terminated — `strcmp`'s contract.
            if unsafe { strcmp(name, b"CameraIndexName\0".as_ptr()) } == 0 {
                return true;
            }
        }

        ParseState::LegacyScenePersistence => {
            if name == sp::SceneInfo.as_ptr() {
                return true;
            }
        }

        ParseState::Reference => {
            // SAFETY: `name` is a NUL-terminated interned parser name and the
            // literal is NUL-terminated — `strcmp`'s contract.
            if unsafe { strcmp(name, b"Object\0".as_ptr()) } == 0 {
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
        let result: *mut DomMapping = uc.dom_node_map_view().find(hash, &mapping);
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
// ufbx.c:10719-10811 `ufbxi_retain_dom_node`
// `ufbxi_recursive_function(int, ufbxi_retain_dom_node, ..., UFBXI_MAX_NODE_DEPTH + 1, ...)`
// (ufbx.c:10720-10721): under regression a thread-local depth guard wraps the
// recursive body; otherwise the macro is empty and the wrapper is a plain call.
#[inline(never)]
pub(crate) fn retain_dom_node(
    uc: &Context,
    node: &NodeView,
    p_dom_node: Option<&ScalarView<*mut DomNode>>,
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
fn retain_dom_node_rec(
    uc: &Context,
    node_view: &NodeView,
    p_dom_node: Option<&ScalarView<*mut DomNode>>,
) -> Result<(), Fail> {
    let dst: *mut DomNode = uc.result_view().push_zero(1);
    ufbxi_check!(uc, !dst.is_null(), "dst");
    ufbxi_check!(
        uc,
        !uc.tmp_dom_nodes_view().push_copy_ref(&dst).is_null(),
        "((ufbx_dom_node**)ufbxi_push_size_copy((&uc->tmp_dom_nodes), sizeof(ufbx_dom_node*), (1), (&dst)))"
    );

    if let Some(p_dom_node) = p_dom_node {
        p_dom_node.set(dst);
    }

    // SAFETY: `dst` is the freshly pushed result `DomNode`; copy the node's name
    // span across.
    unsafe {
        (*dst).name.data = node_view.name();
        (*dst).name.length = node_view.name_len() as usize;
    }

    {
        let mapping = DomMapping {
            node_ptr: node_view.as_ptr() as usize,
            dom_node: core::ptr::null_mut(),
        };
        let hash = hash_uptr(mapping.node_ptr);
        let mut result: *mut DomMapping = uc.dom_node_map_view().find(hash, &mapping);
        if result.is_null() {
            result = uc.dom_node_map_view().insert(hash, &mapping);
            ufbxi_check!(uc, !result.is_null(), "result");
        }
        // SAFETY: `result` is a non-null entry owned by `uc`'s `dom_node_map`.
        unsafe {
            (*result).node_ptr = node_view.as_ptr() as usize;
            (*result).dom_node = dst;
        }
    }

    // SAFETY: `dst` is the live result `DomNode`, so its `name` leaf is a live
    // `String` slot.
    let dst_name = unsafe { View::<DomNode>::from_ptr(dst).name_view() };
    sp::push_string_place_str(uc.string_pool_view(), dst_name, false)?;

    if node_view.value_type_mask() == ValueType::Array as u16 {
        // `value_type_mask == Array` selects the `array` arm of `node`'s
        // `content` union (PORTING.md "Unions").
        let arr = node_view.array();
        let val: *mut DomValue = uc.result_view().push_zero(1);
        ufbxi_check!(uc, !val.is_null(), "val");

        // SAFETY: `dst` is the live result `DomNode`; `val` is the freshly pushed
        // `DomValue`, and `arr` the node's array descriptor.
        unsafe {
            (*dst).values.data = val;
            (*dst).values.count = 1;

            let elem_size = array_type_size((*arr).type_);
            (*val).value_str.data = EMPTY_CHAR.as_ptr();
            (*val).value_blob.data = (*arr).data as *const u8;
            (*val).value_blob.size = (*arr).size.wrapping_mul(elem_size);
            // C: `val->value_float = (double)(val->value_int = (int64_t)arr->size);`
            (*val).value_int = (*arr).size as i64;
            (*val).value_float = (*val).value_int as f64;
        }

        // SAFETY: reads `arr`'s `type_` byte and writes `val`'s `type_`, both live.
        match unsafe { (*arr).type_ } {
            b'c' => unsafe { (*val).type_ = DomValueType::Blob },
            b'b' => unsafe { (*val).type_ = DomValueType::Blob },
            b'i' => unsafe { (*val).type_ = DomValueType::ArrayI32 },
            b'l' => unsafe { (*val).type_ = DomValueType::ArrayI64 },
            b'f' => unsafe { (*val).type_ = DomValueType::ArrayF32 },
            b'd' => unsafe { (*val).type_ = DomValueType::ArrayF64 },
            b's' => unsafe { (*val).type_ = DomValueType::ArrayBlob },
            b'C' => unsafe { (*val).type_ = DomValueType::ArrayBlob },
            b'-' => unsafe { (*val).type_ = DomValueType::ArrayIgnored },
            _ => ufbxi_fail!(uc, "Bad array type"),
        }
    } else {
        let mut ix: usize = 0;
        while ix < MAX_NON_ARRAY_VALUES {
            // `as i32` mirrors C's promotion of the `uint16_t` mask to `int`.
            let mask = (((node_view.value_type_mask() as i32) >> (2 * ix)) & 0x3) as u32;
            if mask == 0 {
                break;
            }
            let val: *mut DomValue = uc.tmp_stack_view().push_zero(1);
            ufbxi_check!(uc, !val.is_null(), "val");
            // SAFETY: `val` is the freshly pushed `DomValue`.
            unsafe { (*val).value_str.data = EMPTY_CHAR.as_ptr() };

            if mask == ValueType::String as u32 {
                // SAFETY: `val` is the freshly pushed `DomValue`.
                unsafe { (*val).type_ = DomValueType::String };
                // SAFETY: `val` is the freshly pushed `DomValue`; each fetch
                // yields its value, so the only raw ops are the writes into its
                // own `value_str` / `value_blob` fields.
                unsafe {
                    if let Some(got) = get_val_at::<Checked<String>>(node_view, ix) {
                        (*val).value_str = got.0;
                    }
                    if let Some(got) = get_val_at::<Blob>(node_view, ix) {
                        (*val).value_blob = got;
                    }
                }
            } else {
                ufbx_assert!(mask == ValueType::Number as u32);
                // `node->vals[ix]` reads the `vals` arm of the `ufbxi_node`
                // union (PORTING.md "Unions"); both `i` and `f` of the
                // `ufbxi_value` overlay are read, as in C.
                // SAFETY: `val` is the freshly pushed `DomValue`; `mask == Number`
                // means the value-type mask bit for `ix` is set, which the parsers
                // do only for `ix < num_values`, so `vals.add(ix)` is inside
                // `node`'s value array and `vals`/`num` are the stored union arms.
                unsafe {
                    (*val).type_ = DomValueType::Number;
                    (*val).value_int = (*node_view.vals().add(ix)).num.i;
                    (*val).value_float = (*node_view.vals().add(ix)).num.f;
                }
            }

            ix += 1;
        }

        // SAFETY: `dst` is the live result `DomNode`.
        unsafe { (*dst).values.count = ix };
        let values_data = uc
            .result_view()
            .push_pop::<DomValue>(uc.tmp_stack_view(), ix);
        // SAFETY: `dst` is the live result `DomNode`.
        unsafe { (*dst).values.data = values_data };
        // SAFETY: `dst` is the live result `DomNode`; reading back its
        // `values.data` pointer.
        ufbxi_check!(
            uc,
            !unsafe { (*dst).values.data }.is_null(),
            "dst->values.data"
        );
    }

    if node_view.num_children() > 0 {
        // ufbxi_for(ufbxi_node, child, node->children, node->num_children)
        // `children`/`num_children` describe a contiguous run of child nodes, so
        // `child_end` is one-past-the-end.
        let mut child = node_view.children();
        let child_end = add_ptr(node_view.children(), node_view.num_children() as usize);
        while child != child_end {
            // SAFETY: `child` walks the child run, each a valid parse node
            // living in `uc`'s arena, which outlives the call.
            let child_view: &NodeView = unsafe { NodeView::from_ptr(child) };
            retain_dom_node(uc, child_view, None)?;
            // SAFETY: `child` is before `child_end` within the run, so `add(1)`
            // stays in bounds (up to one-past-the-end).
            child = unsafe { child.add(1) };
        }

        // SAFETY: `dst` is the live result `DomNode`.
        unsafe { (*dst).children.count = node_view.num_children() as usize };
        let children_data = uc
            .result_view()
            .push_pop::<*mut DomNode>(uc.tmp_dom_nodes_view(), node_view.num_children() as usize)
            as *const Ref<DomNode>;
        // SAFETY: `dst` is the live result `DomNode`.
        unsafe { (*dst).children.data = children_data };
        // SAFETY: `dst` is the live result `DomNode`; reading back its
        // `children.data` pointer.
        ufbxi_check!(
            uc,
            !unsafe { (*dst).children.data }.is_null(),
            "dst->children.data"
        );
    }

    Ok(())
}

// ufbx.c:10813-10844 `ufbxi_retain_toplevel`
#[inline(never)]
pub(crate) fn retain_toplevel(uc: &Context, node: Option<&NodeView>) -> Result<(), Fail> {
    if uc.dom_parse_num_children() > 0 {
        let children: *mut *mut DomNode = uc
            .result_view()
            .push_pop(uc.tmp_dom_nodes_view(), uc.dom_parse_num_children());
        ufbxi_check!(uc, !children.is_null(), "children");
        // SAFETY: `dom_parse_toplevel()` yields `uc`'s live top-level `DomNode`
        // pointer (non-null while `dom_parse_num_children > 0`); writing its
        // `children` span.
        unsafe {
            (*uc.dom_parse_toplevel()).children.data = children as *const Ref<DomNode>;
            (*uc.dom_parse_toplevel()).children.count = uc.dom_parse_num_children();
        }
        uc.set_dom_parse_num_children(0);
    }

    if let Some(node_view) = node {
        // SAFETY: `dom_parse_toplevel_mut_ptr` addresses `uc`'s own
        // `dom_parse_toplevel` field — context-owned, write-capable memory live
        // for the call; `ScalarView` is `repr(transparent)` over the slot, and
        // nothing writes that field through `uc` while the slot reference lives.
        let p_dom_node: &ScalarView<*mut DomNode> =
            unsafe { &*(uc.dom_parse_toplevel_mut_ptr() as *const ScalarView<*mut DomNode>) };
        retain_dom_node(uc, node_view, Some(p_dom_node))?;
    } else {
        uc.set_dom_parse_toplevel(core::ptr::null_mut());

        // Called with NULL argument to finish retaining DOM, collect the final nodes to `ufbx_scene`.
        let num_top_nodes = uc.tmp_dom_nodes_view().num_items();
        let nodes: *mut *mut DomNode = uc
            .result_view()
            .push_pop(uc.tmp_dom_nodes_view(), num_top_nodes);
        ufbxi_check!(uc, !nodes.is_null(), "nodes");

        let dom_root: *mut DomNode = uc.result_view().push_zero(1);
        ufbxi_check!(uc, !dom_root.is_null(), "dom_root");

        // SAFETY: `dom_root` is the freshly pushed result `DomNode`.
        unsafe {
            (*dom_root).name.data = EMPTY_CHAR.as_ptr();
            (*dom_root).children.data = nodes as *const Ref<DomNode>;
            (*dom_root).children.count = num_top_nodes;
        }

        // SAFETY: `dom_root` is a live result `DomNode`, so `Ref::from_ptr`
        // adopts it as a retained reference.
        uc.scene_view()
            .set_dom_root(Some(unsafe { Ref::from_ptr(dom_root) }));
    }

    Ok(())
}

// ufbx.c:10846-10853 `ufbxi_retain_toplevel_child`
#[inline(never)]
pub(crate) fn retain_toplevel_child(uc: &Context, child: &NodeView) -> Result<(), Fail> {
    ufbx_assert!(!uc.dom_parse_toplevel().is_null());
    // C passes a NULL out-pointer here; `None` is that absent out-slot.
    retain_dom_node(uc, child, None)?;
    uc.set_dom_parse_num_children(uc.dom_parse_num_children().wrapping_add(1));

    Ok(())
}

// -- General parsing (ufbx.c:10855-11407)

// ufbx.c:10857-10879 `ufbxi_next_line`
#[inline(never)]
pub(crate) fn next_line(line: &StringView, buf: &StringView, skip_space: bool) -> bool {
    if buf.length() == 0 {
        return false;
    }
    let buf_bytes = buf.bytes();
    let length = buf_bytes
        .iter()
        .position(|&c| c == b'\n')
        .map_or(buf_bytes.len(), |newline| newline + 1);

    line.set_data(buf_bytes.as_ptr());
    line.set_length(length);
    buf.set_data(buf_bytes[length..].as_ptr());
    buf.set_length(buf_bytes.len() - length);

    if skip_space {
        let line_bytes = line.bytes();
        let mut begin = 0;
        while begin < line_bytes.len() && is_space(line_bytes[begin]) {
            begin += 1;
        }
        let mut end = line_bytes.len();
        while begin < end && is_space(line_bytes[end - 1]) {
            end -= 1;
        }
        line.set_data(line_bytes[begin..].as_ptr());
        line.set_length(end - begin);
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
        // SAFETY: forwards the caller's `fmt` validity contract to the recursive
        // body unchanged.
        let ret = unsafe { match_skip_rec(fmt, alternation) };
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        ret
    }
    #[cfg(not(feature = "regression"))]
    // SAFETY: forwards the caller's `fmt` validity contract to the recursive
    // body unchanged.
    unsafe {
        match_skip_rec(fmt, alternation)
    }
}

// ufbx.c:10885-10914 `ufbxi_match_skip` body (the `_rec` half of the
// `ufbxi_recursive_function` body; see the wrapper above)
#[inline(never)]
unsafe fn match_skip_rec(mut fmt: *const u8, alternation: bool) -> *const u8 {
    loop {
        // C-parity: `char c = *fmt++;` — C `char` is signed on the oracle
        // targets (PORTING.md char-value rule).
        // SAFETY: `fmt` walks a NUL-terminated match-pattern string (fn contract);
        // reading the current byte, then advancing one within it.
        let mut c: i8 = unsafe { *(fmt as *const i8) };
        fmt = unsafe { fmt.add(1) };
        match c as u8 {
            b'(' => {
                // SAFETY: `fmt` points inside the pattern; `match_skip` returns a
                // pointer at the matching `)`, past which one byte is valid.
                fmt = unsafe { match_skip(fmt, false).add(1) };
            }
            b'\\' => {
                // SAFETY: `\` is followed by an escaped byte within the pattern.
                fmt = unsafe { fmt.add(1) };
            }
            b'[' => {
                // SAFETY: `fmt` points inside the pattern; the class is
                // `]`-terminated, so each read/advance stays within it.
                c = unsafe { *(fmt as *const i8) };
                while c != b']' as i8 {
                    // SAFETY: as above; reading the class byte and advancing.
                    unsafe {
                        c = *(fmt as *const i8);
                        fmt = fmt.add(1);
                    }
                    if c == b'\\' as i8 {
                        // SAFETY: `\` is followed by an escaped byte in the class.
                        unsafe {
                            c = *(fmt as *const i8);
                            fmt = fmt.add(1);
                        }
                    }
                }
                // C-parity trailing `fmt++`: the scan loop above advances past the
                // `]` before it exits (only an empty class leaves `fmt` on the `]`
                // itself), so this steps over the class's quantifier rather than
                // the `]`.
                // SAFETY: the compile-time patterns are well formed and quantify
                // every class, so `fmt` sits at most on the pattern's NUL here and
                // the step lands within the NUL-terminated pattern allocation.
                fmt = unsafe { fmt.add(1) };
            }
            b'|' => {
                if alternation {
                    // SAFETY: `fmt` was advanced past the `|`, so stepping back one
                    // lands on it, still inside the pattern.
                    return unsafe { fmt.offset(-1) };
                }
            }
            b')' | b'\0' => {
                // SAFETY: `fmt` was advanced past the `)`/NUL, so stepping back one
                // lands on it, still inside the pattern.
                return unsafe { fmt.offset(-1) };
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
        // SAFETY: forwards the caller's `p_str`/`end`/`p_fmt` validity contract to
        // the recursive body unchanged.
        let ret = unsafe { match_imp_rec(p_str, end, p_fmt) };
        UFBXI_RECURSION_DEPTH.with(|d| d.set(d.get() - 1));
        ret
    }
    #[cfg(not(feature = "regression"))]
    // SAFETY: forwards the caller's `p_str`/`end`/`p_fmt` validity contract to the
    // recursive body unchanged.
    unsafe {
        match_imp_rec(p_str, end, p_fmt)
    }
}

// ufbx.c:10920-11084 `ufbxi_match_imp` body (the `_rec` half of the
// `ufbxi_recursive_function` body; see the wrapper above)
#[inline(never)]
unsafe fn match_imp_rec(p_str: *mut *const u8, end: *const u8, p_fmt: *mut *const u8) -> bool {
    // SAFETY: `p_str`/`p_fmt` are valid pointers to the caller's `str`/`fmt`
    // cursors (fn contract); read the current cursor values.
    let str_original_begin: *const u8 = unsafe { *p_str };
    let mut str_: *const u8 = str_original_begin;
    let mut fmt_begin: *const u8 = unsafe { *p_fmt };
    let mut fmt: *const u8 = fmt_begin;
    let mut case_insensitive: bool = false;

    let mut count: usize = 0;
    loop {
        // C-parity: `char c = *fmt++;` — signed `char` (PORTING.md char-value
        // rule); every literal compared against it is ASCII.
        // SAFETY: `fmt` walks a NUL-terminated match pattern; read the current
        // byte, then advance one within it.
        let mut c: i8 = unsafe { *(fmt as *const i8) };
        fmt = unsafe { fmt.add(1) };
        if c == 0 {
            // SAFETY: `p_str`/`p_fmt` are valid out-cursors; store the matched
            // `str_` and rewind `fmt` onto the NUL byte just consumed.
            unsafe {
                *p_str = str_;
                *p_fmt = fmt.offset(-1);
            }
            return true;
        }

        let str_begin: *const u8 = str_;
        // SAFETY: when `str_ != end`, `str_` addresses a live input byte within
        // `[str_original_begin, end)`; otherwise it is not dereferenced.
        let mut ref_: i8 = if str_ != end {
            unsafe { *(str_ as *const i8) }
        } else {
            0
        };

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
                // SAFETY: `\` is followed by an escape byte within the pattern;
                // read it and advance one.
                c = unsafe { *(fmt as *const i8) };
                fmt = unsafe { fmt.add(1) };
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
                            // SAFETY: consumes the matched input byte; `str_ < end`
                            // held (a space `ref_` is only read while in range).
                            str_ = unsafe { str_.add(1) };
                        }
                    }
                    b'S' => {
                        if !is_space(ref_ as u8) {
                            ok = true;
                            // SAFETY: `str_ < end` here — every pattern places
                            // `\S` after a matched `\s+` and `next_line(.., true)`
                            // trims trailing spaces, so `ref_` is a real input
                            // byte rather than the synthetic 0; consume it,
                            // mirroring C's `str++`.
                            str_ = unsafe { str_.add(1) };
                        }
                    }
                    b'c' | b'C' => {
                        case_insensitive = c == b'c' as i8;
                        ok = true;
                    }
                    _ => {
                        if ref_ == c {
                            ok = true;
                            // SAFETY: `ref_ == c`, and the escaped pattern byte `c`
                            // is non-NUL (the compile-time patterns never end on a
                            // `\` nor escape a NUL), so `ref_ != 0` implies
                            // `str_ < end`; consume the matched input byte.
                            str_ = unsafe { str_.add(1) };
                        }
                    }
                }
                if !macro_.is_null() {
                    // SAFETY: `str_`/`macro_` are valid cursors into the input and
                    // a NUL-terminated sub-pattern — `match_imp`'s contract.
                    ok = unsafe { match_imp(&raw mut str_, end, &raw mut macro_) };
                }
            }

            b'[' => {
                // SAFETY: the class body is `]`-terminated within the pattern, so
                // every `fmt.add(k)` read below stays inside it.
                while unsafe { *(fmt.add(0) as *const i8) } != b']' as i8 {
                    if unsafe { *(fmt.add(0) as *const i8) } == b'\\' as i8 {
                        // SAFETY: an escaped class member has a following byte.
                        if ref_ == unsafe { *(fmt.add(1) as *const i8) } {
                            ok = true;
                        }
                        fmt = unsafe { fmt.add(2) };
                    } else if unsafe { *(fmt.add(1) as *const i8) } == b'-' as i8 {
                        // SAFETY: a range `a-b` has three bytes within the class.
                        if ref_ >= unsafe { *(fmt.add(0) as *const i8) }
                            && ref_ <= unsafe { *(fmt.add(2) as *const i8) }
                        {
                            ok = true;
                        }
                        fmt = unsafe { fmt.add(3) };
                    } else {
                        // SAFETY: a single class member byte is within the class.
                        if ref_ == unsafe { *(fmt.add(0) as *const i8) } {
                            ok = true;
                        }
                        fmt = unsafe { fmt.add(1) };
                    }
                }
                // SAFETY: step past the terminating `]`, inside the pattern.
                fmt = unsafe { fmt.add(1) };
                if ok {
                    // SAFETY: a class match implies `ref_` was a real input byte,
                    // so `str_ < end`; consume it.
                    str_ = unsafe { str_.add(1) };
                }
            }

            b'(' => {
                // SAFETY: `str_`/`fmt` are valid cursors into the input and the
                // NUL-terminated pattern — `match_imp`'s contract.
                if unsafe { match_imp(&raw mut str_, end, &raw mut fmt) } {
                    ok = true;
                }
            }

            b'|' => {
                // SAFETY: `fmt` points inside the pattern; `match_skip` skips to
                // the alternation/group end within it.
                fmt = unsafe { match_skip(fmt, false) };
                ok = true;
            }

            b')' => {
                // SAFETY: `p_str`/`p_fmt` are valid out-cursors; store the matched
                // position at the group close.
                unsafe {
                    *p_str = str_;
                    *p_fmt = fmt;
                }
                return true;
            }

            b'.' => {
                if ref_ != 0 {
                    ok = true;
                    // SAFETY: `ref_ != 0` implies `str_ < end`; consume the byte.
                    str_ = unsafe { str_.add(1) };
                }
            }

            _ => {
                if c == ref_ {
                    // SAFETY: `c == ref_ != 0` (c was checked non-zero above), so
                    // `str_ < end`; consume the matched byte.
                    str_ = unsafe { str_.add(1) };
                    ok = true;
                }
            }
        }

        let mut did_fail: bool = false;
        // SAFETY: `fmt` points inside the NUL-terminated pattern; read the
        // trailing quantifier byte.
        c = unsafe { *(fmt as *const i8) };
        match c as u8 {
            b'*' => {
                // SAFETY: consume the `*` quantifier byte within the pattern.
                fmt = unsafe { fmt.add(1) };
                if ok {
                    fmt = fmt_begin;
                    count += 1;
                    continue;
                }
            }
            b'+' => {
                // SAFETY: consume the `+` quantifier byte within the pattern.
                fmt = unsafe { fmt.add(1) };
                if ok {
                    fmt = fmt_begin;
                    count += 1;
                    continue;
                } else if count == 0 {
                    did_fail = true;
                }
            }
            b'?' => {
                // SAFETY: consume the `?` quantifier byte within the pattern.
                fmt = unsafe { fmt.add(1) };
            }
            _ => {
                did_fail = !ok;
            }
        }

        if did_fail {
            // SAFETY: `fmt` points inside the pattern; `match_skip` skips to the
            // next alternation.
            fmt = unsafe { match_skip(fmt, true) };
            // SAFETY: `fmt` addresses a byte within the NUL-terminated pattern.
            if unsafe { *fmt } == b'|' {
                // SAFETY: step past the `|`, inside the pattern.
                fmt = unsafe { fmt.add(1) };
                str_ = str_original_begin;
            } else {
                // SAFETY: `p_fmt` is a valid out-cursor; `match_skip` returns a
                // pointer at the group close, past which one byte is valid.
                unsafe { *p_fmt = match_skip(fmt, false).add(1) };
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
//
// `str_: &[u8]` carries C's `str->data`/`str->length` pair — the fn only reads
// the byte span. `fmt` stays raw: the matcher walks the pattern by its NUL
// terminator, a run length no parameter type carries.
//
// # Safety
// `fmt` must address a NUL-terminated match pattern.
#[inline(never)]
pub(crate) unsafe fn r#match(str_: &[u8], fmt: *const u8) -> bool {
    // C: `const char *ptr = str->data, *end = str->data + str->length;`
    let mut ptr: *const u8 = str_.as_ptr();
    let end: *const u8 = str_.as_ptr_range().end;
    let mut fmt: *const u8 = fmt;
    // SAFETY: `ptr`/`end` bound the string's byte span and `fmt` is a
    // NUL-terminated pattern — `match_imp`'s contract.
    if unsafe { match_imp(&raw mut ptr, end, &raw mut fmt) } {
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
    let line = StringView::from_mut(&mut line);
    let buf = StringView::from_mut(&mut buf);

    if format == FileFormat::Fbx {
        // SAFETY: guarded by `size >= BINARY_MAGIC_SIZE`, so `data` and the
        // literal both span at least `BINARY_MAGIC_SIZE` bytes — `memcmp`'s
        // contract.
        if size >= BINARY_MAGIC_SIZE
            && unsafe { memcmp(data, BINARY_MAGIC.as_ptr(), BINARY_MAGIC_SIZE) } == 0
        {
            return true;
        }

        while next_line(line, buf, true) {
            // SAFETY: the pattern literal is NUL-terminated — `r#match`'s
            // format-pointer contract.
            if unsafe {
                r#match(
                    line.bytes(),
                    b";\\s*FBX\\s*\\d+\\.\\d+\\.\\d+\\s*project\\s+file\0".as_ptr(),
                )
            } {
                return true;
            }
            // SAFETY: the pattern literal is NUL-terminated.
            if unsafe { r#match(line.bytes(), b"FBXHeaderExtension:.*\0".as_ptr()) } {
                return true;
            }
        }
    } else if format == FileFormat::Obj {
        while next_line(line, buf, true) {
            let pattern: *const u8 = b"(vn?\\s+\\F|vt)\\s+\\F\\s+\\F.*|f\\s+[\\-/0-9]+\\s+[\\-/0-9]+\\s*[\\-/0-9]+.*|(usemtl|mtllib)\\s+\\S.*\0".as_ptr();
            // SAFETY: `pattern` is NUL-terminated — `r#match`'s
            // format-pointer contract.
            if unsafe { r#match(line.bytes(), pattern) } {
                return true;
            }
        }
    } else if format == FileFormat::Mtl {
        while next_line(line, buf, true) {
            let pattern: *const u8 = b"newmtl\\s+\\S.*\0".as_ptr();
            // SAFETY: `pattern` is NUL-terminated — `r#match`'s
            // format-pointer contract.
            if unsafe { r#match(line.bytes(), pattern) } {
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
                unsafe {
                    let fmt_enum: FileFormat = core::mem::transmute::<u32, FileFormat>(fmt);
                    if is_format(uc.data(), data_size, fmt_enum) {
                        format = fmt_enum;
                        break;
                    }
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
            // above — `String::as_bytes`' contract; the format strings are
            // NUL-terminated literals, `r#match`'s contract.
            unsafe {
                if r#match(extension.as_bytes(), b"\\c\\.fbx\0".as_ptr()) {
                    format = FileFormat::Fbx;
                } else if r#match(extension.as_bytes(), b"\\c\\.obj\0".as_ptr()) {
                    format = FileFormat::Obj;
                } else if r#match(extension.as_bytes(), b"\\c\\.mtl\0".as_ptr()) {
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
        crate::native::parse_ascii::ascii_next_token(uc, uc.ascii_view().token_view())?;

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
//
// Rust-port: the C `bool *p_end` out-flag is RETURNED (`Ok(true)` == C
// `*p_end = true`, "no more children"), so the out-flag slot is a local here and
// no caller needs a raw pointer to reach it.
pub(crate) fn parse_toplevel_child_imp(
    uc: &Context,
    state: ParseState,
    buf: &BufView,
) -> Result<bool, Fail> {
    let mut end: bool = false;
    if uc.from_ascii() {
        crate::native::parse_ascii::ascii_parse_node(uc, 0, state, &mut end, buf, true)?;
    } else {
        crate::native::parse_binary::binary_parse_node(uc, 0, state, &mut end, buf, true)?;
    }

    Ok(end)
}

// ufbx.c:11253-11330 `ufbxi_parse_toplevel`
#[inline(never)]
pub(crate) unsafe fn parse_toplevel(uc: &Context, name: *const u8) -> Result<(), Fail> {
    // C: `ufbxi_for(ufbxi_node, node, uc->top_nodes, uc->top_nodes_len)`
    // SAFETY: `top_nodes`/`top_nodes_len` describe `uc`'s own contiguous run of
    // cached top-level nodes; nothing in the walk below grows or pops that array,
    // so every element stays live and unmoved for the iteration.
    let top_nodes: SliceViewIter<'_, Node> =
        unsafe { SliceViewIter::from_raw_parts(uc.top_nodes(), uc.top_nodes_len()) };
    for node in top_nodes {
        if node.name() == name {
            uc.set_top_node(node.get());
            uc.set_top_child_index(0);
            return Ok(());
        }
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
                uc.tmp_view(),
                false,
            )?;
        } else {
            crate::native::parse_binary::binary_parse_node(
                uc,
                0,
                ParseState::Root,
                &mut end,
                uc.tmp_view(),
                false,
            )?;
        }

        // Top-level node not found
        if end {
            uc.set_top_node(core::ptr::null_mut());
            uc.set_top_child_index(0);
            uc.set_parsed_to_end(true);
            if uc.opts_view().retain_dom() {
                // C: a null `node` tells `retain_toplevel` to finalize the DOM.
                retain_toplevel(uc, None)?;
            }

            // Not needed anymore
            buf_free(uc.tmp_parse_view());

            return Ok(());
        }

        uc.set_top_nodes_len(uc.top_nodes_len() + 1);
        ufbxi_check!(
            uc,
            // SAFETY: grows `uc`'s own paired `top_nodes`/`top_nodes_cap` growth
            // state through its temp allocator (uc construction invariant).
            unsafe {
                grow_array(
                    uc.ator_tmp_view(),
                    uc.top_nodes_mut_ptr(),
                    uc.top_nodes_cap_mut_ptr(),
                    uc.top_nodes_len()
                )
            },
            "ufbxi_grow_array_size((&uc->ator_tmp), sizeof(**(&uc->top_nodes)), (&uc->top_nodes), (&uc->top_nodes_cap), (uc->top_nodes_len))"
        );
        // SAFETY: `top_nodes_len >= 1` (just incremented), so `top_nodes_len - 1`
        // indexes the just-grown array's last slot.
        let node: *mut Node = unsafe { uc.top_nodes().add(uc.top_nodes_len() - 1) };
        // SAFETY: the node parsed above is the top of `uc`'s own stack buffer
        // and `node` is a live slot receiving it — `pop`'s contract.
        unsafe { pop::<Node>(uc.tmp_stack_view(), 1, node) };
        if uc.opts_view().retain_dom() {
            // SAFETY: `node` is a live top-node slot in `uc`'s own `top_nodes`
            // array, which outlives the call.
            retain_toplevel(uc, Some(unsafe { NodeView::from_ptr(node) }))?;
        }

        // Return if we parsed the right one
        // SAFETY: `node` is a live top-node slot; read its `name` field.
        if unsafe { (*node).name } == name {
            uc.set_top_node(node);
            uc.set_top_child_index(usize::MAX);
            return Ok(());
        }

        // If not we need to parse all the children of the node for later
        let mut num_children: u32 = 0;
        // SAFETY: `node` is a live top-node slot; `name` is its NUL-terminated
        // interned name — `update_parse_state`'s contract.
        let state: ParseState = unsafe { update_parse_state(ParseState::Root, (*node).name) };
        if uc.has_next_child() {
            loop {
                if parse_toplevel_child_imp(uc, state, uc.tmp_view())? {
                    break;
                }
                num_children += 1;
            }
        }

        // SAFETY: `node` is a live top-node slot; write its child span.
        unsafe {
            (*node).num_children = num_children;
            (*node).children = uc
                .tmp_view()
                .push_pop::<Node>(uc.tmp_stack_view(), num_children as usize);
        }
        // SAFETY: `node` is a live top-node slot; read back its `children` pointer.
        ufbxi_check!(uc, !unsafe { (*node).children }.is_null(), "node->children");

        if uc.opts_view().retain_dom() {
            // C: `for (size_t i = 0; i < num_children; i++)`
            let mut i: usize = 0;
            while i < num_children as usize {
                // SAFETY: `node` is live; `i < num_children` bounds
                // `children.add(i)` inside the just-populated child run, which
                // lives in `uc`'s arena.
                let child: &NodeView = unsafe { NodeView::from_ptr((*node).children.add(i)) };
                retain_toplevel_child(uc, child)?;
                i += 1;
            }
        }
    }
}

// ufbx.c:11332-11377 `ufbxi_parse_toplevel_child`
//
// Rust-port: the C `ufbxi_node **p_node` out-param is RETURNED as
// `Option<&NodeView>` — `None` is the C `*p_node = NULL` end-of-children signal
// that every call site loops on. The returned view borrows `uc`, the same
// lifetime anchoring `top_node_view()` and the `find_child` family use: the node
// lives either in uc's own `top_child` slot, in its `tmp_parse` buffer, or in
// the caller-supplied `tmp_buf` (itself uc-owned arena memory), all of which
// outlive the borrow.
#[inline(never)]
pub(crate) fn parse_toplevel_child<'a>(
    uc: &'a Context,
    tmp_buf: Option<&'a BufView>,
) -> Result<Option<&'a NodeView>, Fail> {
    // Top-level node not found
    let Some(top_node) = uc.top_node_view() else {
        return Ok(None);
    };

    if uc.top_child_index() == usize::MAX {
        // Parse children on demand
        if tmp_buf.is_none() {
            buf_clear(uc.tmp_parse_view());
        }
        // SAFETY: `top_node`'s `name` is its pooled NUL-terminated interned name
        // — `update_parse_state`'s contract.
        let state: ParseState = unsafe { update_parse_state(ParseState::Root, top_node.name()) };
        let buf: &BufView = match tmp_buf {
            Some(tmp_buf) => tmp_buf,
            None => uc.tmp_parse_view(),
        };
        let end: bool = parse_toplevel_child_imp(uc, state, buf)?;
        if end {
            Ok(None)
        } else {
            // Parse to either reused `uc->top_child` or push if retaining to `tmp_buf`.
            let mut dst: *mut Node = uc.top_child_mut_ptr();
            if let Some(tmp_buf) = tmp_buf {
                dst = tmp_buf.push_zero::<Node>(1);
                ufbxi_check!(uc, !dst.is_null(), "dst");
            }

            // SAFETY: the node parsed above is the top of `uc`'s own stack
            // buffer and `dst` a live `Node` slot receiving it — `pop`'s
            // contract.
            unsafe { pop::<Node>(uc.tmp_stack_view(), 1, dst) };

            // SAFETY: `dst` is the just-popped live `Node`, held either in uc's
            // own `top_child` field or in a `tmp_buf` push — uc-owned arena
            // memory, valid and unmoved for the borrow of `uc`.
            let dst_view: &NodeView = unsafe { NodeView::from_ptr(dst) };

            if uc.opts_view().retain_dom() {
                retain_toplevel_child(uc, dst_view)?;
            }

            Ok(Some(dst_view))
        }
    } else {
        // Iterate already parsed nodes
        let child_index = uc.top_child_index();
        if child_index == top_node.num_children() as usize {
            Ok(None)
        } else {
            uc.set_top_child_index(child_index.wrapping_add(1));
            // SAFETY: `child_index < num_children`, so `children.add(child_index)`
            // stays inside `top_node`'s contiguous child run in uc's arena, valid
            // and unmoved for the borrow of `uc`.
            let child = unsafe { NodeView::from_ptr(top_node.children().add(child_index)) };
            Ok(Some(child))
        }
    }
}

// ufbx.c:11379-11407 `ufbxi_parse_legacy_toplevel`
#[inline(never)]
pub(crate) fn parse_legacy_toplevel(uc: &Context) -> Result<(), Fail> {
    ufbx_assert!(uc.top_nodes_len() == 0);

    let mut end: bool = false;
    if uc.from_ascii() {
        crate::native::parse_ascii::ascii_parse_node(
            uc,
            0,
            ParseState::Root,
            &mut end,
            uc.tmp_view(),
            true,
        )?;
    } else {
        crate::native::parse_binary::binary_parse_node(
            uc,
            0,
            ParseState::Root,
            &mut end,
            uc.tmp_view(),
            true,
        )?;
    }

    // Top-level node not found
    if end {
        uc.set_top_node(core::ptr::null_mut());
        uc.set_top_child_index(0);
        uc.set_parsed_to_end(true);
        return Ok(());
    }

    // SAFETY: the parse above pushed the node onto `uc`'s own `tmp_stack`, so
    // popping one `Node` into `uc`'s own `legacy_node` field (addressed through
    // its raw-ptr getter) matches what is stored there.
    unsafe { pop::<Node>(uc.tmp_stack_view(), 1, uc.legacy_node_mut_ptr()) };
    uc.set_top_child_index(0);
    uc.set_top_node(uc.legacy_node_mut_ptr());

    if uc.opts_view().retain_dom() {
        // C: `legacy_node` was just populated by the `pop` above.
        retain_toplevel(uc, Some(uc.legacy_node_view()))?;
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
            ufbx_assert!(sp::str_less(reg_prev.as_bytes(), (*str_).as_bytes()));
            reg_prev = *str_;
        }
        ufbxi_check!(
            uc,
            // SAFETY: `str_` is a `data`/`length` pair over a static literal,
            // and `'static` outlives the pool, which is why the no-copy
            // (`copy == false`) intern is sound here; `p_out_length` is null,
            // which the `raw == true` path never writes.
            !unsafe {
                sp::push_string_imp(
                    uc.string_pool_view(),
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
// ADDRESS-ONLY `name`: C takes the address of the parameter itself (`&name`)
// as the map key, and `prop_type_map` is keyed on canonical interned pointers
// — no byte behind `name` is ever read (the `is_node_property_name`
// precedent), so the raw param carries no caller obligation.
pub(crate) fn get_prop_type(uc: &Context, name: *const u8) -> PropType {
    // C takes the address of the parameter itself (`&name`) as the map key.
    let name: *const u8 = name;
    let hash = crate::native::hash::hash_ptr!(name);
    let entry: *mut PropTypeName = uc.prop_type_map_view().find(hash, &name);
    if !entry.is_null() {
        // SAFETY: `entry` is a non-null entry owned by `uc`'s `prop_type_map`;
        // read its `type_` field.
        return unsafe { (*entry).type_ };
    }
    PropType::Unknown
}

// ufbx.c:11480-11509 `ufbxi_find_prop_with_key`
// C-parity: the match is a POINTER-IDENTITY test against the interned
// `prop->name.data`, so `name` must be the interned run itself (a `ufbxi_*`
// string constant, or a `ufbx_string`'s own bytes) — its length is never read.
#[inline(never)]
pub(crate) fn find_prop_with_key<'a, M: Mode>(
    props: &'a View<Props, M>,
    name: &[u8],
    key: u32,
) -> Option<&'a View<Prop, M>> {
    let mut props: Option<&'a View<Props, M>> = Some(props);
    while let Some(cur) = props {
        let prop_data: *mut Prop = cur.props_data();
        let mut begin: usize = 0;
        let mut end: usize = cur.props_count();
        while end - begin >= 16 {
            let mid: usize = (begin + end) >> 1;
            // SAFETY: `mid < end <= props_count`, so `prop_data.add(mid)` indexes a
            // live `Prop` in `cur`'s run.
            let p: *const Prop = unsafe { prop_data.add(mid) };
            // SAFETY: `p` addresses a live `Prop`; read its `_internal_key`.
            if unsafe { (*p)._internal_key } < key {
                begin = mid + 1;
            } else {
                end = mid;
            }
        }

        end = cur.props_count();
        while begin < end {
            // SAFETY: `begin < end <= props_count`, so `prop_data.add(begin)`
            // indexes a live `Prop` in `cur`'s run.
            let p: *const Prop = unsafe { prop_data.add(begin) };
            // SAFETY: `p` addresses a live `Prop`; read its `_internal_key`.
            if unsafe { (*p)._internal_key } > key {
                break;
            }
            // SAFETY: `p` addresses a live `Prop`; read its `name.data` and `flags`.
            if unsafe { (*p).name.data } == name.as_ptr()
                && (unsafe { (*p).flags.raw() } & PropFlags::NO_VALUE.raw()) == 0
            {
                // Mode-generic mint from the STORED run pointer (`props_data()`
                // value read) — adequate provenance for either mode.
                // SAFETY: `p` addresses a live `Prop` in `cur`'s run, so minting a
                // `View<Prop, M>` over it reinterprets it in place.
                return Some(unsafe { View::<Prop, M>::mint(p as *mut Prop) });
            }
            begin += 1;
        }

        props = cur.defaults_view();
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
// `ufbxi_get_name_key()`, which handles shorter names. Every call site passes a
// `sp::*` constant (the short ones are NUL-padded to 4 bytes) or an interned
// `ufbx_string`'s own run, so the four reads are always in bounds.
#[inline(always)]
pub(crate) fn find_prop<'a, M: Mode>(
    props: &'a View<Props, M>,
    name: &[u8],
) -> Option<&'a View<Prop, M>> {
    let key =
        (name[0] as u32) << 24 | (name[1] as u32) << 16 | (name[2] as u32) << 8 | (name[3] as u32);
    find_prop_with_key(props, name, key)
}

// ufbx.c:11520-11528 `ufbxi_find_real`
#[inline(always)]
pub(crate) fn find_real<M: Mode>(props: &View<Props, M>, name: &[u8], def: Real) -> Real {
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
pub(crate) fn find_vec3<M: Mode>(
    props: &View<Props, M>,
    name: &[u8],
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
pub(crate) fn find_int<M: Mode>(props: &View<Props, M>, name: &[u8], def: i64) -> i64 {
    match find_prop(props, name) {
        Some(prop) => prop.value_int(),
        None => def,
    }
}

// ufbx.c:11551-11564 `ufbxi_find_enum`
#[inline(always)]
pub(crate) fn find_enum<M: Mode>(
    props: &View<Props, M>,
    name: &[u8],
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
pub(crate) fn matrix_all_zero<M: Mode>(matrix: &View<Matrix, M>) -> bool {
    for i in 0..12 {
        // SAFETY: the view covers a live, initialized `Matrix` whose `m00`..`m23`
        // reals are laid out contiguously (fn comment), so
        // `(matrix.as_ptr() as *const Real).add(i)` for `i in 0..12` reads a live
        // element.
        if unsafe { *(matrix.as_ptr() as *const Real).add(i) } != 0.0 {
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

// Rust-port infrastructure (not a ufbx.c section): the three leaves
// `ufbxi_is_transform_identity` reads off the `ufbx_transform *` its callers
// hand it.
impl<M: Mode> View<Transform, M> {
    #[inline(always)]
    pub(crate) fn translation(&self) -> Vec3 {
        view_read_shared!(self, translation)
    }
    #[inline(always)]
    pub(crate) fn rotation(&self) -> Quat {
        view_read_shared!(self, rotation)
    }
    #[inline(always)]
    pub(crate) fn scale(&self) -> Vec3 {
        view_read_shared!(self, scale)
    }
}

// ufbx.c:11604-11607 `ufbxi_is_transform_identity`
#[inline(never)]
pub(crate) fn is_transform_identity<M: Mode>(t: &View<Transform, M>) -> bool {
    // C: `(bool)((int)ufbxi_is_vec3_zero(..) & (int)ufbxi_is_quat_identity(..)
    // & (int)ufbxi_is_vec3_one(..))` — a non-short-circuiting bitwise `&`.
    ((is_vec3_zero(t.translation()) as i32)
        & (is_quat_identity(t.rotation()) as i32)
        & (is_vec3_one(t.scale()) as i32))
        != 0
}

// ufbx.c:11609-11622 `ufbxi_get_name_key`
#[inline(always)]
pub(crate) fn get_name_key(name: &[u8]) -> u32 {
    let mut key: u32 = 0;
    if name.len() >= 4 {
        key = (name[0] as u32) << 24
            | (name[1] as u32) << 16
            | (name[2] as u32) << 8
            | (name[3] as u32);
    } else {
        for i in 0..4usize {
            key <<= 8;
            if i < name.len() {
                key |= name[i] as u32;
            }
        }
    }
    key
}

// ufbx.c:11624-11631 `ufbxi_get_name_key_c`
#[inline(always)]
pub(crate) unsafe fn get_name_key_c(name: *const u8) -> u32 {
    // SAFETY: `name` is a NUL-terminated C string (fn contract); each `name.add`
    // below is reached only after the preceding bytes are confirmed non-NUL, so
    // it addresses a live byte within the string.
    if unsafe { *name.add(0) } == b'\0' {
        return 0;
    }
    if unsafe { *name.add(1) } == b'\0' {
        return unsafe { (*name.add(0) as u32) << 24 };
    }
    if unsafe { *name.add(2) } == b'\0' {
        return unsafe { (*name.add(0) as u32) << 24 | (*name.add(1) as u32) << 16 };
    }
    unsafe {
        (*name.add(0) as u32) << 24
            | (*name.add(1) as u32) << 16
            | (*name.add(2) as u32) << 8
            | (*name.add(3) as u32)
    }
}

// ufbx.c:11633-11643 `ufbxi_name_key_less`
#[inline(always)]
pub(crate) fn name_key_less<M: Mode>(prop: &View<Prop, M>, data: &[u8], key: u32) -> bool {
    let name_len: usize = data.len();

    if prop._internal_key() < key {
        return true;
    }
    if prop._internal_key() > key {
        return false;
    }

    let prop_len: usize = prop.name().length;
    let len: usize = min_sz(prop_len, name_len);
    // SAFETY: `prop.name.data` spans `prop_len` bytes and `data` spans `name_len`;
    // `len` is their min, so both reads stay in bounds — `memcmp`'s contract.
    let cmp: i32 = unsafe { memcmp(prop.name().data, data.as_ptr(), len) };
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
        uc.node_prop_set_view()
            .grow::<*const u8>(NODE_PROP_NAMES.0.len()),
        "ufbxi_map_grow_size((&uc->node_prop_set), sizeof(const char*), ((sizeof(ufbxi_node_prop_names) / sizeof(*(ufbxi_node_prop_names)))))"
    );
    // C: `for (size_t i = 0; i < ufbxi_arraycount(ufbxi_node_prop_names); i++)`
    let mut i: usize = 0;
    while i < NODE_PROP_NAMES.0.len() {
        let name: *const u8 = NODE_PROP_NAMES.0[i];
        // SAFETY: `name` is a NUL-terminated static literal, so `strlen` finds
        // its terminator and the `'static` bytes outlive the pool — which is
        // what makes the no-copy (`copy == false`) intern into `uc`'s own
        // string pool sound; `p_out_length` is null, which the `raw == true`
        // path never writes.
        let pooled: *const u8 = unsafe {
            sp::push_string_imp(
                uc.string_pool_view(),
                name,
                crate::native::error::strlen(name),
                core::ptr::null_mut(),
                false,
                true,
            )
        };
        ufbxi_check!(uc, !pooled.is_null(), "pooled");
        let hash: u32 = crate::native::hash::hash_ptr!(pooled);
        let entry: *mut *const u8 = uc
            .node_prop_set_view()
            .insert::<*const u8, _>(hash, &pooled);
        ufbxi_check!(uc, !entry.is_null(), "entry");
        // SAFETY: a non-null insert result is a fresh writable entry owned by
        // the map.
        unsafe {
            *entry = pooled;
        }
        i += 1;
    }

    Ok(())
}

// ufbx.c:11736-11744 `ufbxi_is_node_property_name`
// Safe fn: `name` is used address-only — `node_prop_set` is keyed on the
// canonical interned pointer (`hash_ptr` + address-identity `map_cmp_const_char_ptr`),
// so no byte behind `name` is ever read here.
pub(crate) fn is_node_property_name(uc: &Context, name: *const u8) -> bool {
    // You need to call `ufbxi_init_node_prop_names()` before calling this
    ufbx_assert!(uc.node_prop_set_view().size() > 0);

    // C takes the address of the parameter itself (`&name`) as the map key.
    let name: *const u8 = name;
    let hash = crate::native::hash::hash_ptr!(name);
    let entry: *mut *const u8 = uc.node_prop_set_view().find::<*const u8, _>(hash, &name);
    !entry.is_null()
}

// ufbx.c:11746-11760 `ufbxi_load_maps`
#[inline(never)]
pub(crate) fn load_maps(uc: &Context) -> Result<(), Fail> {
    ufbxi_check!(
        uc,
        uc.prop_type_map_view()
            .grow::<PropTypeName>(PROP_TYPE_NAMES.0.len()),
        "ufbxi_map_grow_size((&uc->prop_type_map), sizeof(ufbxi_prop_type_name), ((sizeof(ufbxi_prop_type_names) / sizeof(*(ufbxi_prop_type_names)))))"
    );
    // C: `ufbxi_for(const ufbxi_prop_type_name, name, ufbxi_prop_type_names, ...)`
    for name in PROP_TYPE_NAMES.0.iter() {
        // SAFETY: `name.name` is a NUL-terminated static literal, so `strlen`
        // finds its terminator and the `'static` bytes outlive the pool — what
        // makes the no-copy (`copy == false`) intern into `uc`'s own string
        // pool sound; `p_out_length` is null, which the `raw == true` path
        // never writes.
        let pooled: *const u8 = unsafe {
            sp::push_string_imp(
                uc.string_pool_view(),
                name.name,
                crate::native::error::strlen(name.name),
                core::ptr::null_mut(),
                false,
                true,
            )
        };
        ufbxi_check!(uc, !pooled.is_null(), "pooled");
        let hash: u32 = crate::native::hash::hash_ptr!(pooled);
        let entry: *mut PropTypeName = uc
            .prop_type_map_view()
            .insert::<PropTypeName, _>(hash, &pooled);
        ufbxi_check!(uc, !entry.is_null(), "entry");
        // SAFETY: a non-null insert result is a fresh writable entry owned by
        // the map.
        unsafe {
            (*entry).type_ = name.type_;
            (*entry).name = pooled;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_line_views() {
        let input = b" \tfirst \r\n\n second";
        let mut line = String::new_c(core::ptr::null(), 0);
        let mut buf = String::new_c(input.as_ptr(), input.len());
        let line = StringView::from_mut(&mut line);
        let buf = StringView::from_mut(&mut buf);

        assert!(next_line(line, buf, true));
        assert_eq!(line.bytes(), b"first");
        assert!(next_line(line, buf, true));
        assert_eq!(line.bytes(), b"");
        assert!(next_line(line, buf, true));
        assert_eq!(line.bytes(), b"second");
        assert!(!next_line(line, buf, true));
    }

    // The C static_asserts (ufbx.c:6242/6250) are mirrored as const asserts
    // above; these runtime tests additionally pin the header-trick round trip
    // and the union sizes.
    #[test]
    fn test_get_imp_roundtrip() {
        let mut imp = core::mem::MaybeUninit::<MeshImp>::uninit();
        let imp_ptr = imp.as_mut_ptr();
        unsafe {
            (imp_ptr as *mut u8).expose_provenance();
            let mesh_ptr = &raw mut (*imp_ptr).mesh;
            let back = ImpHandle::<MeshImp>::from_payload(mesh_ptr);
            assert_eq!(back.as_ptr(), imp_ptr);
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
            assert!(get_val_at::<Ignore>(node, 0).is_some());
            assert_eq!(get_val_at::<i32>(node, 0), Some(-7));
            assert_eq!(get_val_at::<f32>(node, 0), Some(-3.5f32));
            assert_eq!(get_val_at::<bool>(node, 0), Some(true));

            // 'Z' rejects negative values.
            assert_eq!(get_val_at::<usize>(node, 0), None);

            // Number formats reject a string value and vice versa.
            assert_eq!(get_val_at::<i32>(node, 1), None);
            assert!(get_val_at::<Checked<String>>(node, 0).is_none());

            // 'S' picks the sanitized UTF-8 copy at `raw_length + 1`.
            let Checked(strv) = get_val_at::<Checked<String>>(node, 1).unwrap();
            assert_eq!(
                core::slice::from_raw_parts(strv.data, strv.length),
                b"utf8xx"
            );
            // 's' always yields the raw string.
            let Unchecked(strv) = get_val_at::<Unchecked<String>>(node, 1).unwrap();
            assert_eq!(core::slice::from_raw_parts(strv.data, strv.length), b"raw");

            let Checked(cstr) = get_val_at::<Checked<*const u8>>(node, 1).unwrap();
            assert_eq!(cstr, raw.as_ptr().add(4));
            let Unchecked(cstr) = get_val_at::<Unchecked<*const u8>>(node, 1).unwrap();
            assert_eq!(cstr, raw.as_ptr());

            let blob: Blob = get_val_at::<Blob>(node, 1).unwrap();
            assert_eq!(blob.size, 3);

            // `utf8_length == UINT32_MAX` marks an unusable sanitized string.
            (*vals.as_mut_ptr().add(1)).s.utf8_length = u32::MAX;
            assert!(get_val_at::<Checked<String>>(node, 1).is_none());
            assert!(get_val_at::<Checked<*const u8>>(node, 1).is_none());
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
        unsafe {
            let node = NodeView::from_ptr(&mut node);
            assert_eq!(get_val1::<i64>(node), Some(1));
            // Value 1 is untyped (NONE), so the second read fails.
            assert!(get_val2::<i64, i64>(node).is_none());
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
        use crate::native::allocator::{init_ator, AllocatorView, NO_ATOR_OPTS};
        use crate::native::buf::{buf_free, BufView};

        let mut uc: std::boxed::Box<InnerContext> =
            unsafe { std::boxed::Box::new_zeroed().assume_init() };
        unsafe {
            // SAFETY: `error`/`ator_tmp` are fields of the boxed context, live
            // and unmoved for the test; the mints are the one vouch for them.
            init_ator(
                &raw mut uc.error,
                AllocatorView::from_ptr(&raw mut uc.ator_tmp),
                NO_ATOR_OPTS,
                c"test",
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

            // SAFETY: `uc.tmp` is a live `Buf` this test owns; minting the
            // `BufView` `buf_free` takes over that field.
            buf_free(BufView::from_ptr(&raw mut uc.tmp));
            crate::native::allocator::free_size(
                uc_ptr.ator_tmp_view(),
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
            assert!(report_progress(Context::from_ptr(&raw mut *uc)).is_err());
            let desc =
                core::slice::from_raw_parts(uc.error.description.data, uc.error.description.length);
            assert_eq!(desc, b"Cancelled");
        }
    }

    #[test]
    fn test_retain_dom_node_tree() {
        use crate::native::allocator::{init_ator, AllocatorView, NO_ATOR_OPTS};
        use crate::native::buf::{buf_free, BufView};
        use crate::native::hash::{map_cmp_uintptr, map_free, map_init, MapView};
        use crate::native::string_pool::{map_cmp_string, string_pool_temp_free, StringPoolView};

        let mut uc: std::boxed::Box<InnerContext> =
            unsafe { std::boxed::Box::new_zeroed().assume_init() };
        unsafe {
            // SAFETY: `error`/`ator_tmp`/`ator_result` are fields of the boxed
            // context, live and unmoved for the test; the mints are the one
            // vouch for them.
            init_ator(
                &raw mut uc.error,
                AllocatorView::from_ptr(&raw mut uc.ator_tmp),
                NO_ATOR_OPTS,
                c"test",
            );
            init_ator(
                &raw mut uc.error,
                AllocatorView::from_ptr(&raw mut uc.ator_result),
                NO_ATOR_OPTS,
                c"test",
            );
            let ator_tmp: *mut Allocator = &mut uc.ator_tmp;
            uc.result.ator = &raw mut uc.ator_result;
            uc.tmp_stack.ator = ator_tmp;
            uc.tmp_dom_nodes.ator = ator_tmp;
            uc.string_pool.error = &mut uc.error;
            uc.string_pool.buf.ator = ator_tmp;
            uc.string_pool.initial_size = 64;
            map_init(
                MapView::from_ptr(&raw mut uc.string_pool.map),
                AllocatorView::from_ptr(ator_tmp),
                map_cmp_string,
                core::ptr::null_mut(),
            );
            map_init(
                MapView::from_ptr(&raw mut uc.dom_node_map),
                AllocatorView::from_ptr(ator_tmp),
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
            assert_eq!(
                retain_dom_node(
                    uc_ptr,
                    NodeView::from_ptr(&mut root),
                    Some(ScalarView::from_mut(&mut dom)),
                ),
                Ok(())
            );
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

            // SAFETY: each is a live `Buf` this test owns; minting the
            // `BufView`s `buf_free` takes over those fields.
            buf_free(BufView::from_ptr(&raw mut uc.result));
            buf_free(BufView::from_ptr(&raw mut uc.tmp_stack));
            buf_free(BufView::from_ptr(&raw mut uc.tmp_dom_nodes));
            buf_free(BufView::from_ptr(&raw mut uc.string_pool.buf));
        }
        // SAFETY: single teardown — `uc` is this test's exclusively owned
        // context, torn down once here at the end of the test, so this is the
        // only release of its `string_pool`'s `temp_str`.
        unsafe {
            string_pool_temp_free(StringPoolView::from_ptr(&raw mut uc.string_pool));
        }
        // SAFETY: `dom_node_map` is `uc`'s own live map, initialized by
        // `map_init` above, and this is its last use.
        map_free(unsafe { MapView::from_ptr(&raw mut uc.dom_node_map) });
        assert_eq!(uc.ator_tmp.current_size, 0);
        assert_eq!(uc.ator_result.current_size, 0);
    }
}
