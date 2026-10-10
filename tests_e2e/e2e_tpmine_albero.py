"""mzFinder: fragmentation tree of the parent (SVG). LR demo (supposed tree: N nodes, N-1 edges, a click opens the structure, keyboard, comparison
with a product, SVG export) and HR (synthetic series + MSn). Run from the repo root: python3 tests_e2e/e2e_tpmine_albero.py   (env: PYODIDE_DIR)"""
import os, shutil, subprocess, sys, time, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)
SRC = Path(os.environ.get("TPMINE_SRC", ROOT / "TP_Mine"))
PORT = 8845
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL line %d: %s" % (e.__traceback__.tb_next.tb_lineno if e.__traceback__.tb_next else 0, str(e)[:500])))
tmp = Path(tempfile.mkdtemp())
cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")]
if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
subprocess.run(cmd, check=True)
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1500, "height": 1400}, accept_downloads=True).new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_timeout(3200)
        for _ in range(5): pg.click("header img.logo")
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        def lr():
            sys.path[:0] = [str(SRC / "py"), str(ROOT)]
            from tpmine.demo import make_demo
            fl = make_demo(Path(tempfile.mkdtemp()))
            pg.set_input_files("#tp-in", [f["path"] for f in fl])
            pg.wait_for_function("document.querySelector('#tp-files table') && document.querySelectorAll('#tp-files tr').length >= 9", timeout=240000)
            pg.fill("#tp-smi", "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O")
            pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('241.0641')", timeout=60000)
            pg.click("#tp-go"); pg.wait_for_selector("#tp-view .gem", timeout=300000); pg.wait_for_timeout(1300)
            pg.click('.vt[data-v="tab"]'); pg.wait_for_selector("#tp-t tr.clk")
            pg.click("#tp-t tr.clk:has(td:nth-child(4):text-is('parent'))")
            pg.wait_for_selector("#tp-alb svg.alb-svg .nd", timeout=60000)
            n, e = pg.evaluate("[document.querySelectorAll('#tp-alb svg.alb-svg .nd').length, document.querySelectorAll('#tp-alb svg.alb-svg g.vp > path').length]")
            assert n >= 2 and e == n - 1, (n, e)
            assert "supposed: unit resolution" in pg.inner_text("#tp-alb")
            assert pg.evaluate("[...document.querySelectorAll('#tp-alb .nd')].every(g => g.getAttribute('aria-label') && g.getAttribute('role') === 'treeitem')")
            pg.click("#tp-alb .nd:nth-of-type(2)")
            pg.wait_for_selector("#tp-alb .alb-det svg", timeout=60000)               # the structure of the clicked fragment (OpenChemLib)
            assert "Alternative formula" in pg.inner_text("#tp-alb .alb-det")
            sel = pg.evaluate("document.activeElement.dataset.id")
            pg.keyboard.press("ArrowUp"); pg.wait_for_function("document.activeElement.dataset.id !== %r" % sel, timeout=5000)     # to the parent
            pg.keyboard.press("Enter"); ae = pg.evaluate("document.activeElement.tagName + ':' + document.activeElement.getAttribute('role') + ':' + (document.activeElement.dataset||{}).id"); assert ae.split(":")[1] == "treeitem", ae
            with pg.expect_download(timeout=30000) as dl: pg.click("#tp-alb .alb-b[data-a=svg]")
            assert "<svg" in Path(dl.value.path()).read_text(encoding="utf-8")
            pg.screenshot(path=str(HERE / "shots" / "tpmine_albero_lr.png"), full_page=True)
        step("LR: supposed tree of the parent (N nodes, N-1 edges, structure on click, keyboard, SVG)", lr)
        def lr_cmp():
            pg.click("#tp-t tr.clk:has-text('hydroxylation @ 7.40')"); pg.wait_for_selector("#tp-alb #alb-cmp", timeout=60000)
            pg.click("#alb-cmp")
            pg.wait_for_function("document.getElementById('alb-leg').textContent.includes('shifted')", timeout=60000)
            assert pg.evaluate("document.querySelectorAll('#tp-alb .nd text').length") > 0 and "Δ" in pg.inner_text("#tp-alb")
            assert pg.get_attribute("#alb-cmp", "aria-pressed") == "true"
            pg.click("#alb-cmp"); pg.wait_for_function("document.getElementById('alb-leg').textContent === ''", timeout=10000)
        step("LR: comparison with a product marks the shifted fragments", lr_cmp)
        def hr():
            sys.path[:0] = [str(SRC / "tests"), str(SRC / "py"), str(ROOT)]
            import lc_synth, msn_synth
            d = Path(tempfile.mkdtemp()); fs = lc_synth.write_series(d, hidden_isf=True); mf = msn_synth.write_msn(d / "cafe_msn.mzML", msn_synth.caffeine_nodes())
            pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.set_input_files("#tp-in", [f["path"] for f in fs] + [str(mf)])
            pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 8", timeout=300000)
            pg.fill("#tp-smi", "Cn1cnc2c1c(=O)n(C)c(=O)n2C"); pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('195.0877')", timeout=60000)
            pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
            pg.wait_for_selector("#hr-tree svg.alb-svg .nd", timeout=60000)
            n, e = pg.evaluate("[document.querySelectorAll('#hr-tree svg.alb-svg .nd').length, document.querySelectorAll('#hr-tree svg.alb-svg g.vp > path').length]")
            assert n >= 5 and e == n - 1, (n, e)
            assert pg.evaluate("document.querySelectorAll('#hr-tree .nd rect.b[stroke-dasharray]').length") >= 1          # a ghost node is dashed
            assert pg.evaluate("document.querySelector('#hr-tree details') === null") and "supposed" not in pg.inner_text("#hr-tree")
            pg.click("#hr-tree .nd[aria-label*='C6H8N3O']")
            pg.wait_for_selector("#hr-tree .alb-det svg", timeout=60000)
            pg.click("#hr-t tr.clk:has-text('C8H11N4O3')"); pg.wait_for_selector("#hr-ms2", timeout=60000)
            pg.click("#hr-tree #alb-cmp")
            pg.wait_for_function("document.getElementById('alb-leg').textContent.length > 0 && !document.getElementById('alb-leg').textContent.includes('Comparing')", timeout=60000)
            pg.screenshot(path=str(HERE / "shots" / "tpmine_albero_hr.png"), full_page=True)
        step("HR: MSn tree with leaves, ghost dashed, structure on click, comparison button", hr)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s_ in steps: print(s_)
print("page errors:", errs)
sys.exit(1 if any(s_[1] != "ok" for s_ in steps) or errs else 0)
