# TPMINE-PRIVATE
"""Fragmentation tree of the parent (fragtree.py): the same JSON for LR (supposed, from the MS2 of the parent) and HR (MSn tree), SMILES of fragments and
the comparison with a product. Synthetic data only."""
import json

import numpy as np
import pytest

import lc_synth as LS
import msn_synth as MS
from mzlab.chem import elements as E
from tpmine import api, chem, fragtree
from tpmine.demo import make_demo
from tpmine.engine import Experiment
from tpmine.hr import engine as EN
from tpmine.hr import formula as F
from tpmine.hr import molgraph as MG

LR_SMILES = "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"
HR_SMILES = "Cn1cnc2c1c(=O)n(C)c(=O)n2C"


def _formula(smiles):
    return F.fmt(F.vec(chem.smiles_formula(smiles), F.element_order(chem.smiles_formula(smiles))), F.element_order(chem.smiles_formula(smiles)))


# ----------------------------------------------------------------------------------------------------------------------------------- SMILES of fragments
@pytest.mark.parametrize("smi", [HR_SMILES, LR_SMILES, "c1ccccc1O", "CC(=O)Oc1ccccc1C(=O)O", "c1ccc2[nH]ccc2c1"])
def test_whole_molecule_roundtrip(smi):
    mol = MG.parse_smiles(smi)
    out = fragtree.mask_smiles(mol, (1 << mol.n) - 1)
    assert MG.parse_smiles(out).formula() == mol.formula(), out


def test_every_fragment_keeps_its_formula():
    mol = MG.parse_smiles(LR_SMILES)
    subs = MG.substructures(mol)
    for i in np.linspace(0, len(subs) - 1, 60).astype(int):
        out = fragtree.mask_smiles(mol, int(subs.masks[i]))
        f = {el: int(v) for el, v in zip(subs.els, subs.F[i]) if v}
        assert MG.parse_smiles(out).formula() == f, out


# ------------------------------------------------------------------------------------------------------------------------------------------------ LR
@pytest.fixture(scope="module")
def lr_exp(tmp_path_factory):
    files = make_demo(tmp_path_factory.mktemp("d"))
    ex = Experiment(files, {"name": "Parent", "neutral": LR_SMILES, "smiles": LR_SMILES})
    s = ex.run()
    return ex, s


def test_lr_tree_is_supposed_with_alternatives(lr_exp):
    ex, _ = lr_exp
    t = fragtree.lr(ex)
    assert t["ok"] and t["mode"] == "lr" and t["supposed"] and "unit resolution" in t["note"]
    root, *fr = t["nodes"]
    assert root["formula"] == "C10H13N2O3S" and abs(root["mz"] - 241.06) < 0.6 and root["parent"] is None
    f199 = next(n for n in fr if abs(n["mz"] - 199) < 0.6)
    assert f199["parent"] == root["id"] and f199["level"] == 2 and f199["rel"] > 5
    assert {"C3H6", "C2H2O"} <= {a["loss"] for a in f199["alternatives"] if a["source"] == "loss"}       # all the nominal alternatives are kept
    assert f199["loss"] in ("C3H6", "C2H2O")
    assert all(abs(a["err"]) <= 0.5 for n in fr for a in n["alternatives"]) and len(f199["alternatives"]) <= fragtree.MAX_ALT
    assert f199["smiles"] and all(MG.parse_smiles(x["smiles"]) for x in f199["smiles"])
    json.dumps(t)


def test_lr_compare_marks_shifted_fragment(lr_exp):
    ex, s = lr_exp
    oh = next(r for r in s["rows"] if r["name"] == "hydroxylation @ 7.40 min")
    t = fragtree.lr(ex)
    e = next(x for x in ex.entries if x["id"] == oh["id"])
    c = fragtree.compare_lr(t, ex.ms2(e), e["neutral_mass"] - ex.entries[0]["neutral_mass"])
    assert c["ok"] and c["n_shifted"] >= 1
    n199 = next(n for n in t["nodes"] if n["kind"] == "frag" and abs(n["mz"] - 199) < 0.6)
    assert c["marks"][n199["id"]]["state"] == "shifted" and c["marks"][n199["id"]]["delta"] == "+15.99"
    assert not fragtree.compare_lr(t, {"fragments": []}, 16.0)["ok"]


