"""Axis and trace of the small XIC of the magnifier (mzlab/web/xic-lente-disegno.js): whole, evenly spaced ticks, at least 4; a narrow peak is not lost (tests/xic_lente_disegno_node.js)."""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def test_xic_lens_drawing():
    r = subprocess.run(["node", str(ROOT / "tests/xic_lente_disegno_node.js"), str(ROOT / "mzlab/web/xic-lente-disegno.js")], capture_output=True, encoding="utf-8")
    assert r.returncode == 0 and r.stdout.strip() == "ok", r.stderr[-1500:]
