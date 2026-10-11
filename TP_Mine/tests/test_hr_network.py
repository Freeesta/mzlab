# TPMINE-PRIVATE
"""WP11 (public, synthetic): known transformations, isotopes, kinetic coherence, priority score, confidence levels and the network of predecessors."""
import numpy as np
import pytest

from mzlab.chem import elements as E
from tpmine.hr import features as FT
from tpmine.hr import formula as F
from tpmine.hr import network as NW

PARENT = "C8H11N4O2"
ELS = F.element_order(E.parse_formula(PARENT))
TABLE = NW.transformation_table(ELS)


def v(f):
    return F.vec(f, ELS)


def test_transformation_table_and_derive():
    names = [n for n, _ in TABLE]
    assert "hydroxylation" in names and "loss of C4H8" in names and "loss of CO" in names
    assert len({tuple(x.tolist()) for _, x in TABLE}) == len(TABLE)                       # no duplicates
    assert NW.derive(v("C8H11N4O3") - v(PARENT), TABLE) == "hydroxylation"
    assert NW.derive(v("C8H9N4O2") - v(PARENT), TABLE) == "dehydrogenation"
    assert NW.derive(v("C8H9N4O3") - v(PARENT), TABLE) == "oxidation to carbonyl"
    assert NW.derive(v("C8H11N4O4") - v(PARENT), TABLE, max_steps=1) == "dihydroxylation"
    two = NW.derive(v("C8H9N4O4") - v(PARENT), TABLE)                                      # +O2 -H2: two steps
    assert two and " + " in two and NW.derive(v("C8H9N4O4") - v(PARENT), TABLE, max_steps=1) is None
    assert NW.derive(v(PARENT) - v(PARENT), TABLE) is None
    assert NW.derive(v("C20H11N4O2") - v(PARENT), TABLE) is None
    user = NW.transformation_table(ELS, extra=[{"name": "metilazione speciale", "delta": {"C": 3}}])
    assert NW.derive(v("C11H11N4O2") - v(PARENT), user) == "metilazione speciale"


def test_cleavage_is_a_derivation():
    pv = v(PARENT)
    assert NW.derivation_name(v("C5H8N3"), pv, ELS, TABLE) == "cleavage (substructure of the parent)"
    assert NW.derivation_name(v("C8H11N4O3"), pv, ELS, TABLE) == "hydroxylation"                       # a transformation comes first
    assert NW.derivation_name(v("C9H5N2"), pv, ELS, TABLE) is None                                        # more carbon than the parent
    assert NW.derivation_name(v("C5H20N3"), pv, ELS, TABLE) is None                                       # far more hydrogen than the parent
    assert NW.derivation_name(pv, pv, ELS, TABLE) is None
    assert NW.derivation_name(v("C5H8N3O3"), pv, ELS, TABLE).startswith("cleavage and oxidation (+1 O")           # a part with one more oxygen than the parent has
    assert NW.derivation_name(v("C5H8N3O9"), pv, ELS, TABLE) is None


def test_expected_and_observed_isotopes():
    exp = NW.expected_isotopes(v(PARENT), ELS)
    assert exp["m1"] == pytest.approx(8 * 0.0107 + 4 * 0.00364 + 11 * 0.000115 + 2 * 0.00038, rel=1e-6) and not exp["has_s"]
    assert NW.expected_isotopes(F.vec("C13H25N4O3S", F.element_order({"S": 1})), F.element_order({"S": 1}))["has_s"]
    mono = FT.Features(mz=np.array([300.10, 301.1034, 302.1066, 400.0]), rt=np.array([5.0, 5.01, 5.0, 5.0]), apex=np.zeros(4, int), area=np.array([1e8, 1.5e7, 8e6, 1e7]),
                       height=np.ones(4), npts=np.full(4, 9), left=np.zeros(4), right=np.ones(4), background=np.zeros(4, bool), artefact=np.zeros(4, bool))
    al = FT.align([mono])
    g = int(np.argmin(np.abs(al.mz - 300.10)))
    obs = NW.observed_isotopes(al, g)
    assert obs["m1"] == pytest.approx(0.15, rel=1e-6) and obs["m2"] == pytest.approx(0.08, rel=1e-6)
    sl = int(np.argmin(np.abs(al.mz - 400.0)))
    assert NW.observed_isotopes(al, sl) == {"file": 0, "m1": None, "m2": None}
    assert NW.isotopes_coherent({"m1": 0.1, "m2": None}, {"m1": 0.10, "m2": 0.0, "has_s": False})[0] == "pass"
    assert NW.isotopes_coherent({"m1": 0.5, "m2": None}, {"m1": 0.1, "m2": 0.0, "has_s": False})[0] == "fail"
    assert NW.isotopes_coherent({"m1": None, "m2": None}, {"m1": 0.1, "m2": 0.0, "has_s": False})[0] == "n/a"
    s = NW.isotopes_coherent({"m1": 0.12, "m2": 0.30}, {"m1": 0.12, "m2": 0.05, "has_s": True})
    assert s[0] == "fail" and "M+2" in s[1]


