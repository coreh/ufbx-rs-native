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
3. `python3 rust/tools/remap_line_refs.py <prev-upstream-sha> <new-upstream-sha>
   --apply` — remaps every `ufbx.c:N`/`ufbx.h:N[-M]` anchor comment in
   `rust/**/*.rs`, `PORTING.md`, and `UPSTREAM.md` to the new line numbers.
   Run it once without `--apply` first to read the table.
   - Refs landing inside a changed hunk get a `?stale` suffix; refs whose
     target line text no longer matches get `?review`. Both exit nonzero.
   - **A `?stale`/`?review` number still refers to the PREVIOUS base commit**
     (`?review` was renumbered, but the text underneath moved). Resolve every
     marker by hand (`grep -rn '?stale\|?review'`) **before the next sync** —
     otherwise the following run remaps an already-outdated number.
   - Ranges whose *interior* changed are listed under
     "interior changed — re-read these regions". Numbers are correct, the
     ported code may not be; re-read those regions while routing the diff.
   - The anchor base the tree is currently numbered against is recorded in a
     `<!-- line-ref-anchor-base: SHA -->` marker in the State section above
     (the tool inserts and maintains it). `--apply` refuses
     to run unless `<prev-upstream-sha>` matches it (this is what stops a
     double-apply from silently corrupting every ref) and advances it on
     success. First ever run: add `--init-anchor`. `--force` overrides.
4. Route the `ufbx.c` diff: each hunk's enclosing `ufbxi_*`/`ufbx_*` function
   maps to one module under `rust/ufbx/src/native/` (section list in
   `native/mod.rs`; PORTING.md has the naming rules). Port each hunk 1:1;
   adversarial review per PORTING.md pipeline.
5. Diff `test/unit_tests.c` — port any new white-box tests to Rust `#[test]`s.
6. C oracle regenerates `hashes.txt`; run the full matrix
   (`misc/run_tests.py tests hashes --rust-lib rust/target/release/libufbx.a`).
7. If error paths moved, regenerate the Rust-side fuzz table (`runner --fuzz`
   against the Rust build).
8. Record the ported upstream SHA above.

## Local modifications to upstream-owned files (keep this list complete)

| File | Change | Upstreamable? |
|---|---|---|
| `misc/run_tests.py` | `--rust-lib` flag + `apply_rust_lib()` + hash_scene EXTERNAL_UFBX wiring | maybe |
| `test/hash_scene.c` | trailing `#include "../ufbx.c"` wrapped in `#ifndef EXTERNAL_UFBX` (mirrors `check_fbx.c`) | yes |

Everything else added by this fork lives under `rust/`, `PORTING.md`, `UPSTREAM.md`.
