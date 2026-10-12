"""Tests of mzlab.ionfamily on synthetic scenes with known truth (demo.isf_scene) and, when the data are there, on the real files."""
from __future__ import annotations

import json
import math
import os
from pathlib import Path

import numpy as np
import pytest

from mzlab import demo, ionfamily as f

OFF = demo.ISF_OFFSET
ISF_NAMES = {"ISF_H2O", "ISF_big", "ISF_small", "M+1", "M+Na"}
TP_NAMES = {"TP_early", "TP_late", "TP_coelute"}


def _mz(ion):
    return ion["mz"] + OFF


def _scene(t=15.0, seed=1, **kw):
    tb, ions = demo.isf_scene(t, seed, **kw)
    return tb, {i["name"]: i for i in ions}


# ----------------------------------------------------------------------------------------------------------------------- signal
def test_savgol_smooths_and_derives():
    x = np.linspace(-3, 3, 61)
    y = np.exp(-x ** 2) + np.random.default_rng(0).normal(0, 0.02, len(x))
    s = f.savgol(y, 9, 2)
    assert np.abs(s - np.exp(-x ** 2)).mean() < np.abs(y - np.exp(-x ** 2)).mean()
    d = f.savgol(np.exp(-x ** 2), 9, 3, 1) / (x[1] - x[0])
    assert np.abs(d - (-2 * x * np.exp(-x ** 2))).max() < 0.05


def test_baselines_remove_drift():
    n = 200
    t = np.arange(n)
    peak = 1000 * np.exp(-0.5 * ((t - 100) / 4) ** 2)
    y = peak + 50 + 0.5 * t
    for b in (f.asls_baseline(y), f.snip_baseline(y, 30)):
        assert abs(b[100] - (50 + 50)) < 40            # baseline under the peak ~ drift line
        assert (y - b).max() > 900


def test_detect_peaks_apex_fwhm_and_sigma():
    rt = np.arange(0, 4, 1 / 60)
    y = 1e5 * np.exp(-0.5 * ((rt - 2.0) / 0.05) ** 2) + np.random.default_rng(0).normal(0, 300, len(rt)) + 2000
    d = f.detect_peaks(rt, y)
    assert len(d["peaks"]) == 1
    p = d["peaks"][0]
    assert abs(p["apex_rt"] - 2.0) < 0.01 and p["apex_sigma"] < 0.02
    assert abs(p["fwhm_min"] - 2.355 * 0.05) < 0.03
    assert p["reliable"] and p["n_scans"] >= 6


def test_detect_peaks_edge_cases_do_not_break():
    rt = np.arange(0, 1, 1 / 60)
    assert f.detect_peaks(rt, np.zeros(len(rt)))["peaks"] == []
    assert f.detect_peaks(rt[:4], np.ones(4))["peaks"] == []
    assert f.detect_peaks(rt, np.random.default_rng(0).normal(0, 1, len(rt)))["peaks"] == []
    narrow = np.zeros(len(rt))
    narrow[30:33] = [500, 3000, 500]
    d = f.detect_peaks(rt, narrow + 5)
    assert all(not p["reliable"] for p in d["peaks"])           # < 6 scans


def test_noise_level_ignores_peaks():
    rng = np.random.default_rng(1)
    y = rng.normal(0, 10, 300)
    y[100:110] += 5000
    assert 7 < f.noise_level(y) < 14


# ----------------------------------------------------------------------------------------------------------------------- data
def test_rois_find_the_ions_and_are_cached():
    tb, ions = _scene()
    w = f.build_rois(tb, 7.5, 8.6, key="a")
    assert w.X.shape[0] == len(w.mz) and w.X.shape[1] == len(w.rt)
    for name in ("P", "ISF_big", "ISF_H2O"):
        j = int(np.argmin(np.abs(w.mz - _mz(ions[name]))))
        assert abs(w.mz[j] - _mz(ions[name])) < 0.1
        assert w.X[j].max() > 0.5 * ions[name]["amp"] * 0.5
    assert f.build_rois(tb, 7.5, 8.6, key="a") is w            # cache hit
    f.clear_cache()
    empty = f.build_rois(tb, 100, 101)
    assert empty.X.shape[0] == 0


