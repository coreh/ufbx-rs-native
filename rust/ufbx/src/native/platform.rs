//! Port of the `// -- Platform` banner section and its satellite one-liners:
//! ufbx.c:399-630 (Platform), 631-716 (`// -- Atomic counter`), 743-853 (byte
//! readers + sizeof asserts, pulled in
//! because they are pure platform shims), 856-1081 (Alignment / Version / Fast
//! copy / Large fast integer / Wrapping right shift / Bit manipulation / Bit
//! conversion / Pointer alignment / Debug), plus `ufbxi_swap` (ufbx.c:1293-1316)
//! whose scratch union is a platform alignment device, the `// -- Utility`
//! banner section (ufbx.c:1082-1348: pointer helpers, min/max, float→int
//! clamps, `ufbxi_to_size`, both stable sorts, the bound searches, and the
//! unstable heapsort), and the math shim surface (ufbx.c:257-276 / 337-368)
//! as the `math` submodule.
//!
//! Collapsed-away C apparatus (no Rust analogue, per PORTING.md):
//! - ufbx.c:401-413 `UFBXI_MSC_VER` / `UFBXI_GNUC` / `UFBXI_GNUC_VERSION`:
//!   compiler-identification macros, meaningless under rustc.
//! - ufbx.c:415-440 `ufbxi_noinline` / `ufbxi_forceinline` / `ufbxi_restrict` /
//!   `ufbxi_nodiscard` / `ufbxi_unused`: map at use sites to
//!   `#[inline(never)]` / `#[inline(always)]` / (nothing — Rust `&mut` is
//!   already noalias) / `#[must_use]` / (nothing).
//!   `ufbxi_likely`/`ufbxi_unlikely` are the `likely`/`unlikely` fns below
//!   (PORTING.md "Branch hints": core::hint under the `nightly` feature,
//!   identity otherwise — same degradation as C without `__builtin_expect`).
//! - ufbx.c:442-450 `ufbxi_nounroll`: optimizer pragma, no analogue.
//! - ufbx.c:458-549 MSVC/clang/GCC warning-pragma blocks: no analogue.
//! - ufbx.c:551-557 `ufbx_static_assert`: `const _: () = assert!(...)`.
//! - ufbx.c:559-576 UBSan / static-analyzer detection (`UFBX_UBSAN`,
//!   `UFBX_STATIC_ANALYSIS`): not supported in the Rust build; the identity
//!   forms of the dependent macros are ported below.
//! - ufbx.c:593-599 `ufbxi_trace`: `UFBX_TRACE` is not supported; the
//!   evaluate-once contract it carries is owned by the check macros in the
//!   error module (see PORTING.md "Error threading").
//! - ufbx.c:609-615 `UFBX_HAS_FTELLO`: file IO capability probe, owned by the
//!   io module when ported.
//! - ufbx.c:617-630 `UFBXI_ARCH_X64` / `UFBXI_HAS_SSE`: only consumers are
//!   `ufbxi_copy_16_bytes` (ported as the SSE load-then-store semantics — see
//!   its overlap note) and `ufbxi_wrap_shr64` (ported as the portable masked
//!   form; bit-identical).
//! - ufbx.c:635-639 `ufbxi_extern_c`: C/C++ linkage shim, meaningless under
//!   rustc.
//! - ufbx.c:648-716 non-GCC atomic-counter branches (MSVC interlocked, TCC
//!   inline asm, C++11 `std::atomic`, C11 `stdatomic`, non-atomic fallback):
//!   all atomic branches implement one contract, ported once on `AtomicUsize`
//!   (see the Atomic counter section below).
//! - ufbx.c:718-719 include-ordering guard comments ("No references to
//!   <string.h> before this point" / "No more includes past this point"):
//!   build-hygiene markers with no Rust analogue.
//! - ufbx.c:721-731 `UFBX_STRING_PREFIX` / `ufbxi_string_fn`: libc
//!   string-function renaming for freestanding builds; Rust uses `core::ptr` /
//!   slice intrinsics, nothing to rename.
//! - ufbx.c:733-739 `UFBX_LITTLE_ENDIAN`: consumed only by the unaligned-read
//!   fork (791) and `ufbxi_swap_endian*`; collapses into `from_le_bytes` /
//!   `#[cfg(target_endian)]` at use sites (PORTING.md "Byte order").
//! - ufbx.c:759-790 `ufbxi_unaligned_*` / `ufbxi_aliasing_u32` typedef
//!   apparatus: collapses into `ptr::read_unaligned` (PORTING.md "Byte order").
//! - ufbx.c:1042-1051 `ufbxi_thread_local`: consumed only by the recursion
//!   guard below.
//! - ufbx.c:1053-1081 `ufbxi_recursive_function(_void)`: regression-only
//!   recursion-depth guard. Ported inline at each recursive function site
//!   (`std::thread_local!` depth counter under `feature = "regression"`) when
//!   those functions are ported — there is nothing standalone to define here.
//!
//! Phase 1: most items have no consumers yet.
#![allow(dead_code, unused_macros, unused_imports)]

use core::ffi::c_void;
use core::mem::size_of;

// -- Asserts (see PORTING.md "Asserts": three distinct gates, do NOT collapse)

// ufbx.h:102-109 `ufbx_assert`.
// C gate: off under `UFBX_NO_ASSERT` or `UFBX_NO_LIBC`. Neither is exposed as a
// cargo feature yet (C default: asserts ON) — when a `no-assert` feature is
// added, gate the `assert!` arm on `#[cfg(not(feature = "no-assert"))]`.
macro_rules! ufbx_assert {
    ($($tt:tt)*) => { assert!($($tt)*) };
}
pub(crate) use ufbx_assert;

// ufbx.c:1022-1026
#[cfg(feature = "regression")]
macro_rules! ufbxi_regression_assert {
    ($($tt:tt)*) => { $crate::native::platform::ufbx_assert!($($tt)*) };
}
#[cfg(not(feature = "regression"))]
macro_rules! ufbxi_regression_assert {
    // C: `(void)0` — the condition tokens are dropped unexpanded, never evaluated.
    ($($tt:tt)*) => {
        ()
    };
}
pub(crate) use ufbxi_regression_assert;

// ufbx.c:1028-1032
// C gate also includes `UFBX_UBSAN`, which has no Rust-build analogue.
#[cfg(any(feature = "regression", feature = "dev"))]
macro_rules! ufbxi_dev_assert {
    ($($tt:tt)*) => { $crate::native::platform::ufbx_assert!($($tt)*) };
}
#[cfg(not(any(feature = "regression", feature = "dev")))]
macro_rules! ufbxi_dev_assert {
    // C: `(void)0` — the condition tokens are dropped unexpanded, never evaluated.
    ($($tt:tt)*) => {
        ()
    };
}
pub(crate) use ufbxi_dev_assert;

// ufbx.c:1034 `#define ufbxi_unreachable(reason) do { ufbx_assert(0 && reason); } while (0)`
// NOT std `unreachable!()` / `unreachable_unchecked` — C keeps executing past
// this when asserts are compiled out (PORTING.md "Asserts").
// The reason is a plain string operand in C (`0 && reason`), never a format
// string — pass it through `"{}"` so `{`/`}` in the literal stay literal.
macro_rules! ufbxi_unreachable {
    ($reason:expr) => {
        $crate::native::platform::ufbx_assert!(false, "{}", $reason)
    };
}
pub(crate) use ufbxi_unreachable;

// -- Static-analysis shims (identity forms; UFBX_STATIC_ANALYSIS unsupported)

// ufbx.c:578-585 `ufbxi_maybe_null` (identity branch, ufbx.c:583)
macro_rules! ufbxi_maybe_null {
    ($ptr:expr) => {
        $ptr
    };
}
pub(crate) use ufbxi_maybe_null;

// ufbx.c:578-585 `ufbxi_analysis_assert` (identity branch, ufbx.c:584: `(void)0`)
macro_rules! ufbxi_analysis_assert {
    // Condition tokens dropped unexpanded, never evaluated (matches `(void)0`).
    ($($tt:tt)*) => {
        ()
    };
}
pub(crate) use ufbxi_analysis_assert;

// ufbx.c:587-591 `ufbxi_maybe_uninit` (non-analysis branch, ufbx.c:590:
// expands to `(value)` — cond and def are dropped unexpanded, never evaluated)
macro_rules! ufbxi_maybe_uninit {
    ($cond:expr, $value:expr, $def:expr) => {
        $value
    };
}
pub(crate) use ufbxi_maybe_uninit;

// ufbx.c:452-456 `ufbxi_ignore(cond)` — evaluate and discard.
macro_rules! ufbxi_ignore {
    ($cond:expr) => {{
        let _ = $cond;
    }};
}
pub(crate) use ufbxi_ignore;

// -- Path separator

// ufbx.c:601-607 `UFBX_PATH_SEPARATOR`
#[cfg(windows)]
pub(crate) const PATH_SEPARATOR: u8 = b'\\';
#[cfg(not(windows))]
pub(crate) const PATH_SEPARATOR: u8 = b'/';

// -- Atomic counter

// ufbx.c:633 `#define UFBXI_THREAD_SAFE 1`
// Redefined to 0 only in the final non-atomic fallback branch (ufbx.c:714-715),
// which the Rust port never takes — `core::sync::atomic` is always available on
// supported targets. Consumed by `ufbx_is_thread_safe` (ufbx.c:30497-30500),
// which returns `UFBXI_THREAD_SAFE != 0`.
pub(crate) const THREAD_SAFE: u32 = 1;

// ufbx.c:641-716 `ufbxi_atomic_counter` + operation macros.
// C forks six ways (GCC/clang `__sync` builtins 641-647, MSVC interlocked
// 648-669, TCC inline asm 670-689, C++11 `std::atomic` 690-698, C11
// `stdatomic` 699-706, non-atomic fallback 707-716). Every atomic branch
// implements the same contract — sequentially-consistent RMW where inc/dec
// return the PREVIOUS value (the MSVC branch compensates -1/+1 because
// `_InterlockedIncrement/Decrement` return the NEW value, ufbx.c:656-657).
// Ported once as the GCC/clang `__sync_fetch_and_*` semantics on `AtomicUsize`
// (PORTING.md "Atomics / refcount"): all ops `Ordering::SeqCst` — do NOT
// "optimize" to `Relaxed`. Use sites depend on the previous-value convention:
// `ufbxi_release_ref` (ufbx.c:30273) does `if dec(...) > 0 { return }`, and the
// counter starts at 0 (`ufbxi_init_ref`, ufbx.c:30255, no self-retain) — so the
// object is freed when the PREVIOUS value was 0.
// ufbx.c:642 `typedef size_t ufbxi_atomic_counter;`
pub(crate) type AtomicCounter = core::sync::atomic::AtomicUsize;

// ufbx.c:429-440 `ufbxi_likely`/`ufbxi_unlikely` — `__builtin_expect` when the
// compiler supports it, identity otherwise. Same degradation here: core::hint
// under the `nightly` feature, identity on stable (PORTING.md "Branch hints").
// Keep call sites where C has them; optimizer-only, cannot affect oracle output.
#[cfg(feature = "nightly")]
#[inline(always)]
pub(crate) fn likely(b: bool) -> bool {
    core::hint::likely(b)
}
#[cfg(feature = "nightly")]
#[inline(always)]
pub(crate) fn unlikely(b: bool) -> bool {
    core::hint::unlikely(b)
}
#[cfg(not(feature = "nightly"))]
#[inline(always)]
pub(crate) fn likely(b: bool) -> bool {
    b
}
#[cfg(not(feature = "nightly"))]
#[inline(always)]
pub(crate) fn unlikely(b: bool) -> bool {
    b
}

