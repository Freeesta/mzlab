"""Tabui (internal name TP Mine, also mzFinder) step (a): the tab exists only in the administrator mode (localStorage «qqq.admin» = «1», QTOOLS admin:true of tpmine-loader.js).
Without it: no tab, no section, the package of Tabui is not downloaded, QTOOLS lists nothing, and the address #/tpmine stays on the Data tab. With it: the tab is in the header
from the start and survives a reload. Five quick clicks on the logo bring the tab up at once, without a reload; five more take it away (the page reloads).
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
step("built site: Tabui in plain text in static/mzfinder/, no encrypted file", packed)

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []; reqs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); TAB = '#nav button[data-v="tpmine"]'
        def fresh(admin, hash_=""):
            c = b.new_context(viewport={"width": 1400, "height": 900}); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("request", lambda r: reqs.append(r.url))
            if admin: pg.add_init_script("try{if(!sessionStorage.getItem('adm0')){sessionStorage.setItem('adm0','1');localStorage.setItem('qqq.admin','1');}}catch(_){}")      # once per tab: a reload after leaving the mode must not switch it back on
            pg.goto(f"http://127.0.0.1:{PORT}/" + hash_); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_timeout(1500)
            return pg
        pg = fresh(False)
        def absent():
            assert not pg.query_selector(TAB) and not pg.query_selector("#v-tpmine"), "the tab exists without the administrator mode"
            assert pg.evaluate("window.QTOOLS.list().length") == 0
            assert not [u for u in reqs if "/mzfinder/" in u], [u for u in reqs if "/mzfinder/" in u][:3]      # the package is not even downloaded
            assert pg.evaluate("[...document.querySelectorAll('#nav button')].map(b => b.dataset.v)") == ["data", "draw", "theory"]
        step("without qqq.admin there is no Tabui tab and nothing is downloaded", absent)
        def hash_stays():
            q = fresh(False, "#/tpmine")
            assert q.evaluate("S.view") == "data" and not q.query_selector("#v-tpmine"), q.evaluate("S.view")
            q.close()
        step("without qqq.admin the address #/tpmine opens the normal page", hash_stays)
        def five_clicks():
            for _ in range(5): pg.click("header img.logo")
            pg.wait_for_function("localStorage.getItem('qqq.admin') === '1'", timeout=15000)
            pg.wait_for_selector(TAB, timeout=30000)                            # no reload: the tab comes up at once
            pg.wait_for_function("window.QTOOLS.list().some(t => t.id === 'tpmine')", timeout=60000)
        step("five quick clicks on the logo bring the tab up without a reload", five_clicks)
        pg.context.close(); reqs.clear()
        pg = fresh(True)
        def visible():
            pg.wait_for_selector(TAB, timeout=30000)
            assert not pg.query_selector("#mzf-badge")           # no mark next to the logo
            pg.wait_for_function("window.QTOOLS.list().some(t => t.id === 'tpmine')", timeout=60000)      # the tab shows at once; the tool registers when its scripts have loaded
            txt = pg.inner_text(TAB)
            assert "Tabui" in txt and "mzFinder" not in txt, txt
            assert [u for u in reqs if u.endswith("/mzfinder/indice.json")]
            pg.click(TAB); pg.wait_for_selector("#tp-go")
            assert not pg.evaluate("document.getElementById('v-draw').offsetParent")
            pg.click('#nav button[data-v="theory"]'); assert pg.evaluate("document.getElementById('v-tpmine').hidden")
            pg.click('#nav button[data-v="data"]')                           # back to Dati: a reload on #/teoria opens the standalone Teoria page
        step("with qqq.admin the Tabui tab is in the header at once", visible)
        def reload_on():
            pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_selector(TAB, timeout=30000)
        step("after a reload it is still there", reload_on)
        def five_off():
            for _ in range(5): pg.click("header img.logo")
            pg.wait_for_function("localStorage.getItem('qqq.admin') === null", timeout=15000)
            pg.wait_for_function("!document.querySelector('#nav button[data-v=\"tpmine\"]')", timeout=30000)      # the page reloads once the logo is whole
            pg.wait_for_selector("#drop", timeout=120000)
            assert not pg.query_selector(TAB) and pg.evaluate("window.QTOOLS.list().length") == 0
        step("five more clicks on the logo leave the mode: the tab is gone", five_off)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