# ----------------------------------------------------------------------------------------------------------------------- profile
def _sim(tb, ions, name, **kw):
    rt, P = f.extract_trace(tb, _mz(ions["P"]), 0.35, 7.0, 9.0)
    _, X = f.extract_trace(tb, _mz(ions[name]), 0.35, 7.0, 9.0)
    return f.profile_similarity(rt, X, P, **kw)


def test_isf_shares_profile_and_tp_does_not():
    tb, ions = _scene(15.0)
    isf = _sim(tb, ions, "ISF_big")
    assert isf["pearson"] > 0.95 and abs(isf["apex_diff_s"]) < 2.5 and 0.8 < isf["fwhm_ratio"] < 1.25
    assert isf["deriv_corr"] > 0.8 and isf["cosine"] > 0.97 and isf["lag_best"] == 0
    assert isf["p_shift"] < 0.05 and isf["corr_ci"][0] > 0.85
    early = _sim(tb, ions, "TP_early")
    assert not early["x_has_peak"] or early["pearson"] < 0.5
    co = _sim(tb, ions, "TP_coelute")
    assert co["pearson"] < isf["pearson"] - 0.05 or abs(co["apex_diff_s"]) > 2
    assert co["fwhm_ratio"] > 1.2 or math.isnan(co["fwhm_ratio"])


def test_few_scans_are_flagged():
    tb, ions = demo.isf_scene(15.0, 3, dt_s=10.0)              # 10-s scans: ~1 scan per FWHM
    ions = {i["name"]: i for i in ions}
    s = _sim(tb, ions, "ISF_big")
    assert not s["reliable"] or s["n_scans"] < 8
    assert any(w["key"] == "of.warn.fewScans" for w in s["warnings"]) or not s["reliable"]


def test_p_shift_and_ci_are_present_and_in_range():
    tb, ions = _scene(10.0)
    s = _sim(tb, ions, "ISF_H2O")
    assert 0 < s["p_shift"] <= 1 and len(s["corr_ci"]) == 2 and s["corr_ci"][0] <= s["pearson"] <= s["corr_ci"][1] + 1e-9


def test_metrics_separate_isf_from_tp_with_high_auc():
    scores = {"pearson": [], "apex": [], "spearman": [], "deriv": [], "cv": []}
    labels = []
    for seed, t in enumerate([5, 10, 15, 20, 30, 45, 60]):
        tb, ions = _scene(t, 100 + seed)
        w = f.build_rois(tb, 7.5, 8.6)
        rt, P = f.extract_trace(tb, _mz(ions["P"]), 0.35, 7.0, 9.0)
        for name in ISF_NAMES | TP_NAMES:
            _, X = f.extract_trace(tb, _mz(ions[name]), 0.35, 7.0, 9.0)
            s = f.profile_similarity(rt, X, P, n_boot=0, n_shift=60)
            lab = int(name in ISF_NAMES)
            labels.append(lab)
            nz = lambda v, d=-1.0: d if v is None or not math.isfinite(v) else v
            scores["pearson"].append(nz(s.get("pearson")))
            scores["spearman"].append(nz(s.get("spearman")))
            scores["deriv"].append(nz(s.get("deriv_corr")))
            scores["apex"].append(-abs(nz(s.get("apex_diff_s"), 60.0)))
            pk = s.get("p_peak")
            rfit = f.ratio_fit(rt, X, P, (pk["lo"], pk["hi"])) if pk else {"ok": False}
            scores["cv"].append(-rfit["cv_ratio"] if rfit.get("ok") and math.isfinite(rfit["cv_ratio"]) else -5.0)
    for k, v in scores.items():
        a = f.auc(v, labels)
        assert a > 0.9, (k, a)


# ----------------------------------------------------------------------------------------------------------------------- ratio
def test_ratio_fit_recovers_the_isf_ratio():
    tb, ions = _scene(5.0, 4)
    rt, P = f.extract_trace(tb, _mz(ions["P"]), 0.35, 7.0, 9.0)
    _, X = f.extract_trace(tb, _mz(ions["ISF_big"]), 0.35, 7.0, 9.0)
    pk = f.detect_peaks(rt, P)["peaks"][0]
    r = f.ratio_fit(rt, X, P, (pk["lo"], pk["hi"]))
    assert r["ok"] and abs(r["r_tls"] - 0.80) < 0.06 and abs(r["r_ts"] - 0.80) < 0.1 and abs(r["r_median"] - 0.80) < 0.08
    assert r["r2"] > 0.95 and r["cv_ratio"] < 0.2 and abs(r["intercept_rel"]) < 0.1 and r["r_se"] < 0.05
    assert len(r["points"]["p"]) == r["n"]


