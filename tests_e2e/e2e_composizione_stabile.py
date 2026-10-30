"""«Elemental composition» panel: a click on the MS1 spectrum that does not change the mass must not rebuild the panel (0 nodes removed, the typed mass stays)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
r = Run(port=8875, wd="/tmp/wd_comp_stable")
try:
    with sync_playwright() as p:
        pg = r.page(p); D = synth("comp_stable")
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); ready(pg)
        attendi(pg, "!!E.panels.find(p=>p.type==='spec'&&p._a&&p._a.hrp)")
        pg.evaluate("setActive(E.panels.find(p=>p.type==='spec'&&p._a&&p._a.hrp))"); drawer(pg, "cmp")
        pg.wait_for_selector("#cx-out table, #cx-out .muted", timeout=20000)
        pg.fill("#cx-mz", "317.16425"); pg.wait_for_selector("#cx-out tr[data-i]", timeout=20000)
        pg.evaluate("window.__rm = 0; new MutationObserver(m => { for (const x of m) window.__rm += x.removedNodes.length; }).observe(document.querySelector('#hri-body'), {childList: true, subtree: true}); 0")
        pg.evaluate("E.panels.find(p=>p.type==='spec'&&p._a&&p._a.hrp).el.scrollIntoView({block:'center'}); 0")
        bb = pg.locator(".pnl.spec canvas").first.bounding_box()
        for i in range(5):
            pg.mouse.click(bb["x"] + bb["width"] * (0.3 + 0.1 * i), bb["y"] + bb["height"] / 2)
            pg.evaluate("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
        rm = pg.evaluate("window.__rm"); assert rm == 0, f"{rm} nodes removed by 5 clicks"
        assert pg.input_value("#cx-mz") == "317.16425", pg.input_value("#cx-mz")
finally:
    r.close(); r.report()
print("\nSTEPS\n  composition panel stays put on clicks: ok")
