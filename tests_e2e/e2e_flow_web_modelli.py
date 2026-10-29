"""Flow «This browser» runs the two TP models of the start screen (flow/lazy/flow-fix.js `templatePlan`: «Bentazone / transformation products» and «TP time series») over the synthetic identity series: the plan the page builds goes to `mzlab-cli flow run`, and the texts the browser keeps
(`filtri/1`, `sites/1`, `rete/1` and the view of the reliable rows) are the files of the native run, byte for byte; a second run takes them from OPFS. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen, python3 and numpy: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_web_modelli.py"""
import json, os, shutil, subprocess, sys, tempfile, threading, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

try: import numpy  # noqa: F401
except ImportError: print("SKIP numpy is needed for the synthetic series"); sys.exit(0)
if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)

NAMES = ["SIN_blank", "SIN_dark", "SIN_t0", "SIN_t05min", "SIN_t10min", "SIN_t20min"]
PARENT = json.dumps({"formula": "C22H31N3O4S", "smiles": "CCN(CC)C(=O)c1ccc(cc1)S(=O)(=O)NCCOCCNCc1ccccc1", "rt": 1.0})
def sample(n, i):
    kind, t = ("blank", None) if n.endswith("blank") else ("dark", None) if n.endswith("dark") else ("t0", 0) if n.endswith("t0") else ("sample", {"t05min": 5, "t10min": 10, "t20min": 20}[n.split("_")[1]])
    return {"composto": "", "concentrazione": None, "file": n + ".mzML", "istd": False, "note": "", "ordine_iniezione": i + 1, "replica": 1, "serie": "", "sha256": "", "tempo": t, "tempo_confermato": False,
            "tempo_min": t, "tipo": kind, "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(n, i) for i, n in enumerate(NAMES)], "schema": "campioni/2"}, separators=(",", ":"))
MODELS = ["bentazone", "tpseries"]
tmp = Path(tempfile.mkdtemp(prefix="flow_tp_link_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_sintetici.py", str(ser), "--solo", "identita"], cwd=ROOT, check=True, capture_output=True)
run = tmp / "run"; run.mkdir()
for n in NAMES: shutil.copy2(ser / "identita" / (n + ".mzML"), run / (n + ".mzML"))
(run / "input1.samples").write_text(SAMPLES); (run / "input1.structure").write_text(PARENT)
data = site / "static" / "dati_flow_tp"; data.mkdir()

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_tp/" + n + ".mzidx")).body.pipeTo(w); }
}"""
# the plan of a model as the start screen builds it, with the files of the series as the source of the first step
BUILD = """([id, names]) => {
  const S = document.querySelector('.flx-list').__flow, p = FLOW.exportPlan(FLOW.templatePlan(S.reg, FLOW.TEMPLATES.find(t => t.id === id)));
  const mz = p.inputs.find(i => i.type === 'mzml').name;
  p.inputs = names.map(n => ({ name: n + '.mzML', sha256: '', type: 'mzml' })).concat(p.inputs.filter(i => i.type !== 'mzml'));
  p.steps.find(s => s.in.source && s.in.source[0] === 'file:' + mz).in.source = names.map(n => 'file:' + n + '.mzML');
  return p;
}"""
# sets the plan, the samples and the parent in the open Flow page, runs it, resolves with the events and the error key
RUN = """async ([plan, samples, parent]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['input1.samples'] = samples;
  await FLOW.parentSet(S, S.plan.inputs.find(i => i.name === 'input1.structure'), parent);
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
        b = p.chromium.launch()
        for model in MODELS:
            # a fresh page per model: the texts kept in OPFS are named by the ids of the steps, which repeat from one model to the other
            pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
            pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
            pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
            pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
            pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_function("window.QTOOLS", timeout=120000)
            pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
            pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
            plan = pg.evaluate(BUILD, [model, NAMES])
            out = run / ("out_" + model); (run / (model + ".json")).write_text(json.dumps(plan))
            nat_run = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / (model + ".json")), "--in", str(run), "--out", str(out)], cwd=ROOT, capture_output=True, text=True)
            assert nat_run.returncode == 0, nat_run.stderr[-1500:]
            print(model, "native outputs:", sorted(x.name for x in out.iterdir() if not x.name.endswith(".mzidx")))
            for n in NAMES: shutil.copy2(next(out.glob("*-" + n + ".mzidx")), data / (n + ".mzidx"))
            pg.evaluate(PUT, NAMES)
            ids = [s_["id"] for s_ in plan["steps"] if s_["step"].split("@")[0] in ("tp.filter", "tp.position", "tp.network", "table.filter")]

            def first():
                r = pg.evaluate(RUN, [plan, SAMPLES, PARENT]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
                for i in ids: assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
                kept = pg.evaluate(KEPT)
                for i in ids:
                    nat = [x for x in out.iterdir() if x.name.startswith(i + ".") and not x.name.endswith(".mzidx")]
                    assert len(nat) == 1, (i, nat, list(out.iterdir()))
                    web = next((t for k, t in kept.items() if k.startswith(i + ".")), None)
                    assert web is not None, (i, list(kept))
                    at = next((j for j, (x, y) in enumerate(zip(nat[0].read_text(), web)) if x != y), 0)
                    assert nat[0].read_text() == web and len(web) > 50, (i, "first difference at", at, nat[0].read_text()[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
            step(model + ": the browser gives the texts of the native flow run, byte for byte", first)

            def again():
                r = pg.evaluate(RUN, [plan, SAMPLES, PARENT]); assert not r.get("err"), r
                assert not [e for e in r["ev"] if e["passo"] in ids and e["stato"] == "running"], r["ev"]
            step(model + ": a second run takes the texts from OPFS", again)
            pg.context.close()
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
