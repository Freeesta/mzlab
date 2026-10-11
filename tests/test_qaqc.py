"""QA/QC measures on synthetic series: known RT drift, known CV of the areas, known error in ppm, known sample / blank ratio."""
import math
import sys
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import dati_sintetici as ds  # noqa: E402

from mzlab import qaqc  # noqa: E402
from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402

MZ, RT, SIG = 305.1000, 10.0, 0.05
AMPS = [0.0, 1000.0, 1000.0, 1100.0, 900.0]            # the blank has no standard
ROLES = ["blank", "sample", "qc", "qc", "qc"]
PPM = [0, 1.0, 2.0, -1.0, 3.0]                           # error of each file
STEP = 0.05                                              # RT shift per file


def _file(path, c, amp, ppm):
    rt = np.arange(8.0, 12.0, 0.005)
    specs = []
    for i, t in enumerate(rt):
        y = amp * math.exp(-0.5 * ((t - c) / SIG) ** 2)
        mz = [250.0, MZ * (1 + ppm * 1e-6)] if y > 0 else [250.0]
        it = [10.0, y] if y > 0 else [10.0]
        specs.append(ds._spec(i, float(t), mz, it))
    ds._write(path, specs, [])


@pytest.fixture(scope="module")
def app(tmp_path_factory):
    d = tmp_path_factory.mktemp("qaqc")
    for i in range(5):
        _file(d / f"f{i}.mzML", RT + STEP * i, AMPS[i], PPM[i])
    a = App(d)
    a.open_session({"samples": [{"file": f"f{i}.mzML"} for i in range(5)]})
    return a


def test_measures_known_values(app):
    r = app.qaqc_ion([0, 1, 2, 3, 4], ROLES, MZ, RT, 1.0, 10, "ppm", ratio=True, thr=3)
    rows = r["rows"]
    assert rows[0] is None                                           # the blank has no standard
    area = AMPS[1] * SIG * math.sqrt(2 * math.pi)
    assert rows[1]["area"] == pytest.approx(area, rel=0.02)
    assert rows[2]["rt"] - rows[2]["drt"] == pytest.approx(RT)
    assert r["rt_drift_per_file"] == pytest.approx(STEP, abs=0.005)  # known drift
    for i in (1, 2, 3, 4):
        assert rows[i]["dppm"] == pytest.approx(PPM[i], abs=0.3)
    ma = r["mass_accuracy"]
    assert ma["mean"] == pytest.approx(np.mean(PPM[1:]), abs=0.3) and ma["max_abs"] == pytest.approx(3.0, abs=0.3)
    assert r["cv_area_qc"] == pytest.approx(qaqc.cv_percent([1000, 1100, 900]), rel=0.03)    # CV of the QC replicates
    assert rows[3]["rel"] == pytest.approx(1100 / 1000, rel=0.03)    # relative to the median of the non-blank files
    assert r["ratio"]["rows"][1]["absent_in_blank"] is True and r["ratio"]["rows"][1]["at_least"] is True


def test_api_and_lr_window(app):
    code, _, body, _ = dispatch(app, "GET", "/api/qaqc", {"k": "0,1,2", "roles": "blank,sample,qc", "mz": "305.3", "rt": "10", "win": "1", "tol": "0.5", "unit": "da"})
    assert code == 200
    import json
    d = json.loads(body)
    assert d["rows"][1]["dppm"] is None and d["rows"][1]["area"] > 0     # in Da no error in ppm
    assert dispatch(app, "GET", "/api/qaqc", {"k": "0,1", "roles": "blank", "mz": "305", "rt": "10"})[0] == 400


def test_pure_functions():
    assert qaqc.ppm_error(305.1003, 305.1) == pytest.approx(0.983, abs=0.001)
    assert qaqc.cv_percent([100, 100, 100]) == 0 and qaqc.cv_percent([1]) is None
    assert qaqc.slope([1, 2, 3, None, 5]) == pytest.approx(1.0)
    q = qaqc.sample_blank_ratios([10, 100, 20, None], ["blank", "sample", "blank", "sample"], 10)
    assert q["blank_area"] == 15 and q["rows"][1]["ratio"] == pytest.approx(100 / 15) and q["rows"][1]["at_least"] is False
    assert q["rows"][3] is None
