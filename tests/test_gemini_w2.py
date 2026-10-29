"""Loading path, page side (shell.js, browser.js, carica.js, nativo.js, cartella-carica.js), run by node with fake globals; each case fails on the code before its fix:
a load error takes the full-screen veil down, the skeleton goes away after F5 with a session, a used lone .wiff leaves WIFF_WAIT, a re-dropped pair with DATA names is a cache hit,
the isolation reload waits for the loading, a failed folder load still ends the drop, a failed script is retried, and the engine module and the OPFS count start with the engine."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def cut(name, start, end):
    s = (WEB / name).read_text(encoding="utf-8")
    a = s.index(start)
    return s[a:s.index(end, a)]


def node(tmp_path, code, ext="cjs"):
    f = tmp_path / f"t.{ext}"
    f.write_text(code, encoding="utf-8")
    r = subprocess.run(["node", str(f)], capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0, (r.stdout + r.stderr)[-3000:]
    return json.loads(r.stdout.strip().splitlines()[-1])


def test_e2_a_load_error_takes_the_veil_down_and_shows_the_message(tmp_path):
    handler = cut("shell.js", '$("#opbtn").onclick=', "// both exits of the loading screen")
    code = """
const calls = [], els = {};
const el = id => els[id] || (els[id] = { innerHTML: "", disabled: false, onclick: null });
const $ = id => el(id);
const loading = v => calls.push(v);
const h = (s, ...v) => s.reduce((a, x, i) => a + x + (i < v.length ? v[i] : ""), "");
let UPBUSY = 0;
const ST = { files: [{ name: "a.mzML" }] }, usable = () => true, checkGo = () => {}, RUOLI = { roleOf: () => "x" }, darkTime = t => t;
const I18N = { t: k => k };
const openStaged = async () => { throw Object.assign(new Error("boom"), { key: "err.load.failed" }); };
%s
(async () => { await $("#opbtn").onclick(); console.log(JSON.stringify({ calls, msg: el("#gomsg").innerHTML, enabled: !el("#opbtn").disabled })) })();
""" % handler
    r = node(tmp_path, code)
    assert r["calls"] == [True, False] and "boom" in r["msg"] and r["enabled"] is True


def test_e3_the_skeleton_goes_away_when_a_session_is_restored(tmp_path):
    init = cut("shell.js", "async function init(){", "\n\n// ----")
    code = """
const log = [], S = { sess: true }, E = { panels: [] };
const $ = () => ({ hidden: true });
const AVVIO = { segnaPronto() {}, segnaSessione() {}, segnaFallito() {}, rimuoviScheletro() { log.push("skeleton"); } };
const api = async () => ({ app: true }), fail = () => {}, bootSession = async () => {}, applyView = () => log.push("view"), showStart = () => log.push("start"), loading = () => {}, prefetchIdle = () => {};
const window = {}, BARRA = null, QQRESET = null;
%s
init().then(() => { console.log(JSON.stringify(log)); process.exit(0); });
""" % init
    assert "skeleton" in node(tmp_path, code)


def test_e5_a_used_lone_wiff_leaves_the_pairing_table(tmp_path):
    fn = cut("shell.js", "async function loneWiffs", "const INFLIGHT")
    code = """
const CartCarica = require(%r);
const I18N = { t: k => k }, namesCut = a => a.join(), WIFF_WAIT = new Map();
const wiff = new File(["x"], "S.wiff"); Object.defineProperty(wiff, "_dir", { value: "a/" });
WIFF_WAIT.set(CartCarica.pairKey(wiff, "S"), { stem: "S", wiff });
%s
const rb = { metodoWiff: async () => ({ matched: [{ name: "S_1.mzML" }] }) }, rep = { notes: [], errs: {} };
loneWiffs({ lone: [{ e: { stem: "S", wiff }, rb }] }, rep).then(() => console.log(JSON.stringify({ left: WIFF_WAIT.size, notes: rep.notes.length })));
""" % (str(WEB / "cartella-carica.js"), fn)
    assert node(tmp_path, code) == {"left": 0, "notes": 1}


def test_e6_a_pair_with_data_names_is_found_in_the_cache(tmp_path):
    code = """
