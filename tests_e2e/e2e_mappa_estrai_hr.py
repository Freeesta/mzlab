"""«How it is extracted» in high resolution (synthetic Orbitrap file): the box is a few ppm wide and its numbers are the ones of the API; the map has no
switch for features of a search tool (the Tabui page that fed it was deleted)."""
# covers: mzlab/web/mappa.js, mzlab/web/mappa-estrai.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_estrai_hr.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
M = "E.panels.find(q=>q.type==='map')"
D = synth("estrai_hr")
r = Run(port=8894, wd="/tmp/wd_mappa_estrai_hr")
CHECK = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "e2e_mappa_estrai.py")).read().split('CHECK = """')[1].split('""" % M')[0] % M
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 950})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        pg.evaluate("document.querySelectorAll('#tour-invito button').forEach(b => b.textContent.trim() === 'Not now' && b.click())")
        pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(500)
        pg.evaluate("reveal(addPanel('map', { k: E.files.find(f => f.kind === 'full').k }))"); ready(pg); pg.wait_for_timeout(1500)
        pg.evaluate(M + ".el.scrollIntoView({block:'start'})"); pg.wait_for_timeout(300)
        box = lambda: pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
        at = lambda fx, fy: (lambda c: (c[0] + c[2] * fx, c[1] + c[3] * fy))(box())
        win = lambda: pg.evaluate("MAPEX.numbers(" + M + ")")
        tr = lambda k, **kw: pg.evaluate("([k, o]) => I18N.t(k, o)", [k, kw])
        def no_switch():
            assert pg.evaluate("document.querySelector('.pnl.map [data-nav=mapfeat]') === null"), "the map has no switch for search-tool features"
        step("HR: no switch for search-tool features on the map", no_switch)
        def box_ppm():
            x, y = at(.5, .5); pg.mouse.move(x, y); pg.wait_for_timeout(300)
            pg.evaluate("MAPPA.explain(" + M + ", true)"); pg.wait_for_function("window.MAPEX && MAPEX.numbers(" + M + ")", timeout=15000); pg.wait_for_timeout(600)
            n = win(); assert 0 < n["win"]["mz1"] - n["win"]["mz0"] < 0.05, n["win"]
            o = pg.evaluate(CHECK)
            for k in ["area", "height", "rtMax", "mzMax"]:
                a, b = o["n"][k], o["api"][k]; assert (a is None and b is None) or abs(a - b) <= 1e-6 * max(1, abs(b)), (k, a, b)
            pg.click(".pnl.map .mzex [data-ex=off]"); pg.wait_for_timeout(400)
        step("HR: the box is a few ppm wide and its numbers are the ones of the API", box_ppm)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
