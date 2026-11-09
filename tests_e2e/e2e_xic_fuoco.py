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
        pg = r.page(p); wait_for_js(pg, "typeof openXic === 'function'")
        pg.evaluate("openXic(null)"); wait_for_js(pg, "document.querySelector('#xicdlg').open && document.querySelectorAll('#xic-rows .xrw').length === 2")
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
        def named():
            name = pg.evaluate("(()=>{const d=document.querySelector('#xicdlg'),id=d.getAttribute('aria-labelledby');return id&&document.getElementById(id)?document.getElementById(id).textContent:''})()")
            assert name == pg.evaluate("I18N.t('xicdlg.title')"), f"the window has no accessible name: {name!r}"
            e = pg.evaluate("(()=>{const e=document.querySelector('#xic-err');return [e.getAttribute('role'),e.getAttribute('aria-live')]})()")
            assert e == ["status", "polite"], f"the message line is not announced: {e}"
            pg.click("#xic-add")
            small = pg.evaluate("[...document.querySelectorAll('#xicdlg button,#xicdlg select,#xicdlg input')].filter(n=>n.offsetParent).map(n=>n.getBoundingClientRect()).filter(b=>b.width<24||b.height<24).length")
            assert small == 0, f"{small} targets under 24x24 px"
        step("the window is named, its message line is a polite status, every target is at least 24x24 px", named)
        def esc():
            pg.keyboard.press("Escape"); wait_for_js(pg, "!document.querySelector('#xicdlg').open")
        step("Esc still closes the window", esc)
        def back():
            pg.evaluate("(()=>{const b=document.createElement('button');b.id='xf-opener';b.textContent='x';document.body.appendChild(b);b.focus();b.onclick=()=>openXic(null)})()")
            pg.keyboard.press("Enter"); wait_for_js(pg, "document.querySelector('#xicdlg').open")
            pg.keyboard.press("Escape"); wait_for_js(pg, "!document.querySelector('#xicdlg').open")
            assert pg.evaluate("document.activeElement.id") == "xf-opener", pg.evaluate("document.activeElement.outerHTML.slice(0, 120)")
        step("Esc gives the focus back to what opened the window", back)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
