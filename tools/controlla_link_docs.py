"""Finds links and `backticked/paths` in the tracked .md files that point to a file that does not exist.

A path counts as broken only if it has a file extension and no tracked file ends with it (crate-relative
paths such as `tests/x.rs` match any crate). Historical documents (docs/studi, docs/archivio), test fixtures and
vendored READMEs are skipped, and so are generated files that are not tracked. Run from the repository root; prints
one line per document and exits 1 when something is broken (part of pre_pr)."""
import os
import re
import subprocess
import sys

SKIP = ("docs/studi/", "docs/archivio/", "tests/data/", "tools/libro/LEGGIMI.md", "vendor/", "mzlab/web/vendor")
# produced by a tool and not tracked (tools/indice.py)
GENERATED = {"docs/agenti/mappa.md"}
TICK = re.compile(r"`((?:docs|tools|crates|mzlab|tests|tests_e2e|TP_Mine|flow|fuzz|esperimenti|\.github|\.claude)/[A-Za-z0-9_./\-]+)`")
LINK = re.compile(r"\]\(([^)#\s]+)(?:#[^)]*)?\)")
EXT = re.compile(r"\.[a-z0-9]{1,5}$")


def main() -> int:
    tracked = [f for f in subprocess.check_output(["git", "ls-files"], text=True).split("\n") if f]
    have = set(tracked)
    broken = {}
    for f in tracked:
        if not f.endswith(".md") or f.startswith(SKIP):
            continue
        text = open(f, encoding="utf-8", errors="ignore").read()
        cands = [m.group(1).rstrip(".,:;") for m in TICK.finditer(text) if not any(c in m.group(1) for c in "*<>{}")]
        for m in LINK.finditer(text):
            p = m.group(1)
            if not p.startswith(("http", "mailto", "#")):
                q = os.path.normpath(os.path.join(os.path.dirname(f), p))
                if q not in have and EXT.search(q) and not os.path.exists(q):
                    broken.setdefault(f, set()).add(p)
        for p in cands:
            if EXT.search(p) and p not in have and p not in GENERATED and not any(t.endswith("/" + p) for t in tracked) and not os.path.exists(p):
                broken.setdefault(f, set()).add(p)
    for f, v in sorted(broken.items()):
        print(f, sorted(v))
    print(f"{sum(len(v) for v in broken.values())} broken in {len(broken)} documents")
    return 1 if broken else 0


if __name__ == "__main__":
    sys.exit(main())
