"""The notice box class is `notice`, never the old Italian `avv`: a rename left half done shows an unstyled box in the page."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OLD = re.compile(r"(?<![\w-])\.?avv(?![\w-])")
AREAS = [ROOT / "mzlab" / "web", ROOT / "tests", ROOT / "tests_e2e", ROOT / "docs" / "agenti"]
KINDS = {".js", ".css", ".html", ".py", ".md"}


def test_the_old_notice_class_is_gone():
    left = []
    for area in AREAS:
        for f in area.rglob("*"):
            if f.suffix not in KINDS or "vendor" in f.parts or f == Path(__file__).resolve() or not f.is_file():
                continue
            for n, line in enumerate(f.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
                if OLD.search(line):
                    left.append(f"{f.relative_to(ROOT)}:{n}")
    assert not left, left