def test_kinetic_coherence():
    good = {"ok": True, "times": list(range(6)), "unimodal": True, "absent_in_reference": True, "class": ["early"], "class_text": "early"}
    assert NW.kinetic_coherence(good)[0] == 1.0
    assert NW.kinetic_coherence({**good, "absent_in_reference": False})[0] == 0.0
    assert NW.kinetic_coherence({**good, "times": [0, 1, 2]})[0] == 0.5
    assert NW.kinetic_coherence({**good, "unimodal": False, "class": ["early"]})[0] == 0.0
    assert NW.kinetic_coherence({**good, "unimodal": False, "class": ["late", "persistent"]})[0] == 1.0
    assert NW.kinetic_coherence(None)[0] == 0.5 and NW.kinetic_coherence({"ok": False})[0] == 0.5
    # coverage: 11 treated samples, the product in all of them / in 6 (more than half: full score) / in 3 / in 1
    t = list(range(-1, 12))
    prof = lambda n: [0, 0] + [1.0] * n + [0] * (11 - n)
    full = lambda n: {**good, "times": t, "profile": prof(n)}
    assert [round(NW.kinetic_coherence(full(n))[0], 2) for n in (11, 6, 3, 1)] == [1.0, 1.0, 0.73, 0.51]
    assert "present in 3 of 11 treated samples" in NW.kinetic_coherence(full(3))[1]


def cand(**kw):
    c = {"id": 1, "mz": 211.0, "rt": 4.0, "formula": "C8H11N4O3", "n_formulas": 1, "ppm": 0.5, "derivation": "hydroxylation", "area_max": 1e7,
         "kinetics": {"ok": True, "times": list(range(6)), "unimodal": True, "absent_in_reference": True, "class": ["early"], "class_text": "early", "tmax": 10.0},
         "ms2": {"n_scans": 8, "modcos": 0.8, "n_matched": 9}, "isotopes": {"status": "pass", "text": "ok"}, "time_step": 5.0,
         "localization": {"ok": True, "region_atoms": [1, 2], "n_atoms": 14, "margin": 1.2, "region": "anello A (2/9)", "fraction_explained": 0.8, "evidence": [1, 2, 3, 4, 5]}}
    c.update(kw)
    return c


def test_priority_components_and_weights():
    p = NW.priority(cand(), 1.0)
    comp = p["components"]
    assert comp["ms2"] == pytest.approx(0.8) and comp["loc"] == pytest.approx(1 - 2 / 14) and comp["kin"] == 1.0 and comp["form"] == 1.0 and comp["int"] == 1.0
    assert p["score"] == pytest.approx(100 * (0.45 * 0.8 + 0.15 * (1 - 2 / 14) + 0.15 + 0.10 + 0.15)) and p["penalty"] == 1.0
    assert sum(NW.WEIGHTS.values()) == pytest.approx(1.0) and all(comp["text"][k] for k in NW.WEIGHTS)
    none = NW.priority(cand(ms2=None, localization=None), 0.5)["components"]
    assert none["ms2"] == 0.0 and none["loc"] == 0.0
    few = NW.priority(cand(ms2={"n_scans": 2, "modcos": 0.9, "n_matched": 2}), 0.5)["components"]["ms2"]
    assert few == 0.0                                                                       # fewer than three matched peaks: no credit
    assert NW.priority(cand(localization={**cand()["localization"], "margin": 0.1}), 0.5)["components"]["loc"] == 0.0
    f = NW.priority(cand(n_formulas=2), 0.5)["components"]["form"]
    g = NW.priority(cand(derivation=None), 0.5)["components"]["form"]
    assert f == 0.0 and g == 0.5
    custom = NW.priority(cand(), 1.0, weights={"ms2": 1, "loc": 0, "kin": 0, "form": 0, "int": 0})
    assert custom["score"] == pytest.approx(80.0)


