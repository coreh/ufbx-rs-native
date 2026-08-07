#!/usr/bin/env python3
"""Remap stale C source line references after an upstream merge.

The Rust port and docs contain anchor comments like `ufbx.c:1234`,
`ufbx.c:1234-1301`, `ufbx.h:102-109` referencing lines in the upstream C
sources. When upstream changes, this tool remaps those line numbers using
the hunk headers of `git diff --unified=0 <old> <new> -- <file>`.

Usage:
    python3 rust/tools/remap_line_refs.py <old-commit> <new-commit> [--apply] [--files GLOB...]
    python3 rust/tools/remap_line_refs.py --self-test

Modes:
    default   dry run: print a table of every ref and its mapping
    --apply   rewrite unambiguous refs in place; append `?stale` after
              ambiguous/invalid refs (greppable, harmless inside comments)

Exit code: 0 if every ref mapped cleanly, 1 if any ref is ambiguous or
invalid (or already carries a `?stale` marker).

Mapping rules (per C file, hunks sorted by old start line):
    - lines before a hunk keep the cumulative offset accumulated so far
    - lines after a hunk accumulate offset += (new_count - old_count)
    - lines falling inside a removed/replaced old range are AMBIGUOUS
    - pure insertions (old_count == 0, "after line a") make no line ambiguous
    - a range ref is ambiguous if either endpoint is ambiguous
    - a ref beyond EOF of the old commit's file is INVALID
"""

import argparse
import glob
import os
import re
import subprocess
import sys

REF_RE = re.compile(r"\b(ufbx\.[ch]):(\d+)(?:-(\d+))?(\?stale)?")
HUNK_RE = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")

DEFAULT_GLOBS = ["rust/**/*.rs", "PORTING.md", "UPSTREAM.md"]

AMBIGUOUS = "AMBIGUOUS"
INVALID = "INVALID"


# ---------------------------------------------------------------------------
# Core mapping math (pure; exercised by --self-test)
# ---------------------------------------------------------------------------

def parse_hunks(diff_text):
    """Parse unified=0 diff output into (old_start, old_count, new_start, new_count)."""
    hunks = []
    for line in diff_text.splitlines():
        m = HUNK_RE.match(line)
        if m:
            a = int(m.group(1))
            b = 1 if m.group(2) is None else int(m.group(2))
            c = int(m.group(3))
            d = 1 if m.group(4) is None else int(m.group(4))
            hunks.append((a, b, c, d))
    hunks.sort(key=lambda h: (h[0], h[2]))
    return hunks


def map_line(n, hunks, old_line_count=None):
    """Map old-commit line `n` to a new-commit line.

    Returns (status, new_line) where status is 'ok', AMBIGUOUS, or INVALID
    (new_line is None unless status == 'ok').
    """
    if n < 1 or (old_line_count is not None and n > old_line_count):
        return (INVALID, None)
    offset = 0
    for a, b, c, d in hunks:
        if b == 0:
            # Pure insertion after old line `a`: lines <= a are untouched by it.
            if n <= a:
                return ("ok", n + offset)
            offset += d
        else:
            if n < a:
                return ("ok", n + offset)
            if n <= a + b - 1:
                return (AMBIGUOUS, None)
            offset += d - b
    return ("ok", n + offset)


def map_ref(start, end, hunks, old_line_count=None):
    """Map a single line (end=None) or a range. Either endpoint ambiguous or
    invalid taints the whole ref. Returns (status, new_start, new_end)."""
    s_status, s_new = map_line(start, hunks, old_line_count)
    if end is None:
        return (s_status, s_new, None)
    e_status, e_new = map_line(end, hunks, old_line_count)
    for bad in (INVALID, AMBIGUOUS):
        if s_status == bad or e_status == bad:
            return (bad, None, None)
    return ("ok", s_new, e_new)


# ---------------------------------------------------------------------------
# Git plumbing
# ---------------------------------------------------------------------------

def run_git(repo_root, args):
    result = subprocess.run(
        ["git", "-C", repo_root] + args,
        capture_output=True, text=True,
    )
    return result


