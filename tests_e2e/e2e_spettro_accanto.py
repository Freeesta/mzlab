# e2e_spettro_accanto: the spectrum of the double click always goes below the chromatogram, in its own row; an old stored «beside» preference is ignored;
# the settings panel has no switch for it; a narrow window keeps it below
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
        wait_for_js(pg, "performance.now() > 1000"); pg.click("text=Carica dati"); ready(pg)
        CH = "E.panels.find(p=>p.type==='chrom'&&p.full)"
        # the chromatogram alone in its row, then a double click far from the cursor
        alone = lambda: pg.evaluate(f"(()=>{{const c={CH};return LAYOUT.righeDi(stackOrder()).find(r=>r.includes(c)).length}})()")
        def dbl(fx):
            c = pg.locator(".pnl.chrom canvas").first; c.scroll_into_view_if_needed(); b = c.bounding_box()
            pg.mouse.dblclick(b["x"] + b["width"] * fx, b["y"] + b["height"] * 0.5)
        def live(): return pg.evaluate(f"(()=>{{const c={CH};const s=E.panels.find(q=>q.type==='spec'&&q.link===c.id&&q.el);return s?s.id:null}})()")
        def below():
            open_kind(pg, "full"); ready(pg)
            dbl(0.4); wait_for_js(pg, f"(()=>{{const c={CH};return !!E.panels.find(q=>q.type==='spec'&&q.link===c.id&&q.el)}})()")
            assert alone() == 1, "the chromatogram stays alone in its row"
        step("the spectrum goes below the chromatogram", below)
        def old_pref():
            # a value stored by an older visit: read back, ignored, dropped at the next save
            pg.evaluate("(()=>{const o=JSON.parse(localStorage.getItem('mzlab.prefs')||'{}');o.beside=true;localStorage.setItem('mzlab.prefs',JSON.stringify(o));uipLoad();uipSave()})()")
            assert pg.evaluate("'beside' in UIP") is False, "no beside field in the preferences"
            assert "beside" not in pg.evaluate("localStorage.getItem('mzlab.prefs')"), "the old key is dropped at the next save"
            pg.evaluate(f"(()=>{{const c={CH};c.cur=null}})()")
            dbl(0.6); wait_for_js(pg, f"(()=>{{const c={CH};return !!E.panels.find(q=>q.type==='spec'&&q.link===c.id&&q.el)}})()")
            assert alone() == 1, "an old beside:true still leaves the spectrum below"
        step("an old stored «beside» is ignored", old_pref)
        def no_switch():
            pg.evaluate("document.querySelector('#np-set').click()"); wait_for_js(pg, "!!document.querySelector('#uipset')")      # as the command «settings» of the bar (clickEl)
            assert pg.locator("#uip-beside").count() == 0, "the switch is gone"
            assert pg.locator("#uip-xiclens").count() == 1, "the panel still opens with its other switches"
            pg.evaluate("document.querySelector('#np-set').click()")
        step("the settings panel has no «beside» switch", no_switch)
        def narrow():
            pg.set_viewport_size({"width": 700, "height": 900}); wait_for_js(pg, "performance.now() > 0")
            pg.evaluate(f"(()=>{{const c={CH};c.cur=null}})()")
            dbl(0.3); wait_for_js(pg, f"!!{CH}")
            assert alone() == 1, "narrow window: stays stacked"
        step("a narrow window keeps it below", narrow)
finally:
    print("STEPS"); [print(" ", s) for s in steps]
    r.close()
