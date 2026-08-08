# PORTING.md — C→Rust pattern map for the ufbx native port

Rules for the semi-mechanical 1:1 port of `ufbx.c` into `rust/ufbx/src/native/`.
The prime directive: **translate, don't improve.** Every deviation from the C
structure is a place where upstream diffs stop applying and where behavior can
silently diverge. Idiomatic cleanup happens in the unsafe-reduction phase, under
green tests — never during the port.

This document was adversarially reviewed against ufbx.c before porting began;
line references are to the current upstream state and are re-anchored on sync.

## Ground rules

0. **NEVER hand-edit `rust/ufbx/src/generated.rs` or `rust/ufbx/tests/layout.rs`**
   — both are produced by `./rust/regen.sh` and silently overwritten on every
   upstream sync (CI enforces diff-cleanliness). A wrong type/signature in
   generated code is a bug in `rust/ufbx/bindgen/generate_rust.py`: fix the
   generator, re-run regen.sh, and leave a NOTE(ufbx-rs-native) comment there
   (precedents: the uint8_t→u8 field fix; enum-return callbacks emitted as u32).

1. **One C function → one Rust function, same name minus prefix (see Naming),
   same argument order, same control flow.** Keep C comments.
2. **Do not reorder fields, computations, allocations, or error checks.**
   Allocation *sequence* is observable (allocation-limit fuzzing); float
   *operation order* is observable (scene hashes).
3. **Public types come from `generated.rs`; container primitives (`String`,
   `List<T>`, `RefList<T>`, `Blob`, `Ref<T>`) live in `prelude.rs`** (re-exported
   from both via `lib.rs`) — never redefine either. Internal `ufbxi_*` structs
   are hand-ported `#[repr(C)]` in the module that owns them.
4. When the C is UB-adjacent but works (aliasing, unaligned reads), port it with
   raw pointers and an explicit `// C-parity:` comment rather than "fixing" it.
5. **Every ported function carries a `// ufbx.c:NNNN-MMMM` anchor comment** —
   diff-only review and upstream-diff reapplication both depend on it.

## Naming

| C | Rust |
|---|---|
| `ufbxi_foo_bar` (internal fn) | `pub(crate) fn foo_bar` in the owning module |
| `ufbxi_foo_bar` (internal type — C uses snake_case) | `pub(crate) struct FooBar` (`#[repr(C)]`) — case conversion. NOTE: `ufbxi_buf`/`ufbxi_buf_*` and `ufbxi_thread_pool`/`ufbxi_thread_pool_*` are type+function families sharing a stem; the case split (type `Buf`, fns `buf_*`) resolves the collision. |
| `ufbx_foo` (public fn) | native impl `native::api::foo`; `#[no_mangle] pub extern "C" fn ufbx_foo` shim in `capi.rs` |
| `UFBXI_CONSTANT` | `pub(crate) const CONSTANT` in owning module |
| local `ufbx_string`/lists | `prelude.rs` types (`String`, `List<T>`, …) |

Function→module routing: each module header lists the C banner section it owns;
`git diff` hunks name the enclosing C function (see `native/mod.rs` order).

**Not present in ufbx.c** (verified; no mapping rules needed): `goto` (0 uses),
`setjmp`/`longjmp` (0), `alloca`/VLAs (0), bitfields (0 — bit packing is manual
shifts/masks, e.g. `value_type_mask` ufbx.c:6206).

## Core idiom mappings

### Error threading (`ufbxi_check` family) — FULL table

Internal fallible fns return `Result<T, Fail>` (`Fail` zero-sized; the actual
`ufbx_error` lives in the context, as in C). `?` replaces
`ufbxi_check(ufbxi_inner(...))` chains.

The complete macro family (ufbx.c:3550-3557 error-target forms, 6664-6671
context forms). ALL of these must exist as Rust macros with matching semantics:

