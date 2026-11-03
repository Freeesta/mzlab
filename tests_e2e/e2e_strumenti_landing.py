"""In the empty landing the tools (adducts & losses, lists) open docked in the side panel, never as a floating window; closing one gives the file list back. The calculator is docked on top of Adducts & losses."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
lib.AUTO_TEMPI = False
from lib import *

VIS = "id => { const e = document.querySelector(id); return !!e && !e.hidden && e.offsetParent !== null; }"

if __name__ == "__main__":
    r = Run(port=8830 + 172, wd="/tmp/wd_strumenti_landing"); bad = []
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "window.qqReady && document.querySelector('#fl-hint')"); frames(pg, 5)
            assert not pg.evaluate(VIS, "#np-calc2"), "no calculator button on the start page: it lives in Adducts & losses"
            for cmd, panel in (("addloss", "#sb-panel-adducts"), ("lists", "#sb-panel-lists")):
                pg.click(f'#stbar [data-cmd="{cmd}"]'); frames(pg, 6)
                if cmd == "addloss" and not pg.evaluate(VIS, "#calcin"):
                    print("FAIL addloss: the calculator is not on top of the page"); bad.append("calc")
                ok = pg.evaluate(VIS, panel) and not pg.evaluate(VIS, "#sb-panel-file") and pg.evaluate(VIS, "#dfiles")
                floating = pg.evaluate("[...document.querySelectorAll('body > *')].some(e => e.id !== 'dfiles' && getComputedStyle(e).position === 'fixed' && /calc|adduct|nl|liste|ref/i.test(e.id) && e.offsetParent !== null)")
                print(("OK  " if ok and not floating else "FAIL"), cmd, "docked" if ok else "NOT docked", "floating" if floating else "")
                if not ok or floating: bad.append(cmd)
                pg.click(f'#stbar [data-cmd="{cmd}"]'); frames(pg, 6)
                back = pg.evaluate(VIS, "#sb-panel-file") and not pg.evaluate(VIS, panel)
                print(("OK  " if back else "FAIL"), cmd, "closed: file list back")
                if not back: bad.append(cmd + " close")
            cls = pg.evaluate("window.__cls || 0")
            print("page errors:", r.errs)
    finally:
        r.close()
    sys.exit(1 if bad else 0)
