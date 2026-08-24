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
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]
use core::ffi::c_void;
use core::mem::size_of;

use crate::native::allocator::{
    align_to_mask, alloc_size, does_overflow, free_size, size_align_mask, Allocator,
    BUF_CHUNK_IMP_MAGIC, ZERO_SIZE_BUFFER,
};
#[cfg(feature = "regression")]
use crate::native::error::ufbxi_check_return_err_msg;
use crate::native::platform::{ufbx_assert, ufbxi_regression_assert};
use crate::native::view::{view_read, view_write, View};
use core::marker::PhantomData;

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
// Stays `unsafe fn`: it never dereferences `chunk`, but `.add()` still
// carries the same-allocation/in-bounds invariant on `chunk` that a
// dereference would (offsetting a dangling/undersized pointer is UB even
// without reading through it) — not sound to expose as a safe fn.
#[inline(always)]
pub(crate) unsafe fn chunk_data(chunk: *mut BufChunk) -> *mut u8 {
    // SAFETY: this fn's contract is that `chunk` addresses a live `BufChunk`
    // whose backing allocation is `sizeof(BufChunk) + size` bytes; offsetting by
    // the header size lands at the start of the flexible `data` array (one past
    // the header, in the same allocation — the very start of `data`).
    unsafe { (chunk as *mut u8).add(size_of::<BufChunk>()) }
}

// -- Chunk views and the chunk-list walker (port-local)
//
// Every C loop over a chunk chain is `for (c = head; c; c = c->next)` (or
// `->prev`) whose body reads/writes header fields and may retire or free the
// chunk it holds. `ChunkIter` is that loop shape with ONE vouch at
// construction: it reads the link BEFORE yielding, so a body freeing the chunk
// it was handed is fine, and bodies work through `View<BufChunk>` accessors
// (freeing takes the raw pointer back out with `get()`). Magic asserts stay in
// the bodies exactly where C has them.

impl View<BufChunk> {
    #[inline(always)]
    pub(crate) fn prev(&self) -> *mut BufChunk {
        view_read!(self, prev)
    }
    #[inline(always)]
    pub(crate) fn magic(&self) -> usize {
        view_read!(self, magic)
    }
    #[inline(always)]
    pub(crate) fn size(&self) -> usize {
        view_read!(self, size)
    }
    #[inline(always)]
    pub(crate) fn pushed_pos(&self) -> usize {
        view_read!(self, pushed_pos)
    }
    #[inline(always)]
    pub(crate) fn set_pushed_pos(&self, pushed_pos: usize) {
        view_write!(self, pushed_pos, pushed_pos)
    }
}

/// One chunk as the walker yields it: the header as a view for field access
/// (deref), plus the ORIGINAL pointer for everything that reaches past the
/// header. A `View<BufChunk>` covers `size_of::<BufChunk>()` bytes only — the
/// flexible `data` array and the allocation as a whole are outside its
/// provenance, so freeing the chunk or addressing its payload must go through
/// `ptr()` / `data()`, never through a pointer derived from the view.
pub(crate) struct ChunkRef<'a> {
    ptr: *mut BufChunk,
    view: &'a View<BufChunk>,
}

impl<'a> ChunkRef<'a> {
    /// The chunk pointer with whole-allocation provenance (for `free_chunk`,
    /// relinking, and as the `Buf`'s stored chunk pointer).
    #[inline(always)]
    pub(crate) fn ptr(&self) -> *mut BufChunk {
        self.ptr
    }
    /// Start of the chunk's flexible `data` array.
    #[inline(always)]
    pub(crate) fn data(&self) -> *mut u8 {
        // SAFETY: a yielded chunk is live (walker contract), so its backing
        // allocation is `sizeof(BufChunk) + size` bytes — `chunk_data`'s contract.
        unsafe { chunk_data(self.ptr) }
    }
}

impl<'a> core::ops::Deref for ChunkRef<'a> {
    type Target = View<BufChunk>;
    #[inline(always)]
    fn deref(&self) -> &View<BufChunk> {
        self.view
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ChunkDir {
    Next,
    Prev,
}

/// Walks a chunk chain from `head` following `->next` / `->prev`, yielding
/// [`ChunkRef`]s. The link is read before the yield, so the body may free the
/// chunk it receives; `cursor()` exposes the chunk the next call would yield
/// (null at the end) for the bounded scans that continue past it.
pub(crate) struct ChunkIter<'a> {
    cur: *mut BufChunk,
    dir: ChunkDir,
    _marker: PhantomData<&'a View<BufChunk>>,
}

impl<'a> ChunkIter<'a> {
    /// # Safety
    /// `head` is null or a live `BufChunk` whose chain in `dir` consists of
    /// live chunks ending in null, each staying alive at least until the walk
    /// has yielded it (the link is read before the yield, so a body may free
    /// the chunk it holds but no earlier one). `'a` is unconstrained by the
    /// raw `head`: the caller must not let a `ChunkRef` outlive the chain —
    /// there is no borrow for the compiler to tie it to.
    ///
    /// Reading the link before the yield also means a corrupted chunk has its
    /// link read before the body's `magic` assert fires (C's two scan loops
    /// assert first); the tripwire is weaker only on already-invalid memory.
    #[inline]
    unsafe fn new(head: *mut BufChunk, dir: ChunkDir) -> Self {
        Self {
            cur: head,
            dir,
            _marker: PhantomData,
        }
    }
    /// # Safety
    /// As [`ChunkIter::new`] over the `->next` chain.
    #[inline]
    pub(crate) unsafe fn forward(head: *mut BufChunk) -> Self {
        // SAFETY: forwarded contract.
        unsafe { Self::new(head, ChunkDir::Next) }
    }
    /// # Safety
    /// As [`ChunkIter::new`] over the `->prev` chain.
    #[inline]
    pub(crate) unsafe fn backward(head: *mut BufChunk) -> Self {
        // SAFETY: forwarded contract.
        unsafe { Self::new(head, ChunkDir::Prev) }
    }
    /// The chunk `next()` would yield (null once the chain is exhausted).
    #[inline]
    pub(crate) fn cursor(&self) -> *mut BufChunk {
        self.cur
    }
}

impl<'a> Iterator for ChunkIter<'a> {
    type Item = ChunkRef<'a>;

    #[inline]
    fn next(&mut self) -> Option<ChunkRef<'a>> {
        let cur = self.cur;
        if cur.is_null() {
            return None;
        }
        // SAFETY: `cur` is a live chunk of the vouched chain (construction
        // contract), so reading its link and minting a header view over it are
        // valid; the link is read now so the body may free `cur`.
        unsafe {
            self.cur = match self.dir {
                ChunkDir::Next => (*cur).next,
                ChunkDir::Prev => (*cur).prev,
            };
            Some(ChunkRef {
                ptr: cur,
                view: View::<BufChunk>::from_ptr(cur),
            })
        }
    }
}

// The one place a chunk is retired and returned to its allocator.
// C at every site: `ufbx_assert(chunk->magic == MAGIC); chunk->magic = 0;
// ufbxi_free_size(ator, 1, chunk, sizeof(ufbxi_buf_chunk) + chunk->size);`.
// Takes the `ChunkRef` by value: after the free the body cannot touch the
// chunk through its (now dangling) accessors without a compile error.
//
// # Safety
// `chunk` was yielded by a `ChunkIter` whose chain is allocated from `ator`
// (each chunk `size_of::<BufChunk>() + size` bytes) and is live at this point.
#[inline]
unsafe fn free_chunk(ator: *mut Allocator, chunk: ChunkRef<'_>) {
    let chunk: *mut BufChunk = chunk.ptr();
    // SAFETY: the fn contract above — a live chunk of exactly that allocation,
    // reached through its whole-allocation pointer.
    unsafe {
        ufbx_assert!((*chunk).magic == BUF_CHUNK_IMP_MAGIC as usize);
        (*chunk).magic = 0;
        free_size(
            ator,
            1,
            chunk as *mut c_void,
            size_of::<BufChunk>() + (*chunk).size,
        );
    }
}

