"""File list: the show/hide box of a file stays a small box (<= 18 px) inside a 24 px target (WCAG target size), the group box is small too, and the box still toggles the file."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
r = Run(port=8993, wd="/tmp/wd_lista_casella")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 900})
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        attendi(pg, "document.querySelector('#flist input[data-k=use]')", 20000)
        pg.click("text=Carica dati"); ready(pg)
        pg.wait_for_selector("#flst .fl > input[data-k]", timeout=5000)
        def small_box():
            m = pg.evaluate("""() => { const e = document.querySelector('#flst .fl > input[data-k]'), b = e.getBoundingClientRect(), c = getComputedStyle(e, '::before');
                return {w: b.width, h: b.height, vw: parseFloat(c.width), vh: parseFloat(c.height)}; }""")
            assert m["w"] >= 24 and m["h"] >= 24, m             # the target keeps its 24 px
            assert 10 <= m["vw"] <= 18 and 10 <= m["vh"] <= 18, m   # the drawn box stays small
        step("the file box is a 24 px target that draws a small box", small_box)
        def group_box():
            m = pg.evaluate("""() => { const e = document.querySelector('#flst .fgh input.gall'); if (!e) return null; const b = e.getBoundingClientRect(); return {w: b.width, h: b.height}; }""")
            if m: assert m["w"] <= 18 and m["h"] <= 18, m
        step("the group box is small", group_box)
        def toggles():
            before = pg.evaluate("document.querySelector('#flst .fl > input[data-k]').checked")
            pg.click("#flst .fl > input[data-k]"); frames(pg, 3)
            assert pg.evaluate("document.querySelector('#flst .fl > input[data-k]').checked") != before
        step("the box still toggles", toggles)
finally:
    r.close()
for s in steps: print(s)
sys.exit(1 if any(x[1] != "ok" for x in steps) else 0)
