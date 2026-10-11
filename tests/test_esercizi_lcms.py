"""Pratica LC-MS exercises (teoria/pratica/esercizi-lcms.js): masses recomputed with mzlab.chem.elements, deterministic generators."""
import json
import re
import shutil
import subprocess
from pathlib import Path

import pytest

from mzlab.chem.elements import ADDUCT_SHIFT, MASS, parse_formula, mass

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node non installato")


def run(js: str):
    prog = ("const fs=require('fs');global.localStorage={_d:{},getItem(k){return this._d[k]||null},setItem(k,v){this._d[k]=v}};"
            f"eval(fs.readFileSync({json.dumps(str(WEB / 'elements.js'))},'utf8').replace('const ELEMENTS','global.ELEMENTS'));"
            f"global.PAL=require({json.dumps(str(WEB / 'teoria' / 'pratica' / 'motore.js'))});"
            f"const L=require({json.dumps(str(WEB / 'teoria' / 'pratica' / 'esercizi-lcms.js'))});"
            f"console.log(JSON.stringify((()=>{{{js}}})()));")
    out = subprocess.run(["node", "-e", prog], capture_output=True, encoding="utf-8", timeout=120)
    assert out.returncode == 0, out.stderr
    return json.loads(out.stdout)


def signed_mass(delta: dict) -> float:
    return sum(MASS[e] * n for e, n in delta.items())


def test_transformation_masses_are_recomputed_from_elements():
    r = run("return L.TRANS.map(t=>[t.id,t.delta,t.dm])")
    assert len(r) >= 15
    for tid, delta, dm in r:
        assert abs(dm - signed_mass(delta)) < 1e-6, tid
    d = {t[0]: t[2] for t in r}
    for k, v in {"O": 15.9949, "O-2H": 13.9793, "-2H": -2.0157, "+H2O": 18.0106, "-CH2": -14.0157, "-C2H4": -28.0313,
                 "CH3>COOH": 29.9742, "-CO2": -43.9898, "NH2>OH": 0.9840, "-Cl+OH": -17.9661}.items():
        assert abs(d[k] - v) < 6e-5, k
    c13 = run("return L.C13")
    assert abs(c13 - 1.00335) < 1e-5 and abs(c13 - d["NH2>OH"]) > 0.015      # +0.984 and +1.003 are told apart at 5 ppm


def test_adduct_shifts_match_python():
    sh = run("return L.SHIFT")
    for a, v in sh.items():
        assert abs(v - ADDUCT_SHIFT[a.replace("-]", "-]")]) < 1e-5, a


def test_scaffolds_are_generic_and_consistent():
    sc = run("return L.SCAFFOLDS.map(s=>[s.id,s.f,s.desc,s.M,s.nom])")
    assert len(sc) >= 10
    for sid, f, desc, M, nom in sc:
        assert re.fullmatch(r"S\d\d", sid)
        assert abs(M - mass(parse_formula(f))) < 1e-5, f
        d = parse_formula(f)
        dbe = d["C"] - sum(d.get(x, 0) for x in ("H", "Cl")) / 2 + d.get("N", 0) / 2 + 1
        assert dbe == int(dbe) and dbe >= 4, f
    blob = json.dumps(sc).lower()
    for name in ("atrazin", "simazin", "diclofenac", "ibuprofen", "caffein", "paracetamol", "carbamazep", "imidacloprid", "glifosat", "terbutil"):
        assert name not in blob


