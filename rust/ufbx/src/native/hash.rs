//! Port of the `// -- Hash functions` banner section (ufbx.c:4702-4821) and
//! the `// -- Hash map` banner section (ufbx.c:4356-4700: ufbxi_map struct,
//! AA-tree overflow storage, open-addressing Robin Hood grow/find/insert, the
//! `ufbxi_map_cmp_*` comparators). The hash functions come first (they were
//! ported first); the hash map follows below them.
//!
//! Phase 1: not all items have consumers yet.
#![allow(dead_code, unused_macros, unused_imports)]
// Ratchet allow (PORTING.md "Unsafe reduction / isolation strategy"): this
// file still has whole-body-implicit unsafe fns; remove this allow once every
// op inside its unsafe fns sits in a narrow annotated `unsafe {}` block.
#![allow(unsafe_op_in_unsafe_fn)]
use core::ffi::c_void;
use core::mem::size_of;

use crate::native::allocator::{alloc, free, free_ator, ufbx_free, ufbx_malloc, Allocator};
use crate::native::buf::{buf_free, push, Buf};
use crate::native::error::{ufbxi_check_return_err, ufbxi_check_return_err_msg};
use crate::native::platform::{read_u32, ufbx_assert, ufbxi_maybe_null, ufbxi_regression_assert};

// ufbx.c:4688-4691 `ufbxi_ptr_id` — key type of the hash-map unit; kept up
// here (out of C declaration order) because `ufbxi_hash_ptr_id` takes it by
// value and the hash functions were ported first. The comparator
// `ufbxi_map_cmp_ptr_id` (ufbx.c:4693-4700) lives with the map port below.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PtrId {
    pub ptr: usize,
    pub id: u64,
}

// ufbx.c:4704-4730 `ufbxi_hash_string`
#[inline(never)]
pub(crate) unsafe fn hash_string(mut str_: *const u8, mut length: usize) -> u32 {
    let mut hash = length as u32;
    let seed = 0x9e3779b9u32;
    if length >= 4 {
        loop {
            let word = read_u32(str_);
            hash = ((hash << 5 | hash >> 27) ^ word).wrapping_mul(seed);
            str_ = str_.add(4);
            length -= 4;
            if !(length >= 4) {
                break;
            }
        }

        let word = read_u32(str_.add(length).sub(4));
        hash = ((hash << 5 | hash >> 27) ^ word).wrapping_mul(seed);
    } else {
        let mut word = 0u32;
        if length >= 1 {
            word |= (*str_.add(0) as u32) << 0;
        }
        if length >= 2 {
            word |= (*str_.add(1) as u32) << 8;
        }
        if length >= 3 {
            word |= (*str_.add(2) as u32) << 16;
        }
        hash = ((hash << 5 | hash >> 27) ^ word).wrapping_mul(seed);
    }
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb352d);
    hash ^= hash >> 15;
    hash
}

// ufbx.c:4732-4779 `ufbxi_hash_string_check_ascii`
// NOTE: _Must_ match `ufbxi_hash_string()`
#[inline(never)]
pub(crate) unsafe fn hash_string_check_ascii(
    mut str_: *const u8,
    mut length: usize,
    p_non_ascii: *mut bool,
) -> u32 {
    let mut ascii_mask = 0u32;
    let mut zero_mask = 0u32;

    ufbx_assert!(length > 0);

    let mut hash = length as u32;
    let seed = 0x9e3779b9u32;
    if length >= 4 {
        loop {
            let word = read_u32(str_);
            ascii_mask |= word;
            zero_mask |= 0x80808080u32.wrapping_sub(word);

            hash = ((hash << 5 | hash >> 27) ^ word).wrapping_mul(seed);
            str_ = str_.add(4);
            length -= 4;
            if !(length >= 4) {
                break;
            }
        }

        let word = read_u32(str_.add(length).sub(4));
        ascii_mask |= word;
        zero_mask |= 0x80808080u32.wrapping_sub(word);

        hash = ((hash << 5 | hash >> 27) ^ word).wrapping_mul(seed);
    } else {
        let mut word = 0u32;
        if length >= 1 {
            word |= (*str_.add(0) as u32) << 0;
        }
        if length >= 2 {
            word |= (*str_.add(1) as u32) << 8;
        }
        if length >= 3 {
            word |= (*str_.add(2) as u32) << 16;
        }

        ascii_mask |= word;
        // C-parity: at length == 0 the C shift amount is 32 (UB in C,
        // ufbx.c:4764; masks to `>> 0` on x86/ARM) and would be a debug-build
        // overflow panic here. Unreachable today via the unconditional
        // `ufbx_assert!(length > 0)` above; if that assert is ever feature-
        // gated off (no-assert), this shift must gain a `& 31` mask.
        zero_mask |= (0x80808080u32 >> ((4 - length) * 8)).wrapping_sub(word);

        hash = ((hash << 5 | hash >> 27) ^ word).wrapping_mul(seed);
    }

    // If any character has high bit set or is zero we're not ASCII
    if ((ascii_mask | zero_mask) & 0x80808080u32) != 0 {
        *p_non_ascii = true;
    }

    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb352d);
    hash ^= hash >> 15;

    hash
}

