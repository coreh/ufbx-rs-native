//! Port of the `// -- Utility` banner section (ufbx.c:30083-30241) — the
//! vertex-stream index generator backing `ufbx_generate_indices`.
//!
//! The section holds the map comparator `ufbxi_map_cmp_vertex`
//! (ufbx.c:30087-30099), the internal `ufbxi_vertex_stream` cursor struct
//! (30101-30105) and `ufbxi_generate_indices` (30107-30227). The `#else`
//! (feature-disabled) arm at ufbx.c:30231-30239 is ported here as well so the
//! module's contract is complete in both feature configurations.
//!
//! The remainder of the C banner section — `ufbxi_free_scene_imp`,
//! `ufbxi_init_ref` / `ufbxi_retain_ref` / `ufbxi_release_ref`,
//! `ufbxi_uninitialized_options` (ufbx.c:30243-30311) and the
//! `ufbxi_check_opts_*` macros (30313-30332) — lives in `native/api.rs` and
//! `native/error.rs` respectively, next to the entry points that use them.
//!
//! The whole `#if` branch is gated on `UFBXI_FEATURE_INDEX_GENERATION`
//! (`#[cfg(feature = "index-gen")]`).
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
use crate::generated::{Error, RawAllocatorOpts, RawVertexStream};
#[cfg(feature = "index-gen")]
use crate::native::allocator::{
    align_to_mask, alloc, free, free_ator, init_ator, size_align_mask, Allocator, AllocatorView,
};
#[cfg(feature = "index-gen")]
use crate::native::error::{clear_error, fix_error_type, ufbxi_fmt_err_info, ufbxi_report_err_msg};
#[cfg(not(feature = "index-gen"))]
use crate::native::error::{ufbxi_fmt_err_info, ufbxi_report_err_msg};
#[cfg(feature = "index-gen")]
use crate::native::hash::{
    hash_string, map_find_size, map_free, map_grow_size, map_init, map_insert_size, Map, MapView,
};
#[cfg(feature = "index-gen")]
use crate::native::platform::{add_ptr, to_size, ufbx_assert};
#[cfg(feature = "index-gen")]
use crate::native::view::{Const, View};
#[cfg(feature = "index-gen")]
use core::ffi::c_void;
use core::mem::size_of;
#[cfg(feature = "index-gen")]
use core::mem::MaybeUninit;

// ufbx.c:30087-30099 `ufbxi_map_cmp_vertex`
// C-parity: the `uint64_t` loads are typed (aligned) in C — both operands are
// 8-aligned in practice (`map.items` sits at a `ufbxi_alloc` base plus an
// 8-multiple offset, and `packed_vertex` is either a `uint64_t` array or a
// `uint64_t` allocation), and `packed_size` is rounded up to a multiple of 8.
#[cfg(feature = "index-gen")]
unsafe extern "C" fn map_cmp_vertex(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> i32 {
    // SAFETY: `map_init` receives `&packed_size` as the comparator user pointer
    // (ufbx.c:30163), so `user` addresses that live `usize`.
    let size: usize = unsafe { *(user as *mut usize) };
    #[cfg(feature = "regression")]
    ufbx_assert!(size % 8 == 0);
    let mut i: usize = 0;
    while i < size {
        // SAFETY: the map comparator contract gives `va`/`vb` as packed vertices
        // of `size` bytes each; `i + 8 <= size` since `size` is a multiple of 8
        // (`align_to_mask(packed_size, 7)`), and both blocks are 8-aligned (map
        // items sit at an 8-multiple offset from an `ufbxi_alloc` base, the probe
        // key is a `uint64_t` array or allocation).
        let a: u64 = unsafe { *((va as *const u8).add(i) as *const u64) };
        let b: u64 = unsafe { *((vb as *const u8).add(i) as *const u64) };
        if a != b {
            return if a < b { -1 } else { 1 };
        }
        i += 8;
    }
    0
}

// ufbx.c:30101-30105 `ufbxi_vertex_stream`
#[cfg(feature = "index-gen")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct VertexStream {
    pub begin: *mut u8,
    pub ptr: *mut u8,
    pub vertex_size: usize,
    pub packed_offset: usize,
}

