"""Disegno: extra Ctrl+Z with the focus outside the canvas raises no error box; the canvas menu is a keyboard menu and closes on a click or Esc inside the Ketcher frame."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=8993, wd="/tmp/wd_disegno_menu")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000); pg.wait_for_timeout(800)
        KJ = "document.querySelector('#kframe').contentWindow.ketcher"
        pg.evaluate(f"{KJ}.addFragment('CCO')"); pg.wait_for_timeout(800)
        def undo():
            pg.evaluate("document.activeElement && document.activeElement.blur()")
            if pg.locator("#errbox").count(): pg.evaluate("document.getElementById('errbox').remove()")       # an earlier error (e.g. a wasm not built) must not hide this check
            for _ in range(3): pg.keyboard.press("Control+z"); pg.wait_for_timeout(150)
            pg.wait_for_timeout(300); assert pg.locator("#errbox").count() == 0, "error box: " + (pg.locator("#errbox").inner_text() if pg.locator("#errbox").count() else "")
        step("3x Ctrl+Z with the focus outside the canvas raise no error box", undo)
        pg.evaluate(f"{KJ}.addFragment('CCO')"); pg.wait_for_timeout(800)
        OPEN = """(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:200,clientY:200}))})()"""
        def opened():
            for _ in range(20):
                pg.evaluate(OPEN)
                if pg.locator("#ctx:not([hidden]) div[role=menuitem]").count(): return
                pg.wait_for_timeout(400)
            raise AssertionError("menu did not open")
        step("the menu opens with role=menu and menuitem rows", lambda: (opened(), pg.wait_for_selector("#ctx[role=menu]")))
        def click_closes():
            pg.evaluate("(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('mousedown',{bubbles:true}))})()")
            assert pg.locator("#ctx").is_hidden()
        step("a click inside the Ketcher frame closes it", click_closes)
        def esc_closes():
            opened()
            pg.evaluate("(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.KeyboardEvent('keydown',{key:'Escape',bubbles:true}))})()")
            assert pg.locator("#ctx").is_hidden()
        step("Esc inside the Ketcher frame closes it", esc_closes)
        def keys():
            opened()
            pg.locator("#ctx div[role=menuitem]").first.focus(); pg.keyboard.press("ArrowDown")
            assert pg.evaluate("document.activeElement === document.querySelectorAll('#ctx div[role=menuitem]')[1]"), "arrow did not move"
            pg.keyboard.press("Escape"); assert pg.locator("#ctx").is_hidden()
        step("arrows move between items and Esc closes", keys)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
