"""site/theory/index.html (build_site.theory_entry): the old address of the Teoria forwards to the Teoria site (BasePeak)."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import build_site  # noqa: E402


def test_theory_entry_forwards_to_basepeak():
    html = build_site.theory_entry()
    assert 'http-equiv="refresh" content="0;url=https://freeesta.github.io/BasePeak/"' in html
    assert "<title>Teoria" in html and "static/teoria" not in html
    assert "<script" not in html.lower()
