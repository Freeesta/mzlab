"""Pipeline of mzFinder in the browser (crates/mzlab-feat-wasm, TP_Mine/files/feat-worker.js): the pool of workers reads .mzidx files from OPFS, detects, aligns and builds
the graph of the proofs, and the bytes are the ones of the native engine (SHA-256). A worker that dies (killed from outside, or a trap of the .wasm) is replaced, the job
runs again and the page stays alive; the files written in OPFS have no `.part` left; each .wasm is under 450 KB when wasm-opt is there (the CI builds and checks it).
No site is needed: a folder with the worker, the two .wasm and the synthetic series is served by itself. Needs cargo, the wasm32 target, wasm-bindgen and numpy."""
import os, shutil, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT
from feat_site import sh, build_wasm, serve

errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("  ", ("wasm", "SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm")); sys.exit(0)
tmp = Path(tempfile.mkdtemp(prefix="feat_worker_"))

# the series, the native reference and the two .wasm
data = tmp / "data"; data.mkdir()
sh(sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(data))
idx = sorted(p.name for p in data.glob("*.mzidx"))
shutil.copy(ROOT / "TP_Mine/data/regole_tp.json", data / "regole_tp.json")
nat = tmp / "nat"; nat.mkdir()
PARENT = ["--parent", "C21H32N2O2", "--rules", str(data / "regole_tp.json"), "--parent-mz", "345.253655", "--parent-rt", "1.0"]
native = dict(l.split(" ")[:2] for l in sh("cargo", "run", "--release", "-q", "-p", "mzlab-feat-wasm", "--example", "nativo", "--", str(nat), *[str(data / n) for n in idx], "--recipe", '{"polarity":1,"mode":"family"}', *PARENT).splitlines() if l.endswith(tuple("0123456789abcdef")) and " " in l and "hypotheses" not in l)
sizes = build_wasm(tmp)
srv, URL = serve(tmp)

