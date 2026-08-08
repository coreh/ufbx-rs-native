//! C ABI shim: #[no_mangle] extern "C" ufbx_* definitions matching ufbx.h exactly.
//! Test/validation only (feature = "c-abi"). Populated in Phase 1 step 8.

// ufbx.c:878 `ufbx_abi_data_def const uint32_t ufbx_source_version = UFBX_SOURCE_VERSION;`
#[no_mangle]
pub static ufbx_source_version: u32 = crate::native::platform::SOURCE_VERSION;

// ufbx.c:3131-3276 `ufbx_inflate` (impl: native/deflate.rs `inflate`)
#[no_mangle]
pub unsafe extern "C" fn ufbx_inflate(
    dst: *mut core::ffi::c_void,
    dst_size: usize,
    input: *const crate::generated::InflateInput,
    retain: *mut crate::generated::InflateRetain,
) -> isize {
    crate::native::deflate::inflate(dst, dst_size, input, retain)
}

// ufbx.c:30406-30410 `ufbx_default_open_file`: NO shim here. C compares this
// callback BY ADDRESS (`uc->opts.open_file_cb.fn == &ufbx_default_open_file`,
// ufbx.c:25224, stored at 24645/25532/32712), so there must be exactly one
// function address; the export is `#[export_name = "ufbx_default_open_file"]`
// directly on the impl `native::api::default_open_file`.

// ufbx.c:30412-30415 `ufbx_open_file` (impl: native/api.rs `open_file`)
#[no_mangle]
pub unsafe extern "C" fn ufbx_open_file(
    stream: *mut crate::generated::RawStream,
    path: *const u8,
    path_len: usize,
    opts: *const crate::generated::RawOpenFileOpts,
    error: *mut crate::generated::Error,
) -> bool {
    crate::native::api::open_file(stream, path, path_len, opts, error)
}

// ufbx.c:30417-30435 `ufbx_open_file_ctx` (impl: native/api.rs `open_file_ctx`)
#[no_mangle]
pub unsafe extern "C" fn ufbx_open_file_ctx(
    stream: *mut crate::generated::RawStream,
    ctx: crate::prelude::OpenFileContext,
    path: *const u8,
    path_len: usize,
    opts: *const crate::generated::RawOpenFileOpts,
    error: *mut crate::generated::Error,
) -> bool {
    crate::native::api::open_file_ctx(stream, ctx, path, path_len, opts, error)
}

// ufbx.c:30437-30440 `ufbx_open_memory` (impl: native/api.rs `open_memory`)
#[no_mangle]
pub unsafe extern "C" fn ufbx_open_memory(
    stream: *mut crate::generated::RawStream,
    data: *const core::ffi::c_void,
    data_size: usize,
    opts: *const crate::generated::RawOpenMemoryOpts,
    error: *mut crate::generated::Error,
) -> bool {
    crate::native::api::open_memory(stream, data, data_size, opts, error)
}

// ufbx.c:30442-30495 `ufbx_open_memory_ctx` (impl: native/api.rs `open_memory_ctx`)
#[no_mangle]
pub unsafe extern "C" fn ufbx_open_memory_ctx(
    stream: *mut crate::generated::RawStream,
    ctx: crate::prelude::OpenFileContext,
    data: *const core::ffi::c_void,
    data_size: usize,
    opts: *const crate::generated::RawOpenMemoryOpts,
    error: *mut crate::generated::Error,
) -> bool {
    crate::native::api::open_memory_ctx(stream, ctx, data, data_size, opts, error)
}