// ufbx.c:3853-3870 `ufbxi_buf`
// NOT `Copy`/`Clone`: a `Buf` owns its chunk lists (freed via `free_all_chunks`
// / `buf_free`), so a by-value copy aliases ownership — a latent double-free.
// C copies the struct freely; the ported sites that genuinely move one
// (ownership transfer into a `Refcount`, `release_ref` stack copies) use
// explicit `ptr::read`. See PORTING.md "Copy vs non-Copy structs".
#[repr(C)]
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

// Typed interior-mutable VIEW over a `Buf` field, reinterpreted in place. `Buf` is
// large + Copy, and its subfields are written, so a value getter is wrong (would
// copy 88 bytes and drop write-backs). Getters + setters for the accessed subfields.
pub(crate) type BufView = crate::native::view::View<Buf>;

// Safe typed push family (mirrors `MapView::grow/find/insert`). The safety
// argument, written once for every method below:
//
// - `self.get()` yields a write-provenance `*mut Buf` (view invariant: a
//   `BufView` is minted only over a live, initialized `Buf` embedded in
//   context/arena-owned memory).
// - `(*b).ator` is the buf's stored allocator back-pointer into the same
//   context (construction invariant — the same standing as `Map`'s stored
//   `ator` in `MapView`'s methods).
// - The push/pop imps write only the buf header and chunk memory they
//   allocate/own; they dereference no caller pointers except where a method
//   takes `&T`/`&[T]`, which carries the validity in the type.
//
// Allocation failure returns null — the caller's `ufbxi_check` pattern is
// unchanged. The returned region is uninitialized (`push`), zeroed
// (`push_zero`), or copied-from-src; DEREFERENCING the returned pointer is the
// caller's obligation, under its own narrow `unsafe`.
impl BufView {
    #[inline(always)]
    #[must_use]
    pub(crate) fn push<T>(&self, n: usize) -> *mut T {
        unsafe { push::<T>(self.get(), n) }
    }
    #[inline(always)]
    #[must_use]
    pub(crate) fn push_fast<T>(&self, n: usize) -> *mut T {
        unsafe { push_fast::<T>(self.get(), n) }
    }
    #[inline(always)]
    #[must_use]
    pub(crate) fn push_zero<T>(&self, n: usize) -> *mut T {
        unsafe { push_zero::<T>(self.get(), n) }
    }
    // Copy-in from a borrow. Raw-pointer sources (arena runs reached by
    // pointer arithmetic) stay on the free `unsafe fn push_copy` —
    // pointer-carrying-param rule.
    #[inline(always)]
    #[must_use]
    pub(crate) fn push_copy_ref<T>(&self, src: &T) -> *mut T {
        unsafe { push_copy::<T>(self.get(), 1, src) }
    }
    #[inline(always)]
    #[must_use]
    pub(crate) fn push_copy_slice<T>(&self, src: &[T]) -> *mut T {
        unsafe { push_copy::<T>(self.get(), src.len(), src.as_ptr()) }
    }
    #[inline(always)]
    #[must_use]
    pub(crate) fn push_copy_fast_ref<T>(&self, src: &T) -> *mut T {
        unsafe { push_copy_fast::<T>(self.get(), 1, src) }
    }
    /// Copy-in from a RAW source run (arena pointers reached by pointer
    /// arithmetic, where no borrow exists to carry validity) — the buf-side
    /// vouch lives in the view like the rest of the family; only the
    /// source-run validity stays on the caller.
    ///
    /// # Safety
    /// `src` must be readable for `n` `T`s, and the run must not overlap the
    /// chunk memory the push writes.
    #[inline(always)]
    #[must_use]
    pub(crate) unsafe fn push_copy_raw<T>(&self, n: usize, src: *const T) -> *mut T {
        // SAFETY: buf side per the view invariant (family comment above); the
        // source run is the caller's vouch (fn contract).
        unsafe { push_copy::<T>(self.get(), n, src) }
    }
    /// `push_copy_fast` flavor of [`Self::push_copy_raw`] (same contract).
    #[inline(always)]
    #[must_use]
    pub(crate) unsafe fn push_copy_fast_raw<T>(&self, n: usize, src: *const T) -> *mut T {
        // SAFETY: as for `push_copy_raw`.
        unsafe { push_copy_fast::<T>(self.get(), n, src) }
    }
    // Two-buf transfer: pop `n` items off the top of `src` and push them onto
    // `self`. The bufs must be distinct (C call sites always pair
    // result ← tmp_stack); same-buf transfer would interleave header updates.
    #[inline(always)]
    #[must_use]
    pub(crate) fn push_pop<T>(&self, src: &BufView, n: usize) -> *mut T {
        debug_assert!(!core::ptr::eq(self, src));
        unsafe { push_pop::<T>(self.get(), src.get(), n) }
    }
    #[inline(always)]
    #[must_use]
    pub(crate) fn push_peek<T>(&self, src: &BufView, n: usize) -> *mut T {
        debug_assert!(!core::ptr::eq(self, src));
        unsafe { push_peek::<T>(self.get(), src.get(), n) }
    }
}

impl BufView {
    #[inline(always)]
    pub(crate) fn set_ator(&self, ator: *mut Allocator) {
        view_write!(self, ator, ator)
    }
    #[inline(always)]
    pub(crate) fn num_items(&self) -> usize {
        view_read!(self, num_items)
    }
    #[inline(always)]
    pub(crate) fn pos(&self) -> usize {
        view_read!(self, pos)
    }
    #[inline(always)]
    pub(crate) fn pushed_size(&self) -> usize {
        view_read!(self, pushed_size)
    }
    #[inline(always)]
    pub(crate) fn set_unordered(&self, unordered: bool) {
        view_write!(self, unordered, unordered)
    }
    #[inline(always)]
    pub(crate) fn set_clearable(&self, clearable: bool) {
        view_write!(self, clearable, clearable)
    }
}

