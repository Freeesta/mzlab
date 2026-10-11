"""Full isotope score and fragment constraint of the candidate formulas (mzlab/chem/isoscore.py): synthetic spectra, never a verdict."""
import json
import random

from mzlab.api import dispatch
from mzlab.chem.composition import compose, ion_mass, parse_elements
from mzlab.chem.elements import parse_formula
from mzlab.chem.isoscore import fine_pattern, fragment_constraint, isotope_score, merge_res
from mzlab.chem.subformulas import subformulas

TRUE = "C13H25N4O3S"


def _obs(counts, rng=None, r200=70000.0, noise=0.0):
    pat = merge_res(fine_pattern(counts, 1), r200)
    rng = rng or random.Random(1)
    return [(p["mz"] * (1 + rng.gauss(0, 1e-6)), p["rel"] * 1e4 * (1 + rng.gauss(0, noise))) for p in pat if p["rel"] >= 0.5]


def test_fine_structure_and_merging():
    c = parse_formula(TRUE)
    fine = fine_pattern(c, 1)
    merged = merge_res(fine, 70000.0)
    assert len(fine) > len(merged) >= 3 and abs(merged[0]["rel"] - 100) < 1e-9
    assert abs(merged[1]["rel"] - 16.7) < 0.3                           # M+1/M of the ion, as isotope_ratios
    assert merge_res(fine, 0) == fine                                   # R = 0 keeps the fine structure
    assert abs(fine[0]["mz"] - ion_mass(c, 1)) < 1e-9


def test_halogens_are_evident_in_the_pattern():
    cl2 = merge_res(fine_pattern({"C": 10, "H": 9, "Cl": 2, "N": 1}, 1), 70000.0)
    m2 = max((p for p in cl2 if 1.99 < p["mz"] - cl2[0]["mz"] < 2.01), key=lambda p: p["rel"])
    assert 55 < m2["rel"] < 70 and abs(m2["mz"] - cl2[0]["mz"] - 1.9970) < 0.002                    # 37Cl: M+2 near 64 %
    br = merge_res(fine_pattern({"C": 8, "H": 8, "Br": 1, "O": 1}, 1), 70000.0)
    assert max(p["rel"] for p in br if 1.99 < p["mz"] - br[0]["mz"] < 2.01) > 90


def test_true_formula_scores_above_a_wrong_one_with_the_terms_listed():
    c = parse_formula(TRUE)
    obs = _obs(c, noise=0.05)
    good = isotope_score(c, 1, obs)
    bad = isotope_score(parse_formula("C14H29N2O4S"), 1, obs)
    assert good["score"] > 0.7 > bad["score"] or good["score"] > bad["score"] + 0.2
    assert all({"mz", "rel", "w", "t_mass", "t_int", "t"} <= set(p) for p in good["peaks"])
    assert abs(sum(p["w"] for p in good["peaks"]) + sum(e["w"] for e in good["extra"]) - 1) < 1e-9
    assert abs(good["score"] - sum(p["w"] * p["t"] for p in good["peaks"])) < 1e-9


def test_missing_and_extra_peaks_count_against_the_formula():
    c = parse_formula(TRUE)
    obs = _obs(c)
    full = isotope_score(c, 1, obs)["score"]
    no_m2 = isotope_score(c, 1, [o for o in obs if abs(o[0] - obs[0][0] - 1.9953) > 0.5])
    assert no_m2["n_missing"] >= 1 and no_m2["score"] < full
    extra = isotope_score(c, 1, obs + [(obs[0][0] + 0.5 + 1.0, obs[0][1] * 0.3)])
    assert extra["n_extra"] == 1 and extra["score"] < full