global.window = {}; global.convName = require(%r).convName;
const n = require(%r);
(async () => {
  const asked = [];
  const rb = { nativeSame: async (mem, names) => { asked.push(names); return names; }, nativeSamples: async () => ({ samples: [1, 2] }) };
  const list = [{ name: "dataA.mzML" }, { name: "dataB.mzML" }, { name: "other.mzML" }];
  const hit = await n.cached(rb, [], list);
  console.log(JSON.stringify({ asked: asked[0], hit: hit && hit.length }));
})();
""" % (str(WEB / "conversione.js"), str(WEB / "nativo.js"))
    assert node(tmp_path, code) == {"asked": ["dataA.mzML", "dataB.mzML", "other.mzML"], "hit": None}


def test_e12a_a_failed_folder_load_shows_its_message_and_ends_the_drop(tmp_path):
    code = """
const els = {}; global.document = { getElementById: id => els[id] || (els[id] = { textContent: "", hidden: true }) };
global.I18N = { t: k => k }; global.ST = { files: [] };
const c = require(%r);
const file = n => ({ isFile: true, isDirectory: false, name: n, file: ok => ok(new File(["x"], n)) });
const dir = { isFile: false, isDirectory: true, name: "lab", createReader() { let i = 0; return { readEntries(ok) { ok(i++ ? [] : [file("a.mzML")]); } }; } };
const e = { dataTransfer: { items: [{ kind: "file", webkitGetAsEntry: () => dir }] } };
process.on("unhandledRejection", () => { console.log(JSON.stringify({ unhandled: true })); process.exit(0); });
let done = false;
c.fromDrop(e, () => { done = true; });
setTimeout(() => { console.log(JSON.stringify({ done, note: els.cartinfo.textContent !== "" })); process.exit(0); }, 100);
""" % str(WEB / "cartella-carica.js")
    assert node(tmp_path, code) == {"done": True, "note": True}


def test_f7_a_failed_script_is_loaded_again_by_a_later_call(tmp_path):
    code = """
const added = [], timers = [];
let fail = true;
const body = { appendChild(s) { added.push(s); setImmediate(() => (fail ? s.onerror() : s.onload())); } };
global.window = { QQQ_PHONE: false };
global.document = { body, createElement: () => ({ parentNode: null }), addEventListener() {} };
global.localStorage = { getItem: () => null }; global.setTimeout = (f) => { timers.push(f); return 0; };
const src = require("fs").readFileSync(%r, "utf8").replace(/var L = \\[[^\\]]*\\];/, "var L = [];");
eval(src);
(async () => {
  const first = await window.carica("x.js").then(() => "ok", () => "fail");
  fail = false;
  const second = await window.carica("x.js").then(() => "ok", () => "fail");
  console.log(JSON.stringify({ first, second, scripts: added.filter(s => s.src === "static/x.js").length }));
  process.exit(0);
})();
""" % str(WEB / "carica.js")
    assert node(tmp_path, code) == {"first": "fail", "second": "ok", "scripts": 2}


BROWSER_HARNESS = """
const fs = require("fs");
const src = fs.readFileSync(%(src)r, "utf8");
const run = (extra) => {
  const g = Object.assign({ window: {}, timers: [], reloads: 0 }, extra);
  const document = { currentScript: { src: %(base)r + "browser.js" }, readyState: "complete", addEventListener() {}, getElementById: () => null, body: {} };
  const navigator = Object.assign({ serviceWorker: { register: () => Promise.resolve(), ready: Promise.resolve(), controller: {}, addEventListener() {} } }, g.navigator);
  const window = Object.assign(g.window, { addEventListener() {}, Worker: function () {}, indexedDB: {}, Response: globalThis.Response, WebAssembly: globalThis.WebAssembly, Promise, fetch: async () => ({}) });
  const location = { href: "http://x/", search: "", hash: "", reload() { g.reloads++; } };
  const sessionStorage = { getItem: () => null, setItem() {} }, caches = { keys: async () => [] };
  const setTimeoutFake = f => { g.timers.push(f); return 0; };
  const names = ["window", "document", "navigator", "location", "sessionStorage", "caches", "NOMI", "E", "UPBUSY", "PROGRESSO", "CONVROWS", "setTimeout", "history"];
  const vals = [window, document, navigator, location, sessionStorage, caches, { CACHE_APP: "x" }, g.E, g.UPBUSY, g.PROGRESSO, g.CONVROWS, g.fakeTimers ? setTimeoutFake : setTimeout, { replaceState() {} }];
  new Function(...names, src)(...vals);
  return g;
};
"""


def test_e11_the_isolation_reload_waits_for_the_loading(tmp_path):
    code = BROWSER_HARNESS % {"src": str(WEB / "browser.js"), "base": (tmp_path.as_uri() + "/")} + """
