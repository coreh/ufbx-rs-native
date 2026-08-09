#!/usr/bin/env bash
#
# Differential fuzz table: run test/runner.c's --fuzz suite twice, once against
# the C reference (ufbx.c) and once against the Rust staticlib, then compare the
# results. This is the widest oracle available — it mutates every file in data/
# (truncation, byte patches, read-buffer sizes, cancellation) and checks that
# both implementations accept and reject exactly the same things.
#
# It is also the slowest: single-threaded it is an overnight run, so the default
# is an OpenMP build across half the cores. Threaded output interleaves
# nondeterministically, hence the comparison normalizes and sorts rather than
# diffing positionally — order does not carry information here, the per-case
# results do.
#
# Usage:
#   rust/tools/fuzz_table.sh                 # build, run both, compare
#   rust/tools/fuzz_table.sh -j 4            # gentler: 4 threads per side
#   rust/tools/fuzz_table.sh -j 1            # single-threaded, deterministic order
#   rust/tools/fuzz_table.sh --sequential    # one side at a time (quieter fans)
#   rust/tools/fuzz_table.sh --compare-only  # just re-compare existing output
#
# Output lands in build/fuzz/ ({c,rust}.txt raw, {c,rust}.norm normalized).
# Exits non-zero if the two sides disagree.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

OUT_DIR="build/fuzz"
JOBS=""
SEQUENTIAL=0
COMPARE_ONLY=0
DATA_DIR="data"

while [[ $# -gt 0 ]]; do
	case "$1" in
		-j) JOBS="$2"; shift 2 ;;
		-o) OUT_DIR="$2"; shift 2 ;;
		-d) DATA_DIR="$2"; shift 2 ;;
		--sequential) SEQUENTIAL=1; shift ;;
		--compare-only) COMPARE_ONLY=1; shift ;;
		-h|--help) sed -n '2,26p' "${BASH_SOURCE[0]}" | sed 's|^# \?||'; exit 0 ;;
		*) echo "unknown argument: $1" >&2; exit 2 ;;
	esac
done

mkdir -p "$OUT_DIR"

# Default to half the cores per side, so the two runs together saturate the
# machine but leave it usable. `nproc` on Linux, `sysctl` on macOS.
if [[ -z "$JOBS" ]]; then
	if command -v nproc >/dev/null 2>&1; then
		NCPU=$(nproc)
	else
		NCPU=$(sysctl -n hw.ncpu)
	fi
	JOBS=$(( NCPU / 2 ))
	[[ "$JOBS" -lt 1 ]] && JOBS=1
fi

# `--fuzz` output is progress-dominated, and the progress is emitted on a timer
# rather than at fixed intervals — so which counter values appear is genuinely
# nondeterministic and must be discarded, not merely reordered. What survives is
# the signal: the case names, the per-group completion lines, and any failure.
#
#   - carriage returns separate in-place counter updates    -> real newlines
#   - "Fuzzing 12/34" appended to a case-name line          -> stripped inline
#   - "Fuzzing <what> <file>: 5584/24560" progress lines    -> dropped entirely
#   - "fuzzing done in 4.60s"                               -> time neutralized
#   - "Fuzzing with N threads"                              -> thread count neutralized
#
# Sorted, so thread interleaving does not register as a difference.
normalize() {
	tr '\r' '\n' < "$1" \
		| sed -E 's/Fuzzing [0-9]+\/[0-9]+//g' \
		| grep -vE '^Fuzzing .*: [0-9]+\/[0-9]+[[:space:]]*$' \
		| sed -E 's/done in [0-9.]+s/done in Xs/; s/^Fuzzing with [0-9]+ threads/Fuzzing with N threads/' \
		| grep -v '^[[:space:]]*$' \
		| sort > "$2"
}

compare() {
	local c_norm="$OUT_DIR/c.norm" rust_norm="$OUT_DIR/rust.norm"
	normalize "$OUT_DIR/c.txt" "$c_norm"
	normalize "$OUT_DIR/rust.txt" "$rust_norm"

	local n_lines n_groups
	n_lines=$(wc -l < "$c_norm" | tr -d ' ')
	n_groups=$(grep -c 'fuzzing done' "$c_norm" || true)

	if diff -q "$c_norm" "$rust_norm" >/dev/null; then
		echo "MATCH: $n_lines normalized lines, $n_groups fuzz groups, zero divergence"
		return 0
	fi

	echo "DIVERGENCE between C and Rust:"
	diff "$c_norm" "$rust_norm" | head -60
	echo
	echo "(full normalized output: $c_norm vs $rust_norm)"
	return 1
}

if [[ "$COMPARE_ONLY" == 1 ]]; then
	compare
	exit $?
fi

# -- Build

echo "== Building Rust staticlib (c-abi,dev)"
# `dev` is required: the runner is compiled with -DUFBX_DEV and asserts
# dev-only behavior such as error-stack depth.
cargo build --manifest-path rust/Cargo.toml --features c-abi,dev --release

OMP_FLAGS=()
if [[ "$JOBS" -gt 1 ]]; then
	# Apple's clang needs libomp out of Homebrew; Linux clang/gcc have it built in.
	if [[ "$(uname)" == "Darwin" ]]; then
		LIBOMP=$(brew --prefix libomp 2>/dev/null || true)
		if [[ -n "$LIBOMP" && -f "$LIBOMP/lib/libomp.dylib" ]]; then
			OMP_FLAGS=(-Xpreprocessor -fopenmp -I"$LIBOMP/include" -L"$LIBOMP/lib" -lomp)
		else
			echo "note: libomp not found (brew install libomp) — falling back to 1 thread"
			JOBS=1
		fi
	else
		OMP_FLAGS=(-fopenmp)
	fi
fi

CFLAGS=(-O2 -std=gnu99 -DUFBX_DEV -ffp-contract=off)

echo "== Building fuzz runners (threads per side: $JOBS)"
clang "${CFLAGS[@]}" "${OMP_FLAGS[@]}" \
	-o "$OUT_DIR/runner_c" test/runner.c ufbx.c -lpthread -lm
clang "${CFLAGS[@]}" "${OMP_FLAGS[@]}" -DEXTERNAL_UFBX \
	-o "$OUT_DIR/runner_rust" test/runner.c rust/target/release/libufbx.a -lpthread -lm

# -- Run

echo "== Fuzzing (this takes hours; ^C is safe, partial output still compares)"
export OMP_NUM_THREADS="$JOBS"

if [[ "$SEQUENTIAL" == 1 ]]; then
	"$OUT_DIR/runner_c" --fuzz -d "$DATA_DIR" > "$OUT_DIR/c.txt" 2>&1 || true
	"$OUT_DIR/runner_rust" --fuzz -d "$DATA_DIR" > "$OUT_DIR/rust.txt" 2>&1 || true
else
	"$OUT_DIR/runner_c" --fuzz -d "$DATA_DIR" > "$OUT_DIR/c.txt" 2>&1 &
	pid_c=$!
	"$OUT_DIR/runner_rust" --fuzz -d "$DATA_DIR" > "$OUT_DIR/rust.txt" 2>&1 &
	pid_rust=$!
	wait "$pid_c" || true
	wait "$pid_rust" || true
fi

echo "== Comparing"
compare
