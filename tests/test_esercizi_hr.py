"""Pratica HR exercises (teoria/pratica/esercizi-hr.js, game hr-nt): masses recomputed with mzlab.chem.elements, deterministic generators, no verdicts in the aids."""
import json
import re
import shutil
import subprocess
from pathlib import Path

import pytest

from mzlab.chem.elements import ADDUCT_SHIFT, mass, parse_formula

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node non installato")


def run_hr(js: str):
    prog = ("const fs=require('fs');global.localStorage={_d:{},getItem(k){return this._d[k]||null},setItem(k,v){this._d[k]=v}};"
            f"eval(fs.readFileSync({json.dumps(str(WEB / 'elements.js'))},'utf8').replace('const ELEMENTS','global.ELEMENTS'));"
            f"global.PAL=require({json.dumps(str(WEB / 'teoria' / 'pratica' / 'motore.js'))});"
            f"const H=require({json.dumps(str(WEB / 'teoria' / 'pratica' / 'esercizi-hr.js'))});"
            f"console.log(JSON.stringify((()=>{{{js}}})()));")
    out = subprocess.run(["node", "-e", prog], capture_output=True, encoding="utf-8", timeout=180)
    assert out.returncode == 0, out.stderr
    return json.loads(out.stdout)


@pytest.mark.parametrize("level", [3])
def test_nt_generator_is_deterministic_with_one_surviving_formula(level):
    r = run_hr(f"const a=H.genNt({level},777),b=H.genNt({level},777);const es=[];for(let s=1;s<=25;s++)es.push(H.genNt({level},s));return {{same:JSON.stringify(a)===JSON.stringify(b),es}}")
    assert r["same"]
    for e in r["es"]:
        cs = e["cands"]
        assert 6 <= len(cs) <= 7
        assert [c["id"] for c in cs] == list("ABCDEFG")[:len(cs)]
        assert [c["truth"] for c in cs].count(True) == 1
        assert [c for c in cs if not c["fails"]] == [c for c in cs if c["truth"]]          # only the generating formula passes every test
        assert len({t for c in cs for t in c["fails"]}) >= 3
        for c in cs:
            f = c["f"]
            m = mass(parse_formula(c["fs"])) + ADDUCT_SHIFT["[M+H]+"]
            assert abs(m - c["mzT"]) < 2e-4 and abs((e["mz"] - m) / m * 1e6) <= e["tolPpm"] + 0.1
            dbe = f["C"] - f["H"] / 2 + f.get("N", 0) / 2 + 1
            assert abs(dbe - c["dbe"]) < 1e-9
        assert e["cRange"][0] <= next(c for c in cs if c["truth"])["nC"] <= e["cRange"][1]


def test_nt_aids_do_not_name_the_answer():
    r = run_hr("const o=[];for(let s=1;s<=15;s++){const e=H.genNt(3,s);o.push({help:e.help.join(' '),ids:e.cands.map(c=>c.fs),t:e.truthFormula})}return o")
    for e in r:
        assert e["t"] not in e["help"]
        assert "giusta" not in e["help"] and "scartare" not in e["help"]


def test_fine_structure_adds_up_to_unit_resolution_pattern():
    r = run_hr("const o=[];for(let s=1;s<=60;s++){const e=H.genFine(1,s);o.push({fs:e.fs,sums:e.sums,unit:e.unit,mz0:e.mz0,peaks:e.peaks,nS:e.nS,nN:e.nN,help:e.help.join(' ')})}return o")
    for e in r:
        for i in (1, 2):
            assert abs(e["sums"][i] - e["unit"][i]) < 0.01
        f = parse_formula(e["fs"])
        ion = dict(f); ion["H"] += 1
        assert abs(mass(f) + ADDUCT_SHIFT["[M+H]+"] - e["mz0"]) < 2e-4
        assert e["nS"] == f.get("S", 0) and e["nN"] == f.get("N", 0)
        labels = {p["label"] for p in e["peaks"]}
        assert "13C" in labels
        assert e["nN"] == 0 or "15N" in labels
        ids = [p["id"] for p in e["peaks"]]
        assert ids == [f"p{i + 1}" for i in range(len(ids))]
        assert [p["mz"] for p in e["peaks"]] == sorted(p["mz"] for p in e["peaks"])      # ids follow m/z, never the label
        assert not re.search(rf"\b{e['nS']} atom", e["help"])


def test_fine_mass_differences():
    r = run_hr("const o={};['13C','15N','2H','33S','34S','18O'].forEach(l=>{o[l]=H.subOf(l).dm});return o")
    from mzlab.chem.elements import MASS
    assert abs(r["13C"] - 1.0033548) < 2e-6 and abs(r["15N"] - 0.9970349) < 2e-6
    assert abs(r["34S"] - 1.9957959) < 2e-6 and abs(r["18O"] - 2.0042449) < 2e-6
    assert MASS["C"] == 12.0