// ufbx.c:3872-3876 `ufbxi_buf_state`
// C-parity: `ufbxi_buf_state` has no users in ufbx.c either (C does not warn on
// an unreferenced typedef); kept so the `ufbxi_buf` type family is complete.
#[allow(dead_code)]
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
    // SAFETY: `b` addresses a live, initialized `Buf` (this fn's raw-pointer
    // contract) and `(*b).ator` is its live stored allocator back-pointer;
    // reading `huge_size` off that allocator.
    let huge = size >= unsafe { (*(*b).ator).huge_size };

    // Use the second chunk "list" for huge unordered chunks.
    // The state of these chunks is not tracked by `ufbxi_buf.pos/size`.
    // SAFETY: `b` is the live `Buf`; reading its `unordered` flag.
    let list_ix: u32 = (unsafe { (*b).unordered } as u32) & (huge as u32);

    // SAFETY: `b` is the live `Buf`; reading the head of the selected chunk list.
    let mut chunk = unsafe { (*b).chunks[list_ix as usize] };
    if !chunk.is_null() {
        // SAFETY (every `(*b)`/`(*chunk)`/`(*next)`/`(*best_chunk)` access in
        // this reuse-scan block, incl. the `chunk_data(...).add(pos)` early
        // return): `b` is the live `Buf`; `chunk`/`next`/`best_chunk` are live
        // `BufChunk`s of the selected `chunks[list_ix]` list, walked through
        // their own `->next` links (for `list_ix == 1` the swap HACK leaves the
        // list's `->root` pointers inconsistent, so only `->next` is relied on
        // here). Each is backed by `sizeof(BufChunk) + chunk->size` bytes; the
        // best-fit check (`size <= space`) established `pos + size <=
        // best_chunk->size`, keeping the early-return `.add(pos)` in-bounds.
        if list_ix == 0 {
            // Store the final position for the retired chunk and scan free
            // chunks in case we find one the allocation fits in.
            unsafe {
                (*b).pushed_size += (*b).pos;
                (*chunk).pushed_pos = (*b).pos;
            }
            // SAFETY: `chunk->next` heads the live `->next` chain of this list.
            for c in unsafe { ChunkIter::forward((*chunk).next) } {
                ufbx_assert!(c.magic() == BUF_CHUNK_IMP_MAGIC as usize);
                chunk = c.ptr();
                ufbx_assert!(unsafe { (*b).unordered } || c.pushed_pos() == 0);
                c.set_pushed_pos(0);
                if size <= c.size() {
                    unsafe {
                        (*b).chunks[0] = chunk;
                        // C-parity: C truncates through `(uint32_t)size` here (ufbx.c:3901).
                        (*b).pos = size as u32 as usize;
                        (*b).size = c.size();
                    }
                    return c.data() as *mut c_void;
                }
            }
        } else if unsafe { (*b).clearable } {
            // Keep track of the `UFBXI_HUGE_MAX_SCAN` largest chunks and
            // retain them. Overflowing chunks are freed in `ufbxi_buf_clear()`
            let align_mask = size_align_mask(size);

            let mut best_chunk: *mut BufChunk = core::ptr::null_mut();
            let mut best_space = usize::MAX;

            // Clearable huge chunks are sorted by descending size. Check the first N
            // chunks for reuse and find the place a new block should be inserted if
            // no suitable space is found. Chunk ordering in the tail doesn't matter
            // as those chunks are never reused.
            // Unreachable chunks in the tail are freed in `ufbxi_buf_clear()`.
            let mut i = 0usize;
            // SAFETY: `chunk` heads the live `->next` chain of the huge list.
            let mut chunks = unsafe { ChunkIter::forward(chunk) };
            // C: `for (; next && i < UFBXI_HUGE_MAX_SCAN; i++)`
            while i < HUGE_MAX_SCAN {
                let Some(c) = chunks.next() else {
                    break;
                };
                ufbx_assert!(c.magic() == BUF_CHUNK_IMP_MAGIC as usize);
                if c.size() < size {
                    break;
                }
                chunk = c.ptr();

                // Try to reuse chunks using a best-fit strategy.
                let pos = align_to_mask(c.pushed_pos(), align_mask);
                // C-parity: unsigned wrap when `pos > chunk->size` (over-aligned
                // position past the chunk end) would yield a huge `space`, making
                // the `size <= space` check below PASS — identical to C
                // (ufbx.c:3928-3929). Unreachable in practice: chunk sizes are
                // 16-aligned and `align_mask <= 15`, so `pos <= chunk->size`
                // always. Do not replace this with `checked_sub`: that would
                // diverge from C.
                let space = c.size().wrapping_sub(pos);
                if size <= space {
                    if space < best_space {
                        best_chunk = chunk;
                        best_space = space;
                    }
                }

                i += 1;
            }

            // Early return if we found a slot.
            if !best_chunk.is_null() {
                let pos = align_to_mask(unsafe { (*best_chunk).pushed_pos }, align_mask);
                unsafe {
                    (*best_chunk).pushed_pos = pos + size;
                    (*b).pushed_size += size;
                }
                return unsafe { chunk_data(best_chunk).add(pos) } as *mut c_void;
            }
        }
    }

    // Allocate a new chunk, grow `next_size` geometrically but don't double
    // the current or previous user sizes if they are larger.
    let mut chunk_size: usize;
    let mut next_size: usize;

    // If `size` is larger than `huge_size` don't grow `next_size` geometrically,
    // but use a dedicated allocation.
    // SAFETY (this if/else): `chunk` is either null or a live `BufChunk` of
    // `b`'s list; `(*b).ator` is the buf's live allocator. Reads of
    // `(*chunk).next_size` are guarded by `!chunk.is_null()`.
    if huge {
        next_size = if !chunk.is_null() {
            unsafe { (*chunk).next_size }
        } else {
            4096
        };
        if next_size > unsafe { (*(*b).ator).chunk_max } {
            next_size = unsafe { (*(*b).ator).chunk_max };
        }
        chunk_size = size;
    } else {
        next_size = if !chunk.is_null() {
            unsafe { (*chunk).next_size }.wrapping_mul(2)
        } else {
            4096
        };
        if next_size > unsafe { (*(*b).ator).chunk_max } {
            next_size = unsafe { (*(*b).ator).chunk_max };
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

    // SAFETY: `(*b).ator` is the buf's live allocator; `alloc_size` is its own
    // `unsafe fn` contract (allocate `sizeof(BufChunk) + chunk_size` bytes).
    let new_chunk = unsafe {
        alloc_size((*b).ator, 1, size_of::<BufChunk>().wrapping_add(chunk_size)) as *mut BufChunk
    };
    if new_chunk.is_null() {
        return core::ptr::null_mut();
    }

    // SAFETY: `new_chunk` is the freshly allocated live `BufChunk` (checked
    // non-null above) with `sizeof(BufChunk) + chunk_size` writable bytes;
    // initializing its header fields. `chunk` is the prior link, null or live.
    unsafe {
        (*new_chunk).prev = chunk;
        (*new_chunk).size = chunk_size;
        (*new_chunk).next_size = next_size;
        (*new_chunk).magic = BUF_CHUNK_IMP_MAGIC as usize;
        (*new_chunk).padding_pos = 0;
        (*new_chunk).pushed_pos = 0;
    }

    // Link the chunk to the list and set it as the active one
    // SAFETY (this if/else and the `list_ix` block below): `new_chunk` is the
    // live new chunk; `chunk`/`next`/`root` are null or live `BufChunk`s of
    // `b`'s list, and `b` is the live `Buf`. All accesses read/write header
    // fields of these live chunks or `b`'s own fields.
    if !chunk.is_null() {
        let next = unsafe { (*chunk).next };
        if !next.is_null() {
            unsafe { (*next).prev = new_chunk };
        }
        unsafe {
            (*new_chunk).next = next;
            (*chunk).next = new_chunk;
            (*new_chunk).root = (*chunk).root;
        }
    } else {
        unsafe {
            (*new_chunk).next = core::ptr::null_mut();
            (*new_chunk).root = new_chunk;
        }
    }

    if list_ix == 0 {
        unsafe {
            (*b).chunks[0] = new_chunk;
            (*b).pos = size;
            (*b).size = chunk_size;
        }
    } else {
        let root = unsafe { (*b).chunks[1] };
        unsafe { (*b).pushed_size += size };
        if root.is_null() {
            unsafe { (*b).chunks[1] = new_chunk };
        } else if unsafe { (*root).size } < chunk_size {
            // Swap root and self if necessary, we should have bailed out
            // in the search loop in the first iteration so `new_chunk` should
            // directly follow `root`.
            // HACK: This ends up with `chunks[1]` entries having inconsistent
            // `ufbxi_buf_chunk.root` pointers but other code only reads `chunks[1].root`
            // TODO: Move roots out of the chunks?
            ufbx_assert!(unsafe { (*root).next } == new_chunk);
            ufbx_assert!(unsafe { (*new_chunk).prev } == root);
            if !unsafe { (*new_chunk).next }.is_null() {
                unsafe { (*(*new_chunk).next).prev = root };
            }
            unsafe {
                (*root).next = (*new_chunk).next;
                (*new_chunk).next = root;
                (*new_chunk).prev = core::ptr::null_mut();
                (*new_chunk).root = new_chunk;
                (*b).chunks[1] = new_chunk;
            }
        }
        unsafe { (*new_chunk).pushed_pos = size };
    }

    (unsafe { chunk_data(new_chunk) }) as *mut c_void
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
        // SAFETY: `b` addresses a live `Buf` (this fn's raw-pointer contract);
        // reading its live stored allocator back-pointer.
        let ator = unsafe { (*b).ator };
        ufbxi_check_return_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr((*ator).error) },
            // SAFETY: `ator` is the buf's live allocator; reading its alloc
            // counters.
            unsafe { (*ator).num_allocs < (*ator).max_allocs },
            core::ptr::null_mut(),
            "Allocation limit exceeded",
            "ator->num_allocs < ator->max_allocs"
        );
        // SAFETY: `ator` is the buf's live allocator; bumping its alloc count.
        unsafe { (*ator).num_allocs += 1 };
    }

    // C-parity: size_t add wraps in C; keep wrapping semantics (checklist #2).
    // SAFETY: `b` is the live `Buf`; updating its item count.
    unsafe { (*b).num_items = (*b).num_items.wrapping_add(n) };

    // Align to the natural alignment based on the size
    let align_mask = size_align_mask(size);
    // SAFETY: `b` is the live `Buf`; reading its current push position.
    let pos = align_to_mask(unsafe { (*b).pos }, align_mask);

    // SAFETY: `b` is the live `Buf`; reading its `unordered` flag and position.
    if !unsafe { (*b).unordered } && pos != unsafe { (*b).pos } {
        // Alignment mismatch in an unordered block. Align to 16 bytes to guarantee
        // sufficient alignment for anything afterwards and mark the padding.
        // If we overflow the current block we don't need to care as the block
        // boundaries are not contiguous.
        // NOTE(ufbx-rs-native): the C comment says "unordered"; the guarded path
        // is the `!b->unordered` (ordered) one.
        // SAFETY: `b` is the live `Buf`; reading its current position.
        let pos = align_to_mask(unsafe { (*b).pos }, 0xf);
        // C-parity: `b->size - pos` (ufbx.c:4051) wraps if the 16-aligned
        // position passed the chunk end, yielding a huge value that would make
        // the `<=` check PASS — identical to C. Unreachable in practice: chunk
        // sizes are 16-aligned, so `pos <= b->size` always. Do not replace this
        // with a checked subtraction: that would diverge from C.
        // SAFETY: `b` is the live `Buf`; reading its current chunk size.
        if total < usize::MAX - 16 && total + 16 <= unsafe { (*b).size }.wrapping_sub(pos) {
            // SAFETY: `b` is the live `Buf`; `chunks[0]` is its live active
            // chunk (`pos != b.pos` means `b.pos` is unaligned hence nonzero,
            // so a prior push installed `chunks[0]`), so
            // `chunk_data(chunk).add(pos)` lands inside its `data` array — `pos`
            // is 16-aligned and `pos + 16 + total <= b.size` by the check above.
            // `padding` addresses that in-bounds region, written as a
            // `BufPadding` record, then `+16` skips past it to the returned
            // block.
            let chunk = unsafe { (*b).chunks[0] };
            let padding = unsafe { chunk_data(chunk).add(pos) } as *mut BufPadding;
            unsafe {
                (*padding).original_pos = (*b).pos;
                (*padding).prev_padding = (*chunk).padding_pos;
                (*chunk).padding_pos = pos + 16 + 1;
                (*b).pos = pos + 16 + total;
            }
            (unsafe { (padding as *mut u8).add(16) }) as *mut c_void
        } else {
            // SAFETY: forwarding this fn's live-`Buf` contract to
            // `push_size_new_block`.
            unsafe { push_size_new_block(b, total) }
        }
    } else {
        // Try to push to the current block. Allocate a new block
        // if the aligned size doesn't fit.
        // C-parity: `b->size - pos` (ufbx.c:4065) wraps when `pos > b->size`
        // (aligned past the chunk end), yielding a huge value that would make
        // the `<=` check PASS — identical to C. Unreachable in practice: chunk
        // sizes are 16-aligned and `align_mask <= 15`, so `pos <= b->size`
        // always. Do not replace this with a checked subtraction: that would
        // diverge from C.
        // SAFETY: `b` is the live `Buf`; reading its current chunk size.
        if total <= unsafe { (*b).size }.wrapping_sub(pos) {
            // SAFETY: `b` is the live `Buf`; `total >= 1` (size > 0, n > 0) and
            // the check above force `b.size >= 1`, so `chunks[0]` is the live
            // active chunk (`b.size == chunks[0]->size`, 0 only when null), and
            // `pos + total <= b.size` keeps the returned pointer in-bounds of
            // its `data` array.
            unsafe { (*b).pos = pos + total };
            (unsafe { chunk_data((*b).chunks[0]).add(pos) }) as *mut c_void
        } else {
            // SAFETY: forwarding this fn's live-`Buf` contract to
            // `push_size_new_block`.
            unsafe { push_size_new_block(b, total) }
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
        // SAFETY: `b` addresses a live `Buf` (this fn's raw-pointer contract);
        // reading its live stored allocator back-pointer.
        let ator = unsafe { (*b).ator };
        ufbxi_check_return_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr((*ator).error) },
            // SAFETY: `ator` is the buf's live allocator; reading its alloc
            // counters.
            unsafe { (*ator).num_allocs < (*ator).max_allocs },
            core::ptr::null_mut(),
            "Allocation limit exceeded",
            "ator->num_allocs < ator->max_allocs"
        );
        // SAFETY: `ator` is the buf's live allocator; bumping its alloc count.
        unsafe { (*ator).num_allocs += 1 };
    }

    // C-parity: size_t add wraps in C; keep wrapping semantics (checklist #2).
    // SAFETY: `b` is the live `Buf`; updating its item count.
    unsafe { (*b).num_items = (*b).num_items.wrapping_add(n) };

    // Homogeneous arrays should always be aligned
    // SAFETY: `b` is the live `Buf`; reading its current push position.
    let pos = unsafe { (*b).pos };
    ufbxi_regression_assert!((pos & size_align_mask(size)) == 0);

    // Try to push to the current block. Allocate a new block
    // if the aligned size doesn't fit.
    // C-parity: `b->size - pos` unsigned arithmetic as in `ufbxi_push_size`
    // (see the comment there: a wrap would make the `<=` check PASS, as in C;
    // unreachable since `pos <= b->size` always holds).
    // SAFETY: `b` is the live `Buf`; reading its current chunk size.
    if total <= unsafe { (*b).size }.wrapping_sub(pos) {
        // SAFETY: `b` is the live `Buf`; `total >= 1` (size > 0, n > 0) and the
        // check above force `b.size >= 1`, so `chunks[0]` is the live active
        // chunk (`b.size == chunks[0]->size`, 0 only when null), and `pos +
        // total <= b.size` keeps the returned pointer in-bounds of its `data`
        // array.
        unsafe { (*b).pos = pos + total };
        (unsafe { chunk_data((*b).chunks[0]).add(pos) }) as *mut c_void
    } else {
        // SAFETY: forwarding this fn's live-`Buf` contract to
        // `push_size_new_block`.
        unsafe { push_size_new_block(b, total) }
    }
}

