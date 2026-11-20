"""Header walk (1 of 4): without samples, on Data and Drawing every header button opens its window, menu or panel and Esc (or the same button) closes it; the page comes back to the state it had and no main block (header, side panel, toolbar, work area, panels) has moved; Data → each view → Data moves nothing. The walk is split into four tests (e2e_giro_*) that share tests_e2e/lib_giro.py so the smoke shards can balance them. Audit: tests_e2e/audit_pagine.py."""
# covers: mzlab/web/barra.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_giro import *

steps = []
step = make_step(steps)
r = Run(port=8996, wd="/tmp/wd_giro_intestazione")
VIEWS = ["draw", "flow"]
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        wait_for_js(pg, "typeof AVVIO !== 'undefined' && AVVIO.stato === 'ready'", 120000); ready(pg)
        wait_for_js(pg, "!!document.querySelector('#nav [data-v=flow]')")
        step("no samples, Data: every header button opens and closes, nothing moves", lambda: walk(pg, "data, empty"))
        step("no samples, draw: every header button opens and closes, nothing moves", lambda: (view(pg, "draw"), walk(pg, "draw, empty"), view(pg, "data")))
        step("no samples: Data → each view → Data moves nothing", lambda: round_trip(pg, VIEWS))
        step("no console error, no failed request", lambda: clean(r))
finally:
    try: r.close()
    except Exception: pass
finish(steps)
