//! Port of the `// -- Memory buffer` banner section (ufbx.c:3817-4354).
//!
//! C comment (ufbx.c:3817-3821):
//! General purpose memory buffer that can be used either as a chunked linear memory
//! allocator or a non-contiguous stack. You can convert the contents of `ufbxi_buf`
//! to a contiguous range of memory by calling `ufbxi_make_array[_all]()`
//!
//! See PORTING.md "Allocator + ufbxi_buf": chunk geometry identical (growth
//! doubling, align rounding); `ufbxi_buf_chunk` flexible array member becomes a
//! header-only struct + pointer arithmetic; `UFBXI_HUGE_MAX_SCAN` and
//! `ator->huge_size` are two different mechanisms — both kept.
//!
//! Phase 1: not all items have consumers yet.
#![allow(dead_code)]

use core::ffi::c_void;
use core::mem::size_of;

use crate::native::allocator::{
    align_to_mask, alloc_size, does_overflow, free_size, size_align_mask, Allocator,
    BUF_CHUNK_IMP_MAGIC, ZERO_SIZE_BUFFER,
};
#[cfg(feature = "regression")]
use crate::native::error::ufbxi_check_return_err_msg;
use crate::native::platform::{ufbx_assert, ufbxi_regression_assert};

// ufbx.c:57 `#define UFBXI_HUGE_MAX_SCAN 16` (no UFBX_REGRESSION override)
pub(crate) const HUGE_MAX_SCAN: usize = 16;

// ufbx.c:3826-3829 `ufbxi_buf_padding`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct BufPadding {
    pub original_pos: usize, // < Original position before aligning
    pub prev_padding: usize, // < Starting offset of the previous `ufbxi_buf_padding`
}

// ufbx.c:3831-3849 `ufbxi_buf_chunk`
// C ends with a flexible array member `char data[];` — the Rust struct holds
// the header only; `data` is reached by pointer arithmetic from the header end
// (`chunk_data()` below). `size_of::<BufChunk>()` == C header size, pinned by
// the const asserts after the struct.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct BufChunk {
    // Linked list of nodes
    pub root: *mut BufChunk,
    pub prev: *mut BufChunk,
    pub next: *mut BufChunk,

    // C: union { size_t magic; void *align_0; } (ufbx.c:3838-3841) — the
    // `align_0` member is an alignment device ("Align to 4x pointer size
    // (16/32 bytes)"); `size_t` and `void*` coincide in size/alignment on all
    // supported targets, so a plain `usize` field maps the union.
    pub magic: usize, // < Magic for debugging

    pub size: usize,       // < Size of the chunk `data`, excluding this header
    pub pushed_pos: usize, // < Size of valid data when pushed to the list
    pub next_size: usize,  // < Next geometrically growing chunk size to allocate
    pub padding_pos: usize, // < One past the offset of the most recent `ufbxi_buf_padding`
                           // char data[]; // < Must be aligned to 8 bytes
}

// C header size: 3 pointers + union(size_t/void*) + 4 size_t = 8 words.
const _: () = assert!(size_of::<BufChunk>() == 8 * size_of::<usize>());
// ufbx.c:3851 `ufbx_static_assert(buf_chunk_align, offsetof(ufbxi_buf_chunk, data) % 8 == 0);`
// `data` begins at `size_of::<BufChunk>()` in the Rust mapping.
const _: () = assert!(size_of::<BufChunk>() % 8 == 0);
// The padding record written by `ufbxi_push_size` occupies a hardcoded 16
// bytes (ufbx.c:4056-4058); the struct must fit in it.
const _: () = assert!(size_of::<BufPadding>() <= 16);

// ufbx.c:3848 `char data[]` — flexible-array-member accessor.
#[inline(always)]
pub(crate) unsafe fn chunk_data(chunk: *mut BufChunk) -> *mut u8 {
    (chunk as *mut u8).add(size_of::<BufChunk>())
}

// ufbx.c:3853-3870 `ufbxi_buf`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Buf {
    pub ator: *mut Allocator,

    // Current chunks for normal and huge allocations.
    // Ordered buffers (`!ufbxi_buf.unordered`) never use `chunks[1]`
    pub chunks: [*mut BufChunk; 2],

    // Inline state for non-huge chunks
    pub pos: usize,  // < Next offset to allocate from
    pub size: usize, // < Size of the current chunk ie. `chunks[0]->size` (or 0 if `chunks[0] == NULL`)

    pub num_items: usize, // < Number of individual items pushed to the buffer

    pub pushed_size: usize, // < Cumulative size of pushed chunks, not tracked across pops

    pub unordered: bool, // < Does not support popping from the buffer
    pub clearable: bool, // < Supports clearing the whole buffer even if `unordered`
}

// ufbx.c:3872-3876 `ufbxi_buf_state`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct BufState {
    pub chunk: *mut BufChunk,
    pub pos: usize,
    pub num_items: usize,
}

