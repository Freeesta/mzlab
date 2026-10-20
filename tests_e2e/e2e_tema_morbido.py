"""Soft theme change: with motion allowed the switch goes through a view transition (or, without the API, the .tema-tr colour fade) and ends in the new theme;
with reduced motion the theme is applied at once, with neither."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
SKIP_HINTS = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1')}catch(_){}"
SPY = "window.__vt=0;window.__tr=0;new MutationObserver(()=>{if(document.documentElement.classList.contains('tema-tr'))window.__tr++}).observe(document,{attributes:true,attributeFilter:['class'],subtree:true});"
SPY_VT = "if(document.startViewTransition){const o=document.startViewTransition.bind(document);document.startViewTransition=f=>{window.__vt++;return o(f)}}"
NO_VT = "document.startViewTransition=undefined;"
r = Run(port=8971, wd="/tmp/wd_tm")
def opened(b, motion, init):
    c = b.new_context(viewport={"width": 1280, "height": 800}, reduced_motion=motion, color_scheme="light")
    c.add_init_script(SKIP_HINTS + SPY + init)
    pg = c.new_page(); pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#drop", timeout=120000)
    pg.click("#np-set"); pg.wait_for_selector("input[name=uip-th][value=dark]", state="attached", timeout=30000)
    return c, pg
try:
    with sync_playwright() as p:
        r.page(p); b = r.b
        def vt():
            c, pg = opened(b, "no-preference", SPY_VT)
            pg.check("input[name=uip-th][value=dark]", force=True)
            pg.wait_for_function("document.documentElement.getAttribute('data-theme')==='dark'", timeout=10000)
            assert pg.evaluate("__vt") == 1, pg.evaluate("__vt")
            bg = pg.evaluate("getComputedStyle(document.body).backgroundColor"); assert bg != "rgb(255, 255, 255)", bg
            c.close()
        step("motion allowed: the switch goes through one view transition and ends dark", vt)
        def fallback():
            c, pg = opened(b, "no-preference", NO_VT)
            pg.check("input[name=uip-th][value=dark]", force=True)
            pg.wait_for_function("document.documentElement.getAttribute('data-theme')==='dark'", timeout=10000)
            assert pg.evaluate("__tr") >= 1, "the .tema-tr fade class was never set"
            pg.wait_for_function("!document.documentElement.classList.contains('tema-tr')", timeout=10000)
            c.close()
        step("without view transitions the colour fade class is set and removed", fallback)
        def still():
            c, pg = opened(b, "reduce", SPY_VT)
            got = pg.evaluate("(() => { document.querySelector('input[name=uip-th][value=dark]').click(); return document.documentElement.getAttribute('data-theme'); })()")
            assert got == "dark", got
            assert pg.evaluate("__vt") == 0 and pg.evaluate("__tr") == 0, "reduced motion must not animate"
            c.close()
        step("reduced motion: the theme is applied at once, no transition", still)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
