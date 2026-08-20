//! Port of the `// -- IO` (ufbx.c:6714-6931), `// -- File IO`
//! (ufbx.c:6991-7188) and `// -- Memory IO` (ufbx.c:7190-7243) banner sections:
//! the `ufbxi_refill`/read/skip buffer machinery on `ufbxi_context`, the
//! `ufbxi_file_context` begin/end pair, the stdio-backed file streams and the
//! memory streams. Their only entry points, the public `ufbx_open_file`/
//! `ufbx_open_memory`/`ufbx_default_open_file` plumbing (ufbx.c:30406-30495,
//! `-- API` banner), live in `native::api` per the PORTING.md Naming table.
//!
//! `ufbxi_init_ator` (ufbx.c:6936-6953) sits between `ufbxi_read_to` and
//! `ufbxi_file_context` in the C source but is owned by `native::allocator`
//! (see the note in that module's header) — used from here, not redefined.
//!
//! stdio access goes through `extern "C"` libc declarations (`fopen`, `fread`,
//! ...) mirroring the C build's direct libc calls — same rationale as the
//! `malloc` externs in `native::allocator`: libc is already linked on std
//! targets. The `UFBX_EXTERNAL_STDIO` branch (ufbx.c:7149-7188) has no
//! corresponding cargo feature and is not ported.
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
use core::ffi::c_void;
use core::mem::{size_of, MaybeUninit};

use crate::generated::{Error, RawAllocatorOpts, RawCloseMemoryCb, RawStream};
use crate::native::allocator::{alloc, free, free_ator, init_ator, Allocator};
use crate::native::error::{
    clear_error, fix_error_type, set_err_info, ufbxi_check, ufbxi_check_msg, ufbxi_check_return,
    ufbxi_check_return_msg, ufbxi_report_err_msg, Fail,
};
use crate::native::parse::{get_read_offset, report_progress, Context};
use crate::native::platform::{max64, max_sz, min64, min_sz, to_size, ufbx_assert, MAX_SKIP_SIZE};
use crate::prelude::OpenFileContext;

// -- IO

// ufbx.c:6716-6781 `ufbxi_refill`
// C: `static ufbxi_noinline const char *` — returns NULL on failure.
#[inline(never)]
pub(crate) fn refill(uc: &Context, size: usize, require_size: bool) -> *const u8 {
    ufbx_assert!(uc.data_size() < size);
    ufbxi_check_return!(uc, !uc.eof(), core::ptr::null(), "!uc->eof");
    if require_size {
        ufbxi_check_return_msg!(
            uc,
            uc.read_fn().is_some() || uc.data_size() > 0,
            core::ptr::null(),
            "Empty file",
            "uc->read_fn || uc->data_size > 0"
        );
        ufbxi_check_return_msg!(
            uc,
            uc.read_fn().is_some(),
            core::ptr::null(),
            "Truncated file",
            "uc->read_fn"
        );
    } else if uc.read_fn().is_none() {
        uc.set_eof(true);
        return uc.data();
    }

    let mut data_to_free: *mut u8 = core::ptr::null_mut();
    let mut size_to_free: usize = 0;

    // Grow the read buffer if necessary, data is copied over below with the
    // usual path so the free is deferred (`size_to_free`, `data_to_free`)
    if size > uc.read_buffer_size() {
        let mut new_size = max_sz(size, uc.opts_view().read_buffer_size());
        // C-parity: `uc->read_buffer_size * 2` is a size_t multiply that wraps.
        new_size = max_sz(new_size, uc.read_buffer_size().wrapping_mul(2));
        size_to_free = uc.read_buffer_size();
        data_to_free = uc.read_buffer();
        // SAFETY: allocating from `uc`'s own temp allocator through its
        // raw-ptr getter (uc construction invariant).
        let new_buffer: *mut u8 = unsafe { alloc::<u8>(uc.ator_tmp_mut_ptr(), new_size) };
        ufbxi_check_return!(uc, !new_buffer.is_null(), core::ptr::null(), "new_buffer");
        uc.set_read_buffer(new_buffer);
        uc.set_read_buffer_size(new_size);
    }

    // Copy the remains of the previous buffer to the beginning of the new one
    let mut data_size: usize = uc.data_size();
    if data_size > 0 {
        ufbx_assert!(!uc.read_buffer().is_null() && !uc.data().is_null());
        // C: `memmove(uc->read_buffer, uc->data, data_size)` — the ranges may
        // overlap when the buffer did not grow.
        // SAFETY: `uc->data` has `data_size` readable bytes left (the buffered
        // window invariant), and `read_buffer` is at least `read_buffer_size >=
        // data_size` bytes — either the pre-existing buffer `data` points into,
        // or the larger one just allocated.
        unsafe { core::ptr::copy(uc.data(), uc.read_buffer(), data_size) };
    }

    if size_to_free != 0 {
        // SAFETY: `data_to_free`/`size_to_free` are the previous
        // `read_buffer`/`read_buffer_size` pair, allocated from this same
        // allocator and already copied out of above.
        unsafe { free::<u8>(uc.ator_tmp_mut_ptr(), data_to_free, size_to_free) };
    }

    // Fill the rest of the buffer with user data
    let data_capacity: usize = uc.read_buffer_size();
    while data_size < data_capacity {
        let to_read: usize = data_capacity - data_size;
        // SAFETY: `read_fn` is `Some` (checked at entry: `require_size` demands
        // it, the `else` branch returned early without it); `read_buffer` holds
        // `data_capacity` bytes and `data_size + to_read == data_capacity`, so
        // the destination window is in bounds; `read_user` is the pointer
        // paired with the callback.
        let read_result: usize = unsafe {
            (uc.read_fn().unwrap_unchecked())(
                uc.read_user(),
                uc.read_buffer().add(data_size) as *mut c_void,
                to_read,
            )
        };
        ufbxi_check_return_msg!(
            uc,
            read_result != usize::MAX,
            core::ptr::null(),
            "IO error",
            "read_result != SIZE_MAX"
        );
        ufbxi_check_return!(
            uc,
            read_result <= to_read,
            core::ptr::null(),
            "read_result <= to_read"
        );
        data_size += read_result;
        if read_result == 0 {
            uc.set_eof(true);
            break;
        }
    }

    if require_size {
        if uc.data_offset() == 0 {
            ufbxi_check_return_msg!(
                uc,
                data_size > 0,
                core::ptr::null(),
                "Empty file",
                "data_size > 0"
            );
        }
        ufbxi_check_return_msg!(
            uc,
            data_size >= size,
            core::ptr::null(),
            "Truncated file",
            "data_size >= size"
        );
    }

    // C-parity: `uc->data - uc->data_begin` — both may legitimately be NULL
    // (`ufbxi_read_to` leaves them so), which `<*const u8>::offset_from` treats
    // as UB in Rust; the address subtraction is spelled out with casts instead.
    uc.set_data_offset(
        uc.data_offset()
            .wrapping_add(to_size(uc.data() as isize - uc.data_begin() as isize) as u64),
    );
    uc.set_data_begin(uc.read_buffer());
    uc.set_data(uc.read_buffer());
    uc.set_data_size(data_size);

    uc.read_buffer()
}

