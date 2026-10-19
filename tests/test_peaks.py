"""Peaks of profile spectra (one per nominal mass), merging of centroids, and the profile path of the API."""
import json
import re
import sys
from pathlib import Path

import numpy as np
import pytest

from mzlab import api
from mzlab.app import App
from mzlab.peaks import XIC_BELOW, XIC_DRIFT, merge_unit, nominal, pad_zeros, profile_peaks

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from dati_percorsi import percorso  # noqa: E402


def _gauss_profile(centres, heights, sigma=0.3, lo=300.0, hi=372.0, step=0.06, flat=()):
    mz = np.round(np.arange(lo, hi, step), 2)
    y = np.zeros(mz.size)
    for c, h in zip(centres, heights):
        y += h * (np.exp(-(((mz - c) / 0.42) ** 8)) if c in flat else np.exp(-0.5 * ((mz - c) / sigma) ** 2))
    return mz, y


def test_xic_constants_are_shared_with_the_front_end():
    js = "".join((ROOT / "mzlab" / "web" / n).read_text(encoding="utf-8") for n in ("explore.js", "pannelli.js", "grafici.js", "picchi.js", "eventi.js"))
    assert float(re.search(r"XIC_BELOW\s*=\s*([0-9.]+)", js).group(1)) == XIC_BELOW
    assert float(re.search(r"XIC_DRIFT\s*=\s*([0-9.]+)", js).group(1)) == XIC_DRIFT


def test_one_peak_per_nominal_mass_and_isotopes_stay_apart():
    mz, y = _gauss_profile([364.3, 365.35, 366.4], [1e6, 3e5, 1e5])
    pm, py = profile_peaks(mz, y)
    assert [int(np.floor(m + XIC_BELOW)) for m in pm] == [364, 365, 366]
    assert np.allclose(pm, [364.3, 365.35, 366.4], atol=0.2)
    assert py[0] > py[1] > py[2]


def test_flat_top_gives_one_peak():
    mz, y = _gauss_profile([364.3], [1e6], flat=(364.3,))
    y = y * (1 + 0.02 * np.sin(np.arange(y.size)))                 # a little ripple: a centroiding algorithm would split it
    pm, py = profile_peaks(mz, y)
    assert len(pm) == 1 and abs(pm[0] - 364.3) < 0.1


def test_the_tail_of_a_neighbour_is_not_a_peak():
    mz, y = _gauss_profile([364.0], [1e6], sigma=0.35)               # wide: its tail reaches into the window of 365
    pm, _ = profile_peaks(mz, y)
    assert len(pm) == 1 and nominal(pm[0]) == 364


def test_merge_of_centroids():
    mz, y = merge_unit([364.0, 364.4], [1e6, 2e6])
    assert len(mz) == 1 and abs(mz[0] - 364.2667) < 0.01 and y[0] == 3e6
    mz, y = merge_unit([364.4, 365.4], [1.0, 1.0])
    assert len(mz) == 2
    for n in (300, 364):                                             # window edge: n + 0.79 belongs to n, n + 0.81 to n + 1
        mz, y = merge_unit([n + 0.79, n + 0.81], [1.0, 1.0])
        assert len(mz) == 2 and np.allclose(mz, [n + 0.79, n + 0.81])
        mz, y = merge_unit([n - 0.19, n + 0.79], [1.0, 1.0])
        assert len(mz) == 1


def test_zeros_put_back_next_to_gaps():
    m, y = pad_zeros([100.0, 100.06, 100.5, 100.56], [5.0, 6.0, 7.0, 8.0], 0.06)
    assert list(np.round(m, 2)) == [100.0, 100.06, 100.12, 100.44, 100.5, 100.56] and list(y) == [5, 6, 0, 0, 7, 8]


def _real_profile():
    d = percorso("dati/3200 qtrap/mzML_profilo")
    return sorted(d.glob("B_FullMass-t0.mzML")) if d and d.is_dir() else []


def test_real_profile_file_isotopes_of_the_t0_are_separate():
    f = _real_profile()
    if not f:
        pytest.skip("profile files of the lab not available")
    from mzlab.explore import Item
    it = Item("x", None, 0, "sample", f[0])
    assert it.spectrum_mode() == "profile" and it._step(1) == 0.06
    rt, tic = it.total("tic")
    r = rt[int(np.argmax(tic))]
    mz, y, _ = it.spectrum(r - 0.05, r + 0.05)
    pm, py = profile_peaks(mz, y)
    sel = {int(np.floor(m + XIC_BELOW)): (m, h) for m, h in zip(pm, py) if 363 < m < 368}
    assert {364, 365, 366} <= set(sel), sel
    assert 363.9 < sel[364][0] < 364.7 and sel[364][1] > sel[365][1] > sel[366][1]


def test_api_profile_and_centroid(tmp_path):
    sys.path.insert(0, str(ROOT / "tools"))
    import dati_sintetici as ds
    ds.full_scan_profile(tmp_path / "P.mzML", 15, np.random.default_rng(1))
    ds.full_scan(tmp_path / "C.mzML", 15, np.random.default_rng(1))
    app = App(tmp_path / "w")
    (tmp_path / "w").mkdir(exist_ok=True)
    for n in ("P.mzML", "C.mzML"):
        (tmp_path / "w" / n).write_bytes((tmp_path / n).read_bytes())
    app.open_session({"samples": [{"file": "P.mzML"}, {"file": "C.mzML"}]})
    info = {i["file"]: i for i in app.session.info()}
    assert info["P.mzML"]["spectrum_mode"] == "profile" and info["C.mzML"]["spectrum_mode"] == "centroid"

    def get(path, **q):
        code, _, body, _ = api.dispatch(app, "GET", path, {k: str(v) for k, v in q.items()})
        assert code == 200
        return json.loads(body)
    p = get("/api/spectrum", k=0, rt0=14.6, rt1=14.8, level=1)
    assert p["mode"] == "profile" and len(p["pmz"]) > 100 and p["pmz"] == sorted(p["pmz"])
    near = [m for m in p["mz"] if 304.8 <= m < 305.8]
    assert len(near) == 1, "the flat-top ion at 305 is ONE peak"
    one = get("/api/spectra", k=0, i0=150, i1=150, level=1)["scans"][0]
    rt = one["rt"]
    w = get("/api/spectrum", k=0, rt0=rt - 1e-3, rt1=rt + 1e-3, level=1)
    assert one["mz"] == w["mz"] and one["y"] == w["y"] and one["pmz"] == w["pmz"]
    c = get("/api/spectrum", k=1, rt0=14.2, rt1=14.4, level=1)
    cm = get("/api/spectrum", k=1, rt0=14.2, rt1=14.4, level=1, merge=1)
    assert c["mode"] == "centroid" and "pmz" not in c and len(cm["mz"]) <= len(c["mz"])
    nom = [int(np.floor(m + XIC_BELOW)) for m in cm["mz"]]
    assert len(nom) == len(set(nom)), "merged: one peak per nominal mass"
