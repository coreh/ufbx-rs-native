#![cfg_attr(feature = "nightly", feature(likely_unlikely))]
#![allow(unused_braces)]
// Crate-wide clippy allows for lints that are a poor fit for a line-faithful C
// port of ufbx.c, in the current state of the port. Each is a deliberate,
// pervasive stance rather than a fixable one-off; genuine idiom lints are fixed
// at their sites instead. Generator-emitted style lints are allowed narrowly in
// generated.rs (see its module header), not here.
//
// -- C-ABI surface (ufbx.h mirrored verbatim) --
// unsafe fns mirror the ufbx.h C ABI; documenting each `# Safety` contract is a separate future effort.
#![allow(clippy::missing_safety_doc)]
// public fns take raw pointers by C-ABI design, matching ufbx.h signatures.
#![allow(clippy::not_unsafe_ptr_arg_deref)]
// function arities are preserved verbatim from ufbx.c.
#![allow(clippy::too_many_arguments)]
// `b"...\0"` byte strings mirror C string handling; `c"..."` would be churn.
#![allow(clippy::manual_c_str_literals)]
// Error size mirrors ufbx_error; boxing it is a deferred design decision.
#![allow(clippy::result_large_err)]
// -- Byte-exact numeric fidelity (hash-oracle parity, see PORTING.md) --
// float literals are copied verbatim from ufbx.c for bit-exact results.
#![allow(clippy::excessive_precision)]
// π/e/ln2/… are ufbx.c's literals, not std consts, to keep bits identical.
#![allow(clippy::approx_constant)]
// (clippy::unnecessary_cast is deliberately NOT allowed here: hand-written Real→f64 promotions use the
// `as_f64!` macro, which suppresses the lint at those sites while keeping it live everywhere else to catch
// genuinely redundant casts.)
// -- C control-flow / bit-expression structure preserved for line-by-line correspondence --
// explicit bound comparisons mirror the C source.
#![allow(clippy::manual_range_contains)]
// `x % n == 0` mirrors C (and the lint is very new).
#![allow(clippy::manual_is_multiple_of)]
// bit-width expressions mirror C.
#![allow(clippy::manual_bits)]
// explicit shift/or rotates mirror C bit ops in hash/deflate.
#![allow(clippy::manual_rotate)]
// `x | 0`, `<< 0` etc. preserve C bit-expression structure.
#![allow(clippy::identity_op)]
// `!(a < b)` preserves C float/NaN comparison semantics; `>=` would differ on NaN.
#![allow(clippy::neg_cmp_op_on_partial_ord)]
// nested `if`s mirror ufbx.c's conditional structure line-for-line.
#![allow(clippy::collapsible_if)]
// explicit branches mirror the C source's per-name/per-version handling even where arms currently coincide.
#![allow(clippy::if_same_then_else)]
// `!(a >= b)`, `!(a < b)` etc. preserve the C source's boolean spelling (overflow/loop guards).
#![allow(clippy::nonminimal_bool)]
// `T x; ... x = v;` mirrors C's declare-then-assign; the remaining sites are conditional inits.
#![allow(clippy::needless_late_init)]
// `for i in 0..n` mirrors C index loops that also index raw pointers / a second array (not iterable).
#![allow(clippy::needless_range_loop)]
// Unsafe-reduction ratchet (PORTING.md "Unsafe reduction / isolation strategy"):
// every op inside an `unsafe fn` must sit in an explicit narrow `unsafe {}`
// block. Files not yet converted carry a file-level allow; a cleaned file
// deletes its allow, and new code is gated from the start.
#![warn(unsafe_op_in_unsafe_fn)]

// Generated code keeps C-shaped implicit unsafe fn bodies; the generator owns
// its emission style (NEVER hand-edit generated.rs — see PORTING.md rule 0).
#[allow(unsafe_op_in_unsafe_fn)]
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
    let opts_raw = RawOpenMemoryOpts::to_raw_mut(&mut opts_mut);
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
pub fn triangulate_face_vec(indices: &mut Vec<u32>, mesh: &Mesh, face: Face) -> u32 {
    if face.num_indices < 3 {
        indices.clear();
        return 0;
    }

    let num_triangles = face.num_indices as usize - 2;
    indices.resize(num_triangles * 3, 0);
    let num_triangles = triangulate_face(indices, mesh, face);
    indices.shrink_to(num_triangles as usize * 3);
    num_triangles
}
