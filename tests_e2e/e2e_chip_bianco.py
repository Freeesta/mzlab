"""Internal blank: its text is a chip in the bar above the plot above the plot («Bianco 0.3–2 min · 1.23e5 cps»), never drawn over the trace; × removes it, a click re-picks the stretch."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_chip_bianco.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = HERE / "shots" / "chip_bianco_tmp"; D.mkdir(parents=True, exist_ok=True)
F = D / "BK_t0.mzML"; ds.full_scan(F, 0, np.random.default_rng(3))
r = Run(port=8978, wd="/tmp/wd_chip_bianco")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(F)]); pg.wait_for_selector("text=Carica dati"); pg.click("text=Carica dati"); ready(pg)
        P = "E.panels.find(q=>q.tab==='full'&&q.type==='chrom')"
        pg.evaluate(f"ibkSet({P},'mean',[0.3,2])"); ready(pg); frames(pg, 6)
        def chip():
            t = pg.evaluate(f"({P}.el.querySelector('.ibkc')||{{}}).textContent||''")
            import re; assert re.match(r"^Bianco 0\.3–2 min · [\d.]+(e\d+)? cps×$", t), t
        step("chip «Bianco 0.3–2 min · level cps» in the bar above the plot", chip)
        def outside():
            v = pg.evaluate(f"(()=>{{const p={P}, c=p.el.querySelector('.ibkc').getBoundingClientRect(), v=p.el.querySelector('canvas').getBoundingClientRect(), k=v.height/p._a.H; return [c.bottom, v.top+M.t*k, c.width>0, c.top, v.top, v.bottom]}})()")
            assert v[2] and v[0] <= v[4] + 1, v  # the chip ends above the canvas: it cannot touch the trace
        step("the chip box ends above the plot area that holds the trace", outside)
        def targets():
            h = pg.evaluate(f"Math.min(...[...{P}.el.querySelectorAll('.ibkc button')].map(b=>b.getBoundingClientRect().height))"); assert h >= 24, h
        step("targets of at least 24 px", targets)
        def edit():
            pg.evaluate(f"{P}.el.querySelector('.ibkc [data-ib=edit]').click()"); assert pg.evaluate(f"!!{P}._pickIbk")
        step("a click on the chip asks for another stretch", edit)
        def off():
            pg.evaluate(f"{P}.el.querySelector('.ibkc [data-ib=x]').click()"); ready(pg); frames(pg, 4)
            assert pg.evaluate(f"!{P}.el.querySelector('.ibkc')") and pg.evaluate(f"!{P}.fondo||{P}.fondo.mode!=='tract'")
        step("× removes the subtraction", off)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
