"""A session can grow one file at a time (api/explore with append: the page opens the first file at once and the others as they are ready):
the result is the same as opening the whole list in one call, and a file that cannot be added leaves the session as it was."""
import sys
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import dati_sintetici as ds  # noqa: E402
from mzlab.app import App  # noqa: E402
from mzlab.i18n import UserError  # noqa: E402


@pytest.fixture()
def app(tmp_path):
    for i, t in enumerate((0, 5, 10)):
        ds.full_scan(tmp_path / f"FullScan_t{t}.mzML", t, np.random.default_rng(i + 1))
    ds.hr_dda(tmp_path / "HRMS_t000.mzML", np.random.default_rng(7))
    return App(tmp_path)


def sample(name):
    return {"file": name, "time": None, "type": "sample", "conc": None, "cunit": None, "label": None}


def test_one_by_one_equals_all_at_once(app):
    names = [f"FullScan_t{t}.mzML" for t in (0, 5, 10)]
    app.open_session({"samples": [sample(n) for n in names]})
    whole = app.session_state()["session"]
    app.open_session({"samples": [sample(names[0])]})
    for n in names[1:]:
        app.open_session({"samples": [sample(n)], "append": True})
        assert [x["file"] for x in app.session_state()["session"]][-1] == n
    assert app.session_state()["session"] == whole


def test_append_without_a_session_opens_one(app):
    app.open_session({"samples": [sample("FullScan_t0.mzML")], "append": True})
    assert [x["file"] for x in app.session_state()["session"]] == ["FullScan_t0.mzML"]


def test_a_file_that_does_not_fit_leaves_the_session_as_it_was(app):
    app.open_session({"samples": [sample("FullScan_t0.mzML")]})
    before = app.session_state()["session"]
    with pytest.raises(UserError) as e:
        app.open_session({"samples": [sample("HRMS_t000.mzML")], "append": True})
    assert e.value.key == "err.hr.mix"
    assert app.session_state()["session"] == before
    with pytest.raises(UserError):
        app.open_session({"samples": [sample("missing.mzML")], "append": True})
    app.open_session({"samples": [sample("FullScan_t5.mzML")], "append": True})
    assert len(app.session_state()["session"]) == 2
