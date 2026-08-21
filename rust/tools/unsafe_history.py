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
  * Generated files are excluded by default (they would swamp the
    hand-written trend); pass --include-generated to count them too.

Usage:
  python3 rust/tools/unsafe_history.py                # CSV + SVG next to repo root
  python3 rust/tools/unsafe_history.py --branch main --step 5 -o /tmp/hist
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


def git(repo, *args, check=True):
    r = subprocess.run(
        ["git", "-C", repo, *args], capture_output=True, text=True, errors="replace"
    )
    if check and r.returncode not in (0, 1):  # git grep exits 1 on no matches
        sys.exit(f"git {' '.join(args[:3])}... failed: {r.stderr.strip()}")
    return r.stdout


def count_at(repo, rev, pathspec, exclude):
    # One `git grep` per commit: pull every line containing `unsafe` out of the
    # commit's tree, then classify in-process. -I skips binary blobs.
    out = git(repo, "grep", "-I", "--no-color", "-e", "unsafe", rev, "--", pathspec)
    n_fn = n_block = 0
    for line in out.splitlines():
        # Format: rev:path:content — split on the first two colons.
        try:
            _, path, content = line.split(":", 2)
        except ValueError:
            continue
        if os.path.basename(path) in exclude:
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
    ap.add_argument("--include-generated", action="store_true")
    ap.add_argument("-o", "--out", default="unsafe_history", help="output basename")
    args = ap.parse_args()
    repo = os.path.realpath(args.repo)
    exclude = () if args.include_generated else DEFAULT_EXCLUDE

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
        n_fn, n_block = count_at(repo, short, args.pathspec, exclude)
        rows.append((i, short, date, n_fn, n_block))
        if i % 50 == 0:
            el = (datetime.datetime.now() - t0).total_seconds()
            print(f"  {i}/{len(sampled)} ({el:.0f}s)", file=sys.stderr)

    csv_path = args.out + ".csv"
    with open(csv_path, "w") as f:
        f.write("index,commit,date,unsafe_fn,unsafe_block\n")
        for r in rows:
            f.write(",".join(map(str, r)) + "\n")

    suffix = "" if args.include_generated else " (generated files excluded)"
    svg_path = args.out + ".svg"
    svg_chart(rows, svg_path, f"unsafe markers in {args.pathspec}{suffix}")
    print(f"wrote {csv_path} and {svg_path} ({len(rows)} samples)")


if __name__ == "__main__":
    main()
