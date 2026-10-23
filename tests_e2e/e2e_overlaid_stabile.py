"""Overlaid chromatograms: ticking another file must not move the axes or the trace of a file already shown (same x of the peak, same plot edges, same left margin)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
GEOM = """() => { const p = E.panels.find(q => q.type === 'chrom' && q._a); const a = p._a, cv = p.cv;
  const s0 = a.sr[0]; return { n: a.sr.length, x0: a.x0, x1: a.x1, ymax: a.ymax, px: a.X(14.3), left: a.X(a.x0), right: a.X(a.x1), W: a.W, H: a.H, ml: window.MARG_L, mr: window.MARG_R, k0: s0.k, nx: s0.x.length }; }"""
r = Run(port=8887, wd="/tmp/wd87")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 950})
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t15.mzML")])
        pg.click("#opbtn"); ready(pg)
        boxes = pg.locator("#flst .fl:not(.pr) > input[data-k]")
        def only_t0():
            n = boxes.count(); assert n >= 2, n
            for i in range(1, n):
                if boxes.nth(i).is_checked(): boxes.nth(i).click()
            ready(pg); frames(pg, 3)
        step("one file ticked", only_t0)
        g1 = pg.evaluate(GEOM); print("1 file:", g1)
        boxes.nth(1).click(); ready(pg); frames(pg, 3)
        g2 = pg.evaluate(GEOM); print("2 files:", g2)
        def same():
            for k in ("x0", "x1", "px", "left", "right", "W", "ml", "mr"):
                assert g1[k] == g2[k], f"{k}: {g1[k]} -> {g2[k]}"
        step("x geometry identical with 1 and 2 files", same)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
