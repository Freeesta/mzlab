"""Flow «This browser» runs spectra.lookup@1 (`flow_lookup` of the `lib` .wasm): a synthetic three-compound MSP library is the file of the plan, and the plan (by name, whole name, formula cut by `top_n`,
precursor m/z in a 0.5 ppm and in a 10 ppm window) goes to `mzlab-cli flow run`; the MGF the browser keeps is the native file byte for byte (same SHA-256); every job is `done` with `via: "browser"`,
a second run takes the texts from OPFS, no request is left unserved (`MZLAB_LR_MISS`). Synthetic data only.
Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen, python3: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_web_lookup.py"""
# covers: flow/lazy/flow-web-ident.js, crates/mzlab-lib/src/flowmatch/lookup.rs, crates/mzlab-lib-wasm/src/lib.rs
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

LIB = ("Name: Alpha\nPrecursorMZ: 300.1234\nPrecursor_type: [M+H]+\nIon_mode: Positive\nFormula: C10H10O\nNum Peaks: 5\n100.05 400\n120.08 1000\n150.1 700\n180.12 300\n210.15 500\n\n"
       "Name: Beta\nPrecursorMZ: 300.124\nPrecursor_type: [M+H]+\nIon_mode: Positive\nFormula: C10H10O\nNum Peaks: 5\n90.03 800\n130.06 1000\n160.09 300\n190.11 600\n230.14 200\n\n"
       "Name: Gamma\nPrecursorMZ: 400.1234\nPrecursor_type: [M+H]+\nIon_mode: Positive\nFormula: C10H10O\nNum Peaks: 5\n100.05 300\n120.08 600\n200.12 1000\n230.15 900\n260.18 400\n\n")
FILES = {"lib.msp": LIB}
CASES = {"nam": {"by": "name", "value": "ALP"}, "exa": {"by": "name", "value": "alpha", "exact": True}, "frm": {"by": "formula", "value": "C10H10O", "top_n": 2},
         "mz1": {"by": "precursor_mz", "value": "300.1234", "ppm": 0.5}, "mz10": {"by": "precursor_mz", "value": "300.1234", "ppm": 10}}
IDS = tuple(CASES)

def plan():
    inputs = [{"name": "lib.msp", "sha256": "", "type": "library"}]
    st = [{"id": i, "step": "spectra.lookup@1", "in": {"library": "file:lib.msp"}, "params": c} for i, c in CASES.items()]
    return {"format": "mzflow/2", "inputs": inputs, "steps": st}

tmp = Path(tempfile.mkdtemp(prefix="flow_lookup_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
for n, t in FILES.items(): (run / n).write_text(t)
(run / "lookup.json").write_text(json.dumps(plan()))
out = run / "out"
nat_run = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "lookup.json"), "--in", str(run), "--out", str(out)], cwd=ROOT, capture_output=True, text=True)
assert nat_run.returncode == 0, nat_run.stderr[-1500:]
NATIVE = {i: (out / (i + ".mgf")).read_bytes() for i in IDS}

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
                web = kept.get(i + ".mgf"); assert web is not None, (i, list(kept))
                nat = NATIVE[i].decode()
                at = next((j for j, (x, y) in enumerate(zip(nat, web)) if x != y), 0)
                assert nat == web and len(web) > 50, (i, "first difference at", at, nat[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
                assert hashlib.sha256(web.encode()).hexdigest() == hashlib.sha256(NATIVE[i]).hexdigest(), i
                assert web.count("BEGIN IONS") == {"nam": 1, "exa": 1, "frm": 2, "mz1": 1, "mz10": 2}[i], (i, web[:300])
        step("the browser gives the MGF of the native flow run, byte for byte (name, whole name, formula with top_n, precursor m/z in two windows)", first)

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
