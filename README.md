# ufbx-rs-native

Native Rust port of [ufbx](https://github.com/ufbx/ufbx).

## What is this?

This is a (LLM-assisted) mechanical port of `ufbx` (the “single source file FBX file loader”) from C to Rust. 

### Primary goals

- Produce verifiably byte-identical output, by reusing the existing C tests;
- Support the full feature-set of the original C library;
- Remain structurally/architecturally a near-perfect 1:1 match to it, to allow continuous long-term porting of all upstream changes from C to Rust;
- Act as a drop-in pure-Rust replacement for [ufbx-rust](https://github.com/ufbx/ufbx-rust) that requires no FFI, C/C++ compilation, etc.

### Secondary goals

- Reduce the amount of `unsafe` code to a minimum, to maximize security and enable loading of FBX assets even from untrusted sources; (e.g. asset stores, user-generated content)
- Allow detection of undefined behavior via [Miri](https://github.com/rust-lang/miri).

### Non-goals

- Be idiomatic or “nice-looking” Rust code;
- Implement additional features or bug-fixes on top of the upstream C project, except where strictly needed for Rust interop or port correctness.

### Repo Structure

```
(root)
│   ... largely kept the exact same layout as the upstream C project ...
│
├── rust/                    — Rust port “sidecar”
│   │
│   ├── tools/               — Misc tools (e.g. line reference update script)
│   └── ufbx/                — Main rust crate
│       │
│       ├── bindgen/         — Binding generation scripts
│       ├── src/
│       │   ├── native/      — Mechanically ported source files
│       │   └── capi.rs      — extern "C" API consumed by tests
│       └── tests/           — Rust tests
│
├── PORTING.md               — Porting idioms/instructions (LLM-generated/targeted)
├── COMPAT.md                — Registry of deviations from ufbx-rust/C, with rationale (LLM-generated)
├── UPSTREAM.md              — “Keeping up with upstream” instructions (LLM-generated/targeted)
└── README-C.md              — Original repo readme
```

## Porting Approach

We used LLMs to:

- Analyze the `ufbx-rust` crate to identify the Rust public API shape to match;
- Analyze the original upstream C code, to identify common patterns and potential porting pitfalls/traps;
- Prepare a [list of idioms](PORTING.md) with clear mappings from C to Rust, accounting for said pitfalls/traps;
- Vendor and adapt `ufbx-rust`'s Python script for generating Rust IR types, consuming the existing upstream IR generation script's data;
- Progressively and mechanically translate every function/type in the original C source to Rust, under a new “sidecar” directory:
  - Since in Rust the compilation unit is the crate, we split the original sections of the single `ufbx.c` file into various Rust files to allow for parallelism in this step;
  - References to the original C source lines are kept as comments, a specialized python script was also produced to keep those references up-to-date as upstream changes are merged in;
  - An adversarial structure (1 implementer agent → 2 reviewer agents → 1 fixer agent) was used for each ported segment;
  - An LLM was also used to high-level orchestrate this mechanical porting step.
- Produce [`extern "C"` bindings](rust/ufbx/src/capi.rs) to be consumed by the existing C public API “black-box” tests; (“white-box” C tests were ported to Rust along with the source files)
- Apply various required fixes until all compile-time checks and all tests are passing, and the library is verified to produce byte-identical output;
- Verify suitability as a drop-in replacement for `ufbx-rust`;
- Iteratively reduce the number of `unsafe` calls to a minimum, while keeping the underlying behavior the same:
  - Consolidate repeated unsafe operations into a few isolated, easy-to-audit spots (e.g. shared *view* types replacing raw-pointer navigation with borrow-based access);
  - Progressively narrow the remaining `unsafe {}` blocks and flip `unsafe fn`s to safe ones as their obligations are discharged. (See [PORTING.md](PORTING.md) “Unsafe reduction / isolation strategy”)

## Why do this?

`ufbx` was especially well suited for such a LLM translation due to its:

- Very mature, high-quality and platform-agnostic C codebase;
- Largely self-contained and deliberately easy-to-integrate structure with no external dependencies;
- Adherence to a subset of C features/patterns that are relatively straightforward to port to Rust;
- Extensive test coverage;
- Well-defined set of IR types with existing code generation scripts;
- Permissive license/dedication to the Public Domain.

## Why not some other approach?

### `c2rust`

We could have used `c2rust` to more quickly produce a transpiled Rust version upfront to work as a foundation, however:

- It translates specific preprocessed compilation units, not the underlying source input files; (`ufbx` has various feature gates/macros that whose structure would like to preserve in the ported code)
- It produces its own set of types and quirky code style, that would have required additional manual or LLM massaging to reach a usable state;
- It would make long-term/continuous porting of changes from the upstream project more challenging.

### An idiomatic, ground-up Rust implementation (either human or LLM-authored)

FBX is a massively complex file format with poor/no official publicly available documentation. (FOSS implementations are reverse-engineered)

Getting a brand-new non-toy implementation off the ground in Rust is a serious undertaking, that has been attempted before with various degrees of success. Present-day LLMs are likely not yet capable enough to do this to the level of quality/reliability required, and the person-hours this would entail are probably in the thousands. (See the ***Thanks / Acknowledgements*** section below)

### Just keep using the FFI bindings

- Complicates downstream building/setup requirements;
- Limits how safe/UB-free the code can be made.

### Also porting the “black-box” C tests to Rust

This would probably still produce decent-quality results, but not as strict/deterministic.

By keeping the C tests exactly as-is we avoid a whole class of bugs where both the underlying source and the tests diverge from the original library in tandem in a way that still “passes” but produces incorrect, hard to detect behavior. We can also use them as an extra safety net around LLM mistakes/hallucinations.

## Issues / PRs

- **Problems with this port, correctness/soundness/safety fixes, improvements to the tooling/approach, cleanup**? → Please open an issue or a PR against this repo. Keep in mind we want to keep the port as closely as a 1:1 match to the original code as possible, so the set of changes we're willing to accept is deliberately constrained.

- **New features, improvements or (confirmed reproducible) problems with the the underlying FBX implementation**? → Please contribute to the upstream repo instead, if accepted the changes will (eventually) be ported and land here.

> [!IMPORTANT]  
> **Do not** open issues/ask for support on the original `ufbx` repo or project spaces without first **confirming the issue you're encountering is reproducible upstream**. We do not want to burden or bother the project maintainer with issues related to this port. If you're unsure and would like help figuring it out, please open an issue here.

## Prior Art / Inspiration

- bun — https://bun.com/blog/bun-in-rust
- LibJS — https://ladybird.org/posts/adopting-rust/
- rav1d — https://www.memorysafety.org/blog/porting-c-to-rust-for-av1/
- zlib-rs — https://trifectatech.org/projects/zlib-rs/

## Thanks / Acknowledgements

We would like to thank and acknowledge the exceptional work of Samuli Raivio / @bqqbarbhg on the original upstream C `ufbx` implementation, as well as the `ufbx-rust` FFI bindings, without which this port obviously wouldn't be possible.

## Alternatives

- https://github.com/ufbx/ufbx-rust
- https://github.com/lo48576/fbxcel
- https://github.com/JulianKnodt/pars3d
- https://github.com/Latias94/asset-importer
