# e2e_intestazione_stretta: at 400 px the header is at most two rows, nothing leaves the screen and every interactive target is at least 24 px
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8845, wd="/tmp/wd_hstretta")
# visible interactive elements of the page that sit inside the viewport (the header plus the first cards)
JS = """(()=>{const o=[];for(const e of document.querySelectorAll("button, a[href], select, input:not([type=hidden]), summary")){const b=e.getBoundingClientRect();
if(!b.width||!b.height||getComputedStyle(e).visibility==="hidden"||b.top>=innerHeight)continue;
o.push({id:e.id||e.className||e.tagName,l:Math.round(b.left),r:Math.round(b.right),w:Math.round(b.width),h:Math.round(b.height),hd:!!e.closest("header")})}
return {h:document.querySelector("header").getBoundingClientRect().height,sw:document.documentElement.scrollWidth,items:o}})()"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 400, "height": 800}); pg.reload(); pg.wait_for_timeout(3000)
        d = pg.evaluate(JS)
        step("header height <= 80 px (two rows)", lambda: (_ for _ in ()).throw(AssertionError(d["h"])) if d["h"] > 80 else None)
        step("page does not scroll sideways", lambda: (_ for _ in ()).throw(AssertionError(d["sw"])) if d["sw"] > 400 else None)
        def inside():
            bad = [i for i in d["items"] if i["hd"] and (i["l"] < 0 or i["r"] > 400)]
            assert not bad, bad
        step("every header control is inside the screen", inside)
        def sizes():
            bad = [i for i in d["items"] if min(i["w"], i["h"]) < 24]
            assert not bad, bad
        step("no target under 24 px", sizes)
        pg.set_viewport_size({"width": 1500, "height": 2200}); pg.wait_for_timeout(500)
        step("desktop header stays on one row", lambda: (_ for _ in ()).throw(AssertionError(pg.evaluate("document.querySelector('header').getBoundingClientRect().height"))) if pg.evaluate("document.querySelector('header').getBoundingClientRect().height") > 50 else None)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
