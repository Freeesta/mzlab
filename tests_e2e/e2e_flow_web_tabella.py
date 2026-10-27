"""Flow «This browser» table at the end of a run (flow/lazy/flow-run-end.js, flow-results.js): a run in the browser opens series.mzfeat from OPFS as the «Feature table» view under the canvas, with the same rows as
PIPEV.readFeat of the file; after a reload the Results of the step offer «Open series.mzfeat» again; no request the engine cannot serve (MZLAB_LR_MISS empty); axe finds nothing serious in the view.
Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen and numpy: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_web_tabella.py"""
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
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)

tmp = Path(tempfile.mkdtemp(prefix="flow_web_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
data = site / "static" / "dati_flow_web"; data.mkdir()
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
for f in sorted(ser.glob("*.mzidx"))[:3]: shutil.copy2(f, data / f.name)
names = sorted(p.name for p in data.glob("*.mzidx"))

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
URL = f"http://127.0.0.1:{PORT}/"
for _ in range(100):               # the server answers before the page is asked for
    try: urllib.request.urlopen(URL, timeout=2); break
    except OSError: threading.Event().wait(0.2)
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n, { create: true })).createWritable(); await (await fetch("static/dati_flow_web/" + n)).body.pipeTo(w); }
}"""
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n, { create: true })).createWritable(); await (await fetch("static/dati_flow_web/" + n)).body.pipeTo(w); }
}"""
READ = """async () => { const d = await navigator.storage.getDirectory(), f = await (await d.getDirectoryHandle('mzlab')).getDirectoryHandle('flow');
  const b = new Uint8Array(await (await (await f.getFileHandle('series.mzfeat')).getFile()).arrayBuffer()); const x = PIPEV.readFeat(b); return { rows: x.rows, files: x.files }; }"""
def open_flow(pg):
    pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
    pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.add_init_script("window.__fo = []; document.addEventListener('focusin', e => window.__fo.push('in ' + (e.target.title || e.target.className || e.target.tagName) + '@' + Math.round(performance.now())), true); document.addEventListener('focusout', e => window.__fo.push('out ' + (e.target.title || e.target.className || e.target.tagName) + '@' + Math.round(performance.now())), true);")
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.evaluate(PUT, names)
        open_flow(pg)
        state = {}

        def run_opens_table():
            pg.click("#flx-run")
            pg.locator('.fl-meta button', has_text="Run in this browser").click()
            pg.wait_for_selector(".flx-res .flw-feat table", timeout=240000)
            assert pg.locator(".flx-res h3").inner_text().startswith("Feature table: series.mzfeat"), pg.locator(".flx-res h3").inner_text()
            try: pg.wait_for_function("document.activeElement === document.querySelector('.flx-res h3')", timeout=30000)
            except Exception: raise AssertionError("focus is not on the title of the view: " + str(pg.evaluate("window.__fo.slice(-12)")))
            want = pg.evaluate(READ); state["want"] = want; assert want["rows"] > 0, want
            say = pg.locator(".flx-res .flw-feat").evaluate("e => e.parentElement.querySelector('[role=status]').textContent")
            assert say.replace(",", "").startswith(f"{want['rows']} rows in {want['files']} files"), (say, want)
            assert "feature table is open" in pg.inner_text(".fl-meta [role=status]"), pg.inner_text(".fl-meta [role=status]")
        step("a run in the browser opens series.mzfeat as the Feature table with the rows of readFeat", run_opens_table)

        def axe():
            path = lib.axe_path()
            if not path: print("SKIP axe: axe-core not available"); return
            pg.add_script_tag(path=path)
            r = pg.evaluate("axe.run('.flx-res', {runOnly: {type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag22aa']}}).then(x => x.violations.filter(v => ['serious', 'critical'].includes(v.impact)).map(v => v.id + ': ' + v.nodes.length))")
            assert not r, r
        step("axe: no serious violation in the opened view", axe)

        def after_reload():
            pg.reload(); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.QTOOLS", timeout=120000)
            open_flow(pg)
            titles = pg.evaluate("[...document.querySelectorAll('.fln-t')].map(e => e.textContent)")
            assert "Build feature table" in titles, titles
            pg.locator(".fln", has_text="Build feature table").first.click()
            pg.wait_for_timeout(1500)
            assert pg.locator('.fl-results button[aria-label="Open series.mzfeat"]').count(), (pg.evaluate("(async () => { const i = await FLOW.webTableItem(); return String(!!i) })()"), pg.evaluate("(document.querySelector('.fl-results') || {}).outerHTML"))
            pg.locator('.fl-results button[aria-label="Open series.mzfeat"]').click()
            pg.wait_for_selector(".flx-res .flw-feat table", timeout=120000)
            say = pg.locator(".flx-res .flw-feat").evaluate("e => e.parentElement.querySelector('[role=status]').textContent")
            assert say.replace(",", "").startswith(f"{state['want']['rows']} rows in"), say
        step("after a reload the Results of the step open the table kept in OPFS", after_reload)

        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
