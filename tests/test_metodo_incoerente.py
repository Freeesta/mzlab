"""A method (.dam) of another experiment type than the open files is reported (MRM against Full Scan, MS2 method with a Full Scan file...)."""
import shutil, sys
from pathlib import Path
import pytest
import numpy as np
from mzlab.app import App
from mzlab.reader.methodinfo import method_kind

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from dati_percorsi import percorso  # noqa: E402


def _dam_dir():
    return percorso("dati/3200 qtrap/dam")


def test_method_kind_from_the_experiments_and_the_name():
    assert method_kind({"name": "Lab_x_MRM_a.dam", "experiments": [{"kind": "mrm"}]}) == "mrm"
    assert method_kind({"name": "Lab_inq_FullMass_pos_max480.dam", "experiments": [{"kind": "scan"}]}) == "full"
    assert method_kind({"name": "Lab_inq_MS2_Carbamazepine.dam", "experiments": [{"kind": "scan"}]}) == "ms2"
    assert method_kind({"name": "mio.dam", "experiments": [{"kind": "scan"}]}) is None          # cannot be said: no warning


@pytest.fixture(scope="module")
def full_scan_app(tmp_path_factory):
    import sys
    sys.path.insert(0, str(ROOT / "tools"))
    import dati_sintetici as ds
    d = tmp_path_factory.mktemp("meth")
    (d / "wd").mkdir()
    ds.full_scan(d / "wd" / "B_FullMass-t0.mzML", 0, np.random.default_rng(1))
    a = App(d / "wd")
    a.open_session({"samples": [{"file": "B_FullMass-t0.mzML"}]})
    return a, d


@pytest.mark.skipif(_dam_dir() is None, reason="the .dam files of the lab (QQQ_DATI/dam) are not available")
def test_real_dam_files_warn_only_when_the_type_differs(full_scan_app):
    app, d = full_scan_app
    dam = _dam_dir()
    for n in ("Lab_inq_FullMass_pos_max480.dam", "Lab_inq_MRM_Carba.dam", "Lab_inq_MS2_Carbamazepine.dam"):
        shutil.copy(dam / n, app.workdir / n)
    w = {x["method"]: x for x in app.method_warnings()["warnings"]}
    assert "Lab_inq_FullMass_pos_max480.dam" not in w                       # same type as the open file
    assert w["Lab_inq_MRM_Carba.dam"]["expects"] == "MRM" and w["Lab_inq_MRM_Carba.dam"]["files"] == ["Full Scan"]
    assert w["Lab_inq_MS2_Carbamazepine.dam"]["expects"] == "MS2"


def test_no_session_no_warnings(tmp_path):
    (tmp_path / "w").mkdir()
    assert App(tmp_path / "w").method_warnings() == {"warnings": []}
