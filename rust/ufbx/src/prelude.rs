// The `ToRaw` trait converts a Rust-side options/callback value into its raw
// C-ABI representation; its methods are spelled `to_raw(&self)` / `to_raw_mut(&mut
// self)` — non-consuming conversions that build (and may arena-allocate) an owned
// raw value, so `to_*` per Rust convention.
use crate::generated::format_error;
use crate::generated::{
    Error, Progress, ProgressResult, RawAllocator, RawStream, RawVertexStream, Vec2, Vec3, Vec4,
};
use crate::native::view::{view_raw_mut, view_read, view_read_shared, view_write};
use crate::{OpenFileInfo, RawThreadPool};
use std::alloc::{self, GlobalAlloc, Layout, System};
use std::any::Any;
use std::cmp::min;
use std::ffi::c_void;
use std::fmt::{self, Debug, Display, Formatter};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, Index};
use std::ptr::NonNull;
use std::string;
use std::{ptr, slice, str};

// Mirrors C `ufbx_real` (ufbx.h UFBX_REAL_IS_FLOAT): f64 by default, f32
// under the `real-is-f32` feature. C `double` sites (times, curve math the
// C source spells `double`) stay `f64` regardless — only `ufbx_real` follows.
#[cfg(not(feature = "real-is-f32"))]
pub type Real = f64;
#[cfg(feature = "real-is-f32")]
pub type Real = f32;

/// Promote an expression to `f64` for a double-precision intermediate, mirroring
/// ufbx.c's explicit `(double)` casts on `ufbx_real` operands. Under the default
/// `Real = f64` this is an identity cast; under `real-is-f32` it is a genuine
/// f32→f64 widening, so the cast must not be stripped in either build. Spelling
/// it as a macro documents that intent and — because clippy skips casts that
/// originate in a macro expansion — keeps `clippy::unnecessary_cast` live at
/// hand-written sites to catch genuinely redundant casts.
macro_rules! as_f64 {
    ($e:expr) => {
        $e as f64
    };
}
pub(crate) use as_f64;

pub type ThreadPoolContext = usize;
pub type OpenFileContext = usize;

#[repr(C)]
pub struct List<T> {
    pub data: *const T,
    pub count: usize,
    _marker: PhantomData<T>,
}

impl<T> List<T> {
    pub(crate) unsafe fn from_slice(slice: &[T]) -> List<T> {
        List {
            data: slice.as_ptr(),
            count: slice.len(),
            _marker: PhantomData,
        }
    }
    pub(crate) unsafe fn as_static_ref(&self) -> &'static [T] {
        // SAFETY: the caller vouches `data`/`count` describe a live `T` run for
        // `'static` (a context/arena-owned list); `slice_from_ptr` treats a
        // zero `count` as the empty slice, never dereferencing a dangling `data`.
        unsafe { slice_from_ptr(self.data, self.count) }
    }
}

impl<T> AsRef<[T]> for List<T> {
    fn as_ref(&self) -> &[T] {
        unsafe { slice_from_ptr(self.data, self.count) }
    }
}

impl<T> Deref for List<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe { slice_from_ptr(self.data, self.count) }
    }
}

impl<'a, T> IntoIterator for &'a List<T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_ref().iter()
    }
}

impl<T> Index<usize> for List<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        &self.as_ref()[index]
    }
}

#[repr(C)]
pub struct RefList<T> {
    // pub(crate): the native DOM-retention port writes `data` directly from a
    // `ufbxi_push_pop` result (C: `dst->children.data = ufbxi_push_pop(...)`);
    // still private outside the crate.
    pub(crate) data: *const Ref<T>,
    pub count: usize,
    _marker: PhantomData<T>,
}

impl<T> RefList<T> {
    #[allow(dead_code)]
    pub(crate) unsafe fn as_static_ref(&self) -> &'static [Ref<T>] {
        // SAFETY: the caller vouches `data`/`count` describe a live `Ref<T>` run
        // for `'static` (a context/arena-owned list); `slice_from_ptr` maps a
        // zero `count` to the empty slice, never dereferencing a dangling `data`.
        unsafe { slice_from_ptr(self.data, self.count) }
    }
}

impl<T> AsRef<[Ref<T>]> for RefList<T> {
    fn as_ref(&self) -> &[Ref<T>] {
        unsafe { slice_from_ptr(self.data, self.count) }
    }
}

impl<T> Deref for RefList<T> {
    type Target = [Ref<T>];
    fn deref(&self) -> &Self::Target {
        unsafe { slice_from_ptr(self.data, self.count) }
    }
}

pub struct RefIter<'a, T> {
    inner: slice::Iter<'a, Ref<T>>,
}

impl<'a, T> Iterator for RefIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|v| v.as_ref())
    }
}

impl<'a, T> IntoIterator for &'a RefList<T> {
    type Item = &'a T;
    type IntoIter = RefIter<'a, T>;
    fn into_iter(self) -> RefIter<'a, T> {
        RefIter::<'_, T> {
            inner: self.as_ref().iter(),
        }
    }
}

impl<T> Index<usize> for RefList<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        &self.as_ref()[index]
    }
}

#[repr(transparent)]
pub struct Ref<T> {
    ptr: NonNull<T>,
    _marker: PhantomData<T>,
}

impl<T> Ref<T> {
    /// The referenced element's address as a raw pointer — for pointer-identity
    /// compares and raw forwarding without forming a `&T` (the `Ref::as_ref`
    /// Stacked Borrows trap). No deref happens, so this is safe.
    pub(crate) fn ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// View over the referenced element (native-port navigation): the safe
    /// `Ref` → `&View` bridge, so following a stored `ufbx_element*` needs no
    /// raw deref at the call site.
    ///
    /// Safe because the `Ref` invariant (`from_ptr` contract) is exactly the
    /// view mint contract: the pointer is non-null, addresses a live and
    /// unmoved `T` in a stable allocation, and carries that allocation's
    /// write-capable provenance — adequate for both [`Mut`] and [`Const`]
    /// (`crate::native::view::Mode`). The returned lifetime is unbounded, like
    /// the raw `from_ptr` mints, on the arena-stability invariant. The aliasing
    /// discipline is the view's own: no `&mut` over the element while a `Mut`
    /// view is in use, and a `Const` view is not held across a write.
    ///
    /// By value (`Ref` is `Copy`) so a `Ref` read out of viewed memory can be
    /// followed without forming an `&Ref` over the arena.
    #[inline(always)]
    pub(crate) fn view<'a, M: crate::native::view::Mode>(
        self,
    ) -> &'a crate::native::view::View<T, M> {
        // SAFETY: liveness, stability, and write-capable provenance are the
        // `Ref` invariant, established by `from_ptr`'s contract (above).
        unsafe { crate::native::view::View::mint(self.ptr.as_ptr()) }
    }

    // pub(crate): the native port stores raw result-buffer pointers into
    // `ufbx_*` reference fields (C: `uc->scene.dom_root = dom_root;`); the
    // pointer is null-checked by the surrounding `ufbxi_check` first.
    //
    /// # Safety
    /// `ptr` must be non-null and address a live `T` in a stable, write-capable
    /// allocation — a scene/subdivide/tessellate result arena, an externally
    /// retained cache, or (test scaffolding) storage the test keeps alive — that
    /// stays alive and unmoved for as long as any copy of the returned `Ref`,
    /// or the struct that embeds it, exists; and `ptr` must carry that
    /// allocation's write-capable provenance (a `*mut` into it, never a `&T`).
    /// This is the `Ref` invariant that the safe [`Ref::view`] mint and the
    /// public `Deref` rely on.
    pub(crate) unsafe fn from_ptr(ptr: *mut T) -> Ref<T> {
        Ref {
            // SAFETY: the caller vouches `ptr` is non-null — typically a
            // result-buffer pointer guarded by a preceding `ufbxi_check`, or a
            // pointer read back out of an existing `NonNull`-backed `Ref` (see
            // the impl comment above).
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            _marker: PhantomData,
        }
    }
}