// ufbx.c:3878-4020 `ufbxi_push_size_new_block`
#[inline(never)]
pub(crate) unsafe fn push_size_new_block(b: *mut Buf, size: usize) -> *mut c_void {
    let huge = size >= (*(*b).ator).huge_size;

    // Use the second chunk "list" for huge unordered chunks.
    // The state of these chunks is not tracked by `ufbxi_buf.pos/size`.
    let list_ix: u32 = ((*b).unordered as u32) & (huge as u32);

    let mut chunk = (*b).chunks[list_ix as usize];
    if !chunk.is_null() {
        if list_ix == 0 {
            // Store the final position for the retired chunk and scan free
            // chunks in case we find one the allocation fits in.
            (*b).pushed_size += (*b).pos;
            (*chunk).pushed_pos = (*b).pos;
            let mut next = (*chunk).next;
            while !next.is_null() {
                ufbx_assert!((*next).magic == BUF_CHUNK_IMP_MAGIC as usize);
                chunk = next;
                ufbx_assert!((*b).unordered || (*chunk).pushed_pos == 0);
                (*chunk).pushed_pos = 0;
                if size <= (*chunk).size {
                    (*b).chunks[0] = chunk;
                    // C-parity: C truncates through `(uint32_t)size` here (ufbx.c:3901).
                    (*b).pos = size as u32 as usize;
                    (*b).size = (*chunk).size;
                    return chunk_data(chunk) as *mut c_void;
                }
                next = (*chunk).next;
            }
        } else if (*b).clearable {
            // Keep track of the `UFBXI_HUGE_MAX_SCAN` largest chunks and
            // retain them. Overflowing chunks are freed in `ufbxi_buf_clear()`
            let align_mask = size_align_mask(size);
            let mut next = chunk;

            let mut best_chunk: *mut BufChunk = core::ptr::null_mut();
            let mut best_space = usize::MAX;

            // Clearable huge chunks are sorted by descending size. Check the first N
            // chunks for reuse and find the place a new block should be inserted if
            // no suitable space is found. Chunk ordering in the tail doesn't matter
            // as those chunks are never reused.
            // Unreachable chunks in the tail are freed in `ufbxi_buf_clear()`.
            let mut i = 0usize;
            while !next.is_null() && i < HUGE_MAX_SCAN {
                ufbx_assert!((*next).magic == BUF_CHUNK_IMP_MAGIC as usize);
                if (*next).size < size {
                    break;
                }
                chunk = next;

                // Try to reuse chunks using a best-fit strategy.
                let pos = align_to_mask((*chunk).pushed_pos, align_mask);
                // C-parity: unsigned wrap when `pos > chunk->size` (over-aligned
                // position past the chunk end) would yield a huge `space`, making
                // the `size <= space` check below PASS — identical to C
                // (ufbx.c:3928-3929). Unreachable in practice: chunk sizes are
                // 16-aligned and `align_mask <= 15`, so `pos <= chunk->size`
                // always. Do NOT "fix" this with checked_sub in the
                // unsafe-reduction phase — that would diverge from C.
                let space = (*chunk).size.wrapping_sub(pos);
                if size <= space {
                    if space < best_space {
                        best_chunk = chunk;
                        best_space = space;
                    }
                }

                next = (*chunk).next;
                i += 1;
            }

            // Early return if we found a slot.
            if !best_chunk.is_null() {
                let pos = align_to_mask((*best_chunk).pushed_pos, align_mask);
                (*best_chunk).pushed_pos = pos + size;
                (*b).pushed_size += size;
                return chunk_data(best_chunk).add(pos) as *mut c_void;
            }
        }
    }

    // Allocate a new chunk, grow `next_size` geometrically but don't double
    // the current or previous user sizes if they are larger.
    let mut chunk_size: usize;
    let mut next_size: usize;

    // If `size` is larger than `huge_size` don't grow `next_size` geometrically,
    // but use a dedicated allocation.
    if huge {
        next_size = if !chunk.is_null() {
            (*chunk).next_size
        } else {
            4096
        };
        if next_size > (*(*b).ator).chunk_max {
            next_size = (*(*b).ator).chunk_max;
        }
        chunk_size = size;
    } else {
        next_size = if !chunk.is_null() {
            (*chunk).next_size.wrapping_mul(2)
        } else {
            4096
        };
        if next_size > (*(*b).ator).chunk_max {
            next_size = (*(*b).ator).chunk_max;
        }
        // C-parity: unsigned wrap if `next_size < sizeof(ufbxi_buf_chunk)`
        // (tiny user-provided `max_chunk_size`); the `< size` fix-up below and
        // the overflow checks in `ufbxi_alloc_size` behave as in C.
        chunk_size = next_size.wrapping_sub(size_of::<BufChunk>());
        if chunk_size < size {
            chunk_size = size;
        }
    }

    // Align chunk sizes to 16 bytes
    chunk_size = align_to_mask(chunk_size, 0xf);

    let new_chunk =
        alloc_size((*b).ator, 1, size_of::<BufChunk>().wrapping_add(chunk_size)) as *mut BufChunk;
    if new_chunk.is_null() {
        return core::ptr::null_mut();
    }

    (*new_chunk).prev = chunk;
    (*new_chunk).size = chunk_size;
    (*new_chunk).next_size = next_size;
    (*new_chunk).magic = BUF_CHUNK_IMP_MAGIC as usize;
    (*new_chunk).padding_pos = 0;
    (*new_chunk).pushed_pos = 0;

    // Link the chunk to the list and set it as the active one
    if !chunk.is_null() {
        let next = (*chunk).next;
        if !next.is_null() {
            (*next).prev = new_chunk;
        }
        (*new_chunk).next = next;
        (*chunk).next = new_chunk;
        (*new_chunk).root = (*chunk).root;
    } else {
        (*new_chunk).next = core::ptr::null_mut();
        (*new_chunk).root = new_chunk;
    }

    if list_ix == 0 {
        (*b).chunks[0] = new_chunk;
        (*b).pos = size;
        (*b).size = chunk_size;
    } else {
        let root = (*b).chunks[1];
        (*b).pushed_size += size;
        if root.is_null() {
            (*b).chunks[1] = new_chunk;
        } else if (*root).size < chunk_size {
            // Swap root and self if necessary, we should have bailed out
            // in the search loop in the first iteration so `new_chunk` should
            // directly follow `root`.
            // HACK: This ends up with `chunks[1]` entries having inconsistent
            // `ufbxi_buf_chunk.root` pointers but other code only reads `chunks[1].root`
            // TODO: Move roots out of the chunks?
            ufbx_assert!((*root).next == new_chunk);
            ufbx_assert!((*new_chunk).prev == root);
            if !(*new_chunk).next.is_null() {
                (*(*new_chunk).next).prev = root;
            }
            (*root).next = (*new_chunk).next;
            (*new_chunk).next = root;
            (*new_chunk).prev = core::ptr::null_mut();
            (*new_chunk).root = new_chunk;
            (*b).chunks[1] = new_chunk;
        }
        (*new_chunk).pushed_pos = size;
    }

    chunk_data(new_chunk) as *mut c_void
}

