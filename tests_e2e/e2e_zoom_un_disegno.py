"""Ctrl+wheel zoom of the TIC and of the spectrum: one draw() of the zoomed panel per wheel step, and the picture is the one a fresh draw gives
(same y axis labels, same margins: nothing is drawn late)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_zoom_un_disegno.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("zoom_un_disegno")
r = Run(port=8865, wd="/tmp/wd_zoom_un_disegno")
# counts every draw() of every panel: the page calls the global one, so the wrapper sees them all
COUNT = "window.__dr = []; if (!window.__drWrapped) { window.__drWrapped = true; const d0 = window.draw; window.draw = async function (p) { window.__dr.push(p && p.id); return d0.apply(this, arguments); }; } 0"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "B_FullMass-t0.mzML"], 6000, 1); ready(pg)
        PN = lambda typ: f"E.panels.find(p => p.type === '{typ}' && p.tab === E.tab)"
        def zoom_once(typ, dy):
            c = pg.evaluate(f"(() => {{ const p = {PN(typ)}; p.el.scrollIntoView({{block: 'center'}}); const q = p.cv.getBoundingClientRect(); return [q.left + q.width * 0.45, q.top + q.height * 0.5, p.id]; }})()")
            frames(pg); ready(pg)
            c = pg.evaluate(f"(() => {{ const p = {PN(typ)}, q = p.cv.getBoundingClientRect(); return [q.left + q.width * 0.45, q.top + q.height * 0.5, p.id]; }})()")
            pg.mouse.move(c[0], c[1]); frames(pg); pg.evaluate(COUNT)
            ctrl_wheel(pg, 0, dy); frames(pg, 4); ready(pg)
            n = pg.evaluate(f"window.__dr.filter(i => i === {c[2]}).length")
            shot = pg.evaluate(f"{PN(typ)}.cv.toDataURL()")
            pg.evaluate(f"draw({PN(typ)})"); frames(pg); ready(pg)
            return n, shot, pg.evaluate(f"{PN(typ)}.cv.toDataURL()")
        for typ, nm in (("chrom", "TIC"), ("spec", "spectrum")):
            def one(typ=typ):
                for dy in (-200, -200, 200):
                    n, a, b = zoom_once(typ, dy)
                    assert n == 1, f"{n} draw() calls for one wheel step"
                    assert a == b, "the picture after the wheel step differs from a fresh draw (late margins or labels)"
            step(f"{nm}: one draw() per Ctrl+wheel step, same picture as a fresh draw", one)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
