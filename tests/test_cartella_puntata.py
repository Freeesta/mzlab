"""Pure parts of the remembered folder (mzlab/web/cartella-puntata.js, run by node): the kinds of file, the recursive list of a (fake) directory handle
with hidden folders skipped, and the pairing of .wiff with its .wiff.scan."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "cartella-puntata.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    prelude = "const c=require(%r);" % str(JS) + """
const fileH=n=>({kind:'file',name:n});
const dirH=(n,kids)=>({kind:'directory',name:n,async*entries(){for(const k of kids)yield [k.name,k]}});
"""
    out = subprocess.run(["node", "-e", prelude + "(async()=>{const r=await(" + code + ");console.log(JSON.stringify(r))})()"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_kinds_of_file():
    names = ["a.mzML", "b.RAW", "c.wiff", "m.dam", "l.msp", "l.mgf", "l.mzlib", "t.csv", "t.tsv", "r.txt", "x.json", "d.pdf", "c.wiff.scan"]
    r = run("%s.map(n=>c.kindOf(n))" % json.dumps(names))
    assert r == ["data", "data", "data", "method", "library", "library", "library", "table", "table", None, None, None, None]


def test_collect_is_recursive_skips_hidden_and_keeps_scan_companions():
    r = run("""(async()=>{const root=dirH('r',[fileH('a.mzML'),fileH('note.txt'),dirH('sub',[fileH('w.wiff'),fileH('w.wiff.scan'),fileH('m.dam'),dirH('.git',[fileH('x.mzML')])]),dirH('.cache',[fileH('y.mzML')])]);
      const it=await c.collect(root);return [it.map(x=>x.path+':'+x.kind),c.visible(it).map(x=>x.path)]})()""")
    assert r[0] == ["a.mzML:data", "sub/w.wiff:data", "sub/w.wiff.scan:scan", "sub/m.dam:method"]
    assert r[1] == ["a.mzML", "sub/w.wiff", "sub/m.dam"]


def test_scan_pairs_only_in_the_same_folder():
    r = run("""(async()=>{const root=dirH('r',[dirH('a',[fileH('S.wiff')]),dirH('b',[fileH('s.wiff.scan')])]);
      const it=await c.collect(root);const w=it.find(x=>x.kind==='data');return [c.pairScan(it,w),c.visible(it).length]})()""")
    assert r == [None, 2]


def test_desktop_side_lists_through_the_native_commands_and_opens_a_stub_file():
    desk = JS.with_name("cartella-puntata-desktop.js")
    code = """(async()=>{
      const calls=[];global.window={MZLAB_DESKTOP:true,__TAURI__:{core:{invoke:async(c,a)=>{calls.push(c);
        if(c==='data_get_dir')return 'lab'; if(c==='data_list')return [{path:'sub/a.mzML',name:'a.mzML',dir:'sub/',kind:'data',size:7,mtime:5}];
        if(c==='data_register')return {id:3,name:'a.mzML',size:7}; }}}};
      global.document={hidden:false};
      const d=require(%r);const S={items:[],stats:new Map(),opened:new Set(),name:'',perm:'none'};let drawn=0;
      await d.start(S,()=>drawn++);const f=await d.fileOf(S.items[0]);
      clearInterval(setInterval(()=>{},1));
      return [d.available(),S.name,S.items.length,S.stats.get('sub/a.mzML').size,drawn>0,f.type,f.size,f.name,calls]})()""" % str(desk)
    out = subprocess.run(["node", "-e", "(async()=>{const r=await(" + code + ");console.log(JSON.stringify(r));process.exit(0)})()"], check=True, capture_output=True, encoding="utf-8").stdout
    r = json.loads(out)
    assert r[:8] == [True, "lab", 1, 7, True, "application/x-mzlab-file;id=3", 7, "a.mzML"]
    assert r[8] == ["data_get_dir", "data_list", "data_register"]
