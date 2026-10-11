# TPMINE-PRIVATE
"""Identities of a feature table (hr/identity.py), synthetic: adducts and isotopes of one molecule are one identity, two co-eluting compounds whose
m/z difference is not explained are not, a low correlation or a far apex breaks the link, the table's own ion identities enter as evidence and only
the steps it did not group are computed, negative mode, the neutral-mass hypotheses and the pairs of the two polarities."""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "py"))
from mzlab.chem import elements as E  # noqa: E402
from tpmine.hr import identity  # noqa: E402

SAMPLES = [f"S{i}" for i in range(1, 7)]
P = E.PROTON
NA, NH4 = E.ADDUCT_SHIFT["[M+Na]+"], E.ADDUCT_SHIFT["[M+NH4]+"]
RAMP = np.array([1.0, 2.5, 4.0, 3.0, 6.0, 1.5])         # the profile of the molecule over the samples
OTHER = np.array([5.0, 1.0, 4.0, 6.0, 0.5, 3.0])        # a different compound


def feat(k, mz, rt, prof, scale=1e5):
    return {"k": f"POS:{k}", "id": str(k), "mz": float(mz), "rt": float(rt), "a": [[i, float(scale * v)] for i, v in enumerate(prof)]}


def ids_of(res):
    return {tuple(sorted(x["members"])) for x in res["identities"]}


def run(features, **kw):
    return identity.build({"pol": kw.pop("pol", 1), "samples": SAMPLES, "features": features, **kw})


def test_adducts_and_isotope_make_one_identity():
    M = 300.1000
    fs = [feat(1, M + P, 5.20, RAMP), feat(2, M + NA, 5.21, RAMP * 0.3 * np.array([1, 1.1, 0.9, 1, 1.05, 0.95])), feat(3, M + NH4, 5.20, RAMP * 0.1 * np.array([1, 0.9, 1.1, 1, 0.97, 1.03])),
          feat(4, M + P + 1.003355, 5.20, RAMP * 0.2), feat(9, 450.2, 7.0, OTHER)]
    r = run(fs)
    assert ids_of(r) == {("POS:1", "POS:2", "POS:3", "POS:4"), ("POS:9",)}
    idn = next(x for x in r["identities"] if len(x["members"]) == 4)
    assert abs(idn["neutral"]["mass"] - M) < 1e-4 and idn["neutral"]["explained"] == 4 and idn["neutral"]["form"] == "[M+H]+"
    roles = {k: v["label"] for k, v in idn["neutral"]["roles"].items()}
    assert roles["POS:2"] == "[M+Na]+" and roles["POS:3"] == "[M+NH4]+" and "isotope" in roles["POS:4"]
    e = next(e for e in r["edges"] if {e["a"], e["b"]} == {"POS:1", "POS:2"})              # every edge carries its evidence
    assert e["source"] == "computed" and e["r"] >= 0.85 and e["n_common"] == 6 and abs(e["dmz"] - (NA - P)) < 1e-4 and e["dscans"] < 2 and len(e["samples"]) == 6
    assert e["relations"][0]["name"] == "[M+Na]+" and abs(e["relations"][0]["ppm"]) < 1


def test_na_and_nh4_without_the_protonated_ion_are_still_linked():
    M = 410.2
    r = run([feat(1, M + NA, 6.0, RAMP), feat(2, M + NH4, 6.0, RAMP * 0.5 + 1e4)])
    assert len(r["identities"]) == 1 and abs(r["identities"][0]["neutral"]["mass"] - M) < 1e-4
    assert "vs" in r["edges"][0]["label"]


def test_two_coeluting_compounds_with_an_unexplained_difference_are_not_grouped():
    fs = [feat(1, 301.1073, 5.2, RAMP), feat(2, 317.0999, 5.2, RAMP * 0.7)]            # dm = 15.99: not a relation of the table, the shapes are identical
    r = run(fs)
    assert ids_of(r) == {("POS:1",), ("POS:2",)} and not r["edges"]


def test_low_correlation_or_a_far_apex_breaks_the_link():
    M = 300.1
    assert len(run([feat(1, M + P, 5.2, RAMP), feat(2, M + NA, 5.2, OTHER)])["identities"]) == 2          # explained, but another shape
    assert len(run([feat(1, M + P, 5.2, RAMP), feat(2, M + NA, 5.4, RAMP)])["identities"]) == 2            # explained and correlated, 12 s apart
    assert len(run([feat(1, M + P, 5.2, RAMP), feat(2, M + NA, 5.4, RAMP)], opt={"cycle_s": 10})["identities"]) == 1      # 2 scans of 10 s
    two = [feat(1, M + P, 5.2, RAMP[:2]), feat(2, M + NA, 5.2, RAMP[:2])]
    assert len(run(two)["identities"]) == 2                                                  # two shared samples: the shape cannot be told


def test_tolerance_in_ppm():
    M = 300.1
    assert len(run([feat(1, M + P, 5.2, RAMP), feat(2, M + NA + 0.0015, 5.2, RAMP)])["identities"]) == 1
    assert len(run([feat(1, M + P, 5.2, RAMP), feat(2, M + NA + 0.02, 5.2, RAMP)])["identities"]) == 2