def test_fragment_constraint():
    c = parse_formula(TRUE)
    _, subs = subformulas(TRUE, 1, 60.0)
    chosen = [subs[len(subs) // 3], subs[len(subs) // 2]]
    frags = [(m, 1000.0) for _, m in chosen] + [(150.12345, 1000.0)]    # one peak no sub-formula explains (probably)
    r = fragment_constraint(TRUE, 1, frags)
    assert r["n_peaks"] == 3 and r["n_explained"] >= 2 and 0.6 < r["explained"] <= 1.0
    foreign = fragment_constraint("C5H9N2OS", 1, [(m, 1000.0) for _, m in chosen if m > 200])
    assert foreign["explained"] in (None, 0.0) or foreign["explained"] < 1.0
    assert fragment_constraint(TRUE, 1, [(300.0, 0.0)])["explained"] is None
    assert ion_mass(c, 1) > 0


def test_compose_sorts_by_the_combined_score_and_lists_the_terms():
    c = parse_formula(TRUE)
    mz = ion_mass(c, 1)
    els = parse_elements("C:0-20,H:0-40,N:0-6,O:0-6,S:0-2")
    _, subs = subformulas(TRUE, 1, 60.0)
    frags = [(m, 1e4) for _, m in subs[::37][:5]]
    r = compose(mz, elements=els, tol=10, obs=_obs(c, noise=0.05), frags=frags, max_results=20)
    assert r["results"][0]["formula"] == TRUE
    top = r["results"][0]
    assert {"mass", "iso", "frag", "total"} == set(top["score"]) and top["frag"]["n_explained"] >= 1
    tot = [x["score"]["total"] for x in r["results"]]
    assert tot == sorted(tot, reverse=True)
    w0 = compose(mz, elements=els, tol=10, obs=_obs(c), weights=(1.0, 0.0, 0.0), max_results=5)         # weights are the user's
    assert w0["results"][0]["score"]["total"] == w0["results"][0]["score"]["mass"]
    plain = compose(mz, elements=els, tol=10, max_results=5)                                           # without evidence: no score, order of the error
    assert "score" not in plain["results"][0]


def test_halogenated_candidate_wins_with_the_full_pattern():
    c = {"C": 12, "H": 11, "Cl": 2, "N": 3, "O": 1}
    mz = ion_mass(c, 1)
    r = compose(mz, elements=parse_elements("C:0-20,H:0-30,N:0-5,O:0-4,S:0-2,Cl:0-3"), tol=10, obs=_obs(c, noise=0.04), max_results=10)
    assert r["results"][0]["formula"] == "C12H11Cl2N3O"


def test_api_passes_peaks_and_weights():
    c = parse_formula(TRUE)
    obs = ";".join(f"{m:.5f}:{i:.2f}" for m, i in _obs(c))
    q = {"mz": f"{ion_mass(c, 1):.5f}", "elements": "C:0-20,H:0-40,N:0-6,O:0-6,S:0-2", "tol": "5", "obs": obs, "r200": "70000", "sppm": "3", "sint": "0.10", "w": "1,2,0.5"}
    code, _, body, _ = dispatch(None, "GET", "/api/composition", q)
    j = json.loads(body)
    assert code == 200 and j["results"][0]["formula"] == TRUE and j["results"][0]["score"]["iso"] > 0.5
    code, _, body, _ = dispatch(None, "GET", "/api/composition", {**q, "obs": "abc"})
    assert code == 400


def test_normalisation_modes_and_thresholds():
    """Another normalisation mode scales the observed intensities differently; a perfect pattern scores about the same in every mode; thresholds are passed on."""
    c = parse_formula("C13H25N4O3S")
    pat = [p for p in merge_res(fine_pattern(c, 1), 70000.0) if p["rel"] >= 1.0]
    obs = [(p["mz"], p["rel"] * 1e4) for p in pat]
    base = isotope_score(c, 1, obs)["score"]
    for mode in ("mono", "max", "sum"):
        assert abs(isotope_score(c, 1, obs, norm=mode)["score"] - base) < 0.02, mode
    bent = [(m, i * (3.0 if k == 0 else 1.0)) for k, (m, i) in enumerate(obs)]               # a monoisotopic peak 3x too high
    assert isotope_score(c, 1, bent, norm="mono")["score"] < isotope_score(c, 1, bent, norm="sum")["score"] + 0.5
    few = isotope_score(c, 1, obs, min_rel=50.0)
    assert len(few["peaks"]) < len(isotope_score(c, 1, obs, min_rel=0.1)["peaks"])
