#!/usr/bin/env bash
# Drop-in parity gate: run ufbx-rust's OWN test suite (unmodified, from the
# pinned v0.11.2 tag) against this crate as the `ufbx` dependency.
# The crates.io tarball excludes tests/, so the suite comes from git.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
UFBX_RUST_REV="b608754e3d513572bcc1f59eb8604a2cfe3b2a0a"  # v0.11.2

HARNESS="$(mktemp -d)"
trap 'rm -rf "$HARNESS"' EXIT

git clone --quiet https://github.com/ufbx/ufbx-rust.git "$HARNESS/ufbx-rust"
git -C "$HARNESS/ufbx-rust" checkout --quiet "$UFBX_RUST_REV"

mkdir -p "$HARNESS/parity"
cp -r "$HARNESS/ufbx-rust/tests" "$HARNESS/parity/tests"
echo "// empty: this crate exists to run ufbx-rust's tests against the native port" > "$HARNESS/parity/lib.rs"
cat > "$HARNESS/parity/Cargo.toml" << EOF
[package]
name = "ufbx-rust-parity"
version = "0.0.0"
edition = "2021"
publish = false

[lib]
path = "lib.rs"

[dependencies]
ufbx = { path = "$REPO_ROOT/rust/ufbx" }

# ufbx-rust's dev-dependencies (its tests use them)
[dev-dependencies]
libc = "0.2"
panic-message = "0.3"

[workspace]
EOF

cd "$HARNESS/parity"
cargo test