// ufbx.c:4022-4072 `ufbxi_push_size`
#[inline(never)]
pub(crate) unsafe fn push_size(b: *mut Buf, size: usize, n: usize) -> *mut c_void {
    // Always succeed with an empty non-NULL buffer for empty allocations
    ufbx_assert!(size > 0);
    if n == 0 {
        return ZERO_SIZE_BUFFER.as_ptr() as *mut c_void;
    }

    let total = size.wrapping_mul(n);
    if does_overflow(total, size, n) {
        return core::ptr::null_mut();
    }

    #[cfg(feature = "regression")]
    {
        let ator = (*b).ator;
        ufbxi_check_return_err_msg!(
            (*ator).error,
            (*ator).num_allocs < (*ator).max_allocs,
            core::ptr::null_mut(),
            "Allocation limit exceeded",
            "ator->num_allocs < ator->max_allocs"
        );
        (*ator).num_allocs += 1;
    }

    // C-parity: size_t add wraps in C; keep wrapping semantics (checklist #2).
    (*b).num_items = (*b).num_items.wrapping_add(n);

    // Align to the natural alignment based on the size
    let align_mask = size_align_mask(size);
    let pos = align_to_mask((*b).pos, align_mask);

    if !(*b).unordered && pos != (*b).pos {
        // Alignment mismatch in an unordered block. Align to 16 bytes to guarantee
        // sufficient alignment for anything afterwards and mark the padding.
        // If we overflow the current block we don't need to care as the block
        // boundaries are not contiguous.
        let pos = align_to_mask((*b).pos, 0xf);
        // C-parity: `b->size - pos` (ufbx.c:4051) wraps if the 16-aligned
        // position passed the chunk end, yielding a huge value that would make
        // the `<=` check PASS — identical to C. Unreachable in practice: chunk
        // sizes are 16-aligned, so `pos <= b->size` always. Do NOT replace with
        // checked_sub-then-bail in the unsafe-reduction phase.
        if total < usize::MAX - 16 && total + 16 <= (*b).size.wrapping_sub(pos) {
            let chunk = (*b).chunks[0];
            let padding = chunk_data(chunk).add(pos) as *mut BufPadding;
            (*padding).original_pos = (*b).pos;
            (*padding).prev_padding = (*chunk).padding_pos;
            (*chunk).padding_pos = pos + 16 + 1;
            (*b).pos = pos + 16 + total;
            (padding as *mut u8).add(16) as *mut c_void
        } else {
            push_size_new_block(b, total)
        }
    } else {
        // Try to push to the current block. Allocate a new block
        // if the aligned size doesn't fit.
        // C-parity: `b->size - pos` (ufbx.c:4065) wraps when `pos > b->size`
        // (aligned past the chunk end), yielding a huge value that would make
        // the `<=` check PASS — identical to C. Unreachable in practice: chunk
        // sizes are 16-aligned and `align_mask <= 15`, so `pos <= b->size`
        // always. Do NOT replace with checked_sub-then-bail in the
        // unsafe-reduction phase.
        if total <= (*b).size.wrapping_sub(pos) {
            (*b).pos = pos + total;
            chunk_data((*b).chunks[0]).add(pos) as *mut c_void
        } else {
            push_size_new_block(b, total)
        }
    }
}

// ufbx.c:4074-4105 `ufbxi_push_size_fast`
#[inline(always)]
pub(crate) unsafe fn push_size_fast(b: *mut Buf, size: usize, n: usize) -> *mut c_void {
    // Always succeed with an empty non-NULL buffer for empty allocations
    ufbxi_regression_assert!(size > 0);
    ufbxi_regression_assert!(n > 0);

    let total = size.wrapping_mul(n);
    ufbxi_regression_assert!(!does_overflow(total, size, n));

    #[cfg(feature = "regression")]
    {
        let ator = (*b).ator;
        ufbxi_check_return_err_msg!(
            (*ator).error,
            (*ator).num_allocs < (*ator).max_allocs,
            core::ptr::null_mut(),
            "Allocation limit exceeded",
            "ator->num_allocs < ator->max_allocs"
        );
        (*ator).num_allocs += 1;
    }

    // C-parity: size_t add wraps in C; keep wrapping semantics (checklist #2).
    (*b).num_items = (*b).num_items.wrapping_add(n);

    // Homogeneous arrays should always be aligned
    let pos = (*b).pos;
    ufbxi_regression_assert!((pos & size_align_mask(size)) == 0);

    // Try to push to the current block. Allocate a new block
    // if the aligned size doesn't fit.
    // C-parity: `b->size - pos` unsigned arithmetic as in `ufbxi_push_size`
    // (see the comment there: a wrap would make the `<=` check PASS, as in C;
    // unreachable since `pos <= b->size` always holds).
    if total <= (*b).size.wrapping_sub(pos) {
        (*b).pos = pos + total;
        chunk_data((*b).chunks[0]).add(pos) as *mut c_void
    } else {
        push_size_new_block(b, total)
    }
}

