#![cfg_attr(feature = "nightly", feature(likely_unlikely))]
#![allow(unused_braces)]

pub mod generated;
pub mod prelude;

// Native port of ufbx.c, one module per C banner section (declared in ufbx.c
// order). Crate-internal: the public surface is the ufbx-rust API above it.
pub(crate) mod native;
// C ABI shim exposing the exact ufbx.h surface for the upstream C test suite.
// The #[no_mangle] symbols export via linkage, not visibility.
#[cfg(feature = "c-abi")]
pub(crate) mod capi;

pub use generated::*;
pub use prelude::*;

use std::vec::Vec;

#[cfg(feature = "mint")]
pub mod impl_mint;

/*
pub fn open_memory(data: &[u8], opts: OpenMemoryOpts) -> Result<Stream> {
    let mut stream: RawStream = Default::default();
    let mut opts_mut = opts;
    let opts_raw = RawOpenMemoryOpts::from_rust(&mut opts_mut);
    let ok = unsafe { open_memory_raw(&mut stream, data, &opts_raw) }?;
    assert!(ok);
    Ok(Stream::Raw(unsafe { Unsafe::new(stream) }))
}
*/

/// Native-port extension: register a handler for ufbx panics (API-misuse
/// reports from the non-`catch` entry points; see `ufbx_panic_handler` in
/// ufbx.c). The C library only allows a compile-time `#define` override. The handler
/// may return, in which case the panicking call bails out gracefully with a
/// zero/default result exactly as in C; the default handler (when none is
/// registered) prints to stderr and asserts. The cost is a single atomic load,
/// paid only on the panic path.
pub fn set_panic_handler(handler: fn(&str)) {
    native::error::set_user_panic_handler(handler);
}

/// ufbx-rust extension: Vec-resizing convenience wrapper over
/// `triangulate_face` (which fills a caller-sized `&mut [u32]` like the C API).
///
/// No C counterpart — ufbx-rust's own hand-written addition to its generated
/// surface, reproduced verbatim (warts included) for drop-in parity;
/// bevy_ufbx calls it.
pub fn triangulate_face_vec(mut indices: &mut Vec<u32>, mesh: &Mesh, face: Face) -> u32 {
    if face.num_indices < 3 {
        indices.clear();
        return 0;
    }

    let num_triangles = face.num_indices as usize - 2;
    indices.resize(num_triangles * 3, 0);
    let num_triangles = triangulate_face(&mut indices, mesh, face);
    indices.shrink_to(num_triangles as usize * 3);
    num_triangles
}
