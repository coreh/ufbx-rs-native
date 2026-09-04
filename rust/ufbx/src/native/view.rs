//! Rust-port infrastructure (not a ufbx.c section): a generic reinterpret-in-place
//! *view* over an arena-allocated struct, plus safe iteration over contiguous
//! arena arrays of them.
//!
//! [`View<T>`] is `#[repr(transparent)]` over `UnsafeCell<MaybeUninit<T>>`,
//! reached by casting a `*mut T` that points into a stable arena — the same
//! reinterpret pattern the hand-written `StringView`/`ErrorView`/context wrappers
//! use, factored into one type. A concrete view is a thin alias plus an inherent
//! accessor impl:
//!
//! ```ignore
//! pub(crate) type XmlTagView = View<XmlTag>;
//! impl View<XmlTag> {
//!     pub(crate) fn num_children(&self) -> usize { unsafe { (*self.get()).num_children } }
//! }
//! ```
//!
//! `UnsafeCell` gives interior mutability (shared `&View<T>` may coexist, like
//! `&Context`); `MaybeUninit` drops the whole-value validity requirement so a
//! `&View<T>` can be formed over a `T` embedding not-yet-valid bytes.
//!
//! ufbx stores child/attrib/element/etc. runs as contiguous `push_pop` arena
//! arrays walked in C by `ufbxi_for` (plain `ptr++`). [`Run`] carries such a
//! `(base, count)` pair after one raw construction vouch and provides safe
//! indexing, sub-runs, and iteration; [`SliceViewIter`] is its iterator and the
//! legacy direct-construction surface. The iterator is a dumb contiguous walk —
//! morally `slice::Iter` with a reinterpret on the yield — and knows nothing
//! about the allocator: it is for contiguous `push_pop`-materialized runs ONLY.
//! The allocator's own structures get their own walkers with the same shape —
//! one vouch at construction, safe bodies: `buf::ChunkIter` follows a chunk
//! chain (`->next` / `->prev`, link read before the yield so a body may free
//! the chunk it holds) yielding `buf::ChunkRef` — the header as a view plus
//! the whole-allocation pointer, since a view over a flexible-array-member
//! struct covers the header bytes only — and the map re-hash walks
//! its entry tables as plain slices. Neither is a `T`-array, which is why
//! they are neither `Run` nor `SliceViewIter`.

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

mod sealed {
    pub trait Sealed {}
}

/// Mutability mode of a [`View`]: picks the storage cell, and with it what the
/// Rust abstract machine believes about the viewed bytes.
///
/// - [`Mut`] stores `UnsafeCell<MaybeUninit<T>>`: forming `&View<T, Mut>` retags
///   SharedReadWrite, so it may only be minted from a pointer whose provenance
///   is write-capable (context/arena-owned memory). This is the mode every
///   internal view uses; writes and raw pointers coexist with it.
/// - [`Const`] stores plain `MaybeUninit<T>`: forming `&View<T, Const>` retags
///   SharedReadOnly, so it may be minted from ANY readable pointer — including
///   a `*const T` derived from a public caller's `&T`. In exchange it is a
///   frozen tag: it must not be held across a write to the same bytes through
///   any parent pointer.
///
/// Read accessors written once on `impl<T, M: Mode> View<T, M>` (or per-type
/// `impl<M: Mode> View<Foo, M>`) serve both modes; setters and `get()` live on
/// `Mut` only. Mode propagates through navigation: a chain rooted `Const`
/// stays `Const`.
pub(crate) trait Mode: sealed::Sealed {
    #[doc(hidden)]
    type Storage<T>;
}

/// Interior-mutable view mode (the default; today's `View<T>`).
pub(crate) struct Mut;
/// Read-only view mode, mintable from `&`-derived pointers.
pub(crate) struct Const;

impl sealed::Sealed for Mut {}
impl sealed::Sealed for Const {}
impl Mode for Mut {
    type Storage<T> = UnsafeCell<MaybeUninit<T>>;
}
impl Mode for Const {
    type Storage<T> = MaybeUninit<T>;
}

/// Reinterpret-in-place view over an arena-allocated `T`.
#[repr(transparent)]
pub(crate) struct View<T, M: Mode = Mut>(M::Storage<T>);

impl<T, M: Mode> View<T, M> {
    /// Raw read pointer to the viewed `T` (for mode-generic read accessors).
    /// Reads through it are legal in both modes; never write through it.
    #[inline(always)]
    pub(crate) fn as_ptr(&self) -> *const T {
        self as *const Self as *const T
    }

