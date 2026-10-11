"""Drawing tab on a touch screen: ONE tap on an atom with the bond tool adds ONE bond (not two), and the floating bar of keys is not shown over the editor.
Real pointer events through CDP Input.dispatchTouchEvent."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tocco_disegno.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8852, wd="/tmp/wd_tocco_disegno")
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        ctx = r.b.new_context(viewport={"width": 1180, "height": 820}, has_touch=True)
        pg = ctx.new_page(); pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        cdp = ctx.new_cdp_session(pg)
        def T(kind, pts): cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": x, "y": y, "id": i} for i, (x, y) in enumerate(pts)]})
        def wait(ms=250): hold(pg, ms)
        K = "document.querySelector('#kframe').contentWindow.ketcher"
        nb = lambda: pg.evaluate(f"{K}.editor.struct().bonds.size")
        na = lambda: pg.evaluate(f"{K}.editor.struct().atoms.size")
        def tapatom():
            pg.click("#nav [data-v=draw]"); wait(7000)
            assert pg.evaluate("TOUCH._state().ketcher"), "the bridge reached the editor"
            pg.evaluate(f"(() => {{ {K}.editor.tool('bond', {{type: 1, stereo: 0}}); return 1; }})()"); wait(300)
            pg.evaluate("document.querySelector('#kframe').scrollIntoView({block: 'start'}); window.scrollBy(0, -60)"); wait(400)
            c = pg.evaluate("(() => { const r = document.querySelector('#kframe').getBoundingClientRect(); return [r.left + r.width * 0.5, 450]; })()")
            T("touchStart", [(c[0] - 80, c[1])]); wait(60)
            for k in range(1, 9): T("touchMove", [(c[0] - 80 + 15 * k, c[1] - 6 * k)]); wait(30)
            T("touchEnd", []); wait(500)
            assert (na(), nb()) == (2, 1), (na(), nb())
        step("molecule editor: a finger drag draws a first bond", tapatom)
        def one():
            # find the atom under the end point of the drag by the editor's own hit-test
            x, y = pg.evaluate("(() => { const r = document.querySelector('#kframe').getBoundingClientRect(); return [r.left + r.width * 0.5 - 80 + 15 * 8, 450 - 6 * 8]; })()")
            if os.environ.get("DBG"): pg.evaluate("(() => { const d = document.querySelector('#kframe').contentDocument; window.__ev = []; ['pointerdown','pointerup','mousedown','mouseup','click','touchstart','touchend'].forEach(t => d.addEventListener(t, e => window.__ev.push(t + ':' + e.isTrusted + ':' + (e.target.tagName||'')), true)); })()")
            b0 = nb(); T("touchStart", [(x, y)]); wait(40); T("touchEnd", []); wait(900)
            if os.environ.get("DBG"): print(pg.evaluate("window.__ev"))
            b1 = nb(); assert b1 == b0 + 1, f"one tap on an atom must add ONE bond, bonds {b0} -> {b1}"
        step("molecule editor: one tap on an atom adds one bond", one)
        def nobar():
            assert pg.locator("#tbar").count() == 0, "no floating bar of scan keys"
        step("there is no floating bar of scan keys", nobar)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
