"""A5: sharp canvases at any pixel ratio (also when it changes while the page is open), area label 13 px, no canvas text under 12 px."""
# covers: mzlab/web/grafici.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_nitido.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8840, wd="/tmp/wd_nit")
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        ctx = r.b.new_context(viewport={"width": 1280, "height": 900}, device_scale_factor=2)
        pg = ctx.new_page(); pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(800)
        ok = lambda d: pg.evaluate("d => [...document.querySelectorAll('.pnl canvas')].filter(c => c.clientWidth > 50).every(c => c.width === Math.round(c.clientWidth * d))", d)
        def dpr2(): assert ok(2) and pg.evaluate("document.querySelectorAll('.pnl canvas').length") >= 2
        step("deviceScaleFactor 2: canvas.width = clientWidth x 2", dpr2)
        def change():            # CDP does not fire the media-query event of a real zoom, so the size changes by 1 px too (a real zoom changes both)
            cdp = ctx.new_cdp_session(pg)
            cdp.send("Emulation.setDeviceMetricsOverride", {"width": 1281, "height": 900, "deviceScaleFactor": 1, "mobile": False}); pg.wait_for_timeout(900)
            assert ok(1), ("redrawn at 1x after the ratio changed", pg.evaluate("devicePixelRatio"), pg.evaluate("[...document.querySelectorAll('.pnl canvas')].map(c => [c.width, c.clientWidth])"))
            cdp.send("Emulation.setDeviceMetricsOverride", {"width": 1282, "height": 900, "deviceScaleFactor": 1.5, "mobile": False}); pg.wait_for_timeout(900)
            assert ok(1.5), "redrawn at 1.5x"
        step("the ratio changes while the page is open (browser zoom): everything is redrawn", change)
        def dprnow():
            assert pg.evaluate("dprNow()") == 1.5 and pg.evaluate("typeof dprNow") == "function"
        step("dprNow() = devicePixelRatio x pinch scale", dprnow)
        def area():
            pg.evaluate("(() => { const p = E.panels.find(p => p.type === 'chrom'); p.imode = 'auto'; })()")
            pg.evaluate("window.__fonts = []; const d = Object.getOwnPropertyDescriptor(CanvasRenderingContext2D.prototype, 'font'); Object.defineProperty(CanvasRenderingContext2D.prototype, 'font', { set(v) { window.__fonts.push(v); d.set.call(this, v); }, get() { return d.get.call(this); } })")
            pg.evaluate("redrawAll()"); pg.wait_for_timeout(1200)
            sizes = pg.evaluate("window.__fonts.map(f => parseFloat((String(f).match(/([\\d.]+)px/) || [0, 99])[1])).filter(x => x < 99)")
            assert sizes and min(sizes) >= 12, sorted(set(sizes))[:6]
        step("no canvas text smaller than 12 px", area)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