    /// Mode-generic mint for internal navigation (e.g. following a pointer
    /// STORED in viewed memory, which carries its own stored provenance).
    ///
    /// # Safety
    /// `ptr` must point to a `T` that stays alive and unmoved for `'a`, and its
    /// provenance must be adequate for `M`: write-capable for [`Mut`], readable
    /// for [`Const`]. Prefer the per-mode `from_ptr` at boundaries — this exists
    /// so mode-generic fns can propagate their caller's `M`.
    #[inline(always)]
    pub(crate) unsafe fn mint<'a>(ptr: *mut T) -> &'a Self {
        // SAFETY: caller vouches for liveness and `M`-adequate provenance
        // (fn contract above); `View` is `repr(transparent)` over the storage.
        unsafe { &*(ptr as *const Self) }
    }
}

impl<T> View<T, Mut> {
    /// Raw pointer to the viewed `T` (for accessor impls).
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut T {
        // Field access needs the concrete storage type, which `Mut` fixes.
        let cell: &UnsafeCell<MaybeUninit<T>> = &self.0;
        cell.get().cast()
    }

    /// Replace the whole viewed C-POD value without reading or dropping the
    /// previous bytes. This is the whole-struct counterpart of `view_write!`:
    /// callers can publish coupled fields (for example pointer/length pairs)
    /// in one logical write after constructing and validating the complete
    /// descriptor.
    #[inline(always)]
    pub(crate) fn write_value(&self, value: T) {
        // SAFETY: `get()` is the receiver's live, write-capable storage. The
        // no-Drop requirement is enforced by `write_no_drop`'s const assert.
        unsafe { write_no_drop(self.get(), value) }
    }

    /// The viewed element as a storable `Ref<T>` (the inverse of
    /// [`crate::prelude::Ref::view`]), for writing a `ufbx_element*` field
    /// from a view. `Mut` only — a `Ref` promises write-capable provenance to
    /// every later reader, which a `Const` mint (possibly `&`-derived) cannot
    /// supply.
    ///
    /// # Safety
    /// The view's own contract covers liveness only for the view's lifetime,
    /// and a `Mut` view may legitimately be minted over a stack local. A `Ref`
    /// promises more: the viewed `T` must live in a stable allocation that
    /// stays alive and unmoved for as long as any copy of the `Ref`, or the
    /// struct it is stored into, exists (the [`crate::prelude::Ref::from_ptr`]
    /// contract) — i.e. the receiver must be minted over arena/result-buffer
    /// (or externally retained) storage, never a stack local.
    #[inline(always)]
    pub(crate) unsafe fn to_ref(&self) -> crate::prelude::Ref<T> {
        // SAFETY: a `Mut` view is minted over a live `T` with write-capable
        // provenance (its `from_ptr`/`mint` contract), so `get()` is non-null;
        // the storage-lifetime half of the `Ref::from_ptr` contract is the
        // caller's (fn contract above).
        unsafe { crate::prelude::Ref::from_ptr(self.get()) }
    }

    /// Reinterpret a raw arena pointer as an interior-mutable view reference.
    ///
    /// # Safety
    /// `ptr` must point to a `T`-sized, `T`-aligned slot that stays alive and
    /// unmoved for `'a` (the arena-stability invariant). The slot's bytes need
    /// NOT be initialized — the storage is `MaybeUninit`, so the mint asserts no
    /// whole-value validity (module invariant above) and a freshly pushed arena
    /// run may be minted before anything is written into it; it is each READ
    /// accessor that asserts the field it touches is initialized. The
    /// PROVENANCE must be
    /// write-capable: it must trace to context/arena-owned memory via `*mut`
    /// (or FFI ingress), NEVER to a `&T` — forming `&View<T, Mut>` (an
    /// `&UnsafeCell`) retags SharedReadWrite, which is UB over a read-only
    /// parent even if nothing is ever written. For `&`-derived pointers use
    /// `View<T, Const>::from_ptr`. Per `UnsafeCell`, no `&mut` to the same `T`
    /// may be active while the returned view is used.
    #[inline(always)]
    pub(crate) unsafe fn from_ptr<'a>(ptr: *mut T) -> &'a Self {
        // SAFETY: caller vouches for liveness and write-capable provenance
        // (fn contract above); `View` is `repr(transparent)` over the storage.
        unsafe { &*(ptr as *const Self) }
    }

