# TPMINE-PRIVATE
"""Parent-TP pairs (public, synthetic): the three components of the similarity (equal fragments, fragments shifted by the mass difference, common
neutral losses), the rules with their prediction level and the network of mass differences. No compound of the experience: caffeine and invented masses."""
import json

import numpy as np
import pytest

import lc_synth as LS
from tpmine import api, chem
from tpmine.hr import engine as EN
from tpmine.hr import formula as F
from tpmine.hr import network as NW
from tpmine.hr import rules as RU
from tpmine.hr import similarity as SM

O = 15.994915
SMILES = "Cn1cnc2c1c(=O)n(C)c(=O)n2C"


def spectra(delta=O, noise=0.0003, seed=1):
    """A parent of 300 and its product (+delta): two equal fragments, two shifted ones (far from the precursor) and two shifted losses (near it)."""
    rng = np.random.default_rng(seed)
    pm, pi = np.array([90.0, 120.0, 180.0, 210.0, 282.0, 270.0]), np.array([100, 70, 60, 50, 40, 30.0])
    tm = np.array([90.0, 120.0, 180.0 + delta, 210.0 + delta, 282.0 + delta, 270.0 + delta]) + rng.normal(0, noise, 6)
    return SM.prepare(pm, pi, 300.0), SM.prepare(tm, pi * 0.9, 300.0 + delta)


def test_components_type_the_pairs_and_add_up_to_the_totals():
    a, b = spectra()
    r = SM.pair_components(a, 300.0, b, 300.0 + O)
    assert r["n"] == {"equal": 2, "shifted": 2, "loss": 2}
    assert r["cos"]["equal"] > 0 and r["cos"]["shifted"] > 0 and r["cos"]["loss"] > 0
    M, W, P = SM.pad([a, b], [300.0, 300.0 + O])
    assert r["cos"]["total"] == pytest.approx(SM.modified_cosine(M, W, P, [0], [1])[0][0], abs=1e-9)
    assert r["ent"]["total"] == pytest.approx(SM.entropy_similarity(M, W, P, [0], [1])[0][0], abs=1e-9)
    assert {p["type"] for p in r["pairs"]} == {"equal", "shifted", "loss"} and len(r["pairs"]) == 6
    for p in r["pairs"]:
        assert abs(p["b"] - p["a"] - (0 if p["type"] == "equal" else O)) < 0.002


def test_a_product_that_keeps_every_fragment_has_no_shifted_ones():
    pm, pi = np.array([90.0, 120.0, 180.0, 210.0]), np.array([100, 70, 60, 50.0])
    a, b = SM.prepare(pm, pi, 300.0), SM.prepare(pm + 0.0002, pi, 300.0 + O)
    r = SM.pair_components(a, 300.0, b, 300.0 + O)
    assert r["n"] == {"equal": 4, "shifted": 0, "loss": 0} and r["cos"]["total"] == pytest.approx(r["cos"]["equal"]) and r["cos"]["equal"] > 0.99


def test_empty_spectra_and_unrelated_ones():
    e = (np.zeros(0), np.zeros(0))
    a, b = spectra()
    assert SM.pair_components(e, 300.0, b, 316.0)["cos"]["total"] == 0.0
    c = SM.prepare([55.0, 77.0, 133.0], [100, 50, 20.0], 316.0)
    assert SM.pair_components(a, 300.0, c, 316.0)["n"] == {"equal": 0, "shifted": 0, "loss": 0}


# ---------------------------------------------------------------------------------------------------------------------- rules
ELS = ["C", "H", "N", "O", "S", "Cl"]
TABLE = NW.transformation_table(ELS)
PV = F.vec("C12H10N3O2SCl", ELS)


def info(name, parent=PV, smiles=None, lit=None):
    return RU.step_info(name, TABLE, parent, ELS, smiles, lit or {})


def test_scope_of_the_rules():
    assert info("hydroxylation")["level"] == 4 and info("hydroxylation")["scope"] == "generic"
    assert info("reductive dechlorination")["level"] == 3 and info("loss of SO2")["level"] == 3
    assert info("reductive dechlorination", parent=F.vec("C8H10N4O2", ELS))["level"] == 4          # the parent has no chlorine: no class rule
    assert info("decarboxylation", smiles="c1ccccc1C(=O)O")["level"] == 3 and info("decarboxylation", smiles="c1ccccc1CO")["level"] == 4