def test_api_entries_lr(lr_exp):
    ex, s = lr_exp
    api._ex, api._hr = ex, None
    t = json.loads(api.frag_tree())
    assert t["mode"] == "lr" and len(t["nodes"]) >= 2
    oh = next(r for r in s["rows"] if r["name"] == "hydroxylation @ 7.40 min")
    c = json.loads(api.frag_tree_compare(oh["id"]))
    assert c["n_shifted"] >= 1 and set(c["marks"]) <= {n["id"] for n in t["nodes"]}


# ------------------------------------------------------------------------------------------------------------------------------------------------ HR
@pytest.fixture(scope="module")
def hr_exp(tmp_path_factory):
    d = tmp_path_factory.mktemp("lc")
    files = LS.write_series(d)
    msn = MS.write_msn(d / "cafe_msn.mzML", MS.caffeine_nodes())
    e = EN.ExperimentHR(files + [{"name": "cafe_msn.mzML", "path": str(msn), "time": None, "type": "sample"}], {"smiles": HR_SMILES})
    e.run()
    return e


def test_hr_tree_structure(hr_exp):
    t = fragtree.hr(hr_exp)
    assert t["ok"] and t["mode"] == "hr" and not t["supposed"]
    nodes = t["nodes"]
    ids = [n["id"] for n in nodes]
    assert len(set(ids)) == len(ids)
    roots = [n for n in nodes if n["parent"] is None]
    assert len(roots) == 1 and roots[0]["formula"] == "C8H11N4O2" and roots[0]["rel"] == 100.0
    assert all(n["parent"] in ids for n in nodes if n["parent"] is not None)
    els = hr_exp.tree.els
    top = F.vec("C8H11N4O2", els)
    by = {n["id"]: n for n in nodes}
    for n in nodes:
        if n["formula"]:
            assert (F.vec(n["formula"], els) <= top).all()
        if n["loss"]:
            p = by[n["parent"]]
            assert (F.vec(p["formula"], els) - F.vec(n["formula"], els) == F.vec(n["loss"], els)).all()
            assert n["loss_mass"] == pytest.approx(E.mass(E.parse_formula(n["loss"])), abs=1e-3)
    assert any(n["kind"] == "frag" and n["formula"] == "C6H8N3O" for n in nodes)
    assert any(n["ghost"] for n in nodes)
    assert any(n["smiles"] for n in nodes if n["kind"] == "frag")
    assert all(len(n["smiles"]) <= fragtree.MAX_SMILES for n in nodes)
    json.dumps(t)


def test_hr_tree_without_msn_uses_dda_ms2(tmp_path):
    files = LS.write_series(tmp_path)
    e = EN.ExperimentHR(files, {"smiles": HR_SMILES})
    e.run()
    t = fragtree.hr(e)
    if e.parent_ms2 is None:
        assert not t["ok"]
    else:
        assert t["ok"] and not t["supposed"] and t["nodes"][0]["formula"] == "C8H11N4O2"


def test_hr_compare_uses_localisation(hr_exp):
    t = fragtree.hr(hr_exp)
    loc = {"evidence": [{"kind": "U", "library": "C6H8N3O"}, {"kind": "S", "library": "C5H8N3"}], "delta": "O", "delta_sign": "+"}
    c = fragtree.compare_hr(t, loc)
    states = {t["nodes"][[n["id"] for n in t["nodes"]].index(i)]["formula"]: m for i, m in c["marks"].items()}
    assert states["C6H8N3O"]["state"] == "same" and states["C5H8N3"] == {"state": "shifted", "delta": "+O"}
    assert not fragtree.compare_hr(t, None)["ok"]
    api._hr, api._ex = hr_exp, None
    assert json.loads(api.frag_tree())["mode"] == "hr"
    cid = next(c["id"] for c in hr_exp.cands if c.get("localization", {}).get("evidence"))
    assert json.loads(api.frag_tree_compare(cid))["ok"]
