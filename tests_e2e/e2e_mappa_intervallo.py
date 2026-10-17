"""Whole view of the RT x m/z map = where the files have data: a Full Scan acquired from 0.5 to 20 min (declared run 0-22) starts at ~0.5 and ends at ~20,
the m/z axis at the first and last peak; zoom and pan stay within the data (plus the margin); «whole view» comes back to it; the sight, the plots at the margins
and the 3D viewer use the same limits. A file with data from 0 keeps today's view. High resolution too."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, hr_variants
import numpy as np
sys.path.insert(0, str(ROOT))
from mzlab import demo
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_intervallo.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
M = "E.panels.find(q=>q.type==='map')"
def write(path, t0, t1):
    rng = np.random.default_rng(4); specs = []
    for i, rt in enumerate(np.arange(t0, t1 + 1e-9, 0.1)):
        mz = np.sort(rng.uniform(120, 360, 15)); specs.append(demo._spectrum(i, 1, float(rt), mz, rng.lognormal(8, .5, 15)))
    head = '<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><run id="r"><spectrumList count="%d">' % len(specs)
    Path(path).write_text(head + "".join(specs) + "</spectrumList></run></mzML>", encoding="utf-8")
D = HERE / "shots" / "intervallo_tmp"; D.mkdir(parents=True, exist_ok=True)
write(D / "late-t0.mzML", 0.5, 20.0); write(D / "zero-t0.mzML", 0.0, 20.0)
r = Run(port=8901, wd="/tmp/wd_intervallo")
def opened(pg, f, hr=False, run=None):
    pg.goto(f"http://127.0.0.1:{(run or r).port}/"); pg.wait_for_timeout(800)
    pg.set_input_files("#pick", [str(f)]); pg.wait_for_timeout(800); pg.click("#opbtn"); ready(pg)
    if hr: pg.wait_for_timeout(1500); pg.evaluate("document.querySelector('.fl')?.click()"); ready(pg); pg.wait_for_timeout(800)
    pg.evaluate("reveal(addPanel('map',{k:E.files[0].k}))"); ready(pg); pg.wait_for_timeout(800)
view = lambda pg: pg.evaluate(f"(()=>{{const a={M}._a;return {{x:[a.x0,a.x1],y:[a.y0,a.y1],full:a.full,fy:a.fullY,g:a.grid}}}})()")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        def page():
            pg = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}).new_page()
            pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
            pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e)))); return pg
        pg = page(); opened(pg, D / "late-t0.mzML")
        def start():
            v = view(pg)
            assert 0.3 < v["x"][0] <= 0.5 and 20 <= v["x"][1] < 20.4, v                       # ~0.5 to ~20, not 0 to 20 / 21
            assert v["full"] == v["x"] and v["fy"] == v["y"], v
            assert 100 < v["y"][0] <= 120.2 and 359.8 <= v["y"][1] < 362, v                    # the m/z of the first and the last peak (+ margin)
            assert v["g"]["rt"][0] <= 0.0 + 1e-9 and v["g"]["rt"][1] >= 20, v                  # the grid of the bins is the same as always
        step("LR: the whole view goes from the first to the last scan and from the first to the last peak", start)
        def zoom():
            v0 = view(pg); box = pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
            pg.mouse.move(box[0] + box[2] * .5, box[1] + box[3] * .5)
            for _ in range(3): pg.mouse.wheel(0, -250); pg.wait_for_timeout(80)
            v1 = view(pg); assert v1["x"][1] - v1["x"][0] < v0["x"][1] - v0["x"][0]
            for _ in range(12): pg.mouse.wheel(0, 400); pg.wait_for_timeout(60)               # zooming out never goes beyond the data (+ margin)
            v2 = view(pg); assert v2["x"][0] >= v0["x"][0] - 1e-6 and v2["x"][1] <= v0["x"][1] + 1e-6, (v0, v2)
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("0"); pg.wait_for_timeout(300)
            assert pg.evaluate(f"[{M}.zoom,{M}.zoomY]") == [None, None] and view(pg)["x"] == v0["x"]       # whole view = the data
            pg.mouse.wheel(0, -300); pg.wait_for_timeout(200); pg.keyboard.press("Backspace"); pg.wait_for_timeout(200)
            assert view(pg)["x"] == v0["x"]
        step("zoom out and pan stay within the data; 0 and Backspace come back to the whole view", zoom)
        def sight():
            pg.keyboard.press("f"); pg.wait_for_timeout(900); ready(pg); v = view(pg)
            box = pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
            pg.mouse.move(box[0] + box[2] * .5, box[1] + box[3] * .5); pg.mouse.move(box[0] + box[2] * .52, box[1] + box[3] * .5)
            pg.wait_for_function(f"!!({M}._mz.spec && {M}._mz.xic)", timeout=15000); ready(pg)
            rts = pg.evaluate(f"{M}._mz.xic.tr[0].rt"); assert rts[0] >= 0.49 and rts[-1] <= 20.01, (rts[0], rts[-1])      # the XIC below spans the acquisition, the axis of the map is the same
            al = pg.evaluate(f"(()=>{{const s={M}._mz;return s.xc.getBoundingClientRect().left-{M}.cv.getBoundingClientRect().left}})()"); assert abs(al) < 1, al
            pg.keyboard.press("f"); pg.wait_for_timeout(500)
        step("the sight and its plots follow the same limits", sight)
        def d3():
            pg.evaluate(f"MAPPA.open3d({M}, document.body)"); pg.wait_for_timeout(2500)
            rt = pg.evaluate("(()=>{const e=document.querySelector('[data-m=rt0]');return e?[parseFloat(e.value),parseFloat(document.querySelector('[data-m=rt1]').value)]:null})()")
            assert rt and 0.3 < rt[0] <= 0.55 and 19.9 <= rt[1] < 20.4, rt                      # the 3D viewer opens on the same part
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
        step("3D map opens on the same RT range", d3)
        r2 = Run(port=8902, wd="/tmp/wd_intervallo2"); pg2 = page(); opened(pg2, D / "zero-t0.mzML", run=r2)
        def zero():
            v = view(pg2); assert abs(v["x"][0]) < 1e-9 and 19.9 < v["x"][1] <= 21.01, v        # data from 0: the view starts at 0 as today
        step("data from 0 min: the view starts at 0", zero)
        pg3 = page(); r_hr = synth("intervallo"); hr = hr_variants(r_hr / "HR_DDA-Exploris-t30.mzML", r_hr)[0]
        r3 = Run(port=8903, wd="/tmp/wd_intervallo3"); opened(pg3, hr, True, run=r3)
        def hrcase():
            v = view(pg3); g = v["g"]
            assert g["rt"][0] <= v["x"][0] and v["x"][1] <= g["rt"][1] + 1e-9 and v["x"][0] < v["x"][1], v
            assert v["full"] == v["x"] and v["fy"] == v["y"] and v["y"][0] >= g["mz"][0] - 1e-9 and v["y"][1] <= g["mz"][1] + 1e-9, v
        step("HR: the view is inside the grid and equals the limits of the data", hrcase)
        b.close(); r2.close(); r3.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
