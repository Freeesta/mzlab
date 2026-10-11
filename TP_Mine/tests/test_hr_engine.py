# TPMINE-PRIVATE
"""WP12 (public, synthetic): the high-resolution experiment from the files to the ranked list, the API switch and the outputs."""
import json

import pytest

import lc_synth as LS
import msn_synth as MS
from tpmine import api
from tpmine.hr import engine as EN

SMILES = "Cn1cnc2c1c(=O)n(C)c(=O)n2C"


@pytest.fixture(scope="module")
def series(tmp_path_factory):
    d = tmp_path_factory.mktemp("lc")
    files = LS.write_series(d)
    msn = MS.write_msn(d / "cafe_msn.mzML", MS.caffeine_nodes())
    return d, files, str(msn)


@pytest.fixture(scope="module")
def exp(series):
    d, files, msn = series
    e = EN.ExperimentHR(files + [{"name": "cafe_msn.mzML", "path": msn, "time": None, "type": "sample"}], {"smiles": SMILES})
    e.run()
    return e


def test_classify(series):
    from mzlab.reader.mzml import Run
    d, files, msn = series
    assert EN.classify(Run(files[1]["path"])) == "hr" and EN.classify(Run(msn)) == "msn"


def test_ranking_finds_the_products(exp):
    r = exp.result
    assert r["mode"] == "hr" and r["parent"]["ion"] == "C8H11N4O2" and abs(r["parent"]["mz"] - 195.0877) < 1e-3
    rows = {x["formula"]: x for x in r["rows"] if x["formula"]}
    assert "C8H11N4O3" in rows and "C7H9N4O2" in rows
    top = [x["formula"] for x in r["rows"][:6]]
    assert "C8H11N4O3" in top and "C7H9N4O2" in top
    # background ions and the parent itself are not among the candidates
    assert all(abs(x["mz"] - 195.0877) > 0.003 for x in r["rows"])
    assert all(x["mz"] not in (150.0583, 166.0863) for x in r["rows"])


def test_rows_have_the_requested_fields(exp):
    x = next(x for x in exp.result["rows"] if x["formula"] == "C8H11N4O3")
    assert x["level"] in (5, 4, 3) and x["class"] and x["tmax"] is not None and abs(x["ppm"]) < 3
    assert set(x["components"]) == {"ms2", "loc", "kin", "form", "int"} and x["delta"] == "+O"
    y = next(x for x in exp.result["rows"] if x["formula"] == "C7H9N4O2")
    assert y["tmax"] < x["tmax"] and "early" in y["class"]
    assert "persistent" in x["class"] or "late" in x["class"]


def test_detail_and_json(exp):
    cid = next(x["id"] for x in exp.result["rows"] if x["formula"] == "C8H11N4O3")
    d = exp.detail(cid)
    assert json.loads(json.dumps(d))["id"] == cid
    assert d["ms2"]["candidate"]["mz"] and d["ms2"]["parent"]["mz"] and len(d["areas"]) == len(d["times"])
    json.dumps(exp.result)


def test_tree_and_inclusion_list(exp):
    t = exp.msn_tree()
    assert t and t[0]["formula"] == "C8H11N4O2"
    csv = exp.inclusion_csv(5).splitlines()
    assert csv[0].startswith("m/z,charge") and len(csv) <= 6


def test_without_msn_file_there_is_no_localisation(series):
    d, files, _ = series
    e = EN.ExperimentHR(files, {"neutral": "C8H10N4O2"})
    r = e.run()
    assert any("MSn" in w for w in r["warnings"]) and r["tree"] is None
    assert all(x["region"] is None for x in r["rows"])


def test_needs_treated_samples(series):
    d, files, _ = series
    with pytest.raises(ValueError):
        EN.ExperimentHR([f for f in files if f["time"] <= 0], {"neutral": "C8H10N4O2"}).run()


def test_api_switches_to_hr(series, monkeypatch):
    d, files, msn = series
    monkeypatch.setattr(api, "DATA", d)
    names = json.dumps([f["name"] for f in files] + ["cafe_msn.mzML"])
    kinds = {x["name"]: x["hr_kind"] for x in json.loads(api.classify(names))}
    assert kinds["cafe_msn.mzML"] == "msn" and kinds[files[2]["name"]] == "hr"
    fl = [{"name": f["name"], "time": f["time"], "type": "sample"} for f in files] + [{"name": "cafe_msn.mzML", "time": None, "type": "sample"}]
    out = json.loads(api.run(json.dumps(fl), json.dumps({"smiles": SMILES}), "{}", ""))
    assert out["mode"] == "hr" and out["rows"]
    assert json.loads(api.detail(out["rows"][0]["id"]))["id"] == out["rows"][0]["id"]
    assert json.loads(api.msn_tree()) and api.inclusion_csv(3).startswith("m/z")


# ---------------------------------------------------------------------------------------------------------------------- rows hidden under an ISF
@pytest.fixture(scope="module")
def hidden(tmp_path_factory):
    d = tmp_path_factory.mktemp("lch")
    files = LS.write_series(d, hidden_isf=True)
    msn = MS.write_msn(d / "cafe_msn.mzML", MS.caffeine_nodes())
    e = EN.ExperimentHR(files + [{"name": "cafe_msn.mzML", "path": str(msn), "time": None, "type": "sample"}], {"smiles": SMILES})
    e.run()
    return d, files, str(msn), e


