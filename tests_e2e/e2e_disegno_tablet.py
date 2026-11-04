"""Disegno on a 768 px tablet: Quick draw leaves the canvas in view, the header buttons and the side-panel controls are at least 24 px, and an empty canvas exports nothing."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=8996, wd="/tmp/wd_disegno_tablet")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 768, "height": 1024})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        KJ = "document.querySelector('#kframe').contentWindow.ketcher"
        def empty_export():
            pg.evaluate("document.getElementById('ex-png').scrollIntoView()")
            downloads = []; pg.on("download", lambda d: downloads.append(d))
            pg.click("#ex-png"); attendi(pg, "!document.getElementById('ex-warn').hidden")
            t = pg.inner_text("#ex-warn"); assert "Niente da esportare" in t, t
            assert not downloads, "a file was downloaded"
        step("PNG of an empty canvas warns and downloads nothing", empty_export)
        def quick():
            pg.evaluate("document.getElementById('ex-smi').scrollIntoView()")
            pg.fill("#ex-smi", "CCO"); pg.click("#ex-load")
            attendi(pg, f"!{KJ}.editor.struct().isBlank()")
            attendi(pg, "(() => { const r = document.getElementById('kframe').getBoundingClientRect(); return r.top >= 0 && r.top < innerHeight / 2 })()")
        step("Quick draw brings the canvas into view", quick)
        def sizes():
            bad = pg.evaluate("""[...document.querySelectorAll('#v-draw input[type=checkbox], #v-draw input[type=radio], #v-draw summary, header .hq')].filter(e => e.offsetParent).map(e => { const r = e.getBoundingClientRect(); return [e.id || e.tagName, Math.round(r.width), Math.round(r.height)] }).filter(x => x[1] < 24 || x[2] < 24)""")
            assert not bad, bad
        step("checkboxes, radios, summaries and the «i» button are at least 24x24", sizes)
        def header():
            right = pg.evaluate("Math.round(document.querySelector('header .hq').getBoundingClientRect().right)")
            assert right <= 768, right
        step("the «i» button of the header is inside the 768 px width", header)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