// Layout parity with the C `size_t` counter (refcount header layout depends on
// it). Size equality is guaranteed for `AtomicUsize`; the alignment check is the
// one that can actually differ from the plain integer on some targets.
const _: () = assert!(size_of::<AtomicCounter>() == size_of::<usize>());
const _: () = assert!(align_of::<AtomicCounter>() == align_of::<usize>());

// ufbx.c:643 `#define ufbxi_atomic_counter_init(ptr) (*(ptr) = 0)`
// C-parity: a plain (non-atomic) store — init runs before the counter is shared.
#[inline(always)]
pub(crate) unsafe fn atomic_counter_init(ptr: *mut AtomicCounter) {
    ptr.write(AtomicCounter::new(0));
}

// ufbx.c:644 `#define ufbxi_atomic_counter_free(ptr) (*(ptr) = 0)`
// C-parity: plain store — free runs after the last owner is done sharing it.
#[inline(always)]
pub(crate) unsafe fn atomic_counter_free(ptr: *mut AtomicCounter) {
    ptr.write(AtomicCounter::new(0));
}

// ufbx.c:645 `#define ufbxi_atomic_counter_inc(ptr) __sync_fetch_and_add((ptr), 1)`
// Returns the PREVIOUS value.
#[inline(always)]
pub(crate) unsafe fn atomic_counter_inc(ptr: *mut AtomicCounter) -> usize {
    (*ptr).fetch_add(1, core::sync::atomic::Ordering::SeqCst)
}

// ufbx.c:646 `#define ufbxi_atomic_counter_dec(ptr) __sync_fetch_and_sub((ptr), 1)`
// Returns the PREVIOUS value.
#[inline(always)]
pub(crate) unsafe fn atomic_counter_dec(ptr: *mut AtomicCounter) -> usize {
    (*ptr).fetch_sub(1, core::sync::atomic::Ordering::SeqCst)
}

// ufbx.c:647 `#define ufbxi_atomic_counter_load(ptr) __sync_fetch_and_add((ptr), 0)`
// C: "// TODO: Proper atomic load" — ported as `fetch_add(0, SeqCst)` to match
// (PORTING.md "Atomics / refcount"), not a plain `load`.
#[inline(always)]
pub(crate) unsafe fn atomic_counter_load(ptr: *mut AtomicCounter) -> usize {
    (*ptr).fetch_add(0, core::sync::atomic::Ordering::SeqCst)
}

// -- Configuration constants with regression overrides
// Base values ufbx.c:54-61; regression overrides ufbx.c:999-1020 (Debug banner).
// This module owns them because the override block lives in the Debug section.

// ufbx.c:54 / override ufbx.c:1000-1002
#[cfg(not(feature = "regression"))]
pub(crate) const MAX_SKIP_SIZE: usize = 0x40000000;
#[cfg(feature = "regression")]
pub(crate) const MAX_SKIP_SIZE: usize = 128;

// ufbx.c:55 / override ufbx.c:1004-1005
#[cfg(not(feature = "regression"))]
pub(crate) const MAP_MAX_SCAN: usize = 32;
#[cfg(feature = "regression")]
pub(crate) const MAP_MAX_SCAN: usize = 2;

// ufbx.c:56 / override ufbx.c:1007-1008
#[cfg(not(feature = "regression"))]
pub(crate) const KD_FAST_DEPTH: usize = 6;
#[cfg(feature = "regression")]
pub(crate) const KD_FAST_DEPTH: usize = 2;

// ufbx.c:59 / override ufbx.c:1010-1011
#[cfg(not(feature = "regression"))]
pub(crate) const FACE_GROUP_HASH_BITS: u32 = 8;
#[cfg(feature = "regression")]
pub(crate) const FACE_GROUP_HASH_BITS: u32 = 2;

// ufbx.c:60 / override ufbx.c:1014-1016
// C also overrides under `UFBX_EXTENSIVE_THREADING`, which is not exposed as a
// cargo feature (add it to the cfg below if it ever is).
#[cfg(not(feature = "regression"))]
pub(crate) const MIN_THREADED_DEFLATE_BYTES: usize = 256;
#[cfg(feature = "regression")]
pub(crate) const MIN_THREADED_DEFLATE_BYTES: usize = 2;

// ufbx.c:61 / override ufbx.c:1018-1019
#[cfg(not(feature = "regression"))]
pub(crate) const MIN_THREADED_ASCII_VALUES: usize = 64;
#[cfg(feature = "regression")]
pub(crate) const MIN_THREADED_ASCII_VALUES: usize = 2;

// ufbx.c:995-997 `ufbxi_clamp_linear_threshold`
// C also forces 2 under `UFBX_DEBUG_BINARY_SEARCH` (no cargo feature).
#[inline(always)]
pub(crate) const fn clamp_linear_threshold(v: usize) -> usize {
    if cfg!(feature = "regression") {
        2
    } else {
        v
    }
}

// ufbx.c:1036-1040 `UFBXI_IS_REGRESSION` (runtime-visible, e.g. ufbx.c:7330)
#[cfg(feature = "regression")]
pub(crate) const IS_REGRESSION: bool = true;
#[cfg(not(feature = "regression"))]
pub(crate) const IS_REGRESSION: bool = false;

// -- Alignment

// ufbx.c:858-860 `UFBX_MAXIMUM_ALIGNMENT` (user override unsupported)
pub(crate) const MAXIMUM_ALIGNMENT: usize = if size_of::<*const u8>() > 8 {
    size_of::<*const u8>()
} else {
    8
};

// ufbx.c:862-873 `UFBXI_UINTPTR_SIZE`
// C: "CHERI lies about UINTPTR_MAX" — C derives this from `UINTPTR_MAX`
// (skipped on `__CHERI__`) and static-asserts it equals `sizeof(uintptr_t)`
// (the 0 "unknown" fallback has no Rust analogue).
pub(crate) const UINTPTR_SIZE: usize = size_of::<usize>();

// -- Version

// ufbx.h:260 `ufbx_pack_version(major, minor, patch)`
// C computes in unsigned (`1000000u`), which wraps — keep wrapping semantics.
#[inline(always)]
pub(crate) const fn pack_version(major: u32, minor: u32, patch: u32) -> u32 {
    major
        .wrapping_mul(1000000)
        .wrapping_add(minor.wrapping_mul(1000))
        .wrapping_add(patch)
}

// ufbx.h:270 `UFBX_HEADER_VERSION`
pub(crate) const HEADER_VERSION: u32 = pack_version(0, 23, 0);

// ufbx.h:396 `#define UFBX_NO_INDEX ((uint32_t)~0u)`
pub(crate) const NO_INDEX: u32 = !0u32;

// ufbx.c:877 `UFBX_SOURCE_VERSION`
pub(crate) const SOURCE_VERSION: u32 = pack_version(0, 23, 0);

// ufbx.c:878 `ufbx_abi_data_def const uint32_t ufbx_source_version` — the
// public ABI data symbol lives in `capi.rs` (feature = "c-abi") per the
// PORTING.md naming table; exporting it unconditionally would collide with the
// C original when both are linked (validation ladder rung 1.5/2).

// ufbx.c:880 `ufbx_static_assert(source_header_version, ...)`
const _: () = assert!(SOURCE_VERSION / 1000 == HEADER_VERSION / 1000);

// -- Fast copy

// ufbx.c:883-895 `ufbxi_copy_16_bytes`
// C-parity: ported as a full 16-byte load followed by a full 16-byte store —
// exactly the SSE branch (ufbx.c:885, `_mm_storeu_si128(dst,
// _mm_loadu_si128(src))`) that every default x86-64 build compiles. This is
// deliberate: the DEFLATE match copies (ufbx.c:2877-2886, 3065-3067) call this
// with OVERLAPPING regions whenever `distance < 16` (guard is only
// `distance >= min(length, 16)`), so `ptr::copy_nonoverlapping`/memcpy
// semantics would be UB there. Load-then-store is well defined under overlap
// and bit-identical to the SSE branch.
#[inline(always)]
pub(crate) unsafe fn copy_16_bytes(dst: *mut u8, src: *const u8) {
    let t = (src as *const [u8; 16]).read_unaligned();
    (dst as *mut [u8; 16]).write_unaligned(t);
}

// -- Large fast integer

// ufbx.c:899-904 `ufbxi_fast_uint`
// C picks `uint64_t` on wasm (unless `UFBX_WASM_32BIT`, unsupported), `size_t`
// otherwise.
#[cfg(target_family = "wasm")]
pub(crate) type FastUint = u64;
#[cfg(not(target_family = "wasm"))]
pub(crate) type FastUint = usize;

// -- Wrapping right shift

// ufbx.c:907-912 `ufbxi_wrap_shr64`
// C fast branch (`(a) >> (b)`, ufbx.c:909) relies on x86-64 implicit shift
// masking; the portable branch masks explicitly. Rust: always mask `& 63`
// (PORTING.md integer table) — a bare `>>` with b >= 64 panics in debug.
#[inline(always)]
pub(crate) fn wrap_shr64(a: u64, b: u32) -> u64 {
    a >> (b & 63)
}

// -- Bit manipulation

// ufbx.c:916-967 `ufbxi_lzcnt32`
// C: "DeBrujin table lookup" (fallback branch, ufbx.c:941-966).
// C-parity: all three C branches (BitScanReverse / __builtin_clz / DeBrujin
// table) agree for v != 0 and are undefined or inconsistent at v == 0; callers
// never pass 0 (e.g. `ufbxi_lzcnt64(digits)` at ufbx.c:1719 is reached only
// when `digits_valid`, which implies `big_mantissa.length == 0` at ufbx.c:1687,
// where `digits == 0` returns early at ufbx.c:1691; the bigint call sites are
// guarded by `length != 0` / normalized top limbs). `leading_zeros` is
// bit-identical for v != 0.
#[inline(always)]
pub(crate) fn lzcnt32(v: u32) -> u32 {
    v.leading_zeros()
}

// ufbx.c:916-967 `ufbxi_lzcnt64` (same C-parity note as `lzcnt32`)
#[inline(always)]
pub(crate) fn lzcnt64(v: u64) -> u32 {
    v.leading_zeros()
}

// -- Bit conversion

// ufbx.c:971-976 `ufbxi_bit_cast(m_dst_type, m_dst, m_src_type, m_src)`:
// union type-pun / memcpy — the semantic is a bit-copy. Collapses at use sites
// to `f64::to_bits` / `f64::from_bits` / `f32::to_bits` / `f32::from_bits`
// (PORTING.md "Unions" table row 975-976); no standalone definition needed.

// -- Pointer alignment

// ufbx.c:980-990 `ufbxi_is_aligned(m_ptr, m_align)` — align is a power of two.
#[inline(always)]
pub(crate) fn is_aligned<T>(ptr: *const T, align: usize) -> bool {
    (ptr as usize) & align.wrapping_sub(1) == 0
}

// ufbx.c:980-990 `ufbxi_is_aligned_mask(m_ptr, m_align)` — takes the mask
// (align - 1) directly.
#[inline(always)]
pub(crate) fn is_aligned_mask<T>(ptr: *const T, mask: usize) -> bool {
    (ptr as usize) & mask == 0
}