// ufbx.c:4781-4789 `ufbxi_hash32`
#[inline(always)]
pub(crate) fn hash32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846ca68b);
    x ^= x >> 16;
    x
}

// ufbx.c:4791-4799 `ufbxi_hash64`
#[inline(always)]
pub(crate) fn hash64(mut x: u64) -> u32 {
    x ^= x >> 32;
    x = x.wrapping_mul(0xd6e8feb86659fd93);
    x ^= x >> 32;
    x = x.wrapping_mul(0xd6e8feb86659fd93);
    x ^= x >> 32;
    x as u32
}

// ufbx.c:4801-4812 `ufbxi_hash_uptr`
// The C three-way `UFBXI_UINTPTR_SIZE` fork (8 / 4 / unknown-at-preprocess)
// maps to `target_pointer_width` cfgs; the runtime-sizeof fallback branch
// (CHERI targets, ufbx.c:862) has no rustc analogue — the byte-hash arm below
// preserves its behavior for any other pointer width.
#[inline(always)]
pub(crate) fn hash_uptr(ptr: usize) -> u32 {
    #[cfg(target_pointer_width = "64")]
    {
        hash64(ptr as u64)
    }
    #[cfg(target_pointer_width = "32")]
    {
        hash32(ptr as u32)
    }
    #[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
    {
        // C fallback: hash the pointer's bytes.
        unsafe {
            hash_string(
                &ptr as *const usize as *const u8,
                core::mem::size_of::<usize>(),
            )
        }
    }
}

// ufbx.c:4814-4818 `ufbxi_hash_ptr_id`
#[inline(always)]
pub(crate) fn hash_ptr_id(id: PtrId) -> u32 {
    // Trivial reduction is fine: Only `ptr` or `id` is defined.
    hash_uptr(id.ptr) ^ hash64(id.id)
}

// ufbx.c:4820 `#define ufbxi_hash_ptr(ptr) ufbxi_hash_uptr((uintptr_t)(ptr))`
macro_rules! hash_ptr {
    ($ptr:expr) => {
        $crate::native::hash::hash_uptr(($ptr) as usize)
    };
}
pub(crate) use hash_ptr;

// -- Hash map (ufbx.c:4356-4700)
//
// The actual element comparison is left to the user of `ufbxi_map`, see usage below.
//
// NOTES:
//   ufbxi_map_insert() does not support duplicate values, use find first if duplicates are possible!
//   Inserting duplicate elements fails with an assertion if `UFBX_REGRESSION` is enabled.

// ufbx.c:55 `#define UFBXI_MAP_MAX_SCAN 32`
#[cfg(not(feature = "regression"))]
pub(crate) const MAP_MAX_SCAN: u32 = 32;
// ufbx.c:1004-1005 regression override
#[cfg(feature = "regression")]
pub(crate) const MAP_MAX_SCAN: u32 = 2;

// ufbx.c:4366 `typedef int ufbxi_cmp_fn(void *user, const void *a, const void *b);`
pub(crate) type CmpFn =
    unsafe extern "C" fn(user: *mut c_void, a: *const c_void, b: *const c_void) -> i32;

// ufbx.c:4364-4371 `ufbxi_aa_node`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct AaNode {
    pub left: *mut AaNode,
    pub right: *mut AaNode,
    pub level: u32,
    pub index: u32,
}

// ufbx.c:4373-4391 `ufbxi_map`
// C-parity: `cmp_fn` is a plain (never-null after `ufbxi_map_init`) function
// pointer in C; `Option` here only so a zero-initialized map (C callers memset
// the containing context) is representable — call sites use
// `unwrap_unchecked`, matching C's unchecked indirect call.
// NOT `Copy`/`Clone`: owns its allocations (`items`/`entries`, freed by
// `map_free`) — see PORTING.md "Copy vs non-Copy structs".
#[repr(C)]
pub(crate) struct Map {
    pub ator: *mut Allocator,
    pub data_size: usize,

    pub items: *mut c_void,
    pub entries: *mut u64,
    pub mask: u32,

    pub capacity: u32,
    pub size: u32,

    pub cmp_fn: Option<CmpFn>,
    pub cmp_user: *mut c_void,

    pub aa_buf: Buf,
    pub aa_root: *mut AaNode,
}

// Typed interior-mutable VIEW over an owned `Map` field, reinterpreted in place.
// Read-only leaf getters for the sites that inspect an owned map's size/items.
pub(crate) type MapView = crate::native::view::View<Map>;

impl MapView {
    #[inline(always)]
    pub(crate) fn size(&self) -> u32 {
        unsafe { (*self.get()).size }
    }
    #[inline(always)]
    pub(crate) fn items(&self) -> *mut core::ffi::c_void {
        unsafe { (*self.get()).items }
    }

    // Safe typed map operations over the view (ufbx.c:4657-4659 macros).
    //
    // `K` is the key type the map's `cmp_fn` was initialized for (untyped
    // `const void *` in C). Key validity: the cmp may dereference pointers
    // *inside* `K` (e.g. `map_cmp_const_char_ptr` follows the stored `char *`
    // to a NUL-terminated string) — callers must pass keys meeting that
    // pointee contract, same standing as the printf `%s` PrintArg contract.

