"""Guard: the Browse input keeps a SHORT accept list of plain lowercase extensions (no MIME types, no duplicates):
macOS resolves every entry to a system type, and a long or MIME-based list makes the file dialog open slowly."""
import re
from pathlib import Path

HTML = (Path(__file__).resolve().parents[1] / "mzlab" / "web" / "index.html").read_text(encoding="utf-8")


def test_pick_accept_is_short_lowercase_extensions():
    tag = re.search(r'<input[^>]*\bid="pick"[^>]*>', HTML).group(0)
    entries = re.search(r'\baccept="([^"]*)"', tag).group(1).split(",")
    assert 0 < len(entries) <= 8, entries
    assert len(set(entries)) == len(entries), entries
    assert all(re.fullmatch(r"\.[a-z0-9]+", e) for e in entries), entries
    assert {".mzml", ".mzxml", ".raw", ".wiff", ".scan"} <= set(entries)
    assert "multiple" in tag
