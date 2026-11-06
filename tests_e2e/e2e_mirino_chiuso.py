"""Closing a panel while its crosshair is shown: the line it put on the linked panels (same time axis) goes with it."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mirino_chiuso.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("mirino_chiuso")
r = Run(port=8883, wd="/tmp/wd_mirino_chiuso")
TIC = "E.panels.find(q => q.type === 'chrom' && q.tab === E.tab)"
XVL = "E.panels.filter(q => q.type === 'xic').map(q => !!q.vl && !q.vl.hidden)"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "B_FullMass-t0.mzML"], 6000, 1); ready(pg)
        pg.evaluate("openXic(null,{})"); pg.wait_for_selector("#xic-mz", state="visible"); pg.fill("#xic-mz", "364.4"); pg.click("#xic-go"); attendi(pg, "E.panels.some(q => q.type === 'xic')"); frames(pg, 4); ready(pg)

        def closed():
            pg.evaluate(f"{TIC}.el.scrollIntoView({{block: 'center'}})"); frames(pg)
            c = pg.evaluate(f"(() => {{ const r = {TIC}.cv.getBoundingClientRect(); return [r.left + r.width * 0.5, r.top + r.height * 0.5]; }})()")
            pg.mouse.move(c[0], c[1]); pg.mouse.move(c[0] + 3, c[1]); frames(pg, 3)
            assert pg.evaluate(XVL) == [True], f"the TIC crosshair is not on the XIC: {pg.evaluate(XVL)}"
            pg.evaluate(f"closePanel({TIC})"); frames(pg, 3)                    # closed without the pointer leaving it (keyboard, menu)
            assert pg.evaluate(XVL) == [False], "the line of the closed TIC stays on the XIC"
        step("closing the hovered TIC removes its line from the XIC", closed)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
