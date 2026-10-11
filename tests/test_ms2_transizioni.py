"""Precursor > fragment pairs of the MS2 scans (/api/ms2tr): numbers only, per precursor, usable as a chromatogram window."""
import json
import shutil
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import dati_sintetici as ds  # noqa: E402
from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402


@pytest.fixture()
def ms2_app(tmp_path):
    p = tmp_path / "B_MS2-t15.mzML"
    ds.ms2(p, __import__("numpy").random.default_rng(1), 15)
    app = App(); app.workdir = tmp_path
    app.open_session({"samples": [{"file": p.name, "time": 15, "type": "sample"}]})
    return app


def _get(app, path, **q):
    code, _, body, _ = dispatch(app, "GET", path, {k: str(v) for k, v in q.items()})
    return code, json.loads(body)


def test_transitions_per_precursor(ms2_app):
    code, d = _get(ms2_app, "/api/ms2tr", k=0)
    assert code == 200
    tr = d["files"][0]["transitions"]
    assert {t["q1"] for t in tr} == {229.1, 305.0}
    assert all(set(t) == {"q1", "q3", "y"} and t["y"] > 0 for t in tr)
    assert len([t for t in tr if t["q1"] == 229.1]) <= 5


def test_transition_window_gives_a_chromatogram(ms2_app):
    t = _get(ms2_app, "/api/ms2tr", k=0)[1]["files"][0]["transitions"][0]
    _, c = _get(ms2_app, "/api/chrom", k=0, kind="tic", level=2, mz0=t["q3"] - 0.5, mz1=t["q3"] + 0.5, prec=t["q1"])
    _, whole = _get(ms2_app, "/api/chrom", k=0, kind="tic", level=2, prec=t["q1"])
    assert len(c["rt"]) == len(whole["rt"]) and max(c["y"]) > 0 and max(c["y"]) <= max(whole["y"])
