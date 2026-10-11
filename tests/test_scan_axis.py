"""/api/scan_axis and the path of the sight of the map (mirino.js): the nearest scan found in the page on the axis, then /api/spectra by position,
gives exactly what /api/nearest_scan + /api/spectrum gave (same scan, same RT, same m/z and intensities), in low and high resolution."""
import json
import shutil
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote

import numpy as np
import pytest

from mzlab.app import App
from mzlab.api import dispatch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds  # noqa: E402

JS = ROOT / "mzlab" / "web" / "mirino.js"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def call(app, url):
    path, _, qs = url.partition("?")
    q = {k: unquote(v) for k, v in (x.split("=", 1) for x in qs.split("&") if x)}
    st, ct, body, _ = dispatch(app, "GET", path, q)
    return json.loads(body)


def nearest_js(rts, x):
    code = f"const o=require({str(JS)!r});console.log(JSON.stringify({json.dumps(list(x))}.map(x=>o.nearest({json.dumps(rts)},x))))"
    return json.loads(subprocess.run(["node", "-e", code], check=True, capture_output=True, encoding="utf-8").stdout)


@pytest.fixture(scope="module")
def apps(tmp_path_factory):
    d = tmp_path_factory.mktemp("axis")
    ds.full_scan(d / "lr-t0.mzML", 0, np.random.default_rng(3))
    ds.hr_dda(d / "hr-t30.mzML", np.random.default_rng(5), "exploris")
    lr, hr = App(d), App(d)
    lr.open_session({"samples": [{"file": "lr-t0.mzML"}]}); hr.open_session({"samples": [{"file": "hr-t30.mzML"}]})
    return {"lr": lr, "hr": hr}


@needs_node
@pytest.mark.parametrize("which,level", [("lr", 1), ("hr", 1), ("hr", 2)])
def test_axis_matches_nearest_scan_and_spectrum(apps, which, level):
    apps, k = apps[which], 0
    ax = call(apps, f"/api/scan_axis?k={k}&level={level}")
    assert len(ax["rt"]) == len(ax["sid"]) > 5 and ax["rt"] == sorted(ax["rt"])
    rng = np.random.default_rng(7)
    xs = [float(v) for v in rng.uniform(ax["rt"][0] - 0.3, ax["rt"][-1] + 0.3, 40)] + [ax["rt"][3], (ax["rt"][3] + ax["rt"][4]) / 2]      # inside, outside, on a scan, between two
    for x, i in zip(xs, nearest_js(ax["rt"], xs)):
        n = call(apps, f"/api/nearest_scan?k={k}&rt={x!r}&level={level}")
        assert ax["sid"][i] == n["sid"] and ax["rt"][i] == n["rt"], (x, i, n)
        old = call(apps, f"/api/spectrum?k={k}&rt0={n['rt'] - 1e-4}&rt1={n['rt'] + 1e-4}&level={level}&bin=0.1")
        new = call(apps, f"/api/spectra?k={k}&i0={i}&i1={i}&level={level}&bin=0.1")["scans"][0]
        assert new["sid"] == n["sid"] and new["mz"] == old["mz"] and new["y"] == old["y"] and new["mode"] == old["mode"], (x, i)


def test_axis_errors_are_json(apps):
    st, ct, body, _ = dispatch(apps["lr"], "GET", "/api/scan_axis", {})
    assert json.loads(body).get("error")
