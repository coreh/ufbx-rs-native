//! Port of the API-section prelude of ufbx.c. Phase 1: only the refcount
//! lifecycle functions are ported (`ufbxi_free_scene_imp` ufbx.c:30243-30247
//! and `ufbxi_init_ref`/`ufbxi_retain_ref`/`ufbxi_release_ref`
//! ufbx.c:30249-30300 — C forward-declares the first two next to
//! `ufbxi_refcount` at ufbx.c:6229-6230 but defines them here), plus the
//! first public `ufbx_*` entry points: the `ufbx_open_file`/`ufbx_open_memory`/
//! `ufbx_default_open_file` plumbing (ufbx.c:30406-30495). The remaining
//! public entry points are NOT YET PORTED.
//!
//! HIGHEST-STAKES invariants (PORTING.md "Allocator + ufbxi_buf" /
//! "Atomics / refcount"):
//! - `ufbxi_release_ref` free order is VERBATIM: stack-copy `ator` and `buf`,
//!   re-point `buf.ator` to the STACK copy, then `buf_free` + `free_ator` —
//!   the `ufbxi_refcount` header lives inside the buffer being freed.
//! - The parent-chain walk is an ITERATIVE loop, not recursion.
//! - The counter starts at 0 (`init_ref` does no self-retain); inc/dec return
//!   the PREVIOUS value (SeqCst); the object is freed when the previous value
//!   was 0 (`if dec(...) > 0 { return }`).
#![allow(dead_code)]

use core::ffi::c_void;
use core::mem::{size_of, MaybeUninit};

use crate::generated::{Error, OpenFileInfo, RawOpenFileOpts, RawOpenMemoryOpts, RawStream};
use crate::native::allocator::{
    align_to_mask, alloc, free_ator, Allocator, CACHE_IMP_MAGIC, REFCOUNT_IMP_MAGIC,
    SCENE_IMP_MAGIC,
};
use crate::native::buf::{buf_free, Buf};
use crate::native::cache::{free_geometry_cache_imp, GeometryCacheImp};
use crate::native::error::strlen;
use crate::native::io::{
    begin_file_context, end_file_context, memory_close, memory_read, memory_size, memory_skip,
    stdio_open, FileContext, MemoryStream,
};
use crate::native::parse::{Refcount, SceneImp};
use crate::native::platform::{
    atomic_counter_dec, atomic_counter_free, atomic_counter_inc, atomic_counter_init, ufbx_assert,
    ufbxi_ignore,
};
use crate::prelude::OpenFileContext;

// ufbx.c:30243-30247 `ufbxi_free_scene_imp`
#[inline(never)]
pub(crate) unsafe fn free_scene_imp(imp: *mut SceneImp) {
    ufbx_assert!((*imp).magic == SCENE_IMP_MAGIC);
    buf_free(&mut (*imp).string_buf);
}

// ufbx.c:30249-30259 `ufbxi_init_ref`
#[inline(never)]
pub(crate) unsafe fn init_ref(refcount: *mut Refcount, magic: u32, parent: *mut Refcount) {
    if !parent.is_null() {
        retain_ref(parent);
    }

    atomic_counter_init(&mut (*refcount).refcount);
    (*refcount).self_magic = REFCOUNT_IMP_MAGIC;
    (*refcount).type_magic = magic;
    (*refcount).parent = parent;
}

// ufbx.c:30261-30267 `ufbxi_retain_ref`
#[inline(never)]
pub(crate) unsafe fn retain_ref(refcount: *mut Refcount) {
    ufbx_assert!((*refcount).self_magic == REFCOUNT_IMP_MAGIC);
    let count: usize = atomic_counter_inc(&mut (*refcount).refcount);
    ufbxi_ignore!(count);
    ufbx_assert!(count < usize::MAX / 2);
}

// ufbx.c:30269-30300 `ufbxi_release_ref`
#[inline(never)]
pub(crate) unsafe fn release_ref(mut refcount: *mut Refcount) {
    while !refcount.is_null() {
        ufbx_assert!((*refcount).self_magic == REFCOUNT_IMP_MAGIC);
        if atomic_counter_dec(&mut (*refcount).refcount) > 0 {
            return;
        }
        atomic_counter_free(&mut (*refcount).refcount);

        let parent: *mut Refcount = (*refcount).parent;
        let type_magic: u32 = (*refcount).type_magic;

        (*refcount).self_magic = 0;
        (*refcount).type_magic = 0;

        // Type-specific cleanup
        match type_magic {
            SCENE_IMP_MAGIC => free_scene_imp(refcount as *mut SceneImp),
            CACHE_IMP_MAGIC => free_geometry_cache_imp(refcount as *mut GeometryCacheImp),
            _ => {}
        }

        // We need to free `data_buf` last and be careful to copy it to
        // the stack since the `ufbxi_refcount` that contains it is allocated
        // from the same result buffer!
        let mut ator: Allocator = (*refcount).ator;
        let mut buf: Buf = (*refcount).buf;
        buf.ator = &mut ator;
        buf_free(&mut buf);
        free_ator(&mut ator);

        refcount = parent;
    }
}

