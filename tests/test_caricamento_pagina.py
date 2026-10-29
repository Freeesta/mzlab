"""Page side of the folder and module loading (cartella-puntata.js, lettore-esterno.js, lettori-modulo.js), run by node with fake DOM, handles and workers:
one-pass visible(), same-named files, "Load all" pressed twice, a scan of the folder left behind, parallel stats, no redraw under the pointer,
a stale SHA-1 in the engine's module bridge and the retried drop of a cancelled conversion."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

PRELUDE = """
const handlers = {};
const box = { hidden: false, _html: "", renders: 0, els: {},
  set innerHTML(v) { this._html = v; this.renders++; this.els = {}; }, get innerHTML() { return this._html; },
  querySelector(s) { if (!(s in this.els)) { const has = { "#cartforget": 1, "#cartrefresh": 1, "#cartgo": this._html.includes('id="cartgo"'), "#cartgrant": this._html.includes('id="cartgrant"') }[s]; this.els[s] = has ? {} : null; } return this.els[s]; },
  querySelectorAll() { return []; } };
const pick = { hidden: false };
global.document = { hidden: false, getElementById: id => id === "cartbody" ? box : id === "cartpick" ? pick : null, addEventListener: (t, f) => { handlers[t] = f; } };
global.addEventListener = (t, f) => { handlers[t] = f; };
global.ST = { files: [] };
global.CAPACITA = { ha: () => true };
global.window = { ST, CAPACITA, showDirectoryPicker: async () => { throw new Error("none"); } };
global.I18N = { t: k => k };
global.CartCarica = { setDir: f => f, note() {} };
global.sz = n => String(n);
const sleep = ms => new Promise(r => setTimeout(r, ms));
const fileH = (n, ms = 0, log = null) => ({ kind: "file", name: n, getFile: async () => { if (log) log.live++, log.max = Math.max(log.max, log.live); if (ms) await sleep(ms); if (log) log.live--; return { name: n, size: n.length, lastModified: 1 }; } });
const dirH = (n, kids, gate = null) => ({ kind: "directory", name: n, queryPermission: async () => "granted", async *entries() { if (gate) await gate; for (const k of kids) yield [k.name, k]; } });
const c = require(%r);
const S = c.state;
"""


def run(code, tmp_path):
    f = tmp_path / "t.cjs"
    f.write_text(PRELUDE % str(WEB / "cartella-puntata.js") + "(async()=>{const r=await(" + code + ");console.log(JSON.stringify(r));process.exit(0)})()")
    return json.loads(subprocess.run(["node", str(f)], check=True, capture_output=True, encoding="utf-8").stdout)


def test_visible_matches_the_old_rule_and_is_fast(tmp_path):
    r = run("""(()=>{
      const old=(items,pair)=>items.filter(x=>x.kind!=="scan"||!items.some(w=>/\\.wiff$/i.test(w.name)&&pair(items,w)===x));
      const mk=n=>{const it=[];for(let i=0;i<n;i++){const d='d'+(i%7)+'/',nm='S'+i+'.wiff';it.push({path:d+nm,name:nm,dir:d,kind:'data'});if(i%3)it.push({path:d+nm+'.scan',name:nm+'.scan',dir:d,kind:'scan'});}
        it.push({path:'x/lone.wiff.scan',name:'lone.wiff.scan',dir:'x/',kind:'scan'},{path:'x/A.WIFF',name:'A.WIFF',dir:'x/',kind:'data'},{path:'x/a.wiff.SCAN',name:'a.wiff.SCAN',dir:'x/',kind:'scan'});return it};
      const small=mk(60),same=JSON.stringify(c.visible(small).map(x=>x.path))===JSON.stringify(old(small,c.pairScan).map(x=>x.path));
      const big=mk(1000),t=Date.now(),v=c.visible(big);return [same,v.length,Date.now()-t]})()""", tmp_path)
    assert r[0] is True
    assert r[1] > 1000
    assert r[2] < 300


def test_same_name_in_two_folders_is_not_loaded_by_name(tmp_path):
    r = run("""(()=>{
      S.handle={};S.perm='granted';S.name='r';ST.files=[{name:'qc.mzML'},{name:'solo.mzML'}];
      S.items=['pos/qc.mzML','neg/qc.mzML','solo.mzML'].map(p=>({path:p,name:p.split('/').pop(),dir:p.includes('/')?p.split('/')[0]+'/':'',kind:'data',h:fileH('x')}));
      c.render();return [(box.innerHTML.match(/cprow done/g)||[]).length]})()""", tmp_path)
    assert r == [1]


def test_load_all_pressed_twice_loads_each_file_once(tmp_path):
    r = run("""(async()=>{
      const ups=[];global.upload=async(fs)=>{ups.push(fs[0].name);await sleep(30)};
      S.handle={};S.perm='granted';S.name='r';
      S.items=['a.mzML','b.mzML'].map(n=>({path:n,name:n,dir:'',kind:'data',h:fileH(n)}));
      c.render();const p=box.querySelector('#cartgo').onclick();await sleep(45);
      const mid=box._html,b=box.querySelector('#cartgo');if(b&&b.onclick)b.onclick();await p;await sleep(80);
      return [ups,/id="cartgo"[^>]*disabled/.test(mid)]})()""", tmp_path)
    assert r == [["a.mzML", "b.mzML"], True]


def test_scan_of_the_old_folder_does_not_land_under_the_new_name(tmp_path):
    r = run("""(async()=>{
      let open;const gate=new Promise(r=>open=r);
      S.handle=dirH('old',[fileH('old.mzML')],gate);S.name='old';
      const first=c.rescan();await sleep(10);
      window.showDirectoryPicker=async()=>dirH('new',[fileH('new.mzML')]);
      await c.choose();open();await first;
      for(let i=0;i<50&&(S.busy||!S.items.length);i++)await sleep(10);
      return [S.name,S.items.map(x=>x.path)]})()""", tmp_path)
    assert r == ["new", ["new.mzML"]]


def test_stats_are_read_in_parallel_and_keep_their_order(tmp_path):
    r = run("""(async()=>{
      const log={live:0,max:0},kids=[];for(let i=0;i<60;i++)kids.push(fileH('f'+String(i).padStart(2,'0')+'.mzML',5,log));
      S.handle=dirH('r',kids);S.name='r';await c.rescan();
      return [S.stats.size,[...S.stats.keys()].every((k,i,a)=>!i||a[i-1]<k),log.max>1]})()""", tmp_path)
    assert r == [60, True, True]


def test_no_redraw_while_the_pointer_is_down_on_the_list(tmp_path):
    r = run("""(()=>{
      S.handle={};S.perm='granted';S.name='r';S.items=[{path:'a.mzML',name:'a.mzML',dir:'',kind:'data',h:fileH('a')}];
      c.render();const n0=box.renders;handlers.pointerdown({target:{closest:()=>({})}});c.render();const held=box.renders;handlers.pointerup();
      return [held-n0,box.renders-n0]})()""", tmp_path)
    assert r == [0, 1]


def esm(tmp_path, files, code):
    for name in files:
        (tmp_path / name).write_text((WEB / name).read_text(encoding="utf-8"), encoding="utf-8")
    (tmp_path / "package.json").write_text('{"type":"module"}')
    f = tmp_path / "t.mjs"
    f.write_text(code)
    return json.loads(subprocess.run(["node", str(f)], check=True, capture_output=True, encoding="utf-8").stdout)


def test_a_member_without_sha1_does_not_inherit_the_earlier_one(tmp_path):
    thermo = tmp_path / "wasm-lettori" / "thermo"
    thermo.mkdir(parents=True)
    (tmp_path / "wasm-lettori" / "manifest.json").write_text(json.dumps({"thermo": {"sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}}))
    (thermo / "mzlab_lettori_wasm_bg.wasm").write_bytes(b"")
    (thermo / "mzlab_lettori_wasm.js").write_text("export default async () => {}; export const apri_con_sha1 = (n, s, r, sums) => { globalThis.calls.push(sums); return 1; }; export const apri = () => { globalThis.calls.push(null); return 1; }; export const chiudi = () => {};")
    r = esm(tmp_path, ["lettore-esterno.js", "carica-modulo.js"], """
      import { readFileSync } from "node:fs"; import { fileURLToPath } from "node:url";
      globalThis.fetch = async u => { const b = readFileSync(fileURLToPath(u)); return { json: async () => JSON.parse(b.toString()), arrayBuffer: async () => b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength) }; };
      globalThis.calls = [];
      const { createEsterno } = await import("./lettore-esterno.js");
      let open; const e = createEsterno(o => { open = o; });
      await e.prepara(["a.raw"], [{ name: "a.raw", size: 5, sha1: "AA" }]);
      const first = open(["a.raw"], [5], () => {}); first.chiudi();
      await e.prepara(["a.raw"], [{ name: "a.raw", size: 5 }]);
      open(["a.raw"], [5], () => {}).chiudi();
      console.log(JSON.stringify(calls));
      process.exit(0);
    """)
    assert r == [["AA"], None]


def test_drop_of_a_cancelled_conversion_is_retried(tmp_path):
    r = esm(tmp_path, ["lettori-modulo.js"], """
      let fails = 2, removed = null;
      globalThis.window = globalThis; window.LETTORI_FAM = {};
      globalThis.Worker = class { postMessage() {} terminate() {} };
      const conv = { removeEntry: async n => { if (fails-- > 0) throw Object.assign(new Error("busy"), { name: "NoModificationAllowedError" }); removed = n; } };
      Object.defineProperty(globalThis, "navigator", { value: { storage: { getDirectory: async () => ({ getDirectoryHandle: async n => n === "mzlab" ? ({ getDirectoryHandle: async () => conv }) : conv }) } }, configurable: true });
      const M = (await import("./lettori-modulo.js")).default, ac = new AbortController();
      const p = M.toMzml("x", [], 0, "d.mzML", undefined, ac.signal).catch(e => e.cancelled);
      ac.abort(); const cancelled = await p;
      await new Promise(r => setTimeout(r, 700));
      console.log(JSON.stringify([cancelled, removed]));
      process.exit(0);
    """)
    assert r == [True, "d.mzML"]
