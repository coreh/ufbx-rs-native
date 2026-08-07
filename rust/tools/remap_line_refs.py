#!/usr/bin/env python3
"""Remap stale C source line references after an upstream merge.

The Rust port and docs contain anchor comments like `ufbx.c:1234`,
`ufbx.c:1234-1301`, `ufbx.h:102-109` referencing lines in the upstream C
sources. When upstream changes, this tool remaps those line numbers using
the hunk headers of `git diff --unified=0 <old> <new> -- <file>`.

Usage:
    python3 rust/tools/remap_line_refs.py <old-commit> <new-commit> [--apply]
            [--files GLOB...] [--init-anchor] [--force]
    python3 rust/tools/remap_line_refs.py --self-test

Modes:
    default   dry run: print a table of every ref and its mapping; never
              touches the working tree (not even the anchor marker)
    --apply   rewrite refs in place; append `?stale` after ambiguous/invalid
              refs and `?review` after refs whose target line text changed
              (both greppable, harmless inside comments)

IMPORTANT: a number carrying `?stale` or `?review` still refers to the
PREVIOUS base commit -- it was NOT remapped onto the new one (`?review` was
remapped but the text underneath moved). Resolve every marker by hand
(`grep -rn '?stale\\|?review'`) BEFORE running the next sync, otherwise the
following run remaps an already-outdated number.

Anchor-base guard:
    UPSTREAM.md carries a marker line `<!-- line-ref-anchor-base: <sha> -->`
    recording the commit the refs in the tree are anchored to. `--apply`
    refuses to run unless `<old-commit>` resolves to that sha (this is what
    prevents a double-apply from silently corrupting every ref). On success
    the marker is rewritten to `<new-commit>`. If no marker exists yet,
    `--apply --init-anchor` records one. `--force` overrides the guard.

Atomicity:
    every source file is read and every rewrite computed in memory before
    anything is written; any abort leaves the working tree untouched.

Exit codes:
    0  clean: every ref mapped, no markers present
    1  stale/review markers present (or skipped conflicted files)
    2  aborted before writing anything (bad args, guard failure, undecodable
       or merge-conflicted file under --apply, wrong-commit guardrail)
    3  diff-parse sanity failure (blobs differ but no hunks were parsed)

Mapping rules (per C file, hunks sorted by old start line):
    - lines before a hunk keep the cumulative offset accumulated so far
    - lines after a hunk accumulate offset += (new_count - old_count)
    - lines falling inside a removed/replaced old range are AMBIGUOUS
    - pure insertions (old_count == 0, "after line a") make no line ambiguous
    - a range ref is ambiguous if either endpoint is ambiguous
    - a ref beyond EOF of the old commit's file is INVALID
    - a mapped endpoint whose line text differs between old and new is REVIEW
    - a range with a hunk strictly between its endpoints is OK-REVIEW: the
      numbers are remapped and no marker is written, but the region is listed
      so the porter can re-read it
"""

import argparse
import glob
import os
import re
import subprocess
import sys

# A ref, optionally already carrying a `?stale` / `?review` marker (with an
# optional numeric/dashed suffix such as `?stale-2`).
REF_RE = re.compile(
    r"\b(ufbx\.[ch]):(\d+)(?:-(\d+))?(\s*\?(?:stale|review)[-\d]*)?"
)
MARKER_TOKEN_RE = re.compile(r"\?(?:stale|review)[-\d]*")
CODE_SPAN_RE = re.compile(r"`[^`]*`")
HUNK_RE = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")
CONFLICT_RE = re.compile(r"^<<<<<<< ", re.MULTILINE)
ANCHOR_RE = re.compile(r"<!--\s*line-ref-anchor-base:\s*([0-9a-fA-F]+)\s*-->")

ANCHOR_FILE = "UPSTREAM.md"
DEFAULT_GLOBS = ["rust/**/*.rs", "PORTING.md", "UPSTREAM.md"]

AMBIGUOUS = "AMBIGUOUS"
INVALID = "INVALID"
REVIEW = "REVIEW"
OK_REVIEW = "OK-REVIEW"
ALREADY_MARKED = "ALREADY-MARKED"

INVALID_ABORT_RATIO = 0.25


class Abort(Exception):
    """Fatal, raised before anything is written."""

    def __init__(self, code, message):
        super().__init__(message)
        self.code = code
        self.message = message


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


