"""Flow «This browser» runs chrom.uv_peaks@1 (flow/lazy/flow-web-uv.js, `flow_uv_peaks` of the `flowtab` .wasm): the example mzML that carries a synthetic UV matrix (mzlab/web/esempi/FullScan_t10.mzML)
is the file of the plan; the CSV the browser keeps is the one of `mzlab-cli flow run` byte for byte for the default trace, for another wavelength and bandwidth, and with the broad bands on; the job runs
`via: "browser"`, past the memory budget the read is refused before the file is decoded (`error.memoryBudget`, phase read) and no request is left unserved. Anonymous example data only.
Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen and python3: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_web_uv.py"""
# covers: flow/lazy/flow-web-uv.js, crates/mzlab-feat/src/uv_text.rs, crates/mzlab-feat-wasm/src/run_tab.rs
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

NAMES = ["FullScan_t10.mzML"]
VARIANTS = {"narrow": {}, "other": {"wavelength_nm": 280, "band_nm": 8}, "broad": {"broad": True, "broad_min_s": 20}}

tmp = Path(tempfile.mkdtemp(prefix="flow_uv_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
shutil.copy2(ROOT / "mzlab" / "web" / "esempi" / NAMES[0], run / NAMES[0])
CLI = ["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--"]
def plan(v, names=NAMES):
    return {"format": "mzflow/2", "inputs": [{"name": n, "sha256": "", "type": "mzml"} for n in names],
            "steps": [{"id": "uv", "step": "chrom.uv_peaks@1", "in": {"source": ["file:" + n for n in names]}, "params": VARIANTS[v]}]}

NAT = {}
for v in VARIANTS:
    (run / (v + ".json")).write_text(json.dumps(plan(v)))
    subprocess.run(CLI + ["flow", "run", str(run / (v + ".json")), "--in", str(run), "--out", str(run / ("out_" + v))], cwd=ROOT, check=True, capture_output=True)
    NAT[v] = (run / ("out_" + v) / "uv.csv").read_text()
assert len(NAT["narrow"].strip().splitlines()) > 1, NAT["narrow"]
assert NAT["narrow"] != NAT["other"], "another wavelength must change the CSV"
data = site / "static" / "dati_flow_uv"; data.mkdir()
for n in NAMES: shutil.copy2(run / n, data / n)
# the page finds a file of a plan through its OPFS index (mzlab/idx/<stem>.mzidx), made here by the native index step
(run / "idx.json").write_text(json.dumps({"format": "mzflow/2", "inputs": [{"name": n, "sha256": "", "type": "mzml"} for n in NAMES],
    "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n for n in NAMES]}}]}))
subprocess.run(CLI + ["flow", "run", str(run / "idx.json"), "--in", str(run), "--out", str(run / "out_idx")], cwd=ROOT, check=True, capture_output=True)
for n in NAMES: shutil.copy2(run / "out_idx" / ("idx-" + n[:-5] + ".mzidx"), data / (n[:-5] + ".mzidx"))

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_uv/" + n + ".mzidx")).body.pipeTo(w); }
}"""
HOLD = """async (names) => { for (const name of names) { const b = await (await fetch("static/dati_flow_uv/" + name)).blob(); window.MZLAB_RUST.noteUpload(name, b); } }"""
RUN = """async (plan) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  document.addEventListener('flow:web-step', on);
  try { await FLOW.runners.web.run(S); return { ev }; }
  catch (e) { return { ev, err: { key: e.key, params: e.params, msg: String(e && e.message || e) } }; }
  finally { document.removeEventListener('flow:web-step', on); }
}"""
KEPT = """async () => {
  const d = await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'), f = await (await d.getDirectoryHandle('flow')).getFileHandle('uv.csv');
  return { "uv.csv": await (await f.getFile()).text() };
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
        pg.evaluate(HOLD, NAMES); pg.evaluate(PUT, [n[:-5] for n in NAMES])
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        def same(v):
            def check():
                r = pg.evaluate(RUN, plan(v)); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
                assert any(e["passo"] == "uv" and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), [e for e in r["ev"] if e["passo"] == "uv"][-3:]
                assert asked, "the flowtab module was not fetched when chrom.uv_peaks ran"
                web = pg.evaluate(KEPT).get("uv.csv"); assert web is not None
                assert sha(web) == sha(NAT[v]), (v, NAT[v][:400], web[:400])
            return check
        for v in VARIANTS: step(f"the browser gives the CSV of the native flow run, byte for byte ({v})", same(v))
        LEVELS = "(high) => { const r = window.MZLAB_RUST; window.__lv = [r.budgetMb, r.choice]; r.budgetMb = 0; r.choice = { ...(r.choice || {}), mb: 8, high_mb: high }; }"
        UNDO = "() => { const r = window.MZLAB_RUST; [r.budgetMb, r.choice] = window.__lv; }"
        def read_refused():
            VARIANTS["refused"] = {"wavelength_nm": 230}   # a plan not run before, so nothing comes from the cache
            pg.evaluate(LEVELS, 8)
            try: r = pg.evaluate(RUN, plan("refused"))
            finally: pg.evaluate(UNDO)
            assert any(e["passo"] == "uv" and e["stato"] in ("error", "refused") for e in r["ev"]), [e for e in r["ev"] if e["passo"] == "uv"][-3:]
            title = pg.evaluate("() => FLOW.liveView.look(FLOW.liveView.node('uv')).title")
            assert ("Reading this mzML file whole would need about" in title or "Leggere per intero questo file mzML" in title) and "8 MB" in title, title
        step("past the budget and the high level, the mzML file is refused before it is read, with the numbers of the read", read_refused)
        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