def repo_toplevel():
    here = os.path.dirname(os.path.abspath(__file__))
    result = subprocess.run(
        ["git", "-C", here, "rev-parse", "--show-toplevel"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        sys.exit("error: not inside a git repository")
    return result.stdout.strip()


def load_file_mapping(repo_root, old, new, cfile):
    """Return (hunks, old_line_count) for one C file, or (None, None) if the
    file does not exist at the old commit."""
    show = run_git(repo_root, ["show", "%s:%s" % (old, cfile)])
    if show.returncode != 0:
        return (None, None)
    old_line_count = show.stdout.count("\n")
    if show.stdout and not show.stdout.endswith("\n"):
        old_line_count += 1
    diff = run_git(repo_root, [
        "diff", "--unified=0", old, new, "--", cfile,
    ])
    if diff.returncode not in (0, 1):
        sys.exit("error: git diff failed for %s: %s" % (cfile, diff.stderr.strip()))
    return (parse_hunks(diff.stdout), old_line_count)


# ---------------------------------------------------------------------------
# Scan / apply
# ---------------------------------------------------------------------------

def collect_files(repo_root, patterns):
    files = []
    for pat in patterns:
        for path in sorted(glob.glob(os.path.join(repo_root, pat), recursive=True)):
            if os.sep + "target" + os.sep in path:
                continue
            if os.path.isfile(path):
                files.append(path)
    # de-dup, keep order
    seen = set()
    out = []
    for f in files:
        if f not in seen:
            seen.add(f)
            out.append(f)
    return out


def format_ref(cfile, start, end):
    return "%s:%d" % (cfile, start) if end is None else "%s:%d-%d" % (cfile, start, end)


def process(repo_root, old, new, patterns, apply_changes):
    mappings = {}  # cfile -> (hunks, old_line_count) or (None, None)
    rows = []      # (location, old_ref, result_str)
    problems = []  # locations of ambiguous/invalid/pre-stale refs

    for path in collect_files(repo_root, patterns):
        with open(path, "r", encoding="utf-8") as f:
            text = f.read()
        rel = os.path.relpath(path, repo_root)
        changed = False
        out_lines = []
        for lineno, line in enumerate(text.splitlines(keepends=True), 1):
            def repl(m):
                nonlocal changed
                cfile, s, e, stale = m.group(1), int(m.group(2)), m.group(3), m.group(4)
                e = int(e) if e is not None else None
                loc = "%s:%d" % (rel, lineno)
                old_ref = format_ref(cfile, s, e)
                if stale:
                    rows.append((loc, old_ref + "?stale", "ALREADY-STALE (resolve by hand)"))
                    problems.append(loc)
                    return m.group(0)
                if cfile not in mappings:
                    mappings[cfile] = load_file_mapping(repo_root, old, new, cfile)
                hunks, count = mappings[cfile]
                if hunks is None:
                    status, ns, ne = (INVALID, None, None)
                else:
                    status, ns, ne = map_ref(s, e, hunks, count)
                if status == "ok":
                    new_ref = format_ref(cfile, ns, ne)
                    rows.append((loc, old_ref, new_ref if new_ref != old_ref else "(unchanged)"))
                    if apply_changes and new_ref != old_ref:
                        changed = True
                        return new_ref
                    return m.group(0)
                rows.append((loc, old_ref, status))
                problems.append(loc)
                if apply_changes:
                    changed = True
                    return old_ref + "?stale"
                return m.group(0)

            out_lines.append(REF_RE.sub(repl, line))
        if apply_changes and changed:
            with open(path, "w", encoding="utf-8") as f:
                f.write("".join(out_lines))
    return rows, problems


def print_table(rows):
    if not rows:
        print("no line references found")
        return
    w0 = max(len(r[0]) for r in rows)
    w1 = max(len(r[1]) for r in rows)
    print("%-*s  %-*s  %s" % (w0, "REFERENCE AT", w1, "OLD TARGET", "NEW TARGET"))
    for loc, old_ref, result in rows:
        print("%-*s  %-*s  %s" % (w0, loc, w1, old_ref, result))


# ---------------------------------------------------------------------------
# Self-test
# ---------------------------------------------------------------------------

def self_test():
    failures = []

    def check(name, got, want):
        if got != want:
            failures.append("%s: got %r, want %r" % (name, got, want))

    # Replacement hunk: old lines 5-6 replaced by 3 new lines (net +1),
    # then a pure insertion of 4 lines after old line 10 (net +4).
    diff = (
        "@@ -5,2 +5,3 @@\n"
        "@@ -10,0 +12,4 @@\n"
    )
    hunks = parse_hunks(diff)
    check("parse_hunks", hunks, [(5, 2, 5, 3), (10, 0, 12, 4)])

    check("before-hunk", map_line(3, hunks), ("ok", 3))
    check("just-before-hunk", map_line(4, hunks), ("ok", 4))
    check("inside-replacement-start", map_line(5, hunks), (AMBIGUOUS, None))
    check("inside-replacement-end", map_line(6, hunks), (AMBIGUOUS, None))
    check("after-hunk", map_line(7, hunks), ("ok", 8))
    check("insertion-point-unmoved", map_line(10, hunks), ("ok", 11))
    check("inside-insertion-is-not-ambiguous", map_line(11, hunks), ("ok", 16))
    check("after-both-hunks", map_line(20, hunks), ("ok", 25))

    # Pure deletion: old lines 7-9 removed (net -3).
    del_hunks = parse_hunks("@@ -7,3 +6,0 @@\n")
    check("before-deletion", map_line(6, del_hunks), ("ok", 6))
    check("inside-deletion", map_line(8, del_hunks), (AMBIGUOUS, None))
    check("after-deletion", map_line(10, del_hunks), ("ok", 7))

    # Default counts: "@@ -3 +3 @@" means -3,1 +3,1.
    one = parse_hunks("@@ -3 +3 @@\n")
    check("implicit-count", one, [(3, 1, 3, 1)])
    check("implicit-count-inside", map_line(3, one), (AMBIGUOUS, None))
    check("implicit-count-after", map_line(4, one), ("ok", 4))

    # Ranges.
    check("range-clean", map_ref(1, 4, hunks), ("ok", 1, 4))
    check("range-shifted", map_ref(7, 9, hunks), ("ok", 8, 10))
    check("range-straddling-start", map_ref(4, 6, hunks), (AMBIGUOUS, None, None))
    check("range-straddling-end", map_ref(6, 8, hunks), (AMBIGUOUS, None, None))
    # A range fully spanning a replaced hunk keeps mappable endpoints;
    # only ambiguous endpoints taint a range.
    check("range-spanning-hunk", map_ref(4, 8, hunks), ("ok", 4, 9))
    check("single-line-ref", map_ref(7, None, hunks), ("ok", 8, None))

    # EOF / invalid.
    check("beyond-eof", map_line(51, hunks, old_line_count=50), (INVALID, None))
    check("at-eof", map_line(50, hunks, old_line_count=50), ("ok", 55))
    check("line-zero", map_line(0, hunks), (INVALID, None))
    check("range-with-invalid-end", map_ref(7, 51, hunks, 50), (INVALID, None, None))

    # No hunks (identical commits) -> identity.
    check("identity", map_line(123, []), ("ok", 123))

    if failures:
        for f in failures:
            print("FAIL %s" % f)
        print("self-test: %d failure(s)" % len(failures))
        return 1
    print("self-test: all checks passed")
    return 0


# ---------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("old_commit", nargs="?", help="upstream commit the refs currently anchor to")
    ap.add_argument("new_commit", nargs="?", help="upstream commit to remap the refs onto")
    ap.add_argument("--apply", action="store_true", help="rewrite refs in place (default: dry run)")
    ap.add_argument("--files", nargs="+", metavar="GLOB",
                    help="glob(s) relative to repo root (default: %s)" % " ".join(DEFAULT_GLOBS))
    ap.add_argument("--self-test", action="store_true", help="run embedded unit tests and exit")
    args = ap.parse_args()

    if args.self_test:
        sys.exit(self_test())
    if not args.old_commit or not args.new_commit:
        ap.error("old-commit and new-commit are required (or use --self-test)")

    repo_root = repo_toplevel()
    for commit in (args.old_commit, args.new_commit):
        if run_git(repo_root, ["rev-parse", "--verify", commit + "^{commit}"]).returncode != 0:
            sys.exit("error: unknown commit %r" % commit)

    patterns = args.files if args.files else DEFAULT_GLOBS
    rows, problems = process(repo_root, args.old_commit, args.new_commit,
                             patterns, args.apply)
    print_table(rows)
    n_changed = sum(1 for r in rows if r[2] not in ("(unchanged)", AMBIGUOUS, INVALID)
                    and not r[2].startswith("ALREADY-STALE"))
    print()
    print("%d ref(s) scanned, %d remapped, %d problem(s)%s"
          % (len(rows), n_changed, len(problems),
             " [applied]" if args.apply else " [dry run]"))
    if problems:
        print("problem refs (marked with ?stale%s):"
              % ("" if args.apply else " if --apply is used"))
        for loc in problems:
            print("  " + loc)
        sys.exit(1)
    sys.exit(0)


if __name__ == "__main__":
    main()
