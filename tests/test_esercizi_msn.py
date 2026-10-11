"""Pratica MSn exercise (teoria/pratica/msn-lcms.js, game lc-ms2): masses recomputed with mzlab.chem.elements, deterministic generator, no verdict in the aids."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

from mzlab.chem.elements import ADDUCT_SHIFT, mass, parse_formula

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node non installato")
LOSS = {"NH3": "NH3", "H2O": "H2O", "CO": "CO", "C2H4": "C2H4", "HCl": "HCl", "CH3OH": "CH4O", "HCOOH": "CH2O2"}


def run_msn(js: str):
    prog = ("const fs=require('fs');global.localStorage={_d:{},getItem(k){return this._d[k]||null},setItem(k,v){this._d[k]=v}};"
            f"eval(fs.readFileSync({json.dumps(str(WEB / 'elements.js'))},'utf8').replace('const ELEMENTS','global.ELEMENTS'));"
            f"global.PAL=require({json.dumps(str(WEB / 'teoria' / 'pratica' / 'motore.js'))});"
            f"const M=require({json.dumps(str(WEB / 'teoria' / 'pratica' / 'msn-lcms.js'))});"
            f"console.log(JSON.stringify((()=>{{{js}}})()));")
    out = subprocess.run(["node", "-e", prog], capture_output=True, encoding="utf-8", timeout=120)
    assert out.returncode == 0, out.stderr
    return json.loads(out.stdout)


@pytest.mark.parametrize("level", [1, 2, 3])
def test_msn_generator_is_deterministic_and_consistent(level):
    r = run_msn(f"const a=M.genMsn({level},99),b=M.genMsn({level},99);const es=[];for(let s=1;s<=40;s++){{const e=M.genMsn({level},s);es.push({{e,info:e.ions.map(x=>[x.id,M.infoOf(e,x),M.feasible(x)])}})}}return {{same:JSON.stringify(a)===JSON.stringify(b),es}}")
    assert r["same"]
    for item in r["es"]:
        e = item["e"]
        f = parse_formula(e["fs"])
        prec = mass(f) + ADDUCT_SHIFT["[M+H]+"]
        assert abs(prec - e["precMz"]) < 0.06
        assert len(e["groups"]) == (4 if level == 1 else 5)
        for ion in e["ions"]:
            loss = sum(mass(parse_formula(LOSS[g_loss(g)])) for g in ion["groups"])
            assert abs(prec - loss - ion["mz"]) < 0.06
        assert len({i["mz"] for i in e["ions"]}) == len(e["ions"])
        assert e["bestIds"] and set(e["bestIds"]) < {i["id"] for i in e["ions"] if i["kind"] == "single"}
        info = {i[0]: i for i in item["info"]}
        best = max(i[1] for i in item["info"] if i[2])
        assert all(info[b][1] == best and info[b][2] for b in e["bestIds"])
        if level >= 2:
            top = next(i for i in e["ions"] if i["abund"] == 100 and i["kind"] == "single")["id"]
            assert top not in e["bestIds"]                                  # the base peak is never the answer
            assert any(i["abund"] < e["threshold"] for i in e["ions"] if i["kind"] == "single")
        assert "help" in e and len(e["help"]) == 3


def g_loss(g):
    return {"NH2": "NH3", "OH": "H2O", "CO": "CO", "Et": "C2H4", "Cl": "HCl", "OMe": "CH3OH", "COOH": "HCOOH"}[g]


def test_msn_aids_do_not_name_an_ion():
    r = run_msn("const o=[];for(let s=1;s<=20;s++){const e=M.genMsn(2,s);o.push({h:e.help.join(' '),ids:e.bestIds})}return o")
    for e in r:
        assert not any(f"{i} " in e["h"] for i in e["ids"])
        assert "giusto" not in e["h"].lower()