// Native-port extension: `Ref<T>` is `Clone + Copy` — C struct assignment is
// memcpy (PORTING.md checklist #15) and the structs embedding `ufbx_element*`
// (`ufbx_connection`, `ufbx_name_element`) are copied by value through the sort
// scratch. A manual impl rather than `derive` so the bound is not `T: Copy`.
// See COMPAT.md §1.
impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Ref<T> {}

impl<T> AsRef<T> for Ref<T> {
    fn as_ref(&self) -> &T {
        unsafe { &*self.ptr.as_ptr() }
    }
}

impl<T> Deref for Ref<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr.as_ptr() }
    }
}

/// Layout twin of [`String`] for values that cross the public boundary, and it
/// carries NO per-leaf discipline of its own: `data` is whatever the caller
/// stored, and `length` may be the `SIZE_MAX` NUL-terminated sentinel that
/// `ufbxi_check_string` normalizes (ufbx.c:26500). Hence no `bytes()` here —
/// a byte run is only recoverable under a caller promise. Strings that have
/// passed `ufbxi_check_string` or the string pool are [`String`].
#[repr(C)]
pub struct RawString {
    pub data: *const u8,
    pub length: usize,
}

// `ScalarView<T>` = std `Cell<T>`, aliased for naming uniformity with the other
// `*View` element handles (used by `_at(i)` scalar-array accessors).
pub(crate) type ScalarView<T> = core::cell::Cell<T>;

// Typed interior-mutable VIEW over `RawString` (non-Copy; subfields read+written).
pub(crate) type RawStringView = crate::native::view::View<RawString>;

// Reads serve both modes: a caller's `&ufbx_string` mints `Const` here.
impl<M: crate::native::view::Mode> crate::native::view::View<RawString, M> {
    #[inline(always)]
    pub(crate) fn data(&self) -> *const u8 {
        view_read_shared!(self, data)
    }
    #[inline(always)]
    pub(crate) fn length(&self) -> usize {
        view_read_shared!(self, length)
    }
}

impl RawStringView {
    #[inline(always)]
    pub(crate) fn set_data(&self, data: *const u8) {
        view_write!(self, data, data)
    }
    #[inline(always)]
    pub(crate) fn set_length(&self, length: usize) {
        view_write!(self, length, length)
    }
}

impl RawString {
    fn new(s: &[u8]) -> Self {
        RawString {
            data: s.as_ptr(),
            length: s.len(),
        }
    }
}

impl Default for RawString {
    fn default() -> Self {
        RawString {
            data: ptr::null(),
            length: 0,
        }
    }
}

#[repr(C)]
pub struct RawBlob {
    pub data: *const u8,
    pub size: usize,
}

// Typed interior-mutable VIEW over `RawBlob` (non-Copy; subfields read+written).
pub(crate) type RawBlobView = crate::native::view::View<RawBlob>;

impl RawBlobView {
    #[inline(always)]
    pub(crate) fn data(&self) -> *const u8 {
        view_read!(self, data)
    }
    #[inline(always)]
    pub(crate) fn size(&self) -> usize {
        view_read!(self, size)
    }
}

// Typed interior-mutable VIEW over the public `Blob` (Copy; subfields read+written).
pub(crate) type BlobView = crate::native::view::View<Blob>;

impl<M: crate::native::view::Mode> crate::native::view::View<Blob, M> {
    #[inline(always)]
    pub(crate) fn data(&self) -> *const u8 {
        view_read_shared!(self, data)
    }
    #[inline(always)]
    pub(crate) fn size(&self) -> usize {
        view_read_shared!(self, size)
    }
}

impl BlobView {
    #[inline(always)]
    pub(crate) fn set_data(&self, data: *const u8) {
        view_write!(self, data, data)
    }
    #[inline(always)]
    pub(crate) fn set_size(&self, size: usize) {
        view_write!(self, size, size)
    }
    // `size` as an in-out slot for callees taking a `*mut usize` out-param.
    #[inline(always)]
    pub(crate) fn size_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, size)
    }
}

// Typed interior-mutable VIEW over `crate::generated::RawThreadOpts` (non-Copy; subfields read+written).
pub(crate) type RawThreadOptsView = crate::native::view::View<crate::generated::RawThreadOpts>;

impl RawThreadOptsView {
    #[inline(always)]
    pub(crate) fn memory_limit(&self) -> usize {
        view_read!(self, memory_limit)
    }
    #[inline(always)]
    pub(crate) fn set_memory_limit(&self, memory_limit: usize) {
        view_write!(self, memory_limit, memory_limit)
    }
}

// Typed interior-mutable VIEW over `RawStream` (the C-callback I/O stream struct);
// callback/`user` leaves are read through it.
// Only the geometry-cache loader reads streams through this view; lean builds
// legitimately strand it (see the dead-code convention in native/).
#[cfg_attr(not(feature = "geometry-cache"), allow(dead_code))]
pub(crate) type RawStreamView = crate::native::view::View<crate::generated::RawStream>;

#[cfg_attr(not(feature = "geometry-cache"), allow(dead_code))]
impl RawStreamView {
    #[inline(always)]
    pub(crate) fn read_fn(
        &self,
    ) -> Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize> {
        view_read!(self, read_fn)
    }
    #[inline(always)]
    pub(crate) fn skip_fn(&self) -> Option<unsafe extern "C" fn(*mut c_void, usize) -> bool> {
        view_read!(self, skip_fn)
    }
    #[inline(always)]
    pub(crate) fn close_fn(&self) -> Option<unsafe extern "C" fn(*mut c_void)> {
        view_read!(self, close_fn)
    }
    #[inline(always)]
    pub(crate) fn user(&self) -> *mut c_void {
        view_read!(self, user)
    }
}

// Typed interior-mutable VIEW over `RawOpenFileCb` — Copy, but `.fn_` is WRITTEN
// (default cb install), so it needs a view not a value getter.
pub(crate) type RawOpenFileCbView = crate::native::view::View<crate::generated::RawOpenFileCb>;

impl RawOpenFileCbView {
    #[inline(always)]
    pub(crate) fn fn_(
        &self,
    ) -> Option<
        unsafe extern "C" fn(
            *mut core::ffi::c_void,
            *mut crate::generated::RawStream,
            *const u8,
            usize,
            *const crate::generated::OpenFileInfo,
        ) -> bool,
    > {
        view_read!(self, fn_)
    }
    #[inline(always)]
    pub(crate) fn set_fn_(
        &self,
        fn_: Option<
            unsafe extern "C" fn(
                *mut core::ffi::c_void,
                *mut crate::generated::RawStream,
                *const u8,
                usize,
                *const crate::generated::OpenFileInfo,
            ) -> bool,
        >,
    ) {
        view_write!(self, fn_, fn_)
    }
}

