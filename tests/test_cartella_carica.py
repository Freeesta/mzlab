"""Pure parts of the folder loading (mzlab/web/cartella-carica.js, run by node): folder of a file, compatible files, wiff pairing inside the same
folder, equal names in different folders and the recursive reading of dropped entries (simulated, in batches)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

JS = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "cartella-carica.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run(code):
    prelude = "const c=require(%r);const F=(n,p)=>{const f=new File(['x'],n);if(p!==undefined)Object.defineProperty(f,'webkitRelativePath',{value:p+n});return f};" % str(JS)
    out = subprocess.run(["node", "-e", prelude + "(async()=>{const r=await(" + code + ");console.log(JSON.stringify(r))})()"], check=True, capture_output=True, encoding="utf-8").stdout
    return json.loads(out)


def test_dir_of_a_file_chosen_alone_and_of_a_folder():
    assert run("[c.dir(F('a.mzML')), c.dir(F('a.mzML','top/sub/'))]") == ["", "top/sub/"]


def test_split_keeps_compatible_files_and_counts_the_others():
    r = run("(()=>{const s=c.split([F('a.mzML','t/'),F('b.dam','t/'),F('c.wiff','t/'),F('c.wiff.scan','t/'),F('d.raw','t/'),F('notes.txt','t/'),F('e.d','t/'),F('.hid.mzML','t/'),F('x.mzML','t/.cache/')]);return [s.ok.map(f=>f.name),s.ignored.length]})()")
    assert r == [["a.mzML", "b.dam", "c.wiff", "c.wiff.scan", "d.raw"], 4]


def test_wiff_pairs_only_inside_the_same_folder():
    r = run("(()=>{const k=(n,p)=>c.pairKey(F(n,p),n.replace(/\\.wiff(\\.scan)?$/i,''));return [k('S.wiff','a/')===k('S.wiff.scan','a/'),k('S.wiff','a/')===k('S.wiff.scan','b/'),k('S.wiff')===k('s.WIFF.scan')]})()")
    assert r == [True, False, True]


def test_equal_names_in_different_folders_are_told_apart_and_wiff_keeps_its_name():
    r = run("(()=>{const o=c.uniqueNames([F('s.mzML','t/a/'),F('s.mzML','t/b/'),F('s.mzML','t/c/'),F('w.wiff','t/a/'),F('w.wiff','t/b/')]);return [o.map(f=>f.name),o.map(c.dir)]})()")
    assert r[0] == ["s.mzML", "s (b).mzML", "s (c).mzML", "w.wiff", "w.wiff"]
    assert r[1] == ["t/a/", "t/b/", "t/c/", "t/a/", "t/b/"]
    assert run("c.uniqueNames([F('s.mzML','t/')],['S.mzML']).map(f=>f.name)") == ["s (t).mzML"]


FAKE = """
const file=n=>({isFile:true,isDirectory:false,name:n,file:ok=>ok(new File(['x'],n))});
const dirE=(n,kids,batch=2)=>({isFile:false,isDirectory:true,name:n,createReader(){let i=0;return{readEntries(ok){const b=kids.slice(i,i+batch);i+=batch;ok(b)}}}});
const root=dirE('lab',[file('a.mzML'),file('b.txt'),dirE('sub',[file('c.mzML'),file('d.dam'),file('e.wiff'),dirE('.git',[file('x.mzML')])]),file('f.mzML')]);
"""


def test_dropped_folder_is_read_recursively_in_batches():
    r = run("(async()=>{" + FAKE + "const seen=[];const fs=await c.readEntries([root],n=>seen.push(n));"
            "const dt={items:[{kind:'file',webkitGetAsEntry:()=>root}]},no={items:[{kind:'file',webkitGetAsEntry:()=>file('z.mzML')}]};"
            "return [fs.map(f=>c.dir(f)+f.name),seen.length,!!c.entriesOf(dt),c.entriesOf(no)]})()")
    assert r[0] == ["lab/a.mzML", "lab/b.txt", "lab/sub/c.mzML", "lab/sub/d.dam", "lab/sub/e.wiff", "lab/f.mzML"]
    assert r[1] == 6 and r[2] is True and r[3] is None