// -- Byte order (unaligned little-endian readers)

// C (ufbx.c:740-742): "Unaligned little-endian load functions
// On platforms that support unaligned access natively (x86, x64, ARM64) just
// use normal loads, with unaligned attributes, otherwise do manual byte-wise
// load."

// ufbx.c:745 `#define ufbxi_read_u8(ptr) (*(const uint8_t*)(ptr))`
#[inline(always)]
pub(crate) unsafe fn read_u8(ptr: *const u8) -> u8 {
    *ptr
}

// ufbx.c:791-836 `ufbxi_read_u16`
// The unaligned-pointer fast path (ufbx.c:792-796) and the byte-assembly
// portable path (ufbx.c:798-835) collapse to read_unaligned + from_le_bytes
// (PORTING.md "Byte order"); correct on big-endian targets too.
#[inline(always)]
pub(crate) unsafe fn read_u16(ptr: *const u8) -> u16 {
    u16::from_le_bytes((ptr as *const [u8; 2]).read_unaligned())
}

// ufbx.c:791-830 `ufbxi_read_u32`
#[inline(always)]
pub(crate) unsafe fn read_u32(ptr: *const u8) -> u32 {
    u32::from_le_bytes((ptr as *const [u8; 4]).read_unaligned())
}

// ufbx.c:791-830 `ufbxi_read_u64`
#[inline(always)]
pub(crate) unsafe fn read_u64(ptr: *const u8) -> u64 {
    u64::from_le_bytes((ptr as *const [u8; 8]).read_unaligned())
}

// ufbx.c:791-830 `ufbxi_read_f32` (u32 read + bit copy, ufbx.c:824-829)
#[inline(always)]
pub(crate) unsafe fn read_f32(ptr: *const u8) -> f32 {
    f32::from_bits(read_u32(ptr))
}

// ufbx.c:791-830 `ufbxi_read_f64` (u64 read + bit copy, ufbx.c:824-829)
#[inline(always)]
pub(crate) unsafe fn read_f64(ptr: *const u8) -> f64 {
    f64::from_bits(read_u64(ptr))
}

// ufbx.c:838 `#define ufbxi_read_i8(ptr) (int8_t)(ufbxi_read_u8(ptr))`
#[inline(always)]
pub(crate) unsafe fn read_i8(ptr: *const u8) -> i8 {
    read_u8(ptr) as i8
}

// ufbx.c:839 `ufbxi_read_i16`
#[inline(always)]
pub(crate) unsafe fn read_i16(ptr: *const u8) -> i16 {
    read_u16(ptr) as i16
}

// ufbx.c:840 `ufbxi_read_i32`
#[inline(always)]
pub(crate) unsafe fn read_i32(ptr: *const u8) -> i32 {
    read_u32(ptr) as i32
}

// ufbx.c:841 `ufbxi_read_i64`
#[inline(always)]
pub(crate) unsafe fn read_i64(ptr: *const u8) -> i64 {
    read_u64(ptr) as i64
}

// ufbx.c:843-854 sizeof static asserts — fixed-width types make most of these
// tautological in Rust; kept for grep-parity where meaningful.
const _: () = {
    assert!(size_of::<bool>() == 1);
    assert!(size_of::<u8>() == 1 && size_of::<i8>() == 1);
    assert!(size_of::<u16>() == 2 && size_of::<i16>() == 2);
    assert!(size_of::<u32>() == 4 && size_of::<i32>() == 4);
    assert!(size_of::<u64>() == 8 && size_of::<i64>() == 8);
    assert!(size_of::<f32>() == 4 && size_of::<f64>() == 8);
};

// -- Swap

// ufbx.c:1305-1311 scratch union `{ void *align_ptr; uintptr_t align_uptr;
// uint64_t align_u64; char data[256]; }` — an alignment device (PORTING.md
// "Unions" table).
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub(crate) struct SwapScratch {
    pub(crate) data: [u8; 256],
}

// ufbx.c:1293-1316 `ufbxi_swap`
// C aliasing-branch guard comment (ufbx.c:1295): "CHERI needs to copy pointer
// metadata tag bits.."
// C-parity: the u32-aliasing branch (ufbx.c:1295-1303, taken by every real
// clang/gcc/MSVC build via UFBXI_HAS_ALIASING) and the memcpy-via-scratch
// branch (ufbx.c:1304-1315) produce identical results for inputs satisfying
// BOTH branches' preconditions; ported as the portable scratch branch. To keep
// dev-build parity with the aliasing branch actually compiled by real C
// builds, its dev-assert (`size % 4 == 0` and 4-aligned pointers, ufbx.c:1296)
// is checked here alongside the portable branch's `size <= sizeof(tmp)`
// (ufbx.c:1312), which the scratch copy additionally requires.
#[inline(always)]
pub(crate) unsafe fn swap(a: *mut u8, b: *mut u8, size: usize) {
    let mut tmp = core::mem::MaybeUninit::<SwapScratch>::uninit();
    ufbxi_dev_assert!(size % 4 == 0 && (a as usize) % 4 == 0 && (b as usize) % 4 == 0);
    ufbxi_dev_assert!(size <= size_of::<SwapScratch>());
    let tmp_data = (*tmp.as_mut_ptr()).data.as_mut_ptr();
    core::ptr::copy_nonoverlapping(a as *const u8, tmp_data, size);
    core::ptr::copy_nonoverlapping(b as *const u8, a, size);
    core::ptr::copy_nonoverlapping(tmp_data as *const u8, b, size);
}

// -- Utility (ufbx.c:1082-1348)
//
// `ufbxi_swap` (ufbx.c:1293-1316) belongs to this banner section in C; it is
// ported in the `// -- Swap` section above (its scratch union is a platform
// alignment device).
//
// Collapsed-away C apparatus in this section (no Rust analogue):
// - ufbx.c:1084-1091 `ufbxi_add_ptr` / `ufbxi_sub_ptr`: see below (ported as
//   fns; the UBSAN branch, ufbx.c:1085-1087, is diagnostics-only).
// - ufbx.c:1093 `ufbxi_arraycount(arr)`: `sizeof` array-length computation —
//   collapses to `.len()` / `N` at use sites.
// - ufbx.c:1094-1099 `ufbxi_for` / `ufbxi_for_ptr` / `ufbxi_for_list` /
//   `ufbxi_for_ptr_list`: pointer-iteration loop sugar — expanded to explicit
//   pointer loops at use sites. C comment (ufbx.c:1097): "WARNING: Evaluates
//   `m_list` twice!" — irrelevant once expanded, noted for grep-parity.

// ufbx.c:1101 `#define ufbxi_string_literal(str) { str, sizeof(str) - 1 }`
// C: aggregate initializer for `ufbx_string`, deriving the length from the
// literal so it can never drift. Rust byte-string literals are not
// NUL-terminated, so the NUL is spelled out in the literal and `.len() - 1` is
// the exact analogue of `sizeof(str) - 1`.
macro_rules! ufbxi_string_literal {
    ($str:expr) => {
        crate::prelude::String::new_c($str.as_ptr(), $str.len() - 1)
    };
}
pub(crate) use ufbxi_string_literal;

// ufbx.c:1084-1091 `ufbxi_add_ptr(ptr, offset)` / `ufbxi_sub_ptr(ptr, offset)`
// C: plain pointer arithmetic (ufbx.c:1089-1090); the UBSAN branch
// (ufbx.c:1085-1087) exists because callers reach `NULL + 0` (well-formed in
// practice, UB pedantically). `wrapping_add`/`wrapping_sub` is defined for the
// null-with-zero-offset case and bit-identical to C's `+`/`-` otherwise.
#[inline(always)]
pub(crate) fn add_ptr<T>(ptr: *mut T, offset: usize) -> *mut T {
    ptr.wrapping_add(offset)
}
#[inline(always)]
pub(crate) fn sub_ptr<T>(ptr: *mut T, offset: usize) -> *mut T {
    ptr.wrapping_sub(offset)
}

// ufbx.c:1103 `ufbxi_min32`
#[inline(always)]
pub(crate) fn min32(a: u32, b: u32) -> u32 {
    if a < b {
        a
    } else {
        b
    }
}
// ufbx.c:1104 `ufbxi_max32`
#[inline(always)]
pub(crate) fn max32(a: u32, b: u32) -> u32 {
    if a < b {
        b
    } else {
        a
    }
}
// ufbx.c:1105 `ufbxi_min64`
#[inline(always)]
pub(crate) fn min64(a: u64, b: u64) -> u64 {
    if a < b {
        a
    } else {
        b
    }
}
// ufbx.c:1106 `ufbxi_max64`
#[inline(always)]
pub(crate) fn max64(a: u64, b: u64) -> u64 {
    if a < b {
        b
    } else {
        a
    }
}
// ufbx.c:1107 `ufbxi_min_sz`
#[inline(always)]
pub(crate) fn min_sz(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}
// ufbx.c:1108 `ufbxi_max_sz`
#[inline(always)]
pub(crate) fn max_sz(a: usize, b: usize) -> usize {
    if a < b {
        b
    } else {
        a
    }
}
// ufbx.c:1109 `ufbxi_min_real`
// NaN-parity: `a < b ? a : b` verbatim — NOT `f64::min` (trap #6).
#[inline(always)]
pub(crate) fn min_real(a: crate::prelude::Real, b: crate::prelude::Real) -> crate::prelude::Real {
    if a < b {
        a
    } else {
        b
    }
}
// ufbx.c:1110 `ufbxi_max_real`
#[inline(always)]
pub(crate) fn max_real(a: crate::prelude::Real, b: crate::prelude::Real) -> crate::prelude::Real {
    if a < b {
        b
    } else {
        a
    }
}

// ufbx.c:1112-1119 `ufbxi_f64_to_i32`
// C-parity: `(double)INT32_MAX` is exactly 2147483647.0 (INT32_MAX < 2^53, so
// it is exactly representable — it does NOT round up). Every `value` passing
// the guard is in [-INT32_MAX, INT32_MAX], where the C `(int32_t)` cast is
// well-defined truncation toward zero and Rust `as` is identical (never
// saturates in-range). Values like 2147483648.0 fail the guard and take the
// else branch in both C and Rust — unlike `f64_to_i64` below, this function
// has NO UB boundary case.
#[inline(always)]
pub(crate) fn f64_to_i32(value: f64) -> i32 {
    if math::fabs(value) <= i32::MAX as f64 {
        value as i32
    } else {
        if value >= 0.0 {
            i32::MAX
        } else {
            i32::MIN
        }
    }
}

// ufbx.c:1121-1128 `ufbxi_f64_to_i64`
// C-parity boundary at `value == +2^63` (9223372036854775808.0):
// `(double)INT64_MAX` rounds to 2^63 exactly, so 2^63 passes the guard and the
// C `(int64_t)` cast is UB with target-split behavior: x86-64 `cvttsd2si`
// yields INT64_MIN; AArch64 `fcvtzs` saturates to INT64_MAX. Rust `as`
// saturates to i64::MAX on ALL targets — this matches the AArch64 C build (and
// the C else-branch intent) but DIVERGES from an x86-64 C build, which
// produces INT64_MIN for exactly this input. Deliberate decision recorded in
// PORTING.md ("f64→i64 boundary"): the port is deterministic saturation; no
// portable choice can match both C builds. All other inputs are bit-identical.
#[inline(always)]
pub(crate) fn f64_to_i64(value: f64) -> i64 {
    if math::fabs(value) <= i64::MAX as f64 {
        value as i64
    } else {
        if value >= 0.0 {
            i64::MAX
        } else {
            i64::MIN
        }
    }
}

