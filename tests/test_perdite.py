"""Neutral losses table (mzlab/web/perdite.js): the exact masses shown by the program are the ones computed by mzlab/chem/elements.py,
and the search for combinations of a mass difference (Cerca Δm) finds the right pairs and repetitions."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

from mzlab.chem.elements import parse_formula, formula_mz

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    js = f"""const fs=require('fs');const c=require({str(WEB / 'perdite.js')!r});
    const E=new Function(fs.readFileSync({str(WEB / 'elements.js')!r},'utf8')+';return ELEMENTS')();
    const EL=Object.fromEntries(E.map(e=>[e.s,e]));
    const mono=s=>EL[s].iso.reduce((a,b)=>b[2]>a[2]?b:a)[1];
    const mass=f=>{{let m=0;for(const [,s,n] of f.matchAll(/([A-Z][a-z]?)(\\d*)/g))m+=mono(s)*(n?+n:1);return m}};
    {code}"""
    return json.loads(subprocess.run(["node", "-e", js], check=True, capture_output=True, encoding="utf-8").stdout)


def neutral_mass(formula):
    return formula_mz(formula, "[M+H]+")["neutral_da"]


def test_masses_of_the_table_are_the_computed_ones():
    r = run("console.log(JSON.stringify(c.LOSSES.map(l=>[l.f,mass(l.f)])))")
    assert len(r) >= 20
    for f, m in r:
        assert abs(m - neutral_mass(f)) < 5e-4, (f, m, neutral_mass(f))     # the program and the Python side agree on every row


def test_nominal_groups_and_polarity_are_filled():
    r = run("console.log(JSON.stringify(c.LOSSES.map(l=>[l.f,Math.round(mass(l.f)),l.pol,!!l.seen,!!l.mech,!!l.rad])))")
    nominal = {}
    for f, n, pol, seen, mech, rad in r:
        assert pol in ("+", "-", "±") and seen and mech
        nominal.setdefault(n, []).append(f)
    assert sorted(nominal[28]) == ["C2H4", "CO"] and sorted(nominal[42]) == ["C2H2O", "C3H6"]
    assert sorted(nominal[46]) == ["CH2O2", "NO2"] and sorted(nominal[80]) == ["HBr", "SO3"]
    assert [f for f, *_ , rad in r if rad] == ["CH3", "Cl", "NO2"]              # the radical losses


def test_search_62_is_water_plus_co2():
    r = run("console.log(JSON.stringify(c.lossCombos(62,mass)))")
    assert ["H2O", "CO2"] in [[x["f"] for x in p] for p in r["pairs"]]


def test_search_72_is_two_hcl():
    r = run("console.log(JSON.stringify(c.lossCombos(72,mass)))")
    assert {"n": 2, "f": "HCl"} in [{"n": x["n"], "f": x["l"]["f"]} for x in r["reps"]]


def test_search_28_lists_both_isobaric_losses():
    r = run("console.log(JSON.stringify(c.lossCombos(28,mass)))")
    assert sorted(x[0]["f"] for x in r["single"]) == ["C2H4", "CO"]
