"""Statistics of a series (mzlab/statistica.py): time profile, blank subtraction, PCA: known values on synthetic data."""
import io
import json

import numpy as np
import pytest

from mzlab import statistica as st
from mzlab.api import dispatch
from mzlab.i18n import UserError


def pts(**kw):
    base = [("a", 60.0, 10.0, 100.0), ("b", 0.0, 20.0, 200.0), ("c", None, 5.0, 50.0), ("d", 30.0, None, None)]
    return [{"file": f, "time": t, "rt": 8.5, "area": a, "height": h} for f, t, a, h in base]


def test_profile_orders_by_time_and_keeps_the_files_without_time_last():
    r = st.time_profile(pts(), "area")
    assert [x["file"] for x in r["rows"]] == ["b", "d", "a", "c"]            # 0, 30, 60 minutes, then the file without a time
    assert [r["rows"][i]["file"] for i in r["series"]] == ["b", "a"]         # d has no peak, c has no time: not joined
    assert r["rows"][0]["value"] == 20.0 and r["normalized"] is False
    h = st.time_profile(pts(), "height")
    assert h["rows"][0]["value"] == 200.0


def test_profile_normalizes_on_the_internal_standard_of_the_same_file():
    istd = {"a": {"area": 50.0, "height": 10.0}, "b": {"area": 10.0, "height": 40.0}, "c": {"area": 0.0, "height": 1.0}}
    r = st.time_profile(pts(), "area", istd)
    by = {x["file"]: x for x in r["rows"]}
    assert by["a"]["norm"] == pytest.approx(10.0 / 50.0) and by["b"]["norm"] == pytest.approx(2.0)
    assert by["c"]["norm"] is None and by["d"]["norm"] is None             # zero or missing standard: no ratio
    assert [r["rows"][i]["file"] for i in r["series"]] == ["b", "a"] and r["normalized"] is True


def test_profile_rejects_an_unknown_metric():
    with pytest.raises(UserError):
        st.time_profile(pts(), "intensity")


def test_profile_route():
    body = json.dumps({"points": pts(), "metric": "area"}).encode()
    code, _, out, _ = dispatch(None, "POST", "/api/stat/profile", {}, io.BytesIO(body), len(body))
    assert code == 200 and json.loads(out)["series"] == [0, 2]


# ---------------------------------------------------------------------------------------------------- blank subtraction
ROLES = ["blank", "blank", "sample", "sample", "sample", "dark"]
M = [
    [100, 300, 0, 0, 0, 5000],           # 0 blank only; the dark file is neither blank nor sample
    [10, 20, 500, 800, 900, 0],          # 1 clearly above the blank
    [0, 0, 50, 0, 0, 0],                 # 2 absent from the blank, one detection
    [100, 100, 250, 290, 310, 0],        # 3 max blank 100 -> threshold 300 (x3): only one sample above
    [None, 0, 1, 2, 3, 0],               # 4 missing values count as 0
]


def test_blank_filter_known_values():
    r = st.blank_filter(M, ROLES, factor=3, stat="max", min_detect=1)
    assert r["kept"] == [1, 2, 3, 4] and r["discarded"] == [0]               # only the feature of the blank (the dark file is ignored) goes
    assert [(x["blank"], x["threshold"], x["n_above"]) for x in r["rows"]] == [(300.0, 900.0, 0), (20.0, 60.0, 3), (0.0, 0.0, 1), (100.0, 300.0, 1), (0.0, 0.0, 3)]
    assert r["rows"][1]["max_sample"] == 900.0 and r["n_blank"] == 2 and r["n_sample"] == 3


def test_blank_filter_minimum_of_detections_and_mean_level():
    assert st.blank_filter(M, ROLES, 3, "max", 2)["kept"] == [1, 4]           # two detections are needed: features 2 and 3 have one
    r = st.blank_filter(M, ROLES, 3, "mean", 1)
    assert r["rows"][0]["blank"] == 200.0 and r["rows"][1]["threshold"] == pytest.approx(45.0)
    assert st.blank_filter(M, ROLES, 1, "max", 3)["kept"] == [1, 3, 4]        # factor 1: feature 3 (blank 100) is above in all three samples


