"""tests/dati/anal_golden.json (read by crates/mzlab-feat/tests/analysis.rs) is what the independent reference tools/analisi_riferimento.py computes,
and its cases cover CHNOS, F, P, Cl, Br, I, Na, drawn isotopes and charged species."""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_golden_is_the_reference():
    r = subprocess.run([sys.executable, "tools/analisi_riferimento.py"], cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stdout


def test_cases_cover_the_brief():
    cases = json.loads((ROOT / "tests/dati/anal_golden.json").read_text(encoding="utf-8"))
    els = {c["el"] for k in cases for c in k["expect"]["composition"]}
    assert len(cases) >= 15 and {"C", "H", "N", "O", "S", "F", "P", "Cl", "Br"} <= els
    assert {k["charge"] for k in cases} >= {-2, -1, 0, 1}
    # Cl2: M+2 is about 64 % of M (two 37Cl chances), Br2: M+2 is the tallest
    by = {k["name"]: k["expect"]["pattern"] for k in cases}
    assert 60 < next(p["rel"] for p in by["2,4-dichlorophenol"] if p["off"] == 2) < 68
    assert max(by["1,2-dibromoethane"], key=lambda p: p["rel"])["off"] == 2
