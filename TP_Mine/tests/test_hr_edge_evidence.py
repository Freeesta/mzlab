# TPMINE-PRIVATE
"""Evidence on the edges of an identity (hr/edge_evidence.py), synthetic: every step of the table of Attributio is read by the tables of mzFinder,
and every case gives its label (adduct, in-source fragment?, not distinguishable) with the evidence that supports it. A signal, never a decision."""
import sys
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "py"))
from mzlab.chem import elements as E  # noqa: E402
from tpmine.hr import edge_evidence as EV  # noqa: E402
from tpmine.hr import identity, iimn  # noqa: E402

P = E.PROTON
M = 300.1000
LO = M + P                                  # the base ion [M+H]+
STABLE = [1.0, 2.0, 3.5, 2.8, 5.0, 1.2]     # areas of the lighter ion over six samples
UP_STABLE = [4 * v * f for v, f in zip(STABLE, (1.0, 1.05, 0.97, 1.02, 0.99, 1.03))]     # ratio ~4, CV ~3 %
UP_VARIES = [a * b for a, b in zip(STABLE, (1, 3, 0.5, 6, 1, 9))]


def spec(*mzs, it=None):
    return {"mz": list(mzs), "it": list(it or [100.0 - 10 * i for i in range(len(mzs))])}


def ev(lo, up, pol=1, **kw):
    return EV.edge_evidence(lo, up, pol, **kw)


def keys(r, strength=None):
    return {e["key"] for e in r["evidence"] if strength is None or e["strength"] == strength}


@pytest.mark.parametrize("pol,kind,name,step", [(p, k, n, v) for p, t in EV.STEPS.items() for k, d in t.items() for n, v in d.items()])
def test_every_step_of_the_table_is_read_by_the_tables_of_mzfinder(pol, kind, name, step):
    r = EV.relations_of(LO, LO + step, pol)
    assert any(x["role"] == kind and abs(x["step"] - step) < 1e-5 for x in r), (name, r)


def test_adduct_by_the_step_alone_when_nothing_else_fits():
    r = ev(LO, M + E.ADDUCT_SHIFT["[M+Na]+"], drt_min=0.01)               # [M+Na]+ vs [M+H]+: 21.9819, no loss reads it, no MS2
    assert r["kind"] == "adduct" and r["label"] == "adduct" and r["text"].startswith("adduct (evidence: Δm/z 21.9819")
    assert "coelution" in keys(r, "evidence")


def test_nh4_step_without_ms2_is_not_distinguishable_it_is_also_the_loss_of_nh3():
    r = ev(LO, M + E.ADDUCT_SHIFT["[M+NH4]+"], areas_lo=STABLE, areas_up=UP_STABLE)
    assert {x["name"] for x in r["relations"]} == {"[M+NH4]+", "NH3"} and r["kind"] == "undetermined" and r["label"] == "not distinguishable"


def test_nh4_with_the_typical_loss_in_its_ms2_is_an_adduct():
    up = M + E.ADDUCT_SHIFT["[M+NH4]+"]
    r = ev(LO, up, ms2_up=spec(up - EV.NH3, 150.0), areas_lo=STABLE, areas_up=UP_STABLE)
    assert r["kind"] == "adduct" and "typical_loss" in keys(r, "evidence") and "-17.027" in r["text"]


def test_nh4_without_the_typical_loss_and_without_the_fragment_stays_undetermined():
    up = M + E.ADDUCT_SHIFT["[M+NH4]+"]
    r = ev(LO, up, ms2_up=spec(120.0, 150.0), areas_lo=STABLE, areas_up=UP_STABLE)
    assert r["kind"] == "undetermined" and "typical_loss" in keys(r, "against") and "frag_ms2" in keys(r, "against")


def test_in_source_fragment_needs_the_fragment_in_the_ms2_and_a_stable_ratio():
    up = LO + EV.STEPS[1]["loss"]["H2O"]                                    # X = Y - H2O
    r = ev(LO, up, ms2_up=spec(LO, 100.0, 80.0), areas_lo=STABLE, areas_up=UP_STABLE, drt_min=0.0)
    assert r["kind"] == "isf" and r["label"] == "in-source fragment?" and "frag_ms2" in keys(r, "evidence") and "CV" in r["text"]
    cv = next(e["cv"] for e in r["evidence"] if e["key"] == "ratio")
    assert cv < 0.1


def test_the_fragment_in_the_ms2_with_a_ratio_that_changes_is_not_enough():
    up = LO + EV.STEPS[1]["loss"]["H2O"]
    r = ev(LO, up, ms2_up=spec(LO, 100.0), areas_lo=STABLE, areas_up=UP_VARIES)
    assert r["kind"] == "undetermined" and "ratio" in keys(r, "against")


def test_a_stable_ratio_without_the_fragment_in_the_ms2_is_not_enough():
    up = LO + EV.STEPS[1]["loss"]["CO2"]
    assert ev(LO, up, ms2_up=spec(250.0, 100.0), areas_lo=STABLE, areas_up=UP_STABLE)["kind"] == "undetermined"
    assert ev(LO, up, areas_lo=STABLE, areas_up=UP_STABLE)["kind"] == "undetermined"                   # no MS2 at all