def test_literature_rows_come_from_the_comment_of_the_list():
    extra = chem.parse_transformations("hydroxylation;+O;lit: Author et al. 2020\nmy rule;+C2H4;lit:\nother;-CH2;a comment")
    lit = RU.literature(extra, ELS)
    assert list(lit.values()) == ["Author et al. 2020"]                                      # empty reference and plain comment do not count
    assert info("hydroxylation", lit=lit) == {"rule": "hydroxylation", "scope": "literature", "level": 2, "source": "Author et al. 2020"}
    assert info("demethylation", lit=lit)["level"] == 4


def cand(derivation, sim=None, kin=None):
    return {"derivation": derivation, "ms2": {"sim": sim} if sim else {}, "kinetics": kin}


def test_prediction_level_takes_the_weakest_step_and_the_data_raise_it_to_one():
    t = lambda d, **k: RU.prediction(cand(d, **k), TABLE, PV, ELS, None, {})
    assert t("hydroxylation")["level"] == 4
    assert t("reductive dechlorination")["level"] == 3
    assert t("reductive dechlorination + hydroxylation")["level"] == 4                       # weakest step
    assert t("cleavage (substructure of the parent)")["level"] == 4 and t(None) is None
    a, b = spectra()
    sim = SM.pair_components(a, 300.0, b, 300.0 + O)
    p = t("hydroxylation", sim=sim, kin={"ok": True, "absent_in_reference": True})
    assert p["level"] == 1 and p["confirmed"] and "shifted" in p["evidence"][-1]
    assert t("hydroxylation", sim=sim, kin={"ok": True, "absent_in_reference": False})["level"] == 4          # present in the reference: not confirmed
    pm, pi = np.array([90.0, 120.0, 180.0]), np.array([100, 70, 60.0])
    flat = SM.pair_components(SM.prepare(pm, pi, 300.0), 300.0, SM.prepare(pm, pi, 316.0), 316.0)
    assert t("hydroxylation", sim=flat)["level"] == 4                                      # equal fragments only: nothing shifted to confirm
    assert t("cleavage and oxidation (+1 O relative to the parent)", sim=sim)["level"] == 4
    for d in ("hydroxylation", "reductive dechlorination", "cleavage"):
        assert "2b" not in json.dumps(t(d, sim=sim))


# ---------------------------------------------------------------------------------------------------------------------- network
def test_mass_network_links_the_formulas_that_differ_by_a_rule():
    els = ["C", "H", "N", "O"]
    table = NW.transformation_table(els)
    mk = lambda i, f, t, **k: {"id": i, "formula": f, "mz": float(F.ion_mass(F.vec(f, els), els)), "rt": 4.0, "kinetics": {"tmax": t}, "derivation": "x",
                               "priority": {"score": 50.0}, "confidence": {"level": 4}, **k}
    cands = [mk(1, "C8H11N4O3", 10.0), mk(2, "C7H9N4O2", 5.0), mk(3, "C7H9N4O3", 20.0), mk(4, "C9H13N4O3", 30.0), mk(5, "C6H5N", 5.0)]
    net = NW.mass_network(cands, els, "C8H11N4O2", table, preds=[{"id": 3, "predecessor": 1, "transformation": "demethylation"}])
    got = {(e["from"], e["to"]): e for e in net["edges"]}
    assert got[("progenitore", 1)]["rule"] == "hydroxylation" and got[("progenitore", 1)]["kind"] == "mass"
    assert got[("progenitore", 2)]["rule"] == "demethylation" and abs(got[("progenitore", 2)]["dm"] + 14.01565) < 1e-3
    assert got[(2, 3)]["rule"] == "hydroxylation" and got[(1, 3)]["rule"] == "demethylation" and got[(1, 3)]["kind"] == "both"
    assert got[(1, 4)]["rule"] == "methylation" and got[(1, 4)]["kinetic"] == "consistent"
    assert not any(5 in (e["from"], e["to"]) for e in net["edges"])                                # no rule links C6H5N to anything
    assert {n["id"] for n in net["nodes"]} == {"progenitore", 1, 2, 3, 4}
    # the pair parent / +O is one edge, from the parent (hydroxylation), not also a dehydroxylation from the product
    assert sum(1 for e in net["edges"] if {e["from"], e["to"]} == {"progenitore", 1}) == 1
    # a candidate whose maximum comes BEFORE its source: the edge stays, flagged
    cands[2]["kinetics"]["tmax"] = 1.0
    again = {(e["from"], e["to"]): e for e in NW.mass_network(cands, els, "C8H11N4O2", table)["edges"]}
    assert any(e["kinetic"] == "against" for e in again.values())


