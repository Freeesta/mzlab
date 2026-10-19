"""/api/spectra (single scans, scan-by-scan navigation) must agree with /api/spectrum on a window holding only that scan."""
import json
import os
import sys
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
from dati_percorsi import percorso  # noqa: E402
from mzlab import api  # noqa: E402
from mzlab.demo import make_demo  # noqa: E402
from mzlab.app import App  # noqa: E402


def _real_files():
    mzml = os.environ.get("MZLAB_MZML", os.environ.get("QQQ_MZML"))     # an explicit folder wins over the data repository
    for c in (mzml, percorso("dati/3200 qtrap/mzML")):
        if c and Path(c).is_dir():
            return sorted(Path(c).glob("B_*.mzML"))
    return []


def _get(app, path, **q):
    code, _, body, _ = api.dispatch(app, "GET", path, {k: str(v) for k, v in q.items()})
    return code, json.loads(body)


def _open(tmp_path, files):
    app = App(tmp_path / "w")
    (tmp_path / "w").mkdir(exist_ok=True)
    for f in files:
        (tmp_path / "w" / f.name).write_bytes(f.read_bytes())
    app.open_session({"samples": [{"file": f.name} for f in files]})
    return app


def _same(app, k, level=1, prec=None, i0=0, n=8):
    code, j = _get(app, "/api/spectra", k=k, i0=i0, i1=i0 + n - 1, level=level, prec=prec if prec is not None else "")
    assert code == 200
    item = app._item(k)
    ids, rt = item._scan_ids(level, prec, 0.6)
    assert j["n"] == len(ids) and len(j["scans"]) == min(n, len(ids) - i0)
    for s in j["scans"]:
        i = s["i"]
        lo = (rt[i - 1] + rt[i]) / 2 if i > 0 else rt[i] - 0.001
        hi = (rt[i] + rt[i + 1]) / 2 if i + 1 < len(rt) else rt[i] + 0.001
        # a window that holds exactly this scan: the same call the browser makes with api/spectrum
        _, ref = _get(app, "/api/spectrum", k=k, rt0=(lo + rt[i]) / 2 - 1e-6, rt1=(rt[i] + hi) / 2 + 1e-6, level=level, precursor=prec if prec is not None else "")
        if ref["scans"] != 1:                         # window not isolating one scan (duplicated RT): skip, nothing to compare
            continue
        assert s["mz"] == ref["mz"] and s["y"] == ref["y"], (k, i)


def test_spectra_equals_spectrum_on_demo(tmp_path):
    demo = make_demo(tmp_path / "d")
    files = sorted(Path(demo[1][0]).parent.glob("*.mzML")) if isinstance(demo, tuple) else sorted(Path(demo).glob("*.mzML"))
    app = _open(tmp_path, files[:2])
    _same(app, 0, i0=3, n=10)
    _same(app, 1, i0=0, n=5)


def test_spectra_limits_and_errors(tmp_path):
    demo = make_demo(tmp_path / "d")
    files = sorted(Path(demo[1][0]).parent.glob("*.mzML")) if isinstance(demo, tuple) else sorted(Path(demo).glob("*.mzML"))
    app = _open(tmp_path, files[:1])
    n = app._item(0).scan_count(1)
    assert _get(app, "/api/spectra", k=0, i0=5, i1=2)[0] == 400                    # i0 > i1
    assert _get(app, "/api/spectra", k=0, i0=0, i1=60)[0] == 400                   # 61 scans
    assert _get(app, "/api/spectra", k=0, i0=0, i1=59)[0] == 200                   # exactly 60
    assert _get(app, "/api/spectra", k=0, i0=n + 5, i1=n + 6)[0] == 400            # beyond the end
    code, j = _get(app, "/api/spectra", k=0, i0=n - 2, i1=n + 20)                   # i1 past the end is clamped
    assert code == 200 and [s["i"] for s in j["scans"]] == [n - 2, n - 1]
    assert "error" in _get(app, "/api/spectra", k=0, i0=0)[1]                      # missing parameter -> JSON error, 400
    assert _get(app, "/api/spectra", k=0, i0=0)[0] == 400
    assert _get(app, "/api/spectra", k=9, i0=0, i1=1)[0] == 400                    # no such file


@pytest.mark.skipif(not _real_files(), reason="real mzML files not available")
def test_spectra_equals_spectrum_on_real_files(tmp_path):
    real = _real_files()
    full = [f for f in real if "FullMass" in f.name][:1]
    ms2 = [f for f in real if "MS2" in f.name][:1]
    mrm = [f for f in real if f.name.startswith("B_MRM-t")][:1]
    app = _open(tmp_path, full + ms2 + mrm)
    _same(app, 0, i0=500, n=12)
    k_ms2 = 1
    prec = app._item(k_ms2).info()["precursors"][0]
    _same(app, k_ms2, level=2, prec=prec, i0=0, n=10)
    code, j = _get(app, "/api/spectra", k=2, i0=0, i1=3)                          # an MRM file has no scans: clear error
    assert code == 400 and "MRM" in j["error"]
