"""«How it is extracted» on the RT x m/z map (mappa-estrai.js): a box on the map gives the XIC of its m/z window and the mean spectrum of its RT window;
the numbers under the map are the ones of /api/xic and /api/spectrum; the box moves and resizes (mouse, arrows, finger) and the plots and numbers follow;
in low resolution nothing is labelled by the program. LR example files."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_estrai.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8895, wd="/tmp/wd_mappa_estrai")
M = "E.panels.find(q=>q.type==='map')"
CHECK = """async () => {
  const p = %s, n = MAPEX.numbers(p), f = p._a.f, w = n.win, c = (w.mz0 + w.mz1) / 2, tol = (w.mz1 - w.mz0) / 2;
  const xj = await (await fetch(`api/xic?k=${f.k}&mz=${c}&tol=${tol}&level=${f.lv || 1}`)).json(), sj = await (await fetch(`api/spectrum?k=${f.k}&rt0=${w.rt0}&rt1=${w.rt1}&level=${f.lv || 1}&precursor=&bin=0.1`)).json();
  const t = xj.traces[0]; let ar = 0, mx = -1, at = -1, prev = -1;
  for (let i = 0; i < t.rt.length; i++) if (t.rt[i] >= w.rt0 && t.rt[i] <= w.rt1) { if (t.y[i] > mx) { mx = t.y[i]; at = t.rt[i]; } if (prev >= 0) ar += (t.y[i] + t.y[prev]) / 2 * (t.rt[i] - t.rt[prev]) * 60; prev = i; }
  let sm = -1, mzm = null; for (let i = 0; i < sj.mz.length; i++) if (sj.mz[i] >= w.mz0 && sj.mz[i] <= w.mz1 && sj.y[i] > sm) { sm = sj.y[i]; mzm = sj.mz[i]; }
  return { n, api: { area: ar, height: mx, rtMax: at, mzMax: mzm }, shown: Object.fromEntries([...document.querySelectorAll('.pnl.map .mzex output')].map(o => [o.dataset.k, o.textContent])) };
}""" % M
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        ctx = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}, has_touch=True)
        pg = ctx.new_page()
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")]); pg.wait_for_timeout(800)
        pg.click("#opbtn"); ready(pg)
        pg.click("#np-map"); ready(pg); pg.wait_for_timeout(600)
        pg.evaluate("document.querySelectorAll('#tour-invito button').forEach(b => b.textContent.trim() === 'Not now' && b.click())")      # the invitation to the tour would cover the page
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); pg.wait_for_timeout(300)
        box = lambda: pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
        at = lambda fx, fy: (lambda c: (c[0] + c[2] * fx, c[1] + c[3] * fy))(box())
        win = lambda: pg.evaluate("MAPEX.numbers(" + M + ")")
        def nothing_before():
            assert pg.evaluate(f"!{M}._mz.ex && !document.querySelector('.pnl.map .mzex')"), "off at the start"
            assert not pg.evaluate(M + "._mz.ex")
        step("off at the start; the bar has the button", nothing_before)
        def on():
            x, y = at(.4, .5); pg.mouse.move(x, y); pg.wait_for_timeout(300)
            pg.evaluate("MAPPA.explain(" + M + ", true)"); pg.wait_for_function("window.MAPEX && MAPEX.numbers(" + M + ")", timeout=15000); pg.wait_for_timeout(500)
            assert pg.evaluate("!!" + M + "._mz.ex")
            assert pg.evaluate(f"!{M}.el.querySelector('.mzside').hidden && !{M}.el.querySelector('.mzbot').hidden"), "the two plots at the margins are on"
            n = win(); assert n["win"]["mz1"] - n["win"]["mz0"] > 0.5 and n["win"]["rt1"] > n["win"]["rt0"], n
            assert pg.inner_text(".pnl.map .mzex").count("Σ") >= 1 and "XIC" in pg.inner_text(".pnl.map .mzex")
            assert pg.evaluate("document.querySelector('.pnl.map .mzex').getAttribute('role')") == "region"
        step("turning it on: box at the pointer, XIC, spectrum, numbers and the rule", on)
        def numbers_match():
            for _ in range(2):
                o = pg.evaluate(CHECK); n, a = o["n"], o["api"]
                for k in ["area", "height", "rtMax", "mzMax"]:
                    assert n[k] is not None and abs(n[k] - a[k]) <= 1e-6 * max(1, abs(a[k])), (k, n[k], a[k])
                assert o["shown"]["height"] and o["shown"]["rtmax"], o["shown"]
                pg.evaluate(M + ".cv.focus()")      # arrows with the focus elsewhere change file
                pg.keyboard.press("ArrowRight"); pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(700)
        step("the numbers are the ones of /api/xic and /api/spectrum for the window of the box", numbers_match)
        def move_keys():
            pg.evaluate(M + ".cv.focus()"); w0 = win()["win"]
            pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(700); w1 = win()["win"]; assert w1["rt0"] > w0["rt0"] and w1["mz0"] == w0["mz0"], (w0, w1)
            pg.keyboard.press("ArrowUp"); pg.wait_for_timeout(700); w2 = win()["win"]; assert w2["mz0"] > w1["mz0"], (w1, w2)
            pg.keyboard.press("Alt+ArrowRight"); pg.wait_for_timeout(700); w3 = win()["win"]; assert (w3["rt1"] - w3["rt0"]) > (w2["rt1"] - w2["rt0"]), (w2, w3)
        step("arrows move the box, Alt + arrows widen it; numbers follow", move_keys)
        def drag_mouse():
            c = pg.evaluate(f"(()=>{{const a={M}._a, b={M}._mz.ex.box, r={M}.cv.getBoundingClientRect(); return [r.left+a.X((b.rt0+b.rt1)/2), r.top+a.Y((b.mz0+b.mz1)/2), r.left+a.X(b.rt1), r.top+a.Y((b.mz0+b.mz1)/2)]}})()")
            w0 = win()["win"]
            pg.mouse.move(c[0], c[1]); ready(pg)      # the pointer entering the map shifts the layout: the box is read again where it is now
            c = pg.evaluate(f"(()=>{{const a={M}._a, b={M}._mz.ex.box, r={M}.cv.getBoundingClientRect(); return [r.left+a.X((b.rt0+b.rt1)/2), r.top+a.Y((b.mz0+b.mz1)/2)]}})()")
            pg.mouse.move(c[0], c[1]); pg.mouse.down(); pg.mouse.move(c[0] + 60, c[1] - 20, steps=5); pg.mouse.up(); pg.wait_for_timeout(800)
            w1 = win()["win"]; assert w1["rt0"] > w0["rt0"] and w1["mz0"] > w0["mz0"] and abs((w1["rt1"] - w1["rt0"]) - (w0["rt1"] - w0["rt0"])) < 1e-9, (w0, w1)
            c = pg.evaluate(f"(()=>{{const a={M}._a, b={M}._mz.ex.box, r={M}.cv.getBoundingClientRect(); return [r.left+a.X(b.rt1), r.top+a.Y((b.mz0+b.mz1)/2)]}})()")
            pg.mouse.move(c[0], c[1]); pg.mouse.down(); pg.mouse.move(c[0] + 40, c[1], steps=5); pg.mouse.up(); pg.wait_for_timeout(800)
            w2 = win()["win"]; assert w2["rt1"] > w1["rt1"] and w2["rt0"] == w1["rt0"], (w1, w2)
            o = pg.evaluate(CHECK); assert abs(o["n"]["area"] - o["api"]["area"]) <= 1e-6 * max(1, o["api"]["area"]), o
        step("drag the box and its right edge with the mouse; the numbers still match the API", drag_mouse)
        def menu_item():
            pg.evaluate(M + ".el.scrollIntoView({block:'start'})"); pg.wait_for_timeout(200)
            x, y = at(.2, .3); pg.mouse.click(x, y, button="right"); pg.wait_for_timeout(250)
            its = pg.evaluate("[...document.querySelectorAll('#ctx [role^=menuitem]')].map(d=>d.textContent.trim())")
            assert not any("extracted" in i.lower() or "both" in i.lower() for i in its), its                 # the teaching view is not in the menu: it has its own button
            pg.keyboard.press("Escape")
            assert pg.evaluate("!!" + M + "._mz.ex")
            assert not pg.evaluate("!!document.querySelector('.pnl.map [data-nav=mapexplain]')")                 # no button for it any more
        step("the box opens from MAPPA.explain; there is no button and the menu does not carry it", menu_item)
        def touch_drag():
            cdp = ctx.new_cdp_session(pg)
            c = pg.evaluate(f"(()=>{{const a={M}._a, b={M}._mz.ex.box, r={M}.cv.getBoundingClientRect(); return [r.left+a.X((b.rt0+b.rt1)/2), r.top+a.Y((b.mz0+b.mz1)/2)]}})()")
            w0 = win()["win"]; z0 = pg.evaluate(f"[{M}.zoom,{M}.zoomY]")
            cdp.send("Input.dispatchTouchEvent", {"type": "touchStart", "touchPoints": [{"x": c[0], "y": c[1], "id": 0}]})
            for k in range(1, 7): cdp.send("Input.dispatchTouchEvent", {"type": "touchMove", "touchPoints": [{"x": c[0] + k * 8, "y": c[1], "id": 0}]}); pg.wait_for_timeout(30)
            cdp.send("Input.dispatchTouchEvent", {"type": "touchEnd", "touchPoints": []}); pg.wait_for_function("1", timeout=1000); pg.wait_for_timeout(700)
            w1 = win()["win"]; assert w1["rt0"] > w0["rt0"], (w0, w1); assert pg.evaluate(f"[{M}.zoom,{M}.zoomY]") == z0, "the map did not move"
        step("a finger on the box moves the box, not the map", touch_drag)
        def no_labels():
            pg.evaluate("window.MAPFEAT = { features: () => [{ id: 1, mz: 259, rt: 8, ppm: 5 }] }"); pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"!{M}._mz.feat && document.querySelector('.pnl.map [data-nav=mapfeat]').hidden"), "LR: no automatic marker, no switch for the features of a search, even if a provider exists"
            pg.evaluate("delete window.MAPFEAT")
            txt = pg.inner_text(".pnl.map .mzex"); import re
            assert not re.search(r"(?i)product|\bTP\b|candidate|identified", txt), txt
        step("low resolution: no marker and no label from the program", no_labels)
        def off():
            pg.click(".pnl.map .mzex [data-ex=off]"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"!{M}._mz.ex && !document.querySelector('.pnl.map .mzex')")
            assert not pg.evaluate(M + "._mz.ex")
            assert pg.evaluate(f"{M}.el.querySelector('.mzside').hidden")
        step("close: the view returns to the plain map", off)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
