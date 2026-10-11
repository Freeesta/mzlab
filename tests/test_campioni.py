"""Sample metadata (mzlab/web/campioni.js, pure functions run by node) and their change in an open session (Session.set_meta)."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import dati_sintetici as ds  # noqa: E402

from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
needs_node = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

LOADER = """
const vm=require('vm'),fs=require('fs');
const ctx={module:{exports:{}},console};vm.createContext(ctx);
vm.runInContext(fs.readFileSync(%r,'utf8')+';this.O=module.exports',ctx);
vm.runInContext(fs.readFileSync(%r,'utf8')+';this.C=module.exports',ctx);
const C=ctx.C;
"""


def js(code):
    src = LOADER % (str(WEB / "ordine.js"), str(WEB / "campioni.js")) + code
    out = subprocess.run(["node", "-e", src], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


@needs_node
def test_tsv_roundtrip_in_mzmine_columns():
    rows = [{"filename": "a_t0.mzML", "type": "sample", "time": 0, "conc": None, "cunit": "", "rep": "R1"},
            {"filename": "blank.mzML", "type": "blank", "time": None, "conc": 2.5, "cunit": "µg/L", "rep": ""}]
    tsv = js(f"console.log(JSON.stringify(C.toTsv({json.dumps(rows)},'h')))")
    lines = tsv.rstrip("\n").split("\n")
    assert lines[0] == "filename\tsample_type\ttime\ttime_unit\tconcentration\tconcentration_unit\treplicate"
    assert lines[1].split("\t") == ["a_t0.mzML", "sample", "0", "h", "", "", "R1"]
    assert lines[2].split("\t") == ["blank.mzML", "blank", "", "", "2.5", "µg/L", ""]
    p = js(f"console.log(JSON.stringify(C.parseTsv({json.dumps(tsv)})))")
    assert p["warnings"] == []
    assert p["rows"][0]["time"] == 0 and p["rows"][0]["type"] == "sample" and p["rows"][0]["rep"] == "R1"
    assert p["rows"][1]["conc"] == 2.5 and p["rows"][1]["cunit"] == "µg/L" and p["rows"][1]["time"] is None


@needs_node
def test_parse_converts_units_and_reports_unknowns():
    tsv = "Filename;Sample_Type;Time;Time_Unit\nx.mzML;QC;1,5;h\ny.mzML;weird;2;min\nz.mzML;sample;1;fortnight\n"
    p = js(f"console.log(JSON.stringify(C.parseTsv({json.dumps(tsv)})))")
    r = p["rows"]
    assert r[0]["type"] == "qc" and r[0]["time"] == 90                     # 1.5 h in minutes
    assert r[1]["type"] is None and r[1]["time"] == 2                       # unknown type: kept as is, with a warning
    assert r[2]["time"] is None
    assert len(p["warnings"]) == 2
    assert js("console.log(JSON.stringify(C.parseTsv('a\\tb\\n1\\t2')))")["rows"] == []        # no filename column


@needs_node
def test_replicate_groups():
    fs = [{"file": "a", "time": 0}, {"file": "b", "time": 0}, {"file": "c", "time": 30}, {"file": "d", "time": None},
          {"file": "e", "time": None}, {"file": "f", "time": 30, "rep": "X"}, {"file": "g", "time": 0, "rep": "X"}]
    g = js(f"console.log(JSON.stringify(C.groups({json.dumps(fs)},f=>'sample').map(x=>x.files.map(f=>f.file))))")
    assert g == [["a", "b"], ["c"], ["d"], ["e"], ["f", "g"]]                 # a file without time or concentration is alone; a typed group wins
    assert js("console.log(JSON.stringify(C.groups([{file:'a',time:0},{file:'b',time:0}],f=>f.file==='a'?'blank':'sample').length))") == 2   # the type separates


@needs_node
def test_time_fraction_and_names():
    assert js("console.log(JSON.stringify([C.timeFrac(30,[0,30,60]),C.timeFrac(0,[0,30,60]),C.timeFrac(5,[5]),C.timeFrac(null,[1,2])]))") == [0.5, 0, 0.5, None]
    assert js("console.log(JSON.stringify([C.baseName('D:\\\\x\\\\Run_1.mzML'),C.baseName('Run_1.MZML#MS2')]))") == ["run_1", "run_1"]


@pytest.fixture()
def app(tmp_path):
    for i, t in enumerate((0, 30)):
        ds.full_scan(tmp_path / f"S_t{t}.mzML", t, np.random.default_rng(i))
    a = App(tmp_path)
    a.open_session({"samples": [{"file": "S_t0.mzML"}, {"file": "S_t30.mzML", "rep": "G"}]})
    return a


def test_set_meta_changes_the_open_files(app):
    inf = {x["file"]: x for x in app.session.info()}
    assert inf["S_t0.mzML"]["time"] == 0 and inf["S_t0.mzML"]["rep"] is None and inf["S_t30.mzML"]["rep"] == "G"
    r = app.set_meta(
        {"files": [{"file": "S_t0.mzML", "type": "blank", "time": "", "conc": 5, "cunit": "mg/L", "rep": " B "}, {"file": "nofile.mzML", "type": "blank"}]})
    assert r == {"changed": 1}
    it = app.session.items[0]
    assert (it.type, it.time, it.conc, it.cunit, it.rep) == ("blank", None, 5.0, "mg/L", "B")
    app.set_meta({"files": [{"file": "S_t0.mzML", "type": "qc", "rep": ""}]})          # qc / dark are roles of the page: the server keeps the base type
    assert app.session.items[0].type == "blank" and app.session.items[0].rep is None
    assert app.session.items[1].time == 30 and app.session.items[1].rep == "G"


def test_meta_route(app):
    import io
    body = json.dumps({"files": [{"file": "S_t30.mzML", "time": 45}]}).encode()
    code, _, out, _ = dispatch(app, "POST", "/api/meta", {}, io.BytesIO(body), len(body))
    assert code == 200 and json.loads(out) == {"changed": 1} and app.session.items[1].time == 45.0


def test_set_meta_without_session_is_an_error(tmp_path):
    from mzlab.i18n import UserError
    with pytest.raises(UserError):
        App(tmp_path).set_meta({"files": []})