def test_fewer_than_three_common_samples_cannot_show_a_stable_ratio():
    up = LO + EV.STEPS[1]["loss"]["CO"]
    r = ev(LO, up, ms2_up=spec(LO, 100.0), areas_lo=[1, 2, 0, 0, 0, 0], areas_up=[4, 8, 0, 0, 0, 0])
    assert r["kind"] == "undetermined" and "ratio" in keys(r, "info")


def test_adduct_with_a_common_skeleton_in_the_two_ms2():
    up = M + E.ADDUCT_SHIFT["[M+K]+"]
    d = up - LO
    r = ev(LO, up, ms2_lo=spec(200.1, 150.2, 120.3), ms2_up=spec(200.1, 150.2 + d, 90.0))        # two of three fragments: same m/z or shifted
    assert r["kind"] == "adduct" and "skeleton" in keys(r, "evidence")


def test_adduct_whose_fragments_do_not_match_is_not_distinguishable():
    up = M + E.ADDUCT_SHIFT["[M+Na]+"]
    r = ev(LO, up, ms2_lo=spec(200.1, 150.2), ms2_up=spec(111.1, 99.9))
    assert r["kind"] == "undetermined" and "skeleton" in keys(r, "against")


def test_sodiated_that_does_not_fragment_is_a_hint_not_a_proof():
    up = M + E.ADDUCT_SHIFT["[M+Na]+"]
    r = ev(LO, up, ms2_up=spec(up, it=[100.0]))
    assert r["kind"] == "adduct" and "no_fragmentation" in keys(r, "hint") and "no_fragmentation" not in keys(r, "evidence")


def test_negative_formate_is_also_the_loss_of_hcooh():
    lo = M - P
    up = lo + EV.STEPS[-1]["adduct"]["HCOO"]
    assert ev(lo, up, pol=-1)["kind"] == "undetermined"                                         # two readings, nothing to tell them apart
    assert ev(lo, up, pol=-1, ms2_lo=spec(200.1, 150.2), ms2_up=spec(200.1, 150.2))["kind"] == "adduct"          # same skeleton
    r = ev(lo, up, pol=-1, ms2_up=spec(lo, 100.0), areas_lo=STABLE, areas_up=UP_STABLE)          # the lighter ion is a fragment, stable ratio
    assert r["kind"] == "isf"


def test_chloride_and_isotope_and_unexplained_steps():
    lo = M - P
    assert ev(lo, lo + EV.STEPS[-1]["adduct"]["Cl+H"], pol=-1)["kind"] in ("adduct", "undetermined")
    assert ev(LO, LO + 1.003355)["kind"] == "other"                                                  # an isotope has no adduct/fragment label
    assert ev(LO, LO + 15.99)["kind"] == "other" and ev(LO, LO + 15.99)["text"] == ""


def test_co_elution_beyond_the_window_is_flagged():
    r = ev(LO, M + E.ADDUCT_SHIFT["[M+Na]+"], drt_min=0.08)
    assert "coelution" in keys(r, "against") and "beyond 0.05 min" in r["text"]


def test_the_tolerance_is_the_larger_of_2_mda_and_5_ppm():
    assert EV.tol(300.0) == 0.002 and abs(EV.tol(1000.0) - 0.005) < 1e-9
    up = LO + EV.STEPS[1]["loss"]["H2O"]
    off = ev(LO, up, ms2_up=spec(LO + 0.0015, 100.0), areas_lo=STABLE, areas_up=UP_STABLE)
    assert off["kind"] == "isf"                                                                       # 1.5 mDa off: inside
    far = ev(LO, up, ms2_up=spec(LO + 0.004, 100.0), areas_lo=STABLE, areas_up=UP_STABLE)
    assert far["kind"] == "undetermined"                                                              # 4 mDa off: outside


def test_identity_edges_carry_the_evidence():
    S = [f"S{i}" for i in range(6)]
    f = lambda k, mz, a, ms2=None: {"k": f"POS:{k}", "id": str(k), "mz": mz, "rt": 5.2, "a": [[i, 1e5 * v] for i, v in enumerate(a)], **({"ms2": ms2} if ms2 else {})}  # noqa: E731
    up = LO + EV.STEPS[1]["loss"]["H2O"]
    r = identity.build({"pol": 1, "samples": S, "features": [f(1, LO, STABLE), f(2, up, UP_STABLE, spec(LO, 100.0))]})
    e = r["edges"][0]["evidence"]
    assert e["kind"] == "isf" and e["text"].startswith("in-source fragment? (evidence:")


def test_report_evidence_lists_the_proofs_only():
    up = LO + EV.STEPS[1]["loss"]["H2O"]
    rep = {"mz": LO, "parent_mz": up, "profile": {"apex_diff_s": 0.4}, "ratio_constancy": {"cv": 0.05, "k": 5, "p": 0.6}, "ratio": {"ok": True},
           "ms2_peaks": {"p": ([LO, 100.0], [100.0, 20.0]), "x": None}}
    out = EV.report_evidence(rep, 1)
    assert {e["key"] for e in out} >= {"dm", "coelution", "frag_ms2", "ratio"} and all("kind" not in e for e in out)
