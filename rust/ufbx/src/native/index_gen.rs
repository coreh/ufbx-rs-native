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
#![allow(dead_code)]

use crate::generated::{Error, RawAllocatorOpts, RawVertexStream};
#[cfg(feature = "index-gen")]
use crate::native::allocator::{
    align_to_mask, alloc, free, free_ator, init_ator, size_align_mask, Allocator,
};
#[cfg(feature = "index-gen")]
use crate::native::error::{clear_error, fix_error_type, ufbxi_fmt_err_info, ufbxi_report_err_msg};
#[cfg(not(feature = "index-gen"))]
use crate::native::error::{ufbxi_fmt_err_info, ufbxi_report_err_msg};
#[cfg(feature = "index-gen")]
use crate::native::hash::{
    hash_string, map_find_size, map_free, map_grow_size, map_init, map_insert_size, Map,
};
#[cfg(feature = "index-gen")]
use crate::native::platform::{add_ptr, to_size, ufbx_assert};
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
    let size: usize = *(user as *mut usize);
    #[cfg(feature = "regression")]
    ufbx_assert!(size % 8 == 0);
    let mut i: usize = 0;
    while i < size {
        let a: u64 = *((va as *const u8).add(i) as *const u64);
        let b: u64 = *((vb as *const u8).add(i) as *const u64);
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
    let mut ator: Allocator = MaybeUninit::<Allocator>::zeroed().assume_init();
    init_ator(error, &mut ator, allocator, b"allocator\0".as_ptr());

    let mut local_streams = MaybeUninit::<[VertexStream; LOCAL_STREAMS_COUNT]>::uninit(); // ufbxi_uninit
    let mut local_packed_vertex = MaybeUninit::<[u64; LOCAL_PACKED_VERTEX_COUNT]>::uninit(); // ufbxi_uninit
    let local_streams_ptr: *mut VertexStream = local_streams.as_mut_ptr() as *mut VertexStream;
    let local_packed_vertex_ptr: *mut u8 = local_packed_vertex.as_mut_ptr() as *mut u8;

    let mut streams: *mut VertexStream = core::ptr::null_mut();
    if num_streams > LOCAL_STREAMS_COUNT {
        streams = alloc::<VertexStream>(&mut ator, num_streams);
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
            if (*user_streams.add(i)).vertex_count < num_indices {
                ufbxi_fmt_err_info!(error, "%zu", i);
                ufbxi_report_err_msg!(
                    error,
                    "user_streams[i].vertex_count < num_indices",
                    "Truncated vertex stream"
                );
                fail = true;
                break;
            }

            let vertex_size: usize = (*user_streams.add(i)).vertex_size;
            let align: usize = size_align_mask(vertex_size);
            packed_size = align_to_mask(packed_size, align);
            // C: `streams[i].ptr = streams[i].begin = (char*)user_streams[i].data;`
            let begin: *mut u8 = (*user_streams.add(i)).data as *mut u8;
            (*streams.add(i)).begin = begin;
            (*streams.add(i)).ptr = begin;
            (*streams.add(i)).vertex_size = vertex_size;
            (*streams.add(i)).packed_offset = packed_size;
            packed_size = packed_size.wrapping_add(vertex_size);
            i += 1;
        }
        packed_size = align_to_mask(packed_size, 7);
    }

    if !fail && packed_size == 0 {
        ufbxi_report_err_msg!(error, "packed_size != 0", "Zero vertex size");
        fail = true;
    }

    let mut packed_vertex: *mut u8 = core::ptr::null_mut();
    if !fail {
        if packed_size > size_of::<[u64; LOCAL_PACKED_VERTEX_COUNT]>() {
            ufbx_assert!(packed_size % 8 == 0);
            packed_vertex = alloc::<u64>(&mut ator, packed_size / 8) as *mut u8;
            if packed_vertex.is_null() {
                fail = true;
            }
        } else {
            packed_vertex = local_packed_vertex_ptr;
        }
    }

    // C: `ufbxi_map map = { 0 };`
    let mut map: Map = MaybeUninit::<Map>::zeroed().assume_init();
    map_init(
        &mut map,
        &mut ator,
        map_cmp_vertex,
        &mut packed_size as *mut usize as *mut c_void,
    );

    if num_indices > 0 && !map_grow_size(&mut map, packed_size, num_indices) {
        fail = true;
    }

    if !fail {
        ufbx_assert!(!packed_vertex.is_null());
        core::ptr::write_bytes(packed_vertex, 0, packed_size);

        let mut i: usize = 0;
        while i < num_indices {
            let mut si: usize = 0;
            while si < num_streams {
                let size: usize = (*streams.add(si)).vertex_size;
                let offset: usize = (*streams.add(si)).packed_offset;
                let ptr: *mut u8 = (*streams.add(si)).ptr;
                core::ptr::copy_nonoverlapping(ptr, packed_vertex.add(offset), size);
                (*streams.add(si)).ptr = ptr.add(size);
                si += 1;
            }

            let hash: u32 = hash_string(packed_vertex, packed_size);
            let mut entry: *mut c_void =
                map_find_size(&mut map, packed_size, hash, packed_vertex as *const c_void);
            if entry.is_null() {
                entry =
                    map_insert_size(&mut map, packed_size, hash, packed_vertex as *const c_void);
                if entry.is_null() {
                    fail = true;
                    break;
                }
                core::ptr::copy_nonoverlapping(packed_vertex, entry as *mut u8, packed_size);
            }
            let index: u32 = (to_size((entry as *mut u8).offset_from(map.items as *mut u8))
                / packed_size) as u32;
            *indices.add(i) = index;
            i += 1;
        }
    }

    let mut result_vertices: usize = 0;
    if !fail {
        result_vertices = map.size as usize;

        let mut si: usize = 0;
        while si < num_streams {
            let vertex_size: usize = (*streams.add(si)).vertex_size;
            let mut dst: *mut u8 = (*streams.add(si)).begin;
            let mut src: *mut u8 = add_ptr(map.items as *mut u8, (*streams.add(si)).packed_offset);
            let mut i: usize = 0;
            while i < result_vertices {
                core::ptr::copy_nonoverlapping(src, dst, vertex_size);
                dst = dst.add(vertex_size);
                // C-parity: the final `src += packed_size` steps past the end of
                // the map item block by `packed_offset` bytes; `wrapping_add`
                // keeps that defined without changing the value.
                src = src.wrapping_add(packed_size);
                i += 1;
            }
            si += 1;
        }

        clear_error(error);
    } else {
        fix_error_type(
            error,
            b"Failed to generate indices\0".as_ptr(),
            core::ptr::null_mut(),
        );
    }

    if !streams.is_null() && streams != local_streams_ptr {
        free::<VertexStream>(&mut ator, streams, num_streams);
    }
    if !packed_vertex.is_null() && packed_vertex != local_packed_vertex_ptr {
        free::<u64>(&mut ator, packed_vertex as *mut u64, packed_size / 8);
    }

    map_free(&mut map);
    free_ator(&mut ator);

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
        core::ptr::write_bytes(error as *mut u8, 0, size_of::<Error>());
        ufbxi_fmt_err_info!(error, "UFBX_ENABLE_INDEX_GENERATION");
        ufbxi_report_err_msg!(error, "UFBXI_FEATURE_INDEX_GENERATION", "Feature disabled");
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
    use core::mem::MaybeUninit;

    fn vec3(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 { x, y, z }
    }

    #[test]
    fn empty_vertex_size_fails() {
        unsafe {
            let mut indices = [0u32; 9];
            let mut error = MaybeUninit::<crate::generated::Error>::uninit();
            let num_vertices = generate_indices(
                core::ptr::null(),
                0,
                indices.as_mut_ptr(),
                9,
                core::ptr::null(),
                error.as_mut_ptr(),
            );
            let error = error.assume_init();
            assert_eq!(num_vertices, 0);
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
            let mut error = MaybeUninit::<crate::generated::Error>::uninit();
            let num_vertices = generate_indices(
                streams.as_ptr(),
                1,
                indices.as_mut_ptr(),
                0,
                core::ptr::null(),
                error.as_mut_ptr(),
            );
            let error = error.assume_init();
            assert_eq!(num_vertices, 0);
            assert_eq!(error.type_, ErrorType::None);
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
            let mut error = MaybeUninit::<crate::generated::Error>::uninit();
            let num_vertices = generate_indices(
                streams.as_ptr(),
                1,
                indices.as_mut_ptr(),
                3,
                core::ptr::null(),
                error.as_mut_ptr(),
            );
            let error = error.assume_init();
            assert_eq!(num_vertices, 0);
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
                core::ptr::null_mut(),
            );
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
                core::ptr::null_mut(),
            );
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
                core::ptr::null_mut(),
            );
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
                core::ptr::null_mut(),
            );
            assert_eq!(num_wide, 2);
            assert_eq!(wide_indices, [0, 1]);
        }
    }
}
