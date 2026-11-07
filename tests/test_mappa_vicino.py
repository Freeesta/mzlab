"""◀ ▶ arrows of the map (mzlab/web/mappa-vicino.js): previous/next file, skipping the reference, checked in Node by tests/mappa_vicino_node.js."""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def test_mappa_vicino():
    r = subprocess.run(["node", str(ROOT / "tests/mappa_vicino_node.js"), str(ROOT / "mzlab/web/mappa-vicino.js")], capture_output=True, encoding="utf-8")
    assert r.returncode == 0 and r.stdout.strip() == "ok", r.stderr[-1500:]