// ufbx.c:1130-1137 `ufbxi_to_size(delta)`
// Regression branch (ufbx.c:1131-1134) asserts `delta >= 0`; release branch
// (ufbx.c:1136) is a bare `(size_t)` cast.
#[inline(always)]
pub(crate) fn to_size(delta: isize) -> usize {
    #[cfg(feature = "regression")]
    ufbx_assert!(delta >= 0);
    delta as usize
}

// C comment (ufbx.c:1139-1141):
// Stable sort array `m_type m_data[m_size]` using the predicate `m_cmp_lambda(a, b)`
// `m_linear_size` is a hint for how large blocks handle initially do with insertion sort
// `m_tmp` must be a memory buffer with at least the same size and alignment as `m_data`
//
// ufbx.c:1142-1186 `ufbxi_macro_stable_sort(m_type, m_linear_size, m_data,
// m_tmp, m_size, m_cmp_lambda)`.
// Token-pasting generic macro → generic fn mirroring the actual instantiations
// (PORTING.md "Macros & feature gates"); the comparator lambda becomes a
// closure over `(a, b)` element pointers, called exactly where and as often as
// the C macro evaluates `m_cmp_lambda`.
// NOTE (PORTING.md "Sorting & searching"): this is a DIFFERENT algorithm from
// `stable_sort` below — the insertion pass here copies `src[0] = dst[i]`
// unconditionally and has NO early-out, and the merge tails are element loops,
// not bulk memcpys. Do not unify.
pub(crate) unsafe fn macro_stable_sort<T: Copy>(
    linear_size: usize,
    data: *mut T,
    tmp: *mut T,
    size: usize,
    mut cmp_lambda: impl FnMut(*const T, *const T) -> bool,
) {
    let mut src: *mut T = tmp;
    let data: *mut T = data;
    let mut dst: *mut T = data;
    let mut block_size = clamp_linear_threshold(linear_size);
    // Insertion sort in `m_linear_size` blocks
    let mut base = 0usize;
    while base < size {
        let mut i_end = base + block_size;
        if i_end > size {
            i_end = size;
        }
        let mut i = base + 1;
        while i < i_end {
            let mut j = i;
            *src = *dst.add(i); // mi_src[0] = mi_dst[mi_i];
            while j != base {
                let a: *const T = &*src;
                let b: *const T = dst.add(j - 1);
                if !cmp_lambda(a, b) {
                    break;
                }
                *dst.add(j) = *dst.add(j - 1);
                j -= 1;
            }
            *dst.add(j) = *src;
            i += 1;
        }
        base += block_size;
    }
    // Merge sort ping-ponging between `m_data` and `m_tmp`
    while block_size < size {
        let swap = dst;
        dst = src;
        src = swap;
        let mut base = 0usize;
        while base < size {
            let mut i = base;
            let mut i_end = base + block_size;
            let mut j = i_end;
            let mut j_end = j + block_size;
            let mut k = base;
            if i_end > size {
                i_end = size;
            }
            if j_end > size {
                j_end = size;
            }
            // C: `(mi_i < mi_i_end) & (mi_j < mi_j_end)` — non-short-circuit.
            while (i < i_end) & (j < j_end) {
                let a: *const T = src.add(j);
                let b: *const T = src.add(i);
                if cmp_lambda(a, b) {
                    *dst.add(k) = *a;
                    j += 1;
                } else {
                    *dst.add(k) = *b;
                    i += 1;
                }
                k += 1;
            }
            while i < i_end {
                *dst.add(k) = *src.add(i);
                k += 1;
                i += 1;
            }
            while j < j_end {
                *dst.add(k) = *src.add(j);
                k += 1;
                j += 1;
            }
            base += block_size * 2;
        }
        block_size *= 2;
    }
    // Copy the result to `m_data` if we ended up in `m_tmp`
    if dst != data {
        core::ptr::copy_nonoverlapping(dst as *const T, data, size);
    }
}

// ufbx.c:1188-1204 `ufbxi_macro_lower_bound_eq(m_type, m_linear_size,
// m_result_ptr, m_data, m_begin, m_size, m_cmp_lambda, m_eq_lambda)`.
// Does NOT write `*result_ptr` on miss — callers pre-initialize (e.g.
// `size_t index = SIZE_MAX;`, ufbx.c:13362). Keep the out-param shape; do NOT
// return `Option` (PORTING.md "Sorting & searching").
pub(crate) unsafe fn macro_lower_bound_eq<T>(
    linear_size: usize,
    result_ptr: *mut usize,
    data: *const T,
    begin: usize,
    size: usize,
    mut cmp_lambda: impl FnMut(*const T) -> bool,
    mut eq_lambda: impl FnMut(*const T) -> bool,
) {
    let mut lo = begin;
    let mut hi = size;
    let linear_size = clamp_linear_threshold(linear_size);
    ufbx_assert!(linear_size > 1);
    // Binary search until we get down to `m_linear_size` elements
    while hi - lo > linear_size {
        let mid = lo + (hi - lo) / 2;
        let a: *const T = data.add(mid);
        if cmp_lambda(a) {
            lo = mid + 1;
        } else {
            hi = mid + 1;
        }
    }
    // Linearly scan until we find the edge
    while lo < hi {
        let a: *const T = data.add(lo);
        if eq_lambda(a) {
            *result_ptr = lo;
            break;
        }
        lo += 1;
    }
}

// ufbx.c:1206-1229 `ufbxi_macro_upper_bound_eq(m_type, m_linear_size,
// m_result_ptr, m_data, m_begin, m_size, m_eq_lambda)`.
// ALWAYS writes `*result_ptr` (contrast with `macro_lower_bound_eq`).
pub(crate) unsafe fn macro_upper_bound_eq<T>(
    linear_size: usize,
    result_ptr: *mut usize,
    data: *const T,
    begin: usize,
    size: usize,
    mut eq_lambda: impl FnMut(*const T) -> bool,
) {
    let mut lo = begin;
    let mut hi = size;
    let linear_size = clamp_linear_threshold(linear_size);
    ufbx_assert!(linear_size > 1);
    // Linearly scan with galloping
    let mut step = 1usize;
    while step < 100 && hi - lo > step {
        let a: *const T = data.add(lo + step);
        if !eq_lambda(a) {
            hi = lo + step;
            break;
        }
        lo += step;
        step *= 2;
    }
    // Binary search until we get down to `m_linear_size` elements
    while hi - lo > linear_size {
        let mid = lo + (hi - lo) / 2;
        let a: *const T = data.add(mid);
        if eq_lambda(a) {
            lo = mid + 1;
        } else {
            hi = mid + 1;
        }
    }
    // Linearly scan until we find the edge
    while lo < hi {
        let a: *const T = data.add(lo);
        if !eq_lambda(a) {
            break;
        }
        lo += 1;
    }
    *result_ptr = lo;
}

// ufbx.c:1231 `typedef bool ufbxi_less_fn(void *user, const void *a, const void *b);`
// C passes function designators (`&ufbxi_uv_set_less`) — fn pointers, never
// closures (PORTING.md "Callbacks").
pub(crate) type LessFn =
    unsafe extern "C" fn(user: *mut c_void, a: *const c_void, b: *const c_void) -> bool;

// ufbx.c:1233-1291 `ufbxi_stable_sort` (fn — NOT the macro above; different
// algorithm: the insertion pass has an early-out `continue`, and the merge
// tails are bulk memcpys. Do not unify. PORTING.md "Sorting & searching").
// The sorts do not allocate — `in_tmp` is caller-supplied scratch.
#[inline(never)]
pub(crate) unsafe fn stable_sort(
    stride: usize,
    linear_size: usize,
    in_data: *mut c_void,
    in_tmp: *mut c_void,
    size: usize,
    less_fn: LessFn,
    less_user: *mut c_void,
) {
    // C: `(void)linear_size;` (consumed only through the clamp macro, which
    // ignores it under UFBX_DEBUG_BINARY_SEARCH/UFBX_REGRESSION).
    let _ = linear_size;

    let mut src: *mut u8 = in_tmp as *mut u8;
    let data: *mut u8 = in_data as *mut u8;
    let mut dst: *mut u8 = data;
    let mut block_size = clamp_linear_threshold(linear_size);
    // Insertion sort in `linear_size` blocks
    let mut base = 0usize;
    while base < size {
        let mut i_end = base + block_size;
        if i_end > size {
            i_end = size;
        }
        let mut i = base + 1;
        while i < i_end {
            // Early-out (the macro variant lacks this):
            // C: { char *a = dst + i*stride, *b = dst + (i-1)*stride;
            //      if (!less_fn(less_user, a, b)) continue; }
            {
                let a = dst.add(i * stride);
                let b = dst.add((i - 1) * stride);
                if !less_fn(less_user, a as *const c_void, b as *const c_void) {
                    i += 1;
                    continue;
                }
            }

            let mut j = i - 1;
            // memcpy(src, dst + i * stride, stride);
            core::ptr::copy_nonoverlapping(dst.add(i * stride) as *const u8, src, stride);
            // memcpy(dst + i * stride, dst + j * stride, stride);
            core::ptr::copy_nonoverlapping(
                dst.add(j * stride) as *const u8,
                dst.add(i * stride),
                stride,
            );
            while j != base {
                let a = src as *const c_void;
                let b = dst.add((j - 1) * stride);
                if !less_fn(less_user, a, b as *const c_void) {
                    break;
                }
                // memcpy(dst + j * stride, dst + (j - 1) * stride, stride);
                core::ptr::copy_nonoverlapping(
                    dst.add((j - 1) * stride) as *const u8,
                    dst.add(j * stride),
                    stride,
                );
                j -= 1;
            }
            // memcpy(dst + j * stride, src, stride);
            core::ptr::copy_nonoverlapping(src as *const u8, dst.add(j * stride), stride);
            i += 1;
        }
        base += block_size;
    }
    // Merge sort ping-ponging between `data` and `tmp`
    while block_size < size {
        let swap = dst;
        dst = src;
        src = swap;
        let mut base = 0usize;
        while base < size {
            let mut i = base;
            let mut i_end = base + block_size;
            let mut j = i_end;
            let mut j_end = j + block_size;
            let mut k = base;
            if i_end > size {
                i_end = size;
            }
            if j_end > size {
                j_end = size;
            }
            // C: `(i < i_end) & (j < j_end)` — non-short-circuit.
            while (i < i_end) & (j < j_end) {
                let a = src.add(j * stride);
                let b = src.add(i * stride);
                if less_fn(less_user, a as *const c_void, b as *const c_void) {
                    core::ptr::copy_nonoverlapping(a as *const u8, dst.add(k * stride), stride);
                    j += 1;
                } else {
                    core::ptr::copy_nonoverlapping(b as *const u8, dst.add(k * stride), stride);
                    i += 1;
                }
                k += 1;
            }

            // Bulk-memcpy merge tails (the macro variant uses element loops):
            // memcpy(dst + k * stride, src + i * stride, (i_end - i) * stride);
            core::ptr::copy_nonoverlapping(
                src.add(i * stride) as *const u8,
                dst.add(k * stride),
                (i_end - i) * stride,
            );
            if j < j_end {
                // memcpy(dst + (k + (i_end - i)) * stride, src + j * stride, (j_end - j) * stride);
                core::ptr::copy_nonoverlapping(
                    src.add(j * stride) as *const u8,
                    dst.add((k + (i_end - i)) * stride),
                    (j_end - j) * stride,
                );
            }
            base += block_size * 2;
        }
        block_size *= 2;
    }
    // Copy the result to `data` if we ended up in `tmp`
    if dst != data {
        core::ptr::copy_nonoverlapping(dst as *const u8, data, size * stride);
    }
}

