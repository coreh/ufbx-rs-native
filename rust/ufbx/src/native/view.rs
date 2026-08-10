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
//! arrays walked in C by `ufbxi_for` (plain `ptr++`); [`ArenaViewIter`] walks
//! such a `(base, count)` run yielding `&View<T>`, with the only raw-pointer work
//! — the in-bounds index and the per-element `*mut T -> &View<T>` bridge —
//! localized to `new` (the arena vouch) and `next`. It is for contiguous runs
//! ONLY; skip-flagged / free-list structures (maps, retired chunks) need a
//! filtering iterator or stay raw.

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

/// Reinterpret-in-place view over an arena-allocated `T`.
#[repr(transparent)]
pub(crate) struct View<T>(UnsafeCell<MaybeUninit<T>>);

impl<T> View<T> {
    /// Raw pointer to the viewed `T` (for accessor impls).
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut T {
        self.0.get().cast()
    }

    /// Reinterpret a raw arena pointer as a view reference.
    ///
    /// # Safety
    /// `ptr` must point to a valid, initialized `T` that stays alive and unmoved
    /// for `'a` (the arena-stability invariant) — and, per `UnsafeCell`, no `&mut`
    /// to the same `T` may be active while the returned `&View<T>` is used.
    #[inline(always)]
    pub(crate) unsafe fn from_ptr<'a>(ptr: *mut T) -> &'a Self {
        &*(ptr as *const View<T>)
    }
}

/// Safe iterator over a contiguous arena run of `T`, yielding `&View<T>`.
///
/// Construction (`new`) is the single `unsafe` boundary that vouches for the run;
/// iteration is then fully safe.
pub(crate) struct ArenaViewIter<'a, T> {
    base: *mut T,
    count: usize,
    idx: usize,
    _marker: PhantomData<&'a View<T>>,
}

impl<'a, T> ArenaViewIter<'a, T> {
    /// # Safety
    /// `base` must point to `count` contiguous, valid, initialized `T` that stay
    /// alive and unmoved for `'a` — one arena allocation run (e.g.
    /// `tag->children` / `tag->num_children`). The run must be genuinely
    /// contiguous (`push_pop`-materialized), not skip-flagged.
    #[inline]
    pub(crate) unsafe fn new(base: *mut T, count: usize) -> Self {
        Self {
            base,
            count,
            idx: 0,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> Iterator for ArenaViewIter<'a, T> {
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
