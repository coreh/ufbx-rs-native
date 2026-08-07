//! C ABI shim: #[no_mangle] extern "C" ufbx_* definitions matching ufbx.h exactly.
//! Test/validation only (feature = "c-abi"). Populated in Phase 1 step 8.

// ufbx.c:878 `ufbx_abi_data_def const uint32_t ufbx_source_version = UFBX_SOURCE_VERSION;`
#[no_mangle]
pub static ufbx_source_version: u32 = crate::native::platform::SOURCE_VERSION;