    // ufbx.c:4657 `ufbxi_map_grow(map, type, min_size)`
    #[inline(always)]
    pub(crate) fn grow<T>(&self, min_size: usize) -> bool {
        // SAFETY: the view is only minted over a live, `map_init`ed `Map`
        // (write provenance); growth allocates through the map's own stored
        // allocator, live for the map's lifetime.
        unsafe { map_grow::<T>(self.get(), min_size) }
    }

    // ufbx.c:4658 `ufbxi_map_find(map, type, hash, key)`
    #[inline(always)]
    pub(crate) fn find<T, K>(&self, hash: u32, key: &K) -> *mut T {
        // SAFETY: view invariant as in `grow`; `cmp_fn` is the C-callback
        // contract the map was initialized with, comparing `key` against
        // stored items of the same key discipline (see impl-level note).
        unsafe { map_find::<T>(self.get(), hash, key as *const K as *const c_void) }
    }

    // ufbx.c:4659 `ufbxi_map_insert(map, type, hash, key)`
    #[inline(always)]
    #[must_use]
    pub(crate) fn insert<T, K>(&self, hash: u32, key: &K) -> *mut T {
        // SAFETY: same as `find`; insertion may grow through the map's own
        // stored allocator.
        unsafe { map_insert::<T>(self.get(), hash, key as *const K as *const c_void) }
    }
}

// ufbx.c:4393-4420 `ufbxi_map_init`
#[inline(never)]
pub(crate) unsafe fn map_init(
    map: *mut Map,
    ator: *mut Allocator,
    cmp_fn: CmpFn,
    cmp_user: *mut c_void,
) {
    (*map).ator = ator;
    #[cfg(feature = "regression")]
    {
        // HACK: Maps contain pointers that are not stable between runs, in regression
        // mode this causes instability in allocation patterns due to different AA trees
        // being built, which is a problem in fuzz checks that need to have deterministic
        // allocation counts. We can work around this using a local allocator that doesn't
        // count the allocations.
        {
            let regression_ator = ufbx_malloc(size_of::<Allocator>()) as *mut Allocator;
            ufbx_assert!(!regression_ator.is_null());
            core::ptr::write_bytes(regression_ator as *mut u8, 0, size_of::<Allocator>());
            (*regression_ator).name = b"regression\0".as_ptr();
            (*regression_ator).error = (*ator).error;
            (*regression_ator).huge_size = (*ator).huge_size;
            (*regression_ator).max_size = usize::MAX;
            (*regression_ator).max_allocs = usize::MAX;
            (*regression_ator).chunk_max = 0x1000000;
            (*map).aa_buf.ator = regression_ator;
        }
    }
    #[cfg(not(feature = "regression"))]
    {
        (*map).aa_buf.ator = ator;
    }
    (*map).cmp_fn = Some(cmp_fn);
    (*map).cmp_user = cmp_user;
}

// ufbx.c:4421-4441 `ufbxi_map_free`
#[inline(never)]
pub(crate) unsafe fn map_free(map: *mut Map) {
    #[cfg(feature = "regression")]
    let regression_ator: *mut Allocator = (*map).aa_buf.ator;

    buf_free(&mut (*map).aa_buf);
    free::<u8>((*map).ator, (*map).entries as *mut u8, (*map).data_size);
    (*map).entries = core::ptr::null_mut();
    (*map).items = core::ptr::null_mut();
    (*map).aa_root = core::ptr::null_mut();
    // C: `map->mask = map->capacity = map->size = 0;` — decomposed, C
    // assignment order (rightmost first).
    (*map).size = 0;
    (*map).capacity = 0;
    (*map).mask = 0;

    #[cfg(feature = "regression")]
    {
        if !regression_ator.is_null() {
            free_ator(regression_ator);
            ufbx_free(regression_ator as *mut c_void, size_of::<Allocator>());
        }
    }
}

