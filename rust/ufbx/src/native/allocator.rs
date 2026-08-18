//! Port of the `// -- Allocator` banner section (ufbx.c:3614-3815), plus its
//! two satellites:
//! - the default global allocator `ufbx_malloc`/`ufbx_realloc`/`ufbx_free`
//!   (ufbx.c:370-386 — macros over libc in the default build; the
//!   `UFBX_NO_MALLOC` / `UFBX_EXTERNAL_MALLOC` / user-macro forks are not
//!   exposed as cargo features),
//! - `ufbxi_init_ator` (ufbx.c:6936-6953 — C defines it next to
//!   `ufbxi_context`, but it is pure allocator setup and every non-context
//!   user (`ufbxi_begin_file_context`, caches, baking) reaches it through
//!   this module).
//!
//! See PORTING.md "Allocator + ufbxi_buf": exact size/limit accounting, exact
//! allocation sequence (fuzz-observable), raw pointers throughout.
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]

use core::ffi::c_void;
use core::mem::size_of;

use crate::generated::{Error, RawAllocatorOpts};
use crate::native::error::{
    ufbxi_check_return_err, ufbxi_check_return_err_msg, ufbxi_fmt_err_info, ufbxi_report_err_msg,
};
use crate::native::platform::{
    is_aligned_mask, max_sz, ufbx_assert, ufbxi_maybe_null, MAXIMUM_ALIGNMENT,
};

// -- Default global allocator (ufbx.c:370-386)
//
// C: `#define ufbx_malloc(size) malloc((size))` etc. (ufbx.c:381-386, the
// default branch). Routed through libc so allocation behavior (and interop
// with a C-side `free`) matches the C build exactly. The names keep the
// `ufbx_` prefix because C treats them as (potentially user-overridable)
// global allocator entry points, not `ufbxi_` internals.
mod libc_alloc {
    use core::ffi::c_void;
    extern "C" {
        pub(super) fn malloc(size: usize) -> *mut c_void;
        pub(super) fn realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void;
        pub(super) fn free(ptr: *mut c_void);
    }
}

// ufbx.c:381 `#define ufbx_malloc(size) malloc((size))`
// No raw-pointer parameters to carry an obligation from the caller; the only
// unsafe operation is the FFI call itself, isolated below.
#[inline(always)]
pub(crate) fn ufbx_malloc(size: usize) -> *mut c_void {
    unsafe { libc_alloc::malloc(size) }
}

// ufbx.c:382 `#define ufbx_realloc(ptr, old_size, new_size) realloc((ptr), (new_size))`
#[inline(always)]
pub(crate) unsafe fn ufbx_realloc(
    ptr: *mut c_void,
    old_size: usize,
    new_size: usize,
) -> *mut c_void {
    let _ = old_size; // C macro discards `old_size`
    libc_alloc::realloc(ptr, new_size)
}

// ufbx.c:383 `#define ufbx_free(ptr, old_size) free((ptr))`
#[inline(always)]
pub(crate) unsafe fn ufbx_free(ptr: *mut c_void, old_size: usize) {
    let _ = old_size; // C macro discards `old_size`
    libc_alloc::free(ptr)
}

// -- Allocator

// ufbx.c:3616-3622
// C comment: Returned for zero size allocations, place in the constant data
// to catch writes to bad allocations.
// C-parity: C returns `(void*)ufbxi_zero_size_buffer` — a const object with
// the constness cast away; writing through the pointer is UB in both trees.
// C-parity (alignment): the C array is `char[]` (align 1) but is handed out
// as the zero-size allocation for EVERY element type; C only ever performs
// zero-count `memset`/`memcpy` through the pointer, which have no alignment
// demand in practice. Rust's `ptr::write_bytes::<T>`/`copy_nonoverlapping::<T>`
// carry an alignment precondition even for count 0 (debug builds abort on it),
// so the static is aligned to `UFBX_MAXIMUM_ALIGNMENT` (ufbx.c:858-860) — the
// strongest alignment the allocator ever provides. Observable behavior is
// unchanged: no bytes are read or written either way.
#[repr(C, align(8))]
pub(crate) struct ZeroSizeBuffer<const N: usize>(pub(crate) [u8; N]);

impl<const N: usize> ZeroSizeBuffer<N> {
    #[inline(always)]
    pub(crate) const fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }
}