def test_blank_filter_needs_a_blank_and_a_sample():
    with pytest.raises(UserError):
        st.blank_filter(M, ["sample"] * 6)
    with pytest.raises(UserError):
        st.blank_filter(M, ["blank"] * 6)
    with pytest.raises(UserError):
        st.blank_filter(M, ROLES, factor=-1)
    with pytest.raises(UserError):
        st.blank_filter(M, ROLES, stat="median")


def test_blank_route():
    body = json.dumps({"matrix": M, "roles": ROLES, "factor": 3, "stat": "max", "min_detect": 2}).encode()
    code, _, out, _ = dispatch(None, "POST", "/api/stat/blank", {}, io.BytesIO(body), len(body))
    assert code == 200 and json.loads(out)["kept"] == [1, 4]


# ---------------------------------------------------------------------------------------------------- PCA
def trajectory(n=8, p=12, seed=0, noise=0.05):
    """Synthetic time series: growing and decaying features, log-normal noise (features x samples)."""
    rng = np.random.default_rng(seed)
    t = np.arange(n)
    rows = []
    for _ in range(p):
        a, k = rng.uniform(1e4, 1e5), rng.choice([-1, 1]) * rng.uniform(0.1, 0.3)
        rows.append(list(a * np.exp(k * t) * rng.lognormal(0, noise, n)))
    return rows


def test_pca_recovers_a_known_component():
    # rank one: x[f][s] = base_f + v_f * z_s -> without transform or scaling PC1 is v/|v| exactly and explains everything
    v = np.array([3.0, -1.0, 2.0, 0.5])
    z = np.array([-3.0, -1.0, 0.0, 1.0, 3.0, 0.0])
    z = z - z.mean()
    base = np.array([1e4, 1e5, 5e4, 2e4])
    m = (base[:, None] + v[:, None] * z[None, :]).tolist()
    r = st.pca_analysis(m, ["sample"] * 6, transform="none", scaling="none")
    pc1 = np.array([row[0] for row in r["loadings"]])
    assert np.allclose(np.abs(pc1), np.abs(v) / np.linalg.norm(v), atol=1e-9)
    assert r["explained"][0] == pytest.approx(1.0, abs=1e-12)
    sc = np.array([row[0] for row in r["scores"]])
    assert np.allclose(np.abs(sc), np.abs(z) * np.linalg.norm(v), atol=1e-9)
    assert pc1[np.argmax(np.abs(pc1))] > 0                                   # sign convention: the largest loading is positive


def test_pca_explained_variance_matches_the_covariance_eigenvalues():
    m = trajectory(seed=3)
    r = st.pca_analysis(m, ["sample"] * 8, transform="log", scaling="pareto")
    z = np.log2(np.array(m)).T
    z = z - z.mean(axis=0)
    z = z / np.sqrt(z.std(axis=0, ddof=1))
    ev = np.sort(np.linalg.eigvalsh(np.cov(z.T)))[::-1]
    assert np.allclose(r["explained"], (ev / ev.sum())[: len(r["explained"])], atol=1e-9)
    assert sum(r["explained"]) <= 1.0 + 1e-9 and r["explained"][0] > 0.9      # a time trajectory: one dominant axis


def test_pca_trajectory_is_ordered_in_time():
    r = st.pca_analysis(trajectory(seed=1), ["sample"] * 8)
    pc1 = np.array([row[0] for row in r["scores"]])
    d = np.diff(pc1)
    assert (d > 0).all() or (d < 0).all(), pc1                                # PC1 follows the time order