def test_ratio_constancy_between_samples():
    ratios, ses = [], []
    for t, tb, ions in demo.isf_series((0, 5, 10, 15, 30), 7):
        ions = {i["name"]: i for i in ions}
        rt, P = f.extract_trace(tb, _mz(ions["P"]), 0.35, 7.0, 9.0)
        _, X = f.extract_trace(tb, _mz(ions["ISF_big"]), 0.35, 7.0, 9.0)
        pk = f.detect_peaks(rt, P)["peaks"][0]
        r = f.ratio_fit(rt, X, P, (pk["lo"], pk["hi"]), n_boot=40)
        ratios.append(r["r_tls"])
        ses.append(max(r["r_se"], 0.01))
    c = f.ratio_constancy(ratios, ses)
    assert c["cv"] < 0.1 and c["p"] > 0.001
    changing = f.ratio_constancy([0.1, 0.3, 0.6, 0.9, 1.4], [0.02] * 5)
    assert changing["p"] < 1e-6 and changing["cv"] > 0.5


def test_chi2_sf_matches_known_values():
    assert abs(f.chi2_sf(3.841, 1) - 0.05) < 1e-3
    assert abs(f.chi2_sf(11.07, 5) - 0.05) < 1e-3
    assert f.chi2_sf(0, 3) == 1.0


# ----------------------------------------------------------------------------------------------------------------------- kinetics
def test_kinetics_parent_ion_and_product():
    t = np.array([0, 5, 10, 15, 30, 45, 60], float)
    k1, k2 = 0.06, 0.02
    P = 1e6 * np.exp(-k1 * t)
    isf = 0.3 * P
    tp = 5e5 * k1 / (k2 - k1) * (np.exp(-k1 * t) - np.exp(-k2 * t))
    rng = np.random.default_rng(0)
    r_isf = f.kinetics(t, P * rng.normal(1, 0.02, 7), isf * rng.normal(1, 0.03, 7))
    assert r_isf["spearman_parent"] > 0.95 and r_isf["slope_sign"] == -1
    assert abs(r_isf["parent_decay"]["k"] - k1) < 0.01 and r_isf["parent_decay"]["k_ci"][0] < k1 < r_isf["parent_decay"]["k_ci"][1] + 0.01
    assert max(r_isf["ratio"][:-1]) / min(r_isf["ratio"][:-1]) < 1.2
    r_tp = f.kinetics(t, P * rng.normal(1, 0.02, 7), tp * rng.normal(1, 0.03, 7))
    assert r_tp["spearman_parent"] < 0 and r_tp["spearman_time"] > 0 or r_tp["kendall_time"] < 0.5
    m = r_tp["models"]
    assert m["formation_decay"]["sse"] < m["tracks_parent"]["sse"]
    assert abs(m["formation_decay"]["k2"] - k2) < 0.02
    assert m["formation_decay"]["t_max"] > 5


def test_kinetics_few_points_warns_and_does_not_crash():
    r = f.kinetics([0, 10, 30], [1, 0.5, 0.1], [1, 0.4, 0.2])
    assert r["warnings"] and r["n"] == 3
    assert f.kinetics([0, 10], [1, 0.5], [1, 0.4])["warnings"]
    d = f.fit_decay(np.array([0, 5, 10, 20.0]), np.array([1, 0.7, 0.5, 0.25]))
    assert not d["reliable"] and d["k"] > 0
    assert f.kendall_tau([1, 2, 3, 4], [1, 2, 3, 4]) == 1.0 and f.kendall_tau([1, 2, 3], [3, 2, 1]) == -1.0