    /// View an exclusively borrowed `T` (a stack local, a test-owned struct)
    /// in place, for the borrow's lifetime. Safe: the `&mut` supplies liveness
    /// and write-capable provenance, and its exclusivity guarantees no other
    /// reference to the `T` is active while the view exists — the borrow
    /// checker enforces the `UnsafeCell` no-concurrent-`&mut` rule for us.
    /// This is the caller-side form for `String`/`Blob`/`Buf`-typed locals
    /// passed to fns that take a `&View<T, Mut>` place.
    #[inline(always)]
    pub(crate) fn from_mut(value: &mut T) -> &Self {
        // SAFETY: `value` is a live, aligned `T` for the returned lifetime with
        // Unique (write-capable) provenance; the retag to SharedReadWrite via
        // `&UnsafeCell` is a legal child of it, and the `&mut` itself is
        // reborrowed away for as long as the view lives.
        unsafe { &*(value as *mut T as *const Self) }
    }
}

impl<T> View<T, Const> {
    /// Reinterpret a raw pointer as a read-only view reference. Legal for any
    /// readable provenance, including a `*const T` cast from a caller's `&T`.
    ///
    /// # Safety
    /// `ptr` must point to a `T` that stays alive and unmoved for `'a`, and the
    /// viewed bytes must not be written through any parent pointer while the
    /// returned view (or anything derived from it) is live — this is a frozen
    /// (SharedReadOnly) tag.
    #[inline(always)]
    pub(crate) unsafe fn from_ptr<'a>(ptr: *const T) -> &'a Self {
        // SAFETY: caller vouches for liveness and the no-parent-writes freeze
        // (fn contract above); `View` is `repr(transparent)` over the storage.
        unsafe { &*(ptr as *const Self) }
    }

    /// View a shared-borrowed `T` in place, for the borrow's lifetime. Safe:
    /// the `&T` supplies liveness and readable provenance, and Rust's shared
    /// borrow already freezes the bytes for that lifetime (`T` is a C POD
    /// without interior mutability), which is exactly the `Const` contract.
    #[inline(always)]
    pub(crate) fn from_ref(value: &T) -> &Self {
        // SAFETY: `value` is a live, aligned `T` frozen for the returned
        // lifetime by the shared borrow; `View<T, Const>` is
        // `repr(transparent)` over `MaybeUninit<T>` (no `UnsafeCell`), so the
        // retag is SharedReadOnly under a SharedReadOnly parent.
        unsafe { &*(value as *const T as *const Self) }
    }
}

// Per-leaf accessor-body macros: the ONE safety argument every single-level
// field accessor on a view/context handle shares, written once here instead of
// repeated in ~1000 method bodies (user-directed unification, 2026-08-23).
//
// SAFETY argument, common to every expansion: the receiver is a view or
// context handle whose `get()` / `as_ptr()` yields a raw pointer to a live,
// unmoved allocation (its mint/construction invariant); the expansion projects
// and touches exactly ONE leaf field as a raw place — no `&`/`&mut` to the
// containing struct is ever formed and no whole-struct validity is asserted.
// A read asserts only that this field's bytes are initialized (the per-leaf
// discipline: the same assertion the hand-written body made); a write goes
// through `get()`, which only `Mut`-mode views and context handles expose, so
// write capability is type-checked. `&raw` projections compute an address with
// the handle's provenance and assert nothing further.
//
// These are for ACCESSOR IMPL BODIES — a single-level `$field` only. Deeper
// paths, stored-pointer derefs, and anything whose vouch exceeds the handle
// invariant stay as explicit `unsafe` with their own SAFETY comments.
//
// Reads expand to `ptr::read`, which for a non-`Copy` field produces a BITWISE
// DUPLICATE. That is sound here only because the viewed data model is C PODs
// with no `Drop` — bitwise duplication is the ported C struct-assignment
// semantics (PORTING checklist #15; the same rule that makes `Ref<T>` `Copy`).
// If a droppable type ever ends up behind a view, its reads must not use
// these macros.

/// Drop-glue guard for the leaf macros: `ptr::read` duplicates and `ptr::write`
/// discards bitwise, which is only sound for the Drop-free C-POD model — this
/// turns a future droppable-type-behind-a-view into a COMPILE ERROR at the
/// offending accessor (post-monomorphization const assert, zero runtime cost).
///
/// # Safety
/// As `ptr::read`: `p` must be valid for reads of an initialized `T`.
#[inline(always)]
pub(crate) unsafe fn read_no_drop<T>(p: *const T) -> T {
    const {
        assert!(
            !core::mem::needs_drop::<T>(),
            "leaf-macro read of a Drop type: bitwise duplication would double-drop              (see the view.rs macro doc)"
        )
    };
    // SAFETY: forwarded unchanged from this fn's own contract.
    unsafe { core::ptr::read(p) }
}

