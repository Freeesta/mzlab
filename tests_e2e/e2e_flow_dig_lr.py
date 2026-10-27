"""Flow «This browser» tp.dig_lr (flow/lazy/flow-web-tp.js `digInputs`, flow-live-jobs.js `tpDig`): over the five demo full-scan files (unit resolution) the browser makes the JSON of the Dig that
`mzlab-cli flow run` makes of the same plan, byte for byte; a second run takes it from OPFS. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen and python3: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_dig_lr.py"""
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
PARENT = json.dumps({"formula": "C14H13F4N3O2S"})
def sample(n, t):
    return {"composto": "", "concentrazione": None, "file": n + ".mzML", "istd": False, "note": "", "ordine_iniezione": NAMES.index(n) + 1, "replica": 1, "serie": "", "sha256": "", "tempo": t, "tempo_confermato": False,
            "tempo_min": t, "tipo": "t0" if t == 0 else "sample", "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(n, t) for n, t in zip(NAMES, TIMES)], "schema": "campioni/2"}, separators=(",", ":"))
PLAN = {"format": "mzflow/2", "inputs": [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in NAMES] + [{"name": "samples.json", "sha256": "", "type": "samples"}, {"name": "parent.json", "sha256": "", "type": "structure"}],
        "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}},
                  {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
                  {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
                  {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}},
                  {"id": "dig", "step": "tp.dig_lr@1", "in": {"table": "gap.table", "samples": "file:samples.json", "parent": "file:parent.json", "index": ["idx.index"]}}]}

tmp = Path(tempfile.mkdtemp(prefix="flow_dig_lr_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
for n, t in zip(NAMES, TIMES): shutil.copy2(ROOT / "mzlab" / "web" / "esempi" / ("FullScan_t%d.mzML" % t), run / (n + ".mzML"))
(run / "plan.json").write_text(json.dumps(PLAN)); (run / "samples.json").write_text(SAMPLES); (run / "parent.json").write_text(PARENT)
subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "plan.json"), "--in", str(run), "--out", str(run / "out")], cwd=ROOT, check=True, capture_output=True)
print("native outputs:", sorted(x.name for x in (run / "out").iterdir() if not x.name.endswith(".mzidx")))
data = site / "static" / "dati_flow_dig"; data.mkdir()
for n in NAMES: shutil.copy2(run / "out" / ("idx-" + n + ".mzidx"), data / (n + ".mzidx"))

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_dig/" + n + ".mzidx")).body.pipeTo(w); }
}"""
RUN = """async ([plan, samples, parent]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['samples.json'] = samples;
  await FLOW.parentSet(S, S.plan.inputs.find(i => i.name === 'parent.json'), parent);
  document.addEventListener('flow:web-step', on);
  try { await FLOW.runners.web.run(S); return { ev }; }
  catch (e) { return { ev, err: { key: e.key, params: e.params, msg: String(e && e.message || e) } }; }
  finally { document.removeEventListener('flow:web-step', on); }
}"""
# the texts the run kept in OPFS (mzlab/flow/msn/<key>.json = {name, text}), by name
KEPT = """async () => {
  const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow'), msn = await d.getDirectoryHandle('msn'), out = {};
  for await (const [n, h] of msn.entries()) if (n !== 'index.json') { const j = JSON.parse(await (await h.getFile()).text()); out[j.name] = j.text; }
  return out;
}"""
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
            r = pg.evaluate(RUN, [PLAN, SAMPLES, PARENT]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            assert any(e["passo"] == "dig" and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), r["ev"][-6:]
            kept = pg.evaluate(KEPT)
            assert "dig.dig.json" in kept, list(kept)
            nat = (run / "out" / "dig.json").read_text()
            web = kept["dig.dig.json"]
            at = next((i for i, (x, y) in enumerate(zip(nat, web)) if x != y), min(len(nat), len(web)))
            assert nat == web, ("first difference at", at, len(nat), len(web), nat[max(0, at - 150):at + 150], web[max(0, at - 150):at + 150])
        step("tp.dig_lr in the browser gives the JSON of the native flow run", first)

        def again():
            r = pg.evaluate(RUN, [PLAN, SAMPLES, PARENT]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] == "dig" and e["stato"] == "running"], r["ev"]
        step("a second run takes the JSON from OPFS", again)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
