"""Flow «This browser» runs features.estimate@1 (`flow_estimate` of the .wasm `hr`): over the synthetic identity series one `estimates/1` per file, named
`<instance>-<root>.json` as `mzlab-cli flow run` names it, is the native file of the same plan byte for byte, with the plan's defaults and with the axis written;
the run gives the texts in the order of the files; a second run takes them from OPFS. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen,
python3 and numpy: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_web_estimate.py"""
import hashlib, json, os, shutil, subprocess, sys, tempfile, threading, urllib.request
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

NAMES = ["SIN_blank", "SIN_t0", "SIN_t10min"]
# instance id -> its parameters: the defaults (the file decides polarity and axis), and the axis written by the plan
INSTANCES = {"est": {}, "esd": {"axis": "Da"}}
OUTS = [f"{i}-{n}.json" for i in INSTANCES for n in NAMES]

def plan():
    inputs = [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in NAMES]
    st = [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}}]
    st += [{"id": i, "step": "features.estimate@1", "in": {"index": ["idx.index"]}, **({"params": p} if p else {})} for i, p in INSTANCES.items()]
    return {"format": "mzflow/2", "inputs": inputs, "steps": st}

tmp = Path(tempfile.mkdtemp(prefix="flow_estimate_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_sintetici.py", str(ser), "--solo", "identita"], cwd=ROOT, check=True, capture_output=True)
run = tmp / "run"; run.mkdir()
for n in NAMES: shutil.copy2(ser / "identita" / (n + ".mzML"), run / (n + ".mzML"))
(run / "plan.json").write_text(json.dumps(plan()))
nat_run = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "plan.json"), "--in", str(run), "--out", str(run / "out")], cwd=ROOT, capture_output=True, text=True)
assert nat_run.returncode == 0, nat_run.stderr[-1500:]
NATIVE = {n: (run / "out" / n).read_bytes() for n in OUTS}
data = site / "static" / "dati_flow_estimate"; data.mkdir()
for n in NAMES: shutil.copy2(run / "out" / ("idx-" + n + ".mzidx"), data / (n + ".mzidx"))

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_estimate/" + n + ".mzidx")).body.pipeTo(w); }
}"""
# sets the plan in the open Flow page, runs it, resolves with the events, the names of the texts the run gives and the error key
RUN = """async (plan) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  document.addEventListener('flow:web-step', on);
  try { const r = await FLOW.runners.web.run(S); return { ev, names: (r.files || []).map(f => f.name) }; }
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
            r = pg.evaluate(RUN, plan()); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in INSTANCES: assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert r["names"] == OUTS, ("the texts of the run, in the order of the steps and of the files", r["names"])
            kept = pg.evaluate(KEPT)
            for n in OUTS:
                web = kept.get(n); assert web is not None, (n, list(kept))
                nat = NATIVE[n].decode()
                at = next((j for j, (x, y) in enumerate(zip(nat, web)) if x != y), min(len(nat), len(web)))
                assert nat == web and len(web) > 50, (n, len(nat), len(web), "first difference at", at, nat[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
                assert hashlib.sha256(web.encode()).hexdigest() == hashlib.sha256(NATIVE[n]).hexdigest(), n
            for n in NAMES:
                d, da = json.loads(kept[f"est-{n}.json"]), json.loads(kept[f"esd-{n}.json"])
                assert d["schema"] == "estimates/1" and d["file"] == n + ".mzML" and "noise" in d, kept[f"est-{n}.json"][:300]
                assert da["axis"] == "Da", ("the axis of the plan did not reach the engine", kept[f"esd-{n}.json"][:300])
        step("the browser gives the estimates/1 of the native flow run, one per file, byte for byte", first)

        def again():
            r = pg.evaluate(RUN, plan()); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] in INSTANCES and e["stato"] == "running"], r["ev"]
            for i in INSTANCES: assert any(e["passo"] == i and e["stato"] in ("done", "reused") and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert r["names"] == OUTS, r["names"]
        step("a second run takes the texts from OPFS", again)

        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
