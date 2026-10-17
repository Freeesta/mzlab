"""Sideways spectrum of the RT x m/z map (mappa.js, drawSide) on a PROFILE file: the profile is a continuous line/area of the original points
(min-max decimated by pixel row), not one bar per peak; at strong zoom every point is drawn. Synthetic profile file (tools/dati_sintetici.py)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds
M = "E.panels.find(q=>q.type==='map')"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:900]))
D = Path("/tmp/mapprof_e2e"); D.mkdir(exist_ok=True)
ds.full_scan_profile(D / "P_FullMass-t15.mzML", 15, np.random.default_rng(3))
r = Run(port=8902, wd="/tmp/wd_mapprof_e2e")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        pg = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}).new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#pick", state="attached")
        pg.set_input_files("#pick", [str(D / "P_FullMass-t15.mzML")]); pg.wait_for_selector("#opbtn")
        pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg)
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); ready(pg)
        c = pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
        def pts():
            pg.mouse.move(c[0] + c[2] * .5, c[1] + c[3] * .5); ready(pg)
            pg.wait_for_function(f"{M}._mz.sp.dataset.pts", timeout=15000)
            return pg.evaluate(f"(()=>{{const d={M}._mz.sp.dataset;return [+d.pts,+d.full,{M}._mz.sp.getBoundingClientRect().height]}})()")
        def whole():
            n, full, h = pts(); assert n > 0.3 * h and full > n, (n, full, h)    # a continuous line: about one point per pixel row, not a few bars
        step("profile scan: continuous line, many points", whole)
        def zoom():
            for _ in range(25):
                pg.mouse.wheel(0, -300); ready(pg)
                n, full, h = pts()
                if n == full: break
            assert full >= 5 and n == full, (n, full)                                             # zoomed in: every original point is drawn
        step("strong zoom: every point is drawn", zoom)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
