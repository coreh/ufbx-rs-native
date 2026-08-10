#![cfg_attr(feature = "nightly", feature(likely_unlikely))]
#![allow(unused_braces)]
// Crate-wide clippy allows for lints that are a poor fit for a line-faithful C
// port of ufbx.c, in the current state of the port. Each is a deliberate,
// pervasive stance rather than a fixable one-off; genuine idiom lints are fixed
// at their sites instead. Generator-emitted style lints are allowed narrowly in
// generated.rs (see its module header), not here.
//
// C-ABI surface (ufbx.h mirrored verbatim):
#![allow(clippy::missing_safety_doc)] // unsafe fns mirror the ufbx.h C ABI; documenting each `# Safety` contract is a separate future effort
#![allow(clippy::not_unsafe_ptr_arg_deref)] // public fns take raw pointers by C-ABI design, matching ufbx.h signatures
#![allow(clippy::too_many_arguments)] // function arities are preserved verbatim from ufbx.c
#![allow(clippy::manual_c_str_literals)] // `b"...\0"` byte strings mirror C string handling; `c"..."` would be churn
#![allow(clippy::result_large_err)] // Error size mirrors ufbx_error; boxing it is a deferred design decision
// Byte-exact numeric fidelity (hash-oracle parity, see PORTING.md):
#![allow(clippy::excessive_precision)] // float literals are copied verbatim from ufbx.c for bit-exact results
#![allow(clippy::approx_constant)] // π/e/ln2/… are ufbx.c's literals, not std consts, to keep bits identical
// C control-flow / bit-expression structure preserved for line-by-line correspondence:
#![allow(clippy::manual_range_contains)] // explicit bound comparisons mirror the C source
#![allow(clippy::manual_is_multiple_of)] // `x % n == 0` mirrors C (and the lint is very new)
#![allow(clippy::manual_bits)] // bit-width expressions mirror C
#![allow(clippy::manual_rotate)] // explicit shift/or rotates mirror C bit ops in hash/deflate
#![allow(clippy::identity_op)] // `x | 0`, `<< 0` etc. preserve C bit-expression structure
#![allow(clippy::neg_cmp_op_on_partial_ord)] // `!(a < b)` preserves C float/NaN comparison semantics; `>=` would differ on NaN

pub mod generated;
pub mod prelude;

// Native port of ufbx.c, one module per C banner section (declared in ufbx.c
// order). Crate-internal: the public surface is the ufbx-rust API above it.
pub(crate) mod native;
// The ufbx.h surface with exact C signatures; the generated safe wrappers
// call these directly (no FFI). Under `c-abi` the symbols are additionally
// exported with C linkage for the upstream C test suite.
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