impl RawBlob {
    fn new(s: &[u8]) -> Self {
        RawBlob {
            data: s.as_ptr(),
            size: s.len(),
        }
    }
}

impl Default for RawBlob {
    fn default() -> Self {
        RawBlob {
            data: ptr::null(),
            size: 0,
        }
    }
}

#[repr(C)]
pub struct RawList<T> {
    pub data: *const T,
    pub count: usize,
}

impl<T> Default for RawList<T> {
    fn default() -> Self {
        RawList {
            data: ptr::null(),
            count: 0,
        }
    }
}

// Typed interior-mutable VIEW over a `List<T>` field (the public safe list),
// reinterpreted in place. Getters + setters (List fields are built by writing
// `.count`/`.data`).
pub(crate) type ListView<T> = crate::native::view::View<List<T>>;

// Mode-generic read surface: serves both `Mut` (arena/context provenance) and
// `Const` (frozen `&`-derived provenance) list views.
impl<T, M: crate::native::view::Mode> crate::native::view::View<List<T>, M> {
    #[inline(always)]
    pub(crate) fn count(&self) -> usize {
        view_read_shared!(self, count)
    }
    #[inline(always)]
    pub(crate) fn data(&self) -> *const T {
        view_read_shared!(self, data)
    }
    /// Safe indexed element view: per the mint's per-leaf discipline a viewed
    /// `List` field holds a valid list — `data` is live and unmoved for `count`
    /// contiguous elements (arena-stable) — so a bounds-checked index yields a
    /// live element (the sibling vouch of `View<String, M>::bytes`).
    #[inline(always)]
    pub(crate) fn at(&self, index: usize) -> &crate::native::view::View<T, M> {
        assert!(index < self.count());
        // SAFETY: `index` is in bounds of the list's own count (checked above),
        // so `data + index` addresses a live element of the run vouched by the
        // list invariant above; the stored `data` pointer carries its own
        // stored provenance, adequate for `M`.
        unsafe { crate::native::view::View::mint((self.data() as *mut T).add(index)) }
    }

    /// Bounds-checked value read from an initialized viewed list.
    #[cfg_attr(not(feature = "triangulation"), allow(dead_code))]
    #[inline(always)]
    pub(crate) fn copy_at(&self, index: usize) -> T
    where
        T: Copy,
    {
        assert!(index < self.count());
        // SAFETY: a viewed `List<T>` holds `count` contiguous initialized
        // elements; the assertion bounds this read to one of them, and `T:
        // Copy` permits returning the stored value by value.
        unsafe { *self.data().add(index) }
    }

    /// C `ufbxi_macro_lower_bound_eq` over this viewed list. The probe
    /// closures receive in-bounds `&View<T, M>` elements minted under the same
    /// list invariant as `at`, so callers' probes are safe code; the search
    /// living here (rather than at each call site) is what moves the probe
    /// vouch onto the list itself. Returns the first matching index.
    #[inline]
    pub(crate) fn lower_bound_eq(
        &self,
        linear_size: usize,
        mut less: impl FnMut(&crate::native::view::View<T, M>) -> bool,
        mut eq: impl FnMut(&crate::native::view::View<T, M>) -> bool,
    ) -> Option<usize> {
        let run = crate::native::view::Run::from_list(self);
        let mut index: usize = usize::MAX;
        crate::native::platform::macro_lower_bound_eq(
            linear_size,
            &mut index,
            0,
            run.len(),
            |ix| less(run.at(ix)),
            |ix| eq(run.at(ix)),
        );
        if index != usize::MAX {
            Some(index)
        } else {
            None
        }
    }

    /// C `ufbxi_macro_upper_bound_eq` (see `lower_bound_eq`): extends a match
    /// found at `begin` to the end of its equal-run, returning the exclusive
    /// end index.
    #[inline]
    pub(crate) fn upper_bound_eq(
        &self,
        linear_size: usize,
        begin: usize,
        mut eq: impl FnMut(&crate::native::view::View<T, M>) -> bool,
    ) -> usize {
        // Snapshot the list's stored data/count pair once, matching the C
        // macro's local `mi_data`/`mi_hi` values for the whole search.
        let run = crate::native::view::Run::from_list(self);
        let mut end: usize = begin;
        crate::native::platform::macro_upper_bound_eq(
            linear_size,
            &mut end,
            begin,
            run.len(),
            |index| eq(run.at(index)),
        );
        end
    }
}

impl<T> ListView<T> {
    #[inline(always)]
    pub(crate) fn set_count(&self, count: usize) {
        view_write!(self, count, count)
    }
    #[inline(always)]
    pub(crate) fn set_data(&self, data: *const T) {
        view_write!(self, data, data)
    }
    /// Address-of-parity projections for the C `&list->data` / `&list->count`
    /// out-param idiom (`ufbxi_read_truncated_array`): a single-leaf `&raw mut`
    /// carrying the view's own provenance, so the caller writes no `unsafe`.
    #[inline(always)]
    pub(crate) fn data_raw(&self) -> *mut *const T {
        view_raw_mut!(self, data)
    }
    #[inline(always)]
    pub(crate) fn count_raw(&self) -> *mut usize {
        view_raw_mut!(self, count)
    }
}

// Typed interior-mutable VIEW over a `RefList<T>` field (the public reference
// list), reinterpreted in place. Getters + setters (`RefList` fields are built
// by writing `.count`/`.data`, where `data` points at `Ref<T>` elements).
pub(crate) type RefListView<T> = crate::native::view::View<RefList<T>>;

// Mode-generic read surface (see the `View<List<T>, M>` impl above).
impl<T, M: crate::native::view::Mode> crate::native::view::View<RefList<T>, M> {
    #[inline(always)]
    pub(crate) fn count(&self) -> usize {
        view_read_shared!(self, count)
    }
    #[inline(always)]
    pub(crate) fn data(&self) -> *const Ref<T> {
        view_read_shared!(self, data)
    }
    /// Safe indexed element view: per the mint's per-leaf discipline a viewed
    /// `RefList` field holds a valid list — `data` is live for `count`
    /// contiguous non-null `Ref<T>` slots — so a bounds-checked index yields a
    /// live element (the ref-list sibling of `View<List<T>, M>::at`).
    #[inline(always)]
    pub(crate) fn at(&self, index: usize) -> &crate::native::view::View<T, M> {
        assert!(index < self.count());
        // SAFETY: `index` is in bounds of the list's own count (checked above),
        // so the slot read is inside the vouched run; the non-null `Ref<T>` is
        // read as bare pointer bits (never through `Ref::as_ref`) and names a
        // live same-scene element whose stored provenance is adequate for `M`.
        unsafe {
            let elem: *mut T = *(self.data() as *const *mut T).add(index);
            crate::native::view::View::mint(elem)
        }
    }
}

impl<T> RefListView<T> {
    #[inline(always)]
    pub(crate) fn set_count(&self, count: usize) {
        view_write!(self, count, count)
    }
    #[inline(always)]
    pub(crate) fn set_data(&self, data: *const Ref<T>) {
        view_write!(self, data, data)
    }
}

// Typed interior-mutable VIEW over a `RawList<T>` field, reinterpreted in place
// (same pattern as the `*OptsView` handles). Leaf getters read the Copy fields;
// `MaybeUninit` means forming `&RawListView` asserts no validity.
pub(crate) type RawListView<T> = crate::native::view::View<RawList<T>>;

