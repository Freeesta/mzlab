"""Margin plots of the 2D map (mzlab/web/mappa-margini.js): bitmap follows the canvas' own box, also hidden -> shown; verified in Node by tests/mappa_margini_node.js."""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def test_mappa_margini():
    r = subprocess.run(["node", str(ROOT / "tests/mappa_margini_node.js"), str(ROOT / "mzlab/web/mappa-margini.js")], capture_output=True, encoding="utf-8")
    assert r.returncode == 0 and r.stdout.strip() == "ok", r.stderr[-1500:]
