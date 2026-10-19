"""Scan filters (WP-H3a): /api/filters lists the scan types of a file with their counts, and `filt` restricts the chromatogram, the spectra and the
single scans to one type. Synthetic Orbitrap DDA file; the infusion MSn file of the private data repository (skipped without it) for the paths."""
import json, struct, sys
from pathlib import Path
import numpy as np
import pytest
from mzlab.app import App
from mzlab.api import dispatch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds  # noqa: E402
from dati_percorsi import percorso  # noqa: E402


def call(app, url):
    path, _, qs = url.partition("?")
    q = dict(x.split("=", 1) for x in qs.split("&") if x)
    from urllib.parse import unquote
    q = {k: unquote(v) for k, v in q.items()}
    st, ct, body, _ = dispatch(app, "GET", path, q, b"") if False else dispatch(app, "GET", path, q)
    return json.loads(body) if ct.startswith("application/json") else body


@pytest.fixture(scope="module")
def hr(tmp_path_factory):
    d = tmp_path_factory.mktemp("flt")
    ds.hr_dda(d / "e.mzML", np.random.default_rng(5), "exploris")
    a = App(d); a.open_session({"samples": [{"file": "e.mzML"}]})
    return a


def test_groups_and_counts(hr):
    g = call(hr, "/api/filters?k=0")["filters"]
    keys = [x["key"] for x in g]
    assert any("Full ms" in k and "ms2" not in k for k in keys), keys
    assert any("ms2 @hcd" in k for k in keys), keys
    assert sum(x["n"] for x in g) == len(hr._item(0).run.scans)
    assert [x["level"] for x in g] == sorted(x["level"] for x in g)            # the survey first
    assert all(x["rt0"] <= x["rt1"] for x in g)


def test_filter_restricts_chromatogram_and_scans(hr):
    g = call(hr, "/api/filters?k=0")["filters"]
    for x in g:
        lv = x["level"]
        c = call(hr, f"/api/chrom?k=0&kind=tic&level={lv}&filt={x['key'].replace(' ', '%20')}")
        assert len(c["rt"]) == x["n"], (x["key"], len(c["rt"]), x["n"])
    c0 = call(hr, "/api/chrom?k=0&kind=tic&level=1")
    k1 = [x for x in g if x["level"] == 1][0]
    assert len(c0["rt"]) == k1["n"]                                            # the only survey type is the whole level
    assert len(call(hr, "/api/chrom?k=0&kind=tic&level=2&filt=nessun%20filtro")["rt"]) == 0


def test_filter_on_spectra_and_nearest(hr):
    g = [x for x in call(hr, "/api/filters?k=0")["filters"] if x["level"] == 2][0]
    f = g["key"].replace(" ", "%20")
    b = call(hr, f"/api/scanbin?k=0&i0=0&i1=2&level=2&filt={f}")
    hl = struct.unpack_from("<I", b, 0)[0]
    head = json.loads(b[4:4 + hl]); assert head["n"] == g["n"]
    s = call(hr, f"/api/spectra?k=0&i0=0&i1=1&level=2&filt={f}")
    assert s["n"] == g["n"]
    n = call(hr, f"/api/nearest_scan?k=0&rt=12&level=2&filter={f}")
    assert n["filter"] and "ms2" in n["filter"]


def test_scan_key_forms():
    from mzlab.explore import scan_key
    from mzlab.reader.mzml import Scan
    mk = lambda **k: Scan(index=0, native="", start=0, end=0, level=k.pop("level"), rt=1.0, polarity=1, tic=0, precursor=None, collision_energy=None, filter=k.pop("filter"), **k)
    assert scan_key(mk(level=1, filter="FTMS + p ESI Full ms [100.0000-900.0000]")) == "FTMS + p ESI Full ms"
    assert scan_key(mk(level=2, filter="FTMS + c ESI d Full ms2 317.1639@hcd30.00 [50.0000-350.0000]", act="HCD")) == "FTMS + ms2 @hcd"
    assert scan_key(mk(level=4, filter="FTMS + p ESI Full ms4 317.0000@cid25.00 261.0000@cid30.00 244.0000@cid35.00 [50-400]", act="CID",
                       path=((317.0, "CID", 25.0), (261.0, "CID", 30.0), (244.0, "CID", 35.0)))) == "FTMS + ms4 317 > 261 > 244 @cid35"


def _msn():
    hrms = percorso("dati/HRMS")
    return next(iter(hrms.glob("*/*direct-infusion_MSn.mzML")), None) if hrms else None


@pytest.mark.skipif(_msn() is None, reason="infusion MSn file of the data repository not available")
def test_msn_paths(tmp_path):
    import shutil
    shutil.copy(_msn(), tmp_path / "m.mzML")
    a = App(tmp_path); a.open_session({"samples": [{"file": "m.mzML"}]})
    k = [i for i, f in enumerate(a.session.items) if f.part and f.part["mode"] == "ms2"][0]
    g = call(a, f"/api/filters?k={k}")["filters"]
    paths = [x for x in g if x["level"] >= 3]
    assert len(paths) >= 17 and max(x["level"] for x in paths) == 8
    top = [x for x in paths if x["key"].endswith("317 > 261 > 244 @cid35")]
    assert top and top[0]["level"] == 4
    c = call(a, f"/api/chrom?k={k}&kind=tic&level=4&filt={top[0]['key'].replace(' ', '%20').replace('>', '%3E')}")
    assert len(c["rt"]) == top[0]["n"]


# ---- information bar of the bench: header of a scan, list of scans, what the file says
def test_scan_list_matches_the_chromatogram(hr):
    for lv in (1, 2):
        t = call(hr, f"/api/scanlist?k=0&level={lv}")
        c = call(hr, f"/api/chrom?k=0&kind=tic&level={lv}")
        assert t["n"] == len(c["rt"]) == len(t["sid"]) == len(t["bpint"])
        assert all(a <= b for a, b in zip(t["rt"], t["rt"][1:]))
        assert abs(sum(t["tic"]) - sum(c["y"])) / max(sum(c["y"]), 1) < 1e-3
        assert min(t["bpint"]) > 0 and all(m > 0 for m in t["bpmz"])
    g = [x for x in call(hr, "/api/filters?k=0")["filters"] if x["level"] == 2][0]
    assert call(hr, "/api/scanlist?k=0&level=2&filt=" + g["key"].replace(" ", "%20"))["n"] == g["n"]


def test_scan_header_has_the_pairs_of_the_file(hr):
    sid = call(hr, "/api/scanlist?k=0&level=2")["sid"][0]
    h = call(hr, f"/api/scaninfo?k=0&sid={sid}")
    names = [p["name"] for p in h["pairs"]]
    assert "ms level" in names and h["level"] == 2 and h["filter"] and h["prec"] and h["iso"] and h["act"] == "HCD"
    assert call(hr, "/api/scaninfo?k=0&sid=99999999").get("error")


def test_file_info(hr):
    f = call(hr, "/api/fileinfo?k=0")
    assert f["instrument"] and f["scans"] == len(hr._item(0).run.scans) and f["levels"]["1"] > 0 and f["filters"]
    assert f["mz_range"]["1"][0] < f["mz_range"]["1"][1]


def test_method_gives_the_lower_limit_of_the_ms2_scan_window(hr):
    sp = call(hr, "/api/method?k=0").get("scan_params")
    if sp and sp.get("ms2"):
        lo = sp["ms2"]["window_lo"]
        assert lo is None or lo > 0
