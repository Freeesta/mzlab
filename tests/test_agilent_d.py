"""Pure parts of the Agilent .D loading (mzlab/web/agilent-d.js, run by node): which folder a file belongs to, the runs among the files of a loaded folder, and the names of the mzML."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "agilent-d.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    prelude = "const a=require(%r);const F=(n,p)=>{const f=new File(['x'],n);Object.defineProperty(f,'webkitRelativePath',{value:p+n});return f};" % str(JS)
    out = subprocess.run(["node", "-e", prelude + "(async()=>{const r=await(" + code + ");console.log(JSON.stringify(r))})()"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_a_file_knows_its_folder():
    r = run("[a.locate(F('MSScan.bin','top/x.D/AcqData/')), a.locate(F('acqmeth.txt','x.d/')), a.locate(F('a.mzML','top/')), a.locate(F('f','.D/'))]")
    assert r[0] == {"root": "top/x.D/", "name": "x.D", "rel": "AcqData/MSScan.bin"}
    assert r[1] == {"root": "x.d/", "name": "x.d", "rel": "acqmeth.txt"}
    assert r[2] is None and r[3] is None


def test_runs_are_grouped_and_the_other_files_pass():
    code = """(()=>{const fs=[F('MSScan.bin','t/a.D/AcqData/'),F('MSPeak.bin','t/a.D/AcqData/'),F('acqmeth.txt','t/a.D/'),F('TEMPBASE','t/a.D/'),F('x.mzML','t/'),
      F('MSScan.bin','t/b.D/AcqData/'),F('data.ms','t/c.D/')];const g=a.group(fs);
      return [g.rest.map(f=>f.name),g.runs.map(r=>[r.name,Object.keys(r.files).sort(),r.missing])]})()"""
    rest, runs = run(code)
    assert rest == ["x.mzML"]
    assert runs[0] == ["a.D", ["acqmeth", "peak", "scan"], []]
    assert runs[1] == ["b.D", ["scan"], ["AcqData/MSPeak.bin"]]
    assert runs[2] == ["c.D", ["datams"], []]


def test_the_name_of_the_mzml():
    r = run("[a.mzmlName({name:'x.D',root:'t/x.D/'},[]), a.mzmlName({name:'x.D',root:'t/s2/x.D/'},['x.mzML']), a.mzmlName({name:'x.D',root:'x.D/'},['X.mzML'])]")
    assert r == ["x.mzML", "x (s2).mzML", "x (1).mzML"]
