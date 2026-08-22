#!/usr/bin/env bash
# The full local commit gate for the rust-port branch. Run from anywhere; the
# script anchors itself to the repo root. Every check asserts its own literal
# success marker — two historical false-greens motivate the paranoia:
#   * `misc/run_tests.py … --rust-lib` can print `0/0 targets succeeded` and
#     exit 0; the hash gate here links hash_scene directly (the rust-port.yml
#     recipe) and greps for the literal `2174/2174 hashes match`.
#   * `test/runner` run without `-d data/` fails every test with "File not
#     found", and `runner | tail` masks its exit code; here pipefail + a literal
#     `Tests passed: 647/647` grep make that impossible to miss.
# Miri (SB+TB over tests/miri.rs) is slow (~3.5 min) and only relevant to
# aliasing-affecting changes: opt in with GATE_MIRI=1.
set -euo pipefail

cd "$(dirname "$0")/../.."   # repo root
ROOT=$(pwd)

step() { printf '\n== %s\n' "$*"; }

step "cargo fmt --check"
(cd rust && cargo fmt --check)

step "clippy (default / real-is-f32 / c-abi,dev / +regression / lean), all targets"
(cd rust && cargo clippy --all-targets --message-format=short -- -D warnings)
(cd rust && cargo clippy --features real-is-f32 --all-targets --message-format=short -- -D warnings)
(cd rust && cargo clippy --features c-abi,dev --all-targets --message-format=short -- -D warnings)
(cd rust && cargo clippy --features c-abi,dev,regression --all-targets --message-format=short -- -D warnings)
(cd rust && cargo clippy --no-default-features --all-targets --message-format=short -- -D warnings)

step "tests (default / real-is-f32)"
(cd rust && cargo test --quiet)
(cd rust && cargo test --quiet --features real-is-f32)

step "lean-config check matrix (every single-feature-off + fully lean)"
ALL="subdivision,tessellation,geometry-cache,scene-eval,skinning-eval,baking,triangulation,index-gen,obj"
for feat in ${ALL//,/ }; do
  rest=$(tr ',' '\n' <<<"$ALL" | grep -v "^$feat\$" | paste -sd, -)
  echo "   -- without $feat"
  (cd rust && cargo check --quiet --no-default-features --features "$rest")
done
echo "   -- fully lean (--no-default-features)"
(cd rust && cargo check --quiet --no-default-features)

step "c-abi,dev release build"
(cd rust && cargo build --quiet --features c-abi,dev --release)

step "hash oracle (literal 2174/2174)"
clang -O2 -std=gnu99 -DUFBX_DEV -ffp-contract=off -DEXTERNAL_UFBX \
  -o build/hash_scene_rust test/hash_scene.c extra/ufbx_math.c \
  rust/target/release/libufbx.a -lpthread -lm
HASH_OUT=$(./build/hash_scene_rust --check build/hashes.txt)
echo "$HASH_OUT" | tail -1
grep -q '^2174/2174 hashes match$' <<<"$HASH_OUT"

step "C suite (literal 647/647)"
clang -DUFBX_DEV -ffp-contract=off -DEXTERNAL_UFBX \
  -o build/runner_rust test/runner.c extra/ufbx_math.c \
  rust/target/release/libufbx.a -lpthread -lm
RUNNER_OUT=$(./build/runner_rust -d data/ 2>/dev/null | tail -1)
echo "$RUNNER_OUT"
grep -q '^Tests passed: 647/647$' <<<"$RUNNER_OUT"

if [[ "${GATE_MIRI:-0}" == "1" ]]; then
  step "Miri SB + TB (tests/miri.rs corpus)"
  (cd rust && MIRIFLAGS="-Zmiri-disable-isolation" \
    cargo +nightly miri test --test miri)
  (cd rust && MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-tree-borrows" \
    cargo +nightly miri test --test miri)
fi

step "GATE GREEN"