// `#[repr(align(8))]` takes a literal; pin it against `MAXIMUM_ALIGNMENT`.
const _: () = assert!(core::mem::align_of::<ZeroSizeBuffer<64>>() >= MAXIMUM_ALIGNMENT);

#[cfg(feature = "regression")]
pub(crate) static ZERO_SIZE_BUFFER: ZeroSizeBuffer<4096> = ZeroSizeBuffer([0; 4096]);
#[cfg(not(feature = "regression"))]
pub(crate) static ZERO_SIZE_BUFFER: ZeroSizeBuffer<64> = ZeroSizeBuffer([0; 64]);

// ufbx.c:3624-3627 `ufbxi_align_to_mask`
#[inline(always)]
pub(crate) fn align_to_mask(value: usize, align_mask: usize) -> usize {
    // C: `value + (((size_t)0 - value) & align_mask)` — unsigned wrap.
    value.wrapping_add(0usize.wrapping_sub(value) & align_mask)
}

// ufbx.c:3629-3633 `ufbxi_size_align_mask`
#[inline(always)]
pub(crate) fn size_align_mask(size: usize) -> usize {
    // Align to the all bits below the lowest set one in `size` up to the maximum alignment.
    ((size ^ size.wrapping_sub(1)) >> 1) & (MAXIMUM_ALIGNMENT - 1)
}

// ufbx.c:3635-3645 `ufbxi_allocator`
// `Clone, Copy` so C by-value copies (e.g. `ufbxi_release_ref`,
// ufbx.c:30289-30297) stay memcpy-like `=` (PORTING.md checklist #15).
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Allocator {
    pub error: *mut Error,
    pub current_size: usize,
    pub max_size: usize,
    pub num_allocs: usize,
    pub max_allocs: usize,
    pub huge_size: usize,
    pub chunk_max: usize,
    pub ator: RawAllocatorOpts,
    pub name: *const u8,
}

// Typed interior-mutable VIEW over an OWNED `Allocator` field, reinterpreted in
// place. Applies only to owned `Allocator` fields (`ator`/`ator_tmp`/`ator_result`);
// pointer-to-allocator fields (`*mut Allocator`) are reached via their value getter
// + deref instead. Getters read POD bookkeeping leaves; `set_error` wires the error
// slot. The C-ABI `alloc()`/`free()` boundary keeps using the raw `*mut` getter.
pub(crate) type AllocatorView = crate::native::view::View<Allocator>;

impl AllocatorView {
    #[inline(always)]
    pub(crate) fn current_size(&self) -> usize {
        unsafe { (*self.get()).current_size }
    }
    #[inline(always)]
    pub(crate) fn num_allocs(&self) -> usize {
        unsafe { (*self.get()).num_allocs }
    }
    #[inline(always)]
    pub(crate) fn set_error(&self, error: *mut Error) {
        unsafe {
            (*self.get()).error = error;
        }
    }
}

// ufbx.c:3647-3653 `ufbxi_does_overflow`
#[inline(always)]
pub(crate) fn does_overflow(total: usize, a: usize, b: usize) -> bool {
    // If `a` and `b` have at most 4 bits per `size_t` byte, the product can't overflow.
    if ((a | b) >> (size_of::<usize>() * 4)) != 0 {
        if a != 0 && total / a != b {
            return true;
        }
    }
    false
}

