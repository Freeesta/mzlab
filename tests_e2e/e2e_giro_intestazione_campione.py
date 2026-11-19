"""Header walk (3 of 4): with one sample, on Data and Drawing every header button opens and closes with nothing moved; Data → each view → Data moves nothing, also with the side panel shut; adding and closing a chromatogram, spectrum and map panel gives the work area back as it was (see e2e_giro_intestazione.py)."""
# covers: mzlab/web/barra.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_giro import *

steps = []
step = make_step(steps)
r = Run(port=8996, wd="/tmp/wd_giro_intestazione_campione")
VIEWS = ["draw", "flow", "tpmine"]
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        wait_for_js(pg, "typeof AVVIO !== 'undefined' && AVVIO.stato === 'ready'", 120000); ready(pg)
        wait_for_js(pg, "!!document.querySelector('#nav [data-v=flow]') && !!document.querySelector('#nav [data-v=tpmine]')")
        def with_sample():
            stage(pg, [mz("B_FullMass-t15")]); open_first(pg); ready(pg)
            pg.evaluate("window.scrollTo(0, 0)"); frames(pg, 2)
        step("one sample loaded", with_sample)
        step("one sample, Data: every header button opens and closes, nothing moves", lambda: walk(pg, "data, sample"))
        step("one sample, draw: every header button opens and closes, nothing moves", lambda: (view(pg, "draw"), walk(pg, "draw, sample"), view(pg, "data")))
        step("one sample: Data → each view → Data moves nothing", lambda: round_trip(pg, VIEWS))

        def side_shut():
            pg.evaluate("BARRA.close()"); frames(pg, 3); ready(pg, settle=200)
            round_trip(pg, ["draw"])
            pg.evaluate("BARRA.show('file', {noFocus: true, instant: true})"); frames(pg, 3)
        step("one sample, side panel shut: Data → Drawing → Data moves nothing", side_shut)

        def add_close_panels():
            bad = []
            for sel in ("#np-chrom", "#np-spec", "#np-map"):
                if not pg.is_visible(sel) or pg.is_disabled(sel): continue
                n0, g0 = pg.evaluate("E.panels.length"), pg.evaluate(GEOM)
                pg.click(sel); wait_for_js(pg, f"E.panels.length === {n0 + 1}"); ready(pg, settle=200)
                pg.evaluate("E.panels[E.panels.length - 1].el.querySelector('.x').click()"); wait_for_js(pg, f"E.panels.length === {n0}"); ready(pg, settle=200)
                m = moved(g0, pg.evaluate(GEOM))
                if m: bad.append(f"{sel}: {m}")
            assert not bad, " | ".join(bad)
        step("one sample: adding a chromatogram, spectrum and map panel and closing it with its × gives the work area back as it was", add_close_panels)
        step("no console error, no failed request", lambda: clean(r))
finally:
    try: r.close()
    except Exception: pass
finish(steps)
