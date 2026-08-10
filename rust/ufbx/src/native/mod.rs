//! 1:1 port of ufbx.c, one module per C banner section, declared in ufbx.c order.
//! See PORTING.md for the C→Rust pattern map and the function→module routing table.

// -- extra/ufbx_math.c (not ufbx.c): ufbx's own freestanding libm, which the
//    C oracle builds link via UFBX_EXTERNAL_MATH; backs the ufbx.c:257-276
//    math shim in `platform::math` (PORTING.md "Floats").
pub mod math;
// -- Platform / Atomic counter / Bit manipulation / Utility
pub mod platform;
// -- Rust-port infrastructure (not a ufbx.c section): reinterpret-in-place views
//    + safe iteration over contiguous arena arrays (see module docs).
pub mod view;
// -- Float parsing (hand-rolled strtod + bigint)
pub mod float_parse;
// -- DEFLATE implementation
pub mod deflate;
// -- Printf
pub mod printf;
// -- Errors
pub mod error;
// -- Allocator
pub mod allocator;
// -- Memory buffer (ufbxi_buf)
pub mod buf;
// -- Hash map / Hash functions
pub mod hash;
// -- Warnings
pub mod warnings;
// -- String pool / String constants
pub mod string_pool;
// -- Threading
pub mod thread;
// -- IO / File IO / Memory IO
pub mod io;
// -- XML (geometry cache only)
pub mod xml;
// -- Parsing state machine / Binary parsing
pub mod parse_binary;
// -- ASCII parsing
pub mod parse_ascii;
// -- DOM retention / General parsing / Setup
pub mod parse;
// -- Reading the parsed data / Pre-7000 "Take" animation
pub mod read;
// -- .obj file parser
pub mod obj;
// -- Scene pre-processing / Scene processing / Property updates
pub mod scene_process;
// -- Geometry caches / External files
pub mod cache;
// -- Curve evaluation / Animation evaluation / Baking
pub mod evaluate;
// -- NURBS / Tessellation
pub mod nurbs;
// -- Topology / KD tree / Triangulation
pub mod topology;
// -- Subdivision
pub mod subdivision;
// -- Index generation / Utility
pub mod index_gen;
// -- API entry points (thin wrappers over the above)
pub mod api;
