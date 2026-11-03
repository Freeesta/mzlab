"""On the loading screen (cold start, no file open) Adducts & losses opens with the calculator docked on top: visible, and closes again."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
r = Run(port=8885, wd="/tmp/wd85")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        def opens():
            attendi(pg, "document.querySelector('#stbar [data-cmd=addloss]') && !document.querySelector('#start').hidden")
            assert not pg.is_visible("#np-calc2"), "on the start page the calculator lives in Adducts & losses"
            pg.click("#stbar [data-cmd=addloss]")
            attendi(pg, "(() => { const b = document.querySelector('#calcdlg').getBoundingClientRect(); return b.width > 100 && b.height > 50 })()")
            assert pg.is_visible("#calcdlg") and pg.is_visible("#calcin") and pg.evaluate("document.querySelector('#al-calc-body').contains(document.querySelector('#calcdlg'))")
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length")
        step("loading screen: Adducts & losses opens with the calculator in the side panel", opens)
        def closes():
            pg.click("#stbar [data-cmd=addloss]"); attendi(pg, "document.querySelector('#sb-panel-addloss').hidden || !document.querySelector('#calcdlg').offsetParent")
            pg.click("#stbar [data-cmd=addloss]"); attendi(pg, "!!document.querySelector('#calcdlg').offsetParent"); assert pg.is_visible("#calcdlg")
        step("a second press closes the panel, a third opens it again", closes)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
