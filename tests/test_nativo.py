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
    r = run("""(()=>{const m=[{name:'a.wiff',blob:new Blob(['xx'])},{name:'a.wiff.scan',blob:new Blob(['yyyy'])}],
      f=n.sample('a_s1.mzML','k',m,1,7,false),
      f_with_sz=n.sample('a_s2.mzML','k',m,2,8,false,500),
      f_single=n.sample('r.mzML','k',[{name:'r.raw',blob:new Blob(['abc'])}],0,1,false);
      return [f.name,f.mzlabNat.key,f.mzlabNat.c,f.mzlabNat.id,f.mzlabNat.size,n.size(f),n.size(f_with_sz),n.size(f_single),n.size(new File(['abc'],'x.mzML'))]})()""")
    assert r == ["a_s1.mzML", "k", 1, 7, None, None, 500, 3, 3]


def test_raw_files_become_samples_and_the_rest_goes_on():
    r = run("""(()=>{const out=n.raws([new File(['RAW'],'t5.raw'),new File(['<x/>'],'a.mzML')],['t5.mzML']);
      return out.map(f=>[f.name,!!f.mzlabNat,f.mzlabNat&&f.mzlabNat.members[0].name])})()""")
    assert r[0][1] is True and r[0][2] == "t5.raw" and r[0][0] != "t5.mzML" and r[0][0].endswith(".mzML")
    assert r[1] == ["a.mzML", False, None]


def test_the_state_of_a_row_follows_the_phase():
    assert run("['read','scans','chrom','x',undefined].map(n.rowState)") == ["nread", "nscan", "nchrom", "index", "index"]


def test_only_a_plain_numpress_mzml_becomes_a_native_sample_and_the_order_is_kept():
    r = run("""(async()=>{
      const flagged=n.sample('n.mzML','k',[{name:'n.mzML',blob:new Blob(['x'])}],0,1,false),
        mod=new File(['m'],'m.mzML'); Object.defineProperty(mod,'mzlabModulo',{value:{size:1}});
      const names=fs=>fs.map(f=>[f.name,!!f.mzlabNat]), yes=async()=>true, no=async()=>false;
      const mk=(...ns)=>ns.map(x=>new File(['<x/>'],x)), seen=[];
      return {
        empty:(await n.numpress([],yes)).length,
        lower:names(await n.numpress(mk('a.mzml'),yes)), upper:names(await n.numpress(mk('a.MZML'),yes)),
        two:names(await n.numpress(mk('a.mzML','b.mzML'),yes)),
        mixed:names(await n.numpress(mk('a.mzML','t.raw','b.mzML'),yes)),
        flagged:(await n.numpress([flagged,mod],yes)).map(f=>f.mzlabNat===flagged.mzlabNat||f===mod),
        notNumpress:names(await n.numpress(mk('a.mzML','b.mzML'),no)),
        skipped:await (async()=>{await n.numpress([...mk('t.raw'),flagged,mod],async f=>{seen.push(f.name);return true});return seen})(),
        some:names(await n.numpress(mk('x.mzML','y.mzML','z.mzML'),async f=>f.name==='y.mzML'))};
    })()""")
    assert r["empty"] == 0
    assert r["lower"] == [["a.mzml", True]] and r["upper"] == [["a.MZML", True]]
    assert r["two"] == [["a.mzML", True], ["b.mzML", True]]
    assert r["mixed"] == [["a.mzML", True], ["t.raw", False], ["b.mzML", True]]
    assert r["flagged"] == [True, True]
    assert r["notNumpress"] == [["a.mzML", False], ["b.mzML", False]]
    assert r["skipped"] == []
    assert r["some"] == [["x.mzML", False], ["y.mzML", True], ["z.mzML", False]]


def test_a_dataset_the_module_wrote_is_found_again_by_its_members():
    """cachedModulo: the same members (names, sizes, dates) as the source of a listed sample -> its row says «from the cache»; any difference -> not found."""
    r = run("""(()=>{const rows=[];global.CONVROWS={add:(k,s)=>rows.push(['add',k,s]),finish:(k,s,o)=>rows.push(['finish',k,s,o.cached])};
      const mk=(n,sz,t)=>({name:n,blob:{size:sz,lastModified:t}}), src=[mk('x.D/AcqData/a.bin',10,5),mk('x.D/AcqData/b.bin',20,6)];
      global.CONV={sources:new Map([['x.mzML',{fam:'agilent',mem:src,c:0}]])};
      const list=[{name:'x.mzML'},{name:'other.mzML'}];
      const same=n.cachedModulo([src[1],src[0]],list), rowsSame=rows.splice(0);
      const size=n.cachedModulo([mk('x.D/AcqData/a.bin',11,5),src[1]],list), date=n.cachedModulo([mk('x.D/AcqData/a.bin',10,9),src[1]],list),
        fewer=n.cachedModulo([src[0]],list), gone=n.cachedModulo(src,[{name:'other.mzML'}]);
      return [same,rowsSame,size,date,fewer,gone,rows.length]})()""")
    assert r == [True, [["add", "x.mzML", "open"], ["finish", "x.mzML", "done", True]], False, False, False, False, 0]


def test_the_desktop_turns_the_module_types_into_the_samples_its_native_side_lists():
    """desktop: an extension only a module reads, or an mzML `maybe` lets through that the native side lists (Numpress), becomes its samples; a plain mzML the native side
    refuses, a file of another type, an mzML `maybe` stops and a sample already made go on as they are; the samples share one key and the stub as member."""
    r = run("""(async()=>{
      const mk=(...ns)=>ns.map(x=>new File(['zz'],x)), asked=[], flagged=n.sample('f.mzML','k',[{name:'f.raw',blob:new Blob(['x'])}],0,1,false);
      const samplesOf=async m=>{asked.push(m[0].name);if(m[0].name==='plain.mzML')throw new Error('err.file.notAccepted');
        return m[0].name==='two.lcd'?{samples:[{id:3,name:'A',size:7},{id:4,name:'B',size:9}]}:{samples:[{id:0,name:'s'}]}};
      const nameOf=x=>x.replace(/\\.[^.]+$/,'')+'.mzML', maybe=async f=>f.name!=='page.mzML';
      const out=await n.desktop([...mk('np.mzML','plain.mzML','page.mzML','x.mzXML','t.txt','two.lcd'),flagged],['.mzxml','.lcd'],maybe,samplesOf,nameOf);
      return {rows:out.map(f=>[f.name,f.mzlabNat?[f.mzlabNat.c,f.mzlabNat.id,f.mzlabNat.size,f.mzlabNat.members[0].name]:null]),
        keys:out.filter(f=>f.name==='A.mzML'||f.name==='B.mzML').map(f=>f.mzlabNat.key),asked,flagged:out[out.length-1]===flagged};
    })()""")
    assert r["rows"][:-1] == [["np.mzML", [0, 0, 2, "np.mzML"]], ["plain.mzML", None], ["page.mzML", None], ["x.mzML", [0, 0, 2, "x.mzXML"]], ["t.txt", None],
                              ["A.mzML", [0, 3, 7, "two.lcd"]], ["B.mzML", [1, 4, 9, "two.lcd"]]]
    assert len(set(r["keys"])) == 1 and r["flagged"]
    assert r["asked"] == ["np.mzML", "plain.mzML", "x.mzXML", "two.lcd"]
