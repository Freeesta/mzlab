# e2e_tutti_esperimenti: with «all experiments together» the tab bar (Full Scan / MS2 / MRM) disappears and the panels of every type are on the page; switching it off brings the tabs back; the choice is saved
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8846, wd="/tmp/wd_tutti")
TABS_N = "document.querySelectorAll('#dtabs [data-t]').length"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES]); pg.wait_for_timeout(800); pg.click("text=Carica dati"); ready(pg)
        def on():
            assert pg.evaluate(TABS_N) == 3, pg.evaluate(TABS_N)
            pg.click("#np-set"); pg.wait_for_selector("#uip-tog"); pg.check("#uip-tog"); pg.wait_for_timeout(600)
            assert pg.evaluate(TABS_N) == 0, "tab buttons still visible"
            assert pg.evaluate("new Set(E.panels.filter(p=>p.el&&p.el.style.display!=='none').map(p=>p.tab)).size") >= 2, "panels of several types expected"
        step("on: no tab buttons, panels of several experiments on the page", on)
        def kept():
            pg.keyboard.press("Escape")
            assert pg.evaluate("JSON.parse(uipRead(UIP_KEY) || '{}').tog === true"), "the choice is not saved in the settings"
        step("the choice is saved with the other settings", kept)
        def off():
            pg.evaluate("setTogether(false)"); pg.wait_for_timeout(500)
            assert pg.evaluate(TABS_N) == 3, pg.evaluate(TABS_N)
        step("off: the tabs are back", off)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