// ufbx.c:4107-4112 `ufbxi_push_size_zero`
#[inline(never)]
pub(crate) unsafe fn push_size_zero(b: *mut Buf, size: usize, n: usize) -> *mut c_void {
    let ptr = push_size(b, size, n);
    if !ptr.is_null() {
        core::ptr::write_bytes(ptr as *mut u8, 0, size.wrapping_mul(n));
    }
    ptr
}

// ufbx.c:4114-4124 `ufbxi_push_size_copy`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn push_size_copy(
    b: *mut Buf,
    size: usize,
    n: usize,
    data: *const c_void,
) -> *mut c_void {
    // Always succeed with an empty non-NULL buffer for empty allocations, even if `data == NULL`
    ufbx_assert!(size > 0);
    if n == 0 {
        return ZERO_SIZE_BUFFER.as_ptr() as *mut c_void;
    }

    ufbx_assert!(!data.is_null());
    let ptr = push_size(b, size, n);
    if !ptr.is_null() {
        core::ptr::copy_nonoverlapping(data as *const u8, ptr as *mut u8, size.wrapping_mul(n));
    }
    ptr
}

// ufbx.c:4126-4136 `ufbxi_push_size_copy_fast`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn push_size_copy_fast(
    b: *mut Buf,
    size: usize,
    n: usize,
    data: *const c_void,
) -> *mut c_void {
    // Always succeed with an empty non-NULL buffer for empty allocations, even if `data == NULL`
    ufbx_assert!(size > 0);
    if n == 0 {
        return ZERO_SIZE_BUFFER.as_ptr() as *mut c_void;
    }

    ufbx_assert!(!data.is_null());
    let ptr = push_size_fast(b, size, n);
    if !ptr.is_null() {
        core::ptr::copy_nonoverlapping(data as *const u8, ptr as *mut u8, size.wrapping_mul(n));
    }
    ptr
}

// ufbx.c:4138-4171 `ufbxi_buf_free_unused`
#[inline(never)]
pub(crate) unsafe fn buf_free_unused(b: *mut Buf) {
    ufbx_assert!(!(*b).unordered);

    let chunk = (*b).chunks[0];
    if chunk.is_null() {
        return;
    }

    let mut next = (*chunk).next;
    while !next.is_null() {
        let to_free = next;
        next = (*next).next;
        ufbx_assert!((*to_free).magic == BUF_CHUNK_IMP_MAGIC as usize);
        (*to_free).magic = 0;
        free_size(
            (*b).ator,
            1,
            to_free as *mut c_void,
            size_of::<BufChunk>() + (*to_free).size,
        );
    }
    (*chunk).next = core::ptr::null_mut();

    let mut chunk = chunk;
    while (*b).pos == 0 && !chunk.is_null() {
        let prev = (*chunk).prev;
        ufbx_assert!((*chunk).magic == BUF_CHUNK_IMP_MAGIC as usize);
        (*chunk).magic = 0;
        free_size(
            (*b).ator,
            1,
            chunk as *mut c_void,
            size_of::<BufChunk>() + (*chunk).size,
        );
        chunk = prev;
        (*b).chunks[0] = prev;
        if !prev.is_null() {
            (*prev).next = core::ptr::null_mut();
            (*b).pos = (*prev).pushed_pos;
            (*b).size = (*prev).size;
        } else {
            (*b).pos = 0;
            (*b).size = 0;
        }
    }
}