// ufbx.c:4107-4112 `ufbxi_push_size_zero`
#[inline(never)]
pub(crate) unsafe fn push_size_zero(b: *mut Buf, size: usize, n: usize) -> *mut c_void {
    // SAFETY: forwarding this fn's live-`Buf` contract to `push_size`.
    let ptr = unsafe { push_size(b, size, n) };
    if !ptr.is_null() {
        // SAFETY: on a non-null return `push_size` handed back a region of at
        // least `size * n` writable bytes (its allocation size); zeroing it.
        unsafe { core::ptr::write_bytes(ptr as *mut u8, 0, size.wrapping_mul(n)) };
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
    // SAFETY: forwarding this fn's live-`Buf` contract to `push_size`.
    let ptr = unsafe { push_size(b, size, n) };
    if !ptr.is_null() {
        // SAFETY: on a non-null return `push_size` handed back a fresh region of
        // `size * n` writable bytes, and `data` is the caller's readable source
        // of the same `size * n` bytes (non-null, asserted above; C:
        // `memcpy(ptr, data, size*n)`); the two regions are distinct.
        unsafe {
            core::ptr::copy_nonoverlapping(data as *const u8, ptr as *mut u8, size.wrapping_mul(n))
        };
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
    // SAFETY: forwarding this fn's live-`Buf` contract to `push_size_fast`.
    let ptr = unsafe { push_size_fast(b, size, n) };
    if !ptr.is_null() {
        // SAFETY: on a non-null return `push_size_fast` handed back a fresh
        // region of `size * n` writable bytes, and `data` is the caller's
        // readable source of the same `size * n` bytes (non-null, asserted
        // above; C: `memcpy(ptr, data, size*n)`); the two regions are distinct.
        unsafe {
            core::ptr::copy_nonoverlapping(data as *const u8, ptr as *mut u8, size.wrapping_mul(n))
        };
    }
    ptr
}

// ufbx.c:4138-4171 `ufbxi_buf_free_unused`
#[inline(never)]
pub(crate) unsafe fn buf_free_unused(b: *mut Buf) {
    // SAFETY: `b` addresses a live `Buf` (this fn's raw-pointer contract);
    // reading its `unordered` flag and `chunks[0]` head.
    ufbx_assert!(!unsafe { (*b).unordered });

    // SAFETY: `b` is the live `Buf`; reading its active chunk.
    let chunk = unsafe { (*b).chunks[0] };
    if chunk.is_null() {
        return;
    }

    // SAFETY: `chunk` is `b`'s non-null active chunk (checked above); its
    // `->next` chain holds live chunks allocated from `(*b).ator` — the
    // `free_chunk` contract for each, freed on its own iteration only.
    for c in unsafe { ChunkIter::forward((*chunk).next) } {
        unsafe { free_chunk((*b).ator, c) };
    }
    // SAFETY: `chunk` is `b`'s live active chunk; unlinking its freed tail.
    unsafe { (*chunk).next = core::ptr::null_mut() };

    // SAFETY: `chunk` and its `->prev` chain are live chunks of `b` allocated
    // from `(*b).ator` (the `free_chunk` contract); `b` is the live `Buf` whose
    // own fields the loop rewinds.
    // C: `while (b->pos == 0 && chunk)` — the link is read before the free.
    let mut chunks = unsafe { ChunkIter::backward(chunk) };
    while unsafe { (*b).pos } == 0 {
        let Some(c) = chunks.next() else {
            break;
        };
        let prev = c.prev();
        unsafe { free_chunk((*b).ator, c) };
        unsafe { (*b).chunks[0] = prev };
        if !prev.is_null() {
            unsafe {
                (*prev).next = core::ptr::null_mut();
                (*b).pos = (*prev).pushed_pos;
                (*b).size = (*prev).size;
            }
        } else {
            unsafe {
                (*b).pos = 0;
                (*b).size = 0;
            }
        }
    }
}

// ufbx.c:4173-4260 `ufbxi_pop_size`
#[inline(never)]
pub(crate) unsafe fn pop_size(b: *mut Buf, size: usize, n: usize, dst: *mut c_void, peek: bool) {
    // SAFETY: `b` addresses a live `Buf` (this fn's raw-pointer contract);
    // reading its `unordered` flag and `num_items`.
    ufbx_assert!(!unsafe { (*b).unordered });
    ufbx_assert!(size > 0);
    ufbx_assert!(unsafe { (*b).num_items } >= n);
    if !peek {
        // C-parity: size_t sub wraps in C; keep wrapping semantics (checklist #2).
        // SAFETY: `b` is the live `Buf`; decrementing its item count.
        unsafe { (*b).num_items = (*b).num_items.wrapping_sub(n) };
    }

    let mut ptr = dst as *mut u8;
    let mut bytes_left = size.wrapping_mul(n);

    // We've already pushed this, it better not overflow
    ufbx_assert!(!does_overflow(bytes_left, size, n));

    if !ptr.is_null() {
        // SAFETY: `dst` is the caller's destination for `size * n == bytes_left`
        // bytes (the pop contract); advancing to its one-past-the-end so the
        // chunk walk can fill it back-to-front.
        ptr = unsafe { ptr.add(bytes_left) };
        // SAFETY (every access in this copying pop loop): `b` is the live `Buf`;
        // `chunk` starts at its active `chunks[0]` and walks the `->prev` chain,
        // each a live `BufChunk` whose `data` array holds `pushed_pos`/`pos`
        // valid bytes; `chunk_data(chunk).add(pos)` reads inside that array, and
        // `ptr` retreats within the caller's `bytes_left`-byte destination as
        // bytes are consumed, so each `copy_nonoverlapping` stays in bounds of
        // two distinct objects. Over-pop dereferencing a null `chunk->prev`
        // matches C, guarded only by the `num_items` assert above.
        let mut pos = unsafe { (*b).pos };
        let mut chunk = unsafe { (*b).chunks[0] };
        loop {
            if bytes_left <= pos {
                // Rest of the data is in this single chunk
                pos -= bytes_left;
                if !peek {
                    unsafe { (*b).pos = pos };
                }
                ptr = unsafe { ptr.sub(bytes_left) };
                if bytes_left > 0 {
                    unsafe {
                        core::ptr::copy_nonoverlapping(chunk_data(chunk).add(pos), ptr, bytes_left);
                    }
                }
                break;
            } else {
                // Pop the whole chunk
                ptr = unsafe { ptr.sub(pos) };
                bytes_left -= pos;
                unsafe { core::ptr::copy_nonoverlapping(chunk_data(chunk), ptr, pos) };
                // C-parity: on over-pop `chunk->prev` may be NULL and C
                // dereferences it (crash), guarded only by the num_items
                // assert above — do not add a null check here.
                if !peek {
                    unsafe {
                        (*chunk).pushed_pos = 0;
                        chunk = (*chunk).prev;
                        (*b).chunks[0] = chunk;
                        (*b).size = (*chunk).size;
                    }
                } else {
                    chunk = unsafe { (*chunk).prev };
                }
                pos = unsafe { (*chunk).pushed_pos };
            }
        }
    } else {
        // SAFETY (every access in this discarding pop loop): `b` is the live
        // `Buf`; `chunk` starts at its active `chunks[0]` and walks the `->prev`
        // chain of live `BufChunk`s. No data is copied (null `dst`). Over-pop
        // dereferencing a null `chunk->prev` matches C, guarded only by the
        // `num_items` assert above.
        let mut pos = unsafe { (*b).pos };
        let mut chunk = unsafe { (*b).chunks[0] };
        loop {
            if bytes_left <= pos {
                // Rest of the data is in this single chunk
                pos -= bytes_left;
                if !peek {
                    unsafe { (*b).pos = pos };
                }
                break;
            } else {
                // Pop the whole chunk
                bytes_left -= pos;
                // C-parity: on over-pop `chunk->prev` may be NULL and C
                // dereferences it (crash), guarded only by the num_items
                // assert above — do not add a null check here.
                if !peek {
                    unsafe {
                        (*chunk).pushed_pos = 0;
                        chunk = (*chunk).prev;
                        (*b).chunks[0] = chunk;
                        (*b).size = (*chunk).size;
                    }
                } else {
                    chunk = unsafe { (*chunk).prev };
                }
                pos = unsafe { (*chunk).pushed_pos };
            }
        }
    }

    if !peek {
        // Check if we need to rewind past some alignment padding
        // SAFETY: `b` is the live `Buf`; reading its active chunk.
        let chunk = unsafe { (*b).chunks[0] };
        if !chunk.is_null() {
            // SAFETY: `b` is the live `Buf` and `chunk` is its non-null active
            // `BufChunk`; reading the current position and padding offset.
            let pos = unsafe { (*b).pos };
            let padding_pos = unsafe { (*chunk).padding_pos };
            if pos < padding_pos {
                ufbx_assert!(pos + 1 == padding_pos);
                // SAFETY: `padding_pos - 1` is one past the END of the 16-byte
                // `BufPadding` record `push_size` wrote (it set `padding_pos =
                // pos + 16 + 1` with the record at `pos`), so
                // `chunk_data(chunk).add(padding_pos - 1 - 16)` addresses the
                // record's in-bounds start inside `chunk`'s `data` array;
                // reading its saved fields and writing them back into
                // `b`/`chunk`.
                let padding =
                    unsafe { chunk_data(chunk).add(padding_pos - 1 - 16) } as *mut BufPadding;
                unsafe {
                    (*b).pos = (*padding).original_pos;
                    (*chunk).padding_pos = (*padding).prev_padding;
                }
            }
        }

        // Immediately free popped items if all the allocations are huge
        // as it means we want to have dedicated allocations for each push.
        // SAFETY: `b` is the live `Buf` and `(*b).ator` its live allocator;
        // reading `huge_size`, then forwarding the live-`Buf` contract to
        // `buf_free_unused`.
        if unsafe { (*(*b).ator).huge_size } <= 1 {
            unsafe { buf_free_unused(b) };
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
    // SAFETY: `dst`/`src` are live `Buf`s (this fn's raw-pointer contract);
    // forwarding to `push_size`.
    let data = unsafe { push_size(dst, size, n) };
    if data.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: `src` is a live `Buf`; `data` is non-null and valid as
    // `pop_size`'s destination for `size * n` bytes — the fresh region
    // `push_size` allocated in `dst`, or, when `n == 0`, the static
    // `ZERO_SIZE_BUFFER` into which `pop_size` writes zero bytes.
    unsafe { pop_size(src, size, n, data, false) };
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
    // SAFETY: `dst`/`src` are live `Buf`s (this fn's raw-pointer contract);
    // forwarding to `push_size`.
    let data = unsafe { push_size(dst, size, n) };
    if data.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: `src` is a live `Buf`; `data` is non-null and valid as
    // `pop_size`'s destination for `size * n` bytes — the fresh region
    // `push_size` allocated in `dst`, or, when `n == 0`, the static
    // `ZERO_SIZE_BUFFER` into which `pop_size` writes zero bytes.
    unsafe { pop_size(src, size, n, data, true) };
    data
}

// ufbx.c:4278-4297 `ufbxi_buf_free`
#[inline(never)]
pub(crate) unsafe fn buf_free(buf: *mut Buf) {
    // C: `ufbxi_nounroll` — optimizer pragma, no Rust analogue (platform.rs).
    // SAFETY: `buf` addresses a live `Buf` (this fn's raw-pointer contract);
    // each `chunks[i]` head and the chain from its `->root` are live chunks
    // allocated from `(*buf).ator` — the `free_chunk` contract for each.
    for i in 0..2usize {
        let chunk = unsafe { (*buf).chunks[i] };
        if !chunk.is_null() {
            for c in unsafe { ChunkIter::forward((*chunk).root) } {
                unsafe { free_chunk((*buf).ator, c) };
            }
        }
        unsafe { (*buf).chunks[i] = core::ptr::null_mut() };
    }
    // SAFETY: `buf` is the live `Buf`; resetting its inline state fields.
    unsafe {
        (*buf).pos = 0;
        (*buf).size = 0;
        (*buf).num_items = 0;
    }
}

// ufbx.c:4299-4344 `ufbxi_buf_clear`
#[inline(never)]
pub(crate) unsafe fn buf_clear(buf: *mut Buf) {
    // Only unordered or clearable buffers can be cleared
    // SAFETY: `buf` addresses a live `Buf` (this fn's raw-pointer contract);
    // reading its `unordered`/`clearable` flags.
    ufbx_assert!(!unsafe { (*buf).unordered } || unsafe { (*buf).clearable });

    // Free the memory if using ASAN
    // SAFETY: `buf` is the live `Buf` and `(*buf).ator` its live allocator;
    // reading `huge_size`, then forwarding the live-`Buf` contract to `buf_free`.
    if unsafe { (*(*buf).ator).huge_size } <= 1 {
        unsafe { buf_free(buf) };
        return;
    }

    // Reset the non-huge chunks as `chunk->next` is always free.
    // SAFETY: `buf` is the live `Buf`; `chunks[0]` and its `->root` are live
    // `BufChunk`s of the non-huge list. Resetting the buf to its root chunk.
    let chunk = unsafe { (*buf).chunks[0] };
    if !chunk.is_null() {
        let root = unsafe { (*chunk).root };
        unsafe {
            (*buf).chunks[0] = root;
            (*buf).pos = 0;
            (*buf).size = (*root).size;
        }
    }
    // SAFETY: `buf` is the live `Buf`; resetting its item/pushed counters.
    unsafe {
        (*buf).num_items = 0;
        (*buf).pushed_size = 0;
    }

    // Huge chunks are always sorted by descending size and
    // `chunks[1]` points to the largest one.
    // SAFETY: `buf` is the live `Buf`; reading the huge-list head.
    let huge = unsafe { (*buf).chunks[1] };
    if !huge.is_null() {
        // Reset the first N ones that are tracked.
        // SAFETY: `huge` heads the live `->next` chain of `buf`'s huge list,
        // allocated from `(*buf).ator` (the `free_chunk` contract below).
        let mut chunks = unsafe { ChunkIter::forward(huge) };
        let mut i = 0usize;
        // C: `for (size_t i = 0; huge && i < UFBXI_HUGE_MAX_SCAN; i++)`
        while i < HUGE_MAX_SCAN {
            let Some(c) = chunks.next() else {
                break;
            };
            c.set_pushed_pos(0);
            i += 1;
        }

        // Got unreachable tail that should be freed: Unlink from the last
        // tracked chunk and free the rest.
        let huge = chunks.cursor();
        if !huge.is_null() {
            // SAFETY: `huge` is non-null here only because the reset loop ran
            // the full `HUGE_MAX_SCAN` iterations, so its `->prev` is the last
            // tracked live `BufChunk` (non-null); unlinking the tail from it.
            unsafe { (*(*huge).prev).next = core::ptr::null_mut() };
            // SAFETY: the unreachable tail is a live chain allocated from
            // `(*buf).ator`; each chunk is freed on its own iteration only.
            for c in unsafe { ChunkIter::forward(huge) } {
                unsafe { free_chunk((*buf).ator, c) };
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
    // SAFETY: forwarding this fn's live-`Buf` contract to `push_size`.
    (unsafe { push_size(b, size_of::<T>(), n) }) as *mut T
}

// ufbx.c:4347 `#define ufbxi_push_zero(b, type, n)`
#[inline(always)]
pub(crate) unsafe fn push_zero<T>(b: *mut Buf, n: usize) -> *mut T {
    // SAFETY: forwarding this fn's live-`Buf` contract to `push_size_zero`.
    (unsafe { push_size_zero(b, size_of::<T>(), n) }) as *mut T
}

// ufbx.c:4348 `#define ufbxi_push_copy(b, type, n, data)`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn push_copy<T>(b: *mut Buf, n: usize, data: *const T) -> *mut T {
    // SAFETY: forwarding this fn's live-`Buf` and readable-`data` contract to
    // `push_size_copy`.
    (unsafe { push_size_copy(b, size_of::<T>(), n, data as *const c_void) }) as *mut T
}

// ufbx.c:4349 `#define ufbxi_push_copy_fast(b, type, n, data)`
#[inline(always)]
#[must_use]
pub(crate) unsafe fn push_copy_fast<T>(b: *mut Buf, n: usize, data: *const T) -> *mut T {
    // SAFETY: forwarding this fn's live-`Buf` and readable-`data` contract to
    // `push_size_copy_fast`.
    (unsafe { push_size_copy_fast(b, size_of::<T>(), n, data as *const c_void) }) as *mut T
}

// ufbx.c:4350 `#define ufbxi_push_fast(b, type, n)`
#[inline(always)]
pub(crate) unsafe fn push_fast<T>(b: *mut Buf, n: usize) -> *mut T {
    // SAFETY: forwarding this fn's live-`Buf` contract to `push_size_fast`.
    (unsafe { push_size_fast(b, size_of::<T>(), n) }) as *mut T
}

// ufbx.c:4351 `#define ufbxi_pop(b, type, n, dst)`
#[inline(always)]
pub(crate) unsafe fn pop<T>(b: *mut Buf, n: usize, dst: *mut T) {
    // SAFETY: forwarding this fn's live-`Buf` and writable-`dst` contract to
    // `pop_size`.
    unsafe { pop_size(b, size_of::<T>(), n, dst as *mut c_void, false) }
}

// ufbx.c:4352 `#define ufbxi_peek(b, type, n, dst)`
// C-parity: the `ufbxi_peek` macro has zero call sites in ufbx.c (only its
// sibling `ufbxi_pop` is used); kept for 1:1 coverage of the pop/peek family.
#[allow(dead_code)]
#[inline(always)]
pub(crate) unsafe fn peek<T>(b: *mut Buf, n: usize, dst: *mut T) {
    // SAFETY: forwarding this fn's live-`Buf` and writable-`dst` contract to
    // `pop_size`.
    unsafe { pop_size(b, size_of::<T>(), n, dst as *mut c_void, true) }
}

// ufbx.c:4353 `#define ufbxi_push_pop(dst, src, type, n)`
#[inline(always)]
pub(crate) unsafe fn push_pop<T>(dst: *mut Buf, src: *mut Buf, n: usize) -> *mut T {
    // SAFETY: forwarding this fn's live-`Buf` contract for `dst`/`src` to
    // `push_pop_size`.
    (unsafe { push_pop_size(dst, src, size_of::<T>(), n) }) as *mut T
}

// ufbx.c:4354 `#define ufbxi_push_peek(dst, src, type, n)`
#[inline(always)]
pub(crate) unsafe fn push_peek<T>(dst: *mut Buf, src: *mut Buf, n: usize) -> *mut T {
    // SAFETY: forwarding this fn's live-`Buf` contract for `dst`/`src` to
    // `push_peek_size`.
    (unsafe { push_peek_size(dst, src, size_of::<T>(), n) }) as *mut T
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::Error;
    use crate::native::allocator::init_ator;
    use core::mem::MaybeUninit;

    fn make_buf(ator: *mut Allocator, unordered: bool, clearable: bool) -> Buf {
        // SAFETY: `Buf` is all pointers/usizes/bools; the all-zero bit
        // pattern is a valid value for every field (null pointers, zero
        // sizes, `false`). No `chunk`/`ator` dereference happens here.
        let mut buf = unsafe { MaybeUninit::<Buf>::zeroed().assume_init() };
        buf.ator = ator;
        buf.unordered = unordered;
        buf.clearable = clearable;
        buf
    }

    fn free_all_chunks(b: &mut Buf) {
        // Test-only teardown (separate from `buf_free` above so these tests
        // don't depend on it, sharing only `free_chunk`): walk both chunk
        // lists from their roots and free every chunk.
        for list_ix in 0..2 {
            let chunk = b.chunks[list_ix];
            if chunk.is_null() {
                continue;
            }
            // SAFETY: `b` is a test fixture the caller owns exclusively
            // (`&mut Buf`); its `chunks[list_ix]` entry is a live chunk of the
            // `root`-linked list this buf pushed, each chunk allocated from
            // `b.ator` with `sizeof(BufChunk) + size` bytes. Nothing else
            // holds these chunks, so freeing them here is the last use.
            unsafe {
                for c in ChunkIter::forward((*chunk).root) {
                    free_chunk(b.ator, c);
                }
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
        let mut err = Error::default();
        let mut ator = MaybeUninit::<Allocator>::zeroed();
        unsafe {
            init_ator(&mut err, ator.as_mut_ptr(), core::ptr::null(), c"test");
        }
        let ator = ator.as_mut_ptr();
        let mut buf = make_buf(ator, false, false);

        // Zero-count push returns the shared zero-size buffer.
        let z = unsafe { push_size(&mut buf, 4, 0) };
        assert_eq!(z as *const u8, ZERO_SIZE_BUFFER.as_ptr());
        assert_eq!(buf.num_items, 0);

        let p = unsafe { push_size(&mut buf, 4, 3) } as *mut u32;
        assert!(!p.is_null());
        assert_eq!(buf.num_items, 3);
        // First chunk: next_size 4096, chunk_size = 4096 - header, 16-aligned.
        let expect_size = align_to_mask(4096 - size_of::<BufChunk>(), 0xf);
        assert_eq!(buf.size, expect_size);
        assert_eq!(buf.pos, 12);
        assert_eq!(
            unsafe { (*buf.chunks[0]).magic },
            BUF_CHUNK_IMP_MAGIC as usize
        );

        // Aligned follow-up push in the same chunk (u64 after 12 bytes pads to 16,
        // ordered buffer writes a 16-byte padding record).
        let q = unsafe { push_size(&mut buf, 8, 1) } as *mut u64;
        assert!(!q.is_null());
        unsafe {
            *q = 0x1122334455667788;
        }
        assert_eq!(buf.pos, 16 + 16 + 8);
        assert_eq!(unsafe { (*buf.chunks[0]).padding_pos }, 16 + 16 + 1);

        free_all_chunks(&mut buf);
        assert_eq!(unsafe { (*ator).current_size }, 0);
    }

    #[test]
    fn test_new_block_growth_doubling() {
        let mut err = Error::default();
        let mut ator = MaybeUninit::<Allocator>::zeroed();
        unsafe {
            init_ator(&mut err, ator.as_mut_ptr(), core::ptr::null(), c"test");
        }
        let ator = ator.as_mut_ptr();
        let mut buf = make_buf(ator, false, false);

        let p = unsafe { push_size(&mut buf, 1, 100) };
        assert!(!p.is_null());
        assert_eq!(unsafe { (*buf.chunks[0]).next_size }, 4096);

        // Overflow the first chunk: next chunk doubles next_size.
        let big = buf.size; // larger than remaining space
        let q = unsafe { push_size(&mut buf, 1, big) };
        assert!(!q.is_null());
        assert_eq!(unsafe { (*buf.chunks[0]).next_size }, 8192);
        // Retired chunk stored its final position.
        assert_eq!(unsafe { (*(*buf.chunks[0]).prev).pushed_pos }, 100);
        assert_eq!(buf.pushed_size, 100);

        free_all_chunks(&mut buf);
        assert_eq!(unsafe { (*ator).current_size }, 0);
    }

    #[test]
    fn test_huge_unordered_second_list() {
        let mut err = Error::default();
        let mut ator = MaybeUninit::<Allocator>::zeroed();
        unsafe {
            init_ator(&mut err, ator.as_mut_ptr(), core::ptr::null(), c"test");
        }
        let ator = ator.as_mut_ptr();
        let mut buf = make_buf(ator, true, false);

        let huge = unsafe { (*ator).huge_size }; // 0x100000
        let p = unsafe { push_size(&mut buf, 1, huge) };
        assert!(!p.is_null());
        // Huge unordered pushes go to chunks[1]; chunks[0]/pos/size untouched.
        assert!(buf.chunks[0].is_null());
        assert!(!buf.chunks[1].is_null());
        assert_eq!(buf.pos, 0);
        assert_eq!(unsafe { (*buf.chunks[1]).pushed_pos }, huge);
        assert_eq!(buf.pushed_size, huge);

        free_all_chunks(&mut buf);
        assert_eq!(unsafe { (*ator).current_size }, 0);
    }

    #[test]
    fn test_push_zero_and_copy() {
        let mut err = Error::default();
        let mut ator = MaybeUninit::<Allocator>::zeroed();
        unsafe {
            init_ator(&mut err, ator.as_mut_ptr(), core::ptr::null(), c"test");
        }
        let ator = ator.as_mut_ptr();
        let mut buf = make_buf(ator, false, false);

        let p = unsafe { push_size_zero(&mut buf, 1, 32) } as *mut u8;
        assert!(!p.is_null());
        for i in 0..32 {
            assert_eq!(unsafe { *p.add(i) }, 0);
        }

        let src: [u32; 4] = [1, 2, 3, 4];
        let q = unsafe { push_size_copy(&mut buf, 4, 4, src.as_ptr() as *const core::ffi::c_void) }
            as *mut u32;
        assert!(!q.is_null());
        for i in 0..4 {
            assert_eq!(unsafe { *q.add(i) }, src[i]);
        }

        // Copy with n == 0 succeeds even with NULL data.
        let z = unsafe { push_size_copy(&mut buf, 4, 0, core::ptr::null()) };
        assert_eq!(z as *const u8, ZERO_SIZE_BUFFER.as_ptr());

        free_all_chunks(&mut buf);
        assert_eq!(unsafe { (*ator).current_size }, 0);
    }

    #[test]
    fn test_pop_and_peek_across_chunks() {
        let mut err = Error::default();
        let mut ator = MaybeUninit::<Allocator>::zeroed();
        unsafe {
            init_ator(&mut err, ator.as_mut_ptr(), core::ptr::null(), c"test");
        }
        let ator = ator.as_mut_ptr();
        let mut buf = make_buf(ator, false, false);

        // Push enough u32 items to span multiple chunks (first chunk holds
        // ~4032 bytes; 4096 items = 16384 bytes).
        const N: usize = 4096;
        for i in 0..N {
            let p = unsafe { push::<u32>(&mut buf, 1) };
            assert!(!p.is_null());
            unsafe {
                *p = i as u32;
            }
        }
        assert_eq!(buf.num_items, N);
        assert!(
            unsafe { !(*buf.chunks[0]).prev.is_null() },
            "must span chunks"
        );

        // Peek the last 100 items — non-destructive; flattening walks the
        // chunk chain backwards.
        let mut out = [0u32; 100];
        unsafe {
            peek::<u32>(&mut buf, 100, out.as_mut_ptr());
        }
        for i in 0..100 {
            assert_eq!(out[i], (N - 100 + i) as u32);
        }
        assert_eq!(buf.num_items, N);

        // Pop all items in chunks of 300, spanning chunk boundaries.
        let mut remaining = N;
        let mut dst = [0u32; 300];
        while remaining > 0 {
            let take = remaining.min(300);
            unsafe {
                pop::<u32>(&mut buf, take, dst.as_mut_ptr());
            }
            for i in 0..take {
                assert_eq!(dst[i], (remaining - take + i) as u32);
            }
            remaining -= take;
        }
        assert_eq!(buf.num_items, 0);
        assert_eq!(buf.pos, 0);

        unsafe {
            buf_free(&mut buf);
        }
        assert_eq!(unsafe { (*ator).current_size }, 0);
    }

    #[test]
    fn test_pop_null_dst_rewinds_padding() {
        let mut err = Error::default();
        let mut ator = MaybeUninit::<Allocator>::zeroed();
        unsafe {
            init_ator(&mut err, ator.as_mut_ptr(), core::ptr::null(), c"test");
        }
        let ator = ator.as_mut_ptr();
        let mut buf = make_buf(ator, false, false);

        // 12 bytes, then an 8-aligned push forces a padding record.
        let _ = unsafe { push_size(&mut buf, 4, 3) };
        let q = unsafe { push_size(&mut buf, 8, 1) };
        assert!(!q.is_null());
        assert_eq!(buf.pos, 16 + 16 + 8);
        assert_eq!(unsafe { (*buf.chunks[0]).padding_pos }, 16 + 16 + 1);

        // Discarding pop (dst == NULL) rewinds through the padding record.
        unsafe {
            pop_size(&mut buf, 8, 1, core::ptr::null_mut(), false);
        }
        assert_eq!(buf.pos, 12);
        assert_eq!(unsafe { (*buf.chunks[0]).padding_pos }, 0);

        unsafe {
            buf_free(&mut buf);
        }
        assert_eq!(unsafe { (*ator).current_size }, 0);
    }

    #[test]
    fn test_push_pop_flatten() {
        let mut err = Error::default();
        let mut ator = MaybeUninit::<Allocator>::zeroed();
        unsafe {
            init_ator(&mut err, ator.as_mut_ptr(), core::ptr::null(), c"test");
        }
        let ator = ator.as_mut_ptr();
        let mut stack = make_buf(ator, false, false);
        let mut result = make_buf(ator, false, false);

        const N: usize = 3000;
        for i in 0..N {
            let p = unsafe { push::<u64>(&mut stack, 1) };
            assert!(!p.is_null());
            unsafe {
                *p = i as u64;
            }
        }
        assert!(
            unsafe { !(*stack.chunks[0]).prev.is_null() },
            "must span chunks"
        );

        // Flatten the non-contiguous stack into a contiguous array.
        let arr = unsafe { push_pop::<u64>(&mut result, &mut stack, N) };
        assert!(!arr.is_null());
        for i in 0..N {
            assert_eq!(unsafe { *arr.add(i) }, i as u64);
        }
        assert_eq!(stack.num_items, 0);
        assert_eq!(result.num_items, N);

        unsafe {
            buf_free(&mut stack);
            buf_free(&mut result);
        }
        assert_eq!(unsafe { (*ator).current_size }, 0);
    }

    #[test]
    fn test_buf_free_unused_frees_forward_chunks() {
        let mut err = Error::default();
        let mut ator = MaybeUninit::<Allocator>::zeroed();
        unsafe {
            init_ator(&mut err, ator.as_mut_ptr(), core::ptr::null(), c"test");
        }
        let ator = ator.as_mut_ptr();
        let mut buf = make_buf(ator, false, false);

        // Span two chunks, then pop everything back to zero.
        let n1 = 4000usize;
        unsafe {
            let _ = push_size(&mut buf, 1, n1);
            let _ = push_size(&mut buf, 1, 1000);
        }
        assert!(unsafe { !(*buf.chunks[0]).prev.is_null() });
        unsafe {
            pop_size(&mut buf, 1, 1000, core::ptr::null_mut(), false);
            pop_size(&mut buf, 1, n1, core::ptr::null_mut(), false);
        }
        assert_eq!(buf.pos, 0);

        // Frees the empty head chunks and the retired next-chain entirely.
        unsafe {
            buf_free_unused(&mut buf);
        }
        assert!(buf.chunks[0].is_null());
        assert_eq!(buf.size, 0);
        assert_eq!(unsafe { (*ator).current_size }, 0);

        unsafe {
            buf_free(&mut buf);
        }
    }

    #[test]
    fn test_buf_clear_resets_and_trims_huge() {
        let mut err = Error::default();
        let mut ator = MaybeUninit::<Allocator>::zeroed();
        unsafe {
            init_ator(&mut err, ator.as_mut_ptr(), core::ptr::null(), c"test");
        }
        let ator = ator.as_mut_ptr();
        let mut buf = make_buf(ator, true, true);

        // Normal chunks plus more huge chunks than UFBXI_HUGE_MAX_SCAN.
        unsafe {
            let _ = push_size(&mut buf, 1, 100);
        }
        let huge = unsafe { (*ator).huge_size };
        for i in 0..(HUGE_MAX_SCAN + 4) {
            let p = unsafe { push_size(&mut buf, 1, huge + i) };
            assert!(!p.is_null());
        }
        assert!(!buf.chunks[1].is_null());

        unsafe {
            buf_clear(&mut buf);
        }
        assert_eq!(buf.pos, 0);
        assert_eq!(buf.num_items, 0);
        assert_eq!(buf.pushed_size, 0);
        // chunks[0] rewound to root.
        assert_eq!(buf.chunks[0], unsafe { (*buf.chunks[0]).root });
        // Exactly HUGE_MAX_SCAN huge chunks remain, each reset.
        let mut count = 0usize;
        let mut c = buf.chunks[1];
        while !c.is_null() {
            assert_eq!(unsafe { (*c).pushed_pos }, 0);
            count += 1;
            c = unsafe { (*c).next };
        }
        assert_eq!(count, HUGE_MAX_SCAN);

        unsafe {
            buf_free(&mut buf);
        }
        assert_eq!(unsafe { (*ator).current_size }, 0);
    }
}
