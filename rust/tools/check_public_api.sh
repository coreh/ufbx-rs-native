#!/usr/bin/env bash
# Public-API parity gate: diff this crate's public API against the published
# ufbx-rust 0.11.2 (fetched from crates.io by cargo-public-api) and require
# the divergences to EXACTLY match rust/tools/api/expected-divergences.txt.
# Every line of that file is documented in COMPAT.md (§1 additions, §2
# divergences) — a new divergence fails, and so does one that silently
# disappears (e.g. upstream annotating a field would obsolete an override).
#
# Requires: rustup nightly toolchain + cargo-public-api.
# Refresh the expectations after an intentional API change with:
#   cargo public-api -p ufbx diff 0.11.2 | grep '^[+-]' | sort \
#     > rust/tools/api/expected-divergences.txt
set -euo pipefail
cd "$(dirname "$0")/../.."

cd rust
cargo public-api -p ufbx diff 0.11.2 | grep '^[+-]' | sort > /tmp/api_divergences_actual.txt
cd ..

if diff -u rust/tools/api/expected-divergences.txt /tmp/api_divergences_actual.txt; then
    echo "public API matches ufbx-rust 0.11.2 modulo the $(wc -l < rust/tools/api/expected-divergences.txt | tr -d ' ') documented divergences"
else
    echo ""
    echo "FAIL: public-API divergence set changed."
    echo "Lines only in expected = a documented divergence disappeared;"
    echo "lines only in actual = a NEW undocumented divergence."
    echo "If intentional: document it in COMPAT.md, then refresh the expectations (see header)."
    exit 1
fi
