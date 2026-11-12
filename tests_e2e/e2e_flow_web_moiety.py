"""Flow «This browser» runs structure.moiety@1 (.wasm `molkit`, `flow_moiety`) over the synthetic candidate table of tests/dati/moiety.json: the CSV the browser keeps for each instance (rows that
contain the moiety, rows that lack it) is the file of `mzlab-cli flow run` of the same plan, byte for byte; `molkit` is fetched only when the step runs; a second run takes the texts from OPFS;
an empty moiety stops the job with error.sub.query. Builds the site and the .wasm modules (needs cargo and wasm-bindgen: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_web_moiety.py"""
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
    print("SKIP cargo and wasm-bindgen are needed to build the .wasm modules"); sys.exit(0)

FX = json.loads((ROOT / "tests" / "dati" / "moiety.json").read_text(encoding="utf-8"))
TABLE = FX["table"]
OUTS = {"mc": "mc.csv", "ml": "ml.csv"}

def plan(moiety=True, query="c1ccccc1O"):
    st = [{"id": "mc", "step": "structure.moiety@1", "in": {"candidates": "file:list.csv"}, "params": {"query": query}},
          {"id": "ml", "step": "structure.moiety@1", "in": {"candidates": "file:list.csv"}, "params": {"query": query, "keep": "lacking"}}] if moiety else []
    return {"format": "mzflow/2", "inputs": [{"name": "list.csv", "sha256": "", "type": "candidates"}], "steps": st}

tmp = Path(tempfile.mkdtemp(prefix="flow_moiety_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
(run / "list.csv").write_text(TABLE, encoding="utf-8")
(run / "plan.json").write_text(json.dumps(plan()))
nat_run = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "plan.json"), "--in", str(run), "--out", str(run / "out")], cwd=ROOT, capture_output=True, text=True)
assert nat_run.returncode == 0, nat_run.stderr[-1500:]
NATIVE = {i: (run / "out" / n).read_text(encoding="utf-8") for i, n in OUTS.items()}

# sets the plan and its file in the open Flow page, runs it, resolves with the events and the error key
RUN = """async ([plan, table]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['list.csv'] = table;
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
errs = []; asked = []
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
        pg.on("request", lambda r: asked.append(r.url) if "/molkit/" in r.url else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        def none():
            r = pg.evaluate(RUN, [plan(False), TABLE]); assert not r.get("err"), r
            assert not asked, ("molkit was fetched by a plan without structure.moiety", asked)
        step("a plan without structure.moiety does not fetch molkit", none)

        def first():
            r = pg.evaluate(RUN, [plan(), TABLE]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in OUTS: assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert any("/molkit/" in u for u in asked), asked
            kept = pg.evaluate(KEPT)
            for i, n in OUTS.items():
                assert kept.get(n) == NATIVE[i], (i, kept.get(n), NATIVE[i])
            assert NATIVE["mc"] == FX["cases"][0]["out"] and NATIVE["ml"] == FX["cases"][1]["out"], NATIVE
        step("the browser gives the CSV of the native flow run for both instances, byte for byte (= the fixture)", first)

        def again():
            before = len(asked)
            r = pg.evaluate(RUN, [plan(), TABLE]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] in OUTS and e["stato"] == "running"], r["ev"]
            assert len(asked) == before, ("the second run fetched molkit again", asked[before:])
        step("a second run takes the texts from OPFS", again)

        def empty():
            r = pg.evaluate(RUN, [plan(query=""), TABLE])
            text = json.dumps(r)
            assert "error.sub.query" in text, text[-1200:]
        step("an empty moiety stops the job with error.sub.query", empty)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
