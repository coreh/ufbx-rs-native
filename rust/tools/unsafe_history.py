#!/usr/bin/env python3
"""Plot `unsafe fn` and `unsafe {` counts over the branch history.

Walks the first-parent history of a branch, counts unsafe markers in the Rust
sources at every commit (via `git grep` against the commit's tree — no
checkouts), and emits a CSV plus a dependency-free SVG line chart.

Counting rules:
  * `unsafe fn`  — occurrences of `unsafe fn` / `unsafe extern "C" fn`
                   (fn-level obligations: caller must uphold a contract).
  * `unsafe {`   — occurrences of `unsafe {` blocks (op-level obligations:
                   body vouches locally).
  * `//` line comments are stripped before counting, so SAFETY prose that
    mentions `unsafe { }` is not counted. Block comments are not handled
    (the codebase convention is `//`).
  * Generated files (generated.rs, generated_views.rs) are counted by
    default; pass --exclude-generated to plot only the hand-written trend.
  * The capi surface (capi.rs) is EXCLUDED by default — it is `unsafe extern
    "C"` by nature and never trends down; pass --include-capi to count it.
  * Test code is EXCLUDED by default: the `tests/` directory by path, and
    in-file test modules by the crate convention that a column-0
    `#[cfg(test)]` attribute introduces the test tail of the file (first such
    line through end-of-file). Pass --include-tests to count test bodies.

Usage:
  python3 rust/tools/unsafe_history.py                # CSV + SVG next to repo root
  python3 rust/tools/unsafe_history.py --branch main --step 5 -o /tmp/hist
  python3 rust/tools/unsafe_history.py --include-capi --include-tests
"""

import argparse
import datetime
import os
import re
import subprocess
import sys

UNSAFE_FN = re.compile(r'\bunsafe\s+(?:extern\s+"C"\s+)?fn\b')
UNSAFE_BLOCK = re.compile(r"\bunsafe\s*\{")
LINE_COMMENT = re.compile(r"//.*$")

DEFAULT_EXCLUDE = ("generated.rs", "generated_views.rs")
CFG_TEST = re.compile(r"^#\[cfg\(test\)\]")


def git(repo, *args, check=True):
    r = subprocess.run(
        ["git", "-C", repo, *args], capture_output=True, text=True, errors="replace"
    )
    if check and r.returncode not in (0, 1):  # git grep exits 1 on no matches
        sys.exit(f"git {' '.join(args[:3])}... failed: {r.stderr.strip()}")
    return r.stdout


def test_cutoffs(repo, rev, pathspec):
    # Per file: the first column-0 `#[cfg(test)]` line begins the test tail
    # (crate convention: test modules close out the file). Lines at or past it
    # are test code.
    out = git(repo, "grep", "-n", "-I", "--no-color",
              "-e", "^#\\[cfg(test)\\]", rev, "--", pathspec)
    cutoffs = {}
    for line in out.splitlines():
        try:
            _, path, lineno, _ = line.split(":", 3)
            lineno = int(lineno)
        except ValueError:
            continue
        if path not in cutoffs or lineno < cutoffs[path]:
            cutoffs[path] = lineno
    return cutoffs


def count_at(repo, rev, pathspec, exclude, include_tests):
    # One `git grep -n` per commit: pull every line containing `unsafe` out of
    # the commit's tree, then classify in-process. -I skips binary blobs.
    out = git(repo, "grep", "-n", "-I", "--no-color", "-e", "unsafe", rev, "--", pathspec)
    cutoffs = {} if include_tests else test_cutoffs(repo, rev, pathspec)
    n_fn = n_block = 0
    for line in out.splitlines():
        # Format: rev:path:lineno:content — split on the first three colons.
        try:
            _, path, lineno, content = line.split(":", 3)
            lineno = int(lineno)
        except ValueError:
            continue
        if os.path.basename(path) in exclude:
            continue
        if not include_tests:
            if "/tests/" in path or path.startswith("tests/"):
                continue
            if lineno >= cutoffs.get(path, sys.maxsize):
                continue
        content = LINE_COMMENT.sub("", content)
        n_fn += len(UNSAFE_FN.findall(content))
        n_block += len(UNSAFE_BLOCK.findall(content))
    return n_fn, n_block


