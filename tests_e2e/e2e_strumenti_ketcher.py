"""Disegno: the quick rings are a column on the left of Ketcher's tools (no bottom row), the editor fits the window, the first zoom is
above 100 %, no tooltip on the canvas, and the «Strumenti dell'editor» card shows/hides groups of buttons (kept after a reload)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
VIS = """id => [...document.querySelector('#kframe').contentDocument.querySelectorAll('[data-testid="' + id + '"]')].some(x => x.getBoundingClientRect().width > 0)"""
def ready(pg):
    pg.click("#nav [data-v=draw]")
    for _ in range(300):
        if pg.evaluate("() => !!(window.TPDraw && TPDraw.ready())"): break
        pg.wait_for_timeout(100)
    pg.wait_for_timeout(500)
r = Run(port=8904, wd="/tmp/wd_kstr")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1440, "height": 900}); ready(pg)
        def layout():
            o = pg.evaluate("""() => { const f = document.querySelector('#kframe'), d = f.contentDocument, box = q => d.querySelector(q).getBoundingClientRect();
              const b = box('[class*="BottomToolbar-module_root"]'), l = box('[class*="LeftToolbar-module_root"]'), fr = f.getBoundingClientRect();
              return { bx: b.left, bw: b.width, bh: b.height, lx: l.left, fb: fr.bottom, vh: innerHeight, title: f.getAttribute('title'),
                       zoom: f.contentWindow.ketcher.editor.zoom() }; }""")
            assert o["bh"] > o["bw"] and o["bx"] < o["lx"], ("rings: a column left of the tools", o)
            assert o["fb"] <= o["vh"] + 2, ("the editor fits the window", o)
            assert o["zoom"] == 1, ("first zoom 100 %", o)
            assert not o["title"], "no tooltip over the canvas"
        step("rings column on the left, editor fits the window, zoom = 100 %, no tooltip", layout)
        def tools():
            assert pg.evaluate(VIS, "template-0") and pg.evaluate(VIS, "reaction-plus")
            for hidden in ("sgroup", "enhanced-stereo", "create-monomer", "polymer-toggler", "any-atom"): assert not pg.evaluate(VIS, hidden), hidden
            pg.click("#tools-card summary"); pg.click("#tools-all"); pg.wait_for_timeout(300)
            for shown in ("sgroup", "enhanced-stereo", "create-monomer", "polymer-toggler", "any-atom"): assert pg.evaluate(VIS, shown), shown
            pg.click("#tools-def"); pg.wait_for_timeout(300); pg.uncheck("input[data-t=rings]"); pg.wait_for_timeout(300)
            assert not pg.evaluate(VIS, "template-0")
        step("tool groups: essentials by default, «Mostra tutti», single groups", tools)
        def keep():
            pg.reload(); pg.wait_for_timeout(1500); ready(pg)
            assert not pg.evaluate(VIS, "template-0"), "the choice is kept after a reload"
            pg.click("#tools-card summary"); pg.click("#tools-def"); pg.wait_for_timeout(300)
            assert pg.evaluate(VIS, "template-0")
        step("the choice is kept in this browser", keep)
        def font():
            pg.click("#tools-card summary") if not pg.is_visible("#font-body") else None
            sz = lambda: pg.evaluate("() => JSON.parse(document.querySelector('#kframe').contentWindow.localStorage.getItem('ketcher-opts') || '{}').fontsz")
            pg.check("input[name=dfont][value='20']"); pg.wait_for_timeout(500)
            assert sz() == 20, sz()
            pg.reload(); pg.wait_for_timeout(1500); ready(pg)
            assert pg.evaluate("localStorage.getItem('qqq.disegno.font')") == "20"
            assert sz() == 20, ("applied again after a reload", sz())
            pg.click("#tools-card summary") if not pg.is_visible("#font-body") else None
            pg.check("input[name=dfont][value='16']"); pg.wait_for_timeout(300)
        step("atom symbol size: three sizes, applied to Ketcher and kept", font)
        pg.screenshot(path=SH + "k_strumenti.png")
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