| C macro (line) | Behavior |
|---|---|
| `ufbxi_check_err(err, cond)` 3550 | fail+return 0 on !cond |
| `ufbxi_check_return_err(err, cond, ret)` 3551 | fail+return `ret` |
| `ufbxi_fail_err(err, desc)` 3552 | unconditional fail |
| `ufbxi_check_err_msg(err, cond, msg)` 3554 | fail with `"$msg\0<stringified cond>"` |
| `ufbxi_check_return_err_msg(err, cond, ret, msg)` 3555 | ditto, return `ret` |
| `ufbxi_fail_err_msg(err, desc, msg)` 3556 | unconditional, **desc is a verbatim literal, NOT stringified** |
| `ufbxi_report_err_msg(err, desc, msg)` 3557 | **records the error and KEEPS GOING** (`(void)` result; used e.g. ufbx.c:24776). NOT a return. |
| `ufbxi_check(cond)` 6664 | uc-context form of check_err |
| `ufbxi_check_return(cond, ret)` 6665 | |
| `ufbxi_fail(desc)` 6666 | |
| `ufbxi_fail_return(desc, ret)` 6667 | |
| `ufbxi_check_msg(cond, msg)` 6669 | |
| `ufbxi_check_return_msg(cond, ret, msg)` 6670 | |
| `ufbxi_fail_msg(desc, msg)` 6671 | desc verbatim (e.g. `ufbxi_fail_msg("UFBXI_FEATURE_FORMAT_OBJ", "Feature disabled")` ufbx.c:18052) |
| `ufbxi_check_opts_ptr` / `ufbxi_check_opts_return` / `ufbxi_check_opts_return_no_error` (30313-30328) | public-entry-point options-validation guards |

Rules:
- **The condition is evaluated EXACTLY ONCE.** C wraps it in
  `ufbxi_trace(cond)` (ufbx.c:593-597), a comma expression evaluating `cond`
  once (optionally logging under `UFBX_TRACE`). In `macro_rules!`, bind the
  condition to a local before testing; `stringify!` the token tree separately.
  Double expansion double-executes every `ufbxi_check(ufbxi_push_...(...))`
  call — the highest-probability macro bug in this port.
- `_check*_msg` stringifies the condition via `ufbxi_cond_str`; `_fail*_msg`
  takes the description literal verbatim. Do not mix these up — it changes
  `ufbx_error_frame.description` bytes.
- **First error wins**: `ufbxi_fail_imp_err` (ufbx.c:3453-3486) strips the `'$'`
  prefix, pushes the stack frame, and sets `description` only if unset. It does
  NOT resolve the error type.
- **`ufbxi_fix_error_type` (ufbx.c:3559-3614)** does the strcmp ladder, called
  from ~10 top-level entry points; it also substitutes per-entry-point DEFAULT
  descriptions ("Failed to load", "Failed to evaluate", …) when none was set.
  These literals are part of byte-exact error parity.
- Record Rust `line!()`/function into `ufbx_error_frame` (values differ from C;
  fuzz table is regenerated per-build, but the mechanism must exist).

### Asserts (three distinct gates — do NOT collapse)

| C (line) | Gate | Rust |
|---|---|---|
| `ufbx_assert` (ufbx.h:102-109) | off under `UFBX_NO_ASSERT` **or `UFBX_NO_LIBC`** | `assert!` behind `#[cfg(not(feature = "no-assert"))]`-equivalent |
| `ufbxi_regression_assert` (ufbx.c:1023+, 25 uses) | `UFBX_REGRESSION` | `#[cfg(feature = "regression")] assert!` |
| `ufbxi_dev_assert` (38 uses) | `UFBX_REGRESSION \|\| UFBX_DEV \|\| UFBX_UBSAN` | `#[cfg(any(feature = "regression", feature = "dev"))] assert!` |
| `ufbxi_unreachable(reason)` | expands to `ufbx_assert(0 && reason)` | `assert!(false, ...)` under the same gate — **NOT** `unreachable!()`/`unreachable_unchecked` (C keeps executing past it when asserts are off) |

Verified: all 179 `ufbx_assert` / 38 dev / 25 regression sites are currently
side-effect-free. Re-verify only for NEW asserts arriving in upstream syncs.

### Allocator + `ufbxi_buf`

Ported byte-for-byte semantics; raw pointers throughout in the 1:1 phase.
- `ufbxi_alloc(ator, type, n)` → `alloc::<T>(ator, n) -> *mut T` (same size/limit
  accounting, same zeroing rules).
- Huge-allocation machinery: `UFBXI_HUGE_MAX_SCAN` (ufbx.c:57, value 16) bounds
  the largest-chunk scan (3921, 4326); `ator->huge_size` (set at ufbx.c:6950,
  default `0x100000`, from `opts->huge_threshold`) is the per-allocator
  threshold. Two different mechanisms — keep both.
