"""Desktop Safari (docs/agenti/compat-safari.md): a trackpad pinch on a Mac arrives as gesturestart/gesturechange/gestureend, not as the Ctrl+wheel of
the other browsers. compat.js turns it into that wheel: a pinch on a chart zooms the chart (the page does not zoom), a pinch outside a chart is left
to the browser. With --browser webkit (the nightly round on macOS) the gesture events are WebKit's own; in Chromium GestureEvent is stubbed before
the page runs and the gesture events are dispatched by hand (a real trackpad cannot be driven by Playwright). No browser errors."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_safari_mac.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8829, wd="/tmp/wd_safari_mac")
SKIP_TOUR = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');sessionStorage.setItem('mzlab.coiReload','1');}catch(_){}"
# Chromium and Firefox have no GestureEvent: a stand-in, so compat.js installs the pinch as on a Mac
GESTURE = "if(!window.GestureEvent){window.GestureEvent=function GestureEvent(){};}"
# a gesture event as WebKit sends it (scale is cumulative from gesturestart); returns whether the page zoom was stopped
PINCH = """async ([sel, scales]) => {
  const t = typeof sel === 'string' ? document.querySelector(sel) : sel, b = t.getBoundingClientRect(), x = b.left + b.width / 2, y = b.top + b.height / 2;
  const ev = (type, scale) => { const e = new Event(type, { bubbles: true, cancelable: true }); Object.assign(e, { scale, clientX: x, clientY: y, screenX: x, screenY: y }); t.dispatchEvent(e); return e.defaultPrevented; };
  const frame = () => new Promise(ok => requestAnimationFrame(() => requestAnimationFrame(ok)));      // one event a frame, as a trackpad sends them
  const out = [ev('gesturestart', 1)]; for (const s of scales) { await frame(); out.push(ev('gesturechange', s)); } await frame(); out.push(ev('gestureend', scales[scales.length - 1])); return out; }"""

if BROWSER == "firefox":
    print("  skipped in Firefox: the gesture* pinch is WebKit only; the steps run in WebKit and Chromium"); r.srv.terminate(); sys.exit(0)
try:
    from playwright.sync_api import sync_playwright
    print(f"  browser: {BROWSER}" + ("" if BROWSER == "webkit" else " (GestureEvent stubbed; the WebKit round is the nightly --browser webkit)"))
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        c = b.new_context(viewport={"width": 1440, "height": 900}, device_scale_factor=2, reduced_motion="reduce")
        c.add_init_script(SKIP_TOUR); c.add_init_script(GESTURE)
        pg = c.new_page()
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.on("console", lambda m: r.errs.append(("console." + m.type, m.text)) if m.type == "error" else None)
        pg.goto(f"http://127.0.0.1:{r.port}/"); ready(pg)
        def installed():
            assert not pg.evaluate("matchMedia('(hover: none) and (pointer: coarse)').matches"), "the desktop context is touch-only"
            assert "pinch" in pg.evaluate("window.__compat || []"), pg.evaluate("window.__compat")
        step("Mac: compat.js installs the pinch", installed)
        def chart_pinch():
            with pg.expect_file_chooser() as fc: pg.evaluate("document.getElementById('pick').click()")
            fc.value.set_files([str(EX / "FullScan_t0.mzML")])
            wait_for_js(pg, "document.querySelectorAll('#flist input[data-k=use]').length>=1", 60000)
            pg.click("#opbtn"); ready(pg)
            wait_for_js(pg, "E.panels.some(q => q.type === 'chrom' && q._a && q._a.full && q.cv.width > 0)", 60000)
            pg.evaluate("window.__chrom = E.panels.find(q => q.type === 'chrom'); window.__chrom.zoom = null; draw(window.__chrom)"); frames(pg, 2)
            stopped = pg.evaluate(PINCH, [pg.evaluate_handle("window.__chrom.cv"), [1.25, 1.6, 2.0]])
            assert all(stopped), f"the page zoom was not stopped on the chart: {stopped}"
            z, full = pg.evaluate("[window.__chrom.zoom, window.__chrom._a.full]")
            assert z and (z[1] - z[0]) < 0.8 * (full[1] - full[0]), f"spreading the fingers did not zoom the chart in: zoom {z}, full {full}"
            span = z[1] - z[0]
            pg.evaluate(PINCH, [pg.evaluate_handle("window.__chrom.cv"), [0.8, 0.6]])
            z2 = pg.evaluate("window.__chrom.zoom || window.__chrom._a.full")
            assert (z2[1] - z2[0]) > span * 1.2, f"pinching the fingers together did not zoom out: {z} -> {z2}"
        step("Mac: a pinch on a chart zooms the chart in and out, not the page", chart_pinch)
        def outside():
            stopped = pg.evaluate(PINCH, ["header", [1.5, 2.0]])
            assert not any(stopped), f"a pinch outside the charts was taken from the browser: {stopped}"
        step("Mac: a pinch outside the charts stays the browser's (page zoom)", outside)
        c.close(); b.close()
finally:
    r.srv.terminate()
for s in steps: print("  ", s)
bad = [s for s in steps if s[1] != "ok"]
if r.errs: print("browser errors:", r.errs[:5])
sys.exit(1 if bad or r.errs else 0)
