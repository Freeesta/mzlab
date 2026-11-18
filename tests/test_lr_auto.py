"""`mzlab-cli peak-auto-lr` (mzlab_core::peakq_lrauto, «Integrate every peak shown» on low-resolution traces) on the jagged synthetic
SIM/MRM traces of tools/synthetic_data_jagged.py: every true peak found once, touching peaks split at the valley of the noise-free trace, the
baseline drawn between the two ends only (never extrapolated, so never raised to zero) and never above the trace, and each area equal to an
independent trapezoid of the raw trace above the reported line (written here, not taken from the engine): only the part above the line counts,
the part under it is `below`, so no area is negative, also on a trace below zero.
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
    """(above, under) (counts x min) of y against the straight line [t0, y0, t1, y1] over the points lo..hi: trapezoids, a segment that crosses the
    line split where it crosses it."""
    t0, y0, t1, y1 = line
    tt = t[lo:hi + 1]
    d = y[lo:hi + 1] - (y0 + (y1 - y0) * (tt - t0) / (t1 - t0))
    up = down = 0.0
    for a, b, w in zip(d[:-1], d[1:], np.diff(tt)):
        if a >= 0 and b >= 0:
            up += (a + b) / 2 * w
        elif a <= 0 and b <= 0:
            down -= (a + b) / 2 * w
        else:
            x = w * abs(a) / (abs(a) + abs(b))      # where the segment crosses the line
            pos, neg = (a, b) if a > 0 else (b, a)
            up += pos * (x if a > 0 else w - x) / 2
            down -= neg * (w - x if a > 0 else x) / 2
    return up, down


@pytest.fixture(scope="module")
def answers(tmp_path_factory):
    d = tmp_path_factory.mktemp("lr-auto")
    out = {}
    for name in CASES:
        for seed in SEEDS:
            t, y, truth = trace(name, seed)
            # the engine reads the same rounded numbers as the CSV
            t, y = np.round(t, 6), np.round(y, 3)
            r = json.loads(cli("peak-auto-lr", str(write_csv(d / f"{name}-{seed}.csv", t, y)), "--set", "area_unit=min").stdout)
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
            for b, i, c in ((p["b0"], lo, p["clamp0"]), (p["b1"], hi, p["clamp1"])):
                assert b <= y[i] and c in ("none", "trace") and (c == "none" or b == y[i]), (name, seed, p)   # as found, or lowered onto the trace
            up, down = _trapezoid(t, y, lo, hi, [p["t0"], p["b0"], p["t1"], p["b1"]])
            assert up == pytest.approx(p["area"], rel=1e-9, abs=1e-9) and down == pytest.approx(p["below"], rel=1e-9, abs=1e-9), (name, seed, p)
            assert _trapezoid(t, y, lo, hi, [p["t0"], 0, p["t1"], 0])[0] == pytest.approx(p["area_zero"], rel=1e-9)
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


def test_a_trace_below_zero_is_never_a_negative_area(tmp_path):
    """A blank subtracted too much (the whole trace 600 counts lower) and one over-subtracted point in a flank: the baseline ends stay on the
    background below zero (never raised to zero: the line is not extrapolated) and never above the trace, the dip under the line is
    `below`, every area >= 0."""
    cut = 0
    for seed in SEEDS:
        t, y, truth = trace("singles", seed)
        y = y - 600.0
        a = int(np.argmin(np.abs(t - truth["apex"][0])))
        y[a - 4] = -1000.0
        t, y = np.round(t, 6), np.round(y, 3)
        r = json.loads(cli("peak-auto-lr", str(write_csv(tmp_path / f"neg-{seed}.csv", t, y)), "--set", "area_unit=min").stdout)
        assert r["params"]["baseline"] == "engine"
        for p in r["peaks"]:
            lo, hi = p["lo"], p["hi"]
            for b, i, c in ((p["b0"], lo, p["clamp0"]), (p["b1"], hi, p["clamp1"])):
                assert b < 0 and b <= y[i] and c in ("none", "trace") and (c == "none" or b == y[i]), (seed, p)
            up, down = _trapezoid(t, y, lo, hi, [p["t0"], p["b0"], p["t1"], p["b1"]])
            assert p["area"] >= 0 and p["below"] >= 0 and p["area_zero"] >= 0, (seed, p)
            assert up == pytest.approx(p["area"], rel=1e-9, abs=1e-9) and down == pytest.approx(p["below"], rel=1e-9, abs=1e-9), (seed, p)
            cut += lo < a - 4 < hi and p["below"] > 0
    assert cut >= len(SEEDS) // 2, cut


def test_areas_are_counts_seconds_by_default_and_minutes_on_request(tmp_path):
    """The default unit is counts x s (as peak-area); area_unit=min gives the old counts x min: the same peaks, every area 60 times larger."""
    t, y, _ = trace("singles", 1)
    f = str(write_csv(tmp_path / "unit.csv", np.round(t, 6), np.round(y, 3)))
    s = json.loads(cli("peak-auto-lr", f).stdout)
    m = json.loads(cli("peak-auto-lr", f, "--set", "area_unit=min").stdout)
    assert s["peaks"] and len(s["peaks"]) == len(m["peaks"])
    for a, b in zip(s["peaks"], m["peaks"]):
        for k in ("area", "below", "area_zero"):
            assert a[k] == pytest.approx(60 * b[k], rel=1e-12, abs=1e-12), (k, a, b)
        assert {k: v for k, v in a.items() if k not in ("area", "below", "area_zero")} == {k: v for k, v in b.items() if k not in ("area", "below", "area_zero")}