- `ufbxi_buf` push/pop/`make_array` keep chunk geometry identical: same growth
  doubling, same alignment rounding. Any change shows up as allocation-count
  drift in the fuzz sweep.
- `ufbxi_buf_chunk` ends with a C flexible array member (`char data[];`,
  ufbx.c:3848): Rust struct holds the header only; data is reached by pointer
  arithmetic from the header end. `size_of::<BufChunk>()` == C header size
  (layout-pinned by const assert).
- The `ufbxi_refcount` header trick (`ufbxi_get_imp` = pointer minus
  `size_of::<Refcount>()`) is ported as-is; layout pinned by a const assert
  mirroring C's static assert.
- **`ufbxi_release_ref` free-order, port VERBATIM** (use-after-free otherwise):
  ```c
  ufbxi_allocator ator = refcount->ator;
  ufbxi_buf buf = refcount->buf;
  buf.ator = &ator;          // re-point to the STACK copy before freeing
  ufbxi_buf_free(&buf);
  ufbxi_free_ator(&ator);
  ```
  The parent-chain walk is an **iterative loop** (`refcount = parent`), not
  recursion — deliberate (deep ownership chains); keep it iterative.

### Atomics / refcount

`ufbxi_atomic_counter_{inc,dec}` (ufbx.c:645-646) return the **previous** value
and are sequentially consistent (`__sync_fetch_and_add/sub`; the MSVC branch
compensates ±1 because `_InterlockedIncrement64` returns the new value). Port as
`AtomicUsize::fetch_add/fetch_sub(1, Ordering::SeqCst)` — do not "optimize" to
`Relaxed`. `ufbxi_atomic_counter_load` is `fetch_add(0)` (ufbx.c:647), port as
`fetch_add(0, SeqCst)`. Use sites depend on the previous-value convention:
the counter starts at 0 (`ufbxi_init_ref`, ufbx.c:30255, no self-retain), and
`ufbxi_release_ref` (ufbx.c:30273) does `if dec(...) > 0 { return }` — the
object is freed when the previous value was 0.

### Integer semantics (TRAP-DENSE — see checklist)

