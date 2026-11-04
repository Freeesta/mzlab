"""Draw canvas with a pen and a finger together: palm rejection for the whole stroke, a palm that rests before the pen, a pen that pauses (no long-press menu),
and the menu of a finger long press that stays open after the lift. Real pointer events through CDP: the pen is Input.dispatchMouseEvent with pointerType "pen",
the finger is Input.dispatchTouchEvent."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tocco_penna.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8853, wd="/tmp/wd_tocco_penna")
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        ctx = r.b.new_context(viewport={"width": 820, "height": 1180}, screen={"width": 1024, "height": 1366}, has_touch=True, is_mobile=True)
        pg = ctx.new_page(); pg.goto(f"http://127.0.0.1:{r.port}/"); frames(pg, 20)
        cdp = ctx.new_cdp_session(pg)
        def T(kind, pts): cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": x, "y": y, "id": i} for i, (x, y) in enumerate(pts)]})
        def PEN(kind, x, y):
            cdp.send("Input.dispatchMouseEvent", {"type": kind, "x": x, "y": y, "button": "left" if kind != "mouseMoved" or True else "none", "buttons": 0 if kind == "mouseReleased" else 1, "clickCount": 1, "pointerType": "pen"})
        def wait(ms=250): hold(pg, ms)
        K = "document.querySelector('#kframe').contentWindow.ketcher"
        nb = lambda: pg.evaluate(f"{K}.editor.struct().bonds.size")
        view = lambda: pg.evaluate(f"(() => {{ const e = {K}.editor; const o = e.render.options; return JSON.stringify([e.zoom(), o.scrollTop, o.scrollLeft, e.render.paper.canvas.getAttribute('viewBox')]); }})()")
        zoom = lambda: pg.evaluate(f"{K}.editor.zoom()")
        ctxvis = lambda: pg.evaluate("!document.querySelector('#ctx').hidden")
        def origin():
            pg.evaluate("document.querySelector('#kframe').scrollIntoView({block: 'start'}); window.scrollBy(0, -60)"); wait(400)
            return pg.evaluate("(() => { const r = document.querySelector('#kframe').getBoundingClientRect(); return [r.left, r.top, r.width, r.height]; })()")
        def fresh(smiles=""):
            pg.evaluate(f"(async () => {{ await {K}.setMolecule({smiles!r}); {K}.editor.tool('bond', {{type: 1, stereo: 0}}); {K}.editor.zoom(1); document.querySelector('#ctx').hidden = true; return 1; }})()"); wait(500)
        def setup():
            pg.click("#nav [data-v=draw]"); wait(7000)
            assert pg.evaluate("TOUCH._state().ketcher"), "the bridge reached the editor"
        step("molecule editor ready", setup)
        def finger_bond():
            fresh(); x, y, w, h = origin(); c = (x + w * 0.5, y + 250)
            T("touchStart", [(c[0] - 80, c[1])]); wait(60)
            for k in range(1, 9): T("touchMove", [(c[0] - 80 + 15 * k, c[1] - 6 * k)]); wait(30)
            T("touchEnd", []); wait(500)
            assert nb() == 1, nb()
        step("a finger drag still draws one bond", finger_bond)
        def pinch():
            fresh(); x, y, w, h = origin(); c = (x + w * 0.5, y + 300); z0 = zoom()
            T("touchStart", [(c[0] - 40, c[1]), (c[0] + 40, c[1])]); wait(60)
            for k in range(1, 12): T("touchMove", [(c[0] - 40 - 8 * k, c[1]), (c[0] + 40 + 8 * k, c[1])]); wait(40)
            T("touchEnd", []); wait(400)
            assert zoom() != z0, "two fingers still zoom"
        step("a two-finger pinch still zooms", pinch)
        def stroke_with_palm():
            fresh(); x, y, w, h = origin(); c = (x + w * 0.5, y + 250); v0 = view()
            PEN("mousePressed", c[0] - 80, c[1]); wait(40)
            for k in range(1, 13):
                PEN("mouseMoved", c[0] - 80 + 12 * k, c[1] - 5 * k); wait(100)
                if k == 10: T("touchStart", [(c[0] + 200, c[1] + 200)])               # the palm lands 1.0 s into the stroke
                if k > 10: T("touchMove", [(c[0] + 200 + 20 * (k - 10), c[1] + 200 - 10 * (k - 10))])
            PEN("mouseReleased", c[0] - 80 + 12 * 12, c[1] - 5 * 12); wait(100); T("touchEnd", []); wait(600)
            assert nb() == 1, f"bonds {nb()}"; assert zoom() == 1, zoom(); assert view() == v0, (view(), v0)
        step("(1) a palm touch during a long pen stroke is ignored", stroke_with_palm)
        def palm_first():
            fresh(); x, y, w, h = origin(); c = (x + w * 0.5, y + 250); v0 = view()
            T("touchStart", [(c[0] + 200, c[1] + 200)]); wait(700)
            PEN("mousePressed", c[0] - 80, c[1]); wait(40)
            for k in range(1, 9): PEN("mouseMoved", c[0] - 80 + 15 * k, c[1] - 6 * k); wait(30)
            PEN("mouseReleased", c[0] - 80 + 120, c[1] - 48); wait(100); T("touchEnd", []); wait(500)
            assert nb() == 1, f"bonds {nb()}"; assert zoom() == 1, zoom(); assert view() == v0; assert not ctxvis(), "no menu"
        step("(2) a palm resting before the pen does not break the stroke", palm_first)
        def pen_pause():
            fresh(); x, y, w, h = origin(); c = (x + w * 0.5, y + 250)
            pg.evaluate("window.__cm = 0; document.querySelector('#kframe').contentDocument.addEventListener('contextmenu', () => window.__cm++, true); document.addEventListener('contextmenu', () => window.__cm++, true)")
            PEN("mousePressed", c[0] - 80, c[1]); wait(700)
            for k in range(1, 9): PEN("mouseMoved", c[0] - 80 + 15 * k, c[1] - 6 * k); wait(30)
            PEN("mouseReleased", c[0] - 80 + 120, c[1] - 48); wait(500)
            assert nb() == 1, f"bonds {nb()}"; assert pg.evaluate("window.__cm") == 0, "no contextmenu"; assert not ctxvis()
        step("(3) a pen that pauses before moving still draws, no menu", pen_pause)
        def long_menu():
            fresh("CCO"); pg.evaluate(f"(() => {{ {K}.editor.tool('select'); return 1; }})()"); x, y, w, h = origin(); p0 = (x + w * 0.8, y + h * 0.8)
            pg.evaluate(f"(() => {{ window.__cp = 0; window.__ce = 0; try {{ navigator.clipboard.writeText = () => {{ window.__cp++; return Promise.resolve(); }}; }} catch (_) {{}}; const e = {K}.editor, f = e.centerStruct && e.centerStruct.bind(e); if (f) e.centerStruct = (...a) => {{ window.__ce++; return f(...a); }}; return 1; }})()")
            T("touchStart", [p0]); wait(800); T("touchEnd", []); wait(1000)
            assert ctxvis(), "the menu stays open after the lift"
            assert pg.evaluate("window.__cp") == 0 and pg.evaluate("window.__ce") == 0, "no row ran"
            top = pg.evaluate("(() => { const m = document.querySelector('#ctx'), r = m.getBoundingClientRect(); return [r.left, r.top, r.right, r.bottom]; })()")
            assert not (top[0] <= p0[0] <= top[2] and top[1] <= p0[1] <= top[3]), f"the finger is not over the menu {top} {p0}"
        step("(4) a finger long press leaves the menu open, no row runs", long_menu)
        def next_tap():
            T("touchStart", [(30, 30)]); wait(60); T("touchEnd", []); wait(600)
            assert not ctxvis(), "the next tap outside closes the menu"
        step("the next real tap still closes the menu", next_tap)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
