"""Flow «This browser» runs chrom.transitions@1 (flow/lazy/flow-web-transitions.js, `flow_transitions` of the `flowtab` .wasm): the two samples of the synthetic SIM .wiff
(tools/wiff_sim.py) converted to mzML by `mzlab-cli convert`, a transition table with two isomers on one ion, a negative ion and a row without RT; the CSV the browser keeps is
the one of `mzlab-cli flow run` byte for byte for the three outputs, the job runs `via: "browser"` and no request is left unserved (`MZLAB_LR_MISS`). Builds the site and the
.wasm of mzFinder (needs cargo, wasm-bindgen and python3: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_transitions.py"""
import hashlib, json, os, shutil, subprocess, sys, tempfile, threading, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build the .wasm"); sys.exit(0)

TABLE = ("name;q1;q3;polarity;rt;rt_window;isomer_group\n"
         "isoA;237.1;;+;2.5;0.3;237\n"
         "isoB;237.1;;+;4.0;;237\n"
         "ionN;235.1;;-;3.0;;\n"
         "unknown;237.1;;+;;;237\n")
NAMES = ["sim-1.mzML", "sim-2.mzML"]
OUTPUTS = ["peaks", "table", "candidates"]

tmp = Path(tempfile.mkdtemp(prefix="flow_transitions_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(tmp / "w")], check=True, capture_output=True)
CLI = ["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--"]
subprocess.run(CLI + ["convert", str(tmp / "w" / "sim.wiff"), "--out", str(run)], cwd=ROOT, check=True, capture_output=True)
assert all((run / n).exists() for n in NAMES), sorted(p.name for p in run.iterdir())

def plan(output):
    return {"format": "mzflow/2", "inputs": [{"name": n, "sha256": "", "type": "mzml"} for n in NAMES],
            "steps": [{"id": "tr", "step": "chrom.transitions@1", "in": {"source": ["file:" + n for n in NAMES]}, "params": {"output": output, "transitions": TABLE}}]}

NAT = {}
for o in OUTPUTS:
    (run / (o + ".json")).write_text(json.dumps(plan(o)))
    subprocess.run(CLI + ["flow", "run", str(run / (o + ".json")), "--in", str(run), "--out", str(run / ("out_" + o))], cwd=ROOT, check=True, capture_output=True)
    NAT[o] = (run / ("out_" + o) / "tr.csv").read_text()
assert NAT["peaks"].count(",ok,") + NAT["peaks"].count(",ok\n") >= 6, NAT["peaks"]
assert len(NAT["candidates"].strip().splitlines()) == 3, NAT["candidates"]
data = site / "static" / "dati_flow_transitions"; data.mkdir()
for n in NAMES: shutil.copy2(run / n, data / n)

HOLD = """async (names) => { for (const name of names) { const b = await (await fetch("static/dati_flow_transitions/" + name)).blob(); window.MZLAB_RUST.noteUpload(name, b); } }"""
RUN = """async (plan) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  document.addEventListener('flow:web-step', on);
  try { await FLOW.runners.web.run(S); return { ev }; }
  catch (e) { return { ev, err: { key: e.key, params: e.params, msg: String(e && e.message || e) } }; }
  finally { document.removeEventListener('flow:web-step', on); }
}"""
KEPT = """async () => {
  const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow'), msn = await d.getDirectoryHandle('msn'), out = {};
  for await (const [n, h] of msn.entries()) if (n !== 'index.json') { const j = JSON.parse(await (await h.getFile()).text()); out[j.name] = j.text; }
  return out;
}"""
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []; asked = []
URL = f"http://127.0.0.1:{PORT}/"
for _ in range(100):
    try: urllib.request.urlopen(URL, timeout=2); break
    except OSError: threading.Event().wait(0.2)
sha = lambda b: hashlib.sha256(b.encode()).hexdigest()
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.on("request", lambda r: asked.append(r.url) if "/flowtab/" in r.url else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.wait_for_function("!!(window.MZLAB_RUST && window.MZLAB_RUST.noteUpload)", timeout=120000)
        pg.evaluate(HOLD, NAMES)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        def same(output):
            def check():
                r = pg.evaluate(RUN, plan(output)); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
                assert any(e["passo"] == "tr" and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), [e for e in r["ev"] if e["passo"] == "tr"][-3:]
                assert asked, "the flowtab module was not fetched when chrom.transitions ran"
                web = pg.evaluate(KEPT).get("tr.csv"); assert web is not None
                assert sha(web) == sha(NAT[output]), (output, NAT[output][:400], web[:400])
            return check
        for o in OUTPUTS: step(f"the browser gives the {o} CSV of the native flow run, byte for byte", same(o))
        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
