"""«How it is extracted» in high resolution (synthetic Orbitrap file): the box is a few ppm wide; the features that a search tool (mzFinder) already holds can be
shown on the map as markers (switch OFF at the start, only in HR); a click on a marker puts the box on that feature (XIC, spectrum, numbers equal to the API)
and opens «How it was found» (window in ppm, peak with its edges, the filters with their thresholds), read-only. The provider here is a fake window.MAPFEAT."""
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
PROV = """window.MAPFEAT = { calls: 0, features(f) { this.calls++; return [
  { id: 'A1', mz: 305.0702, rt: 9.0, ppm: 5, filters: [{ key: 'h', label: 'Peak height', thr: 2e5, from: 'height', cmp: '>=' }, { key: 't', label: 'Treated files with the peak', value: 3, thr: 2, cmp: '>=' }] },
  { id: 'A2', mz: 305.1066, rt: 12.0, ppm: 5, rt0: 11.8, rt1: 12.25, filters: [{ key: 'f', label: 'Fold over the blanks', value: null, thr: 5, cmp: '>=', note: 'no signal in the blanks' }] }] } }"""
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
        feat_btn_hidden = lambda: pg.evaluate("document.querySelector('.pnl.map [data-nav=mapfeat]').hidden")
        def no_provider():
            assert feat_btn_hidden(), "no provider: no switch"
            assert pg.evaluate("!" + M + "._mz.feat")
        step("HR without a provider: no switch, nothing on the map", no_provider)
        def box_ppm():
            x, y = at(.5, .5); pg.mouse.move(x, y); pg.wait_for_timeout(300)
            pg.evaluate("MAPPA.explain(" + M + ", true)"); pg.wait_for_function("window.MAPEX && MAPEX.numbers(" + M + ")", timeout=15000); pg.wait_for_timeout(600)
            n = win(); assert 0 < n["win"]["mz1"] - n["win"]["mz0"] < 0.05, n["win"]
            o = pg.evaluate(CHECK)
            for k in ["area", "height", "rtMax", "mzMax"]:
                a, b = o["n"][k], o["api"][k]; assert (a is None and b is None) or abs(a - b) <= 1e-6 * max(1, abs(b)), (k, a, b)
            pg.click(".pnl.map .mzex [data-ex=off]"); pg.wait_for_timeout(400)
        step("HR: the box is a few ppm wide and its numbers are the ones of the API", box_ppm)
        def switch():
            pg.evaluate(PROV); pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(600)
            assert not feat_btn_hidden(), "with a provider the switch appears"
            assert pg.get_attribute(".pnl.map [data-nav=mapfeat]", "aria-pressed") == "false" and pg.evaluate("!" + M + "._mz.feat"), "off at the start"
            pg.click(".pnl.map [data-nav=mapfeat]"); pg.wait_for_timeout(500)
            assert pg.get_attribute(".pnl.map [data-nav=mapfeat]", "aria-pressed") == "true" and pg.evaluate(M + "._mz.feat")
            # the markers are on the map: the pixel of the first feature is a marker
            pt = pg.evaluate(f"(()=>{{const a={M}._a, r={M}.cv.getBoundingClientRect(); return [r.left+a.X(9.0), r.top+a.Y(305.0702), a.x0<9&&a.x1>9&&a.y0<305.07&&a.y1>305.07]}})()")
            assert pt[2], "the feature is inside the view"
        step("the switch appears with a provider, off at the start; turned on", switch)
        def click_marker():
            pt = pg.evaluate(f"(()=>{{const a={M}._a, r={M}.cv.getBoundingClientRect(); return [r.left+a.X(9.0), r.top+a.Y(305.0702)]}})()")
            pg.mouse.click(pt[0], pt[1]); pg.wait_for_function("window.MAPEX && MAPEX.numbers(" + M + ") && document.querySelector('.pnl.map .mzex-found')", timeout=15000); pg.wait_for_timeout(1200)
            n = win(); w = n["win"]; assert abs((w["mz0"] + w["mz1"]) / 2 - 305.0702) < 1e-3 and (w["mz1"] - w["mz0"]) < 0.01, w
            assert w["rt0"] < 9.0 < w["rt1"], w
            txt = pg.inner_text(".pnl.map .mzex-found"); assert "A1" in txt and "5 ppm" in txt and "Peak height" in txt and (tr("mapfeat.pass") in txt or tr("mapfeat.below") in txt), txt
            assert pg.evaluate("document.querySelector('.pnl.map .mzex-found canvas').width > 0")
            o = pg.evaluate(CHECK)
            for k in ["area", "height", "rtMax", "mzMax"]:
                a, b = o["n"][k], o["api"][k]; assert (a is None and b is None) or abs(a - b) <= 1e-6 * max(1, abs(b)), (k, a, b)
            assert o["n"]["height"] > 1e4, o["n"]
        step("a click on a marker: box on the feature, XIC, spectrum, numbers and «How it was found»", click_marker)
        def recorded_edges():
            pg.evaluate(M + ".el.scrollIntoView({block:'start'})"); pg.wait_for_timeout(600)
            pt = pg.evaluate(f"(()=>{{const a={M}._a, r={M}.cv.getBoundingClientRect(); return [r.left+a.X(12.0), r.top+a.Y(305.1066)]}})()")
            pg.mouse.click(pt[0], pt[1]); pg.wait_for_timeout(1500)
            w = win()["win"]; assert abs(w["rt0"] - 11.8) < 1e-6 and abs(w["rt1"] - 12.25) < 1e-6, w
            txt = pg.inner_text(".pnl.map .mzex-found"); assert "A2" in txt and "no signal in the blanks" in txt and tr("mapfeat.edges.rec", a="11.800", b="12.250") in txt, txt
        step("edges recorded by the provider are used as they are; a filter without a value says why", recorded_edges)
        def readonly():
            assert pg.evaluate("document.querySelectorAll('.pnl.map .mzex-found input, .pnl.map .mzex-found select').length") == 0, "read-only"
            assert pg.evaluate("window.MAPFEAT.calls") > 0
        step("«How it was found» is read-only", readonly)
        def off():
            pg.click(".pnl.map [data-nav=mapfeat]"); pg.wait_for_timeout(400)
            assert pg.evaluate("!" + M + "._mz.feat && !" + M + "._mz.fq && !document.querySelector('.pnl.map .mzex-found')")
        step("switch off: markers and the panel go away", off)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
