#!/usr/bin/env bash
# Build and run the differential math fuzzer (native/math.rs vs
# extra/ufbx_math.c, bit-for-bit). Run from anywhere; deterministic seed.
# Optional arg: random sample count (default 400000 in main.c).
set -euo pipefail
cd "$(dirname "$0")"

cargo build --release --quiet
clang -O2 -ffp-contract=off -std=gnu99 -o target/mathfuzz \
    main.c ../../../extra/ufbx_math.c target/release/libmathfuzz.a -lm
exec ./target/mathfuzz "$@"
