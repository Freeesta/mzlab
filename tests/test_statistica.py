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
