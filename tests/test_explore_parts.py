"""Mixed files (block I): one file with several experiments is split in parts that share one Run; a notebook name 'x.mzML#MS2' opens only that part."""
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import numpy as np  # noqa: E402

import dati_sintetici as ds  # noqa: E402
from mzlab.explore import Session, file_parts, mixed_text  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


@pytest.fixture(scope="module")
def d(tmp_path_factory):
    p = tmp_path_factory.mktemp("mixed")
    ds.mixed_ida(p / "ida.mzML", np.random.default_rng(1))
    ds.mixed_mrm_epi(p / "epi.mzML", np.random.default_rng(2))
    ds.mixed_polarity(p / "pol.mzML", np.random.default_rng(3))
    ds.full_scan(p / "plain.mzML", 0, np.random.default_rng(4))
    ds.hr_dda(p / "e.mzML", np.random.default_rng(5), "exploris")
    ds.full_scan(p / "q.mzML", 0, np.random.default_rng(6), "3200 QTRAP")
    return p


def test_which_files_are_mixed(d):
    assert file_parts(Run(d / "plain.mzML")) == [] and mixed_text(Run(d / "plain.mzML")) == []
    assert [p["tag"] for p in file_parts(Run(d / "ida.mzML"))] == ["MS1", "MS2"]
    assert [p["tag"] for p in file_parts(Run(d / "epi.mzML"))] == ["MS2", "MRM"]
    assert [p["tag"] for p in file_parts(Run(d / "pol.mzML"))] == ["MS1 pos", "MS1 neg"]


def test_parts_are_separate_items(d):
    s = Session([{"file": "ida.mzML"}, {"file": "pol.mzML"}, {"file": "plain.mzML"}], d)
    info = {i["file"]: i for i in s.info()}
    assert set(info) == {"ida.mzML#MS1", "ida.mzML#MS2", "pol.mzML#MS1 pos", "pol.mzML#MS1 neg", "plain.mzML"}
    assert info["ida.mzML#MS1"]["ms1"] == 160 and info["ida.mzML#MS1"]["ms2"] == 0 and info["ida.mzML#MS1"]["kind"] == "full"
    assert info["ida.mzML#MS2"]["ms2"] == 320 and info["ida.mzML#MS2"]["kind"] == "ms2" and len(info["ida.mzML#MS2"]["ms2_events"]) == 320
    assert info["pol.mzML#MS1 pos"]["polarity"] == "positive" and info["pol.mzML#MS1 neg"]["polarity"] == "negative"
    assert info["pol.mzML#MS1 pos"]["ms1"] == 200
    it = s.items[0]
    it2 = s.items[1]
    assert it.run is it2.run                                          # one Run per file, not one per part


def test_tic_of_a_polarity_part_is_not_summed(d):
    s = Session([{"file": "pol.mzML"}], d)
    pos, neg = s.items
    rt_p, y_p = pos.total("tic", 1); rt_n, y_n = neg.total("tic", 1)
    assert len(rt_p) == 200 and len(rt_n) == 200 and not np.allclose(y_p, y_n)


def test_notebook_name_opens_one_part(d):
    s = Session([{"file": "ida.mzML#MS2"}], d)
    assert [i.file for i in s.items] == ["ida.mzML#MS2"]
    assert s.items[0].label.endswith("DDA") and not s.items[0].label.endswith("DDA · DDA")


def test_dda_precursors_are_grouped(d):
    s = Session([{"file": "ida.mzML"}], d)
    ex = s.items[1].info()["ms2_exps"]
    assert 5 <= len(ex) <= 12 and sum(e["n"] for e in ex) == 320


def test_mrm_part_has_the_transitions(d):
    s = Session([{"file": "epi.mzML"}], d)
    mrm = [i for i in s.items if i.kind() == "mrm"][0]
    assert mrm.info()["srm"] == 2 and mrm.info()["scans"] == 0


def test_hr_lr_mix_raises_error(d):
    with pytest.raises(ValueError) as e:
        Session([{"file": "q.mzML"}, {"file": "e.mzML"}], d)
    assert e.value.key == "err.hr.mix"