# ---------------------------------------------------------------------------------------------------------------------- engine
def run_series(tmp_path, frag_oh=None, transf=""):
    saved = dict(LS.FRAG)
    try:
        if frag_oh is not None:
            LS.FRAG[LS.HYDROXY] = frag_oh
        files = LS.write_series(tmp_path)
    finally:
        LS.FRAG.clear()
        LS.FRAG.update(saved)
    settings = {"extra_transformations": chem.parse_transformations(transf)} if transf else None
    e = EN.ExperimentHR(files, {"smiles": SMILES}, settings)
    e.run()
    return e


SHIFTED_OH = [(154.0611, 100), (139.0502, 40), (126.0662, 20), (138.0662, 30)]       # the parent's fragments +O (and one unshifted)


@pytest.fixture(scope="module")
def exp(tmp_path_factory):
    return run_series(tmp_path_factory.mktemp("tpms2"), SHIFTED_OH, "hydroxylation;+O;lit: Doe 2020")


def row(e, formula):
    return next(x for x in e.result["rows"] if x["formula"] == formula)


def test_the_known_tps_show_shifted_fragments_and_are_confirmed_by_the_data(exp):
    oh, dm = row(exp, "C8H11N4O3"), row(exp, "C7H9N4O2")
    assert oh["sim"]["n"][1] + oh["sim"]["n"][2] >= 2 and oh["pred"] == 1
    assert dm["sim"]["n"][1] + dm["sim"]["n"][2] >= 2 and dm["pred"] == 1                 # the demethylated product keeps the parent's fragments -CH2
    d = exp.detail(oh["id"])
    assert d["prediction"]["confirmed"] and d["similarity"]["pairs"] and {p["type"] for p in d["similarity"]["pairs"]} <= {"equal", "shifted", "loss"}
    assert d["similarity"]["cos"]["total"] == pytest.approx(oh["modcos"], abs=2e-3)       # the sum of the three is the modified cosine of the table
    assert "2b" not in json.dumps(exp.result["rows"])


def test_the_network_of_mass_differences_overlays_the_predecessors(exp):
    net = exp.result["mass_network"]
    ids = {r["formula"]: r["id"] for r in exp.result["rows"] if r["formula"]}
    edges = {(e["from"], e["to"]): e for e in net["edges"]}
    e1 = edges[("progenitore", ids["C8H11N4O3"])]
    assert e1["rule"] == "hydroxylation" and e1["kind"] in ("both", "mass") and e1["shifted"]["n"] >= 2 and e1["scope"] == "literature"
    assert edges[("progenitore", ids["C7H9N4O2"])]["rule"] == "demethylation"
    assert all(e["kind"] in ("both", "mass", "network") for e in net["edges"])


def test_without_shifted_fragments_the_level_is_that_of_the_rule(tmp_path):
    e = run_series(tmp_path)                                   # the +O product of lc_synth keeps its fragments: nothing shifted by +O
    oh = row(e, "C8H11N4O3")
    assert oh["pred"] == 4 and e.detail(oh["id"])["prediction"]["steps"][0]["scope"] == "generic"


def test_the_literature_level_when_the_data_do_not_confirm(tmp_path):
    e = run_series(tmp_path, transf="hydroxylation;+O;lit: Doe 2020")
    oh = row(e, "C8H11N4O3")
    assert oh["pred"] == 2 and e.detail(oh["id"])["prediction"]["steps"][0]["source"] == "Doe 2020"


def test_the_page_list_of_transformations_reaches_the_engine(tmp_path):
    files = LS.write_series(tmp_path)
    for f in files:
        f["kind"] = "hr"
    api.DATA = tmp_path
    out = json.loads(api.run(json.dumps([{**f, "name": f["name"]} for f in files]), json.dumps({"smiles": SMILES}), "{}", "hydroxylation;+O;lit: Doe 2020"))
    assert [r["pred"] for r in out["rows"] if r["formula"] == "C8H11N4O3"] == [2]