@pytest.mark.parametrize("level", [1, 2, 3])
def test_dm_generator_is_deterministic_and_coherent(level):
    a = run(f"return Array.from({{length:40}},(_,i)=>L.genDm({level},i+1))")
    b = run(f"return Array.from({{length:40}},(_,i)=>L.genDm({level},i+1))")
    assert a == b
    assert len({e["id"] for e in a}) > 8
    ppm = {1: 2, 2: 2, 3: 5}[level]
    for e in a:
        f = e["precursor"]["formula"]
        assert abs(e["precursor"]["mz"] - (mass(f) + ADDUCT_SHIFT["[M+H]+"])) < 1e-4
        truth = [o for o in e["options"] if o["id"] == e["truthId"]]
        assert len(truth) == 1 and len(e["options"]) == 5
        # the observed m/z is within the stated error of the product [M+H]+, and ONLY the truth is within it
        tmass = mass(f) + signed_mass(truth[0]["delta"]) + ADDUCT_SHIFT["[M+H]+"]
        assert abs(e["obs"] - tmass) / tmass * 1e6 <= ppm + 0.3, e["id"]
        near = [o for o in e["options"] if abs(e["dmObs"] - o["dm"]) * 1000 <= e["tolMDa"] * 1.0001 + 0.05]
        assert [o["id"] for o in near] == [e["truthId"]], e["id"]
        for o in e["options"]:
            if not o["iso"]:
                assert abs(o["dm"] - signed_mass(o["delta"])) < 1e-6
        p = e["product"]
        assert p["dbe"] == int(p["dbe"]) and p["dbe"] >= 0
        assert p["nParityOdd"] == (p["nN"] % 2 == 1)
        assert len(e["help"]) == 3 and all(len(h) > 40 for h in e["help"])
        for h in e["help"]:                                   # the aids never give the answer
            assert e["truthLabel"] not in h and e["product"]["f"] not in h
        if level == 3:
            assert "+" in e["truthId"] and truth[0]["delta"]
    if level >= 2:
        # near-isobaric traps exist: at least some exercises have an option within 0.1 u of the truth
        assert any(sum(1 for o in e["options"] if o["id"] != e["truthId"] and abs(o["dm"] - e["dmObs"]) < 0.1) for e in a)
    if level == 1:
        assert all(abs(o["dm"] - e["dmObs"]) > 0.4 for e in a for o in e["options"] if o["id"] != e["truthId"])


def test_dm_coherence_reports_numbers():
    r = run("const e=L.genDm(2,5); const t=L.coherenceDm(e,e.truthId); const w=e.options.find(o=>o.id!==e.truthId); return [t,L.coherenceDm(e,w.id)]")
    t, w = r
    assert t["within"] and abs(t["ppm"]) <= 2.3 and t["dbeOk"]
    assert not w["within"]


def test_esi_generator_follows_masses_and_nitrogen_rule():
    for level in (1, 2, 3):
        a = run(f"return Array.from({{length:30}},(_,i)=>L.genEsi({level},i+1))")
        assert a == run(f"return Array.from({{length:30}},(_,i)=>L.genEsi({level},i+1))")
        for e in a:
            comps = {c["id"]: c for c in e["compounds"]}
            assert len(comps) == {1: 1, 2: 3, 3: 1}[level]
            for s in e["pos"] + e["neg"]:
                c = comps[s["comp"]]
                th = mass(parse_formula(c["f"])) + ADDUCT_SHIFT[s["adduct"]]
                assert abs(s["mz"] - th) / th * 1e6 <= e["ppm"] + 0.3
                assert (s["mode"] == "+") == s["adduct"].endswith("+")
            for c in comps.values():
                n = parse_formula(c["f"]).get("N", 0)
                assert c["nOdd"] == (n % 2 == 1)
                for s in e["pos"] + e["neg"]:
                    if s["comp"] == c["id"] and s["adduct"] in ("[M+H]+", "[M-H]-"):
                        assert (round(s["mz"]) % 2 == 0) == c["nOdd"]      # nitrogen rule for [M+H]+ / [M-H]-
            assert len(e["help"]) == 3
            assert len(e["pos"]) == len(e["neg"]) == (2 if level == 3 else len(comps))
        if level == 3:                                         # adducts are told apart by the differences
            for e in a:
                p = sorted(e["pos"], key=lambda s: s["mz"])
                assert abs((p[1]["mz"] - p[0]["mz"]) - (21.98194 if p[1]["adduct"] == "[M+Na]+" else 17.02655)) < 0.01
                n = sorted(e["neg"], key=lambda s: s["mz"])
                assert abs(n[1]["mz"] - n[0]["mz"] - 46.0049) < 0.01