impl<T> RawListView<T> {
    #[inline(always)]
    pub(crate) fn count(&self) -> usize {
        view_read!(self, count)
    }
    #[inline(always)]
    pub(crate) fn data(&self) -> *const T {
        view_read!(self, data)
    }
}

#[repr(C)]
#[allow(dead_code)] // Currently not used
pub struct OptionRef<T> {
    ptr: *const T,
    _marker: PhantomData<T>,
}

impl<T> OptionRef<T> {
    pub fn is_some(&self) -> bool {
        self.ptr.is_null()
    }
    pub fn is_none(&self) -> bool {
        !self.ptr.is_null()
    }

    pub fn as_ref(&self) -> Option<&T> {
        unsafe { self.ptr.as_ref() }
    }
}

#[repr(C)]
// Clone/Copy: C `ufbx_string` assignment is a memcpy (PORTING.md checklist
// #15); the native string-pool port stores `ufbx_string` values in the
// interning hashmap and copies them by value.
#[derive(Clone, Copy)]
pub struct String {
    // pub(crate): the native error/string-pool port writes `data` directly
    // (C: `err->description.data = ...`); still private outside the crate.
    pub(crate) data: *const u8,
    pub length: usize,
    _marker: PhantomData<u8>,
}

// Typed interior-mutable VIEW over a `String` field, reinterpreted in place — for
// sites that read OR write String subfields (`err.description.data = ...`).
pub(crate) type StringView = crate::native::view::View<String>;

impl<M: crate::native::view::Mode> crate::native::view::View<String, M> {
    #[inline(always)]
    pub(crate) fn data(&self) -> *const u8 {
        view_read_shared!(self, data)
    }
    #[inline(always)]
    pub(crate) fn length(&self) -> usize {
        view_read_shared!(self, length)
    }
    /// The string's bytes, borrowed for the view's own lifetime.
    #[inline(always)]
    pub(crate) fn bytes(&self) -> &[u8] {
        // SAFETY: per the mint's per-leaf discipline a viewed `String` field
        // holds a valid `ufbx_string`: `data` addresses `length` readable
        // bytes that stay live and unwritten while this view borrow lasts
        // (interned pool strings are never moved or rewritten; ufbx.c:4897).
        unsafe { slice_from_ptr(self.data(), self.length()) }
    }
}

impl StringView {
    #[inline(always)]
    pub(crate) fn set_data(&self, data: *const u8) {
        view_write!(self, data, data)
    }
    #[inline(always)]
    pub(crate) fn set_length(&self, length: usize) {
        view_write!(self, length, length)
    }
    // `data`/`length` as in-out slots for callees taking split `*mut` place
    // out-params (`ufbxi_push_string_place`).
    #[inline(always)]
    pub(crate) fn data_mut_ptr(&self) -> *mut *const u8 {
        view_raw_mut!(self, data)
    }
    #[inline(always)]
    pub(crate) fn length_mut_ptr(&self) -> *mut usize {
        view_raw_mut!(self, length)
    }
}

impl String {
    // Raw constructor for the native port (C: `ufbx_string s = { data, len };`).
    // `_marker` is private to this module, so aggregate construction is only
    // possible here. `const` so the static `ufbxi_strings[]` table can use it.
    pub(crate) const fn new_c(data: *const u8, length: usize) -> String {
        String {
            data,
            length,
            _marker: PhantomData,
        }
    }

    /// Project a by-value `String` to its byte run with a caller-chosen
    /// lifetime — the raw residue path for operands with no view (locals,
    /// temporaries, raw-probe fields). Prefer `View<String, M>::bytes()`,
    /// which carries the interning vouch itself.
    ///
    /// # Safety
    /// `data` must be readable for `length` bytes, unmoved and unwritten for
    /// `'a`.
    #[inline(always)]
    pub(crate) unsafe fn as_bytes<'a>(self) -> &'a [u8] {
        // SAFETY: fn contract above; `slice_from_ptr` never touches `data`
        // when `length == 0`.
        unsafe { slice_from_ptr(self.data, self.length) }
    }

    pub(crate) unsafe fn as_static_ref(&self) -> &'static str {
        // SAFETY: the caller vouches `data`/`length` describe a live, interned
        // UTF-8 run for `'static`; `slice_from_ptr` maps a zero `length` to the
        // empty slice, and interned strings are validated UTF-8 so the
        // `from_utf8_unchecked` invariant holds.
        unsafe { str::from_utf8_unchecked(slice_from_ptr(self.data, self.length)) }
    }
}

impl AsRef<str> for String {
    fn as_ref(&self) -> &str {
        unsafe { str::from_utf8_unchecked(slice_from_ptr(self.data, self.length)) }
    }
}

impl Deref for String {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Default for String {
    fn default() -> String {
        String {
            data: ptr::null(),
            length: 0,
            _marker: PhantomData,
        }
    }
}

impl Display for String {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.deref())
    }
}

impl<'a> PartialEq<&'a str> for String {
    fn eq(&self, rhs: &&'a str) -> bool {
        &self.as_ref() == rhs
    }
}

impl Debug for String {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_ref())
    }
}

#[repr(C)]
// Clone/Copy: C `ufbx_blob` assignment is a memcpy (PORTING.md checklist #15);
// the native port copies `ufbx_prop` (which embeds one) by value, e.g. the
// property sort at ufbx.c:11881 and `ufbxi_deduplicate_properties` ufbx.c:11894.
#[derive(Clone, Copy)]
pub struct Blob {
    // pub(crate): the native string-pool port interns blob payloads in place
    // (C: `p_blob->data = ufbxi_push_string(...)`); still private outside the
    // crate.
    pub(crate) data: *const u8,
    pub size: usize,
    _marker: PhantomData<u8>,
}

impl Blob {
    /// Empty blob descriptor. Its data pointer is never dereferenced.
    pub(crate) const fn empty() -> Blob {
        Blob {
            data: ptr::null(),
            size: 0,
            _marker: PhantomData,
        }
    }

    /// Raw constructor for the native port (C: `ufbx_blob b = { data, size };`).
    /// `_marker` is private to this module, so raw-parts construction is
    /// centralized here.
    ///
    /// # Safety
    /// When `size > 0`, `data` must address `size` readable bytes whose storage
    /// remains live and unwritten for every use of the returned descriptor (and
    /// any copies of it) until that descriptor's pair is replaced. A zero-sized
    /// descriptor never dereferences `data`.
    pub(crate) const unsafe fn new_c(data: *const u8, size: usize) -> Blob {
        Blob {
            data,
            size,
            _marker: PhantomData,
        }
    }
}

pub(crate) unsafe fn slice_from_ptr<'a, T>(data: *const T, len: usize) -> &'a [T] {
    if len > 0 {
        // SAFETY: `len > 0` here, so the caller's contract that `data` points at
        // `len` live `T` values for `'a` supplies a valid, aligned run; the
        // empty branch never touches `data`.
        unsafe { slice::from_raw_parts(data, len) }
    } else {
        &[]
    }
}

unsafe fn slice_from_ptr_mut<'a, T>(data: *mut T, len: usize) -> &'a mut [T] {
    if len > 0 {
        // SAFETY: `len > 0` here, so the caller's contract that `data` points at
        // `len` live `T` values exclusively borrowable for `'a` supplies a
        // valid, aligned run; the empty branch never touches `data`.
        unsafe { slice::from_raw_parts_mut(data, len) }
    } else {
        &mut []
    }
}