/// Write half of the guard (see [`read_no_drop`]): `ptr::write` never runs
/// `Drop` on the old value, which under the assert is the only inhabited case
/// anyway — and a place assignment WOULD drop stale bytes if `T` gained glue.
///
/// # Safety
/// As `ptr::write`: `p` must be valid for writes of `T`.
#[inline(always)]
pub(crate) unsafe fn write_no_drop<T>(p: *mut T, value: T) {
    const {
        assert!(
            !core::mem::needs_drop::<T>(),
            "leaf-macro write of a Drop type: the discarded old value would need              dropping (see the view.rs macro doc)"
        )
    };
    // SAFETY: forwarded unchanged from this fn's own contract.
    unsafe { core::ptr::write(p, value) }
}

/// Single-leaf read through `$self.get()` (Mut views / context handles).
macro_rules! view_read {
    ($self:expr, $field:ident) => {
        // SAFETY: single-leaf read; see the macro-level argument above.
        unsafe { $crate::native::view::read_no_drop(&raw const (*$self.get()).$field) }
    };
}
pub(crate) use view_read;

/// Single-leaf read through `$self.as_ptr()` (mode-generic impls).
macro_rules! view_read_shared {
    ($self:expr, $field:ident) => {
        // SAFETY: single-leaf read; see the macro-level argument above.
        unsafe { $crate::native::view::read_no_drop(&raw const (*$self.as_ptr()).$field) }
    };
}
pub(crate) use view_read_shared;

/// Single-leaf `&raw const` projection through `$self.as_ptr()` (mode-generic
/// impls; the read pointer inherits the view's provenance).
macro_rules! view_raw_shared {
    ($self:expr, $field:ident) => {
        // SAFETY: single-leaf address projection; see the macro-level argument
        // above.
        unsafe { &raw const (*$self.as_ptr()).$field }
    };
}
pub(crate) use view_raw_shared;

/// In-place nested-view projection through `$self.as_ptr()`: mints a view over
/// one embedded field. Liveness and `M`-adequate provenance carry over from
/// the receiver's own mint (the projection stays inside the same allocation).
macro_rules! view_project {
    ($self:expr, $field:ident) => {
        // SAFETY: in-place single-field projection; see the macro doc above.
        unsafe {
            $crate::native::view::View::mint((&raw const (*$self.as_ptr()).$field).cast_mut())
        }
    };
}
pub(crate) use view_project;

/// Single-leaf place write through `$self.get()` (write capability is carried
/// by `get()` existing on the receiver).
macro_rules! view_write {
    ($self:expr, $field:ident, $value:expr) => {
        // SAFETY: single-leaf place write; see the macro-level argument above.
        unsafe {
            $crate::native::view::write_no_drop(&raw mut (*$self.get()).$field, $value);
        }
    };
}
pub(crate) use view_write;

/// Single-leaf `&raw mut` projection through `$self.get()`.
macro_rules! view_raw_mut {
    ($self:expr, $field:ident) => {
        // SAFETY: single-leaf address projection; see the macro-level argument
        // above.
        unsafe { &raw mut (*$self.get()).$field }
    };
}
pub(crate) use view_raw_mut;

/// Single-leaf `&raw const` projection (either pointer getter).
macro_rules! view_raw_const {
    ($self:expr, $field:ident) => {
        // SAFETY: single-leaf address projection; see the macro-level argument
        // above.
        unsafe { &raw const (*$self.get()).$field }
    };
}
pub(crate) use view_raw_const;

/// Borrowed handle over a contiguous run of `T` slots.
///
/// This is the typed carrier for C's recurring `(data, count)` pair. Its raw
/// constructor is the single vouch for the whole run; bounds-checked element
/// access, iteration, and sub-runs are safe. Like [`View`], the mode records
/// whether the pointer's provenance is write-capable ([`Mut`]) or merely
/// readable and frozen ([`Const`]).
// Some Run consumers are feature-gated; reduced configurations retain the
// shared carrier and operations used by their enabled subsystems.
#[cfg_attr(not(feature = "baking"), allow(dead_code))]
pub(crate) struct Run<'a, T, M: Mode = Mut> {
    base: *mut T,
    count: usize,
    _marker: PhantomData<&'a View<T, M>>,
}

