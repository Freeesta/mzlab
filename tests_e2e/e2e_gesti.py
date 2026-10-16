"""Gestures of the RT x m/z map with mouse and keyboard (SC-4): the wheel alone scrolls the page, Ctrl/⌘ + wheel zooms, a drag draws the zoom box,
Backspace = whole view, Esc does not take the zoom back (the sight only), the right-click menu on the plot and on the margins. LR example files."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_gesti.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8902, wd="/tmp/wd_gesti")
M = "E.panels.find(q=>q.type==='map')"
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 700})
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")])
        pg.click("#opbtn"); ready(pg)
        pg.click("#np-map"); ready(pg); frames(pg, 37)
        box = lambda: pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
        at = lambda fx, fy: (lambda c: (c[0] + c[2] * fx, c[1] + c[3] * fy))(box())
        z = lambda: pg.evaluate(f"[{M}.zoom,{M}.zoomY]")
        show = lambda: pg.evaluate(M + ".el.scrollIntoView({block:'center'})") or frames(pg, 15)
        show()
        def wheel_alone():
            pg.evaluate("document.scrollingElement.scrollTop = Math.max(0, document.scrollingElement.scrollTop - 40)"); frames(pg, 9)
            y0 = pg.evaluate("document.scrollingElement.scrollTop"); x, y = at(.5, .45); pg.mouse.move(x, y)
            for _ in range(3): pg.mouse.wheel(0, 120); frames(pg, 5)
            frames(pg, 18)
            assert z() == [None, None], ("the wheel alone does not zoom the map", z())
            assert pg.evaluate("document.scrollingElement.scrollTop") > y0, "the page scrolls"
        step("the wheel alone scrolls the page, the map does not zoom", wheel_alone)
        def wheel_ctrl():
            show(); x, y = at(.5, .45); pg.mouse.move(x, y)
            for _ in range(3): ctrl_wheel(pg, 0, -200); frames(pg, 3)
            frames(pg, 15); zz = z(); assert zz[0] and zz[1], zz
        step("Ctrl/⌘ + wheel zooms RT and m/z", wheel_ctrl)
        def backspace():
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("Backspace"); frames(pg, 15); assert z() == [None, None], z()
        step("Backspace = whole view", backspace)
        def esc_keeps_zoom():
            show(); (x0, y0), (x1, y1) = at(.3, .3), at(.6, .6)
            pg.mouse.move(x0, y0); pg.mouse.down(); pg.mouse.move(x1, y1, steps=6); pg.mouse.up(); frames(pg, 18)
            zz = z(); assert zz[0] and zz[1], ("the drag draws the zoom box", zz)
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("Escape"); frames(pg, 15)
            assert z() == zz, ("Esc does not take the zoom back", zz, z())
            pg.keyboard.press("Control+z"); frames(pg, 18); assert z() != zz, "Ctrl/⌘+Z takes it back"
        step("a drag zooms to the box; Esc keeps the zoom, Ctrl/⌘+Z undoes it", esc_keeps_zoom)
        def menu_everywhere():
            for fx, fy in ((.5, .5), (.5, .97), (.01, .5)):                      # plot, bottom margin, left margin
                show(); x, y = at(fx, fy); pg.mouse.click(x, y, button="right"); frames(pg, 18)
                assert pg.evaluate("!document.getElementById('ctx').hidden") and pg.evaluate("document.querySelectorAll('#ctx [role^=menuitem]').length") >= 5, (fx, fy)
                pg.keyboard.press("Escape"); pg.wait_for_function("document.getElementById('ctx').hidden", timeout=4000)
        step("the right-click menu opens on the plot and on the margins", menu_everywhere)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
