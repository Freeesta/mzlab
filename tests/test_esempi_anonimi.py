"""The public example files stay small."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ESEMPI = ROOT / "mzlab" / "web" / "esempi"
MAX_BYTES = 10 * 1024 * 1024


def test_no_example_file_exceeds_10_mb():
    big = {p.name: p.stat().st_size for p in ESEMPI.iterdir() if p.is_file() and p.stat().st_size > MAX_BYTES}
    assert not big, big
