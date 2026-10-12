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


def test_phase_and_percent():
    cases = [({"total": 0, "read": 0, "written": 0}, "reading", None), ({"total": 1000, "read": 100, "written": 0}, "reading", 10),
             ({"total": 1000, "read": 500, "written": 4000}, "converting", 50), ({"total": 1000, "read": 1000, "written": 9000}, "writing", 100),
             ({"total": 1, "read": 1, "written": 900, "copied": 300, "size": 900}, "writing", 33)]
    got = run(f"console.log(JSON.stringify({json.dumps([c for c, _, _ in cases])}.map(p=>[c.convPhase(p),c.convPercent(p)])))")
    assert got == [[ph, pc] for _, ph, pc in cases]
    assert run("console.log(JSON.stringify([c.convPhase(null),c.convPercent(null)]))") == ["reading", None]


def test_elapsed_and_megabytes():
    assert run("console.log(JSON.stringify([0,999,1000,65000,3599000,3600000,7325000].map(c.convElapsed)))") == ["0:00", "0:00", "0:01", "1:05", "59:59", "1:00:00", "2:02:05"]
    assert run("console.log(JSON.stringify([c.convMB(1048576*3.14), c.convMB(1048576*250)]))") == ["3.1", "250"]


def test_converted_name_keeps_the_extension_when_taken():
    got = run("""console.log(JSON.stringify([c.convName("a.raw",[]), c.convName("a.RAW",["b.mzML"]), c.convName("a.raw",["A.mzml"]), c.convName("a.raw")]))""")
    assert got == ["a.mzML", "a.mzML", "a.raw.mzML", "a.mzML"]


def test_wiff_sample_name():
    got = run("""console.log(JSON.stringify([c.wiffName("FullMass",3,false,[]), c.wiffName("FullMass",3,true,[]), c.wiffName("FullMass",3,true,["fullmass_S3.mzml"]), c.wiffName("x",1,false,["X.mzML"])]))""")
    assert got == ["FullMass.mzML", "FullMass_s3.mzML", "FullMass_s3.wiff.mzML", "x.wiff.mzML"]
