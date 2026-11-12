"""The DAD of a file with no scans on the built site with the Rust engine: an MRM and a SIM (Q1MI) 3200 `.wiff` with a UV/Vis matrix (synthetic pairs of
tools/wiff_dad_mrm.py, no laboratory data). `api/chrom?kind=pda&mz0&mz1` answers the wavelength trace (its band in nm, its maximum the synthetic absorbance),
not the TWC; without a band it answers the TWC; the MS chromatograms of the file stay as they were. Run from the repo root: python3 tests_e2e/e2e_dad_mrm.py"""
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="dad_mrm_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_dad_mrm.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "dadmrm.json").read_text())
CHROM = "fetch('api/chrom?k=%d&kind=%s&level=1%s').then(r => r.json()).then(j => ({max: Math.max(...j.y), at: j.rt[j.y.indexOf(Math.max(...j.y))], n: j.y.length, nm0: j.nm0 ?? null, nm1: j.nm1 ?? null}))"
r = Run(port=8879, wd="/tmp/wd_dad_mrm")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / f) for f in ("dadmrm.wiff", "dadmrm.wiff.scan", "dadsim.wiff", "dadsim.wiff.scan")])     # two pairs: two samples
        wait_for_js(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=90000); pg.click("text=Carica dati"); ready(pg)
        wait_for_js(pg, "E.files.length >= 2", timeout=120000)
        K = {kind: pg.evaluate(f"E.files.find(f => f.name.startsWith('{kind}')).k") for kind in ("dadmrm", "dadsim")}
        def files():
            got = pg.evaluate("E.files.map(f => [f.kind, f.pda === true])")
            assert got == [["mrm", True], ["mrm", True]], got
        step("both files have no scans and a DAD", files)
        def band(kind):
            def f():
                for b in ("a", "b"):
                    t = TRUE[b]
                    got = pg.evaluate(CHROM % (K[kind], "pda", f"&mz0={t['nm0']:g}&mz1={t['nm1']:g}"))
                    assert (got["nm0"], got["nm1"]) == (t["nm0"], t["nm1"]), (b, got)
                    assert got["n"] == TRUE["rows"] and abs(got["max"] - t["max"]) < 0.01 * t["max"] and abs(got["at"] - t["rt"]) < 0.03, (b, got, t)
                twc = pg.evaluate(CHROM % (K[kind], "pda", ""))
                assert twc["nm0"] is None and abs(twc["max"] - TRUE["twc_max"]) < 0.01 * TRUE["twc_max"], twc
                assert twc["max"] > 10 * TRUE["a"]["max"], twc
            return f
        step("MRM: the 270-278 nm trace is the band (max and apex of the synthetic absorbance), the 228-232 nm one too, without a band the TWC", band("dadmrm"))
        step("SIM: the same", band("dadsim"))
        def ms_kinds():
            for kind in ("dadmrm", "dadsim"):
                tic = pg.evaluate(CHROM % (K[kind], "tic", ""))
                assert tic["n"] > 300 and tic["nm0"] is None and tic["max"] > 5.0e4, (kind, tic)
        step("the TIC of both files is still the MS one", ms_kinds)
        step("the engine answered every request", lambda: (lambda m: m and (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