// Recursion limit: log2(2^64 / sizeof(ufbxi_aa_node))
// ufbx.c:4443-4481 `ufbxi_aa_tree_insert`
// `ufbxi_recursive_function(..., 59, ...)` (ufbx.c:4444-4445): under
// regression, a thread-local depth guard wraps the recursive body (which C
// splits into `ufbxi_aa_tree_insert_rec`); otherwise the macro is empty and
// the wrapper is a plain call.
#[inline(never)]
pub(crate) unsafe fn aa_tree_insert(
    map: *mut Map,
    node: *mut AaNode,
    value: *const c_void,
    index: u32,
    item_size: usize,
) -> *mut AaNode {
    #[cfg(feature = "regression")]
    {
        std::thread_local! {
            static UFBXI_RECURSION_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        UFBXI_RECURSION_DEPTH.with(|depth| {
            ufbx_assert!(depth.get() < 59);
            depth.set(depth.get() + 1);
        });
        let ret = aa_tree_insert_rec(map, node, value, index, item_size);
        UFBXI_RECURSION_DEPTH.with(|depth| depth.set(depth.get() - 1));
        ret
    }
    #[cfg(not(feature = "regression"))]
    {
        aa_tree_insert_rec(map, node, value, index, item_size)
    }
}

// The recursive body (attached directly to `ufbxi_aa_tree_insert` in
// non-regression C builds; recursive calls go through the guarded wrapper).
unsafe fn aa_tree_insert_rec(
    map: *mut Map,
    node: *mut AaNode,
    value: *const c_void,
    index: u32,
    item_size: usize,
) -> *mut AaNode {
    let mut node = node;
    if node.is_null() {
        let new_node = push::<AaNode>(&mut (*map).aa_buf, 1);
        if new_node.is_null() {
            return core::ptr::null_mut();
        }
        (*new_node).left = core::ptr::null_mut();
        (*new_node).right = core::ptr::null_mut();
        (*new_node).level = 1;
        (*new_node).index = index;
        return new_node;
    }

    let entry = ((*map).items as *mut u8).add((*node).index as usize * item_size) as *mut c_void;
    // C-parity: C calls through the raw `cmp_fn` pointer without a null check.
    let cmp = ((*map).cmp_fn.unwrap_unchecked())((*map).cmp_user, value, entry);
    if cmp < 0 {
        (*node).left = aa_tree_insert(map, (*node).left, value, index, item_size);
    } else if cmp >= 0 {
        (*node).right = aa_tree_insert(map, (*node).right, value, index, item_size);
    }

    if !(*node).left.is_null() && (*(*node).left).level == (*node).level {
        let left = (*node).left;
        (*node).left = (*left).right;
        (*left).right = node;
        node = left;
    }

    if !(*node).right.is_null()
        && !(*(*node).right).right.is_null()
        && (*(*(*node).right).right).level == (*node).level
    {
        let right = (*node).right;
        (*node).right = (*right).left;
        (*right).left = node;
        (*right).level += 1;
        node = right;
    }

    node
}

// ufbx.c:4483-4498 `ufbxi_aa_tree_find`
#[inline(never)]
pub(crate) unsafe fn aa_tree_find(
    map: *mut Map,
    value: *const c_void,
    item_size: usize,
) -> *mut c_void {
    let mut node = (*map).aa_root;
    while !node.is_null() {
        let entry =
            ((*map).items as *mut u8).add((*node).index as usize * item_size) as *mut c_void;
        // C-parity: C calls through the raw `cmp_fn` pointer without a null check.
        let cmp = ((*map).cmp_fn.unwrap_unchecked())((*map).cmp_user, value, entry);
        if cmp < 0 {
            node = (*node).left;
        } else if cmp > 0 {
            node = (*node).right;
        } else {
            return entry;
        }
    }
    core::ptr::null_mut()
}

// ufbx.c:4500-4574 `ufbxi_map_grow_size_imp`
#[inline(never)]
pub(crate) unsafe fn map_grow_size_imp(map: *mut Map, item_size: usize, min_size: usize) -> bool {
    ufbx_assert!(min_size > 0);
    let load_factor = 0.7f64;

    // Find the lowest power of two size that fits `min_size` within `load_factor`
    // C: `map->mask + 1` — uint32 arithmetic, then widened to size_t.
    let mut num_entries = (*map).mask.wrapping_add(1) as usize;
    let mut new_size = (num_entries as f64 * load_factor) as usize;
    let mut min_size = min_size;
    if min_size < (*map).capacity.wrapping_add(1) as usize {
        min_size = (*map).capacity.wrapping_add(1) as usize;
    }
    while new_size < min_size {
        num_entries = num_entries.wrapping_mul(2);
        new_size = (num_entries as f64 * load_factor) as usize;
    }

    // Check for overflow
    ufbxi_check_return_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*(*map).ator).error) },
        usize::MAX / num_entries > size_of::<u64>(),
        false,
        "SIZE_MAX / num_entries > sizeof(uint64_t)"
    );
    let alloc_size = num_entries * size_of::<u64>();

    // Allocate a combined entry/item memory block
    ufbxi_check_return_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*(*map).ator).error) },
        (usize::MAX - alloc_size) / new_size > item_size,
        false,
        "(SIZE_MAX - alloc_size) / new_size > item_size"
    );
    let data_size = alloc_size + new_size * item_size;

    let data = alloc::<u8>((*map).ator, data_size);
    ufbxi_check_return_err!(
        unsafe { crate::native::error::ErrorView::from_ptr((*(*map).ator).error) },
        !data.is_null(),
        false,
        "data"
    );

    // Copy the previous user items over
    let old_entries = (*map).entries;
    let new_entries = data as *mut u64;
    let new_items = data.add(alloc_size) as *mut c_void;
    if (*map).size > 0 {
        core::ptr::copy_nonoverlapping(
            (*map).items as *const u8,
            new_items as *mut u8,
            item_size * (*map).size as usize,
        );
    }

    // Re-hash the entries
    let old_mask = (*map).mask;
    let new_mask = (num_entries as u32).wrapping_sub(1);
    core::ptr::write_bytes(new_entries, 0, num_entries);
    if old_mask != 0 {
        for i in 0..=old_mask {
            let mut entry: u64;
            let mut new_entry = *old_entries.add(i as usize);
            if new_entry == 0 {
                continue;
            }

            // Reconstruct the hash of the old entry at `i`
            let old_scan = (new_entry as u32 & old_mask).wrapping_sub(1);
            let hash = ((new_entry as u32) & !old_mask) | (i.wrapping_sub(old_scan) & old_mask);
            let mut slot = hash & new_mask;
            new_entry &= !(new_mask as u64);

            // Scan forward until we find an empty slot, potentially swapping
            // `new_element` if it has a shorter scan distance (Robin Hood).
            let mut scan: u32 = 1;
            loop {
                entry = *new_entries.add(slot as usize);
                if entry == 0 {
                    break;
                }
                let entry_scan = (entry & new_mask as u64) as u32;
                if entry_scan < scan {
                    *new_entries.add(slot as usize) = new_entry.wrapping_add(scan as u64);
                    new_entry = entry & !(new_mask as u64);
                    scan = entry_scan;
                }
                scan = scan.wrapping_add(1);
                slot = slot.wrapping_add(1) & new_mask;
            }
            *new_entries.add(slot as usize) = new_entry.wrapping_add(scan as u64);
        }
    }

    // And finally free the previous allocation
    free::<u8>((*map).ator, old_entries as *mut u8, (*map).data_size);
    (*map).items = new_items;
    (*map).data_size = data_size;
    (*map).entries = new_entries;
    (*map).mask = new_mask;
    (*map).capacity = new_size as u32;

    true
}