def test_family_penalty_keeps_the_candidate():
    base = NW.priority(cand(), 1.0)
    pen = NW.priority(cand(iimn={"role": "adduct"}), 1.0)
    assert pen["score"] == pytest.approx(0.3 * base["score"]) and pen["explained_by_family"] and not base["explained_by_family"]
    assert NW.priority(cand(iimn={"role": "ion"}), 1.0)["score"] == pytest.approx(base["score"])


def lv(c, **kw):
    return NW.confidence(c, parent_rt=5.0, **kw)["level"]


def diag(c, **kw):
    return NW.confidence(c, parent_rt=5.0, **kw)["diagnostic"]


def test_confidence_levels():
    assert lv(cand()) == 3 and diag(cand())                      # the program never assigns 2b: level 3 with diagnostic evidence
    assert not diag(cand(rt=6.0))
    assert lv(cand(rt=6.0)) == 3                                                           # a hydroxylated product after the parent: RT incoherent
    assert lv(cand(localization={**cand()["localization"], "margin": 0.1})) == 3           # region without margin
    assert lv(cand(), groups_of_region=3) == 3                                             # more than two groups
    assert lv(cand(ms2={"n_scans": 8, "modcos": 0.3, "n_matched": 4}, localization={"ok": False, "note": "x"})) == 4
    assert lv(cand(ms2=None, localization=None)) == 4                                      # no MS2 at all: level 4 at best
    assert lv(cand(isotopes={"status": "fail", "text": "x"})) == 5
    assert lv(cand(derivation=None, ms2=None, localization=None)) == 5
    assert lv(cand(n_formulas=3, ms2=None, localization=None)) == 5
    assert lv(cand(predecessor_kinetics=100.0)) == 3                                      # the predecessor peaks long after this product
    r = NW.confidence(cand(), parent_rt=5.0)
    assert r["level"] in NW.LEVEL_TEXT and r["text"] == NW.DIAGNOSTIC_TEXT and r["diagnostic"] and len(r["criteria"]) >= 9
    assert {c["status"] for c in r["criteria"]} <= {"pass", "fail", "n/a"} and all(c["text"] for c in r["criteria"])
    assert "2a" not in {str(x) for x in (NW.confidence(cand())["level"],)}


def test_rank_orders_by_score_or_by_level():
    a, b, c = cand(id=1), cand(id=2, rt=6.0), cand(id=3, ms2=None, localization=None)
    rows = []
    for x, area in ((a, 0.2), (b, 1.0), (c, 1.0)):
        x["confidence"], x["priority"] = NW.confidence(x, parent_rt=5.0), NW.priority(x, area)
        rows.append(x)
    assert [x["id"] for x in NW.rank(rows, by_level=True)] == [1, 2, 3]                  # 3 with diagnostic evidence, then 3, then 4 although 3 is the most intense
    by_score = [x["id"] for x in NW.rank(rows)]
    assert by_score == sorted(by_score, key=lambda i: -next(x["priority"]["score"] for x in rows if x["id"] == i))


def test_predecessors():
    pf = PARENT
    mk = lambda i, f, tmax, **kw: {"id": i, "formula": f, "kinetics": {"tmax": tmax}, **kw}
    cands = [mk("a", "C8H11N4O3", 5.0), mk("b", "C8H11N4O4", 20.0), mk("c", "C8H9N4O3", 10.0), mk("d", "C8H11N4O4", 2.0), mk("e", "C20H11N4O2", 5.0), mk("f", None, 5.0)]
    res = NW.predecessors(cands, ELS, pf, TABLE)
    by = {r["id"]: r for r in res}
    assert by["a"]["predecessor"] == "progenitore" and by["a"]["transformation"] == "hydroxylation"
    assert by["b"]["predecessor"] in ("a", "progenitore") and by["b"]["kinetic"] is not None
    assert by["e"]["predecessor"] is None and by["f"]["predecessor"] is None
    assert by["d"]["predecessor"] != "a"                                                 # 'a' peaks at 5 min, this product at 2 min: more than one step too late
    cos = {(0, 2): (0.8, 7)}                                                              # a related MS2 between a and c
    res2 = NW.predecessors(cands, ELS, pf, TABLE, cos=cos)
    assert {r["id"]: r for r in res2}["c"]["predecessor"] == "a" and {r["id"]: r for r in res2}["c"]["cos"] == pytest.approx(0.8)