// ufbx.c:6783-6787 `ufbxi_pause_progress`
#[inline(always)]
pub(crate) fn pause_progress(uc: &Context) {
    uc.set_data_size(uc.data_size() + uc.yield_size());
    uc.set_yield_size(0);
}

// ufbx.c:6789-6799 `ufbxi_resume_progress`
#[inline(never)]
pub(crate) fn resume_progress(uc: &Context) -> Result<(), Fail> {
    uc.set_yield_size(min_sz(uc.data_size(), uc.progress_interval()));
    uc.set_data_size(uc.data_size() - uc.yield_size());

    if get_read_offset(uc).wrapping_sub(uc.latest_progress_bytes()) >= uc.progress_interval() as u64
    {
        // C: `ufbxi_check(ufbxi_report_progress(uc));` — the caller-side check
        // pushes its own error-stack frame on top of the callee's (a bare `?`
        // would drop it).
        ufbxi_check!(uc, report_progress(uc).is_ok(), "ufbxi_report_progress(uc)");
    }

    Ok(())
}

// ufbx.c:6801-6815 `ufbxi_yield`
// (`yield` is a reserved word in Rust — trailing underscore, cf. `type_`.)
#[inline(never)]
pub(crate) fn yield_(uc: &Context, size: usize) -> *const u8 {
    let ret: *const u8;
    uc.set_data_size(uc.data_size() + uc.yield_size());
    if uc.data_size() >= size {
        ret = uc.data();
    } else {
        ret = refill(uc, size, true);
    }
    uc.set_yield_size(min_sz(uc.data_size(), max_sz(size, uc.progress_interval())));
    uc.set_data_size(uc.data_size() - uc.yield_size());

    ufbxi_check_return!(
        uc,
        report_progress(uc).is_ok(),
        core::ptr::null(),
        "ufbxi_report_progress(uc)"
    );
    ret
}

// ufbx.c:6817-6824 `ufbxi_peek_bytes`
#[inline(always)]
pub(crate) fn peek_bytes(uc: &Context, size: usize) -> *const u8 {
    if uc.yield_size() >= size {
        uc.data()
    } else {
        yield_(uc, size)
    }
}

// ufbx.c:6826-6841 `ufbxi_read_bytes`
#[inline(always)]
pub(crate) fn read_bytes(uc: &Context, size: usize) -> *const u8 {
    // Refill the current buffer if necessary
    let ret: *const u8;
    if uc.yield_size() >= size {
        ret = uc.data();
    } else {
        ret = yield_(uc, size);
        if ret.is_null() {
            return core::ptr::null();
        }
    }

    // Advance the read position inside the current buffer
    uc.set_yield_size(uc.yield_size() - size);
    // SAFETY: `ret` points into the current buffer, which holds at least `size`
    // bytes here (checked above / guaranteed by `yield_`), so advancing by
    // `size` stays in-bounds.
    uc.set_data(unsafe { ret.add(size) });
    ret
}

// ufbx.c:6843-6849 `ufbxi_consume_bytes`
#[inline(always)]
pub(crate) fn consume_bytes(uc: &Context, size: usize) {
    // Bytes must have been checked first with `ufbxi_peek_bytes()`
    ufbx_assert!(size <= uc.yield_size());
    uc.set_yield_size(uc.yield_size() - size);
    // SAFETY: `size <= yield_size()` is asserted above (`ufbx_assert!` is an
    // unconditional `assert!`), so the cursor stays inside the buffered read
    // window (`data .. data + yield_size + data_size`).
    uc.set_data(unsafe { uc.data().add(size) });
}

// ufbx.c:6851-6896 `ufbxi_skip_bytes`
#[inline(never)]
pub(crate) fn skip_bytes(uc: &Context, mut size: u64) -> Result<(), Fail> {
    if uc.skip_fn().is_some() {
        pause_progress(uc);

        if size > uc.data_size() as u64 {
            size -= uc.data_size() as u64;
            // SAFETY: advancing the read cursor by exactly the bytes still
            // buffered lands on the end of the buffered window.
            uc.set_data(unsafe { uc.data().add(uc.data_size()) });
            uc.set_data_size(0);

            uc.set_data_offset(uc.data_offset().wrapping_add(size));
            while size >= MAX_SKIP_SIZE as u64 {
                size -= MAX_SKIP_SIZE as u64;
                ufbxi_check_msg!(
                    uc,
                    // SAFETY: `skip_fn` is `Some` (enclosing branch); it is
                    // called with its paired `read_user` — the C stream
                    // callback contract.
                    unsafe { (uc.skip_fn().unwrap_unchecked())(uc.read_user(), MAX_SKIP_SIZE - 1) },
                    "Truncated file",
                    "uc->skip_fn(uc->read_user, UFBXI_MAX_SKIP_SIZE - 1)"
                );

                // Check that we can read at least one byte in case the file is broken
                // and causes us to seek indefinitely forwards as `fseek()` does not
                // report if we hit EOF...
                let mut single_byte = MaybeUninit::<[u8; 1]>::uninit(); // ufbxi_uninit
                                                                        // SAFETY: a stream that provides `skip_fn` also provides
                                                                        // `read_fn` (stream-setup invariant); the destination is a
                                                                        // local 1-byte buffer and exactly 1 byte is requested.
                let num_read: usize = unsafe {
                    (uc.read_fn().unwrap_unchecked())(
                        uc.read_user(),
                        single_byte.as_mut_ptr() as *mut c_void,
                        1,
                    )
                };
                ufbxi_check_msg!(uc, num_read <= 1, "IO error", "num_read <= 1");
                ufbxi_check_msg!(uc, num_read == 1, "Truncated file", "num_read == 1");
            }

            if size > 0 {
                ufbxi_check_msg!(
                    uc,
                    // SAFETY: `skip_fn` is `Some` (enclosing branch); C stream
                    // callback contract as above.
                    unsafe { (uc.skip_fn().unwrap_unchecked())(uc.read_user(), size as usize) },
                    "Truncated file",
                    "uc->skip_fn(uc->read_user, (size_t)size)"
                );
            }
        } else {
            // SAFETY: this branch has `size <= data_size`, so the advance stays
            // inside the buffered read window.
            uc.set_data(unsafe { uc.data().add(size as usize) });
            uc.set_data_size(uc.data_size() - size as usize);
        }

        // C: `ufbxi_check(ufbxi_resume_progress(uc));` — caller-side frame.
        ufbxi_check!(uc, resume_progress(uc).is_ok(), "ufbxi_resume_progress(uc)");
    } else {
        // Read and discard bytes in reasonable chunks
        let skip_size: u64 = max64(
            uc.read_buffer_size() as u64,
            uc.opts_view().read_buffer_size() as u64,
        );
        while size > 0 {
            let to_skip: u64 = min64(size, skip_size);
            ufbxi_check!(
                uc,
                !read_bytes(uc, to_skip as usize).is_null(),
                "ufbxi_read_bytes(uc, (size_t)to_skip)"
            );
            size -= to_skip;
        }
    }

    Ok(())
}

