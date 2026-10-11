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