def has_interior_hunk(start, end, hunks):
    """True if any hunk touches old lines strictly between `start` and `end`.

    Used to flag ranges whose *interior* changed even though both endpoints
    mapped cleanly: the numbers are fine, the content needs re-reading.
    """
    if end is None or end - start < 2:
        return False
    lo, hi = start + 1, end - 1  # inclusive interior
    for a, b, _c, _d in hunks:
        if b == 0:
            # Insertion after old line `a` lands inside the range when the
            # split point is strictly between the endpoints.
            if start <= a <= hi:
                return True
        else:
            if a <= hi and a + b - 1 >= lo:
                return True
    return False


def lines_match(old_lines, new_lines, old_n, new_n):
    """Content verification for one mapped endpoint.

    Returns True when the text is identical (or when it cannot be checked
    because a blob is unavailable / the index is out of range).
    """
    if old_lines is None or new_lines is None:
        return True
    if old_n is None or new_n is None:
        return True
    if not (1 <= old_n <= len(old_lines)) or not (1 <= new_n <= len(new_lines)):
        return True
    return old_lines[old_n - 1] == new_lines[new_n - 1]


def find_orphan_markers(line):
    """Return `?stale`/`?review` tokens on `line` not attached to a ref.

    Backtick-quoted spans are ignored: prose that *talks about* the markers
    (PORTING.md, UPSTREAM.md, doc comments) always quotes them, while the
    tool never writes a backtick around a marker it appends.
    """
    stripped = CODE_SPAN_RE.sub("", line)
    stripped = REF_RE.sub("", stripped)
    return MARKER_TOKEN_RE.findall(stripped)


# ---------------------------------------------------------------------------
# Anchor marker (pure text helpers; exercised by --self-test)
# ---------------------------------------------------------------------------

def read_anchor_base(text):
    m = ANCHOR_RE.search(text)
    return m.group(1) if m else None


def write_anchor_base(text, sha):
    """Return `text` with the anchor marker set to `sha` (inserting it if
    absent, right after the `Last fully ported` bullet or the first heading)."""
    marker = "<!-- line-ref-anchor-base: %s -->" % sha
    if ANCHOR_RE.search(text):
        return ANCHOR_RE.sub(lambda _m: marker, text, count=1)
    lines = text.splitlines(keepends=True)
    insert_at = None
    for i, line in enumerate(lines):
        if "Last fully ported upstream commit" in line:
            insert_at = i + 1
            break
    if insert_at is None:
        for i, line in enumerate(lines):
            if line.startswith("# "):
                insert_at = i + 1
                break
    if insert_at is None:
        insert_at = 0
    lines.insert(insert_at, marker + "\n")
    return "".join(lines)


def check_anchor_guard(recorded_sha, old_sha, init_anchor, force):
    """Return None if `--apply` may proceed, else an error message."""
    if force:
        return None
    if recorded_sha is None:
        if init_anchor:
            return None
        return ("no anchor base recorded in %s; the refs in the tree may be "
                "anchored to any commit. Re-run with --init-anchor to record "
                "%s as the base (or --force to override)." % (ANCHOR_FILE, old_sha))
    if init_anchor and recorded_sha != old_sha:
        return ("--init-anchor given but %s already records anchor base %s "
                "(you passed %s); drop --init-anchor or use --force."
                % (ANCHOR_FILE, recorded_sha, old_sha))
    if recorded_sha != old_sha:
        return ("anchor-base mismatch: %s records %s but <old-commit> resolves "
                "to %s. The refs in the tree are NOT anchored to the commit you "
                "passed -- applying would corrupt them (double-apply?). Use the "
                "recorded sha, or --force if you know better."
                % (ANCHOR_FILE, recorded_sha, old_sha))
    return None


# ---------------------------------------------------------------------------
# Git plumbing
# ---------------------------------------------------------------------------

def run_git(repo_root, args, binary=False):
    if binary:
        return subprocess.run(["git", "-C", repo_root] + args, capture_output=True)
    return subprocess.run(["git", "-C", repo_root] + args,
                          capture_output=True, text=True)


