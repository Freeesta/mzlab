"""Disegno side panel: every checkbox paints a 16-20 px box (not the 24 px native one that looked huge) inside a target of at least 24x24 px, and the
example link takes its colour from a CSS variable, not the browser's default blue. Desktop and tablet widths (820 and 1180 px)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
# painted box = the ::before box when the checkbox is custom drawn, else the element itself
BOX = """() => [...document.querySelectorAll('#v-draw input[type=checkbox]')].filter(e => e.offsetParent).map(e => {
  const r = e.getBoundingClientRect(), b = getComputedStyle(e, '::before'), custom = getComputedStyle(e).appearance === 'none';
  const side = custom ? Math.min(parseFloat(b.width), parseFloat(b.height)) : Math.min(r.width, r.height);
  return [e.id || (e.closest('label') || e).textContent.trim().slice(0, 24), Math.round(r.width), Math.round(r.height), Math.round(side)] })"""
LINK = """() => { const a = document.querySelector('#ex-link'); const c = getComputedStyle(a).color;
  const t = document.createElement('i'); t.style.color = 'var(--accent)'; document.body.appendChild(t); const v = getComputedStyle(t).color; t.remove(); return [c, v] }"""
r = Run(port=8872, wd="/tmp/wd_disegno_caselle")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        for w in (1440, 1180, 820):
            def sizes(w=w):
                pg.set_viewport_size({"width": w, "height": 1000}); attendi(pg, f"innerWidth === {w}")
                boxes = pg.evaluate(BOX); assert len(boxes) >= 3, boxes
                bad = [b for b in boxes if b[1] < 24 or b[2] < 24 or not 16 <= b[3] <= 20]
                assert not bad, bad
            step(f"checkboxes of the Drawing panels: target >= 24 px, painted box 16-20 px at {w} px", sizes)
        def link():
            c, v = pg.evaluate(LINK); assert c == v, f"link colour {c} is not var(--accent) {v}"
        step("«Vedi un esempio di disegno» takes its colour from a CSS variable", link)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
