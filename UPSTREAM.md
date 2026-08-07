# UPSTREAM.md — upstream sync state & delta-port workflow

## State

- Upstream: https://github.com/ufbx/ufbx
- **Last fully ported upstream commit:** _(none yet — initial port in progress)_
- Rust fuzz-check table generation state: _(not yet generated)_

## Delta-port workflow (per upstream sync)

1. `git merge upstream/master` — C sources, header, tests, and data arrive
   together; the C side is green by construction.
2. `./rust/regen.sh` — regenerates IR, `generated.rs`, and layout assertions.
   Header changes (new fields/types/functions) surface here mechanically.
3. Route the `ufbx.c` diff: each hunk's enclosing `ufbxi_*`/`ufbx_*` function
   maps to one module under `rust/ufbx/src/native/` (section list in
   `native/mod.rs`; PORTING.md has the naming rules). Port each hunk 1:1;
   adversarial review per PORTING.md pipeline.
4. Diff `test/unit_tests.c` — port any new white-box tests to Rust `#[test]`s.
5. C oracle regenerates `hashes.txt`; run the full matrix
   (`misc/run_tests.py tests hashes --rust-lib rust/target/release/libufbx.a`).
6. If error paths moved, regenerate the Rust-side fuzz table (`runner --fuzz`
   against the Rust build).
7. Record the ported upstream SHA above.

## Local modifications to upstream-owned files (keep this list complete)

| File | Change | Upstreamable? |
|---|---|---|
| `misc/run_tests.py` | `--rust-lib` flag + `apply_rust_lib()` + hash_scene EXTERNAL_UFBX wiring | maybe |
| `test/hash_scene.c` | trailing `#include "../ufbx.c"` wrapped in `#ifndef EXTERNAL_UFBX` (mirrors `check_fbx.c`) | yes |

Everything else added by this fork lives under `rust/`, `PORTING.md`, `UPSTREAM.md`.