// ufbx.c:30406-30410 `ufbx_default_open_file`
// `extern "C"`: this exact function pointer is stored into
// `ufbx_open_file_cb.fn` defaults and compared by address (ufbx.c:24645,
// 25224, 25532, 32712). C has exactly ONE address for it — the exported
// symbol — so under `feature = "c-abi"` this impl IS the export
// (`export_name`, no shim in `capi.rs`): a C caller that assigns
// `ufbx_default_open_file` into a callback must pass the loader's
// compare-by-address fast path (ufbx.c:25224) exactly as in C.
#[cfg_attr(feature = "c-abi", export_name = "ufbx_default_open_file")]
pub(crate) unsafe extern "C" fn default_open_file(
    user: *mut c_void,
    stream: *mut RawStream,
    path: *const u8,
    path_len: usize,
    info: *const OpenFileInfo,
) -> bool {
    let _ = user; // C: `(void)user;`
    open_file_ctx(
        stream,
        (*info).context,
        path,
        path_len,
        core::ptr::null(),
        core::ptr::null_mut(),
    )
}

// ufbx.c:30412-30415 `ufbx_open_file`
pub(crate) unsafe fn open_file(
    stream: *mut RawStream,
    path: *const u8,
    path_len: usize,
    opts: *const RawOpenFileOpts,
    error: *mut Error,
) -> bool {
    open_file_ctx(stream, 0 as OpenFileContext, path, path_len, opts, error)
}

// ufbx.c:30417-30435 `ufbx_open_file_ctx`
pub(crate) unsafe fn open_file_ctx(
    stream: *mut RawStream,
    ctx: OpenFileContext,
    path: *const u8,
    mut path_len: usize,
    opts: *const RawOpenFileOpts,
    error: *mut Error,
) -> bool {
    let ok: bool;
    let mut fc = MaybeUninit::<FileContext>::uninit(); // ufbxi_uninit
    let fc: *mut FileContext = fc.as_mut_ptr();
    begin_file_context(fc, ctx, core::ptr::null());
    if path_len == usize::MAX {
        path_len = strlen(path);
    }
    // C: `#if !defined(UFBX_NO_STDIO)` — always taken (no matching feature);
    // the disabled branch reports `"UFBX_NO_STDIO", "Feature disabled"`.
    ok = stdio_open(
        fc,
        stream,
        path,
        path_len,
        if !opts.is_null() {
            (*opts).filename_null_terminated
        } else {
            false
        },
    );
    end_file_context(fc, error, ok);
    ok
}

// ufbx.c:30437-30440 `ufbx_open_memory`
pub(crate) unsafe fn open_memory(
    stream: *mut RawStream,
    data: *const c_void,
    data_size: usize,
    opts: *const RawOpenMemoryOpts,
    error: *mut Error,
) -> bool {
    open_memory_ctx(stream, 0 as OpenFileContext, data, data_size, opts, error)
}