BOOT = """async ([names, size]) => {
  const { createFeatPool } = await import("./feat-worker.js");
  const dir = await (await navigator.storage.getDirectory()).getDirectoryHandle("idx", { create: true });
  for (const n of names) { const w = await (await dir.getFileHandle(n, { create: true })).createWritable(); await w.write(await (await fetch("data/" + n)).arrayBuffer()); await w.close(); }
  const sha = async b => [...new Uint8Array(await crypto.subtle.digest("SHA-256", b))].map(x => x.toString(16).padStart(2, "0")).join("");
  window.SHA = sha; window.NOTES = []; window.PROG = [];
  const w = (name) => { const u = new URL(name, location.href).href; return u; };
  window.POOL = createFeatPool({ workerUrl: w("feat-worker.js"), size,
    wasm: { feat: { jsUrl: w("feat/mzlab_feat_wasm.js"), wasmUrl: w("feat/mzlab_feat_wasm_bg.wasm") }, proofs: { jsUrl: w("proofs/mzlab_feat_wasm.js"), wasmUrl: w("proofs/mzlab_feat_wasm_bg.wasm") } },
    notify: (k, p) => window.NOTES.push(k), onProgress: p => window.PROG.push(p) });
  window.STORED = null;
  return true;
}"""
RECIPE = '{"polarity":1,"mode":"family"}'
DETECT = """async ([names, recipe]) => {
  const t0 = performance.now();
  const files = names.map(n => ({ name: n, path: "idx/" + n }));
  const out = await POOL.detectAll(files, recipe);
  window.STORED = out;
  const res = {}; for (let i = 0; i < names.length; i++) res[names[i] + ".mzfeatf"] = await SHA(out[i]);
  return { res, ms: Math.round(performance.now() - t0) };
}"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); pg = b.new_context().new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.on("console", lambda m: errs.append(("console." + m.type, m.text)) if m.type == "error" and "404" not in m.text else None)
        pg.goto(URL)
        pg.evaluate(BOOT, [idx, 3])
        def detect():
            r = pg.evaluate(DETECT, [idx, RECIPE])
            for k, v in r["res"].items(): assert v == native[k], (k, v, native[k])
            print("   detect", len(idx), "files:", r["ms"], "ms; per file", [x["ms"] for x in pg.evaluate("PROG")])
        step("the workers detect the files read from OPFS: the .mzfeatf are the native ones", detect)
        def align():
            t = pg.evaluate("""async ([names, recipe]) => { const t0 = performance.now(); const b = await POOL.align(STORED, names.map(n => "idx/" + n), recipe); window.TABLE = b; return { sha: await SHA(b), ms: Math.round(performance.now() - t0) }; }""", [idx, RECIPE])
            assert t["sha"] == native["series.mzfeat"], (t, native["series.mzfeat"]); print("   align", t["ms"], "ms")
        step("alignment and filling in one worker: the .mzfeat is the native one", align)
        def proofs():
            r = pg.evaluate("""async ([names, recipe]) => { const t0 = performance.now();
              const rules = new Uint8Array(await (await fetch("data/regole_tp.json")).arrayBuffer());
              const minutes = names.map(n => { const m = /_t(\\d+)min/.exec(n); return m ? +m[1] : null; });
              const r = await POOL.proofs({ table: TABLE, stored: STORED, paths: names.map(n => "idx/" + n), rules, parent: "C21H32N2O2", parentRt: 1.0, minutes, parentMz: 345.253655, recipe });
              return { sha: await SHA(r.table), tried: r.tried, found: r.found, ms: Math.round(performance.now() - t0) }; }""", [idx, RECIPE])
            assert r["sha"] == native["series_evid.mzfeat"], (r, native["series_evid.mzfeat"]); print("   proofs", r["ms"], "ms;", r["tried"], "tried,", r["found"], "found")
        step("hypotheses and graph of the proofs: the .mzfeat with the evid section is the native one", proofs)
        def killed():
            pg.evaluate("window.NOTES.length = 0")
            r = pg.evaluate("""async ([names, recipe]) => { const pr = POOL.detectAll(names.map(n => ({ name: n, path: "idx/" + n })), recipe);
              await new Promise(r => setTimeout(r, 150)); POOL._killBusy();
              const out = await pr; const res = {}; for (let i = 0; i < names.length; i++) res[names[i] + ".mzfeatf"] = await SHA(out[i]); return res; }""", [idx, RECIPE])
            for k, v in r.items(): assert v == native[k], k
            assert "tpmine.feat.restarted" in pg.evaluate("window.NOTES"), pg.evaluate("window.NOTES")
            assert pg.evaluate("1 + 1") == 2
        step("a worker killed from outside is replaced, its file runs again, the page is alive", killed)
        def trap():
            pg.evaluate("window.NOTES.length = 0")
            r = pg.evaluate("POOL._die().then(() => 'ok', e => e.key)")
            assert r == "error.featWorker", r
            assert pg.evaluate("window.NOTES").count("tpmine.feat.restarted") == 1
            r = pg.evaluate("""async ([n, recipe]) => SHA(await POOL.detectAll([{ name: n, path: "idx/" + n }], recipe).then(x => x[0]))""", [idx[0], RECIPE])
            assert r == native[idx[0] + ".mzfeatf"]
        step("a trap of the .wasm: the job runs again once, then it is rejected; the pool still works", trap)
        def archive():
            r = pg.evaluate("""async () => { const b = new Uint8Array(100000).map((_, i) => i % 251); await POOL.fs.write("mzlab/feat/a.mzfeatf", b.slice());
              const back = await POOL.fs.read("mzlab/feat/a.mzfeatf"); const same = back.length === b.length && back.every((x, i) => x === b[i]);
              const names = []; for await (const [n] of (await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("feat")).entries()) names.push(n);
              const had = await POOL.fs.exists("mzlab/feat/a.mzfeatf"); await POOL.fs.remove("mzlab/feat/a.mzfeatf"); return { same, names, had, gone: !(await POOL.fs.exists("mzlab/feat/a.mzfeatf")) }; }""")
            assert r == {"same": True, "names": ["a.mzfeatf"], "had": True, "gone": True}, r
        step("the archive worker writes and reads OPFS, no .part is left", archive)
        def cancel():
            r = pg.evaluate("""async ([names, recipe]) => { const pr = POOL.detectAll(names.map(n => ({ name: n, path: "idx/" + n })), recipe).then(() => 'done', e => e.key);
              POOL.cancel(); return pr; }""", [idx, RECIPE])
            assert r in ("tpmine.feat.cancelled", "error.featWorker"), r
            assert pg.evaluate("1 + 1") == 2
        step("cancel stops the workers and rejects the jobs", cancel)
        def size():
            if not sizes: print("   (no wasm-opt here: the size is checked by the CI)"); return
            for k, v in sizes.items(): assert v < 450 * 1024, f"{k} .wasm is {v} bytes"
            print("   .wasm sizes:", sizes)
        step("each .wasm is under 450 KB", size)
        step("no page errors", lambda: [(_ for _ in ()).throw(AssertionError(str(errs[:3]))) for _ in [0] if errs])
        b.close()
finally:
    srv.shutdown(); shutil.rmtree(tmp, ignore_errors=True)