impl Deref for Blob {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        unsafe { slice_from_ptr(self.data, self.size) }
    }
}

pub trait AllocatorInterface {
    fn alloc(&mut self, layout: Layout) -> *mut u8;
    fn free(&mut self, ptr: *mut u8, layout: Layout);
    fn realloc(&mut self, ptr: *mut u8, old_layout: Layout, new_layout: Layout) -> *mut u8 {
        self.free(ptr, old_layout);
        self.alloc(new_layout)
    }
    fn free_allocator(&mut self) {}
}

#[repr(transparent)]
#[derive(Default)]
pub struct Unsafe<T>(T);

impl<T> Unsafe<T> {
    pub unsafe fn new(t: T) -> Self {
        Self(t)
    }
}

/// Native-port extension: wrapper for a raw (open, unvalidated) enum value
/// crossing the C ABI, e.g.
/// returned by a user callback: ABI-wise a bare `u32` (`#[repr(transparent)]`),
/// because C only guarantees an integer comes back — materializing `T` directly
/// from a misbehaving callback would be UB for out-of-range values. Compare
/// `.as_raw()` against `T as u32` (what the C code does), or validate
/// explicitly where a genuine `T` is needed.
/// (Same pattern and name as rustc's internal `RawEnum<T>` for LLVM C APIs.)
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RawEnum<T>(u32, PhantomData<fn() -> T>);

impl<T> RawEnum<T> {
    #[inline(always)]
    pub fn from_raw(v: u32) -> Self {
        Self(v, PhantomData)
    }
    #[inline(always)]
    pub fn as_raw(self) -> u32 {
        self.0
    }
}

impl<T> Unsafe<T>
where
    T: Default,
{
    pub fn take(&mut self) -> T {
        mem::take(&mut self.0)
    }
}

pub trait StreamInterface {
    fn read(&mut self, buf: &mut [u8]) -> Option<usize>;
    fn skip(&mut self, bytes: usize) -> bool {
        #![allow(deprecated)]
        unsafe {
            let mut local_buf: [mem::MaybeUninit<u8>; 512] =
                mem::MaybeUninit::uninit().assume_init();
            let mut left = bytes;
            while left > 0 {
                let to_read = min(left, local_buf.len());
                let num_read = self
                    .read(mem::transmute::<&mut [mem::MaybeUninit<u8>], &mut [u8]>(
                        &mut local_buf[0..to_read],
                    ))
                    .unwrap_or(0);
                if num_read != to_read {
                    return false;
                }
                left -= num_read
            }
            true
        }
    }
    fn size(&mut self) -> u64 {
        0
    }
    fn close(&mut self) {}
}

pub enum Stream {
    File(File),
    Read(Box<dyn Read>),
    Box(Box<dyn StreamInterface>),
    Raw(Unsafe<RawStream>),
}

unsafe extern "C" fn global_alloc(_user: *mut c_void, size: usize) -> *mut c_void {
    let layout = Layout::from_size_align(size, 8).unwrap();
    // SAFETY: `layout` carries the fixed valid alignment 8; ufbx's allocator
    // layer drives this C-ABI callback with the non-zero `size` it wants, so
    // the `GlobalAlloc::alloc` non-zero-size precondition holds.
    unsafe { alloc::alloc(layout) as *mut _ }
}

unsafe extern "C" fn global_realloc(
    _user: *mut c_void,
    ptr: *mut c_void,
    old_size: usize,
    new_size: usize,
) -> *mut c_void {
    let old_layout = Layout::from_size_align(old_size, 8).unwrap();
    // SAFETY: `ptr` was returned by `global_alloc`/`global_realloc` for a block
    // described by `old_layout` (same fixed alignment 8, matching `old_size`),
    // and `new_size` is the non-zero size ufbx requests; discharges the
    // `GlobalAlloc::realloc` contract.
    unsafe { alloc::realloc(ptr as *mut _, old_layout, new_size) as *mut _ }
}

unsafe extern "C" fn global_free(_user: *mut c_void, ptr: *mut c_void, size: usize) {
    let layout = Layout::from_size_align(size, 8).unwrap();
    // SAFETY: `ptr` was returned by `global_alloc`/`global_realloc` for a block
    // described by `layout` (fixed alignment 8, matching the recorded `size`);
    // discharges the `GlobalAlloc::dealloc` contract.
    unsafe { alloc::dealloc(ptr as *mut _, layout) }
}

unsafe extern "C" fn system_alloc(_user: *mut c_void, size: usize) -> *mut c_void {
    let layout = Layout::from_size_align(size, 8).unwrap();
    // SAFETY: `layout` carries the fixed valid alignment 8; ufbx's allocator
    // layer drives this C-ABI callback with the non-zero `size` it wants, so
    // the `GlobalAlloc::alloc` non-zero-size precondition holds.
    unsafe { System.alloc(layout) as *mut _ }
}

unsafe extern "C" fn system_realloc(
    _user: *mut c_void,
    ptr: *mut c_void,
    old_size: usize,
    new_size: usize,
) -> *mut c_void {
    let old_layout = Layout::from_size_align(old_size, 8).unwrap();
    // SAFETY: `ptr` was returned by `system_alloc`/`system_realloc` for a block
    // described by `old_layout` (same fixed alignment 8, matching `old_size`),
    // and `new_size` is the non-zero size ufbx requests; discharges the
    // `GlobalAlloc::realloc` contract.
    unsafe { System.realloc(ptr as *mut _, old_layout, new_size) as *mut _ }
}

unsafe extern "C" fn system_free(_user: *mut c_void, ptr: *mut c_void, size: usize) {
    let layout = Layout::from_size_align(size, 8).unwrap();
    // SAFETY: `ptr` was returned by `system_alloc`/`system_realloc` for a block
    // described by `layout` (fixed alignment 8, matching the recorded `size`);
    // discharges the `GlobalAlloc::dealloc` contract.
    unsafe { System.dealloc(ptr as *mut _, layout) }
}

unsafe extern "C" fn allocator_imp_alloc(user: *mut c_void, size: usize) -> *mut c_void {
    // SAFETY: `user` is the `Box<Box<dyn AllocatorInterface>>` leaked by value
    // in `Allocator::to_raw_mut`, handed back unchanged by ufbx, and live until
    // the `free_allocator` callback reclaims it; the reborrow of the inner box
    // is unique for the duration of this callback.
    let ator: &mut Box<dyn AllocatorInterface> =
        unsafe { &mut *(user as *mut Box<dyn AllocatorInterface>) };
    let layout = Layout::from_size_align(size, 8).unwrap();
    ator.alloc(layout) as *mut _
}

unsafe extern "C" fn allocator_imp_realloc(
    user: *mut c_void,
    ptr: *mut c_void,
    old_size: usize,
    new_size: usize,
) -> *mut c_void {
    // SAFETY: `user` is the `Box<Box<dyn AllocatorInterface>>` leaked by value
    // in `Allocator::to_raw_mut`, handed back unchanged by ufbx, and live until
    // the `free_allocator` callback reclaims it; the reborrow of the inner box
    // is unique for the duration of this callback.
    let ator: &mut Box<dyn AllocatorInterface> =
        unsafe { &mut *(user as *mut Box<dyn AllocatorInterface>) };
    let old_layout = Layout::from_size_align(old_size, 8).unwrap();
    let new_layout = Layout::from_size_align(new_size, 8).unwrap();
    ator.realloc(ptr as *mut _, old_layout, new_layout) as *mut _
}

