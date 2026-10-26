"""Flow «This browser» with the steps of the transformation products (flow/lazy/flow-web-tp.js, flow-parent.js): tp.generate over a parent written in the plan (a formula, then a SMILES) gives the registry
`ipotesi/1` of the native `flow run` of the same plan, byte for byte, and a second run takes it from OPFS (cached). Builds the site and the .wasm of mzFinder (needs cargo and wasm-bindgen: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_tp_web.py"""
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

FORMULA = "C13H24N4O3S"
SMILES = "CC(C)(C)NCC(O)COc1nsnc1N1CCOCC1"
PLAN = {"format": "mzflow/2", "inputs": [{"name": "parent.txt", "sha256": "", "type": "structure"}, {"name": "s.mzML", "sha256": "", "type": "mzml"}], "steps": [{"id": "g", "step": "tp.generate@1", "in": {"parent": "file:parent.txt"}}]}

tmp = Path(tempfile.mkdtemp(prefix="flow_tp_web_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)

def native(parent):
    """the registry of `mzlab-cli flow run` on the same plan and parent"""
    d = tmp / ("nat_" + str(abs(hash(parent)))); d.mkdir()
    (d / "plan.json").write_text(json.dumps(dict(PLAN, inputs=PLAN["inputs"][:1]))); (d / "parent.txt").write_text(parent)
    subprocess.run(["cargo", "run", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(d / "plan.json"), "--in", str(d / "parent.txt"), "--out", str(d / "out")], cwd=ROOT, check=True, capture_output=True)
    return (d / "out" / "g.json").read_text()

# the scheduler of the live session needs one file open (an index in OPFS), though tp.generate reads only the parent
data = site / "static" / "dati_flow_tp"; data.mkdir()
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
shutil.copy2(sorted(ser.glob("*.mzidx"))[0], data / "s.mzidx")
PUT = """async () => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  const w = await (await d.getFileHandle("s.mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_tp/s.mzidx")).body.pipeTo(w);
}"""
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
URL = f"http://127.0.0.1:{PORT}/"
for _ in range(100):
    try: urllib.request.urlopen(URL, timeout=2); break
    except OSError: threading.Event().wait(0.2)
# sets the parent text of the plan, runs it and gives the events and the text of the step (or the error key)
RUN = """async ([plan, text]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail), keep = [S.plan.inputs, S.plan.steps];
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  await FLOW.parentSet(S, S.plan.inputs[0], text);
  document.addEventListener('flow:web-step', on);
  try { const r = await FLOW.runners.web.run(S); return { ev, text: (r.files.find(f => f.id === 'g') || {}).text }; }
  catch (e) { return { ev, err: { key: e.key, params: e.params } }; }
  finally { document.removeEventListener('flow:web-step', on); [S.plan.inputs, S.plan.steps] = keep; }
}"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.evaluate(PUT)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        def same_as_native(parent):
            def check():
                r = pg.evaluate(RUN, [PLAN, parent]); assert not r.get("err"), r
                assert any(e["passo"] == "g" and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), r["ev"]
                assert r["text"] == native(parent), (r["text"][:300], parent)
            return check
        step("tp.generate of a formula gives the registry of the native run", same_as_native(FORMULA))
        step("tp.generate of a SMILES (its groups choose the rules) gives the registry of the native run", same_as_native(SMILES))

        def cached():
            r = pg.evaluate(RUN, [PLAN, SMILES]); assert not r.get("err"), r
            assert any(e["passo"] == "g" and e["stato"] == "reused" for e in r["ev"]), r["ev"]
        step("a second run takes the registry from OPFS", cached)

        def no_parent():
            r = pg.evaluate(RUN, [PLAN, ""]); assert r["err"]["key"] == "err.flow.tp.parent", r
        step("a plan without a parent says so", no_parent)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
