#![cfg_attr(feature = "nightly", feature(likely_unlikely))]
#![allow(unused_braces)]

pub mod generated;
pub mod prelude;

// Native port of ufbx.c, one module per C banner section (declared in ufbx.c order).
pub mod native;
// C ABI shim exposing the exact ufbx.h surface for the upstream C test suite.
#[cfg(feature = "c-abi")]
pub mod capi;

pub use prelude::*;
pub use generated::*;

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

/// Register a handler for ufbx panics (API-misuse reports from the non-`catch`
/// entry points; see `ufbx_panic_handler` in ufbx.c). Native-port extension —
/// the C library only allows a compile-time `#define` override. The handler
/// may return, in which case the panicking call bails out gracefully with a
/// zero/default result exactly as in C; the default handler (when none is
/// registered) prints to stderr and asserts. The cost is a single atomic load,
/// paid only on the panic path.
pub fn set_panic_handler(handler: fn(&str)) {
    native::error::set_user_panic_handler(handler);
}

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