# ----------------------------------------------------------------------------------------------------------------------- families, roles
def test_roles_isotope_adduct_fragment_and_formula_constraint():
    P = 364.35
    roles = {r["label"]: r for r in f.annotate_roles(P + 1.0, P)}
    assert "M+1 of P" in roles and roles["M+1 of P"]["role"] == "isotope"
    assert any(r["label"] == "[M+Na]+" for r in f.annotate_roles(P + 21.98, P))
    assert any(r["label"] == "[2M+H]+" and r["role"] == "dimer" for r in f.annotate_roles(2 * (P - 1.007) + 1.007, P))
    fr = f.annotate_roles(P - 18.01, P, "C14H13F4N3O2S2")
    h2o = [r for r in fr if r["label"] == "P - H2O"]
    assert h2o and h2o[0]["formula_ok"] and h2o[0]["fragment_formula"] == "C14H11F4N3OS2"
    # a fragment that would need more atoms than the parent has is refused
    ok, _ = f._subformula("C3H6O", ["C6H6"])
    assert not ok
    assert f.loss_candidates(17.03)[0]["loss"] == "NH3"


def test_isotope_pattern_and_chlorine_check():
    p = f.isotope_pattern("C9H10Cl2N2O")           # two Cl: M+2/M ~ 0.64
    assert 0.55 < p[2] / p[0] < 0.75
    one = f.isotope_pattern("C9H10ClN2O")
    assert 0.28 < one[2] / one[0] < 0.4
    assert f.isotope_pattern("C9H10N2O")[2] < 0.05


def test_families_group_isf_with_parent_and_kinetics_separates_the_coeluting_product():
    series = demo.isf_series((0, 5, 10, 15, 30, 45, 60), 5)
    tb, ions = series[3][1], {i["name"]: i for i in series[3][2]}
    w = f.build_rois(tb, 7.4, 8.8)
    mz = lambda n: _mz(ions[n])
    # kinetic profile (areas per sample) of every ROI of the reference window
    kin = np.array([[f.peak_area(*f.extract_trace(t_b, m, 0.35, 7.4, 8.8)[::1], 0, len(f.extract_trace(t_b, m, 0.35, 7.4, 8.8)[0]) - 1)
                     for _, t_b, _ in series] for m in w.mz])
    fam = f.ion_families(w, kin=kin)
    def cl(n):
        for c in fam["clusters"]:
            if any(abs(m - mz(n)) < 0.2 for m in c["mz"]):
                return id(c)
    assert cl("P") is not None and cl("P") == cl("ISF_big") == cl("M+1")
    assert cl("TP_coelute") != cl("P")
    assert fam["dist"].shape[0] == len(fam["ions"])
    # without kinetics the same data still yields clusters (profile + apex only)
    assert f.ion_families(w)["clusters"]


# ----------------------------------------------------------------------------------------------------------------------- MCR
def _two_component_matrix(rng, noise=0.01, shift=0.5):
    n = 80
    t = np.arange(n)
    c1 = np.exp(-0.5 * ((t - 35) / 6) ** 2)
    c2 = np.exp(-0.5 * ((t - 35 + shift * 6) / 7) ** 2)
    s1 = np.array([1.0, 0.5, 0.0, 0.2, 0.0, 0.0, 0.1])
    s2 = np.array([0.0, 0.0, 1.0, 0.0, 0.6, 0.3, 0.1])
    D = np.outer(c1, s1) + 0.7 * np.outer(c2, s2) + rng.normal(0, noise, (n, 7))
    return np.maximum(D, 0), (c1, c2), (s1, s2)


def test_mcr_als_resolves_coeluting_components():
    D, (c1, c2), (s1, s2) = _two_component_matrix(np.random.default_rng(0))
    est = f.estimate_components(D)
    assert est["k"] == 2
    r = f.mcr_als(D, 2)
    assert r["lof"] < 5 and (r["C"] >= 0).all() and (r["S"] >= 0).all()
    cos = [max(f.cosine(r["S"][:, j], s) for j in range(2)) for s in (s1, s2)]
    assert min(cos) > 0.92
    # unimodal elution profiles
    for j in range(2):
        c = r["C"][:, j]
        k = int(np.argmax(c))
        assert (np.diff(c[:k + 1]) >= -1e-9 * c.max()).all() and (np.diff(c[k:]) <= 1e-9 * c.max()).all()
    st = f.mcr_stability(D, 2, n_starts=4)
    assert min(st["cos_min"]) > 0.9
    n = f.nmf(D, 2)
    assert n["lof"] < 8


