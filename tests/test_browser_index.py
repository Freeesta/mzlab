"""The index in the browser (mzlab/browser.py): built right after an upload in a temporary folder, handed to the worker, which puts it in OPFS and mounts it;
the open files switch to the mounted copy and the temporary one goes."""
import shutil
import sys
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import dati_sintetici as ds  # noqa: E402
from mzlab import browser  # noqa: E402
from mzlab.app import App  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402


@pytest.fixture
def site(tmp_path, monkeypatch):
    work, tmpidx = tmp_path / "work", tmp_path / "idxtmp"
    work.mkdir()
    monkeypatch.setenv("MZLAB_CACHE", str(tmpidx))
    monkeypatch.setattr(mzidx, "FLAT", True)
    monkeypatch.setattr(mzidx, "REGISTRY", {})
    monkeypatch.setattr(browser, "_tried", set())
    monkeypatch.setattr(browser, "WORK", work)
    ds.full_scan(work / "a.mzML", 0, np.random.default_rng(1))
    monkeypatch.setattr(browser, "app", App(work))
    return work, tmpidx, tmp_path


def test_build_hand_over_adopt(site):
    work, tmpidx, root = site
    assert browser.new_indexes() == []
    assert browser.build_index("a.mzML") is True
    assert browser.new_indexes() == [["a.mzML", str(tmpidx / "a.mzML.mzidx")]]
    assert browser.new_indexes() == []                                     # handed over once
    browser.app.open_session({"samples": [{"file": "a.mzML", "time": 0, "type": "sample"}]})
    run = browser.app.session.items[0].run
    before = np.asarray(run.table(1).mz).copy()
    mounted = root / "mounted" / "data"                                    # what the worker does: copy to OPFS, mount, tell Python
    mounted.parent.mkdir()
    shutil.copy(tmpidx / "a.mzML.mzidx", mounted)
    assert browser.adopt_index("a.mzML", str(mounted)) is True
    assert not (tmpidx / "a.mzML.mzidx").exists() and mzidx.REGISTRY["a.mzML"] == str(mounted)
    assert Path(run.index_path) == mounted and np.array_equal(np.asarray(run.table(1).mz), before)
    # a visit later: the registry points the new Run at the mounted index, and nothing is built
    browser.app.session.items[0].run.close()
    browser.app.session = None
    browser.app.open_session({"samples": [{"file": "a.mzML", "time": 0, "type": "sample"}]})
    run2 = browser.app.session.items[0].run
    assert run2._idx and Path(run2.index_path) == mounted and "costruzione .mzidx" not in run2.timing
    assert mzidx.known_sha(work / "a.mzML") == run2.sha256 != ""
    browser.forget_index("a.mzML")
    assert "a.mzML" not in mzidx.REGISTRY


def test_a_stale_registered_index_is_replaced_not_written_over(site):
    work, tmpidx, root = site
    browser.build_index("a.mzML")
    mounted = root / "m" / "data"
    mounted.parent.mkdir()
    shutil.copy(tmpidx / "a.mzML.mzidx", mounted)
    (tmpidx / "a.mzML.mzidx").unlink()
    mzidx.REGISTRY["a.mzML"] = str(mounted)
    ds.full_scan(work / "a.mzML", 15, np.random.default_rng(7))            # the file is replaced
    browser.app.open_session({"samples": [{"file": "a.mzML", "time": 0, "type": "sample"}]})
    run = browser.app.session.items[0].run
    assert np.array_equal(np.asarray(run.table(1).mz), run.table_reference(1).mz)
    assert "a.mzML" not in mzidx.REGISTRY and Path(run.index_path) == tmpidx / "a.mzML.mzidx"
    assert browser.new_indexes() == [["a.mzML", str(tmpidx / "a.mzML.mzidx")]]
