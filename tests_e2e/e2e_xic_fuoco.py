"""«Extract ions (XIC)» window, keyboard: the × of a row removes it and moves the focus to the next row (the one before when the last row goes),
never to the body of the page behind the modal window. Found by tools/fuzz_ui.py. No data needed."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
r = Run(port=free_port(), wd="/tmp/wd_xicfuoco")
try:
    with sync_playwright() as p:
        pg = r.page(p); attendi(pg, "typeof openXic === 'function'")
        pg.evaluate("openXic(null)"); attendi(pg, "document.querySelector('#xicdlg').open && document.querySelectorAll('#xic-rows .xrw').length === 2")
        fo = "[...document.querySelectorAll('#xic-rows input')].indexOf(document.activeElement)"
        def middle():
            pg.click("#xic-add"); pg.click("#xic-add"); pg.locator("#xic-rows .xr-x").nth(1).click()
            assert pg.locator("#xic-rows .xrw").count() == 3 and pg.evaluate(fo) == 1, pg.evaluate("document.activeElement.outerHTML.slice(0, 120)")
        step("× with the mouse: the next row takes the focus", middle)
        def last():
            pg.locator("#xic-rows .xr-x").nth(2).focus(); pg.keyboard.press("Enter")
            assert pg.locator("#xic-rows .xrw").count() == 2 and pg.evaluate(fo) == 1, pg.evaluate("document.activeElement.outerHTML.slice(0, 120)")
            pg.locator("#xic-rows .xr-x").nth(0).focus(); pg.keyboard.press("Space")
            assert pg.locator("#xic-rows .xrw").count() == 1 and pg.evaluate(fo) == 0
        step("× from the keyboard on the last row: the one before; on the first: the one after", last)
        def esc():
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('#xicdlg').open")
        step("Esc still closes the window", esc)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
