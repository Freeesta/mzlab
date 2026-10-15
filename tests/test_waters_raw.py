"""Pure parts of the MassLynx .raw loading (mzlab/web/waters-raw.js, run by node): which folder a file belongs to, the runs among the files of a loaded folder, and the names of the mzML."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "waters-raw.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    prelude = "const a=require(%r);const F=(n,p)=>{const f=new File(['x'],n);Object.defineProperty(f,'webkitRelativePath',{value:p+n});return f};" % str(JS)
    out = subprocess.run(["node", "-e", prelude + "(async()=>{const r=await(" + code + ");console.log(JSON.stringify(r))})()"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_a_file_knows_its_folder():
    r = run("[a.locate(F('_FUNC001.DAT','top/a.raw/')), a.locate(F('s.raw','top/')), a.locate(F('_FUNC002.DAT','top/a.raw/sub/')), a.locate(F('f','.raw/'))]")
    assert r[0] == {"root": "top/a.raw/", "name": "a.raw", "rel": "_FUNC001.DAT"}
    assert r[1] is None      # a Thermo .raw is a file
    assert r[2] is None      # only the files directly in the folder are members
    assert r[3] is None


def test_runs_are_grouped_and_the_other_files_pass():
    code = """(()=>{const fs=['_FUNC001.DAT','_FUNC001.IDX','_FUNCTNS.INF','_HEADER.TXT','.DS_Store'].map(n=>F(n,'top/a.raw/')).concat([F('_FUNC001.DAT','top/b.RAW/'),F('x.txt','top/c.raw/'),
      F('s.raw','top/'),F('x.mzML','top/'),F('MSScan.bin','top/d.D/AcqData/')]);const g=a.group(fs);
      return [g.rest.map(f=>f.name).sort(),g.runs.map(w=>[w.name,w.root,w.files.map(x=>x.rel)])]})()"""
    rest, runs = run(code)
    assert rest == sorted(["x.txt", "s.raw", "x.mzML", "MSScan.bin"])      # a *.raw folder without _FUNC001.DAT is not a run
    assert runs == [["a.raw", "top/a.raw/", ["_FUNC001.DAT", "_FUNC001.IDX", "_FUNCTNS.INF", "_HEADER.TXT"]], ["b.RAW", "top/b.RAW/", ["_FUNC001.DAT"]]]


def test_the_name_of_the_mzml():
    r = run("[a.mzmlName({name:'x.raw',root:'t/x.raw/'},[]), a.mzmlName({name:'x.raw',root:'t/s2/x.raw/'},['x.mzML']), a.mzmlName({name:'x.raw',root:'x.raw/'},['X.mzML'])]")
    assert r == ["x.mzML", "x (s2).mzML", "x (1).mzML"]
