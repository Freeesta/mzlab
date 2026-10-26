"""Cache of the detections of mzFinder in the browser (TP_Mine/files/feat-run.js over feat-worker.js): the second run of the same files and recipe reads every .mzfeatf from
`mzlab/feat/cache/` (`timings` row «detect» `cached` = files) and gives the same .mzfeat byte for byte; another detection number (`min_height`) detects again; a cache entry of a newer
format is neither used nor touched; the folder is trimmed to its cap (oldest first) and «Clear cache» empties it. Synthetic series `piccolo`; needs cargo, the wasm32 target and wasm-bindgen."""
import os, shutil, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT
from feat_site import sh, build_wasm, serve

errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600])); errs.append(name)

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("  ", ("wasm", "SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm")); sys.exit(0)
tmp = Path(tempfile.mkdtemp(prefix="feat_cache_"))
data = tmp / "data"; data.mkdir()
sh(sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(data))
idx = sorted(p.name for p in data.glob("*.mzidx"))
build_wasm(tmp)
srv, URL = serve(tmp)

BOOT = """async ([names]) => {
  const { createFeatPool } = await import("./feat-worker.js");
  const fr = await import("./feat-run.js"); window.RUN = fr.runFeatPipeline; window.CLEAR = fr.clearDetectCache;
  const dir = await (await navigator.storage.getDirectory()).getDirectoryHandle("idx", { create: true });
  for (const n of names) { const w = await (await dir.getFileHandle(n, { create: true })).createWritable(); await (await fetch("data/" + n)).body.pipeTo(w); }
  window.SHA = async b => [...new Uint8Array(await crypto.subtle.digest("SHA-256", b))].map(x => x.toString(16).padStart(2, "0")).join("");
  const u = n => new URL(n, location.href).href;
  window.POOL = createFeatPool({ workerUrl: u("feat-worker.js"), size: 2,
    wasm: { feat: { jsUrl: u("feat/mzlab_feat_wasm.js"), wasmUrl: u("feat/mzlab_feat_wasm_bg.wasm") }, proofs: { jsUrl: u("proofs/mzlab_feat_wasm.js"), wasmUrl: u("proofs/mzlab_feat_wasm_bg.wasm") } } });
  window.CACHE = async () => { const d = await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab").then(m => m.getDirectoryHandle("feat")).then(f => f.getDirectoryHandle("cache")).catch(() => null); const o = []; if (d) for await (const [n, h] of d.entries()) o.push([n, (await h.getFile()).size]); return o.sort(); };
}"""
RUN = """async ([names, recipe]) => {
  const r = await RUN(POOL, { files: names.map(n => ({ name: n, path: "idx/" + n })), recipe, parent: null });
  const stored = []; for (const s of r.stored) stored.push(await SHA(s));
  return { cached: r.timings[0].cached, files: r.timings[0].files, table: await SHA(r.table), stored };
}"""
A, B = '{"polarity":1,"mode":"family"}', '{"polarity":1,"mode":"family","min_height":300000}'
try:
    with sync_playwright() as p:
        b = p.chromium.launch_persistent_context(str(tmp / "profile"), executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        pg = b.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.goto(URL); pg.evaluate(BOOT, [idx])
        run = lambda rec: pg.evaluate(RUN, [idx, rec])
        R = {}
        def first():
            R["a"] = run(A); assert R["a"]["cached"] == 0 and R["a"]["files"] == len(idx), R["a"]
            R["names"] = [n for n, _ in pg.evaluate("CACHE()")]; assert len(R["names"]) == len(idx), "one entry per file"
        step("the first run detects every file and fills the cache (save off)", first)
        def second():
            r = run(A); assert r["cached"] == len(idx), r
            assert r["table"] == R["a"]["table"] and r["stored"] == R["a"]["stored"], "the .mzfeat and the .mzfeatf are the same bytes"
        step("the second run reads every file from the cache: same bytes", second)
        def other():
            r = run(B); assert r["cached"] == 0, r
            assert len(pg.evaluate("CACHE()")) == 2 * len(idx)
        step("another min_height detects every file again", other)
        def newer():
            p = "mzlab/feat/cache/" + R["names"][0]
            h = pg.evaluate("""async p => { const b = await POOL.fs.read(p); b.set(new TextEncoder().encode("MZFEAT09")); await POOL.fs.write(p, b.slice()); return SHA(b); }""", p)
            r = run(A); assert r["cached"] == len(idx) - 1 and r["table"] == R["a"]["table"], r
            assert pg.evaluate("async p => SHA(await POOL.fs.read(p))", p) == h, "the entry of a newer format was written over"
        step("an entry of a newer format is left as it is", newer)
        def trim():
            names = pg.evaluate("CACHE()"); total = sum(s for _, s in names); cap = total // 2
            pg.evaluate("async c => POOL.fs.trim('mzlab/feat/cache', c)", cap)
            left = pg.evaluate("CACHE()"); assert 0 < len(left) < len(names) and sum(s for _, s in left) <= cap, (names, left)
        step("the folder is trimmed to its cap", trim)
        def clear():
            freed = pg.evaluate("CLEAR(POOL)"); assert freed > 0 and pg.evaluate("CACHE()") == []
            assert run(A)["cached"] == 0
        step("«Clear cache» empties it and the next run detects again", clear)
        step("no page errors", lambda: [(_ for _ in ()).throw(AssertionError(str(errs[:3]))) for _ in [0] if any(isinstance(e, tuple) for e in errs)])
        b.close()
finally:
    srv.shutdown(); shutil.rmtree(tmp, ignore_errors=True)
sys.exit(1 if any(isinstance(e, str) for e in errs) else 0)