// ufbx.c:4173-4260 `ufbxi_pop_size`
#[inline(never)]
pub(crate) unsafe fn pop_size(b: *mut Buf, size: usize, n: usize, dst: *mut c_void, peek: bool) {
    ufbx_assert!(!(*b).unordered);
    ufbx_assert!(size > 0);
    ufbx_assert!((*b).num_items >= n);
    if !peek {
        // C-parity: size_t sub wraps in C; keep wrapping semantics (checklist #2).
        (*b).num_items = (*b).num_items.wrapping_sub(n);
    }

    let mut ptr = dst as *mut u8;
    let mut bytes_left = size.wrapping_mul(n);

    // We've already pushed this, it better not overflow
    ufbx_assert!(!does_overflow(bytes_left, size, n));

    if !ptr.is_null() {
        ptr = ptr.add(bytes_left);
        let mut pos = (*b).pos;
        let mut chunk = (*b).chunks[0];
        loop {
            if bytes_left <= pos {
                // Rest of the data is in this single chunk
                pos -= bytes_left;
                if !peek {
                    (*b).pos = pos;
                }
                ptr = ptr.sub(bytes_left);
                if bytes_left > 0 {
                    core::ptr::copy_nonoverlapping(chunk_data(chunk).add(pos), ptr, bytes_left);
                }
                break;
            } else {
                // Pop the whole chunk
                ptr = ptr.sub(pos);
                bytes_left -= pos;
                core::ptr::copy_nonoverlapping(chunk_data(chunk), ptr, pos);
                // C-parity: on over-pop `chunk->prev` may be NULL and C
                // dereferences it (crash), guarded only by the num_items
                // assert above — do not add a null check here.
                if !peek {
                    (*chunk).pushed_pos = 0;
                    chunk = (*chunk).prev;
                    (*b).chunks[0] = chunk;
                    (*b).size = (*chunk).size;
                } else {
                    chunk = (*chunk).prev;
                }
                pos = (*chunk).pushed_pos;
            }
        }
    } else {
        let mut pos = (*b).pos;
        let mut chunk = (*b).chunks[0];
        loop {
            if bytes_left <= pos {
                // Rest of the data is in this single chunk
                pos -= bytes_left;
                if !peek {
                    (*b).pos = pos;
                }
                break;
            } else {
                // Pop the whole chunk
                bytes_left -= pos;
                // C-parity: on over-pop `chunk->prev` may be NULL and C
                // dereferences it (crash), guarded only by the num_items
                // assert above — do not add a null check here.
                if !peek {
                    (*chunk).pushed_pos = 0;
                    chunk = (*chunk).prev;
                    (*b).chunks[0] = chunk;
                    (*b).size = (*chunk).size;
                } else {
                    chunk = (*chunk).prev;
                }
                pos = (*chunk).pushed_pos;
            }
        }
    }

    if !peek {
        // Check if we need to rewind past some alignment padding
        let chunk = (*b).chunks[0];
        if !chunk.is_null() {
            let pos = (*b).pos;
            let padding_pos = (*chunk).padding_pos;
            if pos < padding_pos {
                ufbx_assert!(pos + 1 == padding_pos);
                let padding = chunk_data(chunk).add(padding_pos - 1 - 16) as *mut BufPadding;
                (*b).pos = (*padding).original_pos;
                (*chunk).padding_pos = (*padding).prev_padding;
            }
        }

        // Immediately free popped items if all the allocations are huge
        // as it means we want to have dedicated allocations for each push.
        if (*(*b).ator).huge_size <= 1 {
            buf_free_unused(b);
        }
    }
}

// ufbx.c:4262-4268 `ufbxi_push_pop_size`
#[inline(never)]
pub(crate) unsafe fn push_pop_size(
    dst: *mut Buf,
    src: *mut Buf,
    size: usize,
    n: usize,
) -> *mut c_void {
    let data = push_size(dst, size, n);
    if data.is_null() {
        return core::ptr::null_mut();
    }
    pop_size(src, size, n, data, false);
    data
}

// ufbx.c:4270-4276 `ufbxi_push_peek_size`
#[inline(never)]
pub(crate) unsafe fn push_peek_size(
    dst: *mut Buf,
    src: *mut Buf,
    size: usize,
    n: usize,
) -> *mut c_void {
    let data = push_size(dst, size, n);
    if data.is_null() {
        return core::ptr::null_mut();
    }
    pop_size(src, size, n, data, true);
    data
}

// ufbx.c:4278-4297 `ufbxi_buf_free`
#[inline(never)]
pub(crate) unsafe fn buf_free(buf: *mut Buf) {
    // C: `ufbxi_nounroll` — optimizer pragma, no Rust analogue (platform.rs).
    for i in 0..2usize {
        let mut chunk = (*buf).chunks[i];
        if !chunk.is_null() {
            chunk = (*chunk).root;
            while !chunk.is_null() {
                let next = (*chunk).next;
                ufbx_assert!((*chunk).magic == BUF_CHUNK_IMP_MAGIC as usize);
                (*chunk).magic = 0;
                free_size(
                    (*buf).ator,
                    1,
                    chunk as *mut c_void,
                    size_of::<BufChunk>() + (*chunk).size,
                );
                chunk = next;
            }
        }
        (*buf).chunks[i] = core::ptr::null_mut();
    }
    (*buf).pos = 0;
    (*buf).size = 0;
    (*buf).num_items = 0;
}

// ufbx.c:4299-4344 `ufbxi_buf_clear`
#[inline(never)]
pub(crate) unsafe fn buf_clear(buf: *mut Buf) {
    // Only unordered or clearable buffers can be cleared
    ufbx_assert!(!(*buf).unordered || (*buf).clearable);

    // Free the memory if using ASAN
    if (*(*buf).ator).huge_size <= 1 {
        buf_free(buf);
        return;
    }

    // Reset the non-huge chunks as `chunk->next` is always free.
    let chunk = (*buf).chunks[0];
    if !chunk.is_null() {
        let root = (*chunk).root;
        (*buf).chunks[0] = root;
        (*buf).pos = 0;
        (*buf).size = (*root).size;
    }
    (*buf).num_items = 0;
    (*buf).pushed_size = 0;

    // Huge chunks are always sorted by descending size and
    // `chunks[1]` points to the largest one.
    let huge = (*buf).chunks[1];
    if !huge.is_null() {
        // Reset the first N ones that are tracked.
        let mut huge = huge;
        let mut i = 0usize;
        while !huge.is_null() && i < HUGE_MAX_SCAN {
            (*huge).pushed_pos = 0;
            huge = (*huge).next;
            i += 1;
        }

        // Got unreachable tail that should be freed: Unlink from the last
        // tracked chunk and free the rest.
        if !huge.is_null() {
            (*(*huge).prev).next = core::ptr::null_mut();
            while !huge.is_null() {
                let next = (*huge).next;
                ufbx_assert!((*huge).magic == BUF_CHUNK_IMP_MAGIC as usize);
                (*huge).magic = 0;
                free_size(
                    (*buf).ator,
                    1,
                    huge as *mut c_void,
                    size_of::<BufChunk>() + (*huge).size,
                );
                huge = next;
            }
        }
    }
}

