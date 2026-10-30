"""The header is final from the first paint: the tabs (Tabui, Flow), the panel button and the side list hint are written in index.html, so neither the header
height, the width of #nav nor the top of #drop change while the page starts (at 400 px the header used to wrap in steps and push #drop down)."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
lib.AUTO_TEMPI = False
from lib import *

# rAF log from the first frame: [header height, #nav width, #drop top] whenever it changes
LOG = """window.__h = []; (function f() { try { const h = document.querySelector('header'), n = document.querySelector('#nav'), d = document.querySelector('#drop');
  if (h && n && d && !document.documentElement.classList.contains('i18n-wait')) { const v = [Math.round(h.getBoundingClientRect().height), Math.round(n.getBoundingClientRect().width), Math.round(d.getBoundingClientRect().top)];
    const l = __h[__h.length - 1]; if (!l || l.join() !== v.join()) __h.push(v); } } catch (e) {} requestAnimationFrame(f); })()"""
SKIP_TOUR = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}))}catch(e){}"

if __name__ == "__main__":
    r = Run(port=8830 + 171, wd="/tmp/wd_testata_stabile"); bad = []
    try:
        with sync_playwright() as p:
            for w, h, loc in ((400, 800, "en-US"), (400, 800, "it-IT"), (800, 700, "en-US"), (1400, 900, "it-IT")):
                b = p.chromium.launch(); ctx = b.new_context(viewport={"width": w, "height": h}, locale=loc)
                pg = ctx.new_page(); pg.add_init_script(SKIP_TOUR); pg.add_init_script(LOG)
                pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "window.qqReady || document.querySelector('#fl-hint')"); frames(pg, 30)
                seen = pg.evaluate("window.__h")
                dw, tw = pg.evaluate("(() => { const t = document.querySelector('#wtable'), was = t.hidden; t.hidden = false; const v = [document.querySelector('#drop').getBoundingClientRect().width, t.getBoundingClientRect().width]; t.hidden = was; return v; })()")
                print(f"{w}x{h} {loc}: {seen} drop {dw} table {tw}")
                if abs(dw - tw) > 1: bad.append((w, loc, "drop width", dw, tw))
                if len(seen) != 1: bad.append((w, loc, seen))
                ctx.close(); b.close()
    finally:
        r.close()
    print("OK   header, #nav and #drop do not move at start" if not bad else f"FAIL header moves at start: {bad}")
    sys.exit(1 if bad else 0)
