# e2e_mzml_molte_scansioni: an msconvert-style DDA mzML with tens of thousands of scans (the scan table and the index of the engine grow with the scans, not only with the peaks) opens in the
# browser: the file is listed, nothing is refused, the engine answers every route. Synthetic (tools/mzml_msconvert_sintetico.py), no laboratory data. The cases of a file that does not fit are in
# crates/mzlab-core/tests/mzml_grande_memoria.rs (peak of the heap per phase) and tests/test_diagnosi.py (cause «engine» for a trap).
import os, sys, tempfile, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import mzml_msconvert_sintetico as gen
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
def many_scans():
    d = tempfile.mkdtemp(prefix="molte_"); f = os.path.join(d, "dda_lungo.mzML")
    size = gen.scrivi(f, scans=40000, peaks1=400, peaks2=60)
    r = Run(port=8851, wd="/tmp/wd_molte_scansioni")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.set_input_files("#pick", [f])
            pg.wait_for_function("UPBUSY === 0 && (ST.files.length > 0 || QQQ_LOADFAIL.size > 0)", timeout=600000)
            assert pg.evaluate("QQQ_LOADFAIL.size") == 0, pg.evaluate("[...QQQ_LOADFAIL]")
            assert pg.evaluate("document.querySelector('#unopen .diag') === null"), pg.inner_text("#unopen")
            if pg.evaluate("!document.getElementById('start').hidden && document.getElementById('loading').hidden"): pg.click("#opbtn")
            ready(pg, timeout=600000)
            assert pg.evaluate("E.files.length") >= 1, f"{size} B: no file open"
            assert not pg.evaluate("window.MZLAB_LR_MISS || []"), pg.evaluate("window.MZLAB_LR_MISS")
    finally:
        r.close(); shutil.rmtree(d, ignore_errors=True)
step("40000 scans: the file opens and the engine answers every route", many_scans)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
