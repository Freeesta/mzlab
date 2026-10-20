"""Header of the MS1 spectrum panel (HRMS DDA): at typical widths the controls never overlap (what does not fit goes into «⋯»), the file menu keeps a readable
width, and the active panel is marked by a bar on the left, not by a frame on every side."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_spettro_header.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("spettroheader")
r = Run(port=8987, wd="/tmp/wd_spettroheader")
# [class, left, right, text overflow] of every visible control of the header of each spectrum panel, in document order
BOXES = """() => E.panels.filter(q => q.type === 'spec').map(q => { const hd = q.el.querySelector('.hd');
  const l = [...hd.querySelectorAll(':scope > *, .ctl > *')].filter(e => e.offsetWidth).map(e => { const b = e.getBoundingClientRect(); return [(e.className.toString() || e.tagName), b.left, b.right, e.scrollWidth - e.clientWidth]; });
  return { l, over: hd.scrollWidth - hd.clientWidth }; })"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2)
        attendi(pg, "E.panels.some(q => q.type === 'spec' && q.link != null)")
        def bad(w):
            """The first problem found at this width, or None (the second pass of fitHd runs 300 ms after the resize: the caller retries)."""
            for h in pg.evaluate(BOXES):
                if h["over"] > 1: return (w, "header wider than its panel", h["over"])
                for a, b in zip(h["l"], h["l"][1:]):
                    if a[2] > b[1] + 1: return (w, "overlap", a, b)
                # a label never spills out of its box (the title and the file menu cut with «…» on purpose)
                spill = [c for c in h["l"] if c[3] > 1 and c[0] != "SELECT" and "ttl" not in c[0]]
                if spill: return (w, "text spills", spill)
        def no_overlap():
            for w in (1440, 1100, 900):
                pg.set_viewport_size({"width": w, "height": 900}); attendi(pg, f"innerWidth === {w}")
                for _ in range(10):
                    frames(pg, 10); problem = bad(w)
                    if not problem: break
                assert not problem, problem
        step("the controls of the spectrum header do not overlap (1440, 1100, 900 px)", no_overlap)
        def menu():
            pg.set_viewport_size({"width": 1440, "height": 900}); attendi(pg, "innerWidth === 1440"); frames(pg, 8)
            w = pg.evaluate("E.panels.filter(q => q.type === 'spec').map(q => q.el.querySelector('.ctl select').getBoundingClientRect().width)")
            assert all(x >= 100 for x in w), w
        step("the file menu keeps at least 100 px", menu)
        def active():
            pg.evaluate("setActive(E.panels.find(q => q.type === 'spec'))")
            st = pg.evaluate("(() => { const c = getComputedStyle(document.querySelector('.pnl.act')); return [c.outlineStyle, c.boxShadow]; })()")
            assert st[0] == "none" and "inset" in st[1], st
        step("the active panel has no frame, a bar on the left", active)
finally:
    r.close()
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