// ufbx.c:3656-3696 `ufbxi_alloc_size`
#[inline(never)]
pub(crate) unsafe fn alloc_size(ator: *mut Allocator, size: usize, n: usize) -> *mut c_void {
    // Always succeed with an empty non-NULL buffer for empty allocations
    ufbx_assert!(size > 0);
    if n == 0 {
        return ZERO_SIZE_BUFFER.as_ptr() as *mut c_void;
    }

    let total = size.wrapping_mul(n);
    ufbxi_check_return_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*ator).error) },
        !does_overflow(total, size, n),
        core::ptr::null_mut(),
        "!ufbxi_does_overflow(total, size, n)"
    );
    // Make sure it's always safe to double allocations
    ufbxi_check_return_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*ator).error) },
        total <= usize::MAX / 2,
        core::ptr::null_mut(),
        "total <= SIZE_MAX / 2"
    );
    if !(total < (*ator).max_size - (*ator).current_size) {
        let a = &mut *ator;
        ufbxi_report_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(a.error) },
            "total <= ator->max_size - ator->current_size",
            "Memory limit exceeded"
        );
        ufbxi_fmt_err_info!(a.error, "%s", a.name);
        return core::ptr::null_mut();
    }
    if !((*ator).num_allocs < (*ator).max_allocs) {
        let a = &mut *ator;
        ufbxi_report_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(a.error) },
            "ator->num_allocs < ator->max_allocs",
            "Allocation limit exceeded"
        );
        ufbxi_fmt_err_info!(a.error, "%s", a.name);
        return core::ptr::null_mut();
    }
    (*ator).num_allocs += 1;

    let ptr: *mut c_void;
    if let Some(alloc_fn) = (*ator).ator.allocator.alloc_fn {
        ptr = alloc_fn((*ator).ator.allocator.user, total);
    } else if let Some(realloc_fn) = (*ator).ator.allocator.realloc_fn {
        ptr = realloc_fn((*ator).ator.allocator.user, core::ptr::null_mut(), 0, total);
    } else {
        ptr = ufbx_malloc(total);
    }

    if ptr.is_null() {
        let a = &mut *ator;
        ufbxi_report_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(a.error) },
            "ptr",
            "Out of memory"
        );
        ufbxi_fmt_err_info!(a.error, "%s", a.name);
        return core::ptr::null_mut();
    }
    ufbx_assert!(is_aligned_mask(ptr, size_align_mask(total)));

    // Expose the allocation's provenance so address-based widening
    // (`with_exposed_provenance`, e.g. the `as_*` element downcasts and the
    // `get_imp` container-of) is legal from any pointer into it — the
    // arena-wide generalization of the per-`*Imp` G-class expose. Runtime
    // no-op; Miri flags it only under `-Zmiri-strict-provenance`, which the
    // CI gate deliberately leaves off.
    (ptr as *mut u8).expose_provenance();

    (*ator).current_size += total;

    ptr
}

// ufbx.c:3699-3740 `ufbxi_realloc_size`
// (C forward-declares `ufbxi_free_size` at 3698; no Rust analogue needed.)
#[inline(never)]
pub(crate) unsafe fn realloc_size(
    ator: *mut Allocator,
    size: usize,
    old_ptr: *mut c_void,
    old_n: usize,
    n: usize,
) -> *mut c_void {
    ufbx_assert!(size > 0);
    // realloc() with zero old/new size is equivalent to alloc()/free()
    if old_n == 0 {
        return alloc_size(ator, size, n);
    }
    if n == 0 {
        free_size(ator, size, old_ptr, old_n);
        return core::ptr::null_mut();
    }

    let old_total = size.wrapping_mul(old_n);
    let total = size.wrapping_mul(n);

    // The old values have been checked by a previous allocate call
    ufbx_assert!(!does_overflow(old_total, size, old_n));
    ufbx_assert!(old_total <= (*ator).current_size);

    ufbxi_check_return_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*ator).error) },
        !does_overflow(total, size, n),
        core::ptr::null_mut(),
        "!ufbxi_does_overflow(total, size, n)"
    );
    // Make sure it's always safe to double allocations
    ufbxi_check_return_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*ator).error) },
        total <= usize::MAX / 2,
        core::ptr::null_mut(),
        "total <= SIZE_MAX / 2"
    );
    ufbxi_check_return_err_msg!(
        unsafe { crate::native::error::ErrorView::from_ptr((*ator).error) },
        total <= (*ator).max_size - (*ator).current_size,
        core::ptr::null_mut(),
        "Memory limit exceeded",
        "total <= ator->max_size - ator->current_size"
    );
    ufbxi_check_return_err_msg!(
        unsafe { crate::native::error::ErrorView::from_ptr((*ator).error) },
        (*ator).num_allocs < (*ator).max_allocs,
        core::ptr::null_mut(),
        "Allocation limit exceeded",
        "ator->num_allocs < ator->max_allocs"
    );
    (*ator).num_allocs += 1;

    let ptr: *mut c_void;
    if let Some(realloc_fn) = (*ator).ator.allocator.realloc_fn {
        ptr = realloc_fn((*ator).ator.allocator.user, old_ptr, old_total, total);
    } else if let Some(alloc_fn) = (*ator).ator.allocator.alloc_fn {
        // Use user-provided alloc_fn() and free_fn()
        ptr = alloc_fn((*ator).ator.allocator.user, total);
        if !ptr.is_null() {
            core::ptr::copy_nonoverlapping(old_ptr as *const u8, ptr as *mut u8, old_total);
        }
        if let Some(free_fn) = (*ator).ator.allocator.free_fn {
            free_fn((*ator).ator.allocator.user, old_ptr, old_total);
        }
    } else {
        ptr = ufbx_realloc(old_ptr, old_total, total);
    }

    ufbxi_check_return_err_msg!(
        unsafe { crate::native::error::ErrorView::from_ptr((*ator).error) },
        !ptr.is_null(),
        core::ptr::null_mut(),
        "Out of memory",
        "ptr"
    );
    ufbx_assert!(is_aligned_mask(ptr, size_align_mask(total)));

    // Same exposure as `alloc_size`: a realloc mints a NEW allocation whose
    // provenance the old block's exposure does not carry over to.
    (ptr as *mut u8).expose_provenance();

    let a = &mut *ator;
    a.current_size += total;
    a.current_size -= old_total;

    ptr
}

