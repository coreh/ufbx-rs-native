#!/usr/bin/env bash
# Public-API parity gate: diff this crate's public API against the published
# ufbx-rust 0.11.2 (fetched from crates.io by cargo-public-api) and require
# the divergences to EXACTLY match rust/tools/api/expected-divergences.txt.
# Every line of that file is documented in COMPAT.md (§1 additions, §2
# divergences) — a new divergence fails, and so does one that silently
# disappears (e.g. upstream annotating a field would obsolete an override).
#
# Requires: rustup nightly toolchain + cargo-public-api 0.52.
# The comparison is normalized to be toolchain-agnostic: --simplified drops
# blanket/auto impls (rendering varies by rustdoc version) and re-export
# paths are canonicalized to the crate root (rustdoc versions differ on which
# duplicate path they report).
# It is also HOST-agnostic: LC_ALL=C forces byte-wise collation so `sort`
# produces one canonical order everywhere. Without it a en_US.UTF-8 host
# (macOS default) folds case and ignores leading punctuation, so `+`/`-`
# lines interleave and `set_panic_handler` sorts before `ShaderPropBinding`,
# while a C-locale CI runner orders them the other way — the same item set,
# rendered in two orders, which `diff` reports as a spurious API change.
# To reproduce a specific nightly locally: RUSTUP_TOOLCHAIN=nightly-YYYY-MM-DD
# (cargo-public-api 0.52 has no --toolchain flag; it honours the rustup env).
# Refresh the expectations after an intentional API change by re-running this
# script and copying /tmp/api_divergences_actual.txt over the expectations.
set -euo pipefail
export LC_ALL=C
cd "$(dirname "$0")/../.."

cd rust
cargo public-api -p ufbx --simplified diff 0.11.2 \
    | grep '^[+-]' \
    | sed 's/ufbx::generated::/ufbx::/g; s/ufbx::prelude::/ufbx::/g' \
    | sort -u > /tmp/api_divergences_actual.txt
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
