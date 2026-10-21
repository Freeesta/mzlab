"""Disegno loading: once the editor is visible its bars never move (no native-Ketcher flash followed by a jump), and the side panels stay shown."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
r = Run(port=8933, wd="/tmp/wd_dcarica")
# bounding boxes of the bars and of the proxy buttons; null while the editor is hidden or not built yet
SAMPLE = """() => { const f = document.getElementById('kframe'), d = f && f.contentDocument;
 if (!d || !d.body || getComputedStyle(f).visibility === 'hidden') return null;
 const q = s => { const e = d.querySelector(s); if (!e) return 'absent'; const b = e.getBoundingClientRect(); return [b.x, b.y, b.width, b.height].map(Math.round).join(','); };
 if (!d.querySelector('[class*=App-module_top]')) return null;   // blank frame before Ketcher is built: no buttons yet
 return {top: q('[class*=App-module_top]'), left: q('[class*=LeftToolbar-module_root]'), right: q('[class*=RightToolbar-module_root]'), rings: q('[class*=BottomToolbar-module_root]'), bar: q('#tp-bar'), zoom: q('[data-testid=zoom-in-tp]')}; }"""
fail = []
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1300, "height": 900})
        pg.evaluate("() => { window.__s = []; const t0 = performance.now(); window.__iv = setInterval(() => { try { const v = (%s)(); if (v) window.__s.push([Math.round(performance.now() - t0), v, !document.getElementById('side-toggle')?.closest('[hidden]'), getComputedStyle(document.getElementById('side-toggle')).display]); } catch (_) {} }, 30); }" % SAMPLE)
        pg.click("#nav [data-v=draw]")
        attendi(pg, "document.getElementById('kframe').classList.contains('kwait') === false && !!(window.TPDraw && TPDraw.ready())", timeout=60000)
        frames(pg, 30)       # the samples after the reveal
        s = pg.evaluate("() => { clearInterval(window.__iv); return window.__s; }")
        if not s: fail.append("the editor never became visible")
        else:
            first = s[0][1]
            if first["bar"] == "absent": fail.append("first visible frame has no drop-down bar: " + str(first))
            if first["rings"] != s[-1][1]["rings"] or any(x[1] != s[-1][1] for x in s): 
                moved = [(x[0], x[1]) for x in s if x[1] != s[-1][1]]
                fail.append("bars moved after becoming visible: %d of %d samples differ, first %s final %s" % (len(moved), len(s), moved[0] if moved else None, s[-1][1]))
            if any(x[3] == "none" for x in s): fail.append("the side panels were hidden")
        print("samples", len(s), "final", s[-1][1] if s else None)
        # the editor must be visible at the end
        if pg.evaluate("() => getComputedStyle(document.getElementById('kframe')).visibility") == "hidden": fail.append("editor still hidden")
        if pg.evaluate("() => document.getElementById('kload').hidden") is not True: fail.append("loading note still shown")
finally:
    r.close()
print("FAIL: " + "; ".join(fail) if fail else "OK")
sys.exit(1 if fail else 0)
