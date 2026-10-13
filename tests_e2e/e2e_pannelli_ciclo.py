# e2e_pannelli_ciclo: opening and closing panels leaves no listener behind on document/window (closePanel aborts p._ac)
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8841, wd="/tmp/wd_pciclo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        cdp = pg.context.new_cdp_session(pg)
        def count():
            n = 0
            for expr in ("window", "document"):
                o = cdp.send("Runtime.evaluate", {"expression": expr})["result"]["objectId"]
                n += len(cdp.send("DOMDebugger.getEventListeners", {"objectId": o})["listeners"])
            return n
        def cycle():
            pg.evaluate("addPanel('spec',{tab:E.tab,k:E.cur,level:1}); closePanel(E.panels[E.panels.length-1])")
            pg.wait_for_timeout(50)
        def leak():
            for _ in range(3): cycle()
            base = count()
            for _ in range(20): cycle()
            assert count() == base, (base, count())
            assert pg.evaluate("STATO.verifica()") == []
        step("20 open/close cycles: listeners do not grow", leak)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
