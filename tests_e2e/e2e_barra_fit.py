# e2e_barra_fit: the toolbar fits at six widths (nothing overflows, the labels step down as the room shrinks), fit() forces few layouts, and Ctrl+B still toggles the drawer
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
WIDTHS = (320, 480, 768, 1024, 1366, 1920)
r = Run(port=8842, wd="/tmp/wd_barrafit")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0")])
        frames(pg, 3); pg.click("text=Carica dati"); ready(pg)
        cdp = pg.context.new_cdp_session(pg); cdp.send("Performance.enable")
        layouts = lambda: next(m["value"] for m in cdp.send("Performance.getMetrics")["metrics"] if m["name"] == "LayoutCount")
        res = {}
        def fits():
            for w in WIDTHS:
                pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 3); pg.evaluate("BARRA.fit()")
                a = layouts(); pg.evaluate("BARRA.fit()"); n = layouts() - a
                over = pg.evaluate("(()=>{const b=document.querySelector('#stbar');return b.scrollWidth-b.clientWidth})()")
                res[w] = (pg.evaluate("BARRA.labelLevel"), n)
                assert over <= 0, (w, "overflow", over)
                assert n <= 12, (w, "layouts", n)               # the loop is one layout per step (3 label levels + at most 9 dropped groups)
        step("six widths: no overflow, bounded layouts", fits)
        def steps_down():
            order = ["all", "main", "none"]
            lv = [order.index(res[w][0]) for w in WIDTHS]
            assert lv == sorted(lv, reverse=True), res          # wider bar never has less text than a narrower one
        step("label level never grows as the bar narrows", steps_down)
        def ctrlb():
            pg.set_viewport_size({"width": 1440, "height": 900}); frames(pg, 3)
            before = pg.evaluate("BARRA.isOpen()")
            pg.keyboard.press("Control+b"); frames(pg, 3)
            assert pg.evaluate("BARRA.isOpen()") != before, "Ctrl+B did not toggle the drawer"
        step("Ctrl+B toggles the drawer (merged keydown handler)", ctrlb)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