// ufbx.c:1318-1348 `ufbxi_unstable_sort` (heapsort; uses `ufbxi_swap`,
// ported above in the `// -- Swap` section).
#[inline(never)]
pub(crate) unsafe fn unstable_sort(
    in_data: *mut c_void,
    size: usize,
    stride: usize,
    less_fn: LessFn,
    less_user: *mut c_void,
) {
    if size <= 1 {
        return;
    }

    let data = in_data as *mut u8;
    let mut start = (size - 1) >> 1;
    let mut end = size - 1;
    loop {
        let mut root = start;
        // C: `while ((child = root*2 + 1) <= end)` — assignment decomposed.
        loop {
            let child = root * 2 + 1;
            if !(child <= end) {
                break;
            }
            let mut next = if less_fn(
                less_user,
                data.add(child * stride) as *const c_void,
                data.add(root * stride) as *const c_void,
            ) {
                root
            } else {
                child
            };
            if child + 1 <= end
                && less_fn(
                    less_user,
                    data.add(next * stride) as *const c_void,
                    data.add((child + 1) * stride) as *const c_void,
                )
            {
                next = child + 1;
            }
            if next == root {
                break;
            }
            swap(data.add(root * stride), data.add(next * stride), stride);
            root = next;
        }

        if start > 0 {
            start -= 1;
        } else if end > 0 {
            swap(data.add(end * stride), data, stride);
            end -= 1;
        } else {
            break;
        }
    }
}

// -- Math shim
//
// The C surface is exactly ufbx.c:257-276 (`ufbx_sqrt` .. `ufbx_isnan`).
// NOTE: the C oracle builds are NOT plain libm: both the test runner
// (misc/run_tests.py:996) and the hash oracle `hash_scene`
// (misc/run_tests.py:1542/1551, test/hash_scene.c:285 `UFBX_EXTERNAL_MATH`)
// link `extra/ufbx_math.c`, so ufbx_math semantics are the parity target and
// platform libm is NOT usable here. Per PORTING.md "Floats" every entry below
// routes through `native::math`, the 1:1 port of that file: libm agrees with
// it only to within ~1 ULP, and the per-call disagreement rate is
// percent-scale (measured on this target: sin 4.0%, cos 4.7%, tan 39.5%,
// atan 7.0%, atan2 18.0%, asin 8.8%, acos 17.3%, pow 8.7%), which reaches the
// scene hashes directly through `ufbx_euler_to_quat` (ufbx.c:31566-31620) and
// friends. `sqrt`/`fabs` are the exception only in spelling: the C picks SSE/
// NEON intrinsics for them and `native::math` documents the equivalence.
pub(crate) mod math {
    use crate::native::math as ufbxm;

    // ufbx.c:337-350 `UFBX_INFINITY` / `UFBX_NAN`
    pub(crate) const INFINITY: f64 = f64::INFINITY;
    pub(crate) const NAN: f64 = f64::NAN;
    // ufbx.c:351-357 `UFBX_FLT_EPSILON`
    pub(crate) const FLT_EPSILON: f32 = f32::EPSILON;
    // ufbx.c:68-72 `UFBX_EPSILON`
    // C comment: By default enough to have squares be non-denormal
    // C: `sizeof(ufbx_real) == sizeof(float) ? (ufbx_real)1.0842021795674597e-19f
    //     : (ufbx_real)1.4916681462400413e-154` — same compile-time select.
    pub(crate) const EPSILON: crate::prelude::Real =
        if core::mem::size_of::<crate::prelude::Real>() == core::mem::size_of::<f32>() {
            1.0842021795674597e-19
        } else {
            1.4916681462400413e-154
        };
    // ufbx.c:358-368 `UFBX_FLT_EVAL_METHOD`
    // C-parity: 0 on every target this port supports — Rust never evaluates
    // f32/f64 at excess precision (no x87 codegen on supported targets). The
    // strtod probe (float_parse unit) consumes this; C builds on x87 targets
    // would see -1/2 there.
    pub(crate) const FLT_EVAL_METHOD: i32 = 0;

    // ufbx.c:259 `ufbx_sqrt`
    #[inline(always)]
    pub(crate) fn sqrt(x: f64) -> f64 {
        ufbxm::sqrt(x)
    }

    // ufbx.c:262 `ufbx_sin`
    #[inline(always)]
    pub(crate) fn sin(x: f64) -> f64 {
        ufbxm::sin(x)
    }

    // ufbx.c:263 `ufbx_cos`
    #[inline(always)]
    pub(crate) fn cos(x: f64) -> f64 {
        ufbxm::cos(x)
    }

    // ufbx.c:264 `ufbx_tan`
    #[inline(always)]
    pub(crate) fn tan(x: f64) -> f64 {
        ufbxm::tan(x)
    }

    // ufbx.c:265 `ufbx_asin`
    #[inline(always)]
    pub(crate) fn asin(x: f64) -> f64 {
        ufbxm::asin(x)
    }

    // ufbx.c:266 `ufbx_acos`
    #[inline(always)]
    pub(crate) fn acos(x: f64) -> f64 {
        ufbxm::acos(x)
    }

    // ufbx.c:267 `ufbx_atan`
    #[inline(always)]
    pub(crate) fn atan(x: f64) -> f64 {
        ufbxm::atan(x)
    }

    // ufbx.c:268 `ufbx_atan2(y, x)`
    #[inline(always)]
    pub(crate) fn atan2(y: f64, x: f64) -> f64 {
        ufbxm::atan2(y, x)
    }

    // ufbx.c:261 `ufbx_pow(x, y)`
    #[inline(always)]
    pub(crate) fn pow(x: f64, y: f64) -> f64 {
        ufbxm::pow(x, y)
    }

    // ufbx.c:270 `ufbx_fmin` — C: `return a < b ? a : b;`. NaN semantics
    // differ from libm/std `fmin`/`f64::min` (fmin(2.0, NaN) here is NaN,
    // std's is 2.0), trap #6.
    #[inline(always)]
    pub(crate) fn fmin(a: f64, b: f64) -> f64 {
        ufbxm::fmin(a, b)
    }

    // ufbx.c:271 `ufbx_fmax` — same NaN caveat as `fmin`.
    #[inline(always)]
    pub(crate) fn fmax(a: f64, b: f64) -> f64 {
        ufbxm::fmax(a, b)
    }

    // ufbx.c:260 `ufbx_fabs`
    #[inline(always)]
    pub(crate) fn fabs(x: f64) -> f64 {
        ufbxm::fabs(x)
    }

    // ufbx.c:269 `ufbx_copysign(x, y)`
    #[inline(always)]
    pub(crate) fn copysign(x: f64, y: f64) -> f64 {
        ufbxm::copysign(x, y)
    }

    // ufbx.c:272 `ufbx_nextafter(x, y)`
    #[inline(always)]
    pub(crate) fn nextafter(x: f64, y: f64) -> f64 {
        ufbxm::nextafter(x, y)
    }

    // ufbx.c:273 `ufbx_rint` — half-to-even, NEVER `f64::round`
    // (half-away-from-zero); on the keyframe-time quantization path straight
    // into the hash oracle (PORTING.md "Floats").
    #[inline(always)]
    pub(crate) fn rint(x: f64) -> f64 {
        ufbxm::rint(x)
    }

    // ufbx.c:274 `ufbx_floor`
    #[inline(always)]
    pub(crate) fn floor(x: f64) -> f64 {
        ufbxm::floor(x)
    }

    // ufbx.c:275 `ufbx_ceil`
    #[inline(always)]
    pub(crate) fn ceil(x: f64) -> f64 {
        ufbxm::ceil(x)
    }

