#!/usr/bin/env python3
"""Load-time benchmark: C ufbx vs the Rust port, via the hash_scene harness.

Both binaries do identical work (full load + scene walk + FNV hash), so the
Rust/C ratio isolates codegen/implementation differences. Uses min-of-N
timing (min is robust to scheduler noise; both sides measured interleaved on
the same machine so the ratio survives shared-runner variance).

Usage: bench.py [--c EXE] [--rust EXE] [-n RUNS] [files...]
Defaults: build/hash_scene_c, build/hash_scene_rust, curated corpus below.
Exit code is always 0 — this is a report, not a gate (promote to a gate with
--max-ratio when Phase 4 begins).
"""
import argparse
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# Curated to cover the distinct hot paths: big ASCII parse, big binary +
# DEFLATE, legacy 6100, animation-heavy, OBJ, and a string-pool stress file.
DEFAULT_FILES = [
    "data/maya_slime_7500_ascii.fbx",
    "data/motionbuilder_thumbnail_7700_ascii.fbx",
    "data/maya_kenney_character_7700_binary.fbx",
    "data/maya_human_ik_6100_binary.fbx",
    "data/maya_human_ik_6100_ascii.fbx",
    "data/synthetic_id_collision_7500_ascii.fbx",
    "data/blender_293_suzanne_subsurf_uv.obj",
]

def time_one(exe, file, runs, warmup=2):
    times = []
    for i in range(warmup + runs):
        t0 = time.perf_counter_ns()
        r = subprocess.run([exe, file], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        dt = time.perf_counter_ns() - t0
        if r.returncode != 0:
            print(f"  ERROR: {exe} {file} exited {r.returncode}", file=sys.stderr)
            return None
        if i >= warmup:
            times.append(dt)
    return min(times) / 1e6  # ms

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--c", default=str(ROOT / "build/hash_scene_c"))
    ap.add_argument("--rust", default=str(ROOT / "build/hash_scene_rust"))
    ap.add_argument("-n", type=int, default=10, help="timed runs per file (min taken)")
    ap.add_argument("--max-ratio", type=float, help="fail if geomean ratio exceeds this")
    ap.add_argument("files", nargs="*", default=None)
    args = ap.parse_args()

    files = args.files or [str(ROOT / f) for f in DEFAULT_FILES]
    load1, _, _ = os.getloadavg()
    ncpu = os.cpu_count() or 1
    if load1 > ncpu * 0.5:
        print(f"WARNING: system load {load1:.1f} on {ncpu} cores — numbers will be noisy\n")

    print(f"{'file':<44} {'C (ms)':>9} {'Rust (ms)':>10} {'Rust/C':>7}")
    ratios = []
    for f in files:
        name = Path(f).name
        # interleave to keep thermal/cache conditions comparable
        tc = time_one(args.c, f, args.n)
        tr = time_one(args.rust, f, args.n)
        if tc is None or tr is None:
            continue
        ratio = tr / tc
        ratios.append(ratio)
        print(f"{name:<44} {tc:>9.2f} {tr:>10.2f} {ratio:>6.2f}x")

    if ratios:
        geomean = statistics.geometric_mean(ratios)
        print(f"\ngeomean Rust/C ratio: {geomean:.3f}x over {len(ratios)} files ({args.n} runs each, min-of-N)")
        if args.max_ratio is not None and geomean > args.max_ratio:
            print(f"FAIL: geomean {geomean:.3f} exceeds --max-ratio {args.max_ratio}")
            return 1
    return 0

if __name__ == "__main__":
    sys.exit(main())