// ufbx.c:3742-3766 `ufbxi_free_size`
#[inline(never)]
pub(crate) unsafe fn free_size(ator: *mut Allocator, size: usize, ptr: *mut c_void, n: usize) {
    ufbx_assert!(size > 0);
    if n == 0 {
        return;
    }
    ufbx_assert!(!ptr.is_null());

    let total = size.wrapping_mul(n);

    // The old values have been checked by a previous allocate call
    ufbx_assert!(!does_overflow(total, size, n));
    let a = &mut *ator;
    ufbx_assert!(total <= a.current_size);
    a.current_size -= total;

    if (*ator).ator.allocator.alloc_fn.is_some() || (*ator).ator.allocator.realloc_fn.is_some() {
        // Don't call default free() if there is an user-provided `alloc_fn()`
        if let Some(free_fn) = (*ator).ator.allocator.free_fn {
            free_fn((*ator).ator.allocator.user, ptr, total);
        } else if let Some(realloc_fn) = (*ator).ator.allocator.realloc_fn {
            realloc_fn((*ator).ator.allocator.user, ptr, total, 0);
        }
    } else {
        ufbx_free(ptr, total);
    }
}

// ufbx.c:3768-3787 `ufbxi_grow_array_size`
// C: `ufbxi_noinline ufbxi_nodiscard static bool`; `p_ptr` is a `void *`
// pointing at the caller's `T *` slot, read/written via `*(void**)p_ptr`.
#[inline(never)]
#[must_use]
pub(crate) unsafe fn grow_array_size(
    ator: *mut Allocator,
    size: usize,
    p_ptr: *mut c_void,
    p_cap: *mut usize,
    n: usize,
) -> bool {
    #[cfg(feature = "regression")]
    {
        let a = &mut *ator;
        ufbxi_check_return_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(a.error) },
            a.num_allocs < a.max_allocs,
            false,
            "Allocation limit exceeded",
            "ator->num_allocs < ator->max_allocs"
        );
        a.num_allocs += 1;
    }

    if n <= *p_cap {
        return true;
    }
    let ptr: *mut c_void = *(p_ptr as *mut *mut c_void);
    let old_n = *p_cap;
    if old_n >= n {
        return true;
    }
    let new_n = max_sz(old_n.wrapping_mul(2), n);
    let new_ptr = realloc_size(ator, size, ptr, old_n, new_n);
    if new_ptr.is_null() {
        return false;
    }
    *(p_ptr as *mut *mut c_void) = new_ptr;
    *p_cap = new_n;
    true
}

// ufbx.c:3789-3798 `ufbxi_free_ator`
#[inline(never)]
pub(crate) unsafe fn free_ator(ator: *mut Allocator) {
    let a = &*ator;
    ufbx_assert!(a.current_size == 0);

    let free_fn = a.ator.allocator.free_allocator_fn;
    if let Some(free_fn) = free_fn {
        let user = a.ator.allocator.user;
        free_fn(user);
    }
}

