"""PFAS evidence (mzlab/web/pfas-calc.js, run with node): MD/C and m/C of a candidate formula, the diagnostic fragments and neutral losses of an MS2 are counted
with their ppm error, an unrelated spectrum has none, the formula picked from the composition results needs fluorine."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def node(tmp_path, code: str):
    mod = tmp_path / "pfas-calc.mjs"                        # the page loads it as a module; node wants the extension
    mod.write_text((WEB / "pfas-calc.js").read_text(encoding="utf-8"), encoding="utf-8")
    out = subprocess.run(["node", "--input-type=module", "-e", f"import * as K from {mod.as_uri()!r};" + code], check=True, capture_output=True, text=True, encoding="utf-8").stdout
    return json.loads(out)


PFOA = 412.96640                                             # [M-H]- of C8HF15O2
F, E = 18.99840316273, 0.00054857990946
ANIONS = [round(n * 12 + (2 * n + 1) * F + E + e, 5) for n, e in ((1, 1e-5), (2, -2e-5), (3, 3e-5), (4, 0.0))]     # CnF(2n+1)-, a fraction of a ppm off
SPEC = {"mz": ANIONS + [368.97657, 250.1234], "y": [30, 40, 100, 60, 80, 90]}


def test_mass_defect_per_carbon(tmp_path):
    r = node(tmp_path, f"console.log(JSON.stringify([K.mdc({PFOA}, 8), K.mdc({PFOA}, 0), K.carbons('C8F15O2'), K.carbons('F3Cl2'), K.atoms('C8F15O2', 'F')]))")
    assert r[0]["mc"] == pytest.approx(51.6208, abs=1e-3) and r[0]["mdc"] == pytest.approx(-0.0336 / 8, abs=1e-5) and r[1] is None
    assert r[2:] == [8, 0, 15]


def test_fragments_and_losses_are_counted_with_their_error(tmp_path):
    r = node(tmp_path, f"console.log(JSON.stringify(K.search({json.dumps(SPEC)}, {PFOA}, -1, {{tolPpm: 5, minRel: 1}})))")
    assert [f["id"] for f in r["frags"]] == ["CF3", "C2F5", "C3F7", "C4F9"] and all(abs(f["ppm"]) < 2 for f in r["frags"]), r["frags"]
    assert [l["id"] for l in r["losses"]] == ["CO2"] and abs(r["losses"][0]["ppm"]) < 2, r["losses"]
    assert r["cf2"] == 3 and r["n"] == 6                                         # 68.9952 > 118.9920 > 168.9888 > 218.9856 are CF2 apart


def test_an_unrelated_spectrum_has_no_evidence_and_the_polarity_counts(tmp_path):
    other = {"mz": [101.0233, 155.5, 210.7, 288.1], "y": [100, 40, 30, 20]}
    r = node(tmp_path, f"console.log(JSON.stringify([K.search({json.dumps(other)}, {PFOA}, -1), K.search({json.dumps(SPEC)}, {PFOA}, 1).frags.length]))")
    assert r[0]["n"] == 0 and r[1] == 0, r
    weak = {"mz": SPEC["mz"], "y": [0.5, 0.5, 0.5, 0.5, 100, 90]}                 # the CF3 ... C4F9 peaks are under 1 % of the base peak
    r = node(tmp_path, f"console.log(JSON.stringify(K.search({json.dumps(weak)}, {PFOA}, -1)))")
    assert r["frags"] == [] and len(r["losses"]) == 1


def test_the_formula_needs_fluorine_and_the_smallest_error(tmp_path):
    res = [{"formula": "C24HN2O4S", "delta_ppm": -0.1}, {"formula": "C8F15O2", "delta_ppm": 0.3}, {"formula": "C10HF8N2O7", "delta_ppm": -0.6}]
    r = node(tmp_path, f"console.log(JSON.stringify([K.pickFormula({json.dumps(res)}), K.pickFormula([{json.dumps(res[0])}])]))")
    assert r[0] == {"formula": "C8F15O2", "ppm": 0.3, "n": 2} and r[1] is None