// ufbx.c:4346-4354 typed wrappers. The C macros cast `ufbxi_push_size` &co to
// `type*` with `sizeof(type)`; the Rust mapping is generic `#[inline(always)]`
// fns (PORTING.md "Macros & feature gates": expression macros → inline fns).
// `ufbxi_maybe_null` is a clang-analyzer annotation — no Rust analogue.

// ufbx.c:4346 `#define ufbxi_push(b, type, n)`
#[inline(always)]
pub(crate) unsafe fn push<T>(b: *mut Buf, n: usize) -> *mut T {
    push_size(b, size_of::<T>(), n) as *mut T
}

// ufbx.c:4347 `#define ufbxi_push_zero(b, type, n)`
#[inline(always)]
pub(crate) unsafe fn push_zero<T>(b: *mut Buf, n: usize) -> *mut T {
    push_size_zero(b, size_of::<T>(), n) as *mut T
}

// ufbx.c:4348 `#define ufbxi_push_copy(b, type, n, data)`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn push_copy<T>(b: *mut Buf, n: usize, data: *const T) -> *mut T {
    push_size_copy(b, size_of::<T>(), n, data as *const c_void) as *mut T
}

// ufbx.c:4349 `#define ufbxi_push_copy_fast(b, type, n, data)`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn push_copy_fast<T>(b: *mut Buf, n: usize, data: *const T) -> *mut T {
    push_size_copy_fast(b, size_of::<T>(), n, data as *const c_void) as *mut T
}

// ufbx.c:4350 `#define ufbxi_push_fast(b, type, n)`
#[inline(always)]
pub(crate) unsafe fn push_fast<T>(b: *mut Buf, n: usize) -> *mut T {
    push_size_fast(b, size_of::<T>(), n) as *mut T
}

// ufbx.c:4351 `#define ufbxi_pop(b, type, n, dst)`
#[inline(always)]
pub(crate) unsafe fn pop<T>(b: *mut Buf, n: usize, dst: *mut T) {
    pop_size(b, size_of::<T>(), n, dst as *mut c_void, false)
}

// ufbx.c:4352 `#define ufbxi_peek(b, type, n, dst)`
#[inline(always)]
pub(crate) unsafe fn peek<T>(b: *mut Buf, n: usize, dst: *mut T) {
    pop_size(b, size_of::<T>(), n, dst as *mut c_void, true)
}

// ufbx.c:4353 `#define ufbxi_push_pop(dst, src, type, n)`
#[inline(always)]
pub(crate) unsafe fn push_pop<T>(dst: *mut Buf, src: *mut Buf, n: usize) -> *mut T {
    push_pop_size(dst, src, size_of::<T>(), n) as *mut T
}

// ufbx.c:4354 `#define ufbxi_push_peek(dst, src, type, n)`
#[inline(always)]
pub(crate) unsafe fn push_peek<T>(dst: *mut Buf, src: *mut Buf, n: usize) -> *mut T {
    push_peek_size(dst, src, size_of::<T>(), n) as *mut T
}

