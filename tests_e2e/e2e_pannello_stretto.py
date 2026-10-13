"""A narrow window hides the side panel first (the charts keep their room); a tool button brings it back; widening restores it."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
r = Run(port=8852, wd="/tmp/wd_stretto")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1400, "height": 900})
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        st = lambda: pg.evaluate("(() => { const v = document.querySelector('#v-data'), w = document.querySelector('#v-workspace'); return {open: BARRA.open, cls: v.className, ws: w.getBoundingClientRect().width, wsShown: getComputedStyle(w).display !== 'none'}; })()")
        def at(w):
            pg.set_viewport_size({"width": w, "height": 900}); pg.wait_for_timeout(500); return st()
        drawer(pg, "file")
        def wide():
            s = at(1400); assert s["open"] and "sb-open" in s["cls"] and s["wsShown"], s
        step("1400: panel docked next to the charts", wide)
        def narrow():
            for w in (1090, 900, 700):
                s = at(w); assert not s["open"] and "sb-open" not in s["cls"] and s["wsShown"] and s["ws"] > w * 0.8, (w, s)
        step("1090 / 900 / 700: panel hidden, charts keep the width", narrow)
        def back():
            s = at(1400); assert s["open"] and "sb-open" in s["cls"], s
        step("widening restores the panel", back)
        def reopen():
            at(1000); assert not st()["open"]
            pg.evaluate("BARRA.setTab('file', {noFocus: true, instant: true})"); pg.wait_for_timeout(400)
            s = st(); assert s["open"] and "sb-open" in s["cls"] and "sb-sheet" not in s["cls"], s
            s = at(700); assert s["open"] and "sb-sheet" in s["cls"], s
            pg.click("#sb-x"); pg.wait_for_timeout(300); s = st(); assert not s["open"] and s["wsShown"], s
            s = at(1400); assert not s["open"], s
        step("reopened by hand at 1000 (docked) and 700 (sheet); closed by hand stays closed", reopen)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
