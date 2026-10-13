"""Pure parts of the loading of the files of a maker as they are (mzlab/web/nativo.js, run by node): the key of a dataset, the File that stands for a sample, the .raw files, the state of a row by phase."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    prelude = "global.convName=require(%r).convName;const n=require(%r);" % (str(WEB / "conversione.js"), str(WEB / "nativo.js"))
    out = subprocess.run(["node", "-e", prelude + "(async()=>{const r=await(" + code + ");console.log(JSON.stringify(r))})()"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_the_key_is_safe_and_unique():
    a, b = run("[n.key('a b/c.wiff'), n.key('a b/c.wiff')]")
    assert a != b and a.startswith("a_b_c.wiff-") and "/" not in a and " " not in a


def test_a_sample_is_a_file_with_its_members():
    r = run("""(()=>{const m=[{name:'a.wiff',blob:new Blob(['xx'])},{name:'a.wiff.scan',blob:new Blob(['yyyy'])}],f=n.sample('a_s1.mzML','k',m,1,7,false);
      return [f.name,f.mzlabNat.key,f.mzlabNat.c,f.mzlabNat.id,f.mzlabNat.size,n.size(f),n.size(new File(['abc'],'x.mzML'))]})()""")
    assert r == ["a_s1.mzML", "k", 1, 7, 6, 6, 3]


def test_raw_files_become_samples_and_the_rest_goes_on():
    r = run("""(()=>{const out=n.raws([new File(['RAW'],'t5.raw'),new File(['<x/>'],'a.mzML')],['t5.mzML']);
      return out.map(f=>[f.name,!!f.mzlabNat,f.mzlabNat&&f.mzlabNat.members[0].name])})()""")
    assert r[0][1] is True and r[0][2] == "t5.raw" and r[0][0] != "t5.mzML" and r[0][0].endswith(".mzML")
    assert r[1] == ["a.mzML", False, None]


def test_the_state_of_a_row_follows_the_phase():
    assert run("['read','scans','chrom','x',undefined].map(n.rowState)") == ["nread", "nscan", "nchrom", "index", "index"]