def repo_toplevel():
    here = os.path.dirname(os.path.abspath(__file__))
    result = subprocess.run(
        ["git", "-C", here, "rev-parse", "--show-toplevel"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        raise Abort(2, "not inside a git repository")
    return result.stdout.strip()


def blob_sha(repo_root, rev, path):
    r = run_git(repo_root, ["rev-parse", "--verify", "%s:%s" % (rev, path)])
    return r.stdout.strip() if r.returncode == 0 else None


def blob_lines(repo_root, rev, path):
    """Line list of `<rev>:<path>` (no trailing newlines), or None if absent."""
    r = run_git(repo_root, ["show", "%s:%s" % (rev, path)], binary=True)
    if r.returncode != 0:
        return None
    text = r.stdout.decode("utf-8", errors="replace")
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()  # trailing newline does not start a new line
    return lines


class FileMap(object):
    def __init__(self, hunks, old_lines, new_lines):
        self.hunks = hunks
        self.old_lines = old_lines
        self.new_lines = new_lines
        self.old_line_count = len(old_lines) if old_lines is not None else None


def load_file_mapping(repo_root, old, new, cfile):
    """Build the FileMap for one C file.

    Aborts (2) when the file is missing at `old` (wrong old commit) and
    aborts (3) when the blobs differ but no hunks could be parsed.
    """
    old_sha = blob_sha(repo_root, old, cfile)
    if old_sha is None:
        raise Abort(2, "%s does not exist at <old-commit> %s -- the old commit "
                       "is probably wrong" % (cfile, old))
    new_sha = blob_sha(repo_root, new, cfile)
    old_lines = blob_lines(repo_root, old, cfile)
    new_lines = blob_lines(repo_root, new, cfile) if new_sha else None

    diff = run_git(repo_root, [
        "-c", "color.ui=false",
        "diff", "--no-color", "--no-ext-diff", "--no-textconv",
        "--unified=0", old, new, "--", cfile,
    ])
    if diff.returncode not in (0, 1):
        raise Abort(2, "git diff failed for %s: %s" % (cfile, diff.stderr.strip()))
    hunks = parse_hunks(diff.stdout)
    if not hunks and new_sha is not None and old_sha != new_sha:
        raise Abort(3, "diff sanity check failed for %s: blob %s -> %s differ but "
                       "zero hunks were parsed from `git diff --unified=0` "
                       "(unexpected diff output; refusing to guess)"
                       % (cfile, old_sha[:12], new_sha[:12]))
    return FileMap(hunks, old_lines, new_lines)


# ---------------------------------------------------------------------------
# Source loading (pure enough to unit-test)
# ---------------------------------------------------------------------------

def collect_files(repo_root, patterns):
    files = []
    for pat in patterns:
        for path in sorted(glob.glob(os.path.join(repo_root, pat), recursive=True)):
            if os.sep + "target" + os.sep in path:
                continue
            if os.path.isfile(path):
                files.append(path)
    seen = set()
    out = []
    for f in files:
        if f not in seen:
            seen.add(f)
            out.append(f)
    return out


def load_sources(paths, apply_changes):
    """Read every candidate file up front.

    Returns (texts, warnings, skipped) where `texts` maps path -> str.
    Undecodable files and files carrying merge-conflict markers are skipped
    with a loud warning; under --apply either is fatal (Abort 2) so the tree
    is never half-rewritten and the anchor base is never advanced past refs
    that were not remapped.
    """
    texts = {}
    warnings = []
    skipped = []
    undecodable = []
    conflicted = []
    for path in paths:
        try:
            with open(path, "rb") as f:
                raw = f.read()
        except OSError as exc:
            raise Abort(2, "cannot read %s: %s" % (path, exc))
        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError as exc:
            undecodable.append(path)
            skipped.append(path)
            warnings.append("!!! SKIPPED (not valid UTF-8): %s (%s)" % (path, exc))
            continue
        if CONFLICT_RE.search(text):
            conflicted.append(path)
            skipped.append(path)
            warnings.append("!!! SKIPPED (unresolved merge conflict markers): %s"
                            % path)
            continue
        texts[path] = text
    if apply_changes and (undecodable or conflicted):
        raise Abort(2, "refusing to apply: %d file(s) could not be scanned "
                       "(%s%s). Resolve them first -- nothing was written."
                       % (len(skipped),
                          "not valid UTF-8: %s; " % ", ".join(undecodable)
                          if undecodable else "",
                          "merge conflict markers: %s" % ", ".join(conflicted)
                          if conflicted else ""))
    return texts, warnings, skipped


# ---------------------------------------------------------------------------
# Scan / apply
# ---------------------------------------------------------------------------

def format_ref(cfile, start, end):
    return "%s:%d" % (cfile, start) if end is None else "%s:%d-%d" % (cfile, start, end)


class Scan(object):
    def __init__(self):
        self.rows = []       # (loc, old_ref, result)
        self.problems = []   # (loc, ref_text, status)
        self.interior = []   # (loc, old_ref, new_ref)
        self.warnings = []
        self.new_texts = {}  # path -> rewritten text
        self.n_refs = 0
        self.n_invalid = 0
        self.n_remapped = 0


def scan_texts(repo_root, old, new, texts, apply_changes, mappings=None):
    """Compute every rewrite in memory. Writes nothing."""
    scan = Scan()
    mappings = {} if mappings is None else mappings

    for path in sorted(texts):
        text = texts[path]
        rel = os.path.relpath(path, repo_root)
        out_lines = []
        changed = False
        for lineno, line in enumerate(text.splitlines(keepends=True), 1):
            for orphan in find_orphan_markers(line):
                scan.warnings.append(
                    "!!! orphan marker `%s` not attached to a ref at %s:%d"
                    % (orphan, rel, lineno))

            def repl(m, lineno=lineno, rel=rel):
                nonlocal changed
                cfile, s, e, marked = (m.group(1), int(m.group(2)),
                                       m.group(3), m.group(4))
                e = int(e) if e is not None else None
                loc = "%s:%d" % (rel, lineno)
                old_ref = format_ref(cfile, s, e)
                scan.n_refs += 1
                if marked:
                    scan.rows.append((loc, old_ref + marked.strip(),
                                      "%s (resolve by hand)" % ALREADY_MARKED))
                    scan.problems.append((loc, old_ref + marked.strip(),
                                          ALREADY_MARKED))
                    return m.group(0)
                if cfile not in mappings:
                    mappings[cfile] = load_file_mapping(repo_root, old, new, cfile)
                fm = mappings[cfile]
                status, ns, ne = map_ref(s, e, fm.hunks, fm.old_line_count)
                if fm.new_lines is None:
                    status, ns, ne = (INVALID, None, None)

                if status != "ok":
                    if status == INVALID:
                        scan.n_invalid += 1
                    scan.rows.append((loc, old_ref, status))
                    scan.problems.append((loc, old_ref, status))
                    if apply_changes:
                        changed = True
                        return old_ref + "?stale"
                    return m.group(0)

                new_ref = format_ref(cfile, ns, ne)
                ok_start = lines_match(fm.old_lines, fm.new_lines, s, ns)
                ok_end = lines_match(fm.old_lines, fm.new_lines, e, ne)
                if not (ok_start and ok_end):
                    scan.rows.append((loc, old_ref, "%s -> %s" % (REVIEW, new_ref)))
                    scan.problems.append((loc, old_ref, REVIEW))
                    if apply_changes:
                        changed = True
                        return new_ref + "?review"
                    return m.group(0)

                if has_interior_hunk(s, e, fm.hunks):
                    scan.interior.append((loc, old_ref, new_ref))
                    scan.rows.append((loc, old_ref, "%s %s" % (new_ref, OK_REVIEW)))
                else:
                    scan.rows.append((loc, old_ref,
                                      new_ref if new_ref != old_ref else "(unchanged)"))
                if new_ref != old_ref:
                    scan.n_remapped += 1
                    if apply_changes:
                        changed = True
                        return new_ref
                return m.group(0)

            out_lines.append(REF_RE.sub(repl, line))
        if apply_changes and changed:
            scan.new_texts[path] = "".join(out_lines)
    return scan


def check_invalid_ratio(scan):
    if scan.n_refs and scan.n_invalid > INVALID_ABORT_RATIO * scan.n_refs:
        raise Abort(2, "%d of %d refs (%.0f%%) came out INVALID -- the old "
                       "commit is probably wrong. Nothing was written."
                       % (scan.n_invalid, scan.n_refs,
                          100.0 * scan.n_invalid / scan.n_refs))


def commit_writes(new_texts):
    for path, text in sorted(new_texts.items()):
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

def print_table(rows):
    if not rows:
        print("no line references found")
        return
    headers = ("REFERENCE AT", "OLD TARGET", "NEW TARGET")
    w0 = max([len(r[0]) for r in rows] + [len(headers[0])])
    w1 = max([len(r[1]) for r in rows] + [len(headers[1])])
    print("%-*s  %-*s  %s" % (w0, headers[0], w1, headers[1], headers[2]))
    for loc, old_ref, result in rows:
        print("%-*s  %-*s  %s" % (w0, loc, w1, old_ref, result))


# ---------------------------------------------------------------------------
# Self-test
# ---------------------------------------------------------------------------

def self_test():
    import tempfile

    failures = []

    def check(name, got, want):
        if got != want:
            failures.append("%s: got %r, want %r" % (name, got, want))

    # -- mapping math ------------------------------------------------------
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

    del_hunks = parse_hunks("@@ -7,3 +6,0 @@\n")
    check("before-deletion", map_line(6, del_hunks), ("ok", 6))
    check("inside-deletion", map_line(8, del_hunks), (AMBIGUOUS, None))
    check("after-deletion", map_line(10, del_hunks), ("ok", 7))

    one = parse_hunks("@@ -3 +3 @@\n")
    check("implicit-count", one, [(3, 1, 3, 1)])
    check("implicit-count-inside", map_line(3, one), (AMBIGUOUS, None))
    check("implicit-count-after", map_line(4, one), ("ok", 4))

    check("range-clean", map_ref(1, 4, hunks), ("ok", 1, 4))
    check("range-shifted", map_ref(7, 9, hunks), ("ok", 8, 10))
    check("range-straddling-start", map_ref(4, 6, hunks), (AMBIGUOUS, None, None))
    check("range-straddling-end", map_ref(6, 8, hunks), (AMBIGUOUS, None, None))
    check("range-spanning-hunk", map_ref(4, 8, hunks), ("ok", 4, 9))
    check("single-line-ref", map_ref(7, None, hunks), ("ok", 8, None))

    check("beyond-eof", map_line(51, hunks, old_line_count=50), (INVALID, None))
    check("at-eof", map_line(50, hunks, old_line_count=50), ("ok", 55))
    check("line-zero", map_line(0, hunks), (INVALID, None))
    check("range-with-invalid-end", map_ref(7, 51, hunks, 50), (INVALID, None, None))
    check("identity", map_line(123, []), ("ok", 123))

    # -- diff parsing hardening -------------------------------------------
    check("no-hunks-from-colored-noise",
          parse_hunks("\x1b[36m@@ -5,2 +5,3 @@\x1b[m\n"), [])
    check("hunk-with-context-heading",
          parse_hunks("@@ -12,3 +14,5 @@ static void ufbxi_foo(void)\n"),
          [(12, 3, 14, 5)])

    # -- interior-hunk (OK-REVIEW) categorization -------------------------
    inner = parse_hunks("@@ -20,2 +20,2 @@\n")
    check("interior-hit", has_interior_hunk(10, 30, inner), True)
    check("interior-miss-outside", has_interior_hunk(30, 40, inner), False)
    check("interior-endpoint-only",
          has_interior_hunk(21, 30, parse_hunks("@@ -21,1 +21,1 @@\n")), False)
    check("interior-single-line", has_interior_hunk(20, None, inner), False)
    check("interior-adjacent-range", has_interior_hunk(19, 20, inner), False)
    ins = parse_hunks("@@ -25,0 +26,3 @@\n")
    check("interior-insertion-inside", has_interior_hunk(20, 30, ins), True)
    check("interior-insertion-outside", has_interior_hunk(10, 20, ins), False)

    # -- content verification ---------------------------------------------
    old_src = ["a", "b", "target", "d"]
    new_src = ["a", "x", "y", "target", "d"]
    check("verify-match", lines_match(old_src, new_src, 3, 4), True)
    check("verify-mismatch", lines_match(old_src, new_src, 3, 3), False)
    check("verify-out-of-range", lines_match(old_src, new_src, 99, 1), True)
    check("verify-no-blob", lines_match(None, new_src, 1, 1), True)

    # -- ref regex hardening ----------------------------------------------
    def refs(s):
        return [(m.group(1), m.group(2), m.group(3),
                 (m.group(4) or "").strip()) for m in REF_RE.finditer(s)]

    check("re-plain", refs("// ufbx.c:12"), [("ufbx.c", "12", None, "")])
    check("re-range", refs("// ufbx.h:12-20"), [("ufbx.h", "12", "20", "")])
    check("re-stale", refs("// ufbx.c:12?stale"),
          [("ufbx.c", "12", None, "?stale")])
    check("re-review", refs("// ufbx.c:12-14?review"),
          [("ufbx.c", "12", "14", "?review")])
    check("re-stale-suffix", refs("// ufbx.c:12 ?stale-2"),
          [("ufbx.c", "12", None, "?stale-2")])
    check("re-orphan-not-consumed", find_orphan_markers("// ufbx.c:12?stale ?review"),
          ["?review"])
    check("re-orphan-alone", find_orphan_markers("// leftover ?stale here"),
          ["?stale"])
    check("re-no-orphan", find_orphan_markers("// ufbx.c:12-13 ?review"), [])
    check("re-orphan-ignores-prose",
          find_orphan_markers("refs get a `?stale` suffix and `?review`"), [])

    # -- anchor marker helpers --------------------------------------------
    doc = "# UPSTREAM.md\n\n- **Last fully ported upstream commit:** abc\n\nbody\n"
    check("anchor-absent", read_anchor_base(doc), None)
    doc1 = write_anchor_base(doc, "deadbeef")
    check("anchor-inserted", read_anchor_base(doc1), "deadbeef")
    check("anchor-insert-position", doc1.splitlines()[3],
          "<!-- line-ref-anchor-base: deadbeef -->")
    doc2 = write_anchor_base(doc1, "cafebabe")
    check("anchor-rewritten", read_anchor_base(doc2), "cafebabe")
    check("anchor-no-duplicate", doc2.count("line-ref-anchor-base"), 1)
    check("anchor-body-preserved", doc2.endswith("body\n"), True)

    # -- guard -------------------------------------------------------------
    check("guard-ok", check_anchor_guard("aaa", "aaa", False, False), None)
    check("guard-mismatch-blocks",
          check_anchor_guard("aaa", "bbb", False, False) is None, False)
    check("guard-mismatch-mentions-double-apply",
          "double-apply" in check_anchor_guard("aaa", "bbb", False, False), True)
    check("guard-force-overrides",
          check_anchor_guard("aaa", "bbb", False, True), None)
    check("guard-missing-blocks",
          check_anchor_guard(None, "aaa", False, False) is None, False)
    check("guard-missing-init-ok",
          check_anchor_guard(None, "aaa", True, False), None)
    check("guard-init-on-existing-mismatch-blocks",
          check_anchor_guard("aaa", "bbb", True, False) is None, False)
    check("guard-init-on-existing-match-ok",
          check_anchor_guard("aaa", "aaa", True, False), None)

    # -- source loading / atomicity ---------------------------------------
    with tempfile.TemporaryDirectory() as tmp:
        good = os.path.join(tmp, "a.rs")
        binf = os.path.join(tmp, "bin.rs")
        conf = os.path.join(tmp, "conflict.rs")
        with open(good, "w", encoding="utf-8") as f:
            f.write("// ufbx.c:10\n")
        with open(binf, "wb") as f:
            f.write(b"// ufbx.c:10\n\xff\xfe\x00garbage\n")
        with open(conf, "w", encoding="utf-8") as f:
            f.write("<<<<<<< HEAD\n// ufbx.c:10\n=======\n// ufbx.c:11\n>>>>>>> x\n")

        texts, warns, skipped = load_sources([good, binf, conf], apply_changes=False)
        check("load-good-only", sorted(texts), [good])
        check("load-skipped", sorted(skipped), sorted([binf, conf]))
        check("load-warn-count", len(warns), 2)
        check("load-warn-utf8", any("not valid UTF-8" in w for w in warns), True)
        check("load-warn-conflict", any("merge conflict" in w for w in warns), True)

        try:
            load_sources([good, binf], apply_changes=True)
            check("apply-aborts-on-undecodable", "no-abort", "Abort(2)")
        except Abort as exc:
            check("apply-aborts-on-undecodable", exc.code, 2)
        try:
            load_sources([good, conf], apply_changes=True)
            check("apply-aborts-on-conflict", "no-abort", "Abort(2)")
        except Abort as exc:
            check("apply-aborts-on-conflict", exc.code, 2)
        # The tree must be untouched by the aborted apply.
        with open(good, "r", encoding="utf-8") as f:
            check("atomicity-tree-untouched", f.read(), "// ufbx.c:10\n")

        # scan_texts must never write; commit_writes is the only writer.
        # old lines 5-6 replaced by 3 new lines: every line >= 7 shifts by +1
        # and keeps its text, so old 10 maps to new 11 with matching content.
        old_body = ["l%d" % i for i in range(1, 21)]
        new_body = old_body[:4] + ["n1", "n2", "n3"] + old_body[6:]
        fm = FileMap(parse_hunks("@@ -5,2 +5,3 @@\n"), old_body, new_body)
        with open(good, "w", encoding="utf-8") as f:
            f.write("// ufbx.c:10\n")
        scan = scan_texts(tmp, "old", "new", {good: "// ufbx.c:10\n"},
                          apply_changes=True, mappings={"ufbx.c": fm})
        check("scan-plans-write", scan.new_texts.get(good), "// ufbx.c:11\n")
        with open(good, "r", encoding="utf-8") as f:
            check("scan-does-not-write", f.read(), "// ufbx.c:10\n")
        commit_writes(scan.new_texts)
        with open(good, "r", encoding="utf-8") as f:
            check("commit-writes", f.read(), "// ufbx.c:11\n")

        # content mismatch -> REVIEW marker, number still remapped
        fm_bad = FileMap(parse_hunks("@@ -5,2 +5,3 @@\n"),
                         ["l%d" % i for i in range(1, 21)],
                         ["x%d" % i for i in range(1, 22)])
        scan = scan_texts(tmp, "old", "new", {good: "// ufbx.c:10\n"},
                          apply_changes=True, mappings={"ufbx.c": fm_bad})
        check("review-marker", scan.new_texts.get(good), "// ufbx.c:11?review\n")
        check("review-problem", [p[2] for p in scan.problems], [REVIEW])
        check("review-problem-has-ref-text", scan.problems[0][1], "ufbx.c:10")

        # interior hunk -> OK-REVIEW: remapped, no marker, listed separately
        interior_lines = ["l%d" % i for i in range(1, 41)]
        fm_int = FileMap(parse_hunks("@@ -20,1 +20,1 @@\n"),
                         interior_lines, list(interior_lines))
        scan = scan_texts(tmp, "old", "new", {good: "// ufbx.c:10-30\n"},
                          apply_changes=True, mappings={"ufbx.c": fm_int})
        check("ok-review-no-marker", scan.new_texts.get(good, "// ufbx.c:10-30\n"),
              "// ufbx.c:10-30\n")
        check("ok-review-not-a-problem", scan.problems, [])
        check("ok-review-listed", [i[1] for i in scan.interior], ["ufbx.c:10-30"])

        # ambiguous -> ?stale
        scan = scan_texts(tmp, "old", "new", {good: "// ufbx.c:5\n"},
                          apply_changes=True, mappings={"ufbx.c": fm})
        check("stale-marker", scan.new_texts.get(good), "// ufbx.c:5?stale\n")

        # already-marked refs are reported, never remapped again
        scan = scan_texts(tmp, "old", "new", {good: "// ufbx.c:10?stale\n"},
                          apply_changes=True, mappings={"ufbx.c": fm})
        check("already-marked-untouched", scan.new_texts.get(good), None)
        check("already-marked-problem", [p[2] for p in scan.problems],
              [ALREADY_MARKED])

        # wrong-commit guardrail on INVALID ratio
        fm_short = FileMap([], ["l1", "l2"], ["l1", "l2"])
        scan = scan_texts(tmp, "old", "new",
                          {good: "// ufbx.c:900 ufbx.c:901 ufbx.c:1\n"},
                          apply_changes=False, mappings={"ufbx.c": fm_short})
        try:
            check_invalid_ratio(scan)
            check("invalid-ratio-aborts", "no-abort", "Abort(2)")
        except Abort as exc:
            check("invalid-ratio-aborts", exc.code, 2)
            check("invalid-ratio-message", "old commit is probably wrong"
                  in exc.message, True)

    # -- table widths ------------------------------------------------------
    check("table-width-header",
          max([len("a:1")] + [len("REFERENCE AT")]), len("REFERENCE AT"))

    if failures:
        for f in failures:
            print("FAIL %s" % f)
        print("self-test: %d failure(s)" % len(failures))
        return 1
    print("self-test: all checks passed")
    return 0


# ---------------------------------------------------------------------------

def run(args):
    repo_root = repo_toplevel()

    resolved = {}
    for key, commit in (("old", args.old_commit), ("new", args.new_commit)):
        r = run_git(repo_root, ["rev-parse", "--verify", commit + "^{commit}"])
        if r.returncode != 0:
            raise Abort(2, "unknown commit %r" % commit)
        resolved[key] = r.stdout.strip()

    anchor_path = os.path.join(repo_root, ANCHOR_FILE)
    anchor_text = None
    recorded = None
    if os.path.isfile(anchor_path):
        with open(anchor_path, "r", encoding="utf-8") as f:
            anchor_text = f.read()
        recorded = read_anchor_base(anchor_text)

    if args.apply:
        err = check_anchor_guard(recorded, resolved["old"],
                                 args.init_anchor, args.force)
        if err:
            raise Abort(2, err)

    patterns = args.files if args.files else DEFAULT_GLOBS
    paths = collect_files(repo_root, patterns)
    texts, warnings, skipped = load_sources(paths, args.apply)

    scan = scan_texts(repo_root, args.old_commit, args.new_commit,
                      texts, args.apply)
    scan.warnings = warnings + scan.warnings
    check_invalid_ratio(scan)

    # Anchor marker bookkeeping (in memory; dry runs never touch it).
    if args.apply:
        if anchor_text is None:
            raise Abort(2, "%s not found at repo root; cannot record the anchor "
                           "base" % ANCHOR_FILE)
        base_text = scan.new_texts.get(anchor_path, anchor_text)
        scan.new_texts[anchor_path] = write_anchor_base(base_text, resolved["new"])

    if args.apply:
        commit_writes(scan.new_texts)

    # -- report ------------------------------------------------------------
    print_table(scan.rows)
    print()
    for w in scan.warnings:
        print(w)
    if scan.warnings:
        print()
    print("%d ref(s) scanned, %d remapped, %d problem(s), %d interior-changed"
          "%s" % (scan.n_refs, scan.n_remapped, len(scan.problems),
                  len(scan.interior),
                  " [applied]" if args.apply else " [dry run]"))
    if args.apply:
        print("anchor base recorded in %s: %s" % (ANCHOR_FILE, resolved["new"]))

    if scan.interior:
        print()
        print("interior changed -- re-read these regions (no marker written):")
        for loc, old_ref, new_ref in scan.interior:
            print("  %s  %s -> %s" % (loc, old_ref, new_ref))

    if scan.problems:
        print()
        print("problem refs (marked with ?stale/?review%s):"
              % ("" if args.apply else " if --apply is used"))
        for loc, ref_text, status in scan.problems:
            print("  %s  %s  [%s]" % (loc, ref_text, status))
        print("NOTE: a ?stale/?review number still refers to the PREVIOUS base "
              "commit; resolve every marker before the next sync.")

    if scan.problems or skipped:
        return 1
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("old_commit", nargs="?", help="upstream commit the refs currently anchor to")
    ap.add_argument("new_commit", nargs="?", help="upstream commit to remap the refs onto")
    ap.add_argument("--apply", action="store_true", help="rewrite refs in place (default: dry run)")
    ap.add_argument("--files", nargs="+", metavar="GLOB",
                    help="glob(s) relative to repo root (default: %s)" % " ".join(DEFAULT_GLOBS))
    ap.add_argument("--init-anchor", action="store_true",
                    help="record the anchor base in %s when none exists yet" % ANCHOR_FILE)
    ap.add_argument("--force", action="store_true",
                    help="override the anchor-base guard (dangerous)")
    ap.add_argument("--self-test", action="store_true", help="run embedded unit tests and exit")
    args = ap.parse_args()

    if args.self_test:
        sys.exit(self_test())
    if not args.old_commit or not args.new_commit:
        ap.error("old-commit and new-commit are required (or use --self-test)")

    try:
        sys.exit(run(args))
    except Abort as exc:
        sys.stderr.write("error: %s\n" % exc.message)
        sys.exit(exc.code)


if __name__ == "__main__":
    main()