// CONTINUATION POINT: `// -- Memory buffer` section complete (ufbx.c:3817-4354).
// Next banner: ufbx.c:4356 `// -- Hash map` (owned by native/hash.rs or a new
// map unit; `ufbxi_map_init` at ufbx.c:4393).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::Error;
    use crate::native::allocator::init_ator;
    use core::mem::MaybeUninit;

    unsafe fn make_buf(ator: *mut Allocator, unordered: bool, clearable: bool) -> Buf {
        let mut buf = MaybeUninit::<Buf>::zeroed().assume_init();
        buf.ator = ator;
        buf.unordered = unordered;
        buf.clearable = clearable;
        buf
    }

    unsafe fn free_all_chunks(b: &mut Buf) {
        // Test-only teardown (kept separate from `buf_free` above so these
        // tests don't depend on it): walk both chunk lists from their roots
        // and free every chunk.
        for list_ix in 0..2 {
            let chunk = b.chunks[list_ix];
            if chunk.is_null() {
                continue;
            }
            let mut c = (*chunk).root;
            while !c.is_null() {
                let next = (*c).next;
                crate::native::allocator::free_size(
                    b.ator,
                    1,
                    c as *mut core::ffi::c_void,
                    size_of::<BufChunk>() + (*c).size,
                );
                c = next;
            }
            b.chunks[list_ix] = core::ptr::null_mut();
        }
    }

    #[test]
    fn test_chunk_header_layout() {
        assert_eq!(size_of::<BufChunk>(), 8 * size_of::<usize>());
        assert_eq!(size_of::<BufChunk>() % 8, 0);
    }

    #[test]
    fn test_push_basic_geometry() {
        unsafe {
            let mut err = Error::default();
            let mut ator = MaybeUninit::<Allocator>::zeroed();
            init_ator(
                &mut err,
                ator.as_mut_ptr(),
                core::ptr::null(),
                b"test\0".as_ptr(),
            );
            let ator = ator.as_mut_ptr();
            let mut buf = make_buf(ator, false, false);

            // Zero-count push returns the shared zero-size buffer.
            let z = push_size(&mut buf, 4, 0);
            assert_eq!(z as *const u8, ZERO_SIZE_BUFFER.as_ptr());
            assert_eq!(buf.num_items, 0);

            let p = push_size(&mut buf, 4, 3) as *mut u32;
            assert!(!p.is_null());
            assert_eq!(buf.num_items, 3);
            // First chunk: next_size 4096, chunk_size = 4096 - header, 16-aligned.
            let expect_size = align_to_mask(4096 - size_of::<BufChunk>(), 0xf);
            assert_eq!(buf.size, expect_size);
            assert_eq!(buf.pos, 12);
            assert_eq!((*buf.chunks[0]).magic, BUF_CHUNK_IMP_MAGIC as usize);

            // Aligned follow-up push in the same chunk (u64 after 12 bytes pads to 16,
            // ordered buffer writes a 16-byte padding record).
            let q = push_size(&mut buf, 8, 1) as *mut u64;
            assert!(!q.is_null());
            *q = 0x1122334455667788;
            assert_eq!(buf.pos, 16 + 16 + 8);
            assert_eq!((*buf.chunks[0]).padding_pos, 16 + 16 + 1);

            free_all_chunks(&mut buf);
            assert_eq!((*ator).current_size, 0);
        }
    }

    #[test]
    fn test_new_block_growth_doubling() {
        unsafe {
            let mut err = Error::default();
            let mut ator = MaybeUninit::<Allocator>::zeroed();
            init_ator(
                &mut err,
                ator.as_mut_ptr(),
                core::ptr::null(),
                b"test\0".as_ptr(),
            );
            let ator = ator.as_mut_ptr();
            let mut buf = make_buf(ator, false, false);

            let p = push_size(&mut buf, 1, 100);
            assert!(!p.is_null());
            assert_eq!((*buf.chunks[0]).next_size, 4096);

            // Overflow the first chunk: next chunk doubles next_size.
            let big = buf.size; // larger than remaining space
            let q = push_size(&mut buf, 1, big);
            assert!(!q.is_null());
            assert_eq!((*buf.chunks[0]).next_size, 8192);
            // Retired chunk stored its final position.
            assert_eq!((*(*buf.chunks[0]).prev).pushed_pos, 100);
            assert_eq!(buf.pushed_size, 100);

            free_all_chunks(&mut buf);
            assert_eq!((*ator).current_size, 0);
        }
    }

    #[test]
    fn test_huge_unordered_second_list() {
        unsafe {
            let mut err = Error::default();
            let mut ator = MaybeUninit::<Allocator>::zeroed();
            init_ator(
                &mut err,
                ator.as_mut_ptr(),
                core::ptr::null(),
                b"test\0".as_ptr(),
            );
            let ator = ator.as_mut_ptr();
            let mut buf = make_buf(ator, true, false);

            let huge = (*ator).huge_size; // 0x100000
            let p = push_size(&mut buf, 1, huge);
            assert!(!p.is_null());
            // Huge unordered pushes go to chunks[1]; chunks[0]/pos/size untouched.
            assert!(buf.chunks[0].is_null());
            assert!(!buf.chunks[1].is_null());
            assert_eq!(buf.pos, 0);
            assert_eq!((*buf.chunks[1]).pushed_pos, huge);
            assert_eq!(buf.pushed_size, huge);

            free_all_chunks(&mut buf);
            assert_eq!((*ator).current_size, 0);
        }
    }

    #[test]
    fn test_push_zero_and_copy() {
        unsafe {
            let mut err = Error::default();
            let mut ator = MaybeUninit::<Allocator>::zeroed();
            init_ator(
                &mut err,
                ator.as_mut_ptr(),
                core::ptr::null(),
                b"test\0".as_ptr(),
            );
            let ator = ator.as_mut_ptr();
            let mut buf = make_buf(ator, false, false);

            let p = push_size_zero(&mut buf, 1, 32) as *mut u8;
            assert!(!p.is_null());
            for i in 0..32 {
                assert_eq!(*p.add(i), 0);
            }

            let src: [u32; 4] = [1, 2, 3, 4];
            let q = push_size_copy(&mut buf, 4, 4, src.as_ptr() as *const core::ffi::c_void)
                as *mut u32;
            assert!(!q.is_null());
            for i in 0..4 {
                assert_eq!(*q.add(i), src[i]);
            }

            // Copy with n == 0 succeeds even with NULL data.
            let z = push_size_copy(&mut buf, 4, 0, core::ptr::null());
            assert_eq!(z as *const u8, ZERO_SIZE_BUFFER.as_ptr());

            free_all_chunks(&mut buf);
            assert_eq!((*ator).current_size, 0);
        }
    }

    #[test]
    fn test_pop_and_peek_across_chunks() {
        unsafe {
            let mut err = Error::default();
            let mut ator = MaybeUninit::<Allocator>::zeroed();
            init_ator(
                &mut err,
                ator.as_mut_ptr(),
                core::ptr::null(),
                b"test\0".as_ptr(),
            );
            let ator = ator.as_mut_ptr();
            let mut buf = make_buf(ator, false, false);

            // Push enough u32 items to span multiple chunks (first chunk holds
            // ~4032 bytes; 4096 items = 16384 bytes).
            const N: usize = 4096;
            for i in 0..N {
                let p = push::<u32>(&mut buf, 1);
                assert!(!p.is_null());
                *p = i as u32;
            }
            assert_eq!(buf.num_items, N);
            assert!(!(*buf.chunks[0]).prev.is_null(), "must span chunks");

            // Peek the last 100 items — non-destructive; flattening walks the
            // chunk chain backwards.
            let mut out = [0u32; 100];
            peek::<u32>(&mut buf, 100, out.as_mut_ptr());
            for i in 0..100 {
                assert_eq!(out[i], (N - 100 + i) as u32);
            }
            assert_eq!(buf.num_items, N);

            // Pop all items in chunks of 300, spanning chunk boundaries.
            let mut remaining = N;
            let mut dst = [0u32; 300];
            while remaining > 0 {
                let take = remaining.min(300);
                pop::<u32>(&mut buf, take, dst.as_mut_ptr());
                for i in 0..take {
                    assert_eq!(dst[i], (remaining - take + i) as u32);
                }
                remaining -= take;
            }
            assert_eq!(buf.num_items, 0);
            assert_eq!(buf.pos, 0);

            buf_free(&mut buf);
            assert_eq!((*ator).current_size, 0);
        }
    }

    #[test]
    fn test_pop_null_dst_rewinds_padding() {
        unsafe {
            let mut err = Error::default();
            let mut ator = MaybeUninit::<Allocator>::zeroed();
            init_ator(
                &mut err,
                ator.as_mut_ptr(),
                core::ptr::null(),
                b"test\0".as_ptr(),
            );
            let ator = ator.as_mut_ptr();
            let mut buf = make_buf(ator, false, false);

            // 12 bytes, then an 8-aligned push forces a padding record.
            let _ = push_size(&mut buf, 4, 3);
            let q = push_size(&mut buf, 8, 1);
            assert!(!q.is_null());
            assert_eq!(buf.pos, 16 + 16 + 8);
            assert_eq!((*buf.chunks[0]).padding_pos, 16 + 16 + 1);

            // Discarding pop (dst == NULL) rewinds through the padding record.
            pop_size(&mut buf, 8, 1, core::ptr::null_mut(), false);
            assert_eq!(buf.pos, 12);
            assert_eq!((*buf.chunks[0]).padding_pos, 0);

            buf_free(&mut buf);
            assert_eq!((*ator).current_size, 0);
        }
    }

    #[test]
    fn test_push_pop_flatten() {
        unsafe {
            let mut err = Error::default();
            let mut ator = MaybeUninit::<Allocator>::zeroed();
            init_ator(
                &mut err,
                ator.as_mut_ptr(),
                core::ptr::null(),
                b"test\0".as_ptr(),
            );
            let ator = ator.as_mut_ptr();
            let mut stack = make_buf(ator, false, false);
            let mut result = make_buf(ator, false, false);

            const N: usize = 3000;
            for i in 0..N {
                let p = push::<u64>(&mut stack, 1);
                assert!(!p.is_null());
                *p = i as u64;
            }
            assert!(!(*stack.chunks[0]).prev.is_null(), "must span chunks");

            // Flatten the non-contiguous stack into a contiguous array.
            let arr = push_pop::<u64>(&mut result, &mut stack, N);
            assert!(!arr.is_null());
            for i in 0..N {
                assert_eq!(*arr.add(i), i as u64);
            }
            assert_eq!(stack.num_items, 0);
            assert_eq!(result.num_items, N);

            buf_free(&mut stack);
            buf_free(&mut result);
            assert_eq!((*ator).current_size, 0);
        }
    }

    #[test]
    fn test_buf_free_unused_frees_forward_chunks() {
        unsafe {
            let mut err = Error::default();
            let mut ator = MaybeUninit::<Allocator>::zeroed();
            init_ator(
                &mut err,
                ator.as_mut_ptr(),
                core::ptr::null(),
                b"test\0".as_ptr(),
            );
            let ator = ator.as_mut_ptr();
            let mut buf = make_buf(ator, false, false);

            // Span two chunks, then pop everything back to zero.
            let n1 = 4000usize;
            let _ = push_size(&mut buf, 1, n1);
            let _ = push_size(&mut buf, 1, 1000);
            assert!(!(*buf.chunks[0]).prev.is_null());
            pop_size(&mut buf, 1, 1000, core::ptr::null_mut(), false);
            pop_size(&mut buf, 1, n1, core::ptr::null_mut(), false);
            assert_eq!(buf.pos, 0);

            // Frees the empty head chunks and the retired next-chain entirely.
            buf_free_unused(&mut buf);
            assert!(buf.chunks[0].is_null());
            assert_eq!(buf.size, 0);
            assert_eq!((*ator).current_size, 0);

            buf_free(&mut buf);
        }
    }

    #[test]
    fn test_buf_clear_resets_and_trims_huge() {
        unsafe {
            let mut err = Error::default();
            let mut ator = MaybeUninit::<Allocator>::zeroed();
            init_ator(
                &mut err,
                ator.as_mut_ptr(),
                core::ptr::null(),
                b"test\0".as_ptr(),
            );
            let ator = ator.as_mut_ptr();
            let mut buf = make_buf(ator, true, true);

            // Normal chunks plus more huge chunks than UFBXI_HUGE_MAX_SCAN.
            let _ = push_size(&mut buf, 1, 100);
            let huge = (*ator).huge_size;
            for i in 0..(HUGE_MAX_SCAN + 4) {
                let p = push_size(&mut buf, 1, huge + i);
                assert!(!p.is_null());
            }
            assert!(!buf.chunks[1].is_null());

            buf_clear(&mut buf);
            assert_eq!(buf.pos, 0);
            assert_eq!(buf.num_items, 0);
            assert_eq!(buf.pushed_size, 0);
            // chunks[0] rewound to root.
            assert_eq!(buf.chunks[0], (*buf.chunks[0]).root);
            // Exactly HUGE_MAX_SCAN huge chunks remain, each reset.
            let mut count = 0usize;
            let mut c = buf.chunks[1];
            while !c.is_null() {
                assert_eq!((*c).pushed_pos, 0);
                count += 1;
                c = (*c).next;
            }
            assert_eq!(count, HUGE_MAX_SCAN);

            buf_free(&mut buf);
            assert_eq!((*ator).current_size, 0);
        }
    }
}
