# e2e_tabella_demo: the sample table lists the example files after loading the demo AND after a reload (the session comes back from the engine:
# the rows of the table were once missing there), in the order of the set, with role, time and size filled in; the same rows as the side list
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
NAMES = ["FullScan_t%d" % t for t in (0, 5, 10, 15, 30)]
TIMES = [0, 5, 10, 15, 30]
r = Run(port=8893, wd="/tmp/wd_tabdemo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1400, "height": 900})
        pg.click("#demobtn"); pg.click("#demochoice [data-demo=full]")
        pg.wait_for_function("document.querySelectorAll('#flist [data-k=time]').length===5", timeout=60000)
        pg.click("text=Carica dati"); ready(pg)
        def table():
            pg.click("#samptab"); frames(pg, 4)
            rows = pg.evaluate("[...document.querySelectorAll('#wtable tbody tr')].map(t => [t.querySelector('[data-open]')?.dataset.open || '', +t.querySelector('[data-k=time]').value])")
            assert [x[0] for x in rows] == [n + ".mzML" for n in NAMES], rows
            assert [x[1] for x in rows] == TIMES, rows
            side = pg.evaluate("[...document.querySelectorAll('#flst .fl .nm')].map(x => x.textContent.trim())")
            assert side == NAMES, side
        step("demo: the table has the five samples, in the order of the side list", table)
        def after_reload():
            pg.reload(); pg.wait_for_function("window.qqReady && E.files.length === 5", timeout=60000); frames(pg, 4)
            if not pg.evaluate("S.table"): pg.click("#samptab"); frames(pg, 4)
            rows = pg.evaluate("[...document.querySelectorAll('#wtable tbody tr')].map(t => t.querySelector('[data-open]')?.dataset.open || '')")
            assert rows == [n + ".mzML" for n in NAMES], rows
            assert pg.evaluate("workState()") == "table"
        step("reload: the table still has the five samples", after_reload)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