def test_isotonic_up_down_and_simplisma():
    c = np.array([0, 1, 3, 2, 4, 6, 5, 3, 1, 2, 0.0])
    u = f._isotonic_up_down(c)
    k = int(np.argmax(u))
    assert (np.diff(u[:k + 1]) >= -1e-12).all() and (np.diff(u[k:]) <= 1e-12).all()
    D, _, _ = _two_component_matrix(np.random.default_rng(1))
    assert set(f.simplisma(D, 2)) & {0, 1, 3} and set(f.simplisma(D, 2)) & {2, 4, 5}


def test_mcr_separates_isobaric_isf_from_coeluting_tp():
    tb, ions = _scene(30.0, 11)
    w = f.build_rois(tb, 7.6, 8.5)
    res = f.deconvolve_window(w, 0, len(w.rt) - 1, k=2, min_snr=8, stability=False)
    assert res["ok"] and res["k"] == 2 and res["lof"] < 30
    S = np.array(res["S"])
    mz = np.array(res["mz"])
    def comp_of(name):
        j = int(np.argmin(np.abs(mz - _mz(ions[name]))))
        return int(np.argmax(S[:, j]))
    # parent, its ISF fragments and the M+1 end up in the same component, the coeluting product in the other
    assert comp_of("P") == comp_of("ISF_big") == comp_of("M+1")
    assert comp_of("TP_coelute") != comp_of("P")
    # the isobaric ion (346) has weight in both components: the parent's family and the product's
    j = int(np.argmin(np.abs(mz - _mz(ions["ISF_H2O"]))))
    w2 = S[:, j] / S[:, j].sum()
    assert w2.min() > 0.05


# ----------------------------------------------------------------------------------------------------------------------- ramp, MS2
def test_source_ramp_isf_vs_tp():
    dp = np.array([20, 40, 60, 80, 100, 120, 140.0])
    P = 1e6 * (1 - 0.7 / (1 + np.exp(-(dp - 90) / 12)))
    F = 1e6 * 0.7 / (1 + np.exp(-(dp - 90) / 12)) * 0.8
    isf = f.source_ramp(dp, F, P)
    assert isf["spearman"] > 0.9 and isf["sigmoid"]["r2"] > 0.95 and 70 < isf["sigmoid"]["dp50"] < 110
    tp = f.source_ramp(dp, np.full(7, 3e5) * np.random.default_rng(0).normal(1, 0.02, 7), np.full(7, 1e6))
    assert abs(tp["dp_dependence"]) < 0.1
    assert f.source_ramp([20, 40], [1, 2], [3, 4])["warnings"]


def test_ms2_similarity_cosine_and_modified_cosine():
    p_spec = (np.array([194.0, 152.0, 124.0, 346.0]), np.array([100.0, 40.0, 20.0, 15.0]))
    # product spectrum of an ISF fragment that is itself a fragment of P: shares 152 and 124
    x_spec = (np.array([152.0, 124.0, 96.0]), np.array([100.0, 50.0, 10.0]))
    r = f.ms2_similarity(194.0, x_spec, 364.0, p_spec)
    assert r["x_in_p"] and r["cosine"] > 0.5 and r["modified_cosine"] >= r["cosine"] and r["subset_score"] > 0.8
    other = (np.array([210.0, 180.0]), np.array([100.0, 30.0]))
    r2 = f.ms2_similarity(250.0, other, 364.0, p_spec)
    assert not r2["x_in_p"] and r2["cosine"] < 0.1
    assert math.isnan(f.ms2_similarity(1, (np.zeros(0), np.zeros(0)), 2, p_spec)["cosine"])
    # a shifted spectrum (loss of 18 from the precursor) is found by the modified cosine
    sh = (p_spec[0] - 18.0, p_spec[1])
    assert f.ms2_similarity(346.0, sh, 364.0, p_spec)["modified_cosine"] > f.ms2_similarity(346.0, sh, 364.0, p_spec)["cosine"]


def _greedy_reference(a, ya, b, yb, tol, shifts):
    """Plain global greedy (the definition): heaviest candidate pair first, each peak used once."""
    cand = sorted(((ya[i] * yb[j], -i, -j) for i in range(len(a)) for j in range(len(b)) if any(abs(b[j] - a[i] - d) <= tol for d in shifts)), reverse=True)
    ua, ub, out = set(), set(), []
    for w, i, j in cand:
        if -i not in ua and -j not in ub:
            ua.add(-i)
            ub.add(-j)
            out.append((-i, -j))
    return out