def test_in_source_loss_and_dimer():
    M = 250.1
    h2o = E.mass({"H": 2, "O": 1})
    r = run([feat(1, M + P, 4.0, RAMP), feat(2, M + P - h2o, 4.0, RAMP * 0.2), feat(3, 2 * M + P, 4.0, RAMP * 0.05)])
    assert len(r["identities"]) == 1
    roles = {k: v["label"] for k, v in r["identities"][0]["neutral"]["roles"].items()}
    assert "-H2O in source" in roles["POS:2"] and roles["POS:3"] == "[2M+H]+"


def test_mzmine_ion_identities_enter_as_evidence_and_only_the_rest_is_computed():
    M = 300.1
    f1, f2, f3 = feat(1, M + P, 5.2, RAMP), feat(2, M + NA, 5.2, RAMP), feat(3, M + NH4, 5.2, RAMP)
    f1["iin"] = {"id": "7", "ions": "[M+H]+", "neutral": M}
    f2["iin"] = {"id": "7", "ions": "[M+Na]+", "neutral": M}
    r = run([f1, f2, f3])
    kinds = sorted((e["kind"], e["source"]) for e in r["edges"])
    assert len(r["identities"]) == 1 and r["identities"][0]["mzmine_groups"] == ["7"]
    assert ("mzMine IIN", "mzMine table") in kinds and r["stats"]["edges_mzmine"] == 1
    assert not any({e["a"], e["b"]} == {"POS:1", "POS:2"} and e["source"] == "computed" for e in r["edges"])          # mzMine already grouped 1 and 2
    assert any("POS:3" in (e["a"], e["b"]) and e["source"] == "computed" for e in r["edges"])                     # the NH4 adduct is a step it did not group
    # FBMN edges: the MS1 annotation ones count, the cosine ones (MS2 similarity) do not
    g = run([feat(1, 100.0, 1.0, RAMP), feat(2, 150.0, 9.0, OTHER), feat(3, 200.0, 5.0, RAMP)], edges=[{"a": "1", "b": "2", "type": "MS1 annotation", "score": 0.9, "annotation": "[M+H]+ [M+Na]+ dm/z=21.98"},
                                                                                                      {"a": "2", "b": "3", "type": "Cosine", "score": 0.99}])
    assert ids_of(g) == {("POS:1", "POS:2"), ("POS:3",)} and g["edges"][0]["source"] == "mzMine edges"


def test_negative_mode():
    M = 280.2
    fa = E.mass({"C": 1, "H": 2, "O": 2})
    fs = [{**feat(1, M - P, 8.0, RAMP), "k": "NEG:1"}, {**feat(2, M + fa - P, 8.0, RAMP * 0.3), "k": "NEG:2"}, {**feat(3, M + E.MASS["Cl"] + E.ELECTRON, 8.0, RAMP * 0.1), "k": "NEG:3"}]
    r = run(fs, pol=-1)
    assert len(r["identities"]) == 1
    idn = r["identities"][0]
    assert abs(idn["neutral"]["mass"] - M) < 1e-4 and {v["label"] for v in idn["neutral"]["roles"].values()} == {"[M-H]-", "[M+FA-H]-", "[M+Cl]-"}


def test_alternatives_are_kept():
    M = 300.1
    # the same two ions can be [M+H]+ / [M+Na]+ of one mass: the alternatives list the other readings, the main one explains both
    r = run([feat(1, M + P, 5.2, RAMP), feat(2, M + NA, 5.2, RAMP)])["identities"][0]
    assert r["neutral"]["explained"] == 2 and abs(r["neutral"]["mass"] - M) < 1e-4
    assert all(a["explained"] <= 2 for a in r["alternatives"])


def test_many_features_are_fast_and_single_features_are_identities():
    rng = np.random.default_rng(1)
    fs = [feat(i, 100 + 400 * rng.random(), 10 * rng.random(), rng.random(6) + 0.1) for i in range(4000)]
    r = run(fs)
    assert r["stats"]["identities"] >= 3900 and sum(len(x["members"]) for x in r["identities"]) == 4000


def test_polarity_pairs():
    M = 300.1
    pos = [{"k": "POS:1", "mz": M + P, "rt": 5.20}, {"k": "POS:2", "mz": 500.0, "rt": 5.2}]
    neg = [{"k": "NEG:7", "mz": M - P, "rt": 5.28}, {"k": "NEG:8", "mz": M - P, "rt": 6.0}, {"k": "NEG:9", "mz": M + E.MASS["Cl"] + E.ELECTRON, "rt": 5.2}]
    out = identity.polarity_pairs(pos, neg)
    got = {(o["pos"], o["neg"], o["pos_form"], o["neg_form"]) for o in out}
    assert got == {("POS:1", "NEG:7", "[M+H]+", "[M-H]-"), ("POS:1", "NEG:9", "[M+H]+", "[M+Cl]-")} and abs(out[0]["mass"] - M) < 1e-4 and abs(out[0]["drt"]) <= 0.15