| C | Rust |
|---|---|
| unsigned arithmetic (wraps) | `wrapping_*` — bare ops panic in debug builds; a debug panic is a bug in the port. Canonical wrap sites: `value + (((size_t)0 - value) & mask)` (3626) → `0usize.wrapping_sub`; `(int64_t)(0 - abs_val)` (1827) → `(0u64.wrapping_sub(abs_val)) as i64`; string hashes `hash * seed` / `hash *= 0x7feb352d` (4717-4726) → `wrapping_mul` (and the duplicate `ufbxi_hash_string_check_ascii` must match bit-for-bit); ring-buffer index `(ix - suffix_len) & wrap_mask` (7366-7371). |
| `x >> n` where `n` may be ≥ 64 | C: `ufbxi_wrap_shr64` (ufbx.c:907-912; 10 uses, all in the DEFLATE fast path). The portable C branch masks `& 63`; the fast branch relies on x86/ARM implicit masking. **Rust: always `x >> (n & 63)`.** There is NO wrapping-left-shift helper; every `<<` in ufbx.c has statically bounded amounts — port as plain `<<`. |
| `(int)size_t` / narrowing casts | `as` casts (truncating, C-equivalent) — do NOT use `try_into`. Exception: `ufbxi_to_size` (1129-1136) becomes an asserting function under `UFBX_REGRESSION`. |
| **f64→i64 boundary** (`ufbxi_f64_to_i64`, 1121-1128) | At `value == +2^63` the C guard admits the value (`(double)INT64_MAX` rounds to 2^63) and the `(int64_t)` cast is UB with target-split results: x86-64 `cvttsd2si` → INT64_MIN, AArch64 `fcvtzs` → INT64_MAX. No portable port matches both C builds. **Decision: Rust `as` (saturating, = AArch64 behavior) on all targets.** Known, accepted divergence vs an x86-64-built C oracle for exactly this one input. `ufbxi_f64_to_i32` has no such boundary (`(double)INT32_MAX` is exact; 2^31 fails the guard). |
| **bare `(uint8_t)`/`(int32_t)`/`(int64_t)` on a FLOAT operand** (i.e. NOT via `ufbxi_f64_to_i32`/`_i64`) | Out-of-range float→integer conversion is UB in C; clang emits a `cvttsd2si`/`fcvtzs` + narrow, so an x86-64 oracle **truncates modulo 2^N** where Rust `as` **saturates**. **Decision: plain `as` (saturating) everywhere** — same accepted-divergence class as the f64→i64 row, and the port is pinned to the oracle targets. Do NOT hand-roll a truncating helper. Known sites: `ufbxi_binary_convert_array` (ufbx.c:8709-8710, via the `ufbxi_cast_*` appliers in `parse_binary.rs`) and the `UFBXI_ASCII_FLOAT` `case 'c'` in `ufbxi_ascii_parse_node` (ufbx.c:10502, reachable via `Thumbnail`/`ImageData` whose `info->type` is `'c'`, ufbx.c:8105). Neighbouring `case 'i'`/`case 'l'` arms route through `ufbxi_f64_to_i32`/`_i64` and must keep doing so. Integer→integer narrowing casts stay exact and are unaffected. |
| `if (ptr)` / `if (n)` truthiness | explicit `!ptr.is_null()` / `n != 0` |
| `char` (storage) | `u8` everywhere; the C `(uint8_t)` casts at use sites become no-ops, but the **arithmetic still needs widening** (see promotion trap). |
| **`char` (value, where signedness is observable)** | C `char` has implementation-defined signedness, and a bare `*val` on a `const char *` therefore feeds *signed* bytes into arithmetic on the oracle targets (x86-64 SysV, Apple AArch64: `char` = `signed char`). Where the converted value is observable, port the *dereference* as `*(p as *const i8)` — **the `u8` storage rule above does NOT apply to these reads**, and "fixing" them back to `u8` changes every byte ≥ 0x80. Known sites: `ufbxi_binary_convert_array`'s `case 'c'` source arms (ufbx.c:8714, 8726, 8738, 8749) and `ufbxi_convert_parse_switch`'s `case 'C'/'B'` (ufbx.c:8853) — a 0xFF byte converts to −1, not 255. Signedness is **hardcoded signed, not `cfg`-guarded**: on targets whose ABI makes C `char` unsigned (e.g. `aarch64-unknown-linux-gnu`, most ARM/PowerPC Linux ABIs) the port diverges from a C build on that same target. Same accepted-divergence class as the f64→i64 boundary row: the port is pinned to the oracle targets. Adding new `*(… as *const i8)` sites requires a C `char` dereference to justify it. |
| `--x` / `x++` in expressions, `a = b = c` | decompose into statements preserving evaluation order |

### Byte order and unaligned access

`ufbxi_read_u16/u32/u64/f32/f64` (ufbx.c:791-830) → `ptr::read_unaligned` +
`from_le_bytes`. The `ufbxi_unaligned_*` / `ufbxi_aliasing_u32` typedef
apparatus (763-790) has no Rust analogue and collapses away — do NOT reproduce
the three-way cfg fork, and NEVER use `ptr::read` (aligned) on parse-buffer
pointers. The big-endian path (`ufbxi_swap_endian*`, ufbx.c:8609-8670) is
reachable on BE targets and test-exercised — port it. `ufbxi_swap`'s scratch
union (1305-1311) is an alignment device → `#[repr(C, align(8))] struct { data: [u8; 256] }`.

### Floats (hash-oracle-critical)

- Same operation, same order, same width. No `mul_add` (FMA), no re-association,
  no strength reduction. C oracle is built `-ffp-contract=off`.
- NaN: `!(a < b)` ≠ `a >= b`. Port the exact C expression.
- **The math shim list is exactly ufbx.c:257-276**: sqrt, sin, cos, tan, asin,
  acos, atan, atan2, pow, fmin, fmax, fabs, copysign, nextafter, **rint**,
  floor, ceil, isnan. Route ALL of them through the port of `extra/ufbx_math.c`
  (bit-exact across platforms) — never platform libm. **`ufbx_rint` ≠
  `f64::round`** (rint = half-to-even; round = half-away-from-zero); rint is on
  the keyframe-time quantization path (ufbx.c:26003-26004) — straight into the
  hash oracle.