// ufbx.c:30118 `ufbxi_arraycount(local_streams)`
#[cfg(feature = "index-gen")]
const LOCAL_STREAMS_COUNT: usize = 16;
// ufbx.c:30115 `uint64_t local_packed_vertex[64]`
#[cfg(feature = "index-gen")]
const LOCAL_PACKED_VERTEX_COUNT: usize = 64;

// ufbx.c:30107-30227 `ufbxi_generate_indices`
// `allow(unused_assignments)`: C's `ufbxi_vertex_stream *streams = NULL;`
// initializer (ufbx.c:30117) is kept even though both branches below assign it.
#[cfg(feature = "index-gen")]
#[allow(unused_assignments)]
#[inline(never)]
pub(crate) unsafe fn generate_indices(
    user_streams: *const RawVertexStream,
    num_streams: usize,
    indices: *mut u32,
    num_indices: usize,
    allocator: *const RawAllocatorOpts,
    error: *mut Error,
) -> usize {
    let mut fail = false;

    // C: `ufbxi_allocator ator = { 0 };`
    // SAFETY: all-zero bits are a valid `Allocator` — scalars, raw pointers
    // (null when zeroed), and the `Option<fn>` callbacks of the nested
    // `RawAllocatorOpts` (`None` when zeroed, via the null-fn niche), matching
    // the C zero-initializer.
    let mut ator: Allocator = unsafe { MaybeUninit::<Allocator>::zeroed().assume_init() };
    // SAFETY: a non-null `allocator` is the caller's live, initialized options
    // struct (this `unsafe fn`'s contract), written nowhere while the
    // read-only view is held; null is C's NULL, which `init_ator` answers with
    // the zeroed defaults.
    let allocator = if allocator.is_null() {
        None
    } else {
        Some(unsafe { View::<RawAllocatorOpts, Const>::from_ptr(allocator) })
    };
    init_ator(
        error,
        // SAFETY: the zeroed allocator above is this frame's live, unmoved
        // local.
        unsafe { AllocatorView::from_ptr(&raw mut ator) },
        allocator,
        c"allocator",
    );

    let mut local_streams = MaybeUninit::<[VertexStream; LOCAL_STREAMS_COUNT]>::uninit(); // ufbxi_uninit
    let mut local_packed_vertex = MaybeUninit::<[u64; LOCAL_PACKED_VERTEX_COUNT]>::uninit(); // ufbxi_uninit
    let local_streams_ptr: *mut VertexStream = local_streams.as_mut_ptr() as *mut VertexStream;
    let local_packed_vertex_ptr: *mut u8 = local_packed_vertex.as_mut_ptr() as *mut u8;

    let mut streams: *mut VertexStream = core::ptr::null_mut();
    if num_streams > LOCAL_STREAMS_COUNT {
        // SAFETY: `ator` is this frame's live, unmoved allocator, initialized
        // above.
        streams = alloc::<VertexStream>(
            unsafe { AllocatorView::from_ptr(&raw mut ator) },
            num_streams,
        );
        if streams.is_null() {
            fail = true;
        }
    } else {
        streams = local_streams_ptr;
    }

    // C-parity: `packed_size` is a local whose ADDRESS is handed to
    // `ufbxi_map_init` as the comparator's user pointer (ufbx.c:30163), so the
    // comparator reads the final value through that pointer.
    let mut packed_size: usize = 0;
    if !fail {
        let mut i: usize = 0;
        while i < num_streams {
            // SAFETY (this read and the field reads below): `user_streams`
            // addresses `num_streams` streams (fn raw-param contract) and
            // `i < num_streams`.
            if unsafe { (*user_streams.add(i)).vertex_count } < num_indices {
                // SAFETY: `error` is non-null and points at a live `Error` —
                // the sole caller (`api::generate_indices`) substitutes a local
                // error slot for null — so this is a write-capable mint of the
                // caller's slot; the single `%zu` conversion is matched by the
                // `usize` argument.
                unsafe {
                    ufbxi_fmt_err_info!(
                        Some(crate::native::error::ErrorView::from_ptr(error)),
                        "%zu",
                        i
                    )
                };
                ufbxi_report_err_msg!(
                    unsafe { crate::native::error::ErrorView::from_ptr(error) },
                    "user_streams[i].vertex_count < num_indices",
                    "Truncated vertex stream"
                );
                fail = true;
                break;
            }

            let vertex_size: usize = unsafe { (*user_streams.add(i)).vertex_size };
            let align: usize = size_align_mask(vertex_size);
            packed_size = align_to_mask(packed_size, align);
            // C: `streams[i].ptr = streams[i].begin = (char*)user_streams[i].data;`
            let begin: *mut u8 = unsafe { (*user_streams.add(i)).data as *mut u8 };
            // SAFETY (these four writes): `streams` addresses `num_streams`
            // `VertexStream` slots — either the local array (`num_streams <=
            // LOCAL_STREAMS_COUNT`) or the allocation checked non-null above —
            // and `i < num_streams`.
            unsafe { (*streams.add(i)).begin = begin };
            unsafe { (*streams.add(i)).ptr = begin };
            unsafe { (*streams.add(i)).vertex_size = vertex_size };
            unsafe { (*streams.add(i)).packed_offset = packed_size };
            packed_size = packed_size.wrapping_add(vertex_size);
            i += 1;
        }
        packed_size = align_to_mask(packed_size, 7);
    }

    if !fail && packed_size == 0 {
        ufbxi_report_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            "packed_size != 0",
            "Zero vertex size"
        );
        fail = true;
    }

    let mut packed_vertex: *mut u8 = core::ptr::null_mut();
    if !fail {
        if packed_size > size_of::<[u64; LOCAL_PACKED_VERTEX_COUNT]>() {
            ufbx_assert!(packed_size % 8 == 0);
            // SAFETY: `ator` is this frame's live, unmoved allocator,
            // initialized above.
            packed_vertex = alloc::<u64>(
                unsafe { AllocatorView::from_ptr(&raw mut ator) },
                packed_size / 8,
            ) as *mut u8;
            if packed_vertex.is_null() {
                fail = true;
            }
        } else {
            packed_vertex = local_packed_vertex_ptr;
        }
    }

    // C: `ufbxi_map map = { 0 };`
    // SAFETY: all-zero bits are a valid `Map` — scalars, raw pointers (null when
    // zeroed), the `bool`s of the nested `Buf` (`false`), and `cmp_fn`
    // (`Option<CmpFn>`, `None` when zeroed via the null-fn niche), matching the
    // C zero-initializer.
    let mut map: Map = unsafe { MaybeUninit::<Map>::zeroed().assume_init() };
    // SAFETY: the two views are minted over the zeroed map and the initialized
    // allocator, both locals that outlive every use of the map below. The user
    // pointer identifies `packed_size`, a live `usize` — the type
    // `map_cmp_vertex` reads through it — and a local that outlives every
    // comparator call (all of which happen below, before this fn returns).
    unsafe {
        map_init(
            MapView::from_ptr(&raw mut map),
            AllocatorView::from_ptr(&raw mut ator),
            map_cmp_vertex,
            (&raw mut packed_size).cast::<c_void>(),
        );
    }

    // SAFETY: the view is minted over the map initialized above, a local that
    // outlives the call; `packed_size` is the map's element stride, the same
    // stride `map_init` above set the comparator up for.
    if num_indices > 0
        && !unsafe { map_grow_size(MapView::from_ptr(&raw mut map), packed_size, num_indices) }
    {
        fail = true;
    }

    if !fail {
        ufbx_assert!(!packed_vertex.is_null());
        // SAFETY: `packed_vertex` addresses `packed_size` writable bytes — the
        // local buffer when `packed_size` fits it, else the allocation checked
        // non-null above.
        unsafe { core::ptr::write_bytes(packed_vertex, 0, packed_size) };

        let mut i: usize = 0;
        while i < num_indices {
            let mut si: usize = 0;
            while si < num_streams {
                // SAFETY (these three reads and the write below): `streams`
                // addresses `num_streams` filled `VertexStream` slots and
                // `si < num_streams`.
                let size: usize = unsafe { (*streams.add(si)).vertex_size };
                let offset: usize = unsafe { (*streams.add(si)).packed_offset };
                let ptr: *mut u8 = unsafe { (*streams.add(si)).ptr };
                // SAFETY: `ptr` walks the caller's stream, which holds at least
                // `num_indices` vertices of `size` bytes (checked above) and
                // `i < num_indices`; the destination sits at `offset` in the
                // `packed_size`-byte packed vertex, where `offset + size <=
                // packed_size` by construction of the offsets. The two blocks are
                // distinct allocations, hence non-overlapping.
                unsafe { core::ptr::copy_nonoverlapping(ptr, packed_vertex.add(offset), size) };
                // SAFETY: the advanced pointer stays within the stream (one
                // vertex per index, `i < num_indices <= vertex_count`), at worst
                // one past its end after the final index.
                unsafe { (*streams.add(si)).ptr = ptr.add(size) };
                si += 1;
            }

            // SAFETY: `packed_vertex` addresses `packed_size` readable bytes,
            // all initialized — zeroed by the `write_bytes` above, with the
            // stream loop overwriting the non-padding ranges each iteration.
            let hash: u32 = unsafe { hash_string(packed_vertex, packed_size) };
            // SAFETY: the mint addresses the initialized stack-local map, live and
            // unmoved for the rest of this fn; the map's item size is the
            // `packed_size` it was grown with, and the key addresses
            // `packed_size` readable bytes.
            let mut entry: *mut c_void = unsafe {
                map_find_size(
                    MapView::from_ptr(&raw mut map),
                    packed_size,
                    hash,
                    packed_vertex as *const c_void,
                )
            };
            if entry.is_null() {
                // SAFETY: same map and key contract as the lookup above.
                entry = unsafe {
                    map_insert_size(
                        MapView::from_ptr(&raw mut map),
                        packed_size,
                        hash,
                        packed_vertex as *const c_void,
                    )
                };
                if entry.is_null() {
                    fail = true;
                    break;
                }
                // SAFETY: a non-null insert result addresses a fresh
                // `packed_size`-byte item inside the map's own item block,
                // disjoint from the packed vertex buffer.
                unsafe {
                    core::ptr::copy_nonoverlapping(packed_vertex, entry as *mut u8, packed_size)
                };
            }
            // SAFETY: `entry` points into the map's item block, which starts at
            // `map.items` — both derived from the same allocation.
            let index: u32 =
                (to_size(unsafe { (entry as *mut u8).offset_from(map.items as *mut u8) })
                    / packed_size) as u32;
            // SAFETY: `indices` addresses `num_indices` writable `u32`s (fn
            // raw-param contract) and `i < num_indices`.
            unsafe { *indices.add(i) = index };
            i += 1;
        }
    }

    let mut result_vertices: usize = 0;
    if !fail {
        result_vertices = map.size as usize;

        let mut si: usize = 0;
        while si < num_streams {
            // SAFETY (these three field reads): `streams` addresses
            // `num_streams` filled `VertexStream` slots and `si < num_streams`.
            let vertex_size: usize = unsafe { (*streams.add(si)).vertex_size };
            let mut dst: *mut u8 = unsafe { (*streams.add(si)).begin };
            let mut src: *mut u8 = add_ptr(map.items as *mut u8, unsafe {
                (*streams.add(si)).packed_offset
            });
            let mut i: usize = 0;
            while i < result_vertices {
                // SAFETY: `src` walks the map's `result_vertices` items of
                // `packed_size` bytes, reading the `vertex_size` bytes this
                // stream packed at `packed_offset`; `dst` walks the caller's
                // stream, which holds at least `num_indices >= result_vertices`
                // vertices. Distinct allocations, hence non-overlapping.
                unsafe { core::ptr::copy_nonoverlapping(src, dst, vertex_size) };
                // SAFETY: one vertex per compacted index keeps `dst` inside the
                // caller's stream, at worst one past its end after the last.
                dst = unsafe { dst.add(vertex_size) };
                // C-parity: the final `src += packed_size` steps past the end of
                // the map item block by `packed_offset` bytes; `wrapping_add`
                // keeps that defined without changing the value.
                src = src.wrapping_add(packed_size);
                i += 1;
            }
            si += 1;
        }

        // SAFETY: `error` is non-null and points at a live `Error` — the sole
        // caller (`api::generate_indices`) substitutes a local error slot for
        // null — so this is a write-capable mint of the caller's slot.
        let err = unsafe { crate::native::error::ErrorView::from_ptr(error) };
        clear_error(Some(err));
    } else {
        // SAFETY: `error` is non-null and points at a live `Error` — the sole
        // caller (`api::generate_indices`) substitutes a local error slot for
        // null — so this is a write-capable mint of the caller's slot.
        let err = unsafe { crate::native::error::ErrorView::from_ptr(error) };
        fix_error_type(err, b"Failed to generate indices\0", None);
    }

    if !streams.is_null() && streams != local_streams_ptr {
        // SAFETY: `ator` is this frame's live, unmoved `Allocator` local, and
        // the guards single out the `alloc::<VertexStream>(&raw mut ator,
        // num_streams)` result above, returned to the same allocator with the
        // count it was allocated with.
        unsafe {
            free::<VertexStream>(
                Some(AllocatorView::from_ptr(&raw mut ator)),
                streams,
                num_streams,
            )
        };
    }
    if !packed_vertex.is_null() && packed_vertex != local_packed_vertex_ptr {
        // SAFETY: `ator` is this frame's live, unmoved `Allocator` local, and
        // the guards single out the `alloc::<u64>(&raw mut ator,
        // packed_size / 8)` result above, returned to the same allocator with the
        // count it was allocated with (`packed_size` is unchanged since).
        unsafe {
            free::<u64>(
                Some(AllocatorView::from_ptr(&raw mut ator)),
                packed_vertex as *mut u64,
                packed_size / 8,
            )
        };
    }

    // SAFETY: `map` and `ator` are the live initialized locals above, and the map
    // is freed through the allocator that owns its storage before that allocator
    // itself is torn down.
    map_free(unsafe { MapView::from_ptr(&raw mut map) });
    // SAFETY: `ator` is that same live, unmoved local allocator, torn down
    // exactly once here.
    unsafe { free_ator(AllocatorView::from_ptr(&raw mut ator)) };

    result_vertices
}

