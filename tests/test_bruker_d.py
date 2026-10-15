"""A Bruker timsTOF .d folder among the files (mzlab/web/agilent-d.js `group`, lettori-famiglie.js, run by node): the members tell Bruker from Agilent, never the extension."""
import json
import re
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    prelude = "const a=require(%r);const F=(n,p)=>{const f=new File(['x'],n);Object.defineProperty(f,'webkitRelativePath',{value:p+n});return f};" % str(WEB / "agilent-d.js")
    out = subprocess.run(["node", "-e", prelude + "(async()=>{const r=await(" + code + ");console.log(JSON.stringify(r))})()"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_a_bruker_folder_is_a_bruker_run_not_a_broken_agilent_one():
    code = """(()=>{const fs=['analysis.tdf','analysis.tdf_bin','analysis.tdf-wal','chromatography-data.sqlite'].map(n=>F(n,'t/s1.d/')).concat([F('analysis.tsf','t/m.d/')]);
      const g=a.group(fs);return g.runs.map(r=>[r.name,r.bruker,Object.keys(r.files).sort(),r.missing])})()"""
    runs = run(code)
    assert runs[0] == ["s1.d", True, ["tdf", "tdfbin", "tdfwal"], []]
    assert runs[1] == ["m.d", True, ["tsf"], ["analysis.tsf_bin"]]


def test_an_agilent_folder_is_not_bruker():
    code = """(()=>{const fs=[F('MSScan.bin','t/a.D/AcqData/'),F('MSPeak.bin','t/a.D/AcqData/')];return a.group(fs).runs.map(r=>[r.bruker,r.missing])})()"""
    assert run(code) == [[False, []]]


def test_baf_only_is_a_bruker_run():
    assert run("(()=>a.group([F('analysis.baf','t/b.d/')]).runs.map(r=>[r.bruker,Object.keys(r.files)]))()") == [[True, ["baf"]]]


def test_the_family_of_a_bruker_dataset_is_bruker_before_agilent():
    js = (WEB / "lettori-famiglie.js").read_text(encoding="utf-8").replace("export default LETTORI_FAM;", "").replace("window", "globalThis")
    names = ["x.d/analysis.tdf", "x.d/analysis.tdf_bin"]
    code = js + "const fam=(n)=>{const m=n.map(x=>x.toLowerCase());return Object.keys(LETTORI_FAM).find(k=>m.some(x=>LETTORI_FAM[k].ext.some(e=>x.endsWith(e))||LETTORI_FAM[k].membri.some(p=>x.includes(p))))||''};console.log(JSON.stringify([fam(%s),fam(['y.D/AcqData/MSScan.bin']),fam(['z.d/analysis.baf'])]))" % json.dumps(names)
    out = subprocess.run(["node", "-e", re.sub(r"^// @ts-check", "", code)], check=True, capture_output=True, encoding="utf-8").stdout
    assert json.loads(out) == ["bruker", "agilent", "bruker"]