// ufbx.c:4576-4588 `ufbxi_map_grow_size`
#[inline(always)]
pub(crate) unsafe fn map_grow_size(map: *mut Map, size: usize, min_size: usize) -> bool {
    #[cfg(feature = "regression")]
    {
        let ator = (*map).ator;
        ufbxi_check_return_err_msg!(
            unsafe { crate::native::error::ErrorView::from_ptr((*ator).error) },
            (*ator).num_allocs < (*ator).max_allocs,
            false,
            "Allocation limit exceeded",
            "ator->num_allocs < ator->max_allocs"
        );
        (*ator).num_allocs += 1;
    }

    if (*map).size < (*map).capacity && (*map).capacity as usize >= min_size {
        return true;
    }
    map_grow_size_imp(map, size, min_size)
}

// ufbx.c:4590-4617 `ufbxi_map_find_size`
#[inline(never)]
pub(crate) unsafe fn map_find_size(
    map: *mut Map,
    size: usize,
    hash: u32,
    value: *const c_void,
) -> *mut c_void {
    let entries = (*map).entries;
    let mask = (*map).mask;
    let mut scan: u32 = 0;

    let ref_ = hash & !mask;
    if mask == 0 || scan == u32::MAX {
        return core::ptr::null_mut();
    }

    // Scan entries until we find an exact match of the hash or until we hit
    // an element that has lower scan distance than our search (Robin Hood).
    // The encoding guarantees that zero slots also terminate with the same test.
    loop {
        let entry = *entries.add((hash.wrapping_add(scan) & mask) as usize);
        scan = scan.wrapping_add(1);
        if entry as u32 == ref_.wrapping_add(scan) {
            let index = (entry >> 32u32) as u32;
            let data = ((*map).items as *mut u8).add(size * index as usize) as *mut c_void;
            // C-parity: C calls through the raw `cmp_fn` pointer without a null check.
            let cmp = ((*map).cmp_fn.unwrap_unchecked())((*map).cmp_user, value, data);
            if cmp == 0 {
                return data;
            }
        } else if (entry & mask as u64) < scan as u64 {
            if !(*map).aa_root.is_null() {
                return aa_tree_find(map, value, size);
            } else {
                return core::ptr::null_mut();
            }
        }
    }
}

// ufbx.c:4619-4655 `ufbxi_map_insert_size`
#[inline(never)]
pub(crate) unsafe fn map_insert_size(
    map: *mut Map,
    size: usize,
    hash: u32,
    value: *const c_void,
) -> *mut c_void {
    if !map_grow_size(map, size, 64) {
        return core::ptr::null_mut();
    }

    ufbxi_regression_assert!(map_find_size(map, size, hash, value).is_null());

    // C: `uint32_t index = map->size++;`
    let index = (*map).size;
    (*map).size = (*map).size.wrapping_add(1);

    let entries = (*map).entries;
    let mask = (*map).mask;

    // Scan forward until we find an empty slot, potentially swapping
    // `new_element` if it has a shorter scan distance (Robin Hood).
    let mut slot = hash & mask;
    let mut entry: u64;
    let mut new_entry = (index as u64) << 32u32 | (hash & !mask) as u64;
    let mut scan: u32 = 1;
    loop {
        entry = *entries.add(slot as usize);
        if entry == 0 {
            break;
        }
        let entry_scan = (entry & mask as u64) as u32;
        if entry_scan < scan {
            *entries.add(slot as usize) = new_entry.wrapping_add(scan as u64);
            new_entry = entry & !(mask as u64);
            scan = entry_scan;
        }
        scan = scan.wrapping_add(1);
        slot = slot.wrapping_add(1) & mask;

        if scan > MAP_MAX_SCAN {
            let new_index = (new_entry >> 32u32) as u32;
            let new_value = if new_index == index {
                value
            } else {
                ((*map).items as *const u8).add(size * new_index as usize) as *const c_void
            };
            (*map).aa_root = aa_tree_insert(map, (*map).aa_root, new_value, new_index, size);
            return ((*map).items as *mut u8).add(size * index as usize) as *mut c_void;
        }
    }
    *entries.add(slot as usize) = new_entry.wrapping_add(scan as u64);

    ((*map).items as *mut u8).add(size * index as usize) as *mut c_void
}

