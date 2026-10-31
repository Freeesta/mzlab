"""Opening Disegno answers at once at CPU x4: the click's slowest event stays under 200 ms, the loading state shows, and the editor still starts."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
r = Run(port=8863, wd="/tmp/wd_dclick")
OBS = """() => { window.__ev = []; new PerformanceObserver(l => l.getEntries().forEach(e => window.__ev.push([e.name, Math.round(e.duration)]))).observe({ type: 'event', buffered: true, durationThreshold: 16 }); }"""
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.wait_for_timeout(2500)
        pg.context.new_cdp_session(pg).send("Emulation.setCPUThrottlingRate", {"rate": 4})
        pg.evaluate(OBS)
        pg.click("#nav [data-v=draw]")
        def fast():
            pg.wait_for_timeout(300)
            worst = pg.evaluate("Math.max(0, ...window.__ev.map(e => e[1]))")
            assert worst < 200, ("slowest event of the click", worst, pg.evaluate("window.__ev"))
            assert pg.is_visible("#kload"), "loading state shown at once"
        step("click answers under 200 ms with the loading state", fast)
        def starts():
            pg.wait_for_selector("#kload", state="hidden", timeout=90000)
            assert pg.evaluate("!!document.querySelector('#kframe').contentWindow.ketcher"), "editor not ready"
        step("the editor starts after the pause in input", starts)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
