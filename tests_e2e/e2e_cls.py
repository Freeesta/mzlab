"""Layout stability of the first load: Disegno (direct #/draw) and a Teoria chapter (mouse and touch header sizes) must not shift after the first paint
(layout-shift sum without recent input <= 0.02), with the CPU slowed 4x so that late scripts really land after the first paint."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
LIMIT = 0.02
OBSERVE = """window.__cls = 0; window.__shifts = [];
new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) { window.__cls += e.value;
  window.__shifts.push([Math.round(e.startTime), +e.value.toFixed(4), (e.sources || []).map(s => s.node && (s.node.id || s.node.className || s.node.tagName)).slice(0, 4)]); } }).observe({type: 'layout-shift', buffered: true});"""
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_cls.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:300]))
r = Run(port=8978, wd="/tmp/wd_cls")
try:
    with sync_playwright() as p:
        r.page(p)
        def cold(url, ready, **ctx_args):
            """CLS of a first load in a fresh context (empty cache and storage) with a 4x slower CPU."""
            c = r.b.new_context(viewport={"width": 1300, "height": 900}, locale="it-IT", **ctx_args); q = c.new_page()
            q.add_init_script(OBSERVE)
            q.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');}catch(_){}")
            c.new_cdp_session(q).send("Emulation.setCPUThrottlingRate", {"rate": 4})
            q.goto(url); attendi(q, ready, timeout=90000); frames(q, 30)
            cls, shifts = q.evaluate("() => [window.__cls, window.__shifts]"); c.close()
            assert cls <= LIMIT, f"CLS {cls:.4f} > {LIMIT}: {shifts}"
        T = f"http://127.0.0.1:{r.port}/static/teoria/09-quadrupolo.html"
        TREADY = "!!document.querySelector('#sidecmd') && !!document.querySelector('#a11y-btn') && !!document.querySelector('#tbar')"
        step("Disegno at #/draw: no shift after the first paint", lambda: cold(f"http://127.0.0.1:{r.port}/#/draw",
             "document.getElementById('kframe').classList.contains('kwait') === false && !!(window.TPDraw && TPDraw.ready())"))
        step("Teoria (mouse): header and index tools in their final place", lambda: cold(T, TREADY))
        step("Teoria (touch): header and index tools in their final place", lambda: cold(T, TREADY, has_touch=True))
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