def svg_chart(rows, out_path, title):
    # rows: list of (index, short_hash, date_iso, n_fn, n_block)
    W, H = 1000, 520
    ML, MR, MT, MB = 64, 24, 44, 56  # margins
    pw, ph = W - ML - MR, H - MT - MB
    n = len(rows)
    ymax = max(1, max(max(r[3], r[4]) for r in rows))
    # round the y ceiling up to a friendly step
    step = max(1, 10 ** (len(str(ymax)) - 1) // 2)
    ymax = ((ymax // step) + 1) * step

    def x(i):
        return ML + (pw * i / max(1, n - 1))

    def y(v):
        return MT + ph - (ph * v / ymax)

    def polyline(idx, color):
        pts = " ".join(f"{x(i):.1f},{y(r[idx]):.1f}" for i, r in enumerate(rows))
        return (
            f'<polyline fill="none" stroke="{color}" stroke-width="2" '
            f'points="{pts}"/>'
        )

    # y grid lines
    grid = []
    for gv in range(0, ymax + 1, step):
        gy = y(gv)
        grid.append(
            f'<line x1="{ML}" y1="{gy:.1f}" x2="{W - MR}" y2="{gy:.1f}" '
            f'stroke="#ddd" stroke-width="1"/>'
            f'<text x="{ML - 8}" y="{gy + 4:.1f}" text-anchor="end" '
            f'font-size="12" fill="#666">{gv}</text>'
        )
    # x ticks: ~8 date labels
    ticks = []
    for i in range(0, n, max(1, n // 8)):
        tx = x(i)
        ticks.append(
            f'<line x1="{tx:.1f}" y1="{MT + ph}" x2="{tx:.1f}" y2="{MT + ph + 5}" '
            f'stroke="#999"/>'
            f'<text x="{tx:.1f}" y="{MT + ph + 20}" text-anchor="middle" '
            f'font-size="11" fill="#666">{rows[i][2]}</text>'
        )

    fn_color, blk_color = "#c0392b", "#2c6fbb"
    last = rows[-1]
    svg = f"""<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}"
     viewBox="0 0 {W} {H}" font-family="system-ui, sans-serif">
  <rect width="{W}" height="{H}" fill="white"/>
  <text x="{ML}" y="24" font-size="16" fill="#222">{title}</text>
  <text x="{W - MR}" y="24" font-size="12" fill="#666" text-anchor="end">
    {n} commits, latest {last[1]} ({last[2]})</text>
  {''.join(grid)}
  {''.join(ticks)}
  {polyline(3, fn_color)}
  {polyline(4, blk_color)}
  <rect x="{ML}" y="{MT - 10}" width="12" height="3" fill="{fn_color}"/>
  <text x="{ML + 18}" y="{MT - 5}" font-size="12" fill="#222">unsafe fn ({last[3]})</text>
  <rect x="{ML + 140}" y="{MT - 10}" width="12" height="3" fill="{blk_color}"/>
  <text x="{ML + 158}" y="{MT - 5}" font-size="12" fill="#222">unsafe {{ }} ({last[4]})</text>
</svg>
"""
    with open(out_path, "w") as f:
        f.write(svg)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--repo", default=os.path.join(os.path.dirname(__file__), "..", ".."))
    ap.add_argument("--branch", default="HEAD")
    ap.add_argument("--pathspec", default="rust/ufbx/src", help="tree prefix to count")
    ap.add_argument("--step", type=int, default=1, help="sample every Nth commit")
    ap.add_argument("--exclude-generated", action="store_true")
    ap.add_argument("--include-capi", action="store_true",
                    help="count capi.rs (excluded by default: unsafe extern by nature)")
    ap.add_argument("--include-tests", action="store_true",
                    help="count test code (tests/ dir + in-file #[cfg(test)] tails)")
    ap.add_argument("-o", "--out", default="unsafe_history", help="output basename")
    args = ap.parse_args()
    repo = os.path.realpath(args.repo)
    exclude = DEFAULT_EXCLUDE if args.exclude_generated else ()
    if not args.include_capi:
        exclude = exclude + ("capi.rs",)

    revs = git(
        repo, "rev-list", "--first-parent", "--reverse",
        "--format=%h %cs", "--no-commit-header", args.branch, "--", args.pathspec,
    ).split("\n")
    revs = [r.split() for r in revs if r.strip()]
    sampled = revs[:: args.step]
    if sampled[-1] != revs[-1]:
        sampled.append(revs[-1])  # always include the tip

    rows = []
    t0 = datetime.datetime.now()
    for i, (short, date) in enumerate(sampled):
        n_fn, n_block = count_at(repo, short, args.pathspec, exclude, args.include_tests)
        rows.append((i, short, date, n_fn, n_block))
        if i % 50 == 0:
            el = (datetime.datetime.now() - t0).total_seconds()
            print(f"  {i}/{len(sampled)} ({el:.0f}s)", file=sys.stderr)

    csv_path = args.out + ".csv"
    with open(csv_path, "w") as f:
        f.write("index,commit,date,unsafe_fn,unsafe_block\n")
        for r in rows:
            f.write(",".join(map(str, r)) + "\n")

    excluded = []
    if args.exclude_generated:
        excluded.append("generated")
    if not args.include_capi:
        excluded.append("capi")
    if not args.include_tests:
        excluded.append("tests")
    suffix = f" (excluding {', '.join(excluded)})" if excluded else ""
    svg_path = args.out + ".svg"
    svg_chart(rows, svg_path, f"unsafe markers in {args.pathspec}{suffix}")
    print(f"wrote {csv_path} and {svg_path} ({len(rows)} samples)")


if __name__ == "__main__":
    main()
