# e2e_stato: the page state changes only through the transitions of stato.js (event "stato:cambiato") and its invariants hold after each step
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8839, wd="/tmp/wd_stato")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("window.__ev = []; document.addEventListener('stato:cambiato', e => window.__ev.push(e.detail.motivo))")
        def inv():
            assert pg.evaluate("STATO.verifica()") == [], pg.evaluate("STATO.verifica()")
        def tabs():
            for t in ("mrm", "ms2", "full"):
                pg.click("#dtabs [data-t=%s]" % t); ready(pg)
                assert pg.evaluate("E.tab") == t
                inv()
            assert "tab" in pg.evaluate("window.__ev"), "tab switches fire stato:cambiato"
        step("tabs: invariants hold and the switches are events", tabs)
        def dbl():
            pg.click("#dtabs [data-t=mrm]"); ready(pg); pg.evaluate("window.__ev = []")
            c = pg.locator(".pnl.mrm canvas").first; c.scroll_into_view_if_needed(); b = c.bounding_box()
            pg.mouse.dblclick(b["x"] + b["width"] / 2, b["y"] + b["height"] / 2); pg.wait_for_timeout(500)
            assert pg.evaluate("E.tab") == "mrm" and "tab" not in pg.evaluate("window.__ev"), pg.evaluate("window.__ev")
            inv()
        step("MRM double click: no tab change event", dbl)
        def close():
            n = pg.evaluate("E.panels.length"); pg.evaluate("closePanel(E.panels[E.panels.length-1])"); pg.wait_for_timeout(300)
            assert pg.evaluate("E.panels.length") == n - 1 and "close" in pg.evaluate("window.__ev")
            inv()
        step("closing a panel: event and invariants", close)
        def broken():
            pg.evaluate("E.cur = 9999"); assert pg.evaluate("STATO.verifica()"), "an invalid E.cur is reported"
            pg.evaluate("STATO.file(0, 'prova')"); inv()
        step("verifica() reports a broken state", broken)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
