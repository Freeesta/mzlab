"""mzFinder, HR: the RT alignment card (shift against RT, one line per file) and the filled cells of a weak product (marked, never a found peak).
Synthetic series with a known RT drift. Run from the repo root: python3 tests_e2e/e2e_tpmine_rtalign.py   (env: PYODIDE_DIR)"""
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
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1500, "height": 1400}).new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_timeout(3200)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        def card():
            sys.path[:0] = [str(SRC / "tests"), str(SRC / "py"), str(ROOT)]
            import lc_synth, msn_synth
            d = Path(tempfile.mkdtemp())
            fs = lc_synth.write_series(d, weak=0.2, anchors=True, drift={-1: 0.0, 0: 0.06, 5: -0.04, 10: 0.0, 20: 0.05, 40: -0.03})
            mf = msn_synth.write_msn(d / "cafe_msn.mzML", msn_synth.caffeine_nodes())
            pg.set_input_files("#tp-in", [f["path"] for f in fs] + [str(mf)])
            pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 8", timeout=300000)
            pg.fill("#tp-smi", "Cn1cnc2c1c(=O)n(C)c(=O)n2C"); pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('195.0877')", timeout=60000)
            pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
            pg.wait_for_selector("#hr-rta", timeout=30000)
            assert not pg.evaluate("document.querySelector('#hr-rta').open")                 # closed until asked for
            pg.click("#hr-rta summary")
            n = pg.evaluate("document.querySelectorAll('#hr-rta svg.rta-svg polyline').length"); assert n == 6, n
            assert pg.evaluate("document.querySelectorAll('#hr-rta .rta-leg > span').length") == 6
            assert "anchor" in pg.inner_text("#hr-rta summary")
            lab = pg.get_attribute("#hr-rta svg.rta-svg", "aria-label"); assert lab and "6 files" in lab, lab
            # the colours come from variables, never from fixed values in the drawing
            assert pg.evaluate("[...document.querySelectorAll('#hr-rta polyline')].every(p => p.getAttribute('stroke').startsWith('var(--rta-'))")
            pg.screenshot(path=str(HERE / "shots" / "tpmine_rtalign.png"), full_page=True)
        step("HR: the RT alignment card draws one line per file with its legend", card)
        def filled():
            row = pg.locator("#hr-t tr.clk", has_text="C8H11N4O3"); assert row.count() == 1
            assert "○1" in row.inner_text(), row.inner_text()                                # the table says one area was filled
            row.click(); pg.wait_for_selector("#hr-kleg", timeout=60000)
            pg.wait_for_function("document.getElementById('hr-kleg').textContent.includes('filled')", timeout=30000)
            assert "5" in pg.inner_text("#hr-kleg").split("filled")[1]                       # the file of the first treated time (5 min)
        step("HR: the weak product has one filled area, marked in the table and in the kinetics legend", filled)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s_ in steps: print(s_)
print("page errors:", errs)
sys.exit(1 if any(s_[1] != "ok" for s_ in steps) or errs else 0)