// -- Typed wrappers (ufbx.c:3800-3806)
//
// C: `#define ufbxi_alloc(ator, type, n) ufbxi_maybe_null((type*)ufbxi_alloc_size((ator), sizeof(type), (n)))`
// The `sizeof(type)` argument becomes a generic parameter.
//
// NOTE: `ufbxi_alloc_zero` (ufbx.c:3801) and `ufbxi_realloc_zero` (ufbx.c:3803)
// are DEAD macros upstream: they expand to `ufbxi_alloc_zero_size` /
// `ufbxi_realloc_zero_size`, which are not defined anywhere in ufbx.c, and the
// macros have zero call sites (any use would fail to compile in C too). Not
// ported; revisit if an upstream sync adds the backing functions.

// ufbx.c:3800 `ufbxi_alloc(ator, type, n)`
#[inline(always)]
pub(crate) unsafe fn alloc<T>(ator: *mut Allocator, n: usize) -> *mut T {
    ufbxi_maybe_null!(alloc_size(ator, size_of::<T>(), n) as *mut T)
}

// ufbx.c:3802 `ufbxi_realloc(ator, type, old_ptr, old_n, n)`
// C-parity: the `ufbxi_realloc` macro has zero call sites in ufbx.c (C never
// warns about an unexpanded macro); kept for 1:1 coverage of the alloc family.
#[allow(dead_code)]
#[inline(always)]
pub(crate) unsafe fn realloc<T>(
    ator: *mut Allocator,
    old_ptr: *mut T,
    old_n: usize,
    n: usize,
) -> *mut T {
    ufbxi_maybe_null!(
        realloc_size(ator, size_of::<T>(), old_ptr as *mut c_void, old_n, n) as *mut T
    )
}

// ufbx.c:3804 `ufbxi_free(ator, type, ptr, n)`
#[inline(always)]
pub(crate) unsafe fn free<T>(ator: *mut Allocator, ptr: *mut T, n: usize) {
    free_size(ator, size_of::<T>(), ptr as *mut c_void, n)
}

// ufbx.c:3806 `ufbxi_grow_array(ator, p_ptr, p_cap, n)` — `sizeof(**(p_ptr))`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn grow_array<T>(
    ator: *mut Allocator,
    p_ptr: *mut *mut T,
    p_cap: *mut usize,
    n: usize,
) -> bool {
    grow_array_size(ator, size_of::<T>(), p_ptr as *mut c_void, p_cap, n)
}

// ufbx.c:3808-3815 implementation-header magic values
pub(crate) const SCENE_IMP_MAGIC: u32 = 0x58424655;
pub(crate) const MESH_IMP_MAGIC: u32 = 0x48534d55;
pub(crate) const LINE_CURVE_IMP_MAGIC: u32 = 0x55434c55;
pub(crate) const CACHE_IMP_MAGIC: u32 = 0x48434355;
pub(crate) const ANIM_IMP_MAGIC: u32 = 0x494e4155;
pub(crate) const BAKED_ANIM_IMP_MAGIC: u32 = 0x4b414255;
pub(crate) const REFCOUNT_IMP_MAGIC: u32 = 0x46455255;
pub(crate) const BUF_CHUNK_IMP_MAGIC: u32 = 0x46554255;