- **The strtod runtime probe (ufbx.c:1800-1810) is `volatile`-load-bearing**:
  `ufbxi_parse_double_init_flags` reads a `static volatile double` to detect
  x87 excess precision and selects between two strtod paths, gated on
  `UFBX_FLT_EVAL_METHOD` (derived ufbx.c:358-368). Port the read as
  `ptr::read_volatile` (a plain static read const-folds and silently pins one
  path). Rust targets have no x87 excess precision, but the probe must return
  the same flag value the C build would on the same target.
- `(float)double` casts and `Real` width follow the C source text exactly.
- C ternaries on floats keep operand evaluation order.

### Printf and variadics

ufbx.c ships its own printf (banner `// -- Printf`, ufbx.c:3280-3382) with a
restricted format subset; `ufbxi_vprint` calls `ufbxi_unreachable("Bad printf
format")` on anything else. **Port `ufbxi_vprint`/`ufbxi_vsnprintf` byte-for-
byte**; variadic entry points (`ufbxi_snprintf` 3375, `ufbxi_panicf_imp` 3384,
`ufbxi_fmt_err_info` 3510, `ufbxi_warnf_imp` 4874 via the `ufbxi_warnf*`
macros) become `&[PrintArg]` slices built by `macro_rules!` wrappers at call
sites. **Never substitute `format!`/`write!`** — truncation semantics and the
returned length feed `ufbx_error.info_length` (3516, 4854) and geometry-cache
frame filenames (`"Frame%uTick%u.%s"`, ufbx.c:24521-24523 — divergence changes
which external files are opened).

### Callbacks (public function pointers)

Internal function-pointer typedefs are just `ufbxi_less_fn` (ufbx.c:1231) and
`ufbxi_mat_transform_fn` (19374) → `extern "C" fn` pointers (C passes function
designators like `&ufbxi_uv_set_less` — fn pointers, never closures).
The public callback structs (`ufbx_open_file_cb`, `ufbx_close_memory_cb`,
`ufbx_progress_cb`, allocator callbacks, the 4-function thread pool) are
`{ fn, user }` pairs invoked from deep inside the port:
- They are `extern "C"`; unwinding from user code aborts (Rust ≥1.81 semantics)
  — matches the C contract (no unwinding expected).
- `ufbx_progress_cb` is invoked mid-parse; its return drives
  `ufbxi_check_msg(result != UFBX_PROGRESS_CANCEL, "Cancelled")` (ufbx.c:6700)
  — cancellation ordering is fuzz-table-observable.

### Sorting & searching (two implementations, NOT interchangeable)

- **The sorts do not allocate.** Scratch is caller-supplied: every call site
  pre-grows `uc->tmp_arr` via `ufbxi_grow_array` immediately before sorting
  (e.g. 11880/11881, 12978/12979, 18586/18587). The allocation-parity invariant
  is that PAIRED grow call — omit or merge it and the fuzz sweep drifts.
- **`ufbxi_stable_sort` (fn, ufbx.c:1233) and `ufbxi_macro_stable_sort`
  (macro, 1142) are different algorithms**: the fn's insertion pass has an
  early-out (`if (!less_fn(...)) continue;`, 1245-1248) the macro lacks; merge
  tails differ (bulk memcpy 1281-1285 vs element loops 1180-1181). Different
  comparator call counts. Port BOTH, separately; do not unify. Never substitute
  `slice::sort_by` for either.
- `ufbxi_clamp_linear_threshold` (995-997) is **2 under `UFBX_REGRESSION`**,
  identity otherwise — the 32/16/8 literals at call sites are overridden in
  regression builds.
- `ufbxi_macro_lower_bound_eq` (1188) **does not write the result on miss**
  (callers pre-initialize, e.g. `index = SIZE_MAX;` 13362); `upper_bound_eq`
  (1206) always writes. Keep the out-param shape; do NOT return `Option`.

### Unions and flexible array members

Internal unions are **untagged overlays discriminated by a sibling field —
never convert to a Rust `enum`** (changes layout and the legality of
cross-member reads):