def test_pca_filters_come_first_and_are_counted():
    base = [r + [1e3] for r in trajectory(n=6, p=6, seed=2)]                    # six samples and one blank column
    contaminant = [9e5] * 7                                                     # as high in the samples as in the blank
    faint = [50, 60, 55, 40, 70, 65, 10]                                        # above the blank, under the intensity threshold
    rare = [0, 0, 0, 0, 0, 7e4, 0]                                              # present in one sample only
    r = st.pca_analysis(base + [contaminant, faint, rare], ["sample"] * 6 + ["blank"], factor=3, min_intensity=1000, min_samples=2)
    n = {s["step"]: s["n"] for s in r["steps"]}
    assert n == {"table": 9, "blank": 8, "intensity": 7, "presence": 6}, n
    assert r["features"] == list(range(6)) and r["samples"] == list(range(6))
    assert r["steps"][1]["applied"] is True


def test_pca_without_blank_says_so():
    r = st.pca_analysis(trajectory(), ["sample"] * 8)
    assert r["steps"][1] == {"step": "blank", "n": 12, "applied": False}


def test_imputation_is_a_fifth_of_the_minimum_and_counted():
    x = np.array([[0.0, 10.0, 20.0], [5.0, 6.0, 7.0], [0.0, 0.0, 4.0]])
    y, n = st.impute(x)
    assert n == 3 and y[0, 0] == pytest.approx(2.0) and y[2, 0] == pytest.approx(0.8) and (y[1] == x[1]).all()
    r = st.pca_analysis([[0, 1e4, 2e4, 3e4, 4e4, 5e4], [5e4, 4e4, 3e4, 2e4, 1e4, 1e4 + 1], [1e4, 3e4, 2e4, 5e4, 1e4, 4e4]], ["sample"] * 6)
    assert r["imputed"] == 1


def test_pca_is_off_below_five_samples():
    with pytest.raises(UserError) as e:
        st.pca_analysis(trajectory(n=4), ["sample"] * 4)
    assert e.value.key == "err.stat.pca.few" and e.value.params == {"n": 4, "min": 5}
    with pytest.raises(UserError):
        st.pca_analysis(trajectory(n=6), ["sample"] * 4 + ["blank", "dark"])    # blank and dark files do not count


def test_pca_rejects_unknown_scaling():
    with pytest.raises(UserError):
        st.pca_analysis(trajectory(), ["sample"] * 8, scaling="range")


def test_loadings_stability_flags_a_feature_driven_by_one_sample():
    m = trajectory(seed=0)
    out = [1e3] * 8
    out[3] = 1e7
    r = st.pca_analysis(m + [out], ["sample"] * 8)
    s = np.array(r["stability"])
    assert s[-1] < 0.6 < np.median(s[:-1]) and np.median(s[:-1]) > 0.9, s


def test_ellipses_need_three_replicates():
    m = trajectory(n=9, p=10, seed=4)
    groups = ["a", "a", "a", "b", "b", "b", "c", "c", "x"]
    r = st.pca_analysis(m, ["sample"] * 9, groups)
    assert set(r["ellipses"]) == {"a", "b"}                                     # c has two samples, x one
    e = r["ellipses"]["a"]
    pts = np.array(r["scores"])[[0, 1, 2]][:, :2]
    assert e["cx"] == pytest.approx(pts[:, 0].mean()) and e["a"] >= e["b"] >= 0
    assert st.pca_analysis(m, ["sample"] * 9)["ellipses"] == {}


def test_pca_route_and_golden():
    import pathlib
    g = json.loads((pathlib.Path(__file__).parent / "golden" / "statistica_pca.json").read_text())
    body = json.dumps(g["input"]).encode()
    code, _, out, _ = dispatch(None, "POST", "/api/stat/pca", {}, io.BytesIO(body), len(body))
    r = json.loads(out)
    assert code == 200
    assert np.allclose(r["explained"], g["explained"], atol=1e-9)
    assert np.allclose(np.abs(r["scores"]), np.abs(g["scores"]), atol=1e-8) and np.allclose(np.abs(r["loadings"]), np.abs(g["loadings"]), atol=1e-8)
    assert np.allclose(r["stability"], g["stability"], atol=1e-8) and r["steps"] == g["steps"] and r["imputed"] == g["imputed"]