// ufbx.c:30231-30239 `ufbxi_generate_indices` (`#else` branch — feature disabled)
#[cfg(not(feature = "index-gen"))]
#[inline(never)]
pub(crate) unsafe fn generate_indices(
    user_streams: *const RawVertexStream,
    num_streams: usize,
    indices: *mut u32,
    num_indices: usize,
    allocator: *const RawAllocatorOpts,
    error: *mut Error,
) -> usize {
    let _ = (user_streams, num_streams, indices, num_indices, allocator);
    if !error.is_null() {
        // SAFETY: `error` is the caller's error slot, non-null per the guard, so
        // it addresses `size_of::<Error>()` writable bytes; all-zero bits are a
        // valid `Error` (C: `memset` of the same struct).
        unsafe { core::ptr::write_bytes(error as *mut u8, 0, size_of::<Error>()) };
        // SAFETY: `error` addresses the zeroed error struct above (non-null
        // per the guard), so this is a write-capable mint of the caller's slot;
        // the format string is a literal with no conversions.
        unsafe {
            ufbxi_fmt_err_info!(
                Some(crate::native::error::ErrorView::from_ptr(error)),
                "UFBX_ENABLE_INDEX_GENERATION"
            )
        };
        ufbxi_report_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            "UFBXI_FEATURE_INDEX_GENERATION",
            "Feature disabled"
        );
    }
    0
}