// ufbx.c:6898-6931 `ufbxi_read_to`
#[inline(never)]
pub(crate) unsafe fn read_to(uc: &Context, dst: *mut c_void, mut size: usize) -> Result<(), Fail> {
    let mut ptr = dst as *mut u8;

    pause_progress(uc);

    // Copy data from the current buffer first
    let len: usize = min_sz(uc.data_size(), size);
    // C-parity: `memcpy(ptr, uc->data, len)` — `uc->data` may be NULL when
    // `len == 0` (memory input fully consumed), as in C.
    // SAFETY: `len <= uc.data_size()`, so `uc.data()` has `len` readable bytes
    // in the buffered window, and `dst` has at least `size >= len` writable
    // bytes (the raw-pointer contract of this `unsafe fn`); the caller's
    // destination and `uc`'s read buffer are distinct allocations.
    unsafe { core::ptr::copy_nonoverlapping(uc.data(), ptr, len) };
    // SAFETY: `len <= uc.data_size()`, so the cursor stays inside the buffered
    // read window.
    uc.set_data(unsafe { uc.data().add(len) });
    uc.set_data_size(uc.data_size() - len);
    // SAFETY: `len` bytes of `dst` were consumed above, so `ptr + len` is at
    // most one past the end of the caller's destination buffer.
    ptr = unsafe { ptr.add(len) };
    size -= len;

    // If there's data left to copy try to read from user IO
    if size > 0 {
        // C-parity: `uc->data - uc->data_begin` — see `ufbxi_refill`; both are
        // NULL after a previous `ufbxi_read_to` streamed past the buffer, so
        // the subtraction is done on addresses rather than via `offset_from`.
        uc.set_data_offset(
            uc.data_offset()
                .wrapping_add(to_size(uc.data() as isize - uc.data_begin() as isize) as u64),
        );

        uc.set_data_begin(core::ptr::null());
        uc.set_data(core::ptr::null());
        uc.set_data_size(0);
        ufbxi_check!(uc, uc.read_fn().is_some(), "uc->read_fn");

        while size > 0 {
            // SAFETY: `read_fn` is `Some` (checked just above); it is called
            // with its paired `read_user` per the C stream-callback contract,
            // and `ptr`/`size` are the unconsumed tail of the caller's
            // destination buffer.
            let read_result: usize = unsafe {
                (uc.read_fn().unwrap_unchecked())(uc.read_user(), ptr as *mut c_void, size)
            };
            ufbxi_check_msg!(
                uc,
                read_result != usize::MAX,
                "IO error",
                "read_result != SIZE_MAX"
            );
            ufbxi_check!(uc, read_result != 0, "read_result != 0");

            // C-parity: a misbehaving `read_fn` may return more than `size`;
            // C wraps the size_t subtraction and keeps going.
            ptr = ptr.wrapping_add(read_result);
            size = size.wrapping_sub(read_result);
            uc.set_data_offset(uc.data_offset().wrapping_add(read_result as u64));
        }
    }

    // C: `ufbxi_check(ufbxi_resume_progress(uc));` — caller-side frame.
    ufbxi_check!(uc, resume_progress(uc).is_ok(), "ufbxi_resume_progress(uc)");

    Ok(())
}

// ufbx.c:6936-6953 `ufbxi_init_ator` — owned by `native::allocator` (see the
// module header); used below via `allocator::init_ator`.

// ufbx.c:6955-6960 `ufbxi_file_context`
#[repr(C)]
pub(crate) struct InnerFileContext {
    pub error: Error,

    pub parent_ator: *mut Allocator,
    pub ator: Allocator,
}

// Safe `&FileContext` handle over the fields-struct `InnerFileContext`, mirroring
// the `Context`/`InnerContext` seam in `parse.rs`. `MaybeUninit` keeps it uniform
// with the other context wrappers (and the whole context is born uninitialized —
// `ufbxi_uninit`); `UnsafeCell` gives the interior mutability every
// `&FileContext` site needs.
#[repr(transparent)]
pub(crate) struct FileContext(
    pub(crate) core::cell::UnsafeCell<core::mem::MaybeUninit<InnerFileContext>>,
);

impl FileContext {
    #[inline(always)]
    pub(crate) fn get(&self) -> *mut InnerFileContext {
        self.0.get().cast()
    }

    #[inline(always)]
    pub(crate) fn ator(&self) -> crate::native::allocator::Allocator {
        unsafe { (*self.get()).ator }
    }

    #[inline(always)]
    pub(crate) fn set_ator(&self, ator: crate::native::allocator::Allocator) {
        unsafe {
            (*self.get()).ator = ator;
        }
    }

