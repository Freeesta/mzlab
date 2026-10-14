"""mzFinder HR: parent-TP pairs. Synthetic series whose products carry fragments shifted by the mass difference: the Pred. and Sim. columns, the three
components of the similarity and the mirror, the prediction level, the network of mass differences (SVG, table, keyboard). Run from the repo root:
python3 tests_e2e/e2e_tpmine_ms2.py   (env: PYODIDE_DIR)"""
import os, shutil, subprocess, sys, time, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)
SRC = Path(os.environ.get("TPMINE_SRC", ROOT / "TP_Mine"))
PORT = 8846
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL line %d: %s" % (e.__traceback__.tb_next.tb_lineno if e.__traceback__.tb_next else 0, str(e)[:500])))
tmp = Path(tempfile.mkdtemp())
cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")]
if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
subprocess.run(cmd, check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")          # the engine of the page is Rust: the .wasm goes in the copy of the site
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1500, "height": 1400}, accept_downloads=True).new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_timeout(3200)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        def hr():
            sys.path[:0] = [str(SRC / "tests"), str(SRC / "py"), str(ROOT)]
            import lc_synth, msn_synth
            lc_synth.FRAG[lc_synth.HYDROXY] = [(154.0611, 100), (139.0502, 40), (126.0662, 20), (138.0662, 30)]       # the parent's fragments + O
            d = Path(tempfile.mkdtemp()); fs = lc_synth.write_series(d); mf = msn_synth.write_msn(d / "cafe_msn.mzML", msn_synth.caffeine_nodes())
            pg.wait_for_selector("#tp-go")
            pg.set_input_files("#tp-in", [f["path"] for f in fs] + [str(mf)])
            pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 8", timeout=300000)
            pg.fill("#tp-smi", "Cn1cnc2c1c(=O)n(C)c(=O)n2C"); pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('195.0877')", timeout=60000)
            pg.evaluate("t => { document.getElementById('tp-tr').value += t; }", "\nmy oxygen rule;+O;lit: Doe 2020")                  # an own row with a reference (same change as hydroxylation)
            pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
            pg.locator("#hr-colm summary").click(); pg.check('#hr-cols input[data-col="pred"]'); pg.check('#hr-cols input[data-col="sim"]')          # off by default
            tab = pg.inner_text("#hr-t")
            assert "Pred." in tab and "Sim." in tab, tab[:300]
            assert pg.evaluate("document.querySelectorAll('#hr-t .lvb.pd-1').length") >= 2          # both known products are confirmed by their shifted fragments
            assert pg.evaluate("document.querySelectorAll('#hr-t .sim-bar i').length") >= 6
            pg.click("#hr-t tr.clk:has-text('C8H11N4O3')"); pg.wait_for_selector("#hr-ms2", timeout=60000); pg.wait_for_selector("#tp-det .sim3", timeout=10000)
            det = pg.inner_text("#tp-det")
            assert "shifted fragments" in det and "common neutral losses" in det and "modified cosine" in det and "Prediction" in det and "confirmed by the data" in det, det[:900]
            assert pg.evaluate("document.querySelectorAll('#tp-det .sim3 .bar3').length") == 6
            assert pg.evaluate("[...document.querySelectorAll('#tp-right .lvb:not([class*=pd-])')].every(b => !b.textContent.includes('2'))")          # the program never writes 2b
            nn, ne = pg.evaluate("[document.querySelectorAll('#hr-mnet g.mn').length, document.querySelectorAll('#hr-mnet g.me line').length]")
            assert nn >= 3 and ne >= 2, (nn, ne, pg.inner_text("#hr-mnet")[:500])
            assert pg.evaluate("[...document.querySelectorAll('#hr-mnet g.mn')].every(g => g.getAttribute('aria-label') && g.getAttribute('role') === 'button')")
            assert "hydroxylation" in pg.inner_text("#hr-mnet") and "demethylation" in pg.inner_text("#hr-mnet")
            pg.click("#hr-mnet details summary"); assert "mass difference" in pg.inner_text("#hr-mnet details")
            pg.focus("#hr-mnet g.mn[aria-label*='C7H9N4O2']"); pg.keyboard.press("Enter")
            pg.wait_for_function("document.querySelector('#tp-det h3').textContent.includes('C7H9N4O2')", timeout=60000)
            pg.select_option("#hr-mn-f", "sel"); assert pg.evaluate("document.querySelectorAll('#hr-mnet g.me line').length") == 1
            with pg.expect_download(timeout=60000) as dl: pg.click("#hr-x-csv")
            assert "Prediction level" in Path(dl.value.path()).read_text(encoding="utf-8")
            pg.screenshot(path=str(HERE / "shots" / "tpmine_ms2_hr.png"), full_page=True)
        step("HR: three components, prediction level, mass-difference network", hr)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s_ in steps: print(s_)
print("page errors:", errs)
sys.exit(1 if any(s_[1] != "ok" for s_ in steps) or errs else 0)
