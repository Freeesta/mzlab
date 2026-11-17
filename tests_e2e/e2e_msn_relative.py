"""Spectrum cards: an MSn spectrum (level >= 2) opens with the y axis in % of the base peak, MS1 stays absolute, and a card the user switched
to the other scale keeps it across redraws, zoom and a change of level."""
# covers: mzlab/web/spettro.js, mzlab/web/pannelli.js, mzlab/web/picchi.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_msn_relative.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
S1 = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
S2 = "E.panels.find(p=>p.type==='spec'&&p.tab==='ms2')"
BTN = "[data-a=rel]"
r = Run(port=8880, wd="/tmp/wd80")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_MS2-t15")])
        pg.click("text=Load data"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        def ms1_abs():
            assert pg.evaluate(f"{S1}.level") == 1 and pg.evaluate(f"{S1}.rel") is None
            assert pg.evaluate(f"!{S1}.el.querySelector('{BTN}').classList.contains('on')"), "MS1: % button off"
            assert pg.evaluate(f"{S1}._a.yticks.some(v=>v>100)"), "MS1 axis is absolute"
        step("MS1 opens in absolute intensity", ms1_abs)
        open_kind(pg, "ms2")
        wait_for_js(pg, f"(()=>{{const p={S2};return !!p&&!!p._a&&p._a.data[0].d.mz.length>0}})()")
        def ms2_rel():
            assert pg.evaluate(f"{S2}.level") == 2 and pg.evaluate(f"{S2}.rel") is None, "no explicit choice stored"
            assert pg.evaluate(f"{S2}.el.querySelector('{BTN}').classList.contains('on')"), "MS2: % button on"
            t = pg.evaluate(f"{S2}._a.yticks"); assert t and max(t) == 100, t
        step("MS2 opens in % of the base peak (0-100)", ms2_rel)
        def exact_range():
            a = pg.evaluate(f"(()=>{{const a={S2}._a;return {{ymax:a.ymax,ymin:a.ymin,top:a.Y(100)-M.t,base:Math.max(...a.data[0].d.y),yf:a.yfull}}}})()")
            assert a["ymax"] == 100 and a["ymin"] == 0 and abs(a["top"]) <= 1 and a["base"] == 100 and a["yf"][1] == 100, a
            pg.evaluate(f"(()=>{{const p={S2};p.zoom=[p._a.x0+5,p._a.x1-5];draw(p)}})()"); frames(pg, 3)
            assert pg.evaluate(f"{S2}._a.ymax") == 100, "zoom keeps 0-100"
            pg.evaluate(f"(()=>{{const p={S2};p.zoom=null;draw(p)}})()"); frames(pg, 3)
            assert pg.evaluate(f"{S2}._a.ymax") == 100, "reset keeps 0-100"
        step("in %: axis 0-100 exactly, base peak touches the top (also after zoom and reset)", exact_range)
        def stays_abs():
            pg.evaluate(f"{S2}.el.querySelector('{BTN}').click()"); frames(pg, 3)
            assert pg.evaluate(f"{S2}.rel") is False and pg.evaluate(f"!{S2}.el.querySelector('{BTN}').classList.contains('on')")
            assert pg.evaluate(f"{S2}._a.yticks.some(v=>v>100)"), "axis absolute after the toggle"
            pg.evaluate(f"(()=>{{const p={S2};p.zoom=[p._a.x0+5,p._a.x1-5];draw(p)}})()"); frames(pg, 3)
            assert pg.evaluate(f"{S2}.rel") is False and pg.evaluate(f"{S2}._a.yticks.some(v=>v>100)"), "zoom keeps absolute"
            pg.evaluate(f"(()=>{{const p={S2};p.zoom=null;draw(p)}})()"); frames(pg, 3)
            assert pg.evaluate(f"{S2}.rel") is False, "a redraw keeps the user's choice"
        step("MS2 switched to absolute stays absolute through zoom and redraw", stays_abs)
        def back():
            pg.evaluate(f"{S2}.el.querySelector('{BTN}').click()"); frames(pg, 3)
            assert pg.evaluate(f"{S2}.rel") is True and max(pg.evaluate(f"{S2}._a.yticks")) == 100
        step("toggling again returns to %", back)
        errs = [e for e in r.errs if "iframe" not in str(e)]
        print("--- BROWSER ERRORS", len(errs)); [print(e) for e in errs[:5]]
finally: r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
