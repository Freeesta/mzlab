"""Restored session: the second toolbar row (file selector and «+ panel» buttons) is in the first paint of the work area, with its final
place, size and buttons, before the engine is ready (no pop-in), and nothing of it shifts afterwards (CLS ~ 0). With and without restored
panels, at a wide and at a narrow width. Build first: python tools/build_site.py"""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
TRACE = """window.__first = null; window.__cls = 0;
new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__cls += e.value; }).observe({type: 'layout-shift', buffered: true});
const geo = () => { const t = document.getElementById('tools'), v = document.getElementById('v-data'); if (!t || !v || v.hidden) return null; const r = t.getBoundingClientRect();
  return JSON.stringify({top: Math.round(r.top), h: Math.round(r.height), ready: !!window.qqReady,
    btn: [...t.querySelectorAll('button,select')].map(e => (e.hidden || !e.offsetWidth) ? 0 : Math.round(e.getBoundingClientRect().width))}); };
window.__geo = geo;
const loop = () => { if (!window.__first) { const g = geo(); if (g) window.__first = g; } requestAnimationFrame(loop); };
document.addEventListener('DOMContentLoaded', loop);"""
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8980, wd="/tmp/wd_riga")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES[:2]]); ready(pg)
        pg.click("text=Carica dati"); ready(pg)
        pg.click("#np-chrom"); ready(pg)
        pg.add_init_script(TRACE)
        pg.context.new_cdp_session(pg).send("Emulation.setCPUThrottlingRate", {"rate": 4})
        def restore(name, w, panels):
            def run():
                if not panels:
                    pg.set_viewport_size({"width": 1500, "height": 900}); pg.click("#np-closeall"); ready(pg)
                pg.set_viewport_size({"width": w, "height": 900})
                pg.reload(); pg.wait_for_function("window.qqReady === true", timeout=120000); ready(pg); frames(pg, 20)
                first = json.loads(pg.evaluate("window.__first")); last = json.loads(pg.evaluate("window.__geo()"))
                assert not first["ready"], "the work area must be drawn before the engine is ready: " + str(first)
                assert first["top"] == last["top"] and first["h"] == last["h"], f"row moved or resized: {first} -> {last}"
                assert all(abs(a - b) <= 4 for a, b in zip(first["btn"], last["btn"])) and len(first["btn"]) == len(last["btn"]), f"buttons changed: {first} -> {last}"
                assert sum(1 for x in first["btn"] if x) >= 4, f"the row is missing at first paint: {first}"
                cls = pg.evaluate("window.__cls"); assert cls <= 0.02, f"CLS {cls}"
            step(name, run)
        import json
        restore("wide, panels restored", 1500, True)
        restore("narrow (800 px), panels restored", 800, True)
        restore("wide, no panels", 1500, False)
        restore("narrow (800 px), no panels", 800, False)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