// ufbx.c:30442-30495 `ufbx_open_memory_ctx`
pub(crate) unsafe fn open_memory_ctx(
    stream: *mut RawStream,
    ctx: OpenFileContext,
    data: *const c_void,
    data_size: usize,
    opts: *const RawOpenMemoryOpts,
    error: *mut Error,
) -> bool {
    let mut local_opts = MaybeUninit::<RawOpenMemoryOpts>::uninit(); // ufbxi_uninit
    let mut opts = opts;
    if opts.is_null() {
        core::ptr::write_bytes(
            local_opts.as_mut_ptr() as *mut u8,
            0,
            size_of::<RawOpenMemoryOpts>(),
        );
        opts = local_opts.as_ptr();
    }
    ufbx_assert!((*opts)._begin_zero == 0 && (*opts)._end_zero == 0);

    let mut fc = MaybeUninit::<FileContext>::uninit(); // ufbxi_uninit
    let fc: *mut FileContext = fc.as_mut_ptr();
    begin_file_context(fc, ctx, &(*opts).allocator);

    let copy_size: usize = if (*opts).no_copy { 0 } else { data_size };

    // Align the allocation size to 8 bytes to make sure the header is aligned.
    let self_size: usize = align_to_mask(size_of::<MemoryStream>().wrapping_add(copy_size), 7);

    let memory: *mut u8 = alloc::<u8>(&mut (*fc).ator, self_size);
    if memory.is_null() {
        end_file_context(fc, error, false);
        return false;
    }

    let mem = memory as *mut MemoryStream;
    core::ptr::write_bytes(mem as *mut u8, 0, size_of::<MemoryStream>());

    (*mem).size = data_size;
    (*mem).self_size = self_size;
    (*mem).close_cb = (*opts).close_cb;

    if (*opts).no_copy {
        (*mem).data = data;
    } else {
        // C: `memcpy(mem->data_copy, data, data_size)` — the flexible array
        // member starts right after the header (see `MemoryStream`).
        let data_copy: *mut u8 = (mem as *mut u8).add(size_of::<MemoryStream>());
        core::ptr::copy_nonoverlapping(data as *const u8, data_copy, data_size);
        (*mem).data = data_copy as *const c_void;
    }

    // Transplant the allocator in the result blob
    if !(*fc).parent_ator.is_null() {
        (*mem).parent_ator = (*fc).parent_ator;
    } else {
        (*fc).parent_ator = &mut (*mem).local_ator;
    }

    (*stream).read_fn = Some(memory_read);
    (*stream).skip_fn = Some(memory_skip);
    (*stream).size_fn = Some(memory_size);
    (*stream).close_fn = Some(memory_close);
    (*stream).user = mem as *mut c_void;

    end_file_context(fc, error, true);

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::Error;
    use crate::generated::RawAllocatorOpts;
    use crate::native::allocator::{init_ator, MESH_IMP_MAGIC};
    use crate::native::buf::push_size;
    use crate::native::parse::{get_imp, MeshImp};
    use core::ffi::c_void;
    use core::mem::size_of;

    // Build a refcounted object the way the C setup code does: an allocator
    // feeding a result buffer, with the `ufbxi_refcount` header pushed into
    // that same buffer (the header-inside-own-buffer trick `release_ref` must
    // survive).
    unsafe fn make_imp(error: *mut Error, parent: *mut Refcount) -> *mut MeshImp {
        let mut ator = core::mem::MaybeUninit::<Allocator>::zeroed().assume_init();
        let opts = RawAllocatorOpts::default();
        init_ator(error, &mut ator, &opts, b"test\0".as_ptr());

        let mut buf = core::mem::MaybeUninit::<Buf>::zeroed().assume_init();
        buf.ator = &mut ator;

        let imp = push_size(&mut buf, size_of::<MeshImp>(), 1) as *mut MeshImp;
        assert!(!imp.is_null());
        core::ptr::write_bytes(imp as *mut u8, 0, size_of::<MeshImp>());
        init_ref(&mut (*imp).refcount, MESH_IMP_MAGIC, parent);
        (*imp).magic = MESH_IMP_MAGIC;

        // Transfer the allocator/buffer into the refcount header, as the C
        // setup paths do before returning the object to the user.
        (*imp).refcount.ator = ator;
        (*imp).refcount.buf = buf;
        imp
    }

    #[test]
    fn test_refcount_lifecycle_and_get_imp() {
        unsafe {
            let mut error = Error::default();
            let imp = make_imp(&mut error, core::ptr::null_mut());

            // Counter starts at 0 (no self-retain); retain makes the previous
            // value 1 so one release only decrements.
            assert_eq!(
                (*imp)
                    .refcount
                    .refcount
                    .load(core::sync::atomic::Ordering::SeqCst),
                0
            );
            retain_ref(&mut (*imp).refcount);

            let mesh_ptr = core::ptr::addr_of_mut!((*imp).mesh) as *mut c_void;
            let back: *mut MeshImp = get_imp(mesh_ptr);
            assert_eq!(back, imp);

            release_ref(&mut (*imp).refcount);
            // Still alive: previous value was 1.
            assert_eq!((*imp).refcount.self_magic, REFCOUNT_IMP_MAGIC);

            // Final release frees the object (previous value 0). The
            // header-inside-own-buffer free order is exercised for real here;
            // miri/asan-style UAF would fire on a wrong port.
            release_ref(&mut (*imp).refcount);
        }
    }

    #[test]
    fn test_release_ref_walks_parent_chain() {
        unsafe {
            let mut error = Error::default();
            let parent = make_imp(&mut error, core::ptr::null_mut());
            // Child holds the only reference to the parent (init_ref retains).
            let child = make_imp(&mut error, &mut (*parent).refcount);
            assert_eq!(
                (*parent)
                    .refcount
                    .refcount
                    .load(core::sync::atomic::Ordering::SeqCst),
                1
            );

            // Releasing the child (count 0) frees it AND iteratively releases
            // the parent, whose count drops from 1 to 0 -> freed too.
            release_ref(&mut (*child).refcount);
        }
    }

    use crate::generated::RawCloseMemoryCb;

    #[test]
    fn test_open_memory_ctx_copy_and_close() {
        unsafe {
            let data = *b"hello, memory stream";
            let mut stream = RawStream::default();
            let mut error = MaybeUninit::<Error>::zeroed().assume_init();
            assert!(open_memory(
                &mut stream,
                data.as_ptr() as *const c_void,
                data.len(),
                core::ptr::null(),
                &mut error,
            ));
            assert_eq!(error.type_ as u32, 0);

            // The stream owns a copy: reads survive the original going away.
            assert_eq!((stream.size_fn.unwrap())(stream.user), data.len() as u64);
            let mut buf = [0u8; 5];
            assert_eq!(
                (stream.read_fn.unwrap())(stream.user, buf.as_mut_ptr() as *mut c_void, 5),
                5
            );
            assert_eq!(&buf, b"hello");
            assert!((stream.skip_fn.unwrap())(stream.user, 7));
            assert_eq!(
                (stream.read_fn.unwrap())(stream.user, buf.as_mut_ptr() as *mut c_void, 5),
                5
            );
            assert_eq!(&buf, b"y str");
            // Reads clamp at the end of the memory block.
            assert_eq!(
                (stream.read_fn.unwrap())(stream.user, buf.as_mut_ptr() as *mut c_void, 5),
                3
            );
            assert!(!(stream.skip_fn.unwrap())(stream.user, 1));

            (stream.close_fn.unwrap())(stream.user);
        }
    }

    #[test]
    fn test_open_memory_no_copy_close_cb() {
        unsafe extern "C" fn close_cb(user: *mut c_void, data: *mut c_void, data_size: usize) {
            let hits = user as *mut (usize, usize);
            (*hits).0 = data as usize;
            (*hits).1 = data_size;
        }

        unsafe {
            let data = *b"no-copy";
            let mut hits: (usize, usize) = (0, 0);
            let mut opts = RawOpenMemoryOpts::default();
            opts.no_copy = true;
            opts.close_cb = RawCloseMemoryCb {
                fn_: Some(close_cb),
                user: &mut hits as *mut (usize, usize) as *mut c_void,
            };
            let mut stream = RawStream::default();
            assert!(open_memory(
                &mut stream,
                data.as_ptr() as *const c_void,
                data.len(),
                &opts,
                core::ptr::null_mut(),
            ));
            // no_copy: the stream reads the caller's bytes in place.
            let mem = stream.user as *mut MemoryStream;
            assert_eq!((*mem).data as usize, data.as_ptr() as usize);
            (stream.close_fn.unwrap())(stream.user);
            assert_eq!(hits, (data.as_ptr() as usize, data.len()));
        }
    }

    #[test]
    fn test_open_file_missing_reports_file_not_found() {
        unsafe {
            let mut stream = RawStream::default();
            let mut error = MaybeUninit::<Error>::zeroed().assume_init();
            let path = b"definitely/not/a/real/file.fbx";
            assert!(!open_file(
                &mut stream,
                path.as_ptr(),
                path.len(),
                core::ptr::null(),
                &mut error,
            ));
            let desc =
                core::slice::from_raw_parts(error.description.data, error.description.length);
            assert_eq!(desc, b"File not found");
            assert_eq!(error.info(), core::str::from_utf8(path).unwrap());
        }
    }

    #[test]
    fn test_open_file_reads_real_file() {
        unsafe {
            let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
            let path = std::format!("{}/Cargo.toml", dir);
            let expected = std::fs::read(&path).unwrap();

            let mut stream = RawStream::default();
            let mut error = MaybeUninit::<Error>::zeroed().assume_init();
            assert!(open_file(
                &mut stream,
                path.as_ptr(),
                path.len(),
                core::ptr::null(),
                &mut error,
            ));
            assert_eq!(
                (stream.size_fn.unwrap())(stream.user),
                expected.len() as u64
            );

            let mut got = std::vec::Vec::new();
            got.resize(expected.len(), 0u8);
            let mut read_total = 0usize;
            while read_total < expected.len() {
                let n = (stream.read_fn.unwrap())(
                    stream.user,
                    got.as_mut_ptr().add(read_total) as *mut c_void,
                    expected.len() - read_total,
                );
                assert!(n != 0 && n != usize::MAX);
                read_total += n;
            }
            assert_eq!(got, expected);
            (stream.close_fn.unwrap())(stream.user);
        }
    }
}