def test_hidden_isf_row_exists_and_is_flagged(hidden):
    e = hidden[3]
    rows = [x for x in e.result["rows"] if x["id"] < 0]
    assert len(rows) == 1 and abs(rows[0]["mz"] - LS.ISF_FRAGMENT) < 0.003
    x = rows[0]
    assert "possible TP co-eluting with an ISF" in x["flags"] and x["formula"] == "C6H8N3O" and x["series"] == [0] * len(e.lc)
    assert x["level"] in (5, 4) and x["tmax"] is None and x["region"] is None and x["ms2_scans"] == 0
    assert any(c["id"] == x["id"] for c in e.ranked)


def test_hidden_isf_row_detail_selection_and_json(hidden):
    e = hidden[3]
    cid = next(x["id"] for x in e.result["rows"] if x["id"] < 0)
    d = json.loads(json.dumps(e.detail(cid)))
    assert d["id"] == cid and d["row"]["id"] == cid and len(d["areas"]) == len(d["times"]) and d["criteria"] and "ms2" not in d and "localization" not in d
    assert "isf_hidden" not in json.dumps(e.result["inclusion"])
    json.dumps(e.result)
    assert all(x["mz"] != LS.ISF_FRAGMENT for x in e.result["inclusion"])                # no inclusion-list line for a mass with no peaks of its own
    assert e.inclusion_csv(5).count("\n") <= 5


def test_hidden_isf_row_through_the_api(hidden, monkeypatch):
    d, files, msn, e = hidden
    monkeypatch.setattr(api, "DATA", d)
    fl = [{"name": f["name"], "time": f["time"], "type": "sample"} for f in files] + [{"name": "cafe_msn.mzML", "time": None, "type": "sample"}]
    out = json.loads(api.run(json.dumps(fl), json.dumps({"smiles": SMILES}), "{}", ""))
    cid = next(x["id"] for x in out["rows"] if x["id"] < 0)
    assert json.loads(api.detail(cid))["id"] == cid
    assert api.inclusion_csv(5).startswith("m/z")


# ---------------------------------------------------------------------------------------------------------------------- gap filling
def test_a_weak_product_at_the_first_time_gets_its_area_filled(tmp_path):
    files = LS.write_series(tmp_path, weak=0.2)                       # the +O product at t = 5 is under the detection height (1.2e5 < 2e5)
    msn = MS.write_msn(tmp_path / "cafe_msn.mzML", MS.caffeine_nodes())
    files.append({"name": "cafe_msn.mzML", "path": str(msn), "time": None, "type": "sample"})
    plain = EN.ExperimentHR(files, {"smiles": SMILES}, {"fill_gaps": False})
    plain.run()
    filled = EN.ExperimentHR(files, {"smiles": SMILES})
    filled.run()
    a = next(x for x in plain.result["rows"] if x["formula"] == "C8H11N4O3")
    b = next(x for x in filled.result["rows"] if x["formula"] == "C8H11N4O3")
    k5 = [f["time"] for f in filled.result["files"]].index(5.0)
    assert a["series"][k5] == 0 and a["filled"] == []
    assert b["filled"] == [k5] and 3e5 < b["series"][k5] < 3e6            # peak height 1.2e5 x about 9 scans x 2.4 s = 1.1e6 counts x s
    assert [v for i, v in enumerate(b["series"]) if i != k5] == [v for i, v in enumerate(a["series"]) if i != k5]      # nothing else changes
    assert b["onset"] <= a["onset"] and b["onset"] <= 5.0
    # the reference files are never filled
    ref = [i for i, f in enumerate(filled.result["files"]) if f["time"] <= 0]
    assert all(b["series"][i] == a["series"][i] for i in ref)
    assert not any(set(x["filled"]) & set(ref) for x in filled.result["rows"])


def test_trace_area_needs_a_real_trace():
    import numpy as np
    from mzlab.reader.mzml import PeakTable
    from tpmine.hr import features as FT
    rt = np.arange(0, 2, 0.05)
    z = np.arange(len(rt))
    # 12 scans of a trace at 200.0 around 1.0 min (height 5e4, under MIN_HEIGHT), single noise peaks elsewhere
    mz = np.r_[np.full(12, 200.0), 150.0, 250.0]
    it = np.r_[np.full(12, 5e4), 3e4, 3e4]
    pos = np.r_[np.arange(14, 26), 3, 30]
    o = np.argsort(mz, kind="stable")
    t = PeakTable(rt=rt, scan_ids=z, mz=mz[o], inten=it[o], pos=pos[o])
    area, apex = FT.trace_area(t, 200.0, 0.9, 1.3)
    assert area == pytest.approx(12 * 5e4 * 3.0, rel=1e-6) and 0.7 <= apex <= 1.3
    assert FT.trace_area(t, 200.0, 0.0, 0.5)[0] == 0.0                  # apex outside the window
    assert FT.trace_area(t, 150.0, 0.0, 0.5)[0] == 0.0                  # one point is not a trace
    assert FT.trace_area(t, 200.002, 0.9, 1.3)[0] == 0.0                # 10 ppm away
