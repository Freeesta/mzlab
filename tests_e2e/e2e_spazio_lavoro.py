"""Work area vertical spacing: the first card starts right under the toolbar row (<= 2 px), cards are <= 4 px apart, the card header keeps its 24 px targets."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
r = Run(port=8895, wd="/tmp/wd95")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("window.scrollTo(0,0)"); frames(pg, 10)
        def gaps():
            g = pg.evaluate("""(() => { const R = e => e.getBoundingClientRect(), ps = [...document.querySelectorAll('#dpanels > .pnl')].filter(e => e.offsetParent).sort((a, b) => R(a).top - R(b).top);
              return { n: ps.length, top: R(ps[0]).top - R(document.querySelector('#tools')).bottom, between: ps.slice(1).map((q, i) => R(q).top - R(ps[i]).bottom) }; })()""")
            assert g["n"] >= 2, g
            assert 0 <= g["top"] <= 2.5, f"toolbar to first card: {g['top']}px"
            assert all(0 <= x <= 4 for x in g["between"]), f"card to card: {g['between']}"
        step("toolbar -> first card <= 2 px, card -> card <= 4 px", gaps)
        def targets():
            small = pg.evaluate("""[...document.querySelectorAll('#dpanels > .pnl .hd button, #tools button')].filter(b => b.offsetParent && !b.closest('[hidden]')).map(b => { const r = b.getBoundingClientRect(); return [b.className + ':' + (b.getAttribute('aria-label') || b.textContent.trim()).slice(0, 20), Math.round(r.width), Math.round(r.height)]; }).filter(x => x[1] < 24 || x[2] < 24)""")
            assert not small, small
        step("header and toolbar buttons keep >= 24 px targets", targets)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