def test_modified_cosine_matches_shifted_peaks_jointly():
    # X has one strong peak (102). Unshifted it could take P's weak 102.2; shifted by the precursor difference (18) it takes P's strong 120.
    # The old order (everything at shift 0 first) paired it with 102.2 and left the strong shared fragment unmatched.
    p = (np.array([102.2, 120.0]), np.array([1.0, 100.0]))
    x = (np.array([102.0]), np.array([100.0]))
    r = f.ms2_similarity(300.0, x, 318.0, p, tol=0.3)
    # joint matching pairs X(102) with P(120) (weight 10*10), not with P(102.2) (weight 10*1)
    assert r["modified_cosine"] == pytest.approx(10 * 10 / (10 * math.sqrt(1 + 100)))
    assert r["cosine"] == pytest.approx(10 * 1 / (10 * math.sqrt(1 + 100)))
    assert r["modified_cosine"] > 5 * r["cosine"]


def test_match_is_the_global_greedy_and_the_dense_kernel_agrees():
    rng = np.random.default_rng(5)
    for _ in range(40):
        a, b = np.sort(rng.uniform(50, 60, 14)), np.sort(rng.uniform(50, 60, 17))
        ya, yb = rng.uniform(0.1, 5, 14), rng.uniform(0.1, 5, 17)
        for shifts in ((0.0,), (0.0, 1.7)):
            got = f._match(a, ya, b, yb, 0.4, shifts[0], shifts[1] if len(shifts) > 1 else None)
            assert got == _greedy_reference(a, ya, b, yb, 0.4, shifts)
            S = np.zeros((14, 17))
            ci, cj = f._candidates(a, b, 0.4, shifts)
            S[ci, cj] = ya[ci] * yb[cj]
            tot, n = f.greedy_match(S)
            assert n == len(got) and tot == pytest.approx(sum(ya[i] * yb[j] for i, j in got), rel=1e-9)


def test_greedy_match_batched_and_empty():
    S = np.zeros((3, 4, 5))
    S[1, 0, 0], S[1, 1, 0], S[2, 2, 3] = 3.0, 2.0, 7.0
    tot, n = f.greedy_match(S)
    assert n.tolist() == [0, 1, 1] and tot == pytest.approx([0.0, 3.0, 7.0])
    assert f._match([], [], [1.0], [1.0], 0.5) == [] and f._match([1.0], [1.0], [9.0], [1.0], 0.5) == []


def test_roc_auc_and_thresholds():
    rng = np.random.default_rng(0)
    y = np.r_[np.ones(60), np.zeros(60)].astype(int)
    s = np.r_[rng.normal(1, 0.5, 60), rng.normal(-1, 0.5, 60)]
    assert f.auc(s, y) > 0.95 and abs(f.auc(rng.normal(0, 1, 120), y) - 0.5) < 0.15
    assert f.auc([1, 1, 1], [1, 0, 1]) == 0.5 and math.isnan(f.auc([1, 2], [1, 1]))
    y_t = f.youden_threshold(s, y)
    assert -1 < y_t["thr"] < 1 and y_t["j"] > 0.8
    cv = f.cv_threshold(s, y)
    assert cv["balanced_accuracy"] > 0.9 and cv["thr_sd"] < 0.5


# ----------------------------------------------------------------------------------------------------------------------- report
def _samples(series):
    return [{"label": "t%g" % t, "time": float(t), "table": tb, "key": ("syn", t)} for t, tb, _ in series]


def test_origin_report_is_serializable_and_has_no_verdict():
    series = demo.isf_series((0, 5, 10, 15, 30, 45, 60), 21)
    rep = f.origin_report(_samples(series), 194.0 + OFF, 364.0 + OFF)
    json.dumps(rep)
    assert abs(rep["parent_apex_rt"] - 8.0) < 0.1
    assert rep["profile"]["pearson"] > 0.95 and rep["ratio"]["ok"]
    assert rep["ratio_constancy"]["cv"] < 0.15
    assert rep["kinetics"]["spearman_parent"] > 0.9
    assert rep["candidates"] and rep["candidates"][0]["pearson"] >= rep["candidates"][-1]["pearson"]
    assert any(c["mz"] > 340 for c in rep["candidates"])
    for banned in ("verdict", "is_isf", "classification", "probability"):
        assert banned not in json.dumps(rep).lower()
    assert rep["note"]["key"] == "of.note"
    assert rep["timing"]["total_s"] < 10


