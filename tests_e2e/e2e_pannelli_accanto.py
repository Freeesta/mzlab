# e2e_pannelli_accanto: the «Show panels» button (#sb-toggle) sits right next to the last visible tab of the header at every width (also when the header wraps)
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8846, wd="/tmp/wd_pannelli_accanto")
JS = """(()=>{const t=[...document.querySelectorAll('#nav button')].filter(b=>{const q=b.getBoundingClientRect();return q.width&&getComputedStyle(b).visibility!=='hidden'});
const l=t[t.length-1].getBoundingClientRect(),s=document.querySelector('#sb-toggle').getBoundingClientRect();
return {tabs:t.length,gap:Math.round(s.left-l.right),dy:Math.round(s.top-l.top),w:Math.round(s.width)}})()"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        for w in (500, 600, 700, 900, 1200):
            def chk(w=w):
                pg.set_viewport_size({"width": w, "height": 900}); pg.reload()
                attendi(pg, "document.querySelector('#stbar .tb') && !document.documentElement.classList.contains('i18n-wait')"); frames(pg, 5)
                d = pg.evaluate(JS)
                assert d["w"] > 0 and abs(d["dy"]) < 20 and 0 <= d["gap"] <= 16, (w, d)
            step(f"{w} px: the panels button follows the last tab within 16 px", chk)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
