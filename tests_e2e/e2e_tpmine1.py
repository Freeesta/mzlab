"""Tabui (internal name TP Mine, also mzFinder) step (a): the tab is in the header at first load, with no flag and no click (tpmine-loader.js), survives a reload.
Five quick clicks on the logo do not change it.
Run from the repo root: python3 tests_e2e/e2e_tpmine1.py   (needs Playwright and a built Pyodide dir or internet)"""
import os, shutil, subprocess, sys, time, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port; PORT = free_port()
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:300]))

tmp = Path(tempfile.mkdtemp())
cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")]
subprocess.run(cmd, check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")          # the engine of the page is Rust: the .wasm goes in the copy of the site
site = tmp / "site"
def packed():
    d = site / "static" / "mzfinder"
    assert (d / "indice.json").exists() and (site / "static" / "tpmine-loader.js").exists()
    assert not (site / "static" / "tpmine.enc").exists()
    assert not (d / "tpmine.zip").exists() and not (d / "pyodide").exists()
step("built site: Tabui in plain text in static/mzfinder/, no encrypted file", packed)

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []; reqs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); TAB = '#nav button[data-v="tpmine"]'
        def fresh():
            c = b.new_context(viewport={"width": 1400, "height": 900}); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("request", lambda r: reqs.append(r.url))
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_timeout(1500)
            return pg
        pg = fresh()
        def visible():
            pg.wait_for_selector(TAB, timeout=30000)                         # in the header at first load, no flag and no click
            assert pg.evaluate("localStorage.getItem('mzlab.admin')") is None
            assert not pg.query_selector("#mzf-badge")           # no mark next to the logo
            pg.wait_for_function("window.QTOOLS.list().some(t => t.id === 'tpmine')", timeout=60000)      # the tab shows at once; the tool registers when its scripts have loaded
            txt = pg.inner_text(TAB)
            assert "Tabui" in txt and "mzFinder" not in txt, txt
            assert [u for u in reqs if u.endswith("/mzfinder/indice.json")]
            pg.click(TAB); pg.wait_for_selector("#tp-go")
            assert not pg.evaluate("document.getElementById('v-draw').offsetParent")
            pg.click('#nav button[data-v="draw"]'); assert pg.evaluate("document.getElementById('v-tpmine').hidden")
            pg.click('#nav button[data-v="data"]')
        step("the Tabui tab is in the header at first load", visible)
        def after_ready():
            # Tabui downloads after the page is usable: nothing of static/mzfinder/ is requested up to qqReady, and the first click on the tab still opens it
            c = b.new_context(viewport={"width": 1400, "height": 900}); q = c.new_page()
            q.add_init_script("""(() => { try { performance.setResourceTimingBufferSize(5000); } catch (_) {} const t = setInterval(() => { if (window.qqReady) { clearInterval(t);
              window.__mzfAtReady = performance.getEntriesByType('resource').filter(e => e.name.includes('/mzfinder/')).length; } }, 5); })()""")
            q.goto(f"http://127.0.0.1:{PORT}/"); q.wait_for_selector("#drop", timeout=120000)
            q.wait_for_function("window.__mzfAtReady !== undefined", timeout=60000)
            n = q.evaluate("window.__mzfAtReady"); assert n == 0, f"{n} mzfinder resources requested before qqReady"
            q.click(TAB); q.wait_for_selector("#tp-go", timeout=60000)
            assert q.evaluate("performance.getEntriesByType('resource').filter(e => e.name.includes('/mzfinder/')).length") > 20
            c.close()
        step("no static/mzfinder/ request before qqReady; the first Tabui click opens it", after_ready)
        def reload_on():
            pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_selector(TAB, timeout=30000)
        step("after a reload it is still there", reload_on)
        def logo_normal():
            pg.wait_for_selector("header img.logo[role=button]", timeout=60000)      # logo-piccone.js arms the logo when its script has run, which can be after #drop
            pg.evaluate("for (let i = 0; i < 5; i++) document.querySelector('header img.logo').click()")
            pg.wait_for_function("document.querySelector('header img.logo').style.opacity !== '0'", timeout=15000)
            assert pg.query_selector(TAB) and pg.evaluate("window.QTOOLS.list().some(t => t.id === 'tpmine')")      # the logo has nothing to do with the tab
        step("five clicks on the logo do not touch the tab", logo_normal)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