// White-box counterparts of the `ufbx_generate_indices` cases in
// `test/test_topology.h` (`generate_indices_empty_vertex` 425-434,
// `generate_indices_no_indices` 436-453, `generate_indices_truncated_stream`
// 455-473) plus dedup coverage for the single- and multi-stream paths.
#[cfg(all(test, feature = "index-gen"))]
mod tests {
    use crate::generated::{ErrorType, Vec3};
    use crate::native::api::generate_indices;
    use crate::prelude::Real;

    fn vec3(x: Real, y: Real, z: Real) -> Vec3 {
        Vec3 { x, y, z }
    }

    #[test]
    fn empty_vertex_size_fails() {
        unsafe {
            let mut indices = [0u32; 9];
            let error = generate_indices(
                core::ptr::null(),
                0,
                indices.as_mut_ptr(),
                9,
                core::ptr::null(),
            )
            .unwrap_err();
            // An `Err` implies C's zero count (`result_vertices` is only
            // assigned on the success arm).
            assert_eq!(error.type_, ErrorType::ZeroVertexSize);
        }
    }

    #[test]
    fn zero_indices_succeeds() {
        unsafe {
            let mut vertices = [
                vec3(0.0, 0.0, 0.0),
                vec3(1.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
            ];
            let streams = [crate::generated::RawVertexStream {
                data: vertices.as_mut_ptr() as *mut core::ffi::c_void,
                vertex_count: 3,
                vertex_size: core::mem::size_of::<Vec3>(),
            }];
            let mut indices = [0u32; 9];
            let num_vertices = generate_indices(
                streams.as_ptr(),
                1,
                indices.as_mut_ptr(),
                0,
                core::ptr::null(),
            )
            .unwrap();
            assert_eq!(num_vertices, 0);
        }
    }

    #[test]
    fn truncated_stream_reports_index() {
        unsafe {
            let mut vertices = [
                vec3(0.0, 0.0, 0.0),
                vec3(1.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
            ];
            let streams = [crate::generated::RawVertexStream {
                data: vertices.as_mut_ptr() as *mut core::ffi::c_void,
                vertex_count: 2,
                vertex_size: core::mem::size_of::<Vec3>(),
            }];
            let mut indices = [0u32; 9];
            let error = generate_indices(
                streams.as_ptr(),
                1,
                indices.as_mut_ptr(),
                3,
                core::ptr::null(),
            )
            .unwrap_err();
            // An `Err` implies C's zero count (`result_vertices` is only
            // assigned on the success arm).
            assert_eq!(error.type_, ErrorType::TruncatedVertexStream);
            assert_eq!(error.info(), "0");
        }
    }

    #[test]
    fn single_stream_deduplicates_and_compacts() {
        unsafe {
            // Two distinct vertices repeated in a 6-entry stream.
            let a = vec3(1.0, 2.0, 3.0);
            let b = vec3(4.0, 5.0, 6.0);
            let mut vertices = [a, b, a, b, a, b];
            let streams = [crate::generated::RawVertexStream {
                data: vertices.as_mut_ptr() as *mut core::ffi::c_void,
                vertex_count: 6,
                vertex_size: core::mem::size_of::<Vec3>(),
            }];
            let mut indices = [0u32; 6];
            let num_vertices = generate_indices(
                streams.as_ptr(),
                1,
                indices.as_mut_ptr(),
                6,
                core::ptr::null(),
            )
            .unwrap();
            assert_eq!(num_vertices, 2);
            assert_eq!(indices, [0, 1, 0, 1, 0, 1]);
            assert_eq!(vertices[0].x, 1.0);
            assert_eq!(vertices[1].x, 4.0);
        }
    }

    #[test]
    fn multi_stream_packs_both_attributes() {
        unsafe {
            // Positions repeat, normals differ — the pair is what dedups.
            let mut positions = [
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 0.0, 0.0),
            ];
            let mut normals = [
                vec3(1.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
                vec3(1.0, 0.0, 0.0),
            ];
            let streams = [
                crate::generated::RawVertexStream {
                    data: positions.as_mut_ptr() as *mut core::ffi::c_void,
                    vertex_count: 3,
                    vertex_size: core::mem::size_of::<Vec3>(),
                },
                crate::generated::RawVertexStream {
                    data: normals.as_mut_ptr() as *mut core::ffi::c_void,
                    vertex_count: 3,
                    vertex_size: core::mem::size_of::<Vec3>(),
                },
            ];
            let mut indices = [0u32; 3];
            let num_vertices = generate_indices(
                streams.as_ptr(),
                2,
                indices.as_mut_ptr(),
                3,
                core::ptr::null(),
            )
            .unwrap();
            assert_eq!(num_vertices, 2);
            assert_eq!(indices, [0, 1, 0]);
            assert_eq!(normals[0].x, 1.0);
            assert_eq!(normals[1].y, 1.0);
        }
    }

    #[test]
    fn heap_paths_for_many_streams_and_large_vertices() {
        unsafe {
            // > 16 streams forces the `ufbxi_alloc` stream array, and 20 f64
            // streams pack to 160 bytes/vertex — under the 512-byte local
            // packed vertex buffer, so also exercise a wide single stream.
            const N: usize = 20;
            let mut data: [[f64; 2]; N] = [[0.0; 2]; N];
            let mut streams: [crate::generated::RawVertexStream; N] =
                core::mem::zeroed::<[crate::generated::RawVertexStream; N]>();
            for si in 0..N {
                data[si] = [si as f64, 0.0];
                streams[si] = crate::generated::RawVertexStream {
                    data: data[si].as_mut_ptr() as *mut core::ffi::c_void,
                    vertex_count: 2,
                    vertex_size: core::mem::size_of::<f64>(),
                };
            }
            let mut indices = [0u32; 2];
            let num_vertices = generate_indices(
                streams.as_ptr(),
                N,
                indices.as_mut_ptr(),
                2,
                core::ptr::null(),
            )
            .unwrap();
            assert_eq!(num_vertices, 2);
            assert_eq!(indices, [0, 1]);

            // Wide single stream: 520 bytes/vertex exceeds the 512-byte
            // `local_packed_vertex`, taking the `ufbxi_alloc(uint64_t)` path.
            let mut wide = [[0u64; 65]; 2];
            wide[1][0] = 1;
            let wide_streams = [crate::generated::RawVertexStream {
                data: wide.as_mut_ptr() as *mut core::ffi::c_void,
                vertex_count: 2,
                vertex_size: 65 * core::mem::size_of::<u64>(),
            }];
            let mut wide_indices = [0u32; 2];
            let num_wide = generate_indices(
                wide_streams.as_ptr(),
                1,
                wide_indices.as_mut_ptr(),
                2,
                core::ptr::null(),
            )
            .unwrap();
            assert_eq!(num_wide, 2);
            assert_eq!(wide_indices, [0, 1]);
        }
    }
}
