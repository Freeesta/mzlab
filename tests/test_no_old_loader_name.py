"""The lazy script loader is `loadScript`, never the old Italian `carica`: a call left behind throws in the page the first time it runs."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD = re.compile(r"(?<![\w/-])carica(?![\w-])(?!\.js)")
AREAS = ["mzlab/web", "TP_Mine", "flow", "tools/lab", "tests", "tests_e2e", "docs/agenti"]
KINDS = {".js", ".py", ".md", ".html"}


def test_no_call_to_the_old_loader_name():
    left = []
    for area in AREAS:
        for f in (ROOT / area).rglob("*"):
            if f.suffix not in KINDS or "vendor" in f.parts or f == Path(__file__).resolve() or not f.is_file():
                continue
            for n, line in enumerate(f.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
                if OLD.search(line):
                    left.append(f"{f.relative_to(ROOT)}:{n}")
    assert not left, left
