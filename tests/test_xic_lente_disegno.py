"""Asse e tracciato del piccolo XIC della lente (mzlab/web/xic-lente-disegno.js): tacche intere, equidistanti, almeno 4; il picco stretto non si perde (tests/xic_lente_disegno_node.js)."""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def test_xic_lente_disegno():
    r = subprocess.run(["node", str(ROOT / "tests/xic_lente_disegno_node.js"), str(ROOT / "mzlab/web/xic-lente-disegno.js")], capture_output=True, encoding="utf-8")
    assert r.returncode == 0 and r.stdout.strip() == "ok", r.stderr[-1500:]