(async () => {
  const out = {};
  for (const [k, extra] of Object.entries({
    upbusy: { UPBUSY: 2 },
    progress: { UPBUSY: 0, PROGRESSO: { busy: () => true } },
    rows: { UPBUSY: 0, PROGRESSO: { busy: () => false }, CONVROWS: { rows: [{ state: "wait" }] } },
    quiet: { UPBUSY: 0, PROGRESSO: { busy: () => false }, CONVROWS: { rows: [{ state: "done" }] }, E: { files: [] } } })) {
    const g = run({ ...extra, fakeTimers: true });
    await new Promise(r => setTimeout(r, 30));
    out[k] = g.reloads;
  }
  console.log(JSON.stringify(out)); process.exit(0);
})();
"""
    assert node(tmp_path, code) == {"upbusy": 0, "progress": 0, "rows": 0, "quiet": 1}


def test_f6_the_engine_module_and_the_opfs_count_start_with_the_engine(tmp_path):
    (tmp_path / "rust-bridge.js").write_text("""
globalThis.__order.push("bridge-start");
await Promise.race([globalThis.__lrP, new Promise(r => setTimeout(() => { globalThis.__order.push("bridge-timeout"); r(); }, 600))]);
export function createBridge() { return { start() {}, disabled: false }; }
""")
    (tmp_path / "lr.js").write_text("""
globalThis.__order.push("lr-eval"); globalThis.__lrResolve();
export function createLr() { return { reset: async () => {}, boot: async () => { globalThis.__order.push("boot"); globalThis.__bootResolve(); }, handle: async () => null }; }
""")
    code = BROWSER_HARNESS % {"src": str(WEB / "browser.js"), "base": (tmp_path.as_uri() + "/")} + """
globalThis.__order = [];
globalThis.__lrP = new Promise(r => { globalThis.__lrResolve = r; });
const bootP = new Promise(r => { globalThis.__bootResolve = r; });
const keys = async function* () { __order.push("keys"); await Promise.race([bootP, new Promise(r => setTimeout(() => { __order.push("keys-timeout"); r(); }, 600))]); __order.push("keys-done"); };
const dir = { getDirectoryHandle: async () => dir, keys };
(async () => {
  const g = run({ navigator: { storage: { getDirectory: async () => dir } } });
  const t = Date.now();
  while (!g.window.MZLAB_LR && Date.now() - t < 3000) await new Promise(r => setTimeout(r, 10));
  await new Promise(r => setTimeout(r, 50));
  console.log(JSON.stringify({ order: __order, booted: !!g.window.MZLAB_LR }));
  process.exit(0);
})();
"""
    r = node(tmp_path, code)
    o = r["order"]
    assert r["booted"] and "bridge-timeout" not in o and "keys-timeout" not in o, o
    assert o.index("lr-eval") < o.index("bridge-start") + 3 and o.index("boot") < o.index("keys-done"), o