// -- Typed wrappers (ufbx.c:4657-4659)

// ufbx.c:4657 `ufbxi_map_grow(map, type, min_size)`
#[inline(always)]
pub(crate) unsafe fn map_grow<T>(map: *mut Map, min_size: usize) -> bool {
    map_grow_size(map, size_of::<T>(), min_size)
}

// ufbx.c:4658 `ufbxi_map_find(map, type, hash, value)`
// C-parity: the C macro passes `value` untyped into `const void*`; its pointee
// type is usually unrelated to `type` (key pointer, e.g. `&fbx_id` as uint64_t*
// with ufbxi_fbx_id_entry at ufbx.c:12310), so `value` is *const c_void here.
#[inline(always)]
pub(crate) unsafe fn map_find<T>(map: *mut Map, hash: u32, value: *const c_void) -> *mut T {
    ufbxi_maybe_null!(map_find_size(map, size_of::<T>(), hash, value) as *mut T)
}

// ufbx.c:4659 `ufbxi_map_insert(map, type, hash, value)`
// C-parity: `value` is untyped in the C macro (see map_find above).
#[inline(always)]
pub(crate) unsafe fn map_insert<T>(map: *mut Map, hash: u32, value: *const c_void) -> *mut T {
    ufbxi_maybe_null!(map_insert_size(map, size_of::<T>(), hash, value) as *mut T)
}

