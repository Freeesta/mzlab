"""Chromatogram cursor: the mouse box, the pass-over line, the peak labels and the handle say only the time («14.33 min»), never the m/z (it is in the title and the legend). TIC and XIC."""
import sys, os, re; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_cursore_tempo.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
TIME = re.compile(r"^\d+\.\d{2} min$")
def probe(pg, sel):
    P = f"E.panels.find(p=>p.type==='{sel}')"
    c = pg.evaluate(f"(()=>{{const p={P},q=p.cv.getBoundingClientRect();return {{x:q.x+q.width*0.5,y:q.y+q.height*0.5}}}})()")
    pg.mouse.move(c["x"] - 20, c["y"]); pg.mouse.move(c["x"], c["y"], steps=4)
    attendi(pg, f"(()=>{{const t={P}.el.querySelector('.tip');return t&&!t.hidden}})()")
    tip = pg.evaluate(f"(()=>{{const t={P}.el.querySelector('.tip');return t&&!t.hidden?t.querySelector('b').textContent:null}})()")
    rd = pg.evaluate(f"{P}.rd.textContent")
    lbl = pg.evaluate(f"({P}._a.lbls||[]).filter(l=>l.etpk).map(l=>l.text)")
    cur = pg.evaluate(f"(()=>{{const p={P};p.cur=10;cursorLine(p);return p.ch.getAttribute('aria-valuetext')}})()")
    return tip, rd, lbl, cur
r = Run(port=8886, wd="/tmp/wd86")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz("B_FullMass-t0")])
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg); frames(pg, 8)
        def check(sel):
            tip, rd, lbl, cur = probe(pg, sel); print(sel, tip, rd, lbl, cur)
            assert tip and TIME.match(tip), tip
            assert TIME.match(rd), rd
            assert lbl and all(TIME.match(t) for t in lbl), lbl
            assert TIME.match(cur), cur
        step("TIC: one number with «min» in the mouse box, the line, the peak labels and the handle, no m/z", lambda: check("chrom"))
        def xic():
            pg.evaluate("xicDirect(364.3)"); ready(pg); frames(pg, 10)
            pg.wait_for_function("(()=>{const p=E.panels.find(p=>p.type==='xic');return p&&p._a&&p._a.sr&&p._a.sr.length})()", timeout=15000); frames(pg, 6)
            check("xic")
        step("XIC: the same, no m/z next to the time", xic)
        print([e for e in r.errs if e[0] != "console.warning"])
        assert not pg.evaluate("window.MZLAB_LR_MISS&&window.MZLAB_LR_MISS.length")
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
