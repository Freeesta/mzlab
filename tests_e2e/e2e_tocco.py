"""Touch and pen (touch.js): one finger / pen = the mouse, long press = right click, double tap, two-finger pinch / pan / scroll, pen hover, palm rejection,
the bar of keys, the grip for the height, 3D rotation. The touches are real pointer events (CDP Input.dispatchTouchEvent / pen mouse events)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tocco.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8848, wd="/tmp/wd_tocco")
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        ctx = r.b.new_context(viewport={"width": 1280, "height": 900}, has_touch=True)
        pg = ctx.new_page(); pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        cdp = ctx.new_cdp_session(pg)
        def T(kind, pts): cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": x, "y": y, "id": i} for i, (x, y) in enumerate(pts)]})
        def pen(kind, x, y, **kw): cdp.send("Input.dispatchMouseEvent", {"type": kind, "x": x, "y": y, "button": "left" if kind != "mouseMoved" or kw.get("buttons") else "none", "buttons": kw.get("buttons", 0), "clickCount": 1 if kind != "mouseMoved" else 0, "pointerType": "pen"})
        def wait(ms=250): hold(pg, ms)        # gestures are timed (long press, double tap): real pauses
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); wait(1000)
        pg.click("text=Carica dati"); wait(4500)
        pg.evaluate("setTab('full',true)"); wait(800)
        CH = "E.panels.find(p => p.type === 'chrom' && p.tab === 'full')"
        pg.evaluate(f"{CH}.el.scrollIntoView({{block:'start'}})"); wait(400)
        geo = lambda: pg.evaluate(f"(() => {{ const p = {CH}, q = p.cv.getBoundingClientRect(), a = p._a; return {{l: q.left, t: q.top, w: q.width, h: q.height, x: rt => q.left + a.X(rt), y: q.top + q.height * 0.4, x0: a.x0, x1: a.x1}}; }})()")
        def X(rt): return pg.evaluate(f"(() => {{ const p = {CH}, q = p.cv.getBoundingClientRect(); return q.left + p._a.X({rt}); }})()")
        def Ymid(): return pg.evaluate(f"(() => {{ const q = {CH}.cv.getBoundingClientRect(); return q.top + q.height * 0.4; }})()")
        def tap():
            x, y = X(14.3), Ymid(); T("touchStart", [(x, y)]); T("touchEnd", []); wait(900)
            c = pg.evaluate(f"{CH}.cur"); assert c is not None and abs(c - 14.3) < 0.25, c
            assert pg.evaluate("TOUCH.seen") and pg.evaluate("document.documentElement.classList.contains('touch')")
        step("one finger tap sets the cursor (as a click) and switches the touch mode on", tap)
        def drag():
            x0, x1, y = X(13), X(15), Ymid(); T("touchStart", [(x0, y)]); wait(80)
            for k in range(1, 9): T("touchMove", [(x0 + (x1 - x0) * k / 8, y)]); wait(30)
            T("touchEnd", []); wait(500)
            s = pg.evaluate(f"{CH}.sel"); assert s and abs(s[0] - 13) < 0.3 and abs(s[1] - 15) < 0.3, s
        step("one finger drag selects an interval", drag)
        def zoomtool():
            pg.evaluate(f"{CH}.el.querySelector('[data-a=izoom]').click()"); wait(200)
            x0, x1, y = X(12), X(16), Ymid(); T("touchStart", [(x0, y)]); wait(60)
            for k in range(1, 9): T("touchMove", [(x0 + (x1 - x0) * k / 8, y + 20)]); wait(30)
            T("touchEnd", []); wait(500)
            z = pg.evaluate(f"{CH}.zoom"); assert z and abs(z[0] - 12) < 0.3 and abs(z[1] - 16) < 0.3, z
            pg.evaluate(f"{CH}.zoom = null; {CH}.zoomY = null; {CH}.imode = null; draw({CH})"); wait(300)
        step("the zoom tool works with a finger", zoomtool)
        def pinch():
            g = geo(); cx = g["l"] + g["w"] / 2; y = Ymid(); w0 = pg.evaluate(f"(() => {{ const a = {CH}._a; return a.x1 - a.x0; }})()")
            T("touchStart", [(cx - 60, y), (cx + 60, y)]); wait(60)
            for k in range(1, 9): T("touchMove", [(cx - 60 - 12 * k, y), (cx + 60 + 12 * k, y)]); wait(30)
            T("touchEnd", []); wait(500)
            w1 = pg.evaluate(f"(() => {{ const a = {CH}._a; return a.x1 - a.x0; }})()"); assert w1 < w0 * 0.7, (w0, w1)
            T("touchStart", [(cx - 160, y), (cx + 160, y)]); wait(60)
            for k in range(1, 9): T("touchMove", [(cx - 160 + 12 * k, y), (cx + 160 - 12 * k, y)]); wait(30)
            T("touchEnd", []); wait(500)
            w2 = pg.evaluate(f"(() => {{ const a = {CH}._a; return a.x1 - a.x0; }})()"); assert w2 > w1 * 1.3, (w1, w2)
        step("pinch out zooms in, pinch in zooms out", pinch)
        def pan():
            g = geo(); pg.evaluate(f"zoomTo({CH}, 10, 14)"); wait(400); a0 = pg.evaluate(f"{CH}._a.x0"); cx = g["l"] + g["w"] / 2; y = Ymid()
            T("touchStart", [(cx - 40, y), (cx + 40, y)]); wait(60)
            for k in range(1, 9): T("touchMove", [(cx - 40 + 15 * k, y), (cx + 40 + 15 * k, y)]); wait(30)
            T("touchEnd", []); wait(500)
            a1 = pg.evaluate(f"{CH}._a.x0"); assert a1 < a0 - 0.2, (a0, a1)                    # dragging to the right shows earlier times
            pg.evaluate(f"{CH}.zoom = null; draw({CH})"); wait(300)
        step("two fingers sideways pan the graph", pan)
        def scroll():
            for sel in ("#np-spec", "#np-chrom", "#np-spec"):
                if pg.evaluate("document.documentElement.scrollHeight") < 1700: pg.click(sel); wait(900)
            pg.evaluate("window.scrollTo(0, 0)"); wait(200); g = geo(); cx = g["l"] + g["w"] / 2; y = max(g["t"], 150) + 60; z0 = pg.evaluate(f"{CH}.zoom")
            T("touchStart", [(cx - 40, y), (cx + 40, y)]); wait(60)
            for k in range(1, 9): T("touchMove", [(cx - 40, y - 30 * k), (cx + 40, y - 30 * k)]); wait(30)
            T("touchEnd", []); wait(400)
            assert pg.evaluate("scrollY") > 100 and pg.evaluate(f"{CH}.zoom") == z0, (pg.evaluate("scrollY"), pg.evaluate(f"{CH}.zoom"))
            pg.evaluate(f"{CH}.el.scrollIntoView({{block:'start'}})"); wait(300)
        step("two fingers up / down scroll the page (nothing else moves)", scroll)
        def longpress():
            x, y = X(8), Ymid(); T("touchStart", [(x, y)]); wait(800); T("touchEnd", []); wait(300)
            assert pg.is_visible("#ctx") and "Spettro a questo RT" in pg.inner_text("#ctx"), pg.inner_text("#ctx")
            pg.keyboard.press("Escape"); pg.mouse.click(2, 2); wait(200)
        step("long press opens the menu of the graph (right click)", longpress)
        def dtap():
            n0 = pg.evaluate("E.panels.filter(p => p.type === 'spec').length"); x, y = X(14.3), Ymid()
            T("touchStart", [(x, y)]); T("touchEnd", []); wait(120); T("touchStart", [(x, y)]); T("touchEnd", []); wait(900)
            n1 = pg.evaluate("E.panels.filter(p => p.type === 'spec').length"); assert n1 == n0 + 1, (n0, n1)
        step("double tap = double click (a new spectrum)", dtap)
        def penhover():
            pg.evaluate(f"{CH}.el.scrollIntoView({{block:'start'}})"); wait(300); x, y = X(10), Ymid()
            pen("mouseMoved", x, y); wait(200); pen("mouseMoved", x + 20, y); wait(300)
            assert pg.evaluate(f"!{CH}.tip.hidden || !{CH}.vl.hidden"), "hover information under the pen"
            pen("mousePressed", x, y, buttons=1); wait(50)
            for k in range(1, 7): pen("mouseMoved", x + 20 * k, y, buttons=1); wait(30)
            pen("mouseReleased", x + 120, y); wait(400)
            s = pg.evaluate(f"{CH}.sel"); assert s and s[1] > s[0], s
        step("pen: hover shows the information, a drag acts as the mouse", penhover)
        def palm():
            pg.evaluate(f"{CH}.sel = null; draw({CH})"); x, y = X(6), Ymid()
            pen("mouseMoved", x, y); wait(100)
            T("touchStart", [(x - 80, y)]); wait(50)
            for k in range(1, 6): T("touchMove", [(x - 80 + 30 * k, y)]); wait(20)
            T("touchEnd", []); wait(400)
            assert pg.evaluate(f"{CH}.sel") is None, "a palm next to the pen did nothing"
            wait(1100); pen("mouseMoved", 5, 5); wait(1100)
        step("palm rejection: a touch while the pen is near is ignored", palm)
        def keybar():
            assert pg.is_visible("#tbar") and pg.locator("#tbar button").count() == 6
            c0 = pg.evaluate(f"E.active && E.active.cur"); x, y = X(12), Ymid(); T("touchStart", [(x, y)]); T("touchEnd", []); wait(500)
            c0 = pg.evaluate(f"{CH}.cur"); pg.evaluate(f"setActive({CH})")
            b = pg.locator('#tbar [data-k=next]').bounding_box(); bx, by = b["x"] + b["width"] / 2, b["y"] + b["height"] / 2
            T("touchStart", [(bx, by)]); wait(700); T("touchEnd", []); wait(300)
            c1 = pg.evaluate(f"{CH}.cur"); assert c1 > c0 + 0.1, (c0, c1)                      # held: it keeps stepping
            b = pg.locator('#tbar [data-k=fit]').bounding_box(); pg.evaluate(f"zoomTo({CH}, 10, 14)"); wait(300)
            T("touchStart", [(b["x"] + 20, b["y"] + 20)]); T("touchEnd", []); wait(400)
            assert pg.evaluate(f"{CH}.zoom") is None, "the bar's whole-view key"
        step("the bar of keys: hold next to step through the scans, whole view", keybar)
        def fold():
            pg.click("#tbar .tg"); wait(200)
            assert pg.evaluate("document.querySelector('#tbar').classList.contains('min')") and pg.locator("#tbar [data-k=next]").is_hidden(), "folded to one small button"
            pg.click("#tbar .tg"); wait(200)
            assert pg.locator("#tbar [data-k=next]").is_visible(), "unfolded"
        step("the bar of keys folds away to a small button and back", fold)
        def grip():
            h0 = pg.evaluate(f"{CH}.h"); b = pg.evaluate(f"(() => {{ const r = {CH}.el.querySelector('.rzg').getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; }})()")
            T("touchStart", [(b[0], b[1])]); wait(60)
            for k in range(1, 7): T("touchMove", [(b[0], b[1] + 20 * k)]); wait(30)
            T("touchEnd", []); wait(500)
            h1 = pg.evaluate(f"{CH}.h"); assert h1 > h0 + 60, (h0, h1)
        step("the grip changes the height of a panel", grip)
        def filelong():
            b = pg.locator("#flst .fl:not(.ghost) .nm").first.bounding_box(); x, y = b["x"] + 10, b["y"] + b["height"] / 2
            T("touchStart", [(x, y)]); wait(800); T("touchEnd", []); wait(300)
            assert pg.is_visible("#ctx") and "Rinomina" in pg.inner_text("#ctx"), pg.inner_text("#ctx")
            pg.keyboard.press("Escape"); pg.mouse.click(2, 2); wait(200)
        step("long press on a file of the list = right click", filelong)
        def map3d():
            pg.click("#np-map"); wait(2500)
            M3 = "E.panels.find(p => p.type === 'map')"
            pg.evaluate(f"{M3}.el.scrollIntoView({{block:'center'}})"); wait(400)
            pg.locator(".pnl.map [data-o=view][data-v='3d']").click()
            pg.wait_for_function("window.__m3d && window.__m3d.V.g && window.__m3d.V.Z", timeout=60000); wait(500)
            c = pg.evaluate("(() => { const q = document.querySelector('.m3d-gl').getBoundingClientRect(); return [q.left + q.width / 2, q.top + q.height / 2]; })()"); az0 = pg.evaluate("window.__m3d.V.cam.az")
            T("touchStart", [(c[0], c[1])]); wait(60)
            for k in range(1, 9): T("touchMove", [(c[0] + 25 * k, c[1])]); wait(30)
            T("touchEnd", []); wait(500)
            az1 = pg.evaluate("window.__m3d.V.cam.az"); assert abs(az1 - az0) > 20, (az0, az1)
            pg.keyboard.press("Escape"); wait(300)
        step("3D map: one finger rotates", map3d)
        K = "document.querySelector('#kframe').contentWindow.ketcher"
        def kfinger():
            pg.click("#nav [data-v=draw]"); wait(7000)
            assert pg.evaluate("TOUCH._state().ketcher"), "the bridge reached the editor"
            pg.evaluate(f"(() => {{ {K}.editor.tool('bond', {{type: 1, stereo: 0}}); return 1; }})()"); wait(300)
            pg.evaluate("document.querySelector('#kframe').scrollIntoView({block: 'start'}); window.scrollBy(0, -60)"); wait(400)
            c = pg.evaluate("(() => { const r = document.querySelector('#kframe').getBoundingClientRect(); return [r.left + r.width * 0.5, 450]; })()")
            n0 = pg.evaluate(f"{K}.editor.struct().atoms.size"); assert n0 == 0, n0
            T("touchStart", [(c[0] - 80, c[1])]); wait(60)
            for k in range(1, 9): T("touchMove", [(c[0] - 80 + 15 * k, c[1] - 6 * k)]); wait(30)
            T("touchEnd", []); wait(500)
            assert pg.evaluate(f"{K}.editor.struct().atoms.size") == 2, "a finger drag draws a bond"
        step("molecule editor: a finger drag draws a bond", kfinger)
        def kpen():
            c = pg.evaluate("(() => { const r = document.querySelector('#kframe').getBoundingClientRect(); return [r.left + r.width * 0.5, 570]; })()")
            pen("mouseMoved", c[0], c[1]); wait(100); pen("mousePressed", c[0], c[1], buttons=1); wait(50)
            for k in range(1, 9): pen("mouseMoved", c[0] + 15 * k, c[1] + 5 * k, buttons=1); wait(30)
            pen("mouseReleased", c[0] + 120, c[1] + 40); wait(500)
            assert pg.evaluate(f"{K}.editor.struct().atoms.size") == 4, "the pen draws a second bond"
        step("molecule editor: the pen draws", kpen)
        def kpinch():
            c = pg.evaluate("(() => { const r = document.querySelector('#kframe').getBoundingClientRect(); return [r.left + r.width * 0.5, 300]; })()")
            z0 = pg.evaluate(f"{K}.editor.zoom()")
            T("touchStart", [(c[0] - 50, c[1]), (c[0] + 50, c[1])]); wait(60)
            for k in range(1, 9): T("touchMove", [(c[0] - 50 - 10 * k, c[1]), (c[0] + 50 + 10 * k, c[1])]); wait(30)
            T("touchEnd", []); wait(500)
            z1 = pg.evaluate(f"{K}.editor.zoom()"); assert z1 > z0 * 1.3, (z0, z1)
        step("molecule editor: pinch zooms the drawing", kpinch)
        def kundo():
            pg.evaluate(f"(() => {{ try {{ {K}.editor.undo(); {K}.editor.undo(); }} catch (e) {{}} return 1; }})()"); wait(300)
        step("molecule editor: (cleanup)", kundo)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
