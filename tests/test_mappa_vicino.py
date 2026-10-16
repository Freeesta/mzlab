"""Frecce ◀ ▶ della mappa (mzlab/web/mappa-vicino.js): file precedente/successivo con salto del riferimento, verificato in Node da tests/mappa_vicino_node.js."""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def test_mappa_vicino():
    r = subprocess.run(["node", str(ROOT / "tests/mappa_vicino_node.js"), str(ROOT / "mzlab/web/mappa-vicino.js")], capture_output=True, encoding="utf-8")
    assert r.returncode == 0 and r.stdout.strip() == "ok", r.stderr[-1500:]