impl<T, M: Mode> Copy for Run<'_, T, M> {}

impl<T, M: Mode> Clone for Run<'_, T, M> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

#[cfg_attr(not(feature = "baking"), allow(dead_code))]
impl<'a, T, M: Mode> Run<'a, T, M> {
    /// Derive a run from a viewed `List<T>` field.
    ///
    /// Safe because the viewed-list invariant already vouches that its stored
    /// `(data, count)` pair describes a contiguous stable run, and the view's
    /// mode records whether that stored pointer may be used for writes.
    #[inline(always)]
    pub(crate) fn from_list(list: &'a View<crate::prelude::List<T>, M>) -> Self {
        Self {
            base: list.data() as *mut T,
            count: list.count(),
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) fn len(self) -> usize {
        self.count
    }

    /// Raw read pointer to the first slot, preserving the run's original
    /// allocation provenance. An empty run may retain a null base or a legal
    /// one-past pointer from its parent run.
    #[cfg_attr(not(feature = "subdivision"), allow(dead_code))]
    #[inline(always)]
    pub(crate) fn as_ptr(self) -> *const T {
        self.base
    }

    /// Bounds-checked element view carrying the run's mode and lifetime.
    #[inline(always)]
    pub(crate) fn at(self, index: usize) -> &'a View<T, M> {
        assert!(index < self.count);
        // SAFETY: the constructor vouches for `count` contiguous slots and the
        // assertion keeps `index` in that run. Its provenance is adequate for
        // `M`, and the slot stays alive and unmoved for `'a`.
        unsafe { View::mint(self.base.add(index)) }
    }

    /// Bounds-checked contiguous sub-run.
    #[inline(always)]
    pub(crate) fn subrun(self, begin: usize, count: usize) -> Self {
        assert!(begin <= self.count);
        assert!(count <= self.count - begin);
        // Preserve null-with-zero without performing pointer arithmetic. For
        // a non-empty parent the constructor vouches for the allocation, so
        // `begin <= self.count` permits an in-bounds or one-past projection.
        let base = if begin == 0 {
            self.base
        } else {
            // SAFETY: checked above; `begin` is within or one past the run.
            unsafe { self.base.add(begin) }
        };
        Self {
            base,
            count,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) fn iter(self) -> SliceViewIter<'a, T, M> {
        SliceViewIter {
            base: self.base,
            count: self.count,
            idx: 0,
            _marker: PhantomData,
        }
    }
}

#[cfg_attr(not(feature = "baking"), allow(dead_code))]
impl<'a, T> Run<'a, T, Mut> {
    /// Borrow an exclusively held slice as a bounded interior-mutable run.
    ///
    /// The returned capability keeps the exclusive slice borrow active for
    /// `'a`; individual slots may then be initialized or accessed through the
    /// same `View<T, Mut>` surface as raw-backed native runs.
    #[cfg_attr(not(feature = "triangulation"), allow(dead_code))]
    #[inline(always)]
    pub(crate) fn from_mut_slice(slice: &'a mut [T]) -> Self {
        Self {
            base: slice.as_mut_ptr(),
            count: slice.len(),
            _marker: PhantomData,
        }
    }

    /// Vouch for one contiguous interior-mutable run.
    ///
    /// # Safety
    /// `base` must point to `count` contiguous, allocated, write-capable `T`
    /// slots that stay alive and unmoved for `'a`. The slots may be
    /// uninitialized: reads retain the per-leaf initialization obligation of
    /// [`View<T, Mut>`]. Null is allowed exactly when `count == 0`.
    #[inline(always)]
    pub(crate) unsafe fn from_raw_parts(base: *mut T, count: usize) -> Self {
        debug_assert!(count == 0 || !base.is_null());
        Self {
            base,
            count,
            _marker: PhantomData,
        }
    }

