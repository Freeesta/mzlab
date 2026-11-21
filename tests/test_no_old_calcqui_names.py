"""The calculator's API and group ids are English (CALCQUI.groups/lists/losses, ids lists/losses/adducts); the Italian ones must not come back."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
FILES = ["../../docs/agenti/data-3200.md", "index.html", "calc-qui.js", "calc-qui-html.js", "calc-qui-ui.js", "calc-qui-pannello.js", "lang/en/calcqui.js"]
OLD = re.compile(r"\bgruppi\b|[\"'](liste|perdite|addotti)[\"']|cq\.g\.(liste|perdite|addotti)|\bfunction (liste|perdite|addotti)\b|\blisteRows\b")


def test_old_calculator_names_are_gone():
    hits = [f"{f}:{n}" for f in FILES for n, line in enumerate((WEB / f).read_text(encoding="utf8").splitlines(), 1) if OLD.search(line)]
    assert not hits, hits
