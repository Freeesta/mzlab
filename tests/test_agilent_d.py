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


def test_a_gc_run_with_data_ms_on_top_and_only_contents_in_acqdata_is_a_run():
    # synthetic shape of a GC/MS .D copied without MSScan.bin: data.ms at the top, AcqData holds Contents.xml only
    code = """(()=>{const fs=[F('data.ms','x/s1.D/'),F('Contents.xml','x/s1.D/AcqData/'),F('acqmeth.txt','x/s1.D/'),F('checksum.xml','x/s1.D/'),F('GC.ini','x/s1.D/')];
      const g=a.group(fs);return g.runs.map(r=>[r.name,Object.keys(r.files).sort(),r.missing])})()"""
    assert run(code) == [["s1.D", ["acqmeth", "contents", "datams"], []]]


def test_a_folder_that_is_not_whole_lists_what_was_received_with_sizes():
    code = """(()=>{const mk=(n,p,k)=>{const f=new File([new Uint8Array(k)],n);Object.defineProperty(f,'webkitRelativePath',{value:p+n});return f};
      const g=a.group([mk('Contents.xml','x/s2.D/AcqData/',1200),mk('GC.ini','x/s2.D/',10)]);return [g.runs[0].missing,g.runs[0].found]})()"""
    missing, found = run(code)
    assert missing == ["AcqData/MSScan.bin", "AcqData/MSPeak.bin"]
    assert found == ["AcqData/Contents.xml (1.2 kB)", "GC.ini (10 B)"]


def test_the_name_of_the_mzml():
    r = run("[a.mzmlName({name:'x.D',root:'t/x.D/'},[]), a.mzmlName({name:'x.D',root:'t/s2/x.D/'},['x.mzML']), a.mzmlName({name:'x.D',root:'x.D/'},['X.mzML'])]")
    assert r == ["x.mzML", "x (s2).mzML", "x (1).mzML"]


def test_method_folders_are_held_not_ignored():
    code = """(()=>{const fs=[F('Acq.RTP.xml','t/Geosmin method for scan.m/'),F('7890Method.bin','t/Geosmin method for scan.m/'),F('.DS_Store','t/Geosmin method for scan.m/'),
      F('x.mzML','t/'),F('m.m','t/'),F('AcqMethod.xml','t/b.M/DaMethod/'),F('acqmeth.txt','t/a.D/')];const g=a.groupMethods(fs);
      return [g.rest.map(f=>f.name),g.methods.map(m=>[m.name,m.root,m.files.map(f=>f.name)])]})()"""
    rest, methods = run(code)
    assert rest == ["x.mzML", "m.m", "acqmeth.txt"]      # a lone «m.m» file is not a folder; the .D files are left to group()
    assert methods == [["Geosmin method for scan.m", "t/Geosmin method for scan.m/", ["Acq.RTP.xml", "7890Method.bin"]], ["b.M", "t/b.M/", ["AcqMethod.xml"]]]


def test_uppercase_d_folders_with_acqdata_are_runs():
    code = """(()=>{const fs=['MSScan.bin','MSPeak.bin'].map(n=>F(n,'Dichloromethane/dichloromethane.D/AcqData/')).concat([F('acqmeth.txt','Dichloromethane/dichloromethane.D/'),
      F('MSScan.bin','blank m6en3.D/ACQDATA/'),F('MSPeak.bin','blank m6en3.D/ACQDATA/')]);const g=a.group(fs);
      return [g.rest.length,g.runs.map(r=>[r.name,Object.keys(r.files).sort(),r.missing])]})()"""
    rest, runs = run(code)
    assert rest == 0
    assert runs == [["dichloromethane.D", ["acqmeth", "peak", "scan"], []], ["blank m6en3.D", ["peak", "scan"], []]]
