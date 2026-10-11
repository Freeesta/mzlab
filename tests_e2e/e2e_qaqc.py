"""QA/QC panel: low resolution (only the standards chosen and the blanks, no Δppm), high resolution (Δppm, mass accuracy), sample/blank ratio."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_qaqc.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))

D = HERE / "shots" / "qaqc_tmp"; D.mkdir(parents=True, exist_ok=True)
ds.full_scan(D / "S_t0.mzML", 0, np.random.default_rng(1)); ds.full_scan(D / "S_t30.mzML", 30, np.random.default_rng(2))
rng = np.random.default_rng(3)
ds._write(D / "Blank_1.mzML", [ds._spec(i, 0.5 + i * 0.0175, list(rng.uniform(120, 480, 45)), list(rng.lognormal(np.log(4e3), 0.9, 45))) for i in range(1111)], [])
H = synth("hrqaqc")

def load(pg, files, n):
    pg.set_input_files("#pick", [str(f) for f in files]); 
    pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length>={len(files)}", timeout=120000); pg.wait_for_timeout(1500)
    if pg.is_visible("#opbtn"): pg.click("#opbtn")
    try: pg.wait_for_function(f"typeof E!=='undefined' && E.files.length>={n}", timeout=60000)
    except TimeoutError: print(pg.evaluate("[S.view, !!S.sess, typeof E, document.body.innerText.slice(0,400)]")); raise
    ready(pg)

def add(pg, mz, rt, kind=None):
    pg.fill("#qq-in", mz); pg.fill("#qq-rt", rt)
    if kind: pg.select_option("#qq-kind", kind)
    pg.click("#qq-addb"); pg.wait_for_selector("#qq-res .qq-ion table", timeout=60000); ready(pg)

r = Run(port=8931, wd="/tmp/wdqaqc")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "S_t0.mzML", D / "S_t30.mzML", D / "Blank_1.mzML"], 3)
        def lr_panel():
            pg.evaluate("localStorage.removeItem('qqq.qaqc')"); drawer(pg, "qaqc"); pg.wait_for_selector("#sb-panel-qaqc select[data-role]")
            roles = pg.evaluate("Array.from(document.querySelectorAll('#sb-panel-qaqc select[data-role]')).map(s=>[s.dataset.role,s.value])")
            assert dict(roles)["Blank_1.mzML"] == "blank" and dict(roles)["S_t0.mzML"] == "sample", roles
            assert pg.locator("#qq-kind").count() == 0                                         # LR: standards only, no list of known ions
        step("LR: types guessed from the names, no «known ion» choice", lr_panel)
        def lr_std():
            add(pg, "391.3", "18.3")
            hdr = pg.evaluate("Array.from(document.querySelectorAll('#qq-res th')).map(x=>x.textContent)")
            assert "Δppm" not in hdr and "RT" in hdr and "ΔRT" in hdr, hdr
            rows = pg.evaluate("QAQC.results.ions[0].r.rows.map(x=>x&&[x.rt,x.area])")
            assert abs(rows[1][0] - 18.3) < 0.1 and rows[1][1] > 1e5 and (rows[0] is None or rows[0][1] < 0.01 * rows[1][1]), rows        # the blank (first by name) has only noise
            assert pg.evaluate("QAQC.results.ions[0].r.mass_accuracy") is None
            assert pg.locator("#qq-res svg.qq-chart").count() == 2
        step("LR: RT, area and relative area of a standard along the series; the blank empty; two charts", lr_std)
        def lr_ratio():
            pg.fill("#qq-rin", "391.3"); pg.fill("#qq-rrt", "18.3"); pg.select_option("#qq-thr", "10")
            pg.click("#qq-rform button[type=submit]"); pg.wait_for_selector("#qq-rres table", timeout=60000)
            q = pg.evaluate("QAQC.results.ratio.r.ratio")
            assert q["threshold"] == 10 and q["rows"][0] is None and q["rows"][1]["ratio"] > 100 and q["rows"][1]["at_least"], q
        step("sample/blank ratio at 10:1 (far above the blank)", lr_ratio)
        def lr_role_fix():
            pg.select_option("select[data-role='S_t30.mzML']", "qc"); ready(pg); pg.wait_for_timeout(600)
            assert pg.evaluate("QAQC.results.ions[0].roles.join()") == "blank,sample,qc"
            assert "qc" in pg.evaluate("localStorage.getItem('qqq.qaqc')")
        step("a type corrected by hand is used and remembered", lr_role_fix)

        r.close(); r = Run(port=8932, wd="/tmp/wdqaqchr"); pg = r.page(p)
        load(pg, [H / ds.HR_NAMES["exploris"], H / ds.HR_NAMES["fusion"]], 4)
        def hr_panel():
            assert pg.evaluate("BANCO.on()")
            drawer(pg, "qaqc"); pg.wait_for_selector("#qq-kind")
            add(pg, "346.1218", "10.5", "known")
            rows = pg.evaluate("QAQC.results.ions[0].r.rows.map(x=>x&&x.dppm)")
            assert rows[0] is not None and abs(rows[0]) < 2.5 and rows[1] is not None and 1.0 < rows[1] < 5.0, rows       # Exploris ~0, Fusion +3 ppm
            ma = pg.evaluate("QAQC.results.ions[0].r.mass_accuracy")
            assert ma and ma["max_abs"] >= 1.0 and ma["n"] == 2, ma
            assert "Δppm" in pg.evaluate("Array.from(document.querySelectorAll('#qq-res th')).map(x=>x.textContent)")
        step("HR: Δppm per file and mass accuracy (mean, maximum) of a known ion", hr_panel)
finally:
    r.close()
for s in steps: print(s)
print("ERRORS", [e for e in r.errs if e[0] != "console.warning"][:5])
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
