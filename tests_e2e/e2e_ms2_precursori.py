"""Two MS2 .wiff with different precursors on the built site with the Rust engine (synthetic pairs of tools/wiff_modi.py, EPI of 55.5, and tools/wiff_misto.py,
product ion of 305; no laboratory data): after a click on a file of the list (it becomes the current one) or a double click (it is shown alone), the
MS2 chromatogram and product-ion spectrum draw a precursor of THAT file, not the precursor of the file shown before nor the lowest of all the MS2 files
(a precursor the file does not have left both panels blank), whether the MS2 pair already exists (from MS2 file to MS2 file) or is made again (after a
Full Scan file).
Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_ms2_precursori.py"""
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="ms2_prec_"))
for tool in ("wiff_modi.py", "wiff_misto.py"):
    subprocess.run([sys.executable, str(ROOT / "tools" / tool), str(D)], check=True, capture_output=True)
FILES = [D / "modi" / "EPI.wiff", D / "modi" / "EPI.wiff.scan", D / "misto.wiff", D / "misto.wiff.scan"]
# the MS2 panels that are on screen: precursor, files drawn, whether the chart has data
SHOWN = """() => E.panels.filter(p => p.tab === 'ms2' && p.el && p.el.offsetParent).map(p => ({ type: p.type, prec: p.prec, k: p.k,
  data: p.type === 'spec' ? !!(p._a && p._a.data && p._a.data[0] && p._a.data[0].d.mz.length) : !!(p._a && (p._a.sr || []).some(s => s.y.some(v => v > 0))) }))"""

def show(pg, name, dbl=False):
    k, kind = pg.evaluate(f"(f => [f.k, f.kind])(E.files.find(f => f.file === {json.dumps(name)} && !f.gone))")
    row = pg.locator(f"#flst .fl[data-k='{k}'] .nm")
    if not row.is_visible():          # only the first group of the list starts open: open the one of the file, as a student does
        pg.click(f"#flst .fgh[data-gt='{kind}'] .chv"); row.wait_for(state="visible")
    row.dblclick() if dbl else row.click()
    ready(pg); frames(pg, 3)
    return k

def drawn(pg, k, prec, alone=False):
    """Waits until the MS2 chromatogram and spectrum on screen draw precursor `prec` with data and the spectrum is the one of file `k`."""
    ok = f"""(() => {{ const g = ({SHOWN})(); return g.some(x => x.type === 'chrom') && g.some(x => x.type === 'spec')
      && g.every(x => Math.abs(Number(x.prec) - {prec}) < 0.6 && x.data && (x.type !== 'spec' || x.k === {k})); }})()"""
    try: pg.wait_for_function(ok, timeout=30000)
    except Exception: raise AssertionError((k, prec, pg.evaluate(SHOWN))) from None
    vis = pg.evaluate("E.files.filter(f => f.vis && !f.gone).map(f => f.k)")
    assert k in vis and (not alone or vis == [k]), vis

r = Run(port=8877, wd="/tmp/wd_ms2_prec")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, FILES); open_first(pg); ready(pg); drawer(pg)
        def ms2_to_ms2():
            show(pg, "EPI_s1.mzML"); drawn(pg, pg.evaluate("E.cur"), 55.5)
            k = show(pg, "misto_s2.mzML"); drawn(pg, k, 305.0)
            k = show(pg, "EPI_s2.mzML"); drawn(pg, k, 55.5)
        step("from an MS2 file to one of another precursor: the pair follows the file shown", ms2_to_ms2)
        def after_full():
            show(pg, "EPI_s1.mzML"); show(pg, "misto_s1.mzML")
            k = show(pg, "misto_s5.mzML", dbl=True); drawn(pg, k, 305.0, alone=True)
            k = show(pg, "EPI_s2.mzML", dbl=True); drawn(pg, k, 55.5, alone=True)
        step("after a Full Scan file (double clicks) the MS2 pair takes a precursor of the file shown", after_full)
        step("the engine answered every request", lambda: (lambda m: m and (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
bad = [n for n, s in steps if s != "ok"]
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report() if bad else None
sys.exit(1 if bad else 0)
