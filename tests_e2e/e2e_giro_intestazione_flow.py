"""Header walk (2 of 4): without samples, on Flow and Tabui every header button opens and closes with nothing moved, and the tool buttons of Drawing, Flow and Tabui open their window (see e2e_giro_intestazione.py)."""
# covers: mzlab/web/barra.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_giro import *

steps = []
step = make_step(steps)
r = Run(port=8996, wd="/tmp/wd_giro_intestazione_flow")
VIEWS = ["draw", "flow", "tpmine"]
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        wait_for_js(pg, "typeof AVVIO !== 'undefined' && AVVIO.stato === 'ready'", 120000); ready(pg)
        wait_for_js(pg, "!!document.querySelector('#nav [data-v=flow]') && !!document.querySelector('#nav [data-v=tpmine]')")
        for v in ("flow", "tpmine"):
            step(f"no samples, {v}: every header button opens and closes, nothing moves", lambda v=v: (view(pg, v), walk(pg, f"{v}, empty"), view(pg, "data")))
        for v in VIEWS:
            step(f"no samples, {v}: the tool buttons open their window", lambda v=v: (floating_outside_data(pg, v), view(pg, "data")))
        step("no console error, no failed request", lambda: clean(r))
finally:
    try: r.close()
    except Exception: pass
finish(steps)
