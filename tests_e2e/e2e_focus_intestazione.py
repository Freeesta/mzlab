# e2e_focus_intestazione: header buttons ((i), Search, toolbar) show no ring after a mouse click or after closing the search; from the keyboard the ring is 2 px and INSIDE the button (never clipped)
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8843, wd="/tmp/wd_focus")
OUT = "(s=>{const e=document.querySelector(s), c=getComputedStyle(e);return {st:c.outlineStyle,w:c.outlineWidth,o:c.outlineOffset,f:document.activeElement===e}})"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES]); pg.wait_for_timeout(800); pg.click("text=Carica dati"); ready(pg)
        def mouse():
            for s in ("header .hq", "#fs-btn", "#tb-search"):
                if not pg.locator(s).count() or not pg.is_visible(s): continue
                pg.click(s); pg.wait_for_timeout(300)
                if s == "#tb-search": pg.mouse.click(5, 400); pg.wait_for_timeout(300)      # leave the search with the mouse, outside its list of results (a click on one would open it)
                pg.mouse.click(5, 400); pg.wait_for_timeout(100)
                pg.evaluate("document.querySelector(%r).focus()" % s)      # programmatic focus after a mouse use (what closing the search does)
                o = pg.evaluate(OUT + "(%r)" % s)
                assert o["f"] and o["st"] == "none", (s, o)
        step("after mouse use no ring shows on the header buttons", mouse)
        def keyboard():
            pg.keyboard.press("Tab"); pg.evaluate("document.querySelector('#tb-search').focus()")
            o = pg.evaluate(OUT + "('#tb-search')")
            assert o["f"] and o["st"] == "solid" and o["w"] == "2px" and float(o["o"][:-2]) < 0, o
        step("keyboard focus: 2 px ring drawn inside the button", keyboard)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
