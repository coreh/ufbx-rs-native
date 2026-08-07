#!/usr/bin/env bash
# Regenerate the Rust API types + layout assertions from ufbx.h.
# Run from the repo root after any upstream merge that touches ufbx.h.
set -euo pipefail
cd "$(dirname "$0")/.."

python3 bindgen/ufbx_parser.py -i ufbx.h
python3 bindgen/ufbx_ir.py
PYTHONPATH=bindgen python3 rust/ufbx/bindgen/generate_rust.py \
    -i bindgen/build/ufbx_typed.json -o rust/ufbx/src
# The ufbx_* symbols are provided natively by this crate, not an external C lib.
sed -i.bak 's/^#\[link(name="ufbx")\]$/\/\/ #[link(name="ufbx")] — symbols provided natively by this crate (capi \/ native port)/' \
    rust/ufbx/src/generated.rs && rm rust/ufbx/src/generated.rs.bak
PYTHONPATH=bindgen python3 rust/ufbx/bindgen/generate_layout_tests.py \
    -i bindgen/build/ufbx_typed.json -o rust/ufbx/tests/layout.rs \
    --src rust/ufbx/src/generated.rs rust/ufbx/src/prelude.rs

(cd rust && cargo test --test layout)