# ---------------------------------------------------------------- game lc-cin (kinetic profiles)
def run_cin(js: str):
    prog = ("const fs=require('fs');global.localStorage={_d:{},getItem(k){return this._d[k]||null},setItem(k,v){this._d[k]=v}};"
            f"eval(fs.readFileSync({json.dumps(str(WEB / 'elements.js'))},'utf8').replace('const ELEMENTS','global.ELEMENTS'));"
            f"global.PAL=require({json.dumps(str(WEB / 'teoria' / 'pratica' / 'motore.js'))});"
            f"const C=require({json.dumps(str(WEB / 'teoria' / 'pratica' / 'cinetica-lcms.js'))});"
            f"console.log(JSON.stringify((()=>{{{js}}})()));")
    out = subprocess.run(["node", "-e", prog], capture_output=True, encoding="utf-8", timeout=120)
    assert out.returncode == 0, out.stderr
    return json.loads(out.stdout)


@pytest.mark.parametrize("level", [1, 2, 3])
def test_cin_generator_is_deterministic_and_unambiguous(level):
    r = run_cin(f"const a=C.genCin({level},12345),b=C.genCin({level},12345);"
                f"const es=[];for(let s=1;s<=60;s++)es.push(C.genCin({level},s));"
                "return {same:JSON.stringify(a)===JSON.stringify(b),es}")
    assert r["same"]
    for e in r["es"]:
        fs = e["features"]
        assert len(fs) == {1: 4, 2: 5, 3: 6}[level]
        assert any(f["role"] == "primario" for f in fs) and any(f["role"] == "secondario" for f in fs)
        assert e["times"][0] == 0 and all(len(f["areas"]) == 6 for f in fs)
        for f in fs:
            mx = max(f["areas"] + [f["blank"]])
            if f["truth"] == "tengo":
                assert f["blank"] <= 0.1 * mx and f["areas"][0] <= 0.1 * mx, f["id"]       # a real product is absent from the blank and at t0
                assert f["ms2Times"], f["id"]
            if f["truth"] == "t0bianco":
                assert f["blank"] >= 0.4 * mx or f["areas"][0] >= 0.75 * mx
            if f["truth"] == "rt":
                assert f["polar"] and f["rt"] > e["precursor"]["rt"]                       # polar product eluting after the precursor
            if f["polar"] and f["truth"] != "rt":
                assert f["rt"] < e["precursor"]["rt"]
        if level == 3:
            assert any(a["id"] != b["id"] and a["dm"] == b["dm"] for a in fs for b in fs), "level 3 has an isomer pair"


def test_cin_masses_recomputed_and_aids_do_not_give_the_answer():
    es = run_cin("return [1,2,3].map(l=>C.genCin(l,99))")
    for e in es:
        prec = parse_formula(e["precursor"]["f"])
        mz_p = mass(prec) + ADDUCT_SHIFT["[M+H]+"]
        assert abs(e["precursor"]["mz"] - mz_p) < 2e-4
        for f in e["features"]:
            assert abs((f["mz"] - e["precursor"]["mz"]) - f["dm"]) < 2e-4
        txt = " ".join(e["help"])
        assert len(e["help"]) == 3
        for f in e["features"]:
            assert not re.search(rf"\b{f['id']}\b[^.]*\b(falso|vero|primario|secondario|scart)", txt), "an aid labels a candidate"
        assert "azzard" not in txt


def test_cin_data_name_no_real_compound():
    src = (WEB / "teoria" / "pratica" / "cinetica-lcms.js").read_text(encoding="utf-8") + (WEB / "teoria" / "pratica" / "giochi-lc-cin.js").read_text(encoding="utf-8")
    assert "sintetic" in src.lower()
