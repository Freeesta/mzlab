"""Disegno opens and stays responsive at CPU x4: the longest main-thread long task (Ketcher's iframe shares the page thread) stays under a ceiling.
The ceiling only goes down: every Ketcher start-up improvement lowers it toward 200 ms (the inline worker removal brought the longest task from 4.1 s to below it)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
CEILING_MS = 6000          # CI headroom over the measured ~4.2 s; lower it with each step
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8864, wd="/tmp/wd_dlong")
OBS = """() => { window.__lt = []; new PerformanceObserver(l => l.getEntries().forEach(e => window.__lt.push([Math.round(e.startTime), Math.round(e.duration)]))).observe({ type: 'longtask', buffered: true }); }"""
try:
    with sync_playwright() as p:
        pg = r.page(p); attendi(pg, "!!document.querySelector(\"#nav button[data-v=draw]\") && typeof S !== \"undefined\"")
        pg.context.new_cdp_session(pg).send("Emulation.setCPUThrottlingRate", {"rate": 4})
        pg.evaluate(OBS)
        pg.click("#nav [data-v=draw]")
        pg.wait_for_selector("#kload", state="hidden", timeout=120000)
        def interactions():
            for _ in range(3):
                pg.keyboard.press("Escape"); frames(pg, 10)
                pg.click("#nav [data-v=draw]"); frames(pg, 10)
            frames(pg, 20)
            lt = pg.evaluate("window.__lt")
            worst = max([d for _, d in lt] + [0])
            assert worst < CEILING_MS, ("longest long task", worst, lt)
        step("longest long task stays under the ceiling", interactions)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