// ufbx.c:4661-4668 `ufbxi_map_cmp_uint64`
pub(crate) unsafe extern "C" fn map_cmp_uint64(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> i32 {
    let _ = user; // (void)user
    let a = *(va as *const u64);
    let b = *(vb as *const u64);
    if a < b {
        return -1;
    }
    if a > b {
        return 1;
    }
    0
}

// ufbx.c:4670-4677 `ufbxi_map_cmp_const_char_ptr`
pub(crate) unsafe extern "C" fn map_cmp_const_char_ptr(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> i32 {
    let _ = user; // (void)user
    let a = *(va as *const *const u8);
    let b = *(vb as *const *const u8);
    if a < b {
        return -1;
    }
    if a > b {
        return 1;
    }
    0
}

// ufbx.c:4679-4686 `ufbxi_map_cmp_uintptr`
pub(crate) unsafe extern "C" fn map_cmp_uintptr(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> i32 {
    let _ = user; // (void)user
    let a = *(va as *const usize);
    let b = *(vb as *const usize);
    if a < b {
        return -1;
    }
    if a > b {
        return 1;
    }
    0
}

// (`ufbxi_ptr_id`, ufbx.c:4688-4691, is defined at the top of this file next
// to `ufbxi_hash_ptr_id` — see the note there.)

// ufbx.c:4693-4700 `ufbxi_map_cmp_ptr_id`
pub(crate) unsafe extern "C" fn map_cmp_ptr_id(
    user: *mut c_void,
    va: *const c_void,
    vb: *const c_void,
) -> i32 {
    let _ = user; // (void)user
    let a = *(va as *const PtrId);
    let b = *(vb as *const PtrId);
    if a.id != b.id {
        return if a.id < b.id { -1 } else { 1 };
    }
    if a.ptr != b.ptr {
        return if a.ptr < b.ptr { -1 } else { 1 };
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hs(b: &[u8]) -> u32 {
        // SAFETY: pointer and length come from the same local slice `b`.
        unsafe { hash_string(b.as_ptr(), b.len()) }
    }

    fn hsca(b: &[u8]) -> (u32, bool) {
        let mut non_ascii = false;
        // SAFETY: pointer and length come from the same local slice `b`;
        // `p_non_ascii` points at a live local.
        let h = unsafe { hash_string_check_ascii(b.as_ptr(), b.len(), &mut non_ascii) };
        (h, non_ascii)
    }

    // Reference values from the C algorithm (little-endian reads).
    #[test]
    fn hash_string_known_values() {
        assert_eq!(hs(b"a"), 0x88b1a51d);
        assert_eq!(hs(b"ab"), 0x198f65f0);
        assert_eq!(hs(b"abc"), 0x021f1b84);
        assert_eq!(hs(b"abcd"), 0xec8b77cd);
        assert_eq!(hs(b"abcde"), 0x2fe5a905);
        assert_eq!(hs(b"Geometry"), 0xf63e5026);
        assert_eq!(hs(b"ObjectType"), 0x11aa0eb0);
        assert_eq!(hs(b"\xffx"), 0x732948c2);
        assert_eq!(hs(b"a\x00b"), 0x257d0e0a);
    }

    // C comment: "NOTE: _Must_ match `ufbxi_hash_string()`" — verify
    // bit-for-bit equality across all length classes (1-3, exact multiple of
    // 4, and the overlapping-tail path).
    #[test]
    fn check_ascii_matches_hash_string() {
        let data: &[u8] = b"The quick brown fox jumps over the lazy dog \xff\x00\x80\x01";
        for start in 0..8 {
            for len in 1..=(data.len() - start) {
                let slice = &data[start..start + len];
                let (h, _) = hsca(slice);
                assert_eq!(h, hs(slice), "mismatch for {:?}", slice);
            }
        }
    }

    #[test]
    fn check_ascii_flag() {
        // Pure ASCII (0x01..=0x7f) must NOT set the flag.
        assert!(!hsca(b"a").1);
        assert!(!hsca(b"ab").1);
        assert!(!hsca(b"abc").1);
        assert!(!hsca(b"abcd").1);
        assert!(!hsca(b"abcdefg").1);
        assert!(!hsca(b"\x01\x7f").1);
        // High bit set → non-ASCII, in every byte position of both paths.
        assert!(hsca(b"\x80").1);
        assert!(hsca(b"a\xff").1);
        assert!(hsca(b"ab\x80").1);
        assert!(hsca(b"abc\x80").1);
        assert!(hsca(b"abcd\x80").1);
        assert!(hsca(b"abcdefg\x80").1);
        // Embedded zero → non-ASCII.
        assert!(hsca(b"a\x00").1);
        assert!(hsca(b"a\x00b").1);
        assert!(hsca(b"abc\x00").1);
        assert!(hsca(b"abcdef\x00h").1);
    }

    #[test]
    fn check_ascii_only_sets_flag() {
        // The C only ever writes `true`; a pre-set flag must survive an
        // all-ASCII string.
        unsafe {
            let mut non_ascii = true;
            hash_string_check_ascii(b"abc".as_ptr(), 3, &mut non_ascii);
            assert!(non_ascii);
        }
    }

    #[test]
    fn hash32_known_values() {
        assert_eq!(hash32(0), 0);
        assert_eq!(hash32(1), 0x688990c0);
        assert_eq!(hash32(0xdeadbeef), 0xe628c683);
    }

    #[test]
    fn hash64_known_values() {
        assert_eq!(hash64(0), 0);
        assert_eq!(hash64(1), 0xe0c0e0d0);
        assert_eq!(hash64(0x123456789abcdef0), 0x079081a9);
    }

    #[test]
    fn hash_uptr_and_ptr_id() {
        #[cfg(target_pointer_width = "64")]
        assert_eq!(hash_uptr(1), hash64(1));
        #[cfg(target_pointer_width = "32")]
        assert_eq!(hash_uptr(1), hash32(1));

        // "Only `ptr` or `id` is defined" — trivial xor reduction.
        let a = PtrId { ptr: 0x1234, id: 0 };
        assert_eq!(hash_ptr_id(a), hash_uptr(0x1234) ^ hash64(0));
        let b = PtrId { ptr: 0, id: 77 };
        assert_eq!(hash_ptr_id(b), hash_uptr(0) ^ hash64(77));

        let x = 5u32;
        let p = &x as *const u32;
        assert_eq!(hash_ptr!(p), hash_uptr(p as usize));
    }

    // -- Hash map tests

    use crate::generated::Error;
    use crate::native::allocator::init_ator;
    use core::mem::MaybeUninit;

    unsafe fn make_map(ator: *mut Allocator, cmp_fn: CmpFn) -> Map {
        // C maps live inside zero-initialized contexts; `ufbxi_map_init` only
        // sets the fields it names.
        let mut map = MaybeUninit::<Map>::zeroed().assume_init();
        map_init(&mut map, ator, cmp_fn, core::ptr::null_mut());
        map
    }

    unsafe fn make_test_ator(err: *mut Error) -> Allocator {
        let mut ator = MaybeUninit::<Allocator>::zeroed();
        init_ator(
            err,
            ator.as_mut_ptr(),
            core::ptr::null(),
            b"test\0".as_ptr(),
        );
        ator.assume_init()
    }

    #[test]
    fn map_insert_find_u64() {
        unsafe {
            let mut err = Error::default();
            let mut ator = make_test_ator(&mut err);
            let mut map = make_map(&mut ator, map_cmp_uint64);

            for i in 0..1000u64 {
                let v = i * 7919;
                let h = hash64(v);
                assert!(map_find::<u64>(&mut map, h, &v as *const u64 as *const c_void).is_null());
                let p = map_insert::<u64>(&mut map, h, &v as *const u64 as *const c_void);
                assert!(!p.is_null());
                *p = v;
            }
            assert_eq!(map.size, 1000);
            for i in 0..1000u64 {
                let v = i * 7919;
                let p = map_find::<u64>(&mut map, hash64(v), &v as *const u64 as *const c_void);
                assert!(!p.is_null(), "missing {}", v);
                assert_eq!(*p, v);
            }
            // Missing keys stay missing.
            let v = 3u64;
            assert!(
                map_find::<u64>(&mut map, hash64(v), &v as *const u64 as *const c_void).is_null()
            );

            map_free(&mut map);
            assert_eq!(ator.current_size, 0);
            assert!(map.entries.is_null());
            assert_eq!(map.size, 0);
        }
    }

    // First grow with `min_size = 64` (the `ufbxi_map_insert_size` constant):
    // num_entries doubles from 1 until 0.7 * n >= 64 → n = 128, capacity 89.
    #[test]
    fn map_first_grow_geometry() {
        unsafe {
            let mut err = Error::default();
            let mut ator = make_test_ator(&mut err);
            let mut map = make_map(&mut ator, map_cmp_uint64);

            let v = 1u64;
            let p = map_insert::<u64>(&mut map, hash64(v), &v as *const u64 as *const c_void);
            assert!(!p.is_null());
            *p = v;
            assert_eq!(map.mask, 127);
            assert_eq!(map.capacity, 89);
            assert_eq!(map.size, 1);

            map_free(&mut map);
            assert_eq!(ator.current_size, 0);
        }
    }

    // Degenerate hashing (all keys hash to 0) drives scan > UFBXI_MAP_MAX_SCAN
    // and spills into the AA tree; find must still resolve every key.
    #[test]
    fn map_collision_aa_tree() {
        unsafe {
            let mut err = Error::default();
            let mut ator = make_test_ator(&mut err);
            let mut map = make_map(&mut ator, map_cmp_uint64);

            let n = (MAP_MAX_SCAN + 20) as u64;
            for i in 0..n {
                let p = map_insert::<u64>(&mut map, 0, &i as *const u64 as *const c_void);
                assert!(!p.is_null());
                *p = i;
            }
            assert!(!map.aa_root.is_null());
            for i in 0..n {
                let p = map_find::<u64>(&mut map, 0, &i as *const u64 as *const c_void);
                assert!(!p.is_null(), "missing {}", i);
                assert_eq!(*p, i);
            }
            let missing = n + 1;
            assert!(
                map_find::<u64>(&mut map, 0, &missing as *const u64 as *const c_void).is_null()
            );

            map_free(&mut map);
            assert_eq!(ator.current_size, 0);
        }
    }

    // Find on a never-grown (zeroed) map bails on `mask == 0`.
    #[test]
    fn map_find_empty() {
        unsafe {
            let mut err = Error::default();
            let mut ator = make_test_ator(&mut err);
            let mut map = make_map(&mut ator, map_cmp_uint64);
            let v = 42u64;
            assert!(
                map_find::<u64>(&mut map, hash64(v), &v as *const u64 as *const c_void).is_null()
            );
            map_free(&mut map);
        }
    }

    #[test]
    fn map_grow_reserves_capacity() {
        unsafe {
            let mut err = Error::default();
            let mut ator = make_test_ator(&mut err);
            let mut map = make_map(&mut ator, map_cmp_uint64);
            assert!(map_grow::<u64>(&mut map, 1000));
            assert!(map.capacity as usize >= 1000);
            let allocs_after_grow = ator.num_allocs;
            // Inserting within capacity performs no further table allocations.
            for i in 0..1000u64 {
                let p = map_insert::<u64>(&mut map, hash64(i), &i as *const u64 as *const c_void);
                assert!(!p.is_null());
                *p = i;
            }
            #[cfg(not(feature = "regression"))]
            assert_eq!(ator.num_allocs, allocs_after_grow);
            // Regression counts every `ufbxi_map_grow_size` call as an
            // allocation (ufbx.c:4578-4584).
            #[cfg(feature = "regression")]
            assert_eq!(ator.num_allocs, allocs_after_grow + 1000);
            map_free(&mut map);
            assert_eq!(ator.current_size, 0);
        }
    }

    #[test]
    fn map_cmp_fns() {
        unsafe {
            let n = core::ptr::null_mut();
            let a = 1u64;
            let b = 2u64;
            let ap = &a as *const u64 as *const c_void;
            let bp = &b as *const u64 as *const c_void;
            assert_eq!(map_cmp_uint64(n, ap, bp), -1);
            assert_eq!(map_cmp_uint64(n, bp, ap), 1);
            assert_eq!(map_cmp_uint64(n, ap, ap), 0);

            let arr = [10u8, 20u8];
            let p0: *const u8 = &arr[0];
            let p1: *const u8 = &arr[1];
            let pp0 = &p0 as *const *const u8 as *const c_void;
            let pp1 = &p1 as *const *const u8 as *const c_void;
            assert_eq!(map_cmp_const_char_ptr(n, pp0, pp1), -1);
            assert_eq!(map_cmp_const_char_ptr(n, pp1, pp0), 1);
            assert_eq!(map_cmp_const_char_ptr(n, pp0, pp0), 0);

            let ua = 5usize;
            let ub = 9usize;
            let uap = &ua as *const usize as *const c_void;
            let ubp = &ub as *const usize as *const c_void;
            assert_eq!(map_cmp_uintptr(n, uap, ubp), -1);
            assert_eq!(map_cmp_uintptr(n, ubp, uap), 1);
            assert_eq!(map_cmp_uintptr(n, uap, uap), 0);

            // id is the primary key, ptr the secondary.
            let x = PtrId { ptr: 2, id: 1 };
            let y = PtrId { ptr: 1, id: 2 };
            let z = PtrId { ptr: 9, id: 1 };
            let xp = &x as *const PtrId as *const c_void;
            let yp = &y as *const PtrId as *const c_void;
            let zp = &z as *const PtrId as *const c_void;
            assert_eq!(map_cmp_ptr_id(n, xp, yp), -1);
            assert_eq!(map_cmp_ptr_id(n, yp, xp), 1);
            assert_eq!(map_cmp_ptr_id(n, xp, zp), -1);
            assert_eq!(map_cmp_ptr_id(n, zp, xp), 1);
            assert_eq!(map_cmp_ptr_id(n, xp, xp), 0);
        }
    }
}