    // `error` — const raw-ptr getter (read-only sites); see `error_mut_ptr` for mutation.

    // `ator` — const raw-ptr getter (read-only sites); see `ator_mut_ptr` for mutation.

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

    // `ator` — raw-ptr getter (address of field for out-param/mutation sites).
    #[inline(always)]
    pub(crate) fn ator_mut_ptr(&self) -> *mut Allocator {
        // SAFETY: `&raw mut` computes the field address with the cell's
        // provenance without forming a reference; no aliasing assertion.
        unsafe { &raw mut (*self.get()).ator }
    }

    // `ator` (Allocator) — typed VIEW handle (reinterpret-in-place); accessors on AllocatorView.
    #[inline(always)]
    pub(crate) fn ator_view(&self) -> &crate::native::allocator::AllocatorView {
        // SAFETY: reinterpret the owned Allocator field in place; interior-mutable, no validity asserted.
        unsafe { &*(&raw mut (*self.get()).ator as *mut crate::native::allocator::AllocatorView) }
    }

    // `parent_ator` — scalar value accessor.
    #[inline(always)]
    pub(crate) fn parent_ator(&self) -> *mut Allocator {
        // SAFETY: reading a scalar field; all bit patterns of `*mut Allocator` are valid.
        unsafe { (*self.get()).parent_ator }
    }

    #[inline(always)]
    pub(crate) fn set_parent_ator(&self, parent_ator: *mut Allocator) {
        // SAFETY: storing a scalar; cannot violate validity.
        unsafe {
            (*self.get()).parent_ator = parent_ator;
        }
    }
}

// ufbx.c:6962-6972 `ufbxi_begin_file_context`
#[inline(never)]
pub(crate) unsafe fn begin_file_context(
    fc: &FileContext,
    ctx: OpenFileContext,
    ator_opts: *const RawAllocatorOpts,
) {
    // SAFETY: `fc.get()` addresses the `MaybeUninit<InnerFileContext>` cell the
    // `&FileContext` borrow keeps alive, so exactly `size_of::<InnerFileContext>()`
    // bytes are writable there (C: `memset(fc, 0, sizeof(*fc))`).
    unsafe { core::ptr::write_bytes(fc.get() as *mut u8, 0, size_of::<InnerFileContext>()) };
    if ctx != 0 {
        fc.set_parent_ator(ctx as *mut Allocator);
        // SAFETY: a non-zero `OpenFileContext` is a live `*mut Allocator` handed
        // out by `ufbx_open_file_ctx` — the contract of this `unsafe fn`.
        fc.set_ator(unsafe { *fc.parent_ator() });
        fc.ator_view().set_error(fc.error_mut_ptr());
    } else {
        // SAFETY: `error`/`ator` are `fc`'s own fields (live for the borrow) and
        // `ator_opts` is the caller's `RawAllocatorOpts` or null, which
        // `init_ator` accepts.
        unsafe { init_ator(fc.error_mut_ptr(), fc.ator_mut_ptr(), ator_opts, c"file") };
    }
}

// ufbx.c:6974-6989 `ufbxi_end_file_context`
#[inline(never)]
pub(crate) unsafe fn end_file_context(fc: &FileContext, error: *mut Error, ok: bool) {
    if !fc.parent_ator().is_null() {
        // SAFETY: a non-null `parent_ator` is the live caller-owned `Allocator`
        // `begin_file_context` was handed (checked non-null just above); the
        // read of its error slot and the write-back of the local copy both
        // target that same live allocator.
        fc.ator_view()
            .set_error(unsafe { (*fc.parent_ator()).error });
        unsafe { *fc.parent_ator() = fc.ator() };
    } else {
        // SAFETY: with no parent, `fc` owns its `ator` field, which is live for
        // the duration of the `&FileContext` borrow.
        unsafe { free_ator(fc.ator_mut_ptr()) };
    }
    if !error.is_null() {
        if !ok {
            // SAFETY: `error` is the caller's live `Error` slot (checked
            // non-null above) and `fc.error_mut_ptr()` is `fc`'s own field.
            unsafe { fix_error_type(fc.error_mut_ptr(), b"Failed to open file\0".as_ptr(), error) };
        } else {
            // SAFETY: `error` is the caller's live `Error` slot, checked
            // non-null above.
            unsafe { clear_error(error) };
        }
    }
}

// -- File IO

// C: `#if !defined(UFBX_NO_STDIO) && !defined(UFBX_EXTERNAL_STDIO)` — no cargo
// feature maps to either define; the default (stdio-enabled) branch is ported.

// libc stdio externs (same rationale as the `malloc` externs in
// `native::allocator`: libc is already linked on std targets). `FILE *` is
// opaque and carried as `*mut c_void`, exactly like the `void *user` slot the
// C code stores it in.
mod libc_stdio {
    use core::ffi::{c_long, c_void};
    extern "C" {
        pub(super) fn fopen(filename: *const u8, mode: *const u8) -> *mut c_void;
        pub(super) fn fread(
            ptr: *mut c_void,
            size: usize,
            nmemb: usize,
            stream: *mut c_void,
        ) -> usize;
        pub(super) fn ferror(stream: *mut c_void) -> i32;
        pub(super) fn fseek(stream: *mut c_void, offset: c_long, whence: i32) -> i32;
        pub(super) fn fclose(stream: *mut c_void) -> i32;
        pub(super) fn fgetpos(stream: *mut c_void, pos: *mut super::FposT) -> i32;
        pub(super) fn fsetpos(stream: *mut c_void, pos: *const super::FposT) -> i32;
        pub(super) fn rewind(stream: *mut c_void);
        // C: `ftello()` under `UFBX_HAS_FTELLO` (`_POSIX_C_SOURCE >= 200112L`,
        // ufbx.c:609-615), `_ftelli64()` under `_MSC_VER`, plain `ftell()`
        // otherwise. `ftello` returns `off_t`, which is only guaranteed 64-bit
        // where we can see it from Rust on 64-bit unix (32-bit glibc defaults
        // to a 32-bit `off_t` — declaring it `i64` there would be an ABI
        // mismatch), so other targets take C's `#else` plain-`ftell` branch.
        #[cfg(all(unix, target_pointer_width = "64"))]
        pub(super) fn ftello(stream: *mut c_void) -> i64;
        #[cfg(windows)]
        pub(super) fn _ftelli64(stream: *mut c_void) -> i64;
        #[cfg(not(any(all(unix, target_pointer_width = "64"), windows)))]
        pub(super) fn ftell(stream: *mut c_void) -> c_long;
        #[cfg(windows)]
        pub(super) fn _wfopen(filename: *const u16, mode: *const u16) -> *mut c_void;
    }
}

