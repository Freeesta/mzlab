"""site/theory/index.html (build_site.theory_entry): short address of the Teoria, forwards the hash to the real page."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import build_site  # noqa: E402


def test_theory_entry_forwards_with_hash():
    html = build_site.theory_entry()
    assert 'location.replace("../static/teoria/index.html"+location.hash)' in html
    assert 'http-equiv="refresh"' in html and "<title>Teoria" in html
    assert "pyodide" not in html.lower() and "<script src" not in html
