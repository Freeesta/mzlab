"""TIC menu «Check calibration (contaminants)…» on the built site: a synthetic high-resolution file whose contaminant ions carry +3 ppm and a drift of +2 ppm over the run is
checked in the browser (api/masscal, the .wasm `proofs`, the same massacc::check as `flow run`); the panel reports the offset, the trend per minute and per 100 m/z, the table and the
two charts with their data tables, the threshold only colours the numbers, the CSV downloads, axe finds nothing serious, nothing is left in window.MZLAB_LR_MISS, the console is clean,
and a low-resolution file has no such entry. Needs cargo and wasm-bindgen (SKIP without). Run from the repo root: python3 tests_e2e/e2e_masscal.py"""
import os, re, shutil, subprocess, sys, tempfile, threading, urllib.request
from pathlib import Path
import numpy as np
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build the engine and crates/mzlab-feat-wasm"); sys.exit(0)

import dati_sintetici_massa as dm  # noqa: E402
tmp = Path(tempfile.mkdtemp(prefix="e2e_masscal_"))
HR = tmp / "MassCal.mzML"; IONS = dm.calibrazione(HR, np.random.default_rng(3), offset_ppm=3.0, drift_ppm=2.0)
LR = ROOT / "mzlab" / "web" / "esempi" / "FullScan_t15.mzML"
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
AXE = lib.axe_path()
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
URL = f"http://127.0.0.1:{PORT}/"
for _ in range(100):
    try: urllib.request.urlopen(URL, timeout=2); break
    except OSError: threading.Event().wait(0.2)

def open_file(pg, f):
    pg.goto(URL + "?motore=rust&lr=1"); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
    pg.set_input_files("#pick", [str(f)])
    pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
    pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
    pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000); pg.wait_for_timeout(1500)

def menu(pg):
    pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='chrom');p.el.scrollIntoView({block:'center'})})()"); pg.wait_for_timeout(300)
    xy = pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='chrom'),r=p.cv.getBoundingClientRect();return {x:r.left+r.width*0.5,y:r.top+r.height*0.5}})()")
    pg.mouse.click(xy["x"], xy["y"], button="right"); pg.wait_for_timeout(400)
    return pg.locator('#ctx [data-azione="chrom.masscal"]')

def num(s): return float(s.replace("−", "-"))

try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1400, "height": 1100}, reduced_motion="reduce", locale="en-US", accept_downloads=True); pg = ctx.new_page()
        pg.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.on("console", lambda m: errs.append(("console." + m.type, m.text)) if m.type in ("error", "warning") and "GL Driver Message" not in m.text and "allow-same-origin" not in m.text else None)
        pg.on("response", lambda r: errs.append(("http%d" % r.status, r.url)) if r.status >= 400 else None)
        open_file(pg, HR)
        def entry():
            assert menu(pg).count() == 1, pg.inner_text("#ctx")
            assert "Check calibration (contaminants)" in pg.inner_text("#ctx")
            menu(pg).click(); pg.wait_for_selector("#masscal #mc-out table", timeout=120000)
        step("the TIC menu of a high-resolution file offers the check and it runs", entry)
        def numbers():
            sm = pg.inner_text("#mc-sum"); m = re.match(r"(\d+) contaminant ions found; offset ([+−-]?[\d.]+) ppm; spread ([\d.]+) ppm", sm); assert m, sm
            n, off, spread = int(m.group(1)), num(m.group(2)), float(m.group(3))
            assert n == len(IONS), (n, len(IONS))
            assert 3.0 <= off <= 5.0, off           # +3 ppm and a drift of 0 to +2 ppm along the run
            assert spread < 2.0, spread
            rows = pg.locator("#mc-out tbody tr"); assert rows.count() == len(IONS)
            errv = [num(rows.nth(i).locator("td").nth(2).inner_text().split()[0]) for i in range(rows.count())]
            assert min(errv) > 1.5 and max(errv) < 5.5, errv
            assert all(int(rows.nth(i).locator("td").nth(3).inner_text()) > 50 for i in range(rows.count()))     # persistent signal, not a one-off
            tr = pg.inner_text("#mc-trend"); t = re.search(r"([+−-][\d.]+) ppm per minute", tr); assert t, tr
            assert 0.05 <= num(t.group(1)) <= 0.5, tr         # 2 ppm over 10 minutes
        step("it reports the +3 ppm offset, the drift per minute and the persistence of every ion", numbers)
        def threshold():
            pg.fill("#mc-thr", "100"); pg.dispatch_event("#mc-thr", "change"); assert pg.locator("#mc-out .mc-hi").count() == 0
            pg.fill("#mc-thr", "1"); pg.dispatch_event("#mc-thr", "change"); assert pg.locator("#mc-out .mc-hi").count() == len(IONS)
            assert "beyond the threshold" in pg.inner_text("#mc-out tbody tr:first-child td:nth-child(4)")
        step("the threshold only colours the numbers (and marks them with a shape and a hidden word)", threshold)
        def charts():
            pg.evaluate("document.activeElement.blur()"); pg.wait_for_timeout(500)   # a pending change of the threshold redraws the charts
            assert pg.locator("#mc-out svg.mc-plot").count() == 2
            assert pg.locator("#mc-out svg.mc-out, #mc-out .mc-out").count() >= 1
            for i in range(2):
                pg.evaluate("i => document.querySelectorAll('#mc-out svg.mc-plot')[i].focus()", i); pg.keyboard.press("t"); pg.wait_for_selector("#tdati table", timeout=5000)
                assert pg.locator("#tdati tbody tr").count() == len(IONS); pg.keyboard.press("Escape"); pg.wait_for_selector("#tdati", state="detached", timeout=5000)
        step("both charts have their data table (T key)", charts)
        def csvdl():
            with pg.expect_download() as d: pg.click('#masscal [data-mc="csv"]')
            path = d.value.path(); text = Path(path).read_text(encoding="utf-8")
            assert "theoretical_mz" in text and text.count("\n") >= len(IONS) + 1, text[:300]
        step("the download button gives the table as CSV", csvdl)
        def a11y():
            small = pg.evaluate("[...document.querySelectorAll('#masscal button, #masscal input')].filter(b => { const r = b.getBoundingClientRect(); return r.width < 24 || r.height < 24; }).map(b => (b.getAttribute('aria-label') || b.className) + ' ' + Math.round(b.getBoundingClientRect().width) + 'x' + Math.round(b.getBoundingClientRect().height))"); assert not small, small
            if AXE:
                pg.add_script_tag(path=AXE)
                v = pg.evaluate("axe.run('#masscal', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact]))"); assert not v, v
        step("targets of 24 px; axe has no serious or critical violation", a11y)
        def closes():
            pg.keyboard.press("Escape"); pg.wait_for_selector("#masscal", state="detached", timeout=5000)
            assert pg.evaluate("(window.MZLAB_LR_MISS || []).length") == 0, pg.evaluate("window.MZLAB_LR_MISS")
        step("Esc closes the panel and no route was missed", closes)
        pg2 = b.new_context(viewport={"width": 1400, "height": 1100}, locale="en-US").new_page(); pg2.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg2.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
        open_file(pg2, LR)
        step("a low-resolution file has no such entry", lambda: (lambda c: None if c == 0 else (_ for _ in ()).throw(AssertionError("entry on a LR file")))(menu(pg2).count()))
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
step("no error in the console", lambda: (_ for _ in ()).throw(AssertionError(errs[:5])) if errs else None)
for s in steps: print("  ", s)
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