// `fpos_t` storage for `fgetpos`/`fsetpos`: the type differs per libc
// (macOS/MSVC: 64-bit integer; glibc: 16-byte struct). 32 bytes at alignment
// 16 over-covers all supported libcs.
#[repr(C, align(16))]
pub(crate) struct FposT {
    _opaque: [u8; 32],
}

// `SEEK_CUR`/`SEEK_END` share these values on every supported libc.
const SEEK_CUR: i32 = 1;
const SEEK_END: i32 = 2;

// ufbx.c:6995-7070 `ufbxi_fopen` — the `_WIN32` branch (UTF-8 to UTF-16
// conversion + `_wfopen`).
#[cfg(windows)]
#[inline(never)]
pub(crate) unsafe fn fopen(
    fc: &FileContext,
    path: *const u8,
    path_len: usize,
    null_terminated: bool,
) -> *mut c_void {
    let file: *mut c_void;
    let _ = null_terminated; // C: `(void)null_terminated;`
    let mut wpath_buf = MaybeUninit::<[u16; 256]>::uninit(); // ufbxi_uninit
    let wpath: *mut u16;
    if path_len < 256 - 1 {
        wpath = wpath_buf.as_mut_ptr() as *mut u16;
    } else {
        // SAFETY: allocating from `fc`'s own `ator` field, live for the duration
        // of the `&FileContext` borrow.
        wpath = unsafe { alloc::<u16>(fc.ator_mut_ptr(), path_len + 1) };
        if wpath.is_null() {
            return core::ptr::null_mut();
        }
    }

    // Convert UTF-8 to UTF-16 but allow stray surrogate pairs as the Windows
    // file system encoding allows them as well..
    let mut wlen: usize = 0;
    let mut i: usize = 0;
    // SAFETY (every `*path.add(i)` and `*wpath.add(wlen)` in this loop and the
    // terminator write that follows it): `path` has `path_len` readable bytes —
    // the raw-pointer contract of this `unsafe fn` — and each read is guarded by
    // `i < path_len`; `wpath` addresses the C-parity destination, the 256-unit
    // local when `path_len < 255` and otherwise the `path_len + 1`-unit
    // allocation made above, indexed by `wlen` exactly as C indexes
    // `wpath[wlen++]` (ufbx.c:7011-7036).
    while i < path_len {
        let mut code: u32 = u32::MAX;
        let c: u8 = unsafe { *path.add(i) };
        i += 1;
        if (c & 0x80) == 0 {
            code = c as u32;
        } else if (c & 0xe0) == 0xc0 {
            code = (c & 0x1f) as u32;
            if i < path_len {
                code = code << 6 | (unsafe { *path.add(i) } & 0x3f) as u32;
                i += 1;
            }
        } else if (c & 0xf0) == 0xe0 {
            code = (c & 0x0f) as u32;
            if i < path_len {
                code = code << 6 | (unsafe { *path.add(i) } & 0x3f) as u32;
                i += 1;
            }
            if i < path_len {
                code = code << 6 | (unsafe { *path.add(i) } & 0x3f) as u32;
                i += 1;
            }
        } else if (c & 0xf8) == 0xf0 {
            code = (c & 0x07) as u32;
            if i < path_len {
                code = code << 6 | (unsafe { *path.add(i) } & 0x3f) as u32;
                i += 1;
            }
            if i < path_len {
                code = code << 6 | (unsafe { *path.add(i) } & 0x3f) as u32;
                i += 1;
            }
            if i < path_len {
                code = code << 6 | (unsafe { *path.add(i) } & 0x3f) as u32;
                i += 1;
            }
        }
        if code < 0x10000 {
            unsafe { *wpath.add(wlen) = code as u16 };
            wlen += 1;
        } else {
            // C-parity: `code` may still be UINT32_MAX for malformed UTF-8;
            // the unsigned subtraction wraps as in C.
            code = code.wrapping_sub(0x10000);
            unsafe { *wpath.add(wlen) = 0xd800u32.wrapping_add(code >> 10) as u16 };
            wlen += 1;
            unsafe { *wpath.add(wlen) = 0xdc00u32.wrapping_add(code & 0x3ff) as u16 };
            wlen += 1;
        }
    }
    unsafe { *wpath.add(wlen) = 0 };

    // C: `_wfopen_s` under `UFBXI_MSC_VER >= 1400`, `_wfopen` otherwise —
    // the compiler-version fork collapses to the plain `_wfopen` call.
    // SAFETY: `wpath` is the NUL-terminated UTF-16 path just built.
    file = unsafe { libc_stdio::_wfopen(wpath, [0x72u16, 0x62u16, 0u16].as_ptr()) }; // L"rb"
    if wpath != wpath_buf.as_mut_ptr() as *mut u16 {
        // SAFETY: reaching here `wpath` is the `path_len + 1`-unit block
        // allocated above from this same allocator.
        unsafe { free::<u16>(fc.ator_mut_ptr(), wpath, path_len + 1) };
    }
    if file.is_null() {
        // SAFETY: `fc`'s own `error` field, plus the caller's `path`/`path_len`
        // readable range per the fn contract.
        unsafe { set_err_info(fc.error_mut_ptr(), path, path_len) };
        ufbxi_report_err_msg!(fc.error_view(), "file", "File not found");
    }
    file
}