    // ufbx.c:276 `ufbx_isnan` (C returns int; used only as a truth value)
    #[inline(always)]
    pub(crate) fn isnan(x: f64) -> bool {
        ufbxm::isnan(x) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_shr64() {
        // Portable C branch: `(a) >> ((b) & 63)`
        assert_eq!(wrap_shr64(0xDEAD_BEEF_1234_5678, 0), 0xDEAD_BEEF_1234_5678);
        assert_eq!(wrap_shr64(0xDEAD_BEEF_1234_5678, 4), 0x0DEA_DBEE_F123_4567);
        assert_eq!(wrap_shr64(u64::MAX, 63), 1);
        // b >= 64 wraps (x86 implicit-masking semantics)
        assert_eq!(wrap_shr64(0xDEAD_BEEF_1234_5678, 64), 0xDEAD_BEEF_1234_5678);
        assert_eq!(wrap_shr64(0x8000_0000_0000_0000, 65), 0x4000_0000_0000_0000);
    }

    #[test]
    fn test_lzcnt() {
        assert_eq!(lzcnt32(1), 31);
        assert_eq!(lzcnt32(0x8000_0000), 0);
        assert_eq!(lzcnt32(u32::MAX), 0);
        assert_eq!(lzcnt32(0x0000_1000), 19);
        assert_eq!(lzcnt64(1), 63);
        assert_eq!(lzcnt64(0x8000_0000_0000_0000), 0);
        assert_eq!(lzcnt64(0x0000_0001_0000_0000), 31);
        // C's DeBrujin fallback agrees with these for every nonzero input by
        // construction; spot-check the table math for a few values.
        for &v in &[1u32, 2, 3, 0x1234, 0x8000_0000, 0xFFFF_FFFF] {
            let mut x = v;
            x |= x >> 1;
            x |= x >> 2;
            x |= x >> 4;
            x |= x >> 8;
            x |= x >> 16;
            const TABLE: [u8; 32] = [
                31, 22, 30, 21, 18, 10, 29, 2, 20, 17, 15, 13, 9, 6, 28, 1, 23, 19, 11, 3, 16, 14,
                7, 24, 12, 4, 8, 25, 5, 26, 27, 0,
            ];
            let expect = TABLE[(x.wrapping_mul(0x07c4_acdd) >> 27) as usize] as u32;
            assert_eq!(lzcnt32(v), expect);
        }
    }

    #[test]
    fn test_read_unaligned_le() {
        // Offset by 1 to force unaligned access.
        let buf: [u8; 17] = [
            0xFF, 0x78, 0x56, 0x34, 0x12, 0xEF, 0xBE, 0xAD, 0xDE, 0x00, 0x00, 0x80,
            0x3F, // 1.0f LE
            0x01, 0x02, 0x03, 0x04,
        ];
        unsafe {
            let p = buf.as_ptr().add(1);
            assert_eq!(read_u8(p), 0x78);
            assert_eq!(read_u16(p), 0x5678);
            assert_eq!(read_u32(p), 0x1234_5678);
            assert_eq!(read_u64(p), 0xDEAD_BEEF_1234_5678);
            assert_eq!(read_i32(buf.as_ptr().add(5)), 0xDEAD_BEEFu32 as i32);
            assert_eq!(read_i16(p), 0x5678);
            assert_eq!(read_i8(buf.as_ptr().add(8)), 0xDEu8 as i8);
            assert_eq!(read_f32(buf.as_ptr().add(9)), 1.0f32);
            assert_eq!(read_f64(p), f64::from_bits(0xDEAD_BEEF_1234_5678));
        }
    }

    #[test]
    fn test_swap() {
        // 4-aligned, size % 4 == 0: the contract the C aliasing-branch
        // dev-assert (ufbx.c:1296) imposes on all callers.
        #[repr(C, align(4))]
        struct Aligned([u8; 12]);
        let mut a = Aligned(*b"hello world!");
        let mut b = Aligned(*b"HELLO WORLD?");
        unsafe { swap(a.0.as_mut_ptr(), b.0.as_mut_ptr(), 12) };
        assert_eq!(&a.0, b"HELLO WORLD?");
        assert_eq!(&b.0, b"hello world!");
        // Scratch struct is the C union's alignment device.
        assert_eq!(core::mem::size_of::<SwapScratch>(), 256);
        assert_eq!(core::mem::align_of::<SwapScratch>(), 8);
    }

    #[test]
    fn test_copy_16_bytes() {
        let src: [u8; 16] = *b"0123456789abcdef";
        let mut dst = [0u8; 16];
        unsafe { copy_16_bytes(dst.as_mut_ptr(), src.as_ptr()) };
        assert_eq!(dst, src);
    }

    #[test]
    fn test_copy_16_bytes_overlap() {
        // DEFLATE match copy shape (ufbx.c:2877-2886): length 5, distance 5 —
        // src = dst - 5, 16-byte ranges overlap. Must behave like the C SSE
        // branch: full load, then full store.
        let mut buf = [0u8; 32];
        for (i, b) in buf.iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut expect = buf;
        let snapshot: [u8; 16] = buf[3..19].try_into().unwrap();
        expect[8..24].copy_from_slice(&snapshot);
        unsafe {
            let p = buf.as_mut_ptr();
            copy_16_bytes(p.add(8), p.add(3));
        }
        assert_eq!(buf, expect);
    }

    // Bit-exactness of the `extra/ufbx_math.c` sin/cos/tan port against the C
    // implementation the hash oracle links (misc/run_tests.py:1542,1551).
    // Reference bits produced by compiling `extra/ufbx_math.c` with
    // `clang -O2 -ffp-contract=off` and printing `ufbx_sin`/`ufbx_cos`/`ufbx_tan`.
    // The corpus spans every branch: the |x| <= pi/4 kernels, the +-1 and
    // medium `rem_pio2` arms, the `kernel_rem_pio2` large-argument path, the
    // tiny-|x| early-outs, and the two `ufbx_euler_to_quat`
    // (ufbx.c:31566-31620) half-angles where platform libm disagrees by 1 ULP.
    #[test]
    fn test_ufbx_math_sin_cos_tan_bits() {
        // (arg bits, ufbx_sin bits, ufbx_cos bits, ufbx_tan bits)
        const CASES: &[(u64, u64, u64, u64)] = &[
            (0x0000000000000000, 0x0000000000000000, 0x3ff0000000000000, 0x0000000000000000),
            (0x8000000000000000, 0x8000000000000000, 0x3ff0000000000000, 0x8000000000000000),
            (0x39b4484bfeebc2a0, 0x39b4484bfeebc2a0, 0x3ff0000000000000, 0x39b4484bfeebc2a0),
            (0xb9b4484bfeebc2a0, 0xb9b4484bfeebc2a0, 0x3ff0000000000000, 0xb9b4484bfeebc2a0),
            (0x3fb999999999999a, 0x3fb98eaecb8bcb2c, 0x3fefd712f9a817c0, 0x3fb9af8877430b80),
            (0xbfb999999999999a, 0xbfb98eaecb8bcb2c, 0x3fefd712f9a817c0, 0xbfb9af8877430b80),
            (0x3fe0000000000000, 0x3fdeaee8744b05f0, 0x3fec1528065b7d50, 0x3fe17b4f5bf3474a),
            (0x3fe8f5c28f5c28f6, 0x3fe68143d72d4ce4, 0x3fe6bfcdbf817bfa, 0x3fefa807cf826c45),
            (0x3fe921fb54442d18, 0x3fe6a09e667f3bcc, 0x3fe6a09e667f3bcd, 0x3fefffffffffffff),
            (0x3fe999999999999a, 0x3fe6f494c2bffecd, 0x3fe64b6bde719865, 0x3ff079664793b60b),
            (0x3ff0000000000000, 0x3feaed548f090cee, 0x3fe14a280fb5068c, 0x3ff8eb245cbee3a6),
            (0x3ff921fb54442d18, 0x3ff0000000000000, 0x3c91a62633145c07, 0x434d02967c31cdb5),
            (0x4000000000000000, 0x3fed18f6ead1b446, 0xbfdaa22657537205, 0xc0017af62e0950f8),
            (0x4008000000000000, 0x3fc210386db6d55b, 0xbfefae04be85e5d2, 0xbfc23ef71254b86f),
            (0x400921fb54442d18, 0x3ca1a62633145c07, 0xbff0000000000000, 0xbca1a62633145c07),
            (0x4010000000000000, 0xbfe837b9dddc1eae, 0xbfe4eaa606db24c1, 0x3ff2866f9be4de14),
            (0x401921fb54442d18, 0xbcb1a62633145c07, 0x3ff0000000000000, 0xbcb1a62633145c07),
            (0x4024000000000000, 0xbfe1689ef5f34f52, 0xbfead9ac890c6b1f, 0x3fe4bf5f34be3782),
            (0x4059000000000000, 0xbfe03425b78c4db8, 0x3feb981dbf665fdf, 0xbfe2ca74d62b5d38),
            (0x408f400000000000, 0x3fea75cc150a206b, 0x3fe1ff026793f1bc, 0x3ff786729f34311a),
            (0x40c81cd6e631f8a1, 0xbfe68298a1cec146, 0x3fe6be7c89fe4a8e, 0xbfefabbca285aaa3),
            (0x412e848000000000, 0xbfd6664b2568d867, 0x3fedf9df9906d32c, 0xbfd7e9768ab734c0),
            (0x4202a05f20000000, 0xbfdf334c7896a4e3, 0x3febf098901c931a, 0xbfe1de000f443f50),
            (0x430c6bf526340000, 0x3feb76f88136ceba, 0xbfe06c154609d33e, 0xbffac23600a95be4),
            (0x412921fa00000000, 0xbfe3bc41ec71c06d, 0x3fe930880dddf826, 0xbfe91238517e869a),
            (0x4376345785d8a000, 0xbfddbadc7a119fc8, 0xbfec567c5278afcb, 0x3fe0c93726adf98c),
            (0x4480f0cf064dd592, 0xbfeb453ab76bf397, 0x3fe0be2cef01c8f4, 0xbffa0f79c1b6b258),
            (0x46293e5939a08cea, 0x3f831c608f107767, 0xbfefffa4b11f1b45, 0xbf831c97177a2330),
            (0x7e37e43c8800759c, 0xbfea2c16b010e385, 0xbfe2699022adc4c1, 0x3ff6be411f37ac77),
            (0xc480f0cf064dd592, 0x3feb453ab76bf397, 0x3fe0be2cef01c8f4, 0x3ffa0f79c1b6b258),
            (0xc0c81cd6e631f8a1, 0x3fe68298a1cec146, 0x3fe6be7c89fe4a8e, 0x3fefabbca285aaa3),
            (0x4002d97c7f3321d2, 0x3fe6a09e667f3bcd, 0xbfe6a09e667f3bcc, 0xbff0000000000001),
            (0x3fe921fb54442d3a, 0x3fe6a09e667f3be4, 0x3fe6a09e667f3bb5, 0x3ff0000000000022),
            (0xbfe921fb7fffd84c, 0xbfe6a09e856bc43e, 0x3fe6a09e4792b331, 0xbff000002bbbab6f),
            (0x3fe5942800000000, 0x3fe3fae8600608f7, 0x3fe8fef3a3aa8b7e, 0x3fe99427887a14d6),
            (0x3e2fffffffffffff, 0x3e2fffffffffffff, 0x3ff0000000000000, 0x3e2fffffffffffff),
            (0xbe2fffffffffffff, 0xbe2fffffffffffff, 0x3ff0000000000000, 0xbe2fffffffffffff),
        ];
        for &(arg, s, c, t) in CASES {
            let x = f64::from_bits(arg);
            assert_eq!(math::sin(x).to_bits(), s, "sin({arg:016x})");
            assert_eq!(math::cos(x).to_bits(), c, "cos({arg:016x})");
            assert_eq!(math::tan(x).to_bits(), t, "tan({arg:016x})");
        }
        // sin/cos/tan of Inf and NaN are NaN.
        assert!(math::sin(f64::INFINITY).is_nan());
        assert!(math::cos(f64::NEG_INFINITY).is_nan());
        assert!(math::tan(f64::INFINITY).is_nan());
        assert!(math::sin(f64::NAN).is_nan());
        assert!(math::cos(f64::NAN).is_nan());
        assert!(math::tan(f64::NAN).is_nan());
    }

    #[test]
    fn test_is_aligned() {
        let p = 0x1000usize as *const u8;
        assert!(is_aligned(p, 16));
        assert!(is_aligned_mask(p, 15));
        let q = 0x1001usize as *const u8;
        assert!(!is_aligned(q, 16));
        assert!(!is_aligned_mask(q, 15));
        assert!(is_aligned(q, 1));
    }

    #[test]
    fn test_version() {
        assert_eq!(pack_version(0, 23, 0), 23000);
        assert_eq!(SOURCE_VERSION, 23000);
        #[cfg(feature = "c-abi")]
        assert_eq!(crate::capi::ufbx_source_version, SOURCE_VERSION);
    }

    #[test]
    fn test_regression_constants() {
        if cfg!(feature = "regression") {
            assert_eq!(MAX_SKIP_SIZE, 128);
            assert_eq!(MAP_MAX_SCAN, 2);
            assert_eq!(KD_FAST_DEPTH, 2);
            assert_eq!(FACE_GROUP_HASH_BITS, 2);
            assert_eq!(MIN_THREADED_DEFLATE_BYTES, 2);
            assert_eq!(MIN_THREADED_ASCII_VALUES, 2);
            assert_eq!(clamp_linear_threshold(32), 2);
            assert!(IS_REGRESSION);
        } else {
            assert_eq!(MAX_SKIP_SIZE, 0x40000000);
            assert_eq!(MAP_MAX_SCAN, 32);
            assert_eq!(KD_FAST_DEPTH, 6);
            assert_eq!(FACE_GROUP_HASH_BITS, 8);
            assert_eq!(MIN_THREADED_DEFLATE_BYTES, 256);
            assert_eq!(MIN_THREADED_ASCII_VALUES, 64);
            assert_eq!(clamp_linear_threshold(32), 32);
            assert!(!IS_REGRESSION);
        }
    }

    #[test]
    fn test_math_rint_half_to_even() {
        assert_eq!(math::rint(0.5), 0.0);
        assert_eq!(math::rint(1.5), 2.0);
        assert_eq!(math::rint(2.5), 2.0);
        assert_eq!(math::rint(-0.5), 0.0);
        assert_eq!(math::rint(-1.5), -2.0);
        assert_eq!(math::rint(-2.5), -2.0);
        assert_eq!(math::rint(2.4), 2.0);
        assert_eq!(math::rint(2.6), 3.0);
    }

    #[test]
    fn test_math_fmin_fmax_nan() {
        // ufbx_math.c:1866/1871 ternary semantics (NOT libm/std fmin/fmax).
        assert_eq!(math::fmin(1.0, 2.0), 1.0);
        assert_eq!(math::fmax(1.0, 2.0), 2.0);
        // `a < b` false when either is NaN -> fmin returns b, fmax returns a.
        assert!(math::fmin(2.0, f64::NAN).is_nan());
        assert_eq!(math::fmin(f64::NAN, 2.0), 2.0);
        assert!(math::fmax(f64::NAN, 0.0).is_nan()); // ufbx.c:31882 shape
        assert_eq!(math::fmax(0.0, f64::NAN), 0.0);
    }

    #[test]
    fn test_math_nextafter() {
        assert_eq!(math::nextafter(1.0, 2.0).to_bits(), 1.0f64.to_bits() + 1);
        assert_eq!(math::nextafter(1.0, 0.0).to_bits(), 1.0f64.to_bits() - 1);
        assert_eq!(math::nextafter(1.0, 1.0), 1.0);
        assert_eq!(math::nextafter(0.0, 1.0).to_bits(), 1); // smallest subnormal
        assert_eq!(math::nextafter(0.0, -1.0).to_bits(), 0x8000_0000_0000_0001);
        assert!(math::nextafter(f64::NAN, 1.0).is_nan());
        assert!(math::nextafter(1.0, f64::NAN).is_nan());
        assert_eq!(math::nextafter(f64::MAX, f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn test_atomic_counter() {
        // ufbx.c:641-647 contract: inc/dec return the PREVIOUS value.
        unsafe {
            let mut c = core::mem::MaybeUninit::<AtomicCounter>::uninit();
            let p = c.as_mut_ptr();
            atomic_counter_init(p);
            assert_eq!(atomic_counter_load(p), 0);
            assert_eq!(atomic_counter_inc(p), 0);
            assert_eq!(atomic_counter_inc(p), 1);
            assert_eq!(atomic_counter_load(p), 2);
            assert_eq!(atomic_counter_dec(p), 2);
            // Refcount idiom (ufbx.c:30255/30273): the counter starts at 0 and
            // `if dec(...) > 0 { return }` — the object is freed when the
            // PREVIOUS value was 0.
            assert_eq!(atomic_counter_dec(p), 1);
            assert_eq!(atomic_counter_load(p), 0);
            atomic_counter_free(p);
            assert_eq!(atomic_counter_load(p), 0);
        }
        // ufbx.c:633 / 30497-30500 `ufbx_is_thread_safe` logic.
        assert!(THREAD_SAFE != 0);
    }

    #[test]
    fn test_atomic_counter_threaded() {
        // Cross-thread previous-value uniqueness: N threads x M incs must
        // observe every previous value in [0, N*M) exactly once.
        use std::sync::atomic::{AtomicBool, Ordering};
        static COUNTER: AtomicCounter = AtomicCounter::new(0);
        static SEEN: [AtomicBool; 64] = [const { AtomicBool::new(false) }; 64];
        let threads: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(|| unsafe {
                    let p = &COUNTER as *const AtomicCounter as *mut AtomicCounter;
                    for _ in 0..16 {
                        let prev = atomic_counter_inc(p);
                        assert!(!SEEN[prev].swap(true, Ordering::SeqCst));
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        unsafe {
            let p = &COUNTER as *const AtomicCounter as *mut AtomicCounter;
            assert_eq!(atomic_counter_load(p), 64);
        }
        assert!(SEEN.iter().all(|s| s.load(Ordering::SeqCst)));
    }

    #[test]
    fn test_assert_macros() {
        // Passing asserts do nothing in any configuration.
        ufbx_assert!(true);
        ufbxi_regression_assert!(true);
        ufbxi_dev_assert!(true);
        ufbxi_analysis_assert!(false); // never evaluated
        assert_eq!(ufbxi_maybe_null!(3), 3);
        assert_eq!(ufbxi_maybe_uninit!(false, 7, 9), 7);
        ufbxi_ignore!(41 + 1);
    }

    #[test]
    #[should_panic]
    fn test_unreachable_panics() {
        // Asserts are always compiled in for now (no `no-assert` feature).
        ufbxi_unreachable!("test reason");
    }
}

// Port of the sort/bounds unit tests from test/unit_tests.c (uint_pair sorts,
// bound searches, string sorts, test_sorts driver). The driver sweep is
// reduced from C (MAX_SORT_SIZE 2048, size step `1+size/128+size/512*32`,
// linear sizes 2..64 stepping `1+n/8`) to keep debug-mode `cargo test` time
// reasonable — the algorithms, comparator shapes, and checks are identical.
#[cfg(test)]
mod utility_tests {
    use super::*;
    use core::ffi::c_void;
    use core::ptr;
    use std::ffi::CString;
    use std::os::raw::c_char;

    // test/unit_tests.c:17-19 `uint_pair`
    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct UintPair {
        a: u32,
        b: u32,
    }

    // test/unit_tests.c:21-27 `sort_mode`
    #[derive(Clone, Copy, PartialEq, Debug)]
    enum SortMode {
        StableMacro,
        StableFunction,
        UnstableFunction,
    }
    const SORT_MODES: [SortMode; 3] = [
        SortMode::StableMacro,
        SortMode::StableFunction,
        SortMode::UnstableFunction,
    ];

    // test/unit_tests.c:32-36 `uint_less`
    unsafe extern "C" fn uint_less(
        _user: *mut c_void,
        va: *const c_void,
        vb: *const c_void,
    ) -> bool {
        let a = *(va as *const u32);
        let b = *(vb as *const u32);
        a < b
    }

    // test/unit_tests.c:38-42 `pair_less_a`
    unsafe extern "C" fn pair_less_a(
        _user: *mut c_void,
        va: *const c_void,
        vb: *const c_void,
    ) -> bool {
        let a = *(va as *const UintPair);
        let b = *(vb as *const UintPair);
        a.a < b.a
    }

    // test/unit_tests.c:44-48 `pair_less_b`
    unsafe extern "C" fn pair_less_b(
        _user: *mut c_void,
        va: *const c_void,
        vb: *const c_void,
    ) -> bool {
        let a = *(va as *const UintPair);
        let b = *(vb as *const UintPair);
        a.b < b.b
    }

    // test/unit_tests.c:50-54 `str_less`
    unsafe extern "C" fn str_less(
        _user: *mut c_void,
        va: *const c_void,
        vb: *const c_void,
    ) -> bool {
        let a = *(va as *const *const c_char);
        let b = *(vb as *const *const c_char);
        libc::strcmp(a, b) < 0
    }

    // test/unit_tests.c:56-72 `sort_uints`
    fn sort_uints(mode: SortMode, linear_size: usize, data: &mut [u32], tmp: &mut [u32]) {
        let size = data.len();
        unsafe {
            match mode {
                SortMode::StableMacro => macro_stable_sort(
                    linear_size,
                    data.as_mut_ptr(),
                    tmp.as_mut_ptr(),
                    size,
                    |a, b| *a < *b,
                ),
                SortMode::StableFunction => stable_sort(
                    size_of::<u32>(),
                    linear_size,
                    data.as_mut_ptr() as *mut c_void,
                    tmp.as_mut_ptr() as *mut c_void,
                    size,
                    uint_less,
                    ptr::null_mut(),
                ),
                SortMode::UnstableFunction => unstable_sort(
                    data.as_mut_ptr() as *mut c_void,
                    size,
                    size_of::<u32>(),
                    uint_less,
                    ptr::null_mut(),
                ),
            }
        }
    }

    // test/unit_tests.c:74-90 `sort_pairs_by_a`
    fn sort_pairs_by_a(
        mode: SortMode,
        linear_size: usize,
        data: &mut [UintPair],
        tmp: &mut [UintPair],
    ) {
        let size = data.len();
        unsafe {
            match mode {
                SortMode::StableMacro => macro_stable_sort(
                    linear_size,
                    data.as_mut_ptr(),
                    tmp.as_mut_ptr(),
                    size,
                    |a, b| (*a).a < (*b).a,
                ),
                SortMode::StableFunction => stable_sort(
                    size_of::<UintPair>(),
                    linear_size,
                    data.as_mut_ptr() as *mut c_void,
                    tmp.as_mut_ptr() as *mut c_void,
                    size,
                    pair_less_a,
                    ptr::null_mut(),
                ),
                SortMode::UnstableFunction => unstable_sort(
                    data.as_mut_ptr() as *mut c_void,
                    size,
                    size_of::<UintPair>(),
                    pair_less_a,
                    ptr::null_mut(),
                ),
            }
        }
    }

    // test/unit_tests.c:92-108 `sort_pairs_by_b`
    fn sort_pairs_by_b(
        mode: SortMode,
        linear_size: usize,
        data: &mut [UintPair],
        tmp: &mut [UintPair],
    ) {
        let size = data.len();
        unsafe {
            match mode {
                SortMode::StableMacro => macro_stable_sort(
                    linear_size,
                    data.as_mut_ptr(),
                    tmp.as_mut_ptr(),
                    size,
                    |a, b| (*a).b < (*b).b,
                ),
                SortMode::StableFunction => stable_sort(
                    size_of::<UintPair>(),
                    linear_size,
                    data.as_mut_ptr() as *mut c_void,
                    tmp.as_mut_ptr() as *mut c_void,
                    size,
                    pair_less_b,
                    ptr::null_mut(),
                ),
                SortMode::UnstableFunction => unstable_sort(
                    data.as_mut_ptr() as *mut c_void,
                    size,
                    size_of::<UintPair>(),
                    pair_less_b,
                    ptr::null_mut(),
                ),
            }
        }
    }

    // test/unit_tests.c:110-119 `find_uint`
    fn find_uint(linear_size: usize, data: &[u32], value: u32) -> usize {
        let mut index = usize::MAX;
        unsafe {
            macro_lower_bound_eq(
                linear_size,
                &mut index,
                data.as_ptr(),
                0,
                data.len(),
                |a| *a < value,
                |a| *a == value,
            );
        }
        index
    }

    // test/unit_tests.c:121-126 `find_uint_end`
    fn find_uint_end(linear_size: usize, data: &[u32], begin: usize, value: u32) -> usize {
        let mut index = usize::MAX;
        unsafe {
            macro_upper_bound_eq(
                linear_size,
                &mut index,
                data.as_ptr(),
                begin,
                data.len(),
                |a| *a == value,
            );
        }
        index
    }

    // test/unit_tests.c:128-135 `find_pair_by_a`
    fn find_pair_by_a(linear_size: usize, data: &[UintPair], value: u32) -> usize {
        let mut pair_ix = usize::MAX;
        unsafe {
            macro_lower_bound_eq(
                linear_size,
                &mut pair_ix,
                data.as_ptr(),
                0,
                data.len(),
                |a| (*a).a < value,
                |a| (*a).a == value,
            );
        }
        pair_ix
    }

    // test/unit_tests.c:137-151 `sort_strings`
    fn sort_strings(
        mode: SortMode,
        linear_size: usize,
        data: &mut [*const c_char],
        tmp: &mut [*const c_char],
    ) {
        let size = data.len();
        unsafe {
            match mode {
                SortMode::StableMacro => macro_stable_sort(
                    linear_size,
                    data.as_mut_ptr(),
                    tmp.as_mut_ptr(),
                    size,
                    |a, b| libc::strcmp(*a, *b) < 0,
                ),
                SortMode::StableFunction => stable_sort(
                    size_of::<*const c_char>(),
                    linear_size,
                    data.as_mut_ptr() as *mut c_void,
                    tmp.as_mut_ptr() as *mut c_void,
                    size,
                    str_less,
                    ptr::null_mut(),
                ),
                SortMode::UnstableFunction => unstable_sort(
                    data.as_mut_ptr() as *mut c_void,
                    size,
                    size_of::<*const c_char>(),
                    str_less,
                    ptr::null_mut(),
                ),
            }
        }
    }

    // test/unit_tests.c:153-162 `find_first_string`
    fn find_first_string(linear_size: usize, data: &[*const c_char], s: &CString) -> usize {
        let mut str_index = usize::MAX;
        let sp = s.as_ptr();
        unsafe {
            macro_lower_bound_eq(
                linear_size,
                &mut str_index,
                data.as_ptr(),
                0,
                data.len(),
                |a| libc::strcmp(*a, sp) < 0,
                |a| libc::strcmp(*a, sp) == 0,
            );
        }
        str_index
    }

    // test/unit_tests.c:163-171 `xorshift32`
    fn xorshift32(state: &mut u32) -> u32 {
        // Algorithm "xor" from p. 4 of Marsaglia, "Xorshift RNGs"
        let mut x = *state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *state = x;
        x
    }

    // test/unit_tests.c:179-185 `generate_linear`
    fn generate_linear(dst: &mut [u32], mut start: u32, delta: u32) {
        for d in dst.iter_mut() {
            *d = start;
            start = start.wrapping_add(delta); // C unsigned wrap
        }
    }

    // test/unit_tests.c:187-193 `generate_random`
    fn generate_random(dst: &mut [u32], seed: u32, m: u32) {
        let mut state = seed | 1;
        for d in dst.iter_mut() {
            *d = xorshift32(&mut state) % m;
        }
    }

    // test/unit_tests.c:197-248 `test_sort`
    fn test_sort_case(mode: SortMode, linear_size: usize, data: &mut [u32]) {
        let size = data.len();
        let mut uint_tmp_buffer = vec![0u32; size];
        let mut pair_buffer = vec![UintPair { a: 0, b: 0 }; size];
        let mut pair_tmp_buffer = vec![UintPair { a: 0, b: 0 }; size];

        for i in 0..size {
            pair_buffer[i].a = data[i];
            pair_buffer[i].b = i as u32;
        }

        sort_uints(mode, linear_size, data, &mut uint_tmp_buffer);
        sort_pairs_by_a(mode, linear_size, &mut pair_buffer, &mut pair_tmp_buffer);

        for i in 1..size {
            assert!(data[i - 1] <= data[i]);
            assert!(pair_buffer[i - 1].a <= pair_buffer[i].a);
            if pair_buffer[i - 1].a == pair_buffer[i].a {
                if mode != SortMode::UnstableFunction {
                    assert!(pair_buffer[i - 1].b < pair_buffer[i].b);
                }
            }
        }

        for i in 0..size {
            let value = data[i];

            let index = find_uint(linear_size, data, value);
            assert!(index <= i);
            assert!(data[index] == value);
            assert!(index == find_pair_by_a(linear_size, &pair_buffer, value));
            if index > 0 {
                assert!(data[index - 1] < value);
            }

            let end = find_uint_end(linear_size, data, index, value);
            assert!(end > i);
            assert!(data[end - 1] == value);
            if end < size {
                assert!(data[end] > value);
            }
        }
        // Miss: lower_bound_eq must NOT write the pre-initialized SIZE_MAX.
        assert!(find_uint(linear_size, data, u32::MAX) == usize::MAX);

        sort_pairs_by_b(mode, linear_size, &mut pair_buffer, &mut pair_tmp_buffer);
        for i in 0..size {
            assert!(pair_buffer[i].b == i as u32);
        }
    }

    // test/unit_tests.c:250-285 `test_sort_strings`
    fn test_sort_strings_case(mode: SortMode, linear_size: usize, data: &[u32]) {
        let size = data.len();
        let strings: Vec<CString> = data
            .iter()
            .map(|v| CString::new(v.to_string()).unwrap())
            .collect();
        let mut str_buffer: Vec<*const c_char> = strings.iter().map(|s| s.as_ptr()).collect();
        let mut str_tmp_buffer: Vec<*const c_char> = vec![ptr::null(); size];

        sort_strings(mode, linear_size, &mut str_buffer, &mut str_tmp_buffer);

        for i in 1..size {
            unsafe {
                assert!(libc::strcmp(str_buffer[i - 1], str_buffer[i]) <= 0);
            }
        }

        for v in data.iter() {
            let find_str = CString::new(v.to_string()).unwrap();
            let index = find_first_string(linear_size, &str_buffer, &find_str);
            assert!(index < size);
            unsafe {
                assert!(libc::strcmp(str_buffer[index], find_str.as_ptr()) == 0);
                if index > 0 {
                    assert!(libc::strcmp(str_buffer[index - 1], find_str.as_ptr()) < 0);
                }
            }
        }
    }

    // test/unit_tests.c:884-922 `test_sorts` (reduced sweep — see module comment)
    #[test]
    fn test_sorts() {
        const MAX_SORT_SIZE: usize = 512;
        let mut sort_buffer = vec![0u32; MAX_SORT_SIZE];

        for &mode in &SORT_MODES {
            for &linear_size in &[2usize, 3, 4, 8, 12, 16, 32, 64] {
                let mut size = 0usize;
                while size < MAX_SORT_SIZE {
                    let buf = &mut sort_buffer[..size];
                    generate_linear(buf, 0, 1);
                    test_sort_case(mode, linear_size, buf);
                    generate_linear(buf, size as u32, 1u32.wrapping_neg());
                    test_sort_case(mode, linear_size, buf);
                    generate_random(buf, size as u32, 1 + (size % 10) as u32);
                    test_sort_case(mode, linear_size, buf);
                    generate_random(buf, size as u32, u32::MAX);
                    test_sort_case(mode, linear_size, buf);
                    size += 1 + size / 16;
                }

                {
                    let size = MAX_SORT_SIZE;
                    let buf = &mut sort_buffer[..size];
                    generate_linear(buf, 0, 1);
                    test_sort_strings_case(mode, linear_size, buf);
                    generate_linear(buf, size as u32, 1u32.wrapping_neg());
                    test_sort_strings_case(mode, linear_size, buf);
                    generate_random(buf, size as u32, 1 + (size % 10) as u32);
                    test_sort_strings_case(mode, linear_size, buf);
                    generate_random(buf, size as u32, u32::MAX);
                    test_sort_strings_case(mode, linear_size, buf);
                }
            }
        }
    }

    #[test]
    fn test_bound_write_contracts() {
        // PORTING.md "Sorting & searching": lower_bound_eq does NOT write on
        // miss; upper_bound_eq ALWAYS writes.
        let data: [u32; 4] = [1, 3, 3, 7];
        let mut index = 12345usize;
        unsafe {
            macro_lower_bound_eq(2, &mut index, data.as_ptr(), 0, 4, |a| *a < 5, |a| *a == 5);
        }
        assert_eq!(index, 12345); // untouched on miss
        let mut end = 12345usize;
        unsafe {
            macro_upper_bound_eq(2, &mut end, data.as_ptr(), 0, 4, |a| *a == 5);
        }
        assert_eq!(end, 0); // always written
        let mut end2 = 12345usize;
        unsafe {
            macro_upper_bound_eq(2, &mut end2, data.as_ptr(), 1, 4, |a| *a == 3);
        }
        assert_eq!(end2, 3);
    }

    #[test]
    fn test_stable_sort_algorithms_differ_but_agree() {
        // The two stable sorts are different algorithms with different
        // comparator call counts; results must agree element-for-element.
        let mut state = 0xC0FFEEu32;
        for &size in &[0usize, 1, 2, 3, 7, 64, 200] {
            let base: Vec<UintPair> = (0..size)
                .map(|i| UintPair {
                    a: xorshift32(&mut state) % 8,
                    b: i as u32,
                })
                .collect();
            let mut m = base.clone();
            let mut f = base.clone();
            let mut tmp = vec![UintPair { a: 0, b: 0 }; size];
            sort_pairs_by_a(SortMode::StableMacro, 4, &mut m, &mut tmp);
            sort_pairs_by_a(SortMode::StableFunction, 4, &mut f, &mut tmp);
            assert_eq!(m, f);
        }
    }

    #[test]
    fn test_min_max_helpers() {
        assert_eq!(min32(1, 2), 1);
        assert_eq!(max32(1, 2), 2);
        assert_eq!(min64(u64::MAX, 0), 0);
        assert_eq!(max64(u64::MAX, 0), u64::MAX);
        assert_eq!(min_sz(5, 3), 3);
        assert_eq!(max_sz(5, 3), 5);
        assert_eq!(min_real(1.0, 2.0), 1.0);
        assert_eq!(max_real(1.0, 2.0), 2.0);
        // NaN semantics: `a < b ? ...` ternary verbatim (trap #6).
        assert!(min_real(crate::prelude::Real::NAN, 2.0) == 2.0);
        assert!(max_real(crate::prelude::Real::NAN, 2.0).is_nan());
    }

    #[test]
    fn test_f64_to_int_clamps() {
        assert_eq!(f64_to_i32(0.0), 0);
        assert_eq!(f64_to_i32(-1.75), -1); // C cast truncates toward zero
        assert_eq!(f64_to_i32(1.75), 1);
        assert_eq!(f64_to_i32(2147483647.0), i32::MAX);
        assert_eq!(f64_to_i32(1e300), i32::MAX);
        assert_eq!(f64_to_i32(-1e300), i32::MIN);
        assert_eq!(f64_to_i32(f64::INFINITY), i32::MAX);
        assert_eq!(f64_to_i32(f64::NEG_INFINITY), i32::MIN);
        // NaN: fabs(NaN) <= MAX is false; NaN >= 0.0 is false -> MIN (C shape).
        assert_eq!(f64_to_i32(f64::NAN), i32::MIN);

        assert_eq!(f64_to_i64(0.0), 0);
        assert_eq!(f64_to_i64(-1.75), -1);
        assert_eq!(f64_to_i64(1e300), i64::MAX);
        assert_eq!(f64_to_i64(-1e300), i64::MIN);
        assert_eq!(f64_to_i64(f64::NAN), i64::MIN);
        assert_eq!(f64_to_i64(9007199254740993.0), 9007199254740992);
    }

    #[test]
    fn test_to_size_and_ptr_helpers() {
        assert_eq!(to_size(42), 42usize);
        assert_eq!(to_size(0), 0usize);
        #[cfg(not(feature = "regression"))]
        assert_eq!(to_size(-1), usize::MAX); // release: bare (size_t) cast
        let buf = [0u8; 8];
        let p = buf.as_ptr() as *mut u8;
        assert_eq!(add_ptr(p, 3), unsafe { p.add(3) });
        assert_eq!(sub_ptr(add_ptr(p, 3), 3), p);
        // NULL + 0 tolerated (the reason C has the UBSAN branch).
        assert!(add_ptr(ptr::null_mut::<u8>(), 0).is_null());
        assert!(sub_ptr(ptr::null_mut::<u8>(), 0).is_null());
    }
}
