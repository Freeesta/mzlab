"""Flow «This browser» runs rt.predict@1 (`flow_rtpredict` of the `attributio-all` .wasm): synthetic alcohols (RT grows by 0.5 min per carbon) are the standards (CSV smiles,rt), three hits (ethanol, decanol and an
unreadable SMILES) the candidates/1, and q.csv the observed RT of the query. The plan (window with defaults; window with `order` residual; forest) goes to `mzlab-cli flow run`, and the candidates/1 text the browser keeps is
the native file byte for byte (same SHA-256); every job is `done` with `via: "browser"`, a second run takes the texts from OPFS, no request is left unserved (`MZLAB_LR_MISS`). Synthetic data only.
Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen, python3: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_web_rtpredict.py"""
# covers: flow/lazy/flow-web-rtpredict.js, crates/mzlab-feat/src/flow_rtpredict.rs, crates/mzlab-attributio-wasm/src/flow_rtpredict.rs
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
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)

STANDARDS = "smiles,rt\n" + "".join(f"{'C' * n}O,{1.0 + 0.5 * n + (0.02 if n % 2 == 0 else -0.02)}\n" for n in range(1, 21))
HITS = ('{"format":"candidates/1","hits":[{"query":"q0","id":"far","name":"Ethanol","smiles":"CCO","score":0.9},'
        '{"query":"q0","id":"near","name":"Decanol","smiles":"CCCCCCCCCCO","score":0.5},{"query":"q0","id":"bad","name":"Unreadable","smiles":"C((","score":0.1}]}')
QUERIES = "query,rt\nq0,6.0\n"
FILES = {"hits.json": HITS, "standards.csv": STANDARDS, "q.csv": QUERIES}
IDS = ("win", "ord", "for")

def plan():
    inputs = [{"name": "hits.json", "sha256": "", "type": "candidates"}, {"name": "standards.csv", "sha256": "", "type": "table"}, {"name": "q.csv", "sha256": "", "type": "table"}]
    ins = {"candidates": "file:hits.json", "standards": "file:standards.csv", "queries": "file:q.csv"}
    st = [{"id": "win", "step": "rt.predict@1", "algorithm": "logp_window", "in": ins},
          {"id": "ord", "step": "rt.predict@1", "algorithm": "logp_window", "in": ins, "params": {"order": "residual"}},
          {"id": "for", "step": "rt.predict@1", "algorithm": "forest", "in": ins}]
    return {"format": "mzflow/2", "inputs": inputs, "steps": st}

tmp = Path(tempfile.mkdtemp(prefix="flow_rtpredict_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
for n, t in FILES.items(): (run / n).write_text(t)
(run / "rtpredict.json").write_text(json.dumps(plan()))
out = run / "out"
nat_run = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "rtpredict.json"), "--in", str(run), "--out", str(out)], cwd=ROOT, capture_output=True, text=True)
assert nat_run.returncode == 0, nat_run.stderr[-1500:]
NATIVE = {i: (out / (i + ".json")).read_bytes() for i in IDS}

# sets the plan and the files of the plan in the open Flow page, runs it, resolves with the events and the error key
RUN = """async ([plan, files]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = Object.assign(S.samplesJson || {}, files);
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
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
        def first():
            r = pg.evaluate(RUN, [plan(), FILES]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in IDS: assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            kept = pg.evaluate(KEPT)
            for i in IDS:
                web = kept.get(i + ".json"); assert web is not None, (i, list(kept))
                nat = NATIVE[i].decode()
                at = next((j for j, (x, y) in enumerate(zip(nat, web)) if x != y), 0)
                assert nat == web and len(web) > 50, (i, "first difference at", at, nat[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
                assert hashlib.sha256(web.encode()).hexdigest() == hashlib.sha256(NATIVE[i]).hexdigest(), i
                d = json.loads(web); assert d["format"] == "candidates/1" and d["step"] == "rt.predict@1", (i, web[:200])
        step("the browser gives the candidates/1 of the native flow run, byte for byte (window, window ordered by residual, forest)", first)

        def again():
            r = pg.evaluate(RUN, [plan(), FILES]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] in IDS and e["stato"] == "running"], r["ev"]
            for i in IDS: assert any(e["passo"] == i and e["stato"] in ("done", "reused") and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
        step("a second run takes the texts from OPFS", again)

        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