unsafe extern "C" fn allocator_imp_free(user: *mut c_void, ptr: *mut c_void, size: usize) {
    // SAFETY: `user` is the `Box<Box<dyn AllocatorInterface>>` leaked by value
    // in `Allocator::to_raw_mut`, handed back unchanged by ufbx, and live until
    // the `free_allocator` callback reclaims it; the reborrow of the inner box
    // is unique for the duration of this callback.
    let ator: &mut Box<dyn AllocatorInterface> =
        unsafe { &mut *(user as *mut Box<dyn AllocatorInterface>) };
    let layout = Layout::from_size_align(size, 8).unwrap();
    ator.free(ptr as *mut _, layout)
}

unsafe extern "C" fn allocator_imp_box_free_allocator(user: *mut c_void) {
    // SAFETY: `user` is the `Box<Box<dyn AllocatorInterface>>` leaked by value
    // in `Allocator::to_raw_mut`, and ufbx calls this `free_allocator` callback
    // exactly once, so `Box::from_raw` at that same type reclaims ownership and
    // frees it exactly once.
    let mut ator = unsafe { Box::from_raw(user as *mut Box<dyn AllocatorInterface>) };
    ator.free_allocator()
}

pub enum Allocator {
    Libc,
    Global,
    System,
    Box(Box<dyn AllocatorInterface>),
    Raw(Unsafe<RawAllocator>),
}

// Manual impl (rather than `#[derive(Default)]`) to keep the public-API
// rendering stable for the ufbx-rust parity gate; see rust/tools/api/.
#[allow(clippy::derivable_impls)]
impl Default for Allocator {
    fn default() -> Self {
        Allocator::Global
    }
}

impl Allocator {
    pub(crate) fn to_raw(&self) -> RawAllocator {
        match self {
            Allocator::Libc => RawAllocator {
                alloc_fn: None,
                realloc_fn: None,
                free_fn: None,
                free_allocator_fn: None,
                user: ptr::null::<c_void>() as *mut c_void,
            },
            Allocator::Global => RawAllocator {
                alloc_fn: Some(global_alloc),
                realloc_fn: Some(global_realloc),
                free_fn: Some(global_free),
                free_allocator_fn: None,
                user: ptr::null::<c_void>() as *mut c_void,
            },
            Allocator::System => RawAllocator {
                alloc_fn: Some(system_alloc),
                realloc_fn: Some(system_realloc),
                free_fn: Some(system_free),
                free_allocator_fn: None,
                user: ptr::null::<c_void>() as *mut c_void,
            },
            _ => panic!("required mutable reference"),
        }
    }
    pub(crate) fn to_raw_mut(&mut self) -> RawAllocator {
        match self {
            Allocator::Box(_) => {
                // Take the box out BY VALUE so `user` points to the two-word fat
                // `Box<dyn AllocatorInterface>` the `allocator_imp_*` callbacks
                // cast it to; binding it through `&mut self` would leak a
                // one-word `&mut` pointee instead.
                let Allocator::Box(b) = mem::take(self) else {
                    unreachable!()
                };
                RawAllocator {
                    alloc_fn: Some(allocator_imp_alloc),
                    realloc_fn: Some(allocator_imp_realloc),
                    free_fn: Some(allocator_imp_free),
                    free_allocator_fn: Some(allocator_imp_box_free_allocator),
                    user: Box::into_raw(Box::new(b)) as *mut _,
                }
            }
            Allocator::Raw(raw) => raw.take(),
            _ => self.to_raw(),
        }
    }
}

pub enum ThreadPool {
    None,
    Raw(Unsafe<RawThreadPool>),
}

// Manual impl (see Allocator's Default) to keep the public-API rendering stable.
#[allow(clippy::derivable_impls)]
impl Default for ThreadPool {
    fn default() -> Self {
        ThreadPool::None
    }
}

impl ThreadPool {
    pub(crate) fn to_raw(&self) -> RawThreadPool {
        match self {
            ThreadPool::None => RawThreadPool {
                init_fn: None,
                run_fn: None,
                wait_fn: None,
                free_fn: None,
                user: ptr::null::<c_void>() as *mut c_void,
            },
            _ => panic!("required mutable reference"),
        }
    }
    pub(crate) fn to_raw_mut(&mut self) -> RawThreadPool {
        match self {
            ThreadPool::None => RawThreadPool {
                init_fn: None,
                run_fn: None,
                wait_fn: None,
                free_fn: None,
                user: ptr::null::<c_void>() as *mut c_void,
            },
            ThreadPool::Raw(raw) => raw.take(),
        }
    }
}

pub struct VertexStream<'a> {
    pub(crate) data: *mut c_void,
    pub(crate) vertex_count: usize,
    pub(crate) vertex_size: usize,
    _marker: PhantomData<&'a mut ()>,
}

impl VertexStream<'_> {
    pub fn new<T: Copy + Sized>(data: &mut [T]) -> VertexStream<'_> {
        VertexStream {
            data: data.as_mut_ptr() as *mut c_void,
            vertex_count: data.len(),
            vertex_size: mem::size_of::<T>(),
            _marker: PhantomData,
        }
    }
}

impl<'a> ToRaw for [VertexStream<'a>] {
    type Result = Vec<RawVertexStream>;
    fn to_raw_mut(&mut self, _arena: &mut Arena) -> Self::Result {
        self.iter()
            .map(|s| RawVertexStream {
                data: s.data,
                vertex_count: s.vertex_count,
                vertex_size: s.vertex_size,
            })
            .collect()
    }
}

unsafe extern "C" fn stream_read_read(user: *mut c_void, buf: *mut c_void, size: usize) -> usize {
    // SAFETY: `user` is the `Box<dyn Read>` stored as the stream's `user`
    // pointer (see `Stream::to_raw_mut`); live and exclusively owned here.
    let imp = unsafe { &mut *(user as *mut Box<dyn Read>) };
    // SAFETY: ufbx's read callback contract makes `buf` a writable run of
    // `size` bytes for this call. Read destinations are freshly allocated and
    // never written (e.g. the cache reader's own buffer, or a `MaybeUninit`
    // local in `io`), so materializing a `&mut [u8]` over them rests on `u8`
    // having no invalid bit patterns — the C contract promises writability,
    // not initialization.
    imp.read(unsafe { slice_from_ptr_mut(buf as *mut u8, size) })
        .unwrap_or(usize::MAX)
}

unsafe extern "C" fn stream_read_close(user: *mut c_void) {
    // SAFETY: `user` is the `Box<dyn Read>` leaked in `Stream::to_raw_mut`;
    // ufbx calls `close` once, so reclaiming ownership frees it exactly once.
    let _ = unsafe { Box::from_raw(user as *mut Box<dyn Read>) };
}