def test_origin_report_isobaric_ion_is_not_constant_across_samples():
    series = demo.isf_series((0, 5, 10, 15, 30, 45, 60), 31)
    clean = f.origin_report(_samples(series), 194.0 + OFF, 364.0 + OFF)
    mixed = f.origin_report(_samples(series), 346.0 + OFF, 364.0 + OFF)
    assert mixed["ratio_constancy"]["cv"] > 3 * clean["ratio_constancy"]["cv"]
    assert mixed["ratio_constancy"]["p"] < clean["ratio_constancy"]["p"]


def test_origin_report_robust_to_missing_parent_and_empty_window():
    series = demo.isf_series((0, 15), 41)
    rep = f.origin_report(_samples(series), 194.3, 250.0)           # no parent at this m/z
    assert rep["warnings"]
    json.dumps(rep)
    assert f.origin_report([], 1.0, 2.0)["warnings"]


def test_saturated_parent_is_reported():
    tb, ions = demo.isf_scene(0.0, 5, saturate=1.2e6)
    rep = f.origin_report([{"label": "t0", "time": 0.0, "table": tb, "key": "sat"}], 194.0 + OFF, 364.0 + OFF)
    assert any(w["key"] == "of.warn.flatTop" for w in rep["warnings"])


# ----------------------------------------------------------------------------------------------------------------------- real data
def _real_dir():
    mzml = os.environ.get("MZLAB_MZML", os.environ.get("QQQ_MZML"))
    for c in (mzml, str(Path(__file__).resolve().parents[2] / "Data" / "mzML")):
        if c and (Path(c) / "B_FullMass-t0.mzML").exists():
            return Path(c)
    return None


def test_real_flufenacet_series_if_available():
    d = _real_dir()
    if d is None:
        pytest.skip("real mzML not available")
    from mzlab.project import guess_sample
    from mzlab.reader.mzml import Run
    samples = []
    for p in sorted(d.glob("B_FullMass-t*.mzML")):
        lab, t, _ = guess_sample(p.name)
        samples.append({"label": lab, "time": t, "table": Run(p).table(1, 1), "key": str(p)})
    rep = f.origin_report(samples, 194.25, 364.35)
    json.dumps(rep)
    assert 14.2 < rep["parent_apex_rt"] < 14.5
    assert rep["profile"]["pearson"] > 0.7 and abs(rep["profile"]["apex_diff_s"]) < 5
    assert rep["timing"]["total_s"] < 10


def test_api_origin_endpoint(tmp_path):
    import io
    from mzlab import api
    from mzlab.app import App
    folder = demo.make_demo(tmp_path / "d")
    app = App(tmp_path / "w")
    for x in sorted(folder.glob("demo_t*.mzML")):
        app.save_upload(x.name, io.BytesIO(x.read_bytes()), x.stat().st_size)
    app.open_session({"samples": [{"file": x.name} for x in sorted(folder.glob("demo_t*.mzML"))]})
    code, ctype, body, _ = api.dispatch(app, "GET", "/api/origin", {"k": "0", "mz": "253.1", "parent": "237.1"})
    d = json.loads(body)
    assert code == 200 and ctype == "application/json" and "profile" in d and "note" in d, d
    assert abs(d["parent_apex_rt"] - 6.0) < 0.2
    code, _, body, _ = api.dispatch(App(tmp_path / "w2"), "GET", "/api/origin", {"mz": "1", "parent": "2"})
    assert code == 500 and "error" in json.loads(body)


def test_ms2_membership():
    spec = (np.array([194.0, 152.0, 364.0]), np.array([100.0, 40.0, 5.0]))
    assert f.ms2_membership(194.2, 364.1, spec)["x_in_p"]
    m = f.ms2_membership(250.0, 364.1, spec)
    assert not m["x_in_p"] and m["nearest_product"] in (194.0, 152.0)
    assert f.ms2_membership(364.0, 364.1, spec)["x_in_p"] is False            # the precursor itself does not count
