"""Correzione del fondo (mzlab/web/fondo.js): proprietà verificate in Node da tests/fondo_node.js."""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def test_fondo_proprieta():
    r = subprocess.run(["node", str(ROOT / "tests/fondo_node.js"), str(ROOT / "mzlab/web/fondo.js"), str(ROOT / "mzlab/web/fondo-spettro.js")], capture_output=True, encoding="utf-8")
    assert r.returncode == 0 and r.stdout.strip() == "ok", r.stderr[-1500:]
