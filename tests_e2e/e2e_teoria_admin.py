"""Teoria tab and administrator mode: the gear of the program is hidden while the Teoria is open (it has its own settings on the right); five hits on the logo with the Teoria already
open reload it, so the editor button and Alt+E appear; the button opens the chapter editor, and the editor stays closed (no button) without the mode."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_admin.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:1500]))
r = Run(port=8870, wd="/tmp/wd_teoadmin")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        tf = lambda: next(x for x in pg.frames if "/teoria/" in x.url)
        pg.click("#nav [data-v=theory]"); pg.wait_for_selector("#stbar [data-tcmd=indice]", timeout=15000)
        def gear():
            assert pg.locator("#np-set").is_hidden(), "the gear of the program must not show in the Teoria"
            assert pg.locator("#stbar [data-tcmd=impostazioni]").is_visible()
            pg.click("#nav [data-v=data]"); assert pg.locator("#np-set").is_visible()
            pg.click("#nav [data-v=theory]"); pg.wait_for_selector("#stbar [data-tcmd=indice]")
        step("the gear of the program is hidden in the Teoria, back in Dati", gear)
        def nomode():
            assert pg.locator("#stbar [data-tcmd=editor]").count() == 0, "no editor button without the administrator mode"
        step("without the mode there is no editor button", nomode)
        def unlock():
            for _ in range(5): pg.click("header img.logo")
            pg.wait_for_selector("#stbar [data-tcmd=editor]", timeout=15000)       # the Teoria in the frame reloads by itself: no manual reload
            assert pg.evaluate("localStorage.getItem('mzlab.admin')") == "1"
        step("five logo hits with the Teoria open: the editor button appears without a reload by hand", unlock)
        def edit():
            pg.click("#stbar [data-tcmd=editor]"); tf().wait_for_selector("body.tedit-active #tedit-bar", timeout=10000)
        step("the button opens the chapter editor", edit)
        def alt_e():
            pg.once("dialog", lambda d: d.accept()); pg.focus("#tframe")        # the key is read by the Teoria: focus inside the frame
            pg.keyboard.press("Alt+e"); tf().wait_for_selector("body:not(.tedit-active)", timeout=10000)
            pg.keyboard.press("Alt+e"); tf().wait_for_selector("body.tedit-active", timeout=10000)
        step("Alt+E closes and opens the editor", alt_e)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
