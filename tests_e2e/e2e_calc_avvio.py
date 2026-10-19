"""The Calculator opens from the toolbar on the loading screen (cold start, no file open): the window is visible and closes again."""
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
            attendi(pg, "document.querySelector('#np-calc2') && !document.querySelector('#start').hidden")
            pg.click("#np-calc2")
            attendi(pg, "(() => { const b = document.querySelector('#calcdlg').getBoundingClientRect(); return b.width > 100 && b.height > 50 })()")
            assert pg.is_visible("#calcdlg") and pg.is_visible("#calcin")
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length")
        step("loading screen: the Calculator button opens the window", opens)
        def closes():
            pg.click("#np-calc2"); attendi(pg, "document.querySelector('#calcdlg').hidden")
            pg.click("#np-calc2"); attendi(pg, "!document.querySelector('#calcdlg').hidden"); assert pg.is_visible("#calcdlg")
        step("a second press closes it, a third opens it again", closes)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
