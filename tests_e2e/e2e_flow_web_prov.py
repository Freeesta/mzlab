"""Flow «This browser» prov/1 and corsa/1 (flow-web-prov.js, `record_prov` and `record_corsa` of the engine, `flow_job_meta`): over the five demo full-scan files the browser writes the record of the run of the base chain (index, detection, alignment, gap filling)
that `mzlab-cli flow run` writes of the same plan, apart from the declared fields (times, memory peak, who ran it, reused or not); the files are kept in OPFS and the Report view opens the corsa/1. Builds the site and the .wasm of mzFinder (needs cargo,
wasm-bindgen and python3: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_web_prov.py"""
import json, os, shutil, subprocess, sys, tempfile, threading, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)

# the demo files under names that sort like the plan: the browser lists the indexes of OPFS by name, `flow run` takes the order of the plan
NAMES = ["FullScan_t00", "FullScan_t05", "FullScan_t10", "FullScan_t15", "FullScan_t30"]
TIMES = [0, 5, 10, 15, 30]
PLAN = {"format": "mzflow/2", "inputs": [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in NAMES],
        "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}},
                  {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
                  {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
                  {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}}]}

tmp = Path(tempfile.mkdtemp(prefix="flow_web_export_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
for n, t in zip(NAMES, TIMES): shutil.copy2(ROOT / "mzlab" / "web" / "esempi" / ("FullScan_t%d.mzML" % t), run / (n + ".mzML"))
(run / "plan.json").write_text(json.dumps(PLAN))
subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "plan.json"), "--in", str(run), "--out", str(run / "out"), "--budget-mb", "192"], cwd=ROOT, check=True, capture_output=True)
print("native outputs:", sorted(x.name for x in (run / "out").iterdir() if not x.name.endswith(".mzidx")))
data = site / "static" / "dati_flow_prov"; data.mkdir()
for n in NAMES: shutil.copy2(run / "out" / ("idx-" + n + ".mzidx"), data / (n + ".mzidx"))

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_prov/" + n + ".mzidx")).body.pipeTo(w); }
}"""
RUN = """async (plan) => {
  const S = document.querySelector('.flx-list').__flow;
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  try { const r = await FLOW.runners.web.run(S); return { record: r.record }; }
  catch (e) { return { err: { key: e.key, params: e.params, msg: String(e && e.message || e) } }; }
}"""
KEPT = """async () => {
  const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow'), out = {};
  for (const n of ['prov.json', 'corsa.json']) out[n] = await (await (await d.getFileHandle(n)).getFile()).text();
  out.alone = await (async () => { try { await d.getFileHandle('alone.mzfeat'); return true; } catch (_) { return false; } })();
  out.report = await (await FLOW.lastRunReport()).text();
  return out;
}"""
# the fields of a record that differ by nature between the two runs: times, the memory peak, who ran it and whether the step was reused
BLANK = {"ms", "total_ms", "heap_peak_bytes", "cached", "reused"}
def blank(v):
    if isinstance(v, dict): return {k: (None if k in BLANK else blank(x)) for k, x in v.items()}
    if isinstance(v, list): return [blank(x) for x in v]
    return v
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
URL = f"http://127.0.0.1:{PORT}/"
for _ in range(100):
    try: urllib.request.urlopen(URL, timeout=2); break
    except OSError: threading.Event().wait(0.2)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.evaluate(PUT, NAMES)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        def first():
            r = pg.evaluate(RUN, PLAN); assert not r.get("err"), r["err"]
            assert r["record"], "the run gave no record"
            kept = pg.evaluate(KEPT)
            assert not kept["alone"], "the table of the alignment alone stayed in OPFS"
            for name, nat_name in (("prov.json", "prov.json"), ("corsa.json", "corsa.json")):
                nat = json.loads((run / "out" / nat_name).read_text()); web = json.loads(kept[name])
                assert blank(nat) == blank(web), (name, json.dumps(blank(nat), sort_keys=True)[:1500], json.dumps(blank(web), sort_keys=True)[:1500])
            assert kept["report"] == kept["corsa.json"] and r["record"]["corsa"] == kept["corsa.json"]
        step("the prov/1 and the corsa/1 of the browser are the records of the native flow run apart from the declared fields", first)

        def again():
            r = pg.evaluate(RUN, PLAN); assert not r.get("err"), r["err"]
            web = json.loads(r["record"]["prov"]); nat = json.loads((run / "out" / "prov.json").read_text())
            assert blank(web) == blank(nat), "the record of a run that reuses everything differs"
        step("a second run, which reuses every step, gives the same record", again)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
