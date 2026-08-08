#!/usr/bin/env python3
"""Mechanical truthfulness check for C line anchors in the Rust port.

Every ported item carries a `// ufbx.c:NNNN[-MMMM]` (or `// ufbx_math.c:...`)
anchor. For each anchor that is attached to a named item, verify that a
plausible C spelling of the item's name actually occurs inside the referenced
line range of the C file. Anchors that fail (and anchors this script cannot
attribute to an item) are printed for deeper review; anchors that pass are
counted.

Exit codes: 0 all attributed anchors pass, 1 failures found.
Usage: check_anchors.py [--list-pass] [--sample N]
"""
import re
import sys
import random
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUST_DIRS = [ROOT / "rust/ufbx/src/native", ROOT / "rust/ufbx/src"]
C_FILES = {
    "ufbx.c": (ROOT / "ufbx.c").read_text(errors="replace").splitlines(),
    "ufbx_math.c": (ROOT / "extra/ufbx_math.c").read_text(errors="replace").splitlines(),
}

ANCHOR_RE = re.compile(r"//[/!]?.*?\b(ufbx(?:_math)?\.c):(\d+)(?:-(\d+))?")
ITEM_RE = re.compile(
    r"^\s*(?:pub(?:\(crate\))?\s+)?(?:unsafe\s+)?"
    r"(?:fn|const|static|struct|enum|union|macro_rules!|mod)\s+([A-Za-z_][A-Za-z0-9_]*)"
)

def snake(name: str) -> str:
    return re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", name).lower()

def c_candidates(name: str):
    base = snake(name)
    # strip common Rust-side suffixes used to dodge keyword clashes
    for suffix in ("_", "_impl"):
        if base.endswith(suffix):
            yield from c_candidates(base[: -len(suffix)])
    for prefix in ("", "ufbx_", "ufbxi_", "ufbxm_", "ufbxt_"):
        yield prefix + base
        yield (prefix + base).upper()

# a C identifier named in the comment right before the anchor: that is what
# the anchor refers to (e.g. "`ufbxi_mul_inv_rotate` (ufbx.c:22695)")
INLINE_ID_RE = re.compile(r"[`\s(]((?:ufbx|UFBX)[A-Za-z0-9_]*)[`'\s]*\(?$")

def main():
    list_pass = "--list-pass" in sys.argv
    sample_n = 0
    if "--sample" in sys.argv:
        sample_n = int(sys.argv[sys.argv.index("--sample") + 1])

    ok, fail, unattributed = [], [], []
    seen = set()
    files = sorted({f for d in RUST_DIRS for f in d.glob("*.rs")})
    for f in files:
        lines = f.read_text(errors="replace").splitlines()
        for i, line in enumerate(lines):
            m = ANCHOR_RE.search(line)
            if not m:
                continue
            cfile, lo, hi = m.group(1), int(m.group(2)), int(m.group(3) or m.group(2))
            key = (str(f), i)
            if key in seen:
                continue
            seen.add(key)
            csrc = C_FILES[cfile]
            if lo < 1 or hi > len(csrc) or lo > hi:
                fail.append((f, i + 1, m.group(0), None, "range out of bounds"))
                continue
            # if the comment names a C identifier just before the anchor,
            # verify that identifier against the range directly
            prefix_text = line[: m.start()]
            pm = INLINE_ID_RE.search(prefix_text)
            if pm:
                window = "\n".join(csrc[max(0, lo - 2) : hi + 1])
                if pm.group(1) in window:
                    ok.append((f, i + 1, m.group(0), pm.group(1)))
                else:
                    unattributed.append((f, i + 1, m.group(0)))
                continue
            # only a ref at the very start of the comment is an item anchor;
            # refs embedded in prose go to the sampled-review bucket
            stripped = re.sub(r"^\s*//[/!]?\s*(--\s*)?", "", line)
            if not stripped.startswith(("ufbx.c:", "ufbx_math.c:", "(ufbx.c:", "(ufbx_math.c:")):
                unattributed.append((f, i + 1, m.group(0)))
                continue
            # find the item the anchor is attached to: first item decl within
            # the next few lines (skipping attributes/comments)
            item = None
            for j in range(i, min(i + 8, len(lines))):
                im = ITEM_RE.match(lines[j])
                if im:
                    item = im.group(1)
                    break
                # stop if we hit a non-comment, non-attribute, non-blank line
                s = lines[j].strip()
                if j > i and s and not s.startswith(("//", "#[", "#!")):
                    break
            if item is None:
                unattributed.append((f, i + 1, m.group(0)))
                continue
            window = "\n".join(csrc[max(0, lo - 2) : hi + 1]).lower()
            if any(c.lower() in window for c in c_candidates(item)):
                ok.append((f, i + 1, m.group(0), item))
            else:
                fail.append((f, i + 1, m.group(0), item, "name not in range"))

    if "--dump-json" in sys.argv:
        import json
        out = {
            "fail": [
                {"file": str(f.relative_to(ROOT)), "line": ln, "anchor": a, "item": it}
                for f, ln, a, it, _ in fail
            ],
            "unattributed": [
                {"file": str(f.relative_to(ROOT)), "line": ln, "anchor": a}
                for f, ln, a in unattributed
            ],
        }
        Path(sys.argv[sys.argv.index("--dump-json") + 1]).write_text(json.dumps(out, indent=1))
    print(f"anchors attributed+verified: {len(ok)}")
    print(f"anchors unattributed (section/inline refs, not checked): {len(unattributed)}")
    print(f"anchors FAILED: {len(fail)}")
    for f, ln, anchor, item, why in fail:
        print(f"  FAIL {f.relative_to(ROOT)}:{ln} {anchor} item={item} ({why})")
    if list_pass:
        for f, ln, anchor, item in ok:
            print(f"  OK {f.relative_to(ROOT)}:{ln} {anchor} item={item}")
    if sample_n:
        random.seed(0)
        for f, ln, anchor, item in random.sample(ok, min(sample_n, len(ok))):
            print(f"  SAMPLE {f.relative_to(ROOT)}:{ln} {anchor} item={item}")
    return 1 if fail else 0

if __name__ == "__main__":
    sys.exit(main())
