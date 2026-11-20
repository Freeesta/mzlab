"""The first Ctrl/Cmd+K: paletta.js loads on first use, the keys typed meanwhile are replayed into the box (nothing lost)."""
# covers: mzlab/web/carica.js, mzlab/web/paletta.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
def fresh(p):
    r = Run(port=8873, wd="/tmp/wd_paletta_primo"); r.b = p.chromium.launch()
    ctx = r.b.new_context(viewport={"width": 1440, "height": 900}, locale="it-IT", reduced_motion="reduce"); pg = ctx.new_page()
    pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}))}catch(_){}")
    pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#pick", state="attached"); wait_for_js(pg, "typeof CMD !== 'undefined'")
    return r, ctx, pg
with sync_playwright() as p:
    def typed_at_once():
        r, ctx, pg = fresh(p)
        try:
            assert pg.evaluate("typeof PALETTE") == "undefined", "the palette must not be in the start-up list"
            pg.keyboard.press("Control+k"); pg.keyboard.type("calcol")      # no wait: the script is still arriving
            wait_for_js(pg, "!!document.getElementById('pal-in') && document.getElementById('pal-in').value === 'calcol'")
            wait_for_js(pg, "document.querySelectorAll('#pal-l [role=option]').length > 0")
        finally: ctx.close(); r.close()
    step("Ctrl+K then typing at once: every key reaches the box and the results appear", typed_at_once)
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