// ufbx.c:6995-7070 `ufbxi_fopen` — the plain-`fopen` branch.
#[cfg(not(windows))]
#[inline(never)]
pub(crate) unsafe fn fopen(
    fc: &FileContext,
    path: *const u8,
    path_len: usize,
    null_terminated: bool,
) -> *mut c_void {
    let mut copy_buf = MaybeUninit::<[u8; 256]>::uninit(); // ufbxi_uninit
    let copy: *mut u8;
    if null_terminated {
        copy = path as *mut u8;
    } else {
        if path_len < 256 - 1 {
            copy = copy_buf.as_mut_ptr() as *mut u8;
        } else {
            // SAFETY: allocating from `fc`'s own `ator` field, live for the
            // duration of the `&FileContext` borrow.
            copy = unsafe { alloc::<u8>(fc.ator_mut_ptr(), path_len + 1) };
            if copy.is_null() {
                return core::ptr::null_mut();
            }
        }
        // SAFETY: `path` has `path_len` readable bytes (the raw-pointer
        // contract of this `unsafe fn`) and `copy` holds `path_len + 1` writable
        // bytes — either the 256-byte local (this branch requires
        // `path_len < 255`) or the allocation just made; the two are distinct
        // allocations, and the trailing byte is the terminator slot.
        unsafe { core::ptr::copy_nonoverlapping(path, copy, path_len) };
        unsafe { *copy.add(path_len) = b'\0' };
    }
    // SAFETY: `copy` is a NUL-terminated path — either written above or the
    // caller's own NUL-terminated `path` (`null_terminated`).
    let file: *mut c_void = unsafe { libc_stdio::fopen(copy, b"rb\0".as_ptr()) };
    if !null_terminated && copy != copy_buf.as_mut_ptr() as *mut u8 {
        // SAFETY: reaching here `copy` is the `path_len + 1` block allocated
        // above from this same allocator.
        unsafe { free::<u8>(fc.ator_mut_ptr(), copy, path_len + 1) };
    }
    if file.is_null() {
        // SAFETY: `fc`'s own `error` field, plus the caller's `path`/`path_len`
        // readable range per the fn contract.
        unsafe { set_err_info(fc.error_mut_ptr(), path, path_len) };
        ufbxi_report_err_msg!(fc.error_view(), "file", "File not found");
    }
    file
}

// ufbx.c:7073-7086 `ufbxi_ftell`
pub(crate) unsafe fn ftell(file: *mut c_void) -> u64 {
    // C: `ftello()` (`UFBX_HAS_FTELLO`), `_ftelli64()` (`_MSC_VER`) or plain
    // `ftell()` — one branch per target, selected by cfg instead of #if (see
    // the extern block for why `ftello` is limited to 64-bit unix).
    #[cfg(all(unix, target_pointer_width = "64"))]
    {
        // SAFETY: `file` is a live `FILE *` — the raw-pointer contract of this
        // `unsafe fn`.
        let result: i64 = unsafe { libc_stdio::ftello(file) };
        if result >= 0 {
            return result as u64;
        }
    }
    #[cfg(windows)]
    {
        // SAFETY: `file` is a live `FILE *` — the raw-pointer contract of this
        // `unsafe fn`.
        let result: i64 = unsafe { libc_stdio::_ftelli64(file) };
        if result >= 0 {
            return result as u64;
        }
    }
    #[cfg(not(any(all(unix, target_pointer_width = "64"), windows)))]
    {
        // SAFETY: `file` is a live `FILE *` — the raw-pointer contract of this
        // `unsafe fn`.
        let result: i64 = unsafe { libc_stdio::ftell(file) } as i64;
        if result >= 0 {
            return result as u64;
        }
    }
    u64::MAX
}

// ufbx.c:7088-7093 `ufbxi_stdio_read`
pub(crate) unsafe extern "C" fn stdio_read(
    user: *mut c_void,
    data: *mut c_void,
    max_size: usize,
) -> usize {
    let file: *mut c_void = user;
    // SAFETY: `user` is the `FILE *` this callback set was installed with by
    // `stdio_init`, and the stream contract guarantees it is still open.
    if unsafe { libc_stdio::ferror(file) } != 0 {
        return usize::MAX;
    }
    // SAFETY: same live `FILE *`; `data` has `max_size` writable bytes — the
    // `ufbx_read_fn` contract the caller of this callback honors.
    unsafe { libc_stdio::fread(data, 1, max_size, file) }
}

// ufbx.c:7095-7102 `ufbxi_stdio_skip`
pub(crate) unsafe extern "C" fn stdio_skip(user: *mut c_void, size: usize) -> bool {
    let file: *mut c_void = user;
    ufbx_assert!(size <= MAX_SKIP_SIZE);
    // SAFETY (both calls): `user` is the `FILE *` this callback set was
    // installed with by `stdio_init`, still open per the stream contract.
    if unsafe { libc_stdio::fseek(file, size as core::ffi::c_long, SEEK_CUR) } != 0 {
        return false;
    }
    if unsafe { libc_stdio::ferror(file) } != 0 {
        return false;
    }
    true
}

// ufbx.c:7104-7124 `ufbxi_stdio_size`
pub(crate) unsafe extern "C" fn stdio_size(user: *mut c_void) -> u64 {
    let file: *mut c_void = user;
    let mut result: u64 = 0;
    // SAFETY (every libc call below, and both `ftell` calls): `user` is the
    // `FILE *` this callback set was installed with by `stdio_init`, still open
    // per the stream contract. `pos` is a local `FposT` slot, sized and aligned
    // to over-cover every supported libc's `fpos_t`, and `fsetpos` reads it back
    // only on the `fgetpos == 0` path that initialized it.
    let begin: u64 = unsafe { ftell(file) };
    if begin < u64::MAX {
        let mut pos = MaybeUninit::<FposT>::uninit(); // ufbxi_uninit
        if unsafe { libc_stdio::fgetpos(file, pos.as_mut_ptr()) } == 0 {
            if unsafe { libc_stdio::fseek(file, 0, SEEK_END) } == 0 {
                let end: u64 = unsafe { ftell(file) };
                if end != u64::MAX && begin < end {
                    result = end - begin;
                }
                // Both `rewind()` and `fsetpos()` to reset error and EOF
                unsafe { libc_stdio::rewind(file) };
                unsafe { libc_stdio::fsetpos(file, pos.as_ptr()) };
            }
        }
    }
    result
}

// ufbx.c:7126-7130 `ufbxi_stdio_close`
pub(crate) unsafe extern "C" fn stdio_close(user: *mut c_void) {
    let file: *mut c_void = user;
    // SAFETY: `user` is the `FILE *` this callback set was installed with by
    // `stdio_init`; `close_fn` is invoked once, and the stream is not used after.
    unsafe { libc_stdio::fclose(file) };
}

