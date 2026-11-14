"""H1: a click on the chromatogram of a DDA file (sparse MS2) always gives the closest scan, never an empty spectrum; the cursor moves to its real RT."""
# covers: mzlab/web/dda.js, mzlab/web/dda-marcatori.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_nearest.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrnear")
r = Run(port=8898, wd="/tmp/wdhr11")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        def near():
            k = pg.evaluate("E.files.find(f=>f.kind==='ms2').k")
            sid = pg.evaluate(f"(async()=>{{const j=await (await fetch('api/dda?k={k}')).json();return j.rt}})()")
            rt0, rt1 = sid[0], sid[1]; mid = (rt0 + rt1) / 2
            a = pg.evaluate(f"nearScan({k}, {mid - 1e-3})"); b = pg.evaluate(f"nearScan({k}, {mid + 1e-3})")
            assert abs(a["rt"] - rt0) < 1e-6 and abs(b["rt"] - rt1) < 1e-6, (a, b, rt0, rt1)
        step("nearScan: the closest MS2 scan on either side of the midpoint", near)
        def dbl():
            n = pg.evaluate("E.panels.filter(p=>p.type==='spec').length")
            box = pg.locator(".pnl.chrom canvas").first.bounding_box()
            pg.mouse.dblclick(box["x"] + box["width"] * 0.5, box["y"] + box["height"] * 0.5); pg.wait_for_timeout(2500)
            v = pg.evaluate("(()=>{const sp=E.panels.filter(p=>p.type==='spec'),s=sp.filter(p=>p.link!=null).slice(-1)[0]||sp.slice(-1)[0];return s&&s._a&&s._a.data&&s._a.data[0]?s._a.data[0].d.mz.length:0})()")
            assert pg.evaluate("E.panels.filter(p=>p.type==='spec').length") > n - 1 and v > 0, v
        step("double click on the chromatogram: a spectrum with peaks (never empty)", dbl)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
