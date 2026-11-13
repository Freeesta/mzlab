"""Draw: Ketcher's clipboard textarea is not focused before the first gesture in the frame (Chromium reports that as a policy violation), and is after it."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
r = Run(port=8997, wd="/tmp/wd_disegno_focus")
fail = []
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.goto(f"http://127.0.0.1:{r.port}/#/draw")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        T = ("(() => { const d = document.querySelector('#kframe').contentDocument; let t = d.querySelector('textarea.cliparea');"
             " if (!t) { t = d.createElement('textarea'); t.className = 'cliparea'; d.body.appendChild(t); } t.blur(); return new Promise(res => d.defaultView.setTimeout(() => { t.focus(); res(d.activeElement === t); }, 50)); })()")
        # Playwright's evaluate counts as a user gesture; a raw CDP call does not
        cdp = pg.context.new_cdp_session(pg)
        ev = lambda x: cdp.send("Runtime.evaluate", {"expression": x, "awaitPromise": True, "returnByValue": True, "userGesture": False})["result"].get("value")
        pg.reload()        # a new document starts without user activation (the first visit's evaluate calls of the harness gave it)
        for _ in range(240):
            if ev("!!(window.TPDraw && TPDraw.ready())"): break
            time.sleep(0.25)
        if ev(T): fail.append("the clipboard textarea took focus before any gesture")
        box = pg.locator("#kframe").bounding_box(); pg.mouse.click(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2)
        if not pg.evaluate(T): fail.append("the clipboard textarea cannot be focused after a click in the frame")
finally:
    r.close()
print("FAIL: " + "; ".join(fail) if fail else "OK")
sys.exit(1 if fail else 0)