// ufbx.c:7132-7139 `ufbxi_stdio_init`
#[inline(never)]
pub(crate) unsafe fn stdio_init(stream: *mut RawStream, file: *mut c_void, close: bool) {
    // SAFETY: `stream` points at the caller's live `RawStream` slot — the
    // raw-pointer contract of this `unsafe fn`; all five stores are scalar
    // writes into that one struct.
    unsafe {
        (*stream).read_fn = Some(stdio_read);
        (*stream).skip_fn = Some(stdio_skip);
        (*stream).size_fn = Some(stdio_size);
        (*stream).close_fn = if close { Some(stdio_close) } else { None };
        (*stream).user = file;
    }
}

// ufbx.c:7141-7147 `ufbxi_stdio_open`
#[inline(never)]
pub(crate) unsafe fn stdio_open(
    fc: &FileContext,
    stream: *mut RawStream,
    path: *const u8,
    path_len: usize,
    null_terminated: bool,
) -> bool {
    // SAFETY: forwarding this fn's own `path`/`path_len`/`null_terminated`
    // contract to `fopen`.
    let file: *mut c_void = unsafe { fopen(fc, path, path_len, null_terminated) };
    if file.is_null() {
        return false;
    }
    // SAFETY: forwarding the caller's live `RawStream` slot; `file` is the
    // freshly opened stream, checked non-null above, and `close` transfers its
    // ownership to the stream.
    unsafe { stdio_init(stream, file, true) };
    true
}

// ufbx.c:7149-7188 — the `UFBX_EXTERNAL_STDIO` variants of `ufbxi_stdio_init`/
// `ufbxi_stdio_open` (user-provided `ufbx_stdio_*` functions): no cargo
// feature maps to `UFBX_EXTERNAL_STDIO`; not ported.

// -- Memory IO

// ufbx.c:7192-7204 `ufbxi_memory_stream`
// C ends with a flexible array member `char data_copy[];` — header-only
// struct; the copied bytes live at `(stream as *mut u8).add(size_of::<
// MemoryStream>())` — `size_of::<MemoryStream>()` == C's
// `offsetof(ufbxi_memory_stream, data_copy)`, pinned by the const asserts after
// the struct.
#[repr(C)]
pub(crate) struct MemoryStream {
    pub data: *const c_void,
    pub size: usize,
    pub position: usize,
    pub close_cb: RawCloseMemoryCb,

    // Own allocation information
    pub self_size: usize,
    pub parent_ator: *mut Allocator,
    pub local_ator: Allocator,
    pub error: Error,
}

// C's `offsetof(ufbxi_memory_stream, data_copy)` is the end of the last member
// (a `char[]` FAM needs no padding), and both the copy destination
// (ufbx.c:30475) and the allocation size (ufbx.c:30459) are derived from
// `size_of::<MemoryStream>()` in the Rust mapping — so the header must have no
// trailing padding, and it must stay 8-byte aligned for the "header is aligned"
// invariant the 8-byte size rounding at ufbx.c:30459 relies on.
const _: () = assert!(
    size_of::<MemoryStream>() == core::mem::offset_of!(MemoryStream, error) + size_of::<Error>()
);
const _: () = assert!(size_of::<MemoryStream>() % 8 == 0);

// ufbx.c:7206-7213 `ufbxi_memory_read`
pub(crate) unsafe extern "C" fn memory_read(
    user: *mut c_void,
    data: *mut c_void,
    max_size: usize,
) -> usize {
    let stream = user as *mut MemoryStream;
    // SAFETY: `user` is the `MemoryStream` this callback set was installed with
    // (`ufbx_open_memory`), live until `memory_close` runs.
    let to_read: usize = min_sz(unsafe { (*stream).size - (*stream).position }, max_size);
    // SAFETY: `data`/`size` describe the stream's readable byte range and
    // `position + to_read <= size` by the `min_sz` above, so the source window is
    // in bounds; `data` (the destination) has `max_size >= to_read` writable
    // bytes per the `ufbx_read_fn` contract, and the caller's destination is a
    // different allocation from the stream's source buffer.
    unsafe {
        core::ptr::copy_nonoverlapping(
            ((*stream).data as *const u8).add((*stream).position),
            data as *mut u8,
            to_read,
        )
    };
    // SAFETY: live `MemoryStream` as above; advancing its own cursor.
    unsafe { (*stream).position += to_read };
    to_read
}

// ufbx.c:7215-7221 `ufbxi_memory_skip`
pub(crate) unsafe extern "C" fn memory_skip(user: *mut c_void, size: usize) -> bool {
    let stream = user as *mut MemoryStream;
    // SAFETY (both accesses): `user` is the `MemoryStream` this callback set was
    // installed with (`ufbx_open_memory`), live until `memory_close` runs; the
    // second reads and advances its own cursor.
    if unsafe { (*stream).size - (*stream).position } < size {
        return false;
    }
    unsafe { (*stream).position += size };
    true
}

// ufbx.c:7223-7227 `ufbxi_memory_size`
pub(crate) unsafe extern "C" fn memory_size(user: *mut c_void) -> u64 {
    let stream = user as *mut MemoryStream;
    // SAFETY: `user` is the `MemoryStream` this callback set was installed with
    // (`ufbx_open_memory`), live until `memory_close` runs.
    unsafe { (*stream).size as u64 }
}

