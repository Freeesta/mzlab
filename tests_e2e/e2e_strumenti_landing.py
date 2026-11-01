"""In the empty landing the tools (adducts & losses, lists) open docked in the side panel, never as a floating window; closing one gives the file list back. The calculator is the drop-down under its button there (no side panel): the file list stays and a second press closes it."""
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
            for cmd, panel in (("calc", "#sb-panel-calc"), ("addloss", "#sb-panel-adducts"), ("lists", "#sb-panel-lists")):
                pg.click(f'#stbar [data-cmd="{cmd}"]'); frames(pg, 6)
                if cmd == "calc":
                    # outside a session the calculator is a drop-down, not a docked panel (e2e_calc_avvio)
                    ok = pg.is_visible("#calcdlg") and not pg.evaluate(VIS, panel) and pg.evaluate(VIS, "#sb-panel-file")
                    print(("OK  " if ok else "FAIL"), cmd, "drop-down, file list stays")
                    if not ok: bad.append(cmd)
                    pg.click(f'#stbar [data-cmd="{cmd}"]'); frames(pg, 6)
                    back = not pg.is_visible("#calcdlg") and pg.evaluate(VIS, "#sb-panel-file")
                    print(("OK  " if back else "FAIL"), cmd, "closed: file list back")
                    if not back: bad.append(cmd + " close")
                    continue
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