// ufbx.c:6936-6953 `ufbxi_init_ator`
#[inline(never)]
pub(crate) unsafe fn init_ator(
    error: *mut Error,
    ator: *mut Allocator,
    opts: *const RawAllocatorOpts,
    name: *const u8,
) {
    // C: `ufbx_allocator_opts zero_opts;` + `memset` in the null branch only.
    let mut zero_opts = core::mem::MaybeUninit::<RawAllocatorOpts>::uninit();
    let mut opts = opts;
    if opts.is_null() {
        core::ptr::write_bytes(zero_opts.as_mut_ptr(), 0, 1);
        opts = zero_opts.as_ptr();
    }

    // `opts` is either passed in or `zero_opts`.
    // cppcheck-suppress uninitvar
    // C: `ator->ator = *opts` — struct assignment (memcpy; `RawAllocatorOpts` is `Copy`).
    let a = &mut *ator;
    let o = &*opts;
    a.ator = *o;
    a.error = error;
    a.max_size = if o.memory_limit != 0 {
        o.memory_limit
    } else {
        usize::MAX
    };
    a.max_allocs = if o.allocation_limit != 0 {
        o.allocation_limit
    } else {
        usize::MAX
    };
    a.huge_size = if o.huge_threshold != 0 {
        o.huge_threshold
    } else {
        0x100000
    };
    a.chunk_max = if o.max_chunk_size != 0 {
        o.max_chunk_size
    } else {
        0x1000000
    };
    a.name = name;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::ErrorType;
    use core::mem::MaybeUninit;

    unsafe fn make_ator(error: *mut Error, opts: *const RawAllocatorOpts) -> Allocator {
        let mut ator = MaybeUninit::<Allocator>::zeroed();
        init_ator(error, ator.as_mut_ptr(), opts, b"test\0".as_ptr());
        ator.assume_init()
    }

    #[test]
    fn test_align_helpers() {
        assert_eq!(align_to_mask(0, 7), 0);
        assert_eq!(align_to_mask(1, 7), 8);
        assert_eq!(align_to_mask(8, 7), 8);
        assert_eq!(align_to_mask(9, 3), 12);
        assert_eq!(align_to_mask(usize::MAX, 0), usize::MAX);
        // size_align_mask: all bits below the lowest set bit, capped at max alignment
        assert_eq!(size_align_mask(1), 0);
        assert_eq!(size_align_mask(2), 1);
        assert_eq!(size_align_mask(4), 3);
        assert_eq!(size_align_mask(8), 7);
        assert_eq!(size_align_mask(16), MAXIMUM_ALIGNMENT - 1);
        assert_eq!(size_align_mask(12), 3);
        assert_eq!(size_align_mask(0), MAXIMUM_ALIGNMENT - 1); // size==0 wraps: 0 ^ SIZE_MAX
    }

    #[test]
    fn test_does_overflow() {
        assert!(!does_overflow(6, 2, 3));
        let a = usize::MAX / 2;
        assert!(does_overflow(a.wrapping_mul(4), a, 4));
        assert!(!does_overflow(0, 0, usize::MAX));
    }

    #[test]
    fn test_init_ator_defaults() {
        unsafe {
            let mut err = Error::default();
            let ator = make_ator(&mut err, core::ptr::null());
            assert_eq!(ator.max_size, usize::MAX);
            assert_eq!(ator.max_allocs, usize::MAX);
            assert_eq!(ator.huge_size, 0x100000);
            assert_eq!(ator.chunk_max, 0x1000000);
        }
    }

    #[test]
    fn test_alloc_free_accounting() {
        unsafe {
            let mut err = Error::default();
            let mut ator = make_ator(&mut err, core::ptr::null());

            // Zero-size allocation returns the shared zero-size buffer, no accounting.
            let z = alloc_size(&mut ator, 4, 0);
            assert_eq!(z as *const u8, ZERO_SIZE_BUFFER.as_ptr());
            assert_eq!(ator.num_allocs, 0);
            assert_eq!(ator.current_size, 0);

            let p = alloc::<u32>(&mut ator, 16);
            assert!(!p.is_null());
            assert_eq!(ator.num_allocs, 1);
            assert_eq!(ator.current_size, 64);

            let p = realloc::<u32>(&mut ator, p, 16, 32);
            assert!(!p.is_null());
            assert_eq!(ator.num_allocs, 2);
            assert_eq!(ator.current_size, 128);

            free::<u32>(&mut ator, p, 32);
            assert_eq!(ator.current_size, 0);
            free_ator(&mut ator);
        }
    }

    #[test]
    fn test_allocation_limit() {
        unsafe {
            let mut err = Error::default();
            let opts = RawAllocatorOpts {
                allocation_limit: 1,
                ..Default::default()
            };
            let mut ator = make_ator(&mut err, &opts);

            let p = alloc::<u8>(&mut ator, 8);
            assert!(!p.is_null());
            let q = alloc::<u8>(&mut ator, 8);
            assert!(q.is_null());
            // The description is recorded here; the type is resolved by the
            // `fix_error_type` strcmp ladder at top-level entry points.
            crate::native::error::fix_error_type(
                &mut err,
                b"Failed to load\0".as_ptr(),
                core::ptr::null_mut(),
            );
            assert_eq!(err.type_, ErrorType::AllocationLimit);
            // info carries the allocator name via `%s`
            assert_eq!(err.info(), "test");

            free::<u8>(&mut ator, p, 8);
            assert_eq!(ator.current_size, 0);
        }
    }

    #[test]
    fn test_memory_limit() {
        unsafe {
            let mut err = Error::default();
            let opts = RawAllocatorOpts {
                memory_limit: 100,
                ..Default::default()
            };
            let mut ator = make_ator(&mut err, &opts);

            // C check is `total < max_size - current_size`: exactly-at-limit fails.
            let p = alloc_size(&mut ator, 1, 100);
            assert!(p.is_null());
            crate::native::error::fix_error_type(
                &mut err,
                b"Failed to load\0".as_ptr(),
                core::ptr::null_mut(),
            );
            assert_eq!(err.type_, ErrorType::MemoryLimit);

            let p = alloc_size(&mut ator, 1, 99);
            assert!(!p.is_null());
            free_size(&mut ator, 1, p, 99);
        }
    }

    #[test]
    fn test_overflow_check() {
        unsafe {
            let mut err = Error::default();
            let mut ator = make_ator(&mut err, core::ptr::null());
            let p = alloc_size(&mut ator, 8, usize::MAX / 4);
            assert!(p.is_null());
            let p = alloc_size(&mut ator, 1, usize::MAX / 2 + 1);
            assert!(p.is_null());
            assert_eq!(ator.num_allocs, 0);
        }
    }

    #[test]
    fn test_grow_array() {
        unsafe {
            let mut err = Error::default();
            let mut ator = make_ator(&mut err, core::ptr::null());

            let mut ptr: *mut u32 = core::ptr::null_mut();
            let mut cap: usize = 0;
            assert!(grow_array::<u32>(&mut ator, &mut ptr, &mut cap, 4));
            assert!(!ptr.is_null());
            assert_eq!(cap, 4);
            // Growth doubles: n=5 with cap 4 grows to max(8, 5) = 8.
            assert!(grow_array::<u32>(&mut ator, &mut ptr, &mut cap, 5));
            assert_eq!(cap, 8);
            // No-op growth.
            let before = ptr;
            assert!(grow_array::<u32>(&mut ator, &mut ptr, &mut cap, 8));
            assert_eq!(ptr, before);
            assert_eq!(cap, 8);

            free::<u32>(&mut ator, ptr, cap);
            assert_eq!(ator.current_size, 0);
        }
    }

    #[test]
    fn test_user_callbacks_dispatch() {
        // alloc through alloc_fn, realloc via alloc+memcpy+free_fn, free via free_fn.
        use core::sync::atomic::{AtomicUsize, Ordering};
        static ALLOCS: AtomicUsize = AtomicUsize::new(0);
        static FREES: AtomicUsize = AtomicUsize::new(0);

        unsafe extern "C" fn my_alloc(_user: *mut c_void, size: usize) -> *mut c_void {
            ALLOCS.fetch_add(1, Ordering::SeqCst);
            ufbx_malloc(size)
        }
        unsafe extern "C" fn my_free(_user: *mut c_void, ptr: *mut c_void, old_size: usize) {
            FREES.fetch_add(1, Ordering::SeqCst);
            ufbx_free(ptr, old_size)
        }

        unsafe {
            let mut err = Error::default();
            let mut opts = RawAllocatorOpts::default();
            opts.allocator.alloc_fn = Some(my_alloc);
            opts.allocator.free_fn = Some(my_free);
            let mut ator = make_ator(&mut err, &opts);

            let p = alloc::<u64>(&mut ator, 4);
            assert!(!p.is_null());
            *p = 0x1122334455667788;
            assert_eq!(ALLOCS.load(Ordering::SeqCst), 1);

            // No realloc_fn: realloc dispatches to alloc_fn + memcpy + free_fn.
            let q = realloc::<u64>(&mut ator, p, 4, 8);
            assert!(!q.is_null());
            assert_eq!(*q, 0x1122334455667788);
            assert_eq!(ALLOCS.load(Ordering::SeqCst), 2);
            assert_eq!(FREES.load(Ordering::SeqCst), 1);

            free::<u64>(&mut ator, q, 8);
            assert_eq!(FREES.load(Ordering::SeqCst), 2);
            assert_eq!(ator.current_size, 0);
        }
    }
}
