"""Guard: the Browse input must not carry a long `accept` list (macOS resolves each entry
to a type and the file dialog opens very slowly). Unknown files are rejected by the intake."""
import re
from pathlib import Path

HTML = (Path(__file__).resolve().parents[1] / "mzlab" / "web" / "index.html").read_text(encoding="utf-8")


def test_pick_input_has_no_long_accept():
    tag = re.search(r'<input[^>]*\bid="pick"[^>]*>', HTML).group(0)
    m = re.search(r'\baccept="([^"]*)"', tag)
    entries = [e for e in (m.group(1).split(",") if m else []) if e.strip()]
    assert len(entries) <= 3, tag
    assert "multiple" in tag
