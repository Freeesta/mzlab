"""Difference from a reference (mzlab/differenza.py and the `ref` parameters of /api/xic, /api/chrom, /api/mrm, /api/spectrum)."""
import json
import sys
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import dati_sintetici as ds  # noqa: E402
from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402
from mzlab.differenza import area, norm_factor, subtract_spectrum, subtract_trace  # noqa: E402


# ---------------------------------------------------------------------------------------------------------------- pure functions
def test_trace_plain_difference_and_golden_values():
    x = np.array([0.0, 1.0, 2.0, 3.0])
    d = subtract_trace(x, [10, 20, 30, 40], x, [1, 2, 3, 4], 2.0, 0.0)
    assert d.tolist() == [8, 16, 24, 32]


def test_trace_negative_is_kept_unless_clipped():
    x = np.array([0.0, 1.0, 2.0])
    assert subtract_trace(x, [1, 1, 1], x, [5, 5, 5], 1.0, 0.0).tolist() == [-4, -4, -4]
    assert subtract_trace(x, [1, 1, 1], x, [5, 5, 5], 1.0, 0.0, clip=True).tolist() == [0, 0, 0]


def test_trace_outside_the_reference_range_is_nan_not_zero():
    d = subtract_trace([0.0, 1.0, 5.0], [3, 3, 3], [0.5, 1.5], [1, 1], 1.0, 0.0)
    assert np.isnan(d[0]) and np.isnan(d[2]) and d[1] == pytest.approx(2.0)


def test_trace_rt_tolerance_cancels_a_drift():
    t = np.arange(0, 10, 0.01)
    peak = lambda c: 100 * np.exp(-0.5 * ((t - c) / 0.03) ** 2)
    plain = subtract_trace(t, peak(5.0), t, peak(5.04), 1.0, 0.0)
    tol = subtract_trace(t, peak(5.0), t, peak(5.04), 1.0, 0.1)
    assert np.abs(plain).max() > 20 and tol.max() <= 1e-9          # a drift of 0.04 min leaves a pair of signs; the tolerance leaves the ones only the sample has
    assert tol.min() < 0                                           # what is lower in the sample than in the reference stays visible


def test_norm_factor():
    assert norm_factor("none", 5, 2) == 1.0
    assert norm_factor("tic", 6, 3) == 2.0
    assert norm_factor("istd", 0, 3) == 1.0
    with pytest.raises(ValueError):
        norm_factor("area", 1, 1)
    assert area([0, 1, 2], [0, 2, 0]) == 2.0


def test_spectrum_low_resolution_nominal_cells_and_reference_only_lines():
    mz, y = subtract_spectrum([253.3, 301.3], [100.0, 50.0], [253.1, 150.2], [40.0, 30.0], 1.0, unit=True)
    assert dict(zip(np.round(mz, 1), y)) == {150.2: -30.0, 253.3: 60.0, 301.3: 50.0}     # 253.1 and 253.3 are the same nominal mass
    mz, y = subtract_spectrum([253.3], [10.0], [253.3], [40.0], 1.0, unit=True, clip=True)
    assert len(mz) == 0


def test_spectrum_high_resolution_ppm():
    mz, y = subtract_spectrum([200.0000, 300.0], [10.0, 20.0], [200.0004, 400.0], [4.0, 7.0], 1.0, unit=False, tol=5.0)       # 2 ppm apart: the same line
    assert dict(zip(np.round(mz, 3), y)) == {200.0: 6.0, 300.0: 20.0, 400.0: -7.0}
    mz, y = subtract_spectrum([200.0000], [10.0], [200.01], [4.0], 1.0, unit=False, tol=5.0)                                  # 50 ppm: another line
    assert sorted(y) == [-4.0, 10.0]


# ---------------------------------------------------------------------------------------------------------------- the routes
@pytest.fixture(scope="module")
def app(tmp_path_factory):
    d = tmp_path_factory.mktemp("sub")
    ds.subtraction_series(d)
    a = App(d)
    a.open_session({"samples": [{"file": f"SUB_{n}.mzML", "type": "blank" if n == "blank" else "sample", "time": t}
                                for n, t in (("t0", 0), ("t10", 10), ("t20", 20), ("blank", None))]})
    return a                                                # files 0 = t0, 1 = t10, 2 = t20, 3 = blank


