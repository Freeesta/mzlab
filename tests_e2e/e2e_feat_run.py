"""Run of the whole pipeline of mzFinder in the browser (TP_Mine/files/feat-run.js over feat-worker.js): detect (L1-L3) in parallel, align (L4-L6), proofs (L7-L9), save to OPFS.
The bytes are the native ones (SHA-256) and a table of times per phase is printed. By default the synthetic series `piccolo`; with MZLAB_FEAT_MZIDX=<folder of .mzidx files>
(and MZLAB_FEAT_PARENT=FORMULA:MZ:RT for the proofs) the files of that folder, and then the limits of the delivery apply: whole run under 60 s, about 3 s per file of ~80 MB
for detect (a guard at 8 s), at most 128 MB of .wasm memory per worker. Needs cargo, the wasm32 target, wasm-bindgen and numpy."""
import json, os, shutil, sys, tempfile
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
tmp = Path(tempfile.mkdtemp(prefix="feat_run_"))
data = tmp / "data"; data.mkdir()
real = os.environ.get("MZLAB_FEAT_MZIDX")
if real:
    for p in sorted(Path(real).glob("*.mzidx")): (data / p.name).symlink_to(p.resolve())
    fm, mz, rt = (os.environ.get("MZLAB_FEAT_PARENT") or "::").split(":")
else:
    sh(sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(data))
    fm, mz, rt = "C21H32N2O2", "345.253655", "1.0"
idx = sorted(p.name for p in data.glob("*.mzidx"))
shutil.copy(ROOT / "TP_Mine/data/regole_tp.json", data / "regole_tp.json")
nat = tmp / "nat"; nat.mkdir()
RECIPE = os.environ.get("MZLAB_FEAT_RECIPE") or '{"polarity":1,"mode":"family"}'
args = ["--recipe", RECIPE] + (["--parent", fm, "--rules", str(data / "regole_tp.json"), "--parent-mz", mz, "--parent-rt", rt] if fm else [])
out = sh("cargo", "run", "--release", "-q", "-p", "mzlab-feat-wasm", "--example", "nativo", "--", str(nat), *[str(data / n) for n in idx], *args)
native = dict(l.split(" ")[:2] for l in out.splitlines() if len(l.split(" ")) == 2 and len(l.split(" ")[1]) == 64)
sizes = build_wasm(tmp)
srv, URL = serve(tmp)

BOOT = """async ([names, size]) => {
  const { createFeatPool } = await import("./feat-worker.js");
  window.RUN = (await import("./feat-run.js")).runFeatPipeline;
  const dir = await (await navigator.storage.getDirectory()).getDirectoryHandle("idx", { create: true });
  for (const n of names) { const w = await (await dir.getFileHandle(n, { create: true })).createWritable(); await (await fetch("data/" + n)).body.pipeTo(w); }
  const sha = async b => [...new Uint8Array(await crypto.subtle.digest("SHA-256", b))].map(x => x.toString(16).padStart(2, "0")).join("");
  window.SHA = sha; window.NOTES = [];
  const u = n => new URL(n, location.href).href;
  window.POOL = createFeatPool({ workerUrl: u("feat-worker.js"), size: size || Math.max(1, Math.min((navigator.hardwareConcurrency || 2) - 1, 4)),
    wasm: { feat: { jsUrl: u("feat/mzlab_feat_wasm.js"), wasmUrl: u("feat/mzlab_feat_wasm_bg.wasm") }, proofs: { jsUrl: u("proofs/mzlab_feat_wasm.js"), wasmUrl: u("proofs/mzlab_feat_wasm_bg.wasm") } },
    notify: k => window.NOTES.push(k) });
  return { workers: window.POOL.workers, cores: navigator.hardwareConcurrency };
}"""
RUN = """async ([names, recipe, parent]) => {
  const files = names.map(n => ({ name: n, path: "idx/" + n }));
  let par = null;
  if (parent.formula) {
    const rules = new Uint8Array(await (await fetch("data/regole_tp.json")).arrayBuffer());
    const minutes = names.map(n => { const m = /_t(\\d+)min/.exec(n); return m ? +m[1] : null; });
    par = { rules, parent: parent.formula, parentRt: parent.rt, parentMz: parent.mz, minutes };
  }
  const r = await RUN(POOL, { files, recipe, parent: par, save: true });
  const res = {}; for (let i = 0; i < names.length; i++) res[names[i] + ".mzfeatf"] = await SHA(r.stored[i]);
  res["series.mzfeat"] = await SHA(r.table); if (r.evid) res["series_evid.mzfeat"] = await SHA(r.evid);
  const back = await POOL.fs.read("mzlab/feat/series.mzfeat");
  return { res, timings: r.timings, totalMs: r.totalMs, tried: r.tried, found: r.found, saved: await SHA(back) === await SHA(r.evid || r.table) };
}"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch_persistent_context(str(tmp / "profile"), executable_path=os.environ.get("MZLAB_CHROMIUM") or None)   # a persistent profile: the quota of OPFS of a private window is too small for ~1 GB of .mzidx
        pg = b.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.on("console", lambda m: errs.append(("console." + m.type, m.text)) if m.type == "error" and "404" not in m.text else None)
        pg.goto(URL)
        info = pg.evaluate(BOOT, [idx, int(os.environ.get("MZLAB_FEAT_WORKERS") or 0)]); print("   cores:", info["cores"])
        R = {}
        def run():
            R.update(pg.evaluate(RUN, [idx, RECIPE, {"formula": fm, "mz": float(mz) if mz else None, "rt": float(rt) if rt else None}]))
            for k, v in R["res"].items(): assert v == native[k], (k, v, native[k])
            assert R["saved"], "the .mzfeat read back from OPFS differs"
        step("the run from the click to the table: every file is the native one", run)
        if R:
            print("\n   | phase | ms | detail |\n   |---|---|---|")
            for t in R["timings"]:
                d = ""
                if t["phase"] == "detect": d = f"{t['files']} files in parallel, per file max {max(t['perFileMs'])} ms, peak {t['peakMB']:.0f} MB"
                if t["phase"] == "align": d = f"peak {t['peakMB']:.0f} MB"
                if t["phase"] == "proofs": d = f"hypotheses {t['hypothesesMs']} ms, {R['tried']} tried, {R['found']} found, peak {t['peakMB']:.0f} MB"
                print(f"   | {t['phase']} | {t['ms']} | {d} |")
            print(f"   | total | {R['totalMs']} | |\n")
            if real:
                def limits():
                    d = R["timings"][0]; assert R["totalMs"] < 60000, R["totalMs"]
                    assert max(d["perFileMs"]) <= 8000, d["perFileMs"]   # ~3 s alone on a 2.1 GHz core (L1 1 s, the rest of detect 2 s), up to 6 s with 4 workers on 4 cores
                    assert d["peakMB"] <= 128 and max(t.get("peakMB", 0) for t in R["timings"]) <= 256, R["timings"]   # align and proofs hold every file's features: see mzfeat-wasm.md
                step("whole run under 60 s, detect at most 128 MB per worker", limits)
        def alive():
            assert pg.evaluate("1 + 1") == 2
        step("the page is alive", alive)
        step("no page errors", lambda: [(_ for _ in ()).throw(AssertionError(str(errs[:3]))) for _ in [0] if errs])
        b.close()
finally:
    srv.shutdown(); shutil.rmtree(tmp, ignore_errors=True)