unsafe extern "C" fn stream_imp_read(user: *mut c_void, buf: *mut c_void, size: usize) -> usize {
    // SAFETY: `user` is the `Box<dyn StreamInterface>` stored as the stream's
    // `user` pointer (see `Stream::to_raw_mut`); live and exclusively owned here.
    let imp = unsafe { &mut *(user as *mut Box<dyn StreamInterface>) };
    // SAFETY: ufbx's read callback contract makes `buf` a writable run of
    // `size` bytes for this call. Read destinations are freshly allocated and
    // never written (e.g. the cache reader's own buffer, or a `MaybeUninit`
    // local in `io`), so materializing a `&mut [u8]` over them rests on `u8`
    // having no invalid bit patterns — the C contract promises writability,
    // not initialization.
    imp.read(unsafe { slice_from_ptr_mut(buf as *mut u8, size) })
        .unwrap_or(usize::MAX)
}

unsafe extern "C" fn stream_imp_skip(user: *mut c_void, size: usize) -> bool {
    // SAFETY: `user` is the `Box<dyn StreamInterface>` stored as the stream's
    // `user` pointer (see `Stream::to_raw_mut`); live and exclusively owned here.
    let imp = unsafe { &mut *(user as *mut Box<dyn StreamInterface>) };
    imp.skip(size)
}

unsafe extern "C" fn stream_imp_size(user: *mut c_void) -> u64 {
    // SAFETY: `user` is the `Box<dyn StreamInterface>` stored as the stream's
    // `user` pointer (see `Stream::to_raw_mut`); live and exclusively owned here.
    let imp = unsafe { &mut *(user as *mut Box<dyn StreamInterface>) };
    imp.size()
}

unsafe extern "C" fn stream_imp_box_close(user: *mut c_void) {
    // SAFETY: `user` is the `Box<dyn StreamInterface>` leaked in
    // `Stream::to_raw_mut`; ufbx calls `close` once, so reclaiming ownership
    // frees it exactly once.
    let mut imp = unsafe { Box::from_raw(user as *mut Box<dyn StreamInterface>) };
    imp.close()
}

// TODO: Expose these somehow
#[allow(dead_code)]
struct StreamRead<T: Read>(T);

impl<T: Read> StreamInterface for StreamRead<T> {
    fn read(&mut self, buf: &mut [u8]) -> Option<usize> {
        self.0.read(buf).ok()
    }
}

struct StreamReadSeek<T: Read + Seek>(T);

impl<T: Read + Seek> StreamInterface for StreamReadSeek<T> {
    fn read(&mut self, buf: &mut [u8]) -> Option<usize> {
        self.0.read(buf).ok()
    }
    fn skip(&mut self, bytes: usize) -> bool {
        match self.0.stream_position() {
            Ok(cur) => match self.0.seek(SeekFrom::Current(bytes as i64)) {
                Ok(pos) => pos == cur + (bytes as u64),
                Err(_) => false,
            },
            Err(_) => false,
        }
    }
    fn size(&mut self) -> u64 {
        if let Ok(start) = self.0.stream_position() {
            if let Ok(end) = self.0.seek(SeekFrom::End(0)) {
                if self.0.seek(SeekFrom::Start(start)).is_ok() {
                    return end - start;
                } else {
                    return u64::MAX;
                }
            }
        }
        0
    }
}

impl Stream {
    pub(crate) fn to_raw_mut(&mut self) -> RawStream {
        let local = mem::replace(
            self,
            Stream::Raw(unsafe { Unsafe::new(Default::default()) }),
        );
        match local {
            Stream::File(file) => {
                let mut inner = Stream::Box(Box::new(StreamReadSeek(file)));
                inner.to_raw_mut()
            }
            Stream::Read(b) => RawStream {
                read_fn: Some(stream_read_read),
                skip_fn: None,
                size_fn: None,
                close_fn: Some(stream_read_close),
                user: Box::into_raw(Box::new(b)) as *mut _,
            },
            Stream::Box(b) => RawStream {
                read_fn: Some(stream_imp_read),
                skip_fn: Some(stream_imp_skip),
                size_fn: Some(stream_imp_size),
                close_fn: Some(stream_imp_box_close),
                user: Box::into_raw(Box::new(b)) as *mut _,
            },
            Stream::Raw(mut raw) => raw.take(),
        }
    }
}

pub unsafe extern "C" fn call_progress_cb<F>(
    user: *mut c_void,
    progress: *const Progress,
) -> RawEnum<ProgressResult>
where
    F: FnMut(&Progress) -> ProgressResult,
{
    // SAFETY: `user` is the `&mut F` closure ufbx was given as the progress
    // callback's user pointer; live and exclusively borrowable for this call.
    let func: &mut F = unsafe { &mut *(user as *mut F) };
    // SAFETY: ufbx passes a valid `*const Progress` for the duration of the call.
    RawEnum::<ProgressResult>::from_raw((func)(unsafe { &*progress }) as u32)
}

pub unsafe extern "C" fn call_open_file_cb<F>(
    user: *mut c_void,
    dst: *mut RawStream,
    path: *const u8,
    path_len: usize,
    info: *const OpenFileInfo,
) -> bool
where
    F: FnMut(&str, &OpenFileInfo) -> Option<Stream>,
{
    // SAFETY: `user` is the `&mut F` closure ufbx was given as the open-file
    // callback's user pointer; live and exclusively borrowable for this call.
    let func: &mut F = unsafe { &mut *(user as *mut F) };

    // SAFETY: ufbx's open-file contract makes `path`/`path_len` a readable byte
    // run for this call.
    let path_str = match str::from_utf8(unsafe { slice_from_ptr(path, path_len) }) {
        Ok(path_str) => path_str,
        Err(_) => return false,
    };

    // SAFETY: ufbx passes a valid `*const OpenFileInfo` for the duration of the call.
    let mut stream = match (func)(path_str, unsafe { &*info }) {
        Some(stream) => stream,
        None => return false,
    };

    // SAFETY: ufbx passes a valid, writable `*mut RawStream` destination.
    unsafe {
        *dst = stream.to_raw_mut();
    }
    true
}

pub unsafe extern "C" fn call_close_memory_cb<F>(
    user: *mut c_void,
    data: *mut c_void,
    data_size: usize,
) where
    F: FnMut(*mut c_void, usize),
{
    // SAFETY: `user` is the `&mut F` closure ufbx was given as the
    // close-memory callback's user pointer; live and exclusively borrowable
    // for this call.
    let func: &mut F = unsafe { &mut *(user as *mut F) };
    (func)(data, data_size)
}

#[repr(transparent)]
pub struct InlineBuf<T> {
    pub data: mem::MaybeUninit<T>,
}

impl<T> Default for InlineBuf<T> {
    fn default() -> Self {
        Self {
            data: mem::MaybeUninit::uninit(),
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #![allow(deprecated)]
        unsafe {
            let mut local_buf: [mem::MaybeUninit<u8>; 1024] =
                mem::MaybeUninit::uninit().assume_init();
            let length = format_error(
                mem::transmute::<&mut [mem::MaybeUninit<u8>], &mut [u8]>(local_buf.as_mut_slice()),
                self,
            );
            f.write_str(str::from_utf8_unchecked(mem::transmute::<
                &[mem::MaybeUninit<u8>],
                &[u8],
            >(&local_buf[..length])))
        }
    }
}

#[repr(C)]
pub struct ExternalRef<'a, T> {
    data: T,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> ExternalRef<'a, T> {
    pub unsafe fn new(t: T) -> Self {
        Self {
            data: t,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> AsRef<T> for ExternalRef<'a, T> {
    fn as_ref(&self) -> &T {
        &self.data
    }
}

impl<'a, T> Deref for ExternalRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

pub(crate) struct Arena {
    items: Vec<Box<dyn Any>>,
}

impl Arena {
    pub fn new() -> Arena {
        Arena { items: Vec::new() }
    }

    #[allow(unused)]
    pub fn push_box<T: 'static>(&mut self, s: Box<T>) -> *const T {
        let ptr = Box::as_ref(&s) as *const T;
        self.items.push(s);
        ptr
    }
    pub fn push_vec<T: 'static>(&mut self, vec: Vec<T>) -> *const T {
        if vec.is_empty() {
            return ptr::null();
        }
        let ptr = vec.as_ptr();
        self.items.push(Box::new(vec));
        ptr
    }
}

