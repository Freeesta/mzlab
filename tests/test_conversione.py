"""Progress helpers of the .raw conversion (mzlab/web/conversione.js, run by node)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "conversione.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    out = subprocess.run(["node", "-e", f"const c=require({str(JS)!r});{code}"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_phase_and_percent_come_from_the_scans():
    # the bytes read are at 100% from the start (the tail of the file is read to open it): only the scans count
    cases = [({"total": 1000, "read": 1000, "written": 0, "scans": 0, "scansTotal": 0}, "reading", None), ({"total": 1000, "read": 1000, "written": 0, "scans": 0, "scansTotal": None}, "reading", None),
             ({"total": 1000, "read": 1000, "written": 0, "scans": 0, "scansTotal": 200}, "converting", 0),
             ({"total": 1000, "read": 1000, "written": 4000, "scans": 50, "scansTotal": 200}, "converting", 25), ({"total": 1000, "read": 1000, "written": 9000, "scans": 200, "scansTotal": 200}, "converting", 100),
             ({"total": 1, "read": 1, "written": 900, "copied": 300, "size": 900}, "converting", 33)]
    got = run(f"console.log(JSON.stringify({json.dumps([c for c, _, _ in cases])}.map(p=>[c.convPhase(p),c.convPercent(p)])))")
    assert got == [[ph, pc] for _, ph, pc in cases]
    assert run("console.log(JSON.stringify([c.convPhase(null),c.convPercent(null),c.convFraction(null)]))") == ["reading", None, 0]
    assert run("console.log(JSON.stringify([c.convFraction({scans:50,scansTotal:200}),c.convFraction({copied:1,size:2})]))") == [0.25, 1]


def test_total_percent_is_weighted_by_the_size_of_the_files():
    assert run("console.log(JSON.stringify([c.convTotalPercent([100,300],0,0.5),c.convTotalPercent([100,300],1,0.5),c.convTotalPercent([100,300],1,1),c.convTotalPercent([0],0,0)]))") == [12, 62, 100, None]


def test_elapsed_and_megabytes():
    assert run("console.log(JSON.stringify([0,999,1000,65000,3599000,3600000,7325000].map(c.convElapsed)))") == ["0:00", "0:00", "0:01", "1:05", "59:59", "1:00:00", "2:02:05"]
    assert run("console.log(JSON.stringify([c.convMB(1048576*3.14), c.convMB(1048576*250)]))") == ["3.1", "250"]


def test_converted_name_keeps_the_extension_when_taken():
    got = run("""console.log(JSON.stringify([c.convName("a.raw",[]), c.convName("a.RAW",["b.mzML"]), c.convName("a.raw",["A.mzml"]), c.convName("a.raw")]))""")
    assert got == ["a.mzML", "a.mzML", "a.raw.mzML", "a.mzML"]


def test_wiff_sample_name():
    got = run("""console.log(JSON.stringify([c.wiffName("FullMass",3,false,[]), c.wiffName("FullMass",3,true,[]), c.wiffName("FullMass",3,true,["fullmass_S3.mzml"]), c.wiffName("x",1,false,["X.mzML"])]))""")
    assert got == ["FullMass.mzML", "FullMass_s3.mzML", "FullMass_s3.wiff.mzML", "x.wiff.mzML"]


def test_the_rows_of_the_files_being_converted_have_a_text_for_each_state():
    # CONVROWS shows a sample in the list of the files from the pairing of its .wiff and .wiff.scan: one key per state, an unknown state is "queued"
    got = run("console.log(JSON.stringify(['open','wait','run','conv','index','x'].map(c.convRowKey)))")
    assert got == ["conv.row.open", "conv.row.wait", "conv.row.run", "conv.row.conv", "conv.row.index", "conv.row.wait"]
