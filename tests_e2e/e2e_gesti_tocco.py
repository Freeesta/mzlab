"""Gestures of the RT x m/z map on a touch screen (SC-4): one finger draws the zoom box, two fingers move and pinch, a double tap = whole view, a long press = the menu."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_gesti_tocco.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8903, wd="/tmp/wd_gesti_tocco")
M = "E.panels.find(q=>q.type==='map')"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        ctx = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}, has_touch=True)
        pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); frames(pg, 50)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")]); frames(pg, 50)
        pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); frames(pg, 37)
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); frames(pg, 18)
        c = pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()"); cx, cy = c[0] + c[2] / 2, c[1] + c[3] / 2
        z = lambda: pg.evaluate(f"[{M}.zoom,{M}.zoomY]")
        cdp = ctx.new_cdp_session(pg)
        def touch(kind, pts): cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": a, "y": d, "id": i} for i, (a, d) in enumerate(pts)]})
        zr_hidden = lambda: pg.evaluate("document.querySelector('.pnl.map .zr').hidden")
        def one_finger_box():
            touch("touchStart", [(cx - 60, cy - 40)]); frames(pg, 3)
            for k in range(1, 7): touch("touchMove", [(cx - 60 + k * 20, cy - 40 + k * 14)]); frames(pg, 2)
            assert not zr_hidden(), "the box of the finger is drawn"
            touch("touchEnd", []); frames(pg, 18); zz = z(); assert zz[0] and zz[1] and zr_hidden(), ("one finger zooms to the box", zz)
        step("one finger draws the zoom box and zooms to it", one_finger_box)
        def two_fingers():
            pg.evaluate("MAPPA.whole(" + M + ")"); frames(pg, 18)
            touch("touchStart", [(cx - 40, cy), (cx + 40, cy)]); frames(pg, 3)
            for k in range(1, 7): touch("touchMove", [(cx - 40 - k * 12, cy), (cx + 40 + k * 12, cy)]); frames(pg, 2)
            touch("touchEnd", []); frames(pg, 18); a = z(); assert a[0] and a[1], ("pinch out zooms in", a)
            touch("touchStart", [(cx - 40, cy), (cx + 40, cy)]); frames(pg, 3)
            for k in range(1, 7): touch("touchMove", [(cx - 40 + k * 15, cy), (cx + 40 + k * 15, cy)]); frames(pg, 2)
            touch("touchEnd", []); frames(pg, 18); b2 = z(); assert b2[0][0] < a[0][0], ("two fingers move the map", a, b2)
        step("two fingers pinch and move the map", two_fingers)
        def double_tap():
            for _ in range(2): touch("touchStart", [(cx, cy)]); frames(pg, 2); touch("touchEnd", []); frames(pg, 5)
            frames(pg, 18); assert z() == [None, None], z()
        step("double tap = whole view", double_tap)
        def long_press():
            touch("touchStart", [(cx, cy)]); pg.wait_for_function("!document.getElementById('ctx').hidden", timeout=4000); touch("touchEnd", []); frames(pg, 18)
            assert pg.evaluate("document.querySelectorAll('#ctx [role^=menuitem]').length") >= 5
        step("a long press opens the menu", long_press)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