// ufbx.c:7229-7243 `ufbxi_memory_close`
pub(crate) unsafe extern "C" fn memory_close(user: *mut c_void) {
    let stream = user as *mut MemoryStream;
    // SAFETY (every access to `*stream` below): `user` is the `MemoryStream`
    // this callback set was installed with (`ufbx_open_memory`), live until this
    // fn frees it — which is the last thing it does.
    if unsafe { (*stream).close_cb.fn_.is_some() } {
        // SAFETY: `fn_` is `Some` (checked on the line above) and is invoked with
        // the `user` pointer stored beside it plus the stream's own
        // `data`/`size` — the `ufbx_close_memory_cb` contract.
        unsafe {
            ((*stream).close_cb.fn_.unwrap_unchecked())(
                (*stream).close_cb.user,
                (*stream).data as *mut c_void,
                (*stream).size,
            )
        };
    }

    if !unsafe { (*stream).parent_ator }.is_null() {
        // SAFETY: a non-null `parent_ator` is the live allocator the
        // `self_size`-byte stream block was allocated from.
        unsafe {
            free::<u8>(
                (*stream).parent_ator,
                stream as *mut u8,
                (*stream).self_size,
            )
        };
    } else {
        // SAFETY: with no parent, the stream owns `local_ator`; the stack copy
        // keeps the allocator usable while the block holding it is released
        // (C-parity, ufbx.c:7238-7242).
        let mut ator: Allocator = unsafe { (*stream).local_ator };
        // SAFETY: `ator` is that same allocator (by value) and the stream block
        // of `self_size` bytes came from it; `free_ator` then tears down an
        // allocator with zero live bytes.
        unsafe { free::<u8>(&mut ator, stream as *mut u8, (*stream).self_size) };
        unsafe { free_ator(&mut ator) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::RawLoadOpts;
    use crate::native::parse::InnerContext;

    // A zeroed context is what C builds via `memset` before setup; only the
    // IO fields need real values (precedent: `native::parse` tests).
    fn zeroed_context() -> std::boxed::Box<InnerContext> {
        unsafe { std::boxed::Box::new_zeroed().assume_init() }
    }

    fn init_tmp_ator(uc: &Context) {
        // SAFETY: initializing `uc`'s own `ator_tmp`/`error` fields through its
        // raw-ptr getters; a null `opts` selects the defaults.
        unsafe {
            init_ator(
                uc.error_mut_ptr(),
                uc.ator_tmp_mut_ptr(),
                core::ptr::null(),
                c"tmp",
            );
        }
    }

    struct SliceReader {
        data: &'static [u8],
        pos: usize,
        chunk: usize,
    }

    unsafe extern "C" fn slice_read(
        user: *mut c_void,
        data: *mut c_void,
        max_size: usize,
    ) -> usize {
        // SAFETY: the tests install this callback with `read_user` pointing at a
        // live `SliceReader` that outlives the read; it is the only reference to
        // it while the callback runs.
        let r = unsafe { &mut *(user as *mut SliceReader) };
        let n = r.chunk.min(max_size).min(r.data.len() - r.pos);
        // SAFETY: `pos + n <= data.len()` by the `min` above, so the source
        // window is in bounds, and `data` has `max_size >= n` writable bytes per
        // the `ufbx_read_fn` contract; source and destination are distinct.
        unsafe { core::ptr::copy_nonoverlapping(r.data.as_ptr().add(r.pos), data as *mut u8, n) };
        r.pos += n;
        n
    }

    #[test]
    fn test_refill_read_bytes_and_skip() {
        static DATA: [u8; 64] = {
            let mut d = [0u8; 64];
            let mut i = 0;
            while i < 64 {
                d[i] = i as u8;
                i += 1;
            }
            d
        };
        let mut reader = SliceReader {
            data: &DATA,
            pos: 0,
            chunk: 7,
        };
        let mut uc = zeroed_context();
        unsafe {
            init_tmp_ator(Context::from_ptr(&raw mut *uc));
            uc.read_fn = Some(slice_read);
            uc.read_user = &mut reader as *mut SliceReader as *mut c_void;
            uc.opts = MaybeUninit::<RawLoadOpts>::zeroed().assume_init();
            uc.opts.read_buffer_size = 16;
            uc.progress_interval = 0x4000;

            // `ufbxi_read_bytes` refills through `ufbxi_yield` and returns the
            // first 8 bytes.
            let p = read_bytes(Context::from_ptr(&raw mut *uc), 8);
            assert!(!p.is_null());
            assert_eq!(core::slice::from_raw_parts(p, 8), &DATA[..8]);

            // Skipping without `skip_fn` reads-and-discards in chunks.
            assert!(skip_bytes(Context::from_ptr(&raw mut *uc), 40).is_ok());
            let p = read_bytes(Context::from_ptr(&raw mut *uc), 8);
            assert!(!p.is_null());
            assert_eq!(core::slice::from_raw_parts(p, 8), &DATA[48..56]);

            // Read offset accounting covers everything consumed so far.
            assert_eq!(get_read_offset(Context::from_ptr(&raw mut *uc)), 56);

            // Reading past EOF fails with "Truncated file".
            let p = read_bytes(Context::from_ptr(&raw mut *uc), 64);
            assert!(p.is_null());
            let desc =
                core::slice::from_raw_parts(uc.error.description.data, uc.error.description.length);
            assert_eq!(desc, b"Truncated file");

            free(&mut uc.ator_tmp, uc.read_buffer, uc.read_buffer_size);
            free_ator(&mut uc.ator_tmp);
        }
    }

    #[test]
    fn test_read_to_copies_buffered_and_streamed_data() {
        static DATA: [u8; 32] = {
            let mut d = [0u8; 32];
            let mut i = 0;
            while i < 32 {
                d[i] = (i as u8) ^ 0x5a;
                i += 1;
            }
            d
        };
        let mut reader = SliceReader {
            data: &DATA,
            pos: 0,
            chunk: 5,
        };
        let mut uc = zeroed_context();
        unsafe {
            init_tmp_ator(Context::from_ptr(&raw mut *uc));
            uc.read_fn = Some(slice_read);
            uc.read_user = &mut reader as *mut SliceReader as *mut c_void;
            uc.opts = MaybeUninit::<RawLoadOpts>::zeroed().assume_init();
            uc.opts.read_buffer_size = 8;
            uc.progress_interval = 0x4000;

            // Buffer the first 8 bytes, then `read_to` a 24-byte destination:
            // part from the buffer, the rest straight from the reader.
            let p = peek_bytes(Context::from_ptr(&raw mut *uc), 8);
            assert!(!p.is_null());
            let mut dst = [0u8; 24];
            assert!(read_to(
                Context::from_ptr(&raw mut *uc),
                dst.as_mut_ptr() as *mut c_void,
                24
            )
            .is_ok());
            assert_eq!(&dst[..], &DATA[..24]);
            assert_eq!(uc.data_offset, 24);

            free(&mut uc.ator_tmp, uc.read_buffer, uc.read_buffer_size);
            free_ator(&mut uc.ator_tmp);
        }
    }

    // The `ufbx_open_file`/`ufbx_open_memory` entry-point tests live in
    // `native::api` next to the functions they exercise.
}
