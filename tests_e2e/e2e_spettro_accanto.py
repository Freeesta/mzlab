# e2e_spettro_accanto: the option «Spectrum beside the chromatogram» (Settings): off = the spectrum of the double click goes below; on = in the row of the chromatogram; Ctrl+Z undoes; a narrow window keeps it below
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8896, wd="/tmp/wd_accanto")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        attendi(pg, "performance.now() > 1000"); pg.click("text=Carica dati"); ready(pg)
        CH = "E.panels.find(p=>p.type==='chrom'&&p.full)"
        # the chromatogram alone in its row, then a double click far from the cursor
        alone = lambda: pg.evaluate(f"(()=>{{const c={CH};return LAYOUT.righeDi(stackOrder()).find(r=>r.includes(c)).length}})()")
        def dbl(fx):
            c = pg.locator(".pnl.chrom canvas").first; c.scroll_into_view_if_needed(); b = c.bounding_box()
            pg.mouse.dblclick(b["x"] + b["width"] * fx, b["y"] + b["height"] * 0.5)
        def live(): return pg.evaluate(f"(()=>{{const c={CH};const s=E.panels.find(q=>q.type==='spec'&&q.link===c.id&&q.el);return s?s.id:null}})()")
        def spenta():
            open_kind(pg, "full"); ready(pg); assert pg.evaluate("UIP.beside") is False, "off by default"
            dbl(0.4); attendi(pg, f"(()=>{{const c={CH};return !!E.panels.find(q=>q.type==='spec'&&q.link===c.id&&q.el)}})()")
            assert alone() == 1, "off: the chromatogram stays alone in its row"
        step("off (default): the spectrum goes below the chromatogram", spenta)
        def accesa():
            pg.evaluate("UIP.beside = true; uipSave()")
            pg.evaluate(f"(()=>{{const c={CH};c.cur=null}})()")
            dbl(0.6); attendi(pg, f"{CH} && LAYOUT.righeDi(stackOrder()).find(r=>r.includes({CH})).length === 2")
            same = pg.evaluate(f"(()=>{{const c={CH};const s=E.panels.find(q=>q.type==='spec'&&q.link===c.id&&q.el);return LAYOUT.righeDi(stackOrder()).some(r=>r.includes(c)&&r.includes(s))}})()")
            assert same, "on: the live spectrum is in the row of the chromatogram"
        step("on: the spectrum is in the same row as the chromatogram", accesa)
        def ricordata():
            assert pg.evaluate("JSON.parse(localStorage.getItem('qqq.prefs')).beside") is True, "remembered in the preferences"
        step("the choice is remembered", ricordata)
        def annulla():
            pg.keyboard.press("Control+z"); attendi(pg, f"LAYOUT.righeDi(stackOrder()).find(r=>r.includes({CH})).length === 1")
        step("Ctrl+Z puts the spectrum back below", annulla)
        def stretta():
            pg.set_viewport_size({"width": 700, "height": 900}); attendi(pg, "performance.now() > 0")
            pg.evaluate(f"(()=>{{const c={CH};c.cur=null}})()")
            dbl(0.3); attendi(pg, f"!!{CH}")
            assert alone() == 1, "narrow window: stays stacked"
        step("a narrow window keeps it below", stretta)
finally:
    print("STEPS"); [print(" ", s) for s in steps]
    r.close()