pub fn format_flags(f: &mut fmt::Formatter<'_>, names: &[(&str, u32)], value: u32) -> fmt::Result {
    let mut has_any = false;

    for (name, v) in names {
        if (value & v) != 0 {
            let prefix = if has_any { "|" } else { "" };
            has_any = true;
            write!(f, "{}{}", prefix, name)?;
        }
    }

    if !has_any {
        write!(f, "NONE")?;
    }

    Ok(())
}

impl fmt::Display for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (f.precision(), f.sign_plus()) {
            (None, false) => write!(f, "({}, {})", self.x, self.y),
            (None, true) => write!(f, "({:+}, {:+})", self.x, self.y),
            (Some(p), false) => write!(f, "({1:.0$}, {2:.0$})", p, self.x, self.y),
            (Some(p), true) => write!(f, "({1:+.0$}, {2:+.0$})", p, self.x, self.y),
        }
    }
}

impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (f.precision(), f.sign_plus()) {
            (None, false) => write!(f, "({}, {}, {})", self.x, self.y, self.z),
            (None, true) => write!(f, "({:+}, {:+}, {:+})", self.x, self.y, self.z),
            (Some(p), false) => write!(f, "({1:.0$}, {2:.0$}, {3:.0$})", p, self.x, self.y, self.z),
            (Some(p), true) => write!(
                f,
                "({1:+.0$}, {2:+.0$}, {3:+.0$})",
                p, self.x, self.y, self.z
            ),
        }
    }
}

impl fmt::Display for Vec4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (f.precision(), f.sign_plus()) {
            (None, false) => write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w),
            (None, true) => write!(f, "({:+}, {:+}, {:+}, {})", self.x, self.y, self.z, self.w),
            (Some(p), false) => write!(
                f,
                "({1:.0$}, {2:.0$}, {3:.0$}, {4:.0$})",
                p, self.x, self.y, self.z, self.w
            ),
            (Some(p), true) => write!(
                f,
                "({1:+.0$}, {2:+.0$}, {3:+.0$}, {4:+.0$})",
                p, self.x, self.y, self.z, self.w
            ),
        }
    }
}

pub(crate) trait ToRaw {
    type Result: 'static;
    fn to_raw(&self, _arena: &mut Arena) -> Self::Result {
        panic!("type must be used via mutable reference")
    }
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        self.to_raw(arena)
    }
}

pub enum StringOpt<'a> {
    Unset,
    Ref(&'a str),
    Owned(string::String),
}

// Manual impl (see Allocator's Default) to keep the public-API rendering stable.
#[allow(clippy::derivable_impls)]
impl Default for StringOpt<'_> {
    fn default() -> Self {
        StringOpt::Unset
    }
}

impl<'a> From<&'a str> for StringOpt<'a> {
    fn from(v: &'a str) -> Self {
        StringOpt::Ref(v)
    }
}

impl<'a> From<string::String> for StringOpt<'a> {
    fn from(v: string::String) -> Self {
        StringOpt::Owned(v)
    }
}

impl<'a> ToRaw for StringOpt<'a> {
    type Result = RawString;
    fn to_raw(&self, _arena: &mut Arena) -> Self::Result {
        match self {
            StringOpt::Unset => RawString::default(),
            StringOpt::Ref(r) => RawString::new(r.as_bytes()),
            StringOpt::Owned(r) => RawString::new(r.as_bytes()),
        }
    }
}

pub enum BlobOpt<'a> {
    Unset,
    Ref(&'a [u8]),
    Owned(Vec<u8>),
}

// Manual impl (see Allocator's Default) to keep the public-API rendering stable.
#[allow(clippy::derivable_impls)]
impl Default for BlobOpt<'_> {
    fn default() -> Self {
        BlobOpt::Unset
    }
}

impl<'a> From<&'a [u8]> for BlobOpt<'a> {
    fn from(v: &'a [u8]) -> Self {
        BlobOpt::Ref(v)
    }
}

impl<'a> From<Vec<u8>> for BlobOpt<'a> {
    fn from(v: Vec<u8>) -> Self {
        BlobOpt::Owned(v)
    }
}

impl<'a> ToRaw for BlobOpt<'a> {
    type Result = RawBlob;
    fn to_raw(&self, _arena: &mut Arena) -> Self::Result {
        match self {
            BlobOpt::Unset => RawBlob::default(),
            BlobOpt::Ref(r) => RawBlob::new(r),
            BlobOpt::Owned(r) => RawBlob::new(r.as_slice()),
        }
    }
}

pub enum ListOpt<'a, T> {
    Unset,
    Ref(&'a [T]),
    Mut(&'a mut [T]),
    Owned(Vec<T>),
}

// Manual impl (see Allocator's Default) to keep the public-API rendering stable.
#[allow(clippy::derivable_impls)]
impl<T> Default for ListOpt<'_, T> {
    fn default() -> Self {
        ListOpt::Unset
    }
}

impl<'a, T> From<&'a [T]> for ListOpt<'a, T> {
    fn from(v: &'a [T]) -> Self {
        ListOpt::Ref(v)
    }
}

impl<'a, T> From<Vec<T>> for ListOpt<'a, T> {
    fn from(v: Vec<T>) -> Self {
        ListOpt::Owned(v)
    }
}

impl<'a, T: ToRaw> ToRaw for ListOpt<'a, T> {
    type Result = RawList<T::Result>;

    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        let items: Vec<T::Result> = match self {
            ListOpt::Unset => return RawList::default(),
            ListOpt::Ref(v) => v.iter().map(|v| T::to_raw(v, arena)).collect(),
            ListOpt::Mut(v) => v.iter().map(|v| T::to_raw(v, arena)).collect(),
            ListOpt::Owned(v) => v.iter().map(|v| T::to_raw(v, arena)).collect(),
        };
        let count = items.len();
        RawList {
            data: arena.push_vec(items),
            count,
        }
    }

    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        let items: Vec<T::Result> = match mem::take(self) {
            ListOpt::Unset => return RawList::default(),
            ListOpt::Ref(v) => v.iter().map(|v| T::to_raw(v, arena)).collect(),
            ListOpt::Mut(v) => v.iter_mut().map(|v| T::to_raw_mut(v, arena)).collect(),
            ListOpt::Owned(v) => v
                .into_iter()
                .map(|mut v| T::to_raw_mut(&mut v, arena))
                .collect(),
        };
        let count = items.len();
        RawList {
            data: arena.push_vec(items),
            count,
        }
    }
}

impl<T: Copy + 'static> ToRaw for T {
    type Result = T;
    fn to_raw(&self, _arena: &mut Arena) -> Self::Result {
        *self
    }
}