| ufbx.c | Construct | Rust mapping |
|---|---|---|
| 975-976 | `ufbxi_bit_cast` | `f64::to_bits`/`from_bits` (C++ branch already memcpy-based — that's the semantic) |
| 1305-1311 | `ufbxi_swap` scratch union | `#[repr(C, align(8))]` byte array (alignment device) |
| 2012-2020 | deflate `ufbxi_trees`: named struct overlaid with `trees[2]` — both members read | real overlay: `[HuffTree; 2]` + accessors for index 0/1 |
| 3838-3841 | `ufbxi_buf_chunk` `{ magic; align_0 }` | aligned `usize` field |
| 6186-6189 | `ufbxi_value` `{ {f; i}; s }` — untagged, tag = `value_type_mask` in parent; **f and i are BOTH read** (see "False positive" comments 10467/10495/10542) | `#[repr(C)] union` |
| 6209-6212 | `ufbxi_node` `{ array; vals }` | `#[repr(C)] union` |
| 16531-16534 | `ufbxi_strblob` `{ str; blob }` — written as one, read as the other | `#[repr(C)] union`, layout-pinned |

Flexible array member: `char data[];` (`ufbxi_buf_chunk`, 3848) → header-only
struct + pointer arithmetic; `size_of` must equal C's header size.

### Branch hints

`ufbxi_likely`/`ufbxi_unlikely` (C: `__builtin_expect`, degrading to identity
without compiler support) → `pub(crate) fn likely/unlikely(b: bool) -> bool`
in `native::platform`: `core::hint::likely/unlikely` under
`#[cfg(feature = "nightly")]` (crate root gains
`#![cfg_attr(feature = "nightly", feature(likely_unlikely))]`), identity
otherwise — the same degradation the C performs. Keep hints at exactly the
call sites C has them (diff parity); they are optimizer-only and cannot affect
oracle output.

### Macros & feature gates

- Expression macros → `#[inline(always)] fn` when typed; else `macro_rules!`.
- Token-pasting generics (`UFBX_LIST_TYPE`, sort macros) → generics or
  `macro_rules!` mirroring actual instantiations.
- **Feature polarity: C is opt-OUT (`UFBX_NO_X`, all on by default; `UFBX_MINIMAL`
  + `UFBX_ENABLE_X` opt-in escape, ufbx.c:75-172). Cargo features are additive →
  the parity lives in `default = [...]` listing all 9 features ON**, gated with
  `#[cfg(feature = "x")]`. `error-stack` is OFF by default (C: only under
  `UFBX_DEV`, ufbx.c:104-108). `xml` and `kd` are DERIVED (177-185:
  xml ⇐ geometry-cache, kd ⇐ triangulation), not independently selectable.
  Disabled features still export their entry points returning
  `UFBX_ERROR_FEATURE_DISABLED` (as C does).
- **`regression` is a PARAMETER change, not just extra asserts**:
  `ufbxi_clamp_linear_threshold` → 2 (995-997); `UFBXI_FACE_GROUP_HASH_BITS` → 2
  (1009-1011); `UFBXI_MIN_THREADED_DEFLATE_BYTES`/`_ASCII_VALUES` → 2
  (1014-1020, also under `UFBX_EXTENSIVE_THREADING`); `ufbxi_zero_size_buffer`
  4096 vs 64 (3620-3624); `ufbxi_to_size` asserting (1129-1136);
  `UFBXI_IS_REGRESSION` runtime-visible (1027). Hardcoding release constants
  silently defeats the regression test group.

## Semantic-traps checklist (reviewers: check EVERY item against the diff)

1. **Assert side effects / gating**: three assert families with distinct cfg
   gates (see Asserts). New upstream asserts: verify side-effect-free.
2. **Wrapping**: any bare `+ - * <<` on values that can overflow → `wrapping_*`.
   Worked examples in the integer table (3626, 1827, 4717-4726, 7366-7371).
3. **Silent truncation vs panic**: C casts truncate (`as` matches); slice
   indexing panics where C pointer arithmetic reads — port pointer-style where
   C's bounds logic tolerates over-read-then-check.
4. **Bounds checks changing behavior**: an added panic path where C continued
   with garbage (then failed gracefully) changes fuzz-observable error ordering.
5. **Evaluation order**: decompose `x++`-in-expression carefully; ternary/
   short-circuit order is specified in C and must be preserved. Conditions in
   check macros evaluate exactly once.
6. **NaN comparisons**: port `!(a < b)` verbatim.
7. **Float contraction/reassociation**: none. No `mul_add`. `ufbx_rint` ≠
   `round`. All libm via the ufbx_math port.
8. **Integer promotion**: C computes `u8`/`u16` arithmetic in `int`. Real
   example: `(uint32_t)(c + (10 - 'a'))` (ufbx.c:1838) is `c - 87` computed in
   `int` — port as `(c as i32 + (10 - b'a' as i32)) as u32`, NOT `u8` math.
   Counter-example: byte-assembly reads (798-822) write promotions explicitly
   and port mechanically.
9. **`%` sign**: C and Rust agree (truncated); still verify negative operands.
10. **Uninitialized memory**: C partial-init sites are marked `// ufbxi_uninit`
    upstream (86 sites) — preserve the marker verbatim in the Rust port
    (grep-parity across trees); use `MaybeUninit`/zero-fill only where C
    provably never reads uninit.
11. **Non-UTF8**: internal strings are byte slices; `str::from_utf8` only at
    the safe-API boundary.
12. **Allocation-count parity**: no extra/removed/merged allocations. `Vec` is
    banned in ported paths; all memory via `ufbxi_allocator`. Includes the
    paired `ufbxi_grow_array`-before-sort calls.
13. **Error-string parity**: `$`-descriptions, stringified conditions, AND the
    `ufbxi_fix_error_type` default descriptions — byte-identical. Where Rust
    `stringify!` of the ported condition differs from C's `#cond` bytes
    (`ator->x` vs `(*ator).x`, `SIZE_MAX` vs `usize::MAX`, `ptr` vs
    `!ptr.is_null()`), pass the verbatim C condition text as the check macro's
    optional trailing string literal (arms on `ufbxi_cond_str!` /
    `ufbxi_error_msg_cond!` and the check macros in `native/error.rs`).
14. **Statics**: const tables → `static`; the mutable panic handler → atomic
    with C's synchronization assumptions; **`volatile` reads (strtod probe) →
    `ptr::read_volatile`**.
15. **Struct copy semantics**: C assignment is memcpy; derive `Clone, Copy` on
    internal `#[repr(C)]` structs so `=` stays memcpy-like.
16. **Non-returning error macro**: `ufbxi_report_err_msg` records and CONTINUES
    — never translate to an early return.
17. **Union discipline**: overlays stay unions (see table); both-member reads
    are intentional (`// False positive` comments).

## Review pipeline

Per unit: implementer (Fable 5) → 2 independent diff-only reviewers with this
file + the C source (Opus 5 default; Fable 5 for allocator/buf, strtod/bigint +
the volatile probe, DEFLATE + `ufbxi_wrap_shr64`, the sort pair, threaded ASCII,
refcount/ownership, and the check-macro definitions themselves) → fixer applies
feedback → commit.

- **Review units ≤ ~400 C lines**, split at banner boundaries.
- Diffs must carry `// ufbx.c:NNNN-MMMM` anchors (ground rule 5) so reviewers
  can locate the C side without searching.
- Reviewer disagreement → third reviewer reads both findings and the C source;
  the fixer never merges conflicting guidance on its own.
- Reviewers answer per checklist item: "does the diff violate this?"

## Validation ladder (per milestone)

1. `cargo test` unit tests (ported from `test/unit_tests.c` where white-box).
1.5. **Differential leaf fuzzing**: for context-free leaf functions (parse_uint,
   string hashes, align helpers, both stable sorts, huffman builders), link the
   C original alongside the Rust port and fuzz for bit-identical outputs
   (model: `misc/fuzz_deflate_roundtrip.c`, `misc/fuzz_strtod_parse_persist.c`).
2. Linker work-queue shrink: `clang test/runner.c rust/.../libufbx.a` —
   undefined-symbol count must only decrease (a stub also shrinks it; rung 1.5
   is what makes shrinkage meaningful).
3. Once linking: `python3 misc/run_tests.py tests --rust-lib <lib>` group-by-group.
4. `hash_scene --check build/hashes.txt` (byte-exact C oracle).
5. Repeat 3-4 under `--features regression` and `--features dev` — matching
   run_tests.py's `-DUFBX_REGRESSION=1` / `-DUFBX_DEV` build groups (regression
   changes algorithm constants; a green default run proves nothing about it).
