"""Teoria start-up: the chapter text and the side index are painted before the quadrupole simulations are built (they start after the first
paint, so heavy numerics do not hold DOMContentLoaded), and the simulations still appear afterwards."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_avvio.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:220]))
r = Run(port=8977, wd="/tmp/wd_teoria_avvio")
try:
    with sync_playwright() as p:
        r.page(p)
        T = f"http://127.0.0.1:{r.port}/static/teoria/"
        def shell_first():
            c = r.b.new_context(viewport={"width": 1280, "height": 900}); q = c.new_page()
            q.add_init_script("""window.__t={};document.addEventListener('DOMContentLoaded',()=>{__t.h1=!!document.querySelector('h1');setTimeout(()=>{__t.sims=document.querySelectorAll('#sim-field canvas').length},0);},{once:true});""")
            q.goto(T + "09-quadrupolo.html")
            q.wait_for_selector("#sim-field canvas", timeout=15000)
            t = q.evaluate("window.__t")
            assert t["h1"] and t["sims"] == 0, f"the simulation was built inside DOMContentLoaded: {t}"
            assert q.locator("#side a").count() > 5, "the side index must be there once the page is loaded"
            c.close()
        step("09-quadrupolo: text and index first, simulations after the first paint", shell_first)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
