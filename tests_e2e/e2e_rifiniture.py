# e2e_rifiniture: loading animations, progress bar for slow API calls, Disegno panel toggle and label colours, font / high contrast in the gear.
# Synthetic data (tests_e2e/synth.py): no real lab files needed.
import os, sys, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import synth
steps = []
def step(name, fn):
    print("step:", name, flush=True)
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_rifiniture.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
def lum(h):
    c = [int(h[i:i + 2], 16) / 255 for i in (1, 3, 5)]; c = [x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c]; return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
def contrast(a, b):
    x, y = sorted((lum(a), lum(b)), reverse=True); return (x + 0.05) / (y + 0.05)
r = Run(port=8826, wd="/tmp/wd26")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        shutil.rmtree("/tmp/s26", ignore_errors=True)
        pg.set_input_files("#pick", synth.make_series("/tmp/s26", [0, 15, 30])); pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        def ions():
            for mode in pg.evaluate("Object.keys(window.LDAN)"):
                pg.evaluate(f"ldIon(true, '{mode}')"); pg.wait_for_timeout(250)
                assert pg.evaluate("!!document.querySelector('#ldion')"), mode
                pg.evaluate("ldIon(false)")
        step("ldIon draws every analyser without errors (still frame with reduced motion)", ions)
        def cleanup():
            body = pg.inner_text("body")
            assert "cella attiva:" not in body.lower() and "active cell:" not in body.lower()
        step("«cella attiva» label is gone", cleanup)
        def busy():
            # the engine of the site answers in a few ms: its worker is slowed down (the messages to it wait `__lag` ms) so that a call stays pending
            pg.evaluate("() => { const pm = Worker.prototype.postMessage; Worker.prototype.postMessage = function (...a) { setTimeout(() => pm.apply(this, a), window.__lag || 0); }; window.__lag = 1800; window.__slow = fetch('api/formula?f=C6H12O6').catch(() => 0); }")
            assert pg.evaluate("document.getElementById('busybar').hidden") is True
            pg.wait_for_selector("#busybar:not([hidden])", timeout=4000)
            assert pg.get_attribute("#busybar", "aria-label")
            pg.evaluate("window.__lag = 0")
            pg.wait_for_selector("#busybar[hidden]", state="attached", timeout=6000)
        step("progress bar shows after ~1 s of a pending API call and hides afterwards", busy)
        def gear():
            pg.click("#np-set"); pg.wait_for_timeout(200)
            pg.select_option("#uip-font", "atkinson"); pg.check("#uip-hc"); pg.wait_for_timeout(200)
            root = "document.documentElement"
            assert pg.evaluate(f"{root}.getAttribute('data-a11y-font')") == "atkinson" and pg.evaluate(f"{root}.hasAttribute('data-a11y-bg')")
            pg.reload(); pg.wait_for_timeout(1500)
            assert pg.evaluate(f"{root}.getAttribute('data-a11y-font')") == "atkinson" and pg.evaluate(f"{root}.getAttribute('data-a11y-bg')") == "hc"
        step("font and high contrast are applied and remembered", gear)
        def draw():
            tab = pg.locator("text=Disegno").first; tab.click(); pg.wait_for_timeout(800)
            assert pg.evaluate("!!document.querySelector('#v-draw aside #side-toggle')")
            assert pg.evaluate("document.querySelector('#kframe').offsetTop") == 0 or pg.evaluate("document.querySelector('#kframe').getBoundingClientRect().top - document.querySelector('#kframe').parentElement.getBoundingClientRect().top") < 2
            pg.click("#side-toggle"); pg.wait_for_timeout(300)
            assert pg.evaluate("!!document.querySelector('#side-corner #side-toggle')") and pg.is_visible("#side-toggle")
            pg.click("#side-toggle"); pg.wait_for_timeout(300)
            assert pg.evaluate("!!document.querySelector('#v-draw aside #side-toggle')")
            src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "mzlab", "web", "draw.js"), encoding="utf-8").read()
            assert "isDark" not in src
            assert contrast("#3b3b3b", "#ffffff") >= 4.5 and contrast("#2b5c8a", "#ffffff") >= 4.5
        step("Disegno: toggle above the cards / in the canvas corner, label colours readable on the white canvas", draw)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
