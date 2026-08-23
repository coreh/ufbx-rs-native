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
//! arrays walked in C by `ufbxi_for` (plain `ptr++`); [`SliceViewIter`] walks
//! such a `(base, count)` run yielding `&View<T>`, with the only raw-pointer work
//! — the in-bounds index and the per-element `*mut T -> &View<T>` bridge —
//! localized to `from_raw_parts` (the run vouch) and `next`. It is a dumb
//! contiguous walk —
//! morally `slice::Iter` with a reinterpret on the yield — and knows nothing
//! about the allocator: it is for contiguous `push_pop`-materialized runs ONLY.
//! Skip-flagged / free-list structures (maps, retired chunks) need an
//! allocator-aware `ArenaViewIter` (not yet built) or stay raw.

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

    /// Reinterpret a raw arena pointer as an interior-mutable view reference.
    ///
    /// # Safety
    /// `ptr` must point to a valid, initialized `T` that stays alive and unmoved
    /// for `'a` (the arena-stability invariant), and its PROVENANCE must be
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

/// Single-leaf place read through `$self.get()` (Mut views / context handles).
macro_rules! view_read {
    ($self:expr, $field:ident) => {
        // SAFETY: single-leaf place read; see the macro-level argument above.
        unsafe { (*$self.get()).$field }
    };
}
pub(crate) use view_read;

/// Single-leaf place read through `$self.as_ptr()` (mode-generic impls).
macro_rules! view_read_shared {
    ($self:expr, $field:ident) => {
        // SAFETY: single-leaf place read; see the macro-level argument above.
        unsafe { (*$self.as_ptr()).$field }
    };
}
pub(crate) use view_read_shared;

/// Single-leaf place write through `$self.get()` (write capability is carried
/// by `get()` existing on the receiver).
macro_rules! view_write {
    ($self:expr, $field:ident, $value:expr) => {
        // SAFETY: single-leaf place write; see the macro-level argument above.
        unsafe {
            (*$self.get()).$field = $value;
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

/// Safe iterator over a contiguous run of `T`, yielding `&View<T>`.
///
/// A dumb contiguous walk — `slice::Iter` with a reinterpret on the yield — that
/// knows nothing about the allocator. Construction (`from_raw_parts`) is the
/// single `unsafe` boundary that vouches for the run; iteration is then fully safe.
pub(crate) struct SliceViewIter<'a, T> {
    base: *mut T,
    count: usize,
    idx: usize,
    _marker: PhantomData<&'a View<T>>,
}

impl<'a, T> SliceViewIter<'a, T> {
    /// # Safety
    /// `base` must point to `count` contiguous, valid, initialized `T` that stay
    /// alive and unmoved for `'a` — one arena allocation run (e.g.
    /// `tag->children` / `tag->num_children`). The run must be genuinely
    /// contiguous (`push_pop`-materialized), not skip-flagged. When `count == 0`
    /// `base` is never offset or dereferenced, so null-with-zero is allowed
    /// (unlike `slice::from_raw_parts`).
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

impl<'a, T> Iterator for SliceViewIter<'a, T> {
    type Item = &'a View<T>;

    #[inline]
    fn next(&mut self) -> Option<&'a View<T>> {
        if self.idx >= self.count {
            return None;
        }
        // SAFETY: `idx < count`, so `base + idx` is in-bounds of the run vouched
        // at `new`; that element is a valid/initialized/stable `T` for 'a.
        let elem = unsafe { self.base.add(self.idx) };
        self.idx += 1;
        Some(unsafe { View::<T>::from_ptr(elem) })
    }
}
