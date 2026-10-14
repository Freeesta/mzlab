# e2e_dblclick_pannelli: double click on a graph panel never changes the tab (MRM used to jump to Full Scan)
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8838, wd="/tmp/wd_dblclick")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        def dbl(sel):
            c = pg.locator(sel + " canvas").first; c.scroll_into_view_if_needed(); b = c.bounding_box()
            pg.mouse.dblclick(b["x"] + b["width"] / 2, b["y"] + b["height"] / 2); pg.wait_for_timeout(500)
        def mrm():
            open_kind(pg, "mrm"); ready(pg)
            n = pg.evaluate("E.panels.length"); pg.evaluate("E.panels.find(p=>p.type==='mrm').zoom=[1,2]")
            dbl(".pnl.mrm")
            assert pg.evaluate("E.tab") == "mrm", pg.evaluate("E.tab")
            assert pg.evaluate("E.panels.length") == n, "a panel was added in the MRM tab"
            assert pg.evaluate("E.panels.find(p=>p.type==='mrm').zoom") is None, "double click did not reset the zoom"
        step("MRM: double click keeps the tab and resets the zoom", mrm)
        def others():
            for t, sel in (("full", ".pnl.chrom"), ("ms2", ".pnl.chrom")):
                open_kind(pg, t); ready(pg)
                if not pg.locator(sel + ":visible").count(): continue
                dbl(sel)
                assert pg.evaluate("E.tab") == t, (t, pg.evaluate("E.tab"))
        step("Full Scan and MS2: double click keeps the tab", others)
        def zoomed():
            open_kind(pg, "full"); ready(pg)
            n = pg.evaluate("E.panels.length"); pg.evaluate("E.panels.find(p=>p.type==='chrom').zoom=[1,2]")
            dbl(".pnl.chrom")
            assert pg.evaluate("E.panels.find(p=>p.type==='chrom').zoom") is None, "double click did not reset the zoom"
            assert pg.evaluate("E.panels.length") == n, "a spectrum was extracted while zoomed"
        step("Chromatogram zoomed: double click resets the zoom and extracts no spectrum", zoomed)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
