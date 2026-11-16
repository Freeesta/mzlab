"""`mzlab-cli peak-auto-lr` (mzlab_core::peakq_lrauto, «Integrate every peak shown» on low-resolution traces) on the jagged synthetic SIM/MRM traces of
tools/synthetic_data_jagged.py: every true peak found once, touching peaks split at the valley of the noise-free trace, baseline >= 0 and never above the
trace, and each area equal to an independent trapezoid of the raw trace above the reported line (written here, not taken from the engine).
"""
import json
import sys

import numpy as np
import pytest

from cargo_util import ROOT, cli

sys.path.insert(0, str(ROOT / "tools"))
from synthetic_data_jagged import CASES, CYCLE, trace, write_csv  # noqa: E402

SEEDS = range(1, 11)


def _trapezoid(t, y, lo, hi, line):
    """Signed area (counts x min) of y minus the straight line [t0, y0, t1, y1] over the points lo..hi."""
    t0, y0, t1, y1 = line
    base = y0 + (y1 - y0) * (t[lo:hi + 1] - t0) / (t1 - t0)
    d = y[lo:hi + 1] - base
    return float(np.sum((d[1:] + d[:-1]) / 2 * np.diff(t[lo:hi + 1])))


@pytest.fixture(scope="module")
def answers(tmp_path_factory):
    d = tmp_path_factory.mktemp("lr-auto")
    out = {}
    for name in CASES:
        for seed in SEEDS:
            t, y, truth = trace(name, seed)
            # the engine reads the same rounded numbers as the CSV
            t, y = np.round(t, 6), np.round(y, 3)
            r = json.loads(cli("peak-auto-lr", str(write_csv(d / f"{name}-{seed}.csv", t, y))).stdout)
            out[(name, seed)] = (t, y, truth, r)
    return out


def test_every_peak_once(answers):
    found = expected = 0
    for (name, seed), (t, y, truth, r) in answers.items():
        pk = r["peaks"]
        assert len(pk) == truth["expected"], (name, seed, [p["rt"] for p in pk])
        for a in truth["apex"]:
            assert sum(p["t0"] <= a <= p["t1"] for p in pk) == 1, (name, seed, a)
        found += len(pk); expected += truth["expected"]
    assert found == expected == 150


def test_touching_peaks_split_at_the_valley(answers):
    for (name, seed), (t, y, truth, r) in answers.items():
        pk = r["peaks"]
        for v in truth["valley"]:
            cut = [p["t1"] for p, q in zip(pk, pk[1:]) if p["t1"] == q["t0"] and p["rt"] < v < q["rt"]]
            assert len(cut) == 1, (name, seed, v)
            assert abs(cut[0] - v) <= 2.5 * CYCLE, (name, seed, v, cut)       # within two or three points of the clean valley
            assert [p["end1"] for p in pk if p["t1"] == cut[0]] == ["valley"]


def test_baseline_and_area_on_the_raw_trace(answers):
    for (name, seed), (t, y, truth, r) in answers.items():
        for p in r["peaks"]:
            lo, hi = p["lo"], p["hi"]
            assert t[lo] == p["t0"] and t[hi] == p["t1"]
            assert 0 <= p["b0"] <= y[lo] and 0 <= p["b1"] <= y[hi], (name, seed, p)
            area = _trapezoid(t, y, lo, hi, [p["t0"], p["b0"], p["t1"], p["b1"]])
            assert area == pytest.approx(p["area"], rel=1e-9, abs=1e-9), (name, seed, p)
            assert _trapezoid(t, y, lo, hi, [p["t0"], 0, p["t1"], 0]) == pytest.approx(p["area_zero"], rel=1e-9)
            assert p["sn"] == pytest.approx(p["height"] / r["params"]["noise"], rel=1e-9)


def test_areas_near_the_truth(answers):
    """Areas of the isolated and the resolved peaks against the noise-free ones: the median within 5 %, none beyond 25 %."""
    err = []
    for (name, seed), (t, y, truth, r) in answers.items():
        if name in ("noise", "low_sn"):
            continue
        for a, true in zip(truth["apex"], truth["area"]):
            p = next(p for p in r["peaks"] if p["t0"] <= a <= p["t1"])
            err.append(abs(p["area"] - true) / true)
    assert np.median(err) < 0.05 and max(err) < 0.25, (np.median(err), max(err))


def test_parameters_are_reported(answers):
    t, y, truth, r = answers[("singles", 1)]
    q = r["params"]
    assert q["half_auto"] is True and 2 <= q["half"] <= 30 and q["valley"] == 0.1 and q["sn"] == 5 and q["split"] == "drop" and q["noise"] > 0