    /// Empty run with no allocation or provenance requirement.
    #[inline(always)]
    pub(crate) const fn empty() -> Run<'static, T, Mut> {
        Run {
            base: core::ptr::null_mut(),
            count: 0,
            _marker: PhantomData,
        }
    }

    /// Raw write pointer to the first slot. For an empty run this may be null.
    #[inline(always)]
    pub(crate) fn as_mut_ptr(self) -> *mut T {
        self.base
    }

    /// Bounds-checked initialization or replacement of one slot.
    ///
    /// This intentionally has `ptr::write` semantics: an existing value is not
    /// dropped. Native runs contain C-compatible arena values, and fresh arena
    /// pushes may still be uninitialized when the run is minted.
    #[cfg_attr(not(feature = "subdivision"), allow(dead_code))]
    #[inline(always)]
    pub(crate) fn write_at(self, index: usize, value: T) {
        assert!(index < self.count);
        // SAFETY: the constructor vouches for `count` contiguous allocated,
        // write-capable slots, and the assertion keeps this write in the run.
        unsafe { self.base.add(index).write(value) };
    }
}

impl<'a, T> Run<'a, T, Const> {
    /// Borrow a shared slice as a bounded initialized read-only run.
    ///
    /// The returned capability keeps the shared slice borrow active for `'a`;
    /// its elements stay readable and frozen for the run's lifetime.
    #[inline(always)]
    pub(crate) fn from_slice(slice: &'a [T]) -> Self {
        Self {
            base: slice.as_ptr().cast_mut(),
            count: slice.len(),
            _marker: PhantomData,
        }
    }

    /// Vouch for one contiguous initialized read-only run.
    ///
    /// # Safety
    /// `base` must point to `count` contiguous, initialized, readable `T`
    /// elements that stay alive, unmoved, and frozen for `'a`. Null is allowed
    /// exactly when `count == 0`.
    #[inline(always)]
    pub(crate) unsafe fn from_const_raw_parts(base: *const T, count: usize) -> Self {
        debug_assert!(count == 0 || !base.is_null());
        Self {
            base: base.cast_mut(),
            count,
            _marker: PhantomData,
        }
    }

    /// Bounds-checked value read from an initialized read-only run.
    #[cfg_attr(not(feature = "subdivision"), allow(dead_code))]
    #[inline(always)]
    pub(crate) fn copy_at(self, index: usize) -> T
    where
        T: Copy,
    {
        assert!(index < self.count);
        // SAFETY: the constructor vouches that every slot is initialized and
        // readable, and the assertion keeps this read in the run.
        unsafe { *self.base.add(index) }
    }
}

/// Safe iterator over a contiguous run of `T`, yielding `&View<T, M>`.
///
/// A dumb contiguous walk — `slice::Iter` with a reinterpret on the yield — that
/// knows nothing about the allocator. Construction (`from_raw_parts`) is the
/// single `unsafe` boundary that vouches for the run; iteration is then fully safe.
pub(crate) struct SliceViewIter<'a, T, M: Mode = Mut> {
    base: *mut T,
    count: usize,
    idx: usize,
    _marker: PhantomData<&'a View<T, M>>,
}

impl<'a, T> SliceViewIter<'a, T, Mut> {
    /// # Safety
    /// `base` must point to `count` contiguous, allocated, write-capable `T`
    /// slots that stay alive and unmoved for `'a` — one arena allocation run
    /// (e.g. `tag->children` / `tag->num_children`, or a freshly pushed run).
    /// The run must be genuinely contiguous (`push_pop`-materialized), not
    /// skip-flagged. The slots need NOT hold initialized `T`: the yielded
    /// `View<T, Mut>` stores `UnsafeCell<MaybeUninit<T>>`, so an
    /// `ufbxi_push`-fresh, still-uninitialized run is a legal vouch — the
    /// caller owes initialization before any read through `get()`. When
    /// `count == 0` `base` is never offset or dereferenced, so null-with-zero
    /// is allowed (unlike `slice::from_raw_parts`).
    #[inline]
    pub(crate) unsafe fn from_raw_parts(base: *mut T, count: usize) -> Self {
        Self {
            base,
            count,
            idx: 0,
            _marker: PhantomData,
        }
    }
}

impl<'a, T, M: Mode> Iterator for SliceViewIter<'a, T, M> {
    type Item = &'a View<T, M>;

    #[inline]
    fn next(&mut self) -> Option<&'a View<T, M>> {
        if self.idx >= self.count {
            return None;
        }
        // SAFETY: `idx < count`, so `base + idx` is in-bounds of the run vouched
        // at construction (`from_raw_parts`, or the `Run` this iterator was
        // derived from); that slot is allocated and stable for 'a, with
        // provenance adequate for `M` — write-capable for `Mut`, whose
        // `MaybeUninit` storage also tolerates a still-uninitialized slot.
        let elem = unsafe { self.base.add(self.idx) };
        self.idx += 1;
        Some(unsafe { View::<T, M>::mint(elem) })
    }
}