def get(app, path, **q):
    code, _, body, _ = dispatch(app, "GET", path, {k: str(v) for k, v in q.items()})
    return code, json.loads(body)


def at(j, t):
    i = int(np.argmin(np.abs(np.array(j["rt"]) - t)))
    return j["y"][i]


def test_xic_difference_parent_decays_product_appears(app):
    code, j = get(app, "/api/xic", k=1, mz=253.3, tol=0.5, level=1, ref=0, rttol=0.15)
    assert code == 200 and j["ref"] == 0
    tr = j["traces"][0]
    assert at(tr, 5.25) < -1e5                               # the parent is lower at t10 than at t0: negative, not cut
    code, j = get(app, "/api/xic", k=1, mz=301.3, tol=0.5, level=1, ref=0, rttol=0.15)
    assert at(j["traces"][0], 6.45) > 1e5                    # the product only exists at t10
    code, j = get(app, "/api/xic", k=1, mz=253.3, tol=0.5, level=1, ref=0, rttol=0.15, clip=1)
    assert min(j["traces"][0]["y"]) >= 0


def test_xic_factor_and_blank_line_cancels(app):
    _, a = get(app, "/api/xic", k=1, mz=150.2, tol=0.5, level=1, ref=3, rttol=0.1)
    _, b = get(app, "/api/xic", k=1, mz=150.2, tol=0.5, level=1, ref=3, rttol=0.1, factor=0)
    assert abs(at(a["traces"][0], 3.0)) < 0.2 * abs(at(b["traces"][0], 3.0))            # the background line is in both: it cancels
    assert at(b["traces"][0], 3.0) > 1e5                                                 # factor 0: the trace itself


def test_chrom_tic_normalization_follows_the_gain(app):
    _, raw = get(app, "/api/chrom", k=2, kind="tic", level=1, ref=0, rttol=0.1)
    _, nrm = get(app, "/api/chrom", k=2, kind="tic", level=1, ref=0, rttol=0.1, norm="tic")
    assert nrm["scale"] > 1.0 and raw["scale"] == 1.0
    _, w = get(app, "/api/xic", k=2, mz=150.2, tol=0.5, level=1, ref=0, rttol=0.1)
    _, v = get(app, "/api/xic", k=2, mz=150.2, tol=0.5, level=1, ref=0, rttol=0.1, norm="tic")
    assert abs(at(v["traces"][0], 3.0)) < abs(at(w["traces"][0], 3.0))                  # gain 1.3 on the line: the normalization takes it away


def test_istd_needs_its_mz_and_errors_are_json(app):
    code, j = get(app, "/api/xic", k=1, mz=253.3, tol=0.5, level=1, ref=0, norm="istd")
    assert code == 400 and "error" in j


def test_spectrum_difference_keeps_negatives(app):
    code, j = get(app, "/api/spectrum", k=1, rt0=5.0, rt1=5.5, level=1, bgk=0, bgrt0=5.0, bgrt1=5.5, bgf=1, bgclip=0)
    assert code == 200 and j["diff"] is True
    by = {round(m): v for m, v in zip(j["mz"], j["y"])}
    assert by[253] < 0                                                                   # the parent has fallen
    _, c = get(app, "/api/spectrum", k=1, rt0=5.0, rt1=5.5, level=1, bgk=0, bgrt0=5.0, bgrt1=5.5, bgf=1)
    assert min(c["y"]) >= 0                                                              # default of the older callers: clipped at zero


def test_mrm_difference_by_transition(tmp_path):
    ds.mrm(tmp_path / "a.mzML", 1.0e4 * 2.4, np.random.default_rng(1), 3)
    ds.mrm(tmp_path / "b.mzML", 1.0e4 * 12, np.random.default_rng(2), 4)
    a = App(tmp_path)
    a.open_session({"samples": [{"file": "a.mzML"}, {"file": "b.mzML"}]})
    code, j = get(a, "/api/mrm", k="1", ref=0, rttol=0.1)
    assert code == 200 and j["ref"] == 0
    t = j["files"][0]["transitions"][0]
    assert max(t["y"]) > 0 and len(t["rt"]) == len(t["y"])
