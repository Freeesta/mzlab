"""Chosen-peak dot of the spectrum XIC button (xic-spettro.js `mark`): drawn on the apex in high resolution, absent in low resolution (unit mass).
Synthetic files only. The dot is read from the canvas: a pixel 2 px right of the apex is the accent fill with the dot and the background without."""
# covers: mzlab/web/xic-spettro.js, mzlab/web/picchi.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
def apex(pg, sp):
    """Screen position (click) and canvas pixel colours of the tallest peak: [px, py, dot-pixel RGB, accent RGB]."""
    return pg.evaluate("""sp => { const p = eval(sp), q = p.cv.getBoundingClientRect(), a = p._a, d = a.data[0].d; let b = 0; d.y.forEach((v, i) => { if (v > d.y[b]) b = i; });
        const px = a.X(d.mz[b]), py = Math.max(M.t + 6, a.Y(d.y[b])), k = p.cv.width / p.cv.clientWidth, g = p.cv.getContext('2d');
        const c = document.createElement('canvas').getContext('2d'); c.fillStyle = ecss('--accent'); c.fillRect(0, 0, 1, 1);
        return { x: q.left + px, y: q.top + a.Y(d.y[b]) + 6, dot: [...g.getImageData(Math.round((px + 2) * k), Math.round(py * k), 1, 1).data].slice(0, 3), accent: [...c.getImageData(0, 0, 1, 1).data].slice(0, 3) }; }""", sp)
def choose(pg, sp):
    pg.evaluate(f"{sp}.el.scrollIntoView({{block:'center'}})"); frames(pg, 3)
    pk = apex(pg, sp)
    pg.evaluate(f"{sp}.el.querySelector('[data-a=xicb]').click()"); pg.mouse.click(pk["x"], pk["y"]); ready(pg); frames(pg, 3)
    wait_for_js(pg, f"!!XICBTN.chosen({sp})")
    return apex(pg, sp)
def has_dot(a): return all(abs(u - v) <= 8 for u, v in zip(a["dot"], a["accent"]))
D = synth("xicmark")
r = Run(port=8941, wd="/tmp/wd_xicmark")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [str(D / "B_FullMass-t0.mzML")]); pg.click("text=Load data"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        LR = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
        wait_for_js(pg, f"!!{LR}&&!!{LR}._a"); ready(pg)
        def lr():
            assert pg.evaluate(f"!{LR}._a.hrp"), "the file is not unit mass"
            a = choose(pg, LR)
            assert pg.evaluate(f"!!{LR}.el.querySelector('[data-a=xicl]').textContent"), "the choice is not named by the XIC button label"
            assert not has_dot(a), ("low resolution: the chosen-peak dot is drawn", a)
        step("low resolution: the chosen peak has no dot, the choice stays in the button label", lr)
        pg2 = r.page(p)
        load(pg2, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); pg2.evaluate("setTab('full')"); ready(pg2)
        HR = "E.panels.find(p=>p.type==='spec'&&p.level===1)"
        wait_for_js(pg2, f"!!{HR}&&!!{HR}._a"); ready(pg2)
        def hr():
            assert pg2.evaluate(f"!!{HR}._a.hrp"), "the file is not high resolution"
            a = choose(pg2, HR)
            assert has_dot(a), ("high resolution: the chosen-peak dot is gone", a)
        step("high resolution: the chosen peak keeps its dot", hr)
finally: r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
sys.exit(1 if bad else 0)
