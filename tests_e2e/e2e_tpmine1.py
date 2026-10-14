"""mzFinder (internal name TP Mine) step (a): the switch. 5 clicks on the header logo turn it on, 5 more turn it off; the state survives a
reload; with mzFinder off not one of its files is downloaded (checked on the network requests).
Run from the repo root: python3 tests_e2e/e2e_tpmine1.py   (needs Playwright and a built Pyodide dir or internet)"""
import os, shutil, subprocess, sys, time, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
PORT = 8841
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:300]))

tmp = Path(tempfile.mkdtemp())
cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")]
if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
subprocess.run(cmd, check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")          # the engine of the page is Rust: the .wasm goes in the copy of the site
site = tmp / "site"
def packed():
    d = site / "static" / "mzfinder"
    assert (d / "indice.json").exists() and (d / "tpmine.zip").exists() and (site / "static" / "tpmine-loader.js").exists()
    assert not (site / "static" / "tpmine.enc").exists()
step("built site: mzFinder in plain text in static/mzfinder/, no encrypted file", packed)

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []; reqs = []
def clicks(pg):
    pg.wait_for_timeout(3200)                    # a fresh window of 3 s
    for _ in range(5): pg.click("header img.logo")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1400, "height": 900}).new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.on("request", lambda r: reqs.append(r.url))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        def off():
            pg.wait_for_timeout(1500)
            assert pg.evaluate("window.QTOOLS.list().length") == 0
            for _ in range(5): pg.click('button.hq[data-help="header"]')          # the old trigger does nothing now
            pg.keyboard.press("Escape")
            assert not pg.query_selector("#qt-dlg") and not pg.query_selector('#nav button[data-v="tpmine"]')
            assert not [u for u in reqs if "/mzfinder/" in u], [u for u in reqs if "/mzfinder/" in u]
        step("off: nothing visible and no mzFinder file downloaded", off)
        def turn_on():
            clicks(pg)
            pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
            assert not pg.query_selector("#mzf-badge")           # no mark next to the logo
            pg.wait_for_function("window.QTOOLS.list().some(t => t.id === 'tpmine')", timeout=60000)      # the tab shows at once; the tool registers when its scripts have loaded
            assert "mzFinder" in pg.inner_text('#nav button[data-v="tpmine"]')
            assert pg.evaluate("localStorage.getItem('qqq.mzfinder')") == "1"
            assert [u for u in reqs if u.endswith("/mzfinder/indice.json")]
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            assert not pg.evaluate("document.getElementById('v-draw').offsetParent")
            pg.click('#nav button[data-v="theory"]'); assert pg.evaluate("document.getElementById('v-tpmine').hidden")
            pg.click('#nav button[data-v="data"]')                           # back to Dati: a reload on #/teoria opens the standalone Teoria page
            pg.screenshot(path=str(HERE / "shots" / "tpmine1.png")) if (HERE / "shots").exists() else None
        step("5 clicks on the logo: tab, state saved", turn_on)
        def reload_on():
            pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
        step("after a reload it stays on", reload_on)
        def turn_off():
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            clicks(pg)
            pg.wait_for_function("!document.querySelector('#nav button[data-v=\"tpmine\"]')")
            assert not pg.query_selector("#v-tpmine")
            assert pg.evaluate("localStorage.getItem('qqq.mzfinder')") is None
            assert pg.evaluate("window.QTOOLS.list().length") == 0
            assert pg.evaluate("S.view") == "data"
        step("5 more clicks: off, back to Dati", turn_off)
        def reload_off():
            reqs.clear(); pg.reload(); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_timeout(1500)
            assert not pg.query_selector("#mzf-badge") and not pg.query_selector('#nav button[data-v="tpmine"]')
            assert not [u for u in reqs if "/mzfinder/" in u]
        step("after a reload it stays off, nothing downloaded", reload_off)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
